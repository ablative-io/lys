# directory — what was asked, what it means, and what was written

## The words, as they were typed

Conformance row 8.3 says the graph draws the same answers as Access and never computes its own. Access is the permission check DIRECTORY-006 builds, which answers yes with a path or no with a reason and the model version (row 8.1), and lists who can reach a resource (row 8.2). The mock-up at docs/design/identity/mockup/index.v5.html shows a graph of people, agents, grants and resources. DIRECTORY-006 defers the graph to a brief of the lead's that was never written.

This card builds the graph screen in surface/identity/. Every edge and every reachability it draws comes from an Access answer returned by the directory, and the graph holds no rule of its own. It draws a path exactly as Access returns it, and a "no" as Access's reason. When the person asks the graph who can reach a resource, it asks Access and draws the answer it gets. The graph names the model version that Access gave for what it shows.

Acceptance is that the graph screen's code calls no permission logic of its own, checked by a test that the graph module imports nothing from the permission engine except the Access client. For a fixture directory, every edge the graph draws matches a path Access returns for the same question, compared by one test over every pair in the fixture. A grant revoked in the fixture disappears from the graph on its next draw, because Access no longer returns it. The model version shown matches the one Access returned. The brief names row 8.3 as the row it passes.

It waits for DIRECTORY-005 and DIRECTORY-006 to land, each checked by a command a stranger can run against lys main. Filed by Archie, lead for the identity line, on 27 September 2026, against lys main 7b536253.

Rulings of the lead, Archie, given on 27 September 2026 to the run 031db72d-e077-435b-96b0-2185adbefe5c in answer to its rounds. That run took every answer and then failed before writing, when the account pool refused every session. They are settled here, and the author reopens none of them.

It draws what Access says, and nothing more. When Access's answer carries a grant as not standing, with its reason, the graph draws it as the mock-up's dashed 'does not stand' edge with that reason. When Access no longer returns a grant at all, it disappears. The graph never infers that a grant does not stand on its own. The brief corrects the sentence to say this, and acceptance lines cover a suspended holder drawn as not standing with Access's reason, and a grant Access no longer returns gone on the next draw. Answered by Archie, lead for the identity line.

They stay, drawn from the directory's records. A resource's parent and an agent's responsible person are recorded facts, not rules, and the graph draws them as recorded. Grant edges and every reachability come only from Access answers. The brief amends the sentence to say exactly that split, and the Containment and 'Who answers for whom' toggles stay. Answered by Archie.

Before a question, it shows the people, agents and resources the person may see, with containment and answers-to edges from the records, and no grant edges. Choosing a node asks Access about it and draws the answer. No grant edge is ever drawn without an Access answer behind it. Answered by Archie.

A person opens the graph over what they may see, their own reach, under DIRECTORY-006 R6. The reverse question, who can reach a resource, needs its own visibility permission under R5. A directory administrator with that permission sees the whole directory. A hidden grant is never drawn or hinted at, and a question the person may not ask is refused by name. Acceptance lines cover a person's view, an administrator's view and a refused reverse question. Answered by Archie.

Show each answer's version. Every edge carries the model version of the Access answer it came from, and when a draw holds more than one version, a line above the graph says so and names each. The graph neither refuses a mixed picture nor re-asks. Answered by Archie.

Navigating to the graph, reloading it, or pressing a visible refresh control. The graph does not re-ask on its own. It shows the time of its draw beside the version, so a person knows how old what they see is. Answered by Archie.

This card adds the graph's deep links, #/graph/<id>, and the 'Show in graph' links on the screens that exist. The rail entry and the g h shortcut belong to the shell card AL4OyeKG, which registers every screen's route and key, and this card names it. Until the shell lands, the graph is reached by its deep link and the 'Show in graph' links. Answered by Archie.

The lifecycle transitions brief on brief/directory/lifecycle-hand keeps DIRECTORY-009, because it wrote the id first and its build is running. This brief takes the next free DIRECTORY id past lys main and every open brief and draft branch. It does not take DIRECTORY-014 by arithmetic, because the sessions brief d5055cc1 holds DIRECTORY-014 and other open branches may hold more. Every C, S, RM and ADR id it holds is renumbered the same way. The author reads every branch head with git ls-remote and git show immediately before writing, records the heads and ids read in the dev record, follows the new ids through every file and cross-reference, and scripts/design/gate.sh passes. Answered by Archie, lead for the identity line.

## What the survey found, and its angles

Archie wants a brief for the access graph screen in surface/identity/. The graph draws only what Access (the permission check DIRECTORY-006 builds) returns: every grant edge, path, refusal reason and model version, each stamped with the time of the draw. Resource containment and each agent's responsible person are drawn from recorded facts, and the graph holds no permission rule of its own. The brief passes conformance row 8.3. It adds the #/graph/<id> deep links and the 'Show in graph' links, leaves the rail entry and the g h shortcut to shell card AL4OyeKG, and takes the next free DIRECTORY id and C, S, RM and ADR ids past lys main and every open branch.

### What the tree holds

- `docs/design/directory/briefs/DIRECTORY-006.json` — Defines Access. R5 exposes a forward explanation (why-permitted: path, responsible person, scope, policy revision; why-refused: the blocking condition) and a reverse enumeration that needs its own visibility permission. R6 says 'The access graph consumes the same explanation response; its rendering belongs to Archie's graph brief', and the Task puts 'graph renderer' out of scope. This brief is the deferred one.
- `docs/design/directory/briefs/DIRECTORY-005.json` — Creates surface/identity/ as a wholly new module whose exact file manifest is reviewed before the row starts (CN9). It builds the directory list and record detail, the screens that would carry the 'Show in graph' links, and its verification needs frontend checks with strict types and the generated API schema.
- `docs/design/identity/mockup/index.v5.html` — Graph reference at lines 1309-1353: graphModel(), a fixed-seed force layout (seed 7, 420 iterations), viewGraph(), three toggles (Grants, Containment, Who answers for whom) and a legend of grant / does not stand (dashed) / inside / answers to. It computes standing(g) (line 420), can() (line 431), reach() and whoCan() in the browser. That local computation is exactly what row 8.3 forbids the built graph to do. 'Show in graph' appears once, in the people drawer at line 1206.
- `docs/design/identity/CONFORMANCE.md` — Row 8.3 ('The graph draws the same answers as Access; it never computes its own', Brief: DIRECTORY (new row), Owner: Archie) is the row this brief passes. Rows 8.1 and 8.2 define the Access answers, 9.1 covers the shell's deep links, and the build order puts the graph in step 7.
- `docs/design/directory/design.json` — The cluster design this brief continues: principles P1-P9, constraints CN1-CN12 (CN9 is the file wall, CN12 says don't dispatch a dependency-blocked brief), and the structure table, which gains a row for every surface/identity graph path the brief names.
- `docs/design/directory/checklist.json and stories.json` — Main tops out at C30 and S12. The new brief's checklist and story ids must go past every open branch, not just main.
- `docs/design/roadmap.json` — Main's highest row is RM-016, and open branches reach at least RM-029. The graph's roadmap row takes the next free RM id.
- `docs/design/decisions.json` — Main tops out at ADR-018 and open branches reach at least ADR-039. Any new decision (the graph as a pure renderer of Access answers) takes the next free id.
- `scripts/design/gate.sh` — The design leg: validate, check-coverage and a byte-identical render of every cluster. It has to pass on the brief's tree.
- `surface/identity/ (does not exist)` — Where the graph code is meant to land. Missing on main 7b53625 and on fa3dd53, and so are crates/lys-identity and crates/lys-identity-server.

### What was already decided

- DIRECTORY-006 R5 — One authenticated seam for list, read, delegate, revoke and explain. Forward and reverse answers use the same evaluator and revision. The reverse question needs its own visibility permission. Hidden private grants never leak ids, labels or existence.
- DIRECTORY-006 R6 — The You page is personal scope, and an independently authorised administrator sees wider. The access graph consumes the same explanation response, and its rendering is deferred to Archie's graph brief.
- DIRECTORY-006 R1 GRANT_MODEL — The decision keeps its model version. The words need this to show 'the model version that Access gave'.
- DIRECTORY-005 R1 — surface/identity/ is created with its manifest reviewed first. It is served by lys-identity-server routes.rs and assets.rs, with boundary types generated from the server schema.
- CONFORMANCE 8.1/8.2/8.3 — Access answers yes with a path to a person, or no with a named reason and the model version. 'Who can reach' is built from the same answers. The graph draws them and never computes its own.
- CN9 — A wholly new module wall needs an exact file manifest reviewed before its row starts. A file outside the wall stops the row.
- CN12 — DIRECTORY-006 is only executable after the DIRECTORY-002 and DIRECTORY-003 foundations are implemented. Don't dispatch from a schema-valid but dependency-blocked brief.
- P5 — No success before durable evidence. Pending or refused outcomes stay visible.
- ADR-010 — Aion structure, typography and interaction, with the identity product's orange accent (#D4975A). No build dependency on Cambium or Aion, and no purple.
- ADR-011 (proposed) — Four states: registered, active, suspended, retired. A suspended identity keeps its grants but they are not effective. The mock-up draws node state (filled when active, dashed when retired).
- design non-goal: suspension semantics — What stops with a suspended identity is OPEN for Tom, and the states are only proposed.
- DIRECTORY-009 (brief/directory/lifecycle-hand 28e4603a) — Moves identities between lifecycle states by signed record and gates every check on the folded state. It depends on DIRECTORY-003 and DIRECTORY-006.

### What was measured

- lys main head at survey time vs the words' base: fa3dd531 vs 7b536253: two commits later (lys ca row 6.4). Neither touches docs/
- DIRECTORY briefs on main: 8 briefs: DIRECTORY-001 to 006 and 008. No 007
- DIRECTORY ids seen on open branches whose objects are local: 007 (63bd2f2e), 009 (lifecycle-hand), 011 (28d15c0d), 013 (1e30a4cb). The words say 014 is held by d5055cc1
- Open directory brief and draft branches on origin: 47. The heads of 39 are not in the local object store, so this read-only survey could not read their ids
- Highest ids seen (main / readable open branches): C30 / C71, S12 / S34, RM-016 / RM-029, ADR-018 / ADR-039, CN12 / CN21. The branch maxima are lower bounds
- surface/, crates/lys-identity, crates/lys-identity-server, deploy/ on main: 0 of 4 exist
- Mentions of 'resource' in the DIRECTORY-003, 005 and 006 brief JSON: 0, 0 and 9. The directory records brief records no resources
- Mock-up graph code: about 45 lines (1309-1353) of the 197,852-byte index.v5.html. 3 toggles, 4 legend edge kinds, 'Show in graph' linked from 1 screen (people drawer, line 1206), 7 lines naming #/graph
- Measured gate legs in this tree: 7 (fmt, 2 clippy, tests, 2 doc, design). None runs a frontend or TypeScript check
- Estimate of the mock-up's DIRECTORY-006 R6 for comparison: 3h, of 17h across DIRECTORY-006

### What it means for the other projects

- cambium — The brief gets a card on the identity board that stays blocked until DIRECTORY-005 and DIRECTORY-006 land. It names the shell card AL4OyeKG as the owner of the rail entry and the g h key. The surface takes no build dependency on Cambium (ADR-010).
- aion — The card runs through Aion's chain (brief_card, sign-off, card_build_v3, src_pr, src_land). The graph follows Aion's shell structure and interaction with the orange accent, and imports no Aion package.
- method — The brief has to validate against the design-system schemas and render byte-identically with the method scripts vendored at 3c3bac7 (scripts/design).

### The decisions it stands on

- ADR-003 (honour) — Every drawn grant path ends at a person because Access returns it that way. The graph adds no authority and removes none.
- ADR-010 (honour) — The graph screen uses Aion's structure with the identity orange and no Cambium or Aion build dependency. It keeps the mock-up's legend colours for the grant and 'does not stand' edges.
- ADR-011 (honour) — Nodes show recorded lifecycle state as the directory records it. A suspended holder's grant is drawn as not standing only when Access says so, and suspension semantics stay open for Tom.
- ADR-004 (honour) — The graph works in the standalone product with Cambium, Manifold and Aion absent.
-  (new) — A proposed decision (next free ADR id): the access graph is a renderer of Access answers and directory records only. It holds no permission rule, never infers standing, and never re-asks on its own. Grant edges and reachability come only from Access, and containment and answers-to edges only from records.

### What it requires

- The brief is written at the next free DIRECTORY id past lys main and every open brief and draft branch (not 009, not 014), with C, S, RM and ADR ids renumbered the same way and the branch heads read recorded in the dev record.
- The brief names CONFORMANCE row 8.3 as the row it passes.
- A test fails if the graph module imports anything from permission logic other than the Access client, and passes on the built module.
- One test compares every edge the graph draws for a fixture directory with the path Access returns for the same question, over every pair in the fixture, and asserts how many pairs it compared.
- A grant revoked in the fixture is absent on the next draw because Access no longer returns it.
- A suspended holder's grant is drawn as the dashed 'does not stand' edge carrying Access's reason, verbatim.
- Before any question, the graph shows the people, agents and resources the person may see, with containment and answers-to edges from records and zero grant edges.
- Every drawn edge carries the model version of the Access answer it came from. When a draw holds more than one version, a line above the graph names each.
- The draw time is shown beside the version. The graph re-asks only on navigation, reload or the visible refresh control.
- A person's view shows their own reach, an administrator's view with the reverse permission shows the whole directory, and a reverse question without permission is refused by name. Each has its own acceptance line.
- No hidden grant's id, label or existence appears in the drawn graph or its DOM.
- #/graph/<id> deep links open the graph focused on that node, and 'Show in graph' links exist on each screen present at build time.
- The brief names shell card AL4OyeKG as the owner of the rail entry and the g h shortcut.
- sh scripts/design/gate.sh passes on the brief's tree, and the repository battery passes at implementation time.

### What must not change

- The graph contains no permission rule: nothing like the mock-up's standing(), can(), reach() or whoCan() is ported into surface/identity.
- No grant edge is drawn without an Access answer behind it, and standing is never inferred locally.
- DIRECTORY-006's Access contract and DIRECTORY-005's surface foundation are not rewritten from inside this card; any change goes through their own brief revision (CN9).
- DIRECTORY-009 stays with brief/directory/lifecycle-hand and DIRECTORY-014 with d5055cc1. No existing checklist row, story, brief or ADR id is reused or edited.
- docs/design/identity/IDENTITY-001 files and the mock-up are not changed. The mock-up's sample data and simulated confirmations are not behaviour.
- No change to lys-core or to any published wire format.
- The rail entry and the g h shortcut are not implemented here; they belong to AL4OyeKG.
- No build dependency on Cambium or Aion. The accent is the identity orange, with no Aion-blue and no purple.

### What we must put in place first

- DIRECTORY-005 lands on lys main with surface/identity/ and its reviewed manifest, route seam and generated types. It rests on DIRECTORY-002, 003 and 004, none of which is implemented.
- DIRECTORY-006 lands with Access (R5 forward and reverse explanations carrying the model version) and R6's personal and administrator scope. It is itself blocked on the open grant representation and the freshness review.
- A frontend check leg (strict types, tests) exists in the measured gates. Today's 7 legs run no TypeScript.
- The heads of the 39 open directory branches whose objects are not local are fetched and read, so the next free ids can be settled.

### The risks

- The dependency chain is long: DIRECTORY-002 to 006 are all unimplemented, and DIRECTORY-006 carries open blockers for Tom and for review, so the card may sit blocked for a long time.
- Access as specified may give the graph nothing to draw for 'choose a person' or for grants that do not stand, which could force an amendment to DIRECTORY-006.
- Resource containment may turn out to be a permission-model relation rather than a directory record, which would blur the records-versus-Access split the rulings draw.
- The suspended-holder acceptance may silently depend on DIRECTORY-009's lifecycle fold and on suspension semantics that are still open.
- Id collision: most open branches were unreadable here, and ids move while the brief is being written.
- A fixture comparison built from the same fixture source as the Access stub could agree with itself. The fixture's expected paths need a second party (Access's real evaluator).
- Main has moved past the words' base (7b536253 to fa3dd531). It touches no docs today, but the author must rebase on the main they land on.

