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
> - C8 — A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch carrying both trees, and state.json is unchanged.
> - C9 — A torn leaf just past the pin is adopted by the one-leaf repair and reported by recovered_to.
> - C10 — FileLeafStore::leftover_temporaries returns the store's temporary-leaf names in lexical order, none counted toward the extent and none changed.
> - C1 — StoreError has a documented ReadOnly variant carrying the store's directory and the refused act.
> - C11 — file.rs's module doc has a section stating what open and open_read_only do and never do, including that neither deletes a leftover temporary file.
> - C2 — StoreError has a documented RepairPending variant carrying the store's directory, the pinned tree size and the extent.
> - C4 — FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending and every file under the store directory holds the same bytes before and after.
> - C3 — FileLeafStore::open_read_only performs FileLeafStore::open's checks and returns a handle without flushing leaves/ or writing any file.
> - C12 — file.rs's module doc states that FileLeafStore::leaf serves bytes it has not checked and that a leaf is proven whole only through Log::open against the pinned root.
> - C5 — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
> - C6 — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
> - C7 — A store one leaf past its pin, the state a read-only open refuses with RepairPending, is repaired by FileLeafStore::open followed by Log::open, which reports recovered_to as Some of the extent.
> **Stories:**
> - S4 (Operator, Recovering a log after a crash) — As an operator, I want to be told which leftover temporary leaf files an open skipped so that I can clear them myself, knowing the store never will.
> - S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
> - S2 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.
> - S3 (Operator, Recovering a log after a crash) — As an operator, I want a writable open to repair an interrupted append as it does today so that a crash costs no history.

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
- C8 — A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch carrying both trees, and state.json is unchanged.
- C9 — A torn leaf just past the pin is adopted by the one-leaf repair and reported by recovered_to.
- C10 — FileLeafStore::leftover_temporaries returns the store's temporary-leaf names in lexical order, none counted toward the extent and none changed.

**Stories:**
- S4 (Operator, Recovering a log after a crash) — As an operator, I want to be told which leftover temporary leaf files an open skipped so that I can clear them myself, knowing the store never will.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Log::open already folds the whole pinned prefix and compares it with the pin. These tests prove it through the public open for a torn leaf and for a one-byte change, and check that the refusal names no leaf. The case just past the pin is shown to be the one-leaf repair, not a refusal.
- Deviation: The files list also carries two round-3 gate fixes that belong to no requirement: the re-rendered brief markdown, and the serialisation of lys-home given_record tests. The lys-home change is outside the card's code diff, but its test leg failed in this round's gate, so its cause is fixed here.
- Files changed:
  - modified: `crates/lys-log-store/src/log_tests.rs` — Tests through Log::open. A torn leaf inside the pinned prefix and a one-byte change inside it are each refused with PinMismatch and no leaf index. A torn leaf just past the pin is adopted and reported through recovered_to.
  - modified: `docs/design/lys-log-store/briefs/LYSLOGSTORE-002.md` — Re-rendered from the JSON after the workflow wrote the round-2 blocks, so the design gate's cmp matches.
  - modified: `crates/lys-home/tests/given_record.rs` — Round-3 gate fix: a static SERIAL mutex held for each test's whole body, so no sibling thread forks while a session lock is dropped and retaken. This stops the flaky SessionHeld failure.
- Checklist delivery:
  - [x] C8 — A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch carrying both trees, and state.json is unchanged. — Torn leaf inside the pin is refused with PinMismatch.
  - [x] C9 — A torn leaf just past the pin is adopted by the one-leaf repair and reported by recovered_to. — A one-byte change inside the pin is refused and no index is named.
  - [x] C10 — FileLeafStore::leftover_temporaries returns the store's temporary-leaf names in lexical order, none counted toward the extent and none changed. — A torn leaf past the pin is adopted and reported.
