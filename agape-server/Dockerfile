# Build from the folder that holds agape-kernel, agape-server and agape-app:
#   docker build -f agape-server/Dockerfile -t agape .
FROM rust:1.97-slim AS build
WORKDIR /src
COPY agape-kernel agape-kernel
COPY agape-server agape-server
RUN cd agape-server && cargo build --release

FROM debian:bookworm-slim
RUN useradd -m agape && mkdir -p /data && chown agape /data
COPY --from=build /src/agape-server/target/release/agape-server /usr/local/bin/agape-server
COPY agape-app/dist/server /app
USER agape
# AGAPE_SPEED: clock ticks per second (0.1 is real time). AGAPE_ADMIN_TOKEN: the operator secret for staff accounts.
ENV AGAPE_DATA=/data AGAPE_STATIC=/app PORT=8080 AGAPE_SPEED=0.1
VOLUME /data
EXPOSE 8080
CMD ["agape-server"]
