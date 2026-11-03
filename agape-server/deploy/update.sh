#!/bin/sh
# Take newer Agape code when there is some, and rebuild. Only one runs at a time.
exec 9>/tmp/agape-update.lock
flock -n 9 || exit 0
cd /opt/agape || exit 0
git fetch -q origin pilot || exit 0
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/pilot)" ] && exit 0
git reset -q --hard origin/pilot
echo "$(date) updating to $(git rev-parse --short HEAD)"
sh agape-server/deploy/install.sh
