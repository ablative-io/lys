---
type: brief
id: DIRECTORY-109
cluster: directory
title: The organisation is a directory record and a grants kind; every person belongs to one; the install's own people are the first organisation
---

# DIRECTORY-109: The organisation is a directory record and a grants kind; every person belongs to one; the install's own people are the first organisation

> **Cluster:** directory
> **Design anchor:**
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-027 — An identity's lifecycle state is folded from its signed transition records and never set directly — The state at any moment is the fold of the identity's transition records in log order, from registered. No API, migration or repair sets a state; a transition validates against the folded state and the cause rule, appends its record through the directory's event path, and the state is read again by folding. A read answers the fold or refuses by name at the coordinate of a record the transition table refuses, and never answers a state past it, repairs the log or skips the record. Rejected: a stored state column set at transition and audited after it; a repair path that writes a state; and treating provisioned as a fifth state rather than a view over the facts beside the state.
> **Checklist:**
> - C795 — The organisation is a directory record (IdentityId::Organisation) founded by one person with ADR-027 lifecycle states, and every person and agent record carries exactly one organisation id, set at registration and never moved across organisations.
> - C796 — The install's own people are an organisation founded at first-run setup with the administrator as founder; an existing store is attached to it at the first boot in one durable batch with one flush, recorded as one boot_stage line, and a failed batch refuses to serve by name.
> - C797 — The grants model ships the kind organisation with organisation.read, organisation.admit, organisation.grant and organisation.administer (the last three withheld from agents), and founding issues exactly one administrator grant to the founder with pass-on bounded to people.
> - C798 — GET /people, /directory/people, /identities, /identities/{id}, /teams, /grants and the who, why, reach and which questions answer only the caller's organisation, another organisation's records answered as absent, with no special case for the install's own.
> - C799 — GET /me names the caller's organisation and the People header shows it.
> **Stories:**
> - S455 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As the founding administrator of my organisation, I hold its administrator grant from the moment it exists, I may pass admitting and granting to my own people, and nobody from the service provider holds anything inside it.
> - S456 (Person who signs in, Keeps their sign-in identities to themselves) — As a person of one organisation, every list I read, people, agents, teams and grants, is my organisation's and nothing else exists as far as Lys tells me; the provider's own administrator reads their organisation the same way.

## Purpose

Tom, 20 October 2026, 10:5x, by voice: an organisation must be able to onboard itself, "seamless and doable alone, without me and without one of my devices". The design page docs/design/onboarding/ORGANISATION-ALONE.md (Lys main bb6c09cc) walks the path and finds Lys has no organisation a tenant could hang off: the only one is the install's settings zone (crates/lys-identity-server/src/configuration_api.rs:87, :100), teams carry no authority (crates/lys-identity-server/src/teams_api.rs:7-9), and lys-secrets' Organisation scope is a path label. This brief is the first seam the page names: the organisation as a record and a kind, every person under one, and our own people the first one so there is one path and not two. ACCESS-009 then scopes admission and roots by it; DIRECTORY-110 founds one from nothing.

## Task

Add the organisation identity kind and its founding change to lys-identity; give every person and agent record one organisation; found the install's organisation at first-run setup and, for an existing store, at the first boot in one durable batch; ship the kind organisation with its four actions and the founding administrator grant; partition every identities, people, teams and grants read by the caller's organisation with no special case for ours; name the organisation on /me and the People header.

## Requirements

### R1: The organisation is a directory record with a lifecycle, founded by one person, and every person and agent carries exactly one

Behavioural. THE SYSTEM SHALL hold organisations in the directory log as records of a new identity kind, IdentityId::Organisation(OrganisationId) (crates/lys-identity/src/id.rs:287), written by two new signed changes in crates/lys-identity/src/event.rs:113 (Change): FoundOrganisation { profile, founder: PersonId } and the existing lifecycle transitions (ADR-027: Registered at founding, Active once the founder's address is proven or, for the install's own organisation, at once; Retired by its administrator). The organisation's name is its profile and is a label: no uniqueness is claimed or enforced beyond the founding person's own, two organisations may share a name and are told apart by id everywhere (docs/design/onboarding/ORGANISATION-ALONE.md, ruling 7). A person's Record (crates/lys-identity/src/projection.rs:45-53) SHALL carry organisation: OrganisationId, set by RegisterPerson and SetupPerson (event.rs:113 onward) from the registering caller's organisation and never changed afterwards; an agent carries its responsible person's organisation (RegisterAgent, the responsible edge) and follows it on ReportsToChanged only within the same organisation, a move across organisations being refused by name. One person, one organisation (ruling 8): the field is one id, named on the page as the thing that becomes a list if ever needed. Projection (crates/lys-identity/src/projection.rs:108) SHALL answer organisation_of(IdentityId) and the organisations it holds; a record with no organisation cannot exist once R2 has run.

