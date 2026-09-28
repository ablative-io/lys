---
type: brief
id: DIRECTORY-003
cluster: directory
title: Build the directory contract and signed authoritative identity changes
---

# DIRECTORY-003: Build the directory contract and signed authoritative identity changes

> **Cluster:** directory
> **Depends on:** DIRECTORY-002
> **Blocked by:** Waffles' review of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it), The exact file manifest for crates/lys-identity/, crates/lys-identity-server/ and tests/identity_contract/, reviewed before this row starts; revision 5 requires one for every wholly new module (docs/design/identity/briefs/IDENTITY-001.json:28, docs/design/identity/briefs/IDENTITY-001.json:190) and none is written yet
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> **Checklist:**
> - C11 — People and agents are registered with enduring identifiers and issuer-subject bindings, each agent under the signed-in person responsible for it, and registration issues no login, credential or certificate (ID001_DIRECTORY).
> - C12 — Every identity change is one signed event through lys-log-store under a jointly reviewed envelope, every answered projection equals replay across the crash boundaries, and a receipt verifies independently (ID001_AUDIT_FAULTS, ID001_RECEIPT).
> - C13 — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN).
> - C14 — The link-audit receiver is built and proved against fixtures before row 03 needs it (ID001_RECEIVER).
> - C15 — Each identity's lifecycle state is recorded as ADR-011 proposes, every transition one signed event, and the grant path's row is recorded open for Tom with its criteria drafted.
> **Stories:**
> - S1 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
> - S4 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.
> - S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

## Purpose

Row 04 of IDENTITY-001 revision 5, built before row 03 as Waffles reordered it: the directory of people and agents, where one signed committed event is both the identity change and its audit record, a receipt anyone can verify, the bounded step-1 administrator, and the link-audit receiver row 03 will need. Revised for the grant ruling (ADR-003): every agent is registered under the signed-in person responsible for it; and for the working lifecycle states (ADR-011, proposed): each identity's state is recorded beside it.

## Task

Carry IDENTITY-001 revision 5 row 04 (docs/design/identity/briefs/IDENTITY-001.json:168-207) into requirements: registration under a responsible person (R1), signed events and receipts (R2), the administrator (R3), the link-audit receiver (R4) and the lifecycle state, with the grant path's row recorded open for Tom (R5). Estimate: 10 focused implementer hours, revision 5's figure for this row (docs/design/identity/briefs/IDENTITY-001.json:206); no re-estimate, because the responsible person and the lifecycle state ride the same signed event envelope and projection as registration. If Tom places the grant path in this row, its hours are estimated then and any ceiling change goes to Tom through Waffles (CN7). The row's crates are wholly new modules: their exact file manifest is reviewed before the row starts (blocked_by, CN9). Every path is relative to the repository root. The reviewed file manifest is docs/design/directory/DIRECTORY-003-MANIFEST.md.

## Requirements

### R1: Register people and agents with enduring identifiers, each agent under its responsible person

THE SYSTEM SHALL add the domain crates crates/lys-identity (directory records, typed API, event projection) and crates/lys-identity-server (OIDC session handling, administrator admission), keeping lys-core, its cryptographic primitives and its published formats unchanged, every new module named in the reviewed manifest (docs/design/identity/briefs/IDENTITY-001.json:190). It SHALL define stable person and agent identifiers, external issuer-subject bindings, registration and display-profile changes, explicit provenance and operation-ID retry semantics, written as the contract in docs/design/identity/DIRECTORY-CONTRACT.md (docs/design/identity/briefs/IDENTITY-001.json:191; P1). Revised for the grant ruling (ADR-003; docs/design/identity/STATEMENT-2026-09-22.md:21): an agent is registered by a signed-in person, and its signed registration event records that person as the agent's responsible person for life (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:77-81). ADR-011 proposes that a person registers themselves by first sign-in; step 1's bounded administrator policy refuses every other mutation caller (P9; docs/design/identity/briefs/IDENTITY-001.json:38), so in step 1 the only caller that may register a person or an agent is the configured administrator (R3), a first sign-in registers nobody, and every agent's responsible person is the administrator who registered it; wider registration arrives with step 2's assignment (docs/design/identity/briefs/IDENTITY-001.json:38). No API of this row changes an agent's responsible person. Registering an agent SHALL NOT pretend it is running, manufacture a human login for it, or issue it a credential, handle or certificate (P3; docs/design/identity/briefs/IDENTITY-001.json:37). The deployment files gain the directory service beside the three dependencies.

