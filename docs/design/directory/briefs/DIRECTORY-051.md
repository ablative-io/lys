---
type: brief
id: DIRECTORY-051
cluster: directory
title: Lys runs and monitors its agents, enforces budgets and goals, and shows authoritative refused acts
---

# DIRECTORY-051: Lys runs and monitors its agents, enforces budgets and goals, and shows authoritative refused acts

> **Cluster:** directory
> **Depends on:** DIRECTORY-050
> **Design anchor:**
> - ADR-118 — Lys owns running and monitoring its agents and enforcing values; Argus reads data for analytics — Lys owns everything about running and monitoring an agent it starts: installing its hooks/status line, reading its session stream and recording tokens, context and time. Lys holds budgets, goals, expectations and deliverables and enforces them through its runner with notices, compaction requests or stops as the value says. The context-watch notices previously driven by Argus become Lys-driven acts. Lys exposes the resulting data for Argus, which keeps analytics and visualisation; Lys does not depend on it. Lys screens are plain controls/state plus the authoritative tool-policy and grant refused-acts list on each agent page. Context compaction, goals and reminders are Lys-owned background acts. Argus may read explicitly permitted Lys variables alongside its data stream; no secret or arbitrary environment export follows from this. DIRECTORY-051 creates the missing tool-boundary policy and its browser editor. It does not claim an OS sandbox. The runner proves socket peer identity in peer.rs and makes grantable decisions against the live grant authority, denying uncertainty. Codex pre-tool refusal coverage is explicitly unavailable. OS process containment is the separate card11VnzLRp.
> **Checklist:**
> - C381 — Lys installs its own session hooks/status line and incrementally follows the runner-local stream; usage is durable, authenticated and exported for optional Argus analytics (DIRECTORY-051 R1).
> - C382 — Budgets for context, tokens and time are set on an agent, a team or a person, and inherited downward (DIRECTORY-051 R2).
> - C383 — A reached budget compacts, stops or tells, as the budget says, once, with a receipt (DIRECTORY-051 R3).
> - C384 — Goals on an agent carry a deadline and reminders delivered into its session (DIRECTORY-051 R4).
> - C385 — Plain controls set and read budgets and goals and show reached or uncertain state; analytics stays in Argus (DIRECTORY-051 R5).
> - C421 — Lys creates an editable tool-boundary policy and records runner/grant denials on the agent page; Codex pre-tool coverage is explicitly unavailable (DIRECTORY-051 R6).
> **Stories:**
> - S156 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want each agent held to a budget for context, tokens and time, compacted or stopped when it reaches it, so that no agent burns what I cannot afford.
> - S157 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to give an agent goals with deadlines and have it reminded, so that work is paced and I can see where each goal stands.

## Purpose

Tom, 28 September 2026 19:50 to 19:51, on Dot: 'Argus is not going to be able to do what we need, to be able to track budgets and token usage, time, goals and everything like that through Lys. We're going to completely rewrite that, in Rust.' Argus keeps crashing, and the budgets and goals our agents run under must live with their identity, grants and sessions, in Lys. Tom, 19:51: 'Not the day-to-day operational tasks. That will still go into Cambium. But I want to start setting budgets and goals and expectations and deliverables that need to be done, and I think the only way to do that is through Lys.' Amended 28 September 2026 20:45 by Waffles after Archie's review of the brief against the tree. Clarification relayed by Waffles from Tom on Dot, 29 September 2026 06:59 to 07:00: Argus remains the real-time observability/control, parsed-stream analytics and visualisation service. Lys consumes tracking behind the scenes and sets and enforces budgets, goals, expectations and deliverables. Its screen is plain controls, not an analytics dashboard. The 06:55 request for an authoritative refused-acts list on each agent's page stands. These later directions replace transcript parsing inside Lys and the earlier page-performance proposal. Final clarification, Tom on Dot at 07:02 as relayed by Waffles: Lys owns running and monitoring its agents, installs their hooks/status line and reads their streams itself. Lys enforces held values and drives context notices, compaction and stops. Lys can export the resulting data to Argus, which keeps analytics and visualisation. This replaces the 06:59 to 07:00 inbound-Argus-feed proposal; no Argus dependency or upstream feed card remains. Independent review of 5cac25e7 returned the absent enforcement source, unnamed harness contract/writers, nonexistent provisioning.rs wall and compound acceptance checks. This correction creates the enforcer, pins the signal contract, creates the settings writers and separates the checks. Waffles and Archie ruled on 29 September 07:18 to07:20: peer.rs is created in051, 060 later reuses it, and grantable rules retain a live deny-on-uncertain check.

