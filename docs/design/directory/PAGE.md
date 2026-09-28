# directory — what was asked, what it means, and what was written

## The words, as they were typed

Conformance rows 9.1, 9.2 and 9.3 in docs/design/identity/CONFORMANCE.md (lines 103 to 105) are the shell rows: a thin expandable rail, dock left or right, keyboard navigation and deep links for every screen and tab; a help overlay that numbers what is on screen, whose dismissal never presses what is underneath, and which Escape exits with focus returning; and every clickable element reachable and usable by keyboard. Their brief column says SHELL (new), so no brief names them. No surface app is on lys main today; the surface with its screens comes with the DIRECTORY-006 R6 screens change, and this card builds after that lands. The shell exists only in Tom's mock-up, docs/design/identity/mockup/index.v5.html, and that mock-up is the definition the built shell does not deviate from: the rail at line 261 with its open state kept as a preference (lines 42, 52, 285, 457), the dock side at lines 39, 41, 456, 908 and 1070, the shortcuts at lines 1113 to 1133 (bracket toggles the rail, backslash switches the dock side, g starts a two-key go-to, j and k and Enter move through rows) and the palette at 1097, hash routes at lines 923 and 932 with a route for each screen at 263 to 278 and for some tabs at 759, 760, 868, 996 and 1537, the help overlay's numbered marks at 1439 to 1447, its dismiss layer at 1453 to 1455 and its Escape at 1463. The mock-up itself falls short of the rows in two places: explainOff at line 1449 focuses the Help button or the main screen, not the element that had focus, so opening with the question mark at 1466 does not return focus; and clickable rows at lines 1065, 1072 and 1367 are plain table rows and divs with a mouse handler and no tabindex or key handler, so only j and k reach them and row 9.3 fails as written.

This card builds the shell of the surface as the mock-up has it and closes those two gaps in both the built shell and the mock-up, so the two stay the same. Every screen and every tab has a route, listed in one table that the tests read, and a screen or tab with no route is a test failure by name. The help overlay returns focus to the element that had it. Every element that acts on a click is a button or a link, or carries a tabindex and acts on Enter and Space, and the keyboard cursor moves focus rather than re-rendering. Nothing about the screens' contents changes.

Acceptance: the surface's test command runs in the gate and proves each clause: the rail opens and closes by its button and by the bracket key and the choice is kept across a reload; the dock side switches by its control and by the backslash key and the layout flips; the route table names every screen and every tab, and following each route renders that screen or tab; the palette and the go-to keys reach every screen; the help overlay places one numbered mark per explained element and states the count; a click on the dismiss layer reaches nothing underneath, proved by a handler that would record it; Escape closes the overlay and focus is on the element that had it before; and a walk with Tab over every screen reaches every element with a click action and activates it with Enter or Space, with none left out. This brief names rows 9.1, 9.2 and 9.3 as the rows it passes. It waits for the DIRECTORY-006 R6 screens change to land, checked by a command a stranger can run against lys main.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 27 September 2026, against lys main fa3dd5311e98d1a751c770319e1cfe2570718c76.

Rulings of the lead, Archie, given on 28 September 2026 to the run 3e63b2d8-6ec4-4b50-bd1c-3cb24a268ca5 in answer to its rounds 1 to 7. That run wrote the brief DIRECTORY-037 at 115ae18d, which is not signed off. They are settled here, and the author reopens none of them.

Both, through one landing. The surface on lys today is pull request 35, hand/identity-surface-land at 6f57bf7, which carries DIRECTORY-005's surface and the DIRECTORY-006 grant wire and screens, and this card builds after it lands. The words' sentence is corrected to name that pull request by number and head. The stranger's check is one command against lys main that finds surface/identity/src/shell/Rail.tsx and surface/identity/src/features/me/You.tsx; both present means the dependency is met. Answered by Archie, lead for the identity line.

Yes, limited to what fails as the tree has it. The second gap is restated: the SVG network node g.gn[data-act=node] at 1362 and any data-act element KEYABLE at 1558 to 1562 does not cover, the palette's div rows, Enter at 1133 opening the cursor row rather than the focused one, and j and k moving the cursor without moving focus. The first gap is restated: explainOff at 1449 selects [data-act=explain], not the element that had focus. The words' sentence is corrected to those lines, and each fix has an acceptance line in the built shell and in the mock-up. Answered by Archie.

The set is every route the mock-up has, the fourteen rail screens and every tab at 263 to 278, 759, 760, 868, 996 and 1537, in one table with a column saying whether main draws that screen or tab when this card lands. The rail shows every screen as the mock-up has it, and a screen not yet built routes to the not-yet page pull request 35 already has, so the person sees the mock-up's rail. The test reads the rows marked built and fails by name on a built screen or tab with no route or a route the shell does not serve; a row marked not yet is not a failure until its own card lands and marks it built. Answered by Archie.

No. A segmented filter changes what a screen shows and sets no hash in the mock-up, so it is not a tab and takes no route. The tabs are the ones the mock-up gives a hash at 759, 760, 868, 996 and 1537, and the route table names screens and those tabs only. The people kind filter at 533, the secrets scope filter at 1302 and the graph show toggles at 1348 stay as the mock-up has them, screen content this card leaves unchanged, and the linkable URLs do not change. Answered by Archie.

The rail, routes, keys, palette, explain overlay and the help dock are this shell; the assistant composer is not. The dock drawer, renderDock and toggleDock at 1420 to 1438, is built with its help panel, and the composer's place holds a not-yet note naming row 9.4 as proposed, so no person sees an assistant box. When row 9.4 is decided, its own card fills that place. Answered by Archie.

The dock side switches by the backslash key and by the palette act at 1092, and those two are the controls this card builds and tests; the acceptance's 'by its control' reads as the palette act. The Left/Right segment in Configuration at 908 is screen content and comes with Configuration's own card, which reads the same preference. The words' sentence is corrected to name the palette act, and the route table's Configuration row is marked not yet. Answered by Archie.

A new index.v6.html beside v5, the way v4 was kept beside v5. CONFORMANCE.md line 3 names v6 as the reference with v5 as the prior, and DIRECTORY-006's GRANT_CONFORMANCE pin is updated to v6's hash in the same change, both listed in this card's design files. v6 differs from v5 by the two fixes only, and an acceptance line diffs the two files and prints only those hunks. Answered by Archie.

Keep rendering Settings. Pull request 35 draws Configuration, and its Layout section works, so the route table marks Configuration built with a note that only Layout is built and its other six sections show not yet inside the screen, as Settings.tsx has them. The round 1 answer on the dock control is corrected accordingly: the dock side switches by the Layout segment in Settings.tsx, by the palette act at 1092 and by the backslash key, and the acceptance tests all three. Answered by Archie, lead for the identity line.

Yes. The design's gate array gains a surface leg, npm ci and npm test run in surface/identity with exit 0 expected, so a card_build round measures the surface as src_land does, and R1 names that leg beside the .land/gates.sh line. The brief states that the leg needs node on the build venue, names the check as node --version printing a version, and names a venue without node as a blocker to fix on the venue, not a reason to leave the surface unmeasured. Answered by Archie.

Yes, both, as exact-version development dependencies of surface/identity, @testing-library/user-event and its peer @testing-library/dom, with the versions stated in the brief and package.json and package-lock.json named in the card's file list. The Tab walk drives real keyboard events through user-event rather than hand-made ones, which is what row 9.3 measures. Answered by Archie, lead for the identity line.

docs/design/project.json's trees gain the surface/identity tree in this card, with its measured legs npm ci and npm test, and the design's gate array copies the project setup verbatim again. The design carries no leg of its own. Answered by Archie.

The finding stands and the fix in the third key is the ruling: one listing rule, and both acceptance lines follow it. Decided by Archie, lead for the identity line.

The finding stands and the fix in the third key is the ruling. Decided by Archie.

One rule, the general one: the walk lists every element with a click action that is not inside an aria-hidden layer and is not one of the two backdrops. The palette-open walk therefore lists the whole page plus the open palette, because the rail and page at 6f57bf7 are not aria-hidden while the palette is open and the mock-up does not trap focus there, and the shell does not add a trap the mock-up lacks. The first acceptance line becomes: the listed count in the palette-open walk equals the closed-palette walk's count plus one for #palIn plus the number of #palette .it[data-n] rows, at least 19. The second becomes: the removed set in that walk is #scrim and the help dismiss layer, count two, and nothing else. The sentence that the walk lists the open palette's rows is corrected to say it lists the page and the open palette's rows. Decided by Archie.

The finding stands and the fix in the second key is the ruling: case 4 states its start route. Decided by Archie, lead for the identity line.

R9 case 4 starts on #/access, a route that is neither the first Go to row's destination nor the second's, so choosing the first row assigns a different hash, hashchange fires, and a correct index.v6.html passes while v5 fails on the cursor row; the case states #/access as its start input and '#/people' as its expected newURL. Decided by Archie.

Line 2 stays a clause the suite proves: the count test asserts exactly 16 rows marked built, and its command and expected ok line are stated. The drift injection changes so that the count cannot move: it does not mark roles built. Ruled under r2-built-count-breaks-single-failure-injection. Decided by Archie, lead for the identity line.

The injection is one that only the follow test can catch: swap one built row's target for another built row's component, so the Settings row renders Access while the table still marks exactly 16 rows built. The count test passes, the built-row follow test fails on the Settings row, and nothing else fails. Line 4 is restated to name that injection and the one test it fails. Decided by Archie.

Drop the requirement of at least one hunk in each of the four ranges. Keep the rule that every hunk of the v5 to v6 diff lies within the four ranges, and name the two fixes by their effect: palette rows keyable through the KEYABLE rule and case 4 landing on #/people from #/access. 'Differs by the two fixes only' stands and is measured by the range rule plus the six cases, not by a hunk count. Decided by Archie.

Ruled under r1-acceptance-depends-on-r2-file: R1 is measured against a test that exists at 6f57bf7, so the order R1 to R10 stands and no line reaches forward. Decided by Archie, lead for the identity line.

R1's second acceptance line measures the surface leg against surface/identity/tests/shell.test.tsx as it stands at 6f57bf7: one named assertion in it is changed, the leg reports status 1 and names that test, the assertion is restored, and the leg reports status 0; the committed bytes of shell.test.tsx do not change. The line names the assertion by its test title. Decided by Archie.

Keep the ruled injection and narrow the measurement. Line 4 runs npm test -- tests/routes.test.tsx and requires exactly one failed test, the built-row follow test naming settings, while the test that marks exactly 16 rows built passes in the same run. The full suite is not the measurement of that injection, and surface/identity/tests/shell.test.tsx does not change. Decided by Archie.


