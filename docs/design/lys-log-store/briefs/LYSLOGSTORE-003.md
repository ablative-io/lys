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
> - S5 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.
> - S6 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.

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
- S5 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.
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
- S6 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.

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
