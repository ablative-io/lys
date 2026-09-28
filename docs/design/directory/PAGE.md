# directory — what was asked, what it means, and what was written

## The words, as they were typed

An agent whose person is retired shows as needing a new person, and the four identity states are recorded as authority only.

This card covers the first two rows of section 3 of the identity CONFORMANCE document on lys main 7b536253, Lifecycle, and each row gets at least one acceptance line that tests it on the lifecycle record.
No brief on main makes either row true, and the lifecycle-hand brief on its own branch cites neither row.
This brief reads that brief's lifecycle record rather than keeping its own copy.

The first row is that every agent answers to a person.
When that person is retired, each of their agents is flagged in its lifecycle record as needing a new person, and a test retires a person and reads the flag on every one of their agents.
An agent with no person is refused by name when it is recorded, and a test shows that refusal.

The second row is that the identity states are registered, active, suspended and retired.
A test walks each allowed change of state in the lifecycle record and shows every other change refused by name.
The state is authority only.
A test shows that changing an agent's state starts and stops no process, and that the record says nothing about whether the agent is running.

Giving an agent a new person is out of scope and is its own card.

## What the survey found, and its angles

The words ask for a new directory brief that makes CONFORMANCE rows 3.1 and 3.2 true. Row 3.1: every agent has a responsible person, recording an agent without one is refused by name, and when a person is retired each of their agents is flagged as needing a new person. Row 3.2: an identity has exactly four states, only the allowed transitions are admitted and every other one is refused by name, and the state is authority only, so it neither starts nor stops anything and says nothing about running. The brief must build on the lifecycle record from DIRECTORY-009, which sits unmerged on brief/directory/lifecycle-hand, rather than keep its own copy. Giving an agent a new person is left to another card.

### What the tree holds

- `docs/design/identity/CONFORMANCE.md:36-37` — Rows 3.1 and 3.2 are the behaviours this card makes true. Both are Kind test, Brief 'LIFECYCLE-STATES (to brief: new)', owner Waffles. Build order step 3 (line 112) groups them with 3.3, 3.4 and 5.1 to 5.6.
- `origin/brief/directory/lifecycle-hand @ 28e4603: docs/design/directory/briefs/DIRECTORY-009.json` — The lifecycle-hand brief the words say to read and not copy. Its rows are R1 LIFECYCLE-CONTRACT.md, R2 causes.rs, R3 fold.rs/state.rs, R4 gate.rs, R5 the session gate and R6 the typed read. The lifecycle record is the fold of signed transition records (ADR-027). It never cites CONFORMANCE 3.1/3.2, and has zero hits for 'needing', 'new person', 'running' or 'process'.
- `docs/design/directory/briefs/DIRECTORY-003.json R1 and R5` — R1 records the responsible person in the signed registration event 'for life', says 'No API of this row changes an agent's responsible person', and refuses a registration with no signed-in caller. R5 records the state beside each identity, not enforced, and refuses retired→active, registered→suspended and registered→retired by name. This is where 'an agent with no person is refused when it is recorded' lands. Not landed: execution is null.
- `docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44,71-95` — The source for the four states and the transition table (retire only from active or suspended) and for the responsible human carried 'for life'. Lines 87-89 say retiring the human makes derived grants refuse. Line 23 ('Running executions stop at their next admission') and line 59 ('Executions as a count of running ones') talk about running beside the state.
- `docs/design/directory/DESIGN.md (design.json)` — The cluster this card continues. P3 says registration creates no running state. The Structure table gains this brief's paths. The Non-Goals keep suspension semantics and the grant representation open for Tom.
- `crates/lys-identity/src/lifecycle/ (planned by DIRECTORY-009)` — The code seam for the flag read and the transition-walk tests. It does not exist on main: crates/lys-identity and crates/lys-identity-server are both absent.
- `docs/design/decisions.json` — Main holds ADR-001 to ADR-018. ADR-027 (fold, never set) and ADR-028 (next-check rule) exist only on the lifecycle-hand branch, so this brief cites decisions that are not yet on main.
- `docs/design/directory/checklist.json, stories.json, docs/design/roadmap.json` — This brief gains checklist items, stories and a roadmap row. Main has C1-C30 and RM-001 to RM-016. The branches already claim C36-C41, S16-S18 and RM-017 (DIRECTORY-009), and DIRECTORY-007/011/013 sit on other branches, so ids must avoid collisions.
- `scripts/design/gate.sh, validate.py, check-coverage.py` — The design gate leg this documents-only brief has to pass.

### What was already decided

- ADR-011 — Proposed: four states and five transitions, each a signed audit record. An agent's registering person is its responsible person 'for life'. A retired identity is never reactivated.
- ADR-003 — Everything is pegged to a human authority, and withdrawing the authority stops every grant derived from it.
- ADR-007 — The product never runs an agent, it only gives its start command. A started agent reports back to the sessions screen, so 'running' belongs there and not in the lifecycle record.
- ADR-027 (branch lifecycle-hand only) — The state is folded from signed transition records and never set directly. No API, migration or repair writes a state.
- ADR-028 (branch lifecycle-hand only) — A suspension or retirement takes effect at the next check. Nothing calls Rauthy to revoke a session.
- DIRECTORY-009 R1 (branch) — 'no policy causes a transition in this contract: the only automatic effect a suspension or retirement has on other identities is the grant-check one DIRECTORY-006 R4 enforces'.
- DIRECTORY-003 R1 — The responsible person is recorded at registration. A registration without a signed-in caller is refused. No API changes an agent's responsible person. Registering does not pretend the agent is running.
- DIRECTORY-003 R5 — State is recorded beside each identity and not enforced (CN11). Out-of-table transitions are refused by name and record nothing.
- directory P3 — Registration creates no running state and issues no login, credential or certificate.
- directory Non-Goal (suspension semantics) — What else stops with a suspended identity is OPEN for Tom.
- CONFORMANCE build order step 3 — Lifecycle and the start command (3.1 to 3.4, 5.1 to 5.6) come after directory grants and delegation.

### What was measured

- CONFORMANCE.md length and location of the rows: 118 lines; row 3.1 at line 36, row 3.2 at line 37
- Briefs on main or on any origin brief/directory, conformance branch that cite 3.1/3.2 or 'needing a new person': 0 (checked main plus 8 branches, including lifecycle-hand)
- DIRECTORY-009 on lifecycle-hand: 6 rows, 34 acceptance lines, 18 estimated hours, 244 JSON lines; branch base 1756688 is 4 commits behind main 7b53625
- crates/lys-identity and crates/lys-identity-server on main: 0 files (neither directory exists)
- DIRECTORY-003 execution status on main: null (not landed)
- Decisions on main: 18 (ADR-001 to ADR-018); ADR-027 and ADR-028 exist only on lifecycle-hand
- Directory checklist items on main: 30 (C1 to C30); DIRECTORY-009 claims C36 to C41
- Roadmap rows on main: 11 (last RM-016); lifecycle-hand adds RM-017
- Allowed transitions in the tree's table: 6 edges: none→registered, registered→active, active→suspended, suspended→active, active→retired, suspended→retired
- Mentions of 'needing', 'new person', 'running' or 'process' in DIRECTORY-009.json: 0 each

### What it means for the other projects

- aion — The card runs through the chain (brief_card → sign-off → card_build_v3 → src_pr → src_land). Its inputs name the repository, commit 7b53625 and the card, and it depends on DIRECTORY-009 landing through the same chain.
- cambium — The card lives on Cambium's board. Nothing in Cambium changes: rows 06 and 07 and the participant-id carry-over stay a non-goal.

### The decisions it stands on

- ADR-011 (honour) — This card uses its four states and its transition table unchanged, and adds no fifth state. Tension: 'responsible person for life' against a flag that points to reassignment. That tension goes to the lead and is not superseded here.
- ADR-003 (honour) — Every agent is pegged to a person. Refusing an agent with no person enforces the root.
- ADR-007 (honour) — The product never runs an agent. 'Authority only' is the lifecycle-record side of that rule.
- ADR-027 (honour) — The flag and the state come from the fold and are never set. It is currently decided only on the lifecycle-hand branch.
- ADR-028 (honour) — The effect of retiring someone shows at the next check, and nothing is revoked at once.
-  (new) — If the lead chooses a derived flag, a decision should record that 'needs a new person' is a view computed from the agent's registration and its person's retire record, never a stored or appended value. That is the flag's equivalent of 'provisioned is a view'.

### What it requires

- A test registers a person and at least two agents under them, retires the person, and reads the needs-a-new-person flag as true on every one of those agents and false on an agent of another person. The test asserts the count of agents flagged.
- A test attempts to record an agent with no responsible person and asserts a named refusal with 0 records appended.
- A test walks all 6 allowed transitions on the lifecycle record and asserts each is admitted, then attempts every other pair among none, registered, active, suspended and retired and asserts each is refused by name with the log size unchanged. The test asserts both counts.
- A test drives every allowed transition and asserts that no process was started or stopped across them.
- A test or check shows that the lifecycle record's type carries no running, execution or session field.
- The brief reads DIRECTORY-009's lifecycle record (fold) and adds no state store or copy of the state.
- Each of CONFORMANCE rows 3.1 and 3.2 is cited by at least one acceptance line of the brief.
- The brief passes scripts/design/gate.sh (validate.py and check-coverage.py exit 0).

### What must not change

- No API, record or row changes an agent's responsible person. Reassignment is a separate card.
- No fifth state and no new transition. A retired identity is never reactivated.
- The state is never set directly (ADR-027), and the flag adds no stored copy of lifecycle state.
- Nothing in lifecycle starts, stops or observes a process or session (ADR-007, P3).
- No change to LIFECYCLE-STATES-2026-09-22.md, the IDENTITY-001 files, lys-core, lys-log-store or any shipped wire tag.
- No call to Rauthy. SpiceDB is asked only through the grant leg.
- No wall clock in new code, and test identities only (CN2).

### What we must put in place first

- DIRECTORY-009 (brief/directory/lifecycle-hand) signed off and merged to main, rebased past 7b53625, so that its lifecycle record, ADR-027/028 and ids exist on main to cite.
- DIRECTORY-003 landed, with crates/lys-identity and the transition commit path, before any code row of this brief starts.
- DIRECTORY-009 R3 (the fold) landed before the flag or transition-walk rows can run.

### The risks

- The flag contradicts ADR-011 and DIRECTORY-003 R1 ('for life'). If this is not settled, the brief is refused at sign-off or fixes a flag that can never clear.
- Appending per-agent flag records would breach DIRECTORY-009 R1's rule of no automatic effect on other identities, and would add audit lines nobody decided to write.
- Id collisions: parallel branches already claim DIRECTORY-007/009/011/013, C31-C41, S16-S18, RM-017 and ADR-019 to ADR-028.
- The code rows are blocked behind DIRECTORY-003 (not landed) and DIRECTORY-009 (not on main), so the card may sit signed-off but unbuildable.
- In step 1 every agent's responsible person is the configured administrator (DIRECTORY-003 R1), so retiring the administrator flags every agent and may remove the only actor who can register or transition.
- The table has no registered→retired edge, so a flagged agent still in the registered state can never be retired. That may surprise a reader of 'every other change refused'.
- 'Starts and stops no process' is hard to show by a test alone, and a weak check (no process observed) only agrees with itself unless it is paired with a structural check.

### Still open

