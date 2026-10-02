---
type: brief
id: DIRECTORY-052
cluster: directory
title: Provision a working team in one act: agents with memories, an opening conversation, a budget, goals, deliverables and a checker
---

# DIRECTORY-052: Provision a working team in one act: agents with memories, an opening conversation, a budget, goals, deliverables and a checker

> **Cluster:** directory
> **Depends on:** DIRECTORY-048, DIRECTORY-050, DIRECTORY-051
> **Design anchor:**
> - ADR-119 — A team is provisioned in one act: agents with memories, an opening conversation, a budget, goals and deliverables, and a checker — A team plan is a record: its purpose, a total budget, its deliverables with the evidence each needs, and its members, each with a profile, the memories it starts with, an opening conversation, its share of the budget, its goals and reminders, and a checker (a named person or agent who must accept each deliverable). Provisioning a plan creates the agents, grants, homes, budgets and goals in one act that is all or nothing, and starts them through the runner. A deliverable is met only when its checker accepts it with the evidence.
> **Checklist:**
> - C387 — A team plan names its purpose, total budget, deliverables with their evidence, and each member's profile, memories, opening conversation, budget share, goals and checker (DIRECTORY-052 R1).
> - C388 — Provisioning a plan creates every agent, grant, home, budget and goal in one all-or-nothing act and starts them (DIRECTORY-052 R2).
> - C389 — Each member starts with its chosen memories and its opening conversation already in its session (DIRECTORY-052 R3).
> - C390 — A deliverable is met only when its checker accepts it with the named evidence; the team's spend is held to its total (DIRECTORY-052 R4).
> - C391 — A Teams screen builds a plan from a template, provisions it, and shows each member's state, spend, goals and deliverables (DIRECTORY-052 R5).
> - C392 — Accounts and secrets are stored once in the broker and assigned to members by handle; values are never shown again (DIRECTORY-052 R6).
> **Stories:**
> - S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

## Purpose

Tom, 28 September 2026 19:55, on Dot: 'I want you to be able to provision new agents so we can provision them off with the right memories, with the right few-shot kind of prompting to start it off, and to be able to send them off to do work without having to know that they've got a budget, know that they've got reminders, know that it's actually going to happen, know that things are actually being checked. So I could say, Waffles, we need a team to work on this. And you could provision them, give them a certain budget, and we can start to budget out things properly.' Tom, 19:56: 'So you would provision a few new agents in a hierarchy, set which of my permissions you're assigning, what their budget allowance is, what account they're going to be assigned to. Which also means the secrets need to be storable and accessible.' Amended 28 September 2026 21:05 by Waffles after Archie's read against the tree. Amended 29 September 2026 by Gypsy from the measured author blockers in af8e4bf3, sequence 58, to include the API registration, refusal, permission-model and surface type owners required by these same behaviours.

## Task

Add team plans: a record that names a team's purpose, budget, deliverables and members; provision it in one all-or-nothing act that creates agents, grants, homes with their starting memories and opening conversation, budgets and goals, and starts them through the runner; hold deliverables to a checker's acceptance with evidence; build, provision and watch plans on a Teams screen and through the API. DIRECTORY-051 is a code dependency. Its brief commit is not evidence of implementation. Integrate its actual implementation before measuring completion of these requirements. Preserve the one owner of budgets, goals and runner operations.

## Requirements

### R1: A team plan record

