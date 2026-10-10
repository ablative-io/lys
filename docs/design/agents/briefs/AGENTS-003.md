---
type: brief
id: AGENTS-003
cluster: agents
title: Import one seat's existing configuration by a confirmed, restartable plan
---

# AGENTS-003: Import one seat's existing configuration by a confirmed, restartable plan

> **Cluster:** agents
> **Depends on:** HOME-037, DIRECTORY-051, DIRECTORY-064, AGENTS-001, AGENTS-002
> **Blocked by:** DIRECTORY-064 qualification box: crates/lys-runner/src/harness_control/process.rs:26 has QUALIFIED = &[] at 71e44c78360d2b2228657dc5213026f920526fb9. Amendment 37's non-fixture launched-binary proof and final pin commit are owed. An import must never qualify an adapter or work around control_adapter_unqualified., AGENTS-002 R1 must create crates/lys-identity-server/src/seats_api.rs and its seat records before the importer publishes a seat. The importer extends the seat through that contract; it does not create a second seat registry.
> **Design anchor:**
> - ADR-130 — Managed harness channels carry automatic context and reminder acts — 064 adds one runner-owned typed control/event channel per managed Claude or Codex session. It reuses 051 operation and usage owners, serialises all inputs at proved boundaries, requires actual harness evidence and retains uncertain outcomes without automatic resend. Legacy PTY sessions remain manual and cannot claim this capability. 065 extends the same Codex event reader for refusal coverage.
> - ADR-136 — Permissions across the stack: Lys is the one authority; products decide hot routes from a signed pass and ask Lys for deliberate ones; a grant carries its mode — Lys holds every identity and grant; a product permission model is data in its app schema (ADR-116). Every product route is classified once as hot (decided offline from the pass) or deliberate (asked of Lys live, never falling back). The pass is the access token with a rights claim for its audience, short-lived, signed with the key Lys publishes. A grant carries one mode: outright, by_draft or by_two; a held act is a Lys draft the product executes after approval by reading Lys. Reach is by placement, never a pattern; a placement may be restricted. Roles are named bundles in the product schema; a widening by an app waits for the administrator. A machine is the fifth identity. One client library, lys-pass, with conformance fixtures every product gate runs. Availability is stated on each product screen; every deliberate act is receipted in Lys.
> - ADR-137 — Argus's remaining functions move into Lys as data beside goals, and a seat is a Lys record launched by the runner through the proxy; no shared library — Words (five slots, four layers, templates), variables (per agent and session, revision, author, expiry), schedules and seats are identity-server records in Lys's log, rendered and acted on at delivery through DIRECTORY-064's dispatcher with the contributing revisions in the receipt. No shared library: Lodestone is a read-only query engine over a store and holds nothing; haematite is the store under Lys already; a library would be a second place for the same data. Seat start, stop and restart are deliberate acts under Lys rights (ADR-136). Argus's alarms, rules, launches and queue are not moved into Lys: they become liminal services and aion work (ADR-136 section 3). A seat moves off herdr and Argus one at a time, Waffles's first.
> **Checklist:**
> - C701 — The five message slots (context warning, preparation, compaction, wake-up, scheduled reminder) resolve built-in, then workspace, then agent, then session, with named templates; an empty layer inherits (AGENTS-001 R1).
> - C702 — An agent's and a session's variables are JSON values with a revision, an author and an optional expiry; a stale revision is refused by name; the agent reads and sets its own through lys and the Lys MCP server (AGENTS-001 R2).
> - C704 — A schedule has at, until, interval, max occurrences and recipients; missed intervals coalesce to one send; an uncertain delivery stops it for inspection; every state survives restart (AGENTS-001 R4).
> - C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
> - C707 — The Sessions screen shows every seat online, offline, not seen by the runner, or seen but unregistered, from the runner's own liveness, never from a terminal program (AGENTS-002 R2).
> - C710 — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).
> **Stories:**
> - S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.
> - S402 (Tom, owner) — As the owner, I want to set a seat's words, variables, countdowns and schedules in one place and see what was actually delivered, so that overnight work runs without a second system.
> - S403 (Archie, an agent) — As an agent, I want to read and set my own variables with a revision, so that my focus survives a compaction and nobody overwrites it blind.
> - S404 (Archie, an agent) — As an agent, I want a reminder to carry the goal's words and the time left, so that I can plan the hour.