- ADR-011 and DIRECTORY-003 R1 say an agent keeps its responsible person for life and no API changes it. Does flagging an agent as 'needing a new person' mean this card revises that rule, or should the flag stand while 'for life' stays in force until the reassignment card? The sentence of the words it stands on: "Giving an agent a new person is out of scope and is its own card.". Why only the lead can settle it: docs/design/decisions.json ADR-011 ('carries it as its responsible person for life') and docs/design/directory/briefs/DIRECTORY-003.json R1 ('No API of this row changes an agent's responsible person') rule out the reassignment the flag points to. A reader would see a flag that the recorded design says can never be cleared.
- Should the flag be computed when the lifecycle record is read (from the agent's registration and its person's retire record, with nothing appended), or should retiring a person append a signed record on each of their agents? The sentence of the words it stands on: "When that person is retired, each of their agents is flagged in its lifecycle record as needing a new person, and a test retires a person and reads the flag on every one of their agents.". Why only the lead can settle it: An appended record puts a new line in each agent's audit history, and DIRECTORY-009 R1 (branch) says retiring someone has no automatic effect on other identities except the grant check. A computed flag leaves the history as it is.
- When an agent is flagged, does its own state stay unchanged (for example still active, with its grants refusing at the check), or does retiring the person also move the agent to another state? The sentence of the words it stands on: "When that person is retired, each of their agents is flagged in its lifecycle record as needing a new person, and a test retires a person and reads the flag on every one of their agents.". Why only the lead can settle it: The words only say the agent is flagged. DIRECTORY-009 R1 forbids automatic transitions, and LIFECYCLE-STATES lines 87-89 say derived grants refuse. Whether the agent shows as active, suspended or retired changes what an operator sees.
- Is 'shows' met by the flag in the lifecycle record and the typed read (DIRECTORY-009 R6), with the step-7 screen left to a later card, or must a screen show it in this card? The sentence of the words it stands on: "An agent whose person is retired shows as needing a new person, and the four identity states are recorded as authority only.". Why only the lead can settle it: No identity screen exists on main (the surface/ folder is absent) and DIRECTORY-009 R6 builds only the typed read. Whether a person sees the flag on a screen at landing is a scope boundary.
- Should an agent whose person is suspended (not retired) also be flagged, or only when the person is retired? The sentence of the words it stands on: "When that person is retired, each of their agents is flagged in its lifecycle record as needing a new person, and a test retires a person and reads the flag on every one of their agents.". Why only the lead can settle it: The words name retirement only, and what stops with a suspended identity is OPEN for Tom (DESIGN.md Non-Goals). A suspended person's agents would or would not show the flag.

### The units beyond the first

- Give an agent whose person is retired a new responsible person — Excluded by the words as its own card, and it needs ADR-011 and DIRECTORY-003 R1's 'for life' rule revised.
- Show the needs-a-new-person flag on the people list and the agent file — The step-7 screen is not built. DIRECTORY-009 R6 only exposes the typed read the screen consumes.
- CONFORMANCE 3.4: emergency stop revokes tokens and asks sessions to end — The next row of section 3, a separate LIFECYCLE (new) brief, and it touches running sessions, which this card keeps out of the record.

### The smallest complete shape

One documents brief in docs/design/directory (JSON plus rendered MD, with design.json, DESIGN.md, checklist, stories and one roadmap row), depending on DIRECTORY-003 and DIRECTORY-009. Its rows: (1) state rows 3.1 and 3.2 against DIRECTORY-009's lifecycle contract; (2) refuse by name an agent recorded without a responsible person, with a test; (3) the needs-a-new-person flag read from the lifecycle record when the agent's person is retired, with a test that retires a person and reads it on every one of their agents; (4) a transition walk over all 6 allowed edges with every other change refused by name; (5) a test that a state change starts and stops no process and that the record has no running field. It passes the design gate and goes through sign-off, with its code rows blocked on DIRECTORY-003 and DIRECTORY-009 R3 landing.

## The roadmap row

- **RM-032** — Flag an agent whose person is retired as needing a new person, and keep the four identity states authority only (feature, idea)
- Summary: Makes CONFORMANCE rows 3.1 and 3.2 true on the lifecycle record the lifecycle-hand brief DIRECTORY-009 (RM-017) defines, reading its fold and typed read and keeping no copy: an agent recorded with no person is refused by name; an agent's read carries needs_a_new_person, worked out on read from its registration and its person's retire record, true on every one of a retired person's agents, never for a suspended person, with nothing appended and each agent's own state unchanged; every change of state is walked, six admitted and fourteen refused by name; and a test shows the state starts and stops no process and says nothing about running. One brief, DIRECTORY-019, of five rows.
- Asked by: tom on 2026-09-27T13:26:00+10:00
- Context: The lifecycle conformance card on Cambium's board, run through the brief_card chain at lys main 7b53625. The survey's five questions were answered by the lead for the identity line and are written into DIRECTORY-019 as settled: the flag stands and ADR-011's responsible person for life stays in force until the reassignment card revises it; the flag is worked out on read with nothing appended (the new decision ADR-056); a flagged agent's own state is unchanged; the flag shows in the lifecycle read and the typed read, the screen being a later card; and only a retired person's agents are flagged, never a suspended person's.
- Quote: An agent whose person is retired shows as needing a new person, and the four identity states are recorded as authority only.

This card covers the first two rows of section 3 of the identity CONFORMANCE document on lys main 7b536253, Lifecycle, and each row gets at least one acceptance line that tests it on the lifecycle record.
No brief on main makes either row true, and the lifecycle-hand brief on its own branch cites neither row.
This brief reads that brief's lifecycle record rather than keeping its own copy.

The first row is that every agent answers to a person.
When that person is retired, each of their agents is flagged in its lifecycle record as needing a new person, and a test retires a person and reads the flag on every one of their agents.
An agent with no person is refused by name when it is recorded, and a test shows that refusal.

The second row is that the identity states are registered, active, suspended and retired.
A test walks each allowed change of state in the lifecycle record and shows every other change refused by name.
The state is authority only.
A test shows that changing an agent's state starts and stops no process, and that the record says nothing about whether the agent is running.

Giving an agent a new person is out of scope and is its own card.
- Cluster: directory; briefs: DIRECTORY-019
- Notes: Its own row rather than riding RM-001, which carries IDENTITY-001's rows; it depends on RM-001 for DIRECTORY-003. Ids under the ruling that a brief's ids are the next after main's highest and every open branch's: ADR-056, C123 to C127 and S52 to S53 (the open branches reach ADR-055, C122 and S51 in this cluster); RM-032 is kept as the draft's row. The brief id DIRECTORY-019 is the next after main's highest and every open branch's (the open branches reach DIRECTORY-018; the lifecycle-hand brief holds DIRECTORY-009 on brief/directory/lifecycle-hand). DIRECTORY-019 depends on DIRECTORY-009, which creates the lifecycle contract, module and typed read its rows modify; RM-017 is named in its blocked_by rather than in this row's depends_on because RM-017 is not yet on main. The flag stands against ADR-011's responsible person for life and DIRECTORY-003 R1's rule that no request changes it; this card revises neither, and clearing the flag needs the reassignment card, which is the one that revises that rule. Further units, not written: Give an agent whose person is retired a new responsible person; Show the needs-a-new-person flag on the people list and the agent file; CONFORMANCE 3.4: emergency stop revokes tokens and asks sessions to end.

## The design

---
type: design
cluster: directory
title: The standalone identity directory, with every grant rooted in a person
---

# The standalone identity directory, with every grant rooted in a person

> **Cluster:** directory

## Intention

An operator installs the identity product without Cambium or Manifold, signs in, links Google and GitHub to one person, registers an agent under a responsible person, and inspects the signed history of every identity change. Cambium later uses this issuer, keeping its participant ids (rows 06 and 07); that direction is recorded here while rows 06 and 07 stay the non-goal recorded below, and this sentence promises neither row.

## Problem

IDENTITY-001 revision 5 is the reviewed plan for this, in the older row form, and it predates Tom's ruling of 22 September 17:15 that every grant is pegged to a human authority, his PostgreSQL ruling of 23 September 14:14, and the working lifecycle states. Its row 02 installs SpiceDB without saying what it enforces. It cannot be dispatched to the design-system loop as it stands.

## Solution

Carry IDENTITY-001's open rows (02, 04, 03, 05) into design-system briefs in this cluster, DIRECTORY-002 to DIRECTORY-005, revised for the grant ruling (ADR-003), the PostgreSQL ruling (ADR-005) and the working lifecycle states (ADR-011, proposed), with the fork (ADR-009) and the product accents (ADR-010) in the project ledger and every decision still open for Tom marked open. The IDENTITY-001 files stay as they are, as the record of revision 5.

## Principles

- **P1** — An enduring identity ID is stable through provider additions, key rotation and later sessions; issuer plus subject identifies an external login; email and display name never establish identity equivalence.
- **P2** — Everything is pegged to a human authority: the responsible person's permissions are the ceiling and an agent holds an explicit subset; exercising an act and delegating it are separate grants; withdrawing the authority stops every grant derived from it.
- **P3** — A person or agent may be registered before any session exists; registration creates no running state and issues no Rauthy login, runtime credential or capability certificate.
- **P4** — One signed committed directory event is both the identity change and its audit record; never a database change followed by a best-effort log append.
- **P5** — No success before durable evidence; an uncertain append is reconciled before its projection answers as current, and pending or refused outcomes stay visible until resolved.
- **P6** — A future execution or fork ID refers to its enduring identity and, for a fork, its parent execution; step 1 reserves that distinction in the contract and implements neither session history nor launch.
- **P7** — An audit receipt carries a version, a stable operation ID, the actor, the affected identity, the operation, a payload commitment and the resulting log coordinate or checkpoint; secrets and whole context objects are excluded, and its exact signed encoding is reviewed before use.
- **P8** — The service attests the authenticated human actor and their authentication provenance; it never claims a person signed bytes with a key they do not hold, and a registration records the person who made it, never an invented agent signature.
- **P9** — The initial directory administrator is bootstrapped by an explicitly configured issuer and subject, never by email or first visitor; every other mutation caller is refused in step 1.

## Decisions

- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
- ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
- ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
- ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
- ADR-056 — Needs a new person is a view worked out on read from an agent's registration and its person's retire record, never stored or appended — An agent's needs-a-new-person flag is worked out each time its lifecycle record is read, from the responsible person its registration names and that person's retire record: it is true when that person's folded state is retired and false otherwise, a suspended person included. Nothing is appended to any agent's history, no transition is made on the agent, the agent keeps its own state, and no responsible person changes. The flag records a fact, that the agent's person is retired; clearing it needs the reassignment card, which is the one that revises ADR-011's responsible person for life. Rejected: a signed flag record appended on each agent when its person is retired; moving the agent to another state with its person.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.
- CONFORMANCE rows 3.1 and 3.2 are true on the lifecycle record, each tested by at least one acceptance line of DIRECTORY-019, reading the lifecycle-hand brief DIRECTORY-009's record and keeping no copy of it.

## Non-Goals

- Rows 06 (connect Cambium) and 07 (gate, install and demonstrate the release) — They change the Cambium repository and depend on IDENTITY-002, the upstream release rebase; they need a Cambium cluster or an agreed cross-repository arrangement first.
- Row 03's changes inside the Rauthy fork — The fork is its own repository under vendor/rauthy; a row whose files live there needs its own brief in the fork, which does not yet exist and blocks DIRECTORY-004. DIRECTORY-004 names the fork files as work with their owner, not as files of this repository.
- The grant representation: how one grant records who may exercise it, whether it may be passed on (person, agent or nobody), what it derives from, and whether it can be bounded. OPEN for Tom. — The statement leaves the exact delegation schema unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003); AGENT-PARITY-2026-09-23's questions (docs/design/identity/AGENT-PARITY-2026-09-23.md:17-23) are inputs to it, not answers.
- Suspension semantics: whether suspending a person also ends their sign-in session at Rauthy or only makes our checks refuse, and what else stops with a suspended identity. OPEN for Tom. — The lifecycle document asks it of the room (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:99-100, docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:104-105) and its states are only proposed (ADR-011); two systems, one decision.
- The name of the identity service. OPEN for Tom. — The statement lists it as not decided (docs/design/identity/STATEMENT-2026-09-22.md:192); no crate or directory name selects it.
- Which anchor the first agents pin to: our hosted anchor, a self-hosted one, or both. OPEN for Tom. — Lys leaves it as a product decision and so does the statement (docs/design/identity/STATEMENT-2026-09-22.md:193).
- A nightly Rauthy base versus waiting for an upstream release carrying #1696 and #1728. OPEN for Tom. — Waffles' ruling of 15:36:25: if upstream has published no such release by the row 05 showing, Tom decides (docs/design/identity/briefs/IDENTITY-001.json:18; docs/design/identity/STATEMENT-2026-09-22.md:177); rows 02 to 05 run on v0.36.2 meanwhile.
- Road step 2 onward: capability certificates, arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working.
- The examples in AGENT-PARITY-2026-09-23 (abilities with an assignment or project, seat provisioning within a budget, private and shared notes) — Tom gave them as not yet decided (docs/design/identity/AGENT-PARITY-2026-09-23.md:11-15); they are never turned into requirements.
- A production Cambium auth cutover, and any upstream Rauthy contribution as a prerequisite — Revision 5 forbids both before scratch acceptance, review and Gypsy's coordinated install (docs/design/identity/briefs/IDENTITY-001.json:31).
- A shared design-system package extracted for every product — Tom left it as a thing to look at, not a row (ADR-010).
- Giving an agent a new person — It is its own card, the one that revises ADR-011's responsible person for life and clears the flag; DIRECTORY-019 only flags that an agent's person is retired (ADR-056).
- A screen that shows the needs-a-new-person flag on the people list and the agent file — It belongs to a later card; DIRECTORY-019 shows the flag in the lifecycle read and the lifecycle-hand brief's typed read (RM-017), which that screen consumes.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/directory/design.json` | the directory design; gains a structure row for every path a row brief names | DIRECTORY-001 |
| `docs/design/directory/DESIGN.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/directory/checklist.json` | the directory checklist; gains the row items | DIRECTORY-001 |
| `docs/design/directory/CHECKLIST.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/directory/stories.json` | the directory stories; gains the row stories | DIRECTORY-001 |
| `docs/design/directory/USER-STORIES.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/decisions.json` | the project decision ledger; gains the identity decisions | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-002.json` | row 02 of IDENTITY-001, converted | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-002.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-003.json` | row 04 of IDENTITY-001, converted | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-003.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-004.json` | row 03 of IDENTITY-001, converted | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-004.md` | rendered markdown | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-005.json` | row 05 of IDENTITY-001, converted | DIRECTORY-001 |
| `docs/design/directory/briefs/DIRECTORY-005.md` | rendered markdown | DIRECTORY-001 |
| `Cargo.toml` | workspace manifest; gains the identity dependencies (DIRECTORY-002) and the directory crates (DIRECTORY-003) | DIRECTORY-002 |
| `Cargo.lock` | lock file; follows Cargo.toml (DIRECTORY-002, DIRECTORY-003) | DIRECTORY-002 |
| `crates/lys/Cargo.toml` | the CLI crate's manifest; gains the identity subcommand's dependencies | DIRECTORY-002 |
| `crates/lys/src/main.rs` | the CLI entry; dispatches lys identity | DIRECTORY-002 |
| `crates/lys/src/cli.rs` | the CLI arguments; gains the identity subcommand | DIRECTORY-002 |
| `crates/lys/src/commands/error.rs` | the CLI error type; carries the identity errors | DIRECTORY-002 |
| `crates/lys/src/identity/mod.rs` | declarations and re-exports only | DIRECTORY-002 |
| `crates/lys/src/identity/cli.rs` | identity subcommand argument declarations | DIRECTORY-002 |
| `crates/lys/src/identity/config.rs` | typed deployment configuration and validation; no secret values in diagnostics | DIRECTORY-002 |
| `crates/lys/src/identity/credentials.rs` | Zeroizing, redacted credential material and stable reuse | DIRECTORY-002 |
| `crates/lys/src/identity/private_files.rs` | restricted-mode durable file creation and outcome reconciliation | DIRECTORY-002 |
| `crates/lys/src/identity/prepare.rs` | validates inputs and materialises the declared private deployment artifacts | DIRECTORY-002 |
| `crates/lys/src/identity/configure.rs` | idempotent client and theme reconciliation with stable operation identifiers | DIRECTORY-002 |
| `crates/lys/src/identity/rauthy.rs` | typed Rauthy API requests and responses, named status errors, read-back after an uncertain outcome | DIRECTORY-002 |
| `crates/lys/src/identity/themes.rs` | reads the declared estate palette mapping and validates both client themes | DIRECTORY-002 |
| `crates/lys/src/identity/health.rs` | named readiness checks for the declared services; SpiceDB readiness only | DIRECTORY-002 |
| `crates/lys/src/identity/error.rs` | typed errors carrying operation, resource and path, never secret bytes | DIRECTORY-002 |
| `crates/lys/tests/identity_deploy.rs` | ID001_DEPLOY | DIRECTORY-002 |
| `crates/lys/tests/identity_refusals.rs` | ID001_DEPLOY_REFUSAL | DIRECTORY-002 |
| `crates/lys/tests/identity_theme.rs` | ID001_THEME | DIRECTORY-002 |
| `crates/lys/tests/identity_shared_db.rs` | ID001_SHARED_DB | DIRECTORY-002 |
| `crates/lys/tests/identity_restart.rs` | restart and restore of the dependencies | DIRECTORY-002 |
| `crates/lys/tests/identity_support/mod.rs` | shared test support for the identity tests | DIRECTORY-002 |
| `crates/lys/tests/identity_support/fixtures.rs` | test identities and configuration fixtures, never real credentials | DIRECTORY-002 |
| `crates/lys/tests/identity_support/server.rs` | test server harness | DIRECTORY-002 |
| `crates/lys/tests/identity_support/compose.rs` | compose harness for the three dependencies | DIRECTORY-002 |
| `deploy/identity/compose.yaml` | Rauthy, SpiceDB and one PostgreSQL service (DIRECTORY-002); gains the directory service (DIRECTORY-003) | DIRECTORY-002 |
| `deploy/identity/versions.json` | pinned releases and image digests | DIRECTORY-002 |
| `deploy/identity/config.example.toml` | example configuration without secrets, the database address included (DIRECTORY-002, DIRECTORY-003) | DIRECTORY-002 |
| `deploy/identity/postgres-init.sql` | roles and schema namespaces for Rauthy and SpiceDB in one database | DIRECTORY-002 |
| `deploy/identity/README.md` | install, readiness, backup and restore, and SpiceDB's step-1 sentence (DIRECTORY-002); the directory (DIRECTORY-003) and the screens (DIRECTORY-005) | DIRECTORY-002 |
| `deploy/identity/rauthy-themes.json` | both Rauthy client themes | DIRECTORY-002 |
| `deploy/identity/theme-map.md` | source tokens and colour conversions of the themes | DIRECTORY-002 |
| `docs/design/identity/reports/IDENTITY-001-deployment.md` | row 02's report: digests, versions, resolved configuration without secrets, restore result | DIRECTORY-002 |
| `crates/lys-identity/` | directory records, typed API and event projection; exact manifest reviewed before the row starts | DIRECTORY-003 |
| `crates/lys-identity-server/` | OIDC session handling, administrator admission and the link-audit receiver (DIRECTORY-003); routes and assets of the screens (DIRECTORY-005) | DIRECTORY-003 |
| `tests/identity_contract/` | the directory's contract tests: ID001_DIRECTORY, ID001_AUDIT_FAULTS, ID001_ADMIN, ID001_RECEIPT, ID001_RECEIVER and the lifecycle transitions | DIRECTORY-003 |
| `docs/design/identity/DIRECTORY-CONTRACT.md` | the directory contract: identifiers, bindings, responsible person, lifecycle state | DIRECTORY-003 |
| `docs/design/identity/IDENTITY-EVENTS.md` | the versioned event envelope, reviewed jointly with Archie | DIRECTORY-003 |
| `vendor/rauthy` | the maintained fork's pin (ADR-009); moves to the gated fork commit that links two providers | DIRECTORY-004 |
| `docs/design/identity/PROVIDER-LINK-CONTRACT.md` | the typed contract between the fork's link audit and the receiver | DIRECTORY-004 |
| `docs/design/identity/reports/IDENTITY-001-links.md` | row 03's report: pinned fork commit, gate result, counted legs | DIRECTORY-004 |
| `surface/identity/` | the standalone screens; exact manifest reviewed before the row starts | DIRECTORY-005 |
| `crates/lys-identity-server/src/assets.rs` | serves the screens' assets | DIRECTORY-005 |
| `crates/lys-identity-server/src/routes.rs` | the screens' routes; DIRECTORY-006 later extends the same route seam after reconciling the dependency-owned manifest | DIRECTORY-005 |
| `docs/design/identity/reports/IDENTITY-001-standalone.md` | row 05's report: screenshots, observed actions, artifact hashes | DIRECTORY-005 |
| `docs/design/directory/briefs/DIRECTORY-006.json` | The human-rooted grants and delegation implementation brief, awaiting reviewed foundation manifests and contract decisions | DIRECTORY-006 |
| `docs/design/directory/briefs/DIRECTORY-006.md` | Rendered grants and delegation brief | DIRECTORY-006 |
| `crates/lys-identity/src/grants/mod.rs` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/types.rs` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/error.rs` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_contract.rs` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `docs/design/identity/GRANT-CONTRACT.md` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/lib.rs` | Define the reviewed grant contract without changing published cryptography; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/admission.rs` | Enforce affirmative delegation and bounded ancestry; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/lineage.rs` | Enforce affirmative delegation and bounded ancestry; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/authority.rs` | Enforce affirmative delegation and bounded ancestry; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_delegation.rs` | Enforce affirmative delegation and bounded ancestry; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/events.rs` | Commit grants and their audit as one replayable operation; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/projection.rs` | Commit grants and their audit as one replayable operation; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/recovery.rs` | Commit grants and their audit as one replayable operation; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_faults.rs` | Commit grants and their audit as one replayable operation; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_receipts.rs` | Commit grants and their audit as one replayable operation; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/revocation.rs` | Enforce revocation, inherited expiry and current permission decisions; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/expiry.rs` | Enforce revocation, inherited expiry and current permission decisions; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/src/grants/permission.rs` | Enforce revocation, inherited expiry and current permission decisions; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_revocation.rs` | Enforce revocation, inherited expiry and current permission decisions; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity/tests/grant_expiry.rs` | Enforce revocation, inherited expiry and current permission decisions; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity-server/src/grants.rs` | Expose one authenticated grant and explanation seam; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity-server/src/grant_contract.rs` | Expose one authenticated grant and explanation seam; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity-server/tests/grants.rs` | Expose one authenticated grant and explanation seam; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `crates/lys-identity-server/tests/grant_explanations.rs` | Expose one authenticated grant and explanation seam; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/src/features/grants/YouGrants.tsx` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/src/features/grants/DelegateGrant.tsx` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/src/features/grants/GrantExplanation.tsx` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/tests/grants.test.tsx` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/tests/acceptance/grants.spec.ts` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/src/routes.tsx` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `surface/identity/src/generated/index.ts` | Implement the You and delegation screens from the server contract; planned grant wall, reconcile dependency-owned integration files before dispatch | DIRECTORY-006 |
| `docs/design/directory/briefs/DIRECTORY-008.json` | the grant brief residue after PR 6: DIRECTORY-005's verification line, three inventory rows and the intention sentence, as requirements on the documents | DIRECTORY-008 |
| `docs/design/directory/briefs/DIRECTORY-008.md` | rendered markdown | DIRECTORY-008 |
| `docs/design/identity/LIFECYCLE-CONTRACT.md` | the lifecycle contract, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 R1 adds its conformance section for rows 3.1 and 3.2 | DIRECTORY-019 |
| `crates/lys-identity/src/lifecycle/mod.rs` | the lifecycle module list, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 R2 adds the responsible module | DIRECTORY-019 |
| `crates/lys-identity/src/lifecycle/error.rs` | the lifecycle refusal names, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 R2 adds lifecycle_agent_without_person | DIRECTORY-019 |
| `crates/lys-identity/src/lifecycle/causes.rs` | who may cause each transition, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 R2 adds one line calling the no-person check | DIRECTORY-019 |
| `crates/lys-identity/tests/lifecycle_support/mod.rs` | the lifecycle test support, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 adds persons with agents to its fixtures | DIRECTORY-019 |
| `crates/lys-identity-server/src/lifecycle_read.rs` | the typed lifecycle read, created by the lifecycle-hand brief DIRECTORY-009 (RM-017); DIRECTORY-019 R3 adds the needs_a_new_person field | DIRECTORY-019 |
| `crates/lys-identity/src/lifecycle/responsible.rs` | the no-person refusal and the needs-a-new-person flag worked out on read (DIRECTORY-019 R2, R3) | DIRECTORY-019 |
| `crates/lys-identity/tests/lifecycle_no_person.rs` | CONFORMANCE 3.1: an agent recorded with no person is refused by name (DIRECTORY-019 R2) | DIRECTORY-019 |
| `crates/lys-identity/tests/lifecycle_needs_new_person.rs` | CONFORMANCE 3.1: retiring a person flags every one of their agents as needing a new person, and suspending one flags none (DIRECTORY-019 R3) | DIRECTORY-019 |
| `crates/lys-identity/tests/lifecycle_walk.rs` | CONFORMANCE 3.2: the walk of all 20 changes of state (DIRECTORY-019 R4) | DIRECTORY-019 |
| `crates/lys-identity-server/tests/lifecycle_authority.rs` | CONFORMANCE 3.2: the state is authority only and the read says nothing about running (DIRECTORY-019 R5) | DIRECTORY-019 |

## Inventory

- `docs/design/identity/briefs/IDENTITY-001.json` — IDENTITY-001 revision 5, the reviewed plan these briefs carry forward: rows, walls, acceptance identifiers (ID001_*), estimates, authority and review decisions. Read, never changed.
- `docs/design/identity/briefs/IDENTITY-001.md` — its rendered twin
- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority, including 'Everything is pegged to a human authority (Tom, 17:15 to 17:16)'
- `docs/design/identity/LIFECYCLE-STATES-2026-09-22.md` — the working lifecycle states: registered, active, suspended, retired
- `docs/design/identity/PROVISIONING-2026-09-22.md` — provisioning an agent under a human, the seven-step path
- `docs/design/identity/AGENT-PARITY-2026-09-23.md` — Tom's 23 September ruling: 'If a human can do it through the UI, then I want an agent to be able to do it'; whether an agent may is a permission question. Input to the directory and permission rows. Its examples are recorded as undecided and are never turned into requirements.
- `docs/design/identity/RAUTHY-BASELINE.md` — the fork baseline: pin, maintenance owner, the IDENTITY-001-UPSTREAM-AUTH-STATE blocker; read, never changed
- `docs/design/identity/CONFORMANCE.md` — Accepted mock-up behaviour map; DIRECTORY-006 maps You/delegation/access requirements and does not claim mock-up sample policy is authoritative.
- `vendor/rauthy` — the maintained Rauthy fork (ADR-009), the git submodule pinned at dd61ac3c84d6b238108dc8438b53043b5177a662, the upstream v0.36.2 commit the ablative branch was created from; DIRECTORY-004 moves the pin (structure row); read here, never changed by a document row
- `crates/lys` — the lys CLI crate: Cargo.toml, src/main.rs, src/cli.rs and src/commands/ (attest, ca, key, log, inspect, files); DIRECTORY-002 adds src/identity/ and the identity subcommand to it (structure rows)
- `docs/design/decisions.json` — the project decision ledger, ADR-001 to ADR-018 at main, holding the decisions this cluster cites (ADR-003, ADR-004, ADR-005, ADR-007 to ADR-011); DIRECTORY-001 recorded that it gained the identity decisions (structure row); read here, never changed by a document row

## Constraints

- **CN1** — Documents only: nothing outside docs/design/directory/ and docs/design/decisions.json is created or modified; the IDENTITY-001 files are not changed.
- **CN2** — Development isolation: rows 02 to 05 use only disposable test identities and test provider registrations; no production tokens, real business sign-in or live Cambium participant migration.
- **CN3** — Every path written in a document of this cluster is relative to the repository root, whatever directory a session starts in; a command runs from its own tree and spells its paths from there.
- **CN4** — No structure row or files entry carries a root token; a file in another repository is named in a requirement's spec with its owner.
- **CN5** — A live demonstration to Tom is never an acceptance criterion of a loop requirement; it is a verification step a person performs after the row lands.
- **CN6** — The live demonstrations ID001_LINK_LIVE (after row 03) and ID001_DIRECTORY_LIVE (after row 05) are mandatory operator hold points: the brief that follows each is blocked by it until Tom's demonstration receipt is recorded; a loop completion never stands in for one.
- **CN7** — Revision 5's ceiling stands: 48 focused implementer hours for IDENTITY-001 (row 01 1.5, closed; 02 8; 04 10; 03 10; 05 6; 06 4; 07 5; total 44.5, contingency 3.5), with IDENTITY-002's 4 hours outside it. An overrun is reported as soon as it is known, and Waffles takes any ceiling change to Tom (docs/design/identity/briefs/IDENTITY-001.json:24).
- **CN8** — One implementer, one row in implementation and one gate invocation at a time in this lane; release builds, checks and tests run through the gate workflow at the venue, and a development exception never bypasses it (docs/design/identity/briefs/IDENTITY-001.json:25-27, docs/design/identity/briefs/IDENTITY-001.json:135).
- **CN9** — A row that needs a file outside its wall stops and names it, and the reviewer approves a brief revision before that file is edited; a directory wall for a wholly new module allows only its named responsibility and needs an exact file manifest reviewed before its row starts (docs/design/identity/briefs/IDENTITY-001.json:28).
- **CN10** — Rows 02 to 05 run on Rauthy v0.36.2 under Waffles' ruling of 15:36:25; each development install checks current releases and advisories and records the accepted exception; IDENTITY-001-UPSTREAM-AUTH-STATE binds real sign-in and install, rows 06 and 07 (docs/design/identity/briefs/IDENTITY-001.json:18, docs/design/identity/briefs/IDENTITY-001.json:46).
- **CN11** — Step 1 is directory records, sign-in and the minimum signed identity audit. SpiceDB is installed and checked in row 02 and enforces nothing in step 1; live capability policy and its enforcement are step 2's, and running SpiceDB is not permission enforcement (docs/design/identity/briefs/IDENTITY-001.json:29-30).
- **CN12** — DIRECTORY-006 is implementation work after the frozen planning task: its source paths are only executable after the DIRECTORY-002/003 foundations are implemented and their integration manifests are reconciled. R6 also waits for the standalone surface foundation. Do not dispatch from a schema-valid but dependency-blocked brief.


---
type: brief
id: DIRECTORY-001
cluster: directory
title: Revise the identity directory plan for human-rooted grants, as design-system briefs
---

# DIRECTORY-001: Revise the identity directory plan for human-rooted grants, as design-system briefs

> **Cluster:** directory
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> **Checklist:**
> - C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
> - C2 — The project decision ledger holds the identity decisions with their authority and quote: the maintained Rauthy fork, one PostgreSQL service and database (ADR-005, already recorded), the product accents, and the lifecycle states as working team decisions marked proposed.
> - C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task.
> - C4 — The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement with acceptance criteria in the row that owns it, and row 02 states what SpiceDB enforces in step 1 and what it does not.
> - C5 — Every decision still open for Tom is recorded as open: the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting for the upstream release.
> - C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.
> **Stories:**
> - S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
> - S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

## Purpose

Rewrite the identity plan so every grant traces back to a person (Tom, 22 September 17:15), with the PostgreSQL decision and the four lifecycle states in, as design-system briefs the loop can build. Documents only; comes before any code.

## Task

Read IDENTITY-001.json revision 5 in full, the statement's section 'Everything is pegged to a human authority', LIFECYCLE-STATES, PROVISIONING, AGENT-PARITY-2026-09-23 (its examples are undecided and are not requirements), and ADR-005 in docs/design/decisions.json, which already records Tom's PostgreSQL ruling. Add the identity decisions to docs/design/decisions.json, complete this cluster's design, checklist and stories, and write DIRECTORY-002 (row 02, 8 hours), DIRECTORY-003 (row 04, 10 hours), DIRECTORY-004 (row 03, 10 hours) and DIRECTORY-005 (row 05, 6 hours), each depending on the one before, revision 5's 48-hour ceiling kept and any re-estimate stated. Carry every ID001 acceptance identifier forward unchanged. Every path in a document of this cluster is relative to the repository root, even though the session starts in docs/.

## Requirements

### R1: Record the identity decisions and complete the directory design

THE SYSTEM SHALL add to docs/design/decisions.json one ADR per identity decision named in C2 that the ledger does not already hold (ADR-005, PostgreSQL, is already there and is cited, not rewritten), each with its authority, date, decider and, where the source holds Tom's own words, the quote; a working team decision SHALL be status proposed, never decided. The directory design SHALL gain those ADR ids and SHALL keep revision 5's outcome, shared contract, constraints and non-goals, revised for the grant ruling. Each ADR's source SHALL be cited from IDENTITY-001's authority or review_decisions, the statement, or the lifecycle document; nothing is invented.

**Acceptance:**
- validate.py reports decisions.json and design.json valid.
- Every ADR added cites its source, and the lifecycle states ADR is status proposed.
- ADR-005 is unchanged and the directory design anchors to it.

**Files:**
- modify: docs/design/decisions.json
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
- C2 — The project decision ledger holds the identity decisions with their authority and quote: the maintained Rauthy fork, one PostgreSQL service and database (ADR-005, already recorded), the product accents, and the lifecycle states as working team decisions marked proposed.

