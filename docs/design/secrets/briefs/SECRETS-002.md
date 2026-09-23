---
type: brief
id: SECRETS-002
cluster: secrets
title: Build the secrets broker: handles, the proxy, rotation, sealed records, revocation and leases
---

# SECRETS-002: Build the secrets broker: handles, the proxy, rotation, sealed records, revocation and leases

> **Cluster:** secrets
> **Blocked by:** Waffles' review of each row before it is dispatched (CN2), The live permission decision, SpiceDB beside the door, which the statement records as not started (docs/design/identity/STATEMENT-2026-09-22.md:127) and places in step 2 of the road (docs/design/identity/STATEMENT-2026-09-22.md:143); every proxy check asks it (docs/design/identity/STATEMENT-2026-09-22.md:17), A root naming the door repository, which is not set for this round; every door-owned file moves into files when it is, The delegation schema, which the statement leaves unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21); the handle (R1) and the lease's time window (R9) wait on it, The lys-core release that freezes lys/delegation/v1, which waits until the fold that enforces its ordering rule exists (CLAUDE.md:23); until then no delegation is signed outside tests (crates/lys-core/src/delegation/mod.rs:237-242)
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> **Checklist:**
> - C4 — A seat's call with its handle goes through the door's proxy, which checks SpiceDB, swaps the handle for the real credential, forwards the call and writes one audit line naming the seat, the handle, the real account and the time; the credential never leaves the server.
> - C5 — The store keeps a set of real accounts behind one handle, the proxy takes the next in turn and logs which one served, and resting an account is a store change with no file copied to any machine.
> - C6 — The token revolver asks the broker for its next account instead of walking its own list.
> - C7 — The proxy refreshes an OAuth access token itself from the refresh token in the store, and the seat never sees the refresh token.
> - C8 — The door puts the seat's own login into its environment at spawn, read from the store, rotating at spawn, and records which token went to which seat.
> - C9 — Sealed records are kept encrypted in the store, tagged in SpiceDB with which identities may read them, read by name with a relation check and one audit line, and a key is used through the proxy, never read.
> - C10 — Each of the three revocation cases has its own behaviour: a dropped handle refuses every new call, the login token's seat is ended by its engine, and read sealed knowledge is disclosed and recorded, never claimed back.
> - C11 — The proxy has an explicit cancellation rule for calls already admitted when their handle is dropped, written down before it is built.
> - C12 — Everything handed out is a lease counted by uses, a time window and a spend cap: the window in the signed delegation, uses and spend counted by the door, a one-use grant never spent twice.
> - C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
> - C14 — No broker row depends on any one engine: an engine without the broker reads its own pool file as it does today.
> **Stories:**
> - S2 (Person, Grants an agent provisioned under them access to an account) — As a person granting an agent access, I want to give it a handle under my own grant, limited by uses, time and spend, so that it can use the account without ever holding the credential and the grant traces back to me.
> - S3 (Person, Grants an agent provisioned under them access to an account) — As a person, I want to drop an agent's handle and have every new call on it refused at once, so that taking access back does not wait on rotating the real key.
> - S4 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to make my call with my handle and have the proxy swap in the credential, refreshing an expired OAuth token itself, so that I can do my work without ever seeing a credential.
> - S5 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to ask for one of my sealed memories by name and get back only the piece I am permitted, so that a secret never sits in plain text in my memory files.
> - S6 (Token revolver, The worker that runs Claude sessions and builders and turns to the next account on usage-limit words) — As the token revolver, when a session prints its usage-limit words, I want to ask the broker for my next account instead of walking my own list, so that the spread across accounts is the broker's and no pool file is copied to my machine.
> - S7 (Engine, Whichever runtime starts and ends a seat) — As the engine starting a seat, I want the door to give me the seat's own login from the store at spawn and record which token went to which seat, so that I read no pool file when the broker is there and still work from my own when it is not.
> - S8 (Operator, Keeps the accounts and revokes access) — As an operator, I want to rest an account by one change in the store, so that no worker gives it another call and no file is copied to any machine.
> - S9 (Operator, Keeps the accounts and revokes access) — As an operator revoking access, I want each case to say what it reaches: a handle refuses new calls, a call already admitted follows the stated cancellation rule, a login token's seat is ended by its engine, and read knowledge stays read, so that I am never told revocation took back what it cannot.
> - S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.
> - S11 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want each call that was in flight when its handle was dropped to show the outcome the cancellation rule gave it, so that no call's end is unaccounted for.

