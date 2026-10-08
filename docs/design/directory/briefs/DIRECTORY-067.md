---
type: brief
id: DIRECTORY-067
cluster: directory
title: An approved app signs in through Lys, and a suspended person or a retired app is refused at every step
---

# DIRECTORY-067: An approved app signs in through Lys, and a suspended person or a retired app is refused at every step

> **Cluster:** directory
> **Depends on:** DIRECTORY-048, DIRECTORY-059
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> **Checklist:**
> - C449 — An approved app's client is a provider client (DIRECTORY-067 R1).
> - C450 — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).
> - C451 — A suspended or retired person gets no code, no token and no answer (DIRECTORY-067 R3).
> - C452 — A code presented twice ends the tokens it bought (DIRECTORY-067 R4).
> **Stories:**
> - S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.
> - S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

## Purpose

An administrator approves an app on one screen and the app receives a client id and a secret (apps_api.rs approve, lines 405 to 450, keeps Line::Approved with Client {client_id, secret_sha256}). The OpenID provider never reads that record. OpenIdProvider::open copies only the clients the configuration lists (provider.rs lines 158 to 175, ProviderSettings.clients), and provider.client refuses every other id, so an approved app cannot sign anyone in and a retired app listed in the configuration keeps signing people in. The provider also hands a code to whoever holds a Lys session: authorize resolves the person with person_for (provider.rs lines 322 to 325) and never reads the person's lifecycle state, so a suspended or retired person still receives codes and tokens, and userinfo keeps answering for them (security read SEC-101). A code presented twice is refused, but the tokens it already bought keep working (SEC-103). This brief makes the approved-app record the provider's client list, and refuses a suspended or retired person and a retired app at authorize, token and userinfo.

## Task

Read the provider's clients from the apps' folded record alongside the configured clients, judged fresh on every request. Refuse authorize, token and userinfo for a person whose record is suspended or retired and for an app that is retired or not approved. Revoke the tokens a code bought when that code is presented again. Keep every existing refusal and its name.

## Requirements

### R1: An approved app's client is a provider client

Behavioural. WHEN authorize, token or userinfo names a client id, THE SYSTEM SHALL resolve it against the configured clients and the apps whose standing is approved in the apps' folded record (apps_state.rs App::standing), reading the record at request time so an approval or a retirement takes effect on the next request with no restart. An approved app's redirect addresses are its Registered.redirects and its secret digest is its Approved.client.secret_sha256, compared in constant time exactly as a configured client's is. A client id held both by the configuration and by an approved app refuses at start as client_id_conflict, naming the id, and starts nothing. The app lys is not a sign-in client unless the configuration lists it. Pending, declined and retired apps are unknown clients and refuse with the existing unknown-client refusal. The provider module comment that says products do not yet register themselves is corrected.

**Acceptance:**
- An app approved on the screen completes authorize and token with its issued secret and PKCE.
- An approved app's ID token carries the person's directory id as its subject.
- A redirect address the app did not register is refused RedirectUnregistered.
- A pending app's client id is refused as an unknown client.
- A declined app's client id is refused as an unknown client.
- An approval takes effect on the next request without a restart.
- A configured client still signs in as before.
- A client id held by the configuration and an approved app refuses start as client_id_conflict.
- The client_id_conflict refusal names the id.

**Files:**
- create: crates/lys-identity-server/src/provider_clients.rs
- create: crates/lys-identity-server/src/provider_clients_tests.rs
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/error.rs

**Checklist:**
- C449 — An approved app's client is a provider client (DIRECTORY-067 R1).

**Stories:**
- S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.

### R2: A retired app's codes and tokens stop at once

Behavioural. WHEN an app is retired, THE SYSTEM SHALL refuse its client at authorize, refuse a code it holds at token, and refuse userinfo for every token issued to it, each as client_retired naming the app, judged against the record at the request. Codes and tokens are not trusted for their lifetime once their client is gone. A configured client removed from the configuration is unknown after the restart that reads it.

