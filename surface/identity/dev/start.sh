#!/bin/sh
# Start a disposable development identity service and serve the screens.
#
#   dev/start.sh <state-dir> <log-dir>
#
# HOST is the address a browser reaches this machine on (default 127.0.0.1).
# Sign-in goes through the contract tests' fake issuer, which signs every
# sign-in as the seed's first person, "ada"; the service itself does the whole
# OIDC exchange. On first start the directory is filled by
# lys-identity-dev-seed, and dev/seed-grants.sh then gives grants through the
# grant routes. Never point <state-dir> at records that matter.
# Binaries are read from CARGO_TARGET_DIR (default <repo>/target), release profile.
set -eu

if [ $# -ne 2 ]; then
  echo "usage: dev/start.sh <state-dir> <log-dir>" >&2
  exit 2
fi
HOST=${HOST:-127.0.0.1}
APP_PORT=${APP_PORT:-8470}
SERVICE_PORT=${SERVICE_PORT:-8471}
ISSUER_PORT=${ISSUER_PORT:-8472}
SURFACE=$(cd "$(dirname "$0")/.." && pwd)
REPO=$(cd "$SURFACE/../.." && pwd)
BIN=${CARGO_TARGET_DIR:-$REPO/target}/release
mkdir -p "$1" "$2"
STATE=$(cd "$1" && pwd)
LOGS=$(cd "$2" && pwd)
ISSUER="http://$HOST:$ISSUER_PORT"

for port in "$APP_PORT" "$SERVICE_PORT" "$ISSUER_PORT"; do
  if lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "port $port is already in use; run dev/stop.sh $STATE first" >&2
    exit 1
  fi
done

key() {
  if [ ! -f "$1" ]; then
    head -c 32 /dev/urandom >"$1"
    chmod 600 "$1"
  fi
}
key "$STATE/service.key"
key "$STATE/issuer.key"
# The fake issuer's fixed client secret, a test constant in fake_issuer.rs.
if [ ! -f "$STATE/client.secret" ]; then
  printf 'contract-test-client-secret' >"$STATE/client.secret"
  chmod 600 "$STATE/client.secret"
fi

cat >"$STATE/config.json" <<JSON
{
  "listen": "127.0.0.1:$SERVICE_PORT",
  "log_dir": "$STATE/log",
  "log_origin": "dev.lys/directory",
  "event_key_file": "$STATE/service.key",
  "issuer": "$ISSUER",
  "client_id": "lys-directory",
  "client_secret_file": "$STATE/client.secret",
  "redirect_url": "http://$HOST:$APP_PORT/callback",
  "administrator": { "issuer": "$ISSUER", "subject": "ada" },
  "link_audit_source": { "issuer": "$ISSUER", "subject": "link-audit" },
  "session_seconds": 28800,
  "secure_cookie": false,
  "grant_log_dir": "$STATE/grant-log",
  "grant_log_origin": "dev.lys/grants",
  "grant_model_file": "$STATE/model.json"
}
JSON

# The development permission model: each relation and the actions it carries.
cat >"$STATE/model.json" <<JSON
{
  "version": 1,
  "relations": {
    "owner": ["view", "edit", "grant"],
    "editor": ["view", "edit"],
    "viewer": ["view"]
  }
}
JSON

if [ ! -d "$STATE/log" ]; then
  "$BIN/lys-identity-dev-seed" "$STATE/config.json" ada bea >"$LOGS/seed.log" 2>&1
fi

nohup "$BIN/dev_issuer" "0.0.0.0:$ISSUER_PORT" "$ISSUER" "$STATE/issuer.key" ada ada@example.test >"$LOGS/issuer.log" 2>&1 &
echo $! >"$STATE/issuer.pid"
sleep 1
nohup "$BIN/lys-identity-server" "$STATE/config.json" >"$LOGS/service.log" 2>&1 &
echo $! >"$STATE/service.pid"
sleep 1
if [ ! -f "$STATE/grants.seeded" ]; then
  "$SURFACE/dev/seed-grants.sh" "http://127.0.0.1:$SERVICE_PORT" "$LOGS/seed.log" >"$LOGS/seed-grants.log" 2>&1
  touch "$STATE/grants.seeded"
fi

cd "$SURFACE"
npm run build >"$LOGS/app-build.log" 2>&1
LYS_IDENTITY_SERVICE="http://127.0.0.1:$SERVICE_PORT" nohup node node_modules/vite/bin/vite.js preview --host 0.0.0.0 --port "$APP_PORT" --strictPort >"$LOGS/app.log" 2>&1 &
echo $! >"$STATE/app.pid"
echo "open http://$HOST:$APP_PORT/"
