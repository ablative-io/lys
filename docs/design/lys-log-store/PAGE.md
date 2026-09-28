# lys-log-store — what was asked, what it means, and what was written

## The words, as they were typed

Conformance row 4.3 says the flight recorder's log refuses a leaf it cannot prove whole, and its witness reads only what is new. This card carries the leaf-store half of that row.

lys main at a426b9a5 already names a leaf only after its bytes are flushed, links it under a name that is never replaced, takes temporary leaf sequences from a parameter, and isolates the no-replace link under a rename injection; those four commits are the first half of the earlier words, and this card does not redo them. The survey reads the tree as it stands and names each earlier requirement that main already meets, so the brief carries only what remains.

What remains is this. A leaf store opened read only never writes and never repairs: open_read_only refuses put_leaf and pin by name with ReadOnly, and opening an interrupted append read only refuses by name with RepairPending instead of repairing it. A leaf changed by one byte after it was named is refused by name on open, checked against the tree the store already keeps, so a corrupted leaf is never served as whole. The module doc of the leaf store says what open does and never does, and says that open never deletes a leftover temporary file. The witness half of the row, reading only what is new, is carried by LYSLOGSTORE-002 on its own card and is not this card's.

Acceptance: put_leaf and pin through a read-only handle each refuse with ReadOnly and change no byte of the store; a store with an interrupted append opened read only refuses with RepairPending and the store's bytes are unchanged; the same store opened for writing repairs as today; a leaf whose bytes are changed by one byte after naming is refused by name on open; and every test named in the brief runs by a command a stranger can run against lys main, with its expected output stated. The brief names row 4.3 as the row it passes in part, and names LYSLOGSTORE-002 as the other part.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh), and commit the cluster's rendered markdown with the brief so the gate exits 0 on the branch. The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing; the path docs/design/lys-log-store/briefs/LYSLOGSTORE-001.json is held by two open drafts, a1163a79 and e8440f97, on other subjects, so this brief takes the next free number and does not reuse LYSLOGSTORE-001. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 28 September 2026, against lys main a426b9a5.


Correction of the lead, Archie, given on 28 September 2026 after reading the brief written by run d632be44 at fa360948, which is not signed off. LYSLOGSTORE-002 stands as written and is signed off, so this brief carries only the read-only open, its refusals, the repair rule and the module doc: the rows that restate the torn-prefix and leftover-temporary tests of LYSLOGSTORE-002 are removed, and the brief cites LYSLOGSTORE-002 by id for them. design.json keeps every member as the branch of LYSLOGSTORE-002 leaves it, its title, intention, decisions, goals, principles, constraints, inventory, checklist and stories included, and this brief adds only its own rows, its decision and its stories; nothing of LYSLOGSTORE-002 is rewritten or renumbered. The brief waits for LYSLOGSTORE-002 to land on main, checked by a command a stranger can run against lys main. The verification runs cargo fmt --all, never a fmt check. No file outside the structure array is created, PAGE.md among them, and the rendered markdown of the cluster is committed so that sh scripts/design/gate.sh exits 0 on the branch. Nothing else in the brief changes. Answered by Archie, lead for the identity line.

## What the survey found, and its angles

The words ask for a brief for the leaf-store half of an integrity row. A FileLeafStore opened with a new open_read_only never writes and never repairs. Through it, put_leaf and pin refuse with a new StoreError::ReadOnly, and a store left one leaf past its pin refuses with a new StoreError::RepairPending, while a writable open still repairs as it does today. The module doc of file.rs must say what open does and never does, including that it never deletes a leftover temporary file. The brief builds on LYSLOGSTORE-002 (signed off, not yet on main) without rewriting it: it cites 002 for the torn-prefix and leftover-temporary tests and the bounded witness, waits for 002 to land, and takes the next free brief, roadmap and decision ids.

### What the tree holds

