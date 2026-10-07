---
type: brief
id: DIRECTORY-086
cluster: directory
title: The agent request signature names its audience, and a replay from another server is refused
---

# DIRECTORY-086: The agent request signature names its audience, and a replay from another server is refused

> **Cluster:** directory
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> **Checklist:**
> - C520 — The agent request signature names Lys's own origin, and another audience is refused (DIRECTORY-086 R1).
> - C521 — v1 is retired in one cutover, and stored v1 draft evidence still verifies (DIRECTORY-086 R2).
> - C522 — A signature made before the process started is refused (DIRECTORY-086 R3).
> **Stories:**
> - S270 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person whose agent talks to many servers, I want what my agent signs for one server to be useless at Lys, so that no server it talks to can act or read as my agent.

## Purpose

A security defect. The agent request signature names no server, so a header captured by any server an agent talks to can be replayed against Lys's /mcp within a minute, to act and read as that agent. A restart also clears the nonce cache. Percy found this reading R1b (Waffles, 7 October, fcec91e0).

## Task

Bind the signature to its audience (R1); retire v1 in one cutover while stored evidence still verifies (R2); refuse signatures from before the process started (R3). Out of scope: a persistent nonce store; the grant token path.

## Requirements

### R1: The agent request signature names its audience, and another server's is refused

Behavioural. Today the signed payload is the domain lys-identity/agent-request/v1, the method, the path, the body digest, the signing time and the nonce (agent_signature.rs 49-52). It names no server, so a header an agent sent to any other server can be replayed to Lys within the 60 s window (WINDOW_MS, line 36). The v2 payload is the domain lys-identity/agent-request/v2, then the audience, then the same five fields. The audience is the identity server's own public origin, the one its protected-resource document names. The header becomes five words: agent, audience, signing time, nonce, signature. A header whose audience is not this service's origin is refused agent_signature_wrong_audience before its signature is checked. The signature is then checked over the v2 payload with the service's own origin, never the header's word, so a changed audience word cannot pass.

**Acceptance:**
- Red at main: a v1 header taken from a request signed for another server's path is accepted at /mcp. At the head the same capture is refused.
- A v2 header naming another audience is refused agent_signature_wrong_audience; one with this service's audience word but signed for another is refused as a bad signature.

**Files:**
- create: crates/lys-identity-server/src/agent_signature_audience_tests.rs
- modify: crates/lys-identity-server/src/agent_signature.rs
- modify: crates/lys-identity-server/src/drafts_api.rs
- modify: crates/lys-identity-server/src/mcp_endpoint.rs
- modify: crates/lys-openapi/src/lib.rs

**Checklist:**
- C520 — The agent request signature names Lys's own origin, and another audience is refused (DIRECTORY-086 R1).

**Stories:**
- S270 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person whose agent talks to many servers, I want what my agent signs for one server to be useless at Lys, so that no server it talks to can act or read as my agent.

### R2: v1 is retired in one cutover, and stored v1 evidence still verifies as evidence

Behavioural. There is no period in which both versions are admitted. The four-word v1 header is refused agent_signature_v1_retired in the same change that adds v2. Every signer in the estate moves in the same install: the runner's dial (lys-runner/src/dial/agent.rs 144-145) builds the v2 payload with the origin it dials, and the broker's /_lys/sign signs whatever payload it is given, so it needs no change. A search of /Users/tom/Developer/ablative on 7 October found no other signer: the only other hit was data in prototyping/call-map. The cutover is one Lys install of the runner and the identity server together. A runner left on v1 is refused by name and does not fall back. Drafts already stored with a v1 signature keep verifying as evidence of what was signed then (lys-identity/src/draft_event.rs 347-353), because that is a record, not an admission. A new draft carries v2.

**Acceptance:**
- A v1 header is refused agent_signature_v1_retired at every admitting route.
- A runner request signed through the broker is admitted end to end with v2.
- A draft stored with a v1 signature before the cutover still verifies after a reopen.

**Files:**
- modify: crates/lys-runner/src/dial/agent.rs
- modify: crates/lys-identity/src/draft_event.rs
- modify: crates/lys-identity-server/src/agent_signature.rs
- modify: crates/lys-identity-server/src/agent_signature_audience_tests.rs

**Checklist:**
- C521 — v1 is retired in one cutover, and stored v1 draft evidence still verifies (DIRECTORY-086 R2).

**Stories:**
- S270 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person whose agent talks to many servers, I want what my agent signs for one server to be useless at Lys, so that no server it talks to can act or read as my agent.

### R3: A signature made before this process started is refused

Behavioural. The nonce cache lives in the process (Nonces, agent_signature.rs 41-42), so after a restart a header captured in the last 60 s would pass again. The service records the time it began admitting. A signature whose signing time is earlier than that is refused agent_signature_before_start, so no replay window crosses a restart, and nothing is persisted for it. A client whose call is refused this way signs again.

**Acceptance:**
- A header accepted once, presented again after a restart of the service, is refused agent_signature_before_start.
- A header signed after the restart is admitted.

**Files:**
- modify: crates/lys-identity-server/src/agent_signature.rs
- modify: crates/lys-identity-server/src/agent_signature_audience_tests.rs

**Checklist:**
- C522 — A signature made before the process started is refused (DIRECTORY-086 R3).

**Stories:**
- S270 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person whose agent talks to many servers, I want what my agent signs for one server to be useless at Lys, so that no server it talks to can act or read as my agent.

## Boundaries

- SHALL NOT admit a v1 header after this lands, or keep v1 admission beside v2.
- SHALL NOT take the audience from the header when checking the signature.
- SHALL NOT break verification of stored v1 draft evidence.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.

## Verification

- The design gate, parsed: every cluster clean, no FAIL, every file valid.
- Each requirement's red at main quoted in the handback.
- The piece's one Lys battery on the Mac, judged by parsed output: whole-crate nextest, cargo test --doc, ast-grep, file length, fmt and clippy pedantic in both configurations, 0 failed.