## Task

When Lys starts an agent, install the harness hooks/status line, follow the runner-local session stream incrementally and record measured usage. Hold and enforce budgets, goals, expectations and deliverables through the runner; show plain values and their state, plus authoritative refused acts on each agent page. Expose figures for Argus analytics without relying on Argus to run or monitor a session. Base implementation on complete, green DIRECTORY-050; 051 has not previously been built. Archie independently signs this brief before dispatch. All six requirements need real acceptance evidence before the card is complete. R6 supplies a narrow judge/collector peer proof within this card, so 060 is not an implementation prerequisite. Its later session-key proof must reuse or reconcile this primitive rather than silently weaken it.

## Requirements

### R1: Lys-owned hooks, status line and incremental session-stream tracking

Behavioural. When starting a session, Lys installs its own supported harness hooks and status-line collector in the isolated provisioned home, and starts a stream follower on the runner machine. It does not rewrite a person's global harness configuration or depend on Argus. Use actual supported Claude Code and Codex integration points verified from the installed harness contract; inventory each supported signal before implementation. A harness lacking a requested required signal is refused by name, not given a fabricated hook. The hooks/status line are bounded local collection and enforcement adapters; no synchronous telemetry or analytics network call lies on the turn path. R6 live grant-authority checks are intentional security enforcement: they are the explicit exception to the no-network-telemetry rule. Analytics, durable aggregation and UI reads run off the session path. An explicit enforcement block is distinguished from incidental telemetry delay. Preserve pre-existing permitted session hooks/settings when installing the Lys-owned entries, never duplicate the entries on restart. Each local collector connection uses R6's in-card peer credentials and ancestry proof to bind its session. A body naming another session cannot change attribution. No readable local capability file or environment token substitutes for that peer proof. Deliver figure records to the server through the existing signed runner protocol extended with a versioned tracking read/follow act and opaque durable cursor. Do not treat a fresh connection challenge as a durable event identity. A record names version, source runner/session, generation, source offset or stable event id, turn id, observed instant, kind and measured figures. Count each completed turn once across hook/status-line/stream overlap, using declared source precedence and stable identities, not summing overlapping snapshots. Measure input, output, cache tokens, context in use, running time and paying account; unavailable figures remain null with a named reason. Context window is explicitly declared and validated in the profile, and that profile version is retained; missing window refuses window_undeclared. Account attribution follows the runner's rotation evidence at the turn instant. Follow stream changes by filesystem notifications; persist generation/offset and incomplete-line boundary. Never reread an entire file after restart, poll it on a timer or silently skip an unreadable line; truncation, rotation or lost source coverage is named and reconciled. Persist received rows and their consumed cursor atomically with replay deduplication. Expose authorised paged GET /agents/{id}/usage and GET /runtime/sessions/{id}/usage plus a new authenticated GET /tracking/events cursor-follow route for Argus to read figures. This outbound read uses Lys service-account grants with an explicit tracking_read action in the configured grant model, preserving the data-driven model semantics; unauthenticated or ungranted reads refuse. Export version, stable event identity, source generation/cursor, agent/session binding, instant and figures only, never transcript text or credentials. Replay-to-live handover loses no event; an expired cursor gives a named gap rather than silently restarting. A slow or disconnected Argus reader cannot block collection, an agent turn or enforcement. Named stale/incomplete tracking stays visible; missing measurements cannot prove a hard cap was respected. rules/ast-grep/no-poll.yml must reject a deliberately inserted proof poll, then pass the final tree. The 07:04 clarification permits Argus to read Lys variables as well as its stream. Export only explicitly granted tracking/budget/goal values, with the same holder visibility checks as the control API; never expose arbitrary process environment or secret values. Concrete harness contract: Claude Code's isolated settings.json is written by new claude_code/settings.rs, integrated with render.rs, render_write.rs, template.rs and launch.rs. Install command hooks SessionStart (bind session/transcript), UserPromptSubmit (turn start), Stop (turn end, honour stop_hook_active), SessionEnd (end), PreCompact (compaction observation), PreToolUse with matcher * (R6 judge), PostToolUse and PostToolUseFailure (outcome observations, not proof of policy denial). Commands invoke the new lys runner-hook CLI with the event name and send JSON on stdin to the local runner adapter. Common input is session_id, transcript_path, cwd and hook_event_name; tool events additionally carry tool_name, tool_input and tool_use_id. PreToolUse denial returns hookSpecificOutput with hookEventName PreToolUse, permissionDecision deny and permissionDecisionReason. A non-denial does not return allow or bypass the harness's own permissions. The installed statusLine command invokes lys runner-statusline; stdin supplies session_id, transcript_path, model.id, context_window.current_usage (input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens), context_window.context_window_size and cost.total_duration_ms when present. Missing or null values stay unavailable. Status-line context figures are snapshots, never added as spend deltas. The session's validated transcript_path names the Claude JSONL under its isolated config home's projects/<cwd-slug>/<session-id>.jsonl; follow only that file by generation and byte offset. Per-response spend comes from source message usage with message/request identity deduplication; partial records do not become turns. Hook/outcome observations do not count as extra usage. The Codex writer is NEW harness/codex/config.rs: it writes config.toml in the isolated CODEX_HOME and sets notify to an argv array invoking lys runner-notify. The final argv is JSON with type agent-turn-complete, thread-id, turn-id and cwd; message-text fields are discarded. This is after-turn notification, never a pre-tool denial hook or a source of token figures. Bind the thread to the actual session_meta in CODEX_HOME/sessions/YYYY/MM/DD/rollout-<timestamp>-<id>.jsonl before following its offsets. Read event_msg/token_count info.last_token_usage, total_token_usage and model_context_window; treat repeated totals as snapshots and derive deltas under a named adapter version. No Codex statusLine command or pre-tool policy hook is invented. Check the exact launched harness version against the measured adapter contract; unsupported versions fail with tracking_contract_unsupported or explicit missing coverage, never guessed compatibility. The inspected local codex-channels source was 7cbba483f7c83aed15c5fbc0258c72651b6fdf91; it is not evidence that the PATH codex-cli 0.36.0 is that executable. The chain must record the executable it actually launches.

