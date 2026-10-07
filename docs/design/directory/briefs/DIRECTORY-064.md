---
type: brief
id: DIRECTORY-064
cluster: directory
title: Lys controls context and delivers goals through managed harness channels
---

# DIRECTORY-064: Lys controls context and delivers goals through managed harness channels

> **Cluster:** directory
> **Depends on:** DIRECTORY-051
> **Design anchor:**
> - ADR-130 — Managed harness channels carry automatic context and reminder acts — 064 adds one runner-owned typed control/event channel per managed Claude or Codex session. It reuses 051 operation and usage owners, serialises all inputs at proved boundaries, requires actual harness evidence and retains uncertain outcomes without automatic resend. Legacy PTY sessions remain manual and cannot claim this capability. 065 extends the same Codex event reader for refusal coverage.
> **Checklist:**
> - C433 — One managed harness channel, with proved turn boundaries (DIRECTORY-064 R1).
> - C434 — Use Claude and Codex control protocols, never terminal typing (DIRECTORY-064 R2).
> - C435 — Enforce context thresholds at the owned boundary (DIRECTORY-064 R3).
> - C436 — Deliver current goal and reminder words at turn boundaries (DIRECTORY-064 R4).
> - C437 — Reconcile uncertain delivery with existing operation receipts (DIRECTORY-064 R5).
> - C438 — Plain controls and a real managed-session proof (DIRECTORY-064 R6).
> **Stories:**
> - S173 (Person setting agent context and reminders, Controls agents without a terminal) — As the person responsible for an agent, I want Lys to compact or stop it at my context limit without terminal typing, so the next turn obeys the limit.
> - S174 (Person setting agent context and reminders, Controls agents without a terminal) — As the person setting a goal, I want my saved words delivered at a turn boundary with honest receipts, so a reminder never interrupts a tool or silently arrives twice.

## Purpose

Waffles's 29 September 07:32 assignment relays Tom's 07:02 intent: Lys drives context compaction, goals and reminders behind the scenes, with plain controls and no terminal typing. The DIRECTORY-051 brief at 17eeac97 assigns tracking, budgets, goals, operation identities and the Usage screen to 051. That commit contains brief documents only, not their implementation. Its R3/R4 leave delivery at a generic runner/terminal seam; this brief closes that gap without implementing those owners twice. Card k7sAzdWw.

## Task

Extend 051 with one managed harness control channel, verified turn boundaries, real compaction and reminder admission evidence, no PTY automation, and explicit unknown-outcome reconciliation. Keep tracking and policy in their existing owners. The new Codex event owner is also the integration seam for DIRECTORY-065. Use an integrated base containing the actual DIRECTORY-051 implementation before measuring these requirements. A brief publication or an active 051 run does not satisfy that code dependency. Do not create 051 owners again in 064.

## Requirements

### R1: One managed harness channel, with proved turn boundaries

Behavioural. WHEN Lys starts a session requiring context actions or goal delivery, THE SYSTEM SHALL select a versioned managed-control adapter and bind it to the runner session, proved process start identity and actual harness conversation identity before accepting control operations. Extend DIRECTORY-050's Launch/session transport with a managed-pipe mode; its existing PTY mode remains available for manual sessions, but a profile requiring these controls refuses control_transport_unsupported on PTY-only sessions. Never silently restart or migrate a running terminal session. The managed process runs under the same isolated home, environment, credentials, process ownership and policy as the normal runner launch. Keep 051's hooks, usage collector, cursor accounting, budget/goal stores and operations log; do not create a second usage or reminder system. All session inputs, including the person's messages and automation, pass through one dispatcher so a turn cannot start between a boundary check and control dispatch. Map started/completed/compaction events to a typed event projection with source identity, session generation, conversation and turn identifiers. Subscribe other consumers (including 065) through this one reader, with durable cursors and a named gap when transient evidence was not retained. Hook callbacks enqueue and return; they never wait for the turn that invoked them to end. Events from another generation cannot open a boundary. A reconnect starts in unknown boundary state until authoritative harness state is reconciled. A transport loss is visible; silence is never an idle signal. Version/executable and capability evidence are recorded from the actual launched binary, not inferred from a source checkout.

**Acceptance:**
- A managed launch records the actual executable identity.
- A PTY-only session requiring automatic controls refuses control_transport_unsupported.
- The existing manual terminal launch still accepts manual input.
- A completed event for another session cannot release a queued reminder.
- A duplicate completed event cannot release a second operation.
- A reconnect without proved idle state dispatches no message.
- A hook callback returns while its turn remains active.
- A concurrent human message cannot overtake the boundary dispatcher.
- An expired event cursor yields an explicit coverage gap.
- Repeated adapter events add no second usage charge in 051.

