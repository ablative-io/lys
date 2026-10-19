---
type: brief
id: DIRECTORY-108
cluster: directory
title: An approved app asks whether a resource of its own kind has any holder of an action, and Lys answers held or none from the whole book, naming no one
---

# DIRECTORY-108: An approved app asks whether a resource of its own kind has any holder of an action, and Lys answers held or none from the whole book, naming no one

> **Cluster:** directory
> **Depends on:** DIRECTORY-080
> **Design anchor:**
> - ADR-126 — Applications have connector identities; people register them with explicit permission — An application connector is its own third identity and grant-holder kind. Apps do not act as agents and do not use service accounts, which are accounts people use to service things. Only a signed-in person with an explicit ordinary register_app grant registers an app. The super administrator may grant that permission to a person; an agent, connector or service account cannot register, even if presented with such a grant. Approval creates a connector identity and binds the app to it. No app authority is implicit in approval, binding or ownership of a kind prefix: every permission is an explicit grant with a chain tracing to the super administrator. Own-kind checks, schema acts, batch and which use that connector identity and the ordinary grant engine. Extend IdentityId, grant admission and the permission-store holder, plus event and snapshot representation with compatibility tests. Preserve historical person/agent bytes and signatures; version a representation when necessary, never rewrite history or coerce identities. This supersedes DIRECTORY-048 wording allowing a service account or connector holding register_app to register, and forbids a separate registrar credential as substitute authority.
> **Checklist:**
> - C792 — POST /grants/held answers an approved app held or none for an action on a resource of its own kind, judged from the whole book regardless of the caller's sight, naming no holder.
> - C793 — A held question outside the app's own kinds, with an unknown action, or without a known credential is refused by its existing name and answers nothing.
> - C794 — The OpenAPI document lists POST /grants/held and the grants reference says what held discloses and how it differs from who.
> **Stories:**
> - S454 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app that holds its permissions in Lys, I want to ask whether anyone may administer my app's own resource, so that my app can tell its operator the resource has no root before a person is refused, without my app seeing whom anyone granted.

## Purpose

liminal SRV-038 and SRV-039 (liminal main 8e4bf119, published in liminal-server 0.24.6 on 20 October 2026) ask Lys once at start whether the network has a root: whether any identity may take admin on liminal.network:<network>. A rootless network refuses every grant-bound request, and without the question nothing says so until a seat is refused. Installed on Tom's two exchanges at 08:40 on 20 October, both report lys_root unknown, cause lys_root_refused: the exchange asks POST /grants/who with its app's workspace credential and Lys answers 403 NotAdmitted, "the credential names no service account" (service_account_grants.rs:123-158, reached from grants/handlers.rs:303-326). Reading further shows that letting the app in would not make the answer true. who lists a holder only when the caller sees the grant (grant_sight.rs:28-58): it holds it, issued it, answers for it, or it derives from one of those. Only a root sees every grant. An app's connector (DIRECTORY-080 R1) or a bound service account asking who may admin the network would see an empty page while Tom holds admin, and liminal would log that the network has no root when it has one. The question an app needs is narrower than who and must not depend on sight: does anyone hold this action on this resource of my own kind, yes or no. The answer discloses only existence, never an identity, a grant or a count, and only for kinds the app owns, so it gives the app nothing ADR-126 withholds: no authority, and no sight of whom anyone granted. Hermes (liminal lead, holding the Lys lane), 20 October 2026.

## Task

Add POST /grants/held: an approved app's own credential, or a person who may read grants, asks whether any holder may take one action on one resource of a kind the app owns; Lys judges every grant on the resource through the ordinary engine at the current revision, ignoring the caller's sight, and answers held or none with the revision, naming no holder; refusals are named and nothing is written.

## Requirements

### R1: An app asks whether its own resource has any holder of an action and is answered held or none, from the whole book

Behavioural. WHEN POST /grants/held carries {route, resource {kind, id}, action} and an approved app's credential (Acting::App, apps_binding.rs:201), whether or not the app is bound to a service account, THE SYSTEM SHALL admit the kind by the app's own ownership (admit_kind with Some(app), as grants_batch.rs:177 admits it) and the action by the approved schema (admit_action), then judge every grant on the resource that names the action, as who does (grants/handlers.rs:330-347) but without the caller's sight filter (sees_with is not consulted), each through explain_in at the current frame and instant (grants/handlers.rs:351-376). It answers {held: true, revision} when at least one holder's grant permits the action now, and {held: false, revision} when none does. The answer names no holder, no grant id and no count. A decision that could not be made (crate::grants_batch::unanswered) is the error, never false. The route reads only: it writes nothing, records no receipt and changes no revision. The same route answers a signed-in person or a bearer that grants::caller admits when that caller may read grants as GET /grants admits it today, so an operator can ask the same question by hand.