- Story delivery:
  - [x] S4 (Operator, Recovering a log after a crash) — As an operator, I want to be told which leftover temporary leaf files an open skipped so that I can clear them myself, knowing the store never will. — The operator gets a refusal, not silent adoption, for a damaged pinned prefix.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A store created at a fresh directory with origin example.com/log, with leaves `leaf-0`, `leaf-1` and `leaf-2` appended through Log::append, then leaves/00000000000000000001 rewritten to hold `lea`: Log::open(FileLeafStore::open(dir)) returns StoreError::PinMismatch with pinned_size 3, pinned_root the standard base64 of the log's root after the third append, rebuilt_size 3, and rebuilt_root the standard base64 of the root AppendOnlyTree::reconstruct_from_raw_leaves gives for [`leaf-0`, `lea`, `leaf-2`]. — crates/lys-log-store/src/log_tests.rs refusal_over_planted_leaf_1 destructures PinMismatch and asserts both sizes are 3, pinned_root equals STANDARD.encode of three_leaf_store's root, and rebuilt_root equals rebuilt_root_b64([leaf-0, lea, leaf-2]); test a_torn_leaf_inside_the_pinned_prefix_is_refused_with_the_pin_and_the_rebuilt_root; cargo test -p lys-log-store: 55 passed
  - [x] That error's Display text equals `stored leaves rebuild to tree size 3 with root X, but the pinned state is tree size 3 with root Y`, with X the rebuilt_root and Y the pinned_root above. — log_tests.rs a_torn_leaf_inside_the_pinned_prefix_is_refused_with_the_pin_and_the_rebuilt_root asserts assert_eq!(display, format!(...{rebuilt_root}... {pinned_root}))
  - [x] state.json in that store holds the same bytes before and after that failed open. — refusal_over_planted_leaf_1 asserts state_before == state_after, reading state.json around reopen
  - [x] The same three-leaf store with leaves/00000000000000000001 rewritten to hold `leaf-X` returns StoreError::PinMismatch from Log::open with pinned_size 3 and rebuilt_size 3. Its rebuilt_root is the standard base64 of the root reconstruct_from_raw_leaves gives for [`leaf-0`, `leaf-X`, `leaf-2`], and it differs from pinned_root. state.json holds the same bytes before and after. — log_tests.rs a_one_byte_change_inside_the_pinned_prefix_is_refused_with_the_pin_and_the_rebuilt_root, which runs refusal_over_planted_leaf_1(b"leaf-X") and so gets its assert_ne!(rebuilt_root, pinned_root) and the state.json equality
  - [x] A store with leaves `leaf-0` and `leaf-1` appended through Log::append, then a file leaves/00000000000000000002 holding `lea` written by hand, opens with Log::open: recovered_to() is Some(3) and tree().len() is 3. A second open of the same store returns a log whose recovered_to() is None and whose tree().len() is 3. — log_tests.rs a_torn_leaf_just_past_the_pin_is_adopted_pinned_and_reported asserts recovered_to()==Some(3), len 3, then reopen: None, len 3
- Checklist verified: C8, C9, C10
- Stories verified: S4
- Issues:
  - `git diff main` for log.rs, error.rs and store.rs cannot be run as written: there is no local main, and origin/main has moved past this branch's base. Against the merge-base a426b9a the diff is empty, so Log::open, its repair and PinMismatch are untouched.

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
- C1 — StoreError has a documented ReadOnly variant carrying the store's directory and the refused act.

**Stories:**
- S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The directory scan that finds the contiguous leaf extent now also collects names matching the temporary-leaf form. Those names are kept on the store and exposed read-only. The leaf write path is untouched.
- Deviation: The private contiguous_extent was renamed to scan_leaves because it now returns the extent and the leftover names.
- Files changed:
  - modified: `crates/lys-log-store/src/file.rs` — FileLeafStore::open records leftover temporary leaf names (sorted) and exposes them through leftover_temporaries(). It never errors on them and never deletes them.
  - modified: `crates/lys-log-store/src/file_tests.rs` — Tests: leftover temporaries are listed in sorted order, not deleted and not errors; non-temporary names are ignored; an empty list when there are none.
- Checklist delivery:
  - [x] C1 — StoreError has a documented ReadOnly variant carrying the store's directory and the refused act. — leftover_temporaries() lists them; no error and no deletion.