Behavioural. A plan names its purpose, its responsible person, its total budget (tokens and time), its deliverables (each with words, a deadline and the evidence that proves it), and its members; each member names a provisioning profile, the memories it starts with (lantern notes or a predecessor's letter from a named home), an opening conversation (turns authored as lys-home fewshot writes them), its share of the budget, its goals with reminders, and its checker (a person or another member), its place in the team's hierarchy (the member it reports to), the grants it is given (each delegated from the provisioning person's own grants, never beyond them, as the delegation form's cannot-give list says), and the account it runs on (a handle to an account in the secrets broker, or an ordered list of handles for rotation). A plan is refused plan_invalid naming the field when shares exceed the total, a member has no checker, or a deliverable names no evidence. A checker who reports, directly or up the chain, to the member it checks is refused checker_reports_to_member, and any cycle of checkers, of any length, is refused checkers_circular naming every member in the cycle: the judged party never holds the pen. A plan's account handle names its kind: a key (a value held by the secrets broker) or a login (a folder of credentials on the runner's machine that never travels and is never copied); rotation over logins is DIRECTORY-050 R5's. A plan is stored as log events with a projection (the checkpoint beside the log, so a start reads the checkpoint and the tail only). Creating, changing or provisioning a plan needs a grant of provision_team held by the person acting, who becomes its responsible person; a plan is read only by its responsible person, its members' checkers and administrators, and anyone else is refused not_your_plan. Register these routes, bodies, answers and named refusals in the existing OpenAPI tables and error owners. Include provision_team in the fresh-install permission model. For an established install, the apps log remains authoritative and the administrator uses the existing versioned schema-change route to add the permission. Neither service startup nor this card silently rewrites an established schema or grants the action to a caller. Exercise plan access through the existing grant authority.

**Acceptance:**
- A plan whose member shares exceed its total is refused naming the total.
- A member with no checker is refused naming the member.
- A member given a grant the provisioning person does not hold is refused, naming the grant and the reason from cannot-give.
- A member naming an account handle the broker does not hold is refused account_unknown.
- Three members who check each other in a ring (A checks B, B checks C, C checks A) are refused checkers_circular naming all three.
- A person without provision_team who creates a plan is refused, and nothing is written.
- A plan is read back unchanged after the service restarts.
- That restart reads no plan event from before the checkpoint.
- A person who is not the plan's responsible person, one of its checkers or an administrator is refused not_your_plan when reading it.
- The OpenAPI route coverage test includes every new team-plan route and its named refusals.
- A fresh-install model can represent provision_team. An established schema without that action remains unchanged until an administrator records its versioned schema change.
- After that explicit schema change and a valid grant, the granted caller can create a plan. An otherwise identical caller without the grant remains refused.

**Files:**
- create: crates/lys-identity-server/src/team_plans_api.rs
- create: crates/lys-identity-server/src/team_plans_state.rs
- create: crates/lys-identity-server/tests/team_plans.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/openapi_types.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/src/openapi_refusals.rs
- modify: crates/lys-identity-server/tests/openapi.rs
- modify: crates/lys/src/identity/install/layout.rs
- modify: crates/lys-identity-server/src/apps_schema_api.rs
- modify: crates/lys-identity-server/tests/apps_schema.rs

**Checklist:**
- C387 — A team plan names its purpose, total budget, deliverables with their evidence, and each member's profile, memories, opening conversation, budget share, goals and checker (DIRECTORY-052 R1).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R1 — Execution record

**Dev (recorded):**

