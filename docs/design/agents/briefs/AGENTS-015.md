---
type: brief
id: AGENTS-015
cluster: agents
title: An agent sets up and starts another agent under the grants a person gave it, from any device
---

# AGENTS-015: An agent sets up and starts another agent under the grants a person gave it, from any device

> **Cluster:** agents
> **Depends on:** AGENTS-004
> **Design anchor:**
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> - ADR-125 — An agent session's holder key is held by the runner, and a session proves itself by peer credentials and ancestry — The runner makes one holder key per session in memory. A start becomes key, then issue, then spawn. The harness and lys mcp ask the runner for single presentations over its socket. The session is proved by peer credentials plus the process ancestry reaching the session's own root pid, checked on every connection, on macOS and Linux both, through a named maintained dependency and with no unsafe in Lys code.
> - ADR-137 — Argus's remaining functions move into Lys as data beside goals, and a seat is a Lys record launched by the runner through the proxy; no shared library — Words (five slots, four layers, templates), variables (per agent and session, revision, author, expiry), schedules and seats are identity-server records in Lys's log, rendered and acted on at delivery through DIRECTORY-064's dispatcher with the contributing revisions in the receipt. No shared library: Lodestone is a read-only query engine over a store and holds nothing; haematite is the store under Lys already; a library would be a second place for the same data. Seat start, stop and restart are deliberate acts under Lys rights (ADR-136). Argus's alarms, rules, launches and queue are not moved into Lys: they become liminal services and aion work (ADR-136 section 3). A seat moves off herdr and Argus one at a time, Waffles's first.
> **Checklist:**
> - C774 — The agent, seat, machine-runner and grant pass-on routes admit an agent's signed call held to its own live grants, refused not_permitted by name otherwise; an agent's new agent answers to the agent's person; no agent approves a profile version it set.
> - C775 — An agent that registers an agent holds, under the person's agent.create grant and in the same durable operation, exactly the rights to set its profile, start, start-command, stop and restart it, withdrawn when that grant is revoked.
> - C776 — lys mcp reaches Lys's MCP over verified https from any device, http only on loopback, and says a connection or TLS failure by name.
> - C782 — The agent file's Give form offers 'Create agents for me', 'Run my agents on this computer' and 'Set up and review this kind of profile' as one choice each in plain words, each posting exactly its grant, absent where the person has nothing to give.
> **Stories:**
> - S444 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person running agents on several computers, I give one of my agents the right to set up and start my assistants once, and it does so on any of my computers without me at a screen, every act on the record as the agent's and every right one I can take back.
> - S447 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person who trusts one of my agents to run my assistants, I give it the right to create agents for me, run them on my computer and set up their profiles with three plain choices on its file, once, instead of approving six steps for every assistant.

## Purpose

