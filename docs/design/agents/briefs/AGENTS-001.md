---
type: brief
id: AGENTS-001
cluster: agents
title: Prompt words, variables, countdowns and schedules, held in Lys and rendered at delivery
---

# AGENTS-001: Prompt words, variables, countdowns and schedules, held in Lys and rendered at delivery

> **Cluster:** agents
> **Depends on:** DIRECTORY-051, DIRECTORY-064
> **Design anchor:**
> - ADR-118 — Lys owns running and monitoring its agents and enforcing values; Argus reads data for analytics — Lys owns everything about running and monitoring an agent it starts: installing its hooks/status line, reading its session stream and recording tokens, context and time. Lys holds budgets, goals, expectations and deliverables and enforces them through its runner with notices, compaction requests or stops as the value says. The context-watch notices previously driven by Argus become Lys-driven acts. Lys exposes the resulting data for Argus, which keeps analytics and visualisation; Lys does not depend on it. Lys screens are plain controls/state plus the authoritative tool-policy and grant refused-acts list on each agent page. Context compaction, goals and reminders are Lys-owned background acts. Argus may read explicitly permitted Lys variables alongside its data stream; no secret or arbitrary environment export follows from this. DIRECTORY-051 creates the missing tool-boundary policy and its browser editor. It does not claim an OS sandbox. The runner proves socket peer identity in peer.rs and makes grantable decisions against the live grant authority, denying uncertainty. Codex pre-tool refusal coverage is explicitly unavailable. OS process containment is the separate card11VnzLRp.
> - ADR-130 — Managed harness channels carry automatic context and reminder acts — 064 adds one runner-owned typed control/event channel per managed Claude or Codex session. It reuses 051 operation and usage owners, serialises all inputs at proved boundaries, requires actual harness evidence and retains uncertain outcomes without automatic resend. Legacy PTY sessions remain manual and cannot claim this capability. 065 extends the same Codex event reader for refusal coverage.
> - ADR-137 — Argus's remaining functions move into Lys as data beside goals, and a seat is a Lys record launched by the runner through the proxy; no shared library — Words (five slots, four layers, templates), variables (per agent and session, revision, author, expiry), schedules and seats are identity-server records in Lys's log, rendered and acted on at delivery through DIRECTORY-064's dispatcher with the contributing revisions in the receipt. No shared library: Lodestone is a read-only query engine over a store and holds nothing; haematite is the store under Lys already; a library would be a second place for the same data. Seat start, stop and restart are deliberate acts under Lys rights (ADR-136). Argus's alarms, rules, launches and queue are not moved into Lys: they become liminal services and aion work (ADR-136 section 3). A seat moves off herdr and Argus one at a time, Waffles's first.
> **Checklist:**
> - C701 — The five message slots (context warning, preparation, compaction, wake-up, scheduled reminder) resolve built-in, then workspace, then agent, then session, with named templates; an empty layer inherits (AGENTS-001 R1).
> - C702 — An agent's and a session's variables are JSON values with a revision, an author and an optional expiry; a stale revision is refused by name; the agent reads and sets its own through lys and the Lys MCP server (AGENTS-001 R2).
> - C703 — Delivery renders {{vars.key | fallback}}, {{goals}} and {{time_left}} at the moment of delivery and captures the rendered text with every contributing revision in the operation's receipt (AGENTS-001 R3).
> - C704 — A schedule has at, until, interval, max occurrences and recipients; missed intervals coalesce to one send; an uncertain delivery stops it for inspection; every state survives restart (AGENTS-001 R4).
> - C705 — The Usage screen shows and sets words, variables and schedules with named refusals (AGENTS-001 R5).
> **Stories:**
> - S402 (Tom, owner) — As the owner, I want to set a seat's words, variables, countdowns and schedules in one place and see what was actually delivered, so that overnight work runs without a second system.
> - S403 (Archie, an agent) — As an agent, I want to read and set my own variables with a revision, so that my focus survives a compaction and nobody overwrites it blind.
> - S404 (Archie, an agent) — As an agent, I want a reminder to carry the goal's words and the time left, so that I can plan the hour.

## Purpose

