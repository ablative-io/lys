---
type: brief
id: DIRECTORY-044
cluster: directory
title: The identity install and its clients wait on signals: no sleeps, no poll loops, no timeouts
---

# DIRECTORY-044: The identity install and its clients wait on signals: no sleeps, no poll loops, no timeouts

> **Cluster:** directory
> **Design anchor:**
> - ADR-113 — The identity install and its clients wait on signals, never on a clock — A wait ends on the event it is waiting for or on the failure that makes the event impossible: a service is ready when it says so or when a single connection to it succeeds after a readiness event (its log gaining its listening line, observed through the platform's file-change notification), a stopped process is gone when the platform reports its exit (kqueue EVFILT_PROC on macOS, pidfd on Linux), a peer that refuses or closes is an error at once, and nothing carries a timeout. The screen re-measures on a ResizeObserver or animation-frame signal. Rejected: shorter sleeps, which still wait on a clock; keeping timeouts as a safety net, which the rule forbids and which hides a stuck peer's cause.
> **Checklist:**
> - C340 — Waiting for a service to answer waits on its readiness event, not a one-second sleep (DIRECTORY-044 R1).
> - C341 — Stopping a service waits on the platform's exit notification (DIRECTORY-044 R2).
> - C342 — Loopback, Rauthy and health exchanges carry no timeout (DIRECTORY-044 R3).
> - C343 — The Explain view re-measures on a layout signal (DIRECTORY-044 R4).
> **Stories:**
> - S146 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person installing the identity stack, I want the install to go on the moment each service is ready and to name at once the one that died, so that nothing is cut off by a clock and nothing waits longer than the service does.

## Purpose

CLAUDE.md forbids every timeout and every wait on a clock, and the identity install still sleeps between readiness checks, polls a stopped process every 200 ms, and bounds every loopback, Rauthy and health exchange in seconds. A slow or stuck service is then either cut off by a clock or found late, instead of named by its signal.

## Task

Replace each wait and bound named below with a wait on the event itself, each proved by a test that counts sleeps and polls rather than timing anything.

## Requirements

### R1: Waiting for a service to answer waits on its readiness event, not a one-second sleep

Behavioural. crates/lys/src/identity/install/services.rs:135-150 (the readiness loop that sleeps 1 s between checks and prints every fifth round) and :302-317 (wait_answering). Replace both with a wait on events: the service's log file is watched through the platform's file-change notification (kqueue EVFILT_VNODE on macOS, inotify on Linux, through a crate already in Cargo.lock or the standard library and libc if none is), and the process's exit is watched as in R2; on each log change the check runs once. It ends ready when the check passes, and refused by name, with the log's path, when the process exits. The periodic 'waiting for' line is replaced by one line when the wait begins naming what it waits for, and one when each service becomes ready.

**Acceptance:**
- A test starts a scratch service that writes its listening line after it is told to over a pipe, and shows the wait returns within the same event, with 0 sleeps (a counting double on the sleep function, or grep showing no thread::sleep in services.rs).
- A test whose service exits before becoming ready shows the named refusal carrying the log path.
- grep -n 'thread::sleep' crates/lys/src/identity/install/services.rs prints nothing.

**Files:**
- modify: crates/lys/src/identity/install/services.rs

**Checklist:**
- C340 — Waiting for a service to answer waits on its readiness event, not a one-second sleep (DIRECTORY-044 R1).

**Stories:**
- S146 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person installing the identity stack, I want the install to go on the moment each service is ready and to name at once the one that died, so that nothing is cut off by a clock and nothing waits longer than the service does.

### R2: Stopping a service waits on the platform's exit notification

Behavioural. crates/lys/src/identity/install/services.rs:220-231 polls alive(pid_file) every 200 ms after kill. Wait for the process's exit through kqueue EVFILT_PROC (NOTE_EXIT) on macOS and a pidfd on Linux; a pid that is already gone returns at once; a pid the platform refuses to watch is an error naming it.

**Acceptance:**
- A test stops a scratch process and shows stop returns after the exit event with 0 polls.
- A test with a pid file naming a process already gone returns Ok(false) or its existing answer at once, as the base does.

**Files:**
- create: crates/lys/src/identity/install/exit_wait.rs
- modify: crates/lys/src/identity/install/services.rs

**Checklist:**
- C341 — Stopping a service waits on the platform's exit notification (DIRECTORY-044 R2).

**Stories:**
- S146 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person installing the identity stack, I want the install to go on the moment each service is ready and to name at once the one that died, so that nothing is cut off by a clock and nothing waits longer than the service does.

### R3: Loopback, Rauthy and health exchanges carry no timeout

Behavioural. crates/lys/src/identity/loopback_http.rs:76-160 (connect, connect_any, exchange take a timeout and set connect, read and write timeouts), rauthy.rs:21 (TIMEOUT 10 s), health.rs:23 (TIMEOUT 3 s), services.rs:298 (2 s). Remove the timeout parameter and every connect_timeout, set_read_timeout and set_write_timeout: connect with TcpStream::connect, a refused or reset connection is an error at once naming the address and cause, and a peer that closes mid-answer is an error naming how many bytes arrived. Every caller is updated; no caller keeps a Duration.

**Acceptance:**
- grep -nE 'timeout|Duration' crates/lys/src/identity/loopback_http.rs crates/lys/src/identity/rauthy.rs crates/lys/src/identity/health.rs prints nothing.
- Tests for a refused port and a peer that closes mid-answer show the named errors.

**Files:**
- modify: crates/lys/src/identity/loopback_http.rs
- modify: crates/lys/src/identity/rauthy.rs
- modify: crates/lys/src/identity/health.rs
- modify: crates/lys/src/identity/install/services.rs

**Checklist:**
- C342 — Loopback, Rauthy and health exchanges carry no timeout (DIRECTORY-044 R3).

**Stories:**
- S146 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person installing the identity stack, I want the install to go on the moment each service is ready and to name at once the one that died, so that nothing is cut off by a clock and nothing waits longer than the service does.

### R4: The Explain view re-measures on a layout signal

Behavioural. surface/identity/src/shell/Explain.tsx:52-55 re-measures its marks on a 120 ms setTimeout. Re-measure from a ResizeObserver on the measured elements (and once in a requestAnimationFrame after the content changes), and disconnect it on unmount. The toast's display duration (ShellContext.tsx:140) and the two-key chord window (keys.ts:65) are interaction timings, not waits, and stay.

**Acceptance:**
- A vitest test drives a resize through a ResizeObserver double and shows setMarks called once per resize with no timer (vi.useFakeTimers shows no pending timer from Explain).
- npx tsc --noEmit, npx vitest run and npx vite build exit 0 in surface/identity.

**Files:**
- modify: surface/identity/src/shell/Explain.tsx

**Checklist:**
- C343 — The Explain view re-measures on a layout signal (DIRECTORY-044 R4).

**Stories:**
- S146 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person installing the identity stack, I want the install to go on the moment each service is ready and to name at once the one that died, so that nothing is cut off by a clock and nothing waits longer than the service does.

## Boundaries

- SHALL NOT add any timeout, deadline, sleep, poll interval or bound in seconds anywhere.
- SHALL NOT change what the install installs, the order it starts services in, or any refusal's name.
- SHALL NOT change certificate validity (certificates_issue.rs VALID_FOR) or the lys CLI's duration parsing (commands/duration.rs): those are durations in the domain, not waits.
- SHALL NOT add #[allow], #[ignore] or any other bypass; every file stays under 500 lines of code; ast-grep scan stays at zero hits.
- SHALL NOT measure anything with a clock in a test.

## Verification

- The full Lys gate and the surface checks (npx tsc --noEmit, npx vitest run, node --test scripts/package.test.mjs, npx vite build) exit 0 at the card's head, measured by the card round.
- A grep over crates/lys/src/identity and surface/identity/src/shell/Explain.tsx for thread::sleep, connect_timeout, set_read_timeout, set_write_timeout and setTimeout prints nothing.
- A live install of the identity stack on the build machine from the card's head completes, and its output names each service as ready once.
