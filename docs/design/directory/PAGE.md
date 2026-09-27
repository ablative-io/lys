# directory — what was asked, what it means, and what was written

## The words, as they were typed

Conformance rows 1.4, 1.5, 2.1, 2.2, 2.3, 2.5 and 8.4 are the grant rows of the directory. What you hold shows each grant, its source and whether it may be passed on (1.4). You shows the signed-in person's own agents, grants and personal secrets as personal scope, not global hiding, and a directory administrator lawfully sees others through the directory screens (1.5). A person gives an agent only a relation at or below one they hold (2.1). Only a grant marked may-pass-on is delegated, and may-pass-on is an affirmative fact on the grant, never no prohibition found (2.2). The form shows the source grant, the actions it allows, may-pass-on, and that the new grant ends no later than its source (2.3). Withdrawing a grant withdraws everything derived from it (2.5). Every grant shows last used, its source and window, and not seen is never shown as never used (8.4). DIRECTORY-006 built the grant contract, the seam and the You and delegation screens that carry each of these, and no brief names a row, so nothing in the tree says which test passes which row.

This card names them. For each row it states the one test that passes it, over the landed server and screens, and adds a test where none exists. On the server, the tests of crates/lys-identity for the relation bound, affirmative pass_on, inherited expiry, cascading revocation and last used are named row by row, and each test's name carries its row. On the screens, one vitest per row over You, GrantCard, Delegate and Revoke proves that the row's words appear from the server's answer and from nothing else: a grant with no use event shows not seen and never never used; the delegation form shows the source grant, its actions, may-pass-on and the end bound; a revoked grant's derived grants leave the screen with it; and a signed-in administrator's People screen shows others while You shows only the person's own. Where a row's words are not met by what landed, this card changes the screen or the seam to meet them and says so by row; it invents no new seam. The Brief column of CONFORMANCE.md is amended on the seven rows to name this brief.

Acceptance: a stated command per row runs its named test and passes; each row's test fails when the row's fact is taken away, checked once for a grant shown without its source, may-pass-on read from a missing prohibition, a derived grant outliving its source, and not seen rendered as never used; CONFORMANCE.md names this brief on the seven rows; and the design gate passes. This brief names rows 1.4, 1.5, 2.1, 2.2, 2.3, 2.5 and 8.4 as the rows it passes. It waits for DIRECTORY-006 to land on main, carried by PR 34, and for the identity screens on hand/identity-surface-land to land, each checked by a command a stranger can run against lys main.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 27 September 2026, against lys main fa3dd531.

## What the survey found, and its angles

Archie wants one brief that ties each of the seven grant rows of CONFORMANCE.md (1.4, 1.5, 2.1, 2.2, 2.3, 2.5, 8.4) to the one test that passes it. On the server that means the lys-identity tests renamed so each name carries its row. On the screens it means one vitest per row that reads the row's words from the server's answer, with a test added where none exists and a drift check proving four of the facts. Where the landed screens or seam fall short of a row's words, the card fixes them inside the existing seam, then amends those seven Brief cells to name itself. It can only be built after PR 34 (DIRECTORY-003/006 grants, R1 to R5) and PR 35 (hand/identity-surface-land, the screens) land. Neither is on main today: main at fa3dd53 has no identity crate and no surface at all.

### What the tree holds

- `docs/design/identity/CONFORMANCE.md` — The seven rows' Brief cells are amended here. The cells read 1.4 'DIRECTORY-001 R3 (amend)', 1.5 'DIRECTORY (amend)', 2.1 'DIRECTORY-001 R3 (amend)', 2.2 'DIRECTORY (amend)', 2.3 'DIRECTORY (amend)', 2.5 'DIRECTORY-001 R3 (exists)' and 8.4 'DIRECTORY (amend)'. The file is identical on PR 34 and PR 35. docs/design/identity has no design.json, so scripts/design/gate.sh never measures this file.
- `docs/design/directory (design.json, checklist.json, stories.json, briefs/)` — The cluster this brief continues. Main holds DIRECTORY-001 to 006 and 008, C1–C30, S1–S12 and CN1–CN12. CN1 is 'Documents only'. CN8 says release builds and tests run at the venue. A non-goal still lists 'arbitrary grants and their enforcement' as road step 2, which DIRECTORY-006 has since built. The gate array uses place:here legs.
- `docs/design/directory/briefs/DIRECTORY-006.md` — The brief whose landed work this card names. Its R6 lists YouGrants.tsx, DelegateGrant.tsx, GrantExplanation.tsx and tests/acceptance/grants.spec.ts, none of which exist on PR 35. It cites the rows per requirement (R1 'Conformance 1.3, 1.4, 2.2, 2.3', R4 '2.5, 2.6, 3.3, 4.5', R5 '8.1, 8.2, 8.4', R6 '1.3–1.5, 2.3–2.4, 8.1 and 8.4').
- `crates/lys-identity/tests/grant_delegation.rs (PR 34/35)` — The server tests for relation bound and affirmative pass-on (rows 2.1 and 2.2): grant_use_vs_lend_use_only_is_exercised_and_never_passed_on, grant_recipient_each_kind_is_permitted_or_refused_by_name, grant_ancestry_refuses_at_the_blocking_boundary_and_explains_a_permitted_chain and grant_agent_parity_routes_decide_alike_and_pass_on_decides_who_delegates. Their names carry DIRECTORY-006 acceptance keys, not rows.
- `crates/lys-identity/tests/grant_contract.rs` — grant_contract_supplies_no_permission_by_default and grant_model_judges_by_action_sets_and_keeps_its_version: the 'may-pass-on read from a missing prohibition' drift (2.2) and the model-based relation bound (2.1).
- `crates/lys-identity/tests/grant_expiry.rs` — grant_expiry_every_end_binds_at_its_boundary_and_no_role_change_extends_it is the inherited-expiry test the words name. It maps to row 2.3's end bound or to row 2.6, which is not among the seven.
- `crates/lys-identity/tests/grant_revocation.rs` — grant_revoke_withdraws_every_derived_grant_and_leaves_the_independent_one is the server test for cascading revocation (2.5).
- `crates/lys-identity/tests/grant_last_used.rs and crates/lys-identity/src/grants/projection.rs` — The last-used tests for 8.4 (three tests; line 51 asserts 'never exercised reads as not seen'). projection.rs:34-35 states the not-seen invariant.
- `crates/lys-identity-server/src/grant_contract/views.rs` — The GrantView the screens read. It has revoked with the doc 'Whether it was revoked directly' (line 139) and window.ends_at with the doc 'no end of its own' (line 72). The server answers neither a grant's effective standing nor its inherited end, so the screen derives both itself.
- `crates/lys-identity-server/tests/read_api.rs` — Server side of 1.5: people_answers_only_the_signed_in_person_with_their_agents and the_administrators_wider_view_is_its_own_route.
- `surface/identity/src/features/me/You.tsx` — Row 1.4 lists What you hold with source ('root' or the source holder) and pass-on (lines 64-73). Row 1.5's 'Your personal secrets' section (lines 134-135) renders NotBuilt 'The secrets store is SECRETS-002.'
- `surface/identity/src/features/grants/GrantCard.tsx` — Row 8.4 shows Chain (source), 'Lasts' (own ends_at only, no start, no inherited end) and 'Last used' via lastUsedText. A revoked-upstream grant stays on screen marked void.
- `surface/identity/src/features/grants/Delegate.tsx` — The row 2.3 form rows: Source grant, Actions it allows, You may pass it on, and 'Ends no later than' (line 124). That last row carries the open-q tag 'illustrative' and is computed from source.window.ends_at, the source's own end rather than its inherited end.
- `surface/identity/src/features/grants/model.ts` — standing() works out revoked, expired, not-active and withdrawn-upstream on the client by walking source ids. lastUsedText returns 'not seen' (line 98). This bears on 'from the server's answer and from nothing else'.
- `surface/identity/src/features/grants/Revoke.tsx and surface/identity/tests/revoke.test.tsx` — Row 2.5 on screen. The landed test (lines 38-56) asserts that a derived grant stays on the agent's Access tab as 'void … which no longer stands', not that it leaves the screen.
- `surface/identity/src/features/people/People.tsx and surface/identity/tests/people.test.tsx` — Row 1.5's administrator half: /directory/people with a fallback to /people ('falls back to the personal view when not admitted to the directory'). People is not among the four screens the words list, but row 1.5's test needs it.
- `surface/identity/tests/grants.test.tsx, me.test.tsx, fixtures.ts, harness.tsx` — The existing vitests. Their describe blocks already name 'conformance 1.4', '2.1 to 2.3', '2.4', '8.1' and '8.2'. Line 200 asserts ['not seen','not seen','27 Sep 12:00 · tool']. No test names 1.5 or 8.4 in its title.
- `docs/design/project.json and .github/workflows/ci.yml and .land/gates.sh (PR 35)` — On PR 35 there are 8 legs (fmt, two clippy, tests, two doc, design, ast-grep). None runs npm or vitest, and CI and .land/gates.sh mention neither, so no gate runs a screen test.
- `scripts/design/gate.sh` — The design gate the acceptance names. It validates decisions.json and project.json, then validates, checks coverage for and byte-compares the render of every cluster with a design.json.
- `docs/design/roadmap.json and docs/design/decisions.json` — Where the new roadmap row and any new decision go. Main holds RM-001 to RM-010 and RM-016, and ADR-001 to ADR-018.

### What was already decided

- ADR-003 — Everything is pegged to a human authority. The person's permissions are the ceiling, every grant says who may exercise it and who may pass it on, and withdrawing the authority stops every grant derived from it. This is the substance of rows 2.1, 2.2 and 2.5.
- ADR-004 — Every project stands alone, so the row tests run against the standalone identity server with no Cambium, Aion or Argus.
- ADR-010 — The identity screens follow Aion's structure with an orange accent. Any screen change made to meet a row keeps it.
- ADR-011 — Proposed: registered, active, suspended, retired. You's agent list drops retired agents and standing() voids grants of non-active holders, which bears on what row 1.5 shows.
- DIRECTORY-006 — R1 to R5 (contract, affirmative delegation, durable events, revocation and expiry, seam) sit on PR 34. R6 (the screens) names files that were never built; the screens came from hand/identity-surface on PR 35.
- DIRECTORY-006 C28 — Observed grant usage names its source and time; not seen is not reported as never used.
- DIRECTORY-006 C29 — The You and delegation screens show real server authority and preserve personal versus administrator visibility.
- directory CN1 — Documents only: nothing outside docs/design/directory/ and docs/design/decisions.json is created or modified. The brief itself stays inside that wall; the CONFORMANCE.md edit and the test changes are build acts.
- directory CN8 — Release builds, checks and tests run through the gate workflow at the venue. Tom's rules 3 and 7 send every compile, test battery and gate to Dean's laptop.
- directory non_goals — Still lists the grant representation as OPEN for Tom, and 'arbitrary grants and their enforcement' as road step 2, which PR 34 implements.
- RM-001 — The briefed feature row for the identity directory with every grant rooted in a person, under which DIRECTORY-006 was planned.
- DIRECTORY-019 (draft/directory/844a587d, not a brief branch) — A draft that writes '; <brief id> <leg> <test name>' conformance entries into every test row's Brief cell. Its acceptance pins rows 1.4, 1.5, 2.1, 2.2, 2.3, 2.5 and 8.4 to end exactly '; DIRECTORY-006 none' and diffs against 7b536253. It proposes ADR-058 (conformance judged by a demand leg) and says screen specs belong to a surface leg that a surface card declares.
- DIRECTORY-025 (draft, blocker list) — Records that DIRECTORY-005 registers no frontend leg, and that every row registers the leg it needs in its own brief.
- mock-up index.v5.html — Tags 'Ends no later than' with open-q 'illustrative' and the title 'The rule is shown, not exercised: nothing here expires'. It also says '"not seen" is not "never used"'.