**Acceptance:**
- A FoundOrganisation change folds into a Registered organisation record whose founder is the person named; a transition to Active and to Retired folds as ADR-027 folds every identity (lys-identity directory tests)
- RegisterPerson by a caller of organisation A writes a person of organisation A; the projection answers organisation_of for the person and for an agent they register; ReportsToChanged to a person of organisation B is refused by name and moves nothing
- Two organisations with the same name fold to two records with different ids; no refusal

**Files:**
- create: crates/lys-identity/src/organisation.rs
- modify: crates/lys-identity/src/id.rs
- modify: crates/lys-identity/src/event.rs
- modify: crates/lys-identity/src/projection.rs
- modify: crates/lys-identity/src/directory.rs
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C795 — The organisation is a directory record (IdentityId::Organisation) founded by one person with ADR-027 lifecycle states, and every person and agent record carries exactly one organisation id, set at registration and never moved across organisations.

**Stories:**
- S455 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As the founding administrator of my organisation, I hold its administrator grant from the moment it exists, I may pass admitting and granting to my own people, and nobody from the service provider holds anything inside it.

### R2: The install's own people are an organisation too, founded by the install from admin_email with the administrator as founder, and an existing store is brought under it in one durable batch

Behavioural. One path, not two (docs/design/onboarding/ORGANISATION-ALONE.md, ruling 1). WHEN first-run setup records the administrator (SetupPerson, crates/lys-identity-server/src/admission.rs:13-14, the one administrator login per install) THE SYSTEM SHALL in the same batch found the install's organisation, profile from the install's name in deployment.toml and, for an unattended install, admin_email (crates/lys/src/identity/config.rs:70-74), with the administrator as founder, Active at once, and SHALL register the administrator under it. WHEN the server boots a store written before this brief (people with no organisation) THE SYSTEM SHALL, before serving, found the install's organisation and attach every existing person and agent to it as one durable batch with one full flush (Tom, 3 October 2026: a batch is one durable operation, never leaf by leaf), recorded as one boot_stage line naming the count, and SHALL refuse to serve by name if the batch cannot be committed; a second boot finds nothing to do. There is no record without an organisation after this boot, and no code path special-cases the install's own organisation: routes::administrator (crates/lys-identity-server/src/routes.rs:336-344) stays for install-level acts only, apps, the settings zone and the issuer, and every people, teams and grants read of R4 filters our administrator exactly as it filters Priya. Rollback, read at the previous build 043cc007 (the installed Lys): crates/lys-identity/src/encoding_read.rs:94 answers an unknown change kind with Err(malformed("a change kind is 1 to 10")) and :229 propagates it into the fold, and :28-33 decodes a person registration with fields::<1> (crates/lys-identity/src/encoding.rs:276-284, a map of exactly one key), so a FoundOrganisation change or an organisation field on RegisterPerson makes the directory log unreadable to the previous build: it refuses the log, it does not ignore what it does not know. So the install that carries this brief is one-way: THE SYSTEM SHALL, in the installer, copy every store the batch will touch (the directory log and the grant log) to data.previous-organisation/<rfc3339>/ before the batch runs, in the same step that reads the receipt (DIRECTORY-107), and the upgrade fence SHALL name that copy's path in its refusal and on /health, so a put-back to 043cc007 is the copy, by hand, and nothing else. The same batch SHALL stamp every grant in the grant log that carries no organisation (Tom's roots, the agent roots, the door's connector grant, every grant written before this brief) with the install's organisation, as one durable append to the grant log with one flush in the same boot step, the count of stamped grants on the same boot_stage line as the attached records; the judge never reads a missing organisation as the install's (ruling 1 refuses the special case). The copy's path SHALL be written onto the upgrade's own record, install/upgrade.json (crates/lys/src/identity/upgrade/intent.rs:1, :30, where the data.previous copy is already recorded), so a rollback has it in hand without reading a log.

**Acceptance:**
- First-run setup on an empty store ends with one organisation, Active, founder the administrator, and the administrator's record carrying it (the setup test in lys-identity-server)
- A fixture store holding three people and two agents with no organisation boots once: one boot_stage line names 5 attached, every record then carries the install's organisation, and the second boot writes nothing and names 0
- A fixture whose batch is made to fail at commit leaves the store as it was, and the server refuses to serve naming the batch; nothing is attached leaf by leaf
- Rollback is one-way and said so: a test reads the migrated fixture with the previous build's decode (the 043cc007 encoding kept as a test fixture, or the current decoder with the new kinds removed) and asserts the refusal at encoding_read.rs:94; the installer's step copies the directory and grant logs to data.previous-organisation/<rfc3339>/ before the batch and the fence names the path
- The boot batch stamps every grant without an organisation with the install's: a fixture grant log with Tom's three roots, two agent roots and the connector grant boots once, the boot_stage line names 6 stamped, every grant then carries the organisation, and the second boot names 0
- install/upgrade.json names the organisation copy's path after the step, read back by the test

