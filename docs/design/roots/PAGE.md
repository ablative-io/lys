# roots — what was asked, what it means, and what was written

## The words, as they were typed

The file docs/design/project.json at lines 54 to 59 (roots) names aion, argus, cambium, haematite and the method by absolute paths under /Users/tom/Developer/ablative. Any worker on another machine (Dean's laptop, Annabel) resolves none of them, and Tom's 2026-09-23 19:23 rule says workflow inputs name a repository, a commit, a card and a brief, never a folder. Make each root a repository URL plus the commit the design was checked against, and have scripts/design/*.py resolve a root through the worker's own checkout of that repository rather than a path on this Mac. Defect 8 of Waffles' 2026-09-25 22:35 audit.

Rulings of the lead, Archie, given on 27 September 2026 to the runs a31929b9 and fe5dd7f7 in answer to their rounds. Both runs took every answer and then failed, the first under the wrong cluster and the second while writing. They are settled here, and the author reopens none of them.

The brief is written in two parts. This card does not change the shape of lys's project.json while the method's reader refuses it, because that would stop every survey on lys. R1 is documents-only on lys. It records the baseline of the five absolute roots at lines 54 to 59 and the contract of the new root shape, an object with repository as a URL and commit as a full hash, resolved through the worker's own checkout of that repository. R2 changes lys's project.json to that shape. R2 is blocked by a method commit whose ds2_ledger/project.py accepts the shape, with a check a stranger can run, naming the method commit and the command that runs the method's reader against lys's project.json at that commit exiting 0. The method change is its own card on the design-system board, written from R1's contract. The refresh of lys's copy of the method from that commit is R2's first act.

The resolver never fetches or clones. When the worker holds no checkout of the root's repository at the pinned commit, it refuses by name, naming the repository, the commit and the checkout path it looked in, and says the act that answers it, which is to fetch that commit into the worker's checkout of that repository. That matches documents.py, which refuses rather than guesses, and keeps a survey on Dean's laptop or Annabel from quietly cloning five repositories.

A root's commit moves by hand, in a gated row, and never automatically. It is the commit the design was checked against, and it moves only when someone re-checks the design against a newer tree and says so in the commit that moves it. A survey reads the pinned commit and never the moving head, so a stale pin is a visible fact and not a silent drift. The brief adds one check and no mover. The check lists each root's pin beside origin's head for that repository and prints the distance, exiting 0 either way. There is one pin per root in project.json and no per-design pin in this card.

The $NAME/ tokens and the roots map stay apart in this card. The tokens are launch settings and are not read from a file in the tree, which is documents.py's rule, and this card changes only the roots map in project.json. Unifying the two would change how every brief path outside the repository resolves, and that is not in these words. The brief records the two meanings of root as an open point with both readings, for a later card, and changes neither the tokens nor roots.py.

Only lys's project.json changes. The words name lys's lines 54 to 59 and this card's repository is lys. The other four project.json files, of aion, cambium, haematite and the design system, are their own leads' cards on their own boards, each fired once the method accepts the new shape. The brief lists them under further units not written, naming the stale meridian path in the method root of aion and of haematite as a finding.

The worker finds its checkouts through one launch setting, DS2_CHECKOUTS, naming a directory that holds one checkout per repository. Each checkout is named by the last path segment of the repository URL without .git. The resolver requires the checkout at that name under DS2_CHECKOUTS to hold the pinned commit, checked with git cat-file -e on the commit in that checkout, and refuses by name otherwise, naming that path, the repository and the commit, with the act that answers it. The pin-versus-origin-head check reads origin from the same checkout. There is one setting, so every survey on a worker resolves the same way and the refusal names one path.

This design lives in a new cluster directory, docs/design/roots. The directory docs/design/lys-core is left exactly as it is, and no file there is renamed, rendered or added, because lys-core has one owner, the ast-grep leg card. The brief id is ROOTS-001, the first under the new cluster's prefix. The roadmap row and any decision take the next free id after main's highest and every open brief/* and draft/* branch's, checked with git ls-remote immediately before writing.

The resolver lives only in the method's card. ROOTS-001 adds no lys-local resolver script, and CN5 stands, so lys's scripts under scripts/design stay verbatim copies of the method as SOURCE.md says. The DS2_CHECKOUTS resolver is written in the method, in workers/ds2_ledger/project.py, under the design-system card filed from R1's contract, and lys receives it when R2 refreshes its copies from the method commit that carries it. The words' sentence is met that way, because after the refresh the scripts in lys's tree resolve a root through the worker's own checkout. A person who wants to run the resolver on lys before R2 lands runs the method's reader from that commit against lys's project.json, which is R2's check. Answered by Archie, lead for lys.

## What the survey found, and its angles

The words ask that lys's docs/design/project.json stop naming the other estate projects by absolute folders on Tom's Mac. Each root becomes a repository URL plus the full commit the design was checked against, and a worker resolves a root through its own checkout of that repository. Archie's rulings split this in two. ROOTS-001 R1 is documents only: a new docs/design/roots cluster that records the baseline of the five absolute roots and the contract of the new shape, the DS2_CHECKOUTS resolver and its refusal, and a pin-versus-origin-head check. R2 changes lys's project.json to the new shape. R2 is blocked on a design-system method commit whose workers/ds2_ledger/project.py accepts that shape and carries the resolver, and R2's first act is to refresh lys's copy of the method from that commit.

### What the tree holds

- `docs/design/project.json:54-60` — The roots map: five string values, all absolute paths under /Users/tom/Developer/ablative (aion, argus, cambium, haematite, method). R1 records this as the baseline and R2 is what changes it.
- `scripts/design/schemas/project.schema.json:81-84` — lys's copy of the schema types roots only as "type": "object" and says each value is an absolute directory checked by the ledger. validate.py therefore does not constrain values, so the gate alone would not refuse the new object shape. The method's reader is what refuses it.
- `/Users/tom/Developer/ablative/tools/design-system/workers/ds2_ledger/project.py:111-127` — _roots calls _text on each value, so it requires a string, and refuses any path that is not absolute. It is the reader that refuses the new {repository, commit} object today, and it is the seam the method card changes. root_lines (153-155) prints 'name: directory' into the survey prompt.
- `/Users/tom/Developer/ablative/tools/design-system/workers/ds2_ledger/words_source.py:105-110` — _estate_lines already drops root directories from the source-based survey prompt because 'a folder on one machine is no use to a session on another'. words.py:223 still prints project.root_lines with the absolute paths, and that is the prompt this survey was given.
- `/Users/tom/Developer/ablative/tools/design-system/workers/ds2_ledger/documents.py:1-50` — This is the precedent the rulings cite. The $NAME/ root tokens resolve only from DS2_<NAME> launch settings, never from a file in the tree, and are refused by name with RootUnset. The new resolver should match its refuse-don't-guess style, and it must stay separate from this module.
- `scripts/design/workers/ds2_ledger/roots.py` — A 44-line verbatim copy of the method's roots.py (diff is empty), the $NAME/ token parser. The rulings say it is not changed and the two meanings of root stay apart.
- `scripts/design/SOURCE.md` — States that the scripts are copies of the method at commit 3c3bac7, with two named line differences. R1 adds no lys-local resolver, and R2 refreshes these copies from the new method commit and updates this file.
- `scripts/design/gate.sh` — The design leg. It validates docs/design/project.json and every cluster that has a design.json. The new docs/design/roots cluster will be measured by it (validate, check-coverage, render comparison).
- `docs/design/lys-core/` — Holds only CHECKLIST.md, DESIGN.md and USER-STORIES.md, with no design.json. The rulings say it is left exactly as it is, because it belongs to the ast-grep leg card.
- `docs/design/roadmap.json and docs/design/decisions.json` — The ROOTS-001 roadmap row and any decision take the next free id after the highest on main and on every open brief/* and draft/* branch.

### What was already decided

- Tom 2026-09-23 19:23 rule (CLAUDE.md, How a carded row is built, rule 5) — Workflow inputs name a repository, a commit, a card and a brief, never a folder. This is the rule the absolute roots break.
- documents.py root-token rule (method) — A root token's directory comes from launch settings (DS2_<NAME>). It is never read from a file in the tree or guessed, and it is refused by name. The rulings keep this apart from the roots map.
- SOURCE.md (scripts/design) — lys's scripts are copies of the method at 3c3bac7 with two stated differences, so resolver logic arrives only by refreshing from the method.
- ADR-004 — Every project stands alone. A lys survey must not depend on another project's folder being present on the machine.
- ADR-009 — This is the precedent for pinning an exact commit of another repository and moving the pin only in its own gated row, the same discipline the rulings set for a root's commit.
- directory CN5 / home CN5 — In main's clusters, CN5 is 'a live demonstration is never an acceptance criterion' (directory) and 'the pass-through proxy forwards unchanged' (home). Neither says the scripts stay verbatim copies of the method, which the ruling attributes to CN5.
- CLAUDE.md 'Wire formats are forever' / 'A test needs a second party' — The new root shape is a contract the method's reader and lys's document must both honour. R2's check, running the method's reader at a named commit against lys's project.json, is the second party.

### What was measured

- Roots in lys docs/design/project.json: 5 (aion, argus, cambium, haematite, method), all absolute paths under /Users/tom/Developer/ablative, lines 55-59
- Origin URLs of the five root repositories: aion https://github.com/ablative-io/aion.git; argus https://github.com/ablative-io/argus.git; cambium https://github.com/ablative-io/cambium.git; haematite https://github.com/ablative-io/haematite.git; method https://github.com/ablative-io/design-system.git
- HEAD of each local checkout on this Mac at survey time: aion f143dd73173ff1673a0108f38d822d0abaf386da; argus 424e7789be282d097a9ea3d6f02062b9a30c5379; cambium 1be80d8ec9cfbad84cc222d3430fba5f1459f2df; haematite 27ec726bc0722ab405e3b3a48a8eb1fd212b34b2; design-system 3c3bac715fb60348017f0d5cf40b412cf168da77
- Method commit lys's scripts are copied from, compared with the method's HEAD: Both are 3c3bac7, so the method has no commit yet that accepts the new shape
- Occurrences of DS2_CHECKOUTS in the design-system tree: 0
- Method workers/ds2_ledger/project.py size: 155 lines; _roots at 111-127
- Method tests for the project reader: 1 file, workers/ds2_ledger/tests/test_project.py
- lys scripts/design Python: 1518 lines across 4 scripts, plus roots.py (44 lines, identical to the method's)
- Value constraint on roots in lys's project.schema.json: none: 'type: object' only, with no value schema
- Other project.json files with absolute roots: 4: aion (6 roots), cambium (8 roots, including SEATS and CAMBIUM_LOCAL), haematite (4 roots), design-system (4 roots); argus has no docs/design/project.json
- Stale meridian method path: aion and haematite both name method as /Users/tom/Developer/projects/deno_rust/meridian/.meridian/design-system-v2
- Highest roadmap and decision ids on main (7b53625): RM-016, ADR-018
- Open brief/* and draft/* refs plus main on origin: 73 refs; only 14 of their commits are present locally and 59 are missing, so their highest ids could not be read without a fetch
- docs/design/roots exists: no
- docs/design/lys-core contents: 3 files (CHECKLIST.md, DESIGN.md, USER-STORIES.md), no design.json

### What it means for the other projects

- method — The design-system board gets a card written from R1's contract. It changes workers/ds2_ledger/project.py _roots to accept {repository, commit}, adds the DS2_CHECKOUTS resolver with its by-name refusal (git cat-file -e), updates root_lines and the schema, adds tests in test_project.py, and adds the pin-versus-origin-head check. Its own project.json (4 absolute roots) is a further unit on its board.
- aion — No change from this card. Its project.json has 6 absolute roots, including the stale meridian method path, and becomes its lead's card once the method accepts the new shape. As a root of lys, it is pinned by URL and commit in R2.
- cambium — No change from this card. Its project.json has 8 roots, including SEATS and CAMBIUM_LOCAL, which are not repositories, and becomes its lead's card. It is pinned as a root of lys in R2.
- haematite — No change from this card. Its project.json has 4 absolute roots, including the stale meridian method path, and becomes its lead's card. It is pinned as a root of lys in R2.
- argus — It has no docs/design/project.json, so it has nothing to convert. It is pinned only as a root of lys in R2.

### The decisions it stands on

- ADR-004 (honour) — Resolving through the worker's own checkout by URL and commit keeps lys's documents free of any other project's folder, so each project stands alone.
- ADR-009 (honour) — It is the same pinning discipline: an exact commit that moves only in its own gated row, never automatically.
-  (new) — A root in project.json is a repository URL plus a full commit, resolved through DS2_CHECKOUTS/<repo-name> and never fetched. The pin moves by hand in a gated row. This proposes a decision under the next free ADR id.

### What it requires

- docs/design/roots exists as a new cluster with a design.json, and the ROOTS-001 brief passes validate.py, check-coverage.py and the render comparison in scripts/design/gate.sh.
- The ROOTS-001 brief records the five absolute roots at project.json lines 55-59 verbatim as the baseline.
- The brief states the root contract: each root is an object with repository (URL) and commit (full 40-hex hash) and nothing else.
- The brief states the resolver: under DS2_CHECKOUTS, the checkout named by the last URL segment without .git must hold the commit (git cat-file -e). Otherwise it refuses by name, naming the checkout path, the repository and the commit, with the act 'fetch that commit into that checkout'. It never fetches or clones.
- The brief states the pin-versus-origin-head check, which reads origin from the same checkout, prints the distance per root and exits 0 either way.
- R2 is recorded as blocked on a named method commit, with the command that runs the method's reader at that commit against lys's project.json and exits 0.
- R2's first act is to refresh scripts/design from that method commit and update SOURCE.md.
- The brief records the two meanings of root ($NAME/ tokens versus the roots map) as an open point with both readings.
- The four other project.json files are listed under further units, with the stale meridian method path in aion and haematite named as a finding.
- The roadmap row and any decision use ids above the highest on main and on every open brief/* and draft/* branch.

### What must not change

- R1 does not change docs/design/project.json.
- docs/design/lys-core is untouched: no file there is renamed, rendered or added.
- scripts/design/* stays a verbatim copy of the method, with no lys-local resolver.
- roots.py and the $NAME/ token resolution in documents.py are unchanged.
- The aion, cambium, haematite and design-system project.json files do not change under this card.
- No automatic mover for a root's commit, and no per-design pin.
- No Rust crate or wire format changes.

### What we must put in place first

- Before writing, read the highest RM and ADR ids across all 73 origin refs. 59 are not present locally, so they must be read via ls-remote and fetch or the forge API.
- R2 only: a design-system commit, landed through that board's chain, whose project.py accepts the new root shape and carries the DS2_CHECKOUTS resolver.

### The risks

- Committing the new shape before the method accepts it makes project.py._text refuse roots/<name>, which stops every survey on lys. lys's validate.py would not catch this, because the schema leaves values unconstrained.
- The ruling's 'CN5' does not match either CN5 on main, so the brief could cite a constraint that says something else.
- The id collision check may miss ids on the 59 branches whose commits are not local.
- Cambium's roots SEATS and CAMBIUM_LOCAL are not repositories, so the URL-plus-commit contract has no answer for them. This surfaces in cambium's own card.
- Two checkouts whose URLs share a last segment (different owners, same repository name) would collide under DS2_CHECKOUTS.
- A pinned commit that is never moved makes surveys read a stale estate. The check only makes this visible; it does not correct it.

### The units beyond the first

- Method: project.py accepts {repository, commit} roots and resolves them through DS2_CHECKOUTS (design-system board) — The resolver lives only in the method, and this is its own card on another board, written from R1's contract.
- ROOTS-001 R2: refresh lys's scripts/design from the method commit and move lys's project.json roots to URL plus commit — It is blocked until the method accepts the shape. Doing it earlier stops every survey on lys.
- aion project.json roots to URL plus commit, fixing the stale meridian method path — This is aion's lead's card on its own board, fired once the method accepts the shape.
- haematite project.json roots to URL plus commit, fixing the stale meridian method path — This is haematite's lead's card on its own board.
- cambium project.json roots to URL plus commit, deciding what SEATS and CAMBIUM_LOCAL become — This is cambium's lead's card, and two of its roots are not repositories.
- design-system project.json roots to URL plus commit — This is the method's own lead's card.
- Resolve the two meanings of root ($NAME/ launch tokens versus the project.json roots map) — It is recorded as an open point. Unifying the two changes how every brief path outside the repository resolves, which is outside these words.

### The smallest complete shape

ROOTS-001 R1: a documents-only change on lys that adds the docs/design/roots cluster (design.json, rendered markdown and the ROOTS-001 brief), a roadmap row, and optionally one proposed decision. It records the five-root baseline, the {repository, commit} contract, the DS2_CHECKOUTS resolver and its refusal, and the pin-distance check. It names R2 as blocked on the method commit, together with the reader command that must exit 0. It passes sh scripts/design/gate.sh and leaves project.json and scripts/design untouched.

## The roadmap row

- **RM-028** — Name each root in lys's project.json by repository URL and pinned commit, resolved through the worker's own checkout (fix, idea)
- Summary: lys's docs/design/project.json names aion, argus, cambium, haematite and the method by absolute directories on one machine, which no other worker resolves. ROOTS-001 records the baseline and the contract in a new docs/design/roots cluster (R1, documents only): each root an object with a repository URL and the full commit the design was checked against, resolved through DS2_CHECKOUTS/<repository name> with git cat-file -e, refused by name and never fetched, a pin check that prints each pin's distance from origin's head and exits 0, or names pin_absent or origin_head_unset and exits 1 after reporting every root, and the method's repositories map retired into root.repository. R2 moves lys's five roots to that shape once a design-system commit's reader accepts it, refreshing lys's method copies from that commit first.
- Asked by: tom on 2026-09-27T11:30:00+10:00
- Context: Defect 8 of the estate audit, carded on the Lys board as the roots card. The lead for lys ruled on the rounds of runs a31929b9 and fe5dd7f7: the brief in two parts (R1 documents only, R2 blocked on the method commit), a resolver that never fetches, one launch setting DS2_CHECKOUTS, pins moved by hand with one check and no mover, the $NAME/ tokens kept apart from the roots map, only lys's project.json changing, the new docs/design/roots cluster with lys-core untouched, and the resolver living only in the method.
- Quote: The file docs/design/project.json at lines 54 to 59 (roots) names aion, argus, cambium, haematite and the method by absolute paths under /Users/tom/Developer/ablative. Any worker on another machine (Dean's laptop, Annabel) resolves none of them, and Tom's 2026-09-23 19:23 rule says workflow inputs name a repository, a commit, a card and a brief, never a folder. Make each root a repository URL plus the commit the design was checked against, and have scripts/design/*.py resolve a root through the worker's own checkout of that repository rather than a path on this Mac. Defect 8 of Waffles' 2026-09-25 22:35 audit.
- Cluster: roots; briefs: ROOTS-001
- Notes: Ids: checked with git ls-remote against origin's main (7b53625) and all 75 open brief/* and draft/* branches immediately before writing, every ref's roadmap.json and decisions.json read after fetching its commit; the highest were RM-027 and ADR-041 (draft/rauthy-rebase/be0c979d), so this row is RM-028 and the decision ADR-042. Further units, not written: Method: project.py accepts {repository, commit} roots and resolves them through DS2_CHECKOUTS (design-system board); aion project.json roots to URL plus commit, fixing the stale meridian method path; haematite project.json roots to URL plus commit, fixing the stale meridian method path; cambium project.json roots to URL plus commit, deciding what SEATS and CAMBIUM_LOCAL become; design-system project.json roots to URL plus commit; Resolve the two meanings of root ($NAME/ launch tokens versus the project.json roots map). Finding: aion and haematite each name their method root as /Users/tom/Developer/projects/deno_rust/meridian/.meridian/design-system-v2, the retired meridian copy, not the design-system repository. The method's origin main (25827d8) reads an optional repositories map (name to remote URL) beside roots; the contract retires it, the method's card moving every entry into its root.repository and refusing by name a project.json that still carries the map. Re-checked for this round against 79 refs (main and every open brief/* and draft/* branch), every commit fetched and read: RM-028 and ADR-042 appear only on this draft's branch, and the one higher id, RM-029 on draft/directory/28d15c0d, was taken after this row. Re-checked for the second round against 90 refs, every commit fetched and read: ADR-042 appears only on this draft's branch; RM-028 also appears on brief/home/a3186728 (9a5e50e, 12:38 on 27 September), a different row on moving the home record's logic out of record/mod.rs, written 23 minutes after this draft's 868eb04 (12:15) had taken RM-028. This row keeps RM-028 as the earlier claim; the collision is a finding for the home card, whose row takes the next free id, RM-031 as of this check (the highest elsewhere being RM-030 and ADR-045). Re-checked for the third round against 97 refs (main and every open brief/* and draft/* branch), every commit fetched and read: ADR-042 still appears only on this draft's branch, RM-028 still also on brief/home/a3186728, and the highest elsewhere are RM-032 and ADR-050, so the home card's row now takes RM-033.

## The design

---
type: design
cluster: roots
title: A root is a repository and a pinned commit, never a folder on one machine
---

# A root is a repository and a pinned commit, never a folder on one machine

> **Cluster:** roots

## Intention

A survey on any worker reads the same estate. Each other project in lys's project.json is named by where anyone can fetch it and by the exact commit the design was checked against, and a worker reaches it through its own checkout of that repository. When the checkout does not hold the commit, the worker is told by name what is missing and what answers it; nothing is fetched or cloned behind anyone's back. A pin that has fallen behind is a fact anyone can print, not a drift nobody sees.

## Problem

The roots map in docs/design/project.json names aion, argus, cambium, haematite and the method by absolute directories on one machine. A worker on any other machine resolves none of them, and the carded-row rule is that workflow inputs name a repository, a commit, a card and a brief, never a folder. The shape cannot simply be rewritten today: the method's reader (workers/ds2_ledger/project.py, _roots) requires every root to be a string naming an absolute directory and refuses anything else, so a project.json in the new shape would stop every survey on lys. lys's own copy of project.schema.json types roots only as an object, so validate.py and the design gate would not catch that change before it landed.

## Solution

The work is one brief, ROOTS-001, in two requirements. R1 is documents only on lys: this cluster records the baseline (the five absolute roots, in the inventory, exactly as project.json spells them) and the contract below, and its rendered markdown lands so scripts/design/gate.sh measures it. R2 changes lys's project.json to the new shape and is blocked until a design-system commit, landed through that board's own chain from this contract, makes workers/ds2_ledger/project.py accept the shape and carry the resolver. R2's first act is to refresh lys's copies of the method from that commit and name it in scripts/design/SOURCE.md; only then does it rewrite the roots.

The root contract. Each value of the roots map is an object with exactly two fields: repository, the remote URL the project is fetched from, and commit, the full 40-character hexadecimal hash of the commit the design was checked against. The method's project.schema.json types each roots value that way, with both fields required and no other field allowed. There is one pin per root, in project.json, and no pin per design. The repositories map (project name to remote URL) that the method reads beside roots is retired, so a root's repository has one home: the root object's own repository is the only source, the method's card moves every existing repositories entry into its root in the same change, and the method's validator then refuses a project.json that still carries a repositories map, by name, naming the move to root.repository as the act that answers it. The pins R2 writes are the origin heads this design was checked against: aion: repository https://github.com/ablative-io/aion.git, commit 5594f7fa0081f9cec2c84ef9a30ad36599bfbdd9; argus: repository https://github.com/ablative-io/argus.git, commit dc2397c92ecf03e420aaa5a72b10cad673386e49; cambium: repository https://github.com/ablative-io/cambium.git, commit 105c03ded311b0ea5de9463eaab27320b2649004; haematite: repository https://github.com/ablative-io/haematite.git, commit 36536826f4d04fc10037be49c1c2c9f1d073d459; method: repository https://github.com/ablative-io/design-system.git, commit 25827d8d5d747b13039901e8d9deed755735c6fb.

The resolver lives in the method, in workers/ds2_ledger/project.py, as resolve_root. A worker names one launch setting, DS2_CHECKOUTS, a directory holding one checkout per repository. The checkout for a root is the directory under DS2_CHECKOUTS named by the last path segment of the root's repository URL with a trailing .git removed, so https://github.com/ablative-io/design-system.git is DS2_CHECKOUTS/design-system. The resolver requires `git -C <checkout> cat-file -e <commit>` to exit 0 and returns the checkout. Otherwise it refuses by name with RootUnresolved, naming the checkout path it looked in, the repository and the commit, and saying the act that answers it: fetch that commit into the worker's checkout of that repository. When DS2_CHECKOUTS is unset it refuses by name, as documents.py refuses an unset root token. It never fetches and never clones. It is not documents.py and does not read the $NAME/ tokens.

The pin check lives in the method as scripts/check-root-pins.py. It reads project.json, and for each root in name order reads origin's head from the same checkout (refs/remotes/origin/HEAD, as last fetched; the check fetches nothing) and prints one line: `<name> <repository> pin <commit> origin <head> distance <n>`, where n is `git rev-list --count <commit>..refs/remotes/origin/HEAD` in that checkout. When every root's checkout holds its pin and has refs/remotes/origin/HEAD, it exits 0 whatever the distance. A root whose checkout does not hold its pin is not a distance: its line is `<name> <repository> pin <commit> pin_absent act: git -C <checkout> fetch origin`, naming the refusal pin_absent and the act that answers it, fetching that root's remote. A root whose checkout holds its pin but has no refs/remotes/origin/HEAD is not a distance either: its line is `<name> <repository> pin <commit> origin_head_unset act: git -C <checkout> remote set-head origin --auto`. Either refusal makes the check exit 1, and only after it has printed a line for every root, so one bad root hides nothing. It moves nothing: a root's commit moves by hand, in a gated row whose commit says the design was re-checked against the newer tree (ADR-009's discipline, ADR-042).

lys holds copies only (CN5), and a copy must run where it lands. At 25827d8 the method's workers/ds2_ledger/project.py imports aion_worker, the worker SDK, for TerminalError, and imports venues, which itself imports aion_worker and spawn; lys carries none of them, so a copy of that file would not import in lys. The contract therefore bounds what the resolver and the pin check depend on. At the method commit R2 refreshes from, workers/ds2_ledger/project.py imports only the Python standard library and, within ds2_ledger, legs.py and roots.py. legs.py is a new method module holding the leg-entry reading project.py takes from venues today (LEG_ENTRY_FIELDS, declares_legs, legs_of_entry and LegRefused), and imports only the standard library. Neither module imports aion_worker or any package outside the standard library. ProjectRefused and RootUnresolved derive from a standard-library exception, and the worker turns them into its TerminalError where it calls the reader, outside project.py. scripts/check-root-pins.py imports only the standard library and ds2_ledger.project, and finds workers/ by the line check-coverage.py uses, `sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "workers"))`. After R2 refreshes them, lys's scripts/design/workers/ds2_ledger/project.py and legs.py are byte-identical copies of the method's files at the named commit, and scripts/design/check-root-pins.py differs from the method's in that one line only, reading `.parent / "workers"` because lys keeps workers/ beside its scripts, as it already does for check-coverage.py; SOURCE.md names that line beside the two it names today. So `python3 -I` from a fresh clone of lys, with no worker SDK installed, resolves a root and runs the pin check through the worker's own checkout. Before R2 lands, a person runs the method's reader from that commit against lys's project.json, which is R2's check.

Two meanings of root, recorded open. The $NAME/ tokens a structure path or gate tree is spelled under are launch settings, DS2_<NAME>, never read from a file in the tree (documents.py, roots.py). The roots map in project.json is the estate's other projects, read by a survey. Reading one: they stay two things, as this card leaves them. Reading two: one map serves both, so a $NAME/ token resolves through the pinned checkout of the root of that name. The second changes how every brief path outside the repository resolves; a later card decides it, and this card changes neither the tokens nor roots.py (CN3).

## Principles

- **P1** — A root names where anyone can fetch the project and the commit the design was checked against; never a folder on one machine.
- **P2** — What a worker lacks is refused by name, with the path, the repository, the commit and the act that answers it; nothing is fetched, cloned or guessed.
- **P3** — A pin moves by hand in a gated row and never automatically; a survey reads the pin, never the moving head, so staleness is visible rather than silent.
- **P4** — The resolver is the method's; lys receives it only by refreshing its copies from a named method commit.
- **P5** — The document never runs ahead of its reader: the shape changes only after the reader that must accept it has landed.

## Decisions

- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-042 — A root is a repository URL and a pinned commit, resolved through the worker's own checkout and never fetched — Each root in project.json is an object with repository, a remote URL, and commit, the full hash of the commit the design was checked against. A worker resolves it through DS2_CHECKOUTS/<last URL segment without .git>, which must hold the commit (git cat-file -e), and otherwise is refused by name with the checkout path, the repository, the commit and the act of fetching that commit into that checkout. The repositories map (project name to remote URL) is retired into root.repository, so a root's repository has one home, and the method's validator refuses a project.json that still carries it, by name, naming the move to root.repository. The pin moves by hand in a gated row. A check prints each root's pin beside origin's head and the distance between them, and exits 0 whatever the distance; a root whose checkout lacks its pin is reported as pin_absent with the act git -C <checkout> fetch origin, a root whose checkout has no refs/remotes/origin/HEAD is reported as origin_head_unset with the act git -C <checkout> remote set-head origin --auto, and either makes the check exit 1, only after it has reported every root. The resolver and the check import only the Python standard library and SDK-free modules of ds2_ledger, so lys's copies run under python3 from a fresh clone. Rejected: keeping absolute directories; a resolver that fetches or clones what is missing; following the moving head or moving pins automatically; a pin per design; a lys-local resolver; and unifying the roots map with the $NAME/ launch tokens in this card.

## Goals

- docs/design/roots renders, validates and passes check-coverage under scripts/design/gate.sh, with docs/design/project.json unchanged by R1.
- The design records all five baseline roots, byte for byte as project.json spells them before this card.
- After R2, the method's reader at the named method commit reads lys's docs/design/project.json and exits 0, every roots value is an object carrying a repository URL and a 40-character commit, and lys's own copies of the resolver and the pin check run under python3 -I from a fresh clone with no worker SDK installed.
- After R2, lys's copy of the pin check prints one line per root with its pin, origin's head and the distance, and exits 0 when every pin and origin head is present; a root whose checkout lacks its pin is named pin_absent, a root with no origin head is named origin_head_unset, and either makes it exit 1 after every root is reported.

## Non-Goals

- Changing the project.json of aion, cambium, haematite or the design system — Each is its own lead's card on its own board, fired once the method accepts the new shape.
- Unifying the $NAME/ launch tokens with the roots map — It would change how every brief path outside the repository resolves; recorded as an open point for a later card.
- An automatic mover for a root's commit, or a pin per design — A pin moves by hand in a gated row; one pin per root in project.json.
- A lys-local resolver script — The resolver is written in the method and lys receives it by refreshing its copies (CN5).
- Fetching or cloning a missing repository or commit — A worker is told what to fetch; it is never done for it, so no survey quietly clones five repositories.
- Writing the method's change — It is its own card on the design-system board, written from this contract.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/roots/design.json` | this design: the baseline, the root contract, the resolver and the pin check | ROOTS-001 |
| `docs/design/roots/DESIGN.md` | rendered design | ROOTS-001 |
| `docs/design/roots/checklist.json` | checklist | ROOTS-001 |
| `docs/design/roots/CHECKLIST.md` | rendered checklist | ROOTS-001 |
| `docs/design/roots/stories.json` | stories | ROOTS-001 |
| `docs/design/roots/USER-STORIES.md` | rendered stories | ROOTS-001 |
| `docs/design/roots/briefs/ROOTS-001.json` | the brief: R1 records the contract, R2 moves lys's roots once the method accepts them | ROOTS-001 |
| `docs/design/roots/briefs/ROOTS-001.md` | rendered brief | ROOTS-001 |
| `docs/design/project.json` | lys's setup; R2 rewrites its roots map to the object shape and changes nothing else in it |  |
| `scripts/design/SOURCE.md` | names the method commit the copies come from and the lines that differ; R2 names the new commit and the pin check's workers/ line |  |
| `scripts/design/validate.py` | copy of the method's scripts/validate.py; refreshed by R2 |  |
| `scripts/design/check-coverage.py` | copy of the method's scripts/check-coverage.py; refreshed by R2 |  |
| `scripts/design/render-cluster.py` | copy of the method's scripts/render-cluster.py; refreshed by R2 |  |
| `scripts/design/render-brief.py` | copy of the method's scripts/render-brief.py; refreshed by R2 |  |
| `scripts/design/schemas` | copies of the method's schemas, project.schema.json among them; refreshed by R2 |  |
| `scripts/design/workers/ds2_ledger/roots.py` | copy of the method's $NAME/ token parser; unchanged by this card (CN3) |  |
| `scripts/design/workers/ds2_ledger/project.py` | copy of the method's project reader carrying resolve_root, byte-identical at the named commit; imports only the standard library, legs.py and roots.py; added by R2's refresh | ROOTS-001 |
| `scripts/design/workers/ds2_ledger/legs.py` | copy of the method's SDK-free leg-entry reader that project.py imports in place of venues; byte-identical at the named commit; added by R2's refresh | ROOTS-001 |
| `scripts/design/check-root-pins.py` | copy of the method's pin check; differs from it only in the line that finds workers/ beside the script, which SOURCE.md names; added by R2's refresh | ROOTS-001 |

## Inventory

- `docs/design/project.json` — The baseline, lines 54 to 59 at main 7b53625 (the map's closing brace is line 60): "roots": {"aion": "/Users/tom/Developer/ablative/stack/aion", "argus": "/Users/tom/Developer/ablative/tools/argus", "cambium": "/Users/tom/Developer/ablative/apps/cambium", "haematite": "/Users/tom/Developer/ablative/stack/haematite", "method": "/Users/tom/Developer/ablative/tools/design-system"}. Five string values, each an absolute directory on one machine.
- `scripts/design/schemas/project.schema.json` — Types roots as "type": "object" with no value schema; each value is described as an absolute directory checked by the ledger, so validate.py accepts either shape.
- `scripts/design/SOURCE.md` — The copies are the method at commit 3c3bac7, with two stated line differences. The method's origin main is 25827d8, where workers/ds2_ledger/project.py _roots (lines 126 to 141) still requires each root to be a non-empty string naming an absolute directory, and refuses an object with 'is not a non-empty string'. At 25827d8 the method also reads an optional repositories map, name to remote URL, beside roots, which the contract retires; lys's project.json carries none. DS2_CHECKOUTS occurs nowhere in the method. At 25827d8 project.py imports aion_worker (TerminalError) and venues, and venues imports aion_worker and spawn; none of the three is in lys. Against 25827d8 the copies have drifted beyond the two stated lines, measured with `git -C <method> show 25827d8:scripts/validate.py | diff - scripts/design/validate.py` and `git -C <method> archive 25827d8 schemas | tar -x -C <empty directory>` followed by `diff -r <empty directory>/schemas scripts/design/schemas`: validate.py differs in the stated SCHEMA_DIR line and in four further hunks, the minimum check at 25827d8 lines 143 to 149, the docstring at lines 244 to 245, and the requirements and files handling at lines 259 to 263 and 265 to 268; brief.schema.json at 25827d8 carries shared_rows, and project.schema.json at 25827d8 carries repositories, gate_host and gate_needs, none of which lys's copies carry. Only check-coverage.py, render-cluster.py, render-brief.py and workers/ds2_ledger/__init__.py match 25827d8 apart from the stated line: check-coverage.py differs only in its stated sys.path line, and the other three are byte-identical.
- `scripts/design/workers/ds2_ledger/roots.py` — 44 lines, identical to the method's: the $NAME/ root-token parser. Tokens resolve only from DS2_<NAME> launch settings (the method's documents.py), refused by name when unset.
- `scripts/design/gate.sh` — The design leg: validates decisions.json and project.json, and for every cluster with a design.json runs validate.py, check-coverage.py and a render comparison against the committed markdown.
- `docs/design/lys-core` — CHECKLIST.md, DESIGN.md and USER-STORIES.md with no design.json; owned by the ast-grep leg card and untouched here (CN2).

## Constraints

- **CN1** — docs/design/project.json does not change shape while the method's reader refuses the new shape: R1 leaves it byte-identical, and R2 changes it only after its copies are refreshed from a method commit whose reader accepts it.
- **CN2** — docs/design/lys-core is left exactly as it is: no file there is renamed, rendered or added.
- **CN3** — The $NAME/ root tokens and the roots map stay apart: scripts/design/workers/ds2_ledger/roots.py and the token resolution are unchanged.
- **CN4** — Only lys's project.json changes; the project.json of aion, cambium, haematite and the design system do not change under this card.
- **CN5** — lys's scripts under scripts/design stay verbatim copies of the method as scripts/design/SOURCE.md says; no lys-local resolver script is written.
- **CN6** — No automatic mover for a root's commit and no per-design pin: one pin per root, moved by hand in a gated row.
- **CN7** — No Rust crate and no wire format changes.


---
type: brief
id: ROOTS-001
cluster: roots
title: Record the root contract, then move lys's roots to a repository URL and a pinned commit once the method reads them
---

# ROOTS-001: Record the root contract, then move lys's roots to a repository URL and a pinned commit once the method reads them

> **Cluster:** roots
> **Blocked by:** R2 only: a design-system commit, landed through that board's own chain from this cluster's contract, whose workers/ds2_ledger/project.py accepts the root object {repository, commit}, carries resolve_root over DS2_CHECKOUTS and imports only the standard library, legs.py and roots.py, whose workers/ds2_ledger/legs.py imports only the standard library, and whose scripts/check-root-pins.py is the pin check. The check a stranger runs to see it lifted: with "$DS2_CHECKOUTS/design-system" checked out at that commit and lys's docs/design/project.json in the new shape, `python3 -I -c "import sys; sys.path.insert(0, sys.argv[2] + '/workers'); from ds2_ledger.project import read_setup; read_setup(sys.argv[1])" "$PWD/docs/design" "$DS2_CHECKOUTS/design-system"`, run from lys's repository root with no worker SDK installed, exits 0. R1 is not blocked by it.
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-042 — A root is a repository URL and a pinned commit, resolved through the worker's own checkout and never fetched — Each root in project.json is an object with repository, a remote URL, and commit, the full hash of the commit the design was checked against. A worker resolves it through DS2_CHECKOUTS/<last URL segment without .git>, which must hold the commit (git cat-file -e), and otherwise is refused by name with the checkout path, the repository, the commit and the act of fetching that commit into that checkout. The repositories map (project name to remote URL) is retired into root.repository, so a root's repository has one home, and the method's validator refuses a project.json that still carries it, by name, naming the move to root.repository. The pin moves by hand in a gated row. A check prints each root's pin beside origin's head and the distance between them, and exits 0 whatever the distance; a root whose checkout lacks its pin is reported as pin_absent with the act git -C <checkout> fetch origin, a root whose checkout has no refs/remotes/origin/HEAD is reported as origin_head_unset with the act git -C <checkout> remote set-head origin --auto, and either makes the check exit 1, only after it has reported every root. The resolver and the check import only the Python standard library and SDK-free modules of ds2_ledger, so lys's copies run under python3 from a fresh clone. Rejected: keeping absolute directories; a resolver that fetches or clones what is missing; following the moving head or moving pins automatically; a pin per design; a lys-local resolver; and unifying the roots map with the $NAME/ launch tokens in this card.
> **Checklist:**
> - C1 — The cluster records, as the baseline, the five roots of docs/design/project.json before this card, each name and absolute directory exactly as the file spells them.
> - C2 — The cluster records the root contract: each roots value is an object with exactly two fields, repository, a remote URL, and commit, a full 40-character hexadecimal commit hash; project.schema.json types each value that way with both fields required and no other field; one pin per root and no pin per design; the repositories map retired, its entries moved into root.repository and a project.json still carrying it refused by name; and the five pins the design was checked against.
> - C3 — The cluster records the resolver: one launch setting, DS2_CHECKOUTS; the checkout named by the last path segment of the repository URL without .git; git cat-file -e on the commit in that checkout; a refusal by name naming the checkout path, the repository and the commit, with the act of fetching that commit into that checkout; never a fetch and never a clone; and what the resolver and the pin check may import: project.py only the standard library, legs.py and roots.py, legs.py only the standard library, the pin check only the standard library and ds2_ledger.project, none of them aion_worker, and the pin check finding workers/ by the line check-coverage.py uses.
> - C4 — The cluster records the pin check: each root's pin beside origin's head read from the same checkout, the distance printed, exit 0 whatever the distance when every pin and origin head is present; a root whose checkout lacks its pin refused as pin_absent with the act of fetching that root's remote, a root with no refs/remotes/origin/HEAD refused as origin_head_unset with the act git remote set-head origin --auto, either making it exit 1 after every root is reported; and that a pin moves only by hand in a gated row, with no mover.
> - C5 — The cluster records the two meanings of root, the $NAME/ launch tokens and the roots map, as an open point with both readings, and changes neither.
> - C6 — The cluster's rendered markdown matches its JSON, scripts/design/gate.sh exits 0, and docs/design/project.json, scripts/design and docs/design/lys-core are unchanged by R1.
> - C7 — lys's copies of the method are refreshed from the method commit whose reader accepts the root object, before project.json changes; scripts/design/SOURCE.md names that commit; project.py, legs.py, render-cluster.py and render-brief.py are byte-identical to the method's, validate.py, check-coverage.py and check-root-pins.py differ only in the one line each that SOURCE.md names, and the schemas match.
> - C8 — docs/design/project.json holds the five roots as objects carrying the recorded repository and commit; the method's reader at the named commit reads it and exits 0; and lys's own copies, under python3 -I, resolve a root, refuse a missing checkout by name, and print the pin lines with both refusals.
> **Stories:**
> - S1 (Worker, Runs a survey or a round on a machine other than the one the design was written on) — As a worker, I want each root resolved through my own checkout of its repository at the pinned commit, so that a survey reads the same estate on any machine.
> - S2 (Worker, Runs a survey or a round on a machine other than the one the design was written on) — As a worker whose checkout lacks a pinned commit, I want a refusal naming the checkout path, the repository, the commit and the fetch that answers it, so that nothing is cloned or fetched without my knowing.
> - S3 (Method lead, Writes the design-system card that makes the method's reader accept the new shape) — As the method's lead, I want the root contract, the resolver and the pin check written down in lys before the method changes, so that the method's card is written from a contract rather than from a guess.
> - S4 (Reviewer, Decides whether a root's pin should be moved) — As a reviewer, I want each root's pin printed beside origin's head with the distance between them, and a root with no pin or no origin head named rather than skipped, so that a stale pin is a visible fact and moving it is a decision someone makes.

## Purpose

lys's project.json names the estate's other projects by folders on one machine, which no other worker can resolve. This brief first writes down, in lys, the shape that replaces them and how a worker resolves it (R1, documents only, so the method's card can be written from it), and then, once the method's reader accepts that shape, moves lys's five roots to it (R2). The design holds the contract; this brief delivers it in the order that never leaves lys's project.json ahead of the reader that must read it.

## Task

R1: render this cluster's markdown from its JSON with scripts/design/render-cluster.py so that scripts/design/gate.sh measures it clean. The JSON already carries the baseline (design inventory) and the contract (design solution, principles and constraints); R1 changes no JSON and touches nothing outside docs/design/roots. R2 waits for the method commit named under blocked_by. Its first act is to refresh lys's copies of the method from that commit (every copy scripts/design/SOURCE.md names except workers/ds2_ledger/roots.py, which stays as it is, plus the three new copies the design's structure names: workers/ds2_ledger/project.py, workers/ds2_ledger/legs.py and check-root-pins.py) and to name the commit in SOURCE.md; only after that does it rewrite the roots map of docs/design/project.json to the five objects the design records, changing nothing else in the file. The method's change itself is not in this brief: it is its own card on the design-system board. Units not written here, each its own card: the method's project.py accepting and resolving the root object (design-system board); the project.json roots of aion, of haematite, of cambium (whose SEATS and CAMBIUM_LOCAL roots are not repositories) and of the design system, each on its own lead's board once the method accepts the shape; and resolving the two meanings of root, an open point with two readings: the $NAME/ tokens stay launch settings apart from the roots map, as this brief leaves them; or one map serves both, so a $NAME/ token resolves through the pinned checkout of the root of that name, which would change how every brief path outside the repository resolves. A finding for the aion and haematite cards: each names its method root as /Users/tom/Developer/projects/deno_rust/meridian/.meridian/design-system-v2, the retired meridian copy of the method, not the design-system repository. Every path in this brief is relative to the repository root.

## Requirements

### R1: Render the roots cluster so the baseline and the contract land as measured documents

The roots cluster's markdown (DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/ROOTS-001.md) is created by scripts/design/render-cluster.py from the cluster's JSON as it stands, so the recorded baseline, root contract, resolver, pin check with its two refusals, the bound on what the resolver and the pin check import, the retirement of the repositories map and the open point on the two meanings of root are rendered and measured by scripts/design/gate.sh. R1 SHALL NOT edit any JSON of the cluster, SHALL NOT change docs/design/project.json, anything under scripts/design, or anything under docs/design/lys-core, and SHALL NOT add a resolver or check of its own to lys.

**Acceptance:**
- `sh scripts/design/gate.sh` run from the repository root exits 0 and prints no line starting with `rendered markdown differs`.
- `git diff --name-only 7b53625 HEAD -- docs/design/project.json scripts/design docs/design/lys-core` prints nothing.
- For each of the five values of `roots` in `git show 7b53625:docs/design/project.json`, `grep -cF` of that value in docs/design/roots/DESIGN.md prints a count of at least 1.
- `grep -cF DS2_CHECKOUTS docs/design/roots/DESIGN.md`, `grep -cF 'cat-file -e' docs/design/roots/DESIGN.md`, `grep -cF 'rev-list --count' docs/design/roots/DESIGN.md`, `grep -cF pin_absent docs/design/roots/DESIGN.md`, `grep -cF origin_head_unset docs/design/roots/DESIGN.md`, `grep -cF root.repository docs/design/roots/DESIGN.md`, `grep -cF legs.py docs/design/roots/DESIGN.md` and `grep -cF aion_worker docs/design/roots/DESIGN.md` each print a count of at least 1.

**Files:**
- create: docs/design/roots/DESIGN.md
- create: docs/design/roots/CHECKLIST.md
- create: docs/design/roots/USER-STORIES.md
- create: docs/design/roots/briefs/ROOTS-001.md

**Checklist:**
- C1 — The cluster records, as the baseline, the five roots of docs/design/project.json before this card, each name and absolute directory exactly as the file spells them.
- C2 — The cluster records the root contract: each roots value is an object with exactly two fields, repository, a remote URL, and commit, a full 40-character hexadecimal commit hash; project.schema.json types each value that way with both fields required and no other field; one pin per root and no pin per design; the repositories map retired, its entries moved into root.repository and a project.json still carrying it refused by name; and the five pins the design was checked against.
- C3 — The cluster records the resolver: one launch setting, DS2_CHECKOUTS; the checkout named by the last path segment of the repository URL without .git; git cat-file -e on the commit in that checkout; a refusal by name naming the checkout path, the repository and the commit, with the act of fetching that commit into that checkout; never a fetch and never a clone; and what the resolver and the pin check may import: project.py only the standard library, legs.py and roots.py, legs.py only the standard library, the pin check only the standard library and ds2_ledger.project, none of them aion_worker, and the pin check finding workers/ by the line check-coverage.py uses.
- C4 — The cluster records the pin check: each root's pin beside origin's head read from the same checkout, the distance printed, exit 0 whatever the distance when every pin and origin head is present; a root whose checkout lacks its pin refused as pin_absent with the act of fetching that root's remote, a root with no refs/remotes/origin/HEAD refused as origin_head_unset with the act git remote set-head origin --auto, either making it exit 1 after every root is reported; and that a pin moves only by hand in a gated row, with no mover.
- C5 — The cluster records the two meanings of root, the $NAME/ launch tokens and the roots map, as an open point with both readings, and changes neither.
- C6 — The cluster's rendered markdown matches its JSON, scripts/design/gate.sh exits 0, and docs/design/project.json, scripts/design and docs/design/lys-core are unchanged by R1.

**Stories:**
- S3 (Method lead, Writes the design-system card that makes the method's reader accept the new shape) — As the method's lead, I want the root contract, the resolver and the pin check written down in lys before the method changes, so that the method's card is written from a contract rather than from a guess.

### R2: Refresh lys's method copies from the accepting commit, then write the five roots as repository and commit

WHEN a design-system commit C exists whose workers/ds2_ledger/project.py accepts the root object and carries resolve_root, THE SYSTEM SHALL first refresh scripts/design's copies of the method from C and name C in full in scripts/design/SOURCE.md, and SHALL then write each value of the roots map in docs/design/project.json as the object {repository, commit} the design records for that root. It SHALL NOT change docs/design/project.json before the refresh is committed, SHALL NOT change anything in project.json outside the roots map, SHALL NOT modify scripts/design/workers/ds2_ledger/roots.py, SHALL NOT make any copy differ from C beyond the lines SOURCE.md states (in validate.py the line finding schemas/, and in check-coverage.py and check-root-pins.py the line finding workers/, each beside the script rather than one directory up), SHALL NOT copy any module of C's workers/ds2_ledger other than project.py and legs.py beside the existing __init__.py and roots.py, SHALL NOT leave a copy that imports aion_worker or any package outside the Python standard library, and SHALL NOT add a pin mover, a per-design pin, or any step that fetches or clones. IF the checkout DS2_CHECKOUTS names for a root does not hold that root's commit, THEN the resolver received from C SHALL refuse by name, naming the checkout path, the repository and the commit and saying to fetch that commit into that checkout, and SHALL NOT create anything under DS2_CHECKOUTS. IF a root's checkout does not hold that root's commit, THEN the pin check received from C SHALL print that root's line as pin_absent with the act of fetching that root's remote; IF a root's checkout holds its commit but has no refs/remotes/origin/HEAD, THEN it SHALL print that root's line as origin_head_unset with the act git remote set-head origin --auto; in either case it SHALL exit 1, and SHALL NOT exit before it has printed a line for every root, SHALL NOT print a distance for a refused root, and SHALL NOT fetch or set anything in any checkout.

**Acceptance:**
- Setup for every line below: each command runs from the repository root on R2's branch after its last commit; C is the full commit hash scripts/design/SOURCE.md names after the refresh; DS2_CHECKOUTS is set to an absolute directory with no trailing slash that holds five checkouts named aion, argus, cambium, haematite and design-system, of https://github.com/ablative-io/aion.git, https://github.com/ablative-io/argus.git, https://github.com/ablative-io/cambium.git, https://github.com/ablative-io/haematite.git and https://github.com/ablative-io/design-system.git respectively; each checkout holds its root's pinned commit (aion 5594f7fa0081f9cec2c84ef9a30ad36599bfbdd9, argus dc2397c92ecf03e420aaa5a72b10cad673386e49, cambium 105c03ded311b0ea5de9463eaab27320b2649004, haematite 36536826f4d04fc10037be49c1c2c9f1d073d459, design-system 25827d8d5d747b13039901e8d9deed755735c6fb) and has refs/remotes/origin/HEAD set, and design-system also holds C with C checked out; M is "$DS2_CHECKOUTS/design-system"; E is an empty directory. The person measuring prepares that state before measuring, and the preparation is not a step of R2: for each checkout name r, `git -C "$DS2_CHECKOUTS/$r" remote set-head origin --auto` is run in every checkout where `git -C "$DS2_CHECKOUTS/$r" symbolic-ref -q refs/remotes/origin/HEAD` exits non-zero; `git -C "$DS2_CHECKOUTS/$r" fetch origin <pin>` is run in every checkout where `git -C "$DS2_CHECKOUTS/$r" cat-file -e <pin>^{commit}` exits non-zero, with <pin> that checkout's pinned commit; `git -C "$DS2_CHECKOUTS/design-system" fetch origin "$C"` is run when `git -C "$DS2_CHECKOUTS/design-system" cat-file -e "$C^{commit}"` exits non-zero; then `git -C "$DS2_CHECKOUTS/design-system" checkout --detach "$C"` is run. The state is ready when, for each r, `git -C "$DS2_CHECKOUTS/$r" symbolic-ref -q refs/remotes/origin/HEAD` and `git -C "$DS2_CHECKOUTS/$r" cat-file -e <pin>^{commit}` each exit 0, and `git -C "$DS2_CHECKOUTS/design-system" rev-parse HEAD` prints C.
- `git log --reverse --format= --name-only 7b53625..HEAD -- scripts/design/SOURCE.md docs/design/project.json | grep -v '^$'` prints `scripts/design/SOURCE.md` as its first line and prints `docs/design/project.json` on a later line, so the first commit changing SOURCE.md comes before, and is not, the first commit changing project.json.
- `grep -cF "$C" scripts/design/SOURCE.md` prints a count of at least 1, and `grep -cF check-root-pins.py scripts/design/SOURCE.md` and `grep -cF legs.py scripts/design/SOURCE.md` each print a count of at least 1.
- `python3 -c "import json; print(json.dumps(json.load(open('docs/design/project.json'))['roots'], sort_keys=True, separators=(',', ':')))"` prints exactly `{"aion":{"commit":"5594f7fa0081f9cec2c84ef9a30ad36599bfbdd9","repository":"https://github.com/ablative-io/aion.git"},"argus":{"commit":"dc2397c92ecf03e420aaa5a72b10cad673386e49","repository":"https://github.com/ablative-io/argus.git"},"cambium":{"commit":"105c03ded311b0ea5de9463eaab27320b2649004","repository":"https://github.com/ablative-io/cambium.git"},"haematite":{"commit":"36536826f4d04fc10037be49c1c2c9f1d073d459","repository":"https://github.com/ablative-io/haematite.git"},"method":{"commit":"25827d8d5d747b13039901e8d9deed755735c6fb","repository":"https://github.com/ablative-io/design-system.git"}}`.
- `python3 -c "import json; d=json.load(open('docs/design/project.json')); d.pop('roots'); print(json.dumps(d, sort_keys=True))"` prints the same line after R2 as it prints on 7b53625's docs/design/project.json.
- `find scripts/design/workers/ds2_ledger -maxdepth 1 -name '*.py' | sort` prints exactly four lines: scripts/design/workers/ds2_ledger/__init__.py, scripts/design/workers/ds2_ledger/legs.py, scripts/design/workers/ds2_ledger/project.py and scripts/design/workers/ds2_ledger/roots.py.
- `git -C "$M" show "$C":workers/ds2_ledger/project.py | cmp - scripts/design/workers/ds2_ledger/project.py`, `git -C "$M" show "$C":workers/ds2_ledger/legs.py | cmp - scripts/design/workers/ds2_ledger/legs.py`, `git -C "$M" show "$C":scripts/render-cluster.py | cmp - scripts/design/render-cluster.py` and `git -C "$M" show "$C":scripts/render-brief.py | cmp - scripts/design/render-brief.py` each exit 0.
- `git -C "$M" show "$C":scripts/validate.py | diff - scripts/design/validate.py | grep '^[<>]'` prints exactly two lines, `< SCHEMA_DIR = Path(__file__).resolve().parent.parent / "schemas"` then `> SCHEMA_DIR = Path(__file__).resolve().parent / "schemas"`.
- `git -C "$M" show "$C":scripts/check-coverage.py | diff - scripts/design/check-coverage.py | grep '^[<>]'` prints exactly two lines, `< sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "workers"))` then `> sys.path.insert(0, str(Path(__file__).resolve().parent / "workers"))`.
- `git -C "$M" show "$C":scripts/check-root-pins.py | diff - scripts/design/check-root-pins.py | grep '^[<>]'` prints exactly two lines, `< sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "workers"))` then `> sys.path.insert(0, str(Path(__file__).resolve().parent / "workers"))`.
- `diff -r "$M/schemas" scripts/design/schemas` prints nothing.
- `git diff --exit-code 7b53625 -- scripts/design/workers/ds2_ledger/roots.py` exits 0.
- `python3 -I -c "import sys; sys.path.insert(0, sys.argv[2] + '/workers'); from ds2_ledger.project import read_setup; read_setup(sys.argv[1])" "$PWD/docs/design" "$M"` exits 0.
- `python3 -I -c "import sys; sys.path.insert(0, 'scripts/design/workers'); import ds2_ledger.project; print(sorted(m for m in sys.modules if m.split('.')[0] in ('ds2_ledger', 'aion_worker')))"` exits 0 and prints a list containing 'ds2_ledger.project' in which every member is one of 'ds2_ledger', 'ds2_ledger.legs', 'ds2_ledger.project' and 'ds2_ledger.roots'.
- `python3 -I -c "import sys; sys.path.insert(0, 'scripts/design/workers'); from ds2_ledger.project import read_setup, resolve_root; print(resolve_root(read_setup(sys.argv[1]), 'aion'))" "$PWD/docs/design"` prints "$DS2_CHECKOUTS/aion" and exits 0.
- `DS2_CHECKOUTS="$E" python3 -I -c "import sys; sys.path.insert(0, 'scripts/design/workers'); from ds2_ledger.project import read_setup, resolve_root; print(resolve_root(read_setup(sys.argv[1]), 'aion'))" "$PWD/docs/design"` exits non-zero; its stderr contains "$E/aion", `https://github.com/ablative-io/aion.git`, `5594f7fa0081f9cec2c84ef9a30ad36599bfbdd9` and `fetch`; and `ls -A "$E"` afterwards prints nothing.
- `env -u DS2_CHECKOUTS python3 -I -c "import sys; sys.path.insert(0, 'scripts/design/workers'); from ds2_ledger.project import read_setup, resolve_root; resolve_root(read_setup(sys.argv[1]), 'aion')" "$PWD/docs/design"` exits non-zero and its stderr contains `DS2_CHECKOUTS`.
- `python3 -I scripts/design/check-root-pins.py "$PWD/docs/design"` exits 0 and prints exactly five lines whose first fields are aion, argus, cambium, haematite and method in that order, each of the form `<name> <repository> pin <commit> origin <head> distance <n>`; the method line's n equals the output of `git -C "$DS2_CHECKOUTS/design-system" rev-list --count 25827d8d5d747b13039901e8d9deed755735c6fb..refs/remotes/origin/HEAD`.
- Fixture pin-absent: D is an empty directory into which `python3 -c "import json, sys; d=json.load(open('docs/design/project.json')); d['roots']['aion']['commit']='0'*40; json.dump(d, open(sys.argv[1] + '/project.json', 'w'))" "$D"` has written a copy of project.json whose aion commit is forty zeros. `python3 -I scripts/design/check-root-pins.py "$D"` exits 1 and prints exactly five lines whose first fields are aion, argus, cambium, haematite and method in that order; the aion line is exactly `aion https://github.com/ablative-io/aion.git pin 0000000000000000000000000000000000000000 pin_absent act: git -C $DS2_CHECKOUTS/aion fetch origin` with $DS2_CHECKOUTS expanded, and each of the other four lines has the form `<name> <repository> pin <commit> origin <head> distance <n>`.
- Fixture origin-head-unset: F is an empty directory into which `for r in aion argus cambium haematite design-system; do cp -R "$DS2_CHECKOUTS/$r" "$F/$r"; done` has copied the five checkouts, after which `git -C "$F/argus" remote set-head origin --delete` has run. `DS2_CHECKOUTS="$F" python3 -I scripts/design/check-root-pins.py "$PWD/docs/design"` exits 1 and prints exactly five lines whose first fields are aion, argus, cambium, haematite and method in that order; the argus line is exactly `argus https://github.com/ablative-io/argus.git pin dc2397c92ecf03e420aaa5a72b10cad673386e49 origin_head_unset act: git -C $F/argus remote set-head origin --auto` with $F expanded, each of the other four lines has the form `<name> <repository> pin <commit> origin <head> distance <n>`, and `git -C "$F/argus" symbolic-ref -q refs/remotes/origin/HEAD` afterwards exits non-zero.
- `git -C "$DS2_CHECKOUTS/aion" for-each-ref` prints the same output before and after one run of `python3 -I scripts/design/check-root-pins.py "$PWD/docs/design"`, and `git diff --exit-code -- docs/design/project.json` exits 0 after it.
- `sh scripts/design/gate.sh` run from the repository root exits 0.

**Files:**
- create: scripts/design/workers/ds2_ledger/project.py
- create: scripts/design/workers/ds2_ledger/legs.py
- create: scripts/design/check-root-pins.py
- modify: scripts/design/SOURCE.md
- modify: scripts/design/validate.py
- modify: scripts/design/check-coverage.py
- modify: scripts/design/render-cluster.py
- modify: scripts/design/render-brief.py
- modify: scripts/design/schemas
- modify: docs/design/project.json

**Checklist:**
- C7 — lys's copies of the method are refreshed from the method commit whose reader accepts the root object, before project.json changes; scripts/design/SOURCE.md names that commit; project.py, legs.py, render-cluster.py and render-brief.py are byte-identical to the method's, validate.py, check-coverage.py and check-root-pins.py differ only in the one line each that SOURCE.md names, and the schemas match.
- C8 — docs/design/project.json holds the five roots as objects carrying the recorded repository and commit; the method's reader at the named commit reads it and exits 0; and lys's own copies, under python3 -I, resolve a root, refuse a missing checkout by name, and print the pin lines with both refusals.

**Stories:**
- S1 (Worker, Runs a survey or a round on a machine other than the one the design was written on) — As a worker, I want each root resolved through my own checkout of its repository at the pinned commit, so that a survey reads the same estate on any machine.
- S2 (Worker, Runs a survey or a round on a machine other than the one the design was written on) — As a worker whose checkout lacks a pinned commit, I want a refusal naming the checkout path, the repository, the commit and the fetch that answers it, so that nothing is cloned or fetched without my knowing.
- S4 (Reviewer, Decides whether a root's pin should be moved) — As a reviewer, I want each root's pin printed beside origin's head with the distance between them, and a root with no pin or no origin head named rather than skipped, so that a stale pin is a visible fact and moving it is a decision someone makes.

## Boundaries

- SHALL NOT change docs/design/project.json in R1, nor in R2 before the refresh from the method commit is committed.
- SHALL NOT rename, render, add or edit anything under docs/design/lys-core.
- SHALL NOT write a lys-local resolver or pin check; scripts/design holds only copies of the method, as SOURCE.md says.
- SHALL NOT modify scripts/design/workers/ds2_ledger/roots.py or change how the $NAME/ root tokens resolve.
- SHALL NOT change the project.json of aion, cambium, haematite or the design system.
- SHALL NOT add anything that moves a root's commit, and SHALL NOT add a pin per design.
- SHALL NOT fetch or clone any repository, in a script or in a step of this brief.
- SHALL NOT change any Rust crate or wire format.

## Verification

- From the repository root: `sh scripts/design/gate.sh` exits 0.
- `python3 scripts/design/validate.py docs/design/roots` and `python3 scripts/design/check-coverage.py docs/design/roots` each exit 0.
- `git diff --name-only 7b53625 HEAD -- docs/design/lys-core scripts/design/workers/ds2_ledger/roots.py` prints nothing.
- After R2: the reader command under blocked_by exits 0 with "$DS2_CHECKOUTS/design-system" at the commit SOURCE.md names, and `python3 -I scripts/design/check-root-pins.py "$PWD/docs/design"` exits 0.
- After R2: `git grep -n '/Users/' -- docs/design/project.json` prints nothing.

