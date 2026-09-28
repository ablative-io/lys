---
type: brief
id: DIRECTORY-050
cluster: directory
title: Lys runs and drives the agents it starts: its own background terminal, or any runner speaking its protocol
---

# DIRECTORY-050: Lys runs and drives the agents it starts: its own background terminal, or any runner speaking its protocol

> **Cluster:** directory
> **Depends on:** DIRECTORY-048
> **Design anchor:**
> - ADR-117 — Lys runs the agents it starts: its own background terminal, or a runner another tool provides — Lys runs the agents it starts. A runner is the part that holds an agent's terminal: Lys ships one of its own, a background process on each machine that runs each started agent in its own pseudo-terminal, and accepts any other runner that speaks the same small, published runner protocol. Through a runner, Lys starts, types into, presses keys in, reads, waits on, resizes, compacts and stops a session, rotates it across its accounts when it reports a usage limit, and wakes it with a message, each act under the caller's grants and recorded. Lys depends on no particular runner: its own is the default, another is chosen per machine.
> **Checklist:**
> - C374 — Lys ships a runner that holds each started agent in its own pseudo-terminal in the background, surviving the screen closing (DIRECTORY-050 R1).
> - C375 — A published runner protocol lets another tool be a machine's runner; Lys needs no particular runner (DIRECTORY-050 R2).
> - C376 — Starting an agent on a machine with a runner runs it there and reports it running, from the screen, the API and the MCP (DIRECTORY-050 R3).
> - C377 — A session can be typed into, sent keys, read, waited on for a pattern, resized and compacted through Lys, each under a grant and with a receipt (DIRECTORY-050 R4).
> - C378 — A session that prints its usage-limit words moves to the next account in its list, by handle, never by value (DIRECTORY-050 R5).
> - C379 — A message to an agent wakes its session; the emergency stop ends sessions through the runner and reports each confirmed (DIRECTORY-050 R6).
> - C380 — A Sessions screen shows every running agent, its terminal read live, with type, keys and stop (DIRECTORY-050 R7).
> **Stories:**
> - S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool.
> - S155 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control.

## Purpose

Tom, 28 September 2026 19:48, on Dot: 'I've decided that I want this to actually handle the launching of agents, either through a background terminal of its own or plugging into something else. I want it to be able to do all of the things that Argus can do currently through herdr, and what was supposed to be done through manifold.' Today Lys gives a start command and never runs it, so every agent still needs a person at a terminal or another tool that Lys cannot rely on.

## Task

Add a runner: a background process on each machine that holds each started agent's pseudo-terminal, reached by lys-identity-server over a published protocol; make start run the agent through it; expose type, keys, read, wait, resize, compact, account rotation, wake and stop as routes (so the API and MCP carry them) and on a Sessions screen. Lys depends on no other runner.

## Requirements

### R1: Lys's own runner: each agent in its own background pseudo-terminal

Behavioural. New crate lys-runner and the command 'lys runner' (started by install as a login item beside the server, as the server is). It holds each session in its own pseudo-terminal with a scrollback of a configured size in bytes, keeps running when every screen is closed, and records each session's exit status and instant. Sessions survive a restart of lys-identity-server; a restart of the runner reports every session it held as ended, never as running. It listens only on a Unix socket in the install's run folder at mode 0600, and accepts only the server's signed requests.

**Acceptance:**
- A started session keeps running after the server restarts, and its output is still readable.
- A runner restart reports its old sessions ended with their instants.
- A request not signed by the server is refused runner_request_unsigned.

**Files:**
- create: crates/lys-runner/Cargo.toml
- create: crates/lys-runner/src/lib.rs
- create: crates/lys-runner/src/pty.rs
- create: crates/lys-runner/src/session.rs
- create: crates/lys-runner/src/socket.rs
- create: crates/lys-runner/tests/runner.rs
- create: crates/lys/src/cli/runner.rs
- modify: Cargo.toml
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/identity/install.rs

**Checklist:**
- C374 — Lys ships a runner that holds each started agent in its own pseudo-terminal in the background, surviving the screen closing (DIRECTORY-050 R1).

**Stories:**
- S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool.

### R2: A published runner protocol; any tool may be the runner

Behavioural. The protocol is a short list of acts (start, input, keys, read, wait, resize, end, status) over JSON, described in the OpenAPI document of DIRECTORY-048 R6 as the runner's own section. A machine's record names its runner: Lys's own by default, or another tool's socket or address speaking the protocol. The server holds no code specific to any other tool. A conformance test suite in the crate runs against any runner given its address.

**Acceptance:**
- The conformance suite passes against Lys's own runner.
- A machine naming a runner that answers a wrong protocol version is refused runner_protocol_mismatch at start, by name.
- ast-grep finds no other tool's name in the server's runner code.

**Files:**
- create: crates/lys-runner/src/protocol.rs
- create: crates/lys-runner/tests/conformance.rs
- create: crates/lys-identity-server/src/runner_client.rs
- modify: crates/lys-identity-server/src/network_store.rs
- modify: crates/lys-identity-server/src/openapi.rs

**Checklist:**
- C375 — A published runner protocol lets another tool be a machine's runner; Lys needs no particular runner (DIRECTORY-050 R2).

**Stories:**
- S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool.