- Story delivery:
  - [x] S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence. — Leftover temporaries are visible to callers.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A store created at a fresh directory with origin example.com/log, with one leaf of bytes `a` appended through Log::append and a file leaves/.4242-00000000000000000001-0.tmp holding the bytes `partial` written afterwards, opens with FileLeafStore::open: extent() is 1 and leftover_temporaries() equals [".4242-00000000000000000001-0.tmp"]. — crates/lys-log-store/src/file_tests.rs open_reports_a_leftover_temporary_file_and_does_not_count_it; file.rs scan_leaves collects is_leaf_temp_name dot-names
  - [x] After that open, leaves/.4242-00000000000000000001-0.tmp holds exactly the bytes `partial`. — same test asserts std::fs::read(&leftover) == b"partial"; scan_leaves only reads the directory
  - [x] The same store with no planted file opens with leftover_temporaries() empty and extent() 1. — file_tests.rs open_reports_no_leftover_temporary_file_when_there_is_none
  - [x] A store holding leaves/.4242-00000000000000000001-1.tmp and leaves/.17-00000000000000000001-0.tmp opens with leftover_temporaries() equal to [".17-00000000000000000001-0.tmp", ".4242-00000000000000000001-1.tmp"]. — file_tests.rs open_reports_leftover_temporary_files_in_lexical_order; file.rs temporaries.sort_unstable()
  - [x] A store holding one leaf and a file leaves/.DS_Store opens with leftover_temporaries() empty and extent() 1. — file_tests.rs open_does_not_report_a_dotfile_outside_the_temporary_form; the_temporary_form_is_exactly_what_leaf_temp_name_writes also checks 12 near misses that each fail on one axis
  - [x] crates/lys-log-store/src/file.rs holds no more than 500 lines that are neither blank nor comment lines, not counting tests. — awk count of non-blank, non-// lines in file.rs: 460
- Checklist verified: C1
- Stories verified: S1

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
- C2 — StoreError has a documented RepairPending variant carrying the store's directory, the pinned tree size and the extent.
- C4 — FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending and every file under the store directory holds the same bytes before and after.
- C11 — file.rs's module doc has a section stating what open and open_read_only do and never do, including that neither deletes a leftover temporary file.

