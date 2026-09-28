---
type: brief
id: DIRECTORY-059
cluster: directory
title: Lys finds its own credentials, says where it is, serves its members to any app and signs out everywhere, needing no app to run
---

# DIRECTORY-059: Lys finds its own credentials, says where it is, serves its members to any app and signs out everywhere, needing no app to run

> **Cluster:** directory
> **Depends on:** DIRECTORY-048
> **Checklist:**
> - C411 — A start reads its credentials from Lys's own broker, and no route reaches an app for them (DIRECTORY-059 R1).
> - C412 — Lys writes a discovery record any app can find (DIRECTORY-059 R2).
> - C413 — Any registered app reads Lys's people, seats and agents (DIRECTORY-059 R3).
> - C414 — Signing out of Lys signs the person out at the issuer, and the issuer tells every registered app (DIRECTORY-059 R4).
> - C415 — An app asks for its registration, an administrator approves it on one screen, and the app receives its secret by a one-time code (DIRECTORY-059 R5).
> **Stories:**
> - S165 (Operator, Installs and runs the standalone identity product) — As an operator, I want Lys to start agents with their credentials, tell apps where it is and sign people out of every app, with no app needing to run, so that Lys and each app work on their own and together.

## Purpose

Lys and every app on a machine integrate natively, but neither depends on the other, and each works without the other. Today a Lys start that needs a credential asks another program for it. The start route reads an agent's handle records only through an app's HTTP endpoint (crates/lys-identity-server/src/door_handles.rs lines 1 to 20, wired at crates/lys-identity-server/src/routes.rs lines 168 to 183). No configuration names that endpoint, so the client is built with DoorHandles::unconfigured() (routes.rs line 175) and answers that no door address is configured (door_handles.rs line 106). So every start that needs a credential is refused today, and it could only work with that app running. Lys's own broker already keeps who holds which handle (crates/lys-secrets/src/access.rs and crates/lys-secrets/src/bin/lys-secrets/callers.rs). Beside that, an app cannot find a Lys on the machine, cannot read who Lys's people and agents are, and is not told when a person signs out of Lys.

## Task

Read the handle records from the broker Lys already runs and remove the app endpoint client. Write a discovery record at Lys's data root. Serve people, seats and agents to the registered apps allowed to see them. Have the issuer tell every registered app when a person signs out. Let an app ask for its registration and receive its client secret by a one-time code once an administrator approves it. Nothing here names any app. Overlap. This brief builds after DIRECTORY-048 lands, and routes.rs is shared with 048 R1 and R6 and with DIRECTORY-050 R4. The file door_handles.rs is changed by 048 R7 and deleted here. The file crates/lys/src/identity/install.rs is shared with 050 R1. The files serve.rs and view.rs of lys-secrets are shared with SECRETS-006. The files apps_api.rs and apps_store.rs are created by 048 and changed here. DIRECTORY-045 and DIRECTORY-055 also change crates/lys/src/identity/install.rs, and 055 changes routes.rs, prepare.rs, layout.rs and server_config.rs. Each of those lands first and this brief is cut from the main that holds them.

## Requirements

### R1: A start reads its credentials from Lys's own broker, and no route reaches an app for them