**Acceptance:**
- The rendered isolated Claude Code settings contain each named hook exactly once.
- The isolated Claude Code settings contain the lys runner-statusline command.
- The global Claude Code settings are byte-identical after a Lys launch.
- A SessionStart fixture binds the validated transcript_path to the launched session.
- A Stop fixture with stop_hook_active true cannot create a repeated context-watch act.
- The Claude usage fixture matches independently counted per-response spend.
- Repeated status-line snapshots do not add spend.
- Null status-line current_usage is recorded as unavailable.
- The isolated Codex config.toml contains the lys runner-notify argv.
- The global Codex configuration is byte-identical after a Lys launch.
- The Codex notify fixture yields the expected thread-id and turn-id.
- The Codex token_count fixture yields the independently expected token deltas.
- A rollout whose session_meta names another thread is refused.
- The chain records the launched harness executable and measured adapter version.
- An unsupported adapter version is refused tracking_contract_unsupported.
- Restart with a partial final line preserves the unconsumed bytes.
- Repeated delivery yields one durable row for the completed source event.
- Instrumented restart reads start at the saved stream offset.
- Stream truncation produces an explicit source-generation transition.
- An unreadable stream record retains its offset and refusal reason.
- A collector from another session cannot submit a record for this session.
- An invalid profile window is refused.
- A valid declared profile window is used exactly.
- An account-rotation fixture attributes the measured turn to its original paying account.
- Ambiguous paying-account evidence remains unknown.
- A disconnected analytics reader cannot delay a runner turn.
- Collection pressure produces an explicit incomplete-coverage state.
- An authorised Argus reader resumes export after the saved cursor without a missing event.
- An ungranted Argus reader is refused.
- No stored usage record contains transcript text.
- No exported record contains a session authentication material.
- The no-poll rule fails on the deliberate proof poll.
- The no-poll rule passes on the final tree.
- Argus reads a permitted budget value through its authorised API.
- Argus cannot read another holder's ungranted variable.

