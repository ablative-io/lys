---
type: design
cluster: lys-log-store
title: lys-log-store: a leaf store that never serves a leaf as whole unless it is proven whole
---

# lys-log-store: a leaf store that never serves a leaf as whole unless it is proven whole

> **Cluster:** lys-log-store

## Intention

A flight recorder is only evidence if reading it cannot change it and nothing it serves as whole is anything less. The leaf store is where that starts: it holds the leaves of a lys log on disk, and every other part of the log trusts what it says it holds.

When this cluster's first brief is done, a reader can open a leaf store and be certain that the open wrote nothing, repaired nothing and deleted nothing, and a store that a crash left mid-append tells that reader so by name instead of being quietly repaired under them. A leaf is proven whole in exactly one place, a Log opened against the pinned root, and the module doc says so where the next implementor will read it.

## Problem

FileLeafStore has one way to open, and it is the writer's. A caller that only wants to read a log gets a handle that will write a leaf or move the pin if asked, and a store left one leaf ahead of its pin by a crash is repaired by the first Log::open over it, whoever that caller is. So a read path, such as a status command or Anchor::open_read_only, performs a write. Nothing in the store names that state to a reader, and nothing tells a reader which leftover temporary files the open skipped.

The store's module doc says how a leaf is written but not what open does and never does: that open never deletes a leftover temporary file, and that FileLeafStore::leaf returns bytes it has not checked, so a leaf is proven whole only through Log::open against the pinned root. A reader who calls leaf() directly can be served a changed leaf without knowing it.

The refusal of a damaged pinned prefix exists on main as StoreError::PinMismatch from Log::open, but only a one-byte change is held under a test; a torn leaf inside the prefix, and a torn leaf just past the pin, are not.

## Solution

The store gains a second constructor for readers, FileLeafStore::open_read_only, beside the writer's FileLeafStore::open. It is an inherent constructor on FileLeafStore: the LeafStore trait, BACKENDS.md and Log are unchanged, and Log and Anchor gain no read-only mode (ADR-106). The read-only open performs the same checks as the writable open (log.json, state.json, the 20-digit leaf names and their contiguity) and differs in three ways: it does not flush leaves/, it refuses by name with StoreError::RepairPending a store whose extent is exactly one past its pinned tree size, and the handle it returns refuses put_leaf and pin with StoreError::ReadOnly before any other check. RepairPending is decided from the extent and the pin alone, without reading leaf bytes, so it fires before any Log could reach the repair; whether the pending leaf is a clean interrupted append or a damaged prefix is decided by a writable open, which repairs or refuses with PinMismatch exactly as it does today. A read-only handle over a store that is two or more leaves past its pin, or behind it, opens, and Log::open over it refuses with PinMismatch without writing.

The walk of leaves/ that counts the extent also collects the store's own temporary-leaf names, which it already skips, and returns them in lexical order from FileLeafStore::leftover_temporaries, so a caller can name what the open ignored. The walk never removes, renames or changes one.

The refusal of a changed leaf stays where it is: Log::open rebuilds the tree from every stored leaf and compares it with the one tree size and one root state.json keeps, and a leaf changed inside the pinned prefix is refused as StoreError::PinMismatch, which carries both trees and never a leaf index. The store keeps no per-leaf record, so which leaf changed is not known and is not claimed. New tests hold that refusal for a torn leaf and hold the one-leaf repair for a torn leaf just past the pin.

file.rs's module doc gains a section saying what open and open_read_only do and never do, including that neither ever deletes a leftover temporary file, and that leaf() serves bytes it has not checked: every reader that wants a proven leaf goes through Log::open.

The witness half of the same integrity row, a witness that reads only what is new, is LYSLOGSTORE-002's and is not designed here. The consumers' read paths (Anchor::open_read_only and the lys log commands) still open with the writable open after this cluster's first brief; moving them onto the read-only open is a further unit.

## Principles

- **P1** — A read-only open writes no byte: no leaf, no pin, no flush, no removal of a leftover.
- **P2** — A refusal is a named StoreError variant, never a repaired or partial answer.
- **P3** — A leaf is proven whole only by Log::open against the pinned root; the store serves bytes, it does not vouch for them.
- **P4** — The store claims only what it knows: it keeps one tree size and one root, so no refusal names a leaf index.
- **P5** — The LeafStore trait stays what BACKENDS.md says a backend must satisfy; reading without writing is FileLeafStore's own.

## Decisions

- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-106 — A FileLeafStore opened read-only refuses to write and to repair; Log and Anchor gain no read-only mode — FileLeafStore gains an inherent open_read_only whose handle refuses put_leaf and pin with StoreError::ReadOnly and which refuses a store exactly one leaf past its pin with StoreError::RepairPending, decided from the count and the pin without reading leaf bytes and without flushing. Log and Anchor gain no read-only mode: Log::open over a read-only handle either finds the tree matching the pin, or refuses with PinMismatch, and never reaches a pin, so the Log needs no mode for the store to hold. Both hold because the refusal of a repair lives at the one place that can know a repair would be needed before any Log sees the store. Rejected: opening a read-only log at its pinned head and reporting the repair as pending (a Log mode, and a served state the store has not reconciled); a read-only mode on Log or Anchor; making read-only opening a LeafStore trait method, which would add a criterion to BACKENDS.md; and a per-leaf hash record so a refusal could name the changed leaf.

## Goals

