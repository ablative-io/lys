# lys-log-store — what was asked, what it means, and what was written

## The words, as they were typed

A sweep of crates/lys-log-store and crates/lys-anchor found one correctness defect and one cost defect. In crates/lys-log-store/src/file.rs, put_leaf (line 245 as main stands) creates the leaf file with create_new and then writes and syncs it; if write_all or sync_all fails, or the process dies between them, the half-written leaf stays on disk, and Log::open in log.rs lines 128 to 137 then counts it as one more leaf, finds the prefix matching the pin, and pins it as a completed append, so a truncated leaf is committed to the tree with no error. In crates/lys-anchor/src/witness/observe.rs line 91, every observe calls WitnessProjection::rebuild_prefix, which parses every leaf in the log (projection.rs line 141) to find one origin's latest checkpoint, so the witness's cost grows with the square of the log's length.

This card makes the leaf store refuse a leaf it cannot prove whole and makes the witness read only what it needs. A leaf is written to a temporary name beside its final one, synced, and renamed into place, with the directory synced after, so a leaf either exists whole or not at all; on open, a leaf whose bytes do not hash to the leaf the tree expects is refused by name, and the log opens read-only rather than pinning it. The witness finds an origin's latest checkpoint by scanning from the newest leaf back and stopping at the first match, or from a per-origin position it keeps, and never re-parses the whole log per observe.

Acceptance: a test that kills the write after part of a leaf proves the next open refuses that leaf by name and does not pin it; a test that leaves a stray temporary file proves open ignores it and says so; a leaf changed by one byte is refused by name on open; a test over a log of a stated length proves observe parses a bounded number of leaves, measured by a counter on the parse path and not by time; every existing test in both crates still passes, and scripts/verify_inclusion.py still verifies an artifact from a store written after the change. Not in scope: the leaf format, the checkpoint format and the artifact format, which do not change.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 27 September 2026, against lys main fa3dd5311e98d1a751c770319e1cfe2570718c76.

## What the survey found, and its angles

The card asks for two fixes. First, the file leaf store must never let a torn leaf become part of the tree: each leaf is written under a temporary name, synced, and put in place only when whole, and on open a leaf whose bytes do not match what the tree expects is refused by name instead of being pinned. Second, the federation witness must stop re-parsing the whole log on every observe (the cost is quadratic today) and find an origin's latest checkpoint by reading only what it needs. The card was filed against fa3dd53, but main now stands at a426b9a, where four lys-log-store commits have already landed the temporary-name write, using a no-replace hard link rather than a rename. What the store still lacks is the per-leaf check at open, the report of a stray temporary file, and a read-only open. The witness half is untouched.

### What the tree holds

