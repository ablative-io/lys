---
type: brief
id: ACCESS-005
cluster: access
title: A machine is the fifth identity and a grant holder
---

# ACCESS-005: A machine is the fifth identity and a grant holder

> **Cluster:** access
> **Depends on:** ACCESS-001
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> - ADR-126 — Applications have connector identities; people register them with explicit permission — An application connector is its own third identity and grant-holder kind. Apps do not act as agents and do not use service accounts, which are accounts people use to service things. Only a signed-in person with an explicit ordinary register_app grant registers an app. The super administrator may grant that permission to a person; an agent, connector or service account cannot register, even if presented with such a grant. Approval creates a connector identity and binds the app to it. No app authority is implicit in approval, binding or ownership of a kind prefix: every permission is an explicit grant with a chain tracing to the super administrator. Own-kind checks, schema acts, batch and which use that connector identity and the ordinary grant engine. Extend IdentityId, grant admission and the permission-store holder, plus event and snapshot representation with compatibility tests. Preserve historical person/agent bytes and signatures; version a representation when necessary, never rewrite history or coerce identities. This supersedes DIRECTORY-048 wording allowing a service account or connector holding register_app to register, and forbids a separate registrar credential as substitute authority.
> - ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.
> - ADR-136 — Permissions across the stack: Lys is the one authority; products decide hot routes from a signed pass and ask Lys for deliberate ones; a grant carries its mode — Lys holds every identity and grant; a product permission model is data in its app schema (ADR-116). Every product route is classified once as hot (decided offline from the pass) or deliberate (asked of Lys live, never falling back). The pass is the access token with a rights claim for its audience, short-lived, signed with the key Lys publishes. A grant carries one mode: outright, by_draft or by_two; a held act is a Lys draft the product executes after approval by reading Lys. Reach is by placement, never a pattern; a placement may be restricted. Roles are named bundles in the product schema; a widening by an app waits for the administrator. A machine is the fifth identity. One client library, lys-pass, with conformance fixtures every product gate runs. Availability is stated on each product screen; every deliberate act is receipted in Lys.
> **Checklist:**
> - C613 — A machine is the fifth identity and grant holder, made at join, answering to the administrator who issued its code; old bytes stay readable (ACCESS-005 R1).
> - C614 — The Network screen shows machines as holders and their grants; a machine never holds a kept responsibility (ACCESS-005 R2).
> **Stories:**
> - S305 (Hermes, a product lead (liminal)) — As a product lead, I want a device to hold export and import grants on a link, so that federation has a holder.

## Purpose

A computer with a runner joins with a one-use code and its own key (network_join.rs) but is not an identity and holds no grant; a liminal node or a federation link has nowhere to hold export and import rights (Hermes, 6 October). ADR-126 settled how a kind is added: a new code, versions, old bytes untouched.

## Task

Add IdentityId::Machine (code 5), made at join from the recorded key, answering to the administrator who issued the code; grants with a machine holder; the Network screen shows machines and their grants.

## Requirements

### R1: The machine identity

Behavioural. WHEN POST /runner/join admits a computer, THE SYSTEM SHALL make a machine identity (word machine- and hex from the secure random source, made once and stored) with its public key, its responsible person the administrator who asked for the code, in the join's own batch; the codec, state value, grant events (a version), SpiceDB definition and the three readers that take an unknown prefix for another kind SHALL take the machine by name; a machine SHALL be refused as an issuer and as an approver and SHALL never hold a kept responsibility.

**Acceptance:**
- Join; the machine appears as an identity with its key and responsible person; an old join record still reads.
- A grant to the machine on a liminal link kind is issued, read back and in its pass.

**Files:**
- create: crates/lys-identity/tests/machine_compatibility.rs
- modify: crates/lys-identity/src/id.rs
- modify: crates/lys-identity/src/grants/codec.rs
- modify: crates/lys-identity/src/grants/types.rs
- modify: crates/lys-identity/src/grants/events.rs
- modify: crates/lys-identity/src/grants/admission.rs
- modify: crates/lys-identity/src/state_value.rs
- modify: crates/lys-identity/src/grants/permission.rs
- modify: crates/lys-identity-server/src/network_join.rs
- modify: crates/lys-identity-server/src/spicedb.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C613 — A machine is the fifth identity and grant holder, made at join, answering to the administrator who issued its code; old bytes stay readable (ACCESS-005 R1).

**Stories:**
- S305 (Hermes, a product lead (liminal)) — As a product lead, I want a device to hold export and import grants on a link, so that federation has a holder.

### R2: Machines on the Network screen

Behavioural. THE SYSTEM SHALL show each machine on the Network screen with its key id, responsible person, the grants it holds and the links they are on, and let an administrator issue a grant to it from there.

**Acceptance:**
- Walked with pictures on the install: a machine with one link grant.

**Files:**
- modify: surface/identity/src/features/network/Network.tsx
- modify: surface/identity/tests/network-machines.test.tsx

**Checklist:**
- C614 — The Network screen shows machines as holders and their grants; a machine never holds a kept responsibility (ACCESS-005 R2).

**Stories:**
- S305 (Hermes, a product lead (liminal)) — As a product lead, I want a device to hold export and import grants on a link, so that federation has a holder.

## Boundaries

- SHALL NOT coerce an existing runner record into a machine without the join's key.
- SHALL NOT let a machine issue, approve, or hold a kept responsibility.

## Verification

- This handwritten brief passes the design gate (scripts/design/gate.sh), judged by its parsed failures, never by its exit code; the rendered markdown matches what render-cluster.py writes.
- Written whole, read whole by the lead, then one battery on the Mac (every leg green) and the whole gate on Dean through aion at the same sha; then one install and a browser walk with pictures of every screen the brief touches.
- Every handback has four headings (BLOCKED ON, CHANGED, FOUND, NOT CONFIRMED) and the hospital sentence; no test proves less than before; file-length gate 500 code lines; clippy --workspace --all-targets -- -D warnings; only cargo nextest.
