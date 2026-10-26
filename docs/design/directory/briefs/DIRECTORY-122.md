---
type: brief
id: DIRECTORY-122
cluster: directory
title: Nobody becomes someone else by opening a link, changing an email or signing in with a provider; a mix-up shows on People and is repaired by one recorded act
---

# DIRECTORY-122: Nobody becomes someone else by opening a link, changing an email or signing in with a provider; a mix-up shows on People and is repaired by one recorded act

> **Cluster:** directory
> **Depends on:** DIRECTORY-112, DIRECTORY-121
> **Checklist:**
> - C850 — A handed link's page names the person and masked email it opens before anything is spent; 'Not me' spends nothing; a browser signed in as another person is refused by name (SignedInAsAnother). DIRECTORY-122/R1.
> - C851 — A person's own email change never marks the address verified and never takes effect until a code mailed to it is returned; with no mail, the change is refused and names the administrator route. DIRECTORY-122/R2.
> - C852 — A provider login begins a session only on an account whose email is verified and equal to the provider's verified email; otherwise the callback refuses by name and no session is made. DIRECTORY-122/R3.
> - C853 — People shows each person's sign-in email and flags, by name, a person whose sign-in email is not the email they were admitted under. DIRECTORY-122/R4.
> - C854 — An administrator moves a sign-in from one person to another person, or to a new person made in the same act, in one signed directory batch with both persons' events naming the actor; refusals by name. DIRECTORY-122/R5.
> - C855 — Standing bars hold for DIRECTORY-122; the incident is a red end to end; no fixed wait; no test proves less than before. DIRECTORY-122/R6.
> **Stories:**
> - S485 (Person who signs in, Keeps their sign-in identities to themselves) — As a person opening a sign-in link, I want to be told whose account it opens before it signs me in, and to stop without using it up if it is not mine, so that I never become someone else by clicking the wrong link.
> - S486 (Person who signs in, Keeps their sign-in identities to themselves) — As a person, I want my account's email to change only once I have proved I hold the new address, so that nobody, me included, can quietly put an address on an account that isn't theirs.
> - S487 (Person who signs in, Keeps their sign-in identities to themselves) — As a person signing in with Google, I want Google to open only the account whose proven email is mine, so that my Google login never joins someone else's record.
> - S488 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator reading People, I want each person's sign-in email beside their name and a mismatch with the email they were added under called out, so that I see a mix-up the day it happens.
> - S489 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want to move a sign-in that landed on the wrong person to the right person in one recorded act, so that a mix-up is repaired in Lys and never by hand.
> - S490 (Directory administrator, Resolves a refused act) — As a directory administrator, I want the 26 October incident replayed as a failing test that passes only with this change, so that it is proven it cannot happen again.

## Purpose

On 24 October 2026 Lys admitted Dean Whiting (person-6c865659a2fceb3bf7c350661f311d7e) and handed his sign-in link. Annabel opened that link by mistake. POST /organisations/found/prove signed whoever held it in as Dean's record (organisations_prove.rs admission_steps: the link is a bearer token and the page, Prove.tsx, says only "This link signs you in to Lys once", never whose it is). On the account page she set the account's email to annabel@ablative.com.au: accounts_own.rs own_email changes the email of an account with no password with no confirmation at all and writes email_verified: true (lines 160-163), so an address nobody proved became the account's verified address. Her Google sign-in then joined that account, because the issuer is configured with auto_link by email (sign_in_providers.rs:563). From then on Annabel signed in as "Dean Whiting" holding Dean's grants, nothing on any screen showed it, and on 26 October an admit of annabel@ (DIRECTORY-121's incident) handed a second link to the same record. Tom, 26 October: "annabel has tried to login, but she accidentally clicked on the link meant for dean the otherday and now its stuck with her as dean, this needs to be much more robust", and "dont just rename things, we have to fix things properly". Three holes compounded: a link that never says whose it is, an email change that proves nothing, and a provider join that trusts it. A fourth made it invisible (no screen shows a person's sign-in email), and a fifth makes it unrepairable in Lys (no act moves a sign-in to the right person). DIRECTORY-121 closes the administrator's side; this brief closes the person's side and gives the repair.

