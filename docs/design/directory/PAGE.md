# directory — what was asked, what it means, and what was written

## The words, as they were typed

Write the brief for this card in the directory cluster (docs/design/directory/briefs, the next id not used on main or on any origin branch, which at 13:00 on 27 September was DIRECTORY-015 with three more briefs being written beside this one, so take the id from the branches at write time), as road step 2 of the directory design (docs/design/directory/DESIGN.md): roles and their versions, to ADR-006. This brief carries conformance rows 4.2, 4.3, 4.4 and the roles half of 4.5 of docs/design/identity/CONFORMANCE.md, and each of those rows gets at least one acceptance line that tests it and names the row. Editing a role makes a new version, and no holder changes version by itself. Moving a holder to a newer version is a deliberate act that shows what changes before it is taken and is recorded with who did it. The policy of a move at the next renewal is explicit, and each holder shows the date it will move. A provisional holding has its end date on its grant, it lapses when the date passes and is never renewed quietly, and a version change never extends it. The directory half of row 4.5, that the grants a lapsed holding carried stop at the next check, is DIRECTORY-006 R4's and is named here as the neighbour, not repeated. Done when a role edited while it has holders leaves every holder on the version it held with its grants unchanged, when one holder moved by hand shows the change and leaves an audit record naming the actor, when a holder under the renewal policy shows its move date and moves on that date and not before, and when a provisional holding's end date is the same after an edit and after a move. Two open rows of the conformance list are decided here, and the brief writes both decisions into the directory design's Decisions section with these reasons, so that Archie reads them as second reader. Row 4.7 is decided as a template copied at grant time, not a live group. A holding names the role and the version it holds, the grants it carries are made from that version's templates when the holding is granted, and an edit to the role changes none of them, since a live group would change every holder's permissions at the moment of the edit, which ADR-006 forbids. Who holds a role remains answerable as a query over holdings. Row 4.6 is decided as a move at the next renewal being the default for a holding with an end date, shown on the holder as the date it will move, since a renewal is already a deliberate recorded act by the person the holder answers to, so the move rides on an act that exists and no holder ever moves without one. A holding with no end date has no renewal, so its default is no move without a deliberate act, and the screen says so on that holder. The person the holder answers to may change the policy of one holding, and the change is recorded with who did it. Hold to R4 of DIRECTORY-002 and to ADR-009, so that every check about a role version asks SpiceDB and the browser shows what is permitted and is never the authority. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after Tom or the lead signs it off on the card. Rulings of the lead, Apollo, given on 27 September 2026 to run 35d44b68-873f-4a10-8f4c-01cc2ecf1ee7 in answer to its rounds, settled here and not reopened by the author. It carries CONFORMANCE's build step 6, roles after the ADR, and my phrase road step 2 is withdrawn. It depends on DIRECTORY-006 for grants and amends no non-goal. If a sentence of DESIGN.md reads as excluding roles and versions, quote it as a question for the lead in the brief rather than rewriting it. ADR-009 was the wrong id and is withdrawn. The decision meant is the one the SpiceDB enforcement brief carries, ADR-043 on draft/directory/d40aa430, cited by id with a note that it lands with that brief. This brief does not bring enforcement in itself. The requirement that makes a role-version check ask SpiceDB is blocked on an artifact, the evaluator file that the enforcement brief creates, taken by path from that brief's file list on its origin branch and checked with git cat-file -e against origin/main. Until then the checks read the holding record through one seam so the swap is in one place. The date shown is the holding's end date, and the words beside it say that the next renewal moves the holding to the current version. Moves on that date means that a renewal made at or before the end date is made at the current version, so the holder is on the new version from the renewal. If nobody renews, the holding lapses on its end date under ADR-006 and DIRECTORY-006 R4 and there is no move. The acceptance has both lines, one where the renewal happens and the holder is on the new version, and one where it does not and the holding has lapsed. This brief defines renewal, since nothing on main does. A renewal is a new grant of the holding by the person the holder answers to, the responsible person of ADR-011, made at the role's current version with a new end date, recorded as a renewal that names the actor, the holding it renews and the version it lands on. It is one requirement with its own acceptance line, and my sentence saying the act already exists is corrected to say this brief introduces it. Both. A role carries a default policy, the mock-up's When this role changes, and a holding takes that default at grant time and may be changed on its own afterwards. The holding's own policy governs it. Changing the role's default applies to holdings granted after the change and moves nothing that exists, so no holder changes as a side effect, and the role screen shows the default and marks the holdings whose policy differs from it. The actor chooses, as the mock-up's move drawer offers, between at its next start, which is the default, and now, which ends the running session first. A running session keeps the version it started with. The audit record names which of the two was chosen. The person the holder answers to, and the owners of the project the role is assigned in, as the mock-up says. A directory administrator moves a holder only in one of those two capacities. The audit record names the actor and the capacity. Once the enforcement brief lands the check asks SpiceDB through the seam of my second answer. Both new entries are recorded as proposed, decided_by empty, with Apollo as author and Archie as second reader named in the context, and they become decided only when Tom rules or signs the card off. They do not supersede ADR-006; they answer the sentence it left open and cite it. The Confirm button that makes a provisional holding permanent is outside this brief and is named as outside in its boundaries, since it is a new grant without an end date and belongs with the grant rows. Only the versioned grant templates make a version. Row 4.1's job and profile stay proposed and outside this brief. The role screen offers the title and the grant templates, an edit to the templates makes a new version, and an edit to the title is recorded and makes no version. The owners of the project the role is assigned in edit its grant templates and its title and change its default policy, and a directory administrator does so only in that capacity. It is the same set that may move a holder, so one admission serves R4, R9 and the move, and every such act is recorded with the actor and the capacity. Under the policy of a move only by a deliberate act, a renewal is made at the version the holding holds, and it is still a new grant with a new end date recorded as a renewal. Under the policy of a move at the next renewal, it is made at the role's current version, as answered. R8 defines renewal for both policies in those two sentences, and the move to a newer version under the first policy still needs its own deliberate act. A move taken now asks for the session's end through DIRECTORY-015 R6's operator stop, with the mover as the actor and the move named as the cause in the audit record. The mover is admitted to that stop as the responsible person or a project owner only if DIRECTORY-015 admits them as an operator of that agent's sessions. If it does not admit them, the now leg is refused by name and the actor may take the move at its next start instead. R7's now leg is blocked on DIRECTORY-015's artifact, the file that defines the operator stop, taken by path from that brief's file list on its origin branch and checked with git cat-file -e against origin/main. Yes. An owner of the project a role is assigned in may grant a holding of it, as the mock-up says, and every grant copied from the version's templates is still admitted under DIRECTORY-006 for that granting actor, so the owner must hold at or above what each template gives. A template the owner cannot give makes the assign refuse with the reasons row 2.4 names, and R3 admits by both, the owner's capacity and DIRECTORY-006's admission of each copied grant. The non-goal sentence stands unamended and does not exclude this brief, exactly as ruled for the sibling brief 687de276. It describes what step 1 of the directory design set out to build, DIRECTORY-006 already sits beside it on main making grants, and its amendment is a documents card for the design's owner. The brief records that in one sentence of its boundaries and changes nothing in DESIGN.md. CN1 binds only the planning task of the directory design round, and each brief's build walls are the brief's own, as DIRECTORY-006 on main already shows. No amendment is needed before this brief's code paths or its addition to docs/design/identity/IDENTITY-EVENTS.md are built, and the brief says so in one sentence beside its file walls. Rulings of the lead, Apollo, given on 27 September 2026 to run 6af1f2da-2030-4d5d-b0d6-f7b685d99179 in answer to its round 1, settled here and not reopened by the author. A newly made role's default starts as move at the next renewal, as row 4.6 is decided. The mock-up's initial value of stay is a drawing default and does not govern, and the brief records the difference in one sentence of its boundaries so the mock-up's owner can bring the drawing into line. A holding with an end date takes that default at grant time and shows its move date. A holding with no end date has no renewal, so whatever the role's default it reads as no move without a deliberate act, and the screen says so on that holder. The now leg is taken only when the mover is the admitted operator of DIRECTORY-003 R3, since that is the only party DIRECTORY-015 R6 admits to a stop, and this brief does not widen that admission. When the mover is that operator and is also the responsible person or a project owner, the now leg is taken, the stop is recorded under DIRECTORY-015 with the operator as its actor, and the move's own audit record names the mover's capacity for the move, responsible person or project owner, and names the stop it caused. A mover who is not the admitted operator has the now leg refused by name and may take the move at its next start. My earlier sentence about an operator of that agent's sessions is corrected by this answer, since DIRECTORY-015 has no per-agent operator. Yes, it shows who can stop it, as the mock-up says. The holder row under the renewal policy shows the holding's end date, the words that the next renewal moves it to the current version, and who can stop the move, which is the person the holder answers to and the owners of the project the role is assigned in, the same set that may change the holding's policy. The acceptance line for row 4.4 asserts all three on the row. Yes. The person the holder answers to may grant a holding of a role to their own agent, and so may an owner of the project the role is assigned in, and each is admitted only if DIRECTORY-006 admits every grant copied from the version's templates for that granting actor. The earlier acceptance with the responsible person H1 granting the holding stands, and one more acceptance line has a project owner granting it. The Assign action is shown to both and to nobody else, and a directory administrator assigns only in one of those two capacities.

Rulings. The lead settled these in brief run 7494cb8b-adbe-4346-a333-1f003bc1f82a on 27 September 2026. Each is decided, so the brief takes it as given and does not ask it again.

The lead was asked this.
What makes someone an owner of a project: which relation or grant under DIRECTORY-006's model?
Nothing on main defines project ownership.
The lead ruled as follows.
Ownership of a project is an explicit relation, owner, on the project object in the directory's authorization model.
It is written as a grant like any other and rooted in a person, and only a directory administrator or an existing owner of that project may give it.
It is never inferred from a name, a label or a rank, as DIRECTORY-006 R1 and R2 require.
This brief adds the owner relation to the model's project definition and names it as the one ownership test for Assign, Move, the policy change and the role editor.
Answered by Archie, lead for the identity line.

The lead was asked this.
Is a role defined in exactly one project, or can holdings of one role sit in several projects?
If several, whose owners may edit the role's templates and change its default?
The lead ruled as follows.
A role is defined in exactly one project, and every holding of it sits in that project.
Only that project's owners edit its templates and title and change its default policy, and a directory administrator does so only in that capacity.
A holding of the same job in another project needs a role defined there.
So no owner of one project can make a version that holders in another project move to.
Answered by Archie.

The lead was asked this.
May an owner of the role's project renew a holding, or only the person the holder answers to?
The lead ruled as follows.
Both may renew: the person the holder answers to, and an owner of the role's project.
The renewal record names which one acted, the holding it renews and the version it lands on.
Anyone else is refused as ROLE_RENEW_REFUSED.
A provisional holding still lapses on its end date unless one of them renews it, and it is never renewed quietly.
Answered by Archie.

The lead was asked this.
Which id does the brief cite for the operator stop: the session brief's current DIRECTORY-019 R6 on 8c1bee6c, or DIRECTORY-015 as ruled?
The lead ruled as follows.
Cite the session brief by its current id, DIRECTORY-019 R6 on draft/directory/8c1bee6c, and key the blocker on the artifact crates/lys-identity/src/session/end.rs being on lys main.
That check doesn't move if the id changes again.
Drop DIRECTORY-015 from the sentence.
Answered by Archie.

The lead was asked this.
When a move is taken now and the agent has more than one open session, does it stop every open session or only one the mover names?
The lead ruled as follows.
Every open session of that agent.
A move taken now asks for each open session's end through DIRECTORY-019 R6's stop, one session id at a time.
The mover sees each session listed as stopped, or as unconfirmed until it reports.
The audit record names each session the move caused to end.
Answered by Archie.

The lead was asked this.
Does this brief cover people holding roles, and if so, who is 'the person the holder answers to' for a person holder?
The lead ruled as follows.
No.
This brief covers agent holders only.
People holding roles is named under further units not written, and that unit settles who a person holder answers to.
It never lets a holder renew, move or change the policy of their own holding.
Answered by Archie.

The lead was asked this.
The words block only R7's now leg on the session brief.
Should R7's next-start leg also be blocked on crates/lys-identity/src/session/start.rs (DIRECTORY-019 R4) being on origin/main, as the author added?
The lead ruled as follows.
Yes.
The next-start leg takes effect when a session starts, which is code in crates/lys-identity/src/session/start.rs that DIRECTORY-019 R4 creates, so the block is a fact of the code and stays.
Name that dependency in R7.

The lead was asked this.
A holding with no end date is stored as deliberate_only, so when the role's default is move_at_next_renewal the role screen marks every such holding as differing from the default.
Should a no-end-date holding be marked as differing, or shown unmarked with only 'Moves only by a deliberate act'?
The lead ruled as follows.
Show it unmarked, with only the words Moves only by a deliberate act.
A holding with no end date has no renewal to move at, so it does not differ from the role's default in any way the screen can act on.

The lead was asked this.
The session brief on draft/directory/8c1bee6c was renumbered to DIRECTORY-021 at 16:55, one minute after this brief took that id.
DIRECTORY-019 no longer exists on that branch and names three other briefs elsewhere.
Which brief keeps DIRECTORY-021, and by what id should this brief's blockers and R7 cite the session brief's start (R4) and stop (R6)?
The lead ruled as follows.
This brief keeps DIRECTORY-021, because it wrote the id first (16:54, against 16:55), and the first writer keeps an id.
The session brief moves to the next free id, and I will say so on its next round.
Because that number is not settled yet, this brief's blockers and R7 cite the session brief by its run and requirement: brief run 8c1bee6c, its start requirement R4 and its stop requirement R6, on draft/directory/8c1bee6c.
Each blocker carries a command a stranger can run that reads that branch's brief for those requirements.
When the session brief lands under its final id, a later edit may swap the citation for the number.
Answered by Archie, lead for the identity line.