**Files:**
- create: crates/lys-identity-server/src/organisation_boot.rs
- create: crates/lys-identity-server/src/organisation_boot_tests.rs
- modify: crates/lys-identity-server/src/admission.rs
- modify: crates/lys-identity-server/src/routes_startup.rs
- modify: crates/lys/src/identity/config.rs
- modify: crates/lys/src/identity/upgrade/swap.rs
- modify: crates/lys-identity/src/grants/change_stream.rs
- modify: crates/lys/src/identity/upgrade/intent.rs
- modify: crates/lys/src/identity/upgrade.rs

**Checklist:**
- C796 — The install's own people are an organisation founded at first-run setup with the administrator as founder; an existing store is attached to it at the first boot in one durable batch with one flush, recorded as one boot_stage line, and a failed batch refuses to serve by name.

**Stories:**
- S456 (Person who signs in, Keeps their sign-in identities to themselves) — As a person of one organisation, every list I read, people, agents, teams and grants, is my organisation's and nothing else exists as far as Lys tells me; the provider's own administrator reads their organisation the same way.

### R3: organisation is a kind in the grants model with four actions, and founding issues the founder's administrator grant with pass-on bounded to the organisation's own people

Behavioural. THE SYSTEM SHALL ship the kind organisation in the grants model (crates/lys-identity/src/grants/shipped.rs, beside the actions at :24-68; crates/lys-identity/src/grants/model.rs:105 kinds) with the actions organisation.read, organisation.admit (admit a person into it), organisation.grant (issue a root on a resource it owns) and organisation.administer (change its name, retire it, pass its administration on), and the role administrator holding all four. organisation.admit, organisation.grant and organisation.administer are in WITHHELD_FROM_AGENTS (shipped.rs:232 onward): an agent may read its organisation and nothing more. WHEN an organisation is founded (R1, R2) THE SYSTEM SHALL issue, in the same durable batch as the founding change, one root grant on organisation:<id> to the founder with relation administrator and pass_on To { actions: [organisation.admit, organisation.grant, organisation.administer], recipients: [Person] } (crates/lys-identity/src/grants/types.rs:245-255), through a founding path in crates/lys-identity/src/grants/authority.rs beside issue_root (:314) that takes no caller and records the issuer as the organisation itself; it is the one grant Lys issues without a person asking, and the record says so. Nobody else is issued anything at founding. Our administrator holds this grant on the install's organisation and no grant on any other (ruling 2): there is no standing power inside an organisation for anyone but the people its administrator passes it to, and a grant Priya can revoke (crates/lys-identity/src/grants/revocation.rs:71, unchanged) is the only way in.

**Acceptance:**
- The model answers the four actions on kind organisation and the administrator role; an agent asking to hold organisation.admit, organisation.grant or organisation.administer is refused as withheld (the shipped tests beside shipped.rs:448)
- Founding an organisation leaves exactly one grant in the grant log: holder the founder, resource organisation:<id>, relation administrator, pass_on To with the three actions and Person, issuer the organisation; the founder may then delegate organisation.admit to a person of the same organisation and the delegation is judged allowed
- Our administrator, asked by /grants/check for organisation.admit on another organisation, is refused; the install's organisation answers allowed

**Files:**
- modify: crates/lys-identity/src/grants/shipped.rs
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/src/grants/types.rs
- modify: crates/lys-identity/src/grants/admission.rs
- modify: crates/lys-identity-server/src/grants.rs

**Checklist:**
- C797 — The grants model ships the kind organisation with organisation.read, organisation.admit, organisation.grant and organisation.administer (the last three withheld from agents), and founding issues exactly one administrator grant to the founder with pass-on bounded to people.

**Stories:**
- S455 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As the founding administrator of my organisation, I hold its administrator grant from the moment it exists, I may pass admitting and granting to my own people, and nobody from the service provider holds anything inside it.

### R4: Every people, teams, identities and grants read answers within the caller's organisation, with no special case for the install's own

