---
type: brief
id: ACCESS-009
cluster: access
title: Admission and root grants are organisation-scoped: organisation.admit admits into one's own organisation, organisation.grant is the root authority for its resources, pass-on stays inside, and the People and Access screens are the organisation's
---

# ACCESS-009: Admission and root grants are organisation-scoped: organisation.admit admits into one's own organisation, organisation.grant is the root authority for its resources, pass-on stays inside, and the People and Access screens are the organisation's

> **Cluster:** access
> **Depends on:** DIRECTORY-109
> **Design anchor:**
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> **Checklist:**
> - C800 — POST /people/admit and POST /people are judged by organisation.admit on the caller's own organisation through the grants engine; a body naming another organisation is refused organisation_not_own before any write; routes::administrator is not consulted.
> - C801 — judge_root's root authority is a function of the resource: organisation.grant on the caller's organisation for organisation-owned kinds, the install's person for install-level kinds; every root records its organisation; a resource rooted under another organisation is refused resource_owned_elsewhere while any grant on it lives.
> - C802 — A delegation to a recipient outside the issuer's organisation, directly or through an agent's, service account's, connector's or machine's owner, is refused recipient_outside_organisation; revocation keeps revocation.rs:71's rule.
> - C803 — People and Access show the caller's organisation and only its records, from the partitioned routes without client filtering, with the add-person and root acts shown by GET /authority's organisation.admit and organisation.grant; no screen branches on the install's own organisation.
> **Stories:**
> - S457 (Tom, owner and administrator) — As the administrator of my own organisation's Lys, I admit my people and give them our Cambium by grants I hold on my organisation, and the provider's administrator cannot do either inside it unless I pass them the right and I can take it back.
> - S458 (A clinician, a person in a hospital workspace) — As a person admitted into my organisation, what my administrator passes to me stays inside our organisation: I cannot be given anything by, or pass anything to, someone outside it, and my People and Access screens show only us.

## Purpose

The design page docs/design/onboarding/ORGANISATION-ALONE.md (Lys main bb6c09cc) finds that an organisation's own administrator cannot admit her people (/people/admit is routes::administrator at crates/lys-identity-server/src/people_admit.rs:195) or give them anything (judge_root refuses every caller but the one root authority person, crates/lys-identity/src/grants/admission.rs:277), and that a delegation cannot admit anyone. With DIRECTORY-109's organisation in place, this brief moves both acts to grants she holds on her organisation, keeps pass-on inside it, and gives her the screens, so Tom's bar, alone and without us, holds for the two acts that let her people in and give them Cambium. Our own administrator does the same acts by the same grants on the install's organisation, with no wider power anywhere.

## Task