The lead was asked this.
A newer enforcement draft, DIRECTORY-020 on draft/directory/09a6cc80 carrying ADR-062, creates the same spicedb/check.rs, schema.zed and relationships.rs as DIRECTORY-014/ADR-043 on d40aa430.
Should the brief keep citing ADR-043 and DIRECTORY-014 on d40aa430, as the words say, or cite the newer draft?
The artifact keys are the same either way.
The lead ruled as follows.
Cite the newer draft, DIRECTORY-020 with ADR-062 on draft/directory/09a6cc80.
That is the enforcement brief the lead is ruling now, and it carries the grant and permission design being built. Run d40aa430's DIRECTORY-014 and ADR-043 are superseded for spicedb/check.rs, schema.zed and relationships.rs, so the brief names 09a6cc80 and keeps the same artifact keys.
Answered by Archie.

## What the survey found, and its angles

Write the directory cluster's brief for roles and their versions (CONFORMANCE build step 6, under ADR-006). It has to carry conformance rows 4.2, 4.3, 4.4 and the roles half of 4.5, each with a named acceptance line, and it is for agent holders only. A role is a set of versioned grant templates, defined in exactly one project. A holding copies its version's templates into grants when it is granted. A holder moves to a newer version only by a deliberate, recorded move (at its next start, or now after every open session is stopped) or by a renewal, which this brief introduces. The brief also writes two new proposed decisions into DESIGN.md's Decisions section for Archie to read second: 4.7 (a template copied at grant time) and 4.6 (move at the next renewal is the default for a holding with an end date, and deliberate-only for one without). It adds the explicit owner relation on a project, and it keys every blocker on artifact paths checked with git cat-file -e against origin/main.

### What the tree holds

- `docs/design/directory/design.json → DESIGN.md (render-cluster.py)` — The Decisions section lists 8 ADRs on main (003, 004, 005, 007, 008, 009, 010, 011) and not ADR-006. The two new decisions have to go into design.json and be re-rendered; DESIGN.md is never hand-edited, because gate.sh compares the rendered markdown byte for byte.
- `docs/design/directory/DESIGN.md:63 (Non-Goals, 'Road step 2 onward')` — The ruling says this sentence stands unamended and the brief records one boundary sentence about it. It is the sentence the words tell the author to quote rather than rewrite.
- `docs/design/directory/DESIGN.md:57 (Non-Goals, 'The grant representation … OPEN for Tom')` — Grant templates are written in the grant representation, which main still records as OPEN for Tom. ADR-062 on draft/directory/09a6cc80 (proposed) is what decides it.
- `docs/design/directory/DESIGN.md:185 CN1 and :195 CN11` — CN1 (documents only) binds the planning task, as ruled. CN11 keeps enforcement out of step 1, which is why the SpiceDB check sits behind a seam and waits on the enforcement brief's artifact.
- `docs/design/identity/CONFORMANCE.md §4 rows 4.1–4.7 and build step 6` — The rows this brief carries (4.2 to 4.5) and decides (4.6, 4.7). Row 4.1 stays proposed. The brief must not edit this file.
- `docs/design/directory/briefs/DIRECTORY-006.md:116-122 (R4, GRANT_EXPIRY)` — The directory half of 4.5 lives here and is named as a neighbour. It already says 'a role edit or version move SHALL NOT renew a provisional grant', and its freshness mechanism is still marked 'Proposal under review … must be settled before dispatch'.
- `docs/design/directory/briefs/DIRECTORY-006.md R1/R2/R3` — The grant contract that templates are written in (R1, still a proposal), the per-grant admission every copied template goes through for the granting actor (R2), and the one signed commit path (R3).
- `docs/design/directory/briefs/DIRECTORY-002.md:125-127 (R4)` — 'SpiceDB enforces nothing in step 1'; no SpiceDB check in crates/lys/src/identity/. The words require holding to it.
- `docs/design/directory/briefs/DIRECTORY-003.md:85-87 (R3) and :76 (creates docs/design/identity/IDENTITY-EVENTS.md)` — R3 defines the configured administrator: the only operator the session stop admits, and the only party besides an owner that may give the owner relation. The envelope document the role events are added to is created by DIRECTORY-003 and is not on main.
- `origin/draft/directory/09a6cc80-… DIRECTORY-020 (ADR-062) R2/R3` — This branch creates crates/lys-identity-server/src/spicedb/check.rs, schema.zed and crates/lys-identity/src/grants/relationships.rs: the artifact keys for the SpiceDB leg of the role-check seam and for the owner relation.
- `origin/draft/directory/8c1bee6c-… session brief R4 (session/start.rs) and R6 (session/end.rs)` — R7's next-start leg is blocked on crates/lys-identity/src/session/start.rs and its now leg on crates/lys-identity/src/session/end.rs. R6's stop admits only DIRECTORY-003 R3's configured administrator, and each stop ends one session id.
- `origin/draft/directory/7494cb8b-… DIRECTORY-021.md/.json` — This card's earlier draft: 11 requirements, 49 ROLE_* acceptance lines, ADR-066/067, C151–C162, S62–S67, and the boundaries the rulings answered. It is the nearest text for the author to reconcile against the rulings.
- `docs/design/identity/mockup/index.v5.html (roles view)` — 'When this role changes' with the initial value 'stay'; the move drawer 'At its next start' / 'Now: ends its running session first'; 'each holder shows the date it will move, and who can stop it'; the Confirm button on a provisional holding. The brief differs from this drawing in two places (the default, and 'draft vN+1 when you change anything') and records both as boundaries.
- `scripts/design/gate.sh, validate.py, check-coverage.py, render-cluster.py, schemas/brief.schema.json` — The method the brief, checklist, stories, decisions and roadmap entries must pass. gate.sh re-renders every cluster and compares it with the committed markdown.
- `docs/design/decisions.json, docs/design/roadmap.json, docs/design/directory/checklist.json, stories.json` — Where the two proposed ADRs, the roadmap row, the checklist items and the stories are added.

### What was already decided

- ADR-006 — Editing a role makes a new version; a holder never changes silently; a move is a deliberate recorded act or an explicit rule shown in advance; which is the default is open; a provisional holding lapses at its end date and nothing renews it unless granted again.
- ADR-003 — Every grant is rooted in a person; the person's permissions are the ceiling, so an owner or responsible person must hold at or above what each template gives.
- ADR-011 (proposed) — An agent's responsible person is fixed for life and is 'the person the holder answers to'. It is only proposed, yet the renewal, move and assign authority rests on it.
- ADR-005 — The identity database is PostgreSQL.
- ADR-010 — The role screen follows Aion's structure with the identity orange accent.
- ADR-062 (proposed, draft/directory/09a6cc80, DIRECTORY-020) — The grant representation and the one SpiceDB evaluator, which the lead ruled this brief cites in place of ADR-043/DIRECTORY-014 on d40aa430.
- ADR-066, ADR-067 (proposed, draft/directory/7494cb8b only) — This card's earlier draft of the 4.7 and 4.6 decisions. The ids are unused on main and on any other branch.
- DIRECTORY-002 R4 — SpiceDB enforces nothing in step 1; no permission check or relationship write in crates/lys/src/identity/.
- DIRECTORY-003 R3 — Only the explicitly configured administrator changes the directory; general assignment arrives in step 2.
- DIRECTORY-006 R1–R4 — The grant contract, bounded admission per grant, one signed replayable commit, and enforcement of revocation and inherited expiry, where a role edit or version move never renews a provisional grant.
- Session brief, run 8c1bee6c R4/R6 — Start on report-back (session/start.rs); stop one session id, admitted only for the configured administrator (session/end.rs).
- CONFORMANCE §4 and build order step 6 — Rows 4.2–4.5 are 'test (once ADR lands)'; 4.6 and 4.7 are open, for an ADR; 4.1 stays proposed; roles come after the ADR.
- DESIGN.md CN1, CN9, CN11, CN12 — Documents-only planning task; a file outside the wall stops the row; no enforcement in step 1; do not dispatch a dependency-blocked brief.
- RM-001 (briefed) — The directory feature row, the only roadmap row on main in this cluster's line. The earlier draft instead added RM-041.

### What was measured

- Highest DIRECTORY id on any origin branch: DIRECTORY-023 (d2066fa6); 234 remote branches scanned
- Branches holding DIRECTORY-021 now: 2: 7494cb8b (this card's earlier draft) and 8c1bee6c (the session brief, last commit 18:53 on 27 Sep, not yet renumbered)
- Briefs on main in docs/design/directory/briefs: 7 (DIRECTORY-001 to 006, and 008)
- Highest ADR id on main / on any branch: ADR-018 / ADR-075; ADR-066 and ADR-067 are used only on 7494cb8b; ADR-071 is used nowhere
- Highest RM id on main / on branches: RM-016 / RM-041, added by both 7494cb8b and 8c1bee6c (a collision)
- Checklist and story ids on main in the directory cluster: C30 highest, S12 highest; the earlier draft used C151–C162 and S62–S67
- Decisions listed in the directory DESIGN.md Decisions section on main: 8, and ADR-006 is not among them
- Earlier draft size: 11 requirements, 49 ROLE_* acceptance lines, DIRECTORY-021.md 329 lines, .json 400 lines
- Artifact files on origin/main: 0 of 5: spicedb/check.rs, schema.zed, grants/relationships.rs, session/start.rs and session/end.rs are all absent (git cat-file -e)
- crates/lys-identity, crates/lys-identity-server, surface/identity on main: none exist; main's crates are lys, lys-anchor, lys-anchor-cli, lys-core, lys-home, lys-log-store
- docs/design/identity/IDENTITY-EVENTS.md and GRANT-CONTRACT.md on main: both absent; DIRECTORY-003 creates the first and DIRECTORY-006 the second
- sh scripts/design/gate.sh on HEAD 7b53625: exit 0, coverage clean
- HEAD against origin/main: origin/main is 2 commits ahead (fa3dd53), with no change under docs/design
- Directory cluster file sizes on main: DESIGN.md 196 lines, CHECKLIST.md 49, design.json 770, checklist.json 185, stories.json 83, decisions.json 330, roadmap.json 336, CONFORMANCE.md 118, DIRECTORY-006.md 210
- Mock-up role policy initial value: 'stay' (S.rolePolicy[roleId] || 'stay'), in index.v5.html (197,852 bytes)

### What it means for the other projects

- aion — The brief goes through the chain brief_card → sign-off → card_build_v3 → src_pr → src_land. The blockers are artifact keys the chain has to check with git cat-file -e before it dispatches R2/R10 and R7.
- cambium — The card lives on Cambium and is signed off there by Tom or the lead. No Cambium code changes, and rows 06 and 07 stay a non-goal.
- method — The brief, the decisions, the checklist, the stories and the roadmap entries must validate against the method's schemas as vendored in scripts/design/schemas. Nothing in the method changes.

### The decisions it stands on

- ADR-006 (honour) — The brief implements it: edits make versions, no holder moves silently, and a provisional holding lapses. The two new entries answer its open sentence and cite it, and supersede nothing.
- ADR-003 (honour) — Every copied template grant is admitted under DIRECTORY-006 for a granting actor rooted in a person, and ownership is a grant rooted in a person.
- ADR-011 (honour) — The responsible person is the person the holder answers to. The ADR is still proposed (see questions).
- ADR-005 (honour) — Holdings and role records live in the identity PostgreSQL store behind the directory's signed events.
- ADR-010 (honour) — The role screen uses the identity orange and Aion's structure.
- ADR-009 (honour) — The words cited it by mistake and withdrew it. Nothing in this brief touches Rauthy.
-  (new) — 4.7: a role is a set of versioned grant templates copied at grant time, not a live group. A holding names its role and version, an edit changes no existing grant, holders are answered by a query over holdings, and only templates make a version. Recorded as proposed, decided_by empty, with Apollo as author and Archie as second reader.
-  (new) — 4.6: move at the next renewal is the default for a holding with an end date, shown as the end date with the words about the move and who can stop it; a holding with no end date moves only by a deliberate act; the role carries a default and each holding its own policy. Recorded as proposed.

### What it requires

- The brief is written as docs/design/directory/briefs/DIRECTORY-021.json and .md, and sh scripts/design/gate.sh exits 0.
- Each of CONFORMANCE 4.2, 4.3, 4.4 and 4.5 is named by at least one acceptance line that tests it.
- Two new decisions, one for 4.7 and one for 4.6, are in docs/design/decisions.json with status proposed and decided_by empty, with Apollo as author and Archie as second reader named in context. They appear in design.json's decisions and in the rendered DESIGN.md Decisions section, and they cite ADR-006 without superseding it.
- Role, version and holding records: a role belongs to exactly one project, a committed version is immutable, and a holding names its role, its version, its project, its grant ids, its end date and its own policy.
- The owner relation on a project is added to the authorization model. Only a directory administrator or an existing owner may give it, it is never inferred, and it is the one ownership test for Assign, Move, policy change and the role editor.
- An acceptance line shows that editing a role's templates while it has holders makes a new version and leaves every holder on the version it held with its grants unchanged, and that a title edit is recorded and makes no version.
- Assign by the responsible person (H1) and assign by a project owner each have an acceptance line. A template the actor cannot give refuses the assign with row 2.4's reasons.
- A move by hand shows what it adds and removes before it is taken, is admitted only for the responsible person or a project owner, and leaves an audit record naming the actor, the capacity and the timing chosen.
- A move at the next start leaves a running session on its version. A move now is taken only by the admitted operator, stops every open session one id at a time, lists each as stopped or unconfirmed, and names each in the audit record. Any other mover's now leg is refused by name.
- Renewal (R8) is defined for both policies: under move-at-next-renewal it lands at the current version, and under deliberate-only at the held version. It is a new grant with a new end date, recorded with the actor, the holding and the version. Anyone else is refused as ROLE_RENEW_REFUSED.
- Two acceptance lines for the renewal policy: one where a renewal at or before the end date puts the holder on the new version, and one where nobody renews and the holding has lapsed without moving.
- A holder under the renewal policy shows its end date, the words that the next renewal moves it to the current version, and who can stop the move, and the 4.4 acceptance line asserts all three. A holding with no end date shows 'Moves only by a deliberate act' and is unmarked.
- A provisional holding's end date is the same after an edit and after a move.
- Changing a role's default applies only to later grants. Changing one holding's policy is recorded with the actor and the capacity. The role screen marks the holdings whose policy differs from the default.
- Every role-version check goes through one seam that reads the holding record now and asks SpiceDB once crates/lys-identity-server/src/spicedb/check.rs is on origin/main. The browser only displays what the server permits.
- The blockers carry the git cat-file -e origin/main commands for spicedb/check.rs, schema.zed, grants/relationships.rs, session/start.rs and session/end.rs, and each session blocker carries a command that reads branch 8c1bee6c's brief for R4 or R6.
- The boundaries hold one sentence each on: the unamended non-goal; the mock-up's 'stay' default; the mock-up's 'draft vN+1 when you change anything'; Confirm is outside; 4.1 is outside; the directory half of 4.5 is DIRECTORY-006 R4's; and no amendment is needed before the file walls, placed beside the walls.

### What must not change

- DESIGN.md's hand-written sentences (non-goals, constraints, principles) are not rewritten; only the rendered decision and structure listings change.
- docs/design/identity/CONFORMANCE.md, the mock-up, and every other brief's requirements are not edited.
- ADR-006 is not superseded or rewritten.
- DIRECTORY-002 R4 stands: this brief brings in no enforcement and no SpiceDB check in crates/lys/src/identity/.
- DIRECTORY-006 R4's expiry rules are named as the neighbour, not repeated.
- The session stop's admission (the session brief's R6, the configured administrator) is not widened.
- No published lys-core wire format changes; role events are drafted in the envelope document and reviewed before any is signed.
- No holder changes version and no end date moves as a side effect of an edit, a default change, a policy change or a retry.
- No code is written before sign-off on the card.

