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
> - C14 — FileLeafStore::open_read_only is a documented inherent constructor that never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and the LeafStore trait has the same methods as on main.
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

A file log store has no open that is guaranteed to leave it as it found it: FileLeafStore::open flushes leaves/, and Log::open repairs an interrupted append by moving the pin. This brief adds FileLeafStore::open_read_only, which never writes and never repairs. Through it put_leaf and pin refuse with StoreError::ReadOnly, and a store whose leaves are past its pin is refused with StoreError::RepairPending instead of being repaired, decided from the count and the pin without reading a leaf (ADR-108). A writable open repairs as before. The file store's module doc says what open and open_read_only do and never do, including that open never deletes a leftover temporary file. The card names conformance row 4.3 as the row it passes in part, in the words "Conformance row 4.3 says the flight recorder's log refuses a leaf it cannot prove whole, and its witness reads only what is new." That is the lead's statement of the row that is owed. docs/design/identity/CONFORMANCE.md, the only conformance table in the tree, has no flight-recorder row, and its row 4.3 is a roles row, so this brief names no row of it as passed and adds none. The design goal of this cluster it serves is the first entry of the goals array in docs/design/lys-log-store/design.json (goals[0]: a store whose pinned prefix holds a torn or changed leaf is refused at open), with principle P1. LYSLOGSTORE-002 is the other part: it carries the refusal of a torn or changed pinned prefix (C8, C9), the one-leaf repair (C10), the leftover-temporary report (C1) and the witness that reads only what is new (C5 to C7).

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

FileLeafStore gains a public, documented inherent constructor open_read_only(dir: &Path) -> StoreResult<FileLeafStore>. WHEN open_read_only is called, THE SYSTEM SHALL perform every check FileLeafStore::open performs, with the same errors, SHALL count the named leaves as FileLeafStore::open does, and SHALL return the same leftover_temporaries() FileLeafStore::open returns. It SHALL NOT flush leaves/ or any other directory, and SHALL NOT create, write, rename, link or remove any file, leftover temporary files included. WHEN put_leaf is called through a handle from open_read_only, THE SYSTEM SHALL return StoreError::ReadOnly with path the store's directory and operation `write a leaf` before any other check, SHALL NOT create any file and SHALL NOT change extent(). WHEN pin is called through a handle from open_read_only, THE SYSTEM SHALL return StoreError::ReadOnly with path the store's directory and operation `move the pin` before PinWentBackwards or PinRootChanged is checked, SHALL NOT write state.json or state.json.tmp, and SHALL NOT change pinned(). THE SYSTEM SHALL NOT add a method to the LeafStore trait, and SHALL NOT change what FileLeafStore::open or a handle from it does.

