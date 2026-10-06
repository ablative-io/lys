---
type: brief
id: DIRECTORY-084
cluster: directory
title: Every runner operation ID carries its issue time, an expired one is refused by its age, and the used-ID set is pruned
---

# DIRECTORY-084: Every runner operation ID carries its issue time, an expired one is refused by its age, and the used-ID set is pruned

> **Cluster:** directory
> **Depends on:** DIRECTORY-064
> **Blocked by:** DIRECTORY-064
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> - ADR-130 — Managed harness channels carry automatic context and reminder acts — 064 adds one runner-owned typed control/event channel per managed Claude or Codex session. It reuses 051 operation and usage owners, serialises all inputs at proved boundaries, requires actual harness evidence and retains uncertain outcomes without automatic resend. Legacy PTY sessions remain manual and cannot claim this capability. 065 extends the same Codex event reader for refusal coverage.
> **Checklist:**
> - C509 — Runner operation IDs have one dated form, `op-`, 12 hex digits of issue time, `-` and 32 hex digits, minted and read only in lys-runner operation_id.rs; every other form reads as undated (DIRECTORY-084 R1).
> - C510 — The runner refuses an undated, expired or future operation ID by name before any other judgment, live and after a reopen (DIRECTORY-084 R2).
> - C511 — The expiry pass prunes `seen` oldest first under a kept horizon, so the set holds at most two windows of IDs and a clock set back never readmits a pruned one (DIRECTORY-084 R2).
> - C512 — Goal reminders, budget crossings, the runner's managed input and peer restarts all mint dated IDs, and a resend after a restart carries the same ID (DIRECTORY-084 R3).
> - C513 — The named migration operations_undated_ids_retired runs once and removes every undated ID from `seen`, each refused by its form after (DIRECTORY-084 R4).
> - C514 — Tests show the set flat over ten windows, an expired ID refused after a reopen with nothing kept for it, and an old ID replayed after its window refused (DIRECTORY-084 R5).
> **Stories:**
> - S268 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want the runner's record of used operations to stay the same size however long it runs, while no operation is ever accepted twice, so that a runner that has run for a year starts and answers as fast as on its first day.

## Purpose

The runner keeps every operation ID it has ever seen, for good. operations.rs (main 19ebdcca) holds them in `seen` (line 174), inserts at 310 and checks at 355; prune (343 to 352) drops an expired outcome but never its ID, and repeated (354 to 362) refuses operation_repeated only by finding it there. No operation ID carries a time, so an expired ID can be told from a new one only by keeping it; dropping it would let a stale replay run a second time. Every checkpoint serializes the whole set under the Table. Measured on the BOX 20 draft (Brisket e8d905bf, run b113ff3a; observations, not bounds): at 1k used IDs a 1.6 MB checkpoint, 3.5 ms Table hold and 5.1 ms startup; at 1m, 46.6 MB, 146.6 ms and 208.5 ms. It is Tom's no-pruning failure (3 October 14:31), recorded FOUND in DIRECTORY-064 amendment 49 and briefed here by Waffles' ruling (d84942a7). The identity server's operation indexes (apps_index.rs, cord_store.rs, teams_state.rs and the like) index records that are themselves kept as history (CN21) and are not this defect.

## Task

Give every runner operation ID its issue time, in one form minted in one place; refuse an ID older than the retention window or dated beyond it by its age; prune `seen` in the existing expiry pass under a kept horizon; retire the old random-only IDs once by a named migration; prove the set flat. Out of scope: lys-identity's OperationId and the service's own operation indexes, the retention window's length, and what an admitted operation does.

## Requirements

### R1: One dated operation ID form, minted and read in one place

Ubiquitous. THE SYSTEM SHALL define the runner's operation ID once, in lys-runner operation_id.rs: the text `op-`, the issue time in milliseconds since the Unix epoch as exactly 12 lowercase hex digits, `-`, and exactly 32 lowercase hex digits (16 bytes, random or a digest, as the producer needs). It SHALL offer one function that mints an ID from an issue time and 16 bytes, and one that reads an ID's issue time, answering absent for any text not exactly in that form. Any other text is undated. The form is distinct from every ID a runner holds today: lys-identity's `op-` and 32 hex digits (operation.rs line 13), a budget crossing's bare 32 hex digits (budgets_crossing.rs lines 86 to 98) and the runner's `human-` and 32 hex digits (harness_control/events.rs line 308 on main) all read as undated, so no old ID can be mistaken for a dated one. lys-identity's OperationId, which is signed into events as 16 bytes, is not changed.

**Acceptance:**
- Minting from an issue time and 16 bytes and reading the issue time back answers the same time, for 0, a present time and the largest 12-hex-digit time.
- Each of today's forms (lys-identity `op-` and 32 hex, bare 32 hex, `human-` and 32 hex), upper-case hex, a short or long part and a missing `-` read as undated.
- No file but operation_id.rs formats or parses the dated form.

**Files:**
- create: crates/lys-runner/src/operation_id.rs
- modify: crates/lys-runner/src/lib.rs