## Purpose

Agents get a short-lived, limited handle to a credential, never the credential itself (ADR-001), and the token revolver is the first user (ADR-002). This brief is the temporary key model of the statement, one requirement per part, so each row can be reviewed by Waffles and built on its own; it is step 3 of the road: 'Store, handle, proxy, rotation under 1 handle' (docs/design/identity/STATEMENT-2026-09-22.md:144).

## Task

Build the broker the statement's 'Secrets: the temporary key model' describes, in Rust, inside the door (docs/design/identity/STATEMENT-2026-09-22.md:23-57), applying the lys formats 'Where it lives: lys' names (docs/design/identity/STATEMENT-2026-09-22.md:82). The store and the proxy are the door's; the handle's and the sealed record's formats are lys's. In: the handle and the proxy swap with its audit line (R1); rotation under one handle (R2); the token revolver as the first consumer (R3); OAuth refresh at the proxy (R4); the seat's own login at spawn (R5); sealed knowledge tagged in SpiceDB (R6); the three revocation cases (R7); the cancellation rule for calls in flight (R8); leases counted by uses, time window and spend (R9). Out: OpenBao or any external secrets engine; the proxy-handle login path; the delegation schema; anything the statement leaves open, each recorded open in the row it touches. Door-owned files are named in each spec with owner and citation and are not in files until a root naming the door repository is set. Every path is relative to its own repository's root.

## Requirements

### R1: Issue a handle and swap it at the proxy, with one audit line per use

WHEN a seat makes an outbound call carrying its handle, THE SYSTEM SHALL have the door's proxy check SpiceDB, swap the handle for the real credential, forward the call, and write one audit line naming which seat, which handle, which real account and when (docs/design/identity/STATEMENT-2026-09-22.md:27). A handle is a short-lived value bound to the seat's identity; the real credential sits in the door's encrypted store and never leaves the server (docs/design/identity/STATEMENT-2026-09-22.md:27). A handle is issued only under a grant that traces to a person (ADR-003; docs/design/identity/STATEMENT-2026-09-22.md:21). THE SYSTEM SHALL NOT return, log, or put into any error or Debug output a byte of the real credential, and SHALL NOT forward a call whose handle is unknown, dropped, bound to another identity, or whose SpiceDB check is not a permit.

Owners. The store and the proxy are the door's (docs/design/identity/STATEMENT-2026-09-22.md:25, 'We build this ourselves, in Rust, inside the door'; docs/design/identity/STATEMENT-2026-09-22.md:27). The handle's format is lys: 'Credential handover and sealed knowledge are lys sealed envelopes and the delegation format (lys/delegation/v1, a seat as a typed subject). The secrets broker's handle and the sealed record are these, applied' (docs/design/identity/STATEMENT-2026-09-22.md:82). The audit line is a lys log entry, 'not a row in a database' (docs/design/identity/STATEMENT-2026-09-22.md:80), appended through lys-log-store's Log (crates/lys-log-store/src/lib.rs:1-7).

Lys-owned, in files: crates/lys-core/src/delegation/mod.rs (lys; docs/design/identity/STATEMENT-2026-09-22.md:82; the module is the lys/delegation/v1 artifact, crates/lys-core/src/delegation/mod.rs:1-3), whose docs record the handle as an application of the format; docs/design/WIRE-FORMATS.md (lys; its section 1 is the register of frozen contracts, docs/design/WIRE-FORMATS.md:11-21), which records the handle's format before any handle is signed.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the encrypted store of real credentials (docs/design/identity/STATEMENT-2026-09-22.md:27); crates/cambium-door/src/http/secrets_handle.rs, issuing and resolving handles bound to an identity (docs/design/identity/STATEMENT-2026-09-22.md:27); crates/cambium-door/src/http/secrets_proxy.rs, the check, swap, forward and audit line (docs/design/identity/STATEMENT-2026-09-22.md:27). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: (a) the delegation schema, which the statement leaves unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003). (b) Which lys/delegation/v1 pair a handle would use: the only pair v1 defines for a seat is seat with speaks-for, and that role 'has defined semantics, no implementation and no consumer ... Do not invent a consumer for it' (crates/lys-core/src/delegation/artifact.rs:242-254), so whether the handle is that consumer or a new version alongside is Tom's to settle. (c) Delegation is behind unstable-anchor and 'No delegation may be signed outside tests until it is' ratified (crates/lys-core/src/delegation/mod.rs:237-242), and the lys-core release that freezes it waits for the fold (CLAUDE.md:23). (d) Whose key signs a proxy audit line: the statement says each proxy call is 'signed with the agent's key' (docs/design/identity/STATEMENT-2026-09-22.md:80), while the door holds only the seat's public key and never persists the private half (crates/cambium-door/src/http/agent_seat.rs:13-15). (e) The leaf schema of an audit line, which the statement leaves open as 'the event schema shared with the audit lines' (docs/design/identity/STATEMENT-2026-09-22.md:187).