Judge POST /people/admit and POST /people by organisation.admit on the caller's own organisation; make judge_root's root authority a function of the resource (organisation.grant for organisation-owned kinds, the install's person for install-level kinds) and record the organisation on every root; refuse a delegation to a recipient outside the issuer's organisation; make the People and Access screens the caller's organisation's with the add and root acts shown by authority only.

## Requirements

### R1: POST /people/admit admits into the caller's own organisation by the organisation.admit grant, never by routes::administrator

Behavioural. THE SYSTEM SHALL replace the check at crates/lys-identity-server/src/people_admit.rs:194-195 (signed_in, then crate::routes::administrator) with: the caller is an active person (caller_admission::active_caller) who holds organisation.admit on their own organisation, judged by the grants engine at the current revision exactly as a route grant is judged (crates/lys-identity-server/src/route_actions.rs:13-40); the body MAY name organisation, and WHEN it does and it is not the caller's own THE SYSTEM SHALL refuse by name (organisation_not_own) before any step, writing nothing. The five steps (people_admit.rs:35) run as today; register_person (:265-281) writes the person under the caller's organisation (DIRECTORY-109 R1); the optional root grant (:327-343) is issued under R2's rule with the caller as its root authority. Our administrator admits into the install's organisation by the same grant and no other way (DIRECTORY-109 R3, ruling 1); routes::administrator is not consulted on this route. POST /people (routes_table.rs:15) and the declared scope person.create (openapi_table.rs:150, 166) SHALL be judged the same way, so no second path registers a person.

**Acceptance:**
- A person of organisation A holding organisation.admit on A admits a person, who is written under A, Active, with the issuer account made as today; a person of A without the grant is refused NotAdmitted naming organisation.admit (people_admit tests)
- A body naming organisation B from a caller of A is refused organisation_not_own with no directory, issuer or grant write; the receipt names no step
- Our administrator admits into the install's organisation through the founding grant; with that grant revoked in the fixture they are refused like anyone else
- POST /people without organisation.admit is refused the same way; openapi lists the refusal on both routes

**Files:**
- modify: crates/lys-identity-server/src/people_admit.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/tests/people_admit.rs

**Checklist:**
- C800 — POST /people/admit and POST /people are judged by organisation.admit on the caller's own organisation through the grants engine; a body naming another organisation is refused organisation_not_own before any write; routes::administrator is not consulted.

**Stories:**
- S457 (Tom, owner and administrator) — As the administrator of my own organisation's Lys, I admit my people and give them our Cambium by grants I hold on my organisation, and the provider's administrator cannot do either inside it unless I pass them the right and I can take it back.

### R2: The root authority for a resource is a function of the resource: the holder of organisation.grant for a resource of an organisation, the install's person for an install-level kind

Behavioural. judge_root (crates/lys-identity/src/grants/admission.rs:273-281) today compares request.caller with one root_authority: PersonId (crates/lys-identity/src/grants/authority.rs:121, passed at crates/lys-identity-server/src/grants.rs:134 and :172). THE SYSTEM SHALL judge a root request by the resource: WHEN the resource's kind is organisation-owned (every kind except the one install-level kind the tree roots today: directory with collection apps (directory:apps, the root the administrator holds for registering apps, crates/lys-identity-server/src/agent_roots.rs:35 COLLECTIONS and crates/lys-identity-server/src/apps_register.rs:28); the settings zone and the issuer are not grant kinds, they are routes::administrator acts and stay so; directory:agents, person, agent, machine, account, budget, team and policy (the other kinds crates/lys-identity-server/src/setup.rs and agent_roots.rs root) are organisation-owned) THE SYSTEM SHALL require the caller to hold organisation.grant on their own organisation and SHALL record the organisation on the issued grant, so every grant in the lineage below it carries the organisation that owns the resource; WHEN the kind is install-level THE SYSTEM SHALL require today's root authority person, unchanged. A root on an organisation-owned resource already rooted under another organisation is refused by name (resource_owned_elsewhere): a resource belongs to one organisation for as long as any grant on it lives. POST /grants/roots (crates/lys-identity-server/src/grants/handlers.rs:105-122) and the admit step that issues one (people_admit.rs:327-343) both go through this judge; POST /grants/agent-roots (grants.rs:287, agent_roots.rs:135, :151-158) issues an agent's setup roots under its responsible person's organisation. Our administrator gains nothing and loses nothing: root authority for our install-level kinds and, by the founding grant, for our organisation's resources, and none for Priya's. Every grant already in the installed log carries its organisation from DIRECTORY-109 R2's boot batch (the install's), so resource_owned_elsewhere reads a stamped organisation on every lineage and never a missing one. An app's credential belongs to no organisation: an app is never a root authority and never a holder under an organisation.

**Acceptance:**
- A founder of organisation A issues a root on cambium.workspace:<A> to one of A's people and the grant records organisation A; the same caller asked for a root on an install-level kind is refused RootAuthorityRefused as today
- A founder of B asked for a root on cambium.workspace:<A> while A's grant lives is refused resource_owned_elsewhere; after A's grants on it are all revoked, B may root it
- The install's root authority person issues a root on an install-level kind as today, and is refused a root on a resource of B
- The grant log's record and /grants/{id} carry the organisation; /grants/why names it in the lineage
- A root on directory:apps is judged by the install's root authority person as today, and a founder of A asking for it is refused RootAuthorityRefused; a root on a machine or a team by A's founder is allowed and records A