**Stories:**
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (validate.py reports decisions.json and design.json valid): both files were written to the decisions and design schemas. I checked keys and types against docs/design/decisions.json:146-199 and docs/design/directory/design.json with my own code, not validate.py, which the workflow runs. Row 2 (every added ADR cites its source; the lifecycle ADR is proposed): ADR-009 at docs/design/decisions.json:146 cites the statement at :191 and IDENTITY-001 at :10, :17 and :35. ADR-010 at :164 cites the statement at :194 and IDENTITY-001 at :14-15 and :43-44. ADR-011 at :182 cites LIFECYCLE-STATES at :17-44 and :71-95 and has "status": "proposed" (:184). Row 3 (ADR-005 unchanged, design anchors to it): a comparison against HEAD shows the ADR-001 to ADR-008 prefix is identical. ADR-005 stays in docs/design/directory/design.json 'decisions' (:45). Revision 5's content is kept, revised for the grant ruling: the outcome is the intention, the shared contract is P1 to P9 (:7-44), the constraints are CN1 to CN11 (:444) and the non-goals start at :61.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/decisions.json` — 'updated' is now 2026-09-24. Three ADRs added: ADR-009 (the maintained Rauthy fork, decided, Tom with Waffles' branch rule, with Tom's quote), ADR-010 (shared design with each product keeping its own accent, identity orange, decided, Tom's 15:18 quote) and ADR-011 (registered/active/suspended/retired, status proposed, Archie for the working team). Each cites its sources; ADR-001 to ADR-008 are byte-identical.
  - modified: `docs/design/directory/design.json` — Adds principles P6 to P9 from revision 5's shared contract and review decisions, with P3 revised. Decisions now ADR-003, 004, 005, 007, 008, 009, 010, 011. Goals, non-goals (five marked OPEN for Tom), constraints CN7 to CN11 from revision 5's ceiling, a structure row for every listed path, and the RAUTHY-BASELINE inventory entry.
  - modified: `docs/design/directory/DESIGN.md` — Markdown rendered from design.json with the method's render_design.
- Checklist delivery:
  - [x] C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling. — Outcome, principles P1 to P9, CN7 to CN11 and the revised non-goals are in docs/design/directory/design.json.
  - [x] C2 — The project decision ledger holds the identity decisions with their authority and quote: the maintained Rauthy fork, one PostgreSQL service and database (ADR-005, already recorded), the product accents, and the lifecycle states as working team decisions marked proposed. — Fork (ADR-009), PostgreSQL (ADR-005, cited), accents (ADR-010) and lifecycle states (ADR-011, proposed).
- Story delivery:
  - [x] S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it. — P2 and ADR-003 anchor the design. Every agent is registered under its responsible person (DIRECTORY-003 R1).

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] validate.py reports decisions.json and design.json valid. — From docs/: validate.py docs/design/decisions.json -> 'design/decisions.json: OK [decisions.schema.json] All 1 document(s) valid.' (exit 0); validate.py docs/design/directory -> 'design/directory/design.json: OK [design.schema.json]', all 8 valid (exit 0).
  - [x] Every ADR added cites its source, and the lifecycle states ADR is status proposed. — docs/design/decisions.json ADR-009 context cites STATEMENT-2026-09-22.md:191 and IDENTITY-001.json:10, :17, :35, all verified; the quote matches STATEMENT:191 verbatim. ADR-010 cites STATEMENT:194 and IDENTITY-001.json:14-15, :43-44; its hex values match IDENTITY-001.json:44; its quote matches STATEMENT:194 verbatim. ADR-011 cites LIFECYCLE-STATES:1-8, :17-44, :71-95, :101-103, with status 'proposed' and decided_by 'Archie ... not ruled by Tom'.
  - [x] ADR-005 is unchanged and the directory design anchors to it. — A Python comparison of HEAD:docs/design/decisions.json decisions[] against the working tree's first 8 entries -> 'prefix same True'. design.json decisions list contains ADR-005 (DESIGN.md Decisions section).
- Checklist verified: C1, C2
- Stories verified: S1

### R2: Carry rows 02, 04, 03 and 05 into design-system briefs

THE SYSTEM SHALL write DIRECTORY-002 to DIRECTORY-005 from revision 5's rows 02, 04, 03 and 05, each valid against the brief schema: its work as requirements, its wall as files (a file in the Rauthy fork, which is its own repository under vendor/rauthy, is named in the spec with its owner and is not listed in files), its acceptance as criteria keeping every ID001 identifier, its dependencies as depends_on, its estimate in its task. A live demonstration to Tom SHALL be a verification step, never an acceptance criterion. Every path a row brief lists SHALL gain a structure row in design.json. DIRECTORY-004 (row 03) SHALL carry in blocked_by the fork-owned brief that does not yet exist, named as such, so it is never an executable brief with an empty wall. DIRECTORY-005 SHALL carry in blocked_by the ID001_LINK_LIVE demonstration to Tom, and the design's hold point for ID001_DIRECTORY_LIVE SHALL be kept, per CN6.

**Acceptance:**
- DIRECTORY-002 to DIRECTORY-005 exist and validate.py reports each valid.
- Each ID001 acceptance identifier of rows 02, 04, 03 and 05 appears in its brief.
- depends_on runs DIRECTORY-002, then 003, then 004, then 005, matching revision 5's order 02, 04, 03, 05.
- No acceptance criterion requires a live demonstration; each row's demonstration appears under verification.
- check-coverage.py reports every listed path present in the structure.
- DIRECTORY-004's blocked_by names the missing fork-owned brief.
- DIRECTORY-005's blocked_by names the ID001_LINK_LIVE demonstration.

**Files:**
- create: docs/design/directory/briefs/DIRECTORY-002.json
- create: docs/design/directory/briefs/DIRECTORY-002.md
- create: docs/design/directory/briefs/DIRECTORY-003.json
- create: docs/design/directory/briefs/DIRECTORY-003.md
- create: docs/design/directory/briefs/DIRECTORY-004.json
- create: docs/design/directory/briefs/DIRECTORY-004.md
- create: docs/design/directory/briefs/DIRECTORY-005.json
- create: docs/design/directory/briefs/DIRECTORY-005.md
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task.

**Stories:**
- S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (the four briefs exist and are valid): all four follow brief.schema.json's required keys, and my own key check passed. Row 2 (every ID001 identifier kept): a script compared revision 5's acceptance for each row with the briefs and found nothing missing. Row 02 has DEPLOY, DEPLOY_REFUSAL, THEME, SHARED_DB and PIN_CLONE. Row 04 has DIRECTORY, AUDIT_FAULTS, ADMIN, RECEIPT and RECEIVER. Row 03 has LINK_PAIR, LINK_REFUSAL, LINK_MIGRATION, LINK_AUDIT and LINK_LIVE. Row 05 has STANDALONE, SCREEN_REFUSAL and DIRECTORY_LIVE. Row 3 (dependency order): depends_on at line 5 of each brief is DIRECTORY-001 for 002, then 002, 003 and 004 in turn. Row 4 (no live demonstration in acceptance): no acceptance string names a demonstration. ID001_LINK_LIVE and ID001_DIRECTORY_LIVE are the last verification entries of DIRECTORY-004 and DIRECTORY-005. Row 5 (every listed path in the structure): I checked each path, directory prefixes included, against docs/design/directory/design.json:107 onward, with none missing. Row 6: DIRECTORY-004.json:8 blocked_by names 'The fork-owned brief for row 03's Rauthy changes, which does not yet exist'. Row 7: DIRECTORY-005.json:8 blocked_by starts with 'ID001_LINK_LIVE: Tom's live demonstration of row 03'. The ID001_DIRECTORY_LIVE hold point is kept in CN6 (design.json:466), a design goal, and DIRECTORY-005's verification. Estimates are 8, 10, 10 and 6 hours with no re-estimate, stated in each task, and CN7 keeps the 48-hour ceiling.
- Deviation: Four judgement calls. (1) DIRECTORY-002 depends_on DIRECTORY-001, because row 01 is IDENTITY-001's and is already accepted; that acceptance is stated in the task. (2) Revision 5's report names under docs/design/identity/reports/IDENTITY-001-*.md are kept as each row's wall, so they carry forward unchanged. (3) DIRECTORY-003 and DIRECTORY-005 carry in blocked_by the exact file manifest that revision 5 (IDENTITY-001.json:28) requires before a wholly new module's row starts. (4) Revision 5's 'edit main in the main Mac checkout, no worktree' venue rule is not carried as a constraint, because this loop now runs in clones. CN8 keeps the part that still binds: one implementer, one row and one gate at a time, with the gate workflow never bypassed.
- Files changed:
  - created: `docs/design/directory/briefs/DIRECTORY-002.json` — Row 02. R1: packaging and the development install on one PostgreSQL database. R2: the lys identity CLI per the module manifest. R3: the two themes. R4: SpiceDB's step-1 role. 8 hours.
  - created: `docs/design/directory/briefs/DIRECTORY-002.md` — Rendered markdown.
  - created: `docs/design/directory/briefs/DIRECTORY-003.json` — Row 04. R1: registration under a responsible person. R2: signed events and receipts. R3: the administrator. R4: the link-audit receiver. R5: the lifecycle state, with the grant path's row recorded open. 10 hours. blocked_by: the new-module manifest.
  - created: `docs/design/directory/briefs/DIRECTORY-003.md` — Rendered markdown.
  - created: `docs/design/directory/briefs/DIRECTORY-004.json` — Row 03, the lys side. R1: the pin, with fork files named in the spec alongside their owner. R2: the provider-link contract and the report. blocked_by: the fork-owned brief, which does not exist yet. ID001_LINK_LIVE is under verification. 10 hours.
  - created: `docs/design/directory/briefs/DIRECTORY-004.md` — Rendered markdown.
  - created: `docs/design/directory/briefs/DIRECTORY-005.json` — Row 05: the screen journey, visible refusals and appearance. blocked_by: ID001_LINK_LIVE. ID001_DIRECTORY_LIVE is under verification as the hold point. 6 hours.
  - created: `docs/design/directory/briefs/DIRECTORY-005.md` — Rendered markdown.
  - modified: `docs/design/directory/design.json` — A structure row for every path the four briefs list (61 rows, no duplicates).
  - modified: `docs/design/directory/DESIGN.md` — Rendered.
- Checklist delivery:
  - [x] C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task. — DIRECTORY-002 to 005 in order 02, 04, 03, 05, with walls as files, identifiers kept and estimates in the task.
- Story delivery:
  - [x] S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria. — Each row is a brief with numbered requirements and criteria, blocked until Waffles reviews it.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] DIRECTORY-002 to DIRECTORY-005 exist and validate.py reports each valid. — validate.py docs/design/directory: DIRECTORY-002.json to DIRECTORY-005.json each 'OK [brief.schema.json]', exit 0 (re-run after the harden edit).
  - [x] Each ID001 acceptance identifier of rows 02, 04, 03 and 05 appears in its brief. — Running grep -o 'ID001_[A-Z_]*' on each brief gives 002: DEPLOY, DEPLOY_REFUSAL, PIN_CLONE, SHARED_DB, THEME; 003: ADMIN, AUDIT_FAULTS, DIRECTORY, RECEIPT, RECEIVER; 004: LINK_PAIR, LINK_REFUSAL, LINK_MIGRATION, LINK_AUDIT, LINK_LIVE; 005: STANDALONE, SCREEN_REFUSAL, DIRECTORY_LIVE. These equal IDENTITY-001 rows 02/04/03/05 acceptance, and the acceptance texts are verbatim. CAMBIUM_LINK, GOOGLE_SEPARATE and PKCE belong to rows 06 and 07, which are out of scope.
  - [x] depends_on runs DIRECTORY-002, then 003, then 004, then 005, matching revision 5's order 02, 04, 03, 05. — check-coverage.py 'Brief dependencies': 002 depends on 001, 003 on 002, 004 on 003, 005 on 004. 003 is row 04, 004 is row 03 and 005 is row 05, matching revision 5's order 02, 04, 03, 05.
  - [x] No acceptance criterion requires a live demonstration; each row's demonstration appears under verification. — The ID001_LINK_LIVE text appears only in DIRECTORY-004 verification[3] and DIRECTORY-005 blocked_by[0]. ID001_DIRECTORY_LIVE appears only in DIRECTORY-005 verification[3]. No acceptance array names a demonstration.
  - [x] check-coverage.py reports every listed path present in the structure. — check-coverage.py compares every R# files path with the design.json structure (lines 322-336 of the script) and reports 'Coverage clean: all items covered, briefs consistent.', exit 0.
  - [x] DIRECTORY-004's blocked_by names the missing fork-owned brief. — DIRECTORY-004.json blocked_by[0]: 'The fork-owned brief for row 03's Rauthy changes, which does not yet exist: a brief in the ablative-io/rauthy fork's own repository ...'
  - [x] DIRECTORY-005's blocked_by names the ID001_LINK_LIVE demonstration. — DIRECTORY-005.json blocked_by[0]: 'ID001_LINK_LIVE: Tom's live demonstration of row 03 (DIRECTORY-004) ...'. CN6 keeps the ID001_DIRECTORY_LIVE hold point.
- Checklist verified: C3
- Stories verified: S2
- Issues:
  - DIRECTORY-003 R1 spec contradicted itself. It said 'a person is registered by first sign-in' and also 'in step 1 the only caller that may register is the configured administrator'. Under P9 and IDENTITY-001.json:38 every non-admin mutation is refused in step 1, so the first clause would lead an implementer to break ID001_ADMIN.
- Fixes:
  - docs/design/directory/briefs/DIRECTORY-003.json R1 spec: self-registration by first sign-in is now attributed to ADR-011's proposal. The spec states that in step 1 the only caller that may register a person or an agent is the configured administrator (P9, IDENTITY-001.json:38), and that a first sign-in registers nobody. DIRECTORY-003.md was re-rendered with render-cluster.py.

### R3: Put the grant path and the enforcement boundary in the row that owns them

THE SYSTEM SHALL make the grant path named in C4 a requirement, with acceptance criteria, in the row brief the sources place it in, and SHALL state in DIRECTORY-002 what SpiceDB enforces in step 1 and what it does not. Where the sources do not settle which row owns the grant path, the brief SHALL record that as open for Tom and SHALL NOT pick one.

**Acceptance:**
- The grant path appears as one requirement with criteria for allowed-before-revoke and refused-after-revoke, or is recorded as open with the question stated.
- DIRECTORY-002 states SpiceDB's step-1 role in one sentence.

**Files:**
- modify: docs/design/directory/briefs/DIRECTORY-002.json
- modify: docs/design/directory/briefs/DIRECTORY-003.json

**Checklist:**
- C4 — The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement with acceptance criteria in the row that owns it, and row 02 states what SpiceDB enforces in step 1 and what it does not.

**Stories:**
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (grant path as a requirement, or open with the question stated): the sources conflict. Revision 5 keeps arbitrary grants and enforcement in step 2 (IDENTITY-001.json:29-30). Chippy's 17:08 first screen (STATEMENT:183) and PROVISIONING:23-24 put the path in 'steps 1 and 2' without naming a row. So DIRECTORY-003.json:138 (R5 spec) records it as OPEN for Tom and does not pick one, as the spec requires. It asks whether the path lands in DIRECTORY-003, a new row before DIRECTORY-005, or step 2's first brief. It drafts allowed-before-revoke (a), refused-after-revoke (b), refused-after-suspend (c) and audit (d), so nothing is lost. An R5 acceptance criterion requires it to still be open at review. Row 2 (DIRECTORY-002 states SpiceDB's step-1 role in one sentence): DIRECTORY-002.json:139 opens with that sentence: installed, migrated, backed up and health-checked, enforcing nothing, with no decision asked and no grant written, and enforcement left to step 2. It then states what SpiceDB does not do. R4's acceptance requires the sentence verbatim in deploy/identity/README.md.
- Deviation: (none)
- Files changed:
  - created: `docs/design/directory/briefs/DIRECTORY-002.json` — R4 states SpiceDB's step-1 role in one sentence and forbids any check or grant write.
  - created: `docs/design/directory/briefs/DIRECTORY-003.json` — R5 records the grant path's owning row as OPEN for Tom. It states the question and drafts criteria (a) to (d) for whichever row Tom places it in.
- Checklist delivery:
  - [x] C4 — The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement with acceptance criteria in the row that owns it, and row 02 states what SpiceDB enforces in step 1 and what it does not. — Uses the brief's allowed alternative: the grant path's row is recorded open with the question and criteria stated, and row 02 states SpiceDB's step-1 role.
- Story delivery:
  - [x] S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it. — The responsible person is recorded on every agent, and the grant path's criteria are drafted for the row Tom chooses.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] The grant path appears as one requirement with criteria for allowed-before-revoke and refused-after-revoke, or is recorded as open with the question stated. — DIRECTORY-003.json R5 spec records 'OPEN for Tom, not decided here: which row owns the grant path'. It states the question (DIRECTORY-003, a new row before DIRECTORY-005, or step 2's first brief) and the conflicting sources: IDENTITY-001.json:29-30 against STATEMENT:183 and PROVISIONING:23-24, which I verified. It drafts criteria (a) allowed before revoke, (b) refused after revoke, (c) refused after suspend and (d) audit.
  - [x] DIRECTORY-002 states SpiceDB's step-1 role in one sentence. — DIRECTORY-002.json R4 spec opens with a single sentence: 'in step 1 SpiceDB is installed, migrated, backed up and health-checked against the shared PostgreSQL database, and it enforces nothing, because no component asks it for a permission decision or writes a grant to it, and live capability policy and its enforcement stay with road step 2'. Its acceptance requires that sentence word for word in deploy/identity/README.md.
- Checklist verified: C4
- Stories verified: S1

### R4: Mark the open decisions open and keep the cluster covered

THE SYSTEM SHALL record each open decision named in C5 in the design's non-goals or the owning brief as open for Tom, deciding none of them, and SHALL add the rows' checklist items and stories so that every one is named by a row brief and the rendered markdown matches the JSON.

**Acceptance:**
- Each open decision in C5 appears once, marked open.
- check-coverage.py exits 0.
- render-cluster.py leaves the rendered markdown unchanged after the commit.

**Files:**
- modify: docs/design/directory/checklist.json
- modify: docs/design/directory/CHECKLIST.md
- modify: docs/design/directory/stories.json
- modify: docs/design/directory/USER-STORIES.md

**Checklist:**
- C5 — Every decision still open for Tom is recorded as open: the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting for the upstream release.
- C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.

**Stories:**
- S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (each C5 open decision appears once, marked open): docs/design/directory/design.json non_goals (:61) has one entry each, marked 'OPEN for Tom', for the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting. The rendered DESIGN.md has them at lines 57-61, and the phrase occurs nowhere else in the cluster except DIRECTORY-003's grant-path question, which is not a C5 item. Row 2 (check-coverage.py exits 0): my own check found every C1 to C20 and S1 to S7 named by a brief and each brief-level array equal to the union of its requirements. Every design_anchor exists in decisions.json, every file path is in the structure and there are no cycles. I did not run check-coverage.py. Row 3 (render leaves the markdown unchanged): every .md was produced by the method's own render_design, render_checklist, render_stories and render_brief functions from the final JSON. DIRECTORY-001.md was re-rendered byte-identical, so the render leg should find no change.
- Deviation: To produce the markdown I imported render-cluster.py's and render-brief.py's functions in a Python snippet instead of hand-writing it. I did not invoke the render-cluster.py command itself, which is the gate leg.
- Files changed:
  - modified: `docs/design/directory/checklist.json` — Four row sections with C7 to C20, each named by a row brief. C1 to C6 are unchanged.
  - modified: `docs/design/directory/CHECKLIST.md` — Rendered.
  - modified: `docs/design/directory/stories.json` — S4 and S5 added under Responsible person, plus two new personas, Operator (S3, S6) and Verifier (S7). S1 and S2 are unchanged.
  - modified: `docs/design/directory/USER-STORIES.md` — Rendered.
- Checklist delivery:
  - [x] C5 — Every decision still open for Tom is recorded as open: the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting for the upstream release. — Five non-goals, each marked OPEN for Tom once. None is decided.
  - [x] C6 — The rendered markdown of this cluster matches its JSON and coverage is clean. — Markdown was rendered by the method's functions from the final JSON, and my own coverage check is clean. The gate legs will confirm this.
- Story delivery:
  - [x] S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria. — Every checklist item and story is named by a row brief for review.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] Each open decision in C5 appears once, marked open. — design.json non_goals lines 71, 75, 79, 83 and 87 carry one 'OPEN for Tom' entry each for the grant representation, suspension semantics, the service name, the anchor and nightly versus waiting (DESIGN.md:57-61). A grep across the cluster JSON finds no other open-marked record of them; DIRECTORY-003's OPEN is the grant-path row, which is not a C5 item, and DIRECTORY-001's hits are the dev record.
  - [x] check-coverage.py exits 0. — From docs/: check-coverage.py docs/design/directory -> 20 items, 7 stories, 5 briefs, 'Coverage clean', exit 0. Two legal warnings: S1 and S3 are each claimed by more than one brief.
  - [x] render-cluster.py leaves the rendered markdown unchanged after the commit. — After the harden edit and one render, I took md5 of every .md in docs/design/directory and briefs, ran render-cluster.py docs/design/directory again and re-hashed: no difference ('RENDER-STABLE').
- Checklist verified: C5, C6
- Stories verified: S2

## Boundaries

- Documents only: only docs/design/directory/ and docs/design/decisions.json change; the IDENTITY-001 files, and everything else, stay byte for byte.
- Nothing open for Tom is decided.
- Rows 06 and 07 are not written here.
- No credential, token or key value is written.
- No row brief is dispatched until Waffles has reviewed it.

## Verification

- From the repository root: python3 scripts/design/validate.py docs/design/directory exits 0.
- From the repository root: python3 scripts/design/validate.py docs/design/decisions.json exits 0.
- From the repository root: python3 scripts/design/check-coverage.py docs/design/directory exits 0.


---
type: brief
id: DIRECTORY-002
cluster: directory
title: Install the standalone service dependencies on one PostgreSQL database
---

# DIRECTORY-002: Install the standalone service dependencies on one PostgreSQL database

