---
type: brief
id: DIRECTORY-073
cluster: directory
title: Lys restarts a seat from its latest reviewed profile version, a seat can ask to restart itself, and prompts reach a seat with no screen open
---

# DIRECTORY-073: Lys restarts a seat from its latest reviewed profile version, a seat can ask to restart itself, and prompts reach a seat with no screen open

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

Add a restart: end the agent's live session, wait on its end event, and start it again from its latest reviewed profile version through the existing start path. Let a session restart itself by presenting its own session credential, and only itself. Prove that a compaction request and a goal reminder reach a session that no screen is reading, after reading Manifold's send-keys to take over what it gets right. Out: new kinds of prompt.

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

**Checklist:**
- C466 — A restart ends the live session and starts it from the latest reviewed version. (DIRECTORY-073 R1).

**Stories:**
- S187 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person setting agent context and reminders, I want Lys to restart a seat onto its new settings and prompt a seat nobody is watching, so that Argus and Manifold are no longer needed for either.

### R2: A seat restarts itself, and only itself

Behavioural. WHEN a session asks for a restart presenting its own session credential, THE SYSTEM SHALL restart that session's agent as in R1, and SHALL refuse a session asking to restart any other agent as restart_not_self naming both.

**Acceptance:**
- A seat asking for its own restart comes back on its latest reviewed version.
- A seat asking to restart another is refused by name.

**Files:**
- modify: crates/lys-identity-server/src/restart_api.rs
- modify: crates/lys-identity-server/tests/restart_api.rs

**Checklist:**
- C467 — A seat can restart itself and no other. (DIRECTORY-073 R2).

**Stories:**
- S187 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person setting agent context and reminders, I want Lys to restart a seat onto its new settings and prompt a seat nobody is watching, so that Argus and Manifold are no longer needed for either.

### R3: Compaction and reminders reach a session no screen is reading

Behavioural. WHEN a budget asks for a compaction or notice, or a goal reminder falls due, for a session that no client is reading, THE SYSTEM SHALL deliver it through the runner's input at the turn boundary as it does for a watched session, and SHALL NOT block the runner or any other session while it waits.

**Acceptance:**
- A compaction request reaches a session with no viewer and the session compacts.
- A goal reminder reaches a session with no viewer.
- A second session keeps answering while the first is being written to.

**Files:**
- create: crates/lys-identity-server/tests/input_no_screen.rs
- modify: crates/lys-identity-server/src/budgets_act.rs
- modify: crates/lys-identity-server/src/goals_api.rs

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