Behavioural. The start route's HandleRecords (crates/lys-identity/src/start/credentials.rs line 59) is read from the secrets broker the service is already configured with (the secrets member of the service configuration, crates/lys-identity-server/src/config.rs lines 67 to 70, rendered by the install at crates/lys/src/identity/install/server_config.rs lines 74 to 75). The broker's existing GET /_lys/handles (crates/lys-secrets/src/bin/lys-secrets/serve.rs line 87, answered by view.rs lines 116 to 146) is not used, because it answers the secret's name, the uses and the limits, and access.rs lines 17 to 20 let only the holder or the person acted for discover a lease. The broker gains GET /_lys/held-states?holder= in serve.rs and view.rs. It answers, for one holder, each handle's id and whether it is active, and no other member. The access rule is new and lives in crates/lys-secrets/src/access.rs. The trusted screen service asks for itself in the signed form it already uses for a person (callers.rs lines 104 to 141, the headers lys-service, lys-on-behalf-of, lys-operation, lys-signed-at and lys-service-signature). It names itself in lys-on-behalf-of and held-states in lys-operation. A caller admitted that way whose identity is a trusted service's own name may read the held states of any holder and nothing else. Every other caller asking this route is refused. The client is broker_handles.rs in lys-identity-server. It refuses an answer carrying any member beside id and state with credential_value_in_answer, as door_handles.rs does today. The function door_handles::credential_id (door_handles.rs line 39) moves unchanged into broker_handles.rs. These are deleted. The file door_handles.rs, tests/door_handles.rs, examples/door_handles.rs, the module lines at routes.rs 84 to 85, and the three assertions at tests/start.rs 437 to 439 that name the old client. The start test is given assertions that name the broker client instead. Lines 168 to 183 of routes.rs build the broker client. A service with no broker configured answers every credentialed start with a new start refusal, broker_not_configured, in crates/lys-identity/src/start/error.rs. The name secrets_unavailable is already the secrets screens' refusal (config.rs lines 66 to 67) and is not reused.

**Acceptance:**
- A test starts an agent that needs a credential against a scratch Lys service and broker with no other program listening, and the start answers the credential's id.
- A test of an agent holding a dropped handle answers that handle as not valid.
- A broker answer carrying a member beside id and state is refused credential_value_in_answer, naming the member and never its value.
- A signed-in person asking GET /_lys/held-states for another holder is refused.
- A service with no broker configured refuses a credentialed start with broker_not_configured.
- A search of crates/ for DoorHandles prints nothing.
- A search of crates/ for door_handles prints nothing.

**Files:**
- create: crates/lys-identity-server/src/broker_handles.rs
- create: crates/lys-identity-server/src/broker_handles_tests.rs
- create: crates/lys-identity-server/tests/start_with_own_broker.rs
- create: crates/lys-secrets/tests/held_states.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/tests/start.rs
- modify: crates/lys-identity/src/start/error.rs
- modify: crates/lys-secrets/src/access.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/callers.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/serve.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/view.rs
- delete: crates/lys-identity-server/src/door_handles.rs
- delete: crates/lys-identity-server/tests/door_handles.rs
- delete: crates/lys-identity-server/examples/door_handles.rs

**Checklist:**
- C411 — A start reads its credentials from Lys's own broker, and no route reaches an app for them (DIRECTORY-059 R1).

**Stories:**
- S165 (Operator, Installs and runs the standalone identity product) — As an operator, I want Lys to start agents with their credentials, tell apps where it is and sign people out of every app, with no app needing to run, so that Lys and each app work on their own and together.

### R2: Lys writes a discovery record any app can find

Behavioural. The install and every upgrade write install/lys.json under the data root (crates/lys/src/identity/install/layout.rs data_root), by a temporary name and a rename. It names the issuer address, the address of the app registration API of DIRECTORY-048, the service address and the record's format version, and nothing secret. The data root is the one path the install already uses, so an app on the same machine finds it by the same platform rule. Removing Lys removes the record. An app that finds no record, or a record whose service does not answer, runs on its own configuration as before.

**Acceptance:**
- After a scratch install, install/lys.json holds exactly the issuer, the registration API, the service address and the format version, and no key, password or secret.
- A test reads the record, asks the service at its address, and gets its authority answer.

**Files:**
- create: crates/lys/src/identity/install/discovery.rs
- create: crates/lys/src/identity/install/discovery_tests.rs
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys/src/identity/install/layout.rs

**Checklist:**
- C412 — Lys writes a discovery record any app can find (DIRECTORY-059 R2).

**Stories:**
- S165 (Operator, Installs and runs the standalone identity product) — As an operator, I want Lys to start agents with their credentials, tell apps where it is and sign people out of every app, with no app needing to run, so that Lys and each app work on their own and together.

### R3: Any registered app reads Lys's people, seats and agents

