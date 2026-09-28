---
type: brief
id: DIRECTORY-060
cluster: directory
title: The runner holds one holder key per session and signs presentations for that session only
---

# DIRECTORY-060: The runner holds one holder key per session and signs presentations for that session only

> **Cluster:** directory
> **Depends on:** DIRECTORY-050, DIRECTORY-051, SECRETS-008
> **Design anchor:**
> - ADR-125 — An agent session's holder key is held by the runner, and a session proves itself by peer credentials and ancestry — The runner makes one holder key per session in memory. A start becomes key, then issue, then spawn. The harness and lys mcp ask the runner for single presentations over its socket. The session is proved by peer credentials plus the process ancestry reaching the session's own root pid, checked on every connection, on macOS and Linux both, through a named maintained dependency and with no unsafe in Lys code.
> **Checklist:**
> - C418 — A runner session's handles are issued to a key the runner made for it in memory, before anything is spawned (DIRECTORY-060 R1).
> - C419 — The runner signs a presentation only for a peer proved by credentials and ancestry to be the session the handle was issued to, on macOS and Linux (DIRECTORY-060 R2).
> - C420 — The harness and lys mcp get every presentation from the runner and hold no key (DIRECTORY-060 R3).
> **Stories:**
> - S168 (Started agent, Reports back to the directory and presents its session credential) — As an agent started by Lys, I want every handle I am launched with to work, without ever holding the key that presents for it.

## Purpose

A broker handle is usable only with presentations signed by its holder key (crates/lys-secrets/src/broker/admit.rs line 77), and nothing gives an agent session one. So every handle DIRECTORY-050 R3 launches an agent with, and every account handle 050 R5 rotates through, is unusable. ADR-125 rules that the runner holds one key per session in memory and signs single presentations for that session, proved by peer credentials and process ancestry. This brief builds that on top of DIRECTORY-050 once it has landed, and on the issue route of SECRETS-008.

## Task

Give each runner session one Ed25519 holder key made in memory. Make the start key, then issue, then spawn. Add a presentation act to the runner's socket that signs one presentation for one of the session's own handles, for a peer proved to belong to that session. The seed never leaves the runner.

## Requirements

### R1: One key per session, and a start that is key, then derive, then render, then spawn

Behavioural. When the runner is asked to start a session (DIRECTORY-050 R3), it makes one Ed25519 key for that session in memory with a new Ed25519Identity::generate in crates/lys-core/src/keys/identity.rs, which draws its seed from the operating system's randomness and takes no key material. It answers the key's public key to lys-identity-server before it spawns anything. The launch record names the agent's held handles, each as a HandleName whose id is a handle the agent already holds (crates/lys-identity-server/src/launch_template.rs lines 24 to 31). For each one the server derives a session handle under it through POST /_lys/handles/issue (SECRETS-008 R1), with under set to that id and holder_key set to the session's public key, on behalf of the person starting the agent. A derived handle takes the uses, the end and the spend left on the held handle, and ends when the held handle is dropped, so the start invents no bound of its own. Only when every derive has answered does the server render the launch template with render (launch_template.rs line 156), and each HandleName it passes carries the derived id in place of the held id, so the template's use_only list (lines 141 to 145) names only handles bound to the session's key. It then hands each derived id and token to the runner in the spawn act over the runner's socket, keeps no copy of any token, and tells the runner to spawn. Each issued token is kept in the session's record beside the key and never leaves the runner except in an answer of present (R2). The session's environment carries only the handle ids, as crates/lys-identity-server/src/launch_template.rs lines 6 to 9 say secrets ride, and LYS_RUNNER_SOCKET, because a process of the same user can read another's environment. A refused derive ends the start with that refusal by name, nothing is rendered and nothing is spawned. The key and the tokens live in the session's record in crates/lys-runner/src/session.rs, are never written to a file, a log, an argument or the environment, and are dropped when the session ends. A restart of the runner loses every key and token, as it already ends every session (050 R1).

**Acceptance:**
- A started session's handles are admitted by the broker for presentations the runner signs, in crates/lys-runner/tests/session_key.rs.
- Every handle id in the rendered template's use_only list is a handle whose parent is the held handle it replaced.
- No held handle id appears in the rendered template.
- A derived handle's end is the end of the held handle it was derived under.
- A start whose derive is refused answers that refusal by name.
- After a start whose derive is refused, the runner has spawned no process for it.
- No file the runner writes holds the session key's seed or its hex.
- The spawned process's environment and arguments hold no byte of the session key's seed.
- Two sessions started together hold different public keys.
- The spawned process's environment holds no handle token.
- The service lys-identity-server keeps no handle token after the spawn act answers.

**Files:**
- create: crates/lys-runner/src/session_key.rs
- create: crates/lys-runner/tests/session_key.rs
- modify: crates/lys-core/src/keys/identity.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-identity-server/src/start.rs
- modify: crates/lys-identity-server/src/runner_client.rs
- modify: crates/lys-identity-server/src/launch_template.rs

**Checklist:**
- C418 — A runner session's handles are issued to a key the runner made for it in memory, before anything is spawned (DIRECTORY-060 R1).

