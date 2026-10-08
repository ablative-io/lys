---
type: brief
id: DIRECTORY-090
cluster: directory
title: Name degraded grant settlement and certify complete tail evidence
---

# DIRECTORY-090: Name degraded grant settlement and certify complete tail evidence

> **Cluster:** directory
> **Depends on:** DIRECTORY-006
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on the operator's workstation.
> **Checklist:**
> - C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.
> - C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.
> - C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.
> **Stories:**
> - S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
> - S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

## Purpose

Grant settlement deliberately preserves unrelated authority, but frame.rs:82-86, authority.rs:191 and commit.rs:199-201 discard real errors; four server boundaries turn every grant failure into a boolean. This amendment to DIRECTORY-006 R3/C24/S10 separates proved projection-only availability from an unread, uncertified log tail. Every actual failure remains named with its cause and real selected revision; a readable independent chain can still be used. Log reconciliation failures return their original error. A complete authenticated tail witness is a separate named deliverable in this series, not an assumption that a single Uncertain or a per-act writer lock bounds every possible tail. Source diagnosis is pinned to f12df005d0013d8dd0ee9c0a388ef06fa41a1973/tree032b6e11c411f826d822ed742532b62dc88e8756 and its docs-only descendant eab3d7753a5d8be1fa793ef2c349fb9f8449204a; the original availability policy entered at6ea490183debf787e8a4a864e0f1ac3a015579d8. C23/C24/C25 and S8/S10 are deliberately split with DIRECTORY-006; its original durability, idempotence and audit obligations remain, with the log-unreadable availability control explicitly amended below.

## Task

Write the red first. Before implementation, run behavior reds using existing interfaces: projection-only failure silently permits an unrelated grant; unreadable reconciliation can permit an older unrelated grant; boot drops projection failure; four independent server boundaries collapse the original GrantError; mutation acknowledgement loses a projection cause and turns failed revision reads into revision0. Assert exact original causes, actual known revisions and no privileged/event side effect. Preserve the exact existing Some(required)->StaleDecision and None->Revoked control in grant_revocation.rs:227-243. Preserve every original count, reopen, retry and audit assertion. New marker or witness API absence is not a behavioral red; write those API/proof assertions separately. Implement all seven requirements below in bounded source handovers. No requirement is implemented by publishing this brief. Marker propagation and source compatibility are explicit changes; no stored migration is needed because these additions are ephemeral metadata/capabilities, not new stored bytes. The complete-tail witness is implemented and tested as evidence in this series; it does not change the approved reconciliation-error refusal rule without a separately reviewed availability decision.

## Requirements

### R1: Retain failures at settlement and startup

THE SYSTEM SHALL remove both settle_log().ok() and project-error revision fallback from grants/frame.rs, and project().ok() from Grants::open_with. Reconciliation failure SHALL emit its step and original typed error and return that original error without constructing a Frame or Permit. No complete affected-tail witness exists in the current generic store. R6 delivers that evidence contract separately; this brief still returns the original reconciliation error for every reconciliation-failed permission read, including unrelated controls. A witness is never an implicit waiver of that refusal. Projection-only failure after successful log settlement MAY produce Settled at the actually readable relationship revision, with an immutable ProjectionDegraded carrying the original GrantError and that revision. If revision cannot be read, preserve and observe both the original projection failure and the revision-read error and return the latter named error; no zero or cached revision substitutes. Open/startup retains and reports projection degradation rather than discarding it; startup consumes that state rather than hiding it in a second project attempt. Metadata is in-memory decision/startup state, never a persisted grant event or permission.

**Acceptance:**
- RED-1 observes exact stage/original cause/revision on every failing projection attempt and no healthy warning.
- RED-2 returns the exact reconciliation error and writes no use event for the formerly unrelated control.
- RED-3 captures the original boot projection error; the service startup consumes it explicitly.
- Two simultaneous failures remain separately named; revision/read failure creates no frame and no fabricated revision.

