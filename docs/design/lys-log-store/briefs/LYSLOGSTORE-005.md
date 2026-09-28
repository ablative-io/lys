---
type: brief
id: LYSLOGSTORE-005
cluster: lys-log-store
title: The proof tree keeps hashes only, is built by streaming, and never hashes a leaf twice
---

# LYSLOGSTORE-005: The proof tree keeps hashes only, is built by streaming, and never hashes a leaf twice

> **Cluster:** lys-log-store
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C20 — A hash-only proof tree built by streaming (LYSLOGSTORE-005 R1), proved by a counting test that fails at the base.
> **Stories:**
> - S8 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Purpose

The first proof after every start reads every leaf of the log into memory, copies each again into the tree, keeps every leaf's bytes resident for good although proofs need only hashes, and every later append hashes each leaf twice. It grows with the log, and the certificate and identity ledgers serve proofs from it.

## Task

Replace the leaf-holding proof tree in crates/lys-log-store (and the lys-core tree it uses) with a hash-only tree built by streaming, proved by counting tests that fail at the base. Where a file named here has moved or been split on main since 9712505, the fix follows the code to where it now lives and the dev record names the new path.

## Requirements

### R1: A hash-only proof tree built by streaming

Behavioural. crates/lys-log-store/src/frontier_log.rs:250 and :309, crates/lys-core/src/merkle/tree.rs. proof_tree() calls leaves_from(0), collecting every leaf into a Vec, reconstruct_from_raw_leaves copies each again, the tree keeps all leaf bytes for good, and append hashes each new leaf twice (frontier.push and tree.append_raw). Add a tree type that holds only the RFC 6962 leaf and interior hashes, is fed the leaf hashes one at a time from the store, and on append takes the hash frontier.push already computed. Inclusion and consistency proofs are byte-identical to the current tree's.

