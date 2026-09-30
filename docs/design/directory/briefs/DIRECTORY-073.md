---
type: brief
id: DIRECTORY-073
cluster: directory
title: Operators restart reviewed profiles, proved peers restart their held launch, and prompts reach unwatched sessions
---

# DIRECTORY-073: Operators restart reviewed profiles, proved peers restart their held launch, and prompts reach unwatched sessions

> **Cluster:** directory
> **Design anchor:**
> - ADR-117 — Lys runs the agents it starts: its own background terminal, or a runner another tool provides — Lys runs the agents it starts. A runner is the part that holds an agent's terminal: Lys ships one of its own, a background process on each machine that runs each started agent in its own pseudo-terminal, and accepts any other runner that speaks the same small, published runner protocol. Through a runner, Lys starts, types into, presses keys in, reads, waits on, resizes, compacts and stops a session, rotates it across its accounts when it reports a usage limit, and wakes it with a message, each act under the caller's grants and recorded. Lys depends on no particular runner: its own is the default, another is chosen per machine.
> - ADR-125 — An agent session's holder key is held by the runner, and a session proves itself by peer credentials and ancestry — The runner makes one holder key per session in memory. A start becomes key, then issue, then spawn. The harness and lys mcp ask the runner for single presentations over its socket. The session is proved by peer credentials plus the process ancestry reaching the session's own root pid, checked on every connection, on macOS and Linux both, through a named maintained dependency and with no unsafe in Lys code.
> - ADR-113 — The identity install and its clients wait on signals, never on a clock — A wait ends on the event it is waiting for or on the failure that makes the event impossible: a service is ready when it says so or when a single connection to it succeeds after a readiness event (its log gaining its listening line, observed through the platform's file-change notification), a stopped process is gone when the platform reports its exit (kqueue EVFILT_PROC on macOS, pidfd on Linux), a peer that refuses or closes is an error at once, and nothing carries a timeout. The screen re-measures on a ResizeObserver or animation-frame signal. Rejected: shorter sleeps, which still wait on a clock; keeping timeouts as a safety net, which the rule forbids and which hides a stuck peer's cause.
> **Checklist:**
> - C466 — A restart ends the live session and starts it from the latest reviewed version. (DIRECTORY-073 R1).
> - C467 — A seat can restart itself and no other. (DIRECTORY-073 R2).
> - C468 — Compaction requests and reminders reach a session nobody is watching without blocking another. (DIRECTORY-073 R3).
> **Stories:**
> - S187 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person setting agent context and reminders, I want Lys to restart a seat onto its new settings and prompt a seat nobody is watching, so that Argus and Manifold are no longer needed for either.

## Purpose

Lys can start a seat and end it (runner end, runner_acts.rs), and a budget already asks the runner for a compaction or a notice at the turn boundary (budgets_act.rs), and a due goal reminder is typed into each live session (goals_api.rs line 15). There is no restart, so a seat given a new profile version keeps running the old one until a person stops and starts it by hand, and a seat cannot ask for its own restart to pick up new MCP configuration. Argus and Manifold do this today with send-keys; it moves to Lys.

## Task

Add an operator restart that ends a selected live session, waits on its end event and starts a new session from the latest reviewed profile through the existing start path. Add a runner peer restart that proves the caller by socket credentials and ancestry and restarts only its own held signed launch, with unchanged credentials and no call to the identity server. Prove compaction, notices and reminders reach unwatched sessions without blocking another session. No new prompt kind.

## Requirements

### R1: A restart ends the session and starts it from the latest reviewed version

Behavioural. WHEN a person or lead with the start right restarts an agent, THE SYSTEM SHALL end its live session through the runner, SHALL wait on the runner's end event, and SHALL start the agent again from its latest reviewed profile version through the existing start path, keeping a new launch record; with no reviewed version it SHALL refuse as profile_version_unreviewed and SHALL NOT end the session.

**Acceptance:**
- Pikelet restarted after a new version runs that version, and its launch record names it.
- A restart with no reviewed version is refused by name and the session keeps running.
- No step of the restart waits on a clock.

**Files:**
- create: crates/lys-identity-server/src/restart_api.rs
- create: crates/lys-identity-server/tests/restart_api.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/runner_acts.rs
- modify: crates/lys-identity-server/src/launch_api.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/openapi_refusals.rs
- modify: crates/lys-identity-server/src/openapi_types.rs

**Checklist:**
- C466 — A restart ends the live session and starts it from the latest reviewed version. (DIRECTORY-073 R1).

**Stories:**
- S187 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person setting agent context and reminders, I want Lys to restart a seat onto its new settings and prompt a seat nobody is watching, so that Argus and Manifold are no longer needed for either.

### R2: A proved peer restarts its own held launch

Behavioural. WHEN a peer on the runner socket asks for a restart, THE RUNNER SHALL prove it by socket credentials and ancestry through peer.rs, SHALL restart only the proved session from its held signed Launch with the same credentials, and SHALL record the operation under its stable id without calling the identity server. A session claim in the request SHALL be ignored. An unproved peer SHALL be refused as not_a_session and a session with no held Launch SHALL be refused by name.

**Acceptance:**
- The proved session exits and restarts from its held Launch with unchanged credentials.
- A request naming another session affects only the proved session.
- An unproved peer is refused as not_a_session.
- A replayed operation id answers its kept outcome and never restarts twice.

**Files:**
- create: crates/lys-runner/src/session/restart.rs
- create: crates/lys-runner/src/operations/restart.rs
- create: crates/lys-runner/tests/peer_restart.rs
- create: crates/lys-runner/tests/peer_restart/cases.rs
- modify: crates/lys-runner/src/peer.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/operations.rs

**Checklist:**
- C467 — A seat can restart itself and no other. (DIRECTORY-073 R2).

**Stories:**
- S187 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person setting agent context and reminders, I want Lys to restart a seat onto its new settings and prompt a seat nobody is watching, so that Argus and Manifold are no longer needed for either.

### R3: Compaction and reminders reach a session no screen is reading

Behavioural. WHEN a budget asks for a compaction or notice, or a goal reminder falls due, for a session that no client is reading, THE SYSTEM SHALL deliver it through the runner's input at the turn boundary as it does for a watched session, and SHALL NOT block the runner or any other session while it waits. Each session has its own ordered write path. A full input pipe never holds the shared session table; completion arrives by signal, with no timeout.

**Acceptance:**
- A compaction request reaches a session with no viewer and the session compacts.
- A goal reminder reaches a session with no viewer.
- A second session keeps answering while the first is being written to.
- A signal-controlled full input pipe leaves the shared session table available and a second session answers before that pipe is released.

**Files:**
- create: crates/lys-identity-server/tests/input_no_screen.rs
- create: crates/lys-runner/src/input.rs
- create: crates/lys-runner/tests/input_no_screen/shared_lock.rs
- modify: crates/lys-identity-server/src/budgets_act.rs
- modify: crates/lys-identity-server/src/goals_api.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/operations.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/session/lifecycle.rs

**Checklist:**
- C468 — Compaction requests and reminders reach a session nobody is watching without blocking another. (DIRECTORY-073 R3).

**Stories:**
- S187 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person setting agent context and reminders, I want Lys to restart a seat onto its new settings and prompt a seat nobody is watching, so that Argus and Manifold are no longer needed for either.

## Boundaries

- SHALL NOT poll, sleep, add a timer, timeout, deadline, watchdog or background loop; every wait ends on the event it waits for, and an idle page and an idle service use no CPU.
- SHALL NOT add unsafe, an ignored test, an allow attribute, an underscore rename or a discarded result.
- SHALL NOT change a stored shape without a migration tested from an old install, and SHALL NOT edit an installed configuration by hand.
- SHALL NOT make Lys depend on Cambium or name a harness detail in code; harness details are data.
- SHALL NOT restart a seat that has no reviewed profile version, and SHALL NOT read or send any transcript content.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Built directly by the assigned seat on its own branch in its registered worktree: red first, then green, with fmt, Clippy pedantic, tests and ast-grep.
- The full src_gate runs on Dean at the exact head that lands; the lead reviews that head and gives the landing word.
- Shown to Tom on the dev server at 127.0.0.1:5190 against the live Lys as one step of the night's demonstration.
