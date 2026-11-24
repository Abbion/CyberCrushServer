# One image definition for every server. Pick the server with --build-arg SERVER=<package name>.
FROM rust:1-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY shared_server_lib ./shared_server_lib
COPY cyber_crush_authentication_server ./cyber_crush_authentication_server
COPY cyber_crush_bank_server ./cyber_crush_bank_server
COPY cyber_crush_chat_server ./cyber_crush_chat_server
COPY cyber_crush_data_server ./cyber_crush_data_server
COPY cyber_crush_game_state_server ./cyber_crush_game_state_server
COPY cyber_crush_hacking_server ./cyber_crush_hacking_server
COPY cyber_crush_news_server ./cyber_crush_news_server
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --workspace && \
    mkdir /out && \
    for bin in cyber_crush_*_server; do cp target/release/$bin /out/; done

FROM debian:bookworm-slim
ARG SERVER
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home app
COPY --from=builder /out/${SERVER} /usr/local/bin/server
# Servers load "../server.conf", so the working directory sits one level below it.
WORKDIR /app/server
USER app
CMD ["server"]
