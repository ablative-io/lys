#!/bin/sh
# Give the development directory's grants through the service's own grant
# routes, using password sign-in as the seed's root authority.
# The disposable issuer accepts the non-empty development placeholder;
# no credential from a real identity install is used.
#
#   dev/seed-grants.sh <service-url> <seed-log>
#
# <seed-log> is what lys-identity-dev-seed printed; the ids are read from it.
# Ada gets a root grant, owner of project:identity, that she may pass on to
# agents (view and edit), and a use-only root grant, viewer of project:ledger.
# She then gives her scribe viewer of project:identity from her own grant.
set -eu
if [ $# -ne 2 ]; then
  echo "usage: dev/seed-grants.sh <service-url> <seed-log>" >&2
  exit 2
fi
SERVICE=$1
ADA=$(awk '/signs in as ada$/ { print $1 }' "$2")
SCRIBE=$(awk '$2 == "Scribe" { print $1 }' "$2")
case "$SERVICE" in
  http://*|https://*) ;;
  *) echo "service-url must be an HTTP address" >&2; exit 2 ;;
esac
for id in "$ADA" "$SCRIBE"; do
  case "$id" in
    ''|*[!a-zA-Z0-9-]*) echo "seed-log must name one root person and one scribe" >&2; exit 2 ;;
  esac
done
JAR=$(mktemp)
trap 'rm -f "$JAR"' EXIT

op() { printf 'op-%s' "$(od -An -tx1 -N16 /dev/urandom | tr -d ' \n')"; }
post() { curl -sfS -b "$JAR" -H 'content-type: application/json' -d "$2" "$SERVICE$1"; }

# The service carries the issuer exchange and returns the session cookie.
signed_in=$(curl -sfS -c "$JAR" -b "$JAR" -H 'content-type: application/json' \
  -d '{"email":"ada@example.test","password":"development-only"}' \
  -o /dev/null -w '%{http_code}' "$SERVICE/sign-in")
if [ "$signed_in" != 200 ]; then
  echo "development sign-in answered $signed_in instead of a session" >&2
  exit 1
fi

NOW=$(date +%s)
MONTH=$((NOW + 30 * 86400))
WEEK=$((NOW + 7 * 86400))

root=$(post /grants/roots '{"operation":"'"$(op)"'","route":"api","holder":"'"$ADA"'","resource":{"kind":"project","id":"identity"},"relation":"owner","pass_on":{"kind":"to","actions":["view","edit"],"recipients":["agent"]},"window":{"starts_at":'"$NOW"',"ends_at":'"$MONTH"'}}')
echo "root: $root"
post /grants/roots '{"operation":"'"$(op)"'","route":"api","holder":"'"$ADA"'","resource":{"kind":"project","id":"ledger"},"relation":"viewer","pass_on":{"kind":"use_only"},"window":{"starts_at":'"$NOW"',"ends_at":null}}'
echo
ROOT=$(printf '%s' "$root" | sed -n 's/.*"grant":"\([^"]*\)".*/\1/p')
if [ -z "$ROOT" ]; then
  echo "root grant answered no grant id" >&2
  exit 1
fi
post /grants '{"operation":"'"$(op)"'","route":"api","source":"'"$ROOT"'","recipient":"'"$SCRIBE"'","responsible":"'"$ADA"'","resource":{"kind":"project","id":"identity"},"relation":"viewer","pass_on":{"kind":"use_only"},"window":{"starts_at":'"$NOW"',"ends_at":'"$WEEK"'}}'
echo
