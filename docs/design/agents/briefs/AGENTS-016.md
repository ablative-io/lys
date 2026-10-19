---
type: brief
id: AGENTS-016
cluster: agents
title: The computer Lys runs on is a device its agents start on, an agent's file shows its connection lines, and an agent-made agent exists before its rights, gets its first certificate from its creator, and is seen and stoppable by its person
---

# AGENTS-016: The computer Lys runs on is a device its agents start on, an agent's file shows its connection lines, and an agent-made agent exists before its rights, gets its first certificate from its creator, and is seen and stoppable by its person

> **Cluster:** agents
> **Depends on:** AGENTS-015
> **Design anchor:**
> - ADR-125 — An agent session's holder key is held by the runner, and a session proves itself by peer credentials and ancestry — The runner makes one holder key per session in memory. A start becomes key, then issue, then spawn. The harness and lys mcp ask the runner for single presentations over its socket. The session is proved by peer credentials plus the process ancestry reaching the session's own root pid, checked on every connection, on macOS and Linux both, through a named maintained dependency and with no unsafe in Lys code.
> - ADR-137 — Argus's remaining functions move into Lys as data beside goals, and a seat is a Lys record launched by the runner through the proxy; no shared library — Words (five slots, four layers, templates), variables (per agent and session, revision, author, expiry), schedules and seats are identity-server records in Lys's log, rendered and acted on at delivery through DIRECTORY-064's dispatcher with the contributing revisions in the receipt. No shared library: Lodestone is a read-only query engine over a store and holds nothing; haematite is the store under Lys already; a library would be a second place for the same data. Seat start, stop and restart are deliberate acts under Lys rights (ADR-136). Argus's alarms, rules, launches and queue are not moved into Lys: they become liminal services and aion work (ADR-136 section 3). A seat moves off herdr and Argus one at a time, Waffles's first.
> **Checklist:**
> - C783 — On the computer that runs Lys, a launch matches the machine whose runner is the install's own through the runner key the view carries, and every advice sentence in launch_device names only labels the Computers page renders.
> - C784 — An agent's file shows the complete claude mcp add and codex mcp add lines for this install and agent with a copy act each, and says which origin works now and which waits on AGENTS-015 R3.
> - C775 — An agent that registers an agent holds, under the person's agent.create grant, exactly the rights to set its profile, start, start-command, stop and restart it, withdrawn when that grant is revoked; the agent is registered and activated before its grants are appended, and the grant append is idempotent on retry, so a crash never leaves rights on an agent that does not exist (AGENTS-015 R2, ordered by AGENTS-016 R3).
> - C785 — An agent holding agent.create issues the first certificate of an agent whose record names it as registrar, and only while that agent holds none; a second certificate, one for an agent it did not register, one after the person revoked its agent.create, and every withdrawal are refused by name (certificate_not_first, agent_not_yours, agent_create_revoked, not_permitted), and the certificate acts stay withheld from grants.
> - C786 — GET /dashboard lists every agent an agent registered and every start an agent made for the signed-in person, and the dashboard shows them under one heading in plain words with Stop beside each, posting the agent's stop as the agent file does; no lines, no heading.
> **Stories:**
> - S448 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person on the computer that runs Lys, I start my agent here like on any other computer, and I copy the line that connects Claude Code or Codex to it from the agent's own page instead of assembling it from help text.
> - S449 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person whose agent registers agents for me, I want a crash in the middle to leave at most an agent with no rights that the next try finishes, never rights on an agent that does not exist, so that nothing invisible accumulates in my name.
> - S450 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person whose agent sets up an assistant for me, I want it to be able to start that assistant without me at a screen, and I want to see on my dashboard every agent it registered or started, with Stop beside each, so that nothing acts in my name that I cannot see and stop.

## Purpose

On 19 October 2026 Tom could not start an agent on the Mac that runs Lys: launch_device.rs:163 matches a device only to a dialled runner, so the host, whose machine names the install's own runner, is told it has not joined and to use a Connect button the Computers page does not show. Connecting Claude Code or Codex to an agent through lys mcp means typing the claude mcp add or codex mcp add line by hand from the bridge's --help, with the install's address, the agent's id and the key file's path, and nothing in Lys shows that line. This brief makes the host a device like any other, replaces advice that names labels the screens do not show, and puts the two exact connection lines on the agent's file, copyable, saying plainly which form works today and which waits on AGENTS-015 R3. The AGENTS-015 lane (46080233) left three things open, decided here by Waffles on 19 October: the registration and its grants cannot be one cross-log write, so their order is fixed with the agent first; an agent-made agent would wait on a person for its first certificate, the very wait Tom asked to end, so its creator issues the first one and only the first; and nothing an agent does for a person may be invisible to that person, so every agent-made registration and start is on the dashboard with Stop beside it.

