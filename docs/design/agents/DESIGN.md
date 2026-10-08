---
type: design
cluster: agents
title: Lys runs and watches its agents: what Argus and herdr do today, done in Lys
---

# Lys runs and watches its agents: what Argus and herdr do today, done in Lys

> **Cluster:** agents

## Intention

Every seat is started by Lys through its own runner and proxy, watched by Lys's own hooks and status line, warned, compacted, reminded and scheduled by Lys through the harness's control channel, with its words, variables, countdowns and schedules held in Lys beside its goals, so Argus and herdr can be switched off one seat at a time and nothing a seat needs is typed into a terminal.

## Problem

On 6 October 2026 every seat is started through herdr (a third-party terminal workspace binary with a socket API) and watched by Argus: Claude Code hooks and the status line forward every event to Argus (tools/argus/plugin/hooks/hooks.json, scripts/argus-statusline.sh); the context warning, the preparation note and /compact are typed into the pane as user turns (scripts/context-text-inject.sh via `herdr agent prompt`), which herdr cannot receipt (docs/herdr-compaction-permission-20260925.md); Argus holds the five prompt messages in four layers with templates, the agent variables ({{vars.key | fallback}}), the goals placeholder and the scheduled messages (docs/agent-prompts.md, docs/scheduled-messages-20260907.md). Lys already starts agents (DIRECTORY-050), tracks them through its own hooks, status line and proxy usage file, and holds budgets and goals with reminders (DIRECTORY-051); DIRECTORY-064 moves delivery from the terminal to the harness's control channel and has no commits. What has no owner in Lys: the prompt words and their layers, the variables with their substitutions and countdowns, the schedules, the seat registry with liveness, attaching to a seat's terminal, and the move itself.

## Solution

Two briefs over the landed 050 and 051 and the written 064. AGENTS-001: prompt words in four layers with templates, variables per agent and session with revision and expiry, countdown and goal substitutions rendered at delivery with the contributing revisions captured, and schedules with interval and occurrence bounds, all delivered through 064's dispatcher and shown on the Usage screen; the agent reads and sets its own variables through lys and the Lys MCP server. AGENTS-002: a seat is a Lys record launched by the runner through the proxy with the profile HOME-037 carries, a registry with liveness (online, not seen, seen but unregistered), start, stop and restart as deliberate acts under Lys rights, an attach command that shows a seat's terminal, and the move of a seat off herdr and Argus one at a time, proved by the first warning arriving from Lys. ADR-137 settles where the data lives: in Lys beside goals, rendered at delivery; no shared library (Lodestone is a read-only query engine; haematite is already the store under Lys); alarms, rules and launches go to liminal services and aion per ADR-136 section 3.

## Principles

- **P1** — Nothing a seat needs is typed into a terminal: warnings, compaction, reminders and stops go through the harness's control channel and the runner (ADR-130).
- **P2** — The data lives in Lys beside goals, rendered at the moment of delivery, with every contributing revision in the receipt; no shared library (ADR-137).
- **P3** — A seat's state comes from the runner's own knowledge and the proxy, never from a terminal program's listing.
- **P4** — Seat acts are deliberate acts under Lys rights and are receipted (ADR-136).

## Decisions

- ADR-118 — Lys owns running and monitoring its agents and enforcing values; Argus reads data for analytics — Lys owns everything about running and monitoring an agent it starts: installing its hooks/status line, reading its session stream and recording tokens, context and time. Lys holds budgets, goals, expectations and deliverables and enforces them through its runner with notices, compaction requests or stops as the value says. The context-watch notices previously driven by Argus become Lys-driven acts. Lys exposes the resulting data for Argus, which keeps analytics and visualisation; Lys does not depend on it. Lys screens are plain controls/state plus the authoritative tool-policy and grant refused-acts list on each agent page. Context compaction, goals and reminders are Lys-owned background acts. Argus may read explicitly permitted Lys variables alongside its data stream; no secret or arbitrary environment export follows from this. DIRECTORY-051 creates the missing tool-boundary policy and its browser editor. It does not claim an OS sandbox. The runner proves socket peer identity in peer.rs and makes grantable decisions against the live grant authority, denying uncertainty. Codex pre-tool refusal coverage is explicitly unavailable. OS process containment is the separate card11VnzLRp.
- ADR-130 — Managed harness channels carry automatic context and reminder acts — 064 adds one runner-owned typed control/event channel per managed Claude or Codex session. It reuses 051 operation and usage owners, serialises all inputs at proved boundaries, requires actual harness evidence and retains uncertain outcomes without automatic resend. Legacy PTY sessions remain manual and cannot claim this capability. 065 extends the same Codex event reader for refusal coverage.
- ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.
- ADR-136 — Permissions across the stack: Lys is the one authority; products decide hot routes from a signed pass and ask Lys for deliberate ones; a grant carries its mode — Lys holds every identity and grant; a product permission model is data in its app schema (ADR-116). Every product route is classified once as hot (decided offline from the pass) or deliberate (asked of Lys live, never falling back). The pass is the access token with a rights claim for its audience, short-lived, signed with the key Lys publishes. A grant carries one mode: outright, by_draft or by_two; a held act is a Lys draft the product executes after approval by reading Lys. Reach is by placement, never a pattern; a placement may be restricted. Roles are named bundles in the product schema; a widening by an app waits for the administrator. A machine is the fifth identity. One client library, lys-pass, with conformance fixtures every product gate runs. Availability is stated on each product screen; every deliberate act is receipted in Lys.
- ADR-137 — Argus's remaining functions move into Lys as data beside goals, and a seat is a Lys record launched by the runner through the proxy; no shared library — Words (five slots, four layers, templates), variables (per agent and session, revision, author, expiry), schedules and seats are identity-server records in Lys's log, rendered and acted on at delivery through DIRECTORY-064's dispatcher with the contributing revisions in the receipt. No shared library: Lodestone is a read-only query engine over a store and holds nothing; haematite is the store under Lys already; a library would be a second place for the same data. Seat start, stop and restart are deliberate acts under Lys rights (ADR-136). Argus's alarms, rules, launches and queue are not moved into Lys: they become liminal services and aion work (ADR-136 section 3). A seat moves off herdr and Argus one at a time, Waffles's first.

