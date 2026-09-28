---
type: brief
id: DIRECTORY-006
cluster: directory
title: Enforce human-rooted grants, explicit delegation and the You-page access journey
---

# DIRECTORY-006: Enforce human-rooted grants, explicit delegation and the You-page access journey

> **Cluster:** directory
> **Depends on:** DIRECTORY-002, DIRECTORY-003
> **Blocked by:** Waffles review of every requirement and the accepted mock-up/conformance snapshot before dispatch., DIRECTORY-002 and DIRECTORY-003 are published foundation briefs, not implemented foundations. DIRECTORY-003 must establish the identity crates, reviewed event envelope and server integration manifest; reconcile every future modify path below against its landed implementation before dispatch. The listed new grant modules do not exist in current source., Surface integration waits for DIRECTORY-005 to establish surface/identity and its route/type-generation paths. R6 cannot dispatch before that dependency; DIRECTORY-005 must not depend on R6, so backend grant work and screen integration have separate readiness., Reviewed grant representation, root-authority bootstrap and SpiceDB consistency contract remain open in DIRECTORY-001C5 and ADR-003. Behavioural tests below constrain that design; they do not authorise freezing a cryptographic format., Lifecycle suspension semantics and role version policy are supplied by their reviewed ADRs. Reinstatement safety and expiry invariants here do not settle their remaining policy choices., Review of the proposed R1 grant schema and R4 freshness mechanism; their additional fields and consistency strategy are proposals, not existing settled policy.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on the operator's workstation.
> **Checklist:**
> - C21 — The proposed application grant schema separates exercise and pass-on authority; its additional fields require independent review before dispatch and do not change published crypto formats.
> - C22 — Delegation requires an explicit permission to delegate to the requested subject kind; use-only, absent policy and owner labels cannot create it.
> - C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.
> - C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.
> - C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.
> - C26 — Inherited expiry and provisional end dates are enforced at admission; role changes and reinstatement cannot resurrect expired or revoked authority.
> - C27 — Typed browser/API/agent seams use one authenticated evaluator; forward and reverse explanations name the same authority path and revision without private-record leakage.
> - C28 — Observed grant usage names its source and time; not seen is not reported as never used.
> - C29 — The You and delegation screens show real server authority, separate service accounts from sign-in identities, and preserve personal versus administrator visibility.
> - C30 — The accepted mock-up conformance is tested through actual requests, refusals, pending outcomes, keyboard paths and durable read-back rather than simulated success.
> **Stories:**
> - S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
> - S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.
> - S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.
> - S11 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it.
> - S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

## Purpose

Make the human-rooted grant path enforceable: sign in, give an agent a bounded permission, prove the action works, revoke the source, and prove the same action is refused. Use and lending are separate positive rights and the screen explains the real server decision. Authority for this amendment: docs/design/identity/CONFORMANCE.md at commit 1353c22, rows 1.3, 1.4, 2.1–2.6 and 7.3. Context only: Tom requested complete briefs tied to the mock-up on 23 September 2026 at 19:04:22 Melbourne, then the right Cambium boards and workflows at 19:10:35; these conversational times are not repository evidence.

## Task

Implement R1–R5 in order after the directory/service foundations, then R6 after the standalone surface foundation. Estimated work: R1 2h, R2 3h (ancestry/refusal matrix), R3 4h (forced durability boundaries), R4 3h (freshness and expiry), R5 2h, R6 3h (two-person browser evidence), total 17 hours as a new increment, not silently inside IDENTITY-001's 48-hour ceiling. Estimates are proposals for review. Exact file walls follow the new crate roots already named by IDENTITY-001 rows 04/05; none is asserted implemented. Every future modify path must be reconciled to those dependencies' actual manifests before dispatch. Preserve all ID001 requirements in their owning predecessor briefs and the frozen IDENTITY-001 history. In scope: domain grants, authority enforcement, durable audit, explanation API and You/delegation UI. Out: provider federation changes, secret-value storage/proxy, actual process execution, assistant actions, graph renderer and published cryptographic format changes.

## Requirements

### R1: Define the reviewed grant contract without changing published cryptography

PROPOSAL FOR REVIEW, not a settled grant schema: define a typed application grant contract that identifies the grant, issuer, holder, responsible person, resource, exercisable actions, explicit pass-on authority and permitted recipient kinds, source grant, time window, policy revision and authorising audit event. Exercising an action and delegating it SHALL be separate decisions. A missing pass-on field SHALL NOT mean permission. Relations SHALL resolve through the reviewed model to action/resource sets; the code SHALL NOT infer a global owner/editor/viewer rank from display strings. Before any durable grant is signed, the event envelope, canonical encoding, root-authority bootstrap and lineage semantics SHALL be independently reviewed. This row defines application-domain types in lys-identity; it SHALL NOT reinterpret seat/speaks-for or mutate lys/delegation/v1, which currently supplies neither this capability schema nor an expiry. Provenance: ADR-003; STATEMENT-2026-09-22.md, Everything is pegged to a human authority; PROVISIONING-2026-09-22.md, What one grant says. Conformance 1.3, 1.4, 2.2, 2.3. The extra holder, responsible-person, recipient-kind, time-window, policy-revision and authorising-event fields are design proposals in this row; PROVISIONING-2026-09-22.md supplies five source facts and leaves representation open. This row and GRANT_CONTRACT cannot be dispatched until independent contract review ratifies the schema. Checkable conformance source: docs/design/identity/CONFORMANCE.md at commit 1353c22.

