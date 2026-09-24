#!/bin/sh
# The docs leg of .land/gates.sh: every design document under docs/design validates against the
# method's schemas; every cluster's coverage is clean; and every cluster's rendered markdown is
# what its JSON renders to, so a brief that breaks brief.schema.json or a DESIGN.md that drifted
# from design.json cannot land. A cluster is one with a design.json; docs/design/identity holds
# the earlier revision-form briefs (IDENTITY-001, CONTEXT-001) that predate the method and are
# not measured here. Runs from the repository root in a fresh clone.
set -u
cd "$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)" || exit 2
here=scripts/design
status=0
python3 "$here/validate.py" docs/design/decisions.json || status=1
tmp=$(mktemp -d) || exit 2
mkdir -p "$tmp/docs" && cp -R docs/design "$tmp/docs/design" || exit 2
for cluster in docs/design/*/; do
  [ -f "$cluster/design.json" ] || continue
  python3 "$here/validate.py" "$cluster" || status=1
  python3 "$here/check-coverage.py" "$cluster" || status=1
  python3 "$here/render-cluster.py" "$tmp/$cluster" >/dev/null || status=1
  for md in $(cd "$cluster" && find . -name '*.md'); do
    if ! cmp -s "$tmp/$cluster/$md" "$cluster/$md"; then
      echo "rendered markdown differs from the committed file: $cluster$md"; status=1
    fi
  done
done
rm -rf "$tmp"
exit "$status"