> **Cluster:** directory
> **Depends on:** DIRECTORY-001
> **Blocked by:** Waffles' review of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it), Waffles' re-check of revision 5's row-02 module manifest, which IDENTITY-001 records as due before row 02 source starts (docs/design/identity/briefs/IDENTITY-001.json:20, docs/design/identity/briefs/IDENTITY-001.json:407), The venue gate path ID001_PIN_CLONE needs, which revision 5 records as resting with Heimdall (docs/design/identity/briefs/IDENTITY-001.json:17, docs/design/identity/briefs/IDENTITY-001.json:407); the row's gate is not submitted until it is confirmed
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> **Checklist:**
> - C7 — Rauthy, SpiceDB and one PostgreSQL service and database are packaged and installed for development, with separate roles and schema namespaces, the database address as configuration, and readiness, restart, named refusals, the shared database and restore proved (ID001_DEPLOY, ID001_DEPLOY_REFUSAL, ID001_SHARED_DB, ID001_PIN_CLONE).
> - C8 — lys identity prepare, configure and health exist in the CLI exactly as revision 5's module manifest names them, register the platform and Cambium clients idempotently, and keep every secret out of Git, logs, errors and health output.
> - C9 — Both Rauthy client themes carry Aion's neutral vocabulary with each product's own accent, Cambium green and the identity orange (ID001_THEME).
> - C10 — SpiceDB's step-1 role is written down and held: installed and checked, asked for no decision, holding no grant.
> **Stories:**
> - S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

## Purpose

Row 02 of IDENTITY-001 revision 5: the three dependency processes of the standalone identity product (the maintained Rauthy, ADR-009; SpiceDB; one PostgreSQL service and database, ADR-005) installed for development with durable storage, the lys CLI that prepares, configures and checks them, and both Rauthy client themes (ADR-010). Revised for Tom's PostgreSQL ruling, which lets the database live on a network device so its address is configuration, and for the grant ruling (ADR-003), which makes SpiceDB's step-1 role explicit: installed and checked, enforcing nothing yet.

## Task

Carry IDENTITY-001 revision 5 row 02 (docs/design/identity/briefs/IDENTITY-001.json:89-167) into requirements: packaging and the development install (R1), the lys identity CLI (R2), the two client themes (R3) and SpiceDB's step-1 role (R4). Row 01, the fork and its pin, was accepted by Waffles at 15:36:25 (docs/design/identity/briefs/IDENTITY-001.json:18); its fresh recursive-clone proof is due at this row's venue gate (ID001_PIN_CLONE). Estimate: 8 focused implementer hours, revision 5's figure for this row (docs/design/identity/briefs/IDENTITY-001.json:20); no re-estimate. Revision 5's 48-hour ceiling stands (CN7); an overrun is reported as soon as it is known. Out: the directory itself (DIRECTORY-003), any grant or permission check, and real sign-in (rows 06 and 07). Every path is relative to the repository root; the estate colour tokens live in another repository and are named in R3's spec with their owner.

## Requirements

### R1: Package Rauthy, SpiceDB and one PostgreSQL database, and install them for development

THE SYSTEM SHALL package the maintained Rauthy at the pinned vendor/rauthy commit (ADR-009), SpiceDB, and one PostgreSQL service with one durable database (ADR-005) in deploy/identity/compose.yaml, with pinned supported releases and image digests in deploy/identity/versions.json, separate least-privilege roles and schema namespaces created by deploy/identity/postgres-init.sql through PostgreSQL's own init directory, explicitly configured credentials, and documented local and TLS origins in deploy/identity/config.example.toml (docs/design/identity/briefs/IDENTITY-001.json:136, docs/design/identity/briefs/IDENTITY-001.json:36). The database address SHALL be configuration and SHALL NOT assume Tom's Mac (ADR-005). Both migration runners and their connection search paths SHALL be verified to coexist before one-database readiness is claimed. These are three dependency processes beside the platform door; Cambium, Manifold, Argus and Aion servers are not runtime dependencies (ADR-004). THE SYSTEM SHALL define readiness, migration order, named configuration failures, stop and start, and backup and restore (docs/design/identity/briefs/IDENTITY-001.json:137), documenting in deploy/identity/README.md the cache, key and config files Rauthy's internal Hiqlite cache needs rather than claiming a SQL dump alone backs up the product; no Hiqlite identity datastore is used in the installed configuration (docs/design/identity/briefs/IDENTITY-001.json:36). The development instance is installed on a node the operator names (ADR-005) with test identities only; release builds, checks and tests stay on the gate workflow, and each development install checks upstream releases and advisories and records the accepted v0.36.2 exception (docs/design/identity/briefs/IDENTITY-001.json:135, CN10). Results, digests and the restore outcome go to docs/design/identity/reports/IDENTITY-001-deployment.md, the report name revision 5 gives this row.

**Acceptance:**
- ID001_DEPLOY: from a fresh venue directory, dependencies become ready with no other Ablative service running; restart preserves issuer identity and database contents.
- ID001_DEPLOY_REFUSAL: missing secret, invalid issuer/redirect and unavailable database produce named failures, never a substitute identity or development database.
- Dependency artifact digests, versions, resolved configuration without secrets and backup/restore result are recorded in docs/design/identity/reports/IDENTITY-001-deployment.md. This row does not claim the product directory exists yet.
- ID001_SHARED_DB: prove Rauthy and SpiceDB both write/reopen against the same PostgreSQL database, migrations do not collide and the service roles cannot read the other schema. Restore the database plus documented key/config/cache dependencies and repeat readiness.
- ID001_PIN_CLONE: the venue fetches the exact pushed Lys ref with recursive submodules and verifies the expected Rauthy commit on ablative without using the Mac reading copy.
- A test configures a database address other than the local host: the resolved configuration names that address, and no default local address is substituted when it is unreachable (ADR-005).

**Files:**
- create: deploy/identity/compose.yaml
- create: deploy/identity/versions.json
- create: deploy/identity/config.example.toml
- create: deploy/identity/postgres-init.sql
- create: deploy/identity/README.md
- create: docs/design/identity/reports/IDENTITY-001-deployment.md
- create: crates/lys/tests/identity_deploy.rs
- create: crates/lys/tests/identity_refusals.rs
- create: crates/lys/tests/identity_shared_db.rs
- create: crates/lys/tests/identity_restart.rs
- create: crates/lys/tests/identity_support/mod.rs
- create: crates/lys/tests/identity_support/fixtures.rs
- create: crates/lys/tests/identity_support/server.rs
- create: crates/lys/tests/identity_support/compose.rs

**Checklist:**
- C7 — Rauthy, SpiceDB and one PostgreSQL service and database are packaged and installed for development, with separate roles and schema namespaces, the database address as configuration, and readiness, restart, named refusals, the shared database and restore proved (ID001_DEPLOY, ID001_DEPLOY_REFUSAL, ID001_SHARED_DB, ID001_PIN_CLONE).

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R2: Prepare, configure and check the deployment from the lys CLI

THE SYSTEM SHALL implement lys identity prepare, configure and health in the existing lys CLI with exactly revision 5's module manifest (docs/design/identity/briefs/IDENTITY-001.json:154-166): mod.rs declarations and re-exports only; cli.rs the identity subcommand arguments; config.rs typed deployment configuration and validation with no secret value in diagnostics; credentials.rs Zeroizing, redacted credential material and its stable reuse; private_files.rs restricted-mode durable file creation and outcome reconciliation; prepare.rs validating inputs and materialising the declared private artifacts; configure.rs idempotent client and theme reconciliation with stable operation identifiers; rauthy.rs typed Rauthy API requests and responses, named status errors and read-back after an uncertain outcome; health.rs named readiness checks for the declared services; error.rs typed errors carrying operation, resource and path, never secret bytes; themes.rs is R3's. configure SHALL register separate platform and Cambium OIDC clients with exact redirect URIs and RS256 where the Cambium verifier requires it, and the platform's own confidential client with S256 as revision 5 proposes (docs/design/identity/briefs/IDENTITY-001.json:138, docs/design/identity/briefs/IDENTITY-001.json:39). Google and GitHub federation credentials belong to Rauthy; Google API consent remains a different client purpose. Generated credentials stay out of Git and logs, and health output excludes secrets. No Python or shell provisioning engine: health.sh and the external tests/identity_deployment are replaced by these subcommands and Rust integration tests (docs/design/identity/briefs/IDENTITY-001.json:19, docs/design/identity/briefs/IDENTITY-001.json:140).

**Acceptance:**
- A redaction test formats every credential and error type under crates/lys/src/identity/ with Debug and Display, and captures the health output of a configured deployment: none contains a byte of a generated secret.
- prepare creates every declared private file with a restricted mode; a test reads each file's mode and counts the files against the declared set.
- configure run twice against the same Rauthy leaves exactly two clients and their themes with unchanged operation identifiers; a transport failure after a request is resolved by read-back and never creates a second client.
- health names each unready service or database: a test makes each one unavailable in turn and counts one named failure per case, equal to the number of declared services.
- Every module of the manifest exists under crates/lys/src/identity/, no other file is added there, and none exceeds 500 lines of code.

**Files:**
- create: crates/lys/src/identity/mod.rs
- create: crates/lys/src/identity/cli.rs
- create: crates/lys/src/identity/config.rs
- create: crates/lys/src/identity/credentials.rs
- create: crates/lys/src/identity/private_files.rs
- create: crates/lys/src/identity/prepare.rs
- create: crates/lys/src/identity/configure.rs
- create: crates/lys/src/identity/rauthy.rs
- create: crates/lys/src/identity/health.rs
- create: crates/lys/src/identity/error.rs
- modify: Cargo.toml
- modify: Cargo.lock
- modify: crates/lys/Cargo.toml
- modify: crates/lys/src/main.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/commands/error.rs

**Checklist:**
- C8 — lys identity prepare, configure and health exist in the CLI exactly as revision 5's module manifest names them, register the platform and Cambium clients idempotently, and keep every secret out of Git, logs, errors and health output.

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R3: Theme both Rauthy clients with Aion's vocabulary and each product's own accent

THE SYSTEM SHALL configure both Rauthy client themes from Aion's pinned neutral, text and radius vocabulary and each client's own product accent (ADR-010): the Cambium client green, the identity client the identity orange. It SHALL map text, text_high, bg, bg_high, action, accent and error into HSL for light and dark, plus button text colour and border radius, in deploy/identity/rauthy-themes.json, recording every source token and colour conversion in deploy/identity/theme-map.md, read and validated by crates/lys/src/identity/themes.rs, and SHALL verify readable contrast (docs/design/identity/briefs/IDENTITY-001.json:139). Fonts and page layout remain Rauthy's; no font or layout patch and no cross-application build dependency enters the fork. The colour values are read from the estate colour tokens, docs/design-system-v2/palette/estate-colour-tokens.json in the ablative docs repository (owner: Waffles, who added the identity entry at ablative-docs 385916e; docs/design/identity/briefs/IDENTITY-001.json:44); that file is another repository's, is read and never changed, and is named here rather than listed in files (CN4). Its purple status token is not copied.

**Acceptance:**
- ID001_THEME: inspect both client login pages in light and dark mode; all seven HSL fields, button text and border radius match the declared mapping, persist after restart and retain readable contrast. Record the source-token ref and Rauthy theme export without credentials.
- The Cambium client's accent is Cambium green and the identity client's is the identity orange of the estate tokens; neither is Aion blue, and no purple token appears in deploy/identity/rauthy-themes.json.

**Files:**
- create: deploy/identity/rauthy-themes.json
- create: deploy/identity/theme-map.md
- create: crates/lys/src/identity/themes.rs
- create: crates/lys/tests/identity_theme.rs

**Checklist:**
- C9 — Both Rauthy client themes carry Aion's neutral vocabulary with each product's own accent, Cambium green and the identity orange (ID001_THEME).

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R4: State what SpiceDB enforces in step 1, and hold to it

SpiceDB's step-1 role, in one sentence: in step 1 SpiceDB is installed, migrated, backed up and health-checked against the shared PostgreSQL database, and it enforces nothing, because no component asks it for a permission decision or writes a grant to it, and live capability policy and its enforcement stay with road step 2 (docs/design/identity/briefs/IDENTITY-001.json:30; docs/design/identity/STATEMENT-2026-09-22.md:143). What it does not do in step 1: it answers no check for any identity, it holds no grant or relationship of the directory, the lifecycle state recorded by DIRECTORY-003 gates nothing through it, and running it is not permission enforcement. THE SYSTEM SHALL write that sentence into deploy/identity/README.md, and SHALL NOT add to crates/lys/src/identity/ any SpiceDB permission check, relationship write or schema write; health reads SpiceDB's readiness only. Relationships a test writes to prove ID001_SHARED_DB are test fixtures, not grants.

**Acceptance:**
- deploy/identity/README.md carries the step-1 sentence of this requirement word for word.
- A search of crates/lys/src/identity/ finds SpiceDB named only by the health readiness check and its configuration: no permission check, relationship write or schema write call.

**Files:**
- modify: deploy/identity/README.md

**Checklist:**
- C10 — SpiceDB's step-1 role is written down and held: installed and checked, asked for no decision, holding no grant.

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and Waffles approves a brief revision before that file is edited (CN9).
- Development isolation: disposable test identities and test provider registrations only; no production tokens, real business sign-in or live Cambium participant migration (CN2).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion and never claimed by a loop completion (CN5, CN6).
- No row is dispatched until Waffles has reviewed it.
- lys-core, its cryptographic primitives and its published wire formats are unchanged.
- SpiceDB is asked for no permission decision and holds no grant in this row (R4).
- No Python or shell provisioning engine.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root: rg -n 'ID001_DEPLOY|ID001_DEPLOY_REFUSAL|ID001_THEME|ID001_SHARED_DB|ID001_PIN_CLONE' crates/lys/tests finds each identifier in a test.
- A search of every file this row touches finds no credential, token or key value.


---
type: brief
id: DIRECTORY-003
cluster: directory
title: Build the directory contract and signed authoritative identity changes
---

# DIRECTORY-003: Build the directory contract and signed authoritative identity changes

> **Cluster:** directory
> **Depends on:** DIRECTORY-002
> **Blocked by:** Waffles' review of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it), The exact file manifest for crates/lys-identity/, crates/lys-identity-server/ and tests/identity_contract/, reviewed before this row starts; revision 5 requires one for every wholly new module (docs/design/identity/briefs/IDENTITY-001.json:28, docs/design/identity/briefs/IDENTITY-001.json:190) and none is written yet
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> **Checklist:**
> - C11 — People and agents are registered with enduring identifiers and issuer-subject bindings, each agent under the signed-in person responsible for it, and registration issues no login, credential or certificate (ID001_DIRECTORY).
> - C12 — Every identity change is one signed event through lys-log-store under a jointly reviewed envelope, every answered projection equals replay across the crash boundaries, and a receipt verifies independently (ID001_AUDIT_FAULTS, ID001_RECEIPT).
> - C13 — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN).
> - C14 — The link-audit receiver is built and proved against fixtures before row 03 needs it (ID001_RECEIVER).
> - C15 — Each identity's lifecycle state is recorded as ADR-011 proposes, every transition one signed event, and the grant path's row is recorded open for Tom with its criteria drafted.
> **Stories:**
> - S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
> - S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.
> - S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

## Purpose

Row 04 of IDENTITY-001 revision 5, built before row 03 as Waffles reordered it: the directory of people and agents, where one signed committed event is both the identity change and its audit record, a receipt anyone can verify, the bounded step-1 administrator, and the link-audit receiver row 03 will need. Revised for the grant ruling (ADR-003): every agent is registered under the signed-in person responsible for it; and for the working lifecycle states (ADR-011, proposed): each identity's state is recorded beside it.

## Task

Carry IDENTITY-001 revision 5 row 04 (docs/design/identity/briefs/IDENTITY-001.json:168-207) into requirements: registration under a responsible person (R1), signed events and receipts (R2), the administrator (R3), the link-audit receiver (R4) and the lifecycle state, with the grant path's row recorded open for Tom (R5). Estimate: 10 focused implementer hours, revision 5's figure for this row (docs/design/identity/briefs/IDENTITY-001.json:206); no re-estimate, because the responsible person and the lifecycle state ride the same signed event envelope and projection as registration. If Tom places the grant path in this row, its hours are estimated then and any ceiling change goes to Tom through Waffles (CN7). The row's crates are wholly new modules: their exact file manifest is reviewed before the row starts (blocked_by, CN9). Every path is relative to the repository root.

## Requirements

### R1: Register people and agents with enduring identifiers, each agent under its responsible person

THE SYSTEM SHALL add the domain crates crates/lys-identity (directory records, typed API, event projection) and crates/lys-identity-server (OIDC session handling, administrator admission), keeping lys-core, its cryptographic primitives and its published formats unchanged, every new module named in the reviewed manifest (docs/design/identity/briefs/IDENTITY-001.json:190). It SHALL define stable person and agent identifiers, external issuer-subject bindings, registration and display-profile changes, explicit provenance and operation-ID retry semantics, written as the contract in docs/design/identity/DIRECTORY-CONTRACT.md (docs/design/identity/briefs/IDENTITY-001.json:191; P1). Revised for the grant ruling (ADR-003; docs/design/identity/STATEMENT-2026-09-22.md:21): an agent is registered by a signed-in person, and its signed registration event records that person as the agent's responsible person for life (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:77-81). ADR-011 proposes that a person registers themselves by first sign-in; step 1's bounded administrator policy refuses every other mutation caller (P9; docs/design/identity/briefs/IDENTITY-001.json:38), so in step 1 the only caller that may register a person or an agent is the configured administrator (R3), a first sign-in registers nobody, and every agent's responsible person is the administrator who registered it; wider registration arrives with step 2's assignment (docs/design/identity/briefs/IDENTITY-001.json:38). No API of this row changes an agent's responsible person. Registering an agent SHALL NOT pretend it is running, manufacture a human login for it, or issue it a credential, handle or certificate (P3; docs/design/identity/briefs/IDENTITY-001.json:37). The deployment files gain the directory service beside the three dependencies.

**Acceptance:**
- ID001_DIRECTORY: register a person and an agent, edit the display profile, list/read both and reopen; enduring IDs and signed history remain unchanged.
- A registered agent's record and its signed registration event both name its responsible person, the signed-in person who registered it; a registration without a signed-in caller is refused and creates no record; no request of the API changes an agent's responsible person.
- Registering an agent creates no Rauthy user and issues no credential, handle or certificate: a test counts Rauthy users and issued credentials before and after a registration and finds both counts unchanged.

**Files:**
- create: crates/lys-identity/
- create: crates/lys-identity-server/
- create: tests/identity_contract/
- create: docs/design/identity/DIRECTORY-CONTRACT.md
- modify: Cargo.toml
- modify: Cargo.lock
- modify: deploy/identity/compose.yaml
- modify: deploy/identity/config.example.toml
- modify: deploy/identity/README.md

**Checklist:**
- C11 — People and agents are registered with enduring identifiers and issuer-subject bindings, each agent under the signed-in person responsible for it, and registration issues no login, credential or certificate (ID001_DIRECTORY).

**Stories:**
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
- S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

### R2: Commit every identity change as one signed event through lys-log-store

THE SYSTEM SHALL commit every signed directory change through lys-log-store and rebuild projections from those events at open, reconciling an uncertain write before any affected read answers as current (P4, P5; docs/design/identity/briefs/IDENTITY-001.json:192, docs/design/identity/briefs/IDENTITY-001.json:40). The versioned event envelope, outside lys-core, with typed audit and context payloads, the log coordinate returned in the receipt outside the leaf and service-attested human actions, SHALL be reviewed jointly with Archie before it signs durable bytes, and written in docs/design/identity/IDENTITY-EVENTS.md; the commitment hash is named explicitly, and a SHA-256 attestation commitment is never confused with a BLAKE3 content address (docs/design/identity/briefs/IDENTITY-001.json:45). A receipt carries the fields of P7. THE SYSTEM SHALL provide a read-only receipt and inclusion-verification path (docs/design/identity/briefs/IDENTITY-001.json:193). The service attests the authenticated human actor and their provenance and never claims a person signed bytes with a key they do not hold (P8). The log is lys-log-store's file storage, not a Haematite backend (docs/design/identity/briefs/IDENTITY-001.json:45).

**Acceptance:**
- ID001_AUDIT_FAULTS: enumerate append/pin/projection crash boundaries and count exercised cases; every answered projection equals replay; uncertain operations retain identity and resolve once without silent loss or double application.
- ID001_RECEIPT: independently verify a recorded change against a checkpoint/key; changed actor, payload, sequence or signature fails verification. Secret-redaction tests cover debug, errors and serialized public responses.
- docs/design/identity/IDENTITY-EVENTS.md records the envelope's version, its typed payloads and the named commitment hash, and records Archie's review of it before any durable bytes are signed under it.

**Files:**
- create: crates/lys-identity/
- create: tests/identity_contract/
- create: docs/design/identity/IDENTITY-EVENTS.md

**Checklist:**
- C12 — Every identity change is one signed event through lys-log-store under a jointly reviewed envelope, every answered projection equals replay across the crash boundaries, and a receipt verifies independently (ID001_AUDIT_FAULTS, ID001_RECEIPT).

**Stories:**
- S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.
- S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

### R3: Admit only the configured administrator to change the directory

THE SYSTEM SHALL authenticate the initial directory administrator by an explicitly configured issuer and subject, never by email or by being the first visitor, fail closed for every other mutation caller, and make the limited step-1 authority visible (docs/design/identity/briefs/IDENTITY-001.json:193, docs/design/identity/briefs/IDENTITY-001.json:38; P9). General assignment and why-access views arrive in step 2.

**Acceptance:**
- ID001_ADMIN: an unauthenticated caller, a non-admin with the same email, and wrong issuer/subject are refused; denied calls cannot mutate state.

**Files:**
- create: crates/lys-identity-server/
- create: tests/identity_contract/

**Checklist:**
- C13 — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN).

**Stories:**
- S4 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to register an agent under my name before it ever runs, with every change to it signed, so that who created it and who answers for it is never reconstructed after the fact.

### R4: Build the link-audit receiver before row 03 needs it

THE SYSTEM SHALL build and test the authenticated link-audit receiver against the reviewed typed contract and fixtures before DIRECTORY-004 needs it. It consumes the minimal durable source selected in row 01 (a same-transaction link audit record or outbox with stable operation IDs and explicit pending and acknowledged state; docs/design/identity/briefs/IDENTITY-001.json:42), deduplicates stable source operation IDs and returns verifiable receipts. It separates issuer observations from human-signed claims, and audit actor provenance survives replay (docs/design/identity/briefs/IDENTITY-001.json:194).

**Acceptance:**
- ID001_RECEIVER: fixture delivery, duplicate delivery, lost acknowledgement, receiver restart and unauthorized source exercise the reviewed link-audit contract without requiring the future multi-provider fork. Count each leg and prove one logical event per source operation.

