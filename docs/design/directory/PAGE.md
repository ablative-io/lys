# directory — what was asked, what it means, and what was written

## The words, as they were typed

The brief for this card already stands on main: docs/design/identity/briefs/IDENTITY-001.md and .json, revision 5, section "Row 07 — Gate, install and demonstrate the exact release". Read it in full (Authority, Ceiling, Design choices, Required venue checks, then Row 07 with its file wall, work and acceptance) and carry it as this card's brief, in the identity cluster. Keep its work and acceptance exactly as written: every required venue leg green on the exact pushed Lys, Rauthy and Cambium refs; the row 03 and row 05 live install receipts verified; standalone acceptance recorded before Cambium cutover; backups and a rollback that respects migrated schemas; every command, result, ref and hash recorded in reports/IDENTITY-001-release.md and IDENTITY-001-commands.jsonl; every requirement not met is named; and health alone is never completion.

Keep its order: Row 07 depends on row 06 and IDENTITY-002, and on rows 01 to 06 being done with their live demonstrations at the ends of rows 03 and 05. The brief says so as a blocker, and the build does not start until those have landed. Keep its file wall, including its one cambium file, docs/design/identity/IDENTITY-001-install.md. Where that row's rows have since moved into the directory cluster (DIRECTORY-002 to DIRECTORY-005 carry rows 02, 04, 03 and 05), name the directory brief that now owns each row the release proof checks. No production key, no receipt outside a test, and no production anchor: those acts stay Tom's. Keep to the method (scripts/design/validate.py, check-coverage.py and render-cluster.py, run by scripts/design/gate.sh). If the survey finds a sentence of Row 07 open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the row around it. The card is built from this brief only after Tom or the lead signs it off on the card.

Rulings of the lead, Archie, on 27 September 2026 at 08:33, given to the first run 19c38e64 in answer to its seven survey questions. That run took all seven and then failed with ClusterRefused, because docs/design/identity/briefs holds CONTEXT and IDENTITY briefs. They are settled here, and the author reopens none of them. R1: where these words say the identity cluster, the brief lives in the directory cluster, which scripts/design/gate.sh measures, under the next free DIRECTORY id after main's highest and every open brief/* and draft/* branch's, checked with git ls-remote just before writing. The docs/design/identity folder stays as it is, unregistered in roadmap.json, and IDENTITY-001.json and CONTEXT-001.json are neither moved nor excluded. R2: the wall's gates.json means .land/gates.sh, mirrored in CLAUDE.md's gate list, and docs/design/project.json. Each row registers the leg it needs in its own brief; the Rauthy container leg is DIRECTORY-002's identity leg, and this card registers only the release leg it adds. A leg that DIRECTORY-004 or 005 needs and nothing registers first is named as a finding for that brief's owner, and those briefs are not edited here. R3: IDENTITY-001.json and .md are never changed, as the directory design's "Read, never changed" says. Status from evidence is recorded only in this brief and the release report, and both files leave the wall. R4: this lys card does not land docs/design/identity/IDENTITY-001-install.md. That file lands through the Cambium identity cluster, ruled at 08:30 for card v0CuMstE, as its own Cambium card with its own PR and gate. This card names it as a dependency and leaves it off its wall. R5: row 06 is already carded in two halves, the lys half LsCN9H7- (brief run c7d95bf6) and the Cambium half v0CuMstE, which owns ID001_CAMBIUM_LINK, ID001_GOOGLE_SEPARATE and ID001_PKCE, with S256 ruled in scope. Both are dependencies, each checked by its landed brief file, and this card opens neither. R6: row 07 stops at a staged install on the node the operator names, keyed with a test service key. It proves standalone acceptance there and records it before any cutover, and it names the production signing key, production receipts and the Cambium cutover as Tom's three acts. The operator is shown a staging install, labelled as such. R7: "no receipt outside a test" means no signed Lys receipt under a production key. Verifying the row 03 and 05 install posts is allowed, and so is inspecting the agent's recorded creation on the staged install, whose receipt is test-keyed. The release report marks each such receipt as test-keyed.

Rulings of the lead, Archie, given on 27 September 2026 to the run dc4ff2c9-8791-4de3-afe4-425d614efe34 in answer to its round 1. That run took every answer and then failed before writing, when its writer found saved state for its session already standing under another config folder. They are settled here, and the author reopens none of them.

Preserved Cambium identities are recorded as a named unmet requirement until the cutover. Row 07 stops before any cutover, as R6 rules, so no staging Cambium is stood up by this row. The release report lists 'preserved Cambium identities are observed' as not met, naming the cutover as the act that meets it and Tom as the one who performs it. The coordination with Gypsy in Work bullet 3 is recorded as the handover of the staged issuer's address and the install receipts, not as an install. Health of the staged issuer is never counted toward this line.

The staging label is given in the demonstration and the release report only. No file under surface/identity or crates/lys-identity-server changes, so the row stays inside its wall. The demonstration's opening states that the operator is looking at a staging install, and the release report records that it was said. A visible staging mark on the screens is a later card on the directory board, and the brief names it under further units not written.

The install document is a separate Cambium card, not v0CuMstE. The 08:30 ruling placed the install document in Cambium's identity cluster, and R5 keeps v0CuMstE as row 06's Cambium half, so the install document gets a card of its own on the Cambium board with its own PR and gate. The brief's blocked_by names that card as the Cambium install-document card, to be made by Cambium's lead, and states that Cambium has no docs/design/identity folder yet, so that card creates it.

Rulings of the lead, Archie, given on 27 September 2026 to the run a3c1d516-d589-4177-a1fb-34169fb53cbd in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

The path that lands. The blocker names the Cambium install-document card, RM-010 on draft/identity/debf7614, and checks the document at the path that card lands, docs/identity-install.md as its draft writes, read from Cambium main once it has landed. The words' docs/design/identity/IDENTITY-001-install.md is superseded, and the brief says so, so the blocker can clear. If that card lands the document at another path, the blocker reads that path and the dev record names it. Answered by Archie, lead for the identity line.

A demand-cadence leg in docs/design/project.json only. It is not added to .land/gates.sh, so no other lys card's land runs it or goes red for want of Rauthy and Cambium refs and a staged install. CLAUDE.md's gate list names it as a demand leg, run by the release card and not by every land, in one line. Answered by Archie.

Yes. The missing identity leg of DIRECTORY-002 is named as a finding for its owner, as DIRECTORY-004 and 005 are, and held as a blocker of this card. The release proof's 'every required venue leg green' checks against the legs the tree actually registers, and this card does not register DIRECTORY-002's leg for it. The lead cards that leg for DIRECTORY-002's line. Answered by Archie.

Lift it, with a decision. This card records the cross-repository arrangement for row 07 as a decision at the next free ADR id, read immediately before writing, and amends the non-goal so it no longer covers row 07, naming that decision. Row 06 is draft c7d95bf6's. Whichever of the two lands second reads the non-goal from main as the first left it and changes only its own row's words, so neither undoes the other. Answered by Archie.

Edit it only to take row 07 out, and rewrite nothing else in it. The sentence names row 06 alone, for example '...row 06 stays the non-goal recorded below, and this sentence does not promise it.', so that it is grammatical and true once row 07 is lifted. DIRECTORY-008's recorded endswith check is that card's own evidence of what it landed, and it is not a live gate, so it is not kept true by leaving a false sentence. The dev record names the check, says it no longer holds after this card, and gives the ruling as the reason. Draft c7d95bf6 owns row 06's words, so whichever of the two lands second reads the sentence from main and changes only its own row. Answered by Archie, lead for the identity line.

Accepted. Make every correction the round proposes, as Q10 to Q20 word it, and settle each finding by that change. For Q12, move receipt recording ahead of the handover, so that no requirement depends on a later one. For Q17, fix a 'kind' per jsonl line and classify test commands by it. For Q20, add the service stop and start and the restore from the recorded backup to R3, each recorded as a jsonl line with a fixed report line and measured by an acceptance line. Nothing here goes to Tom. Answered by Archie, lead for the identity line.

Met by r4-backup-before-migration-unmeasured as ruled there. Answered by Archie.

Met by r6-not-met-act-performer-unmeasured and r6-status-format-unmeasured as ruled there: the spec fixes both line formats and the completion line. Answered by Archie.

Met by the same two rulings, each with its acceptance line. Answered by Archie.

Accepted as proposed. git merge-base --is-ancestor <base> <lys commit> must exit 0, where <base> is the commit the build started from, which holds the landed blockers. It is recorded as a jsonl line of kind ancestry, with a '## Refs' line, and one acceptance line checks it. A lys commit from before DIRECTORY-002 to 005 landed fails. Answered by Archie.

Accepted as proposed. Each backup line under '## Install' cites its commands.jsonl:<n>. An acceptance line requires every jsonl line of kind backup to come before the first line of kind install that starts a Rauthy or lys binary against the database. Answered by Archie.

Accepted as proposed, both checks. No line of '## Standalone acceptance' cites a commands.jsonl line whose command contains 'health', and every 'screen:' line's observed text names a screen action, not a command. Answered by Archie.

Accepted as proposed. The text after 'staging statement: ' must contain 'staging install', and it is measured. Answered by Archie.

Measure it, and don't drop it. Record 'configuration: <file> sha256 <64-hex>' for each configuration file of the staged install. An acceptance line checks that no recorded file names a Cambium or Manifold address or host. Answered by Archie.

Accepted as proposed. Each '## Not met' line has the fixed form '<requirement> — met by: <act> — performed by: <role>', and every line is measured for both. The line for 'preserved Cambium identities are observed' names the product owner as performer, and that is measured. Answered by Archie.

Accepted as proposed. '## Status' holds exactly one 'met: <line>' or 'not met: <line>' per acceptance line of revision 5's Row 07, counted as exactly that many. The completion line is the literal 'row 07: complete', present only when every one is met, and checked against that fixed form. Answered by Archie.

Accepted as proposed. For each of ID001_LINK_LIVE and ID001_DIRECTORY_LIVE with no receipt line, or with an 'is-ancestor: ' token not followed by 0, '## Not met' holds a line naming that identifier, and an acceptance line measures it. Drop the 'verified' marker: a receipt is either on the receipt line with its ancestry exit 0, or it is under '## Not met'. Answered by Archie.

Accepted as proposed. tool:rustfmt and tool:cargo go into the identity-release leg's requires and into the acceptance's expected list. Answered by Archie.

Accepted as written: R1 also modifies docs/design/directory/design.json and DESIGN.md, appends the same leg to the design gate, and gains the acceptance line that design.json gate equals project.json trees. Ruled by Archie, lead for the identity line.

Accepted as written: failing test names of both runs are recorded in a fixed form, every failing test named for line <n> must be named for its baseline line <m>, and line <m>'s ref must be the upstream release commit IDENTITY-002's rebase names. Ruled by Archie.

Accepted as written: an acceptance line that every jsonl line of kind gate has its commands.jsonl:<n> line under ## Venue legs. Ruled by Archie.

Accepted. The trigger reads the kinds gate, test and R2's own two ancestry lines. An acceptance line asserts that every install line comes after every line of those kinds, and that each of those has exit_status 0 or is a Rauthy line cited on an upstream: commands.jsonl:<n> line. Ruled by Archie.

Accepted as written: a backup of the migrated staged install, cited by its own backup: line, is what is restored, and a fixed report line records one directory record read before the stop and again after the start. The acceptance measures that the restored file is that backup and that the two reads are equal. Ruled by Archie.

Accepted as written: both status lines are constrained by the ## Not met and ## Venue legs contents as the finding states, each with its acceptance line. Ruled by Archie.

Accepted as written, with one correction to the names. Add the acceptance line. The jsonl lines with repository 'rauthy', ref the Refs 'rauthy' commit and kind 'gate' or 'test', counted by distinct command, are exactly the legs of the Rauthy venue leg registration that the DIRECTORY-004 blocker lands, counted as for Cambium's gates.json. At least one such 'test' line's block under '## Venue legs' also carries 'target:' lines for the link, migration and audit regressions under vendor/rauthy/tests/identity_links. The correction: Rust's lints forbid uppercase function names, so the fork's tests are named id001_link_pair, id001_link_refusal, id001_link_migration and id001_link_audit, with the upper-case ids in their comments and printed output. The target lines match those lowercase names, and the brief says the ID001_LINK_* ids map to them. A report with no Rauthy gate or test line then fails R2. Answered by Archie, lead for the identity line.

## What the survey found, and its angles

Carry IDENTITY-001 revision 5's Row 07 (gate, install and demonstrate the exact release) as a new brief in the directory cluster, under the next free DIRECTORY id. Its work and acceptance stay as written, and the lead's rulings reshape its wall, blockers and report format. The row ends at a staged install keyed with a test key on the node the operator names. Every venue leg the tree registers must be green on exact pushed Lys, Rauthy and Cambium refs. The row 03 and row 05 live receipts are verified, and standalone acceptance, backups, a restore and a rollback go on record in a fixed-form release report and commands.jsonl. The production key, production receipts and the Cambium cutover are named as Tom's three acts, and the brief also adds an ADR, a demand-cadence release leg, and a one-line non-goal amendment.

### What the tree holds

