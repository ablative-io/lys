---
type: brief
id: DIRECTORY-049
cluster: directory
title: Lys MCP server, secure and compact: three tools over the published API, the caller's own identity on every call
---

# DIRECTORY-049: Lys MCP server, secure and compact: three tools over the published API, the caller's own identity on every call

> **Cluster:** directory
> **Depends on:** DIRECTORY-047, DIRECTORY-048, DIRECTORY-050, DIRECTORY-060, SECRETS-006, SECRETS-008
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

Behavioural. lys_schema: with no argument, answers every verb (the OpenAPI operation ids of DIRECTORY-048 R6) in one line each, marking each read, write or not served through MCP; with a verb, answers its request and response schema. lys_read calls a verb the document marks as a read, whatever its method, so DIRECTORY-048 R5's batch check and its which route (reads sent by POST) are lys_read verbs. lys_write calls a verb the document marks as a write. The mark is the document's own: an x-lys-effect member on each operation with one of three values, read, write or destructive, and an x-lys-secret member on any response member that carries a secret. The effect is declared once per route, beside the route in the route table in routes.rs, as a required argument of the function that registers a route, so a route with no declared effect does not compile; openapi.rs copies it into the document. Nothing defaults to read. lys_read serves read verbs; lys_write serves write and destructive verbs. Verbs come from the generated OpenAPI document, so a new route is reachable with no MCP change. Each tool call is dispatched in-process into the router; there is no second implementation of any verb. Dispatch carries the caller as a VerifiedCaller value: a type whose only constructors are the MCP edge's verification of a person's token or an agent's signature (R2) and the HTTP edge's own verification, with no public constructor and no header, extension or query member that can produce one. This is a refactor of how every handler learns its caller, and it is part of this brief. Today each handler verifies inside itself from its own headers (signed_in(..) in any form, 52 calls in 22 files, and the agent signature checks in link_audit_api.rs, routes.rs and runtime_api.rs). That verification moves into one axum extractor, VerifiedCaller, which every handler takes as an argument. The HTTP edge builds it from the request's headers with exactly today's checks; the MCP edge builds it once from the /mcp request; a handler never reads identity from headers itself. signed_in's definition (routes.rs line 252) moves into verified_caller.rs. A caller that is optional today (start.rs line 333 takes signed_in(..).ok()) takes Option<VerifiedCaller> from the same extractor, never a header read. A helper that verifies from headers today (agent_sight.rs seen_agent) takes the VerifiedCaller from its handler instead. An ast-grep rule, rules/ast-grep/no-handler-verification.yml, fails the build on any call of signed_in or any agent-signature check outside verified_caller.rs, which is the one file it allows. Tools carry MCP annotations: lys_read readOnlyHint, lys_write destructiveHint. A verb whose route holds the call open until something happens (DIRECTORY-050's wait route) stays open as one tool call; the caller ends it by closing the call or the session, as 050 says for the route, and nothing in MCP ends it by a clock.

**Acceptance:**
- tools/list answers exactly three tools.
- Every OpenAPI operation is reachable through lys_read, lys_write or listed as not served through MCP, walked by a test.
- The batch check and the which route of DIRECTORY-048 R5 are reached through lys_read, and carry readOnlyHint.
- A route added in a test router is reachable through MCP with no MCP code change.
- An agent-signed route reached through MCP by a verified agent applies; the same route reached over HTTP with a forged caller header and no signature is refused, and no header produces a VerifiedCaller (a compile-fail test on constructing one outside the edges).
- A call to the wait route through MCP ends when the client closes it, and the server holds nothing for it afterwards.
- A test walks every operation in the document and fails on any operation without x-lys-effect; a route registered without an effect does not compile (a compile-fail test).
- The no-handler-verification rule reports zero findings on the tree and one on a planted handler that reads its caller from headers.