On 19 October 2026 Tom could not get liminal's assistant or haematite's running. Every step it needed (register the agent, set its profile, start it, name a machine's runner) is a route only a person's session or the operator may call, a service install keeps no operator token, and no seat may hold Tom's sign-in. So every assistant waited on his hands at a screen, on one machine. The grant model already names every one of those acts (agent.create, agent.provisioning.set, agent.start and the rest), and agents already call Lys's MCP with their own signature through lys mcp, but the routes do not admit an agent and the bridge reaches Lys only on loopback. This brief lets a person give an agent those rights once, as grants, and lets that agent do the acts itself, from any device, with every act recorded as the agent's and every right standing in the person's chain.

## Task

Admit an agent's signed call on the agent, seat and machine-runner routes, held to its own grants and refused by name otherwise; give an agent that creates an agent exactly the rights to set it up and start it, under the person's chain, in one durable operation; keep review apart from the one who posted; and let lys mcp reach Lys over verified https from any device.

## Requirements

### R1: An agent's signed call may set up, review, start and stop agents and seats, held to its own live grants

Behavioural. These routes admit an agent's signed request beside a person's session and the operator, the door set G that GET and POST /agents/{id}/variables already use (openapi_table.rs): POST /agents (agent.create), POST /agents/{id}/provisioning (agent.provisioning.set), POST /agents/{id}/provisioning/{version}/review and /seat-program-review (agent.provisioning.review), POST /agents/{id}/start (agent.start), POST /agents/{id}/start-command (agent.start-command), POST /agents/{id}/restart (agent.restart), POST /agents/{id}/stop (agent.stop), POST /seats and POST /seats/{name}/version, /start, /stop and /restart (their seat actions as the table's scope names them; where a seat route names no action, the lane names the shipped action it checks in the report), POST /network/machines/{id}/runner (machine.runner.set), POST /network/machines/{id}/join-code and POST /network/machines/{id}/installer/{kind} (machine.runner.set, so an agent holding it may connect a computer for its person), POST /network/machines/{id}/agents (machine.agent.set: per machine, granted by that machine's owner; network_api/writes.rs:156 today admits only a person), POST /grants (grant.delegate: an agent passes on part of a grant it holds, narrowing only, within that grant's own pass-on rules exactly as a person does today; POST /grants/roots and /grants/agent-roots stay a person's, since a root has no holder above it), and POST /grants/{id}/revoke (grant.revoke) only for a grant the calling agent itself passed on. WITHHELD_FROM_AGENTS (crates/lys-identity/src/grants/shipped.rs:108-135) loses agent.provisioning.set, agent.provisioning.review, machine.agent.set, grant.delegate and grant.revoke; agent.certificate.issue and .withdraw, agent.policy.set, budget.set, configuration.zone.set, grant, identity.transition, the person.* acts, request.approve and .decline, the role.* acts, runtime.stop-everything and the secret acts stay withheld. When the caller is an agent, the handler admits the call only where that agent's own live grants reach the route's action on the route's resource, by the same grant check the variables route makes, and refuses it by name otherwise (not_permitted, naming the action and the resource, never the grants it could not see). An agent never acts as a person: every record the call writes names the agent as the actor and the person it answers to as responsible; an agent's POST /agents registers a new agent that answers to the person the calling agent answers to, never to the calling agent, and registers it active, with no separate transition; an agent may not register itself again. agent.provisioning.review is refused to an agent on a version that same agent set (refusal provisioning_review_own_version): the one who posts a profile never approves it. Start admission (crates/lys-identity/src/start/authority.rs:6-10, read by start_directory.rs:73-75) admits a caller holding a live agent.start grant on that agent beside the responsible person and the administrator; start_right_missing stays the refusal for everyone else. A person's session and the operator are judged exactly as today. The budget gate on start (START_BUDGET) and every refusal the routes name today apply to an agent caller unchanged.

**Acceptance:**
- Red first, in a new crates/lys-identity-server/tests/agent_starts_agent.rs, against the crate's own server fixture: an agent holding agent.create from its person registers a new agent, which answers to that person; holding agent.provisioning.set on it, posts its profile; a second agent holding agent.provisioning.review approves that version, and the first agent's own review of it is refused provisioning_review_own_version; holding agent.start, starts it, and the start record names the calling agent as actor and the person as responsible
- In the same file: each route above, called by an agent holding no grant on the resource, is refused not_permitted naming the action; a person's session gets today's answer on every one of them
- In the same file: an agent holding a grant it may pass on gives part of it to another agent through POST /grants, the new grant in that grant's chain; an agent asking to pass on more than it holds, or a grant marked use-only, is refused by the route's own refusals; an agent calling POST /grants/roots is refused
- In the same file: an agent holding machine.agent.set on a machine from that machine's owner allows its person's agent on it through POST /network/machines/{id}/agents, and the record names the agent as actor; an agent holding machine.runner.set draws a join code and an installer for an own computer; each is refused not_permitted without the grant
- In the same file: an agent revokes a grant it passed on through POST /grants/{id}/revoke, and is refused revoking a grant it did not pass on; the start of an agent by a caller holding only agent.start on it is admitted by start/authority.rs, and a caller holding nothing is refused start_right_missing as today
- In crates/lys-identity-server/tests/agent_roots.rs, which already reads WITHHELD_FROM_AGENTS: the list names exactly the acts this requirement keeps withheld, and none of the five it releases
- gate -stages nextest -crates lys-identity-server -test 'package(lys-identity-server)' green, counts in the report

**Files:**
- create: crates/lys-identity-server/tests/agent_starts_agent.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/signed_first.rs
- modify: crates/lys-identity-server/src/routes/identities.rs
- modify: crates/lys-identity-server/src/provisioning_api.rs
- modify: crates/lys-identity-server/src/start.rs
- modify: crates/lys-identity-server/src/seats_api.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity/src/grants/shipped.rs
- modify: crates/lys-identity/src/start/authority.rs
- modify: crates/lys-identity-server/src/start_directory.rs
- modify: crates/lys-identity-server/src/network_api/writes.rs
- modify: crates/lys-identity-server/tests/agent_roots.rs

**Checklist:**
- C774 — The agent, seat, machine-runner and grant pass-on routes admit an agent's signed call held to its own live grants, refused not_permitted by name otherwise; an agent's new agent answers to the agent's person; no agent approves a profile version it set.

**Stories:**
- S444 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person running agents on several computers, I give one of my agents the right to set up and start my assistants once, and it does so on any of my computers without me at a screen, every act on the record as the agent's and every right one I can take back.

### R2: An agent that creates an agent holds exactly what it needs to set it up and start it, under the person's chain

Behavioural. When an agent registers a new agent under agent.create (R1), the server records, in the same operation as the registration, grants from the responsible person to the creating agent on the new agent of exactly agent.provisioning.set, agent.start, agent.start-command, agent.stop and agent.restart, each one relation carrying one action as the grant model already expresses it (crates/lys-identity/src/grants/shipped.rs; no new action is added). They stand in the chain of the person's grant of agent.create to the creating agent: revoking that grant withdraws them, and GET /grants for the person shows them under it. The creating agent holds no other right on the new agent and none of the new agent's own rights (an app's store, a channel): those are the person's to give, by grant or by approving the agent's drafts. One registration writes the agent and its grants in one durable operation with one flush (Tom's storage rule, 3 October 2026), so a crash between them leaves neither.

**Acceptance:**
- Red first, in tests/agent_starts_agent.rs: after an agent registers an agent, GET /grants for the person lists the five grants to the creator on the new agent under the agent.create grant, and the creator's what-can-I-do over MCP names exactly those five actions on it
- Revoking the person's agent.create grant to the creator withdraws the five, and the creator's next start of the new agent is refused not_permitted
- A crash injected between the registration and its grants, with the crate's existing fault injection, leaves neither the agent nor the grants

**Files:**
- modify: crates/lys-identity-server/src/routes/identities.rs

**Checklist:**
- C775 — An agent that registers an agent holds, under the person's agent.create grant and in the same durable operation, exactly the rights to set its profile, start, start-command, stop and restart it, withdrawn when that grant is revoked.

**Stories:**
- S444 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person running agents on several computers, I give one of my agents the right to set up and start my assistants once, and it does so on any of my computers without me at a screen, every act on the record as the agent's and every right one I can take back.

### R3: lys mcp reaches Lys over https from any device

Behavioural. lys mcp (crates/lys/src/commands/mcp_bridge.rs) accepts an https:// URL to Lys's MCP route on any host, verifying the server's certificate against the system's trust roots with the TLS stack the workspace already carries (read the workspace Cargo.toml and use what is there; no new TLS crate if one is present), and an http:// URL only on a loopback address, as today. There is no flag that turns verification off. Each message is signed exactly as today and the answer written a line on stdout; a TLS or connection failure is said on stderr by name and answered to the MCP client as a JSON-RPC error for that message, never a silent drop. The refusal for a plain http:// address off loopback says to use https. The command's help says the form a seat uses from any device: lys mcp --url https://<lys host>/api/mcp --agent <id> --key-file <seat file>.

**Acceptance:**
- Red first, in crates/lys/src/commands/mcp_bridge_tests.rs: an https URL is accepted by the URL check; an http URL off loopback is refused with words naming https; an http loopback URL is accepted as today
- An integration test serves Lys's MCP route over TLS with a test certificate the test trusts through the same trust path the bridge uses, and a tools/list through the bridge answers; a server presenting an untrusted certificate is refused by name
- gate -stages nextest -crates lys -test 'package(lys)' green, counts in the report

**Files:**
- modify: crates/lys/src/commands/mcp_bridge.rs
- modify: crates/lys/src/commands/mcp_bridge_tests.rs
- modify: crates/lys/src/identity/loopback_http.rs

**Checklist:**
- C776 — lys mcp reaches Lys's MCP over verified https from any device, http only on loopback, and says a connection or TLS failure by name.

**Stories:**
- S444 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person running agents on several computers, I give one of my agents the right to set up and start my assistants once, and it does so on any of my computers without me at a screen, every act on the record as the agent's and every right one I can take back.

### R4: The agent's file offers the three standing grants in plain words, one choice each

Behavioural. On an agent's file, the Give form (surface/identity/src/features/file/GiveAccess.tsx) offers, above the resource and action pickers, three standing grants a person gives once, each one choice with a plain sentence and nothing to configure: "Create agents for me" (agent.create on the person, which R2 then extends to each new agent's profile, start, start-command, stop and restart), "Run my agents on this computer" (machine.agent.set on one of the person's computers, picked by name from GET /network; absent when the person owns none, with that said), and "Set up and review this kind of profile" (agent.provisioning.set and agent.provisioning.review on the person's agents, with the words that this agent's own versions are reviewed by someone else). Choosing one posts the grant through POST /grants exactly as the pickers do today, and the receipt line names it in the same words. The form says, once, why these are one choice each: six approvals per agent is security too awkward to use, and the standing grant is the consent, every act under it on the record as the agent's. The pickers below stay as they are. If starting an agent needs a certificate the new agent does not hold, the lane says so in its report and stops there; it never issues one (agent.certificate.issue stays a person's).

**Acceptance:**
- Red first, in a new surface/identity/src/features/file/give-standing.test.tsx: the three choices render with their sentences for an agent the signed-in person answers for; choosing each posts one POST /grants with exactly the actions named (read from the request body in the test), and the receipt line repeats the sentence
- In the same file: with no owned computer, the second choice is absent and the words say so; for an agent the person does not answer for, none of the three renders
- surface/identity typecheck and biome 0; vitest counts in the report

**Files:**
- create: surface/identity/src/features/file/give-standing.test.tsx
- modify: surface/identity/src/features/file/GiveAccess.tsx

**Checklist:**
- C782 — The agent file's Give form offers 'Create agents for me', 'Run my agents on this computer' and 'Set up and review this kind of profile' as one choice each in plain words, each posting exactly its grant, absent where the person has nothing to give.

**Stories:**
- S447 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person who trusts one of my agents to run my assistants, I give it the right to create agents for me, run them on my computer and set up their profiles with three plain choices on its file, once, instead of approving six steps for every assistant.

## Boundaries

- No person's credential is held, copied or used by any agent: an agent acts only by its own signature and its own grants
- No new action, relation kind or door type: the shipped actions and the existing G door set are used as they are; off WITHHELD_FROM_AGENTS go agent.provisioning.set, agent.provisioning.review (never on a version the same agent set), machine.agent.set (per machine, from that machine's owner), grant.delegate (narrowing only, within pass-on rules) and grant.revoke (only a grant the agent itself passed on); certificate issue and withdrawal, roots, policy, secrets, the person acts, roles, budgets and stop-everything stay withheld
- An agent never approves a profile version it set, and never acts as the person it answers to
- TLS is always verified; no option turns verification off, in code, config or test
- No timer, no fixed wait, no poll and no deadline anywhere, in code or test
- Nothing installed by the lane; every file touched stays under 500 code lines (ADR-111, by scripts/file-length.sh)

## Verification

- gate -stages nextest -crates lys-identity-server -test 'package(lys-identity-server)'
- gate -stages nextest -crates lys -test 'package(lys)'
- surface/identity: bun run typecheck, biome check, vitest (give-standing.test.tsx counts in the report)
- gate -stages clippy, then the whole declared gate
- python3 scripts/design/validate.py docs/design/agents and python3 scripts/design/check-coverage.py docs/design/agents read for FAIL lines

## Amendments

### Amendment 1: Which acts leave WITHHELD_FROM_AGENTS; start admission by grant; the agent file's standing grants

- **Date:** 2026-10-19
- **By:** Waffles (the decisions) and Flapjack (the text), under Tom's direction of 12:04 that Lys is fixed first

Six approvals per agent is security too awkward to use, and people route around security they cannot use; the person's standing grant is the consent, and every act under it is on the record as the agent's. So agent.provisioning.set, agent.provisioning.review (never on the agent's own version), machine.agent.set (per machine, from its owner), grant.delegate (narrowing only) and grant.revoke (own pass-ons only) leave the withheld list; certificate issue and withdrawal, roots, policy, secrets, the person acts, roles, budgets and stop-everything stay. start/authority.rs admits a live agent.start grant on the agent. An agent's POST /agents registers the new agent active, answering to the person. R1 gains POST /network/machines/{id}/agents, /join-code and /installer/{kind}; R4 gives the agent file's Give form the three standing grants, one choice each. If a start needs a certificate for the new agent, the lane says so and stops; it never issues one.