- Status: blocked
- How: No team plan code was written. This round only made the base compile, so the gate can measure the card's own work. The team plan record (team_plans_api.rs, team_plans_state.rs, tests/team_plans.rs) needs two things first: a budget share and goals shape from DIRECTORY-051, and routes and refusals whose OpenAPI and error entries are outside this wall.
- Deviation: Four files outside R1's wall were edited: stop_api.rs, grant_contract/views.rs, provisioning_store.rs and the ToSchema-side of the base. routes.rs is inside the wall. These edits fix measured base compile errors. They add no team plan behaviour, and they use no allow, cfg or ignore. The review asked for every measured failure to be fixed at its cause in this directory. They are declared here so the reviewer can approve them or move them to their own card. Still needed before R1 can start: DIRECTORY-051 landed, and a CN9 brief revision adding openapi_table.rs, openapi_types.rs, error.rs, error_status.rs, openapi_refusals.rs and the file defining the provision_team permission.
- Files changed:
  - modified: `crates/lys-identity-server/src/routes.rs` — service_saying passes Arc::clone(&key) to apps_api::opened, so the key is still held when Runners::new takes it. This fixes E0382 at routes.rs:181:53, 'the type `Arc` does not implement `Copy`'.
  - modified: `crates/lys-identity-server/src/stop_api.rs` — ConfirmedEnd derives utoipa::ToSchema, fixing E0277 at stop_api.rs:91:33 ('the trait bound `ConfirmedEnd: ToSchema` is not satisfied').
  - modified: `crates/lys-identity-server/src/grant_contract/views.rs` — UnreportedView, StandingView and the RefusedView that StandingView flattens now derive utoipa::ToSchema. This fixes E0277 at views.rs:117:28 and views.rs:208:19.
  - modified: `crates/lys-identity-server/src/provisioning_store.rs` — SessionSettings derives utoipa::ToSchema, fixing E0277 at provisioning_api.rs:136:21. Its lys_runner::Rotation field is given #[schema(value_type = Option<Object>)], the same pattern apps_views.rs uses for foreign types. lys-runner does not depend on utoipa.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] A plan whose member shares exceed its total is refused naming the total. — No team_plans_api.rs or team_plans_state.rs in crates/lys-identity-server/src (ls shows only teams_api/state/store.rs, which belong to other work); nothing in the diff.
  - [ ] A member with no checker is refused naming the member. — Not implemented; no team plan code in the diff.
  - [ ] A member given a grant the provisioning person does not hold is refused, naming the grant and the reason from cannot-give. — Not implemented.
  - [ ] A member naming an account handle the broker does not hold is refused account_unknown. — Not implemented; no account_unknown refusal exists.
  - [ ] Three members who check each other in a ring (A checks B, B checks C, C checks A) are refused checkers_circular naming all three. — Not implemented.
  - [ ] A person without provision_team who creates a plan is refused, and nothing is written. — grep -rn provision_team crates surface/identity/src returns nothing.
  - [ ] A plan is read back unchanged after the service restarts. — Not implemented; crates/lys-identity-server/tests/team_plans.rs does not exist.
  - [ ] That restart reads no plan event from before the checkpoint. — Not implemented.
  - [ ] A person who is not the plan's responsible person, one of its checkers or an administrator is refused not_your_plan when reading it. — Not implemented.
- Issues:
  - Build the team plan record in team_plans_api.rs and team_plans_state.rs with tests/team_plans.rs covering all nine rows; it is entirely unbuilt.
  - DIRECTORY-051 (a declared depends_on of this brief) has not landed: budgets_state.rs, goals_state.rs are missing, so a member's budget share and goals have no shape. Land DIRECTORY-051 first; this brief cannot be dispatched until then (compare CN12's rule on dependency-blocked briefs).
  - R1's file wall omits files the work must touch: openapi_table.rs, openapi_types.rs (tests/openapi.rs requires every route to have an entry), error.rs, error_status.rs, openapi_refusals.rs (for plan_invalid, checkers_circular, checker_reports_to_member, not_your_plan, account_unknown) and wherever the provision_team permission is defined. A reviewed brief revision adding them to the wall is required under CN9 before implementation; the developer correctly stopped and named them.

### R2: Provision in one all-or-nothing act

Behavioural. POST /team-plans/{id}/provision creates, under the responsible person's grants: each agent, its grants as the profile names, its home, its budgets (DIRECTORY-051 R2) and goals (R4), then starts each through the runner (DIRECTORY-050 R3). Provisioning runs in three passes: check every member and step, then create every record, then start every agent, so no agent starts until every member exists. If a step is refused after records exist, or a member fails to start after others have started, every started member is stopped through the runner and its process's end is taken from the runner's own exit record (never assumed from the stop request), then each created agent is retired and its grants revoked by recorded acts, so nothing of theirs runs or can be used (a log keeps what happened; nothing is unmade), and the refusal names the member and the step; a second provision under the same operation id answers what was done and does nothing twice. The operation id and each pass's progress are recorded before the pass acts, so a provision cut off by a restart and sent again with the same id finishes or rolls back from where it stood, and never starts a member twice. Each member's start carries a stable per-member operation identity (the operation id and the member) to the runner, which records the start's outcome under it durably before it answers; a provision resumed after a restart first reads the runner's start records for each member's identity and reconciles with them, so a member whose process started but whose answer never arrived is recognised, not started again.