**Acceptance:**
- A test sends one call through the proxy with a live handle to an upstream test double: the double receives exactly 1 request carrying the stored credential, and the seat-visible request and response contain no byte of that credential.
- A test sends one call each with an unknown handle, a dropped handle, and a handle bound to another seat's identity: each is refused, and the upstream test double's request count stays 0 across all 3.
- A test in which SpiceDB answers anything other than a permit for the call's relation: the call is refused and the upstream test double's request count is 0.
- A test forwards 3 calls: exactly 3 audit lines are appended to the lys log, each naming the seat, the handle, the real account and the time, and the log's size grows by exactly 3.
- A redaction test formats the store's credential type, the proxy's error type and the audit line with Debug and Display: none of the outputs contains a byte of the credential.
- A test asks the door to issue a handle for an agent whose grant does not trace to a person: no handle is issued.
- docs/design/WIRE-FORMATS.md and crates/lys-core/src/delegation/mod.rs name the handle's format before any handle is signed outside tests, and the points recorded open above are still recorded open, not decided, when the row is reviewed.

**Files:**
- modify: crates/lys-core/src/delegation/mod.rs
- modify: docs/design/WIRE-FORMATS.md

**Checklist:**
- C4 — A seat's call with its handle goes through the door's proxy, which checks SpiceDB, swaps the handle for the real credential, forwards the call and writes one audit line naming the seat, the handle, the real account and the time; the credential never leaves the server.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.

**Stories:**
- S2 (Person, Grants an agent provisioned under them access to an account) — As a person granting an agent access, I want to give it a handle under my own grant, limited by uses, time and spend, so that it can use the account without ever holding the credential and the grant traces back to me.
- S4 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to make my call with my handle and have the proxy swap in the credential, refreshing an expired OAuth token itself, so that I can do my work without ever seeing a credential.
- S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.

### R2: Rotate across a set of real accounts under one handle

WHEN a call arrives on a handle whose store entry keeps a set of real accounts, THE SYSTEM SHALL have the proxy take the next one in turn for that call and log which one served it (docs/design/identity/STATEMENT-2026-09-22.md:33). The handle is the stable name (docs/design/identity/STATEMENT-2026-09-22.md:33). Resting an account SHALL be a change in the store with no file copied to any machine, and this SHALL replace the account pool file (docs/design/identity/STATEMENT-2026-09-22.md:33; ADR-002). THE SYSTEM SHALL NOT give a rested account a call, and SHALL NOT trust the spread across accounts to each worker: it is enforced at the proxy (docs/design/identity/STATEMENT-2026-09-22.md:33).

Owner: the door, which keeps the store and the proxy (docs/design/identity/STATEMENT-2026-09-22.md:25-27, docs/design/identity/STATEMENT-2026-09-22.md:33). No lys-owned file changes for rotation; the account that served each call is a field of the R1 audit line.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the set of real accounts behind a handle and the rested mark (docs/design/identity/STATEMENT-2026-09-22.md:33); crates/cambium-door/src/http/secrets_rotation.rs, taking the next account in turn (docs/design/identity/STATEMENT-2026-09-22.md:33). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

Retiring the pool file waits until its consumers have migrated (docs/design/identity/STATEMENT-2026-09-22.md:167), and an engine without the broker reads its own pool file as it does today (docs/design/identity/STATEMENT-2026-09-22.md:62; ADR-004).

**Acceptance:**
- A test with 3 accounts behind one handle sends 6 calls: each account serves exactly 2, in turn, and each of the 6 audit lines names the account that served it.
- A test rests 1 of the 3 accounts in the store and sends 4 calls: the rested account serves 0 of them and the other 2 serve 2 each.
- Resting an account is a single store change: the test performs no file write outside the store and touches no pool file.
- A test with every account behind a handle rested sends 1 call: it is refused and the upstream test double's request count is 0.