Correction of the lead, Archie, given on 28 September 2026 after reading the brief written by run 3e63b2d8 at 115ae18d, which is not signed off. No file outside the structure array is created, PAGE.md among them. R10 adds the path of index.v6.html and its sha256, and ADR-104, in the brief and in decisions.json, says that DIRECTORY-006 pins no file and no hash, so this brief is the first to pin one. The gate array of design.json stays as main has it at the brief commit, and the only change to the gate is the leg R1 adds, removing no existing leg. The structure rows for .land/gates.sh, project.json and identity/CONFORMANCE.md name DIRECTORY-037 in their brief member. Every line number cited from index.v5.html is re-read at 6f57bf7 before writing and corrected: Configuration at 263 to 282, the [ and backslash keys at 1127 and 1128, the palette rows at 1085, 1112 and 1125 to 1126, Escape at 1464, and tr[data-href] and tr[data-act] at 1559 and 1367 named as the rows. Every acceptance line is a command a stranger runs with its expected output, on the model of the second line of R2, each python comparison states its command, and each test line names the summary line expected. Blocked by is a list. RM-064 records the round 4 ruling that #scrim and the help dismiss layer are the removed set, count two, and R8 reads the words' none left out with that ruling. Nothing else in the brief changes. Answered by Archie, lead for the identity line.
Second correction of the lead, Archie, given on 28 September 2026 after reading the brief written by run 073840b5 at 8019110, which is not signed off. The round 3 answer that the design's gate array copies the project setup verbatim is withdrawn, and the first correction's sentence on the gate is the rule: the gate array of docs/design/directory/design.json is byte for byte what main has at the build's parent commit with one leg appended, the surface leg R1 adds, and nothing else changes in it; no tree, leg or requirement is removed, the docs tree with its render, validate and coverage legs and the place:here requirements stay. R1's check compares the head's gate array with the parent's plus that one appended leg, the blockquote that says the only change to the gate is the surface tree is removed, and the brief states that docs/design/project.json's trees and design.json's gate differ on main and that this brief does not reconcile them, so a rebase onto a moved base carries whatever the base's two files hold and changes nothing more. R9's first line states the exact summary line vitest prints for that run, skipped tests included, or runs vitest with --reporter=json and states the command that reads the passed, failed and skipped counts with its exact output; no range. The rendered Blocked by line follows scripts/design/render-brief.py as main has it; the comma join at its line 235 is the renderer's fault, carded separately on this board, and is not a defect of this brief. .land/gates.sh is listed once in design.json, as a structure row naming DIRECTORY-037, and the inventory row for it with its note that there is no leg for the surface is removed. Every verification line that is prose becomes a command with its exact expected output or is removed. No PAGE.md is created. Nothing else in the brief changes. Answered by Archie, lead for the identity line.

Third correction of the lead, Archie, given on 28 September 2026 after the reading of the brief written by run 5c8f331c at ddee159, which is not signed off. The brief adds one line, in its findings and in RM-064's notes, saying that the blockers line joins its entries with a comma and a space because of render-brief.py line 235, that the fault is carded as DIRECTORY-039 under RM-072, and that it is not fixed here. The validate line runs python3 scripts/design/validate.py docs/design/project.json and then python3 scripts/design/validate.py docs/design/directory as two commands, each exiting 0, since validate.py takes one path. Every npm test call whose output is then read as a count sends npm's own output away with a redirect of stdout and stderr to /dev/null, as the verification line already does, so the stated output is the counts alone. The two blockers of DIRECTORY-037.json that had no check gain one: the base blocker is checked by git merge-base --is-ancestor of PR 35's merge commit against HEAD exiting 0, and the sign-off blocker names the card's sign-off field read through its board as the signal. R1 states the tools it requires, node and npm, and the round cadence that its acceptance checks. Every other sentence of the brief at ddee159 stands, and the ids DIRECTORY-037, RM-064 and ADR-104 are kept. Answered by Archie, lead for the identity line.

## What the survey found, and its angles

Build the identity surface's shell (rail, dock side, keys, palette, hash routes, help/explain overlay, help dock) exactly as index.v5.html defines it, on top of the surface pull request 35 brings, and close the mock-up's two keyboard gaps (explain overlay focus return; palette rows, the network SVG node and Enter/j/k focus handling) in both the built shell and a new index.v6.html beside v5. Every clause of conformance rows 9.1–9.3 is proved by the surface's own test command, which becomes a leg of the repository gate. The lead's rulings and three corrections already settle scope, ids (DIRECTORY-037, RM-064, ADR-104), the route set, the dock controls, the v6 file, the gate leg and the acceptance measurements; this card is a rewrite of the unsigned brief at ddee159 with those corrections applied.

### What the tree holds

- `docs/design/identity/CONFORMANCE.md:3,103-105` — Line 3 names index.v5.html as the reference (v4 as prior) and has no hash. Rows 9.1–9.3 are kind 'test' with Brief 'SHELL (new)'. R10 moves line 3 to v6 and adds its path and sha256. Rows 9.1–9.3 and their Brief cells stay unchanged.
- `docs/design/identity/mockup/index.v5.html` — This is the definition. It has 1680 lines and is byte-identical at main a426b9a and at 6f57bf7 (blob 821c72d9). Line numbers re-read: rail at 261; screens at 263–282 (Configuration at 282); #railBtn at 285; iam.labels applied at 457; dock segment at 908; route() at 923; hashchange at 932; PAGES at 1085; dock act at 1092; openPalette at 1097; GO map at 1112; g-pending at 1125–1126; [ and \ at 1127–1128; Enter on the cursor row at 1133; SVG gn node at 1362; tr[data-act=node] at 1367; toggleDock at 1438; explainOn at 1439–1447; explainOff at 1449; dismiss layer at 1455; Escape at 1464; ? at 1466; KEYABLE at 1559; its Enter/Space handler at 1562.
- `docs/design/identity/mockup/index.v6.html (new)` — ADR-104 puts the two fixes in a new file beside v5. Every v5→v6 diff hunk must fall within the four ruled ranges.
- `surface/identity/src/shell/{Rail,Shell,Palette,Explain,Dock,keys,keyable,prefs,railItems,ShellContext}.tsx/.ts @6f57bf7` — This is the shell pull request 35 already carries. The card extends and proves it; it does not start from nothing. keyable.ts and keys.ts are where Enter/Space, j/k focus and the Enter-on-focused-row fixes land.
- `surface/identity/src/routes.tsx @6f57bf7` — This is the router the route table is checked against. The ruled drift injection swaps its /settings/:sec? element to render Access.
- `surface/identity/src/features/notyet/Settings.tsx @6f57bf7` — It draws Configuration's Layout segment, the third dock-side control. Its other six sections show not yet.
- `surface/identity/src/shell/routeTable.ts (new) and surface/identity/tests/{routes,rail,...}.test.tsx (new)` — routeTable.ts is the one table of 44 screen and tab rows with a built column (16 rows built). The tests read it.
- `surface/identity/tests/shell.test.tsx @6f57bf7` — R1's injection target: the 'opens and collapses the rail with [ and keeps the choice' assertion. The committed bytes must not change.
- `surface/identity/package.json and package-lock.json` — @testing-library/user-event and @testing-library/dom are added as exact-version devDependencies. Neither is present at 6f57bf7.
- `.land/gates.sh` — It has 22 lines and 7 `leg` calls on main, with no surface leg (6f57bf7 adds identity_leg but still no surface leg). R1 adds surface_leg and removes nothing.
- `docs/design/project.json` — trees holds only '.' on main and at 6f57bf7. R1 adds a surface/identity tree with the npm ci and npm test legs.
- `docs/design/directory/design.json:686 (gate)` — The gate array must equal the parent's byte for byte plus one appended surface leg on tree '.'. This brief does not reconcile it with project.json.
- `scripts/design/gate.sh, validate.py, check-coverage.py, render-cluster.py, render-brief.py:235` — These are the method. validate.py takes one path per call. render-brief.py:235 joins blocked_by with ', ' (DIRECTORY-039 under RM-072 cards that fault). No script renders PAGE.md.

### What was already decided

- CONFORMANCE 9.1/9.2/9.3 — The shell rows are kind 'test' and owned by Archie. The build conforms when each passes as an acceptance check in the brief named against it.
- CONFORMANCE 9.4 — The assistant's screen access is 'proposed', so the dock's composer place holds a not-yet note naming 9.4.
- DIRECTORY-005 — It delivers the identity surface (surface/identity) this shell sits in; it is carried by PR 35.
- DIRECTORY-006 — It delivers the grant wire and screens, also carried by PR 35. It pins no mock-up file or hash, and this card does not change it.
- ADR-010 — The shell keeps the estate design and the orange accent. Nothing about the look changes.
- ADR-004 — Each project stands alone. The surface is measured in lys's own gate with no dependency on Cambium or Aion.
- DIRECTORY-037 @ ddee159 (unsigned) — This is the prior brief, R1–R10. The third correction says every other sentence stands, and ids DIRECTORY-037, RM-064 and ADR-104 are kept.
- RM-064 / ADR-104 (this card's own drafts) — ADR-104: the shell and mock-up change together, and a corrected mock-up is a new file beside the prior. RM-064 records the ruling that #scrim and the dismiss layer are the removed set, count two.
- RM-072 / DIRECTORY-039 (another card) — This carries the fault in render-brief.py:235's comma join. It is not fixed here.

### What was measured

- lys main head at survey: a426b9a5; the words were filed against fa3dd53, and the four commits between touch lys-log-store only
- surface/ on main: absent
- PR 35 state: OPEN, head 6f57bf7, mergeCommit null
- files under surface/identity at 6f57bf7: 63
- index.v5.html size: 1680 lines; blob 821c72d9, identical at main and 6f57bf7
- index.v4.html size: 1566 lines
- rail screen links in the mock-up: 14 (lines 263–282)
- route table rows ruled: 44 (14 screens and 30 tabs); 16 built, 28 not yet
- legs in .land/gates.sh on main: 7 `leg` lines, 22 lines total, no surface leg
- trees in docs/design/project.json on main and at 6f57bf7: 1 ('.')
- @testing-library/user-event and /dom in package.json at 6f57bf7: 0; present devDeps are vitest 5.0.2 and jsdom 30.1.1
- highest ids on main: ADR-018, RM-016, DIRECTORY-008
- highest ids on any origin branch: ADR-110, RM-072, DIRECTORY-039
- branches holding DIRECTORY-037, ADR-104 or RM-064: 5, all this card's own (brief/ and draft/ directory 3e63b2d8, 073840b5 and 5c8f331c)
- files in ddee159's brief commit: 8, including PAGE.md (1932 lines), which the rulings forbid

### What it means for the other projects

- cambium — The card sits on Cambium's board and must run brief_card → sign-off → card_build_v3 → src_pr → src_land. Sign-off is a named blocker, read from the card's sign-off field.
- aion — The card_build round must measure the new surface/identity tree (npm ci, npm test). The venue needs node and npm. A venue without node is a blocker to fix on the venue.
- method — No change to the method scripts. The render-brief.py comma join is a known fault carded separately (DIRECTORY-039/RM-072).

### The decisions it stands on

- ADR-010 (honour) — The shell keeps the estate structure and the orange accent. No visual change.
- ADR-004 (honour) — The surface is proved inside lys's own gate with no cross-project dependency.
-  (new) — ADR-104 (id kept from this card's drafts): the shell and mock-up change together, and a corrected mock-up is a new file beside the prior. v6 is the first pinned mock-up, by path and sha256. DIRECTORY-006 pins nothing and does not change.

### What it requires

- R1: .land/gates.sh gains surface_leg (npm ci then npm test in surface/identity) and loses no leg.
- R1: project.json gains exactly one tree, surface/identity, with legs npm ci and npm test, requiring tool:node and tool:npm, cadence round.
- R1: design.json's gate equals the parent's plus one appended 'surface' leg on tree '.'.
- R1: validate.py is run as two separate calls, one on project.json and one on docs/design/directory, each exiting 0.
- R1: the injected shell.test.tsx assertion makes surface_leg report status 1 and name the test. Once restored, it reports status 0. shell.test.tsx bytes are unchanged.
- R2: routeTable.ts names exactly 44 rows and marks exactly 16 built. Following each built row renders its screen or tab.
- R2: the Settings→Access injection fails exactly the 'follows every built row' test in tests/routes.test.tsx, and the count test passes.
- R3: the rail toggles by #railBtn and [, and the choice persists in iam.labels across a remount.
- R3: the dock side switches by the Settings Layout segment, the palette act and \, and the shell's dock-right class follows.
- R4: the palette and the g-keys reach every screen.
- R5: every click-action element, palette rows included, is a button or link, or has tabindex and acts on Enter and Space.
- R6: j and k move focus. Enter opens the focused row.
- R7: the overlay places one numbered mark per explained element and states the count. A dismiss-layer click reaches no underlying handler, proved by a recording handler. Escape returns focus to the element that had it.
- R8: a user-event Tab walk over every built screen reaches and activates every click-action element except the two backdrops. In the palette-open walk, the count equals the closed count + 1 + the number of palette rows, which is at least 19.
- R9: index.v6.html exists beside v5, and every v5→v6 hunk lies within the four ruled ranges.
- R10: CONFORMANCE.md line 3 names v6 as the reference with v5 as the prior, with v6's path and sha256. No other line changes.
- Every acceptance line is a command with its exact expected output. npm output read as counts is redirected to /dev/null.
- The brief's findings and RM-064's notes carry the render-brief.py:235 comma-join line naming DIRECTORY-039/RM-072.

### What must not change

- No change to any screen's contents, the segmented filters (533, 1302, 1348) or linkable URLs.
- index.v5.html is not edited.
- DIRECTORY-006 and its brief do not change.
- CONFORMANCE.md rows, kinds, Brief cells and owners do not change.
- No leg, tree or requirement is removed from .land/gates.sh or design.json's gate. The docs tree and the place:here requirements stay.
- The brief does not reconcile project.json's trees with design.json's gate.
- No PAGE.md and no file outside the structure array.
- No assistant composer is built. Its place holds a not-yet note naming row 9.4.
- The shell adds no focus trap the mock-up lacks.
- The render-brief.py comma join is not fixed here.

### What we must put in place first

- PR 35 (6f57bf7) lands on lys main. The brief is rebased onto that main, and the stranger's check finds Rail.tsx and You.tsx.
- Node and npm are on Dean's build venue (node --version prints a vX.Y.Z line).
- The card is signed off through its board's chain.

### The risks

- PR 35 may change before it lands, moving the 16-built count, file paths or test titles. Every 6f57bf7 fact must be re-read at the rebase.
- The ddee159 draft carries PAGE.md, which the rulings forbid. It could be carried forward by mistake.
- The 'next free id' sentence in the words conflicts with the kept ids. The author must follow the correction, not the words.
- Tests that run under jsdom with user-event may differ from a real browser for focus and Tab order. The claim holds on the jsdom axis only.
- npm ci on the venue needs network access or a cache. A cold venue could fail the leg for reasons unrelated to the surface.
- A drift injection that moves the built count would make two tests fail and prove neither. The ruled injection must keep 16 rows built.

### The units beyond the first

- Configuration's other six sections (and its own card's dock segment reading the same preference) — This is screen content outside the shell. The route table marks those rows not yet until that card lands and marks them built.
- Build the remaining ten rail screens and their tabs — Each screen has its own card, and each flips its route-table rows to built when it lands.
- Assistant composer in the dock (row 9.4) — Row 9.4 is only proposed. Its own card fills the not-yet place once 9.4 is decided.
- Fix render-brief.py's blocked_by comma join — This is already carded as DIRECTORY-039 under RM-072 and is a method defect separate from this brief.

### The smallest complete shape

One card, DIRECTORY-037 under RM-064 with ADR-104, built after PR 35 lands. It contains: the gate leg and project tree (R1); routeTable.ts with its tests (R2); the proofs for rail and dock (R3) and for palette and go-to (R4); the fixes for Enter/Space keyability (R5) and focus-moving cursor (R6); the overlay proof (R7); the Tab walk (R8); index.v6.html (R9); and the CONFORMANCE.md line-3 pin (R10). All of it lands in one gated change so the shell, the mock-up and their proofs move together.

## The roadmap row

- **RM-064** — Build the identity surface's shell to the mock-up and pass conformance rows 9.1 to 9.3 (feature, idea)
- Summary: The identity surface's shell as the mock-up defines it: the rail with its kept open state, the dock on either side, the keys, the palette, hash deep links for every screen and tab in one route table, and the numbered help overlay; the mock-up's two gaps (focus return from the overlay, and click actions the keyboard cannot reach or press) closed in the built shell and in index.v6.html beside v5; every clause proved by the surface's test command, run as a leg of .land/gates.sh. Builds after pull request 35 lands on lys main.
- Asked by: tom on 2026-09-27T22:50:00+10:00
- Context: Filed as a card on the identity line against lys main fa3dd531. Its survey asked seven questions and the lead answered all seven: the card waits for pull request 35 (hand/identity-surface-land at 6f57bf7), which carries DIRECTORY-005's surface and DIRECTORY-006's grant wire and screens, checked by finding surface/identity/src/shell/Rail.tsx and surface/identity/src/features/me/You.tsx on main; the second gap is the network node at 1362, any data-act element KEYABLE misses, the palette rows, Enter at 1133 and j and k at 1132, and the first is explainOff at 1449 selecting [data-act=explain]; the route table holds every route the mock-up has with a built column, and a not-yet row is not a failure; segmented filters are not tabs; the assistant composer is not this shell and its place holds a not-yet note; the corrected mock-up is index.v6.html beside v5, with CONFORMANCE.md line 3 moved to it. Two further answers followed the first draft: Configuration, which pull request 35 already draws in Settings.tsx, is marked built with a note that only its Layout section is built, and the dock side switches by that Layout segment, by the palette act at 1092 and by \, all three tested; the project setup's trees gain a surface/identity tree measured by npm ci and npm test, which needs node on the build venue, and the design's gate array gains the one surface leg; and the Tab walk drives real keys through @testing-library/user-event 14.6.7 and its peer @testing-library/dom 10.4.2, exact-version development dependencies. In round 4 the lead ruled the Tab walk's listing rule: every element with a click action that is not inside an aria-hidden layer and is not one of the two backdrops is listed, so with the palette open the walk lists the page and the open palette's rows, and the removed set in that walk is #scrim and the help dismiss layer, count two, and nothing else; the words' 'with none left out' is read with that ruling. The lead's correction after the first brief: no file outside the design's structure array is created; DIRECTORY-006 pins no file and no hash (ADR-104), so this brief is the first to pin one, and R10 adds index.v6.html's path and sha256; the structure rows for .land/gates.sh, project.json and CONFORMANCE.md name this brief; every cited line of index.v5.html is re-read at 6f57bf7; and every acceptance line is a command a stranger runs with its expected output. The lead's second correction withdrew the round 3 answer that the design's gate array copies the project setup: the gate array is byte for byte what the build's parent commit has with the one surface leg appended, whatever that parent holds, and project.json's trees and design.json's gate differ on main and are not reconciled here; pull request 35 already drops the docs tree and the place:here requirements from design.json's gate, which is recorded as a finding for the lead, not as this card's work. Every test line states exact passed, failed and skipped counts read from vitest's JSON report, every verification line is a command with its exact output, and .land/gates.sh is listed once in the design, as a structure row. The lead's answers to this run's survey: DIRECTORY-037, RM-064 and ADR-104 are kept, because the heads that also hold them are this card's own earlier drafts and a card's own earlier heads take no id from it; the brief's base is lys main a426b9a5bfa47abc935497b2caddee800b5d72f0, the filing against fa3dd5311e98d1a751c770319e1cfe2570718c76 standing as history, the four commits between them lys-log-store only. The lead's answer to the finding that the brief commit replaced design.json's gate with project.json's trees: at the brief commit the gate array stays byte for byte as lys main a426b9a5 has it, the surface leg is appended only by the build on top of its parent, the method's rule that copies project.json's trees into the gate is not followed here because that copy removes the docs tree and the place:here requirements, and the reconciliation of the two files is a separate card the lead has taken.
- Quote: Conformance rows 9.1, 9.2 and 9.3 in docs/design/identity/CONFORMANCE.md (lines 103 to 105) are the shell rows: a thin expandable rail, dock left or right, keyboard navigation and deep links for every screen and tab; a help overlay that numbers what is on screen, whose dismissal never presses what is underneath, and which Escape exits with focus returning; and every clickable element reachable and usable by keyboard. Their brief column says SHELL (new), so no brief names them. No surface app is on lys main today; the surface with its screens comes with the DIRECTORY-006 R6 screens change, and this card builds after that lands. The shell exists only in Tom's mock-up, docs/design/identity/mockup/index.v5.html, and that mock-up is the definition the built shell does not deviate from: the rail at line 261 with its open state kept as a preference (lines 42, 52, 285, 457), the dock side at lines 39, 41, 456, 908 and 1070, the shortcuts at lines 1113 to 1133 (bracket toggles the rail, backslash switches the dock side, g starts a two-key go-to, j and k and Enter move through rows) and the palette at 1097, hash routes at lines 923 and 932 with a route for each screen at 263 to 278 and for some tabs at 759, 760, 868, 996 and 1537, the help overlay's numbered marks at 1439 to 1447, its dismiss layer at 1453 to 1455 and its Escape at 1463. The mock-up itself falls short of the rows in two places: explainOff at line 1449 focuses the Help button or the main screen, not the element that had focus, so opening with the question mark at 1466 does not return focus; and clickable rows at lines 1065, 1072 and 1367 are plain table rows and divs with a mouse handler and no tabindex or key handler, so only j and k reach them and row 9.3 fails as written.

This card builds the shell of the surface as the mock-up has it and closes those two gaps in both the built shell and the mock-up, so the two stay the same. Every screen and every tab has a route, listed in one table that the tests read, and a screen or tab with no route is a test failure by name. The help overlay returns focus to the element that had it. Every element that acts on a click is a button or a link, or carries a tabindex and acts on Enter and Space, and the keyboard cursor moves focus rather than re-rendering. Nothing about the screens' contents changes.

Acceptance: the surface's test command runs in the gate and proves each clause: the rail opens and closes by its button and by the bracket key and the choice is kept across a reload; the dock side switches by its control and by the backslash key and the layout flips; the route table names every screen and every tab, and following each route renders that screen or tab; the palette and the go-to keys reach every screen; the help overlay places one numbered mark per explained element and states the count; a click on the dismiss layer reaches nothing underneath, proved by a handler that would record it; Escape closes the overlay and focus is on the element that had it before; and a walk with Tab over every screen reaches every element with a click action and activates it with Enter or Space, with none left out. This brief names rows 9.1, 9.2 and 9.3 as the rows it passes. It waits for the DIRECTORY-006 R6 screens change to land, checked by a command a stranger can run against lys main.

Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). The brief id, roadmap row and any decision take the next free id past main and every open brief, draft and hand branch, checked with git ls-remote immediately before writing. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie, lead for the identity line, on 27 September 2026, against lys main fa3dd5311e98d1a751c770319e1cfe2570718c76.
- Cluster: directory; briefs: DIRECTORY-037
- Notes: Ids kept under the lead's answer that a card's own earlier heads take no id from it: 115ae18d, 2132984f and 8019110 are this card's own drafts. Rechecked with git ls-remote over 355 refs, every head fetched, immediately before this write: only this card's own heads (brief/ and draft/directory/3e63b2d8, brief/directory/073840b5 and brief/ and draft/directory/5c8f331c) hold them as ids; brief/ and draft/directory/09884e04, brief/ and draft/directory/504746ca and draft/directory/b0d75195 name them in prose only, and no other head holds DIRECTORY-037, RM-064, ADR-104, C303 to C312 or S132 to S135. Filed against lys main fa3dd531, written against lys main a426b9a5; the four commits between them are lys-log-store only. Left for later, by title: Configuration's other six sections (Sign-in, Directory, Permissions, Secrets, Runtimes, Storage and keys); Row 9.4: the assistant that sees the screen only on opt-in; Mark each not-yet screen built as its card lands (roles, resources, graph, requests, reviews, secrets, connections, network, sessions, model); Reconcile docs/design/project.json trees with design.json's gate; Fix render-brief.py's comma join of Blocked by. Findings. The Blocked by line of DIRECTORY-037's rendered page joins its entries with a comma and a space because scripts/design/render-brief.py line 235 joins blocked_by that way; the fault is carded as DIRECTORY-039 under RM-072 and is not fixed here.

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

IDENTITY-001 revision 5 is the reviewed plan for this, in the older row form, and it predates Tom's ruling of 22 September 17:15 that every grant is pegged to a human authority, his PostgreSQL ruling of 23 September 14:14, and the working lifecycle states. Its row 02 installs SpiceDB without saying what it enforces. It cannot be dispatched to the design-system loop as it stands. Separately, conformance rows 9.1 to 9.3, the identity surface's shell, name no brief, and the mock-up that defines the shell falls short of rows 9.2 and 9.3 itself: its help overlay does not return focus to the element that had it, and the network map's nodes, the palette's rows and the j and k cursor are not fully usable by keyboard.

## Solution

Carry IDENTITY-001's open rows (02, 04, 03, 05) into design-system briefs in this cluster, DIRECTORY-002 to DIRECTORY-005, revised for the grant ruling (ADR-003), the PostgreSQL ruling (ADR-005) and the working lifecycle states (ADR-011, proposed), with the fork (ADR-009) and the product accents (ADR-010) in the project ledger and every decision still open for Tom marked open. The IDENTITY-001 files stay as they are, as the record of revision 5.

DIRECTORY-037 builds the shell to the mock-up on the surface pull request 35 lands, which already carries a shell under surface/identity/src/shell. It adds one route table (surface/identity/src/shell/routeTable.ts) naming every rail screen and every hash-routed tab with a built column; the tests read it as the list the router is checked against, so routes.tsx does not read it. It closes the two gaps in the built shell (keyable.ts, Palette.tsx, keys.ts) and in the mock-up, written as index.v6.html beside v5, with CONFORMANCE.md's reference line moved to it and carrying its path and sha256; DIRECTORY-006 pins no file and no hash, so this is the first pin of the mock-up (ADR-104), and adds the surface's test command to .land/gates.sh, to the project setup's trees and, as one appended leg, to this design's gate, so every clause is proved on every landing.

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
- ADR-104 — The shell and its mock-up change together, and a corrected mock-up is a new version beside the prior — A change to the shell's behaviour is made in the built shell and in the mock-up in the same change, and the corrected mock-up is written as a new file, index.v6.html, beside the prior, the way v4 was kept beside v5. CONFORMANCE.md's reference line moves to the new file in that change and carries its path and sha256; DIRECTORY-006 pins no file and no hash, so DIRECTORY-037 is the first brief to pin one, and DIRECTORY-006 does not change. Rejected: fixing only the build, which lets the definition drift from what is built; and editing index.v5.html in place, which changes the file the conformance table was written against.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.
- DIRECTORY-037 passes conformance rows 9.1 to 9.3: the surface's test command runs in .land/gates.sh and proves the rail, the dock side by all three of its controls, the route table, the palette and go-to keys, the help overlay and a counted Tab walk, and index.v6.html carries the same two fixes as the built shell.

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
- Building the screens and sections the route table marks not yet: roles, resources, graph, requests, reviews, secrets, connections, network, sessions, model, and the six Configuration sections other than Layout — Each is screen content, which DIRECTORY-037 leaves unchanged, and each belongs to its own conformance rows and card; its row is marked built when that card lands.
- The assistant's composer in the dock (conformance 9.4) — Row 9.4 is proposed with no brief and its runtime is open; the dock holds a not-yet note in its place.
- A digit key for the identity file's eighth tab — The mock-up's digit keys stop at 7 while v5 has eight tabs; changing that departs from the mock-up and needs its own decision.
- Pointing the Brief cells of CONFORMANCE.md rows 9.1 to 9.3 at DIRECTORY-037 — The words change only line 3 of that file in this card; the Brief cells follow once the brief id is fixed on main.

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
| `docs/design/directory/briefs/DIRECTORY-037.json` | the identity surface shell brief: the route table, the two keyboard gaps closed in the shell and the mock-up, and the gate leg | DIRECTORY-037 |
| `docs/design/directory/briefs/DIRECTORY-037.md` | rendered markdown | DIRECTORY-037 |
| `.land/gates.sh` | the repository gate; DIRECTORY-037 adds the surface's test leg, surface_leg | DIRECTORY-037 |
| `docs/design/project.json` | the project setup; DIRECTORY-037 adds the surface/identity tree with its measured legs npm ci and npm test | DIRECTORY-037 |
| `surface/identity/package.json` | the surface package, arriving with pull request 35 (DIRECTORY-005); DIRECTORY-037 adds @testing-library/user-event 14.6.7 and its peer @testing-library/dom 10.4.2 as development dependencies | DIRECTORY-005 |
| `surface/identity/package-lock.json` | the surface lock file, arriving with pull request 35 (DIRECTORY-005); follows package.json | DIRECTORY-005 |
| `surface/identity/src/shell/keyable.ts` | makes a non-button click action focusable and answer Enter and Space; arrives with pull request 35, DIRECTORY-037 keeps keyable(activate) and makes Enter and Space dispatch the click that runs it | DIRECTORY-005 |
| `surface/identity/src/shell/Palette.tsx` | the command palette; arrives with pull request 35, DIRECTORY-037 makes its rows keyboard-reachable | DIRECTORY-005 |
| `surface/identity/src/shell/keys.ts` | the shell key registry; arrives with pull request 35, DIRECTORY-037 makes j and k move focus and Enter open the focused row | DIRECTORY-005 |
| `surface/identity/src/shell/routeTable.ts` | the one table of every screen and tab route, with its built column | DIRECTORY-037 |
| `surface/identity/tests/routes.test.tsx` | route table checks: the 44 rows, built rows followed, drawn hashes matched | DIRECTORY-037 |
| `surface/identity/tests/rail.test.tsx` | rail labels and dock side, by control and key, kept across a reload | DIRECTORY-037 |
| `surface/identity/tests/goto.test.tsx` | palette Go to entries and g go-to letters reach every screen | DIRECTORY-037 |
| `surface/identity/tests/keyable.test.tsx` | Enter and Space on click actions and palette rows | DIRECTORY-037 |
| `surface/identity/tests/cursor.test.tsx` | j and k move focus; Enter opens the focused row | DIRECTORY-037 |
| `surface/identity/tests/overlay.test.tsx` | the help overlay's count, dismissal and focus return | DIRECTORY-037 |
| `surface/identity/tests/walk.test.tsx` | the counted Tab walk over every built route | DIRECTORY-037 |
| `surface/identity/tests/mockup.test.tsx` | index.v6.html's two fixes, proved on v6 and shown failing on v5 | DIRECTORY-037 |
| `docs/design/identity/mockup/index.v6.html` | the mock-up with the focus fix and the keyboard fix, beside v5 (ADR-104) | DIRECTORY-037 |
| `docs/design/identity/CONFORMANCE.md` | the mock-up conformance table; DIRECTORY-037 changes its line 3 to name index.v6.html, with its sha256, as the reference and v5 as the prior | DIRECTORY-037 |

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
- `docs/design/identity/mockup/index.v5.html` — the accepted mock-up, 1680 lines, sha256 e67622e19f450e2335029f293f18b4757801f3245758a8b667521afa120ab91f; the shell's definition; kept as the prior, never changed

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
id: DIRECTORY-037
cluster: directory
title: Build the identity surface's shell to the mock-up and close its two keyboard gaps
---

# DIRECTORY-037: Build the identity surface's shell to the mock-up and close its two keyboard gaps

> **Cluster:** directory
> **Depends on:** DIRECTORY-005, DIRECTORY-006
> **Blocked by:** Pull request 35 (hand/identity-surface-land at head 6f57bf7), which carries DIRECTORY-005's surface and DIRECTORY-006's grant wire and screens, lands on lys main. The check, from any clone of lys: git fetch https://github.com/ablative-io/lys.git main && git ls-tree --name-only FETCH_HEAD -- surface/identity/src/shell/Rail.tsx surface/identity/src/features/me/You.tsx prints exactly two lines, surface/identity/src/features/me/You.tsx and then surface/identity/src/shell/Rail.tsx; both present means the dependency is met, and fewer than two lines means it is not and the brief is not dispatched (CN12)., Node is on the build venue: node --version, run there, prints one line matching ^v[0-9]+\.[0-9]+\.[0-9]+$. A venue without node is a blocker to fix on the venue, not a reason to leave the surface unmeasured., This brief is rebased onto lys main after pull request 35 lands, because that pull request changes docs/design/directory/design.json, docs/design/project.json and the directory briefs this brief is written beside; the rebase carries whatever the base's project.json and design.json hold and changes nothing more in them than R1 names. The check, from a clone of lys with the brief's head checked out: m=$(curl -s https://api.github.com/repos/ablative-io/lys/pulls/35 | python3 -c "import json,sys; p=json.load(sys.stdin); print(p['merge_commit_sha'] if p['merged'] else '')"); git fetch -q https://github.com/ablative-io/lys.git main && git merge-base --is-ancestor "$m" HEAD; echo $? prints 0, meaning pull request 35's merge commit is an ancestor of the brief's head; any other status, an unmerged pull request's empty commit among them, means the rebase has not happened and the brief is not dispatched., Sign-off of this brief through its board's chain before it is dispatched. The signal is the card's sign-off field, read through its board: set means signed off, unset means the brief is not dispatched.
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> - ADR-104 — The shell and its mock-up change together, and a corrected mock-up is a new version beside the prior — A change to the shell's behaviour is made in the built shell and in the mock-up in the same change, and the corrected mock-up is written as a new file, index.v6.html, beside the prior, the way v4 was kept beside v5. CONFORMANCE.md's reference line moves to the new file in that change and carries its path and sha256; DIRECTORY-006 pins no file and no hash, so DIRECTORY-037 is the first brief to pin one, and DIRECTORY-006 does not change. Rejected: fixing only the build, which lets the definition drift from what is built; and editing index.v5.html in place, which changes the file the conformance table was written against.
> **Checklist:**
> - C303 — .land/gates.sh runs the surface's test command as a leg that fails by name when npm is missing, the project setup's trees measure surface/identity by npm ci and npm test, and the design's gate array is its parent's with the one surface leg appended and nothing else changed.
> - C304 — One route table names the 14 rail screens and the 30 hash-routed tabs with a built column, and the tests fail by name on a built row the shell does not serve.
> - C305 — The rail toggles by its button and by [, kept under iam.labels across a reload, and the dock side switches by the Configuration Layout segment, by the palette act and by \, kept under iam.dock, with the layout flipped.
> - C306 — The palette Go to entries and the fourteen g go-to letters reach every rail screen.
> - C307 — Every element with a click action is a button or a link, or has tabindex 0 and dispatches one click on Enter and Space, palette rows included, with no caller under surface/identity/src/features changed.
> - C308 — j and k move focus with the cursor without replacing the screen, and Enter opens the focused row.
> - C309 — The help overlay places one numbered mark per explained element and states the count, swallows its dismissing click, and returns focus on Escape to the element that had it.
> - C310 — A Tab walk over every built route, driven by @testing-library/user-event, reaches every element with a click action other than those in a closed layer (the closed palette's rows, walked instead with the palette open) and the two dismiss backdrops, and activates each with Enter and, for non-links, Space, with a non-zero count asserted.
> - C311 — index.v6.html sits beside index.v5.html and differs from it by the focus fix and the keyboard fix only, each proved against v6 and shown failing on v5.
> - C312 — CONFORMANCE.md line 3 names index.v6.html as the reference, with v5 as the prior, and pins index.v6.html by its path and sha256, the first pin of the mock-up.
> **Stories:**
> - S132 (Keyboard user, Working the identity screens without a mouse) — As a person working by keyboard, I want to reach and press every control on every screen without a mouse, so that nothing the screens offer is closed to me.
> - S133 (Responsible person, Sharing and returning to a place in the screens) — As a responsible person, I want every screen and tab to have its own address, so that a link I send opens exactly the view I was looking at.
> - S134 (Newcomer to the screens, Learning what a screen shows) — As a newcomer, I want the help overlay to number what is on screen and put me back where I was when I leave it, so that asking for help never costs me my place.
> - S135 (Identity line lead, Keeping the mock-up and the build in step) — As the identity line lead, I want the mock-up and the built shell to change together and the proof to run in the gate, so that the mock-up stays the definition the build is held to.

## Purpose

Pass conformance rows 9.1, 9.2 and 9.3 (docs/design/identity/CONFORMANCE.md:103-105): the identity surface's shell as the mock-up defines it, the rail with its kept open state, the dock on either side, the keys, the palette and hash deep links for every screen and tab, and a help overlay that numbers what is on screen, swallows the click that dismisses it and returns focus on Escape; and every clickable element reachable and usable by keyboard. The mock-up falls short of rows 9.2 and 9.3 in two places, so this brief closes both gaps in the built shell and in the mock-up, written as index.v6.html beside v5 (ADR-104), and proves every clause with the surface's test command run in the gate.

## Task

The shell is defined by docs/design/identity/mockup/index.v5.html and the built shell does not deviate from it: the rail (261-286) with its open state kept as a preference (42, 52, 285, 453-457), the dock side (39, 41, 456, 1092, 1128), the key registry (1111-1135, extended at 1177-1178), the palette (1084-1108), hash routes (922-932, replaced at 1546-1557) with a route for every rail screen (263-282) and every tab the mock-up gives a hash, the help overlay's numbered marks (1439-1447), its dismiss layer (1453-1455) and its Escape (1464). The surface this builds on is the one pull request 35 lands (head 6f57bf7): it already carries a shell under surface/identity/src/shell (Rail, Dock, Explain, Palette, keys, keyable, prefs, railItems) and a not-yet page for screens with no server. Read those files first; this brief adds the route table, the fixes and the proof, and builds nothing twice.

The two gaps, as the tree has them. Focus: v5's explainOff (1449) selects [data-act="explain"], or #screen, not the element that had focus, so opening the overlay with ? (1466) does not return focus. Keyboard: KEYABLE (1559-1562) already covers the rows, tr[data-href] and tr[data-act] in its selector at 1559, the network table's tr[data-act=node] at 1367 among them, but not the network map's node g.gn[data-act=node] (1362) nor the palette's div rows (1103); Enter at 1133 opens the cursor row rather than the focused one; and j and k (1132) move the cursor by re-rendering the screen without moving focus. Each fix is made in the built shell (R5, R6, with R7 proving the focus return the built shell already has) and in the mock-up (R9), and each has its acceptance line in both.

In: the gate leg, the project setup's surface tree and the one surface leg appended to the design's gate array (R1); the route table and its checks (R2); the proof of the rail and dock side (R3) and of the palette and go-to keys (R4); Enter and Space on every click action, palette rows included (R5); the cursor moving focus (R6); the overlay's count, dismissal and focus return (R7); the Tab walk (R8); index.v6.html (R9); and naming v6 as the reference with its path and sha256 on CONFORMANCE.md's line 3 (R10); DIRECTORY-006 pins no file and no hash (ADR-104), so this is the first pin of the mock-up, and DIRECTORY-006 does not change. Out: the contents of every screen, which do not change; building any screen the route table marks not yet; the assistant's composer, whose place in the dock holds a not-yet note naming row 9.4 as proposed, as pull request 35 has it; the Brief cells of CONFORMANCE.md rows 9.1 to 9.3; and a digit key for the file's eighth tab.

The route table's built column is set from what main draws when this card lands. At 6f57bf7 that is the You, People and agents, Access and Configuration screens, the eight file tabs, the three Access modes and Configuration's Layout section. Configuration keeps rendering surface/identity/src/features/notyet/Settings.tsx: its row is marked built with a note that only Layout is built, and its other six sections are marked not yet, with the same note, because Settings.tsx shows them as not built yet inside the screen; their route is left as it stands. IF main draws a further screen or tab by the time this builds, the build stops and names it before marking it (CN9).

Tests run under the surface's own vitest and jsdom configuration. jsdom lays nothing out, so the overlay test gives elements a box through a getBoundingClientRect stub and the dock-side test reads the stylesheet rule the dock-right class selects; the independence these tests claim is of the table and the mock-up against the router and the shell, on one toolchain, not of browser platform. Heavy runs, the full gate among them, go to the build venue. No file outside the design's structure array is created. Every path is relative to the repository root (CN3).

The gate. docs/design/project.json's trees and the gate array of docs/design/directory/design.json differ on main, and this brief does not reconcile them; the reconciliation, adding the docs tree and the place:here requirements to project.json's trees, is a separate card the lead has taken. The commit that carries this brief does not touch that gate array: it stays byte for byte as lys main a426b9a5bfa47abc935497b2caddee800b5d72f0 has it, and the design's gate is not copied from project.json's trees, because that copy would remove the docs tree and the place:here requirements. The build leaves the gate array byte for byte as its parent commit has it, whatever that parent holds, and appends the one surface leg R1 names; nothing the parent has is removed. Pull request 35 at 6f57bf7 already drops the docs tree (render, validate, coverage) and every place:here requirement from docs/design/directory/design.json's gate, so after it lands the parent's gate is the one tree '.' and this brief appends to it; that drop is recorded as a finding for the lead, not as this card's work, and this brief does not restore the docs tree.

Ids and base. The brief was filed against lys main fa3dd5311e98d1a751c770319e1cfe2570718c76 and is written against lys main a426b9a5bfa47abc935497b2caddee800b5d72f0; the four commits between them change lys-log-store only and touch nothing this brief reads, and every line number and file fact here is read at a426b9a5 and, for the surface, at 6f57bf7. DIRECTORY-037, RM-064 and ADR-104 are kept: the heads that also hold them as ids, 115ae18d, 2132984f, 8019110 and ddee159 and their draft heads, are this card's own earlier drafts, and the heads of other cards that name them (brief/directory/09884e04, brief/directory/504746ca, draft/directory/b0d75195) name them in prose only and take no id from them, and a card's own earlier heads take no id from it; an id is free when no other card's branch holds it. Every other open branch was re-read with git ls-remote immediately before writing, and none holds DIRECTORY-037, RM-064, ADR-104, C303 to C312 or S132 to S135.

Measuring the tests. Each test file this brief creates holds exactly the tests its requirement's acceptance lines name by title, and no others: routes 4, rail 5, goto 3, keyable 6, cursor 4, overlay 5, walk 5, mockup 7. A line that runs one test with -t reads vitest's JSON report, written to a temporary file by --reporter=json --outputFile, and prints its passed, failed and skipped counts, skipped counting the file's other tests.

Findings. The Blocked by line of this brief's rendered page joins its entries with a comma and a space because scripts/design/render-brief.py line 235 joins blocked_by that way; the fault is carded as DIRECTORY-039 under RM-072 and is not fixed here.

## Requirements

### R1: Run the surface's test command as a leg of the repository gate, the project setup and the design's gate

THE SYSTEM SHALL add to .land/gates.sh one leg, surface_leg, that runs npm ci and then npm test, both with surface/identity as the package directory, and counts the leg's exit status toward the gate's as every other leg's is counted, so the surface's test command runs on every landing. Beside it, THE SYSTEM SHALL add to the trees of docs/design/project.json one tree, surface/identity, whose measured legs are npm ci and then npm test, each with exit 0 expected, so a card_build round measures the surface as the landing does. THE SYSTEM SHALL append one leg, and only one, to the legs of the tree '.' in the gate array of docs/design/directory/design.json: the surface leg, named surface, whose command npm --prefix surface/identity install-ci-test runs npm ci and then npm test in surface/identity, requiring tool:node and tool:npm, every round. The gate array SHALL otherwise be byte for byte what the build's parent commit has, whatever that parent holds: no tree, leg or requirement SHALL be removed, reordered or changed. docs/design/project.json's trees and design.json's gate differ on main, and this requirement SHALL NOT reconcile them; a rebase onto a moved base carries whatever the base's two files hold and changes nothing more in them than the tree and the leg named here. The legs need node on the build venue, checked by node --version printing a version; a venue without node is a blocker to fix on the venue and SHALL NOT be a reason to leave the surface unmeasured. IF npm is not on the PATH, THEN THE SYSTEM SHALL fail surface_leg with a line beginning surface_npm_missing: and SHALL NOT skip it. The leg SHALL NOT be scoped away by a changed-path filter. This is the gate the tests of R2 to R9 run in; they are written in surface/identity/tests, where the surface's vitest configuration already looks. R1 requires two tools on the build venue, node and npm, and its acceptance is checked every round: the cadence of the design's surface leg and of both legs of the project setup's surface/identity tree is round.

**Acceptance:**
- grep -c '^leg surface_leg$' .land/gates.sh prints 1, and git diff --no-ext-diff <base> -- .land/gates.sh | grep -c '^-[^-]', where <base> is the commit the build started from, prints 0.
- Against surface/identity/tests/shell.test.tsx as it stands at 6f57bf7: with the assertion expect(localStorage.getItem('iam.labels')).toBe('labels') in the test titled 'opens and collapses the rail with [ and keeps the choice' changed to expect 'icons', f=$(mktemp); sh .land/gates.sh > "$f" 2>&1; echo $?; grep -xF -- '--- status 1: surface_leg ---' "$f"; grep -F 'FAIL' "$f" | grep -cF 'opens and collapses the rail with [ and keeps the choice' prints 1, then '--- status 1: surface_leg ---', then 1. With the assertion restored, sh .land/gates.sh 2>&1 | grep -xF -- '--- status 0: surface_leg ---' prints '--- status 0: surface_leg ---', and git diff --no-ext-diff <base> -- surface/identity/tests/shell.test.tsx prints nothing.
- Input: a PATH that holds no npm on any venue, built as d=$(mktemp -d) && ln -s "$(command -v sh)" "$d/sh", a temporary directory holding only a link to sh (echo, cd, [ and command are sh builtins, and gates.sh's other legs fail by name without their tools, which this line does not read). env PATH="$d" "$d/sh" -c 'command -v npm' prints nothing, and env PATH="$d" "$d/sh" .land/gates.sh 2>&1 | grep -E '^surface_npm_missing:|^--- status [01]: surface_leg ---$', with grep found on the caller's own PATH, prints two lines: the first beginning 'surface_npm_missing:', the second '--- status 1: surface_leg ---'.
- On the build venue, node --version prints one line matching ^v[0-9]+\.[0-9]+\.[0-9]+$, and (cd surface/identity && npm ci && npm test) >/dev/null 2>&1; echo $? prints 0.
- python3 -c "import json,subprocess; b=json.loads(subprocess.check_output(['git','show','<base>:docs/design/project.json']))['trees']; t=json.load(open('docs/design/project.json'))['trees']; n=t[-1]; print(len(t)==len(b)+1, json.dumps(t[:-1])==json.dumps(b), n['tree'], [l['command'] for l in n['legs']], [l['cadence'] for l in n['legs']], [l['requires'] for l in n['legs']])" prints: True True surface/identity ['npm ci', 'npm test'] ['round', 'round'] [['tool:node', 'tool:npm'], ['tool:node', 'tool:npm']]
- python3 -c "import json,subprocess; b=json.loads(subprocess.check_output(['git','show','<base>:docs/design/directory/design.json']))['gate']; h=json.load(open('docs/design/directory/design.json'))['gate']; i=[t['tree'] for t in b].index('.'); leg=h[i]['legs'][-1]; b[i]['legs'].append(leg); print(json.dumps(h)==json.dumps(b), json.dumps(leg))" prints: True {"name": "surface", "command": "npm --prefix surface/identity install-ci-test", "requires": ["tool:node", "tool:npm"], "cadence": "round"} — the head's gate array is the parent's with that one leg appended to the tree '.' and nothing else changed. python3 scripts/design/validate.py docs/design/project.json >/dev/null; echo $? prints 0, and then python3 scripts/design/validate.py docs/design/directory >/dev/null; echo $? prints 0: two commands, one path each, since validate.py takes one path.

**Files:**
- modify: .land/gates.sh
- modify: docs/design/project.json
- modify: docs/design/directory/design.json

**Checklist:**
- C303 — .land/gates.sh runs the surface's test command as a leg that fails by name when npm is missing, the project setup's trees measure surface/identity by npm ci and npm test, and the design's gate array is its parent's with the one surface leg appended and nothing else changed.

**Stories:**
- S135 (Identity line lead, Keeping the mock-up and the build in step) — As the identity line lead, I want the mock-up and the built shell to change together and the proof to run in the gate, so that the mock-up stays the definition the build is held to.

### R2: Keep every screen and tab route in one table the tests read

THE SYSTEM SHALL create surface/identity/src/shell/routeTable.ts holding one exported, typed table with one row per screen and per tab the mock-up routes by hash: the fourteen rail screens (index.v5.html:263-282, Configuration at 282) and the tabs the mock-up gives a hash, which are the identity file's eight tabs (553, with certificate spliced in at 1623), Access's three modes (756, 759-762), Requests' two (790), Model's two (868), Configuration's seven sections (901, 918), Roles' five (1215, 1254) and Secrets' three (1291, 1305, 1537). Each row names its screen, its tab (none for a screen row), its hash pattern as the mock-up spells it, a built column saying whether main draws that screen or tab when this card lands, and a note, empty unless the row needs one. A segmented filter that sets no hash (the people kind filter at 533, the secrets scope filter at 1302, the graph show toggles at 1348) SHALL NOT be a row. WHEN the surface's test command runs, THE SYSTEM SHALL follow the hash of every row marked built and fail, naming the row's screen and tab, IF the route renders the not-yet page or does not mark that screen and tab as current. IF the rail or a built screen's tab navigation draws a hash link that no row's pattern matches, THEN THE SYSTEM SHALL fail naming the hash. A row marked not yet SHALL NOT fail the run. The table SHALL NOT be read by routes.tsx and SHALL NOT change which component any route renders: it is the list the router is checked against, not a second router.

**Acceptance:**
- The table's (screen, tab) pairs equal exactly these 44: the 14 screen rows me, people, roles, resources, access, graph, requests, reviews, secrets, connections, network, sessions, model, settings; and the 30 tab rows file/profile, file/access, file/provisioning, file/memory, file/credentials, file/sessions, file/certificate, file/record, access/can, access/reach, access/who, requests/access, requests/acts, model/types, model/try, settings/layout, settings/signin, settings/directory, settings/permissions, settings/secrets, settings/runtimes, settings/storage, roles/job, roles/profile, roles/access, roles/versions, roles/preview, secrets/store, secrets/virtual, secrets/leases. The suite proves it in its own test, titled 'names exactly the 44 screen and tab rows'. Run in surface/identity: f=$(mktemp); npm test -- tests/routes.test.tsx -t 'names exactly the 44 screen and tab rows' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 3': 1 passed, 0 failed and 3 skipped, the file's other 3 tests.
- Against the tree as pull request 35 leaves it at 6f57bf7, exactly 16 rows are marked built: the screens me, people, access and settings, the eight file tabs, the three access modes and settings/layout; the other 28 are marked not yet, the six Configuration sections other than layout among them, and the settings row and those six rows each carry a note saying only Layout is built and the other six sections show not yet inside the screen. The suite proves the count in its own test, titled 'marks exactly 16 rows built'. Run in surface/identity: f=$(mktemp); npm test -- tests/routes.test.tsx -t 'marks exactly 16 rows built' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 3': 1 passed, 0 failed and 3 skipped, the file's other 3 tests.
- For each built row, with :id taken as the fixture's first person, following its hash leaves the rail link whose data-nav is the row's screen (people for a file tab) carrying class on, leaves the tab link whose href equals the hash carrying class on for a tab row, renders as the text of #screen's first h1 the screen's own heading, 'Configuration' for the settings rows, 'Access' for the access rows, 'People and agents' for people and the display name of the fixture person the route shows for me and the file tabs, and renders no element containing the text 'Nothing is shown here until its server answers.'; the test counts 16 rows followed and asserts 16. The suite proves it in its own test, titled 'follows every built row'. Run in surface/identity: f=$(mktemp); npm test -- tests/routes.test.tsx -t 'follows every built row' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 3': 1 passed, 0 failed and 3 skipped, the file's other 3 tests.
- With surface/identity/src/routes.tsx's /settings/:sec? route changed to render Access in place of Settings and the route table left unchanged, so exactly 16 rows are still marked built, run in surface/identity: f=$(mktemp); npm test -- tests/routes.test.tsx --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); a=[x for t in r['testResults'] for x in t['assertionResults']]; print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests']); print([x['title'] for x in a if x['status']=='failed'], any('settings' in m for x in a if x['status']=='failed' for m in x['failureMessages']), [x['status'] for x in a if x['title']=='marks exactly 16 rows built'])" "$f" prints 1, then '3 1 0', then "['follows every built row'] True ['passed']": the one failed test is the built-row follow test, its message names settings, and the count test passes in the same run. The full suite is not the measurement of this injection, and surface/identity/tests/shell.test.tsx does not change. With routes.tsx restored, the same commands print 0, then '4 0 0', then "[] False ['passed']".
- Every hash link the rail and the built screens' tab navigations draw is matched by a row's pattern, counted and asserted greater than 0, and no row's pattern contains kind, scope or show. The suite proves it in its own test, titled 'matches every drawn hash link to a row'. Run in surface/identity: f=$(mktemp); npm test -- tests/routes.test.tsx -t 'matches every drawn hash link to a row' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 3': 1 passed, 0 failed and 3 skipped, the file's other 3 tests.

**Files:**
- create: surface/identity/src/shell/routeTable.ts
- create: surface/identity/tests/routes.test.tsx

**Checklist:**
- C304 — One route table names the 14 rail screens and the 30 hash-routed tabs with a built column, and the tests fail by name on a built row the shell does not serve.

**Stories:**
- S133 (Responsible person, Sharing and returning to a place in the screens) — As a responsible person, I want every screen and tab to have its own address, so that a link I send opens exactly the view I was looking at.

### R3: Prove the rail and the dock side as the mock-up has them

WHEN the rail's labels button (#railBtn) is pressed, or the [ key is pressed outside a text field, THE SYSTEM SHALL toggle the rail between icons and labels and keep the choice under the preference key iam.labels, as the mock-up does (index.v5.html:42, 52, 285, 453-457, 1127). WHEN the Configuration screen's Layout segment Left or Right is pressed (index.v5.html:908), the palette's act Toggle dock side is chosen (index.v5.html:1092), or the \ key is pressed outside a text field (index.v5.html:1128), THE SYSTEM SHALL set or switch the dock side and keep it under iam.dock, and the shell SHALL carry the dock-right class exactly while the side is right, the class the stylesheet flips the layout on (index.v5.html:39, 41, 456). These three are the dock-side controls this card proves; the Layout segment is screen content that surface/identity/src/features/notyet/Settings.tsx already draws, and it SHALL NOT be changed. WHILE a text field has focus, [ and \ SHALL NOT toggle anything. Neither preference SHALL be stored under any other key.

**Acceptance:**
- With iam.labels unset, a click on #railBtn gives #rail the class open and sets iam.labels to 'labels'; a second click removes the class and sets 'icons'. The suite proves it in its own test, titled 'rail button toggles the labels'. Run in surface/identity: f=$(mktemp); npm test -- tests/rail.test.tsx -t 'rail button toggles the labels' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- Pressing [ with focus on document.body toggles #rail's class open the same way; pressing [ with focus in #palIn leaves the class and iam.labels unchanged. The suite proves it in its own test, titled 'bracket toggles the rail outside a text field'. Run in surface/identity: f=$(mktemp); npm test -- tests/rail.test.tsx -t 'bracket toggles the rail outside a text field' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- After [ sets iam.labels to 'labels', unmounting the app and mounting it again over the same storage renders #rail with class open; after 'icons', without it. The suite proves it in its own test, titled 'rail choice survives a remount'. Run in surface/identity: f=$(mktemp); npm test -- tests/rail.test.tsx -t 'rail choice survives a remount' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- With iam.dock unset, choosing Toggle dock side in the palette gives #shell the class dock-right and sets iam.dock to 'right'; pressing \ on document.body then removes the class and sets iam.dock to 'left'. The suite proves it in its own test, titled 'palette act and backslash switch the dock side'. Run in surface/identity: f=$(mktemp); npm test -- tests/rail.test.tsx -t 'palette act and backslash switch the dock side' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- With the Settings component of surface/identity/src/features/notyet/Settings.tsx mounted by the test inside the shell under the test's own route /settings/:sec? at #/settings/layout, not through routes.tsx, and iam.dock unset, a click on the button [data-dock=right] gives #shell the class dock-right and sets iam.dock to 'right'; a click on [data-dock=left] removes the class and sets iam.dock to 'left'. The suite proves it in its own test, titled 'Layout segment sets the dock side'. Run in surface/identity: f=$(mktemp); npm test -- tests/rail.test.tsx -t 'Layout segment sets the dock side' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- grep -cF '.shell.dock-right { flex-direction: row-reverse; }' surface/identity/src/styles/rail.css prints 1, the rule the mock-up has at line 39 that flips the layout.

**Files:**
- create: surface/identity/tests/rail.test.tsx

**Checklist:**
- C305 — The rail toggles by its button and by [, kept under iam.labels across a reload, and the dock side switches by the Configuration Layout segment, by the palette act and by \, kept under iam.dock, with the layout flipped.

**Stories:**
- S132 (Keyboard user, Working the identity screens without a mouse) — As a person working by keyboard, I want to reach and press every control on every screen without a mouse, so that nothing the screens offer is closed to me.

### R4: Prove the palette and the go-to keys reach every screen

WHEN a person chooses a Go to entry in the palette, THE SYSTEM SHALL route to that entry's hash; WHEN a person presses g and then, within the mock-up's 900 ms, one of the fourteen go-to letters p o r a q w v n x m s h t u outside a text field, THE SYSTEM SHALL route to that letter's screen (index.v5.html:1085, 1094, 1112, 1125-1126, 1177-1178). Every one of the fourteen rail screens SHALL be reached by at least one palette Go to entry and by exactly one go-to letter. A g followed by a letter that names no screen SHALL NOT route anywhere.

**Acceptance:**
- For each of the 14 screens, choosing a palette Go to entry leaves the rail link with that screen's data-nav carrying class on (Credentials, whose hash is #/vault, counts for secrets); the test asserts 14 screens reached. The suite proves it in its own test, titled 'palette Go to reaches all 14 screens'. Run in surface/identity: f=$(mktemp); npm test -- tests/goto.test.tsx -t 'palette Go to reaches all 14 screens' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 2': 1 passed, 0 failed and 2 skipped, the file's other 2 tests.
- For each of the 14 letters, pressing g and then the letter on document.body leaves the rail link of that letter's screen carrying class on; the 14 letters reach 14 distinct screens, asserted as 14. The suite proves it in its own test, titled 'g letters reach all 14 screens'. Run in surface/identity: f=$(mktemp); npm test -- tests/goto.test.tsx -t 'g letters reach all 14 screens' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 2': 1 passed, 0 failed and 2 skipped, the file's other 2 tests.
- Pressing g and then z on document.body leaves location.hash unchanged. The suite proves it in its own test, titled 'g then an unmapped letter goes nowhere'. Run in surface/identity: f=$(mktemp); npm test -- tests/goto.test.tsx -t 'g then an unmapped letter goes nowhere' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 2': 1 passed, 0 failed and 2 skipped, the file's other 2 tests.

**Files:**
- create: surface/identity/tests/goto.test.tsx

**Checklist:**
- C306 — The palette Go to entries and the fourteen g go-to letters reach every rail screen.

**Stories:**
- S132 (Keyboard user, Working the identity screens without a mouse) — As a person working by keyboard, I want to reach and press every control on every screen without a mouse, so that nothing the screens offer is closed to me.

### R5: Make every click action answer Enter and Space, palette rows included

THE SYSTEM SHALL make every element that acts on a click a button or a link, or give it tabindex 0 and role button and make Enter and Space dispatch one click on that element, as the mock-up's KEYABLE rule does (index.v5.html:1559-1562). The shared helper surface/identity/src/shell/keyable.ts SHALL keep its signature keyable(activate): it SHALL return the element's click handler as onClick, set to activate, beside tabIndex 0, role button and a key handler that, on Enter and Space, dispatches one click on the element itself, so activate runs once through that click; the key handler SHALL NOT call activate directly. Every caller that passes activate spreads keyable's props after its own onClick with the same action, so keyable's onClick is the one the element carries, and no caller SHALL change: the calls at surface/identity/src/features/people/People.tsx, features/me/You.tsx, features/access/Access.tsx, features/grants/Delegate.tsx and shell/Dock.tsx stay as they are, and activate is not left a dead parameter. WHILE the palette is open, THE SYSTEM SHALL make each palette row (index.v5.html:1103), each a div.it carrying data-n inside #palette in surface/identity/src/shell/Palette.tsx, reachable by Tab from the palette's input, and WHEN Enter or Space is pressed on a focused row, SHALL choose that row and SHALL NOT choose the row the arrow selection marks. The arrow keys and Enter in the palette's input SHALL keep working as they do. WHILE the palette is closed, its rows stay rendered, #palette SHALL keep aria-hidden true and #palIn tabindex -1 as Palette.tsx has them on the tree this builds on, and the rows SHALL NOT be reachable by Tab.

**Acceptance:**
- Rendered with an action that counts its calls, keyable(action)'s props on a div give the div tabIndex 0 and role button; Enter on the focused div counts one click event on the div and one call of the action, and Space counts the same; a mouse click counts one click and one call. The suite proves it in its own test, titled 'keyable answers Enter Space and click once each'. Run in surface/identity: f=$(mktemp); npm test -- tests/keyable.test.tsx -t 'keyable answers Enter Space and click once each' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 5': 1 passed, 0 failed and 5 skipped, the file's other 5 tests.
- Enter on a focused People row dispatches exactly one click on that row, counted by a click listener on the row, and routes to that row's #/file/<id>; Space does the same. The suite proves it in its own test, titled 'Enter and Space on a People row open its file'. Run in surface/identity: f=$(mktemp); npm test -- tests/keyable.test.tsx -t 'Enter and Space on a People row open its file' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 5': 1 passed, 0 failed and 5 skipped, the file's other 5 tests.
- git diff --no-ext-diff --stat <base> -- surface/identity/src/features surface/identity/src/shell/Dock.tsx prints nothing.
- With the palette open and at least two rows listed, ArrowDown in #palIn marks the second row selected, Tab then focuses the first row, and Enter routes to the first row's destination, asserted against its hash. The suite proves it in its own test, titled 'Enter on a focused palette row chooses that row'. Run in surface/identity: f=$(mktemp); npm test -- tests/keyable.test.tsx -t 'Enter on a focused palette row chooses that row' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 5': 1 passed, 0 failed and 5 skipped, the file's other 5 tests.
- Space on a focused palette row chooses it the same way, and the palette closes. The suite proves it in its own test, titled 'Space on a focused palette row chooses it'. Run in surface/identity: f=$(mktemp); npm test -- tests/keyable.test.tsx -t 'Space on a focused palette row chooses it' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 5': 1 passed, 0 failed and 5 skipped, the file's other 5 tests.
- ArrowDown twice then Enter in #palIn still routes to the third row's destination. The suite proves it in its own test, titled 'arrows and Enter in the palette input still choose'. Run in surface/identity: f=$(mktemp); npm test -- tests/keyable.test.tsx -t 'arrows and Enter in the palette input still choose' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 5': 1 passed, 0 failed and 5 skipped, the file's other 5 tests.
- With the palette closed, #palette carries aria-hidden true, #palIn has tabIndex -1, and no #palette .it[data-n] row has tabIndex 0. The suite proves it in its own test, titled 'closed palette rows stay out of the Tab order'. Run in surface/identity: f=$(mktemp); npm test -- tests/keyable.test.tsx -t 'closed palette rows stay out of the Tab order' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 5': 1 passed, 0 failed and 5 skipped, the file's other 5 tests.

**Files:**
- create: surface/identity/tests/keyable.test.tsx
- modify: surface/identity/src/shell/keyable.ts
- modify: surface/identity/src/shell/Palette.tsx

**Checklist:**
- C307 — Every element with a click action is a button or a link, or has tabindex 0 and dispatches one click on Enter and Space, palette rows included, with no caller under surface/identity/src/features changed.

**Stories:**
- S132 (Keyboard user, Working the identity screens without a mouse) — As a person working by keyboard, I want to reach and press every control on every screen without a mouse, so that nothing the screens offer is closed to me.

### R6: Move focus with the keyboard cursor and open the focused row on Enter

WHEN j or k is pressed outside a text field on a screen with rows, THE SYSTEM SHALL move the cursor one row and move focus to the row now under the cursor, and SHALL NOT replace the screen's elements to do it (index.v5.html:1132). The rows are the elements the mock-up's KEYABLE selector names as rows, tr[data-href] and tr[data-act] (index.v5.html:1559), the network table's tr[data-act=node] among them (index.v5.html:1367). WHEN Enter is pressed while a row has focus, THE SYSTEM SHALL open that row and SHALL NOT open the cursor row in its place (index.v5.html:1133). WHEN Enter is pressed while no row has focus, THE SYSTEM SHALL open the cursor row, as the mock-up does.

**Acceptance:**
- On the People screen with at least three rows and focus on document.body, pressing j twice leaves document.activeElement the third row, which carries class cursor, and the first element child of #screen is the same node as before the presses. The suite proves it in its own test, titled 'j moves focus with the cursor without re-rendering'. Run in surface/identity: f=$(mktemp); npm test -- tests/cursor.test.tsx -t 'j moves focus with the cursor without re-rendering' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 3': 1 passed, 0 failed and 3 skipped, the file's other 3 tests.
- After j twice, pressing k once leaves document.activeElement the second row, carrying class cursor. The suite proves it in its own test, titled 'k moves focus back one row'. Run in surface/identity: f=$(mktemp); npm test -- tests/cursor.test.tsx -t 'k moves focus back one row' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 3': 1 passed, 0 failed and 3 skipped, the file's other 3 tests.
- With focus moved to the second row and Enter pressed on it, location.hash is that row's #/file/<id>, not the first row's. The suite proves it in its own test, titled 'Enter opens the focused row'. Run in surface/identity: f=$(mktemp); npm test -- tests/cursor.test.tsx -t 'Enter opens the focused row' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 3': 1 passed, 0 failed and 3 skipped, the file's other 3 tests.
- With focus on document.body and the cursor on the first row, Enter routes to the first row's #/file/<id>. The suite proves it in its own test, titled 'Enter with no row focused opens the cursor row'. Run in surface/identity: f=$(mktemp); npm test -- tests/cursor.test.tsx -t 'Enter with no row focused opens the cursor row' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 3': 1 passed, 0 failed and 3 skipped, the file's other 3 tests.

**Files:**
- create: surface/identity/tests/cursor.test.tsx
- modify: surface/identity/src/shell/keys.ts

**Checklist:**
- C308 — j and k move focus with the cursor without replacing the screen, and Enter opens the focused row.

**Stories:**
- S132 (Keyboard user, Working the identity screens without a mouse) — As a person working by keyboard, I want to reach and press every control on every screen without a mouse, so that nothing the screens offer is closed to me.

### R7: Prove the help overlay counts its marks, swallows its dismissal and returns focus

WHEN the help overlay opens, by ? outside a text field or by the help dock's Explain this screen button, THE SYSTEM SHALL place one numbered mark, 1 to N, for each help concept with an element laid out on screen and state N in its bar as 'N things explained' (index.v5.html:1399, 1439-1447). WHEN the dismiss layer or the Done button is clicked, THE SYSTEM SHALL close the overlay and SHALL NOT let the click reach any handler underneath (index.v5.html:1453-1455). WHEN Escape is pressed while the overlay is open, THE SYSTEM SHALL close it and return focus to the element that had focus when it opened (index.v5.html:1464); IF that element is no longer in the document, THEN THE SYSTEM SHALL focus the main screen, #screen. Focus SHALL NOT be sent to the Explain button unless the Explain button had it.

**Acceptance:**
- On each built route, with every element given a laid-out box, the overlay holds exactly as many .xm marks as there are concepts whose selector matches an element outside the overlay, their text reads 1 to N in order, and the bar's text starts with 'N things explained'; across the built routes at least one N is greater than 0, asserted. The suite proves it in its own test, titled 'places one numbered mark per explained element'. Run in surface/identity: f=$(mktemp); npm test -- tests/overlay.test.tsx -t 'places one numbered mark per explained element' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- With a click recorder added to document in the bubble phase, a click on [data-xabsorb] closes the overlay and the recorder counts 0; a click on [data-xoff] closes it and counts 0; a control click on #screen with the overlay closed counts 1. The suite proves it in its own test, titled 'dismissal reaches nothing underneath'. Run in surface/identity: f=$(mktemp); npm test -- tests/overlay.test.tsx -t 'dismissal reaches nothing underneath' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- With focus on the third People row, pressing ? and then Escape removes #xlayer and leaves document.activeElement that row. The suite proves it in its own test, titled 'Escape returns focus to the focused row'. Run in surface/identity: f=$(mktemp); npm test -- tests/overlay.test.tsx -t 'Escape returns focus to the focused row' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- With the help dock open and its Explain this screen button clicked, Escape leaves document.activeElement that button. The suite proves it in its own test, titled 'Escape returns focus to the Explain button'. Run in surface/identity: f=$(mktemp); npm test -- tests/overlay.test.tsx -t 'Escape returns focus to the Explain button' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- With focus on a People row that the test removes from the document while the overlay is open, Escape leaves document.activeElement #screen. The suite proves it in its own test, titled 'Escape focuses the screen when the opener is gone'. Run in surface/identity: f=$(mktemp); npm test -- tests/overlay.test.tsx -t 'Escape focuses the screen when the opener is gone' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.

**Files:**
- create: surface/identity/tests/overlay.test.tsx

**Checklist:**
- C309 — The help overlay places one numbered mark per explained element and states the count, swallows its dismissing click, and returns focus on Escape to the element that had it.

**Stories:**
- S134 (Newcomer to the screens, Learning what a screen shows) — As a newcomer, I want the help overlay to number what is on screen and put me back where I was when I leave it, so that asking for help never costs me my place.

### R8: Walk every built screen with Tab and activate every click action from the keyboard

WHEN the surface's test command runs, THE SYSTEM SHALL, for every route-table row marked built, render the row's route and list every element with a click action there by one listing rule: every a with an href, every enabled button, and every element whose rendered props carry an onClick handler, that is not inside an aria-hidden layer and is not one of the two dismiss backdrops. An aria-hidden layer is an element carrying aria-hidden true; on the tree this builds on the one such layer is the closed palette, #palette in surface/identity/src/shell/Palette.tsx, which Shell.tsx always renders and whose rows (#palette .it[data-n]) carry an onClick handler while it is closed; R5 keeps them out of the Tab order until the palette opens. The two dismiss backdrops are #scrim (surface/identity/src/shell/Shell.tsx) and the help overlay's dismiss layer [data-xabsorb] (surface/identity/src/shell/Explain.tsx), which is rendered only while the overlay is open. Those two are not click actions a person takes on an element: each is a layer behind what is open that closes it when the pointer lands outside it, and its keyboard equivalent is Escape, which closes the palette and the drawer the scrim sits behind and the overlay the dismiss layer belongs to; they SHALL NOT be listed and SHALL NOT be made focusable, and Shell.tsx SHALL NOT change. The walk SHALL move focus through the page with the Tab key alone and fail naming the route and the element IF any listed element is never focused. It SHALL then activate each listed element with Enter, and each listed element that is not a link with Space, restoring the walk's open layers before each activation, and fail naming the element IF a click on it is not recorded exactly once. The walk SHALL run once more with the help dock open, once more on the People route with the help overlay open, and once more on the People route with the help overlay open and the palette then opened by Command-K. The same listing rule applies in the palette-open walk: the rail and the page are not aria-hidden while the palette is open and the mock-up does not trap focus in it, so that walk lists the page and the open palette's rows, and the shell SHALL NOT add a focus trap the mock-up lacks. In that walk the palette's input #palIn is also listed as the palette's entry point, since Enter in it chooses the selected row; it is reached by Tab, and Enter in it with the first row selected reaches that row's destination, with no Space press, as it is a text input. The words' 'with none left out' is read with this rule: in the palette-open walk the only elements with a click action the walk does not list are #scrim and the help dismiss layer, count two, and in the closed-palette walks the only others are the closed palette's rows, which the palette-open walk lists. It SHALL NOT pass on a page where it listed no element. The Tab and key presses SHALL come from @testing-library/user-event 14.6.7, with its required peer @testing-library/dom 10.4.2, both added as exact-version development dependencies of surface/identity, so the keys are pressed the way a browser presses them rather than by a helper written for this test.

**Acceptance:**
- The walk runs 19 times, once per built row, once with the help dock open, once on the People route with the help overlay open and once on the People route with the help overlay and the palette open, and for each asserts the count of listed elements focused by Tab equals the count listed and that the count listed is greater than 0; the count listed in the palette-open walk equals the count listed in the closed-palette walk it starts from, the overlay-open People walk, plus 1 for #palIn plus the number of #palette .it[data-n] rows rendered, which is at least 19 (3 Acts rows and 16 Go to rows) with no directory loaded. The suite proves it in its own test, titled 'Tab reaches every listed element on 19 walks'. Run in surface/identity: f=$(mktemp); npm test -- tests/walk.test.tsx -t 'Tab reaches every listed element on 19 walks' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- Every listed element other than #palIn records exactly one click on Enter, every listed element that is not a link and is not #palIn records exactly one click on Space, and Enter in #palIn with the first #palette .it[data-n] row selected leaves location.hash equal to that row's destination. The suite proves it in its own test, titled 'Enter and Space activate every listed element once'. Run in surface/identity: f=$(mktemp); npm test -- tests/walk.test.tsx -t 'Enter and Space activate every listed element once' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- On every walk with the palette closed and the overlay closed, the elements carrying an onClick handler that the listing rule removed are exactly #scrim and every #palette .it[data-n] row, asserted by selector and by count: 1 plus the number of #palette .it[data-n] rows rendered; in the overlay-open People walk they are exactly those plus the one [data-xabsorb], asserted as that count plus 1. In the palette-open walk the removed set is exactly #scrim and [data-xabsorb], asserted by selector and as count 2, and no #palette .it[data-n] row is in it. The suite proves it in its own test, titled 'removes only the backdrops and hidden rows'. Run in surface/identity: f=$(mktemp); npm test -- tests/walk.test.tsx -t 'removes only the backdrops and hidden rows' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- A div with an onClick handler and no tabindex, planted into the dock-open walk's page by the test's own control case, makes that walk fail naming the div, which the control test asserts, while the real walks pass. The suite proves it in its own test, titled 'a planted unreachable div fails the walk by name'. Run in surface/identity: f=$(mktemp); npm test -- tests/walk.test.tsx -t 'a planted unreachable div fails the walk by name' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- With the palette open, Escape closes it and #scrim loses the class open; with the drawer open, Escape removes the drawer's class open. The suite proves it in its own test, titled 'Escape closes the palette and the drawer'. Run in surface/identity: f=$(mktemp); npm test -- tests/walk.test.tsx -t 'Escape closes the palette and the drawer' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 4': 1 passed, 0 failed and 4 skipped, the file's other 4 tests.
- python3 -c "import json,subprocess; b=json.loads(subprocess.check_output(['git','show','<base>:surface/identity/package.json'])); h=json.load(open('surface/identity/package.json')); dv=h['devDependencies']; print(dv['@testing-library/user-event'], dv['@testing-library/dom'], {k:v for k,v in h.items() if k!='devDependencies'}=={k:v for k,v in b.items() if k!='devDependencies'}, sorted(set(dv)-set(b['devDependencies'])), all(b['devDependencies'][k]==dv[k] for k in b['devDependencies']))" prints: 14.6.7 10.4.2 True ['@testing-library/dom', '@testing-library/user-event'] True; and python3 -c "import json; p=json.load(open('surface/identity/package-lock.json'))['packages']; print(p['node_modules/@testing-library/user-event']['version'], p['node_modules/@testing-library/dom']['version'])" prints: 14.6.7 10.4.2

**Files:**
- create: surface/identity/tests/walk.test.tsx
- modify: surface/identity/package.json
- modify: surface/identity/package-lock.json

**Checklist:**
- C310 — A Tab walk over every built route, driven by @testing-library/user-event, reaches every element with a click action other than those in a closed layer (the closed palette's rows, walked instead with the palette open) and the two dismiss backdrops, and activates each with Enter and, for non-links, Space, with a non-zero count asserted.

**Stories:**
- S132 (Keyboard user, Working the identity screens without a mouse) — As a person working by keyboard, I want to reach and press every control on every screen without a mouse, so that nothing the screens offer is closed to me.

### R9: Write the mock-up's two fixes as index.v6.html beside v5

THE SYSTEM SHALL create docs/design/identity/mockup/index.v6.html as a copy of index.v5.html that differs from it by the two fixes only, and SHALL NOT change index.v5.html. The focus fix: explainOn SHALL record the element that has focus before it closes anything, and explainOff SHALL return focus to that element, or to #screen IF it is no longer in the document; explainOff SHALL NOT select [data-act="explain"] (v5 1439-1449). The keyboard fix: the network map's node g.gn[data-act=node] (v5 1362), any other data-act element the KEYABLE selector (v5 1559) does not cover, and the palette's rows .it[data-n] (v5 1103) SHALL be focusable, carry role button and dispatch one click on Enter and Space (v5 1559-1562); the rows KEYABLE already covers, tr[data-href] and tr[data-act] (v5 1559), the network table's tr[data-act=node] at v5 1367 among them, stay as they are; Enter on a focused palette row SHALL choose that row, not palSel (v5 1120); j and k SHALL move the cursor class and focus to the next row without calling route() (v5 1132); Enter on a focused row SHALL open that row, not the cursor row (v5 1133). No other behaviour, text or style SHALL change. surface/identity/tests/mockup.test.tsx SHALL load the mock-up into jsdom with its scripts running and prove each of six behaviours in its own case: the focus return, the network node answering the keyboard, palette rows reachable by Tab, Enter on a focused palette row, j and k moving focus without re-rendering, and Enter on a focused row. It SHALL run each case against index.v5.html and assert that it fails there, and no case SHALL depend on another fix to reach its own assertion on v5. WHERE a case measures a navigation, it SHALL record the newURL of every hashchange event and SHALL NOT judge by the final location.hash alone.

**Acceptance:**
- diff docs/design/identity/mockup/index.v5.html docs/design/identity/mockup/index.v6.html | grep -E '^[0-9]' | python3 -c "import sys,re; R=[(1097,1103),(1111,1135),(1438,1449),(1558,1562)]; h=sys.stdin.read().split(); print(len(h)>0, all(any(a<=int(n)<=b for a,b in R) for x in h for n in re.match(r'(\d+)(?:,(\d+))?',x).groups() if n))" prints: True True — every hunk header's v5 line numbers lie within 1097-1103, 1111-1135, 1438-1449 and 1558-1562; no range is required to hold a hunk. That v6 differs by the two fixes only is measured by this range rule together with cases 1 to 6, not by a count of hunks: the focus fix is case 1, and the keyboard fix's effects include the palette rows made keyable through the KEYABLE rule (cases 3 and 4) and case 4 landing on #/people from #/access.
- shasum -a 256 docs/design/identity/mockup/index.v5.html prints e67622e19f450e2335029f293f18b4757801f3245758a8b667521afa120ab91f  docs/design/identity/mockup/index.v5.html, as before the build.
- Case 1, focus return, in index.v6.html: with the third row of #/people focused, ? then Escape leaves document.activeElement that row. The suite proves it in its own test, titled 'case 1 focus return'. Run in surface/identity: f=$(mktemp); npm test -- tests/mockup.test.tsx -t 'case 1 focus return' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 6': 1 passed, 0 failed and 6 skipped, the file's other 6 tests.
- Case 2, network node, in index.v6.html on #/network: every g.gn[data-act=node] has tabIndex 0 and role button, and Enter on the first, focused, gives #drawer the class open, as a click on it does. The suite proves it in its own test, titled 'case 2 network node'. Run in surface/identity: f=$(mktemp); npm test -- tests/mockup.test.tsx -t 'case 2 network node' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 6': 1 passed, 0 failed and 6 skipped, the file's other 6 tests.
- Case 3, palette Tab, in index.v6.html: with the palette open, Tab from #palIn leaves document.activeElement the first .it[data-n] row. The suite proves it in its own test, titled 'case 3 palette Tab'. Run in surface/identity: f=$(mktemp); npm test -- tests/mockup.test.tsx -t 'case 3 palette Tab' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 6': 1 passed, 0 failed and 6 skipped, the file's other 6 tests.
- Case 4, palette Enter, in index.v6.html, starting on #/access, a route that is neither the first Go to row's destination (#/people) nor the second's (#/roles): with the palette open and ArrowDown pressed in #palIn until the second Go to row carries class sel, the test sets the first Go to row's tabIndex to -1 when it has no tabindex attribute, calls focus() on it and presses Enter; the recorded hashchange newURLs include one ending with '#/people', the first Go to row's destination, and none ends with '#/roles', the second Go to row's destination. The suite proves it in its own test, titled 'case 4 palette Enter'. Run in surface/identity: f=$(mktemp); npm test -- tests/mockup.test.tsx -t 'case 4 palette Enter' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 6': 1 passed, 0 failed and 6 skipped, the file's other 6 tests.
- Case 5, j and k, in index.v6.html on #/people with the first row focused: pressing j twice leaves document.activeElement the third row, that row carries class cursor, and the first element child of #screen is the same node as before the two presses. The suite proves it in its own test, titled 'case 5 j and k'. Run in surface/identity: f=$(mktemp); npm test -- tests/mockup.test.tsx -t 'case 5 j and k' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 6': 1 passed, 0 failed and 6 skipped, the file's other 6 tests.
- Case 6, Enter on a focused row, in index.v6.html on #/people with the cursor on the first row and no j or k pressed: focus() on the second row then Enter records hashchange newURLs of which exactly one ends with the second row's data-href and none ends with the first row's data-href. The suite proves it in its own test, titled 'case 6 Enter on a focused row'. Run in surface/identity: f=$(mktemp); npm test -- tests/mockup.test.tsx -t 'case 6 Enter on a focused row' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 6': 1 passed, 0 failed and 6 skipped, the file's other 6 tests.
- Each of the six cases, run against index.v5.html with the same steps, fails at its own assertion; the test runs all six there and asserts 6 failures of 6. The suite proves it in its own test, titled 'all six cases fail on v5'. Run in surface/identity: f=$(mktemp); npm test -- tests/mockup.test.tsx -t 'all six cases fail on v5' --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['numPassedTests'], r['numFailedTests'], r['numPendingTests'])" "$f" prints 0 and then '1 0 6': 1 passed, 0 failed and 6 skipped, the file's other 6 tests.

**Files:**
- create: docs/design/identity/mockup/index.v6.html
- create: surface/identity/tests/mockup.test.tsx

**Checklist:**
- C311 — index.v6.html sits beside index.v5.html and differs from it by the focus fix and the keyboard fix only, each proved against v6 and shown failing on v5.

**Stories:**
- S135 (Identity line lead, Keeping the mock-up and the build in step) — As the identity line lead, I want the mock-up and the built shell to change together and the proof to run in the gate, so that the mock-up stays the definition the build is held to.

### R10: Name index.v6.html as the reference and pin its path and sha256

THE SYSTEM SHALL change line 3 of docs/design/identity/CONFORMANCE.md so it names docs/design/identity/mockup/index.v6.html as the reference, with index.v5.html kept beside it as the prior, and adds that file's path and its sha256, so this brief pins the accepted mock-up; DIRECTORY-006 pins no file and no hash (ADR-104), so this is the first pin, and DIRECTORY-006 SHALL NOT change. THE SYSTEM SHALL change nothing else in CONFORMANCE.md: no row, kind, brief cell or owner.

**Acceptance:**
- sed -n 3p docs/design/identity/CONFORMANCE.md prints: Reference: `docs/design/identity/mockup/index.v6.html` (sha256 <hex>; v5 kept beside it as the prior). — where <hex> is the first field shasum -a 256 docs/design/identity/mockup/index.v6.html prints; python3 -c "import hashlib; h=hashlib.sha256(open('docs/design/identity/mockup/index.v6.html','rb').read()).hexdigest(); print(open('docs/design/identity/CONFORMANCE.md').read().splitlines()[2]=='Reference: `docs/design/identity/mockup/index.v6.html` (sha256 '+h+'; v5 kept beside it as the prior).')" prints True.
- git diff --no-ext-diff --numstat <base> -- docs/design/identity/CONFORMANCE.md prints 1, a tab, 1, a tab and the path: 1 added and 1 deleted line.
- git diff --no-ext-diff --stat <base> -- docs/design/directory/briefs/DIRECTORY-006.json docs/design/directory/briefs/DIRECTORY-006.md prints nothing.
- sh scripts/design/gate.sh >/dev/null 2>&1; echo $? prints 0.

**Files:**
- modify: docs/design/identity/CONFORMANCE.md

**Checklist:**
- C312 — CONFORMANCE.md line 3 names index.v6.html as the reference, with v5 as the prior, and pins index.v6.html by its path and sha256, the first pin of the mock-up.

**Stories:**
- S135 (Identity line lead, Keeping the mock-up and the build in step) — As the identity line lead, I want the mock-up and the built shell to change together and the proof to run in the gate, so that the mock-up stays the definition the build is held to.

## Boundaries

- SHALL NOT change what any screen shows: no screen component's text, data, layout or controls change, and no file under surface/identity/src/features changes, nor surface/identity/src/shell/Shell.tsx or Dock.tsx.
- SHALL NOT change which component any route renders, and SHALL NOT make routes.tsx read the route table.
- SHALL NOT change index.v5.html or index.v4.html; index.v6.html differs from v5 only by the two fixes.
- SHALL NOT build a screen or tab the route table marks not yet, and SHALL NOT present any control as working that is not (DIRECTORY-005).
- SHALL NOT add an assistant composer or input to the dock; the assistant is conformance row 9.4, proposed.
- SHALL NOT change any row, kind, brief cell or owner of CONFORMANCE.md; only its line 3 changes.
- SHALL NOT change any published lys-core wire format or any crate, SHALL NOT change the project setup's existing trees or any leg of them, and SHALL NOT change the gate array of docs/design/directory/design.json except to append the one surface leg R1 names; SHALL NOT reconcile project.json's trees with design.json's gate.
- SHALL NOT add a build dependency on Aion or Cambium packages, and SHALL NOT use Aion blue or purple (ADR-010).
- SHALL NOT present running an agent as something the product does on any screen the rail reaches (ADR-007).
- SHALL NOT decide the service name, the assistant runtime or row 9.4.
- SHALL NOT add a dependency beyond @testing-library/user-event 14.6.7 and @testing-library/dom 10.4.2, both development dependencies.
- SHALL NOT change DIRECTORY-006; it pins no mock-up file or hash (ADR-104).
- SHALL NOT create a file outside the design's structure array.

## Verification

- From any clone of lys: git fetch https://github.com/ablative-io/lys.git main && git ls-tree --name-only FETCH_HEAD -- surface/identity/src/shell/Rail.tsx surface/identity/src/features/me/You.tsx prints surface/identity/src/features/me/You.tsx and then surface/identity/src/shell/Rail.tsx before the build starts.
- Run in surface/identity: f=$(mktemp); npm ci >/dev/null 2>&1 && npm test -- --reporter=json --outputFile="$f" >/dev/null 2>&1; echo $?; python3 -c "import json,sys,os; r=json.load(open(sys.argv[1])); print(r['numFailedTests'], sorted(os.path.basename(t['name']) for t in r['testResults'] if os.path.basename(t['name']).split('.')[0] in ('routes','rail','goto','keyable','cursor','overlay','walk','mockup')))" "$f" prints 0, then: 0 ['cursor.test.tsx', 'goto.test.tsx', 'keyable.test.tsx', 'mockup.test.tsx', 'overlay.test.tsx', 'rail.test.tsx', 'routes.test.tsx', 'walk.test.tsx']
- On the build venue, sh .land/gates.sh 2>&1 | grep -xF -- '--- status 0: surface_leg ---' prints '--- status 0: surface_leg ---'.
- python3 -c "import json,subprocess; b=json.loads(subprocess.check_output(['git','show','<base>:docs/design/directory/design.json']))['gate']; h=json.load(open('docs/design/directory/design.json'))['gate']; i=[t['tree'] for t in b].index('.'); leg=h[i]['legs'][-1]; b[i]['legs'].append(leg); print(json.dumps(h)==json.dumps(b), json.dumps(leg))" prints: True {"name": "surface", "command": "npm --prefix surface/identity install-ci-test", "requires": ["tool:node", "tool:npm"], "cadence": "round"}, with <base> the build's parent commit.
- diff docs/design/identity/mockup/index.v5.html docs/design/identity/mockup/index.v6.html | grep -E '^[0-9]' | python3 -c "import sys,re; R=[(1097,1103),(1111,1135),(1438,1449),(1558,1562)]; h=sys.stdin.read().split(); print(len(h)>0, all(any(a<=int(n)<=b for a,b in R) for x in h for n in re.match(r'(\d+)(?:,(\d+))?',x).groups() if n))" prints: True True
- git diff --no-ext-diff --name-only <base> | LC_ALL=C sort, with <base> the build's parent commit, prints exactly these 19 paths, one per line: .land/gates.sh, docs/design/directory/design.json, docs/design/identity/CONFORMANCE.md, docs/design/identity/mockup/index.v6.html, docs/design/project.json, surface/identity/package-lock.json, surface/identity/package.json, surface/identity/src/shell/Palette.tsx, surface/identity/src/shell/keyable.ts, surface/identity/src/shell/keys.ts, surface/identity/src/shell/routeTable.ts, surface/identity/tests/cursor.test.tsx, surface/identity/tests/goto.test.tsx, surface/identity/tests/keyable.test.tsx, surface/identity/tests/mockup.test.tsx, surface/identity/tests/overlay.test.tsx, surface/identity/tests/rail.test.tsx, surface/identity/tests/routes.test.tsx, surface/identity/tests/walk.test.tsx.
- sh scripts/design/gate.sh >/dev/null 2>&1; echo $? prints 0.

