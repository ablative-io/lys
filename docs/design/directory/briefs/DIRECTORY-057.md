---
type: brief
id: DIRECTORY-057
cluster: directory
title: A service's exit lock is opened only in the service's own process, so no other process ever carries a copy of it
---

# DIRECTORY-057: A service's exit lock is opened only in the service's own process, so no other process ever carries a copy of it

> **Cluster:** directory
> **Depends on:** DIRECTORY-044
> **Design anchor:**
> - ADR-121 — A service's exit lock is opened in the service's own process by a holder command, not in the starter — The service is started through `lys identity hold`, which opens the lock, takes it and execs the service. Refused: code run between fork and exec (unsafe, async-signal-safe calls only); the spawn's file actions with O_EXLOCK (macOS and BSD only, a foreign call in an unsafe block, no equal on Linux).
> **Checklist:**
> - C400 — The three ways to keep the exit lock out of the starter are compared in ADR-121 and the holder command is chosen, with no unsafe code (DIRECTORY-057 R1).
> - C401 — A holder command opens the exit lock, takes it and becomes the service by exec, keeping its pid; a program that cannot run is refused by name (DIRECTORY-057 R2).
> - C402 — The starter never opens the exit lock; a service that ends at once beside other starts is seen ended every time (DIRECTORY-057 R3).
> **Stories:**
> - S161 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want a service that has ended to be seen as ended at once, whatever else Lys was starting at that moment, so that a stop, a restart and an upgrade never wait on or misread a process that is gone.

## Purpose

A service the install starts holds an exclusive lock on its exit lock, and its exit is seen as the release of that lock. Today the starter opens and takes the lock itself and hands it to the service as its standard input (exit_wait::hold, services::start_detached). While the starter holds that descriptor, a process started by any other thread of the starter carries a copy of it until its exec, and the lock stays held in that copy. After the watch side was fixed (a watch asks for a shared lock and releases it by name), this is what remains: it only delays an exit being seen just after a start, and closing it in the starter needs code run between fork and exec, which is unsafe code. This card closes it without unsafe code, and records why the other ways were refused.

## Task

Start each service through a small holder that is the service's own process: the holder opens the exit lock, takes it, and then becomes the service by exec, keeping its pid. The starter never opens the exit lock. Record the comparison of the three ways in ADR-121.

## Requirements

### R1: The decision is recorded with the three ways compared

Structural. ADR-121 records the three ways to keep the lock out of the starter and the reason for the one chosen. (a) Code run between fork and exec in the child (pre_exec): needs an unsafe block in Lys code and may call only async-signal-safe functions; refused. (b) The spawn's own file actions (posix_spawn_file_actions_addopen) with O_EXLOCK: opens and locks in the child with no code of ours run there, but O_EXLOCK exists on macOS and the BSDs only, the standard library exposes no file actions so it needs a foreign call in an unsafe block, and Linux has no equal; refused. (c) A holder command in the lys binary that opens the lock, takes it and execs the service: safe standard library calls only, the same on macOS and Linux, and the pid written to the pid file is the service's because exec keeps it; chosen.

**Acceptance:**
- ADR-121 is in decisions.json with the three ways, and the brief cites it.
- No unsafe block is added anywhere in the workspace, checked by the existing lint.

**Files:**
- modify: docs/design/decisions.json

**Checklist:**
- C400 — The three ways to keep the exit lock out of the starter are compared in ADR-121 and the holder command is chosen, with no unsafe code (DIRECTORY-057 R1).

**Stories:**
- S161 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want a service that has ended to be seen as ended at once, whatever else Lys was starting at that moment, so that a stop, a restart and an upgrade never wait on or misread a process that is gone.

### R2: The holder opens the lock, takes it and becomes the service

Behavioural. `lys identity hold --exit-lock <path> -- <program> [args]` opens the exit lock, waits for an earlier holder to let go, takes the exclusive lock, and execs the program with the lock as its standard input and its own standard output and error unchanged. It starts no thread and no other process before the exec. A program that cannot be run is refused service_not_startable naming the program and the cause, written to standard error, and the holder exits with a status other than 0. The command is not listed in the help an ordinary person reads.

**Acceptance:**
- A service started through the holder has the pid the starter wrote, holds the exit lock, and a watch answers not exited while it lives and exited after it ends.
- A second holder on the same lock waits for the first service's exit and then starts; two never run at once.
- A program that does not exist is refused service_not_startable naming it, and the exit lock is free afterwards.

**Files:**
- create: crates/lys/src/identity/install/held_start.rs
- create: crates/lys/src/identity/install/held_start_tests.rs
- modify: crates/lys/src/identity/cli.rs
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys/src/identity/error.rs

**Checklist:**
- C401 — A holder command opens the exit lock, takes it and becomes the service by exec, keeping its pid; a program that cannot run is refused by name (DIRECTORY-057 R2).

**Stories:**
- S161 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want a service that has ended to be seen as ended at once, whatever else Lys was starting at that moment, so that a stop, a restart and an upgrade never wait on or misread a process that is gone.

### R3: The starter never opens the exit lock

Behavioural. services::start_detached starts the holder in place of the program and no longer calls exit_wait::hold; exit_wait::hold is called by the holder only. Every start goes this way: install, the upgrade's units and any later unit. The proof is the one that showed the watch side: a service that ends at once is started 300 times beside eight threads that start /usr/bin/true without pause; its end is learnt from an event other than the lock (the far end of a pipe it held closes), and a watch opened at that moment answers exited every time. The test is seen red against today's start before the change.

**Acceptance:**
- The proof hides 0 of 300 exits after the change and more than 0 before it, both counts recorded in the card's report.
- A search of the workspace finds exit_wait::hold called from held_start.rs only.
- The existing install, exit watch and upgrade tests pass unchanged.

**Files:**
- create: crates/lys/src/identity/install/start_race_tests.rs
- modify: crates/lys/src/identity/install/services.rs
- modify: crates/lys/src/identity/install/exit_wait.rs

**Checklist:**
- C402 — The starter never opens the exit lock; a service that ends at once beside other starts is seen ended every time (DIRECTORY-057 R3).

**Stories:**
- S161 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want a service that has ended to be seen as ended at once, whatever else Lys was starting at that moment, so that a stop, a restart and an upgrade never wait on or misread a process that is gone.

## Boundaries

- SHALL NOT add an unsafe block, a foreign call or a new dependency.
- SHALL NOT change the number of test threads in any gate, or serialise tests, to hide a race.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass; a wait ends on an event.
- SHALL NOT add a silent fallback: every failure is a named refusal.
- SHALL NOT change what a watch does: it asks for a shared lock and releases it by name.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- The 300-start proof's two counts, before and after, are in the card's report.