- `crates/lys-log-store/src/file.rs` — At fa3dd53, put_leaf (lines 230-272) opens the final leaf name with create_new and then writes and syncs it, which is the defect as described. At a426b9a (main, 658 lines) put_leaf_with writes a hidden temporary file `.<pid>-<index>-<seq>.tmp`, syncs it, hard-links it to the final name so an existing leaf is never replaced, removes the temporary file and syncs leaves/. Open syncs leaves/ before counting. contiguous_extent still skips every entry that starts with '.', silently.
- `crates/lys-log-store/src/log.rs` — reconcile_with_pin (lines 122-146) adopts exactly one extra leaf when the pinned-size prefix rebuilds to the pinned root, and pins it through store.pin. Nothing in it checks a single leaf's hash, because the only expectation it holds is the (tree_size, root) pin. Log has no read-only mode: open always repairs and pins. The refusal by name and the read-only open land here.
- `crates/lys-log-store/src/store.rs` — The LeafStore trait contract (durable on return, write-once, contiguous, monotonic pin, immutable origin) and PinnedRoot, which holds only (tree_size, root). Any per-leaf expected hash is a new fact the contract does not carry today, and BACKENDS.md holds a second backend to this contract.
- `crates/lys-log-store/src/error.rs` — On main it gained LeafDurabilityUncertain, ReopenRequired and LeafTempNamesTaken. A refusal that names a leaf, and a reported stray temporary file, need their own variant or return value here.
- `crates/lys-log-store/src/file_tests.rs` — On main (783 lines) it already has a_leftover_temporary_file_is_ignored_at_open, open_leaves_a_leftover_temporary_file_byte_identical, a_refused_link_leaves_no_temporary_file and others. The acceptance tests for this card sit beside them, and some of them overlap.
- `crates/lys-anchor/src/witness/observe.rs` — Line 91 calls WitnessProjection::rebuild_prefix(anchor, recorded.leaf_index) on every observe, which is the quadratic cost.
- `crates/lys-anchor/src/witness/projection.rs` — rebuild_prefix (lines 136-160) runs checkpoint_in_leaf over indices 0..leaves, and a later index overwrites an earlier one ('last recorded wins'). Its module invariant says 'Derived, rebuildable, and never authoritative. Nothing here is stored.' checkpoint_in_leaf (line 99) is the parse path where the acceptance counter would go.
- `crates/lys-anchor/src/anchor/open.rs` — Anchor holds pub(super) log: Log<S>, signer, config and policy. A per-origin position kept across observes would live here or in a witness-side wrapper. The witness is behind the off-by-default `federation` feature, and no item outside it may name anything inside it.
- `crates/lys-anchor/src/anchor/read_only.rs` — Anchor::open_read_only calls Log::open, so a read-only anchor today still repairs and pins an extra leaf. A log that 'opens read-only rather than pinning it' has to reach this path too.
- `crates/lys-anchor-cli/src/commands/anchor/open.rs and crates/lys/src/commands/log/store.rs` — The two CLI callers of FileLeafStore::open and Log::open. The anchor CLI prints 'recovered interrupted append' to stderr. Whatever a refused leaf or a stray temporary file produces is what an operator sees here.
- `scripts/verify_inclusion.py` — Takes <artifact.json> <leaf-file> and hashes the leaf file's raw bytes as the RFC 6962 preimage. Any per-leaf hash record must therefore sit outside the leaf file, or this acceptance check breaks.
- `docs/design/lys-log-store/BACKENDS.md` — The only document in the cluster: the five adoption criteria for a second backend, and haematite as the blocked candidate. It has no design.json, so scripts/design/gate.sh skips the cluster entirely.

### What was already decided

- docs/design/lys-log-store/BACKENDS.md — Criteria 1-5: durable on return, write-once enforced by the backend, contiguity established at open, every stored leaf readable back, no fork/merge/delete/truncate. Adding a per-leaf check at open adds a criterion that haematite would also have to meet.
- crates/lys-log-store/src/store.rs module docs — LeafAlreadyWritten is a conflict, not a resume signal; catching up happens at open by repairing exactly the one-leaf-ahead state. This card narrows that repair.
- crates/lys-log-store/src/log.rs module docs — Append order is load-bearing: the leaf is stored before the pin advances, so one leaf ahead of the pin is the only divergence a crash can produce and the only one open repairs.
- crates/lys-anchor/src/witness/projection.rs invariants — The projection is derived, rebuildable and never authoritative, and nothing in it is stored; last recorded wins, not largest size. A kept per-origin position has to answer to this.
- docs/design/lys-anchor/DECISIONS.md DP18 — A witness records; it does not audit. observe's check is a report, and nothing from it reaches a signature.
- docs/design/lys-anchor/DECISIONS.md DP19 — Standalone operation is a hard requirement, so the witness speed-up must stay behind the `federation` feature and leave the default build untouched.
- CLAUDE.md 'Wire formats are forever' — The leaf encoding is a versioned wire contract. The on-disk layout is local state (file.rs module docs), but the leaf file must stay the raw RFC 6962 preimage.
- CLAUDE.md coding standards — No file over 500 lines of code excluding tests, comments and whitespace; tests go in sibling *_tests.rs files; no unwrap/expect/panic in library code.

### What was measured

