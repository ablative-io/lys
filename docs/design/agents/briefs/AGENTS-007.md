---
type: brief
id: AGENTS-007
cluster: agents
title: The operations screen opens on a network tree of machines and their running agents, and selecting one shows its terminal
---

# AGENTS-007: The operations screen opens on a network tree of machines and their running agents, and selecting one shows its terminal

> **Cluster:** agents
> **Depends on:** AGENTS-002
> **Design anchor:**
> - ADR-117 — Lys runs the agents it starts: its own background terminal, or a runner another tool provides — Lys runs the agents it starts. A runner is the part that holds an agent's terminal: Lys ships one of its own, a background process on each machine that runs each started agent in its own pseudo-terminal, and accepts any other runner that speaks the same small, published runner protocol. Through a runner, Lys starts, types into, presses keys in, reads, waits on, resizes, compacts and stops a session, rotates it across its accounts when it reports a usage limit, and wakes it with a message, each act under the caller's grants and recorded. Lys depends on no particular runner: its own is the default, another is chosen per machine.
> - ADR-137 — Argus's remaining functions move into Lys as data beside goals, and a seat is a Lys record launched by the runner through the proxy; no shared library — Words (five slots, four layers, templates), variables (per agent and session, revision, author, expiry), schedules and seats are identity-server records in Lys's log, rendered and acted on at delivery through DIRECTORY-064's dispatcher with the contributing revisions in the receipt. No shared library: Lodestone is a read-only query engine over a store and holds nothing; haematite is the store under Lys already; a library would be a second place for the same data. Seat start, stop and restart are deliberate acts under Lys rights (ADR-136). Argus's alarms, rules, launches and queue are not moved into Lys: they become liminal services and aion work (ADR-136 section 3). A seat moves off herdr and Argus one at a time, Waffles's first.
> **Checklist:**
> - C737 — The operations screen has a third view, the network tree: every machine the network knows as a branch, every seat on it as a leaf carrying the agent's display name and the seat's state as the runner last reported it; a stopped or never-started seat stays on the tree greyed, with its start act in place (AGENTS-007 R1).
> - C738 — Selecting a running leaf opens that seat's terminal beside the tree, fed by the same session stream the canvas uses, with the seat's send and stop acts; selecting another leaf swaps the terminal; a seat whose runner cannot be read is named with its reason, never shown blank (AGENTS-007 R2).
> - C739 — The network tree is the default view of the operations screen; a person's chosen view is kept with their canvas arrangement by the service and restored on their next visit; the canvas and the sessions table remain one click away (AGENTS-007 R3).
> **Stories:**
> - S420 (Tom, owner) — As the owner, I want to open Lys and see the network as a tree of machines and the agents running on them, and click one to watch its terminal, so that the state of the whole estate is one glance and one click away.

## Purpose

The operations screen today has two views: the canvas, where a person arranges seat windows by hand, and the sessions table. Neither answers the first question a person has when they open Lys: what is running where. Tom asked (13 October 2026, 11:4x) for a third view, the new default, that shows the network as a tree, the machines as branches and the agents running on them as leaves, and lets a person select one and see its terminal. Everything it needs exists on the server: GET /network lists the machines with the agents each may run, GET /seats lists every seat with its machine, session and state as the runner last reported, and the canvas already streams a session's terminal through the runtime session routes. This brief is the view and its default, nothing new on the server.

## Task

Add the network tree view to the operations screen: a tree of machines and seats read from GET /network and GET /seats, a terminal pane fed by the existing session stream when a running leaf is selected, the seat's send and stop acts beside it, and the tree as the screen's default view with the person's choice kept by the service with their canvas arrangement.

## Requirements

### R1: The network tree: machines as branches, seats as leaves with their state

Behavioural. WHEN the operations screen shows the network tree THE SYSTEM SHALL read GET /network and GET /seats and draw every machine that is not retired as a branch named by the machine's name and kind, and under it every seat whose machine it is as a leaf carrying the agent's display name, the seat's transport (pty or managed) and the seat's state exactly as the seat list answers it (online-idle, online-working, offline, not-seen, unknown), with a mark per state. A seat that is offline or not-seen stays on the tree greyed with its start act in place; a seat the runner could not be read for shows the list's reason beside it. A machine with no seats shows as an empty branch, never hidden. The tree refreshes on the seat list's own cadence (the same interval the canvas uses), never by a page reload. Text that would overflow a leaf is truncated with the whole name in its title.

**Acceptance:**
- A fixture seat list of two machines (one retired) and five seats (states online-idle, online-working, offline, not-seen, unknown) renders one branch with five leaves, each leaf's mark matching its state, and no branch for the retired machine; a seat list answering runner 'unknown' with a reason renders every leaf with that reason.
- Clicking the start act on an offline leaf posts POST /seats/{name}/start with a fresh operation id and the leaf changes to the state the answer names, without a reload.
- A machine with no seats renders as a branch with the words 'no seats' under it.
- Rendering is under 50 ms for 10 machines and 200 seats in the component test's clock.

**Files:**
- create: surface/identity/src/features/runtime/NetworkTree.tsx
- create: surface/identity/src/features/runtime/network-tree.ts
- create: surface/identity/src/features/runtime/network-tree.css
- create: surface/identity/src/features/runtime/NetworkTree.test.tsx
- modify: surface/identity/src/api.ts

