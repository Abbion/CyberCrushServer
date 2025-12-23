import subprocess

components = [ "authentication", "bank", "chat", "data" ]

def run_tmux_session(name):
    server_dir = "cyber_crush_" + name + "_server"

    cmd = [
        "tmux", "new-window",
        "-n", (name + "_server"),
        f"cd {server_dir} && cargo run"
    ]

    print(f"Starting server: {name}...")
    subprocess.run(cmd, check=True)

def main():
    for comp in components:
        run_tmux_session(comp)

    print("Build finished")

if __name__ == "__main__":
    main()
