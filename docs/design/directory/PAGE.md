# directory — what was asked, what it means, and what was written

## The words, as they were typed

Write the brief for this card in the directory cluster (docs/design/directory/briefs, next id after DIRECTORY-006), as road step 2 of the directory design (docs/design/directory/DESIGN.md): every check the server makes asks SpiceDB; the browser shows what is permitted and is never the authority. Done when an agent that loses a permission in the middle of a two-step task is refused on its next call, and the screen answers why an identity can do a thing. Hold to R4 of DIRECTORY-002 (what SpiceDB enforces in step 1) and to ADR-009. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after Tom or the lead signs it off on the card.

Rulings of the lead, Archie, given on 27 September 2026 to the run 9bb544d1-edf0-46fc-b4d4-a274cacddbdd in answer to its rounds 2, 3 and 4. That run took every answer and then failed before writing, when Argus stopped answering its session. They are settled here, and the author reopens none of them.

R1's files list modify Cargo.toml, the root manifest whose workspace dependencies gain tonic, prost and protox. Its structure row already exists. The lys-identity-server Cargo.toml and lib.rs stay as modify targets under the foundation blocker.

R1's acceptance names the concrete response in place of an empty-schema answer. ReadSchema against a fresh disposable SpiceDB answers with the gRPC NOT_FOUND status, the client maps it to a typed no-schema result, and the acceptance line asserts exactly that result.

There is one evaluator. The file check.rs is the only site that calls CheckPermission, and it offers a traced variant that returns the verdict with its trace. The file explain.rs consumes that variant and never calls CheckPermission itself. R3's spec and acceptance, R6's spec and acceptance, the boundary that no second evaluator is added, and the verification all say the same.

Every search in an acceptance line runs over Rust source only and excludes crates/lys-identity-server/proto/ and every tests/ directory. The brief writes each search as a command that names its excluded paths, with its exact expected count.

The requirements are ordered so that none depends on a later one. R3 comes after R2 and is blocked on the same C5 blocker, stated in its task and in blocked_by. The consistency rule moves into R3's spec, where it is first used, and R4 refers back to it. No forward reference remains, and every C and S id stays claimed by exactly one requirement.

While the projector is behind the committed head, only a check that depends on an unapplied grant event is refused with permission_not_current, naming the affected grant. Unrelated authority stays usable, as DIRECTORY-006 R3 requires. An acceptance line pauses the projector behind one unapplied grant and shows that a check on an unrelated grant is permitted while a check on the affected grant is refused by name.

The expected relationship list for R2's fixture log is written by hand in the test fixture from the recorded grant contract and schema, and is never produced by relationships.rs. The store is compared against that list.

R2's acceptance asserts behaviour per permission, independently of the grant representation. The delegations G2 and G3 derive from G1, so after G1 is revoked C and D are refused. The fixture also adds a truly unrelated grant G4, a different root with its own subject and resource that does not derive from G1. The test asserts that G4's permission is allowed before the revoke and still allowed after it, which proves the revoke refuses only what descends from G1. The number of relationships remaining after the revoke is declared in the fixture as a named constant with a comment naming C5, and it is set when Tom decides C5. No number goes into the acceptance before then.

R7 defines its fixture by name. It names person P, a second person Q, agent A and identity B, the grants between them, one permitted question and one refused question, and the verdict expected for each. Reusing R6's fixture of P, A and B is allowed if the line says so and still names every identity, grant, question and verdict.

R5's blocker is keyed on an artifact, never on commit text. Card hhAN8h77 is the typed capability claim, whose brief DIRECTORY-012 creates the claim type in crates/lys-identity/src/capability/claim.rs. R5's blocker is the command git cat-file -e origin/main:crates/lys-identity/src/capability/claim.rs, which succeeds only once the claim type has landed.

S19 goes under the existing Responsible person persona and S21 under the existing Operator persona, so each persona name appears once. The choices_made entry that called them new personas is corrected.

The brief runs python3 scripts/design/render-cluster.py docs/design/directory and includes the rendered USER-STORIES.md, CHECKLIST.md, DESIGN.md and its own brief file. The command sh scripts/design/gate.sh exits 0 at the published commit.

Rulings of the lead, Archie, given on 27 September 2026 to the run d40aa430-3dff-46fe-9774-fbfd87c0c735 in answer to its rounds. That run took every answer and then failed before writing, when the account pool refused every session. They are settled here, and the author reopens none of them.

Neither. Take the next free id past lys main and every open brief and draft branch, checked with git ls-remote immediately before writing, and take the C and S ranges the same way, next free past every branch. DIRECTORY-007 and DIRECTORY-010 are both held on other branches, and the brief that wrote an id first keeps it. The words' 'next id after DIRECTORY-006' is read as that next free id. Answered by Archie, lead for the identity line.

Amend it narrowly, the same way as on the typed capability claim brief (hhAN8h77). Permission enforcement through SpiceDB on the identity server leaves the 'Road step 2 onward' non-goal and becomes a goal, and CN11 gets one appended ruling line saying so. Everything else in the non-goal stays, and neither sentence is reworded. Record the amendment as this brief's own decision at the next free ADR id, citing STATEMENT-2026-09-22.md for where step 2 is defined. Answered by Archie, lead for the identity line.

Only the standalone identity server. The Done line is met for calls made to the identity server. The door asking SpiceDB on every call is a Cambium card, and this brief names it as the act that answers the statement's framing at STATEMENT-2026-09-22.md:17 and :143 without doing it. The Non-Goal on rows 06 and 07 stays, and the brief states plainly that an agent calling through the door is not refused by this work until that card lands. Answered by Archie, lead for the identity line.

Both. The screen answers why an identity can do a thing and why it cannot, as the draft does. A refusal shows its reason by name, permission_revoked or no_grant, and the grant it concerns. R7's fixture asserts one permitted and one refused question, each with its expected verdict and reason. The words' 'can' is read as the question the screen answers either way. Answered by Archie, lead for the identity line.

Yes, CN1 binds only DIRECTORY-001's documents-only work. DIRECTORY-006 on main already names code paths, and that reading is how it stands. CN1 is not rewritten. One line is appended beneath it saying that CN1 bounds DIRECTORY-001, and that each later brief of the cluster states its own file walls. This is the same form as the lines appended beneath CN11 and P6 in the revocation and sessions briefs. Answered by Archie, lead for the identity line.

It stays with the lifecycle brief. ADR-011 is proposed and the meaning of suspension is open, so a check that refuses a suspended agent would decide it here. This brief's checks read grants and nothing else. The brief names, as a further unit not written, the lifecycle card's check that reads the state once ADR-011 is decided, and it quotes ADR-011's consequence sentence as the reason. Answered by Archie, lead for the identity line.

Accepted, by the first way. R1's spec states that the tests' relationship writes are built inside crates/lys-identity-server/tests/spicedb_support/, and that client.rs builds no WriteRelationshipsRequest, DeleteRelationshipsRequest or WriteSchemaRequest. R2's search then holds as written, with projector.rs the only file outside tests/ and proto/ that builds them.

Accepted as proposed. R3's acceptance, R6's acceptance and the verification each search for -e 'CheckPermissionRequest' -e 'check_permission(', with the same excluded paths, and expect exactly one line, check.rs, for the whole-tree search and 0 for explain.rs.

Accepted. The RM-030 note is corrected to what the branches hold now, and the collision with origin/draft/directory/174bf6ee over C72 to C78 and S35 to S37 is recorded beside the ADR-043 and RM-030 collisions. This draft wrote those ids first at 12:39 and keeps them, and the note says so. The author re-reads every branch head immediately before writing and records the heads read.

Rulings of the lead, Archie, given on 27 September 2026 to the run 09a6cc80-5c46-4b7a-9e08-0561887a7d3a in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

Take the next free ids past main and every open brief and draft branch as they stand now, re-checked immediately before writing: DIRECTORY-018, C123, S52, ADR-056 and RM-036, or later if taken by then. C72 to C78, ADR-043 and RM-030 are already used on other branches, so keeping them would collide. This ruling supersedes the words' sentence that the draft keeps its ids, and the note says so. Every reference follows the new numbers. Answered by Archie, lead for the identity line.

Only grant and permission checks. The administrator's admission in P9 and C13 stays with the configured issuer and subject and never asks SpiceDB, because that admission is how an operator bootstraps the directory before any relation exists. The brief states that as the one named exception to 'every check'. Answered by Archie.

Through SpiceDB, under this brief. The reverse question is answered by LookupSubjects at the same revision, the zedtoken, as the forward CheckPermission, so the two always agree, and GRANT_EXPLAIN's acceptance holds. check.rs stays the only CheckPermission site, and one named module is the only LookupSubjects site. An acceptance line asks both questions at one revision and asserts they agree. Answered by Archie.

Yes. deploy/identity/README.md gains one step-2 sentence beside DIRECTORY-002 R4's step-1 sentence, which stays word for word. The new sentence says that from this row on SpiceDB answers every grant and permission check the identity server makes, holds the directory's relations, and that the administrator's admission is the one exception. Answered by Archie.

The non-goal's text is not reworded, but its reason gains one appended sentence, as P6 was appended: 'Permission enforcement through SpiceDB on the standalone identity server is not covered by this non-goal; ADR-062 and CN11 carve it out.' The CN11 line under ADR-062 stays as well, so both places say the same thing. Answered by Archie, lead for the identity line.

C5 is a technical decision and is not Tom's. It is the owning lead's with a second reader, as Waffles ruled at 12:13 for the typed claim format. I rule it here, so R2 and R3 are not blocked. A grant is one SpiceDB relationship on its resource, carrying an expiry caveat that ends no later than its source. A delegated grant also records its source grant through a source relation, so derivation is a walk of that relation. Withdrawing a grant deletes it and every relationship derived from it through the source relation in the same write, so RELATIONSHIPS_AFTER_ROOT_REVOKE is 0. The brief records C5 as decided by the lead, with Apollo named as second reader, and Waffles records the ratification on the card. Answered by Archie.

Yes, but not in this brief. The collisions are a defect of the method, and the ledger should hand out ids when a brief is written. That is its own card on the method's board; this brief names it under further units and does not build it. Until it lands, the rule stands: the brief that writes an id first keeps it. Between 82abef61 and 8c1bee6c, whichever wrote ADR-058 and RM-037 first on its branch keeps them, and the other takes the next free ids, re-checked with git ls-remote before writing. This brief takes the next free ids past all of them. Answered by Archie.

Refuse, never clamp. The bound is enforced at delegation: a delegation asking for an expiry later than its source grant's is refused by name before anything is committed, naming the source grant and both expiries. A committed delegation event that nonetheless asks for a later expiry is a corrupt or foreign event: the projector refuses to project it, by name, and the directory stops there as it does for any leaf the projection refuses, writing no relationship for it. A silent clamp would make the record say one thing and the grant another, which row 2.6 forbids. R2 keeps its in-bound fixture and gains two acceptance lines: a delegation past its source's expiry is refused by name and commits nothing, and a committed event past the bound is refused by the projector by name with no relationship written. Answered by Archie, lead for the identity line.

Each gets one appended line. The non-goal, the goal and checklist C5 each gain a line saying the grant representation is decided under ADR-062, ruled by the lead with Apollo as second reader, on Waffles' ruling of 12:13 on Tom's word of 12:12 that technical formats are the owning lead's with a second reader. The original text stays as written above the appended line, so the history is kept and the contradiction is not. Answered by Archie, lead for the identity line.

More than one write is allowed, and the revoke stays one committed event. The revoke event is committed to the log first. The projector then deletes the root relationship in the first write, so no check through that source succeeds from then on. After that it deletes the derived relationships in writes no larger than the configured max_updates_per_write, read from the deployment and never raised for this. Every write is idempotent, and the projector does not advance past the revoke event until a read shows RELATIONSHIPS_AFTER_ROOT_REVOKE = 0, so a crash mid-revoke resumes on replay. Acceptance lines use a test cap smaller than the derived set: the revoke takes more than one write, a permission check through the revoked root fails after the first write, the count reaches 0, and a replay after a crash injected between writes also reaches 0. Answered by Archie.

Fix both as set out in the two findings below: R3 names no later requirement, and R6's freshness line moves into R4. Answered by Archie, lead for the identity line.

Take both suggested rewordings. R3's spec says nothing about R4 or R5. R4's spec states that its refused check still makes its one CheckPermission call through check.rs, with that acceptance line in R4. R3 says that this row does not depend on card hhAN8h77's typed capability claim, and the blocker on that card stays on the requirement that uses it. Answered by Archie.

Move it into R4. The acceptance line about the projector paused behind an unapplied revoke of PA, and the permission_not_current clause of R6's spec, go to R4, which waits on C25. R6 then keeps its explanation paths with no freshness dependency and goes ahead, as its task and the first blocked_by entry already say. R6's spec may say that a check refused as not current has no path to explain, without naming R4. Answered by Archie.

Yes. Remove the stray full stop so it reads 'never clamped; a root revoke deletes', then re-render CHECKLIST.md and DIRECTORY-020.md. Answered by Archie.

## What the survey found, and its angles

The words ask for the directory cluster's road-step-2 brief, for the standalone identity server only. Every grant and permission check the server makes is answered by SpiceDB through one evaluator, and the relationships SpiceDB holds are a projection of the signed directory log. The browser only shows what the server decided. Done means two things: an agent whose grant is revoked between the two steps of a task is refused on its next call to the identity server, and the screen answers why an identity can or cannot do a thing. Three earlier runs of this card took the lead's rulings and then failed; one of them published a complete draft (DIRECTORY-020 at dcd8e881 on draft/directory/09a6cc80). The author re-issues that draft under fresh ids, with the narrow amendments to the design and their rendered files, so that sh scripts/design/gate.sh exits 0.

### What the tree holds