### Still open

- The directory records no resources, so where do resource nodes and their containment edges come from: a read of the permission model through Access, or a new directory resource record that some card must add first? The sentence of the words it stands on: "A resource's parent and an agent's responsible person are recorded facts, not rules, and the graph draws them as recorded.". Why only the lead can settle it: docs/design/directory/briefs/DIRECTORY-003.json mentions resources zero times. Resources and their parents exist only in DIRECTORY-006's permission model (R1 GRANT_MODEL, R2 GRANT_ANCESTRY). In the mock-up, can() walks ancestors(r), so containment feeds reachability there. Which answer is chosen decides whether resources and containment appear at all before a question, and whose visibility governs them.
- When a person or agent node is chosen, which Access question is asked? A forward check for every visible resource and action, or a 'what can this identity reach' enumeration that DIRECTORY-006 R5 must gain first? The sentence of the words it stands on: "Choosing a node asks Access about it and draws the answer.". Why only the lead can settle it: DIRECTORY-006 R5 defines a forward explanation for one action on one resource and a reverse enumeration per resource. It has no per-identity reach operation, and the mock-up's reach() computes one locally. The choice changes what the person sees (completeness, paging, latency) and whether DIRECTORY-006 needs amending.
- Must Access's answers list grants that do not stand, each with its reason (an amendment to DIRECTORY-006 R5), or is the dashed edge drawn only when a forward refusal names a blocking grant? The sentence of the words it stands on: "When Access's answer carries a grant as not standing, with its reason, the graph draws it as the mock-up's dashed 'does not stand' edge with that reason.". Why only the lead can settle it: DIRECTORY-006 R5 returns a permit path, or a refusal naming its blocking condition. Neither the forward nor the reverse contract carries a list of grants that exist but do not stand, and the mock-up gets that list from its local standing(g). Without it, the dashed edge and the suspended-holder acceptance line have nothing to draw from.
- Does the card also wait for DIRECTORY-009 (lifecycle fold), since a suspended holder drawn as not standing needs Access to know about suspension? The sentence of the words it stands on: "It waits for DIRECTORY-005 and DIRECTORY-006 to land, each checked by a command a stranger can run against lys main.". Why only the lead can settle it: brief/directory/lifecycle-hand (28e4603a) DIRECTORY-009 'gate[s] every check on the folded state' and depends on DIRECTORY-006. The design's non-goals keep suspension semantics OPEN for Tom, and ADR-011 is only proposed. The acceptance line for a suspended holder may not be satisfiable on DIRECTORY-005 and DIRECTORY-006 alone.
- Does the mock-up's third toggle, 'Grants', stay (hiding or showing the Access answers already drawn), or is it removed? The sentence of the words it stands on: "The brief amends the sentence to say exactly that split, and the Containment and 'Who answers for whom' toggles stay.". Why only the lead can settle it: index.v5.html line 1348 has three toggles: grants, parents and answers. The rulings keep two and say nothing about the third, and a person would see a different control set depending on the answer.

### The units beyond the first

- Access answers who an identity can reach and which grants do not stand, with reasons (DIRECTORY-006 R5 amendment) — If the lead rules that choosing a node needs a per-identity reach answer or a list of non-standing grants, it is a change to Access's contract, owned by DIRECTORY-006, not the graph.
- Record resources and their containment in the directory — The rulings draw containment from records, but no directory brief records resources today. If that is the ruling, the record is its own row.
- The rail entry and the g h shortcut for the graph (shell card AL4OyeKG) — The rulings assign it to the shell card, which registers every screen's route and key.
- 'Show in graph' links on screens that land later (lifecycle, secrets, roles) — This card adds links only to screens that exist. Each later screen's card adds its own.
- Record row 8.3's brief in CONFORMANCE.md — CONFORMANCE.md is in docs/design/identity, outside the directory cluster's CN1 document wall.

### The smallest complete shape

One brief under docs/design/directory/briefs at the next free DIRECTORY id, rendered, with its checklist, stories, structure rows and roadmap row (plus the ADR if taken), gate.sh passing. It specifies the whole graph screen in surface/identity/: record-drawn nodes and containment and answers-to edges before a question; Access-drawn grant edges, paths, dashed not-standing edges and reasons on choosing a node or asking the reverse question; a version on every edge, a mixed-version line and the draw time; re-asking only on navigation, reload or refresh; person, administrator and refused-reverse views; #/graph/<id> and the 'Show in graph' links. Its acceptance lines are the import-boundary test, the every-pair fixture comparison, a revoked grant gone, a suspended holder drawn as not standing, version match, and the three views. It is blocked on DIRECTORY-005 and DIRECTORY-006 and passes CONFORMANCE row 8.3.

## The roadmap row

- **RM-059** — Draw the access graph from Access's answers only (conformance 8.3) (feature, idea)
- Summary: The access graph screen in surface/identity/, passing conformance row 8.3: grant edges and every reachability come only from Access answers (DIRECTORY-006 R5), a resource's parent (served read-only from the permission model, a hidden parent marked withheld and never named) and an agent's responsible person are drawn as recorded, and the graph holds no rule of its own. Choosing a person or agent asks the forward question for each resource and action its visible grants carry; choosing a resource asks the reverse question. Paths are drawn as returned and a no as Access's reason; a grant a forward refusal names is drawn dashed with Access's reason; a grant Access no longer returns is gone on the next draw; every edge carries its answer's model version and each draw its time. Reached by #/graph/<id> and a Show in graph link on each screen that shows one identity and on the grant explanation, which opens the holder's node with that grant's edge selected. Proved by an import-boundary test, one test over every fixture pair against the real evaluator, a revoked and an expired grant drawn dashed with Access's reason, and the version match. Waits for DIRECTORY-005 and DIRECTORY-006 to land on lys main.
- Asked by: tom on 2026-09-27T14:24:00+10:00
- Context: The graph card filed by the lead for the identity line against lys main 7b536253, surveyed, and answered by the lead in nine rulings and five answers to the survey's questions: resources and their parents read from the permission model through a read-only route that records nothing, under the grants' visibility; a chosen identity drawn from the forward question per resource and action its visible grants carry, with no new reach operation; a dashed does not stand edge only where a forward refusal names the grant, with no amendment to Access; no wait on DIRECTORY-009, the suspended-holder line moved to a further unit; and all three toggles kept. Recorded as ADR-098.
- Quote: Conformance row 8.3 says the graph draws the same answers as Access and never computes its own. Access is the permission check DIRECTORY-006 builds, which answers yes with a path or no with a reason and the model version (row 8.1), and lists who can reach a resource (row 8.2). The mock-up at docs/design/identity/mockup/index.v5.html shows a graph of people, agents, grants and resources. DIRECTORY-006 defers the graph to a brief of the lead's that was never written.

This card builds the graph screen in surface/identity/. Every edge and every reachability it draws comes from an Access answer returned by the directory, and the graph holds no rule of its own. It draws a path exactly as Access returns it, and a "no" as Access's reason. When the person asks the graph who can reach a resource, it asks Access and draws the answer it gets. The graph names the model version that Access gave for what it shows.

Acceptance is that the graph screen's code calls no permission logic of its own, checked by a test that the graph module imports nothing from the permission engine except the Access client. For a fixture directory, every edge the graph draws matches a path Access returns for the same question, compared by one test over every pair in the fixture. A grant revoked in the fixture disappears from the graph on its next draw, because Access no longer returns it. The model version shown matches the one Access returned. The brief names row 8.3 as the row it passes.