- Commits on main past the card's stated base fa3dd53: 4 (9e0d1fb, 770f234, 1298836, a426b9a), all in lys-log-store: 662 insertions and 56 deletions across error.rs, file.rs and file_tests.rs
- main head per git ls-remote: a426b9a5bfa47abc935497b2caddee800b5d72f0 (the card names fa3dd5311e98d1a751c770319e1cfe2570718c76)
- crates/lys-log-store/src/file.rs on main: 658 lines, 430 of them neither comment nor blank, against a limit of 500 code lines (70 lines of headroom)
- crates/lys-log-store/src/file.rs at fa3dd53: 462 lines; put_leaf at lines 230-272, create_new at 245
- crates/lys-log-store/src/log.rs: 234 lines; the adopt-one-extra-leaf branch is at lines 128-138
- crates/lys-log-store/src/file_tests.rs on main: 783 lines
- #[test] count on main: lys-log-store 47, lys-anchor 141
- crates/lys-anchor/src/witness: observe.rs 155 lines, projection.rs 188, observe_tests.rs 535, projection_tests.rs 246
- Leaves parsed by one observe today: recorded.leaf_index, i.e. every leaf before the new one; across n observes about n²/2 parses
- Callers of rebuild_prefix outside tests: 1 (observe.rs:91); WitnessProjection::rebuild has 0 non-test callers
- Per-leaf hash records the store keeps today: 0; state.json holds tree_size and root_hash only
- Read-only mode on Log: none; Anchor::open_read_only calls Log::open, which repairs and pins
- Design documents in docs/design/lys-log-store: 1 (BACKENDS.md); no design.json, stories.json or briefs/, so gate.sh does not measure the cluster
- Remote branch tips (excluding refs/pull and card-gate) whose objects exist locally: 71 of 221; 150 cannot be read without a fetch
- Highest ids seen in the 71 readable tips: RM-045 and ADR-074; main itself holds RM-016 and ADR-018

### What it means for the other projects

- haematite — It is the intended second LeafStore backend (BACKENDS.md). If the per-leaf check becomes part of the trait contract or of Log::open, haematite gains a sixth adoption criterion on top of its three existing blockers, #11 recovery fails open, #58 append not fsynced and #57 no committed lockfile, and BACKENDS.md should say so.
- aion — The card runs through the aion chain (brief_card → sign-off → card_build_v3 → src_pr → src_land), and its inputs must name the repository, a426b9a (not fa3dd53), the card and the brief.
- method — The lys-log-store cluster is not measured by the method's gate today because it has no design.json. Writing the brief there means creating the cluster in the method's form.

### The decisions it stands on

- ADR-004 (honour) — lys-log-store and lys-anchor stand alone. The fix must not make the store depend on haematite or the witness depend on another anchor, and the witness stays behind `federation` (DP19).
-  (new) — Record where the expected per-leaf hash comes from (a new local-state record beside the leaf files, or the pin alone), and that a leaf failing it is refused by name while the log opens read-only instead of pinning. This changes the LeafStore/Log recovery contract that BACKENDS.md holds backends to.
-  (new) — Record that the witness keeps a derived, in-memory, rebuildable per-origin index, or scans backward with a stated bound, reconciling this with projection.rs's 'nothing here is stored' invariant and DP18.

### What it requires

- The card's base is main as it stands at writing (a426b9a or later), not fa3dd53.
- No crash or failed write at any point in put_leaf leaves a file under a 20-digit leaf name whose bytes are not the whole leaf.
- A leaf never replaces an existing leaf at its index (write-once holds; the no-replace test stays green).
- Opening a store whose leaf at a final name has been truncated returns a refusal that names that leaf's index, and state.json's pin is unchanged afterwards.
- Opening a store with one byte of a leaf changed returns a refusal that names that leaf's index.
- Opening a store with a stray hidden temporary file in leaves/ succeeds, does not count it, leaves it byte-identical, and reports that it is there.
- A test over a log of a stated length N proves observe calls the parse path at most a stated bound of times, asserted on a counter and not on time.
- observe's Relation and previous results are unchanged for every existing observe_tests and projection_tests case, including rollback ('last recorded wins').
- All 47 lys-log-store tests and 141 lys-anchor tests existing on main still pass under cargo test --workspace --all-features.
- scripts/verify_inclusion.py exits 0 on an artifact and leaf file from a store written after the change.
- fmt, both clippy runs, tests, both doc runs and scripts/design/gate.sh are clean; no source file goes over 500 code lines.

