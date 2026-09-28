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
