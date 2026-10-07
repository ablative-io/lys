---
type: brief
id: LYSLOGSTORE-009
cluster: lys-log-store
title: Lys's stores meet the estate's six on their own engine: no full-history open, kept copies and dead outcomes go, and a bound test holds the size
---

# LYSLOGSTORE-009: Lys's stores meet the estate's six on their own engine: no full-history open, kept copies and dead outcomes go, and a bound test holds the size

> **Cluster:** lys-log-store
> **Depends on:** LYSLOGSTORE-007
> **Design anchor:**
> - ADR-138 — Lys's log stays on lys-log-store's own segment engine; haematite stays the intended second backend, on named conditions — Lys's log keeps lys-log-store's own segment engine. Why, by name: 1. Nothing in a Lys log is superseded. A record is appended once and never rewritten (LYSLOGSTORE-008), so haematite's reclaim of superseded pages has nothing to reclaim here. Lys's unreachable bytes are whole files (kept migration copies, unreferenced home blocks), and removing them is an unlink. 2. Lys acknowledges a leaf only after the flush that covers it. Haematite's ordinary stores do not: native production runs FsyncPolicy::CommitOnly (crates/haematite/src/shard/actor/native/boot.rs:577 at af5e18a) and N1 stands. Haematite's STOR-012 LeafLog, written for Lys's log and sealed from that path by its ADR-040, does: LeafLog::append takes one contiguous batch with its pin and flushes its frame before returning (crates/haematite/src/api/leaf_log.rs:177-178, present at tag v0.15.0, one device flush per act counted in api/leaf_log_count_tests.rs). So this is not a reason against the move; it says the move is onto LeafLog and never onto an ordinary store. (Corrected 7 October on Apollo's post 446b2b0e.) 3. A Lys log is evidence a verifier reads without the server: the segment format, its CRCs and the tiles of LYSLOGSTORE-006 are read by lys log and the anchor with no database engine. 4. Haematite's own criteria 2, 3, 5 and 6 are written but not built or measured (Apollo, 7 October 12:22, post 6d2037f7: prune designed in STOR-013, startup still reads in proportion to history until format 4, no bound test, not installed). Lys would wait on them for what its own engine does with an unlink. Haematite under Lys's log is in scope and gets done, by Apollo's team (Tom, 3 October, rule 11). The move is scheduled haematite work now, not a waiting list (Waffles, 7 October, post 1afd2474). It waits on four conditions, each a named haematite brief: 1. per-append durability, meaning an acknowledgement only after the flush that covers it: met by STOR-012's LeafLog in haematite 0.15.0, with its flushes counted, and its start-up read bound (leaf_log_open_bound.rs) in 0.15.1 once published; 2. prune and online reclaim installed and measured (STOR-013 and STOR-014); 3. a start-up that reads no history (STOR-015); 4. a green bound test (STOR-014). When all four are green at a named haematite commit, the Lys log move gets its own Lys brief, and that brief goes into the next Lys piece. Any Lys store that comes to hold values rewritten in place, rather than appended, goes on haematite and not into the log.
> - ADR-134 — Leaves and the pin are appended together to checksummed segment files, with one flush per append — A log's leaves are appended as length-prefixed, checksummed records to segment files rolled at a fixed size. Each record carries the pin it makes, so the leaf and the pin are written by one append and made durable by one flush; state.json is no longer rewritten. Appends given together share one flush. A store in the per-file layout is migrated once, on its first writable open.
> **Checklist:**
> - C39 — Every name Lys creates under its data path has a row with its creator, contract, open, growing collections and bytes per act, and a test holds the list whole (LYSLOGSTORE-009 R1).
> - C40 — No production open reads every leaf, and Log with its full-leaf opens is deleted (LYSLOGSTORE-009 R2).
> - C41 — The runner's operations open from a checkpoint and their live outcomes, rewritten with one flush (LYSLOGSTORE-009 R3).
> - C42 — A migration's kept copy is removed while running once a later start has checked it, and a mismatch is refused and kept (LYSLOGSTORE-009 R4).
> - C43 — A bound test drives a stated mix to 2,000 sessions and measures the data path from outside, red at main (LYSLOGSTORE-009 R5).
> - C44 — Every growing in-process collection holds only live records or contract-kept ids (LYSLOGSTORE-009 R6).
> - C45 — Installed, with the data path's levels posted over seven days beside the acts counted (LYSLOGSTORE-009 R7).
> **Stories:**
> - S12 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want every Lys store to stay within a bound I can measure from outside while it runs, so that the disk never fills with bytes nobody can reach and no start or command reads all of history.

## Purpose