**Acceptance:**
- A test over a 100 000-leaf log shows proof_tree() holds no leaf bytes (the tree's retained size is hashes only: at most 2 x 32 bytes per leaf) and never holds more than one leaf in memory while building.
- A test shows each append hashes the leaf once, where the base hashes it twice.
- Property tests over random logs show every inclusion and consistency proof equals the base tree's and verifies with lys-core's verifier.

**Files:**
- modify: crates/lys-log-store/src/frontier_log.rs
- modify: crates/lys-core/src/merkle/tree.rs

**Checklist:**
- C20 — A hash-only proof tree built by streaming (LYSLOGSTORE-005 R1), proved by a counting test that fails at the base.

**Stories:**
- S8 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Acceptance 1 (100 000 leaves: hashes only, never more than one leaf held): met. Test `a_100_000_leaf_proof_tree_holds_hashes_only_and_is_built_one_leaf_at_a_time` is at crates/lys-log-store/src/proof_tree_tests.rs:103. It resumes a 100 000-leaf log, starts `Holding::watch`, and calls `proof_tree()`. It asserts: reads == 100000; leaf hashes == 100000; `assert_eq!(log.store().peak.get(), 1, "the build held more than one leaf at once")` (line 112); and `retained <= 2 * 32 * LEAVES` (line 115), where `retained_bytes()` counts all hash capacity. `HashTree` (tree.rs:327) has no field that can hold leaf bytes. The streaming loop is in frontier_log.rs:323. At the base (9712505 / bc2849b) the test does not compile: `crate::leaf_count` is unresolved and `AppendOnlyTree<RawLeaf>` has no `retained_bytes`. The mechanism it would catch is `leaves_from(0)` collecting every leaf before any hashing. If the seam were ported to the base, the peak assertion would read 100000 against 1, and the leaf-hash delta would read 0 against 100000.

Acceptance 2 (each append hashes its leaf once): met. Test `each_append_to_a_built_proof_tree_hashes_its_leaf_once` (proof_tree_tests.rs:124) builds the proof tree, makes 8 appends and asserts `leaf_hashes() - before == 8` (line 133). It also asserts reads stay at 12 (the tree was extended, not rebuilt) and verifies all 20 inclusion proofs. At head, `HashTree::push_leaf_hash` accepts only a `[u8; 32]`, so the tree cannot hash a leaf, and `append` (frontier_log.rs:260) passes it `frontier.push`'s hash. At the base the test does not compile: `leaf_count` is missing and there is no `push_leaf_hash`. The base's second hash is `tree.append_raw(leaf_bytes)`, which hashes inside ct-merkle's `MemoryBackedTree::push`, where no seam in this crate can count it. The count shows head hashes once; the difference from the base is shown by that API change, not by a counted 16.

Acceptance 3 (property tests: every inclusion and consistency proof equals the base tree's and verifies): met. In crates/lys-core/src/merkle/hash_tree_tests.rs, `every_proof_over_every_small_log_is_the_base_trees_and_verifies` (line 102) checks every index and every old size for sizes 1 to 70. `random_proofs_over_random_larger_logs_are_the_base_trees_and_verify` (line 122) covers 24 seeded random logs of up to 2070 leaves. Both compare `as_bytes()` with ct-merkle's `AppendOnlyTree<RawLeaf>`, verify with `verify_inclusion_raw` / `verify_consistency`, and compare roots at every size; case counts are asserted. crates/lys-log-store/src/proof_tree_tests.rs:150 does the same through `FrontierLog::proof_tree()`. The existing frontier-log tests pass unchanged in substance: built once (reads == 12), damaged leaf refused with PinMismatch.

Constraints: no leaf-file, checkpoint, artifact, receipt or bundle format changed (CN1, CN2, CN5). `LeafStore` is untouched (CN3, CN4). No witness items (CN6). tree.rs has 280 counted code lines and frontier_log.rs 221 (CN7). No store layout change (CN8). No new dependency, no clock, no timeout, no allow/ignore attributes, and no `_`-prefixed binding added. Downstream callers (lys-identity restart.rs:384, lys-identity-server certificates_store.rs:310) use only `prove_inclusion`, which has the same signature.
- Deviation: 1) Two things were added beyond the files the brief names. First, `crates/lys-log-store/src/frontier.rs` gains a `hash_leaf` seam with a `#[cfg(test)]` count call, plus a new test-only `leaf_count.rs`: counting leaf hashes needs a single counted hashing point, and this is a test probe, not a lint bypass. Second, `frontier_log_tests.rs` gains an import, because it used `AppendOnlyTree`/`RawLeaf` through the parent's imports, which this change removes. Because that file was touched, the crusher hook refused its pre-existing `.unwrap()` calls, so those were converted mechanically to `?` (and three `_tail` to `_`); no assertion or expected value changed. 2) `HashTree::root()` returns `TrustResult<RootHash>` instead of an infallible `RootHash`. A missing node cannot be looked up without a fallible path, and the library may not panic. 3) The append counting test fails at the base by not compiling, not by a counted 2, because the base's second hash happens inside ct-merkle, which no seam here can count. 4) No file has moved since 9712505, so no new paths needed naming.
- Files changed:
  - modified: `crates/lys-core/src/merkle/tree.rs` — Adds `HashTree` (line 327): the leaf hashes plus the root of every complete perfect subtree, kept per level. It has `new`, `with_capacity` (reserves exactly floor(n/2^h) per level), `len`, `is_empty`, `retained_bytes` (hash storage capacity × 32), `push_leaf_hash` (line 376: takes a hash and hashes only the interior nodes it completes), `root` (TrustResult), `prove_inclusion` (line 422, RFC PATH) and `prove_consistency` (line 448, RFC SUBPROOF). The range checks are factored into `check_inclusion_index` and `check_consistency_sizes`, which `AppendOnlyTree` now also calls, so both trees refuse with the same unchanged messages. Module docs describe the new type.
  - modified: `crates/lys-core/src/merkle/mod.rs` — Re-exports `HashTree` and documents the hash-only tree in the module docs.
  - created: `crates/lys-core/src/merkle/hash_tree_tests.rs` — Property tests against ct-merkle's `AppendOnlyTree<RawLeaf>`, using a seeded splitmix64 generator. Every inclusion and consistency proof is checked for every size from 1 to 70 (2485 of each), plus 24 random logs of 71 to 2070 leaves (1032 inclusion and 1056 consistency proofs). Each proof must be byte-identical to the base tree's and verify with `verify_inclusion_raw` or `verify_consistency`, and the root must match at every size. Also covers extension after appends, the empty-tree root, and out-of-range refusals with the same reason text. An exact retained-bytes check confirms 32·(2n − popcount(n)). Every case count is asserted.
  - modified: `crates/lys-log-store/src/frontier_log.rs` — `proofs` is now `OnceLock<HashTree>`. `proof_tree()` (line 323) builds with `HashTree::with_capacity(len)` by reading one leaf, hashing it with `hash_leaf` and pushing the hash, then repeating; the root check against the frontier is unchanged (PinMismatch). `append` (line 260) calls `tree.push_leaf_hash(leaf_hash)` with the hash `frontier.push` returned, so the leaf is not hashed again. The return type is now `&HashTree`, which has the same `prove_inclusion` and `prove_consistency` signatures. Module docs state the hash-only and streaming invariant.
  - modified: `crates/lys-log-store/src/frontier.rs` — Adds `pub(crate) fn hash_leaf`, the crate's one leaf-hash seam. It calls `raw_leaf_hash` and, in test builds only, counts the call. `Frontier::push` goes through it.
  - created: `crates/lys-log-store/src/leaf_count.rs` — Test-only per-thread count of leaf hashes (`count_leaf_hash` and `leaf_hashes`), used as the counting probe.
  - modified: `crates/lys-log-store/src/lib.rs` — Declares the test-only `leaf_count` module.
  - created: `crates/lys-log-store/src/proof_tree_tests.rs` — Counting tests. `Holding` is a store double: at every leaf read it records how many leaves have been read but not yet hashed, plus the one being read. Tests: the 100 000-leaf build (reads, hashes, held-leaf peak, retained bytes, plus a verified proof); each append hashing once; and a log-level check that every inclusion and consistency proof from `proof_tree()` is byte-identical to the leaf-holding tree's and verifies (249 of each).
  - modified: `crates/lys-log-store/src/frontier_log_tests.rs` — Now imports `AppendOnlyTree` and `RawLeaf` itself, since it used to get them from the parent module's imports, which this change removes. Because the file was touched, the crusher hook refused its pre-existing `.unwrap()` calls. These were converted mechanically to `?`: the tests return `Result`, `filled` returns `Result<Disk, StoreError>`, and three `_tail` bindings became `_`. Every assertion and expected value is unchanged.
