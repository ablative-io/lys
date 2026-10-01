---
type: brief
id: DIRECTORY-077
cluster: directory
title: An agent Lys starts acts through MCP with a pass for that run, can be granted any action a person can take, and asks the person who can grant what it is refused
---

# DIRECTORY-077: An agent Lys starts acts through MCP with a pass for that run, can be granted any action a person can take, and asks the person who can grant what it is refused

> **Cluster:** directory
> **Design anchor:**
> - ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.
> - ADR-133 — The launch is everything Lys records for the seat, for Claude Code and our Codex build — A provisioning profile declares its harness build; the launch renders every field for that build or the profile is refused by name when it is recorded. Command MCP servers carry secrets only as handles. Claude Code permissions come from the profile and the Tool policy; Codex permissions are DIRECTORY-065's render of the same policy.
> **Checklist:**
> - C476 — Every action a person can take in Lys is a permission an agent can be granted. (DIRECTORY-077 R1).
> - C477 — Lys issues an agent a pass for one run when it starts it, written into the seat's own config, and ends it when the run ends. (DIRECTORY-077 R2).
> - C478 — Every route takes the pass as the agent and judges each call against the agent's live grants, except the few responsibilities a person keeps. (DIRECTORY-077 R3).
> - C479 — A refusal names who can grant the action, walking up the chain. (DIRECTORY-077 R4).
> - C480 — An agent asks that person, and they answer once, for a while, ongoing or no. (DIRECTORY-077 R5).
> - C481 — Grants can be one-time, and can be transferred and returned. (DIRECTORY-077 R6).
> - C482 — Proven on Tom's live install with an agent Lys starts. (DIRECTORY-077 R7).
> **Stories:**
> - S257 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want to start an agent from Lys and never sign it in or paste anything, so that it can act at once and only within its grants.
> - S258 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want an agent refused outside its grants to tell me it needs me and ask, and to answer once, for a while, ongoing or no, so that I give only what I mean to.
> - S259 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want the few responsibilities that are mine to stay mine whatever I grant, so that no one can hand away being responsible.

## Purpose

Tom, 2 October (relayed by Waffles at 07:32): you start an AI with Lys and never worry about it again; it never signs in, it can't act outside its grants, and it is secure. On a18cdfc5 an agent reaches 18 of Lys's 173 routes, only by a signed request it has no way to make when Lys starts it, and the grant-token path needs a person to sign in and paste a 24-hour token. Tom: every action a person can take becomes a permission an agent can be granted, "not because I don't want AI to have the power... because I don't want a human to get rid of their responsibility"; only a handful of real responsibilities stay with a person. Agents started outside Lys are out of scope.

## Task

Make every non-public route declare the action it exercises and make the grant model name it. Issue a pass for each run Lys starts and render it with the Lys MCP address into the Claude Code and Codex configs, ending it when the run ends. Take the pass as the agent on every route, judged per call against live grants, and keep the named responsibilities with people. Answer a refusal with who can grant, let the agent ask them, and let them answer once, for a while, ongoing or no. Add one-time and transfer-and-return grants. Prove it on Tom's live install. The responsibilities a person keeps, proposed by Archie for Tom to confirm, held as data so his answer changes data, not code: First-run setup: POST /setup, /setup/open, /setup/administrator, /setup/password. Issuing a root grant: POST /grants/roots, the point where a person takes responsibility. A person's sign-in credentials: POST /directory/people/{id}/account/email and /password, POST /me/account/email and /password. Setting how people sign in: POST /sign-in-providers. The responsible person's answer to a draft: POST /drafts/{id}/approve, /refuse, /correct. Out: agents started outside Lys, a helper command, an assertion exchange, a second MCP server, and the person-issued credential button (parked).

## Requirements

### R1: Every action a person can take in Lys is a permission an agent can be granted

Behavioural. WHEN the service starts, THE SYSTEM SHALL hold, for every route in the table that is not public, the resource kind, the action and the path parameter naming the resource that the route exercises, and the grant model SHALL name each of those actions so a grant can carry it; a route with no declared action SHALL fail the service's start naming the route. Public routes (sign-in, first-run code entry, the issuer's documents, receipts, the runner's dial) are protocol, not actions, and stay as they are.

**Acceptance:**
- Every one of the 155 routes an agent cannot reach on a18cdfc5 declares its resource kind and action, and the grant model answers each action.
- A table row added without a declared action fails start, naming its method and path.
- A signed-in person's session is judged as it is today on every route.

**Files:**
- create: crates/lys-identity-server/src/route_actions.rs
- create: crates/lys-identity-server/tests/route_actions.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity/src/grants/model.rs
- modify: crates/lys-identity/src/grants/schema.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C476 — Every action a person can take in Lys is a permission an agent can be granted. (DIRECTORY-077 R1).

**Stories:**
- S257 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want to start an agent from Lys and never sign it in or paste anything, so that it can act at once and only within its grants.

### R2: Lys issues an agent a pass for one run when it starts it, written into the seat's own config, and ends it when the run ends