- `docs/design/directory/DESIGN.md (design.json)` — 196 lines. Every amendment the rulings name lives here. The goal 'Every decision still open for Tom is recorded as open...' and the grant-representation non-goal ('OPEN for Tom') each get one appended line. The 'Road step 2 onward' non-goal's reason gets one appended sentence. CN1 and CN11 each get one appended ruling line. New structure rows are added for every path the brief names.
- `docs/design/directory/checklist.json / CHECKLIST.md` — Main holds C1–C30. C5 lists the grant representation as open for Tom and gets an appended decided line. C10 is SpiceDB's step-1 role and must stay. C25 (revocation freshness, 'reviewed before dispatch') is what R4 waits on. C27 covers the single evaluator and forward/reverse explanations. The new C items start at C186.
- `docs/design/directory/stories.json / USER-STORIES.md` — Main holds S1–S12 under five personas: Responsible person, Reviewer, Operator, Verifier, and Grant holder and reviewer. The new stories go under existing personas and no persona is duplicated. The new S range starts at S77.
- `docs/design/directory/briefs/DIRECTORY-002.json R4` — This is SpiceDB's step-1 sentence in deploy/identity/README.md, and it says nothing is added to crates/lys/src/identity/. It must stay word for word. The step-2 sentence goes beside it.
- `docs/design/directory/briefs/DIRECTORY-006.json` — Owns the seams this brief answers behind. R3 has the committed grant event and 'unrelated authority stays usable'. R4 is permission.rs, with the freshness 'Proposal under review ... must be settled before dispatch' (C25). R5 is the explain seam in grants.rs and the reverse question. R1's GRANT_CONTRACT 'cannot be dispatched until independent contract review ratifies the schema'. Its blocked_by still lists the grant representation, root-authority bootstrap and SpiceDB consistency contract as open.
- `docs/design/decisions.json` — Main holds ADR-001 to ADR-018. The new amendment ADR goes at the next free id past every branch, ADR-076 as measured. ADR-011's consequence sentence, 'DIRECTORY-003 records the state beside each identity; no check reads it until the permission work of road step 2.', is the reason the lifecycle check stays out.
- `docs/design/roadmap.json` — Main tops out at RM-016. The brief's roadmap row goes at the next free id, RM-047 as measured.
- `docs/design/identity/STATEMENT-2026-09-22.md:17 and :143` — Line 17 says 'Every check the door makes asks SpiceDB. The browser shows what is permitted; it is never the authority.' Line 143 defines step 2, including the Done sentence. The words narrow 'the door' to the identity server, and the lead's ruling confirms that. The brief names the Cambium door card as the act that answers these lines.
- `scripts/design/gate.sh (validate.py 409 lines, check-coverage.py 401, render-cluster.py 256)` — The method gate. It validates decisions.json and project.json, checks coverage for every cluster, and requires the committed markdown to be byte-equal to a fresh render. Per-brief coverage is bidirectional, so every new C and S must be claimed by exactly one requirement.
- `Cargo.toml (root)` — [workspace.dependencies] is at line 20 and holds no tonic, prost or protox. R1 modifies it, as the ruling says.
- `draft/directory/09a6cc80-5c46-4b7a-9e08-0561887a7d3a @ dcd8e881 (DIRECTORY-020)` — The latest full draft of this card, with the rulings applied: R1–R7, C144–C150, S59–S61, ADR-062, RM-039, 11 files changed. Its C145 text carries the stray 'never clamped.;'. It is the base to renumber from.
- `crates/lys-identity/, crates/lys-identity-server/, surface/identity/, deploy/identity/` — None of these exists on origin/main (fa3dd53). Every code path in the brief is therefore planned, and dispatch is blocked on the DIRECTORY-002/003/005/006 foundations (CN9, CN12).

### What was already decided

- DIRECTORY-002 R4 — In step 1 SpiceDB enforces nothing: no check, no relationship or schema write in crates/lys/src/identity/, and the step-1 sentence sits in deploy/identity/README.md.
- DIRECTORY-006 — The grant contract, delegation, one committed event, revocation and freshness (R4, a proposal under review), and the explain seam (R5). This brief supplies what answers behind R4 and R5 and replaces neither.
- CN1 — 'Documents only' as written. The ruling appends one line saying it bounds DIRECTORY-001 only.
- CN11 — SpiceDB enforces nothing in step 1, and enforcement is step 2's. It gets one appended ruling line under the new ADR.
- CN9 / CN12 — A row stops on any file outside its wall, and a brief blocked on dependencies is not dispatched.
- P4 / P5 — One signed committed event is both the change and its audit, and nothing succeeds before durable evidence. Relationships are a projection, never an independent write.
- P9 / C13 — The configured administrator issuer and subject is the one exception that never asks SpiceDB.
- Non-goal 'Road step 2 onward' — Its text stays. Its reason gets one appended sentence carving out enforcement through SpiceDB on the standalone server.
- Non-goal 'The grant representation ... OPEN for Tom' — It stays as written, with one appended 'Decided under ADR-…' line.
- C5 — Lists the grant representation as open for Tom. It gets one appended line saying it was ruled by the lead, with Apollo as second reader.
- C25 — Revocation freshness is reviewed before dispatch. R4 waits on it.
- ADR-003 — Every grant is pegged to a human, and withdrawing a grant stops everything derived from it. The exact delegation schema is not settled by it, and the new ADR settles the SpiceDB representation.
- ADR-009 — The Rauthy fork and its vendor/rauthy pin at dd61ac3c are untouched.
- ADR-010 — The why view uses Aion's appearance with the identity orange.
- ADR-011 (proposed) — Suspension semantics are open, so no check here reads lifecycle state.
- ADR-004 — Tests run standalone against a disposable SpiceDB, with other servers absent.
- RM-001 — The directory roadmap row the new RM row depends on.

### What was measured

- Remote heads under refs/heads on origin (ls-remote): 235 heads (278 refs in total); 163 of them absent from the local object store
- Local HEAD compared with origin/main: HEAD 7b53625 is 2 commits behind origin/main fa3dd53. The 16 files those 2 commits change are all under crates/lys and crates/lys-core, none under docs/design.
- Highest directory brief id on any branch: DIRECTORY-024 (draft/directory/2da3cf8c), so the next free id is DIRECTORY-025
- Highest checklist id in the directory cluster on any branch: C185 (2da3cf8c), so the next free is C186
- Highest story id in the directory cluster on any branch: S76 (2da3cf8c), so the next free is S77
- Highest ADR on any branch: ADR-075 (draft/directory/8c1bee6c), so the next free is ADR-076
- Highest RM on any branch: RM-046 (2da3cf8c), so the next free is RM-047
- Ids held on main: Briefs DIRECTORY-001 to 006 and 008 (7 briefs, 14 files); C1–C30; S1–S12; ADR-001–ADR-018; RM up to RM-016
- Earlier drafts of this card on origin: 3: DIRECTORY-010 (9bb544d1), DIRECTORY-014 (d40aa430), DIRECTORY-020 (09a6cc80 @ dcd8e881)
- The DIRECTORY-020 draft: 7 requirements, 7 checklist items (C144–C150), 3 stories (S59–S61), a 19-hour estimate, 11 files changed
- Branches holding ADR-062: 2 (09a6cc80 and 0f34978d): a live collision
- Branches holding DIRECTORY-021: At least 2 different briefs (7494cb8b roles; 8c1bee6c sessions): the method's id collisions continue
- Branches holding a DIRECTORY-012 that creates capability/claim.rs (card hhAN8h77): 3; one other branch uses DIRECTORY-012 for a different brief (certificate revocation)
- On origin/main: crates/lys-identity, crates/lys-identity-server, surface/identity, deploy/identity/README.md, crates/lys-identity/src/capability/claim.rs: 0 of 5 exist
- tonic / prost / protox in the root Cargo.toml: 0 occurrences
- DESIGN.md / CHECKLIST.md / USER-STORIES.md size on main: 196 / 49 / 35 lines

### What it means for the other projects

- cambium — Nothing in Cambium changes. The brief names the later Cambium door card, in which the door asks SpiceDB on every call, as the act that answers STATEMENT lines 17 and 143, and it states that an agent calling through the door is not refused until that card lands. The card's brief_card, sign-off and card_build_v3 run on the Cambium board.
- method — The id collisions (ADR-062 held on two branches, DIRECTORY-021 and DIRECTORY-012 each used by two different briefs) are the method's defect. They are named as a further unit, a ledger that hands out ids when a brief is written, on the method's board, and are not built here. validate.py, check-coverage.py and render-cluster.py are used unchanged.
- aion — The card travels the workflow chain (brief_card, sign-off, card_build_v3, src_pr, src_land). Three runs of this card failed from infrastructure (Argus not answering, the account pool refusing sessions, the usage limit) after taking every answer, so the chain's resilience matters to this card reaching its brief.
- argus — There is no change to Argus. The first run failed when Argus stopped answering its session, and the Argus MCP server is failing to connect in this session too, which is a risk to the next run.

### The decisions it stands on

