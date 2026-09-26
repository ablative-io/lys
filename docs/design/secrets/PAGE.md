# secrets — what was asked, what it means, and what was written

## The words, as they were typed

The unlanded draft IDENTITY-003 under docs/design/identity/briefs does not land; that directory sits outside the gate. The broker's brief on main is the secrets cluster: docs/design/secrets/briefs/SECRETS-001.json (done) and SECRETS-002.json, R1 to R9 with SEC_ identifiers, anchored to ADR-001 (secrets sit behind a handle the door swaps for the credential), ADR-002 (the token revolver is the first consumer), ADR-003 and ADR-004.

This brief is SECRETS-003, a brief of its own beside SECRETS-002, so what has landed stays stable; SECRETS-002 is not edited. It adds, in docs/design/secrets/, what the draft carries and main lacks: a documents-only baseline row recording from source the revolver's next-account call site, every consumer of the account pool file, every credential path into a seat today classed as the login exception, a proxied credential or neither, and what lys/delegation/v1 and the sealed envelope can carry for a handle, a lease and a sealed record, by file and line; a contract row before any code stating the invariant, the handle, the lease with its atomic use step and retry outcome, the cancellation rule, the SpiceDB relations and the audit line fields, with an adversarial review recording every attack tried (a replayed or forged handle, a use spent or counted twice, an admitted call outliving the rule, a credential in a log, a sealed key read, a stale SpiceDB answer) and the clause defeating each; counted acceptance legs on every code row (each refusal leg, a forced race on a k-use lease admitting exactly k, a redaction canary with a positive control, kill and reopen at each append boundary, each in-flight outcome its own test); hours per row; dependencies on DIRECTORY-002, 003 and 006 blocking the code rows.

Settled on main, not reopened: the seat's own login reaches its process at spawn as the one named exception (R5); OAuth refresh and spend caps are in scope (R4, R9); every point it records open stays open.

Boundaries: documents only, no code; no credential value; lys-core and its published formats unchanged, a signed lease format only as a new version alongside with its own adversarial review; no engine structural; no row dispatched before the lead's review; the gate is scripts/design/gate.sh.

