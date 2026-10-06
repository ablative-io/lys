---
type: brief
id: ACCESS-004
cluster: access
title: Roles are named bundles in the app schema; a placement may be restricted
---

# ACCESS-004: Roles are named bundles in the app schema; a placement may be restricted

> **Cluster:** access
> **Depends on:** ACCESS-001
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> - ADR-126 — Applications have connector identities; people register them with explicit permission — An application connector is its own third identity and grant-holder kind. Apps do not act as agents and do not use service accounts, which are accounts people use to service things. Only a signed-in person with an explicit ordinary register_app grant registers an app. The super administrator may grant that permission to a person; an agent, connector or service account cannot register, even if presented with such a grant. Approval creates a connector identity and binds the app to it. No app authority is implicit in approval, binding or ownership of a kind prefix: every permission is an explicit grant with a chain tracing to the super administrator. Own-kind checks, schema acts, batch and which use that connector identity and the ordinary grant engine. Extend IdentityId, grant admission and the permission-store holder, plus event and snapshot representation with compatibility tests. Preserve historical person/agent bytes and signatures; version a representation when necessary, never rewrite history or coerce identities. This supersedes DIRECTORY-048 wording allowing a service account or connector holding register_app to register, and forbids a separate registrar credential as substitute authority.
> - ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.
> - ADR-136 — Permissions across the stack: Lys is the one authority; products decide hot routes from a signed pass and ask Lys for deliberate ones; a grant carries its mode — Lys holds every identity and grant; a product permission model is data in its app schema (ADR-116). Every product route is classified once as hot (decided offline from the pass) or deliberate (asked of Lys live, never falling back). The pass is the access token with a rights claim for its audience, short-lived, signed with the key Lys publishes. A grant carries one mode: outright, by_draft or by_two; a held act is a Lys draft the product executes after approval by reading Lys. Reach is by placement, never a pattern; a placement may be restricted. Roles are named bundles in the product schema; a widening by an app waits for the administrator. A machine is the fifth identity. One client library, lys-pass, with conformance fixtures every product gate runs. Availability is stated on each product screen; every deliberate act is receipted in Lys.
> **Checklist:**
> - C611 — An app schema names roles as action bundles; a grant may name a role and the pass expands it; a change that widens a role by an app waits for the administrator (ACCESS-004 R1).
> - C612 — A placement may be restricted: the parent's relations do not flow to it and only an explicit grant reaches it (ACCESS-004 R2).
> **Stories:**
> - S302 (Tom, owner and administrator) — As the owner, I want one screen that shows who can do what in every product, so that permissions are not a nightmare to operate.
> - S303 (Tom, owner and administrator) — As the owner, I want a product deploy that widens a role to wait for me, so that nobody holds more because a release shipped.
> - S308 (A clinician, a person in a hospital workspace) — As a clinician, I want a private place reached only by those granted it, so that a new colleague does not see every ward's channel on their first day.

## Purpose

People think in roles (administrator, member, observer); code checks actions. Without roles in the schema, every grant is thirty actions assembled by hand and the Access screen is unreadable. Without a restricted placement, a parent's relations reach every child and a new person sees every channel (Cambium today).

## Task

Let an app schema name roles as action bundles; let a grant name a role; expand roles at pass issue; make a role widening by an app wait for the administrator; add restricted to placements.

## Requirements

### R1: Roles in the schema and in grants

Behavioural. WHEN an app schema version names roles, THE SYSTEM SHALL take each as a name and a set of that app's actions on one kind; a grant MAY name a role instead of actions and SHALL be judged as its actions at the version current when judged; WHEN an app (not the administrator) submits a schema version that widens a role, the change SHALL wait for the administrator on the Apps screen as app-made changes do today, and the screen SHALL show the widening by name; a role narrowed SHALL be refused while standing grants name it unless the narrowing strands nothing.

**Acceptance:**
- A grant naming cambium.workspace.administrator expands in the pass to the role's actions.
- An app widening a role waits; the administrator sees "adds seat_retire to administrator" and approves or refuses.

**Files:**
- modify: crates/lys-identity-server/src/apps_schema_api.rs
- modify: crates/lys-identity-server/src/apps_schema.rs
- modify: crates/lys-identity/src/grants/types.rs
- modify: crates/lys-identity-server/src/grants_reach.rs
- modify: surface/identity/src/features/apps/Apps.tsx
- modify: surface/identity/src/features/access/Issue.tsx

**Checklist:**
- C611 — An app schema names roles as action bundles; a grant may name a role and the pass expands it; a change that widens a role by an app waits for the administrator (ACCESS-004 R1).

**Stories:**
- S302 (Tom, owner and administrator) — As the owner, I want one screen that shows who can do what in every product, so that permissions are not a nightmare to operate.
- S303 (Tom, owner and administrator) — As the owner, I want a product deploy that widens a role to wait for me, so that nobody holds more because a release shipped.

### R2: A restricted placement

Behavioural. WHEN a placement is recorded with restricted true, THE SYSTEM SHALL NOT flow the parent's relations to that child; only a grant naming the child reaches it; the reach views and the pass SHALL reflect it; an unrestricted placement is as today.

**Acceptance:**
- A member of the workspace does not reach a restricted place; a grant on the place does.
- The reach view shows the restricted place apart.

**Files:**
- modify: crates/lys-identity-server/src/apps_schema_api.rs
- modify: crates/lys-identity-server/src/grants_reach.rs
- modify: crates/lys-identity-server/tests/apps_schema.rs
- modify: surface/identity/src/features/access/Reach.tsx

**Checklist:**
- C612 — A placement may be restricted: the parent's relations do not flow to it and only an explicit grant reaches it (ACCESS-004 R2).

**Stories:**
- S308 (A clinician, a person in a hospital workspace) — As a clinician, I want a private place reached only by those granted it, so that a new colleague does not see every ward's channel on their first day.

## Boundaries

- SHALL NOT let a role widening by an app take effect before the administrator approves it.
- SHALL NOT add a wildcard or pattern resource.

## Verification

- This handwritten brief passes the design gate (scripts/design/gate.sh), judged by its parsed failures, never by its exit code; the rendered markdown matches what render-cluster.py writes.
- Written whole, read whole by the lead, then one battery on the Mac (every leg green) and the whole gate on Dean through aion at the same sha; then one install and a browser walk with pictures of every screen the brief touches.
- Every handback has four headings (BLOCKED ON, CHANGED, FOUND, NOT CONFIRMED) and the hospital sentence; no test proves less than before; file-length gate 500 code lines; clippy --workspace --all-targets -- -D warnings; only cargo nextest.