**Acceptance:**
- A log made at a fresh directory D with origin example.com/log and the leaves `leaf-0` and `leaf-1` appended through Log::append: FileLeafStore::open_read_only(D) returns a handle whose extent() is 2, whose pinned().tree_size is 2 and whose pinned().root equals the log's root after the second append, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::open_read_only_opens_a_store_at_its_pin_with_its_extent_and_pin, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_opens_a_store_at_its_pin_with_its_extent_and_pin -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_opens_a_store_at_its_pin_with_its_extent_and_pin ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The same two-leaf log with a file leaves/.4242-00000000000000000002-0.tmp holding `partial` planted afterwards, opened with FileLeafStore::open_read_only: put_leaf(2, b"leaf-2") returns Err(StoreError::ReadOnly { path: D, operation: "write a leaf" }), extent() is still 2, leaves/.4242-00000000000000000002-0.tmp still holds `partial`, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::put_leaf_through_a_read_only_handle_is_refused_and_changes_no_byte, and `cargo test -p lys-log-store --all-features --lib file::tests::put_leaf_through_a_read_only_handle_is_refused_and_changes_no_byte -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::put_leaf_through_a_read_only_handle_is_refused_and_changes_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Through a handle from FileLeafStore::open_read_only on the same two-leaf log: pin(PinnedRoot { tree_size: 1, root: the log's root after the first append }) returns Err(StoreError::ReadOnly { path: D, operation: "move the pin" }), pin(PinnedRoot { tree_size: 3, root: [0; 32] }) returns the same error, pinned() is unchanged, D/state.json.tmp does not exist, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::pin_through_a_read_only_handle_is_refused_before_any_pin_check_and_changes_no_byte, and `cargo test -p lys-log-store --all-features --lib file::tests::pin_through_a_read_only_handle_is_refused_before_any_pin_check_and_changes_no_byte -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::pin_through_a_read_only_handle_is_refused_before_any_pin_check_and_changes_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Read in crates/lys-log-store/src/file.rs, no call path from FileLeafStore::open_read_only reaches any of fsync_dir, sync_dir, File::sync_all, write_state and write_durably.
- `git diff origin/main -- crates/lys-log-store/src/store.rs crates/lys-log-store/src/log.rs docs/design/lys-log-store/BACKENDS.md` prints nothing.
- crates/lys-log-store/src/file.rs holds no more than 500 lines that are neither blank nor comment lines, not counting tests.

**Files:**
- modify: crates/lys-log-store/src/file.rs
- modify: crates/lys-log-store/src/file_tests.rs

**Checklist:**
- C14 — FileLeafStore::open_read_only is a documented inherent constructor that never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and the LeafStore trait has the same methods as on main.
- C15 — put_leaf through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and changes no byte of the store.
- C16 — pin through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly before any other pin check and changes no byte of the store.

**Stories:**
- S5 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.

### R3: Refuse a store past its pin at read-only open, and keep the writable repair

IF the leaves open_read_only counts are more than the pinned tree size, THEN THE SYSTEM SHALL return StoreError::RepairPending with path the store's directory, pinned_size the pinned tree size and extent the leaves counted, and SHALL NOT return a handle. THE SYSTEM SHALL decide this from state.json and the count alone: it SHALL NOT read any leaf file's bytes, and SHALL NOT judge whether the pinned prefix rebuilds to the pin, which belongs to the writable open. WHEN the leaves counted are at or below the pinned tree size, THE SYSTEM SHALL return the handle; over it Log::open accepts a tree that matches the pin and refuses any other with PinMismatch, without calling pin. Under the test configuration only, every read of a leaf file's bytes in crates/lys-log-store/src/file.rs SHALL add one to a per-thread counter the tests read; no counter SHALL exist outside the test configuration. WHEN a store that open_read_only refused with RepairPending is opened with FileLeafStore::open and Log::open, THE SYSTEM SHALL repair it and report it through recovered_to as main does. THE SYSTEM SHALL NOT change Log::open, reconcile_with_pin, the one-leaf repair or StoreError::PinMismatch.

**Acceptance:**
- A log made at a fresh directory D with origin example.com/log, `leaf-0` appended through Log::append, state.json's bytes saved, `leaf-1` appended, state.json rewritten with the saved bytes, and a file leaves/.4242-00000000000000000002-0.tmp holding `partial` planted: FileLeafStore::open_read_only(D) returns Err(StoreError::RepairPending { path: D, pinned_size: 1, extent: 2 }), the per-thread leaf-read counter is the same before and after that call, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::open_read_only_refuses_a_store_past_its_pin_reading_no_leaf_and_changing_no_byte, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_a_store_past_its_pin_reading_no_leaf_and_changing_no_byte -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_refuses_a_store_past_its_pin_reading_no_leaf_and_changing_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The same construction with `leaf-1` and `leaf-2` appended after the saved state.json: FileLeafStore::open_read_only(D) returns Err(StoreError::RepairPending { path: D, pinned_size: 1, extent: 3 }), and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::open_read_only_refuses_any_count_past_the_pin_with_repair_pending, and `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_any_count_past_the_pin_with_repair_pending -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_refuses_any_count_past_the_pin_with_repair_pending ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The store of the first line of this requirement, after open_read_only refused it: Log::open(FileLeafStore::open(D)) raises the leaf-read counter by exactly 2 and returns a log whose recovered_to() is Some(2) and whose tree().len() is 2. A second Log::open(FileLeafStore::open(D)) returns a log whose recovered_to() is None, and FileLeafStore::open_read_only(D) then returns a handle whose extent() is 2 and whose pinned().tree_size is 2. The test is file::tests::the_store_a_read_only_open_refused_is_repaired_by_a_writable_open, and `cargo test -p lys-log-store --all-features --lib file::tests::the_store_a_read_only_open_refused_is_repaired_by_a_writable_open -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::the_store_a_read_only_open_refused_is_repaired_by_a_writable_open ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- A log made at a fresh directory D with `leaf-0` and `leaf-1` appended through Log::append, then leaves/00000000000000000001 removed: FileLeafStore::open_read_only(D) returns a handle whose extent() is 1, Log::open over that handle returns StoreError::PinMismatch with pinned_size 2 and rebuilt_size 1, and every file under the store directory, dot-prefixed names included, holds the same bytes before and after, and no file is added or removed. The test is file::tests::a_read_only_store_below_its_pin_opens_and_log_open_refuses_it, and `cargo test -p lys-log-store --all-features --lib file::tests::a_read_only_store_below_its_pin_opens_and_log_open_refuses_it -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::a_read_only_store_below_its_pin_opens_and_log_open_refuses_it ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Read in crates/lys-log-store/src/file.rs, the leaf-read counter and every statement that touches it carry #[cfg(test)], and `cargo clippy -p lys-log-store --all-features -- -D warnings`, which builds the library without its tests, finishes with no warning.

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
- SHALL NOT add a test for the torn-prefix, one-byte or leftover-temporary refusals, which LYSLOGSTORE-002 carries.
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
- Run `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_a_store_past_its_pin_reading_no_leaf_and_changing_no_byte -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_refuses_a_store_past_its_pin_reading_no_leaf_and_changing_no_byte ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::open_read_only_refuses_any_count_past_the_pin_with_repair_pending -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::open_read_only_refuses_any_count_past_the_pin_with_repair_pending ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::the_store_a_read_only_open_refused_is_repaired_by_a_writable_open -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::the_store_a_read_only_open_refused_is_repaired_by_a_writable_open ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- Run `cargo test -p lys-log-store --all-features --lib file::tests::a_read_only_store_below_its_pin_opens_and_log_open_refuses_it -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test file::tests::a_read_only_store_below_its_pin_opens_and_log_open_refuses_it ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- The one-byte refusal this brief cites: `cargo test -p lys-log-store --all-features --lib log::tests::a_tampered_leaf_byte_is_detected_at_open -- --exact`, run in a clone of https://github.com/ablative-io/lys at main with this brief landed, prints `test log::tests::a_tampered_leaf_byte_is_detected_at_open ... ok` and a line starting `test result: ok. 1 passed; 0 failed`.
- `git diff origin/main -- crates/lys-log-store/src/store.rs crates/lys-log-store/src/log.rs crates/lys-log-store/src/log_tests.rs docs/design/lys-log-store/BACKENDS.md` prints nothing, and `git diff --stat origin/main -- crates` lists only files under crates/lys-log-store/src.
- Count the lines of crates/lys-log-store/src/file.rs that are neither blank nor comment lines, not counting tests: at most 500.
- Count the #[test] functions in crates/lys-log-store on this branch and on main: every test name on main is present on this branch and passes.