Tom, 7 October, through Waffles (a3f69eae): every app's stores need a long-term solution very soon, judged by six criteria. They are: online reclaim, retention as one batch with one flush, no re-replay, bounded caches, a bound test red today, and installed and measured. ADR-138 records why Lys's log stays on its own segment engine. This brief is how Lys meets the six there. Read on 7 October: the live data directory was 8 MiB at 12:23 (du, sizes only, no file opened, data.previous outside the walk). Server stores open from their tail (FrontierLog::open). The lys log commands and the anchor's read-only open read every leaf (Log::open; LYSLOGSTORE-007, written, not built). The runner's operations.jsonl is folded whole on every open and never shortened (operations.rs:204-262). Seventeen kept v1 copies stay for good by design (migrate.rs:1-26). No test measures any of it. The criteria map onto the requirements like this. 1, online reclaim, is R4 and R3: Lys's unreachable bytes are whole files, and nothing in a log is superseded (ADR-138). 2, retention: every log leaf is evidence under a signed root, kept for good by contract. The records that end are R3's outcomes and R4's copies, each ended by one act with one flush. 3, no re-replay, is R2 after 007, plus R3. 4, bounded caches, is R6: Lys has no engine cache, so the bound is every growing in-process collection. 5, the bound test, is R5. 6, installed and measured, is R7.

## Task

After LYSLOGSTORE-006 and 007 are built: list every store with its contract (R1); delete Log and its full-leaf open (R2); open the runner's operations from a checkpoint (R3); remove each migration's kept copy once proved (R4); write the bound test, red at main first (R5); bound every growing collection (R6); install and read the levels for a week (R7). Out of scope: moving any store onto haematite (ADR-138 names when); any retention period for evidence, which is Tom's to set; the install's .previous directories, which each install replaces.

## Requirements

### R1: Every store under Lys's data path has a row, and a test holds the list whole

Behavioural. docs/design/lys-log-store/STORES.md holds one row for each name Lys creates under its data path. That covers every *_dir and *_file that server_config.rs renders, the runner's directory (feed.jsonl, feed.index.json, operations.jsonl, sessions, sessions.json), runner-acts, the proxy's home (blocks, sessions) and state, agent-keys, canvas, estate-approval.json, and each kept <name>.v1 copy that LYSLOGSTORE-008's migration leaves. Each row gives: the crate and function that creates it; what it holds; its retention contract, which is either evidence kept for good, naming why, or the event that ends a record; how it opens, by file and line; every in-process collection that grows with its records, by type and field, and what bounds it; and its bytes per act, measured by R5. A row whose answer is not known says not checked, and the card does not finish with one. tests/store_inventory.rs starts the identity server, the runner and the proxy from a fresh install layout in a temporary directory, runs one act of each kind the R5 mix uses, lists every name under the data path, and refuses any name with no row and any row with no name.

**Acceptance:**
- store_inventory fails when a test creates an unlisted directory under the data path, naming it.
- store_inventory fails when a row names a path nothing creates, naming the row.
- Every row's retention, open and collection columns cite a file and line at the card's head.

**Files:**
- create: docs/design/lys-log-store/STORES.md
- create: crates/lys/tests/store_inventory.rs
- modify: crates/lys/src/identity/install/server_config.rs

**Checklist:**
- C39 — Every name Lys creates under its data path has a row with its creator, contract, open, growing collections and bytes per act, and a test holds the list whole (LYSLOGSTORE-009 R1).

**Stories:**
- S12 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want every Lys store to stay within a bound I can measure from outside while it runs, so that the disk never fills with bytes nobody can reach and no start or command reads all of history.

### R2: Nothing opens a log by reading every leaf, and Log goes

Behavioural. Production stores already open with FrontierLog::open and read the tail. Two production paths do not: lys log (crates/lys/src/commands/log/store.rs:60) and the anchor's read-only open (crates/lys-anchor/src/anchor/read_only.rs:192), used by lys-anchor-cli. Both call Log::open, which reads every leaf (crates/lys-log-store/src/log.rs:109-118), and LYSLOGSTORE-007 moves them onto the tiles of LYSLOGSTORE-006. After 007, Log, Log::open and Log::open_at_pin have no production caller. They are deleted with crates/lys-log-store/examples/restart_timing.rs. lys-identity's revocation append functions (revocation/append.rs) take FrontierLog. Each case in log_tests.rs that proves a behaviour FrontierLog keeps (divergence, repair, pinned recovery) moves to frontier_log_tests.rs, and the handback lists each one moved and each one dropped with its reason. A counting LeafStore proves that every production open reads no leaf below the pinned tail.