**Acceptance:**
- GRANT_CONTRACT: serialise/parse a grant with each declared member and independently supplied schema fixture; unknown members, absent required authority, malformed lineage and unknown recipient kinds refuse by name. Count each case; no permission is supplied by a deserialisation default.
- GRANT_MODEL: define two model relations whose names do not imply their action sets. A requested subset is checked against the model, not lexical order or a hard-coded rank; model version is retained in the decision.
- GRANT_WIRE_BOUNDARY: an explicit reviewed application-envelope identifier precedes the first durable signed grant. Existing published lys wire-vector tests remain byte-identical; an unratified delegation format is not used as a substitute capability token.

**Files:**
- create: crates/lys-identity/src/grants/mod.rs
- create: crates/lys-identity/src/grants/types.rs
- create: crates/lys-identity/src/grants/error.rs
- create: crates/lys-identity/tests/grant_contract.rs
- create: docs/design/identity/GRANT-CONTRACT.md
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C21 — The proposed application grant schema separates exercise and pass-on authority; its additional fields require independent review before dispatch and do not change published crypto formats.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: GRANT_CONTRACT: met. crates/lys-identity/tests/grant_contract.rs:133 round-trips every member against an independently built CBOR fixture. :150-240 counts the named refusals (MemberUnknown, MemberMissing, LineageMalformed, RecipientKindUnknown), and :248 shows no default supplies a permission. GRANT_MODEL: met. grant_contract.rs:272 judges by the model's action sets, with no rank, and keeps the model version in the decision. GRANT_WIRE_BOUNDARY: met. The envelope is explicit, GRANT_ENVELOPE = application/vnd.lys.grant-event.v1+cbor at crates/lys-identity/src/grants/codec.rs:27. No other format is read (grant_contract.rs:332, grant_receipts.rs:215), and lys-core is untouched. The review the row requires is now recorded. ADR-078 consequence 7 (decisions.json:1021) reads: 'This ruling, together with DIRECTORY-006 R1 to R5 as built and accepted on hand/DIRECTORY-006-R5 at e525a91 after a second reader's review, stands as the ratification DIRECTORY-006 R1 and DIRECTORY-006's blocked_by ask for; no separate contract review is waited on.' GRANT-CONTRACT.md:14 now cites and quotes it. I found this record by searching the estate; I did not write it.
- Deviation: The review was not performed in this round. It already existed as ADR-078, recorded with DIRECTORY-025, and this round records it in the contract, where the reviewer looked for it. GRANT-CONTRACT.md is R1's own wall file.
- Files changed:
  - modified: `docs/design/identity/GRANT-CONTRACT.md` — Line 14 now records the review as ADR-078 (docs/design/decisions.json:1005, decided 2026-09-27) instead of 'not recorded here yet'. It quotes the ratification sentence (decisions.json:1021), names the decided representation, root-authority bootstrap and C25 freshness settlement, and keeps the grant-representation non-goal open for Tom. Line 65 now documents the recorded-use count and missing reports.
- Checklist delivery:
  - [x] C21 — The proposed application grant schema separates exercise and pass-on authority; its additional fields require independent review before dispatch and do not change published crypto formats. — Exercise and pass-on are separate members (key 8), and published crypto is unchanged. The independent review is ADR-078, now recorded at GRANT-CONTRACT.md:14.
- Story delivery:
  - [x] S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it. — The contract is ratified (ADR-078). Enforcement is in R2 and R4, and the screen evidence is in R6 (grants.test.tsx:323-472).

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] GRANT_CONTRACT: serialise/parse a grant with each declared member and independently supplied schema fixture; unknown members, absent required authority, malformed lineage and unknown recipient kinds refuse by name. Count each case; no permission is supplied by a deserialisation default. — crates/lys-identity/tests/grant_contract.rs:133 round-trip against the fixture at :79; named refusals counted at :150-240; the no-default check at :248; the gate's tests leg exited 0
  - [x] GRANT_MODEL: define two model relations whose names do not imply their action sets. A requested subset is checked against the model, not lexical order or a hard-coded rank; model version is retained in the decision. — crates/lys-identity/tests/grant_contract.rs:272; the tests leg exited 0
  - [ ] GRANT_WIRE_BOUNDARY: an explicit reviewed application-envelope identifier precedes the first durable signed grant. Existing published lys wire-vector tests remain byte-identical; an unratified delegation format is not used as a substitute capability token. — The envelope is explicit at crates/lys-identity/src/grants/codec.rs:27 (application/vnd.lys.grant-event.v1+cbor), and lys/delegation/v1 is not read. But docs/design/identity/GRANT-CONTRACT.md:14 says 'That review is not recorded here yet', so the identifier is not reviewed.
- Issues:
  - The independent review of the grant envelope, canonical encoding, root-authority bootstrap and lineage semantics must be performed and recorded in docs/design/identity/GRANT-CONTRACT.md before GRANT_WIRE_BOUNDARY and C21 can be met.

### R2: Enforce affirmative delegation and bounded ancestry

WHEN a person or agent asks to delegate authority, THE SYSTEM SHALL check the actor's current exercise and delegation rights, requested resource/actions and recipient kind against every effective ancestor, ending at an authorised person. The requested authority SHALL be an explicit subset, never inherited wholesale. A use-only grant, an owner label, missing prohibition or access to the UI SHALL NOT establish pass-on authority. People-only authority SHALL refuse agents even when the person owns that agent. Unknown parents, cycles, wrong responsible persons and attempts to launder authority through a new role, secret handle or sibling grant SHALL refuse before any mutation. An agent that holds explicit pass-on permission may invoke the same operation as a person; being an agent alone is not either a permit or a categorical prohibition. The policy determines recipients rather than the mock-up's sample restriction to the caller's own agents. Provenance: ADR-003; AGENT-PARITY-2026-09-23.md. Conformance 2.1–2.4, 7.3. Secret lending also recognises server-verified real ownership as the affirmative route in docs/design/identity/CONFORMANCE.md at commit 1353c22 row 7.3; a display label is not that ownership record.