## Purpose

Bring the configuration the existing seats run on into the owners already specified for Lys, without silently dropping fields, copying a credential, deleting a source or starting a seat. A person sees a complete dry run for one named seat and confirms that exact plan. An interrupted import resumes the same operation and source revisions, and a completed rerun changes nothing. Rules are inactive transfer entries owned by liminal services, not an executable rule engine in Lys. Checklist/story ownership is split deliberately: AGENTS-001 supplies the words, variables and schedule owners; AGENTS-002 supplies the seat and move; this brief imports their source records and proves the import preserves those behaviours.

## Task

Implement a per-seat source manifest, documented readers, a sending-nothing dry run, person-authorised confirmation and a durable import receipt. Map profiles into HOME-037, budgets into DIRECTORY-051, words/variables/schedules into AGENTS-001 and seat identity into AGENTS-002. Preserve credential references and every excluded source item with its reason. Importing and starting are separate deliberate acts; move Waffles first, then one seat at a time under AGENTS-002 R5. Write the entire slice before its build qualification. This brief specifies the import and the four delivery bars, not a source cleanup, a bulk move or a new supervisor.

## Requirements

### R1: Read named, documented sources and map every member

WHEN a person requests a dry run for one seat, THE SYSTEM SHALL read only the manifest's declared files and authenticated APIs, validating their shapes at the boundary and naming every unreadable, unsupported or ambiguous member. The Codex profile source is $CODEX_HOME/<name>-channels.config.toml, observed here as /Users/tom/.codex/pikelet-channels.config.toml and brisket-channels.config.toml; its profile-file layout is documented in docs/harness/reference/codex/config-file-config-reference.md, and its native configuration contract is the declared HOME-037/DIRECTORY-065 build. The Claude sources are $SEAT_RESOURCES/<address>/settings.json, system-prompt.md and mcp.json, documented by /Users/tom/Developer/seats/README.md and _templates/claude/SETTINGS.md; the observed Waffles folder is /Users/tom/Developer/seats/ablative/waffles/. These are launch resources, not /Users/tom/.manifold/seats/*.json supervisor documents. Select the resources by the supplied manifest, never by guessing a directory named seats. Profiles, system/developer instructions, permissions, model/effort, MCP transports/settings/channel policy and declared nonsecret settings SHALL map into the versioned provisioning Profile/Version/Settings and Tool policy HOME-037 carries. Unsupported settings SHALL refuse the plan by member name; they SHALL NOT be discarded, evaluated as shell code or represented by a best-effort fallback. A source file revision is its SHA-256 over captured noncredential bytes; if a credential-bearing member is encountered, refuse before including its bytes in any plan or receipt. Shared defaults/templates are read once per batch and indexed, with provenance retained for each seat.

**Acceptance:**
- seat_import_sources_maps_declared_files supplies a Codex channels profile and the three Claude resource files; each declared nonsecret member appears in the named provisioning or Tool policy destination and retains its source locator/revision. An unsupported native config member produces ImportMemberUnsupported naming the member, with no destination writes.
- seat_import_sources_distinguishes_roots supplies two different directories called seats and selects one in the manifest; only the selected files are read. Missing files, malformed TOML/JSON and unreadable API responses return distinct named errors and no partial success.
- seat_import_sources_preserves_channels imports command and HTTP MCP servers and off/wake channel policies through the declared harness contract; permissions are not widened, Argus hook/status-line settings are shown as replacements by Lys-owned hooks/status line, and the person must confirm those differences. No launch happens.