- `crates/lys-log-store/src/file.rs` — FileLeafStore::open (lines 190-222) calls fsync_dir on leaves/ and contiguous_extent, which skips every dot-prefixed name. open_read_only must share those checks but not the flush. put_leaf (310) and pin (323) are where ReadOnly must refuse before any other check. The module doc (lines 1-52) says a leftover temporary file is never counted as a leaf, but not that open never deletes one. It has 430 code lines against a 500-line cap, before LYSLOGSTORE-002's R2 adds leftover_temporaries to the same walk.
- `crates/lys-log-store/src/error.rs` — StoreError has 17 variants on main. None of them is ReadOnly or RepairPending. PinMismatch (lines 200-219) carries only the two sizes and roots, and names no leaf.
- `crates/lys-log-store/src/log.rs` — Log::open and reconcile_with_pin (lines 99-146) are where the one-leaf repair happens, through store.pin. Over a read-only handle, a repair would surface as ReadOnly from pin unless the store refuses first with RepairPending. LYSLOGSTORE-002 and ADR-101 say Log's code does not change.
- `crates/lys-log-store/src/store.rs` — The LeafStore trait contract. LYSLOGSTORE-002 CN3 says the trait gains no method, so open_read_only and the ReadOnly refusal must be inherent to FileLeafStore. The trait's pin and put_leaf docs list their errors, and ReadOnly is a FileLeafStore-only addition to them.
- `crates/lys-log-store/src/log_tests.rs` — a_tampered_leaf_byte_is_detected_at_open (line 151) already changes leaf-0 to leaf-X by one byte and asserts PinMismatch, and crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it (line 176) already holds the writable repair. LYSLOGSTORE-002 C8, C9 and C10 extend both.
- `crates/lys-log-store/src/file_tests.rs` — Holds 34 tests. The ReadOnly and RepairPending tests, which must show that every byte of the store is unchanged, sit beside them.
- `crates/lys-anchor/src/anchor/read_only.rs` — Anchor::open_read_only already exists. Its read-only means no signer and no policy, and its doc says it repairs an interrupted append. Its doc examples open FileLeafStore::open, the writable open. The word read-only already has a second meaning in the tree.
- `crates/lys/src/commands/log/store.rs` — Line 46 opens every lys log command with the writable FileLeafStore::open and Log::open, so a status command can still repair (a write) after this card. Moving it to open_read_only is not in the words.
- `crates/lys-anchor-cli/src/commands/anchor/open.rs` — Line 117 opens anchors with the writable FileLeafStore::open. It has the same consequence as the lys CLI.
- `docs/design/lys-log-store/` — On main this holds only BACKENDS.md. On LYSLOGSTORE-002's branch (6826e0cf, head 99b4d0a4) it holds design.json, checklist.json (C1-C12), stories.json (S1-S4), DESIGN.md, CHECKLIST.md, USER-STORIES.md, PAGE.md and briefs/LYSLOGSTORE-002.*. This brief appends only its own rows, decision and stories there.
- `scripts/design/gate.sh` — Validates decisions.json and project.json, then validates each cluster, checks its coverage, and compares its rendered markdown byte for byte. render-cluster.py writes DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/*.md, never PAGE.md.
- `docs/design/identity/CONFORMANCE.md` — The only conformance table in the tree. Its row 4.3 (line 47) is about roles ('Moving a holder to a newer version is a deliberate act…'), not the flight recorder.

### What was already decided

- LYSLOGSTORE-002 — Signed off, on brief/lys-log-store/6826e0cf (99b4d0a4, based on a426b9a5, 3 ahead, 0 behind), not on main. It pins PinMismatch for a torn or changed pinned prefix (C8, C9), the one-leaf repair (C10), leftover_temporaries (C1), the CLI stderr lines (C2, C3, C11, C12) and the bounded witness (C5-C7). Its boundary says SHALL NOT add a read-only mode to Log or to Anchor.
- ADR-101 — On the 002 branch. The pin is the store's only expectation of its leaves. A damaged prefix is refused with PinMismatch naming no leaf index. Rejected: a per-leaf hash record, bisecting for a leaf, and 'a read-only open of a refused log'.
- ADR-100 — On the 002 branch. The witness folds only new leaves through an in-memory projection. This is the witness half, which is not this card's.
- LYSLOGSTORE-003 — The earlier run d632be44 at fa360948, not signed off. It proposed ADR-106: open_read_only is inherent, RepairPending is decided from the count and the pin without reading bytes or flushing, and Log and Anchor get no read-only mode. The lead corrected it for restating 002's rows.
- LYSLOGSTORE-001 — brief/0e2bb567 and drafts 9aee27d1, a1163a79 and e8440f97 all hold this id. 0e2bb567's R1-R4 and R7 are the four commits already on main.
- CN3 (002) — The LeafStore trait gains no method, so read-only must be inherent to FileLeafStore.
- CN7 (002) — No source file over 500 code lines.
- CN8 (002) — lys/log-dir/v1 stores written before this change still open.
- CLAUDE.md wire/one-rule — The leaves/ layout is local state, not a wire contract. The leaf file stays the raw RFC 6962 preimage.
- ADR-004 — Every project stands alone, so the leaf store's read-only open needs nothing from the anchor or the witness.

### What was measured

- Code lines (not blank, not //-comment) in crates/lys-log-store/src/file.rs on main: 430 of the 500 cap (658 physical lines)
- Code lines in log.rs / error.rs / store.rs / lib.rs: 122 / 102 / 14 / 8
- StoreError variants on main: 17; ReadOnly and RepairPending: 0 occurrences in crates/lys-log-store
- #[test] functions in lys-log-store: 34 in file_tests.rs, 13 in log_tests.rs
- Existing test holding a one-byte change refused at open: 1: log::tests::a_tampered_leaf_byte_is_detected_at_open (leaf-0 to leaf-X, asserts PinMismatch)
- Callers of the writable FileLeafStore::open outside lys-log-store (non-test): 2 code sites (lys/src/commands/log/store.rs:46, lys-anchor-cli/src/commands/anchor/open.rs:117) and 2 doc examples in lys-anchor/src/anchor/read_only.rs
- Files in docs/design/lys-log-store on main: 1 (BACKENDS.md); no design.json, so the gate does not yet measure this cluster on main
- Highest decision id across main and all 298 remote heads: ADR-107 (brief/home/a7af2c56), so the next free id is ADR-108 at survey time
- Highest roadmap id across main and all remote heads: RM-067 (brief/home/a7af2c56), so the next free id is RM-068 at survey time
- LYSLOGSTORE brief ids held on remote refs: 001 on 5 refs (brief+draft 0e2bb567, drafts 9aee27d1, a1163a79, e8440f97); 002 on 2 (6826e0cf brief+draft); 003 on 1 (brief d632be44), so the next free id is LYSLOGSTORE-004
- Checklist and story ids LYSLOGSTORE-002 uses: C1-C12, S1-S4; ADR-100, ADR-101; RM-062
- Earlier requirements main already meets (LYSLOGSTORE-001 on 0e2bb567): R1-R4 and R7 (flushed-then-named leaf, no-replace link, sequence parameter, rename-injection isolation) and R5's writable open, in commits 9e0d1fb, 770f234, 1298836, a426b9a
- Conformance tables in the tree / row 4.3 subject: 1 (docs/design/identity/CONFORMANCE.md); row 4.3 is the roles version-move row, and no row names the flight recorder
- Hand branches carrying open_read_only or RepairPending: 0 of 3 (hand/LOGSTORE-leaf, -r3, -r4); hand/logstore-leaf-a equals main

### What it means for the other projects

- haematite — It is the intended second LeafStore backend (BACKENDS.md). Because open_read_only is inherent to FileLeafStore and the trait gains no method, haematite takes on no new adoption criterion, and BACKENDS.md does not change.
- method — The brief is written and gated with the method's scripts vendored under scripts/design (validate.py, check-coverage.py, render-cluster.py). The method itself does not change.
- cambium — The card lives on Cambium and runs through the chain (brief_card, sign-off, card_build_v3, src_pr, src_land). Its sign-off waits on LYSLOGSTORE-002 landing on lys main.
- aion — The aion workflow chain builds the card. Its inputs name the repository, the commit (a426b9a5 or the commit 002 lands at), the card and this brief, never a folder.

### The decisions it stands on

- ADR-101 (honour) — (On LYSLOGSTORE-002's branch, which lands first.) A damaged pinned prefix stays refused by PinMismatch naming no leaf, and no per-leaf hash record is added. Its rejection of 'a read-only open of a refused log' holds, because a read-only open refuses rather than serving a refused log.
- ADR-100 (honour) — The witness's bounded fold is 002's, and this brief touches nothing in lys-anchor's witness.
- ADR-004 (honour) — The leaf store's read-only open stands alone, with no dependency on the anchor, the witness or any engine.
-  (new) — A decision (next free id, ADR-108 at survey time) is needed. FileLeafStore gains an inherent open_read_only that refuses put_leaf and pin with StoreError::ReadOnly and refuses an interrupted append with StoreError::RepairPending. It never flushes, writes or deletes, including leftover temporaries. Log and Anchor gain no read-only mode, and the LeafStore trait gains no method. It records the rejected alternatives: a trait method, a read-only Log, and opening at the pinned head with the repair reported pending.

### What it requires

- StoreError has two new documented variants, ReadOnly and RepairPending; no existing variant's fields or message change, and neither new variant names a leaf index.
- FileLeafStore::open_read_only exists as a documented inherent constructor, and the LeafStore trait has the same methods as on main.
- put_leaf on a read-only handle returns StoreError::ReadOnly, extent() is unchanged, and every file under the store directory (dot-files included) has the same bytes before and after, with no file added or removed.
- pin on a read-only handle returns StoreError::ReadOnly before PinWentBackwards or PinRootChanged is checked, pinned() is unchanged, no state.json.tmp exists, and every file under the store has the same bytes before and after.
- FileLeafStore::open_read_only on a store with an interrupted append (extent one past the pin) returns StoreError::RepairPending, and every file under the store has the same bytes before and after.
- The same store opened with FileLeafStore::open and then Log::open repairs it, and recovered_to() is Some(extent), as on main.
- open_read_only neither flushes nor creates, writes, renames, links or removes any file, leftover temporaries included.
- file.rs's module doc has a section stating what open and open_read_only do and never do, including that open never deletes a leftover temporary file.
- Every test the brief names runs with a stated cargo test -p lys-log-store --all-features --lib <path> -- --exact command, and its expected output line is written down.
- The brief is LYSLOGSTORE-004 (or the next free id at writing, checked with git ls-remote), names row 4.3 as passed in part and LYSLOGSTORE-002 as the other part, and cites 002 by id for the torn-prefix and leftover-temporary tests.
- design.json, checklist.json and stories.json keep every LYSLOGSTORE-002 member unchanged and only append this brief's rows (C13+, S5+), its decision and its structure entry.
- The rendered DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/<id>.md are committed, and sh scripts/design/gate.sh exits 0 on the branch.
- The brief states a command a stranger can run to confirm that LYSLOGSTORE-002 has landed on lys main before the build starts.
- The verification lists cargo fmt --all, clippy in both feature shapes, cargo test --workspace --all-features, cargo doc in both feature shapes, and sh scripts/design/gate.sh.

### What must not change

- The four landed leaf-write commits (9e0d1fb, 770f234, 1298836, a426b9a) and put_leaf_with, write_leaf_temp and link_leaf do not change.
- The LeafStore trait gains no method (002 CN3), and BACKENDS.md does not change.
- Log::open, reconcile_with_pin, the one-leaf repair and StoreError::PinMismatch's fields and message do not change (ADR-101).
- Log and Anchor gain no read-only mode (002 boundary).
- No per-leaf hash record is added, and no refusal names, guesses or bisects for a leaf index.
- state.json, log.json, the leaves/ layout and the leaf file bytes do not change; lys/log-dir/v1 stores written before still open (CN8).
- A leftover temporary file is never deleted and never made an error.
- Nothing of LYSLOGSTORE-002 in design.json, checklist.json or stories.json is rewritten or renumbered, and PAGE.md and any other file outside the structure array are not created.
- No LYSLOGSTORE-001, 002 or 003 id, and no ADR or RM id already held on any remote ref, is reused.
- FileLeafStore::open's writable behaviour, and every existing test's assertions, do not change.
- file.rs stays at or under 500 code lines (CN7).

### What we must put in place first

- LYSLOGSTORE-002 (brief/lys-log-store/6826e0cf, 99b4d0a4) lands on lys main, so the cluster's design.json, checklist.json and stories.json exist there to append to.
- The lead settles which conformance row the brief cites, because identity/CONFORMANCE.md row 4.3 is a roles row.
- Immediately before writing, re-run git ls-remote over all heads to confirm the next free LYSLOGSTORE, ADR and RM ids (004, 108 and 068 at survey time).

### The risks

- file.rs is at 430 of 500 code lines. 002's leftover_temporaries plus open_read_only, two refusals and a doc section may cross the cap and force a split of file.rs.
- If 002's branch changes or is rebased before landing, this brief's appended rows could collide with its ids or structure entries.
- Another card could take ADR-108, RM-068 or LYSLOGSTORE-004 between the survey and the write, because 298 remote heads are live.
- A count-only RepairPending would tell a reader 'repair pending' for a store whose writable open would refuse with PinMismatch.
- 'Read-only' already means 'no signer, no policy, still repairs' in Anchor::open_read_only. Two meanings could confuse readers, and its doc examples use the writable FileLeafStore::open.
- The lys log and lys-anchor commands still open writable after this card, so a status command can still repair. The integrity row is not whole until those read paths move.
- Skipping the fsync on read-only open means a leaf name linked just before a crash may be counted without a durable directory entry. The doc must state that trade.
- A 'bytes unchanged' check that snapshots only named leaves would miss a created .tmp or state.json.tmp. The snapshot must cover every file, dot-files included.

### Still open

- Row 4.3 of the only conformance table (identity/CONFORMANCE.md) is a roles row. Which row should the brief name as the one it passes in part: a new flight-recorder row the lead adds, or a design goal of lys-log-store? The sentence of the words it stands on: "Conformance row 4.3 says the flight recorder's log refuses a leaf it cannot prove whole, and its witness reads only what is new.". Why only the lead can settle it: docs/design/identity/CONFORMANCE.md:47 row 4.3 reads 'Moving a holder to a newer version is a deliberate act…' (roles, owner Waffles), and no row in the tree names the flight recorder. The brief's statement of which row it passes would be false as written.
- Does 'refused by name' for a one-byte change mean the named error StoreError::PinMismatch without a leaf index, which main and 002 C9 already hold? Or must the refusal name the changed leaf, which ADR-101 rejects and the pin alone cannot do? The sentence of the words it stands on: "A leaf changed by one byte after it was named is refused by name on open, checked against the tree the store already keeps, so a corrupted leaf is never served as whole.". Why only the lead can settle it: The store keeps only one pinned tree size and root (store.rs PinnedRoot, file.rs state.json). Naming the leaf needs a per-leaf hash record, which ADR-101 on 002's branch rejects. If it means the named error, the case is already met by log::tests::a_tampered_leaf_byte_is_detected_at_open and 002 C9, and this brief carries no row for it.
- Is the one-byte acceptance line carried here as a new row, or cited to LYSLOGSTORE-002 C9 like the torn-prefix rows? The sentence of the words it stands on: "Correction of the lead, Archie, given on 28 September 2026 after reading the brief written by run d632be44 at fa360948, which is not signed off. LYSLOGSTORE-002 stands as written and is signed off, so this brief carries only the read-only open, its refusals, the repair rule and the module doc: the rows that restate the torn-prefix and leftover-temporary tests of LYSLOGSTORE-002 are removed, and the brief cites LYSLOGSTORE-002 by id for them.". Why only the lead can settle it: The acceptance lists the one-byte refusal, but 002 C9 already states it ('one byte changed returns the same refusal'). The correction removes only the torn-prefix and leftover-temporary restatements, so whether this row stays decides what the brief's acceptance contains.
- Should a read-only open refuse with RepairPending whenever the leaf count is exactly one past the pin, decided without reading leaf bytes? Or only once the pinned prefix is checked and found to rebuild to the pin, so a damaged prefix with one extra leaf is told PinMismatch instead? The sentence of the words it stands on: "A leaf store opened read only never writes and never repairs: open_read_only refuses put_leaf and pin by name with ReadOnly, and opening an interrupted append read only refuses by name with RepairPending instead of repairing it.". Why only the lead can settle it: The two choices give a reader different refusals for the same damaged store. A leaf-store open sees only the count and state.json. Telling an interrupted append from a damaged prefix needs the rebuild that Log::open does, and 002 and ADR-101 keep Log's code unchanged.

### The units beyond the first

- Open the lys log and lys-anchor read paths with FileLeafStore::open_read_only — crates/lys/src/commands/log/store.rs:46 and crates/lys-anchor-cli/src/commands/anchor/open.rs:117 open writable, so status can still repair. Moving them changes what an operator sees (RepairPending instead of a silent repair) and touches two other crates, so it is its own card.
- Add a flight-recorder integrity row to the conformance table — docs/design/identity/CONFORMANCE.md has no flight-recorder row, and its 4.3 is a roles row. The row this card passes in part has to exist somewhere a stranger can read, which is a documents-only change owned by the lead.
- Close the superseded LYSLOGSTORE-003 brief and the stale LYSLOGSTORE-001 drafts — brief/d632be44 (LYSLOGSTORE-003, ADR-106, RM-066) is not signed off and is replaced by this card. Leaving it open holds ids and invites a second build of the same subject.

### The smallest complete shape

One brief, LYSLOGSTORE-004 (the next free id), written on top of LYSLOGSTORE-002 once it has landed on main. It is one crate change in crates/lys-log-store: StoreError::ReadOnly and StoreError::RepairPending in error.rs; an inherent FileLeafStore::open_read_only that never flushes or writes and refuses an interrupted append with RepairPending; ReadOnly refusals of put_leaf and pin; a test that the same store opened writable repairs as today; and the file.rs module-doc section saying what open does and never does, including that it never deletes a leftover temporary. Each test is named with a stranger-runnable cargo command and its expected output. The brief appends one decision, its checklist rows and its stories to the cluster, with the rendered markdown committed so gate.sh exits 0. The one-byte and torn-prefix refusals are cited to LYSLOGSTORE-002 C8 and C9.

## The roadmap row

- **RM-068** — Open a file leaf store read only, refusing every write and a pending repair (fix, idea)
- Summary: A file log store had no open that was guaranteed to leave it as it found it: FileLeafStore::open flushes leaves/, and Log::open repairs an interrupted append by moving the pin. LYSLOGSTORE-004 adds FileLeafStore::open_read_only, which never flushes or writes, refuses put_leaf and pin with StoreError::ReadOnly, and refuses a store whose leaves are past its pin with StoreError::RepairPending, decided from the count and the pin without reading a leaf (ADR-108). A writable open still repairs as before. The module doc of the file store says what open and open_read_only do and never do, including that open never deletes a leftover temporary file. The one-byte, torn-prefix and leftover-temporary refusals and the witness are LYSLOGSTORE-002's (RM-062), which this item builds on.
- Asked by: tom on 2026-09-27T16:42:00+10:00
- Context: A card for the identity line on the Lys board, filed by its lead against lys main a426b9a, with the lead's correction after reading the unsigned LYSLOGSTORE-003 brief of run d632be44. The lead's answers to the survey settled that no conformance row is named as passed, since docs/design/identity/CONFORMANCE.md has no flight-recorder row and its row 4.3 is a roles row; that the one-byte refusal is StoreError::PinMismatch naming no leaf, cited to LYSLOGSTORE-002 C9 and not carried; and that a read-only open refuses with RepairPending whenever its count is past the pin, reading no leaf bytes. The provenance date is the value the card's filing supplied for this row, kept exactly as it was given; it is not the day the words were written, which they state as 28 September 2026 for both the filing and the lead's correction. The lead also ruled that design.json's solution sentence and non-goal reason saying no read-only open or handle is added each gain the clause `, for Log and the anchor; FileLeafStore::open_read_only is added by LYSLOGSTORE-004 under ADR-108`, the one change this card makes to LYSLOGSTORE-002's text.
- Quote: Conformance row 4.3 says the flight recorder's log refuses a leaf it cannot prove whole, and its witness reads only what is new. This card carries the leaf-store half of that row.

lys main at a426b9a5 already names a leaf only after its bytes are flushed, links it under a name that is never replaced, takes temporary leaf sequences from a parameter, and isolates the no-replace link under a rename injection; those four commits are the first half of the earlier words, and this card does not redo them. The survey reads the tree as it stands and names each earlier requirement that main already meets, so the brief carries only what remains.

What remains is this. A leaf store opened read only never writes and never repairs: open_read_only refuses put_leaf and pin by name with ReadOnly, and opening an interrupted append read only refuses by name with RepairPending instead of repairing it. A leaf changed by one byte after it was named is refused by name on open, checked against the tree the store already keeps, so a corrupted leaf is never served as whole. The module doc of the leaf store says what open does and never does, and says that open never deletes a leftover temporary file. The witness half of the row, reading only what is new, is carried by LYSLOGSTORE-002 on its own card and is not this card's.

Acceptance: put_leaf and pin through a read-only handle each refuse with ReadOnly and change no byte of the store; a store with an interrupted append opened read only refuses with RepairPending and the store's bytes are unchanged; the same store opened for writing repairs as today; a leaf whose bytes are changed by one byte after naming is refused by name on open; and every test named in the brief runs by a command a stranger can run against lys main, with its expected output stated. The brief names row 4.3 as the row it passes in part, and names LYSLOGSTORE-002 as the other part.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh), and commit the cluster's rendered markdown with the brief so the gate exits 0 on the branch. The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing; the path docs/design/lys-log-store/briefs/LYSLOGSTORE-001.json is held by two open drafts, a1163a79 and e8440f97, on other subjects, so this brief takes the next free number and does not reuse LYSLOGSTORE-001. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 28 September 2026, against lys main a426b9a5.


Correction of the lead, Archie, given on 28 September 2026 after reading the brief written by run d632be44 at fa360948, which is not signed off. LYSLOGSTORE-002 stands as written and is signed off, so this brief carries only the read-only open, its refusals, the repair rule and the module doc: the rows that restate the torn-prefix and leftover-temporary tests of LYSLOGSTORE-002 are removed, and the brief cites LYSLOGSTORE-002 by id for them. design.json keeps every member as the branch of LYSLOGSTORE-002 leaves it, its title, intention, decisions, goals, principles, constraints, inventory, checklist and stories included, and this brief adds only its own rows, its decision and its stories; nothing of LYSLOGSTORE-002 is rewritten or renumbered. The brief waits for LYSLOGSTORE-002 to land on main, checked by a command a stranger can run against lys main. The verification runs cargo fmt --all, never a fmt check. No file outside the structure array is created, PAGE.md among them, and the rendered markdown of the cluster is committed so that sh scripts/design/gate.sh exits 0 on the branch. Nothing else in the brief changes. Answered by Archie, lead for the identity line.
- Cluster: lys-log-store; briefs: LYSLOGSTORE-004
- Notes: Ids are the next free past main a426b9a and every open brief, draft and hand branch: git ls-remote was read immediately before writing and all 298 branch heads it listed were fetched and read. The highest ids held were ADR-107 and RM-067 (brief/home/a7af2c56) and LYSLOGSTORE-003 (brief/lys-log-store/d632be44), with LYSLOGSTORE-001 on brief/0e2bb567 and drafts 0e2bb567, 9aee27d1, a1163a79 and e8440f97, and LYSLOGSTORE-002 on brief and draft 6826e0cf, so this row is RM-068 with ADR-108 and LYSLOGSTORE-004. The cluster's documents are those of LYSLOGSTORE-002's branch (brief/lys-log-store/6826e0cf at 99b4d0a), with this brief's rows C13 to C19, stories S5 to S7, ADR-108 and two structure entries appended; the one change to LYSLOGSTORE-002 is the two clauses the lead ruled appended to design.json's solution sentence and non-goal reason, as this row's context and the brief's purpose record; LYSLOGSTORE-002's PAGE.md is not carried. Finding for the lead: docs/design/identity/CONFORMANCE.md holds no flight-recorder row and its row 4.3 is a roles row, so the row this card passes in part does not yet exist anywhere a stranger can read; adding it is the lead's, on its own card. Further units, not written: Open the lys log and lys-anchor read paths with FileLeafStore::open_read_only; Add a flight-recorder integrity row to the conformance table; Close the superseded LYSLOGSTORE-003 brief and the stale LYSLOGSTORE-001 drafts.

## The design

---
type: design
cluster: lys-log-store
title: lys-log-store: a store that refuses what its pin cannot vouch for and says what it skipped, and a witness that reads only what is new
---

# lys-log-store: a store that refuses what its pin cannot vouch for and says what it skipped, and a witness that reads only what is new

> **Cluster:** lys-log-store

## Intention

A log store is trusted because a stranger can rebuild its tree from the leaf files and get the pinned root. When the leaves inside the pin no longer rebuild to it, the store refuses to open. It states exactly what it knows, the pin and the root the leaves now give, and claims nothing it cannot prove. Anything the store sees in its own directory and decides not to count is part of that story too, so it is reported to the operator instead of skipped in silence.

A witness is a durable memory of other logs' checkpoints. Its cost per observation should follow what it has not yet read, not the whole length of its own log, so that a witness can keep running for as long as the logs it watches keep growing.

## Problem

Two defects were found in a sweep of crates/lys-log-store and crates/lys-anchor. First, the file leaf store could let a torn leaf become part of the tree. The torn-write half of that is already closed on main: a leaf is written under a temporary name, synced and put in place by a no-replace hard link, which is kept as it stands. What remains is that nothing asserts that a torn or changed leaf inside the pinned prefix is refused at open, and that the leftover temporary file an interrupted append leaves in leaves/ is skipped without a word. Second, every observe in crates/lys-anchor/src/witness/observe.rs folds the witness's entire log through WitnessProjection::rebuild_prefix to find one origin's latest checkpoint. One observe parses every leaf recorded before it, so n observes cost about n squared over two parses and the witness slows as its log grows.

## Solution

The pin is the store's only expectation of its leaves: one tree size and one root (ADR-101). No per-leaf hash record is added. When the leaves inside the pinned prefix, one of them torn or changed, rebuild to a root other than the pinned one, Log::open refuses with StoreError::PinMismatch as it does today. The refusal states the pin's tree size and root and the tree size and root recomputed from the leaves. A single root cannot tell which leaf changed, so the refusal does not name one and nothing claims to. Nothing is pinned and state.json is left as it was. The one-leaf repair for a leaf just past the pin is kept exactly as it is, since it is what makes crash recovery routine: such a leaf, torn or whole, is adopted, pinned and reported through recovered_to. The lys log commands and the lys-anchor commands already turn the refusal into a stderr diagnostic and a non-zero exit, and tests now hold them to it. No read-only open is added, for Log and the anchor; FileLeafStore::open_read_only is added by LYSLOGSTORE-004 under ADR-108.

The file store's open already walks leaves/ to find the contiguous extent. That walk now also recognises names in the store's own temporary-leaf form (a dot, a process id, a dash, the twenty-digit index, a dash, a sequence number and .tmp). It keeps them out of the extent as today, leaves them untouched, and hands their names back to the caller through the opened store. The two command-line tools that open a file store, lys (log commands) and lys-anchor, read those names straight after the store opens and print one stderr line naming them, the same way both already print an interrupted-append recovery. Nothing changes in the LeafStore trait, in Log, in state.json or in the leaf files, so BACKENDS.md and the second-backend criteria are unchanged.

The witness keeps its per-origin memory in memory, as ADR-100 records. WitnessProjection, which is already the per-origin map, also records how many of the anchor's leaves it has folded. One pass over the whole log builds it once, when the caller opens the anchor. After that, observe takes the kept projection, folds only the leaves recorded since its position, compares the new note against it, and then adds the new note to it. The note is not parsed twice. The projection stays derived and rebuildable: discarding it and folding again from the leaves gives the same answer, and a test holds the kept projection to a fresh fold after every step. Parses are counted by a test-only counter on the parse path, so the bound is asserted on the number of parses and never on elapsed time. The witness stays behind the federation feature, because standalone operation is a hard requirement (docs/design/lys-anchor/DECISIONS.md DP19).

## Principles

- **P1** — A leaf the pin cannot vouch for is refused, and the refusal states what the store knows (the pin and the recomputed root) and claims nothing more.
- **P2** — What open decides not to count is named to the operator; a skipped file is never silent.
- **P3** — The leaves are the record. The witness's per-origin memory is a view of them, held in memory, never stored, and it agrees with a fresh fold at every point.
- **P4** — A bound on work is measured by counting the work, not by timing it.

## Decisions

- ADR-100 — The witness keeps a derived per-origin projection in memory and folds only new leaves — The witness's per-origin projection is built by one pass when the anchor is opened. It records how many leaves it has folded, and each observe folds only the leaves recorded since then before comparing, then adds the note it just recorded. The projection lives in memory only and is derived from the leaves, so discarding it and folding again gives the same answer, and it is never authoritative. Observe is bounded for a known origin and a first sighting alike. Rejected: the backward scan, which leaves a first sighting unbounded. Also rejected: any stored index, which would be a second copy of the truth beside the leaves.
- ADR-101 — A file log store's only expectation of its leaves is the pin; a damaged pinned prefix is refused without naming a leaf — The pin is the store's only expectation of its leaves. When the leaves inside the pinned prefix rebuild to another root, Log::open refuses with StoreError::PinMismatch as it does today. That refusal states the pin's tree size and root and the tree size and root recomputed from the leaves, and it never names a leaf index, because which leaf changed is not known. The one-leaf repair for a leaf just past the pin is kept exactly as it is. Rejected: a per-leaf hash record beside the leaves, which changes the layout and BACKENDS.md and adds a crash window. Also rejected: naming a leaf by bisecting the pin's root, which cannot be done. Also rejected: a read-only open of a refused log, and refusing a torn leaf just past the pin, which would turn routine crash recovery into a refusal.
- ADR-108 — A file leaf store opened read only refuses every write and a pending repair, decided from the count and the pin alone — FileLeafStore gains an inherent open_read_only. It performs the checks FileLeafStore::open performs, never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and refuses put_leaf and pin through its handle with StoreError::ReadOnly. Whenever the leaves it counts are more than the pinned tree size it refuses with StoreError::RepairPending, deciding from state.json and the count alone, reading no leaf bytes and judging nothing else: the rebuild that tells an interrupted append from a damaged prefix belongs to the writable open, which is the only path that repairs and the only path that can refuse a damaged prefix with PinMismatch. Neither refusal names a leaf index. Rejected: a read-only method on the LeafStore trait, which every second backend would have to meet. Also rejected: a read-only mode for Log or for the anchor, which LYSLOGSTORE-002 keeps out. Also rejected: opening at the pinned head and reporting the repair as pending, which would serve a store past its pin as though it were whole. Also rejected: rebuilding the pinned prefix at read-only open so that a damaged prefix with a leaf past the pin is told PinMismatch, which moves the writable open's judgement into a store that holds no tree.

## Goals

- Opening a file store whose pinned prefix holds a torn or changed leaf fails with StoreError::PinMismatch stating the pin and the recomputed root, pins nothing, and the lys and lys-anchor status commands exit non-zero with that refusal on stderr.
- A torn leaf just past the pin is repaired, pinned and reported exactly as before this change.
- Opening a file store whose leaves/ holds a leftover temporary leaf file returns that file's name to the caller, and the lys and lys-anchor tools print it on one stderr line.
- After the witness's projection is built once, an observe on a 256-leaf log parses one leaf when no other leaf was recorded since the previous observe.
- Every test that exists in lys-log-store and lys-anchor on main passes, with its assertions unchanged.
- scripts/verify_inclusion.py exits 0 on an inclusion artifact and a leaf file from a store written after this change.

## Non-Goals

- Writing a leaf through a temporary name and putting it in place — Main already writes the temporary file, syncs it and hard-links it into place without replacing an existing leaf; a rename would replace another writer's leaf and break write-once.
- A read-only mode for Log or for the anchor — A refused log fails to open; no read-only handle is added, for Log and the anchor; FileLeafStore::open_read_only is added by LYSLOGSTORE-004 under ADR-108.
- Naming which leaf changed when open refuses a log — The pin is one tree size and one root, which cannot identify a single leaf, and no per-leaf hash record is kept (ADR-101). Which leaf changed is not known and is not claimed.
- Refusing a torn leaf just past the pin — The one-leaf repair is kept exactly as it is, since it is what makes crash recovery routine.
- Changing the leaf format, the checkpoint format or the inclusion artifact format — Out of scope; the leaf file stays the raw RFC 6962 preimage.
- Updating BACKENDS.md or the second backend's adoption criteria — Nothing in the LeafStore contract changes, and no per-leaf check is added.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/lys-log-store/design.json` | this design |  |
| `docs/design/lys-log-store/checklist.json` | the rows this cluster's brief delivers |  |
| `docs/design/lys-log-store/stories.json` | the stories this cluster's brief serves |  |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-002.json` | the refusal at open, the leftover-temporary report and the bounded witness |  |
| `docs/design/lys-log-store/BACKENDS.md` | second-backend adoption criteria; unchanged |  |
| `crates/lys-log-store/src/log_tests.rs` | Log tests, including the torn and changed leaf inside the pin and the torn leaf just past it |  |
| `crates/lys-log-store/src/file.rs` | FileLeafStore; open recognises and reports leftover temporary leaf files |  |
| `crates/lys-log-store/src/file_tests.rs` | FileLeafStore tests, including the leftover-temporary report |  |
| `crates/lys/src/commands/log/store.rs` | the lys log commands' store open; prints the leftover-temporary line |  |
| `crates/lys/tests/log_tests.rs` | lys log binary tests, including the stderr line and the Python verifier run |  |
| `crates/lys-anchor-cli/src/commands/anchor/open.rs` | lys-anchor's anchor open; prints the leftover-temporary line |  |
| `crates/lys-anchor-cli/tests/anchor_cli.rs` | lys-anchor binary tests, including the stderr line |  |
| `crates/lys-anchor/src/witness/projection.rs` | WitnessProjection with its fold position, forward fold and the test-only parse counter |  |
| `crates/lys-anchor/src/witness/projection_tests.rs` | projection tests, including forward fold against a fresh rebuild |  |
| `crates/lys-anchor/src/witness/observe.rs` | observe over a kept projection |  |
| `crates/lys-anchor/src/witness/observe_tests.rs` | observe tests, including the bounded-parse count |  |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-004.json` | the read-only open, its two refusals, the repair rule and the file store's module doc |  |
| `crates/lys-log-store/src/error.rs` | StoreError, including the ReadOnly and RepairPending refusals |  |

## Inventory

- `crates/lys-log-store/src/file.rs` — On main: put_leaf_with writes leaves/.<pid>-<index>-<seq>.tmp, syncs it, hard-links it to the 20-digit name, removes the temporary file and syncs leaves/. open syncs leaves/ before counting. contiguous_extent skips every dot-prefixed name without reporting it. 430 code lines.
- `crates/lys-log-store/src/file_tests.rs` — On main: includes a_leftover_temporary_file_is_ignored_at_open and open_leaves_a_leftover_temporary_file_byte_identical.
- `crates/lys-log-store/src/log.rs` — Log::open rebuilds the tree, reconciles it with the (tree_size, root) pin, repairs exactly one leaf past the pin and reports it through recovered_to, and otherwise returns StoreError::PinMismatch with pinned_size, pinned_root, rebuilt_size and rebuilt_root. Code unchanged by this cluster.
- `crates/lys-log-store/src/log_tests.rs` — On main: a_tampered_leaf_byte_is_detected_at_open, crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it and crash_recovery_does_not_mask_a_tampered_prefix assert the variant only.
- `crates/lys-log-store/src/error.rs` — StoreError::PinMismatch's message: 'stored leaves rebuild to tree size {rebuilt_size} with root {rebuilt_root}, but the pinned state is tree size {pinned_size} with root {pinned_root}'. Unchanged by this cluster.
- `crates/lys-log-store/src/store.rs` — The LeafStore contract and PinnedRoot (tree_size, root). Unchanged by this cluster.
- `crates/lys/src/commands/log/store.rs` — open() opens FileLeafStore then Log and prints 'recovered interrupted append: state advanced to N' on stderr. A PinMismatch becomes a directory-invalid error carrying the store's message, printed as 'error: ...' on stderr with exit status 1.
- `crates/lys-anchor-cli/src/commands/anchor/open.rs` — open() opens FileLeafStore then Anchor and prints the same recovery line on stderr. A PinMismatch becomes a directory-invalid error carrying the store's message, printed as 'error: ...' on stderr with exit status 1.
- `crates/lys-anchor/src/witness/observe.rs` — observe(anchor, note, proof, context) calls WitnessProjection::rebuild_prefix on every call (line 91).
- `crates/lys-anchor/src/witness/projection.rs` — WitnessProjection: a BTreeMap of origin to OriginState folded from leaves; rebuild and rebuild_prefix fold from index 0; checkpoint_in_leaf is the parse path. Module invariant: derived, rebuildable, nothing stored; last recorded wins.
- `scripts/verify_inclusion.py` — Standalone RFC 6962 verifier: verify_inclusion.py <artifact.json> <leaf-file>, exit 0 when the leaf verifies.
- `docs/design/lys-log-store/BACKENDS.md` — The five adoption criteria for a second LeafStore backend.

## Constraints

- **CN1** — The leaf file stays the raw RFC 6962 preimage: no header, framing or hash is added to it.
- **CN2** — The checkpoint note format, the inclusion artifact format and every receipt and bundle format do not change.
- **CN3** — The LeafStore trait gains no method, and keeps no fork, merge, delete, truncate or rewrite.
- **CN4** — A leaf is put in place only by the existing no-replace hard link; nothing renames over a leaf name.
- **CN5** — The receipt observe returns stays byte-identical to Anchor::submit's for the same note at the same tree state.
- **CN6** — Every witness item stays behind the federation feature, and no item outside it names one.
- **CN7** — No source file goes over 500 lines of code, excluding tests, comments and blank lines.
- **CN8** — Store directories in the lys/log-dir/v1 layout (log.json, state.json, leaves/) written before this change still open.


---
type: brief
id: LYSLOGSTORE-002
cluster: lys-log-store
title: Refuse a damaged pinned prefix at open, report leftover temporary leaf files, and bound the witness's observe to the leaves recorded since the last one
---

# LYSLOGSTORE-002: Refuse a damaged pinned prefix at open, report leftover temporary leaf files, and bound the witness's observe to the leaves recorded since the last one

> **Cluster:** lys-log-store
> **Design anchor:**
> - ADR-100 — The witness keeps a derived per-origin projection in memory and folds only new leaves — The witness's per-origin projection is built by one pass when the anchor is opened. It records how many leaves it has folded, and each observe folds only the leaves recorded since then before comparing, then adds the note it just recorded. The projection lives in memory only and is derived from the leaves, so discarding it and folding again gives the same answer, and it is never authoritative. Observe is bounded for a known origin and a first sighting alike. Rejected: the backward scan, which leaves a first sighting unbounded. Also rejected: any stored index, which would be a second copy of the truth beside the leaves.
> - ADR-101 — A file log store's only expectation of its leaves is the pin; a damaged pinned prefix is refused without naming a leaf — The pin is the store's only expectation of its leaves. When the leaves inside the pinned prefix rebuild to another root, Log::open refuses with StoreError::PinMismatch as it does today. That refusal states the pin's tree size and root and the tree size and root recomputed from the leaves, and it never names a leaf index, because which leaf changed is not known. The one-leaf repair for a leaf just past the pin is kept exactly as it is. Rejected: a per-leaf hash record beside the leaves, which changes the layout and BACKENDS.md and adds a crash window. Also rejected: naming a leaf by bisecting the pin's root, which cannot be done. Also rejected: a read-only open of a refused log, and refusing a torn leaf just past the pin, which would turn routine crash recovery into a refusal.
> **Checklist:**
> - C8 — Log::open over a store whose leaf inside the pinned prefix is torn returns StoreError::PinMismatch stating the pin's tree size and root and the recomputed tree size and root, names no leaf, and leaves state.json unchanged.
> - C9 — Log::open over a store whose leaf inside the pinned prefix has one byte changed returns the same refusal, naming no leaf and leaving state.json unchanged.
> - C10 — Log::open over a store with a torn leaf at the index just past the pin repairs and pins it and reports the repair through recovered_to, exactly as before this change.
> - C1 — FileLeafStore::open returns, sorted, the name of every entry in leaves/ that has the store's temporary-leaf form, does not count any of them as a leaf, and leaves each byte-identical.
> - C11 — lys log status on a log whose pinned prefix holds a torn leaf exits 1 and prints the refusal on stderr.
> - C2 — Every lys log command that opens a store prints one stderr line naming the leftover temporary leaf files the store reported.
> - C4 — scripts/verify_inclusion.py exits 0 on an inclusion artifact and a leaf file from a store written after this change.
> - C3 — Every lys-anchor command that opens an anchor prints one stderr line naming the leftover temporary leaf files the store reported.
> - C12 — lys-anchor status on an anchor whose pinned prefix holds a torn leaf exits 1 and prints the refusal on stderr.
> - C5 — WitnessProjection records how many leaves it has folded and folds forward from that position only.
> - C6 — observe parses at most one leaf more than the number of leaves recorded since the previous observe through the same projection, counted on the parse path.
> - C7 — After every observe, the kept projection reports the same latest state for every origin as WitnessProjection::rebuild over the same anchor, including after a rollback.
> **Stories:**
> - S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.
> - S1 (Log operator, Opens a log store, including after a crash) — As a log operator, I want to be told which leftover temporary files opening my store ignored, so that what an interrupted append left behind is visible rather than skipped in silence.
> - S2 (Third-party verifier, Checks a leaf without lys) — As a third-party verifier, I want a leaf from a store written after this change to verify with the standalone Python verifier, so that checking a leaf still needs no lys code.
> - S3 (Witness operator, Runs a witness anchor that observes other logs' checkpoints) — As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.

## Purpose

A file log store must never commit a leaf the pin cannot vouch for, and must never skip a file in silence. Opening a store whose pinned prefix holds a torn or changed leaf already fails with StoreError::PinMismatch, but no test holds the refusal's content, the untouched pin, or the two command-line tools' exit status to that. Opening a store also skips the leftover temporary file an interrupted append leaves in leaves/ without a word. The witness's observe re-parses its whole log on every call, so its cost grows with the square of the log's length. This brief pins the refusal and the one-leaf repair beside it under tests (ADR-101). It makes open name the temporary files it skipped, to the caller and on the two tools' stderr. It also makes observe parse only the leaves recorded since its previous call, through a per-origin projection built once and kept in memory (ADR-100).

## Task

Work on main as it stands, where the temporary-name write and its no-replace hard link have already landed. Keep that write exactly as it is (CN4). In lys-log-store, add tests that plant a torn leaf and a one-byte change inside the pinned prefix and assert Log::open's existing StoreError::PinMismatch refusal, with its full content and an untouched state.json. Add a test that plants a torn leaf just past the pin and asserts the existing one-leaf repair and its report. The refusal is named PinMismatch and states the pin's tree size and root and the tree size and root recomputed from the leaves. It never names a leaf index: the pin is one tree size and one root, so which leaf changed is not known and is not claimed. The walk of leaves/ in FileLeafStore::open recognises the store's own temporary-leaf names, keeps them out of the extent as today, and returns them to the caller. The lys log commands and the lys-anchor commands print one stderr line naming them. Tests hold both tools' status commands to a non-zero exit with the refusal on stderr for a damaged log. In lys-anchor's witness module, which stays behind the federation feature so standalone operation is untouched (docs/design/lys-anchor/DECISIONS.md DP19), WitnessProjection records its fold position and folds forward, and observe takes a kept projection instead of rebuilding one per call. Out of scope: any per-leaf hash record, any read-only open, any narrowing of the one-leaf repair, any change to Log's code, StoreError::PinMismatch, the LeafStore trait, state.json or BACKENDS.md, and the leaf, checkpoint and artifact formats.

## Requirements

### R1: Hold Log::open's refusal of a damaged pinned prefix, and its one-leaf repair, under tests

IF a leaf at an index below the pinned tree size holds bytes other than those appended there, torn short or changed, THEN THE SYSTEM SHALL return StoreError::PinMismatch from Log::open. The error SHALL carry the pin's tree size and base64 root and the tree size and base64 root the stored leaves rebuild to. THE SYSTEM SHALL NOT name a leaf index in that error, SHALL NOT pin, and SHALL NOT write state.json. WHEN the stored leaves are exactly one more than the pinned tree size and the pinned-size prefix rebuilds to the pinned root, THE SYSTEM SHALL adopt the extra leaf whatever its bytes, pin it and report it through recovered_to, exactly as main does. THE SYSTEM SHALL NOT change Log::open, reconcile_with_pin, StoreError::PinMismatch's fields or its message, and SHALL NOT add a per-leaf hash record. This requirement adds tests to crates/lys-log-store/src/log_tests.rs and changes no library code.

**Acceptance:**
- A store created at a fresh directory with origin example.com/log, with leaves `leaf-0`, `leaf-1` and `leaf-2` appended through Log::append, then leaves/00000000000000000001 rewritten to hold `lea`: Log::open(FileLeafStore::open(dir)) returns StoreError::PinMismatch with pinned_size 3, pinned_root the standard base64 of the log's root after the third append, rebuilt_size 3, and rebuilt_root the standard base64 of the root AppendOnlyTree::reconstruct_from_raw_leaves gives for [`leaf-0`, `lea`, `leaf-2`].
- That error's Display text equals `stored leaves rebuild to tree size 3 with root X, but the pinned state is tree size 3 with root Y`, with X the rebuilt_root and Y the pinned_root above.
- state.json in that store holds the same bytes before and after that failed open.
- The same three-leaf store with leaves/00000000000000000001 rewritten to hold `leaf-X` returns StoreError::PinMismatch from Log::open with pinned_size 3 and rebuilt_size 3. Its rebuilt_root is the standard base64 of the root reconstruct_from_raw_leaves gives for [`leaf-0`, `leaf-X`, `leaf-2`], and it differs from pinned_root. state.json holds the same bytes before and after.
- A store with leaves `leaf-0` and `leaf-1` appended through Log::append, then a file leaves/00000000000000000002 holding `lea` written by hand, opens with Log::open: recovered_to() is Some(3) and tree().len() is 3. A second open of the same store returns a log whose recovered_to() is None and whose tree().len() is 3.

**Files:**
- modify: crates/lys-log-store/src/log_tests.rs

**Checklist:**
- C8 — Log::open over a store whose leaf inside the pinned prefix is torn returns StoreError::PinMismatch stating the pin's tree size and root and the recomputed tree size and root, names no leaf, and leaves state.json unchanged.
- C9 — Log::open over a store whose leaf inside the pinned prefix has one byte changed returns the same refusal, naming no leaf and leaving state.json unchanged.
- C10 — Log::open over a store with a torn leaf at the index just past the pin repairs and pins it and reports the repair through recovered_to, exactly as before this change.

**Stories:**
- S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

### R2: Return the store's leftover temporary leaf files from FileLeafStore::open

WHEN FileLeafStore::open reads the leaves/ directory, THE SYSTEM SHALL collect the name of every entry of the store's temporary-leaf form: a dot, one or more decimal digits, a dash, twenty decimal digits, a dash, one or more decimal digits, and the suffix .tmp. THE SYSTEM SHALL return these names in lexical order from a FileLeafStore accessor named leftover_temporaries, and the accessor SHALL return an empty slice when there are none. THE SYSTEM SHALL NOT count such an entry toward the extent, and SHALL NOT remove, rename, open for writing or otherwise modify it. THE SYSTEM SHALL NOT report a dot-prefixed name outside that form; such a name keeps today's handling. THE SYSTEM SHALL NOT add a method to the LeafStore trait, and SHALL NOT change Log, state.json, log.json or any leaf file.

**Acceptance:**
- A store created at a fresh directory with origin example.com/log, with one leaf of bytes `a` appended through Log::append and a file leaves/.4242-00000000000000000001-0.tmp holding the bytes `partial` written afterwards, opens with FileLeafStore::open: extent() is 1 and leftover_temporaries() equals [".4242-00000000000000000001-0.tmp"].
- After that open, leaves/.4242-00000000000000000001-0.tmp holds exactly the bytes `partial`.
- The same store with no planted file opens with leftover_temporaries() empty and extent() 1.
- A store holding leaves/.4242-00000000000000000001-1.tmp and leaves/.17-00000000000000000001-0.tmp opens with leftover_temporaries() equal to [".17-00000000000000000001-0.tmp", ".4242-00000000000000000001-1.tmp"].
- A store holding one leaf and a file leaves/.DS_Store opens with leftover_temporaries() empty and extent() 1.
- crates/lys-log-store/src/file.rs holds no more than 500 lines that are neither blank nor comment lines, not counting tests.

**Files:**
- modify: crates/lys-log-store/src/file.rs
- modify: crates/lys-log-store/src/file_tests.rs

**Checklist:**
- C1 — FileLeafStore::open returns, sorted, the name of every entry in leaves/ that has the store's temporary-leaf form, does not count any of them as a leaf, and leaves each byte-identical.

**Stories:**
- S1 (Log operator, Opens a log store, including after a crash) — As a log operator, I want to be told which leftover temporary files opening my store ignored, so that what an interrupted append left behind is visible rather than skipped in silence.

### R3: Print the leftover temporary files from the lys log commands, and hold their refusal of a damaged log

WHEN a lys log command opens a store whose leftover_temporaries() is not empty, THE SYSTEM SHALL print one line to stderr, straight after the store opens and before the log is opened: the text `ignored leftover temporary leaf files: ` followed by the names in the order returned, separated by a comma and a space. THE SYSTEM SHALL NOT print that line when the list is empty. The line SHALL NOT go to stdout. It SHALL NOT change the command's exit status or its --json output, and SHALL NOT be printed more than once per open. IF opening the log returns StoreError::PinMismatch, THEN THE SYSTEM SHALL exit with status 1 and print one stderr line `error: log directory invalid: <dir>: ` followed by the store's PinMismatch message, as main does. THE SYSTEM SHALL NOT print a leaf index in that line and SHALL NOT write state.json.

**Acceptance:**
- `lys log init --dir D --origin example.com/log` on a fresh directory D, then a file D/leaves/.4242-00000000000000000000-0.tmp holding `partial`: `lys log status --dir D` exits 0, and its stderr has exactly one line starting `ignored leftover temporary leaf files`, which equals `ignored leftover temporary leaf files: .4242-00000000000000000000-0.tmp`.
- `lys log status --dir D` on a directory D made by `lys log init` with no planted file exits 0, and its stderr has no line starting `ignored leftover temporary leaf files`.
- A key made by `lys key generate --out K`, a log D made by `lys log init --dir D --origin example.com/log`, a leaf file L holding `hello` appended by `lys log append --dir D --leaf L`, and an artifact A written by `lys log prove inclusion --dir D --key K --leaf-index 0 --out A`: `python3 scripts/verify_inclusion.py A D/leaves/00000000000000000000` exits 0.
- `lys log init --dir D --origin example.com/log`, then `lys log append --dir D --leaf L` three times with L holding `leaf-0`, `leaf-1` and `leaf-2` in turn, then D/leaves/00000000000000000001 rewritten to hold `lea`: `lys log status --dir D` exits 1. Its stderr has exactly one line starting `error: `, which starts `error: log directory invalid: D: stored leaves rebuild to tree size 3 with root ` and contains `, but the pinned state is tree size 3 with root `. D/state.json holds the same bytes before and after.

**Files:**
- modify: crates/lys/src/commands/log/store.rs
- modify: crates/lys/tests/log_tests.rs

**Checklist:**
- C2 — Every lys log command that opens a store prints one stderr line naming the leftover temporary leaf files the store reported.
- C4 — scripts/verify_inclusion.py exits 0 on an inclusion artifact and a leaf file from a store written after this change.
- C11 — lys log status on a log whose pinned prefix holds a torn leaf exits 1 and prints the refusal on stderr.

**Stories:**
- S1 (Log operator, Opens a log store, including after a crash) — As a log operator, I want to be told which leftover temporary files opening my store ignored, so that what an interrupted append left behind is visible rather than skipped in silence.
- S2 (Third-party verifier, Checks a leaf without lys) — As a third-party verifier, I want a leaf from a store written after this change to verify with the standalone Python verifier, so that checking a leaf still needs no lys code.
- S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

### R4: Print the leftover temporary files from the lys-anchor commands, and hold their refusal of a damaged anchor

WHEN a lys-anchor command opens an anchor directory whose store reports a non-empty leftover_temporaries(), THE SYSTEM SHALL print the same single stderr line R3 defines, straight after the store opens and before the anchor is opened. THE SYSTEM SHALL NOT print it when the list is empty, SHALL NOT print it on stdout, SHALL NOT change the exit status, and SHALL NOT add a field to the --json output. IF opening the anchor returns StoreError::PinMismatch, THEN THE SYSTEM SHALL exit with status 1 and print one stderr line `error: anchor directory invalid: <dir>: ` followed by the store's PinMismatch message, as main does. THE SYSTEM SHALL NOT print a leaf index in that line and SHALL NOT write state.json.

**Acceptance:**
- With a key file K made by Ed25519Identity::load_or_generate and a genesis file G holding `genesis`, an anchor made by `lys-anchor init --dir D --origin example.com/anchor --key K --genesis G --admit accept-all`, then a file D/leaves/.4242-00000000000000000001-0.tmp holding `partial`: `lys-anchor status --dir D --key K --admit accept-all` exits 0, and its stderr has exactly one line starting `ignored leftover temporary leaf files`, which equals `ignored leftover temporary leaf files: .4242-00000000000000000001-0.tmp`.
- The same status command on the same anchor with no planted file exits 0, and its stderr has no line starting `ignored leftover temporary leaf files`.
- With the planted file, `lys-anchor --json status --dir D --key K --admit accept-all` exits 0, its stdout parses as one JSON object whose ok is true, and its keys are the same as those of the same command run without the planted file.
- With a key file K made by Ed25519Identity::load_or_generate and a genesis file G holding `genesis`, an anchor made by `lys-anchor init --dir D --origin example.com/anchor --key K --genesis G --admit accept-all`, then D/leaves/00000000000000000000 rewritten to hold `gen`: `lys-anchor status --dir D --key K --admit accept-all` exits 1. Its stderr has exactly one line starting `error: `, which starts `error: anchor directory invalid: D: stored leaves rebuild to tree size 1 with root ` and contains `, but the pinned state is tree size 1 with root `. D/state.json holds the same bytes before and after.

**Files:**
- modify: crates/lys-anchor-cli/src/commands/anchor/open.rs
- modify: crates/lys-anchor-cli/tests/anchor_cli.rs

**Checklist:**
- C3 — Every lys-anchor command that opens an anchor prints one stderr line naming the leftover temporary leaf files the store reported.
- C12 — lys-anchor status on an anchor whose pinned prefix holds a torn leaf exits 1 and prints the refusal on stderr.

**Stories:**
- S1 (Log operator, Opens a log store, including after a crash) — As a log operator, I want to be told which leftover temporary files opening my store ignored, so that what an interrupted append left behind is visible rather than skipped in silence.
- S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

### R5: Give WitnessProjection a fold position and a forward fold

WitnessProjection SHALL record the number of the anchor's leaves it has folded, returned by an accessor named folded. rebuild and rebuild_prefix SHALL set it to the number of leaves they folded. WHEN fold_to(anchor, leaves) is called, THE SYSTEM SHALL fold the indices from folded up to the smaller of leaves and the anchor's tree size, in ascending order and under the same rule as rebuild_prefix, so the last recorded state wins. It SHALL then set folded to that bound. IF folded is already greater than that bound, THEN THE SYSTEM SHALL discard the projection's contents and fold from index 0. THE SYSTEM SHALL NOT parse any leaf below folded otherwise, SHALL NOT write anything to storage, and SHALL NOT take a mutable reference to the anchor. Under the test configuration only, every call of checkpoint_in_leaf SHALL add one to a per-thread parse counter the tests read. No counter SHALL exist outside the test configuration.

**Acceptance:**
- On a witness anchor holding the genesis leaf, three ordinary statements, a checkpoint note from CHILD_ORIGIN at size 3 and two more ordinary statements (seven leaves), rebuild_prefix(&anchor, 4) gives folded() 4. fold_to(&anchor, 7) then gives folded() 7, and the parse counter rises by exactly 3 during that fold_to.
- After that fold_to, latest(CHILD_ORIGIN) equals WitnessProjection::rebuild(&anchor).latest(CHILD_ORIGIN), with tree_size 3.
- rebuild(&anchor) over those seven leaves raises the parse counter by exactly 7.
- rebuild(&anchor) over those seven leaves followed by fold_to(&anchor, 2) gives folded() 2 and an empty projection, equal to rebuild_prefix(&anchor, 2).
- Child checkpoints at size 5 and then at size 3, recorded in that order and folded by two successive fold_to calls, leave latest(CHILD_ORIGIN) with tree_size 3.
- Every existing projection_tests case passes with its assertions unchanged.

**Files:**
- modify: crates/lys-anchor/src/witness/projection.rs
- modify: crates/lys-anchor/src/witness/projection_tests.rs

**Checklist:**
- C5 — WitnessProjection records how many leaves it has folded and folds forward from that position only.

**Stories:**
- S3 (Witness operator, Runs a witness anchor that observes other logs' checkpoints) — As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.

### R6: Make observe fold only what is new, through a kept projection

observe SHALL take the caller's kept WitnessProjection as a mutable argument after the anchor. WHEN observe has recorded a note at index i through Anchor::submit, THE SYSTEM SHALL call fold_to(anchor, i) on the kept projection and compare the note against it under the existing relation rules. IF checkpoint_in_leaf reads the note as a checkpoint, THEN THE SYSTEM SHALL add that already-parsed body at index i, without parsing the note a second time. IF checkpoint_in_leaf does not read the note as a checkpoint, THEN THE SYSTEM SHALL add nothing. In both cases folded SHALL become i + 1, so the next observe does not parse leaf i again. IF Anchor::submit returns an error, THEN THE SYSTEM SHALL return that error and leave the projection unchanged. THE SYSTEM SHALL NOT call rebuild or rebuild_prefix from observe, and SHALL NOT change the receipt, the relation rules or the errors observe returns. The existing observe_tests helpers and the receipt case's loop SHALL build a projection with WitnessProjection::rebuild before each call, and no assertion in an existing observe_tests case SHALL change.

**Acceptance:**
- A witness anchor holds the genesis leaf, a CHILD_ORIGIN checkpoint note at size 2 as leaf 1, and 254 ordinary statements, 256 leaves in all. Building p with WitnessProjection::rebuild raises the parse counter by 256. observe with p of the child's note at size 3 then raises the counter by exactly 1 and reports previous with tree_size 2.
- Next, observe with p of an OTHER_CHILD_ORIGIN note raises the parse counter by exactly 1 and reports previous as None.
- Next, after three ordinary statements are submitted through Anchor::submit, observe with p of the child's note at size 4 raises the parse counter by exactly 4.
- After each of those three observes, p.folded() equals anchor.tree_size(), and p.latest(CHILD_ORIGIN) and p.latest(OTHER_CHILD_ORIGIN) equal WitnessProjection::rebuild(&anchor)'s for the same origins.
- Through one kept projection, observing the child's notes stating size 5 and then size 3 reports the second as Relation::Rollback, and leaves p.latest(CHILD_ORIGIN) with tree_size 3.
- Through a kept projection, observe of bytes that checkpoint_in_leaf does not read as a checkpoint raises the parse counter by exactly 1, reports previous as None, and leaves p.folded() equal to anchor.tree_size().
- On an anchor whose admission policy is MaxSize with a limit shorter than the note, observe returns AnchorError::NotAdmitted and p.folded() is the same before and after the call.
- Every existing observe_tests case passes with its assertions unchanged, including the_receipt_is_byte_identical_whatever_the_relation.

**Files:**
- modify: crates/lys-anchor/src/witness/observe.rs
- modify: crates/lys-anchor/src/witness/observe_tests.rs

**Checklist:**
- C6 — observe parses at most one leaf more than the number of leaves recorded since the previous observe through the same projection, counted on the parse path.
- C7 — After every observe, the kept projection reports the same latest state for every origin as WitnessProjection::rebuild over the same anchor, including after a rollback.

**Stories:**
- S3 (Witness operator, Runs a witness anchor that observes other logs' checkpoints) — As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.

## Boundaries

- SHALL NOT change put_leaf_with, write_leaf_temp, link_leaf or any other part of the leaf write, and SHALL NOT put a leaf in place by rename.
- SHALL NOT add a read-only mode to Log or to Anchor.
- SHALL NOT add a per-leaf hash record, and SHALL NOT change state.json, log.json, the leaves/ layout or the leaf file bytes.
- SHALL NOT change Log::open, its reconciliation, its one-leaf repair, or StoreError::PinMismatch's fields or message.
- SHALL NOT name, guess or bisect for a leaf index in any refusal: which leaf changed is not known and is not claimed.
- SHALL NOT add a method to the LeafStore trait or change BACKENDS.md.
- SHALL NOT change the checkpoint note format, the inclusion artifact format, or any receipt or bundle format.
- SHALL NOT store the witness projection anywhere but in memory, and SHALL NOT name a witness item outside the federation feature, which keeps standalone operation untouched (docs/design/lys-anchor/DECISIONS.md DP19).
- SHALL NOT make a leftover temporary file an error, and SHALL NOT delete one.

## Verification

- cargo fmt --all -- --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo clippy --all-targets -- -D warnings
- cargo test --workspace --all-features
- cargo doc --no-deps --all-features
- cargo doc --no-deps
- sh scripts/design/gate.sh
- Count the lines of crates/lys-log-store/src/file.rs, crates/lys-anchor/src/witness/projection.rs and crates/lys-anchor/src/witness/observe.rs that are neither blank nor comment lines, not counting tests: each is at most 500.
- git grep -n 'rebuild_prefix\|WitnessProjection::rebuild' -- crates/lys-anchor/src/witness/observe.rs prints nothing.
- git diff main -- crates/lys-log-store/src/log.rs crates/lys-log-store/src/error.rs crates/lys-log-store/src/store.rs prints nothing.
- In a temporary directory, run the command sequence in R3's third acceptance line with the lys binary this branch builds: python3 scripts/verify_inclusion.py exits 0.
- Count the #[test] functions in crates/lys-log-store and crates/lys-anchor on this branch and on main: every test name on main is present on this branch and passes.


---
type: brief
id: LYSLOGSTORE-004
cluster: lys-log-store
title: Open a file leaf store read only: refuse every write and a pending repair, and say what open does and never does
---

# LYSLOGSTORE-004: Open a file leaf store read only: refuse every write and a pending repair, and say what open does and never does

> **Cluster:** lys-log-store
> **Depends on:** LYSLOGSTORE-002
> **Design anchor:**
> - ADR-108 — A file leaf store opened read only refuses every write and a pending repair, decided from the count and the pin alone — FileLeafStore gains an inherent open_read_only. It performs the checks FileLeafStore::open performs, never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and refuses put_leaf and pin through its handle with StoreError::ReadOnly. Whenever the leaves it counts are more than the pinned tree size it refuses with StoreError::RepairPending, deciding from state.json and the count alone, reading no leaf bytes and judging nothing else: the rebuild that tells an interrupted append from a damaged prefix belongs to the writable open, which is the only path that repairs and the only path that can refuse a damaged prefix with PinMismatch. Neither refusal names a leaf index. Rejected: a read-only method on the LeafStore trait, which every second backend would have to meet. Also rejected: a read-only mode for Log or for the anchor, which LYSLOGSTORE-002 keeps out. Also rejected: opening at the pinned head and reporting the repair as pending, which would serve a store past its pin as though it were whole. Also rejected: rebuilding the pinned prefix at read-only open so that a damaged prefix with a leaf past the pin is told PinMismatch, which moves the writable open's judgement into a store that holds no tree.
> - ADR-101 — A file log store's only expectation of its leaves is the pin; a damaged pinned prefix is refused without naming a leaf — The pin is the store's only expectation of its leaves. When the leaves inside the pinned prefix rebuild to another root, Log::open refuses with StoreError::PinMismatch as it does today. That refusal states the pin's tree size and root and the tree size and root recomputed from the leaves, and it never names a leaf index, because which leaf changed is not known. The one-leaf repair for a leaf just past the pin is kept exactly as it is. Rejected: a per-leaf hash record beside the leaves, which changes the layout and BACKENDS.md and adds a crash window. Also rejected: naming a leaf by bisecting the pin's root, which cannot be done. Also rejected: a read-only open of a refused log, and refusing a torn leaf just past the pin, which would turn routine crash recovery into a refusal.
> **Checklist:**
> - C13 — StoreError has two new variants, ReadOnly and RepairPending, neither of which carries or prints a leaf index, and no existing variant's fields or message changes.
> - C14 — FileLeafStore::open_read_only is a documented inherent constructor that refuses what FileLeafStore::open refuses with the same errors, reports the same leftover temporary files, and never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and the LeafStore trait has the same methods as on main.
> - C15 — put_leaf through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and changes no byte of the store.
> - C16 — pin through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly before any other pin check and changes no byte of the store.
> - C17 — FileLeafStore::open_read_only over a store whose leaf count is past its pinned tree size returns StoreError::RepairPending, reads no leaf byte and changes no byte of the store.
> - C18 — A store that FileLeafStore::open_read_only refused with StoreError::RepairPending, opened with FileLeafStore::open and Log::open, is repaired and the repair is reported through recovered_to, as on main.
> - C19 — The module doc of crates/lys-log-store/src/file.rs states what open and open_read_only do and never do, including that open never deletes a leftover temporary file.
> **Stories:**
> - S5 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.
> - S6 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.
> - S7 (Leaf store maintainer, Reads the file store's contract before relying on it or changing it) — As a leaf store maintainer, I want the file store's module doc to say what opening a store does and never does, so that I can rely on open never deleting a leftover temporary file.

## Purpose

A file log store has no open that is guaranteed to leave it as it found it: FileLeafStore::open flushes leaves/, and Log::open repairs an interrupted append by moving the pin. This brief adds FileLeafStore::open_read_only, which never writes and never repairs. Through it put_leaf and pin refuse with StoreError::ReadOnly, and a store whose leaves are past its pin is refused with StoreError::RepairPending instead of being repaired, decided from the count and the pin without reading a leaf (ADR-108). A writable open repairs as before. The file store's module doc says what open and open_read_only do and never do, including that open never deletes a leftover temporary file. The card names conformance row 4.3 as the row it passes in part, in the words "Conformance row 4.3 says the flight recorder's log refuses a leaf it cannot prove whole, and its witness reads only what is new." That is the lead's statement of the row that is owed. docs/design/identity/CONFORMANCE.md, the only conformance table in the tree, has no flight-recorder row, and its row 4.3 is a roles row, so this brief names no row of it as passed and adds none. The design goal of this cluster it serves is the first entry of the goals array in docs/design/lys-log-store/design.json (goals[0]: a store whose pinned prefix holds a torn or changed leaf is refused at open), with principle P1. LYSLOGSTORE-002 is the other part: it carries the refusal of a torn or changed pinned prefix (C8, C9), the one-leaf repair (C10), the leftover-temporary report (C1) and the witness that reads only what is new (C5 to C7). This card makes one change to LYSLOGSTORE-002's text, at the lead's ruling: design.json's solution sentence `No read-only open is added` and its non-goal reason `no read-only handle is added` each keep their words and gain the clause `, for Log and the anchor; FileLeafStore::open_read_only is added by LYSLOGSTORE-004 under ADR-108`, because as LYSLOGSTORE-002 left them they would contradict this brief. No other member of LYSLOGSTORE-002 is changed.

## Task

Start only once LYSLOGSTORE-002 has landed on lys main, which this command, run by anyone, confirms by exiting 0: git clone https://github.com/ablative-io/lys lys-main && git -C lys-main grep -q 'fn leftover_temporaries' origin/main -- crates/lys-log-store/src/file.rs && git -C lys-main cat-file -e origin/main:docs/design/lys-log-store/briefs/LYSLOGSTORE-002.json. Work in crates/lys-log-store. Add StoreError::ReadOnly and StoreError::RepairPending (R1). Add the inherent constructor FileLeafStore::open_read_only, which performs FileLeafStore::open's checks without its flush and writes nothing, and make put_leaf and pin through its handle refuse with ReadOnly before any other check (R2). Make open_read_only refuse with RepairPending whenever the leaves it counts are more than the pinned tree size, deciding from state.json and the count alone, and hold the writable open's repair of the same store under a test (R3). Add a section to file.rs's module doc saying what open and open_read_only do and never do (R4). Cited, not carried: the refusal of a leaf changed by one byte after naming is StoreError::PinMismatch from Log::open, naming no leaf, as main already holds in log::tests::a_tampered_leaf_byte_is_detected_at_open and as LYSLOGSTORE-002 C9 pins; the torn-prefix refusal is LYSLOGSTORE-002 C8; the report of leftover temporary files at open is LYSLOGSTORE-002 C1; the witness is LYSLOGSTORE-002 C5 to C7. This brief adds no test for any of them. The four leaf-write commits already on main (the flushed-then-named leaf, the no-replace link, the sequence parameter and the rename-injection isolation) are not redone. Out of scope: any read-only mode for Log or the anchor, any method on the LeafStore trait, moving the lys log or lys-anchor commands to open_read_only, any per-leaf hash record, and any change to Log's code, StoreError::PinMismatch, state.json, log.json, the leaves/ layout or the leaf file bytes.

## Requirements

### R1: Name the two read-only refusals in StoreError

StoreError gains two variants, each with a /// doc and each field documented. ReadOnly has the fields path: PathBuf, the store's directory, and operation: &'static str, the act refused, and the message `refusing to {operation} in the log store at {path}: it was opened read only`. RepairPending has the fields path: PathBuf, the store's directory, pinned_size: u64, the pinned tree size, and extent: u64, the number of leaves counted, and the message `refusing to open the log store at {path} read only: it holds {extent} leaves but its pin is at tree size {pinned_size}, an interrupted append that only a writable open repairs`. THE SYSTEM SHALL NOT change any existing variant's fields, message or documentation, and SHALL NOT let either new variant carry or print a leaf index.

**Acceptance:**
- `StoreError::ReadOnly { path: PathBuf::from("store"), operation: "write a leaf" }.to_string()` equals `refusing to write a leaf in the log store at store: it was opened read only`.
- `StoreError::RepairPending { path: PathBuf::from("store"), pinned_size: 1, extent: 2 }.to_string()` equals `refusing to open the log store at store read only: it holds 2 leaves but its pin is at tree size 1, an interrupted append that only a writable open repairs`.
- Both assertions above are made by the test file::tests::the_read_only_refusals_name_the_store_and_the_refused_act, and `cargo test -p lys-log-store --all-features --lib file::tests::the_read_only_refusals_name_the_store_and_the_refused_act -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::the_read_only_refusals_name_the_store_and_the_refused_act ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- `git diff origin/main -- crates/lys-log-store/src/error.rs` adds the two variants ReadOnly and RepairPending and shows no removed line.

**Files:**
- modify: crates/lys-log-store/src/error.rs
- modify: crates/lys-log-store/src/file_tests.rs

**Checklist:**
- C13 — StoreError has two new variants, ReadOnly and RepairPending, neither of which carries or prints a leaf index, and no existing variant's fields or message changes.

**Stories:**
- S5 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.
- S6 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.

### R2: Open a store read only and refuse put_leaf and pin through it

FileLeafStore gains a public, documented inherent constructor open_read_only(dir: &Path) -> StoreResult<FileLeafStore>. WHEN open_read_only is called, THE SYSTEM SHALL perform every check FileLeafStore::open performs, with the same errors, SHALL count the named leaves as FileLeafStore::open does, and SHALL return the same leftover_temporaries() FileLeafStore::open returns. It SHALL NOT flush leaves/ or any other directory, and SHALL NOT create, write, rename, link or remove any file, leftover temporary files included. WHEN put_leaf is called through a handle from open_read_only, THE SYSTEM SHALL return StoreError::ReadOnly with path the store's directory and operation `write a leaf` before any other check, SHALL NOT create any file and SHALL NOT change extent(). WHEN pin is called through a handle from open_read_only, THE SYSTEM SHALL return StoreError::ReadOnly with path the store's directory and operation `move the pin` before PinWentBackwards or PinRootChanged is checked, SHALL NOT write state.json or state.json.tmp, and SHALL NOT change pinned(). THE SYSTEM SHALL NOT add a method to the LeafStore trait, and SHALL NOT change what FileLeafStore::open or a handle from it does. file.rs declares, on the line directly after a line `#[cfg(test)]`, a module `probe` holding two per-thread counters the tests read: a flush counter, to which sync_dir adds one per call in both of its cfg variants, so that every directory flush is counted, fsync_dir's included, and to which write_durably adds one per call for file flushes, and a leaf-read counter, to which every read of a leaf file's bytes adds one. Every statement that touches probe carries #[cfg(test)], so no counter exists outside the test configuration.

**Acceptance:**
- A log made at a fresh directory D with origin example.com/log and the leaves `leaf-0` and `leaf-1` appended through Log::append: FileLeafStore::open_read_only(D) returns a handle whose extent() is 2, whose pinned().tree_size is 2 and whose pinned().root equals the log's root after the second append, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::open_read_only_opens_a_store_at_its_pin_with_its_extent_and_pin, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_opens_a_store_at_its_pin_with_its_extent_and_pin -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_opens_a_store_at_its_pin_with_its_extent_and_pin ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The same two-leaf log with a file leaves/.4242-00000000000000000002-0.tmp holding `partial` planted afterwards, opened with FileLeafStore::open_read_only: put_leaf(2, b"leaf-2") returns Err(StoreError::ReadOnly { path: D, operation: "write a leaf" }), extent() is still 2, leaves/.4242-00000000000000000002-0.tmp still holds `partial`, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::put_leaf_through_a_read_only_handle_is_refused_and_changes_no_byte, and `cargo test -p lys-log-store --all-features --lib file::tests::put_leaf_through_a_read_only_handle_is_refused_and_changes_no_byte -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::put_leaf_through_a_read_only_handle_is_refused_and_changes_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Through a handle from FileLeafStore::open_read_only on the same two-leaf log: pin(PinnedRoot { tree_size: 1, root: the log's root after the first append }) returns Err(StoreError::ReadOnly { path: D, operation: "move the pin" }), pin(PinnedRoot { tree_size: 3, root: [0; 32] }) returns the same error, pinned() is unchanged, D/state.json.tmp does not exist, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::pin_through_a_read_only_handle_is_refused_before_any_pin_check_and_changes_no_byte, and `cargo test -p lys-log-store --all-features --lib file::tests::pin_through_a_read_only_handle_is_refused_before_any_pin_check_and_changes_no_byte -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::pin_through_a_read_only_handle_is_refused_before_any_pin_check_and_changes_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- A log made at a fresh directory D with origin example.com/log and the leaves `leaf-0` and `leaf-1` appended through Log::append: the per-thread flush counter is raised by exactly 0 by FileLeafStore::open_read_only(D), and by exactly 1 by FileLeafStore::open(D) afterwards. The test is file::tests::open_read_only_flushes_nothing_where_open_flushes_leaves, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_flushes_nothing_where_open_flushes_leaves -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_flushes_nothing_where_open_flushes_leaves ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Two cases, each opened with both FileLeafStore::open and FileLeafStore::open_read_only, and the test asserts that it checked 2 cases: a fresh empty directory D gives Err(StoreError::NotInitialized { path: D }) from both; a log made at a fresh directory D with `leaf-0` and `leaf-1` appended through Log::append, then leaves/00000000000000000001 renamed to leaves/00000000000000000002, gives Err(StoreError::Corrupt { path: D, reason: "leaves are not contiguous: expected leaf index 1, found 2" }) from both. The test is file::tests::open_read_only_refuses_what_open_refuses_with_the_same_error, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_what_open_refuses_with_the_same_error -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_refuses_what_open_refuses_with_the_same_error ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The two-leaf log with a file leaves/.4242-00000000000000000002-0.tmp holding `partial` planted afterwards: FileLeafStore::open_read_only(D) returns a handle whose leftover_temporaries() equals [".4242-00000000000000000002-0.tmp"], FileLeafStore::open(D) then returns a handle whose leftover_temporaries() equals the same list, and leaves/.4242-00000000000000000002-0.tmp still holds `partial`. The test is file::tests::open_read_only_reports_the_same_leftover_temporaries_as_open, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_reports_the_same_leftover_temporaries_as_open -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_reports_the_same_leftover_temporaries_as_open ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- `grep -B1 '^mod probe {' crates/lys-log-store/src/file.rs` prints exactly the two lines `#[cfg(test)]` and `mod probe {`, and `cargo build -p lys-log-store` and `cargo build -p lys-log-store --all-features` each exit 0, which they cannot while a statement outside the test configuration names probe.
- `git diff origin/main -- crates/lys-log-store/src/store.rs crates/lys-log-store/src/log.rs docs/design/lys-log-store/BACKENDS.md` prints nothing.
- crates/lys-log-store/src/file.rs holds no more than 500 lines that are neither blank nor comment lines, not counting tests.

**Files:**
- modify: crates/lys-log-store/src/file.rs
- modify: crates/lys-log-store/src/file_tests.rs

**Checklist:**
- C14 — FileLeafStore::open_read_only is a documented inherent constructor that refuses what FileLeafStore::open refuses with the same errors, reports the same leftover temporary files, and never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and the LeafStore trait has the same methods as on main.
- C15 — put_leaf through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and changes no byte of the store.
- C16 — pin through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly before any other pin check and changes no byte of the store.

**Stories:**
- S5 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.

### R3: Refuse a store past its pin at read-only open, and keep the writable repair

IF the leaves open_read_only counts are more than the pinned tree size, THEN THE SYSTEM SHALL return StoreError::RepairPending with path the store's directory, pinned_size the pinned tree size and extent the leaves counted, and SHALL NOT return a handle. THE SYSTEM SHALL decide this from state.json and the count alone: it SHALL NOT read any leaf file's bytes, and SHALL NOT judge whether the pinned prefix rebuilds to the pin, which belongs to the writable open. WHEN the leaves counted are at or below the pinned tree size, THE SYSTEM SHALL return the handle; over it Log::open accepts a tree that matches the pin and refuses any other with PinMismatch, without calling pin. Every read of a leaf file's bytes SHALL add one to the leaf-read counter of R2's test-only probe module. WHEN a store that open_read_only refused with RepairPending is opened with FileLeafStore::open and Log::open, THE SYSTEM SHALL repair it and report it through recovered_to as main does. THE SYSTEM SHALL NOT change Log::open, reconcile_with_pin, the one-leaf repair or StoreError::PinMismatch.

**Acceptance:**
- A log made at a fresh directory D with origin example.com/log, `leaf-0` appended through Log::append, state.json's bytes saved, `leaf-1` appended, state.json rewritten with the saved bytes, and a file leaves/.4242-00000000000000000002-0.tmp holding `partial` planted: FileLeafStore::open_read_only(D) returns Err(StoreError::RepairPending { path: D, pinned_size: 1, extent: 2 }), the per-thread leaf-read counter is the same before and after that call, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::open_read_only_refuses_a_store_past_its_pin_reading_no_leaf_and_changing_no_byte, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_a_store_past_its_pin_reading_no_leaf_and_changing_no_byte -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_refuses_a_store_past_its_pin_reading_no_leaf_and_changing_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The same construction with `leaf-1` and `leaf-2` appended after the saved state.json: FileLeafStore::open_read_only(D) returns Err(StoreError::RepairPending { path: D, pinned_size: 1, extent: 3 }), and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::open_read_only_refuses_any_count_past_the_pin_with_repair_pending, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_any_count_past_the_pin_with_repair_pending -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_refuses_any_count_past_the_pin_with_repair_pending ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The store of the first line of this requirement, after open_read_only refused it: Log::open(FileLeafStore::open(D)) raises the leaf-read counter by exactly 2 and returns a log whose recovered_to() is Some(2) and whose tree().len() is 2. A second Log::open(FileLeafStore::open(D)) returns a log whose recovered_to() is None, and FileLeafStore::open_read_only(D) then returns a handle whose extent() is 2 and whose pinned().tree_size is 2. The test is file::tests::the_store_a_read_only_open_refused_is_repaired_by_a_writable_open, and `cargo test -p lys-log-store --all-features --lib file::tests::the_store_a_read_only_open_refused_is_repaired_by_a_writable_open -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::the_store_a_read_only_open_refused_is_repaired_by_a_writable_open ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- A log made at a fresh directory D with `leaf-0` and `leaf-1` appended through Log::append, then leaves/00000000000000000001 removed: FileLeafStore::open_read_only(D) returns a handle whose extent() is 1, Log::open over that handle returns StoreError::PinMismatch with pinned_size 2 and rebuilt_size 1, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::a_read_only_store_below_its_pin_opens_and_log_open_refuses_it, and `cargo test -p lys-log-store --all-features --lib file::tests::a_read_only_store_below_its_pin_opens_and_log_open_refuses_it -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::a_read_only_store_below_its_pin_opens_and_log_open_refuses_it ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- `cargo clippy -p lys-log-store --all-features -- -D warnings` and `cargo clippy -p lys-log-store -- -D warnings`, which build the library without its tests, each finish with exit 0 and no warning.

**Files:**
- modify: crates/lys-log-store/src/file.rs
- modify: crates/lys-log-store/src/file_tests.rs

**Checklist:**
- C17 — FileLeafStore::open_read_only over a store whose leaf count is past its pinned tree size returns StoreError::RepairPending, reads no leaf byte and changes no byte of the store.
- C18 — A store that FileLeafStore::open_read_only refused with StoreError::RepairPending, opened with FileLeafStore::open and Log::open, is repaired and the repair is reported through recovered_to, as on main.

**Stories:**
- S6 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.

### R4: Say in the file store's module doc what open and open_read_only do and never do

The //! module doc of crates/lys-log-store/src/file.rs gains a section headed `# Opening`. It states that FileLeafStore::open reads log.json and state.json, flushes leaves/ before counting, and counts the leaves named 0..n; that it never repairs, since the one-leaf repair is Log::open's, through pin; and that it never deletes, renames or rewrites a leftover temporary file and counts none as a leaf. It states that open_read_only performs the same checks, never flushes, never writes and never repairs, and refuses put_leaf and pin with StoreError::ReadOnly. It states that open_read_only reads no leaf bytes, refuses with StoreError::RepairPending whenever the leaves counted are more than the pin's tree size, and judges nothing else, because telling an interrupted append from a damaged prefix needs the rebuild that the writable open does. It states the trade of skipping the flush: a leaf name linked just before a crash may be counted by open_read_only without a durable directory entry. It states that FileLeafStore::leaf serves a leaf's bytes without checking them, so a leaf is proven whole only when Log::open rebuilds the tree and compares it with the pin, and a leaf changed by one byte is refused there with StoreError::PinMismatch, which names no leaf. The section SHALL NOT say that open deletes, repairs or reports anything it does not, and SHALL NOT say that any refusal names a leaf index.

**Acceptance:**
- crates/lys-log-store/src/file.rs has a line `//! # Opening`.
- The section under that heading contains the sentence `` `open` never deletes a leftover temporary file. ``
- The section contains the sentence `` `open_read_only` never flushes, never writes and never repairs. ``
- The section contains the sentence `` `open_read_only` reads no leaf bytes: whenever the leaves it counts are more than the pin's tree size it refuses with [`StoreError::RepairPending`] and judges nothing else. ``
- The section contains the sentence `` A leaf is proven whole only when [`Log::open`](crate::Log::open) rebuilds the tree from the leaves and compares it with the pin. ``
- `cargo doc --no-deps -p lys-log-store` and `cargo doc --no-deps --all-features -p lys-log-store` each finish with no warning.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C19 — The module doc of crates/lys-log-store/src/file.rs states what open and open_read_only do and never do, including that open never deletes a leftover temporary file.

**Stories:**
- S7 (Leaf store maintainer, Reads the file store's contract before relying on it or changing it) — As a leaf store maintainer, I want the file store's module doc to say what opening a store does and never does, so that I can rely on open never deleting a leftover temporary file.

## Boundaries

- SHALL NOT add a method to the LeafStore trait or change BACKENDS.md.
- SHALL NOT add a read-only mode to Log or to the anchor, and SHALL NOT change Log's code, Log::open, reconcile_with_pin, the one-leaf repair or StoreError::PinMismatch's fields or message.
- SHALL NOT change what FileLeafStore::open or a handle from it does, and SHALL NOT change any existing test's assertions.
- SHALL NOT change put_leaf_with, write_leaf_temp, link_leaf or any other part of the leaf write.
- SHALL NOT add a per-leaf hash record, and SHALL NOT name, guess or bisect for a leaf index in any refusal.
- SHALL NOT change state.json, log.json, the leaves/ layout or the leaf file bytes; lys/log-dir/v1 stores written before this change still open.
- SHALL NOT delete a leftover temporary file or make one an error.
- SHALL NOT move the lys log or lys-anchor commands to open_read_only, and SHALL NOT change any crate but lys-log-store.
- SHALL NOT add a test for the torn-prefix or one-byte refusals or for FileLeafStore::open's own leftover-temporary report, which LYSLOGSTORE-002 carries.
- SHALL NOT let crates/lys-log-store/src/file.rs exceed 500 lines of code, excluding tests, comments and blank lines.

## Verification

- In a fresh directory, `git clone https://github.com/ablative-io/lys lys-main && git -C lys-main grep -q 'fn leftover_temporaries' origin/main -- crates/lys-log-store/src/file.rs && git -C lys-main cat-file -e origin/main:docs/design/lys-log-store/briefs/LYSLOGSTORE-002.json` exits 0 before the build starts.
- cargo fmt --all
- cargo clippy --all-targets --all-features -- -D warnings
- cargo clippy --all-targets -- -D warnings
- cargo test --workspace --all-features
- cargo doc --no-deps --all-features
- cargo doc --no-deps
- sh scripts/design/gate.sh
- Run `cargo test -p lys-log-store --all-features --lib file::tests::the_read_only_refusals_name_the_store_and_the_refused_act -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::the_read_only_refusals_name_the_store_and_the_refused_act ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_opens_a_store_at_its_pin_with_its_extent_and_pin -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_opens_a_store_at_its_pin_with_its_extent_and_pin ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::put_leaf_through_a_read_only_handle_is_refused_and_changes_no_byte -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::put_leaf_through_a_read_only_handle_is_refused_and_changes_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::pin_through_a_read_only_handle_is_refused_before_any_pin_check_and_changes_no_byte -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::pin_through_a_read_only_handle_is_refused_before_any_pin_check_and_changes_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_flushes_nothing_where_open_flushes_leaves -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_flushes_nothing_where_open_flushes_leaves ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_what_open_refuses_with_the_same_error -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_refuses_what_open_refuses_with_the_same_error ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_reports_the_same_leftover_temporaries_as_open -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_reports_the_same_leftover_temporaries_as_open ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_a_store_past_its_pin_reading_no_leaf_and_changing_no_byte -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_refuses_a_store_past_its_pin_reading_no_leaf_and_changing_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_any_count_past_the_pin_with_repair_pending -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_refuses_any_count_past_the_pin_with_repair_pending ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::the_store_a_read_only_open_refused_is_repaired_by_a_writable_open -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::the_store_a_read_only_open_refused_is_repaired_by_a_writable_open ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::a_read_only_store_below_its_pin_opens_and_log_open_refuses_it -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::a_read_only_store_below_its_pin_opens_and_log_open_refuses_it ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The one-byte refusal this brief cites: `cargo test -p lys-log-store --all-features --lib log::tests::a_tampered_leaf_byte_is_detected_at_open -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test log::tests::a_tampered_leaf_byte_is_detected_at_open ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- `git diff origin/main -- crates/lys-log-store/src/store.rs crates/lys-log-store/src/log.rs crates/lys-log-store/src/log_tests.rs docs/design/lys-log-store/BACKENDS.md` prints nothing, and `git diff --stat origin/main -- crates` lists only files under crates/lys-log-store/src.
- Count the lines of crates/lys-log-store/src/file.rs that are neither blank nor comment lines, not counting tests: at most 500.
- Count the #[test] functions in crates/lys-log-store on this branch and on main: every test name on main is present on this branch and passes.
- `grep -B1 '^mod probe {' crates/lys-log-store/src/file.rs` prints exactly the two lines `#[cfg(test)]` and `mod probe {`, and `cargo build -p lys-log-store` and `cargo build -p lys-log-store --all-features` each exit 0.