- Checklist delivery:
  - [x] C20 — A hash-only proof tree built by streaming (LYSLOGSTORE-005 R1), proved by a counting test that fails at the base. — `HashTree` (crates/lys-core/src/merkle/tree.rs:327) is built by streaming in `FrontierLog::proof_tree` (crates/lys-log-store/src/frontier_log.rs:323). Counting tests are in crates/lys-log-store/src/proof_tree_tests.rs:103 and :124; at the base they fail to compile against the missing API.
- Story delivery:
  - [x] S8 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows. — The first proof after a start now holds no leaf bytes, at most two hashes per leaf, and one leaf at a time while building. Each append hashes its leaf once.

## Boundaries

- SHALL NOT change any answer, refusal, record, wire format, stored byte or audit line: every existing test stays as it is and green; a changed expected value is a defect, not an update.
- SHALL NOT add any timeout, deadline, sleep, poll interval or bound in seconds, and SHALL remove any this brief names (CLAUDE.md, No time limits).
- SHALL NOT add #[allow], #[ignore], a _-prefixed binding or any other bypass; clippy with -D warnings, ast-grep scan and the file-length leg stay green, and no file passes 500 lines of code.
- SHALL NOT add a dependency that is not already in Cargo.lock unless the dev record names it and why nothing in the lock serves.
- SHALL NOT measure speed with a clock in any test: each requirement is proved by counting the work done (reads, parses, scans, syscalls through a counting double, lock acquisitions), never by elapsed time.

## Verification

- cargo fmt --all --check, both clippy legs with -D warnings, cargo test --workspace --all-features --no-fail-fast, both cargo doc legs, ast-grep scan --config sgconfig.yml, sh scripts/design/gate.sh and (once LYSGATE-002 has landed) sh scripts/file-length.sh all exit 0 at the card's head, measured by the card round, never by the builder's own run.
- Every counting test named in an acceptance line fails at the card's base commit and passes at its head: the dev record names each test and quotes its failing assertion at the base.
