#!/bin/sh
# Give the development directory's grants through the service's own grant
# routes, signed in as the seed's first person, who is the root authority.
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
SCRIBE=$(awk '$2 == "Ada'"'"'s" && $3 == "scribe" { print $1 }' "$2")
JAR=$(mktemp)
trap 'rm -f "$JAR"' EXIT

op() { printf 'op-%s' "$(od -An -tx1 -N16 /dev/urandom | tr -d ' \n')"; }
post() { curl -sf -b "$JAR" -H 'content-type: application/json' -d "$2" "$SERVICE$1"; }

# Sign in through the service and its issuer, as a browser would.
authorize=$(curl -s -c "$JAR" -b "$JAR" -o /dev/null -w '%{redirect_url}' "$SERVICE/login")
callback=$(curl -s -o /dev/null -w '%{redirect_url}' "$authorize")
curl -sf -c "$JAR" -b "$JAR" -o /dev/null "$SERVICE/callback?${callback#*\?}"

NOW=$(date +%s)
MONTH=$((NOW + 30 * 86400))
WEEK=$((NOW + 7 * 86400))

root=$(post /grants/roots '{"operation":"'"$(op)"'","route":"api","holder":"'"$ADA"'","resource":{"kind":"project","id":"identity"},"relation":"owner","pass_on":{"kind":"to","actions":["view","edit"],"recipients":["agent"]},"window":{"starts_at":'"$NOW"',"ends_at":'"$MONTH"'}}')
echo "root: $root"
post /grants/roots '{"operation":"'"$(op)"'","route":"api","holder":"'"$ADA"'","resource":{"kind":"project","id":"ledger"},"relation":"viewer","pass_on":{"kind":"use_only"},"window":{"starts_at":'"$NOW"',"ends_at":null}}'
echo
ROOT=$(printf '%s' "$root" | sed -n 's/.*"grant":"\([^"]*\)".*/\1/p')
post /grants '{"operation":"'"$(op)"'","route":"api","source":"'"$ROOT"'","recipient":"'"$SCRIBE"'","responsible":"'"$ADA"'","resource":{"kind":"project","id":"identity"},"relation":"viewer","pass_on":{"kind":"use_only"},"window":{"starts_at":'"$NOW"',"ends_at":'"$WEEK"'}}'
echo
