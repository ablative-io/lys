# Lys-Log-Store — Checklist

## Read-only open

- [ ] **C1** — StoreError has a documented ReadOnly variant carrying the store's directory and the refused act.
- [ ] **C2** — StoreError has a documented RepairPending variant carrying the store's directory, the pinned tree size and the extent.
- [ ] **C3** — FileLeafStore::open_read_only performs FileLeafStore::open's checks and returns a handle without flushing leaves/ or writing any file.
- [ ] **C4** — FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending and every file under the store directory holds the same bytes before and after.
- [ ] **C5** — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
- [ ] **C6** — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and every file under the store directory holds the same bytes before and after.
- [ ] **C7** — A store one leaf past its pin, the state a read-only open refuses with RepairPending, is repaired by FileLeafStore::open followed by Log::open, which reports recovered_to as Some of the extent.
- [ ] **C13** — StoreError has two new variants, ReadOnly and RepairPending, neither of which carries or prints a leaf index, and no existing variant's fields or message changes.
- [ ] **C14** — FileLeafStore::open_read_only is a documented inherent constructor that refuses what FileLeafStore::open refuses with the same errors, reports the same leftover temporary files, and never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and the LeafStore trait has the same methods as on main.
- [ ] **C15** — put_leaf through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and changes no byte of the store.
- [ ] **C16** — pin through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly before any other pin check and changes no byte of the store.
- [ ] **C17** — FileLeafStore::open_read_only over a store whose leaf count is past its pinned tree size returns StoreError::RepairPending, reads no leaf byte and changes no byte of the store.
- [ ] **C18** — A store that FileLeafStore::open_read_only refused with StoreError::RepairPending, opened with FileLeafStore::open and Log::open, is repaired and the repair is reported through recovered_to, as on main.
- [ ] **C19** — The module doc of crates/lys-log-store/src/file.rs states what open and open_read_only do and never do, including that open never deletes a leftover temporary file.

## Pinned prefix

- [ ] **C8** — A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch carrying both trees, and state.json is unchanged.
- [ ] **C9** — A torn leaf just past the pin is adopted by the one-leaf repair and reported by recovered_to.

## Leftover temporary files

- [ ] **C10** — FileLeafStore::leftover_temporaries returns the store's temporary-leaf names in lexical order, none counted toward the extent and none changed.

## Module documentation

- [ ] **C11** — file.rs's module doc has a section stating what open and open_read_only do and never do, including that neither deletes a leftover temporary file.
- [ ] **C12** — file.rs's module doc states that FileLeafStore::leaf serves bytes it has not checked and that a leaf is proven whole only through Log::open against the pinned root.

## Proofs never hold the whole log

- [ ] **C20** — A hash-only proof tree built by streaming (LYSLOGSTORE-005 R1), proved by a counting test that fails at the base.