**Files:**
- create: crates/lys-identity-server/src/seat_import_sources.rs
- create: crates/lys-identity-server/src/seat_import_profiles.rs
- create: crates/lys-identity-server/tests/seat_import_sources.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R2: Map the Argus records without inventing a transport

THE SYSTEM SHALL read budgets_list (GET /api/budgets), prompt_settings_list (GET /api/prompt-settings), agent_variables_get (POST /api/agent-variables/get, exact canonical agent/session scope), scheduled_messages_list (GET /api/scheduled-messages), and rules_list (GET /api/rules) through the authenticated Argus API. The source contracts are tools/argus/README.md, docs/agent-prompts.md and docs/scheduled-messages-20260907.md in the Argus repository. Read response completeness/error/invalid/skipped-line fields as evidence; an unavailable or incomplete required source refuses the plan, never pretending to be empty. Map budget context window/inform levels/compaction policy into DIRECTORY-051's BudgetStore and context-policy configuration for the imported agent; preserve Codex notices as information-only. Argus's transport target is retained only as source provenance in the import receipt; the new destination is the AGENTS-002 seat/identity and later managed session, never a herdr/screen/manifold target. Map prompt defaults, templates, links and agent/session overrides into AGENTS-001 words records; map variable values, author, canonical scope, revision and absolute expiry into its variables records; map schedule definitions and preserved occurrence state under R6. Source revisions are prompt/variable/schedule revision and rule state_revision; APIs without a definition revision, including budgets, use a canonical SHA-256 of the complete definition fields and scope, excluding changing usage observations. Native revisions and content hashes are distinguished by kind in the receipt. No aliases.json, journal or console scraping substitutes for these APIs. A manifest supplies the explicit legacy agent/session-to-seat identity mapping; unresolved or multiple mappings refuse ImportScopeAmbiguous. Session-scoped values remain staged against that exact source session and seat until AGENTS-002 binds the new managed session at start; they are never silently changed into agent-scoped values or attached to an unrelated session.

**Acceptance:**
- seat_import_monitor_maps_records imports a fixture containing a window override, inform levels, a linked words template, an agent variable and a session variable with expiry. All resolve to the declared Lys agent/session destination with original provenance and expiry; source target appears only in the receipt, never as a Lys delivery transport.
- seat_import_monitor_refuses_incomplete returns invalid budget rows, nonzero rule skips and an API health/error indicating incomplete definitions in separate cases; each identifies the API and reason and prevents confirmation. Zero valid rows without an error is distinguished from unavailable.
- seat_import_monitor_revision_projection changes usage percent without changing a budget definition: its source revision stays the same; changing inform levels changes its content revision. A changed native record revision invalidates a preview even if its rendered wording is identical.

**Files:**
- create: crates/lys-identity-server/src/seat_import_monitor.rs
- create: crates/lys-identity-server/tests/seat_import_monitor.rs

**Checklist:**
- C701 — The five message slots (context warning, preparation, compaction, wake-up, scheduled reminder) resolve built-in, then workspace, then agent, then session, with named templates; an empty layer inherits (AGENTS-001 R1).
- C702 — An agent's and a session's variables are JSON values with a revision, an author and an optional expiry; a stale revision is refused by name; the agent reads and sets its own through lys and the Lys MCP server (AGENTS-001 R2).
- C704 — A schedule has at, until, interval, max occurrences and recipients; missed intervals coalesce to one send; an uncertain delivery stops it for inspection; every state survives restart (AGENTS-001 R4).
- C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.
- S402 (Tom, owner) — As the owner, I want to set a seat's words, variables, countdowns and schedules in one place and see what was actually delivered, so that overnight work runs without a second system.
- S403 (Archie, an agent) — As an agent, I want to read and set my own variables with a revision, so that my focus survives a compaction and nobody overwrites it blind.
- S404 (Archie, an agent) — As an agent, I want a reminder to carry the goal's words and the time left, so that I can plan the hour.

