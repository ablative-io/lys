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

Behavioural. New crate lys-runner and the command 'lys runner' (started by install as a login item beside the server, as the server is). It holds each session in its own pseudo-terminal with a scrollback of a configured size in bytes, keeps running when every screen is closed, and records each session's exit status and instant. Sessions survive a restart of lys-identity-server; a restart of the runner reports every session it held as ended, never as running. It listens only on a Unix socket in the install's run folder at mode 0600, and accepts only the server's signed requests. The runner may run on any machine: it dials the server (never the reverse) over the machine's authenticated connection, signing each request with the machine's own key as DIRECTORY-029's machine records name it, so an agent can run on another laptop. Its local control socket stays a Unix socket, mode 0600. The runner is one of the install's units (DIRECTORY-045): a restart of the runner ends every session it holds, and an upgrade says so, naming the running sessions, before its first stop. After a runner crash, sessions it held are reported ended_by_runner_restart with the instant the restart found them gone and no exit status; none is invented.

**Acceptance:**
- A started session keeps running after the server restarts, and its output is still readable.
- A runner restart reports its old sessions ended with their instants.
- A request not signed by the server is refused runner_request_unsigned.
- A runner on a second machine, dialling the server with that machine's key, starts an agent the server asked for.
- A runner killed and restarted reports its sessions ended_by_runner_restart with no exit status.

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
- How: Lys's own runner holds each agent in its own background pseudo-terminal and process group behind a Unix socket, and serves only there. Every request is signed by the server's key over the runner's per-connection greeting (runner id and a fresh challenge), so a replay or a request misaddressed to another runner is refused by name. The runner's state outlives the server, and an exclusive state lock keeps a second runner off it. A restarted runner ends whatever a lost runner left behind and reports each session ended_by_runner_restart. A stop signal ends every session and waits for each exit on the exit event. In round 3, the two lys bin tests that failed the workspace leg under load were fixed at their causes, each proven by a stress that failed 7 of 90 runs on the old code and 0 of 90 on the new.
- Deviation: These need a person's decision or a reviewer-approved brief revision (CN9); I cannot approve them myself.
(1) 'An upgrade says so, naming the running sessions, before its first stop' is not delivered. DIRECTORY-045's upgrade is not in the tree, so this should be deferred to DIRECTORY-045 by a brief revision.
(2) Publish decision: lys is published, but it depends on lys-runner, which is publish = false, so `cargo publish -p lys` cannot succeed. Either lys-runner is published (freezing its wire format) or the runner command is kept out of the published lys; that is a release decision to record in the brief.
(3) Files outside the brief's list were edited and need naming in a revision:
- tests/identity_contract/src/harness.rs
- crates/lys-identity-server/src/{error.rs, error_status.rs, config.rs, stop_api.rs, provisioning_api.rs, provisioning_store.rs, receipts_api.rs, runner_dial.rs, runner_sessions.rs, runner_acts.rs, launch_api.rs, network_store.rs, runtime_api.rs}
- the lys install files
- the new test files
- round 3's crates/lys/src/identity/install/exit_wait.rs and crates/lys/src/identity/loopback_http_tests.rs, which fix pre-existing races the gate measured
(4) Defaults, recorded so the values are known: SCROLLBACK defaults to 1 MiB for the configured --scrollback; COLUMNS 120 and ROWS 40 are resizable start sizes.
(5) Protocol v1 was changed in place (greeting, signed bytes, dial epoch), since nothing under it has landed.
The CN5 demonstration on a scratch install is a person's step after landing; it was not performed.
- Files changed:
  - created: `crates/lys-runner/src/protocol.rs` — Greeting {version, runner, challenge}. signed_bytes covers the version, runner, challenge and act. verify_request refuses runner_request_unsigned, malformed, protocol_mismatch (checked first), replayed and misaddressed.
  - created: `crates/lys-runner/src/socket.rs` — Greets each connection with a fresh challenge and refuses a live socket before opening state. serve_until_stopped holds SIGTERM, SIGINT and SIGHUP, then ends every session and removes the socket. Listens on its Unix socket only.
  - created: `crates/lys-runner/src/state.rs` — Persistent runner.id. An exclusive flock on the state directory (runner_state_held), released on drop.
  - created: `crates/lys-runner/src/session.rs` — Sessions, their record and restart handling. An unended session's leftover process group is ended (SIGKILL recorded) and reported ended_by_runner_restart.
  - created: `crates/lys-runner/src/session/lifecycle.rs` — Session lifecycle. Account moves and exhaustion are logged by handle only.
  - created: `crates/lys-runner/src/pty.rs` — One background pseudo-terminal per session, with its own process group. end_left_group.
  - created: `crates/lys-runner/src/scrollback.rs` — Scrollback bounded by the configured --scrollback.
  - created: `crates/lys-runner/src/rotation.rs` — Account rotation that never wraps.
  - created: `crates/lys-runner/src/client.rs` — Connection reads the greeting; ask signs over it.
  - created: `crates/lys-runner/src/dial.rs` — The dial bridge: machine-signed, bound to the server epoch, rereads the epoch on runner_dial_stale.
  - created: `crates/lys-runner/src/dial/transport.rs` — HTTP/1.1 over rustls. Cleartext only to loopback; a user in the address is refused.
  - created: `crates/lys-runner/src/error.rs` — Named refusals, and said() with an optional sink.
  - created: `crates/lys-runner/src/lib.rs` — Crate invariants.
  - created: `crates/lys-runner/Cargo.toml` — publish = false. rustls, webpki-roots and tokio; rcgen as a dev-dependency.
  - created: `crates/lys-runner/tests/runner.rs` — 12 tests: the session survives the server, restart reporting, a lost runner, state held, and replay after a restart.
  - created: `crates/lys-runner/tests/dial_channel.rs` — A TLS dial signed under the epoch; an unknown authority is refused; seven cleartext or tricky addresses are refused.
  - created: `crates/lys/src/cli/runner.rs` — `lys runner serve` and `lys runner dial` arguments: --scrollback defaulting to 1 MiB, --server-ca.
  - created: `crates/lys/src/commands/runner.rs` — Runs serve (printing the listening line) and dial.
  - created: `crates/lys/tests/runner_cli.rs` — A SIGKILLed runner, once restarted, ends a HUP-ignoring survivor. SIGTERM ends every session, and the next runner reports each one exited.
  - modified: `crates/lys/src/cli.rs` — Runner subcommand.
  - modified: `crates/lys/src/commands/mod.rs` — Declares the runner command.
  - modified: `crates/lys/src/commands/error.rs` — Runner errors.
  - modified: `crates/lys/src/main.rs` — Dispatches runner.
  - modified: `crates/lys/Cargo.toml` — Depends on lys-runner.
  - modified: `crates/lys/src/identity/install.rs` — The install lays out the runner.
  - modified: `crates/lys/src/identity/install/layout.rs` — Runner socket and state paths.
  - modified: `crates/lys/src/identity/install/server_config.rs` — Runner settings in the server config.
  - modified: `crates/lys/src/identity/install/exit_wait.rs` — Round 3: a watch releases the lock by name (flock unlock) as soon as it is granted. A concurrently forked child's inherited copy of the descriptor can no longer keep a gone service reading as alive. hold() keeps the lock for the service through take().
  - modified: `crates/lys/src/identity/loopback_http_tests.rs` — Round 3: the refusing address is the local end of a held live connection (Refusing::hold, then close), so a test running beside it cannot be handed the port.
  - modified: `Cargo.toml` — Workspace member lys-runner.
  - modified: `Cargo.lock` — New dependency edges only.
  - modified: `docs/design/directory/briefs/DIRECTORY-050.md` — Re-rendered from its JSON, so the design gate exits 0.