**Files:**
- create: crates/lys-runner/src/harness_control.rs
- create: crates/lys-runner/src/harness_control/events.rs
- create: crates/lys-runner/src/harness_control/process.rs
- create: crates/lys-runner/tests/harness_control.rs
- create: crates/lys-runner/src/harness_control/approval.rs
- create: crates/lys-identity-server/src/runner_sessions/controls.rs
- create: crates/lys-runner/src/session/lifecycle/managed.rs
- create: crates/lys-runner/src/collector/stop.rs
- create: crates/lys-runner/src/tracking_store/managed.rs
- create: crates/lys-runner/src/protocol/acts.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/session/lifecycle.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-identity-server/src/runner_sessions.rs
- modify: crates/lys-runner/src/operations.rs
- modify: crates/lys-runner/src/operations/delivery.rs
- modify: crates/lys-runner/src/collector.rs
- modify: crates/lys-runner/src/socket/acts.rs
- modify: crates/lys-runner/src/session/control.rs
- modify: crates/lys-runner/src/tracking_store.rs
- modify: crates/lys-identity-server/src/budgets_feed.rs
- modify: crates/lys/src/commands/runner.rs
- modify: crates/lys/src/cli/runner.rs
- modify: crates/lys-runner/Cargo.toml
- modify: crates/lys-identity-server/tests/budget_feed.rs
- modify: crates/lys-identity-server/src/provisioning_store.rs
- modify: crates/lys-identity-server/src/launch_template.rs
- modify: surface/identity/src/features/provisioning/Provisioning.tsx
- modify: surface/identity/src/features/provisioning/ProfileEditor.tsx
- modify: surface/identity/src/features/usage/Usage.tsx
- modify: surface/identity/tests/agent-provisioning.test.tsx
- modify: surface/identity/tests/usage.test.tsx
- modify: crates/lys-identity-server/src/provisioning_compat_tests.rs
- modify: crates/lys-identity-server/src/launch_record_config_tests.rs
- modify: crates/lys-runner/src/launch_config.rs
- modify: crates/lys-identity-server/src/launch_api.rs
- modify: crates/lys-identity-server/src/launch_record_config.rs
- modify: crates/lys-runner/tests/launch_config.rs
- modify: crates/lys-identity-server/src/runner_start_pass_tests.rs
- modify: crates/lys-runner/tests/lifecycle/cases.rs

**Checklist:**
- C433 — One managed harness channel, with proved turn boundaries (DIRECTORY-064 R1).

**Stories:**
- S173 (Person setting agent context and reminders, Controls agents without a terminal) — As the person responsible for an agent, I want Lys to compact or stop it at my context limit without terminal typing, so the next turn obeys the limit.
- S174 (Person setting agent context and reminders, Controls agents without a terminal) — As the person setting a goal, I want my saved words delivered at a turn boundary with honest receipts, so a reminder never interrupts a tool or silently arrives twice.

### R2: Use Claude and Codex control protocols, never terminal typing

Behavioural. WHEN the dispatcher delivers an act, THE SYSTEM SHALL use a machine protocol over runner-owned pipes, with no PTY write, terminal key, shell command construction or external terminal-control application. Claude's adapter launches the selected binary with --print --input-format stream-json --output-format stream-json --verbose --replay-user-messages, an explicit bound session and the existing isolated settings. Send one SDK user envelope with a stable uuid, session_id, parent_tool_use_id null, and message role user/content. A normal result closes a submitted turn; partial assistant text does not. A compaction request is the exact /compact command in this protocol, never terminal text. Discover compact capability from system/init slash_commands. Confirm actual compaction only on that session's system/compact_boundary, not a result success: the documented no-history case succeeds without compacting. User-envelope replay with matching uuid confirms message admission only; it does not prove completion, obedience or goal satisfaction. Validate these wire fixtures against the actual installed Claude version before enabling the adapter; absent required envelope correlation refuses the capability. Codex's adapter owns one app-server stdio child and JSON-RPC initialize/initialized handshake, creates or resumes the explicit bound thread, sends thread/compact/start with threadId and delivers a reminder through turn/start with threadId, clientUserMessageId and input text. The compact response is acceptance only; require the corresponding completed ContextCompaction item and terminal turn observation before declaring compaction complete. A turn/start reply identifies the admitted turn, not a finished goal. Only dispatch while the owned thread is idle; never use turn/steer to inject mid-turn. Resolve server approval requests through the existing policy authority; never automatically approve one to keep the transport moving. Unknown protocol variants or missing proof yield a named unsupported/uncertain state. Preserve existing permissions when starting turns: no sandbox or approval override in a reminder call. The local inspected Codex contract is codex-channels 7cbba483, app-server-protocol/src/protocol/common.rs and v2/{thread,turn,item}.rs and app-server/src/request_processors/thread_processor.rs. A checkout is a fixture source, not installed-binary proof. No app-server function is attributed to legacy notify; notify remains after-turn only.