### R3: Keep the Cambium identity and every secret as references

THE SYSTEM SHALL record a Cambium seat identity as its declared $CAMBIUM_SEATS/<name>.seat.json reference (observed files are under /Users/tom/.cambium/), the owning machine/account, the public seat identity and the use-only resolution owner. The privateKeyPem SHALL NOT be copied, imported, hashed into a plan revision, printed, placed in an environment literal or sent to a model. The importer SHALL NOT parse the credential file to extract its key. A declared secrets-broker handle may replace a file reference only when that handle already resolves through the approved broker; moving a key into the broker is a separately authorised operation, not part of import. The responsible person/identity association is checked through Lys admission and the reference's resolution owner, never inferred from a filename. Reference usability and identity match are checked before confirmation without returning key material. A secret-shaped MCP argument, URL or setting is refused by member name without echoing its value. Records, dry-run output, receipts, logs and redacted fixtures carry paths/handles and public identities only.

**Acceptance:**
- seat_import_keys_reference_only imports a seat with a credential-file reference and one already provisioned use-only handle. Every persisted/output byte is searched for a canary secret held by the fixture resolution owner; it is absent, while both references and their public identity match are present.
- seat_import_keys_refuses_inline supplies the canary in an MCP argument, a bearer URL and an environment literal separately; ImportCredentialInline names the field, every output excludes the canary and no destination write occurs.
- seat_import_keys_refuses_wrong_owner makes a reference unreadable, unresolved and bound to a different public seat in separate signal-controlled fixtures. Each refuses before confirmation; none copies the file or silently changes the seat identity.

**Files:**
- create: crates/lys-identity-server/src/seat_import_references.rs
- create: crates/lys-identity-server/tests/seat_import_references.rs

**Checklist:**
- C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
- C710 — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R4: Show one complete dry run and confirm that exact seat

`lys seat import dry-run <name> --manifest <path>` SHALL return a versioned plan containing plan_id, plan_revision, captured_at, seat/public identity/responsible person, source entries {kind, locator, scope, revision_kind, source_revision, completeness}, destination entries {record_kind, record_id, expected_revision, change, source_entry_ids}, credential references, replacements, schedule_counts {total, live, expired, finished, not_imported}, excluded entries {source_id, revision, reason}, prerequisites and named refusals. Counts are computed at the single captured_at instant, not from successive wall-clock reads. Dry run sends no prompt, starts/stops no process and changes neither source nor destination records. `lys seat import confirm <name> --plan <id> --revision <revision>` SHALL require one signed person authorised live for the seat.import action on this seat, naming the seat, identity and exact plan revision. An agent-only confirmation, missing responsible person, bulk/wildcard confirmation and stale plan are refused by name. Re-read each source definition/revision and check the bound destination revisions before applying; a change refuses ImportSourceMoved or ImportDestinationMoved with the affected locator/record. The receipt names the captured source revision vector; it SHALL NOT claim an atomic snapshot across independent APIs. Confirmation grants import only, never seat.start, a secret move or source deletion.

**Acceptance:**
- seat_import_dry_run_is_read_only snapshots the source and destination stores and observes all launch/delivery calls; dry run changes no bytes, makes zero launch/delivery calls and returns the complete plan fields and exact counts for that seat.
- seat_import_confirmation_is_per_seat confirms plan A for seat A as its authorised person: only A is applied. An agent, another person's plan, a wildcard, changed source revision and changed destination revision each refuse with the exact action/record or locator; none imports another seat.
- seat_import_confirmation_binds_replacements changes a shown replacement from an Argus hook to an unrelated hook after preview. The plan revision no longer matches and confirmation refuses; accepted confirmation receipts name the person, grant and exact replacement list.