**Acceptance:**
- ID001_DIRECTORY: register a person and an agent, edit the display profile, list/read both and reopen; enduring IDs and signed history remain unchanged.
- A registered agent's record and its signed registration event both name its responsible person, the signed-in person who registered it; a registration without a signed-in caller is refused and creates no record; no request of the API changes an agent's responsible person.
- Registering an agent creates no Rauthy user and issues no credential, handle or certificate: a test counts Rauthy users and issued credentials before and after a registration and finds both counts unchanged.

**Files:**
- create: crates/lys-identity/
- create: crates/lys-identity-server/
- create: tests/identity_contract/
- create: docs/design/identity/DIRECTORY-CONTRACT.md
- modify: Cargo.toml
- modify: Cargo.lock
- modify: deploy/identity/compose.yaml
- modify: deploy/identity/config.example.toml
- modify: deploy/identity/README.md

**Checklist:**
- C11 — People and agents are registered with enduring identifiers and issuer-subject bindings, each agent under the signed-in person responsible for it, and registration issues no login, credential or certificate (ID001_DIRECTORY).

**Stories:**
- S1 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
- S4 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (ID001_DIRECTORY): met. tests/identity_contract/tests/registration.rs:19 registers Ada and an agent and changes the agent's profile. It lists both identities (lines 43-50) and reads the person and the agent after a reopen (lines 51-64). It checks the reopened projection equals the live one and the three signed leaves are byte-identical before and after the reopen (lines 32-35 and 65-69).

Row 2: met. registration.rs:76 verifies the agent's registration leaf and matches Change::RegisterAgent { responsible } to the person, and the record's responsible() is the same person. registration.rs:267 shows POST /agents and POST /people with no session are refused 401 NotSignedIn and the identity count does not change. The POST /agents route takes no responsible person: responsible is the caller's bound person (crates/lys-identity-server/src/routes.rs:359). A body naming Grace as responsible is refused as a client error with no identity added, and after a profile change and an activate transition the agent's responsible person still reads as Ada.

Row 3: met. registration.rs:222 starts the service against FakeRauthy and first proves the request counter fires on GET /sign-in-providers. It then counts Rauthy users, Rauthy requests and credentials before and after POST /agents: certificates across every agent, service accounts and sessions (credential_count at line 185). All three counts are unchanged, and the agent reads 'registered' with no logins.

