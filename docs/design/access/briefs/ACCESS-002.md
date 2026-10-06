---
type: brief
id: ACCESS-002
cluster: access
title: The pass: the access token carries the holder's rights for its audience, signed with the key Lys publishes
---

# ACCESS-002: The pass: the access token carries the holder's rights for its audience, signed with the key Lys publishes

> **Cluster:** access
> **Depends on:** ACCESS-001, DIRECTORY-081
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> - ADR-126 — Applications have connector identities; people register them with explicit permission — An application connector is its own third identity and grant-holder kind. Apps do not act as agents and do not use service accounts, which are accounts people use to service things. Only a signed-in person with an explicit ordinary register_app grant registers an app. The super administrator may grant that permission to a person; an agent, connector or service account cannot register, even if presented with such a grant. Approval creates a connector identity and binds the app to it. No app authority is implicit in approval, binding or ownership of a kind prefix: every permission is an explicit grant with a chain tracing to the super administrator. Own-kind checks, schema acts, batch and which use that connector identity and the ordinary grant engine. Extend IdentityId, grant admission and the permission-store holder, plus event and snapshot representation with compatibility tests. Preserve historical person/agent bytes and signatures; version a representation when necessary, never rewrite history or coerce identities. This supersedes DIRECTORY-048 wording allowing a service account or connector holding register_app to register, and forbids a separate registrar credential as substitute authority.
> - ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.
> - ADR-136 — Permissions across the stack: Lys is the one authority; products decide hot routes from a signed pass and ask Lys for deliberate ones; a grant carries its mode — Lys holds every identity and grant; a product permission model is data in its app schema (ADR-116). Every product route is classified once as hot (decided offline from the pass) or deliberate (asked of Lys live, never falling back). The pass is the access token with a rights claim for its audience, short-lived, signed with the key Lys publishes. A grant carries one mode: outright, by_draft or by_two; a held act is a Lys draft the product executes after approval by reading Lys. Reach is by placement, never a pattern; a placement may be restricted. Roles are named bundles in the product schema; a widening by an app waits for the administrator. A machine is the fifth identity. One client library, lys-pass, with conformance fixtures every product gate runs. Availability is stated on each product screen; every deliberate act is receipted in Lys.
> **Checklist:**
> - C605 — The access token for an app carries a rights claim: holder, kind, responsible person, and for that audience only, each right's resource, actions, mode and grant id, roles expanded (ACCESS-002 R1).
> - C606 — The pass lifetime is a setting with a stated default; refresh issues a new pass from live grants; a revoked grant is absent from the next pass (ACCESS-002 R2).
> - C607 — The key that signs passes is published with its id and rotates without invalidating passes already issued within their lifetime (ACCESS-002 R3).
> **Stories:**
> - S304 (Hermes, a product lead (liminal)) — As a product lead, I want one library that verifies a pass, reaches through parents and refuses in one shape, so that I do not implement permissions a fourth way.
> - S306 (Archie, an agent) — As an agent, I want a refusal to name the grant I need and where to ask, so that I ask the right person once.

## Purpose

Lys answers questions about grants but issues nothing a product can verify by itself (provider/endpoints.rs:480-500 carries no rights). A product on a hot path (a bus message, a chat read) cannot ask Lys per request. The pass is the access token with a rights claim for that audience, short-lived, signed with Lys's published key, so a product decides hot routes offline and asks Lys only for deliberate ones (design page D2, D3).

## Task

Put a rights claim in the access token for its audience, expanded from live grants and roles; make its lifetime a setting with refresh; publish and rotate the signing key with a key id.

## Requirements

### R1: The access token carries the rights for its audience

Behavioural. WHEN the token endpoint issues an access token for client (audience) app A, THE SYSTEM SHALL carry a rights claim: the holder id and kind, the responsible person (for a non-person holder), and for app A's kinds only, one entry per effective right: resource kind, resource id, the actions, the mode and the grant id, with roles (ACCESS-004) expanded to their actions and placement reach already applied so a product never recomputes reach it cannot see; rights of other apps SHALL be absent; a token for app A SHALL be refused by a product as audience B's (lys-pass). The claim SHALL be bounded: WHEN it would exceed the configured size, THE SYSTEM SHALL answer the token with rights_truncated naming the app, so the product asks Lys instead of guessing.

