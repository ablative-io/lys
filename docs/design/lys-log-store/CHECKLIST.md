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