Behavioural. A registered app of DIRECTORY-048, signed in with its own client, reads GET /apps/members. An app sees a person, seat or agent when that subject holds the member relation on the object app/<app id> in Lys's own schema (the 'lys' app of DIRECTORY-048 R4). The 'lys' schema gains the kind app with the relation member and the action see_members. Approval of an app gives the member relation to every person and agent of the deployment unless the approving administrator narrows it on the approval screen. The feed answers each subject the app sees with the subject id the app's sign-in carries, a display name, the kind, and whether it is active. It pages from a cursor and never reads a whole log. An unregistered client is refused app_not_registered, and a registered app not yet approved is refused app_not_approved, the refusal DIRECTORY-048 R1 names. The feed is the same for every app.

**Acceptance:**
- A test approves two apps and narrows the second to one person, and the first app's feed lists every person and agent.
- The second app's feed lists only that one person.
- Each feed carries the subject ids that app's own sign-in gives.
- An unregistered client is refused app_not_registered.
- A pending app is refused app_not_approved.

**Files:**
- create: crates/lys-identity-server/src/app_members_api.rs
- create: crates/lys-identity-server/tests/app_members.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/apps_api.rs

**Checklist:**
- C413 — Any registered app reads Lys's people, seats and agents (DIRECTORY-059 R3).

**Stories:**
- S165 (Operator, Installs and runs the standalone identity product) — As an operator, I want Lys to start agents with their credentials, tell apps where it is and sign people out of every app, with no app needing to run, so that Lys and each app work on their own and together.

### R4: Signing out of Lys signs the person out at the issuer, and the issuer tells every registered app

Behavioural. The issuer is Rauthy (crates/lys/src/identity/install/server_config.rs lines 29 to 31), and crates/lys-identity-server/src/oidc.rs is only a sign-in client of it, so Lys holds no key to sign a logout token and signs none. The pinned Rauthy source (dd61ac3c, tag v0.36.2) ends one session and sends the OpenID back-channel logout tokens for it itself, signed by its own key, when the browser posts to its end-session address with an id_token_hint (src/service/src/oidc/logout.rs post_logout_handle, line 96, calling execute_backchannel_logout at line 162). Registration of DIRECTORY-048 gains an optional back-channel address, kept in apps_store.rs and set as the backchannel_logout_uri of the app's client at Rauthy when the app is approved, with the Clients rights the service's Rauthy key already holds (crates/lys/src/identity/prepare.rs lines 149 to 158). No new right and no upgrade step is needed. The Lys session record (crates/lys-identity-server/src/session.rs line 34) keeps the ID token of its sign-in, never written to a log. When a person ends the session they are signed in with (end_own, sessions_api.rs line 187, where the ended session is the current one), Lys ends its own session first. Its answer then names Rauthy's end-session address with that session's ID token as the id_token_hint, and the surface sends the browser there. Only the Rauthy session behind that Lys session ends, whichever of the person's logins it was. An administrator ending another person's session (end_persons, line 217) and a person ending one of their other sessions end only the Lys session and send nothing to Rauthy, because there is no browser of that session to send. Lys never waits for Rauthy or for an app, because the browser carries the end-session call. The pinned Rauthy answers its logout page only after every app's send (logout.rs line 339), and it logs a failed send without answering it (lines 336 and 343), so Lys cannot see per-app outcomes and does not claim them. The sign-out answer lists each approved app with no back-channel address under not_told. Lys adds no timeout, retry or clock of its own.

**Acceptance:**
- A test signs a person out and gets an answer naming Rauthy's end-session address with that session's ID token as the id_token_hint.
- A test with two approved apps holding back-channel addresses follows that address against a scratch Rauthy, and each app's stand-in receives a logout token for the person's subject.
- That token verifies under the key Rauthy's discovery document publishes.
- The Lys session has ended before the answer is sent, measured with Rauthy stopped.
- An administrator ending a person's session gets an answer naming no end-session address.
- An approved app with no back-channel address is listed under not_told on the sign-out answer.
- No log line written during the test holds an ID token.

**Files:**
- create: crates/lys-identity-server/tests/backchannel_logout.rs
- modify: crates/lys-identity-server/src/sessions_api.rs
- modify: crates/lys-identity-server/src/session.rs
- modify: crates/lys-identity-server/src/oidc.rs
- modify: crates/lys-identity-server/src/apps_api.rs
- modify: crates/lys-identity-server/src/apps_store.rs
- modify: surface/identity/src/features/sessions/Sessions.tsx

