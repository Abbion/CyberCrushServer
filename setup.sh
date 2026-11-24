#!/usr/bin/env bash
# One-command backend setup for CyberCrush, using Docker.
#
#   ./setup.sh           start everything (first run also builds and loads example data)
#   ./setup.sh stop      stop the servers (data is kept)
#   ./setup.sh reset     stop and delete ALL data, then start fresh with example data
#   ./setup.sh status    show what is running and the URLs
#   ./setup.sh logs [x]  follow logs (optionally for one service, e.g. chat_server)

set -u
cd "$(dirname "$0")"

SERVICES=(authentication data bank chat news game_state hacking logs_viewer proxy)
DEFAULT_PORTS=(3000 3001 3002 3003 3004 3005 3006 8080 80)
ENV_FILE=".env"

say()  { printf '\n\033[1m%s\033[0m\n' "$*"; }
fail() { printf '\n\033[31mError: %s\033[0m\n' "$*" >&2; exit 1; }

check_docker() {
    command -v docker >/dev/null 2>&1 || fail "Docker is not installed.
Install it from https://docs.docker.com/get-docker/ (on Windows/Mac install Docker Desktop), then run this script again."
    docker info >/dev/null 2>&1 || fail "Docker is installed but not running (or you lack permission).
- Windows/Mac: open Docker Desktop and wait until it says it is running.
- Linux: run 'sudo systemctl start docker', and add yourself to the docker group:
  'sudo usermod -aG docker \$USER' then log out and in again."
    docker compose version >/dev/null 2>&1 || fail "'docker compose' is missing. Update Docker or install the compose plugin: https://docs.docker.com/compose/install/"
}

port_in_use() {
    if command -v ss >/dev/null 2>&1; then
        ss -ltn 2>/dev/null | awk '{print $4}' | grep -Eq "[:.]$1\$"
    else
        (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null
    fi
}

choose_ports() {
    # Keep earlier choices so the ports stay stable between runs.
    if [ -f "$ENV_FILE" ] && [ -n "$(docker compose ps -q 2>/dev/null)" ]; then
        local complete=true
        for s in "${SERVICES[@]}"; do grep -q "^$(echo "$s" | tr a-z A-Z)_HOST_PORT=" "$ENV_FILE" || complete=false; done
        $complete && return
        docker compose down >/dev/null 2>&1
    fi
    : > "$ENV_FILE"
    local used=" "
    for i in "${!SERVICES[@]}"; do
        local port=${DEFAULT_PORTS[$i]}
        while port_in_use "$port" || [[ "$used" == *" $port "* ]]; do port=$((port + 100)); done
        used+="$port "
        echo "$(echo "${SERVICES[$i]}" | tr a-z A-Z)_HOST_PORT=$port" >> "$ENV_FILE"
        [ "$port" != "${DEFAULT_PORTS[$i]}" ] && echo "Port ${DEFAULT_PORTS[$i]} is busy, using $port for ${SERVICES[$i]} server."
    done
}

lan_ip() {
    hostname -I 2>/dev/null | awk '{print $1}' || true
}

show_urls() {
    local ip; ip=$(lan_ip)
    local proxy_port; proxy_port=$(grep "^PROXY_HOST_PORT=" "$ENV_FILE" 2>/dev/null | cut -d= -f2)
    local suffix=""; [ "${proxy_port:-80}" != "80" ] && suffix=":$proxy_port"
    say "Backend is running. Use these addresses (names work in a browser, curl and your frontend code):"
    local names=(authentication:auth data:data bank:bank chat:chat news:news game_state:game-state hacking:hacking logs_viewer:logs)
    for pair in "${names[@]}"; do
        local s=${pair%%:*} host=${pair##*:}
        local var port
        var="$(echo "$s" | tr a-z A-Z)_HOST_PORT"
        port=$(grep "^$var=" "$ENV_FILE" 2>/dev/null | cut -d= -f2)
        printf '  %-15s http://%s.cybercrush.localhost%s   (or http://localhost:%s)\n' "$s" "$host" "$suffix" "${port:-?}"
    done
    echo
    echo "Game state admin page: http://game-state.cybercrush.localhost$suffix/admin (login: admin_username / admin_password from server.docker.conf)"
    [ -n "$ip" ] && echo && echo "A phone or another computer cannot use the .localhost names. Use http://$ip:<port> with the ports above."
    echo
    echo "Test users come from utils/database/example_data/example_users.json."
    echo "Stop: ./setup.sh stop   Logs: ./setup.sh logs   Wipe data: ./setup.sh reset"
}

start() {
    check_docker
    choose_ports
    say "Building and starting servers (the first run takes several minutes)..."
    LOAD_EXAMPLE_DATA=true docker compose up --build -d --wait --wait-timeout 600 \
        || { docker compose ps; fail "Something failed to start. Run './setup.sh logs' to see why, and send the output to a backend developer."; }
    show_urls
}

case "${1:-start}" in
    start)  start ;;
    stop)   check_docker; docker compose down && echo "Stopped. Data is kept." ;;
    reset)
        check_docker
        read -r -p "This deletes ALL database data. Continue? [y/N] " answer
        [[ "$answer" =~ ^[Yy]$ ]] || exit 0
        docker compose down -v
        start ;;
    status) check_docker; docker compose ps; show_urls ;;
    logs)   check_docker; shift; docker compose logs -f --tail 50 "$@" ;;
    *)      sed -n '2,9p' "$0"; exit 1 ;;
esac