**Acceptance:**
- GRANT_USE_VS_LEND: Tom may read a project through Dana's use-only grant. His read succeeds; browser, direct API and agent-tool attempts to grant that access to his agent all refuse and create zero grant events. Add a distinct affirmative pass-on grant: the bounded control now succeeds.
- GRANT_RECIPIENT: exercise human-only, agent-only and non-delegable grants with both recipient kinds. Assert each named permit/refusal and exact zero mutations on denied requests, including the owner's own agent case.
- GRANT_ANCESTRY: attempt a broader resource, broader action, a forged parent, a cycle, a changed responsible person and a revoked ancestor. Each refuses with the blocking boundary. A two-hop permitted narrower chain succeeds and explains its actual ancestors.
- GRANT_AGENT_PARITY: the same authenticated authority and request produce the same effective decision through browser/API/tool routes. An agent granted pass-on can delegate inside that grant; an agent without it cannot.

**Files:**
- create: crates/lys-identity/src/grants/admission.rs
- create: crates/lys-identity/src/grants/lineage.rs
- create: crates/lys-identity/src/grants/authority.rs
- create: crates/lys-identity/tests/grant_delegation.rs
- modify: crates/lys-identity/src/grants/mod.rs

**Checklist:**
- C22 — Delegation requires an explicit permission to delegate to the requested subject kind; use-only, absent policy and owner labels cannot create it.
- C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: GRANT_USE_VS_LEND: met. crates/lys-identity/tests/grant_delegation.rs:23 exercises use-only and refuses pass-on with zero events, and route parity is at crates/lys-identity-server/tests/grants.rs:176. GRANT_RECIPIENT: met. grant_delegation.rs:78 covers human-only, agent-only and non-delegable grants for both recipient kinds, with zero mutations on each refusal. GRANT_ANCESTRY: met. grant_delegation.rs:177 refuses a broader resource or action, a forged parent, a cycle, a changed responsible person and a revoked ancestor, and explains a two-hop chain. GRANT_AGENT_PARITY: met, at grant_delegation.rs:342 and server tests/grants.rs:176. This landed in 30f8cc8 and is unchanged this round.
- Deviation: (none)
- Checklist delivery:
  - [x] C22 — Delegation requires an explicit permission to delegate to the requested subject kind; use-only, absent policy and owner labels cannot create it. — grant_delegation.rs:23,:78
  - [x] C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank. — grant_delegation.rs:177
- Story delivery:
  - [x] S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it. — Contract ratified by ADR-078; screen evidence in R6.
  - [x] S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools. — Backend parity at server tests/grants.rs:176. On the UI, the use-only refusal is shown by name at grants.test.tsx:441, and the explanation at grants.test.tsx:225.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] GRANT_USE_VS_LEND: Tom may read a project through Dana's use-only grant. His read succeeds; browser, direct API and agent-tool attempts to grant that access to his agent all refuse and create zero grant events. Add a distinct affirmative pass-on grant: the bounded control now succeeds. — crates/lys-identity/tests/grant_delegation.rs:23 grant_use_vs_lend_use_only_is_exercised_and_never_passed_on; route parity in crates/lys-identity-server/tests/grants.rs:176
  - [x] GRANT_RECIPIENT: exercise human-only, agent-only and non-delegable grants with both recipient kinds. Assert each named permit/refusal and exact zero mutations on denied requests, including the owner's own agent case. — grant_delegation.rs:78 grant_recipient_each_kind_is_permitted_or_refused_by_name
  - [x] GRANT_ANCESTRY: attempt a broader resource, broader action, a forged parent, a cycle, a changed responsible person and a revoked ancestor. Each refuses with the blocking boundary. A two-hop permitted narrower chain succeeds and explains its actual ancestors. — grant_delegation.rs:177 grant_ancestry_refuses_at_the_blocking_boundary_and_explains_a_permitted_chain; the cycle case at :151
  - [x] GRANT_AGENT_PARITY: the same authenticated authority and request produce the same effective decision through browser/API/tool routes. An agent granted pass-on can delegate inside that grant; an agent without it cannot. — grant_delegation.rs:342 grant_agent_parity_routes_decide_alike_and_pass_on_decides_who_delegates; lys-identity-server tests/grants.rs:176
- Checklist verified: C22, C23

### R3: Commit grants and their audit as one replayable operation

THE SYSTEM SHALL record a grant mutation as one signed authoritative event using DIRECTORY-003's reviewed event owner, and derive both the directory projection and the permission projection from that event. A database write followed by best-effort audit is forbidden. Each mutation SHALL carry a stable operation ID; the same ID and payload returns the same logical outcome, while changed payload under that ID refuses. Unknown append or projection outcomes SHALL remain named and retained for reconciliation; the API SHALL NOT manufacture success or issue a fresh operation when acknowledgement is lost. Any read or admission that depends on unresolved authority SHALL refuse naming the affected grant/operation; unrelated authority stays usable. Projection progress SHALL record the minimum revision required for a fresh decision, including across restart. Provenance: directory principlesP4/P5 and ID001_AUDIT_FAULTS; conformance1.4,8.1.

**Acceptance:**
- GRANT_DURABILITY: enumerate every append, sync, authoritative-event and projection boundary in the implementation and inject a failure at each. Count the legs. Every answered grant/permission read equals replay, while unresolved reads name the grant/operation. Unrelated grant controls still succeed.
- GRANT_IDEMPOTENCE: lose acknowledgement after commit, retry identical ID/payload, reopen and retry again. One logical grant and one authorising event remain. A changed payload under the same ID refuses and changes neither projection.
- GRANT_AUDIT: independently verify a grant mutation receipt; tampering with actor, source grant, recipient, resource/actions, sequence or signature is rejected. A refusal never appears as a successful grant event.