Argus holds the words a seat is sent (five messages in four layers with templates), the variables a seat keeps ({{vars.key | fallback}} with revision and expiry), the goals placeholder and the scheduled messages (tools/argus/docs/agent-prompts.md, scheduled-messages-20260907.md). DIRECTORY-051 holds goals with deadlines and reminders and DIRECTORY-064 delivers at turn boundaries with receipts, but neither holds the words, the variables, the countdown or a schedule's cadence. Tom, 6 October 15:2x: the variables, countdowns and scheduling are needed; where they live is to be decided; maybe no library at all. ADR-137: they live in Lys beside goals.

## Task

Add words (layered slots with templates), variables (per agent and session, with revision, author and expiry), rendering at delivery (variables, goals, time left) with the contributing revisions captured in 064's receipt, schedules (at, until, interval, max occurrences, recipients, coalescing, uncertain stops), the lys and MCP verbs for an agent's own variables, and the Usage screen's controls. Nothing here delivers by itself: every delivery is a 064 operation.

## Requirements

### R1: Words in four layers with templates

Behavioural. THE SYSTEM SHALL hold five message slots (context warning, preparation, compaction command, wake-up, scheduled reminder) at four layers (built-in wording in code, workspace, agent, session), each layer either its own text, a link to a named template, or empty meaning inherit; a slot resolves most specific first; the built-in context warning SHALL carry the marker [Lys context watch]; words are records in the identity server's log with a revision, and a save with a stale revision is refused by name; a preview renders a slot for a session with the numbers the caller supplies and sends nothing.

**Acceptance:**
- Set an agent-layer warning; a session with no session-layer text resolves to it; set a session-layer link to a template; editing the template changes the next resolution; a stale-revision save is refused by name; the preview returns the text and the contributing revisions and the delivery log is unchanged.

**Files:**
- create: crates/lys-identity-server/src/words_api.rs
- create: crates/lys-identity-server/src/words_state.rs
- create: crates/lys-identity-server/src/words_store.rs
- create: crates/lys-identity-server/tests/words.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C701 — The five message slots (context warning, preparation, compaction, wake-up, scheduled reminder) resolve built-in, then workspace, then agent, then session, with named templates; an empty layer inherits (AGENTS-001 R1).

**Stories:**
- S402 (Tom, owner) — As the owner, I want to set a seat's words, variables, countdowns and schedules in one place and see what was actually delivered, so that overnight work runs without a second system.

### R2: Variables per agent and per session, with revision and expiry

Behavioural. THE SYSTEM SHALL hold, for an agent and for a session, a map of lowercase names to JSON values with a revision, an author and an optional expiry per key; a patch supplies the revision read, an author and a non-empty map, leaves omitted keys unchanged, deletes a key with null, and is refused by name on a stale revision; an expired key reads as absent and is named expired; the agent SHALL read and patch its own session's and its own agent's variables through `lys variables get|set` and the Lys MCP server's change verb, presented through the runner (DIRECTORY-060), and no other identity's without a grant; every change is a log record and the Usage screen shows the current map and who changed it.

**Acceptance:**
- Two patches with the same revision: the second is refused by name; a key with an expiry reads as absent after it; the agent patches its own variables through lys and through MCP and another agent's patch is refused with the needed grant named; a restart keeps the map.

**Files:**
- create: crates/lys-identity-server/src/variables_api.rs
- create: crates/lys-identity-server/src/variables_state.rs
- create: crates/lys-identity-server/src/variables_store.rs
- create: crates/lys-identity-server/tests/variables.rs
- create: crates/lys/src/commands/variables.rs
- create: crates/lys/tests/variables.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/commands/mod.rs

**Checklist:**
- C702 — An agent's and a session's variables are JSON values with a revision, an author and an optional expiry; a stale revision is refused by name; the agent reads and sets its own through lys and the Lys MCP server (AGENTS-001 R2).

**Stories:**
- S403 (Archie, an agent) — As an agent, I want to read and set my own variables with a revision, so that my focus survives a compaction and nobody overwrites it blind.

### R3: Rendering at delivery, captured with its revisions

Behavioural. WHEN DIRECTORY-064's dispatcher is about to deliver a slot's words or a schedule's text, THE SYSTEM SHALL render {{vars.key}} and {{vars.key | fallback}} from the session's then the agent's variables, {{goals}} as the recipient's unfinished goals with deadline and time left, {{time_left}} and {{deadline}} for the goal a reminder belongs to, and the context numbers the slot carries, at that moment and never earlier; the rendered bytes and every contributing revision (words, template, variables, goals) SHALL be in the operation's receipt before the write (064 R5); a missing key with no fallback renders empty and the receipt names it; no other substitution exists and the text is never evaluated.

