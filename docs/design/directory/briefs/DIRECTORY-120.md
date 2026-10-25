---
type: brief
id: DIRECTORY-120
cluster: directory
title: A person added to an organisation gets its product's first access by default, an admit that gives no access says so where it is read, and a person refused at a product's door asks to be let in with one click that an administrator approves
---

# DIRECTORY-120: A person added to an organisation gets its product's first access by default, an admit that gives no access says so where it is read, and a person refused at a product's door asks to be let in with one click that an administrator approves

> **Cluster:** directory
> **Depends on:** DIRECTORY-113, DIRECTORY-116
> **Checklist:**
> - C843 — Add a person preselects the organisation's one product's kind, resource and declared first access, changeable by the person adding; never "No access yet" when a first access could be. DIRECTORY-120/R1.
> - C844 — An admission giving no access, whoever asked for it, says so in its receipt (no_access line), its letter and the People row; it is never refused for it. DIRECTORY-120/R2.
> - C845 — An approved app on its agent pass records a draft of its declared first grant for a refused person; one open draft per person and resource; administrators approve or refuse it in Drafts; the door never issues a grant. DIRECTORY-120/R3.
> - C846 — Standing bars hold for DIRECTORY-120; no fixed wait; no test proves less than before. DIRECTORY-120/R4.
> **Stories:**
> - S478 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator adding a person to an organisation that has one product, I want their usual first access already chosen for me, so that adding a person is one act and nobody is made without access by my oversight.
> - S479 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want every person who has no access to anything to be marked so where I read the people, and said so in the admission's receipt and letter, so that I give it before they meet a door and not after.
> - S480 (Person refused at a product's door, Was added to the organisation but holds no access to the product yet) — As a person refused at a product's door, I want to ask to be let in with one click and have the right people see the ask where they already decide things, so that a refusal is a short wait and never a dead end.
> - S481 (Directory administrator, Resolves a refused act) — As a directory administrator, I want this change held to every standing bar with the screens' tests red first, so that the default and the ask are proven on the real routes and nothing is assumed.

## Purpose

On 25 October 2026, 12:18 to 12:21 AEDT, Dean Whiting signed in to Cambium at cambium.ablative.com.au seven times and was refused seven times with "you do not have permission to open this workspace" (the Mac door's log, serve-20261024T215544Z-bcc3a0a3.log), with Tom beside him. He had been admitted to Lys the day before through POST /people/admit with no first grant: people_admit.rs granted_and_written issues a grant only `if let Some(first) = asked.grant`, and nothing in the act, the receipt, the handed letter or the people list said that a person with no access had just been made. The screen has the same default: AddPerson.tsx line 268 preselects "No access yet", and its own comment (line 23) says why: "a fresh install has nothing to grant yet". DIRECTORY-116 R2 made the app's declared first grant the default relation once a kind is chosen, but the kind is never chosen for the person adding, so the usual case, one organisation with one product, still needs a hand on two controls. Cambium declares its first grant (crates/cambium-door/lys-app-schema.json:213, `first_grant: member` on cambium.workspace), so Lys already knows what a newly added person usually gets. At the door the person meets a dead end: Cambium's refusal page offers "Start sign-in again" and nothing else, names no one who could let them in, and gives them no way to ask. Lys holds nine people tonight; four of them (David, Wendy, Josh, Annabel) can sign in to Lys and will be refused at Cambium exactly as Dean was. Tom, 13 October: a wrong refusal that stops people using it is the failure that cannot happen; 20 October: onboarding must be seamless and doable alone, without him and without his devices.

## Task

Make the usual case need no second hand: a person added to an organisation with one product is given that product's declared first access unless the person adding chooses otherwise; an admission that gives no access says so in its receipt, its letter and the people list, whoever asked for it (the screen, a product's pass, an agent); and a person refused at a product's door can ask to be let in with one click, the ask arriving as a draft of the exact grant before the organisation's administrators, who approve it where they already decide drafts. Lys names no product in any of this (DIRECTORY-114): every word comes from the app's own registration and schema.

## Requirements

### R1: Add a person preselects the organisation's product and its first access when there is exactly one

WHEN the organisation has exactly one approved app whose schema declares a first grant (`first_grant`) on a kind of which the person adding may give exactly one resource THE SYSTEM SHALL preselect that kind, that resource and that relation in Add a person, and say beside them "<app name>'s usual first access"; the person adding may change any of the three, including to "No access yet", by their own choice. WHEN there are several such apps, several resources, or none THE SYSTEM SHALL preselect nothing and require the choice, as today; "No access yet" is never preselected when a first access could be. The app's name and relation words come from its registration and schema as DIRECTORY-116 reads them; Lys adds no word of its own.

**Acceptance:**
- Red first (surface/identity/src/features/people/people-add-person.test.tsx): with one approved app declaring first_grant member on a kind the caller may give one resource of, the form opens with kind, resource and relation preselected and the usual-first-access words beside them; with two such apps nothing is preselected; with none, "No access yet" and no grant, as today.
- The same test file: choosing "No access yet" over a preselection sends no grant, and the admitted view says "They have no access yet" as today.

**Files:**
- modify: surface/identity/src/features/people/AddPerson.tsx
- modify: surface/identity/src/features/people/people-add-person.test.tsx

**Checklist:**
- C843 — Add a person preselects the organisation's one product's kind, resource and declared first access, changeable by the person adding; never "No access yet" when a first access could be. DIRECTORY-120/R1.

**Stories:**
- S478 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator adding a person to an organisation that has one product, I want their usual first access already chosen for me, so that adding a person is one act and nobody is made without access by my oversight.

### R2: An admission that gives no access says so in its receipt, its letter and the people list

WHEN POST /people/admit (from the screen, a product's person or agent pass, or an administrator's agent) names no first grant and the organisation has at least one approved app declaring a first grant THE SYSTEM SHALL record in the admit receipt a `no_access` line naming each such app, kind and relation ("<app> declares first access <relation> on <kind>; none given"), SHALL write in the handed or mailed letter that the person can sign in to Lys and has no access to anything else yet, and SHALL show "No access yet" on that person's row in People (People.tsx) with the way to give it (Access). The admission is never refused for it: a person with no access is a valid state the administrator chose, said aloud, never a silent one.

**Acceptance:**
- Red first (crates/lys-identity-server/src/people_admit_tests.rs or the tests beside people_admit.rs): an admit with no grant on an install holding an approved app with first_grant answers 200 with the no_access line in the receipt naming the app, kind and relation; an admit with the grant has no such line; an install with no app declaring one has no such line.
- crates/lys-identity-server/src/people_admit_handed_tests.rs (or beside people_admit_handed.rs): the handed letter's text says "no access to anything else yet" exactly when the receipt carries no_access.
- surface/identity/src/features/people/people-views.test.tsx: a person whose grants name no app resource shows the "No access yet" pill with the Access link; one with a grant does not.

**Files:**
- modify: crates/lys-identity-server/src/people_admit.rs
- modify: crates/lys-identity-server/src/people_admit_app.rs
- modify: crates/lys-identity-server/src/people_admit_handed.rs
- modify: surface/identity/src/features/people/People.tsx
- modify: surface/identity/src/features/people/Pill.tsx
- modify: surface/identity/src/features/people/people-views.test.tsx

**Checklist:**
- C844 — An admission giving no access, whoever asked for it, says so in its receipt (no_access line), its letter and the People row; it is never refused for it. DIRECTORY-120/R2.

**Stories:**
- S479 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want every person who has no access to anything to be marked so where I read the people, and said so in the admission's receipt and letter, so that I give it before they meet a door and not after.

### R3: A person refused at a product's door asks to be let in: one draft of the exact grant, approved where drafts are decided

WHEN an approved app, on its own agent pass (the pass AGENTS-019 R2 admits for admission), records a draft (POST /drafts) of POST /grants/roots naming a person of the organisation as holder, a resource of the app's own kind, and the relation the app's schema declares as its first grant THE SYSTEM SHALL accept the draft with the organisation's administrators as its reviewers, SHALL list it in Drafts as "<person> asked to open <app>'s <resource>" with the app's refusal words beside it, and SHALL answer the app the draft's id and state. WHEN such a draft is already open for the same person and resource THE SYSTEM SHALL answer the open draft's id and record no second. WHEN an administrator approves THE SYSTEM SHALL issue the grant through the existing approval path and nothing else; WHEN they refuse THE SYSTEM SHALL write nothing to the person's grants and keep the refusal's words for the app to read. A draft naming any other relation, kind or holder SHALL be refused by name as today.

**Acceptance:**
- Red first (crates/lys-identity-server/src/drafts_api_tests.rs): an app agent pass records the draft of the declared first grant for a person of the organisation and gets 201 with id and state open; the same ask again answers the same id; a draft naming a relation the app does not declare is refused by name; approval by an administrator issues the grant and the person's /agent/grants-equivalent read shows it; refusal issues nothing.
- crates/lys-identity-server/src/drafts_list_tests.rs: the list row carries the asked-to-open words with the person's and the app's display names.
- surface/identity/src/features/drafts tests: the Drafts page shows the ask and its Approve and Refuse act on the existing routes.

**Files:**
- modify: crates/lys-identity-server/src/drafts_api.rs
- modify: crates/lys-identity-server/src/drafts_prepare.rs
- modify: crates/lys-identity-server/src/drafts_list.rs
- modify: crates/lys-identity-server/src/drafts_api_tests.rs
- modify: crates/lys-identity-server/src/drafts_list_tests.rs
- modify: surface/identity/src/features/drafts/Drafts.tsx
- modify: surface/identity/src/features/drafts/DraftDecide.tsx

**Checklist:**
- C845 — An approved app on its agent pass records a draft of its declared first grant for a refused person; one open draft per person and resource; administrators approve or refuse it in Drafts; the door never issues a grant. DIRECTORY-120/R3.

**Stories:**
- S480 (Person refused at a product's door, Was added to the organisation but holds no access to the product yet) — As a person refused at a product's door, I want to ask to be let in with one click and have the right people see the ask where they already decide things, so that a refusal is a short wait and never a dead end.

### R4: Standing bars hold for DIRECTORY-120; no fixed wait; no test proves less than before

THE SYSTEM SHALL hold every standing bar on the exact final commit: cargo fmt, clippy pedantic -D warnings with all targets, cargo nextest run (never cargo test), cargo test --doc, ast-grep, the file-length gate, tsc and vitest for surface/identity, and scripts/design/gate.sh with the rendered markdown equal to its JSON. No sleep, timeout taken as readiness or poll is added. No test is deleted, weakened or marked serial.

**Acceptance:**
- Every gate leg green on the final commit, each judged by its parsed output lines, never by its exit code.
- scripts/design/gate.sh passes for this brief, judged by its parsed output.

**Checklist:**
- C846 — Standing bars hold for DIRECTORY-120; no fixed wait; no test proves less than before. DIRECTORY-120/R4.

**Stories:**
- S481 (Directory administrator, Resolves a refused act) — As a directory administrator, I want this change held to every standing bar with the screens' tests red first, so that the default and the ask are proven on the real routes and nothing is assumed.

## Boundaries

- SHALL NOT refuse an admission for giving no access, and SHALL NOT issue a grant the person adding did not choose: the default is a preselection the person adding sees and can change, never a grant given behind the screen.
- SHALL NOT approve an ask without an administrator's decision, and SHALL NOT let a product's door issue a grant: the door records the ask, Lys's administrators decide it.
- SHALL NOT name any product in Lys's own words (DIRECTORY-114): the app's name, kind and relation words come from its registration and schema.
- SHALL NOT assume mail: with no mail settings the letter is handed on screen as today (DIRECTORY-110 and the handed path).
- SHALL NOT add a sleep, a timeout taken as readiness, or a poll anywhere.

## Verification

- cargo nextest run -p lys-identity-server -E 'test(people_admit) | test(drafts)' alone, read by its PASS and FAIL lines; vitest on surface/identity for people-add-person, people-views and drafts, read by its summary line; then the full gate.
- Red first: every new test is run before the code and its failing lines kept in the handback.
- Walked on the installed Lys with the installed Cambium (CORE-086): Tom adds a person in Lys and sees the product's first access preselected; a person admitted with no access signs in at Cambium, is refused with the names of who can let them in, clicks ask; the ask appears in Drafts; Tom approves; the person signs in and is in. Each step's screen is read, not assumed.
- Hospital line: If this code were used in a hospital, the worst credible failure is a nurse added to the ward's system in the morning who is refused at its door at the start of their shift with no one named to let them in, and it could harm about every patient on that nurse's round until someone with a terminal is found.
