---
type: brief
id: LYSLOGSTORE-007
cluster: lys-log-store
title: Nothing opens a log by reading every leaf: the anchor and the lys log commands open from the tiles
---

# LYSLOGSTORE-007: Nothing opens a log by reading every leaf: the anchor and the lys log commands open from the tiles

> **Cluster:** lys-log-store
> **Depends on:** LYSLOGSTORE-006
> **Design anchor:**
> - ADR-123 — A log's proofs come from C2SP hash tiles stored beside its leaves — Every append writes its leaf hash, and every tile it completes, into C2SP tlog-tiles beside the leaves before it moves the pin. Proofs read only the tiles that hold their hashes. A log made before tiles is given them once. The tile layout is local state, and it is also the layout a static serving of the log would use.
> **Checklist:**
> - C25 — A frontier log opens from its tiles and reads no leaf under the pin (LYSLOGSTORE-007 R1).
> - C26 — The anchor opens, creates, reads and proves through the frontier log and its tiles (LYSLOGSTORE-007 R2).
> - C27 — The lys log and ca log commands and the revocation appends run on the frontier log (LYSLOGSTORE-007 R3).
> - C28 — Only lys log audit reads every leaf, and the whole-tree Log is removed (LYSLOGSTORE-007 R4).
> - C29 — A gate test fails when any open reads a leaf under the pin (LYSLOGSTORE-007 R5).
> **Stories:**
> - S10 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want opening a log to cost the same however long it is, and one named command that reads every leaf when I ask for an audit, so that I never pay for a full read I did not ask for.

## Purpose

Tom's rule: a start never reads the whole history. The anchor and the lys log commands still open through the whole-tree Log, which reads every leaf, holds them all in memory and rebuilds the tree on every open (crates/lys-log-store/src/log.rs, the loops at lines 107 and 148). Its callers: crates/lys-anchor/src/anchor/open.rs lines 181 and 219, anchor/read_only.rs line 192, anchor/genesis/constructors.rs lines 77 and 184, and crates/lys/src/commands/log/store.rs line 56, which every lys log command goes through. FrontierLog::open from nothing reads every leaf too (crates/lys-log-store/src/frontier_log.rs line 166). With LYSLOGSTORE-006 every log keeps its hashes in tiles, so a log's frontier can be read from a few tiles instead.

## Task

Open a frontier log from its tiles, reading no leaf under the pin. Move the anchor, the lys log and ca log commands and the revocation append functions onto it. Keep the read of every leaf only in a new lys log audit command, and remove the whole-tree Log.

## Requirements

### R1: A frontier log opens from its tiles

Behavioural. FrontierLog opens a store that has tiles by reading, at the pinned size, the hash of each whole subtree the frontier holds (one per set bit of the size) from the tiles, and checking that they give the pinned root. It reads no leaf under the pin. Leaves past the pin, an interrupted append, are read and reconciled as today, and their hashes written into the tiles. A store whose tiles do not give the pinned root is refused tile_mismatch by name, as LYSLOGSTORE-006 R2 refuses it. The open from every leaf stays only for R4's lys log audit and for LYSLOGSTORE-006 R3's one adoption.

**Acceptance:**
- For every size from 1 to 600 and for 2^20 + 7 leaves, the frontier opened from tiles equals the frontier built from every leaf.
- A counting store shows the open of a clean log of 1,048,576 leaves reads no leaf and no more than 8 tiles.
- One leaf past the pin is read, reconciled and its hash written into the tiles; tiles that do not give the pinned root are refused tile_mismatch.

**Files:**
- create: crates/lys-log-store/src/tile_open.rs
- create: crates/lys-log-store/src/tile_open_tests.rs
- modify: crates/lys-log-store/src/frontier_log.rs
- modify: crates/lys-log-store/src/lib.rs

**Checklist:**
- C25 — A frontier log opens from its tiles and reads no leaf under the pin (LYSLOGSTORE-007 R1).

**Stories:**
- S10 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want opening a log to cost the same however long it is, and one named command that reads every leaf when I ask for an audit, so that I never pay for a full read I did not ask for.

### R2: The anchor runs on the frontier log

Behavioural. The anchor opens, creates, opens read-only and writes its genesis through FrontierLog opened from its tiles. Its tree size, root and checkpoint come from the frontier; its inclusion artifacts and the witness fixture's consistency proofs come from LYSLOGSTORE-006's tile proofs; a leaf is read from the store only when an artifact names it. Everything the anchor answers is byte-for-byte what it answers today.

**Acceptance:**
- Every existing lys-anchor test passes unchanged in what it asserts.
- A counting store shows opening an anchor of 100,000 leaves, and opening it read-only, reads no leaf.
- An inclusion artifact for an early leaf reads that one leaf and no other.