**Files:**
- create: crates/lys-identity/
- create: crates/lys-identity-server/
- create: tests/identity_contract/

**Checklist:**
- C14 — The link-audit receiver is built and proved against fixtures before row 03 needs it (ID001_RECEIVER).

**Stories:**
- S7 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to check a recorded identity change against a checkpoint and key with standard tooling, so that the directory's history does not rest on the operator's word.

### R5: Record each identity's lifecycle state, and keep the grant path's row open

THE SYSTEM SHALL record beside each identity its lifecycle state as ADR-011 proposes (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44): registration yields registered; activate, suspend, reinstate and retire are the only transitions, each one signed directory event naming the authenticated actor and their provenance, the identity, from, to, when and the reason given; a retired identity is never reactivated; a transition outside the table is refused by name and records nothing. In step 1 the caller is the configured administrator (R3). The state is recorded, not enforced: nothing in this row reads it to admit or refuse an action (CN11). ADR-011 is status proposed; a change to the states by Tom is a brief revision before this requirement is built.

OPEN for Tom, not decided here: which row owns the grant path. The path: a person signs in, creates an agent under themselves, grants it one project, the agent's action on that project is allowed, the grant is revoked or the agent suspended, and the same action is refused by name, with the audit record naming who made each change (docs/design/identity/STATEMENT-2026-09-22.md:183, Chippy 17:08; docs/design/identity/PROVISIONING-2026-09-22.md:39-46). The sources do not settle its row. Revision 5 keeps arbitrary grants and live capability enforcement out of step 1 and in road step 2 (docs/design/identity/briefs/IDENTITY-001.json:29-30). Chippy's concrete first screen of 17:08, said after revision 5 was written, puts the path on step 1's screen (docs/design/identity/STATEMENT-2026-09-22.md:183), and PROVISIONING places the grant in 'steps 1 and 2 of the road' without naming a row (docs/design/identity/PROVISIONING-2026-09-22.md:23-24). The question for Tom: does the grant path land in this row (DIRECTORY-003), in a new row of this cluster before DIRECTORY-005, or in the first brief of road step 2? Whichever row it lands in carries these criteria, drafted here so none is lost: (a) allowed before revoke: with the agent active and one grant on project P, the agent's action on P is admitted; (b) refused after revoke: once the grant is revoked, the same action is refused by name at the next check and nothing on P changes; (c) refused after suspend: once the agent is suspended, the same action is refused by name while its grant stays recorded, under whatever suspension semantics Tom settles (design non-goals); (d) every grant, revoke and transition is one signed audit record naming the actor. None of (a) to (d) is an acceptance criterion of this brief.

**Acceptance:**
- A test drives register, activate, suspend, reinstate and retire on one agent and counts 5 signed events, each naming actor, identity, from, to and time; the projection after reopen equals replay.
- Each transition outside the table (retired to active, registered to suspended, registered to retired, and a transition of an unknown identity) is refused by name and the log's size does not change; the test counts one refusal per case it names.
- No code in this row asks SpiceDB for a decision or writes a grant to it, and the grant path's row is still recorded open for Tom, with its question and drafted criteria, when the row is reviewed.

**Files:**
- create: crates/lys-identity/
- create: tests/identity_contract/
- create: docs/design/identity/DIRECTORY-CONTRACT.md

**Checklist:**
- C15 — Each identity's lifecycle state is recorded as ADR-011 proposes, every transition one signed event, and the grant path's row is recorded open for Tom with its criteria drafted.

**Stories:**
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and Waffles approves a brief revision before that file is edited (CN9).
- Development isolation: disposable test identities and test provider registrations only; no production tokens, real business sign-in or live Cambium participant migration (CN2).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion and never claimed by a loop completion (CN5, CN6).
- No row is dispatched until Waffles has reviewed it.
- lys-core, its cryptographic primitives and its published wire formats are unchanged; the event envelope lives outside lys-core and signs no durable bytes before its joint review with Archie.
- No grant is written and no permission is enforced in this row; the grant path's row stays open for Tom (R5).
- Registration issues no login, credential, handle or certificate.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root: rg -n 'ID001_DIRECTORY|ID001_AUDIT_FAULTS|ID001_ADMIN|ID001_RECEIPT|ID001_RECEIVER' crates/lys-identity tests/identity_contract finds each identifier in a test.
- A search of every file this row touches finds no credential, token or key value.


---
type: brief
id: DIRECTORY-004
cluster: directory
title: Link two upstream providers to one Rauthy person
---

# DIRECTORY-004: Link two upstream providers to one Rauthy person

> **Cluster:** directory
> **Depends on:** DIRECTORY-003
> **Blocked by:** The fork-owned brief for row 03's Rauthy changes, which does not yet exist: a brief in the ablative-io/rauthy fork's own repository (vendor/rauthy, branch ablative; maintenance owner Chippy, Waffles reviewing, docs/design/identity/RAUTHY-BASELINE.md:39) carrying the fork files R1 names. This brief is not executable until that brief exists and has landed on a gated fork commit, Waffles' review of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it)
> **Design anchor:**
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> **Checklist:**
> - C16 — The pinned fork commit links Google and GitHub to one unchanged Rauthy person, refuses every collision and replay, migrates both storages and audits each link atomically (ID001_LINK_PAIR, ID001_LINK_REFUSAL, ID001_LINK_MIGRATION, ID001_LINK_AUDIT).
> - C17 — The provider-link contract and the links report are written, and the report names the gated fork commit the pin moves to.
> **Stories:**
> - S5 (Responsible person, Signs in and provisions agents under their own authority) — As a person signing in, I want Google and GitHub to resolve to the one me, so that adding a provider never splits me into two people or merges me with someone who shares my email.

## Purpose

Row 03 of IDENTITY-001 revision 5: Google and GitHub resolve to one Rauthy person, collisions are refused rather than merged by email, and each link and unlink is audited atomically and acknowledged by the receiver DIRECTORY-003 built. The linking change lives in the maintained fork (ADR-009), a repository of its own; this brief is the lys side of the row: the pin, the provider-link contract and the report.

## Task

Carry IDENTITY-001 revision 5 row 03 (docs/design/identity/briefs/IDENTITY-001.json:208-262). The fork's files are the fork-owned brief's (blocked_by) and are named in R1's spec with their owner, never listed in files (CN4). This brief moves vendor/rauthy to the fork commit that brief lands and gates (R1), and writes the provider-link contract and the links report (R2). Estimate: 10 focused implementer hours, revision 5's figure for the whole row (docs/design/identity/briefs/IDENTITY-001.json:261), shared between this brief and the fork-owned brief; no re-estimate, and the split is stated when the fork-owned brief is written. The row's live demonstration, ID001_LINK_LIVE, is a verification step after the row lands and holds DIRECTORY-005 (CN6).

## Requirements

### R1: Move the pin to the gated fork commit that links two providers to one person

THE SYSTEM SHALL move the vendor/rauthy submodule pin to a commit on the fork's ablative branch that carries the linking change and has passed the fork-owned brief's gate (ADR-009); the pin never names an ungated or cherry-picked commit.

The linking change, owned by the fork-owned brief and named here only (owner: the ablative-io/rauthy fork, maintained by Chippy with Waffles reviewing; paths relative to the fork's root, from revision 5's wall, docs/design/identity/briefs/IDENTITY-001.json:216-238): src/api/src/auth_providers.rs, src/api_types/src/auth_providers.rs, src/api_types/src/users.rs, src/data/src/entity/auth_providers.rs, src/data/src/entity/users.rs, src/data/src/entity/mod.rs, src/data/src/entity/identity_links.rs, src/data/src/entity/identity_link_audit.rs, src/data/src/migration/inserts.rs, src/service/src/oidc/auth_providers/login_finish.rs, src/service/src/oidc/auth_providers/login_start.rs, frontend/src/api/types/auth_provider.ts, frontend/src/api/types/user.ts, frontend/src/lib/account/AccMain.svelte, frontend/src/lib/account/AccOther.svelte, frontend/src/lib/account/AccLinkedProviders.svelte, frontend/src/lib/admin/users/UserInfo.svelte, frontend/src/routes/providers/callback/+page.svelte, migrations/hiqlite/32_identity_links.sql, migrations/postgres/V27__identity_links.sql, tests/identity_links/. Its work, from revision 5 (docs/design/identity/briefs/IDENTITY-001.json:246-249): replace the single provider pair with a link relation unique on provider and subject, migrating existing links transactionally without changing Rauthy user IDs, with row 01's exact migration filenames confirmed before code starts; require an authenticated person, fresh reauthentication and a single-use target-bound linking intent, bind state, nonce, provider and callback, and reject identity collision instead of merging by matching email; update every reader row 01 found, including administrative deletion, export and import; list links, and unlink only after confirming the identity keeps a usable authentication or recovery method, preserving upstream account security controls; commit link or unlink and its audit provenance atomically through the minimal durable link-audit design selected in row 01, and label the audit complete only on an acknowledged Lys receipt from the DIRECTORY-003 receiver, so a duplicate delivery never creates another identity event.

**Acceptance:**
- ID001_LINK_PAIR: Google then GitHub, and GitHub then Google, resolve to one unchanged user subject; sign out and back in through either provider and reopen storage.
- ID001_LINK_REFUSAL: same email with different subject, already-owned provider identity, replayed/cross-account intent, CSRF/nonce mismatch and final-login unlink are refused with no unintended link.
- ID001_LINK_MIGRATION: both supported storage migrations preserve IDs and links, restart safely after interruption and refuse incompatible schema versions by name.
- ID001_LINK_AUDIT: crash after database commit but before audit acknowledgement retains the outbox; delivery/retry yields one logical signed event. Audit outage is visible, never a false completed receipt.
- vendor/rauthy names a commit on the ablative branch whose fork gate passed, and the four identifiers above are each found in that commit's tests under vendor/rauthy/tests/identity_links/.

**Files:**
- modify: vendor/rauthy

**Checklist:**
- C16 — The pinned fork commit links Google and GitHub to one unchanged Rauthy person, refuses every collision and replay, migrates both storages and audits each link atomically (ID001_LINK_PAIR, ID001_LINK_REFUSAL, ID001_LINK_MIGRATION, ID001_LINK_AUDIT).

**Stories:**
- S5 (Responsible person, Signs in and provisions agents under their own authority) — As a person signing in, I want Google and GitHub to resolve to the one me, so that adding a provider never splits me into two people or merges me with someone who shares my email.

### R2: Write the provider-link contract and the links report

THE SYSTEM SHALL write docs/design/identity/PROVIDER-LINK-CONTRACT.md, the typed contract between the fork's link audit and the DIRECTORY-003 receiver (stable source operation IDs, pending and acknowledged states, what an issuer observes as against what a person claims), and docs/design/identity/reports/IDENTITY-001-links.md, the report of the row: the fork commit the pin moves to, its gate result, the counted legs of each ID001_LINK_* case and the development install's release-and-advisory check (CN10). Both keep the file names revision 5 gives this row (docs/design/identity/briefs/IDENTITY-001.json:241-242).

**Acceptance:**
- docs/design/identity/PROVIDER-LINK-CONTRACT.md names every field the receiver reads and the states a link audit passes through, and the DIRECTORY-003 receiver's fixtures match it.
- docs/design/identity/reports/IDENTITY-001-links.md names the pinned fork commit, its gate result and the count of exercised legs per identifier, and records the release-and-advisory check of the install.

**Files:**
- create: docs/design/identity/PROVIDER-LINK-CONTRACT.md
- create: docs/design/identity/reports/IDENTITY-001-links.md

**Checklist:**
- C17 — The provider-link contract and the links report are written, and the report names the gated fork commit the pin moves to.

**Stories:**
- S5 (Responsible person, Signs in and provisions agents under their own authority) — As a person signing in, I want Google and GitHub to resolve to the one me, so that adding a provider never splits me into two people or merges me with someone who shares my email.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and Waffles approves a brief revision before that file is edited (CN9).
- Development isolation: disposable test identities and test provider registrations only; no production tokens, real business sign-in or live Cambium participant migration (CN2).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion and never claimed by a loop completion (CN5, CN6).
- No row is dispatched until Waffles has reviewed it.
- No fork file is edited by this brief: the fork's changes land through the fork-owned brief and its gate, and reach this repository only as a pin.
- No cherry-pick and no nightly base: rows 02 to 05 run on v0.36.2 (CN10).

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root, with submodules checked out: rg -n 'ID001_LINK_PAIR|ID001_LINK_REFUSAL|ID001_LINK_MIGRATION|ID001_LINK_AUDIT' vendor/rauthy/tests/identity_links finds each identifier in a test.
- ID001_LINK_LIVE, after the row lands (CN5, CN6): a person installs the exact gated fork and shows Tom two linked providers resolving to one person in Rauthy's own account page, signing in through both, then posts a separate install and showing receipt to Tom with the Melbourne pass time, fork ref, artifact hash and observed result. A venue test or screenshot alone does not replace the live demonstration, and DIRECTORY-005 stays blocked until the receipt is recorded.


---
type: brief
id: DIRECTORY-005
cluster: directory
title: Complete the standalone screen journey
---

# DIRECTORY-005: Complete the standalone screen journey

> **Cluster:** directory
> **Depends on:** DIRECTORY-004
> **Blocked by:** ID001_LINK_LIVE: Tom's live demonstration of row 03 (DIRECTORY-004), recorded by its posted install and showing receipt with the Melbourne pass time, fork ref, artifact hash and observed result; a loop completion never stands in for it (CN6), Waffles' review of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it), The exact file manifest for surface/identity/, reviewed before this row starts (docs/design/identity/briefs/IDENTITY-001.json:28)
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> **Checklist:**
> - C18 — The standalone journey (sign in, the directory, a record, registering and editing an agent, linked accounts, signed history) works with Cambium and Manifold absent, each agent showing its responsible person (ID001_STANDALONE).
> - C19 — Expired login, unauthorized edit, provider collision, audit pending and backend outage each have a visible, actionable outcome and no optimistic completed state (ID001_SCREEN_REFUSAL).
> - C20 — The screens follow Aion's appearance with the identity product's own orange accent and no build dependency on Cambium or Aion (ADR-010).
> **Stories:**
> - S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
> - S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.
> - S6 (Operator, Installs and runs the standalone identity product) — As the operator, I want the directory's screens to show every refusal, pending audit and outage as it is, so that I never act on a completed state that did not happen.

## Purpose

Row 05 of IDENTITY-001 revision 5: the standalone product's own screens, 'beyond just a login page', so an operator installs, signs in, links two providers, registers an agent and reads its signed history with Cambium and Manifold absent. Revised for the grant ruling (ADR-003): each agent shows the person responsible for it, and the screen does not present access or delegation it cannot yet grant.

## Task

Carry IDENTITY-001 revision 5 row 05 (docs/design/identity/briefs/IDENTITY-001.json:263-294) into requirements: the screen journey (R1), visible refusals (R2) and the appearance (R3). Estimate: 6 focused implementer hours, revision 5's figure for this row (docs/design/identity/briefs/IDENTITY-001.json:293); no re-estimate. surface/identity/ is a wholly new module: its exact file manifest is reviewed before the row starts (CN9). The row's live demonstration, ID001_DIRECTORY_LIVE, is a verification step after the row lands and is the design's hold point before any row 06 (CN6). Every path is relative to the repository root.

## Requirements

### R1: Build the standalone screen journey

THE SYSTEM SHALL build sign-in, the directory of people and agents, record detail, register and edit agent, the linked-account entry point and signed change history in surface/identity/, served by crates/lys-identity-server/src/routes.rs and crates/lys-identity-server/src/assets.rs, with boundary types generated from the server schema, preserving shared lifecycle extension points without implementing Archie's screens (docs/design/identity/briefs/IDENTITY-001.json:280). Registration states what has been created. Revised for the grant ruling (docs/design/identity/STATEMENT-2026-09-22.md:21, Chippy 17:17, agreed): each agent's record shows its responsible person and its recorded lifecycle state (ADR-011, proposed); where the statement's first screen shows the access granted and the delegation rights beside the responsible person, step 1 shows them as not yet available, never as empty working controls. An agent's certificate shows as not issued, never a placeholder (ADR-008). Controls for launch, a start command (ADR-007), live permissions, secrets and memory are not presented as working in step 1 (docs/design/identity/briefs/IDENTITY-001.json:282). deploy/identity/README.md gains the screen's install and use, and docs/design/identity/reports/IDENTITY-001-standalone.md records the row's evidence.

**Acceptance:**
- ID001_STANDALONE: with Cambium and Manifold absent, an operator installs, signs in, links two providers, creates an agent, returns after restart and finds the same records and inspectable change history.
- Use the native browser against the installed venue artifact; record screenshots and observed actions plus artifact hashes in docs/design/identity/reports/IDENTITY-001-standalone.md. A mockup or frontend build alone is not this acceptance.
- Each agent's record shows its responsible person and its recorded lifecycle state; its access and delegation rights show as not available in step 1, never as empty working controls; its certificate shows as not issued; no control for launch, permissions, secrets or memory is presented as working.

**Files:**
- create: surface/identity/
- create: crates/lys-identity-server/src/assets.rs
- create: crates/lys-identity-server/src/routes.rs
- create: docs/design/identity/reports/IDENTITY-001-standalone.md
- modify: deploy/identity/README.md

**Checklist:**
- C18 — The standalone journey (sign in, the directory, a record, registering and editing an agent, linked accounts, signed history) works with Cambium and Manifold absent, each agent showing its responsible person (ID001_STANDALONE).

**Stories:**
- S1 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want every grant my agent holds to trace back to me, so that withdrawing my authority stops everything derived from it.
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R2: Show every refusal, pending audit and outage as it is

THE SYSTEM SHALL show provider-link audit pending, unavailable services and named refusals accurately, with an actionable visible outcome for each and no optimistic completed state after a refusal (docs/design/identity/briefs/IDENTITY-001.json:281, docs/design/identity/briefs/IDENTITY-001.json:286; P5).

**Acceptance:**
- ID001_SCREEN_REFUSAL: expired login, unauthorized edit, provider collision, audit-pending and backend outage have actionable visible outcomes; no optimistic completed state after refusal.

**Files:**
- create: surface/identity/

**Checklist:**
- C19 — Expired login, unauthorized edit, provider collision, audit pending and backend outage each have a visible, actionable outcome and no optimistic completed state (ID001_SCREEN_REFUSAL).

**Stories:**
- S6 (Operator, Installs and runs the standalone identity product) — As the operator, I want the directory's screens to show every refusal, pending audit and outage as it is, so that I never act on a completed state that did not happen.

### R3: Follow Aion's appearance with the identity product's own accent

THE SYSTEM SHALL follow Aion's structure, DM Sans and JetBrains Mono typography, radii, spacing and browser interaction conventions, with the identity product's own orange accent instead of Aion-blue (ADR-010; docs/design/identity/briefs/IDENTITY-001.json:281). Neutral design values keep their pinned provenance and the accent values are separately declared, read from the estate colour tokens (docs/design-system-v2/palette/estate-colour-tokens.json in the ablative docs repository, owner Waffles; named, not listed in files, CN4), with no build dependency on Cambium or Aion. Central design-system extraction is outside this brief.

**Acceptance:**
- The screens' accent values are the identity orange of the estate tokens with their recorded source ref; no Aion-blue accent and no purple appear, and the frontend build names no Cambium or Aion package.

**Files:**
- create: surface/identity/

**Checklist:**
- C20 — The screens follow Aion's appearance with the identity product's own orange accent and no build dependency on Cambium or Aion (ADR-010).

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and Waffles approves a brief revision before that file is edited (CN9).
- Development isolation: disposable test identities and test provider registrations only; no production tokens, real business sign-in or live Cambium participant migration (CN2).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion and never claimed by a loop completion (CN5, CN6).
- No row is dispatched until Waffles has reviewed it.
- Archie's lifecycle screens are not implemented; their extension points are kept.
- No grant, launch, secret or memory control is presented as working in step 1.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root: rg -n 'ID001_STANDALONE|ID001_SCREEN_REFUSAL' surface/identity finds each identifier in a test.
- ID001_DIRECTORY_LIVE, after the row lands (CN5, CN6): a person installs the gated standalone directory and completes its journey with Tom in a live browser, then posts a separate line to Tom with the Melbourne pass time, installed refs and hashes and what was demonstrated, before any row 06 starts; no deferral to row 07. This is the design's hold point: the brief that follows is blocked by it until Tom's receipt is recorded, and the choice of Rauthy base recorded in the design's non-goals falls due at this showing if no suitable upstream release exists (CN10).
- Frontend checks pass with strict types and the generated API schema, with negative and crash-boundary tests.


---
type: brief
id: DIRECTORY-006
cluster: directory
title: Enforce human-rooted grants, explicit delegation and the You-page access journey
---

# DIRECTORY-006: Enforce human-rooted grants, explicit delegation and the You-page access journey

> **Cluster:** directory
> **Depends on:** DIRECTORY-002, DIRECTORY-003
> **Blocked by:** Waffles review of every requirement and the accepted mock-up/conformance snapshot before dispatch., DIRECTORY-002 and DIRECTORY-003 are published foundation briefs, not implemented foundations. DIRECTORY-003 must establish the identity crates, reviewed event envelope and server integration manifest; reconcile every future modify path below against its landed implementation before dispatch. The listed new grant modules do not exist in current source., Surface integration waits for DIRECTORY-005 to establish surface/identity and its route/type-generation paths. R6 cannot dispatch before that dependency; DIRECTORY-005 must not depend on R6, so backend grant work and screen integration have separate readiness., Reviewed grant representation, root-authority bootstrap and SpiceDB consistency contract remain open in DIRECTORY-001C5 and ADR-003. Behavioural tests below constrain that design; they do not authorise freezing a cryptographic format., Lifecycle suspension semantics and role version policy are supplied by their reviewed ADRs. Reinstatement safety and expiry invariants here do not settle their remaining policy choices., Review of the proposed R1 grant schema and R4 freshness mechanism; their additional fields and consistency strategy are proposals, not existing settled policy.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> **Checklist:**
> - C21 — The proposed application grant schema separates exercise and pass-on authority; its additional fields require independent review before dispatch and do not change published crypto formats.
> - C22 — Delegation requires an explicit permission to delegate to the requested subject kind; use-only, absent policy and owner labels cannot create it.
> - C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.
> - C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.
> - C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.
> - C26 — Inherited expiry and provisional end dates are enforced at admission; role changes and reinstatement cannot resurrect expired or revoked authority.
> - C27 — Typed browser/API/agent seams use one authenticated evaluator; forward and reverse explanations name the same authority path and revision without private-record leakage.
> - C28 — Observed grant usage names its source and time; not seen is not reported as never used.
> - C29 — The You and delegation screens show real server authority, separate service accounts from sign-in identities, and preserve personal versus administrator visibility.
> - C30 — The accepted mock-up conformance is tested through actual requests, refusals, pending outcomes, keyboard paths and durable read-back rather than simulated success.
> **Stories:**
> - S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
> - S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.
> - S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.
> - S11 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it.
> - S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