## Task

Make launch_device match the host's machine through the install's own runner key carried on the runner view, and make every advice sentence name only labels the surface renders; add a Connect section to the agent's file with the exact Claude Code and Codex lines for this install and agent, copyable. Order the registration before the grants with an idempotent retry; admit the creator to issue the first certificate only; show the person every agent-made registration and start on the dashboard with Stop beside each.

## Requirements

### R1: The computer Lys runs on is a device its agents start on

Behavioural. crates/lys/src/commands/launch_device.rs:163 matches this device to a machine only when the runner view names a runner of kind dialled whose key is this computer's runner-machine key, so on the computer that runs Lys itself, whose machine names the install's own runner (RunnerRecord::Lys, network_store.rs:473), every launch ends machine_not_joined with "use Network > Computer > Connect", naming a button the Computers page does not have. From this brief the runner view GET /network/machines/{id}/runner (runner_api.rs:131, openapi_runner_types.rs) carries, for a runner of kind lys, the install's runner-server public key as hex (the service key the server loads at routes_startup.rs:95, the same key install.rs:261-262 writes to run/runner-server.pub), and launch_device accepts a record of kind lys when that key equals the hex in this device's run/runner-server.pub under its install root (Layout::run_dir): the device that holds the install is the install's machine. A record of kind lys whose key is not this device's is not a match, exactly as a dialled record with another key is not. Every advice sentence in launch_device.rs names only what the screens show: the Computers page (railItems.ts:22) and its Connect group for a computer (JoinCode.tsx:78 and :182, aria-label 'Connect <name>'); "Network > Computer > Connect" and any other words naming a label the surface does not render are replaced. The two labels live once, in a new surface/identity/src/features/network/labels.ts (COMPUTERS, and connectLabel(name) for 'Connect <name>'), which railItems.ts:22 and JoinCode.tsx:78 and :182 import instead of their literal strings; launch_device_tests.rs reads that file with include_str! by its path from the crate and asserts each advice sentence's label is a string exported there, so a renamed label fails the Rust test.

**Acceptance:**
- Red first, in crates/lys/src/commands/launch_device_tests.rs: a runner view of kind lys with this device's run/runner-server.pub key matches the machine and launch proceeds; a kind lys view with another key, or no run/runner-server.pub on the device, is machine_not_joined; a dialled view matches exactly as today
- In the same file: every refusal's advice names only labels exported by surface/identity/src/features/network/labels.ts, read with include_str!; railItems.ts and JoinCode.tsx import their labels from it, and surface/identity typecheck, biome and vitest stay green
- In crates/lys-identity-server/tests/runner_join.rs: the runner view for a machine whose runner is RunnerRecord::Lys carries the server's runner key hex; for a dialled runner it carries the dialled key as today
- gate -stages nextest -crates lys,lys-identity-server -test 'package(lys) | package(lys-identity-server)' green, counts in the report

**Files:**
- create: surface/identity/src/features/network/labels.ts
- modify: crates/lys/src/commands/launch_device.rs
- modify: crates/lys/src/commands/launch_device_tests.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: crates/lys-identity-server/src/openapi_runner_types.rs
- modify: crates/lys-identity-server/tests/runner_join.rs
- modify: surface/identity/src/shell/railItems.ts
- modify: surface/identity/src/features/network/JoinCode.tsx

**Checklist:**
- C783 — On the computer that runs Lys, a launch matches the machine whose runner is the install's own through the runner key the view carries, and every advice sentence in launch_device names only labels the Computers page renders.

**Stories:**
- S448 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person on the computer that runs Lys, I start my agent here like on any other computer, and I copy the line that connects Claude Code or Codex to it from the agent's own page instead of assembling it from help text.

### R2: An agent's file shows the exact lines that connect Claude Code and Codex to it through lys mcp, copyable