- No leaf is served as whole unless it is proven whole against the pinned root through Log::open, and file.rs's module doc says so.
- FileLeafStore::open_read_only refuses put_leaf and pin with StoreError::ReadOnly, and every file under the store directory holds the same bytes before and after.
- FileLeafStore::open_read_only on a store exactly one leaf past its pin returns StoreError::RepairPending, every file under the store directory holds the same bytes before and after, and a writable open of the same store repairs it as main does.
- A torn leaf inside the pinned prefix is refused by Log::open with StoreError::PinMismatch and state.json is unchanged, under a named test.
- FileLeafStore::leftover_temporaries names the temporary-leaf files an open skipped, and no open removes or changes one.

## Non-Goals

- A per-leaf hash record, or a refusal that names the index of a changed leaf. — The store keeps one tree size and one root (BACKENDS.md); naming the leaf needs a new layout record, which this cluster does not add.
- A read-only mode on Log or on Anchor. — ADR-106: the read-only open is FileLeafStore's alone.
- Moving Anchor::open_read_only and the lys log commands onto FileLeafStore::open_read_only, or printing leftover temporary files from the lys log and lys-anchor commands. — Those are the consumers' read paths, in lys-anchor, lys-anchor-cli and lys, and are a further unit.
- The witness reading only what is new. — LYSLOGSTORE-002 carries the witness half of the integrity row.
- Redoing flush-before-name, the no-replace link, the temporary-sequence parameter or the rename-injection isolation. — All four are on main already.
- Deleting or tidying leftover temporary files. — Open only skips and names them; clearing them is not the store's act.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `crates/lys-log-store/src/error.rs` | StoreError; gains ReadOnly and RepairPending |  |
| `crates/lys-log-store/src/file.rs` | FileLeafStore; gains open_read_only, the read-only refusals, leftover_temporaries and the 'What open does and never does' module-doc section |  |
| `crates/lys-log-store/src/file_tests.rs` | FileLeafStore tests; gains the read-only and leftover-temporary tests |  |
| `crates/lys-log-store/src/log.rs` | Log over any LeafStore; its open and one-leaf repair are unchanged |  |
| `crates/lys-log-store/src/log_tests.rs` | Log tests; gains the pinned-prefix refusal and repair tests |  |
| `crates/lys-log-store/src/store.rs` | The LeafStore trait contract; unchanged |  |
| `crates/lys-log-store/src/lib.rs` | Crate root and re-exports; unchanged |  |
| `docs/design/lys-log-store/BACKENDS.md` | Criteria a second LeafStore backend must meet; unchanged |  |
| `docs/design/lys-log-store/design.json` | This design | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/checklist.json` | The cluster checklist | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/stories.json` | The cluster stories | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-003.json` | The read-only leaf store brief | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/DESIGN.md` | Rendered from design.json | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/CHECKLIST.md` | Rendered from checklist.json | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/USER-STORIES.md` | Rendered from stories.json | LYSLOGSTORE-003 |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-003.md` | Rendered from LYSLOGSTORE-003.json | LYSLOGSTORE-003 |

## Inventory

- `crates/lys-log-store/src/file.rs` — FileLeafStore with create, open (flushes leaves/, then counts 20-digit names, skipping dot-prefixed ones), leaf, put_leaf (temporary write, flush, no-replace hard link) and pin. No read-only open. 430 code lines of the 500 allowed.
- `crates/lys-log-store/src/error.rs` — StoreError, #[non_exhaustive], 16 variants including PinMismatch; no ReadOnly or RepairPending.
- `crates/lys-log-store/src/log.rs` — Log::open rebuilds from every leaf and reconciles with the pin; repairs exactly one leaf past the pin when the prefix matches, otherwise PinMismatch.
- `crates/lys-log-store/src/store.rs` — The LeafStore trait and PinnedRoot; no fork, merge, delete, truncate or rewrite.
- `crates/lys-log-store/src/file_tests.rs` — 34 tests, including a_leftover_temporary_file_is_ignored_at_open and open_leaves_a_leftover_temporary_file_byte_identical; no read-only test.
- `crates/lys-log-store/src/log_tests.rs` — 13 tests, including a_tampered_leaf_byte_is_detected_at_open (one byte changed in the pinned prefix, PinMismatch), crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it and crash_recovery_does_not_mask_a_tampered_prefix.
- `crates/lys-anchor/src/anchor/read_only.rs` — Anchor::open_read_only opens its log with Log::open over the writable FileLeafStore::open, so it repairs and pins on a read path.
- `crates/lys/src/commands/log/store.rs` — The lys log commands open with Log::open(FileLeafStore::open(dir)), so status repairs on a read path.
- `docs/design/lys-log-store/BACKENDS.md` — Five criteria a second backend must meet; reading without writing is not among them.
- `docs/design/identity/CONFORMANCE.md` — The only numbered conformance table; it carries no flight-recorder row, and its row 4.3 is a roles row.
- `scripts/design/gate.sh` — Validates every cluster with a design.json, checks its coverage and compares its rendered markdown byte for byte.

## Constraints

- **CN1** — The leaf write on main is unchanged: the temporary write and flush, the no-replace hard link, the next_sequence parameter and the AfterLink seams.
- **CN2** — A writable open followed by Log::open repairs a store exactly one leaf past its pin as main does, and reports it through recovered_to.
- **CN3** — The leaves/ layout, the 20-digit leaf name, log.json, state.json's shape and the rule that a leaf file is the RFC 6962 preimage are unchanged.
- **CN4** — The LeafStore trait gains no method, and gains no fork, merge, delete, truncate or leaf rewrite; BACKENDS.md is unchanged.
- **CN5** — No open deletes, renames or modifies a leftover temporary file.
- **CN6** — crates/lys-log-store/src/file.rs holds no more than 500 lines that are neither blank nor comment lines, not counting tests.
- **CN7** — Library code has no unwrap, expect, panic, todo, unimplemented or unreachable, and no #[allow], #[ignore] or _-prefixed bypass is added.