**Acceptance:**
- Provisioning a three-member plan leaves three running agents with their budgets and goals.
- A refusal at the third member leaves the first two retired with their grants revoked and never started, naming the member and step.
- Sending the same operation twice does nothing the second time.
- A start that fails at the third member, after the first two started, leaves the first two stopped.
- That test finds the runner's exit record for each of the first two processes.
- That test finds the first two members retired.
- That test finds the first two members' grants revoked.
- That refusal names the third member and the start step.
- A provision interrupted by a service restart between the record and start passes, sent again with the same operation id, starts each member exactly once.
- A provision cut off after the runner started a member but before its answer was recorded, sent again with the same operation id after a restart, starts that member no second time.
- That resumed provision records the member as started from the runner's own start record.

**Files:**
- create: crates/lys-identity-server/src/team_plans_provision.rs
- modify: crates/lys-identity-server/src/team_plans_api.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/tests/team_plans.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/openapi_types.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/src/openapi_refusals.rs
- modify: crates/lys-identity-server/tests/openapi.rs

**Checklist:**
- C388 — Provisioning a plan creates every agent, grant, home, budget and goal in one all-or-nothing act and starts them (DIRECTORY-052 R2).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R2 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Not built. All-or-nothing provisioning in team_plans_provision.rs needs DIRECTORY-051's budgets and goals, and the runner's durable per-operation start records (crates/lys-runner/src/operations.rs), which are not in the tree. POST /team-plans/{id}/provision also needs the same out-of-wall OpenAPI and error files as R1.
- Deviation: Blocked on DIRECTORY-051 (CN12) and on a CN9 brief revision for the OpenAPI and error files.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] Provisioning a three-member plan leaves three running agents with their budgets and goals. — No team_plans_provision.rs; nothing in the diff.
  - [ ] A refusal at the third member leaves the first two retired with their grants revoked and never started, naming the member and step. — Not implemented.
  - [ ] Sending the same operation twice does nothing the second time. — Not implemented.
  - [ ] A start that fails at the third member, after the first two started, leaves the first two stopped. — Not implemented.
  - [ ] That test finds the runner's exit record for each of the first two processes. — Not implemented.
  - [ ] That test finds the first two members retired. — Not implemented.
  - [ ] That test finds the first two members' grants revoked. — Not implemented.
  - [ ] That refusal names the third member and the start step. — Not implemented.
  - [ ] A provision interrupted by a service restart between the record and start passes, sent again with the same operation id, starts each member exactly once. — Not implemented.
  - [ ] A provision cut off after the runner started a member but before its answer was recorded, sent again with the same operation id after a restart, starts that member no second time. — Not implemented; crates/lys-runner/src has no operations.rs (ls).
  - [ ] That resumed provision records the member as started from the runner's own start record. — Not implemented.
