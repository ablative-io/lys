#!/bin/sh
# The real old-install upgrade proof, as one gate leg on a machine that both
# builds and runs it: upgrade_proof_build.sh writes the candidate's and every
# old release's programs and packaged screens into one private folder, then
# upgrade_proof_run.sh proves each upgrade from them. Where the two halves run
# on different machines, each is run on its own. The old releases are built in
# clones kept under LYS_UPGRADE_PROOF_CLONES (default: a folder beside the
# artifacts, removed with them).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
private=$(mktemp -d /tmp/lys-upgrade-proof-artifacts.XXXXXX)
chmod 700 "$private"
cleanup() {
  status=$?
  rm -rf "$private"
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

clones=${LYS_UPGRADE_PROOF_CLONES:-$private/clones}
sh "$here/upgrade_proof_build.sh" "$private/artifacts" "$clones"
sh "$here/upgrade_proof_run.sh" "$private/artifacts"
