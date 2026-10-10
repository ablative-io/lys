---
type: brief
id: AGENTS-002
cluster: agents
title: A seat is a Lys record launched through the proxy, seen, stopped and attached from Lys, and moved off herdr and Argus one at a time
---

# AGENTS-002: A seat is a Lys record launched through the proxy, seen, stopped and attached from Lys, and moved off herdr and Argus one at a time

> **Cluster:** agents
> **Depends on:** DIRECTORY-050, DIRECTORY-051, HOME-037, DIRECTORY-064
> **Design anchor:**
> - ADR-118 — Lys owns running and monitoring its agents and enforcing values; Argus reads data for analytics — Lys owns everything about running and monitoring an agent it starts: installing its hooks/status line, reading its session stream and recording tokens, context and time. Lys holds budgets, goals, expectations and deliverables and enforces them through its runner with notices, compaction requests or stops as the value says. The context-watch notices previously driven by Argus become Lys-driven acts. Lys exposes the resulting data for Argus, which keeps analytics and visualisation; Lys does not depend on it. Lys screens are plain controls/state plus the authoritative tool-policy and grant refused-acts list on each agent page. Context compaction, goals and reminders are Lys-owned background acts. Argus may read explicitly permitted Lys variables alongside its data stream; no secret or arbitrary environment export follows from this. DIRECTORY-051 creates the missing tool-boundary policy and its browser editor. It does not claim an OS sandbox. The runner proves socket peer identity in peer.rs and makes grantable decisions against the live grant authority, denying uncertainty. Codex pre-tool refusal coverage is explicitly unavailable. OS process containment is the separate card11VnzLRp.
> - ADR-130 — Managed harness channels carry automatic context and reminder acts — 064 adds one runner-owned typed control/event channel per managed Claude or Codex session. It reuses 051 operation and usage owners, serialises all inputs at proved boundaries, requires actual harness evidence and retains uncertain outcomes without automatic resend. Legacy PTY sessions remain manual and cannot claim this capability. 065 extends the same Codex event reader for refusal coverage.
> - ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.
> - ADR-137 — Argus's remaining functions move into Lys as data beside goals, and a seat is a Lys record launched by the runner through the proxy; no shared library — Words (five slots, four layers, templates), variables (per agent and session, revision, author, expiry), schedules and seats are identity-server records in Lys's log, rendered and acted on at delivery through DIRECTORY-064's dispatcher with the contributing revisions in the receipt. No shared library: Lodestone is a read-only query engine over a store and holds nothing; haematite is the store under Lys already; a library would be a second place for the same data. Seat start, stop and restart are deliberate acts under Lys rights (ADR-136). Argus's alarms, rules, launches and queue are not moved into Lys: they become liminal services and aion work (ADR-136 section 3). A seat moves off herdr and Argus one at a time, Waffles's first.
> **Checklist:**
> - C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
> - C707 — The Sessions screen shows every seat online, offline, not seen by the runner, or seen but unregistered, from the runner's own liveness, never from a terminal program (AGENTS-002 R2).
> - C708 — Start, stop and restart are deliberate acts under Lys rights; stop goes through the harness's control channel and the runner, never typed keys (AGENTS-002 R3).
> - C709 — lys attach shows a seat's live terminal to its responsible person or an administrator and nobody else (AGENTS-002 R4).
> - C710 — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).
> - C734 — A message sent to a seat from the Sessions screen or `lys seat send` arrives in its Claude Code transcript as a user turn, never as typed keys (AGENTS-002 R6).
> **Stories:**
> - S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.
> - S405 (An operator, a person running the estate) — As an operator, I want to attach to a seat's terminal from Lys when I need to look, so that I do not need a terminal workspace program to own the seats.
> - S418 (Tom, owner) — As Tom, I want to send a message to any seat from the Sessions screen or the lys CLI and see it arrive in that seat as a real turn, so that I can talk to my agents from Lys.

## Purpose

Today a seat is a pane in a herdr workspace, started by Argus's seat_start (a tab, then a wait for the pane), stopped by typing /exit, seen through `herdr agent list` with the registry reconciled against it (online, not seen by herdr, seen but unregistered), and prompted through `herdr agent prompt` (tools/argus/lib/argus/mcp.ex; plugin/scripts/context-text-inject.sh). Lys's runner starts an agent in its own pseudo-terminal (050 R1), carries the profile (HOME-037), installs its hooks and status line (051 R1), counts through the proxy's usage file (tracking_proxy.rs) and will deliver through the control channel (064). Tom, 6 October 15:2x: look at how it is done and do it in Lys, since Lys starts them and runs them through the proxy. Amended 10 October 2026 (Gaia, on Waffles' 15:2x and 15:25 rulings and the live proof on the installed 91e34d4f). The proof joined This computer (op-e515320c…, runner kind lys) and started a throwaway agent. POST /agents/{id}/start refused agent_not_active, then check_record_missing for four records, after activation. That route (start.rs) wires the review, role-machine, credentials and egress records to NotLanded stubs, so it can never pass. Those four records (Ink1H1Os twice, SECRETS-002, network row 8.5) are NOT this brief's: wiring or retiring that route is its own row. POST /agents/{id}/start-command (launch_api.rs start_profile, the screens' route) started the run in 90 ms with ANTHROPIC_BASE_URL on the proxy. Its first call is in data/proxy/state/usage/<run>.jsonl, answered 429, so no tokens were counted. A seat starts by that path. A second machine record naming the same local runner had its feed refused 'BudgetsUnavailable: runner feed usage names a session on another machine' (budgets_feed.rs keep_page), so an install owns exactly one kind-lys machine. Tom's added done-means (via Waffles 15:2x): a message sent to a seat from the Sessions screen and from the lys CLI arrives in that seat as a user turn, read back in its Claude Code transcript, and lys attach shows the live session; proven on Waffles first.