The crates, the contract text, the Cargo workspace entries and the deploy files were already on main from the earlier DIRECTORY-003 landing; lys-core is untouched.
- Deviation: The crates, DIRECTORY-CONTRACT.md, the workspace Cargo entries and deploy/identity already existed on main (commits 99a7ea2 to 38f15f4, merged in PR #34), so this round wrote no new modules. It added the missing acceptance proofs and ID001 tags inside existing manifest files. tests/identity_contract/src/fake_rauthy.rs is not in docs/design/directory/DIRECTORY-003-MANIFEST.md, because a later brief added it. I modified it inside the brief's tests/identity_contract/ wall rather than creating a new file. CN1 says 'documents only', which conflicts with this brief's own code file walls. I followed the requirement's file lists and named the conflict here. Credentials counted are certificates, service accounts and sessions: the directory service holds no handle store to count, so handles are covered by 'the issuer was sent nothing' rather than a handle count.
- Files changed:
  - modified: `tests/identity_contract/tests/registration.rs` — Six tests. ID001_DIRECTORY registers a person and an agent, edits the profile, lists and reads both, reopens, and checks the projection equals replay and the signed leaves are byte-identical. The signed registration event and the record both name the responsible person. Over HTTP, registering an agent sends the issuer nothing, creates no Rauthy user and issues no certificate, service account or session, with the request counter first shown to fire. A caller with no session is refused NotSignedIn and records nothing. A body naming a responsible person is refused, and a profile change or transition leaves the responsible person where it was.
  - modified: `tests/identity_contract/src/fake_rauthy.rs` — The stand-in issuer API also serves a user list and user creation, and counts every request that reaches it on any route (user_count, request_count), so a test can count Rauthy users and requests before and after.
  - modified: `tests/identity_contract/src/harness.rs` — Adds Service::start_adjusted, which lets a test change the written configuration (used for the wrong-issuer administrator), and Service::log_size, the log's size read through the public receipt route. start_setting now delegates to start_adjusted.
  - modified: `docs/design/identity/DIRECTORY-CONTRACT.md` — Gains the section 'The grant path, recorded open for Tom' (R5). Its identifiers, bindings, registration, profiles, provenance, operation-ID retry and responsible-person-for-life text is unchanged from main.
- Checklist delivery:
  - [x] C11 — People and agents are registered with enduring identifiers and issuer-subject bindings, each agent under the signed-in person responsible for it, and registration issues no login, credential or certificate (ID001_DIRECTORY). — Enduring ids and history across a reopen (registration.rs:19); the responsible person in both the event and the record (registration.rs:76); no Rauthy user, request or credential from a registration (registration.rs:222).
- Story delivery:
  - [x] S1 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it. — Every agent's signed registration names its responsible person, and no API call moves it (registration.rs:76, registration.rs:267). Grants themselves are outside this row.
  - [x] S4 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact. — An agent is registered under the administrator's person before it runs, and every change is a signed event (registration.rs:19, registration.rs:76).

### R2: Commit every identity change as one signed event through lys-log-store

THE SYSTEM SHALL commit every signed directory change through lys-log-store and rebuild projections from those events at open, reconciling an uncertain write before any affected read answers as current (P4, P5; docs/design/identity/briefs/IDENTITY-001.json:192, docs/design/identity/briefs/IDENTITY-001.json:40). The versioned event envelope, outside lys-core, with typed audit and context payloads, the log coordinate returned in the receipt outside the leaf and service-attested human actions, SHALL be reviewed jointly with Archie before it signs durable bytes, and written in docs/design/identity/IDENTITY-EVENTS.md; the commitment hash is named explicitly, and a SHA-256 attestation commitment is never confused with a BLAKE3 content address (docs/design/identity/briefs/IDENTITY-001.json:45). A receipt carries the fields of P7. THE SYSTEM SHALL provide a read-only receipt and inclusion-verification path (docs/design/identity/briefs/IDENTITY-001.json:193). The service attests the authenticated human actor and their provenance and never claims a person signed bytes with a key they do not hold (P8). The log is lys-log-store's file storage, not a Haematite backend (docs/design/identity/briefs/IDENTITY-001.json:45).

**Acceptance:**
- ID001_AUDIT_FAULTS: enumerate append/pin/projection crash boundaries and count exercised cases; every answered projection equals replay; uncertain operations retain identity and resolve once without silent loss or double application.
- ID001_RECEIPT: independently verify a recorded change against a checkpoint/key; changed actor, payload, sequence or signature fails verification. Secret-redaction tests cover debug, errors and serialized public responses.
- docs/design/identity/IDENTITY-EVENTS.md records the envelope's version, its typed payloads and the named commitment hash, and records Archie's review of it before any durable bytes are signed under it.

**Files:**
- create: crates/lys-identity/
- create: tests/identity_contract/
- create: docs/design/identity/IDENTITY-EVENTS.md

**Checklist:**
- C12 — Every identity change is one signed event through lys-log-store under a jointly reviewed envelope, every answered projection equals replay across the crash boundaries, and a receipt verifies independently (ID001_AUDIT_FAULTS, ID001_RECEIPT).

**Stories:**
- S4 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.
- S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (ID001_AUDIT_FAULTS): met. tests/identity_contract/tests/events.rs:821 builds the boundary list from Fault::None through an exhaustive next-boundary match (after(), events.rs:752), so a new Fault variant cannot be left out. The five boundaries are: stop after commit before answer, before the leaf, leaf stored behind a failed write, pin lost, and store unreadable. Each runs under Recovery::Retry and Recovery::Restart. The test asserts 5 boundaries and 10 exercised cases. Each case (exercise(), events.rs:772) checks: no projection is answered while an append is uncertain; the first answer's id is kept; the retry answers the same id; the log has exactly 2 leaves; Grace is one identity; and the answered projection equals a fresh replay. events.rs:842 covers a committed operation whose answer was lost, found by its operation id after a restart. The earlier single-boundary tests at events.rs:311-464 remain.

Row 2 (ID001_RECEIPT): met. events.rs:584 checks the genuine receipt with the crate's verify_receipt and with independently() (events.rs:556), which never calls the crate's decoder. It builds four tampered cases: a different actor re-signed, a different payload re-signed, the coordinate's index moved, and a flipped signature byte. Each fails both checks, counted to 4. events.rs:686 checks for redaction: the service key's Debug and a short-key load error (Display and Debug) carry no seed hex, client secret or session value. So do five serialized answers (POST /people, an OperationReused refusal, /service-key, /receipts/0, /identities), counted to 5, with the public key's presence asserted as a positive control.

Row 3: met by the file already on main. docs/design/identity/IDENTITY-EVENTS.md:1 names lys/identity-event/v1, line 30 gives the version field, lines 61-68 name the SHA-256 commitment apart from the RFC 6962 leaf hash and from BLAKE3, and lines 7 and 70-72 record the joint review by Archie and Buckley at 6590854 before durable bytes were signed.
- Deviation: The receipt and fault machinery was already on main. This round added the independent verification, the per-field tampering, the redaction test and the enumerated boundaries the acceptance rows name. The independent check's independence is of COSE parsing and Merkle algorithm, not of the Ed25519 implementation: it uses lys_core::Ed25519Identity::verify. coset was added as a dev-dependency of the test crate, which touches Cargo.lock by one line.
- Files changed:
  - modified: `tests/identity_contract/tests/events.rs` — Adds two modules. receipts: ID001_RECEIPT verifies a receipt independently with coset's COSE_Sign1 parse, an Ed25519 check, a SHA-256 commitment and leaf hash, and a hand-written RFC 9162 inclusion walk; a changed actor, payload, sequence or signature each fails both the crate verifier and the independent check, counted to 4. A redaction test checks debug forms, a key error and five public or error responses for the signing seed, the client secret and the session value. faults: ID001_AUDIT_FAULTS walks the harness's crash boundaries through an exhaustive match, runs each under retry and restart (10 cases counted), and checks every answered projection equals replay and each operation resolves once. A separate test covers a commit whose answer was lost.
  - modified: `tests/identity_contract/Cargo.toml` — Adds coset (workspace, dev-dependency only) so the receipt test parses the signed event with standard COSE tooling instead of the crate's own reader.
  - modified: `Cargo.lock` — identity-contract's dependency list gains the coset entry already in the lock (0.4.2). No new package and no version change.
- Checklist delivery:
  - [x] C12 — Every identity change is one signed event through lys-log-store under a jointly reviewed envelope, every answered projection equals replay across the crash boundaries, and a receipt verifies independently (ID001_AUDIT_FAULTS, ID001_RECEIPT). — Every answered projection equals replay across 10 enumerated cases (events.rs:821); receipts verify independently and fail on each changed field (events.rs:584); the envelope review is recorded (IDENTITY-EVENTS.md:70-72).
- Story delivery:
  - [x] S4 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact. — Every change is one signed event, and a crash or retry never loses or doubles one (events.rs:821).
  - [x] S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word. — A verifier checks a recorded change with COSE tooling, SHA-256 and RFC 9162 given only the leaf, the service key, a checkpoint and a proof (events.rs:556, events.rs:584).

### R3: Admit only the configured administrator to change the directory

THE SYSTEM SHALL authenticate the initial directory administrator by an explicitly configured issuer and subject, never by email or by being the first visitor, fail closed for every other mutation caller, and make the limited step-1 authority visible (docs/design/identity/briefs/IDENTITY-001.json:193, docs/design/identity/briefs/IDENTITY-001.json:38; P9). General assignment and why-access views arrive in step 2.

**Acceptance:**
- ID001_ADMIN: an unauthenticated caller, a non-admin with the same email, and wrong issuer/subject are refused; denied calls cannot mutate state.

**Files:**
- create: crates/lys-identity-server/
- create: tests/identity_contract/

**Checklist:**
- C13 — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN).