**Stories:**
- S168 (Started agent, Reports back to the directory and presents its session credential) — As an agent started by Lys, I want every handle I am launched with to work, without ever holding the key that presents for it.

### R2: The presentation act, for a peer proved to be the session

Behavioural. The runner's socket (crates/lys-runner/src/socket.rs, mode 0600) gains one act, present, beside the acts of 050 R2. It takes a handle id, an operation id, a signing instant and the request digest of lys_secrets::request_digest (crates/lys-secrets/src/handle.rs line 28). It answers every header a handle route reads (crates/lys-secrets/src/bin/lys-secrets/serve.rs lines 203 to 210). These are the handle's token in hex as lys-handle, from the session's record, and one presentation made by Presentation::sign (handle.rs line 184) with the session's key, as the four wire words of to_wire (handle.rs line 129). It refuses present_not_session when the peer is not proved to be a session's, and present_not_this_handle when the handle id is not one issued to that session. The peer is proved on every connection by its credentials and its ancestry. The peer's pid comes from the socket, through nix 0.31.3 with the option LocalPeerPid on macOS and PeerCredentials on Linux. The peer's user must equal the runner's. Its parent chain is walked to a session's own root pid, the pid the runner spawned. On macOS the parent pid comes from libproc 0.14.11. On Linux it comes from reading /proc/<pid>/stat. A chain that reaches no session root, or breaks, is refused present_not_session. The connection stays open while the proof and the answer are made, so the peer's pid cannot be reused meanwhile. The server-signed acts of 050 R1 are unchanged, and present joins the judge and collector acts of DIRECTORY-051 R6 as the only acts a session peer may ask. No unsafe block is added to Lys code. The peer proof already exists: DIRECTORY-051 R6 creates crates/lys-runner/src/peer.rs to exactly this proof, with the session leader's start identity compared before and after the walk. present calls that module and adds no second proof.

**Acceptance:**
- A child of a session's process asking present for one of its handles gets a presentation the broker admits.
- The answer of present holds the handle's token and the four wire words, and a request carrying them is admitted by the broker.
- A process that is not a descendant of any session's root is refused present_not_session.
- A descendant of one session asking for another session's handle is refused present_not_this_handle.
- A session peer asking any act but present is refused as 050 R1 refuses an unsigned request.
- The answer of present holds no byte of the session key's seed.
- The card's diff adds no line containing the word unsafe under crates/.
- The manifest crates/lys-runner/Cargo.toml names nix at 0.31.3 and libproc at 0.14.11.

**Files:**
- create: crates/lys-runner/tests/present.rs
- modify: crates/lys-runner/Cargo.toml
- modify: crates/lys-runner/src/socket.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-runner/src/peer.rs

**Checklist:**
- C419 — The runner signs a presentation only for a peer proved by credentials and ancestry to be the session the handle was issued to, on macOS and Linux (DIRECTORY-060 R2).

**Stories:**
- S168 (Started agent, Reports back to the directory and presents its session credential) — As an agent started by Lys, I want every handle I am launched with to work, without ever holding the key that presents for it.

### R3: The harness and lys mcp present through the runner

Behavioural. Every broker call a session makes, the hand-over of account values at spawn under 050 R5 included, asks the runner's present act for its presentation. The helper that does so for a Lys-side caller lives in crates/lys-runner/src/present_client.rs, which DIRECTORY-049 R7 names for `lys mcp`. It reaches the runner by the socket path the session's environment names as LYS_RUNNER_SOCKET, sets on the broker request the headers present answers, and holds no key and keeps no token past the one request.

**Acceptance:**
- A rotation under 050 R5 hands the next account's value to the new session through a presentation the runner signed.
- The helper present_client.rs holds no Ed25519Identity and reads no key file.
- The helper present_client.rs reads no handle token from the environment.

**Files:**
- create: crates/lys-runner/src/present_client.rs
- modify: crates/lys-runner/src/rotation.rs
- modify: crates/lys-runner/src/lib.rs

**Checklist:**
- C420 — The harness and lys mcp get every presentation from the runner and hold no key (DIRECTORY-060 R3).

**Stories:**
- S168 (Started agent, Reports back to the directory and presents its session credential) — As an agent started by Lys, I want every handle I am launched with to work, without ever holding the key that presents for it.

## Boundaries

- SHALL NOT write, log, pass as an argument or put in the environment any byte of a session key's seed.
- SHALL NOT put a handle token in a spawned session's environment, arguments or any file.
- SHALL NOT add an unsafe block, or a dependency other than nix 0.31.3 and libproc 0.14.11.
- SHALL NOT sign a presentation for a handle that was not issued to the asking session.
- SHALL NOT refuse on macOS what is admitted on Linux, or the reverse.
- SHALL NOT change the server-signed acts of DIRECTORY-050 R1 and R2.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- `cargo test -p lys-runner --test present` exits 0 on Dean's laptop at the card's head.
- `cargo test -p lys-runner --test session_key` exits 0 on Dean's laptop at the card's head.