**Files:**
- create: crates/lys-identity-server/src/grants/organisation_roots_tests.rs
- modify: crates/lys-identity/src/grants/admission.rs
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/src/grants/mode.rs
- modify: crates/lys-identity/src/grants/types.rs
- modify: crates/lys-identity/src/grants/shipped.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/grants/handlers.rs
- modify: crates/lys-identity-server/src/agent_roots.rs
- modify: crates/lys-identity-server/src/people_admit.rs

**Checklist:**
- C801 — judge_root's root authority is a function of the resource: organisation.grant on the caller's organisation for organisation-owned kinds, the install's person for install-level kinds; every root records its organisation; a resource rooted under another organisation is refused resource_owned_elsewhere while any grant on it lives.

**Stories:**
- S457 (Tom, owner and administrator) — As the administrator of my own organisation's Lys, I admit my people and give them our Cambium by grants I hold on my organisation, and the provider's administrator cannot do either inside it unless I pass them the right and I can take it back.

### R3: Pass-on stays inside the organisation: a delegation names only recipients of the issuer's organisation and resources it owns; revocation is unchanged

Behavioural. THE SYSTEM SHALL add one rule to delegation judging (delegation_authority, crates/lys-identity-server/src/grants/handlers.rs:124-179 and crates/lys-identity/src/grants/admission.rs): the recipient, person, agent, service account, connector or machine, belongs to the issuing grant's organisation (an agent, service account, connector or machine through its owner's record, DIRECTORY-109 R1), else the delegation is refused by name (recipient_outside_organisation) and nothing is written; and the resource is one the lineage's root recorded under that organisation (R2), which the lineage check already walks. PassOn::To's recipients list (crates/lys-identity/src/grants/types.rs:245-255) keeps its meaning; the rule narrows who may stand in it. Revocation (crates/lys-identity/src/grants/revocation.rs:71: the root authority or the issuer) is unchanged in words and widened in meaning by R2: the root authority of a resource of organisation A is A's organisation.grant holder. A grant Priya can revoke is the only way for anyone outside her organisation to act inside it (ruling 2), and this rule is what makes it the only way. Apps and connectors belong to no organisation, so no delegation reaches them: a recipient that is an app's credential or a connector is refused recipient_outside_organisation like any recipient outside it; the door's access to Lys is its app credential and the connect flow, not a grant.

**Acceptance:**
- A holder in A delegating to a person of B is refused recipient_outside_organisation; to a person of A, allowed, and the grant records A
- A holder in A delegating to an agent whose responsible person is in B is refused the same way; the agent of a person in A is allowed
- A's founder revokes a grant their person passed on; B's founder asked to revoke it is refused as today
- A delegation naming an app credential or a connector as recipient is refused recipient_outside_organisation

**Files:**
- modify: crates/lys-identity/src/grants/admission.rs
- modify: crates/lys-identity-server/src/grants/handlers.rs
- modify: crates/lys-identity-server/src/grants_delegation_tests.rs

**Checklist:**
- C802 — A delegation to a recipient outside the issuer's organisation, directly or through an agent's, service account's, connector's or machine's owner, is refused recipient_outside_organisation; revocation keeps revocation.rs:71's rule.

**Stories:**
- S458 (A clinician, a person in a hospital workspace) — As a person admitted into my organisation, what my administrator passes to me stays inside our organisation: I cannot be given anything by, or pass anything to, someone outside it, and my People and Access screens show only us.

### R4: Priya's screens: People and Access are her organisation's, the add-person act names it, and our administrator's screens show only ours