**Stories:**
- S4 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (ID001_ADMIN): met. tests/identity_contract/tests/admission.rs:150: after the administrator writes 3 leaves, a caller with no session is refused 401 NotSignedIn on all 5 mutation routes. A login with the administrator's email but another subject is refused 403 NotAdmitted on all 5. That is 10 refusals, counted. log_size is still 3 and GET /identities is byte-equal to before. admission.rs:194 configures the administrator as (https://elsewhere.example.test, administrator-subject); signing in as administrator-subject at the fake issuer is then the right subject at the wrong issuer. All 5 mutations are refused NotAdmitted (counted), /identities is refused, and the log holds 0 leaves. The earlier tests at admission.rs:29-85 remain: the administrator admitted, same email refused, no session refused, first visit admits nobody. Admission compares issuer and subject exactly (crates/lys-identity-server/src/admission.rs:41).
- Deviation: (none)
- Files changed:
  - modified: `tests/identity_contract/tests/admission.rs` — Tagged ID001_ADMIN. Adds refused_everywhere, which sends all five mutation routes (people, agents, profile, transitions, logins). One test sends them as a caller with no session and as a non-administrator with the administrator's email, 10 refusals counted, then proves the log size and every record are unchanged. Another configures the administrator at another issuer and proves the right subject at the wrong issuer is refused on all five routes and on reads, with nothing logged.
  - modified: `tests/identity_contract/src/harness.rs` — start_adjusted, so the configured administrator can be set to an issuer the fake never signs in through; log_size, used for 'denied calls cannot mutate state'.