- Checklist delivery:
  - [x] C374 — Lys ships a runner that holds each started agent in its own pseudo-terminal in the background, surviving the screen closing (DIRECTORY-050 R1). — Runner, restart reporting and signed requests are delivered and tested. The upgrade clause is deferred (see deviation).
- Story delivery:
  - [x] S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool. — Agents run in Lys's own background terminal and survive a server restart.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] A started session keeps running after the server restarts, and its output is still readable. — crates/lys-runner/tests/runner.rs:155 a_session_keeps_running_when_its_server_goes_and_is_read_by_the_next; passed in my full workspace run (runner: 12 passed)
  - [x] A runner restart reports its old sessions ended with their instants. — crates/lys-runner/tests/runner.rs:185 a_clean_restart_reports_each_session_ended_with_its_instant; crates/lys/tests/runner_cli.rs:142 a_runner_asked_to_stop_ends_every_session_and_the_next_reports_each_exit; both pass
  - [x] A request not signed by the server is refused runner_request_unsigned. — crates/lys-runner/src/protocol.rs verify_request (empty signature, non-hex and a failed Ed25519 verify all give RunnerError::Unsigned); tests/runner.rs:340 a_request_the_server_did_not_sign_is_refused_by_name and conformance.rs:157 every_request_refusal_fires_by_name pass
  - [x] A runner on a second machine, dialling the server with that machine's key, starts an agent the server asked for. — crates/lys-identity-server/tests/runner_start.rs:256 a_runner_on_a_second_machine_dials_in_and_starts_the_agent; admission at crates/lys-identity-server/src/runner_dial.rs admitted() verifies the machine key over dial_signed_bytes under the epoch; runner_start: 6 passed
  - [x] A runner killed and restarted reports its sessions ended_by_runner_restart with no exit status. — crates/lys-runner/tests/runner.rs:224 a_session_lost_with_its_runner_is_ended_by_the_restart_with_no_status; crates/lys/tests/runner_cli.rs:93 a_killed_runner_restarted_reports_its_sessions_ended_by_the_restart; both pass
