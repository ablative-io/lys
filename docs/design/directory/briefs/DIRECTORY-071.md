---
type: brief
id: DIRECTORY-071
cluster: directory
title: Teams nest under a parent team with a lead, and one tree read answers each agent's session and settings
---

# DIRECTORY-071: Teams nest under a parent team with a lead, and one tree read answers each agent's session and settings

> **Cluster:** directory
> **Design anchor:**
> - ADR-119 — A team is provisioned in one act: agents with memories, an opening conversation, a budget, goals and deliverables, and a checker — A team plan is a record: its purpose, a total budget, its deliverables with the evidence each needs, and its members, each with a profile, the memories it starts with, an opening conversation, its share of the budget, its goals and reminders, and a checker (a named person or agent who must accept each deliverable). Provisioning a plan creates the agents, grants, homes, budgets and goals in one act that is all or nothing, and starts them through the runner. A deliverable is met only when its checker accepts it with the evidence.
> - ADR-117 — Lys runs the agents it starts: its own background terminal, or a runner another tool provides — Lys runs the agents it starts. A runner is the part that holds an agent's terminal: Lys ships one of its own, a background process on each machine that runs each started agent in its own pseudo-terminal, and accepts any other runner that speaks the same small, published runner protocol. Through a runner, Lys starts, types into, presses keys in, reads, waits on, resizes, compacts and stops a session, rotates it across its accounts when it reports a usage limit, and wakes it with a message, each act under the caller's grants and recorded. Lys depends on no particular runner: its own is the default, another is chosen per machine.
> - ADR-113 — The identity install and its clients wait on signals, never on a clock — A wait ends on the event it is waiting for or on the failure that makes the event impossible: a service is ready when it says so or when a single connection to it succeeds after a readiness event (its log gaining its listening line, observed through the platform's file-change notification), a stopped process is gone when the platform reports its exit (kqueue EVFILT_PROC on macOS, pidfd on Linux), a peer that refuses or closes is an error at once, and nothing carries a timeout. The screen re-measures on a ResizeObserver or animation-frame signal. Rejected: shorter sleeps, which still wait on a clock; keeping timeouts as a safety net, which the rule forbids and which hides a stuck peer's cause.
> **Checklist:**
> - C462 — The team store reads a team written before nesting as top-level with no lead. (DIRECTORY-071 R1).
> - C463 — The tree read answers only what is under the caller, with each agent's session and settings. (DIRECTORY-071 R2).
> **Stories:**
> - S185 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want one tree of my teams and agents with each one's session and settings, so that I can see and open any of them in one place.

## Purpose

Teams are flat today: a team has an owner and members and no parent (teams_state.rs lines 4 to 9 and 29 to 41), so the organisation Tom runs, himself over Waffles over Archie over Brisket, Crumpet and Pikelet, cannot be drawn. There is no one read that answers who is under whom, which agents are running and what each was started with, so the tree screen has nothing to render.

## Task

Give a team an optional parent team and a lead agent, refusing a cycle by name, with old team records reading as top-level with no lead. Add one read, GET /api/tree, answering the teams the caller owns or leads and everything below them: each team, its lead, its agents, and for each agent whether its session is live, its current goals, the budget it holds and what it has spent against it, and a summary of its latest reviewed profile version. A team's parent and lead decide only what the tree read shows: the owner and lead of a team see it and every team below it. Neither confers any authority, which still comes only from grants. People are not nodes: the person asking is the root and sees only the agents under them. Out: the screen itself, which is designed with Tom at 5190.

## Requirements

### R1: Teams nest under a parent team and name a lead

Behavioural. WHEN a team is created or changed, THE SYSTEM SHALL accept an optional parent team id and an optional lead agent id, SHALL refuse a parent that would make a cycle as team_parent_cycle naming both teams, SHALL refuse a lead that is not a member as team_lead_not_member, and SHALL read every team record written before this change as having no parent and no lead. The parent and lead SHALL be carried by a new versioned team event, so every lys-teams-state/v1 snapshot and log line written before it still reads, and SHALL confer no authority.

**Acceptance:**
- A team created under a parent is answered with that parent.
- Setting a team's parent to its own descendant is refused as team_parent_cycle naming both teams.
- A lead who is not a member is refused by name.
- A team store written by main before this change opens and answers every team with no parent and no lead, tested from that old store.
- Being a team's lead grants nothing that a grant does not.

**Files:**
- create: crates/lys-identity-server/tests/teams_nested.rs
- create: crates/lys-identity-server/tests/support/teams_before_nesting_state.rs
- create: crates/lys-identity-server/tests/support/teams_before_nesting_store.rs
- modify: crates/lys-identity-server/src/teams_state.rs
- modify: crates/lys-identity-server/src/teams_store.rs
- modify: crates/lys-identity-server/src/teams_api.rs
- modify: crates/lys-identity-server/src/teams_migration.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/src/openapi_table.rs

**Checklist:**
- C462 — The team store reads a team written before nesting as top-level with no lead. (DIRECTORY-071 R1).

**Stories:**
- S185 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want one tree of my teams and agents with each one's session and settings, so that I can see and open any of them in one place.

### R2: One tree read answers the caller's teams, agents, live sessions and profile summaries

Behavioural. WHEN a person or agent reads GET /api/tree, THE SYSTEM SHALL answer, from state it already holds and computed on that request, the teams the caller owns or leads and every team below them, each with its lead and agents, and for each agent whether a runner session is live, its current goals, each budget it holds or its team holds with what has been spent against it, its latest reviewed profile version id, and that version's program, model, MCP server names, writable folders and budget. It SHALL NOT include a team or agent outside the caller's reach, and SHALL answer the same shape through the typed client, to an agent reading its own team as to a person.

**Acceptance:**
- Tom's read answers Waffles, Archie and Archie's three seats in their nesting.
- Archie's read answers only Archie's team and below.
- A running seat reads live and a stopped seat reads stopped.
- Each agent carries its latest reviewed profile version id and summary.
- The read starts no task and leaves nothing running after it answers.
- Each agent carries its current goals and its budget with the spend against it.
- Archie, reading as an agent, gets his own team and its members' goals without remembering them.

**Files:**
- create: crates/lys-identity-server/src/tree_api.rs
- create: crates/lys-identity-server/tests/tree_api.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/openapi_types.rs
- modify: crates/lys-identity-server/src/openapi_table.rs

**Checklist:**
- C463 — The tree read answers only what is under the caller, with each agent's session and settings. (DIRECTORY-071 R2).

**Stories:**
- S185 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want one tree of my teams and agents with each one's session and settings, so that I can see and open any of them in one place.

## Boundaries

- SHALL NOT poll, sleep, add a timer, timeout, deadline, watchdog or background loop; every wait ends on the event it waits for, and an idle page and an idle service use no CPU.
- SHALL NOT add unsafe, an ignored test, an allow attribute, an underscore rename or a discarded result.
- SHALL NOT change a stored shape without a migration tested from an old install, and SHALL NOT edit an installed configuration by hand.
- SHALL NOT make Lys depend on Cambium or name a harness detail in code; harness details are data.
- SHALL NOT make people nodes of the tree; the person reading is its root.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Built directly by the assigned seat on its own branch in its registered worktree: red first, then green, with fmt, Clippy pedantic, tests and ast-grep.
- The full src_gate runs on Dean at the exact head that lands; the lead reviews that head and gives the landing word.
- Shown to Tom on the dev server at 127.0.0.1:5190 against the live Lys as one step of the night's demonstration.
