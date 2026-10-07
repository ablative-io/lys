---
type: brief
id: SECRETS-009
cluster: secrets
title: A person connects a service account from a screen, and the broker exchanges and seals the grant
---

# SECRETS-009: A person connects a service account from a screen, and the broker exchanges and seals the grant

> **Cluster:** secrets
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> **Checklist:**
> - C418 — A provider's consent client is sealed once by the operator and never leaves the broker (SECRETS-009 R1).
> - C419 — A signed-in person starts and finishes consent with a single-use state and PKCE bound to the session (SECRETS-009 R2).
> - C420 — The broker exchanges the code and seals the grant with its provenance, owned by the person (SECRETS-009 R3).
> **Stories:**
> - S168 (Person, Grants an agent provisioned under them access to an account) — As a person, I want to connect my own service account to Lys from a screen, so that my agent can use it through a handle and nobody carries my tokens by hand.

## Purpose

Tom's work watcher (WATCHER-001) reads his three Google accounts through Lys (Waffles, 7 October, b6659ede). An OAuth grant enters Lys today only through `lys-secrets seal-oauth` on standard input (cli.rs 202-215), so somebody has to carry tokens by hand. SECRETS-002 R4 left consent to the door when Cambium was the door; Lys's identity server is that door now, and it has no consent flow. The broker already refreshes and reseals a grant at the proxy (oauth_proxy.rs 1-4), and Provenance already names the subject chosen during service consent (oauth.rs 18-31). One grant per account on the www.googleapis.com route covers Gmail and Calendar: both answer there (Percy, by status code, Waffles 83f79486).

## Task

Seal a provider's consent client once (R1); start and finish consent on the identity server's secrets screen with state and PKCE (R2); exchange the code and seal the grant inside the broker (R3). Out of scope: the screen's look, which is designed with Tom; providers other than through the same four steps; the watcher itself.

## Requirements

### R1: The operator seals a provider's consent client once, and it never leaves the broker

Behavioural. `lys-secrets seal-oauth-client --name <provider>` reads one JSON object on standard input: client_id, client_secret, authorization_endpoint, token_endpoint, revocation_endpoint (optional), redirect_uri and the provider's offered scopes. It seals it as one entry owned by the operator and records the seal. Nothing of it is printed except the name and the client_id. A second seal of the same name is refused; a replacement is the existing replace act. The client's secret is read only by R3's exchange and the existing refresh, inside the broker.

**Acceptance:**
- The sealed client reads back by name; its secret appears in no output, log or answer.
- A JSON object missing a member, or with an unknown one, is refused naming it.

**Files:**
- create: crates/lys-secrets/src/broker/oauth_consent_tests.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/cli.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/main.rs
- modify: crates/lys-secrets/src/broker/oauth_grants.rs

**Checklist:**
- C418 — A provider's consent client is sealed once by the operator and never leaves the broker (SECRETS-009 R1).

**Stories:**
- S168 (Person, Grants an agent provisioned under them access to an account) — As a person, I want to connect my own service account to Lys from a screen, so that my agent can use it through a handle and nobody carries my tokens by hand.

### R2: A signed-in person starts consent on a screen and Lys keeps the state and verifier

Behavioural. The identity server's secrets routes gain GET /secrets/connect/{provider}. It admits only a signed-in person, who names the secret's name and the scopes, chosen from the provider's offered scopes. It makes a state of 32 random bytes and a PKCE S256 verifier, holds both server side bound to that session, and redirects to the provider's authorization endpoint with access_type=offline, prompt=consent and the openid scope added. GET /secrets/connect/{provider}/callback admits only the same session with the same state, once. A state for another session, a used state or a missing one is refused by name. A provider error parameter is refused naming it, and nothing is sealed. The pending consent lives until its callback, a new start by the same session, or the session's end; there is no clock.

**Acceptance:**
- A callback with another session's state, a reused state, or none is refused by name.
- The redirect carries S256 and the state, and never the client secret.

**Files:**
- create: crates/lys-identity-server/src/secrets_consent.rs
- create: crates/lys-identity-server/src/secrets_consent_tests.rs
- modify: crates/lys-identity-server/src/secrets_api.rs

**Checklist:**
- C419 — A signed-in person starts and finishes consent with a single-use state and PKCE bound to the session (SECRETS-009 R2).

**Stories:**
- S168 (Person, Grants an agent provisioned under them access to an account) — As a person, I want to connect my own service account to Lys from a screen, so that my agent can use it through a handle and nobody carries my tokens by hand.

### R3: The broker exchanges the code itself and seals the grant, owned by the person

Behavioural. POST /_lys/oauth/consent admits only a trusted screen service acting for a person, through the existing on-behalf admission. It takes the provider, the code, the verifier and the secret's name. The broker posts the exchange to the sealed client's token endpoint with the client secret and the verifier. The person's id is the grant's owner. Provenance is filled from the answer: provider_subject is the ID token's sub, taken from the token endpoint's direct TLS answer, as OIDC Core 3.1.3.7 allows; scopes are the granted scope list. No refresh token is refused oauth_no_refresh_token. A granted list missing a requested scope is refused oauth_scope_narrowed, naming the missing scopes. In both cases nothing is sealed. A name already sealed is refused, unless it is the same owner reconnecting it by name (reconnect_oauth). The answer carries the name and the provider subject only. The seal and its audit record are one act.

**Acceptance:**
- Against a fixture provider: consent seals a grant whose provenance names the subject and scopes, and a proxied call refreshes it after a reopen.
- No refresh token, or a narrowed scope list, seals nothing and is refused by name.
- No answer, log or audit line carries a token or the client secret.

**Files:**
- create: crates/lys-secrets/src/bin/lys-secrets/consent.rs
- create: crates/lys-secrets/src/bin/lys-secrets/consent_tests.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/router.rs
- modify: crates/lys-secrets/src/broker/oauth_grants.rs

**Checklist:**
- C420 — The broker exchanges the code and seals the grant with its provenance, owned by the person (SECRETS-009 R3).

**Stories:**
- S168 (Person, Grants an agent provisioned under them access to an account) — As a person, I want to connect my own service account to Lys from a screen, so that my agent can use it through a handle and nobody carries my tokens by hand.

## Boundaries

- SHALL NOT send a token, a refresh token or a client secret to the browser, a log, an answer or standard output.
- SHALL NOT seal a grant without a refresh token, or with fewer scopes than were asked.
- SHALL NOT keep the stdin seal-oauth path as a second way for a person's grant once this lands; the operator's client seal of R1 is the one stdin act.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.

## Verification

- The design gate, parsed: every cluster clean, no FAIL, every file valid.
- The full Lys gate on Dean, read from its parsed output: fmt, clippy pedantic in both configurations, ast-grep, nextest and cargo test --doc, 0 failed.
- Each requirement's red at main quoted in the handback.