## Task

Add the seat record and its launch through the proxy, the registry with liveness from the runner, start/stop/restart as deliberate acts under rights, lys attach, and the written move of one seat at a time with its proof.

## Requirements

### R1: A seat is a Lys record launched through the proxy

Behavioural. THE SYSTEM SHALL hold a seat as an identity-server record: name, the agent identity it runs as, harness (Claude Code or Codex), profile (HOME-037), home, machine (the runner's), account, and working folder; `lys seat start <name>` SHALL ask the runner to start it (050 R3) with the harness's model base URL pointed at the Lys proxy under the session's run key, Lys's hooks and status line in the provisioned home (051 R1) and no Argus hook or status line, and SHALL record the session id the harness reports; a seat whose profile names a harness or model the machine lacks is refused by name. A seat's start SHALL go through the server's own start path (launch_api.rs start_profile, as POST /agents/{id}/start-command does), never POST /agents/{id}/start, and SHALL refuse seat_not_managed unless the profile's session requires controls, so the runner starts it managed (StartManaged). An install SHALL hold at most one machine whose runner is this install's own (kind lys). Naming a second is refused runner_lys_taken, naming the machine that holds it.

**Acceptance:**
- Start a seat; its first model call appears in the proxy's usage file under its run key; the harness's SessionStart hook reaches the runner's collector and nothing reaches Argus; a profile naming an absent binary is refused by name.
- Naming a second machine's runner {"kind":"lys"} is refused runner_lys_taken naming the first; a seat whose profile does not require controls is refused seat_not_managed.

**Files:**
- create: crates/lys-identity-server/src/seats_api.rs
- create: crates/lys-identity-server/src/seats_state.rs
- create: crates/lys-identity-server/src/seats_store.rs
- create: crates/lys-identity-server/tests/seats.rs
- create: crates/lys/src/commands/seat.rs
- create: crates/lys/tests/seat.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/commands/mod.rs
- modify: crates/lys-identity-server/src/network_store.rs

**Checklist:**
- C706 — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R2: The registry with liveness

Behavioural. THE SYSTEM SHALL show on the Sessions screen every seat with its state from the runner's own knowledge (the process it holds, the harness session bound at SessionStart, the last hook or status line seen): online with working or idle, offline, not seen (a registered seat with no live session), and seen but unregistered (a session the runner holds for no seat), with the time of the last signal; when the runner cannot be read the states are unknown and named so, never zero; no terminal workspace program is consulted.

**Acceptance:**
- Start two seats, stop one: online, offline; kill the runner's child outside Lys: not seen within the follower's next read; a session started by the runner's protocol without a seat: seen but unregistered; runner down: unknown, named.

**Files:**
- create: surface/identity/src/features/sessions/Seats.tsx
- create: surface/identity/tests/seats.test.tsx
- create: crates/lys-runner/src/liveness.rs
- create: crates/lys-runner/tests/liveness.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/session.rs
- modify: surface/identity/src/api.ts

**Checklist:**
- C707 — The Sessions screen shows every seat online, offline, not seen by the runner, or seen but unregistered, from the runner's own liveness, never from a terminal program (AGENTS-002 R2).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R3: Start, stop and restart as deliberate acts

Behavioural. THE SYSTEM SHALL treat seat start, stop and restart as deliberate acts on the seat resource (actions seat.start, seat.stop, seat.restart in Lys's own app schema), asked of the grant engine live with the caller's identity and recorded as grant receipts (ADR-136 D2, D11); stop SHALL go through 064's control channel (the harness's own end-session request) and then the runner (050 R6), never typed keys, refusing by name while the harness reports a turn in progress unless force is named; restart is stop then start with the same profile and a new session; a seat with no responsible person cannot be started.

**Acceptance:**
- Without the right, stop is refused naming seat.stop and the request URL; with it, a stop mid-turn is refused by name, a stop between turns ends the session and the receipt names the grant; restart yields a new session id under the same seat.

**Files:**
- create: crates/lys-identity-server/src/seats_acts.rs
- create: crates/lys-identity-server/tests/seat_acts.rs
- modify: crates/lys-identity-server/src/seats_api.rs
- modify: crates/lys-runner/src/session.rs

**Checklist:**
- C708 — Start, stop and restart are deliberate acts under Lys rights; stop goes through the harness's control channel and the runner, never typed keys (AGENTS-002 R3).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R4: Attach to a seat's terminal

Behavioural. `lys attach <seat>` SHALL show the seat's live session in the caller's terminal. A seat runs managed (R1), so this is drawn from the runner's live frames of that session: user turns, assistant text, tool calls and results, and status. It follows as the session runs. It is read-only by default; with a named flag (--type), each typed line is sent as a user turn through the same input path as R6, never as keys. It is admitted for the seat's responsible person or an administrator and refused by name for anyone else. Detaching never ends the session. Every attach and every typed line is a log record naming who.

**Acceptance:**
- Attach as the responsible person: the seat's session appears and follows as a message lands; as another person: refused by name; detach: the seat runs on; the log names the attach and each typed line.

**Files:**
- create: crates/lys/src/commands/attach.rs
- create: crates/lys-runner/src/attach.rs
- create: crates/lys-runner/tests/attach.rs
- create: crates/lys/tests/attach.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/commands/mod.rs
- modify: crates/lys-runner/src/lib.rs

**Checklist:**
- C709 — lys attach shows a seat's live terminal to its responsible person or an administrator and nobody else (AGENTS-002 R4).

**Stories:**
- S405 (An operator, a person running the estate) — As an operator, I want to attach to a seat's terminal from Lys when I need to look, so that I do not need a terminal workspace program to own the seats.

### R5: The move, one seat at a time

Behavioural. docs/ops/MOVE-A-SEAT.md SHALL state the move of one seat: record its profile as a Lys seat (R1), stop the herdr pane, turn Argus's delivery for that seat off (its transport set to none), start the seat from Lys, and prove it by the first [Lys context watch] warning delivered through 064 and recorded in Lys's delivery log with no Argus delivery for that seat after the move; THE SYSTEM SHALL refuse to start a seat whose name is still online in Argus's registry if Argus is reachable, naming it, so no seat is watched twice; the first seat moved is Waffles's own.

**Acceptance:**
- Move one seat by the document; the proof lines are present; a second start of the same name while Argus still lists it online is refused by name.

**Files:**
- create: docs/ops/MOVE-A-SEAT.md
- modify: crates/lys-identity-server/src/seats_api.rs

**Checklist:**
- C710 — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).