**Files:**
- create: crates/lys-identity/src/grants/settlement.rs
- create: crates/lys-identity/tests/grant_settlement.rs
- create: crates/lys-identity-server/src/grant_settlement_tests.rs
- modify: crates/lys-identity/src/grants/frame.rs
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/src/grants/mod.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.

**Stories:**
- S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

### R2: Prove projection-degraded authority from the selected chain

THE SYSTEM SHALL keep DIRECTORY-006 R3 availability for projection-only failure only when the current owner has a complete settled folded book, a successful read of the actual selected relationship revision, and a valid Frame reading. For each candidate use effective(book,directory,grant,at), which derives lineage and checks every grant is present, unrevoked, in-window, active and responsibly bound (admission.rs:183-209). Use fresh(path,projected), which requires each recorded issue or latest revoke/use-spend index to be strictly below projected (authority.rs:378-394; projection.rs:313-325), then confirm the required relationships at that reading. Frame folded revision must still equal the current owner. Missing evidence refuses by name; a different known unrelated grant is allowed only if its entire chain independently passes. A readable revision or disjointness from the single Uncertain grant is not sufficient evidence. Do not rescan all events or build another all-record delta solely to prove unrelatedness. Any caller supplying at_least retains StaleDecision {required,projected} when the selected known revision is too old; do not rename it to a generic outage.

**Acceptance:**
- CONTROL-1 preserves both exact old StaleDecision and Revoked assertions at grant_revocation.rs:227-243, plus count/reopen/retry assertions.
- CONTROL-2 preserves every original fault boundary and projection-only unrelated control, with named metadata added.
- Ancestor revoke and one-time spending on any hop refuse; another fully proved chain may succeed with its marker.
- Unreadable revision, missing chain, inconsistent selected frontier and stale Frame each refuse, with no event/use side effect.
- No extra full-history, all-grant or all-relationship copy is introduced by the proof beyond existing Frame/project behavior.

**Files:**
- modify: crates/lys-identity/src/grants/frame.rs
- modify: crates/lys-identity/tests/grant_settlement.rs
- modify: crates/lys-identity/tests/grant_revocation.rs
- modify: crates/lys-identity/tests/grant_faults.rs
- modify: crates/lys-identity/tests/grant_reach_frame.rs

**Checklist:**
- C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.
- C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

### R3: Carry degradation through answers and report it

THE SYSTEM SHALL carry one shared immutable projection-degradation value from Settled through Frame into each Permit made against it. Public response extension: optional degraded {step:"project",refusal:<existing stable name>,revision:<actual selected u64>}; raw store text, keys, signed bytes and credentials are never included in that view. The original typed error remains available inside the authority and to the controlled operator log. Healthy responses omit degraded, and every actual grant-answer owner, including PermitView, batch answers and summarized reach/which answers, carries the marker of the reading it used rather than inferring a healthy state from allowed:true. Startup and every failing settlement attempt reach the existing tracing/operator observation route; no log coalescing hides a failure. Query paths that consume the Permit internally retain the decision metadata through their authorization context and its observation point. Existing denial names and at_least semantics stay stable. The Rust Permit field and JSON/OpenAPI/TypeScript extension are explicit source/public response changes. Add release notes and exercise legacy healthy JSON consumers and current generated consumers; do not claim Rust source compatibility for an added public field. Any incompatible wire-policy change beyond the stated optional marker stops for review.

**Acceptance:**
- CANDIDATE-1 checks library and actual serialized grant answers for correct metadata and exact selected revision.
- Healthy and recovered requests omit degradation; separate service instances never share the state.
- Original error details reach controlled operator observation; public output contains only stage/stable refusal/revision and no private sentinels.
- One observation is made for every actual failed projection attempt, including attempts ending in StaleDecision/Revoked; no timed wait or log-rate bypass.
- OpenAPI, views, generated TypeScript and every affected response family serialize the same optional marker. A legacy healthy-answer JSON consumer reads unchanged healthy output; a current consumer reads degraded output. Release notes state the Rust public-struct field addition and the optional JSON extension; no compatibility claim is substituted for those controls.