- Checklist verified: C374
- Stories verified: S154
- Issues:
  - Spec clause not delivered: 'an upgrade says so, naming the running sessions, before its first stop'. DIRECTORY-045's upgrade is not in the tree. Deliver it, or get a brief revision that defers it to DIRECTORY-045.
  - The published crate `lys` (crates/lys/Cargo.toml has no publish = false) now depends on lys-runner, which is publish = false, so `cargo publish -p lys` cannot succeed. A person must decide whether to publish lys-runner (freezing its wire format) or keep lys's runner command out of the published crate. Record that decision in the brief.
  - Files outside the brief's named files were edited without the CN9 reviewer-approved brief revision: tests/identity_contract/src/harness.rs, crates/lys-identity-server/src/{error.rs, error_status.rs, config.rs, stop_api.rs, provisioning_api.rs, provisioning_store.rs, receipts_api.rs, runner_dial.rs, runner_sessions.rs, runner_acts.rs}, the lys install files and the new test files. Revise the brief to name them before this lands.
  - The SCROLLBACK default of 1 MiB (crates/lys/src/cli/runner.rs:6) is a default for the size the spec says is configured (--scrollback), not an unrequested limit; recorded only so the value is known. The COLUMNS/ROWS start size (runner_sessions.rs:36-39) is likewise a resizable default.

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
- How: The protocol is published by lys-runner and served at GET /runner/protocol. Any tool that passes the conformance suite can be the runner. The server checks the version first and refuses a mismatch as runner_protocol_mismatch. An ast-grep rule keeps other runners' names out of Lys code.
- Deviation: (1) The protocol is not yet folded into DIRECTORY-048 R6's OpenAPI document, because crates/lys-identity-server/src/openapi.rs is not in the tree. Until R6 lands, it is served on its own at GET /runner/protocol; a brief revision should record this interim route.
(2) A Socket runner record is trusted by its path alone, with no pinning. The administrator's choice of path is the trust decision; this is to be stated in the brief.
- Files changed:
  - created: `crates/lys-runner/src/published.rs` — The published protocol: greeting, request, signed bytes, dial epoch route and header, runner_dial_stale and five request refusals. Served at GET /runner/protocol.
  - created: `crates/lys-runner/tests/conformance.rs` — A conformance suite any runner can be run against. Every request refusal fires, counted.
  - created: `crates/lys-identity-server/src/runner_client.rs` — Runner records (Socket, or Dialled with a pinned runner id), the dial hub with its per-start epoch, and signing over a greeting.
  - created: `crates/lys-identity-server/src/runner_dial.rs` — Dial routes: the epoch, a stale epoch refused 409, the nonce shape, a pinned runner.
  - modified: `crates/lys-identity-server/src/error.rs` — Runner and DialStale errors.
  - modified: `crates/lys-identity-server/src/error_status.rs` — Runner refusal statuses; DialStale maps to 409.
  - created: `crates/lys-identity-server/tests/runner_start.rs` — A wrong protocol version is refused by name; the dial pin; a captured dial is admitted once.
  - created: `rules/ast-grep/no-other-runner-names.yml` — No other runner's name appears in Lys code.
