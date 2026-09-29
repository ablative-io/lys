---
type: brief
id: DIRECTORY-068
cluster: directory
title: Signing out of Lys ends every app's access, an app can sign a person out, and an app may read the person's email
---

# DIRECTORY-068: Signing out of Lys ends every app's access, an app can sign a person out, and an app may read the person's email

> **Cluster:** directory
> **Depends on:** DIRECTORY-067
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> **Checklist:**
> - C453 — A code or token dies with the Lys session it came from (DIRECTORY-068 R1).
> - C454 — An app signs the person out through Lys's end-session endpoint (DIRECTORY-068 R2).
> - C455 — An app that asks for the email scope receives the person's email (DIRECTORY-068 R3).
> **Stories:**
> - S181 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app that signs in with Lys, I want to sign a person out through Lys and to read their email when I ask for it, so that my app matches people as it always has.
> - S182 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in, I want signing out of Lys to end every app I signed in to, so that nothing keeps acting as me after I leave.

## Purpose

A person ends a Lys session from their own session list (sessions_api.rs end_session, lines 136 to 170, which removes the entry through Sessions::end in session.rs lines 150 to 168). The provider never learns of it. A code records sign_in_ends_at but not which session it came from (provider.rs Grant), and an access token records only its subject and expiry (provider.rs Access), so every product token lives until the session would have run out even after the person signs out (security read SEC-102). An app has no way to sign a person out of Lys: discovery names no end_session_endpoint (provider.rs discovery, around line 215). And an app learns only the person's directory id: scopes_supported is openid alone and userinfo answers sub alone (provider.rs lines 229 and 498 to 514), while the person's email is held on their Lys account (accounts.rs email_of, line 241). Apps that address people by email, Cambium among them, cannot sign anyone in through Lys without it.

## Task

Record the Lys session each code and token came from, and refuse the code or token once that session has ended. Serve an end-session endpoint in discovery that signs the person out of Lys and returns them only to an address the app registered. Serve the email scope, whose claims are read from the person's Lys account when the token is issued.

## Requirements

### R1: A code or token dies with the Lys session it came from

Behavioural. WHEN token redeems a code, or userinfo reads an access token, THE SYSTEM SHALL refuse session_ended if the Lys session the code was issued under is no longer live, judged against the live sessions at the request. Grant and Access record that session's public id, never its cookie secret. Ending a session any way the product allows (the person's own end, an administrator's end, its expiry) ends its codes and tokens with it. Tokens from another live session of the same person keep working.

**Acceptance:**
- A person who ends their Lys session is refused session_ended at userinfo with a token issued under it.
- A code issued under an ended session is refused session_ended at token.
- A token from the same person's other live session keeps working.
- An administrator ending the person's session ends that session's tokens.
- The recorded session id is the public id, never the cookie secret.

**Files:**
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/provider_tests.rs
- modify: crates/lys-identity-server/src/session.rs
- modify: crates/lys-identity-server/src/error.rs

**Checklist:**
- C453 — A code or token dies with the Lys session it came from (DIRECTORY-068 R1).

**Stories:**
- S182 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in, I want signing out of Lys to end every app I signed in to, so that nothing keeps acting as me after I leave.

### R2: An app signs the person out through Lys's end-session endpoint

Behavioural. WHEN an app sends a person to /oauth/end-session with an id_token_hint, THE SYSTEM SHALL verify the hint's signature against Lys's own key, its issuer, and that its audience is a client Lys serves, and SHALL end the caller's Lys session when its person is the hint's subject, clearing the cookie. A post_logout_redirect_uri is followed only when it exactly matches one of the hinting client's registered redirect addresses, carrying state back unchanged. Otherwise the person lands on Lys's sign-in screen. A hint that fails any check is refused hint_invalid and sends the browser nowhere. A caller signed in as another person is not signed out. Discovery names the endpoint as end_session_endpoint.

**Acceptance:**
- Discovery names end_session_endpoint at Lys's own origin.
- A valid hint ends the caller's Lys session and clears the cookie.
- A registered post_logout_redirect_uri is followed with state unchanged.
- An unregistered post_logout_redirect_uri is not followed.
- A hint signed by another key is refused hint_invalid.
- A hint_invalid refusal redirects the browser nowhere.
- A caller signed in as another person keeps their session.

**Files:**
- create: crates/lys-identity-server/src/provider_end_session.rs
- create: crates/lys-identity-server/src/provider_end_session_tests.rs
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/error.rs

**Checklist:**
- C454 — An app signs the person out through Lys's end-session endpoint (DIRECTORY-068 R2).

**Stories:**
- S181 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app that signs in with Lys, I want to sign a person out through Lys and to read their email when I ask for it, so that my app matches people as it always has.
- S182 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in, I want signing out of Lys to end every app I signed in to, so that nothing keeps acting as me after I leave.

### R3: An app that asks for the email scope receives the person's email

Behavioural. WHEN authorize asks for a scope that includes email, THE SYSTEM SHALL record it on the code, and at token SHALL read the person's email and whether it is verified from their Lys account (accounts.rs email_of), putting email and email_verified in the ID token and in userinfo for that token. A person whose Lys account holds no email gets neither claim, and the app is told nothing else. The email is read when the token is issued, never kept by the provider past the token. An app that did not ask for email receives neither claim. Discovery lists email in scopes_supported and email and email_verified in claims_supported. The issuer inside Lys is never named in any claim.

**Acceptance:**
- Discovery lists the email scope.
- A sign-in asking for email carries the person's email in the ID token.
- The same token's userinfo answers the email.
- email_verified reflects the account.
- A sign-in not asking for email carries no email claim.
- A person with no account email gets no email claim.
- No claim names the issuer inside Lys.

**Files:**
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/provider_tests.rs
- modify: crates/lys-identity-server/src/accounts.rs

**Checklist:**
- C455 — An app that asks for the email scope receives the person's email (DIRECTORY-068 R3).

**Stories:**
- S181 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app that signs in with Lys, I want to sign a person out through Lys and to read their email when I ask for it, so that my app matches people as it always has.

## Boundaries

- SHALL NOT show the issuer inside Lys, or any address but Lys's own, to a person or a product.
- SHALL NOT follow a post-logout address the app did not register.
- SHALL NOT store or log a code, a token, a session secret or an email in the provider's records beyond the token it belongs to.
- SHALL NOT change the stored shape of the directory, the apps' record or the sessions.
- SHALL NOT add a timeout, deadline, watchdog, poll, unsafe, ignored test or lint suppression.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Implementation follows card_build_v3, src_pr and src_land with Jev, fmt, Clippy pedantic, tests, ast-grep and the full gate on Dean.
- A scratch install signs a person in to an approved app with the email scope, signs them out through the app, and records the refusal of the old token.