### What must not change

- The leaf file stays the raw RFC 6962 preimage: no header, framing or hash inside it.
- The checkpoint note format, the inclusion artifact format and every receipt or bundle format do not change.
- The LeafStore trait keeps no fork, merge, delete, truncate or rewrite.
- The receipt that observe returns stays byte-identical to Anchor::submit's.
- No witness item reaches a signature or an admission decision, and nothing outside `federation` names a witness item.
- The append order (leaf durable before the pin advances) is not reversed.
- Existing store directories written by the lys-log-store 0.2.0 layout (log.json, state.json, leaves/) still open.

### What we must put in place first

- Fetch or ls-remote the 150 remote tips not held locally and read their roadmap.json and decisions.json, so the next free RM, ADR and brief ids are known. Readable tips show RM-045 and ADR-074.
- Give docs/design/lys-log-store the method's cluster files (design.json and what validate.py and check-coverage.py require) so gate.sh measures the brief.
- Rebase the card's base from fa3dd53 to current main a426b9a.

### The risks

- Using rename as the words say would silently re-open the second-writer overwrite that main's hard link closes.
- A per-leaf hash record written separately from the leaf introduces a new crash window (record written, leaf not, or the reverse) that recovery must handle, or open refuses a healthy log.
- Refusing the one-leaf-ahead state more strictly could turn today's routine crash recovery into a refusal, so logs that open now would stop opening.
- A backward scan still parses the whole log for a new origin; if the bound is measured only on repeat origins, the acceptance passes while the quadratic case remains.
- A kept per-origin index that falls out of step with the leaves becomes a second copy of the truth, the defect projection.rs warns about.
- file.rs has 70 code lines of headroom, so the check and the scan may push it past the 500-line gate unless it is split.
- Ids chosen from the 71 readable tips alone may collide with ids on the 150 unread ones.

### Still open