**Acceptance:**
- The Claude fixture sends /compact through the managed JSON pipe.
- The terminal-writer spy records zero automated writes during Claude compaction.
- A Claude success result without compact_boundary is recorded not_compacted.
- A matching Claude replay uuid records admission.
- A mismatched replay uuid cannot confirm delivery.
- The Codex fixture emits thread/compact/start for the bound thread.
- The empty Codex compact response leaves the operation accepted.
- A completed ContextCompaction item with matching thread/turn confirms compaction.
- A Codex reminder uses turn/start only after turn/completed.
- A reminder request contains no sandbox override.
- A reminder request contains no approval override.
- A harness approval request receives no fabricated approval.

**Files:**
- create: crates/lys-runner/src/harness_control/claude.rs
- create: crates/lys-runner/src/harness_control/codex.rs
- create: crates/lys-runner/tests/harness_claude.rs
- create: crates/lys-runner/tests/harness_codex.rs
- create: crates/lys-runner/src/harness_control/initialize.rs
- modify: crates/lys-runner/src/harness_control.rs
- modify: crates/lys-runner/src/harness_control/events.rs

**Checklist:**
- C434 — Use Claude and Codex control protocols, never terminal typing (DIRECTORY-064 R2).

**Stories:**
- S173 (Person setting agent context and reminders, Controls agents without a terminal) — As the person responsible for an agent, I want Lys to compact or stop it at my context limit without terminal typing, so the next turn obeys the limit.
- S174 (Person setting agent context and reminders, Controls agents without a terminal) — As the person setting a goal, I want my saved words delivered at a turn boundary with honest receipts, so a reminder never interrupts a tool or silently arrives twice.

### R3: Enforce context thresholds at the owned boundary

Behavioural. WHEN 051's authoritative context measurement crosses its held threshold, THE SYSTEM SHALL reuse its stable crossing/operation identity and block release of the next normal input before running the configured compact or stop act. Use 051's measurement validity and budget precedence; repeated snapshots are not new crossings. A compact command waits for the active turn to finish, then runs before queued normal input. The context watch must not recursively react to its own compaction events. After actual compaction, require a fresh measurement below the threshold before releasing held input. When compaction fails, is unsupported, finishes above threshold or yields no valid post-measurement, keep the next turn held and show the named reason, without automatic repeated compaction. The responsible person can choose the existing stop act or an authorised policy change; the agent cannot lift its own hold. A configured stop uses the existing proved process/session owner and exit receipt, never a typed stop command; stop confirmation requires actual process exit. Explicit stop takes priority over queued compaction/reminders. Tracking loss under a required context policy holds the next turn instead of assuming the limit was respected. Setting a threshold is an authorised product policy, not a request timeout or watchdog.

**Acceptance:**
- A threshold crossing holds a queued normal message.
- A compact crossing sends exactly one compact request after the active turn ends.
- Repeated equal snapshots send no additional compact request.
- A valid fresh below-threshold measurement releases the held next message.
- A post-compaction above-threshold measurement keeps the next message held.
- A missing post-compaction measurement keeps the next message held.
- A compaction event cannot trigger recursive compaction.
- An explicit stop supersedes queued reminder delivery.
- A stop receipt remains unconfirmed before process exit.
- The actual process exit confirms the stop.
- The judged agent cannot remove its own context hold.

