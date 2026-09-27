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
