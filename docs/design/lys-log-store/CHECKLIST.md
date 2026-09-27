# Lys-Log-Store — Checklist

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

- [ ] **C13** — StoreError has two new variants, ReadOnly and RepairPending, neither of which carries or prints a leaf index, and no existing variant's fields or message changes.
- [ ] **C14** — FileLeafStore::open_read_only is a documented inherent constructor that never flushes, creates, writes, renames, links or removes a file, leftover temporary files included, and the LeafStore trait has the same methods as on main.
- [ ] **C15** — put_leaf through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly and changes no byte of the store.
- [ ] **C16** — pin through a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly before any other pin check and changes no byte of the store.
- [ ] **C17** — FileLeafStore::open_read_only over a store whose leaf count is past its pinned tree size returns StoreError::RepairPending, reads no leaf byte and changes no byte of the store.
- [ ] **C18** — A store that FileLeafStore::open_read_only refused with StoreError::RepairPending, opened with FileLeafStore::open and Log::open, is repaired and the repair is reported through recovered_to, as on main.
- [ ] **C19** — The module doc of crates/lys-log-store/src/file.rs states what open and open_read_only do and never do, including that open never deletes a leftover temporary file.
