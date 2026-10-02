# Real old-install upgrade proof

`upgrade_live.py` creates a private disposable installation using the clean
release selected by `--old-commit`. Run separate proofs for the historical
`1b568cd90578f5ed5d7d438e628b23724eef7f12` baseline required by HOME-037 and
DIRECTORY-069, and the installed `8c064b62a0c77f0874c203189a2ed3238b7ba57a`.
It creates records through
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
  --old-commit FULL_OLD_COMMIT \
  --old-bin /path/to/clean-old-checkout/target/release \
  --old-surface /path/to/old-surface-package \
  --candidate-bin /path/to/candidate/target/release \
  --candidate-surface /path/to/candidate-surface-package \
  --candidate-commit FULL_COMMIT \
  --work /path/to/new-private-fixture
```

First run the same command with `--prepare-only`. This checks exact binary
stamps, every surface file's hash and length, and binds each fixture's real Unix
socket path using the kernel's limit. It starts no installer, reserves no TCP
port and leaves no work directory. Its output is preparation evidence, never an
upgrade pass. Use a short work path; the named socket refusal comes before an
install, rather than after the runner has been started. Then coordinate the
service window and run without `--prepare-only`.

Preparation also resolves the old and candidate install layouts from their
exact source revisions. Installed readbacks use `Layout::BINARIES`, which names
the broker and identity server; the CLI remains in the input artifact package.
The receipt includes every fixed product path, all verifier modules and their
hashes, external executables, and an inventory of the harness's path expressions.
Inputs must exist now. Future fixture logs and rollback paths are checked against
their source producers; preparation does not pretend those future files exist.

The runner creates separate positive and negative installs under `work/`.
Evidence is written under each one's `evidence/`. A passing root receipt is
written only after both legs and their fixture cleanup succeed. The fixtures use synthetic
accounts; the directory is owner-only because setup and API evidence may contain
credentials. Do not publish its raw contents. A failed run retains its evidence
and tears down only its own processes and compose project.

The old API also creates an ordinary person's real issuer login, a team with
that person and two foreign members, a first self-set personal budget, a raised
budget and a period-only budget edit. After upgrade it requires every old team
member unchanged, the team's `members` unchanged, null `parent` and `lead`, and
`held` naming exactly the two foreign members, each hold carrying the operation,
login and time of the old record that added it. Every old per-measure budget
must read back as one limit of the holder's collection, with the collection
version the sum of the old versions and the last setter and time kept; all three
personal budgets are named unconfirmed with the old request unchanged and the
complete earlier effective limit, period and action enforced. The agent budget
has no confirmation and no team aggregate; live usage figures must each be a
figure or a named gap. The ordinary person's original session must also still work.

An ordinary old provisioning profile includes an MCP server. Its HTTP readback
and complete stored file must survive the window, rollback and normal upgrade.
Only the candidate's declared empty `skill_pins`, null `harness`, null
`permissions`, null `session` and `instructions_mode` of `append` may be added to
the HTTP profile; old fields and stored bytes remain exact. A goal never
deactivated reads back `active`. The configuration keeps every old setting; the
grant model version rises by one only when the old one is below the shipped
version, and an old release gains one organisation zone recorded by the host
setup after its session began. The old setup creates an Active administrator, which is asserted.
The ordinary person is activated through the old transition API before a root
grant is issued. The proof never claims that a Registered administrator was
admitted after upgrade.

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
sessions. It saves the exact candidate intent bytes consumed by that old parser and requires the original configuration restored byte for byte. A second reversible window independently exercises the candidate recovery path, with the same byte and record checks. It finally runs the normal candidate CLI upgrade separately.

The old loader also creates an agent through the real bearer import endpoint. Its original signed receipt, service public key and digest are recorded as a historical vector and must stay identical during both reversible windows, after both recoveries and after the final upgrade. The fixture never re-encodes or signs that vector itself.

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

Between legs, teardown registers a kernel exit observer before killing each
verified fixture process and waits for its exit event (kqueue on macOS, pidfd
on Linux). Docker teardown is also synchronous. The next leg's port check
uses SO_REUSEADDR and bind/listen, matching the Unix Tokio listeners used by
both services; a closed connection in TIME_WAIT must not count as a listener.
An active listener still refuses, naming its address. The check does not
reserve a port for the installer: this proof requires an exclusive venue.

Current boundary: earlier real-install attempts found harness errors before
completion. Every required baseline still needs complete positive and negative
receipts, as well as the separate exact-head repository gates. A successful
positive leg alone does not establish that the negative leg passed.
The old build is a clean reconstruction of the recorded live commit; it does not establish
that the historical working tree that produced the live binary was clean.

The small Python suite checks the verifier itself, not an installation:

```
python3 -m unittest discover -s scripts/identity-gates -p 'test_upgrade_*.py' -v
```
