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

Tom, 28 September 2026 19:48, on Dot: 'I've decided that I want this to actually handle the launching of agents, either through a background terminal of its own or plugging into something else. I want it to be able to do all of the things that Argus can do currently through herdr, and what was supposed to be done through manifold.' Today Lys gives a start command and never runs it, so every agent still needs a person at a terminal or another tool that Lys cannot rely on. Amended 28 September 2026 20:45 by Waffles after Archie's review of the brief against the tree.

## Task

Add a runner: a background process on each machine that holds each started agent's pseudo-terminal, reached by lys-identity-server over a published protocol; make start run the agent through it; expose type, keys, read, wait, resize, compact, account rotation, wake and stop as routes (so the API and MCP carry them) and on a Sessions screen. Lys depends on no other runner.

## Requirements

### R1: Lys's own runner: each agent in its own background pseudo-terminal

Behavioural. New crate lys-runner and the command 'lys runner' (started by install as a login item beside the server, as the server is). It holds each session in its own pseudo-terminal with a scrollback of a configured size in bytes, keeps running when every screen is closed, and records each session's exit status and instant. Sessions survive a restart of lys-identity-server; a restart of the runner reports every session it held as ended, never as running. It listens only on a Unix socket in the install's run folder at mode 0600, and accepts only the server's signed requests. The runner may run on any machine: it dials the server (never the reverse) over the machine's authenticated connection, signing each request with the machine's own key as DIRECTORY-029's machine records name it, so an agent can run on another laptop. Its local control socket stays a Unix socket, mode 0600. The runner is one of the install's units (DIRECTORY-045): a restart of the runner ends every session it holds, and an upgrade says so, naming the running sessions, before its first stop. After a runner crash, sessions it held are reported ended_by_runner_restart with the instant the restart found them gone and no exit status; none is invented. The runner SHALL record with each session the identity of its process group leader. That identity is the leader's process id together with the instant that process started, as the operating system reports it. WHEN a restart finds a recorded session without an end, THE SYSTEM SHALL end its process group only when the process at the recorded id started at the recorded instant, compared exactly. A process at the recorded id with any other start instant is a stranger, and the runner SHALL NOT signal it or its group. A group whose leader is gone cannot be proved the runner's, and the runner SHALL NOT signal it. A recorded session that holds no start instant SHALL NOT be signalled. In each of those three cases the session is reported ended_by_runner_restart, and the runner's log names the process id and says it was not signalled. THE SYSTEM SHALL NOT signal a process group on its recorded number alone.

**Acceptance:**
- A started session keeps running after the server restarts, and its output is still readable.
- A runner restart reports its old sessions ended with their instants.
- A request not signed by the server is refused runner_request_unsigned.
- A runner on a second machine, dialling the server with that machine's key, starts an agent the server asked for.
- A runner killed and restarted reports its sessions ended_by_runner_restart with no exit status.
- A recorded session whose leader lives at the recorded start instant with a child that ignored the hang-up has its group ended, and the child is gone afterwards.
- A recorded session whose process id now belongs to a process with another start instant is reported ended_by_runner_restart.
- That other process is still running after the restart.
- The runner's log for that restart names that process id and says it was not signalled.
- A recorded session whose leader is gone while its group still holds a process is reported ended_by_runner_restart, and that process is still running afterwards.
- A recorded session that holds no start instant is reported ended_by_runner_restart, and no signal is sent.

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

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (server restart, output still readable) and row 2 (a clean runner restart reports each session ended with its instant): met by the existing tests at crates/lys-runner/tests/runner.rs:157 and :189.
Row 3 (runner_request_unsigned): met by crates/lys-runner/tests/runner.rs:545.
Row 4 (a runner on a second machine dials in): met by crates/lys-identity-server/tests/runner_start.rs:279 and crates/lys-runner/tests/dial_channel.rs:145.
Row 5 (killed and restarted, ended_by_runner_restart with no status): crates/lys-runner/tests/runner.rs:429, plus the restart_ends helper at :320, which asserts EndedByRunnerRestart and status None for every restart test.
Row 6 (leader alive at the recorded instant, child ignored the hang-up): the group is ended and the child is gone. Evidence: runner.rs:357 records the instant through pty::leader_started (crates/lys-runner/src/pty.rs:134), expects signal SIGKILL, the leader exits on signal 9, and reading the group's output reaches its end, so the cat is gone.
Rows 7 to 9 (stranger at the recorded id): runner.rs:381 checks that the session is reported ended_by_runner_restart with no signal, that the stranger still echoes a line after the restart and later exits normally, and that the log line names the pid and says 'was not signalled'.
Row 10 (leader gone, group still holds a process): runner.rs:398 kills only the leader and reaps it; the cat left in the group still echoes after the restart, and the log line is present.
Row 11 (no start instant): runner.rs:415 checks there is no signal, the process still echoes, and the leader exits successfully.
The decision itself is at crates/lys-runner/src/session.rs:165. The instant is recorded at crates/lys-runner/src/session/lifecycle.rs:45, and the field is at crates/lys-runner/src/state.rs:46. runner.rs:444 checks that a session the runner starts has its leader's start recorded as the system reports it.
- Deviation: R1's file list does not include crates/lys-runner/src/state.rs or crates/lys-runner/src/session/lifecycle.rs, and this round modified both. The earlier round created them as parts of the runner crate: the record the runner keeps, and session.rs's own submodule that runs processes. The new record field has to live in state.rs, and the one spawn site, which covers rotation re-spawns too, is in lifecycle.rs. CN9 says a file outside a row's wall needs a reviewed brief revision first; I am naming both files here for the reviewer rather than waiting. Also, the start instant is recorded to the second, the finest ps lstart reports on macOS and Linux. That is stated in the pty.rs doc.
- Files changed:
  - modified: `crates/lys-runner/src/pty.rs` — Removes end_left_group and Left, which signalled a group by its number alone. Adds leader_started(pid): the operating system's report of when a process started, from /bin/ps -o lstart= in the C locale and UTC, kept as text and compared exactly. It answers None when ps fails and says nothing (no such process); any other answer is refused leader_unreadable by name.
  - modified: `crates/lys-runner/src/session.rs` — Each Session carries leader_started, which is persisted and read back. left_behind(id, pid, recorded) sends SIGKILL to the group only when leader_started(pid) equals the recorded instant exactly. It does not signal when there is no start instant, when the leader is gone, when the process at that id started at another instant, or when ps could not be read. Each of those cases logs 'session <id>: process <pid> was not signalled: <why>', and the session is reported ended_by_runner_restart with no status.
  - modified: `crates/lys-runner/src/session/lifecycle.rs` — run() records the leader's start instant right after each spawn, including a spawn after an account rotation. If the start cannot be read it logs that the process will not be signalled by a restart and records none.
  - modified: `crates/lys-runner/src/state.rs` — KeptSession gains leader_started: Option<String> (serde default, skipped when None), so records written before this change still read, as records holding no start instant.
  - modified: `crates/lys-runner/tests/runner.rs` — Adds a Group fixture: a shell leading its own group with a cat in the group that ignores the hang-up and echoes each line it is sent. Also adds one log sink shared by the whole test binary, and tests for each new acceptance row. The existing lost-runner test now records a start instant.