**Acceptance:**
- A person with grants on two apps gets a token for A naming A's rights only.
- A connector's token names its responsible person.
- A right held on a parent appears for the placed child; a restricted child does not.

**Files:**
- create: crates/lys-identity-server/src/provider/rights_claim.rs
- create: crates/lys-identity-server/tests/provider_rights.rs
- modify: crates/lys-identity-server/src/provider/endpoints.rs
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/grants_reach.rs

**Checklist:**
- C605 — The access token for an app carries a rights claim: holder, kind, responsible person, and for that audience only, each right's resource, actions, mode and grant id, roles expanded (ACCESS-002 R1).

**Stories:**
- S304 (Hermes, a product lead (liminal)) — As a product lead, I want one library that verifies a pass, reaches through parents and refuses in one shape, so that I do not implement permissions a fourth way.

### R2: The pass lifetime is a setting and refresh re-reads live grants

Behavioural. THE SYSTEM SHALL take identity.pass_lifetime as an install setting with a default of ten minutes (Tom to confirm, design page section 7), recorded in build.json; WHEN a refresh token is exchanged, THE SYSTEM SHALL issue a new access token from the grants live at that moment, so a revoked grant is absent from it; WHEN the holder's session has ended or the holder is retired, the refresh SHALL be refused by name.

**Acceptance:**
- Revoke a grant, refresh: the right is gone.
- Retire the agent, refresh: refused by name.
- The default and a chosen lifetime both read back from build.json.

**Files:**
- modify: crates/lys/src/identity/install/settings.rs
- modify: crates/lys-identity-server/src/provider/endpoints.rs
- modify: crates/lys/tests/identity_install/product.rs

**Checklist:**
- C606 — The pass lifetime is a setting with a stated default; refresh issues a new pass from live grants; a revoked grant is absent from the next pass (ACCESS-002 R2).

**Stories:**
- S306 (Archie, an agent) — As an agent, I want a refusal to name the grant I need and where to ask, so that I ask the right person once.

### R3: The signing key is published with an id and rotates safely

Behavioural. THE SYSTEM SHALL publish the pass-signing public key at the provider's keys route with a key id in every token header; WHEN the administrator rotates the key, THE SYSTEM SHALL publish both keys until every token signed by the old one has expired (the lifetime) and no longer; a product SHALL be able to verify any unexpired token with the published set alone.

**Acceptance:**
- Rotate; a token from before rotation verifies until its expiry and not after; the old key leaves the set after one lifetime.

**Files:**
- modify: crates/lys-identity-server/src/provider/keys.rs
- modify: crates/lys-identity-server/tests/provider.rs

**Checklist:**
- C607 — The key that signs passes is published with its id and rotates without invalidating passes already issued within their lifetime (ACCESS-002 R3).

**Stories:**
- S304 (Hermes, a product lead (liminal)) — As a product lead, I want one library that verifies a pass, reaches through parents and refuses in one shape, so that I do not implement permissions a fourth way.

## Boundaries

- SHALL NOT put rights of another audience in a token.
- SHALL NOT let a product derive reach from a token: reach is applied at issue.
- SHALL NOT issue a pass without an expiry.

## Verification

- This handwritten brief passes the design gate (scripts/design/gate.sh), judged by its parsed failures, never by its exit code; the rendered markdown matches what render-cluster.py writes.
- Written whole, read whole by the lead, then one battery on the Mac (every leg green) and the whole gate on Dean through aion at the same sha; then one install and a browser walk with pictures of every screen the brief touches.
- Every handback has four headings (BLOCKED ON, CHANGED, FOUND, NOT CONFIRMED) and the hospital sentence; no test proves less than before; file-length gate 500 code lines; clippy --workspace --all-targets -- -D warnings; only cargo nextest.