- Main (a426b9a) has already landed the temporary-name write. Does this card rebase onto main and drop that half, so that it carries only the open-time check, the stray-temporary report and the witness? The sentence of the words it stands on: "In crates/lys-log-store/src/file.rs, put_leaf (line 245 as main stands) creates the leaf file with create_new and then writes and syncs it; if write_all or sync_all fails, or the process dies between them, the half-written leaf stays on disk, and Log::open in log.rs lines 128 to 137 then counts it as one more leaf, finds the prefix matching the pin, and pins it as a completed append, so a truncated leaf is committed to the tree with no error.". Why only the lead can settle it: The tree contradicts it. At a426b9a, put_leaf_with in crates/lys-log-store/src/file.rs writes .<pid>-<index>-<seq>.tmp, syncs it and hard-links it into place, and open syncs leaves/ first. The defect as described no longer exists on main, and the card was filed against fa3dd53.
- Should the leaf be put in place by a rename, as the words say, or by main's no-replace hard link, given that a rename silently replaces a leaf another writer already put at that index? The sentence of the words it stands on: "A leaf is written to a temporary name beside its final one, synced, and renamed into place, with the directory synced after, so a leaf either exists whole or not at all; on open, a leaf whose bytes do not hash to the leaf the tree expects is refused by name, and the log opens read-only rather than pinning it.". Why only the lead can settle it: The tree contradicts it. std::fs::rename overwrites an existing destination, which breaks write-once (store.rs contract 2). Main uses std::fs::hard_link in link_leaf precisely because it refuses an existing name, and commit a426b9a proves that swapping in rename makes no_replace_link_alone_refuses_a_leaf_this_store_never_saw fail.
- Where does 'the leaf the tree expects' come from? Should the store start keeping a per-leaf hash record (a new local-state file beside the leaves), or should the refusal cover only leaves inside the pinned prefix, found by bisecting against the pin? The sentence of the words it stands on: "A leaf is written to a temporary name beside its final one, synced, and renamed into place, with the directory synced after, so a leaf either exists whole or not at all; on open, a leaf whose bytes do not hash to the leaf the tree expects is refused by name, and the log opens read-only rather than pinning it.". Why only the lead can settle it: Nothing in the tree records an expected hash per leaf. state.json (crates/lys-log-store/src/file.rs LogState) holds only (tree_size, root), so a changed leaf inside the pin surfaces as PinMismatch with no leaf named. For the one leaf past the pin (log.rs:128) there is no expectation at all. The answer changes the store's layout, what the operator is told, and what BACKENDS.md requires of haematite.
- When a leaf is refused, does opening fail with an error that names it, as PinMismatch does today, or does it succeed and hand back a handle that can read but not append, and if so, what may that handle serve? The sentence of the words it stands on: "A leaf is written to a temporary name beside its final one, synced, and renamed into place, with the directory synced after, so a leaf either exists whole or not at all; on open, a leaf whose bytes do not hash to the leaf the tree expects is refused by name, and the log opens read-only rather than pinning it.". Why only the lead can settle it: Log has no read-only mode (crates/lys-log-store/src/log.rs). Anchor::open_read_only (crates/lys-anchor/src/anchor/read_only.rs) still repairs and pins. Whether `lys log` and `lys-anchor` status keep answering on a damaged log is behaviour an operator sees.
- On main, a write killed partway leaves only a hidden temporary file and no leaf name, so there is nothing for open to refuse. Should the acceptance test instead plant a torn file at a final leaf name, as a store written before the change could hold, and prove open refuses it? The sentence of the words it stands on: "Acceptance: a test that kills the write after part of a leaf proves the next open refuses that leaf by name and does not pin it; a test that leaves a stray temporary file proves open ignores it and says so; a leaf changed by one byte is refused by name on open; a test over a log of a stated length proves observe parses a bounded number of leaves, measured by a counter on the parse path and not by time; every existing test in both crates still passes, and scripts/verify_inclusion.py still verifies an artifact from a store written after the change.". Why only the lead can settle it: The tree contradicts the test as written. With write_leaf_temp and link_leaf on main, a killed write never produces a leaf name, and a_leftover_temporary_file_is_ignored_at_open already covers what remains. The test can only show a refusal by planting a torn leaf by hand.
- Must observe be bounded even for an origin it has never seen? If so, the witness needs a per-origin index built once (at open, or kept as leaves append), because a backward scan still parses the whole log for a first sighting. The sentence of the words it stands on: "The witness finds an origin's latest checkpoint by scanning from the newest leaf back and stopping at the first match, or from a per-origin position it keeps, and never re-parses the whole log per observe.". Why only the lead can settle it: The two mechanisms the words offer do not meet the acceptance equally. For a new origin, a backward scan parses every leaf before the note, so 'observe parses a bounded number of leaves' fails under it; only a kept index passes. projection.rs also says 'Nothing here is stored', which a kept index would have to square with, by staying in memory and derived.
- How does open 'say so' about a stray temporary file: a value returned to the caller only, like recovered_to, or also a line the lys and lys-anchor CLIs print on stderr? The sentence of the words it stands on: "Acceptance: a test that kills the write after part of a leaf proves the next open refuses that leaf by name and does not pin it; a test that leaves a stray temporary file proves open ignores it and says so; a leaf changed by one byte is refused by name on open; a test over a log of a stated length proves observe parses a bounded number of leaves, measured by a counter on the parse path and not by time; every existing test in both crates still passes, and scripts/verify_inclusion.py still verifies an artifact from a store written after the change.". Why only the lead can settle it: Today contiguous_extent skips every '.' entry silently (crates/lys-log-store/src/file.rs), and the anchor CLI prints recovery on stderr (crates/lys-anchor-cli/src/commands/anchor/open.rs:121). Whether operators see a new line is an output change.

### The units beyond the first

- Update BACKENDS.md and haematite's adoption criteria for the per-leaf check at open — It changes what a second backend must satisfy, which is a separate document change with its own estate reader (haematite) and is not needed for the file store fix to land.
- Surface refused leaves and stray temporary files in the lys and lys-anchor CLIs' status output — Only if the lead decides the report goes beyond a library return value; that is operator-facing output with its own tests.

