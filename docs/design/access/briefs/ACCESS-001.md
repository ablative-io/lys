---
type: brief
id: ACCESS-001
cluster: access
title: A grant carries its mode; a held act is a draft the product executes after approval
---

# ACCESS-001: A grant carries its mode; a held act is a draft the product executes after approval

> **Cluster:** access
> **Depends on:** DIRECTORY-081
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> - ADR-126 — Applications have connector identities; people register them with explicit permission — An application connector is its own third identity and grant-holder kind. Apps do not act as agents and do not use service accounts, which are accounts people use to service things. Only a signed-in person with an explicit ordinary register_app grant registers an app. The super administrator may grant that permission to a person; an agent, connector or service account cannot register, even if presented with such a grant. Approval creates a connector identity and binds the app to it. No app authority is implicit in approval, binding or ownership of a kind prefix: every permission is an explicit grant with a chain tracing to the super administrator. Own-kind checks, schema acts, batch and which use that connector identity and the ordinary grant engine. Extend IdentityId, grant admission and the permission-store holder, plus event and snapshot representation with compatibility tests. Preserve historical person/agent bytes and signatures; version a representation when necessary, never rewrite history or coerce identities. This supersedes DIRECTORY-048 wording allowing a service account or connector holding register_app to register, and forbids a separate registrar credential as substitute authority.
> - ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.
> - ADR-136 — Permissions across the stack: Lys is the one authority; products decide hot routes from a signed pass and ask Lys for deliberate ones; a grant carries its mode — Lys holds every identity and grant; a product permission model is data in its app schema (ADR-116). Every product route is classified once as hot (decided offline from the pass) or deliberate (asked of Lys live, never falling back). The pass is the access token with a rights claim for its audience, short-lived, signed with the key Lys publishes. A grant carries one mode: outright, by_draft or by_two; a held act is a Lys draft the product executes after approval by reading Lys. Reach is by placement, never a pattern; a placement may be restricted. Roles are named bundles in the product schema; a widening by an app waits for the administrator. A machine is the fifth identity. One client library, lys-pass, with conformance fixtures every product gate runs. Availability is stated on each product screen; every deliberate act is receipted in Lys.
> **Checklist:**
> - C601 — A grant carries one mode: outright, by_draft or by_two; the codec, events and snapshots version for it and old bytes stay readable (ACCESS-001 R1).
> - C602 — An app schema marks each action hot or deliberate; Lys refuses a by_draft or by_two grant on a hot action by name (ACCESS-001 R2).
> - C603 — A product records a held act as a draft naming the grant, the target and the request digest; one approval executes by_draft, two distinct approvers execute by_two; the product reads approved drafts of its kinds and records done or refused-on-execution (ACCESS-001 R3).
> - C604 — The Access screens show a grant's mode, the drafts held for a product, and who approved each (ACCESS-001 R4).
> **Stories:**
> - S301 (Tom, owner and administrator) — As the owner, I want to say in one grant that Archie may stop seats by draft, so that an agent acts within its grant and I approve the rare act without a second system.
> - S306 (Archie, an agent) — As an agent, I want a refusal to name the grant I need and where to ask, so that I ask the right person once.
> - S307 (Archie, an agent) — As an agent, I want a by_draft act to be held and done after approval without my doing it twice, so that I never work around a draft.

## Purpose

Tom, 6 October 2026, to Hermes and relayed to Waffles: an agent may do "whatever your grant allows without it having to be a draft first", with the grant saying what needs a draft or two approvals. Today a grant (lys-identity grants/types.rs:300-324) carries no such word, drafts (drafts_api.rs) are made by hand, and nothing marks which actions could ever be held. This brief puts the mode on the grant and makes the draft the product's hand-off, with Lys never calling the product (ADR-116).

## Task

Add mode to the grant and to the schema's actions; refuse a held mode on a hot action; make a draft record a product's held act; let the product read approved drafts and record their execution; show it all on the Access screens.

## Requirements

### R1: A grant carries one mode

Behavioural. WHEN a grant is issued, THE SYSTEM SHALL carry one of outright, by_draft or by_two on it, defaulting to outright for every grant written before this brief; the grant body codec, the grant event envelope and the directory snapshot SHALL gain a version for it with the earlier versions still read and pinned by fixture; the Grant type, its admission and its refusal records SHALL carry the mode; /grants/which and /grants/check/batch SHALL answer the mode beside each right. Refusal texts SHALL say what is true about versions (no stale "1 or 2").