**Checklist:**
- C509 — Runner operation IDs have one dated form, `op-`, 12 hex digits of issue time, `-` and 32 hex digits, minted and read only in lys-runner operation_id.rs; every other form reads as undated (DIRECTORY-084 R1).

**Stories:**
- S268 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want the runner's record of used operations to stay the same size however long it runs, while no operation is ever accepted twice, so that a runner that has run for a year starts and answers as fast as on its first day.

### R2: Admission refuses by age, and the expiry pass prunes the used-ID set

Event-driven. WHEN the runner admits an operation (Sessions::operate and operate_admitted through accept, and Sessions::restart_proved), THE SYSTEM SHALL, before any held, seen or digest judgment, refuse an undated ID as operation_id_undated, an ID whose issue time plus RETAIN_MS (operations.rs line 46, 24 hours) is at or before the floor as operation_id_expired, and an ID whose issue time is later than the floor plus RETAIN_MS as operation_id_future, each by name with the reason and the fix (mint a new ID). The floor is the later of the runner's clock and the horizon, the latest time the expiry pass has run at; the horizon is kept with the operations it prunes, in the same batch and the same single sync as amendments 40 and 41 rule, so a clock moved back after a restart can never readmit a pruned ID. After those checks the existing judgments run unchanged: a held outcome is answered, a same-ID retry is compared with its complete-request identity (amendments 44 to 47), and an ID in `seen` is refused as operation_repeated. `seen` SHALL be ordered by issue time, and the existing expiry pass (prune, operations.rs lines 343 to 352) SHALL remove from it every ID whose issue time plus RETAIN_MS is at or before the horizon, from the oldest, stopping at the first that is not, with no scan of the rest. A removed ID is refused by its age from then on, so no ID is ever admitted twice. The future bound is the retention window itself, not a new figure: an ID stays in `seen` until its own issue time plus RETAIN_MS, so `seen` holds only IDs issued within one window either side of the floor. The checkpoint keeps `seen` and the horizon, and reads back the same; nothing else in the checkpoint changes. outcome() reads a held outcome as today, whatever its ID's age.

**Acceptance:**
- An undated, an expired and a future ID are each refused by name before any held or seen lookup, live and after a reopen.
- An ID removed by the expiry pass is refused as operation_id_expired, never admitted, including after the runner's clock is set back below the horizon and the runner reopened.
- The expiry pass removes only IDs past their window, oldest first, and touches none after the first still in its window.
- The horizon is written in the same batch as the expiry it records, with no added sync.

**Files:**
- modify: crates/lys-runner/src/operations.rs
- modify: crates/lys-runner/src/operations/delivery.rs
- modify: crates/lys-runner/src/operations/store.rs
- modify: crates/lys-runner/src/session/restart.rs

**Checklist:**
- C510 — The runner refuses an undated, expired or future operation ID by name before any other judgment, live and after a reopen (DIRECTORY-084 R2).
- C511 — The expiry pass prunes `seen` oldest first under a kept horizon, so the set holds at most two windows of IDs and a clock set back never readmits a pruned one (DIRECTORY-084 R2).

**Stories:**
- S268 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want the runner's record of used operations to stay the same size however long it runs, while no operation is ever accepted twice, so that a runner that has run for a year starts and answers as fast as on its first day.

### R3: Every producer of a runner operation ID mints the dated form

Ubiquitous. THE SYSTEM SHALL mint, through operation_id.rs, every operation ID that reaches the runner's admission. Named by grep at main 19ebdcca and the BOX 20 draft e8d905bf (Act::Operate, Act::Withdraw, PeerAct::Restart and the runner's own Sessions::operate calls): (1) goals_store.rs op_id (line 423, used at 384) mints from the firing time already kept with the send (Sent.at, line 388) and the existing digest, so a resend after a restart finds the same ID. (2) The budget crossing: Crossing::id (budgets_crossing.rs lines 86 to 98) stays the crossing's own stable key for holds (line 239) and dispatch (budgets_enforce/dispatch.rs line 94), and the operation sent to the runner (budgets_act.rs lines 220 and 288, and the control path BOX 20 adds) is minted from the crossing's kept at_ms and that key, so judging again finds the same crossing and the same ID. (3) The runner's own managed input (harness_control/events.rs line 308) mints from the runner's clock and 16 random bytes. (4) A peer restart's ID is chosen by its caller (peer.rs line 362, session/restart.rs lines 19 to 23): the peer protocol states the dated form, restart_proved reads it through R2, and its letters, digits, '-' or '_' check is replaced by it. Checked and unchanged: surface api.ts lines 126 to 130 operationId() and lys identity stop (stop.rs line 265) make lys-identity OperationIds for the service's own signed records; neither reaches the runner's admission at either commit. Withdraw names an ID already admitted and mints none. The build greps again at its own head and names every producer it finds in the handback; one not listed here is a scope request, not a silent change.

**Acceptance:**
- A goal reminder and a budget crossing resent after a restart of the service carry the same dated ID, and the runner answers the held outcome.
- The runner's managed input and a peer restart are admitted with dated IDs and refused with undated ones.
- A grep at the head finds no Operation, Act::Operate or PeerAct::Restart outside tests whose ID is not minted by operation_id.rs.

