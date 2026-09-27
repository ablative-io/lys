# directory — what was asked, what it means, and what was written

## The words, as they were typed

Starting an agent is checked, given as a command and recorded, and lys never runs the agent itself.

This card covers the six rows of section 5 of the identity CONFORMANCE document on lys main 7b536253, Starting an agent, and each row gets at least one acceptance line that tests it.
Nothing from that section is on main yet.
It stands beside the sessions brief d5055cc1 and the provision brief DIRECTORY-011, and reads their records rather than keeping its own copy.

Row one is that lys does not run agents, and a test shows that no start path spawns a process.
Row two is the checks before a command is given, which are that the agent is active, its profile version is reviewed from a record, the machine is allowed for the role, its virtual credentials are valid, and the machine may reach what the profile needs from its egress list.
A failed check names itself in words and no command is given.
Row three is that no credential value is ever on the command line or the clipboard, and a test reads both.
How the command proves itself stays proposed until the launch design chooses it, and this card does not settle it.
Row four is a launch record kept for every command given, naming the machine, the executable, the working directory, the profile version and the credential ids, so a start can be given again without looking at a running process.
Row five is that an agent shows as running only when its signed report names that launch record, and copying the command is not a start.
Row six is that no report means unconfirmed, never not started, and the screen says the request stands and warns against asking elsewhere.
Archie answers this card's rounds and signs it off.


These rulings were given in an earlier run of this brief, which ended when the author pool ran out. They stand, and the brief carries them.

The sessions brief adds the launch record id to lys/session-start/v1 before the tag freezes. No agent signs under it yet, so the change is free now and costs a version later. This card writes the ruling into its depends_on and its dev record, and the lead sends the same ruling to d5055cc1, DIRECTORY-014, as an amendment to its signed message, which then holds the tag, the agent id, the launch record id and the nonce. Running is tied to the command given through that id. Answered by Archie, lead for the identity line.

It builds none of those records and waits for their cards. The check against each is written as a named check reading its owner's record, and each check's R row names the card that makes the record and is blocked on it, the ROLES card Ink1H1Os for the profile version and the role's machines, SECRETS-002 for virtual credentials, and the network row 8.5 for egress. The row that needs only the lifecycle record, the agent is active, ships first. No command is given while a check is missing, and a start attempted then is refused by name, naming the check whose record does not exist yet. Nothing is faked to let a start through. Answered by Archie.

In scope. CONFORMANCE rows 5.1 to 5.6 are front-end tests, so the agent file's Start drawer and the unconfirmed notice are this card's, built on DIRECTORY-005's surface, which goes in depends_on. The route and CLI answer land as well, and the screen rows are blocked on DIRECTORY-005 landing. Answered by Archie.

The agent's responsible person and a directory administrator. In step 1 that means the admitted administrator, as P9 says, and the responsible person gains the button when step 1's admission admits them. A grant to start held by someone else is not in this card and is named under further units not written. Anyone else is refused by name, naming the agent and the right they lack, and sees no working Start button. Answered by Archie.

Yes, it can be withdrawn, by the person who gave it or anyone holding the same right. Withdrawing records that the request no longer stands, with who and when. It never records that the agent did not start, because nobody can know that. If a signed report later arrives for the withdrawn launch record, the session shows as running and the withdrawal is shown beside it. The screen's words say 'withdrawn', never 'not started'. Acceptance lines cover a withdrawal and a report arriving after one. Answered by Archie.

From the reviewed profile version's record, as the mock-up draws it. The command a person gives must be the one that was reviewed, so the executable and the working directory are fields of the profile version, and the start request names only the agent, the profile version and the machine. A start request that tries to set either is refused by name. If the profile version's record does not yet hold those fields, R9 adds them to the profile version under the roles card Ink1H1Os's record, named in depends_on, and stays blocked on it by that name. Answered by Archie, lead for the identity line.

depends_on names the sessions brief by its run d5055cc1, whatever id it lands under, because the next-free-id rule may renumber it. The directory identifier the sessions draft signs today stays in the session-start message, beside the launch record id. Answered by Archie.

Rulings of the lead, Archie, given on 27 September 2026 to the run 6bd1fe16-ea30-4a4e-9b0a-f8c7fb8c72a7 in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

This card supersedes that line for Start alone. In step 1 the admitted administrator has a working Start on the agent file. The brief amends DIRECTORY-005's R1 and its boundary with one appended sentence naming Start, under this card, as the one control that works in step 1. Every other grant, launch-profile, secret and memory control stays not working in step 1, as written. The responsible person gains the button when step 1's admission admits them. Answered by Archie, lead for the identity line.

It makes a new launch record with its own id every time. The kept record is copied, and the new record names the record it was copied from, so every report confirms exactly one command and a withdrawal ties to one command. No record is ever re-issued. An acceptance line gives a start again from a kept record and asserts two distinct record ids, the second naming the first. Answered by Archie.

Refused by name. While a start for an agent stands unconfirmed, a second start for that agent, on any machine, is refused as start_unconfirmed. The refusal names the standing request and the act that answers it: wait for its report, or withdraw it first. So one agent never gets two commands at once. The screen's warning says the same. An acceptance line asks twice and asserts the refusal and that one launch record exists. Answered by Archie.

A start reads DIRECTORY-011's agent record, the enduring identity, and nothing else of that brief. The check that the agent exists resolves the agent id named on the start to its registered agent record, and it refuses by name when no such record exists, because starting a session never creates an agent (DIRECTORY-CONTRACT.md 'Enduring agents and session credentials', rule 3). The session record the start writes points at exactly that enduring agent id (rule 2), and the agent record is not changed by the start (rule 1: it holds no session credential). The brief names that check and that field, with an acceptance line: a start for an agent id with no record is refused by name and writes nothing. Answered by Archie, lead for the identity line.

The profile version's own executable, with the arguments the profile version records, invoked in its recorded working directory. This card adds no lys launcher subcommand; the mock-up's 'identity start <slug>' line is illustrative only and the brief says so. The brief fixes the command as built from those recorded fields alone, states what it must name and must not carry as it already does, and an acceptance line compares the given command with the profile version's recorded executable, arguments and working directory. Answered by Archie.

Given. A second start for an agent that already reads as running is a second session of the same agent, which the contract allows (rule 5): it writes a new session record with its own session id pointing at the same enduring agent id, and it never creates or changes the agent record. Nothing about the first session changes. The session credential's form stays with RM-029, so this brief asserts the second session record and the unchanged agent record only, with an acceptance line for each. Answered by Archie.

Only the report makes it, as the words say. This card reads the session records and keeps no copy, so a second start of a running agent writes only a new launch record, and the session record is made in d5055cc1's store when that start's signed report arrives. My round 2 answer is corrected to that. The acceptance lines become: a second start writes a second launch record naming the same enduring agent id, the agent record is unchanged, and this card writes no session record. Answered by Archie, lead for the identity line.

Move the state derivation first. R11's state.rs, which gives unconfirmed while no verified lys/session-start/v1 report exists, with a withdrawn request no longer standing and a start of an agent already running being given, becomes the requirement before the start refusal. Renumber so the state comes before R10, and R10's start_unconfirmed refusal and its acceptance line stay with the start and cite the state requirement by its new number. No requirement relies on a state that a later one defines. Every reference follows the renumbering: the checklist, the coverage, the acceptance lines and the rendered files. Answered by Archie, lead for the identity line.

As environment assignments before the executable, so the recorded arguments stay exactly as the profile version records them and nothing is appended to them. The given command reads: env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> <executable> <recorded arguments>, with the working directory set to the recorded one. The three are ids only, never a credential value, so row 5.3's rule that no credential value is on the command line holds. Each value is shell-quoted and refused by name if it holds anything outside its id grammar. An acceptance line parses the given command and asserts the three assignments, the executable, the recorded arguments unchanged and in order, and that no credential value appears. The session report names the same launch record id it read from LYS_LAUNCH_RECORD. Answered by Archie, lead for the identity line.

## What the survey found, and its angles

The words ask for one directory brief that makes the six test rows of CONFORMANCE section 5 (Starting an agent, 5.1 to 5.6) true: lys checks an agent before a start, gives a command built only from the reviewed profile version's recorded fields, keeps a launch record for every command, and never runs the agent itself. It shows the agent as running only when a signed lys/session-start/v1 report names that launch record. With no report it shows unconfirmed, and never not started. It reads the records of other cards (lifecycle, roles Ink1H1Os, SECRETS-002, network 8.5, provision DIRECTORY-011, sessions d5055cc1) and never copies them. Every check whose record does not exist yet refuses the start by name. The lead's rulings already fix who may start, withdrawal, one standing start per agent, start-again as a new record, and the env-assignment command form. They also amend DIRECTORY-005 so Start is the one control that works in step 1, and the sessions brief so its signed message carries the launch record id.

### What the tree holds

- `docs/design/identity/CONFORMANCE.md` — Lines 53-63 hold section 5. Rows 5.1-5.6 are this card's acceptance map, and 5.7 (terminal, sandbox, VM, container runners) is proposed and out of scope. Row 5.4 names the Cambium defect 'Hand-started workers have no launch record', and row 5.3 keeps the command's own proof proposed.
- `docs/design/identity/mockup/index.v5.html` — startDrawer (lines 1626-1654) draws the five checks in the order the words give, the illustrative 'identity start <slug> --code' line, the launch record facts (machine, runs, configuration, state) and 'Nothing is shown as running until it does'. Lines 1478-1491 draw the unconfirmed wording: 'It may or may not have started. The request stands'.
- `docs/design/directory/briefs/DIRECTORY-005.md` — Its R1 acceptance and boundary say 'No grant, launch, secret or memory control is presented as working in step 1'. The ruling amends R1 and that boundary with one appended sentence naming Start. The Start drawer is built on its surface/identity/ and crates/lys-identity-server/src/routes.rs, and neither exists on main.
- `docs/design/directory/DESIGN.md` — P3 says registration creates no running state. P4 says one signed event is both the change and its audit. P5 says no success before durable evidence, which matches unconfirmed. P6 says step 1 implements neither session history nor launch, and P9 covers the admitted administrator. The 'Road step 2 onward' non-goal lists session launch, so this card needs the same appended step-2 line the sessions draft carries. CN9 requires a file manifest review.
- `docs/design/directory/design.json / checklist.json / stories.json` — The cluster files the new brief's structure rows, checklist items and stories go into. There are 96 structure rows today, and scripts/design/gate.sh requires the rendered markdown to match the JSON.
- `docs/design/decisions.json` — Holds ADR-001..ADR-018 on main. ADR-007 is the governing decision. The new decisions (launch record, profile-version fields, withdrawal) would be added here.
- `origin draft/directory/d5055cc1-73dc-49b5-ad25-449f195b8796 : docs/design/directory/briefs/DIRECTORY-015.json` — The sessions brief as it stands (13:30 today). It now carries id DIRECTORY-015, not DIRECTORY-014. Its lys/session-start/v1 proof signs the agent's directory id, the directory identifier and a directory-issued challenge. It contains no mention of a launch record, so the ruled amendment has not been applied. ADR-046 creates the session at the first report-back, and the agent's certificate must be issued.
- `origin brief/directory/28d15c0d-5a90-4895-ae9c-5b5dc3b4c8c8 : docs/design/directory/briefs/DIRECTORY-011.json` — The provision brief. Its R1 writes '## Enduring agents and session credentials' into docs/design/identity/DIRECTORY-CONTRACT.md (rules 1-5 the rulings cite) and depends on DIRECTORY-003. DIRECTORY-CONTRACT.md does not exist on main.
- `origin brief/directory/lifecycle-hand : docs/design/directory/briefs/DIRECTORY-009.json` — The lifecycle card that owns the folded state the 'agent is active' check reads (R3 fold, R4 state-first check). It depends on DIRECTORY-003 and DIRECTORY-006, and it also uses the id DIRECTORY-009.
- `origin draft/directory/6bd1fe16-ea30-4a4e-9b0a-f8c7fb8c72a7 : docs/design/directory/briefs/DIRECTORY-009.json` — The earlier run of this very card did write a draft. It has 13 requirements (R1-R13, including R8 amending DIRECTORY-005, R9 profile fields, R10 state, R11 launch record, R12 route/CLI and no spawn, R13 drawer) and ADR-019/020/021 in its decisions.json. It predates the renumbering ruling that moves the state derivation before the start refusal.
- `docs/design/secrets/briefs/SECRETS-002.md` — The virtual-credential (handle) record the credential check reads. Its records are door-owned, in the cambium repository (crates/cambium-store/src/traits/secrets.rs), not in lys. Row 5.3 names SECRETS-002 R5 as an amend.
- `crates/lys/src/commands/ (and a future crates/lys/src/identity/)` — The CLI answer lands here. DIRECTORY-002 adds src/identity/, which does not exist on main. The ruling says no lys launcher subcommand is added.
- `crates/lys-home (ADR-012 render-launch)` — This is a prior art in the tree for 'prints the launch line and never runs it'. It is a separate launch line (harness templates), and this card must not conflate it with the directory's start command.