**Files:**
- create: crates/lys-runner/src/harness_control/context.rs
- create: crates/lys-runner/tests/context_control.rs
- create: crates/lys-identity-server/src/budgets_act/control.rs
- create: crates/lys-identity-server/src/budgets_state/control.rs
- create: crates/lys-runner/src/protocol/acts.rs
- modify: crates/lys-runner/src/harness_control.rs
- modify: crates/lys-runner/src/operations.rs
- modify: crates/lys-identity-server/src/budgets_act.rs
- modify: crates/lys-identity-server/src/budgets_state.rs
- modify: crates/lys-identity-server/src/budgets_feed.rs
- modify: crates/lys-runner/src/socket/acts.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/legacy_input.rs
- modify: crates/lys-identity-server/src/refusals_follow.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-identity-server/src/budgets_context.rs

**Checklist:**
- C435 — Enforce context thresholds at the owned boundary (DIRECTORY-064 R3).

**Stories:**
- S173 (Person setting agent context and reminders, Controls agents without a terminal) — As the person responsible for an agent, I want Lys to compact or stop it at my context limit without terminal typing, so the next turn obeys the limit.

### R4: Deliver current goal and reminder words at turn boundaries

Behavioural. WHEN a 051 reminder occurrence is due for an authorised live session, THE SYSTEM SHALL enqueue its existing stable occurrence and goal version for the same dispatcher. On the first permitted boundary, re-read the current goal state/version and visibility before dispatch. Marked-met, dropped, cancelled or revoked items do not send stale queued words. Updated words replace an unsent occurrence's payload under a recorded revision without inventing a new due occurrence. Delivered content contains the named goal/expectation/deliverable, exact saved words, due instant and time left or lateness; preserve text as data with proper JSON encoding. Team delivery names each intended session and tracks its receipt separately so one unavailable member does not block the others. Idle means no active harness turn; a goal reminder may start the next controlled turn then. An active tool call is not a boundary. A compaction-triggered reminder waits for completion of compaction and release of the context hold; it does not enter the summary operation. A reminder reaching a stopped session is visibly undeliverable and does not secretly relaunch an agent. Reuse 051 timers, late occurrence handling and authority, without a second scheduler, polling or external message app. Text is delivered as a Lys reminder with recorded provenance; it cannot become an arbitrary slash command. In Claude, prefix an ordinary reminder envelope with the fixed plain-text label Lys reminder before the saved text, so a saved /compact line is data, not a command. Codex uses typed text input. Receipt means harness admission, never proof that the agent did the requested work.

**Acceptance:**
- A reminder due during an active turn sends no input until that turn completes.
- The admitted reminder preserves the saved goal words.
- A due occurrence records its original due instant.
- A late occurrence records its dispatch instant.
- A goal marked met before dispatch sends no reminder.
- A queued old version sends the current authorised words after an edit.
- A revoked goal sends no pending words.
- One unavailable team session does not prevent another member delivery.
- A compaction-triggered reminder waits for compaction completion.
- A stopped session is not relaunched by a due reminder.
- Saved text beginning /compact is delivered as reminder data.
- Reminder admission cannot mark the goal met.

**Files:**
- create: crates/lys-runner/src/harness_control/reminders.rs
- create: crates/lys-runner/tests/reminder_delivery.rs
- create: crates/lys-identity-server/src/budgets_act/control.rs
- create: crates/lys-identity-server/src/goals_state/control.rs
- create: crates/lys-identity-server/src/goals_api/control.rs
- create: crates/lys-runner/src/protocol/acts.rs
- create: crates/lys-identity-server/src/goals_state/fold.rs
- create: crates/lys-identity-server/src/goals_store/control.rs
- modify: crates/lys-identity-server/src/goals_state.rs
- modify: crates/lys-identity-server/src/goals_store.rs
- modify: crates/lys-identity-server/src/goals_api.rs
- modify: crates/lys-runner/src/operations.rs
- modify: crates/lys-runner/src/harness_control.rs
- modify: crates/lys-identity-server/src/budgets_feed.rs
- modify: crates/lys-identity-server/src/goals_index.rs
- modify: crates/lys-identity-server/tests/goals_edits.rs
- modify: crates/lys-identity-server/src/refusals_follow.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-identity-server/src/budgets_act.rs
- modify: crates/lys-runner/src/operations/delivery.rs

**Checklist:**
- C436 — Deliver current goal and reminder words at turn boundaries (DIRECTORY-064 R4).

**Stories:**
- S174 (Person setting agent context and reminders, Controls agents without a terminal) — As the person setting a goal, I want my saved words delivered at a turn boundary with honest receipts, so a reminder never interrupts a tool or silently arrives twice.

### R5: Reconcile uncertain delivery with existing operation receipts