### The smallest complete shape

One card on current main, in two parts. In lys-log-store, open refuses by name any leaf whose bytes fail the expected hash and does not pin it, and it reports stray temporary files; the temporary-name write already on main is kept as it stands, with its no-replace link. In lys-anchor, observe finds an origin's latest checkpoint without folding the whole log. The card also carries the acceptance tests the words list and the lys-log-store cluster's method files and brief. Neither part is useful to an operator without the other's tests green, and they share one gate run.

## The roadmap row

- **RM-062** — Refuse a damaged pinned prefix at open, report leftover temporary leaf files, and bound the witness's observe (fix, idea)
- Summary: The file log store could let a torn leaf into the tree. On main the torn write is already closed by the temporary-name write and its no-replace hard link, which are kept. LYSLOGSTORE-002 holds the rest under tests: a torn or changed leaf inside the pinned prefix is refused with StoreError::PinMismatch, which states the pin and the recomputed root and names no leaf (ADR-101), and the one-leaf repair just past the pin stays as it is. Open also returns the leftover temporary names, and the lys and lys-anchor tools print them on one stderr line. The federation witness's observe parses only the leaves recorded since its previous call, through an in-memory per-origin projection built once (ADR-100).
- Asked by: tom on 2026-09-27T22:43:00+10:00
- Context: A card for the identity line on the Lys board, filed against lys main fa3dd53 and briefed against main a426b9a. The lead's answers to the card's survey rebased it onto main and dropped the write half. They replaced the rename with main's no-replace hard link and withdrew the read-only open. They planted the torn leaf at a final name instead of killing a write, and chose an in-memory per-origin index for the witness. They had open report leftover temporary files both to the caller and on the two tools' stderr. Answering the author, the lead ruled that a refusal is PinMismatch stating the pin and the recomputed root and never a leaf index. The torn leaf is planted inside the pin, and the one-leaf repair just past it is kept and asserted.
- Quote: A sweep of crates/lys-log-store and crates/lys-anchor found one correctness defect and one cost defect. In crates/lys-log-store/src/file.rs, put_leaf (line 245 as main stands) creates the leaf file with create_new and then writes and syncs it; if write_all or sync_all fails, or the process dies between them, the half-written leaf stays on disk, and Log::open in log.rs lines 128 to 137 then counts it as one more leaf, finds the prefix matching the pin, and pins it as a completed append, so a truncated leaf is committed to the tree with no error. In crates/lys-anchor/src/witness/observe.rs line 91, every observe calls WitnessProjection::rebuild_prefix, which parses every leaf in the log (projection.rs line 141) to find one origin's latest checkpoint, so the witness's cost grows with the square of the log's length.

This card makes the leaf store refuse a leaf it cannot prove whole and makes the witness read only what it needs. A leaf is written to a temporary name beside its final one, synced, and renamed into place, with the directory synced after, so a leaf either exists whole or not at all; on open, a leaf whose bytes do not hash to the leaf the tree expects is refused by name, and the log opens read-only rather than pinning it. The witness finds an origin's latest checkpoint by scanning from the newest leaf back and stopping at the first match, or from a per-origin position it keeps, and never re-parses the whole log per observe.