- `docs/design/identity/briefs/IDENTITY-001.md (lines 390-427) and IDENTITY-001.json` — The source of Row 07: a 7-path wall, 4 work bullets, 2 acceptance bullets, 1 acceptance grep and a 5-hour estimate, plus the Authority, Ceiling, Design choices and Required venue checks (lines 428-434) it carries. Under R3 it is read and never changed.
- `docs/design/directory/design.json and DESIGN.md` — The cluster this brief continues. The intention sentence ends 'and this sentence promises neither row.', and non_goals[0] reads 'Rows 06 (connect Cambium) and 07 (gate, install and demonstrate the release)…', which must be edited to take row 07 out. The gate array (2 trees, 9 legs) gets the release leg appended. The structure array (96 rows) needs rows for every file this brief touches. CN1, CN6, CN7 (07 = 5h) and CN10 bind this row.
- `docs/design/directory/checklist.json / CHECKLIST.md` — 30 items in 6 sections and none for row 07. check-coverage requires the brief's checklist ids to exist, so new items are needed.
- `docs/design/directory/briefs/DIRECTORY-002.json, -003, -004, -005` — They now own rows 02, 04, 03 and 05. The release proof checks their landed work and receipts: ID001_LINK_LIVE is in DIRECTORY-004's verification and ID001_DIRECTORY_LIVE in DIRECTORY-005's. None of them mentions project.json, gates.sh or gates.json (0 hits each), so none registers a venue leg.
- `docs/design/directory/briefs/DIRECTORY-008.json` — Its recorded acceptance includes an intention endswith('and this sentence promises neither row.') check and an rg count of 1 for 'Rows 06 (connect Cambium) and 07 (...)'. Neither holds once row 07 leaves the non-goal, and the dev record must name both checks.
- `docs/design/project.json` — One tree with 7 legs, all cadence round. The identity-release leg is added here with cadence demand, which project.schema.json allows (enum round|demand), and requires tool:rustfmt and tool:cargo.
- `.land/gates.sh` — 7 legs. The ruling keeps the release leg out of it, so it must stay unchanged.
- `CLAUDE.md ('Gates before any commit')` — Gets one line naming the release leg as a demand leg that the release card runs and every land does not.
- `scripts/design/gate.sh, validate.py, check-coverage.py, render-cluster.py, render-brief.py, schemas/` — The method the brief must pass. gate.sh validates decisions.json and project.json, then validates, checks coverage for and re-renders every cluster with a design.json. docs/design/identity has no design.json and is not measured.
- `docs/design/decisions.json` — 18 ADRs on main (ADR-001 to ADR-018). The new ADR for the cross-repository arrangement takes the next free id across main and open branches.
- `docs/design/roadmap.json` — 11 rows, highest RM-016. RM-001 ('Stand up the identity directory…', briefed) is the row that carries this work.
- `deploy/identity/README.md` — On Row 07's wall, but DIRECTORY-002 already owns it in design.json structure ('install, readiness, backup and restore…'). Neither the file nor deploy/ exists yet.
- `scripts/identity-gates/` — On the wall, and does not exist. It holds the release leg's runner and the report and jsonl checks.
- `docs/design/identity/reports/IDENTITY-001-release.md and IDENTITY-001-commands.jsonl` — The fixed-form report and command log every acceptance line measures. The reports/ folder does not exist yet; structure already plans its sibling reports -deployment, -links and -standalone for DIRECTORY-002, 004 and 005.
- `vendor/rauthy (.gitmodules, branch ablative)` — A submodule pinned at dd61ac3c84d6b238108dc8438b53043b5177a662 and not initialised in this clone. The Rauthy ref gated and the fork tests id001_link_* under tests/identity_links live here.
- `cambium: gates.json, and draft/identity/debf7614 (RM-010)` — Cambium's 23 venue legs must be green on the exact Cambium ref. The install-document card lands docs/identity-install.md, which is absent at cambium origin/main 105c03d, and Cambium has no docs/design/identity.

### What was already decided

- directory design (docs/design/directory/design.json) — Carries rows 02, 04, 03 and 05 as DIRECTORY-002 to 005. Rows 06 and 07 are a non-goal until 'a Cambium cluster or an agreed cross-repository arrangement' exists, and IDENTITY-001 is 'Read, never changed'.
- CN6 — ID001_LINK_LIVE (after row 03) and ID001_DIRECTORY_LIVE (after row 05) are mandatory hold points recorded by Tom's receipts. A loop completion never stands in for one.
- CN7 — Revision 5's ceiling stands: 48 hours, row 07 at 5, IDENTITY-002's 4 outside it. An overrun is reported to Waffles.
- CN8 — One implementer, one gate at a time. Release builds, checks and tests run through the gate workflow at the venue.
- CN10 — IDENTITY-001-UPSTREAM-AUTH-STATE binds real sign-in and install for rows 06 and 07.
- CN1 — 'Documents only: nothing outside docs/design/directory/ and docs/design/decisions.json is created or modified'. This is DIRECTORY-001's constraint, and DIRECTORY-002 to 006 already carry code walls beside it.
- Non-goal: nightly Rauthy base vs waiting — Open for Tom. It falls due at the row 05 showing if no upstream release carries #1696 and #1728.
- Non-goal: production Cambium auth cutover — Forbidden before scratch acceptance, review and Gypsy's coordinated install.
- IDENTITY-001 Required venue checks — Lys six cargo legs; Cambium's exact gates.json legs; Rauthy baseline legs plus counted link, migration and audit regressions with upstream failures named; a native browser journey, two real provider test accounts, service stop and start, and a database restore.
- IDENTITY-001 Ceiling — Live demonstrations at rows 03 and 05. Row 07 is the final combined proof, not their first showing. Rows 06 and 07 depend on IDENTITY-002.
- ADR-009 — The fork branch ablative from v0.36.2, pinned as vendor/rauthy. Upgrades rebase onto upstream release tags only, each in its own gated row.
- ADR-005 — PostgreSQL may live on a network device, which bears on the node the operator names.
- ADR-004 — Every project stands alone. The staged install must run without Cambium or Manifold.
- RM-001 — The roadmap row for the standalone identity directory, which carries this brief.
- RM-003 — 'Move the Rauthy fork onto the next hardened upstream release', the idea row behind IDENTITY-002, with open rauthy-rebase brief and draft branches.

### What was measured

