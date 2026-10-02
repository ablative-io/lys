---
type: brief
id: DIRECTORY-065
cluster: directory
title: Codex uses the Lys policy and shows source-backed refusals
---

# DIRECTORY-065: Codex uses the Lys policy and shows source-backed refusals

> **Cluster:** directory
> **Depends on:** DIRECTORY-051, DIRECTORY-062, DIRECTORY-064
> **Design anchor:**
> - ADR-131 — Codex policy comes from Lys and refusal provenance follows the real harness contract — Render the same Lys policy into isolated native Codex settings. Use064's one transport owner and051's one refusal store. Distinguish Codex-reported rejection, Lys judge denial and062 OS denial. Required unrepresentable policy refuses launch. Coverage is capability-derived, never a blanket claim.
> **Checklist:**
> - C439 — Pin the actual Codex executable and its supported policy contract (DIRECTORY-065 R1).
> - C440 — Render native Codex permissions from the bound Lys policy (DIRECTORY-065 R2).
> - C441 — Bind Codex pre-tool policy checks to the existing Lys judge (DIRECTORY-065 R3).
> - C442 — Record native Codex rejections with honest provenance (DIRECTORY-065 R4).
> - C443 — The Codex agent page shows measured policy and refusal coverage (DIRECTORY-065 R5).
> - C444 — Prove config enforcement and denial delivery through the real harness (DIRECTORY-065 R6).
> **Stories:**
> - S175 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As a person running Codex under Lys, I want its native permissions to reflect my Lys policy without hand-editing settings.
> - S176 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As the person responsible for an agent, I want real refusals and missing coverage shown separately, so I can trust what the page tells me.

## Purpose

Tom via Waffles, 29 September 2026 07:32 Melbourne. Card lRpNSAg7. Close051's Codex gap without inventing a notify denial event or treating every failed command as a policy refusal. Gypsy is the non-writer reviewer.

## Task

Extend the signed policy and existing harness owner. Render Codex native config, bind native and Lys refusal evidence, and replace the blanket unsupported display with measured capabilities. Independent implementation branches may build in parallel under Tom's07:32 ruling. Integration must contain the actual051/062/064 seams and pass the fresh full chain before landing.

## Requirements

### R1: Pin the actual Codex executable and its supported policy contract

Behavioural. Build on DIRECTORY-051's isolated Codex config writer and DIRECTORY-064's single app-server transport owner. Record the actual executable path, package/build identity and measured config/event adapter version for each launch. Ground this adapter in codex-channels source bd3798faee33820aad0044ed732841261d2bbe15, the commit named by the installed 0.0.0-channels.3 package manifest. The relevant source files are unchanged at 7cbba483f7c83aed15c5fbc0258c72651b6fdf91. A PATH codex-cli reporting 0.36.0 is not that package, and the package's --version reports only 0.0.0. Neither version string proves compatibility. Verify the installed artifact against its trusted package record and exercise its config and app-server schema in the capability check. Unknown builds refuse codex_policy_contract_unsupported before starting an agent requiring this policy. Do not download or replace a harness implicitly.

The source contract is explicit. hooks/src/legacy_notify.rs carries only agent-turn-complete after a turn. config/src/config_toml.rs and permissions_toml.rs define sandbox/approval/permission profile keys. hooks/src/events/pre_tool_use.rs supports structured PreToolUse outside legacy notify. core/src/tools/events.rs maps some runtime/setup rejections to Declined and sandbox errors to generic failure. rollout/src/policy.rs omits transient ExecCommandEnd and HookCompleted from durable rollout history. Consequently neither notify nor arbitrary rollout strings prove a refusal. Record these limits in CODEX-POLICY-CONTRACT.md with commit and file:line references and committed redacted fixtures. The adapter refuses an unsupported required signal rather than inferring it.

**Acceptance:**
- The capability receipt names the actual executable and trusted package identity.
- A 0.36.0 executable cannot pass as the measured bd3798fa build.
- An unknown config key fails the native strict-config check.
- An unsupported event schema returns codex_policy_contract_unsupported.
- A turn-complete notify payload creates no refusal.

