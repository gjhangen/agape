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

# The updater goes in first, so even if this build fails, the next run can bring Agape back.
cp agape-server/deploy/update.sh /usr/local/bin/agape-update && chmod +x /usr/local/bin/agape-update
echo "*/3 * * * * root /usr/local/bin/agape-update >> /var/log/agape-update.log 2>&1" > /etc/cron.d/agape-update

# A build needs room. Old Agape images are cleared first; if there still isn't room, nothing is
# built (a full disk would hurt the other project on this machine too).
docker image prune -f >/dev/null 2>&1 || true
FREE=$(df -Pm / | awk 'NR==2{print $4}')
if [ "${FREE:-0}" -lt 3000 ]; then echo "$(date) only ${FREE} MB free; not building"; df -h /; exit 1; fi

# the kernel's fingerprint: a night started on another kernel is closed and a new one opened
KB=$(find agape-kernel/src agape-kernel/templates -type f | sort | xargs cat | sha1sum | cut -c1-12)
echo "Building Agape (10-20 minutes the first time)..."
if ! DOCKER_BUILDKIT=1 docker build -q --build-arg AGAPE_KERNEL_BUILD=$KB -f agape-server/Dockerfile -t agape . >/dev/null 2>/tmp/agape-build.log; then
  # an older Docker without BuildKit: the same build without the caches
  sed -e '/--mount=type=cache/d' -e 's/^    cd agape-server/RUN cd agape-server/' agape-server/Dockerfile > /tmp/agape.Dockerfile
  docker build -q --cpu-shares 128 --build-arg AGAPE_KERNEL_BUILD=$KB -f /tmp/agape.Dockerfile -t agape . >/dev/null || { tail -30 /tmp/agape-build.log; exit 1; }
fi
# The pilot's clock runs at twice real time, so a night (and a delivery) fits a demo.
SPEED=$(cat "$DATA/speed" 2>/dev/null || echo 0.2)

# Before the running Agape is replaced, the new build must start and answer its health check on a
# copy of the data (never the real folder: two servers must not write the same files). If it
# doesn't, the old one keeps running and the reason is written to the log.
if command -v curl >/dev/null && docker ps --filter name='^agape$' --format '{{.Names}}' | grep -q agape; then
  TRY=/tmp/agape-trial; rm -rf "$TRY"; mkdir -p "$TRY"
  (cd "$DATA" && tar --exclude=./mail-files --exclude=./backups --exclude=./admin-backups -cf - .) | tar -xf - -C "$TRY"
  chown -R 1000:1000 "$TRY"
  docker rm -f agape-trial >/dev/null 2>&1 || true
  docker run -d --name agape-trial -p 127.0.0.1:18089:8080 -v "$TRY:/data" --env-file "$DATA/agape.env" -e AGAPE_SPEED="$SPEED" agape >/dev/null
  OK=""
  for i in $(seq 1 30); do
    if curl -sf --max-time 5 http://127.0.0.1:18089/api/health >/dev/null 2>&1 && curl -sf --max-time 5 -o /dev/null http://127.0.0.1:18089/app/; then OK=1; break; fi
    sleep 2
  done
  [ -z "$OK" ] && docker logs --tail 40 agape-trial
  docker rm -f agape-trial >/dev/null 2>&1 || true
  rm -rf "$TRY"
  if [ -z "$OK" ]; then echo "$(date) the new build didn't start cleanly; the running Agape was left in place"; exit 1; fi
fi
docker rm -f agape >/dev/null 2>&1 || true
PUBLIC=""; [ -s "$DATA/tunnel-token" ] && PUBLIC=https://agapetoyourseat.com
docker run -d --name agape --restart unless-stopped -p "$PORT:8080" -v "$DATA:/data" --env-file "$DATA/agape.env" -e AGAPE_SPEED="$SPEED" -e AGAPE_PUBLIC_URL="$PUBLIC" agape >/dev/null
# old images pile up with every update; the build cache is kept so updates stay quick
docker image prune -f >/dev/null 2>&1 || true
if command -v ufw >/dev/null && ufw status | grep -q "Status: active"; then ufw allow "$PORT/tcp" >/dev/null; fi

# HTTPS without touching ports 80/443: if a Cloudflare tunnel token has been saved here, a small
# connector carries https://agapetoyourseat.com to this container. The token never leaves the server.
# Left alone while it's running: restarting it on every update takes the site offline until it reconnects.
if [ -s "$DATA/tunnel-token" ] && ! docker ps --filter name='^agape-tunnel$' --filter status=running --format '{{.Names}}' | grep -q agape-tunnel; then
  docker rm -f agape-tunnel >/dev/null 2>&1 || true
  docker run -d --name agape-tunnel --restart unless-stopped --network host cloudflare/cloudflared:2026.10.7 \
    tunnel --no-autoupdate run --token "$(cat "$DATA/tunnel-token")" >/dev/null && echo "Secure address on: https://agapetoyourseat.com"
fi

sleep 3
docker ps --filter name=agape --format '{{.Status}}' | grep -q Up || { docker logs --tail 30 agape; exit 1; }
echo
echo "Agape is running:  http://agapetoyourseat.com:$PORT   (or http://$(hostname -I | awk '{print $1}'):$PORT)"
echo "Demo staff password: $(grep DEMO "$DATA/agape.env" | cut -d= -f2)"