- Issues:
  - Build all-or-nothing provisioning in team_plans_provision.rs with its eleven rows tested; entirely unbuilt.
  - Depends on DIRECTORY-051 R2/R3/R4 (budgets, the runner's durable per-operation start records in crates/lys-runner/src/operations.rs, goals), none of which is in the tree. Land DIRECTORY-051 first.
  - Same out-of-wall OpenAPI/error files as R1 for POST /team-plans/{id}/provision; needs the same reviewed brief revision.

### R3: Starting memories and opening conversation

Behavioural. Each member's home is created with the memories the plan names copied in as its inherited memory, and its session starts by resuming the opening conversation written for it, so its first turn already holds them. The render records both in the session's given record (lys.given). Writing fewshot turns is exposed from lys-home's library (the writer the fewshot command uses, moved behind a public library function in lib.rs and called by the command in cli.rs), so the identity server calls the same code across the crate boundary; crates/lys-home/tests/fewshot_write.rs calls it through the public function.

**Acceptance:**
- With a fixture harness, the input given to a member's first turn holds its starting memory text and its opening conversation, and no model is called.
- The given record names both by digest.
- The starting memory is written into the member's home as lantern notes through crates/lys-home/src/record/lantern.rs.
- The opening conversation is written as fewshot turns through crates/lys-home/src/cli/fewshot.rs.
- The rendered first turn shows the starting memory text.
- The rendered first turn shows the opening conversation.
- crates/lys-home/tests/fewshot_write.rs writes turns through lys-home's public library function and reads them back as the fewshot command writes them.

**Files:**
- create: crates/lys-home/tests/fewshot_write.rs
- modify: crates/lys-home/src/record/given.rs
- modify: crates/lys-identity-server/src/team_plans_provision.rs
- modify: crates/lys-home/src/record/lantern.rs
- modify: crates/lys-home/src/cli/fewshot.rs
- modify: crates/lys-home/src/lib.rs
- modify: crates/lys-home/src/cli.rs

**Checklist:**
- C389 — Each member starts with its chosen memories and its opening conversation already in its session (DIRECTORY-052 R3).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R3 — Execution record

**Dev (recorded):**

- Status: blocked
- How: The lys-home half is done: the fewshot writer the command uses is a public library function, and a test holds it to the command's output. The review confirmed this refactor is correct and within the wall. The rest is still to do: calling the writer and lantern.rs from team_plans_provision.rs, recording both digests in given.rs, and the fixture-harness first-turn test. All of it rides on R2.
- Deviation: The member-side rows are blocked with R2, on DIRECTORY-051.
- Files changed:
  - modified: `crates/lys-home/src/cli/fewshot.rs` — Adds pub fn write_fewshot(out, turns, cwd). It writes the turns as a Claude Code JSONL transcript with create_new, refuses an existing file as HomeError::Exists, and syncs the file and its directory.
  - modified: `crates/lys-home/src/cli.rs` — Re-exports write_fewshot. The fewshot command parses its turns file and then calls the same library writer.
  - modified: `crates/lys-home/src/lib.rs` — pub use cli::write_fewshot and Role, so the identity server can write a member's opening conversation across the crate boundary.
  - created: `crates/lys-home/tests/fewshot_write.rs` — The command is the second party: turns written through the library read back record for record as the fewshot command writes them from a file. A second test checks the library writer never writes over an existing transcript.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] With a fixture harness, the input given to a member's first turn holds its starting memory text and its opening conversation, and no model is called. — No provisioning exists; no such test.
  - [ ] The given record names both by digest. — crates/lys-home/src/record/given.rs unchanged.
  - [ ] The starting memory is written into the member's home as lantern notes through crates/lys-home/src/record/lantern.rs. — lantern.rs unchanged; no caller.
  - [ ] The opening conversation is written as fewshot turns through crates/lys-home/src/cli/fewshot.rs. — Only half: crates/lys-home/src/cli/fewshot.rs:23 now exposes pub fn write_fewshot, re-exported at src/lib.rs (pub use cli::write_fewshot); no provisioning call site writes a member's opening conversation through it.
  - [ ] The rendered first turn shows the starting memory text. — Not implemented.
  - [ ] The rendered first turn shows the opening conversation. — Not implemented.
  - [x] crates/lys-home/tests/fewshot_write.rs writes turns through lys-home's public library function and reads them back as the fewshot command writes them. — crates/lys-home/tests/fewshot_write.rs turns_written_through_the_library_read_back_as_the_command_writes_them compares lys_home::write_fewshot output with the fewshot command's output record for record (fresh ids and timestamps removed), plus parent chain and session id; the_library_writer_never_writes_over_a_transcript asserts HomeError::Exists and untouched content. `cargo test -p lys-home --test fewshot_write`: 2 passed; `cargo clippy -p lys-home --all-targets -- -D warnings` clean.