**Files:**
- create: crates/lys-runner/src/codex_policy_contract.rs
- create: crates/lys-runner/tests/codex_policy_contract.rs
- create: docs/design/directory/CODEX-POLICY-CONTRACT.md
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/harness_control/codex.rs
- modify: crates/lys-home/src/harness/codex/config.rs
- modify: crates/lys-home/src/harness/codex/config_tests.rs

**Checklist:**
- C439 — Pin the actual Codex executable and its supported policy contract (DIRECTORY-065 R1).

**Stories:**
- S175 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As a person running Codex under Lys, I want its native permissions to reflect my Lys policy without hand-editing settings.

#### R1 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. None of the acceptance rows can be met. Missing prerequisites: crates/lys-runner doesn't exist (so there is no src/lib.rs or src/harness_control/codex.rs to extend, and no DIRECTORY-064 app-server transport owner), and crates/lys-home/src/harness/codex/config.rs doesn't exist (no DIRECTORY-051 isolated config writer; the directory only has account, beside, leaf, parts, rollout and zone). The pinned codex-channels source bd3798fa, the 7cbba483 cross-check and the 0.0.0-channels.3 package aren't on this machine; the installed package is standalone 0.145.0 (~/.codex/packages/standalone/current/codex-package.json). Row by row: the capability receipt naming the executable and trusted package isn't met, because there's no launch path and no trusted package record for the pinned build. Refusing a 0.36.0 executable isn't met, because the measured bd3798fa build isn't installed to measure. Failing an unknown config key isn't met, because there's no native strict-config check to run against the pinned parser. Returning codex_policy_contract_unsupported for an unsupported event schema isn't met, because there's no 064 event projection. A turn-complete notify creating no refusal isn't met, because there's no 051 refusal store. CODEX-POLICY-CONTRACT.md was deliberately not written: its file:line references to hooks/src/legacy_notify.rs, config/src/config_toml.rs, permissions_toml.rs, hooks/src/events/pre_tool_use.rs, core/src/tools/events.rs and rollout/src/policy.rs would be unchecked claims without the source. To unblock: land DIRECTORY-050/051/064, and install the codex-channels 0.0.0-channels.3 package and its source at bd3798fa on the venue.
- Deviation: No file created or modified. The prerequisites are named instead of recreated, per the SHALL NOT boundary on absent prerequisite implementations.
- Checklist delivery:
  - [ ] C439 — Pin the actual Codex executable and its supported policy contract (DIRECTORY-065 R1). — Blocked on DIRECTORY-051/064 code and on the pinned codex-channels package and source being absent.
- Story delivery:
  - [ ] S175 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As a person running Codex under Lys, I want its native permissions to reflect my Lys policy without hand-editing settings. — Blocked: no Codex launch path or config writer exists to carry the policy.

### R2: Render native Codex permissions from the bound Lys policy

Behavioural. Extend the config.toml writer in the isolated CODEX_HOME. Derive settings from the same policy version and digest as DIRECTORY-062's signed containment plan, not a second allow list. Use one named permissions profile selected by default_permissions. Its filesystem entries use the native read, write and deny vocabulary, and its network domains use native allow/deny entries. Only plan-owned writable roots are writable. Runtime reads and permitted hostnames come from the plan. Root, temporary-directory, proxy, loopback and Unix-socket exceptions are explicit and never blanket. Compile the complete policy or refuse codex_policy_unrepresentable naming the unsupported rule. A setting that only approximates a rule cannot silently replace it. Keep native sandboxing enabled. Never generate danger-full-access, a bypass CLI flag or global hook-trust bypass.

For unattended managed sessions set approval_policy to never, meaning reject escalation rather than approve every act. The sandbox still applies. A policy requiring interactive escalation is refused until a separately specified Lys approval flow exists. Existing grants may change the next server-authorised plan but a Codex approval cannot override a hard rule. Preserve 051 notify and allowed provider/MCP settings. Render atomically and idempotently, with no changes to the person's global home. The runner controls launch overrides, profile selection and resumed-thread settings. Reject project config, profiles, environment or launch overrides that would widen the bound policy. Read back effective native settings through the 064 app-server connection before accepting the session as protected. Configured is not enforced. Keep the config, plan and credentials outside the agent's writable roots using 062. A policy change uses 062's stop/exit/relaunch rule and is never a silent live config rewrite.