Behavioural. An agent's file (surface/identity/src/features/file, tabs.ts, sections.tsx) gains a Connect section under Credentials, surface/identity/src/features/file/AgentConnect.tsx, that shows two complete lines built from this install and this agent, each with a copy act (the shell's RecordCopy or JoinCode.tsx:73's copy Act, the same words): for Claude Code, claude mcp add --transport stdio lys -- lys mcp --url <origin>/api/mcp --agent <id> --key-file <path to this agent's key file on that computer>; for Codex, codex mcp add lys -- lys mcp --url <origin>/api/mcp --agent <id> --key-file <the same>. <origin> is the page's own origin (window.location.origin, as AddMachine.tsx and Network.tsx read it) and <id> the agent's id; the key file placeholder is shown as written, with one sentence saying it is the PEM or seat file holding the key certified for this agent on the Certificates tab. Under the lines, one sentence says what works today: with an http loopback origin the line works now; with any other origin the line needs the https bridge of AGENTS-015 R3, named, and nothing else on the section changes between the two. Nothing is issued, posted or stored by this section: it renders strings from the page's origin and the agent's record. The section is absent on a person's file.

**Acceptance:**
- Red first, in a new surface/identity/src/features/file/agent-connect.test.tsx: for an agent, both lines render with the page's origin, /api/mcp and the agent's id, and the copy act puts the exact line on the clipboard (the test's clipboard stub reads it back); the sentence under them names AGENTS-015 R3 for an https origin and says the line works now for http://127.0.0.1
- In the same file: a person's file renders no Connect section
- surface/identity typecheck and biome 0; vitest counts in the report

**Files:**
- create: surface/identity/src/features/file/AgentConnect.tsx
- create: surface/identity/src/features/file/agent-connect.test.tsx
- modify: surface/identity/src/features/file/sections.tsx
- modify: surface/identity/src/features/file/tabs.ts

**Checklist:**
- C784 — An agent's file shows the complete claude mcp add and codex mcp add lines for this install and agent with a copy act each, and says which origin works now and which waits on AGENTS-015 R3.

**Stories:**
- S448 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person on the computer that runs Lys, I start my agent here like on any other computer, and I copy the line that connects Claude Code or Codex to it from the agent's own page instead of assembling it from help text.

### R3: The new agent exists before its rights do: register and activate first, then the five grants, idempotent on retry

Behavioural. AGENTS-015 R2 asked for the registration and its five grants in one durable operation; at 46080233 they are two appends to two logs (the grant-log append holding the five grants, then the directory append RegisterAgent and Activate, crates/lys-identity-server/src/routes/identities_agent.rs registered and crates/lys-identity-server/src/agent_acts.rs), and no file-log pair gives a cross-log atomic write. The order is decided instead: the directory append (RegisterAgent and Activate, one flush) happens first, and only then the grant-log append of the five grants from the responsible person to the creator on the new agent (one flush). A crash between the two leaves an agent with no rights, which is visible on the person's dashboard (R5), harmless, and finished by the creator's retry; the opposite order would leave rights on an agent that does not exist, invisible and never vacuumed. The grant append is idempotent on retry: the retry of a registration whose agent already exists and answers to the same person (the existing first-receipt branch at identities_agent.rs:103-112) appends only the grants that are not already standing on the new agent from that person to that creator, so a completed registration retried appends nothing and a half-done one appends exactly the missing grants. Both appends keep AGENTS-015's actor and responsible on every record.

**Acceptance:**
- Red first, in crates/lys-identity-server/tests/agent_starts_agent.rs: a crash injected after the directory append and before the grant append (the crate's fault injection the AGENTS-015 lane named in its handback) leaves the new agent registered and active with no grants, and GET /grants for the person lists none on it
- In the same file: the creator's retry of the same registration then completes it, and GET /grants lists exactly five grants on the new agent from the person to the creator, no duplicates; a second retry appends nothing (the grant log's head is unchanged, read through the receipt DIRECTORY-107 keeps)
- In the same file: the order is proven by the fault injection: no run ever shows grants on an agent the directory does not hold
- gate -stages nextest -crates lys-identity-server -test 'package(lys-identity-server)' green, counts in the report

**Files:**
- modify: crates/lys-identity-server/src/routes/identities_agent.rs
- modify: crates/lys-identity-server/src/agent_acts.rs
- modify: crates/lys-identity/src/grants/created.rs
- modify: crates/lys-identity-server/tests/agent_starts_agent.rs

**Checklist:**
- C775 — An agent that registers an agent holds, under the person's agent.create grant, exactly the rights to set its profile, start, start-command, stop and restart it, withdrawn when that grant is revoked; the agent is registered and activated before its grants are appended, and the grant append is idempotent on retry, so a crash never leaves rights on an agent that does not exist (AGENTS-015 R2, ordered by AGENTS-016 R3).

**Stories:**
- S449 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person whose agent registers agents for me, I want a crash in the middle to leave at most an agent with no rights that the next try finishes, never rights on an agent that does not exist, so that nothing invisible accumulates in my name.

### R4: The creator issues the new agent's first certificate, and only the first

Behavioural. POST /agents/{id}/certificates (crates/lys-identity-server/src/certificates_issue.rs:188 issue; the table's row at openapi_table.rs:386 admits a person's app pass or sign-in) also admits an agent's signed call when all of these hold: the calling agent holds a live agent.create grant from the person the target agent answers to; the target agent's record names the calling agent as the one that registered it (AgentRecord::registered_by, crates/lys-identity/src/projection.rs:52 and :95, the actor binding of its RegisterAgent event); and the target agent holds no certificate, issued or withdrawn, in the certificates store. The agent.create grant is read live, at the moment of certification, never from the registration: a registrar whose agent.create from that person has since been revoked (or that holds none) is refused agent_create_revoked, so an agent whose right to create was taken away can never finish minting an identity it began. Otherwise the call is refused by name: agent_not_yours when the record's registered_by is not the caller, certificate_not_first when the target already has any certificate. Every later certificate and every withdrawal (POST /agents/{id}/certificates/{serial}/withdrawal) stays a person's: agent.certificate.issue and agent.certificate.withdraw stay in WITHHELD_FROM_AGENTS, and this admission is read from the registration record, not from a grant, so no grant can widen it. The certificate issued this way is recorded with the creator as actor and the person as responsible, exactly as AGENTS-015 records every agent act, and it is no wider than what the creator holds: the new agent's rights are only what the creator passes on, narrowed. Reason (Waffles, 19 October 2026): without this, every agent-made agent waits on a person at a screen, which is the thing Tom asked to end.

**Acceptance:**
- Red first, in crates/lys-identity-server/tests/agent_starts_agent.rs: an agent that registered an agent issues its first certificate through POST /agents/{id}/certificates with its own signature, the certificate is recorded with the creator as actor and the person as responsible, and the new agent's signed call is then admitted
- In the same file: the same creator asking a second certificate is refused certificate_not_first; after the person withdraws the first, the creator asking again is refused certificate_not_first; a withdrawal by an agent is refused not_permitted as today
- In the same file: another agent of the same person, holding agent.create but not the registrar of this agent, is refused agent_not_yours; the creator asking for an agent it did not register is refused agent_not_yours
- In the same file: the creator registers an agent, the person revokes the creator's agent.create (POST /grants/{id}/revoke), and the creator's first-certificate call is refused agent_create_revoked; the new agent still holds no certificate afterwards, and a person can still issue it
- In crates/lys-identity-server/tests/agent_roots.rs: WITHHELD_FROM_AGENTS still names agent.certificate.issue and agent.certificate.withdraw

**Files:**
- modify: crates/lys-identity-server/src/certificates_issue.rs
- modify: crates/lys-identity-server/src/certificates_api.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/error_names.rs
- modify: crates/lys-identity-server/tests/agent_starts_agent.rs
- modify: crates/lys-identity-server/tests/agent_roots.rs

**Checklist:**
- C785 — An agent holding agent.create issues the first certificate of an agent whose record names it as registrar, and only while that agent holds none; a second certificate, one for an agent it did not register, one after the person revoked its agent.create, and every withdrawal are refused by name (certificate_not_first, agent_not_yours, agent_create_revoked, not_permitted), and the certificate acts stay withheld from grants.

**Stories:**
- S450 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person whose agent sets up an assistant for me, I want it to be able to start that assistant without me at a screen, and I want to see on my dashboard every agent it registered or started, with Stop beside each, so that nothing acts in my name that I cannot see and stop.

### R5: The person sees every agent an agent registered and every start an agent made, with Stop beside each

Behavioural. GET /dashboard (crates/lys-identity-server/src/dashboard_api.rs; openapi_table.rs:154) answers, beside the caller's agents, a list acted_by_agents of two kinds of line for the signed-in person: every agent that answers to the person whose RegisterAgent actor was an agent (registered_by names an agent), as {kind: registered, acting_agent, agent, at}; and every start of one of the person's agents whose actor was an agent (the launch records AGENTS-015 R1 writes with the agent as actor), as {kind: started, acting_agent, agent, at, session}. The dashboard (surface/identity/src/features/dashboard/Dashboard.tsx and Widgets.tsx) draws them as one list under the heading Acted for you by your agents, each line in plain words naming the acting agent, the new or started agent and when, with a Stop act beside each that posts POST /agents/{id}/stop for the new or started agent exactly as the agent file's Stop does (features/file/EmergencyStop.tsx), with the same confirmation and the same receipt words; a person with no such line sees no heading. The list is newest first and is read from the records already written (no new store, no new log): the directory projection for registrations and the runtime store's launch records for starts. Reason: nothing acts on a person's records without the person able to see it and stop it (the AGENTS-015 lane's own hospital line).

