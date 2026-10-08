---
type: brief
id: ACCESS-006
cluster: access
title: One Lys-owned channel membership capability for every service
---

# ACCESS-006: One Lys-owned channel membership capability for every service

> **Cluster:** access
> **Depends on:** ACCESS-003, ACCESS-004, DIRECTORY-087
> **Blocked by:** Before implementation the lead assigns protocol/provider/adapter ownership and freezes the shared versioned membership contract/release/conformance vectors. No wire, grant, credential, stored-shape or public interface is edited by this authoring., Lys DIRECTORY-089 signed log/baseline/tail/pass-binding producer and DIRECTORY-090 honest projection/readiness prerequisites must be published/implemented/qualified. DIRECTORY-089 is not present in Lys origin/main85ebb07c; ACCESS-005 names it as an owed prerequisite, not a served API., Cambium CORE-046 R4 is the consumer contract at dbe9118c; CORE-047 owns role/grant seeding and DIRECTORY-087 the verified AI/responsible-person binding. A cross-repository dependency is named here, not mistaken for this repository own ACCESS-006 id.

## Purpose

Tom A3: dedicated channel membership functionality baked into all services. One Lys authority and shared capability, consumed by Cambium and liminal, avoids private membership lists, stale permission, parent/cursor leaks and grants inferred from mere sign-in. Three named services in the source A3 row are Cambium, liminal and Lys; Cambium is already briefed at CORE-046 R4. This pair covers the lys gap. Source cambium@dbe9118cb50164e3436fbf0d769bd33f01f580b0:docs/design/REQUIREMENTS-TOM.md:59, exact excerpt: `dedicated channel membership functionality baked into all services`.

## Task

Authoring START2026-10-08 20:35:35 AEDT, ceiling21:05:35; source base85ebb07c5a00921f2ab98a9fa4e017fc6617f144/tree7cd271fda5007b78c6bc76635bea19a99f03afba. Waffles e843e59a, Hermes8076023c/69a5aa00 close this two-repository docs-only box; each repository ACCESS-006 is a distinct brief id. Current origin/main is refreshed at publication, never overwrite another lead commit. Exactly this JSON/MD pair; existing access cluster/declarations cover all future paths, no bootstrap/checklist/story/register additions required because no new ids are cited. Seven local stories: decide scoped read/post; keep current authority; page safely; admit a guest; recover without stale allow; count bounded work; prove the same real capability in all three consumers. No code/build/grant/wire/install/model/aion/runtime execution. Isolated index/pathspec straight-main publication, foreign source/index preserved.

## Requirements

### R1: Own one versioned channel membership decision contract

THE SYSTEM SHALL provide the single Lys-owned channel membership capability consumed by Cambium CORE-046 R4 and liminal ACCESS-006. Reuse IdentityId, Resource and Action and the existing grant authority; a membership is effective scoped read/post authority, not another app-local roster or boolean role table. Contract version, grant-log identity/reset epoch, workspace resource, verified subject identity/kind, requested channel resource and action, and required minimum receipt revision are explicit logical fields. A successful response names allow or a stable typed refusal, the exact authoritative decision revision and effective resource scope; allow and refusal are correlated to that exact request. Read and post are independent actions: read alone never implies post. Identity, audience, workspace/resource binding, role/model revision and action validation precede lookup; an app may ask only for its registered kinds and authorised workspace. No caller-supplied display label, follow, mute, tag, readable ancestor, guessed id or aggregate cursor creates membership. Channel-specific grants and approved schema inheritance are evaluated by the one authority; a UI ancestor read is never treated as proof of child access. Preserve restricted placements and administrator approval of widened roles. No new signed wire or stored encoding is allocated by this docs box; the implementing owner pins and versions any new external representation before consumers use it. Source cambium@dbe9118cb50164e3436fbf0d769bd33f01f580b0:docs/design/cambium-core/briefs/CORE-046.json:124, exact excerpt: `Read and post are separate actions.`. Source cambium@dbe9118cb50164e3436fbf0d769bd33f01f580b0:docs/design/REQUIREMENTS-TOM.md:59, exact excerpt: `dedicated channel membership functionality baked into all services`. Source lys@85ebb07c5a00921f2ab98a9fa4e017fc6617f144:crates/lys-identity/src/grants/types.rs:134, exact excerpt: `pub struct Resource`. Source lys@85ebb07c5a00921f2ab98a9fa4e017fc6617f144:docs/design/PERMISSIONS-ACROSS-THE-STACK-2026-10-06.md:75, exact excerpt: `restricted = true`.

