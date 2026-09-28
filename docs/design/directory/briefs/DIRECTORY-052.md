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

Tom, 28 September 2026 19:55, on Dot: 'I want you to be able to provision new agents so we can provision them off with the right memories, with the right few-shot kind of prompting to start it off, and to be able to send them off to do work without having to know that they've got a budget, know that they've got reminders, know that it's actually going to happen, know that things are actually being checked. So I could say, Waffles, we need a team to work on this. And you could provision them, give them a certain budget, and we can start to budget out things properly.' Tom, 19:56: 'So you would provision a few new agents in a hierarchy, set which of my permissions you're assigning, what their budget allowance is, what account they're going to be assigned to. Which also means the secrets need to be storable and accessible.' Amended 28 September 2026 21:05 by Waffles after Archie's read against the tree.

## Task

Add team plans: a record that names a team's purpose, budget, deliverables and members; provision it in one all-or-nothing act that creates agents, grants, homes with their starting memories and opening conversation, budgets and goals, and starts them through the runner; hold deliverables to a checker's acceptance with evidence; build, provision and watch plans on a Teams screen and through the API.

## Requirements

### R1: A team plan record

Behavioural. A plan names its purpose, its responsible person, its total budget (tokens and time), its deliverables (each with words, a deadline and the evidence that proves it), and its members; each member names a provisioning profile, the memories it starts with (lantern notes or a predecessor's letter from a named home), an opening conversation (turns authored as lys-home fewshot writes them), its share of the budget, its goals with reminders, and its checker (a person or another member), its place in the team's hierarchy (the member it reports to), the grants it is given (each delegated from the provisioning person's own grants, never beyond them, as the delegation form's cannot-give list says), and the account it runs on (a handle to an account in the secrets broker, or an ordered list of handles for rotation). A plan is refused plan_invalid naming the field when shares exceed the total, a member has no checker, or a deliverable names no evidence. A checker who reports, directly or up the chain, to the member it checks is refused checker_reports_to_member, and any cycle of checkers, of any length, is refused checkers_circular naming every member in the cycle: the judged party never holds the pen. A plan's account handle names its kind: a key (a value held by the secrets broker) or a login (a folder of credentials on the runner's machine that never travels and is never copied); rotation over logins is DIRECTORY-050 R5's. A plan is stored as log events with a projection (the checkpoint beside the log, so a start reads the checkpoint and the tail only). Creating, changing or provisioning a plan needs a grant of provision_team held by the person acting, who becomes its responsible person; a plan is read only by its responsible person, its members' checkers and administrators, and anyone else is refused not_your_plan.

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

**Files:**
- create: crates/lys-identity-server/src/team_plans_api.rs
- create: crates/lys-identity-server/src/team_plans_state.rs
- create: crates/lys-identity-server/tests/team_plans.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C387 — A team plan names its purpose, total budget, deliverables with their evidence, and each member's profile, memories, opening conversation, budget share, goals and checker (DIRECTORY-052 R1).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R1 — Execution record

**Dev (recorded):**

- Status: blocked
- How: None of R1's nine acceptance rows are met; nothing was written. The team-plans routes (POST/GET /team-plans, GET /team-plans/{id}) would each need an entry in crates/lys-identity-server/src/openapi_table.rs and src/openapi_types.rs. Without them, crates/lys-identity-server/tests/openapi.rs fails naming each route that has no entry (src/openapi.rs:12-15). The named refusals R1 requires (plan_invalid, checkers_circular, checker_reports_to_member, not_your_plan, account_unknown) need new ServerError variants in src/error.rs, status codes in src/error_status.rs and refusal names in src/openapi_refusals.rs. None of those five files is in R1's wall (team_plans_api.rs, team_plans_state.rs, tests/team_plans.rs, routes.rs, lib.rs). provision_team is not a permission anywhere in the tree (a search of crates/ and surface/identity/src finds nothing), so adding it means changing the lys app schema, which is also outside the wall. A member's budget share and its goals with reminders are shaped by DIRECTORY-051 R2 and R4, and neither is in the tree (budgets_state.rs and goals_state.rs are missing). Under CN9 the row stops here and names these files until the reviewer approves a brief revision that adds them.
- Deviation: Nothing was implemented. The files R1 needs outside its wall are named in how, as CN9 requires.
- Checklist delivery:
  - [ ] C387 — A team plan names its purpose, total budget, deliverables with their evidence, and each member's profile, memories, opening conversation, budget share, goals and checker (DIRECTORY-052 R1). — Blocked: routes need openapi_table.rs and openapi_types.rs, refusals need error.rs, error_status.rs and openapi_refusals.rs, and budget and goal shapes come from DIRECTORY-051. All are outside the wall or missing.