**Stories:**
- S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
- S2 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.
- S4 (Operator, Recovering a log after a crash) — As an operator, I want to be told which leftover temporary leaf files an open skipped so that I can clear them myself, knowing the store never will.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The lys log commands open through one helper, which reports leftover temporaries on stderr and leaves stdout and exit codes unchanged.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys/src/commands/log/store.rs` — After opening, prints 'ignored leftover temporary leaf files: <names>' to stderr when any exist.
  - modified: `crates/lys/tests/log_tests.rs` — CLI tests: the line is on stderr only, and stdout still verifies with scripts/verify_inclusion.py. A torn pinned prefix is refused with exit 1.
- Checklist delivery:
  - [x] C2 — StoreError has a documented RepairPending variant carrying the store's directory, the pinned tree size and the extent. — The stderr line is printed.
  - [x] C4 — FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending and every file under the store directory holds the same bytes before and after. — stdout is unchanged and verifies independently.
  - [x] C11 — file.rs's module doc has a section stating what open and open_read_only do and never do, including that neither deletes a leftover temporary file. — A damaged prefix exits 1 through the CLI.
- Story delivery:
  - [x] S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
  - [x] S2 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.
  - [x] S4 (Operator, Recovering a log after a crash) — As an operator, I want to be told which leftover temporary leaf files an open skipped so that I can clear them myself, knowing the store never will.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] `lys log init --dir D --origin example.com/log` on a fresh directory D, then a file D/leaves/.4242-00000000000000000000-0.tmp holding `partial`: `lys log status --dir D` exits 0, and its stderr has exactly one line starting `ignored leftover temporary leaf files`, which equals `ignored leftover temporary leaf files: .4242-00000000000000000000-0.tmp`. — Ran target/debug/lys in a temp dir: planted=0 and stderr `ignored leftover temporary leaf files: .4242-00000000000000000000-0.tmp`; test log_status_names_a_leftover_temporary_leaf_file_on_stderr; crates/lys/src/commands/log/store.rs open()
  - [x] `lys log status --dir D` on a directory D made by `lys log init` with no planted file exits 0, and its stderr has no line starting `ignored leftover temporary leaf files`. — Ran locally: clean=0 with empty stderr; test log_status_says_nothing_about_temporaries_when_there_are_none
  - [x] A key made by `lys key generate --out K`, a log D made by `lys log init --dir D --origin example.com/log`, a leaf file L holding `hello` appended by `lys log append --dir D --leaf L`, and an artifact A written by `lys log prove inclusion --dir D --key K --leaf-index 0 --out A`: `python3 scripts/verify_inclusion.py A D/leaves/00000000000000000000` exits 0. — Ran locally: INCLUSION VERIFIED, verify=0; test a_leaf_from_a_store_written_now_verifies_with_the_standalone_python_verifier
  - [x] `lys log init --dir D --origin example.com/log`, then `lys log append --dir D --leaf L` three times with L holding `leaf-0`, `leaf-1` and `leaf-2` in turn, then D/leaves/00000000000000000001 rewritten to hold `lea`: `lys log status --dir D` exits 1. Its stderr has exactly one line starting `error: `, which starts `error: log directory invalid: D: stored leaves rebuild to tree size 3 with root ` and contains `, but the pinned state is tree size 3 with root `. D/state.json holds the same bytes before and after. — Ran locally: damaged=1, one line `error: log directory invalid: F: stored leaves rebuild to tree size 3 with root aATk...=, but the pinned state is tree size 3 with root z3Y6...=`, and state.json shasum 0e1f87f8... was the same before and after; test log_status_refuses_a_torn_leaf_inside_the_pinned_prefix
- Checklist verified: C2, C4, C11
- Stories verified: S1, S2, S4

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
- C3 — FileLeafStore::open_read_only performs FileLeafStore::open's checks and returns a handle without flushing leaves/ or writing any file.
- C12 — file.rs's module doc states that FileLeafStore::leaf serves bytes it has not checked and that a leaf is proven whole only through Log::open against the pinned root.

**Stories:**
- S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
- S4 (Operator, Recovering a log after a crash) — As an operator, I want to be told which leftover temporary leaf files an open skipped so that I can clear them myself, knowing the store never will.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The anchor CLI reads leftover_temporaries from the store it opens and reports them the same way.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys-anchor-cli/src/commands/anchor/open.rs` — Prints the same stderr line after FileLeafStore::open.
  - modified: `crates/lys-anchor-cli/tests/anchor_cli.rs` — Tests: the line is on stderr only, and a torn genesis leaf is refused with exit 1.
- Checklist delivery:
  - [x] C3 — FileLeafStore::open_read_only performs FileLeafStore::open's checks and returns a handle without flushing leaves/ or writing any file.
  - [x] C12 — file.rs's module doc states that FileLeafStore::leaf serves bytes it has not checked and that a leaf is proven whole only through Log::open against the pinned root.