**Files:**
- create: crates/lys-identity-server/src/seat_import_api.rs
- create: crates/lys-identity-server/src/seat_import_plan.rs
- create: crates/lys-identity-server/src/seat_import_rights.rs
- create: crates/lys-identity-server/tests/seat_import_plan.rs
- create: crates/lys/src/commands/seat_import.rs
- create: crates/lys/tests/seat_import.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/commands/mod.rs

**Checklist:**
- C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
- C710 — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R5: Commit by source revision and resume the same import

WHEN confirmation is accepted, THE SYSTEM SHALL durably reserve one import operation keyed by seat identity plus source kind/locator/scope/revision vector and plan revision before any destination step. Each destination write carries that operation's stable step id and expected record revision. A completed rerun of the same source revision vector returns its existing receipt and makes zero destination writes; a changed source revision requires a new dry run and person confirmation. Imported profile/words/variables/budgets/schedules are staged under their own destination owners and become the seat's selected configuration only through one durable completed-import manifest; no incomplete import may be selected by launch, rendering, budget enforcement or scheduling. Uncertain append outcomes are reconciled by operation/step readback, never a new id or a blind retry. A restart resumes the confirmed operation without asking for a second confirmation and without overwriting a person's intervening destination edit; a conflict is a named stopped operation. The receipt records every stage, destination id/revision and not-imported reason. New import record kinds are versioned; existing installed destination shapes retain their migration owner and upgrade-intent fence, with an old-install migration proof for any changed stored shape. Nothing is deleted or modified at the source, including after successful import.

**Acceptance:**
- seat_import_rerun_is_noop confirms the same source vector twice and reads after reopening the stores: both return the same operation/receipt, one set of destination versions, and zero new writes/syncs on the second confirmation.
- seat_import_resumes_after_each_step injects a process-exit signal after reservation and after each staged write, including a durable write whose reply is lost. Reopening resumes the same operation/step ids, publishes exactly one completed manifest and exposes no partially imported configuration.
- seat_import_preserves_edits changes one destination revision while an import is interrupted; resumption stops naming that record and leaves the person's edit intact. Hashes of all source files and API definitions remain unchanged after success and failure.
- seat_import_old_install_upgrade starts with the previously installed destination shapes and a pending upgrade intent. Writes respect the shared migration fence, named intent-read failures refuse, and the shared migration owner preserves all old records; the new versioned import journal has no invented legacy format.

**Files:**
- create: crates/lys-identity-server/src/seat_import_state.rs
- create: crates/lys-identity-server/src/seat_import_store.rs
- create: crates/lys-identity-server/src/seat_import_apply.rs
- create: crates/lys-identity-server/tests/seat_import_recovery.rs
- create: crates/lys-identity-server/tests/seat_import_upgrade.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/budgets_store.rs
- modify: crates/lys-identity-server/src/provisioning_store.rs

**Checklist:**
- C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
- C710 — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R6: Count live schedules and keep rules as inactive transfers

At the plan's captured_at instant, THE SYSTEM SHALL count every source schedule and classify it once: state finished first, otherwise expired when until is at or before captured_at (until is exclusive), otherwise live. Import only live definitions relevant to this seat; record each finished/expired definition as not imported with its id, revision and exact reason. Preserve paused, failed and uncertain states; they remain stopped in Lys and require a separate deliberate resume with the original uncertainty accounted for. Preserve absolute dates, windows, templates/links, completed/max occurrences and stable occurrence provenance; a spent occurrence is never replayed. Legacy recipient targets are provenance only and must map explicitly to the confirmed Lys seat/identity; an unmapped recipient refuses the plan. A shared multi-recipient schedule has one destination definition keyed by source id/revision, with each seat's recipient binding confirmed separately and only those bindings enabled after the independent move proof; importing the second seat must not create a second schedule or resend the first seat's occurrence. Event/row-triggered schedules unsupported by AGENTS-001 are shown as named unrepresentable definitions and block confirmation, never flattened to a guessed interval. Every Argus rule's full noncredential definition and state_revision SHALL land as an inactive transfer entry in the import receipt naming liminal services as its owner (ADR-136); its paused/retired state is retained, no rule engine is added and Lys never activates a rule. A credential-bearing rule member refuses by name. Import itself activates no delivery; AGENTS-002 R5 separately proves the first Lys warning and the absence of Argus delivery for the moved seat.

