# Real old-install upgrade proof

`upgrade_live.py` creates a private disposable installation using the clean
`1b568cd90578f5ed5d7d438e628b23724eef7f12` release. It creates records through
that release's HTTP API, keeps its signed-in cookie across upgrade, invokes the
candidate's real `lys identity upgrade`, and compares the API readbacks. It also
checks old app leaves byte for byte and the declared message-service config
migration. No production root is accepted: the work directory must not exist.

Build the old and candidate binaries on Dean with the normal release target and
package each surface with its own `npm run package`. Coordinate ports 8490 and
8472 with other identity gates before running. The script refuses dirty or wrong
binary stamps and a surface manifest naming a different commit. Install and
upgrade themselves verify the surface files against that manifest.

```
python3 scripts/identity-gates/upgrade_live.py \
  --old-source /path/to/clean-old-checkout \
  --old-bin /path/to/clean-old-checkout/target/release \
  --old-surface /path/to/old-surface-package \
  --candidate-bin /path/to/candidate/target/release \
  --candidate-surface /path/to/candidate-surface-package \
  --candidate-commit FULL_COMMIT \
  --work /path/to/new-private-fixture
```

Evidence is written under `work/evidence`. A passing receipt is written only
after all comparisons and fixture cleanup succeed. The fixture uses synthetic
accounts; the directory is owner-only because setup and API evidence may contain
credentials. Do not publish its raw contents. A failed run retains its evidence
and tears down only its own processes and compose project.

Current boundary: this runner is not yet the complete combined-release gate.
The held legacy team-membership cases and person-set budget confirmation/raise
cases must be added once their migration/API contract is available. A pass of the
current runner cannot authorize installation without those cases. The old build
is a clean reconstruction of the recorded live commit; it does not establish
that the historical working tree that produced the live binary was clean.

The small Python suite checks the verifier itself, not an installation:

```
python3 -m unittest discover -s scripts/identity-gates -p test_upgrade_fixture.py -v
```