Acceptance: a test that kills the write after part of a leaf proves the next open refuses that leaf by name and does not pin it; a test that leaves a stray temporary file proves open ignores it and says so; a leaf changed by one byte is refused by name on open; a test over a log of a stated length proves observe parses a bounded number of leaves, measured by a counter on the parse path and not by time; every existing test in both crates still passes, and scripts/verify_inclusion.py still verifies an artifact from a store written after the change. Not in scope: the leaf format, the checkpoint format and the artifact format, which do not change.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 27 September 2026, against lys main fa3dd5311e98d1a751c770319e1cfe2570718c76.
- Cluster: lys-log-store; briefs: LYSLOGSTORE-002
- Notes: Ids are the next free past main a426b9a and every open brief, draft and hand branch. The draft's commits sit on main a426b9a. git ls-remote was read immediately before each of rounds 1 and 2, and each time every one of the 239 branch tips it listed, outside refs/pull and card-gate, was fetched and read, none unreadable. The highest ids held on any other tip were, both times, RM-061 and ADR-099 (draft/directory/844a587d, which held RM-058 and ADR-097 when the lead read it, now holds RM-059 and ADR-098), and LYSLOGSTORE-001 was the highest other brief under the prefix. The words give no first-writer priority, so this row, first written as RM-058 with ADR-097 and ADR-098, is renumbered to RM-062 with ADR-100 and ADR-101, and it keeps LYSLOGSTORE-002. Further units, not written: Update BACKENDS.md and haematite's adoption criteria for the per-leaf check at open (under ADR-101 no per-leaf check is added, so nothing in it is owed yet); Surface refused leaves and stray temporary files in the lys and lys-anchor CLIs' status output (the lead's answers brought the stderr line and the refusal's exit into LYSLOGSTORE-002; anything beyond them stays here).

## The design

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

The pin is the store's only expectation of its leaves: one tree size and one root (ADR-101). No per-leaf hash record is added. When the leaves inside the pinned prefix, one of them torn or changed, rebuild to a root other than the pinned one, Log::open refuses with StoreError::PinMismatch as it does today. The refusal states the pin's tree size and root and the tree size and root recomputed from the leaves. A single root cannot tell which leaf changed, so the refusal does not name one and nothing claims to. Nothing is pinned and state.json is left as it was. The one-leaf repair for a leaf just past the pin is kept exactly as it is, since it is what makes crash recovery routine: such a leaf, torn or whole, is adopted, pinned and reported through recovered_to. The lys log commands and the lys-anchor commands already turn the refusal into a stderr diagnostic and a non-zero exit, and tests now hold them to it. No read-only open is added.

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

## Goals

- Opening a file store whose pinned prefix holds a torn or changed leaf fails with StoreError::PinMismatch stating the pin and the recomputed root, pins nothing, and the lys and lys-anchor status commands exit non-zero with that refusal on stderr.
- A torn leaf just past the pin is repaired, pinned and reported exactly as before this change.
- Opening a file store whose leaves/ holds a leftover temporary leaf file returns that file's name to the caller, and the lys and lys-anchor tools print it on one stderr line.
- After the witness's projection is built once, an observe on a 256-leaf log parses one leaf when no other leaf was recorded since the previous observe.
- Every test that exists in lys-log-store and lys-anchor on main passes, with its assertions unchanged.
- scripts/verify_inclusion.py exits 0 on an inclusion artifact and a leaf file from a store written after this change.

## Non-Goals