**Acceptance:**
- A reminder with {{time_left}} two hours before its deadline renders the two hours; a variable changed between enqueue and delivery renders the new value and the receipt names both revisions; a missing key renders empty and the receipt names it; a value containing {{ is rendered as text.

**Files:**
- create: crates/lys-runner/src/render.rs
- create: crates/lys-runner/tests/render.rs
- modify: crates/lys-runner/src/lib.rs

**Checklist:**
- C703 — Delivery renders {{vars.key | fallback}}, {{goals}} and {{time_left}} at the moment of delivery and captures the rendered text with every contributing revision in the operation's receipt (AGENTS-001 R3).

**Stories:**
- S404 (Archie, an agent) — As an agent, I want a reminder to carry the goal's words and the time left, so that I can plan the hour.

### R4: Schedules

Behavioural. THE SYSTEM SHALL hold schedules with at (an instant with offset, in the future at creation), optional until (exclusive), interval (none for once, otherwise 60 s to 366 days, anchored to at), optional max occurrences, recipients (agents or sessions), text or a slot, and an author; a due occurrence enqueues one 064 operation per recipient; intervals missed while Lys was down coalesce to one send then continue at the next future slot; a temporary refusal is recorded skipped and counts as attempted; a terminal failure or an uncertain delivery stops the schedule and says so; pause, resume and edit keep the count; every state is a log record and survives restart; nothing fires twice for one occurrence.

**Acceptance:**
- A once schedule fires once; an interval schedule with max 3 stops after 3; stop Lys across two intervals, start it: one send then the next slot; an uncertain delivery stops the schedule and the Usage screen says why.

**Files:**
- create: crates/lys-identity-server/src/schedules_api.rs
- create: crates/lys-identity-server/src/schedules_state.rs
- create: crates/lys-identity-server/src/schedules_store.rs
- create: crates/lys-identity-server/tests/schedules.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C704 — A schedule has at, until, interval, max occurrences and recipients; missed intervals coalesce to one send; an uncertain delivery stops it for inspection; every state survives restart (AGENTS-001 R4).

**Stories:**
- S402 (Tom, owner) — As the owner, I want to set a seat's words, variables, countdowns and schedules in one place and see what was actually delivered, so that overnight work runs without a second system.

### R5: The controls on the Usage screen

Behavioural. THE SYSTEM SHALL extend DIRECTORY-051's Usage screen in place with words (the five slots per layer, templates, preview), variables (the map, revisions, authors, expiry) and schedules (state, next due, occurrences, stop reason), with styled forms, named refusals and no graphs; the API answers paged, authorised reads only.

**Acceptance:**
- Walk: set a workspace warning, preview it for a session, set a variable, make a schedule, see it fire in the delivery log, with pictures.

**Files:**
- create: surface/identity/src/features/usage/Words.tsx
- create: surface/identity/src/features/usage/Variables.tsx
- create: surface/identity/src/features/usage/Schedules.tsx
- create: surface/identity/tests/words.test.tsx
- modify: surface/identity/src/features/usage/Usage.tsx
- modify: surface/identity/src/api.ts

**Checklist:**
- C705 — The Usage screen shows and sets words, variables and schedules with named refusals (AGENTS-001 R5).

**Stories:**
- S402 (Tom, owner) — As the owner, I want to set a seat's words, variables, countdowns and schedules in one place and see what was actually delivered, so that overnight work runs without a second system.

## Boundaries

- No delivery path of its own: every send is a DIRECTORY-064 operation with its receipt.
- No shared library: words, variables and schedules are identity-server records; Lodestone reads, it does not hold; haematite is the store underneath already (ADR-137).
- Alarms, rules and launches are not here: liminal services and aion (ADR-136 section 3).
- No time substitution other than the ones named; text is never evaluated.

## Verification

- This handwritten brief passes scripts/design/gate.sh, judged by its parsed failures, never by its exit code.
- Written whole, read whole by the lead; the whole gate at the sha; then the one battery and install on Tom's Mac; the lead walks the Usage screen with pictures.
- Four-heading handback with the hospital sentence; no test proves less; cargo nextest only; clippy pedantic -D warnings; file length 500.
