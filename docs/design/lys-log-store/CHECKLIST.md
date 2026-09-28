# Lys-Log-Store — Checklist

## Errors

- [ ] **C1** — StoreError has a variant LeafDurabilityUncertain carrying the leaf index and the failed flush's std::io::Error as its source, its message naming both, distinct from Io and from LeafAlreadyWritten, and LeafStore::put_leaf's documented errors name it.

## Writing a leaf

- [ ] **C2** — FileLeafStore::put_leaf writes and flushes a leaf's bytes in a dot-prefixed temporary file in leaves/, named by leaf_temp_name from the process id, the index and a sequence the caller supplies (a process-wide counter for put_leaf) and created with create_new so it never replaces an existing file, before any file exists under the leaf's 20-digit name, and removes that temporary file when writing or flushing it fails.
- [ ] **C3** — put_leaf gives the leaf its final name with a link that refuses to replace an existing entry, and an existing final name returns LeafAlreadyWritten for that index with the existing bytes unchanged and this writer's temporary file removed.
- [ ] **C4** — A successful link advances the extent; put_leaf then removes the temporary name and flushes leaves/, and a failure to remove the temporary name still returns Ok.
- [ ] **C5** — A failure to flush leaves/ after the link returns LeafDurabilityUncertain for that index, and the handle refuses every further put_leaf with that error until the store is reopened.

## Opening a store

- [ ] **C6** — FileLeafStore::open flushes leaves/ before it counts the leaves, and when that flush fails it returns Io naming the leaves directory and the flush, and writes nothing.
- [ ] **C7** — FileLeafStore::open never counts, deletes or changes a leftover dot-prefixed temporary file in leaves/.
- [ ] **C12** — FileLeafStore::open_read_only performs FileLeafStore::open's checks without flushing leaves/, counts the named leaves, and does not fail because a flush of leaves/ would fail.
- [ ] **C14** — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly naming the store's directory and the act, and writes nothing.
- [ ] **C16** — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly naming the store's directory and the act, and leaves state.json and the pinned root unchanged.

## Invariant and tests

- [ ] **C8** — file.rs's module docs state that a named leaf is whole and flushed, that only hidden temporary files may be partial, and that the link is the commit point.
- [ ] **C9** — A fault seam that fails a chosen step of put_leaf or the leaves/ flush at open exists in FileLeafStore and compiles only under cfg(test).
- [ ] **C10** — file_tests.rs holds the eighteen new tests of LYSLOGSTORE-001 R8, names the no-replace link wherever it named create_new, and each drift injection in its module-doc tables fails exactly its own test.
- [ ] **C11** — Every leg of the design's gate array passes and file.rs stays under 500 lines of code.

## Dependent briefs

- [ ] **C13** — DIRECTORY-003's depends_on names LYSLOGSTORE-001 after DIRECTORY-002, and no other field of DIRECTORY-003 changes.

## Read-only callers

- [ ] **C15** — Anchor::open_read_only's doc examples and lys-anchor's read-only tests open their store with FileLeafStore::open_read_only.
- [ ] **C17** — Log::open_at_pin opens a store found one leaf ahead of its pin at the pinned head without pinning, reports the pending repair, and refuses every append by name while it is pending, never as Poisoned; Log::open alone repairs.
- [ ] **C18** — Anchor::open_read_only never repairs a store found one leaf ahead of its pin: it opens at the pinned head, and its status says in words that one leaf stands ahead of the pin and that a writable open repairs it.

## Rendered documents

- [ ] **C19** — The lys-log-store cluster's DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSLOGSTORE-001.md are what render-cluster.py renders from its JSON documents, and the design gate passes with them present.

## Refusal at open