**Acceptance:**
- Red first, in a new crates/lys-identity-server/tests/dashboard_acted_by_agents.rs against the crate's server fixture: after an agent registers an agent and starts it (AGENTS-015 R1 and R2, R3 above), GET /dashboard for the person lists one registered line and one started line naming the acting agent, the agent, and when; a person whose agents were all registered and started by people gets an empty list
- Red first, in a new surface/identity/src/features/dashboard/dashboard-acted.test.tsx: the two lines render under the heading in plain words with a Stop act each; pressing Stop posts POST /agents/{id}/stop for that agent after the same confirmation EmergencyStop uses, and the receipt words appear; an empty list renders no heading
- surface/identity typecheck and vitest counts in the report; gate -stages nextest -crates lys-identity-server -test 'package(lys-identity-server)' green

**Files:**
- create: crates/lys-identity-server/tests/dashboard_acted_by_agents.rs
- create: surface/identity/src/features/dashboard/dashboard-acted.test.tsx
- create: surface/identity/src/features/dashboard/ActedByAgents.tsx
- modify: crates/lys-identity-server/src/dashboard_api.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: surface/identity/src/features/dashboard/Dashboard.tsx
- modify: surface/identity/src/features/dashboard/contract.ts

**Checklist:**
- C786 — GET /dashboard lists every agent an agent registered and every start an agent made for the signed-in person, and the dashboard shows them under one heading in plain words with Stop beside each, posting the agent's stop as the agent file does; no lines, no heading.

