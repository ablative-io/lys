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
> - S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
> - S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.
> - S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

## Purpose

Row 04 of IDENTITY-001 revision 5, built before row 03 as Waffles reordered it: the directory of people and agents, where one signed committed event is both the identity change and its audit record, a receipt anyone can verify, the bounded step-1 administrator, and the link-audit receiver row 03 will need. Revised for the grant ruling (ADR-003): every agent is registered under the signed-in person responsible for it; and for the working lifecycle states (ADR-011, proposed): each identity's state is recorded beside it.

## Task

Carry IDENTITY-001 revision 5 row 04 (docs/design/identity/briefs/IDENTITY-001.json:168-207) into requirements: registration under a responsible person (R1), signed events and receipts (R2), the administrator (R3), the link-audit receiver (R4) and the lifecycle state, with the grant path's row recorded open for Tom (R5). Estimate: 10 focused implementer hours, revision 5's figure for this row (docs/design/identity/briefs/IDENTITY-001.json:206); no re-estimate, because the responsible person and the lifecycle state ride the same signed event envelope and projection as registration. If Tom places the grant path in this row, its hours are estimated then and any ceiling change goes to Tom through Waffles (CN7). The row's crates are wholly new modules: their exact file manifest is reviewed before the row starts (blocked_by, CN9). Every path is relative to the repository root.

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
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
- S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

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
- S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.
- S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

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
- S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

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
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

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
- From docs/: python3 $DS2_METHOD/scripts/validate.py design/directory and python3 $DS2_METHOD/scripts/check-coverage.py design/directory exit 0.
- From the repository root: rg -n 'ID001_DIRECTORY|ID001_AUDIT_FAULTS|ID001_ADMIN|ID001_RECEIPT|ID001_RECEIVER' crates/lys-identity tests/identity_contract finds each identifier in a test.
- A search of every file this row touches finds no credential, token or key value.