## Purpose

Make the human-rooted grant path enforceable: sign in, give an agent a bounded permission, prove the action works, revoke the source, and prove the same action is refused. Use and lending are separate positive rights and the screen explains the real server decision. Authority for this amendment: docs/design/identity/CONFORMANCE.md at commit 1353c22, rows 1.3, 1.4, 2.1–2.6 and 7.3. Context only: Tom requested complete briefs tied to the mock-up on 23 September 2026 at 19:04:22 Melbourne, then the right Cambium boards and workflows at 19:10:35; these conversational times are not repository evidence.

## Task

Implement R1–R5 in order after the directory/service foundations, then R6 after the standalone surface foundation. Estimated work: R1 2h, R2 3h (ancestry/refusal matrix), R3 4h (forced durability boundaries), R4 3h (freshness and expiry), R5 2h, R6 3h (two-person browser evidence), total 17 hours as a new increment, not silently inside IDENTITY-001's 48-hour ceiling. Estimates are proposals for review. Exact file walls follow the new crate roots already named by IDENTITY-001 rows 04/05; none is asserted implemented. Every future modify path must be reconciled to those dependencies' actual manifests before dispatch. Preserve all ID001 requirements in their owning predecessor briefs and the frozen IDENTITY-001 history. In scope: domain grants, authority enforcement, durable audit, explanation API and You/delegation UI. Out: provider federation changes, secret-value storage/proxy, actual process execution, assistant actions, graph renderer and published cryptographic format changes.

## Requirements

### R1: Define the reviewed grant contract without changing published cryptography

PROPOSAL FOR REVIEW, not a settled grant schema: define a typed application grant contract that identifies the grant, issuer, holder, responsible person, resource, exercisable actions, explicit pass-on authority and permitted recipient kinds, source grant, time window, policy revision and authorising audit event. Exercising an action and delegating it SHALL be separate decisions. A missing pass-on field SHALL NOT mean permission. Relations SHALL resolve through the reviewed model to action/resource sets; the code SHALL NOT infer a global owner/editor/viewer rank from display strings. Before any durable grant is signed, the event envelope, canonical encoding, root-authority bootstrap and lineage semantics SHALL be independently reviewed. This row defines application-domain types in lys-identity; it SHALL NOT reinterpret seat/speaks-for or mutate lys/delegation/v1, which currently supplies neither this capability schema nor an expiry. Provenance: ADR-003; STATEMENT-2026-09-22.md, Everything is pegged to a human authority; PROVISIONING-2026-09-22.md, What one grant says. Conformance 1.3, 1.4, 2.2, 2.3. The extra holder, responsible-person, recipient-kind, time-window, policy-revision and authorising-event fields are design proposals in this row; PROVISIONING-2026-09-22.md supplies five source facts and leaves representation open. This row and GRANT_CONTRACT cannot be dispatched until independent contract review ratifies the schema. Checkable conformance source: docs/design/identity/CONFORMANCE.md at commit 1353c22.

**Acceptance:**
- GRANT_CONTRACT: serialise/parse a grant with each declared member and independently supplied schema fixture; unknown members, absent required authority, malformed lineage and unknown recipient kinds refuse by name. Count each case; no permission is supplied by a deserialisation default.
- GRANT_MODEL: define two model relations whose names do not imply their action sets. A requested subset is checked against the model, not lexical order or a hard-coded rank; model version is retained in the decision.
- GRANT_WIRE_BOUNDARY: an explicit reviewed application-envelope identifier precedes the first durable signed grant. Existing published lys wire-vector tests remain byte-identical; an unratified delegation format is not used as a substitute capability token.

**Files:**
- create: crates/lys-identity/src/grants/mod.rs
- create: crates/lys-identity/src/grants/types.rs
- create: crates/lys-identity/src/grants/error.rs
- create: crates/lys-identity/tests/grant_contract.rs
- create: docs/design/identity/GRANT-CONTRACT.md
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C21 — The proposed application grant schema separates exercise and pass-on authority; its additional fields require independent review before dispatch and do not change published crypto formats.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.

### R2: Enforce affirmative delegation and bounded ancestry

WHEN a person or agent asks to delegate authority, THE SYSTEM SHALL check the actor's current exercise and delegation rights, requested resource/actions and recipient kind against every effective ancestor, ending at an authorised person. The requested authority SHALL be an explicit subset, never inherited wholesale. A use-only grant, an owner label, missing prohibition or access to the UI SHALL NOT establish pass-on authority. People-only authority SHALL refuse agents even when the person owns that agent. Unknown parents, cycles, wrong responsible persons and attempts to launder authority through a new role, secret handle or sibling grant SHALL refuse before any mutation. An agent that holds explicit pass-on permission may invoke the same operation as a person; being an agent alone is not either a permit or a categorical prohibition. The policy determines recipients rather than the mock-up's sample restriction to the caller's own agents. Provenance: ADR-003; AGENT-PARITY-2026-09-23.md. Conformance 2.1–2.4, 7.3. Secret lending also recognises server-verified real ownership as the affirmative route in docs/design/identity/CONFORMANCE.md at commit 1353c22 row 7.3; a display label is not that ownership record.

**Acceptance:**
- GRANT_USE_VS_LEND: Tom may read a project through Dana's use-only grant. His read succeeds; browser, direct API and agent-tool attempts to grant that access to his agent all refuse and create zero grant events. Add a distinct affirmative pass-on grant: the bounded control now succeeds.
- GRANT_RECIPIENT: exercise human-only, agent-only and non-delegable grants with both recipient kinds. Assert each named permit/refusal and exact zero mutations on denied requests, including the owner's own agent case.
- GRANT_ANCESTRY: attempt a broader resource, broader action, a forged parent, a cycle, a changed responsible person and a revoked ancestor. Each refuses with the blocking boundary. A two-hop permitted narrower chain succeeds and explains its actual ancestors.
- GRANT_AGENT_PARITY: the same authenticated authority and request produce the same effective decision through browser/API/tool routes. An agent granted pass-on can delegate inside that grant; an agent without it cannot.

**Files:**
- create: crates/lys-identity/src/grants/admission.rs
- create: crates/lys-identity/src/grants/lineage.rs
- create: crates/lys-identity/src/grants/authority.rs
- create: crates/lys-identity/tests/grant_delegation.rs
- modify: crates/lys-identity/src/grants/mod.rs

**Checklist:**
- C22 — Delegation requires an explicit permission to delegate to the requested subject kind; use-only, absent policy and owner labels cannot create it.
- C23 — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.

### R3: Commit grants and their audit as one replayable operation

THE SYSTEM SHALL record a grant mutation as one signed authoritative event using DIRECTORY-003's reviewed event owner, and derive both the directory projection and the permission projection from that event. A database write followed by best-effort audit is forbidden. Each mutation SHALL carry a stable operation ID; the same ID and payload returns the same logical outcome, while changed payload under that ID refuses. Unknown append or projection outcomes SHALL remain named and retained for reconciliation; the API SHALL NOT manufacture success or issue a fresh operation when acknowledgement is lost. Any read or admission that depends on unresolved authority SHALL refuse naming the affected grant/operation; unrelated authority stays usable. Projection progress SHALL record the minimum revision required for a fresh decision, including across restart. Provenance: directory principlesP4/P5 and ID001_AUDIT_FAULTS; conformance1.4,8.1.

**Acceptance:**
- GRANT_DURABILITY: enumerate every append, sync, authoritative-event and projection boundary in the implementation and inject a failure at each. Count the legs. Every answered grant/permission read equals replay, while unresolved reads name the grant/operation. Unrelated grant controls still succeed.
- GRANT_IDEMPOTENCE: lose acknowledgement after commit, retry identical ID/payload, reopen and retry again. One logical grant and one authorising event remain. A changed payload under the same ID refuses and changes neither projection.
- GRANT_AUDIT: independently verify a grant mutation receipt; tampering with actor, source grant, recipient, resource/actions, sequence or signature is rejected. A refusal never appears as a successful grant event.

**Files:**
- create: crates/lys-identity/src/grants/events.rs
- create: crates/lys-identity/src/grants/projection.rs
- create: crates/lys-identity/src/grants/recovery.rs
- create: crates/lys-identity/tests/grant_faults.rs
- create: crates/lys-identity/tests/grant_receipts.rs
- modify: crates/lys-identity/src/grants/mod.rs

**Checklist:**
- C24 — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.

**Stories:**
- S10 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an auditor, I want grant changes and retries tied to one durable signed operation, so restart or an uncertain reply cannot invent, lose or duplicate authority.

### R4: Enforce revocation, inherited expiry and current permission decisions

WHEN an ancestor is revoked or expires, THE SYSTEM SHALL refuse fresh exercise and further delegation through every grant derived from it. Independent grants to the same person or resource SHALL remain independent. Proposal under review: require a permission decision at least as fresh as the authoritative change, and refuse or await freshness rather than permit from a stale replica, unavailable engine or unresolved projection. The mechanism and choice must be settled before dispatch. Expiry SHALL be enforced at admission using the named clock, including inherited limits; a role edit or version move SHALL NOT renew a provisional grant. Reinstating a suspended identity SHALL recheck current ancestry and leases, never resurrect separately revoked or expired grants. Shared ancestor use/spend budgets are counted once by the broker in SECRETS-002 R9, never cloned into each descendant. The exact broader suspension policy remains the lifecycle ADR's, not a decision smuggled into this row. Conformance 2.5, 2.6, 3.3, 4.5.

**Acceptance:**
- GRANT_REVOKE: create a two-hop chain and an independently authorised sibling. A permitted action succeeds, then revoke the chain root and repeat exactly the same action for each descendant: every derived call refuses; the independent control succeeds.
- GRANT_EXPIRY: under a controlled clock, assert permit immediately before each relevant end boundary and refusal at/after it, including an earlier ancestor end. Neither editing a role nor selecting a newer role version extends the holding.
- GRANT_FRESHNESS: pause permission projection before applying a committed revoke; a caller presenting the required revision cannot obtain a permit from the old state. Resume projection and verify the named revoked decision after reopen.
- GRANT_REINSTATE: while an identity is suspended, revoke one grant and expire a second. After reinstatement both remain refused; only a third still-authorised control is usable. No renewal or issuance event is invented.

**Files:**
- create: crates/lys-identity/src/grants/revocation.rs
- create: crates/lys-identity/src/grants/expiry.rs
- create: crates/lys-identity/src/grants/permission.rs
- create: crates/lys-identity/tests/grant_revocation.rs
- create: crates/lys-identity/tests/grant_expiry.rs
- modify: crates/lys-identity/src/grants/mod.rs

**Checklist:**
- C25 — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.
- C26 — Inherited expiry and provisional end dates are enforced at admission; role changes and reinstatement cannot resurrect expired or revoked authority.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S11 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person granting temporary access, I want its end date and ancestor restrictions enforced, so role changes or reinstatement cannot silently extend it.

### R5: Expose one authenticated grant and explanation seam

THE SYSTEM SHALL expose typed operations for list/read, delegate, revoke and explain through the standalone identity server, all calling the same authority owner. API/tool access SHALL enforce the same checks as the UI; browser controls are presentation only. A why-permitted response SHALL identify actual authority path, responsible person, effective scope and policy revision; a why-refused response SHALL name the blocking condition without disclosing another identity's protected records. The reverse question, who can exercise this action on this resource, SHALL use the same evaluator and revision, with its own visibility permission. Incomplete pagination SHALL remain explicit. A last-use value reports observed use at named enforcement points; absence is not evidence of never-used. No live Cambium, Aion, Argus or Manifold is required. Conformance 8.1, 8.2, 8.4 and ADR-004.

**Acceptance:**
- GRANT_API_AUTH: unauthenticated, wrong issuer/subject, use-only and unauthorised-revoke requests are named refusals and create zero mutations. Exercise valid controls and count each route, rather than testing only button visibility.
- GRANT_EXPLAIN: the forward decision and reverse enumeration agree for the same resource/action/revision; paging returns every authorised holder exactly once and names continuation. Hidden private grants do not leak IDs, labels or existence to an unauthorised querier.
- GRANT_LAST_USED: an unobserved grant is labelled not seen; a recorded use reports its source/time; a missing reporting source is distinguished from a zero count. Reopen preserves the attribution.
- GRANT_STANDALONE: execute all operations against disposable local dependencies with other Ablative servers absent. Permission-engine outage is a named refusal, never an implicit standalone permit.

**Files:**
- create: crates/lys-identity-server/src/grants.rs
- create: crates/lys-identity-server/src/grant_contract.rs
- create: crates/lys-identity-server/tests/grants.rs
- create: crates/lys-identity-server/tests/grant_explanations.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C27 — Typed browser/API/agent seams use one authenticated evaluator; forward and reverse explanations name the same authority path and revision without private-record leakage.
- C28 — Observed grant usage names its source and time; not seen is not reported as never used.

**Stories:**
- S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.
- S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

### R6: Implement the You and delegation screens from the server contract

THE SYSTEM SHALL render the accepted mock-up's You page and delegation form from the server's generated grant types and decisions. Show sign-in identities separately from service access, source grants, effective operations/resource, affirmative pass-on rights and inherited end boundary. The form SHALL explain refused choices the caller is allowed to discover; it SHALL NOT enumerate other people's private secrets merely to say no. Personal views are scoped to the signed-in person, while an independently authorised directory administrator may inspect the wider directory. A failed or uncertain mutation SHALL remain refused or pending, with the original operation ID, never an optimistic grant or a new retry. The access graph consumes the same explanation response; its rendering belongs to Archie's graph brief. No simulated confirmation timer or sample-data authority calculation ships. Conformance 1.3–1.5, 2.3–2.4, 8.1 and 8.4.

**Acceptance:**
- GRANT_SCREEN: using two test people and their agents, render the source grant, action/resource scope, pass-on decision and inherited expiry from fixture IDs. Switching the signed-in person changes the personal data; permitted administrator inspection remains possible through its separate route.
- GRANT_SCREEN_REFUSAL: a use-only source, excessive requested scope, people-only policy, expired ancestor and service outage are refused visibly with the server reason; the request count and record count prove no hidden mutation happened.
- GRANT_SCREEN_PENDING: withhold the acknowledgement after durable delegation. The page keeps the original operation pending, retries by that ID, and eventually shows one grant; it never changes to success on a timer.
- GRANT_CONFORMANCE: pin the accepted mock-up file hash and numbered conformance rows in the test evidence. Exercise keyboard operation and deep linking as well as API refusal parity; sample data and visual similarity alone are not a passing acceptance.

**Files:**
- create: surface/identity/src/features/grants/YouGrants.tsx
- create: surface/identity/src/features/grants/DelegateGrant.tsx
- create: surface/identity/src/features/grants/GrantExplanation.tsx
- create: surface/identity/tests/grants.test.tsx
- create: surface/identity/tests/acceptance/grants.spec.ts
- modify: surface/identity/src/routes.tsx
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C29 — The You and delegation screens show real server authority, separate service accounts from sign-in identities, and preserve personal versus administrator visibility.
- C30 — The accepted mock-up conformance is tested through actual requests, refusals, pending outcomes, keyboard paths and durable read-back rather than simulated success.

**Stories:**
- S8 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want to give my agent a bounded part of my authority and revoke it, so the same action is allowed before revocation and refused after it.
- S9 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person or agent with use-only permission, I want the service to explain why I cannot pass it on; with explicit pass-on rights I want the same permitted operation available through UI and tools.
- S12 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As an authorised reviewer, I want to ask why this identity can act and who can reach a resource using the same current decision, without exposing records I may not inspect.

## Boundaries

- A missing file or unreviewed integration seam is a named dispatch blocker; do not silently expand a row wall or label an empty integration ready.
- No real credentials, live operator account mutation or production service writes in tests; use disposable local providers and permission fixtures.
- Every mutation seam is typed and generated; UI and agent callers use the same server checks. Do not infer authority from email, display name, mock-up data or absence of a deny flag.
- The same grant cannot acquire more authority through delegation, a secret handle, role migration, retries or restart. Use, delegation and permission to inspect another identity are distinct.
- The source/mock-up is a conformance specification, not implementation evidence. A test or source-only review never substitutes for the required live demonstration.
- No code in lys-core or published wire format changes in this brief. A capability format decision requires its own independent adversarial review before durable signing.
- Waffles owns board placement and workflow connection review. Create a blocked card with this exact brief path; do not move it into a dispatching status until its named blockers and row review are resolved.

## Verification

- From docs/: python3 "$DS2_METHOD/scripts/validate.py" design/directory and python3 "$DS2_METHOD/scripts/check-coverage.py" design/directory. Render with render-cluster.py and verify a second render is byte-identical.
- At implementation time run the repository battery from the exact revision: fmt, strict default/all-feature Clippy, feature-full tests and both rustdoc shapes. Run the granted/refused matrix and report each exercised leg, fault ordinal and unrun check.
- After the relevant screen row lands, demonstrate sign-in, agent registration, permitted action, source revocation and refusal to Tom using installed artifact hashes; record Melbourne time and exact authority/operation/receipt IDs. This human verification is not an automated acceptance criterion.
- Review the Cambium card: Card key resolves this exact JSON brief; project board rule connects to the supported deployed workflow; blocker/status prevents premature dispatch. No live test run or duplicate submission is used merely to inspect the configuration.


---
type: brief
id: DIRECTORY-008
cluster: directory
title: Grant brief residue after PR 6
---

# DIRECTORY-008: Grant brief residue after PR 6

> **Cluster:** directory
> **Depends on:** DIRECTORY-001
> **Blocked by:** Sign-off of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it)
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> **Checklist:**
> - C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
> - C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task.
> - C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.
> **Stories:**
> - S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

## Purpose

Three small items the 25 September grant-brief patch carries that main lacks after PR 6 (5593354) landed DIRECTORY-001 to DIRECTORY-006: the DIRECTORY-005 verification line for the frontend checks (R1), three design.json inventory rows for vendor/rauthy, crates/lys and docs/design/decisions.json beside the eight rows main lists (R2), and one design.json intention sentence recording that Cambium later uses this issuer keeping its participant ids, while the non-goal for rows 06 and 07 stands (R3). Main wins wherever the patch contradicts it on ADR ids and lifecycle; nothing here reopens a settled row. Documents only under docs/design/directory; no code; no new ADR; no change to any row's identifiers, estimates or dependency order.

## Task

Make the three document edits R1 to R3 on the documents as main holds them at 1756688cc08169bef4a9ac37b9b079efb9e38b18 (docs/design/directory/briefs/DIRECTORY-005.json:114-119, docs/design/directory/design.json:4 and docs/design/directory/design.json:580-613), re-render the cluster so each rendered twin matches its JSON, and pass the gate. Estimate: 1 focused implementer hour (R1 0.25, R2 0.5, R3 0.25), a document increment outside CN7's 48-hour ceiling, which stands unchanged; no row's estimate changes, DIRECTORY-005 keeps its 6 hours. Every path is relative to the repository root (CN3).

Ledger. Under the ruling of 26 September at 20:31 that a brief's ids are the next after main's highest and every open brief branch's, this brief is DIRECTORY-008 at docs/design/directory/briefs/DIRECTORY-008.json and its roadmap row is RM-016 in docs/design/roadmap.json, appended after RM-015, which the audit receipt card's brief DIRECTORY-007 takes in the run fired beside this one; main's ledger ends at RM-010 (docs/design/roadmap.json:276) and RM-011 to RM-015 are taken on open brief branches, so RM-016 is appended as the next row with every row main holds unchanged. It records no new decision: docs/design/decisions.json is not touched. Per the settled answer B1 (round 1 of run a68a7767, 22:49): RM-016 is the brief's own ledger entry, which every brief on main has, written by the brief method the gate runs, as the brief file is written under docs/design/directory/briefs; the boundary 'documents only under docs/design/directory' governs the work R1 to R3 do when built, not the brief's own files; RM-016 is appended with every row main holds unchanged, and no other file outside docs/design/directory changes. The two structure rows for this brief's own files in design.json are the method's listing of the brief, as DIRECTORY-006's rows are (docs/design/directory/design.json:414-423), and are written on the brief branch, not by the build.

Coverage. C1, C3, C6 and S2 are claimed by DIRECTORY-001, which wrote these documents, and by this brief, which completes them with the three items; the split is noted here, as check-coverage.py allows, and is reported as a warning, not a failure.

Dev note. The build starts from the main this brief lands on, which may be later than 1756688 if DIRECTORY-007 lands first; the counts in R1 to R3 are stated against 1756688 and hold, because that brief's words touch neither DIRECTORY-005's verification nor design.json's inventory or intention. Read the four files first. Make each JSON edit as an append that keeps every existing member byte for byte: json.load, append, json.dump with indent=2 and ensure_ascii=False plus one trailing newline reproduces main's formatting exactly (checked at 1756688 for both files by a round trip). Then run python3 scripts/design/render-cluster.py docs/design/directory and commit the rendered DESIGN.md and DIRECTORY-005.md beside their JSON; the gate compares a fresh render against the committed markdown byte for byte. Run bash scripts/design/gate.sh last and commit the four files by exact path.

## Requirements

### R1: Add the frontend verification line to DIRECTORY-005

THE SYSTEM SHALL append to the verification array of docs/design/directory/briefs/DIRECTORY-005.json (docs/design/directory/briefs/DIRECTORY-005.json:114-119, four entries at 1756688cc08169bef4a9ac37b9b079efb9e38b18, the fourth being the ID001_DIRECTORY_LIVE hold point) one fifth and last entry, exactly: 'Frontend checks pass with strict types and the generated API schema, with negative and crash-boundary tests.' The four entries main holds SHALL stay byte for byte and in their order. No other member of DIRECTORY-005 changes: not its id, depends_on, blocked_by, checklist, stories, design_anchor, purpose, task with its 6-hour estimate, requirements or boundaries. THE SYSTEM SHALL re-render docs/design/directory/briefs/DIRECTORY-005.md with python3 scripts/design/render-cluster.py docs/design/directory so its Verification section (docs/design/directory/briefs/DIRECTORY-005.md:103-108 at 1756688cc08169bef4a9ac37b9b079efb9e38b18) lists the five entries.