- Writing a leaf through a temporary name and putting it in place — Main already writes the temporary file, syncs it and hard-links it into place without replacing an existing leaf; a rename would replace another writer's leaf and break write-once.
- A read-only mode for Log or for the anchor — A refused log fails to open; no read-only handle is added.
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
> - C8 — Log::open over a store whose leaf inside the pinned prefix is torn returns StoreError::PinMismatch stating the pin's tree size and root and the recomputed tree size and root, names no leaf, and leaves state.json unchanged.
> - C9 — Log::open over a store whose leaf inside the pinned prefix has one byte changed returns the same refusal, naming no leaf and leaving state.json unchanged.
> - C10 — Log::open over a store with a torn leaf at the index just past the pin repairs and pins it and reports the repair through recovered_to, exactly as before this change.
> - C1 — FileLeafStore::open returns, sorted, the name of every entry in leaves/ that has the store's temporary-leaf form, does not count any of them as a leaf, and leaves each byte-identical.
> - C11 — lys log status on a log whose pinned prefix holds a torn leaf exits 1 and prints the refusal on stderr.
> - C2 — Every lys log command that opens a store prints one stderr line naming the leftover temporary leaf files the store reported.
> - C4 — scripts/verify_inclusion.py exits 0 on an inclusion artifact and a leaf file from a store written after this change.
> - C3 — Every lys-anchor command that opens an anchor prints one stderr line naming the leftover temporary leaf files the store reported.
> - C12 — lys-anchor status on an anchor whose pinned prefix holds a torn leaf exits 1 and prints the refusal on stderr.
> - C5 — WitnessProjection records how many leaves it has folded and folds forward from that position only.
> - C6 — observe parses at most one leaf more than the number of leaves recorded since the previous observe through the same projection, counted on the parse path.
> - C7 — After every observe, the kept projection reports the same latest state for every origin as WitnessProjection::rebuild over the same anchor, including after a rollback.
> **Stories:**
> - S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.
> - S1 (Log operator, Opens a log store, including after a crash) — As a log operator, I want to be told which leftover temporary files opening my store ignored, so that what an interrupted append left behind is visible rather than skipped in silence.
> - S2 (Third-party verifier, Checks a leaf without lys) — As a third-party verifier, I want a leaf from a store written after this change to verify with the standalone Python verifier, so that checking a leaf still needs no lys code.
> - S3 (Witness operator, Runs a witness anchor that observes other logs' checkpoints) — As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.

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
- C8 — Log::open over a store whose leaf inside the pinned prefix is torn returns StoreError::PinMismatch stating the pin's tree size and root and the recomputed tree size and root, names no leaf, and leaves state.json unchanged.
- C9 — Log::open over a store whose leaf inside the pinned prefix has one byte changed returns the same refusal, naming no leaf and leaving state.json unchanged.
- C10 — Log::open over a store with a torn leaf at the index just past the pin repairs and pins it and reports the repair through recovered_to, exactly as before this change.

**Stories:**
- S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

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
- C1 — FileLeafStore::open returns, sorted, the name of every entry in leaves/ that has the store's temporary-leaf form, does not count any of them as a leaf, and leaves each byte-identical.

**Stories:**
- S1 (Log operator, Opens a log store, including after a crash) — As a log operator, I want to be told which leftover temporary files opening my store ignored, so that what an interrupted append left behind is visible rather than skipped in silence.

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
- C2 — Every lys log command that opens a store prints one stderr line naming the leftover temporary leaf files the store reported.
- C4 — scripts/verify_inclusion.py exits 0 on an inclusion artifact and a leaf file from a store written after this change.
- C11 — lys log status on a log whose pinned prefix holds a torn leaf exits 1 and prints the refusal on stderr.

**Stories:**
- S1 (Log operator, Opens a log store, including after a crash) — As a log operator, I want to be told which leftover temporary files opening my store ignored, so that what an interrupted append left behind is visible rather than skipped in silence.
- S2 (Third-party verifier, Checks a leaf without lys) — As a third-party verifier, I want a leaf from a store written after this change to verify with the standalone Python verifier, so that checking a leaf still needs no lys code.
- S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

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
- C3 — Every lys-anchor command that opens an anchor prints one stderr line naming the leftover temporary leaf files the store reported.
- C12 — lys-anchor status on an anchor whose pinned prefix holds a torn leaf exits 1 and prints the refusal on stderr.

**Stories:**
- S1 (Log operator, Opens a log store, including after a crash) — As a log operator, I want to be told which leftover temporary files opening my store ignored, so that what an interrupted append left behind is visible rather than skipped in silence.
- S4 (Log operator, Opens a log store, including after a crash) — As a log operator, I want a store whose pinned leaves no longer rebuild to the pin to refuse to open and tell me the pin and the root the leaves give, so that a damaged leaf is never committed to the tree and I am never told more than the store can prove.

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
- C5 — WitnessProjection records how many leaves it has folded and folds forward from that position only.

**Stories:**
- S3 (Witness operator, Runs a witness anchor that observes other logs' checkpoints) — As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.

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
- C6 — observe parses at most one leaf more than the number of leaves recorded since the previous observe through the same projection, counted on the parse path.
- C7 — After every observe, the kept projection reports the same latest state for every origin as WitnessProjection::rebuild over the same anchor, including after a rollback.

**Stories:**
- S3 (Witness operator, Runs a witness anchor that observes other logs' checkpoints) — As a witness operator, I want each observation to cost only the leaves recorded since the previous one, so that the witness stays usable as its own log grows.

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