**Files:**
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/src/grants/frame.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/grant_contract/views.rs
- modify: crates/lys-identity-server/src/grants_batch.rs
- modify: crates/lys-identity-server/src/grants_reach.rs
- modify: crates/lys-identity-server/src/who_can_grant.rs
- modify: crates/lys-identity-server/src/openapi.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/grant_settlement_tests.rs
- modify: surface/identity/src/generated/grants.ts
- modify: CHANGELOG.md
- modify: crates/lys-identity-server/src/grants/handlers.rs

**Checklist:**
- C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.
- C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

### R4: Propagate the four grant decision errors

THE SYSTEM SHALL replace the four explain(...,None).is_ok() collapses at goals_api.rs:374-375, grants_refusals.rs:148-149, runner_sessions.rs:157-158 and teams_migration.rs:146-147 with explicit successful-proof handling and propagation of every original GrantError as ServerError::Grant. Never substitute false or NotPermitted for a grant failure. Keep successful projection-degraded proofs usable with metadata. Business branches that do not evaluate a grant remain explicit, and every downstream logical-denial behavior change caused by this ruling is reviewed and tested; no catch-all boolean conversion hides it.

**Acceptance:**
- RED-4 has four independently named cases, one per boundary, asserting original stable refusal and no privileged operation side effect.
- Each boundary has a successful healthy proof and a successful proved-unrelated projection-degraded control, both through its actual grant owner.
- Existing non-grant authority/business branches are preserved; changed logical-denial behavior is stated, not hidden.

**Files:**
- modify: crates/lys-identity-server/src/goals_api.rs
- modify: crates/lys-identity-server/src/grants_refusals.rs
- modify: crates/lys-identity-server/src/runner_sessions.rs
- modify: crates/lys-identity-server/src/teams_migration.rs
- modify: crates/lys-identity-server/src/grant_settlement_tests.rs
- modify: crates/lys-identity-server/src/spicedb.rs

**Checklist:**
- C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.
- C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.
- C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

### R5: Retain mutation-acknowledgement failures without revision zero

WHEN Grants::answer cannot project a committed operation THE SYSTEM SHALL retain and observe the original typed projection error using the same named degradation route as R1, rather than the Err(_) branch at commit.rs:199-201. It SHALL read the actual permission revision fallibly. IF that read fails THE SYSTEM SHALL retain both distinct causes, return the original named revision-read error and issue no Recorded acknowledgement; it SHALL NOT replace a failed read with0, a cached revision or ProjectionPending inferred from invented data. IF the actual revision is readable THE SYSTEM SHALL preserve the existing truth test: only projected>receipt.index can acknowledge the existing Recorded {event,index,receipt}; a known projected<=index returns the exact ProjectionPending {operation,grant,index}. An acknowledgement after projection failure carries the shared degradation in its operation-result view and operator observation, so it cannot imply healthy settlement. It SHALL preserve the signed operation identity, event/receipt/index equality, one-event retry/reopen behavior and changed-payload refusal. It SHALL NOT append a replacement operation to resolve an unknown reply. No stored receipt/event or wire-number encoding changes are introduced.

**Acceptance:**
- MUTATION_CAUSE_RED: with a known committed operation, fail projection while the actual revision remains readable; assert the original typed cause reaches controlled observation and any resulting operation-answer marker names that actual revision. The old silent Err(_) branch fails this behavior assertion.
- MUTATION_REVISION_RED: make both projection and revision-read fail with distinct instance-private sentinels; assert both observations, the exact revision-read refusal, no Recorded acknowledgement and no revision0. Baseline substitutes ProjectionPending from0 and fails the existing-interface refusal assertion.
- With readable revisions at receipt.index and receipt.index+1, assert exact ProjectionPending at the former and the same Recorded event/index/receipt at the latter. Marker-only assertions are added after the API exists and never called a baseline behavior red.
- Retry the identical operation before and after reopen; the receipt/index and total authorising-event count stay identical. Changed payload under that id keeps the original refusal and changes neither projection. Every original GRANT_IDEMPOTENCE and GRANT_AUDIT assertion remains.