**Checklist:**
- C5 — The store keeps a set of real accounts behind one handle, the proxy takes the next in turn and logs which one served, and resting an account is a store change with no file copied to any machine.

**Stories:**
- S8 (Operator, Keeps the accounts and revokes access) — As an operator, I want to rest an account by one change in the store, so that no worker gives it another call and no file is copied to any machine.
- S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.

### R3: Serve the token revolver its next account from the broker

WHEN a worker that runs Claude sessions or builders sees a session print its usage-limit words, THE SYSTEM SHALL let it ask the broker for its next account instead of walking its own ordered list (docs/design/identity/STATEMENT-2026-09-22.md:35; ADR-002). The revolver is the first consumer of the handle (docs/design/identity/STATEMENT-2026-09-22.md:35). The broker's answer SHALL come from the same rotation as R2, and SHALL be attributed to the asking seat. THE SYSTEM SHALL NOT make any engine depend on the broker: an engine without it reads its own pool file as it does today (docs/design/identity/STATEMENT-2026-09-22.md:62; ADR-004), and manifold is never a structural part (docs/design/identity/STATEMENT-2026-09-22.md:61).

Owners. The broker's side is the door's (docs/design/identity/STATEMENT-2026-09-22.md:25-27). The worker's side is the engine's: the revolver read for this brief is manifold's (a launcher arms it, crates/manifold-node/src/seat/launcher.rs:31-40 in the manifold repository read at 3df5ac5f64, and it turns through the operator's pool <data-dir>/supervisor/accounts.json, docs/seat-document.md:569 in that repository), but the statement names no engine as the consumer and the design inventory names no engine repository.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-door/src/http/secrets_next_account.rs, the call a worker makes for its next account (docs/design/identity/STATEMENT-2026-09-22.md:35). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: the owner of the worker-side file. The sources do not settle which engine repository takes the first change, and no engine file is named until they do. Also open with R5: whether the answer is the account's login token at spawn (R5) or a handle through the proxy, which stays unproved (docs/design/identity/STATEMENT-2026-09-22.md:45).

**Acceptance:**
- A test asks for the next account 3 times for one seat against a handle with 2 accounts: the answers alternate between the 2 accounts, and each ask appends exactly 1 audit line naming the seat.
- A test asks for the next account when every account behind the handle is rested: the ask is refused and names no account.
- The door's tests for this call start no engine process and depend on no engine crate: the call is exercised with a test client alone.
- The worker-side owner is recorded open in this requirement when the row is reviewed, and no engine file is named.

**Checklist:**
- C6 — The token revolver asks the broker for its next account instead of walking its own list.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C14 — No broker row depends on any one engine: an engine without the broker reads its own pool file as it does today.

**Stories:**
- S6 (Token revolver, The worker that runs Claude sessions and builders and turns to the next account on usage-limit words) — As the token revolver, when a session prints its usage-limit words, I want to ask the broker for my next account instead of walking my own list, so that the spread across accounts is the broker's and no pool file is copied to my machine.

### R4: Refresh OAuth at the proxy; the seat never sees the refresh token

WHEN a call arrives on a handle whose credential is an OAuth grant, THE SYSTEM SHALL have the proxy swap the handle for a live access token, and refresh it itself when it expires (docs/design/identity/STATEMENT-2026-09-22.md:39). The refresh token sits in the store (docs/design/identity/STATEMENT-2026-09-22.md:39). THE SYSTEM SHALL NOT let the seat see the refresh token or the access token (docs/design/identity/STATEMENT-2026-09-22.md:39). Revoking SHALL drop the handle and MAY also revoke the grant upstream (docs/design/identity/STATEMENT-2026-09-22.md:39).

Owner: the door (docs/design/identity/STATEMENT-2026-09-22.md:25-27, docs/design/identity/STATEMENT-2026-09-22.md:39). No lys-owned file changes for refresh.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, holding the refresh token (docs/design/identity/STATEMENT-2026-09-22.md:39); crates/cambium-door/src/http/secrets_oauth.rs, the refresh at the proxy (docs/design/identity/STATEMENT-2026-09-22.md:39). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

**Acceptance:**
- A test with an expired access token sends 1 call: the proxy makes exactly 1 refresh request to the provider test double, forwards the call with the new access token, and the seat-visible response contains neither the refresh token nor either access token.
- A test with a live access token sends 2 calls: the provider test double's refresh request count is 0.
- A test drops an OAuth handle: the next call is refused, and when upstream revocation is asked for, the provider test double receives exactly 1 revocation request.
- A redaction test formats the store's OAuth grant type with Debug: the output contains neither token.

