# lys-log-store — what was asked, what it means, and what was written

## The words, as they were typed

Conformance row 4.3 says the flight recorder's log refuses a leaf it cannot prove whole, and its witness reads only what is new. This card carries the leaf-store half of that row.

lys main at a426b9a5 already names a leaf only after its bytes are flushed, links it under a name that is never replaced, takes temporary leaf sequences from a parameter, and isolates the no-replace link under a rename injection; those four commits are the first half of the earlier words, and this card does not redo them. The survey reads the tree as it stands and names each earlier requirement that main already meets, so the brief carries only what remains.

What remains is this. A leaf store opened read only never writes and never repairs: open_read_only refuses put_leaf and pin by name with ReadOnly, and opening an interrupted append read only refuses by name with RepairPending instead of repairing it. A leaf changed by one byte after it was named is refused by name on open, checked against the tree the store already keeps, so a corrupted leaf is never served as whole. The module doc of the leaf store says what open does and never does, and says that open never deletes a leftover temporary file. The witness half of the row, reading only what is new, is carried by LYSLOGSTORE-002 on its own card and is not this card's.

Acceptance: put_leaf and pin through a read-only handle each refuse with ReadOnly and change no byte of the store; a store with an interrupted append opened read only refuses with RepairPending and the store's bytes are unchanged; the same store opened for writing repairs as today; a leaf whose bytes are changed by one byte after naming is refused by name on open; and every test named in the brief runs by a command a stranger can run against lys main, with its expected output stated. The brief names row 4.3 as the row it passes in part, and names LYSLOGSTORE-002 as the other part.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh), and commit the cluster's rendered markdown with the brief so the gate exits 0 on the branch. The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing; the path docs/design/lys-log-store/briefs/LYSLOGSTORE-001.json is held by two open drafts, a1163a79 and e8440f97, on other subjects, so this brief takes the next free number and does not reuse LYSLOGSTORE-001. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 28 September 2026, against lys main a426b9a5.

## What the survey found, and its angles

The card asks for the leaf-store half of a log-integrity row. First, a FileLeafStore opened read-only never writes and never repairs: put_leaf and pin refuse with a named ReadOnly error, and a read-only open of a store left one leaf ahead of its pin refuses with a named RepairPending error instead of repairing it. Second, a leaf changed after it was named is refused on open. Third, the leaf store's module doc says what open does and never does, including that it never deletes a leftover temporary file. The four landed commits (flush-then-name, no-replace link, sequence parameter, rename-injection isolation) are not to be redone. The witness half belongs to LYSLOGSTORE-002. On main the cluster has no design.json, so the brief also brings the cluster's method documents and their rendered markdown.

### What the tree holds

- `crates/lys-log-store/src/file.rs` — FileLeafStore. open (lines ~180-215) fsyncs leaves/ and then counts names without reading leaf bytes. put_leaf_with and pin are where a read-only refusal goes. There is no open_read_only and no read-only flag. The module doc (lines 1-54) describes the temporary name and the no-replace link, but it does not say that open never deletes a leftover. It has 430 non-blank, non-// lines against the 500-line limit.
- `crates/lys-log-store/src/error.rs` — StoreError is #[non_exhaustive] with 16 variants. ReadOnly and RepairPending do not exist, so adding them is not a semver break for the published lys-log-store 0.2.0.
- `crates/lys-log-store/src/log.rs` — Log::open → reconcile_with_pin is the only place a repair happens: storage exactly one leaf ahead, prefix matching the pin, then store.pin. A leaf changed inside the pinned prefix is already refused here as PinMismatch, without naming the leaf. The one-ahead leaf is adopted whatever its bytes. A read-only store under Log::open would reach store.pin and fail, unless RepairPending is raised first.
- `crates/lys-log-store/src/store.rs` — The LeafStore trait contract, items 1-5. Whether read-only is an inherent FileLeafStore constructor or a trait matter decides whether BACKENDS.md and the trait change.
- `crates/lys-log-store/src/file_tests.rs` — 34 tests. It already holds a_leftover_temporary_file_is_ignored_at_open and open_leaves_a_leftover_temporary_file_byte_identical (the leftover-untouched behaviour exists). It has no read-only test.
- `crates/lys-log-store/src/log_tests.rs` — 13 tests. It already holds a_tampered_leaf_byte_is_detected_at_open (leaf-0→leaf-X, one byte, PinMismatch), crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it (the 'repairs as today' case) and crash_recovery_does_not_mask_a_tampered_prefix.
- `crates/lys-anchor/src/anchor/read_only.rs` — Anchor::open_read_only calls Log::open (line 171) over FileLeafStore::open, so the existing 'read-only' anchor repairs and pins. That is outside this card's leaf-store scope, but it is the obvious consumer.
- `crates/lys/src/commands/log/store.rs` — The lys log commands open with Log::open(FileLeafStore::open(dir)) and print the repair. They are a second read path that still repairs.
- `docs/design/lys-log-store/` — On main it holds only BACKENDS.md: no design.json, stories.json, checklist.json or briefs/. gate.sh measures only clusters with a design.json, so this brief must bring the cluster's JSON and rendered markdown. Two open branches (0e2bb567 and 6826e0cf) each create their own design.json here.
- `scripts/design/gate.sh` — Validates decisions.json and project.json. For every cluster with a design.json it runs validate.py and check-coverage.py, then compares the render-cluster.py output byte for byte with the committed .md files.
- `docs/design/identity/CONFORMANCE.md` — The only numbered conformance table in the tree. Its row 4.3 is about moving a role holder to a newer version, not the flight recorder.

### What was already decided

