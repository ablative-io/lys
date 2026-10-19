---
type: brief
id: AGENTS-016
cluster: agents
title: The computer Lys runs on is a device its agents start on, and an agent's file shows the lines that connect Claude Code and Codex to it
---

# AGENTS-016: The computer Lys runs on is a device its agents start on, and an agent's file shows the lines that connect Claude Code and Codex to it

> **Cluster:** agents
> **Depends on:** AGENTS-015
> **Design anchor:**
> - ADR-125 — An agent session's holder key is held by the runner, and a session proves itself by peer credentials and ancestry — The runner makes one holder key per session in memory. A start becomes key, then issue, then spawn. The harness and lys mcp ask the runner for single presentations over its socket. The session is proved by peer credentials plus the process ancestry reaching the session's own root pid, checked on every connection, on macOS and Linux both, through a named maintained dependency and with no unsafe in Lys code.
> - ADR-137 — Argus's remaining functions move into Lys as data beside goals, and a seat is a Lys record launched by the runner through the proxy; no shared library — Words (five slots, four layers, templates), variables (per agent and session, revision, author, expiry), schedules and seats are identity-server records in Lys's log, rendered and acted on at delivery through DIRECTORY-064's dispatcher with the contributing revisions in the receipt. No shared library: Lodestone is a read-only query engine over a store and holds nothing; haematite is the store under Lys already; a library would be a second place for the same data. Seat start, stop and restart are deliberate acts under Lys rights (ADR-136). Argus's alarms, rules, launches and queue are not moved into Lys: they become liminal services and aion work (ADR-136 section 3). A seat moves off herdr and Argus one at a time, Waffles's first.
> **Checklist:**
> - C783 — On the computer that runs Lys, a launch matches the machine whose runner is the install's own through the runner key the view carries, and every advice sentence in launch_device names only labels the Computers page renders.
> - C784 — An agent's file shows the complete claude mcp add and codex mcp add lines for this install and agent with a copy act each, and says which origin works now and which waits on AGENTS-015 R3.
> **Stories:**
> - S448 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person on the computer that runs Lys, I start my agent here like on any other computer, and I copy the line that connects Claude Code or Codex to it from the agent's own page instead of assembling it from help text.

## Purpose

On 19 October 2026 Tom could not start an agent on the Mac that runs Lys: launch_device.rs:163 matches a device only to a dialled runner, so the host, whose machine names the install's own runner, is told it has not joined and to use a Connect button the Computers page does not show. Connecting Claude Code or Codex to an agent through lys mcp means typing the claude mcp add or codex mcp add line by hand from the bridge's --help, with the install's address, the agent's id and the key file's path, and nothing in Lys shows that line. This brief makes the host a device like any other, replaces advice that names labels the screens do not show, and puts the two exact connection lines on the agent's file, copyable, saying plainly which form works today and which waits on AGENTS-015 R3.

## Task

Make launch_device match the host's machine through the install's own runner key carried on the runner view, and make every advice sentence name only labels the surface renders; add a Connect section to the agent's file with the exact Claude Code and Codex lines for this install and agent, copyable.

## Requirements

### R1: The computer Lys runs on is a device its agents start on

Behavioural. crates/lys/src/commands/launch_device.rs:163 matches this device to a machine only when the runner view names a runner of kind dialled whose key is this computer's runner-machine key, so on the computer that runs Lys itself, whose machine names the install's own runner (RunnerRecord::Lys, network_store.rs:473), every launch ends machine_not_joined with "use Network > Computer > Connect", naming a button the Computers page does not have. From this brief the runner view GET /network/machines/{id}/runner (runner_api.rs:131, openapi_runner_types.rs) carries, for a runner of kind lys, the install's runner-server public key as hex (the service key the server loads at routes_startup.rs:95, the same key install.rs:261-262 writes to run/runner-server.pub), and launch_device accepts a record of kind lys when that key equals the hex in this device's run/runner-server.pub under its install root (Layout::run_dir): the device that holds the install is the install's machine. A record of kind lys whose key is not this device's is not a match, exactly as a dialled record with another key is not. Every advice sentence in launch_device.rs names only what the screens show: the Computers page (railItems.ts:22) and its Connect group for a computer (JoinCode.tsx:78 and :182, aria-label 'Connect <name>'); "Network > Computer > Connect" and any other words naming a label the surface does not render are replaced. The two labels live once, in a new surface/identity/src/features/network/labels.ts (COMPUTERS, and connectLabel(name) for 'Connect <name>'), which railItems.ts:22 and JoinCode.tsx:78 and :182 import instead of their literal strings; launch_device_tests.rs reads that file with include_str! by its path from the crate and asserts each advice sentence's label is a string exported there, so a renamed label fails the Rust test.