**Files:**
- modify: crates/lys-identity-server/src/goals_store.rs
- modify: crates/lys-identity-server/src/budgets_act.rs
- modify: crates/lys-runner/src/harness_control/events.rs
- modify: crates/lys-runner/src/peer.rs

**Checklist:**
- C512 — Goal reminders, budget crossings, the runner's managed input and peer restarts all mint dated IDs, and a resend after a restart carries the same ID (DIRECTORY-084 R3).

**Stories:**
- S268 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want the runner's record of used operations to stay the same size however long it runs, while no operation is ever accepted twice, so that a runner that has run for a year starts and answers as fast as on its first day.

### R4: Old random-only IDs are retired once by a named migration

Event-driven. WHEN the runner first opens an operations record that holds undated IDs in `seen`, THE SYSTEM SHALL run one named migration, operations_undated_ids_retired, that removes every undated ID from `seen`, records in the same batch that the migration ran, and changes nothing else. Until it runs, an undated ID is refused because it sits in `seen`; once it has run, every undated ID is refused by R2 by its form alone, so dropping them admits none. A record that says the migration ran never runs it again, and a record with an undated ID in `seen` after that mark is refused at open by name. Held outcomes under undated IDs stay held and expire on their own outcome time as today.

**Acceptance:**
- A record made by the installed format with undated IDs in `seen` opens once into a `seen` with none, and each of those IDs is refused as operation_id_undated.
- Opening again does not run the migration a second time.
- A record marked migrated that still holds an undated ID in `seen` is refused at open by name.

**Files:**
- modify: crates/lys-runner/src/operations/store.rs
- modify: crates/lys-runner/tests/operations_index/cases.rs

**Checklist:**
- C513 — The named migration operations_undated_ids_retired runs once and removes every undated ID from `seen`, each refused by its form after (DIRECTORY-084 R4).

**Stories:**
- S268 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want the runner's record of used operations to stay the same size however long it runs, while no operation is ever accepted twice, so that a runner that has run for a year starts and answers as fast as on its first day.

### R5: The set stays flat, and no expired ID is ever admitted

Ubiquitous. THE SYSTEM SHALL prove the change with tests that drive the runner's own clock argument (prune and fold take now), with no sleep: (1) the set stays flat: a steady rate of new dated IDs over ten retention windows leaves `seen` no larger at the end than after the second window, and never above the IDs issued in two windows; (2) an expired ID is refused after a reopen with nothing kept for it in `seen`; (3) an old ID replayed after its window is still refused, by its age, not found in `seen`. an_expired_operation_id_is_refused_after_a_restart (tests/operations_index/cases.rs lines 94 to 109) is rewritten to (2), since it passes today only through seen.contains. Tests whose fixed operation IDs reach the runner's admission mint dated IDs through operation_id.rs; a listed file that needs no change is left unchanged and said so.

**Acceptance:**
- The three tests above pass, and fail at the brief's head before R2.
- No test in the listed files gives the runner an undated ID except to prove its refusal.

**Files:**
- modify: crates/lys-runner/tests/as_caller.rs
- modify: crates/lys-runner/tests/conformance.rs
- modify: crates/lys-runner/tests/context_control.rs
- modify: crates/lys-runner/tests/control_recovery.rs
- modify: crates/lys-runner/tests/injection/cases.rs
- modify: crates/lys-runner/tests/input_no_screen/shared_lock.rs
- modify: crates/lys-runner/tests/operations.rs
- modify: crates/lys-runner/tests/reminder_delivery.rs
- modify: crates/lys-runner/tests/peer_restart/cases.rs
- modify: crates/lys-identity-server/tests/control_receipts.rs
- modify: crates/lys-identity-server/tests/goals.rs
- modify: crates/lys-identity-server/tests/budget_crossings.rs

**Checklist:**
- C514 — Tests show the set flat over ten windows, an expired ID refused after a reopen with nothing kept for it, and an old ID replayed after its window refused (DIRECTORY-084 R5).

**Stories:**
- S268 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want the runner's record of used operations to stay the same size however long it runs, while no operation is ever accepted twice, so that a runner that has run for a year starts and answers as fast as on its first day.

## Boundaries

- SHALL NOT admit any ID that has been admitted before, at any age, across any reopen or clock change.
- SHALL NOT change lys-identity's OperationId, its signed bytes or the service's operation indexes.
- SHALL NOT add a figure beside RETAIN_MS: the future bound is the window itself.
- SHALL NOT scan `seen` or the history to prune, add a sync, or clone the Table's state.
- SHALL NOT keep undated IDs alongside dated ones after the migration, or accept the old forms for compatibility.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.

## Verification

- R5's three tests are posted red at the brief's head before R2 lands.
- cargo nextest run for lys-runner and lys-identity-server whole, cargo test --doc, fmt, clippy pedantic in both configurations, ast-grep and the file-length check exit 0 at the head, on Dean, judged by parsed output.
- The checkpoint measurement of DIRECTORY-064 run again at 1k and 1m IDs issued across ten windows shows `seen`, the checkpoint and the Table hold no larger at 1m than at two windows' worth.
