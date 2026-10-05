---
type: brief
id: DIRECTORY-081
cluster: directory
title: A product signs people in with a virtual client credential an administrator issues; the broker authenticates it and the sealed client secret never leaves the broker
---

# DIRECTORY-081: A product signs people in with a virtual client credential an administrator issues; the broker authenticates it and the sealed client secret never leaves the broker

> **Cluster:** directory
> **Depends on:** DIRECTORY-079
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> **Checklist:**
> - C494 — The token exchange authenticates a lys-client. credential by asking the broker, never falls back to the digest path, and refuses a revoked credential whatever its grant (DIRECTORY-081 R1).
> - C495 — An administrator alone issues, lists and revokes an app's client credentials, shown once and kept as a digest; retirement ends them in the same batch as the retirement; the apps snapshot pins the two new lines (DIRECTORY-081 R2).
> - C496 — The Apps screen issues a credential shown once with a copy control, lists who issued and revoked each, and revokes with confirmation (DIRECTORY-081 R3).
> **Stories:**
> - S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.
> - S266 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want to issue, list and revoke the credential a product signs people in with, with nobody ever seeing the app's secret, so that I can arm a product and disarm it in one place.

## Purpose

Found on the real install on 6 October 2026 (Archie, post 2a923faf; Waffles' ruling 9d17c315): no product can complete a sign-in through Lys, because nobody can obtain an app's client secret. Approval asks the secrets broker to prepare custody (apps_api.rs:400-478, apps_credentials.rs:79-120); the broker makes the secret itself and seals it as lys-app-{owner}-{app}-client (lys-secrets save_app.rs:88-118), and the approval answers only the references and no client (apps_views.rs:162-170). No broker route reveals a sealed value (router.rs:17-41), the screens say a sealed value is "never shown, to anyone" (surface/identity/src/shell/concepts.ts:17), and the token exchange takes the secret only as HTTP Basic or the form's client_secret (provider/endpoints.rs:262-276), judged by its SHA-256 against Approved.client.secret_sha256 (apps_binding.rs:291-305). So the token step has nothing to present, and the install test's token, userinfo and keys steps were taken out until this brief (3d8307e3). Waffles' ruling: nobody is ever shown an app's client secret, at approval or after; what a product holds is a virtual credential made for it, and the sealed value never leaves the broker. A product such as Cambium keeps its client secret in a file and sends it as the form's client_secret to the discovered token endpoint, interpreting no prefix or format (Chippy, d19a52c4: cambium-door session.rs:227-246, oidc.rs:416-424), so a virtual credential presented there needs no product change. The Apps screen still tells the administrator that approving made two secrets held on the page (SaveCredentials.tsx:25), which custody made untrue. S179 is split with DIRECTORY-067 and DIRECTORY-079: they made an approved app a sign-in client; this brief gives its product a credential it can present.

## Task

Give an administrator one act that issues a product a virtual client credential for an approved app, shown once and kept nowhere but as a digest in the broker; make the token exchange authenticate such a credential by asking the broker, which checks it and the app's sealed client secret and answers yes or no, so no secret value leaves the broker; let the administrator list and revoke an app's credentials, and end them all when the app is retired; and put issue, list and revoke on the Apps screen. The product keeps calling Lys's own discovered token endpoint, unchanged; discovery names only Lys's address. Out of scope: refresh tokens as a grant (the provider serves authorization_code only, provider.rs:235), the api credential (lys-app-{owner}-{app}-api) and its proxy route, and any change to a product.

## Requirements

### R1: The token exchange authenticates a virtual client credential through the broker

Behavioural. WHEN a token request presents a client id and a secret that begins lys-client., THE SYSTEM SHALL judge the app with sign_in_redirect's refusals first (app_not_approved, app_retired, credential_refused for an id no approved app holds, redirect_invalid), and then ask the broker at a new route POST /_lys/apps/client, signed by the service as prepare and save are (save_app.rs:41-46), with {app, presented}. The broker SHALL answer {app, credential_id} only when the presented value's digest is a credential issued for that app's client entry and not revoked or ended, and the app's sealed client secret's SHA-256 equals Approved.client.secret_sha256; otherwise it refuses by name, and the provider answers credential_refused naming no secret. A secret without the lys-client. prefix is judged by digest as today; nothing issues such a secret, so it is refused credential_refused. A broker that cannot be reached, or answers anything but a confirmation, is refused SecretsUnavailable by name at the token endpoint and the exchange never falls back to the digest path for a lys-client. value. Client authentication is judged before the grant type, so a request with a revoked credential is refused credential_refused whatever grant it asks for, a refresh_token grant included; a request with a good credential and a grant other than authorization_code is refused as today. The exchange awaits the broker and holds no apps or provider lock across that wait (the rule at apps_api.rs:411-412). Codes and tokens issued before a credential is revoked stand until their own expiry. Every exchange writes one broker audit line: client authenticated for {app} with {credential_id}, or refused {reason} for {app}; never a value.

**Acceptance:**
- On a real install the product exchanges its code with an issued virtual credential sent as the form's client_secret, and as HTTP Basic, and receives an ID token whose issuer is Lys's origin and whose audience is the app; userinfo answers the same subject; the keys answer.
- After the install runs again and restarts, the same credential still exchanges a fresh code.
- A revoked credential is refused credential_refused at the next exchange, with no restart; a token issued before the revocation still answers at userinfo until its expiry.
- A refresh_token request presenting a revoked credential is refused credential_refused, not by its grant type.
- Another app's credential, an unknown credential, a credential with one character changed, and a value without the prefix are each refused credential_refused, and no refusal names a secret or a credential value.
- With the broker unreachable, an exchange with a lys-client. credential is refused SecretsUnavailable by name and no token is issued; a test proves the digest path was not tried.
- A retired app's credential is refused app_retired before the broker is asked.
- Every exchange, allowed or refused, writes one broker audit line naming the app and the credential id or the refusal, and no line holds a value.

**Files:**
- create: crates/lys-secrets/src/bin/lys-secrets/app_client.rs
- create: crates/lys-secrets/src/bin/lys-secrets/app_client_tests.rs
- create: crates/lys-secrets/src/broker/app_client.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/router.rs
- modify: crates/lys-identity-server/src/provider/endpoints.rs
- modify: crates/lys-identity-server/src/apps_binding.rs
- modify: crates/lys-identity-server/src/apps_credentials.rs
- modify: crates/lys-identity-server/tests/provider.rs
- modify: tests/identity_contract/src/app_custody.rs
- modify: crates/lys/tests/identity_install/product.rs

**Checklist:**
- C494 — The token exchange authenticates a lys-client. credential by asking the broker, never falls back to the digest path, and refuses a revoked credential whatever its grant (DIRECTORY-081 R1).

**Stories:**
- S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.

### R2: An administrator issues, lists and revokes an app's client credentials; retirement ends them

Behavioural. WHEN an administrator posts POST /apps/{app}/credentials/issue with an operation id for an approved app whose custody is confirmed, THE SYSTEM SHALL have the broker make a credential lys-client.{app}.{64 hex digits from the secure random source}, keep only its digest bound to the app's sealed client entry and its owner, and answer the value once in that response with its credential id; the service keeps the value nowhere. The act is judged by its own named permission, issue_app_client_credential, held today only through the administrator, by the same authority as approval (apps_api.rs:389 administrator); register_app does not hold it. An app may hold any number of credentials. WHEN an administrator posts POST /apps/{app}/credentials/{id}/revoke, THE SYSTEM SHALL end that credential in the broker and refuse it from the next exchange. The apps log gains two line kinds, ClientCredentialIssued {operation, app, credential_id, by, at} and ClientCredentialRevoked {operation, app, credential_id, reason, by, at}, written only after the broker confirms, and GET /apps/{app} lists the app's credentials with who issued each and when, and who revoked it and when, never a value. WHEN an app is retired, THE SYSTEM SHALL end every credential it holds in the broker and write their revocations in the same durable batch as Line::Retired; a retirement whose credential endings the broker does not confirm is refused SecretsUnavailable and nothing is written. An operation id already used answers the same result and keeps nothing new; one used for another app is refused app_operation_reused. A pending, declined or retired app is refused by its standing's name; a caller without the permission is refused as the other Apps decisions refuse. The apps snapshot pin (crates/lys-identity-server/tests/apps_snapshot.rs and fixtures/apps-state-v1.json) grows with both line kinds in the same piece, and a log written before this brief, without them, opens and folds to the same apps.

**Acceptance:**
- An administrator issues a credential for an approved app; the answer holds the value once; GET /apps/{app} lists it with the issuer and time and no value; no file, log line, audit line or later answer holds the value.
- A second credential for the same app is issued and both exchange; revoking the first leaves the second working.
- A person holding register_app on the app, who is not the administrator, is refused issuing and revoking, and nothing is kept.
- Issuing for a pending, declined or retired app is refused by that standing's name.
- Retiring the app ends every credential it holds in one batch with Line::Retired; each is listed as ended by the retirement; a broker that does not confirm leaves the app approved and refused SecretsUnavailable.
- A repeated operation id answers the same credential id with no value and issues nothing new.
- The snapshot fixture holds both new line kinds and today's writer writes it byte for byte; an apps log without them opens and folds to the same apps.
- The OpenAPI document describes both routes with their requests, responses and refusals, and the route test that walks every route passes.

**Files:**
- modify: crates/lys-identity-server/src/apps_api.rs
- modify: crates/lys-identity-server/src/apps_credentials.rs
- modify: crates/lys-identity-server/src/apps_state.rs
- modify: crates/lys-identity-server/src/apps_views.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/tests/apps_snapshot.rs
- modify: crates/lys-identity-server/tests/fixtures/apps-state-v1.json
- modify: crates/lys-secrets/src/bin/lys-secrets/app_client.rs
- modify: crates/lys-secrets/src/broker/app_client.rs

**Checklist:**
- C495 — An administrator alone issues, lists and revokes an app's client credentials, shown once and kept as a digest; retirement ends them in the same batch as the retirement; the apps snapshot pins the two new lines (DIRECTORY-081 R2).

**Stories:**
- S266 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want to issue, list and revoke the credential a product signs people in with, with nobody ever seeing the app's secret, so that I can arm a product and disarm it in one place.

### R3: The Apps screen issues, lists and revokes an app's client credentials

Behavioural. WHEN an administrator views an approved app on the Apps screen (surface/identity/src/features/apps/Apps.tsx), THE SYSTEM SHALL show its client credentials with who issued each and when, and an act to issue one. Issuing shows the value once, as the connection code is shown once (features/network/JoinCode.tsx), with a copy control and words saying it is shown only now and goes when the page is left; the value is held in that view's state only and is gone on leaving. Revoking asks for confirmation on the page and then lists the credential as revoked with who and when. Every refusal is shown in its words. SaveCredentials.tsx and the approval's "two secrets held on this page" words are removed, since approval hands the administrator no secret; the approval shows that custody is confirmed.

**Acceptance:**
- Vitest: an approved app lists its credentials with issuer and time, and offers issue; a pending, declined or retired app offers no issue.
- Vitest: issuing shows the value once with a copy control; leaving and returning shows it no more.
- Vitest: revoke asks for confirmation; cancelling keeps the credential; confirming lists it revoked with who and when.
- Vitest: each refusal from issue and revoke is shown in its words.
- Vitest: approval shows custody confirmed and no secret, and SaveCredentials is gone.
- Waffles walks the screen with pictures after the install.

**Files:**
- create: surface/identity/src/features/apps/ClientCredentials.tsx
- create: surface/identity/tests/app-client-credentials.test.tsx
- modify: surface/identity/src/features/apps/Apps.tsx
- modify: surface/identity/tests/app-credentials.test.tsx
- delete: surface/identity/src/features/apps/SaveCredentials.tsx

**Checklist:**
- C496 — The Apps screen issues a credential shown once with a copy control, lists who issued and revoked each, and revokes with confirmation (DIRECTORY-081 R3).

**Stories:**
- S266 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want to issue, list and revoke the credential a product signs people in with, with nobody ever seeing the app's secret, so that I can arm a product and disarm it in one place.

## Boundaries

- SHALL NOT show, answer, log, audit, store in plain form or name in a refusal an app's sealed client secret, to anyone, at approval or after; it never leaves the broker.
- SHALL NOT show a virtual client credential's value anywhere but the one issue answer, or keep it anywhere but as a digest in the broker.
- SHALL NOT let the token exchange fall back to the digest path, or admit a client, when the broker cannot confirm a lys-client. credential.
- SHALL NOT name a broker address, or any address but Lys's own, in discovery or to a product; the product calls Lys's token endpoint.
- SHALL NOT let register_app, or anything but the issue permission held through the administrator, issue or revoke a credential.
- SHALL NOT name, call, wait on or read any product from Lys; nothing in the code, the tests' names or the configuration names a product (ADR-116).
- SHALL NOT change the bytes or the meaning of any apps line already written; the two new line kinds are added so old logs read unchanged.
- SHALL NOT hold an apps or provider lock across a wait on the broker.
- SHALL NOT add a timeout, deadline, watchdog, poll, cap, unsafe, ignored test, allow attribute, underscore rename or discarded result.

## Verification

- This handwritten brief passes the design gate (scripts/design/gate.sh), judged by its parsed failures, never by its exit code; checklist.json gains C494 to C496, stories.json gains S266, and the rendered CHECKLIST.md, USER-STORIES.md and brief match what render-cluster.py writes.
- Written whole before anything is built; each crate's own tests run with 0 failed before its push; then the battery: cargo fmt, cargo clippy --workspace --all-targets -- -D warnings, cargo nextest run on lys-secrets, lys-identity-server and lys (the identity_install test with its containers), cargo test --doc, ast-grep, the 500-line file limit, the OpenAPI route test, and tsc and vitest for the surface. Every result is judged by its parsed failures.
- The real-install test (crates/lys/tests/identity_install/product.rs) gets back its token, userinfo and keys steps through an issued credential, before and after the install runs again, then proves the revoked credential and the retired app refused.
- Every handback carries BLOCKED ON, CHANGED, FOUND and NOT CONFIRMED (and where was looked), and the sentence: "If this code were used in a hospital, the worst credible failure is ___, and it could harm about ___ people."