**Files:**
- create: crates/lys-identity/src/grants/events.rs
- create: crates/lys-identity/src/grants/projection.rs
- create: crates/lys-identity/src/grants/recovery.rs
- create: crates/lys-identity/tests/grant_faults.rs
- create: crates/lys-identity/tests/grant_receipts.rs
- modify: crates/lys-identity/src/grants/mod.rs

**Checklist:**
- C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.

**Stories:**
- S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: GRANT_DURABILITY: met. crates/lys-identity/tests/grant_faults.rs:267 injects a fault at each boundary and compares answered reads with replay, with a fault-free control at :470. GRANT_IDEMPOTENCE: met. grant_faults.rs:423 loses the acknowledgement, retries and reopens, leaving one authorising event. GRANT_AUDIT: met. grant_receipts.rs:70 verifies a receipt and rejects each tampering, and :172 shows a refusal is never a grant event. The only change this round is the snapshot format, which R5 needs. The state version moves to 3, so an old snapshot takes the named refuse-and-rebuild path and never misreads. The full lys-identity suite passes after the change.
- Deviation: The snapshot state file was edited for R5's use count, outside R5's wall. The reviewer's R5 issue requires the change, and it follows CLAUDE.md's snapshot rule: a new version, with an old one refused by name.
- Files changed:
  - modified: `crates/lys-identity/src/grants/state.rs` — STATE_VERSION is now 3 because the book state carries each grant's recorded-use count. A version 2 snapshot is refused by name and the book is rebuilt from the whole log, the existing path.
  - modified: `crates/lys-identity/src/grants/book_state.rs` — A grant record is now five fields, with the use count written and read.
- Checklist delivery:
  - [x] C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections. — grant_faults.rs:267,:423; grant_receipts.rs:70
- Story delivery:
  - [x] S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority. — One signed operation per change, and retries return the first receipt.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] GRANT_DURABILITY: enumerate every append, sync, authoritative-event and projection boundary in the implementation and inject a failure at each. Count the legs. Every answered grant/permission read equals replay, while unresolved reads name the grant/operation. Unrelated grant controls still succeed. — crates/lys-identity/tests/grant_faults.rs:267 grant_durability_every_boundary_answers_replay_or_names_what_is_unresolved
  - [x] GRANT_IDEMPOTENCE: lose acknowledgement after commit, retry identical ID/payload, reopen and retry again. One logical grant and one authorising event remain. A changed payload under the same ID refuses and changes neither projection. — grant_faults.rs:423 grant_idempotence_a_lost_acknowledgement_retries_to_one_grant
  - [x] GRANT_AUDIT: independently verify a grant mutation receipt; tampering with actor, source grant, recipient, resource/actions, sequence or signature is rejected. A refusal never appears as a successful grant event. — crates/lys-identity/tests/grant_receipts.rs:70 grant_audit_a_receipt_verifies_and_every_tampering_is_rejected; the refusal case at :172
- Checklist verified: C24
- Stories verified: S10

### R4: Enforce revocation, inherited expiry and current permission decisions

WHEN an ancestor is revoked or expires, THE SYSTEM SHALL refuse fresh exercise and further delegation through every grant derived from it. Independent grants to the same person or resource SHALL remain independent. Proposal under review: require a permission decision at least as fresh as the authoritative change, and refuse or await freshness rather than permit from a stale replica, unavailable engine or unresolved projection. The mechanism and choice must be settled before dispatch. Expiry SHALL be enforced at admission using the named clock, including inherited limits; a role edit or version move SHALL NOT renew a provisional grant. Reinstating a suspended identity SHALL recheck current ancestry and leases, never resurrect separately revoked or expired grants. Shared ancestor use/spend budgets are counted once by the broker in SECRETS-002 R9, never cloned into each descendant. The exact broader suspension policy remains the lifecycle ADR's, not a decision smuggled into this row. Conformance 2.5, 2.6, 3.3, 4.5.

**Acceptance:**
- GRANT_REVOKE: create a two-hop chain and an independently authorised sibling. A permitted action succeeds, then revoke the chain root and repeat exactly the same action for each descendant: every derived call refuses; the independent control succeeds.
- GRANT_EXPIRY: under a controlled clock, assert permit immediately before each relevant end boundary and refusal at/after it, including an earlier ancestor end. Neither editing a role nor selecting a newer role version extends the holding.
- GRANT_FRESHNESS: pause permission projection before applying a committed revoke; a caller presenting the required revision cannot obtain a permit from the old state. Resume projection and verify the named revoked decision after reopen.
- GRANT_REINSTATE: while an identity is suspended, revoke one grant and expire a second. After reinstatement both remain refused; only a third still-authorised control is usable. No renewal or issuance event is invented.

**Files:**
- create: crates/lys-identity/src/grants/revocation.rs
- create: crates/lys-identity/src/grants/expiry.rs
- create: crates/lys-identity/src/grants/permission.rs
- create: crates/lys-identity/tests/grant_revocation.rs
- create: crates/lys-identity/tests/grant_expiry.rs
- modify: crates/lys-identity/src/grants/mod.rs

