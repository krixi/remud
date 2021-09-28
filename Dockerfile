FROM rust as builder

COPY . /remud
WORKDIR /remud

# RUN cargo test

RUN cargo build --release --bin remud

FROM debian:buster-slim as runner

RUN apt-get update && apt-get install -y glibc && rm -rf /var/lib/apt/lists/*

WORKDIR /game

COPY --from=builder /remud/target/release/remud /game/remud

# ports for telnet and web APIs
EXPOSE 2004/tcp
EXPOSE 2080/tcp

ENTRYPOINT ["./remud", "--db", "./world/world.db"]