**Files:**
- create: crates/lys-runner/src/tracking.rs
- create: crates/lys-runner/src/tracking_store.rs
- create: crates/lys-runner/tests/tracking.rs
- create: crates/lys-home/src/harness/tracking.rs
- create: crates/lys-home/src/harness/tracking_tests.rs
- create: crates/lys-identity-server/src/usage_api.rs
- create: crates/lys-identity-server/src/usage_state.rs
- create: crates/lys-identity-server/src/usage_store.rs
- create: crates/lys-identity-server/src/tracking_export.rs
- create: crates/lys-identity-server/tests/usage.rs
- create: rules/ast-grep/no-poll.yml
- create: crates/lys-home/src/harness/claude_code/settings.rs
- create: crates/lys-home/src/harness/claude_code/settings_tests.rs
- create: crates/lys-home/src/harness/codex/config.rs
- create: crates/lys-home/src/harness/codex/config_tests.rs
- create: crates/lys/src/commands/runner_hook.rs
- create: crates/lys/src/commands/runner_statusline.rs
- create: crates/lys/src/commands/runner_notify.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/state.rs
- modify: crates/lys-runner/src/socket.rs
- modify: crates/lys-runner/src/session/lifecycle.rs
- modify: crates/lys-runner/Cargo.toml
- modify: crates/lys-home/src/harness/mod.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs
- modify: crates/lys-home/src/harness/claude_code/launch.rs
- modify: crates/lys-home/src/harness/claude_code/launch_env.rs
- modify: crates/lys-home/src/harness/codex/mod.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/main.rs
- modify: crates/lys-identity-server/src/config.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/provisioning_api.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: crates/lys-identity-server/Cargo.toml
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/src/harness/claude_code/render_write.rs
- modify: crates/lys-home/src/harness/claude_code/template.rs
- modify: crates/lys-identity-server/src/provisioning_store.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/commands/mod.rs
- modify: crates/lys/Cargo.toml

**Checklist:**
- C381 — Lys installs its own session hooks/status line and incrementally follows the runner-local stream; usage is durable, authenticated and exported for optional Argus analytics (DIRECTORY-051 R1).

**Stories:**
- S156 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want each agent held to a budget for context, tokens and time, compacted or stopped when it reaches it, so that no agent burns what I cannot afford.

### R2: Budgets on agents, teams and people

Behavioural. A budget names its holder (agent, team or person), its measure (context percent, tokens per period, running time per period), its limit, and its act (compact, stop or tell), set by the responsible person or an administrator. A narrower holder's budget applies before a wider one's; the tightest reached budget acts. Budgets are log events with versions, readable and changeable through the API. A budget's period names its time zone explicitly; there is no machine default, and a period without a zone is refused zone_missing. Holder visibility and authority are checked on reads as well as changes. Aggregate only the members/period the budget names; replaying a usage row adds no spend. Explicit zone and period boundaries, and changes of budget version, are recorded so a restart does not invent a new crossing.

**Acceptance:**
- A team budget applies to each agent it covers.
- A tighter agent budget takes precedence.
- An unauthorised budget change is refused not_permitted.
- A period-boundary fixture yields the expected total in the named zone.
- Restart preserves the budget period.
- A repeated usage event does not increase spend.
- Another person's budget is not disclosed.

**Files:**
- create: crates/lys-identity-server/src/budgets_api.rs
- create: crates/lys-identity-server/src/budgets_state.rs
- create: crates/lys-identity-server/tests/budgets.rs
- create: crates/lys-identity-server/src/budgets_store.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C382 — Budgets for context, tokens and time are set on an agent, a team or a person, and inherited downward (DIRECTORY-051 R2).

**Stories:**
- S156 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want each agent held to a budget for context, tokens and time, compacted or stopped when it reaches it, so that no agent burns what I cannot afford.

### R3: A reached budget acts once

Behavioural. When a measured figure reaches a budget, Lys does the budget's act once per crossing: compact sends the harness's compaction command through the runner at the next turn boundary; stop ends the session through the runner and reports it confirmed; tell delivers a notice to the responsible person. Each act leaves a receipt naming the budget, the figure and the instant. A compaction that does not bring context under the limit is reported, not retried in a loop. A notice ('tell') is a record shown on the Usage screen and read through the API; Lys sends nothing outward. Persist each crossing's stable operation identity and requested act before delivery. Reconcile the same operation against the runner's receipt after a crash or an uncertain reply; never issue a replacement act merely because the answer was lost. A stop is shown confirmed only on actual runner process-exit evidence. Unknown delivery remains visible as unconfirmed until reconciled, rather than being presented as success. A fresh connection challenge is not an idempotency key for an act. Extend the runner with stable operation identities and durable outcome readback shared by compaction and reminder input. If a crash leaves terminal input delivery unknowable, retain uncertain state and do not type again; do not promise exactly-once external input from a durable log alone. Lys, not Argus, drives the context-watch notices and threshold acts. The held value selects notice, compaction request or stop. A required block is actually applied at the runner boundary and recorded; merely displaying a reached threshold is not enforcement. A context notice to the session uses the same durable operation protocol as reminder input, separate from the tell notice shown to the responsible person.