## Task

Make it impossible to become someone else by opening a link, changing an email or signing in with a provider; show a mix-up where administrators read people; and give administrators one recorded act that moves a sign-in to the person it belongs to. Prove all of it by replaying 26 October as a test.

## Requirements

### R1: A handed link says whose account it opens before it is used, and refuses a browser signed in as someone else

THE SYSTEM SHALL answer GET /organisations/found/{proof}/whom, without spending or changing anything, with the display name of the person the link opens and their email masked (first letter and domain, e.g. d…@ablative.com.au), or the same refusal as an unknown or spent proof. The proving page (Prove.tsx) SHALL show "This link is for <name> (<masked email>)" and two acts, "I am <name>, sign me in" and "This isn't me"; "This isn't me" spends nothing and says to ask the person who sent it. WHEN POST /organisations/found/prove is called by a browser that already holds a Lys session for a different person THE SYSTEM SHALL refuse with 409 SignedInAsAnother naming both display names, spend nothing and end no session.

**Acceptance:**
- Red first (crates/lys-identity-server/tests/organisations_handed.rs): the whom read answers the name and masked email and leaves the link unspent; a spent or withdrawn link answers the unknown-proof refusal; a prove carrying Annabel's session cookie on Dean's link answers 409 SignedInAsAnother and the link still proves for Dean afterwards.
- Red first (surface/identity/tests/prove.test.tsx): the page names the person and masked email before any button; This isn't me sends no POST; the SignedInAsAnother refusal is shown in words naming both.

**Files:**
- modify: crates/lys-identity-server/src/organisations_prove.rs
- modify: crates/lys-identity-server/src/people_admit_handed.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/tests/organisations_handed.rs
- modify: surface/identity/src/features/found/Prove.tsx
- modify: surface/identity/tests/prove.test.tsx

**Checklist:**
- C850 — A handed link's page names the person and masked email it opens before anything is spent; 'Not me' spends nothing; a browser signed in as another person is refused by name (SignedInAsAnother). DIRECTORY-122/R1.

**Stories:**
- S485 (Person who signs in, Keeps their sign-in identities to themselves) — As a person opening a sign-in link, I want to be told whose account it opens before it signs me in, and to stop without using it up if it is not mine, so that I never become someone else by clicking the wrong link.

### R2: A person's own email change takes effect only when the new address is proven

WHEN a person asks POST /me/account/email THE SYSTEM SHALL leave the account's email unchanged, mail a single-use code to the new address, and change the email (verified) only when POST /me/account/email/confirm returns that code from the same signed-in person; the code is spent on use and replaced by a newer ask; no write ever sets email_verified true for an address that was not proven this way or typed by an administrator. WHEN Lys has no mail settings THE SYSTEM SHALL refuse the ask with 409 EmailChangeNeedsMail, which names that an administrator changes the email from the person's Account page (POST /directory/people/{id}/account/email). An administrator's change is unchanged. The password confirmation for accounts that have one stays as it is.

**Acceptance:**
- Red first (crates/lys-identity-server/src/accounts_own_tests.rs): with no mail, a password-less account's own email change is refused EmailChangeNeedsMail and the issuer's account is unchanged (email and email_verified); with mail, the ask changes nothing until the mailed code is confirmed, a wrong or spent code is refused by name, and the confirmed change is verified.
- surface/identity/tests/account.test.tsx: the Account page's email change shows the code step with mail and the ask-an-administrator words without it; it never shows the new email as the account's before confirmation.

**Files:**
- modify: crates/lys-identity-server/src/accounts_own.rs
- modify: crates/lys-identity-server/src/accounts_own_tests.rs
- modify: crates/lys-identity-server/src/accounts.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: surface/identity/src/features/people/Account.tsx
- modify: surface/identity/tests/account.test.tsx

**Checklist:**
- C851 — A person's own email change never marks the address verified and never takes effect until a code mailed to it is returned; with no mail, the change is refused and names the administrator route. DIRECTORY-122/R2.