It waits for DIRECTORY-005 and DIRECTORY-006 to land, each checked by a command a stranger can run against lys main. Filed by Archie, lead for the identity line, on 27 September 2026, against lys main 7b536253.

Rulings of the lead, Archie, given on 27 September 2026 to the run 031db72d-e077-435b-96b0-2185adbefe5c in answer to its rounds. That run took every answer and then failed before writing, when the account pool refused every session. They are settled here, and the author reopens none of them.

It draws what Access says, and nothing more. When Access's answer carries a grant as not standing, with its reason, the graph draws it as the mock-up's dashed 'does not stand' edge with that reason. When Access no longer returns a grant at all, it disappears. The graph never infers that a grant does not stand on its own. The brief corrects the sentence to say this, and acceptance lines cover a suspended holder drawn as not standing with Access's reason, and a grant Access no longer returns gone on the next draw. Answered by Archie, lead for the identity line.

They stay, drawn from the directory's records. A resource's parent and an agent's responsible person are recorded facts, not rules, and the graph draws them as recorded. Grant edges and every reachability come only from Access answers. The brief amends the sentence to say exactly that split, and the Containment and 'Who answers for whom' toggles stay. Answered by Archie.

Before a question, it shows the people, agents and resources the person may see, with containment and answers-to edges from the records, and no grant edges. Choosing a node asks Access about it and draws the answer. No grant edge is ever drawn without an Access answer behind it. Answered by Archie.

A person opens the graph over what they may see, their own reach, under DIRECTORY-006 R6. The reverse question, who can reach a resource, needs its own visibility permission under R5. A directory administrator with that permission sees the whole directory. A hidden grant is never drawn or hinted at, and a question the person may not ask is refused by name. Acceptance lines cover a person's view, an administrator's view and a refused reverse question. Answered by Archie.

Show each answer's version. Every edge carries the model version of the Access answer it came from, and when a draw holds more than one version, a line above the graph says so and names each. The graph neither refuses a mixed picture nor re-asks. Answered by Archie.

Navigating to the graph, reloading it, or pressing a visible refresh control. The graph does not re-ask on its own. It shows the time of its draw beside the version, so a person knows how old what they see is. Answered by Archie.

This card adds the graph's deep links, #/graph/<id>, and the 'Show in graph' links on the screens that exist. The rail entry and the g h shortcut belong to the shell card AL4OyeKG, which registers every screen's route and key, and this card names it. Until the shell lands, the graph is reached by its deep link and the 'Show in graph' links. Answered by Archie.