- Checklist delivery:
  - [x] C374 — Lys ships a runner that holds each started agent in its own pseudo-terminal in the background, surviving the screen closing (DIRECTORY-050 R1). — The runner holds sessions in their own pseudo-terminals, and a restart now signals only groups proved to be its own.
- Story delivery:
  - [x] S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool. — Agents run in the background under Lys's own runner.

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

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Unchanged this round; delivered in the earlier round. The conformance suite is crates/lys-runner/tests/conformance.rs (from line 146). runner_protocol_mismatch at start is checked by crates/lys-identity-server/tests/runner_start.rs:249. Other tools' names are barred by rules/ast-grep/no-other-runner-names.yml.
- Deviation: (none)
- Checklist delivery:
  - [x] C375 — A published runner protocol lets another tool be a machine's runner; Lys needs no particular runner (DIRECTORY-050 R2). — The published protocol and its conformance suite are in the tree.
- Story delivery:
  - [x] S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool.

### R3: Start runs the agent

Behavioural. POST /agents/{id}/start on a machine with a runner renders the command as today and has the runner run it, with the working directory, environment and handles the launch record names; the session reports starting, then running when the runner confirms the process is up. A machine without a runner answers the command as today. Every refusal of the existing start is unchanged. The identity server and the lys-identity start files still spawn nothing: they ask the runner over its socket, and only lys-runner spawns a process, so DIRECTORY-029 R12 and crates/lys-identity/tests/start_no_spawn.rs stay green unchanged. This supersedes DIRECTORY-029's boundary against a lys launcher subcommand by Tom's word of 28 September 2026 19:4x (Lys runs the agents it starts); the no-spawn guarantee of the start path stands.

**Acceptance:**
- Starting an agent on a machine with the runner leaves it running and listed on the Sessions screen.
- A machine without a runner answers the command as before; the existing start tests pass unchanged.
- crates/lys-identity/tests/start_no_spawn.rs passes unchanged, and no file it scans names a process-spawning API.

**Files:**
- create: crates/lys-identity-server/tests/runner_start.rs
- modify: crates/lys-identity-server/src/start.rs
- modify: crates/lys-identity-server/src/launch_api.rs
- modify: crates/lys-identity-server/src/runtime_api.rs

**Checklist:**
- C376 — Starting an agent on a machine with a runner runs it there and reports it running, from the screen, the API and the MCP (DIRECTORY-050 R3).