**Acceptance:**
- An old grant fixture reads as outright and its signature still verifies.
- A by_two grant issued, read back, snapshotted and reopened keeps by_two.
- check/batch on a by_draft right answers allowed=false, mode=by_draft, grant id, so a product holds rather than refuses.

**Files:**
- create: crates/lys-identity/tests/grant_mode_compatibility.rs
- modify: crates/lys-identity/src/grants/types.rs
- modify: crates/lys-identity/src/grants/codec.rs
- modify: crates/lys-identity/src/grants/events.rs
- modify: crates/lys-identity/src/grants/admission.rs
- modify: crates/lys-identity/src/grants/refusal_codec.rs
- modify: crates/lys-identity/src/state_value.rs
- modify: crates/lys-identity-server/src/grants/handlers.rs
- modify: crates/lys-identity-server/src/grants_batch.rs

**Checklist:**
- C601 — A grant carries one mode: outright, by_draft or by_two; the codec, events and snapshots version for it and old bytes stay readable (ACCESS-001 R1).

**Stories:**
- S301 (Tom, owner and administrator) — As the owner, I want to say in one grant that Archie may stop seats by draft, so that an agent acts within its grant and I approve the rare act without a second system.

### R2: A schema action is hot or deliberate, and a held mode is refused on a hot action

Behavioural. WHEN an app schema version names an action, THE SYSTEM SHALL take a class for it, hot or deliberate, defaulting every existing action to deliberate; WHEN a grant with mode by_draft or by_two names an action whose class is hot, THE SYSTEM SHALL refuse it as grant_mode_on_hot_action naming the app, action and class, and write nothing; a schema change that moves an action from deliberate to hot while a standing by_draft or by_two grant names it SHALL be refused as schema_change_strands_grants as a stranding is today. An app schema names, per kind, the actions an agent may hold under `agents`, each one the kind declares and named once; a kind that names none gives an agent none of its actions, so an agent holds an app's act only where its schema opts in, act by act, and Lys's own kinds keep the shipped rule.

**Acceptance:**
- PUT /apps/{app}/schema with an action class round-trips at every version.
- Issuing a by_draft grant on a hot action is refused by name with nothing written.
- Moving an action to hot with a standing by_draft grant is refused naming the grant count.
- A kind naming `agents: ["read"]` lets a grant passed to an agent carry read and refuses post as WithheldFromAgents; a kind naming no `agents` refuses every act to an agent; an `agents` action the kind does not declare, or named twice, is refused at its pointer.

**Files:**
- create: crates/lys-identity/tests/schema_agents.rs
- modify: crates/lys-identity-server/src/apps_schema_api.rs
- modify: crates/lys-identity-server/src/apps_schema.rs
- modify: crates/lys-identity-server/src/grants/handlers.rs
- modify: crates/lys-identity-server/src/openapi_typed.rs
- modify: crates/lys-identity-server/tests/apps_schema.rs
- modify: crates/lys-identity/src/grants/schema.rs
- modify: crates/lys-identity/src/grants/schema_class.rs
- modify: crates/lys-identity/src/grants/model.rs
- modify: crates/lys-identity/src/grants/admission.rs

**Checklist:**
- C602 — An app schema marks each action hot or deliberate; Lys refuses a by_draft or by_two grant on a hot action by name (ACCESS-001 R2).

**Stories:**
- S301 (Tom, owner and administrator) — As the owner, I want to say in one grant that Archie may stop seats by draft, so that an agent acts within its grant and I approve the rare act without a second system.

### R3: A product holds an act as a draft and executes it after approval

