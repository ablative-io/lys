---
type: brief
id: DIRECTORY-124
cluster: directory
title: Sign-in accounts follow the directory: People says whose sign-in works, Lys finds or makes a missing account under the person's own address, and an account it cannot tie to one address signs in as nobody
---

# DIRECTORY-124: Sign-in accounts follow the directory: People says whose sign-in works, Lys finds or makes a missing account under the person's own address, and an account it cannot tie to one address signs in as nobody

> **Cluster:** directory
> **Depends on:** DIRECTORY-121, DIRECTORY-122
> **Checklist:**
> - C868 — People says whether each person's sign-in works and what it needs, in words, with their Account link. DIRECTORY-124/R1.
> - C869 — Lys finds or makes each active person's missing sign-in account under their own address at start and after every directory change; a second repair changes nothing. DIRECTORY-124/R2.
> - C870 — An administrator sets the address of a person with no account and Lys makes it; another person's address is refused by name. DIRECTORY-124/R3.
> - C871 — An account Lys cannot tie to exactly one address signs in as nobody, refused by name. DIRECTORY-124/R4.
> - C872 — Standing bars hold for DIRECTORY-124, with the 26 October records replayed. DIRECTORY-124/R5.
> **Stories:**
> - S503 (Directory administrator, Resolves a refused act) — As a directory administrator, I want People to say for each person whether their sign-in works and, when it does not, what it needs, so that a person who cannot sign in is seen before they ask.
> - S504 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in, I want Lys to find or make my sign-in account under my own address whenever it is missing, so that an issuer wiped or an account deleted behind Lys never locks me out or opens someone else's records.
> - S505 (Directory administrator, Resolves a refused act) — As a directory administrator, I want to set the address of a person who has no sign-in account and have Lys make it, refused by name when that address is another person's, so that I give access to the right person and only them.
> - S506 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in, I want an account Lys cannot tie to exactly one of my addresses to sign in as nobody, so that I never open another person's records by signing in with an account that carries their address.
> - S507 (Directory administrator, Resolves a refused act) — As a directory administrator, I want the 26 October records replayed as a test, so that Annabel signing in as Dean is proven impossible and its repair proven to take one move and one address.

## Purpose

Tom, 27 October 2026, after Annabel signed in and Lys opened Dean's record: "Is there just a way for her to log out as Dean and log in properly? ... This is not an issue that can happen again ... This needs to be like production quality. This needs to be enterprise grade." And then: "we need to be able to wipe the system and start again and have it just work ... Think about how actual IAM systems work and like make sure it's done properly and secure." Read on the live install that morning: of eleven people's sign-in accounts, five no longer exist at the issuer (Annabel's, Josh's, two Davids', an early Dean's), deleted outside Lys between 15 and 22 October with nothing in Lys noticing; Dean's second record's account had been rewritten to Annabel's address by Lys's own email change on 22 October (closed by DIRECTORY-122), and the foundings store holds both dean@ and annabel@ for that record, choosing between them by map order. Early people have no address held at all. So a deleted account is never remade, a person can only be fixed by hand, and which address a record answers to is chance. An identity system keeps its directory as the source of truth and provisions the issuer from it: the account follows the person, never the other way round.

## Task

Judge every person's sign-in against the issuer and show it on People; repair missing accounts from the directory at start and after every directory change, never for a suspended person or the administrator; let an administrator set a person's address and have Lys make the account; refuse a sign-in Lys cannot tie to one address; and replay 26 October as a test.

## Requirements

### R1: People says, for every active or suspended person, whether their sign-in works and what it needs