- Checklist delivery:
  - [x] C13 — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN). — Unauthenticated, same-email and wrong-issuer callers are refused on every mutation route, and nothing changes (admission.rs:150, admission.rs:194).
- Story delivery:
  - [x] S4 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact. — Only the configured administrator registers agents in step 1, so the responsible person recorded is always an authenticated, configured login.

### R4: Build the link-audit receiver before row 03 needs it

THE SYSTEM SHALL build and test the authenticated link-audit receiver against the reviewed typed contract and fixtures before DIRECTORY-004 needs it. It consumes the minimal durable source selected in row 01 (a same-transaction link audit record or outbox with stable operation IDs and explicit pending and acknowledged state; docs/design/identity/briefs/IDENTITY-001.json:42), deduplicates stable source operation IDs and returns verifiable receipts. It separates issuer observations from human-signed claims, and audit actor provenance survives replay (docs/design/identity/briefs/IDENTITY-001.json:194).

**Acceptance:**
- ID001_RECEIVER: fixture delivery, duplicate delivery, lost acknowledgement, receiver restart and unauthorized source exercise the reviewed link-audit contract without requiring the future multi-provider fork. Count each leg and prove one logical event per source operation.

**Files:**
- create: crates/lys-identity/
- create: crates/lys-identity-server/
- create: tests/identity_contract/

**Checklist:**
- C14 — The link-audit receiver is built and proved against fixtures before row 03 needs it (ID001_RECEIVER).

**Stories:**
- S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (ID001_RECEIVER): met. tests/identity_contract/tests/link_audit.rs:117 drives three fixture operations (fixtures::linked). Fixture delivery of op-1. A duplicate delivery of op-1 answers the first receipt. Lost acknowledgement: op-2's answer is dropped, it stays pending, and redelivery answers the same receipt. Receiver restart: op-3 is unacknowledged when the directory is dropped and reopened, and the pending op-3 and op-1 are redelivered and answered as first. Each leg is counted exactly once (4 legs), nothing is left pending, the log grows by exactly 3 (one logical event per source operation), and all 3 receipts pass verify_receipt against a checkpoint and proof. link_audit.rs:190 covers the unauthorized-source leg over the real endpoint: none gets 401 NotSignedIn, the administrator and another login get 403 NotAdmitted, 3 refusals counted, and log_size is unchanged. The configured source's delivery sent twice answers identical receipts and adds one leaf. Observations stay apart from claims and provenance survives replay (link_audit.rs:17, unchanged). No multi-provider fork is required.
- Deviation: The restart leg is proved at library level (link_audit.rs:117), because the HTTP harness has no way to restart a running service. The unauthorized-source leg is proved at HTTP level (link_audit.rs:190), because authentication of the source lives in the server.
- Files changed:
  - modified: `tests/identity_contract/tests/link_audit.rs` — Tagged ID001_RECEIVER. Adds an Outbox model of the source's durable outbox: stable ids stay pending until acknowledged, and the first answer is kept. One test counts four legs against the fixtures (fixture delivery, duplicate delivery, lost acknowledgement, receiver restart), proves 3 source operations leave exactly 3 events, and verifies each receipt against the log. Another, over HTTP, refuses no session, the administrator and another login, 3 counted with nothing logged, then shows the configured source's duplicate delivery answered with its first receipt and logged once.