**Checklist:**
- C7 — The proxy refreshes an OAuth access token itself from the refresh token in the store, and the seat never sees the refresh token.

**Stories:**
- S4 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to make my call with my handle and have the proxy swap in the credential, refreshing an expired OAuth token itself, so that I can do my work without ever seeing a credential.

### R5: Put the seat's own login into its environment at spawn

WHEN the engine that runs a seat starts it, THE SYSTEM SHALL have the door put the long-lived OAuth token Claude Code generates into the seat's environment, read from the store (docs/design/identity/STATEMENT-2026-09-22.md:43). This is the one place the credential reaches the process (docs/design/identity/STATEMENT-2026-09-22.md:43, docs/design/identity/STATEMENT-2026-09-22.md:167). It rotates at spawn, not per call (docs/design/identity/STATEMENT-2026-09-22.md:43). THE SYSTEM SHALL record which token went to which seat (docs/design/identity/STATEMENT-2026-09-22.md:56). THE SYSTEM SHALL NOT name any one engine: the engine that starts or ends a seat is whichever one runs the agent (docs/design/identity/STATEMENT-2026-09-22.md:61; ADR-004), and an engine without the broker reads its own pool file as it does today (docs/design/identity/STATEMENT-2026-09-22.md:62).

Owner: the door, which holds the store (docs/design/identity/STATEMENT-2026-09-22.md:43). No lys-owned file changes for the login at spawn.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the login tokens and the record of which went to which seat (docs/design/identity/STATEMENT-2026-09-22.md:43, docs/design/identity/STATEMENT-2026-09-22.md:56); crates/cambium-door/src/http/secrets_spawn_login.rs, the answer an engine receives at spawn (docs/design/identity/STATEMENT-2026-09-22.md:43). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: (a) the proxy-handle login path, where the seat holds a handle in the token variable and its base URL points at the proxy; it 'is not tested with a subscription login and nothing depends on it' (docs/design/identity/STATEMENT-2026-09-22.md:45), and this row builds nothing that depends on it. (b) Whether revoking the login token at the provider makes the seat's next call fail; it 'is proved with the provider before it is promised' (docs/design/identity/STATEMENT-2026-09-22.md:56), and this row promises nothing about it.

**Acceptance:**
- A test spawns 2 seats against a login set of 2 accounts: each spawn answer carries the login for exactly 1 account, the 2 answers differ, and the store's record names which account's token went to which seat.
- A test sends 3 calls from one spawned seat: the seat's login does not change between them (rotation is at spawn only).
- The spawn call is exercised by a test client with no engine crate in its dependencies.
- The proxy-handle login path and the provider-revocation question are recorded open in this requirement when the row is reviewed, and no acceptance criterion of this brief depends on either.

**Checklist:**
- C8 — The door puts the seat's own login into its environment at spawn, read from the store, rotating at spawn, and records which token went to which seat.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C14 — No broker row depends on any one engine: an engine without the broker reads its own pool file as it does today.

**Stories:**
- S7 (Engine, Whichever runtime starts and ends a seat) — As the engine starting a seat, I want the door to give me the seat's own login from the store at spawn and record which token went to which seat, so that I read no pool file when the broker is there and still work from my own when it is not.

### R6: Keep sealed knowledge in the store, tagged in SpiceDB

THE SYSTEM SHALL keep keys that cannot be rotated, and memories an identity wants kept secret, as sealed records in the same store, encrypted, each tagged in SpiceDB with which identities may read it (docs/design/identity/STATEMENT-2026-09-22.md:49). WHEN a seat asks for a sealed record by name, THE SYSTEM SHALL check the relation, return the text, and write one audit line (docs/design/identity/STATEMENT-2026-09-22.md:49). Each identity's sealed records are its own (docs/design/identity/STATEMENT-2026-09-22.md:49). THE SYSTEM SHALL NOT let a sealed record sit in plain text in a memory file (docs/design/identity/STATEMENT-2026-09-22.md:49), and SHALL NOT return a key by reading it: a key is used through the proxy even when it cannot rotate; only memories are read, in the smallest piece asked for (docs/design/identity/STATEMENT-2026-09-22.md:57; ADR-001).

Owners. The store, the SpiceDB tag and the read are the door's (docs/design/identity/STATEMENT-2026-09-22.md:49). The sealed record's format is lys sealed envelopes, applied (docs/design/identity/STATEMENT-2026-09-22.md:82).

