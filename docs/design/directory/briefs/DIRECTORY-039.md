---
type: brief
id: DIRECTORY-039
cluster: directory
title: Render a brief's prose list fields one entry per line
---

# DIRECTORY-039: Render a brief's prose list fields one entry per line

> **Cluster:** directory
> **Blocked by:** The design-system card that carries the prose-list rule into the method's scripts/render-brief.py has not landed on the method's main. This brief copies that file byte for byte at the commit where it lands, so everything R1 and R2 require of the copied bytes must already hold on the method's main, and the brief is not dispatched until this check, run from the lys repository root, prints '6 True True True True True 1': f=$(mktemp) && gh api -H 'Accept: application/vnd.github.raw' 'repos/ablative-io/design-system/contents/scripts/render-brief.py?ref=main' > "$f" && printf '%s %s\n' "$(python3 -c "import importlib.machinery, importlib.util, json, sys; s = importlib.util.spec_from_loader('rb', importlib.machinery.SourceFileLoader('rb', sys.argv[1])); m = importlib.util.module_from_spec(s); s.loader.exec_module(m); b = getattr(m, 'is_bare_id', None); f = getattr(m, 'render_list_field', None); out = m.render(json.load(open('docs/design/directory/briefs/DIRECTORY-006.json'))).split('\n'); print(sum(x.startswith('> - ') for x in out), '> **Depends on:** DIRECTORY-002, DIRECTORY-003' in out, callable(b) and [b(e) for e in ['ADR-009', 'RM-070', 'C31', 'S13', 'DIRECTORY-006', 'DIRECTORY-006 lands first.', 'ADR-9', 'c31', '']] == [True] * 5 + [False] * 4, callable(b) and all(t in (b.__doc__ or '') for t in ('bare id', 'ADR-', 'RM-')), callable(f) and f('Depends on', ['DIRECTORY-002', 'DIRECTORY-003']) == ['> **Depends on:** DIRECTORY-002, DIRECTORY-003'], callable(f) and f('Depends on', ['DIRECTORY-002', 'DIRECTORY-002 lands first.']) == ['> **Depends on:**', '> - DIRECTORY-002', '> - DIRECTORY-002 lands first.'])" "$f")" "$(grep -c -x 'from __future__ import annotations' "$f")" . Its seven fields are: DIRECTORY-006 renders its six blockers as six '> - ' items; DIRECTORY-006's all-bare-id depends_on stays on the one line '> **Depends on:** DIRECTORY-002, DIRECTORY-003'; the file defines is_bare_id(entry), which accepts the five bare-id forms whole and rejects an id followed by prose, a malformed id and the empty string; is_bare_id's docstring states the rule (it names 'bare id', 'ADR-' and 'RM-'); render_list_field(label, entries) returns one ', '-joined line for an all-bare-id list; render_list_field returns the label line followed by one '> - ' item per entry, the bare id included, for a mixed list; and grep -c -x counts exactly one line 'from __future__ import annotations' in the file, which keeps its 'X | None' annotations importable by the gate host's python3 3.9. On the method's main at the time this brief was written the check prints '0 True False False False False 1'.
> **Checklist:**
> - C31 — scripts/design/render-brief.py is byte-identical to the design-system method's scripts/render-brief.py at the method commit that lands the prose-list rule, and scripts/design/SOURCE.md names that commit for render-brief.py.
> - C32 — A brief's blocked_by, and any other list field whose entries are not all bare ids, renders as its label followed by one Markdown list item per entry, in JSON order, each item's text byte-equal to its entry.
> - C33 — A list field whose entries are all bare ids (ADR-, RM-, C, S and brief ids) renders on one ', '-joined line exactly as before, and render-brief.py states the rule that tells the two kinds of list apart in the docstring of is_bare_id, which render_list_field applies to every list field.
> - C34 — Every brief markdown that render-brief.py produces from a brief JSON in a cluster with a design.json is re-rendered in the same change, sh scripts/design/gate.sh exits 0, and docs/design/identity/briefs/IDENTITY-001.md and CONTEXT-001.md are unchanged.
> - C35 — scripts/design/tests/test_render_brief.py proves a two-entry prose blocked_by renders two list items byte-equal to their entries and a bare-id list renders on one line, and scripts/design/gate.sh runs it so the design leg fails when it fails.
> **Stories:**
> - S13 (Brief reader, Reads a rendered brief before it is dispatched or built) — As a reader of a rendered brief, I want each blocker and each other prose entry on its own line in the order the brief gives them, so that I can tell where one entry ends and the next begins.
> - S14 (Design gate maintainer, Keeps the repository's rendered documents what its renderer makes of their JSON) — As the maintainer of the design gate, I want the renderer in this repository to stay the method's copy and every rendered brief to be what it makes of its JSON, so that no .md disagrees with the script that made it and a card finished through the chain never turns the gate red.

## Purpose

scripts/design/render-brief.py joins every list field of a brief onto one line with ', '. For bare ids that reads well; for blocked_by, whose entries are whole paragraphs, it produces one run-on line in which a reader cannot tell where one blocker ends and the next begins (docs/design/directory/briefs/DIRECTORY-006.md joins six blockers on its '> **Blocked by:**' line). This brief brings lys the method's fixed renderer, which renders every list field that holds prose as a Markdown list under its label, one entry per line, in JSON order and byte for byte, while lists of bare ids stay on one line; it proves both behaviours with a test the design gate runs, and re-renders every rendered brief so no .md disagrees with the script that made it.

## Task

The renderer change itself lands in the design-system method's scripts/render-brief.py through the method's own card; lys's scripts/design/render-brief.py is a copy of the method's (scripts/design/SOURCE.md) and never diverges from it, because the method's finish step re-renders lys clusters with the method's own render-cluster.py after every card, and a lys-only change would be re-rendered away in the old joined form on the next card and turn scripts/design/gate.sh red. This brief is the lys side: R1 copies the method's render-brief.py byte for byte at the commit where the rule lands and records that commit in SOURCE.md; R2 adds a stdlib unittest that proves the rule on lys's copy; R3 makes gate.sh run that test; R4 re-renders every rendered brief and passes the gate. It is blocked until the method's main holds the rule, by the check in blocked_by.

The rule (C33) is: an entry is a bare id when the whole entry, with nothing before or after it, is one of: ADR- followed by three digits, RM- followed by three digits, C followed by one or more digits, S followed by one or more digits, or a brief id (one or more capital letters, a hyphen and three digits), the forms brief.schema.json and roadmap.schema.json pattern those ids with; a list field renders on one ', '-joined line after its label only when every entry is a bare id, and otherwise renders as its label on a line of its own followed by one '> - ' line per entry. It is stated in the docstring of the function is_bare_id(entry) in render-brief.py, which returns True exactly for a bare id, and it is applied by the function render_list_field(label, entries), which returns the lines a list field renders to (label being the text between '**' and ':**'); render() renders depends_on and blocked_by, and design_anchor, checklist and stories when no lookup exists, through render_list_field, so the rule applies to every brief. Both functions are carried by the method's file and confirmed by the blocker's check before dispatch; the build copies them and writes neither. Under it: blocked_by renders as a list for every brief that has one, since no blocker is a bare id; depends_on stays on one line because every depends_on entry in the repository is a brief id; design_anchor, checklist and stories keep their existing list form when a lookup exists and stay on one line when none exists, since their entries are ADR, C and S ids; review 'Checklist verified' and 'Stories verified' stay on one line, since their entries are C and S ids. The execution record's 'Gate (measured)' and 'Attestation (believed)' lines are built by the renderer from booleans and one integer, are not list fields of the brief, and are unchanged. A prose list renders inside the metadata blockquote in the form design_anchor, checklist and stories already use: '> **Blocked by:**' on its own line, then '> - ' followed by the entry, one line per entry.

Scope. In: the copy of render-brief.py, its line in SOURCE.md, a test file and the gate.sh line that runs it, and every rendered brief .md whose bytes the new renderer changes. 'Every committed rendered brief' means every .md that render-brief.py produces from a brief JSON in a cluster directory holding a design.json (directory, home and secrets on the tree this brief was written on, and any cluster added since); gate.sh measures exactly these. docs/design/identity/briefs/IDENTITY-001.md and CONTEXT-001.md are hand-kept revision-form documents with no design.json, not this renderer's output, and are excluded and left untouched. Out: any brief's content, ids and JSON schema; the other copied scripts (validate.py, check-coverage.py, render-cluster.py, schemas/, workers/); the method's own change, its tests and its gate, which its own card carries.

The acceptance evidence is DIRECTORY-006, whose '> **Blocked by:**' line on the tree this brief was written on joins six blockers: after this change it renders as six list items, each byte-equal to its JSON entry, in order. Only the blocked_by lines of a rendered brief change; on the tree this brief was written on those are DIRECTORY-002, 003, 004, 005, 006 and 008, HOME-001, 003 and 006, and SECRETS-002, plus this brief's own DIRECTORY-039.md, which the chain renders when the brief lands. Briefs landed on main after that are re-rendered the same way; R4's checks run over whatever the tree then holds. Every path is relative to the repository root.

## Requirements

### R1: Copy the method's render-brief.py at the commit that lands the rule

scripts/design/render-brief.py is replaced by the bytes of scripts/render-brief.py at the design-system method's main commit that lands the prose-list rule in scripts/render-brief.py (the commit on which the blocker's check first prints '6 True True True True True 1'), written <L> below. Those bytes define is_bare_id(entry), whose docstring states the rule and which returns True exactly for a bare id, and render_list_field(label, entries), which applies it, and carry the line 'from __future__ import annotations', which keeps the file importable by python3 3.9; the build copies them and does not write them. scripts/design/SOURCE.md names <L> (its first seven hex characters) as the commit render-brief.py is copied at, and keeps 3c3bac7 as the commit validate.py, check-coverage.py, render-cluster.py, schemas/ and workers/ds2_ledger/roots.py are copied at. The lys copy SHALL NOT differ from the method's file by any byte, SHALL NOT lack the line 'from __future__ import annotations', SHALL NOT carry a rule written in lys by hand, and SOURCE.md SHALL NOT claim any file other than render-brief.py is copied at <L>. No other file under scripts/design changes in this requirement.

