---
type: brief
id: ACCESS-003
cluster: access
title: lys-pass: the one client library and its conformance fixtures
---

# ACCESS-003: lys-pass: the one client library and its conformance fixtures

> **Cluster:** access
> **Depends on:** ACCESS-002
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> - ADR-126 — Applications have connector identities; people register them with explicit permission — An application connector is its own third identity and grant-holder kind. Apps do not act as agents and do not use service accounts, which are accounts people use to service things. Only a signed-in person with an explicit ordinary register_app grant registers an app. The super administrator may grant that permission to a person; an agent, connector or service account cannot register, even if presented with such a grant. Approval creates a connector identity and binds the app to it. No app authority is implicit in approval, binding or ownership of a kind prefix: every permission is an explicit grant with a chain tracing to the super administrator. Own-kind checks, schema acts, batch and which use that connector identity and the ordinary grant engine. Extend IdentityId, grant admission and the permission-store holder, plus event and snapshot representation with compatibility tests. Preserve historical person/agent bytes and signatures; version a representation when necessary, never rewrite history or coerce identities. This supersedes DIRECTORY-048 wording allowing a service account or connector holding register_app to register, and forbids a separate registrar credential as substitute authority.
> - ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.
> - ADR-136 — Permissions across the stack: Lys is the one authority; products decide hot routes from a signed pass and ask Lys for deliberate ones; a grant carries its mode — Lys holds every identity and grant; a product permission model is data in its app schema (ADR-116). Every product route is classified once as hot (decided offline from the pass) or deliberate (asked of Lys live, never falling back). The pass is the access token with a rights claim for its audience, short-lived, signed with the key Lys publishes. A grant carries one mode: outright, by_draft or by_two; a held act is a Lys draft the product executes after approval by reading Lys. Reach is by placement, never a pattern; a placement may be restricted. Roles are named bundles in the product schema; a widening by an app waits for the administrator. A machine is the fifth identity. One client library, lys-pass, with conformance fixtures every product gate runs. Availability is stated on each product screen; every deliberate act is receipted in Lys.
> **Checklist:**
> - C608 — lys-pass verifies a pass offline against the published key, audience and time, and evaluates a right by placement reach and restricted children (ACCESS-003 R1).
> - C609 — lys-pass asks Lys for a deliberate route and refuses "Lys could not be asked" when it cannot, never falling back to the pass; hands a by_draft act to Lys and runs approved drafts (ACCESS-003 R2).
> - C610 — The one refusal shape names resource, action, the grant needed and the Lys request URL; the conformance fixtures ship in the crate and every product gate runs them (ACCESS-003 R3).
> **Stories:**
> - S304 (Hermes, a product lead (liminal)) — As a product lead, I want one library that verifies a pass, reaches through parents and refuses in one shape, so that I do not implement permissions a fourth way.
> - S306 (Archie, an agent) — As an agent, I want a refusal to name the grant I need and where to ask, so that I ask the right person once.
> - S307 (Archie, an agent) — As an agent, I want a by_draft act to be held and done after approval without my doing it twice, so that I never work around a draft.

## Purpose

Four products implementing verification, reach, the deliberate check, the draft hand-off and the refusal four ways is where drift and the operating nightmare come from. One crate in this repository, consumed by pinned commit, carries them once, with fixtures every product gate runs (design page D9, section 5).

## Task

Create crates/lys-pass with offline verification, right evaluation, the deliberate-route check, the draft hand-off and runner, the one refusal shape, and the conformance fixtures; a product depends on it by git commit.

## Requirements

### R1: Offline verification and right evaluation

Behavioural. THE SYSTEM SHALL verify a pass with the published key set (fetched and cached by key id), the audience and the time, and SHALL answer allowed, held (mode by_draft or by_two) or refused for (resource kind, id, action) from the pass's rights, following the placement reach the token already applied; a pass with another audience, an expired pass, or a signature by an unpublished key SHALL be refused by name and never partially trusted.

**Acceptance:**
- Each refusal case has a fixture and a test.
- A right on a parent reaches the placed child in the pass; the restricted child is refused.