### What was measured

- Files under crates/lys-identity*, crates/lys-identity-server and surface/ on lys main fa3dd53: 0
- Files under crates/lys-identity* and surface/identity on PR 34 head 334ec0e (hand/DIRECTORY-006-land): 65 crate files, 0 surface files
- Files under crates/lys-identity* and surface/identity on PR 35 head 6f57bf7 (hand/identity-surface-land, stacked on PR 34): 129; 68 files changed, +6931/−143 against PR 34
- PR 34 and PR 35 state (gh, 27 Sep 2026): Both OPEN against main. PR 34 is titled 'DIRECTORY-003 and DIRECTORY-006: the identity directory and grants R1 to R5'.
- lys-identity integration tests on PR 35: 7 files, 24 tests: contract 5, delegation 4, expiry 3, faults 3, last_used 3, receipts 3, revocation 3
- lys-identity-server integration tests on PR 35: 3 files, 15 tests: grants 5, grant_explanations 3, read_api 7
- lys-identity test names that already carry a conformance row: 0 of 24
- Vitest cases on PR 35: 62 across 7 files: grants 18, people 10, file 10, shell 8, help 6, me 5, revoke 5
- Vitest describe blocks naming one of the seven rows: 1.4, '2.1 to 2.3' and 2.5 are named. 1.5 and 8.4 are named nowhere.
- Legs on PR 35 that run vitest or npm (project.json, .land/gates.sh, ci.yml): 0 of 8 legs; 0 CI mentions
- CONFORMANCE.md size and rows: 118 lines, 52 numbered rows; unchanged on PR 34 and PR 35
- Screen file sizes on PR 35: You.tsx 149, Delegate.tsx 144, People.tsx 171, Revoke.tsx 98, GrantCard.tsx 72, model.ts 144 lines
- Remote heads read for ids (git ls-remote origin, 27 Sep 2026): 243 brief/draft/hand/card/design/conformance heads. All were readable from local clones: 6 were missing from stack/lys and found in the briefs/0e2bb567 and briefs/2d8c28b9 clones.
- Highest ids held on main or any open branch at reading: DIRECTORY-029, RM-053, ADR-090, C229, S98, CN25. Next free: DIRECTORY-030, RM-054, ADR-091, C230, S99, CN26. These must be re-checked right before writing.
- Open directory briefs/drafts touching these rows: 1: DIRECTORY-019 on draft/directory/844a587d at b9b9432, written 27 Sep 16:53, which names the same seven rows as '; DIRECTORY-006 none'
- Row 1.5 personal secrets on You: Not built. You.tsx:134-135 renders 'The secrets store is SECRETS-002.'
- GrantView fields for derived standing and inherited end: 0. revoked is 'revoked directly' and window.ends_at is the grant's own end.

### What it means for the other projects

- cambium — The card sits on Cambium's board and runs brief_card, sign-off, card_build_v3, src_pr and src_land. The card key must resolve to this brief's JSON path, and nothing is hand-built or lead-reviewed in place of the chain.
- aion — The workflow chain carries the card. Its inputs name lys, a commit on main after PRs 34 and 35 land, the card and this brief, never a folder.
- method — scripts/design (validate.py, check-coverage.py, render-cluster.py) is the design-system method vendored into lys. The brief must validate and render byte-identically under it, and nothing in the method changes.

### The decisions it stands on

- ADR-003 (honour) — Rows 2.1, 2.2 and 2.5 are this decision's ceiling, affirmative pass-on and withdrawal cascade. The tests name it and must not weaken it.
- ADR-004 (honour) — Row tests run against the standalone identity server and screens with no other Ablative service present.
- ADR-010 (honour) — Any screen change made to meet a row keeps the Aion structure and the orange accent.
- ADR-011 (honour) — Row 1.5's own-agents list and standing() read the four lifecycle states. The card must not change what a state means.
-  (new) — Only if the lead picks a Brief-cell form other than the draft DIRECTORY-019/ADR-058 one: how a conformance row names its brief and test (form, leg, and whether screen tests have a leg) would then need a decision of its own. ADR-058 exists only on a draft branch, not in the ledger.

### What it requires

- Each of rows 1.4, 1.5, 2.1, 2.2, 2.3, 2.5 and 8.4 has exactly one named test, and the brief states the command that runs it.
- Every lys-identity test for the relation bound, affirmative pass-on, inherited expiry, cascading revocation and last used has a name containing its row number.
- One vitest per row asserts the row's words from a fixture server answer. Row 8.4's shows 'not seen' for a grant with no use event and never shows 'never used'.
- The row 2.3 vitest asserts the form shows the source grant, its actions, may-pass-on and the end bound.
- The row 2.5 vitest asserts a revoked grant's derived grants behave as the lead rules (leave the screen, or shown void).
- The row 1.5 vitest asserts that an administrator's People screen lists other people while You lists only the signed-in person's own agents and grants.
- Four drift injections (source removed from a shown grant, pass-on read from a missing prohibition, derived grant outliving its source, not seen rendered as never used) each make exactly one test fail, and it is that row's test.
- Every stated per-row command exits 0 on lys main after PRs 34 and 35 land, run at the venue.
- CONFORMANCE.md's Brief cell on each of the seven rows names this brief's id, and no other row or cell changes.
- The brief records, row by row, every screen or seam change made to meet a row's words, and adds no new route or seam.
- sh scripts/design/gate.sh exits 0 with the new brief, roadmap row and any checklist or story rows in docs/design/directory.
- The brief id, RM id and any ADR id are the next free past main and every open brief, draft and hand branch, per a git ls-remote read immediately before writing.

### What must not change

- No new server route, grant field family or screen seam: 'it invents no new seam'.
- No change to published lys wire formats or lys-core; the grant envelope and receipt formats on PR 34 are not mutated.
- No CONFORMANCE.md row other than the seven, and no #, Behaviour, Kind or Owner cell, is changed.
- Existing DIRECTORY-006 assertions are not weakened while tests are renamed; the same case counts still run.
- No #[ignore], #[allow], _-prefixed unused variable or skipped vitest stands in for a fix.
- Heavy builds, test batteries and gates are not run on Tom's Mac; they go to Dean's laptop.
- docs/design/identity/briefs/IDENTITY-001.* is not changed.
- The row tests use no sample-data authority and no simulated confirmation timer.

### What we must put in place first

- PR 34 (hand/DIRECTORY-006-land, 334ec0e) landed on lys main, checked by git merge-base --is-ancestor against origin/main.
- PR 35 (hand/identity-surface-land, 6f57bf7) landed on lys main, checked the same way.
- The lead's answers to the product decisions above, particularly the Brief-cell form versus the open DIRECTORY-019 draft.
- A fresh git ls-remote id read immediately before the brief is written.

### The risks

- PRs 34 and 35 may be rebased or amended before landing, changing the test names and files this brief pins.
- Renaming the GRANT_* acceptance-keyed tests could break DIRECTORY-006's acceptance traceability and any tool keyed on test names, such as the draft DIRECTORY-019 judge.
- DIRECTORY-019 and this card both amend the same seven Brief cells in incompatible forms; whichever lands second fails its acceptance.
- No leg runs vitest, so the five screen-row tests can rot unseen after landing.
- Removing a grant's source fails rows 1.4, 2.3 and 8.4 together unless isolated, which breaks the one-test-fails drift rule.
- The screens derive standing and inherited ends on the client, so a screen test can pass while the server disagrees.
- Row 1.5's secrets clause cannot be met without SECRETS-002, which risks a claimed pass on a not-built section.
- CONFORMANCE.md is outside every gate (no design.json in docs/design/identity), so a malformed cell edit passes the design gate.
- Many parallel directory drafts (DIRECTORY-009 to 029 across 40+ heads) race for the next ids.

### Still open

- Row 1.5 needs personal secrets on You, but the only secrets seam (SECRETS-002) has not landed and the card may invent no seam. Does 1.5 pass with that section showing 'not built', or does the card leave 1.5 unmet on its secrets clause and say so? The sentence of the words it stands on: "You shows the signed-in person's own agents, grants and personal secrets as personal scope, not global hiding, and a directory administrator lawfully sees others through the directory screens (1.5).". Why only the lead can settle it: surface/identity/src/features/me/You.tsx:134-135 renders NotBuilt 'The secrets store is SECRETS-002.', and no secrets route exists in surface/identity/src/api.ts. What a person sees on You, and whether row 1.5 is claimed passed, depends on the answer.
- After a revoke, should derived grants leave the screen, or stay shown as void with 'no longer stands' as the landed screen and its test do? The sentence of the words it stands on: "On the screens, one vitest per row over You, GrantCard, Delegate and Revoke proves that the row's words appear from the server's answer and from nothing else: a grant with no use event shows not seen and never never used; the delegation form shows the source grant, its actions, may-pass-on and the end bound; a revoked grant's derived grants leave the screen with it; and a signed-in administrator's People screen shows others while You shows only the person's own.". Why only the lead can settle it: surface/identity/tests/revoke.test.tsx:51-56 asserts the derived grant stays on the agent's Access tab marked 'void' and 'which no longer stands' (GrantCard.tsx). Only You's What you hold drops it. Changing this changes what a person sees after withdrawing a grant.
- The server answers only direct revocation and each grant's own end, so the screen works out derived withdrawal and inherited ends in model.ts standing(). Is that client walk over the server's list acceptable as 'from the server's answer', or must the existing GrantView carry each grant's effective standing and end? The sentence of the words it stands on: "On the screens, one vitest per row over You, GrantCard, Delegate and Revoke proves that the row's words appear from the server's answer and from nothing else: a grant with no use event shows not seen and never never used; the delegation form shows the source grant, its actions, may-pass-on and the end bound; a revoked grant's derived grants leave the screen with it; and a signed-in administrator's People screen shows others while You shows only the person's own.". Why only the lead can settle it: crates/lys-identity-server/src/grant_contract/views.rs:139 documents revoked as 'revoked directly' and :72 documents ends_at as 'no end of its own'. surface/identity/src/features/grants/model.ts standing() walks source ids itself. With the client walk, rows 2.3 and 2.5 prove client logic; the alternative adds fields to the grant wire.
- The server now enforces inherited expiry. Should the delegation form drop the mock-up's 'illustrative' tag on 'Ends no later than' and show the source's effective (inherited) end? The sentence of the words it stands on: "The form shows the source grant, the actions it allows, may-pass-on, and that the new grant ends no later than its source (2.3).". Why only the lead can settle it: surface/identity/src/features/grants/Delegate.tsx:124 tags the end row 'illustrative', whose mock-up title reads 'nothing here expires'. Yet crates/lys-identity/tests/grant_expiry.rs enforces ends including an earlier ancestor end, and Delegate.tsx:40 reads only source.window.ends_at. What a person reads on the form changes.
- Which row does the inherited-expiry server test carry: 2.3 (ends no later than its source) or 2.6 (inherited expiry is enforced), which is not among the seven? If 2.6, is it added to this card? The sentence of the words it stands on: "On the server, the tests of crates/lys-identity for the relation bound, affirmative pass_on, inherited expiry, cascading revocation and last used are named row by row, and each test's name carries its row.". Why only the lead can settle it: docs/design/identity/CONFORMANCE.md row 2.6 reads 'Inherited expiry is enforced, not only displayed', and the words name only 1.4, 1.5, 2.1, 2.2, 2.3, 2.5 and 8.4. The answer sets the rows this brief claims and the name the test carries.
- Is the drift 'a derived grant outliving its source' a derived grant surviving its source's revocation (row 2.5) or its source's end (row 2.3)? The sentence of the words it stands on: "Acceptance: a stated command per row runs its named test and passes; each row's test fails when the row's fact is taken away, checked once for a grant shown without its source, may-pass-on read from a missing prohibition, a derived grant outliving its source, and not seen rendered as never used; CONFORMANCE.md names this brief on the seven rows; and the design gate passes.". Why only the lead can settle it: Both grant_revocation.rs (cascade) and grant_expiry.rs (earlier ancestor end) test a derived grant outliving its source, in different senses. The answer decides which row's test the acceptance proves able to fail.
- DIRECTORY-006 did not build the screens: they came from the hand branch on PR 35, with files other than DIRECTORY-006 R6 names. Should the brief record the screens as PR 35's work, and does this card close DIRECTORY-006 R6 or leave it open? The sentence of the words it stands on: "DIRECTORY-006 built the grant contract, the seam and the You and delegation screens that carry each of these, and no brief names a row, so nothing in the tree says which test passes which row.". Why only the lead can settle it: PR 34 (hand/DIRECTORY-006-land) is titled 'grants R1 to R5' and holds 0 surface files. DIRECTORY-006.md R6 lists YouGrants.tsx, DelegateGrant.tsx, GrantExplanation.tsx and tests/acceptance/grants.spec.ts, none of which exist. DIRECTORY-006 does cite rows per requirement, and grants.test.tsx names conformance 1.4, 2.1 to 2.3 and 2.5 in its describe blocks.
- In what form do the seven Brief cells name this brief? Should it be DIRECTORY-019's draft form ('; <brief id> <leg> <test name>' after the kept planning text), and which card yields if both amend the same cells? The sentence of the words it stands on: "The Brief column of CONFORMANCE.md is amended on the seven rows to name this brief.". Why only the lead can settle it: The open draft DIRECTORY-019 (draft/directory/844a587d) requires these same seven cells to end exactly '; DIRECTORY-006 none' and a 45/45 numstat against 7b536253. Whichever lands second fails its own acceptance, and the stranger-readable form of the table changes.
- Does this card register a leg that runs the identity vitests (a surface leg in docs/design/project.json), or do the per-row screen commands stay outside every gate? The sentence of the words it stands on: "Acceptance: a stated command per row runs its named test and passes; each row's test fails when the row's fact is taken away, checked once for a grant shown without its source, may-pass-on read from a missing prohibition, a derived grant outliving its source, and not seen rendered as never used; CONFORMANCE.md names this brief on the seven rows; and the design gate passes.". Why only the lead can settle it: No leg in docs/design/project.json, .land/gates.sh or .github/workflows/ci.yml on PR 35 runs npm or vitest. The draft DIRECTORY-019 leaves the surface leg to 'the surface card', and DIRECTORY-025 says each row registers its own leg. Without one, the screen tests for 1.4, 1.5, 2.3, 2.5 and 8.4 are never run by any land.

