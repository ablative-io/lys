---
type: brief
id: DIRECTORY-101
cluster: directory
title: The server answers what waits on a person, whether a kept change was recorded, whether an account has a password, Lys's own address, a refusal that carries what was refused, and an app's address for everyone
---

# DIRECTORY-101: The server answers what waits on a person, whether a kept change was recorded, whether an account has a password, Lys's own address, a refusal that carries what was refused, and an app's address for everyone

> **Cluster:** directory
> **Depends on:** DIRECTORY-099
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> **Checklist:**
> - C757 — GET /waiting answers one oldest-first list of the requests the caller may decide, agent drafts, product drafts and reviews due, with a refused part named in place, never dropped or zero; the rail's count and Home's Waiting on you read it.
> - C758 — GET /operations/{operation} answers recorded true with kind and id for a change the caller made, recorded false for an unknown id, and a refusal for another person's operation; the kept-change check asks it.
> - C759 — GET /me/account answers password and providers; own_password and own_email accept no current password when the account has none and require it when it does; accounts.rs is split under 500 lines.
> - C760 — A refusal's request address is /requests?kind=…&id=…&action=…, percent-encoded, with Refused.request still a string.
> - C761 — /sign-in/providers answers origin, the configured redirect_url's origin, in both its answers.
> - C762 — A person's view of an approved app carries address, the origin of its first sign-in redirect, so Open <app> is a link for everyone who sees the app.
> - C763 — "Not this one" on getting started is kept on the server per person (PUT /me/getting-started, read on /me), with the browser's kept value moved once and removed.
> **Stories:**
> - S428 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I open Lys and see one list of everything waiting on me, oldest first, and when one part cannot be read I see that part's own line, so I decide what is mine without opening four pages.
> - S429 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person who gave access and lost the answer, I ask Lys whether my change was recorded by the receipt it gave me, and I am told yes with what it made, or no, so I never give the same access twice.
> - S430 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with Google and has no password, my account page says so, offers to set a password without asking for one I do not have, and lets me change my email.
> - S431 (Person who signs in, Keeps their sign-in identities to themselves) — As a person Lys refused, the link it gives me opens the Ask form already asking for the thing I was refused, so I do not have to describe it.
> - S432 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who opened Lys at a different address, the sign-in page tells me before I try that Sign in with Google works only at Lys's registered address, and links me there.
> - S433 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who is not an administrator, Open Cambium on my page is a link that opens Cambium.
> - S434 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator who said the Lys computer is not for agents, I am not asked again from another browser.

## Purpose

On 18 October 2026 six surface builders reworked every screen of Lys against the review of that morning (lys-review/00-merged-ranking.md) and landed at ca1228b0. Each wrote what the surface could not finish because the server does not answer it; the list is in lys-review/CONSENSUS.md under Server asks. Two of those gaps are the builders' own worst cases. Domain 03's: a person who gave access to someone else cannot ask Lys whether the give was recorded, so a kept change whose outcome is unknown is checked by inference from a list, and a manager who could not see the grant gave it again and later took back one of two; the surface now says it cannot tell and never suggests giving again (StuckRecord.tsx at ca1228b0), which is a floor, not the fix. Domain 03's other: what waits on a person is four reads with four refusal shapes (requests, agent drafts, product drafts, reviews), so the rail counts two of them and Home lists them one list at a time. Domain 05's: an account that signs in with Google has no password, but GET /me/account answers only email and enabled, so the page cannot say "You sign in with Google", and own_password demands a current password the person does not have, so such a person can change neither email nor password; and /sign-in/providers does not say Lys's own address, so a person at a LAN or tailnet address learns the Google sign-in works only at the registered address after one failed attempt. Domain 03's refusal link: lys-pass sends a refused person to #/requests with nothing about what was refused, while the Ask form already reads kind, id and action from the address. Domain 02's: an approved app's address is served only to the administrator, so "Open Cambium" is a link for one person. Each of these is one route or one field, answered by the server from what it already records.

## Task