**Files:**
- modify: crates/lys-identity/src/grants/commit.rs
- modify: crates/lys-identity/src/grants/settlement.rs
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/tests/grant_settlement.rs
- modify: crates/lys-identity/tests/grant_faults.rs
- modify: crates/lys-identity-server/src/grant_contract/views.rs
- modify: crates/lys-identity-server/src/grant_settlement_tests.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/openapi.rs
- modify: surface/identity/src/generated/grants.ts
- modify: CHANGELOG.md

**Checklist:**
- C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.

**Stories:**
- S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

### R6: Deliver a complete authenticated tail witness

THE SYSTEM SHALL deliver a complete, verifiable, ephemeral tail-witness capability for the real FileLeafStore and GrantLedger, with a named witness-verification API and owned integration tests in this series. A witness binds the immutable log identity, the exact trusted settled frontier (size and root), the entire contiguous appended range up to a freshly certified upper frontier (size and root), the authenticated signed event at every included index, and the append-coherence evidence that makes that bound valid for its stated reading. It SHALL identify all authority effects of every included grant event, including ancestor issue/revoke, one-time spending, and every change that can invalidate the selected lineage; incomplete, unknown or undecodable effects cannot prove disjointness. Verification uses the actual log/checkpoint/signature and current folded-owner lineage, not caller-supplied grant names or a single Uncertain.event. A second writer's append after acquisition invalidates an older witness before any later decision relying on it; the provider must prevent or detect that race across every writer through a tested ownership/coherence protocol. FileLeafStore's existing per-act head lock, a cached extent/pin, or this process refusing another append is not evidence of a lifetime writer lease. Keep ordinary LeafStore's opaque-byte responsibility; supply witness capability explicitly at construction, not through downcasts or a default method that fabricates a witness. A store without the capability returns its existing named failure for an attempted witness and cannot grant availability by omission. The file-backed provider and instance-private fault provider both implement the full capability; no stub remains. Tail verification reads only the bounded tail since the selected settled frontier and uses existing lineage/index information, without replaying all history or copying every grant/relationship. This requirement does not authorize using a witness to turn a log-reconciliation error into a permission success: R1's original-error refusal remains the admission rule in this brief. A separately reviewed change may consume the proved witness for unrelated availability later. No witness is persisted and no file/event/snapshot encoding is changed.

**Acceptance:**
- TAIL_COMPLETE: create two real store handles for one log, retain one unresolved local operation, append a second independently signed event through the other handle, and acquire a witness. Assert both tail events and their exact indexes/frontier are represented; a witness of only the held Uncertain fails verification.
- TAIL_RACE: use an owned barrier/event to append a relevant ancestor revocation through the second handle between witness acquisition and later verification. The old witness refuses by its original named coherence/store error; no permission or use event follows. No sleep, timed polling or invented exclusive lease.
- TAIL_INTEGRITY: independently corrupt each log identity, lower frontier, upper frontier, omitted/duplicated/reordered index, event signature and claimed authority effect in owned in-memory inputs. Every invalid case refuses; the complete unmodified real-file witness verifies.
- TAIL_AUTHORITY: compare verification to replay for direct grant, every ancestor, one-time spending and a fully independent chain. A complete unrelated tail proves disjointness; a relevant or unclassifiable effect cannot do so. Counters name the tail length and exact lineage visits.
- TAIL_FAILURE: fail real tail read or capability verification; assert its existing original named failure and no fabricated empty tail. A generic store lacking the explicit capability does not produce a witness. Fault-free real-file and fault-provider controls prove a complete implementation.
- TAIL_ADMISSION_RULE: even with a separately verified disjoint witness in hand, inject log-reconciliation failure into the actual permission read and assert R1 returns the original error. No implicit admission-policy change ships in this requirement.
- Witness/provider APIs and tests are new-interface proof, not baseline behavior reds. Reopen the same existing file-store data without changing bytes and verify a freshly acquired witness; an old in-memory witness is not silently reused across owner/reopen changes.