### What we must put in place first

- DIRECTORY-003 lands crates/lys-identity and docs/design/identity/IDENTITY-EVENTS.md, which the role events are added to.
- DIRECTORY-006 lands the grant contract (GRANT-CONTRACT.md, grants/types.rs), its admission and its commit path, and settles R4's freshness mechanism before dispatch.
- DIRECTORY-005 lands surface/identity for the role screen.
- The enforcement brief DIRECTORY-020 (ADR-062, 09a6cc80) lands spicedb/check.rs, schema.zed and grants/relationships.rs before the SpiceDB leg of the seam and the owner relation's projection.
- The session brief (run 8c1bee6c) lands session/start.rs (R4) and session/end.rs (R6) before R7's two legs.
- The session brief on 8c1bee6c gives up DIRECTORY-021 as the lead ruled; until it does, the two branches collide on the id and on RM-041.

### The risks

- An id collision: both 8c1bee6c and this card claim DIRECTORY-021, and both 7494cb8b and 8c1bee6c add RM-041. Whichever lands second fails validation or overwrites the other.
- ADR-066/067 exist only on the earlier draft. Another draft could take them before this one lands.
- The brief rests on four unlanded foundations and one proposed ADR (ADR-011). Paths named for modification may not match what those briefs land (CN9, CN12).
- The grant representation is still OPEN for Tom in DESIGN.md. If ADR-062 decides it differently, the template shape and the move preview change.
- Role events are new signed directory kinds. Once signed durably they are frozen, so the encoding needs review first.
- Four orderings can be read one-sidedly in tests: the renewal-policy move date, the lapse leg, provisional end dates surviving an edit and a move, and the refusals. Each acceptance line has to count its cases and assert its refusals.
- The now leg is admitted only when the mover is also the configured administrator, so in practice most movers can only choose the next start. A reader may take this for a defect.
- The mock-up still draws 'stay' as the default and 'vN+1 on any change', so screens built from the drawing would contradict the brief until its owner updates it.

### Still open

- The grant templates that make a version must be written in a grant representation, and DESIGN.md's non-goal still records that representation as OPEN for Tom. Are the templates written in ADR-062's representation, making this brief blocked on ADR-062 being decided as well as on its files landing, or in DIRECTORY-006 R1's proposed contract as it stands? The sentence of the words it stands on: "It depends on DIRECTORY-006 for grants and amends no non-goal.". Why only the lead can settle it: docs/design/directory/DESIGN.md:57 says 'The grant representation … OPEN for Tom', and DIRECTORY-006 R1's contract is still marked a proposal for review. What a holder carries, and what a move shows as added and removed, is made of that representation. Leaving the non-goal unamended while depending on it leaves the template shape resting on a decision nobody has made.
- The responsible person, who assigns, renews, moves and changes policy, is defined by ADR-011, which is still proposed. Does this brief rest on ADR-011 as it stands, or wait for it to be decided? The sentence of the words it stands on: "A renewal is a new grant of the holding by the person the holder answers to, the responsible person of ADR-011, made at the role's current version with a new end date, recorded as a renewal that names the actor, the holding it renews and the version it lands on.". Why only the lead can settle it: docs/design/decisions.json marks ADR-011 proposed. If it changed, who may act on a holding would change for every holder.

### The units beyond the first

- People holding roles, and who a person holder answers to — Ruled out of this brief, which covers agent holders only. That unit settles who a person holder answers to.
- Confirm: make a provisional holding permanent — It is a new grant without an end date and belongs with the grant rows. It is named as outside.
- Row 4.1: a role's job and profile — Still proposed in CONFORMANCE. Only the templates make a version here.
- Amend the directory design's road-step-2 non-goal — A documents card for the design's owner, as ruled. This brief only records one sentence about it.
- Bring the mock-up's role policy default and version drawing into line — The mock-up's owner changes 'stay' and 'draft vN+1 when you change anything' to match the brief.
- Swap the brief's session and enforcement citations for their final ids — A later edit, once the session brief lands under its final number.

### The smallest complete shape

One brief, DIRECTORY-021 as .json and a rendered .md in docs/design/directory/briefs. It comes with two proposed decisions added to docs/design/decisions.json and listed in design.json's Decisions, its checklist items and stories in checklist.json and stories.json, a roadmap entry, and CHECKLIST.md, USER-STORIES.md and DESIGN.md re-rendered. It covers records, ownership and the check seam, assign, edit/versioning, the holder query, the move, session timing, renewal, the policy display and change, the server routes, and the role screen. Every conformance row 4.2 to 4.5 is named on an acceptance line, the blockers are keyed on the five artifact paths, and sh scripts/design/gate.sh passes. It is documents only; the code is built from it after sign-off.

## The roadmap row