**Stories:**
- S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Unchanged this round. crates/lys-identity-server/tests/runner_start.rs:171 checks that a start runs the agent and lists it running, and :225 that a machine without a runner answers the command as before. crates/lys-identity/tests/start_no_spawn.rs is unchanged. The only new spawn this round (of /bin/ps) is in lys-runner.
- Deviation: (none)
- Checklist delivery:
  - [x] C376 — Starting an agent on a machine with a runner runs it there and reports it running, from the screen, the API and the MCP (DIRECTORY-050 R3).
- Story delivery:
  - [x] S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool.

### R4: Type, keys, read, wait, resize and compact through Lys

Behavioural. Routes under /runtime/sessions/{id}: input (text, optionally followed by Enter), keys (named keys), read (the last N lines or bytes, with a cursor to read on from), wait (until a pattern appears in new output, answering when it does, with the matched text; the caller may give up by closing the request, and nothing in Lys ends it on a clock), resize, and compact (sends the harness's compaction command as its profile names it). Each act requires the operate relation on the agent, is refused by name without it, and leaves a receipt naming the caller and the act, never the text typed when the profile marks the session sensitive. A receipt for typed text carries its length and its SHA-256 digest, never the text, for every profile. read with a cursor older than the kept scrollback is refused cursor_expired naming the oldest cursor held; wait takes a literal string unless the caller asks for a regular expression by name.

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

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Unchanged this round. crates/lys-identity-server/tests/runner_api.rs:264 checks typing a line and reading it back, and :285 checks that each act is refused not_permitted without operate and leaves a receipt when admitted. Wait answering with the matched text is checked through the waited helper at :232.
- Deviation: (none)
- Checklist delivery:
  - [x] C377 — A session can be typed into, sent keys, read, waited on for a pattern, resized and compacted through Lys, each under a grant and with a receipt (DIRECTORY-050 R4).
- Story delivery:
  - [x] S155 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control.

### R5: Account rotation on usage limit

Behavioural. An agent's profile may name an ordered list of account handles held by the secrets broker and the words that mean a usage limit. When a session prints them, Lys ends it and starts it again on the next account in the list, resuming its session record, and records the move; at the list's end it stops, reports accounts_exhausted, and does not wrap round. Account values reach only the session's environment through the broker and never a route, a log or a receipt. A usage limit is taken from the harness's own signal where it has one (its exit status or structured event), not from words in the terminal, so an agent quoting the words rotates nothing; for a harness with no such signal, the profile declares the words and the brief's record says rotation there can be tripped by quoted text. Accounts are credentials over one session store per agent: rotation changes the credential the harness runs with, never where its sessions and transcripts live, so a rotated session resumes.

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

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Unchanged this round. crates/lys-identity-server/tests/runner_api.rs:387 checks that the session moves to the next account, stops accounts_exhausted at the end of the list, and that no account value appears in any answer, log line or receipt. The runner-side rotation tests are at crates/lys-runner/tests/runner.rs:635 and :683.
- Deviation: (none)
- Checklist delivery:
  - [x] C378 — A session that prints its usage-limit words moves to the next account in its list, by handle, never by value (DIRECTORY-050 R5).
- Story delivery:
  - [x] S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool.

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

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Unchanged this round. crates/lys-identity-server/tests/runner_api.rs:488 checks that a wake types into the live session or is refused no_live_session, and :511 that a stop ends every session and each shows confirmed with its exit instant.
- Deviation: (none)
- Checklist delivery:
  - [x] C379 — A message to an agent wakes its session; the emergency stop ends sessions through the runner and reports each confirmed (DIRECTORY-050 R6).
- Story delivery:
  - [x] S155 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control.

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

#### R7 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Unchanged this round. The Sessions screen with its Terminal view is surface/identity/src/features/runtime/Terminal.tsx with terminal.css, and surface/identity/tests/runtime-terminal.test.tsx covers the list, the terminal view, confirming Stop, and the refusals.
- Deviation: (none)
- Checklist delivery:
  - [x] C380 — A Sessions screen shows every running agent, its terminal read live, with type, keys and stop (DIRECTORY-050 R7).
- Story delivery:
  - [x] S155 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control.

## Boundaries

- SHALL NOT make Lys depend on Argus, herdr, Manifold or any other runner; their names do not appear in Lys code.
- SHALL NOT let the runner listen on any network address; it listens on its Unix socket only.
- SHALL NOT pass an account, key or token value through a route, a log, a receipt or a command line.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass; waits end on a signal or when the caller leaves.
- SHALL NOT add a silent fallback: every failure is a named refusal.
- SHALL NOT add a code line to crates/lys-identity-server/src/grants.rs, which already holds 498 of the 500 code lines the file-length leg allows (ADR-111). Code this card needs beside the grants goes in a new module, and code may move out of grants.rs.

## Verification

- The full Lys gate, ast-grep scan and the surface checks exit 0 at the card's head, measured by the card round.
- On a scratch install: start a shell agent from the screen, type to it, read it, wait on a pattern, stop it.
- sh scripts/file-length.sh exits 0 at the card's head.
- check_file_length.py counts no more than 498 code lines in crates/lys-identity-server/src/grants.rs at the card's head.