**Checklist:**
- C737 — The operations screen has a third view, the network tree: every machine the network knows as a branch, every seat on it as a leaf carrying the agent's display name and the seat's state as the runner last reported it; a stopped or never-started seat stays on the tree greyed, with its start act in place (AGENTS-007 R1).

**Stories:**
- S420 (Tom, owner) — As the owner, I want to open Lys and see the network as a tree of machines and the agents running on them, and click one to watch its terminal, so that the state of the whole estate is one glance and one click away.

### R2: Selecting a running leaf shows its terminal

Behavioural. WHEN a person selects a leaf whose seat is online THE SYSTEM SHALL open that seat's session terminal in a pane beside the tree, using the existing GpuTerminal component and the runtime session stream (read-bytes, resize, input) exactly as the canvas does, with the seat's send box (POST /seats/{name}/send) and stop act (POST /seats/{name}/stop) above it. Selecting another leaf swaps the pane to that seat's session and ends the earlier stream. Selecting a leaf that is not online shows the seat's state and its start act in the pane instead of a terminal. When the session ends while shown, the pane says so by the end the stream names and the leaf's mark follows the next seat list. The pane's width is a person's to drag and is kept with the view (R3).

**Acceptance:**
- Selecting an online-idle leaf mounts GpuTerminal with that seat's session id; selecting a second online leaf unmounts the first (its stream's abort is called once) and mounts the second.
- Typing in the send box and pressing Enter posts POST /seats/{name}/send with the text and clears the box on a delivered answer; a refusal is shown by name beside the box.
- The stop act posts POST /seats/{name}/stop and the pane shows the end the answer names.
- Selecting an offline leaf shows the state and the start act and mounts no terminal.

**Files:**
- create: surface/identity/src/features/runtime/NetworkTreePane.tsx
- create: surface/identity/src/features/runtime/NetworkTreePane.test.tsx
- modify: surface/identity/src/features/runtime/GpuTerminal.tsx

**Checklist:**
- C738 — Selecting a running leaf opens that seat's terminal beside the tree, fed by the same session stream the canvas uses, with the seat's send and stop acts; selecting another leaf swaps the terminal; a seat whose runner cannot be read is named with its reason, never shown blank (AGENTS-007 R2).

**Stories:**
- S420 (Tom, owner) — As the owner, I want to open Lys and see the network as a tree of machines and the agents running on them, and click one to watch its terminal, so that the state of the whole estate is one glance and one click away.

### R3: The tree is the default view and a person's choice is kept

Behavioural. THE SYSTEM SHALL open the operations screen (route #/canvas, and the operations entry in the rail and the palette) on the network tree unless the person has chosen another view; the view switch (tree, canvas, sessions) sits where the canvas's own tools sit today and keeps the keyboard shortcuts the canvas uses. The chosen view, the selected leaf and the pane width are kept with the person's canvas arrangement through the same service-kept layout the canvas keeps (canvas-kept.ts), so the choice follows the person, not the browser; a layout kept before this brief, which names no view, opens on the tree. The canvas and the sessions table are unchanged and one click away.

**Acceptance:**
- A first visit with no kept layout renders the tree; a kept layout naming view 'canvas' renders the canvas; a kept layout from before this brief (no view field) renders the tree.
- Switching views writes the kept layout once per change with the view, the selected leaf and the pane width, and a reload restores all three.
- The rail's operations entry and the palette's operations command land on the kept view.
- The existing canvas tests and the sessions table test pass unchanged.

**Files:**
- modify: surface/identity/src/features/runtime/SessionCanvas.tsx
- modify: surface/identity/src/features/runtime/canvas-kept.ts
- modify: surface/identity/src/features/runtime/CanvasDock.tsx
- modify: surface/identity/src/routes.tsx
- modify: surface/identity/src/shell/railItems.ts
- modify: surface/identity/src/shell/Palette.tsx

**Checklist:**
- C739 — The network tree is the default view of the operations screen; a person's chosen view is kept with their canvas arrangement by the service and restored on their next visit; the canvas and the sessions table remain one click away (AGENTS-007 R3).

**Stories:**
- S420 (Tom, owner) — As the owner, I want to open Lys and see the network as a tree of machines and the agents running on them, and click one to watch its terminal, so that the state of the whole estate is one glance and one click away.

## Boundaries

- Nothing new on the server: the tree reads GET /network and GET /seats as they stand, and the terminal uses the runtime session routes the canvas already uses; a server change found necessary is a FOUND, not a line in this brief.
- No second way to arrange windows: the tree is a view of the estate, the canvas remains the place a person arranges; the tree never writes canvas marks.
- No polling of the terminal: the terminal pane follows the stream; only the seat list is read on its cadence.
- No product's name in surface/ outside tests (rules/ast-grep/no-app-names.yml).

## Verification

- This handwritten brief passes scripts/design/gate.sh, judged by its parsed failures, never by its exit code.
- Written whole before any build (Tom, 9 October 19:2x); then tsc and vitest for surface/identity on Tom's Mac (his 4 October word for the surface), the component tests named in R1 to R3, and the surface package; no cargo leg, since no crate changes.
- Independent review of the rendered screen against Tom's words of 13 October before he sees it (8 October rule): a tree of the network, the agents running on it, select one and see its terminal, as the default.
- Four-heading handback naming every changed file, the kept-layout field added, and the worst credible failure stated plainly.