- Checklist delivery:
  - [x] C14 — The link-audit receiver is built and proved against fixtures before row 03 needs it (ID001_RECEIVER). — All five legs are exercised and counted, with one logical event per source operation (link_audit.rs:117, link_audit.rs:190).
- Story delivery:
  - [x] S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word. — Every accepted observation's receipt verifies against the log's checkpoint and the service key (link_audit.rs:117).

### R5: Record each identity's lifecycle state, and keep the grant path's row open

THE SYSTEM SHALL record beside each identity its lifecycle state as ADR-011 proposes (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44): registration yields registered; activate, suspend, reinstate and retire are the only transitions, each one signed directory event naming the authenticated actor and their provenance, the identity, from, to, when and the reason given; a retired identity is never reactivated; a transition outside the table is refused by name and records nothing. In step 1 the caller is the configured administrator (R3). The state is recorded, not enforced: nothing in this row reads it to admit or refuse an action (CN11). ADR-011 is status proposed; a change to the states by Tom is a brief revision before this requirement is built.

OPEN for Tom, not decided here: which row owns the grant path. The path: a person signs in, creates an agent under themselves, grants it one project, the agent's action on that project is allowed, the grant is revoked or the agent suspended, and the same action is refused by name, with the audit record naming who made each change (docs/design/identity/STATEMENT-2026-09-22.md:183, Chippy 17:08; docs/design/identity/PROVISIONING-2026-09-22.md:39-46). The sources do not settle its row. Revision 5 keeps arbitrary grants and live capability enforcement out of step 1 and in road step 2 (docs/design/identity/briefs/IDENTITY-001.json:29-30). Chippy's concrete first screen of 17:08, said after revision 5 was written, puts the path on step 1's screen (docs/design/identity/STATEMENT-2026-09-22.md:183), and PROVISIONING places the grant in 'steps 1 and 2 of the road' without naming a row (docs/design/identity/PROVISIONING-2026-09-22.md:23-24). The question for Tom: does the grant path land in this row (DIRECTORY-003), in a new row of this cluster before DIRECTORY-005, or in the first brief of road step 2? Whichever row it lands in carries these criteria, drafted here so none is lost: (a) allowed before revoke: with the agent active and one grant on project P, the agent's action on P is admitted; (b) refused after revoke: once the grant is revoked, the same action is refused by name at the next check and nothing on P changes; (c) refused after suspend: once the agent is suspended, the same action is refused by name while its grant stays recorded, under whatever suspension semantics Tom settles (design non-goals); (d) every grant, revoke and transition is one signed audit record naming the actor. None of (a) to (d) is an acceptance criterion of this brief.

**Acceptance:**
- A test drives register, activate, suspend, reinstate and retire on one agent and counts 5 signed events, each naming actor, identity, from, to and time; the projection after reopen equals replay.
- Each transition outside the table (retired to active, registered to suspended, registered to retired, and a transition of an unknown identity) is refused by name and the log's size does not change; the test counts one refusal per case it names.
- No code in this row asks SpiceDB for a decision or writes a grant to it, and the grant path's row is still recorded open for Tom, with its question and drafted criteria, when the row is reviewed.

**Files:**
- create: crates/lys-identity/
- create: tests/identity_contract/
- create: docs/design/identity/DIRECTORY-CONTRACT.md

