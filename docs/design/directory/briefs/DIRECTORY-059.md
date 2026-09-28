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
> - C414 — Signing out of Lys signs the person out of every registered app (DIRECTORY-059 R4).
> **Stories:**
> - S165 (Operator, Installs and runs the standalone identity product) — As an operator, I want Lys to start agents with their credentials, tell apps where it is and sign people out of every app, with no app needing to run, so that Lys and each app work on their own and together.

## Purpose

Lys and every app on a machine integrate natively, but neither depends on the other, and each works without the other. Today a Lys start that needs a credential asks another program for it. The start route reads an agent's handle records only through an app's HTTP endpoint (crates/lys-identity-server/src/door_handles.rs lines 1 to 20, wired at crates/lys-identity-server/src/routes.rs lines 168 to 183). No configuration names that endpoint, so the client is built with DoorHandles::unconfigured() (routes.rs line 175) and answers that no door address is configured (door_handles.rs line 106). So every start that needs a credential is refused today, and it could only work with that app running. Lys's own broker already keeps who holds which handle (crates/lys-secrets/src/access.rs and crates/lys-secrets/src/bin/lys-secrets/callers.rs). Beside that, an app cannot find a Lys on the machine, cannot read who Lys's people and agents are, and is not told when a person signs out of Lys.

## Task

Read the handle records from the broker Lys already runs and remove the app endpoint client. Write a discovery record at Lys's data root. Serve people, seats and agents to any registered app. Send a back-channel sign-out to every registered app when a person signs out. Nothing here names any app.

## Requirements

### R1: A start reads its credentials from Lys's own broker, and no route reaches an app for them

Behavioural. The start route's HandleRecords (crates/lys-identity/src/start/credentials.rs line 59) is read from the secrets broker the service is already configured with (the secrets member of the service configuration, crates/lys-identity-server/src/config.rs lines 67 to 70, rendered by the install at crates/lys/src/identity/install/server_config.rs lines 74 to 75). The broker answers, for one holder, the ids of the handles it holds and whether each is active, from the records it already keeps (crates/lys-secrets/src/access.rs and the holder handling in crates/lys-secrets/src/bin/lys-secrets/callers.rs). The answer carries ids and states only, and an answer with any other member is refused credential_value_in_answer, as door_handles.rs refuses it today. The client that reads it is broker_handles.rs in lys-identity-server, asking as the service the broker trusts. door_handles.rs and its tests are deleted, and routes.rs lines 168 to 183 build the broker client. A service with no broker configured answers every credentialed start refused secrets_unavailable, naming the missing setting.

**Acceptance:**
- A test starts an agent that needs a credential against a scratch Lys service and broker, with no other program listening anywhere, and the start answers the credential's id.
- A test of an agent holding a dropped handle answers that handle as not valid.
- A broker answer carrying a member beside id and state is refused credential_value_in_answer, naming the member and never its value.
- A search of crates/ for DoorHandles and door_handles prints nothing.

**Files:**
- create: crates/lys-identity-server/src/broker_handles.rs
- create: crates/lys-identity-server/src/broker_handles_tests.rs
- create: crates/lys-identity-server/tests/start_with_own_broker.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-secrets/src/access.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/callers.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/main.rs
- delete: crates/lys-identity-server/src/door_handles.rs

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

Behavioural. A registered app of DIRECTORY-048, signed in with its own client, reads GET /apps/members. It answers each person, seat and agent the app is granted to see, with the subject id the app's sign-in carries, a display name, the kind, and whether it is active. It pages from a cursor and never reads a whole log. An app that is not registered, or not approved, is refused by name. The feed is the same for every app.

**Acceptance:**
- A test registers two apps, and each reads the members its grant allows with the subject ids its own sign-in gives.
- An unregistered client is refused app_not_registered.

**Files:**
- create: crates/lys-identity-server/src/app_members_api.rs
- create: crates/lys-identity-server/tests/app_members.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C413 — Any registered app reads Lys's people, seats and agents (DIRECTORY-059 R3).

**Stories:**
- S165 (Operator, Installs and runs the standalone identity product) — As an operator, I want Lys to start agents with their credentials, tell apps where it is and sign people out of every app, with no app needing to run, so that Lys and each app work on their own and together.

### R4: Signing out of Lys signs the person out of every registered app

Behavioural. When a person signs out (crates/lys-identity-server/src/sessions_api.rs line 162), Lys ends the session at its issuer and sends an OpenID back-channel logout token, signed by the issuer, to the back-channel address each registered app declared at registration. Each send is recorded with its answer, and a failed send is named on the person's sign-out answer and never retried by a clock. An app that declared no back-channel address is named as not told.

**Acceptance:**
- A test with two registered apps signs a person out, and each app's stand-in receives a logout token for that person's subject that verifies under the issuer's key.
- An app whose back-channel address refuses is named on the sign-out answer.

**Files:**
- create: crates/lys-identity-server/src/backchannel_logout.rs
- create: crates/lys-identity-server/tests/backchannel_logout.rs
- modify: crates/lys-identity-server/src/sessions_api.rs
- modify: crates/lys-identity-server/src/oidc.rs

**Checklist:**
- C414 — Signing out of Lys signs the person out of every registered app (DIRECTORY-059 R4).

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