**Files:**
- create: crates/lys-identity-server/src/mcp.rs
- create: crates/lys-identity-server/src/mcp_tools.rs
- create: crates/lys-identity-server/tests/mcp.rs
- create: crates/lys-identity-server/src/verified_caller.rs
- create: crates/lys-identity-server/tests/verified_caller.rs
- create: rules/ast-grep/no-handler-verification.yml
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/Cargo.toml
- modify: crates/lys-identity-server/src/openapi.rs
- modify: crates/lys-identity-server/src/agent_signature.rs
- modify: crates/lys-identity-server/src/certificates_issue.rs
- modify: crates/lys-identity-server/src/configuration_api.rs
- modify: crates/lys-identity-server/src/connections_api.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/launch_api.rs
- modify: crates/lys-identity-server/src/link_audit_api.rs
- modify: crates/lys-identity-server/src/network_api.rs
- modify: crates/lys-identity-server/src/provisioning_api.rs
- modify: crates/lys-identity-server/src/read_api.rs
- modify: crates/lys-identity-server/src/reviews_api.rs
- modify: crates/lys-identity-server/src/runtime_api.rs
- modify: crates/lys-identity-server/src/service_accounts_api.rs
- modify: crates/lys-identity-server/src/setup.rs
- modify: crates/lys-identity-server/src/sign_in_providers.rs
- modify: crates/lys-identity-server/src/stop_api.rs
- modify: crates/lys-identity-server/src/teams_api.rs
- modify: sgconfig.yml
- modify: crates/lys-identity-server/src/agent_sight.rs
- modify: crates/lys-identity-server/src/roles_api.rs
- modify: crates/lys-identity-server/src/secrets_api.rs
- modify: crates/lys-identity-server/src/sessions_api.rs
- modify: crates/lys-identity-server/src/start.rs

**Checklist:**
- C369 — The MCP server offers exactly three tools over the published API; the tool list is under 2 KB (DIRECTORY-049 R1, R5).

**Stories:**
- S153 (Person or agent using Lys through an AI assistant, Reads and changes Lys through an MCP client, with only their own rights) — As a person or an agent using an AI assistant, I want the assistant to reach Lys with my own identity and nothing more, so that it can never see a secret or do what I could not.

### R2: Every call is the caller's own: a person's Lys token or an agent's signature