**Files:**
- modify: crates/lys-anchor/src/anchor/open.rs
- modify: crates/lys-anchor/src/anchor/read_only.rs
- modify: crates/lys-anchor/src/anchor/genesis/constructors.rs
- modify: crates/lys-anchor/src/anchor/artifact.rs
- modify: crates/lys-anchor/src/anchor/checkpoint.rs
- modify: crates/lys-anchor/src/anchor/submit.rs
- modify: crates/lys-anchor/src/anchor/append.rs
- modify: crates/lys-anchor/src/witness/fixture.rs

**Checklist:**
- C26 — The anchor opens, creates, reads and proves through the frontier log and its tiles (LYSLOGSTORE-007 R2).

**Stories:**
- S10 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want opening a log to cost the same however long it is, and one named command that reads every leaf when I ask for an audit, so that I never pay for a full read I did not ask for.

### R3: The lys log and ca log commands and the revocation appends run on it

Behavioural. crates/lys/src/commands/log/store.rs opens through FrontierLog from the tiles, and status, checkpoint, append and prove use its size, root, tile proofs and single leaf reads; ca_log.rs does the same. The revocation append functions (crates/lys-identity/src/revocation/append.rs, from line 30) take a FrontierLog. What each command prints is unchanged.

**Acceptance:**
- Every existing test of these commands and of revocation append passes unchanged in what it asserts.
- A counting store shows lys log status, checkpoint, append and prove over a log of 100,000 leaves read no leaf, apart from the one leaf prove names.

**Files:**
- modify: crates/lys/src/commands/log/store.rs
- modify: crates/lys/src/commands/log/status.rs
- modify: crates/lys/src/commands/log/checkpoint.rs
- modify: crates/lys/src/commands/log/append.rs
- modify: crates/lys/src/commands/log/prove.rs
- modify: crates/lys/src/commands/ca_log.rs
- modify: crates/lys-identity/src/revocation/append.rs

**Checklist:**
- C27 — The lys log and ca log commands and the revocation appends run on the frontier log (LYSLOGSTORE-007 R3).

**Stories:**
- S10 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want opening a log to cost the same however long it is, and one named command that reads every leaf when I ask for an audit, so that I never pay for a full read I did not ask for.

### R4: Reading every leaf is the audit, by name, and the whole-tree Log is gone

Behavioural. A new command, lys log audit <dir>, is the one that reads every leaf of a log: it streams them one at a time, checks that they rebuild the pinned root and that every tile equals the hashes the leaves give, and names the first leaf or tile that does not. It sits beside lys log verify, which checks proof artifacts and takes no log directory, and does not change it. Opening a log no longer audits it, and the documentation of open, of the anchor and of the command says so, naming lys log audit as the check. The whole-tree Log, its open, open_at_pin and prefix_tree, and their tests are removed from lys-log-store; no code keeps every leaf of a log in memory.

**Acceptance:**
- lys log audit over a log with one altered leaf names that leaf; with one altered tile it names that tile; over a sound log it exits 0.
- No type in the workspace holds every leaf of a log, checked by the removal of Log and a search in the gate for its name.

**Files:**
- create: crates/lys/src/commands/log/audit.rs
- create: crates/lys/src/commands/log/audit_tests.rs
- modify: crates/lys/src/commands/log/mod.rs
- modify: crates/lys-log-store/src/lib.rs
- delete: crates/lys-log-store/src/log.rs
- delete: crates/lys-log-store/src/log_tests.rs

**Checklist:**
- C28 — Only lys log audit reads every leaf, and the whole-tree Log is removed (LYSLOGSTORE-007 R4).

**Stories:**
- S10 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want opening a log to cost the same however long it is, and one named command that reads every leaf when I ask for an audit, so that I never pay for a full read I did not ask for.

### R5: A gate test fails when an open reads a leaf under the pin

Behavioural. A test runs every open this card touches (the anchor, read-only and full; the lys log store; a service store with a snapshot) over a store that counts leaf reads, and fails when any reads a leaf under the pin other than one an answer names. It runs in the full Lys gate.

**Acceptance:**
- Run against main before this card, it fails naming the anchor's open and the leaves it read.
- At the card's head it passes inside the full Lys gate.

**Files:**
- create: crates/lys-log-store/tests/no_open_reads_every_leaf.rs

**Checklist:**
- C29 — A gate test fails when any open reads a leaf under the pin (LYSLOGSTORE-007 R5).

**Stories:**
- S10 (Log inspector, Opens a log store to read it without changing it) — As a log inspector, I want opening a log to cost the same however long it is, and one named command that reads every leaf when I ask for an audit, so that I never pay for a full read I did not ask for.

## Boundaries

- SHALL NOT read a leaf under the pin on any open, apart from lys log audit and LYSLOGSTORE-006 R3's one adoption.
- SHALL NOT change what the anchor, its artifacts or any lys log command answers.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.
- SHALL NOT add a silent fallback: a tile that does not give the pinned root is a named refusal, never a quiet read of every leaf.

## Verification

- The full Lys gate, the ast-grep scan and the file-length check exit 0 at the card's head, measured by the card round.
- Over a log of a few hundred thousand leaves: lys log status answers without reading the leaves, and lys log audit reads them all and exits 0.
