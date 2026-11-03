# Build from the folder that holds agape-kernel, agape-server and agape-app:
#   docker build -f agape-server/Dockerfile -t agape .
# The cargo caches survive between builds, so an update only recompiles what changed.
FROM rust:1.97-slim AS build
WORKDIR /src
COPY agape-kernel agape-kernel
COPY agape-server agape-server
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/agape-server/target \
    cd agape-server && cargo build --release && cp target/release/agape-server /agape-server

FROM debian:bookworm-slim
RUN useradd -m agape && mkdir -p /data && chown agape /data
COPY --from=build /agape-server /usr/local/bin/agape-server
COPY agape-app/dist/server /app
USER agape
# AGAPE_SPEED: clock ticks per second (0.1 is real time). AGAPE_ADMIN_TOKEN: the operator secret for staff accounts.
ENV AGAPE_DATA=/data AGAPE_STATIC=/app PORT=8080 AGAPE_SPEED=0.1
VOLUME /data
EXPOSE 8080
CMD ["agape-server"]