**Stories:**
- S450 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person whose agent sets up an assistant for me, I want it to be able to start that assistant without me at a screen, and I want to see on my dashboard every agent it registered or started, with Stop beside each, so that nothing acts in my name that I cannot see and stop.

## Boundaries

- No join, grant or key is created by R1 and R2: the host is matched by a key the install already holds, and the Connect section renders strings only; R4's first certificate is the one certificate an agent may issue, read from the registration record and never from a grant
- No new store or log: R5 reads the directory projection and the launch records already written
- No advice sentence names a label the surface does not render
- No fixed wait, timer, poll or deadline anywhere, in code or test
- Nothing installed by the lane; every file touched stays under 500 code lines (ADR-111, by scripts/file-length.sh)

## Verification

- gate -stages nextest -crates lys,lys-identity,lys-identity-server -test 'package(lys) | package(lys-identity) | package(lys-identity-server)'
- surface/identity: bun run typecheck, biome check, vitest (agent-connect.test.tsx and dashboard-acted.test.tsx counts in the report)
- gate -stages clippy, then the whole declared gate
- python3 scripts/design/validate.py docs/design/agents and python3 scripts/design/check-coverage.py docs/design/agents read for FAIL lines

## Amendments

### Amendment 1: R3, R4 and R5 added: ordering closes C775; the creator issues the first certificate; the person sees and stops every agent-made registration and start

- **Date:** 2026-10-19
- **By:** Waffles (the decisions) and Flapjack (the text), on the AGENTS-015 handback at 46080233

Ordering, not one operation: register and activate first, then the five grants, idempotent on retry; a crash between leaves an agent with no rights, visible and finished by the retry, never rights with no agent. An agent holding agent.create issues the first certificate of an agent it registered itself, only while that agent holds no certificate; every later certificate and every withdrawal stays a person's; refusals certificate_not_first and agent_not_yours. Every agent an agent registers and every start an agent makes shows on the person's dashboard naming the acting agent, the agent and when, with Stop beside each. Built as one lane with R1 and R2, pushing as it lands; the install walk after END: issue the root, then an agent registers, certifies and starts an agent on the Mac, read on the dashboard. Added by Waffles the same afternoon: the registrar must still hold a live agent.create from that person at the moment it certifies; if revoked in the meantime the first certificate is refused agent_create_revoked.
