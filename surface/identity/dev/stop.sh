#!/bin/sh
# Stop what dev/start.sh started from <state-dir>. The directory's log is kept.
set -eu
if [ $# -ne 1 ]; then
  echo "usage: dev/stop.sh <state-dir>" >&2
  exit 2
fi
for name in app service issuer; do
  pid="$1/$name.pid"
  if [ -f "$pid" ]; then
    kill "$(cat "$pid")" 2>/dev/null || true
    rm -f "$pid"
  fi
done