**Acceptance:**
- seat_import_schedule_counts uses seven definitions: two finished (one also past until), two expired and three live (paused, failed, uncertain). Counts are total7/live3/expired2/finished2/not_imported4; the receipt records four exclusions, three stopped imported definitions and zero sends.
- seat_import_shared_schedule imports a two-recipient definition for A then B, preserving a completed occurrence id. There is one destination definition, each binding has its own confirmation/move prerequisite, and the completed occurrence is never delivered again. An unmapped recipient and unsupported event trigger each refuse by name.
- seat_import_rules_inactive transfers a current and retired rule: their complete definition/state_revision/status and liminal owner are present in the receipt; zero rule activation or rule-engine calls occur. No source schedule or rule is changed.

**Files:**
- create: crates/lys-identity-server/src/seat_import_schedules.rs
- create: crates/lys-identity-server/src/seat_import_rules.rs
- create: crates/lys-identity-server/tests/seat_import_schedules.rs
- create: crates/lys-identity-server/tests/seat_import_rules.rs

**Checklist:**
- C704 — A schedule has at, until, interval, max occurrences and recipients; missed intervals coalesce to one send; an uncertain delivery stops it for inspection; every state survives restart (AGENTS-001 R4).
- C710 — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.
- S404 (Archie, an agent) — As an agent, I want a reminder to carry the goal's words and the time left, so that I can plan the hour.

### R7: Never down: the imported seat survives its surfaces and services

After the separately confirmed move, THE SYSTEM SHALL keep the seat alive through terminal attachment crash, Lys service restart and Lys upgrade using Lys's own supervision contract, never herdr. Import status and the completed manifest survive each event; restarting the importer does not restart a running seat, rotate its identity or duplicate a session/delivery. This requirement depends on the never-down supervision slice and DIRECTORY-064's launched-binary qualification pin; a source path or an import receipt is not survival proof. On a failed upgrade, the named shared upgrade owner preserves the running seat or returns its explicit failed state with recoverable intent, never an empty/online default.

**Acceptance:**
- seat_import_survives_lifecycle first moves one qualified fixture seat, then injects attachment exit, service exit and upgrade handoff through explicit signals. The held child/session identity and a correlated in-flight operation survive each event; the reconnected registry reads the same seat and one completed import receipt.
- seat_import_upgrade_failure_named injects the upgrade owner's refusal and loss of an import response. The failure and retained intent are named; the running seat is not killed by importer cleanup, and recovery resolves the same import operation without a duplicate start.

**Files:**
- create: crates/lys-identity-server/tests/seat_import_lifecycle.rs

**Checklist:**
- C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
- C707 — The Sessions screen shows every seat online, offline, not seen by the runner, or seen but unregistered, from the runner's own liveness, never from a terminal program (AGENTS-002 R2).
- C710 — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R8: Never slow: count each hot path and ratchet only down

