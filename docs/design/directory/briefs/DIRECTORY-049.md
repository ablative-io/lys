---
type: brief
id: DIRECTORY-049
cluster: directory
title: Lys MCP server, secure and compact: three tools over the published API, the caller's own identity on every call
---

# DIRECTORY-049: Lys MCP server, secure and compact: three tools over the published API, the caller's own identity on every call

> **Cluster:** directory
> **Depends on:** DIRECTORY-048
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> **Checklist:**
> - C369 — The MCP server offers exactly three tools over the published API; the tool list is under 2 KB (DIRECTORY-049 R1, R5).
> - C370 — Every MCP call is a person's Lys token obtained through the MCP authorization flow, or an agent's signed request; there is no key, shared secret or unauthenticated mode (DIRECTORY-049 R2).
> - C371 — Every MCP call runs the same handler and grant check as the HTTP route, as the caller; no MCP answer returns a secret's value (DIRECTORY-049 R3, R4).
> - C372 — A destructive write through MCP needs the target's id repeated in confirm; every write leaves a receipt naming the caller and 'mcp' (DIRECTORY-049 R4, R6).
> - C373 — An agent's launch renders 'lys mcp' into its MCP configuration, signing with its own certificate key held by handle (DIRECTORY-049 R7).
> **Stories:**
> - S153 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person or an agent using an AI assistant, I want the assistant to reach Lys with my own identity and nothing more, so that it can never see a secret or do what I could not.

## Purpose

Tom, 28 September 2026 evening: 'Have you started work on the MCP? Have we thought about that at all? It will need to be secure and compact.' The mock-up gives every agent 'identity (this service)' as an MCP server and nothing serves it. An assistant reaching Lys can reach identities, grants and secrets, so it must never hold more than its caller, never see a secret's value, and cost as little context as possible.

## Task

Serve MCP from lys-identity-server at /mcp and from 'lys mcp' over stdio for agents, with three tools that dispatch into the same handlers the HTTP routes use, authenticated as the caller on every call, with no credential of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.

## Requirements

### R1: Three tools over the published API

Behavioural. lys_schema: with no argument, answers every verb (the OpenAPI operation ids of DIRECTORY-048 R6) in one line each; with a verb, answers its request and response schema. lys_read: calls a reading verb (GET) with its members. lys_write: calls a writing verb with its request. Verbs come from the generated OpenAPI document, so a new route is reachable with no MCP change. Each tool call is dispatched in-process into the router as a request carrying the caller's identity; there is no second implementation of any verb. Tools carry MCP annotations: lys_read readOnlyHint, lys_write destructiveHint.

**Acceptance:**
- tools/list answers exactly three tools.
- Every OpenAPI operation is reachable through lys_read or lys_write, walked by a test.
- A route added in a test router is reachable through MCP with no MCP code change.

**Files:**
- create: crates/lys-identity-server/src/mcp.rs
- create: crates/lys-identity-server/src/mcp_tools.rs
- create: crates/lys-identity-server/tests/mcp.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/Cargo.toml

**Checklist:**
- C369 — The MCP server offers exactly three tools over the published API; the tool list is under 2 KB (DIRECTORY-049 R1, R5).

**Stories:**
- S153 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person or an agent using an AI assistant, I want the assistant to reach Lys with my own identity and nothing more, so that it can never see a secret or do what I could not.

### R2: Every call is the caller's own: a person's Lys token or an agent's signature

Behavioural. /mcp speaks streamable HTTP and follows the MCP authorization specification with Lys as the authorization server: it answers 401 with the protected-resource metadata at /.well-known/oauth-protected-resource; a client signs the person in on Lys's own pages with authorization code and PKCE; the token is bound to the /mcp audience and is refused anywhere else, and /mcp refuses any token not issued for it (no token passthrough). An agent's call carries lys-agent-signature (agent_signature.rs) over the MCP request, with the same checks. There is no API key, shared secret, static token or unauthenticated mode, and loopback is not trust. The Origin header is checked against Lys's own origin and registered clients, and the listener binds 127.0.0.1 unless configured.

**Acceptance:**
- An unauthenticated call is answered 401 with the metadata address.
- A token issued for another audience is refused token_wrong_audience.
- A replayed agent signature is refused by the existing nonce check.
- A request with a foreign Origin is refused origin_refused.

**Files:**
- create: crates/lys-identity-server/src/mcp_auth.rs
- create: crates/lys-identity-server/tests/mcp_auth.rs
- modify: crates/lys-identity-server/src/mcp.rs
- modify: crates/lys-identity-server/src/agent_signature.rs

**Checklist:**
- C370 — Every MCP call is a person's Lys token obtained through the MCP authorization flow, or an agent's signed request; there is no key, shared secret or unauthenticated mode (DIRECTORY-049 R2).

**Stories:**
- S153 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person or an agent using an AI assistant, I want the assistant to reach Lys with my own identity and nothing more, so that it can never see a secret or do what I could not.

### R3: Same grant check as the screens; no authority of its own

Behavioural. Each call runs the route's own grant check as the caller; the MCP layer holds no service account, API key or elevated identity, and adds no check of its own that could allow more. A refusal comes back as the route's named refusal, unchanged.

**Acceptance:**
- For every verb, a caller without the grant is refused through MCP with the same name as through HTTP, walked by a test.
- The MCP module holds no credential: a test builds it with no secrets store and every tool still answers.