Lys-owned, in files: crates/lys-core/src/seal/mod.rs (lys; docs/design/identity/STATEMENT-2026-09-22.md:82; the module is the sealed envelope, crates/lys-core/src/seal/mod.rs:1-10), whose docs record the sealed record as an application of it; docs/design/WIRE-FORMATS.md (lys; the lys/sealed-envelope/v1 row, docs/design/WIRE-FORMATS.md:18), which records the application.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the sealed records (docs/design/identity/STATEMENT-2026-09-22.md:49); crates/cambium-door/src/http/secrets_sealed.rs, the read by name with its relation check and audit line (docs/design/identity/STATEMENT-2026-09-22.md:49). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: lys/sealed-envelope/v1 seals with an empty AAD (docs/design/WIRE-FORMATS.md:18; crates/lys-core/src/seal/sealed_envelope.rs:180-183), so an envelope carries nothing that binds it to its name or its owning identity. Whether 'each identity's sealed records are its own' is held by a construction alongside v1 or by the door is not settled by the statement; any construction is a new version alongside, never a change to the shipped one, and takes an adversarial review (CLAUDE.md, coding standards).

**Acceptance:**
- A test in which SpiceDB permits the asking identity reads one memory record by name: the text is returned and exactly 1 audit line is appended naming the seat, the record and the time.
- A test in which SpiceDB does not permit the asking identity: the read is refused, no text is returned, and the record is not decrypted.
- A test asks to read a record marked as a key: the read is refused, and the key is usable only through the proxy of R1.
- A test moves one identity's sealed record to another identity's name in the store and reads it as the second identity: the read returns no text.
- A test searches every memory file the door writes for a record's plaintext after sealing it: 0 matches.

**Files:**
- modify: crates/lys-core/src/seal/mod.rs
- modify: docs/design/WIRE-FORMATS.md

**Checklist:**
- C9 — Sealed records are kept encrypted in the store, tagged in SpiceDB with which identities may read them, read by name with a relation check and one audit line, and a key is used through the proxy, never read.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.

**Stories:**
- S5 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to ask for one of my sealed memories by name and get back only the piece I am permitted, so that a secret never sits in plain text in my memory files.
- S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.

### R7: Revoke in the three cases the statement names

THE SYSTEM SHALL revoke in the 3 cases the statement splits (docs/design/identity/STATEMENT-2026-09-22.md:53-57), each with its own story. (1) A handle: WHEN a handle is dropped, THE SYSTEM SHALL refuse every new call on it, because the process never had the credential (docs/design/identity/STATEMENT-2026-09-22.md:55, docs/design/identity/STATEMENT-2026-09-22.md:29); a call already admitted follows the cancellation rule of R8. (2) The login token: it is in the process; THE SYSTEM SHALL record which token went to which seat, and the engine that runs the seat ends it on its own (docs/design/identity/STATEMENT-2026-09-22.md:56; docs/design/identity/STATEMENT-2026-09-22.md:61). (3) Sealed knowledge: once read it is in the process's context; permission controls the disclosure and the audit line records it, and neither takes back what was read (docs/design/identity/STATEMENT-2026-09-22.md:57). THE SYSTEM SHALL NOT describe revocation of a login token or of read knowledge as taking anything back. The real key is rotated upstream only when it is suspected leaked (docs/design/identity/STATEMENT-2026-09-22.md:29).

Owners: the door for dropping handles and refusing calls (docs/design/identity/STATEMENT-2026-09-22.md:27-29); the engine that runs the seat for ending it (docs/design/identity/STATEMENT-2026-09-22.md:56, docs/design/identity/STATEMENT-2026-09-22.md:61). Revocation in lys 'is itself an append with the live set folded from the log (DP26)' (docs/design/identity/STATEMENT-2026-09-22.md:79; docs/design/lys-anchor/DECISIONS.md:400), a fold 'ruled (DP26), not built' (docs/design/identity/STATEMENT-2026-09-22.md:126).

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-door/src/http/secrets_revoke.rs, dropping a handle and answering which seat holds which login token (docs/design/identity/STATEMENT-2026-09-22.md:55-56). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: (a) whether revoking the login token at the provider makes the seat's next call fail, proved with the provider before it is promised (docs/design/identity/STATEMENT-2026-09-22.md:56). (b) The file that carries the lys revocation fold: its owner is lys (docs/design/identity/STATEMENT-2026-09-22.md:126), and the key-history artifact that would carry a fold 'is its own future format' (crates/lys-core/src/delegation/mod.rs:208-210); no file is named until that format is designed.