**Acceptance:**
- Red: independent fixtures drive identical workspace/subject/resource/read and post queries through the public Lys provider and both product adapters; exact decisions/revisions/refusals match, case count is asserted and no local membership policy substitutes.
- Red: a place-only reader can read its place, cannot post or read a sibling; a poster can post only in scope; parent read/follow/mute/tag/cursor and forged holder/audience/workspace each grant zero additional access.
- Red: restricted child is reached only by its admitted scope; explicitly approved schema inheritance works through actual grant reach, not through a broad workspace default.
- Red: unknown contract version, mismatched log identity, malformed identity/action/resource, retired app and minimum revision ahead of the provider are each refused with their original named cause; no permission is synthesized.
- Before implementation, lead freezes the shared logical contract/conformance vectors and wire representation, names its owner and exact release; current provider types are not claimed to implement this new capability.

**Files:**
- create: crates/lys-identity/src/grants/channel_membership.rs
- create: crates/lys-identity/src/grants/channel_membership_tests.rs
- modify: crates/lys-identity/src/grants/mod.rs

### R2: Judge under the grant owner at the current revision and never use stale permission

THE SYSTEM SHALL perform every membership decision through the existing grant owner and admitted directory/model projection at one named revision. The required revision is GrantReceipt::revision(), scoped to its grant-log identity; model/event format versions are not permission revisions. Refused reconciliation, degraded projection, unresolved mutation, unavailable relationship store, revoked chain or stale required revision returns its exact typed cause. Do not settle failed projection by reading a stale revision and reporting allow. Independent live grant chains are evaluated correctly; removing one never silently revokes another valid chain. Membership consumers receive a trusted decision baseline and contiguous authenticated changes/watermarks through the DIRECTORY-089 prerequisite; publication of Ready requires the same log identity, baseline/index/cursor agreement and required revision. A gap, reset or transport/authority failure withdraws Ready. Authority availability is part of each current decision, not a permission cached while its source is unavailable. Source lys@85ebb07c5a00921f2ab98a9fa4e017fc6617f144:crates/lys-identity/src/grants/receipt.rs:54, exact excerpt: `pub fn revision(&self) -> u64`. Source lys@85ebb07c5a00921f2ab98a9fa4e017fc6617f144:crates/lys-identity-server/src/grants_batch.rs:85, exact excerpt: `pub struct BatchAnswer`.

**Acceptance:**
- Red: apply an explicit revoke revision, then the next direct check, permitted-resource continuation and recipient page exclude or refuse the revoked right at that revision; no old allow survives the ordered fence.
- Red: reconciliation failure, stale model/directory projection and provider unavailability each return the original typed failure; an unrelated healthy workspace remains usable and no open-room default is selected.
- Red: revoking an ancestor removes only rights derived from it; an independent valid chain retains its exact effective action, with grant/revision evidence.
- Red: authenticated unrelated watermarks advance the cursor without revealing hidden grants; missing interval/reset/foreign log or bad signature refuses Ready until one verified baseline and tail agree.
- Red: race grant mutation, role/model revision and a check under held signals: exactly one named authority ordering is observed, and a decision never combines directory/model/grant revisions from different snapshots.

**Files:**
- create: crates/lys-identity/src/grants/channel_membership_index.rs
- create: crates/lys-identity/src/grants/channel_membership_index_tests.rs
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/src/grants/frame.rs
- modify: crates/lys-identity/src/grants/projection.rs

### R3: Page only permitted resources and a channel own recipients

THE SYSTEM SHALL expose bounded indexed permitted-resource and channel-recipient pages under the same capability and revision as R1. Resource enumeration binds workspace, verified subject and action; recipient enumeration binds workspace, exact channel and required action and is allowed only to an authorised app/service consumer. Requested positive count and encoded-byte limits are required validated operator/caller bounds, never guessed defaults or an unlimited response. A cursor is opaque, authenticated or looked up under its issued capability, and binds log identity, revision, holder/workspace/resource/action and stable continuation key; foreign, expired, reset or incompatible-generation cursors refuse by name. No cursor grants access. A continuation rechecks current authority before emitting rows, never reads a stale snapshot after revocation. Stable ordering, no duplicates, explicit completion and exact returned/refused/skipped counts are required. Indexes address one subject/action or one channel/action recipient set; a page never scans every workspace member or every historical grant. Resource names, counts and recipients outside the caller scope are not disclosed. The public route/schema/OpenAPI registrations derive from these one-owner typed request/result contracts; no independently mirrored consumer schema is introduced. The consumer client transport/binding release remains ACCESS-003 and the exact provider release prerequisite, with conformance required before ready.