**Acceptance:**
- From the repository root: python3 -c "import json; print(len(json.load(open('docs/design/directory/briefs/DIRECTORY-005.json'))['verification']))" prints 5 (it prints 4 at 1756688cc08169bef4a9ac37b9b079efb9e38b18), and the first four entries equal, string for string, the four that git show 1756688cc08169bef4a9ac37b9b079efb9e38b18:docs/design/directory/briefs/DIRECTORY-005.json holds.
- From the repository root: rg -c 'strict types and the generated API schema' docs/design/directory/briefs/DIRECTORY-005.json prints 1, and the same command over docs/design/directory/briefs/DIRECTORY-005.md prints 1; both print nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18.
- From the repository root, on the build branch: git diff --no-ext-diff --numstat <base>..HEAD -- docs/design/directory/briefs/DIRECTORY-005.json prints 2 added and 1 deleted (the new entry's line and the fourth entry's line gaining its trailing comma), where <base> is the commit the build started from.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/briefs/DIRECTORY-005.json')); print(d['task'].count('6 focused implementer hours'), d['depends_on'])" prints 1 ['DIRECTORY-004'], as at 1756688cc08169bef4a9ac37b9b079efb9e38b18.

**Files:**
- modify: docs/design/directory/briefs/DIRECTORY-005.json
- modify: docs/design/directory/briefs/DIRECTORY-005.md

**Checklist:**
- C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task.
- C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.

**Stories:**
- S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. The verification array has 5 entries, and the first four were left untouched by the append (the dump round trip is byte-identical, checked before editing). Accept 2: met. rg -c prints 1 for docs/design/directory/briefs/DIRECTORY-005.json (line 119) and 1 for DIRECTORY-005.md (line 109). Accept 3: met. The working-tree git diff --numstat for DIRECTORY-005.json is '2 1': the new line plus the fourth entry gaining its comma. Accept 4: met. The check prints 1 ['DIRECTORY-004'].
- Deviation: I ran python3 scripts/design/render-cluster.py docs/design/directory once, because R1 to R3 require the re-render and it is the only way to produce the rendered twins. I ran no other listed check, build, lint or test, and made no commit, as instructed.
- Files changed:
  - modified: `docs/design/directory/briefs/DIRECTORY-005.json` — The verification array now holds five entries. The fifth, at line 119, is 'Frontend checks pass with strict types and the generated API schema, with negative and crash-boundary tests.' Every other member is unchanged.
  - modified: `docs/design/directory/briefs/DIRECTORY-005.md` — Re-rendered. The Verification section lists the five entries, with the new one at line 109.
- Checklist delivery:
  - [x] C3 — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task. — DIRECTORY-005 keeps its id, depends_on, estimate and ID001 identifiers. Only the verification line was added.
  - [x] C6 — The rendered markdown of this cluster matches its JSON and coverage is clean. — The rendered twin was regenerated from the JSON.
- Story delivery:
  - [x] S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria. — The brief now carries the frontend verification line for review.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] From the repository root: python3 -c "import json; print(len(json.load(open('docs/design/directory/briefs/DIRECTORY-005.json'))['verification']))" prints 5 (it prints 4 at 1756688cc08169bef4a9ac37b9b079efb9e38b18), and the first four entries equal, string for string, the four that git show 1756688cc08169bef4a9ac37b9b079efb9e38b18:docs/design/directory/briefs/DIRECTORY-005.json holds. — Measured: 4 at 1756688, 5 now. verification[:4] equals the base list (True). No other member differs from base (empty set).
  - [x] From the repository root: rg -c 'strict types and the generated API schema' docs/design/directory/briefs/DIRECTORY-005.json prints 1, and the same command over docs/design/directory/briefs/DIRECTORY-005.md prints 1; both print nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — rg -c prints 1 for DIRECTORY-005.json (line 119) and 1 for DIRECTORY-005.md (line 109). The phrase is absent at base.
  - [x] From the repository root, on the build branch: git diff --no-ext-diff --numstat <base>..HEAD -- docs/design/directory/briefs/DIRECTORY-005.json prints 2 added and 1 deleted (the new entry's line and the fourth entry's line gaining its trailing comma), where <base> is the commit the build started from. — git diff --no-ext-diff --numstat against base 3c1a3b9 (working tree, not yet committed) prints '2	1	docs/design/directory/briefs/DIRECTORY-005.json'.
  - [x] From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/briefs/DIRECTORY-005.json')); print(d['task'].count('6 focused implementer hours'), d['depends_on'])" prints 1 ['DIRECTORY-004'], as at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — Prints 1 ['DIRECTORY-004'].
- Checklist verified: C3, C6
- Stories verified: S2

### R2: Add the three inventory rows beside what main lists

THE SYSTEM SHALL append three rows to the inventory array of docs/design/directory/design.json (docs/design/directory/design.json:580-613, eight rows at 1756688cc08169bef4a9ac37b9b079efb9e38b18, ending with docs/design/identity/RAUTHY-BASELINE.md at docs/design/directory/design.json:606 and docs/design/identity/CONFORMANCE.md at docs/design/directory/design.json:610), after the eight and in this order, each with its path and a note of what exists there at 1756688cc08169bef4a9ac37b9b079efb9e38b18: (1) path 'vendor/rauthy', note 'the maintained Rauthy fork (ADR-009), the git submodule pinned at dd61ac3c84d6b238108dc8438b53043b5177a662, the upstream v0.36.2 commit the ablative branch was created from; DIRECTORY-004 moves the pin (structure row); read here, never changed by a document row'; (2) path 'crates/lys', note 'the lys CLI crate: Cargo.toml, src/main.rs, src/cli.rs and src/commands/ (attest, ca, key, log, inspect, files); DIRECTORY-002 adds src/identity/ and the identity subcommand to it (structure rows)'; (3) path 'docs/design/decisions.json', note 'the project decision ledger, ADR-001 to ADR-018 at main, holding the decisions this cluster cites (ADR-003, ADR-004, ADR-005, ADR-007 to ADR-011); DIRECTORY-001 recorded that it gained the identity decisions (structure row); read here, never changed by a document row'. The eight rows main lists SHALL stay byte for byte and in their order. No structure row, principle, goal, non-goal, constraint or gate leg changes. THE SYSTEM SHALL re-render docs/design/directory/DESIGN.md with python3 scripts/design/render-cluster.py docs/design/directory so its Inventory section (docs/design/directory/DESIGN.md:167-176 at 1756688cc08169bef4a9ac37b9b079efb9e38b18) lists the eleven rows.

**Acceptance:**
- From the repository root: python3 -c "import json; print(len(json.load(open('docs/design/directory/design.json'))['inventory']))" prints 11 (it prints 8 at 1756688cc08169bef4a9ac37b9b079efb9e38b18), and the first eight rows equal, member for member, the eight that git show 1756688cc08169bef4a9ac37b9b079efb9e38b18:docs/design/directory/design.json holds.
- From the repository root: rg -c '"path": "vendor/rauthy"' docs/design/directory/design.json prints 2 (1 at 1756688cc08169bef4a9ac37b9b079efb9e38b18, the structure row at docs/design/directory/design.json:380); rg -c '"path": "crates/lys"' docs/design/directory/design.json prints 1 (nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18); rg -c '"path": "docs/design/decisions.json"' docs/design/directory/design.json prints 2 (1 at 1756688cc08169bef4a9ac37b9b079efb9e38b18, the structure row at docs/design/directory/design.json:140).
- From the repository root: rg -c '^- `vendor/rauthy`' docs/design/directory/DESIGN.md prints 1; rg -c '^- `crates/lys`' docs/design/directory/DESIGN.md prints 1; rg -c '^- `docs/design/decisions.json`' docs/design/directory/DESIGN.md prints 1; each prints nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/design.json')); print([r['path'] for r in d['inventory'][8:]])" prints ['vendor/rauthy', 'crates/lys', 'docs/design/decisions.json'], and rg -c 'RAUTHY-BASELINE.md"|CONFORMANCE.md"' docs/design/directory/design.json prints 2, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18.

**Files:**
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
- C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.

**Stories:**
- S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. len(inventory) is 11, and the first eight rows were left unchanged by the append. Accept 2: met. '"path": "vendor/rauthy"' counts 2 (lines 380 and 624), '"path": "crates/lys"' counts 1 (line 628), and '"path": "docs/design/decisions.json"' counts 2 (lines 140 and 632). Accept 3: met. Each of the three '^- `...`' patterns matches once in DESIGN.md, at lines 179, 180 and 181. Accept 4: met. inventory[8:] paths are ['vendor/rauthy', 'crates/lys', 'docs/design/decisions.json'], and the RAUTHY-BASELINE and CONFORMANCE rows were not touched, so that count stays 2.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/directory/design.json` — The inventory now holds eleven rows. Rows 9 to 11 (lines 624, 628 and 632) are vendor/rauthy, crates/lys and docs/design/decisions.json, each with the note exactly as specified. The intention also changed, under R3.
  - modified: `docs/design/directory/DESIGN.md` — Re-rendered. The Inventory section lists the eleven rows, with the new three at lines 179 to 181.
- Checklist delivery:
  - [x] C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling. — The inventory now also records the fork, the CLI crate and the decision ledger. No other design member changed.
  - [x] C6 — The rendered markdown of this cluster matches its JSON and coverage is clean. — DESIGN.md was re-rendered from design.json.
- Story delivery:
  - [x] S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria. — Reviewers can see what the rows read and move.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] From the repository root: python3 -c "import json; print(len(json.load(open('docs/design/directory/design.json'))['inventory']))" prints 11 (it prints 8 at 1756688cc08169bef4a9ac37b9b079efb9e38b18), and the first eight rows equal, member for member, the eight that git show 1756688cc08169bef4a9ac37b9b079efb9e38b18:docs/design/directory/design.json holds. — Measured: 8 at base, 11 now. inventory[:8] equals the base inventory (True).
  - [x] From the repository root: rg -c '"path": "vendor/rauthy"' docs/design/directory/design.json prints 2 (1 at 1756688cc08169bef4a9ac37b9b079efb9e38b18, the structure row at docs/design/directory/design.json:380); rg -c '"path": "crates/lys"' docs/design/directory/design.json prints 1 (nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18); rg -c '"path": "docs/design/decisions.json"' docs/design/directory/design.json prints 2 (1 at 1756688cc08169bef4a9ac37b9b079efb9e38b18, the structure row at docs/design/directory/design.json:140). — The three rg -c commands print 2, 1 and 2.
  - [x] From the repository root: rg -c '^- `vendor/rauthy`' docs/design/directory/DESIGN.md prints 1; rg -c '^- `crates/lys`' docs/design/directory/DESIGN.md prints 1; rg -c '^- `docs/design/decisions.json`' docs/design/directory/DESIGN.md prints 1; each prints nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — Each prints 1, at DESIGN.md:179-181, as the rendered Inventory shows.
  - [x] From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/design.json')); print([r['path'] for r in d['inventory'][8:]])" prints ['vendor/rauthy', 'crates/lys', 'docs/design/decisions.json'], and rg -c 'RAUTHY-BASELINE.md"|CONFORMANCE.md"' docs/design/directory/design.json prints 2, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — Prints ['vendor/rauthy', 'crates/lys', 'docs/design/decisions.json'], and the rg count is 2. Each new row's note equals the spec text exactly, and its keys are exactly {path, note}. The notes' facts hold: git ls-tree shows the submodule at dd61ac3c84d6b238108dc8438b53043b5177a662, crates/lys holds the named files, and the last ADR is ADR-018.
- Checklist verified: C1, C6
- Stories verified: S2

### R3: Record the Cambium direction in the intention while the non-goal stands

THE SYSTEM SHALL append one sentence to the intention of docs/design/directory/design.json (docs/design/directory/design.json:4 at 1756688cc08169bef4a9ac37b9b079efb9e38b18, one sentence ending 'every identity change.'), after one space, exactly: 'Cambium later uses this issuer, keeping its participant ids (rows 06 and 07); that direction is recorded here while rows 06 and 07 stay the non-goal recorded below, and this sentence promises neither row.' The non-goal for rows 06 and 07 (docs/design/directory/design.json:63-66, text 'Rows 06 (connect Cambium) and 07 (gate, install and demonstrate the release)' with its reason) SHALL stay byte for byte; no other member of the intention, and no goal, principle, non-goal, structure row, constraint or gate leg, changes. THE SYSTEM SHALL re-render docs/design/directory/DESIGN.md with python3 scripts/design/render-cluster.py docs/design/directory so its Intention section (docs/design/directory/DESIGN.md:11-13 at 1756688cc08169bef4a9ac37b9b079efb9e38b18) carries the sentence.

**Acceptance:**
- From the repository root: rg -c 'keeping its participant ids' docs/design/directory/design.json prints 1 and the same command over docs/design/directory/DESIGN.md prints 1; both print nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18.
- From the repository root: python3 -c "import json; i=json.load(open('docs/design/directory/design.json'))['intention']; print(i.startswith('An operator installs the identity product without Cambium or Manifold, signs in,') and i.endswith('and this sentence promises neither row.'))" prints True.
- From the repository root: rg -c 'Rows 06 \(connect Cambium\) and 07 \(gate, install and demonstrate the release\)' docs/design/directory/design.json prints 1, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18, and python3 -c "import json; print(len(json.load(open('docs/design/directory/design.json'))['non_goals']))" prints 11, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/design.json')); print(len(d['goals']), len(d['principles']), len(d['constraints']))" prints 5 9 12, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18.

**Files:**
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
- C6 — The rendered markdown of this cluster matches its JSON and coverage is clean.

**Stories:**
- S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. 'keeping its participant ids' counts 1 in design.json (line 4) and 1 in DESIGN.md. Accept 2: met. The intention starts with the operator sentence and ends with 'and this sentence promises neither row.' Accept 3: met. The non-goals were not edited, so the Rows 06/07 text is still present and non_goals still has 11 entries (measured). Accept 4: met. goals, principles and constraints measure 5, 9 and 12.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/directory/design.json` — At line 4 the intention now ends with one space and then the exact Cambium sentence. The non-goals were not changed.
  - modified: `docs/design/directory/DESIGN.md` — Re-rendered. The Intention section (line 13) carries the sentence.
- Checklist delivery:
  - [x] C1 — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling. — The outcome now records the Cambium direction, and the non-goal is kept.
  - [x] C6 — The rendered markdown of this cluster matches its JSON and coverage is clean. — DESIGN.md was re-rendered.
- Story delivery:
  - [x] S2 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want each open identity row as a design-system brief with numbered requirements and criteria, so that rows can be dispatched to the loop one at a time and reviewed against their criteria. — The direction is recorded without promising any row.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] From the repository root: rg -c 'keeping its participant ids' docs/design/directory/design.json prints 1 and the same command over docs/design/directory/DESIGN.md prints 1; both print nothing at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — Prints 1 for design.json and 1 for DESIGN.md.
  - [x] From the repository root: python3 -c "import json; i=json.load(open('docs/design/directory/design.json'))['intention']; print(i.startswith('An operator installs the identity product without Cambium or Manifold, signs in,') and i.endswith('and this sentence promises neither row.'))" prints True. — Prints True. The intention also equals the base intention + ' ' + the exact spec sentence (True).
  - [x] From the repository root: rg -c 'Rows 06 \(connect Cambium\) and 07 \(gate, install and demonstrate the release\)' docs/design/directory/design.json prints 1, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18, and python3 -c "import json; print(len(json.load(open('docs/design/directory/design.json'))['non_goals']))" prints 11, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — rg -c prints 1, and len(non_goals) is 11. non_goals equals the base value (True).
  - [x] From the repository root: python3 -c "import json; d=json.load(open('docs/design/directory/design.json')); print(len(d['goals']), len(d['principles']), len(d['constraints']))" prints 5 9 12, as at 1756688cc08169bef4a9ac37b9b079efb9e38b18. — Prints 5 9 12. Against HEAD, only intention and inventory differ in design.json.
- Checklist verified: C1, C6
- Stories verified: S2

## Boundaries

- Documents only under docs/design/directory: the build changes exactly the four files R1 to R3 name (docs/design/directory/briefs/DIRECTORY-005.json, docs/design/directory/briefs/DIRECTORY-005.md, docs/design/directory/design.json, docs/design/directory/DESIGN.md) and nothing else; no code; no file under crates/, surface/, deploy/, vendor/, scripts/ or tests/; docs/design/decisions.json is not touched either, narrower than CN1.
- No new ADR: docs/design/decisions.json is unchanged; this brief's design_anchor cites decisions that exist at 1756688cc08169bef4a9ac37b9b079efb9e38b18 and it records no decision.
- No change to any row's identifiers, estimates or dependency order: every brief id, roadmap id, ADR id, C, S, R and ID001 identifier, every estimate (DIRECTORY-005's 6 hours, CN7's ceiling) and every depends_on array stays as main holds it.
- Main wins wherever the 25 September patch contradicts it on ADR ids and lifecycle: nothing from the patch beyond the three items is carried, and no settled row reopens.
- The non-goal for rows 06 and 07 stands: the intention sentence records a direction and promises no row, no Cambium cluster and no cross-repository arrangement.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- The roadmap row RM-016 in docs/design/roadmap.json is this brief's own ledger entry, written on the brief branch by this brief and never by the build; per the settled answer B1 of 22:49 it stands outside the 'documents only under docs/design/directory' boundary, which governs the build's work (R1 to R3). The build changes no file outside docs/design/directory.
- No credential, token or key value is written in any document.
- No row is dispatched until this brief is signed off.

## Verification

- From the repository root: bash scripts/design/gate.sh exits 0 (scripts/design/gate.sh: validate.py over docs/design/decisions.json, docs/design/project.json and every cluster, check-coverage.py over every cluster, and render-cluster.py into a temporary copy compared byte for byte with the committed markdown).
- From the repository root, on the build branch: git diff --no-ext-diff --name-only <base>..HEAD prints exactly four paths, docs/design/directory/DESIGN.md, docs/design/directory/briefs/DIRECTORY-005.json, docs/design/directory/briefs/DIRECTORY-005.md and docs/design/directory/design.json, where <base> is the commit the build started from; git diff --no-ext-diff --name-only <base>..HEAD -- . ':!docs/design/directory' prints nothing.
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0; the coverage report may list C1, C3, C6 and S2 as claimed by DIRECTORY-001 and DIRECTORY-008, a warning whose split this brief's task notes, and lists no failure.
- From the repository root: python3 scripts/design/render-cluster.py docs/design/directory run a second time changes no file (git status --porcelain docs/design/directory prints nothing after it).


---
type: brief
id: DIRECTORY-019
cluster: directory
title: Make CONFORMANCE rows 3.1 and 3.2 true on the lifecycle record: every agent answers to a person, an agent whose person is retired is flagged on read as needing a new person, and the four states are authority only
---

# DIRECTORY-019: Make CONFORMANCE rows 3.1 and 3.2 true on the lifecycle record: every agent answers to a person, an agent whose person is retired is flagged on read as needing a new person, and the four states are authority only

> **Cluster:** directory
> **Depends on:** DIRECTORY-003, DIRECTORY-009
> **Blocked by:** The lifecycle-hand brief DIRECTORY-009 (roadmap row RM-017, branch brief/directory/lifecycle-hand) signed off and landed on main with ADR-027 and ADR-028, because this brief reads that brief's lifecycle record, its fold (its R3, crates/lys-identity/src/lifecycle/fold.rs and state.rs), its cause rule (its R2, crates/lys-identity/src/lifecycle/causes.rs), its typed read (its R6, crates/lys-identity-server/src/lifecycle_read.rs) and its contract (its R1, docs/design/identity/LIFECYCLE-CONTRACT.md), and keeps no copy of any of them. Check: git ls-tree -r --name-only origin/main crates/lys-identity/src/lifecycle crates/lys-identity-server/src/lifecycle_read.rs docs/design/identity/LIFECYCLE-CONTRACT.md prints fold.rs, state.rs, causes.rs, error.rs, mod.rs, lifecycle_read.rs and LIFECYCLE-CONTRACT.md. Until then no row starts, R1 included, because R1 adds to that contract., The lead's sign-off of this brief before any row is dispatched (DIRECTORY-001 boundary), which is also the manifest review CN9 asks for of the four files this brief adds under crates/lys-identity/ and crates/lys-identity-server/ and of the added lines it names in the lifecycle-hand brief's files. Check: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory both exit 0, and the card that carries this brief records the sign-off before card_build_v3 runs.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-056 — Needs a new person is a view worked out on read from an agent's registration and its person's retire record, never stored or appended — An agent's needs-a-new-person flag is worked out each time its lifecycle record is read, from the responsible person its registration names and that person's retire record: it is true when that person's folded state is retired and false otherwise, a suspended person included. Nothing is appended to any agent's history, no transition is made on the agent, the agent keeps its own state, and no responsible person changes. The flag records a fact, that the agent's person is retired; clearing it needs the reassignment card, which is the one that revises ADR-011's responsible person for life. Rejected: a signed flag record appended on each agent when its person is retired; moving the agent to another state with its person.
> **Checklist:**
> - C123 — docs/design/identity/LIFECYCLE-CONTRACT.md maps CONFORMANCE rows 3.1 and 3.2 to named tests and states the needs-a-new-person rule, that ADR-011 is not revised, and that the state is authority only.
> - C124 — Registering an agent whose register request names no responsible person is refused lifecycle_agent_without_person and appends nothing.
> - C125 — An agent's lifecycle read carries needs_a_new_person, worked out on read, true on every one of a person's agents once that person is retired and false while the person is suspended, with no record appended and each agent's state unchanged.
> - C126 — A walk of all 20 origin-by-target changes of state admits exactly the 6 in the table and refuses the other 14 by name with nothing appended.
> - C127 — A change of state starts and stops no process, and no key of the typed lifecycle read says anything about running.
> **Stories:**
> - S52 (Operator, Installs and runs the standalone identity product) — As an operator, I want every agent whose person is retired to show that it needs a new person, so that no agent is left answering to a retired person without anyone seeing it.
> - S53 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want an identity's state to move only along the lifecycle table and to say nothing about running, so that a state is read as authority and never mistaken for whether a process is alive.

## Purpose