Add to lys-identity-server: GET /waiting (one list of everything waiting on the caller), GET /operations/{operation} (whether an operation of the caller's was recorded and what it made), password and providers on GET /me/account with own_password and own_email accepting no current password when the account has none, origin on /sign-in/providers, and address on a person's view of an approved app; and in lys-pass, a refusal's request link that carries kind, id and action. Each route or field is used by the surface in the same lane (Rail and Home read /waiting; StuckRecord asks /operations; Account and AddPerson read password and providers; SignIn already reads origin; CambiumNext reads address). Out of scope, each its own brief: the apps-connect changes (revocation after the exchange, public_address, manifest read server-side, connect-requests and connect_origin), GET /usage/summary, one act for Add and run, and whether assigning a job issues its access (Tom's design question).

## Requirements

### R1: GET /waiting: one read for everything waiting on the caller

Behavioural. GET /waiting answers, for the signed-in caller only, one list of what waits on them: requests they may decide (who may decide is worked out as requests_api.rs's approvers does today, from the grants that stand now), agent drafts and product drafts waiting on them, and grants due a review they hold. Each entry is { kind: "request" | "agent_draft" | "product_draft" | "review", id, asked_by: { id, name }, about: one sentence as the lists say it today, asked_at: RFC 3339, can_decide: true }, oldest first by asked_at. A part the caller may not read is answered as { kind, refusal, reason } in place of its entries, never dropped and never a zero. The route is a row in openapi_table.rs with its answer shape. The rail's count (surface/identity/src/shell/Rail.tsx, useWaiting) and Home's Waiting on you list (the dashboard module whose test is surface/identity/src/features/dashboard/dashboard-waiting.test.tsx) read this one route instead of their own reads, and show a refused part as that part's own line.

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/waiting_api_tests.rs lists, for one caller, one request they may decide, one agent draft, one product draft and one review due, oldest first, and a second caller who may read none of them gets four refused parts and no entries
- The surface's rail count and Home list equal the route's answer in the vitest cases that pin them, and a refused part shows as one line under Waiting on you, not a muted page
- openapi_table.rs carries the route and the contract test that reads it is green

**Files:**
- create: crates/lys-identity-server/src/waiting_api.rs
- create: crates/lys-identity-server/src/waiting_api_tests.rs
- modify: crates/lys-identity-server/src/routes_startup.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: surface/identity/src/shell/Rail.tsx

**Checklist:**
- C757 — GET /waiting answers one oldest-first list of the requests the caller may decide, agent drafts, product drafts and reviews due, with a refused part named in place, never dropped or zero; the rail's count and Home's Waiting on you read it.

**Stories:**
- S428 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I open Lys and see one list of everything waiting on me, oldest first, and when one part cannot be read I see that part's own line, so I decide what is mine without opening four pages.

### R2: GET /operations/{operation}: whether a change of the caller's was recorded

Behavioural. GET /operations/{operation} answers, for the signed-in caller only, whether Lys recorded the operation whose id the caller was handed when it sent a change: { recorded: true, kind: "grant" | "request" | "decision" | "role_change", id } when a record carrying that operation id exists and the caller is the person who made it (the grant's issuer, the request's asker, the decision's decider, the role change's actor), { recorded: false } when no such record exists, and a refusal when the record exists but was made by someone else, so one person can never learn another's operations. The answer is read from the records the server already keeps (grants carry the issuer and the operation id, as delegation.rs records them; requests, decisions and role changes carry theirs); no new store is written. The route is a row in openapi_table.rs. surface/identity/src/features/grants/StuckRecord.tsx asks this route for a kept copy that has an operation id and says the answer in the three-valued words it has at ca1228b0, keeping the cannot-tell words only for a kept copy with no legible operation id.

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/operations_api_tests.rs gives access as one person, asks the route with the receipt's operation id as that person and reads recorded: true with kind grant and the grant's id; asks as the recipient and reads a refusal; asks with an unknown id and reads recorded: false
- StuckRecord's vitest cases read recorded: true as "Lys recorded this" with a link to the record, recorded: false as the not-recorded words with the form reopened and nothing typed lost, and a refusal as the cannot-tell words
- openapi_table.rs carries the route and the contract test is green

**Files:**
- create: crates/lys-identity-server/src/operations_api.rs
- create: crates/lys-identity-server/src/operations_api_tests.rs
- modify: crates/lys-identity-server/src/routes_startup.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: surface/identity/src/features/grants/StuckRecord.tsx

**Checklist:**
- C758 — GET /operations/{operation} answers recorded true with kind and id for a change the caller made, recorded false for an unknown id, and a refusal for another person's operation; the kept-change check asks it.

**Stories:**
- S429 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person who gave access and lost the answer, I ask Lys whether my change was recorded by the receipt it gave me, and I am told yes with what it made, or no, so I never give the same access twice.

### R3: An account says whether it has a password and which providers it signs in with

Behavioural. accounts.rs's shown answer (the account read behind GET /me/account) adds "password": whether the account has a password set, and "providers": the names of the upstream providers bound to it. own_password accepts a body without current when the account has no password (the signed-in session is the only proof there is), and own_email does not require a password for such an account; both keep requiring current when a password exists. No age limit on the session. accounts.rs is 565 lines at ca1228b0; this brief splits the account read and the own-password and own-email handlers into accounts_own.rs so each file is under 500 lines (ADR-111), moving code, not changing it, before adding the fields. The surface: surface/identity/src/features/people/Account.tsx says "You sign in with <provider>." on the Email row when password is false, offers "Set a password" with no Current password field, and Change email asks for no password then; surface/identity/src/features/people/AddPerson.tsx's invitation, when Google is a provider, reads "Hello <name>, you have been added to Lys. Open <origin> and choose Sign in with Google, using <email>."; NoPasswordYet in surface/identity/src/features/settings/Settings.tsx goes.

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/accounts_own_tests.rs reads password false and providers ["google"] for an account bound to Google with no password, sets a password with no current, then reads password true and is refused a second set without current
- An account with a password is refused own_password and own_email without current, as today
- Account.tsx's vitest cases show the three states (password, Google only, both) and Settings.tsx no longer exports NoPasswordYet
- crates/lys-identity-server/src/accounts.rs and accounts_own.rs are each under 500 lines

**Files:**
- create: crates/lys-identity-server/src/accounts_own.rs
- create: crates/lys-identity-server/src/accounts_own_tests.rs
- modify: crates/lys-identity-server/src/accounts.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: surface/identity/src/features/people/Account.tsx
- modify: surface/identity/src/features/people/AddPerson.tsx
- modify: surface/identity/src/features/settings/Settings.tsx

**Checklist:**
- C759 — GET /me/account answers password and providers; own_password and own_email accept no current password when the account has none and require it when it does; accounts.rs is split under 500 lines.

**Stories:**
- S430 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with Google and has no password, my account page says so, offers to set a password without asking for one I do not have, and lets me change my email.

### R4: A refusal's request link carries what was refused

Behavioural. In crates/lys-pass/src/refusal.rs, the fragment set on the refusal's request address (line 72 at ca1228b0, request.set_fragment(Some("/requests"))) becomes /requests?kind=<kind>&id=<id>&action=<action>, each value percent-encoded with url::form_urlencoded::Serializer (append_pair for kind, id and action, prefixed /requests?), built before target moves into needed. Refused.request stays a string; only its value gains a query, so no wire format changes. The surface already reads the three (the Ask form at ca1228b0 opens asking for exactly that); a link missing any of them opens the plain form, so older links keep working.

**Acceptance:**
- Red first: the lys-pass test that reads a refusal's request address expects /requests?kind=…&id=…&action=… with a kind, id and action containing characters that need encoding, decoded back to the originals
- The surface's Ask form, given that address, opens asking for that kind, id and action (the vitest case that pins it at ca1228b0 stays green)

**Files:**
- modify: crates/lys-pass/src/refusal.rs

**Checklist:**
- C760 — A refusal's request address is /requests?kind=…&id=…&action=…, percent-encoded, with Refused.request still a string.

**Stories:**
- S431 (Person who signs in, Keeps their sign-in identities to themselves) — As a person Lys refused, the link it gives me opens the Ask form already asking for the thing I was refused, so I do not have to describe it.

### R5: /sign-in/providers names Lys's own address

Behavioural. crates/lys-identity-server/src/sign_in_help.rs's offered adds "origin": the origin of the configured redirect_url, from state.oidc.public_origin() (oidc.rs:132 at ca1228b0), to both answers (the empty one and the listed one). It is public already, being the address every provider is registered with. openapi_table.rs's /sign-in/providers row gains the field. The surface (surface/identity/src/features/sign-in/SignIn.tsx, useProviders) already reads it when present and, at any other address, replaces the provider buttons with "Sign in with Google works only at <host>. You opened Lys at <here>." and a link to the same page there; no surface change unless its test needs the field named.

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/sign_in_help_tests.rs reads origin equal to the configured redirect_url's origin in both the empty and the listed answer
- The openapi contract test is green with the field

**Files:**
- create: crates/lys-identity-server/src/sign_in_help_tests.rs
- modify: crates/lys-identity-server/src/sign_in_help.rs
- modify: crates/lys-identity-server/src/openapi_table.rs

**Checklist:**
- C761 — /sign-in/providers answers origin, the configured redirect_url's origin, in both its answers.

**Stories:**
- S432 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who opened Lys at a different address, the sign-in page tells me before I try that Sign in with Google works only at Lys's registered address, and links me there.

### R6: A person's view of an approved app carries its address

Behavioural. crates/lys-identity-server/src/apps_read.rs's AppRead::Personal adds address: Option<String>, the origin (scheme, host, port) of the first sign_in.redirects entry, served only for apps the person already sees. surface/identity/src/features/file/CambiumNext.tsx's addressOf reads address first, then its present fallbacks, so "Open Cambium" is a link for everyone the app is shown to, not only the administrator.

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/apps_read_tests.rs reads address for a non-administrator who sees the app, and no app at all for one who does not
- CambiumNext's vitest case shows Open Cambium as a link for a non-administrator

**Files:**
- create: crates/lys-identity-server/src/apps_read_tests.rs
- modify: crates/lys-identity-server/src/apps_read.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: surface/identity/src/features/file/CambiumNext.tsx

**Checklist:**
- C762 — A person's view of an approved app carries address, the origin of its first sign-in redirect, so Open <app> is a link for everyone who sees the app.

**Stories:**
- S433 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who is not an administrator, Open Cambium on my page is a link that opens Cambium.

### R7: Getting started's "Not this one" is kept for the person, not the browser

Behavioural. "Not this one" on Home's getting-started step (surface/identity/src/features/dashboard/GettingStarted.tsx, kept in localStorage under lys.getting-started.not-this-computer.<person> at ca1228b0) is kept on the server: PUT /me/getting-started with { host_declined: true | false } writes a getting_started record for the caller, and GET /me answers it as getting_started: { host_declined }. GettingStarted reads it from /me and writes it with the PUT, and the localStorage key goes; a kept value in localStorage from before this change is written to the server once on first read, then removed, so nobody is asked again.

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/getting_started_tests.rs writes host_declined true as one person and reads it back on /me as that person, and not as another
- GettingStarted's vitest cases read the step hidden when /me says host_declined and shown otherwise, and no localStorage key remains after the one-time move

**Files:**
- create: crates/lys-identity-server/src/getting_started_api.rs
- create: crates/lys-identity-server/src/getting_started_tests.rs
- modify: crates/lys-identity-server/src/routes_startup.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: surface/identity/src/features/dashboard/GettingStarted.tsx

**Checklist:**
- C763 — "Not this one" on getting started is kept on the server per person (PUT /me/getting-started, read on /me), with the browser's kept value moved once and removed.

**Stories:**
- S434 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator who said the Lys computer is not for agents, I am not asked again from another browser.

## Boundaries

- SHALL NOT add a store for operations: GET /operations answers from the records that already carry operation ids.
- SHALL NOT answer another person's waiting list, operation, account fields or app view: every route is for the signed-in caller, refusals named.
- SHALL NOT change the wire shape of Refused: only the request fragment's value changes.
- SHALL NOT put a fixed wait, deadline or poll anywhere, in code or test; a session's age is never a condition.
- SHALL NOT touch the apps-connect flow, usage totals or agent creation; each has its own brief.
- SHALL NOT leave accounts.rs over 500 lines: the split in R3 comes first, as a move with no change.

## Verification

- Red first for every requirement; cargo nextest run only (never cargo test), clippy pedantic -D warnings, cargo fmt; vitest and tsc for the surface files; the design gate's validate.py and check-coverage.py on docs/design/directory parsed for FAIL lines, never by exit code; file length 500 on every file touched.
- Done means: the seven routes and fields answer as specified with the tests named green on the Mac, the surface reads each one, the openapi contract test is green, and the lane's END names any file it touched beyond the lists here so the brief is amended in one commit.