### R3: Start runs the agent

Behavioural. POST /agents/{id}/start on a machine with a runner renders the command as today and has the runner run it, with the working directory, environment and handles the launch record names; the session reports starting, then running when the runner confirms the process is up. A machine without a runner answers the command as today. Every refusal of the existing start is unchanged.

**Acceptance:**
- Starting an agent on a machine with the runner leaves it running and listed on the Sessions screen.
- A machine without a runner answers the command as before; the existing start tests pass unchanged.

**Files:**
- create: crates/lys-identity-server/tests/runner_start.rs
- modify: crates/lys-identity-server/src/start.rs
- modify: crates/lys-identity-server/src/launch_api.rs
- modify: crates/lys-identity-server/src/runtime_api.rs

**Checklist:**
- C376 — Starting an agent on a machine with a runner runs it there and reports it running, from the screen, the API and the MCP (DIRECTORY-050 R3).

**Stories:**
- S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool.

### R4: Type, keys, read, wait, resize and compact through Lys

Behavioural. Routes under /runtime/sessions/{id}: input (text, optionally followed by Enter), keys (named keys), read (the last N lines or bytes, with a cursor to read on from), wait (until a pattern appears in new output, answering when it does, with the matched text; the caller may give up by closing the request, and nothing in Lys ends it on a clock), resize, and compact (sends the harness's compaction command as its profile names it). Each act requires the operate relation on the agent, is refused by name without it, and leaves a receipt naming the caller and the act, never the text typed when the profile marks the session sensitive.

**Acceptance:**
- Typing a line into a shell session and reading it back shows the line and its output.
- wait answers when the pattern appears, matched text included.
- A caller without operate is refused not_permitted for each act.
- Each act leaves a receipt.

**Files:**
- create: crates/lys-identity-server/src/runner_api.rs
- create: crates/lys-identity-server/tests/runner_api.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/receipts_api.rs

**Checklist:**
- C377 — A session can be typed into, sent keys, read, waited on for a pattern, resized and compacted through Lys, each under a grant and with a receipt (DIRECTORY-050 R4).

**Stories:**
- S155 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control.

### R5: Account rotation on usage limit

Behavioural. An agent's profile may name an ordered list of account handles held by the secrets broker and the words that mean a usage limit. When a session prints them, Lys ends it and starts it again on the next account in the list, resuming its session record, and records the move; at the list's end it stops, reports accounts_exhausted, and does not wrap round. Account values reach only the session's environment through the broker and never a route, a log or a receipt.

**Acceptance:**
- A session printing the limit words moves to the next account and resumes.
- At the list's end the session is stopped with accounts_exhausted.
- No account value appears in any answer, log line or receipt, checked by a test.

**Files:**
- create: crates/lys-runner/src/rotation.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: crates/lys-identity/src/provisioning.rs

**Checklist:**
- C378 — A session that prints its usage-limit words moves to the next account in its list, by handle, never by value (DIRECTORY-050 R5).

**Stories:**
- S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool.

### R6: Wake with a message; stop through the runner

Behavioural. POST /agents/{id}/wake with a message types it into the agent's live session as its profile says a message is delivered, or refuses no_live_session by name. The emergency stop (stop_api.rs) asks the runner to end every session and marks each confirmed only when the runner reports its exit.

**Acceptance:**
- A wake types the message into the live session.
- A stop ends every session of the agent and each shows confirmed with its exit instant.

**Files:**
- modify: crates/lys-identity-server/src/stop_api.rs
- modify: crates/lys-identity-server/src/runner_api.rs

**Checklist:**
- C379 — A message to an agent wakes its session; the emergency stop ends sessions through the runner and reports each confirmed (DIRECTORY-050 R6).

**Stories:**
- S155 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control.

### R7: Sessions screen: see, type to and stop every agent

Behavioural. The Sessions screen (surface/identity/src/features/runtime) lists every running session the caller may see, and opening one shows its terminal read live through read with a cursor, a line to type into, a row of common keys, and Stop, styled from the design tokens with no default controls.

**Acceptance:**
- Opening a session shows its live output; typing a line sends it.
- Stop asks to confirm by naming the agent and then ends it.
- Surface tests cover the list, the terminal view and the refusals.

**Files:**
- create: surface/identity/src/features/runtime/Terminal.tsx
- create: surface/identity/src/features/runtime/terminal.css
- modify: surface/identity/src/features/runtime/Sessions.tsx

**Checklist:**
- C380 — A Sessions screen shows every running agent, its terminal read live, with type, keys and stop (DIRECTORY-050 R7).

**Stories:**
- S155 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control.

## Boundaries

- SHALL NOT make Lys depend on Argus, herdr, Manifold or any other runner; their names do not appear in Lys code.
- SHALL NOT let the runner listen on any network address; it listens on its Unix socket only.
- SHALL NOT pass an account, key or token value through a route, a log, a receipt or a command line.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass; waits end on a signal or when the caller leaves.
- SHALL NOT add a silent fallback: every failure is a named refusal.

## Verification

- The full Lys gate, ast-grep scan and the surface checks exit 0 at the card's head, measured by the card round.
- On a scratch install: start a shell agent from the screen, type to it, read it, wait on a pattern, stop it.