Behavioural. WHEN Lys starts, restarts or starts again an agent's launch, THE SYSTEM SHALL issue a pass bound to that agent and that launch record, keep only its digest, and render the Lys MCP address and the pass into the seat's own config: for Claude Code an HTTP MCP server named lys at <service>/api/mcp with the header lys-agent-pass, and for Codex the same address and header in mcp_servers. WHEN the agent is stopped, its session ends or crashes as the runner reports it, or its launch is withdrawn, THE SYSTEM SHALL end that pass, so the next call with it is refused; a restart issues a fresh pass. The pass is never written to a log, an error, an environment variable, a kept template or a profile.

**Acceptance:**
- An agent Lys starts has a config naming the Lys MCP address and a pass, with nothing typed or pasted by a person.
- After a stop, a crash reported by the runner, or a withdrawal, the old pass is refused by name on its next call.
- A restart's config carries a different pass from the run before it, and the earlier pass is refused.
- No log line, error body, environment or template written by the start contains the pass.

**Files:**
- create: crates/lys-identity-server/src/agent_pass.rs
- create: crates/lys-identity-server/src/agent_pass_store.rs
- create: crates/lys-identity-server/tests/agent_pass.rs
- modify: crates/lys-identity-server/src/start.rs
- modify: crates/lys-identity-server/src/restart_api.rs
- modify: crates/lys-identity-server/src/stop_api.rs
- modify: crates/lys-identity-server/src/runtime_api.rs
- modify: crates/lys-identity-server/src/launch_template.rs
- modify: crates/lys-home/src/harness/launch_fields.rs
- modify: crates/lys-home/src/harness/rendering.rs
- modify: crates/lys-home/src/harness/codex/launch.rs
- modify: crates/lys-home/src/harness/claude_code/launch.rs

**Checklist:**
- C477 — Lys issues an agent a pass for one run when it starts it, written into the seat's own config, and ends it when the run ends. (DIRECTORY-077 R2).

**Stories:**
- S257 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want to start an agent from Lys and never sign it in or paste anything, so that it can act at once and only within its grants.

### R3: Every route takes the pass as the agent and judges each call against the agent's live grants, except the few responsibilities a person keeps

Behavioural. WHEN a request carries a live agent pass, THE SYSTEM SHALL take it as that agent on every non-public route, refuse it with a session cookie, an agent signature or an Authorization header beside it, and judge the call against the agent's grants as they stand at that call for the route's declared resource and action; an agent that is suspended or stopped, or whose grant has ended, SHALL be refused on its next call. The responsibilities a person keeps (proposed below for Tom to confirm, held as data) SHALL be refused to every agent whatever it holds, naming the person whose responsibility it is.

**Acceptance:**
- An agent holding the grant for an action makes that change through MCP with its pass and it is recorded as the agent's act.
- The same call without the grant is refused, and nothing is recorded.
- Ending the grant makes the next call refused; suspending the agent does the same.
- Each kept responsibility is refused to an agent that holds every grant, naming the person who keeps it.

**Files:**
- create: crates/lys-identity-server/src/kept_responsibilities.rs
- create: crates/lys-identity-server/tests/agent_pass_routes.rs
- modify: crates/lys-identity-server/src/signed_first.rs
- modify: crates/lys-identity-server/src/agent_signature.rs
- modify: crates/lys-identity-server/src/mcp_endpoint.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/src/openapi.rs

**Checklist:**
- C478 — Every route takes the pass as the agent and judges each call against the agent's live grants, except the few responsibilities a person keeps. (DIRECTORY-077 R3).

**Stories:**
- S257 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want to start an agent from Lys and never sign it in or paste anything, so that it can act at once and only within its grants.
- S259 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want the few responsibilities that are mine to stay mine whatever I grant, so that no one can hand away being responsible.

### R4: A refusal names who can grant the action, walking up the chain

Behavioural. WHEN an agent's call is refused because it holds no grant for the action, THE SYSTEM SHALL answer, beside the refusal, the nearest identity who holds that action on that resource and may pass it on to an agent, walking up the agent's reports-to chain first and then the grant's lineage to its root person, and say in plain words who to ask (for example, ask your lead, or this needs Tom). A refused call is not turned into a draft; drafts stay for an agent asked to propose a change on someone's behalf.

**Acceptance:**
- An agent refused for an action its lead can pass on is told to ask its lead, by name.
- An agent refused for an action only the root person can give is told it needs that person, by name.
- No draft is created by a refusal.

**Files:**
- create: crates/lys-identity-server/src/who_can_grant.rs
- create: crates/lys-identity-server/tests/who_can_grant.rs
- modify: crates/lys-identity-server/src/grants_refusals.rs
- modify: crates/lys-identity-server/src/grants_reach.rs
- modify: crates/lys-identity-server/src/mcp_endpoint.rs

**Checklist:**
- C479 — A refusal names who can grant the action, walking up the chain. (DIRECTORY-077 R4).

**Stories:**
- S258 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want an agent refused outside its grants to tell me it needs me and ask, and to answer once, for a while, ongoing or no, so that I give only what I mean to.

