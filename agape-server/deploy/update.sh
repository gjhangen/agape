#!/bin/sh
# Every 3 minutes: take newer Agape code when there is some, and rebuild. If Agape isn't running
# (a crash, the machine restarted, an update that failed halfway), bring it back: start the
# container that's there, or rebuild if there isn't one (at most once an hour, so a build that
# keeps failing never loads the machine). Only one runs at a time. Touches nothing but Agape.
exec 9>/tmp/agape-update.lock
flock -n 9 || exit 0
cd /opt/agape || exit 0
running() { docker ps --filter name='^agape$' --filter status=running --format '{{.Names}}' | grep -q '^agape$'; }

if git fetch -q origin pilot && [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/pilot)" ]; then
  git reset -q --hard origin/pilot
  echo "$(date) updating to $(git rev-parse --short HEAD)"
  date +%s > /tmp/agape-last-build
  sh agape-server/deploy/install.sh
  exit
fi

running && exit 0
echo "$(date) Agape isn't running: $(docker ps -a --filter name='^agape$' --format '{{.Status}}') $(docker inspect -f 'exit={{.State.ExitCode}} oom={{.State.OOMKilled}} {{.State.Error}}' agape 2>/dev/null)"
df -h / | tail -1; free -m | sed -n 2p
if docker start agape >/dev/null 2>&1; then
  sleep 10
  running && { echo "$(date) started it again"; exit 0; }
  docker logs --tail 20 agape 2>&1 | grep -iv 'code\|password\|token'
fi
LAST=$(cat /tmp/agape-last-build 2>/dev/null || echo 0)
[ $(( $(date +%s) - LAST )) -lt 3600 ] && exit 0
date +%s > /tmp/agape-last-build
echo "$(date) rebuilding"
sh agape-server/deploy/install.sh
