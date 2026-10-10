---
type: brief
id: AGENTS-006
cluster: agents
title: An agent gets a Lys pass to an approved app with its own grants, by its run pass or a grant credential, and the lys CLI writes it to an owner-only file
---

# AGENTS-006: An agent gets a Lys pass to an approved app with its own grants, by its run pass or a grant credential, and the lys CLI writes it to an owner-only file

> **Cluster:** agents
> **Depends on:** AGENTS-002, ACCESS-002, ACCESS-005
> **Design anchor:**
> - ADR-137 — Argus's remaining functions move into Lys as data beside goals, and a seat is a Lys record launched by the runner through the proxy; no shared library — Words (five slots, four layers, templates), variables (per agent and session, revision, author, expiry), schedules and seats are identity-server records in Lys's log, rendered and acted on at delivery through DIRECTORY-064's dispatcher with the contributing revisions in the receipt. No shared library: Lodestone is a read-only query engine over a store and holds nothing; haematite is the store under Lys already; a library would be a second place for the same data. Seat start, stop and restart are deliberate acts under Lys rights (ADR-136). Argus's alarms, rules, launches and queue are not moved into Lys: they become liminal services and aion work (ADR-136 section 3). A seat moves off herdr and Argus one at a time, Waffles's first.
> **Checklist:**
> - C735 — An agent asks POST /agents/{id}/pass with its run pass or a grant credential for a live grant it holds, and is answered the rights_claim pass for an approved app carrying only its own grants, with no refresh token; every other ask is refused by name and each issue is logged (AGENTS-006 R1, R2, R4).
> - C736 — `lys agent pass` reads the grant credential from an owner-only file, asks over loopback with no operator token, and writes the pass to an owner-only file, printing neither (AGENTS-006 R3).
> **Stories:**
> - S419 (Tom, owner) — As the owner, I want an agent to use an app's store with its own grants, so that I can hand it work without lending it my sign-in.

## Purpose

An approved app (a database service, say) accepts only a Lys pass: a JWT minted by Lys's provider, verified offline against /oauth/jwks with the app as audience, carrying the holder's rights on the app's kinds (provider/rights_claim.rs pass, ACCESS-002). People get one through /oauth/token (authorization_code, refresh_token). Machines get one at POST /runner/dial/{machine}/pass, signed with their join key (machine_pass.rs, provider/machine_issue.rs, ACCESS-005). An agent has no way to get one: its run pass (agent_pass.rs, header lys-agent-pass) is Lys's own and opaque, and a grant credential (POST /grants/{id}/tokens, header lys-grant-token) is not a pass. Tom, 10 October 2026: an agent, including a seat Lys did not start, reads and writes an app's store with its own grants, so its owner hands it work without lending it his sign-in. This brief gives an agent the same pass a person or a machine gets, asked with a proof it already holds, and a CLI command that writes it where a tool can present it.

## Task

Add POST /agents/{id}/pass, proved by the agent's run pass or a grant credential for a live grant it holds, answering the rights_claim pass for an approved audience with the agent's own rights; log each issue; refuse by name; add `lys agent pass`; prove it over HTTP and from the CLI.

## Requirements

### R1: An agent asks for its pass to an approved app with its own proof

Behavioural. WHEN a request POST /agents/{id}/pass with the JSON body {"audience": <app id>} carries exactly one proof that it is agent {id}'s own, THE SYSTEM SHALL answer {"pass", "expires_at", "audience"}: the pass is the one rights_claim::pass issues a person or a machine, for that audience, with its holder asserted as `agent` (sub and holder.id the agent, holder.responsible the person who answers for it) and its rights the agent's own live grants on that app's kinds at issue, never the responsible person's; it lives the provider's pass_seconds; no refresh token is issued; it is kept nowhere, as a machine's is. The proof is either (a) a Lys-started run's run pass in lys-agent-pass, judged as every route judges one (agent_pass::holder, its seat's signature included), whose agent is {id}; or (b) a grant credential in lys-grant-token that the person responsible for one of the agent's grants issued through POST /grants/{id}/tokens, known, unrevoked and unexpired, for a grant held by agent {id} and live now. THE SYSTEM SHALL refuse by name: agent_pass_unproven (no proof, a credential that does not admit, a run pass of another agent, or any other credential beside the proof: Authorization, a cookie, lys-agent-signature or lys-operator); agent_not_holder (the credential's grant is held by another identity or is not live); agent_retired (the agent is retired or suspended); audience_not_approved (the audience is not an app approved on the Apps screen); a run pass that does not admit stays AgentPassRefused, before the route. THE SYSTEM SHALL NOT name the credential, the run pass or the pass in any refusal. Asking for a pass exercises no grant: route_actions admits a run pass to this route untouched and the route judges it.