Behavioural. Or she has routes and no door (Waffles, 20 October 2026). THE SYSTEM SHALL show on surface/identity/src/features/people/People.tsx the caller's organisation name from GET /me (DIRECTORY-109 R4) above the list, and list people, agents and teams from the partitioned routes without a client-side filter; surface/identity/src/features/people/AddPerson.tsx:18 onward SHALL name the organisation the person is being added to (read-only, the caller's own), SHALL send no organisation field, and SHALL show the add act only to a caller whose GET /authority (read_api.rs:55) answers organisation.admit, otherwise the words of ADR-011 for a right not held; the copy-invite words (AddPerson.tsx:106-138) stay until IDENTITY-004 removes them. surface/identity/src/features/access/Access.tsx, Resources.tsx and Graph.tsx SHALL list and draw only resources and holders the partitioned grants routes answer (/grants, /grants/reach, /grants/who), the organisation named in the Access header, and SHALL offer the root act (Resources.tsx) only to a caller whose authority answers organisation.grant. No screen has a branch for the install's own organisation: our administrator's People and Access read as Priya's do, scoped to ours.

**Acceptance:**
- people-views.test.tsx: a caller of A sees A's name in the header and only A's people, agents and teams, from a mocked API that also returns nothing of B because the route does not
- people-add-person.test.tsx: the add act is shown to a caller whose authority answers organisation.admit and names A; a caller without it sees the right-not-held words and no form
- a new access-organisation.test.tsx: Access names A, lists only A's resources and holders, and offers the root act only with organisation.grant
- bun run test in surface/identity: the three files pass; the summary line read, never the exit code

**Files:**
- create: surface/identity/src/features/access/access-organisation.test.tsx
- modify: surface/identity/src/features/people/People.tsx
- modify: surface/identity/src/features/people/AddPerson.tsx
- modify: surface/identity/src/features/people/people-views.test.tsx
- modify: surface/identity/src/features/people/people-add-person.test.tsx
- modify: surface/identity/src/features/access/Access.tsx
- modify: surface/identity/src/features/access/Resources.tsx
- modify: surface/identity/src/features/access/Graph.tsx

**Checklist:**
- C803 — People and Access show the caller's organisation and only its records, from the partitioned routes without client filtering, with the add-person and root acts shown by GET /authority's organisation.admit and organisation.grant; no screen branches on the install's own organisation.

**Stories:**
- S458 (A clinician, a person in a hospital workspace) — As a person admitted into my organisation, what my administrator passes to me stays inside our organisation: I cannot be given anything by, or pass anything to, someone outside it, and my People and Access screens show only us.

## Boundaries

- No founding and no mail: DIRECTORY-110 and IDENTITY-004
- No widening for our administrator: no route gains a branch for the install's organisation; routes::administrator keeps apps, the settings zone and the issuer and loses /people/admit and /people
- No change to what a grant permits once issued, to /grants/check, /grants/check/batch or /grants/which: they answer the subject's grants as today
- No change to the door: CORE-073 asks /grants/which for cambium.workspace and is written after this brief
- Agents' seats in an organisation's workspace are the sixth brief the page names, not this one
- Apps and connectors belong to no organisation: no delegation reaches them, an app is never a root authority, and an app's questions for its own kinds (/grants/which, /check, /check/batch) answer across organisations as today
- The install-level kinds are named here from the tree, not decided at build time: directory:apps alone; the settings zone and the issuer are routes::administrator acts, not kinds

## Verification

- gate -stages nextest -crates lys-identity -test 'test(organisation)' then -crates lys-identity-server -test 'test(admit) | test(organisation) | test(delegat)'; every red first with the FAIL line quoted
- bun run test in surface/identity, the summary line read and both new and changed test files named in it
- python3 scripts/design/validate.py docs/design/access and python3 scripts/design/check-coverage.py docs/design/access read for FAIL lines, never the exit code
- The whole declared gate on Tom's Mac, START and END posted with df

## Amendments

### Amendment 1: App callers, existing grants, the install-level kinds named

- **Date:** 2026-10-20
- **By:** Waffles, written by Flapjack

R2: the install-level kinds are named from the tree now (directory:apps; the settings zone and the issuer are administrator acts, not kinds), every existing grant carries the install's organisation from DIRECTORY-109 R2's batch, and an app is never a root authority. R3 and Boundaries: apps and connectors belong to no organisation, so no delegation reaches them; app questions for their own kinds answer across organisations as today.