**Acceptance:**
- The rendered permission profile names exactly the signed plan writable roots.
- A denied path is rendered with native deny access.
- A denied hostname remains denied beside an allowed hostname sharing its address.
- The native config parser accepts the generated TOML.
- An unrepresentable rule refuses launch by rule id.
- approval_policy never does not disable sandboxing.
- A wider project-level permission profile refuses launch.
- A resume carrying wider permission settings refuses launch.
- The global user config is unchanged.
- The effective permission readback matches the signed policy digest.

**Files:**
- create: crates/lys-home/src/harness/codex/policy.rs
- create: crates/lys-home/src/harness/codex/policy_tests.rs
- modify: crates/lys-home/src/harness/codex/config.rs
- modify: crates/lys-home/src/harness/codex/config_tests.rs
- modify: crates/lys-home/src/harness/codex/mod.rs
- modify: crates/lys-runner/src/harness_control/codex.rs
- modify: crates/lys-runner/src/containment_policy.rs
- modify: crates/lys-identity-server/src/agent_policy_store.rs
- modify: crates/lys-identity-server/src/containment_policy.rs

**Checklist:**
- C440 — Render native Codex permissions from the bound Lys policy (DIRECTORY-065 R2).

**Stories:**
- S175 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As a person running Codex under Lys, I want its native permissions to reflect my Lys policy without hand-editing settings.

#### R2 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. None of the ten acceptance rows can be met. Missing: DIRECTORY-062's signed containment plan (no crates/lys-runner/src/containment_policy.rs, crates/lys-identity-server/src/containment_policy.rs or agent_policy_store.rs), DIRECTORY-051's CODEX_HOME config writer (no crates/lys-home/src/harness/codex/config.rs), and DIRECTORY-064's app-server connection for the effective-settings readback. Without the signed plan there are no writable roots, deny paths, hostnames or digest to render, compare or refuse against. Writing a separate allow list in policy.rs would be the 'second allow list' the spec forbids.
- Deviation: No files created. crates/lys-home/src/harness/codex/policy.rs was not made standalone, because it has to derive from 062's plan, which doesn't exist.
- Checklist delivery:
  - [ ] C440 — Render native Codex permissions from the bound Lys policy (DIRECTORY-065 R2). — Blocked on DIRECTORY-062 plan, 051 config writer and 064 readback.
- Story delivery:
  - [ ] S175 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As a person running Codex under Lys, I want its native permissions to reflect my Lys policy without hand-editing settings. — Blocked.

### R3: Bind Codex pre-tool policy checks to the existing Lys judge

Behavioural. Use the measured native PreToolUse contract where supported to cover tool-specific Lys rules that native filesystem/network settings cannot express. Extend 051's judge adapter with explicit Codex tool-name and structured-input mappings read from core/src/tools/handlers. Do not assume Claude and Codex payloads are identical. Codex exec_command maps to Bash, apply_patch carries patch input, and resumed write_stdin does not emit a second PreToolUse in the pinned source. Uninspectable effects remain denied under 051's policy. Reuse peer.rs to prove the caller's socket credentials, ancestry and leader start identity. No hook-supplied agent id establishes authority. Reuse the same current-grant checks and durable refusal-before-deny rule. Return the native hookSpecificOutput PreToolUse deny with a nonempty permissionDecisionReason. Do not emit allow overrides. Install only the Lys-owned hook declaration and its narrowly scoped native trust entry, retaining other admitted hooks.

The source parser can fail open on malformed or unsupported hook output. Therefore capability proof must test the exact output, disabled/untrusted hook and disconnected judge. A required hard rule must remain enforced by the native plan or cause start refusal if the hook cannot prove fail-closed coverage. Do not claim that a telemetry hook alone contains an arbitrary process. Native OS containment remains062. A Codex version with usable native policy and native OS evidence may report those capabilities independently from unavailable pre-tool coverage. The page never replaces one blanket not-observed sentence with a blanket protected claim.

**Acceptance:**
- A proved Codex PreToolUse request reaches the same policy version as a Claude request.
- A denied attempt is durable before the native deny response.
- A caller from outside the session ancestry cannot create an attributed refusal.
- The actual Codex harness blocks a tool on the adapter deny response.
- A malformed deny response cannot pass the required coverage probe.
- A missing hook trust entry leaves pre-tool coverage unavailable.
- An unavailable judge cannot permit a required hard rule.
- A write_stdin continuation is not counted as a second hook observation.