**Acceptance:**
- Red first, in crates/lys/src/commands/launch_device_tests.rs: a runner view of kind lys with this device's run/runner-server.pub key matches the machine and launch proceeds; a kind lys view with another key, or no run/runner-server.pub on the device, is machine_not_joined; a dialled view matches exactly as today
- In the same file: every refusal's advice names only labels exported by surface/identity/src/features/network/labels.ts, read with include_str!; railItems.ts and JoinCode.tsx import their labels from it, and surface/identity typecheck, biome and vitest stay green
- In crates/lys-identity-server/tests/runner_join.rs: the runner view for a machine whose runner is RunnerRecord::Lys carries the server's runner key hex; for a dialled runner it carries the dialled key as today
- gate -stages nextest -crates lys,lys-identity-server -test 'package(lys) | package(lys-identity-server)' green, counts in the report

**Files:**
- create: surface/identity/src/features/network/labels.ts
- modify: crates/lys/src/commands/launch_device.rs
- modify: crates/lys/src/commands/launch_device_tests.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: crates/lys-identity-server/src/openapi_runner_types.rs
- modify: crates/lys-identity-server/tests/runner_join.rs
- modify: surface/identity/src/shell/railItems.ts
- modify: surface/identity/src/features/network/JoinCode.tsx

**Checklist:**
- C783 — On the computer that runs Lys, a launch matches the machine whose runner is the install's own through the runner key the view carries, and every advice sentence in launch_device names only labels the Computers page renders.

**Stories:**
- S448 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person on the computer that runs Lys, I start my agent here like on any other computer, and I copy the line that connects Claude Code or Codex to it from the agent's own page instead of assembling it from help text.

### R2: An agent's file shows the exact lines that connect Claude Code and Codex to it through lys mcp, copyable

Behavioural. An agent's file (surface/identity/src/features/file, tabs.ts, sections.tsx) gains a Connect section under Credentials, surface/identity/src/features/file/AgentConnect.tsx, that shows two complete lines built from this install and this agent, each with a copy act (the shell's RecordCopy or JoinCode.tsx:73's copy Act, the same words): for Claude Code, claude mcp add --transport stdio lys -- lys mcp --url <origin>/api/mcp --agent <id> --key-file <path to this agent's key file on that computer>; for Codex, codex mcp add lys -- lys mcp --url <origin>/api/mcp --agent <id> --key-file <the same>. <origin> is the page's own origin (window.location.origin, as AddMachine.tsx and Network.tsx read it) and <id> the agent's id; the key file placeholder is shown as written, with one sentence saying it is the PEM or seat file holding the key certified for this agent on the Certificates tab. Under the lines, one sentence says what works today: with an http loopback origin the line works now; with any other origin the line needs the https bridge of AGENTS-015 R3, named, and nothing else on the section changes between the two. Nothing is issued, posted or stored by this section: it renders strings from the page's origin and the agent's record. The section is absent on a person's file.

**Acceptance:**
- Red first, in a new surface/identity/src/features/file/agent-connect.test.tsx: for an agent, both lines render with the page's origin, /api/mcp and the agent's id, and the copy act puts the exact line on the clipboard (the test's clipboard stub reads it back); the sentence under them names AGENTS-015 R3 for an https origin and says the line works now for http://127.0.0.1
- In the same file: a person's file renders no Connect section
- surface/identity typecheck and biome 0; vitest counts in the report

**Files:**
- create: surface/identity/src/features/file/AgentConnect.tsx
- create: surface/identity/src/features/file/agent-connect.test.tsx
- modify: surface/identity/src/features/file/sections.tsx
- modify: surface/identity/src/features/file/tabs.ts

**Checklist:**
- C784 — An agent's file shows the complete claude mcp add and codex mcp add lines for this install and agent with a copy act each, and says which origin works now and which waits on AGENTS-015 R3.

**Stories:**
- S448 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person on the computer that runs Lys, I start my agent here like on any other computer, and I copy the line that connects Claude Code or Codex to it from the agent's own page instead of assembling it from help text.

## Boundaries

- No join, certificate, grant or key is created by this brief: the host is matched by a key the install already holds, and the Connect section renders strings only
- No advice sentence names a label the surface does not render
- No fixed wait, timer, poll or deadline anywhere, in code or test
- Nothing installed by the lane; every file touched stays under 500 code lines (ADR-111, by scripts/file-length.sh)

## Verification

- gate -stages nextest -crates lys,lys-identity-server -test 'package(lys) | package(lys-identity-server)'
- surface/identity: bun run typecheck, biome check, vitest (agent-connect.test.tsx counts in the report)
- gate -stages clippy, then the whole declared gate
- python3 scripts/design/validate.py docs/design/agents and python3 scripts/design/check-coverage.py docs/design/agents read for FAIL lines