## Goals

- Words in four layers with templates and a sending-nothing preview.
- Variables per agent and session with revision, author and expiry, set by the agent itself through lys and the Lys MCP server.
- Rendering at delivery of variables, goals, time left and context numbers, captured with its revisions.
- Schedules with interval and occurrence bounds that coalesce and stop on uncertainty.
- A seat as a Lys record launched through the proxy, seen with liveness, started and stopped by right, attached from lys, and moved off herdr and Argus one at a time.

## Non-Goals

- Alarms, rules, launches and the queue. — ADR-136 section 3 puts them in liminal services and aion.
- A dashboard with graphs. — DIRECTORY-051 R5 keeps per-turn analytics out of the Usage screen.
- herdr, screen or manifold as a transport. — The runner's pseudo-terminal and the control channel are the only ways in.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/agents/` | this cluster | AGENTS-001 |
| `docs/design/ARGUS-TO-LYS-2026-10-06.md` | the map from Argus and herdr to Lys | AGENTS-001 |
| `docs/design/decisions.json` | ADR-137 | AGENTS-001 |
| `docs/design/roadmap.json` | RM-108 | AGENTS-001 |
| `crates/lys-identity-server/src/` | words, variables, schedules and seats: API, state, store | AGENTS-001 |
| `crates/lys-identity-server/tests/` | their tests | AGENTS-001 |
| `crates/lys-runner/src/` | rendering at delivery; the runner's liveness and attach | AGENTS-001 |
| `crates/lys-runner/tests/` | the runner's tests | AGENTS-001 |
| `crates/lys/src/cli.rs` | the lys command line | AGENTS-001 |
| `crates/lys/src/commands/` | lys variables, lys attach | AGENTS-001 |
| `crates/lys/tests/` | command tests | AGENTS-001 |
| `surface/identity/src/features/usage/` | the Usage screen's words, variables and schedules | AGENTS-001 |
| `surface/identity/src/features/sessions/` | the Sessions screen's seat registry | AGENTS-002 |
| `surface/identity/src/api.ts` | the surface's API client | AGENTS-001 |
| `surface/identity/tests/` | surface tests | AGENTS-001 |
| `docs/ops/` | the move of a seat from herdr and Argus to Lys | AGENTS-002 |
| `crates/lys/src/cli/runner.rs` | typed independent seat-owner command | AGENTS-004 |
| `crates/lys/src/identity/install.rs` | install an independent Lys seat owner without ending supervised harnesses | AGENTS-004 |
| `crates/lys/src/identity/install/services.rs` | process groups and event-driven custody/readiness | AGENTS-004 |
| `crates/lys/src/identity/install/proxy.rs` | keep accepted proxy streams alive across install swaps | AGENTS-004 |
| `crates/lys/src/identity/upgrade.rs` | stage and select a compatible live handover | AGENTS-004 |
| `crates/lys/src/identity/upgrade/` | replace live-session refusal and proxy stop for supervised seats | AGENTS-004 |
| `crates/lys-home/src/proxy/` | single-writer listener/stream/journal custody | AGENTS-004 |

## Inventory

- `tools/argus:plugin/hooks/hooks.json` — every Claude Code hook event forwarded to Argus. Read, never changed.
- `tools/argus:plugin/scripts/argus-statusline.sh` — the status line that forwards the status payload. Read, never changed.
- `tools/argus:plugin/scripts/context-text-inject.sh` — how a warning or /compact is typed into a herdr pane today. Read, never changed.
- `tools/argus:docs/agent-prompts.md` — the five messages, four layers, templates and variables. Read, never changed.
- `tools/argus:docs/scheduled-messages-20260907.md` — the schedule contract. Read, never changed.
- `docs/design/directory/briefs/DIRECTORY-064.json` — the control-channel delivery this cluster sends through. Read, never changed.
- `crates/lys-runner/src/tracking_proxy.rs` — the proxy's usage file the follower reads. Read.

## Constraints

- **CN1** — Every delivery is a DIRECTORY-064 operation with its receipt; nothing here writes to a terminal.
- **CN2** — Every brief here is executed under the standing rules: written whole before one battery and one install; commits by pathspec on main; the design gate judged by parsed failures; every handback carries the hospital sentence.
- **CN3** — Nothing written is unreadable or rewritten: new record kinds take versions; old bytes stay valid (ADR-126).