**Acceptance:**
- Retiring an app refuses its next authorize as client_retired.
- A code issued before the retirement is refused at token as client_retired.
- A token issued before the retirement is refused at userinfo as client_retired.
- The client_retired refusal names the app.

**Files:**
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/provider_clients.rs
- modify: crates/lys-identity-server/src/provider_clients_tests.rs
- modify: crates/lys-identity-server/src/error.rs

**Checklist:**
- C450 — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).

**Stories:**
- S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

### R3: A suspended or retired person gets no code, no token and no answer

Behavioural. WHEN authorize resolves the signed-in person, or token redeems a code, or userinfo reads a token, THE SYSTEM SHALL read that person's record state from the directory at the request and refuse person_inactive when it is suspended or retired, naming the state and the act that answers it (reinstate the person), exactly as admission.rs link_audit_holder already judges the same states. A registered or active person is admitted. A person suspended between the code and the token is refused at token, and between the token and userinfo is refused at userinfo. The refusal never sends the browser to the product's redirect address.

**Acceptance:**
- A suspended person is refused person_inactive at authorize.
- A retired person is refused person_inactive at authorize.
- A person suspended after authorize is refused at token.
- A person suspended after token is refused at userinfo.
- Reinstating the person admits the next authorize.
- A person_inactive refusal redirects the browser nowhere.
- An active person still signs in.

**Files:**
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/provider_tests.rs
- modify: crates/lys-identity-server/src/error.rs

**Checklist:**
- C451 — A suspended or retired person gets no code, no token and no answer (DIRECTORY-067 R3).

**Stories:**
- S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

### R4: A code presented twice ends the tokens it bought

Behavioural. WHEN a code already redeemed is presented again at token, THE SYSTEM SHALL refuse it CodeUsed as today and also end every access token issued from that code, so userinfo refuses them as TokenUnknown. Tokens record the code they came from; nothing else is revoked.

**Acceptance:**
- A second redemption of a code is refused CodeUsed.
- A token bought with that code is refused at userinfo after the second redemption.
- A token bought with a different code keeps working.

**Files:**
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/provider_tests.rs

**Checklist:**
- C452 — A code presented twice ends the tokens it bought (DIRECTORY-067 R4).

**Stories:**
- S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

## Boundaries

- SHALL NOT show the issuer inside Lys, or any address but Lys's own, to a person or a product.
- SHALL NOT store or log a client secret, a code or a token in plain form.
- SHALL NOT cache an approval, a retirement or a person's state beyond the request that read it.
- SHALL NOT change the stored shape of the apps' record or the directory.
- SHALL NOT add a timeout, deadline, watchdog, poll, unsafe, ignored test or lint suppression.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Implementation follows card_build_v3, src_pr and src_land with Jev, fmt, Clippy pedantic, tests, ast-grep and the full gate on Dean.
- A scratch install approves an app on the screen, signs a person in through it, suspends the person and retires the app, and records each refusal.

## Amendments

### Amendment 1: R1 configured clients kept alongside; R2 refusal name; boundary on the stored shape

- **Date:** 2026-10-05
- **By:** Waffles (ruling 15:34, written by Archie)

DIRECTORY-079 R1 replaces the part of R1 that keeps configured clients alongside approved apps with a client_id_conflict refusal: approved apps are the provider's only sign-in clients, nothing is kept alongside, and no conflict refusal exists (Tom: no backwards compatibility; nothing installed needs the configured list). A retired app's refusal is the name apps_binding::sign_in_client already gives, app_retired naming the app, in place of client_retired. The boundary that the apps' record keeps its stored shape is narrowed to its existing lines: DIRECTORY-079 R2 adds the new line kind SignInSet and changes no existing line. R2 (codes and tokens stop at retirement), R3 and R4 stand as written.