**Acceptance:**
- A context crossing sends one compaction at the next permitted boundary.
- A hard token cap ends the session process.
- The stopped session is shown confirmed only after runner exit evidence.
- Each confirmed act has one receipt.
- A crash before delivery resumes the original operation identity.
- A crash after runner acceptance does not send a replacement act.
- An unknowable input-delivery outcome remains unconfirmed.
- A configured context notice appears in the live session.
- A stopped session cannot execute its next requested act.

**Files:**
- create: crates/lys-identity-server/src/budgets_act.rs
- create: crates/lys-runner/src/operations.rs
- create: crates/lys-runner/tests/operations.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: crates/lys-identity-server/src/receipts_api.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/state.rs
- modify: crates/lys-identity-server/src/runner_acts.rs
- modify: crates/lys-runner/src/lib.rs

**Checklist:**
- C383 — A reached budget compacts, stops or tells, as the budget says, once, with a receipt (DIRECTORY-051 R3).

**Stories:**
- S156 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want each agent held to a budget for context, tokens and time, compacted or stopped when it reaches it, so that no agent burns what I cannot afford.

### R4: Goals, expectations and deliverables, with deadlines and reminders

Behavioural. A goal on an agent or a team is one of three kinds: a goal (an outcome), an expectation (a standard its work is held to) or a deliverable (a named thing handed over, with the evidence that proves it: a landed commit, a document, a passing check). Each has words, a deadline, a state (open, met, missed, dropped) and reminders (before the deadline, at intervals, or on events such as a compaction). Lys delivers each reminder into the agent's live session through the runner, with the goal's words and time left, and records the delivery or its refusal. Reminders are timers held in the log so a restart keeps them; none is lost or fired twice. Day-to-day operational tasks stay in the product that runs them; Lys holds only these three kinds. Only the item's responsible person, or the holder of a relation the plan names for it, may mark a goal, expectation or deliverable met; the agent it judges may never mark its own, refused not_your_judgement. Lys calls no app, so evidence is recorded as a claim by the person who marks it, in those words, naming them. A reminder that fell due while the server was down fires once at start, recorded as late with both the due and the fired instant. Reminder delivery uses the same durable operation/receipt reconciliation rule as R3. Restart recovery distinguishes pending, accepted, confirmed and uncertain delivery; a crash after typing but before recording the receipt must not type a second reminder. Goal reads and changes preserve holder visibility and the named judgement authority. R3 owns the shared runner operation/receipt seam; no second reminder delivery mechanism is added.

**Acceptance:**
- A due reminder is delivered once.
- Restart preserves the reminder's due instant.
- Marking a goal met cancels its pending reminders.
- A deliverable without named evidence is refused evidence_missing.
- The judged agent marking its own deliverable met is refused not_your_judgement.
- A reminder due while stopped is recorded as late.
- A late reminder retains its original due instant.
- A late reminder retains its actual fired instant.
- A crash after acceptance does not type the reminder twice.
- An uncertain reminder is not labelled delivered.

**Files:**
- create: crates/lys-identity-server/src/goals_api.rs
- create: crates/lys-identity-server/src/goals_state.rs
- create: crates/lys-identity-server/tests/goals.rs
- create: crates/lys-identity-server/src/goals_store.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C384 — Goals on an agent carry a deadline and reminders delivered into its session (DIRECTORY-051 R4).

**Stories:**
- S157 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to give an agent goals with deadlines and have it reminded, so that work is paced and I can see where each goal stands.

### R5: Plain budget and goal controls

Behavioural. A small Usage screen under the actual routes and navigation shows authorised current budgets, goals, expectations and deliverables, with their values, deadline and held/reached/open/met/missed/dropped state. Styled forms set permitted values and show named refusals and pending/unconfirmed acts. Tracking freshness or missing coverage is visible in plain words. This is not a dashboard: graphs, per-turn visual analytics and observability remain in Argus; do not add them here. The API may retain paged figures for authorised reads. No page-load bound is invented in this brief.

