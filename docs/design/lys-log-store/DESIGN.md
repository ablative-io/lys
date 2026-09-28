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
- **P5** — The LeafStore trait stays what BACKENDS.md says a backend must satisfy; reading without writing is FileLeafStore's own.

## Decisions

- ADR-059 — A named leaf in the file store is a whole, flushed leaf, and the no-replace link is its commit point — A file under a 20-digit leaf name is always a whole, flushed leaf; only dot-prefixed temporary files in leaves/ may be partial. A leaf is written and flushed in a hidden temporary file, then linked to its final name by an operation that refuses to replace, then the temporary name is removed and leaves/ is flushed. The successful link is the commit point: the extent advances past the leaf whatever happens next. A failed temporary-name removal still returns Ok; a failed leaves/ flush returns StoreError::LeafDurabilityUncertain carrying the index, and the handle refuses further appends with it until reopened. A writable open flushes leaves/ before counting and fails with Io when that flush fails; a read-only open (FileLeafStore::open_read_only) counts the named leaves without flushing; neither counts, deletes or changes a leftover temporary file. Rejected: naming the leaf first and writing into it (today's torn-leaf shape); deleting leftover temporary files at open, which would make a read-only open mutate the store; flushing at a read-only open, which would turn a status on read-only media into an error; and reporting a post-link failure as Io, which left the extent behind and made the next append report the store's own leaf as LeafAlreadyWritten.
- ADR-060 — LeafAlreadyWritten means another writer holds the index, and the store never reports its own leaf as another writer's — LeafAlreadyWritten is returned only when the index is behind the extent or the no-replace link finds the final name taken by an entry this call did not link. Once this call's link succeeds the extent advances past the leaf whatever happens next, so no later step of the same call and no later call on the same handle reports that leaf as LeafAlreadyWritten; a post-link failure that leaves durability in doubt is LeafDurabilityUncertain (ADR-059). Rejected: reporting a post-link failure as Io and leaving the extent behind, which made the next append report the store's own leaf as LeafAlreadyWritten; and treating LeafAlreadyWritten as a resume signal a caller may retry through.
- ADR-100 — The witness keeps a derived per-origin projection in memory and folds only new leaves — The witness's per-origin projection is built by one pass when the anchor is opened. It records how many leaves it has folded, and each observe folds only the leaves recorded since then before comparing, then adds the note it just recorded. The projection lives in memory only and is derived from the leaves, so discarding it and folding again gives the same answer, and it is never authoritative. Observe is bounded for a known origin and a first sighting alike. Rejected: the backward scan, which leaves a first sighting unbounded. Also rejected: any stored index, which would be a second copy of the truth beside the leaves.
- ADR-101 — A file log store's only expectation of its leaves is the pin; a damaged pinned prefix is refused without naming a leaf — The pin is the store's only expectation of its leaves. When the leaves inside the pinned prefix rebuild to another root, Log::open refuses with StoreError::PinMismatch as it does today. That refusal states the pin's tree size and root and the tree size and root recomputed from the leaves, and it never names a leaf index, because which leaf changed is not known. The one-leaf repair for a leaf just past the pin is kept exactly as it is. Rejected: a per-leaf hash record beside the leaves, which changes the layout and BACKENDS.md and adds a crash window. Also rejected: naming a leaf by bisecting the pin's root, which cannot be done. Also rejected: a read-only open of a refused log, and refusing a torn leaf just past the pin, which would turn routine crash recovery into a refusal.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-106 — A FileLeafStore opened read-only refuses to write and to repair; Log and Anchor gain no read-only mode — FileLeafStore gains an inherent open_read_only whose handle refuses put_leaf and pin with StoreError::ReadOnly and which refuses a store exactly one leaf past its pin with StoreError::RepairPending, decided from the count and the pin without reading leaf bytes and without flushing. Log and Anchor gain no read-only mode: Log::open over a read-only handle either finds the tree matching the pin, or refuses with PinMismatch, and never reaches a pin, so the Log needs no mode for the store to hold. Both hold because the refusal of a repair lives at the one place that can know a repair would be needed before any Log sees the store. Rejected: opening a read-only log at its pinned head and reporting the repair as pending (a Log mode, and a served state the store has not reconciled); a read-only mode on Log or Anchor; making read-only opening a LeafStore trait method, which would add a criterion to BACKENDS.md; and a per-leaf hash record so a refusal could name the changed leaf.
- ADR-108 — A Claude Code import builds its session under a staging name and publishes it once — The import command builds its new session under the staging name `<id>.jsonl.importing` beside `sessions/<id>.jsonl`, holding `<id>.lock` throughout: the header and every entry line are written to the staging file with no sync, and the index rows and the head are kept in memory. It then publishes once, in this order: one sync of the staging file; the index written whole to `<id>.index.jsonl` through a temporary file, synced and renamed; the head written once to `<id>.head` through a temporary file, synced and renamed; one sync of the sessions directory; the staging file renamed to `<id>.jsonl`; one more sync of the sessions directory. A crash before the rename leaves no session under the final name, so a re-import is not refused as Exists; the next import of that id and the next open of that session file, each holding the lock, remove the staging file and, when `<id>.jsonl` is absent, the `<id>.index.jsonl` and `<id>.head` a part-way publish left. Rejected: appending in place and leaving a session that refuses by name after a crash; extending reconcile to trim an unsynced tail.

## Goals

- After a put_leaf that fails before its link, no file exists under that leaf's 20-digit name, no temporary file of this writer's remains in leaves/, and a reopen reports the same extent as before the write.
- A reopen with a leftover hidden temporary file in leaves/ counts exactly the named leaves and leaves the temporary file byte-identical.
- A second writer that put_leafs an index another writer has already linked receives LeafAlreadyWritten for that index, the first writer's bytes are unchanged, and no temporary file of the second writer's remains.
- A leaves/ flush failure after the link returns LeafDurabilityUncertain with the index and the flush error as its source, the next put_leaf on that handle is refused with it, and after a reopen the leaf counts and the next put_leaf goes to the following index.
- A read-only open of a directory whose leaves/ flush fails returns the store with the named leaves counted, and a writable open of it returns Io and leaves every file byte-identical.
- put_leaf on a handle from FileLeafStore::open_read_only returns ReadOnly naming the store's directory, and the directory holds no new file.
- Every Anchor::open_read_only call in lys-anchor's doc examples and read-only tests opens its store with FileLeafStore::open_read_only.
- pin on a handle from FileLeafStore::open_read_only returns ReadOnly and leaves state.json byte-identical.
- Anchor::open_read_only over a store one leaf ahead of its pin opens at the pinned head, leaves state.json byte-identical, and its status says that a writable open repairs it; a writable open over the same store repairs and pins.
- Every leg of the gate array passes.
- Opening a file store whose pinned prefix holds a torn or changed leaf fails with StoreError::PinMismatch stating the pin and the recomputed root, pins nothing, and the lys and lys-anchor status commands exit non-zero with that refusal on stderr.
- A torn leaf just past the pin is repaired, pinned and reported exactly as before this change.
- Opening a file store whose leaves/ holds a leftover temporary leaf file returns that file's name to the caller, and the lys and lys-anchor tools print it on one stderr line.
- After the witness's projection is built once, an observe on a 256-leaf log parses one leaf when no other leaf was recorded since the previous observe.
- Every test that exists in lys-log-store and lys-anchor on main passes, with its assertions unchanged.
- scripts/verify_inclusion.py exits 0 on an inclusion artifact and a leaf file from a store written after this change.
- No leaf is served as whole unless it is proven whole against the pinned root through Log::open, and file.rs's module doc says so.
- FileLeafStore::open_read_only refuses put_leaf and pin with StoreError::ReadOnly, and every file under the store directory holds the same bytes before and after.
- FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending, every file under the store directory holds the same bytes before and after, and a writable open of the same store repairs it as main does.
- A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch and state.json is unchanged, under a named test.
- FileLeafStore::leftover_temporaries names the temporary-leaf files an open skipped, and no open removes or changes one.

## Non-Goals

- Clearing leftover hidden temporary files that a crash stranded — A further unit: an explicit maintenance act that runs only while holding the write lock. Open never deletes anything; put_leaf removes only its own temporary file after its own failed write.
- Changing DIRECTORY-003 or its uncertain-append reconciliation — DIRECTORY-003 treats LeafDurabilityUncertain as uncertain and settles by reading the leaf back in its own card; it lands after this one.
- Changing the on-disk layout or the LeafStore trait's methods — The layout is kept byte-identical so earlier stores open, and the trait's absent operations are the crate's reason to exist.
- Directory flushing on non-unix targets — fsync_dir stays a no-op there, as file.rs already says; nothing in this work changes that scope.
- Repairing a store found one leaf ahead of its pin through a read-only open — A pin is a write: pin through a handle from FileLeafStore::open_read_only refuses as ReadOnly, and Anchor::open_read_only opens at the pinned head and reports the pending repair instead. Only Log::open through a writable handle performs the one-leaf-ahead repair.
- Writing a leaf through a temporary name and putting it in place — Main already writes the temporary file, syncs it and hard-links it into place without replacing an existing leaf; a rename would replace another writer's leaf and break write-once.
- A read-only mode for Log or for the anchor — A refused log fails to open; no read-only handle is added, for Log and the anchor; FileLeafStore::open_read_only is added by LYSLOGSTORE-004 under ADR-108.
- Naming which leaf changed when open refuses a log — The pin is one tree size and one root, which cannot identify a single leaf, and no per-leaf hash record is kept (ADR-101). Which leaf changed is not known and is not claimed.
- Refusing a torn leaf just past the pin — The one-leaf repair is kept exactly as it is, since it is what makes crash recovery routine.
- Changing the leaf format, the checkpoint format or the inclusion artifact format — Out of scope; the leaf file stays the raw RFC 6962 preimage.
- Updating BACKENDS.md or the second backend's adoption criteria — Nothing in the LeafStore contract changes, and no per-leaf check is added.
- A per-leaf hash record, or a refusal that names the index of a changed leaf. — The store keeps one tree size and one root (BACKENDS.md); naming the leaf needs a new layout record, which this cluster does not add.
- A read-only mode on Log or on Anchor. — ADR-106: the read-only open is FileLeafStore's alone.
- Moving Anchor::open_read_only and the lys log commands onto FileLeafStore::open_read_only, or printing leftover temporary files from the lys log and lys-anchor commands. — Those are the consumers' read paths, in lys-anchor, lys-anchor-cli and lys, and are a further unit.
- The witness reading only what is new. — LYSLOGSTORE-002 carries the witness half of the integrity row.
- Redoing flush-before-name, the no-replace link, the temporary-sequence parameter or the rename-injection isolation. — All four are on main already.
- Deleting or tidying leftover temporary files. — Open only skips and names them; clearing them is not the store's act.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `crates/lys-log-store/src/error.rs` | StoreError, including the ReadOnly and RepairPending refusals |  |
| `crates/lys-log-store/src/store.rs` | The LeafStore trait contract; unchanged |  |
| `crates/lys-log-store/src/file.rs` | FileLeafStore; open recognises and reports leftover temporary leaf files |  |
| `crates/lys-log-store/src/file_tests.rs` | FileLeafStore tests, including the leftover-temporary report |  |
| `crates/lys-log-store/src/log.rs` | Log over any LeafStore; its open and one-leaf repair are unchanged |  |
| `crates/lys-log-store/src/log_tests.rs` | Log tests, including the torn and changed leaf inside the pin and the torn leaf just past it |  |
| `crates/lys-anchor/src/anchor/read_only.rs` | Anchor::open_read_only; opens its log with Log::open_at_pin, its docs say it never repairs, and its two doc examples open the store with FileLeafStore::open_read_only | LYSLOGSTORE-001 |
| `crates/lys-anchor/src/anchor/read_only_tests.rs` | Anchor::open_read_only's gates; their store is opened with FileLeafStore::open_read_only, and they gain the one-leaf-ahead reader and writable-repair tests | LYSLOGSTORE-001 |
| `crates/lys-anchor/src/anchor/status.rs` | AnchorStatus; gains pending_repair and pending_repair_notice | LYSLOGSTORE-001 |
| `crates/lys-anchor/tests/standalone_is_complete.rs` | its read-only open of the anchor takes a store from FileLeafStore::open_read_only | LYSLOGSTORE-001 |
| `docs/design/lys-log-store/BACKENDS.md` | second-backend adoption criteria; unchanged |  |
| `docs/design/lys-log-store/design.json` | this design |  |
| `docs/design/lys-log-store/DESIGN.md` | Rendered from design.json | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/checklist.json` | the rows this cluster's brief delivers |  |
| `docs/design/lys-log-store/CHECKLIST.md` | Rendered from checklist.json | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/stories.json` | the stories this cluster's brief serves |  |
| `docs/design/lys-log-store/USER-STORIES.md` | Rendered from stories.json | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-001.json` | the brief: a named leaf is whole and flushed | LYSLOGSTORE-001 |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-001.md` | its rendered markdown | LYSLOGSTORE-001 |
| `docs/design/directory/briefs/DIRECTORY-003.json` | the identity directory's contract brief; its depends_on gains LYSLOGSTORE-001 |  |
| `docs/design/directory/briefs/DIRECTORY-003.md` | DIRECTORY-003 rendered from its JSON |  |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-002.json` | the refusal at open, the leftover-temporary report and the bounded witness |  |
| `crates/lys/src/commands/log/store.rs` | the lys log commands' store open; prints the leftover-temporary line |  |
| `crates/lys/tests/log_tests.rs` | lys log binary tests, including the stderr line and the Python verifier run |  |
| `crates/lys-anchor-cli/src/commands/anchor/open.rs` | lys-anchor's anchor open; prints the leftover-temporary line |  |
| `crates/lys-anchor-cli/tests/anchor_cli.rs` | lys-anchor binary tests, including the stderr line |  |
| `crates/lys-anchor/src/witness/projection.rs` | WitnessProjection with its fold position, forward fold and the test-only parse counter |  |
| `crates/lys-anchor/src/witness/projection_tests.rs` | projection tests, including forward fold against a fresh rebuild |  |
| `crates/lys-anchor/src/witness/observe.rs` | observe over a kept projection |  |
| `crates/lys-anchor/src/witness/observe_tests.rs` | observe tests, including the bounded-parse count |  |
| `crates/lys-log-store/src/lib.rs` | Crate root and re-exports; unchanged |  |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-003.json` | The read-only leaf store brief | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-003.md` | Rendered from LYSLOGSTORE-003.json | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-004.json` | the read-only open, its two refusals, the repair rule and the file store's module doc |  |

## Inventory

- `crates/lys-log-store/src/file.rs` — On main: put_leaf_with writes leaves/.<pid>-<index>-<seq>.tmp, syncs it, hard-links it to the 20-digit name, removes the temporary file and syncs leaves/. open syncs leaves/ before counting. contiguous_extent skips every dot-prefixed name without reporting it. 430 code lines.
- `crates/lys-log-store/src/file_tests.rs` — On main: includes a_leftover_temporary_file_is_ignored_at_open and open_leaves_a_leftover_temporary_file_byte_identical.
- `crates/lys-log-store/src/error.rs` — StoreError::PinMismatch's message: 'stored leaves rebuild to tree size {rebuilt_size} with root {rebuilt_root}, but the pinned state is tree size {pinned_size} with root {pinned_root}'. Unchanged by this cluster.
- `crates/lys-log-store/src/store.rs` — The LeafStore contract and PinnedRoot (tree_size, root). Unchanged by this cluster.
- `crates/lys-log-store/src/log.rs` — Log::open rebuilds the tree, reconciles it with the (tree_size, root) pin, repairs exactly one leaf past the pin and reports it through recovered_to, and otherwise returns StoreError::PinMismatch with pinned_size, pinned_root, rebuilt_size and rebuilt_root. Code unchanged by this cluster.
- `docs/design/lys-log-store/BACKENDS.md` — The five adoption criteria for a second LeafStore backend.
- `docs/design/directory/briefs/DIRECTORY-003.json` — The dependent card: R2 commits every identity change through lys-log-store's file storage and reconciles an uncertain write (ID001_AUDIT_FAULTS). depends_on is DIRECTORY-002 only; it lands after LYSLOGSTORE-001.
- `crates/lys/src/commands/log/store.rs` — open() opens FileLeafStore then Log and prints 'recovered interrupted append: state advanced to N' on stderr. A PinMismatch becomes a directory-invalid error carrying the store's message, printed as 'error: ...' on stderr with exit status 1.
- `crates/lys-anchor-cli/src/commands/anchor/open.rs` — open() opens FileLeafStore then Anchor and prints the same recovery line on stderr. A PinMismatch becomes a directory-invalid error carrying the store's message, printed as 'error: ...' on stderr with exit status 1.
- `crates/lys-anchor/src/anchor/read_only.rs` — Anchor::open_read_only opens its log with Log::open over the writable FileLeafStore::open, so it repairs and pins on a read path.
- `crates/lys-anchor/src/anchor/read_only_tests.rs` — Its read_only helper and a_reader_refuses_a_log_with_no_genesis_leaf_exactly_as_open_does pass FileLeafStore::open(dir) to Anchor::open_read_only.
- `crates/lys-anchor/tests/standalone_is_complete.rs` — One Anchor::open_read_only call, taking FileLeafStore::open(dir).
- `crates/lys-anchor/src/anchor/status.rs` — AnchorStatus, #[non_exhaustive]: origin, root, posture and recovered_to; nothing reports a repair that is pending rather than done.
- `crates/lys-log-store/src/log_tests.rs` — On main: a_tampered_leaf_byte_is_detected_at_open, crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it and crash_recovery_does_not_mask_a_tampered_prefix assert the variant only.
- `crates/lys-anchor/src/witness/observe.rs` — observe(anchor, note, proof, context) calls WitnessProjection::rebuild_prefix on every call (line 91).
- `crates/lys-anchor/src/witness/projection.rs` — WitnessProjection: a BTreeMap of origin to OriginState folded from leaves; rebuild and rebuild_prefix fold from index 0; checkpoint_in_leaf is the parse path. Module invariant: derived, rebuildable, nothing stored; last recorded wins.
- `scripts/verify_inclusion.py` — Standalone RFC 6962 verifier: verify_inclusion.py <artifact.json> <leaf-file>, exit 0 when the leaf verifies.
- `docs/design/identity/CONFORMANCE.md` — The only numbered conformance table; it carries no flight-recorder row, and its row 4.3 is a roles row.
- `scripts/design/gate.sh` — Validates every cluster with a design.json, checks its coverage and compares its rendered markdown byte for byte.

## Constraints

- **CN1** — The leaf file stays the raw RFC 6962 preimage: no header, framing or hash is added to it.
- **CN2** — The checkpoint note format, the inclusion artifact format and every receipt and bundle format do not change.
- **CN3** — The LeafStore trait gains no method, and keeps no fork, merge, delete, truncate or rewrite.
- **CN4** — A leaf is put in place only by the existing no-replace hard link; nothing renames over a leaf name.
- **CN5** — The receipt observe returns stays byte-identical to Anchor::submit's for the same note at the same tree state.
- **CN6** — Every witness item stays behind the federation feature, and no item outside it names one.
- **CN7** — No source file goes over 500 lines of code, excluding tests, comments and blank lines.
- **CN8** — Store directories in the lys/log-dir/v1 layout (log.json, state.json, leaves/) written before this change still open.
