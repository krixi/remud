FROM debian:bullseye-slim as runner

WORKDIR /game

COPY --from=remud-build /remud/target/release/remud /game/remud

# ports for telnet and web APIs
EXPOSE 2004/tcp
EXPOSE 2080/tcp

ENV RUST_LOG="warn,remud_lib=info,remud=info"

# By default, we expect a folder 'world' to be mounted next to the binary, and to contain 'world.db'
ENTRYPOINT ["./remud", "--db", "./world/world.db"]