**Acceptance:**
- An authorised person changes a budget through the navigable control screen.
- The changed budget remains after reload.
- A goal entered in the screen remains after reload.
- A reached budget is shown in plain words.
- An uncertain act is not shown confirmed.
- An unrelated caller cannot read another holder's values.
- An unrelated caller cannot change another holder's values.
- Missing runner tracking is shown as incomplete.
- The screen contains no analytics dashboard.

**Files:**
- create: surface/identity/src/features/usage/Usage.tsx
- create: surface/identity/src/features/usage/usage.css
- create: surface/identity/tests/usage.test.tsx
- modify: surface/identity/src/routes.tsx
- modify: surface/identity/src/shell/Shell.tsx
- modify: surface/identity/src/api.ts
- modify: crates/lys-identity-server/src/surface.rs

**Checklist:**
- C385 — Plain controls set and read budgets and goals and show reached or uncertain state; analytics stays in Argus (DIRECTORY-051 R5).

**Stories:**
- S156 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want each agent held to a budget for context, tokens and time, compacted or stopped when it reaches it, so that no agent burns what I cannot afford.
- S157 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to give an agent goals with deadlines and have it reminded, so that work is paced and I can see where each goal stands.

### R6: Lys creates tool-boundary policy enforcement and shows authoritative refusals

Behavioural. There is no existing runner sandbox or hook policy in the DIRECTORY-050 baseline. This requirement creates a tool-boundary enforcer; it does not provide OS process containment. Each agent has a versioned policy in the Lys log, accessed through new GET/POST /agents/{id}/policy and edited on its agent page by a person authorised over it. The policy JSON has version, agent, rules; a rule has id, tool (an exact tool name), kind (tool, path_prefix or host), target (absent for a whole tool, an absolute path prefix, or a canonical hostname), and authority (hard, or permission with a named Lys resource and action). The responsible authority is established by the current grant model, not a person id supplied in the policy. Validate the shape and reject duplicate rule ids or ambiguous targets. Policies are deny constraints: no matching rule permits only the normal harness permission flow, never an override. A hard rule always denies. A grantable rule can cease denying only after the live server grant authority permits the session's bound agent to exercise the named action on the named resource. Unknown or unavailable authority is deny; no cached earlier grant proves current permission. Every decision names the immutable policy version installed for that launch. The server passes the policy in the signed launch act; the runner keeps it under the proved session identity. Changes apply on the next launch and the editor plainly says so.

The generated Claude Code PreToolUse command invokes the new lys runner-judge client. It reads the hook JSON from stdin and sends the structured tool name/input and attempt id over the runner Unix socket. This card creates peer.rs: read Unix peer credentials (macOS LOCAL_PEERPID, Linux SO_PEERCRED), require the runner's uid, walk the peer's ancestry to the recorded session leader, and compare the leader's process start identity to the one recorded at spawn. A same-user unrelated process or a reused pid is not the session. Read macOS process ancestry/start identity through the safe libproc interface and Linux through /proc process records; use nix 0.31.3 with LocalPeerPid on macOS and PeerCredentials on Linux, and libproc 0.14.11 on macOS. Hold the connected peer socket open throughout proof and answer. Compare the leader start identity before and after the ancestry walk; any changed or unprovable identity denies. Broken or unverifiable ancestry denies. Peer identity is derived from the connected socket, never from the request body. This proof is new in051, not assumed present in050. Its peer-only allowlist is judge and collector; it cannot call server administration. Keep the server-signed control acts unchanged. Record start identity under the existing050 lifecycle correction; no unsafe block is added.

For a grantable rule the runner asks the live server grant authority through the authenticated control channel, carrying the proved session/agent, policy version, requested resource/action and stable attempt identity. The server uses the existing grant owner and current directory projection. This security decision is an explicit exception to telemetry network isolation: it may wait for the caller, but it does not add analytics work to the turn. Unavailable, cancelled or uncertain answers deny with grant_state_unavailable; a cached permission is never a substitute. The server records a grant-check refusal at its authority boundary. Correlate that event and the runner's tool decision by attempt identity without losing either source. A response for another attempt or an obsolete policy version is refused. A confirmed revocation governs the next live grant check. No new request timeout is introduced.