- docs/design/lys-log-store/BACKENDS.md — Five criteria a second backend must meet: durable on return, write-once, contiguity at open, read-back, and no fork/merge/delete. Read-only opening is not among them.
- LYSLOGSTORE-001 (brief/lys-log-store/0e2bb567, unlanded) — The earlier words. R1-R4 and most of R5/R7/R8 are now on main. R5's open_read_only/ReadOnly, R6's module-doc section, R9 (DIRECTORY-003 depends_on), R10-R12 (Anchor opens at the pinned head and reports the repair as pending, rather than refusing) and R13 (cluster render) are not.
- LYSLOGSTORE-002 (brief/lys-log-store/6826e0cf, unlanded) — It carries more than the witness: pinned-prefix PinMismatch tests (R1), leftover-temporary reporting (R2-R4) and the witness projection. It also says 'SHALL NOT add a read-only mode to Log or to Anchor' and 'any read-only open' is out of scope.
- ADR-101 (on branch 6826e0cf only, not in main's ledger) — The pin is the store's only expectation of its leaves. A damaged pinned prefix is refused as PinMismatch without naming a leaf. A per-leaf hash record, bisection and a read-only open of a refused log are all rejected.
- ADR-100 (on branch 6826e0cf only) — The witness folds only new leaves through an in-memory projection. That is the witness half the words hand to LYSLOGSTORE-002.
- crates/lys-log-store/src/log.rs module doc — Append stores the leaf before the pin, so 'one leaf ahead' is the only divergence a crash produces and the only one Log::open repairs.
- CLAUDE.md coding standards — No file over 500 code lines, tests in sibling *_tests.rs, every public item documented, and gates in both feature shapes, including cargo doc.
- docs/design/identity/CONFORMANCE.md row 4.3 — 'Moving a holder to a newer version is a deliberate act…' (roles). It does not match the row 4.3 the words cite.

### What was measured

- Code lines in crates/lys-log-store/src/file.rs (non-blank, not starting //): 430 of a 500 limit (658 physical lines)
- Physical lines per lys-log-store source file: file.rs 658, file_tests.rs 783, error.rs 265, log.rs 234, log_tests.rs 345, store.rs 191, lib.rs 60
- Tests in file_tests.rs / log_tests.rs: 34 / 13 #[test] functions
- StoreError variants on main: 16, #[non_exhaustive]; ReadOnly and RepairPending: 0 occurrences in crates/
- open_read_only on FileLeafStore: absent (the only open_read_only is Anchor::open_read_only in lys-anchor)
- Non-test Log::open call sites: 6 (lys-anchor open.rs ×2, genesis.rs ×2, read_only.rs ×1; lys commands/log/store.rs ×1)
- Non-test FileLeafStore::open call sites: 2 (lys commands/log/store.rs:46, lys-anchor-cli commands/anchor/open.rs:117) plus 2 doc examples in read_only.rs
- Files in docs/design/lys-log-store on main: 1 (BACKENDS.md); design.json absent
- Branches holding docs/design/lys-log-store/briefs/LYSLOGSTORE-001.json: 5 refs across 4 cards: brief+draft 0e2bb567 (leaf naming, the earlier words), draft 9aee27d1 (leaf naming), draft a1163a79 (Log memory), draft e8440f97 (Log memory)
- Branches holding LYSLOGSTORE-002.json: 2 refs (brief+draft 6826e0cf)
- Highest ids across all 336 remote refs at survey time: RM-063, ADR-103, LYSLOGSTORE-002; next free: RM-064, ADR-104, LYSLOGSTORE-003
- Ids on main's ledgers: 11 roadmap rows (last RM-016), 18 decisions (last ADR-018)
- Earlier LYSLOGSTORE-001 requirements already met on main a426b9a: R1 (LeafDurabilityUncertain), R2 (temp write+flush, leaf_temp_name, create_new), R3 (no-replace hard_link), R4 (link is commit point, ReopenRequired), R5 writable-open part (fsync_dir before contiguous_extent; leftovers untouched), R7 (fault seam as AfterLink + next_sequence parameter); not met: R5 read-only part, R6 module-doc section, R9, R10-R12, R13
- Existing one-byte-change test: 1: log_tests.rs a_tampered_leaf_byte_is_detected_at_open asserts PinMismatch, names no leaf

### What it means for the other projects

- haematite — BACKENDS.md names haematite as the blocked second backend. If read-only opening becomes part of what a backend must support (a trait change rather than a FileLeafStore constructor), BACKENDS.md's criteria and haematite's adoption check both grow a row. If it stays inherent to FileLeafStore, haematite is untouched.
- aion — The card runs through aion's chain (brief_card → sign-off → card_build_v3 → src_pr → src_land). The id collision on LYSLOGSTORE-001 across five refs, and two open branches each creating docs/design/lys-log-store/design.json, are merge conflicts the chain's landing step will hit.
- method — The design-system scripts are copied at commit 3c3bac7 (scripts/design/SOURCE.md). The brief must validate against those copied schemas. Nothing in the method changes.
- cambium — The card lives on the Lys board on Cambium. If the lead cuts LYSLOGSTORE-002 back or closes 0e2bb567, those cards' rows change there.

### The decisions it stands on

- ADR-004 (honour) — lys-log-store stands alone. A read-only open adds nothing that needs another project, and the trait stays free of any engine.
-  (new) — A FileLeafStore opened read-only refuses to write (ReadOnly) and refuses a one-leaf-ahead store (RepairPending) instead of repairing it. This departs from the earlier unlanded brief's choice to open at the pinned head, and from ADR-101 on branch 6826e0cf (not in main's ledger), which rejects any read-only open. It takes the next free ADR id (ADR-104 at survey time).
-  (new) — If the lead chooses to name a changed leaf by index, a per-leaf expectation record becomes a new local-state layout decision. It would contradict ADR-101 on 6826e0cf and would change BACKENDS.md. If the lead chooses PinMismatch as the refusal, no decision is needed.

### What it requires

- FileLeafStore has a public, documented open_read_only(dir) that performs open's checks (log.json, state.json, contiguity) and returns a handle.
- put_leaf on a read-only handle returns StoreError::ReadOnly and every file under the store directory is byte-identical before and after.
- pin on a read-only handle returns StoreError::ReadOnly and state.json is byte-identical before and after.
- open_read_only (or Log::open over it) on a store with extent == pinned.tree_size + 1 returns StoreError::RepairPending, and every file under the store directory is byte-identical before and after.
- FileLeafStore::open followed by Log::open on that same store repairs it, reports recovered_to() == Some(n), and a second open reports None (the existing crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it still passes unchanged).
- A store whose named leaf has one byte changed after naming is refused on open with a named StoreError variant, as the lead settles it (PinMismatch, or a new variant naming the leaf).
- The leaf store's module doc states what open does (checks, counting, the leaves/ flush on a writable open) and never does (repair on a read-only open, count a dot-prefixed temporary name, delete or change a leftover temporary file).
- StoreError gains exactly the new variants the brief names (ReadOnly, RepairPending), each documented, with messages naming the directory and the refused act.
- Every test the brief names is listed with a command such as `cargo test -p lys-log-store --all-features <test> -- --exact` and its expected output.
- crates/lys-log-store/src/file.rs stays at or under 500 code lines.
- docs/design/lys-log-store gains design.json, stories.json, checklist.json, briefs/LYSLOGSTORE-<next>.json and their rendered .md files, and `sh scripts/design/gate.sh` exits 0 on the branch.
- The brief names row 4.3 as passed in part and LYSLOGSTORE-002 as the other part.
- The brief lists which earlier LYSLOGSTORE-001 requirements main a426b9a already meets (R1-R4, R5's writable open, R7) and carries only what remains.
- All six gate legs pass in both feature shapes (fmt, clippy ×2, tests --all-features, doc ×2), run on Dean's laptop.

### What must not change

- Do not change the four landed commits' behaviour: the temporary write and flush, the no-replace hard_link, the next_sequence parameter, and the AfterLink injection seams.
- Do not change the one-leaf repair on a writable Log::open.
- Do not change the leaf file layout, the leaf name width, log.json, state.json's shape, or the rule that a leaf file is the RFC 6962 preimage, unless the lead chooses a per-leaf record.
- Do not add fork, merge, delete, truncate or leaf rewrite to LeafStore.
- Do not delete, rename or modify a leftover temporary file on any open.
- Do not touch the witness (lys-anchor witness module), which is LYSLOGSTORE-002's.
- Do not reuse LYSLOGSTORE-001, or any id already on main or on an open brief, draft or hand branch.
- Add no #[allow], #[ignore] or _-prefixed bypass. Library code has no unwrap, expect or panic.

### What we must put in place first

- The lead's answers on row 4.3's source, on the meaning of 'refused by name' for a changed leaf, and on LYSLOGSTORE-002's overlap, before the brief's requirements can be written without contradicting the tree.
- Re-run git ls-remote immediately before writing to confirm LYSLOGSTORE-003, RM-064 and ADR-104 are still free (at survey time the highest ids were LYSLOGSTORE-002, RM-063 and ADR-103).
- Dean's laptop available for the full gate runs (standing rule 3/7).

### The risks

- file.rs has 70 code lines of headroom. open_read_only, a mode flag, two guards, a RepairPending check and a doc section could push it past 500 and force a split.
- If RepairPending is checked only in Log::open, a read-only store reaching reconcile_with_pin fails at pin with ReadOnly instead, and the acceptance names the wrong error.
- Two open branches (0e2bb567, 6826e0cf) each create docs/design/lys-log-store/design.json, stories.json and checklist.json. Whichever lands second conflicts, and the gate's render comparison fails until it is regenerated.
- ADR-101 on 6826e0cf rejects a read-only open. If both land, the ledger holds contradictory decisions.
- Anchor::open_read_only and `lys log status` still repair after this card, so a person could believe 'read-only' is now honoured end to end when it is not.
- The existing one-byte test already passes. A new test that asserts only PinMismatch agrees with main as it stands and proves nothing new. The drift injection must fail exactly that test.
- A read-only open that still fsyncs leaves/ may fail on a read-only mount, depending on the platform.

### Still open

- Where is 'conformance row 4.3' for the flight recorder written, given that the tree's only numbered conformance table puts a roles row at 4.3, and which document should the brief cite for it? The sentence of the words it stands on: "Conformance row 4.3 says the flight recorder's log refuses a leaf it cannot prove whole, and its witness reads only what is new.". Why only the lead can settle it: docs/design/identity/CONFORMANCE.md:47 row 4.3 is 'Moving a holder to a newer version is a deliberate act…'. No document in the tree numbers a flight-recorder row 4.3, so the brief would cite a row that says something else.
- Should a changed leaf be refused as today's PinMismatch from Log::open, which names no leaf because the store keeps only one tree size and one root? Or should the store start keeping a per-leaf hash record so the refusal can name the leaf and FileLeafStore::open itself can check bytes? The sentence of the words it stands on: "A leaf changed by one byte after it was named is refused by name on open, checked against the tree the store already keeps, so a corrupted leaf is never served as whole.". Why only the lead can settle it: FileLeafStore keeps only state.json's (tree_size, root_hash) (file.rs LogState). FileLeafStore::open never reads leaf bytes (file.rs open doc). A one-byte change inside the pinned prefix is already refused as PinMismatch (log_tests.rs a_tampered_leaf_byte_is_detected_at_open), but it names no leaf. The one leaf just past the pin cannot be checked against anything. Also, FileLeafStore::leaf serves bytes unchecked to any caller that skips Log::open. Naming the leaf needs a new layout record, which BACKENDS.md and ADR-101 on 6826e0cf reject.
- Does 'refused by name on open' in the acceptance mean returning a named error variant (PinMismatch already does this), or naming the index of the changed leaf? The sentence of the words it stands on: "Acceptance: put_leaf and pin through a read-only handle each refuse with ReadOnly and change no byte of the store; a store with an interrupted append opened read only refuses with RepairPending and the store's bytes are unchanged; the same store opened for writing repairs as today; a leaf whose bytes are changed by one byte after naming is refused by name on open; and every test named in the brief runs by a command a stranger can run against lys main, with its expected output stated.". Why only the lead can settle it: Under the first reading the one-byte case is already met on main by an existing test. Under the second it needs a per-leaf record, which changes what an operator is told and the store layout.
- LYSLOGSTORE-002 on brief/lys-log-store/6826e0cf also carries the pinned-prefix refusal tests, leftover-temporary reporting and 'SHALL NOT add a read-only mode to Log or to Anchor', and its ADR-101 rejects a read-only open. Is 002 to be cut back to the witness only, or does this card supersede those lines? The sentence of the words it stands on: "The witness half of the row, reading only what is new, is carried by LYSLOGSTORE-002 on its own card and is not this card's.". Why only the lead can settle it: docs/design/lys-log-store/briefs/LYSLOGSTORE-002.json on 6826e0cf covers more than the witness and forbids the read-only open this card adds. Landing both as written puts two contradictory briefs and two design.json files in one cluster.
- The earlier brief's unlanded items are R9 (DIRECTORY-003 waits on it), R10-R12 (Anchor::open_read_only and a log opened at its pin, reporting the repair as pending) and R13. Are these dropped, carried by this card, or named as further roadmap units? And is brief/lys-log-store/0e2bb567 (which also holds LYSLOGSTORE-001) closed? The sentence of the words it stands on: "The survey reads the tree as it stands and names each earlier requirement that main already meets, so the brief carries only what remains.". Why only the lead can settle it: The words scope this card to the leaf store. But Anchor::open_read_only (lys-anchor read_only.rs:171) and `lys log` (commands/log/store.rs:46) would still repair on a read path, so a person running `status` still gets a write. The earlier brief also opened at the pinned head where these words refuse.
- Is LYSLOGSTORE-001.json held only by the two drafts named, or also by brief/draft 0e2bb567 and draft 9aee27d1 on this same subject, which the words call the earlier words? The sentence of the words it stands on: "The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing; the path docs/design/lys-log-store/briefs/LYSLOGSTORE-001.json is held by two open drafts, a1163a79 and e8440f97, on other subjects, so this brief takes the next free number and does not reuse LYSLOGSTORE-001.". Why only the lead can settle it: git ls-remote shows LYSLOGSTORE-001.json on five refs across four cards: 0e2bb567 (brief and draft) and 9aee27d1 (draft) on the leaf-naming subject, plus a1163a79 and e8440f97 on Log memory. The 'other subjects' count is incomplete, and that decides which cards the lead closes.

### The units beyond the first

- Open Anchor::open_read_only and `lys log status` over FileLeafStore::open_read_only — The earlier brief's R10-R12. The consumers' read paths still repair after the leaf store learns to refuse. It touches lys-anchor, lys-anchor-cli and lys, which the words keep off this card.
- Record that DIRECTORY-003 waits on the leaf-store brief — The earlier brief's R9. It is a change to the directory cluster, a separate document set with its own gate, and is not part of the leaf store.
- Name the changed leaf through a per-leaf expectation record — Only if the lead wants the leaf index named. It is a layout change that needs a decision, a BACKENDS.md criterion, crash-window analysis and an adversarial review, which is too large and too different to ride with the read-only refusal.
- Witness reads only what is new (LYSLOGSTORE-002) — The other half of row 4.3, assigned by the words to its own card.

### The smallest complete shape

One PR on lys main. In crates/lys-log-store: add StoreError::ReadOnly and StoreError::RepairPending (error.rs). Add FileLeafStore::open_read_only with a read-only mode, in which put_leaf and pin refuse with ReadOnly, and the open refuses a store exactly one leaf ahead of its pin with RepairPending (file.rs). Add a module-doc section on what open does and never does, including never deleting a leftover. Add tests in file_tests.rs/log_tests.rs for each acceptance line (ReadOnly ×2 with byte-identical store, RepairPending with byte-identical store, writable repair unchanged, one-byte change refused as the lead settles it), each with its stranger-runnable command and expected output. In the same PR, add the lys-log-store cluster's design.json, stories.json, checklist.json and brief LYSLOGSTORE-003, their rendered markdown, one roadmap row (RM-064) and, if recorded, ADR-104, so scripts/design/gate.sh exits 0.

## The roadmap row

- **RM-066** — Open a leaf store read-only that never writes and never repairs (feature, idea)
- Summary: The leaf-store half of the flight recorder's log-integrity row: FileLeafStore::open_read_only refuses put_leaf and pin with StoreError::ReadOnly and refuses a store one leaf past its pin with StoreError::RepairPending, changing no byte; a writable open still repairs as today; FileLeafStore::leftover_temporaries names the temporary files an open skipped; Log::open's PinMismatch refusal of a torn pinned prefix is held under tests; and file.rs's module doc says what open does and never does, including that it never deletes a leftover temporary file and that a leaf is proven whole only through Log::open. The one-byte case is already met on main. The witness half is LYSLOGSTORE-002's.
- Asked by: tom on 2026-09-27T16:42:00+10:00
- Context: The leaf-store card on the Lys board, filed by the identity line's lead against lys main a426b9a5 and surveyed against that tree; the lead's answers to the survey's six questions are written into LYSLOGSTORE-003 and ADR-106.
- Quote: Conformance row 4.3 says the flight recorder's log refuses a leaf it cannot prove whole, and its witness reads only what is new. This card carries the leaf-store half of that row.

lys main at a426b9a5 already names a leaf only after its bytes are flushed, links it under a name that is never replaced, takes temporary leaf sequences from a parameter, and isolates the no-replace link under a rename injection; those four commits are the first half of the earlier words, and this card does not redo them. The survey reads the tree as it stands and names each earlier requirement that main already meets, so the brief carries only what remains.

What remains is this. A leaf store opened read only never writes and never repairs: open_read_only refuses put_leaf and pin by name with ReadOnly, and opening an interrupted append read only refuses by name with RepairPending instead of repairing it. A leaf changed by one byte after it was named is refused by name on open, checked against the tree the store already keeps, so a corrupted leaf is never served as whole. The module doc of the leaf store says what open does and never does, and says that open never deletes a leftover temporary file. The witness half of the row, reading only what is new, is carried by LYSLOGSTORE-002 on its own card and is not this card's.

Acceptance: put_leaf and pin through a read-only handle each refuse with ReadOnly and change no byte of the store; a store with an interrupted append opened read only refuses with RepairPending and the store's bytes are unchanged; the same store opened for writing repairs as today; a leaf whose bytes are changed by one byte after naming is refused by name on open; and every test named in the brief runs by a command a stranger can run against lys main, with its expected output stated. The brief names row 4.3 as the row it passes in part, and names LYSLOGSTORE-002 as the other part.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh), and commit the cluster's rendered markdown with the brief so the gate exits 0 on the branch. The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing; the path docs/design/lys-log-store/briefs/LYSLOGSTORE-001.json is held by two open drafts, a1163a79 and e8440f97, on other subjects, so this brief takes the next free number and does not reuse LYSLOGSTORE-001. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 28 September 2026, against lys main a426b9a5.
- Cluster: lys-log-store; briefs: LYSLOGSTORE-003
- Notes: Further units, by title, not written here: (1) Move Anchor::open_read_only and the lys log commands onto FileLeafStore::open_read_only, opening a log at its pin and reporting the repair as pending (the earlier LYSLOGSTORE-001 R10 to R12, with its R13 cluster render). Until it lands, crates/lys-anchor/src/anchor/read_only.rs and crates/lys/src/commands/log/store.rs still repair on a read path, so a person running status still gets a write, and the integrity row is not passed in full. (2) Record that DIRECTORY-003 waits on the leaf-store brief (the earlier LYSLOGSTORE-001 R9; DIRECTORY-003 is the brief that waits, on LYSLOGSTORE-003). (3) Print leftover temporary leaf files from the lys log and lys-anchor commands, and hold their refusal of a damaged log (LYSLOGSTORE-002's former R3 and R4, which leave that brief when it is cut back to the witness). (4) Witness reads only what is new (LYSLOGSTORE-002). Finding: docs/design/identity/CONFORMANCE.md carries no flight-recorder row (its row 4.3 is a roles row); the lead adds one, and LYSLOGSTORE-003 stands on the lys-log-store design's goal meanwhile. Superseded by LYSLOGSTORE-003 and closed at sign-off: brief/lys-log-store/0e2bb567 and draft/lys-log-store/9aee27d1. LYSLOGSTORE-001 is held on five refs across four cards (0e2bb567 brief and draft, 9aee27d1, and a1163a79 and e8440f97 on Log memory, which stay open). Ids read with git ls-remote over all refs immediately before writing, and read again each time a draft branch took the next number between reads (draft/directory/3e63b2d8 took RM-064 and ADR-104, then draft/directory/09884e04 took RM-065 and ADR-105): over 339 refs the highest are RM-065, ADR-105 and LYSLOGSTORE-002, so this row is RM-066, the decision ADR-106 and the brief LYSLOGSTORE-003.

## The design

---
type: design
cluster: lys-log-store
title: lys-log-store: a leaf store that never serves a leaf as whole unless it is proven whole
---

# lys-log-store: a leaf store that never serves a leaf as whole unless it is proven whole

> **Cluster:** lys-log-store

## Intention

A flight recorder is only evidence if reading it cannot change it and nothing it serves as whole is anything less. The leaf store is where that starts: it holds the leaves of a lys log on disk, and every other part of the log trusts what it says it holds.

When this cluster's first brief is done, a reader can open a leaf store and be certain that the open wrote nothing, repaired nothing and deleted nothing, and a store that a crash left mid-append tells that reader so by name instead of being quietly repaired under them. A leaf is proven whole in exactly one place, a Log opened against the pinned root, and the module doc says so where the next implementor will read it.

## Problem

FileLeafStore has one way to open, and it is the writer's. A caller that only wants to read a log gets a handle that will write a leaf or move the pin if asked, and a store left one leaf ahead of its pin by a crash is repaired by the first Log::open over it, whoever that caller is. So a read path, such as a status command or Anchor::open_read_only, performs a write. Nothing in the store names that state to a reader, and nothing tells a reader which leftover temporary files the open skipped.

The store's module doc says how a leaf is written but not what open does and never does: that open never deletes a leftover temporary file, and that FileLeafStore::leaf returns bytes it has not checked, so a leaf is proven whole only through Log::open against the pinned root. A reader who calls leaf() directly can be served a changed leaf without knowing it.

The refusal of a damaged pinned prefix exists on main as StoreError::PinMismatch from Log::open, but only a one-byte change is held under a test; a torn leaf inside the prefix, and a torn leaf just past the pin, are not.

## Solution

The store gains a second constructor for readers, FileLeafStore::open_read_only, beside the writer's FileLeafStore::open. It is an inherent constructor on FileLeafStore: the LeafStore trait, BACKENDS.md and Log are unchanged, and Log and Anchor gain no read-only mode (ADR-106). The read-only open performs the same checks as the writable open (log.json, state.json, the 20-digit leaf names and their contiguity) and differs in three ways: it does not flush leaves/, it refuses by name with StoreError::RepairPending a store whose extent is exactly one past its pinned tree size, and the handle it returns refuses put_leaf and pin with StoreError::ReadOnly before any other check. RepairPending is decided from the extent and the pin alone, without reading leaf bytes, so it fires before any Log could reach the repair; whether the pending leaf is a clean interrupted append or a damaged prefix is decided by a writable open, which repairs or refuses with PinMismatch exactly as it does today. A read-only handle over a store that is two or more leaves past its pin, or behind it, opens, and Log::open over it refuses with PinMismatch without writing.

The walk of leaves/ that counts the extent also collects the store's own temporary-leaf names, which it already skips, and returns them in lexical order from FileLeafStore::leftover_temporaries, so a caller can name what the open ignored. The walk never removes, renames or changes one.

The refusal of a changed leaf stays where it is: Log::open rebuilds the tree from every stored leaf and compares it with the one tree size and one root state.json keeps, and a leaf changed inside the pinned prefix is refused as StoreError::PinMismatch, which carries both trees and never a leaf index. The store keeps no per-leaf record, so which leaf changed is not known and is not claimed. New tests hold that refusal for a torn leaf and hold the one-leaf repair for a torn leaf just past the pin.

file.rs's module doc gains a section saying what open and open_read_only do and never do, including that neither ever deletes a leftover temporary file, and that leaf() serves bytes it has not checked: every reader that wants a proven leaf goes through Log::open.

The witness half of the same integrity row, a witness that reads only what is new, is LYSLOGSTORE-002's and is not designed here. The consumers' read paths (Anchor::open_read_only and the lys log commands) still open with the writable open after this cluster's first brief; moving them onto the read-only open is a further unit.

## Principles

- **P1** — A read-only open writes no byte: no leaf, no pin, no flush, no removal of a leftover.
- **P2** — A refusal is a named StoreError variant, never a repaired or partial answer.
- **P3** — A leaf is proven whole only by Log::open against the pinned root; the store serves bytes, it does not vouch for them.
- **P4** — The store claims only what it knows: it keeps one tree size and one root, so no refusal names a leaf index.
- **P5** — The LeafStore trait stays what BACKENDS.md says a backend must satisfy; reading without writing is FileLeafStore's own.

## Decisions

- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-106 — A FileLeafStore opened read-only refuses to write and to repair; Log and Anchor gain no read-only mode — FileLeafStore gains an inherent open_read_only whose handle refuses put_leaf and pin with StoreError::ReadOnly and which refuses a store exactly one leaf past its pin with StoreError::RepairPending, decided from the count and the pin without reading leaf bytes and without flushing. Log and Anchor gain no read-only mode: Log::open over a read-only handle either finds the tree matching the pin, or refuses with PinMismatch, and never reaches a pin, so the Log needs no mode for the store to hold. Both hold because the refusal of a repair lives at the one place that can know a repair would be needed before any Log sees the store. Rejected: opening a read-only log at its pinned head and reporting the repair as pending (a Log mode, and a served state the store has not reconciled); a read-only mode on Log or Anchor; making read-only opening a LeafStore trait method, which would add a criterion to BACKENDS.md; and a per-leaf hash record so a refusal could name the changed leaf.

## Goals

- No leaf is served as whole unless it is proven whole against the pinned root through Log::open, and file.rs's module doc says so.
- FileLeafStore::open_read_only refuses put_leaf and pin with StoreError::ReadOnly, and every file under the store directory holds the same bytes before and after.
- FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending, every file under the store directory holds the same bytes before and after, and a writable open of the same store repairs it as main does.
- A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch and state.json is unchanged, under a named test.
- FileLeafStore::leftover_temporaries names the temporary-leaf files an open skipped, and no open removes or changes one.

## Non-Goals

- A per-leaf hash record, or a refusal that names the index of a changed leaf. — The store keeps one tree size and one root (BACKENDS.md); naming the leaf needs a new layout record, which this cluster does not add.
- A read-only mode on Log or on Anchor. — ADR-106: the read-only open is FileLeafStore's alone.
- Moving Anchor::open_read_only and the lys log commands onto FileLeafStore::open_read_only, or printing leftover temporary files from the lys log and lys-anchor commands. — Those are the consumers' read paths, in lys-anchor, lys-anchor-cli and lys, and are a further unit.
- The witness reading only what is new. — LYSLOGSTORE-002 carries the witness half of the integrity row.
- Redoing flush-before-name, the no-replace link, the temporary-sequence parameter or the rename-injection isolation. — All four are on main already.
- Deleting or tidying leftover temporary files. — Open only skips and names them; clearing them is not the store's act.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `crates/lys-log-store/src/error.rs` | StoreError; gains ReadOnly and RepairPending |  |
| `crates/lys-log-store/src/file.rs` | FileLeafStore; gains open_read_only, the read-only refusals, leftover_temporaries and the 'What open does and never does' module-doc section |  |
| `crates/lys-log-store/src/file_tests.rs` | FileLeafStore tests; gains the read-only and leftover-temporary tests |  |
| `crates/lys-log-store/src/log.rs` | Log over any LeafStore; its open and one-leaf repair are unchanged |  |
| `crates/lys-log-store/src/log_tests.rs` | Log tests; gains the pinned-prefix refusal and repair tests |  |
| `crates/lys-log-store/src/store.rs` | The LeafStore trait contract; unchanged |  |
| `crates/lys-log-store/src/lib.rs` | Crate root and re-exports; unchanged |  |
| `docs/design/lys-log-store/BACKENDS.md` | Criteria a second LeafStore backend must meet; unchanged |  |
| `docs/design/lys-log-store/design.json` | This design | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/checklist.json` | The cluster checklist | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/stories.json` | The cluster stories | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-003.json` | The read-only leaf store brief | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/DESIGN.md` | Rendered from design.json | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/CHECKLIST.md` | Rendered from checklist.json | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/USER-STORIES.md` | Rendered from stories.json | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-003.md` | Rendered from LYSLOGSTORE-003.json | LYSLOGSTORE-003 |

## Inventory

- `crates/lys-log-store/src/file.rs` — FileLeafStore with create, open (flushes leaves/, then counts 20-digit names, skipping dot-prefixed ones), leaf, put_leaf (temporary write, flush, no-replace hard link) and pin. No read-only open. 430 code lines of the 500 allowed.
- `crates/lys-log-store/src/error.rs` — StoreError, #[non_exhaustive], 16 variants including PinMismatch; no ReadOnly or RepairPending.
- `crates/lys-log-store/src/log.rs` — Log::open rebuilds from every leaf and reconciles with the pin; repairs exactly one leaf past the pin when the prefix matches, otherwise PinMismatch.
- `crates/lys-log-store/src/store.rs` — The LeafStore trait and PinnedRoot; no fork, merge, delete, truncate or rewrite.
- `crates/lys-log-store/src/file_tests.rs` — 34 tests, including a_leftover_temporary_file_is_ignored_at_open and open_leaves_a_leftover_temporary_file_byte_identical; no read-only test.
- `crates/lys-log-store/src/log_tests.rs` — 13 tests, including a_tampered_leaf_byte_is_detected_at_open (one byte changed in the pinned prefix, PinMismatch), crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it and crash_recovery_does_not_mask_a_tampered_prefix.
- `crates/lys-anchor/src/anchor/read_only.rs` — Anchor::open_read_only opens its log with Log::open over the writable FileLeafStore::open, so it repairs and pins on a read path.
- `crates/lys/src/commands/log/store.rs` — The lys log commands open with Log::open(FileLeafStore::open(dir)), so status repairs on a read path.
- `docs/design/lys-log-store/BACKENDS.md` — Five criteria a second backend must meet; reading without writing is not among them.
- `docs/design/identity/CONFORMANCE.md` — The only numbered conformance table; it carries no flight-recorder row, and its row 4.3 is a roles row.
- `scripts/design/gate.sh` — Validates every cluster with a design.json, checks its coverage and compares its rendered markdown byte for byte.

## Constraints

- **CN1** — The leaf write on main is unchanged: the temporary write and flush, the no-replace hard link, the next_sequence parameter and the AfterLink seams.
- **CN2** — A writable open followed by Log::open repairs a store exactly one leaf past its pin as main does, and reports it through recovered_to.
- **CN3** — The leaves/ layout, the 20-digit leaf name, log.json, state.json's shape and the rule that a leaf file is the RFC 6962 preimage are unchanged.
- **CN4** — The LeafStore trait gains no method, and gains no fork, merge, delete, truncate or leaf rewrite; BACKENDS.md is unchanged.
- **CN5** — No open deletes, renames or modifies a leftover temporary file.
- **CN6** — crates/lys-log-store/src/file.rs holds no more than 500 lines that are neither blank nor comment lines, not counting tests.
- **CN7** — Library code has no unwrap, expect, panic, todo, unimplemented or unreachable, and no #[allow], #[ignore] or _-prefixed bypass is added.


---
type: brief
id: LYSLOGSTORE-003
cluster: lys-log-store
title: Open a leaf store read-only that never writes and never repairs, name its leftovers, and say what open does and never does
---

# LYSLOGSTORE-003: Open a leaf store read-only that never writes and never repairs, name its leftovers, and say what open does and never does

> **Cluster:** lys-log-store
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-106 — A FileLeafStore opened read-only refuses to write and to repair; Log and Anchor gain no read-only mode — FileLeafStore gains an inherent open_read_only whose handle refuses put_leaf and pin with StoreError::ReadOnly and which refuses a store exactly one leaf past its pin with StoreError::RepairPending, decided from the count and the pin without reading leaf bytes and without flushing. Log and Anchor gain no read-only mode: Log::open over a read-only handle either finds the tree matching the pin, or refuses with PinMismatch, and never reaches a pin, so the Log needs no mode for the store to hold. Both hold because the refusal of a repair lives at the one place that can know a repair would be needed before any Log sees the store. Rejected: opening a read-only log at its pinned head and reporting the repair as pending (a Log mode, and a served state the store has not reconciled); a read-only mode on Log or Anchor; making read-only opening a LeafStore trait method, which would add a criterion to BACKENDS.md; and a per-leaf hash record so a refusal could name the changed leaf.
> **Checklist:**
> - C1 — StoreError has a documented ReadOnly variant carrying the store's directory and the refused act.
> - C2 — StoreError has a documented RepairPending variant carrying the store's directory, the pinned tree size and the extent.
> - C3 — FileLeafStore::open_read_only performs FileLeafStore::open's checks and returns a handle without flushing leaves/ or writing any file.
> - C4 — FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending and every file under the store directory holds the same bytes before and after.
> - C5 — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
> - C6 — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
> - C7 — A store one leaf past its pin, the state a read-only open refuses with RepairPending, is repaired by FileLeafStore::open followed by Log::open, which reports recovered_to as Some of the extent.
> - C8 — A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch carrying both trees, and state.json is unchanged.
> - C9 — A torn leaf just past the pin is adopted by the one-leaf repair and reported by recovered_to.
> - C10 — FileLeafStore::leftover_temporaries returns the store's temporary-leaf names in lexical order, none counted toward the extent and none changed.
> - C11 — file.rs's module doc has a section stating what open and open_read_only do and never do, including that neither deletes a leftover temporary file.
> - C12 — file.rs's module doc states that FileLeafStore::leaf serves bytes it has not checked and that a leaf is proven whole only through Log::open against the pinned root.
> **Stories:**
> - S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
> - S2 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.
> - S3 (Operator, Recovering a log after a crash) — As an operator, I want a writable open to repair an interrupted append as it does today so that a crash costs no history.
> - S4 (Operator, Recovering a log after a crash) — As an operator, I want to be told which leftover temporary leaf files an open skipped so that I can clear them myself, knowing the store never will.
> - S5 (Verifier, Trusting a leaf the log serves) — As a verifier, I want a leaf damaged inside the pinned prefix to be refused by name at open so that a corrupted leaf is never served as whole.
> - S6 (Developer, Building a reader on the leaf store) — As a developer, I want the leaf store's module doc to say what open does and never does so that I know which call proves a leaf whole.

## Purpose

This brief delivers the leaf-store half of the integrity row the words call conformance row 4.3: the flight recorder's log refuses a leaf it cannot prove whole, and its witness reads only what is new. docs/design/identity/CONFORMANCE.md carries no flight-recorder row (its row 4.3 is a roles row), which is a finding for the lead to add one; until then this brief passes in part the lys-log-store design's goal that no leaf is served as whole unless it is proven whole against the pinned root, and LYSLOGSTORE-002 carries the other part, the witness that reads only what is new. A reader gets an open that never writes and never repairs, refusals that are named errors, the leftovers the open skipped by name, and a module doc that says where a leaf is proven whole.

## Task

Work in crates/lys-log-store on main as it stands. Main already names a leaf only after its bytes are flushed, links it under a name that is never replaced, takes temporary leaf sequences from a parameter, and isolates the no-replace link under a rename injection; do not redo or change any of that (CN1). Of the earlier LYSLOGSTORE-001 on brief/lys-log-store/0e2bb567, main already meets R1 to R4, R5's writable open, R7, and R8's tests for those parts. This brief carries what remains of the leaf store's part: R5's read-only open and R6's module-doc section, reshaped by these words so that a read-only open refuses a store one leaf past its pin with RepairPending instead of opening it at the pinned head. It also carries the leaf store's part of LYSLOGSTORE-002 on brief/lys-log-store/6826e0cf as that brief stands: its pinned-prefix refusal tests (R6 here) and its leftover-temporary accessor (R5 here). LYSLOGSTORE-002 is cut back to the witness only.

In: StoreError::ReadOnly and StoreError::RepairPending (R1); FileLeafStore::open_read_only, an inherent constructor that leaves the LeafStore trait and BACKENDS.md unchanged (R2); ReadOnly refusals of put_leaf and pin (R3); a test that a store one leaf past its pin, the state a read-only open refuses, is repaired by a writable open as today (R4); FileLeafStore::leftover_temporaries (R5); tests of Log::open's refusal of a torn pinned prefix and its repair of a torn leaf past the pin (R6); and the module-doc section (R7). Build R1 first; R2 depends on it, R3 on R2, R4 on R2, R5 extends the walk R2 shares, and R7 describes all of them.

A leaf changed by one byte after it was named is refused by name on open, meaning the caller receives the named error StoreError::PinMismatch from Log::open and never a repaired or partial answer; the store keeps one tree size and one root and cannot name the leaf's index. That case is already met on main by log::tests::a_tampered_leaf_byte_is_detected_at_open, and this brief carries no new requirement for it.

Out: any per-leaf hash record or refusal naming a leaf index; any read-only mode on Log or on Anchor (ADR-106); any change to Log's code, the LeafStore trait, state.json, log.json, the leaves/ layout or BACKENDS.md; the witness, which is LYSLOGSTORE-002's; and the consumers' read paths. The finding stands plainly: after this brief, Anchor::open_read_only (crates/lys-anchor/src/anchor/read_only.rs) and the lys log commands (crates/lys/src/commands/log/store.rs) still open with the writable FileLeafStore::open and Log::open, so a person running status still gets a write, and the integrity row is not passed in full until the further unit that moves them onto open_read_only lands.

This brief supersedes brief/lys-log-store/0e2bb567 and draft/lys-log-store/9aee27d1, this subject's earlier words, which are closed at sign-off. Its id is LYSLOGSTORE-003 because LYSLOGSTORE-001 is held on five refs across four cards and LYSLOGSTORE-002 on two.

## Requirements

### R1: Name the two read-only refusals in StoreError

StoreError gains two variants, each with a /// doc and each field documented. ReadOnly has the fields path: PathBuf, the store's directory, and operation: &'static str, the act refused, and the message `refusing to {operation} in the log store at {path}: it was opened read-only`. RepairPending has the fields path: PathBuf, the store's directory, pinned_size: u64, the pinned tree size, and extent: u64, the number of leaves counted, and the message `refusing to open the log store at {path} read-only: it holds {extent} leaves but its pin is at tree size {pinned_size}, an interrupted append that only a writable open repairs`. StoreError SHALL stay #[non_exhaustive]. No existing variant's fields, message or documentation SHALL change, and neither new variant SHALL carry or print a leaf index.

**Acceptance:**
- `StoreError::ReadOnly { path: PathBuf::from("store"), operation: "write a leaf" }.to_string()` equals `refusing to write a leaf in the log store at store: it was opened read-only`.
- `StoreError::RepairPending { path: PathBuf::from("store"), pinned_size: 1, extent: 2 }.to_string()` equals `refusing to open the log store at store read-only: it holds 2 leaves but its pin is at tree size 1, an interrupted append that only a writable open repairs`.
- Both assertions above are made by the test file::tests::the_read_only_refusals_name_the_store_and_the_refused_act, and `cargo test -p lys-log-store --all-features --lib file::tests::the_read_only_refusals_name_the_store_and_the_refused_act -- --exact` prints `test file::tests::the_read_only_refusals_name_the_store_and_the_refused_act ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- `git diff` of crates/lys-log-store/src/error.rs against the base commit adds exactly the two variants ReadOnly and RepairPending and removes or changes no existing line.

**Files:**
- modify: crates/lys-log-store/src/error.rs
- modify: crates/lys-log-store/src/file_tests.rs

**Checklist:**
- C1 — StoreError has a documented ReadOnly variant carrying the store's directory and the refused act.
- C2 — StoreError has a documented RepairPending variant carrying the store's directory, the pinned tree size and the extent.

**Stories:**
- S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
- S2 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.

### R2: Open a store for reading, and refuse one that is one leaf past its pin

FileLeafStore gains a public, documented inherent constructor open_read_only(dir: &Path) -> StoreResult<FileLeafStore>, the open for a reader, whose # Errors section names StoreError::RepairPending. WHEN open_read_only is called, THE SYSTEM SHALL perform every check FileLeafStore::open performs, in the same order and with the same errors (NotInitialized, Corrupt, Io), and SHALL count the named leaves as FileLeafStore::open does. It SHALL NOT flush leaves/ or any other directory, and SHALL NOT create, write, rename, link or remove any file. IF the counted extent equals the pinned tree size plus one, THEN THE SYSTEM SHALL return StoreError::RepairPending with path the store's directory, pinned_size the pinned tree size and extent the counted extent, and SHALL NOT return a handle. It SHALL decide this from the count and state.json alone and SHALL NOT read leaf bytes: RepairPending says a leaf stands one past the pin, and whether that is a clean interrupted append or a damaged prefix is decided by a writable open, which repairs or refuses with PinMismatch as it does on main. WHEN the counted extent is any other value, THE SYSTEM SHALL return the handle, over which Log::open accepts a tree that matches the pin and refuses any other with PinMismatch without calling pin. The LeafStore trait SHALL NOT gain a method, and FileLeafStore::open's behaviour SHALL NOT change.

**Acceptance:**
- A store made by FileLeafStore::create at a fresh directory D, then put_leaf(0, b"leaf-0") on that handle with no pin (extent 1, pinned tree size 0): FileLeafStore::open_read_only(D) returns Err(StoreError::RepairPending { path: D, pinned_size: 0, extent: 1 }), and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::open_read_only_refuses_a_store_one_leaf_past_its_pin_and_changes_no_byte, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_a_store_one_leaf_past_its_pin_and_changes_no_byte -- --exact` prints `test file::tests::open_read_only_refuses_a_store_one_leaf_past_its_pin_and_changes_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- FileLeafStore::open_read_only on a fresh directory holding no log.json returns Err(StoreError::NotInitialized { path }) with path equal to that directory. The test is file::tests::open_read_only_on_an_uninitialized_dir_is_not_initialized, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_on_an_uninitialized_dir_is_not_initialized -- --exact` prints `test file::tests::open_read_only_on_an_uninitialized_dir_is_not_initialized ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- A log made at a fresh directory D with `leaf-0` and `leaf-1` appended through Log::append: Log::open(FileLeafStore::open_read_only(D)?) returns a log whose tree().len() is 2 and whose recovered_to() is None, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is log::tests::a_log_opens_over_a_read_only_store_at_its_pin_and_changes_no_byte, and `cargo test -p lys-log-store --all-features --lib log::tests::a_log_opens_over_a_read_only_store_at_its_pin_and_changes_no_byte -- --exact` prints `test log::tests::a_log_opens_over_a_read_only_store_at_its_pin_and_changes_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- A log made at a fresh directory D whose state.json bytes are saved right after creation, then `leaf-0` and `leaf-1` appended through Log::append, then state.json rewritten with the saved bytes (extent 2, pinned tree size 0): FileLeafStore::open_read_only(D) returns a handle with extent() 2, Log::open over that handle returns StoreError::PinMismatch with pinned_size 0 and rebuilt_size 2, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is log::tests::a_read_only_store_two_leaves_past_its_pin_is_refused_without_a_write, and `cargo test -p lys-log-store --all-features --lib log::tests::a_read_only_store_two_leaves_past_its_pin_is_refused_without_a_write -- --exact` prints `test log::tests::a_read_only_store_two_leaves_past_its_pin_is_refused_without_a_write ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The body of FileLeafStore::open_read_only in crates/lys-log-store/src/file.rs, and every private function it calls, calls neither fsync_dir, sync_dir, write_state nor write_durably.

**Files:**
- modify: crates/lys-log-store/src/file.rs
- modify: crates/lys-log-store/src/file_tests.rs
- modify: crates/lys-log-store/src/log_tests.rs

**Checklist:**
- C3 — FileLeafStore::open_read_only performs FileLeafStore::open's checks and returns a handle without flushing leaves/ or writing any file.
- C4 — FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending and every file under the store directory holds the same bytes before and after.

**Stories:**
- S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
- S2 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.

### R3: Refuse a leaf write and a pin on a read-only handle

WHEN put_leaf is called on a handle returned by FileLeafStore::open_read_only, THE SYSTEM SHALL return StoreError::ReadOnly with path the store's directory and operation `write a leaf` before any other check, SHALL NOT create, write, link, remove or flush any file, and SHALL NOT advance the extent. WHEN pin is called on such a handle, THE SYSTEM SHALL return StoreError::ReadOnly with path the store's directory and operation `pin` before PinWentBackwards and PinRootChanged are checked, SHALL NOT write state.json or state.json.tmp, and SHALL NOT change pinned(). A handle returned by FileLeafStore::open or FileLeafStore::create SHALL keep main's put_leaf and pin behaviour, and the leaf write SHALL NOT otherwise change.

**Acceptance:**
- A store made by FileLeafStore::create at a fresh directory D, put_leaf(0, b"leaf-0"), and pin to tree size 1 with the root AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves gives for [`leaf-0`], then reopened with FileLeafStore::open_read_only(D): put_leaf(1, b"leaf-1") returns Err(StoreError::ReadOnly { path: D, operation: "write a leaf" }), put_leaf(0, b"leaf-X") returns the same error and not LeafAlreadyWritten, extent() is still 1, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::a_read_only_handle_refuses_a_leaf_write_and_changes_no_byte, and `cargo test -p lys-log-store --all-features --lib file::tests::a_read_only_handle_refuses_a_leaf_write_and_changes_no_byte -- --exact` prints `test file::tests::a_read_only_handle_refuses_a_leaf_write_and_changes_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The same read-only handle: pin(PinnedRoot { tree_size: 2, root: [0; 32] }) returns Err(StoreError::ReadOnly { path: D, operation: "pin" }), pin with tree_size 0 returns the same error and not PinWentBackwards, pinned() equals the pin held before both calls, D holds no state.json.tmp, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::a_read_only_handle_refuses_a_pin_and_changes_no_byte, and `cargo test -p lys-log-store --all-features --lib file::tests::a_read_only_handle_refuses_a_pin_and_changes_no_byte -- --exact` prints `test file::tests::a_read_only_handle_refuses_a_pin_and_changes_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Every test in crates/lys-log-store/src/file_tests.rs that exists on the base commit passes with its assertions unchanged.

**Files:**
- modify: crates/lys-log-store/src/file.rs
- modify: crates/lys-log-store/src/file_tests.rs

**Checklist:**
- C5 — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
- C6 — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.

**Stories:**
- S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.

### R4: Hold the writable repair of a store one leaf past its pin

WHEN a store one leaf past its pin, the state FileLeafStore::open_read_only refuses with RepairPending under R2, is opened with FileLeafStore::open and Log::open, THE SYSTEM SHALL repair it exactly as main does: adopt the one leaf past the pin, pin it, and report it through recovered_to. This requirement adds a test to crates/lys-log-store/src/log_tests.rs. It SHALL NOT change Log::open, reconcile_with_pin or the one-leaf repair. The test SHALL establish the interrupted append through a FileLeafStore::open handle and SHALL NOT call open_read_only before the repair, so that R2's test alone holds the RepairPending refusal and R2's byte-for-byte check alone holds that the refusal leaves the store unchanged.

**Acceptance:**
- A log made at a fresh directory D with `leaf-0` appended through Log::append, its state.json bytes saved, `leaf-1` appended, and state.json rewritten with the saved bytes: a handle from FileLeafStore::open(D) has extent() 2 and pinned().tree_size 1, and is dropped before Log::open is called. Log::open(FileLeafStore::open(D)?) then returns a log whose tree().len() is 2 and whose recovered_to() is Some(2). FileLeafStore::open_read_only(D) after that returns a handle whose extent() is 2 and whose pinned().tree_size is 2. The test is log::tests::a_writable_open_repairs_a_store_one_leaf_past_its_pin, and `cargo test -p lys-log-store --all-features --lib log::tests::a_writable_open_repairs_a_store_one_leaf_past_its_pin -- --exact` prints `test log::tests::a_writable_open_repairs_a_store_one_leaf_past_its_pin ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- log::tests::crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it passes with its assertions unchanged, and `cargo test -p lys-log-store --all-features --lib log::tests::crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it -- --exact` prints `test log::tests::crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.

**Files:**
- modify: crates/lys-log-store/src/log_tests.rs

**Checklist:**
- C7 — A store one leaf past its pin, the state a read-only open refuses with RepairPending, is repaired by FileLeafStore::open followed by Log::open, which reports recovered_to as Some of the extent.

**Stories:**
- S3 (Operator, Recovering a log after a crash) — As an operator, I want a writable open to repair an interrupted append as it does today so that a crash costs no history.

### R5: Name the leftover temporary leaf files an open skipped

WHEN FileLeafStore::open or FileLeafStore::open_read_only reads leaves/, THE SYSTEM SHALL collect the name of every entry of the store's temporary-leaf form: a dot, one or more decimal digits, a dash, twenty decimal digits, a dash, one or more decimal digits, and the suffix .tmp. THE SYSTEM SHALL return these names in lexical order from a public, documented FileLeafStore accessor leftover_temporaries(&self) -> &[String], which SHALL return an empty slice when there are none; a handle from FileLeafStore::create SHALL return an empty slice. THE SYSTEM SHALL NOT count such an entry toward the extent, and SHALL NOT remove, rename, open for writing or otherwise modify it. A dot-prefixed name outside that form SHALL NOT be reported and keeps main's handling. THE SYSTEM SHALL NOT add a method to the LeafStore trait and SHALL NOT change Log, state.json, log.json or any leaf file.

**Acceptance:**
- A store made by FileLeafStore::create at a fresh directory D with put_leaf(0, b"a") and a pin to tree size 1 with the root AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves gives for [`a`], then a file D/leaves/.4242-00000000000000000001-0.tmp holding `partial` written by hand: FileLeafStore::open(D) gives extent() 1 and leftover_temporaries() equal to [".4242-00000000000000000001-0.tmp"], FileLeafStore::open_read_only(D) gives the same two values, and after both opens the file holds exactly `partial`. The test is file::tests::leftover_temporaries_names_the_stores_own_temporary_files, and `cargo test -p lys-log-store --all-features --lib file::tests::leftover_temporaries_names_the_stores_own_temporary_files -- --exact` prints `test file::tests::leftover_temporaries_names_the_stores_own_temporary_files ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- A store holding one leaf and the hand-written files D/leaves/.4242-00000000000000000001-1.tmp and D/leaves/.17-00000000000000000001-0.tmp opens with FileLeafStore::open giving leftover_temporaries() equal to [".17-00000000000000000001-0.tmp", ".4242-00000000000000000001-1.tmp"]. The test is file::tests::leftover_temporaries_are_in_lexical_order, and `cargo test -p lys-log-store --all-features --lib file::tests::leftover_temporaries_are_in_lexical_order -- --exact` prints `test file::tests::leftover_temporaries_are_in_lexical_order ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- A store holding one leaf and no hand-written file opens with FileLeafStore::open giving leftover_temporaries() empty and extent() 1; the same store with a file D/leaves/.DS_Store added opens giving leftover_temporaries() empty and extent() 1. The test is file::tests::leftover_temporaries_is_empty_without_one_of_the_stores_form, and `cargo test -p lys-log-store --all-features --lib file::tests::leftover_temporaries_is_empty_without_one_of_the_stores_form -- --exact` prints `test file::tests::leftover_temporaries_is_empty_without_one_of_the_stores_form ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- file::tests::open_leaves_a_leftover_temporary_file_byte_identical and file::tests::a_leftover_temporary_file_is_ignored_at_open pass with their assertions unchanged.

**Files:**
- modify: crates/lys-log-store/src/file.rs
- modify: crates/lys-log-store/src/file_tests.rs

**Checklist:**
- C10 — FileLeafStore::leftover_temporaries returns the store's temporary-leaf names in lexical order, none counted toward the extent and none changed.

**Stories:**
- S4 (Operator, Recovering a log after a crash) — As an operator, I want to be told which leftover temporary leaf files an open skipped so that I can clear them myself, knowing the store never will.

### R6: Hold Log::open's refusal of a torn pinned prefix, and its repair of a torn leaf past the pin, under tests

IF a leaf at an index below the pinned tree size is torn short after it was named, THEN THE SYSTEM SHALL return StoreError::PinMismatch from Log::open, carrying the pin's tree size and base64 root and the tree size and base64 root the stored leaves rebuild to. THE SYSTEM SHALL NOT name a leaf index in that error, SHALL NOT pin, and SHALL NOT write state.json. WHEN the stored leaves are exactly one more than the pinned tree size and the pinned-size prefix rebuilds to the pinned root, THE SYSTEM SHALL adopt the extra leaf whatever its bytes, pin it and report it through recovered_to, as main does. This requirement adds tests to crates/lys-log-store/src/log_tests.rs and changes no library code; it SHALL NOT change Log::open, reconcile_with_pin, or StoreError::PinMismatch's fields or message, and SHALL NOT add a per-leaf hash record.

**Acceptance:**
- A log made at a fresh directory D with `leaf-0`, `leaf-1` and `leaf-2` appended through Log::append, then D/leaves/00000000000000000001 rewritten to hold `lea`: Log::open(FileLeafStore::open(D)?) returns StoreError::PinMismatch with pinned_size 3, pinned_root the standard base64 of the log's root after the third append, rebuilt_size 3, and rebuilt_root the standard base64 of the root AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves gives for [`leaf-0`, `lea`, `leaf-2`]; its Display text equals `stored leaves rebuild to tree size 3 with root X, but the pinned state is tree size 3 with root Y` with X that rebuilt_root and Y that pinned_root; and D/state.json holds the same bytes before and after. The test is log::tests::a_torn_leaf_inside_the_pinned_prefix_is_refused_with_both_trees, and `cargo test -p lys-log-store --all-features --lib log::tests::a_torn_leaf_inside_the_pinned_prefix_is_refused_with_both_trees -- --exact` prints `test log::tests::a_torn_leaf_inside_the_pinned_prefix_is_refused_with_both_trees ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- A log made at a fresh directory D with `leaf-0` and `leaf-1` appended through Log::append, then a file D/leaves/00000000000000000002 holding `lea` written by hand: Log::open(FileLeafStore::open(D)?) returns a log whose recovered_to() is Some(3) and whose tree().len() is 3, and a second such open returns a log whose recovered_to() is None and whose tree().len() is 3. The test is log::tests::a_torn_leaf_just_past_the_pin_is_repaired_and_reported, and `cargo test -p lys-log-store --all-features --lib log::tests::a_torn_leaf_just_past_the_pin_is_repaired_and_reported -- --exact` prints `test log::tests::a_torn_leaf_just_past_the_pin_is_repaired_and_reported ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- `git diff` of crates/lys-log-store/src/log.rs against the base commit is empty.

**Files:**
- modify: crates/lys-log-store/src/log_tests.rs

**Checklist:**
- C8 — A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch carrying both trees, and state.json is unchanged.
- C9 — A torn leaf just past the pin is adopted by the one-leaf repair and reported by recovered_to.

**Stories:**
- S5 (Verifier, Trusting a leaf the log serves) — As a verifier, I want a leaf damaged inside the pinned prefix to be refused by name at open so that a corrupted leaf is never served as whole.
- S3 (Operator, Recovering a log after a crash) — As an operator, I want a writable open to repair an interrupted append as it does today so that a crash costs no history.

### R7: Say in file.rs's module doc what open does and never does

file.rs's module doc gains a section headed `# What open does and never does`. It SHALL state: FileLeafStore::open reads log.json and state.json, flushes leaves/, counts the 20-digit leaf names and requires them to be contiguous from 0, and returns a writable handle; FileLeafStore::open_read_only performs the same checks without the flush, refuses a store one leaf past its pin with StoreError::RepairPending, and returns a handle whose put_leaf and pin refuse with StoreError::ReadOnly; neither open reads leaf bytes, repairs a store or advances the pin, since the one-leaf repair is Log::open's over a writable handle; neither open counts a dot-prefixed name; neither open ever deletes, renames or changes a leftover temporary file, and both name the store's own through leftover_temporaries; FileLeafStore::leaf serves bytes it has not checked, a leaf is proven whole only through Log::open against the pinned root, and every reader that wants a proven leaf goes through Log::open. The existing Layout and Durability sections SHALL NOT be weakened or removed, and the section SHALL NOT claim that the leaves/ flush holds on targets where fsync_dir does nothing.

**Acceptance:**
- crates/lys-log-store/src/file.rs's //! module doc holds a line `//! # What open does and never does`, and the section under it contains the words `never deletes` applied to a leftover temporary file, names FileLeafStore::open_read_only, StoreError::RepairPending, StoreError::ReadOnly and leftover_temporaries, and states that FileLeafStore::leaf serves bytes it has not checked and that a leaf is proven whole only through Log::open.
- The `//! # Layout` and `//! # Durability` sections of file.rs hold the same lines as on the base commit.
- `cargo doc --no-deps -p lys-log-store` and `cargo doc --no-deps --all-features -p lys-log-store` each finish with no line starting `warning` or `error`.
- `grep -v '^[[:space:]]*$' crates/lys-log-store/src/file.rs | grep -v '^[[:space:]]*//' | wc -l` prints a number no greater than 500.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C11 — file.rs's module doc has a section stating what open and open_read_only do and never do, including that neither deletes a leftover temporary file.
- C12 — file.rs's module doc states that FileLeafStore::leaf serves bytes it has not checked and that a leaf is proven whole only through Log::open against the pinned root.

**Stories:**
- S6 (Developer, Building a reader on the leaf store) — As a developer, I want the leaf store's module doc to say what open does and never does so that I know which call proves a leaf whole.

## Boundaries

- SHALL NOT change put_leaf_with, write_leaf_temp, create_leaf_temp, link_leaf, the next_sequence parameter or the AfterLink seams, except to add the read-only refusal ahead of put_leaf's existing checks.
- SHALL NOT change Log's code, its one-leaf repair, or StoreError::PinMismatch's fields or message.
- SHALL NOT add a read-only mode to Log or to Anchor, and SHALL NOT change lys-anchor, lys-anchor-cli or lys.
- SHALL NOT add a method to the LeafStore trait, and SHALL NOT change store.rs or BACKENDS.md.
- SHALL NOT add a per-leaf hash record, and SHALL NOT change state.json, log.json, the leaves/ layout, the leaf name width or the leaf file bytes.
- SHALL NOT name, guess or bisect for a leaf index in any refusal.
- SHALL NOT delete, rename or modify a leftover temporary file on any open, and SHALL NOT make one an error.
- SHALL NOT touch the lys-anchor witness module, which is LYSLOGSTORE-002's.
- SHALL NOT add #[allow], #[ignore], a _-prefixed bypass, or unwrap, expect, panic, todo, unimplemented or unreachable in library code.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, each exiting 0.
- From the repository root: `sh scripts/design/gate.sh` exits 0.
- Each test named in R1 to R6 runs alone with `cargo test -p lys-log-store --all-features --lib <module>::tests::<name> -- --exact` and prints `test <module>::tests::<name> ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The one-byte case already met on main: `cargo test -p lys-log-store --all-features --lib log::tests::a_tampered_leaf_byte_is_detected_at_open -- --exact` prints `test log::tests::a_tampered_leaf_byte_is_detected_at_open ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- `git diff --stat` against the base commit lists only crates/lys-log-store/src/error.rs, file.rs, file_tests.rs and log_tests.rs outside docs/design.
- Drift injection, one at a time and reverted after each: removing the RepairPending check from open_read_only fails file::tests::open_read_only_refuses_a_store_one_leaf_past_its_pin_and_changes_no_byte and no other test; removing the read-only check from put_leaf fails file::tests::a_read_only_handle_refuses_a_leaf_write_and_changes_no_byte and no other test; removing it from pin fails file::tests::a_read_only_handle_refuses_a_pin_and_changes_no_byte and no other test.