**Files:**
- create: crates/lys-log-store/src/witness.rs
- create: crates/lys-log-store/src/file/witness.rs
- create: crates/lys-log-store/tests/tail_witness.rs
- create: crates/lys-identity/src/grants/tail_witness.rs
- modify: crates/lys-log-store/src/lib.rs
- modify: crates/lys-log-store/src/store.rs
- modify: crates/lys-log-store/src/file.rs
- modify: crates/lys-log-store/src/file/head.rs
- modify: crates/lys-log-store/src/file/open.rs
- modify: crates/lys-log-store/src/error.rs
- modify: crates/lys-identity/src/restart.rs
- modify: crates/lys-identity/src/grants/recovery.rs
- modify: crates/lys-identity/src/grants/mod.rs
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/src/grants/frame.rs
- modify: crates/lys-identity/tests/grant_settlement.rs
- modify: crates/lys-identity/tests/grant_faults.rs

**Checklist:**
- C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.
- C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.
- C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

### R7: Bound costs and qualify the exact implementation later

THE SYSTEM SHALL add no background watcher, clock wait, timer retry, healthy-path allocation, extra disk sync or full-history proof loop. A degraded Frame/Permit shares one bounded immutable error value; do not clone the whole grant book, model, application state or relationship set for metadata. Observation uses the existing logger rather than a new unbounded queue or worker; its actual callback/lock/I/O costs are measured and disclosed. New fixtures use owned fault signals/instance state and always release/join resources before assertions, retaining cleanup errors. Test duration greater than two seconds is an open defect, never tuned away. Source queries, a published brief and test definitions are not runtime proof. Build work needs a separate capacity/handover and runs off this Mac; no runtime suite is authorized by this document-writing box. Tail-witness acquisition is a separately counted bounded-tail operation on explicit demand, not a hidden healthy permission-read scan. Record its ownership/coherence locks and release at the reading boundary; never hold a lifetime writer lock or background scanner.

**Acceptance:**
- Focused remote tests report full method/case counts, exact red then green exits and durations; candidate-only API tests are never mislabeled baseline reds.
- Strict all-targets/all-features touched-crate Clippy and formatter qualify the exact pushed SHA before any gate; unrun commands stay named.
- Measured healthy/degraded operation counts name locks, shared/error copies, disk syncs, lineage visits and existing project all-record/pending-event loops; new metadata/history costs satisfy the stated bounds.

**Files:**
- modify: crates/lys-identity/tests/grant_settlement.rs
- modify: crates/lys-identity/tests/grant_faults.rs
- modify: crates/lys-identity/tests/grant_revocation.rs
- modify: crates/lys-identity/tests/grant_reach_frame.rs
- modify: crates/lys-identity-server/src/grant_settlement_tests.rs
- modify: crates/lys-log-store/tests/tail_witness.rs

**Checklist:**
- C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.
- C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.
- C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

## Boundaries

