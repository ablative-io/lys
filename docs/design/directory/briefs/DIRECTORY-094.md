---
type: brief
id: DIRECTORY-094
cluster: directory
title: A refused token step is logged with the client id and the reason
---

# DIRECTORY-094: A refused token step is logged with the client id and the reason

> **Cluster:** directory
> **Depends on:** DIRECTORY-081
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> **Checklist:**
> - C732 — Every refused /oauth/token request writes one warn line naming the client_id presented, the OAuth code, Lys's refusal name and its reason, and never a secret, code, verifier or token (DIRECTORY-094 R1).
> **Stories:**
> - S416 (Operator, Installs and runs the standalone identity product) — As an operator, I want a refused token step logged with the client id and the reason, so that when a product's sign-in is refused I can read why from Lys without asking the product.

## Purpose

On 10 October 2026 the haematite Studio door's sign-in was refused at Lys's token step. It had sent the app's API bearer as its client secret. Nobody could read why from Lys: the token handler (crates/lys-identity-server/src/provider/endpoints.rs `token`, about line 292) answers the refusal in OAuth's words through refusal::oauth_refusal, and writes nothing to the service's log. The reasons exist in Lys's own words: provider/client_auth.rs `authenticated` names `the app holds no live client credential`, `no approved app holds that client id`, `no live client credential of this app holds that secret`, the broker's refusal and SecretsUnavailable. But they reach only the product, which may not show them. Waffles carded it at 15:23: Lys writes one log line, with the client_id and the reason, on a refused token step.

## Task

On every refused token request, write one warn line to the identity server's log naming the client_id presented (from the Basic header or the form; `none` when neither names one), the OAuth error code answered, Lys's name for the refusal, and its reason. Never write the secret, the code, the verifier or the refresh token. Build the line in one function a test can call.

## Requirements

### R1: One log line per refused token request, naming the client and the reason

Behavioural. WHEN /oauth/token answers a refusal, THE SYSTEM SHALL emit exactly one tracing warn event with the fields client_id (the id the request presented, or `none`), error (the OAuth code oauth_refusal answers), refusal (Lys's error name, as error_names gives it) and reason (the refusal's own words), and the message `token request refused`. The event SHALL NOT carry the client secret, the authorization code, the PKCE verifier, the refresh token or any credential value. A token request that succeeds SHALL emit no such event.

**Acceptance:**
- A function in provider/refusal.rs, `refused_fields(client_id: Option<&str>, error: &ServerError)`, answers the four fields. A unit test gives it CredentialRefused{reason: "no live client credential of this app holds that secret"} with client_id Some("haematite"), and gets client_id haematite, error invalid_client, refusal credential_refused and that reason.
- The same test gives ClientUnknown with client_id None and gets client_id `none`, error invalid_client.
- A provider test posts a token request whose client_secret is a non-credential value for an approved app. It captures the server's tracing output with a scoped subscriber, finds exactly one `token request refused` line naming that client_id and invalid_client, and finds that the presented secret appears nowhere in the captured output.
- A provider test of a successful exchange captures no `token request refused` line.

**Files:**
- modify: crates/lys-identity-server/src/provider/endpoints.rs
- modify: crates/lys-identity-server/src/provider/refusal.rs
- modify: crates/lys-identity-server/src/provider_tests.rs
- modify: crates/lys-identity-server/Cargo.toml

**Checklist:**
- C732 — Every refused /oauth/token request writes one warn line naming the client_id presented, the OAuth code, Lys's refusal name and its reason, and never a secret, code, verifier or token (DIRECTORY-094 R1).

**Stories:**
- S416 (Operator, Installs and runs the standalone identity product) — As an operator, I want a refused token step logged with the client id and the reason, so that when a product's sign-in is refused I can read why from Lys without asking the product.

## Boundaries

- The answer to the product is unchanged: the same status, OAuth code and body as today.
- No secret, code, verifier, refresh token or credential value is ever written to the log; the reason strings are Lys's own and carry none.
- A scoped tracing subscriber for the test may be a dev-dependency (tracing-subscriber); it is not a runtime dependency.

## Verification

- cargo clippy -p lys-identity-server --all-targets -- -D warnings and the provider tests; after the next Lys install, one refused curl to /oauth/token with client_id haematite and a wrong secret writes one `token request refused` line naming haematite in the identity log, read back with a token and secret filter.