**Stories:**
- S401 (Tom, owner) — As the owner, I want every seat started, seen, stopped and warned from Lys, so that herdr and Argus can be switched off.

### R6: A message reaches a seat as a user turn

Behavioural. THE SYSTEM SHALL send a message to a running seat from the Sessions screen (a send box per seat) and from `lys seat send <name> <text>`, through POST /seats/{name}/send (action seat.send under rights, as R3). It delivers the text to the seat's managed session as a user turn (the runner's Human input, written as a stream-json user message), never as typed keys. Text that is empty or carries control characters is refused by name. A seat that is not running is refused seat_not_running.

**Acceptance:**
- A message sent from the Sessions screen and one sent from `lys seat send` each appear in the seat's Claude Code transcript (CLAUDE_CONFIG_DIR/projects/<slug>/<session-uuid>.jsonl) as a user message with that text, and the seat answers it; proven on Waffles's own seat first.
- A send to a stopped seat is refused seat_not_running; a send with a control character is refused by name.

**Files:**
- create: crates/lys-identity-server/tests/seat_send.rs
- modify: crates/lys-identity-server/src/seats_api.rs
- modify: crates/lys/src/commands/seat.rs
- modify: surface/identity/src/features/sessions/Seats.tsx

**Checklist:**
- C734 — A message sent to a seat from the Sessions screen or `lys seat send` arrives in its Claude Code transcript as a user turn, never as typed keys (AGENTS-002 R6).

**Stories:**
- S418 (Tom, owner) — As Tom, I want to send a message to any seat from the Sessions screen or the lys CLI and see it arrive in that seat as a real turn, so that I can talk to my agents from Lys.

## Boundaries

- No herdr, screen or manifold transport in Lys: the runner's pseudo-terminal and the harness's control channel are the only ways in.
- Argus's alarms, rules, launches and queue are not moved here (ADR-136 section 3: liminal services and aion).
- Nothing is typed into a terminal to warn, compact, remind or stop (ADR-130).
- Agents started outside Lys stay out of scope (ADR-135): the move brings a seat inside.

## Verification

- This handwritten brief passes scripts/design/gate.sh, judged by its parsed failures, never by its exit code.
- Written whole, read whole by the lead; the whole gate at the sha; the one battery and install on Tom's Mac; the lead walks the Sessions screen and attaches to a seat, with pictures; the first moved seat's warning is read in its session.
- Four-heading handback with the hospital sentence; no test proves less; cargo nextest only; clippy pedantic -D warnings; file length 500.