**Acceptance:**
- A test drops a handle and then sends 3 new calls on it: all 3 are refused and the upstream test double's request count stays at its value before the drop.
- A test revokes a seat's login: the door's answer names the seat the token went to, and the door itself ends no process.
- A test revokes a sealed record's relation after one read: the next read is refused, the earlier audit line is still in the log, and the door's answer does not claim the earlier read is undone.
- The provider-revocation question and the fold's file are recorded open in this requirement when the row is reviewed.

**Checklist:**
- C10 — Each of the three revocation cases has its own behaviour: a dropped handle refuses every new call, the login token's seat is ended by its engine, and read sealed knowledge is disclosed and recorded, never claimed back.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.

**Stories:**
- S3 (Person, Grants an agent provisioned under them access to an account) — As a person, I want to drop an agent's handle and have every new call on it refused at once, so that taking access back does not wait on rotating the real key.
- S9 (Operator, Keeps the accounts and revokes access) — As an operator revoking access, I want each case to say what it reaches: a handle refuses new calls, a call already admitted follows the stated cancellation rule, a login token's seat is ended by its engine, and read knowledge stays read, so that I am never told revocation took back what it cannot.

### R8: State and enforce the cancellation rule for calls already admitted

WHILE a call already admitted by the proxy is in flight, a drop of its handle does not stop it; THE SYSTEM SHALL have the proxy carry an explicit cancellation rule for calls in flight, and that rule is part of the broker's design (docs/design/identity/STATEMENT-2026-09-22.md:55). THE SYSTEM SHALL write the rule down, in the door's broker module docs, before it is implemented, and SHALL give every call in flight at a drop an outcome the audit records. THE SYSTEM SHALL NOT leave the outcome of a call in flight at a drop undefined. What the rule is (let every admitted call finish, cancel at the next boundary, or another shape) is not stated by the statement; it is settled in this row's review (CN2) before any code, not here.

Owner: the door, whose proxy admits calls (docs/design/identity/STATEMENT-2026-09-22.md:27, docs/design/identity/STATEMENT-2026-09-22.md:55). Chippy's release 2 names 'demonstrate revocation and recovery after a crash' (docs/design/identity/STATEMENT-2026-09-22.md:155) and a retry needing 'a defined outcome' (docs/design/identity/STATEMENT-2026-09-22.md:70), which the rule must also answer for a call in flight when the door stops.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-door/src/http/secrets_in_flight.rs, the register of admitted calls and the rule applied to them at a drop (docs/design/identity/STATEMENT-2026-09-22.md:55). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: the rule itself.

**Acceptance:**
- The door's broker module docs state the cancellation rule in words before the row's first code commit.
- A test admits 2 calls against a slow upstream test double, drops the handle while both are in flight, then sends 1 new call: the new call is refused, and each of the 2 in-flight calls ends with the outcome the stated rule names, each with exactly 1 audit line recording that outcome.
- A test stops the door with 1 call in flight and restarts it: the call's outcome is recorded in the audit, and a retry of that call has the outcome the stated rule names.

**Checklist:**
- C11 — The proxy has an explicit cancellation rule for calls already admitted when their handle is dropped, written down before it is built.

**Stories:**
- S9 (Operator, Keeps the accounts and revokes access) — As an operator revoking access, I want each case to say what it reaches: a handle refuses new calls, a call already admitted follows the stated cancellation rule, a login token's seat is ended by its engine, and read knowledge stays read, so that I am never told revocation took back what it cannot.
- S11 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want each call that was in flight when its handle was dropped to show the outcome the cancellation rule gave it, so that no call's end is unaccounted for.

### R9: Count leases by uses, time window and spend

THE SYSTEM SHALL treat everything handed out as a lease: a number of uses, a time window, a spend cap (docs/design/identity/STATEMENT-2026-09-22.md:68). The time window lives in the signed delegation; uses and spend are counted by the door, because a signed object cannot count (docs/design/identity/STATEMENT-2026-09-22.md:68). WHEN two requests arrive at once on a one-use grant, THE SYSTEM SHALL NOT spend it twice, and a retry SHALL have a defined outcome (docs/design/identity/STATEMENT-2026-09-22.md:70). A hard cap SHALL need a reservation before work starts and a settlement after, so two concurrent sessions cannot both spend the same remaining allowance (docs/design/identity/STATEMENT-2026-09-22.md:70); hard spending caps ship only where reservation and enforcement are proved (docs/design/identity/STATEMENT-2026-09-22.md:156). The proxy sees only the spending that passes through it (docs/design/identity/STATEMENT-2026-09-22.md:70), and THE SYSTEM SHALL NOT report a spend total as covering what did not pass through the proxy.