**Files:**
- create: crates/lys-runner/src/codex_judge.rs
- create: crates/lys-runner/tests/codex_judge.rs
- create: crates/lys-home/src/harness/codex/hooks.rs
- create: crates/lys-home/src/harness/codex/hooks_tests.rs
- modify: crates/lys-runner/src/judge.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/peer.rs
- modify: crates/lys-home/src/harness/codex/mod.rs
- modify: crates/lys-home/src/harness/codex/config.rs
- modify: crates/lys/src/commands/runner_judge.rs

**Checklist:**
- C441 — Bind Codex pre-tool policy checks to the existing Lys judge (DIRECTORY-065 R3).

**Stories:**
- S175 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As a person running Codex under Lys, I want its native permissions to reflect my Lys policy without hand-editing settings.

#### R3 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. None of the eight acceptance rows can be met. Missing: DIRECTORY-051's judge adapter (no crates/lys-runner/src/judge.rs), its peer proof (no crates/lys-runner/src/peer.rs), and crates/lys/src/commands/runner_judge.rs. The durable refusal-before-deny store and the current-grant checks don't exist either. The Codex tool-name and structured-input mappings have to be read from core/src/tools/handlers at the pinned bd3798fa source, which isn't available here. The 'actual Codex harness blocks a tool' row needs the pinned package, which isn't installed.
- Deviation: No files created. Recreating the judge or peer proof would be the inconsistent recreation the boundary forbids.
- Checklist delivery:
  - [ ] C441 — Bind Codex pre-tool policy checks to the existing Lys judge (DIRECTORY-065 R3). — Blocked on DIRECTORY-051 judge and peer code, and on the pinned source.
- Story delivery:
  - [ ] S175 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As a person running Codex under Lys, I want its native permissions to reflect my Lys policy without hand-editing settings. — Blocked.

### R4: Record native Codex rejections with honest provenance

Behavioural. Subscribe to DIRECTORY-064's session-owned typed event projection in harness_control/events.rs. Do not create another app-server process, competing stdout reader or full transcript follower. Bind thread, turn and item ids to the runner's admitted session and policy digest. Record item/completed commandExecution declined and fileChange declined as Codex-reported rejection with reason classification unavailable unless a separate structured authority event identifies the cause. A declined status is not proof that a person declined or a Lys rule denied. Preserve an opaque source identity and safe summary without persisting raw commands or credentials. A generic failed status, stderr containing permission denied, nonzero exit, assistant prose or notify event is not a policy refusal. Native OS denial records from062 remain a separate source and are correlated without collapsing their identities. An authoritative Lys judge denial remains a Lys policy source, never renamed Codex kernel evidence.

Use064's bounded durable cursor handoff. Atomically append the refusal and advance its consumed cursor in051's refusal store. Duplicate delivery reuses the record. A gap or source loss becomes explicit incomplete coverage. If native history can replay a typed item, retain its identity on reconnect. Legacy history cannot recover every transient event, so never infer no refusals from its absence. Checkpoint/replay handles the exact native history mode and fails by name when it cannot recover coverage. A source event is persisted before its subscriber cursor is acknowledged. Unrelated session events never enter this agent's list.

**Acceptance:**
- A typed command rejection creates one Codex-reported refusal.
- A typed file-change rejection creates one Codex-reported refusal.
- A setup failure reported as declined is not labelled a policy denial.
- A failed command whose stderr says permission denied creates no policy refusal.
- Replaying the same thread/turn/item outcome creates no duplicate refusal.
- A crash after append before cursor acknowledgement replays once.
- A lost transient-event range is displayed as incomplete coverage.
- A forged notify denial creates no refusal.
- An event bound to another session is refused by name.

**Files:**
- create: crates/lys-runner/src/codex_refusals.rs
- create: crates/lys-runner/tests/codex_refusals.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/harness_control/codex.rs
- modify: crates/lys-runner/src/harness_control/events.rs
- modify: crates/lys-runner/src/refusals.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-identity-server/src/refusals_store.rs
- modify: crates/lys-identity-server/src/refusals_api.rs
- modify: crates/lys-identity-server/tests/refusals.rs