Behavioural. /mcp speaks streamable HTTP and follows the MCP authorization specification with Lys as the authorization server: it answers 401 with the protected-resource metadata at /.well-known/oauth-protected-resource; a client signs the person in on Lys's own pages with authorization code and PKCE; the token is bound to the /mcp audience and is refused anywhere else, and /mcp refuses any token not issued for it (no token passthrough). An agent's call carries lys-agent-signature (agent_signature.rs) over the MCP request itself (method, path /mcp and the body's digest), checked once at the MCP edge with the existing nonce and certificate checks; the result is the VerifiedCaller that R1 hands to the route. The GET that opens the streamable HTTP event stream is signed and checked when it opens; the stream carries that caller for its life and ends when the certificate is withdrawn. MCP clients such as Claude Code are apps registered and approved through DIRECTORY-048: a client with no approved registration is refused app_not_approved, and dynamic registration answers with a pending app for an administrator to approve, never a working client. A registered loopback redirect (127.0.0.1 or [::1]) matches on any port, as RFC 8252 requires; every other redirect matches exactly. There is no API key, shared secret, static token or unauthenticated mode, and loopback is not trust. The Origin header is checked against Lys's own origin and registered clients, and the listener binds 127.0.0.1 unless configured.

**Acceptance:**
- An unauthenticated call is answered 401 with the metadata address.
- A token issued for another audience is refused token_wrong_audience.
- A replayed agent signature is refused by the existing nonce check.
- A request with a foreign Origin is refused origin_refused.
- The event-stream GET without a valid signature is refused at open; withdrawing the certificate ends an open stream.
- A client not approved through DIRECTORY-048 is refused app_not_approved; after approval it signs in.
- A loopback redirect registered as http://127.0.0.1/callback is accepted at any port; a non-loopback redirect differing only in port is refused.

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

Behavioural. No MCP answer carries a secret's value, a private key, a token or a password: secret verbs answer the handle and its metadata only. A verb whose response schema carries a member marked secret (x-lys-secret; for example DIRECTORY-048's approval that shows an app's client secret once) is listed by lys_schema as not served through MCP and is refused by name, mcp_secret_answer_not_served, pointing to the Lys screen that serves it. A test walks every response schema in the OpenAPI document and fails if any verb served through MCP has a member marked secret. A verb whose x-lys-effect is destructive (at least: retire an identity, revoke a grant, stop, delete a secret, retire an app) is refused confirm_required unless the call carries confirm equal to the target's id; the refusal says in words what the write would do.

**Acceptance:**
- Reading a secret through MCP answers its handle and never its value.
- Approving an app through MCP is refused mcp_secret_answer_not_served and lys_schema lists it as not served.
- The response-schema walk fails on a planted secret field in a served verb.
- A revoke without confirm is refused confirm_required naming the grant; with it, it applies.
- Each of the five named destructive verbs carries x-lys-effect destructive in the document and is refused confirm_required without confirm, one test each.

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

Behavioural. 'lys mcp --agent ID' serves MCP over stdio and forwards each call to the server as a request signed for the agent. It never holds the agent's certificate key. For each call it sends POST /_lys/signature to lys-secrets (SECRETS-006 R2, SECRETS-006.md line 73) with the agent's key handle (R8), the purpose agent_request and the request's members as SECRETS-006 spells them. The members are method, path, body_digest, signed_at_ms and nonce. The member body_digest is the SHA-256 of the request body as 64 lower-case hex characters. The member nonce is at least 16 bytes as hex. The broker builds the bytes to sign itself with lys_core::agent_request::payload and answers a COSE_Sign1 as hex. 'lys mcp' puts that hex as the fourth word of the lys-agent-signature header, the form agent_signature.rs lines 5 to 8 describe and the server verifies. No digest or bytes composed by 'lys mcp' are ever signed. The signature request is itself a handle route, so 'lys mcp' takes its lys-handle and presentation headers from one answer of the runner's present act through crates/lys-runner/src/present_client.rs (DIRECTORY-060 R3), reached at the socket the environment names as LYS_RUNNER_SOCKET. 'lys mcp' holds no holder key, reads no handle token from the environment, and keeps no token past the one request. Nothing is written to a file, an argument or the environment. The launch template (launch_template.rs) renders it into an agent's MCP configuration as 'identity (this service)' when its profile asks for it.

**Acceptance:**
- An agent launched from a profile asking for it lists the three tools and reads its own identity.
- Each call 'lys mcp' forwards carries a lys-agent-signature header whose fourth word is the COSE_Sign1 hex the broker answered, and the server accepts it.
- Each POST /_lys/signature 'lys mcp' sends carries the lys-handle and presentation headers of one present answer from the runner, and the broker admits it.
- 'lys mcp' holds no Ed25519Identity and reads no key file and no handle token.
- The broker's answer to 'lys mcp' holds a signature and a public key and no private key.
- A test on the 'lys mcp' process's memory-held values and its arguments finds no bytes of the agent's certificate key.
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

### R8: The agent's key handle is derived for its session's key at start

Behavioural. When a profile asks for 'identity (this service)', the launch record names the agent's certificate key handle among the agent's held handles, as a HandleName with its id, its secret and its env member LYS_AGENT_KEY_HANDLE (launch_template.rs lines 24 to 31). The start of DIRECTORY-060 R1 derives a session handle under that held handle through POST /_lys/handles/issue (SECRETS-008 R1), with under set to the held handle's id and holder_key set to the session's public key, on behalf of the person starting the agent, before anything is spawned. The derived handle takes the uses, the end and the spend left on the held handle, and ends when the held handle is dropped. The launch template is rendered with the derived id in place of the held id, as 060 R1 says, so LYS_AGENT_KEY_HANDLE carries the derived id. The session's environment carries that derived id and LYS_RUNNER_SOCKET, and no token and no key. 'lys mcp' reads the handle id from LYS_AGENT_KEY_HANDLE and refuses agent_key_handle_missing by name at start when it is absent. A refused derive ends the start by name, as 060 R1 says, and nothing is spawned.

**Acceptance:**
- A session started from a profile asking for 'identity (this service)' has LYS_AGENT_KEY_HANDLE set to a handle id, and a present for it from inside the session is admitted by the broker.
- The session's environment holds no handle token and no key bytes, checked by a test.
- 'lys mcp' started without LYS_AGENT_KEY_HANDLE refuses agent_key_handle_missing.
- A profile that does not ask for it launches with no agent key handle.
- The parent of the handle LYS_AGENT_KEY_HANDLE names is the agent's held certificate key handle.
- LYS_AGENT_KEY_HANDLE never holds the held handle's own id.
- The derived handle's end equals the held handle's end.
- Dropping the held certificate key handle makes the next present for the derived handle refused by the broker.

**Files:**
- modify: crates/lys-identity-server/src/launch_template.rs
- modify: crates/lys/src/cli/mcp.rs

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
- SHALL NOT read a private key out of lys-secrets; signing is a use the broker admits (SECRETS-006).
- SHALL NOT let any header, extension or query member produce a verified caller.
- SHALL NOT let a handler read its caller from headers; every handler takes VerifiedCaller.
- SHALL NOT default an operation's effect; an undeclared effect is a build failure.

## Verification

- The full Lys gate, ast-grep scan and the surface checks exit 0 at the card's head, measured by the card round.
- A real MCP client (Claude Code) connects to a scratch install, signs in through Lys's page, lists three tools and reads its own identity.