For paths, inspect only structured Read/Write/Edit file_path and Glob/Grep path fields. Resolve relative targets against the bound session working directory; normalise dot segments and resolve existing symlink parents before a component-boundary prefix comparison. Refuse ambiguous or unresolvable targets. For hosts inspect WebFetch's initial URL hostname, normalised with the actual URL parser; a redirect or later subprocess connection is outside this hook's coverage. With a restrictive policy, Bash, PowerShell, unknown MCP tools and any operation whose effects cannot be resolved from these named fields are denied policy_uninspectable. Do not parse shell text as a security authority. Subagent spawning is likewise uninspectable unless its independent session is proved to inherit enforcement; no claim of coverage follows from a parent hook alone.

On denial the runner durably appends one refusal before responding: source id, stable attempt id, proved session, instant, tool, safe target summary, policy version, rule and grantability. Repeated delivery of that attempt reuses the record; a lost reply does not create a new denial attempt. A failure to append never becomes permission; return a named denial with incomplete audit coverage. The client emits Claude Code's hookSpecificOutput PreToolUse deny response with its reason. A non-denial emits no allow override. Unreachable runner or failed peer proof returns deny. An unproved request writes no attributed refusal for a named session. Prove this behavior with fault tests, including a dead runner. The installed hook's integrity and capability are checked before claiming coverage; hook policy is not protection against arbitrary same-user process tampering.

Each agent page shows the authenticated refusal records newest first. The row names the attempted act, refusing rule/check, and a real permission/grantor only where the current server authority establishes one. Hard rules say no grant can allow them. Delivery to the server is by the runner control channel with durable cursor/replay; new GET /agents/{id}/refusals and GET /agents/{id}/refusals/events check visibility on every read/event. Source disconnection or missing coverage is visible; an empty list is not an all-clear. Agent prose and PostToolUseFailure text are not authoritative policy denials. No record contains secret values or unredacted tool bodies. The Codex notify contract has no Lys-owned pre-tool denial hook: show 'refusals are not observed for Codex sessions yet'. Codex tracking and runner stop acts still work, but a profile requiring this pre-tool policy refuses policy_not_supported rather than running unguarded. Process containment is separate card 11VnzLRp, after 050 and051; no screen or receipt calls this hook an OS sandbox. grants.rs gains no code line; use grants_refusals.rs with minimal relocation preserving grant decisions. scripts/file-length.sh must pass.

**Acceptance:**
- A permitted person saves a deny-Write-under-/etc policy through the agent page.
- The policy editor states that the saved version applies on the next launch.
- The next scratch launch receives that policy version.
- A Write attempt for /etc/lys-probe is denied by the Lys PreToolUse hook.
- The initially absent /etc/lys-probe remains absent after the attempt.
- The runner log contains exactly one refusal for that attempt.
- The refusal record names the proved session.
- The refusal record names the matched rule.
- The refusal is the newest row on the agent page in the acceptance browser.
- Fault injection after append proves the durable refusal precedes the hook response.
- Replaying the same attempt after a lost answer creates no second refusal record.
- A peer outside the session ancestry receives deny.
- An unproved peer writes no attributed refusal for the claimed session.
- A stopped runner causes the hook client to deny.
- A failed refusal-log append cannot yield permission.
- The page shows incomplete audit coverage for the failed append.
- A hard rule is displayed as not grantable.
- A grantable denial names the required permission.
- A grantable denial names a person whose authority the grant owner verifies.
- After a confirmed revocation, the next live grant check denies the tool call.
- An unavailable live grant authority denies with grant_state_unavailable.
- A non-denial never overrides a native harness refusal.
- A sibling path such as /etc-other does not match the /etc component prefix.
- A symlink into a denied prefix is denied by the path rule.
- An ambiguous path target is denied.
- A denied WebFetch initial hostname is denied before that tool executes.
- Bash is denied policy_uninspectable under the restrictive policy.
- An unknown tool is denied policy_uninspectable under the restrictive policy.
- An unproved subagent tool is denied policy_uninspectable.
- The Codex session page states that pre-tool refusal coverage is not observed.
- A Codex profile requiring pre-tool enforcement refuses policy_not_supported.
- A replayed server refusal row appears once after restart.
- An unrelated viewer sees no protected refusal record.
- Visibility withdrawal stops protected events on an already-open stream.
- A disconnected source is shown as incomplete coverage.
- An agent statement claiming denial creates no authoritative refusal.
- The browser never labels hook policy as an OS sandbox.
- The final grants.rs code-line count does not increase.
- A reused leader pid with a different process start identity receives deny.
- The diff adds no unsafe Rust block.
- A grant answer for a different attempt cannot permit the requested act.