THE SYSTEM SHALL keep import parsing, source fetches, revision hashing, journal reconciliation and transfer-rule work outside proxy forwarding, hook ingest, managed delivery and registry reads. For each path and for dry-run/confirm/recovery, check in a deterministic count fixture and separate ceilings for executed instructions where supported and named calls (source reads, destination lookups/writes, clones, lock acquisitions, syncs and history visits). No whole-state clone or loop over all import history is allowed on those four hot paths. A lookup by seat/operation is indexed; import work is bounded to the selected plan and pending steps, never a repeated fleet scan. Prove each chosen count correlates with wall-clock improvement on the same workload/build by changing the measured work, without pausing other builds; wall-clock results are evidence, never the primary CI verdict. Unsupported instruction counting must be named and use the specified call count, never fabricated instruction data. The initial measured ceiling is reviewed and checked in, subsequent passing counts may only lower it; a regression cannot be hidden by increasing a threshold or reducing the fixture. Test/fixture execution longer than two seconds is a defect to fix, not permission for a larger timeout.

**Acceptance:**
- seat_import_hot_path_counts runs proxy/hook/delivery/registry count fixtures with no import history and with a large retained import journal; per-operation import-related source reads, full-state clones, syncs and history visits are exactly zero in both, and declared path counts remain within checked-in ceilings.
- seat_import_count_ratchet changes the counted implementation to perform one extra named call: the count verdict goes red. Lowering measured work passes and lowers the committed ceiling; a higher ceiling or smaller fixture is rejected.
- seat_import_count_latency_evidence records the same-workload count and wall-clock comparison before/after a real reduction. A metric that cannot demonstrate the correlation is rejected before becoming a ratchet; concurrent estate work is not paused.

**Files:**
- create: crates/lys-identity-server/tests/seat_import_counts.rs
- create: crates/lys-identity-server/tests/seat_import_counts.json
- create: crates/lys-runner/tests/seat_import_hot_paths.rs
- create: crates/lys-runner/tests/seat_import_hot_paths.json

**Checklist:**
- C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
- C707 — The Sessions screen shows every seat online, offline, not seen by the runner, or seen but unregistered, from the runner's own liveness, never from a terminal program (AGENTS-002 R2).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R9: Never break: observe every red and inject faults by signal

Before an import behaviour is implemented, THE SYSTEM'S qualification evidence SHALL record its named test running red on the pinned main base and green on the exact change, with command, exit code and full case counts. A missing test target or a compile failure is not a behavioural red. Fixtures call the public import/admission/store boundaries; they SHALL NOT mirror private helper implementations. Source mutation, denied reads, credential-reference failure, lost write replies, process exit and readiness are driven by explicit fixture signals/events. Tests wait for the event/exit/ready message, never a sleep, wall-clock deadline or watchdog. Every error is propagated or recorded by name, no ignore/allow/timeout bypass and no silent default makes a failed import appear complete. The one assembled qualification follows the complete written slice; focused tests are not a full-suite or installed-product pass.

**Acceptance:**
- seat_import_behaviour_red_matrix enumerates every acceptance behaviour in R1–R8 and R10 with its runnable test name, observed base failure, observed change success, both commits, commands/exits and full counts. Missing rows, compile-only reds and unrun claims fail the matrix.
- seat_import_signal_faults drives all fault cases with an event-controlled fixture and receives a named completion/refusal. Under competing load the same outcomes are produced without sleeps or increased timers; each case remains below the two-second defect bar.

**Files:**
- create: crates/lys-identity-server/tests/seat_import_faults.rs
- create: crates/lys-identity-server/tests/seat_import_red_matrix.json

**Checklist:**
- C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
- C710 — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R10: Scales: load three times the seat count with flat cost per seat