**Stories:**
- S486 (Person who signs in, Keeps their sign-in identities to themselves) — As a person, I want my account's email to change only once I have proved I hold the new address, so that nobody, me included, can quietly put an address on an account that isn't theirs.

### R3: A provider login opens only the account whose proven email is the provider's

WHEN an issuer callback completes for a login made through a provider (Google or any configured upstream) THE SYSTEM SHALL, before beginning any session, read the issuer account and require that its email is verified and equal (case-insensitively, after trimming) to the email the provider asserted as verified for that login; otherwise it SHALL refuse with ProviderEmailMismatch, naming the provider and both addresses masked, begin no session, and record the refusal where sign-in refusals are recorded. The provider join stays as configured; Lys does not trust it alone.

**Acceptance:**
- Red first (crates/lys-identity-server/src/sign_in_callback_tests.rs, against the FakeRauthy the account tests use): a Google login joined to an account whose email was set unverified is refused ProviderEmailMismatch and no session cookie is set; a Google login on an account whose verified email equals Google's verified email signs in as today; a Google login whose asserted email is unverified is refused.

**Files:**
- create: crates/lys-identity-server/src/sign_in_callback_tests.rs
- modify: crates/lys-identity-server/src/sign_in_callback.rs
- modify: crates/lys-identity-server/src/session_admission.rs
- modify: crates/lys-identity-server/src/sign_in_providers.rs

**Checklist:**
- C852 — A provider login begins a session only on an account whose email is verified and equal to the provider's verified email; otherwise the callback refuses by name and no session is made. DIRECTORY-122/R3.

**Stories:**
- S487 (Person who signs in, Keeps their sign-in identities to themselves) — As a person signing in with Google, I want Google to open only the account whose proven email is mine, so that my Google login never joins someone else's record.

### R4: People shows each person's sign-in email and calls out a mismatch

THE SYSTEM SHALL include, for each person in GET /directory/people as an administrator reads it, the email of their sign-in account at this install's issuer (or none) and the email they were admitted or founded under where Lys recorded one; People (People.tsx) SHALL show the sign-in email beside the name and, when the two differ, the words "Signs in as <sign-in email>, added as <admitted email>" with a link to the person's Account page. A non-administrator's read is unchanged and never carries another person's email.

**Acceptance:**
- Red first (crates/lys-identity-server/tests/people_admit.rs or beside read_api.rs): an administrator's list carries sign_in_email and admitted_email; a person whose account email was changed after admission shows both and differs; a non-administrator's list carries neither.
- surface/identity/src/features/people/people-views.test.tsx: the mismatch words and link show for the differing person only.

**Files:**
- modify: crates/lys-identity-server/src/read_api.rs
- modify: crates/lys-identity-server/tests/people_admit.rs
- modify: surface/identity/src/features/people/People.tsx
- modify: surface/identity/src/features/people/people-views.test.tsx

**Checklist:**
- C853 — People shows each person's sign-in email and flags, by name, a person whose sign-in email is not the email they were admitted under. DIRECTORY-122/R4.

**Stories:**
- S488 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator reading People, I want each person's sign-in email beside their name and a mismatch with the email they were added under called out, so that I see a mix-up the day it happens.

### R5: An administrator moves a sign-in to the person it belongs to, in one recorded act

WHEN an administrator of both persons' organisation calls POST /people/{id}/logins/move naming the issuer and subject bound to person {id} and a target that is either an existing person of the same organisation holding no login at that issuer, or a new person to be made with a given display name, THE SYSTEM SHALL in one signed directory batch unbind the login from {id}, make the new person (registered, bound, activated) if asked, and bind the login to the target, each event naming the actor, and SHALL end every session of {id} begun with that login. Grants are not moved: the answer says which grants {id} still holds and the target holds none it did not hold before. Refusals by name: LoginNotBound, TargetHoldsLogin, NotSameOrganisation, and the administrator refusal. Account (PersonSecurity.tsx) SHALL offer "This sign-in belongs to someone else" to administrators beside each sign-in, choosing an existing person or a new name.