### R5: An agent asks that person, and they answer once, for a while, ongoing or no

Behavioural. WHEN an agent with a live pass asks who may act on a resource or sends an access request naming an action and a resource, THE SYSTEM SHALL accept it as the agent, address the request to the identity the refusal named, and let that identity answer once (a one-time grant), for a while (a grant with an end time it chooses), ongoing (a grant until revoked) or no; the answer is recorded and the agent's next call is judged against it.

**Acceptance:**
- An agent sends a request after a refusal, and the named person sees it addressed to them.
- Each of the four answers produces the grant it names, or none, and the agent's next call is judged against it.
- A request for a kept responsibility is refused, naming the person who keeps it.

**Files:**
- create: crates/lys-identity-server/tests/requests_from_agents.rs
- modify: crates/lys-identity-server/src/requests_api.rs
- modify: crates/lys-identity-server/src/requests_decide.rs
- modify: crates/lys-identity-server/src/requests_state.rs
- modify: crates/lys-identity-server/src/requests_store.rs
- modify: crates/lys-identity-server/src/grants_reach.rs

**Checklist:**
- C480 — An agent asks that person, and they answer once, for a while, ongoing or no. (DIRECTORY-077 R5).

**Stories:**
- S258 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want an agent refused outside its grants to tell me it needs me and ask, and to answer once, for a while, ongoing or no, so that I give only what I mean to.

### R6: Grants can be one-time, and can be transferred and returned

Behavioural. THE SYSTEM SHALL let a grant be one-time, spent by the first exercise the check admits so a second exercise is refused, and SHALL let a holder transfer a grant to another identity, losing its use themselves until it is returned or the transfer's window ends, when it comes back to them; both keep the existing pass-on, sharing, window and cascading revoke rules, and a stored grant from before this change reads unchanged.

**Acceptance:**
- A one-time grant admits exactly one exercise; two calls at once admit one and refuse the other.
- A transferred grant is usable by the recipient and refused to the holder until it is returned or its window ends, then the reverse.
- Revoking the source of a one-time or transferred grant ends it.
- A grant log from a18cdfc5 opens and answers every grant as it did.

**Files:**
- create: crates/lys-identity/tests/grant_kinds.rs
- modify: crates/lys-identity/src/grants/types.rs
- modify: crates/lys-identity/src/grants/usage.rs
- modify: crates/lys-identity/src/grants/revocation.rs
- modify: crates/lys-identity/src/grants/commit.rs
- modify: crates/lys-identity/src/grants/events.rs
- modify: crates/lys-identity/src/grants/codec.rs
- modify: crates/lys-identity/src/grants/admission.rs

**Checklist:**
- C481 — Grants can be one-time, and can be transferred and returned. (DIRECTORY-077 R6).

**Stories:**
- S258 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want an agent refused outside its grants to tell me it needs me and ask, and to answer once, for a while, ongoing or no, so that I give only what I mean to.

### R7: Proven on Tom's live install with an agent Lys starts

Behavioural. WHEN an agent is started by Lys on Tom's live install with nothing pasted, THE SYSTEM SHALL let it make one real change inside its grants, refuse it outside them with the name of who can grant, carry its request to that person, and let a one-time approval through exactly once, and the proof SHALL record each step's call and answer.

**Acceptance:**
- The proof records the four steps on the live install, each with its call and answer and no secret.
- The second attempt after the one-time approval is refused.

**Files:**
- create: docs/design/directory/DIRECTORY-077-PROOF.md

**Checklist:**
- C482 — Proven on Tom's live install with an agent Lys starts. (DIRECTORY-077 R7).

**Stories:**
- S257 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want to start an agent from Lys and never sign it in or paste anything, so that it can act at once and only within its grants.
- S258 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want an agent refused outside its grants to tell me it needs me and ask, and to answer once, for a while, ongoing or no, so that I give only what I mean to.

## Boundaries

- SHALL NOT poll, sleep, add a timer, timeout, deadline, watchdog or background loop; a pass ends on the event that ends its run.
- SHALL NOT add unsafe, an ignored test, an allow attribute, an underscore rename or a discarded result.
- SHALL NOT change a stored shape without a migration tested from an a18cdfc5 install, and SHALL NOT edit an installed configuration by hand.
- SHALL NOT write a pass, a token or a key into a log, an error, an environment variable, a template, a profile, a post or a test's output.
- SHALL NOT add a second MCP server, a helper command or an assertion exchange.
- SHALL NOT turn a refused call into a draft.
- SHALL NOT make Lys depend on Cambium or name a harness detail in code; harness details are data.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Built by the assigned GPT seats in 30-minute bounded pieces on their own branches: red first, then green, with fmt, Clippy pedantic in both configurations, nextest on the touched crates, ast-grep and the 500-line file limit.
- One gate after all the code is written; a red is fixed and the fixed head gates again at once. Archie stacks, gates, lands and installs.
- R7 is shown on Tom's live install after the install, and its record is committed.