- Story delivery:
  - [ ] S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford. — Blocked on DIRECTORY-051 and on the brief revision CN9 requires.

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

**Checklist:**
- C388 — Provisioning a plan creates every agent, grant, home, budget and goal in one all-or-nothing act and starts them (DIRECTORY-052 R2).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R2 — Execution record

**Dev (recorded):**

- Status: blocked
- How: None of R2's eleven acceptance rows are met. Provisioning creates each member's budgets (DIRECTORY-051 R2) and goals (DIRECTORY-051 R4), and none of their files exist: crates/lys-identity-server/src/budgets_api.rs, budgets_state.rs, budgets_store.rs, goals_api.rs, goals_state.rs and goals_store.rs are all missing. R2's restart-safety rows need the runner to durably record a start's outcome under a stable per-operation identity before it answers, and to read those records back. That runner work is DIRECTORY-051 R3's crates/lys-runner/src/operations.rs, which is missing, and lys-runner is outside R2's wall. The route POST /team-plans/{id}/provision has the same openapi_table.rs, openapi_types.rs and error.rs gap described under R1. R2 also builds on R1, which is blocked.
- Deviation: Nothing was implemented, because the brief's dependency DIRECTORY-051 is not in the tree.
- Checklist delivery:
  - [ ] C388 — Provisioning a plan creates every agent, grant, home, budget and goal in one all-or-nothing act and starts them (DIRECTORY-052 R2). — Blocked on DIRECTORY-051 R2, R3 and R4 (budgets, the runner's per-operation start records, goals) and on R1.
- Story delivery:
  - [ ] S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford. — Blocked on DIRECTORY-051.

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
- How: Row 7 (fewshot_write.rs writes turns through the public library function and reads them back as the command writes them) is met: crates/lys-home/tests/fewshot_write.rs:52-86 compares the library's output with the command's, record for record. The public function is crates/lys-home/src/cli/fewshot.rs:23, re-exported at crates/lys-home/src/cli.rs:18 and crates/lys-home/src/lib.rs:30, and the command calls it at crates/lys-home/src/cli.rs:340-341. Row 4 (the opening conversation is written as fewshot turns through cli/fewshot.rs) is met on the lys-home side: that writer is now reachable from another crate. The call from provisioning, in team_plans_provision.rs, is not written because R2 is blocked. Rows 1, 2, 5 and 6 are about a provisioned member's first turn and its given record, and row 3 is about writing starting memory into the member's home through lantern.rs. All of them happen during provisioning, which is R2 and is blocked on DIRECTORY-051, so given.rs and lantern.rs were not changed.
- Deviation: Only the lys-home library exposure and its test were built. The given-record digests, the lantern write of starting memory and the provisioning call site wait for R2, which is blocked on DIRECTORY-051.
- Files changed:
  - modified: `crates/lys-home/src/cli/fewshot.rs` — Now holds the one fewshot writer as `pub fn write_fewshot(out, turns: &[(Role, String)], cwd)`, taking parsed turns instead of a turns-file path; what it writes and its exclusive-create and sync behaviour are unchanged.
  - modified: `crates/lys-home/src/cli.rs` — Re-exports `write_fewshot` from the private fewshot module; the `fewshot` command parses its turns file with `parse_turns` and calls `crate::write_fewshot`.
  - modified: `crates/lys-home/src/lib.rs` — Exposes `write_fewshot` and `Role` at the crate root, so the identity server can call the same writer across the crate boundary.
  - created: `crates/lys-home/tests/fewshot_write.rs` — Writes turns through `lys_home::write_fewshot`, runs the `fewshot` command on the same turns from a file, and asserts the two outputs match record for record (fresh ids and timestamps aside), with the parent chain and session id intact. A second test asserts the writer refuses with `Exists` and leaves an existing transcript untouched.
- Checklist delivery:
  - [ ] C389 — Each member starts with its chosen memories and its opening conversation already in its session (DIRECTORY-052 R3). — The lys-home writer is exposed and tested (rows 4 and 7). The rows about a member's first turn and given record ride on the blocked provisioning.
- Story delivery:
  - [ ] S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford. — Blocked on DIRECTORY-051 through R2.

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

**Checklist:**
- C390 — A deliverable is met only when its checker accepts it with the named evidence; the team's spend is held to its total (DIRECTORY-052 R4).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R4 — Execution record

**Dev (recorded):**

- Status: blocked
- How: None of R4's three acceptance rows are met. The brief says to modify crates/lys-identity-server/src/goals_api.rs and src/budgets_act.rs, and neither file exists: DIRECTORY-051 R4 creates goals_api.rs and DIRECTORY-051 R3 creates budgets_act.rs, and 051 is not implemented. Creating them here would build 051's rows outside this brief's wall. The refusal evidence_missing would also need error.rs and openapi_refusals.rs, as under R1.
- Deviation: Nothing was implemented. The two files this requirement modifies do not exist yet.
- Checklist delivery:
  - [ ] C390 — A deliverable is met only when its checker accepts it with the named evidence; the team's spend is held to its total (DIRECTORY-052 R4). — Blocked: goals_api.rs and budgets_act.rs are DIRECTORY-051's, and it has not landed.
- Story delivery:
  - [ ] S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford. — Blocked on DIRECTORY-051.

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

**Checklist:**
- C391 — A Teams screen builds a plan from a template, provisions it, and shows each member's state, spend, goals and deliverables (DIRECTORY-052 R5).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R5 — Execution record

**Dev (recorded):**

- Status: blocked
- How: None of R5's nine acceptance rows are met. The screen builds, provisions and watches plans through R1's and R2's API, and both are blocked. It shows spend and goals per member, which come from DIRECTORY-051's budgets and goals, not in the tree. The surface reaches the service through surface/identity/src/api.ts and the generated OpenAPI types, neither of which is in this row's wall. A screen built against an API that does not exist would only agree with its own mocks.
- Deviation: Nothing was implemented, because the API behind the screen is blocked.
- Checklist delivery:
  - [ ] C391 — A Teams screen builds a plan from a template, provisions it, and shows each member's state, spend, goals and deliverables (DIRECTORY-052 R5). — Blocked on R1, R2 and DIRECTORY-051, and api.ts is outside the wall.
- Story delivery:
  - [ ] S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford. — Blocked.

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

**Checklist:**
- C392 — Accounts and secrets are stored once in the broker and assigned to members by handle; values are never shown again (DIRECTORY-052 R6).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

#### R6 — Execution record

**Dev (recorded):**

- Status: blocked
- How: None of R6's three acceptance rows are met. Row 1 (a stored account assigned to a member, and the member runs on it) needs provisioning, R2, which is blocked. Row 3 (a member reaches its account through the broker's proxy, with no secret in its process environment) needs a member started by the runner, which is also R2, and the proxy wiring from R4, which is blocked on DIRECTORY-051. The screen half lives in TeamPlans.tsx, which R5 creates, and R5 is blocked. I did not change secrets_api.rs on its own: every row of R6 rides on assigning a secret to a plan member, and no plan exists.
- Deviation: Nothing was implemented, because every row rides on the blocked plan and provisioning work.
- Checklist delivery:
  - [ ] C392 — Accounts and secrets are stored once in the broker and assigned to members by handle; values are never shown again (DIRECTORY-052 R6). — Blocked on R2, R4 and R5.
- Story delivery:
  - [ ] S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford. — Blocked.

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