- **RM-052** — Version roles as grant templates and move a holder only by a deliberate, recorded act (feature, idea)
- Summary: Roles and their versions under ADR-006, build step 6 of the identity conformance list: editing a role's templates makes a new version and moves no holder, a move by hand shows what changes and is recorded with its actor and capacity, a renewal introduced here moves a holding with an end date to the current version, project ownership is an explicit owner relation, and a provisional end date never changes on an edit or a move. Carries conformance rows 4.2, 4.3, 4.4 and the roles half of 4.5, and records 4.7 and 4.6 as the proposed ADR-089 and ADR-077.
- Asked by: tom on 2026-09-27T13:07:00+10:00
- Context: The roles card's words on the Lys board as typed, which carry the lead's rulings to runs 35d44b68 and 6af1f2da, with the lead's rulings in brief run 7494cb8b and its answers to this run: project ownership is an explicit owner relation this brief adds; a role is defined in exactly one project; the person the holder answers to and an owner of the role's project may renew; a move now stops every open session of the agent; agent holders only; the session brief is cited by its run 8c1bee6c and its requirements R4 and R6, keyed on their files; the enforcement brief cited is DIRECTORY-020 with ADR-062 on 09a6cc80; the templates are written in DIRECTORY-006's grant fields as built; and the brief rests on ADR-011 as it stands.
- Quote: Write the brief for this card in the directory cluster (docs/design/directory/briefs, the next id not used on main or on any origin branch, which at 13:00 on 27 September was DIRECTORY-015 with three more briefs being written beside this one, so take the id from the branches at write time), as road step 2 of the directory design (docs/design/directory/DESIGN.md): roles and their versions, to ADR-006. This brief carries conformance rows 4.2, 4.3, 4.4 and the roles half of 4.5 of docs/design/identity/CONFORMANCE.md, and each of those rows gets at least one acceptance line that tests it and names the row. Editing a role makes a new version, and no holder changes version by itself. Moving a holder to a newer version is a deliberate act that shows what changes before it is taken and is recorded with who did it. The policy of a move at the next renewal is explicit, and each holder shows the date it will move. A provisional holding has its end date on its grant, it lapses when the date passes and is never renewed quietly, and a version change never extends it. The directory half of row 4.5, that the grants a lapsed holding carried stop at the next check, is DIRECTORY-006 R4's and is named here as the neighbour, not repeated. Done when a role edited while it has holders leaves every holder on the version it held with its grants unchanged, when one holder moved by hand shows the change and leaves an audit record naming the actor, when a holder under the renewal policy shows its move date and moves on that date and not before, and when a provisional holding's end date is the same after an edit and after a move. Two open rows of the conformance list are decided here, and the brief writes both decisions into the directory design's Decisions section with these reasons, so that Archie reads them as second reader. Row 4.7 is decided as a template copied at grant time, not a live group. A holding names the role and the version it holds, the grants it carries are made from that version's templates when the holding is granted, and an edit to the role changes none of them, since a live group would change every holder's permissions at the moment of the edit, which ADR-006 forbids. Who holds a role remains answerable as a query over holdings. Row 4.6 is decided as a move at the next renewal being the default for a holding with an end date, shown on the holder as the date it will move, since a renewal is already a deliberate recorded act by the person the holder answers to, so the move rides on an act that exists and no holder ever moves without one. A holding with no end date has no renewal, so its default is no move without a deliberate act, and the screen says so on that holder. The person the holder answers to may change the policy of one holding, and the change is recorded with who did it. Hold to R4 of DIRECTORY-002 and to ADR-009, so that every check about a role version asks SpiceDB and the browser shows what is permitted and is never the authority. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after Tom or the lead signs it off on the card. Rulings of the lead, Apollo, given on 27 September 2026 to run 35d44b68-873f-4a10-8f4c-01cc2ecf1ee7 in answer to its rounds, settled here and not reopened by the author. It carries CONFORMANCE's build step 6, roles after the ADR, and my phrase road step 2 is withdrawn. It depends on DIRECTORY-006 for grants and amends no non-goal. If a sentence of DESIGN.md reads as excluding roles and versions, quote it as a question for the lead in the brief rather than rewriting it. ADR-009 was the wrong id and is withdrawn. The decision meant is the one the SpiceDB enforcement brief carries, ADR-043 on draft/directory/d40aa430, cited by id with a note that it lands with that brief. This brief does not bring enforcement in itself. The requirement that makes a role-version check ask SpiceDB is blocked on an artifact, the evaluator file that the enforcement brief creates, taken by path from that brief's file list on its origin branch and checked with git cat-file -e against origin/main. Until then the checks read the holding record through one seam so the swap is in one place. The date shown is the holding's end date, and the words beside it say that the next renewal moves the holding to the current version. Moves on that date means that a renewal made at or before the end date is made at the current version, so the holder is on the new version from the renewal. If nobody renews, the holding lapses on its end date under ADR-006 and DIRECTORY-006 R4 and there is no move. The acceptance has both lines, one where the renewal happens and the holder is on the new version, and one where it does not and the holding has lapsed. This brief defines renewal, since nothing on main does. A renewal is a new grant of the holding by the person the holder answers to, the responsible person of ADR-011, made at the role's current version with a new end date, recorded as a renewal that names the actor, the holding it renews and the version it lands on. It is one requirement with its own acceptance line, and my sentence saying the act already exists is corrected to say this brief introduces it. Both. A role carries a default policy, the mock-up's When this role changes, and a holding takes that default at grant time and may be changed on its own afterwards. The holding's own policy governs it. Changing the role's default applies to holdings granted after the change and moves nothing that exists, so no holder changes as a side effect, and the role screen shows the default and marks the holdings whose policy differs from it. The actor chooses, as the mock-up's move drawer offers, between at its next start, which is the default, and now, which ends the running session first. A running session keeps the version it started with. The audit record names which of the two was chosen. The person the holder answers to, and the owners of the project the role is assigned in, as the mock-up says. A directory administrator moves a holder only in one of those two capacities. The audit record names the actor and the capacity. Once the enforcement brief lands the check asks SpiceDB through the seam of my second answer. Both new entries are recorded as proposed, decided_by empty, with Apollo as author and Archie as second reader named in the context, and they become decided only when Tom rules or signs the card off. They do not supersede ADR-006; they answer the sentence it left open and cite it. The Confirm button that makes a provisional holding permanent is outside this brief and is named as outside in its boundaries, since it is a new grant without an end date and belongs with the grant rows. Only the versioned grant templates make a version. Row 4.1's job and profile stay proposed and outside this brief. The role screen offers the title and the grant templates, an edit to the templates makes a new version, and an edit to the title is recorded and makes no version. The owners of the project the role is assigned in edit its grant templates and its title and change its default policy, and a directory administrator does so only in that capacity. It is the same set that may move a holder, so one admission serves R4, R9 and the move, and every such act is recorded with the actor and the capacity. Under the policy of a move only by a deliberate act, a renewal is made at the version the holding holds, and it is still a new grant with a new end date recorded as a renewal. Under the policy of a move at the next renewal, it is made at the role's current version, as answered. R8 defines renewal for both policies in those two sentences, and the move to a newer version under the first policy still needs its own deliberate act. A move taken now asks for the session's end through DIRECTORY-015 R6's operator stop, with the mover as the actor and the move named as the cause in the audit record. The mover is admitted to that stop as the responsible person or a project owner only if DIRECTORY-015 admits them as an operator of that agent's sessions. If it does not admit them, the now leg is refused by name and the actor may take the move at its next start instead. R7's now leg is blocked on DIRECTORY-015's artifact, the file that defines the operator stop, taken by path from that brief's file list on its origin branch and checked with git cat-file -e against origin/main. Yes. An owner of the project a role is assigned in may grant a holding of it, as the mock-up says, and every grant copied from the version's templates is still admitted under DIRECTORY-006 for that granting actor, so the owner must hold at or above what each template gives. A template the owner cannot give makes the assign refuse with the reasons row 2.4 names, and R3 admits by both, the owner's capacity and DIRECTORY-006's admission of each copied grant. The non-goal sentence stands unamended and does not exclude this brief, exactly as ruled for the sibling brief 687de276. It describes what step 1 of the directory design set out to build, DIRECTORY-006 already sits beside it on main making grants, and its amendment is a documents card for the design's owner. The brief records that in one sentence of its boundaries and changes nothing in DESIGN.md. CN1 binds only the planning task of the directory design round, and each brief's build walls are the brief's own, as DIRECTORY-006 on main already shows. No amendment is needed before this brief's code paths or its addition to docs/design/identity/IDENTITY-EVENTS.md are built, and the brief says so in one sentence beside its file walls. Rulings of the lead, Apollo, given on 27 September 2026 to run 6af1f2da-2030-4d5d-b0d6-f7b685d99179 in answer to its round 1, settled here and not reopened by the author. A newly made role's default starts as move at the next renewal, as row 4.6 is decided. The mock-up's initial value of stay is a drawing default and does not govern, and the brief records the difference in one sentence of its boundaries so the mock-up's owner can bring the drawing into line. A holding with an end date takes that default at grant time and shows its move date. A holding with no end date has no renewal, so whatever the role's default it reads as no move without a deliberate act, and the screen says so on that holder. The now leg is taken only when the mover is the admitted operator of DIRECTORY-003 R3, since that is the only party DIRECTORY-015 R6 admits to a stop, and this brief does not widen that admission. When the mover is that operator and is also the responsible person or a project owner, the now leg is taken, the stop is recorded under DIRECTORY-015 with the operator as its actor, and the move's own audit record names the mover's capacity for the move, responsible person or project owner, and names the stop it caused. A mover who is not the admitted operator has the now leg refused by name and may take the move at its next start. My earlier sentence about an operator of that agent's sessions is corrected by this answer, since DIRECTORY-015 has no per-agent operator. Yes, it shows who can stop it, as the mock-up says. The holder row under the renewal policy shows the holding's end date, the words that the next renewal moves it to the current version, and who can stop the move, which is the person the holder answers to and the owners of the project the role is assigned in, the same set that may change the holding's policy. The acceptance line for row 4.4 asserts all three on the row. Yes. The person the holder answers to may grant a holding of a role to their own agent, and so may an owner of the project the role is assigned in, and each is admitted only if DIRECTORY-006 admits every grant copied from the version's templates for that granting actor. The earlier acceptance with the responsible person H1 granting the holding stands, and one more acceptance line has a project owner granting it. The Assign action is shown to both and to nobody else, and a directory administrator assigns only in one of those two capacities.
- Cluster: directory; briefs: DIRECTORY-021
- Notes: Ids read from main and every origin branch at write time. DIRECTORY-021 is kept, as the lead ruled: this brief wrote it first, and the session brief on draft/directory/8c1bee6c, which also holds it, moves to the next free id. ADR-077 and ADR-085: the 4.6 decision took ADR-077, and the 4.7 decision, first written as ADR-076, yielded it to draft/secrets/7a0935db, which wrote it first, then ADR-079 to draft/directory/07bad7e0 and draft/directory/73184e72, which committed it first; the lead named ADR-081 as the next free, but ADR-081 to ADR-084 were committed on draft/directory/8122eb55 and draft/directory/c7c2ac1b before this write, so it was written as ADR-085, which draft/directory/73184e72 then committed on origin first; origin is the record the next-free-id rule reads, so it is ADR-089, the next free past main and every origin branch at write time. RM-051: RM-041, this row's first id, is also held on draft/directory/8c1bee6c; RM-046 was taken on draft/directory/2da3cf8c and RM-049 on draft/directory/07bad7e0 and draft/directory/73184e72, which wrote them first; the lead named RM-050, but RM-050 was committed on draft/directory/8122eb55 and draft/directory/c7c2ac1b before this write, so it was written as RM-051, which draft/directory/73184e72 committed on origin first, so it is RM-052. C169 to C180, S71 to S75 and S98, since C157 is held on draft/directory/0f34978d and C168 and S70 were the highest on any branch; the owner story, first written as S76, yielded S76, S77 and S80 to the branches that committed them first; the lead named S84, but S84 to S94 were committed on draft/directory/8122eb55 and draft/directory/c7c2ac1b before this write, so it was written as S95, which draft/directory/73184e72 committed on origin first, so it is S98. Further units, not written: People holding roles, and who a person holder answers to; Confirm: make a provisional holding permanent; Row 4.1: a role's job and profile; Amend the directory design's road-step-2 non-goal; Bring the mock-up's role policy default and version drawing into line; Swap the brief's session and enforcement citations for their final ids.

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
- ADR-077 — A holding with an end date moves at its next renewal by default; one without moves only by a deliberate act — A move at the next renewal is the default for a holding with an end date, shown on the holder as the date it will move, which is the holding's end date, with the words that its next renewal moves it to the current version and who can stop the move. A renewal, which the roles brief introduces, is a deliberate recorded act by the person the holder answers to (the responsible person of ADR-011) or by an owner of the role's project: a new grant of the holding with a new end date, recorded naming the actor, the holding it renews and the version it lands on. So the move rides on a deliberate recorded act and no holder ever moves without one; unrenewed, the holding lapses at its end date under ADR-006 and does not move. A holding with no end date has no renewal, so whatever the role's default it moves only by a deliberate act, and the screen says so on that holder. A role carries a default policy, and a newly made role's default is a move at the next renewal; a holding with an end date takes that default at grant time, and the person the holder answers to or an owner of the role's project may change the policy of one holding afterwards, recorded with who did it and in which capacity. The holding's own policy governs it. Under a move only by a deliberate act a renewal is made at the version the holding holds; under a move at the next renewal it is made at the role's current version. Changing a role's default applies to holdings granted after the change and moves nothing that exists. Rejected: holders staying on their version until moved as the default for a holding with an end date, because the renewal is a deliberate recorded act the move can ride on; and a policy set only per role, because one holding could not then be kept apart.
- ADR-089 — A role is a set of grant templates copied at grant time, not a live group — A role is a template copied at grant time, not a live group. A holding names the role and the version it holds, the grants it carries are made from that version's templates when the holding is granted, and an edit to the role changes none of them. Who holds a role remains answerable as a query over holdings. Only the versioned grant templates make a version; a title edit is recorded and makes no version, and conformance 4.1's job and profile stay proposed and outside. Rejected: a live group, whose membership carries the role's current grants, because a live group would change every holder's permissions at the moment of the edit, which ADR-006 forbids.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.

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
| `docs/design/directory/briefs/DIRECTORY-021.json` | the roles and their versions brief, conformance 4.2 to 4.5 (roles half) | DIRECTORY-021 |
| `docs/design/directory/briefs/DIRECTORY-021.md` | rendered markdown | DIRECTORY-021 |
| `crates/lys-identity/src/roles/mod.rs` | R1: Define the role, version and holding records and their signed events | DIRECTORY-021 |
| `crates/lys-identity/src/roles/types.rs` | R1: Define the role, version and holding records and their signed events | DIRECTORY-021 |
| `crates/lys-identity/src/roles/events.rs` | R1: Define the role, version and holding records and their signed events | DIRECTORY-021 |
| `crates/lys-identity/src/roles/error.rs` | R1: Define the role, version and holding records and their signed events | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_records.rs` | R1: Define the role, version and holding records and their signed events | DIRECTORY-021 |
| `crates/lys-identity/src/roles/check.rs` | R2: Test project ownership by the explicit owner relation, and answer every role-version check through one seam | DIRECTORY-021 |
| `crates/lys-identity/src/roles/ownership.rs` | R2: Test project ownership by the explicit owner relation, and answer every role-version check through one seam | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_check.rs` | R2: Test project ownership by the explicit owner relation, and answer every role-version check through one seam | DIRECTORY-021 |
| `crates/lys-identity/src/roles/holding.rs` | R3: Assign a holding at the role's current version, its grants copied from that version's templates | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_assign.rs` | R3: Assign a holding at the role's current version, its grants copied from that version's templates | DIRECTORY-021 |
| `crates/lys-identity/src/roles/edit.rs` | R4: Make a role and a new version when its templates are edited, record a title edit, and move no holder | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_edit.rs` | R4: Make a role and a new version when its templates are edited, record a title edit, and move no holder | DIRECTORY-021 |
| `crates/lys-identity/src/roles/holders.rs` | R5: Answer who holds a role, and on which version, as a query over holdings | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_holders.rs` | R5: Answer who holds a role, and on which version, as a query over holdings | DIRECTORY-021 |
| `crates/lys-identity/src/roles/move_holder.rs` | R6: Move a holder to a newer version by a deliberate act that shows what changes first | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_move.rs` | R6: Move a holder to a newer version by a deliberate act that shows what changes first | DIRECTORY-021 |
| `crates/lys-identity/src/roles/timing.rs` | R7: Keep a running session on the version it started with, and stop every open session first for a move now | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_move_timing.rs` | R7: Keep a running session on the version it started with, and stop every open session first for a move now | DIRECTORY-021 |
| `crates/lys-identity/src/roles/renewal.rs` | R8: Renew a holding as a new recorded grant, at the version its policy names | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_renewal.rs` | R8: Renew a holding as a new recorded grant, at the version its policy names | DIRECTORY-021 |
| `crates/lys-identity/src/roles/policy.rs` | R9: Show each holding's policy, move date and who can stop the move, and change a holding's policy or a role's default by a recorded act | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_policy.rs` | R9: Show each holding's policy, move date and who can stop the move, and change a holding's policy or a role's default by a recorded act | DIRECTORY-021 |
| `crates/lys-identity-server/src/roles.rs` | R10: Expose the role acts through the identity server, which alone decides them, and carry the owner relation into SpiceDB | DIRECTORY-021 |
| `crates/lys-identity-server/tests/roles.rs` | R10: Expose the role acts through the identity server, which alone decides them, and carry the owner relation into SpiceDB | DIRECTORY-021 |
| `crates/lys-identity/tests/roles_owner_relationship.rs` | R10: Expose the role acts through the identity server, which alone decides them, and carry the owner relation into SpiceDB | DIRECTORY-021 |
| `surface/identity/src/features/roles/RoleVersions.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/src/features/roles/RoleHolders.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/src/features/roles/MoveHolder.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/src/features/roles/HoldingPolicy.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/src/features/roles/RoleBuilder.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/src/features/roles/AssignRole.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/tests/roles.test.tsx` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |
| `surface/identity/tests/acceptance/roles.spec.ts` | R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers | DIRECTORY-021 |

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
id: DIRECTORY-021
cluster: directory
title: Version roles as grant templates copied at grant time, and move a holder only by a deliberate, recorded act
---

# DIRECTORY-021: Version roles as grant templates copied at grant time, and move a holder only by a deliberate, recorded act

