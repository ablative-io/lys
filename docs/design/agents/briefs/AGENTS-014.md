---
type: brief
id: AGENTS-014
cluster: agents
title: A runner and the owners it starts under a test end when the test ends, on any path
---

# AGENTS-014: A runner and the owners it starts under a test end when the test ends, on any path

> **Cluster:** agents
> **Depends on:** AGENTS-004
> **Design anchor:**
> - ADR-117 — Lys runs the agents it starts: its own background terminal, or a runner another tool provides — Lys runs the agents it starts. A runner is the part that holds an agent's terminal: Lys ships one of its own, a background process on each machine that runs each started agent in its own pseudo-terminal, and accepts any other runner that speaks the same small, published runner protocol. Through a runner, Lys starts, types into, presses keys in, reads, waits on, resizes, compacts and stops a session, rotates it across its accounts when it reports a usage limit, and wakes it with a message, each act under the caller's grants and recorded. Lys depends on no particular runner: its own is the default, another is chosen per machine.
> - ADR-125 — An agent session's holder key is held by the runner, and a session proves itself by peer credentials and ancestry — The runner makes one holder key per session in memory. A start becomes key, then issue, then spawn. The harness and lys mcp ask the runner for single presentations over its socket. The session is proved by peer credentials plus the process ancestry reaching the session's own root pid, checked on every connection, on macOS and Linux both, through a named maintained dependency and with no unsafe in Lys code.
> **Checklist:**
> - C772 — lys runner serve and lys runner seat-owner started with --ends-with-fd end, the way an intentional stop ends them, when that descriptor reaches end of file; the runner passes it to every owner it starts; without it nothing changes and the installed Lys never passes it.
> - C773 — Every lys-runner integration test that starts a runner holds the write end of its lifeline for the test's life, and the Mac's process list read before and after the crate's test run shows no runner or owner the run started outliving it.
> **Stories:**
> - S443 (An operator, a person running the estate) — As an operator running Lys's tests on a shared Mac, no runner or owner a test started is left running after the run, however the run ended, so the machine I share carries no stray session, harness or socket of a test long gone.

## Purpose

An owner outlives the runner that started it by design (AGENTS-004 R1: a seat is never down because its runner restarted), and a test's owners are ended by a Drop guard when the test ends. A guard runs on a return or a panic and never on a SIGKILL, and a test binary is killed whenever a nextest run is killed under it, as the night's gate runs were. On 19 October 2026 eleven lys runner seat-owner processes from the Lys gate runs were found on Tom's Mac, up to thirteen hours old, each holding a session, a harness and a socket of a test long gone; they were ended by pid. The rule that gates run detached and are never killed stands, and it is a rule, not a structure. This brief gives a runner and an owner started under a test a lifeline the kernel holds: a pipe whose write end dies with the test, whose end of file is the event that ends them.

## Task

Add the hidden --ends-with-fd argument to lys runner serve and seat-owner, read it on a thread, end on its end of file the way an intentional stop ends things, pass it from the runner to every owner it starts, prove it red first with a runner and an owner that exit when the test closes the write end, then start every runner the crate's tests spawn on such a pipe, and read the Mac's process list before and after the crate's test run to show nothing outlives it.

## Requirements

### R1: A runner or an owner started with --ends-with-fd ends when that descriptor reaches end of file

Behavioural. lys runner serve and lys runner seat-owner (crates/lys/src/cli/runner.rs, crates/lys/src/commands/runner.rs) accept one more hidden argument, --ends-with-fd <n>: a readable file descriptor the process inherited from whoever started it. Given it, the process reads that descriptor on a thread of its own from the moment it starts; end of file on it is the event that whoever started it is gone (the kernel closes the write end when that process exits, however it exits, SIGKILL included), and nothing else is read into it. On that event the runner stops everything it serves the way its intentional stop does (AGENTS-004 R8: the harnesses it holds ended, the owners it started told to end, the record written) and exits; an owner ends its one managed session the way an intentional stop does and exits. The runner passes its own descriptor on to every owner it starts: OwnerSpawn (crates/lys-runner/src/seat_owner/spawn.rs) gains ends_with: Option<RawFd>, carried from the runner's Options (crates/lys-runner/src/socket.rs), and start() marks the descriptor inheritable on the owner's Command and names it in the owner's arguments; the owner's setsid is unchanged (it still survives the runner's death on purpose when no descriptor was given, which is AGENTS-004 R1). Without the argument nothing changes: the service template, the identity install and lys runner's other commands never pass it. A read error on the descriptor that is not end of file is said by name on the process's error channel and treated as end of file: a process whose lifeline is broken does not go on.

**Acceptance:**
- Red first, in a new crates/lys-runner/tests/ends_with_fd.rs: a runner started with the read end of a pipe whose write end the test holds keeps serving while the test holds it, and exits when the test closes the write end; its exit is read by waiting on the child, never by a sleep; the owner it started for a seat exits too, read by the kernel's process-exit notice (kqueue EVFILT_PROC NOTE_EXIT on macOS, a pidfd on Linux; the crate's own exit_wait.rs is read first and extended rather than duplicated)
- A second red: a runner and owner started without the argument are unaffected when an unrelated pipe closes: the existing survival tests (seat_survival.rs) stay green unchanged
- grep -rn 'ends-with-fd' crates/lys/src/identity crates/lys-identity-server/src answers nothing: the installed Lys never passes it; the command stays hidden from lys --help
- gate -stages nextest -crates lys-runner -test 'package(lys-runner)' green, counts in the report