Owners: the door counts uses and spend (docs/design/identity/STATEMENT-2026-09-22.md:68). The time window lives in the signed delegation, whose format is lys (docs/design/identity/STATEMENT-2026-09-22.md:68, docs/design/identity/STATEMENT-2026-09-22.md:82).

Lys-owned, in files: crates/lys-core/src/delegation/mod.rs (lys; docs/design/identity/STATEMENT-2026-09-22.md:82), whose 'Expiry. There is no not_after' paragraph (crates/lys-core/src/delegation/mod.rs:218-221) must be answered for a lease's time window; docs/design/WIRE-FORMATS.md (lys; docs/design/WIRE-FORMATS.md:11-21), which records the delegation that carries it.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the use and spend counters and reservations (docs/design/identity/STATEMENT-2026-09-22.md:68, docs/design/identity/STATEMENT-2026-09-22.md:70); crates/cambium-door/src/http/secrets_lease.rs, the check, reservation and settlement at the proxy (docs/design/identity/STATEMENT-2026-09-22.md:68-70). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: the delegation schema (docs/design/identity/STATEMENT-2026-09-22.md:21), and with it how a time window is carried: lys/delegation/v1 has no expiry by design (crates/lys-core/src/delegation/mod.rs:218-221), so a window in the signed delegation is a new version alongside v1, never a change to it; its shape waits on the schema.

**Acceptance:**
- A test with a lease of 2 uses sends 3 calls: the first 2 are forwarded and the 3rd is refused, and the upstream test double's request count is 2.
- A test sends 2 simultaneous requests on a one-use grant: with both held at the use check until both have arrived, so the race is forced rather than hoped for, exactly 1 is forwarded and the other is refused.
- A test retries a call whose first attempt was forwarded: the retry has the outcome the row's docs state, and the use count moves at most once.
- A test runs 2 concurrent sessions against a hard cap with room for 1: exactly 1 reservation succeeds, and the settled spend never exceeds the cap.
- A test sends a call after its lease's time window has ended: it is refused.
- The door's spend report labels its total as the spending that passed through the proxy.
- The delegation schema and the carriage of the time window are recorded open in this requirement when the row is reviewed.

**Files:**
- modify: crates/lys-core/src/delegation/mod.rs
- modify: docs/design/WIRE-FORMATS.md

**Checklist:**
- C12 — Everything handed out is a lease counted by uses, a time window and a spend cap: the window in the signed delegation, uses and spend counted by the door, a one-use grant never spent twice.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.

**Stories:**
- S2 (Person, Grants an agent provisioned under them access to an account) — As a person granting an agent access, I want to give it a handle under my own grant, limited by uses, time and spend, so that it can use the account without ever holding the credential and the grant traces back to me.

## Boundaries

- No credential, token or key value is ever written, read or quoted in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- A credential never passes through an agent: the only credential that reaches a process is the seat's own login at spawn (R5).
- Nothing the statement leaves open is decided in a row: the delegation schema, provider revocation of a login token, the proxy-handle login path, the worker-side owner, the fold's file, the audit line's schema and signer, and the cancellation rule until its review.
- Manifold is never a structural part: the engine that starts or ends a seat is whichever one runs the agent, and no row depends on any one engine.
- Every project works without the others: an engine without the broker reads its own pool file as it does today, and the door without the broker signs people in as it does today.
- No OpenBao and no external secrets engine.
- A shipped wire format is never mutated: lys/sealed-envelope/v1 and lys/delegation/v1 evolve only by a new version alongside, and any cryptographic change takes an adversarial review before it lands.
- No door file is edited in a round without a root naming the door repository.
- No row is dispatched until Waffles has reviewed it.

## Verification

- From the lys repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the door repository root: the door's own gates, all clean, once its root is set.
- From docs/ of the lys repository: python3 $DS2_METHOD/scripts/validate.py design/secrets and python3 $DS2_METHOD/scripts/check-coverage.py design/secrets exit 0.
- A search of every file a row touches finds no credential, token or key value.