- IDENTITY-001.md length / Row 07 span: 442 lines; Row 07 at lines 390-427
- Row 07 contents: 7 wall paths (6 lys, 1 cambium), 4 work bullets, 2 acceptance bullets, 1 acceptance grep, 5 estimated hours
- Directory briefs on main: 7 (DIRECTORY-001 to 006 and 008); highest on main DIRECTORY-008
- Open directory brief/* and draft/* branches on origin: 46; 9 heads readable locally, 37 not in the local object store
- Highest DIRECTORY id seen on readable branches: DIRECTORY-013 (brief and draft directory/1e30a4cb). The next free id is at least DIRECTORY-014, pending the 37 unread heads.
- ADRs on main / highest ADR seen on readable branches: 18 (ADR-001 to ADR-018) / ADR-039, so the next free id is at least ADR-040
- Roadmap rows on main: 11, highest RM-016
- docs/design/project.json legs: 1 tree, 7 legs, all cadence round, 0 demand
- .land/gates.sh legs: 7
- directory design.json gate vs project.json trees: Not equal. The gate has 2 trees ('.': 6 cargo legs; 'docs': render, validate, coverage) with requires place:here; project.json has 1 tree with 7 legs including 'design'. home/design.json is equal to project.json and secrets/design.json is not.
- Venue-leg registrations in DIRECTORY-002/004/005 (project.json|gates.sh|gates.json): 0 in each
- Paths on the wall that exist today: 0 of 5 (scripts/identity-gates/, deploy/, docs/design/identity/reports/ and gates.json are all absent)
- design.json structure / checklist / stories: 96 structure rows; 30 checklist items in 6 sections, 0 for row 07; 12 stories
- vendor/rauthy submodule: Pinned at dd61ac3c84d6b238108dc8438b53043b5177a662, not initialised in this clone
- Cambium gates.json legs: 23
- Cambium install document at origin/main 105c03d: Absent. docs/identity-install.md and docs/design/identity both do not exist. The draft/identity/debf7614 head 052e04ba is not in the local store.
- DIRECTORY-008 recorded checks that stop holding after this card: 2 (the intention endswith check and the rg count of 1 for the rows 06-and-07 non-goal)
- Row 07 acceptance lines the '## Status' section must count: 2

### What it means for the other projects

- cambium — Its install-document card (RM-010, draft/identity/debf7614) must land docs/identity-install.md, creating docs/design/identity, and v0CuMstE (row 06's Cambium half) must land before this build starts. Its 23 gates.json legs are run on the exact Cambium ref. No cutover happens, and Gypsy receives the staged issuer's address and the install receipts only.
- aion — The card chain (card_build_v3, src_land) has to run a demand-cadence leg on request for the release card and skip it on every other land. It is not verified here that the workflow reads cadence.
- method — No change. scripts/design is a copy at method commit 3c3bac7, whose schemas already allow cadence 'demand'.

### The decisions it stands on

- ADR-009 (honour) — The Rauthy ref gated is an exact pinned commit on ablative, and the baseline ref is the upstream release commit IDENTITY-002's rebase names. There are no cherry-picks.
- ADR-004 (honour) — Standalone acceptance on a staged install with no Cambium or Manifold address in any configuration file is measured.
- ADR-005 (honour) — The staged install uses one PostgreSQL database, possibly on the node the operator names, and is backed up before migration.
- ADR-003 (honour) — The agent's recorded creation is inspected under its responsible person. No grant enforcement is claimed.
- ADR-008 (honour) — Receipts inspected on the staged install are test-keyed and marked so. No certificate is presented as production.
-  (new) — Records the cross-repository arrangement for row 07 (lys card, Cambium install-document card, v0CuMstE, Tom's three acts) at the next free ADR id, and the non-goal is amended to name it.

### What it requires

- A brief DIRECTORY-0NN.json and .md (next free id, at least 014) exists in docs/design/directory/briefs and passes scripts/design/gate.sh.
- The brief's blocked_by names row 06's lys half (LsCN9H7-, c7d95bf6) and Cambium half (v0CuMstE), IDENTITY-002, DIRECTORY-002 to 005 by the rows they carry, the ID001_LINK_LIVE and ID001_DIRECTORY_LIVE receipts, the Cambium install-document card RM-010 at docs/identity-install.md, and DIRECTORY-002's missing identity leg.
- DIRECTORY-004's and 005's missing venue legs are named as findings for their owners, and those briefs are unchanged.
- docs/design/project.json has one identity-release leg with cadence demand whose requires include tool:rustfmt and tool:cargo, and directory design.json's gate carries the same leg.
- CLAUDE.md's gate list gains exactly one line naming the identity-release leg as a demand leg.
- The directory non-goal names row 06 alone and cites the new ADR, the intention sentence is grammatical for row 06 alone, and nothing else in either changes.
- A new ADR at the next free id records the row 07 cross-repository arrangement.
- IDENTITY-001-commands.jsonl has one kind per line. Gate and test lines precede every install line. Backup lines precede the first install line that starts a binary against the database.
- An ancestry line records git merge-base --is-ancestor <base> <lys commit> exit 0.
- IDENTITY-001-release.md has fixed sections: ## Refs, ## Venue legs, ## Install, ## Standalone acceptance, ## Not met and ## Status.
- Every gate jsonl line is cited under ## Venue legs.
- The Rauthy gate and test lines equal the registered Rauthy legs, with target: lines for id001_link_pair, refusal, migration and audit.
- Standalone acceptance cites no command containing 'health', and every screen: line names a screen action.
- The 'staging statement: ' line contains 'staging install'.
- Each configuration file is recorded as 'configuration: <file> sha256 <64-hex>', and none names a Cambium or Manifold host.
- A service stop and start, and a restore from the recorded backup of the migrated install, are each a jsonl line with a fixed report line, and one directory record reads equal before the stop and after the start.
- Every ## Not met line has the form '<requirement> — met by: <act> — performed by: <role>'. 'preserved Cambium identities are observed' names the product owner. Each missing or failed ID001_LINK_LIVE or ID001_DIRECTORY_LIVE receipt is listed.
- ## Status holds exactly 2 met or not met lines, and 'row 07: complete' appears only if both are met.
- Failing Rauthy test names in both runs are in fixed form, each failing test is also failing on its baseline line, and the baseline ref is IDENTITY-002's upstream release commit.

### What must not change

- IDENTITY-001.json and IDENTITY-001.md are not changed.
- CONTEXT-001 is not changed, and docs/design/identity is not moved, excluded or registered in roadmap.json.
- .land/gates.sh is not changed.
- DIRECTORY-002 to 005 and DIRECTORY-008 are not edited.
- No file under surface/identity or crates/lys-identity-server changes.
- No production key, no signed receipt under a production key and no production anchor.
- No Cambium cutover, and no staging Cambium is stood up.
- No file in the Cambium repository lands through this card.
- No cryptographic or wire format changes.
- Row 06's words in the non-goal and intention are left to draft c7d95bf6.
- An old Rauthy binary is never launched against a migrated, forward-only database.

### What we must put in place first

- DIRECTORY-002 to 005 landed and built, with ID001_LINK_LIVE and ID001_DIRECTORY_LIVE receipts recorded by Tom.
- Row 06 landed in both halves (lys LsCN9H7-/c7d95bf6, Cambium v0CuMstE).
- IDENTITY-002 (the rauthy-rebase under RM-003) opened and landed on an upstream release carrying #1696 and #1728, or Tom's nightly-versus-wait ruling.
- A venue leg registered for DIRECTORY-002's identity container, the Rauthy fork suite that DIRECTORY-004's fork brief lands, and DIRECTORY-005's frontend checks.
- Cambium's install-document card created by Cambium's lead and landed at docs/identity-install.md.
- The operator names a staging node and supplies a test service key and two real upstream provider test-account registrations (Google, GitHub) for the staged issuer.

### The risks

- The design.json gate equality line fails unless the gate is rewritten wholesale.
- The next DIRECTORY or ADR id collides because 37 branch heads went unread.
- IDENTITY-002 has no upstream release, so the baseline-ref check has no commit to name and Tom's nightly decision is needed.
- The Rauthy, DIRECTORY-002 and DIRECTORY-005 legs are never registered, so the card stays blocked indefinitely.
- Missing operator provider registrations block the two-provider demonstration, and it is listed under Not met.
- The row 06 and row 07 non-goal edits race with draft c7d95bf6.
- The aion chain does not honour cadence demand, so the release leg runs on every land or never.
- A heavy Rauthy, Lys and Cambium gate run on this Mac instead of Dean's laptop breaks Tom's rule 3 and 7.
- deploy/identity/README.md is edited by two briefs and the edits conflict.
- A 'staging' install is mistaken for a production release by a reader of the report.

### Still open

- The directory design.json gate differs from project.json trees today (2 trees with place:here legs against 1 tree of 7 legs), so appending one leg cannot make them equal. Should this card replace the directory gate wholesale with project.json's trees, as home/design.json already is, or should the acceptance line check only that the appended leg matches? The sentence of the words it stands on: "Accepted as written: R1 also modifies docs/design/directory/design.json and DESIGN.md, appends the same leg to the design gate, and gains the acceptance line that design.json gate equals project.json trees.". Why only the lead can settle it: docs/design/directory/design.json 'gate' has trees '.' and 'docs' (render/validate/coverage, requires place:here), while docs/design/project.json has one tree '.' with a 'design' leg. The equality line would fail on the tree as it stands unless the whole gate is rewritten, which changes what the directory cluster's gate runs.

### The units beyond the first

- Visible staging mark on the identity screens — This is a surface change outside this wall, ruled a later card on the directory board.
- Cambium install document (docs/identity-install.md) on the Cambium board — It lands through Cambium's own card, PR and gate (RM-010), not this lys card.
- Register DIRECTORY-002's identity container venue leg — The leg belongs to DIRECTORY-002's line and the lead cards it separately. This card is blocked on it.
- Register the Rauthy fork venue leg (DIRECTORY-004's fork brief) and DIRECTORY-005's frontend leg — These are findings for those briefs' owners. This card does not register them.
- Production signing key, production receipts and the Cambium cutover — These are Tom's three acts. Performing the cutover is what meets 'preserved Cambium identities are observed'.

### The smallest complete shape

One lys card, the whole of it:
- The DIRECTORY-0NN brief with its blockers and findings.
- The new ADR.
- The one-line non-goal and intention amendment.
- The demand-cadence identity-release leg in project.json, design.json's gate and CLAUDE.md.
- scripts/identity-gates, with the report and jsonl checks.
- The deploy/identity/README.md release section.
- The staged, test-keyed install with its fixed-form IDENTITY-001-release.md and IDENTITY-001-commands.jsonl, with every unmet requirement named.

## The roadmap row

- **RM-055** — Gate, install and demonstrate the exact identity release, up to a staged install (feature, idea)
- Summary: IDENTITY-001 revision 5's Row 07 carried as DIRECTORY-030 in the directory cluster: every required venue leg green on the exact pushed Lys, Rauthy and Cambium refs, the row 03 and row 05 live install receipts verified, standalone acceptance recorded on a staged, test-keyed install before any cutover, backups and a rollback that respects migrated schemas, every command, result, ref and hash recorded, every requirement not met named, and health alone never completion. The production signing key, production receipts and the Cambium cutover stay Tom's (ADR-092).
- Asked by: tom on 2026-09-27T12:44:00+10:00
- Context: The Lys board card for IDENTITY-001's row 07, with the lead's rulings R1 to R7 given to run 19c38e64, the rulings given to run dc4ff2c9-8791-4de3-afe4-425d614efe34 and to run a3c1d516-d589-4177-a1fb-34169fb53cbd, whose draft this row carries forward, and the lead's answer to this run's question on the directory gate (replaced wholesale with the project's trees). RM-001, the identity directory row, carries the cluster; this row is the brief's own ledger entry and depends on it.
- Quote: The brief for this card already stands on main: docs/design/identity/briefs/IDENTITY-001.md and .json, revision 5, section "Row 07 — Gate, install and demonstrate the exact release". Read it in full (Authority, Ceiling, Design choices, Required venue checks, then Row 07 with its file wall, work and acceptance) and carry it as this card's brief, in the identity cluster. Keep its work and acceptance exactly as written: every required venue leg green on the exact pushed Lys, Rauthy and Cambium refs; the row 03 and row 05 live install receipts verified; standalone acceptance recorded before Cambium cutover; backups and a rollback that respects migrated schemas; every command, result, ref and hash recorded in reports/IDENTITY-001-release.md and IDENTITY-001-commands.jsonl; every requirement not met is named; and health alone is never completion.

Keep its order: Row 07 depends on row 06 and IDENTITY-002, and on rows 01 to 06 being done with their live demonstrations at the ends of rows 03 and 05. The brief says so as a blocker, and the build does not start until those have landed. Keep its file wall, including its one cambium file, docs/design/identity/IDENTITY-001-install.md. Where that row's rows have since moved into the directory cluster (DIRECTORY-002 to DIRECTORY-005 carry rows 02, 04, 03 and 05), name the directory brief that now owns each row the release proof checks. No production key, no receipt outside a test, and no production anchor: those acts stay Tom's. Keep to the method (scripts/design/validate.py, check-coverage.py and render-cluster.py, run by scripts/design/gate.sh). If the survey finds a sentence of Row 07 open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the row around it. The card is built from this brief only after Tom or the lead signs it off on the card.

Rulings of the lead, Archie, on 27 September 2026 at 08:33, given to the first run 19c38e64 in answer to its seven survey questions. That run took all seven and then failed with ClusterRefused, because docs/design/identity/briefs holds CONTEXT and IDENTITY briefs. They are settled here, and the author reopens none of them. R1: where these words say the identity cluster, the brief lives in the directory cluster, which scripts/design/gate.sh measures, under the next free DIRECTORY id after main's highest and every open brief/* and draft/* branch's, checked with git ls-remote just before writing. The docs/design/identity folder stays as it is, unregistered in roadmap.json, and IDENTITY-001.json and CONTEXT-001.json are neither moved nor excluded. R2: the wall's gates.json means .land/gates.sh, mirrored in CLAUDE.md's gate list, and docs/design/project.json. Each row registers the leg it needs in its own brief; the Rauthy container leg is DIRECTORY-002's identity leg, and this card registers only the release leg it adds. A leg that DIRECTORY-004 or 005 needs and nothing registers first is named as a finding for that brief's owner, and those briefs are not edited here. R3: IDENTITY-001.json and .md are never changed, as the directory design's "Read, never changed" says. Status from evidence is recorded only in this brief and the release report, and both files leave the wall. R4: this lys card does not land docs/design/identity/IDENTITY-001-install.md. That file lands through the Cambium identity cluster, ruled at 08:30 for card v0CuMstE, as its own Cambium card with its own PR and gate. This card names it as a dependency and leaves it off its wall. R5: row 06 is already carded in two halves, the lys half LsCN9H7- (brief run c7d95bf6) and the Cambium half v0CuMstE, which owns ID001_CAMBIUM_LINK, ID001_GOOGLE_SEPARATE and ID001_PKCE, with S256 ruled in scope. Both are dependencies, each checked by its landed brief file, and this card opens neither. R6: row 07 stops at a staged install on the node the operator names, keyed with a test service key. It proves standalone acceptance there and records it before any cutover, and it names the production signing key, production receipts and the Cambium cutover as Tom's three acts. The operator is shown a staging install, labelled as such. R7: "no receipt outside a test" means no signed Lys receipt under a production key. Verifying the row 03 and 05 install posts is allowed, and so is inspecting the agent's recorded creation on the staged install, whose receipt is test-keyed. The release report marks each such receipt as test-keyed.

Rulings of the lead, Archie, given on 27 September 2026 to the run dc4ff2c9-8791-4de3-afe4-425d614efe34 in answer to its round 1. That run took every answer and then failed before writing, when its writer found saved state for its session already standing under another config folder. They are settled here, and the author reopens none of them.

Preserved Cambium identities are recorded as a named unmet requirement until the cutover. Row 07 stops before any cutover, as R6 rules, so no staging Cambium is stood up by this row. The release report lists 'preserved Cambium identities are observed' as not met, naming the cutover as the act that meets it and Tom as the one who performs it. The coordination with Gypsy in Work bullet 3 is recorded as the handover of the staged issuer's address and the install receipts, not as an install. Health of the staged issuer is never counted toward this line.

The staging label is given in the demonstration and the release report only. No file under surface/identity or crates/lys-identity-server changes, so the row stays inside its wall. The demonstration's opening states that the operator is looking at a staging install, and the release report records that it was said. A visible staging mark on the screens is a later card on the directory board, and the brief names it under further units not written.

The install document is a separate Cambium card, not v0CuMstE. The 08:30 ruling placed the install document in Cambium's identity cluster, and R5 keeps v0CuMstE as row 06's Cambium half, so the install document gets a card of its own on the Cambium board with its own PR and gate. The brief's blocked_by names that card as the Cambium install-document card, to be made by Cambium's lead, and states that Cambium has no docs/design/identity folder yet, so that card creates it.

Rulings of the lead, Archie, given on 27 September 2026 to the run a3c1d516-d589-4177-a1fb-34169fb53cbd in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

The path that lands. The blocker names the Cambium install-document card, RM-010 on draft/identity/debf7614, and checks the document at the path that card lands, docs/identity-install.md as its draft writes, read from Cambium main once it has landed. The words' docs/design/identity/IDENTITY-001-install.md is superseded, and the brief says so, so the blocker can clear. If that card lands the document at another path, the blocker reads that path and the dev record names it. Answered by Archie, lead for the identity line.

A demand-cadence leg in docs/design/project.json only. It is not added to .land/gates.sh, so no other lys card's land runs it or goes red for want of Rauthy and Cambium refs and a staged install. CLAUDE.md's gate list names it as a demand leg, run by the release card and not by every land, in one line. Answered by Archie.

Yes. The missing identity leg of DIRECTORY-002 is named as a finding for its owner, as DIRECTORY-004 and 005 are, and held as a blocker of this card. The release proof's 'every required venue leg green' checks against the legs the tree actually registers, and this card does not register DIRECTORY-002's leg for it. The lead cards that leg for DIRECTORY-002's line. Answered by Archie.

Lift it, with a decision. This card records the cross-repository arrangement for row 07 as a decision at the next free ADR id, read immediately before writing, and amends the non-goal so it no longer covers row 07, naming that decision. Row 06 is draft c7d95bf6's. Whichever of the two lands second reads the non-goal from main as the first left it and changes only its own row's words, so neither undoes the other. Answered by Archie.

Edit it only to take row 07 out, and rewrite nothing else in it. The sentence names row 06 alone, for example '...row 06 stays the non-goal recorded below, and this sentence does not promise it.', so that it is grammatical and true once row 07 is lifted. DIRECTORY-008's recorded endswith check is that card's own evidence of what it landed, and it is not a live gate, so it is not kept true by leaving a false sentence. The dev record names the check, says it no longer holds after this card, and gives the ruling as the reason. Draft c7d95bf6 owns row 06's words, so whichever of the two lands second reads the sentence from main and changes only its own row. Answered by Archie, lead for the identity line.

Accepted. Make every correction the round proposes, as Q10 to Q20 word it, and settle each finding by that change. For Q12, move receipt recording ahead of the handover, so that no requirement depends on a later one. For Q17, fix a 'kind' per jsonl line and classify test commands by it. For Q20, add the service stop and start and the restore from the recorded backup to R3, each recorded as a jsonl line with a fixed report line and measured by an acceptance line. Nothing here goes to Tom. Answered by Archie, lead for the identity line.

Met by r4-backup-before-migration-unmeasured as ruled there. Answered by Archie.

Met by r6-not-met-act-performer-unmeasured and r6-status-format-unmeasured as ruled there: the spec fixes both line formats and the completion line. Answered by Archie.

Met by the same two rulings, each with its acceptance line. Answered by Archie.

Accepted as proposed. git merge-base --is-ancestor <base> <lys commit> must exit 0, where <base> is the commit the build started from, which holds the landed blockers. It is recorded as a jsonl line of kind ancestry, with a '## Refs' line, and one acceptance line checks it. A lys commit from before DIRECTORY-002 to 005 landed fails. Answered by Archie.

Accepted as proposed. Each backup line under '## Install' cites its commands.jsonl:<n>. An acceptance line requires every jsonl line of kind backup to come before the first line of kind install that starts a Rauthy or lys binary against the database. Answered by Archie.

Accepted as proposed, both checks. No line of '## Standalone acceptance' cites a commands.jsonl line whose command contains 'health', and every 'screen:' line's observed text names a screen action, not a command. Answered by Archie.

Accepted as proposed. The text after 'staging statement: ' must contain 'staging install', and it is measured. Answered by Archie.

Measure it, and don't drop it. Record 'configuration: <file> sha256 <64-hex>' for each configuration file of the staged install. An acceptance line checks that no recorded file names a Cambium or Manifold address or host. Answered by Archie.

Accepted as proposed. Each '## Not met' line has the fixed form '<requirement> — met by: <act> — performed by: <role>', and every line is measured for both. The line for 'preserved Cambium identities are observed' names the product owner as performer, and that is measured. Answered by Archie.

Accepted as proposed. '## Status' holds exactly one 'met: <line>' or 'not met: <line>' per acceptance line of revision 5's Row 07, counted as exactly that many. The completion line is the literal 'row 07: complete', present only when every one is met, and checked against that fixed form. Answered by Archie.

Accepted as proposed. For each of ID001_LINK_LIVE and ID001_DIRECTORY_LIVE with no receipt line, or with an 'is-ancestor: ' token not followed by 0, '## Not met' holds a line naming that identifier, and an acceptance line measures it. Drop the 'verified' marker: a receipt is either on the receipt line with its ancestry exit 0, or it is under '## Not met'. Answered by Archie.

Accepted as proposed. tool:rustfmt and tool:cargo go into the identity-release leg's requires and into the acceptance's expected list. Answered by Archie.

Accepted as written: R1 also modifies docs/design/directory/design.json and DESIGN.md, appends the same leg to the design gate, and gains the acceptance line that design.json gate equals project.json trees. Ruled by Archie, lead for the identity line.

Accepted as written: failing test names of both runs are recorded in a fixed form, every failing test named for line <n> must be named for its baseline line <m>, and line <m>'s ref must be the upstream release commit IDENTITY-002's rebase names. Ruled by Archie.

Accepted as written: an acceptance line that every jsonl line of kind gate has its commands.jsonl:<n> line under ## Venue legs. Ruled by Archie.

Accepted. The trigger reads the kinds gate, test and R2's own two ancestry lines. An acceptance line asserts that every install line comes after every line of those kinds, and that each of those has exit_status 0 or is a Rauthy line cited on an upstream: commands.jsonl:<n> line. Ruled by Archie.

Accepted as written: a backup of the migrated staged install, cited by its own backup: line, is what is restored, and a fixed report line records one directory record read before the stop and again after the start. The acceptance measures that the restored file is that backup and that the two reads are equal. Ruled by Archie.

Accepted as written: both status lines are constrained by the ## Not met and ## Venue legs contents as the finding states, each with its acceptance line. Ruled by Archie.

Accepted as written, with one correction to the names. Add the acceptance line. The jsonl lines with repository 'rauthy', ref the Refs 'rauthy' commit and kind 'gate' or 'test', counted by distinct command, are exactly the legs of the Rauthy venue leg registration that the DIRECTORY-004 blocker lands, counted as for Cambium's gates.json. At least one such 'test' line's block under '## Venue legs' also carries 'target:' lines for the link, migration and audit regressions under vendor/rauthy/tests/identity_links. The correction: Rust's lints forbid uppercase function names, so the fork's tests are named id001_link_pair, id001_link_refusal, id001_link_migration and id001_link_audit, with the upper-case ids in their comments and printed output. The target lines match those lowercase names, and the brief says the ID001_LINK_* ids map to them. A report with no Rauthy gate or test line then fails R2. Answered by Archie, lead for the identity line.
- Cluster: directory; briefs: DIRECTORY-030
- Notes: Ids read immediately before writing from every brief/*, draft/* and hand/* head git ls-remote lists, each fetched, against main fa3dd53: DIRECTORY-030 after DIRECTORY-029, RM-055 after RM-054, ADR-092 after ADR-091, C242 after C241 and S110 after S109; the earlier drafts' DIRECTORY-015, RM-033 and ADR-049, and DIRECTORY-025, RM-048 and ADR-078, are taken on other open branches. Further units, not written: Visible staging mark on the identity screens; Cambium install document (docs/identity-install.md) on the Cambium board; Register DIRECTORY-002's identity container venue leg; Register the Rauthy fork venue leg (DIRECTORY-004's fork brief) and DIRECTORY-005's frontend leg; Production signing key, production receipts and the Cambium cutover.

## The design

---
type: design
cluster: directory
title: The standalone identity directory, with every grant rooted in a person
---

# The standalone identity directory, with every grant rooted in a person

> **Cluster:** directory

## Intention

An operator installs the identity product without Cambium or Manifold, signs in, links Google and GitHub to one person, registers an agent under a responsible person, and inspects the signed history of every identity change. Cambium later uses this issuer, keeping its participant ids (row 06); that direction is recorded here while row 06 stays the non-goal recorded below, and this sentence does not promise it.

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
- ADR-092 — Row 07 is carried in the directory cluster up to a staged install, with its Cambium parts on Cambium-owned cards and the production acts left to Tom — Row 07 leaves the non-goal and is carried by a directory brief that stops at a staged install on the node the operator names, keyed with a test service key, where standalone acceptance is proved and recorded before any cutover. The Cambium install document lands through its own Cambium card with its own pull request and gate, and row 06's Cambium half stays on its own Cambium card; this brief names both as blockers and lists no Cambium file. The brief registers exactly one release leg, as a demand-cadence leg in docs/design/project.json, and every other row registers the leg it needs in its own brief. The production signing key, production receipts and the Cambium cutover are Tom's three acts. Rejected: leaving row 07 a non-goal beside a brief that carries it, listing the Cambium install document on this brief's wall, adding the release leg to .land/gates.sh where every land would run it, and standing up a staging Cambium inside this row.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.

## Non-Goals

- Row 06 (connect Cambium) — It changes the Cambium repository and depends on IDENTITY-002, the upstream release rebase; it needs a Cambium cluster or an agreed cross-repository arrangement first. Row 07 left this non-goal with DIRECTORY-030 (ADR-092).
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
| `docs/design/directory/briefs/DIRECTORY-030.json` | row 07 of IDENTITY-001: gate, install and demonstrate the exact release, up to a staged, test-keyed install | DIRECTORY-030 |
| `docs/design/directory/briefs/DIRECTORY-030.md` | rendered markdown | DIRECTORY-030 |
| `docs/design/project.json` | the project's gate trees; gains the demand-cadence identity-release leg | DIRECTORY-030 |
| `CLAUDE.md` | the repository's standing instructions; its gate list gains one line naming the identity-release demand leg | DIRECTORY-030 |
| `scripts/identity-gates/release.sh` | the identity-release leg: refuses an unclean tree, a vendor/rauthy pin off ablative and an unpushed HEAD, then runs the six lys legs and prints one JSON line per leg | DIRECTORY-030 |
| `docs/design/identity/reports/IDENTITY-001-release.md` | row 07's release report: refs, venue legs, review, install, receipts, standalone acceptance, not met, reserved acts, status | DIRECTORY-030 |
| `docs/design/identity/reports/IDENTITY-001-commands.jsonl` | row 07's command record: one JSON line per command run for the release | DIRECTORY-030 |

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
id: DIRECTORY-030
cluster: directory
title: Gate, install and demonstrate the exact release, up to a staged install
---

# DIRECTORY-030: Gate, install and demonstrate the exact release, up to a staged install

> **Cluster:** directory
> **Depends on:** DIRECTORY-005, DIRECTORY-010
> **Blocked by:** IDENTITY-002, owned by no directory brief and not on main, named by its roadmap row RM-003 and its rauthy-rebase brief branch brief/rauthy-rebase/3c1630c4-b0a7-4c99-adec-19a72deef34d (brief RAUTHYREBASE-001, with its draft/rauthy-rebase/* runs): ablative rebased onto an upstream Rauthy release carrying #1696 and #1728 and landed through its own gated row, checked by vendor/rauthy's pin on lys main moving to that rebased ablative commit; or Tom's nightly-versus-wait ruling, which the design's non-goals record OPEN for Tom and this brief does not decide. IDENTITY-001-UPSTREAM-AUTH-STATE binds install in row 07 (CN10, ADR-009). Upstream's latest release is still v0.36.2, so IDENTITY-002 cannot yet open., Rows 01 to 05 done: row 01 closed (docs/design/identity/RAUTHY-BASELINE.md); rows 02, 04, 03 and 05 landed as DIRECTORY-002, DIRECTORY-003, DIRECTORY-004 and DIRECTORY-005, each checked by its brief file on lys main carrying an execution record with status landed and a landed_commit., ID001_LINK_LIVE, the row 03 live demonstration: Tom's install and showing receipt recorded after DIRECTORY-004 lands (CN6), checked by the receipt's post, which names the fork ref and artifact hash R3 verifies., ID001_DIRECTORY_LIVE, the row 05 live demonstration: Tom's receipt recorded after DIRECTORY-005 lands (CN6), checked by the receipt's post, which names the installed refs and hashes R3 verifies., Row 06, lys half: card LsCN9H7- (brief run c7d95bf6), whose brief is DIRECTORY-010 on draft/directory/c7d95bf6-df47-479b-8574-f50151d1532f and not yet on main; checked by its landed brief file on lys main carrying an execution record with status landed and a landed_commit. If it lands under another id, the dev record names that id. This card does not open it., Row 06, Cambium half: card v0CuMstE on Cambium's board, which owns ID001_CAMBIUM_LINK, ID001_GOOGLE_SEPARATE and ID001_PKCE with S256 ruled in scope; checked by its landed brief file on Cambium main and the landed commit it names, at which rg -n 'ID001_CAMBIUM_LINK|ID001_GOOGLE_SEPARATE|ID001_PKCE' crates/cambium-door/tests crates/cambium-store/tests finds all three. No Cambium brief for it was found when this brief was written. This card does not open it., The Cambium install-document card, to be made by Cambium's lead: its draft is RM-010 on Cambium branch draft/identity/debf7614-d523-4930-bf18-88e6d7f0f3f3. It is a separate Cambium card with its own pull request and gate, not v0CuMstE. Cambium has no docs/design/identity folder on main yet, so that card creates it. Checked by the install document on Cambium main at the path that card lands, docs/identity-install.md as its draft writes; revision 5's docs/design/identity/IDENTITY-001-install.md is superseded by that path. If the card lands the document at another path, this blocker reads that path and the dev record names it., Leg findings for other briefs' owners, held as blockers of this card and not registered by it: DIRECTORY-002 registers no identity (Rauthy container) leg; DIRECTORY-004 registers no Rauthy venue leg; DIRECTORY-005 registers no frontend leg (each brief mentions .land/gates.sh, docs/design/project.json and gates.json 0 times on main). The lead cards each leg for its own brief's line; each is checked by its leg standing in docs/design/project.json on lys main., Sign-off of this brief on the card by Tom or the lead before the card is built.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> - ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-092 — Row 07 is carried in the directory cluster up to a staged install, with its Cambium parts on Cambium-owned cards and the production acts left to Tom — Row 07 leaves the non-goal and is carried by a directory brief that stops at a staged install on the node the operator names, keyed with a test service key, where standalone acceptance is proved and recorded before any cutover. The Cambium install document lands through its own Cambium card with its own pull request and gate, and row 06's Cambium half stays on its own Cambium card; this brief names both as blockers and lists no Cambium file. The brief registers exactly one release leg, as a demand-cadence leg in docs/design/project.json, and every other row registers the leg it needs in its own brief. The production signing key, production receipts and the Cambium cutover are Tom's three acts. Rejected: leaving row 07 a non-goal beside a brief that carries it, listing the Cambium install document on this brief's wall, adding the release leg to .land/gates.sh where every land would run it, and standing up a staging Cambium inside this row.
> **Checklist:**
> - C242 — The identity-release leg is registered once, as a demand-cadence leg in docs/design/project.json running scripts/identity-gates/release.sh, named in one line of CLAUDE.md and mirrored in the directory design's gate, and .land/gates.sh is unchanged.
> - C243 — Every required venue leg is recorded green on the exact pushed Lys, Rauthy and Cambium refs in IDENTITY-001-commands.jsonl and the release report, and a missing leg is named as a blocker.
> - C244 — The row 03 and row 05 live install receipts are verified against the release refs and each is marked test-keyed.
> - C245 — The staged install runs the tested refs on the node the operator names under a test service key, with its backups and a rollback that never launches an older Rauthy binary against a forward-only migrated database recorded.
> - C246 — Standalone acceptance on the staged install is recorded before any cutover, with the staging statement at the demonstration's opening, who the two providers resolve to and the agent's recorded creation.
> - C247 — Every requirement not met is named in the release report, preserved Cambium identities among them until the cutover, the three reserved acts are named, and no health check is counted as completion.
> **Stories:**
> - S110 (Release reviewer, Decides from the record whether the release proof is complete) — As the release reviewer, I want every command, result, ref and hash recorded with every unmet requirement named, so that completion rests on recorded evidence and never on a health check.
> - S111 (Operator, Installs and runs the standalone identity product) — As the operator, I want the exact release installed on a node I name and shown to me as a staging install, so that I can accept the product standalone before anything is cut over.

## Purpose

Carry IDENTITY-001 revision 5's Row 07, 'Gate, install and demonstrate the exact release', as a design-system brief, with its work and acceptance kept: every required venue leg green on the exact pushed Lys, Rauthy and Cambium refs; the row 03 and row 05 live install receipts verified; standalone acceptance recorded before Cambium cutover; backups and a rollback that respects migrated schemas; every command, result, ref and hash recorded in docs/design/identity/reports/IDENTITY-001-release.md and docs/design/identity/reports/IDENTITY-001-commands.jsonl; every requirement not met named; and health alone never completion. Under ADR-092 the row stops at a staged install on the node the operator names, keyed with a test service key, and leaves the production signing key, production receipts and the Cambium cutover as Tom's three acts. It is the final combined release proof, not the first showing of rows 03 and 05.

## Task

Source. docs/design/identity/briefs/IDENTITY-001.md and .json, revision 5, section 'Row 07 — Gate, install and demonstrate the exact release', read with its Authority, Ceiling, Design choices and Required venue checks. Those two files are read and never changed (the design's inventory: 'Read, never changed'); status from evidence is recorded only in this brief's execution record and the release report. Estimate: 5 focused implementer hours, row 07's share of CN7's 44.5 against the 48-hour ceiling, which stands; an overrun is reported as soon as it is known and Waffles takes it to Tom. Every path is relative to the repository root (CN3).

Rows the release proof checks, and the brief that now owns each: row 01, closed, by docs/design/identity/RAUTHY-BASELINE.md; row 02 by DIRECTORY-002; row 04 by DIRECTORY-003; row 03 by DIRECTORY-004; row 05 by DIRECTORY-005; row 06 by its lys half, card LsCN9H7- (DIRECTORY-010 on its draft), and its Cambium half, card v0CuMstE. Order: row 07 depends on row 06 and IDENTITY-002, and on rows 01 to 06 being done with their live demonstrations at the ends of rows 03 and 05; blocked_by holds each, and the build does not start until all have landed.

Wall. Revision 5's wall is kept with the lead's readings. gates.json means the lys gate registration: the release leg is a demand-cadence leg in docs/design/project.json only, named in one line of CLAUDE.md's gate list, and .land/gates.sh is not changed, so no other lys land runs it. The directory design's gate array was replaced wholesale with docs/design/project.json's trees when this brief was written, as the home design's already is, so the build appends the same leg there and re-renders DESIGN.md. This changes what the directory cluster's gate runs: it no longer carries its former 'docs' tree of render, validate and coverage legs or its '.' tree's cargo legs under place:here, and runs exactly what the project's gate runs, one tree '.' with the six cargo legs and the design leg. The design leg, sh scripts/design/gate.sh, validates, checks coverage for and renders docs/design/directory with every other cluster that has a design.json, so nothing checked before goes unchecked. scripts/identity-gates/ holds the leg's one script, scripts/identity-gates/release.sh, a gate script beside scripts/design/gate.sh, not provisioning. deploy/identity/README.md is created by DIRECTORY-002 and gains the release rollback here. The two reports live under docs/design/identity/reports/, beside the row reports DIRECTORY-002, 004 and 005 write. IDENTITY-001.json and IDENTITY-001.md leave the wall. The one Cambium file leaves the wall: the install document lands through the Cambium install-document card, at docs/identity-install.md as that card's draft writes, which supersedes revision 5's docs/design/identity/IDENTITY-001-install.md.

Rauthy test names. Rust's lints forbid upper-case function names, so the fork's link, migration and audit regressions under vendor/rauthy/tests/identity_links are named id001_link_pair, id001_link_refusal, id001_link_migration and id001_link_audit, carrying the upper-case ids in their comments and printed output: ID001_LINK_PAIR maps to id001_link_pair, ID001_LINK_REFUSAL to id001_link_refusal, ID001_LINK_MIGRATION to id001_link_migration and ID001_LINK_AUDIT to id001_link_audit, and the report's 'target:' lines match the lower-case names.

Where the row stops. A staged install on the node the operator names, keyed with a test service key; standalone acceptance is proved there and recorded before any cutover. No staging Cambium is stood up. The product owner is Tom, and the three reserved acts are his: the production signing key, production receipts, and the Cambium cutover, which is the act that meets 'preserved Cambium identities are observed'. 'No receipt outside a test' means no signed Lys receipt under a production key: verifying the row 03 and 05 install posts is allowed, and so is inspecting the agent's recorded creation on the staged install, whose receipt is test-keyed; the report marks each such receipt test-keyed. The Cambium runtime owner is Gypsy: Work bullet 3's coordination with Gypsy is the handover of the staged issuer's address and the install receipts, not an install. The staging label is given in the demonstration's opening and in the release report only; no file under surface/identity or crates/lys-identity-server changes, and a visible staging mark on the screens is a later directory card.

Ceiling lines read under the card chain. Revision 5's 'One implementer, Chippy; no builders' and 'Edit main in the main Mac checkout. No worktree' give way to the standing rules in CLAUDE.md: the card runs through its board's chain (brief_card, sign-off, card_build_v3, src_pr, src_land), is built in the chain's clone, and every compile, test battery and gate runs on the build laptop. One row in implementation and one gate invocation at a time in this lane (CN8) stands.

Ledger. This brief is DIRECTORY-030, the next free DIRECTORY id after main's highest (DIRECTORY-008) and every open brief/*, draft/* and hand/* branch's (DIRECTORY-029). The branches were listed with git ls-remote immediately before writing and every brief/*, draft/* and hand/* head was fetched, because most heads were not in the local object store and ls-remote gives only commits; the ids were read from the fetched trees. Its roadmap row is RM-055, after RM-054 on the open branches, and its decision ADR-092, after ADR-091 on the open branches. Earlier drafts of this card carried DIRECTORY-015, RM-033 and ADR-049, and then DIRECTORY-025, RM-048 and ADR-078; each is now taken on another open branch, DIRECTORY-025 among them by the road step 2 permissions brief, so the draft is renumbered under the rule that a brief's ids are the next free ones. The design's first non-goal no longer covers row 07 (ADR-092); row 06's words in it are draft DIRECTORY-010's, and whichever of the two lands second reads the non-goal from main and changes only its own row's words. The design's intention sentence that DIRECTORY-008 landed is edited only to take row 07 out, so that it names row 06 alone and ends 'row 06 stays the non-goal recorded below, and this sentence does not promise it.'; row 06's words in it are likewise draft DIRECTORY-010's. Two of DIRECTORY-008's recorded acceptance checks no longer hold after this card: its R3 check that the intention ends with 'and this sentence promises neither row.', and its R3 check that rg -c 'Rows 06 \(connect Cambium\) and 07 \(gate, install and demonstrate the release\)' docs/design/directory/design.json prints 1, which now prints nothing. Each is that card's evidence of what it landed, not a live gate, and the lead's ruling to lift row 07 is the reason; DIRECTORY-008 is not edited. The dev record names both checks, says each no longer holds, and gives that ruling as the reason. The checklist items C242 to C247 and stories S110 and S111 are numbered after the highest on every open branch (C241, S109).

## Requirements

### R1: Register the release leg as a demand leg

Structure. docs/design/project.json's tree '.' gains one leg, appended after the legs it holds at the commit the build started from: name 'identity-release', command 'sh scripts/identity-gates/release.sh', requires ['tool:sh', 'tool:git', 'tool:cargo', 'tool:rustfmt', 'rust-build'], cadence 'demand'. Every tree and every leg docs/design/project.json holds at that commit stays byte for byte and in its order. docs/design/directory/design.json's gate array is then set, whole, to docs/design/project.json's trees, so that the directory gate carries the same leg and the two are equal, and docs/design/directory/DESIGN.md is re-rendered from it; a leg a blocker registers in docs/design/project.json before the build is carried into the directory gate by that whole copy. The two are equal when this brief is written because this brief replaced the directory design's gate wholesale with docs/design/project.json's trees: the directory cluster's gate runs exactly what the project's gate runs, and the render, validate and coverage legs it carried before are run by the project's design leg, sh scripts/design/gate.sh, over docs/design/directory. CLAUDE.md's section 'Gates before any commit' gains one line, after 'All five clean. No exceptions.', naming the identity-release leg as a demand leg in docs/design/project.json, run by the identity release card and not by every land. .land/gates.sh is not changed.

Behaviour of scripts/identity-gates/release.sh, run from the repository root. The script does its own fetching; the caller fetches nothing. IF git status --porcelain prints anything, THEN THE SYSTEM SHALL exit 1 with a stderr line containing 'working tree is not clean' and run no leg. Otherwise THE SYSTEM SHALL run git fetch origin in the repository and git -C vendor/rauthy fetch origin ablative in the submodule, and IF either exits non-zero, THEN THE SYSTEM SHALL exit 1 with a stderr line containing 'fetch failed' and the failing command, and run no leg. IF the commit git ls-tree HEAD vendor/rauthy records is not an ancestor of vendor/rauthy's origin/ablative after that fetch, THEN THE SYSTEM SHALL exit 1 with a stderr line containing 'is not on ablative' and the pinned commit, and run no leg. IF git branch -r --contains HEAD prints nothing after that fetch, THEN THE SYSTEM SHALL exit 1 with a stderr line containing 'is not pushed' and the HEAD commit, and run no leg. The four refusals are checked in that order and the first that fires is the only one reported. Otherwise THE SYSTEM SHALL run the six lys legs in this order: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, run every leg even after a red one, send each leg's own output to stderr, print to stdout exactly one JSON object per leg on its own line with exactly the keys repository ('lys'), ref (the full HEAD commit), kind ('test' for cargo test --workspace --all-features and 'gate' for each of the other five), command (the leg's command line) and exit_status (the leg's exit status as an integer), and exit 0 only when every leg exited 0, else 1. THE SYSTEM SHALL NOT create, modify or delete any file in the repository's working tree or vendor/rauthy's working tree; the refs and objects its two fetches store under git's own directories, and the build output under the cargo target directory, which git status --porcelain does not report, are the only writes it makes. THE SYSTEM SHALL NOT add a leg to .land/gates.sh, SHALL NOT change or remove any leg docs/design/project.json holds at the commit the build started from, SHALL NOT leave the directory design's gate different from docs/design/project.json's trees, SHALL NOT restore the directory design's former 'docs' tree or any place:here leg, SHALL NOT register a leg another row's brief needs, SHALL NOT fall back to a local build when a refusal fires, and SHALL NOT print a credential, key or token.

**Acceptance:**
- From the repository root, with <base> the commit the build started from: python3 -c "import json,subprocess; b=json.loads(subprocess.run(['git','show','<base>:docs/design/project.json'],capture_output=True,text=True,check=True).stdout)['trees']; t=json.load(open('docs/design/project.json'))['trees']; l=[x for x in t[0]['legs'] if x['name']=='identity-release']; print(len(l), l[0]['cadence'], l[0]['command'], l[0]['requires'], t[0]['legs'][-1]['name'], [dict(u, legs=[x for x in u['legs'] if x['name']!='identity-release']) for u in t]==b)" prints 1 demand sh scripts/identity-gates/release.sh ['tool:sh', 'tool:git', 'tool:cargo', 'tool:rustfmt', 'rust-build'] identity-release True.
- From the repository root: python3 -c "import json; print(json.load(open('docs/design/directory/design.json'))['gate'] == json.load(open('docs/design/project.json'))['trees'])" prints True, and sh scripts/design/gate.sh exits 0.
- From the repository root: python3 -c "import json; g=json.load(open('docs/design/directory/design.json'))['gate']; n=[l['name'] for l in g[0]['legs']]; print(g[0]['tree'], 'docs' in [t['tree'] for t in g], 'design' in n, n[-1])" prints . False True identity-release, and rg -c 'place:here' docs/design/directory/design.json prints nothing.
- From the repository root on the build branch: git diff --no-ext-diff --name-only <base>..HEAD -- .land/gates.sh prints nothing, and git diff --no-ext-diff --numstat <base>..HEAD -- CLAUDE.md prints 1 added and 0 deleted, where <base> is the commit the build started from; rg -c 'identity-release' CLAUDE.md prints 1.
- In a scratch clone at the build commit, with vendor/rauthy initialised and checked out by git submodule update --init vendor/rauthy before anything else is changed, so that every fetch the script makes in vendor/rauthy runs in the submodule and never in the parent repository, and then one tracked file edited and not committed: sh scripts/identity-gates/release.sh exits 1, its stderr contains 'working tree is not clean', and its stdout is empty.
- In a scratch clone at the build commit, with vendor/rauthy initialised and checked out by git submodule update --init vendor/rauthy before anything else is changed, so that every fetch the script makes in vendor/rauthy runs in the submodule and never in the parent repository, and then the repository's origin remote URL set to a directory that does not exist: sh scripts/identity-gates/release.sh exits 1, its stderr contains 'fetch failed', and its stdout is empty.
- In a scratch clone at the build commit, with vendor/rauthy initialised and checked out by git submodule update --init vendor/rauthy before anything else is changed, so that every fetch the script makes in vendor/rauthy runs in the submodule and never in the parent repository, and then a local commit that moves vendor/rauthy to a commit made locally inside the submodule, with the submodule checked out at that commit: sh scripts/identity-gates/release.sh exits 1, its stderr contains 'is not on ablative' and does not contain 'is not pushed', and its stdout is empty.
- In a scratch clone at the build commit, with vendor/rauthy initialised and checked out by git submodule update --init vendor/rauthy before anything else is changed, so that every fetch the script makes in vendor/rauthy runs in the submodule and never in the parent repository, and then one local commit that changes only a comment line in a tracked text file: sh scripts/identity-gates/release.sh exits 1, its stderr contains 'is not pushed' and the output of git rev-parse HEAD, and its stdout is empty.
- In a clean clone at the pushed build commit, with vendor/rauthy initialised and checked out by git submodule update --init vendor/rauthy: sh scripts/identity-gates/release.sh exits 0 and its stdout is exactly 6 lines; python3 reading them prints, for each line in order, sorted keys ['command', 'exit_status', 'kind', 'ref', 'repository'], repository 'lys', ref equal to git rev-parse HEAD, exit_status 0, and commands equal, in order, to the six commands of CLAUDE.md's gate list; the kinds are, in order, gate, gate, gate, test, gate, gate; git status --porcelain prints nothing afterwards.

**Files:**
- create: scripts/identity-gates/release.sh
- modify: docs/design/project.json
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md
- modify: CLAUDE.md

**Checklist:**
- C242 — The identity-release leg is registered once, as a demand-cadence leg in docs/design/project.json running scripts/identity-gates/release.sh, named in one line of CLAUDE.md and mirrored in the directory design's gate, and .land/gates.sh is unchanged.

**Stories:**
- S110 (Release reviewer, Decides from the record whether the release proof is complete) — As the release reviewer, I want every command, result, ref and hash recorded with every unmet requirement named, so that completion rests on recorded evidence and never on a health check.

### R2: Gate the exact pushed refs and record every command

WHEN the release is gated, THE SYSTEM SHALL gate three exact pushed commits: the lys commit, by running there the identity-release leg and then every other leg of cadence 'round' or 'demand' that docs/design/project.json registers at that commit, in every tree, each from its tree's directory and in the file's order, among them the design leg and the identity and frontend legs the DIRECTORY-002 and DIRECTORY-005 blockers register, and excluding the legs of the Rauthy venue leg registration that the DIRECTORY-004 blocker lands in that file, which are the Rauthy commit's legs and are recorded only as jsonl lines with repository 'rauthy' and the 'rauthy' ref, so that no Rauthy leg is run or recorded twice; the Rauthy commit, the one git ls-tree records for vendor/rauthy at that lys commit, by the fork's pinned baseline legs plus its counted link, migration and audit regressions; and the Cambium commit, by every leg of Cambium's gates.json at that commit, run by Cambium's own gate. The Cambium commit SHALL descend from the landed commit of row 06's Cambium half, card v0CuMstE: THE SYSTEM SHALL run git merge-base --is-ancestor <v0CuMstE's landed commit> <cambium commit> in a Cambium clone and record it. The lys commit SHALL descend from the commit the build started from, which holds the landed blockers: THE SYSTEM SHALL run git merge-base --is-ancestor <base> <lys commit> in the repository, where <base> is that commit, and record it. THE SYSTEM SHALL create docs/design/identity/reports/IDENTITY-001-commands.jsonl holding one JSON object per line for every command run for the release, with exactly the keys repository ('lys', 'rauthy' or 'cambium'), ref (a full 40-hex commit), kind, one of 'gate' (a gate leg that runs no tests), 'test' (a gate leg that runs tests), 'baseline' (a Rauthy comparison run at the upstream release commit), 'ancestry' (a git merge-base --is-ancestor check), 'install', 'backup', 'service' (a stop or a start of the staged service) and 'restore', command (the command line) and exit_status (an integer). The kind is fixed when the line is written, and a command is classed as running tests by its kind alone. Every command run for the release is a line there: the gate legs of the three repositories, of kind 'gate' or 'test'; each Rauthy comparison run, of kind 'baseline'; this requirement's Cambium ancestry check, with repository 'cambium' and ref the Cambium commit, this requirement's lys ancestry check, with repository 'lys' and ref the lys commit, and each ancestry check of R3, carrying the repository and release commit it checks against, of kind 'ancestry'; each install command of R4, carrying the repository and commit of the artifact it installs, of kind 'install'; and each backup, service stop, service start and restore command of R4, with repository 'lys' and the lys commit, of kinds 'backup', 'service' and 'restore'. The lys lines of kind 'gate' or 'test' are, first, the identity-release leg's six stdout lines, as printed and in its order, and then one line for each other leg docs/design/project.json registers at the lys commit, other than the legs of the Rauthy venue leg registration, in the file's order, with repository 'lys', ref the lys commit and command the leg's command. The kind of each such line is fixed by the leg, never by its command line: 'test' exactly for the leg named 'tests' and for any leg whose registering brief, the brief whose requirement adds that leg to docs/design/project.json, states in its words that the leg runs tests; 'gate' for every other leg, the design leg included. THE SYSTEM SHALL create docs/design/identity/reports/IDENTITY-001-release.md whose second-level headings are exactly, in order: '## Refs', '## Venue legs', '## Review', '## Install', '## Receipts', '## Standalone acceptance', '## Not met', '## Reserved acts', '## Status'. Under '## Refs' it writes six lines, 'lys <commit>', 'rauthy <commit>', 'cambium <commit>', 'rauthy upstream <commit>', the upstream release commit the fork is based on as IDENTITY-002's landed rebase names it, each a full 40-hex commit, 'v0CuMstE <landed commit> is-ancestor: <status>', where <status> is the exit status of the Cambium ancestry check, and 'base <base> is-ancestor: <status>', where <status> is the exit status of the lys ancestry check. Under '## Venue legs' it writes, for every line <n> of the jsonl of kind 'gate', 'test' or 'baseline', a line starting 'commands.jsonl:<n>' with its repository, command and result; after the line of every jsonl line of kind 'test' or 'baseline', one line per test target, written exactly 'target: <target> passed <p> failed <f> ignored <i>' with the three counts as integers, followed by one line per failing test, written exactly 'failed test: <test name>'; for a leg that did not run, a line 'not run: <leg> — <reason>'. A Rauthy failure is upstream's only when the same command, run at the upstream release commit the fork is based on, fails the same test: every test named on a 'failed test:' line of the failing run is also named on a 'failed test:' line of the comparison run. That comparison run is its own jsonl line of kind 'baseline' with repository 'rauthy' and ref the 'rauthy upstream' commit under '## Refs', and is a comparison, not a gate of that commit. Each such failure is written on one line 'upstream: commands.jsonl:<n> baseline commands.jsonl:<m>', and each lint allowance the fork carries from upstream on one line starting 'upstream: allowance', never as local policy. A jsonl line of a kind other than 'baseline' and 'ancestry' with a non-zero exit_status is allowed only for a Rauthy command with such an 'upstream:' line; any other is a leg not green and is named under '## Not met' in the form this requirement fixes below. A failing test of the fork not named for its comparison run is the fork's and is never excused as upstream's. The Rauthy legs gated are exactly the legs of the Rauthy venue leg registration that the DIRECTORY-004 blocker lands in docs/design/project.json: the jsonl lines with repository 'rauthy', ref the 'rauthy' commit under '## Refs' and kind 'gate' or 'test', counted by distinct command, equal those legs, counted as Cambium's gates.json legs are, and at least one such line exists. At least one such line of kind 'test' carries in its block under '## Venue legs' the four lines 'target: id001_link_pair ...', 'target: id001_link_refusal ...', 'target: id001_link_migration ...' and 'target: id001_link_audit ...', in the target line format above, for the link, migration and audit regressions under vendor/rauthy/tests/identity_links (ID001_LINK_PAIR, ID001_LINK_REFUSAL, ID001_LINK_MIGRATION and ID001_LINK_AUDIT). Under '## Review' it records an independent reviewer on one line 'reviewer: <name>', the implementer on one line 'implementer: <name>', the two names different, and one verdict each on authentication, linking, actor attribution and crash outcomes, on lines starting 'authentication:', 'linking:', 'actor attribution:' and 'crash outcomes:'. IF a required venue leg has no workflow to run it, or did not run, THEN THE SYSTEM SHALL name it under '## Not met' of docs/design/identity/reports/IDENTITY-001-release.md as a blocker. Every line under '## Not met' has the fixed form '<requirement> — met by: <act> — performed by: <role>', naming the act that meets it and who performs it; the <requirement> of a live leg not run is the leg's name as it stands on its 'not run:' line under '## Venue legs', of a leg not green 'leg not green: commands.jsonl:<n>', and of a receipt its identifier. THE SYSTEM SHALL NOT replace such a leg with a local release build or a pass claim. THE SYSTEM SHALL NOT record a commit that is not on its repository's remote, SHALL NOT gate a Rauthy commit other than the vendor/rauthy pin, SHALL NOT leave a leg docs/design/project.json registers at the lys commit, other than identity-release and the legs of the Rauthy venue leg registration, without its lys jsonl line, SHALL NOT record a leg of the Rauthy venue leg registration on a line with repository 'lys', SHALL NOT record a lys gate or test line after the identity-release leg's six for a command no such leg holds, SHALL NOT leave a registered Rauthy leg without its jsonl line, SHALL NOT record a Rauthy gate or test line for a command the registration does not hold, SHALL NOT gate a Cambium commit that does not descend from v0CuMstE's landed commit, SHALL NOT gate a lys commit that does not descend from the commit the build started from, SHALL NOT excuse a Rauthy failure against a comparison run at any ref other than the 'rauthy upstream' commit, SHALL NOT classify a command as running tests by matching its command line, SHALL NOT give a lys line after the identity-release leg's six a kind other than the one its leg's name and registering brief fix, SHALL NOT cherry-pick into the fork, and SHALL NOT change any lys cryptographic format or wire contract.

**Acceptance:**
- From the repository root: python3 -c "import json; L=[json.loads(x) for x in open('docs/design/identity/reports/IDENTITY-001-commands.jsonl')]; print(sorted({tuple(sorted(x)) for x in L}), sorted({x['repository'] for x in L}), sorted({x['kind'] for x in L} - {'gate','test','baseline','ancestry','install','backup','service','restore'}), sorted({x['repository'] for x in L if x['exit_status']!=0 and x['kind'] not in ('baseline','ancestry')} - {'rauthy'}))" prints [('command', 'exit_status', 'kind', 'ref', 'repository')] ['cambium', 'lys', 'rauthy'] [] [].
- From the repository root: for every line <n> of docs/design/identity/reports/IDENTITY-001-commands.jsonl whose kind is neither 'baseline' nor 'ancestry' and whose exit_status is non-zero, the '## Venue legs' section of docs/design/identity/reports/IDENTITY-001-release.md holds a line 'upstream: commands.jsonl:<n> baseline commands.jsonl:<m>', where jsonl line <m> has kind 'baseline', repository 'rauthy', the same command as line <n>, ref equal to the 'rauthy upstream' commit under '## Refs', and a non-zero exit_status; and every test named on a 'failed test: ' line in the block of 'commands.jsonl:<n>' (the lines after it and before the next line starting 'commands.jsonl:') is also named on a 'failed test: ' line in the block of 'commands.jsonl:<m>'.
- From the repository root: the '## Refs' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly one line matching '^rauthy upstream [0-9a-f]{40}$', its commit equals the upstream release commit named by IDENTITY-002's landed rebase, in vendor/rauthy git merge-base --is-ancestor <that commit> <rauthy commit> exits 0, and every line of docs/design/identity/reports/IDENTITY-001-commands.jsonl of kind 'baseline' carries that commit as its ref.
- From the repository root: rg -o '^## .+$' docs/design/identity/reports/IDENTITY-001-release.md prints exactly these 9 lines in this order: ## Refs, ## Venue legs, ## Review, ## Install, ## Receipts, ## Standalone acceptance, ## Not met, ## Reserved acts, ## Status.
- From the repository root: the rauthy commit under '## Refs' of docs/design/identity/reports/IDENTITY-001-release.md equals the third field of git ls-tree <lys commit> vendor/rauthy, where <lys commit> is the commit on the Refs 'lys' line, and every jsonl line with repository 'lys' and kind other than 'ancestry' carries that lys commit as its ref.
- From the repository root: git branch -r --contains <lys commit> prints at least one line; in vendor/rauthy, git branch -r --contains <rauthy commit> prints a line ending in 'origin/ablative'; in a Cambium clone, git branch -r --contains <cambium commit> prints at least one line.
- In a Cambium clone: the '## Refs' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly one line starting 'v0CuMstE ', its commit equals the landed_commit in the execution record of v0CuMstE's brief file on Cambium main, and it ends 'is-ancestor: 0'; git merge-base --is-ancestor <that commit> <cambium commit> exits 0; and docs/design/identity/reports/IDENTITY-001-commands.jsonl holds a line with kind 'ancestry', repository 'cambium', ref <cambium commit>, command 'git merge-base --is-ancestor <that commit> <cambium commit>' and exit_status 0.
- From the repository root: the '## Refs' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly one line starting 'base ', its commit equals the commit the build started from, and it ends 'is-ancestor: 0'; git merge-base --is-ancestor <that commit> <lys commit> exits 0; and docs/design/identity/reports/IDENTITY-001-commands.jsonl holds a line with kind 'ancestry', repository 'lys', ref <lys commit>, command 'git merge-base --is-ancestor <that commit> <lys commit>' and exit_status 0.
- In a Cambium clone at <cambium commit>: the number of legs in gates.json equals the number of distinct commands on jsonl lines with repository 'cambium', ref <cambium commit> and kind 'gate' or 'test' in docs/design/identity/reports/IDENTITY-001-commands.jsonl.
- From the repository root at <lys commit>: the number of legs of the Rauthy venue leg registration that the DIRECTORY-004 blocker lands in docs/design/project.json is at least 1 and equals the number of distinct commands on jsonl lines with repository 'rauthy', ref the 'rauthy' commit under '## Refs' of docs/design/identity/reports/IDENTITY-001-release.md and kind 'gate' or 'test' in docs/design/identity/reports/IDENTITY-001-commands.jsonl, and the set of those commands equals the set of the registration's leg commands.
- From the repository root: at least one line <n> of docs/design/identity/reports/IDENTITY-001-commands.jsonl with repository 'rauthy' and kind 'test' has, in the block of its 'commands.jsonl:<n>' line under '## Venue legs' of docs/design/identity/reports/IDENTITY-001-release.md, exactly one line matching each of '^target: id001_link_pair passed \d+ failed \d+ ignored \d+$', '^target: id001_link_refusal passed \d+ failed \d+ ignored \d+$', '^target: id001_link_migration passed \d+ failed \d+ ignored \d+$' and '^target: id001_link_audit passed \d+ failed \d+ ignored \d+$'; a report with no Rauthy line of kind 'test' fails this line.
- From the repository root: the first six lines of docs/design/identity/reports/IDENTITY-001-commands.jsonl with repository 'lys' and kind 'gate' or 'test' have commands equal, in order, to the six commands of CLAUDE.md's gate list, and the one of them whose command is cargo test --workspace --all-features has kind 'test'.
- From the repository root at <lys commit>: the number of legs of cadence 'round' or 'demand' in every tree of docs/design/project.json, excluding the leg named 'identity-release' and the legs of the Rauthy venue leg registration that the DIRECTORY-004 blocker lands there, is at least 7 and equals the number of distinct commands on the lines of docs/design/identity/reports/IDENTITY-001-commands.jsonl with repository 'lys', ref <lys commit> and kind 'gate' or 'test' after the first six such lines, and the set of those commands equals the set of those legs' commands; a report with no line for the design leg's command sh scripts/design/gate.sh fails this line.
- From the repository root at <lys commit>: for each line of docs/design/identity/reports/IDENTITY-001-commands.jsonl with repository 'lys', ref <lys commit> and kind 'gate' or 'test' after the first six such lines, take the leg of docs/design/project.json with that line's command; the line's kind is 'test' when that leg is named 'tests' or its registering brief, the brief whose requirement adds it to docs/design/project.json, states in its words that it runs tests, and 'gate' for every other leg; the line whose command is cargo test --workspace --all-features has kind 'test', the line whose command is sh scripts/design/gate.sh has kind 'gate', and a report with any such line whose kind differs from its leg's fails this line.
- From the repository root: for every line <n> of docs/design/identity/reports/IDENTITY-001-commands.jsonl of kind 'gate', the '## Venue legs' section of docs/design/identity/reports/IDENTITY-001-release.md holds a line starting 'commands.jsonl:<n>' that contains that line's repository and command.
- From the repository root: for every line <n> of docs/design/identity/reports/IDENTITY-001-commands.jsonl of kind 'test' or 'baseline', the '## Venue legs' section of docs/design/identity/reports/IDENTITY-001-release.md holds a line starting 'commands.jsonl:<n>' followed, before the next line starting 'commands.jsonl:', by at least one line matching '^target: \S+ passed \d+ failed \d+ ignored \d+$'.
- From the repository root: in the '## Venue legs' section of docs/design/identity/reports/IDENTITY-001-release.md, within the block of each 'commands.jsonl:<n>' line of kind 'test' or 'baseline', the number of lines matching '^failed test: \S.*$' equals the sum of <f> over its 'target: ' lines.
- From the repository root: every line of the '## Venue legs' section of docs/design/identity/reports/IDENTITY-001-release.md containing 'allowance' starts with 'upstream:', and no line of the report contains 'local policy'.
- From the repository root: the '## Review' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly one line starting 'reviewer: ' and exactly one starting 'implementer: ', the text after 'reviewer: ' differs from the text after 'implementer: ', and the section holds one line starting 'authentication:', one 'linking:', one 'actor attribution:' and one 'crash outcomes:'.

**Files:**
- create: docs/design/identity/reports/IDENTITY-001-commands.jsonl
- create: docs/design/identity/reports/IDENTITY-001-release.md

**Checklist:**
- C243 — Every required venue leg is recorded green on the exact pushed Lys, Rauthy and Cambium refs in IDENTITY-001-commands.jsonl and the release report, and a missing leg is named as a blocker.

**Stories:**
- S110 (Release reviewer, Decides from the record whether the release proof is complete) — As the release reviewer, I want every command, result, ref and hash recorded with every unmet requirement named, so that completion rests on recorded evidence and never on a health check.

### R3: Verify the row 03 and row 05 install receipts against the release refs

WHEN the refs of R2 are recorded, THE SYSTEM SHALL, under '## Receipts' of docs/design/identity/reports/IDENTITY-001-release.md, record each of ID001_LINK_LIVE and ID001_DIRECTORY_LIVE that has a receipt on exactly one line, however many commits the receipt names, written 'receipt: <identifier> post <post identifier> test-keyed', followed, for each commit the receipt names, by '; <repository> <commit> is-ancestor: <status>', where <status> is the exit status of git merge-base --is-ancestor <commit> <release commit> run against the commit of that repository under '## Refs', and, for each artifact hash the receipt names, by '; artifact <sha256>'. Each such check SHALL be recorded in docs/design/identity/reports/IDENTITY-001-commands.jsonl as a line of kind 'ancestry' with that repository, ref the release commit, command 'git merge-base --is-ancestor <commit> <release commit>' and its exit status. IF a receipt is absent, or one of its commits is not an ancestor of the release commit, THEN THE SYSTEM SHALL name its identifier on a line under '## Not met', in the form R2 fixes. A receipt counts as verified only by standing on its receipt line with every 'is-ancestor: ' status 0; no other verified marker is written. THE SYSTEM SHALL NOT produce a signed Lys receipt under a production key, and SHALL NOT record a receipt without marking it test-keyed.

**Acceptance:**
- From the repository root: the '## Receipts' section of docs/design/identity/reports/IDENTITY-001-release.md holds at most one line starting 'receipt: ID001_LINK_LIVE post ', at most one starting 'receipt: ID001_DIRECTORY_LIVE post ', and no other line starting 'receipt: '; each such line contains 'test-keyed' and at least one 'is-ancestor: ' token.
- From the repository root: for each of ID001_LINK_LIVE and ID001_DIRECTORY_LIVE with no receipt line under '## Receipts' of docs/design/identity/reports/IDENTITY-001-release.md, or with an 'is-ancestor: ' token on its receipt line not followed by '0', the '## Not met' section holds a line containing that identifier.
- From the repository root: for each '; <repository> <commit> is-ancestor: <status>' entry on a receipt line, git merge-base --is-ancestor <commit> <the commit of that repository under '## Refs'> exits with <status>.
- From the repository root: for each '; <repository> <commit> is-ancestor: <status>' entry on a receipt line, docs/design/identity/reports/IDENTITY-001-commands.jsonl holds a line with kind 'ancestry', that repository, ref the commit of that repository under '## Refs', command 'git merge-base --is-ancestor <commit> <that ref>' and exit_status equal to <status>.
- From the repository root: rg -c 'verified' on the '## Receipts' section of docs/design/identity/reports/IDENTITY-001-release.md prints nothing.

**Files:**
- modify: docs/design/identity/reports/IDENTITY-001-release.md
- modify: docs/design/identity/reports/IDENTITY-001-commands.jsonl

**Checklist:**
- C244 — The row 03 and row 05 live install receipts are verified against the release refs and each is marked test-keyed.

**Stories:**
- S110 (Release reviewer, Decides from the record whether the release proof is complete) — As the release reviewer, I want every command, result, ref and hash recorded with every unmet requirement named, so that completion rests on recorded evidence and never on a health check.

### R4: Stage the install with its backups, a service stop and start, a restore and a rollback that respects migrated schemas

WHEN every line of docs/design/identity/reports/IDENTITY-001-commands.jsonl of kind 'gate' or 'test', and each of R2's own two ancestry lines (the Cambium check against v0CuMstE's landed commit and the lys check against the commit the build started from), has exit_status 0 or is a Rauthy line cited on an 'upstream: commands.jsonl:<n>' line under '## Venue legs', and every one of those lines precedes every line of kind 'install', THE SYSTEM SHALL install the artifacts built from R2's lys and Rauthy commits on the node the operator names, keyed with a test service key, against the one PostgreSQL database (ADR-005), and SHALL take, before any migration runs, a backup of the database and of every cache, key and configuration file the deployment README lists as part of a restore. After the install it SHALL read one directory record, an identity ID, on the staged install's record detail screen, take a backup of the migrated database, stop the staged service, restore the database from that migrated backup, start the staged service again, and read the same record again on the same screen, in that order. The two reads are observations on the screen, not commands. Each install, backup, service stop, service start and restore command is a line of docs/design/identity/reports/IDENTITY-001-commands.jsonl as R2 fixes it. Under '## Install' of docs/design/identity/reports/IDENTITY-001-release.md THE SYSTEM SHALL record, one per line and in these formats: 'node: <name as the operator gives it>'; for each installed artifact, 'artifact: <name> sha256 <64-hex>'; 'installed: lys <commit>' and 'installed: rauthy <commit>', equal to R2's; 'service key: <public-key fingerprint> test-keyed'; for the pre-migration database dump, 'backup: database <file> sha256 <64-hex> commands.jsonl:<n>'; for the migrated database dump, 'backup: migrated database <file> sha256 <64-hex> commands.jsonl:<n>'; and for each other backup file, 'backup: file <file> sha256 <64-hex> commands.jsonl:<n>', each citing the jsonl line of kind 'backup' that took it; 'first database start: commands.jsonl:<n>', citing the first jsonl line of kind 'install' that starts a Rauthy or lys binary against the database, which every jsonl line of kind 'backup' precedes except the one the 'backup: migrated database' line cites, which follows it and precedes the service stop; 'rollback: restore the pre-migration backup with the binaries it was taken under'; 'service stop: commands.jsonl:<n>', 'restore: commands.jsonl:<n> from <file>', naming the file on the 'backup: migrated database' line, and 'service start: commands.jsonl:<n>'; 'record read: before <identity ID> after <identity ID>', the record read before the stop and read again after the start; 'issuer: <address>' for the staged issuer; and the handover to the Cambium runtime owner on one line written exactly 'handover (not an install): ' followed by the staged issuer's address and the post identifier of each install receipt that R3 recorded under '## Receipts', so that it is recorded as a handover and not as an install. deploy/identity/README.md SHALL gain a section '## Release rollback' stating that a rollback restores the pre-migration backup together with the binaries it was taken under, and that an older Rauthy binary is never launched against a database a newer one has migrated. THE SYSTEM SHALL NOT start the install before the lines the trigger reads are all recorded, SHALL NOT restore from the pre-migration backup in the stop, restore and start, SHALL NOT count the restore's exit status alone as the data surviving it, SHALL NOT launch an older Rauthy binary against a forward-only migrated database, SHALL NOT generate, load or name a production signing key, SHALL NOT stand up a staging Cambium, SHALL NOT cut over or restart any production Cambium, SHALL NOT change any file under surface/identity or crates/lys-identity-server, and SHALL NOT write a secret, token or private key value in any document.

**Acceptance:**
- From the repository root: in docs/design/identity/reports/IDENTITY-001-commands.jsonl, every line of kind 'install' has a line number greater than every line of kind 'gate' or 'test' and than R2's two ancestry lines (the one with repository 'cambium' whose command names the commit on the Refs 'v0CuMstE' line, and the one with repository 'lys' whose command names the commit on the Refs 'base' line); and each of those lines has exit_status 0 or is a line with repository 'rauthy' whose number <n> is cited on a line 'upstream: commands.jsonl:<n> baseline commands.jsonl:<m>' under '## Venue legs' of docs/design/identity/reports/IDENTITY-001-release.md.
- From the repository root: rg -c '^## Release rollback$' deploy/identity/README.md prints 1, and the text of that section contains 'older Rauthy binary' and 'pre-migration backup'.
- From the repository root: the '## Install' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly one line starting 'node: ', one 'installed: lys ' and one 'installed: rauthy ', and the commits on the two 'installed:' lines equal the 'lys' and 'rauthy' commits under '## Refs'.
- From the repository root: the '## Install' section of docs/design/identity/reports/IDENTITY-001-release.md holds at least one line starting 'artifact: ', each matching ' sha256 [0-9a-f]{64}$'; exactly one line starting 'backup: database ' and exactly one starting 'backup: migrated database ', and every line starting 'backup: ' matches '^backup: (database|migrated database|file) \S+ sha256 [0-9a-f]{64} commands\.jsonl:\d+$'; exactly one line matches '^service key: \S+ test-keyed$'; and exactly one line reads 'rollback: restore the pre-migration backup with the binaries it was taken under'.
- From the repository root: the '## Install' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly one line 'first database start: commands.jsonl:<s>'; line <s> of docs/design/identity/reports/IDENTITY-001-commands.jsonl has kind 'install'; the line each 'backup: ' line cites has kind 'backup'; every line of docs/design/identity/reports/IDENTITY-001-commands.jsonl of kind 'backup' other than the one the 'backup: migrated database' line cites has a line number less than <s>; and the line the 'backup: migrated database' line cites has a line number greater than <s> and less than the line 'service stop: ' cites.
- From the repository root: the '## Install' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly one line 'service stop: commands.jsonl:<a>', one 'restore: commands.jsonl:<b> from <file>' and one 'service start: commands.jsonl:<c>', with a < b < c; lines <a> and <c> of docs/design/identity/reports/IDENTITY-001-commands.jsonl have kind 'service', line <b> has kind 'restore', all three have exit_status 0, and <file> equals the file on the 'backup: migrated database' line.
- From the repository root: the '## Install' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly one line matching '^record read: before (\S+) after (\S+)$', and its two captured identity IDs are equal.
- From the repository root: every line of docs/design/identity/reports/IDENTITY-001-commands.jsonl with kind 'install' carries repository 'lys' or 'rauthy' and, as ref, the commit of that repository under '## Refs' of docs/design/identity/reports/IDENTITY-001-release.md.
- From the repository root: the '## Install' section of docs/design/identity/reports/IDENTITY-001-release.md contains exactly one line starting 'issuer: ' and exactly one line starting 'handover (not an install): ', and the handover line contains the address on the 'issuer: ' line and the post identifier on each receipt line under '## Receipts'.
- From the repository root on the build branch: git diff --no-ext-diff --name-only <base>..HEAD -- surface/identity crates/lys-identity-server prints nothing, where <base> is the commit the build started from.

**Files:**
- modify: deploy/identity/README.md
- modify: docs/design/identity/reports/IDENTITY-001-release.md
- modify: docs/design/identity/reports/IDENTITY-001-commands.jsonl

**Checklist:**
- C245 — The staged install runs the tested refs on the node the operator names under a test service key, with its backups and a rollback that never launches an older Rauthy binary against a forward-only migrated database recorded.

**Stories:**
- S111 (Operator, Installs and runs the standalone identity product) — As the operator, I want the exact release installed on a node I name and shown to me as a staging install, so that I can accept the product standalone before anything is cut over.

### R5: Record standalone acceptance on the staged install before any cutover

WHEN the staged install of R4 is recorded, THE SYSTEM SHALL record under '## Standalone acceptance' of docs/design/identity/reports/IDENTITY-001-release.md, with Cambium and Manifold absent from the staged install's configuration (ADR-004), one per line and in these formats: for each configuration file of the staged install, 'configuration: <file> sha256 <64-hex>', none of which names a Cambium or Manifold address or host, redirect URIs included, with no exclusion and no allowlist; the staged install runs configure exactly as DIRECTORY-002 R2 landed it, so its client list holds exactly three clients: Rauthy's own built-in admin client and the two clients that configure registers, the platform client and the Cambium OIDC client, each by the client ID the install's configuration gives it, the Cambium client carrying the redirect address the install's own configuration supplies for the staged Cambium, which is none of the Cambium or Manifold addresses and hosts the operator names; whether the staged Cambium is reachable is not asserted, and the Cambium client registered at the production install Cambium signs in through, with Cambium's own addresses at that time, is the production install's act and not this row's; 'staging statement: <what was said>', recording that the demonstration opened by stating the operator was looking at a staging install, the text after 'staging statement: ' containing 'staging install'; 'Google: <identity ID>' and 'GitHub: <identity ID>', the enduring identity ID of the person record each provider's test account resolves to, the same ID for both; 'agent: <agent ID> responsible <person ID> receipt <receipt identifier> test-keyed', the registered agent's recorded creation with its responsible person and its creation receipt; for each installed screen DIRECTORY-005's journey names (sign-in, directory, record detail, register and edit agent, linked-account entry point, signed change history), 'screen: <name> observed: <the action taken> — <what the screen showed>', naming a screen action and never a command or a commands.jsonl line; and one line 'no Cambium cutover has occurred'. THE SYSTEM SHALL NOT record acceptance after a cutover, SHALL NOT count a health check as a screen observed, SHALL NOT cite a commands.jsonl line whose command contains 'health' anywhere in this section, SHALL NOT add, delete or change a client on the staged install after configure, SHALL NOT register any client configure does not register, SHALL NOT exclude any file, key or redirect URI from the configuration check, SHALL NOT show a production certificate as issued (ADR-008), and SHALL NOT produce a signed Lys receipt under a production key.

**Acceptance:**
- From the repository root: the '## Standalone acceptance' section of docs/design/identity/reports/IDENTITY-001-release.md contains exactly one line starting 'staging statement: ', whose text after that prefix contains 'staging install', exactly one line 'no Cambium cutover has occurred', and exactly one line matching '^agent: \S+ responsible \S+ receipt \S+ test-keyed$'.
- From the repository root: the '## Standalone acceptance' section of docs/design/identity/reports/IDENTITY-001-release.md contains exactly one line matching '^Google: \S+$' and exactly one matching '^GitHub: \S+$', and the second whitespace-separated field of the two lines is equal.
- From the repository root: the '## Standalone acceptance' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly six lines starting 'screen: ', whose names are, in any order, 'sign-in', 'directory', 'record detail', 'register and edit agent', 'linked-account entry point' and 'signed change history', each line containing 'observed: ' followed by at least one word.
- From the repository root: for every 'commands.jsonl:<n>' token in the '## Standalone acceptance' section of docs/design/identity/reports/IDENTITY-001-release.md, line <n> of docs/design/identity/reports/IDENTITY-001-commands.jsonl has a command not containing 'health'; and on every line starting 'screen: ', the text after 'observed: ' contains ' — ' with at least one word on each side and contains neither 'commands.jsonl:' nor 'health'.
- From the repository root: the '## Standalone acceptance' section of docs/design/identity/reports/IDENTITY-001-release.md holds at least one line matching '^configuration: \S+ sha256 [0-9a-f]{64}$'; on the staged node, for each such line, sha256 of <file> equals the recorded hash, and for each Cambium and Manifold address and host the operator names, rg -i -F <that address or host> <file> prints nothing.
- On the staged node: the staged Rauthy's client list, read through its admin clients listing, holds exactly three clients, whose client IDs are Rauthy's own built-in admin client's and the two client IDs the install's configuration gives DIRECTORY-002 R2's platform client and Cambium OIDC client; and for each Cambium and Manifold address and host the operator names, no redirect URI of any listed client contains it.

**Files:**
- modify: docs/design/identity/reports/IDENTITY-001-release.md

**Checklist:**
- C246 — Standalone acceptance on the staged install is recorded before any cutover, with the staging statement at the demonstration's opening, who the two providers resolve to and the agent's recorded creation.

**Stories:**
- S111 (Operator, Installs and runs the standalone identity product) — As the operator, I want the exact release installed on a node I name and shown to me as a staging install, so that I can accept the product standalone before anything is cut over.

### R6: Name every requirement not met, the reserved acts and the status from evidence

THE SYSTEM SHALL write under '## Not met' of docs/design/identity/reports/IDENTITY-001-release.md one line per requirement not met, each in the fixed form R2 fixes, '<requirement> — met by: <act> — performed by: <role>', with the <requirement> of a live leg not run, a leg not green and a receipt spelled as R2 fixes. The <requirement> of every missing credential is written 'missing credential: <credential> — <leg it blocks>', and of every missing client registration 'missing client registration: <provider>'. WHILE no Cambium cutover has occurred, it SHALL hold the line 'preserved Cambium identities are observed — met by: the Cambium cutover — performed by: the product owner'. It SHALL name the three deferred items by the fixed phrases 'step-2 permissions', 'step-3 broker' and 'context lane work', each on its own line containing 'deferred'; these are the three deferred lines. Outside the '## Status' section those three phrases SHALL appear nowhere else in the report; the '## Status' lines are exempt from this phrase rule, so the second status line may carry revision 5's acceptance line word for word. Under '## Reserved acts' it SHALL name exactly three acts, the production signing key, production receipts and the Cambium cutover, each as the product owner's and each as not performed. Under '## Status' it SHALL write exactly one line per acceptance line of revision 5's Row 07, in that row's order, each 'met: <acceptance line> — evidence: <proof>' or 'not met: <acceptance line>', where <acceptance line> is that line's text as revision 5 writes it and <proof> is the report section, written '## <name>', or the commands.jsonl line, written 'commands.jsonl:<n>', that proves it; The first status line, for 'All required venue legs green on exact refs', starts 'not met: ' whenever '## Venue legs' holds a 'not run:' line or '## Not met' holds a line for a leg, a receipt, a missing credential or a missing client registration; the second, for 'Health alone is not completion', starts 'not met: ' whenever '## Not met' holds the preserved Cambium identities line. Last comes the literal line 'row 07: complete', present only when every status line starts 'met: ' and '## Not met' holds no line other than the three deferred lines, so the completion line is reachable once every other line under '## Not met' is gone. The section holds no other line. THE SYSTEM SHALL NOT cite a health check as the evidence of any met line, SHALL NOT write 'row 07: complete' in any other case or in any other wording, SHALL NOT claim step-2 permission enforcement, a step-3 broker or the context lane's work, and SHALL NOT change docs/design/identity/briefs/IDENTITY-001.json or docs/design/identity/briefs/IDENTITY-001.md.

**Acceptance:**
- From the repository root: every non-empty line of the '## Not met' section of docs/design/identity/reports/IDENTITY-001-release.md matches '^.+ — met by: .+ — performed by: .+$', and the section holds the line 'preserved Cambium identities are observed — met by: the Cambium cutover — performed by: the product owner'.
- From the repository root: the '## Reserved acts' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly three list lines, containing respectively 'production signing key', 'production receipts' and 'Cambium cutover', each containing 'not performed'.
- From the repository root: for each of the phrases 'step-2 permissions', 'step-3 broker' and 'context lane work', every line of docs/design/identity/reports/IDENTITY-001-release.md containing it, other than the lines of the '## Status' section, lies inside the '## Not met' section and contains 'deferred', and at least one such line exists inside the '## Not met' section; a '## Status' line containing it is not counted against this line.
- From the repository root: for every line of the '## Venue legs' section of docs/design/identity/reports/IDENTITY-001-release.md starting 'not run: <leg> —', the '## Not met' section holds a line containing <leg>; and for each of Google and GitHub with no line starting '<provider>:' under '## Standalone acceptance', the '## Not met' section holds a line starting 'missing client registration: <provider> — met by: '.
- From the repository root: for every line of the '## Venue legs' section of docs/design/identity/reports/IDENTITY-001-release.md starting 'not run: <leg> — ' whose reason names a credential, the '## Not met' section holds a line starting 'missing credential: <credential> — <leg> — met by: '; and on every line of the '## Not met' section starting 'missing credential: ', the text between the first ' — ' and ' — met by: ' is a leg named on a 'commands.jsonl:' or 'not run:' line under '## Venue legs'.
- From the repository root: the '## Status' section of docs/design/identity/reports/IDENTITY-001-release.md holds exactly two lines starting 'met: ' or 'not met: ', the first's text after that prefix starting 'All required venue legs green on exact refs;' and the second's starting 'Health alone is not completion:'; every line starting 'met: ' contains ' — evidence: ', and for every 'commands.jsonl:<n>' such a line cites, line <n> of docs/design/identity/reports/IDENTITY-001-commands.jsonl has a command not containing 'health'.
- From the repository root: in the '## Status' section of docs/design/identity/reports/IDENTITY-001-release.md, the status line whose text starts 'All required venue legs green on exact refs;' starts 'not met: ' whenever the '## Venue legs' section holds a line starting 'not run: ', or the '## Not met' section holds a line starting 'leg not green: ', 'missing credential: ' or 'missing client registration: ', a line containing 'ID001_LINK_LIVE' or 'ID001_DIRECTORY_LIVE', or a line starting with a leg named on a 'not run: ' line.
- From the repository root: in the '## Status' section of docs/design/identity/reports/IDENTITY-001-release.md, the status line whose text starts 'Health alone is not completion:' starts 'not met: ' whenever the '## Not met' section holds the line 'preserved Cambium identities are observed — met by: the Cambium cutover — performed by: the product owner'.
- From the repository root: for every line <n> of docs/design/identity/reports/IDENTITY-001-commands.jsonl whose kind is neither 'baseline' nor 'ancestry', whose exit_status is non-zero, and which no 'upstream: commands.jsonl:<n> ' line under '## Venue legs' cites, the '## Not met' section of docs/design/identity/reports/IDENTITY-001-release.md holds a line starting 'leg not green: commands.jsonl:<n> — met by: '.
- From the repository root: the '## Status' section of docs/design/identity/reports/IDENTITY-001-release.md holds no non-empty line other than those two status lines and the line 'row 07: complete'; that line appears exactly once when both status lines start 'met: ' and the '## Not met' section holds no non-empty line other than the three deferred lines, those containing 'step-2 permissions', 'step-3 broker' or 'context lane work', and not at all otherwise; a report whose '## Not met' holds only those three lines and whose two status lines start 'met: ' carries it.
- From the repository root on the build branch: git diff --no-ext-diff --name-only <base>..HEAD -- docs/design/identity/briefs prints nothing, where <base> is the commit the build started from.

**Files:**
- modify: docs/design/identity/reports/IDENTITY-001-release.md

**Checklist:**
- C247 — Every requirement not met is named in the release report, preserved Cambium identities among them until the cutover, the three reserved acts are named, and no health check is counted as completion.

**Stories:**
- S110 (Release reviewer, Decides from the record whether the release proof is complete) — As the release reviewer, I want every command, result, ref and hash recorded with every unmet requirement named, so that completion rests on recorded evidence and never on a health check.

## Boundaries

- No Cambium file is listed in files and none is written by this card: the install document lands through the Cambium install-document card and row 06's Cambium half through v0CuMstE, each with its own pull request and gate, and neither is opened here (CN4).
- IDENTITY-001.json and IDENTITY-001.md are not changed, and docs/design/identity is not moved, excluded from the gate or registered in docs/design/roadmap.json.
- DIRECTORY-002 to DIRECTORY-006 and DIRECTORY-008 are not edited; a leg another row needs is named as a finding for its owner and never registered here.
- Exactly one leg is registered, identity-release, as a demand leg in docs/design/project.json; .land/gates.sh is not changed.
- No production signing key, no signed Lys receipt under a production key, no production anchor, no staging Cambium, and no Cambium cutover or restart: the production key, production receipts and the cutover are Tom's acts.
- No file under surface/identity or crates/lys-identity-server changes; the staging label is given in the demonstration and the release report only.
- No lys-core cryptographic format or wire contract changes.
- No row's identifiers, estimates or dependency order change, and CN7's ceiling stands.
- Nothing open for Tom is decided, including the nightly-versus-wait Rauthy base and the identity service's name.
- A local release build or pass claim never stands in for a missing venue leg.
- No credential, token or private key value is written in any document, script output or log.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion (CN5).
- The card is built from this brief only after Tom or the lead signs it off on the card.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit), run on the build laptop.
- From the repository root: sh scripts/design/gate.sh exits 0.
- From the repository root: rg -c -F -e 'validate.py" "$cluster"' -e 'check-coverage.py" "$cluster"' -e 'render-cluster.py" "$tmp/$cluster"' scripts/design/gate.sh prints 3 and test -f docs/design/directory/design.json exits 0, so the design leg's loop over every docs/design/*/ with a design.json validates, checks coverage for and renders docs/design/directory.
- In a scratch copy of the repository with 'C242' removed from the top-level checklist array of docs/design/directory/briefs/DIRECTORY-030.json and nothing else changed: sh scripts/design/gate.sh exits 1 and its output contains 'DIRECTORY-030', so the design leg fires on the directory cluster's coverage.
- From the repository root at the pushed release commit: sh scripts/identity-gates/release.sh exits 0 and its six stdout lines equal, in order, the first six lines of docs/design/identity/reports/IDENTITY-001-commands.jsonl with repository 'lys' and kind 'gate' or 'test'.
- From the repository root on the build branch: git diff --no-ext-diff --name-only <base>..HEAD lists only CLAUDE.md, docs/design/project.json, docs/design/directory/design.json, docs/design/directory/DESIGN.md, scripts/identity-gates/release.sh, deploy/identity/README.md, docs/design/identity/reports/IDENTITY-001-release.md and docs/design/identity/reports/IDENTITY-001-commands.jsonl, where <base> is the commit the build started from.
- In the Cambium repository on main: the install document exists at the path the Cambium install-document card landed, docs/identity-install.md as its draft writes, and the release report names that path.
- After the row lands, as a hold point a person performs (CN5): the operator is shown the staged install, told at the opening that it is a staging install, and inspects who the two providers resolve to and the agent's recorded creation; the release report records that it was said.