### What was already decided

- ADR-007 — The product gives the start command and never runs it. The command carries identity and handles, never a credential value, and is rendered from the kept launch record. A started agent reports back.
- ADR-003 — Everything is pegged to a human authority. The responsible person is the ceiling for what the agent may do.
- ADR-011 — Proposed, not decided: the states are registered, active, suspended and retired. The 'agent is active' check rests on this proposed decision.
- ADR-008 — The certificate shows as not issued until one is issued. The sessions draft's report-back needs an issued certificate, so running depends on issuance.
- ADR-001 — Credentials sit behind handles in the door and never leave it. The command carries credential ids only.
- ADR-004 — Any engine (manifold, aion, a customer's own) runs the agent. lys is not the engine.
- ADR-010 — The Start drawer follows Aion's appearance with the identity orange accent.
- ADR-012 — lys-home prints a harness launch line and never runs it. It is a separate artifact from this start command.
- directory P3/P4/P5/P6/P9 — Registration creates no running state. Each change is one signed event. Nothing is a success before durable evidence. Step 1 implements no launch, so this card needs a step-2 line. The admitted administrator is configured by issuer and subject.
- directory CN9/CN12 — A new module needs a reviewed file manifest. No brief is dispatched while its foundations are unbuilt.
- DIRECTORY-005 R1 and boundary — No launch control works in step 1. This card amends that for Start alone.
- CONFORMANCE 5.1-5.7 — Six test rows, and 5.7 proposed as separate products. The build order puts 5.1-5.6 in step 3 with the lifecycle rows.
- d5055cc1 draft ADR-046 — The session is created at the agent's first signed report-back over a directory challenge, and the credential is kept as a hash. It does not yet name a launch record.
- 6bd1fe16 draft ADR-019/020/021 — Draft decisions: a signed launch record per command tied to session-start, the executable and working directory as profile-version fields, and a withdrawal that records only that the request no longer stands.

### What was measured

- CONFORMANCE section 5 rows: 7 rows (5.1-5.7): 6 test or test/proposed rows for this card, and 5.7 proposed and out of scope
- Checks drawn in the mock-up's startDrawer: 5 (active, profile reviewed, machine allowed for role, virtual credentials valid, egress)
- Section-5 artifacts on main 7b53625: 0: no crates/lys-identity, no crates/lys-identity-server, no surface/, no DIRECTORY-CONTRACT.md, no IDENTITY-EVENTS.md, no 'session-start' string anywhere in the tree
- Directory briefs on main: 7 (DIRECTORY-001 to 006, and 008). DIRECTORY-011 and d5055cc1 exist only on origin branches
- Decisions on main: 18 (ADR-001..ADR-018). The 6bd1fe16 draft has 21, and the d5055cc1 draft adds ADR-046
- design.json structure rows in the directory cluster: 96
- Earlier run 6bd1fe16 draft: DIRECTORY-009 with 13 requirements (R1-R13), on origin draft/directory/6bd1fe16-ea30-4a4e-9b0a-f8c7fb8c72a7 at 4f74fa9
- Sessions draft d5055cc1 id and launch-record mentions: DIRECTORY-015 (not 014). Checklist C72-C90. 0 mentions of a launch record. Last commit 2026-09-27 13:30 +1000
- Origin refs carrying a DIRECTORY-009.json: 10 (brief/lifecycle-hand, brief/9c512bc3 sign-in refusal, draft 6bd1fe16 and 7 other drafts)
- Current holder of DIRECTORY-014: draft d40aa430, a SpiceDB permission-check card, not the sessions brief
- origin/main compared with the words' commit: origin/main is at fa3dd53, 2 commits past 7b53625 (row 6.4 ca issue --log). Section 5 is still absent
- mock-up index.v5.html size: 197,852 bytes
- DIRECTORY-005.md length: 109 lines. Boundary line 'No grant, launch, secret or memory control is presented as working in step 1' present once

### What it means for the other projects

- cambium — The virtual-credential check reads SECRETS-002's handle record, which lives in the cambium door (crates/cambium-store, crates/cambium-door), so lys needs a read seam into the door. Row 5.4's launch record is the fix for the Cambium defect 'Hand-started workers have no launch record'. Cambium's hand-started workers could later consume the given command.
- aion — Under ADR-004, aion's workflow engine is one runtime that may take the given command and report back. The card's own build goes through aion's chain (brief_card, sign-off, card_build_v3, src_pr, src_land). Nothing in aion changes.
- method — The brief must validate against the method's brief schema and render cleanly through scripts/design (gate.sh). The Start drawer follows the estate design and the identity orange tokens (ADR-010) without taking a build dependency.

### The decisions it stands on

- ADR-007 (honour) — This card is ADR-007 made testable: it checks, gives a command, records, never runs the agent, and treats a report as the start.
- ADR-003 (honour) — Start is admitted only from the responsible person or the administrator, and the start request carries no authority beyond theirs.
- ADR-001 (honour) — The command carries credential ids only. Credential values stay in the door and never reach the command line or clipboard.
- ADR-004 (honour) — Whatever engine the person chooses runs the command, and lys is not an engine.
- ADR-011 (honour) — The first check reads the active state from the lifecycle record. The ADR is still proposed, so the check stands on it as written.
- ADR-008 (honour) — The report that makes an agent running is signed with the agent's enrolled key, per the sessions draft.
- ADR-010 (honour) — The Start drawer and the unconfirmed notice use the identity product's appearance and accent.
-  (new) — Every command given keeps its own launch record (a new record on every start again, naming its source), and running is tied to it by the launch record id carried in lys/session-start/v1. The 6bd1fe16 draft numbered this ADR-019.
-  (new) — The executable, arguments and working directory are fields of the reviewed profile version, never of a start request. The 6bd1fe16 draft numbered this ADR-020.
-  (new) — A withdrawal records only that the request no longer stands, with who and when, and never that the agent did not start. The 6bd1fe16 draft numbered this ADR-021.
-  (new) — One standing start per agent: start_unconfirmed refuses a second start while one is unconfirmed. The command form is env LYS_AGENT_ID, LYS_LAUNCH_RECORD and LYS_CREDENTIAL_IDS before the recorded executable.

### What it requires

- A test shows no start path (route, CLI or library) spawns a process, and no start module references a process-spawning API.
- A start request names only the agent id, the profile version and the machine. One that sets an executable or working directory is refused by name.
- A start for an agent id with no registered agent record (DIRECTORY-011) is refused by name and writes nothing, and a start never changes the agent record.
- The five checks run before any command: active, profile version reviewed, machine allowed for the role, virtual credentials valid, egress. Each failure names itself in words and no command or launch record is given.
- A check whose owner's record does not exist yet refuses the start by naming that check and its owner card (Ink1H1Os, SECRETS-002, row 8.5), and nothing is faked to let a start through.
- Only the responsible person or a directory administrator may start, start again or withdraw (in step 1, the admitted administrator). Anyone else is refused by name, naming the agent and the missing right, and sees no working Start button.
- Every command given writes one launch record naming the machine, executable, working directory, profile version and credential ids.
- The given command is exactly: env LYS_AGENT_ID=… LYS_LAUNCH_RECORD=… LYS_CREDENTIAL_IDS=… <executable> <recorded arguments>, in the recorded working directory. Each value is shell-quoted and refused if outside its id grammar. A test parses it against the profile version's fields.
- No credential value appears on the given command line or in the value copied to the clipboard, and a test reads both.
- The state derivation comes before the start refusal requirement: unconfirmed while no verified lys/session-start/v1 report names the launch record, running once one does, withdrawn when the request no longer stands.
- A second start while one stands unconfirmed is refused start_unconfirmed, naming the standing request and the act that answers it, and exactly one launch record exists afterwards.
- Start again writes a new launch record with a distinct id naming the record it was copied from.
- A second start of an agent already running writes a second launch record for the same enduring agent id, leaves the agent record unchanged, and writes no session record.
- A withdrawal records who and when and never 'not started'. A report arriving after a withdrawal shows running with the withdrawal beside it.
- The screen with no report says unconfirmed, says the request stands and warns against asking elsewhere, and never says 'not started'.
- The route and the CLI both answer a start. The Start drawer and the unconfirmed notice are built on DIRECTORY-005's surface, and the screen rows are blocked on DIRECTORY-005 landing.
- DIRECTORY-005's R1 and boundary each gain one appended sentence naming Start, under this card, as the one control that works in step 1.
- depends_on names d5055cc1, Ink1H1Os, SECRETS-002, DIRECTORY-005 and DIRECTORY-011, and the network row 8.5 is named as the blocker of the egress check.
- Each of CONFORMANCE rows 5.1-5.6 has at least one acceptance line testing it, and validate.py, check-coverage.py and gate.sh pass on the cluster.

### What must not change

- lys never runs, spawns or supervises an agent process.
- No lys launcher subcommand is added, and the mock-up's 'identity start' line stays illustrative.
- The command's own proof (the one-time code) is not chosen or built, and row 5.3's proof stays proposed.
- This card keeps no copy of the session, agent, profile-version, handle or egress records. It reads each from its owner.
- No existing record is re-issued, and a launch record is never mutated into another command.
- The agent record is never created or changed by a start.
- Every other grant, launch-profile, secret and memory control in DIRECTORY-005 stays not working in step 1.
- Grants to start held by someone other than the responsible person or an administrator are out of scope.
- Rows 5.7 (terminal, sandbox, VM, container runners) are not built.
- A shipped wire format or tag is never mutated. The launch record id enters lys/session-start/v1 only before that tag freezes.
- The IDENTITY-001 files and the other cards' briefs are not rewritten beyond the two ruled appended sentences in DIRECTORY-005.

### What we must put in place first

- The lead sends the amendment to d5055cc1 (now drafted as DIRECTORY-015) so its lys/session-start/v1 signed message carries the launch record id beside the agent id, the directory identifier and the challenge (nonce), before that tag freezes.
- The lead sends the amendment to the roles card Ink1H1Os so its profile version record holds the executable, arguments and working directory.
- A brief id is chosen that no other origin ref holds (DIRECTORY-009, 011, 014 and 015 are all taken on origin branches).

### The risks

- The sessions draft d5055cc1 has moved on (it is now DIRECTORY-015 with a challenge-based proof and ADR-046) without the launch record id. If its tag freezes first, row 5.5 can be tied to a launch record only by a new tag version.
- Row 8.5 has no card, so every start is refused at the egress check until one exists and no end-to-end start can be demonstrated.
- Running also depends on certificate issuance (the sessions draft refuses a report-back from an agent whose certificate is not issued), which is a further card not named in the words.
- The 'agent is active' check stands on ADR-011, which is still proposed, and on the lifecycle card, whose id DIRECTORY-009 collides with nine other refs.
- Two DIRECTORY-009 drafts (6bd1fe16 and lifecycle-hand) and a sign-in-refusal brief share one id, so references may point at the wrong card.
- The virtual-credential check crosses into the cambium door's repository, so a lys test cannot see that record without a fixture seam. That risks a check that agrees only with itself.
- The new launch record and withdrawal event kinds are signed formats and need an adversarial review before the first is signed. Skipping it would freeze a draft.
- Many blocked rows (profile, machines, credentials, egress, running, screens) mean most of the card cannot dispatch for a long time, and a lead may be tempted to hand-build the first row outside the chain.

### Still open

- The earlier run 6bd1fe16 did write a 13-requirement DIRECTORY-009 draft with ADR-019..021 to origin. Does the author build from that draft, or discard it and write afresh? The sentence of the words it stands on: "That run took every answer and then failed before writing, when the account pool was at its usage limit.". Why only the lead can settle it: The tree contradicts the sentence: origin draft/directory/6bd1fe16-ea30-4a4e-9b0a-f8c7fb8c72a7 (4f74fa9) holds docs/design/directory/briefs/DIRECTORY-009.json and decisions ADR-019/020/021. Which base is used changes the decision ids and requirement text the lead will sign.
- Main's DIRECTORY-001 boundary says no row brief is dispatched until Waffles has reviewed it, and DIRECTORY-005 names 'Waffles' review' as a blocker. Does Archie's sign-off replace Waffles' review for this card? The sentence of the words it stands on: "Archie answers this card's rounds and signs it off.". Why only the lead can settle it: docs/design/directory/briefs/DIRECTORY-005.md and DIRECTORY-008.md cite the DIRECTORY-001 boundary 'no row brief is dispatched until Waffles has reviewed it'. The sentence names a different signer, and the brief's blocked_by line changes with the answer.
- The sessions draft's lys/session-start/v1 signs over a directory-issued challenge. Is the 'nonce' in the amendment that challenge, and should the amendment go to the current d5055cc1 head, which carries id DIRECTORY-015? The sentence of the words it stands on: "This card writes the ruling into its depends_on and its dev record, and the lead sends the same ruling to d5055cc1, DIRECTORY-014, as an amendment to its signed message, which then holds the tag, the agent id, the launch record id and the nonce.". Why only the lead can settle it: The tree conflicts on two points. The d5055cc1 draft (bfca0d5) is DIRECTORY-015, and DIRECTORY-014 is SpiceDB draft d40aa430. Its signed message is the agent's directory id, the directory's identifier and a challenge, with no launch record id. The fields a running report must carry depend on the answer.

### The units beyond the first

- A grant to start an agent held by someone other than its responsible person or an administrator — The ruling names it as not in this card. It needs the grant representation that the design records as open.
- The launch command's own proof (one-time code bound to a machine and a time window) — Row 5.3 keeps it proposed until the launch design chooses it, and it touches SECRETS-002 R5.
- The network record: machines, runtimes, reporting state, where each role may run, egress lists (CONFORMANCE 8.5) — The egress check is blocked on it and no brief or card exists for it.
- Add the launch record id to lys/session-start/v1 in the sessions brief d5055cc1 — It lands in d5055cc1's record, not this card's. This card only depends on it.
- Record the executable, arguments and working directory on the reviewed profile version in the roles card Ink1H1Os — The profile version record belongs to the roles card, and this card reads it and is blocked on it.
- Runtimes that run the given command: a terminal view, sandbox, VM or container runners (CONFORMANCE 5.7) — They are proposed as separate products under ADR-007 and ADR-004, and lys does not run agents.

### The smallest complete shape

One directory brief with its design, checklist, stories and decisions entries, rendered and gate-clean. It covers CONFORMANCE rows 5.1-5.6 in full, as requirements in the ruled order (state derivation before the start refusal), each check a named row blocked on its owner card. It carries the two appended DIRECTORY-005 sentences and the d5055cc1 and Ink1H1Os amendments in depends_on and the dev record. Of its rows, the start request, the admission, the 'agent is active' check, the launch record, the given command, the no-spawn proof and the route/CLI answer are the first to dispatch once DIRECTORY-003, DIRECTORY-011 and the lifecycle record have landed. Every other check refuses by name until its owner's record exists.

## The roadmap row

- **RM-054** — Check, give and record an agent's start command, and never run the agent (feature, idea)
- Summary: Make CONFORMANCE section 5 rows 5.1 to 5.6 of the identity product true: before a start command is given, five named checks read their owners' records and a failed check names itself; the command given is env LYS_AGENT_ID, LYS_LAUNCH_RECORD and LYS_CREDENTIAL_IDS assignments before the reviewed profile version's own executable and recorded arguments, and carries no credential value; every command given keeps a signed launch record; an agent shows as running only when its signed report names that record; no report means unconfirmed, never not started, and a second start while one stands is refused as start_unconfirmed; and a request can be withdrawn without claiming the agent did not start. The route, the CLI answer, the agent file's Start drawer and the unconfirmed notice all land, and lys never spawns a process.
- Asked by: tom on 2026-09-27T14:30:00+10:00
- Context: The card for CONFORMANCE section 5, Starting an agent, on lys main 7b53625, written by the brief method from the card's words and the lead's rulings they carry. The earlier run 6bd1fe16 did write a draft, contrary to the words' sentence that it failed before writing; the lead withdrew that sentence and ruled that this brief is built from that draft with every ruling applied over it, and that its ids give way to the next free ids past every brief, draft and hand head, so DIRECTORY-009 is DIRECTORY-029 and ADR-019 to ADR-021 are ADR-086 to ADR-088. The lead also ruled that the nonce in the session-start ruling is the directory-issued challenge and that the amendment goes to d5055cc1's current head, DIRECTORY-015, not DIRECTORY-014; and that the DIRECTORY-001 boundary stands, so the words' sentence on sign-off reads as amended: Archie answers the rounds and signs off after Waffles' review of the written brief.
- Quote: Starting an agent is checked, given as a command and recorded, and lys never runs the agent itself.

This card covers the six rows of section 5 of the identity CONFORMANCE document on lys main 7b536253, Starting an agent, and each row gets at least one acceptance line that tests it.
Nothing from that section is on main yet.
It stands beside the sessions brief d5055cc1 and the provision brief DIRECTORY-011, and reads their records rather than keeping its own copy.

Row one is that lys does not run agents, and a test shows that no start path spawns a process.
Row two is the checks before a command is given, which are that the agent is active, its profile version is reviewed from a record, the machine is allowed for the role, its virtual credentials are valid, and the machine may reach what the profile needs from its egress list.
A failed check names itself in words and no command is given.
Row three is that no credential value is ever on the command line or the clipboard, and a test reads both.
How the command proves itself stays proposed until the launch design chooses it, and this card does not settle it.
Row four is a launch record kept for every command given, naming the machine, the executable, the working directory, the profile version and the credential ids, so a start can be given again without looking at a running process.
Row five is that an agent shows as running only when its signed report names that launch record, and copying the command is not a start.
Row six is that no report means unconfirmed, never not started, and the screen says the request stands and warns against asking elsewhere.
Archie answers this card's rounds and signs it off.


These rulings were given in an earlier run of this brief, which ended when the author pool ran out. They stand, and the brief carries them.

The sessions brief adds the launch record id to lys/session-start/v1 before the tag freezes. No agent signs under it yet, so the change is free now and costs a version later. This card writes the ruling into its depends_on and its dev record, and the lead sends the same ruling to d5055cc1, DIRECTORY-014, as an amendment to its signed message, which then holds the tag, the agent id, the launch record id and the nonce. Running is tied to the command given through that id. Answered by Archie, lead for the identity line.

It builds none of those records and waits for their cards. The check against each is written as a named check reading its owner's record, and each check's R row names the card that makes the record and is blocked on it, the ROLES card Ink1H1Os for the profile version and the role's machines, SECRETS-002 for virtual credentials, and the network row 8.5 for egress. The row that needs only the lifecycle record, the agent is active, ships first. No command is given while a check is missing, and a start attempted then is refused by name, naming the check whose record does not exist yet. Nothing is faked to let a start through. Answered by Archie.

In scope. CONFORMANCE rows 5.1 to 5.6 are front-end tests, so the agent file's Start drawer and the unconfirmed notice are this card's, built on DIRECTORY-005's surface, which goes in depends_on. The route and CLI answer land as well, and the screen rows are blocked on DIRECTORY-005 landing. Answered by Archie.

The agent's responsible person and a directory administrator. In step 1 that means the admitted administrator, as P9 says, and the responsible person gains the button when step 1's admission admits them. A grant to start held by someone else is not in this card and is named under further units not written. Anyone else is refused by name, naming the agent and the right they lack, and sees no working Start button. Answered by Archie.

Yes, it can be withdrawn, by the person who gave it or anyone holding the same right. Withdrawing records that the request no longer stands, with who and when. It never records that the agent did not start, because nobody can know that. If a signed report later arrives for the withdrawn launch record, the session shows as running and the withdrawal is shown beside it. The screen's words say 'withdrawn', never 'not started'. Acceptance lines cover a withdrawal and a report arriving after one. Answered by Archie.

From the reviewed profile version's record, as the mock-up draws it. The command a person gives must be the one that was reviewed, so the executable and the working directory are fields of the profile version, and the start request names only the agent, the profile version and the machine. A start request that tries to set either is refused by name. If the profile version's record does not yet hold those fields, R9 adds them to the profile version under the roles card Ink1H1Os's record, named in depends_on, and stays blocked on it by that name. Answered by Archie, lead for the identity line.

depends_on names the sessions brief by its run d5055cc1, whatever id it lands under, because the next-free-id rule may renumber it. The directory identifier the sessions draft signs today stays in the session-start message, beside the launch record id. Answered by Archie.

Rulings of the lead, Archie, given on 27 September 2026 to the run 6bd1fe16-ea30-4a4e-9b0a-f8c7fb8c72a7 in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

This card supersedes that line for Start alone. In step 1 the admitted administrator has a working Start on the agent file. The brief amends DIRECTORY-005's R1 and its boundary with one appended sentence naming Start, under this card, as the one control that works in step 1. Every other grant, launch-profile, secret and memory control stays not working in step 1, as written. The responsible person gains the button when step 1's admission admits them. Answered by Archie, lead for the identity line.

It makes a new launch record with its own id every time. The kept record is copied, and the new record names the record it was copied from, so every report confirms exactly one command and a withdrawal ties to one command. No record is ever re-issued. An acceptance line gives a start again from a kept record and asserts two distinct record ids, the second naming the first. Answered by Archie.

Refused by name. While a start for an agent stands unconfirmed, a second start for that agent, on any machine, is refused as start_unconfirmed. The refusal names the standing request and the act that answers it: wait for its report, or withdraw it first. So one agent never gets two commands at once. The screen's warning says the same. An acceptance line asks twice and asserts the refusal and that one launch record exists. Answered by Archie.

A start reads DIRECTORY-011's agent record, the enduring identity, and nothing else of that brief. The check that the agent exists resolves the agent id named on the start to its registered agent record, and it refuses by name when no such record exists, because starting a session never creates an agent (DIRECTORY-CONTRACT.md 'Enduring agents and session credentials', rule 3). The session record the start writes points at exactly that enduring agent id (rule 2), and the agent record is not changed by the start (rule 1: it holds no session credential). The brief names that check and that field, with an acceptance line: a start for an agent id with no record is refused by name and writes nothing. Answered by Archie, lead for the identity line.

The profile version's own executable, with the arguments the profile version records, invoked in its recorded working directory. This card adds no lys launcher subcommand; the mock-up's 'identity start <slug>' line is illustrative only and the brief says so. The brief fixes the command as built from those recorded fields alone, states what it must name and must not carry as it already does, and an acceptance line compares the given command with the profile version's recorded executable, arguments and working directory. Answered by Archie.

Given. A second start for an agent that already reads as running is a second session of the same agent, which the contract allows (rule 5): it writes a new session record with its own session id pointing at the same enduring agent id, and it never creates or changes the agent record. Nothing about the first session changes. The session credential's form stays with RM-029, so this brief asserts the second session record and the unchanged agent record only, with an acceptance line for each. Answered by Archie.

Only the report makes it, as the words say. This card reads the session records and keeps no copy, so a second start of a running agent writes only a new launch record, and the session record is made in d5055cc1's store when that start's signed report arrives. My round 2 answer is corrected to that. The acceptance lines become: a second start writes a second launch record naming the same enduring agent id, the agent record is unchanged, and this card writes no session record. Answered by Archie, lead for the identity line.

Move the state derivation first. R11's state.rs, which gives unconfirmed while no verified lys/session-start/v1 report exists, with a withdrawn request no longer standing and a start of an agent already running being given, becomes the requirement before the start refusal. Renumber so the state comes before R10, and R10's start_unconfirmed refusal and its acceptance line stay with the start and cite the state requirement by its new number. No requirement relies on a state that a later one defines. Every reference follows the renumbering: the checklist, the coverage, the acceptance lines and the rendered files. Answered by Archie, lead for the identity line.

As environment assignments before the executable, so the recorded arguments stay exactly as the profile version records them and nothing is appended to them. The given command reads: env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> <executable> <recorded arguments>, with the working directory set to the recorded one. The three are ids only, never a credential value, so row 5.3's rule that no credential value is on the command line holds. Each value is shell-quoted and refused by name if it holds anything outside its id grammar. An acceptance line parses the given command and asserts the three assignments, the executable, the recorded arguments unchanged and in order, and that no credential value appears. The session report names the same launch record id it read from LYS_LAUNCH_RECORD. Answered by Archie, lead for the identity line.
- Cluster: directory; briefs: DIRECTORY-029
- Notes: Further units, named and not written: A grant to start an agent held by someone other than its responsible person or an administrator; The launch command's own proof (one-time code bound to a machine and a time window); The network record: machines, runtimes, reporting state, where each role may run, egress lists (CONFORMANCE 8.5); Add the launch record id to lys/session-start/v1 in the sessions brief d5055cc1; Record the executable, arguments and working directory on the reviewed profile version in the roles card Ink1H1Os; Runtimes that run the given command: a terminal view, sandbox, VM or container runners (CONFORMANCE 5.7). Ids, re-checked with git ls-remote and the refs it lists immediately before writing, each giving way to the next free id past every brief, draft and hand head that held it first: DIRECTORY-009 was held first by the lifecycle card and several drafts; DIRECTORY-027 was held first by draft c7c2ac1b and DIRECTORY-028 by draft 73184e72, so the brief is DIRECTORY-029; ADR-085 was held first by drafts fb954264 and 73184e72, so the new decisions are ADR-086 to ADR-088 and ADR-091, because ADR-089 was held first by draft fb954264 and ADR-090 by brief 8a60bd7a; RM-050 was held first by draft c7c2ac1b, RM-051 by drafts fb954264 and 73184e72, RM-052 by draft fb954264 and RM-053 by draft c7c2ac1b, so this row is RM-054; the cluster's checklist and stories gave way where draft c7c2ac1b held C209 to C215 and S84 to S86 first and draft fb954264 held S98 first, and take C229 to C241, S99 to S108 and S109, past C228 and S97, the highest any other draft held when these were first written. DIRECTORY-029, C229 to C241 and S99 to S108 were held by this draft before any other ref held them. The brief depends on the sessions brief by its run d5055cc1 (drafted as DIRECTORY-015), on the roles card Ink1H1Os, SECRETS-002, DIRECTORY-002, DIRECTORY-003, DIRECTORY-005 and DIRECTORY-011; row 8.5 has no card, so until one exists every start is refused by name at the egress check. Waffles reviews the written brief before the lead signs it off and the build is fired.

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

Carry IDENTITY-001's open rows (02, 04, 03, 05) into design-system briefs in this cluster, DIRECTORY-002 to DIRECTORY-005, revised for the grant ruling (ADR-003), the PostgreSQL ruling (ADR-005) and the working lifecycle states (ADR-011, proposed), with the fork (ADR-009) and the product accents (ADR-010) in the project ledger and every decision still open for Tom marked open. The IDENTITY-001 files stay as they are, as the record of revision 5. DIRECTORY-029 builds CONFORMANCE section 5, Starting an agent, as ADR-007 decides: the directory checks an agent, gives its start command and records it, and never runs it. A start request names the agent, the profile version and the machine (ADR-087), and its agent resolves to the enduring agent record DIRECTORY-011 keeps, which a start never creates or changes; the command given is the profile version's own executable with its recorded arguments in its recorded working directory, preceded by the ids-only assignments LYS_AGENT_ID, LYS_LAUNCH_RECORD and LYS_CREDENTIAL_IDS (ADR-091), and one start stands per agent at a time; five named checks each read their owner's record (the lifecycle state of DIRECTORY-003, the roles card Ink1H1Os's review record and role machines, SECRETS-002's handle record, and the network record of CONFORMANCE row 8.5) and refuse by name while that record does not exist. Every command given keeps one signed launch record (P4, ADR-086), whose id the agent's lys/session-start/v1 report names once the sessions brief (run d5055cc1, drafted as DIRECTORY-015) adds it to that signed message, so running is read from the sessions brief's record and never assumed (P5); a withdrawal records only that the request no longer stands (ADR-088). Giving a command is not a launch, so P6 stands. The launch record is the directory's own and is not the home's template render event (ADR-012).

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
- ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
- ADR-086 — Every command given keeps its own signed launch record, and running is tied to it through lys/session-start/v1 — Every command given keeps one launch record, a signed directory event naming the machine, the executable, the working directory, the profile version and the credential ids. Its id is carried in lys/session-start/v1, whose signed message then holds five things: the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge. The agent reads the id from LYS_LAUNCH_RECORD and its report names it, and running is tied to the command given through that id. A start given again copies a kept record into a new record with its own id that names the record it was copied from. Rejected: tying running to the agent id alone, re-issuing one record for every command given, and keeping the launch facts only in the command line.
- ADR-087 — The executable, its arguments and the working directory are fields of the reviewed profile version, never of a start request — The executable, the arguments and the working directory are read from the reviewed profile version's record, as the mock-up draws them, and a start request names only the agent, the profile version and the machine. Rejected: letting a start request set either, and taking either from the machine or a default.
- ADR-088 — Withdrawing a start request records only that the request no longer stands — A start request can be withdrawn by the person who gave it or anyone holding the same right. The withdrawal records that the request no longer stands, with who and when, and never that the agent did not start. If a signed report later arrives for the withdrawn launch record, the session shows as running with the withdrawal beside it. Rejected: recording a withdrawal as not started, and refusing a report that arrives after a withdrawal.
- ADR-091 — One standing start per agent, and the command given as ids-only environment assignments before the recorded executable — While a start for an agent stands unconfirmed, a second start for that agent, on any machine, is refused as start_unconfirmed, naming the standing request and the act that answers it: wait for its report, or withdraw it first. The given command reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> <executable> <recorded arguments>, with the working directory set to the recorded one. The three values are ids only, each shell-quoted and refused by name if it holds anything outside its id grammar. Rejected: appending the ids to the recorded arguments, and giving a second command while one stands unconfirmed.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.
- DIRECTORY-029 makes CONFORMANCE rows 5.1 to 5.6 true, each with at least one acceptance line that tests it, and no start path spawns a process.

## Non-Goals

- Rows 06 (connect Cambium) and 07 (gate, install and demonstrate the release) — They change the Cambium repository and depend on IDENTITY-002, the upstream release rebase; they need a Cambium cluster or an agreed cross-repository arrangement first.
- Row 03's changes inside the Rauthy fork — The fork is its own repository under vendor/rauthy; a row whose files live there needs its own brief in the fork, which does not yet exist and blocks DIRECTORY-004. DIRECTORY-004 names the fork files as work with their owner, not as files of this repository.
- The grant representation: how one grant records who may exercise it, whether it may be passed on (person, agent or nobody), what it derives from, and whether it can be bounded. OPEN for Tom. — The statement leaves the exact delegation schema unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003); AGENT-PARITY-2026-09-23's questions (docs/design/identity/AGENT-PARITY-2026-09-23.md:17-23) are inputs to it, not answers.
- Suspension semantics: whether suspending a person also ends their sign-in session at Rauthy or only makes our checks refuse, and what else stops with a suspended identity. OPEN for Tom. — The lifecycle document asks it of the room (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:99-100, docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:104-105) and its states are only proposed (ADR-011); two systems, one decision.
- The name of the identity service. OPEN for Tom. — The statement lists it as not decided (docs/design/identity/STATEMENT-2026-09-22.md:192); no crate or directory name selects it.
- Which anchor the first agents pin to: our hosted anchor, a self-hosted one, or both. OPEN for Tom. — Lys leaves it as a product decision and so does the statement (docs/design/identity/STATEMENT-2026-09-22.md:193).
- A nightly Rauthy base versus waiting for an upstream release carrying #1696 and #1728. OPEN for Tom. — Waffles' ruling of 15:36:25: if upstream has published no such release by the row 05 showing, Tom decides (docs/design/identity/briefs/IDENTITY-001.json:18; docs/design/identity/STATEMENT-2026-09-22.md:177); rows 02 to 05 run on v0.36.2 meanwhile.
- Road step 2 onward: capability certificates, arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production
Brought forward as step-2 work under the identity line lead's ruling: DIRECTORY-029's Start on the agent file, which checks, gives a command and keeps its launch record, and never launches, runs or stops a session. — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working.
- The examples in AGENT-PARITY-2026-09-23 (abilities with an assignment or project, seat provisioning within a budget, private and shared notes) — Tom gave them as not yet decided (docs/design/identity/AGENT-PARITY-2026-09-23.md:11-15); they are never turned into requirements.
- A production Cambium auth cutover, and any upstream Rauthy contribution as a prerequisite — Revision 5 forbids both before scratch acceptance, review and Gypsy's coordinated install (docs/design/identity/briefs/IDENTITY-001.json:31).
- A shared design-system package extracted for every product — Tom left it as a thing to look at, not a row (ADR-010).
- CONFORMANCE row 5.7: a terminal view, sandbox, VM or container runner — Proposed as separate products; lys does not run agents (ADR-007).
- How a start command proves itself, such as a one-time code for one machine for a few minutes — It stays proposed until the launch design chooses it (CONFORMANCE 5.3).
- A grant to start an agent held by someone other than its responsible person or a directory administrator — A further unit; it needs DIRECTORY-006's grant contract.
- The records the start checks read: the profile version and its review, the role's machines, the virtual credentials, the egress list, the sessions record and the provision record — Each belongs to its owner's card; DIRECTORY-029 reads them and keeps no copy.

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
| `docs/design/directory/briefs/DIRECTORY-029.json` | the start-an-agent brief: CONFORMANCE rows 5.1 to 5.6 | DIRECTORY-029 |
| `docs/design/directory/briefs/DIRECTORY-029.md` | rendered markdown | DIRECTORY-029 |
| `crates/lys-identity/src/start/mod.rs` | declarations and re-exports of the start module only | DIRECTORY-029 |
| `crates/lys-identity/src/start/request.rs` | the start request: agent, profile version and machine only, resolved to the enduring agent record DIRECTORY-011 keeps | DIRECTORY-029 |
| `crates/lys-identity/src/start/error.rs` | the named start refusals and their words, never a credential value | DIRECTORY-029 |
| `crates/lys-identity/tests/start_request.rs` | R1 test: define the start request, resolve the agent to its enduring record, and name every refusal | DIRECTORY-029 |
| `crates/lys-identity/src/start/authority.rs` | who may start, give again and withdraw: the responsible person and a directory administrator | DIRECTORY-029 |
| `crates/lys-identity/tests/start_authority.rs` | R2 test: admit a start, a start again and a withdrawal only from the agent's responsible person or a directory administrator | DIRECTORY-029 |
| `crates/lys-identity/src/start/checks.rs` | the five named checks, run in order, each reading its owner's record | DIRECTORY-029 |
| `crates/lys-identity/src/start/active.rs` | the agent is active, read from DIRECTORY-003's lifecycle state | DIRECTORY-029 |
| `crates/lys-identity/tests/start_checks.rs` | R3 test: run the five named checks before any command, with the agent is active read live from the lifecycle record | DIRECTORY-029 |
| `crates/lys-identity/src/start/profile_review.rs` | its profile version is reviewed, read from Ink1H1Os's review record | DIRECTORY-029 |
| `crates/lys-identity/tests/start_profile_review.rs` | R4 test: check that the profile version is reviewed, from the roles card's review record | DIRECTORY-029 |
| `crates/lys-identity/src/start/machine_role.rs` | the machine is allowed for the role, read from Ink1H1Os's role machines | DIRECTORY-029 |
| `crates/lys-identity/tests/start_machine_role.rs` | R5 test: check that the machine is allowed for the role, from the role's machines in the roles card's record | DIRECTORY-029 |
| `crates/lys-identity/src/start/credentials.rs` | its virtual credentials are valid, read from SECRETS-002's handle record; ids only | DIRECTORY-029 |
| `crates/lys-identity/tests/start_credentials.rs` | R6 test: check that the agent's virtual credentials are valid, from the door's handle record | DIRECTORY-029 |
| `crates/lys-identity-server/src/door_handles.rs` | the HandleRecords client of the door's handle-resolving endpoint (SECRETS-002 R1); ids and validity only, and an answer carrying a credential value is refused by name as credential_value_in_answer; wired into the start route by R12 | DIRECTORY-029 |
| `crates/lys-identity-server/tests/door_handles.rs` | R6 test: the HandleRecords door client against a local stub of the door's handle read shape; ids and validity for an answer without a value, and the credential_value_in_answer refusal for an answer with one | DIRECTORY-029 |
| `crates/lys-identity-server/examples/door_handles.rs` | R6 verification: reads the door's handle records through the same client for a given door address and agent, and prints credential ids and counts only, never a value | DIRECTORY-029 |
| `crates/lys-identity/src/start/egress.rs` | the machine may reach what the profile needs, read from row 8.5's egress list | DIRECTORY-029 |
| `crates/lys-identity/tests/start_egress.rs` | R7 test: check that the machine may reach what the profile needs, from its egress list | DIRECTORY-029 |
| `crates/lys-identity/src/start/profile_command.rs` | the executable, its arguments and the working directory read from the reviewed profile version through the ProfileVersionRecords seam over Ink1H1Os's record | DIRECTORY-029 |
| `crates/lys-identity/tests/start_profile_command.rs` | R9 test: take the executable, its arguments and the working directory from the reviewed profile version | DIRECTORY-029 |
| `crates/lys-identity/src/start/launch_record.rs` | the launch record and the withdrawal as signed directory events, appended and read by id | DIRECTORY-029 |
| `crates/lys-identity/src/start/state.rs` | running, unconfirmed and withdrawn, derived from the sessions record, and whether a start for an agent stands unconfirmed | DIRECTORY-029 |
| `crates/lys-identity/src/start/withdrawal.rs` | the withdrawal event: the request no longer stands, who and when | DIRECTORY-029 |
| `crates/lys-identity/tests/start_state.rs` | R10 test: running only on a signed report, unconfirmed without one, a withdrawal that never claims not started, and whether a start stands | DIRECTORY-029 |
| `crates/lys-identity/src/start/give.rs` | gives a start: runs the authority step and the checks, keeps a launch record, gives the command, gives a start again, and refuses a second start while one stands unconfirmed | DIRECTORY-029 |
| `crates/lys-identity/src/start/command.rs` | builds the command from a launch record and the profile version it names: the executable with its recorded arguments in its working directory, with no credential value | DIRECTORY-029 |
| `crates/lys-identity/tests/start_launch_record.rs` | R11 test: keep a launch record for every command given, render the command from it, give a start again, refuse a second start while one stands, and give a second start of a running agent without writing a session record | DIRECTORY-029 |
| `crates/lys-identity-server/src/start.rs` | the start route: give, give again, withdraw, state | DIRECTORY-029 |
| `crates/lys-identity-server/tests/start.rs` | R12 test: answer a start through the route and the CLI, and prove that no start path spawns a process | DIRECTORY-029 |
| `crates/lys/src/identity/start.rs` | the lys identity start CLI answer | DIRECTORY-029 |
| `crates/lys/tests/identity_start.rs` | R12 test: answer a start through the route and the CLI, and prove that no start path spawns a process | DIRECTORY-029 |
| `crates/lys-identity/tests/start_no_spawn.rs` | R12 test: answer a start through the route and the CLI, and prove that no start path spawns a process | DIRECTORY-029 |
| `surface/identity/src/features/start/StartDrawer.tsx` | the agent file's Start drawer | DIRECTORY-029 |
| `surface/identity/src/features/start/StartNotice.tsx` | the unconfirmed, withdrawn and running notice | DIRECTORY-029 |
| `surface/identity/tests/start.test.tsx` | R13 test: build the Start drawer and the unconfirmed notice on the agent file | DIRECTORY-029 |
| `surface/identity/tests/acceptance/start.spec.ts` | R13 test: build the Start drawer and the unconfirmed notice on the agent file | DIRECTORY-029 |

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
id: DIRECTORY-029
cluster: directory
title: Check an agent before giving its start command, keep a launch record for every command given, and show it running only on its signed report
---

