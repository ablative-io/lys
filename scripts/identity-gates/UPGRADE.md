# Real old-install upgrade proof

`upgrade_live.py` creates a private disposable installation using the clean
`1b568cd90578f5ed5d7d438e628b23724eef7f12` release. It creates records through
that release's HTTP API, keeps its signed-in cookie across upgrade, invokes the
candidate's real `lys identity upgrade`, and compares the API readbacks. It also
checks old app leaves byte for byte and the declared message-service config
migration. No production root is accepted: the work directory must not exist.

Build the old and candidate binaries on Dean with the normal release target and
package each surface with its own `npm run package`. Build the candidate's
test-only driver with `cargo build -p lys --release --example upgrade_window`;
its build stamp must match the candidate. Coordinate ports 8490 and
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

The runner creates separate positive and negative installs under `work/`.
Evidence is written under each one's `evidence/`. A passing root receipt is
written only after both legs and their fixture cleanup succeed. The fixtures use synthetic
accounts; the directory is owner-only because setup and API evidence may contain
credentials. Do not publish its raw contents. A failed run retains its evidence
and tears down only its own processes and compose project.

The old API also creates an ordinary person's real issuer login, a team with
that person and two foreign members, a first self-set personal budget, a raised
budget and a period-only budget edit. After upgrade it requires exactly the two
foreign members held, all membership history retained, all three budgets named
unconfirmed, and the complete earlier effective limit, period and action.
The ordinary person's original session must also still work. Only the declared
empty `unconfirmed` field is additive on the unchanged agent budget readback.

An ordinary old provisioning profile includes an MCP server. Its HTTP readback
and complete stored file must survive the window, rollback and normal upgrade.

The test-only `upgrade_window` example calls the very same production
`identity::upgrade::upgrade` used by the CLI. At its existing service-ready
callback it invokes the HTTP/byte verifier and exits75 before `Started` is
recorded. It adds no production pause, timer or configuration switch. Both
membership and budget confirmation must answer503 naming `upgrade_pending`;
both logs, snapshots and the provisioning file must be byte-identical. The
profile-set, profile-review and skill-write routes must also answer503 naming
`ProvisioningUnavailable` and `upgrade_pending`; bytes are checked after each
of the five refusals. These require HOME037's guarded writers in the final build.
The positive leg then re-enters the real old installer, which performs its normal
recovery, and requires the old binaries to read all the original records and
sessions. It finally runs the normal candidate CLI upgrade separately.

The negative leg creates its own fresh old install. At the same interruption
point the driver stops only that install using the production unit lifecycle,
then deliberately appends one new-format Held leaf through the actual log
store. The byte checker must name that record and the real old installer's
recovery must fail with that exact leaf's unknown Held variant. A generic boot
failure does not count. The private receipt records the path and decoder refusal.

After the positive normal upgrade clears its intent, the proof checks that the
old family bytes still stand, crashes only its own identity process, and waits
on the installer's exit lock. Restart through the production unit lifecycle
must write exactly two Held rows and one Checked row, retain every old leaf,
and persist the new budget snapshot. Another crash and restart must leave all
these files byte-identical. There is no sleep or PID polling in this proof.

Current boundary: the real installs have not yet been exercised. Running this
against the complete landed release remains required, as do the separate
exact-head repository gates.
The old build is a clean reconstruction of the recorded live commit; it does not establish
that the historical working tree that produced the live binary was clean.

The small Python suite checks the verifier itself, not an installation:

```
python3 -m unittest discover -s scripts/identity-gates -p 'test_upgrade_*.py' -v
```