**Acceptance:**
- An active agent holding one grant on fixture_notes.doc 1, with a credential for it, asks with {"audience":"fixture_notes"}: 200; lys-pass verifies the pass against /oauth/jwks for fixture_notes; sub and holder.id are the agent, holder.kind is agent, holder.responsible is its person; rights are exactly [fixture_notes.doc 1, read and write, that grant]; the person's own grant on fixture_notes.doc 2 evaluates Refused; expires_at equals exp and exp minus iat is at most pass_seconds; no refresh_token; the pass is refused for fixture_files with WrongAudience.
- No proof: 401 agent_pass_unproven. A 43-character credential never issued: 401 agent_pass_unproven. The credential with a session cookie: 401 agent_pass_unproven. The agent's credential presented for another agent: 403 agent_not_holder. Audience nobody_registered, and a registered app not approved: 400 audience_not_approved. A body with an unknown member: 400 RequestMalformed. A run pass that does not admit, alone or beside a credential: 401 AgentPassRefused. The agent retired: 403 agent_retired.

**Files:**
- create: crates/lys-identity-server/src/agent_app_pass.rs
- create: crates/lys-identity-server/src/error_app_pass.rs
- create: crates/lys-identity-server/src/provider/agent_issue.rs
- create: crates/lys-identity-server/tests/agent_app_pass.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/route_actions.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/openapi_agents_types.rs
- modify: crates/lys-identity-server/src/error_agents.rs
- modify: crates/lys-identity-server/src/error_names.rs

**Checklist:**
- C735 — An agent asks POST /agents/{id}/pass with its run pass or a grant credential for a live grant it holds, and is answered the rights_claim pass for an approved app carrying only its own grants, with no refresh token; every other ask is refused by name and each issue is logged (AGENTS-006 R1, R2, R4).

**Stories:**
- S419 (Tom, owner) — As the owner, I want an agent to use an app's store with its own grants, so that I can hand it work without lending it my sign-in.

### R2: Each issue is logged, and each ask is judged anew

Behavioural. WHEN a pass is issued THE SYSTEM SHALL write one line to the service's log naming the agent, the audience and the proof used (run_pass, or grant_credential with the grant it was issued for), and SHALL NOT write the credential, the run pass or the pass. THE SYSTEM SHALL judge the proof, the agent's lifecycle and the audience at every ask, so a credential revoked or expired, a grant revoked, an agent retired or an app retired is refused at the next ask.

**Acceptance:**
- The log line for a credential ask reads `lys-identity-server agent pass issued: agent <id> audience <app> proof grant_credential for grant <grant>`, and for a run pass ends `proof run_pass`.
- Ask: 200; revoke the credential through POST /grants/{grant}/tokens/{id}/revoke; ask again: 401 agent_pass_unproven whose reason names GrantTokenRevoked and never the credential.

**Files:**
- modify: crates/lys-identity-server/src/agent_app_pass.rs
- modify: crates/lys-identity-server/tests/agent_app_pass.rs

**Checklist:**
- C735 — An agent asks POST /agents/{id}/pass with its run pass or a grant credential for a live grant it holds, and is answered the rights_claim pass for an approved app carrying only its own grants, with no refresh token; every other ask is refused by name and each issue is logged (AGENTS-006 R1, R2, R4).

**Stories:**
- S419 (Tom, owner) — As the owner, I want an agent to use an app's store with its own grants, so that I can hand it work without lending it my sign-in.

### R3: lys agent pass writes the pass to an owner-only file