- This publication writes only DIRECTORY-090.json/.md, ten additions-only structure declarations in directory/design.json and its rendered DESIGN.md. Every preexisting design object remains byte-equal. Requirement file walls name later implementation ownership; no source, test, manifest, stored data or other brief changes in this box.
- DIRECTORY-087 through089 are already taken. Fresh origin/main7e094d014f727b1f252cd3f50fd4cd801e6bcb46 currently ends at086. Under the approved publication order,090 is committed above089 at eab3d775 on existing lane/crumpet-grant-stream; the lead brings089 then090 into main only after83c73f12 is qualified and published. No main source is published early by this document push.
- This explicitly amends DIRECTORY-006 R3/C24/S10, preserving its original append/sync/replay/idempotence/audit duties. The grant_faults.rs:355 log-unreadable unrelated-success control becomes original-error refusal because current tail completeness is unproved; projection-only unrelated control:380 remains successful with the marker. No acceptance or fault leg is silently removed.
- Keep grant_revocation.rs:227-243 exact: Some(required) gives StaleDecision {required,projected:engine.revision()?}; None gives Revoked {grant:root}. Add degradation observation, never replace those names with a generic project outage.
- All metadata and witnesses are ephemeral. Existing event, snapshot, log-file, relationship and signed receipt bytes are unchanged; no migration or numeric wire code is invented. Any actual stored-shape change stops for an explicit migration brief and old-install tests.
- The marker is an explicit public Rust/JSON response extension carried through Permit, views, batch and summarized responses, operation acknowledgements, OpenAPI and generated clients. Existing stable refusal names and at_least semantics remain. No new generic error, catch-all boolean conversion or public raw storage detail substitutes for the original error.
- R6 is a complete named evidence deliverable in this series. It does not authorize an admission exception for failed reconciliation; witness-based unrelated availability needs a separate review after the proof exists. Cross-process coherence must be proven by its actual provider, never inferred from a per-act file lock, one uncertain event or a cached head.
- No broader grant/identity/authority rewrite, permission-engine replacement, generic log storage format change, new logger worker/queue, background watcher, full-history proof scan, sleep/retry timer or test assertion removal is authorized.
- No Cargo, runtime tests, Clippy, full build, full gate, deployment or hospital/school qualification occurs in this brief box. Source queries and design validation do not establish runtime correctness or test duration.

## Verification

- Write and run existing-interface behavioral reds before each bounded implementation: seven accepted settlement/boot/server cases plus both mutation acknowledgement cause/read cases. Keep baseline controls separate; metadata/witness API absence is not a red. Preserve full assertion, case, fault-boundary and event-count populations.
- At the declared source parent verify every create target is absent, every modify target exists or is created by an earlier requirement in this same brief, and no undeclared file is needed. R1 creates the shared settlement/test modules before their later modify requirements. No delete target is named.
- Format the hand-authored JSON with the repository-compatible JSON formatter, render using scripts/design/render-brief.py and read back every JSON string leaf in the Markdown. Validate the four documents through scripts/design/gate.sh on an owned exact-parent-plus-four-path docs/scripts candidate. Retain full stdout/stderr and exit, document/cluster/warning/failure counts, while displaying only parsed failures; require exit0 and no failures.
- Use an isolated index based on eab3d775 and add exactly the two owned brief paths, additions-only directory/design.json and rendered DESIGN.md. Read back the committed parent/tree/blobs and ensure the gated candidate matches. Preserve shared main/index/foreign WIP. Push only lane/crumpet-grant-stream, read the exact remote commit/tree, and report the ordered main-publication handoff rather than claiming it occurred.
- Later implementation runs the formatter itself and strict all-targets/all-features touched-crate Clippy on the approved remote venue before its pushes. Run focused nextest on every changed behavioral test at the exact source SHA; report every test duration and full counts. A case over2seconds remains a defect. No timeout is raised and no wait is replaced by polling.
- Report measured healthy/degraded/witness costs: each held lock and scope, shared error copies versus whole-state clones, physical disk syncs per call, lineage visits, existing project all-record/pending loops and tail length. Extra healthy allocation, metadata sync and history-scan counts are zero; witness work is bounded to its explicit tail/reading. Logger callback costs are measured, not assumed.
- The lead assembles all written source, discloses what is in and out, then runs the one remote gate after required focused qualification. Unrun commands, unknown exits, unmeasured costs, landing, installation and runtime readiness remain explicitly unconfirmed.