**Checklist:**
- C15 — Each identity's lifecycle state is recorded as ADR-011 proposes, every transition one signed event, and the grant path's row is recorded open for Tom with its criteria drafted.

**Stories:**
- S1 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: met. tests/identity_contract/tests/lifecycle.rs:18 registers an agent and activates, suspends, reinstates and retires it. It verifies every leaf about the agent and finds exactly 5. Each names the administrator as actor and the agent as identity, and matches (time, from, to, transition, reason) exactly: (11, none to registered), (20, registered to active, activate), (21, active to suspended, suspend, 'review'), (22, suspended to active, reinstate), (23, active to retired, retire, 'replaced'). The projection after reopen equals the live one.

Row 2: met. lifecycle.rs:97 refuses retired to active (TransitionRefused), registered to suspended (TransitionRefused), registered to retired (TransitionRefused) and a transition of an unknown identity (IdentityUnknown), counts 4 refusals and shows the log size unchanged.

Row 3: met. None of this row's modules asks SpiceDB for a decision or writes a grant (directory.rs, lifecycle.rs, projection.rs, event.rs, receipt.rs, link_audit.rs, admission.rs, oidc.rs, session.rs, link_audit_api.rs, receipts_api.rs); the only matches for 'grant' in them are doc text. The grant path's question and its drafted criteria (a) to (d) are recorded open for Tom at docs/design/identity/DIRECTORY-CONTRACT.md:50, as well as in the brief's R5 spec.
- Deviation: Since this brief was written, later rows have added grants and SpiceDB code in their own modules (crates/lys-identity/src/grants/, crates/lys-identity-server/src/spicedb.rs), and DIRECTORY-006's amendment carries a grant path (docs/design/directory/briefs/DIRECTORY-006.md:37). This row does not touch those modules. The contract section records the question as DIRECTORY-003 left it, points to DIRECTORY-006 as a brief that states its own authority, and does not claim that Tom placed the path in a row.
- Files changed:
  - modified: `tests/identity_contract/tests/lifecycle.rs` — The five-event test now checks, for each signed event about the agent, the actor, from, to, the transition, the time and the reason given against an exact expected table. It also shows a retired agent's record and whole history stay readable, and the projection after a reopen equals replay. The refusal test (every transition outside the table, one refusal per case, log size unchanged) is kept.
  - modified: `docs/design/identity/DIRECTORY-CONTRACT.md` — New section at line 50, 'The grant path, recorded open for Tom': the path, the question (this row, a new row before DIRECTORY-005, or road step 2's first brief) with its sources, drafted criteria (a) to (d) marked as not criteria of DIRECTORY-003, and the statement that this row's calls ask SpiceDB nothing and write no grant.
- Checklist delivery:
  - [x] C15 — Each identity's lifecycle state is recorded as ADR-011 proposes, every transition one signed event, and the grant path's row is recorded open for Tom with its criteria drafted. — Five signed lifecycle events each check out field by field (lifecycle.rs:18); out-of-table transitions are refused with nothing recorded (lifecycle.rs:97); the grant path is recorded open with its drafted criteria (DIRECTORY-CONTRACT.md:50).
- Story delivery:
  - [x] S1 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it. — The lifecycle states a withdrawal would act on are recorded as signed events naming the actor, and the grant path that makes withdrawal enforceable stays recorded open for Tom with its criteria.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and Waffles approves a brief revision before that file is edited (CN9).
- Development isolation: disposable test identities and test provider registrations only; no production tokens, real business sign-in or live Cambium participant migration (CN2).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion and never claimed by a loop completion (CN5, CN6).
- No row is dispatched until Waffles has reviewed it.
- lys-core, its cryptographic primitives and its published wire formats are unchanged; the event envelope lives outside lys-core and signs no durable bytes before its joint review with Archie.
- No grant is written and no permission is enforced in this row; the grant path's row stays open for Tom (R5).
- Registration issues no login, credential, handle or certificate.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root: rg -n 'ID001_DIRECTORY|ID001_AUDIT_FAULTS|ID001_ADMIN|ID001_RECEIPT|ID001_RECEIVER' crates/lys-identity tests/identity_contract finds each identifier in a test.
- A search of every file this row touches finds no credential, token or key value.