**Checklist:**
- C442 — Record native Codex rejections with honest provenance (DIRECTORY-065 R4).

**Stories:**
- S176 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As the person responsible for an agent, I want real refusals and missing coverage shown separately, so I can trust what the page tells me.

#### R4 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. None of the nine acceptance rows can be met. Missing: DIRECTORY-064's session-owned typed event projection and its bounded durable cursor handoff (no crates/lys-runner/src/harness_control/events.rs or protocol.rs), and DIRECTORY-051's refusal store (no crates/lys-runner/src/refusals.rs, crates/lys-identity-server/src/refusals_store.rs or refusals_api.rs; crates/lys-identity-server/tests/refusals.rs doesn't exist either). Building a separate event reader would break the SHALL NOT on a second app-server owner or event reader.
- Deviation: No files created. No competing event reader was built.
- Checklist delivery:
  - [ ] C442 — Record native Codex rejections with honest provenance (DIRECTORY-065 R4). — Blocked on DIRECTORY-064 events and cursor, and on the DIRECTORY-051 refusal store.
- Story delivery:
  - [ ] S176 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As the person responsible for an agent, I want real refusals and missing coverage shown separately, so I can trust what the page tells me. — Blocked.

### R5: The Codex agent page shows measured policy and refusal coverage

Behavioural. Replace051's unconditional Codex not-observed sentence with capability-derived fields. Show native config verified, native containment enforced, pre-tool policy observed and rejection stream complete as separate states with their actual evidence and failures. An unavailable capability keeps its named reason. A Codex-reported rejection whose cause is unknown says so in plain language. Show the effective policy version and digest, writable roots, network restrictions and the meaning of unattended approvals. The usual view uses plain words. Advanced details expose the generated non-secret settings and source/build identity. Keep the existing refusal component, visibility checks and push cursor. Never expose raw command text or secrets. An empty list with an incomplete source is not an all-clear. Required protection unavailable prevents start and names the corrective act.

**Acceptance:**
- A supported live Codex session no longer shows the unconditional not-observed sentence.
- A session missing pre-tool coverage displays that specific gap.
- An unclassified native rejection says its cause is unavailable.
- A disconnected source displays incomplete coverage.
- An unrelated viewer cannot read the refusal list.
- The advanced view shows only non-secret generated settings.
- Keyboard access reaches the policy and coverage details.

**Files:**
- create: surface/identity/tests/codex-policy.test.tsx
- modify: surface/identity/src/features/file/AgentRefusals.tsx
- modify: surface/identity/src/features/file/AgentPolicy.tsx
- modify: surface/identity/src/features/file/AgentContainment.tsx
- modify: surface/identity/src/api.ts
- modify: crates/lys-identity-server/src/containment_api.rs
- modify: crates/lys-identity-server/src/refusals_api.rs

**Checklist:**
- C443 — The Codex agent page shows measured policy and refusal coverage (DIRECTORY-065 R5).

**Stories:**
- S176 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As the person responsible for an agent, I want real refusals and missing coverage shown separately, so I can trust what the page tells me.

#### R5 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. None of the seven acceptance rows can be met. The components the spec says to keep and modify don't exist: surface/identity/src/features/file has no AgentRefusals.tsx, AgentPolicy.tsx or AgentContainment.tsx (051/062). There is no unconditional not-observed sentence to replace, and crates/lys-identity-server has no containment_api.rs or refusals_api.rs to carry capability fields, visibility checks or the push cursor. The capability states depend on R1–R4, which are all blocked.
- Deviation: No files created or modified.
- Checklist delivery:
  - [ ] C443 — The Codex agent page shows measured policy and refusal coverage (DIRECTORY-065 R5). — Blocked on 051/062 surface components and APIs, and on R1–R4.
- Story delivery:
  - [ ] S176 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As the person responsible for an agent, I want real refusals and missing coverage shown separately, so I can trust what the page tells me. — Blocked.

### R6: Prove config enforcement and denial delivery through the real harness

