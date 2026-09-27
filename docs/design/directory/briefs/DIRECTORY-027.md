---
type: brief
id: DIRECTORY-027
cluster: directory
title: Check an agent before giving its start command, keep a launch record for every command given, and show it running only on its signed report
---

# DIRECTORY-027: Check an agent before giving its start command, keep a launch record for every command given, and show it running only on its signed report

> **Cluster:** directory
> **Depends on:** DIRECTORY-002, DIRECTORY-003, DIRECTORY-005, d5055cc1, Ink1H1Os, SECRETS-002, DIRECTORY-011
> **Blocked by:** d5055cc1, the sessions brief, by its run whatever id it lands under (currently drafted as DIRECTORY-015, the brief this card's ruling amends; DIRECTORY-014 is another card): after the amendment its lys/session-start/v1 signed message holds five things, the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge, and the agent's report names the same launch record id it read from LYS_LAUNCH_RECORD. The launch record id is added before the tag freezes, while no agent signs under it; the lead sends this amendment to DIRECTORY-015's signed message. R10 and the running state of R13 are blocked on it by that name., Ink1H1Os, the ROLES card: the profile version record with its review record and the role's machines. R4 and R5 are blocked on it by that name., The amendment to the roles card Ink1H1Os that R9 carries and its dev record records: Ink1H1Os's profile version record holds the executable, the arguments and the working directory as fields of the reviewed profile version (ADR-086), in that card's record and never in a copy this card keeps. R9 is blocked on Ink1H1Os by that name until its landed profile version record declares all three fields., SECRETS-002: the virtual-credential (handle) record its R1 keeps, and the handle-resolving endpoint its R1 lands in the door repository (crates/cambium-door/src/http/secrets_handle.rs, door-owned; that brief sets no root naming the door repository yet). R6 is blocked on it by that name: R6's HandleRecords trait and its fixture tests can be built first, and the door client in crates/lys-identity-server/src/door_handles.rs is built and tested against that endpoint once it lands., CONFORMANCE row 8.5, the network record of machines and egress lists: proposed, with no brief or card yet. R7 is blocked on it by that name, and until it exists every start is refused by name at the egress check., DIRECTORY-003: crates/lys-identity, the signed directory event, the reviewed IDENTITY-EVENTS.md envelope, the administrator's admission and the lifecycle state. R1 to R3 wait on it; R3's check, the agent is active, reads the lifecycle state as the lifecycle card folds it on top of DIRECTORY-003 and is the row that ships first., Paths this brief modifies that no landed manifest declares yet, each reconciled against its owner's reviewed manifest before dispatch, and the brief revised if a path differs: crates/lys-identity/src/lib.rs (R1) and docs/design/identity/IDENTITY-EVENTS.md (R10) against DIRECTORY-003's manifest; crates/lys-identity-server/src/routes.rs (R6, R12), surface/identity/src/routes.tsx and surface/identity/src/generated/index.ts (R13) against DIRECTORY-005's crates/lys-identity-server/ and surface/identity/ manifests. If the reviewed crates/lys-identity-server/ manifest carries no HTTP client for R6's door client, crates/lys-identity-server/Cargo.toml is added to R6's files before dispatch. crates/lys/src/identity/mod.rs and crates/lys/src/identity/cli.rs, which R12 creates to add the identity group, are also named as creates by DIRECTORY-002 R2: whichever of the two lands second modifies them, reconciled against the other's landed file before dispatch., Paths this brief modifies that no landed manifest declares yet, each reconciled against its owner's reviewed manifest before dispatch, and the brief revised if a path differs: crates/lys-identity/src/lib.rs (R1) against DIRECTORY-003's crates/lys-identity/ manifest; crates/lys-identity-server/src/routes.rs (R12), surface/identity/src/routes.tsx and surface/identity/src/generated/index.ts (R13) against DIRECTORY-005's crates/lys-identity-server/ and surface/identity/ manifests; crates/lys/src/identity/cli.rs and crates/lys/src/identity/mod.rs (R12) against DIRECTORY-002's., DIRECTORY-011, the provision brief: a start reads its agent record, the enduring identity, and nothing else of that brief. R1's check that the agent exists is blocked on it by that name, and the agent record is read, never copied and never changed by a start., Review of the launch record and withdrawal event kinds R10 adds to IDENTITY-EVENTS.md before the first is signed, as an adversarial review of a signed format., Waffles' review of this written brief, under the DIRECTORY-001 boundary that no row brief is dispatched until Waffles has reviewed it; then the identity line lead's sign-off, and only then is the build fired. Do not dispatch from this schema-valid brief while any blocker above stands (as CN12 says for DIRECTORY-006).
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-085 — Every command given keeps its own signed launch record, and running is tied to it through lys/session-start/v1 — Every command given keeps one launch record, a signed directory event naming the machine, the executable, the working directory, the profile version and the credential ids. Its id is carried in lys/session-start/v1, whose signed message then holds five things: the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge. The agent reads the id from LYS_LAUNCH_RECORD and its report names it, and running is tied to the command given through that id. A start given again copies a kept record into a new record with its own id that names the record it was copied from. Rejected: tying running to the agent id alone, re-issuing one record for every command given, and keeping the launch facts only in the command line.
> - ADR-086 — The executable, its arguments and the working directory are fields of the reviewed profile version, never of a start request — The executable, the arguments and the working directory are read from the reviewed profile version's record, as the mock-up draws them, and a start request names only the agent, the profile version and the machine. Rejected: letting a start request set either, and taking either from the machine or a default.
> - ADR-087 — Withdrawing a start request records only that the request no longer stands — A start request can be withdrawn by the person who gave it or anyone holding the same right. The withdrawal records that the request no longer stands, with who and when, and never that the agent did not start. If a signed report later arrives for the withdrawn launch record, the session shows as running with the withdrawal beside it. Rejected: recording a withdrawal as not started, and refusing a report that arrives after a withdrawal.
> - ADR-088 — One standing start per agent, and the command given as ids-only environment assignments before the recorded executable — While a start for an agent stands unconfirmed, a second start for that agent, on any machine, is refused as start_unconfirmed, naming the standing request and the act that answers it: wait for its report, or withdraw it first. The given command reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> <executable> <recorded arguments>, with the working directory set to the recorded one. The three values are ids only, each shell-quoted and refused by name if it holds anything outside its id grammar. Rejected: appending the ids to the recorded arguments, and giving a second command while one stands unconfirmed.
> **Checklist:**
> - C209 — No start path in the library, the route or the CLI spawns a process, and a test that reads the start files and gives a start with a marker-writing executable proves it (CONFORMANCE 5.1).
> - C210 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
> - C211 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.
> - C212 — No credential value is on the command line or the clipboard, and a test reads both (CONFORMANCE 5.3).
> - C213 — A launch record naming the machine, the executable, the working directory, the profile version and the credential ids is kept for every command given, reads back after a restart, and a start given again from it is a new record naming the one it was copied from (CONFORMANCE 5.4).
> - C214 — A start request names only the agent, the profile version and the machine; the command given reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> before the reviewed profile version's own executable and its recorded arguments, unchanged and in order, with the recorded working directory, each value shell-quoted and refused by name outside its id grammar; it is not a lys subcommand, and a request that sets the executable or the working directory is refused by name.
> - C215 — An agent shows as running only when its verified signed report names its launch record, and copying the command changes no state (CONFORMANCE 5.5).
> - C216 — With no report a start reads unconfirmed, never not started; the screen says the request stands and warns against asking elsewhere, and a second start for the agent is refused as start_unconfirmed (CONFORMANCE 5.6).
> - C217 — A withdrawal by the giver or anyone holding the same right records who and when and that the request no longer stands, never that the agent did not start, and a report arriving afterwards shows running with the withdrawal beside it.
> - C218 — Only the agent's responsible person and a directory administrator (in step 1, the admitted administrator) have a working Start; anyone else is refused by name, naming the agent and the right they lack.
> - C219 — The Start drawer and the unconfirmed notice are built on DIRECTORY-005's surface, and DIRECTORY-005's R1 and boundary name Start as the one control that works in step 1.
> - C220 — The start route gives, gives again, withdraws and reads a start through the library and holds no start logic of its own, and the CLI answer lys identity start-command, the one subcommand of the identity group this card adds, prints exactly what the route answers for the same three inputs and starts nothing.
> - C221 — A start resolves its agent to the enduring agent record the provision brief DIRECTORY-011 keeps and is refused by name, writing nothing, when there is none; a start never creates or changes an agent record and writes no session record, and a start of an agent already running is given as a new launch record naming the same agent, whose session record the report makes in the sessions brief's store.
> **Stories:**
> - S84 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
> - S85 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.
> - S86 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent shown as running only when its own signed report names the command I was given, so that copying a command is never mistaken for a start.
> - S87 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want an unanswered start shown as unconfirmed with its request standing, so that I do not ask elsewhere and start the agent twice.
> - S88 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to withdraw a start request I no longer want, so that the record says the request no longer stands without claiming the agent did not start.
> - S89 (Operator, Installs and runs the standalone identity product) — As the operator, I want a kept launch record for every command given, so that I can give the same start again without looking inside a running process.
> - S90 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that reads the command line and the clipboard and finds no credential value, so that giving a command never discloses a secret.
> - S91 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that fails if any start path spawns a process, so that lys stays a product that checks, gives and records and never runs an agent.
> - S92 (Person without the right to start, Opens an agent file that is not theirs to start) — As a person who is neither the agent's responsible person nor a directory administrator, I want a refusal naming the agent and the right I lack, so that I know why there is no working Start for me.
> - S93 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want the command I am given to run the executable in the working directory that were reviewed, so that no start request can change what runs.
> - S94 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want starting my agent again while it runs to become a second session of the same agent, so that starting never makes another agent and never changes the one I registered.

## Purpose

Make CONFORMANCE section 5 rows 5.1 to 5.6 true: lys checks an agent before giving its start command, gives the command without ever running the agent or putting a credential value on the command line or the clipboard, keeps a launch record for every command given, and shows the agent as running only when its signed report names that launch record. With no report the start is unconfirmed, never not started, and a request can be withdrawn without anyone claiming the agent did not start. This is ADR-007 built: check, give, record, never run. Each check reads its owner's record, so the directory never holds a copy of what the roles card, the door, the network record, the sessions brief or the provision brief keep.

## Task

Build R1 to R13 in order. R1 and R2 fix what a start request is, that its agent exists as the enduring agent record DIRECTORY-011 keeps, and who may make one. R3 runs the five named checks and wires the one that needs only the lifecycle record, the agent is active; it ships first. R4 to R7 each add one check against its owner's record and each is blocked on that owner by name: Ink1H1Os (R4, R5), SECRETS-002 (R6) and network row 8.5 (R7). While an owner's record does not exist the start is refused by name, naming the check and the card, and nothing is faked to let a start through. R8 amends DIRECTORY-005 so that Start is the one control that works in step 1. R9 takes the executable, its arguments and the working directory from the reviewed profile version, and carries the amendment that puts those three fields in Ink1H1Os's profile version record. R10 defines the launch record and the withdrawal as signed events, derives running, unconfirmed and withdrawn from the records it reads, and answers whether a start for an agent stands unconfirmed: a withdrawn record no longer stands and a running one does not. R11 keeps the launch record, renders the command from it as env LYS_AGENT_ID, LYS_LAUNCH_RECORD and LYS_CREDENTIAL_IDS assignments before the recorded executable and its recorded arguments, refuses a value outside its id grammar as command_value_outside_grammar, gives a start again as a new record naming its source, refuses a second start as start_unconfirmed while R10 answers that one stands, and gives a second start of an agent that already reads as running by keeping only a new launch record, leaving the session record to the report in the sessions brief's store. R12 lands the route, which gives, gives again, withdraws and reads a start, the CLI answer (lys identity start-command <agent> --profile-version <pv> --machine <m>, the one subcommand of the identity group this card adds to the lys CLI, which prints exactly what the route answers for the same three inputs and is never the command given) and the no-spawn proof; R13 lands the Start drawer and the unconfirmed notice. In: CONFORMANCE rows 5.1 to 5.6, each with at least one acceptance line testing it (5.1 in R12; 5.2 in R3 to R7; 5.3 in R11, R12 and R13; 5.4 in R11; 5.5 in R10 and R13; 5.6 in R10, R11 and R13). Out: row 5.7 (terminal, sandbox, VM or container runners, proposed as separate products); how the command proves itself, which stays proposed; every record the checks read; a grant to start held by anyone else; the environment a runtime gives a started agent (SECRETS-002 R5), which the command-line and clipboard test does not cover. The brief carries 13 requirements, one more than the method's split signal, because the card holds the six rows as one landable unit and each check keeps its own row naming its owner. It is code-bearing, so it carries its own wall in its boundaries; the cluster's documents-only CN1 governs DIRECTORY-001's work, not this brief. R10, the state derivation, comes before R11's start_unconfirmed refusal, which cites it by number, so no requirement relies on a state a later one defines. The sessions brief is named in depends_on by its run d5055cc1, whatever id it lands under; its current draft carries DIRECTORY-015, and it is that brief whose lys/session-start/v1 signed message the ruling amends to five things: the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge.

## Requirements

### R1: Define the start request as the agent, the profile version and the machine, resolve the agent to its enduring record, and name every refusal

Structure: a start request names exactly three members, the agent, the profile version and the machine, in a new module crates/lys-identity/src/start/ declared from crates/lys-identity/src/lib.rs, which DIRECTORY-003 creates and this requirement modifies only after it lands. Every refusal of a start is one typed error variant with a stable name and words that say what refused: start_field_not_allowed, start_member_missing, agent_unknown, start_right_missing, agent_not_active, profile_version_not_reviewed, machine_not_allowed_for_role, virtual_credentials_not_valid, egress_not_reachable, check_record_missing, profile_version_field_missing, command_value_outside_grammar and start_unconfirmed. Behaviour: IF a start request carries any member beside those three, the executable and the working directory included, THEN THE SYSTEM SHALL refuse it by name as start_field_not_allowed, naming the member, and SHALL NOT run a check, give a command or keep a launch record. IF a start request lacks any of those three members, THEN THE SYSTEM SHALL refuse it by name as start_member_missing, naming the missing member, and SHALL NOT run a check, give a command or keep a launch record. The executable and the working directory come only from the reviewed profile version (R9, ADR-086). WHEN a start request is asked, THE SYSTEM SHALL resolve the agent it names to that agent's registered agent record, the enduring identity the provision brief DIRECTORY-011 keeps, before anything else runs, and every record the start keeps SHALL point at exactly that enduring agent id. IF no agent record exists for the agent named, THEN THE SYSTEM SHALL refuse the start by name as agent_unknown, naming the agent, and SHALL NOT run a check, give a command, keep a launch record or write any record: starting never creates an agent. THE SYSTEM SHALL NOT change the agent record, which holds no session credential, and SHALL NOT keep a copy of it. This check is blocked on DIRECTORY-011 by that name. THE SYSTEM SHALL NOT put a credential value in any refusal, its words, its Debug form or a log line.

**Acceptance:**
- A request with agent agent-fixture-1, profile_version pv-fixture-1 and machine machine-fixture-1 parses into a start request holding exactly those three values.
- A request with the same three members plus executable /bin/sh is refused as start_field_not_allowed, its words contain 'executable', and the launch record count stays 0.
- A request with the same three members plus working_directory /tmp is refused as start_field_not_allowed, its words contain 'working_directory', and the launch record count stays 0.
- A request with agent agent-fixture-1 and profile_version pv-fixture-1 and no machine is refused as start_member_missing, its words contain 'machine', and the launch record count stays 0.
- With the provision record fixture holding only agent-fixture-1, a request naming agent-fixture-missing, profile_version pv-fixture-1 and machine machine-fixture-1 is refused as agent_unknown, its words contain 'agent-fixture-missing', the check-run count is 0, the launch record count is 0 and the agent record count is 1.
- The refusal test builds every one of the 13 named variants once and asserts the count is 13; for each, the Display and Debug output built with the fixture value fixture-credential-value-1f3a in scope does not contain 'fixture-credential-value-1f3a'.

**Files:**
- create: crates/lys-identity/src/start/mod.rs
- create: crates/lys-identity/src/start/request.rs
- create: crates/lys-identity/src/start/error.rs
- create: crates/lys-identity/tests/start_request.rs
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C214 — A start request names only the agent, the profile version and the machine; the command given reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> before the reviewed profile version's own executable and its recorded arguments, unchanged and in order, with the recorded working directory, each value shell-quoted and refused by name outside its id grammar; it is not a lys subcommand, and a request that sets the executable or the working directory is refused by name.
- C221 — A start resolves its agent to the enduring agent record the provision brief DIRECTORY-011 keeps and is refused by name, writing nothing, when there is none; a start never creates or changes an agent record and writes no session record, and a start of an agent already running is given as a new launch record naming the same agent, whose session record the report makes in the sessions brief's store.

**Stories:**
- S93 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want the command I am given to run the executable in the working directory that were reviewed, so that no start request can change what runs.
- S94 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want starting my agent again while it runs to become a second session of the same agent, so that starting never makes another agent and never changes the one I registered.

### R2: Admit a start, a start again and a withdrawal only from the agent's responsible person or a directory administrator

WHEN a start, a start given again or a withdrawal is asked for an agent, THE SYSTEM SHALL admit it only from that agent's responsible person (ADR-003, ADR-011) or a directory administrator. In step 1 that is the administrator the configured issuer and subject admit (P9); the responsible person is admitted when step 1's admission admits them, and until then the directory's admission refuses them as it refuses every other caller. IF anyone else asks, THEN THE SYSTEM SHALL refuse by name as start_right_missing, naming the agent and the right they lack (to start this agent), and SHALL NOT run a check, give a command or keep or change a launch record. THE SYSTEM SHALL NOT admit a start through a grant held by anyone other than the responsible person or an administrator; that grant is a further unit and is not built here.

**Acceptance:**
- admin-fixture, the admitted administrator, asks to start agent-fixture-1: the authority step answers admitted for admin-fixture and no start_right_missing is returned.
- person-fixture-1, recorded as agent-fixture-1's responsible person and admitted by the test's admission fixture, asks to start agent-fixture-1: the authority step answers admitted for person-fixture-1 and no start_right_missing is returned.
- person-fixture-2, neither responsible person nor administrator, asks to start agent-fixture-1: the answer is start_right_missing, its words contain 'agent-fixture-1' and 'start', the check-run count is 0 and the launch record count is 0.
- person-fixture-2 holding a fixture grant on agent-fixture-1 asks to start it and the answer is still start_right_missing.

**Files:**
- create: crates/lys-identity/src/start/authority.rs
- create: crates/lys-identity/tests/start_authority.rs

**Checklist:**
- C218 — Only the agent's responsible person and a directory administrator (in step 1, the admitted administrator) have a working Start; anyone else is refused by name, naming the agent and the right they lack.

**Stories:**
- S92 (Person without the right to start, Opens an agent file that is not theirs to start) — As a person who is neither the agent's responsible person nor a directory administrator, I want a refusal naming the agent and the right I lack, so that I know why there is no working Start for me.

### R3: Run the five named checks before any command, with the agent is active read live from the lifecycle record

WHEN an admitted start is asked, THE SYSTEM SHALL run five named checks, each reading its owner's record and none keeping a copy of it: the agent is active; its profile version is reviewed; the machine is allowed for the role; its virtual credentials are valid; the machine may reach what the profile needs. Each check answers passed, failed with words that name the check, or record missing naming the check and the card that makes the record. Every check runs and every result is returned by its name in words. WHEN any check does not pass, THE SYSTEM SHALL refuse the start and SHALL NOT give a command or keep a launch record. IF a check's owning record does not exist, THEN THE SYSTEM SHALL answer check_record_missing naming that check and the card that makes the record, and SHALL NOT substitute a default, a sample record or a pass. This requirement wires the agent is active live: it reads the lifecycle state DIRECTORY-003 records beside each identity (ADR-011, proposed) and passes only on active; registered, suspended and retired fail as agent_not_active naming the state. It is the row that ships first, and it needs only the lifecycle record. R4 to R7 each fill one of the other four checks.

**Acceptance:**
- agent-fixture-1 in state active: the agent is active answers passed.
- agent-fixture-1 in state suspended: the start is refused as agent_not_active, its words contain 'the agent is active' and 'suspended', no command is returned and the launch record count is 0.
- agent-fixture-1 in state registered: the agent is active answers failed and its words contain 'registered'.
- With the four other owning records absent and agent-fixture-1 active, a start is refused with exactly 4 check_record_missing results, whose words contain respectively 'Ink1H1Os', 'Ink1H1Os', 'SECRETS-002' and 'row 8.5', and the launch record count is 0.
- The check list the start runs has exactly 5 entries, in this order: the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs.

**Files:**
- create: crates/lys-identity/src/start/checks.rs
- create: crates/lys-identity/src/start/active.rs
- create: crates/lys-identity/tests/start_checks.rs

**Checklist:**
- C210 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- C211 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.

**Stories:**
- S84 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S85 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

### R4: Check that the profile version is reviewed, from the roles card's review record

THE SYSTEM SHALL check that the requested profile version has a review on record, reading the review record the roles card Ink1H1Os makes for profile versions. IF the profile version has no review on record, THEN THE SYSTEM SHALL answer profile_version_not_reviewed naming the profile version. WHILE Ink1H1Os's record does not exist, THE SYSTEM SHALL answer check_record_missing naming its profile version is reviewed and Ink1H1Os. THE SYSTEM SHALL NOT treat a sample review, a mock-up record or the absence of a refusal as a review. This requirement is blocked on Ink1H1Os by that name.

**Acceptance:**
- With the review record fixture holding a review of pv-fixture-1, its profile version is reviewed answers passed for pv-fixture-1.
- With the same fixture, a start naming pv-fixture-2, which has no review, answers profile_version_not_reviewed and its words contain 'pv-fixture-2'.
- With no review record present, the check answers check_record_missing and its words contain 'its profile version is reviewed' and 'Ink1H1Os'.

**Files:**
- create: crates/lys-identity/src/start/profile_review.rs
- create: crates/lys-identity/tests/start_profile_review.rs

**Checklist:**
- C210 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- C211 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.

**Stories:**
- S84 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S85 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

### R5: Check that the machine is allowed for the role, from the role's machines in the roles card's record

THE SYSTEM SHALL check that the requested machine is one of the machines the agent's role may run on, reading the role's machines from the roles card Ink1H1Os's record, and SHALL identify the machine by the identifier that record gives it. IF the machine is not among them, THEN THE SYSTEM SHALL answer machine_not_allowed_for_role naming the machine and the role. WHILE Ink1H1Os's record does not exist, THE SYSTEM SHALL answer check_record_missing naming the machine is allowed for the role and Ink1H1Os, and SHALL NOT offer a machine list from any other source. This requirement is blocked on Ink1H1Os by that name.

**Acceptance:**
- With the role machines fixture allowing machine-fixture-1 for role-fixture-builder and agent-fixture-1 holding role-fixture-builder, the machine is allowed for the role answers passed for machine-fixture-1.
- With the same fixture, machine-fixture-2 answers machine_not_allowed_for_role and its words contain 'machine-fixture-2' and 'role-fixture-builder'.
- With no role machines record present, the check answers check_record_missing and its words contain 'the machine is allowed for the role' and 'Ink1H1Os'.

**Files:**
- create: crates/lys-identity/src/start/machine_role.rs
- create: crates/lys-identity/tests/start_machine_role.rs

**Checklist:**
- C210 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- C211 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.

**Stories:**
- S84 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S85 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

### R6: Check that the agent's virtual credentials are valid, from the door's handle record

THE SYSTEM SHALL check that the agent holds valid virtual credentials, reading the handle record SECRETS-002 keeps (its R1, ADR-001), and on a pass SHALL hand the credential ids to the launch record (R11). IF the agent holds no valid virtual credential, THEN THE SYSTEM SHALL answer virtual_credentials_not_valid naming the agent. WHILE SECRETS-002's record does not exist, THE SYSTEM SHALL answer check_record_missing naming its virtual credentials are valid and SECRETS-002. THE SYSTEM SHALL NOT read, hold, log or return a credential value: the check reads ids and validity only. The check reads the handle record through one seam, the trait HandleRecords declared in crates/lys-identity/src/start/credentials.rs, which answers for an agent the ids of its virtual credentials and whether each is valid, or that the record does not exist, and has no method that returns a credential value. Its one production implementation is an HTTP client of the handle-resolving endpoint SECRETS-002 R1 lands in the door (crates/cambium-door/src/http/secrets_handle.rs in the door repository, door-owned), held in crates/lys-identity-server/src/door_handles.rs and wired into the start route from crates/lys-identity-server/src/routes.rs. The library's tests fill HandleRecords with a fixture; the client is tested against the door's own endpoint, never against a double this card writes of it. This requirement is blocked on SECRETS-002 by that name.

**Acceptance:**
- Reading crates/lys-identity/src/start/credentials.rs finds the trait HandleRecords, and the return type of each of its methods holds only credential ids, their validity and the record-missing answer: no method returns a type with a field for a credential value.
- With the handle record fixture holding vc-fixture-1 active for agent-fixture-1, its virtual credentials are valid answers passed and hands on exactly ['vc-fixture-1'].
- With vc-fixture-1 revoked in the fixture, the check answers virtual_credentials_not_valid and its words contain 'agent-fixture-1'.
- With no handle record present, the check answers check_record_missing and its words contain 'its virtual credentials are valid' and 'SECRETS-002'.
- The handle record fixture carries the value fixture-credential-value-1f3a beside vc-fixture-1, and no output of the check, Debug form included, contains 'fixture-credential-value-1f3a'.
- With the door started from the commit at which SECRETS-002 R1 lands, holding vc-fixture-1 active for agent-fixture-1, the HandleRecords client in crates/lys-identity-server/src/door_handles.rs answers passed for agent-fixture-1 and hands on exactly ['vc-fixture-1'].

**Files:**
- create: crates/lys-identity/src/start/credentials.rs
- create: crates/lys-identity/tests/start_credentials.rs
- create: crates/lys-identity-server/src/door_handles.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C210 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- C211 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.

**Stories:**
- S84 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S85 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

### R7: Check that the machine may reach what the profile needs, from its egress list

THE SYSTEM SHALL check that every destination the requested profile version needs is on the machine's egress list, reading what the profile needs from the profile version's record and the egress list from the network record of CONFORMANCE row 8.5. IF any needed destination is not on the list, THEN THE SYSTEM SHALL answer egress_not_reachable naming each destination the machine may not reach. WHILE row 8.5's record does not exist, THE SYSTEM SHALL answer check_record_missing naming the machine may reach what the profile needs and network row 8.5, and SHALL NOT assume open egress. Row 8.5 has no card yet; this requirement is blocked on it by that name, and no start passes every check until it exists.

**Acceptance:**
- With the profile needing 'model providers' and 'mcp-fixture' and machine-fixture-1's egress fixture listing both, the machine may reach what the profile needs answers passed.
- With machine-fixture-1's egress fixture listing only 'model providers', the check answers egress_not_reachable and its words contain 'mcp-fixture'.
- With no egress record present, the check answers check_record_missing and its words contain 'the machine may reach what the profile needs' and 'row 8.5'.

**Files:**
- create: crates/lys-identity/src/start/egress.rs
- create: crates/lys-identity/tests/start_egress.rs

**Checklist:**
- C210 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- C211 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.

**Stories:**
- S84 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S85 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

### R8: Amend DIRECTORY-005 so that Start is the one control that works in step 1

Structure: append one sentence to the spec of DIRECTORY-005 R1 and the same sentence to its boundary 'No grant, launch, secret or memory control is presented as working in step 1.': 'Under DIRECTORY-027, Start on the agent file is the one control that works in step 1, for the admitted administrator; every other grant, launch-profile, secret and memory control stays not working in step 1.' In the same edit, amend DIRECTORY-005 R1's third acceptance line so it agrees: its clause 'no control for launch, permissions, secrets or memory is presented as working' becomes 'no control for permissions, secrets or memory is presented as working', and the line gains the sentence 'Under DIRECTORY-027, the Start control is the one that brief gives, refused by name until the start is confirmed; permissions, secrets and memory are still not presented as working.' Re-render DIRECTORY-005.md from the JSON. THE SYSTEM SHALL NOT change any other text, identifier, dependency or acceptance line of DIRECTORY-005.

**Acceptance:**
- The diff of docs/design/directory/briefs/DIRECTORY-005.json changes exactly three strings: requirements/0/spec and the boundary quoted above, each ending with the appended sentence, and requirements/0/acceptance/2.
- After the edit, DIRECTORY-005's requirements/0/acceptance/2 contains 'no control for permissions, secrets or memory is presented as working' and 'Under DIRECTORY-027, the Start control is the one that brief gives', and does not contain 'launch'.
- python3 scripts/design/render-cluster.py over a copy of docs/design/directory writes a DIRECTORY-005.md byte-identical to the committed one.

**Files:**
- modify: docs/design/directory/briefs/DIRECTORY-005.json
- modify: docs/design/directory/briefs/DIRECTORY-005.md

**Checklist:**
- C219 — The Start drawer and the unconfirmed notice are built on DIRECTORY-005's surface, and DIRECTORY-005's R1 and boundary name Start as the one control that works in step 1.

### R9: Take the executable, its arguments and the working directory from the reviewed profile version

The executable and the working directory a command runs are fields of the reviewed profile version (ADR-086), and the arguments it is given are the ones the profile version records. This requirement adds the executable, the arguments and the working directory to the profile version under the roles card Ink1H1Os's record, named in depends_on, as a named amendment to that card: the fields are added in Ink1H1Os's record and never in a copy this card keeps, the amendment is named in blocked_by, the dev record of this requirement carries it, and this requirement stays blocked on Ink1H1Os by that name until its landed profile version record declares the three fields. WHEN a command is to be given, THE SYSTEM SHALL read the executable, the recorded arguments and the working directory from the requested profile version's record, through one seam, the trait ProfileVersionRecords declared in crates/lys-identity/src/start/profile_command.rs, which answers for a profile version id the executable, the arguments and the working directory exactly as Ink1H1Os's record holds them, each or that it is missing. The implementation of that trait over Ink1H1Os's landed profile version record is written under this requirement when that record lands, and it is where the record is read; until then only the trait and its fixture exist. IF that record does not hold the executable or the working directory, THEN THE SYSTEM SHALL refuse as profile_version_field_missing naming the field and Ink1H1Os, and SHALL NOT take the executable, an argument or the working directory from the request, the machine, a default or the mock-up. THE SYSTEM SHALL NOT define a profile version record of its own.

**Acceptance:**
- Reading crates/lys-identity/src/start/profile_command.rs finds the trait ProfileVersionRecords declaring exactly 3 reads for a profile version id: the executable, the arguments and the working directory.
- rg -n 'struct ProfileVersion' crates/lys-identity/src/start finds 0 lines.
- With the profile version fixture pv-fixture-1 holding executable fixture-exec, arguments ['--fixture-arg', 'two words'] and working directory fixture-cwd, the start reads exactly 'fixture-exec', ['--fixture-arg', 'two words'] and 'fixture-cwd'.
- With pv-fixture-1 holding no working directory, the start is refused as profile_version_field_missing, its words contain 'working directory' and 'Ink1H1Os', and the launch record count is 0.

**Files:**
- create: crates/lys-identity/src/start/profile_command.rs
- create: crates/lys-identity/tests/start_profile_command.rs

**Checklist:**
- C214 — A start request names only the agent, the profile version and the machine; the command given reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> before the reviewed profile version's own executable and its recorded arguments, unchanged and in order, with the recorded working directory, each value shell-quoted and refused by name outside its id grammar; it is not a lys subcommand, and a request that sets the executable or the working directory is refused by name.

**Stories:**
- S89 (Operator, Installs and runs the standalone identity product) — As the operator, I want a kept launch record for every command given, so that I can give the same start again without looking inside a running process.
- S93 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want the command I am given to run the executable in the working directory that were reviewed, so that no start request can change what runs.

### R10: Define the launch record and derive its state: running only on its signed report, unconfirmed without one, withdrawn without claiming it did not start, and whether a start stands

Structure: a launch record is one signed directory event (P4) of a kind added to docs/design/identity/IDENTITY-EVENTS.md, which DIRECTORY-003 creates and this requirement modifies only after it lands, naming its own launch record id, the enduring agent id, the machine, the executable, the working directory, the profile version, the credential ids, the person who gave it and, when it was given again from a kept record, the launch record it was copied from. A withdrawal is one signed directory event of a second kind added there, naming the launch record, who withdrew it and when. Both kinds are reviewed before the first is signed, a launch record is read by its id from the directory store, and neither kind carries a credential value. Behaviour: THE SYSTEM SHALL derive each launch record's state from the records it reads, keeping no copy: running WHEN the record kept by the sessions brief d5055cc1 holds a verified lys/session-start/v1 report, signed with the agent's own key, whose agent id and launch record id equal the launch record's; unconfirmed WHILE no such report exists and no withdrawal names the record; withdrawn WHEN a withdrawal names the record and no such report exists. THE SYSTEM SHALL NOT show or return 'not started' for any state, and copying a command SHALL NOT change any state. WHEN the person who gave a start, or anyone holding the same right (R2), withdraws it, THE SYSTEM SHALL record one signed withdrawal that the request no longer stands, naming who withdrew it and when, and SHALL NOT record that the agent did not start. IF a verified report later arrives for a withdrawn launch record, THEN THE SYSTEM SHALL show it running with the withdrawal beside it. THE SYSTEM SHALL answer, for an agent, whether a start for it stands unconfirmed, and which launch record stands: a launch record stands only while it reads unconfirmed, so a withdrawn record no longer stands and a record that reads running does not stand unconfirmed. R11 refuses and gives a start by this answer. THE SYSTEM SHALL NOT write, copy or change a session record, and SHALL NOT create or change the agent record. The session-start message this reads holds five things: the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge, and the report names the launch record id the agent read from LYS_LAUNCH_RECORD (R11). This card does not define that message: the sessions brief d5055cc1 (currently drafted as DIRECTORY-015) adds the launch record id before its tag freezes, and this requirement is blocked on d5055cc1 by that name. The dev record of this requirement carries that ruling.

**Acceptance:**
- Launch record L1 for agent-fixture-1 with no report in the sessions record fixture reads as unconfirmed. (CONFORMANCE 5.6)
- With the sessions record fixture holding a verified report naming agent-fixture-1 and L1, L1 reads as running. (CONFORMANCE 5.5)
- With the fixture holding a verified report naming L1 and agent-fixture-2, L1 reads as unconfirmed.
- With the fixture holding a report naming agent-fixture-1 and L1 marked not verified, L1 reads as unconfirmed.
- With L1 and L2 kept for agent-fixture-1 and a verified report naming L2 only, L2 reads as running and L1 as unconfirmed.
- With L1 unconfirmed, the answer for agent-fixture-1 is that L1 stands unconfirmed.
- admin-fixture withdraws L1: L1 reads as withdrawn, the withdrawal event names admin-fixture and a time, and the answer for agent-fixture-1 is that no start stands unconfirmed.
- person-fixture-2 asks to withdraw L1: the answer is start_right_missing, L1 still reads as unconfirmed and the withdrawal event count is 0.
- After admin-fixture withdraws L1, a verified report naming agent-fixture-1 and L1 arrives: L1 reads as running with the withdrawal by admin-fixture beside it.
- With L1 reading running from a verified report, the answer for agent-fixture-1 is that no start stands unconfirmed.
- Across every state the tests reach, no state value, words or serialised answer contains 'not started' or 'not_started'.
- Across every state derivation and withdrawal the tests run, the sessions record fixture's bytes are unchanged and agent-fixture-1's agent record is byte-identical to its bytes before the first test step.

**Files:**
- create: crates/lys-identity/src/start/launch_record.rs
- create: crates/lys-identity/src/start/state.rs
- create: crates/lys-identity/src/start/withdrawal.rs
- create: crates/lys-identity/tests/start_state.rs
- modify: docs/design/identity/IDENTITY-EVENTS.md

**Checklist:**
- C215 — An agent shows as running only when its verified signed report names its launch record, and copying the command changes no state (CONFORMANCE 5.5).
- C216 — With no report a start reads unconfirmed, never not started; the screen says the request stands and warns against asking elsewhere, and a second start for the agent is refused as start_unconfirmed (CONFORMANCE 5.6).
- C217 — A withdrawal by the giver or anyone holding the same right records who and when and that the request no longer stands, never that the agent did not start, and a report arriving afterwards shows running with the withdrawal beside it.

**Stories:**
- S86 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent shown as running only when its own signed report names the command I was given, so that copying a command is never mistaken for a start.
- S87 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want an unanswered start shown as unconfirmed with its request standing, so that I do not ask elsewhere and start the agent twice.
- S88 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to withdraw a start request I no longer want, so that the record says the request no longer stands without claiming the agent did not start.

### R11: Keep a launch record for every command given, render the command from it, give a start again as a new record, and refuse a second start while one stands

WHEN every check passes, THE SYSTEM SHALL keep a launch record (R10) naming the agent, the machine, the executable, the working directory, the profile version, the credential ids and the person who gave it, and SHALL give the command built from that record and the reviewed profile version it names, and nothing else: the profile version's own executable, with the arguments the profile version records, invoked in its recorded working directory. The given command reads: env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> <executable> <recorded arguments>, with the working directory set to the recorded one, which the start answer returns beside the command line. The three environment assignments come before the executable, and the recorded arguments follow it exactly as the profile version records them, in their order, with nothing appended. Each value and each argument is shell-quoted, so a POSIX shell splits the command line back into exactly those words. The three assigned values are ids only, the enduring agent id, the launch record's own id and the credential ids R6 handed on, never a credential value. IF any of them holds anything outside its id grammar (the agent id's as DIRECTORY-011's agent record defines it, the launch record id's as the directory event that keeps it defines it, a credential id's as SECRETS-002's handle record defines it), THEN THE SYSTEM SHALL refuse the start by name as command_value_outside_grammar, naming the assignment, and SHALL NOT give a command or keep a launch record for it. THE SYSTEM SHALL NOT add a lys launcher subcommand; the mock-up's 'identity start <slug>' line is illustrative only, and the command given is never a lys command. How the command proves itself stays proposed and the command carries no proof (CONFORMANCE 5.3). WHEN a start is given again from a kept launch record, THE SYSTEM SHALL run R2 and the checks again and keep a new launch record with its own id, copied from the kept one and naming it as the record it was copied from, without reading any running process; THE SYSTEM SHALL NOT re-issue a launch record. WHILE R10 answers that a start for an agent stands unconfirmed, WHEN a second start for that agent is asked on any machine, THE SYSTEM SHALL refuse it as start_unconfirmed, naming the standing launch record and the act that answers it, to wait for its report or to withdraw it first, and SHALL NOT keep a second record. WHEN R10 answers that no start for the agent stands unconfirmed, because its records are withdrawn or read running, THE SYSTEM SHALL give the start. WHEN a start is asked for an agent that already reads as running, THE SYSTEM SHALL give it, keeping only a new launch record with its own id that names the same enduring agent id; the session record that answers it is made in the sessions brief d5055cc1's store when that start's signed report arrives, and nothing about the first session changes. The session credential's form stays with its own card, RM-029 'A session is its own record, issued at session start and pointing at its agent', which is held on the open directory brief branch of run 28d15c0d and not in this tree's roadmap, and it is not asserted here. THE SYSTEM SHALL NOT write, copy or change a session record, and SHALL NOT create or change the agent record, for this or any start. THE SYSTEM SHALL NOT put a credential value in the launch record, the command, a log line or an error.

**Acceptance:**
- With every check fixture passing for agent-fixture-1, pv-fixture-1 (fixture-exec in fixture-cwd), machine-fixture-1 and vc-fixture-1, a start by admin-fixture keeps exactly 1 launch record whose machine, executable, working directory, profile version and credential ids equal machine-fixture-1, fixture-exec, fixture-cwd, pv-fixture-1 and ['vc-fixture-1'], and whose giver is admin-fixture.
- After that start is given, agent-fixture-1's agent record read from the provision record fixture is byte-identical to its bytes before the start, the agent record count is 1, and the launch record names agent-fixture-1.
- With pv-fixture-1 recording executable fixture-exec, arguments ['--fixture-arg', 'two words'] and working directory fixture-cwd, and the handle record fixture handing on vc-fixture-1 and vc-fixture-2 with the values fixture-credential-value-1f3a and fixture-credential-value-2b7c beside them, the given command line split by a POSIX shell word splitter is exactly ['env', 'LYS_AGENT_ID=agent-fixture-1', 'LYS_LAUNCH_RECORD=' followed by the kept launch record's id, 'LYS_CREDENTIAL_IDS=vc-fixture-1,vc-fixture-2', 'fixture-exec', '--fixture-arg', 'two words'], the start answer's working directory is 'fixture-cwd', and neither the command line nor the start answer contains 'fixture-credential-value-1f3a' or 'fixture-credential-value-2b7c' (CONFORMANCE 5.3).
- The executable, the arguments and the working directory of that start answer equal pv-fixture-1's recorded executable, arguments and working directory field for field, the arguments in the recorded order, and the command line's first word is 'env', never 'lys'.
- With the handle record fixture's credential id grammar admitting only lowercase letters, digits and '-', and the credential id 'vc-fixture;1' handed on, the start is refused as command_value_outside_grammar, its words contain 'LYS_CREDENTIAL_IDS', no command is returned and the launch record count is 0.
- Rendering the command twice from the same kept launch record returns byte-identical strings.
- After the directory store fixture is closed and reopened, reading the launch record by its id returns the same agent, machine, executable, working directory, profile version and credential ids.
- With L1 withdrawn by admin-fixture and no sessions record present, giving a start again from kept record L1 keeps record L2: the ids of L1 and L2 differ, L2 names L1 as the record it was copied from, and their five launch fields are equal. (CONFORMANCE 5.4)
- With L1 standing unconfirmed by R10's answer, a second start for agent-fixture-1 on machine-fixture-2 is refused as start_unconfirmed, its words contain L1's id, 'wait for its report' and 'withdraw', and the launch record count is 1.
- After admin-fixture withdraws L1, a new start for agent-fixture-1 on machine-fixture-2 by admin-fixture is admitted, no start_unconfirmed is returned and the launch record count is 2.
- With L1 reading running from a verified report of session sess-fixture-1, a start for agent-fixture-1 by admin-fixture is given and keeps L2, whose id differs from L1's and which names agent-fixture-1, and L1 still reads running.
- Across that start, this card writes 0 session records: the sessions record fixture's bytes after L2 is kept are identical to its bytes before, the agent record count is 1, and agent-fixture-1's agent record is byte-identical to its bytes before L1 was given.
- After a verified report naming agent-fixture-1 and L2 arrives as session sess-fixture-2, L1 reads running tied to session sess-fixture-1 and L2 reads running tied to session sess-fixture-2.
- The signed bytes of every launch record event in the test store do not contain 'fixture-credential-value-1f3a'.

**Files:**
- create: crates/lys-identity/src/start/give.rs
- create: crates/lys-identity/src/start/command.rs
- create: crates/lys-identity/tests/start_launch_record.rs

**Checklist:**
- C212 — No credential value is on the command line or the clipboard, and a test reads both (CONFORMANCE 5.3).
- C213 — A launch record naming the machine, the executable, the working directory, the profile version and the credential ids is kept for every command given, reads back after a restart, and a start given again from it is a new record naming the one it was copied from (CONFORMANCE 5.4).
- C214 — A start request names only the agent, the profile version and the machine; the command given reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> before the reviewed profile version's own executable and its recorded arguments, unchanged and in order, with the recorded working directory, each value shell-quoted and refused by name outside its id grammar; it is not a lys subcommand, and a request that sets the executable or the working directory is refused by name.
- C216 — With no report a start reads unconfirmed, never not started; the screen says the request stands and warns against asking elsewhere, and a second start for the agent is refused as start_unconfirmed (CONFORMANCE 5.6).
- C221 — A start resolves its agent to the enduring agent record the provision brief DIRECTORY-011 keeps and is refused by name, writing nothing, when there is none; a start never creates or changes an agent record and writes no session record, and a start of an agent already running is given as a new launch record naming the same agent, whose session record the report makes in the sessions brief's store.

**Stories:**
- S84 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S87 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want an unanswered start shown as unconfirmed with its request standing, so that I do not ask elsewhere and start the agent twice.
- S89 (Operator, Installs and runs the standalone identity product) — As the operator, I want a kept launch record for every command given, so that I can give the same start again without looking inside a running process.
- S90 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that reads the command line and the clipboard and finds no credential value, so that giving a command never discloses a secret.
- S94 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want starting my agent again while it runs to become a second session of the same agent, so that starting never makes another agent and never changes the one I registered.

### R12: Answer a start through the route and the CLI, and prove that no start path spawns a process

The start route in crates/lys-identity-server/src/start.rs, mounted from crates/lys-identity-server/src/routes.rs, which DIRECTORY-005 creates and this requirement modifies only after it lands, gives a start (POST /api/agents/<agent>/start with the profile version and the machine), gives it again from a kept record (POST /api/launch-records/<id>/start-again), withdraws it (POST /api/launch-records/<id>/withdraw) and reads its state (GET /api/launch-records/<id>/state), each through the library of R1 to R11, and holds no start logic of its own: every check, refusal, launch record and command it answers is the library's. The CLI answer is lys identity start-command <agent> --profile-version <pv> --machine <m>, in crates/lys/src/identity/start.rs. The lys CLI has no identity group yet, so this card adds the group, in crates/lys/src/identity/mod.rs and crates/lys/src/identity/cli.rs, declared from crates/lys/src/cli.rs and crates/lys/src/main.rs, with this one subcommand. WHEN it is run, it SHALL ask the route for a start with the same three inputs and print exactly what the route answers, and nothing else, so a person can copy the command into the machine's shell. It is not a launcher: it starts nothing and holds no process. It reaches the route over HTTP, so crates/lys/Cargo.toml, Cargo.toml and Cargo.lock gain an HTTP client dependency that meets the workspace's pure-Rust rule. WHEN a start is given, the route SHALL answer with the command, its working directory and its launch record; WHEN a start is refused, the route SHALL return every refusal by its name in words, and the CLI prints that answer. THE SYSTEM SHALL NOT spawn a process on any start path: no start file in the library, the route or the CLI names a process-spawning API, and a test proves it both by reading those files and by giving a start whose executable would leave a marker if it ran. THE SYSTEM SHALL NOT put a credential value in the route's answer, the CLI's output or its errors.

**Acceptance:**
- POST /api/agents/agent-fixture-1/start with body {profile_version: pv-fixture-1, machine: machine-fixture-1}, every check fixture passing, answers with a command and a launch record id, and the marker file the fixture executable fixture-exec writes when run does not exist afterwards.
- lys identity start-command agent-fixture-1 --profile-version pv-fixture-1 --machine machine-fixture-1 against the test server, every check fixture passing, prints the command, the working directory 'fixture-cwd' and the launch record id, its stdout is byte-identical to the response body the test server recorded sending for that request, the marker file does not exist afterwards, and neither stdout nor stderr contains 'fixture-credential-value-1f3a'.
- POST /api/agents/agent-fixture-1/start with agent-fixture-1 suspended answers a body naming agent_not_active with words containing 'the agent is active', and carries no command.
- With agent-fixture-1 suspended, the stdout of lys identity start-command agent-fixture-1 --profile-version pv-fixture-1 --machine machine-fixture-1 is byte-identical to the refusal body the test server recorded sending, and it contains 'agent_not_active'.
- After POST /api/agents/agent-fixture-1/start by admin-fixture keeps L1, POST /api/launch-records/<L1>/withdraw by admin-fixture answers state 'withdrawn' naming admin-fixture, and POST /api/launch-records/<L1>/start-again by admin-fixture then answers a command and a launch record id L2 that differs from L1, whose record names L1 as the record it was copied from; the marker file does not exist afterwards.
- GET /api/launch-records/<L1>/state answers 'unconfirmed' with no report in the sessions record fixture, 'withdrawn' after admin-fixture's withdrawal, and 'running' with the withdrawal by admin-fixture beside it after a verified report naming agent-fixture-1 and L1 arrives in the fixture.
- For each of the 4 fixture refusals agent_unknown, start_right_missing, agent_not_active and start_unconfirmed, the route's answer body is byte-identical to the library's serialised refusal for the same inputs, and rg -n 'start_unconfirmed|check_record_missing|agent_not_active|LYS_AGENT_ID|LYS_LAUNCH_RECORD|LYS_CREDENTIAL_IDS' crates/lys-identity-server/src/start.rs crates/lys/src/identity/start.rs crates/lys/src/identity/cli.rs finds 0 lines.
- The no-spawn test reads every start file this brief names under crates/lys-identity/src/start/, crates/lys-identity-server/src/start.rs and crates/lys/src/identity/start.rs, asserts the count of files read equals the count it lists, and finds 0 occurrences of 'std::process', 'tokio::process', 'Command::new', 'fork(' and 'exec('. (CONFORMANCE 5.1)
- The same scanner run over the fixture source text 'let c = std::process::Command::new("x");' finds exactly 1 occurrence, so the scan is shown to fire.
- The route's JSON answer for the passing start does not contain 'fixture-credential-value-1f3a'.
- Reading crates/lys/src/identity/cli.rs finds exactly 1 subcommand declared by this brief, start-command, and no subcommand that runs, spawns or supervises a process.

**Files:**
- create: crates/lys-identity-server/src/start.rs
- create: crates/lys-identity-server/tests/start.rs
- create: crates/lys/src/identity/mod.rs
- create: crates/lys/src/identity/cli.rs
- create: crates/lys/src/identity/start.rs
- create: crates/lys/tests/identity_start.rs
- create: crates/lys-identity/tests/start_no_spawn.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/main.rs
- modify: crates/lys/Cargo.toml
- modify: Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C209 — No start path in the library, the route or the CLI spawns a process, and a test that reads the start files and gives a start with a marker-writing executable proves it (CONFORMANCE 5.1).
- C212 — No credential value is on the command line or the clipboard, and a test reads both (CONFORMANCE 5.3).
- C220 — The start route gives, gives again, withdraws and reads a start through the library and holds no start logic of its own, and the CLI answer lys identity start-command, the one subcommand of the identity group this card adds, prints exactly what the route answers for the same three inputs and starts nothing.

**Stories:**
- S90 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that reads the command line and the clipboard and finds no credential value, so that giving a command never discloses a secret.
- S91 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that fails if any start path spawns a process, so that lys stays a product that checks, gives and records and never runs an agent.

### R13: Build the Start drawer and the unconfirmed notice on the agent file

WHEN the admitted administrator, or the agent's responsible person once step 1's admission admits them, opens an agent's file, THE SYSTEM SHALL show a working Start control on DIRECTORY-005's surface (surface/identity/, which DIRECTORY-005 creates; its routes.tsx and generated/index.ts are modified here only after it lands), following Aion's structure with the identity orange accent (ADR-010), that opens the Start drawer: the profile version, the machine from the role's machines, the five checks by name with each result in words, and, only when every check passes, the command with its working directory, a Copy control and the launch record's facts (machine, the executable in the working directory, the profile version and credential ids, and its state). WHEN anyone else opens it, THE SYSTEM SHALL show no enabled Start control and SHALL show the refusal naming the agent and the right they lack. WHILE a launch record is unconfirmed, the notice SHALL say it is unconfirmed, that the request stands, and warn against asking elsewhere, that asking another machine could start it twice, offering the act that answers it: wait for its report, or withdraw it first. A withdrawn record SHALL read 'withdrawn', and a running one with a withdrawal SHALL show both. THE SYSTEM SHALL NOT show 'not started', SHALL NOT change any state when Copy is pressed, and SHALL NOT carry the mock-up's 'Simulate: it reported in' control or any sample record into the build.

**Acceptance:**
- Signed in as admin-fixture, the agent file of agent-fixture-1 shows an enabled Start control, and the drawer lists the five check names in R3's order.
- Signed in as person-fixture-2, the agent file of agent-fixture-1 shows no enabled Start control and its text contains 'agent-fixture-1' and 'start'.
- With every check fixture passing and the handle record fixture holding the values fixture-credential-value-1f3a and fixture-credential-value-2b7c, pressing Copy puts on the clipboard, read in the browser test with navigator.clipboard.readText(), exactly the command line the drawer shows, and the clipboard text contains neither of the 2 credential values the fixture holds (CONFORMANCE 5.3).
- Pressing Copy sends 0 mutating requests, and the launch record still reads 'Unconfirmed'. (CONFORMANCE 5.5)
- With agent-fixture-1 suspended, the drawer shows 'the agent is active' with its failed result and renders no command and no Copy control.
- The unconfirmed notice's text contains 'Unconfirmed', 'The request stands', 'could start it twice', 'wait for its report' and 'withdraw' (CONFORMANCE 5.6).
- After withdrawing L1 from the notice, L1 reads 'withdrawn'; after a verified report for L1 then arrives in the fixture, the file shows 'running' with 'withdrawn' beside it.
- No page text in any state the browser tests reach contains 'not started', compared without case.
- rg -n 'Simulate' surface/identity/src finds 0 lines.

**Files:**
- create: surface/identity/src/features/start/StartDrawer.tsx
- create: surface/identity/src/features/start/StartNotice.tsx
- create: surface/identity/tests/start.test.tsx
- create: surface/identity/tests/acceptance/start.spec.ts
- modify: surface/identity/src/routes.tsx
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C212 — No credential value is on the command line or the clipboard, and a test reads both (CONFORMANCE 5.3).
- C215 — An agent shows as running only when its verified signed report names its launch record, and copying the command changes no state (CONFORMANCE 5.5).
- C216 — With no report a start reads unconfirmed, never not started; the screen says the request stands and warns against asking elsewhere, and a second start for the agent is refused as start_unconfirmed (CONFORMANCE 5.6).
- C217 — A withdrawal by the giver or anyone holding the same right records who and when and that the request no longer stands, never that the agent did not start, and a report arriving afterwards shows running with the withdrawal beside it.
- C218 — Only the agent's responsible person and a directory administrator (in step 1, the admitted administrator) have a working Start; anyone else is refused by name, naming the agent and the right they lack.
- C219 — The Start drawer and the unconfirmed notice are built on DIRECTORY-005's surface, and DIRECTORY-005's R1 and boundary name Start as the one control that works in step 1.

**Stories:**
- S85 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.
- S86 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent shown as running only when its own signed report names the command I was given, so that copying a command is never mistaken for a start.
- S87 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want an unanswered start shown as unconfirmed with its request standing, so that I do not ask elsewhere and start the agent twice.
- S88 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to withdraw a start request I no longer want, so that the record says the request no longer stands without claiming the agent did not start.
- S90 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that reads the command line and the clipboard and finds no credential value, so that giving a command never discloses a secret.
- S92 (Person without the right to start, Opens an agent file that is not theirs to start) — As a person who is neither the agent's responsible person nor a directory administrator, I want a refusal naming the agent and the right I lack, so that I know why there is no working Start for me.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside that wall stops and names it, and the brief is revised before that file is edited.
- lys never runs an agent: no start path spawns a process, opens a terminal, or asks a sandbox, VM or container runner to run anything (CONFORMANCE row 5.7 stays out, ADR-007).
- No record the checks read is built, copied or faked here: the lifecycle record, the profile version and its review, the role's machines, the virtual credentials, the egress list, the sessions record and the provision record are read from their owners, and no sample record or default stands in for a missing one.
- How a start command proves itself, such as a one-time code for one machine for a few minutes, stays proposed: no proof mechanism is implemented and the command carries none.
- No credential value appears on the command line, the clipboard, the launch record, a log line, an error or a test fixture that is not generated for the test.
- lys/session-start/v1 is not defined or changed here; it belongs to the sessions brief. No WIRE-FORMATS row, no lys-core code and no published lys-core 0.2.0 format changes.
- The only control made working in step 1 is Start; every other grant, launch-profile, secret and memory control stays not working in step 1.
- A grant to start an agent held by someone other than its responsible person or a directory administrator is not built.
- The directory's launch record is not the home's template render event (ADR-012); no home file changes.
- The mock-up's 'Simulate: it reported in' control and its sample review records never enter the build (P5).
- The IDENTITY-001 files do not change, and no Cambium, aion or argus file changes.
- The command given is the profile version's own executable with the arguments it records, invoked in its recorded working directory. It is not a lys subcommand: no lys launcher subcommand is added, and the mock-up's 'identity start <slug>' line is illustrative only.
- A start never creates or changes an agent record: DIRECTORY-011's enduring agent record is read to resolve the agent, and no session record is written here, because the sessions brief keeps them.
- The command given is never extended beyond env LYS_AGENT_ID, LYS_LAUNCH_RECORD and LYS_CREDENTIAL_IDS before the recorded executable and its recorded arguments: no other variable, argument, flag or wrapper is added.

## Verification

- From the repository root: cargo fmt --all; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps; sh scripts/design/gate.sh, all clean.
- From the repository root: rg -n 'std::process|tokio::process' crates/lys-identity/src/start crates/lys-identity-server/src/start.rs crates/lys/src/identity/start.rs finds 0 lines.
- From the repository root: rg -n -i 'not started' crates/lys-identity/src/start surface/identity/src finds 0 lines.
- Each of CONFORMANCE rows 5.1 to 5.6 is named in at least one test name or test comment under crates/lys-identity/tests, crates/lys-identity-server/tests, crates/lys/tests and surface/identity/tests, and the report lists which test covers which row.
- R10's dev record carries the session-start ruling: the launch record id is added to lys/session-start/v1 by the sessions brief d5055cc1 before its tag freezes, so that its signed message holds the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge; the amended brief is DIRECTORY-015, named in depends_on by its run d5055cc1.
- Frontend checks pass with strict types and the generated API schema, the clipboard test included.
- R9's dev record carries the amendment to the roles card Ink1H1Os: the executable, the arguments and the working directory are fields of its profile version record.
- From the repository root: rg -n 'identity start <slug>' crates surface finds 0 lines, and no subcommand the CLI adds is a launcher: lys identity start-command prints a command and never runs one.
