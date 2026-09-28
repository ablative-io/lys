#!/bin/sh
# The file-length gate leg (ADR-111): the checker's own tests, then the checker over every
# tracked code file. Exits non-zero if either does. Runs from the repository root.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
python3 -m unittest scripts/check_file_length_test.py
python3 scripts/check_file_length.py