**Acceptance:**
- From the repository root: gh api -H 'Accept: application/vnd.github.raw' 'repos/ablative-io/design-system/contents/scripts/render-brief.py?ref=<L>' | cmp - scripts/design/render-brief.py exits 0 and prints nothing.
- From the repository root: python3 scripts/design/render-brief.py docs/design/directory/briefs/DIRECTORY-006.json --no-resolve | grep -c '^> - ' prints 6.
- From the repository root, the blocker's check run on the lys copy prints '6 True True True True True 1': printf '%s %s\n' "$(python3 -c "import importlib.machinery, importlib.util, json, sys; s = importlib.util.spec_from_loader('rb', importlib.machinery.SourceFileLoader('rb', sys.argv[1])); m = importlib.util.module_from_spec(s); s.loader.exec_module(m); b = getattr(m, 'is_bare_id', None); f = getattr(m, 'render_list_field', None); out = m.render(json.load(open('docs/design/directory/briefs/DIRECTORY-006.json'))).split('\n'); print(sum(x.startswith('> - ') for x in out), '> **Depends on:** DIRECTORY-002, DIRECTORY-003' in out, callable(b) and [b(e) for e in ['ADR-009', 'RM-070', 'C31', 'S13', 'DIRECTORY-006', 'DIRECTORY-006 lands first.', 'ADR-9', 'c31', '']] == [True] * 5 + [False] * 4, callable(b) and all(t in (b.__doc__ or '') for t in ('bare id', 'ADR-', 'RM-')), callable(f) and f('Depends on', ['DIRECTORY-002', 'DIRECTORY-003']) == ['> **Depends on:** DIRECTORY-002, DIRECTORY-003'], callable(f) and f('Depends on', ['DIRECTORY-002', 'DIRECTORY-002 lands first.']) == ['> **Depends on:**', '> - DIRECTORY-002', '> - DIRECTORY-002 lands first.'])" scripts/design/render-brief.py)" "$(grep -c -x 'from __future__ import annotations' scripts/design/render-brief.py)" . Its fourth field measures that is_bare_id's docstring in scripts/design/render-brief.py states the rule (it names 'bare id', 'ADR-' and 'RM-'); its third, fifth and sixth measure that the stated rule is the one the file applies; its seventh measures that the copy keeps the line 'from __future__ import annotations'.
- From the repository root: grep -c -x 'from __future__ import annotations' scripts/design/render-brief.py prints 1.
- From the repository root, where <L7> is the first seven hex characters of <L>: python3 -c "import re; t = ' '.join(open('scripts/design/SOURCE.md').read().split()); ss = re.split(r'(?<=\.) ', t); a = [x for x in ss if '<L7>' in x]; c = [x for x in ss if '3c3bac7' in x]; n = ('validate.py', 'check-coverage.py', 'render-cluster.py', 'schemas/', 'roots.py'); print(len(a), len(c), 'render-brief.py' in (a or [''])[0], 'render-brief.py' in (c or [''])[0], any(k in (a or [''])[0] for k in n), all(k in (c or [''])[0] for k in n))" prints '1 1 True False False True': SOURCE.md has one sentence naming <L7>, which names render-brief.py and none of the other copied files, and one sentence naming 3c3bac7, which names every other copied file and not render-brief.py.
- git diff --name-only <base> -- scripts/design ':!scripts/design/tests' ':!scripts/design/gate.sh', where <base> is the commit the build started from, prints exactly two lines: scripts/design/SOURCE.md and scripts/design/render-brief.py.