- [ ] **C8** — Log::open over a store whose leaf inside the pinned prefix is torn returns StoreError::PinMismatch stating the pin's tree size and root and the recomputed tree size and root, names no leaf, and leaves state.json unchanged.
- [ ] **C9** — Log::open over a store whose leaf inside the pinned prefix has one byte changed returns the same refusal, naming no leaf and leaving state.json unchanged.
- [ ] **C10** — Log::open over a store with a torn leaf at the index just past the pin repairs and pins it and reports the repair through recovered_to, exactly as before this change.
- [ ] **C11** — lys log status on a log whose pinned prefix holds a torn leaf exits 1 and prints the refusal on stderr.
- [ ] **C12** — lys-anchor status on an anchor whose pinned prefix holds a torn leaf exits 1 and prints the refusal on stderr.

## Leaf store at open

- [ ] **C1** — FileLeafStore::open returns, sorted, the name of every entry in leaves/ that has the store's temporary-leaf form, does not count any of them as a leaf, and leaves each byte-identical.
- [ ] **C2** — Every lys log command that opens a store prints one stderr line naming the leftover temporary leaf files the store reported.
- [ ] **C3** — Every lys-anchor command that opens an anchor prints one stderr line naming the leftover temporary leaf files the store reported.
- [ ] **C4** — scripts/verify_inclusion.py exits 0 on an inclusion artifact and a leaf file from a store written after this change.

## Witness

- [ ] **C5** — WitnessProjection records how many leaves it has folded and folds forward from that position only.
- [ ] **C6** — observe parses at most one leaf more than the number of leaves recorded since the previous observe through the same projection, counted on the parse path.
- [ ] **C7** — After every observe, the kept projection reports the same latest state for every origin as WitnessProjection::rebuild over the same anchor, including after a rollback.

## Read-only open

- [ ] **C1** — StoreError has a documented ReadOnly variant carrying the store's directory and the refused act.
- [ ] **C2** — StoreError has a documented RepairPending variant carrying the store's directory, the pinned tree size and the extent.
- [ ] **C3** — FileLeafStore::open_read_only performs FileLeafStore::open's checks and returns a handle without flushing leaves/ or writing any file.
- [ ] **C4** — FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending and every file under the store directory holds the same bytes before and after.
- [ ] **C5** — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
- [ ] **C6** — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
- [ ] **C7** — A store one leaf past its pin, the state a read-only open refuses with RepairPending, is repaired by FileLeafStore::open followed by Log::open, which reports recovered_to as Some of the extent.

## Pinned prefix

- [ ] **C8** — A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch carrying both trees, and state.json is unchanged.
- [ ] **C9** — A torn leaf just past the pin is adopted by the one-leaf repair and reported by recovered_to.

## Leftover temporary files

- [ ] **C10** — FileLeafStore::leftover_temporaries returns the store's temporary-leaf names in lexical order, none counted toward the extent and none changed.

## Module documentation

- [ ] **C11** — file.rs's module doc has a section stating what open and open_read_only do and never do, including that neither deletes a leftover temporary file.
- [ ] **C12** — file.rs's module doc states that FileLeafStore::leaf serves bytes it has not checked and that a leaf is proven whole only through Log::open against the pinned root.

## Read-only open

- [ ] **C13** — StoreError has two new variants, ReadOnly and RepairPending, neither of which carries or prints a leaf index, and no existing variant's fields or message changes.
- [ ] **C14** — FileLeafStore::open_read_only is a documented inherent constructor that refuses what FileLeafStore::open refuses with the same errors, reports the same leftover temporary files, and never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and the LeafStore trait has the same methods as on main.
- [ ] **C15** — put_leaf through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and changes no byte of the store.
- [ ] **C16** — pin through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly before any other pin check and changes no byte of the store.
- [ ] **C17** — FileLeafStore::open_read_only over a store whose leaf count is past its pinned tree size returns StoreError::RepairPending, reads no leaf byte and changes no byte of the store.
- [ ] **C18** — A store that FileLeafStore::open_read_only refused with StoreError::RepairPending, opened with FileLeafStore::open and Log::open, is repaired and the repair is reported through recovered_to, as on main.
- [ ] **C19** — The module doc of crates/lys-log-store/src/file.rs states what open and open_read_only do and never do, including that open never deletes a leftover temporary file.