- Checklist delivery:
  - [x] C375 — A published runner protocol lets another tool be a machine's runner; Lys needs no particular runner (DIRECTORY-050 R2). — The protocol is published and has a conformance suite. The OpenAPI fold is deferred (see deviation).
- Story delivery:
  - [x] S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool. — Any runner speaking the protocol can be used.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] The conformance suite passes against Lys's own runner. — crates/lys-runner/tests/conformance.rs (5 tests, runs against any runner via LYS_RUNNER_CONFORMANCE_SOCKET/KEY); my full run: 5 passed
  - [x] A machine naming a runner that answers a wrong protocol version is refused runner_protocol_mismatch at start, by name. — crates/lys-identity-server/tests/runner_start.rs:226 a_runner_on_another_protocol_version_is_refused_by_name passes; read_greeting checks the version first; error_status.rs runner_status maps runner_protocol_mismatch to 502
  - [x] ast-grep finds no other tool's name in the server's runner code. — rules/ast-grep/no-other-runner-names.yml covers crates/lys-runner/**, lys-identity-server/src/runner_*.rs and the lys runner CLI files; `ast-grep scan --config sgconfig.yml` exits 0; grep -rniE 'argus|herdr|manifold' over lys-identity-server/src, lys-runner and crates/lys/src finds nothing
- Checklist verified: C375
- Stories verified: S154
- Issues:
  - Spec clause not delivered: the protocol must be 'described in the OpenAPI document of DIRECTORY-048 R6 as the runner's own section'. crates/lys-identity-server/src/openapi.rs does not exist, and the protocol is served alone at GET /runner/protocol (crates/lys-runner/src/published.rs). Fold it in once DIRECTORY-048 R6 lands, or revise the brief to record the interim route.
  - A Socket runner record is trusted by path alone, with no pinning; state in the brief that the administrator's choice of path is the trust decision.

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
- How: When the machine names a runner, a start runs the agent there, and the agent is listed running. A machine without a runner answers the command as before, and the existing start tests are unchanged.
- Deviation: start_launcher.rs uses a fixture launcher, because the directory start owners are still not landed. DirectoryLauncher's body is exercised through /start-command in runner_start.rs.
- Files changed:
  - modified: `crates/lys-identity-server/src/start.rs` — Launcher seam: a start runs on the machine's runner when one is named.
  - modified: `crates/lys-identity-server/src/launch_api.rs` — The start-command path runs on the runner.
  - created: `crates/lys-identity-server/src/runner_sessions.rs` — Session listing and the start size. A missing pid is refused runner_reply_malformed.
  - created: `crates/lys-identity-server/src/runner_acts.rs` — Acts relayed to runners.
  - modified: `crates/lys-identity-server/src/routes.rs` — Runner routes.
  - modified: `crates/lys-identity-server/src/lib.rs` — Module declarations.
  - modified: `crates/lys-identity-server/src/config.rs` — Runner records in the config.
  - modified: `crates/lys-identity-server/Cargo.toml` — Depends on lys-runner.
  - created: `crates/lys-identity-server/tests/start_launcher.rs` — POST /agents/{id}/start through the launcher: the runner member, a refusal by name, or the start alone.
- Checklist delivery:
  - [x] C376 — Starting an agent on a machine with a runner runs it there and reports it running, from the screen, the API and the MCP (DIRECTORY-050 R3). — Tested in runner_start.rs and start_launcher.rs.
- Story delivery:
  - [x] S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool. — Start runs the agent.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] Starting an agent on a machine with the runner leaves it running and listed on the Sessions screen. — crates/lys-identity-server/tests/runner_start.rs:171 a_start_on_a_machine_with_the_runner_runs_and_is_listed_running; start_launcher.rs:286 a_start_the_runner_ran_answers_its_runner_member; both pass
  - [x] A machine without a runner answers the command as before; the existing start tests pass unchanged. — runner_start.rs:202 a_machine_without_a_runner_answers_the_command_as_before; start_launcher.rs:332; git status shows tests/start.rs and start_command.rs unmodified; start 5 passed, start_command 4 passed
  - [x] crates/lys-identity/tests/start_no_spawn.rs passes unchanged, and no file it scans names a process-spawning API. — git status shows no change under crates/lys-identity; start_no_spawn: 3 passed in my full run (it scans crates/lys-identity/src/start); the service asks the runner over its socket and spawns nothing
- Checklist verified: C376
- Stories verified: S154
- Issues:
  - POST /agents/{id}/start is exercised through a fixture launcher (start_launcher.rs) because the start owners are NotLanded; DirectoryLauncher's run path is exercised through /start-command. This matches the tree as it stands; no code change is due.

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
- How: Each act goes through Lys under the operate permission, is signed to the runner and receipted.
- Deviation: (none)
- Files changed:
  - created: `crates/lys-identity-server/src/runner_api.rs` — Type, keys, read, wait, resize and compact, each needing operate and each receipted (text only as length and SHA-256).
  - modified: `crates/lys-identity-server/src/receipts_api.rs` — Receipts for runner acts.
  - modified: `crates/lys-identity-server/src/runtime_api.rs` — Sessions view.
  - created: `crates/lys-identity-server/tests/runner_api.rs` — Typing and reading back, a not_permitted refusal for each act, and a receipt per act.
- Checklist delivery:
  - [x] C377 — A session can be typed into, sent keys, read, waited on for a pattern, resized and compacted through Lys, each under a grant and with a receipt (DIRECTORY-050 R4). — Tested in runner_api.rs and conformance.rs.
- Story delivery:
  - [x] S155 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control. — An operator drives sessions through Lys.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] Typing a line into a shell session and reading it back shows the line and its output. — crates/lys-identity-server/tests/runner_api.rs:264 typing_a_line_and_reading_it_back_shows_the_line_and_its_output; conformance.rs:236 a_session_is_typed_to_read_and_waited_on; pass
  - [x] wait answers when the pattern appears, matched text included. — conformance.rs:236 a_session_is_typed_to_read_and_waited_on; runner.rs:496 a_regex_match_after_bytes_that_are_not_text_answers_the_byte_cursor; pass
  - [x] A caller without operate is refused not_permitted for each act. — runner_api.rs:285 each_act_is_refused_not_permitted_without_operate_and_each_admitted_act_leaves_a_receipt; ServerError::NotPermitted maps to 403 in error_status.rs
  - [x] Each act leaves a receipt. — runner_api.rs:285 (same test) asserts a receipt per admitted act, with typed text only as length and SHA-256; runner_api: 5 passed
- Checklist verified: C377
- Stories verified: S155

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
- How: On a usage limit the session moves to the next account by handle; at the end of the list it stops with accounts_exhausted. Captured runner and service log lines are counted, and a test proves no account value appears in any of them, nor in any answer or file.
- Deviation: The profile settings live in provisioning_store.rs; the brief names crates/lys-identity/src/provisioning.rs, which does not exist. Recorded for the brief revision.
- Files changed:
  - modified: `crates/lys-identity-server/src/provisioning_store.rs` — Profile account handles and limit words.
  - modified: `crates/lys-identity-server/src/provisioning_api.rs` — The rotation settings.
  - modified: `crates/lys-identity-server/src/network_store.rs` — Machine runner records.
  - modified: `crates/lys-identity-server/tests/provisioning.rs` — The settings round-trip.
  - modified: `tests/identity_contract/src/harness.rs` — start_saying captures the service's log lines.
- Checklist delivery:
  - [x] C378 — A session that prints its usage-limit words moves to the next account in its list, by handle, never by value (DIRECTORY-050 R5). — Tested in runner.rs and runner_api.rs.
- Story delivery:
  - [x] S154 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to start an agent from Lys and have it running in the background, so that I don't need a terminal or another tool. — Rotation happens without a person.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A session printing the limit words moves to the next account and resumes. — runner.rs:430 a_usage_limit_moves_the_session_on_and_the_list_end_stops_it; runner.rs:478 declared_words_move_a_harness_with_no_signal; runner_api.rs:387; pass
  - [x] At the list's end the session is stopped with accounts_exhausted. — runner.rs:430 and runner_api.rs:387 a_usage_limit_moves_the_session_to_the_next_account_and_the_list_end_stops_it; rotation.rs never wraps
  - [x] No account value appears in any answer, log line or receipt, checked by a test. — runner_api.rs:387 captures the runner's said() lines (lys_runner::error::also_to) and the service's say lines (start_saying), asserts both logs are non-empty and that PLANTED appears in no answer, log line, receipt or file; rotation.rs carries handles only
- Checklist verified: C378
- Stories verified: S154
- Issues:
  - The brief names crates/lys-identity/src/provisioning.rs, which does not exist; the profile settings live in crates/lys-identity-server/src/provisioning_store.rs. Correct the path in the brief text; no code change is due.

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
- How: A wake types its message into the live session. A stop ends every session, and each shows confirmed with its exit instant (runner_api.rs, emergency_stop.rs).
- Deviation: (none)
- Files changed:
  - modified: `crates/lys-identity-server/src/stop_api.rs` — A stop goes through the runner and asks every unconfirmed session at once, keeping the stop record whole.
- Checklist delivery:
  - [x] C379 — A message to an agent wakes its session; the emergency stop ends sessions through the runner and reports each confirmed (DIRECTORY-050 R6). — Tested.
- Story delivery:
  - [x] S155 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control. — An operator can wake and stop an agent.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A wake types the message into the live session. — runner_api.rs:488 a_wake_types_the_message_into_the_live_session_and_needs_one; passes
  - [x] A stop ends every session of the agent and each shows confirmed with its exit instant. — runner_api.rs:511 a_stop_ends_every_session_and_each_shows_confirmed_with_its_exit_instant; tests/emergency_stop.rs 3 passed; stop_api.rs asks every unconfirmed session at once
- Checklist verified: C379
- Stories verified: S155

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
- How: The Sessions screen lists every agent session and opens a live terminal to type to. Stop asks the user to confirm by naming the agent.
- Deviation: (none)
- Files changed:
  - created: `surface/identity/src/features/runtime/Sessions.tsx` — The Sessions list.
  - created: `surface/identity/src/features/runtime/Terminal.tsx` — Live terminal view, typing, keys, and Stop confirmed by the agent's name.
  - created: `surface/identity/src/features/runtime/terminal.css` — Terminal styles.
  - modified: `surface/identity/src/features/runtime/StartAgent.tsx` — Shows the runner member.
  - modified: `surface/identity/src/features/sessions/Sessions.tsx` — Links to the runtime sessions.
  - modified: `surface/identity/src/routes.tsx` — Routes.
  - created: `surface/identity/tests/runtime-terminal.test.tsx` — The list, the terminal view and the refusals.
- Checklist delivery:
  - [x] C380 — A Sessions screen shows every running agent, its terminal read live, with type, keys and stop (DIRECTORY-050 R7). — Surface tests passed in the round (exit 0).
- Story delivery:
  - [x] S155 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to see what an agent is doing, type to it, and stop it from one screen, so that every agent is in one place I control. — Every agent can be seen, typed to and stopped.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] Opening a session shows its live output; typing a line sends it. — surface/identity/tests/runtime-terminal.test.tsx:66-85 (live reads with cursor, input posted {text:'ls -l', enter:true}); surface leg passed (331 tests)
  - [x] Stop asks to confirm by naming the agent and then ends it. — runtime-terminal.test.tsx:98-104: no /end before confirm, dialog text 'Stop Scribe?', then /end posted and exit shown
  - [x] Surface tests cover the list, the terminal view and the refusals. — runtime-terminal.test.tsx:49-63 list, 66-104 terminal, 114 refusal 'not_permitted: ...' shown in role=alert; surface leg exit 0
- Checklist verified: C380
- Stories verified: S155

## Boundaries

- SHALL NOT make Lys depend on Argus, herdr, Manifold or any other runner; their names do not appear in Lys code.
- SHALL NOT let the runner listen on any network address; it listens on its Unix socket only.
- SHALL NOT pass an account, key or token value through a route, a log, a receipt or a command line.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass; waits end on a signal or when the caller leaves.
- SHALL NOT add a silent fallback: every failure is a named refusal.

## Verification

- The full Lys gate, ast-grep scan and the surface checks exit 0 at the card's head, measured by the card round.
- On a scratch install: start a shell agent from the screen, type to it, read it, wait on a pattern, stop it.