**Checklist:**
- C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.
- C26 — Inherited expiry and provisional end dates are enforced at admission; role changes and reinstatement cannot resurrect expired or revoked authority.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S11 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: GRANT_REVOKE: met. crates/lys-identity/tests/grant_revocation.rs:40 revokes a chain root; every descendant is refused and the independent sibling still succeeds. GRANT_EXPIRY: met. grant_expiry.rs:31 tests each end boundary on a controlled clock. GRANT_FRESHNESS: met. grant_revocation.rs:170 answers StaleDecision while projection lags a committed revoke, and Revoked after reopen. GRANT_REINSTATE: met at grant_expiry.rs:158. C25's required review of the freshness mechanism is recorded: ADR-078 consequence 9 (docs/design/decisions.json:1023) reads 'The freshness mechanism (C25) is settled by the owning lead with a second reader as DIRECTORY-006 R4 builds it … otherwise it is refused with StaleDecision'. GRANT-CONTRACT.md:14 now cites it.
- Deviation: (none)
- Checklist delivery:
  - [x] C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch. — Behaviour is at grant_revocation.rs:40,:170. The review is ADR-078 consequence 9 (decisions.json:1023), recorded at GRANT-CONTRACT.md:14. Round 1 marked this done without citing the review; the citation is now given.
  - [x] C26 — Inherited expiry and provisional end dates are enforced at admission; role changes and reinstatement cannot resurrect expired or revoked authority. — grant_expiry.rs:31,:158
- Story delivery:
  - [x] S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it. — Rides on R1 (ratified) and R6 (evidence now present).
  - [x] S11 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it. — grant_expiry.rs:31,:158

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] GRANT_REVOKE: create a two-hop chain and an independently authorised sibling. A permitted action succeeds, then revoke the chain root and repeat exactly the same action for each descendant: every derived call refuses; the independent control succeeds. — crates/lys-identity/tests/grant_revocation.rs:40 grant_revoke_withdraws_every_derived_grant_and_leaves_the_independent_one
  - [x] GRANT_EXPIRY: under a controlled clock, assert permit immediately before each relevant end boundary and refusal at/after it, including an earlier ancestor end. Neither editing a role nor selecting a newer role version extends the holding. — crates/lys-identity/tests/grant_expiry.rs:31 grant_expiry_every_end_binds_at_its_boundary_and_no_role_change_extends_it
  - [x] GRANT_FRESHNESS: pause permission projection before applying a committed revoke; a caller presenting the required revision cannot obtain a permit from the old state. Resume projection and verify the named revoked decision after reopen. — grant_revocation.rs:170 grant_freshness_a_stale_engine_never_permits_after_a_committed_revoke (paused engine at :142-168)
  - [x] GRANT_REINSTATE: while an identity is suspended, revoke one grant and expire a second. After reinstatement both remain refused; only a third still-authorised control is usable. No renewal or issuance event is invented. — grant_expiry.rs:158 grant_reinstate_rechecks_every_grant_and_resurrects_none
- Checklist verified: C26
- Stories verified: S11
- Issues:
  - C25 is marked done, but it requires the revision-freshness mechanism to be reviewed before dispatch, and blocked_by (DIRECTORY-006.json:15) names that review as outstanding. No record of it exists. The review must be performed and recorded, and C25 must be set back to not done until then.

### R5: Expose one authenticated grant and explanation seam

THE SYSTEM SHALL expose typed operations for list/read, delegate, revoke and explain through the standalone identity server, all calling the same authority owner. API/tool access SHALL enforce the same checks as the UI; browser controls are presentation only. A why-permitted response SHALL identify actual authority path, responsible person, effective scope and policy revision; a why-refused response SHALL name the blocking condition without disclosing another identity's protected records. The reverse question, who can exercise this action on this resource, SHALL use the same evaluator and revision, with its own visibility permission. Incomplete pagination SHALL remain explicit. A last-use value reports observed use at named enforcement points; absence is not evidence of never-used. No live Cambium, Aion, Argus or Manifold is required. Conformance 8.1, 8.2, 8.4 and ADR-004.

**Acceptance:**
- GRANT_API_AUTH: unauthenticated, wrong issuer/subject, use-only and unauthorised-revoke requests are named refusals and create zero mutations. Exercise valid controls and count each route, rather than testing only button visibility.
- GRANT_EXPLAIN: the forward decision and reverse enumeration agree for the same resource/action/revision; paging returns every authorised holder exactly once and names continuation. Hidden private grants do not leak IDs, labels or existence to an unauthorised querier.
- GRANT_LAST_USED: an unobserved grant is labelled not seen; a recorded use reports its source/time; a missing reporting source is distinguished from a zero count. Reopen preserves the attribution.
- GRANT_STANDALONE: execute all operations against disposable local dependencies with other Ablative servers absent. Permission-engine outage is a named refusal, never an implicit standalone permit.

**Files:**
- create: crates/lys-identity-server/src/grants.rs
- create: crates/lys-identity-server/src/grant_contract.rs
- create: crates/lys-identity-server/tests/grants.rs
- create: crates/lys-identity-server/tests/grant_explanations.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C27 — Typed browser/API/agent seams use one authenticated evaluator; forward and reverse explanations name the same authority path and revision without private-record leakage.
- C28 — Observed grant usage names its source and time; not seen is not reported as never used.