**Files:**
- modify: scripts/design/render-brief.py
- modify: scripts/design/SOURCE.md

**Checklist:**
- C31 — scripts/design/render-brief.py is byte-identical to the design-system method's scripts/render-brief.py at the method commit that lands the prose-list rule, and scripts/design/SOURCE.md names that commit for render-brief.py.
- C33 — A list field whose entries are all bare ids (ADR-, RM-, C, S and brief ids) renders on one ', '-joined line exactly as before, and render-brief.py states the rule that tells the two kinds of list apart in the docstring of is_bare_id, which render_list_field applies to every list field.

**Stories:**
- S14 (Design gate maintainer, Keeps the repository's rendered documents what its renderer makes of their JSON) — As the maintainer of the design gate, I want the renderer in this repository to stay the method's copy and every rendered brief to be what it makes of its JSON, so that no .md disagrees with the script that made it and a card finished through the chain never turns the gate red.

#### R1 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. None of the acceptance rows can be met. The spec requires the bytes of the method's scripts/render-brief.py at the main commit where the blocker check first prints '6 True True True True True 1'. No such commit exists. Running the blocker check from the repository root prints '0 True False False False False 1'. On the method's main, scripts/render-brief.py was last changed at f77ef422613a3f577facfa869f8f6c356716ad27 and has neither is_bare_id nor render_list_field. The method's render-lists card is only a brief, on branch brief/render-lists/99b64201-00c2-4fd8-ba9a-35e884a14fe7 (commit a252f20), with no build and no PR. Row by row: the gh api | cmp row has no <L> to name. The '--no-resolve | grep -c' row would print 0 on the current copy. The blocker check on the lys copy would print '0 True False False False False 1'. The 'from __future__ import annotations' row already prints 1 on the current copy, since f77ef42 carried that line, but the requirement also needs the rule. The SOURCE.md row has no <L7> to record. The diff row stays unmet because nothing was changed. Writing is_bare_id or render_list_field into scripts/design/render-brief.py in lys is forbidden by the boundary 'SHALL NOT write the prose-list rule into scripts/design/render-brief.py by hand'. Changing the method's repository is also forbidden. It unblocks when the method's card lands the rule on design-system main; then the build copies the file at that commit and records its first seven hex characters in scripts/design/SOURCE.md.
- Deviation: No files were changed. The brief was dispatched while its blocked_by check still failed, and the brief's own words say 'the brief is not dispatched until this check ... prints 6 True True True True True 1'.
- Checklist delivery:
  - [ ] C31 — scripts/design/render-brief.py is byte-identical to the design-system method's scripts/render-brief.py at the method commit that lands the prose-list rule, and scripts/design/SOURCE.md names that commit for render-brief.py. — There is no method commit carrying the rule to copy from.
  - [ ] C33 — A list field whose entries are all bare ids (ADR-, RM-, C, S and brief ids) renders on one ', '-joined line exactly as before, and render-brief.py states the rule that tells the two kinds of list apart in the docstring of is_bare_id, which render_list_field applies to every list field. — The rule and its docstring must come from the method's file, which does not carry them yet.
- Story delivery:
  - [ ] S14 (Design gate maintainer, Keeps the repository's rendered documents what its renderer makes of their JSON) — As the maintainer of the design gate, I want the renderer in this repository to stay the method's copy and every rendered brief to be what it makes of its JSON, so that no .md disagrees with the script that made it and a card finished through the chain never turns the gate red. — Blocked on the method's card landing first.

### R2: Prove prose lists render one entry per line and bare-id lists stay on one line

WHEN render() of scripts/design/render-brief.py renders a brief whose blocked_by holds two multi-sentence entries, THE SYSTEM SHALL emit the line '> **Blocked by:**' followed by exactly two lines, each '> - ' followed by one entry byte for byte, in the JSON's order, and SHALL NOT join, drop, reorder, trim or reword any entry. WHEN render() renders a brief whose depends_on entries are all bare ids, THE SYSTEM SHALL emit them on one line as '> **Depends on:** ' followed by the ids joined by ', ', exactly as before this change, and SHALL NOT render them as list items. WHEN render() renders a list field in which at least one entry is not a bare id, THE SYSTEM SHALL render every entry of that field as a list item, the bare ids among them included, and SHALL NOT treat an entry that begins with an id and continues with prose as a bare id. scripts/design/tests/test_render_brief.py is a stdlib unittest file that loads scripts/design/render-brief.py by its path relative to the test file, calls render() on brief dictionaries built in the test with no lookups, and runs its cases when executed as python3 scripts/design/tests/test_render_brief.py. Each case asserts the presence of every line it looks up with a unittest assertion before using its position, so a missing line is a test failure and not an error. The test SHALL NOT read any brief under docs/design, SHALL NOT depend on a package outside the standard library, and SHALL NOT pass when it runs no case.

**Acceptance:**
- python3 scripts/design/tests/test_render_brief.py exits 0 and its output contains 'Ran 3 tests' and 'OK'.
- The prose case: a brief whose blocked_by is ['The first blocker lands first. Its check command prints 6.', 'The second blocker, which has a comma, waits on node. Then a rebase follows.'] renders to text containing the three consecutive lines '> **Blocked by:**', '> - The first blocker lands first. Its check command prints 6.' and '> - The second blocker, which has a comma, waits on node. Then a rebase follows.', the test asserts that exactly 2 lines beginning '> - ' follow the label before the next line not beginning '> - ', and asserts each of those two lines with its '> - ' removed equals its entry.
- The bare-id case: a brief whose depends_on is ['DIRECTORY-002', 'DIRECTORY-003'] renders to text containing the line '> **Depends on:** DIRECTORY-002, DIRECTORY-003', and no line of that text equals '> - DIRECTORY-002'.
- The mixed case: a brief whose depends_on is ['DIRECTORY-002', 'DIRECTORY-002 lands with its check command.'] renders to text containing the three consecutive lines '> **Depends on:**', '> - DIRECTORY-002' and '> - DIRECTORY-002 lands with its check command.', and the test asserts the line '> **Depends on:**' is present before it looks up the lines that follow it.
- With every statement of render_list_field in scripts/design/render-brief.py replaced by the three lines `if all(is_bare_id(e) for e in entries):`, `    return [f"> **{label}:** {', '.join(entries)}"]` and `return [f"> **{label}:**"] + [f"> - {e.split('. ')[0]}" for e in entries]` (each item keeps only its entry's first sentence), python3 scripts/design/tests/test_render_brief.py exits 1, reports 'FAILED (failures=1)', and the one failing test is the prose case.
- With every statement of is_bare_id in scripts/design/render-brief.py after its docstring replaced by the one line `return False`, python3 scripts/design/tests/test_render_brief.py exits 1, reports 'FAILED (failures=1)', and the one failing test is the bare-id case.
- With every statement of is_bare_id in scripts/design/render-brief.py after its docstring replaced by the one line `return entry[:1].isupper() and '-' in entry.split()[0]` (an entry that begins with an id counts as a bare id), python3 scripts/design/tests/test_render_brief.py exits 1, reports 'FAILED (failures=1)', and the one failing test is the mixed case.

**Files:**
- create: scripts/design/tests/test_render_brief.py

**Checklist:**
- C32 — A brief's blocked_by, and any other list field whose entries are not all bare ids, renders as its label followed by one Markdown list item per entry, in JSON order, each item's text byte-equal to its entry.
- C33 — A list field whose entries are all bare ids (ADR-, RM-, C, S and brief ids) renders on one ', '-joined line exactly as before, and render-brief.py states the rule that tells the two kinds of list apart in the docstring of is_bare_id, which render_list_field applies to every list field.
- C35 — scripts/design/tests/test_render_brief.py proves a two-entry prose blocked_by renders two list items byte-equal to their entries and a bare-id list renders on one line, and scripts/design/gate.sh runs it so the design leg fails when it fails.

**Stories:**
- S13 (Brief reader, Reads a rendered brief before it is dispatched or built) — As a reader of a rendered brief, I want each blocker and each other prose entry on its own line in the order the brief gives them, so that I can tell where one entry ends and the next begins.

#### R2 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. scripts/design/tests/test_render_brief.py was not created. Its three cases test render() on lys's copied renderer, and the current copy joins every list with ', '. The prose case and the mixed case would fail, so 'Ran 3 tests ... OK' cannot be met. The three mutation rows edit is_bare_id and render_list_field in scripts/design/render-brief.py, and neither function exists in the current copy. Adding the test now would also turn the design leg red once R3 wires it in. It unblocks with R1.
- Deviation: The test file was not created because it depends on R1's copied bytes, and nothing it would test exists yet.
- Checklist delivery:
  - [ ] C32 — A brief's blocked_by, and any other list field whose entries are not all bare ids, renders as its label followed by one Markdown list item per entry, in JSON order, each item's text byte-equal to its entry. — The renderer does not yet render prose lists one entry per line; it waits on R1.
  - [ ] C33 — A list field whose entries are all bare ids (ADR-, RM-, C, S and brief ids) renders on one ', '-joined line exactly as before, and render-brief.py states the rule that tells the two kinds of list apart in the docstring of is_bare_id, which render_list_field applies to every list field. — Waits on R1.
  - [ ] C35 — scripts/design/tests/test_render_brief.py proves a two-entry prose blocked_by renders two list items byte-equal to their entries and a bare-id list renders on one line, and scripts/design/gate.sh runs it so the design leg fails when it fails. — The test was not written; it would fail against the pre-rule renderer.
- Story delivery:
  - [ ] S13 (Brief reader, Reads a rendered brief before it is dispatched or built) — As a reader of a rendered brief, I want each blocker and each other prose entry on its own line in the order the brief gives them, so that I can tell where one entry ends and the next begins. — Blocked on the method's card.

### R3: Run the renderer test in the design leg

WHEN sh scripts/design/gate.sh runs, THE SYSTEM SHALL run python3 scripts/design/tests/test_render_brief.py and SHALL exit non-zero when that test exits non-zero, beside the validate, coverage and render checks it already makes, and whether or not any of those checks passes. gate.sh SHALL NOT drop, reorder or weaken any check it makes today, SHALL NOT discard the test's output, and SHALL NOT treat a missing test file as a pass.

**Acceptance:**
- With the expected entry text of the prose case in scripts/design/tests/test_render_brief.py altered by one character, sh scripts/design/gate.sh exits 1 and its combined output contains the lines 'Ran 3 tests' and 'FAILED (failures=1)', the test's own summary, which proves gate.sh ran the test whether or not its render cmp passed.
- With scripts/design/tests/test_render_brief.py moved aside, sh scripts/design/gate.sh exits 1 and its combined output contains a line ending "test_render_brief.py': [Errno 2] No such file or directory", python3's own refusal to open the test file.
- git diff <base> -- scripts/design/gate.sh, where <base> is the commit the build started from, removes no line.
- git diff --name-only <base> -- scripts/design, where <base> is the commit the build started from, prints exactly four lines: scripts/design/SOURCE.md, scripts/design/gate.sh, scripts/design/render-brief.py and scripts/design/tests/test_render_brief.py.

**Files:**
- modify: scripts/design/gate.sh

**Checklist:**
- C35 — scripts/design/tests/test_render_brief.py proves a two-entry prose blocked_by renders two list items byte-equal to their entries and a bare-id list renders on one line, and scripts/design/gate.sh runs it so the design leg fails when it fails.

**Stories:**
- S14 (Design gate maintainer, Keeps the repository's rendered documents what its renderer makes of their JSON) — As the maintainer of the design gate, I want the renderer in this repository to stay the method's copy and every rendered brief to be what it makes of its JSON, so that no .md disagrees with the script that made it and a card finished through the chain never turns the gate red.

#### R3 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. scripts/design/gate.sh was not changed. Making it run a test that does not exist would make the gate exit 1, and running R2's test against the pre-rule renderer would also fail it. Either way the design leg would turn red before the rule exists. Both mutation rows and both diff rows depend on R1 and R2. It unblocks with R1 and R2.
- Deviation: gate.sh was not changed, to keep the design leg green while the rule is missing upstream.
- Checklist delivery:
  - [ ] C35 — scripts/design/tests/test_render_brief.py proves a two-entry prose blocked_by renders two list items byte-equal to their entries and a bare-id list renders on one line, and scripts/design/gate.sh runs it so the design leg fails when it fails. — Waits on R2.
- Story delivery:
  - [ ] S14 (Design gate maintainer, Keeps the repository's rendered documents what its renderer makes of their JSON) — As the maintainer of the design gate, I want the renderer in this repository to stay the method's copy and every rendered brief to be what it makes of its JSON, so that no .md disagrees with the script that made it and a card finished through the chain never turns the gate red. — Blocked on the method's card.

### R4: Re-render every rendered brief and pass the design gate

WHEN the copied renderer is in place, THE SYSTEM SHALL re-render every cluster under docs/design that holds a design.json with python3 scripts/design/render-cluster.py and commit every brief .md whose bytes change in the same change, so that each brief whose blocked_by is non-empty renders '> **Blocked by:**' followed by one '> - ' line per entry, in the JSON's order, each byte-equal to its entry. THE SYSTEM SHALL NOT change any brief JSON, any schema, any line of a rendered brief other than its blocked_by lines, any rendered DESIGN.md, CHECKLIST.md or USER-STORIES.md, nor docs/design/identity/briefs/IDENTITY-001.md and docs/design/identity/briefs/CONTEXT-001.md, which are hand-kept and not this renderer's output.

**Acceptance:**
- From the repository root: python3 -c "import json; b = json.load(open('docs/design/directory/briefs/DIRECTORY-006.json'))['blocked_by']; lines = open('docs/design/directory/briefs/DIRECTORY-006.md').read().split('\n'); i = lines.index('> **Blocked by:**'); print(len(b), lines[i + 1:i + 1 + len(b)] == ['> - ' + e for e in b], lines[i + 1 + len(b)].startswith('> - '))" prints '6 True False': DIRECTORY-006.md renders its six blockers as six list items, each byte-equal to its JSON entry, in order, and no seventh item follows.
- From the repository root: python3 -c "import glob, json, pathlib; n = bad = 0
for j in sorted(glob.glob('docs/design/*/briefs/*.json')):
    if not (pathlib.Path(j).parents[1] / 'design.json').exists(): continue
    b = json.load(open(j))['blocked_by']
    if not b: continue
    n += 1; lines = open(j[:-5] + '.md').read().split('\n'); i = lines.index('> **Blocked by:**')
    bad += lines[i + 1:i + 1 + len(b)] != ['> - ' + e for e in b]
print(n, bad)" prints two numbers, the second 0 and the first at least 11 (the ten briefs with blockers on the tree this brief was written on, plus DIRECTORY-039).
- From the repository root: sh scripts/design/gate.sh exits 0 and its combined output contains 'Ran 3 tests'.
- git diff --name-only <base>, where <base> is the commit the build started from, prints no path ending in .json, no path under docs/design/identity, no path under scripts/design/schemas and no path ending in DESIGN.md, CHECKLIST.md or USER-STORIES.md.
- git diff -U0 <base> -- 'docs/design/*/briefs/*.md' holds only removed lines that begin '-> **Blocked by:** ' and added lines that are '+> **Blocked by:**' or begin '+> - '.

**Files:**
- modify: docs/design/directory/briefs/DIRECTORY-002.md
- modify: docs/design/directory/briefs/DIRECTORY-003.md
- modify: docs/design/directory/briefs/DIRECTORY-004.md
- modify: docs/design/directory/briefs/DIRECTORY-005.md
- modify: docs/design/directory/briefs/DIRECTORY-006.md
- modify: docs/design/directory/briefs/DIRECTORY-008.md
- modify: docs/design/directory/briefs/DIRECTORY-039.md
- modify: docs/design/home/briefs/HOME-001.md
- modify: docs/design/home/briefs/HOME-003.md
- modify: docs/design/home/briefs/HOME-006.md
- modify: docs/design/secrets/briefs/SECRETS-002.md

**Checklist:**
- C34 — Every brief markdown that render-brief.py produces from a brief JSON in a cluster with a design.json is re-rendered in the same change, sh scripts/design/gate.sh exits 0, and docs/design/identity/briefs/IDENTITY-001.md and CONTEXT-001.md are unchanged.

**Stories:**
- S13 (Brief reader, Reads a rendered brief before it is dispatched or built) — As a reader of a rendered brief, I want each blocker and each other prose entry on its own line in the order the brief gives them, so that I can tell where one entry ends and the next begins.
- S14 (Design gate maintainer, Keeps the repository's rendered documents what its renderer makes of their JSON) — As the maintainer of the design gate, I want the renderer in this repository to stay the method's copy and every rendered brief to be what it makes of its JSON, so that no .md disagrees with the script that made it and a card finished through the chain never turns the gate red.

#### R4 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. With the current renderer, re-rendering every cluster produces the old joined blocked_by lines, which are byte-identical to the committed files. DIRECTORY-006.md therefore still has '> **Blocked by:** ' followed by six joined entries, and the row expecting '6 True False' cannot be met. The all-briefs row would fail at lines.index('> **Blocked by:**'). The row expecting 'Ran 3 tests' needs R2 and R3. The rendered brief .md files were left untouched, because hand-editing them would make gate.sh's render cmp fail. It unblocks with R1 to R3.
- Deviation: No brief .md files were re-rendered because the renderer they depend on has not landed upstream.
- Checklist delivery:
  - [ ] C34 — Every brief markdown that render-brief.py produces from a brief JSON in a cluster with a design.json is re-rendered in the same change, sh scripts/design/gate.sh exits 0, and docs/design/identity/briefs/IDENTITY-001.md and CONTEXT-001.md are unchanged. — Nothing to re-render until R1 lands.
- Story delivery:
  - [ ] S13 (Brief reader, Reads a rendered brief before it is dispatched or built) — As a reader of a rendered brief, I want each blocker and each other prose entry on its own line in the order the brief gives them, so that I can tell where one entry ends and the next begins. — Blocked on the method's card.
  - [ ] S14 (Design gate maintainer, Keeps the repository's rendered documents what its renderer makes of their JSON) — As the maintainer of the design gate, I want the renderer in this repository to stay the method's copy and every rendered brief to be what it makes of its JSON, so that no .md disagrees with the script that made it and a card finished through the chain never turns the gate red. — Blocked on the method's card.

## Boundaries

- SHALL NOT change any brief JSON's content, ids or ordering, nor any file under scripts/design/schemas.
- SHALL NOT write the prose-list rule into scripts/design/render-brief.py by hand: the file is the method's bytes at the commit that lands the rule, and never diverges from it.
- SHALL NOT change scripts/design/validate.py, check-coverage.py, render-cluster.py, schemas/ or workers/.
- SHALL NOT modify docs/design/identity/briefs/IDENTITY-001.md, docs/design/identity/briefs/CONTEXT-001.md, nor anything else under docs/design/identity.
- SHALL NOT change any rendered line of a brief other than its blocked_by lines.
- SHALL NOT change any Rust crate, Cargo manifest or lock file, nor any gate leg other than the design leg's scripts/design/gate.sh.
- SHALL NOT change the design-system method's repository; its change, tests and gate belong to its own card.

## Verification

- Run the blocker's check from the repository root and confirm it prints '6 True True True True True 1' before starting: f=$(mktemp) && gh api -H 'Accept: application/vnd.github.raw' 'repos/ablative-io/design-system/contents/scripts/render-brief.py?ref=main' > "$f" && printf '%s %s\n' "$(python3 -c "import importlib.machinery, importlib.util, json, sys; s = importlib.util.spec_from_loader('rb', importlib.machinery.SourceFileLoader('rb', sys.argv[1])); m = importlib.util.module_from_spec(s); s.loader.exec_module(m); b = getattr(m, 'is_bare_id', None); f = getattr(m, 'render_list_field', None); out = m.render(json.load(open('docs/design/directory/briefs/DIRECTORY-006.json'))).split('\n'); print(sum(x.startswith('> - ') for x in out), '> **Depends on:** DIRECTORY-002, DIRECTORY-003' in out, callable(b) and [b(e) for e in ['ADR-009', 'RM-070', 'C31', 'S13', 'DIRECTORY-006', 'DIRECTORY-006 lands first.', 'ADR-9', 'c31', '']] == [True] * 5 + [False] * 4, callable(b) and all(t in (b.__doc__ or '') for t in ('bare id', 'ADR-', 'RM-')), callable(f) and f('Depends on', ['DIRECTORY-002', 'DIRECTORY-003']) == ['> **Depends on:** DIRECTORY-002, DIRECTORY-003'], callable(f) and f('Depends on', ['DIRECTORY-002', 'DIRECTORY-002 lands first.']) == ['> **Depends on:**', '> - DIRECTORY-002', '> - DIRECTORY-002 lands first.'])" "$f")" "$(grep -c -x 'from __future__ import annotations' "$f")"
- python3 scripts/design/tests/test_render_brief.py prints 'Ran 3 tests' and 'OK'.
- sh scripts/design/gate.sh exits 0 and prints 'Ran 3 tests'.
- git diff --name-only <base> lists only scripts/design/render-brief.py, scripts/design/SOURCE.md, scripts/design/gate.sh, scripts/design/tests/test_render_brief.py and brief .md files under docs/design/*/briefs outside docs/design/identity.
- git status --short prints nothing after the commit: no rendered file is left out of it.
