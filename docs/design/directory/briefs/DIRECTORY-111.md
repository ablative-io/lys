---
type: brief
id: DIRECTORY-111
cluster: directory
title: A stranger founds an organisation: the founding act, the proving link, and the first sign-in
---

# DIRECTORY-111: A stranger founds an organisation: the founding act, the proving link, and the first sign-in

> **Cluster:** directory
> **Depends on:** DIRECTORY-109, ACCESS-009, DIRECTORY-110
> **Checklist:**
> - C809 — POST /organisations/found founds both records Registered, writes the proof's digest in one durable write, sends the founded letter with the link, answers one shape to everyone and refuses by name without mail. DIRECTORY-111/R1.
> - C810 — POST /organisations/found/prove makes the account, binds, activates the person and the organisation, issues the founder's administrator grant, records the issuer's letter, signs the founder in and spends the proof, each step under a derived operation; a failed step is answered by name with the steps done and the proof unspent. DIRECTORY-111/R2.
> - C811 — A session begun by address proof is the person signed in everywhere a sign-in through the issuer is; the founding leaves carry the Founding method and claim no login. DIRECTORY-111/R2.
> - C812 — The sign-in page offers founding; #/found and #/found/:proof serve outside the shell, prove only on a click, and land on the account page saying to set a password or connect Google. DIRECTORY-111/R3.
> - C813 — The founded and already-yours letters are asserted word for word at the sink; proof_refused, already_admitted and mail_unconfigured are named on their routes and produced by tests. DIRECTORY-111/R4.
> **Stories:**
> - S460 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a Stranger, I want to found my organisation in Lys by naming it and my address, and to finish by following the letter Lys writes me, so that my people and I have our own place without anyone from Lys being present.
> - S461 (Person who signs in, Keeps their sign-in identities to themselves) — As a Person who signs in, I want the link in my founding letter to sign me in once and lead me to set a password or connect Google, so that I can come back tomorrow without a second letter.

## Purpose

Tom's bar of 20 October 2026 (docs/design/onboarding/ORGANISATION-ALONE.md, 'Tom's bar'): an organisation onboards itself, 'seamless and doable alone, without me and without one of my devices'. The page's walk fails at the first click: a person who is not yet admitted has no route by which to begin (step 1, MISSING). DIRECTORY-109 gave the directory its organisation record and Directory::found_organisation, ACCESS-009 scoped admission and roots to the organisation, and DIRECTORY-110 gave Lys its own sender and the founded letter. This brief joins them into the first act of all: a stranger names an organisation and an address, Lys writes to the address, and following the letter's link proves the address, activates the person and the organisation, issues the founding grant and signs the founder in. Nothing in the path needs our administrator, our devices, or a credential handed around.

## Task