Ask the lead, do not decide: whether the broker lives in lys (the draft's crates/lys-secrets) or in the cambium door SECRETS-002 names, which CN4 leaves open; whether the lease is a broker record or a signed format; whether the draft's hours stand. The brief records each as an open question with both readings.

The lead read the brief published at 4babcfe on 26 September and found the baseline R1 sound and holes in R2 to R8; every one of these corrections goes in, and R1 stays in this brief.

Blockers a build can act on. SECRETS-003.json carries depends_on [] while DIRECTORY-006 uses that field: name the DIRECTORY-006 requirements this brief waits on as R1 to R4 of 006, never the whole of 006, whose R6 waits on 005; give each blocker a check a stranger can run, a file, a command or a landed commit to look for.

The order of the code rows. R3 to R8 do not block on each other today, yet R4 edits R3's lib.rs, R6 and R7 use R4's revocation states, and R5 and R8 build on R4's proxy: state the order R3, then R4, then R5 to R8, in each row's own Blocked by.

The CONFORMANCE 3.2 and 3.4 review blocks only at brief level: put it in R4's own Blocked by.

Key custody. Nobody holds the store's encryption key: add a key-custody section to CONTRACT.md saying which key, where it is kept, how the store is unlocked and how the key is rotated; add R3 checks that the key is not in the store directory and that opening the store without it is refused by name.

Canaries on disk. Every canary check also searches every file under the test's store and log directories, expecting zero matches, with a positive control that plants the value and finds it.

The audit line and the handle. The audit line carries a handle id or digest only, never the raw handle, since the transparency log can be read and replayed; the handle is stored hashed; the canary plants a handle value. The Handle section says how the proxy authenticates whoever presents a handle.

Points R2 keeps open that the code rows must settle: the audit line's signer and schema, and how a sealed record is bound to its name and owner, since sealed_envelope.rs:182 has an empty AAD; R3's store gains a refusal check for ciphertext swapped between two entries. Either the contract states a broker-internal shape, reviewed adversarially, or those rows stay blocked and say so.

The delegation's time window. ADR-020 says the signed delegation keeps it, v1 has no window, and DIRECTORY-006 R1 puts it in the lys-identity grant: name the grant's window as the limit, amend ADR-020 to match, and list which criteria still wait on it.

ADR-001 says the broker is built in Rust inside the door and ADR-019 moves it into lys with supersedes []: record ADR-019 as amending ADR-001 and fix design.json:54. ADR-001 is Tom's decision; the lead puts the move to him herself, and the brief says the amendment waits on his word.

SEC_REVOKE_FRESHNESS has one fixed expected behaviour: R4 calls the landed DIRECTORY-006 permission check and asserts that behaviour and zero forwarded calls.

SEC3_OAUTH_RACE asserts that the provider sees exactly one refresh for three simultaneous calls.

The criterion mapping is searchable: "the row's report maps each criterion" names the report path, or the criterion ids sit in the test names so a search proves the mapping.

The canary scope is countable: say how logs are captured, a tracing subscriber in tests/support; keep every secret byte in one redacting type; enforce that with an ast-grep rule or a counting test.

File lists. R4 and R5 list crates/lys-secrets/Cargo.toml and Cargo.lock; tests/support/mod.rs is listed on every row whose test doubles live there.

Two different identities are checked as two different commit authors in git log, and SEC3_STORE_RACE states its total case count.

## What the survey found, and its angles

Write SECRETS-003 as a documents-only brief beside SECRETS-002, which is not edited. It takes the draft the lead reviewed at 4babcfe and applies every correction from the lead's 26 September review. R1, the baseline, stays as it was. R2, the contract and its adversarial review, gains a key-custody section, handle authentication, a hashed handle and an audit line that carries only a handle id or digest. R2 must also either state a broker-internal shape for the audit signer and schema and for sealed-record binding, or keep the rows that need them blocked. R3 to R8, the code rows, are ordered R3, then R4, then R5 to R8. Their blockers become checks a stranger can run: DIRECTORY-006 R1 to R4, never the whole of 006. Their canary, race and criterion-mapping legs become countable and searchable. The same unit records ADR-019 as amending ADR-001, pending Tom's word, and ADR-020 with the lys-identity grant's window as the limit.

### What the tree holds

- `docs/design/secrets/briefs/SECRETS-003.json (+ .md)` — The new brief. It is not in this tree. It exists only as the unlanded draft at 4babcfe in the canonical repo /Users/tom/Developer/ablative/stack/lys, on branch origin/brief/secrets/604f95d0-4495-4a47-b6f1-deeacb2177cb: 330 JSON lines, 8 rows, depends_on [], C19-C29 and S15-S18. Every correction lands here.
- `docs/design/secrets/briefs/SECRETS-002.json` — It must stay byte-identical. It has 9 requirements and 57 acceptance criteria. Its blocked_by already names the host discrepancy and the R7 freshness review. SEC_REVOKE_FRESHNESS (R7) reads 'either await ... or refuse by name ... Assert the selected behaviour'. R9 says 'The time window lives in the signed delegation'.
- `docs/design/secrets/design.json` — ADR-019 and ADR-020 go in its decisions list, and its structure gains rows for BASELINE.md, CONTRACT.md, the review report and the crates/lys-secrets files. CN4 gets settled or annotated here. The 'Tom's model is built in Rust inside the door' non-goal reason is at line 51 on main but at line 54 in the draft.
- `docs/design/decisions.json` — The ledger ends at ADR-018 on main, so ADR-019 and ADR-020 exist only in the draft. The schema has additionalProperties:false, carries no 'amends' field, and its status is one of proposed, decided or superseded. So 'amending ADR-001' has to be expressed in the context and consequences fields, or through status.
- `docs/design/secrets/checklist.json, stories.json, CHECKLIST.md, USER-STORIES.md, DESIGN.md` — Main holds C1-C18 and S1-S14. The draft adds C19-C29 and S15-S18, and the gate checks that the rendered markdown matches the JSON.
- `docs/design/roadmap.json` — The draft adds RM-013 with depends_on RM-001 and RM-002. Its further-units notes carry the rows this brief names.
- `scripts/design/gate.sh` — This is the gate. It measures every cluster that has a design.json. docs/design/identity has none, so it is skipped, which is why an identity/briefs draft sits outside the gate.
- `scripts/design/schemas/brief.schema.json` — depends_on is 'Brief IDs that must land first — direct dependencies only, the dispatcher orders by these'. It cannot name R1-R4 of a brief, so requirement-level blockers go in blocked_by or in each row's Blocked by.
- `docs/design/directory/briefs/DIRECTORY-006.json` — R1 (grant contract with a time window, marked PROPOSAL FOR REVIEW), R2, R3 and R4 (freshness, marked 'The mechanism and choice must be settled before dispatch') are the blockers. R6 waits on DIRECTORY-005. It lists 28 files to create, 0 of which exist, under crates/lys-identity/src/grants/ and crates/lys-identity/tests/.
- `crates/lys-core/src/delegation/mod.rs:218-221` — 'There is no `not_after`'. v1 has no time window, which contradicts SECRETS-002 R9 and the draft's ADR-020 wording.
- `crates/lys-core/src/seal/sealed_envelope.rs:182 (and :254)` — `aad: &[]` on both seal and open, so a sealed record is not bound to its name or owner. That is behind the ciphertext-swap refusal check the words add to R3.
- `manifold @ 3df5ac5f643e0479a0bc0464ef76e36f083c0246: crates/manifold-node/src/seat/revolver.rs:158,172` — R1's pinned call site. `fn turned` is at line 158 and `document.account = Some(next.to_owned());` at line 172, both verified.
- `.land/gates.sh` — The lys land gate: the design gate plus the six cargo legs. It has no ast-grep leg, so an ast-grep rule for the redacting type would not run in the gate unless a row adds one.
- `Cargo.toml (workspace)` — 6 members, no lys-secrets and no lys-identity. It has tracing 0.1 but no tracing-subscriber. R3 and R4 add the crate, and the words list crates/lys-secrets/Cargo.toml and Cargo.lock on R4 and R5.

### What was already decided

- ADR-001 — The broker is 'Built in Rust inside the door'. It is Tom's decision, and ADR-019 moves the broker into lys, so the move needs his word.
- ADR-002 — The revolver is the first consumer and the broker replaces the account pool file. R1's call site and the pool consumers are its baseline.
- ADR-003 — Every grant is human-rooted, and the delegation schema is explicitly not settled.
- ADR-004 — Every project stands alone, and an engine without the broker reads its own pool file. This is why no engine becomes structural and why CN4 refuses to make Cambium a required runtime.
- ADR-005 — One PostgreSQL database, which is DIRECTORY-002's install and a blocker for the code rows.
- ADR-011 — The proposed lifecycle states behind CONFORMANCE rows 3.2 and 3.4. Their review is the blocker the words move into R4's own Blocked by.
- secrets CN4 — 'Resolve standalone broker-host ownership before dispatch' leaves the lys-versus-cambium-door host open.
- secrets CN2 — No SECRETS-002 row is dispatched before Waffles reviews it. The draft adds CN5 for SECRETS-003.
- SECRETS-002 R7 SEC_REVOKE_FRESHNESS — Proposed, with either await or refuse. The words fix it as whatever the landed DIRECTORY-006 R4 check does, plus zero forwarded calls.
- SECRETS-002 R9 — 'The time window lives in the signed delegation', which lys/delegation/v1 cannot carry (mod.rs:218-221).
- DIRECTORY-006 R1-R4 — R1: grant contract with a time window, proposal only. R2: delegation admission. R3: signed grant events. R4: revocation, expiry and freshness. All in crates/lys-identity. R6 waits on DIRECTORY-005.
- draft ADR-019 (4babcfe, unlanded) — Broker in lys as crates/lys-secrets, decided_by 'The lead's answers', supersedes [].
- draft ADR-020 (4babcfe, unlanded) — The lease is a broker record, and 'The signed delegation keeps the time window SECRETS-002 R9 gives it', which the words correct to the lys-identity grant's window.
- CLAUDE.md 'Wire formats are forever' / 'A test needs a second party' — A new format goes alongside and never mutates the old one. Count what fired, use positive controls, and require that a second party be arranged, which underlies the two-author check.

### What was measured

- Decisions on main: 18 (ADR-001 to ADR-018). ADR-019 and ADR-020 are absent from main and exist only in draft 4babcfe.
- Secrets cluster on main: 2 briefs (SECRETS-001, SECRETS-002), checklist C1-C18, stories S1-S14
- SECRETS-002: 9 requirements, 57 acceptance criteria, 53,012 bytes of JSON
- Draft SECRETS-003 at 4babcfe: 8 rows (R1-R8), 330 JSON lines, 11 checklist items (C19-C29), 4 stories (S15-S18), depends_on [], plus a 1,101-line PAGE.md
- Draft hours: R1 3, R2 6, R3 14, R4 12, R5 6, R6 5, R7 7, R8 4: 57 h total, 64 h ceiling. The earlier figure it replaced was 40 h over 7 rows with a 45 h ceiling.
- Design gate (sh scripts/design/gate.sh): Exit 0 on main at 1756688. The draft at 4babcfe also reports 'Coverage clean' for secrets (29 items, 18 stories, 3 briefs).
- IDENTITY-003 files under docs/design/identity/briefs in this tree: 0. The directory holds only CONTEXT-001 and IDENTITY-001, and docs/design/identity has no design.json, so the gate skips it.
- 'Rust inside the door' non-goal reason in secrets/design.json: Line 51 on main and line 54 in the draft. Line 54 on main is the proxy-handle login non-goal.
- Files DIRECTORY-002 / 003 / 006 list to create that exist today: 0 of 28, 0 of 15 and 0 of 28. crates/lys-identity does not exist.
- Workspace crates: 6 (lys, lys-anchor, lys-anchor-cli, lys-core, lys-home, lys-log-store). No lys-secrets and no lys-identity.
- Empty AAD sites in lys-core seal: 2: sealed_envelope.rs:182 (seal) and :254 (open)
- manifold revolver at 3df5ac5: turned() at line 158; the next-account assignment at line 172 (verified by git show)
- manifold pool module at 3df5ac5: 7 files under crates/manifold-supervisor/src/values/pool/. 43 .rs files match 'pool' case-insensitively.
- manifold HEAD versus R1 pin: HEAD 456efea671 is ahead of the pinned 3df5ac5f64.
- ast-grep config or rule in lys: 0. There is no sgconfig.yml and .land/gates.sh has no ast-grep leg.
- tracing-subscriber dependency in lys: 0. Only tracing = "0.1" in the workspace, used by lys-core.
- SEC3_STORE_RACE total cases: 10: 8 requests at k=3 plus 2 requests at k=1
- Author of the draft commits b45bff6 and 4babcfe: 'aion brief loop <aion@localhost>', a single author for the whole workflow
- Code rows in the draft that list tests/support/mod.rs: 2 of 6 (R3 creates it, R4 modifies it). R5 to R8 use doubles from it but do not list it.

### What it means for the other projects

- manifold — Read only. R1 cites crates/manifold-node/src/seat/revolver.rs:158 and 172, the 7 pool/ files and seat_document/record.rs at pinned 3df5ac5. HEAD has moved to 456efea, so the baseline stays pinned. No manifold file becomes structural (ADR-004).
- cambium — The door is either the broker's host (ADR-001, SECRETS-002) or only a consumer (ADR-019, pending Tom). Either way no cambium file is edited by this brief, and R1 searches it for credential paths and records the count, including 0.
- aion — R1 searches it for pool-file and credential-path consumers, with counts. Its brief loop commits under one author, 'aion brief loop', which would fail R2's two-author check unless the review is committed under another identity.
- argus — Searched in R1 for pool-file and credential paths, with the count recorded, including 0. No change.
- haematite — Searched in R1 for pool-file and credential paths, with the count recorded, including 0. No change.
- method — The design-system scripts (render, validate, coverage) are the secrets design.json gate legs through DS2_METHOD. The brief must render and validate under them and under scripts/design/gate.sh.

### The decisions it stands on

- ADR-001 (honour) — It stays in force and unedited. ADR-019 is recorded as amending it only once Tom gives his word, and the ledger is append-only.
- ADR-002 (honour) — The revolver as first consumer is R1's baseline subject and R8's delivery.
- ADR-003 (honour) — Handles are issued only under person-rooted grants, and the delegation schema stays open.
- ADR-004 (honour) — No engine becomes structural, and an engine without the broker reads its own pool file.
- ADR-005 (honour) — DIRECTORY-002's single PostgreSQL database is a code-row blocker.
- ADR-011 (honour) — The lifecycle-state review (CONFORMANCE 3.2 and 3.4) blocks R4 and is not decided here.
-  (new) — ADR-019: the broker lives in lys as crates/lys-secrets and Cambium consumes it. It amends ADR-001 and waits on Tom's word, so it is written as proposed rather than decided.
-  (new) — ADR-020: the lease is a broker record under the audit log. Its not_after never widens the lys-identity grant's time window (DIRECTORY-006 R1), which replaces the draft's claim that the signed delegation keeps the window.

### What it requires

- docs/design/secrets/briefs/SECRETS-003.json and .md exist on main, and sh scripts/design/gate.sh exits 0.
- git diff over docs/design/secrets/briefs/SECRETS-002.json and .md against 1756688 is empty.
- The SECRETS-003 commits change no path under crates/ and no Cargo.toml or Cargo.lock.
- SECRETS-003 R1 matches the draft's R1 at 4babcfe (call site revolver.rs:172 at manifold 3df5ac5, 5 repositories searched with counts).
- Each of R3 to R8 names DIRECTORY-006 R1, R2, R3 and R4 individually as blockers, never 'DIRECTORY-006 landed'. Each blocker carries a runnable check: a file path, a command or a commit to look for.
- R4's Blocked by names R3. Each of R5, R6, R7 and R8 names R4 (and through it R3).
- The CONFORMANCE 3.2 and 3.4 review appears in R4's own Blocked by and not in the brief-level blocked_by.
- CONTRACT.md's required sections include Key custody, covering which key, where it is kept, how the store is unlocked and how it is rotated. R3 has one check that the key is absent from the store directory and one that opening without it is refused by name.
- Every canary leg (R3 to R8) also searches every file under the test's store and log directories and expects 0 matches, with a positive control that plants the value and finds exactly 1.
- The Audit line section states the line carries a handle id or digest and never the raw handle. The Handle section states the handle is stored hashed and how the proxy authenticates its presenter. A canary plants a handle value.
- R3 has a refusal check for ciphertext swapped between two store entries.
- The brief states that either CONTRACT.md gives a broker-internal shape for the audit signer and schema and the sealed-record binding, reviewed adversarially, or the rows depending on them stay blocked and say so.
- ADR-020's text names the lys-identity grant's window (DIRECTORY-006 R1) as the limit, and the brief lists each SECRETS-002 criterion that still waits on it.
- ADR-019 is in decisions.json, recorded as amending ADR-001. secrets/design.json's 'built in Rust inside the door' non-goal reason is corrected. The brief says the amendment waits on Tom's word.
- SEC_REVOKE_FRESHNESS in R4 calls the landed DIRECTORY-006 permission check, asserts its one behaviour, and asserts 0 forwarded calls.
- SEC3_OAUTH_RACE asserts exactly 1 provider refresh for 3 simultaneous calls.
- Each code row's criterion mapping names a report path, or its criterion ids appear in test names, so that an rg search proves the mapping.
- The brief names a tracing subscriber in crates/lys-secrets/tests/support as the log capture, one redacting type for every secret byte, and an ast-grep rule or a counting test that enforces it.
- R4 and R5 list crates/lys-secrets/Cargo.toml and Cargo.lock. Every row whose test doubles live in tests/support/mod.rs lists that file.
- R2's acceptance checks the author and the reviewer as two different commit authors in git log. SEC3_STORE_RACE states its total case count.
- The open questions the lead has not answered are recorded with both readings. Each product_decision the lead answers is recorded with the reading it closes.

### What must not change

- SECRETS-002.json and SECRETS-002.md are not edited.
- No code, test, Cargo file or file under crates/ changes. This unit is documents only.
- No credential, token or key value in any document.
- lys-core and its published formats (lys/delegation/v1, lys/sealed-envelope/v1) are unchanged. A signed lease would only ever be a new version alongside, with its own adversarial review.
- No engine (manifold, aion, cambium) becomes structural, and no file outside the lys repository is edited.
- Nothing is added to docs/design/identity/briefs, and the IDENTITY-003 draft does not land.
- Existing ADR entries are not rewritten, because the ledger is append-only. ADR-001's text stays as Tom decided it.
- R1's content stays as the lead found it sound.
- The settled points (the login at spawn as the one exception, OAuth refresh and spend caps in scope, SECRETS-002's open points) are not reopened.
- No row is dispatched before the lead's review.

### What we must put in place first

- Bring the reviewed draft into the working tree's reach: 4babcfe and b45bff6 exist only in /Users/tom/Developer/ablative/stack/lys on origin/brief/secrets/604f95d0-4495-4a47-b6f1-deeacb2177cb, not in this clone.
- The lead's answers to the product decisions: the host and lease as open versus ADR-recorded, the hours, depends_on, whether code rows block on Tom's word, and what a broker-internal shape settles.

### The risks

- DIRECTORY-006 R1's time window and R4's freshness choice are both marked proposal or unsettled. Naming the grant's window as the limit and fixing SEC_REVOKE_FRESHNESS to the landed behaviour both hang on reviews that have not happened, so the waiting criteria may never unblock as written.
- The workflow commits every artifact as 'aion brief loop <aion@localhost>'. R2's two-commit-author check would fail structurally, or pass only if the review commit is made under another identity.
- Putting DIRECTORY-002 or 003 in depends_on would let the dispatcher hold back the documents rows R1 and R2, which the words say only the code rows wait for.
- If Tom keeps the broker inside the door, every code row's file list (crates/lys-secrets) and ADR-019 are void. This work should not stop meanwhile (rule 6), but the code rows would have to be rewritten.
- The corrections add scope, so the draft's 57 h and 64 h ceiling are likely understated.
- The contract may be unable to bind a sealed record to its name and owner without AAD, which sealed_envelope.rs:182 and :254 leave empty. The honest outcome may then be R7 staying blocked, or needing a lys-core sealed-envelope version alongside, which is out of this brief's bounds.
- An ast-grep rule would not run in .land/gates.sh, which has no ast-grep leg. Choosing it without adding a leg is a control that never fires.
- The canary on-disk search over an encrypted store can pass trivially. The positive control must plant the value somewhere the search can see it, or the leg proves nothing.
- Line-number citations drift. design.json:54 is already line 51 on main, and manifold HEAD has moved past the 3df5ac5 pin, so citations must stay pinned to commits.
- Contradiction left in the text: the words ask for host and lease as open questions, yet they amend ADR-019 and ADR-020. A brief that does both will fail review.

### Still open

- Are the broker's host and the lease's form recorded as open questions with both readings, or as ADR-019 (proposed, awaiting Tom) and ADR-020 (decided), as the later corrections assume? The sentence of the words it stands on: "The brief records each as an open question with both readings.". Why only the lead can settle it: The later paragraphs of the words treat ADR-019 and ADR-020 as standing and ask for them to be amended. On main, docs/design/decisions.json ends at ADR-018, and the draft's entries say decided_by 'The lead's answers'. Recording them as open leaves every code row's file wall (crates/lys-secrets) and the lease shape undecided. Recording them as ADRs settles both.
- Do the draft's hours (57 h, 64 h ceiling) stand, or are they re-costed for the work the corrections add? The sentence of the words it stands on: "Ask the lead, do not decide: whether the broker lives in lys (the draft's crates/lys-secrets) or in the cambium door SECRETS-002 names, which CN4 leaves open; whether the lease is a broker record or a signed format; whether the draft's hours stand.". Why only the lead can settle it: The corrections add work: key custody, on-disk canary searches, a ciphertext-swap refusal, handle hashing, the redacting-type enforcement and a single-refresh OAuth race. That changes the per-row estimates the lead dispatches against.
- Does SECRETS-003's brief-level depends_on name DIRECTORY-002 and DIRECTORY-003, which would hold the documents rows R1 and R2 back too, or stay [] with every DIRECTORY blocker carried in the code rows' own Blocked by? The sentence of the words it stands on: "SECRETS-003.json carries depends_on [] while DIRECTORY-006 uses that field: name the DIRECTORY-006 requirements this brief waits on as R1 to R4 of 006, never the whole of 006, whose R6 waits on 005; give each blocker a check a stranger can run, a file, a command or a landed commit to look for.". Why only the lead can settle it: brief.schema.json defines depends_on as whole brief IDs the dispatcher orders by, so it cannot name '006 R1-R4'. A non-empty depends_on blocks R1 and R2, which the words say only the code rows wait for.
- Do R3 to R8, whose files all sit in crates/lys-secrets, carry Tom's word on the ADR-019 amendment as a blocker, or may they be dispatched on the lead's acceptance while it is pending? The sentence of the words it stands on: "ADR-001 is Tom's decision; the lead puts the move to him herself, and the brief says the amendment waits on his word.". Why only the lead can settle it: If Tom keeps the broker inside the door, every code row's file list moves repositories. Blocking on his word changes when any code can start.
- Does a broker-internal audit-line signer and schema, and a sealed-record name and owner binding, stated in CONTRACT.md count as settling points SECRETS-002 records open, or only as a local shape that leaves those points open? The sentence of the words it stands on: "Settled on main, not reopened: the seat's own login reaches its process at spawn as the one named exception (R5); OAuth refresh and spend caps are in scope (R4, R9); every point it records open stays open.". Why only the lead can settle it: SECRETS-002 and the draft's R2 list (3) the audit signer, (4) the leaf schema and (9) the sealed-record binding as open and undecided. The corrections say the code rows must settle them. Both cannot hold unless the lead says what a broker-internal shape settles.

### The units beyond the first

- SECRETS-003 R1: record the baseline in BASELINE.md — A dispatched documents row of its own, after the lead's review.
- SECRETS-003 R2: CONTRACT.md with key custody, and the adversarial review by a second author — It needs a different committing identity and the lead's acceptance before any code.
- SECRETS-003 R3: store, handles and leases in crates/lys-secrets — The first code row. It is blocked on DIRECTORY-002, DIRECTORY-003 and DIRECTORY-006 R1 to R4.
- SECRETS-003 R4: proxy, rotation and cancellation rule — It follows R3 and is also blocked by the CONFORMANCE 3.2 and 3.4 review.
- SECRETS-003 R5 to R8: OAuth refresh, login at spawn, sealed records, next account — Each is its own row after R4.
- Tom's ruling on ADR-019 (broker in lys, amending ADR-001) — His decision to make. The lead puts it to him, and the ledger entry moves from proposed to decided on his word.
- A later Lys brief for the screen and API legs of SEC_USE_NOT_LEND, SEC_PEOPLE_ONLY and SEC_REVOKE_STATES — The draft leaves these legs as not delivered here.
- A lys-core sealed-envelope version with AAD beside v1, if the contract cannot bind a sealed record otherwise — A new wire format, which needs its own brief and adversarial review under 'Wire formats are forever'.

### The smallest complete shape

One documents-only commit set in the secrets cluster that passes sh scripts/design/gate.sh:
- SECRETS-003.json and .md: R1 as reviewed; R2 to R8 with every correction; per-row blockers for DIRECTORY-006 R1 to R4, each with a runnable check; the R3, R4, R5-to-R8 order; hours.
- The matching design.json, DESIGN.md, checklist, stories and their rendered markdown.
- decisions.json gains ADR-019 (amending ADR-001, awaiting Tom) and ADR-020 (the grant's window as the limit).
- One roadmap row.
SECRETS-002 stays untouched and nothing under crates/ changes.

## The roadmap row

- **RM-013** — Build the secrets broker in lys from a baseline, a reviewed contract and counted code rows (feature, idea)
- Summary: SECRETS-003, beside SECRETS-002 and without editing it: a documents-only baseline recorded from source, a contract with key custody and an adversarial review by a second commit author before any code, and code rows in crates/lys-secrets, ordered R3, then R4, then R5 to R8, that deliver SECRETS-002's R1 to R9 with counted acceptance legs, re-costed hours per row, and DIRECTORY-002, DIRECTORY-003 and DIRECTORY-006 R1 to R4 as blockers with runnable checks. The broker's home in lys is ADR-019, proposed as an amendment to ADR-001 and waiting on Tom's word; the lease is a broker record bounded by the lys-identity grant's window (ADR-020).
- Asked by: tom on 2026-09-26T20:17:27+10:00
- Context: The secrets broker card on the Lys board: the lead's review of the SECRETS-003 draft published at 4babcfe, with the corrections to R2 to R8, surveyed against lys main at 1756688. The lead's answers: the broker's host and the lease are recorded as ADR-019 (proposed, amending ADR-001, waiting on Tom's word) and ADR-020 (decided); the hours are re-costed; depends_on stays empty with every directory blocker in the code rows' own Blocked by; the code rows wait on Tom's word on ADR-019; and a broker-internal audit signer, leaf schema and sealed-record binding in the contract settle those points for the broker and leave them open estate-wide.
- Quote: The unlanded draft IDENTITY-003 under docs/design/identity/briefs does not land; that directory sits outside the gate. The broker's brief on main is the secrets cluster: docs/design/secrets/briefs/SECRETS-001.json (done) and SECRETS-002.json, R1 to R9 with SEC_ identifiers, anchored to ADR-001 (secrets sit behind a handle the door swaps for the credential), ADR-002 (the token revolver is the first consumer), ADR-003 and ADR-004.

This brief is SECRETS-003, a brief of its own beside SECRETS-002, so what has landed stays stable; SECRETS-002 is not edited. It adds, in docs/design/secrets/, what the draft carries and main lacks: a documents-only baseline row recording from source the revolver's next-account call site, every consumer of the account pool file, every credential path into a seat today classed as the login exception, a proxied credential or neither, and what lys/delegation/v1 and the sealed envelope can carry for a handle, a lease and a sealed record, by file and line; a contract row before any code stating the invariant, the handle, the lease with its atomic use step and retry outcome, the cancellation rule, the SpiceDB relations and the audit line fields, with an adversarial review recording every attack tried (a replayed or forged handle, a use spent or counted twice, an admitted call outliving the rule, a credential in a log, a sealed key read, a stale SpiceDB answer) and the clause defeating each; counted acceptance legs on every code row (each refusal leg, a forced race on a k-use lease admitting exactly k, a redaction canary with a positive control, kill and reopen at each append boundary, each in-flight outcome its own test); hours per row; dependencies on DIRECTORY-002, 003 and 006 blocking the code rows.

Settled on main, not reopened: the seat's own login reaches its process at spawn as the one named exception (R5); OAuth refresh and spend caps are in scope (R4, R9); every point it records open stays open.

Boundaries: documents only, no code; no credential value; lys-core and its published formats unchanged, a signed lease format only as a new version alongside with its own adversarial review; no engine structural; no row dispatched before the lead's review; the gate is scripts/design/gate.sh.

Ask the lead, do not decide: whether the broker lives in lys (the draft's crates/lys-secrets) or in the cambium door SECRETS-002 names, which CN4 leaves open; whether the lease is a broker record or a signed format; whether the draft's hours stand. The brief records each as an open question with both readings.

The lead read the brief published at 4babcfe on 26 September and found the baseline R1 sound and holes in R2 to R8; every one of these corrections goes in, and R1 stays in this brief.

Blockers a build can act on. SECRETS-003.json carries depends_on [] while DIRECTORY-006 uses that field: name the DIRECTORY-006 requirements this brief waits on as R1 to R4 of 006, never the whole of 006, whose R6 waits on 005; give each blocker a check a stranger can run, a file, a command or a landed commit to look for.

The order of the code rows. R3 to R8 do not block on each other today, yet R4 edits R3's lib.rs, R6 and R7 use R4's revocation states, and R5 and R8 build on R4's proxy: state the order R3, then R4, then R5 to R8, in each row's own Blocked by.

The CONFORMANCE 3.2 and 3.4 review blocks only at brief level: put it in R4's own Blocked by.

Key custody. Nobody holds the store's encryption key: add a key-custody section to CONTRACT.md saying which key, where it is kept, how the store is unlocked and how the key is rotated; add R3 checks that the key is not in the store directory and that opening the store without it is refused by name.

Canaries on disk. Every canary check also searches every file under the test's store and log directories, expecting zero matches, with a positive control that plants the value and finds it.

The audit line and the handle. The audit line carries a handle id or digest only, never the raw handle, since the transparency log can be read and replayed; the handle is stored hashed; the canary plants a handle value. The Handle section says how the proxy authenticates whoever presents a handle.

Points R2 keeps open that the code rows must settle: the audit line's signer and schema, and how a sealed record is bound to its name and owner, since sealed_envelope.rs:182 has an empty AAD; R3's store gains a refusal check for ciphertext swapped between two entries. Either the contract states a broker-internal shape, reviewed adversarially, or those rows stay blocked and say so.

The delegation's time window. ADR-020 says the signed delegation keeps it, v1 has no window, and DIRECTORY-006 R1 puts it in the lys-identity grant: name the grant's window as the limit, amend ADR-020 to match, and list which criteria still wait on it.

ADR-001 says the broker is built in Rust inside the door and ADR-019 moves it into lys with supersedes []: record ADR-019 as amending ADR-001 and fix design.json:54. ADR-001 is Tom's decision; the lead puts the move to him herself, and the brief says the amendment waits on his word.

SEC_REVOKE_FRESHNESS has one fixed expected behaviour: R4 calls the landed DIRECTORY-006 permission check and asserts that behaviour and zero forwarded calls.

SEC3_OAUTH_RACE asserts that the provider sees exactly one refresh for three simultaneous calls.

The criterion mapping is searchable: "the row's report maps each criterion" names the report path, or the criterion ids sit in the test names so a search proves the mapping.

The canary scope is countable: say how logs are captured, a tracing subscriber in tests/support; keep every secret byte in one redacting type; enforce that with an ast-grep rule or a counting test.

File lists. R4 and R5 list crates/lys-secrets/Cargo.toml and Cargo.lock; tests/support/mod.rs is listed on every row whose test doubles live there.

Two different identities are checked as two different commit authors in git log, and SEC3_STORE_RACE states its total case count.
- Cluster: secrets; briefs: SECRETS-003
- Notes: Further units, not written: SECRETS-003 R1: record the baseline in BASELINE.md; SECRETS-003 R2: CONTRACT.md with key custody, and the adversarial review by a second author; SECRETS-003 R3: store, handles and leases in crates/lys-secrets; SECRETS-003 R4: proxy, rotation and cancellation rule; SECRETS-003 R5 to R8: OAuth refresh, login at spawn, sealed records, next account; Tom's ruling on ADR-019 (broker in lys, amending ADR-001); A later Lys brief for the screen and API legs of SEC_USE_NOT_LEND, SEC_PEOPLE_ONLY and SEC_REVOKE_STATES; A lys-core sealed-envelope version with AAD beside v1, if the contract cannot bind a sealed record otherwise.

## The design

---
type: design
cluster: secrets
title: Secrets broker: agents hold handles, never credentials
---

# Secrets broker: agents hold handles, never credentials

> **Cluster:** secrets

## Intention

Every identity, person or agent, uses credentials through a short-lived handle the door swaps for the real credential, so no credential ever reaches an agent and revoking is instant.

## Problem

Credentials live in files on each machine and in each worker's environment. An agent that holds a key cannot have it taken back, usage cannot be attributed in one place, and resting an account means copying a file to every machine.

## Solution

A broker in Rust in the lys repository, as the crate crates/lys-secrets of the standalone identity platform (ADR-019, proposed as an amendment to ADR-001 and waiting on Tom's word), which Cambium consumes through the door and never hosts: an encrypted store of real credentials, handles bound to identities, a proxy that checks SpiceDB, swaps the handle, forwards the call and writes one audit line, rotation across several accounts under one handle, OAuth refresh at the proxy, the seat's own login at spawn, sealed records tagged in SpiceDB, and leases counted by uses, time window and spend. A lease is a broker record under the audit log, read only by the broker; its not_after never extends past the window of the lys-identity grant it counts against (ADR-020). The token revolver is its first consumer. SECRETS-002 states what the broker does, requirement by requirement. SECRETS-003 builds it: a documents-only baseline recorded from source, a contract with its adversarial review before any code, and code rows that each deliver named SECRETS-002 requirements with counted acceptance legs, an estimate in hours, and the directory briefs they wait on.

## Principles

- **P1** — A handle, never a credential: the credential never leaves the server.
- **P2** — Revoking is dropping the handle; what was already handed to a process needs its own revocation story, stated per case.
- **P3** — Every use writes one audit line naming the seat, the handle, the real account and the time.
- **P4** — A key is used through the proxy, never read; only memories are read, in the smallest piece asked for.
- **P5** — Every grant traces back to the person who authorised it.
- **P6** — Permission to use never implies permission to lend. Pass-on rights are affirmative and recipient-specific; a missing prohibition is not a grant. Server-verified ownership is the affirmative may-lend route in docs/design/identity/CONFORMANCE.md at commit 1353c22 row 7.3; a display label is not proof of ownership.
- **P7** — Secret visibility, usage, delegation and revocation follow the same current human-rooted authority at every server seam, independent of how the caller reaches it.

## Decisions

- ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
- ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-019 — The secrets broker lives in lys, as crates/lys-secrets; Cambium consumes it and is never its home (amends ADR-001) — The broker lives in the lys repository as the crate crates/lys-secrets, part of the standalone identity platform, built in Rust. A person installs Lys to hold secrets; Cambium reaches the broker through the door as a consumer and is never its home. Rejected: the broker inside the cambium door that ADR-001 and SECRETS-002 name, which would make Cambium a runtime a person must install to keep a credential.
- ADR-020 — A lease is a broker record under the audit log, not a signed format — The lease is a broker record under the audit log: it counts uses, time and spend against a grant and is read only by the broker, so nothing a stranger verifies changes. The limit on a lease's time is the window of the lys-identity grant it counts against (DIRECTORY-006 R1): the lease's own not_after never extends past it. Rejected: a signed lease format, a new version beside lys/delegation/v1 with its own adversarial review; and reading the window as one the signed delegation keeps, which v1 cannot carry.

## Goals

- An implementation brief, SECRETS-002, covers every part of the temporary key model in the statement with numbered requirements and acceptance criteria.
- Every checklist item and user story of the broker is covered by a SECRETS-002 or SECRETS-003 requirement.
- The token revolver can take its next account from the broker instead of its own list.
- An implementation brief, SECRETS-003, carries a baseline row, a contract row with its adversarial review, and code rows that deliver SECRETS-002's R1 to R9 in crates/lys-secrets, each code row with counted acceptance legs, an estimate in hours, and DIRECTORY-002, DIRECTORY-003 and DIRECTORY-006 R1 to R4 as blockers, each with a check a stranger can run.

## Non-Goals

- OpenBao or another external secrets engine — Tom's model is a broker built in Rust (ADR-001); ADR-019, proposed and waiting on his word, amends where it is built, from inside the door to lys as crates/lys-secrets. An external engine is only reconsidered if credentials minted on demand are needed.
- The seat login through a proxy handle and base URL — Not tested with a subscription login; the statement keeps it open to prove later and nothing depends on it.
- The delegation schema — The statement says it is not settled.
- A signed lease format, a new version beside lys/delegation/v1 — ADR-020: the lease is a broker record read only by the broker; a lease a stranger verifies offline is a new brief with its own adversarial review.
- A release and demonstration row for the broker — SECRETS-003's words list its rows, and a release row is not among them; it would come from a later roadmap decision.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/secrets/briefs/SECRETS-002.json` | the secrets broker implementation brief | SECRETS-001 |
| `docs/design/secrets/briefs/SECRETS-002.md` | its rendered markdown | SECRETS-001 |
| `docs/design/secrets/design.json` | this design; the structure gains a row for every path SECRETS-002 names | SECRETS-001 |
| `docs/design/secrets/DESIGN.md` | rendered design | SECRETS-001 |
| `docs/design/secrets/checklist.json` | checklist; gains the broker implementation items | SECRETS-001 |
| `docs/design/secrets/CHECKLIST.md` | rendered checklist | SECRETS-001 |
| `docs/design/secrets/stories.json` | stories; gains the broker implementation stories | SECRETS-001 |
| `docs/design/secrets/USER-STORIES.md` | rendered stories | SECRETS-001 |
| `crates/lys-core/src/delegation/mod.rs` | lys/delegation/v1; its docs record the handle and the lease's time window as applications of the delegation format (SECRETS-002 R1, R9) | SECRETS-002 |
| `docs/design/WIRE-FORMATS.md` | the lys wire-format register; records the handle, sealed-record and lease formats before any is signed (SECRETS-002 R1, R6, R9) | SECRETS-002 |
| `crates/lys-core/src/seal/mod.rs` | sealed envelopes; its docs record the sealed record as an application of lys/sealed-envelope/v1 (SECRETS-002 R6) | SECRETS-002 |
| `docs/design/secrets/briefs/SECRETS-003.json` | the broker build brief: baseline, contract and counted code rows in crates/lys-secrets | SECRETS-003 |
| `docs/design/secrets/briefs/SECRETS-003.md` | its rendered markdown | SECRETS-003 |
| `docs/design/secrets/BASELINE.md` | SECRETS-003 R1: the revolver call site, the pool file's consumers, every credential path into a seat classed, and what lys/delegation/v1 and lys/sealed-envelope/v1 can carry, by file and line | SECRETS-003 |
| `docs/design/secrets/CONTRACT.md` | SECRETS-003 R2: the invariant, the key custody, the handle, the lease, the cancellation rule, the SpiceDB relations, the sealed-record binding and the audit line | SECRETS-003 |
| `docs/design/secrets/reports/SECRETS-003-adversarial-review.md` | SECRETS-003 R2: every attack tried against the contract and the clause defeating each | SECRETS-003 |
| `Cargo.toml` | the workspace manifest; gains crates/lys-secrets as a member (SECRETS-003 R3) | SECRETS-003 |
| `Cargo.lock` | the workspace lockfile (SECRETS-003 R3, R4 and R5) | SECRETS-003 |
| `crates/lys-secrets/Cargo.toml` | the broker crate's manifest (SECRETS-003 R3, R4 and R5) | SECRETS-003 |
| `crates/lys-secrets/src/lib.rs` | module declarations and the crate's invariant docs | SECRETS-003 |
| `crates/lys-secrets/src/error.rs` | the broker's error type; no credential byte in any variant | SECRETS-003 |
| `crates/lys-secrets/src/secret.rs` | the one redacting type that holds every credential byte and every raw handle byte | SECRETS-003 |
| `crates/lys-secrets/src/store.rs` | the encrypted store of real credentials | SECRETS-003 |
| `crates/lys-secrets/src/key_rotation.rs` | store-key rotation: every entry resealed to the new key, the old key refused by name, one audit line naming both key ids | SECRETS-003 |
| `crates/lys-secrets/src/handle.rs` | handles bound to one identity, issued, resolved and dropped | SECRETS-003 |
| `crates/lys-secrets/src/lease.rs` | the lease record: uses, time window, spend, the atomic use step and the retry outcome | SECRETS-003 |
| `crates/lys-secrets/src/audit.rs` | audit lines appended through lys-log-store | SECRETS-003 |
| `crates/lys-secrets/schema/secrets.zed` | the SpiceDB relations for handle use and sealed-record reads | SECRETS-003 |
| `crates/lys-secrets/src/proxy.rs` | the check, swap, forward and audit line | SECRETS-003 |
| `crates/lys-secrets/src/rotation.rs` | the next real account in turn under one handle | SECRETS-003 |
| `crates/lys-secrets/src/in_flight.rs` | the register of admitted calls and the cancellation rule applied at a drop | SECRETS-003 |
| `crates/lys-secrets/src/oauth.rs` | OAuth refresh at the proxy | SECRETS-003 |
| `crates/lys-secrets/src/spawn_login.rs` | the seat's own login answered at spawn and the record of which went to which seat | SECRETS-003 |
| `crates/lys-secrets/src/sealed.rs` | sealed records read by name under a relation check; keys refused for reading | SECRETS-003 |
| `crates/lys-secrets/src/next_account.rs` | the revolver's ask for its next account | SECRETS-003 |
| `crates/lys-secrets/tests/store_leases.rs` | R3's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/secret_scope.rs` | SEC3_SECRET_SCOPE: the counting test that keeps every secret byte in the redacting type | SECRETS-003 |
| `crates/lys-secrets/tests/proxy.rs` | R4's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/oauth.rs` | R5's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/spawn_login.rs` | R6's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/sealed.rs` | R7's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/next_account.rs` | R8's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/support/mod.rs` | shared test doubles (the upstream, the provider, SpiceDB, the canary and the kill points) and the tracing subscriber that captures every log line | SECRETS-003 |

## Inventory

- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority: 'Secrets: the temporary key model' through 'What revoking reaches', 'Two rules from Tom, 13:55', 'Lifecycle, budgets and leases' and 'Where it lives: lys'
- `docs/design/decisions.json` — the project decision ledger this cluster anchors to
- `crates/` — the five lys crates (lys, lys-core, lys-anchor, lys-anchor-cli, lys-log-store): sealed envelopes, the delegation format, the log. The door repository, where the statement places the store and the proxy, is the cambium checkout, apps/cambium in the ablative estate on this Mac; it is read, never written, by this brief
- `docs/design/identity/CONFORMANCE.md` — Committed behaviour source: docs/design/identity/CONFORMANCE.md at commit 1353c22. Bind the SEC_* acceptance IDs to its rows before dispatch; mock-up sample data is not enforcement evidence.

## Constraints

- **CN1** — No credential, token or key value is ever written, read or quoted in any document of this cluster.
- **CN2** — No row of SECRETS-002 is dispatched before Waffles has reviewed it.
- **CN3** — Every path written in a document of this cluster is relative to the repository root, whatever directory a session starts in; a command runs from its own tree and spells its paths from there.
- **CN4** — Resolve standalone broker-host ownership before dispatch. Earlier SECRETS-002 Cambium path proposals and empty door-owned file walls are not authority to make Cambium a required runtime service; ADR-004 remains binding. Answered by ADR-019, proposed: the broker lives in lys as crates/lys-secrets and Cambium consumes it and is never its home; it amends ADR-001 and is settled when Tom gives his word, which every SECRETS-003 code row waits on.
- **CN5** — No row of SECRETS-003 is dispatched before the lead has reviewed it; no code row starts before the baseline row and the contract row are accepted.


---
type: brief
id: SECRETS-001
cluster: secrets
title: Write the secrets broker implementation brief
---

# SECRETS-001: Write the secrets broker implementation brief

> **Cluster:** secrets
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> **Checklist:**
> - C1 — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend.
> - C2 — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a SECRETS-002 requirement.
> - C3 — The rendered markdown of this cluster matches its JSON.
> **Stories:**
> - S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time.

## Purpose

Agents get a short-lived, limited handle to a credential, never the credential itself, and the token revolver is the first user. This brief produces the implementation brief for that broker, so its rows can be reviewed and built one at a time. Documents only.

## Task

Read docs/design/identity/STATEMENT-2026-09-22.md, the sections 'Secrets: the temporary key model' through 'What revoking reaches', 'Two rules from Tom, 13:55', 'Lifecycle, budgets and leases: the discussion of 14:00' and 'Where it lives: lys', and the decisions ADR-001 to ADR-004 in docs/design/decisions.json. Read the source the design inventory names before assigning any file: the lys crates and the door repository. Write SECRETS-002 in the design-system brief form, add the broker's checklist items and user stories to this cluster, and add a structure row to design.json for every path SECRETS-002 names. Every path written in a document of this cluster is relative to the repository root, even though the session starts in docs/.

## Requirements

### R1: Author the implementation brief SECRETS-002

THE SYSTEM SHALL have a brief docs/design/secrets/briefs/SECRETS-002.json, valid against the design-system brief schema, with one requirement per part of the temporary key model named in C1. Each requirement SHALL state its behaviour in the statement's own terms, list acceptance criteria that can be checked against code and tests, and name the files it creates, modifies or deletes. The repository that owns each file SHALL be grounded in the statement and in the source the inventory names, cited by path and line, and never assumed: the statement places the store and proxy in the door ('Secrets: the temporary key model') and the handle and sealed-record formats in lys ('Where it lives: lys': lys sealed envelopes and the delegation format, applied). A file owned by lys is listed in the requirement's files and gains a structure row. A file owned by the door repository is named in the requirement's spec with its owner and citation and is NOT listed in files and gains NO structure row, because no root naming the door repository is set for this round; it moves into files when that root is set. A requirement whose owner the sources do not settle is recorded as open. Where the statement leaves a point open (the delegation schema; whether revoking a login token at the provider fails the seat's next call; the proxy-handle login path), SECRETS-002 SHALL record it as open and SHALL NOT settle it. design.json SHALL gain a structure row, with brief SECRETS-002, for every path SECRETS-002 names.

**Acceptance:**
- docs/design/secrets/briefs/SECRETS-002.json exists and validate.py reports it valid.
- SECRETS-002 has a requirement for each of the eight parts named in C1, each with at least one acceptance criterion and at least one file path.
- Every file SECRETS-002 names carries its owning repository with a citation to the statement or the source; lys-owned files are in files and structure, door-owned files are in the spec text only, and no structure row or files entry in this cluster carries a root token.
- The cancellation rule for calls already admitted by the proxy is a requirement of its own with its own acceptance criteria.
- Each point the statement leaves open appears in SECRETS-002 as open and is not decided there.
- check-coverage.py reports every SECRETS-002 path present in design.json structure.
- No file in the cluster contains a credential, token or key value.

**Files:**
- create: docs/design/secrets/briefs/SECRETS-002.json
- create: docs/design/secrets/briefs/SECRETS-002.md
- modify: docs/design/secrets/design.json
- modify: docs/design/secrets/DESIGN.md

**Checklist:**
- C1 — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend.

**Stories:**
- S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 met: SECRETS-002.json exists, and validate.py prints 'design/secrets/briefs/SECRETS-002.json: OK [brief.schema.json]'. Row 2 met: C1's eight parts map to R1 to R9, with the revocation part split into R7 (the three cases) and R8 (in-flight cancellation). Each requirement has 3 to 7 acceptance criteria. R1, R6 and R9 list lys files in files. R2, R3, R4, R5, R7 and R8 name only door paths (for example crates/cambium-door/src/http/secrets_proxy.rs and crates/cambium-store/src/traits/secrets.rs), in their spec text, as R1's own rule requires. Row 3 met: every path has an owner and a citation to the statement by line (for example :25, :27, :82) or to the source. Lys paths are in files and structure. Door paths are named in the spec with the door repository (cambium, read at cbcd2cc9d) and cited against crates/cambium-store/src/traits/mod.rs, crates/cambium-door/src/http/router.rs and crates/cambium-door/src/http/agent_seat.rs:13-15. No files entry or structure row carries a root token. Row 4 met: the cancellation rule is R8, with three acceptance criteria of its own. Row 5 met: the delegation schema is recorded OPEN in R1 and R9. Provider revocation of a login token is OPEN in R5 and R7. The proxy-handle login path is OPEN in R5. Points the source raised are also recorded open: the seat/speaks-for role has no consumer (crates/lys-core/src/delegation/artifact.rs:242-254); v1 has no expiry (crates/lys-core/src/delegation/mod.rs:218-221); sealed-envelope/v1 seals with an empty AAD (crates/lys-core/src/seal/sealed_envelope.rs:180-183, docs/design/WIRE-FORMATS.md:18); the audit line's signer (statement :80 against agent_seat.rs:13-15) and its schema (:187); the file for the revocation fold; the revolver's worker-side owner; and the cancellation rule itself until its review. Row 6 met: check-coverage.py exits 0 with no structure failure. Row 7 met: the cluster holds no credential values, and a grep for common token shapes found nothing.
- Deviation: (1) Six requirements (R2, R3, R4, R5, R7, R8) have empty files arrays. They touch only door-owned files, which R1's spec says must be named in the spec text and not in files until a door root is set. Their 'at least one file path' is the door path in their spec. (2) validate.py exits 1 for the cluster because of design.json's existing `gate` field, which is not mine to change. Every document of this round validates OK. (3) The worker side of the token revolver lives in manifold (crates/manifold-node/src/seat/launcher.rs:31-40 at 3df5ac5f64). The design inventory names no engine repository, so I read it only to show that its owner is open.
- Files changed:
  - created: `docs/design/secrets/briefs/SECRETS-002.json` — The implementation brief: nine requirements R1 to R9 in EARS form. Each carries acceptance criteria, owners with citations and open points. Blocked by Waffles' review, SpiceDB beside the door, the door root not being set, the delegation schema, and the lys-core release that freezes lys/delegation/v1.
  - created: `docs/design/secrets/briefs/SECRETS-002.md` — The brief rendered from its JSON by render-cluster.py.
  - modified: `docs/design/secrets/design.json` — Gains three structure rows with brief SECRETS-002, one per lys path the brief lists in files: crates/lys-core/src/delegation/mod.rs, docs/design/WIRE-FORMATS.md, crates/lys-core/src/seal/mod.rs. The existing gate field is untouched.
  - modified: `docs/design/secrets/DESIGN.md` — Re-rendered with the three new structure rows.
- Checklist delivery:
  - [x] C1 — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend. — R1 to R9 cover the eight parts. The cancellation rule is R8, on its own.
- Story delivery:
  - [x] S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time. — Nine numbered requirements with acceptance criteria and blockers, so a reviewer can take them one row at a time; CN2 is repeated in blocked_by and boundaries.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] docs/design/secrets/briefs/SECRETS-002.json exists and validate.py reports it valid. — With DS2_METHOD=/Users/tom/Developer/projects/deno_rust/meridian/.meridian/design-system-v2, validate.py design/secrets prints 'design/secrets/briefs/SECRETS-002.json: OK [brief.schema.json]' and 'All 5 document(s) valid.' It exits 0, before and after the harden edits.
  - [x] SECRETS-002 has a requirement for each of the eight parts named in C1, each with at least one acceptance criterion and at least one file path. — The parts map to R1 to R9: handle and proxy (R1), rotation (R2), revolver (R3), OAuth (R4), spawn login (R5), sealed records (R6), revocation (R7, plus R8 for in-flight calls) and leases (R9). Each has 3 to 7 acceptance criteria. R1, R6 and R9 list lys paths in files. R2, R3, R4, R5, R7 and R8 name door paths in their spec text, as SECRETS-001 R1's spec requires, for example crates/cambium-door/src/http/secrets_rotation.rs in R2 and secrets_in_flight.rs in R8.
  - [x] Every file SECRETS-002 names carries its owning repository with a citation to the statement or the source; lys-owned files are in files and structure, door-owned files are in the spec text only, and no structure row or files entry in this cluster carries a root token. — The lys paths delegation/mod.rs, seal/mod.rs and WIRE-FORMATS.md are in files and in design.json structure. Door paths appear only in the spec text, each under the prefix 'Door-owned, named here only (the door repository ... read at cbcd2cc9d)'. I checked cbcd2cc9d in apps/cambium: agent_seat.rs:13-15 says the private key is never persisted, and the store traits' mod.rs has no secrets module. A script check found no files entry beginning with /, $, < or ~ and none containing '..'. The design.json structure paths are all relative to the repository root.
  - [x] The cancellation rule for calls already admitted by the proxy is a requirement of its own with its own acceptance criteria. — SECRETS-002 R8, 'State and enforce the cancellation rule for calls already admitted', has 3 acceptance criteria of its own and cites the statement at :55.
  - [x] Each point the statement leaves open appears in SECRETS-002 as open and is not decided there. — The delegation schema (:21) is OPEN in R1(a) and R9. Provider revocation of a login token (:56) is OPEN in R5(b) and R7(a). The proxy-handle login path (:45) is OPEN in R5(a). The audit event schema (:187) is OPEN in R1(e). After the harden fix, R7 no longer chooses how an engine learns of a revocation.
  - [x] check-coverage.py reports every SECRETS-002 path present in design.json structure. — check-coverage.py design/secrets reports 'Coverage clean: all items covered, briefs consistent.' and exits 0. The 3 lys paths in SECRETS-002's files are structure rows with brief SECRETS-002.
  - [x] No file in the cluster contains a credential, token or key value. — A grep -rE across docs/design/secrets/ found no matches (exit 1). It looked for sk-, ghp_, eyJ, AKIA, BEGIN key blocks, xox tokens and hex runs of 40 or more characters.
- Checklist verified: C1
- Stories verified: S1
- Issues:
  - R8 cited statement :156 for Chippy's release 2, 'demonstrate revocation and recovery after a crash'. That quotation is on line 155; line 156 is release 3.
  - R9 acceptance fixed its one-use race test at 'over 100 repetitions of the race'. That count traces to nothing in the statement, the design or the brief, and a race that is not forced may never fire.
  - R7 acceptance 2 wrote '(the test double engine receives the end request)'. That decides that the door sends the engine an end request. The statement (:56) says only that the engine that runs the seat ends it on its own.
- Fixes:
  - Changed R8's citation to docs/design/identity/STATEMENT-2026-09-22.md:155.
  - Removed the repetition count of 100 from R9's race criterion. Both requests are now held at the use check until both have arrived, so the race fires on every run.
  - Removed the end-request parenthetical from R7 acceptance 2. The criterion now asserts only that the door names the seat and ends no process.
  - Re-rendered SECRETS-002.md with render-cluster.py; a second run was byte-identical.

### R2: Record the broker's checklist items and stories and cover them

THE SYSTEM SHALL add to checklist.json a section of the broker's implementation items and to stories.json the personas and stories of the people and agents who use the broker (a person granting an agent access, an agent using a handle, an operator resting an account, a reviewer reading the audit). Every item and story added SHALL be named by at least one SECRETS-002 requirement, and the rendered markdown SHALL match the JSON.

**Acceptance:**
- check-coverage.py on the cluster exits 0: no item or story is unassigned and no brief names an unknown id.
- render-cluster.py on the cluster leaves the rendered markdown unchanged after the commit.
- The stories include the token revolver asking for its next account.

**Files:**
- modify: docs/design/secrets/checklist.json
- modify: docs/design/secrets/CHECKLIST.md
- modify: docs/design/secrets/stories.json
- modify: docs/design/secrets/USER-STORIES.md

**Checklist:**
- C2 — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a SECRETS-002 requirement.
- C3 — The rendered markdown of this cluster matches its JSON.

**Stories:**
- S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 met: check-coverage.py exits 0. It reports 'Coverage clean: all items covered, briefs consistent': 14 items and 11 stories, with C4 to C14 and S2 to S11 each named by at least one SECRETS-002 requirement and no unknown ids. Row 2 met: render-cluster.py exits 0, and a second run leaves the rendered markdown byte-identical (same checksum before and after), so the markdown matches the JSON. Row 3 met: S6, of the Token revolver persona, reads 'when a session prints its usage-limit words, I want to ask the broker for my next account instead of walking my own list', and R3 carries it.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/secrets/checklist.json` — Adds the section 'The broker's implementation' with items C4 to C14: one per part, plus C13 (open points stay open) and C14 (no engine dependency).
  - modified: `docs/design/secrets/CHECKLIST.md` — Re-rendered from checklist.json.
  - modified: `docs/design/secrets/stories.json` — Adds personas Person, AI Agent, Token revolver, Engine, Operator and a Reviewer who reads the audit, with stories S2 to S11.
  - modified: `docs/design/secrets/USER-STORIES.md` — Re-rendered from stories.json.
- Checklist delivery:
  - [x] C2 — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a SECRETS-002 requirement. — C4 to C14 and S2 to S11 are recorded and each is covered by a SECRETS-002 requirement; coverage exits 0.
  - [x] C3 — The rendered markdown of this cluster matches its JSON. — Re-rendering changes nothing.
- Story delivery:
  - [x] S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time. — The rendered SECRETS-002.md shows every requirement next to the item and story ids it covers.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] check-coverage.py on the cluster exits 0: no item or story is unassigned and no brief names an unknown id. — check-coverage.py with the ledger's DS2_METHOD reports 14 checklist items, 11 user stories, 2 briefs and 'Coverage clean', and exits 0. C4 to C14 and S2 to S11 are each named by a SECRETS-002 requirement.
  - [x] render-cluster.py on the cluster leaves the rendered markdown unchanged after the commit. — render-cluster.py exits 0. The md5 of every .md file was identical before and after a second run, both before and after the harden edits. git status showed no further change.
  - [x] The stories include the token revolver asking for its next account. — stories.json has the persona 'Token revolver' with S6: 'I want to ask the broker for my next account instead of walking my own list'. SECRETS-002 R3 names S6.
- Checklist verified: C2, C3
- Stories verified: S1

## Boundaries

- Change no code: only paths under docs/design/secrets/ are created or modified.
- A credential, token or key never passes through an agent: no credential value is written, read or quoted.
- The statement is the authority: nothing it leaves open is decided here.
- Manifold is never a structural part: the engine that starts or ends a seat is whichever one runs the agent.
- No row of SECRETS-002 is dispatched until Waffles has reviewed it.

## Verification

- From docs/: python3 $DS2_METHOD/scripts/validate.py design/secrets exits 0.
- From docs/: python3 $DS2_METHOD/scripts/check-coverage.py design/secrets exits 0.
- From docs/: python3 $DS2_METHOD/scripts/render-cluster.py design/secrets exits 0 and git status shows no further change.


---
type: brief
id: SECRETS-002
cluster: secrets
title: Build the secrets broker: handles, the proxy, rotation, sealed records, revocation and leases
---

# SECRETS-002: Build the secrets broker: handles, the proxy, rotation, sealed records, revocation and leases

> **Cluster:** secrets
> **Blocked by:** Waffles' review of each row before it is dispatched (CN2), The live permission decision, SpiceDB beside the door, which the statement records as not started (docs/design/identity/STATEMENT-2026-09-22.md:127) and places in step 2 of the road (docs/design/identity/STATEMENT-2026-09-22.md:143); every proxy check asks it (docs/design/identity/STATEMENT-2026-09-22.md:17), A root naming the door repository, which is not set for this round; every door-owned file moves into files when it is, The delegation schema, which the statement leaves unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21); the handle (R1) and the lease's time window (R9) wait on it, The lys-core release that freezes lys/delegation/v1, which waits until the fold that enforces its ordering rule exists (CLAUDE.md:23); until then no delegation is signed outside tests (crates/lys-core/src/delegation/mod.rs:237-242), Standalone broker-host ownership and complete per-row implementation/test file walls: the prior draft maps the door to Cambium, while ADR-004 and IDENTITY-001 rows 04/05 require standalone identity operation. This discrepancy must be resolved by a reviewed ownership decision, not by making Cambium a mandatory server. No empty-wall row is ready for dispatch., Review of the proposed R7 freshness/refusal mechanism and the pending lifecycle-state policy in CONFORMANCE rows 3.2 and 3.4; accepted suspension/reinstatement behaviour in row 3.3 remains binding.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> **Checklist:**
> - C4 — A seat's call with its handle goes through the door's proxy, which checks SpiceDB, swaps the handle for the real credential, forwards the call and writes one audit line naming the seat, the handle, the real account and the time; the credential never leaves the server.
> - C5 — The store keeps a set of real accounts behind one handle, the proxy takes the next in turn and logs which one served, and resting an account is a store change with no file copied to any machine.
> - C6 — The token revolver asks the broker for its next account instead of walking its own list.
> - C7 — The proxy refreshes an OAuth access token itself from the refresh token in the store, and the seat never sees the refresh token.
> - C8 — The door puts the seat's own login into its environment at spawn, read from the store, rotating at spawn, and records which token went to which seat.
> - C9 — Sealed records are kept encrypted in the store, tagged in SpiceDB with which identities may read them, read by name with a relation check and one audit line, and a key is used through the proxy, never read.
> - C10 — Each of the three revocation cases has its own behaviour: a dropped handle refuses every new call, the login token's seat is ended by its engine, and read sealed knowledge is disclosed and recorded, never claimed back.
> - C11 — The proxy has an explicit cancellation rule for calls already admitted when their handle is dropped, written down before it is built.
> - C12 — Everything handed out is a lease counted by uses, a time window and a spend cap: the window in the signed delegation, uses and spend counted by the door, a one-use grant never spent twice.
> - C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
> - C14 — No broker row depends on any one engine: an engine without the broker reads its own pool file as it does today.
> - C15 — Real secret ownership or a valid human-rooted delegation permitting re-lending establishes affirmative may-lend; mere use and display labels do not. Applicable recipient policy is checked separately, including people-only refusal.
> - C16 — Personal, team and organisation secret boundaries are enforced in listing, metadata, read, use and lending; knowing another identity's record ID grants nothing.
> - C17 — Every derived handle stays inside its live ancestry, including shared use/spend budgets and expiry; revoking its source does not revoke an independently authorised sibling.
> - C18 — Issuance, account selection, retry and revocation preserve exact provenance and current authority; the screen distinguishes local refusal from unconfirmed provider action.
> **Stories:**
> - S2 (Person, Grants an agent provisioned under them access to an account) — As a person granting an agent access, I want to give it a handle under my own grant, limited by uses, time and spend, so that it can use the account without ever holding the credential and the grant traces back to me.
> - S3 (Person, Grants an agent provisioned under them access to an account) — As a person, I want to drop an agent's handle and have every new call on it refused at once, so that taking access back does not wait on rotating the real key.
> - S4 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to make my call with my handle and have the proxy swap in the credential, refreshing an expired OAuth token itself, so that I can do my work without ever seeing a credential.
> - S5 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to ask for one of my sealed memories by name and get back only the piece I am permitted, so that a secret never sits in plain text in my memory files.
> - S6 (Token revolver, The worker that runs Claude sessions and builders and turns to the next account on usage-limit words) — As the token revolver, when a session prints its usage-limit words, I want to ask the broker for my next account instead of walking my own list, so that the spread across accounts is the broker's and no pool file is copied to my machine.
> - S7 (Engine, Whichever runtime starts and ends a seat) — As the engine starting a seat, I want the door to give me the seat's own login from the store at spawn and record which token went to which seat, so that I read no pool file when the broker is there and still work from my own when it is not.
> - S8 (Operator, Keeps the accounts and revokes access) — As an operator, I want to rest an account by one change in the store, so that no worker gives it another call and no file is copied to any machine.
> - S9 (Operator, Keeps the accounts and revokes access) — As an operator revoking access, I want each case to say what it reaches: a handle refuses new calls, a call already admitted follows the stated cancellation rule, a login token's seat is ended by its engine, and read knowledge stays read, so that I am never told revocation took back what it cannot.
> - S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.
> - S11 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want each call that was in flight when its handle was dropped to show the outcome the cancellation rule gave it, so that no call's end is unaccounted for.
> - S12 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person allowed to use an account, I want the screen and server to distinguish that from permission to lend it, so I cannot accidentally grant my agent authority I do not hold.
> - S13 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As Dana, I want my private secrets and their metadata isolated from Tom and his agents unless I grant access, so knowing an identifier or signing in to the same installation does not disclose them.
> - S14 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person reviewing a grant or revocation, I want its source, selected service account and confirmed or unconfirmed outcome shown, so I know exactly what authority changed.

## Purpose

Agents get a short-lived, limited handle to a credential, never the credential itself (ADR-001), and the token revolver is the first user (ADR-002). This brief is the temporary key model of the statement, one requirement per part, so each row can be reviewed by Waffles and built on its own; it is step 3 of the road: 'Store, handle, proxy, rotation under 1 handle' (docs/design/identity/STATEMENT-2026-09-22.md:144). Amendment of 23 September 2026: the You and Secrets screens must distinguish permission to use an account from affirmative permission to lend it, with the responsible person and the source grant visible. This amendment is awaiting Waffles review; the implementation is not dispatched or claimed complete.

## Task

Build the broker the statement's 'Secrets: the temporary key model' describes, in Rust, inside the door (docs/design/identity/STATEMENT-2026-09-22.md:23-57), applying the lys formats 'Where it lives: lys' names (docs/design/identity/STATEMENT-2026-09-22.md:82). The store and the proxy are the door's; the handle's and the sealed record's formats are lys's. In: the handle and the proxy swap with its audit line (R1); rotation under one handle (R2); the token revolver as the first consumer (R3); OAuth refresh at the proxy (R4); the seat's own login at spawn (R5); sealed knowledge tagged in SpiceDB (R6); the three revocation cases (R7); the cancellation rule for calls in flight (R8); leases counted by uses, time window and spend (R9). Out: OpenBao or any external secrets engine; the proxy-handle login path; the delegation schema; anything the statement leaves open, each recorded open in the row it touches. Door-owned files are named in each spec with owner and citation and are not in files until a root naming the door repository is set. Every path is relative to its own repository's root. Conformance amendment source: docs/design/identity/CONFORMANCE.md at commit 1353c22, especially rows 3.3 and 7.1–7.8. Trace each executable acceptance identifier below to that committed behaviour map. Tom's 23 September 2026, 19:04:22 Melbourne request is context, not the checkable source. Existing SECRETS-001 execution records describe the earlier authoring task and are not evidence for this amendment. The broker-host ownership must be settled against the standalone identity product before the proposed Cambium paths below become executable walls.

## Requirements

### R1: Issue a handle and swap it at the proxy, with one audit line per use

WHEN a seat makes an outbound call carrying its handle, THE SYSTEM SHALL have the door's proxy check SpiceDB, swap the handle for the real credential, forward the call, and write one audit line naming which seat, which handle, which real account and when (docs/design/identity/STATEMENT-2026-09-22.md:27). A handle is a short-lived value bound to the seat's identity; the real credential sits in the door's encrypted store and never leaves the server (docs/design/identity/STATEMENT-2026-09-22.md:27). A handle is issued only under a grant that traces to a person (ADR-003; docs/design/identity/STATEMENT-2026-09-22.md:21). THE SYSTEM SHALL NOT return, log, or put into any error or Debug output a byte of the real credential, and SHALL NOT forward a call whose handle is unknown, dropped, bound to another identity, or whose SpiceDB check is not a permit.

Owners. The store and the proxy are the door's (docs/design/identity/STATEMENT-2026-09-22.md:25, 'We build this ourselves, in Rust, inside the door'; docs/design/identity/STATEMENT-2026-09-22.md:27). The handle's format is lys: 'Credential handover and sealed knowledge are lys sealed envelopes and the delegation format (lys/delegation/v1, a seat as a typed subject). The secrets broker's handle and the sealed record are these, applied' (docs/design/identity/STATEMENT-2026-09-22.md:82). The audit line is a lys log entry, 'not a row in a database' (docs/design/identity/STATEMENT-2026-09-22.md:80), appended through lys-log-store's Log (crates/lys-log-store/src/lib.rs:1-7).

Lys-owned, in files: crates/lys-core/src/delegation/mod.rs (lys; docs/design/identity/STATEMENT-2026-09-22.md:82; the module is the lys/delegation/v1 artifact, crates/lys-core/src/delegation/mod.rs:1-3), whose docs record the handle as an application of the format; docs/design/WIRE-FORMATS.md (lys; its section 1 is the register of frozen contracts, docs/design/WIRE-FORMATS.md:11-21), which records the handle's format before any handle is signed.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the encrypted store of real credentials (docs/design/identity/STATEMENT-2026-09-22.md:27); crates/cambium-door/src/http/secrets_handle.rs, issuing and resolving handles bound to an identity (docs/design/identity/STATEMENT-2026-09-22.md:27); crates/cambium-door/src/http/secrets_proxy.rs, the check, swap, forward and audit line (docs/design/identity/STATEMENT-2026-09-22.md:27). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: (a) the delegation schema, which the statement leaves unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003). (b) Which lys/delegation/v1 pair a handle would use: the only pair v1 defines for a seat is seat with speaks-for, and that role 'has defined semantics, no implementation and no consumer ... Do not invent a consumer for it' (crates/lys-core/src/delegation/artifact.rs:242-254), so whether the handle is that consumer or a new version alongside is Tom's to settle. (c) Delegation is behind unstable-anchor and 'No delegation may be signed outside tests until it is' ratified (crates/lys-core/src/delegation/mod.rs:237-242), and the lys-core release that freezes it waits for the fold (CLAUDE.md:23). (d) Whose key signs a proxy audit line: the statement says each proxy call is 'signed with the agent's key' (docs/design/identity/STATEMENT-2026-09-22.md:80), while the door holds only the seat's public key and never persists the private half (crates/cambium-door/src/http/agent_seat.rs:13-15). (e) The leaf schema of an audit line, which the statement leaves open as 'the event schema shared with the audit lines' (docs/design/identity/STATEMENT-2026-09-22.md:187).

Conformance amendment (ADR-003; STATEMENT-2026-09-22.md, Everything is pegged to a human authority; docs/design/identity/CONFORMANCE.md at commit 1353c22 row 7.3): WHEN a person or agent requests a derived handle, THE SYSTEM SHALL independently authorise both exercise and delegation against the current source grant. Permission to use a secret, possession of a handle, a display-only owner label or the absence of a prohibition SHALL NOT establish permission to lend. A person verified by the server as the actual owner has the affirmative may-lend route stated in docs/design/identity/CONFORMANCE.md at commit 1353c22, row 7.3; no separate borrowed pass-on grant is required for that owner route. A non-owner requires a valid human-rooted delegation explicitly permitting re-lending. Both routes enforce the requested recipient policy and bounds, identify their responsible person, and retain the actual ownership or source-grant evidence in the decision. A people-only account SHALL refuse every agent recipient, including the owner's own agent. The same check SHALL apply to direct API and agent-tool calls as to the screen; hiding a button is not enforcement. The refusal SHALL identify the blocking grant or policy without exposing another person's secret metadata or value. Stable operation identity SHALL survive a lost acknowledgement so retry cannot mint a second handle. The public record SHALL show holder, responsible person, source grant, permitted operations/resource, pass-on rights, expiry and current state; it SHALL contain no credential material.

**Acceptance:**
- A test sends one call through the proxy with a live handle to an upstream test double: the double receives exactly 1 request carrying the stored credential, and the seat-visible request and response contain no byte of that credential.
- A test sends one call each with an unknown handle, a dropped handle, and a handle bound to another seat's identity: each is refused, and the upstream test double's request count stays 0 across all 3.
- A test in which SpiceDB answers anything other than a permit for the call's relation: the call is refused and the upstream test double's request count is 0.
- A test forwards 3 calls: exactly 3 audit lines are appended to the lys log, each naming the seat, the handle, the real account and the time, and the log's size grows by exactly 3.
- A redaction test formats the store's credential type, the proxy's error type and the audit line with Debug and Display: none of the outputs contains a byte of the credential.
- A test asks the door to issue a handle for an agent whose grant does not trace to a person: no handle is issued.
- docs/design/WIRE-FORMATS.md and crates/lys-core/src/delegation/mod.rs name the handle's format before any handle is signed outside tests, and the points recorded open above are still recorded open, not decided, when the row is reviewed.
- SEC_USE_NOT_LEND: permit Tom to use Dana's finance-readonly account but give Tom no delegation right; Tom's proxied use succeeds, and both browser issuance and a direct API request for Tom's own agent are refused. Exactly zero derived handles, success-audit events or upstream calls result from the refused issuances.
- SEC_AFFIRMATIVE_LEND: verify both routes from CONFORMANCE row 7.3: the actual secret owner can issue a bounded handle without a separate borrowed pass-on grant; a non-owner can issue only with a valid human-rooted delegation permitting re-lending. Remove that delegation while retaining use-only permission and refuse the non-owner. A display-only owner label or ownership of a different secret cannot substitute for the real ownership record. Record which route authorised each success.
- SEC_PEOPLE_ONLY: Dana is the server-verified owner of the accounts-team credential, so ownership establishes her may-lend authority. An explicit applicable recipient policy permits people only. Issuance to her agent is refused by that recipient policy, not by denying Dana's ownership route. With an agent-permitting policy the owner route succeeds. Direct tool/API requests enforce the same decision.
- SEC_ISSUE_RETRY: lose the response after a committed issuance, repeat the exact operation ID and payload, then change the payload under that ID. The repeat returns the original handle identity with one logical issuance event; the changed payload is refused by name.
- SEC_AUTHORITY_TRACE: the visible handle record and audit carry the same holder, responsible person and source-grant IDs returned by the issuer. Altering a supplied parent, recipient kind, action set or resource cannot increase effective authority. Count the refused cases explicitly.

**Files:**
- modify: crates/lys-core/src/delegation/mod.rs
- modify: docs/design/WIRE-FORMATS.md

**Checklist:**
- C4 — A seat's call with its handle goes through the door's proxy, which checks SpiceDB, swaps the handle for the real credential, forwards the call and writes one audit line naming the seat, the handle, the real account and the time; the credential never leaves the server.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C15 — Real secret ownership or a valid human-rooted delegation permitting re-lending establishes affirmative may-lend; mere use and display labels do not. Applicable recipient policy is checked separately, including people-only refusal.
- C18 — Issuance, account selection, retry and revocation preserve exact provenance and current authority; the screen distinguishes local refusal from unconfirmed provider action.

**Stories:**
- S2 (Person, Grants an agent provisioned under them access to an account) — As a person granting an agent access, I want to give it a handle under my own grant, limited by uses, time and spend, so that it can use the account without ever holding the credential and the grant traces back to me.
- S4 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to make my call with my handle and have the proxy swap in the credential, refreshing an expired OAuth token itself, so that I can do my work without ever seeing a credential.
- S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.
- S12 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person allowed to use an account, I want the screen and server to distinguish that from permission to lend it, so I cannot accidentally grant my agent authority I do not hold.
- S14 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person reviewing a grant or revocation, I want its source, selected service account and confirmed or unconfirmed outcome shown, so I know exactly what authority changed.

### R2: Rotate across a set of real accounts under one handle

WHEN a call arrives on a handle whose store entry keeps a set of real accounts, THE SYSTEM SHALL have the proxy take the next one in turn for that call and log which one served it (docs/design/identity/STATEMENT-2026-09-22.md:33). The handle is the stable name (docs/design/identity/STATEMENT-2026-09-22.md:33). Resting an account SHALL be a change in the store with no file copied to any machine, and this SHALL replace the account pool file (docs/design/identity/STATEMENT-2026-09-22.md:33; ADR-002). THE SYSTEM SHALL NOT give a rested account a call, and SHALL NOT trust the spread across accounts to each worker: it is enforced at the proxy (docs/design/identity/STATEMENT-2026-09-22.md:33).

Owner: the door, which keeps the store and the proxy (docs/design/identity/STATEMENT-2026-09-22.md:25-27, docs/design/identity/STATEMENT-2026-09-22.md:33). No lys-owned file changes for rotation; the account that served each call is a field of the R1 audit line.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the set of real accounts behind a handle and the rested mark (docs/design/identity/STATEMENT-2026-09-22.md:33); crates/cambium-door/src/http/secrets_rotation.rs, taking the next account in turn (docs/design/identity/STATEMENT-2026-09-22.md:33). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

Retiring the pool file waits until its consumers have migrated (docs/design/identity/STATEMENT-2026-09-22.md:167), and an engine without the broker reads its own pool file as it does today (docs/design/identity/STATEMENT-2026-09-22.md:62; ADR-004).

Every account selected by rotation SHALL be eligible under the handle's same resource/action and owner boundaries. Rotation SHALL NOT turn a read-only handle into a write capability or select an account belonging to another person merely because it is in an available pool.

**Acceptance:**
- A test with 3 accounts behind one handle sends 6 calls: each account serves exactly 2, in turn, and each of the 6 audit lines names the account that served it.
- A test rests 1 of the 3 accounts in the store and sends 4 calls: the rested account serves 0 of them and the other 2 serve 2 each.
- Resting an account is a single store change: the test performs no file write outside the store and touches no pool file.
- A test with every account behind a handle rested sends 1 call: it is refused and the upstream test double's request count is 0.
- SEC_ROTATION_SCOPE: place one eligible account and one account outside the source grant in a test pool. Calls under that handle use only the eligible account; after it is rested the next call refuses instead of selecting the out-of-scope account. The audit identifies the selected account without disclosing its credential.

**Checklist:**
- C5 — The store keeps a set of real accounts behind one handle, the proxy takes the next in turn and logs which one served, and resting an account is a store change with no file copied to any machine.
- C17 — Every derived handle stays inside its live ancestry, including shared use/spend budgets and expiry; revoking its source does not revoke an independently authorised sibling.

**Stories:**
- S8 (Operator, Keeps the accounts and revokes access) — As an operator, I want to rest an account by one change in the store, so that no worker gives it another call and no file is copied to any machine.
- S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.
- S12 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person allowed to use an account, I want the screen and server to distinguish that from permission to lend it, so I cannot accidentally grant my agent authority I do not hold.

### R3: Serve the token revolver its next account from the broker

WHEN a worker that runs Claude sessions or builders sees a session print its usage-limit words, THE SYSTEM SHALL let it ask the broker for its next account instead of walking its own ordered list (docs/design/identity/STATEMENT-2026-09-22.md:35; ADR-002). The revolver is the first consumer of the handle (docs/design/identity/STATEMENT-2026-09-22.md:35). The broker's answer SHALL come from the same rotation as R2, and SHALL be attributed to the asking seat. THE SYSTEM SHALL NOT make any engine depend on the broker: an engine without it reads its own pool file as it does today (docs/design/identity/STATEMENT-2026-09-22.md:62; ADR-004), and manifold is never a structural part (docs/design/identity/STATEMENT-2026-09-22.md:61).

Owners. The broker's side is the door's (docs/design/identity/STATEMENT-2026-09-22.md:25-27). The worker's side is the engine's: the revolver read for this brief is manifold's (a launcher arms it, crates/manifold-node/src/seat/launcher.rs:31-40 in the manifold repository read at 3df5ac5f64, and it turns through the operator's pool <data-dir>/supervisor/accounts.json, docs/seat-document.md:569 in that repository), but the statement names no engine as the consumer and the design inventory names no engine repository.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-door/src/http/secrets_next_account.rs, the call a worker makes for its next account (docs/design/identity/STATEMENT-2026-09-22.md:35). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: the owner of the worker-side file. The sources do not settle which engine repository takes the first change, and no engine file is named until they do. Also open with R5: whether the answer is the account's login token at spawn (R5) or a handle through the proxy, which stays unproved (docs/design/identity/STATEMENT-2026-09-22.md:45).

**Acceptance:**
- A test asks for the next account 3 times for one seat against a handle with 2 accounts: the answers alternate between the 2 accounts, and each ask appends exactly 1 audit line naming the seat.
- A test asks for the next account when every account behind the handle is rested: the ask is refused and names no account.
- The door's tests for this call start no engine process and depend on no engine crate: the call is exercised with a test client alone.
- The worker-side owner is recorded open in this requirement when the row is reviewed, and no engine file is named.

**Checklist:**
- C6 — The token revolver asks the broker for its next account instead of walking its own list.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C14 — No broker row depends on any one engine: an engine without the broker reads its own pool file as it does today.

**Stories:**
- S6 (Token revolver, The worker that runs Claude sessions and builders and turns to the next account on usage-limit words) — As the token revolver, when a session prints its usage-limit words, I want to ask the broker for my next account instead of walking my own list, so that the spread across accounts is the broker's and no pool file is copied to my machine.

### R4: Refresh OAuth at the proxy; the seat never sees the refresh token

WHEN a call arrives on a handle whose credential is an OAuth grant, THE SYSTEM SHALL have the proxy swap the handle for a live access token, and refresh it itself when it expires (docs/design/identity/STATEMENT-2026-09-22.md:39). The refresh token sits in the store (docs/design/identity/STATEMENT-2026-09-22.md:39). THE SYSTEM SHALL NOT let the seat see the refresh token or the access token (docs/design/identity/STATEMENT-2026-09-22.md:39). Revoking SHALL drop the handle and MAY also revoke the grant upstream (docs/design/identity/STATEMENT-2026-09-22.md:39).

Owner: the door (docs/design/identity/STATEMENT-2026-09-22.md:25-27, docs/design/identity/STATEMENT-2026-09-22.md:39). No lys-owned file changes for refresh.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, holding the refresh token (docs/design/identity/STATEMENT-2026-09-22.md:39); crates/cambium-door/src/http/secrets_oauth.rs, the refresh at the proxy (docs/design/identity/STATEMENT-2026-09-22.md:39). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

A provider sign-in identity and an OAuth service-access grant SHALL remain distinct records. The account selected during service consent, its provider subject, client registration and consented scopes SHALL be recorded as that service grant's provenance; a sign-in email or the currently displayed person SHALL NOT substitute for the selected provider subject. Lending service access still requires R1's affirmative delegation decision.

**Acceptance:**
- A test with an expired access token sends 1 call: the proxy makes exactly 1 refresh request to the provider test double, forwards the call with the new access token, and the seat-visible response contains neither the refresh token nor either access token.
- A test with a live access token sends 2 calls: the provider test double's refresh request count is 0.
- A test drops an OAuth handle: the next call is refused, and when upstream revocation is asked for, the provider test double receives exactly 1 revocation request.
- A redaction test formats the store's OAuth grant type with Debug: the output contains neither token.
- SEC_OAUTH_ACCOUNT: use separate sign-in and service-consent provider fixtures, select a different account during service consent, and refresh after reopen. The recorded service grant and upstream call use the selected service account and original client; neither a matching email nor sign-in credentials are used to rebind or refresh it. A new client requires a named reconnect rather than an assumed refresh.

**Checklist:**
- C7 — The proxy refreshes an OAuth access token itself from the refresh token in the store, and the seat never sees the refresh token.
- C18 — Issuance, account selection, retry and revocation preserve exact provenance and current authority; the screen distinguishes local refusal from unconfirmed provider action.

**Stories:**
- S4 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to make my call with my handle and have the proxy swap in the credential, refreshing an expired OAuth token itself, so that I can do my work without ever seeing a credential.
- S14 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person reviewing a grant or revocation, I want its source, selected service account and confirmed or unconfirmed outcome shown, so I know exactly what authority changed.

### R5: Put the seat's own login into its environment at spawn

WHEN the engine that runs a seat starts it, THE SYSTEM SHALL have the door put the long-lived OAuth token Claude Code generates into the seat's environment, read from the store (docs/design/identity/STATEMENT-2026-09-22.md:43). This is the one place the credential reaches the process (docs/design/identity/STATEMENT-2026-09-22.md:43, docs/design/identity/STATEMENT-2026-09-22.md:167). It rotates at spawn, not per call (docs/design/identity/STATEMENT-2026-09-22.md:43). THE SYSTEM SHALL record which token went to which seat (docs/design/identity/STATEMENT-2026-09-22.md:56). THE SYSTEM SHALL NOT name any one engine: the engine that starts or ends a seat is whichever one runs the agent (docs/design/identity/STATEMENT-2026-09-22.md:61; ADR-004), and an engine without the broker reads its own pool file as it does today (docs/design/identity/STATEMENT-2026-09-22.md:62).

Owner: the door, which holds the store (docs/design/identity/STATEMENT-2026-09-22.md:43). No lys-owned file changes for the login at spawn.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the login tokens and the record of which went to which seat (docs/design/identity/STATEMENT-2026-09-22.md:43, docs/design/identity/STATEMENT-2026-09-22.md:56); crates/cambium-door/src/http/secrets_spawn_login.rs, the answer an engine receives at spawn (docs/design/identity/STATEMENT-2026-09-22.md:43). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: (a) the proxy-handle login path, where the seat holds a handle in the token variable and its base URL points at the proxy; it 'is not tested with a subscription login and nothing depends on it' (docs/design/identity/STATEMENT-2026-09-22.md:45), and this row builds nothing that depends on it. (b) Whether revoking the login token at the provider makes the seat's next call fail; it 'is proved with the provider before it is promised' (docs/design/identity/STATEMENT-2026-09-22.md:56), and this row promises nothing about it.

**Acceptance:**
- A test spawns 2 seats against a login set of 2 accounts: each spawn answer carries the login for exactly 1 account, the 2 answers differ, and the store's record names which account's token went to which seat.
- A test sends 3 calls from one spawned seat: the seat's login does not change between them (rotation is at spawn only).
- The spawn call is exercised by a test client with no engine crate in its dependencies.
- The proxy-handle login path and the provider-revocation question are recorded open in this requirement when the row is reviewed, and no acceptance criterion of this brief depends on either.

**Checklist:**
- C8 — The door puts the seat's own login into its environment at spawn, read from the store, rotating at spawn, and records which token went to which seat.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C14 — No broker row depends on any one engine: an engine without the broker reads its own pool file as it does today.

**Stories:**
- S7 (Engine, Whichever runtime starts and ends a seat) — As the engine starting a seat, I want the door to give me the seat's own login from the store at spawn and record which token went to which seat, so that I read no pool file when the broker is there and still work from my own when it is not.

### R6: Keep sealed knowledge in the store, tagged in SpiceDB

THE SYSTEM SHALL keep keys that cannot be rotated, and memories an identity wants kept secret, as sealed records in the same store, encrypted, each tagged in SpiceDB with which identities may read it (docs/design/identity/STATEMENT-2026-09-22.md:49). WHEN a seat asks for a sealed record by name, THE SYSTEM SHALL check the relation, return the text, and write one audit line (docs/design/identity/STATEMENT-2026-09-22.md:49). Each identity's sealed records are its own (docs/design/identity/STATEMENT-2026-09-22.md:49). THE SYSTEM SHALL NOT let a sealed record sit in plain text in a memory file (docs/design/identity/STATEMENT-2026-09-22.md:49), and SHALL NOT return a key by reading it: a key is used through the proxy even when it cannot rotate; only memories are read, in the smallest piece asked for (docs/design/identity/STATEMENT-2026-09-22.md:57; ADR-001).

Owners. The store, the SpiceDB tag and the read are the door's (docs/design/identity/STATEMENT-2026-09-22.md:49). The sealed record's format is lys sealed envelopes, applied (docs/design/identity/STATEMENT-2026-09-22.md:82).

Lys-owned, in files: crates/lys-core/src/seal/mod.rs (lys; docs/design/identity/STATEMENT-2026-09-22.md:82; the module is the sealed envelope, crates/lys-core/src/seal/mod.rs:1-10), whose docs record the sealed record as an application of it; docs/design/WIRE-FORMATS.md (lys; the lys/sealed-envelope/v1 row, docs/design/WIRE-FORMATS.md:18), which records the application.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the sealed records (docs/design/identity/STATEMENT-2026-09-22.md:49); crates/cambium-door/src/http/secrets_sealed.rs, the read by name with its relation check and audit line (docs/design/identity/STATEMENT-2026-09-22.md:49). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: lys/sealed-envelope/v1 seals with an empty AAD (docs/design/WIRE-FORMATS.md:18; crates/lys-core/src/seal/sealed_envelope.rs:180-183), so an envelope carries nothing that binds it to its name or its owning identity. Whether 'each identity's sealed records are its own' is held by a construction alongside v1 or by the door is not settled by the statement; any construction is a new version alongside, never a change to the shipped one, and takes an adversarial review (CLAUDE.md, coding standards).

Personal, team and organisation scopes SHALL be enforced at list, metadata, read, use and lending seams. A personal record belongs to its recorded person; another person's sign-in or agent ownership creates no access to it. Knowing a record ID or path SHALL NOT bypass its policy. Public denials SHALL avoid leaking the existence, title or owner of a record the caller cannot discover.

**Acceptance:**
- A test in which SpiceDB permits the asking identity reads one memory record by name: the text is returned and exactly 1 audit line is appended naming the seat, the record and the time.
- A test in which SpiceDB does not permit the asking identity: the read is refused, no text is returned, and the record is not decrypted.
- A test asks to read a record marked as a key: the read is refused, and the key is usable only through the proxy of R1.
- A test moves one identity's sealed record to another identity's name in the store and reads it as the second identity: the read returns no text.
- A test searches every memory file the door writes for a record's plaintext after sealing it: 0 matches.
- SEC_PRIVATE_SCOPE: create distinct personal memories and credentials for Tom and Dana. For each person, enumerate, read by known ID and request a derived handle as the other person and as that person's agent. Every unauthorised leg returns no protected metadata/plaintext, performs no decryption or credential forwarding and creates no handle; same-owner authorised controls prove each route actually ran.

**Files:**
- modify: crates/lys-core/src/seal/mod.rs
- modify: docs/design/WIRE-FORMATS.md

**Checklist:**
- C9 — Sealed records are kept encrypted in the store, tagged in SpiceDB with which identities may read them, read by name with a relation check and one audit line, and a key is used through the proxy, never read.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C16 — Personal, team and organisation secret boundaries are enforced in listing, metadata, read, use and lending; knowing another identity's record ID grants nothing.

**Stories:**
- S5 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to ask for one of my sealed memories by name and get back only the piece I am permitted, so that a secret never sits in plain text in my memory files.
- S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.
- S13 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As Dana, I want my private secrets and their metadata isolated from Tom and his agents unless I grant access, so knowing an identifier or signing in to the same installation does not disclose them.

### R7: Revoke in the three cases the statement names

THE SYSTEM SHALL revoke in the 3 cases the statement splits (docs/design/identity/STATEMENT-2026-09-22.md:53-57), each with its own story. (1) A handle: WHEN a handle is dropped, THE SYSTEM SHALL refuse every new call on it, because the process never had the credential (docs/design/identity/STATEMENT-2026-09-22.md:55, docs/design/identity/STATEMENT-2026-09-22.md:29); a call already admitted follows the cancellation rule of R8. (2) The login token: it is in the process; THE SYSTEM SHALL record which token went to which seat, and the engine that runs the seat ends it on its own (docs/design/identity/STATEMENT-2026-09-22.md:56; docs/design/identity/STATEMENT-2026-09-22.md:61). (3) Sealed knowledge: once read it is in the process's context; permission controls the disclosure and the audit line records it, and neither takes back what was read (docs/design/identity/STATEMENT-2026-09-22.md:57). THE SYSTEM SHALL NOT describe revocation of a login token or of read knowledge as taking anything back. The real key is rotated upstream only when it is suspected leaked (docs/design/identity/STATEMENT-2026-09-22.md:29).

Owners: the door for dropping handles and refusing calls (docs/design/identity/STATEMENT-2026-09-22.md:27-29); the engine that runs the seat for ending it (docs/design/identity/STATEMENT-2026-09-22.md:56, docs/design/identity/STATEMENT-2026-09-22.md:61). Revocation in lys 'is itself an append with the live set folded from the log (DP26)' (docs/design/identity/STATEMENT-2026-09-22.md:79; docs/design/lys-anchor/DECISIONS.md:400), a fold 'ruled (DP26), not built' (docs/design/identity/STATEMENT-2026-09-22.md:126).

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-door/src/http/secrets_revoke.rs, dropping a handle and answering which seat holds which login token (docs/design/identity/STATEMENT-2026-09-22.md:55-56). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: (a) whether revoking the login token at the provider makes the seat's next call fail, proved with the provider before it is promised (docs/design/identity/STATEMENT-2026-09-22.md:56). (b) The file that carries the lys revocation fold: its owner is lys (docs/design/identity/STATEMENT-2026-09-22.md:126), and the key-history artifact that would carry a fold 'is its own future format' (crates/lys-core/src/delegation/mod.rs:208-210); no file is named until that format is designed.

Withdrawing a source grant SHALL stop fresh issuance and fresh use through every grant and handle derived from that source. A separate valid grant to the same holder or resource SHALL remain independent and SHALL NOT be revoked merely because it shares an identity or account. Proposal under review, not a settled source rule: if the broker cannot establish a permission decision at least as fresh as the revocation, refuse the affected admission by name rather than use a cached permit. The freshness mechanism and whether admission can await a fresh decision must be settled before dispatch. Local refusal of new proxy calls and upstream revocation confirmation SHALL be represented separately; a timeout SHALL remain unconfirmed, never displayed as completed. Unknown mutation outcomes retain their operation IDs for reconciliation.

The general identity-state policy is pending in docs/design/identity/CONFORMANCE.md at commit 1353c22, rows 3.2 and 3.4, and must be reviewed before this row is dispatched. The specific virtual-credential hold on suspension follows accepted row 3.3. Reinstatement SHALL re-evaluate current grants and leases; it SHALL NOT resurrect a separately revoked handle, an expired provisional grant, a retired identity or authority withdrawn by an ancestor. A suspension being lifted is not an issuance or renewal event.

**Acceptance:**
- A test drops a handle and then sends 3 new calls on it: all 3 are refused and the upstream test double's request count stays at its value before the drop.
- A test revokes a seat's login: the door's answer names the seat the token went to, and the door itself ends no process.
- A test revokes a sealed record's relation after one read: the next read is refused, the earlier audit line is still in the log, and the door's answer does not claim the earlier read is undone.
- The provider-revocation question and the fold's file are recorded open in this requirement when the row is reviewed.
- SEC_REVOKE_CHAIN: create a person-to-agent-to-agent chain with explicit pass-on grants and an independent sibling source. Revoke the chain root, then exercise every descendant. All chain-derived calls refuse and forward zero new upstream requests; the independently authorised control still works.
- SEC_REVOKE_FRESHNESS (proposed consistency acceptance; review required before dispatch): hold a permission replica behind the revocation revision. Fresh use must either await a sufficiently fresh decision or refuse by name; it never reaches upstream on the stale permit. Assert the selected behaviour and the exact zero-forward count.
- SEC_REVOKE_STATES: acknowledge local handle revocation while withholding provider acknowledgement. API and screen show local use stopped and upstream unconfirmed; no timer changes it to confirmed. Deliver the exact matching provider outcome and verify the state transitions once without a second revoke operation.
- SEC_REINSTATE_CURRENT: suspend a holder with three handles, independently revoke one and expire another while suspended, then reinstate the holder. Only the third still-authorised handle can resume; the revoked and expired handles stay refused and no new issuance/renewal is recorded.

**Checklist:**
- C10 — Each of the three revocation cases has its own behaviour: a dropped handle refuses every new call, the login token's seat is ended by its engine, and read sealed knowledge is disclosed and recorded, never claimed back.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C17 — Every derived handle stays inside its live ancestry, including shared use/spend budgets and expiry; revoking its source does not revoke an independently authorised sibling.
- C18 — Issuance, account selection, retry and revocation preserve exact provenance and current authority; the screen distinguishes local refusal from unconfirmed provider action.

**Stories:**
- S3 (Person, Grants an agent provisioned under them access to an account) — As a person, I want to drop an agent's handle and have every new call on it refused at once, so that taking access back does not wait on rotating the real key.
- S9 (Operator, Keeps the accounts and revokes access) — As an operator revoking access, I want each case to say what it reaches: a handle refuses new calls, a call already admitted follows the stated cancellation rule, a login token's seat is ended by its engine, and read knowledge stays read, so that I am never told revocation took back what it cannot.
- S14 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person reviewing a grant or revocation, I want its source, selected service account and confirmed or unconfirmed outcome shown, so I know exactly what authority changed.

### R8: State and enforce the cancellation rule for calls already admitted

WHILE a call already admitted by the proxy is in flight, a drop of its handle does not stop it; THE SYSTEM SHALL have the proxy carry an explicit cancellation rule for calls in flight, and that rule is part of the broker's design (docs/design/identity/STATEMENT-2026-09-22.md:55). THE SYSTEM SHALL write the rule down, in the door's broker module docs, before it is implemented, and SHALL give every call in flight at a drop an outcome the audit records. THE SYSTEM SHALL NOT leave the outcome of a call in flight at a drop undefined. What the rule is (let every admitted call finish, cancel at the next boundary, or another shape) is not stated by the statement; it is settled in this row's review (CN2) before any code, not here.

Owner: the door, whose proxy admits calls (docs/design/identity/STATEMENT-2026-09-22.md:27, docs/design/identity/STATEMENT-2026-09-22.md:55). Chippy's release 2 names 'demonstrate revocation and recovery after a crash' (docs/design/identity/STATEMENT-2026-09-22.md:155) and a retry needing 'a defined outcome' (docs/design/identity/STATEMENT-2026-09-22.md:70), which the rule must also answer for a call in flight when the door stops.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-door/src/http/secrets_in_flight.rs, the register of admitted calls and the rule applied to them at a drop (docs/design/identity/STATEMENT-2026-09-22.md:55). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: the rule itself.

**Acceptance:**
- The door's broker module docs state the cancellation rule in words before the row's first code commit.
- A test admits 2 calls against a slow upstream test double, drops the handle while both are in flight, then sends 1 new call: the new call is refused, and each of the 2 in-flight calls ends with the outcome the stated rule names, each with exactly 1 audit line recording that outcome.
- A test stops the door with 1 call in flight and restarts it: the call's outcome is recorded in the audit, and a retry of that call has the outcome the stated rule names.

**Checklist:**
- C11 — The proxy has an explicit cancellation rule for calls already admitted when their handle is dropped, written down before it is built.

**Stories:**
- S9 (Operator, Keeps the accounts and revokes access) — As an operator revoking access, I want each case to say what it reaches: a handle refuses new calls, a call already admitted follows the stated cancellation rule, a login token's seat is ended by its engine, and read knowledge stays read, so that I am never told revocation took back what it cannot.
- S11 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want each call that was in flight when its handle was dropped to show the outcome the cancellation rule gave it, so that no call's end is unaccounted for.

### R9: Count leases by uses, time window and spend

THE SYSTEM SHALL treat everything handed out as a lease: a number of uses, a time window, a spend cap (docs/design/identity/STATEMENT-2026-09-22.md:68). The time window lives in the signed delegation; uses and spend are counted by the door, because a signed object cannot count (docs/design/identity/STATEMENT-2026-09-22.md:68). WHEN two requests arrive at once on a one-use grant, THE SYSTEM SHALL NOT spend it twice, and a retry SHALL have a defined outcome (docs/design/identity/STATEMENT-2026-09-22.md:70). A hard cap SHALL need a reservation before work starts and a settlement after, so two concurrent sessions cannot both spend the same remaining allowance (docs/design/identity/STATEMENT-2026-09-22.md:70); hard spending caps ship only where reservation and enforcement are proved (docs/design/identity/STATEMENT-2026-09-22.md:156). The proxy sees only the spending that passes through it (docs/design/identity/STATEMENT-2026-09-22.md:70), and THE SYSTEM SHALL NOT report a spend total as covering what did not pass through the proxy.

Owners: the door counts uses and spend (docs/design/identity/STATEMENT-2026-09-22.md:68). The time window lives in the signed delegation, whose format is lys (docs/design/identity/STATEMENT-2026-09-22.md:68, docs/design/identity/STATEMENT-2026-09-22.md:82).

Lys-owned, in files: crates/lys-core/src/delegation/mod.rs (lys; docs/design/identity/STATEMENT-2026-09-22.md:82), whose 'Expiry. There is no not_after' paragraph (crates/lys-core/src/delegation/mod.rs:218-221) must be answered for a lease's time window; docs/design/WIRE-FORMATS.md (lys; docs/design/WIRE-FORMATS.md:11-21), which records the delegation that carries it.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the use and spend counters and reservations (docs/design/identity/STATEMENT-2026-09-22.md:68, docs/design/identity/STATEMENT-2026-09-22.md:70); crates/cambium-door/src/http/secrets_lease.rs, the check, reservation and settlement at the proxy (docs/design/identity/STATEMENT-2026-09-22.md:68-70). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: the delegation schema (docs/design/identity/STATEMENT-2026-09-22.md:21), and with it how a time window is carried: lys/delegation/v1 has no expiry by design (crates/lys-core/src/delegation/mod.rs:218-221), so a window in the signed delegation is a new version alongside v1, never a change to it; its shape waits on the schema.

Derived leases SHALL be bounded by every live ancestor's resource, action, recipient-kind, time, use and spend restrictions. Splitting authority across two children SHALL NOT duplicate the parent's remaining use or spend budget. Delegation SHALL NOT renew a provisional grant or move its end date. A role-definition version change is distinct from the holder's grant and SHALL NOT silently extend that grant's lease. Expiry is checked against the named clock at admission, not merely displayed by the browser.

**Acceptance:**
- A test with a lease of 2 uses sends 3 calls: the first 2 are forwarded and the 3rd is refused, and the upstream test double's request count is 2.
- A test sends 2 simultaneous requests on a one-use grant: with both held at the use check until both have arrived, so the race is forced rather than hoped for, exactly 1 is forwarded and the other is refused.
- A test retries a call whose first attempt was forwarded: the retry has the outcome the row's docs state, and the use count moves at most once.
- A test runs 2 concurrent sessions against a hard cap with room for 1: exactly 1 reservation succeeds, and the settled spend never exceeds the cap.
- A test sends a call after its lease's time window has ended: it is refused.
- The door's spend report labels its total as the spending that passed through the proxy.
- The delegation schema and the carriage of the time window are recorded open in this requirement when the row is reviewed.
- SEC_LEASE_ATTENUATION: request a child lease ending after its parent, with broader actions/resource or an unpermitted recipient kind. Each request is refused naming the exceeded boundary; a strictly narrower control succeeds. An expired ancestor refuses even when the child's own displayed end date is later.
- SEC_SHARED_ALLOWANCE: derive two handles under a source with one use remaining; hold two calls at their shared reservation boundary and release them together. Exactly one upstream call occurs and the other refuses. Reopen and retry the accepted operation; the source allowance remains consumed once.
- SEC_PROVISIONAL_EXPIRY: advance a controlled clock to the holder's grant end and try both use and child issuance. Both refuse, including after editing the role definition or moving the holder to another role version; no action renews the expired grant without a separately authorised grant operation.

**Files:**
- modify: crates/lys-core/src/delegation/mod.rs
- modify: docs/design/WIRE-FORMATS.md

**Checklist:**
- C12 — Everything handed out is a lease counted by uses, a time window and a spend cap: the window in the signed delegation, uses and spend counted by the door, a one-use grant never spent twice.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C17 — Every derived handle stays inside its live ancestry, including shared use/spend budgets and expiry; revoking its source does not revoke an independently authorised sibling.

**Stories:**
- S2 (Person, Grants an agent provisioned under them access to an account) — As a person granting an agent access, I want to give it a handle under my own grant, limited by uses, time and spend, so that it can use the account without ever holding the credential and the grant traces back to me.
- S12 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person allowed to use an account, I want the screen and server to distinguish that from permission to lend it, so I cannot accidentally grant my agent authority I do not hold.

## Boundaries

- No credential, token or key value is ever written, read or quoted in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- A credential never passes through an agent: the only credential that reaches a process is the seat's own login at spawn (R5).
- Nothing the statement leaves open is decided in a row: the delegation schema, provider revocation of a login token, the proxy-handle login path, the worker-side owner, the fold's file, the audit line's schema and signer, and the cancellation rule until its review.
- Manifold is never a structural part: the engine that starts or ends a seat is whichever one runs the agent, and no row depends on any one engine.
- Every project works without the others: an engine without the broker reads its own pool file as it does today, and the door without the broker signs people in as it does today.
- No OpenBao and no external secrets engine.
- A shipped wire format is never mutated: lys/sealed-envelope/v1 and lys/delegation/v1 evolve only by a new version alongside, and any cryptographic change takes an adversarial review before it lands.
- No door file is edited in a round without a root naming the door repository.
- No row is dispatched until Waffles has reviewed it.
- A mock-up owner label, an absent deny flag or a role name is never ownership evidence. Server-verified ownership of the secret is an affirmative may-lend route under docs/design/identity/CONFORMANCE.md at commit 1353c22, row 7.3; a non-owner needs a valid human-rooted delegation permitting re-lending. Policy and authority checks are performed at the server for every UI/API/MCP route; the UI renders that answer.

## Verification

- From the lys repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the door repository root: the door's own gates, all clean, once its root is set.
- From docs/ of the lys repository: python3 $DS2_METHOD/scripts/validate.py design/secrets and python3 $DS2_METHOD/scripts/check-coverage.py design/secrets exit 0.
- A search of every file a row touches finds no credential, token or key value.
- Map SEC_USE_NOT_LEND through SEC_PROVISIONAL_EXPIRY to the accepted mock-up conformance IDs before dispatch; the code evidence must include independent negative cases and exact exercised counts. A rendered mock-up is a specification artifact, never proof that broker enforcement exists.


---
type: brief
id: SECRETS-003
cluster: secrets
title: Build the secrets broker in crates/lys-secrets: baseline, contract and counted code rows
---

# SECRETS-003: Build the secrets broker in crates/lys-secrets: baseline, contract and counted code rows

> **Cluster:** secrets
> **Blocked by:** The lead's review of each row before it is dispatched (CN5)., Tom's word on ADR-019, which amends his ADR-001 by moving the broker from the door into lys as crates/lys-secrets; the lead puts the move to him. It is proposed until he gives it. R3 to R8 carry it in their own Blocked by; R1 and R2 do not wait on it., The delegation schema, which stays open (ADR-003); SECRETS-002 criteria that depend on it wait on it.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-019 — The secrets broker lives in lys, as crates/lys-secrets; Cambium consumes it and is never its home (amends ADR-001) — The broker lives in the lys repository as the crate crates/lys-secrets, part of the standalone identity platform, built in Rust. A person installs Lys to hold secrets; Cambium reaches the broker through the door as a consumer and is never its home. Rejected: the broker inside the cambium door that ADR-001 and SECRETS-002 name, which would make Cambium a runtime a person must install to keep a credential.
> - ADR-020 — A lease is a broker record under the audit log, not a signed format — The lease is a broker record under the audit log: it counts uses, time and spend against a grant and is read only by the broker, so nothing a stranger verifies changes. The limit on a lease's time is the window of the lys-identity grant it counts against (DIRECTORY-006 R1): the lease's own not_after never extends past it. Rejected: a signed lease format, a new version beside lys/delegation/v1 with its own adversarial review; and reading the window as one the signed delegation keeps, which v1 cannot carry.
> **Checklist:**
> - C19 — docs/design/secrets/BASELINE.md records, by repository, pinned commit, file and line, the revolver's next-account call site and every consumer of the account pool file.
> - C20 — docs/design/secrets/BASELINE.md records every credential path into a seat today, by file and line, each classed as exactly one of: the login exception, a proxied credential, or neither.
> - C21 — docs/design/secrets/BASELINE.md records, by file and line, what lys/delegation/v1 and lys/sealed-envelope/v1 can and cannot carry for a handle, a lease and a sealed record.
> - C22 — docs/design/secrets/CONTRACT.md states the invariant, the key custody, the handle with how it is stored and how its presenter is authenticated, the lease with its atomic use step and retry outcome, the cancellation rule, the SpiceDB relations, the sealed-record binding and the audit line fields, and is accepted before any code row starts.
> - C23 — docs/design/secrets/reports/SECRETS-003-adversarial-review.md records every attack tried in the six named classes, by a reviewer who did not write the contract and is a different commit author, and the contract clause defeating each.
> - C24 — The store, the redacting type, handles and leases in crates/lys-secrets deliver SECRETS-002 R9 and the issuance criteria of R1, with the store key kept out of the store directory, opening without it refused by name, the store key rotated with every entry readable under the new key and none under the old, swapped ciphertext refused, and every counted leg of their row passing; for SEC_USE_NOT_LEND and SEC_PEOPLE_ONLY they deliver the library-level decision, and the screen and API legs are recorded as not delivered here.
> - C25 — The proxy in crates/lys-secrets delivers SECRETS-002 R1's proxy criteria, R2, R8 and R7's handle case, with every counted leg of its row passing; for SEC_REVOKE_STATES it delivers the library-level decision, and the API and screen legs are recorded as not delivered here.
> - C26 — OAuth refresh at the proxy in crates/lys-secrets delivers SECRETS-002 R4, with every counted leg of its row passing.
> - C27 — The seat's own login at spawn from crates/lys-secrets delivers SECRETS-002 R5 and R7's login-token case, with every counted leg of its row passing.
> - C28 — Sealed records in crates/lys-secrets deliver SECRETS-002 R6 and R7's sealed-knowledge case, with every counted leg of their row passing.
> - C29 — The broker's next-account answer in crates/lys-secrets delivers SECRETS-002 R3's broker side, with every counted leg of its row passing, and names no engine file.
> **Stories:**
> - S15 (Lead, Reviews SECRETS-003's rows before any is dispatched) — As the lead, I want the revolver's call site, the pool file's consumers, every credential path into a seat and what the lys formats can carry recorded from source by file and line, so that the contract rests on what the code does today rather than on memory.
> - S16 (Lead, Reviews SECRETS-003's rows before any is dispatched) — As the lead, I want the broker's contract and an adversarial review by someone other than its author before any code, so that every named attack has a clause defeating it before it can be built in.
> - S17 (Implementer, Is dispatched one SECRETS-003 code row at a time) — As the implementer of a code row, I want the row to name the SECRETS-002 requirements it delivers, its counted legs, its hours and what blocks it, so that I build from one row and nothing is described twice.
> - S18 (Person, Keeps credentials with the standalone identity platform) — As a person keeping credentials, I want the broker to come with Lys alone, so that I never have to install Cambium to keep a credential.

## Purpose

SECRETS-002 states what the secrets broker does, requirement by requirement; this brief is how it is built. It adds what SECRETS-002 lacks: a baseline recorded from source before anything is designed further, a contract with an adversarial review before any code, and code rows in crates/lys-secrets that each name the SECRETS-002 requirements they deliver and carry counted acceptance legs, an estimate in hours, and the directory briefs they wait on. SECRETS-002 is not edited, so what has landed stays stable.

## Task

Dispatch from this brief's rows, one at a time, after the lead's review of each (CN5). R1 and R2 are documents only, come first and wait on nothing outside this brief, so depends_on is empty and no brief-level blocker holds them back. R3 to R8 are code rows, run in the order R3, then R4, then R5 to R8, and each carries its own Blocked by: R1 and R2 accepted, DIRECTORY-002 and DIRECTORY-003 landed, DIRECTORY-006 R1, R2, R3 and R4 landed (never the whole of DIRECTORY-006, whose R6 waits on DIRECTORY-005), and the word on ADR-019, each with a check a stranger can run; R4 also waits on R3, on the review of CONFORMANCE row 3.2 (recorded by ADR-011) and on the review of CONFORMANCE row 3.4 (the emergency stop, recorded by its own decided entry), each with its own check, and R5 to R8 on R4. SECRETS-002's R1 to R9 are delivered through R3 to R8: a builder is dispatched from SECRETS-003's rows, never from SECRETS-002's, and each code row names the SECRETS-002 requirement and acceptance criteria it delivers, with each criterion's id carried in a test name so a search proves the mapping. Every SECRETS-002 requirement is delivered by at least one row: R1 by R3 and R4; R2 by R4; R3 by R8; R4 by R5; R5 by R6; R6 by R7; R7 by R4, R6 and R7; R8 by R4; R9 by R3. Wherever SECRETS-002 says the door, read the broker in crates/lys-secrets. Host: ADR-019, proposed, records that the broker lives in lys as crates/lys-secrets and Cambium consumes it through the door; it amends ADR-001, which placed the broker inside the door and is Tom's decision, so the amendment waits on his word. The code rows' files stand on ADR-019 as proposed; if Tom keeps the broker inside the door, their file lists are re-briefed as an amendment before any row is dispatched. R1 and R2 do not wait on it: the baseline is recorded from source whatever the broker's home, and the contract states the broker's behaviour, naming crates/lys-secrets as the intended home. Lease: ADR-020, decided: a broker record under the audit log, read only by the broker; the other reading, a signed format as a new version beside lys/delegation/v1, is not built. The limit on a lease's time is the lys-identity grant's window (DIRECTORY-006 R1), since lys/delegation/v1 carries no window; the SECRETS-002 criteria that still wait on that window are R9 acceptance criteria 5 (a call after the window), 8 (SEC_LEASE_ATTENUATION) and 10 (SEC_PROVISIONAL_EXPIRY), and R7 acceptance criterion 8 (SEC_REINSTATE_CURRENT's expired handle). Open points: every point SECRETS-002 records open stays open, except that the audit line's signer and leaf schema and the sealed record's binding to its name and owner are settled broker-internally by CONTRACT.md's clauses once the lead accepts R2 after its adversarial review; estate-wide, whether that shape becomes a published lys format a stranger verifies offline stays open for a later brief with its own adversarial review. If R2's review leaves either clause unaccepted, R3, R4 and R7 stay blocked and say so. Canaries and logs: every canary leg plants a credential value and a raw handle value, searches every captured output and every file under the test's store and log directories, and carries positive controls; logs are captured by a tracing subscriber in crates/lys-secrets/tests/support/mod.rs; every secret byte lives in one redacting type in crates/lys-secrets/src/secret.rs, enforced by the counting test SEC3_SECRET_SCOPE, which runs under the gate's `cargo test` leg. Hours, re-costed for the corrections: prior estimate, replaced: 57 hours over 8 rows with a 64-hour ceiling. Now: R1 4 (no correction changes R1's content; re-costed for re-running every search at its recorded full commit hash in the five repositories, the empty ones included, with the manifold citations checked at the pinned commit rather than at a head that has moved past it), R2 9, R3 23 (store-key rotation built and tested as its own counted leg, SEC3_STORE_ROTATION), R4 15, R5 9, R6 6, R7 9, R8 5; total 80 hours, ceiling 90 hours; no figure is carried over unchanged from the prior estimate. Settled on main and not reopened: the seat's own login reaches its process at spawn as the one named exception (SECRETS-002 R5); OAuth refresh and spend caps are in scope (SECRETS-002 R4 and R9). Not delivered here: the screen and API legs of SEC_USE_NOT_LEND and SEC_PEOPLE_ONLY (C24, R3) and of SEC_REVOKE_STATES (C25, R4), whose library-level decision R3 and R4 deliver and which belong to a later Lys brief on the identity platform's own surface; SECRETS-002 R1 acceptance criterion 7 and the lys-core documentation edits SECRETS-002 R1, R6 and R9 list under files, because lys-core is unchanged under this brief and they wait on the delegation schema. Out: a release and demonstration row; a signed lease format; any cambium, manifold, aion or other engine file; any change to lys-core or a published lys format.

## Requirements

### R1: Record the baseline from source: the revolver's call site, the pool file's consumers, every credential path into a seat, and what the lys formats can carry

A documents-only row. docs/design/secrets/BASELINE.md records from source, by file and line: (a) the revolver's next-account call site; (b) every consumer of the account pool file; (c) every credential path into a seat today, each classed as exactly one of the login exception, a proxied credential, or neither; (d) what lys/delegation/v1 and lys/sealed-envelope/v1 can and cannot carry for a handle, a lease and a sealed record. A citation into a repository other than lys is spelled as the repository's name, the full commit hash it was read at, and a path relative to that repository's root with its line; the manifold repository is read at 3df5ac5f643e0479a0bc0464ef76e36f083c0246. Every repository searched is named with the exact search that was run, the full commit hash it ran at, and what it returned, including a search that returned nothing; this holds for the search for pool-file consumers and for the search for credential paths alike. The lease capacity records that lys/delegation/v1 has no not_after (crates/lys-core/src/delegation/mod.rs:218-221) as a fact the lease's own window covers (ADR-020); the sealed-record capacity records that lys/sealed-envelope/v1 seals with an empty AAD (crates/lys-core/src/seal/sealed_envelope.rs:182). The row SHALL NOT change any code, Cargo file or file under crates/; SHALL NOT decide the worker-side owner of the revolver change, which SECRETS-002 R3 records open (it records where the call site is, not who owns the change); SHALL NOT write a credential value; and SHALL NOT write a path that is one machine's. Estimate: 4 focused implementer hours.

**Acceptance:**
- BASELINE.md names the revolver's next-account call site as crates/manifold-node/src/seat/revolver.rs:172 inside turned() at line 158, in the manifold repository at 3df5ac5f643e0479a0bc0464ef76e36f083c0246; `git show 3df5ac5f643e0479a0bc0464ef76e36f083c0246:crates/manifold-node/src/seat/revolver.rs` prints `document.account = Some(next.to_owned());` at line 172.
- BASELINE.md states the exact search command it ran for pool-file consumers and the commit it ran at; re-running that command in the manifold repository at 3df5ac5f643e0479a0bc0464ef76e36f083c0246 returns the same set of files BASELINE.md lists, and that set includes all 7 files of crates/manifold-supervisor/src/values/pool/ and crates/manifold-core/src/seat_document/record.rs, each with at least one line number.
- BASELINE.md states, for each repository it searched for credential paths into a seat, the exact search command it ran and the full commit hash it ran at; re-running each command in its repository at that commit returns the same set of files BASELINE.md lists as credential-path entries for that repository, and the manifold set includes crates/manifold-core/src/seat_document/record.rs at 3df5ac5f643e0479a0bc0464ef76e36f083c0246, the path by which the pool value reaches the seat as environment, as one entry with at least one line number.
- Every credential-path entry in BASELINE.md, crates/manifold-core/src/seat_document/record.rs among them, carries exactly one of the three class labels `login exception`, `proxied` and `neither`, and BASELINE.md states the count of entries under each label; the three counts sum to the number of entries.
- BASELINE.md names each of the repositories manifold, aion, cambium, argus and haematite with the search run in it for the account pool file and for a credential path, and the number of files each search returned, including every 0.
- For each of the handle, the lease and the sealed record, BASELINE.md has one `can carry` statement and one `cannot carry` statement, each citing at least one of crates/lys-core/src/delegation/mod.rs:218-221, crates/lys-core/src/delegation/mod.rs:237-242, crates/lys-core/src/delegation/artifact.rs:145, crates/lys-core/src/delegation/artifact.rs:242-254, crates/lys-core/src/delegation/artifact.rs:350, crates/lys-core/src/delegation/artifact.rs:405 and crates/lys-core/src/seal/sealed_envelope.rs:182.
- `rg -n '(^|[[:space:](`])/[A-Za-z]' docs/design/secrets/BASELINE.md` prints 0 lines: every path in it is relative to a repository root.
- The row's commit changes exactly one authored file, docs/design/secrets/BASELINE.md: `git diff --name-only` over the row's commit lists no path under crates/ and no Cargo.toml or Cargo.lock.

**Files:**
- create: docs/design/secrets/BASELINE.md

**Checklist:**
- C19 — docs/design/secrets/BASELINE.md records, by repository, pinned commit, file and line, the revolver's next-account call site and every consumer of the account pool file.
- C20 — docs/design/secrets/BASELINE.md records every credential path into a seat today, by file and line, each classed as exactly one of: the login exception, a proxied credential, or neither.
- C21 — docs/design/secrets/BASELINE.md records, by file and line, what lys/delegation/v1 and lys/sealed-envelope/v1 can and cannot carry for a handle, a lease and a sealed record.

**Stories:**
- S15 (Lead, Reviews SECRETS-003's rows before any is dispatched) — As the lead, I want the revolver's call site, the pool file's consumers, every credential path into a seat and what the lys formats can carry recorded from source by file and line, so that the contract rests on what the code does today rather than on memory.

### R2: Write the broker contract before any code and put it through an adversarial review by a second party

A documents-only row, after R1. docs/design/secrets/CONTRACT.md states, as prose a test can disagree with, each clause under an id (K1, K2, ...): the invariant (a credential never passes through the agent; the one named exception is the seat's own login at spawn, SECRETS-002 R5); the key custody: which keys the broker holds (the store's encryption key, an X25519 static secret every store entry is sealed to with lys/sealed-envelope/v1 through lys-core unchanged, and the broker's Ed25519 audit key, which signs every audit line), where each is kept (a key file outside the store directory and outside the log directory, whose location the operator supplies when the broker starts), how the store is unlocked (the broker reads the key file at start, holds the key only in Zeroizing memory, and refuses by the name StoreKeyMissing to open the store when no key is supplied), and how the store key is rotated (a new store key is generated, every entry is opened with the old key and resealed to the new one, one audit line records the rotation naming the old and the new key id, each the SHA-256 fingerprint of that key's public half, and never a key byte, the old key presented afterwards is refused by the name StoreKeyRetired, and the old key file is removed only after every entry is resealed); the handle (how it is generated, its length, its binding to one identity, that the store keeps only its SHA-256 digest and never the raw handle, that it is compared digest to digest in constant time, and how the proxy authenticates whoever presents it: the presenter signs the handle id and the call's operation id with the Ed25519 key registered for the identity the handle is bound to, the proxy verifies that signature with lys-core's attestation verification unchanged and compares its signer to the bound identity, and a handle presented without that signature is refused); the lease, a broker record under the audit log (ADR-020), counting uses, time window and spend against a grant, with its atomic use step, the outcome of a retry whose first attempt's result is uncertain, the reservation before work and settlement after for a hard spend cap (SECRETS-002 R9), and the rule that the lease's not_after never extends past the window of the lys-identity grant it counts against (DIRECTORY-006 R1); the cancellation rule for calls already admitted when their handle is dropped, listing every outcome an in-flight call can receive, which becomes binding when the lead accepts this row, as SECRETS-002 R8 requires the rule to be settled in review before any code; the SpiceDB relations for handle use and sealed-record reads; the sealed-record binding, a broker-internal shape: because lys/sealed-envelope/v1 seals with an empty AAD (crates/lys-core/src/seal/sealed_envelope.rs:182 and :254) and lys-core is unchanged, the record's associated data (its entry id, its name and its owning identity's id, canonically encoded) is carried as the authenticated prefix of the sealed plaintext, and opening refuses by the name EntryBindingMismatch when the recovered associated data differs from the entry being read, which defeats ciphertext swapped between two entries; and the audit line, a broker-internal shape: its fields, which carry the handle id and never the raw handle nor any credential, because the transparency log can be read and replayed; its signer, the broker's audit key under the key-custody section; and its leaf schema, that field list. Each clause names the counted leg identifier of R3 to R8 that fires for it; the store-key rotation clause names SEC3_STORE_ROTATION. CONTRACT.md lists each point SECRETS-002 records open: (1) the delegation schema; (2) which lys/delegation/v1 subject and role pair a handle would use; (3) whose key signs an audit line; (4) the audit line's leaf schema; (5) the worker-side owner of the revolver change; (6) whether the revolver's answer is the login at spawn or a handle through the proxy; (7) the proxy-handle login path; (8) whether revoking a login token at the provider fails the seat's next call; (9) how a sealed record is bound to its name and owning identity; (10) the file that carries the lys revocation fold; (11) the shape of a time window carried in a signed delegation, a new version alongside v1; (12) the permission-freshness mechanism SEC_REVOKE_FRESHNESS proposes and DIRECTORY-006 R4 builds; (13) the identity lifecycle-state policy pending in CONFORMANCE rows 3.2 and 3.4. Points (3), (4) and (9) are settled broker-internally by the contract's clauses above and stay open estate-wide: whether that shape becomes a published lys format a stranger verifies offline is left to a later brief with its own adversarial review. The other ten stay open and no clause decides them. The clause answering a stale SpiceDB answer cites SEC_REVOKE_FRESHNESS and DIRECTORY-006 R4 and SHALL NOT choose between awaiting a fresh decision and refusing by name: the behaviour is the one the landed DIRECTORY-006 R4 permission check has. docs/design/secrets/reports/SECRETS-003-adversarial-review.md is written and committed by a reviewer who did not write or commit CONTRACT.md, and names both identities. It records every attack tried, each with its constructed inputs, the class it belongs to, the clause id that defeats it, and any clause changed as a result. The six classes are: a replayed or forged handle; a use spent or counted twice; an admitted call outliving the rule; a credential in a log; a sealed key read; a stale SpiceDB answer. IF the review leaves the audit-line clause or the sealed-record binding clause without a defeating answer to an attack against it, THEN the lead does not accept R2, and R3, R4 and R7, which build on those clauses, stay blocked and say so in their Blocked by. The row SHALL NOT change any code, Cargo file or file under crates/; SHALL NOT propose bytes for a signed lease or any new signed format (ADR-020); SHALL NOT change lys-core or any published lys format; SHALL NOT write a credential or key value; SHALL NOT decide points (1), (2), (5) to (8) and (10) to (13); and SHALL NOT start any code row before the lead accepts CONTRACT.md. Estimate: 9 focused implementer hours.

**Acceptance:**
- CONTRACT.md has exactly one section headed each of `Invariant`, `Key custody`, `Handle`, `Lease`, `Cancellation rule`, `SpiceDB relations`, `Sealed record binding` and `Audit line`, and every clause in them carries an id of the form K followed by a number.
- The `Key custody` section has one clause each for which keys the broker holds, where each is kept, how the store is unlocked, and how the store key is rotated: 4 clause ids; the where-kept clause places every key file outside the store directory and the log directory, the unlock clause names the refusal StoreKeyMissing; and the rotation clause names the refusal StoreKeyRetired and an audit line carrying the old and the new key id.
- The `Handle` section has one clause stating the store keeps the handle's SHA-256 digest and never the raw handle, and one clause stating how the proxy authenticates whoever presents a handle: the presenter's signature over the handle id and the call's operation id by the Ed25519 key registered for the bound identity, verified with lys-core's attestation verification unchanged.
- The `Lease` section has one clause each for the atomic use step, the retry outcome, spend reservation, spend settlement, and the lease's not_after never extending past the lys-identity grant's window (DIRECTORY-006 R1): 5 clause ids.
- The `Cancellation rule` section lists every outcome a call admitted before its handle is dropped can receive; each listed outcome names the R4 leg that exercises it.
- The `Sealed record binding` section states the record's associated data (entry id, name, owning identity id), carried as the authenticated prefix of the sealed plaintext, and the refusal EntryBindingMismatch; it cites crates/lys-core/src/seal/sealed_envelope.rs:182 and :254.
- The `Audit line` section lists the fields of an audit line; one of them is the handle id; none is a raw handle, a handle digest or a credential; it names the broker's audit key from the `Key custody` section as the signer and the field list as the leaf schema.
- Every clause id in CONTRACT.md is named by at least one counted leg identifier of R3 to R8 in its clause-to-leg table, the store-key rotation clause by SEC3_STORE_ROTATION, and every leg identifier in that table exists in this brief.
- CONTRACT.md's open section lists 13 points, one for each point numbered (1) to (13) in this requirement; points (3), (4) and (9) each name the clause that settles it broker-internally and state that it stays open estate-wide; the other 10 are decided in no clause.
- The report lists at least 11 attacks: a handle replayed from another identity; a handle guessed or forged; a raw handle recovered from the audit log or the store and replayed; a handle presented by a party that does not hold its bound identity's key; the last use spent twice by simultaneous requests; a retry counted as a second use; a call admitted before revocation outliving the rule; a credential reaching a log, error or Debug line; a sealed key read instead of used; a store entry's ciphertext swapped with another entry's; and a SpiceDB answer older than the revocation.
- Every attack in the report is tagged with exactly one of the six class names and names a clause id that exists in CONTRACT.md; each of the six classes has at least 1 attack.
- The report names its reviewer and the author of CONTRACT.md as two different identities, and they are two different commit authors: `git log --format='%an <%ae>' origin/main -- docs/design/secrets/CONTRACT.md | sort -u` and `git log --format='%an <%ae>' origin/main -- docs/design/secrets/reports/SECRETS-003-adversarial-review.md | sort -u` each print at least 1 line, and no line appears in both outputs.
- The row's commits change only docs/design/secrets/CONTRACT.md and docs/design/secrets/reports/SECRETS-003-adversarial-review.md: `git diff --name-only` lists no path under crates/ and no Cargo.toml or Cargo.lock.

**Files:**
- create: docs/design/secrets/CONTRACT.md
- create: docs/design/secrets/reports/SECRETS-003-adversarial-review.md

**Checklist:**
- C22 — docs/design/secrets/CONTRACT.md states the invariant, the key custody, the handle with how it is stored and how its presenter is authenticated, the lease with its atomic use step and retry outcome, the cancellation rule, the SpiceDB relations, the sealed-record binding and the audit line fields, and is accepted before any code row starts.
- C23 — docs/design/secrets/reports/SECRETS-003-adversarial-review.md records every attack tried in the six named classes, by a reviewer who did not write the contract and is a different commit author, and the contract clause defeating each.

**Stories:**
- S16 (Lead, Reviews SECRETS-003's rows before any is dispatched) — As the lead, I want the broker's contract and an adversarial review by someone other than its author before any code, so that every named attack has a clause defeating it before it can be built in.

### R3: Build the encrypted store, the redacting type, handles and leases in crates/lys-secrets

THE SYSTEM SHALL keep real credentials encrypted at rest in crates/lys-secrets, each store entry sealed with lys/sealed-envelope/v1 through lys-core unchanged to the store key CONTRACT.md's `Key custody` section names, with the entry's associated data bound as that contract's `Sealed record binding` section states; SHALL keep every credential byte and every raw handle byte in one redacting type in crates/lys-secrets/src/secret.rs, whose buffer is Zeroizing and whose Debug and Display print no byte of it; SHALL store a handle only as its SHA-256 digest; SHALL issue a handle bound to one identity only under a grant that traces to a person (ADR-003); and SHALL keep each lease as a broker record under the audit log (ADR-020), counting uses, time window and spend with one atomic step per use and a reservation before work and settlement after for a hard cap. IF the store is opened without its key, THEN THE SYSTEM SHALL refuse by the name StoreKeyMissing and read no entry. WHEN the store key is rotated, THE SYSTEM SHALL reseal every entry to the new key as CONTRACT.md's `Key custody` section states and append one audit line naming the old and the new key id. IF the store is opened with a key it has rotated away from, THEN THE SYSTEM SHALL refuse by the name StoreKeyRetired and read no entry. IF an entry opens to associated data that differs from the entry being read, THEN THE SYSTEM SHALL refuse by the name EntryBindingMismatch and return no plaintext. WHEN a retry arrives for a use whose first outcome was uncertain, THE SYSTEM SHALL answer with that first outcome under the same operation id. Every issue, use, drop, reservation and settlement is one audit line appended through lys-log-store with the fields and signer CONTRACT.md's `Audit line` section states, carrying the handle id. THE SYSTEM SHALL NOT keep the store key or the audit key in the store directory; SHALL NOT write a raw handle to the store, a log or an audit line; SHALL NOT count a use twice; SHALL NOT let a lease's not_after extend past the window of the lys-identity grant it counts against (DIRECTORY-006 R1; ADR-020); SHALL NOT leave an entry readable under a store key it has rotated away from; SHALL NOT put a key byte in the rotation's audit line; SHALL NOT put a credential byte in any Debug, Display, error, log or audit output; and SHALL NOT change lys-core or any published lys format. Delivers SECRETS-002 R9 (all 10 of its acceptance criteria, SEC_LEASE_ATTENUATION, SEC_SHARED_ALLOWANCE and SEC_PROVISIONAL_EXPIRY among them) and SECRETS-002 R1's issuance criteria, its acceptance criteria 6 and 8 to 12 (SEC_USE_NOT_LEND, SEC_AFFIRMATIVE_LEND, SEC_PEOPLE_ONLY, SEC_ISSUE_RETRY, SEC_AUTHORITY_TRACE), read with crates/lys-secrets in place of the door (ADR-019, proposed). SECRETS-002 R9 acceptance criteria 5, 8 and 10 and this row's lease-window refusal leg wait on the lys-identity grant's window: DIRECTORY-006 R1 defines it and DIRECTORY-006 R4 enforces it at admission. For SEC_USE_NOT_LEND and SEC_PEOPLE_ONLY this row delivers the library-level decision each rests on, and the request each criterion names is made as a call to the broker's issuance in crates/lys-secrets. Not delivered here, and left to a later Lys brief on the identity platform's own surface, the lifecycle screen and its API, which calls the same library through Lys and never re-decides the rule: the browser-issuance leg and the direct API leg of SEC_USE_NOT_LEND (C24), and the direct tool/API leg of SEC_PEOPLE_ONLY (C24). Blocked by, in this order, and not by the whole of DIRECTORY-006, whose R5 and R6 this row does not wait on: (a) R1 and R2 accepted by the lead (CN5): `git log --oneline origin/main -- docs/design/secrets/BASELINE.md docs/design/secrets/CONTRACT.md` prints at least 1 commit, and `git show origin/main:docs/design/secrets/briefs/SECRETS-003.json | python3 -c 'import json,sys; r={q["id"]:q.get("review",{}).get("alignment") for q in json.load(sys.stdin)["requirements"]}; print(r["R1"] in ("aligned","fixed"), r["R2"] in ("aligned","fixed"))'` prints `True True`, since a review record's alignment of `aligned` or `fixed` (accepted after fixes) means the row was accepted, and `drifted`, or no review record, means it was not; (b) DIRECTORY-002 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-002.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (c) DIRECTORY-003 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-003.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (d) DIRECTORY-006 R1 (the grant contract and its time window) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/types.rs` prints at least 1 commit; (e) DIRECTORY-006 R2 (delegation admission) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/admission.rs` prints at least 1 commit; (f) DIRECTORY-006 R3 (signed grant events) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/events.rs` prints at least 1 commit; (g) DIRECTORY-006 R4 (revocation, expiry and the permission-freshness check) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/permission.rs` prints at least 1 commit; (h) the word on ADR-019 of the one who decided ADR-001, which ADR-019 amends: `git show origin/main:docs/design/decisions.json | python3 -c 'import json,sys; d={a["id"]:a for a in json.load(sys.stdin)["decisions"]}; print(d["ADR-019"]["status"], d["ADR-019"]["decided_by"] == d["ADR-001"]["decided_by"])'` prints `decided True`, since ADR-019's consequences state that, once decided, its decided_by is set to exactly ADR-001's decided_by value; if ADR-019 is not decided that way, this row's files are re-briefed as an amendment before it is dispatched. Each blocker's check runs from a clone of the lys repository after `git fetch origin`. If any check fails when the row is dispatched, the row stops and names the failed check. This row also builds on CONTRACT.md's `Audit line` and `Sealed record binding` clauses; if the lead has not accepted either, the row stays blocked and says so. Log capture: Logs are captured by the tracing subscriber crates/lys-secrets/tests/support/mod.rs installs, which records every event at every level into an in-memory buffer and into one file under the test's log directory. Counted legs, each asserting the number of cases it ran as well as their outcomes, so a loop that ran zero times fails, are listed in the acceptance: SEC3_STORE_REFUSALS, SEC3_STORE_KEY, SEC3_STORE_ROTATION, SEC3_STORE_RACE, SEC3_STORE_CANARY, SEC3_SECRET_SCOPE, SEC3_STORE_RECOVERY and SEC3_STORE_INFLIGHT. Estimate: 23 focused implementer hours.

**Acceptance:**
- SEC3_STORE_REFUSALS: one test per refusal leg: a handle presented by another identity, after its window, outside its scope, after its drop, and unknown; issuance under a grant that traces to no person; a use on an exhausted lease; a reservation beyond the spend cap; a lease not_after later than its lys-identity grant's window; and a read of an entry whose sealed bytes were swapped with another entry's in the store's files, refused as EntryBindingMismatch with 0 plaintext bytes returned. The test asserts 10 refusals, each by its named reason, and 0 uses counted.
- SEC3_STORE_KEY, key absent: a store is created and 3 entries are written; a byte search of every file under the store directory for the store key's secret bytes and for the audit key's secret bytes finds 0 occurrences of each; a positive control writes the store key's secret bytes into one file under the store directory and the same search finds exactly 1.
- SEC3_STORE_KEY, key refused: the same store opened with no key supplied is refused with StoreKeyMissing, and 0 entries are read.
- SEC3_STORE_ROTATION: a store holding 3 entries is rotated from key A to key B: each of the 3 entries opens under B, and 3 reads attempted with A return 0 plaintext bytes; opening the store with A is refused with StoreKeyRetired and 0 entries are read; the audit log gains exactly 1 rotation line, and it carries the key ids of A and B and 0 occurrences of either key's secret bytes; a byte search of every file under the test's store directory and log directory finds 0 occurrences of A's secret bytes, of B's secret bytes and of the canary credential planted in one of the 3 entries, and a positive control writes the canary credential into one file under the store directory and the same search finds exactly 1; the test asserts it ran 3 reads under B, 3 attempted reads under A and 1 refused open.
- SEC3_STORE_RACE: a lease of k = 3 uses receives 8 requests held at the use step until all 8 have arrived and then released together: exactly 3 are admitted and 5 refused, and the use count is 3; a lease of k = 1 receives 2 requests held and released the same way: exactly 1 is admitted; the test asserts it ran 10 cases in total.
- SEC3_STORE_CANARY: a canary credential and a canary raw handle value are planted in the store; the Debug and Display output of every type the row adds, every error it returns, every log line the tracing subscriber in crates/lys-secrets/tests/support/mod.rs captures and every audit line the row's tests exercise contain 0 occurrences of each planted value, and a byte search of every file under the test's store directory and log directory finds 0 occurrences of each; for each planted value, a first positive control writes it into one captured output and the same output search finds exactly 1, and a second positive control writes it into one file under the store directory and the same file search finds exactly 1. The store's handle records hold the planted handle's SHA-256 digest and 0 occurrences of its raw value.
- SEC3_SECRET_SCOPE: a counting test in crates/lys-secrets/tests/secret_scope.rs reads every .rs file under crates/lys-secrets/src and asserts the number of files it read equals the number a directory walk counts and is at least 1; it finds `Zeroizing` in exactly 1 file, crates/lys-secrets/src/secret.rs; and it finds calls to the redacting type's one byte-exposing method only in the files its allowlist names, which for this row is crates/lys-secrets/src/store.rs. A positive control runs the same scan over one planted source text that names `Zeroizing` outside secret.rs and calls the byte-exposing method from a file off the allowlist, and reports exactly 2 violations.
- SEC3_STORE_RECOVERY: the process is killed before and after each append boundary CONTRACT.md names for issue, use, drop, reservation, settlement and store-key rotation, then reopened: every answer equals a replay of the log, no use is lost or counted twice, every entry opens under exactly one store key, and the test asserts the number of kill points it ran equals twice the number of boundaries.
- SEC3_STORE_INFLIGHT: for a use reserved and not yet settled when its handle is dropped: each outcome CONTRACT.md's cancellation rule names is its own test, and the number of these tests equals the number of outcomes the rule names.
- Every SECRETS-002 acceptance criterion this row delivers is mapped by name: for each of SECRETS-002 R1 criteria 6, 8, 9, 10, 11 and 12 and R9 criteria 1 to 6, 8, 9 and 10, `rg -c 's002_r<N>_ac<M>' crates/lys-secrets/tests` with that criterion's requirement number and criterion number finds at least 1 test whose name carries it, and that test passes with crates/lys-secrets in place of the door. SECRETS-002 R9 criterion 7 is met by CONTRACT.md's open section, points (1) and (11). The 3 screen and API legs of SEC_USE_NOT_LEND and SEC_PEOPLE_ONLY are named in this brief's task as not delivered here.
- Each of this row's 8 SEC3_ leg identifiers appears in the name of at least one test under crates/lys-secrets/tests, and `cargo test --workspace --all-features` runs every one of them with 0 ignored.

**Files:**
- create: crates/lys-secrets/Cargo.toml
- create: crates/lys-secrets/src/lib.rs
- create: crates/lys-secrets/src/error.rs
- create: crates/lys-secrets/src/secret.rs
- create: crates/lys-secrets/src/store.rs
- create: crates/lys-secrets/src/key_rotation.rs
- create: crates/lys-secrets/src/handle.rs
- create: crates/lys-secrets/src/lease.rs
- create: crates/lys-secrets/src/audit.rs
- create: crates/lys-secrets/schema/secrets.zed
- create: crates/lys-secrets/tests/store_leases.rs
- create: crates/lys-secrets/tests/secret_scope.rs
- create: crates/lys-secrets/tests/support/mod.rs
- modify: Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C24 — The store, the redacting type, handles and leases in crates/lys-secrets deliver SECRETS-002 R9 and the issuance criteria of R1, with the store key kept out of the store directory, opening without it refused by name, the store key rotated with every entry readable under the new key and none under the old, swapped ciphertext refused, and every counted leg of their row passing; for SEC_USE_NOT_LEND and SEC_PEOPLE_ONLY they deliver the library-level decision, and the screen and API legs are recorded as not delivered here.

**Stories:**
- S17 (Implementer, Is dispatched one SECRETS-003 code row at a time) — As the implementer of a code row, I want the row to name the SECRETS-002 requirements it delivers, its counted legs, its hours and what blocks it, so that I build from one row and nothing is described twice.
- S18 (Person, Keeps credentials with the standalone identity platform) — As a person keeping credentials, I want the broker to come with Lys alone, so that I never have to install Cambium to keep a credential.

### R4: Build the proxy: check, swap, forward, record, rotate, and apply the cancellation rule

WHEN a seat makes an outbound call carrying its handle, THE SYSTEM SHALL have the proxy in crates/lys-secrets authenticate the presenter as CONTRACT.md's `Handle` section states, check SpiceDB, spend one use of the lease, swap the handle for the real credential, forward the call and append one audit line naming the seat, the handle id, the real account and the time; take the next real account in turn under one handle and record which one served; refuse every new call on a dropped handle at the next check with no restart; and give every call admitted before a drop exactly the outcome CONTRACT.md's cancellation rule names, recorded in the audit. WHEN a handle is revoked, THE SYSTEM SHALL move it through the revocation states SEC_REVOKE_STATES names and append one audit line for each transition; R6 and R7 move a login and a sealed record through them the same way. WHEN the permission decision may be stale, THE SYSTEM SHALL call the permission check DIRECTORY-006 R4 lands in crates/lys-identity/src/grants/permission.rs and act on its answer. THE SYSTEM SHALL NOT forward a call whose handle is unknown, dropped or bound to another identity, whose SpiceDB answer is not a permit, whose lease is exhausted, or whose accounts are all rested; SHALL NOT forward a call on a stale permit; SHALL NOT let a credential byte reach the seat's side of the exchange or any log; and SHALL NOT write a raw handle to any audit line. Delivers SECRETS-002 R1's proxy criteria, its acceptance criteria 1 to 5; SECRETS-002 R2 (all 5 criteria, SEC_ROTATION_SCOPE among them); SECRETS-002 R7's handle case, its acceptance criteria 1 and 4 to 8 (SEC_REVOKE_CHAIN, SEC_REVOKE_FRESHNESS, SEC_REVOKE_STATES, SEC_REINSTATE_CURRENT); and SECRETS-002 R8 (all 3 criteria), read with crates/lys-secrets in place of the door (ADR-019, proposed). SEC_REVOKE_FRESHNESS has one fixed expected behaviour: the one the landed DIRECTORY-006 R4 permission check returns for a replica behind the revocation revision, with 0 calls forwarded; this row does not choose between awaiting and refusing. SECRETS-002 R7 acceptance criterion 8 (SEC_REINSTATE_CURRENT's expired handle) waits on the lys-identity grant's window, DIRECTORY-006 R1 and R4. For SEC_REVOKE_STATES this row delivers the library-level decision. Not delivered here, and left to a later Lys brief on the identity platform's own surface, the lifecycle screen and its API, which calls the same library through Lys and never re-decides the rule: the API and screen legs of SEC_REVOKE_STATES (C25). Blocked by, in this order, and not by the whole of DIRECTORY-006, whose R5 and R6 this row does not wait on: (a) R3 landed, since this row edits R3's crates/lys-secrets/src/lib.rs: `git log --oneline origin/main -- crates/lys-secrets/src/lease.rs` prints at least 1 commit; (b) R1 and R2 accepted by the lead (CN5): `git log --oneline origin/main -- docs/design/secrets/BASELINE.md docs/design/secrets/CONTRACT.md` prints at least 1 commit, and `git show origin/main:docs/design/secrets/briefs/SECRETS-003.json | python3 -c 'import json,sys; r={q["id"]:q.get("review",{}).get("alignment") for q in json.load(sys.stdin)["requirements"]}; print(r["R1"] in ("aligned","fixed"), r["R2"] in ("aligned","fixed"))'` prints `True True`, since a review record's alignment of `aligned` or `fixed` (accepted after fixes) means the row was accepted, and `drifted`, or no review record, means it was not; (c) DIRECTORY-002 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-002.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (d) DIRECTORY-003 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-003.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (e) DIRECTORY-006 R1 (the grant contract and its time window) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/types.rs` prints at least 1 commit; (f) DIRECTORY-006 R2 (delegation admission) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/admission.rs` prints at least 1 commit; (g) DIRECTORY-006 R3 (signed grant events) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/events.rs` prints at least 1 commit; (h) DIRECTORY-006 R4 (revocation, expiry and the permission-freshness check) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/permission.rs` prints at least 1 commit; (i) the word on ADR-019 of the one who decided ADR-001, which ADR-019 amends: `git show origin/main:docs/design/decisions.json | python3 -c 'import json,sys; d={a["id"]:a for a in json.load(sys.stdin)["decisions"]}; print(d["ADR-019"]["status"], d["ADR-019"]["decided_by"] == d["ADR-001"]["decided_by"])'` prints `decided True`, since ADR-019's consequences state that, once decided, its decided_by is set to exactly ADR-001's decided_by value; if ADR-019 is not decided that way, this row's files are re-briefed as an amendment before it is dispatched; (j) the review of docs/design/identity/CONFORMANCE.md row 3.2 (the identity states, LIFECYCLE-STATES), as SECRETS-002 R7 requires, which ADR-011 records: `git show origin/main:docs/design/decisions.json | python3 -c 'import json,sys; d={a["id"]:a for a in json.load(sys.stdin)["decisions"]}; print(d["ADR-011"]["status"])'` prints `decided`; (k) the review of docs/design/identity/CONFORMANCE.md row 3.4 (the emergency stop that revokes tokens at once and asks sessions to end, LIFECYCLE, new), as SECRETS-002 R7 requires, which ADR-011 does not record: its review is recorded by a decided entry in docs/design/decisions.json whose context names `CONFORMANCE.md row 3.4`, and `git show origin/main:docs/design/decisions.json | python3 -c 'import json,sys; print(sum(1 for a in json.load(sys.stdin)["decisions"] if a["status"] == "decided" and "CONFORMANCE.md row 3.4" in a["context"]))'` prints a number of 1 or more. Each blocker's check runs from a clone of the lys repository after `git fetch origin`. If any check fails when the row is dispatched, the row stops and names the failed check. This row also builds on CONTRACT.md's `Audit line` clause; if the lead has not accepted it, the row stays blocked and says so. Log capture: Logs are captured by the tracing subscriber crates/lys-secrets/tests/support/mod.rs installs, which records every event at every level into an in-memory buffer and into one file under the test's log directory. Counted legs are listed in the acceptance: SEC3_PROXY_REFUSALS, SEC3_PROXY_RACE, SEC3_PROXY_CANARY, SEC3_PROXY_RECOVERY and SEC3_PROXY_INFLIGHT. Estimate: 15 focused implementer hours.

**Acceptance:**
- SEC3_PROXY_REFUSALS: one test per refusal leg: an unknown handle, a dropped handle, a handle bound to another identity, a SpiceDB answer that is not a permit, an exhausted lease, and every account rested. The test asserts 6 refusals, each by its named reason, and an upstream test double request count of 0 across all 6.
- SEC3_PROXY_RACE: a lease of k = 3 uses receives 8 calls held at the use step and released together: the upstream test double receives exactly 3 requests, 5 calls are refused, and the test asserts it ran 8 cases.
- SEC3_PROXY_CANARY: a canary credential served by the proxy and a canary raw handle value are planted in the store; the Debug and Display output of every type the row adds, every error it returns, every log line the tracing subscriber in crates/lys-secrets/tests/support/mod.rs captures and every audit line the row's tests exercise contain 0 occurrences of each planted value, and a byte search of every file under the test's store directory and log directory finds 0 occurrences of each; for each planted value, a first positive control writes it into one captured output and the same output search finds exactly 1, and a second positive control writes it into one file under the store directory and the same file search finds exactly 1. The seat-visible request and response of every forwarded call also contain 0 occurrences of the canary credential. This row adds crates/lys-secrets/src/proxy.rs to the byte-exposing allowlist in crates/lys-secrets/tests/secret_scope.rs, and SEC3_SECRET_SCOPE passes with it.
- SEC3_PROXY_RECOVERY: the process is killed before and after each append boundary the contract names for admission, forwarding and the audit line, then reopened: every answer equals a replay of the log, each forwarded call has exactly 1 audit line, and the test asserts the number of kill points it ran equals twice the number of boundaries.
- SEC3_PROXY_INFLIGHT: a call admitted against a slow upstream test double when its handle is dropped: each outcome CONTRACT.md's cancellation rule names is its own test, and the number of these tests equals the number of outcomes the rule names.
- A test revokes one handle while withholding the provider's acknowledgement and then delivers the matching provider outcome: the broker's state for the handle moves through exactly 2 transitions and exactly 2 audit lines are appended, one per transition.
- SEC_REVOKE_FRESHNESS: with the permission replica held behind the revocation revision, a fresh call reaches the proxy; the proxy calls the permission check in crates/lys-identity/src/grants/permission.rs; the test asserts the outcome that check returns for a stale replica, the same outcome DIRECTORY-006 R4's own test asserts, and an upstream test double request count of 0.
- crates/lys-secrets/src/in_flight.rs carries module docs headed `Cancellation rule` that name each outcome CONTRACT.md's cancellation rule names, in the row's first commit that adds in_flight.rs.
- Every SECRETS-002 acceptance criterion this row delivers is mapped by name: for each of SECRETS-002 R1 criteria 1 to 5, R2 criteria 1 to 5, R7 criteria 1, 5, 6, 7 and 8, and R8 criteria 2 and 3, `rg -c 's002_r<N>_ac<M>' crates/lys-secrets/tests` with that criterion's requirement number and criterion number finds at least 1 test whose name carries it, and that test passes with crates/lys-secrets in place of the door. SECRETS-002 R7 criterion 4 is met by CONTRACT.md's open section, points (8) and (10), and R8 criterion 1 by the in_flight.rs module docs above. The API and screen legs of SEC_REVOKE_STATES are named in this brief's task as not delivered here.
- Each of this row's 5 SEC3_ leg identifiers appears in the name of at least one test under crates/lys-secrets/tests, and `cargo test --workspace --all-features` runs every one of them with 0 ignored.

**Files:**
- create: crates/lys-secrets/src/proxy.rs
- create: crates/lys-secrets/src/rotation.rs
- create: crates/lys-secrets/src/in_flight.rs
- create: crates/lys-secrets/tests/proxy.rs
- modify: crates/lys-secrets/Cargo.toml
- modify: Cargo.lock
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/tests/support/mod.rs
- modify: crates/lys-secrets/tests/secret_scope.rs

**Checklist:**
- C25 — The proxy in crates/lys-secrets delivers SECRETS-002 R1's proxy criteria, R2, R8 and R7's handle case, with every counted leg of its row passing; for SEC_REVOKE_STATES it delivers the library-level decision, and the API and screen legs are recorded as not delivered here.

**Stories:**
- S17 (Implementer, Is dispatched one SECRETS-003 code row at a time) — As the implementer of a code row, I want the row to name the SECRETS-002 requirements it delivers, its counted legs, its hours and what blocks it, so that I build from one row and nothing is described twice.
- S18 (Person, Keeps credentials with the standalone identity platform) — As a person keeping credentials, I want the broker to come with Lys alone, so that I never have to install Cambium to keep a credential.

### R5: Refresh OAuth at the proxy; the seat never sees a token

WHEN a call arrives on a handle whose credential is an OAuth grant, THE SYSTEM SHALL have the proxy in crates/lys-secrets swap the handle for a live access token and refresh it itself from the refresh token in the store when it has expired, and SHALL keep the service grant's selected provider account, client and scopes as its provenance. WHEN several calls arrive at once on a handle whose access token has expired, THE SYSTEM SHALL make exactly one refresh request to the provider. THE SYSTEM SHALL NOT let the seat see the refresh token or any access token, SHALL NOT rebind or refresh a grant from a matching sign-in email, and SHALL NOT refresh for a new client without a named reconnect. Delivers SECRETS-002 R4 (all 5 criteria, SEC_OAUTH_ACCOUNT among them), read with crates/lys-secrets in place of the door (ADR-019, proposed). Blocked by, in this order, and not by the whole of DIRECTORY-006, whose R5 and R6 this row does not wait on: (a) R3 and R4 landed, in that order, since this row builds on R4's proxy and revocation states: `git log --oneline origin/main -- crates/lys-secrets/src/proxy.rs crates/lys-secrets/src/in_flight.rs` prints at least 1 commit; (b) R1 and R2 accepted by the lead (CN5): `git log --oneline origin/main -- docs/design/secrets/BASELINE.md docs/design/secrets/CONTRACT.md` prints at least 1 commit, and `git show origin/main:docs/design/secrets/briefs/SECRETS-003.json | python3 -c 'import json,sys; r={q["id"]:q.get("review",{}).get("alignment") for q in json.load(sys.stdin)["requirements"]}; print(r["R1"] in ("aligned","fixed"), r["R2"] in ("aligned","fixed"))'` prints `True True`, since a review record's alignment of `aligned` or `fixed` (accepted after fixes) means the row was accepted, and `drifted`, or no review record, means it was not; (c) DIRECTORY-002 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-002.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (d) DIRECTORY-003 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-003.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (e) DIRECTORY-006 R1 (the grant contract and its time window) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/types.rs` prints at least 1 commit; (f) DIRECTORY-006 R2 (delegation admission) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/admission.rs` prints at least 1 commit; (g) DIRECTORY-006 R3 (signed grant events) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/events.rs` prints at least 1 commit; (h) DIRECTORY-006 R4 (revocation, expiry and the permission-freshness check) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/permission.rs` prints at least 1 commit; (i) the word on ADR-019 of the one who decided ADR-001, which ADR-019 amends: `git show origin/main:docs/design/decisions.json | python3 -c 'import json,sys; d={a["id"]:a for a in json.load(sys.stdin)["decisions"]}; print(d["ADR-019"]["status"], d["ADR-019"]["decided_by"] == d["ADR-001"]["decided_by"])'` prints `decided True`, since ADR-019's consequences state that, once decided, its decided_by is set to exactly ADR-001's decided_by value; if ADR-019 is not decided that way, this row's files are re-briefed as an amendment before it is dispatched. Each blocker's check runs from a clone of the lys repository after `git fetch origin`. If any check fails when the row is dispatched, the row stops and names the failed check. Log capture: Logs are captured by the tracing subscriber crates/lys-secrets/tests/support/mod.rs installs, which records every event at every level into an in-memory buffer and into one file under the test's log directory. Counted legs are listed in the acceptance: SEC3_OAUTH_REFUSALS, SEC3_OAUTH_RACE, SEC3_OAUTH_LEASE_RACE, SEC3_OAUTH_CANARY, SEC3_OAUTH_RECOVERY and SEC3_OAUTH_INFLIGHT. Estimate: 9 focused implementer hours.

**Acceptance:**
- SEC3_OAUTH_REFUSALS: one test per refusal leg: a dropped OAuth handle, a handle bound to another identity, a SpiceDB answer that is not a permit, an exhausted lease, and a new client without a named reconnect. The test asserts 5 refusals, each by its named reason, and a provider test double refresh count of 0 across all 5.
- SEC3_OAUTH_RACE: 3 calls on one OAuth handle whose access token has expired are held at the refresh step until all 3 have arrived and then released together: the provider test double receives exactly 1 refresh request, and the test asserts it ran 3 cases.
- SEC3_OAUTH_LEASE_RACE: a lease of k = 3 uses on an OAuth handle whose access token has expired receives 8 calls held at the use step and released together: exactly 3 are forwarded, 5 are refused, and the test asserts it ran 8 cases.
- SEC3_OAUTH_CANARY: a refresh-token canary, an access-token canary and a canary raw handle value are planted in the store; the Debug and Display output of every type the row adds, every error it returns, every log line the tracing subscriber in crates/lys-secrets/tests/support/mod.rs captures and every audit line the row's tests exercise contain 0 occurrences of each planted value, and a byte search of every file under the test's store directory and log directory finds 0 occurrences of each; for each planted value, a first positive control writes it into one captured output and the same output search finds exactly 1, and a second positive control writes it into one file under the store directory and the same file search finds exactly 1. This row adds crates/lys-secrets/src/oauth.rs to the byte-exposing allowlist in crates/lys-secrets/tests/secret_scope.rs, and SEC3_SECRET_SCOPE passes with it.
- SEC3_OAUTH_RECOVERY: the process is killed before and after each append boundary the contract names for a refresh and its audit line, then reopened: the store holds exactly one live access token, no refresh is repeated for a refresh already recorded, and the test asserts the number of kill points it ran equals twice the number of boundaries.
- SEC3_OAUTH_INFLIGHT: a refresh in flight when its handle is dropped: each outcome CONTRACT.md's cancellation rule names is its own test, and the number of these tests equals the number of outcomes the rule names.
- Every SECRETS-002 acceptance criterion this row delivers is mapped by name: for each of SECRETS-002 R4 criteria 1 to 5, `rg -c 's002_r<N>_ac<M>' crates/lys-secrets/tests` with that criterion's requirement number and criterion number finds at least 1 test whose name carries it, and that test passes with crates/lys-secrets in place of the door.
- Each of this row's 6 SEC3_ leg identifiers appears in the name of at least one test under crates/lys-secrets/tests, and `cargo test --workspace --all-features` runs every one of them with 0 ignored.

**Files:**
- create: crates/lys-secrets/src/oauth.rs
- create: crates/lys-secrets/tests/oauth.rs
- modify: crates/lys-secrets/Cargo.toml
- modify: Cargo.lock
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/tests/support/mod.rs
- modify: crates/lys-secrets/tests/secret_scope.rs

**Checklist:**
- C26 — OAuth refresh at the proxy in crates/lys-secrets delivers SECRETS-002 R4, with every counted leg of its row passing.

**Stories:**
- S17 (Implementer, Is dispatched one SECRETS-003 code row at a time) — As the implementer of a code row, I want the row to name the SECRETS-002 requirements it delivers, its counted legs, its hours and what blocks it, so that I build from one row and nothing is described twice.
- S18 (Person, Keeps credentials with the standalone identity platform) — As a person keeping credentials, I want the broker to come with Lys alone, so that I never have to install Cambium to keep a credential.

### R6: Answer the seat's own login at spawn, and record which login went to which seat

WHEN the engine that runs a seat starts it, THE SYSTEM SHALL have crates/lys-secrets answer the seat's own login, read from the store and rotating at spawn, and SHALL record which account's login went to which seat; this is the one named exception to the invariant (SECRETS-002 R5). WHEN a seat's login is revoked, THE SYSTEM SHALL answer which seat the login went to, SHALL move the login through the revocation states R4 defines, appending one audit line for each transition, and SHALL leave ending the seat to the engine that runs it. THE SYSTEM SHALL NOT name or depend on any one engine, SHALL NOT change a seat's login between spawns, SHALL NOT put the login into any log, error or audit line, and SHALL NOT build anything that depends on the proxy-handle login path. Delivers SECRETS-002 R5 (all 4 criteria) and SECRETS-002 R7's login-token case, its acceptance criterion 2, read with crates/lys-secrets in place of the door (ADR-019, proposed). Blocked by, in this order, and not by the whole of DIRECTORY-006, whose R5 and R6 this row does not wait on: (a) R3 and R4 landed, in that order, since this row builds on R4's proxy and revocation states: `git log --oneline origin/main -- crates/lys-secrets/src/proxy.rs crates/lys-secrets/src/in_flight.rs` prints at least 1 commit; (b) R1 and R2 accepted by the lead (CN5): `git log --oneline origin/main -- docs/design/secrets/BASELINE.md docs/design/secrets/CONTRACT.md` prints at least 1 commit, and `git show origin/main:docs/design/secrets/briefs/SECRETS-003.json | python3 -c 'import json,sys; r={q["id"]:q.get("review",{}).get("alignment") for q in json.load(sys.stdin)["requirements"]}; print(r["R1"] in ("aligned","fixed"), r["R2"] in ("aligned","fixed"))'` prints `True True`, since a review record's alignment of `aligned` or `fixed` (accepted after fixes) means the row was accepted, and `drifted`, or no review record, means it was not; (c) DIRECTORY-002 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-002.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (d) DIRECTORY-003 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-003.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (e) DIRECTORY-006 R1 (the grant contract and its time window) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/types.rs` prints at least 1 commit; (f) DIRECTORY-006 R2 (delegation admission) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/admission.rs` prints at least 1 commit; (g) DIRECTORY-006 R3 (signed grant events) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/events.rs` prints at least 1 commit; (h) DIRECTORY-006 R4 (revocation, expiry and the permission-freshness check) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/permission.rs` prints at least 1 commit; (i) the word on ADR-019 of the one who decided ADR-001, which ADR-019 amends: `git show origin/main:docs/design/decisions.json | python3 -c 'import json,sys; d={a["id"]:a for a in json.load(sys.stdin)["decisions"]}; print(d["ADR-019"]["status"], d["ADR-019"]["decided_by"] == d["ADR-001"]["decided_by"])'` prints `decided True`, since ADR-019's consequences state that, once decided, its decided_by is set to exactly ADR-001's decided_by value; if ADR-019 is not decided that way, this row's files are re-briefed as an amendment before it is dispatched. Each blocker's check runs from a clone of the lys repository after `git fetch origin`. If any check fails when the row is dispatched, the row stops and names the failed check. Log capture: Logs are captured by the tracing subscriber crates/lys-secrets/tests/support/mod.rs installs, which records every event at every level into an in-memory buffer and into one file under the test's log directory. Counted legs are listed in the acceptance: SEC3_SPAWN_REFUSALS, SEC3_SPAWN_RACE, SEC3_SPAWN_CANARY, SEC3_SPAWN_RECOVERY and SEC3_SPAWN_INFLIGHT. Estimate: 6 focused implementer hours.

**Acceptance:**
- SEC3_SPAWN_REFUSALS: one test per refusal leg: a spawn for a seat with no grant that traces to a person, a spawn on a dropped handle, a spawn for a seat bound to another identity, and a spawn when every login account is rested. The test asserts 4 refusals, each by its named reason, and 0 logins answered.
- SEC3_SPAWN_RACE: a lease of k = 3 spawns receives 8 spawn requests held at the use step and released together: exactly 3 are answered with a login, 5 are refused, the store records exactly 3 seat-to-account pairs, and the test asserts it ran 8 cases.
- SEC3_SPAWN_CANARY: a canary login and a canary raw handle value are planted in the store; the Debug and Display output of every type the row adds, every error it returns, every log line the tracing subscriber in crates/lys-secrets/tests/support/mod.rs captures and every audit line the row's tests exercise contain 0 occurrences of each planted value, and a byte search of every file under the test's store directory and log directory finds 0 occurrences of each; for each planted value, a first positive control writes it into one captured output and the same output search finds exactly 1, and a second positive control writes it into one file under the store directory and the same file search finds exactly 1. The spawn answer itself, the one named exception, is excluded from the output search and is the only output that carries the canary login. This row adds crates/lys-secrets/src/spawn_login.rs to the byte-exposing allowlist in crates/lys-secrets/tests/secret_scope.rs, and SEC3_SECRET_SCOPE passes with it.
- SEC3_SPAWN_RECOVERY: the process is killed before and after each append boundary the contract names for a spawn answer and its seat-to-account record, then reopened: every recorded pair equals a replay of the log, no seat has two logins recorded for one spawn, and the test asserts the number of kill points it ran equals twice the number of boundaries.
- SEC3_SPAWN_INFLIGHT: a spawn answer in flight when its handle is dropped: each outcome CONTRACT.md's cancellation rule names is its own test, and the number of these tests equals the number of outcomes the rule names.
- A test revokes one seat's login: the broker's answer names that seat, exactly 1 audit line is appended for the transition, and the test double for the engine records 0 processes ended by the broker.
- Every SECRETS-002 acceptance criterion this row delivers is mapped by name: for each of SECRETS-002 R5 criteria 1, 2 and 3 and R7 criterion 2, `rg -c 's002_r<N>_ac<M>' crates/lys-secrets/tests` with that criterion's requirement number and criterion number finds at least 1 test whose name carries it, and that test passes with crates/lys-secrets in place of the door. SECRETS-002 R5 criterion 4 is met by CONTRACT.md's open section, points (7) and (8).
- Each of this row's 5 SEC3_ leg identifiers appears in the name of at least one test under crates/lys-secrets/tests, and `cargo test --workspace --all-features` runs every one of them with 0 ignored.

**Files:**
- create: crates/lys-secrets/src/spawn_login.rs
- create: crates/lys-secrets/tests/spawn_login.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/tests/support/mod.rs
- modify: crates/lys-secrets/tests/secret_scope.rs

**Checklist:**
- C27 — The seat's own login at spawn from crates/lys-secrets delivers SECRETS-002 R5 and R7's login-token case, with every counted leg of its row passing.

**Stories:**
- S17 (Implementer, Is dispatched one SECRETS-003 code row at a time) — As the implementer of a code row, I want the row to name the SECRETS-002 requirements it delivers, its counted legs, its hours and what blocks it, so that I build from one row and nothing is described twice.
- S18 (Person, Keeps credentials with the standalone identity platform) — As a person keeping credentials, I want the broker to come with Lys alone, so that I never have to install Cambium to keep a credential.

### R7: Keep sealed records under SpiceDB relations, and use keys without reading them

THE SYSTEM SHALL keep keys that cannot rotate, and memories an identity wants kept secret, as sealed records in crates/lys-secrets' store, each tagged in SpiceDB with the identities that may read it and bound to its name and owning identity as CONTRACT.md's `Sealed record binding` section states. WHEN a seat asks for a sealed record by name, THE SYSTEM SHALL check the relation, return only the piece asked for, and append one audit line. WHEN a relation is removed after a read, THE SYSTEM SHALL refuse the next read, SHALL keep the earlier audit line, and SHALL move the sealed record through the revocation states R4 defines, appending one audit line for each transition. THE SYSTEM SHALL NOT return a key-class record by reading it (a key is used through the proxy of R4), SHALL NOT decrypt a record for an identity the relation does not permit, SHALL NOT disclose the existence, title or owner of a record the caller cannot discover, and SHALL NOT claim an earlier read is undone. Delivers SECRETS-002 R6 (all 6 criteria, SEC_PRIVATE_SCOPE among them) and SECRETS-002 R7's sealed-knowledge case, its acceptance criterion 3, read with crates/lys-secrets in place of the door (ADR-019, proposed). Blocked by, in this order, and not by the whole of DIRECTORY-006, whose R5 and R6 this row does not wait on: (a) R3 and R4 landed, in that order, since this row builds on R4's proxy and revocation states: `git log --oneline origin/main -- crates/lys-secrets/src/proxy.rs crates/lys-secrets/src/in_flight.rs` prints at least 1 commit; (b) R1 and R2 accepted by the lead (CN5): `git log --oneline origin/main -- docs/design/secrets/BASELINE.md docs/design/secrets/CONTRACT.md` prints at least 1 commit, and `git show origin/main:docs/design/secrets/briefs/SECRETS-003.json | python3 -c 'import json,sys; r={q["id"]:q.get("review",{}).get("alignment") for q in json.load(sys.stdin)["requirements"]}; print(r["R1"] in ("aligned","fixed"), r["R2"] in ("aligned","fixed"))'` prints `True True`, since a review record's alignment of `aligned` or `fixed` (accepted after fixes) means the row was accepted, and `drifted`, or no review record, means it was not; (c) DIRECTORY-002 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-002.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (d) DIRECTORY-003 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-003.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (e) DIRECTORY-006 R1 (the grant contract and its time window) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/types.rs` prints at least 1 commit; (f) DIRECTORY-006 R2 (delegation admission) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/admission.rs` prints at least 1 commit; (g) DIRECTORY-006 R3 (signed grant events) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/events.rs` prints at least 1 commit; (h) DIRECTORY-006 R4 (revocation, expiry and the permission-freshness check) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/permission.rs` prints at least 1 commit; (i) the word on ADR-019 of the one who decided ADR-001, which ADR-019 amends: `git show origin/main:docs/design/decisions.json | python3 -c 'import json,sys; d={a["id"]:a for a in json.load(sys.stdin)["decisions"]}; print(d["ADR-019"]["status"], d["ADR-019"]["decided_by"] == d["ADR-001"]["decided_by"])'` prints `decided True`, since ADR-019's consequences state that, once decided, its decided_by is set to exactly ADR-001's decided_by value; if ADR-019 is not decided that way, this row's files are re-briefed as an amendment before it is dispatched. Each blocker's check runs from a clone of the lys repository after `git fetch origin`. If any check fails when the row is dispatched, the row stops and names the failed check. This row builds on CONTRACT.md's `Sealed record binding` clause; if the lead has not accepted it, the row stays blocked and says so. Log capture: Logs are captured by the tracing subscriber crates/lys-secrets/tests/support/mod.rs installs, which records every event at every level into an in-memory buffer and into one file under the test's log directory. Counted legs are listed in the acceptance: SEC3_SEALED_REFUSALS, SEC3_SEALED_RACE, SEC3_SEALED_CANARY, SEC3_SEALED_RECOVERY and SEC3_SEALED_INFLIGHT. Estimate: 9 focused implementer hours.

**Acceptance:**
- SEC3_SEALED_REFUSALS: one test per refusal leg: an identity without the relation, the same identity after the relation is removed, a read of a key-class record, a record moved to another identity's name, an unknown name, and another person's record asked for by its known id. The test asserts 6 refusals, each by its named reason, 0 texts returned and 0 decryptions.
- SEC3_SEALED_RACE: a lease of k = 3 reads on one record receives 8 read requests held at the use step and released together: exactly 3 return text, 5 are refused, exactly 3 read audit lines are appended, and the test asserts it ran 8 cases.
- SEC3_SEALED_CANARY: a canary sealed-record plaintext and a canary raw handle value are planted in the store; the Debug and Display output of every type the row adds, every error it returns, every log line the tracing subscriber in crates/lys-secrets/tests/support/mod.rs captures and every audit line the row's tests exercise contain 0 occurrences of each planted value, and a byte search of every file under the test's store directory and log directory finds 0 occurrences of each; for each planted value, a first positive control writes it into one captured output and the same output search finds exactly 1, and a second positive control writes it into one file under the store directory and the same file search finds exactly 1. Every memory file the broker writes is searched too, with 0 matches. This row adds crates/lys-secrets/src/sealed.rs to the byte-exposing allowlist in crates/lys-secrets/tests/secret_scope.rs, and SEC3_SECRET_SCOPE passes with it.
- SEC3_SEALED_RECOVERY: the process is killed before and after each append boundary the contract names for sealing, a read and a relation change, then reopened: every answer equals a replay of the log, and the test asserts the number of kill points it ran equals twice the number of boundaries.
- SEC3_SEALED_INFLIGHT: a read in flight when the handle it was admitted under is dropped: each outcome CONTRACT.md's cancellation rule names is its own test, and the number of these tests equals the number of outcomes the rule names.
- A test removes one sealed record's relation after one read: exactly 1 audit line is appended for the transition, the earlier read's audit line is still in the log, and the next read is refused.
- Every SECRETS-002 acceptance criterion this row delivers is mapped by name: for each of SECRETS-002 R6 criteria 1 to 6 and R7 criterion 3, `rg -c 's002_r<N>_ac<M>' crates/lys-secrets/tests` with that criterion's requirement number and criterion number finds at least 1 test whose name carries it, and that test passes with crates/lys-secrets in place of the door.
- Each of this row's 5 SEC3_ leg identifiers appears in the name of at least one test under crates/lys-secrets/tests, and `cargo test --workspace --all-features` runs every one of them with 0 ignored.

**Files:**
- create: crates/lys-secrets/src/sealed.rs
- create: crates/lys-secrets/tests/sealed.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/schema/secrets.zed
- modify: crates/lys-secrets/tests/support/mod.rs
- modify: crates/lys-secrets/tests/secret_scope.rs

**Checklist:**
- C28 — Sealed records in crates/lys-secrets deliver SECRETS-002 R6 and R7's sealed-knowledge case, with every counted leg of their row passing.

**Stories:**
- S17 (Implementer, Is dispatched one SECRETS-003 code row at a time) — As the implementer of a code row, I want the row to name the SECRETS-002 requirements it delivers, its counted legs, its hours and what blocks it, so that I build from one row and nothing is described twice.
- S18 (Person, Keeps credentials with the standalone identity platform) — As a person keeping credentials, I want the broker to come with Lys alone, so that I never have to install Cambium to keep a credential.

### R8: Answer the revolver's ask for its next account from the broker

WHEN a worker that runs sessions asks for its next account, THE SYSTEM SHALL have crates/lys-secrets answer from the same rotation as R4, by account name, attributed to the asking seat in one audit line. THE SYSTEM SHALL NOT answer with a credential, SHALL NOT answer when every account behind the handle is rested, SHALL NOT start an engine process or depend on an engine crate, and SHALL NOT name any engine file: the worker-side owner stays open as SECRETS-002 R3 records it, and an engine without the broker reads its own pool file as it does today (ADR-004). Delivers SECRETS-002 R3's broker side (all 4 criteria), read with crates/lys-secrets in place of the door (ADR-019, proposed). Blocked by, in this order, and not by the whole of DIRECTORY-006, whose R5 and R6 this row does not wait on: (a) R3 and R4 landed, in that order, since this row builds on R4's proxy and revocation states: `git log --oneline origin/main -- crates/lys-secrets/src/proxy.rs crates/lys-secrets/src/in_flight.rs` prints at least 1 commit; (b) R1 and R2 accepted by the lead (CN5): `git log --oneline origin/main -- docs/design/secrets/BASELINE.md docs/design/secrets/CONTRACT.md` prints at least 1 commit, and `git show origin/main:docs/design/secrets/briefs/SECRETS-003.json | python3 -c 'import json,sys; r={q["id"]:q.get("review",{}).get("alignment") for q in json.load(sys.stdin)["requirements"]}; print(r["R1"] in ("aligned","fixed"), r["R2"] in ("aligned","fixed"))'` prints `True True`, since a review record's alignment of `aligned` or `fixed` (accepted after fixes) means the row was accepted, and `drifted`, or no review record, means it was not; (c) DIRECTORY-002 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-002.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (d) DIRECTORY-003 landed: `git show origin/main:docs/design/directory/briefs/DIRECTORY-003.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["execution"]["status"])'` prints `landed`; (e) DIRECTORY-006 R1 (the grant contract and its time window) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/types.rs` prints at least 1 commit; (f) DIRECTORY-006 R2 (delegation admission) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/admission.rs` prints at least 1 commit; (g) DIRECTORY-006 R3 (signed grant events) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/events.rs` prints at least 1 commit; (h) DIRECTORY-006 R4 (revocation, expiry and the permission-freshness check) landed: `git log --oneline origin/main -- crates/lys-identity/src/grants/permission.rs` prints at least 1 commit; (i) the word on ADR-019 of the one who decided ADR-001, which ADR-019 amends: `git show origin/main:docs/design/decisions.json | python3 -c 'import json,sys; d={a["id"]:a for a in json.load(sys.stdin)["decisions"]}; print(d["ADR-019"]["status"], d["ADR-019"]["decided_by"] == d["ADR-001"]["decided_by"])'` prints `decided True`, since ADR-019's consequences state that, once decided, its decided_by is set to exactly ADR-001's decided_by value; if ADR-019 is not decided that way, this row's files are re-briefed as an amendment before it is dispatched. Each blocker's check runs from a clone of the lys repository after `git fetch origin`. If any check fails when the row is dispatched, the row stops and names the failed check. Log capture: Logs are captured by the tracing subscriber crates/lys-secrets/tests/support/mod.rs installs, which records every event at every level into an in-memory buffer and into one file under the test's log directory. Counted legs are listed in the acceptance: SEC3_NEXT_REFUSALS, SEC3_NEXT_RACE, SEC3_NEXT_CANARY, SEC3_NEXT_RECOVERY and SEC3_NEXT_INFLIGHT. Estimate: 5 focused implementer hours.

**Acceptance:**
- SEC3_NEXT_REFUSALS: one test per refusal leg: every account rested, a dropped handle, a handle bound to another identity, and a SpiceDB answer that is not a permit. The test asserts 4 refusals, each by its named reason, and 0 account names answered.
- SEC3_NEXT_RACE: a lease of k = 3 asks receives 8 asks held at the use step and released together: exactly 3 are answered, 5 are refused, exactly 3 audit lines are appended, and the test asserts it ran 8 cases.
- SEC3_NEXT_CANARY: a canary credential behind the handle and a canary raw handle value are planted in the store; the Debug and Display output of every type the row adds, every error it returns, every log line the tracing subscriber in crates/lys-secrets/tests/support/mod.rs captures and every audit line the row's tests exercise contain 0 occurrences of each planted value, and a byte search of every file under the test's store directory and log directory finds 0 occurrences of each; for each planted value, a first positive control writes it into one captured output and the same output search finds exactly 1, and a second positive control writes it into one file under the store directory and the same file search finds exactly 1. Every answer carries an account name and 0 occurrences of the canary credential.
- SEC3_NEXT_RECOVERY: the process is killed before and after each append boundary the contract names for an ask and its audit line, then reopened: the rotation's position equals a replay of the log, no account is answered twice for one ask, and the test asserts the number of kill points it ran equals twice the number of boundaries.
- SEC3_NEXT_INFLIGHT: an ask in flight when its handle is dropped: each outcome CONTRACT.md's cancellation rule names is its own test, and the number of these tests equals the number of outcomes the rule names.
- Every SECRETS-002 acceptance criterion this row delivers is mapped by name: for each of SECRETS-002 R3 criteria 1, 2 and 3, `rg -c 's002_r<N>_ac<M>' crates/lys-secrets/tests` with that criterion's requirement number and criterion number finds at least 1 test whose name carries it, and that test passes with crates/lys-secrets in place of the door. SECRETS-002 R3 criterion 4 is met by CONTRACT.md's open section, point (5).
- Each of this row's 5 SEC3_ leg identifiers appears in the name of at least one test under crates/lys-secrets/tests, and `cargo test --workspace --all-features` runs every one of them with 0 ignored.

**Files:**
- create: crates/lys-secrets/src/next_account.rs
- create: crates/lys-secrets/tests/next_account.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/tests/support/mod.rs

**Checklist:**
- C29 — The broker's next-account answer in crates/lys-secrets delivers SECRETS-002 R3's broker side, with every counted leg of its row passing, and names no engine file.

**Stories:**
- S17 (Implementer, Is dispatched one SECRETS-003 code row at a time) — As the implementer of a code row, I want the row to name the SECRETS-002 requirements it delivers, its counted legs, its hours and what blocks it, so that I build from one row and nothing is described twice.
- S18 (Person, Keeps credentials with the standalone identity platform) — As a person keeping credentials, I want the broker to come with Lys alone, so that I never have to install Cambium to keep a credential.

## Boundaries

- SHALL NOT edit docs/design/secrets/briefs/SECRETS-002.json or SECRETS-002.md.
- SHALL NOT dispatch any row before the lead has reviewed it, and SHALL NOT start R3 to R8 before R1 and R2 are accepted.
- SHALL NOT write, read or quote a credential, token or key value in any code, test, fixture, log or document; tests use generated values that are never real credentials.
- SHALL NOT keep the store key or the audit key in the store directory, and SHALL NOT write a raw handle to the store, a log or an audit line.
- SHALL NOT change lys-core or any published lys format; SHALL NOT write a signed lease format or any new version beside lys/delegation/v1 (ADR-020).
- SHALL NOT make any engine structural: no row depends on manifold, aion, cambium or any one engine, and an engine without the broker reads its own pool file as it does today (ADR-004).
- SHALL NOT edit any file outside the lys repository; the broker's files sit in crates/lys-secrets (ADR-019, proposed; if it is not decided, the code rows are re-briefed before dispatch).
- SHALL NOT decide any point SECRETS-002 records open, except that CONTRACT.md settles the audit line's signer and leaf schema and the sealed-record binding broker-internally, leaving them open estate-wide; each point is listed in R2.
- SHALL NOT let a credential pass through an agent: the only credential that reaches a process is the seat's own login at spawn (R6).
- SHALL NOT use OpenBao or any external secrets engine.
- SHALL NOT silence a lint, ignore a test, prefix an unused variable with an underscore, or put unwrap, expect, panic, todo, unimplemented or unreachable in library code.

## Verification

- From the lys repository root: sh scripts/design/gate.sh exits 0.
- From the lys repository root, for every code row: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps; all clean.
- git diff --exit-code over docs/design/secrets/briefs/SECRETS-002.json and docs/design/secrets/briefs/SECRETS-002.md against the commit before this brief shows no change.
- git diff --name-only over every row's commits lists no path under crates/lys-core/.
- For each code row, the test run's output shows every SEC3_ leg identifier the row names ran, with the case count each asserts.
- For each code row, `rg -c 's002_r' crates/lys-secrets/tests` finds every SECRETS-002 criterion the row maps.
- A search of every file a row touches finds no credential, token or key value.