- ADR-003 (honour) — Revocation stops only what derives from the withdrawn grant (G4 is unaffected), and the agent's subset is bounded by its source. The new ADR fills in the delegation schema that ADR-003 leaves open, and does not contradict it.
- ADR-004 (honour) — Tests run against a disposable SpiceDB, with other servers absent.
- ADR-005 (honour) — SpiceDB's store stays on the one PostgreSQL service DIRECTORY-002 installs.
- ADR-009 (honour) — The words name it. The Rauthy fork and its vendor/rauthy pin are untouched.
- ADR-010 (honour) — The why view follows Aion's appearance with the identity orange.
- ADR-011 (honour) — It stays proposed. No check reads lifecycle state, and its consequence sentence is quoted as the reason the lifecycle check is a further unit.
-  (new) — The amendment ADR at the next free id (ADR-076 as measured; the draft's ADR-062 collides with 0f34978d). It brings SpiceDB enforcement on the standalone identity server into the design's goals, appends to CN11 and the non-goal's reason, and records C5's grant representation as ruled by the lead with Apollo as second reader, including refusing and never clamping an expiry past its source, and a multi-write revoke under one committed event.

### What it requires

- docs/design/directory/briefs/DIRECTORY-<next free>.json and its rendered .md exist, at the next free id past main and every brief/draft branch, re-checked with git ls-remote immediately before writing (DIRECTORY-025 as measured).
- The C, S, ADR and RM ids are the next free past every branch (C186…, S77…, ADR-076, RM-047 as measured), and every new C and S is claimed by exactly one requirement.
- R1–R7 are ordered so that none refers to a later one. R3 names neither R4 nor R5. R4 carries the permission_not_current acceptance line and waits on C25. R5's blocker is git cat-file -e origin/main:crates/lys-identity/src/capability/claim.rs.
- Every file search in an acceptance line is a git grep over '*.rs' that names ':(exclude)crates/lys-identity-server/proto/' and ':(exclude,glob)**/tests/**' and states its exact expected output.
- The CheckPermissionRequest/check_permission( search prints exactly check.rs over the whole tree and 0 for explain.rs, in R3, R6 and the verification. The LookupSubjects search prints exactly lookup.rs.
- R2's fixture has G1→G2→G3 derived and G4 unrelated. After the revoke, C and D are refused and G4 is still allowed. RELATIONSHIPS_AFTER_ROOT_REVOKE = 0 is a named constant with a comment naming C5 and the new ADR.
- R2 has acceptance lines for a revoke under a test max_updates_per_write smaller than the derived set, for a check refused after the first write, and for a crash between writes followed by replay reaching 0.
- R2 has acceptance lines for a delegation past its source's expiry refused by name with 0 commits, and for a committed event past the bound refused by the projector by name with no relationship written.
- R1's acceptance asserts that ReadSchema against a fresh disposable SpiceDB returns gRPC NOT_FOUND and that the client maps it to exactly the typed no-schema result.
- R7's fixture names P, Q, A and B, their grants, one permitted and one refused question, and each question's verdict and reason.
- The administrator's admission is stated as the one exception to 'every check', with an acceptance line that records 0 CheckPermission calls for it.
- The design documents gain only appended lines: one sentence appended to the non-goal's reason; one line each under CN11 and CN1; one line each appended to the grant-representation non-goal, the 'decided nowhere' goal and C5. The original text stays above each.
- The brief names further units: the Cambium door card, the lifecycle check once ADR-011 is decided, and the method's id-ledger card.
- The RM row's notes record the branch heads read and the collisions (ADR-062 on 0f34978d, the earlier draft ids).
- python3 scripts/design/render-cluster.py docs/design/directory has been run, and USER-STORIES.md, CHECKLIST.md, DESIGN.md and the brief's .md are committed as rendered.
- sh scripts/design/gate.sh exits 0 at the published commit.

### What must not change

- DIRECTORY-002 R4 and its step-1 README sentence are unchanged, and crates/lys/src/identity/ gains no SpiceDB check or write.
- The Rauthy fork, its files and the vendor/rauthy pin are not touched (ADR-009).
- No DIRECTORY-006 requirement is rewritten; this brief answers behind its seams.
- No second evaluator is added.
- The original text of the non-goals, CN1, CN11, C5 and the goal is not reworded; lines are only appended.
- The IDENTITY-001 files and the design's other non-goals stay as they are.
- There are no changes to lys-core or to any published wire format.
- Nothing in Cambium or the door changes.
- Suspension semantics are not decided and ADR-011 stays proposed.
- The brief touches documents only: nothing outside docs/design/directory/, docs/design/decisions.json and docs/design/roadmap.json.
- No requirement is dispatched before Tom or the lead signs the brief off on the card.

### What we must put in place first

- Update the branch to origin/main fa3dd53 (the local HEAD 7b53625 is 2 commits behind; neither commit touches docs/design).
- Re-read every brief/* and draft/* head with git ls-remote plus remote reads immediately before writing. 163 of 235 heads are not in the local object store, so a local-only check misses collisions.
- Take the 09a6cc80 draft (dcd8e881) as the base and renumber every reference to the new ids.

### The risks

- The ids collide again before publishing. Other drafts are being written the same day (ADR-062 is already on two branches, and DIRECTORY-021 and DIRECTORY-012 each name two different briefs).
- A fourth run fails from infrastructure after taking its answers. The earlier failures were Argus, the account pool and the usage limit, and Argus MCP is failing to connect now.
- The Done line cannot be met until C25, card hhAN8h77's claim type and the DIRECTORY-002/003/005/006 foundations all land. None of the five foundation paths exists on main.
- The pure-Rust promise can break through transport security. A tonic TLS feature can pull aws-lc-sys or ring (C and assembly), and the '-sys' count grep does not catch ring.
- The 'exactly 2 WriteRelationships calls at default max_updates_per_write' acceptance depends on the pinned SpiceDB's default and on its in-memory testing mode behaving as described.
- Leaving root-authority bootstrap unresolved leaves the fixtures' 'person P grants' step undefined.
- The rulings name file DIRECTORY-020.md and ids that are now stale. A literal reading would write under a collided id.
- check.rs, which carries the plain and traced variants, the caveat clock and the consistency rule, may pass the 500-line file limit.

### Still open

- Does the lead's C5 ruling also count as the independent ratification DIRECTORY-006 R1 (GRANT_CONTRACT) and DIRECTORY-006's blocked_by require, or do R1 and this brief's R2/R3 still wait on that separate contract review? The sentence of the words it stands on: "I rule it here, so R2 and R3 are not blocked.". Why only the lead can settle it: docs/design/directory/briefs/DIRECTORY-006.json R1 says 'This row and GRANT_CONTRACT cannot be dispatched until independent contract review ratifies the schema'. Its blocked_by still lists 'Reviewed grant representation, root-authority bootstrap and SpiceDB consistency contract remain open in DIRECTORY-001C5'. This brief depends on DIRECTORY-006, so unless the ruling reaches DIRECTORY-006 too, R2 and R3 are still blocked in practice.
- Where does a person's root authority on a resource come from? A root grant has no source to bound its expiry, so what bounds it, and which act writes the first relationship a person grants from? The sentence of the words it stands on: "A grant is one SpiceDB relationship on its resource, carrying an expiry caveat that ends no later than its source.". Why only the lead can settle it: DIRECTORY-006's blocked_by names 'root-authority bootstrap' as open. The ruling covers the administrator's admission, not how a person first holds a grant. Every fixture ('person P grants agent A read on project X') assumes P already holds authority on X, and the schema's expiry bound is undefined for a grant with no source.
- Who settles DIRECTORY-006 R4's freshness proposal (C25), which R4 and then R5 (the Done line) wait on: the lead with a second reader, as C5 was, or Tom? The sentence of the words it stands on: "The acceptance line about the projector paused behind an unapplied revoke of PA, and the permission_not_current clause of R6's spec, go to R4, which waits on C25.". Why only the lead can settle it: The Done line (an agent refused mid-task) is R5, and R5 follows R4. DIRECTORY-006.json R4 says the freshness proposal 'must be settled before dispatch', and nobody is named to settle it. Until someone is, the card cannot reach its Done line.

### The units beyond the first

- Cambium: the door asks SpiceDB on every call — STATEMENT lines 17 and 143 describe the door. It is Cambium's repository and board, and until it lands an agent calling through the door is not refused.
- Lifecycle: a check reads the identity's state once ADR-011 is decided — The meaning of suspension is open, and ADR-011's consequence reserves that check for the permission work. Deciding it here would settle ADR-011.
- Method: a ledger that hands out brief, C, S, ADR and RM ids when a brief is written — The collisions are a defect of the method, and the fix belongs on the method's board, not in a directory brief.
- Settle DIRECTORY-006 R4's freshness mechanism (C25) — R4, and through it R5 and the Done line, wait on it. It is a review act on DIRECTORY-006, not part of this brief.
- Build the brief through card_build_v3 after sign-off — The brief is documents only. The code rows are built by the chain on Dean's laptop once the foundations and the claim type land.

### The smallest complete shape

One documents-only commit on the card's draft branch, containing: the new brief (DIRECTORY-025 JSON and rendered MD) with R1–R7 as ruled; seven checklist items and three stories at the next free C and S ids under existing personas; the amendment ADR at the next free ADR id; the RM row at the next free RM id, carrying the collision notes and the branch heads read; the appended lines in design.json (non-goal reason, CN1, CN11, the grant-representation non-goal, the goal, the structure rows) and in checklist C5; and every rendered file re-rendered. At that commit sh scripts/design/gate.sh exits 0.

## The roadmap row

- **RM-048** — Road step 2: every permission check the identity server makes asks SpiceDB, and the screen answers why (feature, idea)
- Summary: Road step 2 of the directory design for the standalone identity server: SpiceDB answers every grant and permission check through one evaluator behind DIRECTORY-006's permission decision and explain seam, its relationships a projection of the signed directory log, and the administrator's admission by configured issuer and subject is the one named exception; a call after a committed revoke is never admitted, and while the projection lags only a check on the affected grant is refused; the screen answers why an identity can and why it cannot do a thing from the server's answer, and who can act on a resource is answered at the same revision. Done when an agent that loses a permission between the two steps of a task is refused on its next call to the identity server. Cambium's door is a later Cambium card.
- Asked by: tom on 2026-09-27T14:25:00+10:00
- Context: The road step 2 card for the directory cluster on the Lys board, written through the brief method with the lead's rulings of 27 September to the failed runs 9bb544d1, d40aa430 and 09a6cc80 and the lead's answers to this run's survey; the 09a6cc80 draft on origin draft/directory/09a6cc80-5c46-4b7a-9e08-0561887a7d3a (DIRECTORY-020 at dcd8e881) is the base these files amend under new ids.
- Quote: Write the brief for this card in the directory cluster (docs/design/directory/briefs, next id after DIRECTORY-006), as road step 2 of the directory design (docs/design/directory/DESIGN.md): every check the server makes asks SpiceDB; the browser shows what is permitted and is never the authority. Done when an agent that loses a permission in the middle of a two-step task is refused on its next call, and the screen answers why an identity can do a thing. Hold to R4 of DIRECTORY-002 (what SpiceDB enforces in step 1) and to ADR-009. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after Tom or the lead signs it off on the card.

Rulings of the lead, Archie, given on 27 September 2026 to the run 9bb544d1-edf0-46fc-b4d4-a274cacddbdd in answer to its rounds 2, 3 and 4. That run took every answer and then failed before writing, when Argus stopped answering its session. They are settled here, and the author reopens none of them.

R1's files list modify Cargo.toml, the root manifest whose workspace dependencies gain tonic, prost and protox. Its structure row already exists. The lys-identity-server Cargo.toml and lib.rs stay as modify targets under the foundation blocker.

R1's acceptance names the concrete response in place of an empty-schema answer. ReadSchema against a fresh disposable SpiceDB answers with the gRPC NOT_FOUND status, the client maps it to a typed no-schema result, and the acceptance line asserts exactly that result.

There is one evaluator. The file check.rs is the only site that calls CheckPermission, and it offers a traced variant that returns the verdict with its trace. The file explain.rs consumes that variant and never calls CheckPermission itself. R3's spec and acceptance, R6's spec and acceptance, the boundary that no second evaluator is added, and the verification all say the same.

Every search in an acceptance line runs over Rust source only and excludes crates/lys-identity-server/proto/ and every tests/ directory. The brief writes each search as a command that names its excluded paths, with its exact expected count.

The requirements are ordered so that none depends on a later one. R3 comes after R2 and is blocked on the same C5 blocker, stated in its task and in blocked_by. The consistency rule moves into R3's spec, where it is first used, and R4 refers back to it. No forward reference remains, and every C and S id stays claimed by exactly one requirement.

While the projector is behind the committed head, only a check that depends on an unapplied grant event is refused with permission_not_current, naming the affected grant. Unrelated authority stays usable, as DIRECTORY-006 R3 requires. An acceptance line pauses the projector behind one unapplied grant and shows that a check on an unrelated grant is permitted while a check on the affected grant is refused by name.

The expected relationship list for R2's fixture log is written by hand in the test fixture from the recorded grant contract and schema, and is never produced by relationships.rs. The store is compared against that list.

R2's acceptance asserts behaviour per permission, independently of the grant representation. The delegations G2 and G3 derive from G1, so after G1 is revoked C and D are refused. The fixture also adds a truly unrelated grant G4, a different root with its own subject and resource that does not derive from G1. The test asserts that G4's permission is allowed before the revoke and still allowed after it, which proves the revoke refuses only what descends from G1. The number of relationships remaining after the revoke is declared in the fixture as a named constant with a comment naming C5, and it is set when Tom decides C5. No number goes into the acceptance before then.

R7 defines its fixture by name. It names person P, a second person Q, agent A and identity B, the grants between them, one permitted question and one refused question, and the verdict expected for each. Reusing R6's fixture of P, A and B is allowed if the line says so and still names every identity, grant, question and verdict.

R5's blocker is keyed on an artifact, never on commit text. Card hhAN8h77 is the typed capability claim, whose brief DIRECTORY-012 creates the claim type in crates/lys-identity/src/capability/claim.rs. R5's blocker is the command git cat-file -e origin/main:crates/lys-identity/src/capability/claim.rs, which succeeds only once the claim type has landed.

S19 goes under the existing Responsible person persona and S21 under the existing Operator persona, so each persona name appears once. The choices_made entry that called them new personas is corrected.

The brief runs python3 scripts/design/render-cluster.py docs/design/directory and includes the rendered USER-STORIES.md, CHECKLIST.md, DESIGN.md and its own brief file. The command sh scripts/design/gate.sh exits 0 at the published commit.

Rulings of the lead, Archie, given on 27 September 2026 to the run d40aa430-3dff-46fe-9774-fbfd87c0c735 in answer to its rounds. That run took every answer and then failed before writing, when the account pool refused every session. They are settled here, and the author reopens none of them.

Neither. Take the next free id past lys main and every open brief and draft branch, checked with git ls-remote immediately before writing, and take the C and S ranges the same way, next free past every branch. DIRECTORY-007 and DIRECTORY-010 are both held on other branches, and the brief that wrote an id first keeps it. The words' 'next id after DIRECTORY-006' is read as that next free id. Answered by Archie, lead for the identity line.

Amend it narrowly, the same way as on the typed capability claim brief (hhAN8h77). Permission enforcement through SpiceDB on the identity server leaves the 'Road step 2 onward' non-goal and becomes a goal, and CN11 gets one appended ruling line saying so. Everything else in the non-goal stays, and neither sentence is reworded. Record the amendment as this brief's own decision at the next free ADR id, citing STATEMENT-2026-09-22.md for where step 2 is defined. Answered by Archie, lead for the identity line.

Only the standalone identity server. The Done line is met for calls made to the identity server. The door asking SpiceDB on every call is a Cambium card, and this brief names it as the act that answers the statement's framing at STATEMENT-2026-09-22.md:17 and :143 without doing it. The Non-Goal on rows 06 and 07 stays, and the brief states plainly that an agent calling through the door is not refused by this work until that card lands. Answered by Archie, lead for the identity line.

Both. The screen answers why an identity can do a thing and why it cannot, as the draft does. A refusal shows its reason by name, permission_revoked or no_grant, and the grant it concerns. R7's fixture asserts one permitted and one refused question, each with its expected verdict and reason. The words' 'can' is read as the question the screen answers either way. Answered by Archie, lead for the identity line.

Yes, CN1 binds only DIRECTORY-001's documents-only work. DIRECTORY-006 on main already names code paths, and that reading is how it stands. CN1 is not rewritten. One line is appended beneath it saying that CN1 bounds DIRECTORY-001, and that each later brief of the cluster states its own file walls. This is the same form as the lines appended beneath CN11 and P6 in the revocation and sessions briefs. Answered by Archie, lead for the identity line.

It stays with the lifecycle brief. ADR-011 is proposed and the meaning of suspension is open, so a check that refuses a suspended agent would decide it here. This brief's checks read grants and nothing else. The brief names, as a further unit not written, the lifecycle card's check that reads the state once ADR-011 is decided, and it quotes ADR-011's consequence sentence as the reason. Answered by Archie, lead for the identity line.

Accepted, by the first way. R1's spec states that the tests' relationship writes are built inside crates/lys-identity-server/tests/spicedb_support/, and that client.rs builds no WriteRelationshipsRequest, DeleteRelationshipsRequest or WriteSchemaRequest. R2's search then holds as written, with projector.rs the only file outside tests/ and proto/ that builds them.

Accepted as proposed. R3's acceptance, R6's acceptance and the verification each search for -e 'CheckPermissionRequest' -e 'check_permission(', with the same excluded paths, and expect exactly one line, check.rs, for the whole-tree search and 0 for explain.rs.

Accepted. The RM-030 note is corrected to what the branches hold now, and the collision with origin/draft/directory/174bf6ee over C72 to C78 and S35 to S37 is recorded beside the ADR-043 and RM-030 collisions. This draft wrote those ids first at 12:39 and keeps them, and the note says so. The author re-reads every branch head immediately before writing and records the heads read.

Rulings of the lead, Archie, given on 27 September 2026 to the run 09a6cc80-5c46-4b7a-9e08-0561887a7d3a in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

Take the next free ids past main and every open brief and draft branch as they stand now, re-checked immediately before writing: DIRECTORY-018, C123, S52, ADR-056 and RM-036, or later if taken by then. C72 to C78, ADR-043 and RM-030 are already used on other branches, so keeping them would collide. This ruling supersedes the words' sentence that the draft keeps its ids, and the note says so. Every reference follows the new numbers. Answered by Archie, lead for the identity line.

Only grant and permission checks. The administrator's admission in P9 and C13 stays with the configured issuer and subject and never asks SpiceDB, because that admission is how an operator bootstraps the directory before any relation exists. The brief states that as the one named exception to 'every check'. Answered by Archie.

Through SpiceDB, under this brief. The reverse question is answered by LookupSubjects at the same revision, the zedtoken, as the forward CheckPermission, so the two always agree, and GRANT_EXPLAIN's acceptance holds. check.rs stays the only CheckPermission site, and one named module is the only LookupSubjects site. An acceptance line asks both questions at one revision and asserts they agree. Answered by Archie.

Yes. deploy/identity/README.md gains one step-2 sentence beside DIRECTORY-002 R4's step-1 sentence, which stays word for word. The new sentence says that from this row on SpiceDB answers every grant and permission check the identity server makes, holds the directory's relations, and that the administrator's admission is the one exception. Answered by Archie.

The non-goal's text is not reworded, but its reason gains one appended sentence, as P6 was appended: 'Permission enforcement through SpiceDB on the standalone identity server is not covered by this non-goal; ADR-062 and CN11 carve it out.' The CN11 line under ADR-062 stays as well, so both places say the same thing. Answered by Archie, lead for the identity line.

C5 is a technical decision and is not Tom's. It is the owning lead's with a second reader, as Waffles ruled at 12:13 for the typed claim format. I rule it here, so R2 and R3 are not blocked. A grant is one SpiceDB relationship on its resource, carrying an expiry caveat that ends no later than its source. A delegated grant also records its source grant through a source relation, so derivation is a walk of that relation. Withdrawing a grant deletes it and every relationship derived from it through the source relation in the same write, so RELATIONSHIPS_AFTER_ROOT_REVOKE is 0. The brief records C5 as decided by the lead, with Apollo named as second reader, and Waffles records the ratification on the card. Answered by Archie.

Yes, but not in this brief. The collisions are a defect of the method, and the ledger should hand out ids when a brief is written. That is its own card on the method's board; this brief names it under further units and does not build it. Until it lands, the rule stands: the brief that writes an id first keeps it. Between 82abef61 and 8c1bee6c, whichever wrote ADR-058 and RM-037 first on its branch keeps them, and the other takes the next free ids, re-checked with git ls-remote before writing. This brief takes the next free ids past all of them. Answered by Archie.

Refuse, never clamp. The bound is enforced at delegation: a delegation asking for an expiry later than its source grant's is refused by name before anything is committed, naming the source grant and both expiries. A committed delegation event that nonetheless asks for a later expiry is a corrupt or foreign event: the projector refuses to project it, by name, and the directory stops there as it does for any leaf the projection refuses, writing no relationship for it. A silent clamp would make the record say one thing and the grant another, which row 2.6 forbids. R2 keeps its in-bound fixture and gains two acceptance lines: a delegation past its source's expiry is refused by name and commits nothing, and a committed event past the bound is refused by the projector by name with no relationship written. Answered by Archie, lead for the identity line.

Each gets one appended line. The non-goal, the goal and checklist C5 each gain a line saying the grant representation is decided under ADR-062, ruled by the lead with Apollo as second reader, on Waffles' ruling of 12:13 on Tom's word of 12:12 that technical formats are the owning lead's with a second reader. The original text stays as written above the appended line, so the history is kept and the contradiction is not. Answered by Archie, lead for the identity line.

More than one write is allowed, and the revoke stays one committed event. The revoke event is committed to the log first. The projector then deletes the root relationship in the first write, so no check through that source succeeds from then on. After that it deletes the derived relationships in writes no larger than the configured max_updates_per_write, read from the deployment and never raised for this. Every write is idempotent, and the projector does not advance past the revoke event until a read shows RELATIONSHIPS_AFTER_ROOT_REVOKE = 0, so a crash mid-revoke resumes on replay. Acceptance lines use a test cap smaller than the derived set: the revoke takes more than one write, a permission check through the revoked root fails after the first write, the count reaches 0, and a replay after a crash injected between writes also reaches 0. Answered by Archie.

Fix both as set out in the two findings below: R3 names no later requirement, and R6's freshness line moves into R4. Answered by Archie, lead for the identity line.

Take both suggested rewordings. R3's spec says nothing about R4 or R5. R4's spec states that its refused check still makes its one CheckPermission call through check.rs, with that acceptance line in R4. R3 says that this row does not depend on card hhAN8h77's typed capability claim, and the blocker on that card stays on the requirement that uses it. Answered by Archie.

Move it into R4. The acceptance line about the projector paused behind an unapplied revoke of PA, and the permission_not_current clause of R6's spec, go to R4, which waits on C25. R6 then keeps its explanation paths with no freshness dependency and goes ahead, as its task and the first blocked_by entry already say. R6's spec may say that a check refused as not current has no path to explain, without naming R4. Answered by Archie.

Yes. Remove the stray full stop so it reads 'never clamped; a root revoke deletes', then re-render CHECKLIST.md and DIRECTORY-020.md. Answered by Archie.
- Cluster: directory; briefs: DIRECTORY-025
- Notes: Ids under the lead's ruling that this brief takes the next free ids past main and every open brief and draft branch as they stand now, re-checked immediately before writing; that ruling supersedes the earlier words' sentence that a draft keeps its ids, and its measured DIRECTORY-018, C123, S52, ADR-056 and RM-036 were taken before this writing. Heads read immediately before writing: git ls-remote origin (281 lines, 237 branch heads) and a full fetch of every head, all 237 origin branches scanned, main at fa3dd53 (this writing sits on 7b53625, two commits behind, and neither of those commits touches docs/design). The highest ids held on any branch were DIRECTORY-024, C185 and S76 on draft/directory/2da3cf8c-7a51-4451-a261-af2420fc0740 at 825ae1c, ADR-077 on draft/directory/fb954264-07e7-4ba1-929d-05808608ad78 at 7511a7c, and RM-047 on draft/secrets/7a0935db-d08e-4142-ad92-1c1212189f2a at 72201f1, so this row takes DIRECTORY-025, C186 to C192, S77 to S79, ADR-078 and RM-048, none of which is held on any branch. Collisions recorded: the 09a6cc80 draft's DIRECTORY-020, C144 to C150, S59 to S61 and RM-039 are held on no branch but draft/directory/09a6cc80-5c46-4b7a-9e08-0561887a7d3a at dcd8e88, and its ADR-062 is also held on draft/directory/0f34978d-1555-4963-a7b9-96f12d4f0430 at f30ecc2, a live collision; the earlier drafts of this card, DIRECTORY-010 (9bb544d1) and DIRECTORY-014 (d40aa430, with C72 to C78, S35 to S37, ADR-043 and RM-030), are not kept either. Under the ruling these ids are left, and every reference follows the new numbers. The earlier stories S19 (responsible person), S20 and S21 (operator) are S77 under the existing Responsible person persona, S78 under the existing Grant holder and reviewer persona and S79 under the existing Operator persona. The brief also depends on DIRECTORY-006, which main's ledger carries in no row. The grant representation (C5) is decided by the owning lead, Archie, with Apollo as second reader, and recorded in ADR-078; Waffles records the ratification on the card. That ruling, with DIRECTORY-006 R1 to R5 as built and accepted at hand/DIRECTORY-006-R5 e525a91 and read by Buckley as second reader, stands as the ratification DIRECTORY-006 R1 asks for; R2 and R3 wait for the grant contract only on DIRECTORY-006 landing on main (git cat-file -e origin/main:crates/lys-identity/src/grants/authority.rs), and R1 to R3 also wait on DIRECTORY-002 landing on main, whose deploy/identity/versions.json and deploy/identity/README.md they read and write (git cat-file -e origin/main:deploy/identity/versions.json); each blocker is checked by its own command. The refusal for a stale projection is DIRECTORY-006's StaleDecision, used by that name throughout the brief. Heads re-read before this amendment: git ls-remote origin (241 branch heads), every head fetched, and DIRECTORY-025, ADR-078 and RM-048 are held on no branch but this draft's own. The freshness mechanism (C25) is settled by the lead with Buckley as second reader as DIRECTORY-006 R4 builds it, so the Done line in R5 waits on no separate freshness review; R5 still waits on the typed capability claim of card hhAN8h77. Root grants come from DIRECTORY-006's root issue as built. git cat-file -e origin/main:crates/lys-identity/src/capability/claim.rs after the fetch exits 128 because the path does not exist on main: R5's blocker reads not yet landed. Further units, not written here: Cambium: the door asks SpiceDB on every call; Lifecycle: a check reads the identity's state once ADR-011 is decided; Method: a ledger that hands out brief, C, S, ADR and RM ids when a brief is written; Build the brief through card_build_v3 after sign-off. Heads re-read before the round that settled R1's dependency measurement: git ls-remote origin (253 branch heads), every head fetched, main at fa3dd53; DIRECTORY-025, ADR-078 and RM-048 are held only on this draft's branch and on draft/directory/647a3946-54db-4ae7-8cac-3f48ed323db7 (now at b560ed3), as recorded above. Under the lead's answer R1 promises only that nothing it adds brings ring, aws-lc or a -sys crate, measured as equal cargo tree counts at the build's start commit and at R1's head: at e525a91 the line already prints 4 on macOS (ring through lys-core's rcgen and through the rustls behind DIRECTORY-003's openidconnect, and core-foundation-sys). Further unit, not written here: Finding against lys-core and DIRECTORY-003: ring and -sys crates in the identity server's dependency graph. The door is a later Cambium card, and until it lands an agent calling through the door is not refused by this work; the lifecycle check stays with the lifecycle card because ADR-011 is proposed and its consequence reads 'DIRECTORY-003 records the state beside each identity; no check reads it until the permission work of road step 2.' The ledger is a card on the method's board, because concurrent drafts collide on ids; until it lands, the brief that writes an id first keeps it. Heads re-read before the round that settled the grant representation: git ls-remote origin (247 branch heads), every head fetched. DIRECTORY-025, ADR-078 and RM-048 are also held on draft/directory/647a3946-54db-4ae7-8cac-3f48ed323db7 at f7a6724, for a different brief (gate, install and demonstrate the exact release), first written there at 20:11 on 2026-09-27; this draft wrote them first, at 19:57 on 2026-09-27 (7556083), so under the rule above it keeps them and that draft takes the next free ids. The grant representation is stated under the lead's answers: every grant is its own grant object keyed by its grant id and is 3 relationships (resource, holder with the expiry caveat, and standing: the source relationship of a delegated grant or the root relationship of a root grant), so a revoke's first write deletes the standing relationship alone and the capped-write counts follow from that; two grants of one holder, action and resource are two objects, and revoking one leaves the other standing. Heads re-read before the round that named the root relationship's subject: git ls-remote origin (249 branch heads), every head fetched; DIRECTORY-025, ADR-078 and RM-048 are held only on this draft's branch and on draft/directory/647a3946-54db-4ae7-8cac-3f48ed323db7, as recorded above. Under the lead's answer the root relationship's subject is the directory's root authority, one object per directory and the same object the issue_root permission is checked against, so every grant stands on exactly 3 relationships, a walk from any grant ends at the root authority in a bounded number of steps, and a grant that stands on nothing is malformed and refused; R4 adds the affected grant's identifier to DIRECTORY-006's GrantError::StaleDecision, which carries only its required and projected revisions as built.

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
- ADR-078 — Permission enforcement through SpiceDB on the standalone identity server enters the directory design, on a decided grant representation — Amend the design narrowly: permission enforcement through SpiceDB on the standalone identity server leaves the 'Road step 2 onward' non-goal and becomes a goal of the directory design, and CN11 gains one appended ruling line saying so. Everything else in the non-goal stays, and neither the non-goal's text nor CN11's is reworded; the non-goal's reason gains one appended sentence saying it does not cover that enforcement. The grant representation (C5) is a technical decision of the owning lead with a second reader, not one left open for Tom, and is decided here: every grant, root or delegated, is its own grant object in SpiceDB, keyed by its grant identifier, and is exactly 3 relationships: the resource's relationship to the grant object under the grant's action; the grant's holder relationship to the identity that holds it, carrying an expiry caveat that ends no later than its source's; and the grant's standing relationship, which for a delegated grant is its source relationship to its source grant, the source relation the permission walks, and for a root grant, which has no source, is its root relationship, the grant object's relationship to the directory's root authority, one object per directory and the same object the issue_root permission is checked against when a person issues a root grant through POST /grants/roots; a permission holds through a grant only while the grant's standing holds, a root grant's through its root relationship and a delegated grant's through its source grant's standing, so derivation is a walk of the source relation that ends at the directory's root authority in a bounded number of steps, a grant that stands on neither a source relationship nor a root relationship is malformed and refused, and deleting a grant's standing relationship refuses its holder and every grant derived from it at once; two live grants giving the same holder the same action on the same resource are two grant objects, each with its own relationships, and revoking one leaves the other standing; withdrawing a grant deletes its relationships and those of every grant derived from it through the source relation on the same committed revoke event, the withdrawn grant's standing relationship alone in the first write and the rest in further writes within SpiceDB's configured max_updates_per_write. The expiry bound is refused, never clamped: a delegation asking for an expiry later than its source grant's is refused by name before anything is committed, and a committed delegation event that nonetheless asks for one is refused by the projector by name, which writes no relationship for it and stops there; a silent clamp would make the record say one thing and the grant another. Rejected: sitting beside the non-goal as a named exception, which leaves the design's stated scope contradicting the brief; rewording the non-goal or CN11; and bringing Cambium's door into this cluster.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster. Decided under ADR-078: the grant representation was ruled by the owning lead, Archie, with Apollo as second reader, on Waffles' ruling of 12:13 on Tom's word of 12:12 that technical formats are the owning lead's with a second reader.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.
- Every grant and permission check the standalone identity server makes is answered by SpiceDB behind DIRECTORY-006's permission decision, the administrator's admission being the one named exception, a call after a committed revoke is never admitted, and the screen answers why an identity can and why it cannot do a thing from the server's answer (DIRECTORY-025, ADR-078).

## Non-Goals

- Rows 06 (connect Cambium) and 07 (gate, install and demonstrate the release) — They change the Cambium repository and depend on IDENTITY-002, the upstream release rebase; they need a Cambium cluster or an agreed cross-repository arrangement first.
- Row 03's changes inside the Rauthy fork — The fork is its own repository under vendor/rauthy; a row whose files live there needs its own brief in the fork, which does not yet exist and blocks DIRECTORY-004. DIRECTORY-004 names the fork files as work with their owner, not as files of this repository.
- The grant representation: how one grant records who may exercise it, whether it may be passed on (person, agent or nobody), what it derives from, and whether it can be bounded. OPEN for Tom. Decided under ADR-078: the grant representation was ruled by the owning lead, Archie, with Apollo as second reader, on Waffles' ruling of 12:13 on Tom's word of 12:12 that technical formats are the owning lead's with a second reader. — The statement leaves the exact delegation schema unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003); AGENT-PARITY-2026-09-23's questions (docs/design/identity/AGENT-PARITY-2026-09-23.md:17-23) are inputs to it, not answers. Ruling (ADR-078): the grant representation is a technical decision of the owning lead with a second reader, and is decided for the SpiceDB schema: every grant, root or delegated, is its own grant object in SpiceDB, keyed by its grant identifier, and is exactly 3 relationships: the resource's relationship to the grant object under the grant's action; the grant's holder relationship to the identity that holds it, carrying an expiry caveat that ends no later than its source's; and the grant's standing relationship, which for a delegated grant is its source relationship to its source grant, the source relation the permission walks, and for a root grant, which has no source, is its root relationship, the grant object's relationship to the directory's root authority, one object per directory and the same object the issue_root permission is checked against when a person issues a root grant through POST /grants/roots; a permission holds through a grant only while the grant's standing holds, a root grant's through its root relationship and a delegated grant's through its source grant's standing, so derivation is a walk of the source relation that ends at the directory's root authority in a bounded number of steps, a grant that stands on neither a source relationship nor a root relationship is malformed and refused, and deleting a grant's standing relationship refuses its holder and every grant derived from it at once; two live grants giving the same holder the same action on the same resource are two grant objects, each with its own relationships, and revoking one leaves the other standing; withdrawing a grant deletes its relationships and those of every grant derived from it through the source relation on the same committed revoke event, the withdrawn grant's standing relationship alone in the first write and the rest in further writes within SpiceDB's configured max_updates_per_write.
- Suspension semantics: whether suspending a person also ends their sign-in session at Rauthy or only makes our checks refuse, and what else stops with a suspended identity. OPEN for Tom. — The lifecycle document asks it of the room (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:99-100, docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:104-105) and its states are only proposed (ADR-011); two systems, one decision.
- The name of the identity service. OPEN for Tom. — The statement lists it as not decided (docs/design/identity/STATEMENT-2026-09-22.md:192); no crate or directory name selects it.
- Which anchor the first agents pin to: our hosted anchor, a self-hosted one, or both. OPEN for Tom. — Lys leaves it as a product decision and so does the statement (docs/design/identity/STATEMENT-2026-09-22.md:193).
- A nightly Rauthy base versus waiting for an upstream release carrying #1696 and #1728. OPEN for Tom. — Waffles' ruling of 15:36:25: if upstream has published no such release by the row 05 showing, Tom decides (docs/design/identity/briefs/IDENTITY-001.json:18; docs/design/identity/STATEMENT-2026-09-22.md:177); rows 02 to 05 run on v0.36.2 meanwhile.
- Road step 2 onward: capability certificates, arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working. Permission enforcement through SpiceDB on the standalone identity server is not covered by this non-goal; ADR-078 and CN11 carve it out.
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
| `docs/design/directory/briefs/DIRECTORY-025.json` | road step 2: every permission check the identity server makes asks SpiceDB, and the screen answers why | DIRECTORY-025 |
| `docs/design/directory/briefs/DIRECTORY-025.md` | rendered markdown | DIRECTORY-025 |
| `crates/lys-identity-server/Cargo.toml` | the server crate's manifest inside DIRECTORY-003's wall; DIRECTORY-025 adds the SpiceDB client dependencies (tonic, prost, protox) | DIRECTORY-003 |
| `crates/lys-identity-server/src/lib.rs` | the server crate's root inside DIRECTORY-003's wall; DIRECTORY-025 declares the spicedb module | DIRECTORY-003 |
| `crates/lys-identity-server/build.rs` | compiles the vendored authzed v1 protocol files with protox; no protoc | DIRECTORY-025 |
| `crates/lys-identity-server/proto/SOURCE.md` | the authzed API release the protocol files are copied from | DIRECTORY-025 |
| `crates/lys-identity-server/proto/authzed/` | the vendored authzed v1 protocol files | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/mod.rs` | declarations and re-exports only | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/client.rs` | the one SpiceDB gRPC client; NOT_FOUND on ReadSchema is the typed no-schema result; the preshared key redacted | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/error.rs` | typed SpiceDB errors naming the operation and address, never the key | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/schema.rs` | writes the schema at start-up when absent and refuses a stored schema that differs | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/schema.zed` | the SpiceDB schema: the grant representation ADR-078 records, with the expiry caveat and the source relation | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/projector.rs` | the one relationship writer: applies committed signed grant events in log order and records position and revision token; deletes a revoked grant's standing relationship first and its remaining and derived relationships in writes within the configured max_updates_per_write, resuming on replay | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/check.rs` | the one evaluator and the only CheckPermission caller, behind DIRECTORY-006 R4's permission decision; offers the plain check and the traced variant, each at least as fresh as the projector's recorded token; sends the current time from the injected named clock as the expiry caveat's context on every call and refuses CONDITIONAL_PERMISSION as permission_conditional | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/freshness.rs` | refuses with StaleDecision (GrantError::StaleDecision), naming the affected grant, only for a check that depends on a grant event the projection has not applied, discarding the verdict check.rs returned; unrelated authority stays usable | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/explain.rs` | why can and why cannot, from check.rs's traced variant, behind DIRECTORY-006 R5's explain seam; never calls CheckPermission itself | DIRECTORY-025 |
| `crates/lys-identity-server/src/spicedb/lookup.rs` | who can act on a resource, the only LookupSubjects caller, read at the forward answer's revision behind DIRECTORY-006 R5's explain seam | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_support/mod.rs` | declarations only | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_support/server.rs` | starts a disposable in-memory SpiceDB from the release deploy/identity/versions.json pins | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_projection.rs` | schema at start-up and relationship projection, against a hand-written expected list | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_checks.rs` | every route asks CheckPermission once; engine outage refuses | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_freshness.rs` | no call admitted after a committed revoke; only the affected grant is refused while the projection lags | DIRECTORY-025 |
| `crates/lys-identity-server/tests/agent_mid_task.rs` | an agent's two-step task refused on its next call | DIRECTORY-025 |
| `crates/lys-identity-server/tests/spicedb_explain.rs` | why can and why cannot agree with the check; who can agrees with the forward answers at one revision | DIRECTORY-025 |
| `crates/lys-identity/src/grants/relationships.rs` | pure mapping from a committed grant event to SpiceDB relationship updates | DIRECTORY-025 |
| `crates/lys-identity/tests/grant_relationships.rs` | the mapping over fixture events | DIRECTORY-025 |
| `surface/identity/src/features/grants/PermissionWhy.tsx` | the why view, showing the server's answer and never deciding | DIRECTORY-025 |
| `surface/identity/tests/permission_why.test.tsx` | the why view against fixture answers | DIRECTORY-025 |
| `surface/identity/tests/acceptance/permission_why.spec.ts` | the why view against the standalone server | DIRECTORY-025 |

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

- **CN1** — Documents only: nothing outside docs/design/directory/ and docs/design/decisions.json is created or modified; the IDENTITY-001 files are not changed. Ruling (DIRECTORY-025): CN1 bounds DIRECTORY-001's documents-only work; each later brief of the cluster states its own file walls.
- **CN2** — Development isolation: rows 02 to 05 use only disposable test identities and test provider registrations; no production tokens, real business sign-in or live Cambium participant migration.
- **CN3** — Every path written in a document of this cluster is relative to the repository root, whatever directory a session starts in; a command runs from its own tree and spells its paths from there.
- **CN4** — No structure row or files entry carries a root token; a file in another repository is named in a requirement's spec with its owner.
- **CN5** — A live demonstration to Tom is never an acceptance criterion of a loop requirement; it is a verification step a person performs after the row lands.
- **CN6** — The live demonstrations ID001_LINK_LIVE (after row 03) and ID001_DIRECTORY_LIVE (after row 05) are mandatory operator hold points: the brief that follows each is blocked by it until Tom's demonstration receipt is recorded; a loop completion never stands in for one.
- **CN7** — Revision 5's ceiling stands: 48 focused implementer hours for IDENTITY-001 (row 01 1.5, closed; 02 8; 04 10; 03 10; 05 6; 06 4; 07 5; total 44.5, contingency 3.5), with IDENTITY-002's 4 hours outside it. An overrun is reported as soon as it is known, and Waffles takes any ceiling change to Tom (docs/design/identity/briefs/IDENTITY-001.json:24).
- **CN8** — One implementer, one row in implementation and one gate invocation at a time in this lane; release builds, checks and tests run through the gate workflow at the venue, and a development exception never bypasses it (docs/design/identity/briefs/IDENTITY-001.json:25-27, docs/design/identity/briefs/IDENTITY-001.json:135).
- **CN9** — A row that needs a file outside its wall stops and names it, and the reviewer approves a brief revision before that file is edited; a directory wall for a wholly new module allows only its named responsibility and needs an exact file manifest reviewed before its row starts (docs/design/identity/briefs/IDENTITY-001.json:28).
- **CN10** — Rows 02 to 05 run on Rauthy v0.36.2 under Waffles' ruling of 15:36:25; each development install checks current releases and advisories and records the accepted exception; IDENTITY-001-UPSTREAM-AUTH-STATE binds real sign-in and install, rows 06 and 07 (docs/design/identity/briefs/IDENTITY-001.json:18, docs/design/identity/briefs/IDENTITY-001.json:46).
- **CN11** — Step 1 is directory records, sign-in and the minimum signed identity audit. SpiceDB is installed and checked in row 02 and enforces nothing in step 1; live capability policy and its enforcement are step 2's, and running SpiceDB is not permission enforcement (docs/design/identity/briefs/IDENTITY-001.json:29-30). Ruling (ADR-078): permission enforcement through SpiceDB on the standalone identity server is a goal of this cluster, carried by DIRECTORY-025; DIRECTORY-002 R4 and the rest of step 2 are unchanged.
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
> - C5 — Every decision still open for Tom is recorded as open: the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting for the upstream release. Decided under ADR-078: the grant representation was ruled by the owning lead, Archie, with Apollo as second reader, on Waffles' ruling of 12:13 on Tom's word of 12:12 that technical formats are the owning lead's with a second reader.
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
- C5 — Every decision still open for Tom is recorded as open: the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting for the upstream release. Decided under ADR-078: the grant representation was ruled by the owning lead, Archie, with Apollo as second reader, on Waffles' ruling of 12:13 on Tom's word of 12:12 that technical formats are the owning lead's with a second reader.
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
  - [x] C5 — Every decision still open for Tom is recorded as open: the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting for the upstream release. Decided under ADR-078: the grant representation was ruled by the owning lead, Archie, with Apollo as second reader, on Waffles' ruling of 12:13 on Tom's word of 12:12 that technical formats are the owning lead's with a second reader. — Five non-goals, each marked OPEN for Tom once. None is decided.
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
id: DIRECTORY-025
cluster: directory
title: Road step 2: every permission check the identity server makes asks SpiceDB, and the screen answers why
---

# DIRECTORY-025: Road step 2: every permission check the identity server makes asks SpiceDB, and the screen answers why

> **Cluster:** directory
> **Depends on:** DIRECTORY-002, DIRECTORY-003, DIRECTORY-005, DIRECTORY-006
> **Blocked by:** DIRECTORY-006 is a brief on main, not a landed foundation. The grant representation ruled under ADR-078 (C5), together with DIRECTORY-006 R1 to R5 as built and accepted on the branch hand/DIRECTORY-006-R5 at e525a91, whose second reader's review items were fixed in its R5, stands as the ratification DIRECTORY-006 R1 and DIRECTORY-006's blocked_by ask for, and this brief waits on no separate contract review. The freshness mechanism (C25) is settled by the owning lead with a second reader as DIRECTORY-006 R4 builds it. The grant contract, the committed grant event, the root issue, the permission decision, the freshness rule, the explain seam and crates/lys-identity-server/src/lib.rs are DIRECTORY-006's, so R1 to R7 wait on DIRECTORY-006 landing on lys main, and for none of these on anything further. Check, run after git fetch origin main: git cat-file -e origin/main:crates/lys-identity/src/grants/authority.rs exits 0., DIRECTORY-002 is a brief, not a landed foundation: deploy/identity/versions.json, which R1 reads the pinned SpiceDB release from, and deploy/identity/README.md, which R3 writes its step-2 sentence into, are DIRECTORY-002's, and no other source for them is intended. R1 to R3, and R4 to R7 which follow them, wait on DIRECTORY-002 landing on lys main. Check, run after git fetch origin main: git cat-file -e origin/main:deploy/identity/versions.json exits 0., The typed capability claim an agent authenticates its calls with is card hhAN8h77's, whose brief DIRECTORY-012 creates the claim type in crates/lys-identity/src/capability/claim.rs. R5 does not dispatch until that claim type has landed; every other requirement proceeds without it. The blocker is keyed on that artifact: git cat-file -e origin/main:crates/lys-identity/src/capability/claim.rs exits 0 only once the claim type has landed. Run it after git fetch origin main: in a clone that holds no origin/main ref the command exits 128 whatever has landed, and that exit is not a reading of this blocker., DIRECTORY-005 is a brief, not a built foundation, and neither DIRECTORY-006's built branch nor DIRECTORY-002's creates surface/identity/, which R7 writes into; R7 does not dispatch until it has landed. Every modify path of this brief is reconciled to the foundations' landed manifests before dispatch (CN9, CN12). Check, run after git fetch origin main: git cat-file -e origin/main:surface/identity/src/routes.tsx exits 0.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-078 — Permission enforcement through SpiceDB on the standalone identity server enters the directory design, on a decided grant representation — Amend the design narrowly: permission enforcement through SpiceDB on the standalone identity server leaves the 'Road step 2 onward' non-goal and becomes a goal of the directory design, and CN11 gains one appended ruling line saying so. Everything else in the non-goal stays, and neither the non-goal's text nor CN11's is reworded; the non-goal's reason gains one appended sentence saying it does not cover that enforcement. The grant representation (C5) is a technical decision of the owning lead with a second reader, not one left open for Tom, and is decided here: every grant, root or delegated, is its own grant object in SpiceDB, keyed by its grant identifier, and is exactly 3 relationships: the resource's relationship to the grant object under the grant's action; the grant's holder relationship to the identity that holds it, carrying an expiry caveat that ends no later than its source's; and the grant's standing relationship, which for a delegated grant is its source relationship to its source grant, the source relation the permission walks, and for a root grant, which has no source, is its root relationship, the grant object's relationship to the directory's root authority, one object per directory and the same object the issue_root permission is checked against when a person issues a root grant through POST /grants/roots; a permission holds through a grant only while the grant's standing holds, a root grant's through its root relationship and a delegated grant's through its source grant's standing, so derivation is a walk of the source relation that ends at the directory's root authority in a bounded number of steps, a grant that stands on neither a source relationship nor a root relationship is malformed and refused, and deleting a grant's standing relationship refuses its holder and every grant derived from it at once; two live grants giving the same holder the same action on the same resource are two grant objects, each with its own relationships, and revoking one leaves the other standing; withdrawing a grant deletes its relationships and those of every grant derived from it through the source relation on the same committed revoke event, the withdrawn grant's standing relationship alone in the first write and the rest in further writes within SpiceDB's configured max_updates_per_write. The expiry bound is refused, never clamped: a delegation asking for an expiry later than its source grant's is refused by name before anything is committed, and a committed delegation event that nonetheless asks for one is refused by the projector by name, which writes no relationship for it and stops there; a silent clamp would make the record say one thing and the grant another. Rejected: sitting beside the non-goal as a named exception, which leaves the design's stated scope contradicting the brief; rewording the non-goal or CN11; and bringing Cambium's door into this cluster.
> **Checklist:**
> - C186 — The identity server reaches SpiceDB through one pure-Rust gRPC client that maps a fresh store's NOT_FOUND schema answer to a typed no-schema result, tests run against a disposable SpiceDB, and crates/lys/src/identity/ gains no SpiceDB check or write.
> - C187 — SpiceDB's schema is written only by the identity server at start-up, and its relationships only by projecting committed signed grant events; revoking a root refuses every grant derived from it and no unrelated grant; a delegation or a committed delegation event whose expiry passes its source grant's is refused by name, never clamped; every grant is its own grant object of 3 relationships, and two grants of one holder, action and resource stand apart; a root revoke deletes the revoked grant's standing relationship first and its remaining and derived relationships in writes within SpiceDB's configured update cap, resuming on replay after a crash.
> - C188 — Every grant and permission check the identity server makes, from browser, API and agent routes, is answered by SpiceDB through one evaluator behind DIRECTORY-006 R4's decision, at least as fresh as the projection, and an engine outage refuses by name; the administrator's admission by configured issuer and subject is the one check that never asks SpiceDB, and the install guide says so beside SpiceDB's unchanged step-1 sentence. An expired grant is refused by the caveat's time from the named clock, and an answer missing that context is refused, never admitted.
> - C189 — A call after a committed revoke is never admitted: before the projection catches up only a check that depends on the unapplied grant event is refused as not yet current, naming the grant, and unrelated authority stays usable; after it catches up the refusal names the withdrawn grant.
> - C190 — An agent whose permission is withdrawn between the two steps of a task is refused on its next call to the identity server.
> - C191 — The server answers why an identity can and why it cannot do a thing from the one evaluator's traced verdict, with the path to a responsible person, the named reason and the policy revision, and answers who can act on a resource through SpiceDB at the same revision, so the forward and reverse answers agree.
> - C192 — The screen shows the server's why answer for a permitted and a refused question and never decides a permission in the browser.
> **Stories:**
> - S77 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want an agent whose permission I withdraw in the middle of a task to be refused on its next call, so that the withdrawal takes effect at once and not when the task ends.
> - S78 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a grant holder or reviewer looking at an identity, I want the screen to answer why it can or cannot do a thing, with the path to a responsible person or the named reason, so that I can trust or correct its access from the server's own decision.
> - S79 (Operator, Installs and runs the standalone identity product) — As the operator, I want a permission check the permission projection has not yet caught up with to be refused by name, naming the grant, while unrelated checks keep being answered, so that a lagging projection never admits a call and never stops unrelated work.

## Purpose

Road step 2 of the directory design, for the standalone identity server: every grant and permission check it makes is answered by SpiceDB, and the browser shows what the server decided and is never the authority. The one named exception to every check is the administrator's admission (P9, C13), which stays with the configured issuer and subject and never asks SpiceDB, because that admission is how an operator bootstraps the directory before any relation exists. It is done when an agent that loses a permission between the two steps of a task is refused on its next call to the identity server, and the screen answers why an identity can, and why it cannot, do a thing. The reverse question, who can act on a resource, is answered by SpiceDB at the same revision as the forward answer, so the two agree. SpiceDB's relationships are a projection of the signed directory log and are never written on their own (P4). ADR-078 brings this enforcement into the directory design.

## Task

Implement R1 to R7 in order; no requirement depends on a later one. R1 (the client and a disposable SpiceDB) proceeds once the foundations land. R2 (schema and projection) follows R1. R3 (the one evaluator, which checks against R2's schema and relationships, and where the consistency rule is set) comes after R2. The grant representation (C5), which the SpiceDB schema is, is decided by the owning lead with Apollo as second reader and recorded in ADR-078, so neither R2 nor R3 waits on it: every grant, root or delegated, is its own grant object in SpiceDB, keyed by its grant identifier, and is exactly 3 relationships: the resource's relationship to the grant object under the grant's action; the grant's holder relationship to the identity that holds it, carrying an expiry caveat that ends no later than its source's; and the grant's standing relationship, which for a delegated grant is its source relationship to its source grant, the source relation the permission walks, and for a root grant, which has no source, is its root relationship, the grant object's relationship to the directory's root authority, one object per directory and the same object the issue_root permission is checked against when a person issues a root grant through POST /grants/roots; a permission holds through a grant only while the grant's standing holds, a root grant's through its root relationship and a delegated grant's through its source grant's standing, so derivation is a walk of the source relation that ends at the directory's root authority in a bounded number of steps, a grant that stands on neither a source relationship nor a root relationship is malformed and refused, and deleting a grant's standing relationship refuses its holder and every grant derived from it at once; two live grants giving the same holder the same action on the same resource are two grant objects, each with its own relationships, and revoking one leaves the other standing; withdrawing a grant deletes its relationships and those of every grant derived from it through the source relation on the same committed revoke event, the withdrawn grant's standing relationship alone in the first write and the rest in further writes within SpiceDB's configured max_updates_per_write. That ruling, with DIRECTORY-006 R1 to R5 as built and accepted, stands as the ratification DIRECTORY-006 R1 asks for, so no separate contract review is waited on. A person's root authority on a resource comes from a root grant the directory's root authority, one person fixed when the grants are opened, issues through DIRECTORY-006's root issue (Grants::issue_root, served as POST /grants/roots): a root grant has no source, and its expiry is the end its issuer states, or no end. Every fixture in which a person grants an agent an action on a resource first issues that person a root grant on that resource; this brief uses the root issue as DIRECTORY-006 builds it and adds no bootstrap of its own. R4 follows R3, refers back to R3's consistency rule, and answers under the freshness mechanism (C25) as the owning lead settled it with a second reader and DIRECTORY-006 R4 builds it. R3's agent route is DIRECTORY-006 R5's authenticated agent-tool route and does not use the typed capability claim; R5 is the one requirement whose calls authenticate with that claim. R5 follows R4 and waits on the typed capability claim. R6 follows R3, and R7 follows R6. Estimates, proposals for review: R1 3h, R2 3h, R3 3h, R4 3h, R5 1h, R6 3h, R7 3h, 19 hours in total. In scope: the standalone identity server (crates/lys-identity-server), the pure grant-to-relationship mapping in crates/lys-identity, the screen in surface/identity, and one step-2 sentence in the operator's install guide, deploy/identity/README.md, beside DIRECTORY-002 R4's step-1 sentence, which stays word for word (R3). 'Every check' means every grant and permission check; the administrator's admission (P9, C13) is the one named exception and stays with the configured issuer and subject. The reverse question DIRECTORY-006 R5 asks, who can act on a resource, is answered through SpiceDB's LookupSubjects at the same revision as the forward check (R6). This brief puts SpiceDB behind DIRECTORY-006 R4's permission decision and R5's explain seam and depends on DIRECTORY-006; it replaces neither, DIRECTORY-006 keeps owning the seams, and this brief owns what answers behind them. 'The server' is the standalone identity server only: the done condition is met for calls made to the identity server. The door (Cambium) asking SpiceDB on every call is a later Cambium card, and that card is the act that answers the statement's framing of step 2 (docs/design/identity/STATEMENT-2026-09-22.md:17 and :143); this brief names it and does not do it, the non-goal on rows 06 and 07 stands, and an agent calling through the door is not refused by this work until that card lands. This brief's checks read grants and nothing else: no check here reads an identity's lifecycle state or refuses a suspended agent, because ADR-011 is proposed and the meaning of suspension is open, and a check that refused on it would decide it here. The check that reads the state stays with the lifecycle card, a further unit not written here, taken up once ADR-011 is decided; ADR-011's consequence is the reason: 'DIRECTORY-003 records the state beside each identity; no check reads it until the permission work of road step 2.' Out of scope besides: the typed capability claim and any agent certificate (card hhAN8h77), capability certificates, arbitrary grants, credential handles, and any change to the Rauthy fork or its pin (ADR-009). DIRECTORY-002 R4 is held unchanged: SpiceDB's step-1 sentence stands word for word, and nothing in crates/lys/src/identity/ asks SpiceDB for a decision or writes to it. Every search over files that an acceptance line or the verification runs is a git grep over Rust source run from the repository root, excluding crates/lys-identity-server/proto/ and every tests/ directory by name, with its exact expected output; a count is piped through wc -l | tr -d ' ' so that it prints a bare number. Two checks stand outside that rule, and they are the only two: R1's cargo tree line, which counts -sys crates in the dependency graph and is not a search over files, prints its exact expected count; and R3's README line, the one search over a file that is not Rust source, is a git grep over deploy/identity/README.md alone, run from the repository root, that prints exactly one line, deploy/identity/README.md:1, beside a git diff --numstat of that file whose deletions count is 0.

## Requirements

### R1: Connect the identity server to SpiceDB through a pure-Rust gRPC client, with a disposable SpiceDB for tests

THE SYSTEM SHALL give crates/lys-identity-server one SpiceDB client in crates/lys-identity-server/src/spicedb/client.rs, speaking SpiceDB's v1 gRPC API through tonic and prost, with the authzed v1 API protocol files vendored under crates/lys-identity-server/proto/authzed/ from the one tagged authzed/api release that the SpiceDB release pinned in deploy/identity/versions.json builds against, whose tag and commit crates/lys-identity-server/proto/SOURCE.md names and compiled by crates/lys-identity-server/build.rs with protox, and with tonic, prost and protox declared in the root Cargo.toml's [workspace.dependencies], so that no protoc binary is needed to build and nothing R1 adds needs a non-Rust toolchain. Transport security to SpiceDB is backed by rustls, built without its default features and given the rustls-rustcrypto crypto provider, reached through tokio-rustls and tonic's transport with none of tonic's TLS features enabled; THE SYSTEM SHALL NOT declare, for any dependency R1 adds, a tonic, rustls, tokio-rustls or hyper-rustls feature that brings ring or aws-lc-rs, and no crate or feature R1 adds SHALL bring ring, aws-lc or a -sys crate into lys-identity-server's normal dependency graph. The graph R1 starts from already holds such crates from sources outside this brief's wall: ring through lys-core's rcgen, ring through the rustls behind DIRECTORY-003's openidconnect, and core-foundation-sys on macOS. They are one finding against lys-core and DIRECTORY-003, carried by a card of its own; R1 SHALL NOT change lys-core, SHALL NOT change DIRECTORY-003's dependencies, and SHALL NOT be measured on their removal. The SpiceDB address and preshared key SHALL come from the configuration DIRECTORY-002 declares. WHEN the client reads the schema of a store that holds none, SpiceDB answers ReadSchema with the gRPC status NOT_FOUND, and THE SYSTEM SHALL map that status to the typed no-schema result SchemaRead::NoSchema; it SHALL NOT report it as an error and SHALL NOT return it as an empty schema text. The preshared key SHALL NOT appear in Debug output, a log line or an error message. IF SpiceDB cannot be reached or answers with any other error, THEN THE SYSTEM SHALL return a typed error from crates/lys-identity-server/src/spicedb/error.rs naming the operation and the configured address, and SHALL NOT substitute any answer of its own. crates/lys-identity-server/src/spicedb/mod.rs carries only module declarations and re-exports. Tests SHALL run against a disposable SpiceDB started by crates/lys-identity-server/tests/spicedb_support/ from the SpiceDB release pinned in deploy/identity/versions.json, in its in-memory testing mode, one isolated store per test, each test presenting its own preshared key; no test SHALL reach a shared or long-lived SpiceDB. The tests' relationship writes are built inside crates/lys-identity-server/tests/spicedb_support/; client.rs SHALL NOT build a WriteRelationshipsRequest, a DeleteRelationshipsRequest or a WriteSchemaRequest. DIRECTORY-002 R4 is held as written: THE SYSTEM SHALL NOT add a SpiceDB permission check, relationship write or schema write anywhere in crates/lys/src/identity/; road step 2 lives in crates/lys-identity/ and crates/lys-identity-server/ instead.

**Acceptance:**
- With the PROTOC environment variable unset and no protoc on PATH, cargo build -p lys-identity-server exits 0.
- On the platform the gate's tests leg runs on, cargo tree -p lys-identity-server -e normal --prefix none | grep -E -e '-sys ' -e '^ring ' is run at the commit the build of R1 started from and again at R1's head; the line count printed at R1's head equals the line count printed at the start commit, and the proof shows both counts and both sets of matching lines.
- A test starts the disposable SpiceDB through spicedb_support and calls the client's schema read against the fresh store: SpiceDB answers the gRPC status NOT_FOUND, and the client returns exactly SchemaRead::NoSchema.
- Two tests running in parallel each write one relationship to their own disposable store; each then reads back exactly one relationship, its own.
- Run from the repository root, git grep --untracked -c -E -e 'WriteRelationshipsRequest|DeleteRelationshipsRequest|WriteSchemaRequest' -- 'crates/lys-identity-server/src/spicedb/client.rs' ':(exclude)crates/lys-identity-server/proto/' ':(exclude,glob)**/tests/**' | wc -l | tr -d ' ' prints 0.
- With the configured SpiceDB address pointing at a closed local port, a client schema read returns the unreachable-engine error variant, whose Display text contains the configured address and does not contain the configured preshared key.
- format!("{:?}", config) of a configuration whose preshared key is the fixture string spicedb-test-key-fixture does not contain spicedb-test-key-fixture.
- From the repository root, on the build branch: git diff --stat <base>..HEAD -- crates/lys/src/identity/ prints nothing, where <base> is the commit the build of R1 started from.

**Files:**
- create: crates/lys-identity-server/build.rs
- create: crates/lys-identity-server/proto/SOURCE.md
- create: crates/lys-identity-server/proto/authzed/
- create: crates/lys-identity-server/src/spicedb/mod.rs
- create: crates/lys-identity-server/src/spicedb/client.rs
- create: crates/lys-identity-server/src/spicedb/error.rs
- create: crates/lys-identity-server/tests/spicedb_support/mod.rs
- create: crates/lys-identity-server/tests/spicedb_support/server.rs
- modify: Cargo.toml
- modify: Cargo.lock
- modify: crates/lys-identity-server/Cargo.toml
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C186 — The identity server reaches SpiceDB through one pure-Rust gRPC client that maps a fresh store's NOT_FOUND schema answer to a typed no-schema result, tests run against a disposable SpiceDB, and crates/lys/src/identity/ gains no SpiceDB check or write.

### R2: Write the SpiceDB schema at start-up and project committed signed grant events into relationships

The SpiceDB schema in crates/lys-identity-server/src/spicedb/schema.zed is the grant representation ADR-078 records (C5): every grant, root or delegated, is its own grant object keyed by its grant identifier, and is exactly 3 SpiceDB relationships: (1) its resource relationship, the resource's relationship to the grant object under the grant's action; (2) its holder relationship, the grant object's relationship to the identity holding the grant, carrying the expiry caveat, which ends no later than its source grant's; and (3) its standing relationship, which for a delegated grant is its source relationship, the grant object's relationship to its source grant through the source relation, and for a root grant, which has no source, is its root relationship, the grant object's relationship to the directory's root authority, one object per directory and the same object the issue_root permission is checked against when a person issues a root grant through POST /grants/roots. The permission on a resource holds for an identity through a grant only when the identity is that grant's holder within its expiry and the grant stands, a root grant standing through its root relationship and a delegated grant standing through its source relationship while its source grant stands; derivation is therefore a walk of the source relation that SpiceDB makes itself and that ends at the directory's root authority in a bounded number of steps, and once a grant's standing relationship is deleted SpiceDB refuses its holder and the holder of every grant derived from it. Two live grants giving the same holder the same action on the same resource are two grant objects, each with its own 3 relationships; THE SYSTEM SHALL NOT merge them and SHALL NOT refuse the second, and revoking one deletes only its own relationships and those derived from it. A root grant, issued by the directory's root authority through DIRECTORY-006's root issue, has a root relationship and no source relationship, and its holder relationship's expiry caveat carries the end its issuer states, or no end when the issuer states none; THE SYSTEM SHALL NOT give a root grant an expiry other than the one its committed event records. Every grant stands on exactly one standing relationship: IF a committed grant event names neither a source grant nor issue by the directory's root authority, THEN THE SYSTEM SHALL treat it as malformed: the projector SHALL refuse to project it, reporting that refusal, whose words name the event's log position, through the same error it reports any leaf the projection refuses with, SHALL write no relationship for it, and SHALL stop there, leaving its recorded log position before that event and applying no later event, as it does for any leaf the projection refuses; THE SYSTEM SHALL NOT write a grant object that has no standing relationship, and SHALL NOT write a root relationship whose subject is anything other than the directory's root authority. The expiry bound is refused, never clamped: WHEN a delegation asks for an expiry later than its source grant's, THE SYSTEM SHALL refuse it in crates/lys-identity/src/grants/authority.rs, behind DIRECTORY-006 R2's delegation check, with the named refusal delegation_outlives_source, whose words name the source grant's identifier, the source grant's expiry and the requested expiry, before any grant event is committed; IF a committed delegation event nonetheless asks for an expiry later than its source grant's, THEN THE SYSTEM SHALL treat it as a corrupt or foreign event: the projector SHALL refuse to project it with the named refusal projection_expiry_past_source, whose words name the event's log position, the source grant's identifier and both expiries, SHALL write no relationship for it, and SHALL stop there, leaving its recorded log position before that event and applying no later event, as it does for any leaf the projection refuses. THE SYSTEM SHALL NOT clamp a requested expiry to its source's, and SHALL NOT write a relationship whose expiry caveat differs from the expiry its committed event records. WHEN the identity server starts, THE SYSTEM SHALL write the schema held in crates/lys-identity-server/src/spicedb/schema.zed to SpiceDB through crates/lys-identity-server/src/spicedb/schema.rs if the store holds no schema (SchemaRead::NoSchema, R1), and SHALL read the stored schema back; IF the stored schema differs from schema.zed, THEN THE SYSTEM SHALL refuse to start with the named error spicedb_schema_mismatch and SHALL NOT overwrite the stored schema. THE SYSTEM SHALL write SpiceDB relationships only by projecting committed signed grant events (the event DIRECTORY-006 R3 commits), through a pure mapping from event to relationship updates in crates/lys-identity/src/grants/relationships.rs and one writer in crates/lys-identity-server/src/spicedb/projector.rs, which applies each committed event once, in log order, and records the log position and the SpiceDB revision token it reached. In Rust source outside crates/lys-identity-server/proto/ and the tests/ directories, the scope of R2's searches, THE SYSTEM SHALL NOT write a relationship from a route, a screen or any file other than projector.rs, SHALL NOT write one for an event that is not committed, and SHALL NOT delete or rewrite a relationship except as the projection of a committed revoke or expiry event. WHEN the projector applies a committed revoke of a grant, THE SYSTEM SHALL keep the revoke one committed event, SHALL first delete that grant's standing relationship, alone, as the one update of one WriteRelationships call, so that no check through that grant or through any grant derived from it succeeds from then on, and SHALL then delete the grant's two remaining relationships and the 3 relationships of every grant derived from it through the source relation in further WriteRelationships calls, each carrying no more updates than the max_updates_per_write the deployment configures for SpiceDB, which the projector reads from the identity server's configuration, and each carrying as many of the remaining updates as that cap allows. Every such write SHALL be idempotent, and the projector SHALL NOT record its log position past the revoke event until a read of the store shows that no relationship of the withdrawn grant or of a grant derived from it remains, so that a projector stopped between two of those writes resumes the revoke on replay. THE SYSTEM SHALL NOT raise the configured max_updates_per_write for a revoke, SHALL NOT send a write carrying more updates than it, and SHALL NOT delete a relationship of any grant that does not derive from the withdrawn grant (ADR-003). The one relationship write site outside the projector is test-only: the tests' relationship writes are built inside crates/lys-identity-server/tests/spicedb_support/ (R1), reach only a test's own disposable store, and are not part of the built server. The expected relationships in the tests are written by hand from the recorded grant contract and schema, and are never produced by relationships.rs.

**Acceptance:**
- Starting the server against an empty disposable SpiceDB leaves a stored schema byte-equal to schema.zed; a second start against the same store writes nothing and starts.
- Starting the server against a store holding a schema that differs from schema.zed by one relation exits with spicedb_schema_mismatch, and the stored schema read back afterwards equals the differing schema.
- Fixture log in crates/lys-identity-server/tests/spicedb_projection.rs: the directory's root authority issues person P a root grant RP on project X and person Q a root grant RQ on project Y; person P grants agent A read on project X (grant G1, derived from RP); A delegates it to agent C (grant G2, derived from G1); C delegates it to agent D (grant G3, derived from G2); person Q grants agent E read on project Y (grant G4, of a different root: it derives from RQ, has its own subject and resource, and does not derive from G1); then G1 is revoked. The fixture holds, as the named constant EXPECTED_RELATIONSHIPS_BEFORE_REVOKE, the relationship list after G4, exactly 18 relationships, 3 for each of RP, RQ, G1, G2, G3 and G4 (its resource relationship, its holder relationship and its standing relationship), written by hand from the recorded grant contract and schema and not produced by relationships.rs. Replaying the log up to G4 into an empty store leaves exactly that list, compared as sorted lists.
- After replaying the log up to G4, a CheckPermission call the test makes directly against the store permits read on X for A, for C and for D, and permits read on Y for E: four permissions asserted one at a time.
- After replaying the whole fixture log through the revoke of G1, a CheckPermission call the test makes directly against the store refuses read on X for A, for C and for D, three refusals asserted one at a time, and still permits read on Y for E.
- The number of relationships of G1, G2 and G3 remaining in the store after the revoke of G1 is declared in crates/lys-identity-server/tests/spicedb_projection.rs as the named constant RELATIONSHIPS_AFTER_ROOT_REVOKE, with a comment naming C5 and ADR-078; its value is 0, and the test asserts that the count of the store's relationships of G1, G2 and G3, 9 before the revoke, equals that constant after it, and that the counts of RP's, RQ's and G4's relationships are 3 each before and after the revoke.
- With a counting wrapper around the SpiceDB client and the disposable SpiceDB and the projector both at SpiceDB's default max_updates_per_write, which is larger than 8, projecting the revoke of G1 makes exactly 2 WriteRelationships calls: the first carries exactly 1 update, deleting G1's standing relationship, its source relationship to RP, and no other relationship; the second carries exactly 8 updates, deleting G1's resource and holder relationships and the 3 relationships each of G2 and G3, and no relationship of RP, RQ or G4.
- With the disposable SpiceDB and the projector both configured with max_updates_per_write of 1, smaller than the 8 relationships the revoke deletes after its first write, projecting the revoke of G1 makes exactly 9 WriteRelationships calls, one for each of the 9 relationships of G1, G2 and G3, each carrying exactly 1 update, and the first deletes G1's standing relationship, its source relationship to RP, and nothing else. With the projector paused after that first call, a CheckPermission call the test makes directly against the store refuses read on X for A, for C and for D, three refusals asserted one at a time, and permits read on Y for E, and the count of the store's relationships of G1, G2 and G3 is 8. After the ninth call, the count of the store's relationships of G1, G2 and G3 equals RELATIONSHIPS_AFTER_ROOT_REVOKE, the counts of RP's, RQ's and G4's relationships are 3 each, and the projector's recorded log position equals the revoke's position.
- With max_updates_per_write of 1, a crash injected into the projector after the first WriteRelationships call of the revoke of G1 and before the second leaves the count of the store's relationships of G1, G2 and G3 at 8 and the projector's recorded log position before the revoke's position; restarting the projector over the same log and the same store then leaves the count of the store's relationships of G1, G2 and G3 equal to RELATIONSHIPS_AFTER_ROOT_REVOKE, the counts of RP's, RQ's and G4's relationships at 3 each, and the recorded log position equal to the revoke's position.
- The fixture gives the root grants RP and RQ the issued expiries named by the fixture constants E0 and E5, gives G1, G2 and G3 the expiries E1, E2 and E3, E3 before E2, E2 before E1 and E1 before E0, and gives G4 the expiry E4, before E5. In the store after replaying the log up to G4: RP's holder relationship carries the expiry caveat E0, and RP has a root relationship whose subject is the directory's root authority and no source relationship; RQ's holder relationship carries E5, and RQ has a root relationship whose subject is the same directory root authority object and no source relationship; G1's holder relationship carries E1 and its source relationship names RP; G2's carries E2 and its source relationship names G1; G3's carries E3 and its source relationship names G2; G4's carries E4 and its source relationship names RQ; and none of G1, G2, G3 and G4 has a root relationship. EXPECTED_RELATIONSHIPS_BEFORE_REVOKE holds these caveats, root relationships and source relationships, written by hand.
- With the fixture's grant G1 carrying expiry E1, a delegation of G1 from agent A to agent C asking for the expiry E1 plus one hour is refused with delegation_outlives_source, its words containing G1's identifier, E1 and the requested expiry; the count of grant events committed by that request is 0 and the count of relationships in the store is unchanged.
- A fixture log whose committed events are the root grant RP with expiry E0, then G1 derived from RP with expiry E1, then a delegation event G2 derived from G1 carrying the expiry E1 plus one hour, then the root grant RQ, then G4, replayed into an empty store: the projector refuses with projection_expiry_past_source, its words containing G2's log position, G1's identifier, E1 and G2's expiry; the store holds RP's and G1's relationships and no relationship of G2, RQ or G4; and the projector's recorded log position equals G1's position.
- A fixture log whose committed events are the root grant RP, then a grant event G7 giving agent A read on project X that names no source grant and was not issued by the directory's root authority, then the root grant RQ, replayed into an empty store: the replay returns a projection refusal for G7 through the same error the projector returns for any leaf the projection refuses, the error it returns projection_expiry_past_source through, and that refusal's words contain G7's log position; the store holds RP's 3 relationships and no relationship of G7 or RQ, the store holds no grant object without a standing relationship, and the projector's recorded log position equals RP's position.
- A second fixture log in crates/lys-identity-server/tests/spicedb_projection.rs: the directory's root authority issues person P a root grant RP on project X; P grants agent A read on project X twice, as two grant events with two grant identifiers, G5 and G6, each derived from RP; then G5 is revoked. After replaying the log up to G6 the store holds exactly 9 relationships, 3 each of RP, G5 and G6. After replaying the whole log, a CheckPermission call the test makes directly against the store permits read on X for A, through G6; the count of the store's relationships of G5 is 0, and the counts of RP's and G6's relationships are 3 each.
- Replaying the same fixture log a second time into the same store changes no relationship and leaves the recorded log position equal to the log's head.
- A fixture log holding one grant event that is appended but not committed projects zero relationships for that event.
- Run from the repository root, git grep --untracked -l -E -e 'WriteRelationshipsRequest|DeleteRelationshipsRequest' -- '*.rs' ':(exclude)crates/lys-identity-server/proto/' ':(exclude,glob)**/tests/**' prints exactly one line, crates/lys-identity-server/src/spicedb/projector.rs.
- Run from the repository root, git grep --untracked -l -e 'WriteSchemaRequest' -- '*.rs' ':(exclude)crates/lys-identity-server/proto/' ':(exclude,glob)**/tests/**' prints exactly one line, crates/lys-identity-server/src/spicedb/schema.rs.

**Files:**
- create: crates/lys-identity-server/src/spicedb/schema.rs
- create: crates/lys-identity-server/src/spicedb/schema.zed
- create: crates/lys-identity-server/src/spicedb/projector.rs
- create: crates/lys-identity/src/grants/relationships.rs
- create: crates/lys-identity/tests/grant_relationships.rs
- create: crates/lys-identity-server/tests/spicedb_projection.rs
- modify: crates/lys-identity/src/grants/mod.rs
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/tests/grant_delegation.rs
- modify: crates/lys-identity-server/src/spicedb/mod.rs

**Checklist:**
- C187 — SpiceDB's schema is written only by the identity server at start-up, and its relationships only by projecting committed signed grant events; revoking a root refuses every grant derived from it and no unrelated grant; a delegation or a committed delegation event whose expiry passes its source grant's is refused by name, never clamped; every grant is its own grant object of 3 relationships, and two grants of one holder, action and resource stand apart; a root revoke deletes the revoked grant's standing relationship first and its remaining and derived relationships in writes within SpiceDB's configured update cap, resuming on replay after a crash.

### R3: Answer every grant and permission check the identity server makes through the one SpiceDB evaluator, at least as fresh as the projection

This row checks against R2's schema and relationships. WHEN the identity server makes a permission check, from the browser route, the API route or the agent route, THE SYSTEM SHALL answer its grant question through DIRECTORY-006 R4's permission decision (crates/lys-identity/src/grants/permission.rs), and that decision SHALL get its answer from SpiceDB's CheckPermission through crates/lys-identity-server/src/spicedb/check.rs, the one evaluator. check.rs is the only site that calls CheckPermission: no other Rust source file outside crates/lys-identity-server/proto/ and the tests/ directories builds a CheckPermissionRequest or contains the text check_permission(. It offers the plain check, which returns the verdict, and a traced variant, which returns the verdict with SpiceDB's check trace and the checked_at revision token SpiceDB answered at, and which takes an optional revision token: given none it reads under the consistency rule's first form, and given a token it reads at_exact_snapshot of that token; every other module that needs SpiceDB's verdict consumes one of those two and SHALL NOT call CheckPermission itself. The consistency rule: each CheckPermission call SHALL ask for consistency at_least_as_fresh the revision token the projector (R2) recorded for the last grant event it applied, or at_exact_snapshot of a checked_at token that an earlier call made under the first form returned, which is at least as fresh as the projector's token that call asked for; it SHALL NOT ask at minimize_latency or at any consistency weaker than the projector's token. The expiry caveat R2's schema carries is evaluated with context check.rs supplies: on every CheckPermission call, the plain check and the traced variant alike, check.rs SHALL send as the caveat's context the current time, read from the named clock DIRECTORY-006 R4 enforces expiry with, which check.rs takes as an injected clock so that a test controls it; a grant whose caveat's expiry is at or before that time is refused, and one whose expiry is after it is permitted as its relationships allow. IF CheckPermission answers CONDITIONAL_PERMISSION, the answer SpiceDB gives when a caveat's context is missing, THEN THE SYSTEM SHALL refuse with the named refusal permission_conditional and SHALL NOT admit the call. THE SYSTEM SHALL NOT make a CheckPermission call without the caveat's context, and SHALL NOT read the caveat's time from any source other than that clock. The agent route in this row is DIRECTORY-006 R5's authenticated agent-tool route, authenticated as DIRECTORY-006 authenticates it; this row SHALL NOT depend on card hhAN8h77's typed capability claim, and the blocker on that card stays on the requirement that uses it. Every grant and permission check makes exactly one CheckPermission call through check.rs. The one named exception: the administrator's admission, made by the configured issuer and subject (P9, C13), stays with that configuration and SHALL NOT ask SpiceDB, because it is how an operator bootstraps the directory before any relation exists; THE SYSTEM SHALL NOT treat any other check as an exception. WHEN this row lands, THE SYSTEM SHALL write into deploy/identity/README.md, beside DIRECTORY-002 R4's step-1 sentence, this step-2 sentence word for word: "SpiceDB's step-2 role, in one sentence: from road step 2 on, SpiceDB answers every grant and permission check the identity server makes and holds the directory's relations, projected from the signed directory log, and the administrator's admission by its configured issuer and subject is the one check that never asks it." It SHALL NOT change or remove a word of the step-1 sentence. DIRECTORY-006 keeps owning the decision seam; this row supplies what answers behind it and replaces no DIRECTORY-006 requirement. THE SYSTEM SHALL NOT answer a grant question from a cache, a local copy of the relationships, the browser's request, a display label or any evaluator other than check.rs, and SHALL NOT admit a call when SpiceDB gives no answer: IF CheckPermission fails or cannot be reached, THEN THE SYSTEM SHALL refuse with the named refusal permission_engine_unavailable.

**Acceptance:**
- With a counting wrapper around the SpiceDB client, one permitted and one refused request on each of the browser route, the API route and the agent route, DIRECTORY-006 R5's authenticated agent-tool route with no typed capability claim presented (six requests) produce exactly six CheckPermission calls, and each route's answer equals the answer CheckPermission gave.
- Every CheckPermission call the wrapper records in the six-request case carries at_least_as_fresh with the revision token the projector recorded for the last grant event it applied; the count of calls carrying minimize_latency is 0.
- Run from the repository root, git grep --untracked -l -e 'CheckPermissionRequest' -e 'check_permission(' -- '*.rs' ':(exclude)crates/lys-identity-server/proto/' ':(exclude,glob)**/tests/**' prints exactly one line, crates/lys-identity-server/src/spicedb/check.rs.
- A request permitted while the disposable SpiceDB runs is refused with permission_engine_unavailable after SpiceDB is stopped, and the count of admitted calls after the stop is 0.
- The root authority issues person P a root grant on project X whose expiry is after the fixture constant E plus one second; grant G, from P to agent A for read on X, derived from that root grant, carries the expiry caveat at E, and check.rs's clock is under the test's control: a check of A's read on X through check.rs's plain check with the clock at E minus one second is permitted, with the clock at E is refused, and with the clock at E plus one second is refused; the counting wrapper records exactly one CheckPermission call for each of the three checks, and each call carries the clock's time as the caveat's context. The same three checks through the traced variant give the same three verdicts, again with exactly one CheckPermission call each.
- With the counting wrapper set to strip the caveat's context from each request before it reaches SpiceDB, the check of A's read on X through check.rs with the clock at E minus one second receives CONDITIONAL_PERMISSION from SpiceDB and is refused with permission_conditional, and the count of admitted calls is 0.
- A directory mutation request made by the configured administrator issuer and subject against a disposable SpiceDB holding no relationship is admitted, and the counting wrapper records 0 CheckPermission calls for it.
- Run from the repository root, git grep --untracked -c -F -e "SpiceDB's step-2 role, in one sentence: from road step 2 on, SpiceDB answers every grant and permission check the identity server makes and holds the directory's relations, projected from the signed directory log, and the administrator's admission by its configured issuer and subject is the one check that never asks it." -- deploy/identity/README.md prints exactly one line, deploy/identity/README.md:1, and run from the repository root on the build branch, git diff --numstat <base>..HEAD -- deploy/identity/README.md prints a deletions count of 0, where <base> is the commit the build of R3 started from.

**Files:**
- create: crates/lys-identity-server/src/spicedb/check.rs
- create: crates/lys-identity-server/tests/spicedb_checks.rs
- modify: crates/lys-identity/src/grants/permission.rs
- modify: crates/lys-identity-server/src/spicedb/mod.rs
- modify: deploy/identity/README.md

**Checklist:**
- C188 — Every grant and permission check the identity server makes, from browser, API and agent routes, is answered by SpiceDB through one evaluator behind DIRECTORY-006 R4's decision, at least as fresh as the projection, and an engine outage refuses by name; the administrator's admission by configured issuer and subject is the one check that never asks SpiceDB, and the install guide says so beside SpiceDB's unchanged step-1 sentence. An expired grant is refused by the caveat's time from the named clock, and an answer missing that context is refused, never admitted.

### R4: Never admit a call after a committed revoke, refuse only what an unapplied grant event affects, and say which refusal it is

The freshness mechanism (C25) is the one DIRECTORY-006 R4 builds, as the owning lead settled it with a second reader: a check is answered only from a projection that has applied every committed revocation it depends on, and otherwise it is refused with StaleDecision. This row carries that rule behind SpiceDB in crates/lys-identity-server/src/spicedb/freshness.rs, and SHALL NOT change DIRECTORY-006 R4's rule: WHILE the projector's recorded log position (R2) is behind the directory log's committed head of grant events, WHEN a permission check depends on a committed grant event the projector has not applied (the event creates, delegates, revokes or expires a grant on the checked identity's path to the resource), THE SYSTEM SHALL refuse that check with StaleDecision (GrantError::StaleDecision in crates/lys-identity/src/grants/error.rs), whose words name the revision the decision needs, which is the log position of the unapplied event, the revision the permission relationships stand at, and the affected grant's identifier. DIRECTORY-006 builds that variant carrying the required and projected revisions and no grant; this row adds the affected grant's identifier to it and to its words, and keeps both revisions: THE SYSTEM SHALL NOT remove, rename or re-type the required or the projected revision. That refused check still makes its one CheckPermission call through check.rs under R3's consistency rule, so that every check asks SpiceDB; THE SYSTEM SHALL discard that verdict and SHALL NOT admit that call, even when the verdict permits. A check that depends on no unapplied grant event SHALL stay usable and be answered through R3 under R3's consistency rule, as DIRECTORY-006 R3 requires of unrelated authority; THE SYSTEM SHALL NOT refuse it because the projector is behind. WHEN the projection has applied the revoke and the answer is no because a grant on the path was withdrawn by a committed revoke, THE SYSTEM SHALL refuse with permission_revoked naming the withdrawn grant's identifier. A call after a committed revoke SHALL NOT be admitted in either case. WHEN a question asked of DIRECTORY-006 R5's explain seam depends on a committed grant event the projector has not applied, THE SYSTEM SHALL answer it with that same StaleDecision refusal, naming the affected grant, and SHALL NOT return a path, a verdict or a reason for it.

**Acceptance:**
- With person P holding a root grant on project X and P's grant G of read on X to agent A, derived from that root grant, permitting A's request and the projector paused, commit a revoke of G: A's next request is refused with StaleDecision, its words contain G's identifier, the log position of the revoke as the revision the decision needs, and the projector's recorded log position as the revision the relationships stand at; the counting wrapper records exactly 1 CheckPermission call for that refused request and that call's verdict was permitted; and the count of admitted calls after the revoke is 0.
- With the projector still paused behind that one unapplied revoke of G, a request by agent E under person Q's grant H of read on project Y, derived from Q's root grant on Y, which shares no identity, no resource and no ancestry with G, is permitted; in the same state A's request under G is refused with StaleDecision naming G.
- With the projector still paused behind that one unapplied revoke of G, the question why can A read X, asked of DIRECTORY-006 R5's explain seam, returns StaleDecision naming G, and no path.
- Resume the projector until its recorded position equals the log head: A's next identical request is refused with permission_revoked, its words contain G's identifier, and the count of A's admitted calls after the revoke is still 0.

**Files:**
- create: crates/lys-identity-server/src/spicedb/freshness.rs
- create: crates/lys-identity-server/tests/spicedb_freshness.rs
- modify: crates/lys-identity/src/grants/error.rs
- modify: crates/lys-identity-server/src/spicedb/mod.rs

**Checklist:**
- C189 — A call after a committed revoke is never admitted: before the projection catches up only a check that depends on the unapplied grant event is refused as not yet current, naming the grant, and unrelated authority stays usable; after it catches up the refusal names the withdrawn grant.

**Stories:**
- S79 (Operator, Installs and runs the standalone identity product) — As the operator, I want a permission check the permission projection has not yet caught up with to be refused by name, naming the grant, while unrelated checks keep being answered, so that a lagging projection never admits a call and never stops unrelated work.

### R5: Refuse an agent's next call when its permission is withdrawn between the two steps of a task

WHEN an agent has made the first call of a two-step task to the identity server under a grant, and that grant is revoked through the signed event path before its second call, THE SYSTEM SHALL refuse the second call through R3 and R4, and SHALL NOT admit it on the strength of the first call's answer, a session, a connection kept open or any answer given before the revoke. The agent authenticates each call with the typed capability claim card hhAN8h77 delivers; this row SHALL NOT issue, define or change that claim or any certificate, and SHALL NOT be dispatched until the claim type has landed (blocked_by). The refusal's words are those R4 gives: StaleDecision naming the grant while the projection is behind the revoke, and permission_revoked naming the grant once it has applied it.

**Acceptance:**
- A scripted two-step task by one agent: call 1 under grant G is admitted and changes one record; G is revoked through the signed event path; call 2, the same request, is refused. The test counts 2 calls made, 1 admitted, 1 refused, and exactly 1 record changed.
- The same script with the projector paused across the revoke refuses call 2 with StaleDecision, its words containing G's identifier, and counts 1 admitted call and 0 admitted calls after the revoke.
- The same script with the projector caught up refuses call 2 with permission_revoked, its words containing G's identifier, and counts 1 admitted call and 0 admitted calls after the revoke.

**Files:**
- create: crates/lys-identity-server/tests/agent_mid_task.rs

**Checklist:**
- C190 — An agent whose permission is withdrawn between the two steps of a task is refused on its next call to the identity server.

**Stories:**
- S77 (Responsible person, Signs in and provisions agents under their own authority) — As the responsible person, I want an agent whose permission I withdraw in the middle of a task to be refused on its next call, so that the withdrawal takes effect at once and not when the task ends.

### R6: Answer why an identity can, and why it cannot, do a thing, and who can, from the one evaluator at one revision, behind DIRECTORY-006 R5's explain seam

WHEN the explain seam DIRECTORY-006 R5 exposes (crates/lys-identity-server/src/grants.rs) is asked whether an identity can perform an action on a resource, THE SYSTEM SHALL answer in crates/lys-identity-server/src/spicedb/explain.rs from one call to check.rs's traced variant (R3), mapping SpiceDB's check trace onto the directory's lineage. explain.rs consumes that variant and SHALL NOT call CheckPermission itself or build a CheckPermission request, and SHALL NOT be a second evaluator. A yes SHALL carry the path of identities from the identity to a responsible person, the resource, the action, and the policy revision: the SHA-256 of schema.zed and the revision token the answer was read at. A no SHALL carry the named reason (permission_revoked, with the withdrawn grant's identifier, when a revoke is the reason; no_grant when no grant covers the question) and the same policy revision. The explanation's verdict SHALL be the traced variant's verdict; THE SYSTEM SHALL NOT compute an explanation from anything but that answer, and SHALL NOT answer a why when the check refuses as permission_engine_unavailable, returning that refusal instead; a check refused as not current has no path to explain. WHEN the explain seam is asked who can perform an action on a resource, THE SYSTEM SHALL answer through crates/lys-identity-server/src/spicedb/lookup.rs, the only site that calls LookupSubjects: no other Rust source file outside crates/lys-identity-server/proto/ and the tests/ directories builds a LookupSubjectsRequest or contains the text lookup_subjects(. Every answer to one question set, the questions asked of the explain seam together, SHALL be read at one revision T: the set's first forward traced check calls check.rs's traced variant with no token, so it reads at_least_as_fresh the projector's token under R3's consistency rule, and T is the checked_at token it returns; every later forward traced check of the set calls the traced variant with T, reading at_exact_snapshot T, and every LookupSubjects call of the set asks for consistency at_exact_snapshot T. This fits R3's consistency rule because T was returned by a call made at least as fresh as the projector's token. WHEN a question set holds only a who-can question, its first LookupSubjects call asks for consistency at_least_as_fresh the projector's token, T is the looked_up_at token it returns, and every later LookupSubjects call of the set asks at_exact_snapshot T. So an identity is in the who-can answer if and only if check.rs's traced variant, called with T for that identity, the action and the resource, answers yes. lookup.rs SHALL NOT compute a verdict of its own and SHALL NOT be a second evaluator. DIRECTORY-006 R5 keeps the reverse question's visibility permission and paging; this row supplies the answer behind them.

**Acceptance:**
- Fixture: the root authority issues person P a root grant on project X; P grants agent A the action read on X (grant PA); identity B holds no grant on X. Why can A read X answers yes with the path [A, P], P marked as the responsible person, resource X, action read, the SHA-256 of schema.zed and a non-empty revision token.
- After grant PA is revoked and projected, why can A read X answers no with reason permission_revoked and PA's identifier.
- Why can B read X answers no with the reason no_grant.
- In each of those three cases the counting wrapper records exactly one CheckPermission call, made by check.rs's traced variant, and the explanation's verdict equals the verdict R3's plain check returns for the same identity, action, resource and revision.
- Run from the repository root, git grep --untracked -c -e 'CheckPermissionRequest' -e 'check_permission(' -- 'crates/lys-identity-server/src/spicedb/explain.rs' ':(exclude)crates/lys-identity-server/proto/' ':(exclude,glob)**/tests/**' | wc -l | tr -d ' ' prints 0.
- With R6's fixture of P, A, B and X, one question set asks, in this order, why can A read X, why can B read X and who can read X. The counting wrapper records the set's first CheckPermission call, for A, carrying at_least_as_fresh the projector's recorded token, and T is the checked_at token that call returns; the set's second CheckPermission call, for B, carries at_exact_snapshot T; every LookupSubjects call of the set carries at_exact_snapshot T, the count of such calls being greater than 0; and the three answers each report the revision token T. The test then calls check.rs's traced variant with T once for each of P, A and B, action read, resource X, three calls each carrying at_exact_snapshot T; each of P, A and B is in the who-can answer if and only if its call answers yes; A is in it and B is not.
- Run from the repository root, git grep --untracked -l -e 'LookupSubjectsRequest' -e 'lookup_subjects(' -- '*.rs' ':(exclude)crates/lys-identity-server/proto/' ':(exclude,glob)**/tests/**' prints exactly one line, crates/lys-identity-server/src/spicedb/lookup.rs.

**Files:**
- create: crates/lys-identity-server/src/spicedb/explain.rs
- create: crates/lys-identity-server/src/spicedb/lookup.rs
- create: crates/lys-identity-server/tests/spicedb_explain.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/spicedb/mod.rs

**Checklist:**
- C191 — The server answers why an identity can and why it cannot do a thing from the one evaluator's traced verdict, with the path to a responsible person, the named reason and the policy revision, and answers who can act on a resource through SpiceDB at the same revision, so the forward and reverse answers agree.

### R7: Show on the screen why an identity can or cannot do a thing, read from the server's answer

THE SYSTEM SHALL give surface/identity a view, surface/identity/src/features/grants/PermissionWhy.tsx, reached from the identity's record through surface/identity/src/routes.tsx, where a person picks an action and a resource and the screen shows the server's R6 answer: for yes, the path of identities to the responsible person, the resource, the action and the policy revision; for no, the named reason, the grant it concerns when there is one, and the policy revision. The browser shows what is permitted and is never the authority: THE SYSTEM SHALL NOT compute, infer or cache a verdict in the browser from grant lists, labels or earlier answers, SHALL show no verdict until the server has answered, and SHALL show the server's refusal by name when the server answers StaleDecision or permission_engine_unavailable, and an unreachable refusal when the server cannot be reached. The view follows Aion's appearance with the identity orange (ADR-010).

**Acceptance:**
- With the explain API answering R6's yes fixture, the view shows A, P marked responsible, read, X and the policy revision the response carries.
- With the page's loaded grant list containing grant PA from P to A on X and the explain API answering no with permission_revoked and PA's identifier, the view shows refused, permission_revoked and PA's identifier, and shows no yes.
- Asking one question sends exactly 1 explain request; before its response arrives the view shows no verdict.
- With the explain API unreachable, the view shows the unreachable refusal and no verdict.
- With the explain API answering StaleDecision, the view shows StaleDecision and no verdict.
- The browser acceptance test runs against the standalone server with this fixture, extending R6's fixture of P, A and B: person P, a second person Q, agent A and identity B; the root authority issues P a root grant on project X and Q a root grant on project Y; P grants A read on project X (grant PA); Q grants B read on project Y (grant QB); B holds no grant on X. Permitted question: can A read X; expected verdict yes, its reason the path A, P with P the responsible person; the server answers yes and the view shows permitted with the path A, P, P marked responsible. Refused question: can B read X; expected verdict no, its reason no_grant; the server answers no and the view shows refused with the reason no_grant. For each question the view's verdict and reason equal the server's response.

**Files:**
- create: surface/identity/src/features/grants/PermissionWhy.tsx
- create: surface/identity/tests/permission_why.test.tsx
- create: surface/identity/tests/acceptance/permission_why.spec.ts
- modify: surface/identity/src/routes.tsx

**Checklist:**
- C192 — The screen shows the server's why answer for a permitted and a refused question and never decides a permission in the browser.

**Stories:**
- S78 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a grant holder or reviewer looking at an identity, I want the screen to answer why it can or cannot do a thing, with the path to a responsible person or the named reason, so that I can trust or correct its access from the server's own decision.

## Boundaries

- The card is built from this brief only after Tom or the lead signs it off on the card; no requirement is dispatched before that sign-off is recorded.
- SHALL NOT change DIRECTORY-002 R4, its step-1 README sentence, or crates/lys/src/identity/; deploy/identity/README.md gains only R3's step-2 sentence beside the step-1 sentence, and step 2's checks, relationship writes and schema writes live in crates/lys-identity/ and crates/lys-identity-server/.
- SHALL NOT change the Rauthy fork, its files or its pin at vendor/rauthy (ADR-009).
- SHALL NOT decide suspension semantics and SHALL NOT change the freshness mechanism: C25 is the one DIRECTORY-006 R4 builds, as the owning lead settled it with a second reader. The grant representation is the one ADR-078 records (C5), and this brief changes nothing of it. SHALL NOT add a root-authority bootstrap: a root grant comes from DIRECTORY-006's root issue as built.
- SHALL NOT add a second permission evaluator: check.rs is the only site that calls CheckPermission, it offers the plain check and the traced variant, explain.rs consumes the traced variant and never calls CheckPermission itself, and lookup.rs is the only site that calls LookupSubjects, at the forward answer's revision.
- SHALL NOT move the administrator's admission (P9, C13) to SpiceDB; it is the one named exception to every check asking SpiceDB.
- SHALL NOT replace or rewrite a DIRECTORY-006 requirement; this brief answers behind DIRECTORY-006's seams.
- SHALL NOT touch Cambium, the door or any other repository, and SHALL NOT issue or define an agent certificate or capability claim. An agent calling through the door is not refused by this work until the later Cambium card lands.
- SHALL NOT change lys-core or any published wire format.
- The browser never computes, infers, caches or grants a permission; it shows the server's answer.
- No real credentials, production SpiceDB or shared store in tests; every test uses a disposable SpiceDB (ADR-004).

## Verification

- python3 scripts/design/render-cluster.py docs/design/directory is run, and the change includes the rendered docs/design/directory/USER-STORIES.md, docs/design/directory/CHECKLIST.md, docs/design/directory/DESIGN.md and docs/design/directory/briefs/DIRECTORY-025.md.
- sh scripts/design/gate.sh exits 0 from the repository root at the published commit.
- At implementation time run the repository battery at the exact revision: cargo fmt --all, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps, each exiting 0.
- Report the count of CheckPermission calls, admitted calls and refused calls each acceptance test measured, not only that it passed.
- One evaluator: run from the repository root, git grep --untracked -l -e 'CheckPermissionRequest' -e 'check_permission(' -- '*.rs' ':(exclude)crates/lys-identity-server/proto/' ':(exclude,glob)**/tests/**' prints exactly one line, crates/lys-identity-server/src/spicedb/check.rs, and git grep --untracked -c -e 'CheckPermissionRequest' -e 'check_permission(' -- 'crates/lys-identity-server/src/spicedb/explain.rs' ':(exclude)crates/lys-identity-server/proto/' ':(exclude,glob)**/tests/**' | wc -l | tr -d ' ' prints 0: explain.rs consumes check.rs's traced variant.
- DIRECTORY-002 R4 held: run from the repository root, git grep --untracked -c -E -e 'CheckPermission|WriteRelationships|DeleteRelationships|WriteSchema' -- 'crates/lys/src/identity/*.rs' ':(exclude)crates/lys-identity-server/proto/' ':(exclude,glob)**/tests/**' | wc -l | tr -d ' ' prints 0.