**Acceptance:**
- Red: resource and recipient pages at limit B return at most B correctly authorised rows and a valid continuation; B+1, zero/overflow bounds and over-cap encoded bytes refuse before allocation.
- Red: enumerate two pages with a held revocation between them; revoked resource/recipient appears in neither later output nor hidden total/count, and the old cursor cannot preserve its permission.
- Red: cross-workspace/subject/action/channel cursor substitution and consumer without enumeration authority refuse without leaking resource/recipient existence.
- Red: distinct grant chains reaching one recipient produce one recipient; paging the complete stable set has no missing/repeated row, with exact emitted/skipped/refused counts.
- Red: increase unrelated members and retired grant history while holding the selected page fixed; provider rows/index probes remain at the same measured ceiling and full-roster/history scans are zero.

**Files:**
- create: crates/lys-identity-server/src/channel_membership.rs
- create: crates/lys-identity-server/src/channel_membership_tests.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/grants_batch.rs
- modify: crates/lys-identity-server/src/grants_reach.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/openapi.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/openapi_typed.rs
- modify: crates/lys-identity-server/tests/openapi.rs

### R4: Admit a scoped guest without widening the workspace

WHEN a verified person or AI has at least one current grant in a workspace THE SYSTEM SHALL provide the scoped admission decision CORE-046 R4 requires; no current grant refuses sign-in. This is an indexed existence decision under the same current authority, not a workspace-wide read/post grant and not enrolment inferred from a Lys account. Directory-087 owns the AI identity and signed responsible-person binding; do not infer the responsible person as the AI holder. Every subsequent channel act still requires its exact scoped action. App role/grant seeding remains CORE-047, not an implicit membership migration or administrator bypass here.

**Acceptance:**
- Red: a subject with one channel read grant is admitted, can read that channel and cannot read/post a sibling or workspace-wide roster.
- Red: an otherwise valid Lys identity with zero current workspace grants is refused; revoking the final grant changes the next admission/act at the named revision.
- Red: an AI is checked as its own verified identity, not as its responsible person or a seat label; substituting either produces zero inherited patient-channel access.
- Red: an owner/administrator succeeds only through actual admitted schema/grants; unknown role or unseeded workspace is a named refusal, never automatic open access.
- Count pin: one scoped existence query has zero full membership/history scans; rows/probes are instrumented at increasing unrelated roster sizes, not inferred from elapsed time.

**Files:**
- create: crates/lys-identity-server/tests/channel_membership_admission.rs
- modify: crates/lys-identity-server/src/grants_batch.rs

### R5: Restore only authoritative current indexes and state the retention boundary

THE SYSTEM SHALL restore membership projections from the existing authoritative grant/directory model checkpoint and only its bounded tail, with verified log identity and cursor; the capability owns no duplicate historical membership store. Current effective subject/action/resource and recipient indexes are derived state owned with the authority, updated in the same settled projection generation. Removing the final live chain prunes its active index entries; opaque cursor and pending consumer proof retention is finite and released on explicit completion/consumer closure/reset. An unavailable/invalid checkpoint is named; no silent start-from-history fallback or Ready over an incomplete index. If implementation adds a stored shape or changes a shipped signed/logged format, ship its versioned migration and an old-install test in the same lane. No shape is asserted new/unshipped without reading releases and installed data.

**Acceptance:**
- Red: two installs with equal current checkpoint/tail but different retired grant history read the same bounded rows and restore the same membership/recipient indexes, with zero history-prefix reads.
- Red: old-install fixture retains exact existing grants/roles/identities after upgrade and produces the same shared decisions; any new stored shape migration is exercised, not refused as old format.
- Red: corrupted/foreign checkpoint, partial index or wrong-log tail stays NotReady with its exact cause; unrelated authority domains continue.
- Red: revoke the final chain and close/reset cursor consumers; active index and retained cursor/proof counts return to their stated safe horizon, with no credential or patient-payload history retained.
- Red: mutation/index publication interrupted at each owned signal resumes to one authoritative generation; no mixed allow/deny index is served.

**Files:**
- create: crates/lys-identity-server/tests/channel_membership_recovery.rs
- modify: crates/lys-identity/src/grants/recovery.rs
- modify: crates/lys-identity/src/grants/projection.rs

### R6: Count authority work and keep failures separate from success

THE SYSTEM SHALL instrument logical capability calls, directory/model/grant owner reads, indexed rows/probes, page rows emitted/skipped/refused, physical store calls/flushes, lock acquisitions, retained entries and whole-state/history traversals. A point decision performs one logical capability call, no full roster/history traversal and zero added durable writes/flushes; physical read counts are measured at the real backend. Freeze the pre-fix measured count ceiling for selected decisions/pages and ratchet it downward; no assumed timing number stands in for work. Locks cover indexed state/authority decisions only and never hold an app-global lock over consumer network I/O; enumerate pages incrementally, no whole-state clone. Required count/byte/depth bounds are constructor/operator inputs with explicit validation. Ready/change/exit wakes are signals; no sleeps, timeout decisions, polling, detached retries, ignored/serial tests or lowered volume.