- Issues:
  - The lys-home refactor is correct and within R3's wall (fewshot.rs, cli.rs, lib.rs, tests/fewshot_write.rs); behaviour of the fewshot command is preserved (parse_turns then write_fewshot).
  - Still to do: write each member's starting memory as lantern notes through lantern.rs, write its opening conversation through lys_home::write_fewshot from team_plans_provision.rs, record both by digest in the given record (given.rs), and test with a fixture harness that the first turn holds and renders both with no model call. These ride on R2, which is blocked on DIRECTORY-051.

### R4: Deliverables met only by the checker, with evidence; spend held to the total

Behavioural. A member marks a deliverable ready with its evidence (a commit on a named branch, a document, a check that passed); it is met only when its checker accepts it, and the checker may send it back with words. The team's spend is the sum of its members'; when it reaches the plan's total, every member is stopped or the responsible person told, as the plan says. Secrets (R6 in the plan's records): a value given to Lys passes through the identity service to the broker, and the service keeps none of it and never logs that route's body. An agent reaches a secret through the broker's proxy, which adds the credential to the call so the agent never holds it, wherever the harness takes a base address; only a profile whose harness cannot takes it in its environment, and the profile names that.

**Acceptance:**
- A deliverable marked ready without evidence is refused evidence_missing.
- Only the named checker can accept it.
- The team total reached stops every member when the plan says stop.

**Files:**
- modify: crates/lys-identity-server/src/goals_api.rs
- modify: crates/lys-identity-server/src/budgets_act.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/openapi_types.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/src/openapi_refusals.rs
- modify: crates/lys-identity-server/tests/team_plans.rs
- modify: crates/lys-identity-server/tests/openapi.rs

**Checklist:**
- C390 — A deliverable is met only when its checker accepts it with the named evidence; the team's spend is held to its total (DIRECTORY-052 R4).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R4 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Not built. goals_api.rs and budgets_act.rs, the two files R4 modifies, are created by DIRECTORY-051, which has not landed.
- Deviation: Blocked on DIRECTORY-051. The evidence_missing refusal also needs error.rs, error_status.rs and openapi_refusals.rs, which are outside the wall and must be added by CN9 revision.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] A deliverable marked ready without evidence is refused evidence_missing. — Not implemented; goals_api.rs does not exist in crates/lys-identity-server/src.
  - [ ] Only the named checker can accept it. — Not implemented.
  - [ ] The team total reached stops every member when the plan says stop. — Not implemented; budgets_act.rs does not exist.
- Issues:
  - The two files R4 modifies (goals_api.rs, budgets_act.rs) are created by DIRECTORY-051, which has not landed. Land DIRECTORY-051, then implement deliverable acceptance by checker with evidence and the team-total stop.
  - evidence_missing needs error.rs/error_status.rs/openapi_refusals.rs, outside the wall; include them in the brief revision.

### R5: Teams screen

Behavioural. A Teams screen builds a plan from a template (for example: a builder and a reviewer; a lead with three builders and a checker), provisions it, and shows each member running or stopped, its spend against its share, its goals and the deliverables with their state, styled from the design tokens with no default controls.

**Acceptance:**
- A plan built from a template on the screen provisions and appears with every member running.
- The screen shows spend, goals and deliverables per member.
- Surface tests cover building, provisioning and the refusals.
- The Teams screen is reached from the rail.
- The Teams screen is reached at its own route.
- surface/identity/tests/team-plans.test.tsx covers reaching the screen.
- That test file covers building a plan from a template.
- That test file covers provisioning a plan.
- That test file covers each refusal the screen can show.

