---
type: brief
id: LYSLOGSTORE-001
cluster: lys-log-store
title: Give a leaf its final name only after it is whole and flushed
---

# LYSLOGSTORE-001: Give a leaf its final name only after it is whole and flushed

> **Cluster:** lys-log-store
> **Design anchor:**
> - ADR-059 — A named leaf in the file store is a whole, flushed leaf, and the no-replace link is its commit point — A file under a 20-digit leaf name is always a whole, flushed leaf; only dot-prefixed temporary files in leaves/ may be partial. A leaf is written and flushed in a hidden temporary file, then linked to its final name by an operation that refuses to replace, then the temporary name is removed and leaves/ is flushed. The successful link is the commit point: the extent advances past the leaf whatever happens next. A failed temporary-name removal still returns Ok; a failed leaves/ flush returns StoreError::LeafDurabilityUncertain carrying the index, and the handle refuses further appends with it until reopened. A writable open flushes leaves/ before counting and fails with Io when that flush fails; a read-only open (FileLeafStore::open_read_only) counts the named leaves without flushing; neither counts, deletes or changes a leftover temporary file. Rejected: naming the leaf first and writing into it (today's torn-leaf shape); deleting leftover temporary files at open, which would make a read-only open mutate the store; flushing at a read-only open, which would turn a status on read-only media into an error; and reporting a post-link failure as Io, which left the extent behind and made the next append report the store's own leaf as LeafAlreadyWritten.
> - ADR-060 — LeafAlreadyWritten means another writer holds the index, and the store never reports its own leaf as another writer's — LeafAlreadyWritten is returned only when the index is behind the extent or the no-replace link finds the final name taken by an entry this call did not link. Once this call's link succeeds the extent advances past the leaf whatever happens next, so no later step of the same call and no later call on the same handle reports that leaf as LeafAlreadyWritten; a post-link failure that leaves durability in doubt is LeafDurabilityUncertain (ADR-059). Rejected: reporting a post-link failure as Io and leaving the extent behind, which made the next append report the store's own leaf as LeafAlreadyWritten; and treating LeafAlreadyWritten as a resume signal a caller may retry through.
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
> - C13 — StoreError has two new variants, ReadOnly and RepairPending, neither of which carries or prints a leaf index, and no existing variant's fields or message changes.
> - C14 — FileLeafStore::open_read_only is a documented inherent constructor that refuses what FileLeafStore::open refuses with the same errors, reports the same leftover temporary files, and never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and the LeafStore trait has the same methods as on main.
> - C15 — put_leaf through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and changes no byte of the store.
> - C16 — pin through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly before any other pin check and changes no byte of the store.
> - C17 — FileLeafStore::open_read_only over a store whose leaf count is past its pinned tree size returns StoreError::RepairPending, reads no leaf byte and changes no byte of the store.
> - C18 — A store that FileLeafStore::open_read_only refused with StoreError::RepairPending, opened with FileLeafStore::open and Log::open, is repaired and the repair is reported through recovered_to, as on main.
> - C19 — The module doc of crates/lys-log-store/src/file.rs states what open and open_read_only do and never do, including that open never deletes a leftover temporary file.
> **Stories:**
> - S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
> - S2 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.
> - S3 (Witness operator, Runs a witness anchor that observes other logs' checkpoints) — As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.
> - S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.
> - S5 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.
> - S6 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.
> - S7 (Leaf store maintainer, Reads the file store's contract before relying on it or changing it) — As a leaf store maintainer, I want the file store's module doc to say what opening a store does and never does, so that I can rely on open never deleting a leftover temporary file.

## Purpose

FileLeafStore today names a leaf before writing it, so a crash or a failed write leaves a torn leaf that the next open counts and Log::open pins, and a failure after the leaf is named makes the store report its own leaf as another writer's. This brief makes a named leaf mean a whole, flushed leaf (ADR-059): each leaf is written and flushed in a hidden temporary file, linked into place without replacing anything, and the leaves directory is flushed; a writable open flushes leaves/ before counting, a read-only open counts without flushing and returns a handle that refuses to write a leaf or a pin by name, and neither counts or touches a leftover. Because a pin is a write, a read-only open never repairs a store found one leaf ahead of its pin: Anchor::open_read_only opens at the pinned head and says in words that one leaf stands ahead of the pin and that a writable open repairs it, and only Log::open through a writable handle performs that repair. LeafAlreadyWritten keeps meaning another writer only (ADR-060). DIRECTORY-003 lands after this brief, because its uncertain append settles by reading back a leaf it must be able to trust as committed, and this brief records that wait in DIRECTORY-003's depends_on.

## Task

Change crates/lys-log-store/src/file.rs so put_leaf writes through a hidden temporary file named by leaf_temp_name and created by create_leaf_temp without replacing anything, from a sequence supplied to put_leaf_with, flushes it, gives it its final name with std::fs::hard_link, advances the extent at that link, removes the temporary name and flushes leaves/, and removes its own temporary file whenever it fails before a successful link (R2 to R4). Add StoreError::LeafDurabilityUncertain, carrying the index and the failed flush's std::io::Error as its source, in error.rs and name it in LeafStore::put_leaf's docs (R1). Make FileLeafStore::open flush leaves/ before counting and fail with Io when that flush fails, and add FileLeafStore::open_read_only, which performs open's checks without the flush and returns a handle whose put_leaf and pin are refused as the new StoreError::ReadOnly (R5). Write the invariant into file.rs's module docs (R6). Add a cfg(test) fault seam (R7) and the tests with their drift table (R8). Add LYSLOGSTORE-001 to DIRECTORY-003's depends_on and re-render its markdown (R9). Make Anchor::open_read_only's doc examples and lys-anchor's read-only tests open their store with FileLeafStore::open_read_only (R10). Add Log::open_at_pin, which opens a store found one leaf ahead of its pin at the pinned head without pinning, reports the pending repair and, while it is pending, refuses every append by name with the new StoreError::RepairPending, never Poisoned (R11). Make Anchor::open_read_only open its log with Log::open_at_pin, and give AnchorStatus the pending repair and a notice that says it in words (R12). Render the cluster's DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSLOGSTORE-001.md from their JSON (R13). Order: R1 first, since R4 returns its variant; R7 after the steps it fails exist; R8 after R1 to R7; R10 after R5, which adds the constructor it calls; R11 after R5, whose pin refusal it must not reach; R12 after R10 and R11; R13 last, since it renders the documents as they stand when the rest is done.

In: error.rs's three new variants, store.rs's put_leaf doc line, file.rs, file_tests.rs, log.rs's Log::open_at_pin and Log::pending_repair with their test in log_tests.rs, the three lys-anchor files that open a store for Anchor::open_read_only (crates/lys-anchor/src/anchor/read_only.rs, crates/lys-anchor/src/anchor/read_only_tests.rs and crates/lys-anchor/tests/standalone_is_complete.rs), Anchor::open_read_only's log opening and docs in read_only.rs, AnchorStatus's pending repair in crates/lys-anchor/src/anchor/status.rs, DIRECTORY-003's depends_on with its rendered markdown, and the lys-log-store cluster's rendered DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSLOGSTORE-001.md.

Out: clearing leftover hidden temporary files that a crash stranded, which is a further unit not written here: an explicit maintenance act that runs only while holding the write lock. Open never deletes anything. Also out: any change to Log::open, Log::append or Log::open's one-leaf-ahead repair, to the LeafStore trait's methods, to the on-disk layout, to write_state's state.json.tmp, to any crate other than lys-log-store and lys-anchor (the lys CLI and lys-anchor-cli keep calling FileLeafStore::open and render nothing new), to any lys-anchor file other than the four named above, and to any field of DIRECTORY-003 other than depends_on (DIRECTORY-003's uncertain append treats LeafDurabilityUncertain as uncertain and settles by reading the leaf back, in its own card).

The refusal of a second writer is unchanged in meaning: it is still StoreError::LeafAlreadyWritten, now raised when the no-replace link finds the final name taken rather than by create_new.

## Requirements

### R1: Name the uncertain-durability error

Add to StoreError a variant LeafDurabilityUncertain with two fields: index: u64, and source: std::io::Error marked as the error's source, holding the error from the failed flush of the leaves directory. It is documented as: the leaf at index is written under its final name, but flushing the leaves directory afterwards failed, so its durability is uncertain; reopen the store, which flushes the directory and counts the leaf. Its message SHALL be `leaf {index} was written, but the leaves directory could not be flushed afterwards, so its durability is uncertain; reopen the store: {source}`, naming both the index and the source error so the cause of the failed flush is never lost, and telling the caller to reopen the store. It is a new variant alongside the others: Io and LeafAlreadyWritten keep their existing fields, messages and documentation, and LeafAlreadyWritten continues to mean only that another writer holds the index. LeafStore::put_leaf's `# Errors` section in store.rs gains one line naming the new variant as the answer when a leaf was stored but its durability could not be confirmed. The LeafStore trait's methods and signatures SHALL NOT change. No existing StoreError variant SHALL be renamed, removed, or changed in its fields, message or documentation, and the only variants this brief adds are the three its requirements name: LeafDurabilityUncertain (R1), ReadOnly (R5) and RepairPending (R11).

**Acceptance:**
- Test `the_uncertain_error_names_its_index_its_cause_and_the_reopen` in crates/lys-log-store/src/file_tests.rs: `StoreError::LeafDurabilityUncertain { index: 7340033, source: std::io::Error::other("flush failed") }` constructs, and matching it against `StoreError::Io { .. }` and `StoreError::LeafAlreadyWritten { .. }` fails for both.
- In the same test, `to_string()` of that value equals `leaf 7340033 was written, but the leaves directory could not be flushed afterwards, so its durability is uncertain; reopen the store: flush failed`, so it contains the substring `7340033`, the substring `flush failed` and the substring `reopen`; and `std::error::Error::source` on it returns an error whose `to_string()` is `flush failed`.
- The `# Errors` section of `LeafStore::put_leaf` in crates/lys-log-store/src/store.rs contains an intra-doc link to `StoreError::LeafDurabilityUncertain`, and `cargo doc --no-deps` and `cargo doc --no-deps --all-features` both report zero warnings.
- `git diff` of crates/lys-log-store/src/store.rs touches no line outside doc comments.
- `git diff` of crates/lys-log-store/src/error.rs removes no line, and the StoreError variants it adds are exactly three: `LeafDurabilityUncertain`, `ReadOnly` and `RepairPending`.

**Files:**
- modify: crates/lys-log-store/src/error.rs
- modify: crates/lys-log-store/src/store.rs

**Checklist:**
- C1 — StoreError has a documented ReadOnly variant carrying the store's directory and the refused act.

**Stories:**
- S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

### R2: Write and flush each leaf in a hidden, uniquely named temporary file before it has a name

WHEN put_leaf is called with the next free index, THE SYSTEM SHALL first apply its existing index checks unchanged, then write every byte of the leaf into a new temporary file in leaves/ and flush it with sync_all, before any file exists under the leaf's 20-digit name. The temporary file's name SHALL be made by one pure function in file.rs, leaf_temp_name(pid: u32, index: u64, sequence: u64) -> String, returning `.` followed by pid, `-`, index zero-padded to LEAF_NAME_WIDTH digits, `-`, sequence, and `.tmp`. The sequence SHALL be supplied by the caller: create_leaf_temp(leaves_dir, index, next_sequence), write_leaf_temp(leaves_dir, index, write_contents, next_sequence) and put_leaf_with(index, bytes, next_sequence) in file.rs each take next_sequence: &mut impl FnMut() -> u64, and LeafStore::put_leaf calls put_leaf_with with a closure over one process-wide atomic counter, so the name put_leaf uses is not deterministic across calls and two writers never share one temporary file. create_leaf_temp SHALL name the file with the current process id, the index and the sequence next_sequence returns, and SHALL open it with OpenOptions create_new(true) and SHALL NOT open it with truncate; WHEN create_new fails with AlreadyExists, THE SYSTEM SHALL call next_sequence again and try the name it gives, so a leftover temporary file never blocks the write and is never replaced. write_leaf_temp SHALL create the file through create_leaf_temp, write the leaf's bytes into it and flush it. IF writing or flushing the temporary file fails, THEN THE SYSTEM SHALL remove this writer's own temporary file and return StoreError::Io with context naming the temporary file, SHALL NOT create any file under the leaf's 20-digit name, and SHALL NOT advance the extent. THE SYSTEM SHALL NOT write any leaf byte through a file opened under the leaf's final name, and SHALL NOT remove, truncate or write any temporary file other than the one this call created.

**Acceptance:**
- Test `leaf_temp_names_differ_by_sequence_and_by_pid`: `leaf_temp_name(42, 1, 7)` equals `.42-00000000000000000001-7.tmp`; `leaf_temp_name(42, 1, 7)` differs from `leaf_temp_name(42, 1, 8)`; and `leaf_temp_name(42, 1, 7)` differs from `leaf_temp_name(43, 1, 7)`.
- Test `a_temporary_file_never_replaces_an_existing_file`: a store holding leaf 0 = b"leaf-0" has a file holding b"pre-created" written into leaves/ under `leaf_temp_name(std::process::id(), 1, 0)`; the test calls `put_leaf_with(1, b"leaf-1", next)` where `next` is the test's own closure that returns 0, 1, 2 and so on from zero and records every value it returns; afterwards the recorded values are exactly `[0, 1]`, so the write met the pre-created file at sequence 0 and took its temporary file at sequence 1, `leaf(1)` is `Some(b"leaf-1".to_vec())`, and the pre-created file still exists and reads b"pre-created".
- Test `two_handles_on_one_directory_leave_no_temporary_name`: on a store directory holding leaf 0, one handle's `put_leaf(1, b"leaf-1")` returns `Ok(())` and leaves/ then holds no entry whose name begins with `.`; a second handle opened on the same directory afterwards returns `Ok(())` from `put_leaf(2, b"leaf-2")` and leaves/ again holds no entry whose name begins with `.`; `leaves/00000000000000000001` reads b"leaf-1", `leaves/00000000000000000002` reads b"leaf-2", and the counter's value after both calls is at least 2 greater than its value before the first.
- Test `a_write_failing_before_the_link_leaves_no_leaf`: a store holding leaf 0 = b"leaf-0", with the temporary file's flush failed by the fault seam, returns `StoreError::Io` from `put_leaf(1, b"leaf-1")`; afterwards `leaves/00000000000000000001` does not exist, `extent()` is 1, and `FileLeafStore::open` on the same directory returns a store whose `extent()` is 1.
- Test `a_failed_temporary_write_leaves_no_temporary_file`: a store holding leaf 0 = b"leaf-0", with the temporary file's write failed by the fault seam, returns an `Err` from `put_leaf(1, b"leaf-1")`, and afterwards leaves/ holds no entry whose name begins with `.`.
- Within `put_leaf` in crates/lys-log-store/src/file.rs, the value returned by `leaf_path` is passed only to `std::fs::hard_link`, as the link's destination, and to no `OpenOptions::open`, `File::create`, `std::fs::write` or `std::fs::rename` call.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C2 — StoreError has a documented RepairPending variant carrying the store's directory, the pinned tree size and the extent.

**Stories:**
- S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
- S2 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.

### R3: Give the leaf its final name with a link that refuses to replace

WHEN the temporary file is flushed, THE SYSTEM SHALL give the leaf its 20-digit name with std::fs::hard_link from the temporary name, an operation that fails rather than replace an existing entry. IF the link fails because the final name already exists, THEN THE SYSTEM SHALL remove this writer's own temporary file and return StoreError::LeafAlreadyWritten { index }, and SHALL NOT change, replace or remove the existing file under that name. IF the link fails for any other reason, THEN THE SYSTEM SHALL remove this writer's own temporary file and return StoreError::Io with context naming both paths. THE SYSTEM SHALL NOT use std::fs::rename, or any operation that can replace an existing entry, to create a leaf's final name.

**Acceptance:**
- Test `the_link_alone_refuses_a_leaf_this_store_never_saw` (the existing test `create_new_alone_refuses_a_leaf_this_store_never_saw`, renamed, its assertions unchanged): with leaf 0 stored and `leaves/00000000000000000001` written behind the store's back with b"another writers leaf", `put_leaf(1, b"ours")` returns `StoreError::LeafAlreadyWritten { index: 1 }` and `leaves/00000000000000000001` still reads b"another writers leaf".
- Test `a_refused_link_leaves_no_temporary_file`: with leaf 0 stored and `leaves/00000000000000000001` written behind the store's back with b"another writers leaf", the test calls `put_leaf(1, b"ours")` and ignores its result, and afterwards leaves/ holds no entry whose name begins with `.`; the test asserts nothing about the returned value or the bytes under the final name.
- The existing test `a_reused_index_is_refused_by_the_ordinary_route_too` passes unchanged.
- `grep -n 'fs::rename' crates/lys-log-store/src/file.rs` prints exactly one line, inside `write_state`.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C3 — FileLeafStore::open_read_only performs FileLeafStore::open's checks and returns a handle without flushing leaves/ or writing any file.

**Stories:**
- S3 (Witness operator, Runs a witness anchor that observes other logs' checkpoints) — As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.

### R4: Make the link the commit point for the steps after it

WHEN the link succeeds, THE SYSTEM SHALL advance the extent past the leaf before any later step, then remove the temporary name, then flush leaves/. IF removing the temporary name fails, THEN THE SYSTEM SHALL continue to the flush and SHALL return Ok when the flush succeeds; the leftover is ignored at open (R5). IF flushing leaves/ fails, THEN THE SYSTEM SHALL return StoreError::LeafDurabilityUncertain for that leaf's index with the flush's std::io::Error as its source, and SHALL NOT return StoreError::Io or StoreError::LeafAlreadyWritten for it. WHILE a handle holds a leaf whose durability is uncertain, WHEN put_leaf is called with any index, THE SYSTEM SHALL return StoreError::LeafDurabilityUncertain carrying the uncertain leaf's index and a source of the same kind and message as the original flush error, before any other check, and SHALL NOT write any file; the refusal ends only when the store is reopened. THE SYSTEM SHALL NOT refuse leaf, extent, origin, pinned or pin on that handle, and SHALL NOT report a leaf this handle linked as LeafAlreadyWritten.

**Acceptance:**
- Test `a_failed_temporary_removal_after_the_link_still_returns_ok`: a store holding leaf 0, with the temporary name's removal failed by the fault seam, returns `Ok(())` from `put_leaf(1, b"leaf-1")`; then `extent()` is 2, `leaf(1)` is `Some(b"leaf-1".to_vec())`, and `put_leaf(2, b"leaf-2")` returns `Ok(())`.
- Test `a_failed_directory_flush_after_the_link_returns_the_uncertain_error`: a store holding leaf 0, with the leaves/ flush failed by the fault seam, returns an error matching `StoreError::LeafDurabilityUncertain { index: 1, .. }` from `put_leaf(1, b"leaf-1")`.
- Test `the_uncertain_error_carries_the_flush_error_as_its_source`: with the leaves/ flush failed by the fault seam, the error returned by `put_leaf(1, b"leaf-1")` has a `std::error::Error::source` whose `to_string()` equals `injected leaves directory flush failure` and whose downcast `std::io::Error` has `kind()` equal to `std::io::ErrorKind::TimedOut`.
- Test `an_uncertain_handle_refuses_further_appends_by_name`: after that failed flush, on the same handle, `put_leaf(2, b"leaf-2")` and `put_leaf(1, b"leaf-1")` each return an error matching `StoreError::LeafDurabilityUncertain { index: 1, .. }`, and `leaves/00000000000000000002` does not exist.
- Test `the_extent_advances_past_a_linked_leaf_whose_flush_failed`: after that failed flush, `extent()` on the same handle is 2.
- Test `a_reopen_after_an_uncertain_flush_counts_the_leaf_and_appends_at_the_next_index`: after that failed flush, `FileLeafStore::open` on the same directory returns a store whose `extent()` is 2 and whose `leaf(1)` is `Some(b"leaf-1".to_vec())`, and on it `put_leaf(2, b"leaf-2")` returns `Ok(())` and `extent()` is then 3.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C4 — FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending and every file under the store directory holds the same bytes before and after.
- C5 — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.

**Stories:**
- S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.
- S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

### R5: Flush leaves/ at a writable open before counting, count without flushing at a read-only open, refuse a leaf write and a pin on a read-only handle, and leave leftovers untouched

WHEN FileLeafStore::open reaches the leaf count, THE SYSTEM SHALL flush leaves/ with the existing fsync_dir helper before contiguous_extent enumerates it. IF that flush fails, THEN THE SYSTEM SHALL return StoreError::Io whose context names the leaves directory and the flush, and SHALL NOT count the leaves, return a store, or write any file. FileLeafStore gains a public constructor open_read_only(dir: &Path) -> StoreResult<Self>, documented as the open for a reader, such as the store handed to Anchor::open_read_only: WHEN it is called, THE SYSTEM SHALL perform every check FileLeafStore::open performs, in the same order, except the flush of leaves/, and SHALL count the leaves that are named; it SHALL NOT flush leaves/ and SHALL NOT fail because a flush of leaves/ would fail. StoreError gains a variant ReadOnly with two fields, path: PathBuf, the store's directory, and operation: &'static str, the act refused, and the message `refusing to {operation} in the log store at {path}: it was opened read-only`; no existing variant's fields, message or documentation change. WHEN put_leaf is called on a handle returned by FileLeafStore::open_read_only, THE SYSTEM SHALL return StoreError::ReadOnly with path equal to the store's directory and operation `write a leaf`, before any other check, and SHALL NOT create, write, link, remove or flush any file; only a handle returned by FileLeafStore::open writes a leaf. WHEN pin is called on a handle returned by FileLeafStore::open_read_only, THE SYSTEM SHALL return StoreError::ReadOnly with path equal to the store's directory and operation `pin a root`, before any other check, and SHALL NOT write, rename or remove state.json or any other file, and SHALL NOT change the handle's pinned root; a pin is a write, and only a handle returned by FileLeafStore::open pins. contiguous_extent SHALL keep skipping every entry whose name begins with '.', so a leftover temporary file is never counted as a leaf. Neither FileLeafStore::open nor FileLeafStore::open_read_only SHALL delete, rename, truncate, write or otherwise change any file in the store's directory, and neither SHALL refuse to open because a leftover temporary file is present.

**Acceptance:**
- In crates/lys-log-store/src/file.rs, on the path `FileLeafStore::open` takes, the call to `fsync_dir` on the `leaves` directory comes before the call to `contiguous_extent`.
- Test `a_leftover_temporary_file_is_ignored_at_open`: a store holding leaf 0 = b"leaf-0", with a file holding b"torn" written into leaves/ under `leaf_temp_name(std::process::id() + 1, 1, 0)`, another writer's crash, opens with `extent()` 1; afterwards that file still exists and reads b"torn"; on the opened store `put_leaf(1, b"leaf-1")` returns `Ok(())`, `leaves/00000000000000000001` reads b"leaf-1", and the leftover still reads b"torn".
- Test `a_read_only_open_never_flushes_the_leaves_directory`: a store holding leaf 0 = b"leaf-0" and leaf 1 = b"leaf-1", opened read-only through the fault seam with the leaves/ flush at open armed to fail, returns `Ok` with `extent()` 2 and `leaf(1)` equal to `Some(b"leaf-1".to_vec())`; and `FileLeafStore::open_read_only` on the same directory, without the seam, returns a store whose `extent()` is 2.
- Test `a_writable_open_whose_leaves_flush_fails_is_refused_and_writes_nothing`: a store holding leaf 0 = b"leaf-0" and leaf 1 = b"leaf-1" has every file under its directory recorded as (path relative to the directory, bytes); opened writable through the fault seam with the leaves/ flush at open armed to fail, it returns `StoreError::Io` whose `context` contains `dir.join("leaves").display().to_string()` and the substring `flush`, and whose source has `kind()` equal to `std::io::ErrorKind::TimedOut`; the set of (path, bytes) recorded afterwards equals the set recorded before.
- Test `a_read_only_handle_refuses_put_leaf_by_name`: a store holding leaf 0 = b"leaf-0" has every file under its directory recorded as (path relative to the directory, bytes); on the handle `FileLeafStore::open_read_only` returns for it, `put_leaf(1, b"leaf-1")` returns `StoreError::ReadOnly { path, operation }` with `path` equal to the directory and `operation` equal to `write a leaf`, whose `to_string()` contains `dir.display().to_string()` and the substring `read-only`; `extent()` is then 1, and the set of (path, bytes) recorded afterwards equals the set recorded before.
- Test `a_read_only_handle_refuses_pin_by_name`: a store created in a temporary directory, with `put_leaf(0, b"leaf-0")`, `pin(PinnedRoot { tree_size: 1, root: [1; 32] })` and `put_leaf(1, b"leaf-1")` applied through `FileLeafStore::create`'s handle, so it stands one leaf ahead of its pin, is opened with `FileLeafStore::open_read_only`; `lys_core::merkle::raw_leaf_hash` of the bytes of `state.json` is recorded; `pin(PinnedRoot { tree_size: 2, root: [2; 32] })` on that handle returns `StoreError::ReadOnly { path, operation }` with `path` equal to the directory and `operation` equal to `pin a root`; afterwards `pinned().tree_size` on the handle is 1, and `raw_leaf_hash` of the bytes of `state.json` equals the recorded value.
- The existing test `an_unexpected_leaves_entry_is_detected_but_dotfiles_are_ignored` passes unchanged.

**Files:**
- modify: crates/lys-log-store/src/error.rs
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C6 — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
- C7 — A store one leaf past its pin, the state a read-only open refuses with RepairPending, is repaired by FileLeafStore::open followed by Log::open, which reports recovered_to as Some of the extent.
- C12 — file.rs's module doc states that FileLeafStore::leaf serves bytes it has not checked and that a leaf is proven whole only through Log::open against the pinned root.
- C14 — FileLeafStore::open_read_only is a documented inherent constructor that refuses what FileLeafStore::open refuses with the same errors, reports the same leftover temporary files, and never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and the LeafStore trait has the same methods as on main.
- C16 — pin through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly before any other pin check and changes no byte of the store.

**Stories:**
- S2 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a store left mid-append to be refused by name when I open it for reading so that I learn a repair is pending instead of performing one.
- S5 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a store I open read only to refuse every write, so that reading a store can never change it.
- S6 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.
- S7 (Leaf store maintainer, Reads the file store's contract before relying on it or changing it) — As a leaf store maintainer, I want the file store's module doc to say what opening a store does and never does, so that I can rely on open never deleting a leftover temporary file.

### R6: State the invariant in file.rs's module docs

file.rs's module docs gain a section stating: a file under a 20-digit leaf name is always a whole, flushed leaf; only files whose names begin with '.' may be partial, and they are never counted and never changed by open; a write that fails before a successful link removes its own temporary file; the no-replace link is the commit point, after which the leaf is this writer's and the extent advances; a failure to flush leaves/ after the link is LeafDurabilityUncertain and the handle refuses further appends until reopened; a writable open flushes leaves/ before counting, so a leaf named at a writable open is durable, and fails with Io when that flush fails; open_read_only counts the named leaves without flushing; clearing leftover temporary files that a crash stranded is not done here. The existing Layout and Durability sections SHALL NOT be weakened, and the section SHALL NOT claim that the directory flush holds on non-unix targets, where fsync_dir is a no-op.

**Acceptance:**
- The module docs of crates/lys-log-store/src/file.rs contain a heading line `//! # A named leaf is whole`.
- That section names `LeafDurabilityUncertain`, `open_read_only` and the words `commit point`, and states that open never deletes a leftover temporary file.
- The module-doc lines of file.rs present before this change are all still present, byte-identical, in `git diff`.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C8 — A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch carrying both trees, and state.json is unchanged.

### R7: Add a test-only fault seam to FileLeafStore

FileLeafStore gains a private field, compiled only under cfg(test), holding at most one armed fault, and a private enum naming the five steps a fault can fail: the temporary file's write, the temporary file's flush, the temporary name's removal after the link, the leaves/ flush after the link, and the leaves/ flush at open. A private constructor, compiled only under cfg(test), opens a store writable or read-only with one fault already armed, so that the flush at open can be failed before the leaves are counted. WHEN put_leaf or open reaches an armed step, THE SYSTEM SHALL disarm the fault and treat that step as having failed with an injected std::io::Error, taking exactly the path a real failure of that step takes. The error injected for the leaves/ flush after the link, and for the leaves/ flush at open, SHALL be std::io::Error::new(std::io::ErrorKind::TimedOut, "injected leaves directory flush failure"), whose kind is not Other and whose message is not `flush failed`. The seam SHALL NOT be public, SHALL NOT exist in any build other than the crate's own tests, and SHALL NOT change put_leaf's, open's or open_read_only's behaviour when no fault is armed.

**Acceptance:**
- Every line in crates/lys-log-store/src/file.rs that declares or reads the fault field, the fault enum or the constructor that opens with a fault armed is under a `#[cfg(test)]` attribute or inside an item that is.
- `cargo clippy --all-targets -- -D warnings` and `cargo clippy --all-targets --all-features -- -D warnings` pass with no `#[allow]` added to file.rs.
- The fault field, the fault enum and the constructor that opens with a fault armed in crates/lys-log-store/src/file.rs are declared with no visibility modifier: neither `pub` nor `pub(crate)` nor `pub(super)` appears on their declaration lines.
- In crates/lys-log-store/src/file.rs, the seam's error for the leaves/ flush after the link and for the leaves/ flush at open is built as `std::io::Error::new(std::io::ErrorKind::TimedOut, "injected leaves directory flush failure")`.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C9 — A torn leaf just past the pin is adopted by the one-leaf repair and reported by recovered_to.

### R8: Add the tests and prove each against its own drift

file_tests.rs gains the eighteen new tests named in R1, R2, R3, R4 and R5's acceptance lines and renames create_new_alone_refuses_a_leaf_this_store_never_saw to the_link_alone_refuses_a_leaf_this_store_never_saw with its assertions unchanged. Its text stops stating what this brief makes false, in four places, each naming the no-replace link instead of create_new with no assertion changed: the module-doc header sentence that says the index is refused once via create_new at the filesystem; the extent row of the absent-check table, which says create_new succeeds; the comment in extent_check_alone_refuses_a_deleted_leafs_index, which says create_new would succeed; and the comment in the renamed test, which says it isolates the create_new check. The absent-check table's second row is rewritten to name the no-replace link and drift injection 13 of this brief's verification, which replaces the link with an operation that replaces an existing entry. A new drift-injection table holds one row for each of drift injections 1 to 12 and 14 to 18, each naming the one test it fails. Each injection SHALL be applied and reverted before the change lands. No test SHALL be #[ignore]d, and no existing test other than the renamed one SHALL change beyond the comment named above.

**Acceptance:**
- `cargo test -p lys-log-store --all-features` runs 40 tests in `file::tests` and all 40 pass.
- The module docs of crates/lys-log-store/src/file_tests.rs contain a new table with exactly seventeen injection rows, for drift injections 1 to 12 and 14 to 18 of this brief's verification, each naming exactly one test.
- The absent-check table in those module docs has a row naming the no-replace link, drift injection 13 and the test `the_link_alone_refuses_a_leaf_this_store_never_saw`.
- `grep -c create_new crates/lys-log-store/src/file_tests.rs` prints 0.
- `git diff` of crates/lys-log-store/src/file_tests.rs removes no line containing `assert` from any test that exists before this change.
- crates/lys-log-store/src/file.rs has fewer than 500 lines that are not blank, not comments and not under `#[cfg(test)]`.

**Files:**
- modify: crates/lys-log-store/src/file_tests.rs

**Checklist:**
- C10 — FileLeafStore::leftover_temporaries returns the store's temporary-leaf names in lexical order, none counted toward the extent and none changed.
- C11 — file.rs's module doc has a section stating what open and open_read_only do and never do, including that neither deletes a leftover temporary file.

### R9: Record in DIRECTORY-003 that it waits on this brief

DIRECTORY-003's depends_on gains LYSLOGSTORE-001 after DIRECTORY-002, so the dispatcher does not start DIRECTORY-003 before this brief lands, and DIRECTORY-003.md is re-rendered from the JSON by the method's renderer. No other field of DIRECTORY-003.json SHALL change, no other document under docs/design/directory SHALL change, and DIRECTORY-002 SHALL NOT be removed from depends_on.

**Acceptance:**
- `python3 -c "import json; print(json.load(open('docs/design/directory/briefs/DIRECTORY-003.json'))['depends_on'])"` prints `['DIRECTORY-002', 'LYSLOGSTORE-001']`.
- Loading docs/design/directory/briefs/DIRECTORY-003.json before and after the change and deleting the `depends_on` key from both gives equal objects.
- `git diff --name-only` lists no file under docs/design/directory other than briefs/DIRECTORY-003.json and briefs/DIRECTORY-003.md.
- `sh scripts/design/gate.sh` exits 0, so docs/design/directory/briefs/DIRECTORY-003.md is what DIRECTORY-003.json renders to.

**Files:**
- modify: docs/design/directory/briefs/DIRECTORY-003.json
- modify: docs/design/directory/briefs/DIRECTORY-003.md

**Checklist:**
- C13 — StoreError has two new variants, ReadOnly and RepairPending, neither of which carries or prints a leaf index, and no existing variant's fields or message changes.

**Stories:**
- S1 (Auditor, Reading a flight recorder's log without changing it) — As an auditor, I want a leaf store I open for reading to refuse every write so that reading the log never changes the evidence.

### R10: Open the store for Anchor::open_read_only with FileLeafStore::open_read_only

In lys-anchor, every call to Anchor::open_read_only in Anchor::open_read_only's two doc examples in crates/lys-anchor/src/anchor/read_only.rs, in crates/lys-anchor/src/anchor/read_only_tests.rs and in crates/lys-anchor/tests/standalone_is_complete.rs SHALL take as its store the value of FileLeafStore::open_read_only on the directory it opened with FileLeafStore::open before, so the one read-only path is the one they exercise. No other line of crates/lys-anchor/tests/standalone_is_complete.rs SHALL change; the other lines of read_only.rs and read_only_tests.rs change only as R12 states, Anchor::open_read_only's signature SHALL NOT change, and no assertion in the tests that exist before this change SHALL change.

**Acceptance:**
- `grep -c 'FileLeafStore::open_read_only' crates/lys-anchor/src/anchor/read_only.rs` prints 2, `grep -c 'FileLeafStore::open_read_only' crates/lys-anchor/src/anchor/read_only_tests.rs` prints 3, and `grep -c 'FileLeafStore::open_read_only' crates/lys-anchor/tests/standalone_is_complete.rs` prints 1.
- `grep -rn -A1 'Anchor::open_read_only(' crates/lys-anchor/src/anchor/read_only.rs crates/lys-anchor/src/anchor/read_only_tests.rs crates/lys-anchor/tests/standalone_is_complete.rs` prints no line containing `FileLeafStore::open(`.
- `git diff --name-only -- crates/lys-anchor` lists exactly crates/lys-anchor/src/anchor/read_only.rs, crates/lys-anchor/src/anchor/read_only_tests.rs, crates/lys-anchor/src/anchor/status.rs and crates/lys-anchor/tests/standalone_is_complete.rs, and every line `git diff -- crates/lys-anchor/tests/standalone_is_complete.rs` adds contains `FileLeafStore::open_read_only`.
- `cargo test -p lys-anchor --all-features` passes, including the doc tests of crates/lys-anchor/src/anchor/read_only.rs, the passing example and the compile_fail example each as before.

**Files:**
- modify: crates/lys-anchor/src/anchor/read_only.rs
- modify: crates/lys-anchor/src/anchor/read_only_tests.rs
- modify: crates/lys-anchor/tests/standalone_is_complete.rs

**Checklist:**
- C15 — put_leaf through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and changes no byte of the store.

**Stories:**
- S6 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want a read-only open of a store with an interrupted append to refuse and say that a repair is pending, so that I learn the store is past its pin without my open repairing it.

### R11: Open a log at its pin without repairing it

Log gains a public constructor open_at_pin(store: S) -> StoreResult<Self> and a public accessor pending_repair(&self) -> Option<u64>. WHEN open_at_pin is called, THE SYSTEM SHALL read every leaf in the store's extent, rebuild the tree and compare it with the pin exactly as Log::open does. WHEN the rebuilt tree equals the pin, THE SYSTEM SHALL return the log as Log::open would, with pending_repair() None. WHEN the store holds exactly one leaf more than the pinned tree size and the pinned-size prefix rebuilds to the pinned root, THE SYSTEM SHALL NOT call the store's pin, SHALL hold the tree and the leaves of the pinned prefix only, so tree().len() equals the pinned tree size, SHALL report pending_repair() as Some(the store's extent), the tree size a writable open repairs to, and recovered_to() as None, and SHALL return StoreError::AppendAwaitsRepair from every append on that log without calling put_leaf, because the store already holds a leaf at the next index and the store never reports its own leaf as another writer's; it SHALL NOT return StoreError::Poisoned, which keeps its one meaning, an append that failed after storing its leaf on this handle. StoreError gains a variant AppendAwaitsRepair, a name distinct from the RepairPending variant LYSLOGSTORE-003 landed, with two fields, pinned_tree_size: u64, the pin's tree size the log was opened at, and leaves: u64, the count of leaves the store held, and the message `refusing to append: the log has a pending repair, its store holds {leaves} leaves but its pin is at tree size {pinned_tree_size}; a writable open must repair it before any append`; it is documented as: the log was opened at its pin with one leaf standing ahead of the pin, and a writable open (Log::open over a handle from FileLeafStore::open) must repair it before any append. Log::append's `# Errors` section gains one line naming StoreError::AppendAwaitsRepair, and Log::append gains only the check that returns it, placed before its existing checks. Every other divergence SHALL return StoreError::PinMismatch as Log::open does. Log::open SHALL NOT change: it alone performs the one-leaf-ahead repair, through the store's pin, and its pending_repair() is always None. THE SYSTEM SHALL NOT write, rename or delete any file during open_at_pin.

**Acceptance:**
- Test `a_log_opened_at_its_pin_leaves_the_pin_and_refuses_to_append_while_a_repair_is_pending` in crates/lys-log-store/src/log_tests.rs: a directory initialised with `FileLeafStore::create(dir, ORIGIN)`, opened with `Log::open(FileLeafStore::open(dir))` and given one `append(b"leaf-0")`, and that log dropped, then `leaves/00000000000000000001` written behind it with b"an append interrupted before its pin", is opened with `Log::open_at_pin(FileLeafStore::open(dir))`; it returns `Ok` with `tree().len()` 1, `pending_repair()` `Some(2)`, `recovered_to()` `None` and `store().pinned().tree_size` 1; `append(b"leaf-2")` on it returns `Err(err)` where `err` matches `StoreError::AppendAwaitsRepair { pinned_tree_size: 1, leaves: 2 }`, `matches!(err, StoreError::Poisoned)` is false, and `err.to_string()` equals `refusing to append: the log has a pending repair, its store holds 2 leaves but its pin is at tree size 1; a writable open must repair it before any append`; afterwards leaves/ holds exactly the two names `00000000000000000000` and `00000000000000000001`, and the bytes of `state.json` equal the bytes recorded before the open.
- In the same test, after the refused append and with the `open_at_pin` log dropped, `Log::open(FileLeafStore::open(dir))` returns `Ok` with `recovered_to()` `Some(2)`, `pending_repair()` `None` and `store().pinned().tree_size` 2; `append(b"leaf-2")` on that log then returns `Ok` with index 2, and afterwards `tree().len()` is 3 and `store().pinned().tree_size` is 3.
- `git diff` of crates/lys-log-store/src/log.rs removes no line from the body of `Log::open`, `reconcile_with_pin` or `Log::append`, and adds to the body of `Log::append` only the check that returns `StoreError::AppendAwaitsRepair`.

**Files:**
- modify: crates/lys-log-store/src/error.rs
- modify: crates/lys-log-store/src/log.rs
- modify: crates/lys-log-store/src/log_tests.rs

**Checklist:**
- C17 — FileLeafStore::open_read_only over a store whose leaf count is past its pinned tree size returns StoreError::RepairPending, reads no leaf byte and changes no byte of the store.

**Stories:**
- S7 (Leaf store maintainer, Reads the file store's contract before relying on it or changing it) — As a leaf store maintainer, I want the file store's module doc to say what opening a store does and never does, so that I can rely on open never deleting a leftover temporary file.

### R12: Keep Anchor::open_read_only refused on a store one leaf ahead of its pin, and say so

Anchor::open_read_only SHALL NOT change its behaviour, and AnchorStatus SHALL NOT change. Its store comes from FileLeafStore::open_read_only (R10), which LYSLOGSTORE-003 made refuse a store standing exactly one leaf ahead of its pin with StoreError::RepairPending { path, pinned_size, extent } before any leaf is read, so a read-only anchor over that store is refused, never opened, and nothing is repaired or pinned through it. The module docs of crates/lys-anchor/src/anchor/read_only.rs and Anchor::open_read_only's docs SHALL state that a read-only open never repairs, that a store one leaf ahead of its pin is refused by FileLeafStore::open_read_only with StoreError::RepairPending, and that only a writable open repairs it. They SHALL NOT say that open_read_only repairs an interrupted append. Anchor::open and its repair through Log::open SHALL NOT change.

**Acceptance:**
- Test `a_reader_is_refused_a_store_one_leaf_ahead_of_its_pin` in crates/lys-anchor/src/anchor/read_only_tests.rs: an anchor created with its genesis leaf, then `leaves/00000000000000000001` written behind it with b"an append interrupted before its pin", and the bytes of `state.json` recorded; `FileLeafStore::open_read_only(dir)` returns `Err(StoreError::RepairPending { pinned_size: 1, extent: 2, .. })`; afterwards the bytes of `state.json` equal the recorded bytes and leaves/ holds exactly the two names `00000000000000000000` and `00000000000000000001`.
- Test `a_writable_open_repairs_the_log_a_reader_was_refused`: over the same fixture, `Log::open(FileLeafStore::open(dir).unwrap())` returns `Ok` with `recovered_to()` `Some(2)` and `store().pinned().tree_size` 2; after that log is dropped, `Anchor::open_read_only(FileLeafStore::open_read_only(dir).unwrap(), AnchorConfig::unconfigured())` returns `Ok` and its `tree_size()` is 2.
- `git diff` of crates/lys-anchor/src/anchor/status.rs is empty.
- The module docs of crates/lys-anchor/src/anchor/read_only.rs contain the words `never repairs` and `RepairPending`, and contain no sentence stating that open_read_only repairs an interrupted append.

**Files:**
- modify: crates/lys-anchor/src/anchor/read_only.rs
- modify: crates/lys-anchor/src/anchor/read_only_tests.rs
- modify: crates/lys-anchor/src/anchor/status.rs

**Checklist:**
- C18 — A store that FileLeafStore::open_read_only refused with StoreError::RepairPending, opened with FileLeafStore::open and Log::open, is repaired and the repair is reported through recovered_to, as on main.

**Stories:**
- S7 (Leaf store maintainer, Reads the file store's contract before relying on it or changing it) — As a leaf store maintainer, I want the file store's module doc to say what opening a store does and never does, so that I can rely on open never deleting a leftover temporary file.

### R13: Render the lys-log-store cluster's documents from their JSON

docs/design/lys-log-store/DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSLOGSTORE-001.md SHALL be what render-cluster.py renders from design.json, checklist.json, stories.json and briefs/LYSLOGSTORE-001.json, and WHEN any of those JSON documents changes, THE SYSTEM SHALL have its markdown re-rendered in the same change. The rendered files SHALL NOT be edited by hand, and BACKENDS.md SHALL NOT change.

**Acceptance:**
- `ls docs/design/lys-log-store/DESIGN.md docs/design/lys-log-store/CHECKLIST.md docs/design/lys-log-store/USER-STORIES.md docs/design/lys-log-store/briefs/LYSLOGSTORE-001.md` lists all four files and exits 0.
- With those four files present, `sh scripts/design/gate.sh` exits 0 and prints no line containing `rendered markdown differs`.
- `git diff --name-only -- docs/design/lys-log-store/BACKENDS.md` prints nothing.

**Files:**
- modify: docs/design/lys-log-store/DESIGN.md
- modify: docs/design/lys-log-store/CHECKLIST.md
- modify: docs/design/lys-log-store/USER-STORIES.md
- modify: docs/design/lys-log-store/briefs/LYSLOGSTORE-001.md

**Checklist:**
- C19 — The module doc of crates/lys-log-store/src/file.rs states what open and open_read_only do and never do, including that open never deletes a leftover temporary file.

## Boundaries

- SHALL NOT delete, rename, truncate or write any file during FileLeafStore::open or FileLeafStore::open_read_only.
- SHALL NOT add a method to the LeafStore trait or change any existing method's signature.
- SHALL NOT change the on-disk layout: log.json, state.json, leaves/<20-digit index> holding the raw leaf bytes, and the lys/log-dir/v1 marker.
- SHALL NOT change Log::open, Log's append order or Log::open's one-leaf-ahead repair, and SHALL NOT change Log::append other than by R11's refusal with StoreError::RepairPending and its line in the `# Errors` section; crates/lys-log-store/src/log.rs changes only by R11's additions.
- SHALL NOT change any file outside crates/lys-log-store other than DIRECTORY-003.json, DIRECTORY-003.md, R10's three lys-anchor files and R12's crates/lys-anchor/src/anchor/status.rs and the lys-log-store cluster's own design documents under docs/design/lys-log-store other than BACKENDS.md; in crates/lys-anchor/tests/standalone_is_complete.rs only the line that opens a store for Anchor::open_read_only changes.
- SHALL NOT pin, write, rename or delete any file in the store's directory while Anchor::open_read_only or Log::open_at_pin opens a store found one leaf ahead of its pin.
- SHALL NOT use StoreError::LeafAlreadyWritten for any failure other than another writer holding the index.
- SHALL NOT implement clearing of leftover temporary files that a crash stranded; that is a further unit. Removing this writer's own temporary file after its own failed write is not that unit.
- SHALL NOT add a dependency, unsafe code, or unwrap, expect, panic, todo, unimplemented or unreachable in library code.
- SHALL NOT silence a lint with #[allow], #[ignore] a test, or use #[cfg(any())].
- SHALL NOT change any document under docs/design/directory other than DIRECTORY-003's depends_on and its re-rendered DIRECTORY-003.md.
- SHALL NOT return StoreError::Poisoned for any failure other than an append that failed after storing its leaf on the same Log handle.
- Revised by Waffles on 28 Sep 2026: LYSLOGSTORE-003 landed first with a read-only open that refuses a store one leaf ahead of its pin. Main's behaviour and variants stand. R11 names its append refusal AppendAwaitsRepair, and R12 keeps the refusal rather than opening at the pin.

## Verification

- Run every leg of the design's gate array from the repository root: cargo fmt --all then git diff --exit-code, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features, cargo doc --no-deps, sh scripts/design/gate.sh; each exits 0.
- Run every drift injection below with `cargo test -p lys-log-store --all-features`; every test that uses a sequence supplies its own, so the result does not depend on the order in which tests run.
- Drift injection 1: flush the temporary file after the link instead of before it; exactly one test fails, a_write_failing_before_the_link_leaves_no_leaf. Revert.
- Drift injection 2: make open remove every dot-prefixed entry in leaves/ before counting; exactly one test fails, a_leftover_temporary_file_is_ignored_at_open. Revert.
- Drift injection 3: return StoreError::Io instead of StoreError::LeafAlreadyWritten when the link finds the final name taken; exactly one test fails, the_link_alone_refuses_a_leaf_this_store_never_saw. Revert.
- Drift injection 4: return StoreError::Io when removing the temporary name fails; exactly one test fails, a_failed_temporary_removal_after_the_link_still_returns_ok. Revert.
- Drift injection 5: return StoreError::Io, with the same source, instead of LeafDurabilityUncertain when the leaves/ flush after the link fails, leaving the refusal and the extent as they are; exactly one test fails, a_failed_directory_flush_after_the_link_returns_the_uncertain_error. Revert.
- Drift injection 6: skip the handle's refusal after an uncertain flush; exactly one test fails, an_uncertain_handle_refuses_further_appends_by_name. Revert.
- Drift injection 7: advance the extent only after the leaves/ flush succeeds; exactly one test fails, the_extent_advances_past_a_linked_leaf_whose_flush_failed. Revert.
- Drift injection 8: skip removing the temporary file when writing or flushing it fails; exactly one test fails, a_failed_temporary_write_leaves_no_temporary_file. Revert.
- Drift injection 9: skip removing the temporary file when the link fails; exactly one test fails, a_refused_link_leaves_no_temporary_file. Revert.
- Drift injection 10: build LeafDurabilityUncertain's source as a new std::io::Error of kind Other with the message `flush failed` instead of the flush's own error; exactly one test fails, the_uncertain_error_carries_the_flush_error_as_its_source. Revert.
- Drift injection 11: open the temporary file in write_leaf_temp with create(true).truncate(true) instead of create_new(true); exactly one test fails, a_temporary_file_never_replaces_an_existing_file. Revert.
- Drift injection 12: build the temporary name in leaf_temp_name without the pid; exactly one test fails, leaf_temp_names_differ_by_sequence_and_by_pid. Revert.
- Drift injection 13: give the leaf its final name with std::fs::rename in place of std::fs::hard_link; exactly one test fails, the_link_alone_refuses_a_leaf_this_store_never_saw. Revert.
- Drift injection 14: make FileLeafStore::open_read_only flush leaves/ before counting, as FileLeafStore::open does; exactly one test fails, a_read_only_open_never_flushes_the_leaves_directory. Revert.
- Drift injection 15: make FileLeafStore::open ignore a failed leaves/ flush and go on to count; exactly one test fails, a_writable_open_whose_leaves_flush_fails_is_refused_and_writes_nothing. Revert.
- Drift injection 16: remove `; reopen the store` from LeafDurabilityUncertain's message; exactly one test fails, the_uncertain_error_names_its_index_its_cause_and_the_reopen. Revert.
- Drift injection 17: skip the ReadOnly refusal in put_leaf on a handle from FileLeafStore::open_read_only; exactly one test fails, a_read_only_handle_refuses_put_leaf_by_name. Revert.
- Drift injection 18: skip the ReadOnly refusal in pin on a handle from FileLeafStore::open_read_only; exactly one test fails, a_read_only_handle_refuses_pin_by_name. Revert.
- Drift injection 19, run with `cargo test -p lys-log-store --all-features`: remove the check in Log::append that returns StoreError::RepairPending while a repair is pending; exactly one test fails, a_log_opened_at_its_pin_leaves_the_pin_and_refuses_to_append_while_a_repair_is_pending. Revert.
- Drift injection 20, run with `cargo test -p lys-anchor --all-features`: make Anchor::open_read_only open its log with Log::open instead of Log::open_at_pin; exactly one test fails, a_reader_opens_a_log_one_leaf_ahead_at_its_pin_and_reports_the_pending_repair. Revert.
- Confirm that a_reopen_after_an_uncertain_flush_counts_the_leaf_and_appends_at_the_next_index, two_handles_on_one_directory_leave_no_temporary_name and a_writable_open_repairs_the_log_a_reader_found_one_leaf_ahead have no injection row: the first guards the reopen path, and the open-time flush it rides on is verified by reading the line order in FileLeafStore::open, since no test here observes a power loss; the second guards the absence of leftovers across handles, whose removal steps drift injections 8 and 9 already isolate; the third guards Log::open's existing one-leaf-ahead repair, which this brief does not change and which lys-anchor's an_interrupted_append_is_repaired_and_reported_rather_than_swallowed already guards.
- grep -rn 'unwrap()\|expect(\|panic!' crates/lys-log-store/src/file.rs crates/lys-log-store/src/error.rs finds nothing outside the tests module.