Behavioural. WHEN a product presents a pass (or its connector) and a by_draft or by_two right for a target {kind, id, action} and the digest and words of the request, THE SYSTEM SHALL record a draft naming the grant, the holder, the responsible person, the target and the digest, and answer the draft id; approval by one reviewer who may grant that action (walking up the chain as ADR-135 says) SHALL make a by_draft draft approved, and by two distinct such reviewers a by_two draft; the holder and the responsible person SHALL NOT be the approvers of their own draft; GET /drafts?app={app}&state=approved SHALL answer the approved drafts of that app's kinds to that app's connector and to nobody else; POST /drafts/{id}/executed with the product's receipt digest SHALL close it, and POST /drafts/{id}/refused-on-execution with the product's refusal SHALL close it as such; a draft left approved and unexecuted SHALL stay visible on the dashboard's drafts count. Refused calls are not drafts (ADR-135): only a right whose mode is held makes one.

**Acceptance:**
- A by_two draft with one approval is not answered as approved; with two distinct approvers it is; the holder approving is refused by name.
- Another app's connector asking for approved drafts gets none and is refused by name.
- Executed and refused-on-execution close the draft and the dashboard count falls.

**Files:**
- create: crates/lys-identity-server/src/drafts_products.rs
- create: crates/lys-identity-server/tests/drafts_products.rs
- modify: crates/lys-identity-server/src/drafts_api.rs
- modify: crates/lys-identity/src/draft_event.rs
- modify: crates/lys-identity-server/src/openapi_typed.rs
- modify: crates/lys-identity-server/src/openapi_refusals.rs

**Checklist:**
- C603 — A product records a held act as a draft naming the grant, the target and the request digest; one approval executes by_draft, two distinct approvers execute by_two; the product reads approved drafts of its kinds and records done or refused-on-execution (ACCESS-001 R3).

**Stories:**
- S307 (Archie, an agent) — As an agent, I want a by_draft act to be held and done after approval without my doing it twice, so that I never work around a draft.
- S306 (Archie, an agent) — As an agent, I want a refusal to name the grant I need and where to ask, so that I ask the right person once.

### R4: The Access screens show mode and held drafts

Behavioural. WHEN a grant is shown anywhere on the Access screens, THE SYSTEM SHALL show its mode in words (outright, by draft, by two approvals) and let an issuer choose the mode when issuing; the drafts screen SHALL list drafts held for each product with the target, the holder, the responsible person, who approved and when, and executed or refused-on-execution; nothing is derived on the screen.

**Acceptance:**
- Issuing a grant with "by draft" shows it on the who and reach views with that word.
- A product's held draft appears with its approvals and closes when executed.
- Screens walked with pictures on the install.

**Files:**
- modify: surface/identity/src/features/access/Issue.tsx
- modify: surface/identity/src/features/access/Drafts.tsx
- modify: surface/identity/tests/access-drafts.test.tsx

**Checklist:**
- C604 — The Access screens show a grant's mode, the drafts held for a product, and who approved each (ACCESS-001 R4).

**Stories:**
- S301 (Tom, owner and administrator) — As the owner, I want to say in one grant that Archie may stop seats by draft, so that an agent acts within its grant and I approve the rare act without a second system.

## Boundaries

- SHALL NOT call a product, wait on one, or read its store: execution is the product reading Lys (ADR-116).
- SHALL NOT turn a refusal into a draft; only a held-mode right makes one (ADR-135).
- SHALL NOT rewrite or drop any grant, event or snapshot written before this brief; versions only.

## Verification

- This handwritten brief passes the design gate (scripts/design/gate.sh), judged by its parsed failures, never by its exit code; the rendered markdown matches what render-cluster.py writes.
- Written whole, read whole by the lead, then one battery on the Mac (every leg green) and the whole gate on Dean through aion at the same sha; then one install and a browser walk with pictures of every screen the brief touches.
- Every handback has four headings (BLOCKED ON, CHANGED, FOUND, NOT CONFIRMED) and the hospital sentence; no test proves less than before; file-length gate 500 code lines; clippy --workspace --all-targets -- -D warnings; only cargo nextest.

## Amendments

### Amendment 1: the schema's agents field

- **Date:** 2026-10-09
- **By:** Waffles (ruling relayed by Gaia)

An app's schema marks, per kind and per act, which acts an agent may hold, under `agents`; the default stays none, so agent_may_hold's rule (no act of an app's kind is an agent's) is the default an app opts out of act by act, never a silent widening. The shared membership vector world's schema marks read on the channel kind and nothing else, and the vectors pass as written. Cambium's own schema marks its acts on Monday 12 October 2026 through Artemis, in the permissions story.