**Files:**
- create: crates/lys-runner/tests/ends_with_fd.rs
- modify: crates/lys/src/cli/runner.rs
- modify: crates/lys/src/commands/runner.rs
- modify: crates/lys-runner/src/socket.rs
- modify: crates/lys-runner/src/seat_owner/spawn.rs
- modify: crates/lys-runner/src/seat_owner/process.rs
- modify: crates/lys-runner/src/seat_owner/exit_wait.rs

**Checklist:**
- C772 — lys runner serve and lys runner seat-owner started with --ends-with-fd end, the way an intentional stop ends them, when that descriptor reaches end of file; the runner passes it to every owner it starts; without it nothing changes and the installed Lys never passes it.

**Stories:**
- S443 (An operator, a person running the estate) — As an operator running Lys's tests on a shared Mac, no runner or owner a test started is left running after the run, however the run ended, so the machine I share carries no stray session, harness or socket of a test long gone.

### R2: Every test that starts a runner or an owner starts it on a lifeline it holds

Behavioural. Every integration test under crates/lys-runner/tests that starts lys runner serve or an owner (the lane lists them by reading for the spawn: at least crates/lys-runner/tests/seat_survival.rs, crates/lys-runner/tests/seat_survival_faults.rs, crates/lys-runner/tests/seat_cost.rs, crates/lys-runner/tests/seat_conversation.rs, crates/lys-runner/tests/seat_owner_socket.rs, crates/lys-runner/tests/seat_restart.rs) opens a pipe, keeps the write end for the test's whole life in the fixture that holds the runner, and starts the runner with --ends-with-fd naming the read end, so a test that ends by panic, by a killed test binary or by a killed nextest run leaves no runner and no owner behind: the kernel closes the write end and R1 ends them. The Owners guard in seat_survival.rs stays (on a passing test it ends owners at once; the lifeline is for every other path). Where tests share a fixture for starting a runner, the pipe lives in that fixture once.

**Acceptance:**
- On the Mac, pgrep -fl 'runner seat-owner' and pgrep -fl 'runner serve' read before and after one detached gate -stages nextest -crates lys-runner -test 'package(lys-runner)' answer the same set: no runner and no owner the run started outlives it; both readings in the report
- A test in ends_with_fd.rs starts a runner through the shared fixture, kills its own helper process that holds the write end (a child the test spawned for the purpose, ended with SIGKILL), and reads the runner's exit as an event
- gate -stages nextest -crates lys-runner -test 'package(lys-runner)' green, counts in the report

**Files:**
- modify: crates/lys-runner/tests/seat_survival.rs
- modify: crates/lys-runner/tests/seat_survival_faults.rs
- modify: crates/lys-runner/tests/seat_cost.rs
- modify: crates/lys-runner/tests/seat_conversation.rs
- modify: crates/lys-runner/tests/seat_owner_socket.rs
- modify: crates/lys-runner/tests/seat_restart.rs

**Checklist:**
- C773 — Every lys-runner integration test that starts a runner holds the write end of its lifeline for the test's life, and the Mac's process list read before and after the crate's test run shows no runner or owner the run started outliving it.

**Stories:**
- S443 (An operator, a person running the estate) — As an operator running Lys's tests on a shared Mac, no runner or owner a test started is left running after the run, however the run ended, so the machine I share carries no stray session, harness or socket of a test long gone.

## Boundaries

- An owner started without the argument survives its runner exactly as AGENTS-004 R1 requires; nothing about production starts changes, and the installed Lys never passes the argument
- No timer, no fixed wait, no poll and no deadline anywhere: end of file is the event, the exit is read by wait or by the kernel's process-exit notice
- No process is ended by pattern or by name, in code or in test: a test ends only what it started, by pid or by closing the lifeline it holds
- Nothing installed, the installed Lys untouched
- Every file touched stays under 500 code lines (ADR-111, by scripts/file-length.sh)

## Verification

- gate -stages nextest -crates lys-runner -test 'package(lys-runner)', with pgrep -fl 'runner seat-owner' and pgrep -fl 'runner serve' read before and after
- gate -stages clippy
- python3 scripts/design/validate.py docs/design/agents and python3 scripts/design/check-coverage.py docs/design/agents read for FAIL lines
