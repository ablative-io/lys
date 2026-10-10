---
type: brief
id: DIRECTORY-091
cluster: directory
title: A tripped session that cannot receive the hang-up and prints nothing is ended by the runner, not held forever
---

# DIRECTORY-091: A tripped session that cannot receive the hang-up and prints nothing is ended by the runner, not held forever

> **Cluster:** directory
> **Depends on:** DIRECTORY-050
> **Design anchor:**
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-117 — Lys runs the agents it starts: its own background terminal, or a runner another tool provides — Lys runs the agents it starts. A runner is the part that holds an agent's terminal: Lys ships one of its own, a background process on each machine that runs each started agent in its own pseudo-terminal, and accepts any other runner that speaks the same small, published runner protocol. Through a runner, Lys starts, types into, presses keys in, reads, waits on, resizes, compacts and stops a session, rotates it across its accounts when it reports a usage limit, and wakes it with a message, each act under the caller's grants and recorded. Lys depends on no particular runner: its own is the default, another is chosen per machine.
> **Checklist:**
> - C727 — A tripped generation whose program cannot receive the hang-up and prints nothing is killed with its process group REPEAT_GRACE after the first hang-up, stamped rotation_signal_escalated (DIRECTORY-091 R1).
> - C728 — The escalation kills only the leader the hang-up was sent to, judged by pid and start time; an exited or reused leader is left alone (DIRECTORY-091 R2).
> **Stories:**
> - S415 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running a team of agents on rotating accounts, I want a session the runner hung up at its usage limit to end even when its program cannot receive the hang-up and prints nothing, so that a silent program never holds my agent, its next account or the runner forever.

## Purpose

DIRECTORY-050 R5 ends a session at its usage limit with a hang-up and moves it to the next account. Since cd41785a the runner repeats the hang-up when the generation prints after the first one, and kills the process group REPEAT_GRACE (2 s) after that repeat if the same leader still runs. Waffles' read of cd41785a (10 Oct 2026 13:16) names the gap left: the escalation hangs off the repeat and the repeat hangs off output, so a program that cannot receive the hang-up and prints nothing after it is never repeated and never killed, and the generation's watch waits on its exit with no clock. It is real on macOS: /bin/sh is GNU bash 3.2.57, and a shell that runs `exec` from inside its SIGHUP trap hands the new program SIGHUP blocked (probe: `PROBE blocked=['SIGHUP'] HUP=SIG_DFL`); a trap that execs without printing leaves a silent program the runner can only wait on. The only ends today are by hand: an operator stop with kill, or a second stop (session/stop.rs).

## Task

In lys-runner, arm the existing escalation from the first hang-up of a tripped generation as well as from the repeat: once the first hang-up is sent, the same one-shot thread waits REPEAT_GRACE and, if the same leader (pid and start time) still runs, kills its process group with SIGKILL and says so. Prove it with a runner test whose program blocks the hang-up and prints nothing after it, and keep every existing rotation test passing unchanged.

## Requirements

### R1: The escalation is armed from the first hang-up of a tripped generation

Behavioural. THE SYSTEM SHALL, when a generation's output trips its usage limit and the runner sends its first hang-up (pty::end returns Ok), arm the escalation once for that generation: a thread of its own waits REPEAT_GRACE (2 s, the value Waffles ruled at 12:09 for the repeat, reused as he named at 13:16), then calls pty::kill_if_still with the leader recorded at the hang-up, and when it answers true says `session <id>: rotation_signal_escalated: still running 2 s after the hang-up; its process group was killed` on stderr. A repeat hang-up after output does not arm a second escalation for the same generation. The reader still never waits on a clock; a leader that exited, or whose pid now names another process, is left alone.

**Acceptance:**
- A rotating session whose script is `trap 'trap - HUP; exec cat' HUP; echo "at $LYS_ACCOUNT_HANDLE: usage limit reached"; read -r line` on two accounts ends with ended.how == AccountsExhausted, ended.status == None and ended.signal.is_some(), and the test finishes in under 15 s.
- The same session's stderr carries `rotation_signal_escalated` exactly once per generation that held the hang-up (twice for two accounts) and no `rotation_signal_repeated` line, because the program printed nothing after the hang-up.
- A rotating session whose program exits on the first hang-up (`echo "at $LYS_ACCOUNT_HANDLE: usage limit reached"; read -r line`, no trap) ends with no `rotation_signal_escalated` line on stderr.
- a_program_that_drops_one_hang_up_is_told_again_when_it_prints passes unchanged, with exactly one `rotation_signal_escalated` per generation, not two.

**Files:**
- modify: crates/lys-runner/src/session/lifecycle/terminal.rs
- modify: crates/lys-runner/tests/runner.rs

**Checklist:**
- C727 — A tripped generation whose program cannot receive the hang-up and prints nothing is killed with its process group REPEAT_GRACE after the first hang-up, stamped rotation_signal_escalated (DIRECTORY-091 R1).

**Stories:**
- S415 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running a team of agents on rotating accounts, I want a session the runner hung up at its usage limit to end even when its program cannot receive the hang-up and prints nothing, so that a silent program never holds my agent, its next account or the runner forever.

### R2: The kill reaches only the leader that was hung up

Behavioural. THE SYSTEM SHALL kill only a process group whose leader is the process the hang-up was sent to, judged by pid and start time when the grace ends (pty::kill_if_still, cd41785a). A leader that exited during the grace is not signalled and nothing is said; a pid reused by another process within the grace is not signalled.

**Acceptance:**
- A rotating session on two accounts whose script is `echo "at $LYS_ACCOUNT_HANDLE: usage limit reached"; read -r line` (no trap, so the shell is ended by the hang-up within REPEAT_GRACE) ends with ended.how == AccountsExhausted and ended.signal.is_some(), and stderr carries no `rotation_signal_escalated` and no `rotation_escalation_failed` line.
- pty::kill_if_still called with a Leader whose start time differs from the live process at that pid answers Ok(false) and the process at that pid is still running afterwards.

**Files:**
- modify: crates/lys-runner/src/pty.rs
- modify: crates/lys-runner/tests/runner.rs

**Checklist:**
- C728 — The escalation kills only the leader the hang-up was sent to, judged by pid and start time; an exited or reused leader is left alone (DIRECTORY-091 R2).

**Stories:**
- S415 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running a team of agents on rotating accounts, I want a session the runner hung up at its usage limit to end even when its program cannot receive the hang-up and prints nothing, so that a silent program never holds my agent, its next account or the runner forever.

## Boundaries

- lys-runner only. No change to the rotation contract of DIRECTORY-050 R5 (which accounts, in what order, accounts_exhausted at the list's end), to the session record, to the protocol or to any route.
- REPEAT_GRACE stays one value, 2 s, for the first hang-up and the repeat (Waffles 12:09 and 13:16). A harness that needs longer than 2 s after a hang-up to write its own session file loses that file when it is killed, as the repeat already does (terminal.rs comment); a longer grace for the first hang-up is a number for Waffles to rule, not this brief's.
- The reader thread never sleeps; only the one-shot escalation thread waits.

## Verification

- cargo nextest run -p lys-runner --test runner, the new tests and every rotation test, on the Mac (Ghostty is a Developer Tool there since 10 Oct 13:17).
- cargo clippy -p lys-runner --all-targets -- -D warnings, and the full Lys battery at the landing head.
- Reproduce the silent case before the change on the Mac with the Python pty probe from cd41785a's commit message, the trap's echo removed, and record that the runner test hangs at the 300 s terminate-after before the change and passes after it.