**Checklist:**
- C414 — Signing out of Lys signs the person out at the issuer, and the issuer tells every registered app (DIRECTORY-059 R4).

**Stories:**
- S165 (Operator, Installs and runs the standalone identity product) — As an operator, I want Lys to start agents with their credentials, tell apps where it is and sign people out of every app, with no app needing to run, so that Lys and each app work on their own and together.

### R5: An app asks for its registration, an administrator approves it on one screen, and the app receives its secret by a one-time code

Behavioural. DIRECTORY-048 R1 shows an approved app's client secret once, to the approving administrator. An app on the same machine that registers itself needs the secret delivered to it instead, with no person copying it. The app posts its registration request to POST /apps/connect-requests (the app id, display name, redirect addresses, back-channel address, permission schema, a return address among the redirect addresses, and the SHA-256 hash of a one-time verifier the app keeps). Lys keeps the request in the apps store and answers a request id. A later request for the same app id replaces an earlier one that is still waiting. The app sends the browser to GET /apps/connect?request=<id>, so no schema travels in an address. A signed-in administrator sees the request in words, as the Apps screen shows a registration, and one button registers and approves it together. On this path the approval answer and the screen hold no secret. Lys then sends the browser to the return address with a one-time code bound to that app. POST /apps/{app}/secret takes the code and the verifier. A code for another app is refused code_not_for_app. When the verifier's hash matches, Lys asks Rauthy for a new secret for the app's client at that moment and answers it straight through. So the secret waits nowhere before redemption, and a restart loses nothing but an unredeemed code, which the app replaces with a new request. A code is spent by its first use, whether that use succeeds or fails, and a code never redeemed is spent by the app's next connect request or its retirement. A connect request for an app that is already approved asks to replace its secret. On approval the old secret stops working and the new one is delivered by a code in the same way, so an app that lost its secret before saving it recovers with one approval. The secret is never written to a log, a receipt or a stored record in the clear, as 048 R1 requires. A registration made through the ordinary POST /apps keeps 048's behaviour.

**Acceptance:**
- A test posts a connect request, approves it as the administrator, and redeems the code with the verifier, and the answer holds a secret that completes a sign-in for the app.
- The approval answer on the connect path holds no secret.
- A second redemption of the same code is refused code_spent.
- A redemption with a wrong verifier is refused verifier_mismatch.
- After a wrong verifier, the code is spent.
- A code redeemed under another app's id is refused code_not_for_app.
- A connect request for an approved app, once approved, delivers a new secret, and the old secret no longer completes a sign-in.
- A person who is not an administrator is refused on the connect page.
- No log line written during the test holds the secret.

**Files:**
- create: crates/lys-identity-server/src/apps_connect.rs
- create: crates/lys-identity-server/tests/apps_connect.rs
- create: surface/identity/src/features/apps/ConnectRequest.tsx
- modify: crates/lys-identity-server/src/apps_api.rs
- modify: crates/lys-identity-server/src/apps_store.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C415 — An app asks for its registration, an administrator approves it on one screen, and the app receives its secret by a one-time code (DIRECTORY-059 R5).

**Stories:**
- S165 (Operator, Installs and runs the standalone identity product) — As an operator, I want Lys to start agents with their credentials, tell apps where it is and sign people out of every app, with no app needing to run, so that Lys and each app work on their own and together.

## Boundaries

- SHALL NOT name any app in code, configuration or words, so every app is a reader of these like any other.
- SHALL NOT need any app to be running for a Lys start, sign-in, sign-out or test.
- SHALL NOT put a secret, key or password in the discovery record or the member feed.
- SHALL NOT read a whole log to answer the member feed.
- SHALL NOT add a timeout, deadline, sleep, poll interval, retry by a clock, #[allow], #[ignore] or any bypass.
- SHALL NOT add a silent fallback. Every failure is a named refusal.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- On a scratch install with no other program running, an agent that needs a credential starts.
