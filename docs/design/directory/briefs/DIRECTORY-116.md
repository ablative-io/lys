---
type: brief
id: DIRECTORY-116
cluster: directory
title: Add a person offers the app's own relations and its first grant, never a bare read
---

# DIRECTORY-116: Add a person offers the app's own relations and its first grant, never a bare read

> **Cluster:** directory
> **Checklist:**
> - C838 — For an app kind, Add a person lists the app's own schema relations worded by the app's action words with its declared first grant preselected, refuses an undeclared relation by name before any step is written, and a second Add for an admitted email issues the chosen grant. DIRECTORY-116/R1-R3.
> **Stories:**
> - S473 (Directory administrator, Resolves a refused act) — As a Directory administrator adding a person to Cambium, I want the access choices to be Cambium's own (member, administrator, owner, reader) with Cambium's usual first access already chosen, so that the person I add can post the moment they sign in instead of being refused on every act and told to ask on the request page.

## Purpose

Measured 23 Oct 2026 on the live Lys (0.3.2) while Tom added dean@ablative.com.au, david@ablative.com.au and a test person for the 11:30 onboarding. People and agents → Add a person → "Give them access to" cambium.workspace ablative → the "As" select listed Lys's own model relations (`model.relations`, surface/identity/src/features/people/AddPerson.tsx line 259), labelled for an app kind by their raw action words (action-words.ts line 13), so the only choice that read as access was "read; view". Cambium declares four relations on cambium.workspace (administrator, member, owner, reader; member carries post, read, seat_found) and its own Add a person gives member (cambium-door lys_admit.rs). The person was admitted with a relation that carries read alone: David signed in at 11:37 and every post, canvas and item act was refused `grant=none`; Wendy was told "Needed: cambium.place post. Ask for it on the Lys request page." The Give access page already does this right: IssueRoot.tsx readKinds reads each approved app's schema (GET /apps/{id}/schema) and lists `model.kinds[kind].relations ?? model.relations`. Tom, 13 Oct: "a wrong refusal that stops people using it is the failure that cannot happen"; 23 Oct 11:4x: "can you get the permissions fixed".

## Task

Make Add a person list, for an app's kind, that app's own relations from its approved schema, worded by the app's action words, with the app's declared first grant preselected; refuse a first grant naming a relation the app's schema does not declare for that kind; and let a second Add for an already-admitted email issue the chosen grant so the three people admitted on 23 Oct are repaired from the same page.

## Requirements

### R1: An app kind's "As" choices are the app's own relations

WHEN the kind chosen under "Give them access to" is an app's kind (one that contains a dot) THE SYSTEM SHALL list under "As" exactly the relations that app's current approved schema declares for that kind, read the way the Give access page reads them (IssueRoot.tsx readKinds and rolesOf, GET /apps/{id}/schema), each labelled by that app's action words for the relation, and SHALL NOT list a relation of Lys's own model (GET /grants/model `relations`) for an app kind. WHEN the kind is one of Lys's own THE SYSTEM SHALL list Lys's model relations as today.

**Acceptance:**
- surface/identity/src/features/people/people-add-person.test.tsx: with one approved app whose schema declares cambium.workspace with roles administrator, member, owner and reader, choosing cambium.workspace lists those four and none of Lys's own; choosing a Lys kind lists Lys's own.
- The same test: the request sent carries `grant.relation` as the chosen app relation name (member), never an action word.

**Files:**
- modify: surface/identity/src/features/people/AddPerson.tsx
- modify: surface/identity/src/features/people/people-add-person.test.tsx
- modify: surface/identity/src/features/grants/IssueRoot.tsx

**Checklist:**
- C838 — For an app kind, Add a person lists the app's own schema relations worded by the app's action words with its declared first grant preselected, refuses an undeclared relation by name before any step is written, and a second Add for an admitted email issues the chosen grant. DIRECTORY-116/R1-R3.

**Stories:**
- S473 (Directory administrator, Resolves a refused act) — As a Directory administrator adding a person to Cambium, I want the access choices to be Cambium's own (member, administrator, owner, reader) with Cambium's usual first access already chosen, so that the person I add can post the moment they sign in instead of being refused on every act and told to ask on the request page.

### R2: The app's declared first grant is the default choice

An app's schema MAY declare, per kind, the relation a newly admitted person is given first (`first_grant`); Cambium declares member on cambium.workspace. WHEN a kind with a declared first grant is chosen THE SYSTEM SHALL preselect that relation under "As" and say beside it "<app name>'s usual first access"; the person adding may change it. WHEN the schema declares none THE SYSTEM SHALL preselect nothing and require a choice, as today. The declaration is read from the schema Lys already holds; Lys never asks the app.