CONFORMANCE section 3 (docs/design/identity/CONFORMANCE.md:36-37) holds two lifecycle rows no brief on main makes true. Row 3.1: every agent answers to a person, and an agent whose person is retired is flagged as needing a new one. Row 3.2: the identity states are registered, active, suspended and retired, and the state is authority only and says nothing about running. This brief makes both rows true on the lifecycle record the lifecycle-hand brief DIRECTORY-009 (RM-017) defines, reading its fold and its typed read and keeping no copy of either: an agent recorded with no person is refused by name; an agent's read carries a needs-a-new-person flag worked out each time it is read from its registration and its person's retire record (ADR-056); every change of state is walked, the six in the table admitted and the fourteen others refused by name; and a test shows a change of state starts and stops no process and that the read says nothing about running (ADR-007, ADR-004). Each row maps to at least one acceptance line that tests it on the lifecycle record: row 3.1 to R2 and R3, row 3.2 to R4 and R5.

## Task

Five rows, in order: R1 the conformance section of the lifecycle contract (1 hour), R2 the named refusal of an agent recorded with no person (2 hours), R3 the needs-a-new-person flag worked out on read (3 hours), R4 the walk of every change of state (2 hours), R5 the authority-only test (2 hours): 10 focused implementer hours. R1 is documents only; R2 to R5 stand on files the lifecycle-hand brief DIRECTORY-009 (RM-017) creates, and every such file is touched only by the added lines a row names (CN9).

The lead's answers are written in as settled. (1) The flag stands, and ADR-011's responsible person for life stays in force: this brief does not revise ADR-011 or DIRECTORY-003 R1. The flag records a fact: the agent's person is retired. Clearing it needs the reassignment card, which is the one that revises that rule. (2) The flag is computed on read, from the agent's registration and its person's retire record, with nothing appended to the agent: each agent's history stays as it is, and the lifecycle-hand brief's rule that retiring someone has no automatic effect on other identities except the grant check holds (ADR-056). (3) The agent's own state is unchanged: it stays in whatever state it was, for example active, and its derived grants refuse at the check, as the lifecycle-states statement's rule that retiring the human makes every derived grant refuse at the next check already says; retiring the person moves no other identity, and this brief neither asserts nor duplicates that grant rule (DIRECTORY-006 R4). (4) Shows is met by the flag in the lifecycle read and in the lifecycle-hand brief's typed read (its R6); the screen that shows it on the people list and the agent file belongs to a later card. (5) Only an agent whose person is retired is flagged. What a suspension stops stays open, so a suspended person's agents are not flagged.

Split notes. DIRECTORY-003 R5 already refuses four named out-of-table transitions; R4's walk is its superset over all twenty origin-by-target pairs and adds no second table check: every refusal it counts is the one DIRECTORY-003 R5's commit path gives, or the lifecycle-hand brief's lifecycle_unknown_identity. DIRECTORY-003 R1 refuses a registration with no signed-in caller at the server; R2's refusal is of the record, an agent registration naming no responsible person whoever the caller is, and the two stay separate names.

Out: giving an agent a new person (its own card); the screen that shows the flag on the people list and the agent file (a later card); CONFORMANCE row 3.4.

## Requirements

### R1: Add the conformance section to the lifecycle contract: rows 3.1 and 3.2 mapped to their tests, the needs-a-new-person rule, and the state as authority only

Documents only. The contract is created by DIRECTORY-009 R1, on which this brief depends. THE SYSTEM SHALL add to docs/design/identity/LIFECYCLE-CONTRACT.md, after its existing sections, one section headed 'Conformance rows 3.1 and 3.2' holding, in this order: (1) a table with one row per conformance row, beginning '| 3.1 |' and '| 3.2 |', each naming the test files and test functions of this brief that test it on the lifecycle record: 3.1 to crates/lys-identity/tests/lifecycle_no_person.rs and crates/lys-identity/tests/lifecycle_needs_new_person.rs, 3.2 to crates/lys-identity/tests/lifecycle_walk.rs and crates/lys-identity-server/tests/lifecycle_authority.rs; (2) the needs-a-new-person rule: an agent's needs_a_new_person flag is worked out on read from the agent's registration and its responsible person's retire record, and never appended; it is true on every one of that person's agents when the person is retired and false otherwise, a suspended person included, because what a suspension stops stays open; the flag is not a state and not a transition, the agent keeps its own state, its derived grants refuse at the check, and no responsible person changes; (3) the statement that the flag records a fact, the agent's person is retired, that this contract does not revise ADR-011's responsible person for life or DIRECTORY-003 R1, and that clearing the flag needs the reassignment card, which is the one that revises that rule; (4) the sentence 'the state is authority only: it starts and stops no process and says nothing about whether an identity is running'. The section SHALL NOT change any existing line of the contract, SHALL NOT edit the lifecycle-states statement or CONFORMANCE.md, and SHALL NOT add a state or a transition. Estimate: 1 hour.

**Acceptance:**
- grep -c '^| 3.1 |\|^| 3.2 |' docs/design/identity/LIFECYCLE-CONTRACT.md prints 2.
- Each of grep -c 'needs_a_new_person', grep -c 'worked out on read', grep -c 'a suspended person included', grep -c 'clearing the flag needs the reassignment card' and grep -c 'the state is authority only' over docs/design/identity/LIFECYCLE-CONTRACT.md prints at least 1.
- git diff --numstat on the R1 commit for docs/design/identity/LIFECYCLE-CONTRACT.md prints a second number of 0, and git diff --stat on the R1 commit for docs/design/identity/CONFORMANCE.md and the lifecycle-states statement under docs/design/identity/ prints nothing.
- sh scripts/design/gate.sh exits 0 on the R1 commit.

**Files:**
- modify: docs/design/identity/LIFECYCLE-CONTRACT.md

**Checklist:**
- C123 — docs/design/identity/LIFECYCLE-CONTRACT.md maps CONFORMANCE rows 3.1 and 3.2 to named tests and states the needs-a-new-person rule, that ADR-011 is not revised, and that the state is authority only.

**Stories:**
- S52 (Operator, Installs and runs the standalone identity product) — As an operator, I want every agent whose person is retired to show that it needs a new person, so that no agent is left answering to a retired person without anyone seeing it.
- S53 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want an identity's state to move only along the lifecycle table and to say nothing about running, so that a state is read as authority and never mistaken for whether a process is alive.

### R2: Refuse by name an agent recorded with no responsible person, and append nothing

IF a register transition for an agent names no responsible person, THEN THE SYSTEM SHALL refuse it lifecycle_agent_without_person, whose message names the agent and says that every agent is registered under a responsible person, and SHALL append nothing. The check lives in crates/lys-identity/src/lifecycle/responsible.rs, the refusal name in crates/lys-identity/src/lifecycle/error.rs beside the lifecycle-hand brief's names, and it is called by one added line in crates/lys-identity/src/lifecycle/causes.rs on the register path, before the append. THE SYSTEM SHALL NOT fill an absent responsible person from the caller, the administrator or any other source, SHALL NOT apply the check to the registration of a person, and SHALL NOT change DIRECTORY-003 R1's refusal of a registration with no signed-in caller, which stays a separate refusal at the server. The four modified files are created by DIRECTORY-009 R2, on which this brief depends. No wall clock; every time value the test needs is injected through crates/lys-identity/tests/lifecycle_support/mod.rs. Estimate: 2 hours.

**Acceptance:**
- cf3_r2_ac1: the attested administrator registers an agent whose register request names no responsible person: 1 refusal named lifecycle_agent_without_person, its message containing 'responsible person', and the log's leaf count unchanged; the same request naming a registered test person P as responsible person is admitted, the leaf count rises by exactly 1, and the agent's registration record names P; the test asserts 1 refusal and 1 admission; cargo test -p lys-identity --test lifecycle_no_person cf3_r2_ac1 prints 'test result: ok. 1 passed; 0 failed'.
- cf3_r2_ac2: the attested administrator registers a person with no responsible person named: admitted, the leaf count rises by exactly 1, and no refusal named lifecycle_agent_without_person is returned; cargo test -p lys-identity --test lifecycle_no_person cf3_r2_ac2 prints 'test result: ok. 1 passed; 0 failed'.
- rg -c 'responsible::' crates/lys-identity/src/lifecycle/causes.rs prints 1, and git diff --numstat on the row's commits for crates/lys-identity/src/lifecycle/causes.rs prints a second number of 0.
- rg -c 'fn cf3_r2_ac(1|2)' crates/lys-identity/tests/lifecycle_no_person.rs prints 2, and on the row's first commit cargo test -p lys-identity --test lifecycle_no_person fails to compile.

**Files:**
- create: crates/lys-identity/src/lifecycle/responsible.rs
- create: crates/lys-identity/tests/lifecycle_no_person.rs
- modify: crates/lys-identity/src/lifecycle/mod.rs
- modify: crates/lys-identity/src/lifecycle/error.rs
- modify: crates/lys-identity/src/lifecycle/causes.rs
- modify: crates/lys-identity/tests/lifecycle_support/mod.rs

**Checklist:**
- C124 — Registering an agent whose register request names no responsible person is refused lifecycle_agent_without_person and appends nothing.

**Stories:**
- S52 (Operator, Installs and runs the standalone identity product) — As an operator, I want every agent whose person is retired to show that it needs a new person, so that no agent is left answering to a retired person without anyone seeing it.

### R3: Work out each agent's needs-a-new-person flag on read from its registration and its person's retire record, and move nothing

WHEN an agent's lifecycle record is read, THE SYSTEM SHALL work out its needs_a_new_person flag from the responsible person its registration record names and that person's folded state (the lifecycle-hand brief's fold, in which only the person's retire record makes it retired): true when that person is retired, whatever the agent's own state, and false when that person is registered, active or suspended. The derivation is one function in crates/lys-identity/src/lifecycle/responsible.rs over the lifecycle-hand brief's fold, and the typed read in crates/lys-identity-server/src/lifecycle_read.rs gains one field, needs_a_new_person, filled from that function by added lines and serialised as a JSON boolean. THE SYSTEM SHALL NOT append any record to work out or show the flag, SHALL NOT make a transition on the agent, SHALL NOT change the agent's state, SHALL NOT change the agent's responsible person, SHALL NOT store the flag, SHALL NOT make the flag a state, SHALL NOT flag an agent whose person is suspended, and SHALL NOT read DIRECTORY-006's grant decision to work it out. crates/lys-identity-server/src/lifecycle_read.rs is created by DIRECTORY-009 R6, on which this brief depends. No second store or projection of lifecycle state is added. Estimate: 3 hours.

**Acceptance:**
- cf3_r3_ac1 (row 3.1): person P, active, with agents A1 active, A2 suspended and A3 retired, and person Q, active, with agent B1 active; the attested administrator retires P; the reads answer A1 needs_a_new_person true with state active, A2 true with state suspended, A3 true with state retired, and B1 false with state active; the test asserts a flagged count of 3, equal to the count of P's agents, a flagged count of 0 among Q's agents, each agent's state equal to its state before the retire, each agent's history length equal to its length before the retire, and a log leaf count exactly 1 higher (the retire of P); cargo test -p lys-identity --test lifecycle_needs_new_person cf3_r3_ac1 prints 'test result: ok. 1 passed; 0 failed'.
- cf3_r3_ac2: person R, active, with agents C1 active and C2 registered: before any change both read needs_a_new_person false; the administrator suspends R and both read false with their states unchanged; the administrator reinstates R and both read false; the administrator retires R and both read true; the test asserts flagged counts 0, 0, 0 and 2 in that order, and each agent's history length unchanged across all three transitions of R; cargo test -p lys-identity --test lifecycle_needs_new_person cf3_r3_ac2 prints 'test result: ok. 1 passed; 0 failed'.
- cf3_r3_ac3: the test records the log's leaf count, the count of files under the log directory and the SHA-256 of each file, then reads every agent of cf3_r3_ac1 again, and asserts that the leaf count, the file count and each file's SHA-256 equal those recorded before the read; cargo test -p lys-identity --test lifecycle_needs_new_person cf3_r3_ac3 prints 'test result: ok. 1 passed; 0 failed'.
- rg -n '\.append\(|fn set_state|grants::' crates/lys-identity/src/lifecycle/responsible.rs prints nothing; rg -c 'needs_a_new_person' crates/lys-identity-server/src/lifecycle_read.rs prints at least 1; git diff --numstat on the row's commits for crates/lys-identity-server/src/lifecycle_read.rs prints a second number of 0.
- rg -c 'fn cf3_r3_ac(1|2|3)' crates/lys-identity/tests/lifecycle_needs_new_person.rs prints 3, and on the row's first commit cargo test -p lys-identity --test lifecycle_needs_new_person fails to compile.

**Files:**
- create: crates/lys-identity/tests/lifecycle_needs_new_person.rs
- modify: crates/lys-identity/src/lifecycle/responsible.rs
- modify: crates/lys-identity-server/src/lifecycle_read.rs
- modify: crates/lys-identity/tests/lifecycle_support/mod.rs

**Checklist:**
- C125 — An agent's lifecycle read carries needs_a_new_person, worked out on read, true on every one of a person's agents once that person is retired and false while the person is suspended, with no record appended and each agent's state unchanged.

**Stories:**
- S52 (Operator, Installs and runs the standalone identity product) — As an operator, I want every agent whose person is retired to show that it needs a new person, so that no agent is left answering to a retired person without anyone seeing it.

### R4: Walk every change of state on the lifecycle record: admit the six in the table and refuse the fourteen others by name

THE SYSTEM SHALL admit, through the lifecycle-hand brief's transition path over DIRECTORY-003 R5's commit path, exactly the six changes of state in the table: none to registered (register), registered to active (activate), active to suspended (suspend), suspended to active (reinstate), active to retired (retire) and suspended to retired (retire). IF any other change of an origin in none, registered, active, suspended and retired to a target in registered, active, suspended and retired is requested, THEN THE SYSTEM SHALL refuse it by name and append nothing: lifecycle_unknown_identity when the origin is none, and DIRECTORY-003 R5's out-of-table refusal otherwise. This row adds no table check, no refusal name and no source file: the walk is a test, crates/lys-identity/tests/lifecycle_walk.rs, that requests each of the twenty origin-by-target pairs once, self-changes included, as the attested administrator so no refusal is by actor, each pair through the operation the table records for its target (register for registered, activate for active except reinstate from suspended, suspend for suspended, retire for retired). THE SYSTEM SHALL NOT admit a retired identity to any other state, and SHALL NOT add a fifth state. Estimate: 2 hours.

**Acceptance:**
- cf3_r4_ac1 (row 3.2): setup, before the walk and outside it, registers and transitions the identities that give each origin state (a fresh id for none, and one identity each brought to registered, active, suspended and retired for every pair from that origin); the walk then requests the 20 pairs, and the test samples the log's leaf count immediately before each of the 20 requests and immediately after it; the test asserts exactly 20 requests, 6 admissions (the six pairs of the table, each folding to its target) and 14 refusals, a leaf count difference of exactly +1 across each of the 6 admissions and exactly 0 across each of the 14 refusals, and a sum of the 20 differences equal to 6; cargo test -p lys-identity --test lifecycle_walk cf3_r4_ac1 prints 'test result: ok. 1 passed; 0 failed'.
- cf3_r4_ac2: of the 14 refusals, the 3 whose origin is none are named lifecycle_unknown_identity; each of the other 11 carries a non-empty refusal name that is neither lifecycle_actor_not_permitted nor lifecycle_unknown_identity, and a message containing both its origin state's name and its target state's name; the test asserts the counts 3 and 11; cargo test -p lys-identity --test lifecycle_walk cf3_r4_ac2 prints 'test result: ok. 1 passed; 0 failed'.
- cf3_r4_ac3: the listing of every value of the lifecycle-hand brief's state type has length 4 and names registered, active, suspended and retired; cargo test -p lys-identity --test lifecycle_walk cf3_r4_ac3 prints 'test result: ok. 1 passed; 0 failed'.
- rg -c 'fn cf3_r4_ac(1|2|3)' crates/lys-identity/tests/lifecycle_walk.rs prints 3, and the row's commits change no file under crates/lys-identity/src: git diff --stat on them for crates/lys-identity/src prints nothing.

**Files:**
- create: crates/lys-identity/tests/lifecycle_walk.rs

**Checklist:**
- C126 — A walk of all 20 origin-by-target changes of state admits exactly the 6 in the table and refuses the other 14 by name with nothing appended.

**Stories:**
- S53 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want an identity's state to move only along the lifecycle table and to say nothing about running, so that a state is read as authority and never mistaken for whether a process is alive.

### R5: Show that a change of state starts and stops no process, and that the lifecycle read says nothing about running

WHEN an agent's state changes, THE SYSTEM SHALL record the transition and do nothing else: THE SYSTEM SHALL NOT start, stop, signal or ask any engine to start or stop a process, and the lifecycle code SHALL NOT reference a process API. THE SYSTEM SHALL keep the typed lifecycle read free of any field about running: no key of the read, at any depth, names running, an execution, a process, a pid, liveness or a heartbeat. The test counts the child processes of the test process around each transition, and lives in crates/lys-identity-server/tests/lifecycle_authority.rs; it carries a positive control proving its key matcher fires, so the absence it asserts cannot pass vacuously. This row changes no source file. Estimate: 2 hours.

**Acceptance:**
- cf3_r5_ac1 (row 3.2): the attested administrator registers, activates, suspends, reinstates and retires one agent, and after each of the 5 transitions the typed read of the agent is serialised to JSON; the key matcher, the case-insensitive pattern run|exec|process|pid|alive|heartbeat applied to every key at every depth, finds 0 matches in each of the 5 reads, and the same matcher applied to the fixture object {"running": true, "pid": 1} finds 2; the test asserts 5 reads, 0 matches and the control's 2; cargo test -p lys-identity-server --test lifecycle_authority cf3_r5_ac1 prints 'test result: ok. 1 passed; 0 failed'.
- cf3_r5_ac2: the same serialised read carries the key needs_a_new_person at its top level after each of the 5 transitions; cargo test -p lys-identity-server --test lifecycle_authority cf3_r5_ac2 prints 'test result: ok. 1 passed; 0 failed'.
- cf3_r5_ac3 (row 3.2): the attested administrator registers, activates, suspends, reinstates and retires one agent, and the test records the count of child processes of the test process immediately before and immediately after each of the 5 transitions; the test asserts 5 comparisons with 0 differences, each after-count equal to its before-count; cargo test -p lys-identity-server --test lifecycle_authority cf3_r5_ac3 prints 'test result: ok. 1 passed; 0 failed'.
- rg -n 'std::process|Command::new|tokio::process|process::exit|libc::kill|nix::sys::signal' crates/lys-identity/src/lifecycle crates/lys-identity-server/src/lifecycle_read.rs prints nothing, and the same pattern's positive control, printf 'Command::new\n' | rg -c 'std::process|Command::new|tokio::process|process::exit|libc::kill|nix::sys::signal', prints 1.
- rg -c 'fn cf3_r5_ac(1|2|3)' crates/lys-identity-server/tests/lifecycle_authority.rs prints 3, and the row's commits change no file under crates/lys-identity/src and no file under crates/lys-identity-server/src: git diff --stat on them for those two directories prints nothing.

**Files:**
- create: crates/lys-identity-server/tests/lifecycle_authority.rs

**Checklist:**
- C127 — A change of state starts and stops no process, and no key of the typed lifecycle read says anything about running.

**Stories:**
- S53 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want an identity's state to move only along the lifecycle table and to say nothing about running, so that a state is read as authority and never mistaken for whether a process is alive.

## Boundaries

- No way to give an agent a new person, and no change to any agent's responsible person: that is its own card, the one that revises ADR-011's responsible person for life and clears the flag. This brief revises neither ADR-011 nor DIRECTORY-003 R1.
- No copy of the lifecycle-hand brief's lifecycle record: no second store, projection or fold of lifecycle state. Its files are touched only by the added lines a row names: one line in crates/lys-identity/src/lifecycle/causes.rs (R2), added lines in crates/lys-identity/src/lifecycle/error.rs and mod.rs (R2), crates/lys-identity/tests/lifecycle_support/mod.rs (R2, R3), crates/lys-identity-server/src/lifecycle_read.rs (R3) and one added section of docs/design/identity/LIFECYCLE-CONTRACT.md (R1). A row that needs any other file stops and names it (CN9).
- No fifth state; the needs-a-new-person flag and provisioned are never states; no transition is added and none is caused by the flag, and retiring a person moves no other identity. Nothing is appended except the transitions a test drives through DIRECTORY-003 R5's commit path.
- No flag for an agent whose person is suspended: what a suspension stops stays open.
- No screen: showing the flag on the people list and the agent file is a later card; this brief shows it in the lifecycle read and the typed read.
- No start, stop or execution control in the identity product, and no call to any engine (ADR-007, ADR-004).
- DIRECTORY-006 R4's grant rule is neither asserted nor duplicated: the flag reads the person's folded state, never a grant decision.
- The IDENTITY-001 files, docs/design/identity/LIFECYCLE-STATES-2026-09-22.md, docs/design/identity/CONFORMANCE.md and lys-core's shipped formats are unchanged.
- No audit record is deleted, hidden or rewritten; the checklist and story ledgers are append-only.
- No wall clock in new code, no person's name or date in code, no credential, token or key value anywhere, test identities only.

## Verification

- From the repository root: sh scripts/design/gate.sh exits 0, and python3 scripts/design/validate.py docs/design/decisions.json and python3 scripts/design/validate.py docs/design/roadmap.json each exit 0.
- From the repository root, after each code row: cargo fmt --all; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean, with git status showing no change from cargo fmt (heavy runs at the gate venue, CN8).
- From the repository root: rg -c 'fn cf3_r2_ac(1|2)' crates/lys-identity/tests/lifecycle_no_person.rs prints 2; rg -c 'fn cf3_r3_ac(1|2|3)' crates/lys-identity/tests/lifecycle_needs_new_person.rs prints 3; rg -c 'fn cf3_r4_ac(1|2|3)' crates/lys-identity/tests/lifecycle_walk.rs prints 3; rg -c 'fn cf3_r5_ac(1|2|3)' crates/lys-identity-server/tests/lifecycle_authority.rs prints 3.
- grep -c '^| 3.1 |\|^| 3.2 |' docs/design/identity/LIFECYCLE-CONTRACT.md prints 2, and every test function that section names exists in the file it names.
- The row diffs touch no path under vendor/rauthy, docs/design/identity/briefs, docs/design/identity/CONFORMANCE.md or the lifecycle-states statement, and of the lifecycle-hand brief's source files only added lines: git diff --numstat <base> HEAD -- crates/lys-identity/src/lifecycle/causes.rs crates/lys-identity-server/src/lifecycle_read.rs prints lines whose second number is 0.
- rg -n 'SystemTime::now|Utc::now|Local::now|Instant::now' crates/lys-identity/src/lifecycle/responsible.rs prints nothing.

