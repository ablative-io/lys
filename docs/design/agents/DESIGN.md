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

The seat file (AGENTS-017 R1 and R5) is one JSON file, mode 0600, at the path the agent's reviewed profile names as its seat file, written once by `lys agent enrol` on the computer the agent runs on and never replaced. One reader and writer in lys-core (`seat_file`) serve the `lys mcp` bridge and the enrol command; Cambium's door reads the same members by the same spellings. A reader refuses any version but 1, naming the version it found.

| Member | Written by | Read by | What it holds |
|---|---|---|---|
| `version` | enrol; Cambium's door | the bridge; the door; the seat tool | always 1 |
| `doorUrl` | enrol with `--door`; the door | the door; the seat tool | the Cambium door the seat answers at |
| `handle` | enrol with `--handle`; the door | the door; the seat tool | the registry handle |
| `device` | enrol with `--device`; the door | the door; the seat tool | the device label on the seat's session |
| `privateKeyPem` | enrol; the door (a seat founded beside it) | the bridge; the seat tool | the Ed25519 key, PKCS#8 PEM, made where the agent runs, never printed |
| `lysUrl` | enrol; the door | the door; the seat tool | the Lys API that certified the key |
| `lysAgentId` | enrol; the door | the bridge (`--as`); the door; the seat tool | the Lys agent the certificate names |
| `certificatePem` | enrol | a reader that verifies the key | the certificate Lys issued over the key |
| `serial` | enrol | a person reading the file | the certificate's serial in Lys's certificate log |

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
| `crates/lys-home/benches/` | actual-code seat-loop count workloads, per-case ratchets and native-time correlation | AGENTS-005 |
| `crates/lys-runner/benches/` | actual-code seat-loop count workloads, per-case ratchets and native-time correlation | AGENTS-005 |
| `crates/lys-identity-server/benches/` | actual-code seat-loop count workloads, per-case ratchets and native-time correlation | AGENTS-005 |
| `crates/lys-home/src/` | capture/journal/record counted seams and indexed current persistence | AGENTS-005 |
| `crates/lys-home/tests/` | retained-byte, old-install, count and fault regressions | AGENTS-005 |
| `crates/lys-home/Cargo.toml` | owned non-default benchmark target/instrumentation declaration; no unapproved dependency | AGENTS-005 |
| `crates/lys-runner/Cargo.toml` | owned non-default benchmark target/instrumentation declaration; no unapproved dependency | AGENTS-005 |
| `crates/lys-identity-server/Cargo.toml` | owned non-default benchmark target/instrumentation declaration; no unapproved dependency | AGENTS-005 |
| `crates/lys-identity-server/tests/seat_send.rs` | a message to a seat lands as a user turn; refusals by name | AGENTS-002 |
| `crates/lys-identity-server/src/network_store.rs` | the network store; one kind-lys runner per install | AGENTS-002 |
| `crates/lys-identity-server/src/agent_app_pass.rs` | an agent's pass to an app: the route, its two proofs and the issue log line | AGENTS-006 |
| `crates/lys-identity-server/src/error_app_pass.rs` | the agent pass refusals by name | AGENTS-006 |
| `crates/lys-identity-server/src/provider/agent_issue.rs` | the provider issues an agent the rights_claim pass for an approved audience | AGENTS-006 |
| `crates/lys-identity-server/tests/agent_app_pass.rs` | an agent's pass over HTTP: its own grants, refusals by name, a revoked credential | AGENTS-006 |
| `crates/lys/src/commands/agent_pass.rs` | lys agent pass: credential from an owner-only file, pass to an owner-only file | AGENTS-006 |
| `crates/lys/tests/agent_pass.rs` | lys agent pass against a loopback fixture | AGENTS-006 |
| `crates/lys-identity-server/src/lib.rs` | the crate's modules | AGENTS-006 |
| `crates/lys-identity-server/src/provider.rs` | the provider's modules | AGENTS-006 |
| `crates/lys-identity-server/src/routes_table.rs` | the route table | AGENTS-006 |
| `crates/lys-identity-server/src/route_actions.rs` | a run pass reaches the agent pass route untouched | AGENTS-006 |
| `crates/lys-identity-server/src/openapi_table.rs` | the route's OpenAPI row | AGENTS-006 |
| `crates/lys-identity-server/src/openapi_agents_types.rs` | the route's typed request and answer | AGENTS-006 |
| `crates/lys-identity-server/src/error_agents.rs` | the agents cluster's refusal families | AGENTS-006 |
| `crates/lys-identity-server/src/error_names.rs` | refusal names | AGENTS-006 |
| `crates/lys/src/cli.rs` | the lys agent command | AGENTS-006 |
| `crates/lys/src/main.rs` | the lys agent dispatch | AGENTS-006 |
| `crates/lys/src/commands/mod.rs` | the CLI's command modules | AGENTS-006 |
| `crates/lys/src/commands/seat_client.rs` | the loopback client, reachable without the operator token | AGENTS-006 |
| `surface/identity/src/features/runtime/` | the operations screen: the network tree (default), the canvas and the sessions table, and the terminal pane they share | AGENTS-007 |
| `surface/identity/src/routes.tsx` | the operations route lands on the kept view | AGENTS-007 |
| `surface/identity/src/shell/` | the rail's and the palette's operations entries | AGENTS-007 |
| `surface/identity/src/api.ts` | GET /network and GET /seats read for the tree | AGENTS-007 |
| `surface/identity/src/features/runtime/CanvasSurface.tsx` | the existing canvas separated from the operations wrapper to keep source files below 500 lines | AGENTS-007 |
| `surface/identity/vite.config.ts` | discovers the network tree component acceptance tests | AGENTS-007 |
| `surface/identity/tests/bare-buttons.standing.ts` | keeps the existing canvas control count at its child module | AGENTS-007 |
| `surface/identity/src/features/network/` | the Network screen: a person adds their own computer, gets its code and the lys client | AGENTS-008 |
| `surface/identity/src/features/network-self-service.test.tsx` | the self-service add, connect and download rows under test | AGENTS-008 |
| `crates/lys/src/identity/install/layout.rs` | the install's layout gains the clients folder | AGENTS-008 |
| `crates/lys/src/cli/` | the lys CLI's seat and sign-in verbs | AGENTS-008 |
| `crates/lys-identity-server/src/routes/` | the identities routes: a person registers and certifies their own agent | AGENTS-008 |
| `crates/lys-identity-server/src/routes.rs` | the route table's administrator gate | AGENTS-008 |
| `crates/lys/src/identity/cli.rs` | the identity install and upgrade flags | AGENTS-008 |
| `crates/lys-runner/src/protocol/` | the runner protocol's acts | AGENTS-008 |
| `crates/lys-runner/src/socket/` | the runner socket's dispatch | AGENTS-008 |
| `crates/lys-runner/src/harness_control/` | harness admission and qualification | AGENTS-008 |
| `surface/identity/src/features/provisioning/` | the harness description review form and its profile copies, where a seat program's executable, arguments, seat file and digests are reviewed | AGENTS-009 |
| `crates/lys-runner/src/dial/bridge/tests.rs` | the bridge's two-lane arrival test and its fixture; a reply write refused after the barrier by a lane the bridge closed is the expected outcome | AGENTS-011 |
| `crates/lys/src/commands/launch.rs` | the personal launch: server-rendered today; keeps the launch on the device after each render and starts from the kept one when the server does not answer | AGENTS-012 |
| `crates/lys/src/commands/launch_context.rs` | sign-in state and server reach for a launch; the bare-machine first run asks for the address once | AGENTS-012 |
| `crates/lys/src/commands/launch/kept.rs` | the launch kept on the device per agent after every server render: template hash, harness, proxy identity, pass and expiry; atomic write | AGENTS-012 |
| `crates/lys/src/commands/launch/kept_tests.rs` | tests of the sibling module, red first | AGENTS-012 |
| `crates/lys/src/cli/proxy.rs` | the proxy subcommand, started on the device for a run begun while the server was unreachable | AGENTS-012 |
| `crates/lys/src/commands/launch/offline.rs` | the start from a kept launch when the server's connect fails, with the device's proxy and the first line that says so | AGENTS-012 |
| `crates/lys/src/commands/launch/offline_tests.rs` | tests of the sibling module, red first | AGENTS-012 |
| `crates/lys-identity-server/src/routes_startup.rs` | route table; mounts the unreachable-run hand-over route | AGENTS-012 |
| `crates/lys/src/commands/launch/handover.rs` | hand-over of runs started while Lys was unreachable, one request per run, removed on acceptance by id | AGENTS-012 |
| `crates/lys/src/commands/launch/handover_tests.rs` | tests of the sibling module, red first | AGENTS-012 |
| `crates/lys-identity-server/src/sessions_unreachable.rs` | the route that accepts a run started while the server was unreachable, by launch hash and pass validity at start, idempotent by run id | AGENTS-012 |
| `crates/lys-identity-server/src/sessions_unreachable_tests.rs` | tests of the sibling module, red first | AGENTS-012 |
| `surface/identity/src/features/file/sections.tsx` | the agent file's sections; a handed-over session shown as started while Lys was not answering | AGENTS-012 |
| `surface/identity/src/features/sessions/Sessions.tsx` | the sessions list; the handed-over marking | AGENTS-012 |
| `surface/identity/src/features/runtime/Sessions.tsx` | Running agents' sessions view; the handed-over marking | AGENTS-012 |
| `crates/lys/src/cli/sign_in.rs` | personal sign-in; asks for the Lys address on a bare machine instead of config_unreadable | AGENTS-012 |
| `crates/lys-identity-server/src/agents_add_run.rs` | POST /agents with run: register, activate, settings, computer, allow and start under one operation, one receipt | AGENTS-013 |
| `crates/lys-identity-server/src/agents_add_run_tests.rs` | tests of the sibling module, red first | AGENTS-013 |
| `surface/identity/src/features/people/AddAgent.tsx` | adding an agent; one request, the receipt part by part | AGENTS-013 |
| `surface/identity/src/features/people/add-and-run.ts` | the browser-kept add-and-run walk; deleted by AGENTS-013 (one request replaces it) | AGENTS-013 |
| `crates/lys-runner/tests/ends_with_fd.rs` | red first: a runner and its owner exit when the lifeline's write end closes (AGENTS-014 R1, R2) | AGENTS-014 |
| `crates/lys/src/commands/runner.rs` | carries the descriptor into the runner's Options and the owner's OwnerServe (AGENTS-014 R1) | AGENTS-014 |
| `crates/lys-runner/src/socket.rs` | Options gains ends_with; the runner reads the lifeline and ends on its end of file (AGENTS-014 R1) | AGENTS-014 |
| `crates/lys-runner/src/seat_owner/spawn.rs` | OwnerSpawn gains ends_with; start() passes the inheritable descriptor to the owner (AGENTS-014 R1) | AGENTS-014 |
| `crates/lys-runner/src/seat_owner/process.rs` | the owner reads the lifeline and ends its managed session on end of file (AGENTS-014 R1) | AGENTS-014 |
| `crates/lys-runner/src/seat_owner/exit_wait.rs` | the kernel's process-exit notice, read by the new test (AGENTS-014 R1) | AGENTS-014 |
| `crates/lys-runner/tests/seat_survival.rs` | starts its runner on a lifeline the test holds (AGENTS-014 R2) | AGENTS-014 |
| `crates/lys-runner/tests/seat_survival_faults.rs` | starts its runner on a lifeline the test holds (AGENTS-014 R2) | AGENTS-014 |
| `crates/lys-runner/tests/seat_cost.rs` | starts its runner on a lifeline the test holds (AGENTS-014 R2) | AGENTS-014 |
| `crates/lys-runner/tests/seat_conversation.rs` | starts its runner on a lifeline the test holds (AGENTS-014 R2) | AGENTS-014 |
| `crates/lys-runner/tests/seat_owner_socket.rs` | starts its runner on a lifeline the test holds (AGENTS-014 R2) | AGENTS-014 |
| `crates/lys-runner/tests/seat_restart.rs` | starts its runner on a lifeline the test holds (AGENTS-014 R2) | AGENTS-014 |
| `crates/lys-identity-server/tests/agent_starts_agent.rs` | an agent registers, sets up, has reviewed and starts another agent under grants (AGENTS-015 R1, R2) | AGENTS-015 |
| `crates/lys-identity-server/src/signed_first.rs` | admits an agent's signed call under its own grants (AGENTS-015 R1) | AGENTS-015 |
| `crates/lys-identity-server/src/routes/identities.rs` | an agent registers an agent for its person, and holds the five set-up rights under the person's chain (AGENTS-015 R1, R2) | AGENTS-015 |
| `crates/lys-identity-server/src/provisioning_api.rs` | admits an agent's signed call under its own grants (AGENTS-015 R1) | AGENTS-015 |
| `crates/lys-identity-server/src/start.rs` | admits an agent's signed call under its own grants (AGENTS-015 R1) | AGENTS-015 |
| `crates/lys-identity-server/src/seats_api.rs` | admits an agent's signed call under its own grants (AGENTS-015 R1) | AGENTS-015 |
| `crates/lys-identity-server/src/runner_api.rs` | admits an agent's signed call under its own grants (AGENTS-015 R1) | AGENTS-015 |
| `crates/lys-identity-server/src/grants.rs` | admits an agent's signed call under its own grants (AGENTS-015 R1) | AGENTS-015 |
| `crates/lys/src/commands/mcp_bridge.rs` | reaches Lys over verified https from any device (AGENTS-015 R3) | AGENTS-015 |
| `crates/lys/src/commands/mcp_bridge_tests.rs` | the https and loopback URL rules (AGENTS-015 R3) | AGENTS-015 |
| `crates/lys/src/identity/loopback_http.rs` | carries the bridge's https exchange beside loopback (AGENTS-015 R3) | AGENTS-015 |
| `crates/lys-identity/src/grants/shipped.rs` | WITHHELD_FROM_AGENTS loses the five acts a person's standing grant covers (AGENTS-015 R1) | AGENTS-015 |
| `crates/lys-identity/src/start/authority.rs` | a live agent.start grant on the agent admits a start (AGENTS-015 R1) | AGENTS-015 |
| `crates/lys-identity-server/src/start_directory.rs` | reads the grant-admitted start (AGENTS-015 R1) | AGENTS-015 |
| `crates/lys-identity-server/src/network_api/writes.rs` | machine agents admit an agent holding machine.agent.set from the machine's owner (AGENTS-015 R1) | AGENTS-015 |
| `crates/lys-identity-server/tests/agent_roots.rs` | the withheld list names exactly what stays withheld (AGENTS-015 R1) | AGENTS-015 |
| `surface/identity/src/features/file/give-standing.test.tsx` | the standing grants render, post exactly their grant and are absent where there is nothing to give (AGENTS-015 R4) | AGENTS-015 |
| `surface/identity/src/features/file/GiveAccess.tsx` | the three standing grants, one choice each (AGENTS-015 R4) | AGENTS-015 |
| `surface/identity/src/features/network/labels.ts` | the Computers and Connect labels, once, read by the screens and by launch_device's test (AGENTS-016 R1) | AGENTS-016 |
| `crates/lys/src/commands/launch_device.rs` | a kind lys runner matches the device holding the install's runner key; advice names only rendered labels (AGENTS-016 R1) | AGENTS-016 |
| `crates/lys/src/commands/launch_device_tests.rs` | the host match and the advice labels (AGENTS-016 R1) | AGENTS-016 |
| `crates/lys-identity-server/src/openapi_runner_types.rs` | the key on a kind lys runner view (AGENTS-016 R1) | AGENTS-016 |
| `crates/lys-identity-server/tests/runner_join.rs` | the runner view's key for the install's own runner (AGENTS-016 R1) | AGENTS-016 |
| `surface/identity/src/shell/railItems.ts` | imports the Computers label (AGENTS-016 R1) | AGENTS-016 |
| `surface/identity/src/features/network/JoinCode.tsx` | imports the Connect label (AGENTS-016 R1) | AGENTS-016 |
| `surface/identity/src/features/file/AgentConnect.tsx` | the Connect section: the claude and codex lines for this install and agent (AGENTS-016 R2) | AGENTS-016 |
| `surface/identity/src/features/file/agent-connect.test.tsx` | the lines, the copy act and the sentence on what works today (AGENTS-016 R2) | AGENTS-016 |
| `surface/identity/src/features/file/tabs.ts` | Connect under Credentials (AGENTS-016 R2) | AGENTS-016 |
| `crates/lys-identity-server/src/routes/identities_agent.rs` | the directory append first, then the five grants, idempotent on retry (AGENTS-016 R3) | AGENTS-016 |
| `crates/lys-identity-server/src/agent_acts.rs` | actor and responsible on both appends (AGENTS-016 R3) | AGENTS-016 |
| `crates/lys-identity/src/grants/created.rs` | the five grants appended only where not already standing (AGENTS-016 R3) | AGENTS-016 |
| `crates/lys-identity-server/src/certificates_issue.rs` | admits the registrar's first certificate, refuses certificate_not_first and agent_not_yours (AGENTS-016 R4) | AGENTS-016 |
| `crates/lys-identity-server/src/certificates_api.rs` | the agent branch of the issue route (AGENTS-016 R4) | AGENTS-016 |
| `crates/lys-identity-server/tests/dashboard_acted_by_agents.rs` | the registered and started lines on GET /dashboard (AGENTS-016 R5) | AGENTS-016 |
| `surface/identity/src/features/dashboard/dashboard-acted.test.tsx` | the lines, the heading and Stop (AGENTS-016 R5) | AGENTS-016 |
| `surface/identity/src/features/dashboard/ActedByAgents.tsx` | the Acted for you by your agents list with Stop beside each (AGENTS-016 R5) | AGENTS-016 |
| `crates/lys-identity-server/src/dashboard_api.rs` | acted_by_agents from the projection and the launch records (AGENTS-016 R5) | AGENTS-016 |
| `surface/identity/src/features/dashboard/Dashboard.tsx` | draws the acted-by-agents list (AGENTS-016 R5) | AGENTS-016 |
| `surface/identity/src/features/dashboard/contract.ts` | the acted_by_agents shape (AGENTS-016 R5) | AGENTS-016 |
| `crates/lys/src/commands/enrol.rs` | lys agent enrol: key, request, certificate and seat file in one act where the agent runs | AGENTS-017 |
| `crates/lys/src/commands/enrol_tests.rs` | enrol's reds: 0600 seat file the bridge reads; seat_file_exists; the Cambium shape | AGENTS-017 |
| `crates/lys-core/src/seat_file.rs` | the one seat file shape read by the bridge, the enrol command and the runner's seat program | AGENTS-017 |
| `surface/identity/src/features/file/EmergencyStop.tsx` | the emergency act, renamed Withdraw, final, with its receipt sentence | AGENTS-017 |
| `crates/lys-core/src/lib.rs` | exports the seat file reader and writer | AGENTS-017 |
| `crates/lys-identity-server/src/own_computer.rs` | names the computer Lys runs on for the administrator, once, at setup and at agent-roots | AGENTS-017 |
| `surface/identity/src/features/run/SessionStop.tsx` | Stop for one run: ends the session, keeps the certificate | AGENTS-017 |
| `crates/lys/src/cli/enrol.rs` | lys agent enrol's arguments | AGENTS-017 |
| `crates/lys-core/src/seat_file_tests.rs` | the seat file's reds: written back whole in the door's names, other versions and none refused, the key never shown | AGENTS-017 |
| `crates/lys-identity-server/tests/agent_roots/unnamed_at_setup.rs` | a computer setup could not name is named by agent-roots, and a start on it is admitted | AGENTS-017 |
| `crates/lys/Cargo.toml` | the lys crate's test dependencies on the identity contract harness and the server, for enrol's reds | AGENTS-017 |
| `surface/identity/src/features/file/IdentityFile.tsx` | the agent file's head menu names the emergency act Withdraw | AGENTS-017 |
| `crates/lys-core/Cargo.toml` | serde_json for the seat file reader and writer | AGENTS-017 |
| `docs/design/identity/GRANT-CONTRACT.md` | the grant contract: which acts an agent may be given, and which are kept from every agent | AGENTS-019 |
| `crates/lys-identity-server/src/pair.rs` | Pairing routes: an unknown key asks for a code, the held wait answers the decision. | AGENTS-020 |
| `crates/lys-identity-server/src/pair_store.rs` | Undecided pairings kept by code until a person decides. | AGENTS-020 |
| `crates/lys-identity-server/src/pair_tests.rs` | Pairing routes proved end to end with the server's own notification. | AGENTS-020 |
| `surface/identity/src/features/pair/PairPage.tsx` | The Pair page: four words, the agent's name, its roots, Approve and Decline. | AGENTS-020 |
| `surface/identity/src/features/pair/pair-page.test.tsx` | The Pair page proved. | AGENTS-020 |
| `crates/lys/src/commands/pair.rs` | `lys agent pair`: the agent's side of pairing for an agent that is not a product seat. | AGENTS-020 |
| `crates/lys/src/commands/pair_tests.rs` | `lys agent pair` proved against a test server. | AGENTS-020 |
| `crates/lys/src/cli/pair.rs` | The pair command's arguments. | AGENTS-020 |

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