The lifecycle transitions brief on brief/directory/lifecycle-hand keeps DIRECTORY-009, because it wrote the id first and its build is running. This brief takes the next free DIRECTORY id past lys main and every open brief and draft branch. It does not take DIRECTORY-014 by arithmetic, because the sessions brief d5055cc1 holds DIRECTORY-014 and other open branches may hold more. Every C, S, RM and ADR id it holds is renumbered the same way. The author reads every branch head with git ls-remote and git show immediately before writing, records the heads and ids read in the dev record, follows the new ids through every file and cross-reference, and scripts/design/gate.sh passes. Answered by Archie, lead for the identity line.
- Cluster: directory; briefs: DIRECTORY-034
- Notes: Further units, left for later and not written: Draw a suspended holder's grant as not standing from Access's answer, attached to the lifecycle card (DIRECTORY-009) and written when suspension is decided and folded; The rail entry and the g h shortcut for the graph (shell card AL4OyeKG); 'Show in graph' links on screens that land later (lifecycle, secrets, roles); Record row 8.3's brief in CONFORMANCE.md. Dev record of ids: DIRECTORY-034, RM-059, ADR-098, C272 to C278 and S121 to S123, each past the highest on lys main and on every branch head of origin, read with git ls-remote and git show immediately before writing (271 heads listed under refs/heads at the first read and 273 at the second; the 7 heads whose objects were not local at the first read and the 3 new or moved heads at the second were fetched, 0 unreadable; every head's docs/design tree read: the directory cluster's briefs, every cluster's checklist.json and stories.json, roadmap.json and decisions.json, for DIRECTORY, C, S, RM and ADR ids, and a grep of every head's docs for any later DIRECTORY, RM or ADR id found none above those listed here). Main at a426b9a5 holds DIRECTORY-008, RM-016, ADR-018 and, in the directory cluster, C30 and S12. The highest read immediately before writing, on heads other than this card's own branch: DIRECTORY-033, C271 and S120 on draft/directory/844a587d-204d-42f2-b549-3ea03fde5ab1 at 5bc10ac6 (which also holds RM-058 and ADR-097); DIRECTORY-032, C263 and S118 on draft/directory/95308eae-1fc8-47bd-9c77-cd2afb2c3849 at 2c4b56ba (which also holds RM-057); RM-058 and ADR-097 on draft/lys-log-store/6826e0cf-1720-4a68-b4d4-cfbf009258aa at 7e0d7568. Next below them: RM-057 and ADR-096 on draft/home/d948c6a9-b3cf-4597-a007-63a0f3f5c5a0 at 0d9fbde9 and on draft/secrets/0e6892c4-8dfc-4981-8a2e-e473be2fb388 at 2285baa4; ADR-095 on draft/secrets/7a0935db-d08e-4142-ad92-1c1212189f2a at 6f866d3e; DIRECTORY-031, C256 and S115 on brief/directory/8a60bd7a-9093-4b1a-b103-b39bb729a490 at 14e834a1 and brief/directory/c7c2ac1b-02b4-4af2-b3aa-d46d3dd488e7 at 440e6cc8. This round renumbered the card's previous draft on this branch, which carried DIRECTORY-032, RM-057, ADR-096, C257 to C263 and S116 to S118 and collided with draft/directory/95308eae-1fc8-47bd-9c77-cd2afb2c3849, draft/home/d948c6a9-b3cf-4597-a007-63a0f3f5c5a0 and draft/secrets/0e6892c4-8dfc-4981-8a2e-e473be2fb388; earlier drafts of this card carried DIRECTORY-031, RM-056, ADR-093, C248 to C254 and S112 to S114, and before that DIRECTORY-028, RM-055, ADR-092, C242 to C248 and S110 to S112. Those ids are retired by this card and it holds none of them. Mapping this round: DIRECTORY-032 to DIRECTORY-034, RM-057 to RM-059, ADR-096 to ADR-098, C257..C263 to C272..C278, S116..S118 to S121..S123. A second git ls-remote after the first writing found three heads new or moved, each fetched and read with git show: draft/directory/844a587d-204d-42f2-b549-3ea03fde5ab1 moved from 2c6736fa to 5bc10ac6 and now holds DIRECTORY-033, C271, S120, RM-058 and ADR-097, so this card moved from DIRECTORY-033, C264..C270 and S119..S121, which it had taken in the same round and now holds none of; brief/lys-log-store/0e2bb567-cef9-4a99-995a-b53b1df1fbda at 1e8e2c2c (ADR-060, directory cluster at C30 and S12) and brief/home/8d27e91f-fcaa-4778-b2b2-8200931d9f89 at c599f189 (RM-056, ADR-093, directory cluster at C30 and S12) are new and hold nothing higher.

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
- ADR-098 — The access graph draws Access's answers and the directory's records, and holds no permission rule — Grant edges and every reachability the graph draws come only from Access answers returned by the directory, paths exactly as returned and a no as Access's reason. Choosing a person or agent reads the grants it holds that the caller may see and asks the forward question for each resource and action they carry; choosing a resource asks the reverse question for each action the resource declares. A grant is drawn as the dashed does not stand edge only when a forward refusal names it, with Access's reason; a grant Access no longer returns disappears on the next draw; the graph never infers that a grant does not stand. A resource's parent and an agent's responsible person are recorded facts drawn as recorded: parents from the permission model through a read-only route that records nothing, serves each resource's declared actions and the model version it read them under, and shows a resource only where the caller holds a grant on it or an ancestor, or is a directory administrator, who sees every resource; a hidden parent is served as withheld, drawn as a mark with no containment edge and never named; responsible people come from the directory's records. The graph holds no rule of its own and asks Access only on navigation, reload or its refresh control. Rejected: porting the mock-up's raw-grant edges, its standing reckoning and its reach(); amending Access to enumerate reach or list grants that do not stand; a new directory record for resources; serving a resource with a hidden parent as a root; and dropping the containment and answers-to edges because they are not Access answers.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.
- DIRECTORY-034 draws the access graph (conformance 8.3) with grant edges and every reachability from Access's answers, and responsible people and resource parents as recorded, holding no permission rule of its own (ADR-098).

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
- The graph's rail entry and g h shortcut — The shell card AL4OyeKG registers every screen's route and key; DIRECTORY-034 adds only the graph's deep links and its Show in graph links.

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
| `crates/lys-identity-server/src/grant_resources.rs` | The read-only route serving the permission model's resources with their parents, declared actions and model version, for the access graph (conformance 8.3); created on top of DIRECTORY-006, reconcile against its landed manifest before dispatch | DIRECTORY-034 |
| `crates/lys-identity-server/tests/grant_resources.rs` | The read-only route serving the permission model's resources with their parents, declared actions and model version, for the access graph (conformance 8.3); created on top of DIRECTORY-006, reconcile against its landed manifest before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/GraphScreen.tsx` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/graphModel.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/recordEdges.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/graphQuestions.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/graphLayout.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/graph/ShowInGraphLink.tsx` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/tests/graph_boundary.test.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/tests/graph.test.tsx` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/tests/acceptance/graph.spec.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/tests/fixtures/graph-directory.ts` | The access graph from Access's answers only (conformance 8.3); created on top of DIRECTORY-005 and DIRECTORY-006, reconcile against their landed manifests before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/people/Preview.tsx` | The directory's preview drawer (DIRECTORY-005); DIRECTORY-034 adds its Show in graph link. Named at the path the identity surface was built at; reconcile against DIRECTORY-005's landed manifest before dispatch | DIRECTORY-034 |
| `surface/identity/src/features/file/IdentityFile.tsx` | The identity record screen (DIRECTORY-005); DIRECTORY-034 adds its Show in graph link. Named at the path the identity surface was built at; reconcile against DIRECTORY-005's landed manifest before dispatch | DIRECTORY-034 |
| `.gitignore` | DIRECTORY-034 adds two entries, crates/lys-identity-server/tests/grant_resources.ids.json and surface/identity/tests/fixtures/graph-directory.ids.json: the access graph tests' label-to-id maps, uncommitted run outputs each test also removes before it exits | DIRECTORY-034 |
| `docs/design/directory/briefs/DIRECTORY-034.json` | the access graph brief (conformance 8.3) | DIRECTORY-034 |
| `docs/design/directory/briefs/DIRECTORY-034.md` | rendered markdown | DIRECTORY-034 |

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
id: DIRECTORY-034
cluster: directory
title: Draw the access graph from Access's answers only (conformance row 8.3)
---

# DIRECTORY-034: Draw the access graph from Access's answers only (conformance row 8.3)

> **Cluster:** directory
> **Depends on:** DIRECTORY-005, DIRECTORY-006
> **Blocked by:** DIRECTORY-005 landed on lys main, shown by a command anyone can run from any directory: d=$(mktemp -d) && git clone https://github.com/ablative-io/lys "$d" && cd "$d" && test -d surface/identity && grep -rq 'ID001_STANDALONE' surface/identity && grep -rq 'ID001_SCREEN_REFUSAL' surface/identity exits 0. At the commit this brief was written against it exits 1, because surface/identity does not exist., DIRECTORY-006 landed on lys main, shown by a command anyone can run from any directory: d=$(mktemp -d) && git clone https://github.com/ablative-io/lys "$d" && cd "$d" && test -f crates/lys-identity-server/src/grants.rs && grep -q 'GRANT_EXPLAIN' crates/lys-identity-server/tests/grant_explanations.rs && test -f surface/identity/src/generated/index.ts && grep -rq 'GRANT_SCREEN' surface/identity/tests exits 0. At the commit this brief was written against it exits 1, because crates/lys-identity-server does not exist., Reconcile R1 against DIRECTORY-006 as landed before dispatch: if the identity server already serves the permission model's resources with their parents (a hidden parent marked withheld), their declared actions and the model version, read-only and under R1's visibility rule, R1's files become that route's and R1's acceptance is run against it unchanged; otherwise R1 adds the route as written., A frontend test command for surface/identity that anyone can run from a fresh clone, named by DIRECTORY-005's reviewed manifest for surface/identity/. None of the project's seven gate legs runs a frontend test today, so this brief's surface tests are exercised by no gate until that command exists., Reconcile R6's modify paths against DIRECTORY-005's and DIRECTORY-006's landed manifests before dispatch. DIRECTORY-005 declares its screens only as surface/identity/, so its directory preview drawer and identity record screen are named at the paths the identity surface was built at on the branch hand/identity-surface at c94ab69 (surface/identity/src/features/people/Preview.tsx and surface/identity/src/features/file/IdentityFile.tsx); the You page and the grant explanation are named at DIRECTORY-006 R6's declared paths. Any path that differs on landing is written into R6's files and the design's structure array by a revision of this brief before dispatch (CN9)., Reconcile against DIRECTORY-006 as landed before dispatch that the identity server serves the three Access routes this brief calls by the names GET /grants, POST /grants/why and POST /grants/who; that GET /grants still returns a grant after it is revoked and after its time window ends, with that grant's resource and action; and that the POST /grants/why refusal for that grant's resource and action names that grant. Any route name that differs on landing is written into this brief by a revision before dispatch. If either of the other two does not hold, R8's GRAPH_REVOKED and GRAPH_EXPIRED lines, and the R2 and R4 lines that depend on the same behaviour, are revised through a revision of this brief before dispatch, and are not left unsatisfiable., Reconcile R7's walk and R6's registry test against DIRECTORY-005's reviewed manifest before dispatch: the router package the landed screens import useParams from (react-router on the branch hand/identity-surface at c94ab69); the shell modules GRAPH_BOUNDARY lists by file name (surface/identity/src/shell/keyable.ts at that commit); the stylesheets under surface/identity/src/styles/; the surface's own API client the walk refuses by name (surface/identity/src/api.ts at that commit); and the shell's key registry and rail list R6's test reads (GO in surface/identity/src/shell/keys.ts and RAIL in surface/identity/src/shell/railItems.ts at that commit). Any package, path or name that differs on landing is written into R6 and R7 by a revision of this brief before dispatch; a module that calls Access other than through the generated module is never added to the allowed list, and a shell module is added to it only by file name., Sign-off of this brief through the card chain before it is dispatched; the card is created blocked on DIRECTORY-005 and DIRECTORY-006 so it cannot dispatch early (CN12).
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-098 — The access graph draws Access's answers and the directory's records, and holds no permission rule — Grant edges and every reachability the graph draws come only from Access answers returned by the directory, paths exactly as returned and a no as Access's reason. Choosing a person or agent reads the grants it holds that the caller may see and asks the forward question for each resource and action they carry; choosing a resource asks the reverse question for each action the resource declares. A grant is drawn as the dashed does not stand edge only when a forward refusal names it, with Access's reason; a grant Access no longer returns disappears on the next draw; the graph never infers that a grant does not stand. A resource's parent and an agent's responsible person are recorded facts drawn as recorded: parents from the permission model through a read-only route that records nothing, serves each resource's declared actions and the model version it read them under, and shows a resource only where the caller holds a grant on it or an ancestor, or is a directory administrator, who sees every resource; a hidden parent is served as withheld, drawn as a mark with no containment edge and never named; responsible people come from the directory's records. The graph holds no rule of its own and asks Access only on navigation, reload or its refresh control. Rejected: porting the mock-up's raw-grant edges, its standing reckoning and its reach(); amending Access to enumerate reach or list grants that do not stand; a new directory record for resources; serving a resource with a hidden parent as a root; and dropping the containment and answers-to edges because they are not Access answers.
> **Checklist:**
> - C272 — The graph module under surface/identity/src/features/graph/ imports nothing from the permission engine except the generated Access client (its other imports are only React, the surface's router package, the shell modules the test lists by file name, the surface's stylesheets and its own files, followed transitively, the surface's API client refused by name wherever it is reached), declares none of the mock-up's rule functions and references no mutation, and a test that counts what it checked proves it (conformance 8.3).
> - C273 — Every grant edge and every reachability the graph draws equals an Access answer from the real evaluator for the same question, over every identity, resource and declared action of a fixture directory: paths as returned, a no as Access's reason, a does not stand edge only for a grant a forward refusal names, with Access's reason, a grant Access no longer returns gone on the next draw, standing never inferred, and an outage shown as a named refusal.
> - C274 — A read-only route in the identity server serves the permission model's resources with their parents, declared actions and model version, recording nothing; a caller sees a resource only where they hold a grant on it or on an ancestor, a directory administrator sees every resource, pass-on authority widens nothing, and a hidden parent is served as withheld without its name.
> - C275 — Before any question the graph shows the people, agents and resources the person may see, with containment edges from the served parents, a parent withheld mark where the parent is withheld, answers-to edges from the records and no grant edge, under three toggles: Grants, Containment and Who answers for whom, none of which asks Access again.
> - C276 — Choosing a person or agent draws the forward answer for each resource and action its visible grants carry, choosing a resource draws the reverse answer, an administrator holding the reverse question's visibility permission sees the whole directory, a reverse question the person may not ask is refused by name, no hidden grant is drawn or hinted at, and an incomplete reverse answer is marked incomplete.
> - C277 — Every grant and does not stand edge shows the model version of the Access answer it came from and every containment edge that of the resource route's answer, a draw of mixed versions names each above the graph, the draw time shows beside the version, and the graph asks Access again only on navigation, reload or its refresh control.
> - C278 — The graph is reached by the deep links #/graph and #/graph/<id> and by a Show in graph link on each screen present at build time that shows one identity (the directory's preview drawer, the identity record screen and the You page) and on the grant explanation, whose link opens the graph on the holder's node with that grant's edge selected, none on sign-in or the delegation form, with no rail entry or g h shortcut of its own.
> **Stories:**
> - S121 (Graph viewer, Looks at who can reach what on the access graph) — As a person signed in to the directory, I want to see what I may see as a graph, with my own reach drawn from Access's answers and the recorded containment and responsible people, so that what the graph shows is what the directory would decide.
> - S122 (Graph viewer, Looks at who can reach what on the access graph) — As a directory administrator holding the visibility permission, I want to ask the graph who can reach a resource and see Access's answer with its model version and the time it was drawn, so that I can review reach without the screen inventing any of it.
> - S123 (Graph viewer, Looks at who can reach what on the access graph) — As a reviewer of conformance row 8.3, I want tests that compare every drawn edge with the real evaluator and prove the graph module holds no rule, so that the graph cannot drift from Access unnoticed.

## Purpose

The access graph screen in surface/identity/, the row that passes conformance row 8.3: the graph draws the same answers as Access and never computes its own. Access is DIRECTORY-006 R5's grant and explanation seam: the grant list (GET /grants), the forward question (POST /grants/why), which answers yes with the path to a person or no with the named reason, each with the model version used (row 8.1), and the reverse question (POST /grants/who), who can reach a resource (row 8.2). The screen follows the graph of the accepted mock-up docs/design/identity/mockup/index.v5.html (sha256 e67622e19f450e2335029f293f18b4757801f3245758a8b667521afa120ab91f, lines 1309 to 1354) except where ADR-098 replaces the mock-up's own reckoning: grant edges and every reachability come only from Access answers, a resource's parent and an agent's responsible person are recorded facts drawn as recorded, and the graph holds no rule of its own. DIRECTORY-006 R6 hands the graph's rendering to this brief; Access's contract is not amended here.

## Task

Add one read-only route to the identity server that serves the permission model's resources with their parents (R1), build the graph module under surface/identity/src/features/graph/ (R2 to R5), reach it by its deep links and the Show in graph links (R6), and prove it with the import-boundary test (R7) and the fixture comparison against the real DIRECTORY-006 evaluator (R8). The requirements are in dependency order: every file a requirement's acceptance uses is created by that requirement or an earlier one. Every path is relative to the repository root (CN3). Nothing in surface/ or crates/lys-identity-server exists at the commit this brief was written against; every file below is created or modified on top of what DIRECTORY-005 and DIRECTORY-006 land, and each modify path is reconciled against their landed manifests before dispatch (CN9).

What the graph draws. It draws what Access says, and nothing more. Grant edges and every reachability come only from Access answers returned by the directory, and the graph holds no rule of its own. It draws a path exactly as Access returns it, and a no as Access's reason. When a POST /grants/why refusal names a blocking grant, the graph draws that grant as the mock-up's dashed does not stand edge with the reason Access gives; a held grant that is revoked or expired is drawn dashed only because the forward question on it is refused and that refusal names it. When Access no longer returns a grant at all, it disappears from the next draw. The graph never infers on its own that a grant does not stand, and never judges standing from a grant's fields. A resource's parent and an agent's responsible person are recorded facts, not rules, and the graph draws them as recorded: an agent's responsible person from the directory's records, and a resource's parent from the permission model (DIRECTORY-006 R1 GRANT_MODEL, R2 GRANT_ANCESTRY), read through R1's route, which records nothing and adds no directory resource record. R1's route shows a caller a resource only where they hold a grant on it or on one of its ancestors, and a directory administrator under DIRECTORY-006 R6 sees every resource whether or not they hold a grant; pass-on authority confers no further visibility. When a resource's parent is hidden from the caller, the route serves the resource with parent {withheld: true}, and the graph draws a small parent withheld mark and no containment edge, never a root and never the parent's name. The route also serves each resource's declared actions and the model version it read them under. Nothing about containment is computed in the page.

What the person sees. A person opens the graph over what they may see, their own reach (DIRECTORY-006 R6). Before a question the graph shows the people, agents and resources the person may see, with containment and answers-to edges from the records, and no grant edges. Choosing a person or agent reads GET /grants for the grants that identity holds and the caller may see and, for each resource and action those grants carry, asks POST /grants/why, which records nothing, and draws each answer as given; no reach operation is added, and the answer is complete for what the caller may see, paged by the grant list. Choosing a resource asks POST /grants/who, as row 8.2 does, once for each action R1's route serves for it. The reverse question needs its own visibility permission (DIRECTORY-006 R5); a directory administrator with that permission sees the whole directory. A hidden grant is never drawn or hinted at, and a question the person may not ask is refused by name. The graph has three toggles, as the mock-up has: Grants shows or hides the Access answers already drawn without asking again, Containment, and Who answers for whom.

Versions and freshness. Every grant and does not stand edge carries the model version of the Access answer it came from, and each containment edge the model version of R1's answer it came from, shown as the answers' versions are. When a draw holds more than one version, a line above the graph says so and names each; the graph neither refuses a mixed picture nor re-asks. The graph asks Access on navigating to the graph, reloading it, or pressing a visible refresh control, and does not re-ask on its own. It shows the time of its draw beside the version.

Reaching the graph. This brief adds the deep links #/graph, #/graph/<id> and, for a grant explanation's link, #/graph/<holder id>?grant=<grant id> and a Show in graph link on every screen present at build time that shows one identity: the directory's preview drawer, where the mock-up carries it (index.v5.html:1206), and the identity record screen, each for people and agents alike, and the You page, for the signed-in person; and on the grant explanation, whose link opens the graph on the holder identity's node with that one grant's edge selected, so the person sees the same grant the explanation gave reasons for. Sign-in shows nobody and carries no link, and the delegation form is in the middle of an act that has no grant yet and carries none; when it finishes, the resulting grant's explanation carries the link. The rail entry and the g h shortcut belong to the shell card AL4OyeKG, which registers every screen's route and key; until the shell lands, the graph is reached by its deep links and the Show in graph links.

The Access client is DIRECTORY-006's generated module surface/identity/src/generated/index.ts, regenerated from the server schema for R1's route and otherwise used as it is. The graph fixture directory, stated once here and written out twice, in crates/lys-identity-server/tests/grant_resources.rs for the server test and in surface/identity/tests/fixtures/graph-directory.ts for the surface tests: a permission model with two relations whose names do not imply their action sets, relation-k resolving to the actions view and edit and relation-m resolving to the action view, and three resources declaring actions: resource-org (no parent) declares view, edit and share, resource-a (parent resource-org) declares view, and resource-b (parent resource-org) declares view and edit, 6 actions in all; people person-owner (holds P-owner, relation-k with actions view and edit, on resource-org), person-plain (holds P-plain, relation-m with action view, on resource-b) and person-auditor (a directory administrator holding the reverse question's visibility permission and no grant); agents agent-owner (responsible person person-owner) and agent-plain (responsible person person-plain); grants G1 (agent-owner, relation-m with action view, on resource-a, from P-owner), G2 (agent-plain, relation-m with action view, on resource-b, from P-plain) and G3 (agent-owner, relation-k with action edit, on resource-b, from P-owner, with a time window ending at instant E on the server's named clock). Each written copy states every one of these values as a literal, and a test compares the route's and Access's answers against those literals, never against its own reading of the model. From the fixture: the pairs of one identity, one resource and one action it declares number 5 times 6, that is 30; the distinct (resource, action) that agent-owner's grants carry are (resource-a, view) and (resource-b, edit), 2 in all; resource-a declares 1 action and resource-b 2. Only person-auditor holds the reverse question's visibility permission. Both copies are seeded into the standalone identity server with disposable local dependencies through DIRECTORY-003's and DIRECTORY-006's own typed operations, the Rust copy by calling them in the Rust tree and the surface copy through the server's routes, so every Access answer a test compares against comes from the real evaluator and never from a stub the test wrote. Enduring identity ids and grant ids are assigned by the server, no operation accepts a caller-chosen id, and none is added. Each copy's seeding step therefore records, for every labelled entity it creates (each person, agent, resource and grant above), the id the server assigned, in one JSON object from label to id that it writes, on every run, to a file beside its fixture: crates/lys-identity-server/tests/grant_resources.ids.json for the Rust copy and surface/identity/tests/fixtures/graph-directory.ids.json for the surface copy. Each map is an uncommitted run output, never a tree file: the test that writes it removes it before it exits, and .gitignore carries one line for each map, the map's path exactly as written here, so a run leaves nothing behind even if the ignore is missed. Every acceptance line that names a fixture label as an id, as a string searched for or as part of an href means the id read from that map, and states the map file and the key it reads; id(X) in a line means the value that line's map holds at key X. A line that counts occurrences of a labelled entity counts both its assigned id and its label text, and asserts 0 of each. An href line means the string #/graph/ joined to the assigned id of the entity it names, with ?grant= joined to the assigned grant id where a grant is named. surface/identity/tests/graph.test.tsx seeds nothing: the grant and resource ids in it are the literal strings it writes into the answers it constructs (grant-u1, resource-u1, resource-u2), not fixture ids. Test identifiers: GRAPH_RESOURCES, GRAPH_BOUNDARY, GRAPH_EVERY_PAIR, GRAPH_REVOKED, GRAPH_EXPIRED, GRAPH_VISIBILITY, GRAPH_VERSION, GRAPH_OUTAGE, each named in the test that carries it.

Out of scope: the rail entry and g h shortcut (shell card AL4OyeKG); a suspended holder drawn from Access's answer, which waits for the lifecycle card once suspension is decided and folded; Show in graph links on screens that land later; recording row 8.3's brief in CONFORMANCE.md; any change to Access's contract, lys-core or a published wire format; the suspension semantics and the grant representation, which stay open.

## Requirements

### R1: Serve the permission model's resources with their parents, read-only

WHEN an authenticated caller asks GET /grants/resources, THE SYSTEM SHALL answer, in crates/lys-identity-server/src/grant_resources.rs registered in crates/lys-identity-server/src/routes.rs, with each resource of the permission model (DIRECTORY-006 R1 GRANT_MODEL, R2 GRANT_ANCESTRY) the caller may see, its parent, the actions the model declares for it, and the model version the answer was read under, calling the same authority owner as DIRECTORY-006 R5's operations. A caller sees a resource only where they hold a grant on it or on one of its ancestors, or where they are a directory administrator under DIRECTORY-006 R6's administrator view, who sees every resource whether or not they hold a grant. IF a resource's parent is a resource the caller may not see, THEN THE SYSTEM SHALL answer that resource with parent {withheld: true}. IF the caller is unauthenticated, or the permission engine is unavailable, THEN THE SYSTEM SHALL answer with a named refusal. THE SYSTEM SHALL NOT record, append or mutate anything on this route; SHALL NOT name, count or hint at a resource the caller may not see, a withheld parent included; SHALL NOT answer a resource whose parent is withheld as having no parent; SHALL NOT widen what a caller sees because they hold pass-on authority on a grant; SHALL NOT answer an outage as an empty list (ADR-004); and SHALL NOT add a resource record to the directory. The test GRAPH_RESOURCES writes its label-to-id map to crates/lys-identity-server/tests/grant_resources.ids.json, an uncommitted run output that .gitignore ignores by the line crates/lys-identity-server/tests/grant_resources.ids.json, and removes that file before it exits; it SHALL NOT leave the map, or any other file, in the working tree.

**Acceptance:**
- crates/lys-identity-server/tests/grant_resources.rs (GRAPH_RESOURCES) states the graph fixture directory of the Task in Rust, seeds it through DIRECTORY-003's and DIRECTORY-006's typed operations before its first request, and writes its label-to-id map to crates/lys-identity-server/tests/grant_resources.ids.json; every id(X) in GRAPH_RESOURCES's lines is read from that file at key X. As person-owner, the answer holds exactly the ids id(resource-org), id(resource-a) and id(resource-b); id(resource-org) has no parent, id(resource-a) and id(resource-b) each have parent id(resource-org), and the actions served equal, as sets, the fixture's literals: id(resource-org) {view, edit, share}, id(resource-a) {view} and id(resource-b) {view, edit}, 6 in all.
- GRAPH_RESOURCES, as person-auditor, who holds no grant: the answer holds exactly id(resource-org), id(resource-a) and id(resource-b), read from crates/lys-identity-server/tests/grant_resources.ids.json at keys resource-org, resource-a and resource-b, with the same parents as person-owner's answer.
- GRAPH_RESOURCES, as person-plain: the answer holds exactly one resource, id(resource-b), whose parent is {withheld: true}; the serialised response body contains 0 occurrences of id(resource-org) and 0 of id(resource-a), read from crates/lys-identity-server/tests/grant_resources.ids.json at keys resource-org and resource-a, and 0 occurrences of the label texts resource-org and resource-a.
- GRAPH_RESOURCES, as person-owner: the answer's model version equals the model version of the POST /grants/why answer for id(agent-owner) on id(resource-a) with the action view, asked in the same test, both ids read from crates/lys-identity-server/tests/grant_resources.ids.json at keys agent-owner and resource-a.
- GRAPH_RESOURCES: an unauthenticated request is answered with the named unauthenticated refusal; with the permission engine stopped, a request as person-owner is answered with the named outage refusal and no resource list.
- GRAPH_RESOURCES: the authoritative event count and the permission projection revision are equal before and after one request from each of person-owner, person-auditor and person-plain; the test asserts it made 3 such requests.
- From the repository root, on a checkout where git status --porcelain prints nothing: after the test run that carries GRAPH_RESOURCES exits, git status --porcelain prints nothing and test -e crates/lys-identity-server/tests/grant_resources.ids.json exits 1; and git check-ignore crates/lys-identity-server/tests/grant_resources.ids.json prints crates/lys-identity-server/tests/grant_resources.ids.json.

**Files:**
- create: crates/lys-identity-server/src/grant_resources.rs
- create: crates/lys-identity-server/tests/grant_resources.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: surface/identity/src/generated/index.ts
- modify: .gitignore

**Checklist:**
- C274 — A read-only route in the identity server serves the permission model's resources with their parents, declared actions and model version, recording nothing; a caller sees a resource only where they hold a grant on it or on an ancestor, a directory administrator sees every resource, pass-on authority widens nothing, and a hidden parent is served as withheld without its name.

**Stories:**
- S121 (Graph viewer, Looks at who can reach what on the access graph) — As a person signed in to the directory, I want to see what I may see as a graph, with my own reach drawn from Access's answers and the recorded containment and responsible people, so that what the graph shows is what the directory would decide.

### R2: Build the graph's grant edges from Access answers only

WHEN an Access answer arrives for a question the graph asked, THE SYSTEM SHALL build its edges in surface/identity/src/features/graph/graphModel.ts from that answer alone, rendered by surface/identity/src/features/graph/GraphScreen.tsx with its side panel and refusal element: for a POST /grants/why answer that permits, one grant edge per step of the returned path, in the returned order, between the nodes the step names, labelled with the step's relation as returned; for a POST /grants/why refusal that names a blocking grant, one dashed does not stand edge for that grant carrying the reason text Access gives; for a refusal that names no grant, no edge, and Access's reason text in the side panel; for a POST /grants/who answer, each returned holder with its returned path, drawn the same way. IF the Access client call fails, THEN THE SYSTEM SHALL show the refusal the client reports, by its name, and draw no grant edge for that question. THE SYSTEM SHALL NOT draw a grant edge or a reachability highlight without an Access answer behind it; SHALL NOT draw a does not stand edge except for a grant a refusal names; SHALL NOT judge standing, ancestry, expiry or reachability from a grant's fields or any record (ADR-011); SHALL NOT shorten, re-root, merge or reorder a returned path, whose last step reaches a person (ADR-003); SHALL NOT write reason wording of its own; SHALL NOT keep an edge from an earlier draw that the current draw's answers do not return; and SHALL NOT draw an Access outage as an empty graph (ADR-004).

**Acceptance:**
- surface/identity/tests/graph.test.tsx: given one permitting POST /grants/why answer, of DIRECTORY-006's generated type, whose path has three steps, the model built from it holds exactly three grant edges, and their (from, to, label) triples equal the three steps' in the returned order.
- surface/identity/tests/graph.test.tsx: given one POST /grants/why refusal that names the grant grant-u1 with reason text R, the model holds exactly 1 does not stand edge, it carries the id grant-u1, its reason text equals R character for character, and the model holds 0 grant edges.
- surface/identity/tests/graph.test.tsx: given one POST /grants/why refusal that names no grant, with reason text R, the model holds 0 grant edges and 0 does not stand edges, and the rendered side panel's text contains R character for character.
- surface/identity/tests/graph.test.tsx: given a grant list returning grant-u1, whose fields carry a revoked state and an end time before the test clock, and a permitting POST /grants/why answer for grant-u1's resource and action, the model holds a grant edge carrying the id grant-u1 and 0 does not stand edges.
- surface/identity/tests/graph.test.tsx: given a first draw from a grant list returning grant-u1 with a permitting answer, and a second draw from a grant list that does not return grant-u1 and answers none of which names grant-u1, the second draw holds 0 edges carrying the id grant-u1.
- surface/identity/tests/graph.test.tsx: when the Access client call rejects with a transport failure, the screen renders the refusal element carrying the name the client reports, and the draw holds 0 grant edges and 0 does not stand edges.

**Files:**
- create: surface/identity/src/features/graph/graphModel.ts
- create: surface/identity/src/features/graph/GraphScreen.tsx
- create: surface/identity/tests/graph.test.tsx

**Checklist:**
- C273 — Every grant edge and every reachability the graph draws equals an Access answer from the real evaluator for the same question, over every identity, resource and declared action of a fixture directory: paths as returned, a no as Access's reason, a does not stand edge only for a grant a forward refusal names, with Access's reason, a grant Access no longer returns gone on the next draw, standing never inferred, and an outage shown as a named refusal.

**Stories:**
- S121 (Graph viewer, Looks at who can reach what on the access graph) — As a person signed in to the directory, I want to see what I may see as a graph, with my own reach drawn from Access's answers and the recorded containment and responsible people, so that what the graph shows is what the directory would decide.
- S123 (Graph viewer, Looks at who can reach what on the access graph) — As a reviewer of conformance row 8.3, I want tests that compare every drawn edge with the real evaluator and prove the graph module holds no rule, so that the graph cannot drift from Access unnoticed.

### R3: Draw the view before a question and the recorded relations

WHEN the graph opens with no node chosen, THE SYSTEM SHALL show, in surface/identity/src/features/graph/recordEdges.ts through the generated module surface/identity/src/generated/index.ts, rendered with the three toggles and the parent withheld mark by surface/identity/src/features/graph/GraphScreen.tsx, the people and agents the directory's listing rules return to the signed-in person and the resources R1's route returns to them, with no grant edge. WHILE the Containment toggle is on, THE SYSTEM SHALL draw one containment edge from each shown resource to the parent R1's route returns for it, and, for a resource whose parent R1's route returns as {withheld: true}, a parent withheld mark on that resource and no containment edge. WHILE the Who answers for whom toggle is on, THE SYSTEM SHALL draw one answers-to edge from each shown agent to its recorded responsible person. WHILE the Grants toggle is off, THE SYSTEM SHALL hide the grant and does not stand edges already drawn, and WHEN it is turned on again, THE SYSTEM SHALL show them again, in both cases without asking Access. Each toggle is a native button reachable by the Tab key and pressed by the Enter key. THE SYSTEM SHALL NOT draw a grant edge before a node is chosen; SHALL NOT compute a resource's parent, an ancestor or a responsible person, or draw one R1's route or the directory's records did not return; SHALL NOT show a resource R1's route did not return; SHALL NOT draw a resource whose parent is withheld as a root, or name its withheld parent; and SHALL NOT ask Access when a toggle changes. The seeding in surface/identity/tests/acceptance/graph.spec.ts writes its label-to-id map to surface/identity/tests/fixtures/graph-directory.ids.json, an uncommitted run output that .gitignore ignores by the line surface/identity/tests/fixtures/graph-directory.ids.json, and removes that file before the test file exits; it SHALL NOT leave the map, or any other file, in the working tree.

**Acceptance:**
- surface/identity/tests/acceptance/graph.spec.ts (GRAPH_VISIBILITY), with the graph fixture directory of surface/identity/tests/fixtures/graph-directory.ts seeded and its label-to-id map written to surface/identity/tests/fixtures/graph-directory.ids.json, every id(X) below read from that file at key X, signed in as person-auditor with no node chosen: the draw's node ids equal, as a set, the 8 ids id(person-owner), id(person-plain), id(person-auditor), id(agent-owner), id(agent-plain), id(resource-org), id(resource-a) and id(resource-b); it holds 2 containment edges (id(resource-a) to id(resource-org), id(resource-b) to id(resource-org)), 2 answers-to edges (id(agent-owner) to id(person-owner), id(agent-plain) to id(person-plain)), 0 grant edges and 0 does not stand edges.
- surface/identity/tests/acceptance/graph.spec.ts (GRAPH_VISIBILITY), signed in as person-plain with no node chosen: the identity node ids drawn equal, as a set, the ids of the identity records the directory lists to person-plain through the generated module; the resource node ids drawn are exactly id(resource-b), read from surface/identity/tests/fixtures/graph-directory.ids.json at key resource-b; that node carries the parent withheld mark; the draw holds 0 containment edges and 0 grant edges; and the serialised rendered document contains 0 occurrences of each of id(resource-org), id(resource-a), id(G1) and id(G3), read from surface/identity/tests/fixtures/graph-directory.ids.json at keys resource-org, resource-a, G1 and G3, and 0 occurrences of each of the label texts resource-org, resource-a, G1 and G3.
- surface/identity/tests/acceptance/graph.spec.ts, signed in as person-auditor with no node chosen: turning Containment off leaves 0 containment edges and turning it on restores 2; turning Who answers for whom off leaves 0 answers-to edges and turning it on restores 2; the client's request log holds 0 requests issued during the four toggle presses.
- surface/identity/tests/graph.test.tsx: after a draw holding 3 grant edges and 1 does not stand edge, pressing Grants leaves 0 grant and 0 does not stand edges drawn, pressing it again restores 3 and 1, and the client issues 0 requests across both presses; focusing Grants with the Tab key and pressing Enter has the same effect as clicking it.
- surface/identity/tests/graph.test.tsx: given R1's answer holding resource-u1 with no parent and resource-u2 with parent {withheld: true}, the draw holds 0 containment edges, resource-u2 carries exactly 1 parent withheld mark, and resource-u1 carries 0 parent withheld marks.
- From the repository root, on a checkout where git status --porcelain prints nothing: after the frontend test command DIRECTORY-005's manifest names has run surface/identity/tests/acceptance/graph.spec.ts and exited, git status --porcelain prints nothing and test -e surface/identity/tests/fixtures/graph-directory.ids.json exits 1; and git check-ignore surface/identity/tests/fixtures/graph-directory.ids.json prints surface/identity/tests/fixtures/graph-directory.ids.json.

**Files:**
- create: surface/identity/src/features/graph/recordEdges.ts
- create: surface/identity/tests/fixtures/graph-directory.ts
- create: surface/identity/tests/acceptance/graph.spec.ts
- modify: surface/identity/src/features/graph/GraphScreen.tsx
- modify: .gitignore

**Checklist:**
- C275 — Before any question the graph shows the people, agents and resources the person may see, with containment edges from the served parents, a parent withheld mark where the parent is withheld, answers-to edges from the records and no grant edge, under three toggles: Grants, Containment and Who answers for whom, none of which asks Access again.

**Stories:**
- S121 (Graph viewer, Looks at who can reach what on the access graph) — As a person signed in to the directory, I want to see what I may see as a graph, with my own reach drawn from Access's answers and the recorded containment and responsible people, so that what the graph shows is what the directory would decide.

### R4: Ask Access about the chosen node under the person's permissions

WHEN the person chooses a person or agent node, THE SYSTEM SHALL, in surface/identity/src/features/graph/graphQuestions.ts through the generated module, read GET /grants for the grants that identity holds and the caller may see, following every page of the list, and for each resource and action those grants carry ask POST /grants/why once for that identity, that resource and that action, and draw each answer as R2 says. WHEN the person chooses a resource node, THE SYSTEM SHALL ask POST /grants/who once for each action R1's route returns for that resource, and draw exactly the holders and paths returned. IF Access refuses a question the person may not ask, THEN THE SYSTEM SHALL show that refusal in the side panel of surface/identity/src/features/graph/GraphScreen.tsx by the name and reason Access returns and draw no holder for it. IF a POST /grants/who answer names a continuation, THEN THE SYSTEM SHALL mark that answer incomplete in the side panel of surface/identity/src/features/graph/GraphScreen.tsx and draw only the holders returned. Each node is reachable by the Tab key and chosen by the Enter key as by a click. THE SYSTEM SHALL NOT compute a reach of its own, walk ancestors or ask a question for a resource and action no listed grant carries; SHALL NOT add, drop or hint at a holder or grant Access did not return (no id, label, count or placeholder for a hidden grant); SHALL NOT present an incomplete answer as the whole set; and SHALL NOT answer a refused question by asking other questions in its place.

**Acceptance:**
- surface/identity/tests/acceptance/graph.spec.ts, signed in as person-auditor, choosing agent-owner, every id(X) read from surface/identity/tests/fixtures/graph-directory.ids.json at key X: the client issues GET /grants for id(agent-owner), then exactly 2 POST /grants/why, one for (id(resource-a), view) and one for (id(resource-b), edit), the fixture's literals for what G1 and G3 carry, and no other POST /grants/why; and the draw holds an edge carrying id(G1) whose path equals the answer returned for (id(resource-a), view).
- surface/identity/tests/acceptance/graph.spec.ts (GRAPH_VISIBILITY), signed in as person-auditor, choosing resource-a, every id(X) read from surface/identity/tests/fixtures/graph-directory.ids.json at key X: the client issues exactly 1 POST /grants/who, for id(resource-a) and the action view, the one action the fixture declares for resource-a, and the holders drawn equal, as a set of ids, the holders that answer returns, which includes id(agent-owner).
- surface/identity/tests/acceptance/graph.spec.ts (GRAPH_VISIBILITY), signed in as person-plain, choosing resource-b: the client issues exactly 2 POST /grants/who requests for id(resource-b), read from surface/identity/tests/fixtures/graph-directory.ids.json at key resource-b, one for view and one for edit, the actions the fixture declares for resource-b; each of those requests is refused; the side panel shows each refusal's name and reason text character for character as returned; the draw holds 0 holder highlights for id(resource-b); and the client's request log holds 0 requests of any kind issued after the last of those POST /grants/who requests.
- surface/identity/tests/acceptance/graph.spec.ts (GRAPH_VISIBILITY), signed in as person-plain, choosing agent-plain, every id(X) read from surface/identity/tests/fixtures/graph-directory.ids.json at key X: the draw holds an edge carrying id(G2), and the serialised rendered document contains 0 occurrences of each of id(G1), id(G3), id(resource-a) and id(resource-org), and 0 occurrences of each of the label texts G1, G3, resource-a and resource-org.
- surface/identity/tests/graph.test.tsx: given a POST /grants/who answer of two holders that names a continuation, the panel carries the element with data-incomplete="true" and lists exactly 2 holders; given the same answer with no continuation, no element with data-incomplete="true" is rendered.
- surface/identity/tests/graph.test.tsx: given a grant list of two pages, the client issues 2 GET /grants requests and the POST /grants/why requests cover the grants of both pages.
- surface/identity/tests/graph.test.tsx: moving focus to a resource node with the Tab key and pressing Enter issues the same requests as clicking that node.

**Files:**
- create: surface/identity/src/features/graph/graphQuestions.ts
- modify: surface/identity/src/features/graph/GraphScreen.tsx

**Checklist:**
- C276 — Choosing a person or agent draws the forward answer for each resource and action its visible grants carry, choosing a resource draws the reverse answer, an administrator holding the reverse question's visibility permission sees the whole directory, a reverse question the person may not ask is refused by name, no hidden grant is drawn or hinted at, and an incomplete reverse answer is marked incomplete.

**Stories:**
- S121 (Graph viewer, Looks at who can reach what on the access graph) — As a person signed in to the directory, I want to see what I may see as a graph, with my own reach drawn from Access's answers and the recorded containment and responsible people, so that what the graph shows is what the directory would decide.
- S122 (Graph viewer, Looks at who can reach what on the access graph) — As a directory administrator holding the visibility permission, I want to ask the graph who can reach a resource and see Access's answer with its model version and the time it was drawn, so that I can review reach without the screen inventing any of it.

### R5: Name each answer's model version and the draw time, and redraw only when asked

THE SYSTEM SHALL show, in surface/identity/src/features/graph/GraphScreen.tsx, on each grant and does not stand edge the model version of the Access answer it came from, as that answer carries it, and on each containment edge the model version of R1's answer it came from, in the same way. WHILE a draw holds answers, R1's answer among them, of more than one model version, THE SYSTEM SHALL show a line above the graph that says so and names each version. THE SYSTEM SHALL show the time of the draw beside the version. WHEN the person navigates to the graph, reloads it, or presses the visible refresh control, THE SYSTEM SHALL draw again by asking Access afresh. The refresh control is a native button reachable by the Tab key and pressed by the Enter key. The layout in surface/identity/src/features/graph/graphLayout.ts is deterministic: the same nodes and edges place every node at the same coordinates on every draw. THE SYSTEM SHALL NOT re-ask Access on its own (no timer, poll or push); SHALL NOT refuse to draw a picture of mixed versions; SHALL NOT re-ask because the versions differ; and SHALL NOT show a model version that neither an Access answer nor R1's answer in the draw returned.

**Acceptance:**
- surface/identity/tests/graph.test.tsx: given a draw built from two answers, one carrying model version v7 and one carrying v8, every edge's shown model version equals the version of the answer it came from, and the line above the graph contains both v7 and v8.
- surface/identity/tests/graph.test.tsx: given a draw built only from answers carrying v7, no mixed-version line is rendered and the version shown beside the draw time is v7.
- surface/identity/tests/graph.test.tsx: given R1's answer carrying model version v7 and one POST /grants/why answer carrying v8, each containment edge shows v7, each grant edge shows v8, and the line above the graph contains both v7 and v8.
- surface/identity/tests/graph.test.tsx: with the test clock fixed at a named instant T and the zone UTC, the draw time element's datetime attribute equals T in ISO 8601 form and its text equals T formatted as YYYY-MM-DD HH:MM:SS UTC.
- surface/identity/tests/graph.test.tsx: after a draw that issued N Access requests, the fake timer clock holds 0 pending timeouts and 0 pending intervals, and the graph has opened 0 EventSource and 0 WebSocket connections; pressing the refresh control then issues N requests; focusing the refresh control with the Tab key and pressing Enter issues N more.
- surface/identity/tests/acceptance/graph.spec.ts, signed in as person-auditor with agent-owner chosen: reloading the page issues the same number of Access requests as the first draw.
- surface/identity/tests/graph.test.tsx: laying out the same model twice gives equal x and y coordinates for every node.

**Files:**
- create: surface/identity/src/features/graph/graphLayout.ts
- modify: surface/identity/src/features/graph/GraphScreen.tsx

**Checklist:**
- C277 — Every grant and does not stand edge shows the model version of the Access answer it came from and every containment edge that of the resource route's answer, a draw of mixed versions names each above the graph, the draw time shows beside the version, and the graph asks Access again only on navigation, reload or its refresh control.

**Stories:**
- S121 (Graph viewer, Looks at who can reach what on the access graph) — As a person signed in to the directory, I want to see what I may see as a graph, with my own reach drawn from Access's answers and the recorded containment and responsible people, so that what the graph shows is what the directory would decide.
- S122 (Graph viewer, Looks at who can reach what on the access graph) — As a directory administrator holding the visibility permission, I want to ask the graph who can reach a resource and see Access's answer with its model version and the time it was drawn, so that I can review reach without the screen inventing any of it.

### R6: Reach the graph by its deep links and the Show in graph links

WHEN the location is #/graph/<id> for an identity or resource id, THE SYSTEM SHALL open the graph with that node chosen and ask Access about it as R4 says; WHEN the location is #/graph/<holder id>?grant=<grant id>, THE SYSTEM SHALL open the graph with that holder chosen, ask Access about it as R4 says, and mark selected, in surface/identity/src/features/graph/graphModel.ts, rendered by surface/identity/src/features/graph/GraphScreen.tsx as data-selected="true", the one edge drawn from that draw's answers that carries that grant's id; WHEN the location is #/graph, THE SYSTEM SHALL open the view before a question (R3). The route is registered in surface/identity/src/routes.tsx. Each screen present at build time that shows one identity SHALL carry exactly one Show in graph link, surface/identity/src/features/graph/ShowInGraphLink.tsx, to #/graph/<that identity's id>: the directory's preview drawer in surface/identity/src/features/people/Preview.tsx and the identity record screen in surface/identity/src/features/file/IdentityFile.tsx, each for a person and for an agent, and the You page in surface/identity/src/features/grants/YouGrants.tsx, for the signed-in person. The grant explanation in surface/identity/src/features/grants/GrantExplanation.tsx SHALL carry exactly one Show in graph link, to #/graph/<the explained grant's holder id>?grant=<the explained grant's id>. IF the grant a link names is not carried by any edge the draw's answers return, THEN THE SYSTEM SHALL mark no edge selected. THE SYSTEM SHALL NOT add a rail entry or the g h shortcut, which the shell card AL4OyeKG registers; SHALL NOT point a Show in graph link at any node other than the identity its screen shows, or, on the grant explanation, the explained grant's holder; SHALL NOT place a Show in graph link on sign-in or on the delegation form; SHALL NOT draw, select or hint at an edge for a grant the draw's answers do not return because a link names it; and SHALL NOT add any grant, revoke or delegate control to the graph.

**Acceptance:**
- surface/identity/tests/acceptance/graph.spec.ts, signed in as person-auditor, opening the location #/graph/ joined to id(agent-owner), read from surface/identity/tests/fixtures/graph-directory.ids.json at key agent-owner: the graph opens with id(agent-owner) chosen and the client issues the GET /grants and POST /grants/why requests R4 names for agent-owner.
- surface/identity/tests/acceptance/graph.spec.ts, signed in as person-auditor, opening #/graph: the draw holds 0 grant edges and the client issues 0 POST /grants/why and 0 POST /grants/who requests.
- Directory preview drawer, surface/identity/tests/acceptance/graph.spec.ts, signed in as person-auditor, every id(X) read from surface/identity/tests/fixtures/graph-directory.ids.json at key X: previewing agent-plain from the directory shows exactly one link whose text is Show in graph, and its href equals #/graph/ joined to id(agent-plain); previewing person-plain shows exactly one such link, whose href equals #/graph/ joined to id(person-plain).
- Identity record screen, surface/identity/tests/acceptance/graph.spec.ts, signed in as person-auditor, every id(X) read from surface/identity/tests/fixtures/graph-directory.ids.json at key X: the record screen for agent-plain contains exactly one link whose text is Show in graph, and its href equals #/graph/ joined to id(agent-plain); the record screen for person-plain contains exactly one such link, whose href equals #/graph/ joined to id(person-plain).
- You page, surface/identity/tests/acceptance/graph.spec.ts, signed in as person-plain: the You page contains exactly one link whose text is Show in graph, and its href equals #/graph/ joined to id(person-plain), read from surface/identity/tests/fixtures/graph-directory.ids.json at key person-plain.
- Grant explanation, surface/identity/tests/acceptance/graph.spec.ts, signed in as person-auditor, every id(X) read from surface/identity/tests/fixtures/graph-directory.ids.json at key X: the grant explanation for G1 contains exactly one link whose text is Show in graph, and its href equals #/graph/ joined to id(agent-owner), then ?grant= joined to id(G1); following it opens the graph with id(agent-owner) chosen, and the draw holds exactly 1 edge with data-selected="true", which carries id(G1).
- surface/identity/tests/acceptance/graph.spec.ts, signed in as person-plain, opening the location #/graph/ joined to id(agent-plain), then ?grant= joined to id(G1), both read from surface/identity/tests/fixtures/graph-directory.ids.json at keys agent-plain and G1: the draw holds 0 edges with data-selected="true", and the serialised rendered document contains 0 occurrences of id(G1) and 0 occurrences of the label text G1.
- surface/identity/tests/acceptance/graph.spec.ts: the sign-in screen, before anyone signs in, contains 0 links whose text is Show in graph, and the delegation form, signed in as person-owner, contains 0 links whose text is Show in graph.
- surface/identity/tests/graph.test.tsx: the entries of the shell's key registry and of its rail list (GO in surface/identity/src/shell/keys.ts and RAIL in surface/identity/src/shell/railItems.ts, as reconciled in blocked_by) are read before the graph screen is mounted and again after it has mounted and drawn; the two readings are equal entry for entry, so the graph module contributes 0 entries to each.

**Files:**
- create: surface/identity/src/features/graph/ShowInGraphLink.tsx
- modify: surface/identity/src/features/graph/graphModel.ts
- modify: surface/identity/src/features/graph/GraphScreen.tsx
- modify: surface/identity/src/routes.tsx
- modify: surface/identity/src/features/people/Preview.tsx
- modify: surface/identity/src/features/file/IdentityFile.tsx
- modify: surface/identity/src/features/grants/YouGrants.tsx
- modify: surface/identity/src/features/grants/GrantExplanation.tsx
- modify: surface/identity/tests/graph.test.tsx

**Checklist:**
- C278 — The graph is reached by the deep links #/graph and #/graph/<id> and by a Show in graph link on each screen present at build time that shows one identity (the directory's preview drawer, the identity record screen and the You page) and on the grant explanation, whose link opens the graph on the holder's node with that grant's edge selected, none on sign-in or the delegation form, with no rail entry or g h shortcut of its own.

**Stories:**
- S121 (Graph viewer, Looks at who can reach what on the access graph) — As a person signed in to the directory, I want to see what I may see as a graph, with my own reach drawn from Access's answers and the recorded containment and responsible people, so that what the graph shows is what the directory would decide.

### R7: Prove the graph module holds no permission logic of its own

The test surface/identity/tests/graph_boundary.test.ts (GRAPH_BOUNDARY) starts from every source file under surface/identity/src/features/graph/, reads each file's import specifiers (import and export-from statements, type-only imports and dynamic import calls included), and follows every allowed relative specifier transitively, into the allowed shell modules, the stylesheets and the Access client as well, reading each file it reaches in the same way; it does not follow a refused specifier. It allows only: the package react; the surface's router package react-router; a relative specifier resolving inside surface/identity/src/features/graph/; a relative specifier resolving to one of the shell modules the test lists by file name, which is surface/identity/src/shell/keyable.ts alone; a relative specifier resolving to a stylesheet under surface/identity/src/styles/; and a relative specifier resolving to surface/identity/src/generated/index.ts, the Access client (each as reconciled in blocked_by against DIRECTORY-005's reviewed manifest). It refuses every other specifier wherever in the walk it is reached; it refuses by name, as the surface's API client, any specifier resolving to surface/identity/src/api.ts, whichever module the walk reaches it through, and it refuses every specifier resolving into surface/identity/src/features/grants/, the permission screens, whatever it imports. Because the permission engine is Rust and cannot be imported by TypeScript, the same test also fails when any file in the graph module declares a function or constant named can, reach, whoCan, standing, ancestors, chainOf or grantsOf (the mock-up's own rule functions, index.v5.html:418-445), and when any file in the graph module references the generated module's delegate or revoke operation. The test counts each distinct specifier once over the whole walk, however many files carry it, and reports the paths of the files it checked and the distinct specifiers it allowed and refused, each refused one with the file it was reached through. It SHALL fail when it checked no file under surface/identity/src/features/graph/ or collected no specifier; it SHALL NOT pass by scanning nothing; SHALL NOT allow a shell module that is not listed by file name; and SHALL NOT allow a specifier outside the list above. Measured on the branch hand/identity-surface at c94ab69, each by the command named, run from the repository root: git show c94ab69:surface/identity/src/shell/keyable.ts | grep -E "from '" prints the one line import type { KeyboardEvent } from 'react';, so reaching keyable.ts adds the one file surface/identity/src/shell/keyable.ts to the walk and the one distinct specifier react; git show c94ab69:surface/identity/src/generated/index.ts | grep -cE "from '" prints 0, so reaching the Access client adds that one file and no specifier; git show c94ab69:surface/identity/src/shell/Palette.tsx | grep -n "from '../api'" prints 4:import { api } from '../api';, the reach through a shell module that the named refusal of the API client closes. The graph module is 6 files: python3 -c "import json; b=json.load(open('docs/design/directory/briefs/DIRECTORY-034.json')); print(len({p for r in b['requirements'] for p in r['files']['create'] if p.startswith('surface/identity/src/features/graph/')}))" prints 6.

**Acceptance:**
- GRAPH_BOUNDARY, on the module as built, passes and reports 0 refused specifiers and more than 0 distinct specifiers; the file paths it reports as checked include all 6 files R2 to R6 create under surface/identity/src/features/graph/, and every other path it reports is surface/identity/src/shell/keyable.ts, surface/identity/src/generated/index.ts or a stylesheet under surface/identity/src/styles/.
- GRAPH_BOUNDARY, on the module as built: the set of distinct specifiers its walk collected contains no specifier resolving to surface/identity/src/shell/keys.ts and none resolving to surface/identity/src/shell/railItems.ts, asserted by those two file names.
- Adding the line import { GrantExplanation } from '../grants/GrantExplanation'; to surface/identity/src/features/graph/graphModel.ts makes GRAPH_BOUNDARY fail with exactly 1 refused specifier, '../grants/GrantExplanation', while every test in surface/identity/tests/graph.test.tsx passes.
- Adding the line import { keyable } from '../../shell/keyable'; to surface/identity/src/features/graph/graphModel.ts and the line import { api } from '../api'; to surface/identity/src/shell/keyable.ts makes GRAPH_BOUNDARY fail with exactly 1 refused specifier, '../api', refused by the name of the surface's API client and reported as reached through surface/identity/src/shell/keyable.ts, while every test in surface/identity/tests/graph.test.tsx passes.
- Adding the lines import { useParams } from 'react-router'; and import { keyable } from '../../shell/keyable'; to surface/identity/src/features/graph/graphModel.ts leaves GRAPH_BOUNDARY passing with 0 refused specifiers and every test in surface/identity/tests/graph.test.tsx passing; compared with the run on the unmodified module, the set of checked file paths gains exactly the members of {surface/identity/src/shell/keyable.ts} it did not already hold, and the set of distinct allowed specifiers gains exactly the members of {react-router, ../../shell/keyable, react} it did not already hold.
- Adding the line function standing() { return true; } to surface/identity/src/features/graph/graphModel.ts makes GRAPH_BOUNDARY fail while every test in surface/identity/tests/graph.test.tsx passes.
- Adding a reference to the generated module's revoke operation to surface/identity/src/features/graph/GraphScreen.tsx makes GRAPH_BOUNDARY fail while every test in surface/identity/tests/graph.test.tsx passes.

**Files:**
- create: surface/identity/tests/graph_boundary.test.ts

**Checklist:**
- C272 — The graph module under surface/identity/src/features/graph/ imports nothing from the permission engine except the generated Access client (its other imports are only React, the surface's router package, the shell modules the test lists by file name, the surface's stylesheets and its own files, followed transitively, the surface's API client refused by name wherever it is reached), declares none of the mock-up's rule functions and references no mutation, and a test that counts what it checked proves it (conformance 8.3).

**Stories:**
- S123 (Graph viewer, Looks at who can reach what on the access graph) — As a reviewer of conformance row 8.3, I want tests that compare every drawn edge with the real evaluator and prove the graph module holds no rule, so that the graph cannot drift from Access unnoticed.

### R8: Compare every drawn edge with the real evaluator over a fixture directory

The fixture surface/identity/tests/fixtures/graph-directory.ts (created by R3) is seeded into the standalone identity server with disposable local dependencies through DIRECTORY-003's and DIRECTORY-006's own typed operations, and the tests this requirement adds to surface/identity/tests/acceptance/graph.spec.ts (created by R3) compare what the graph draws with what that server's Access returns for the same question. A pair is one identity, one resource and one action that resource declares. THE SYSTEM SHALL NOT compare the graph against a stub of Access or against answers the test wrote, and SHALL NOT need Cambium, Aion, Argus or Manifold (ADR-004). The test evidence names conformance row 8.3 and pins the mock-up hash e67622e19f450e2335029f293f18b4757801f3245758a8b667521afa120ab91f.

**Acceptance:**
- GRAPH_EVERY_PAIR: signed in as person-auditor, for every identity (5) on every resource (3) and every action R1's route returns for that resource, the test asks POST /grants/why through the generated module; it asserts that the number of pairs compared equals 30, the fixture's 5 identities times its 6 declared actions, and that at least 1 answer permits and at least 1 refuses. For each identity it draws the graph with that identity chosen and asserts that every grant edge drawn equals, step for step in order, the path of a permitting answer for the same identity, resource and action, and that every does not stand edge drawn carries the id of the grant a refusal for the same identity, resource and action names, with that refusal's reason text character for character; it asserts the number of drawn edges compared is greater than 0.
- GRAPH_EVERY_PAIR: for each of the 3 resources and each action R1's route returns for it, the holders the graph draws for that resource equal, as a set of ids, the holders the POST /grants/who answer returns; the test asserts the number of reverse questions compared equals 6, the fixture's declared actions.
- GRAPH_REVOKED: every id(X) read from surface/identity/tests/fixtures/graph-directory.ids.json at key X; signed in as person-auditor with agent-owner chosen, the draw holds a grant edge carrying id(G1); after G1 is revoked through DIRECTORY-006's revoke operation and the refresh control is pressed, the draw holds 0 grant edges carrying id(G1), and the number of does not stand edges carrying id(G1) equals the number of that draw's POST /grants/why refusals naming G1, which the test asserts is greater than 0, each edge carrying its refusal's reason text character for character.
- GRAPH_EXPIRED: every id(X) read from surface/identity/tests/fixtures/graph-directory.ids.json at key X; signed in as person-auditor with agent-owner chosen, the draw holds a grant edge carrying id(G3); after the server's named clock is advanced past E and the refresh control is pressed, the draw holds 0 grant edges carrying id(G3), and the number of does not stand edges carrying id(G3) equals the number of that draw's POST /grants/why refusals naming G3, which the test asserts is greater than 0, each edge carrying its refusal's reason text character for character.
- GRAPH_VERSION: for every edge drawn in GRAPH_EVERY_PAIR, the model version shown on it equals the model version of the server's answer it came from; the test asserts the number of edges checked is greater than 0.
- GRAPH_OUTAGE: with the permission engine stopped, signed in as person-auditor, choosing agent-owner shows the refusal element carrying the outage refusal's name as the server returns it, and the draw holds 0 grant edges.
- From the repository root: grep -c 'e67622e19f450e2335029f293f18b4757801f3245758a8b667521afa120ab91f' surface/identity/tests/acceptance/graph.spec.ts prints 1, and grep -c 'conformance 8.3' surface/identity/tests/acceptance/graph.spec.ts prints a number greater than 0.

**Files:**
- modify: surface/identity/tests/acceptance/graph.spec.ts

**Checklist:**
- C273 — Every grant edge and every reachability the graph draws equals an Access answer from the real evaluator for the same question, over every identity, resource and declared action of a fixture directory: paths as returned, a no as Access's reason, a does not stand edge only for a grant a forward refusal names, with Access's reason, a grant Access no longer returns gone on the next draw, standing never inferred, and an outage shown as a named refusal.
- C276 — Choosing a person or agent draws the forward answer for each resource and action its visible grants carry, choosing a resource draws the reverse answer, an administrator holding the reverse question's visibility permission sees the whole directory, a reverse question the person may not ask is refused by name, no hidden grant is drawn or hinted at, and an incomplete reverse answer is marked incomplete.
- C277 — Every grant and does not stand edge shows the model version of the Access answer it came from and every containment edge that of the resource route's answer, a draw of mixed versions names each above the graph, the draw time shows beside the version, and the graph asks Access again only on navigation, reload or its refresh control.

**Stories:**
- S122 (Graph viewer, Looks at who can reach what on the access graph) — As a directory administrator holding the visibility permission, I want to ask the graph who can reach a resource and see Access's answer with its model version and the time it was drawn, so that I can review reach without the screen inventing any of it.
- S123 (Graph viewer, Looks at who can reach what on the access graph) — As a reviewer of conformance row 8.3, I want tests that compare every drawn edge with the real evaluator and prove the graph module holds no rule, so that the graph cannot drift from Access unnoticed.

## Boundaries

- No permission, ancestry, standing, expiry, containment or reachability logic in the graph module under surface/identity/src/features/graph/, and no grant data read other than through Access's answers; responsible people come from the directory's records and resource parents from R1's route, as recorded, a withheld parent drawn only as its mark.
- R1's route records nothing and adds no directory resource record; it serves what the permission model holds, under the visibility of the grants and the directory administrator view, and never names a parent it withholds.
- No mutation from the graph: it has no grant, revoke or delegate control; the only mutations in this brief are the fixture seeding, G1's revocation and the clock advance in the tests, through DIRECTORY-003's and DIRECTORY-006's own operations.
- DIRECTORY-006 is consumed, not amended: no reach operation and no list of grants that do not stand is added to Access.
- No hidden grant's or hidden resource's id, label, count or existence reaches a person who may not see it (DIRECTORY-006 R5, GRANT_EXPLAIN).
- No sample-data authority calculation ships (DIRECTORY-006 R6); the mock-up's can, reach, whoCan and standing are a conformance specification, not code to port.
- No rail entry and no g h shortcut: the shell card AL4OyeKG registers every screen's route and key.
- No suspended-holder drawing and no dependency on DIRECTORY-009: the suspension semantics stay open as DESIGN.md records, and the grant representation stays open; this brief decides neither.
- lys-core, its published wire formats and lys/delegation/v1 are unchanged; no file of IDENTITY-001, no mock-up file and docs/design/identity/CONFORMANCE.md change.
- The card is not moved to a dispatching status until both landed checks in blocked_by exit 0 on lys main.

## Verification

- Before dispatch, from any directory: d=$(mktemp -d) && git clone https://github.com/ablative-io/lys "$d" && cd "$d" && test -d surface/identity && grep -rq 'ID001_STANDALONE' surface/identity && grep -rq 'ID001_SCREEN_REFUSAL' surface/identity exits 0 (DIRECTORY-005 landed).
- Before dispatch, from any directory: d=$(mktemp -d) && git clone https://github.com/ablative-io/lys "$d" && cd "$d" && test -f crates/lys-identity-server/src/grants.rs && grep -q 'GRANT_EXPLAIN' crates/lys-identity-server/tests/grant_explanations.rs && test -f surface/identity/src/generated/index.ts && grep -rq 'GRANT_SCREEN' surface/identity/tests exits 0 (DIRECTORY-006 landed).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0, and sh scripts/design/gate.sh exits 0.
- From the repository root: grep -rEn 'GRAPH_RESOURCES' crates/lys-identity-server/tests and grep -rEn 'GRAPH_BOUNDARY|GRAPH_EVERY_PAIR|GRAPH_REVOKED|GRAPH_EXPIRED|GRAPH_VISIBILITY|GRAPH_VERSION|GRAPH_OUTAGE' surface/identity/tests together find each of the eight identifiers in a test.
- The frontend test command DIRECTORY-005's manifest names runs surface/identity/tests/graph_boundary.test.ts, surface/identity/tests/graph.test.tsx and surface/identity/tests/acceptance/graph.spec.ts with strict types, and the reviewer records the count of tests run and passed for each file.
- The five drift injections of R7 are made one at a time on a scratch branch: each of the four that add a permission-screen import, an API client import reached through surface/identity/src/shell/keyable.ts, the rule function standing or a revoke reference fails GRAPH_BOUNDARY and no test in surface/identity/tests/graph.test.tsx, and the one that adds the router and keyable imports fails no test; the reviewer records the failing test's name for each of the four, and GRAPH_BOUNDARY's checked files and distinct allowed and refused specifiers for all five.
- The repository battery from the exact revision: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean.