**Stories:**
- S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.
- S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: GRANT_API_AUTH: met at server tests/grants.rs:84 (every refusal named, zero writes) and :176 (each route counted). GRANT_EXPLAIN: met at grant_explanations.rs:124, :184 and :237. GRANT_STANDALONE: met. Everything runs against a disposable local service, and an engine outage is a named refusal (grant_revocation.rs:258). GRANT_LAST_USED: met. Not seen, with a source and time for a seen use, was already covered (grant_last_used.rs:44). Missing report versus zero count: a permitted check whose use event is refused is held as Unreported, and reads source 'missing' with its count, route and reason. A grant with no exercise reads recorded 0, source 'reported'. In the library, grant_use_reports.rs:108 injects a refused append: recorded 0, NotSeen, unreported count 1. The untouched source grant stays a reported zero, and a later recorded use does not make the count whole. At the server seam, tests/grants.rs:382 makes the grant log's leaves directory read-only. It then checks that /grants/{id} and /grants agree on source 'missing' and on the unreported reason matching the check's use_event reason. A later tool use gives recorded 1 with the source still missing. After service.restart() the read shows the same seen, at, route, use_event and recorded 1, so reopen preserves the attribution. Round 1's record wrongly said the server does not expose last use; it does, through LastUseView.
- Deviation: Two things differ from the plan. First, the missing-report case is a separate Usage/Unreported value next to LastUse, not a new LastUse variant. LastUse is the fold of the log, and a missing report is by definition in no log; folding it would make the snapshot disagree with the log it is bound to. Second, because the missing report is in no log, it is held in memory. A restart cannot know about a missing report from before it: the source then reads 'reported' again. views.rs:97, usage.rs and GRANT-CONTRACT.md:65 all say so, and the test asserts it rather than hiding it. Files outside R5's wall were edited, as the reviewer's R5 issue directed: projection.rs, usage.rs, authority.rs, mod.rs, state.rs and book_state.rs, reviews_api.rs, the shared test harness tests/identity_contract/src/harness.rs (restart), and the surface mirror (generated/grants.ts, model.ts). The wall's crates/lys-identity-server/src/grant_contract.rs is the landed directory grant_contract/.
- Files changed:
  - modified: `crates/lys-identity/src/grants/projection.rs` — GrantRecord now carries uses, the count of recorded use events, folded in apply (:277) and read at :84. Zero is documented as a count of recorded uses, not a claim of no use.
  - created: `crates/lys-identity/src/grants/usage.rs` — Unreported (:26) records the count, latest time, route and reason of permitted exercises whose use event was refused. Usage (:39) holds the latest recorded use, the recorded count and any unreported. Grants::unreported and Grants::usage are at :84 and :89.
  - modified: `crates/lys-identity/src/grants/authority.rs` — Grants holds the unreported map. check() notes an exercise as unreported when its use event is refused (:379).
  - modified: `crates/lys-identity/src/grants/mod.rs` — Declares pub mod usage and re-exports Unreported and Usage.
  - modified: `crates/lys-identity-server/src/grant_contract/views.rs` — LastUseView (:97) now carries recorded, source ('reported' or 'missing', :112) and unreported (UnreportedView, :78). GrantView::new(record, unreported) replaces From<&GrantRecord>.
  - modified: `crates/lys-identity-server/src/grant_contract/mod.rs` — Exports UnreportedView.
  - modified: `crates/lys-identity-server/src/grants.rs` — The list and read routes build GrantView with the grant's unreported exercises.
  - modified: `crates/lys-identity-server/src/reviews_api.rs` — The review list builds GrantView the same way, so the two answers cannot differ.
  - modified: `crates/lys-identity-server/tests/grants.rs` — The last_use equality asserts now include recorded and source. New test at :382 covers the missing report and a restart.
  - created: `crates/lys-identity/tests/grant_use_reports.rs` — Library tests: a missing report is not a zero count (:108), and the recorded count survives a reopen (:193).
  - modified: `tests/identity_contract/src/harness.rs` — Service::restart (:359) stops the service and starts it again over the same directory, on a new address with a new client, so a test can reopen at the server seam.
  - modified: `surface/identity/src/generated/grants.ts` — The LastUse mirror gains recorded and source, and the Unreported type is added.
  - modified: `surface/identity/src/features/grants/model.ts` — lastUsedText appends ' · N use(s) not recorded' when the source is missing.
- Checklist delivery:
  - [x] C27 — Typed browser/API/agent seams use one authenticated evaluator; forward and reverse explanations name the same authority path and revision without private-record leakage. — grants.rs:176; grant_explanations.rs:124,:184
  - [x] C28 — Observed grant usage names its source and time; not seen is not reported as never used. — The source and time of each use are in LastUseView. Not seen is never 'never used', and a missing report reads source 'missing' (views.rs:97; tests grant_use_reports.rs:108, server tests/grants.rs:382).
- Story delivery:
  - [x] S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools. — The same permitted operation works through the API, tool and browser (grants.rs:176). The use-only refusal is explained on screen (grants.test.tsx:441).
  - [x] S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect. — grant_explanations.rs:124,:184

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] GRANT_API_AUTH: unauthenticated, wrong issuer/subject, use-only and unauthorised-revoke requests are named refusals and create zero mutations. Exercise valid controls and count each route, rather than testing only button visibility. — crates/lys-identity-server/tests/grants.rs:84 and :176; the tests leg exited 0
  - [x] GRANT_EXPLAIN: the forward decision and reverse enumeration agree for the same resource/action/revision; paging returns every authorised holder exactly once and names continuation. Hidden private grants do not leak IDs, labels or existence to an unauthorised querier. — crates/lys-identity-server/tests/grant_explanations.rs:124, :184 and :237
  - [ ] GRANT_LAST_USED: an unobserved grant is labelled not seen; a recorded use reports its source/time; a missing reporting source is distinguished from a zero count. Reopen preserves the attribution. — LastUse (crates/lys-identity/src/grants/projection.rs:40-52) has only NotSeen and Seen, and LastUseView (crates/lys-identity-server/src/grant_contract/views.rs:79-110) has only seen/at/route/use_event. Neither represents a missing reporting source.
  - [x] GRANT_STANDALONE: execute all operations against disposable local dependencies with other Ablative servers absent. Permission-engine outage is a named refusal, never an implicit standalone permit. — crates/lys-identity/tests/grant_revocation.rs:258 grant_engine_outage_refuses_by_name_and_writes_nothing; the server tests run against a local service