THE SYSTEM SHALL record the current unique seat count N from the authenticated source seat roster and use a reproducible load fixture at N and 3N (the planning estimate is 25 and 75, not an observed count). The fixture carries equal bounded profile/word/variable sizes and equal hook/proxy/delivery/registry work per seat, with exact operation counts and a fixed completed-history population. Read fleet source definitions once per batch and index them; account for that initial work in total and per-seat counts. Shared templates/schedules are not fetched once per recipient, and adding seats must not multiply history scans or full-state clones. Compare deterministic work per seat and peak owned bytes at N and 3N; per-seat named source/destination/hot-path calls must remain at the same measured ceiling, with total work proportional to seats and fixed per-seat memory bounds. A roster read failure is unknown, never N=0. All configured byte, member and recipient bounds are enforced with named refusals before reservation; the importer leaves running seats responsive while another seat is imported. One confirmed seat plan is bounded to 4 MiB of noncredential captured data, with each profile/settings/prompt/MCP resource at most 1 MiB, at most 64 variable keys per scope under the existing 64 KiB scope bound, at most 256 live schedule definitions per seat and at most 32 recipients per schedule. Streaming response parsing applies the declared API completeness contract before plan allocation; oversized inputs refuse ImportBoundExceeded naming the bound. These limits are explicit plan fields and cannot be raised to make a qualification pass.

**Acceptance:**
- seat_import_threefold_scale pins roster source/revision/N and runs the equal-work fixture at N and 3N. It reports all completed/failed operations, exact source/destination/hot-path counts and peak owned bytes; per-seat counts do not exceed the N ceiling, no whole-history scan is added and fleet API calls occur once per batch.
- seat_import_bounds_and_liveness submits each declared size/member/recipient bound plus one excess input while another seat forwards a proxy call and reports a hook. The excess input is refused before reservation, the other seat's correlated work completes, and an unreadable roster is named unknown without inventing a zero count.

**Files:**
- create: crates/lys-identity-server/tests/seat_import_scale.rs
- create: crates/lys-identity-server/tests/seat_import_scale.json

**Checklist:**
- C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
- C707 — The Sessions screen shows every seat online, offline, not seen by the runner, or seen but unregistered, from the runner's own liveness, never from a terminal program (AGENTS-002 R2).
- C710 — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

## Boundaries

- This is a deliberate import of one person's confirmed seat plan. No automatic fleet move, source deletion, source mutation, secret extraction/copy, new rule engine or implicit start/stop/resume is permitted.
- Rules remain inactive transfer entries owned by liminal services. Legacy terminal targets remain provenance; every later delivery belongs to DIRECTORY-064. Unrepresentable fields or schedule triggers refuse by name and remain visible.
- AGENTS-002 R1 owns the seat record and seats_api.rs. AGENTS-001 owns words/variables/schedules, HOME-037 owns profile rendering, DIRECTORY-051 owns budget semantics and the never-down slice owns supervision. Import does not duplicate those owners or fill QUALIFIED.
- Move Waffles first, then one seat at a time under a separate AGENTS-002 R5 approval and proof. This brief decides no later seat order, changes no installed service and makes no hospital-readiness claim by being written.

## Verification

- Brief publication: verify every declared modify path with git ls-tree at the pinned tree; render this JSON with scripts/design/render-brief.py; run scripts/design/gate.sh in the Lys main checkout on this Mac under Waffles amendment 5efd2b280a7cd279466de343654b78be31cb771ff10cd6bc9c2d3e39e207abd2 (Python design checks only; no compilation), retaining its exit code and every parsed failure/warning. Parsed failures must be zero. Commit only this brief's JSON/Markdown by pathspec straight to main and read back its exact remote commit/tree. Do not publish the existing nine local main commits before Brisket's qualification passes.
- Implementation starts with the complete red matrix and documented source readers; write the whole agreed slice before builds. Focused nextest and strict touched-crate Clippy run only on the assigned remote seat. The lead owns the one assembled qualification, battery/install and real one-seat move; no local Mac compilation, full-suite claim from a subset or early gate.
- Qualification handback names the exact source and change commits/trees, every command and exit, full case and schedule counts, all unconfirmed API/build/move prerequisites, and for each changed hot path its locks, whole-state clones, disk syncs per call and history loops. No test slower than two seconds is accepted by raising a timeout.
- Hospital line: this brief is a specification, not evidence that importing, survival, performance, live delivery or an installed seat is safe; those claims require the pinned qualification evidence and lead's real move proof.