**Acceptance:**
- Red: the point/page matrices measure provider rows/index probes and locks under normal/degraded/revoked cases at three increasing unrelated history/roster sizes; all counts including zeros are retained.
- Red: point reads produce zero new durable grant/use events or physical syncs; do not call the existing receipted deliberate exercise API to disguise a read as a write.
- Red: only the exact affected grant/channel/subject indexes change after a revision, no whole authority clone or full-roster iteration.
- Red: idle consumers have zero registered readiness timers/polls; held update/exit signals wake their one owner and propagate the named original failure.
- The independent hospital vector forbids a patient resource name, recipient or body reaching any wrong subject after revocation; the correct subject control is still exercised and counted.

**Files:**
- create: crates/lys-identity-server/tests/channel_membership_counts.rs
- modify: crates/lys-identity-server/src/channel_membership.rs

### R7: Publish one real provider and qualify both consumers

THE SYSTEM SHALL publish the exact shared provider contract and conformance fixtures with the implemented Lys release before Cambium CORE-046 R4 and liminal ACCESS-006 are called ready. Complete tests start with observed behavioral reds, fixtures built once and owned ready/exit signals, preserving all failure/cleanup causes. This brief authoring is docs-only and does not run those tests or change grants. Full Rust/public-client gates and actual installed provider/consumer proof are later authorized jobs. No source-only subset, design gate, health response or config-only service is called membership working.

**Acceptance:**
- Future qualification runs the required formatter, strict default/all-feature Clippy, full nextest/doc/AST and public-client conformance on the exact pushed implementation commit at the authorised remote venue; all parsed counts/failures/skips/ignored/slow names are reported.
- Separately prove real Cambium door/direct/aggregate/unread/wire and liminal TCP/WebSocket delivery consume the same current membership decision, including failure and revocation while opened.
- Read back actual installed Lys/product binary/release/contract/log identity and capture real scoped admission/read/post/revocation bytes and rows; synthetic vectors are labelled test evidence.
- Authoring: full scripts/design/gate.sh parsed, exact paired render cmp0, file-plan and source-citation census0, diffcheck0, isolated pathspec main commit/push/remoteSHA/tree/bytes0; foreign code/index retained.
- END states A3 scoped denominator and percentage unbriefed, all unrun proof, each changed hot-path lock/clone/sync/history cost, and exact written/pushed/accepted/installed boundaries.

**Files:**
- create: crates/lys-identity-server/tests/channel_membership_conformance.rs

## Boundaries

- SHALL NOT create a product-local membership list, open-workspace default, implicit administrator bypass, guessed parent grant, label-based identity or stale allow fallback.
- SHALL NOT change code, grant issuance, stored/wire/signature format, runtime/install, public API or other brief/declaration in this authoring; no Mac Cargo/frontend build/test or aion writing workflow.
- SHALL NOT weaken CORE-046 R4, ACCESS-004 restricted/holder authority, ACCESS-005 grant ordering or Directory identity/revocation prerequisites; scope differences are stated and tested explicitly.
- SHALL NOT invent a provider release, head/log identity, numeric tag, default bound, physical count or observed behavioral red/green. Missing prerequisites stay blocked.
- SHALL NOT add clock waits, silent retries, ignored/serial cases, all-history/roster traversal, whole-state clones, or reduce assertions/fixture volumes.
- SHALL NOT equate written/pushed/design-gated with code-qualified, lead-accepted, landed/installed or actual membership traffic.

## Verification

- Full repository sh scripts/design/gate.sh at exact committed source; parse every document/cluster count, warning/failure and rendered comparison, not exit alone.
- Canonical paired renderer output cmp0; every requirement/acceptance/path/obligation in MD, exact future create/modify inventory from git ls-tree, all quoted rev:path:line source excerpts verified.
- Capture foreign index/WIP; private index pathspec commit only owned pair on current main after fetch/readback; normal push/ls-remote/fetch exactSHA/tree/allownedbytes; unpushedrefs0; remove private index output.
- Before implementing, publish exact missing producer/wire/authority semantics and consumer owners; record behavioral reds then full remote code/public client gates with all failures and slow cases.
- At handback report scoped A3 unbriefed services denominator3 (Cambium/liminal/Lys from source row), initially2/3; after both pairs published0/3 (0%). Implementation/runtime percentage remains separate and not measured by this docs box.