**Acceptance:**
- git grep finds no Log::open, Log::open_at_pin or Log< outside a comment at the card's head.
- Opening a 100,000-leaf store through each production open reads at most the unpinned tail's leaves, counted by the counting store; this fails at main for lys log and the anchor.
- The handback's moved and dropped list accounts for every test in log_tests.rs at main.

**Files:**
- modify: crates/lys-log-store/src/lib.rs
- modify: crates/lys-log-store/src/error.rs
- modify: crates/lys-log-store/src/frontier_log.rs
- modify: crates/lys-log-store/src/frontier_log_tests.rs
- modify: crates/lys-identity/src/revocation/append.rs
- delete: crates/lys-log-store/src/log.rs
- delete: crates/lys-log-store/src/log_tests.rs
- delete: crates/lys-log-store/examples/restart_timing.rs

**Checklist:**
- C40 — No production open reads every leaf, and Log with its full-leaf opens is deleted (LYSLOGSTORE-009 R2).

**Stories:**
- S12 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want every Lys store to stay within a bound I can measure from outside while it runs, so that the disk never fills with bytes nobody can reach and no start or command reads all of history.

### R3: The runner's operations open from their live outcomes, not from every outcome ever

Behavioural. Operations::open_at (crates/lys-runner/src/operations.rs:204-262) folds every line of operations.jsonl on each open, and nothing ever shortens the file except a torn tail. Operations::prune (:343) drops expired texts from memory only. The file is rewritten when the outcomes prune has ended outnumber the live ones. The rewrite holds one checkpoint line, which carries every id in seen and nothing else, followed by the live outcomes. It is written as one write and one flush to a temporary file, then renamed, then its directory is flushed. Open reads the checkpoint and the lines after it. seen keeps every operation id for good, by design: a reused id is refused at :354-362. Its bytes per operation go into R1's row. The operations.json reader at :206-225, the format before operations.jsonl, is removed (no backwards compatibility; the live runner has no operations.json, by name listing at 12:31 on 7 October).

**Acceptance:**
- After 10,000 operations of which 10 are live, operations.jsonl holds the checkpoint and 10 outcomes, and open folds 10 outcomes; this fails at main.
- A crash at each write, flush and rename of the rewrite leaves either the old file or the new file whole, and a reused id is still refused after either.
- The rewrite costs one data flush and one directory flush, counted.

**Files:**
- create: crates/lys-runner/src/operations_tests.rs
- modify: crates/lys-runner/src/operations.rs

**Checklist:**
- C41 — The runner's operations open from a checkpoint and their live outcomes, rewritten with one flush (LYSLOGSTORE-009 R3).

**Stories:**
- S12 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want every Lys store to stay within a bound I can measure from outside while it runs, so that the disk never fills with bytes nobody can reach and no start or command reads all of history.

### R4: A migration's kept copy is removed while Lys runs, once a later start has proved its replacement

Behavioural. LYSLOGSTORE-008's migration renames each v1 directory to <name>.v1 and keeps it, never deleted (crates/lys-log-store/src/migrate.rs:1-26). The live install holds 17 of them. A kept copy is removed by the first writable open after the start that migrated it, and only when that open has checked the migrated store's extent and pin. The open also checks that every leaf under the kept copy's pin hashes to the root the migrated store pinned at migration. The check reads the kept copy once, and that is the one read of its history. The removal is: rename <name>.v1 to <name>.v1.removing, flush the parent directory, remove the tree, flush the parent again. An open that finds a .v1.removing finishes it. A kept copy whose leaves do not match is refused by name and kept. Leaves past the v1 pin, which the migration reported and did not move, are named in the refusal, and that copy is kept for a ruling. The start record counts each copy removed and each copy kept.

**Acceptance:**
- Migrate, start again: the kept copy is gone, and the start record counts it.
- Migrate with one changed byte in the kept copy, start again: the copy is kept and the refusal names the store and the leaf.
- A crash at each rename, flush and removal leaves the migrated store opening and the copy either whole or finished removing on the next open.

**Files:**
- create: crates/lys-log-store/src/migrate_removal_tests.rs
- modify: crates/lys-log-store/src/migrate.rs
- modify: crates/lys-log-store/src/file/open.rs
- modify: crates/lys-log-store/src/start.rs
- modify: crates/lys-log-store/src/error.rs

**Checklist:**
- C42 — A migration's kept copy is removed while running once a later start has checked it, and a mismatch is refused and kept (LYSLOGSTORE-009 R4).

**Stories:**
- S12 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want every Lys store to stay within a bound I can measure from outside while it runs, so that the disk never fills with bytes nobody can reach and no start or command reads all of history.

### R5: A bound test drives a stated mix and measures the store from outside, red at main