### The units beyond the first

- Show a person's personal secrets on You once the secrets store lands (row 1.5, secrets clause) — It needs the SECRETS-002 seam, which does not exist, and the card may invent no seam.
- Register a surface leg that runs the identity vitests in docs/design/project.json — No leg runs any screen test today. Both open drafts leave it to its own card, and adding it changes what lands run.
- Name the test that passes row 2.6 (inherited expiry enforced) — The inherited-expiry test exists, but row 2.6 is not among the seven this card claims.
- Judge the conformance table by a demand leg (DIRECTORY-019) — Reading the named tests and legs and reporting pass or fail per row is a separate script and decision, now on a draft branch.
- Close or amend DIRECTORY-006 R6 against the screens PR 35 built — R6's named files were never built. Reconciling the brief with the hand-built screens is brief maintenance, not row naming.

### The smallest complete shape

One brief in docs/design/directory (next free id, measured as DIRECTORY-030, under a new roadmap row measured as RM-054) whose build, after PRs 34 and 35 land, does four things. It gives each of the seven rows one named test with its stated command: server tests in crates/lys-identity renamed to carry their row, and one vitest per row over You, GrantCard, Delegate, Revoke and People, added where missing. It proves the four drift injections each fail exactly their row's test. It changes the landed screens or the existing grant view only where a row's words are unmet, recording each change by row. It amends the seven Brief cells of CONFORMANCE.md to name the brief. The design gate passes.

## The roadmap row

- **RM-061** — Name the test that passes each grant conformance row of the directory (feature, idea)
- Summary: Tie each grant row of the identity conformance table (1.4, 1.5 for its agents and grants clauses, 2.1, 2.2, 2.3, 2.5, 2.6 and 8.4) to the test that passes it and the command that runs it, over the grant contract and seam DIRECTORY-006 landed on PR 34 and the screens that landed on PR 35: the server's grant view carries each grant's effective standing and end so the screens render the service's judgement, the crates/lys-identity tests carry their row in their name, one vitest per screen row reads the row's words from the service's answer, seven drift injections each fail exactly their row's test, a surface leg runs the vitests on every land and joins the directory design's gate, DIRECTORY-006 R6 names the screen files that exist, and the eight Brief cells name the tests.
- Asked by: tom on 2026-09-27T11:28:31Z
- Context: The grant-rows conformance card on the identity line, filed against lys main fa3dd531. Its survey's questions were answered by the lead for the identity line before this brief was written: row 1.5's secrets clause stays unmet and waits for SECRETS-002; on You a revoked grant's derived grants leave with it while the agent's Access tab keeps them void; the service, not the screen, judges standing and effective end, carried on the existing grant view; the delegation form drops its illustrative tag and shows the effective end; the inherited-expiry test carries row 2.6, which joins the card; the outliving-source drift is two drifts, revocation (2.5) and end (2.6); the screens are recorded as PR 35's work and DIRECTORY-006 R6 is closed against the files that exist; the Brief cells take DIRECTORY-019's entry form, whichever card lands second appending after the other; and the card registers a surface leg that runs the vitests.
- Quote: Conformance rows 1.4, 1.5, 2.1, 2.2, 2.3, 2.5 and 8.4 are the grant rows of the directory. What you hold shows each grant, its source and whether it may be passed on (1.4). You shows the signed-in person's own agents, grants and personal secrets as personal scope, not global hiding, and a directory administrator lawfully sees others through the directory screens (1.5). A person gives an agent only a relation at or below one they hold (2.1). Only a grant marked may-pass-on is delegated, and may-pass-on is an affirmative fact on the grant, never no prohibition found (2.2). The form shows the source grant, the actions it allows, may-pass-on, and that the new grant ends no later than its source (2.3). Withdrawing a grant withdraws everything derived from it (2.5). Every grant shows last used, its source and window, and not seen is never shown as never used (8.4). DIRECTORY-006 built the grant contract, the seam and the You and delegation screens that carry each of these, and no brief names a row, so nothing in the tree says which test passes which row.

This card names them. For each row it states the one test that passes it, over the landed server and screens, and adds a test where none exists. On the server, the tests of crates/lys-identity for the relation bound, affirmative pass_on, inherited expiry, cascading revocation and last used are named row by row, and each test's name carries its row. On the screens, one vitest per row over You, GrantCard, Delegate and Revoke proves that the row's words appear from the server's answer and from nothing else: a grant with no use event shows not seen and never never used; the delegation form shows the source grant, its actions, may-pass-on and the end bound; a revoked grant's derived grants leave the screen with it; and a signed-in administrator's People screen shows others while You shows only the person's own. Where a row's words are not met by what landed, this card changes the screen or the seam to meet them and says so by row; it invents no new seam. The Brief column of CONFORMANCE.md is amended on the seven rows to name this brief.

Acceptance: a stated command per row runs its named test and passes; each row's test fails when the row's fact is taken away, checked once for a grant shown without its source, may-pass-on read from a missing prohibition, a derived grant outliving its source, and not seen rendered as never used; CONFORMANCE.md names this brief on the seven rows; and the design gate passes. This brief names rows 1.4, 1.5, 2.1, 2.2, 2.3, 2.5 and 8.4 as the rows it passes. It waits for DIRECTORY-006 to land on main, carried by PR 34, and for the identity screens on hand/identity-surface-land to land, each checked by a command a stranger can run against lys main.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 27 September 2026, against lys main fa3dd531.
- Cluster: directory; briefs: DIRECTORY-035
- Notes: Ids re-read with git ls-remote origin immediately before this amendment (main a426b9a and 234 brief, draft, hand, card, design and conformance heads, every one fetched into this clone and read): the highest held elsewhere are DIRECTORY-035, RM-061, C293 and S129, and DIRECTORY-035, RM-061, C279 to C285 and S124 to S126 are held only by this draft's own branch, draft/directory/95308eae, so the ids stand. First drafted as DIRECTORY-032 under RM-057, renumbered DIRECTORY-035 under RM-061 because DIRECTORY-032, RM-057 and C257 to C263 were taken on draft/directory/73184e72 and RM-057 also on draft/home/d948c6a9 and draft/secrets/0e6892c4. No new decision: the lead chose DIRECTORY-019's Brief-cell form, so the conditional new decision the survey named is not needed. PRs 34 (334ec0e) and 35 (6f57bf7) were not on main a426b9a at writing. Row 2.6 is proved one check at a time: admission's own check_end by an added test that stops at admission, the hop check at commit and on replay by the existing durability test's replay leg, and the hop check at exercise by an added test that resolves a hand-built chain; the earliest end lineage::resolve folds is kept as defence in depth with no injection, since check_hop runs on every hop before it is read. Further units, not written: Show a person's personal secrets on You once the secrets store lands (row 1.5, secrets clause); Judge the conformance table by a demand leg (DIRECTORY-019).

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
| `crates/lys-identity-server/src/grant_contract/views.rs` | the grant view the screens read; DIRECTORY-035 adds each grant's effective standing and effective end |  |
| `crates/lys-identity/tests/grant_last_used.rs` | the last-used tests of conformance 8.4, named by their row under DIRECTORY-035 |  |
| `surface/identity/src/generated/grants.ts` | the grant wire types the screens compile against, mirroring views.rs; gains the two effective fields |  |
| `surface/identity/src/features/grants/model.ts` | the grant screens' reading of the service's answers; loses the client standing walk |  |
| `surface/identity/src/features/grants/GrantCard.tsx` | one grant with its chain, window, last use and standing (conformance 8.4) |  |
| `surface/identity/src/features/grants/Delegate.tsx` | the delegation form (conformance 2.3): source grant, actions, may-pass-on and the end bound |  |
| `surface/identity/src/features/me/You.tsx` | the You page: What you hold (conformance 1.4) and the personal scope of conformance 1.5 |  |
| `surface/identity/src/features/access/Access.tsx` | the access screens that list grants with their standing and last use |  |
| `surface/identity/src/features/file/sections.tsx` | an identity file's sections that read which grants stand |  |
| `surface/identity/tests/fixtures.ts` | the vitests' fixture service answers |  |
| `surface/identity/tests/me.test.tsx` | the You vitests, carrying row 1.5's test |  |
| `surface/identity/tests/revoke.test.tsx` | the Revoke vitests, carrying row 2.5's screen test |  |
| `docs/design/project.json` | the project's trees and legs; DIRECTORY-035 registers the surface leg |  |
| `docs/design/identity/CONFORMANCE.md` | the identity conformance table; DIRECTORY-035 amends the Brief cell of rows 1.4, 1.5, 2.1, 2.2, 2.3, 2.5, 2.6 and 8.4 |  |

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
id: DIRECTORY-035
cluster: directory
title: Name the test that passes each grant conformance row
---

