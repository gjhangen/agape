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

echo "Building Agape (10-20 minutes the first time)..."
docker build -q -f agape-server/Dockerfile -t agape . >/dev/null
docker rm -f agape >/dev/null 2>&1 || true
docker run -d --name agape --restart unless-stopped -p "$PORT:8080" -v "$DATA:/data" --env-file "$DATA/agape.env" agape >/dev/null
if command -v ufw >/dev/null && ufw status | grep -q "Status: active"; then ufw allow "$PORT/tcp" >/dev/null; fi

sleep 3
docker ps --filter name=agape --format '{{.Status}}' | grep -q Up || { docker logs --tail 30 agape; exit 1; }
echo
echo "Agape is running:  http://agapetoyourseat.com:$PORT   (or http://$(hostname -I | awk '{print $1}'):$PORT)"
echo "Demo staff password: $(grep DEMO "$DATA/agape.env" | cut -d= -f2)"