> **Cluster:** directory
> **Depends on:** DIRECTORY-005, DIRECTORY-006
> **Blocked by:** Sign-off of this brief on its card by the person who owns the project or the lead who owns the card. No code is written from it before that sign-off., DIRECTORY-006 landing on the main branch. The grant templates that make a version are written in DIRECTORY-006's grant fields as built: relation, resource, actions, pass_on (use_only, or to with its actions and recipients), window and responsible. This brief does not wait on any further decision about the grant representation. Checked by: git fetch origin main && git cat-file -e origin/main:crates/lys-identity/src/grants/types.rs && git cat-file -e origin/main:crates/lys-identity/src/grants/permission.rs && git cat-file -e origin/main:docs/design/identity/GRANT-CONTRACT.md., DIRECTORY-003 and DIRECTORY-005 landing on the main branch: crates/lys-identity, docs/design/identity/IDENTITY-EVENTS.md and surface/identity do not exist there, and surface/identity/src/generated/index.ts, which R11 modifies and DIRECTORY-006 R6 also modifies, does not exist there either. Reconcile every modify path below against their landed files before dispatch (CN9, CN12). Checked by: git fetch origin main && git cat-file -e origin/main:crates/lys-identity/src/lib.rs && git cat-file -e origin/main:docs/design/identity/IDENTITY-EVENTS.md && git cat-file -e origin/main:crates/lys-identity-server/src/routes.rs && git cat-file -e origin/main:surface/identity/src/routes.tsx && git cat-file -e origin/main:surface/identity/src/generated/index.ts., R2's SpiceDB leg, R10's seam swap and R10's ROLE_ROUTES_SPICEDB are blocked on an artifact: crates/lys-identity-server/src/spicedb/check.rs, created by R3 of the SpiceDB enforcement brief DIRECTORY-020 on origin/draft/directory/09a6cc80-5c46-4b7a-9e08-0561887a7d3a, which carries ADR-062. ADR-062 is proposed, is cited by id with its branch, and lands with that brief; it replaces DIRECTORY-014 and ADR-043 on draft/directory/d40aa430 for this file. Checked by: git fetch origin main && git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/check.rs. Until it passes, the checks read the holding record through R2's one seam., R10's owner relation in SpiceDB is blocked on artifacts: crates/lys-identity-server/src/spicedb/schema.zed and crates/lys-identity/src/grants/relationships.rs, both created by R2 of the SpiceDB enforcement brief DIRECTORY-020 on origin/draft/directory/09a6cc80-5c46-4b7a-9e08-0561887a7d3a, which this brief modifies to add the owner relation and its projection. Checked by: git fetch origin main && git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/schema.zed && git cat-file -e origin/main:crates/lys-identity/src/grants/relationships.rs., R7's now leg is blocked on an artifact: crates/lys-identity/src/session/end.rs, which defines the operator stop, created by the stop requirement R6 of the session brief of brief run 8c1bee6c on draft/directory/8c1bee6c-7421-4bf2-9753-922ad3546590; the session brief is cited by its run and requirement because its final id is not settled. Checked by: git fetch origin main && git cat-file -e origin/main:crates/lys-identity/src/session/end.rs. The requirement is read by: git fetch origin draft/directory/8c1bee6c-7421-4bf2-9753-922ad3546590 && git grep -l -F 'crates/lys-identity/src/session/end.rs' origin/draft/directory/8c1bee6c-7421-4bf2-9753-922ad3546590 -- 'docs/design/directory/briefs/*.json' | while IFS= read -r f; do git show "$f" | python3 -c "import json,sys; b=json.load(sys.stdin); [print(r['id'], r['title']) for r in b['requirements'] if 'crates/lys-identity/src/session/end.rs' in r['files']['create']]"; done, which prints one line beginning R6., R7's next-start leg is blocked on an artifact: crates/lys-identity/src/session/start.rs, which starts a session, created by the start requirement R4 of the session brief of brief run 8c1bee6c on draft/directory/8c1bee6c-7421-4bf2-9753-922ad3546590. Checked by: git fetch origin main && git cat-file -e origin/main:crates/lys-identity/src/session/start.rs. The requirement is read by: git fetch origin draft/directory/8c1bee6c-7421-4bf2-9753-922ad3546590 && git grep -l -F 'crates/lys-identity/src/session/start.rs' origin/draft/directory/8c1bee6c-7421-4bf2-9753-922ad3546590 -- 'docs/design/directory/briefs/*.json' | while IFS= read -r f; do git show "$f" | python3 -c "import json,sys; b=json.load(sys.stdin); [print(r['id'], r['title']) for r in b['requirements'] if 'crates/lys-identity/src/session/start.rs' in r['files']['create']]"; done, which prints one line beginning R4., The seven event kinds of R1 are reviewed with DIRECTORY-003's envelope, and an adversarial review of their signed payloads is done, before any is durably signed.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> - ADR-006 — A role changes only by a deliberate, explicit act — Editing a role makes a new version of it, and a holder never changes version silently as a side effect of the edit. A holder moves between versions either by a deliberate, recorded act or by an explicit rule shown in advance to the person the holder answers to, such as a move at the holding's next renewal. Which of the two is the default is open. A provisional role is a role held under a grant with an end date: when the date passes the holding lapses, and nothing renews it unless someone grants it again.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-077 — A holding with an end date moves at its next renewal by default; one without moves only by a deliberate act — A move at the next renewal is the default for a holding with an end date, shown on the holder as the date it will move, which is the holding's end date, with the words that its next renewal moves it to the current version and who can stop the move. A renewal, which the roles brief introduces, is a deliberate recorded act by the person the holder answers to (the responsible person of ADR-011) or by an owner of the role's project: a new grant of the holding with a new end date, recorded naming the actor, the holding it renews and the version it lands on. So the move rides on a deliberate recorded act and no holder ever moves without one; unrenewed, the holding lapses at its end date under ADR-006 and does not move. A holding with no end date has no renewal, so whatever the role's default it moves only by a deliberate act, and the screen says so on that holder. A role carries a default policy, and a newly made role's default is a move at the next renewal; a holding with an end date takes that default at grant time, and the person the holder answers to or an owner of the role's project may change the policy of one holding afterwards, recorded with who did it and in which capacity. The holding's own policy governs it. Under a move only by a deliberate act a renewal is made at the version the holding holds; under a move at the next renewal it is made at the role's current version. Changing a role's default applies to holdings granted after the change and moves nothing that exists. Rejected: holders staying on their version until moved as the default for a holding with an end date, because the renewal is a deliberate recorded act the move can ride on; and a policy set only per role, because one holding could not then be kept apart.
> - ADR-089 — A role is a set of grant templates copied at grant time, not a live group — A role is a template copied at grant time, not a live group. A holding names the role and the version it holds, the grants it carries are made from that version's templates when the holding is granted, and an edit to the role changes none of them. Who holds a role remains answerable as a query over holdings. Only the versioned grant templates make a version; a title edit is recorded and makes no version, and conformance 4.1's job and profile stay proposed and outside. Rejected: a live group, whose membership carries the role's current grants, because a live group would change every holder's permissions at the moment of the edit, which ADR-006 forbids.
> **Checklist:**
> - C169 — Roles, versions and holdings are typed records whose every act is one signed directory event naming the authenticated actor and its capacity, and a committed version never changes.
> - C170 — Project ownership is the explicit owner relation on the project in the directory's authorization model, given only by the directory's configured administrator or an existing owner of that project, and never inferred from a name, label or rank.
> - C171 — Every role-version check is answered through one server-side seam, SpiceDB's evaluator once it is on the main branch, and no request body supplies an actor, a capacity or a permission.
> - C172 — Assigning a role, by the person the agent answers to or an owner of the role's project, makes a holding that names its role and version with grants copied from that version's templates, each admitted under DIRECTORY-006 for the assigning actor (conformance 4.7, ADR-089).
> - C173 — Editing a role's grant templates makes the next version and leaves every holder on the version it held with its grants unchanged; editing its title is recorded and makes no version; a newly made role's default is move at the next renewal (conformance 4.2).
> - C174 — Who holds a role, and on which version, is answered as a query over holdings.
> - C175 — A move by hand shows what it adds and removes before it is taken, is admitted only for the person the holder answers to or an owner of the role's project, and is one signed record naming the actor, its capacity and the timing chosen (conformance 4.3).
> - C176 — A move at the holder's next start leaves a running session on the version it started with, and a move now, taken only by the admitted operator, stops every open session of the agent first and names in the move's record every session its listing showed, each with its outcome.
> - C177 — A renewal is a new grant of the holding with a new end date by the person the holder answers to or an owner of the role's project, at the role's current version under a move at the next renewal and at the held version otherwise, recorded naming the actor, the holding and the version; an unrenewed holding lapses and does not move (conformance 4.4).
> - C178 — Each holding shows its policy; one with an end date under a move at the next renewal shows its end date, that its next renewal moves it to the current version, and who can stop the move; one with no end date shows it moves only by a deliberate act; a holding's policy and a role's default change only by recorded acts that move no holder (conformance 4.4, ADR-077).
> - C179 — A provisional holding's end date is on its grants and is the same after a role edit and after a move (the roles half of conformance 4.5).
> - C180 — The role screen shows versions, holders on each version, move dates, policies and the move preview from the server's answers, shows Assign only to the person the holder answers to and the owners of the role's project, and offers an act only where the server says it is permitted.
> **Stories:**
> - S71 (Person a role holder answers to, Keeps each holding of a role on a version they chose) — As the person an agent answers to, I want an edit to its role to leave the agent on the version it holds so that its permissions never change without an act I can see.
> - S72 (Mover of a holder, Moves a holder to a newer version of its role) — As the person a holder answers to or an owner of the project its role is defined in, I want to see what a move adds and removes, and choose when it takes effect, before I take it so that I move a holder knowing what it gains and loses.
> - S73 (Person a role holder answers to, Keeps each holding of a role on a version they chose) — As the person a holder answers to or an owner of its role's project, I want to see the date each holding will move and who can stop it, and to change one holding's policy, so that no move surprises me.
> - S74 (Person a role holder answers to, Keeps each holding of a role on a version they chose) — As the person a provisional holder answers to or an owner of its role's project, I want renewing its holding to be a recorded act of one of us so that it is never renewed quietly and lapses when nobody renews it.
> - S75 (Reviewer of a role's history, Checks how each holder came to its version) — As a reviewer, I want every role act recorded with who made it and in which capacity so that I can verify how a holder came to the version it is on.
> - S98 (Owner of a project, Defines the roles of a project and assigns them) — As an owner of a project, I want to make and edit its roles and assign them to agents without any existing holder changing so that a role can improve without silently changing what its holders may do.

## Purpose

Give roles versions under ADR-006 so that no holder's permissions change without an act someone can see and answer for. A role's grant templates are versioned, a holding copies its version's templates into grants when it is granted (ADR-089), a holder moves to a newer version only by a deliberate act that shows what changes first or at a renewal this brief introduces (ADR-077), and every such act is one signed directory record naming its actor and the capacity it acted in (P4, P7). This carries conformance rows 4.2, 4.3, 4.4 and the roles half of 4.5 of docs/design/identity/CONFORMANCE.md, build step 6 of that list, roles and versions after the ADR.

## Task

Implement R1 to R11 in order, after DIRECTORY-006's grants and DIRECTORY-005's surface have landed. Fixtures used by the acceptance lines below: person H1, the responsible person to whom agents A1, A2, A3 and A4 answer; person O1, holding the owner relation on project P1; person O2; agent B1, whose responsible person is O2; person X1, holding neither; D1, the directory's configured administrator (DIRECTORY-003 R3), who is no agent's responsible person here and holds no owner relation on P1 unless a line gives it; role builder, defined in P1, whose version 1 carries the grant templates read-P1 and test-P1 and whose version 2, where one is made, carries read-P1 and push-P1; each template is written in DIRECTORY-006's grant fields: read-P1 is relation reader, resource project P1, actions [read], pass_on use_only, a window with no end of its own, responsible H1, a value R3 replaces on every copy with the holder's own responsible person; test-P1 and push-P1 are the same with relation tester and actions [test], and relation pusher and actions [push]; H1 and O1 each hold read-P1, test-P1 and push-P1 with authority to pass them on to agents, unless a line changes it; a controlled clock starting at T0, and end date E = T0 + 30 days. Every acceptance line that tests one of the carried conformance rows names it in parentheses. The directory half of conformance 4.5, that the grants a lapsed holding carried stop at the next check, is DIRECTORY-006 R4's (GRANT_EXPIRY) and is named here as the neighbour, not repeated: this brief keeps a holding's end date on its grants and never extends it, and DIRECTORY-006 R4 refuses at the end. Renewal is introduced here (R8), because nothing on the main branch defines one. A grant template is a grant with no holder and no source, written in DIRECTORY-006's grant fields as built (relation, resource, actions, pass_on, window, responsible), and a move shows the grants it adds and removes in those fields. The person the holder answers to is the responsible person of ADR-011, which is proposed and is used as it stands: it is the responsible field DIRECTORY-006 carries and enforces on every grant. Conformance rows 4.2 to 4.5 are marked test once the ADR lands, so they pass in full only once the role ADR is decided; ADR-089 and ADR-077, which this brief records, are proposed. Project ownership is the explicit owner relation R2 adds to the model's project definition, and it is the one test for assign, move, the policy change and the role editor. A role is defined in exactly one project and every holding of it sits there, so no owner of one project can make a version that holders in another project move to. Every role-version check goes through R2's one seam, which asks SpiceDB once the evaluator named in blocked_by is on the main branch and reads the holding record until then; the server decides and the browser only shows what the server says is permitted. In scope: role versions, the owner relation, holdings of agents, the move by hand with its preview and timing, renewal, the per-holding and per-role policy, the server routes and the role screen. Out of scope: conformance 4.1's job and profile, people holding roles, the act that makes a provisional holding permanent, SpiceDB enforcement itself (the SpiceDB enforcement brief DIRECTORY-020, ADR-062 on draft/directory/09a6cc80, proposed, which lands with that brief), and CONFORMANCE.md, which is not edited. The file walls are the files named in each requirement; CN1 binds only the planning task of the directory design round, each brief's build walls are the brief's own as DIRECTORY-006 on the main branch already shows, and no amendment is needed before this brief's code paths or its addition to docs/design/identity/IDENTITY-EVENTS.md are built.

## Requirements

### R1: Define the role, version and holding records and their signed events

Define in crates/lys-identity/src/roles/ the records this brief acts on. A role has a stable role id, exactly one project it is defined in, a title, a default move policy and an ordered list of versions. A version has a number, 1 for the first and one more than the last for each after it, and a set of grant templates, each a grant with no holder and no source, written in DIRECTORY-006's grant fields as built: relation, resource, actions, pass_on (use_only, or to with its actions and recipients), window and responsible; a committed version is immutable. A holding has a holding id, the holder's agent directory id, the role id, the version number it holds, the project it sits in, which is always the role's project, the ids of the grants it carries, its end date or none, and its own move policy; its end date is the one written on each grant it carries. The move policy has exactly two values, move_at_next_renewal and deliberate_only. An actor's capacity has exactly two values, responsible_person and project_owner. A move's timing has exactly two values, next_start and now. Each act of this brief is one signed directory event in DIRECTORY-003's reviewed envelope (P4, P7), of one of seven kinds: role version made, role title changed, role default policy changed, holding granted, holding moved, holding renewed, holding policy changed; each names the authenticated actor and the capacity it acted in. The seven kinds are added to docs/design/identity/IDENTITY-EVENTS.md and reviewed with that envelope. A committed version SHALL NOT be changed or deleted. A holding SHALL NOT sit in a project other than its role's, and SHALL NOT carry an end date apart from its grants'. A holding's holder SHALL be an agent; this brief SHALL NOT make a holding for a person. A record SHALL NOT carry conformance 4.1's job or profile. A missing field SHALL NOT be read as a default, and no default grants authority. No event kind SHALL be durably signed before the envelope review accepts it.

**Acceptance:**
- ROLE_RECORDS: a role with versions 1 and 2 round-trips through its event encoding unchanged; an event making version 3 of a role whose last version is 1 is refused as version_gap; an event replacing version 1's templates is refused as version_immutable. Three cases run, three outcomes asserted.
- ROLE_RECORDS_VALUES: a holding record whose move policy is absent is refused as missing_policy, one whose move policy is renewal_or_manual is refused as unknown_policy, one whose project differs from its role's is refused as project_mismatch, and a holding event whose capacity is owner_label is refused as unknown_capacity; none of the four yields a record.
- ROLE_RECORDS_EVENTS: one event of each of the seven kinds, committed in a test store by H1, reads back with actor H1; the count of events read back is 7.
- ROLE_RECORDS_TEMPLATE: template read-P1 round-trips through its encoding with relation reader, resource project P1, actions [read], pass_on use_only, a window with no end of its own and responsible H1; a template carrying a holder is refused as template_has_holder and one carrying a source as template_has_source; three cases run, three outcomes asserted.

**Files:**
- create: crates/lys-identity/src/roles/mod.rs
- create: crates/lys-identity/src/roles/types.rs
- create: crates/lys-identity/src/roles/events.rs
- create: crates/lys-identity/src/roles/error.rs
- create: crates/lys-identity/tests/roles_records.rs
- modify: crates/lys-identity/src/lib.rs
- modify: docs/design/identity/IDENTITY-EVENTS.md

**Checklist:**
- C169 — Roles, versions and holdings are typed records whose every act is one signed directory event naming the authenticated actor and its capacity, and a committed version never changes.

**Stories:**
- S75 (Reviewer of a role's history, Checks how each holder came to its version) — As a reviewer, I want every role act recorded with who made it and in which capacity so that I can verify how a holder came to the version it is on.

### R2: Test project ownership by the explicit owner relation, and answer every role-version check through one seam

Ownership of a project is an explicit relation, owner, on the project object in the directory's authorization model: this row adds that relation to the model's project definition in crates/lys-identity/src/grants/types.rs and to docs/design/identity/GRANT-CONTRACT.md. The owner relation is written as a grant like any other, one signed grant event through DIRECTORY-006 R3's commit path, and it is rooted in the person it is given to: that person is the grant's holder and its responsible person, and the giver is named as the authorising actor. The owner relation is a root of authority on the project and has no ancestor grant, so its admission is this row's rule and not DIRECTORY-006 R2's bounded-ancestry admission for a delegation. WHEN an actor gives the owner relation on a project to a person, THE SYSTEM SHALL admit it only when the actor is the directory's configured administrator, admitted as DIRECTORY-003 R3 admits it, or already holds the owner relation on that project, and otherwise SHALL refuse as not_permitted and commit nothing. The configured administrator needs no grant on the project to give it. The check is made at the moment of the give and asks only whether the giver holds the administrator relation or the owner relation then; how the first directory administrator comes to exist is settled by DIRECTORY-006's root-authority bootstrap review, not by this row, and a change there changes how that relation is granted and leaves this check as it is; this row's tests set up the administrator in their own fixture. THE SYSTEM SHALL NOT give the owner relation to an agent, and SHALL NOT let a holder of the owner relation pass on any other grant by holding it; every other grant still passes DIRECTORY-006 R2. THE SYSTEM SHALL NOT infer ownership from a name, a display label, a rank or any grant other than the owner relation (DIRECTORY-006 R1 and R2). Every check about a role version that this brief makes (assign a holding, make or edit a role, change a role's title or default policy, move a holder, renew a holding, change a holding's policy) SHALL be answered through one seam defined in crates/lys-identity/src/roles/check.rs, which answers the capacity an actor holds for a holding or a role: responsible_person when the actor is the person in the responsible field that DIRECTORY-006 carries and enforces on every grant, read, for a holding and for an assign alike, from the agent's registration record, which DIRECTORY-003 R1 writes naming the person responsible for the agent for life, the value every grant the agent holds carries in its responsible field (ADR-011, proposed, used as it stands), project_owner when the actor holds the owner relation on the role's project, and neither otherwise. This owner test is the one ownership test for assign, move, the policy change and the role editor. WHILE the evaluator crates/lys-identity-server/src/spicedb/check.rs does not exist on the main branch, the seam's one implementation SHALL answer from the holding record and, for the owner relation, from DIRECTORY-006 R4's current permission decision in crates/lys-identity/src/grants/permission.rs (the module grants::permission), and crates/lys-identity/src/roles/check.rs SHALL be the only file of crates/lys-identity/src/roles/ that names that module. WHEN the evaluator exists on the main branch, the identity server SHALL supply the seam with an implementation that asks it (R10), and no other file of this brief SHALL change for that swap. No other file of this brief SHALL decide such a permission itself. IF the seam cannot answer, THEN THE SYSTEM SHALL refuse naming the check and SHALL NOT permit, and SHALL NOT commit an event. Holds to DIRECTORY-002 R4, which is unchanged: nothing in crates/lys/src/identity/ asks SpiceDB.

**Acceptance:**
- ROLE_OWNER_GIVE: D1, holding no grant on P1, gives O1 the owner relation on P1 and it is admitted, with one grant event naming holder O1, responsible person O1 and authorising actor D1; O1 then gives O2 the owner relation on P1 and it is admitted; X1 gives X1 the owner relation on P1 and it is refused as not_permitted; D1 gives agent A1 the owner relation on P1 and it is refused as not_permitted. Four cases run; grant events added: 2.
- ROLE_OWNER_NOT_LABEL: X1, whose display name is Owner of P1 and who holds a use-only read grant on P1, is answered neither by the seam for role builder; O1 is answered project_owner; H1 is answered responsible_person for A1's holding and neither for role builder.
- ROLE_CHECK_SEAM: grep -rln 'grants::permission' crates/lys-identity/src/roles/ run from the repository root prints exactly one line, crates/lys-identity/src/roles/check.rs.
- ROLE_CHECK_UNAVAILABLE: with the permission decision made unavailable in the test, O1's move of A1 is refused as check_unavailable naming the move check, and the event count is the same before and after the call.

**Files:**
- create: crates/lys-identity/src/roles/check.rs
- create: crates/lys-identity/src/roles/ownership.rs
- create: crates/lys-identity/tests/roles_check.rs
- modify: crates/lys-identity/src/roles/mod.rs
- modify: crates/lys-identity/src/grants/types.rs
- modify: docs/design/identity/GRANT-CONTRACT.md

**Checklist:**
- C170 — Project ownership is the explicit owner relation on the project in the directory's authorization model, given only by the directory's configured administrator or an existing owner of that project, and never inferred from a name, label or rank.
- C171 — Every role-version check is answered through one server-side seam, SpiceDB's evaluator once it is on the main branch, and no request body supplies an actor, a capacity or a permission.

### R3: Assign a holding at the role's current version, its grants copied from that version's templates

WHEN an actor assigns a role to an agent, with an end date or without one, THE SYSTEM SHALL admit the assign only when R2's seam answers the actor responsible_person for that agent or project_owner for the role's project, and SHALL then make the holding in the role's project at the role's current version, make one grant from each of that version's templates, the template's fields with the agent as holder, the source DIRECTORY-006 R2 admits, and in the responsible field the agent's responsible person read from its DIRECTORY-003 R1 registration record, never the template's own responsible value, through DIRECTORY-006 R2's admission for that granting actor, write the holding's end date, when given, as the end of each of those grants' window, give a holding with an end date the role's default move policy and a holding with no end date the policy deliberate_only, and commit one holding granted event naming the actor, the capacity, the agent, the role, the version, the grant ids, the end date and the policy. A directory administrator SHALL be admitted only in one of those two capacities. IF the actor is in neither capacity, THEN THE SYSTEM SHALL refuse as not_permitted. IF DIRECTORY-006's admission refuses any template's grant for that actor, THEN THE SYSTEM SHALL refuse the whole assign naming each such template with the reason conformance row 2.4 names (use_only, above_held, lent_to_you, sign_in_identity). Each refusal SHALL NOT commit a holding, a grant or an event. The grants SHALL be copies made at grant time: THE SYSTEM SHALL NOT keep a link by which a later version or edit reaches a granted holding's grants, and SHALL NOT make a holding that does not name its version. Conformance 4.7, as ADR-089.

**Acceptance:**
- ROLE_ASSIGN (CONFORMANCE 4.7): H1 assigns builder to A1 at version 1; A1's holding reads back on version 1 with exactly two grants whose actions are read-P1 and test-P1, and the one holding granted event names actor H1, capacity responsible_person and version 1.
- ROLE_ASSIGN_OWNER: O1 assigns builder to A2 at version 1; A2's holding reads back on version 1 with exactly two grants, read-P1 and test-P1, and the one holding granted event names actor O1 and capacity project_owner.
- ROLE_ASSIGN_OTHER_RESPONSIBLE: O1 assigns builder to agent B1, whose responsible person is O2, at version 1, and it is admitted; B1's holding has exactly two grants, read-P1 and test-P1, and each names responsible O2 and neither names H1; R2's seam answers O2 responsible_person for B1's holding and answers H1 neither for it; with version 2 made, H1's confirmed move of B1 to version 2 is refused as not_permitted and 0 holding moved events are added.
- ROLE_ASSIGN_REFUSED: X1's assign of builder to A1 is refused as not_permitted; D1's assign of builder to A1 is refused as not_permitted while D1 holds no owner relation on P1; with H1's test-P1 changed to a use-only grant, H1's assign to A1 is refused naming test-P1 with reason use_only. Across the three calls holdings 0, grants 0 and events 0 are added.
- ROLE_ASSIGN_POLICY: while builder's default is move_at_next_renewal, A1 assigned with end date E has policy move_at_next_renewal, and A2 assigned with no end date has policy deliberate_only.
- ROLE_ASSIGN_END (CONFORMANCE 4.5): each of the two grants of A1's holding assigned with end date E carries end date E.

**Files:**
- create: crates/lys-identity/src/roles/holding.rs
- create: crates/lys-identity/tests/roles_assign.rs
- modify: crates/lys-identity/src/roles/mod.rs

**Checklist:**
- C172 — Assigning a role, by the person the agent answers to or an owner of the role's project, makes a holding that names its role and version with grants copied from that version's templates, each admitted under DIRECTORY-006 for the assigning actor (conformance 4.7, ADR-089).
- C179 — A provisional holding's end date is on its grants and is the same after a role edit and after a move (the roles half of conformance 4.5).

**Stories:**
- S98 (Owner of a project, Defines the roles of a project and assigns them) — As an owner of a project, I want to make and edit its roles and assign them to agents without any existing holder changing so that a role can improve without silently changing what its holders may do.

### R4: Make a role and a new version when its templates are edited, record a title edit, and move no holder

WHEN an actor makes a role in a project with a title and grant templates, THE SYSTEM SHALL admit it only when R2's seam answers the actor project_owner for that project, and SHALL commit version 1 as one role version made event naming the actor and the capacity, with the role's default move policy move_at_next_renewal. WHEN an actor changes a role's grant templates, THE SYSTEM SHALL admit it only when the seam answers the actor project_owner for the role's project, commit the next version with the new templates as one role version made event naming the actor and the capacity, and SHALL NOT change any holding's version, grants, end date or policy, nor any grant made from an earlier version. WHEN an actor so admitted changes a role's title, THE SYSTEM SHALL commit one role title changed event naming the actor and the capacity, and SHALL NOT make a version. Only the grant templates make a version. A directory administrator SHALL be admitted only as an owner of the role's project. IF the actor is not so admitted, THEN THE SYSTEM SHALL refuse as not_permitted and commit nothing. Conformance 4.2.

**Acceptance:**
- ROLE_EDIT (CONFORMANCE 4.2): builder at version 1 held by A1 and A2; O1's template edit replacing test-P1 with push-P1 makes version 2; A1 and A2 each read back on version 1, and the encoded bytes of each of their grants after the edit equal those before it.
- ROLE_EDIT_TITLE: O1 changing builder's title from Builder to Code builder commits one role title changed event naming actor O1 and capacity project_owner, and builder's last version is still 1.
- ROLE_EDIT_REFUSED: H1's template edit of builder is refused as not_permitted, and D1's is refused as not_permitted while D1 holds no owner relation on P1; builder's last version is still 1 and 0 events are added.
- ROLE_EDIT_NEW_DEFAULT: O1 makes role reviewer in P1 with one template; reviewer is on version 1 and its default move policy reads move_at_next_renewal.
- ROLE_EDIT_END (CONFORMANCE 4.5): A1 holding with end date E before the template edit has end date E after it, on the holding and on each of its grants.

**Files:**
- create: crates/lys-identity/src/roles/edit.rs
- create: crates/lys-identity/tests/roles_edit.rs
- modify: crates/lys-identity/src/roles/mod.rs

**Checklist:**
- C173 — Editing a role's grant templates makes the next version and leaves every holder on the version it held with its grants unchanged; editing its title is recorded and makes no version; a newly made role's default is move at the next renewal (conformance 4.2).
- C179 — A provisional holding's end date is on its grants and is the same after a role edit and after a move (the roles half of conformance 4.5).

**Stories:**
- S71 (Person a role holder answers to, Keeps each holding of a role on a version they chose) — As the person an agent answers to, I want an edit to its role to leave the agent on the version it holds so that its permissions never change without an act I can see.
- S98 (Owner of a project, Defines the roles of a project and assigns them) — As an owner of a project, I want to make and edit its roles and assign them to agents without any existing holder changing so that a role can improve without silently changing what its holders may do.

### R5: Answer who holds a role, and on which version, as a query over holdings

WHEN asked who holds a role, THE SYSTEM SHALL answer from the holding records, grouped by version, each holder with its version, end date and policy, marking the current version. THE SYSTEM SHALL NOT keep or answer from a member list stored on the role, and SHALL NOT list a lapsed holding as holding the role.

**Acceptance:**
- ROLE_HOLDERS: builder at version 2 current; A1 on version 1, A2 on version 2, and agent A3 on version 1 with an end date before the controlled clock's now: the answer is version 1: [A1], version 2: [A2], version 2 marked current, A3 absent.
- ROLE_HOLDERS_QUERY: after one holding granted event for agent A4 is committed, the next answer lists A4 with no other write to the store (one event committed between the two answers).

**Files:**
- create: crates/lys-identity/src/roles/holders.rs
- create: crates/lys-identity/tests/roles_holders.rs
- modify: crates/lys-identity/src/roles/mod.rs

**Checklist:**
- C174 — Who holds a role, and on which version, is answered as a query over holdings.

### R6: Move a holder to a newer version by a deliberate act that shows what changes first

WHEN an actor asks to move a holding to a newer version, THE SYSTEM SHALL answer a preview naming the grants the move adds and the grants it removes, from the difference between the held version's templates and the target version's, each shown in DIRECTORY-006's grant fields relation, resource, actions, pass_on, window and responsible, and SHALL commit nothing. WHEN the actor confirms that preview with a timing, next_start when none is given, or now, THE SYSTEM SHALL admit the move only when R2's seam answers the actor responsible_person for the holder or project_owner for the role's project, and SHALL commit one holding moved event naming the actor, the capacity, the from and to versions, the timing chosen and the grants added and removed; it SHALL make the added grants through DIRECTORY-006 R2's admission with the holding's end date and, as R3 copies them, the holder's responsible person from its DIRECTORY-003 R1 registration record in the responsible field, and withdraw the removed ones, subject to R7 for a running session. A directory administrator SHALL be admitted only in one of those two capacities. IF the target is not newer than the held version, THEN THE SYSTEM SHALL refuse as not_newer; IF the actor is in neither capacity, THEN THE SYSTEM SHALL refuse as not_permitted; each refusal commits nothing. THE SYSTEM SHALL NOT move a holding without a confirmed preview, and SHALL NOT change its end date or its policy. Conformance 4.3 and the roles half of 4.5.

**Acceptance:**
- ROLE_MOVE_PREVIEW (CONFORMANCE 4.3): A1 on version 1 of builder with end date E and version 2 made; the preview of A1 to version 2 answers added exactly one grant, relation pusher, resource project P1, actions [push], pass_on use_only, window ending E, responsible H1, and removed exactly one grant, relation tester, resource project P1, actions [test], pass_on use_only, window ending E, responsible H1; the event count is the same before and after it.
- ROLE_MOVE_RECORD (CONFORMANCE 4.3): H1 confirms that preview with no timing given; exactly one holding moved event is committed, naming actor H1, capacity responsible_person, from 1, to 2, timing next_start, added [push-P1] and removed [test-P1]; A1 reads back on version 2.
- ROLE_MOVE_CAPACITY: O1's confirmed move of A2 is committed with capacity project_owner; X1's is refused as not_permitted; D1's is refused as not_permitted while D1 holds no owner relation on P1, and is committed with capacity project_owner after D1 is given the owner relation on P1; holding moved events added across the four calls: 2.
- ROLE_MOVE_END (CONFORMANCE 4.5): A1 holding with end date E, moved from version 1 to version 2, has end date E on the holding and on each of its grants after the move.

**Files:**
- create: crates/lys-identity/src/roles/move_holder.rs
- create: crates/lys-identity/tests/roles_move.rs
- modify: crates/lys-identity/src/roles/mod.rs

**Checklist:**
- C175 — A move by hand shows what it adds and removes before it is taken, is admitted only for the person the holder answers to or an owner of the role's project, and is one signed record naming the actor, its capacity and the timing chosen (conformance 4.3).
- C179 — A provisional holding's end date is on its grants and is the same after a role edit and after a move (the roles half of conformance 4.5).

**Stories:**
- S72 (Mover of a holder, Moves a holder to a newer version of its role) — As the person a holder answers to or an owner of the project its role is defined in, I want to see what a move adds and removes, and choose when it takes effect, before I take it so that I move a holder knowing what it gains and loses.
- S75 (Reviewer of a role's history, Checks how each holder came to its version) — As a reviewer, I want every role act recorded with who made it and in which capacity so that I can verify how a holder came to the version it is on.

### R7: Keep a running session on the version it started with, and stop every open session first for a move now

WHILE a holder has a running session, WHEN a move of its holding is committed with timing next_start, THE SYSTEM SHALL answer that session's checks from the version the session started on until it ends, and SHALL answer every session the holder starts after the move from the new version. A running session SHALL NOT change version. WHEN a move is confirmed with timing now, THE SYSTEM SHALL take it only when the mover is the directory's configured administrator admitted as DIRECTORY-003 R3 admits it, the only party the stop requirement R6 of the session brief of brief run 8c1bee6c admits to a stop, and is also admitted for the move by R6 of this brief as responsible_person or project_owner; it SHALL then ask for the end of each open session of that agent through that R6 stop, one session id at a time, each stop recorded under the session brief with the operator as its actor, and after that commit the holding moved event, which names the mover's capacity for the move, timing now, and every session the listing showed, each with its outcome. The stop either commits one session_ended event or refuses, and the move acts on each outcome as follows. WHEN a stop commits its session_ended event, THE SYSTEM SHALL list that session as stopped and name it in the holding moved event with the outcome stopped. WHEN the append of a stop's session_ended event returns an uncertain outcome, THE SYSTEM SHALL list that session as unconfirmed and name it in the holding moved event with the outcome unconfirmed; under P5 that outcome is reconciled before the session's projection answers as ended, so the session keeps the version it started with until the reconciliation finds its session_ended event committed, and the listing then reads stopped. WHEN a stop is refused as session_ended because the session ended between the listing and the stop, THE SYSTEM SHALL list that session as already_ended, SHALL name it in the holding moved event with the outcome already_ended and the session_ended refusal it received, SHALL NOT list it as stopped nor record a stop act for it, and SHALL go on to the next session; a session the listing showed SHALL NOT be left off the answer or off the record; the stop's other refusals do not arise for a listed session, since the mover's admission is checked before the first stop and each listed session id names a session. The holding moved event SHALL be committed once a stop has been asked for every listed session, so the holding reads the new version from that commit. IF the mover is not that administrator, THEN THE SYSTEM SHALL refuse the now leg as now_not_operator, SHALL NOT ask for any stop and SHALL NOT commit the move, and the mover may take the move with timing next_start instead. THE SYSTEM SHALL NOT widen the stop's admission. The next-start leg takes effect when a session starts, which is code in crates/lys-identity/src/session/start.rs that the start requirement R4 of the session brief of brief run 8c1bee6c creates, so the next-start leg depends on that file; the now leg asks for the stop defined in crates/lys-identity/src/session/end.rs that its stop requirement R6 creates; each leg dispatches only when its artifact named in blocked_by exists on the main branch.

**Acceptance:**
- ROLE_MOVE_NEXT_START: A1's session S1 starts on version 1; H1 moves A1 to version 2 with timing next_start; a check for S1 on test-P1 permits; A1's session S2, started after the move, is refused on test-P1 and permitted on push-P1.
- ROLE_MOVE_NOW: D1, given the owner relation on P1, moves A1 to version 2 with timing now while A1's sessions S1 and S2 are open; the log holds two session_ended events with how it ended operator_stopped and actor D1, for S1 and for S2, both before one holding moved event naming actor D1, capacity project_owner, timing now and sessions [S1 stopped, S2 stopped]; the answer lists S1 stopped and S2 stopped.
- ROLE_MOVE_NOW_UNCONFIRMED: D1, given the owner relation on P1, moves A1 to version 2 with timing now while A1's sessions S1 and S2 are open, with the test store answering the append of S2's session_ended event as an uncertain outcome; the answer lists S1 stopped and S2 unconfirmed; the log holds exactly one committed session_ended event, for S1 with actor D1, and exactly one holding moved event naming actor D1, capacity project_owner, timing now and sessions [S1 stopped, S2 unconfirmed]; A1's holding reads back on version 2; a check for S2 on test-P1 permits. After the test store reconciles that append as committed, the log holds one session_ended event for S2 with actor D1, S2 reads ended, and the answer lists S2 stopped.
- ROLE_MOVE_NOW_ALREADY_ENDED: D1, given the owner relation on P1, moves A1 to version 2 with timing now while A1's sessions S1 and S2 are listed open, and A1 reports S2's end with its credential after the listing and before S2's stop is asked for, so that stop is refused as session_ended naming S2; the move is committed: the log holds exactly one session_ended event with actor D1, for S1, exactly one session_ended event for S2, with actor A1 and how it ended agent_reported, and exactly one holding moved event naming actor D1, capacity project_owner, timing now and sessions [S1 stopped, S2 already_ended with refusal session_ended]; the answer lists S1 stopped and S2 already_ended; the log holds 0 session_ended events for S2 with actor D1; A1's holding reads back on version 2.
- ROLE_MOVE_NOW_REFUSED: O1's move of A1 with timing now is refused as now_not_operator with 0 session_ended and 0 holding moved events added, and S1 still reads open; O1's move of A1 with timing next_start is then committed.

**Files:**
- create: crates/lys-identity/src/roles/timing.rs
- create: crates/lys-identity/tests/roles_move_timing.rs
- modify: crates/lys-identity/src/roles/mod.rs

**Checklist:**
- C176 — A move at the holder's next start leaves a running session on the version it started with, and a move now, taken only by the admitted operator, stops every open session of the agent first and names in the move's record every session its listing showed, each with its outcome.

**Stories:**
- S72 (Mover of a holder, Moves a holder to a newer version of its role) — As the person a holder answers to or an owner of the project its role is defined in, I want to see what a move adds and removes, and choose when it takes effect, before I take it so that I move a holder knowing what it gains and loses.

### R8: Renew a holding as a new recorded grant, at the version its policy names

This brief introduces renewal; nothing on the main branch defines it. A renewal is a new grant of the holding with a new end date, made by the person the holder answers to, the responsible person of ADR-011, or by an owner of the role's project, and recorded as a renewal. WHEN such an actor, admitted by R2's seam as responsible_person or project_owner, renews a holding at or before its end date, giving a new end date, THE SYSTEM SHALL make the holding's grants anew through DIRECTORY-006 R2's admission for that actor, each with the new end date and, as R3 copies them, the holder's responsible person from its DIRECTORY-003 R1 registration record in the responsible field, from the role's current version's templates when the holding's policy is move_at_next_renewal and from the held version's templates when it is deliberate_only, withdraw the grants it replaces, and commit one holding renewed event naming the actor, the capacity, the holding it renews, the version it lands on and the new end date. A directory administrator SHALL be admitted only in one of those two capacities. IF the actor is in neither capacity, THEN THE SYSTEM SHALL refuse as ROLE_RENEW_REFUSED; IF the holding has no end date, THEN as no_end_date; IF the renewal is made after its end date, THEN as holding_lapsed; each refusal commits nothing. A renewal made at exactly the end date is admitted: it is made before the end date has passed, and its new grants are admitted by DIRECTORY-006 R2 with the new end date, while DIRECTORY-006 R4 still refuses exercise of the replaced grants from their end date. Under deliberate_only, a move to a newer version still needs R6's deliberate act. Nothing but this act SHALL renew a holding: no clock, role edit, move, policy change or retry renews one, and a holding is never renewed quietly. IF nobody renews a holding, THEN it lapses at its end date under ADR-006 and DIRECTORY-006 R4, the neighbour that owns the directory half of conformance 4.5, and THE SYSTEM SHALL NOT move it.

**Acceptance:**
- ROLE_RENEW (CONFORMANCE 4.4): A1 on version 1 under move_at_next_renewal with end date E; version 2 made at T0 + 1 day; at E - 2 hours A1 reads back on version 1; at E - 1 hour H1 renews A1 with end date E + 30 days; one holding renewed event names actor H1, capacity responsible_person, A1's holding, version 2 and E + 30 days, and A1 reads back on version 2 with the grants read-P1 and push-P1 each ending E + 30 days.
- ROLE_RENEW_NONE (CONFORMANCE 4.4): the same holding with no renewal reads back on version 1 at E - 1 second; at E + 1 second it is lapsed as DIRECTORY-006 R4 answers, its version is still 1, and the log holds 0 holding moved and 0 holding renewed events for it.
- ROLE_RENEW_AT_END (CONFORMANCE 4.4): A1 on version 1 under move_at_next_renewal with end date E and version 2 made; with the controlled clock at exactly E, H1 renews A1 with end date E + 30 days; it is admitted, one holding renewed event names actor H1, A1's holding and version 2, and A1 reads back on version 2 with the grants read-P1 and push-P1 each ending E + 30 days.
- ROLE_RENEW_OWNER: A1 as in ROLE_RENEW; at E - 1 hour O1 renews A1 with end date E + 30 days; one holding renewed event names actor O1, capacity project_owner, A1's holding, version 2 and E + 30 days.
- ROLE_RENEW_DELIBERATE: A1 on version 1 under deliberate_only with end date E and version 2 made; at E - 1 hour H1 renews A1 with end date E + 30 days; one holding renewed event names version 1, and A1 reads back on version 1 with the grants read-P1 and test-P1 each ending E + 30 days.
- ROLE_RENEW_REFUSED: X1's renewal of A1 is refused as ROLE_RENEW_REFUSED, H1's renewal at E + 1 second as holding_lapsed, and H1's renewal of A2 (no end date) as no_end_date; 0 events added across the three.

**Files:**
- create: crates/lys-identity/src/roles/renewal.rs
- create: crates/lys-identity/tests/roles_renewal.rs
- modify: crates/lys-identity/src/roles/mod.rs

**Checklist:**
- C177 — A renewal is a new grant of the holding with a new end date by the person the holder answers to or an owner of the role's project, at the role's current version under a move at the next renewal and at the held version otherwise, recorded naming the actor, the holding and the version; an unrenewed holding lapses and does not move (conformance 4.4).
- C179 — A provisional holding's end date is on its grants and is the same after a role edit and after a move (the roles half of conformance 4.5).

**Stories:**
- S74 (Person a role holder answers to, Keeps each holding of a role on a version they chose) — As the person a provisional holder answers to or an owner of its role's project, I want renewing its holding to be a recorded act of one of us so that it is never renewed quietly and lapses when nobody renews it.
- S75 (Reviewer of a role's history, Checks how each holder came to its version) — As a reviewer, I want every role act recorded with who made it and in which capacity so that I can verify how a holder came to the version it is on.

### R9: Show each holding's policy, move date and who can stop the move, and change a holding's policy or a role's default by a recorded act

THE SYSTEM SHALL answer, for each holding, its policy and what it means: for a holding with an end date under move_at_next_renewal, its end date as the date it will move, that its next renewal moves it to the current version, and who can stop the move, which is the person the holder answers to and each holder of the owner relation on the role's project; for a holding under deliberate_only, and for every holding with no end date whatever the role's default, that it moves only by a deliberate act, and no date. WHEN an actor whom R2's seam answers responsible_person for the holder or project_owner for the role's project changes one holding's policy, THE SYSTEM SHALL commit one holding policy changed event naming the actor, the capacity, the holding, and the policy before and after, and SHALL NOT change any other holding, nor the holding's version, grants or end date. IF the actor is in neither capacity, THEN THE SYSTEM SHALL refuse as not_permitted; IF the policy asked for is move_at_next_renewal on a holding with no end date, THEN as no_end_date; each refusal commits nothing. WHEN an actor whom the seam answers project_owner for the role's project changes the role's default policy, THE SYSTEM SHALL commit one role default policy changed event naming the actor and the capacity, apply the new default only to holdings granted after it, and SHALL NOT change any existing holding. A directory administrator SHALL be admitted only in one of those capacities. The answer for a role SHALL name its default and mark each holding with an end date whose own policy differs from it. A holding with no end date SHALL NOT be marked as differing, whatever the role's default, since it has no renewal to move at, and SHALL show only the words Moves only by a deliberate act. Conformance 4.4, as ADR-077.

**Acceptance:**
- ROLE_POLICY_SHOWN (CONFORMANCE 4.4): A1 under move_at_next_renewal with end date E answers move date E, the words Its next renewal moves it to the current version, and who can stop it [H1, O1]; A2 with no end date answers policy deliberate_only, the words Moves only by a deliberate act, and move date none.
- ROLE_POLICY_CHANGE: H1 changes A1's policy to deliberate_only; one holding policy changed event names actor H1, capacity responsible_person, A1's holding, before move_at_next_renewal and after deliberate_only; A2's policy, A1's version, A1's end date and the encoded bytes of A1's grants are the same as before.
- ROLE_POLICY_CHANGE_OWNER: O1 changes A1's policy back to move_at_next_renewal; one holding policy changed event names actor O1 and capacity project_owner. X1's change of A1's policy is refused as not_permitted with 0 events added.
- ROLE_POLICY_NO_END: H1 setting move_at_next_renewal on A2 (no end date) is refused as no_end_date with 0 events added.
- ROLE_DEFAULT: O1 changes builder's default from move_at_next_renewal to deliberate_only; one role default policy changed event names actor O1; A1 keeps move_at_next_renewal and is marked as differing from the default; agent A4 assigned afterwards with end date E has deliberate_only.
- ROLE_DEFAULT_NO_END: with builder's default move_at_next_renewal, A1 with end date E changed to deliberate_only is marked as differing from the default, and A2 with no end date is not marked and answers only the words Moves only by a deliberate act.

**Files:**
- create: crates/lys-identity/src/roles/policy.rs
- create: crates/lys-identity/tests/roles_policy.rs
- modify: crates/lys-identity/src/roles/mod.rs

**Checklist:**
- C178 — Each holding shows its policy; one with an end date under a move at the next renewal shows its end date, that its next renewal moves it to the current version, and who can stop the move; one with no end date shows it moves only by a deliberate act; a holding's policy and a role's default change only by recorded acts that move no holder (conformance 4.4, ADR-077).

**Stories:**
- S73 (Person a role holder answers to, Keeps each holding of a role on a version they chose) — As the person a holder answers to or an owner of its role's project, I want to see the date each holding will move and who can stop it, and to change one holding's policy, so that no move surprises me.
- S75 (Reviewer of a role's history, Checks how each holder came to its version) — As a reviewer, I want every role act recorded with who made it and in which capacity so that I can verify how a holder came to the version it is on.
- S98 (Owner of a project, Defines the roles of a project and assigns them) — As an owner of a project, I want to make and edit its roles and assign them to agents without any existing holder changing so that a role can improve without silently changing what its holders may do.

### R10: Expose the role acts through the identity server, which alone decides them, and carry the owner relation into SpiceDB

The identity server SHALL expose R3, R4 and R6 to R9's acts, R5's answer, R6's preview and R9's answer as authenticated routes in crates/lys-identity-server/src/roles.rs, taking the actor only from the authenticated session (P8), and answering, for the viewer, which of assign, move, move now, renew, change policy, edit the role and change its default it is permitted, as R2's seam answers. Every act SHALL be admitted or refused by R2's seam on the server whatever the client presents; the server SHALL NOT take an actor, a capacity or a permission from a request body. WHEN the evaluator crates/lys-identity-server/src/spicedb/check.rs exists on the main branch, the server SHALL supply R2's seam with an implementation that asks it, and that swap SHALL be confined to crates/lys-identity-server/src/roles.rs and its test. WHEN crates/lys-identity-server/src/spicedb/schema.zed and crates/lys-identity/src/grants/relationships.rs exist on the main branch, this row SHALL add to schema.zed's project definition the relation owner, whose subject is a person, and to relationships.rs the mapping from a committed owner-relation grant event to exactly one relationship, project P owner person O, and from its committed revoke to that relationship's deletion; THE SYSTEM SHALL NOT write the owner relationship by any path other than the projector of the SpiceDB enforcement brief DIRECTORY-020 R2, and SHALL NOT give an agent the owner relation in the schema. The decision that brings that evaluator in is ADR-062 on draft/directory/09a6cc80, proposed, which lands with the SpiceDB enforcement brief DIRECTORY-020; this brief does not bring enforcement in itself.

**Acceptance:**
- ROLE_ROUTES_AUTHORITY: X1's session posts a move of A1 whose body names actor H1 and capacity responsible_person; the answer is not_permitted and 0 events are added.
- ROLE_ROUTES_PERMITTED: the holders answer lists A1 with move, renew and change policy permitted for H1 and edit the role not permitted; move, renew, change policy and edit the role permitted for O1; none of the four permitted for X1; assign permitted for H1 on A1 and for O1, and not for X1.
- ROLE_OWNER_PROJECTION: the committed grant event in which D1 gives O1 the owner relation on P1 maps through relationships.rs to exactly one relationship, project P1 owner person O1, and its committed revoke maps to exactly one deletion of that relationship; two cases run, two outcomes asserted.
- ROLE_ROUTES_SPICEDB: once crates/lys-identity-server/src/spicedb/check.rs, schema.zed and crates/lys-identity/src/grants/relationships.rs exist on the main branch, O1's move of A1 is admitted after D1's owner-relation grant event for O1 on P1 is committed and projected into the disposable SpiceDB, and refused as not_permitted after that grant is revoked through the signed event path and the revoke is projected, with the holding records unchanged between the two calls.

**Files:**
- create: crates/lys-identity-server/src/roles.rs
- create: crates/lys-identity-server/tests/roles.rs
- create: crates/lys-identity/tests/roles_owner_relationship.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/spicedb/schema.zed
- modify: crates/lys-identity/src/grants/relationships.rs

**Checklist:**
- C170 — Project ownership is the explicit owner relation on the project in the directory's authorization model, given only by the directory's configured administrator or an existing owner of that project, and never inferred from a name, label or rank.
- C171 — Every role-version check is answered through one server-side seam, SpiceDB's evaluator once it is on the main branch, and no request body supplies an actor, a capacity or a permission.

### R11: Show versions, holders, move dates and the move preview on the role screen from the server's answers

The role screen SHALL show the role's versions with the current one marked, its default policy under When this role changes, the holders on each version, and a mark on each holding with an end date whose policy differs from the default, with no mark on a holding with no end date. Each holder SHALL show its version, its policy and, for a holding with an end date under move_at_next_renewal, its end date with the words Its next renewal moves it to the current version and who can stop the move; for a holding under deliberate_only or with no end date, the words Moves only by a deliberate act. The move drawer SHALL show the server's preview of what the move adds and removes and a timing choice of At its next start, preselected, and Now: ends its running session first, and after a move now SHALL list every session the server's answer names, each with the state that answer gives it, stopped, unconfirmed or already_ended, and SHALL NOT show a session as stopped unless the answer says stopped nor drop a session the answer names. The role builder SHALL offer the title and the grant templates as its editable fields. The Assign action SHALL be shown only to the person the holder answers to, for their own agents, and to the owners of the role's project. The screen SHALL offer an act only where the server's answer says the viewer is permitted, SHALL NOT decide a permission itself, and SHALL follow ADR-010.

**Acceptance:**
- ROLE_SCREEN_HOLDERS (CONFORMANCE 4.4): with A1 under move_at_next_renewal ending E and A2 with no end date, A1's holder row shows E, the words Its next renewal moves it to the current version, and Who can stop it: H1, O1; A2's row shows the words Moves only by a deliberate act and no date.
- ROLE_SCREEN_MOVE (CONFORMANCE 4.3): opening the move of A1 to version 2 shows Adds: push-P1 and Removes: test-P1 from the preview response, with the timing select on At its next start; confirming sends one request and A1 then shows version 2.
- ROLE_SCREEN_MOVE_NOW: for D1, given the owner relation on P1, confirming the move of A1 to version 2 with Now: ends its running session first, with the server answering sessions [S1 stopped, S2 unconfirmed], shows exactly two session rows, S1 with the word stopped and S2 with the word unconfirmed; with the server answering [S1 stopped, S2 already_ended], it shows exactly two rows, S1 stopped and S2 already ended, and no row reads stopped for S2.
- ROLE_SCREEN_ASSIGN: with agents A1, A2, A3 and A4 answering to H1, one more agent B1 answering to O2, and no agent holding builder, on builder's screen the Assign action is rendered for O1; it is rendered for H1 with exactly A1, A2, A3 and A4 offered as holders and B1 not offered; and it is not rendered for X1, nor for D1 while D1 holds no owner relation on P1.
- ROLE_SCREEN_NOT_AUTHORITY: for X1 no move, renew or policy control is rendered; with the page's permission flags altered in the browser test to render the move control for X1, the request it sends is refused as not_permitted and A1 still shows version 1.
- ROLE_SCREEN_BUILDER (CONFORMANCE 4.2): for O1, saving a template change shows version 2 current with A1 still on version 1; saving a title change leaves version 2 current and makes no version 3.

**Files:**
- create: surface/identity/src/features/roles/RoleVersions.tsx
- create: surface/identity/src/features/roles/RoleHolders.tsx
- create: surface/identity/src/features/roles/MoveHolder.tsx
- create: surface/identity/src/features/roles/HoldingPolicy.tsx
- create: surface/identity/src/features/roles/RoleBuilder.tsx
- create: surface/identity/src/features/roles/AssignRole.tsx
- create: surface/identity/tests/roles.test.tsx
- create: surface/identity/tests/acceptance/roles.spec.ts
- modify: surface/identity/src/routes.tsx
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C180 — The role screen shows versions, holders on each version, move dates, policies and the move preview from the server's answers, shows Assign only to the person the holder answers to and the owners of the role's project, and offers an act only where the server says it is permitted.

**Stories:**
- S72 (Mover of a holder, Moves a holder to a newer version of its role) — As the person a holder answers to or an owner of the project its role is defined in, I want to see what a move adds and removes, and choose when it takes effect, before I take it so that I move a holder knowing what it gains and loses.
- S73 (Person a role holder answers to, Keeps each holding of a role on a version they chose) — As the person a holder answers to or an owner of its role's project, I want to see the date each holding will move and who can stop it, and to change one holding's policy, so that no move surprises me.

## Boundaries

- Do not build the Confirm act that makes a provisional holding permanent: it is a new grant without an end date and belongs with the grant rows.
- Do not add conformance 4.1's job or profile to a role; they stay proposed and outside. Only the versioned grant templates make a version, and a title edit is recorded and makes no version.
- Do not cover people holding roles: this brief makes holdings for agents only, and never lets a holder renew, move or change the policy of its own holding.
- Do not restate DIRECTORY-006 R4's expiry enforcement; the directory half of conformance 4.5, that the grants a lapsed holding carried stop at the next check, is that row's and is its neighbour here.
- The directory design's non-goal sentence on road step 2 onward stands unamended and does not exclude this brief, since it describes what step 1 set out to build and DIRECTORY-006 already sits beside it making grants; its amendment is a documents card for the design's owner, and no hand-written section of DESIGN.md changes: the re-rendered DESIGN.md gains only the two decision ids and the structure rows, which are the method's listing of this brief's files.
- The directory design's non-goal that records the grant representation as open for the owner stands unamended and recorded as open; the templates are written in DIRECTORY-006's grant fields as built, and if a ratification renames or drops any of those fields, the templates follow it by amendment of this brief.
- This brief rests on ADR-011 as it stands, which is proposed: if ADR-011 changes who may act on a holding, this brief is amended to follow.
- The reviewed mock-up draws the role's When this role changes with the initial value stay; that is a drawing default and does not govern, since a newly made role's default is move at the next renewal, and the mock-up's owner brings the drawing into line.
- The reviewed mock-up's role builder says draft vN+1 when you change anything; that differs from this brief, where only a change to the grant templates makes a version and a title change makes none, and the mock-up's owner brings the drawing into line.
- Do not bring SpiceDB enforcement in: the evaluator is the SpiceDB enforcement brief DIRECTORY-020's (ADR-062 on draft/directory/09a6cc80, proposed), and DIRECTORY-002 R4 stays as it is.
- Do not widen the session stop's admission: the now leg is taken only by the operator DIRECTORY-003 R3 admits.
- No holder changes version and no end date moves as a side effect of any edit, default change, policy change or retry; nothing renews a holding but R8's act.
- The browser is never the authority: no screen decides a permission, and no route takes an actor, capacity or permission from a request body.
- Do not change a published lys-core wire format; the event kinds are drafted in the envelope document and reviewed before any is durably signed.
- Do not edit docs/design/identity/CONFORMANCE.md, the mock-up, or any other brief's requirements.

## Verification

- sh scripts/design/gate.sh exits 0 from the repository root.
- git cat-file -e origin/main:crates/lys-identity/src/grants/types.rs, git cat-file -e origin/main:crates/lys-identity/src/grants/permission.rs and git cat-file -e origin/main:docs/design/identity/GRANT-CONTRACT.md are run before any requirement is dispatched; git cat-file -e origin/main:surface/identity/src/routes.tsx and git cat-file -e origin/main:surface/identity/src/generated/index.ts are run before R11 is dispatched; git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/check.rs is run before R2's SpiceDB leg, R10's seam swap and ROLE_ROUTES_SPICEDB are dispatched, git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/schema.zed and git cat-file -e origin/main:crates/lys-identity/src/grants/relationships.rs before R10's owner relation in SpiceDB, git cat-file -e origin/main:crates/lys-identity/src/session/start.rs before R7's next-start leg, and git cat-file -e origin/main:crates/lys-identity/src/session/end.rs before R7's now leg; each result is recorded.
- The two reading commands in blocked_by are run against draft/directory/8c1bee6c-7421-4bf2-9753-922ad3546590 before R7 is dispatched; they print one line beginning R4 for session/start.rs and one line beginning R6 for session/end.rs, and each result is recorded.
- At implementation time the repository's gate legs run at the venue from the exact revision: cargo fmt --all, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps; the reviewer counts each acceptance line's test as run, and a test that ran zero cases is a failure.
- The reviewer finds at least one acceptance line naming each of CONFORMANCE 4.2, 4.3, 4.4 and 4.5, and reruns each.