Behavioural. THE SYSTEM SHALL filter by the caller's organisation, from the projection of R1, every read that lists or resolves identities and grants: GET /people and GET /directory/people (crates/lys-identity-server/src/read_api.rs:57, :59), GET /identities and GET /identities/{id} (crates/lys-identity-server/src/routes_table.rs:24-25), GET /teams (openapi_table.rs:353 and crates/lys-identity-server/src/teams_api.rs:259-265), GET /grants (crates/lys-identity-server/src/grants.rs:284), POST /grants/who, /grants/why, /grants/reach and /grants/which (grants.rs:296, :317-320): an identity, team or grant of another organisation is answered as absent, exactly as one the caller may not see is today, never as refused, so no answer says whether another organisation exists. The administrator's wider view (read_api.rs:6) is the wider view within the install's organisation and nothing more. The directory log stays one log and the projection one projection; the partition is the view. GET /me SHALL name the caller's organisation (id and profile) so the screens can show it (surface/identity/src/features/people/People.tsx:28-29 header; surface/identity/src/features/me/). An app's credential belongs to no organisation: a person's or an agent's question to /grants/who, /why, /reach and /which is partitioned by their organisation; an app's question for its own kinds, through its credential (crates/lys-identity-server/src/grants_batch.rs:1-14's rule, unchanged), answers across organisations as today, so the door may ask which workspaces of kind cambium.workspace a subject of any organisation may read (CORE-073).

**Acceptance:**
- With two organisations A and B each holding two people, a team and a root grant, every route named answers A's caller only A's records and B's caller only B's, in the lys-identity-server route tests, one assertion per route
- A's caller reading B's person by id through /identities/{id} is answered as absent with the same status and body as an unknown id
- Our administrator reading /directory/people sees the install's organisation's people only; a test seeds one person of B and asserts they are absent from the answer
- GET /me carries the caller's organisation id and name, read by the People header and shown as text (people-views.test.tsx)
- An app asking /grants/which for its own kind is answered ids across organisations A and B; a person of A asking the same is answered A's ids only

**Files:**
- create: crates/lys-identity-server/src/organisation_reads_tests.rs
- modify: crates/lys-identity-server/src/read_api.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/teams_api.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/grants_batch.rs
- modify: crates/lys-identity-server/src/grants_reach.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: surface/identity/src/features/people/People.tsx
- modify: surface/identity/src/features/people/people-views.test.tsx

**Checklist:**
- C798 — GET /people, /directory/people, /identities, /identities/{id}, /teams, /grants and the who, why, reach and which questions answer only the caller's organisation, another organisation's records answered as absent, with no special case for the install's own.
- C799 — GET /me names the caller's organisation and the People header shows it.

**Stories:**
- S456 (Person who signs in, Keeps their sign-in identities to themselves) — As a person of one organisation, every list I read, people, agents, teams and grants, is my organisation's and nothing else exists as far as Lys tells me; the provider's own administrator reads their organisation the same way.

## Boundaries

- No founding by a stranger here: POST /organisations/found and the proving letter are DIRECTORY-110, after IDENTITY-004 (mail)
- No change to who may admit a person or issue a root: ACCESS-009 scopes people_admit.rs:195 and judge_root (grants/admission.rs:277) by the organisation; this brief only makes the organisation exist and the reads partition
- No organisation in Cambium: the door's workspace per organisation is CORE-073, which asks POST /grants/which for kind cambium.workspace
- Teams keep carrying no authority (teams_api.rs:7-9); they gain an organisation and nothing else
- No mail, no issuer change: the issuer account is made as today by accounts::make (accounts.rs:148-170)
- One person, one organisation; the field becomes a list only by a later brief and nothing else on the page moves (ruling 8)
- An app's credential belongs to no organisation: app questions for the app's own kinds answer across organisations as today; only people's and agents' questions are partitioned
- Follow-up, not this brief: budgets_state.rs:41-43 HolderKind::Organisation means the install today; after this brief the word has two meanings, and a later DIRECTORY row binds it to the organisation record (one budget holder per organisation) so the install's budgets are the install's organisation's

## Verification

- gate -stages nextest -crates lys-identity -test 'test(organisation)' then -crates lys-identity-server -test 'test(organisation)'; every red first, the FAIL line quoted in the handback before the code
- The existing-store boot proved on a copy of a fixture store, never on the installed store; the installed store moves only at the install that carries this brief
- python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory read for FAIL lines, never the exit code
- The whole declared gate on Tom's Mac, START and END posted with df

## Amendments

### Amendment 1: Rollback, app callers, existing grants, budgets' organisation word

- **Date:** 2026-10-20
- **By:** Waffles, written by Flapjack

R2: the previous build (043cc007) refuses a log it cannot decode (encoding_read.rs:94, fields::<1> at encoding.rs:283), so the install is one-way: a store copy before the batch at data.previous-organisation/<rfc3339>/, the fence naming it; and the same batch stamps every existing grant with the install's organisation, counted on the boot_stage line, the judge never defaulting a missing organisation. R4: an app's credential belongs to no organisation; its questions for its own kinds answer across organisations as today. Boundaries: the HolderKind::Organisation follow-up named.