Behavioural. WHEN delivery starts, THE SYSTEM SHALL persist the existing 051 operation identity and a prepared request before a pipe write, then record accepted, confirmed, refused or uncertain as evidence arrives. Persist the distinction between safely unsent and possibly sent; after a crash in the write/receipt gap, do not replay or mint a replacement merely because no answer is stored. A matching durable harness observation may reconcile the original operation; a request UUID is correlation, not an unsupported exactly-once claim. For Codex, use available typed thread/item history or persisted observed events; a transient event gap stays uncertain. For Claude, reconcile matching saved admission/completion evidence for the explicit session; never read arbitrary other transcript files. If no authoritative readback exists, leave uncertain and expose an explicit responsible-person reconciliation action on that operation, which records the person's decision and never silently resends. A decision to send again creates a distinct authorised occurrence labelled possible prior delivery; it must pass the same boundary and authority checks. A restarted runner does not claim a dead pipe restored: use 050's actual lifecycle state and only resume a conversation through a separately authorised session start. Snapshot-backed folds and cursor reads preserve the repository's bounded-restart invariant. Goal text stays under holder visibility; exported operational telemetry contains identifiers/states only, never the text or credentials.

**Acceptance:**
- A crash before any write preserves the safely unsent operation.
- A crash after a possible write records uncertain delivery.
- Restart after a possible write issues no duplicate pipe request.
- Matching saved admission evidence reconciles the original operation.
- An unrelated turn result cannot reconcile that operation.
- An unavailable readback leaves the operation uncertain.
- An unauthorised reconciliation action is refused.
- An explicit resend records a distinct authorised occurrence.
- Restart read counts depend on the snapshot tail rather than total history length.
- Outbound analytics contains no goal text.

**Files:**
- create: crates/lys-runner/tests/control_recovery.rs
- create: crates/lys-identity-server/tests/control_receipts.rs
- create: crates/lys-runner/src/operations/store.rs
- create: crates/lys-runner/src/operations/control.rs
- create: crates/lys-identity-server/src/goals_state/control.rs
- create: crates/lys-identity-server/src/goals_store/control.rs
- create: crates/lys-identity-server/src/goals_api/control.rs
- create: crates/lys-identity-server/src/receipts_api/control.rs
- create: crates/lys-runner/src/protocol/acts.rs
- create: crates/lys-identity-server/src/runtime_state/control.rs
- modify: crates/lys-runner/src/operations.rs
- modify: crates/lys-runner/src/state.rs
- modify: crates/lys-runner/src/harness_control/events.rs
- modify: crates/lys-identity-server/src/receipts_api.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-identity-server/src/goals_state.rs
- modify: crates/lys-identity-server/src/goals_index.rs
- modify: crates/lys-identity-server/src/goals_store.rs
- modify: crates/lys-runner/tests/operations_index/cases.rs
- modify: crates/lys-runner/tests/operations.rs
- modify: crates/lys-identity-server/src/goals_api.rs
- modify: crates/lys-identity-server/src/runner_sessions.rs
- modify: crates/lys-runner/src/operations/delivery.rs
- modify: crates/lys-runner/src/socket/connection.rs
- modify: crates/lys-runner/src/dial.rs
- modify: crates/lys-identity-server/src/runtime_state.rs
- modify: crates/lys-identity-server/src/runtime_store.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/openapi_runner_types.rs
- modify: crates/lys-runner/src/durable.rs
- modify: crates/lys-runner/src/peer_connection_tests.rs
- modify: crates/lys-runner/tests/peer_restart/cases.rs

**Checklist:**
- C437 — Reconcile uncertain delivery with existing operation receipts (DIRECTORY-064 R5).

**Stories:**
- S173 (Person setting agent context and reminders, Controls agents without a terminal) — As the person responsible for an agent, I want Lys to compact or stop it at my context limit without terminal typing, so the next turn obeys the limit.
- S174 (Person setting agent context and reminders, Controls agents without a terminal) — As the person setting a goal, I want my saved words delivered at a turn boundary with honest receipts, so a reminder never interrupts a tool or silently arrives twice.

### R6: Plain controls and a real managed-session proof