**Acceptance:**
- crates/lys-identity-server/src/apps_schema_tests.rs (or the schema check tests beside it): a schema declaring `first_grant` naming a relation the kind does not declare is refused by name at POST /apps/{app}/schema/check; one naming a declared relation is accepted and answered by GET /apps/{app}/schema.
- surface/identity/src/features/people/people-add-person.test.tsx: with the schema declaring first_grant member for cambium.workspace, choosing that kind preselects member and shows the words; with no declaration nothing is preselected.

**Files:**
- create: crates/lys-identity-server/src/apps_schema.rs
- create: crates/lys-identity-server/src/apps_schema_tests.rs
- modify: surface/identity/src/features/people/AddPerson.tsx
- modify: surface/identity/src/features/people/people-add-person.test.tsx
- modify: surface/identity/src/generated/grants.ts

**Checklist:**
- C838 — For an app kind, Add a person lists the app's own schema relations worded by the app's action words with its declared first grant preselected, refuses an undeclared relation by name before any step is written, and a second Add for an admitted email issues the chosen grant. DIRECTORY-116/R1-R3.

**Stories:**
- S473 (Directory administrator, Resolves a refused act) — As a Directory administrator adding a person to Cambium, I want the access choices to be Cambium's own (member, administrator, owner, reader) with Cambium's usual first access already chosen, so that the person I add can post the moment they sign in instead of being refused on every act and told to ask on the request page.

### R3: A first grant naming a relation the app does not declare is refused by name, and a second Add repairs an admitted person

WHEN POST /people/admit carries a first grant on an app kind whose relation the app's current schema does not declare for that kind THE SYSTEM SHALL refuse the act before any step is written, as `RelationUnknown` naming the kind and the relation (today the grant step is the fifth of six, after the person is registered and activated). WHEN POST /people/admit names an email already admitted and a first grant THE SYSTEM SHALL finish the steps already done without writing them again and SHALL issue the grant asked for as a further root grant (an earlier grant stands; grants are additive), so a person admitted with read alone is repaired by adding them again with member from the same page.

**Acceptance:**
- crates/lys-identity-server/src/people_admit_tests.rs: an admit whose grant names relation `viewer` on cambium.workspace, when the app's schema declares no such relation, is refused RelationUnknown with no register, account, bind or activate leaf written.
- The same file: admitting dean@ a second time with grant member on cambium.workspace after a first admit with reader answers the six steps with register, account, bind and activate marked already done and the grant step written once more; GET /grants/why for dean@ on cambium.workspace post answers the member grant.

**Files:**
- create: crates/lys-identity-server/src/people_admit_tests.rs
- modify: crates/lys-identity-server/src/people_admit.rs
- modify: crates/lys-identity-server/src/people_admit_app.rs

**Checklist:**
- C838 — For an app kind, Add a person lists the app's own schema relations worded by the app's action words with its declared first grant preselected, refuses an undeclared relation by name before any step is written, and a second Add for an admitted email issues the chosen grant. DIRECTORY-116/R1-R3.

**Stories:**
- S473 (Directory administrator, Resolves a refused act) — As a Directory administrator adding a person to Cambium, I want the access choices to be Cambium's own (member, administrator, owner, reader) with Cambium's usual first access already chosen, so that the person I add can post the moment they sign in instead of being refused on every act and told to ask on the request page.

## Boundaries

- Lys never reads an app's own records or asks the app anything; the relations and the first grant come from the schema Lys already holds for the approved app.
- The Give access page (IssueRoot.tsx) is not changed except to share its readKinds and rolesOf with Add a person; its own choices already come from the app schema.
- No relation is invented and no grant is widened on Lys's side: a person gets exactly the relation chosen, and the choice is the person adding's, with the app's declared first grant only preselected.
- No sleep, timeout or poll anywhere in this work; the page waits on its loads and the acts on their answers.

## Verification

- Red first: the two surface tests and the two server tests written and shown red at the brief commit through `bunx vitest run src/features/people/people-add-person.test.tsx` in surface/identity and `cargo nextest run -p lys-identity-server -E 'test(people_admit) | test(apps_schema)'`; the failing lines kept.
- Green: the same runs, then tsc and biome in surface/identity, then `cargo clippy -p lys-identity-server --all-targets -- -D warnings -W clippy::pedantic`.
- Walked on the install after release: Tom (or Waffles once the administrator grant is held) adds a fresh test person with cambium.workspace ablative, sees member preselected, the person signs in to the door and posts in the Lys room; then adds dean@ and david@ again with member and both post.
- If this code were used in a hospital, the worst credible failure is a person admitted with a relation the adder did not intend, wider than read; the preselection is only a default and the choice stays on the page, so about zero people are harmed beyond what the adder chose.