**Files:**
- create: crates/lys-identity-server/src/refusals_api.rs
- create: crates/lys-identity-server/src/refusals_store.rs
- create: crates/lys-identity-server/tests/refusals.rs
- create: crates/lys-runner/src/refusals.rs
- create: crates/lys-runner/tests/refusals.rs
- create: surface/identity/src/features/file/AgentRefusals.tsx
- create: surface/identity/tests/agent-refusals.test.tsx
- create: surface/identity/tests/acceptance/refusals.spec.ts
- create: crates/lys-identity-server/src/grants_refusals.rs
- create: crates/lys-runner/src/judge.rs
- create: crates/lys-runner/src/refusal_log.rs
- create: crates/lys-runner/tests/judge.rs
- create: crates/lys/src/commands/runner_judge.rs
- create: crates/lys-identity-server/src/agent_policy_api.rs
- create: crates/lys-identity-server/src/agent_policy_store.rs
- create: crates/lys-identity-server/tests/agent_policy.rs
- create: surface/identity/src/features/file/AgentPolicy.tsx
- create: surface/identity/tests/agent-policy.test.tsx
- create: crates/lys-runner/src/peer.rs
- create: crates/lys-runner/tests/judge_peer.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/state.rs
- modify: surface/identity/src/features/file/IdentityFile.tsx
- modify: surface/identity/src/api.ts
- modify: surface/identity/src/features/file/sections.tsx
- modify: surface/identity/src/features/file/tabs.ts
- modify: surface/identity/vite.config.ts
- modify: crates/lys-runner/src/socket.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/commands/mod.rs
- modify: crates/lys-identity-server/src/runner_client.rs
- modify: crates/lys-identity-server/src/runner_sessions.rs
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/src/harness/claude_code/render_write.rs
- modify: crates/lys-home/src/harness/claude_code/template.rs
- modify: crates/lys-runner/Cargo.toml
- modify: crates/lys-runner/src/session/lifecycle.rs
- modify: crates/lys-identity-server/src/grant_sight.rs

**Checklist:**
- C421 — Lys creates an editable tool-boundary policy and records runner/grant denials on the agent page; Codex pre-tool coverage is explicitly unavailable (DIRECTORY-051 R6).

**Stories:**
- S156 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want each agent held to a budget for context, tokens and time, compacted or stopped when it reaches it, so that no agent burns what I cannot afford.

## Boundaries

- SHALL NOT depend on Argus for collection, session running or enforcement; Argus is an optional reader of exported analytics figures.
- SHALL NOT measure usage or refusals from the agent's own claims.
- SHALL NOT add synchronous analytics/network telemetry work to a session turn; local enforcement hooks and live grant decisions are required and distinguished from analytics.
- SHALL NOT poll or reread whole transcripts: following uses source change notifications and durable generation/offset; reminders use timers held in the log.
- SHALL NOT add a request timeout, #[allow], #[ignore] or any bypass.
- SHALL NOT add silent fallbacks, anonymous telemetry access, or a permission bypass for a hard tool-policy refusal.
- SHALL NOT claim hook enforcement is OS sandboxing, process containment, complete host/network isolation, or Codex pre-tool enforcement.

## Verification

- The full Lys chain (Jev, fmt, Clippy pedantic, tests, ast-grep and gate) and surface acceptance exit 0 at the final card head, on Dean's laptop.
- On a scratch install run an agent with Lys-installed hooks/status line and stream tracking, show a held value causing a real notice/compaction/stop, one goal reminder and a real denied act visible on the agent page. Argus may read the exported figures for analytics; stopping that reader must not stop Lys enforcement. No terminal-only demonstration or installed-production claim.
- scripts/file-length.sh exits 0 on the final tree; grants.rs gains no code line and new helpers stay within the gate.
- Harness source grounding: https://code.claude.com/docs/en/hooks and https://code.claude.com/docs/en/statusline were inspected on 29 September 2026. Codex source grounding is codex-channels commit 7cbba483f7c83aed15c5fbc0258c72651b6fdf91, hooks/src/legacy_notify.rs, hooks/src/types.rs, rollout/src and protocol/src/protocol.rs. These sources define test inputs; they do not replace the actual launched-binary acceptance proof.