Behavioural. `lys agent pass --agent <id> --audience <app> --credential-file <path> --out <path> [--server <base>]` SHALL read the grant credential from an owner-only file, ask the installed identity server over loopback as `lys seat` does (seat_client.rs) but with no operator token (the install's token is not read, so a service install also serves it), carrying the credential in lys-grant-token, and write the answered pass to --out owner-only (mode 0600) and durably. It SHALL print the agent, the audience, expires_at and the file, or one JSON object with --json, and SHALL NOT print the credential or the pass. A missing credential file is refused credential_file_absent and a file others may read by the private files' own refusal, before anything is sent; the server's refusal is shown by its own name and nothing is written.

**Acceptance:**
- Against a loopback fixture answering {pass, expires_at, audience}: the command exits 0, the --out file holds the pass with mode 0600, the request was POST /api/agents/<id>/pass with lys-grant-token: <credential>, no lys-operator header and the body {"audience":"notes"}, and neither the credential nor the pass appears in its output.
- The fixture answering 401 agent_pass_unproven: exit 1 naming agent_pass_unproven and no --out file; an absent credential file: credential_file_absent; a mode 0644 credential file: refused naming chmod go-rwx, with nothing sent.

**Files:**
- create: crates/lys/src/commands/agent_pass.rs
- create: crates/lys/tests/agent_pass.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/main.rs
- modify: crates/lys/src/commands/mod.rs
- modify: crates/lys/src/commands/seat_client.rs

**Checklist:**
- C736 — `lys agent pass` reads the grant credential from an owner-only file, asks over loopback with no operator token, and writes the pass to an owner-only file, printing neither (AGENTS-006 R3).

**Stories:**
- S419 (Tom, owner) — As the owner, I want an agent to use an app's store with its own grants, so that I can hand it work without lending it my sign-in.

### R4: The route and its refusals are declared

Structural. The route has its OpenAPI row at the end of openapi_table.rs (public door, as the machine's pass route, naming every refusal above) and its typed request and answer at the end of openapi_agents_types.rs; the route is merged at the end of routes_table.rs; the refusals are a small enum (error_app_pass.rs) in the pattern of error_seat.rs, landing in ServerError through AgentsError (error_agents.rs, error_names.rs), since error.rs stands at the 500-line gate; error_status.rs and provider/refusal.rs already answer AgentsError whole.

**Acceptance:**
- Each AppPassError keeps its name and status through ServerError: agent_pass_unproven 401, agent_retired 403, audience_not_approved 400, agent_not_holder 403; the OpenAPI document names POST /agents/{id}/pass with AppPassAsked and AgentAppPass.

**Files:**
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/openapi_agents_types.rs
- modify: crates/lys-identity-server/src/error_agents.rs
- modify: crates/lys-identity-server/src/error_names.rs

**Checklist:**
- C735 — An agent asks POST /agents/{id}/pass with its run pass or a grant credential for a live grant it holds, and is answered the rights_claim pass for an approved app carrying only its own grants, with no refresh token; every other ask is refused by name and each issue is logged (AGENTS-006 R1, R2, R4).

**Stories:**
- S419 (Tom, owner) — As the owner, I want an agent to use an app's store with its own grants, so that I can hand it work without lending it my sign-in.

## Boundaries

- No refresh token and no stored pass: an agent asks again, as a machine does.
- Rights are the agent's own grants; nothing of the responsible person's authority is carried, and the grant credential's own resource and action do not narrow or widen the pass.
- No new credential kind and no new sign-in: the proofs are the run pass and the grant credential that already exist.
- No product's name in crates/ or surface/ outside tests (rules/ast-grep/no-app-names.yml).

## Verification

- This handwritten brief passes scripts/design/gate.sh, judged by its parsed failures, never by its exit code.
- Written whole before any build (Tom, 9 October 19:2x); then cargo fmt, clippy pedantic -D warnings, ast-grep, cargo nextest run for lys-identity-server (agent_app_pass, error_app_pass, agent_app_pass unit tests, openapi) and lys (agent_pass, cli), and cargo test --doc, on the venue Tom's rules name.
- Four-heading handback naming every changed file, the exact route, headers, body and answer, and what was not run; file length 500.
