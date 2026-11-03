#!/bin/sh
# Agape on a shared server: its own folder, container, data and port. Touches nothing else
# except opening its one port in the firewall. Run again any time to update to the newest code.
# Run from the folder that holds agape-kernel, agape-server and agape-app.
set -e
cd "$(dirname "$0")/../.."
DATA=/opt/agape-data
mkdir -p "$DATA"
command -v docker >/dev/null || { echo "Docker is needed (Genesis installed it; run its installer first)"; exit 1; }

# A port nobody else is using, kept the same on later runs.
PORT=$(cat "$DATA/port" 2>/dev/null || true)
if [ -z "$PORT" ]; then
  for p in 8080 8081 8082 8090 8181 8888 9080; do
    if ! ss -ltn | awk '{print $4}' | grep -q ":$p\$"; then PORT=$p; break; fi
  done
  echo "$PORT" > "$DATA/port"
fi

# Secrets made here, on the server; they never leave it.
if [ ! -f "$DATA/agape.env" ]; then
  rand() { head -c 24 /dev/urandom | od -An -tx1 | tr -d ' \n'; }
  printf 'AGAPE_ADMIN_TOKEN=%s\nAGAPE_DEMO_PASSWORD=%s\nAGAPE_SPEED=0.1\n' "$(rand)" "$(rand | cut -c1-12)" > "$DATA/agape.env"
  chmod 600 "$DATA/agape.env"
fi
chown -R 1000:1000 "$DATA" 2>/dev/null || true

# the kernel's fingerprint: a night started on another kernel is closed and a new one opened
KB=$(find agape-kernel/src agape-kernel/templates -type f | sort | xargs cat | sha1sum | cut -c1-12)
echo "Building Agape (10-20 minutes the first time)..."
if ! DOCKER_BUILDKIT=1 docker build -q --build-arg AGAPE_KERNEL_BUILD=$KB -f agape-server/Dockerfile -t agape . >/dev/null 2>/tmp/agape-build.log; then
  # an older Docker without BuildKit: the same build without the caches
  sed -e '/--mount=type=cache/d' -e 's/^    cd agape-server/RUN cd agape-server/' agape-server/Dockerfile > /tmp/agape.Dockerfile
  docker build -q --build-arg AGAPE_KERNEL_BUILD=$KB -f /tmp/agape.Dockerfile -t agape . >/dev/null || { tail -30 /tmp/agape-build.log; exit 1; }
fi
docker rm -f agape >/dev/null 2>&1 || true
# The pilot's clock runs at twice real time, so a night (and a delivery) fits a demo.
SPEED=$(cat "$DATA/speed" 2>/dev/null || echo 0.2)
docker run -d --name agape --restart unless-stopped -p "$PORT:8080" -v "$DATA:/data" --env-file "$DATA/agape.env" -e AGAPE_SPEED="$SPEED" agape >/dev/null
# Updates itself: every 3 minutes the server checks for newer code and rebuilds when there is some.
cp agape-server/deploy/update.sh /usr/local/bin/agape-update && chmod +x /usr/local/bin/agape-update
echo "*/3 * * * * root /usr/local/bin/agape-update >> /var/log/agape-update.log 2>&1" > /etc/cron.d/agape-update
if command -v ufw >/dev/null && ufw status | grep -q "Status: active"; then ufw allow "$PORT/tcp" >/dev/null; fi

# HTTPS without touching ports 80/443: if a Cloudflare tunnel token has been saved here, a small
# connector carries https://agapetoyourseat.com to this container. The token never leaves the server.
if [ -s "$DATA/tunnel-token" ]; then
  docker rm -f agape-tunnel >/dev/null 2>&1 || true
  docker run -d --name agape-tunnel --restart unless-stopped --network host cloudflare/cloudflared:2026.10.7 \
    tunnel --no-autoupdate run --token "$(cat "$DATA/tunnel-token")" >/dev/null && echo "Secure address on: https://agapetoyourseat.com"
fi

sleep 3
docker ps --filter name=agape --format '{{.Status}}' | grep -q Up || { docker logs --tail 30 agape; exit 1; }
echo
echo "Agape is running:  http://agapetoyourseat.com:$PORT   (or http://$(hostname -I | awk '{print $1}'):$PORT)"
echo "Demo staff password: $(grep DEMO "$DATA/agape.env" | cut -d= -f2)"