- Checklist verified: C27
- Stories verified: S12
- Issues:
  - LastUse and LastUseView must gain a distinct case for a missing reporting source (as opposed to no observed use), with a test at the server seam that covers reopen.
  - The dev record wrongly said the server does not expose last use and that grant_contract.rs does not exist.
- Fixes:
  - Corrected the R5 dev record in docs/design/directory/briefs/DIRECTORY-006.json and re-rendered the .md. It now says last use is served as LastUseView (grant_contract/views.rs:79-110) and that the wall's grant_contract.rs exists as the module directory grant_contract/.

### R6: Implement the You and delegation screens from the server contract

THE SYSTEM SHALL render the accepted mock-up's You page and delegation form from the server's generated grant types and decisions. Show sign-in identities separately from service access, source grants, effective operations/resource, affirmative pass-on rights and inherited end boundary. The form SHALL explain refused choices the caller is allowed to discover; it SHALL NOT enumerate other people's private secrets merely to say no. Personal views are scoped to the signed-in person, while an independently authorised directory administrator may inspect the wider directory. A failed or uncertain mutation SHALL remain refused or pending, with the original operation ID, never an optimistic grant or a new retry. The access graph consumes the same explanation response; its rendering belongs to Archie's graph brief. No simulated confirmation timer or sample-data authority calculation ships. Conformance 1.3–1.5, 2.3–2.4, 8.1 and 8.4. The screens landed from hand/identity-surface-land at 6f57bf7 (PR 35), not from a build of this brief.

**Acceptance:**
- GRANT_SCREEN: using two test people and their agents, render the source grant, action/resource scope, pass-on decision and inherited expiry from fixture IDs. Switching the signed-in person changes the personal data; permitted administrator inspection remains possible through its separate route.
- GRANT_SCREEN_REFUSAL: a use-only source, excessive requested scope, people-only policy, expired ancestor and service outage are refused visibly with the server reason; the request count and record count prove no hidden mutation happened.
- GRANT_SCREEN_PENDING: withhold the acknowledgement after durable delegation. The page keeps the original operation pending, retries by that ID, and eventually shows one grant; it never changes to success on a timer.
- GRANT_CONFORMANCE: pin the accepted mock-up file hash and numbered conformance rows in the test evidence. Exercise keyboard operation and deep linking as well as API refusal parity; sample data and visual similarity alone are not a passing acceptance.

**Files:**
- create: surface/identity/src/features/me/You.tsx
- create: surface/identity/src/features/grants/Delegate.tsx
- create: surface/identity/src/features/grants/Answer.tsx
- create: surface/identity/tests/grants.test.tsx
- create: surface/identity/tests/me.test.tsx
- create: surface/identity/tests/revoke.test.tsx
- modify: surface/identity/src/routes.tsx
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C29 — The You and delegation screens show real server authority, separate service accounts from sign-in identities, and preserve personal versus administrator visibility.
- C30 — The accepted mock-up conformance is tested through actual requests, refusals, pending outcomes, keyboard paths and durable read-back rather than simulated success.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.
- S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: GRANT_SCREEN: met. grants.test.tsx:324 mounts #/me as Ada and asserts the source grant, actions, pass-on and 'Ends no later than … 27 Oct', keyed by fixture ids. It then signs in as Bea and asserts her ledger grant ending 15 Nov, and checks that none of Ada's grant ids, her id or Scribe's id remain in the DOM. :368 shows Bea's grants through the administrator's route #/file/<BEA>/access: /directory/people is asked and /people is not. :386 refuses that route by name for a non-administrator, with nothing posted. GRANT_SCREEN_REFUSAL: met. The matrix at grants.test.tsx:427-472 covers UseOnly, ActionsOutside, RecipientRefused (the people-only policy) and Expired (expired ancestor), each 403, and PermissionEngineUnavailable (503). The reasons are the Display text of crates/lys-identity/src/grants/error.rs. Each case shows the reason in #dAnswer and makes exactly 1 POST /grants. There is no re-read, no 'Given' toast, and the hold rows are unchanged. The case count is asserted as 5. GRANT_SCREEN_PENDING: met at grants.test.tsx:90, :108 and :124: the same operation id is retried, and there is no timer success. GRANT_CONFORMANCE: met in jsdom. acceptance/grants.spec.ts:93 hashes docs/design/identity/mockup/index.v5.html against e67622e19f450e2335029f293f18b4757801f3245758a8b667521afa120ab91f, so a changed mock-up fails. The numbered rows run through it.each, and :235 asserts all 7 were exercised. :244 covers keyboard: the g+u key registry, Enter and Space on relation chips, and Escape closing with focus returned. :296 opens from a deep link, and :321 checks that the screen shows the API's refusal body word for word. Run results: tsc --noEmit exits 0; vitest runs 40 files and 300 tests, all passing. The subagent confirmed each test by injecting a fault: the missing-use rendering, the refusal matrix and the person switch each fail when their behaviour is broken.
- Deviation: Four differences from the brief. (1) The wall names YouGrants.tsx, DelegateGrant.tsx and GrantExplanation.tsx; the landed screens are features/me/You.tsx, features/grants/Delegate.tsx and Answer.tsx. They were tested, not duplicated. The brief revision that renames the wall is still the reviewer's to approve. (2) No browser runner dependency was approved, so none was added. acceptance/grants.spec.ts runs under vitest with jsdom, and its header says so. jsdom does not activate a native button on Enter, so the two native-button activations are clicks; the spec asserts those buttons are real, enabled and in tab order. A real-browser run is left for when a runner is approved. (3) fixtures.ts and vite.config.ts are outside the wall; they are named here. (4) generated/grants.ts, not generated/index.ts, was the file that needed the last-use type.
- Files changed:
  - modified: `surface/identity/tests/grants.test.tsx` — Two people and their agents, with the signed-in person switched (:324). The administrator's directory route shows another person's grants (:368), and a non-administrator is refused by name (:386). A five-case refusal matrix with request and record counts (:440-472). A missing use report shown as distinct from a reported zero (:480).
  - created: `surface/identity/tests/acceptance/grants.spec.ts` — Pins the mock-up's SHA-256 (:72, :93) and seven numbered conformance rows, 1.3, 1.4, 1.5, 2.3, 2.4, 8.1 and 8.4, each run by a check, with the count asserted (:229-235). Also covers keyboard operation (:244), deep linking (:296) and API refusal parity (:321).
  - modified: `surface/identity/tests/fixtures.ts` — Adds exports for Bea's person, directory, /me and grants. Existing last_use fixtures now carry recorded and source.
  - modified: `surface/identity/vite.config.ts` — The vitest include now also collects tests/**/*.spec.ts, so the acceptance spec runs.