Evidence. Run the exact packaged Codex through064's transport against a deterministic local model fixture which emits the requested tool calls. No paid model call is needed and no real provider credential enters the test. A real harness, native parser and real sandbox perform the act. The controlled outside-root target is writable to the same user without the policy. Remove the control file and prove absence before the sandboxed attempt. Measure an allowed workspace write, the denied outside-root write, a native approval rejection and an ordinary failing command separately. The rejection and native OS denial reach the real agent page through authenticated routes. A failed command does not become a denial. Test config precedence, replay, malformed hook output, disabled hook, unsupported package and source gap. The browser assertions use received event/cursor checkpoints, not delays.

Register the native and browser legs in the repository gate on Dean for macOS and205 for Linux, reusing062's capability prerequisite. Both native legs and the full integrated battery must pass before claiming full coverage. A blocked venue is reported by name. Include exact executable package identity, source contract commit, Lys commit, policy digest and individual case receipts in PROOF-CODEX-POLICY.md. Config rendering alone is not proof. This brief supersedes051's blanket unsupported display only for configurations actually proved by these capabilities.

**Acceptance:**
- The actual native parser accepts the generated settings.
- The same-user unsandboxed control write succeeds.
- The contained outside-root write is denied.
- The allowed workspace write succeeds.
- A native Codex approval rejection appears on the real agent page.
- An ordinary failing command does not appear as a policy denial.
- The browser receives the native OS denial with062 provenance.
- The replayed event appears once after service reopen.
- The macOS leg names its exact package and policy digest.
- The Linux leg names its exact package and policy digest.
- An unsupported required capability makes its leg fail by name.

**Files:**
- create: scripts/identity-gates/codex-policy.sh
- create: crates/lys-runner/tests/codex_policy_native.rs
- create: crates/lys-runner/tests/support/codex_policy_fixture.rs
- create: surface/identity/tests/acceptance/codex-policy.spec.ts
- create: docs/design/directory/PROOF-CODEX-POLICY.md
- modify: docs/design/project.json
- modify: docs/design/directory/design.json
- modify: .land/gates.sh
- modify: surface/identity/vite.config.ts
- modify: surface/identity/package.json

**Checklist:**
- C444 — Prove config enforcement and denial delivery through the real harness (DIRECTORY-065 R6).

**Stories:**
- S176 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As the person responsible for an agent, I want real refusals and missing coverage shown separately, so I can trust what the page tells me.

#### R6 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. None of the eleven acceptance rows can be met. The evidence needs the exact packaged Codex (0.0.0-channels.3 at bd3798fa, not installed here; only standalone 0.145.0 is present), DIRECTORY-064's transport, DIRECTORY-062's capability prerequisite and native OS denial records, and R1–R5, none of which exist. The native legs on Dean (macOS) and 205 (Linux) can't be registered against code that isn't there. PROOF-CODEX-POLICY.md wasn't written, because it has to carry real case receipts, policy digests and package identities, and making those up would be claiming coverage without evidence.
- Deviation: No gate registration, test, script or proof document created. Registering legs for absent code would make the gate fail or pretend.
- Checklist delivery:
  - [ ] C444 — Prove config enforcement and denial delivery through the real harness (DIRECTORY-065 R6). — Blocked on the pinned package, 062/064, and R1–R5.
- Story delivery:
  - [ ] S176 (Person running a Codex agent, Sets policy and inspects its actual enforcement) — As the person responsible for an agent, I want real refusals and missing coverage shown separately, so I can trust what the page tells me. — Blocked.

## Boundaries

- SHALL NOT change Codex upstream source or silently install a different harness.
- SHALL NOT use notify as a denial hook or parse terminal prose as an authority.
- SHALL NOT weaken native sandboxing, bypass hook trust globally or turn approval_policy never into full access.
- SHALL NOT create a second app-server owner, event reader or usage-accounting owner.
- SHALL NOT invent a timeout, poll on a clock or type control text into a terminal.
- SHALL NOT change signed historical bytes, add unsafe Rust, bypass a lint or ignore a test.
- SHALL NOT write outside a requirement file wall. An absent prerequisite implementation is named, not recreated inconsistently.
- SHALL NOT claim protection or source coverage from settings alone.

## Verification

- The full integrated card chain passes Jev, fmt, strict Clippy, tests, docs, ast-grep, file-length and surface gates.
- Both native Codex policy legs pass on the same integrated commit with real harness and OS evidence.
- A non-writer reviews the source contract and the real-browser evidence.