**Files:**
- modify: crates/lys-identity-server/src/mcp_tools.rs
- modify: crates/lys-identity-server/tests/mcp.rs

**Checklist:**
- C371 — Every MCP call runs the same handler and grant check as the HTTP route, as the caller; no MCP answer returns a secret's value (DIRECTORY-049 R3, R4).

**Stories:**
- S153 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person or an agent using an AI assistant, I want the assistant to reach Lys with my own identity and nothing more, so that it can never see a secret or do what I could not.

### R4: No secret value ever; destructive writes need the target repeated

Behavioural. No MCP answer carries a secret's value, a private key, a token or a password: secret verbs answer the handle and its metadata only, and a test walks every response schema in the OpenAPI document and fails on any field marked secret that MCP would return. A write the document marks destructive (retire an identity, revoke a grant, stop, delete a secret, retire an app) is refused confirm_required unless the call carries confirm equal to the target's id; the refusal says in words what the write would do.

**Acceptance:**
- Reading a secret through MCP answers its handle and never its value.
- The response-schema walk fails on a planted secret field.
- A revoke without confirm is refused confirm_required naming the grant; with it, it applies.

**Files:**
- modify: crates/lys-identity-server/src/mcp_tools.rs
- modify: crates/lys-identity-server/src/openapi.rs

**Checklist:**
- C371 — Every MCP call runs the same handler and grant check as the HTTP route, as the caller; no MCP answer returns a secret's value (DIRECTORY-049 R3, R4).
- C372 — A destructive write through MCP needs the target's id repeated in confirm; every write leaves a receipt naming the caller and 'mcp' (DIRECTORY-049 R4, R6).

**Stories:**
- S153 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person or an agent using an AI assistant, I want the assistant to reach Lys with my own identity and nothing more, so that it can never see a secret or do what I could not.

### R5: Compact

Behavioural. tools/list is under 2 KB. Answers are compact JSON with no pretty printing; every list is paged at 50 with a cursor; lys_schema with no argument answers one line per verb, and the full schema only for a named verb.

**Acceptance:**
- A test measures tools/list under 2048 bytes.
- A list of 120 identities answers in three pages.

**Files:**
- modify: crates/lys-identity-server/src/mcp_tools.rs

**Checklist:**
- C369 — The MCP server offers exactly three tools over the published API; the tool list is under 2 KB (DIRECTORY-049 R1, R5).

**Stories:**
- S153 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person or an agent using an AI assistant, I want the assistant to reach Lys with my own identity and nothing more, so that it can never see a secret or do what I could not.

### R6: Every write leaves a receipt

Behavioural. Every lys_write that applies leaves a receipt (receipts_api.rs) naming the caller, the verb, the target and 'mcp' as the way in, never the request's secret members.

**Acceptance:**
- A write through MCP is listed at GET /receipts with via 'mcp' and its caller.

**Files:**
- modify: crates/lys-identity-server/src/receipts_api.rs
- modify: crates/lys-identity-server/src/mcp_tools.rs

**Checklist:**
- C372 — A destructive write through MCP needs the target's id repeated in confirm; every write leaves a receipt naming the caller and 'mcp' (DIRECTORY-049 R4, R6).

**Stories:**
- S153 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person or an agent using an AI assistant, I want the assistant to reach Lys with my own identity and nothing more, so that it can never see a secret or do what I could not.

### R7: Agents reach it with 'lys mcp', signing with their own key

Behavioural. 'lys mcp --agent ID' serves MCP over stdio and forwards each call to the server as a request signed with the agent's certificate key, which it reads by handle from lys-secrets at each call and never writes to a file, an argument or the environment. The launch template (launch_template.rs) renders it into an agent's MCP configuration as 'identity (this service)' when its profile asks for it.

**Acceptance:**
- An agent launched from a profile asking for it lists the three tools and reads its own identity.
- The rendered configuration and the process arguments carry no key material, checked by a test.
- A withdrawn certificate refuses the next call by name.

**Files:**
- create: crates/lys/src/cli/mcp.rs
- create: crates/lys/tests/mcp_stdio.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys-identity-server/src/launch_template.rs

**Checklist:**
- C373 — An agent's launch renders 'lys mcp' into its MCP configuration, signing with its own certificate key held by handle (DIRECTORY-049 R7).

**Stories:**
- S153 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person or an agent using an AI assistant, I want the assistant to reach Lys with my own identity and nothing more, so that it can never see a secret or do what I could not.

## Boundaries

- SHALL NOT put any app's name, kind, schema or special case in Lys code, configuration or tests beyond fixtures named for the test.
- SHALL NOT make Lys call, poll, wait on or read the store of any app.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass.
- SHALL NOT print or log a secret, token, password or key value on any path.
- SHALL NOT add a silent fallback: every failure is a named refusal.
- SHALL NOT give the MCP layer any credential, service account or authority of its own.
- SHALL NOT return any secret value, key, token or password through any tool.
- SHALL NOT add a fourth tool; a new capability is a new route, reached through the three.

## Verification

- The full Lys gate, ast-grep scan and the surface checks exit 0 at the card's head, measured by the card round.
- A real MCP client (Claude Code) connects to a scratch install, signs in through Lys's page, lists three tools and reads its own identity.