Behavioural. WHEN the responsible person opens 051's Usage page, THE SYSTEM SHALL extend that page in place with current context limit/action, observed value and coverage, held/active context state, pending/accepted/uncertain control operations, saved goals/reminders and their due/delivery state. Reuse its forms, routes, authorisation and stores; do not create a second dashboard. Show managed-control capability on the agent/session page before launch and explain unsupported sessions in plain words. Saved settings persist after reload. Exercise both actual launched harnesses on Dean in scratch homes: enter a goal and reminder in the browser, cause a turn-boundary delivery, and separately cause a valid context crossing, actual compaction and stop. Keep a pipe trace stripped of saved text/credentials, terminal-writer count, source event identifiers, before/after context evidence, operation receipts and process-exit evidence. Mock fixtures cover faults but cannot substitute for the live two-harness proof. If an installed harness cannot supply the required protocol, return a named blocker and keep that acceptance incomplete rather than claiming a fixture as live evidence. Browser proof is a controlled scratch install, not a production install. Argus may be absent for every proof.

**Acceptance:**
- A browser-set context threshold persists after reload.
- A browser-set goal reminder persists after reload.
- The Usage page shows the held next-turn state.
- An uncertain operation is displayed as unconfirmed.
- An unrelated viewer cannot read the saved goal words.
- A managed Claude scratch session receives the browser-created reminder.
- The Claude scratch run records actual compact_boundary evidence.
- A managed Codex scratch session receives the browser-created reminder.
- The Codex scratch run records completed ContextCompaction evidence.
- The scratch stop has process-exit evidence.
- The automation trace records zero terminal-writer calls.
- The proof still completes with no Argus connection.

**Files:**
- create: surface/identity/tests/acceptance/agent-control.spec.ts
- create: crates/lys-runner/examples/qualify_adapter.rs
- create: crates/lys-runner/examples/qualify_adapter/owned.rs
- create: crates/lys-runner/tests/fixtures/qualifier/fail-stop-red.jsonl
- create: crates/lys-runner/tests/fixtures/qualifier/fixture-pass.jsonl
- create: crates/lys-runner/examples/qualify_adapter/diagnostics.rs
- create: crates/lys-runner/src/pty/cleanup.rs
- create: crates/lys-runner/src/pty/cleanup_tests.rs
- modify: surface/identity/src/features/usage/Usage.tsx
- modify: surface/identity/src/features/usage/usage.css
- modify: surface/identity/tests/usage.test.tsx
- modify: surface/identity/src/features/runtime/Sessions.tsx
- modify: surface/identity/src/api.ts
- modify: surface/identity/vite.config.ts
- modify: crates/lys-runner/src/pty.rs
- modify: crates/lys-runner/tests/pty_stop.rs
- modify: crates/lys-runner/src/session/stop.rs

**Checklist:**
- C438 — Plain controls and a real managed-session proof (DIRECTORY-064 R6).

**Stories:**
- S173 (Person setting agent context and reminders, Controls agents without a terminal) — As the person responsible for an agent, I want Lys to compact or stop it at my context limit without terminal typing, so the next turn obeys the limit.
- S174 (Person setting agent context and reminders, Controls agents without a terminal) — As the person setting a goal, I want my saved words delivered at a turn boundary with honest receipts, so a reminder never interrupts a tool or silently arrives twice.

## Boundaries

- SHALL NOT send compaction, context notices, goals or reminders through a PTY, terminal keyboard API, pasted command or shell expansion.
- SHALL NOT duplicate 051 usage accounting, budget/goal stores, scheduler or Usage screen.
- SHALL NOT call Argus, Cambium or another app to enforce a context limit or deliver a reminder.
- SHALL NOT add request timeouts, watchdogs, polling loops, arbitrary sleeps, unsafe, lint suppression or ignored tests. Explicit user-held goal due instants remain product data in 051.
- SHALL NOT treat a successful request, agent prose, empty compact reply or ordinary tool failure as proof of completed compaction or satisfied goal.
- SHALL NOT widen permissions to make a control request succeed.
- SHALL NOT call terminal-only compatibility a managed-control success.

## Verification

- Handwritten brief: design gate and git diff --check exit 0; non-writer Chippy reads the published exact commit.
- Card implementation follows card_build_v3, src_pr and src_land with Jev, fmt, Clippy pedantic, all required tests, ast-grep and full gate on Dean.
- Record actual harness versions and protocol observations from scratch launches, with separate admission/compaction/stop evidence and browser evidence. No installed-production claim follows from a scratch proof.
- Grounding: Claude documentation https://code.claude.com/docs/en/agent-sdk/slash-commands and https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode; installed Claude 2.1.284 --help lists stream-json and replay-user-messages. Codex contract source 7cbba483, codex-rs/app-server-protocol/src/protocol and app-server/src/request_processors/thread_processor.rs. These sources define fixtures; launched-binary proof remains required.