**Files:**
- create: crates/lys-pass/Cargo.toml
- create: crates/lys-pass/src/lib.rs
- create: crates/lys-pass/src/verify.rs
- create: crates/lys-pass/src/rights.rs
- create: crates/lys-pass/tests/verify.rs
- create: crates/lys-pass/fixtures/passes/
- modify: Cargo.toml

**Checklist:**
- C608 — lys-pass verifies a pass offline against the published key, audience and time, and evaluates a right by placement reach and restricted children (ACCESS-003 R1).

**Stories:**
- S304 (Hermes, a product lead (liminal)) — As a product lead, I want one library that verifies a pass, reaches through parents and refuses in one shape, so that I do not implement permissions a fourth way.

### R2: The deliberate check, the draft hand-off and the draft runner

Behavioural. WHEN a product asks for a deliberate route, THE SYSTEM SHALL ask Lys /grants/check/batch with the pass as the caller and answer what Lys answers; WHEN Lys cannot be reached or answers outside its contract, THE SYSTEM SHALL answer "Lys could not be asked" as a refusal and never consult the pass instead; WHEN a right is held, THE SYSTEM SHALL write the draft (ACCESS-001 R3) with the target, digest and words and answer the draft id; the runner SHALL read approved drafts of the product's kinds with the connector's pass, call the product's executor once per draft, and record executed or refused-on-execution; a draft is never executed twice (the receipt digest is checked before executing).

**Acceptance:**
- Lys down: a deliberate route is refused with the words; a hot route still answers from the pass.
- A held act becomes a draft and the runner executes it exactly once after approval.

**Files:**
- create: crates/lys-pass/src/deliberate.rs
- create: crates/lys-pass/src/drafts.rs
- create: crates/lys-pass/tests/deliberate.rs
- create: crates/lys-pass/tests/drafts.rs

**Checklist:**
- C609 — lys-pass asks Lys for a deliberate route and refuses "Lys could not be asked" when it cannot, never falling back to the pass; hands a by_draft act to Lys and runs approved drafts (ACCESS-003 R2).

**Stories:**
- S307 (Archie, an agent) — As an agent, I want a by_draft act to be held and done after approval without my doing it twice, so that I never work around a draft.

### R3: The one refusal shape and the conformance fixtures

Behavioural. THE SYSTEM SHALL give one refusal value {refused: {resource, action, needed, request}} where needed names the grant (app, kind, id, action) and request is the Lys URL that asks for it; the crate SHALL ship a fixture set (wrong audience, expired, unpublished key, parent reaches child, restricted child, held mode on a hot route refused, the refusal bytes, deliberate route with Lys unreachable) and a test harness a product gate runs against its own routes; a product whose gate does not run the harness is not conformant and its brief says so.

**Acceptance:**
- The fixtures run green in this crate; the Cambium brief runs them first.

**Files:**
- create: crates/lys-pass/src/refusal.rs
- create: crates/lys-pass/src/conformance.rs
- create: crates/lys-pass/fixtures/refusals/
- create: crates/lys-pass/tests/conformance.rs

**Checklist:**
- C610 — The one refusal shape names resource, action, the grant needed and the Lys request URL; the conformance fixtures ship in the crate and every product gate runs them (ACCESS-003 R3).

**Stories:**
- S306 (Archie, an agent) — As an agent, I want a refusal to name the grant I need and where to ask, so that I ask the right person once.

## Boundaries

- SHALL NOT hold any authority of its own: a key, a token or a credential.
- SHALL NOT fall back from a deliberate check to the pass.
- SHALL NOT be published to crates.io; products depend by git commit.

## Verification

- This handwritten brief passes the design gate (scripts/design/gate.sh), judged by its parsed failures, never by its exit code; the rendered markdown matches what render-cluster.py writes.
- Written whole, read whole by the lead, then one battery on the Mac (every leg green) and the whole gate on Dean through aion at the same sha; then one install and a browser walk with pictures of every screen the brief touches.
- Every handback has four headings (BLOCKED ON, CHANGED, FOUND, NOT CONFIRMED) and the hospital sentence; no test proves less than before; file-length gate 500 code lines; clippy --workspace --all-targets -- -D warnings; only cargo nextest.