THE SYSTEM SHALL judge each active or suspended person's sign-in against the issuer and answer it on the administrator's People read as sign_in, one of: ready (a live, enabled account under the address Lys holds for them); disabled; repairable (no live account, and the address Lys holds is theirs alone and free at the issuer, or held by an account bound to nobody); no_address (no live account and no address held); several_addresses (Lys holds more than one address for them, named); not_theirs (their live account carries another person's address, naming that person when exactly one holds it); address_taken (the account under their address is bound to another person, named); shared_address (another person holds the same address, named). An address is compared without case or surrounding space. The addresses held for a person are their admitted address when one is recorded, otherwise every unwithdrawn founding or admission address for them, one per address. People draws a line in words for every state but ready, with the person's Account link. A person's own People read carries no sign_in.

**Acceptance:**
- Red first (crates/lys-identity-server/src/sign_in_reconcile_tests.rs): one unit test per state, from the facts the judgement reads, including an address held in another case and a second login under an address nobody else holds, which is ready.
- surface/identity people-views: a person whose address is another's sign-in reads the holder's name and has the Account link; a ready person has neither; several addresses are all named.

**Files:**
- create: crates/lys-identity-server/src/sign_in_reconcile.rs
- create: crates/lys-identity-server/src/sign_in_reconcile_tests.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/read_views.rs
- modify: crates/lys-identity-server/src/read_people_emails.rs
- modify: crates/lys-identity-server/src/foundings_store.rs
- modify: surface/identity/src/generated/index.ts
- modify: surface/identity/src/features/people/People.tsx
- modify: surface/identity/src/features/people/people-views.test.tsx

**Checklist:**
- C868 — People says whether each person's sign-in works and what it needs, in words, with their Account link. DIRECTORY-124/R1.

**Stories:**
- S503 (Directory administrator, Resolves a refused act) — As a directory administrator, I want People to say for each person whether their sign-in works and, when it does not, what it needs, so that a person who cannot sign in is seen before they ask.

### R2: Lys finds or makes each person's missing sign-in account under their own address, at start and after every change to the directory

WHEN the service starts, and after an administrator's address change, login move, retirement or reinstatement, THE SYSTEM SHALL repair every active person judged repairable: it adopts the account bound to nobody under their address, or makes one under their address and display name, binds it to them and unbinds their logins whose accounts the issuer no longer has, as one recorded batch per person whose events name the administrator's login as actor; it ends the sessions begun with an unbound login. Before writing it reads the person's logins again and writes nothing when they changed, and it never binds an account any record holds, a retired one included: an administrator moves that on People. Each person is repaired on their own, so one refused repair never stops another's, and every refusal is named on the repair's line. Dead logins are unbound only together with the live one that replaces them. A suspended person, the administrator's own login and every person in any other state are never repaired, and are counted as waiting. A repair that fails says so by name in the service's log and never stops the service starting; the next start or change repairs again. A second repair over the same records changes nothing. Reason the repair runs on events and not on a schedule: Tom's standing rule of 15 October, nothing waits a set time; a start and a directory change are the events after which an account can be missing.

**Acceptance:**
- Red first (crates/lys-identity-server/tests/people_logins_move/reconcile.rs): the issuer's accounts for two people are deleted, People reads both repairable, the service is restarted, and each has exactly one login, a new account under their own address, signs in as themselves and reads ready; a second restart changes no account.
- A suspended person whose account was deleted gets no account at a restart and keeps their login; reinstating them makes their account and replaces the dead login.
- The 22 October move tests read the replacement: a person whose login is moved away gets one new login under their own address and an unbind event in their log.

**Files:**
- create: crates/lys-identity-server/tests/people_logins_move/reconcile.rs
- modify: crates/lys-identity-server/src/sign_in_reconcile.rs
- modify: crates/lys-identity-server/src/routes_startup.rs
- modify: crates/lys-identity-server/src/people_logins_move.rs
- modify: crates/lys-identity-server/src/routes/identities.rs
- modify: crates/lys-identity-server/tests/people_logins_move.rs
- modify: crates/lys-identity-server/tests/people_logins_move/incident.rs
- modify: tests/identity_contract/src/fake_rauthy.rs

**Checklist:**
- C869 — Lys finds or makes each active person's missing sign-in account under their own address at start and after every directory change; a second repair changes nothing. DIRECTORY-124/R2.

**Stories:**
- S504 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in, I want Lys to find or make my sign-in account under my own address whenever it is missing, so that an issuer wiped or an account deleted behind Lys never locks me out or opens someone else's records.

### R3: An administrator sets the address of a person with no sign-in account and Lys makes it; another person's address is refused by name

WHEN the administrator sets the email of a person who has no live account THE SYSTEM SHALL hold that address for them and repair at once, answering the account it made, or missing when it could not; it SHALL refuse with EmailHeldByAnother, naming the person, an address another person holds or whose account at the issuer is bound to another person, and change nothing. The administrator's account read answers missing for a person with no live account, and password reset and enable or disable act on the person's live account only, never a login whose account the issuer no longer has. The Account panel says a missing account in words, offers Set email, and offers neither password reset nor disable for it.

**Acceptance:**
- Red first (reconcile.rs): setting Bea's address on Cal, who has no account, is refused 409 EmailHeldByAnother naming Bea, with the issuer's accounts and the directory log unchanged; setting Dean's own address after the move answers dean@ and makes his account.
- surface/identity tests/account.test.tsx: a missing account reads the words, has no reset or disable form, and Set email posts the trimmed address and reads the made account.

**Files:**
- modify: crates/lys-identity-server/src/accounts.rs
- modify: crates/lys-identity-server/src/sign_in_reconcile.rs
- modify: surface/identity/src/features/people/Account.tsx
- modify: surface/identity/tests/account.test.tsx
- modify: crates/lys-identity-server/tests/people_logins_move/reconcile.rs

**Checklist:**
- C870 — An administrator sets the address of a person with no account and Lys makes it; another person's address is refused by name. DIRECTORY-124/R3.

**Stories:**
- S505 (Directory administrator, Resolves a refused act) — As a directory administrator, I want to set the address of a person who has no sign-in account and have Lys make it, refused by name when that address is another person's, so that I give access to the right person and only them.

### R4: An account Lys cannot tie to exactly one address signs in as nobody

WHEN a sign-in's account opens a person for whom Lys holds more than one address THE SYSTEM SHALL refuse it with SignInNotTheirs, naming the account's address and the held addresses masked, and admit no session; a provider sign-in SHALL be accepted only when its address is the one address held, held at sign-in when none is, and refused with ProviderEmailMismatch naming every held address otherwise. The sign-in page reads SignInNotTheirs in words and tells the person to ask their administrator to set which address is theirs. The administrator's own login is never refused for it, since the administrator is the one who settles it. A person with one held address signs in exactly as before, including after an administrator's deliberate move.

**Acceptance:**
- Red first (reconcile.rs): Dean's record holding dean@ and annabel@ refuses his account's sign-in with SignInNotTheirs and admits no session.
- The provider refusal sends both addresses to the sign-in page, read by the existing sign_in_callback tests' pattern.

**Files:**
- modify: crates/lys-identity-server/src/session_admission.rs
- modify: crates/lys-identity-server/src/error/directory.rs
- modify: crates/lys-identity-server/src/error_names.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/src/provider/refusal.rs
- modify: crates/lys-identity-server/src/sign_in_upstream.rs
- modify: surface/identity/src/features/sign-in/SignIn.tsx

**Checklist:**
- C871 — An account Lys cannot tie to exactly one address signs in as nobody, refused by name. DIRECTORY-124/R4.

**Stories:**
- S506 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in, I want an account Lys cannot tie to exactly one of my addresses to sign in as nobody, so that I never open another person's records by signing in with an account that carries their address.

### R5: Standing bars hold for DIRECTORY-124, with the 26 October records replayed

THE SYSTEM SHALL pass a replay of the 26 October records: Annabel's account deleted at the issuer, Dean's account rewritten to Annabel's address, and a second founding row holding Annabel's address for Dean. People reads Dean several_addresses and Annabel address_taken naming Dean; Dean's account signs in as nobody; one move of that login to Annabel signs her in as Annabel and reads ready; setting Dean's address to dean@ makes his account, and both read ready. Every standing bar holds: fmt, clippy pedantic with warnings denied, ast-grep, the file-length gate, nextest, doc tests, tsc and vitest.

**Acceptance:**
- Red first: the replay test is run before the code and its failing lines kept in the handback.
- The full gate, run as plain `gate`, judged by its parsed output.

**Files:**
- modify: crates/lys-identity-server/tests/people_logins_move/reconcile.rs

**Checklist:**
- C872 — Standing bars hold for DIRECTORY-124, with the 26 October records replayed. DIRECTORY-124/R5.

**Stories:**
- S507 (Directory administrator, Resolves a refused act) — As a directory administrator, I want the 26 October records replayed as a test, so that Annabel signing in as Dean is proven impossible and its repair proven to take one move and one address.

## Boundaries

- SHALL NOT change any live record by hand, script or raw route: Annabel's login is moved, duplicates retired and addresses set by the administrator through People after install.
- SHALL NOT delete any account at the issuer, nor take an account bound to another person.
- SHALL NOT make or adopt an account for a suspended or retired person, nor replace the administrator's own login.
- SHALL NOT add a sleep, a timeout taken as readiness, a poll or a schedule: the repair runs on the service's start and on directory changes.
- SHALL NOT draw a bare button for an act: every act is an Act.

## Verification

- cargo nextest run -p lys-identity-server -E 'binary(people_logins_move) | test(sign_in_reconcile) | binary(accounts) | test(sign_in_callback)' alone, read by its PASS and FAIL lines; vitest on surface/identity for people-views and account, read by its summary line; then the full gate, as plain `gate`.
- Red first: every new test, the replay included, is run before the code and its failing lines kept in the handback.
- Walked on the installed Lys as the administrator: People reads each person's sign-in; Annabel's login is moved to her and she signs in as herself; Dean's address set and he signs in as himself; each person without an account is given their address and their account appears. Each step's screen is read, not assumed.
- Hospital line: If this code were used in a hospital, the worst credible failure is a nurse who signs in and is given another clinician's record, reading and acting on that person's patients under their name, and it could harm about every patient on both clinicians' lists until it is noticed.