Behavioural. tests/store_bound.rs starts a fresh install layout in a temporary directory and drives a mix of acts at the ratios read from the live install's names on 7 October: agent sessions through the runner and proxy, each with prompts and one compaction; directory, grant, request and review acts; and a restart after every hundredth session. It runs to a stated volume of 2,000 sessions, then stops the processes. It reads every file's allocated size under the data path from the file system, never through a store. The bound is B = F + Σ(acts of each kind × that kind's evidence bytes per act from R1). F is the fixed bytes of the empty layout, measured once at the start. Evidence bytes are counted from the records each act writes and is kept for. The test fails when the measured total exceeds B, naming the five largest names over their row's share. It also fails when an idle hour (a clock step with no acts) or a restart with no acts adds a byte to any store. Fixtures are built once, by batch. The test is shown red at main before any R2 to R4 change, and the handback quotes that red. If it is green at main, the handback names which of R2 to R4 it failed to catch, and the test is made to catch it first.

**Acceptance:**
- Red at main, with the names over their share quoted in the handback.
- Green at the card's head, with B, the measured total and each name's bytes printed.
- Run time on Dean is posted; it runs under cargo nextest with no #[ignore].

**Files:**
- create: crates/lys/tests/store_bound.rs

**Checklist:**
- C43 — A bound test drives a stated mix to 2,000 sessions and measures the data path from outside, red at main (LYSLOGSTORE-009 R5).

**Stories:**
- S12 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want every Lys store to stay within a bound I can measure from outside while it runs, so that the disk never fills with bytes nobody can reach and no start or command reads all of history.

### R6: Every growing in-process collection has a stated bound

Behavioural. For each store in R1, every in-process map, set or vector that grows with records is named in its row with what bounds it: live records only, or every record by a stated contract (as seen in R3). One that holds ended records with no contract is changed to drop them when they end, in the same act that ends them. The bound test of R5 reads each named collection's length after the mix through a test-only count each store already exposes or gains under cfg(test), and fails when an ended record is still held.

**Acceptance:**
- R1's rows name every growing collection, each with its bound and file and line.
- After the R5 mix, each collection's length equals its live records plus any contract-kept ids, printed per store.

**Files:**
- modify: crates/lys/tests/store_bound.rs
- modify: docs/design/lys-log-store/STORES.md

**Checklist:**
- C44 — Every growing in-process collection holds only live records or contract-kept ids (LYSLOGSTORE-009 R6).

**Stories:**
- S12 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want every Lys store to stay within a bound I can measure from outside while it runs, so that the disk never fills with bytes nobody can reach and no start or command reads all of history.

### R7: Installed and measured: the store's levels are read from outside over a week

Behavioural. examples/store_levels.rs prints, for a data path given on its command line, each top-level name's allocated size in KiB and the total, read from file metadata only. It opens no file and does not descend into any name ending .previous. After this piece is installed, the levels are read and posted at the install, an hour after, a day after, and then daily for seven days, each with its time. Each posting sets the growth beside the acts the start records counted over the same period and against R5's bytes per act. The card is done when installed and measured, not when merged.

**Acceptance:**
- store_levels on a test layout prints the same sizes du -sk gives for each name.
- store_levels walks no .previous directory, shown by a test layout holding one.
- Eight postings over seven days, each beside its acts, are linked from the card.

**Files:**
- create: crates/lys/examples/store_levels.rs

**Checklist:**
- C45 — Installed, with the data path's levels posted over seven days beside the acts counted (LYSLOGSTORE-009 R7).

**Stories:**
- S12 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want every Lys store to stay within a bound I can measure from outside while it runs, so that the disk never fills with bytes nobody can reach and no start or command reads all of history.

## Boundaries

- SHALL NOT delete, rewrite or shorten any log leaf, or any record whose contract keeps it.
- SHALL NOT remove a kept v1 copy before a later start has checked the migrated store against it.
- SHALL NOT read the whole history on any open, apart from R4's one check of a kept copy.
- SHALL NOT add a size limit, quota or cap: the bound is measured, never a refusal.
- SHALL NOT add a haematite dependency.
- SHALL NOT open, read or write the live install's data or data.previous; levels are sizes only.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.
- SHALL NOT add a silent fallback: a check that fails is a named refusal.

## Verification

- The design gate, parsed: every cluster clean, no FAIL, every file valid.
- The full Lys gate on Dean, read from its parsed output, never its exit code: fmt, clippy pedantic in both configurations, ast-grep, nextest and cargo test --doc, with 0 failed at the card's head.
- store_bound's red at main and green at the head, both quoted in the handback.
- R7's eight postings over seven days after the install.