**Files:**
- create: surface/identity/src/features/team-plans/TeamPlans.tsx
- create: surface/identity/src/features/team-plans/team-plans.css
- create: surface/identity/tests/team-plans.test.tsx
- modify: surface/identity/src/routes.tsx
- modify: surface/identity/src/shell/railItems.ts
- modify: surface/identity/src/api.ts
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C391 — A Teams screen builds a plan from a template, provisions it, and shows each member's state, spend, goals and deliverables (DIRECTORY-052 R5).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R5 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Not built. The Teams screen, its route, its rail entry and team-plans.test.tsx need the R1 and R2 API to exist.
- Deviation: Blocked on R1 and R2. surface/identity/src/api.ts and the generated OpenAPI types must be added to the wall by CN9 revision.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] A plan built from a template on the screen provisions and appears with every member running. — surface/identity/src/features/team-plans/ does not exist.
  - [ ] The screen shows spend, goals and deliverables per member. — Not implemented.
  - [ ] Surface tests cover building, provisioning and the refusals. — Not implemented.
  - [ ] The Teams screen is reached from the rail. — railItems.ts unchanged.
  - [ ] The Teams screen is reached at its own route. — routes.tsx unchanged.
  - [ ] surface/identity/tests/team-plans.test.tsx covers reaching the screen. — File does not exist.
  - [ ] That test file covers building a plan from a template. — File does not exist.
  - [ ] That test file covers provisioning a plan. — File does not exist.
  - [ ] That test file covers each refusal the screen can show. — File does not exist.
- Issues:
  - Build the Teams screen, route, rail entry and team-plans.test.tsx once R1/R2 API exists; surface/identity/src/api.ts and generated OpenAPI types must be added to the wall by brief revision.

### R6: Accounts and secrets stored and reached by handle

Behavioural. The Teams screen and the API let the responsible person store a model account or other secret in the secrets broker (lys-secrets), under a scope, and assign it to members by handle; the secret's value is entered once, never shown again, and reaches the member only as R4 orders it: through the broker's proxy wherever the harness takes a base address, and in the process environment at start only for a profile that names its harness as unable to take one. Listing shows handles, scopes, holders and last use, never values.

**Acceptance:**
- An account stored on the screen is assignable to a member and the member runs on it.
- No answer, screen or log shows the value after it is stored, checked by a test.
- A member whose harness takes a base address reaches its account through the proxy, and its process environment holds no secret value, checked by a test.

**Files:**
- modify: surface/identity/src/features/team-plans/TeamPlans.tsx
- modify: crates/lys-identity-server/src/secrets_api.rs
- modify: surface/identity/src/api.ts
- modify: surface/identity/src/generated/index.ts
- modify: surface/identity/tests/team-plans.test.tsx

**Checklist:**
- C392 — Accounts and secrets are stored once in the broker and assigned to members by handle; values are never shown again (DIRECTORY-052 R6).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R6 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Not built. Storing and assigning accounts by handle, the no-value-shown test and the proxy/no-env-secret test all depend on R2, R4 and R5.
- Deviation: Blocked on R2, R4 and R5, and so on DIRECTORY-051.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] An account stored on the screen is assignable to a member and the member runs on it. — Not implemented; secrets_api.rs unchanged.
  - [ ] No answer, screen or log shows the value after it is stored, checked by a test. — Not implemented.
  - [ ] A member whose harness takes a base address reaches its account through the proxy, and its process environment holds no secret value, checked by a test. — Not implemented.
- Issues:
  - Implement storing and assigning accounts by handle, the no-value-shown test and the proxy/no-env-secret test; blocked on R2, R4, R5.

## Boundaries

- SHALL NOT start any member without a budget and a checker.
- SHALL NOT leave a partly provisioned team: all or nothing.
- SHALL NOT mark a deliverable met without its checker's acceptance and the evidence.
- SHALL NOT add a timeout, deadline on a request, poll, #[allow], #[ignore] or any bypass.
- SHALL NOT add a silent fallback: every failure is a named refusal.

## Verification

- The full Lys gate, ast-grep scan and the surface checks exit 0 at the card's head, measured by the card round.
- On a scratch install: build a two-member plan from a template, provision it, see both running with budgets, mark and accept a deliverable.