# DIRECTORY-029: Check an agent before giving its start command, keep a launch record for every command given, and show it running only on its signed report

> **Cluster:** directory
> **Depends on:** DIRECTORY-002, DIRECTORY-003, DIRECTORY-005, d5055cc1, Ink1H1Os, SECRETS-002, DIRECTORY-011
> **Blocked by:** d5055cc1, the sessions brief, by its run whatever id it lands under (currently drafted as DIRECTORY-015, the brief this card's ruling amends; DIRECTORY-014 is another card): after the amendment its lys/session-start/v1 signed message holds five things, the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge, and the agent's report names the same launch record id it read from LYS_LAUNCH_RECORD. The launch record id is added before the tag freezes, while no agent signs under it; the lead sends this amendment to DIRECTORY-015's signed message. R10 and the running state of R13 are blocked on it by that name., Ink1H1Os, the ROLES card: the profile version record with its review record and the role's machines. R4 and R5 are blocked on it by that name., The amendment to the roles card Ink1H1Os that R9 carries and its dev record records: Ink1H1Os's profile version record holds the executable, the arguments and the working directory as fields of the reviewed profile version (ADR-087), in that card's record and never in a copy this card keeps. R9 is blocked on Ink1H1Os by that name until its landed profile version record declares all three fields. Ink1H1Os is a card on the Cambium board whose profile version record type has not landed in any repository, so R9 reads it only through the ProfileVersionRecords seam in crates/lys-identity/src/start/profile_command.rs; when that record lands, the file that implements the seam over it, and any Cargo manifest a dependency on its type needs, are named in R9's files by revising this brief before that part of R9 is dispatched., SECRETS-002: the virtual-credential (handle) record its R1 keeps, and the handle-resolving endpoint its R1 lands in the door repository (crates/cambium-door/src/http/secrets_handle.rs, door-owned; that brief sets no root naming the door repository yet). R6 is blocked on it by that name: R6's HandleRecords trait, its fixture tests and the door client tested against a local stub of that endpoint's read shape can be built first; the client is checked against the door's own endpoint, at the commit where SECRETS-002 R1 lands, by the verification command that names it. R12 wires the client into the start route., CONFORMANCE row 8.5, the network record of machines and egress lists: proposed, with no brief or card yet. R7 is blocked on it by that name, and until it exists every start is refused by name at the egress check., DIRECTORY-003: crates/lys-identity, the signed directory event, the reviewed IDENTITY-EVENTS.md envelope, the administrator's admission and the lifecycle state. R1 to R3 wait on it; R3's check, the agent is active, reads the lifecycle state as the lifecycle card folds it on top of DIRECTORY-003 and is the row that ships first., Paths this brief modifies that no landed manifest declares yet, because their owner briefs create them and have not landed, each reconciled against its owner's reviewed and landed manifest before dispatch, and the brief revised if a path differs: crates/lys-identity/src/lib.rs (R1) and docs/design/identity/IDENTITY-EVENTS.md (R10) against DIRECTORY-003's manifest; crates/lys-identity-server/src/routes.rs (R12), surface/identity/src/routes.tsx and surface/identity/src/generated/index.ts (R13) against DIRECTORY-005's crates/lys-identity-server/ and surface/identity/ manifests; crates/lys/src/identity/mod.rs and crates/lys/src/identity/cli.rs (R12) against DIRECTORY-002's crates/lys/src/identity/ manifest, whose R2 creates the identity group this card's one subcommand joins. If the reviewed crates/lys-identity-server/ manifest carries no HTTP client for R6's door client, crates/lys-identity-server/Cargo.toml is added to R6's files before dispatch. If DIRECTORY-002's landed crates/lys/Cargo.toml already carries the HTTP client R12's CLI answer reaches the route with, crates/lys/Cargo.toml, Cargo.toml and Cargo.lock are taken out of R12's files before dispatch., DIRECTORY-011, the provision brief: a start reads its agent record, the enduring identity, and nothing else of that brief. R1's check that the agent exists is blocked on it by that name, and the agent record is read, never copied and never changed by a start., Review of the launch record and withdrawal event kinds R10 adds to IDENTITY-EVENTS.md before the first is signed, as an adversarial review of a signed format., Waffles' review of this written brief, under the DIRECTORY-001 boundary that no row brief is dispatched until Waffles has reviewed it; then the identity line lead's sign-off, and only then is the build fired. Do not dispatch from this schema-valid brief while any blocker above stands (as CN12 says for DIRECTORY-006).
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-086 — Every command given keeps its own signed launch record, and running is tied to it through lys/session-start/v1 — Every command given keeps one launch record, a signed directory event naming the machine, the executable, the working directory, the profile version and the credential ids. Its id is carried in lys/session-start/v1, whose signed message then holds five things: the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge. The agent reads the id from LYS_LAUNCH_RECORD and its report names it, and running is tied to the command given through that id. A start given again copies a kept record into a new record with its own id that names the record it was copied from. Rejected: tying running to the agent id alone, re-issuing one record for every command given, and keeping the launch facts only in the command line.
> - ADR-087 — The executable, its arguments and the working directory are fields of the reviewed profile version, never of a start request — The executable, the arguments and the working directory are read from the reviewed profile version's record, as the mock-up draws them, and a start request names only the agent, the profile version and the machine. Rejected: letting a start request set either, and taking either from the machine or a default.
> - ADR-088 — Withdrawing a start request records only that the request no longer stands — A start request can be withdrawn by the person who gave it or anyone holding the same right. The withdrawal records that the request no longer stands, with who and when, and never that the agent did not start. If a signed report later arrives for the withdrawn launch record, the session shows as running with the withdrawal beside it. Rejected: recording a withdrawal as not started, and refusing a report that arrives after a withdrawal.
> - ADR-091 — One standing start per agent, and the command given as ids-only environment assignments before the recorded executable — While a start for an agent stands unconfirmed, a second start for that agent, on any machine, is refused as start_unconfirmed, naming the standing request and the act that answers it: wait for its report, or withdraw it first. The given command reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> <executable> <recorded arguments>, with the working directory set to the recorded one. The three values are ids only, each shell-quoted and refused by name if it holds anything outside its id grammar. Rejected: appending the ids to the recorded arguments, and giving a second command while one stands unconfirmed.
> **Checklist:**
> - C229 — No start path in the library, the route or the CLI spawns a process, and a test that reads the start files and gives a start with a marker-writing executable proves it (CONFORMANCE 5.1).
> - C230 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
> - C231 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.
> - C232 — No credential value is on the command line or the clipboard, and a test reads both (CONFORMANCE 5.3).
> - C233 — A launch record naming the machine, the executable, the working directory, the profile version and the credential ids is kept for every command given, reads back after a restart, and a start given again from it is a new record naming the one it was copied from (CONFORMANCE 5.4).
> - C234 — A start request names only the agent, the profile version and the machine; the command given reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> before the reviewed profile version's own executable and its recorded arguments, unchanged and in order, with the recorded working directory, each value shell-quoted and refused by name outside its id grammar; it is not a lys subcommand, and a request that sets the executable or the working directory is refused by name.
> - C235 — An agent shows as running only when its verified signed report names its launch record, and copying the command changes no state (CONFORMANCE 5.5).
> - C236 — With no report a start reads unconfirmed, never not started; the screen says the request stands and warns against asking elsewhere, and a second start for the agent is refused as start_unconfirmed (CONFORMANCE 5.6).
> - C237 — A withdrawal by the giver or anyone holding the same right records who and when and that the request no longer stands, never that the agent did not start, and a report arriving afterwards shows running with the withdrawal beside it.
> - C238 — Only the agent's responsible person and a directory administrator (in step 1, the admitted administrator) have a working Start; anyone else is refused by name, naming the agent and the right they lack.
> - C239 — The Start drawer and the unconfirmed notice are built on DIRECTORY-005's surface, and DIRECTORY-005's R1 and boundary name Start as the one control that works in step 1.
> - C240 — The start route gives, gives again, withdraws and reads a start through the library and holds no start logic of its own, and the CLI answer lys identity start-command, the one subcommand this card adds to DIRECTORY-002's identity group, prints exactly what the route answers for the same three inputs and starts nothing.
> - C241 — A start resolves its agent to the enduring agent record the provision brief DIRECTORY-011 keeps and is refused by name, writing nothing, when there is none; a start never creates or changes an agent record and writes no session record, and a start of an agent already running is given as a new launch record naming the same agent, whose session record the report makes in the sessions brief's store.
> **Stories:**
> - S109 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
> - S99 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.
> - S100 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent shown as running only when its own signed report names the command I was given, so that copying a command is never mistaken for a start.
> - S101 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want an unanswered start shown as unconfirmed with its request standing, so that I do not ask elsewhere and start the agent twice.
> - S102 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to withdraw a start request I no longer want, so that the record says the request no longer stands without claiming the agent did not start.
> - S103 (Operator, Installs and runs the standalone identity product) — As the operator, I want a kept launch record for every command given, so that I can give the same start again without looking inside a running process.
> - S104 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that reads the command line and the clipboard and finds no credential value, so that giving a command never discloses a secret.
> - S105 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that fails if any start path spawns a process, so that lys stays a product that checks, gives and records and never runs an agent.
> - S106 (Person without the right to start, Opens an agent file that is not theirs to start) — As a person who is neither the agent's responsible person nor a directory administrator, I want a refusal naming the agent and the right I lack, so that I know why there is no working Start for me.
> - S107 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want the command I am given to run the executable in the working directory that were reviewed, so that no start request can change what runs.
> - S108 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want starting my agent again while it runs to become a second session of the same agent, so that starting never makes another agent and never changes the one I registered.

## Purpose

Make CONFORMANCE section 5 rows 5.1 to 5.6 true: lys checks an agent before giving its start command, gives the command without ever running the agent or putting a credential value on the command line or the clipboard, keeps a launch record for every command given, and shows the agent as running only when its signed report names that launch record. With no report the start is unconfirmed, never not started, and a request can be withdrawn without anyone claiming the agent did not start. This is ADR-007 built: check, give, record, never run. Each check reads its owner's record, so the directory never holds a copy of what the roles card, the door, the network record, the sessions brief or the provision brief keep.

## Task

Build R1 to R13 in order. R1 and R2 fix what a start request is, that its agent exists as the enduring agent record DIRECTORY-011 keeps, and who may make one. R3 runs the five named checks and wires the one that needs only the lifecycle record, the agent is active; it ships first. R4 to R7 each add one check against its owner's record and each is blocked on that owner by name: Ink1H1Os (R4, R5), SECRETS-002 (R6) and network row 8.5 (R7). R6 builds the HandleRecords trait and the door client and tests them against a fixture and a local stub; R12 wires the client into the start route. While an owner's record does not exist the start is refused by name, naming the check and the card, and nothing is faked to let a start through. R8 amends DIRECTORY-005 so that Start is the one control that works in step 1. R9 takes the executable, its arguments and the working directory from the reviewed profile version, and carries the amendment that puts those three fields in Ink1H1Os's profile version record. R10 defines the launch record and the withdrawal as signed events, derives running, unconfirmed and withdrawn from the records it reads, and answers whether a start for an agent stands unconfirmed: a withdrawn record no longer stands and a running one does not. R11 keeps the launch record, renders the command from it as env LYS_AGENT_ID, LYS_LAUNCH_RECORD and LYS_CREDENTIAL_IDS assignments before the recorded executable and its recorded arguments, refuses a value outside its id grammar as command_value_outside_grammar, gives a start again as a new record naming its source, refuses a second start as start_unconfirmed while R10 answers that one stands, and gives a second start of an agent that already reads as running by keeping only a new launch record, leaving the session record to the report in the sessions brief's store. R12 lands the route, which gives, gives again, withdraws and reads a start, the CLI answer (lys identity start-command <agent> --profile-version <pv> --machine <m>, the one subcommand this card adds to the lys identity group DIRECTORY-002 R2 creates, which prints exactly what the route answers for the same three inputs and is never the command given) and the no-spawn proof; R13 lands the Start drawer and the unconfirmed notice. In: CONFORMANCE rows 5.1 to 5.6, each with at least one acceptance line testing it (5.1 in R12; 5.2 in R3 to R7; 5.3 in R11, R12 and R13; 5.4 in R11; 5.5 in R10 and R13; 5.6 in R10, R11 and R13). Out: row 5.7 (terminal, sandbox, VM or container runners, proposed as separate products); how the command proves itself, which stays proposed; every record the checks read; a grant to start held by anyone else; the environment a runtime gives a started agent (SECRETS-002 R5), which the command-line and clipboard test does not cover. The brief carries 13 requirements, one more than the method's split signal, because the card holds the six rows as one landable unit and each check keeps its own row naming its owner. It is code-bearing, so it carries its own wall in its boundaries; the cluster's documents-only CN1 governs DIRECTORY-001's work, not this brief. R10, the state derivation, comes before R11's start_unconfirmed refusal, which cites it by number, so no requirement relies on a state a later one defines. The sessions brief is named in depends_on by its run d5055cc1, whatever id it lands under; its current draft carries DIRECTORY-015, and it is that brief whose lys/session-start/v1 signed message the ruling amends to five things: the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge.

## Requirements

### R1: Define the start request as the agent, the profile version and the machine, resolve the agent to its enduring record, and name every refusal

Structure: a start request names exactly three members, the agent, the profile version and the machine, in a new module crates/lys-identity/src/start/ declared from crates/lys-identity/src/lib.rs, which DIRECTORY-003 creates and this requirement modifies only after it lands. Every refusal of a start is one typed error variant with a stable name and words that say what refused: start_field_not_allowed, start_member_missing, agent_unknown, start_right_missing, agent_not_active, profile_version_not_reviewed, machine_not_allowed_for_role, virtual_credentials_not_valid, egress_not_reachable, check_record_missing, profile_version_field_missing, command_value_outside_grammar, start_unconfirmed and credential_value_in_answer. Behaviour: IF a start request carries any member beside those three, the executable and the working directory included, THEN THE SYSTEM SHALL refuse it by name as start_field_not_allowed, naming the member, and SHALL NOT run a check, give a command or keep a launch record. IF a start request lacks any of those three members, THEN THE SYSTEM SHALL refuse it by name as start_member_missing, naming the missing member, and SHALL NOT run a check, give a command or keep a launch record. The executable and the working directory come only from the reviewed profile version (R9, ADR-087). WHEN a start request is asked, THE SYSTEM SHALL resolve the agent it names to that agent's registered agent record, the enduring identity the provision brief DIRECTORY-011 keeps, before anything else runs, and every record the start keeps SHALL point at exactly that enduring agent id. IF no agent record exists for the agent named, THEN THE SYSTEM SHALL refuse the start by name as agent_unknown, naming the agent, and SHALL NOT run a check, give a command, keep a launch record or write any record: starting never creates an agent. THE SYSTEM SHALL NOT change the agent record, which holds no session credential, and SHALL NOT keep a copy of it. This check is blocked on DIRECTORY-011 by that name. THE SYSTEM SHALL NOT put a credential value in any refusal, its words, its Debug form or a log line.

**Acceptance:**
- A request with agent agent-fixture-1, profile_version pv-fixture-1 and machine machine-fixture-1 parses into a start request holding exactly those three values.
- A request with the same three members plus executable /bin/sh is refused as start_field_not_allowed, its words contain 'executable', and the launch record count stays 0.
- A request with the same three members plus working_directory /tmp is refused as start_field_not_allowed, its words contain 'working_directory', and the launch record count stays 0.
- A request with agent agent-fixture-1 and profile_version pv-fixture-1 and no machine is refused as start_member_missing, its words contain 'machine', and the launch record count stays 0.
- With the provision record fixture holding only agent-fixture-1, a request naming agent-fixture-missing, profile_version pv-fixture-1 and machine machine-fixture-1 is refused as agent_unknown, its words contain 'agent-fixture-missing', the check-run count is 0, the launch record count is 0 and the agent record count is 1.
- The refusal test builds every one of the 14 named variants once and asserts the count is 14; for each, the Display and Debug output built with the fixture value fixture-credential-value-1f3a in scope does not contain 'fixture-credential-value-1f3a'.

**Files:**
- create: crates/lys-identity/src/start/mod.rs
- create: crates/lys-identity/src/start/request.rs
- create: crates/lys-identity/src/start/error.rs
- create: crates/lys-identity/tests/start_request.rs
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C234 — A start request names only the agent, the profile version and the machine; the command given reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> before the reviewed profile version's own executable and its recorded arguments, unchanged and in order, with the recorded working directory, each value shell-quoted and refused by name outside its id grammar; it is not a lys subcommand, and a request that sets the executable or the working directory is refused by name.
- C241 — A start resolves its agent to the enduring agent record the provision brief DIRECTORY-011 keeps and is refused by name, writing nothing, when there is none; a start never creates or changes an agent record and writes no session record, and a start of an agent already running is given as a new launch record naming the same agent, whose session record the report makes in the sessions brief's store.

**Stories:**
- S107 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want the command I am given to run the executable in the working directory that were reviewed, so that no start request can change what runs.
- S108 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want starting my agent again while it runs to become a second session of the same agent, so that starting never makes another agent and never changes the one I registered.

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
- C238 — Only the agent's responsible person and a directory administrator (in step 1, the admitted administrator) have a working Start; anyone else is refused by name, naming the agent and the right they lack.

**Stories:**
- S106 (Person without the right to start, Opens an agent file that is not theirs to start) — As a person who is neither the agent's responsible person nor a directory administrator, I want a refusal naming the agent and the right I lack, so that I know why there is no working Start for me.

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
- C230 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- C231 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.

**Stories:**
- S109 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S99 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

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
- C230 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- C231 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.

**Stories:**
- S109 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S99 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

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
- C230 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- C231 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.

**Stories:**
- S109 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S99 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

### R6: Check that the agent's virtual credentials are valid, from the door's handle record

THE SYSTEM SHALL check that the agent holds valid virtual credentials, reading the handle record SECRETS-002 keeps (its R1, ADR-001), and on a pass SHALL hand the credential ids to the launch record (R11). IF the agent holds no valid virtual credential, THEN THE SYSTEM SHALL answer virtual_credentials_not_valid naming the agent. WHILE SECRETS-002's record does not exist, THE SYSTEM SHALL answer check_record_missing naming its virtual credentials are valid and SECRETS-002. THE SYSTEM SHALL NOT read, hold, log or return a credential value: the check reads ids and validity only. The check reads the handle record through one seam, the trait HandleRecords declared in crates/lys-identity/src/start/credentials.rs, which answers for an agent the ids of its virtual credentials and whether each is valid, or that the record does not exist, or the refusal credential_value_in_answer, and has no method that returns a credential value. Its one production implementation is an HTTP client of the handle-resolving endpoint SECRETS-002 R1 lands in the door (crates/cambium-door/src/http/secrets_handle.rs in the door repository, door-owned), held in crates/lys-identity-server/src/door_handles.rs. IF an answer of the door carries a credential value, THEN the client SHALL refuse it by name as credential_value_in_answer, naming the record and the field that carried a value and never the value itself, SHALL return no ids from that answer, and the check built over the client SHALL answer that refusal, so no command is given and no launch record is kept; the client SHALL NOT silently drop the value and return the ids, and nothing it prints or logs SHALL hold the value. This requirement builds and tests only the trait and that client; the start route that calls the client through the trait is created and wired in R12. The library's tests fill HandleRecords with a fixture. crates/lys-identity-server/tests/door_handles.rs tests the client under cargo test in this workspace against a local stub that answers the read shape of the door's handle-resolving endpoint, and checks the client's records for an answer without a value and its credential_value_in_answer refusal for an answer with one; nothing in this requirement needs the door running. Because the door does not run inside this workspace's cargo test, the client is checked against the door's own endpoint by a verification command a person runs, with the door started by name from its own tree at the commit where SECRETS-002 R1 lands, through crates/lys-identity-server/examples/door_handles.rs, which depends on nothing but this crate, reads the door's handle records through the same client for the door address and the agent it is given, and prints credential ids and counts only, never a value. This requirement is blocked on SECRETS-002 by that name.

**Acceptance:**
- Reading crates/lys-identity/src/start/credentials.rs finds the trait HandleRecords, and the return type of each of its methods holds only credential ids, their validity and the record-missing answer: no method returns a type with a field for a credential value.
- With the handle record fixture holding vc-fixture-1 active for agent-fixture-1, its virtual credentials are valid answers passed and hands on exactly ['vc-fixture-1'].
- With vc-fixture-1 revoked in the fixture, the check answers virtual_credentials_not_valid and its words contain 'agent-fixture-1'.
- With no handle record present, the check answers check_record_missing and its words contain 'its virtual credentials are valid' and 'SECRETS-002'.
- The handle record fixture carries the value fixture-credential-value-1f3a beside vc-fixture-1, and no output of the check, Debug form included, contains 'fixture-credential-value-1f3a'.
- crates/lys-identity-server/tests/door_handles.rs starts a local stub answering the door's handle read shape with vc-fixture-1 active for agent-fixture-1: the client answers for agent-fixture-1 exactly 1 credential id, vc-fixture-1, and that it is valid.
- With the stub answering vc-fixture-1 revoked for agent-fixture-1, the client answers vc-fixture-1 not valid, and the check built over the client answers virtual_credentials_not_valid.
- With the stub answering that no handle record exists for agent-fixture-1, the client answers that the record does not exist, and the check built over the client answers check_record_missing with words containing 'SECRETS-002'.
- With the stub's answer carrying the value fixture-credential-value-1f3a in the value field of vc-fixture-1's record for agent-fixture-1, the client answers credential_value_in_answer, its words contain 'vc-fixture-1' and 'value', it returns no credential id, neither its words nor its Debug form contains 'fixture-credential-value-1f3a', the check built over the client answers credential_value_in_answer, and the test asserts that its 4 stub cases each ran once.

**Files:**
- create: crates/lys-identity/src/start/credentials.rs
- create: crates/lys-identity/tests/start_credentials.rs
- create: crates/lys-identity-server/src/door_handles.rs
- create: crates/lys-identity-server/tests/door_handles.rs
- create: crates/lys-identity-server/examples/door_handles.rs

**Checklist:**
- C230 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- C231 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.

**Stories:**
- S109 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S99 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

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
- C230 — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- C231 — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.

**Stories:**
- S109 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S99 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.

### R8: Amend DIRECTORY-005 so that Start is the one control that works in step 1

Structure: append one sentence to the spec of DIRECTORY-005 R1 and the same sentence to its boundary 'No grant, launch, secret or memory control is presented as working in step 1.': 'Under DIRECTORY-029, Start on the agent file is the one control that works in step 1, for the admitted administrator; every other grant, launch-profile, secret and memory control stays not working in step 1.' In the same edit, amend DIRECTORY-005 R1's third acceptance line so it agrees: its clause 'no control for launch, permissions, secrets or memory is presented as working' becomes 'no control for permissions, secrets or memory is presented as working', and the line gains the sentence 'Under DIRECTORY-029, the Start control is the one that brief gives, refused by name until the start is confirmed; permissions, secrets and memory are still not presented as working.' Re-render DIRECTORY-005.md from the JSON. THE SYSTEM SHALL NOT change any other text, identifier, dependency or acceptance line of DIRECTORY-005.

**Acceptance:**
- The diff of docs/design/directory/briefs/DIRECTORY-005.json changes exactly three strings: requirements/0/spec and the boundary quoted above, each ending with the appended sentence, and requirements/0/acceptance/2.
- After the edit, DIRECTORY-005's requirements/0/acceptance/2 contains 'no control for permissions, secrets or memory is presented as working' and 'Under DIRECTORY-029, the Start control is the one that brief gives', and does not contain 'launch'.
- python3 scripts/design/render-cluster.py over a copy of docs/design/directory writes a DIRECTORY-005.md byte-identical to the committed one.

**Files:**
- modify: docs/design/directory/briefs/DIRECTORY-005.json
- modify: docs/design/directory/briefs/DIRECTORY-005.md

**Checklist:**
- C239 — The Start drawer and the unconfirmed notice are built on DIRECTORY-005's surface, and DIRECTORY-005's R1 and boundary name Start as the one control that works in step 1.

### R9: Take the executable, its arguments and the working directory from the reviewed profile version

The executable and the working directory a command runs are fields of the reviewed profile version (ADR-087), and the arguments it is given are the ones the profile version records. This requirement adds the executable, the arguments and the working directory to the profile version under the roles card Ink1H1Os's record, named in depends_on, as a named amendment to that card: the fields are added in Ink1H1Os's record and never in a copy this card keeps, the amendment is named in blocked_by, the dev record of this requirement carries it, and this requirement stays blocked on Ink1H1Os by that name until its landed profile version record declares the three fields. WHEN a command is to be given, THE SYSTEM SHALL read the executable, the recorded arguments and the working directory from the requested profile version's record, through one seam, the trait ProfileVersionRecords declared in crates/lys-identity/src/start/profile_command.rs, which answers for a profile version id the executable, the arguments and the working directory exactly as Ink1H1Os's record holds them, each or that it is missing. Ink1H1Os is a card on the Cambium board and its profile version record type has not landed in any repository, so that trait is the one place this card reads the record: this requirement takes no Cargo dependency on a roles-card type. The implementation of the trait over Ink1H1Os's landed profile version record, and any manifest a dependency on its type needs, are named in this requirement's files by revising the brief when that record lands (blocked_by); until then only the trait and its fixture exist. IF that record does not hold the executable or the working directory, THEN THE SYSTEM SHALL refuse as profile_version_field_missing naming the field and Ink1H1Os, and SHALL NOT take the executable, an argument or the working directory from the request, the machine, a default or the mock-up. THE SYSTEM SHALL NOT define a profile version record of its own.

**Acceptance:**
- Reading crates/lys-identity/src/start/profile_command.rs, the seam through which this card reads Ink1H1Os's profile version record, finds the trait ProfileVersionRecords declaring exactly 3 reads for a profile version id, named for the executable, the arguments and the working directory, and 0 other methods.
- rg -n 'struct ProfileVersion' crates/lys-identity/src/start finds 0 lines.
- With the profile version fixture pv-fixture-1 holding executable fixture-exec, arguments ['--fixture-arg', 'two words'] and working directory fixture-cwd, the start reads exactly 'fixture-exec', ['--fixture-arg', 'two words'] and 'fixture-cwd'.
- With pv-fixture-1 holding no working directory, the start is refused as profile_version_field_missing, its words contain 'working directory' and 'Ink1H1Os', and the launch record count is 0.

**Files:**
- create: crates/lys-identity/src/start/profile_command.rs
- create: crates/lys-identity/tests/start_profile_command.rs

**Checklist:**
- C234 — A start request names only the agent, the profile version and the machine; the command given reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> before the reviewed profile version's own executable and its recorded arguments, unchanged and in order, with the recorded working directory, each value shell-quoted and refused by name outside its id grammar; it is not a lys subcommand, and a request that sets the executable or the working directory is refused by name.

**Stories:**
- S103 (Operator, Installs and runs the standalone identity product) — As the operator, I want a kept launch record for every command given, so that I can give the same start again without looking inside a running process.
- S107 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want the command I am given to run the executable in the working directory that were reviewed, so that no start request can change what runs.

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
- C235 — An agent shows as running only when its verified signed report names its launch record, and copying the command changes no state (CONFORMANCE 5.5).
- C236 — With no report a start reads unconfirmed, never not started; the screen says the request stands and warns against asking elsewhere, and a second start for the agent is refused as start_unconfirmed (CONFORMANCE 5.6).
- C237 — A withdrawal by the giver or anyone holding the same right records who and when and that the request no longer stands, never that the agent did not start, and a report arriving afterwards shows running with the withdrawal beside it.

**Stories:**
- S100 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent shown as running only when its own signed report names the command I was given, so that copying a command is never mistaken for a start.
- S101 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want an unanswered start shown as unconfirmed with its request standing, so that I do not ask elsewhere and start the agent twice.
- S102 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to withdraw a start request I no longer want, so that the record says the request no longer stands without claiming the agent did not start.

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
- C232 — No credential value is on the command line or the clipboard, and a test reads both (CONFORMANCE 5.3).
- C233 — A launch record naming the machine, the executable, the working directory, the profile version and the credential ids is kept for every command given, reads back after a restart, and a start given again from it is a new record naming the one it was copied from (CONFORMANCE 5.4).
- C234 — A start request names only the agent, the profile version and the machine; the command given reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> before the reviewed profile version's own executable and its recorded arguments, unchanged and in order, with the recorded working directory, each value shell-quoted and refused by name outside its id grammar; it is not a lys subcommand, and a request that sets the executable or the working directory is refused by name.
- C236 — With no report a start reads unconfirmed, never not started; the screen says the request stands and warns against asking elsewhere, and a second start for the agent is refused as start_unconfirmed (CONFORMANCE 5.6).
- C241 — A start resolves its agent to the enduring agent record the provision brief DIRECTORY-011 keeps and is refused by name, writing nothing, when there is none; a start never creates or changes an agent record and writes no session record, and a start of an agent already running is given as a new launch record naming the same agent, whose session record the report makes in the sessions brief's store.

**Stories:**
- S109 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent's start command given only after its checks pass, so that I never start an agent that is not active, reviewed, allowed on the machine, credentialed and able to reach what it needs.
- S101 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want an unanswered start shown as unconfirmed with its request standing, so that I do not ask elsewhere and start the agent twice.
- S103 (Operator, Installs and runs the standalone identity product) — As the operator, I want a kept launch record for every command given, so that I can give the same start again without looking inside a running process.
- S104 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that reads the command line and the clipboard and finds no credential value, so that giving a command never discloses a secret.
- S108 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want starting my agent again while it runs to become a second session of the same agent, so that starting never makes another agent and never changes the one I registered.

### R12: Answer a start through the route and the CLI, and prove that no start path spawns a process

The start route in crates/lys-identity-server/src/start.rs, mounted from crates/lys-identity-server/src/routes.rs, which DIRECTORY-005 creates and this requirement modifies only after it lands, gives a start (POST /api/agents/<agent>/start with the profile version and the machine), gives it again from a kept record (POST /api/launch-records/<id>/start-again), withdraws it (POST /api/launch-records/<id>/withdraw) and reads its state (GET /api/launch-records/<id>/state), each through the library of R1 to R11, and holds no start logic of its own: every check, refusal, launch record and command it answers is the library's. This requirement wires R6's door client of crates/lys-identity-server/src/door_handles.rs into the start route from routes.rs, and the start route calls that client only through the HandleRecords trait. The CLI answer is lys identity start-command <agent> --profile-version <pv> --machine <m>, in crates/lys/src/identity/start.rs. It joins the lys identity group DIRECTORY-002 R2 creates, with prepare, configure and health, in crates/lys/src/identity/mod.rs and crates/lys/src/identity/cli.rs, which this requirement modifies only after DIRECTORY-002 lands: this card adds the one subcommand start-command to that group and creates no group of its own. WHEN it is run, it SHALL ask the route for a start with the same three inputs and print exactly what the route answers, and nothing else, so a person can copy the command into the machine's shell. It is not a launcher: it starts nothing and holds no process. It reaches the route over HTTP, so crates/lys/Cargo.toml, Cargo.toml and Cargo.lock gain an HTTP client dependency that meets the workspace's pure-Rust rule, unless DIRECTORY-002's landed identity group already carries one (blocked_by). WHEN a start is given, the route SHALL answer with the command, its working directory and its launch record; WHEN a start is refused, the route SHALL return every refusal by its name in words, and the CLI prints that answer. THE SYSTEM SHALL NOT spawn a process on any start path: no start file in the library, the route or the CLI names a process-spawning API, and a test proves it both by reading those files and by giving a start whose executable would leave a marker if it ran. THE SYSTEM SHALL NOT put a credential value in the route's answer, the CLI's output or its errors.

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
- Reading crates/lys-identity-server/src/routes.rs finds the start route given the door client of crates/lys-identity-server/src/door_handles.rs as its HandleRecords, and crates/lys-identity-server/src/start.rs reaches the credentials record only through the HandleRecords trait, naming no type declared in door_handles.rs.
- Reading crates/lys/src/identity/cli.rs finds exactly 1 subcommand declared by this brief, start-command, beside DIRECTORY-002's prepare, configure and health, and no subcommand that runs, spawns or supervises a process.

**Files:**
- create: crates/lys-identity-server/src/start.rs
- create: crates/lys-identity-server/tests/start.rs
- create: crates/lys/src/identity/start.rs
- create: crates/lys/tests/identity_start.rs
- create: crates/lys-identity/tests/start_no_spawn.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys/src/identity/mod.rs
- modify: crates/lys/src/identity/cli.rs
- modify: crates/lys/Cargo.toml
- modify: Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C229 — No start path in the library, the route or the CLI spawns a process, and a test that reads the start files and gives a start with a marker-writing executable proves it (CONFORMANCE 5.1).
- C232 — No credential value is on the command line or the clipboard, and a test reads both (CONFORMANCE 5.3).
- C240 — The start route gives, gives again, withdraws and reads a start through the library and holds no start logic of its own, and the CLI answer lys identity start-command, the one subcommand this card adds to DIRECTORY-002's identity group, prints exactly what the route answers for the same three inputs and starts nothing.

**Stories:**
- S104 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that reads the command line and the clipboard and finds no credential value, so that giving a command never discloses a secret.
- S105 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that fails if any start path spawns a process, so that lys stays a product that checks, gives and records and never runs an agent.

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
- C232 — No credential value is on the command line or the clipboard, and a test reads both (CONFORMANCE 5.3).
- C235 — An agent shows as running only when its verified signed report names its launch record, and copying the command changes no state (CONFORMANCE 5.5).
- C236 — With no report a start reads unconfirmed, never not started; the screen says the request stands and warns against asking elsewhere, and a second start for the agent is refused as start_unconfirmed (CONFORMANCE 5.6).
- C237 — A withdrawal by the giver or anyone holding the same right records who and when and that the request no longer stands, never that the agent did not start, and a report arriving afterwards shows running with the withdrawal beside it.
- C238 — Only the agent's responsible person and a directory administrator (in step 1, the admitted administrator) have a working Start; anyone else is refused by name, naming the agent and the right they lack.
- C239 — The Start drawer and the unconfirmed notice are built on DIRECTORY-005's surface, and DIRECTORY-005's R1 and boundary name Start as the one control that works in step 1.

**Stories:**
- S99 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want each failed check named in words, so that I know what to fix before a command can be given.
- S100 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want my agent shown as running only when its own signed report names the command I was given, so that copying a command is never mistaken for a start.
- S101 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want an unanswered start shown as unconfirmed with its request standing, so that I do not ask elsewhere and start the agent twice.
- S102 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want to withdraw a start request I no longer want, so that the record says the request no longer stands without claiming the agent did not start.
- S104 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a test that reads the command line and the clipboard and finds no credential value, so that giving a command never discloses a secret.
- S106 (Person without the right to start, Opens an agent file that is not theirs to start) — As a person who is neither the agent's responsible person nor a directory administrator, I want a refusal naming the agent and the right I lack, so that I know why there is no working Start for me.

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
- With the door started by name from its own repository at the commit where SECRETS-002 R1 lands, holding vc-fixture-1 active for agent-fixture-1, from this repository's root: cargo run -p lys-identity-server --example door_handles -- <that door's address> agent-fixture-1 prints the count 1 and the credential id vc-fixture-1 as valid, and prints no credential value. This is a verification step and not an acceptance line, because the door does not run inside this workspace's cargo test.
- From the repository root: rg -n -i 'not started' crates/lys-identity/src/start surface/identity/src finds 0 lines.
- Each of CONFORMANCE rows 5.1 to 5.6 is named in at least one test name or test comment under crates/lys-identity/tests, crates/lys-identity-server/tests, crates/lys/tests and surface/identity/tests, and the report lists which test covers which row.
- R10's dev record carries the session-start ruling: the launch record id is added to lys/session-start/v1 by the sessions brief d5055cc1 before its tag freezes, so that its signed message holds the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge; the amended brief is DIRECTORY-015, named in depends_on by its run d5055cc1.
- Frontend checks pass with strict types and the generated API schema, the clipboard test included.
- R9's dev record carries the amendment to the roles card Ink1H1Os: the executable, the arguments and the working directory are fields of its profile version record.
- From the repository root: rg -n 'identity start <slug>' crates surface finds 0 lines, and no subcommand the CLI adds is a launcher: lys identity start-command prints a command and never runs one.