Serve POST /organisations/found and POST /organisations/found/prove on the identity server, unauthenticated, as one two-act path with the idempotence POST /people/admit has (every step under an operation derived from the act's), with the founding pages on the public surface, the two letters asserted at the test-owned sink, and no answer to a stranger that says whether an address is known. Every requirement's red is written and shown red before its code, through the gate on the Mac; the whole brief is written before anything is built.

## Requirements

### R1: A stranger founds: the founding act and its letter

Behavioural. THE SYSTEM SHALL serve POST /organisations/found unauthenticated, with a body of operation, organisation (the organisation's name), display_name (the founder's name) and email, deny_unknown_fields, the email checked as sign_in_refusals::checked_email checks one and the names as Profile::new takes them. When the install has no mail settings the act SHALL be refused mail_unconfigured (503) before any step, as admit is. Otherwise the act SHALL run three steps in order, each under the operation derived from the act's operation and the step's name as people_admit::derived derives one: found, calling Directory::found_organisation with the organisation's and the founder's profiles so both records are Registered and the founder is recorded on the organisation; proof, drawing 32 random bytes from the operating system, encoding them URL-safe base64 without padding as the proving token, and writing the token's lowercase hex SHA-256 to the foundings store with the organisation id, the founder's person id, the email and the founding operation, in one durable write (the whole file written to a sibling, fsynced and renamed over the old, as grant_token_store writes); and letter, sending Letter::Founded through Mailer::send_once under the act's operation with the link '<public origin>/#/found/<token>'. The token SHALL exist in the letter alone: never in the store, the answer, a log, a refusal or a test's output. The answer SHALL be the same shape whether or not the address is known to Lys, carrying the operation and letter: {sent: true, to: <email>} and no identity id, so the route is never an oracle for addresses. When the issuer already holds an account for the email whose subject is bound to an active person, nothing SHALL be founded and the letter sent SHALL be Letter::AlreadyYours, naming the organisation asked for and the address people open Lys at. When the foundings store already holds an unspent founding for the email, the act SHALL found nothing and SHALL re-send Letter::Founded for that founding with a fresh token, the old digest replaced in the same durable write, under the new operation, whatever organisation name was typed this time (one unproven founding per address). Asking again with the same operation SHALL answer the same leaves and send nothing twice. The founding leaves SHALL be recorded under a founding actor whose binding is the install's issuer with the subject 'founding:<operation>' and whose provenance is the new AuthMethod::Founding (no credential; the address unproven at this leaf), so no leaf claims a login that does not exist. The route SHALL be declared in openapi_table.rs as a public route with its refusals and mounted in routes_table.rs beside /people/admit.

**Acceptance:**
- A founding answers 200 with the operation and letter {sent: true, to: <email>} and no member naming an identity; the sink holds one letter to the email with subject 'Found <organisation> in Lys' whose text carries one link under the public origin with the path #/found/<token>; the directory holds the organisation and the person, both Registered, the person recorded as the organisation's founder; the foundings store file holds the token's digest and not the token
- Without [mail] the act is refused 503 mail_unconfigured, the directory's leaf count is unchanged and the foundings store is absent
- The same operation asked twice answers byte-identical bodies, the sink holds one letter and the leaf count is unchanged
- A second founding with the same email under a new operation adds no record, the sink holds a second founded letter with a different link, the first link is refused proof_refused and the second proves
- A founding naming an active person's email answers 200 in the same shape, the sink holds one AlreadyYours letter whose text is asserted word for word, and no record is added
- An email that is not an address, a blank name or an unknown member is refused RequestMalformed before any step, with nothing recorded and nothing sent

**Files:**
- create: crates/lys-identity-server/src/organisations_found.rs
- create: crates/lys-identity-server/src/foundings_store.rs
- create: crates/lys-identity-server/tests/organisations_found.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/src/mail_letters.rs
- modify: crates/lys-identity-server/tests/mail_letters.rs
- modify: crates/lys-identity/src/provenance.rs
- modify: crates/lys-identity/src/event.rs
- modify: crates/lys-identity-server/src/session_store.rs
- modify: crates/lys/src/identity/install/server_config/rendering.rs

**Checklist:**
- C809 — POST /organisations/found founds both records Registered, writes the proof's digest in one durable write, sends the founded letter with the link, answers one shape to everyone and refuses by name without mail. DIRECTORY-111/R1.

**Stories:**
- S460 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a Stranger, I want to found my organisation in Lys by naming it and my address, and to finish by following the letter Lys writes me, so that my people and I have our own place without anyone from Lys being present.

### R2: The proving link activates everything and signs the founder in

Behavioural. THE SYSTEM SHALL serve POST /organisations/found/prove unauthenticated with a body of proof alone. The proof's digest SHALL be looked up in the foundings store; an unknown or spent digest SHALL be refused proof_refused with the one text 'This link is not one Lys issued, or it was already used. Found again from the sign-in page.' Otherwise the act SHALL run these steps in order, each under the operation derived from the founding operation and the step's name: account, accounts::make for the email with the founder's name (the issuer's account found by email before one is made), refused already_admitted when the subject is bound to an active person who is not the founder; bind, Directory::bind_login of the founder to the issuer's subject; activate, Directory::transition Activate on the founder; organisation, Directory::transition Activate on the organisation; grant, organisation_boot::founder_grant for the founder (the administrator grant on the organisation, as first-run setup issues it); letter, when the account step made the account, Mailer::note_sent_by_issuer under the prove operation for the added letter, since the issuer writes to every new account (DIRECTORY-110's leg of 20 October) and that letter is this act's, Lys sending none; session, a session begun through session_admission::begin for the founder's new binding with provenance AuthMethod::AddressProof (new; authenticated_at the proof's instant); and spend, the digest marked spent in one durable write, last. The answer SHALL be what POST /sign-in answers (SignedInView with its Set-Cookie) with organisation and person added. A step that fails SHALL be answered by its own refusal with a receipt naming the steps completed, as admit answers one, the digest left unspent, so asking again with the same proof replays the finished steps and finishes what is left. The prove and found leaves SHALL be recorded under the founding actor of R1 until the bind, and the founder's own binding from the activate step on. A session begun by proof SHALL be the person signed in exactly as a sign-in through the issuer is: the three places that admit a person's act only under AuthMethod::Oidc (grants/administrator.rs:26, drafts_api.rs:287, mcp_oauth_flow.rs:200) SHALL admit AddressProof the same way, the wire forms of the method (event.rs:465 and :482, session_store.rs:152 and :180) SHALL carry both new variants, and provenance.rs:108 and :118 SHALL answer for them as for Oidc. Founding SHALL issue no product grant: the page's line that founding issues a workspace resource under organisation.grant is replaced, because a kind only a product declares cannot be granted before that product connects; the organisation is the workspace (one id), and the door asks Lys which organisation admits the subject (CORE-073's question becomes POST /grants/which for kind organisation, action organisation.read). The page's 'Made by whom and when' paragraph carries that ruling from this brief's commit on.

**Acceptance:**
- found, then the link's token posted to prove, answers 200 with a Set-Cookie, signed_in naming the issuer and the new subject, and the organisation and person ids; the directory shows both Active and the founder bound to the issuer's subject; the grants show the founder holding administrator on the organisation, read as 9846f30c's founder-grant test reads it
- GET /me/account with that cookie answers the founder's email and password false; GET /identities with that cookie lists the founder alone; the configured administrator's GET /identities does not list the founder (ACCESS-009's partition, one assertion here)
- The same proof posted again is refused proof_refused; a proof of thirty-two other bytes is refused proof_refused with the same text; the foundings store shows the digest spent
- With the fake issuer refusing POST /users once, prove answers the account refusal with receipt.done empty; posted again it completes, the directory holds one bind and one activate per identity, and the letters-sent record holds one added letter by the issuer under the prove operation
- Proving after the email was admitted by an administrator in between is refused already_admitted with nothing recorded and the digest unspent
- A session with provenance AddressProof passes grants/administrator.rs:26, drafts_api.rs:287 and mcp_oauth_flow.rs:200 as an Oidc session does, one test each; a stored session with AddressProof reads back from the session store; an event with AuthMethod::Founding round-trips through event.rs

**Files:**
- modify: crates/lys-identity-server/src/organisations_found.rs
- modify: crates/lys-identity-server/src/foundings_store.rs
- modify: crates/lys-identity-server/tests/organisations_found.rs
- modify: crates/lys-identity-server/src/organisation_boot.rs
- modify: crates/lys-identity-server/src/session_admission.rs
- modify: crates/lys-identity-server/src/grants/administrator.rs
- modify: crates/lys-identity-server/src/drafts_api.rs
- modify: crates/lys-identity-server/src/mcp_oauth_flow.rs
- modify: crates/lys-identity-server/src/session_store.rs
- modify: crates/lys-identity/src/provenance.rs
- modify: crates/lys-identity/src/event.rs

**Checklist:**
- C810 — POST /organisations/found/prove makes the account, binds, activates the person and the organisation, issues the founder's administrator grant, records the issuer's letter, signs the founder in and spends the proof, each step under a derived operation; a failed step is answered by name with the steps done and the proof unspent. DIRECTORY-111/R2.
- C811 — A session begun by address proof is the person signed in everywhere a sign-in through the issuer is; the founding leaves carry the Founding method and claim no login. DIRECTORY-111/R2.

**Stories:**
- S460 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a Stranger, I want to found my organisation in Lys by naming it and my address, and to finish by following the letter Lys writes me, so that my people and I have our own place without anyone from Lys being present.
- S461 (Person who signs in, Keeps their sign-in identities to themselves) — As a Person who signs in, I want the link in my founding letter to sign me in once and lead me to set a password or connect Google, so that I can come back tomorrow without a second letter.

### R3: The founding pages on the public surface

Behavioural. The sign-in page (SignIn.tsx) SHALL carry one line under its form, 'New here? Found an organisation.', linking to #/found. THE SYSTEM SHALL serve #/found outside the signed-in shell, as App.tsx:35 serves /sign-in: a form of the organisation's name, the person's name and their email, posted to POST /organisations/found with a fresh operation id; on 200 the page SHALL say 'Lys wrote to <email>. Open the letter and follow its link to prove the address and finish founding <organisation>.' and show no identity; a refusal SHALL be shown in words (mail_unconfigured: 'This Lys cannot send mail yet, so nobody can found an organisation here. Ask whoever runs it.'; RequestMalformed: the reason without its name). THE SYSTEM SHALL serve #/found/:proof outside the shell with one button, 'Finish founding', and SHALL post to POST /organisations/found/prove only on that click, never on load, because mail scanners open links; on 200 the page SHALL navigate to #/settings/account?founded=1, and the account page SHALL read founded from the address and say 'You are signed in by the letter's link. Set a password or connect Google here so you can sign in again.'; proof_refused and already_admitted SHALL be shown in their words with a link to #/sign-in. The generated types (surface/identity/src/generated/index.ts) SHALL carry the two answers as the route table's schemas name them. The pages reuse sign-in.css and recorded-form.css and add no stylesheet.

**Acceptance:**
- Found.test.tsx: the form posts the three fields and an operation id, shows the written-to sentence naming the email and the organisation on 200, shows mail_unconfigured's words on 503 and never renders an id
- Prove.test.tsx: rendering #/found/<token> makes no request; the click posts {proof: <token>} once; on 200 the location becomes #/settings/account?founded=1; proof_refused shows its words and the sign-in link
- Account page test: with founded=1 in the address the sentence is shown; without it, not
- SignIn.test or the existing sign-in test: the founding line and its link are present
- tsc and vitest green through the gate

**Files:**
- create: surface/identity/src/features/found/Found.tsx
- create: surface/identity/src/features/found/Prove.tsx
- create: surface/identity/src/features/found/Found.test.tsx
- create: surface/identity/src/features/found/Prove.test.tsx
- modify: surface/identity/src/App.tsx
- modify: surface/identity/src/features/sign-in/SignIn.tsx
- modify: surface/identity/src/features/me/Account.tsx
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C812 — The sign-in page offers founding; #/found and #/found/:proof serve outside the shell, prove only on a click, and land on the account page saying to set a password or connect Google. DIRECTORY-111/R3.

**Stories:**
- S461 (Person who signs in, Keeps their sign-in identities to themselves) — As a Person who signs in, I want the link in my founding letter to sign me in once and lead me to set a password or connect Google, so that I can come back tomorrow without a second letter.

### R4: The letters, word for word, and the refusals the document names

Behavioural. Letter::Founded SHALL carry the link of R1 and its text SHALL be 'Hello <name>, open <link> to prove this address and found <organisation> in Lys. The link is yours alone; if you did not ask for it, ignore this letter.' Letter::AlreadyYours (new) SHALL be named 'already-yours', subject 'This address already signs in to Lys', text 'Hello, someone asked to found <organisation> in Lys with this address. It already signs in to Lys at <address>, so nothing was founded; sign in there. If this was not you, ignore this letter.' Every letter SHALL be asserted word for word at the sink in tests/mail_letters.rs, once per act, none for a refused act. The refusals proof_refused, already_admitted and mail_unconfigured SHALL be named in openapi_table.rs on their routes and each SHALL be produced by a test, as tests/openapi.rs requires; proof_refused and already_admitted SHALL answer 403 and carry no member naming an identity.

**Acceptance:**
- tests/mail_letters.rs asserts the founded letter with a link and the already-yours letter word for word, one each, at the sink
- tests/openapi.rs's every_refusal_the_document_names_is_produced_by_a_test passes with the three names on their rows
- A refused founding or proof leaves the sink empty and the letters-sent record unchanged

**Files:**
- modify: crates/lys-identity-server/src/mail_letters.rs
- modify: crates/lys-identity-server/tests/mail_letters.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/tests/openapi.rs
- modify: crates/lys-identity-server/src/error.rs

**Checklist:**
- C813 — The founded and already-yours letters are asserted word for word at the sink; proof_refused, already_admitted and mail_unconfigured are named on their routes and produced by tests. DIRECTORY-111/R4.

**Stories:**
- S460 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a Stranger, I want to found my organisation in Lys by naming it and my address, and to finish by following the letter Lys writes me, so that my people and I have our own place without anyone from Lys being present.

## Boundaries

- No answer to an unauthenticated caller says whether an address is known: the founding route answers one shape and the difference is in the letter.
- The proving token lives in the letter alone; the store holds its digest; nothing logs, answers or prints it.
- No expiry and no timer on a proving link (Tom, 15 October: no fixed waits): a link is spent by its use or replaced by a later founding from the same address, never by the clock.
- No second person's confirmation: an organisation of one is valid and the proven address is the confirmation (ruling 5 on the page).
- No product grant is issued by founding; the organisation is the workspace and the door asks Lys which organisation admits the subject (R2's ruling, replacing the page's organisation.grant line).
- One letter per act: founding sends Lys's founded (or already-yours) letter; proof records the issuer's own new-account letter and sends none.
- Our administrator has no part in the path and gains no power inside the organisation (ruling 2).
- The password policy, the account screen and the Google link are the existing ones; this brief adds no second way to set a password.
- No file over 500 lines of code; organisations_found.rs holds the two routes and their steps, foundings_store.rs the store, and nothing else moves into either.

## Verification

- Every red written and shown red at the brief commit through the gate on the Mac, green at the fix commit, named in the END with their seconds; the whole gate (tokei, ast-grep, clippy, tsc, nextest, doc, vitest) at the landing sha.
- The path walked once on the dev install against the fake issuer and the sink: found from the sign-in page, the letter read at the sink, the link followed, the account page reached signed in, a password set, signed out and in again; the walk's record quoted in the END.
- The closing note says plainly that R1 to R4 are done, or names what is not, and carries the hospital line.