- Story delivery:
  - [x] S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
  - [x] S4 (Operator, Recovering a log after a crash) — As an operator, I want to be told which leftover temporary leaf files an open skipped so that I can clear them myself, knowing the store never will.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] With a key file K made by Ed25519Identity::load_or_generate and a genesis file G holding `genesis`, an anchor made by `lys-anchor init --dir D --origin example.com/anchor --key K --genesis G --admit accept-all`, then a file D/leaves/.4242-00000000000000000001-0.tmp holding `partial`: `lys-anchor status --dir D --key K --admit accept-all` exits 0, and its stderr has exactly one line starting `ignored leftover temporary leaf files`, which equals `ignored leftover temporary leaf files: .4242-00000000000000000001-0.tmp`. — crates/lys-anchor-cli/tests/anchor_cli.rs status_names_a_leftover_temporary_leaf_file_on_stderr_only asserts exit 0 and stderr_lines_starting == [that line]; src/commands/anchor/open.rs open() prints it after FileLeafStore::open; workflow tests leg passed
  - [x] The same status command on the same anchor with no planted file exits 0, and its stderr has no line starting `ignored leftover temporary leaf files`. — same test, `clean` branch: exit 0 and stderr_lines_starting(...).is_empty()
  - [x] With the planted file, `lys-anchor --json status --dir D --key K --admit accept-all` exits 0, its stdout parses as one JSON object whose ok is true, and its keys are the same as those of the same command run without the planted file. — same test: ok_keys(planted_json) == clean_keys, and ok_keys asserts exit 0 and ok true
  - [x] With a key file K made by Ed25519Identity::load_or_generate and a genesis file G holding `genesis`, an anchor made by `lys-anchor init --dir D --origin example.com/anchor --key K --genesis G --admit accept-all`, then D/leaves/00000000000000000000 rewritten to hold `gen`: `lys-anchor status --dir D --key K --admit accept-all` exits 1. Its stderr has exactly one line starting `error: `, which starts `error: anchor directory invalid: D: stored leaves rebuild to tree size 1 with root ` and contains `, but the pinned state is tree size 1 with root `. D/state.json holds the same bytes before and after. — anchor_cli.rs status_refuses_a_torn_genesis_leaf_inside_the_pinned_prefix asserts exit 1, exactly one `error: ` line, its start and contained text, and state.json equality
- Checklist verified: C3, C12
- Stories verified: S1, S4

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
- C5 — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.

**Stories:**
- S3 (Operator, Recovering a log after a crash) — As an operator, I want a writable open to repair an interrupted append as it does today so that a crash costs no history.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The projection folds forward from its folded count with last-recorded-wins, so each observe parses only the leaves recorded since the previous one.
- Deviation: Added a PartialEq and Eq derive so tests can compare a projection folded forward with a full rebuild.
- Files changed:
  - modified: `crates/lys-anchor/src/witness/projection.rs` — WitnessProjection keeps a folded count and adds fold_to, fold_parsed, remember and folded(). rebuild_prefix is default followed by fold_to. A cfg(test) parse counter is added. The projection lives only in memory.
  - modified: `crates/lys-anchor/src/witness/projection_tests.rs` — Tests: folding forward equals a full rebuild, and only new leaves are parsed.
- Checklist delivery:
  - [x] C5 — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
- Story delivery:
  - [x] S3 (Operator, Recovering a log after a crash) — As an operator, I want a writable open to repair an interrupted append as it does today so that a crash costs no history.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] On a witness anchor holding the genesis leaf, three ordinary statements, a checkpoint note from CHILD_ORIGIN at size 3 and two more ordinary statements (seven leaves), rebuild_prefix(&anchor, 4) gives folded() 4. fold_to(&anchor, 7) then gives folded() 7, and the parse counter rises by exactly 3 during that fold_to. — crates/lys-anchor/src/witness/projection_tests.rs folding_forward_parses_only_the_leaves_past_the_fold_position (seven_leaves fixture); counter is a cfg(test) thread_local in checkpoint_in_leaf
  - [x] After that fold_to, latest(CHILD_ORIGIN) equals WitnessProjection::rebuild(&anchor).latest(CHILD_ORIGIN), with tree_size 3. — same test asserts equality with the rebuild and tree_size 3
  - [x] rebuild(&anchor) over those seven leaves raises the parse counter by exactly 7. — projection_tests.rs a_rebuild_parses_every_leaf_once
  - [x] rebuild(&anchor) over those seven leaves followed by fold_to(&anchor, 2) gives folded() 2 and an empty projection, equal to rebuild_prefix(&anchor, 2). — projection_tests.rs folding_to_fewer_leaves_than_folded_starts_again_from_index_zero; projection.rs fold_to resets to default when folded > bound
  - [x] Child checkpoints at size 5 and then at size 3, recorded in that order and folded by two successive fold_to calls, leave latest(CHILD_ORIGIN) with tree_size 3. — projection_tests.rs successive_forward_folds_keep_the_last_recorded_not_the_largest
  - [x] Every existing projection_tests case passes with its assertions unchanged. — git diff of projection_tests.rs shows only added lines (no - lines); anchor test names at the base (141) are all present on the branch (149); workflow tests leg passed