# DIRECTORY-035: Name the test that passes each grant conformance row

> **Cluster:** directory
> **Depends on:** DIRECTORY-006
> **Blocked by:** DIRECTORY-006 R1 to R5 landed on lys main, carried by PR 34 (hand/DIRECTORY-006-land at 334ec0e): checked by git fetch https://github.com/ablative-io/lys.git main && git ls-tree --name-only FETCH_HEAD -- crates/lys-identity/src/grants/admission.rs crates/lys-identity/tests/grant_last_used.rs, which prints both paths., The identity screens landed on lys main, carried by PR 35 (hand/identity-surface-land at 6f57bf7): checked by git fetch https://github.com/ablative-io/lys.git main && git ls-tree --name-only FETCH_HEAD -- surface/identity/src/features/me/You.tsx surface/identity/src/features/grants/GrantCard.tsx surface/identity/tests/revoke.test.tsx, which prints all three paths., Sign-off of this brief on its card before it is dispatched through the card's workflow chain.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> **Checklist:**
> - C279 — Each of conformance rows 1.4, 1.5 (its agents and grants clauses), 2.1, 2.2, 2.3, 2.5, 2.6 and 8.4 names the test that passes it with a stated command that runs that test alone, and every crates/lys-identity test of the relation bound, affirmative pass-on, inherited expiry, cascading revocation and last used carries its row in its name.
> - C280 — The grant view the service answers carries each grant's effective standing and effective end, judged over its chain on the server, and the screens render those two fields and walk no chain of their own.
> - C281 — The screens meet the rows' words from the service's answer and from nothing else: What you hold shows each grant's source and whether it may be passed on and drops the grants derived from a revoked one, as does each agent's holdings on You; an agent's Access tab keeps them marked void; the delegation form shows the source grant, its actions, may-pass-on and the effective end it ends no later than; a grant card shows last used, its source and its window, and not seen is never shown as never used.
> - C282 — An administrator's People screen shows other people while You shows only the signed-in person's own agents and grants; row 1.5's personal-secrets clause stays unmet and You keeps its not-built panel naming SECRETS-002.
> - C283 — Each of the seven drift injections (a grant shown without its source, may-pass-on read from a missing prohibition, a derived grant surviving its source's revocation, a derived grant surviving its source's end past admission's own check, the same past the hop check at commit and on replay, the same past the hop check at exercise, not seen rendered as never used) makes exactly one test fail, and it is the named test of that row.
> - C284 — A surface leg registered in docs/design/project.json installs surface/identity's dependencies from its lockfile and runs its vitests, so every land runs the screen tests.
> - C285 — CONFORMANCE.md's Brief cell on each of the eight rows names this brief's tests in the '<brief id> <leg> <test name>' entry form after the kept planning text, and DIRECTORY-006 R6 names the screen files that exist in place of the four it names that do not, with no structure path of the directory design named twice.
> **Stories:**
> - S124 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a signed-in person, I want You to show each grant I hold with its source and whether I may pass it on, and only my own agents and grants, while an administrator's People screen shows others, so what I see is what I hold.
> - S125 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a person giving an agent part of a grant, I want the form to show the source grant, the actions it allows, whether I may pass it on and the end the new grant can last no later than, as the service judged them, so I give only what my chain allows.
> - S126 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a stranger checking the conformance table, I want each grant row to name the test that passes it and the command that runs it, and each test to fail when its row's fact is taken away, so the row can be verified without trusting the author.

## Purpose

Conformance rows 1.4, 1.5, 2.1, 2.2, 2.3, 2.5, 2.6 and 8.4 are the grant rows of the directory, carried by the grant contract and seam DIRECTORY-006 landed and by the You, delegation, grant-card and revoke screens that landed from hand/identity-surface-land, yet nothing in the tree says which test passes which row. This brief names, for each row, the test that passes it and the command that runs it, over the landed server and screens; moves the judgement of whether a grant stands and when it ends to the server so the screen rows prove the service's answer; makes each row's test fail when its fact is taken away; registers the leg that runs the screen tests on every land; and writes the entries into the conformance table so a stranger can verify each row.

## Task

After PR 34 and PR 35 land on main, build R1 to R9 in order. R1 adds standing and effective_ends_at to the existing GrantView, judged on the server over each grant's chain, with a refusal the caller may not read whole carried under its own name, a null grant and the withheld text; no route is added. R2 prefixes the crates/lys-identity tests of the relation bound, affirmative pass-on, the end bound at delegation, inherited expiry, cascading revocation and last used with their row; row 2.3's server test is the existing ancestry test, which already refuses an end past its source; row 2.6 carries one test per check that guards it: the existing expiry test for each end binding at exercise, the existing durability test's replay leg for the hop check at commit and on replay, one added test that stops at admission for admission's own check, and one added test that resolves a hand-built chain for the hop check at exercise; these two are the only server tests added to crates/lys-identity. R3 removes the client standing walk from the screens and renders the server's standing; R4 changes the delegation form and the grant card where rows 2.3 and 8.4 are not met. R5 names one vitest per screen row (1.4, 1.5, 2.3, 2.5, 8.4) in the screen's own test file, adding the three that do not exist. R6 registers the surface leg in the project's trees and re-copies the directory design's gate from them. R7 applies the seven drift injections once each and records which test failed. R8 appends this brief's entries to the eight Brief cells; R9 closes DIRECTORY-006 R6 against the files that exist. Changes made to meet a row's words, by row: 1.4, none beyond R3's reading of the server's standing; 1.5, none, the secrets clause stays unmet with the not-built panel naming SECRETS-002; 2.1 and 2.2, none; 2.3, the 'Ends no later than' row reads the source's effective end and loses its 'illustrative' tag (R1, R4); 2.5, the service answers derived grants of a revoked grant void (R1), You drops them from What you hold and from each agent's holdings, and the agent's Access tab keeps them void (R3), since a withdrawal is an append and the history stays visible; 2.6, none, its tests are named and two are added, for admission's own check and for the hop check at exercise (R2); 8.4, the grant card shows its window (R4). Row 2.3 carries two tests, the server's ancestry test, which proves the end bound at delegation, and the form's vitest; rows 2.5 and 8.4 carry a server test and a vitest each; row 2.6 carries four server tests. The earliest end lineage::resolve folds and expiry::within_window reads is a fourth guard of row 2.6, kept as defence in depth with no test and no injection of its own: lineage::resolve runs lineage::check_hop on every hop before that fold is read, so a derived grant ending after its source is refused ExpiryBeyondSource at exercise and the fold never decides on its own, and the drift rule's injection is not owed for a check no case reaches. Out of scope: the personal-secrets section of You, the judge of the conformance table by a demand leg, and any other conformance row. Heavy builds, test batteries and gates run at the venue the gate runs at, never on the authoring machine; only a small single-crate check runs there. Estimate: 6 focused implementer hours.

## Requirements

### R1: Carry each grant's effective standing and effective end on the grant view

WHEN the service answers a grant on GET /grants or GET /grants/{id}, THE SYSTEM SHALL write in its GrantView (crates/lys-identity-server/src/grant_contract/views.rs) two members beside the existing ones: standing and effective_ends_at. standing SHALL be judged on the server by lys_identity::grants::admission::effective over the grant's chain at the service's clock, the same judgement a check makes: {"stands": true} when effective answers a lineage, and {"stands": false, "refusal": <name>, "grant": <id or null>, "reason": <text>} when it refuses, where refusal, reason and grant are read from the ServerError that the existing grant_sight::as_seen_by returns for the refusal and the caller: refusal is that error's name as ServerError::name writes it, reason is that error's text, and grant is the id of the grant on the chain the refusal names when as_seen_by answers the refusal whole and null otherwise. WHEN as_seen_by withholds the refusal because it names a grant or identity the caller may not see, refusal SHALL carry the original refusal's name, which is the name ServerError::Withheld writes (its text begins with the refusal it carries), never the word Withheld; reason SHALL be ServerError::Withheld's text, '<refusal>: the refusal names a grant or identity the caller may not inspect'; and grant SHALL be null. effective_ends_at SHALL be the earliest end on the grant's chain, read from Lineage.ends, and null when nothing on the chain ends or the chain does not resolve. Because admission refuses a derived grant that ends after any end on its chain (check_end in crates/lys-identity/src/grants/lineage.rs, called for every ancestor from crates/lys-identity/src/grants/admission.rs, and lineage::check_hop in crates/lys-identity/src/grants/lineage.rs when a chain is resolved), effective_ends_at is always the grant's own end. The members' TypeScript mirror in surface/identity/src/generated/grants.ts SHALL change with them. The existing members, revoked with its doc 'Whether it was revoked directly' and window.ends_at with its doc 'no end of its own', SHALL keep their meaning. THE SYSTEM SHALL NOT add a route, a request body, a query parameter or any member beyond these two, SHALL NOT change the grant envelope, the grant log's leaves or any receipt format, and SHALL NOT reveal through reason or grant a grant or identity the caller may not see.

**Acceptance:**
- In crates/lys-identity-server/tests/grants.rs a new test a_derived_grant_whose_source_is_revoked_reads_void_naming_the_revoked_source issues a root grant and a grant derived from it, revokes the root, reads GET /grants as the root's holder, and asserts the derived grant's standing is exactly {"stands": false, "refusal": "Revoked", "grant": <the root's id>, "reason": <the text GrantError::Revoked writes for the root>} while its revoked member is false.
- In the same file a new test a_delegation_ending_after_its_ancestor_is_refused_and_effective_end_is_the_grants_own issues a root ending at T+1000 and proves three things: POST /grants for a grant derived from the root ending at T+1001 is refused with refusal ExpiryBeyondSource and the grant revision read before and after it is unchanged, so no grant is created; a grant derived from the root ending at T+500 reads effective_ends_at T+500 on GET /grants and on GET /grants/{id}, equal to its own window.ends_at; and a second root with no end and a grant derived from it with no end each read effective_ends_at null.
- In the same file a new test a_derived_grant_whose_chain_names_an_identity_the_caller_may_not_see_reads_withheld builds, through the service's existing routes and persons only, a chain of three grants on one resource: it registers two persons through POST /people and binds a login to each through POST /people/{id}/logins; the administrator activates each of the two persons through POST /identities/{id}/transitions with transition activate, since a person registered through POST /people starts registered and the service refuses a grant to or from an identity that is not active; the administrator then issues a root grant to the first person with pass-on to persons; the first person, signed in, delegates from it to the second person with pass-on to persons; and the second person, signed in, delegates from that grant to BEA, use-only. BEA does not see the middle grant, whose holder and responsible are the second person and whose issuer is the first person. It then moves the first person from active to suspended through POST /identities/{id}/transitions with transition suspend and reads GET /grants signed in as BEA. BEA's grant's standing is exactly {"stands": false, "refusal": "IdentityNotActive", "grant": null, "reason": "IdentityNotActive: the refusal names a grant or identity the caller may not inspect"}, and the first person's id and the root grant's id each occur 0 times in the whole response body text.
- A grant that stands reads standing exactly {"stands": true} on GET /grants and on GET /grants/{id}, asserted in the second test for the root ending at T+1000 and for its derived grant ending at T+500, read at a service clock before T+500.
- cargo test -p lys-identity-server --all-features --test grants reports 8 passed and 0 failed (5 at the start of the build plus the three tests above).
- rg -n 'effective_ends_at: number \| null' surface/identity/src/generated/grants.ts prints exactly one line, inside the Grant interface, and that interface's standing member is typed as the union of {stands: true} and {stands: false; refusal: string; grant: string | null; reason: string}.
- git diff --stat <base>..HEAD -- crates/lys-core crates/lys-identity/src/grants/codec.rs crates/lys-identity/src/grants/receipt.rs crates/lys-identity/src/grants/events.rs prints nothing, where <base> is the commit the build started from, and git diff <base>..HEAD -- crates/lys-identity-server/src/routes.rs prints nothing.

**Files:**
- modify: crates/lys-identity-server/src/grant_contract/views.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/tests/grants.rs
- modify: surface/identity/src/generated/grants.ts

**Checklist:**
- C280 — The grant view the service answers carries each grant's effective standing and effective end, judged over its chain on the server, and the screens render those two fields and walk no chain of their own.

**Stories:**
- S125 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a person giving an agent part of a grant, I want the form to show the source grant, the actions it allows, whether I may pass it on and the end the new grant can last no later than, as the service judged them, so I give only what my chain allows.

### R2: Name the server's grant tests by the row each passes

The crates/lys-identity integration tests of the relation bound, affirmative pass-on, the end bound at delegation, inherited expiry (each end binding at exercise, the hop check at commit and on replay, admission's own check, and the hop check at exercise), cascading revocation and last used SHALL each carry their conformance row in their name as a row_<major>_<minor>_ prefix put before the existing name, so the DIRECTORY-006 acceptance key the name already carries stays readable; a test that carries two rows carries both prefixes, in row order. The renames are exactly: grant_contract.rs grant_model_judges_by_action_sets_and_keeps_its_version to row_2_1_grant_model_judges_by_action_sets_and_keeps_its_version; grant_contract.rs grant_contract_supplies_no_permission_by_default to row_2_2_grant_contract_supplies_no_permission_by_default; grant_delegation.rs grant_use_vs_lend_use_only_is_exercised_and_never_passed_on to row_2_2_grant_use_vs_lend_use_only_is_exercised_and_never_passed_on; grant_delegation.rs grant_recipient_each_kind_is_permitted_or_refused_by_name to row_2_2_grant_recipient_each_kind_is_permitted_or_refused_by_name; grant_delegation.rs grant_ancestry_refuses_at_the_blocking_boundary_and_explains_a_permitted_chain to row_2_1_row_2_3_grant_ancestry_refuses_at_the_blocking_boundary_and_explains_a_permitted_chain, since its ActionsOutside and ResourceOutside refusals are the relation bound (row 2.1) and its ExpiryBeyondSource refusals, for a requested end one second past its source's end and for no end under a source that ends, are the end bound at delegation (row 2.3); grant_delegation.rs grant_agent_parity_routes_decide_alike_and_pass_on_decides_who_delegates to row_2_2_grant_agent_parity_routes_decide_alike_and_pass_on_decides_who_delegates, since pass-on decides who delegates (row 2.2); grant_expiry.rs grant_expiry_every_end_binds_at_its_boundary_and_no_role_change_extends_it to row_2_6_grant_expiry_every_end_binds_at_its_boundary_and_no_role_change_extends_it, since its exercises at each end are refused Expired naming the grant whose end binds (the check at exercise); grant_faults.rs grant_durability_every_boundary_answers_replay_or_names_what_is_unresolved to row_2_6_grant_durability_every_boundary_answers_replay_or_names_what_is_unresolved, since its leg 'a committed event past its source's end is refused by the projector' appends a signed grant ending after its source to the log past admission, reopens the service so the log is replayed, and asserts the book refuses it ExpiryBeyondSource, writes no relationship for it and holds nothing for its holder (the hop check lineage::check_hop runs at commit and on replay); grant_revocation.rs grant_revoke_withdraws_every_derived_grant_and_leaves_the_independent_one to row_2_5_grant_revoke_withdraws_every_derived_grant_and_leaves_the_independent_one; grant_last_used.rs grant_last_used_an_exercise_is_recorded_with_its_holder_route_and_time to row_8_4_grant_last_used_an_exercise_is_recorded_with_its_holder_route_and_time, grant_last_used_a_reopen_restores_it_from_the_log to row_8_4_grant_last_used_a_reopen_restores_it_from_the_log and grant_last_used_a_refused_check_writes_no_use to row_8_4_grant_last_used_a_refused_check_writes_no_use. Row 2.3's server test is the renamed ancestry test. Two tests are added, both to grant_expiry.rs. The first is for admission's own check (check_end, called in crates/lys-identity/src/grants/admission.rs judge_delegation for every grant on the source's lineage): row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit builds with the test world a root held by one of the world's people, ending at T0+1000 whose pass-on lets both recipient kinds be given read, then calls lys_identity::grants::admission::judge_delegation directly, with the world's grant book, the directory's projection, the support model and the world's clock, on a request from the root's holder to another of the world's people deriving from that root for relation tern ending at T0+1001, and asserts Err(GrantError::ExpiryBeyondSource) with requested '<T0+1001>', source_grant the root's id and source_ends T0+1000, and that the book holds the same records after the call as before it; it never calls World::delegate or Grants::delegate, so the identical refusal the hop check makes at commit is never reached. The second is for the hop check at exercise (lineage::check_hop, called by lineage::resolve in crates/lys-identity/src/grants/lineage.rs on every hop of the walk admission::effective runs through GrantBook::lineage whenever a grant is exercised): row_2_6_resolve_refuses_a_derived_grant_ending_past_its_source_at_exercise writes two grants by hand into a map, as grant_delegation.rs's cycle() writes its chain, so neither admission nor the book's check at commit ever runs on them: a root held by a person, issued by that person, for relation tern with actions read and a pass-on of read to both recipient kinds, window starting at T0 and ending at T0+1000; and a grant derived from that root, issued by the same person to the same person, on the same resource with actions read and the same pass-on, window starting at T0 and ending at T0+2000. It resolves the derived grant with lys_identity::grants::lineage::resolve over that map and asserts Err(GrantError::ExpiryBeyondSource) with requested '<T0+2000>', source_grant the root's id and source_ends T0+1000. The book cannot be built past its own check from outside the crate, since GrantBook::apply runs GrantBook::check, which runs lineage::check_hop itself, so the test stands on the walk every exercise at a time between the two ends runs, and resolve takes no time: the refusal it asserts is the one such an exercise gets. THE SYSTEM SHALL NOT change any renamed test's body or weaken, remove or skip any assertion in it, SHALL NOT rename a test outside the list above, SHALL NOT add a test to crates/lys-identity/tests other than row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit and row_2_6_resolve_refuses_a_derived_grant_ending_past_its_source_at_exercise, and SHALL NOT mark any test #[ignore].

**Acceptance:**
- cargo test -p lys-identity --all-features -- --list (at the venue) lists 26 tests in all under the seven grant_* integration binaries (grant_contract, grant_delegation, grant_expiry, grant_faults, grant_last_used, grant_receipts, grant_revocation): the 24 at the start of the build plus row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit and row_2_6_resolve_refuses_a_derived_grant_ending_past_its_source_at_exercise.
- rg -c '^fn row_' crates/lys-identity/tests prints grant_contract.rs:2, grant_delegation.rs:4, grant_expiry.rs:3, grant_faults.rs:1, grant_revocation.rs:1 and grant_last_used.rs:3, and rg -c '^fn row_' crates/lys-identity/tests/grant_receipts.rs prints nothing.
- For every renamed test, git diff <base>..HEAD -- <its file> shows its fn line as the only changed line of its body, where <base> is the commit the build started from.
- cargo test -p lys-identity --all-features --test grant_delegation -- --exact row_2_1_row_2_3_grant_ancestry_refuses_at_the_blocking_boundary_and_explains_a_permitted_chain reports 1 passed and 0 failed, and its body still asserts GrantError::ExpiryBeyondSource with source_ends equal to the source's end for a requested end one second later and for a requested no end, and GrantError::ActionsOutside and GrantError::ResourceOutside for a relation and a resource outside the source.
- cargo test -p lys-identity --all-features --test grant_expiry -- --exact row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit reports 1 passed and 0 failed, rg -n 'judge_delegation' crates/lys-identity/tests/grant_expiry.rs prints at least one line inside that test, and the test's body contains no call to delegate.
- cargo test -p lys-identity --all-features --test grant_expiry -- --exact row_2_6_resolve_refuses_a_derived_grant_ending_past_its_source_at_exercise reports 1 passed and 0 failed, rg -n 'resolve' crates/lys-identity/tests/grant_expiry.rs prints at least one line inside that test, and the test's body contains no call to delegate, apply or judge_delegation.
- rg -n '#\[ignore\]' crates/lys-identity/tests prints nothing.

**Files:**
- modify: crates/lys-identity/tests/grant_contract.rs
- modify: crates/lys-identity/tests/grant_delegation.rs
- modify: crates/lys-identity/tests/grant_expiry.rs
- modify: crates/lys-identity/tests/grant_faults.rs
- modify: crates/lys-identity/tests/grant_revocation.rs
- modify: crates/lys-identity/tests/grant_last_used.rs

**Checklist:**
- C279 — Each of conformance rows 1.4, 1.5 (its agents and grants clauses), 2.1, 2.2, 2.3, 2.5, 2.6 and 8.4 names the test that passes it with a stated command that runs that test alone, and every crates/lys-identity test of the relation bound, affirmative pass-on, inherited expiry, cascading revocation and last used carries its row in its name.

**Stories:**
- S126 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a stranger checking the conformance table, I want each grant row to name the test that passes it and the command that runs it, and each test to fail when its row's fact is taken away, so the row can be verified without trusting the author.

### R3: Render the service's standing on every screen and remove the client walk

WHILE a grant screen shows whether a grant stands, THE SYSTEM SHALL read the grant's standing member as R1 answers it and nothing else: GrantCard.tsx (the void mark, the reason line and the chain pills), You.tsx (What you hold lists the signed-in person's grants whose standing stands, and the held() line that lists each agent's holdings lists only the agent's grants whose standing stands, in place of its filter on !g.revoked, so a grant derived from a revoked source never shows under its agent), Access.tsx, file/sections.tsx and model.ts cannotGive. standing() SHALL be removed from surface/identity/src/features/grants/model.ts with its Standing type. WHEN a grant's standing does not stand and names a grant other than itself, THE SYSTEM SHALL show it marked void with the text 'it derives from <grantNo of the named grant>, which no longer stands' followed by the service's reason; WHEN it names the grant itself or no grant, THE SYSTEM SHALL show it void with the service's reason alone. The open-question tag 'what suspension refuses: open' SHALL stay on a void grant whose refusal is IdentityNotActive and whose own holder the service answers as suspended. surface/identity/tests/fixtures.ts SHALL answer every fixture grant with both R1 members, and every fixture that answers a revoked grant SHALL answer the standing the service would: the revoked grant and each grant derived from it void with refusal Revoked naming the revoked grant. THE SYSTEM SHALL NOT walk source ids, compare a window with the clock, or read a holder's lifecycle state to decide whether a grant stands, SHALL NOT keep any other client derivation of standing, and SHALL NOT change the ADR-010 structure or the orange accent of any screen.

**Acceptance:**
- rg -n 'standing\(' surface/identity/src prints nothing, and rg -n 'Date.now' surface/identity/src/features/grants/model.ts prints nothing.
- rg -n '\.revoked' surface/identity/src/features/me/You.tsx prints nothing, and R5's row 2.5 test asserts that after the source is revoked the derived grant is absent from its agent's holdings on You.
- In revoke.test.tsx, after the revoke the agent's Access tab card for the derived viewer grant contains 'void' and 'which no longer stands' and the revoked-grant fixture's reason text, and the test's title reads 'withdraws it through the service, and on the Access tab what derives from it stays, void'.
- The grants.test.tsx test 'shows a void grant of a suspended holder with the open question, and no Allows line' passes with its fixture answering the Scribe grant standing {stands: false, refusal: 'IdentityNotActive', grant: null, reason: <the IdentityNotActive text for Scribe as suspended>}, and asserts the card contains that reason followed by 'what suspension refuses: open'.
- A fixture grant answered with standing {stands: true} and window.ends_at one second in the past renders on its GrantCard with no 'void' mark, asserted in grants.test.tsx: the screen does not compare the window with the clock.

**Files:**
- modify: surface/identity/src/features/grants/model.ts
- modify: surface/identity/src/features/grants/GrantCard.tsx
- modify: surface/identity/src/features/me/You.tsx
- modify: surface/identity/src/features/access/Access.tsx
- modify: surface/identity/src/features/file/sections.tsx
- modify: surface/identity/tests/fixtures.ts

**Checklist:**
- C280 — The grant view the service answers carries each grant's effective standing and effective end, judged over its chain on the server, and the screens render those two fields and walk no chain of their own.
- C281 — The screens meet the rows' words from the service's answer and from nothing else: What you hold shows each grant's source and whether it may be passed on and drops the grants derived from a revoked one, as does each agent's holdings on You; an agent's Access tab keeps them marked void; the delegation form shows the source grant, its actions, may-pass-on and the effective end it ends no later than; a grant card shows last used, its source and its window, and not seen is never shown as never used.

**Stories:**
- S124 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a signed-in person, I want You to show each grant I hold with its source and whether I may pass it on, and only my own agents and grants, while an administrator's People screen shows others, so what I see is what I hold.
- S125 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a person giving an agent part of a grant, I want the form to show the source grant, the actions it allows, whether I may pass it on and the end the new grant can last no later than, as the service judged them, so I give only what my chain allows.

### R4: Meet rows 2.3 and 8.4 on the delegation form and the grant card

WHEN the delegation form opens for a source grant, THE SYSTEM SHALL show in its 'Where it comes from' card the rows Source grant, Actions it allows, You may pass it on and 'Ends no later than', the last reading the day of the source's effective_ends_at as R1 answers it, and 'no end' when effective_ends_at is null. The 'illustrative' open-question tag on that row SHALL be removed, and the row SHALL NOT read source.window.ends_at. WHILE a GrantCard shows a grant, THE SYSTEM SHALL show its window as 'Window: <day of window.starts_at> to <day of effective_ends_at>', and 'Window: from <day of window.starts_at>, no end' when effective_ends_at is null, in place of the 'Lasts' line, beside its chain (its source) and 'Last used'. THE SYSTEM SHALL NOT change the window the form requests, the relations it offers or what it sends, and SHALL NOT change lastUsedText or the 'not seen' text of a grant with no use event.

**Acceptance:**
- rg -n 'illustrative' surface/identity/src/features/grants/Delegate.tsx prints nothing, and rg -n 'effective_ends_at' surface/identity/src/features/grants/Delegate.tsx prints the line of the 'Ends no later than' row, and the window the form sends for '7 days' and for 'no end' is the one it sent at the start of the build, asserted by the existing test 'gives through the service, naming source, recipient, relation and an end no later than the source'.
- With a source whose window.ends_at and effective_ends_at are both the fixture's end at(27, 10), the form's first card contains 'Ends no later than' followed by day(at(27, 10)), and with the same source answered with window.ends_at null and effective_ends_at null, a chain with no end at any hop, it contains 'Ends no later than' followed by 'no end' (asserted by R5's row 2.3 test). A source with no end of its own under an ancestor that ends is not a fixture: admission refuses such a grant, and that refusal is proven by row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit (R2, R7), not by a vitest.
- A GrantCard for a grant with window.starts_at S and effective_ends_at E contains 'Window: ' + day(S) + ' to ' + day(E), and one with effective_ends_at null contains 'Window: from ' + day(S) + ', no end'; neither contains 'Lasts:' (asserted by R5's row 8.4 test).

**Files:**
- modify: surface/identity/src/features/grants/Delegate.tsx
- modify: surface/identity/src/features/grants/GrantCard.tsx

**Checklist:**
- C281 — The screens meet the rows' words from the service's answer and from nothing else: What you hold shows each grant's source and whether it may be passed on and drops the grants derived from a revoked one, as does each agent's holdings on You; an agent's Access tab keeps them marked void; the delegation form shows the source grant, its actions, may-pass-on and the effective end it ends no later than; a grant card shows last used, its source and its window, and not seen is never shown as never used.

**Stories:**
- S125 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a person giving an agent part of a grant, I want the form to show the source grant, the actions it allows, whether I may pass it on and the end the new grant can last no later than, as the service judged them, so I give only what my chain allows.

### R5: Name one vitest per screen row over the service's answers

THE SYSTEM SHALL carry, each in the vitest file of the screen it reads, one vitest named by its row whose assertions read the row's words from a fixture service answer and from nothing else. row_1_4_what_you_hold_shows_each_grant_its_source_and_whether_it_may_be_passed_on in grants.test.tsx is the existing What you hold test renamed, with its assertions of source ('root' for a root grant, the source holder's name for a derived one) and pass-on kept. row_2_3_the_form_shows_the_source_grant_its_actions_may_pass_on_and_the_effective_end in grants.test.tsx is the existing delegation-form test renamed, asserting the Source grant, Actions it allows, You may pass it on and 'Ends no later than' rows over two answers the service gives: ROOT_G as fixtures.ts holds it, whose own end at(27, 10) is its effective end, shown as that day, and ROOT_G answered with window.ends_at null and effective_ends_at null, a chain with no end at any hop, shown as 'no end'. A source with no end of its own under an ancestor that ends is not used, since admission refuses such a grant and the service never holds it; that refusal is proven by row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit. row_8_4_a_grant_card_shows_last_used_its_source_and_window_and_never_never_used is added to grants.test.tsx: a GrantCard for a grant with no use event shows 'Last used: not seen', its chain names its source, it shows its window as R4 writes it, and its text contains no 'never used'. row_1_5_an_administrators_people_shows_others_while_you_shows_only_the_persons_own is added to me.test.tsx: signed in as the administrator the fixture service admits to /directory/people, with the test's answer to GET /grants carrying, beside the fixture grants, one grant held by BEA, the People screen lists BEA's display name as fixtures.ts DIRECTORY holds it, and You lists only the signed-in person's agents (Scribe, Courier and Archivist, none of Reviewer and Lamplighter) and What you hold lists only the grants the signed-in person holds (the fixture's owner of project:identity and viewer of project:ledger), none held by BEA, identifying each row by its relation and resource only and never reading its From cell, and You's personal-secrets card still reads 'The secrets store is SECRETS-002.'. row_2_5_what_you_hold_drops_the_grants_derived_from_a_revoked_one is added to revoke.test.tsx over a service answer that adds to the fixture grants three grants on project:handbook, every holder and every delegator of which is an identity fixtures.ts answers as active (ADA and Scribe; BEA, retired in fixtures.ts, is not in this chain): HANDBOOK_G, a root grant ADA issued to ADA as editor with pass-on of view to persons and agents; LENT_G, derived from HANDBOOK_G, delegated by ADA to Scribe as viewer with pass-on of view to persons, ADA answering for Scribe; and RETURNED_G, derived from LENT_G, delegated by Scribe to ADA as viewer, use-only. Each hop's recipient kind and actions lie within its source's pass-on, each holder and each delegator is active, and each grant's responsible is the person the directory records, so admission admits all three with ADA as the root's holder and the first delegator and Scribe as the second. Signed in as ADA, the test reads You, revokes HANDBOOK_G through its revoke control on ADA's Access tab (#/file/<ADA>/access), where GrantCard renders the Revoke control because ADA is active and HANDBOOK_G is not revoked, and reads You again, where the service now answers HANDBOOK_G, LENT_G and RETURNED_G with standing void, refusal Revoked naming HANDBOOK_G. Before the revoke You's What you hold lists editor of project:handbook and viewer of project:handbook, and Scribe's holdings on You list viewer of project:handbook; after it, none of them does, while owner of project:identity, viewer of project:ledger and Scribe's viewer of project:identity stay. The test identifies each row by its relation and resource only and never reads a From cell or a Last used line. The Access tab's keeping of those grants void is the existing revoke test R3 retitles. Each existing describe title that names a conformance row a row test now carries SHALL drop the row number so each of rows 1.4, 1.5, 2.3, 2.5 and 8.4 is named by exactly one vitest. THE SYSTEM SHALL NOT use it.skip, it.todo, describe.skip or .only, SHALL NOT compute a row's words in a test from anything other than the fixture answer and the rendered screen, and SHALL NOT use sample-data authority or a simulated confirmation timer.

**Acceptance:**
- In surface/identity, npx vitest run reports 65 tests passed and 0 failed: the 62 at the start of the build plus the three added row tests for 1.5, 2.5 and 8.4.
- rg -c "it\('row_[0-9]_[0-9]_" surface/identity/tests prints grants.test.tsx:3 (rows 1.4, 2.3 and 8.4), me.test.tsx:1 (row 1.5) and revoke.test.tsx:1 (row 2.5), and nothing for any other file.
- rg -n 'conformance (1\.4|1\.5|2\.3|2\.5|8\.4)|conformance 2\.1 to 2\.3' surface/identity/tests prints nothing.
- rg -n '\.skip|\.todo|\.only' surface/identity/tests prints nothing.
- The row 1.5 test asserts the People screen's names include BEA's display name, You's agent names equal exactly ['Scribe', 'Courier', 'Archivist'], You's What you hold rows, each identified by its relation and resource only, equal exactly the signed-in person's owner of project:identity and viewer of project:ledger, the test reads no From cell, and You's personal-secrets card text contains 'The secrets store is SECRETS-002.'.
- The row 2.5 test asserts You's What you hold holds 4 rows before the revoke (owner of project:identity, viewer of project:ledger, editor of project:handbook, viewer of project:handbook) and 2 after (owner of project:identity, viewer of project:ledger), and Scribe's holdings on You hold 2 rows before (viewer of project:identity, viewer of project:handbook) and 1 after (viewer of project:identity); the revoke is posted to /grants/<HANDBOOK_G>/revoke from the Revoke control of ADA's Access tab; and with the service's post-revoke answer for RETURNED_G replaced by its pre-revoke answer, standing {stands: true}, the test fails at its What you hold assertion.
- In the row 2.5 fixture, every holder and every issuer of HANDBOOK_G, LENT_G and RETURNED_G is ADA or SCRIBE, each answered with state 'active' in fixtures.ts, and none of them is BEA, whose state fixtures.ts answers as 'retired'.

**Files:**
- modify: surface/identity/tests/grants.test.tsx
- modify: surface/identity/tests/me.test.tsx
- modify: surface/identity/tests/revoke.test.tsx

**Checklist:**
- C279 — Each of conformance rows 1.4, 1.5 (its agents and grants clauses), 2.1, 2.2, 2.3, 2.5, 2.6 and 8.4 names the test that passes it with a stated command that runs that test alone, and every crates/lys-identity test of the relation bound, affirmative pass-on, inherited expiry, cascading revocation and last used carries its row in its name.
- C281 — The screens meet the rows' words from the service's answer and from nothing else: What you hold shows each grant's source and whether it may be passed on and drops the grants derived from a revoked one, as does each agent's holdings on You; an agent's Access tab keeps them marked void; the delegation form shows the source grant, its actions, may-pass-on and the effective end it ends no later than; a grant card shows last used, its source and its window, and not seen is never shown as never used.
- C282 — An administrator's People screen shows other people while You shows only the signed-in person's own agents and grants; row 1.5's personal-secrets clause stays unmet and You keeps its not-built panel naming SECRETS-002.

**Stories:**
- S124 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a signed-in person, I want You to show each grant I hold with its source and whether I may pass it on, and only my own agents and grants, while an administrator's People screen shows others, so what I see is what I hold.
- S125 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a person giving an agent part of a grant, I want the form to show the source grant, the actions it allows, whether I may pass it on and the end the new grant can last no later than, as the service judged them, so I give only what my chain allows.
- S126 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a stranger checking the conformance table, I want each grant row to name the test that passes it and the command that runs it, and each test to fail when its row's fact is taken away, so the row can be verified without trusting the author.

### R6: Register the surface leg that runs the identity vitests

THE SYSTEM SHALL add to docs/design/project.json's trees one entry for the tree surface/identity with one leg named surface, whose command is 'npm cit' (npm's install-clean-and-test: npm ci from surface/identity/package-lock.json, then the package's test script, vitest run), whose requires are 'tool:node', 'tool:npm' and 'lockfile:package-lock.json', and whose cadence is round. Each screen row's stated command is that leg's runner filtered to the row's test name: in surface/identity, after npm ci, npx vitest run tests/<file> -t <row test name>. After the tree is appended to project.json, the gate of docs/design/directory/design.json SHALL be re-copied verbatim from project.json's trees as they then stand, so it carries every leg of every tree project.json holds on the main the build starts from, and DESIGN.md SHALL be re-rendered with python3 scripts/design/render-cluster.py docs/design/directory. THE SYSTEM SHALL NOT change any existing tree or leg in docs/design/project.json, SHALL NOT change surface/identity/package.json or its lockfile, and SHALL NOT make the leg demand-only.

**Acceptance:**
- python3 scripts/design/validate.py docs/design/project.json exits 0.
- python3 -c "import json; t=[x for x in json.load(open('docs/design/project.json'))['trees'] if x['tree']=='surface/identity']; print(len(t), [(l['name'], l['command'], l['requires'], l['cadence']) for l in t[0]['legs']])" prints 1 [('surface', 'npm cit', ['tool:node', 'tool:npm', 'lockfile:package-lock.json'], 'round')].
- python3 -c "import json; p=json.load(open('docs/design/project.json'))['trees']; d=json.load(open('docs/design/directory/design.json'))['gate']; print('equal' if p == d else 'differ')" prints equal.
- git diff <base>..HEAD -- docs/design/project.json adds lines only inside the trees array, for the surface/identity entry, and removes none, where <base> is the commit the build started from.
- Run at the venue in surface/identity, npm cit exits 0 and its vitest summary reports 65 passed and 0 failed.

**Files:**
- modify: docs/design/project.json
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C284 — A surface leg registered in docs/design/project.json installs surface/identity's dependencies from its lockfile and runs its vitests, so every land runs the screen tests.

**Stories:**
- S126 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a stranger checking the conformance table, I want each grant row to name the test that passes it and the command that runs it, and each test to fail when its row's fact is taken away, so the row can be verified without trusting the author.

### R7: Prove each row's test fails when the row's fact is taken away

WHEN each of the seven drift injections below is applied alone to the built tree, THE SYSTEM SHALL show exactly one failing test across the suite that runs it, and it SHALL be the named test of that row; each injection is applied once, measured, and reverted before the next, and none is committed. (1) A grant shown without its source: You.tsx's What you hold 'From' cell renders empty for every grant; the surface suite fails only row_1_4_what_you_hold_shows_each_grant_its_source_and_whether_it_may_be_passed_on, since the row 1.5 test reads no From cell. (2) May-pass-on read from a missing prohibition: codec.rs read_pass_on answers a null pass-on member as PassOn::To with the grant's own actions and both recipient kinds in place of refusing it; the lys-identity suite fails only row_2_2_grant_contract_supplies_no_permission_by_default. (3) A derived grant surviving its source's revocation: recovery.rs's GrantChange::Revoke arm removes the relationships of the revoked grant alone, not of the grants derived from it; the workspace suite fails only row_2_5_grant_revoke_withdraws_every_derived_grant_and_leaves_the_independent_one. (4) Not seen rendered as never used: GrantCard.tsx's 'Last used' renders 'never used' for a grant whose last_use.seen is false; the surface suite fails only row_8_4_a_grant_card_shows_last_used_its_source_and_window_and_never_never_used. (5) A derived grant surviving its source's end, past admission's own check: judge_delegation in crates/lys-identity/src/grants/admission.rs no longer calls check_end for the grants on the source's lineage; the workspace suite fails only row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit, since wherever another test reaches commit the hop check refuses the same delegation with the same ExpiryBeyondSource. (6) A derived grant surviving its source's end, past the hop check at commit and on replay: GrantBook::check in crates/lys-identity/src/grants/projection.rs, on an issue event whose source is a grant, runs every check of lineage::check_hop except its closing check_end on the grant's end and its source's, while lineage::resolve still calls check_hop whole; the workspace suite fails only row_2_6_grant_durability_every_boundary_answers_replay_or_names_what_is_unresolved, at its 'refused by name' assertion, since the replayed grant enters the book, while admission's check_end still refuses every delegation the other tests ask for. (7) A derived grant surviving its source's end, past the hop check at exercise: lineage::resolve in crates/lys-identity/src/grants/lineage.rs, on each hop, runs every check of lineage::check_hop except its closing check_end, while GrantBook::check still calls check_hop whole; the workspace suite fails only row_2_6_resolve_refuses_a_derived_grant_ending_past_its_source_at_exercise, since resolve answers the derived grant's lineage with the root's end as its earliest, so an exercise between the two ends is refused only by the fold's Expired naming the source, not ExpiryBeyondSource at the hop, while no other test's book holds a chain the check at commit let through. Each injection changes one call site, so the hop check at commit and the hop check at exercise are each proven by a case only it catches. The earliest-end fold in lineage::resolve, read by expiry::within_window, is the fourth guard of row 2.6 and has no injection: check_hop runs on every hop before the fold is read, so no case reaches the fold unrefused. IF an injection fails more than one test, THEN THE SYSTEM SHALL isolate the row's test with a case only it catches, without weakening the other failing test, and measure again. THE SYSTEM SHALL NOT leave any injection in the tree and SHALL NOT count an injection as proven when its row test did not run.

**Acceptance:**
- For injection (1), in surface/identity npx vitest run reports exactly 1 failed, named row_1_4_what_you_hold_shows_each_grant_its_source_and_whether_it_may_be_passed_on, and 64 passed.
- For injection (2), cargo test -p lys-identity --all-features --no-fail-fast reports exactly 1 failed test, row_2_2_grant_contract_supplies_no_permission_by_default.
- For injection (3), cargo test --workspace --all-features --no-fail-fast reports exactly 1 failed test, row_2_5_grant_revoke_withdraws_every_derived_grant_and_leaves_the_independent_one, failing at its RELATIONSHIPS_AFTER_ROOT_REVOKE assertion.
- For injection (4), in surface/identity npx vitest run reports exactly 1 failed, named row_8_4_a_grant_card_shows_last_used_its_source_and_window_and_never_never_used, and 64 passed.
- For injection (5), cargo test --workspace --all-features --no-fail-fast reports exactly 1 failed test, row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit.
- For injection (6), cargo test --workspace --all-features --no-fail-fast reports exactly 1 failed test, row_2_6_grant_durability_every_boundary_answers_replay_or_names_what_is_unresolved, failing at its 'refused by name' assertion.
- For injection (7), cargo test --workspace --all-features --no-fail-fast reports exactly 1 failed test, row_2_6_resolve_refuses_a_derived_grant_ending_past_its_source_at_exercise, failing at its assertion of ExpiryBeyondSource.
- After the seven measurements git status --porcelain -- crates surface prints only the files R1 to R5 change, and git diff <base>..HEAD -- crates/lys-identity/src/grants/codec.rs crates/lys-identity/src/grants/recovery.rs crates/lys-identity/src/grants/admission.rs crates/lys-identity/src/grants/lineage.rs crates/lys-identity/src/grants/projection.rs prints nothing.

**Checklist:**
- C283 — Each of the seven drift injections (a grant shown without its source, may-pass-on read from a missing prohibition, a derived grant surviving its source's revocation, a derived grant surviving its source's end past admission's own check, the same past the hop check at commit and on replay, the same past the hop check at exercise, not seen rendered as never used) makes exactly one test fail, and it is the named test of that row.

**Stories:**
- S126 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a stranger checking the conformance table, I want each grant row to name the test that passes it and the command that runs it, and each test to fail when its row's fact is taken away, so the row can be verified without trusting the author.

### R8: Name this brief's tests in the Brief cell of the eight rows

THE SYSTEM SHALL amend docs/design/identity/CONFORMANCE.md's Brief cell on rows 1.4, 1.5, 2.1, 2.2, 2.3, 2.5, 2.6 and 8.4, and no other cell or row, in the '<brief id> <leg> <test name>' entry form: the kept planning text stays first, and this brief's entries follow it after '; ' when the cell holds no '; ' yet, and after ', ' at the end of the cell's existing entries when another brief's entries are already there on the main the build starts from. A test name for the tests leg is <binary>::<test name> and for the surface leg is <vitest file>::<test name>. The entries, in this order, are: row 1.4: 'DIRECTORY-035 surface grants.test.tsx::row_1_4_what_you_hold_shows_each_grant_its_source_and_whether_it_may_be_passed_on'; row 1.5: 'DIRECTORY-035 surface me.test.tsx::row_1_5_an_administrators_people_shows_others_while_you_shows_only_the_persons_own, SECRETS-002 none'; row 2.1: 'DIRECTORY-035 tests grant_contract::row_2_1_grant_model_judges_by_action_sets_and_keeps_its_version'; row 2.2: 'DIRECTORY-035 tests grant_contract::row_2_2_grant_contract_supplies_no_permission_by_default'; row 2.3: 'DIRECTORY-035 tests grant_delegation::row_2_1_row_2_3_grant_ancestry_refuses_at_the_blocking_boundary_and_explains_a_permitted_chain, DIRECTORY-035 surface grants.test.tsx::row_2_3_the_form_shows_the_source_grant_its_actions_may_pass_on_and_the_effective_end'; row 2.5: 'DIRECTORY-035 tests grant_revocation::row_2_5_grant_revoke_withdraws_every_derived_grant_and_leaves_the_independent_one, DIRECTORY-035 surface revoke.test.tsx::row_2_5_what_you_hold_drops_the_grants_derived_from_a_revoked_one'; row 2.6: 'DIRECTORY-035 tests grant_expiry::row_2_6_grant_expiry_every_end_binds_at_its_boundary_and_no_role_change_extends_it, DIRECTORY-035 tests grant_expiry::row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit, DIRECTORY-035 tests grant_expiry::row_2_6_resolve_refuses_a_derived_grant_ending_past_its_source_at_exercise, DIRECTORY-035 tests grant_faults::row_2_6_grant_durability_every_boundary_answers_replay_or_names_what_is_unresolved'; row 8.4: 'DIRECTORY-035 tests grant_last_used::row_8_4_grant_last_used_an_exercise_is_recorded_with_its_holder_route_and_time, DIRECTORY-035 surface grants.test.tsx::row_8_4_a_grant_card_shows_last_used_its_source_and_window_and_never_never_used'. The 1.5 entries say that this brief proves the agents and grants clauses and that the personal-secrets clause waits for SECRETS-002; row 1.5 passes in full only when SECRETS-002 lands and its card adds the personal-secrets section to You. THE SYSTEM SHALL NOT change a #, Behaviour, Kind or Owner cell, any other row, the legend or any other brief's entry, and SHALL NOT claim row 1.5's personal-secrets clause.

**Acceptance:**
- Where the build starts from a main on which the eight Brief cells read exactly the planning text (1.4 'DIRECTORY-001 R3 (amend)', 1.5 'DIRECTORY (amend)', 2.1 'DIRECTORY-001 R3 (amend)', 2.2 'DIRECTORY (amend)', 2.3 'DIRECTORY (amend)', 2.5 'DIRECTORY-001 R3 (exists)', 2.6 'DIRECTORY (new row)', 8.4 'DIRECTORY (amend)'), the cells read afterwards exactly the planning text followed by '; ' and the entries above, e.g. row 2.2 reads 'DIRECTORY (amend); DIRECTORY-035 tests grant_contract::row_2_2_grant_contract_supplies_no_permission_by_default'.
- Where a cell already holds another brief's entries after '; ', the cell afterwards is exactly that text followed by ', ' and this brief's entries for the row.
- git diff --no-ext-diff --numstat <base>..HEAD -- docs/design/identity/CONFORMANCE.md prints 8 added and 8 deleted, where <base> is the commit the build started from.
- For each of the eight rows, splitting its Brief cell after its last '; ' on ', ' yields this brief's entries for the row as listed, each matching ^[A-Z][A-Z0-9]*-[0-9]{3} ([a-z][a-z0-9-]* [^ ,]+|none)$.
- Each entry's stated command exits 0 at the venue: row 1.4: (in surface/identity, after npm ci) npx vitest run tests/grants.test.tsx -t row_1_4_what_you_hold_shows_each_grant_its_source_and_whether_it_may_be_passed_on; row 1.5: (in surface/identity, after npm ci) npx vitest run tests/me.test.tsx -t row_1_5_an_administrators_people_shows_others_while_you_shows_only_the_persons_own; row 2.1: cargo test -p lys-identity --all-features --test grant_contract -- --exact row_2_1_grant_model_judges_by_action_sets_and_keeps_its_version; row 2.2: cargo test -p lys-identity --all-features --test grant_contract -- --exact row_2_2_grant_contract_supplies_no_permission_by_default; row 2.3: cargo test -p lys-identity --all-features --test grant_delegation -- --exact row_2_1_row_2_3_grant_ancestry_refuses_at_the_blocking_boundary_and_explains_a_permitted_chain; row 2.3: (in surface/identity, after npm ci) npx vitest run tests/grants.test.tsx -t row_2_3_the_form_shows_the_source_grant_its_actions_may_pass_on_and_the_effective_end; row 2.5: cargo test -p lys-identity --all-features --test grant_revocation -- --exact row_2_5_grant_revoke_withdraws_every_derived_grant_and_leaves_the_independent_one; row 2.5: (in surface/identity, after npm ci) npx vitest run tests/revoke.test.tsx -t row_2_5_what_you_hold_drops_the_grants_derived_from_a_revoked_one; row 2.6: cargo test -p lys-identity --all-features --test grant_expiry -- --exact row_2_6_grant_expiry_every_end_binds_at_its_boundary_and_no_role_change_extends_it; row 2.6: cargo test -p lys-identity --all-features --test grant_expiry -- --exact row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit; row 2.6: cargo test -p lys-identity --all-features --test grant_expiry -- --exact row_2_6_resolve_refuses_a_derived_grant_ending_past_its_source_at_exercise; row 2.6: cargo test -p lys-identity --all-features --test grant_faults -- --exact row_2_6_grant_durability_every_boundary_answers_replay_or_names_what_is_unresolved; row 8.4: cargo test -p lys-identity --all-features --test grant_last_used -- --exact row_8_4_grant_last_used_an_exercise_is_recorded_with_its_holder_route_and_time; row 8.4: (in surface/identity, after npm ci) npx vitest run tests/grants.test.tsx -t row_8_4_a_grant_card_shows_last_used_its_source_and_window_and_never_never_used; each server command reports 1 passed, and each vitest command reports 1 passed and the rest of the file skipped by the filter.

**Files:**
- modify: docs/design/identity/CONFORMANCE.md

**Checklist:**
- C279 — Each of conformance rows 1.4, 1.5 (its agents and grants clauses), 2.1, 2.2, 2.3, 2.5, 2.6 and 8.4 names the test that passes it with a stated command that runs that test alone, and every crates/lys-identity test of the relation bound, affirmative pass-on, inherited expiry, cascading revocation and last used carries its row in its name.
- C282 — An administrator's People screen shows other people while You shows only the signed-in person's own agents and grants; row 1.5's personal-secrets clause stays unmet and You keeps its not-built panel naming SECRETS-002.
- C285 — CONFORMANCE.md's Brief cell on each of the eight rows names this brief's tests in the '<brief id> <leg> <test name>' entry form after the kept planning text, and DIRECTORY-006 R6 names the screen files that exist in place of the four it names that do not, with no structure path of the directory design named twice.

**Stories:**
- S126 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a stranger checking the conformance table, I want each grant row to name the test that passes it and the command that runs it, and each test to fail when its row's fact is taken away, so the row can be verified without trusting the author.

### R9: Close DIRECTORY-006 R6 against the screens that landed

THE SYSTEM SHALL amend DIRECTORY-006 R6's files in docs/design/directory/briefs/DIRECTORY-006.json so it names the files that exist in place of the four that do not: surface/identity/src/features/grants/YouGrants.tsx becomes surface/identity/src/features/me/You.tsx, surface/identity/src/features/grants/DelegateGrant.tsx becomes surface/identity/src/features/grants/Delegate.tsx, surface/identity/src/features/grants/GrantExplanation.tsx becomes surface/identity/src/features/grants/Answer.tsx, and surface/identity/tests/acceptance/grants.spec.ts becomes surface/identity/tests/me.test.tsx and surface/identity/tests/revoke.test.tsx, and SHALL append to R6's spec one sentence saying the screens landed from hand/identity-surface-land at 6f57bf7 (PR 35), not from a build of this brief. In docs/design/directory/design.json's structure, the four rows naming surface/identity/src/features/grants/YouGrants.tsx, surface/identity/src/features/grants/DelegateGrant.tsx, surface/identity/src/features/grants/GrantExplanation.tsx and surface/identity/tests/acceptance/grants.spec.ts SHALL be removed, since the rows for You.tsx, Delegate.tsx, me.test.tsx and revoke.test.tsx already stand; one row SHALL be added for surface/identity/src/features/grants/Answer.tsx under brief DIRECTORY-035 when no row names it; no row is respelled, and the grants.spec.ts row is removed, not split. THE SYSTEM SHALL re-render DIRECTORY-006.md and DESIGN.md with python3 scripts/design/render-cluster.py docs/design/directory. THE SYSTEM SHALL NOT change any other member of DIRECTORY-006 (its id, requirements R1 to R5, R6's acceptance, checklist, stories, boundaries or verification) and SHALL NOT change docs/design/identity/briefs/IDENTITY-001.*, and SHALL NOT leave any structure path named twice.

**Acceptance:**
- rg -c 'YouGrants|DelegateGrant|GrantExplanation|acceptance/grants.spec' docs/design/directory/briefs/DIRECTORY-006.json docs/design/directory/briefs/DIRECTORY-006.md docs/design/directory/design.json docs/design/directory/DESIGN.md prints nothing.
- python3 -c "import json, collections; c=collections.Counter(s['path'] for s in json.load(open('docs/design/directory/design.json'))['structure']); print([p for p, n in c.items() if n > 1])" prints [], and the same structure holds exactly one row whose path is surface/identity/src/features/grants/Answer.tsx.
- Every path in DIRECTORY-006 R6's files exists in the tree: for each, test -e <path> exits 0.
- rg -c 'hand/identity-surface-land at 6f57bf7' docs/design/directory/briefs/DIRECTORY-006.json prints 1, and the same over DIRECTORY-006.md prints 1.
- python3 -c "import json; a=json.load(open('docs/design/directory/briefs/DIRECTORY-006.json')); print([r['id'] for r in a['requirements']])" prints ['R1', 'R2', 'R3', 'R4', 'R5', 'R6'], and git diff <base>..HEAD -- docs/design/identity/briefs prints nothing.

**Files:**
- modify: docs/design/directory/briefs/DIRECTORY-006.json
- modify: docs/design/directory/briefs/DIRECTORY-006.md
- modify: docs/design/directory/design.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C285 — CONFORMANCE.md's Brief cell on each of the eight rows names this brief's tests in the '<brief id> <leg> <test name>' entry form after the kept planning text, and DIRECTORY-006 R6 names the screen files that exist in place of the four it names that do not, with no structure path of the directory design named twice.

**Stories:**
- S126 (Grant conformance reader, Reads the grant screens and the conformance table and verifies each row against a test) — As a stranger checking the conformance table, I want each grant row to name the test that passes it and the command that runs it, and each test to fail when its row's fact is taken away, so the row can be verified without trusting the author.

## Boundaries

- No new server route, request body, query parameter, grant member beyond standing and effective_ends_at, or screen seam; the service's existing routes and the screens' existing api.ts calls are the whole seam.
- No change to lys-core, to any published lys wire format, or to the grant envelope, grant log leaves or receipt formats PR 34 carries.
- No CONFORMANCE.md row other than 1.4, 1.5, 2.1, 2.2, 2.3, 2.5, 2.6 and 8.4, and no #, Behaviour, Kind or Owner cell, changes; row 1.5's personal-secrets clause is not claimed.
- No existing DIRECTORY-006 assertion is weakened or removed while tests are renamed, and the case counts stay those at the start of the build plus the tests this brief adds.
- No #[ignore], #[allow], _-prefixed unused variable, it.skip, it.todo, describe.skip or .only stands in for a fix.
- No drift injection is committed.
- docs/design/identity/briefs/IDENTITY-001.* is not changed.
- The row tests use no sample-data authority and no simulated confirmation timer.
- Every screen change keeps ADR-010's Aion structure and orange accent, and no lifecycle state (ADR-011) changes meaning.
- Heavy builds, test batteries and full gates run at the venue the gate runs at; only a small, warm, single-crate check runs on the authoring machine.

## Verification

- From the repository root: sh scripts/design/gate.sh exits 0.
- At the venue: cargo fmt --all leaves no change, and cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps all exit 0.
- At the venue, in surface/identity: npm cit exits 0 with 65 vitests passed and 0 failed, and npx tsc --noEmit exits 0.
- Each of the fourteen stated per-row commands in R8 exits 0 and reports 1 passed.
- The dev report records, for each of R7's seven injections, the command run and the one failing test's name.
- git diff --name-only <base>..HEAD lists only files named in R1 to R9, where <base> is the commit the build started from.