**Acceptance:**
- Red first (crates/lys-identity-server/tests/people_logins_move.rs): replay 26 October: admit Dean; prove his link as Annabel's session; set the account email to annabel@ (pre-R2 state built directly in FakeRauthy); then move the login to a new person named Annabel: one batch, both persons' events name the actor, the login signs in as Annabel's record, Dean's record keeps its grants and has no login, and the moved session is ended; each refusal by name; a move across organisations is refused.
- surface/identity/src/features/people/people-views.test.tsx: the move control shows for administrators only and posts the chosen target.

**Files:**
- create: crates/lys-identity-server/src/people_logins_move.rs
- create: crates/lys-identity-server/tests/people_logins_move.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/error/directory.rs
- modify: crates/lys-identity-server/src/error_names.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/src/provider/refusal.rs
- modify: surface/identity/src/features/file/PersonSecurity.tsx
- modify: surface/identity/src/features/people/people-views.test.tsx

**Checklist:**
- C854 — An administrator moves a sign-in from one person to another person, or to a new person made in the same act, in one signed directory batch with both persons' events naming the actor; refusals by name. DIRECTORY-122/R5.

**Stories:**
- S489 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want to move a sign-in that landed on the wrong person to the right person in one recorded act, so that a mix-up is repaired in Lys and never by hand.

### R6: Standing bars hold for DIRECTORY-122, with 26 October replayed end to end

THE SYSTEM SHALL hold every standing bar on the exact final commit: cargo fmt, clippy pedantic -D warnings with all targets, cargo nextest run (never cargo test), cargo test --doc, ast-grep, the file-length gate, tsc and vitest for surface/identity, and scripts/design/gate.sh with the rendered markdown equal to its JSON. One test replays the whole incident through the public routes (admit Dean, Annabel opens his link signed in as herself, tries to change the email, signs in with Google) and passes only when every step is refused or named as R1 to R3 require. No sleep, timeout taken as readiness or poll is added. No test is deleted, weakened or marked serial.

**Acceptance:**
- The replay test (crates/lys-identity-server/tests/people_logins_move.rs, incident module) fails on 0d5c97b5 and passes on the final commit, both runs' lines kept in the handback.
- Every gate leg green on the final commit, each judged by its parsed output lines, never by its exit code.

**Checklist:**
- C855 — Standing bars hold for DIRECTORY-122; the incident is a red end to end; no fixed wait; no test proves less than before. DIRECTORY-122/R6.

**Stories:**
- S490 (Directory administrator, Resolves a refused act) — As a directory administrator, I want the 26 October incident replayed as a failing test that passes only with this change, so that it is proven it cannot happen again.

## Boundaries

- SHALL NOT repair any live person, account or login by hand, script or raw route: the records of 26 October are put right after install through R5's act, by an administrator, recorded.
- SHALL NOT move grants between persons in R5: grants are given and revoked through the existing grant acts, and R5's answer says what each person still holds.
- SHALL NOT mark any address verified that was not typed by an administrator or proven by a returned mailed code.
- SHALL NOT reveal another person's full email to a link holder or a non-administrator: masked on the link page, absent from non-administrator reads.
- SHALL NOT add a sleep, a timeout taken as readiness, or a poll anywhere.

## Verification

- cargo nextest run -p lys-identity-server -E 'binary(organisations_handed) | binary(people_logins_move) | binary(people_admit) | test(accounts_own) | test(sign_in_callback)' alone, read by its PASS and FAIL lines; vitest on surface/identity for prove, account and people-views, read by its summary line; then the full gate.
- Red first: every new test, the replay included, is run before the code and its failing lines kept in the handback.
- Walked on the installed Lys: open a handed link while signed in as another person and read the refusal; open it signed out and read whose it is; press This isn't me and confirm the link still works for its person; try an own email change with no mail and read the administrator words; then, as Tom, move Annabel's sign-in off Dean's record with R5's act and sign in as each to confirm who they are. Each step's screen is read, not assumed.
- Hospital line: If this code were used in a hospital, the worst credible failure is a nurse who opens a colleague's sign-in link by mistake and works the rest of the week as that colleague, every order and every note under someone else's name, and it could harm about every patient either of them touches until an audit notices.