**Acceptance:**
- An app with no bound service account, asking about liminal.network:ablative and admin, is answered held: true when an administrator grant from the root exists on that resource, and held: false when none does (two fixtures in one test file)
- The held answer is the same whether or not the asking app's connector holds or sees any grant on the resource
- The body holds held and revision only: the test asserts no holder, grant id or count appears in the response bytes
- A grant that has ended by time, or whose holder is retired, is not counted: held is false when it is the only one
- The grant book's revision and every store's head are byte-equal before and after the request

**Files:**
- create: crates/lys-identity-server/src/grants/held.rs
- create: crates/lys-identity-server/src/grants/held_tests.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/openapi_types.rs

**Checklist:**
- C792 — POST /grants/held answers an approved app held or none for an action on a resource of its own kind, judged from the whole book regardless of the caller's sight, naming no holder.

**Stories:**
- S454 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app that holds its permissions in Lys, I want to ask whether anyone may administer my app's own resource, so that my app can tell its operator the resource has no root before a person is refused, without my app seeing whom anyone granted.

### R2: A held question outside the asker's own kinds, or with no credential, is refused by name and answered nothing

Behavioural. WHEN POST /grants/held names a kind the asking app does not own, THE SYSTEM SHALL refuse it not_your_app naming the owning app, as the batch refuses it; WHEN it names an action outside the kind's approved schema, the refusal admit_action gives today; WHEN it carries no credential, or a credential Lys does not know, the refusal POST /grants/who gives today for the same credential; WHEN the caller is a person or service account that may not read grants, NotAdmitted naming the route. A refused question answers no held value and writes nothing. No refusal names a holder or says whether one exists.

**Acceptance:**
- liminal's app asking about a cambium.* kind is refused not_your_app naming cambium, and the body carries no held member
- An unknown action, no credential and an unknown credential are each refused with the refusal /grants/who gives today for the same request, each in one test
- A refused question leaves every store's head byte-equal

**Files:**
- modify: crates/lys-identity-server/src/grants/held.rs
- modify: crates/lys-identity-server/src/grants/held_tests.rs

**Checklist:**
- C793 — A held question outside the app's own kinds, with an unknown action, or without a known credential is refused by its existing name and answers nothing.

**Stories:**
- S454 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app that holds its permissions in Lys, I want to ask whether anyone may administer my app's own resource, so that my app can tell its operator the resource has no root before a person is refused, without my app seeing whom anyone granted.

### R3: The route is in the OpenAPI document and the grants page of the docs, with what it discloses and what it does not

Documentation. THE SYSTEM SHALL list POST /grants/held in the served OpenAPI document beside /grants/who (openapi_types.rs:271) with its body and answer types, and the grants reference shall say in two sentences that held answers from the whole book regardless of the caller's sight, naming no one, and that who answers only what the caller sees, so an app that needs to know whether its network has a root asks held.

**Acceptance:**
- The OpenAPI conformance test that walks the routes finds /grants/held with its request and response schemas
- The grants reference page carries the two sentences, read in the diff

**Files:**
- modify: crates/lys-identity-server/src/openapi_types.rs

**Checklist:**
- C794 — The OpenAPI document lists POST /grants/held and the grants reference says what held discloses and how it differs from who.

**Stories:**
- S454 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app that holds its permissions in Lys, I want to ask whether anyone may administer my app's own resource, so that my app can tell its operator the resource has no root before a person is refused, without my app seeing whom anyone granted.

## Boundaries

- held discloses existence only: never a holder, a grant id, a count, a responsible person or a time; a refusal never says whether a holder exists
- An app asks only about kinds it owns; the route gives an app no authority, no grant and no sight of any grant (ADR-126)
- who keeps its sight filter unchanged; held does not widen what who answers
- The route reads only: no write, no receipt, no revision change
- No fixed wait, timer, poll or deadline anywhere, in code or test (Tom, 15 October 2026)
- Every file touched stays under 500 code lines (ADR-111, by scripts/file-length.sh)

## Verification

- gate -stages nextest -crates lys-identity-server -test 'test(held)'
- gate -stages clippy, then the whole declared gate
- python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory read for FAIL lines

## Amendments

### Amendment 1: Who may ask, where the tests live, and the route's scope

- **Date:** 2026-10-20
- **By:** Hermes, writing R1-R3 (save/directory-108 b418575c)

held admits an approved app's credential for kinds it owns and the administrator for any kind; a signed-in person who is not the administrator, and a registrar, are refused NotAdmitted naming the route's two askers, so R1's last sentence (any caller who may read grants) is replaced by that. The tests are an integration file, crates/lys-identity-server/tests/grants_held.rs, on the identity_contract harness the apps tests use, in place of src/grants/held_tests.rs. The route sits in the route table under the existing grant.who scope, since it discloses less than who, so the shipped grant model gains no action. One test asks who with the app's credential first and fails if the app sees the grant, so the held answer is proven independent of sight.
