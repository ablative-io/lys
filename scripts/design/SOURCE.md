# Where these scripts come from

`validate.py`, `check-coverage.py`, `render-cluster.py`, `render-brief.py`, `schemas/` and
`workers/ds2_ledger/roots.py` are copied from the design-system method at commit 3c3bac7.
Two lines differ from the method's copies so the files stand beside each other here:
`validate.py` finds `schemas/` and `check-coverage.py` finds `workers/` next to the script
rather than one directory up. They are here so a fresh clone measures `docs/design` with no
path into anyone's folders; `gate.sh` is the leg `.land/gates.sh` runs.