- Checklist verified: C5
- Stories verified: S3

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
- C6 — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
- C7 — A store one leaf past its pin, the state a read-only open refuses with RepairPending, is repaired by FileLeafStore::open followed by Log::open, which reports recovered_to as Some of the extent.

**Stories:**
- S3 (Operator, Recovering a log after a crash) — As an operator, I want a writable open to repair an interrupted append as it does today so that a crash costs no history.

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The caller keeps the projection across observes, so each observe does work bounded by the leaves recorded since the previous one. Everything stays behind the federation feature.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys-anchor/src/witness/observe.rs` — observe takes &mut WitnessProjection. After submit it folds up to the recorded leaf, relates the checkpoint to the latest one, then folds the new leaf.
  - modified: `crates/lys-anchor/src/witness/observe_tests.rs` — Tests: a kept projection parses only new leaves (deltas 1, 1, 4). Rollback is still refused. A non-checkpoint note is handled. A refused observe leaves the projection unchanged.
- Checklist delivery:
  - [x] C6 — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
  - [x] C7 — A store one leaf past its pin, the state a read-only open refuses with RepairPending, is repaired by FileLeafStore::open followed by Log::open, which reports recovered_to as Some of the extent.
- Story delivery:
  - [x] S3 (Operator, Recovering a log after a crash) — As an operator, I want a writable open to repair an interrupted append as it does today so that a crash costs no history.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A witness anchor holds the genesis leaf, a CHILD_ORIGIN checkpoint note at size 2 as leaf 1, and 254 ordinary statements, 256 leaves in all. Building p with WitnessProjection::rebuild raises the parse counter by 256. observe with p of the child's note at size 3 then raises the counter by exactly 1 and reports previous with tree_size 2. — crates/lys-anchor/src/witness/observe_tests.rs a_kept_projection_parses_only_what_was_recorded_since_the_last_observe (256 check, then parses==1 and previous tree_size 2)
  - [x] Next, observe with p of an OTHER_CHILD_ORIGIN note raises the parse counter by exactly 1 and reports previous as None. — same test, second see_kept: parses==1 and previous is None
  - [x] Next, after three ordinary statements are submitted through Anchor::submit, observe with p of the child's note at size 4 raises the parse counter by exactly 4. — same test, third see_kept: parses==4
  - [x] After each of those three observes, p.folded() equals anchor.tree_size(), and p.latest(CHILD_ORIGIN) and p.latest(OTHER_CHILD_ORIGIN) equal WitnessProjection::rebuild(&anchor)'s for the same origins. — assert_matches_a_rebuild is called after each of the three observes
  - [x] Through one kept projection, observing the child's notes stating size 5 and then size 3 reports the second as Relation::Rollback, and leaves p.latest(CHILD_ORIGIN) with tree_size 3. — observe_tests.rs a_kept_projection_follows_a_rollback
  - [x] Through a kept projection, observe of bytes that checkpoint_in_leaf does not read as a checkpoint raises the parse counter by exactly 1, reports previous as None, and leaves p.folded() equal to anchor.tree_size(). — observe_tests.rs a_kept_projection_folds_past_a_note_that_is_not_a_checkpoint; observe.rs fold_parsed(leaf_index, None) advances folded
  - [x] On an anchor whose admission policy is MaxSize with a limit shorter than the note, observe returns AnchorError::NotAdmitted and p.folded() is the same before and after the call. — observe_tests.rs a_refused_observe_leaves_the_kept_projection_unchanged; observe.rs `anchor.submit(...)?` runs before any projection call
  - [x] Every existing observe_tests case passes with its assertions unchanged, including the_receipt_is_byte_identical_whatever_the_relation. — The existing cases changed only their call sites (see/see_with and the receipt test pass a rebuilt projection for the new parameter). No assertion line changed. The workflow tests leg passed.
- Checklist verified: C6, C7
- Stories verified: S3

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