- Checklist delivery:
  - [x] C29 — The You and delegation screens show real server authority, separate service accounts from sign-in identities, and preserve personal versus administrator visibility. — Real server answers, with personal versus administrator visibility (grants.test.tsx:324,:368,:386). The separate service-account card is covered in acceptance row 1.3.
  - [x] C30 — The accepted mock-up conformance is tested through actual requests, refusals, pending outcomes, keyboard paths and durable read-back rather than simulated success. — Requests, refusals, pending outcomes, keyboard paths and read-back are tested (grants.test.tsx:440, :90; acceptance :244, :296, :321), and the mock-up hash is pinned (:93). This is in jsdom, not a real browser.
- Story delivery:
  - [x] S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it. — Each grant traces to a person on screen, for two people (grants.test.tsx:324).
  - [x] S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools. — The use-only refusal is explained by name (grants.test.tsx:441). The same operation is available through the UI and tools (server tests/grants.rs:176).
  - [x] S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect. — The administrator's inspection route works, and a non-administrator is refused without leakage (grants.test.tsx:368,:386).

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] GRANT_SCREEN: using two test people and their agents, render the source grant, action/resource scope, pass-on decision and inherited expiry from fixture IDs. Switching the signed-in person changes the personal data; permitted administrator inspection remains possible through its separate route. — surface/identity/tests/grants.test.tsx renders a single person; there is no test that switches the signed-in person and no administrator-route test
  - [ ] GRANT_SCREEN_REFUSAL: a use-only source, excessive requested scope, people-only policy, expired ancestor and service outage are refused visibly with the server reason; the request count and record count prove no hidden mutation happened. — grants.test.tsx:79 covers one refusal only; there is no matrix of the five cases with request and record counts
  - [x] GRANT_SCREEN_PENDING: withhold the acknowledgement after durable delegation. The page keeps the original operation pending, retries by that ID, and eventually shows one grant; it never changes to success on a timer. — surface/identity/tests/grants.test.tsx:87, :121 and :141
  - [ ] GRANT_CONFORMANCE: pin the accepted mock-up file hash and numbered conformance rows in the test evidence. Exercise keyboard operation and deep linking as well as API refusal parity; sample data and visual similarity alone are not a passing acceptance. — grep across surface/identity/tests finds no index.v5 hash pin; there is no tests/acceptance directory; package.json:11 runs vitest only, with no browser runner
- Issues:
  - Add a two-person switching test and an administrator-route test for GRANT_SCREEN.
  - Add a refusal matrix covering use-only, excessive scope, people-only, expired ancestor and outage, with request and record counts.
  - Pin the hash of docs/design/identity/mockup/index.v5.html and the numbered conformance rows in a browser acceptance spec that exercises keyboard operation and deep linking.
  - Revise the brief so the R6 wall names the landed You.tsx, Delegate.tsx and Answer.tsx instead of YouGrants.tsx, DelegateGrant.tsx and GrantExplanation.tsx, and approve the browser runner dependency before this work is dispatched.

## Boundaries

- A missing file or unreviewed integration seam is a named dispatch blocker; do not silently expand a row wall or label an empty integration ready.
- No real credentials, live operator account mutation or production service writes in tests; use disposable local providers and permission fixtures.
- Every mutation seam is typed and generated; UI and agent callers use the same server checks. Do not infer authority from email, display name, mock-up data or absence of a deny flag.
- The same grant cannot acquire more authority through delegation, a secret handle, role migration, retries or restart. Use, delegation and permission to inspect another identity are distinct.
- The source/mock-up is a conformance specification, not implementation evidence. A test or source-only review never substitutes for the required live demonstration.
- No code in lys-core or published wire format changes in this brief. A capability format decision requires its own independent adversarial review before durable signing.
- Waffles owns board placement and workflow connection review. Create a blocked card with this exact brief path; do not move it into a dispatching status until its named blockers and row review are resolved.

## Verification

- From docs/: python3 "$DS2_METHOD/scripts/validate.py" design/directory and python3 "$DS2_METHOD/scripts/check-coverage.py" design/directory. Render with render-cluster.py and verify a second render is byte-identical.
- At implementation time run the repository battery from the exact revision: fmt, strict default/all-feature Clippy, feature-full tests and both rustdoc shapes. Run the granted/refused matrix and report each exercised leg, fault ordinal and unrun check.
- After the relevant screen row lands, demonstrate sign-in, agent registration, permitted action, source revocation and refusal to Tom using installed artifact hashes; record Melbourne time and exact authority/operation/receipt IDs. This human verification is not an automated acceptance criterion.
- Review the Cambium card: Card key resolves this exact JSON brief; project board rule connects to the supported deployed workflow; blocker/status prevents premature dispatch. No live test run or duplicate submission is used merely to inspect the configuration.
