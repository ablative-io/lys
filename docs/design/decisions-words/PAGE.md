# decisions-words — what was asked, what it means, and what was written

## The words, as they were typed

docs/design/decisions.json holds eight decisions (ADR-001 to ADR-008) whose quote field carries a person's words. The canon rule since 24 September: repository canon holds no person's words and nothing personal. Rewrite each quote as a plain statement of the decision in the team's words, keeping the decision, context, consequences and sources exactly as they are. ADR-009 and ADR-010 already read that way (lys PR 6, 586ef5f4) and are the pattern. Documents only; scripts/design/gate.sh must stay green (decisions.json validates against decisions.schema.json, which keeps the field required).

## What the survey found, and its angles

The ledger at docs/design/decisions.json has a quote field on every decision. In ADR-001 to ADR-008 that field holds a person's spoken words, which breaks the 24 September canon rule. Each of those eight quotes should become a short plain statement of the decision in the team's words, the way ADR-009 and ADR-010 were rewritten in 586ef5f4. Every other field stays byte for byte, the file is documents only, and scripts/design/gate.sh stays green.

### What the tree holds

- `docs/design/decisions.json` — The only file the words change. It has 330 lines and 18 decisions under {project, updated, decisions}. Rows 0 to 7 (ADR-001 to ADR-008) hold verbatim spoken words in quote, three of them joined with ellipses. json.dump(indent=2, ensure_ascii=False) plus a trailing newline gives back the committed bytes exactly, so each quote can be replaced without touching any other byte.
- `scripts/design/schemas/decisions.schema.json` — All 12 row fields are required, quote included, and additionalProperties is false, so the field stays and must be a string. The schema's own description of quote says 'Verbatim words of the decider when they carry the intent; empty string when none', which the canon rule and the ADR-009/010 pattern now contradict. The validator ignores descriptions, so the gate does not see this. Note the words place the schema at the wrong path: it lives under scripts/design/schemas/, not beside decisions.json.
- `scripts/design/gate.sh` — The design leg. It runs validate.py on docs/design/decisions.json, then validate, coverage and render comparison on every cluster that has a design.json. It exits 0 at 7b53625. The ledger has no rendered markdown twin, so no re-render is needed for it.
- `scripts/design/validate.py` — Maps decisions.json to decisions.schema.json by file name and checks only the schema subset plus the rule that no document names a machine. That means rewritten quote text must not contain an absolute path.
- `git show 586ef5f4 -- docs/design/decisions.json` — The pattern. Only the quote line of ADR-009 and of ADR-010 changed: each became one declarative sentence (103 and 57 characters) with no speaker, no time and no ellipsis. decided_by and context were left as they were, still naming Tom and Waffles.
- `docs/design/roadmap.json (RM-004)` — The roadmap row for this work, status idea. It links cluster decisions-words and ADR-001 to ADR-008, and its notes repeat the documents-only boundary and the gate. Its briefs and commits lists are empty.
- `docs/design/decisions-words/` — The cluster to be written. It does not exist yet. Once it has a design.json, gate.sh will validate it, check its coverage and compare its render, so its own documents must pass the gate too.
- `docs/design/identity/STATEMENT-2026-09-22.md and docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md` — The same spoken words also appear here. The ADR-001, 002, 003 and 004 quotes are all in STATEMENT (lines 21, 25 and 61 among others), and 'structural cog' is in CONTEXT-ROADMAP line 57. Both are canon under the same rule, but the words name only decisions.json.
- `docs/design/directory/design.json:140,632` — Lists docs/design/decisions.json as a structure row and an inventory row whose note says it is 'read here, never changed by a document row'. Editing quotes does not change that listing.

### What was already decided

- ADR-009 — Quote rewritten in 586ef5f4 to 'The platform keeps its own maintained fork of Rauthy rather than depending on an upstream contribution.' This is the pattern: one plain sentence of the decision.
- ADR-010 — Quote rewritten in 586ef5f4 to 'Every product shares one design and keeps its own accent.' A second pattern example, which reuses the title's phrasing.
- ADR-001 — Quote is spoken words about a temporary key mapped to the credential. It is the target to rewrite.
- ADR-002 — Quote is spoken words about plugging the revolver in and drawing from there. Target.
- ADR-003 — Quote is spoken words about pegging everything to the human's base permissions. Target.
- ADR-004 — Quote is spoken words joined with '...' about the optional execution engine and not a structural cog. Target.
- ADR-005 — Quote is spoken words agreeing to Postgres on a network device. Target. Its context and decision also name Tom and 'Tom's Mac'.
- ADR-006 — Quote is spoken words joined with two '…' ellipses about deliberate, explicit role moves. Target. Its context names Tom and a time.
- ADR-007 — Quote is spoken words joined with '…', naming Waffles, about not being an execution engine. Target. Its context names Tom.
- ADR-008 — Quote is spoken words about the original lys certificating stuff. Target. Its context names Tom.
- ADR-011 — Proposed. Its quote is also a person's spoken words ('We got to talk about identity access management…'), attributed through decided_by to Archie. It is outside the eight the words name.
- RM-004 — The roadmap row that carries this work. Status idea, cluster decisions-words, documents only, gate green.
- decisions.schema.json — quote is required and must be a string. Its description still reads 'Verbatim words of the decider … empty string when none'.

### What was measured

- Decisions in docs/design/decisions.json: 18 (ADR-001 to ADR-018), 330 lines, 31,822 bytes, updated 2026-09-26
- Quotes that are a person's spoken words: 9: ADR-001 to ADR-008, plus ADR-011
- Quotes already in plain team words: 2 (ADR-009, ADR-010)
- Empty quotes: 7 (ADR-012 to ADR-018)
- Target quote lengths (ADR-001 to ADR-008): 115, 99, 77, 137, 128, 149, 180 and 83 characters
- Pattern quote lengths (ADR-009, ADR-010): 103 and 57 characters, one sentence each
- Target quotes joined with ellipses: 3 (ADR-004, ADR-006, ADR-007)
- ADR-001 to ADR-008 rows whose context carries a 'Sources:' clause: 0 of 8 (ADR-009 and ADR-010 both have one)
- Lines of decisions.json naming Tom in context or decision among ADR-001 to ADR-008: 6 (lines 79, 80, 84, 96, 115, 133); decided_by is 'Tom' on all 8
- Required fields per decision in the schema: 12, quote among them; additionalProperties false
- JSON round trip (indent=2, ensure_ascii=False, trailing newline) against the committed file: byte-identical: True
- scripts/design/gate.sh at 7b53625: exit 0, coverage clean
- docs/design/decisions.schema.json (the path the words imply): does not exist; the schema is at scripts/design/schemas/decisions.schema.json
- docs/design/decisions-words: does not exist
- Other canon files holding the same spoken words: 2 (identity/STATEMENT-2026-09-22.md, which holds 4 of the 8 quotes; identity/CONTEXT-ROADMAP-2026-09-22.md, which holds 1)
- Rendered markdown twin of decisions.json: none (the gate renders only clusters that have a design.json)

### What it means for the other projects

- method — scripts/design/schemas/decisions.schema.json is copied from the design-system method at 3c3bac7, and its description of quote ('Verbatim words of the decider…') now contradicts the canon rule. The method may want its own row to reword that description upstream. Nothing changes there in this unit.

### The decisions it stands on

- ADR-001 (honour) — Only the quote is restated. The decision, context and consequences stay byte for byte.
- ADR-002 (honour) — Only the quote is restated.
- ADR-003 (honour) — Only the quote is restated.
- ADR-004 (honour) — Only the quote is restated. Its ellipsis-joined spoken words go.
- ADR-005 (honour) — Only the quote is restated. The context and consequences, which name Tom's Mac, stay unless the lead rules otherwise.
- ADR-006 (honour) — Only the quote is restated.
- ADR-007 (honour) — Only the quote is restated.
- ADR-008 (honour) — Only the quote is restated.
- ADR-009 (honour) — This is the pattern, and it is not touched.
- ADR-010 (honour) — This is the pattern, and it is not touched.
- ADR-011 (honour) — Left untouched unless the lead brings it into scope. Its quote is spoken words too.

### What it requires

- The quote of each of ADR-001 to ADR-008 in docs/design/decisions.json is a plain declarative statement of that decision, with no speaker, no time of day, no first person and no ellipsis.
- None of the eight current quote strings appears anywhere in docs/design/decisions.json.
- For every row, every field except quote is byte-identical to 7b53625. That includes id, title, status, scope, date, decided_by, context, decision, consequences, supersedes and superseded_by.
- Rows ADR-009 to ADR-018 are byte-identical to 7b53625.
- The number of decisions stays 18 and their order is unchanged.
- python3 scripts/design/validate.py docs/design/decisions.json passes.
- sh scripts/design/gate.sh exits 0.
- The file keeps the formatting that json.dump(indent=2, ensure_ascii=False) plus a trailing newline produces, so the diff touches only the eight quote lines, plus 'updated' if the author moves it.

### What must not change

- No change to scripts/design/schemas/decisions.schema.json; quote stays required.
- No code, script or crate changes.
- The decision, context, consequences, decided_by, date, status and supersession fields of any ADR do not change.
- No ADR is added, renumbered or superseded.
- docs/design/identity/* files are not changed within this unit's words.
- No other roadmap row or cluster document changes beyond the unit's own brief and cluster files.

### The risks

- A restatement could drift from the decision: it might add a nuance the decision field lacks, or drop the rejected alternative, and so quietly change what the ledger records as decided.
- The ledger could be re-serialised with different escaping or indentation, turning an eight-line change into a whole-file diff that hides accidental edits.
- ADR-011 and the personal content in the contexts would still break the canon rule after this lands, which could be read as the rule being met.
- The schema's description of quote still says verbatim words, so a later author following the schema could put spoken words back.
- The decisions-words cluster documents could fail the gate's coverage or render comparison once a design.json exists.

### Still open

- Should ADR-011's quote, which is Archie's spoken words, be rewritten in this same unit, or left for a separate row? The sentence of the words it stands on: "docs/design/decisions.json holds eight decisions (ADR-001 to ADR-008) whose quote field carries a person's words.". Why only the lead can settle it: The tree contradicts the count. docs/design/decisions.json ADR-011 has quote 'We got to talk about identity access management, permissions, and the start of lifecycle management before we get to the nitty gritty…', so nine decisions carry a person's words, not eight. Whether a reader of the ledger still sees spoken words after this lands depends on the answer.
- Should the personal content in the contexts of ADR-005 to ADR-008 stay untouched, as the words say, even though it breaks the canon rule? That content is Tom named with times of day, 'Tom said he had hoped…', and 'Tom's Mac' in ADR-005's decision and a consequence. The sentence of the words it stands on: "Rewrite each quote as a plain statement of the decision in the team's words, keeping the decision, context, consequences and sources exactly as they are.". Why only the lead can settle it: docs/design/decisions.json lines 79, 80, 84, 96, 115 and 133 carry a named person and a personal machine in context, decision and consequences. Keeping them exactly leaves the ledger still breaking 'nothing personal' after this work is done. ADR-009 and ADR-010 set the precedent of leaving context as it was.
- Are docs/design/identity/STATEMENT-2026-09-22.md and CONTEXT-ROADMAP-2026-09-22.md, which carry the same spoken words, out of scope for this unit? The sentence of the words it stands on: "The canon rule since 24 September: repository canon holds no person's words and nothing personal.". Why only the lead can settle it: The rule as quoted covers all of repository canon. STATEMENT-2026-09-22.md holds the ADR-001, 002, 003 and 004 quotes verbatim (for example line 25), and CONTEXT-ROADMAP line 57 holds 'structural cog'. After this unit, a person would still find those words in the repository.

### The units beyond the first

- Restate ADR-011's quote in the team's words — It carries Archie's spoken words but falls outside the eight the words name, so it is its own row unless the lead folds it in.
- Remove personal content from the ledger's contexts (ADR-005 to ADR-008) — Names, times of day and 'Tom's Mac' remain in context, decision and consequences, which the words keep exactly. Removing them means changing those fields, so it is a separate call.
- Bring docs/design/identity STATEMENT and CONTEXT-ROADMAP to the canon rule — They hold the same verbatim words. They are outside the words' single-file scope, and they are earlier revision-form documents that the gate does not measure.
- Reword the method's decisions.schema.json description of quote — The description still says verbatim words. The schema is owned by the design-system method and copied here, so the change lands there first.

### The smallest complete shape

One document commit to docs/design/decisions.json that replaces the quote of each of ADR-001 to ADR-008 with one plain sentence of its decision in the ADR-009/010 style, leaves every other byte unchanged (apart from 'updated' if the author moves it), and passes scripts/design/gate.sh, together with the decisions-words cluster and brief that the chain carries it under as RM-004.

## The roadmap row

- **RM-017** — Restate the decision ledger's spoken quotes and personal content in the team's words (design, idea)
- Summary: docs/design/decisions.json carries a person's spoken words in the quote of ADR-001 to ADR-008 and ADR-011, and names a person, times of day and a personal machine in the context, decision and consequences of ADR-005 to ADR-008. Each of the nine quotes becomes one plain statement of its decision in the team's words, in the form of ADR-009 and ADR-010, and in ADR-005 to ADR-008 only what is personal becomes a role. Every other field and every source stays as it is; documents only; scripts/design/gate.sh stays green.
- Asked by: tom on 2026-09-27T07:08:00+10:00
- Context: The decisions-words card, surveyed at 7b53625. The lead's answers brought ADR-011's quote into this unit and ruled that the canon rule wins over keeping ADR-005 to ADR-008's context, decision and consequences exactly where the two meet; the identity STATEMENT and CONTEXT-ROADMAP records stay outside it.
- Quote: docs/design/decisions.json holds eight decisions (ADR-001 to ADR-008) whose quote field carries a person's words. The canon rule since 24 September: repository canon holds no person's words and nothing personal. Rewrite each quote as a plain statement of the decision in the team's words, keeping the decision, context, consequences and sources exactly as they are. ADR-009 and ADR-010 already read that way (lys PR 6, 586ef5f4) and are the pattern. Documents only; scripts/design/gate.sh must stay green (decisions.json validates against decisions.schema.json, which keeps the field required).
- Cluster: decisions-words; briefs: DECISIONSWORDS-001
- Notes: Further units, not written: Bring docs/design/identity STATEMENT and CONTEXT-ROADMAP to the canon rule (a separate card the lead files); Reword the method's decisions.schema.json description of quote (the design-system method's own row). RM-004 is the earlier row carrying the same words; it is left unchanged. Id RM-017 is the next after RM-016, main's highest, with RM-011 to RM-015 taken on open brief branches.

## The design

---
type: design
cluster: decisions-words
title: Decision ledger in the team's words
---

# Decision ledger in the team's words

> **Cluster:** decisions-words

## Intention

The decision ledger is repository canon, and repository canon holds no person's words and nothing personal. When this is done a reader of docs/design/decisions.json finds every decision stated in the team's own plain words: the quote of each decision is one declarative sentence of what was decided, and the context, decision and consequences name roles rather than people, times of day or personal machines, while what was decided, why, and what it obliges stay exactly as they were ruled.

The ledger is cited by every brief that anchors a decision, so a restatement that shifts a decision's meaning would shift every review that checks against it. The intent is a change of voice, never a change of substance: the smallest edit that removes what is personal and leaves the ruling intact.

## Problem

Nine decisions in docs/design/decisions.json carry a person's spoken words in their quote field: ADR-001 to ADR-008 and ADR-011, three of them joined with ellipses and one naming a person inside the words. ADR-005 to ADR-008 also name a person, with times of day, in their context, and ADR-005 and ADR-008 name that person in their decision or consequences, including a personal machine. That breaks the canon rule that repository canon holds no person's words and nothing personal. ADR-009 and ADR-010 have already been brought to the rule in their quotes (586ef5f4) and are the pattern. Every brief that anchors one of these decisions projects the ledger row into its stage prompts, so the personal content travels into every run that cites it.

## Solution

One document change to docs/design/decisions.json, delivered by DECISIONSWORDS-001 and nothing else.

The quote of each of ADR-001 to ADR-008 and ADR-011 is replaced by one declarative sentence of that decision, written in the team's words in the form ADR-009 and ADR-010 already take: no speaker, no time of day, no first person and no ellipsis, restating only what the decision field already records (P1). The quote field stays present and non-empty, because the schema keeps it required and the words ask for a plain statement (CN1).

In ADR-005 to ADR-008 only what is personal in the context, decision and consequences changes: a named person becomes the operator, a time of day tied to that person is dropped while its date stays, what the person said or hoped becomes the need it stated, and the personal machine becomes the operator's workstation (P2). Every other sentence of those fields, and every source, stays byte for byte.

Because the gate compares every cluster's committed markdown with what its JSON renders to, and a rendered design or brief carries the decision field of each ADR it cites, the change to ADR-005's decision field alters the rendered markdown that cites ADR-005: docs/design/directory/DESIGN.md, the DIRECTORY-001, DIRECTORY-002 and DIRECTORY-006 briefs, and this cluster's own DESIGN.md and brief. Those files are re-rendered with scripts/design/render-cluster.py, never edited by hand, so the gate stays green (CN5).

Every other field of every decision, the order and number of decisions, and the file's serialisation stay as they are at 7b53625, so the diff touches only the changed lines plus the ledger's updated date (CN2, CN3). The ledger is validated by scripts/design/gate.sh against scripts/design/schemas/decisions.schema.json, which this work does not touch (CN4, CN5).

The same spoken words also sit in docs/design/identity/STATEMENT-2026-09-22.md and docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md, which are dated records carried by a separate card; the names in the context of ADR-009, ADR-010 and ADR-011 are left as they are. Decided_by is not a person's words: it names who decided, the ledger needs it to say whose decision each entry is, and it stays byte-identical on every row.

## Principles

- **P1** — A restated quote says only what the row's decision field already says: no nuance added, no rejected road dropped, no new claim.
- **P2** — Only what is personal changes. A named person becomes the operator, a time of day tied to that person goes, a person's saying or hoping becomes the need it stated, and a personal machine becomes the operator's workstation; everything else stays byte for byte.
- **P3** — The diff is the review surface: the ledger keeps its exact serialisation so that every changed line is one the brief names.

## Decisions

- ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
- ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
- ADR-006 — A role changes only by a deliberate, explicit act — Editing a role makes a new version of it, and a holder never changes version silently as a side effect of the edit. A holder moves between versions either by a deliberate, recorded act or by an explicit rule shown in advance to the person the holder answers to, such as a move at the holding's next renewal. Which of the two is the default is open. A provisional role is a role held under a grant with an end date: when the date passes the holding lapses, and nothing renews it unless someone grants it again.
- ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
- ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
- ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.

## Goals

- Every committed markdown file under docs/design/directory and docs/design/decisions-words equals what scripts/design/render-cluster.py renders from the changed ledger.
- The quote of each of ADR-001 to ADR-008 and ADR-011 in docs/design/decisions.json is one declarative sentence with no speaker, time of day, first person or ellipsis.
- No context, decision or consequence of ADR-005 to ADR-008 contains its row's decided_by value or a time of day.
- git diff --numstat 7b53625 -- docs/design/decisions.json on the build's commit reports 17 lines added and 17 removed.
- sh scripts/design/gate.sh exits 0 with docs/design/decisions.json changed.

## Non-Goals

- Bring docs/design/identity/STATEMENT-2026-09-22.md and docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md to the canon rule. — They are dated records of their own, carried by a separate card the lead files.
- Reword the description of quote in scripts/design/schemas/decisions.schema.json. — The schema is copied from the design-system method, and the words rule out a schema change here; any rewording lands in the method first.
- Change the context, decision or consequences of ADR-001 to ADR-004, ADR-009, ADR-010 or ADR-011. — The lead's ruling limits the personal-content change to ADR-005 to ADR-008 and leaves ADR-009 and ADR-010 untouched.
- Change the decided_by field of any decision. — decided_by is attribution, not a person's words: it names whose decision each entry is, and the words keep every field but the quote.
- Add a Sources clause to ADR-001 to ADR-008. — The words keep the sources exactly as they are.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/decisions.json` | The project decision ledger; the quotes of nine rows and the personal content of ADR-005 to ADR-008 are restated here |  |
| `docs/design/directory/DESIGN.md` | Rendered from docs/design/directory/design.json; its ADR-005 line carries ADR-005's decision |  |
| `docs/design/directory/briefs/DIRECTORY-001.md` | Rendered brief; its design anchor carries ADR-005's decision |  |
| `docs/design/directory/briefs/DIRECTORY-002.md` | Rendered brief; its design anchor carries ADR-005's decision |  |
| `docs/design/directory/briefs/DIRECTORY-006.md` | Rendered brief; its design anchor carries ADR-005's decision |  |
| `docs/design/decisions-words/DESIGN.md` | Rendered from this cluster's design.json when the brief lands; its ADR-005 line carries ADR-005's decision |  |
| `docs/design/decisions-words/briefs/DECISIONSWORDS-001.md` | Rendered from this brief when it lands; its design anchor carries ADR-005's decision |  |

## Inventory

- `docs/design/decisions.json` — 18 decisions under {project, updated, decisions}, updated 2026-09-26. ADR-001 to ADR-008 and ADR-011 hold spoken words in quote; ADR-009 and ADR-010 hold plain statements; ADR-012 to ADR-018 hold empty quotes. Serialised exactly as json.dump(indent=2, ensure_ascii=False) plus a trailing newline.
- `scripts/design/schemas/decisions.schema.json` — Twelve required fields per decision, quote among them, additionalProperties false; copied from the design-system method. Its description of quote still speaks of verbatim words.
- `scripts/design/gate.sh` — The design leg: validates decisions.json and project.json, then validates, checks coverage of and compares the render of every cluster with a design.json. Exits 0 at 7b53625.
- `scripts/design/validate.py` — Validates a document against the schema its file name maps to and refuses an absolute path in a document.
- `docs/design/identity/STATEMENT-2026-09-22.md` — Dated record holding the spoken words of four of the restated quotes; outside this cluster.
- `docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md` — Dated record holding the spoken words of one restated quote; outside this cluster.

## Constraints

- **CN1** — scripts/design/schemas/decisions.schema.json is unchanged; every decision keeps a quote field holding a non-empty string.
- **CN2** — docs/design/decisions.json holds 18 decisions, ADR-001 to ADR-018 in that order, with no decision added, removed, renumbered or superseded.
- **CN3** — docs/design/decisions.json is byte-identical to json.dumps(its parsed content, indent=2, ensure_ascii=False) followed by one newline.
- **CN4** — No code, script or crate changes in the build; the only files it changes are docs/design/decisions.json and the rendered markdown that scripts/design/render-cluster.py regenerates from it.
- **CN5** — sh scripts/design/gate.sh exits 0.


---
type: brief
id: DECISIONSWORDS-001
cluster: decisions-words
title: Restate the ledger's spoken quotes and personal content in the team's words
---

# DECISIONSWORDS-001: Restate the ledger's spoken quotes and personal content in the team's words

> **Cluster:** decisions-words
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> - ADR-006 — A role changes only by a deliberate, explicit act — Editing a role makes a new version of it, and a holder never changes version silently as a side effect of the edit. A holder moves between versions either by a deliberate, recorded act or by an explicit rule shown in advance to the person the holder answers to, such as a move at the holding's next renewal. Which of the two is the default is open. A provisional role is a role held under a grant with an end date: when the date passes the holding lapses, and nothing renews it unless someone grants it again.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> **Checklist:**
> - C1 — The quote of each of ADR-001 to ADR-008 in docs/design/decisions.json is one declarative sentence of its decision with no speaker, time of day, first person or ellipsis.
> - C2 — The quote of ADR-011 in docs/design/decisions.json is one declarative sentence of its decision with no speaker, time of day, first person or ellipsis.
> - C3 — No quote value held by docs/design/decisions.json at 7b53625 for ADR-001 to ADR-008 or ADR-011 appears anywhere in the file.
> - C4 — No context, decision or consequence of ADR-005 to ADR-008 contains its row's decided_by value.
> - C5 — No context, decision or consequence of ADR-005 to ADR-008 contains a time of day.
> - C6 — ADR-005's context states the need the operator raised in place of what a person said or hoped.
> - C7 — ADR-005's decision and its second consequence name the operator's workstation in place of a personal machine.
> - C8 — Every field of every decision that C1 to C7 do not name is byte-identical to its value at 7b53625.
> - C9 — docs/design/decisions.json holds 18 decisions, ADR-001 to ADR-018 in order.
> - C10 — The ledger's updated field is the date of the commit that makes the change.
> - C11 — docs/design/decisions.json keeps the serialisation of json.dump(indent=2, ensure_ascii=False) plus a trailing newline.
> - C12 — scripts/design/schemas/decisions.schema.json is unchanged from 7b53625.
> - C13 — sh scripts/design/gate.sh exits 0.
> - C14 — Every committed markdown file that renders ADR-005's decision is the output of scripts/design/render-cluster.py over the changed ledger.
> **Stories:**
> - S1 (Ledger reader, Reading a decision) — As a ledger reader, I want each decision's quote to state the decision in the team's words so that the ledger holds no person's spoken words.
> - S2 (Ledger reader, Reading a decision) — As a ledger reader, I want a decision's context, decision and consequences to name roles instead of people, times of day and personal machines so that the ledger holds nothing personal.
> - S3 (Brief author, Anchoring a decision) — As a brief author anchoring a decision, I want its decision, context and consequences to mean exactly what was ruled so that a review against the ledger checks the same obligations as before.
> - S4 (Card lead, Landing a document change) — As a card lead landing the change, I want the design gate to pass with the schema untouched so that the ledger stays valid under the method it is measured by.

## Purpose

Brings docs/design/decisions.json to the canon rule that repository canon holds no person's words and nothing personal: the nine quotes that carry spoken words become plain statements of their decisions, and the personal content in ADR-005 to ADR-008 becomes roles, without changing what any decision records. See the decisions-words design for the principles (P1 to P3) and constraints (CN1 to CN5).

## Task

Edit docs/design/decisions.json, then re-render what it feeds. (1) Replace the quote of ADR-001 to ADR-008 and of ADR-011 with the exact sentences R1 gives. ADR-009 and ADR-010 already read this way (586ef5f4) and are the pattern; they are not touched. The quote stays a non-empty string because the schema requires it. (2) In ADR-005 to ADR-008 change only what is personal in context, decision and consequences, as R2 gives: the named person becomes the operator, a time of day tied to that person is dropped while its date stays, what the person said or hoped becomes the need it stated, and the personal machine becomes the operator's workstation. (3) Move updated to the date of the commit, and write the file back with json.dump(indent=2, ensure_ascii=False) plus a trailing newline so that only the changed lines differ (R3). (4) Re-render the directory and decisions-words clusters with scripts/design/render-cluster.py: a rendered design or brief carries the decision field of each ADR it cites, so the ADR-005 decision change alters six rendered markdown files, and the gate compares them (R4). This cluster's own DESIGN.md and brief markdown are rendered when this brief lands, so they exist before the build starts.

Out of scope: every other field of every decision, including decided_by on every row; the context, decision and consequences of ADR-001 to ADR-004 and of ADR-009 to ADR-018; scripts/design/schemas/decisions.schema.json; every script, crate and other document. Known remaining personal content after this brief, so a reader knows where it is: docs/design/identity/STATEMENT-2026-09-22.md and docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md hold the same spoken words and are carried by a separate card; ADR-009, ADR-010 and ADR-011 still name people in context; docs/design/directory/briefs/DIRECTORY-002.json names the same personal machine in a requirement's spec. The decided_by field is deliberately kept: it names who decided, not what anyone said, and the ledger needs it to say whose decision each entry is, so a name there is attribution and not a missed quote. The schema's description of quote still speaks of verbatim words; that belongs to the design-system method.

R1 to R3 edit the same single file, so the primary-file rule cannot hold for them; each owns a disjoint set of fields within it.

## Requirements

### R1: Restate nine quotes as plain statements of their decisions

The quote field of ADR-001 to ADR-008 and of ADR-011 in docs/design/decisions.json SHALL each hold exactly one declarative sentence that restates that row's decision field in the team's words, as given in the acceptance below. A restated quote SHALL NOT name a speaker, SHALL NOT carry a time of day, SHALL NOT use the first person, SHALL NOT contain an ellipsis, SHALL NOT add a claim the row's decision field does not make, and SHALL NOT be the empty string. The quote of any other decision SHALL NOT change.

**Acceptance:**
- ADR-001's quote equals "A seat uses a credential through a short-lived handle that the door swaps for the real credential, which never leaves the server.".
- ADR-002's quote equals "The token revolver takes its next account from the broker through one handle instead of walking its own list.".
- ADR-003's quote equals "An agent's permissions are an explicit subset of the permissions of the person it is provisioned under.".
- ADR-004's quote equals "Manifold is one optional execution engine, and every project in the stack works without the others.".
- ADR-005's quote equals "The identity product's database is PostgreSQL, which may run on a network device rather than on the operator's workstation.".
- ADR-006's quote equals "A holder changes role version only by a deliberate, recorded act or by an explicit rule shown in advance, and a provisional holding lapses when its grant's end date passes.".
- ADR-007's quote equals "The product is not an execution engine, and for the first release it gives the command that starts an agent rather than running it.".
- ADR-008's quote equals "An agent's file shows its lys certificate and the signed receipts of the changes made to it.".
- ADR-011's quote equals "An identity is in one of four states: registered, active, suspended or retired.".
- For each of the nine quote values that git show 7b53625:docs/design/decisions.json holds for ADR-001 to ADR-008 and ADR-011, a substring search of docs/design/decisions.json finds 0 occurrences.
- Counting the characters '…' and the string '...' across the quote fields of all 18 decisions gives 0.
- The quote of every decision other than ADR-001 to ADR-008 and ADR-011 equals its value at 7b53625.

**Files:**
- modify: docs/design/decisions.json

**Checklist:**
- C1 — The quote of each of ADR-001 to ADR-008 in docs/design/decisions.json is one declarative sentence of its decision with no speaker, time of day, first person or ellipsis.
- C2 — The quote of ADR-011 in docs/design/decisions.json is one declarative sentence of its decision with no speaker, time of day, first person or ellipsis.
- C3 — No quote value held by docs/design/decisions.json at 7b53625 for ADR-001 to ADR-008 or ADR-011 appears anywhere in the file.

**Stories:**
- S1 (Ledger reader, Reading a decision) — As a ledger reader, I want each decision's quote to state the decision in the team's words so that the ledger holds no person's spoken words.
- S3 (Brief author, Anchoring a decision) — As a brief author anchoring a decision, I want its decision, context and consequences to mean exactly what was ruled so that a review against the ledger checks the same obligations as before.

### R2: Replace the personal content of ADR-005 to ADR-008 with roles

In the context, decision and consequences of ADR-005 to ADR-008, the named person SHALL become the operator, a time of day tied to that person SHALL be removed while its date stays, what the person said or hoped SHALL become the need it stated, and the personal machine SHALL become the operator's workstation. No other sentence, word or source in those fields SHALL change, the decision's meaning SHALL NOT change, the number of consequences of each row SHALL NOT change, and the context, decision and consequences of any other decision SHALL NOT change.

**Acceptance:**
- ADR-005's context equals "In the Dot room the operator stated a need to get rid of PostgreSQL, and asked whether the team thought it was the best solution. The working team decision was one PostgreSQL service and one database for Rauthy and SpiceDB, with separate roles and schema namespaces.".
- ADR-005's decision equals "PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on the operator's workstation.".
- ADR-005's second consequence equals "The host is not assumed to be the operator's workstation; the deployment names its database address as configuration.", and its first consequence equals its value at 7b53625.
- Let old be ADR-006's context at 7b53625 and name be ADR-006's decided_by: ADR-006's context equals re.sub(r'At \d{1,2}:\d{2} on ', 'On ', old).replace(name, 'the operator').
- Let old be ADR-007's context at 7b53625 and name be ADR-007's decided_by: ADR-007's context equals re.sub(r' at \d{1,2}:\d{2}', '', old).replace(name, 'the operator').replace(' he asked', ' the operator asked').
- Let old be ADR-008's context at 7b53625 and name be ADR-008's decided_by: ADR-008's context equals re.sub(r' at \d{1,2}:\d{2}', '', old).replace(name, 'the operator').
- ADR-008's third consequence equals "Production keys and receipts outside tests remain the operator's acts.", and its first, second and fourth consequences equal their values at 7b53625.
- Across the context, decision and consequences of ADR-005 to ADR-008, occurrences of the row's own decided_by value total 7 at 7b53625 and 0 after the change.
- Across the same fields, matches of the regular expression \b\d{1,2}:\d{2}\b total 4 at 7b53625 and 0 after the change.
- Across the same fields, matches of the regular expression \bhe\b total 2 at 7b53625 and 0 after the change.
- ADR-006's decision and consequences, ADR-007's decision and consequences, and ADR-008's decision equal their values at 7b53625.

**Files:**
- modify: docs/design/decisions.json

**Checklist:**
- C4 — No context, decision or consequence of ADR-005 to ADR-008 contains its row's decided_by value.
- C5 — No context, decision or consequence of ADR-005 to ADR-008 contains a time of day.
- C6 — ADR-005's context states the need the operator raised in place of what a person said or hoped.
- C7 — ADR-005's decision and its second consequence name the operator's workstation in place of a personal machine.

**Stories:**
- S2 (Ledger reader, Reading a decision) — As a ledger reader, I want a decision's context, decision and consequences to name roles instead of people, times of day and personal machines so that the ledger holds nothing personal.
- S3 (Brief author, Anchoring a decision) — As a brief author anchoring a decision, I want its decision, context and consequences to mean exactly what was ruled so that a review against the ledger checks the same obligations as before.

### R3: Keep every other byte of the ledger, move its updated date, and keep the gate green

docs/design/decisions.json SHALL keep every field that R1 and R2 do not name byte-identical to 7b53625, SHALL keep 18 decisions in their order, and SHALL be written as json.dump(indent=2, ensure_ascii=False) plus one trailing newline. Its updated field SHALL become the date of the commit that makes the change. The build SHALL NOT change scripts/design/schemas/decisions.schema.json, SHALL NOT add, remove, renumber, reorder or supersede a decision, and SHALL NOT change the decided_by, date, status, title, scope, supersedes or superseded_by of any decision. WHEN sh scripts/design/gate.sh runs on the build's commit, THE SYSTEM SHALL exit 0.

**Acceptance:**
- json.load of docs/design/decisions.json gives a decisions array whose ids are ADR-001 to ADR-018 in that order.
- For ADR-009, ADR-010 and ADR-012 to ADR-018, the whole row equals its row at 7b53625.
- For every decision, id, title, status, scope, date, decided_by, supersedes and superseded_by equal their values at 7b53625.
- For ADR-001 to ADR-004 and ADR-011, context, decision and consequences equal their values at 7b53625.
- project equals its value at 7b53625, and updated equals the output of git log -1 --format=%cs on the build's commit.
- json.dumps(json.load(docs/design/decisions.json), indent=2, ensure_ascii=False) + '\n' equals the file's bytes.
- git diff --numstat 7b53625 -- docs/design/decisions.json on the build's commit prints 17 added and 17 removed lines.
- git diff 7b53625 -- scripts/design/schemas/decisions.schema.json prints nothing.
- python3 scripts/design/validate.py docs/design/decisions.json exits 0.
- sh scripts/design/gate.sh exits 0.

**Files:**
- modify: docs/design/decisions.json

**Checklist:**
- C8 — Every field of every decision that C1 to C7 do not name is byte-identical to its value at 7b53625.
- C9 — docs/design/decisions.json holds 18 decisions, ADR-001 to ADR-018 in order.
- C10 — The ledger's updated field is the date of the commit that makes the change.
- C11 — docs/design/decisions.json keeps the serialisation of json.dump(indent=2, ensure_ascii=False) plus a trailing newline.
- C12 — scripts/design/schemas/decisions.schema.json is unchanged from 7b53625.
- C13 — sh scripts/design/gate.sh exits 0.

**Stories:**
- S3 (Brief author, Anchoring a decision) — As a brief author anchoring a decision, I want its decision, context and consequences to mean exactly what was ruled so that a review against the ledger checks the same obligations as before.
- S4 (Card lead, Landing a document change) — As a card lead landing the change, I want the design gate to pass with the schema untouched so that the ledger stays valid under the method it is measured by.

### R4: Re-render the markdown that carries ADR-005's decision

WHEN docs/design/decisions.json has changed as R1 to R3 give, THE SYSTEM SHALL regenerate the rendered markdown of the directory and decisions-words clusters with python3 scripts/design/render-cluster.py, so that each committed markdown file equals what its JSON renders to. A rendered markdown file SHALL NOT be edited by hand, the JSON documents of the directory cluster SHALL NOT change, and no markdown line SHALL change other than the lines that render ADR-005's decision.

**Acceptance:**
- After python3 scripts/design/render-cluster.py docs/design/directory and python3 scripts/design/render-cluster.py docs/design/decisions-words run on the build's commit, git status --porcelain prints nothing.
- git diff --numstat against the build's parent commit reports 1 added and 1 removed line for each of docs/design/directory/DESIGN.md, docs/design/directory/briefs/DIRECTORY-001.md, docs/design/directory/briefs/DIRECTORY-002.md, docs/design/directory/briefs/DIRECTORY-006.md, docs/design/decisions-words/DESIGN.md and docs/design/decisions-words/briefs/DECISIONSWORDS-001.md.
- In each of those six files, the one added line contains the string "PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on the operator's workstation." and begins with the same text before ADR-005's decision as the removed line.
- git diff --name-only against the build's parent commit lists exactly docs/design/decisions.json and those six markdown files.
- sh scripts/design/gate.sh exits 0 and prints no line containing 'rendered markdown differs'.

**Files:**
- modify: docs/design/directory/DESIGN.md
- modify: docs/design/directory/briefs/DIRECTORY-001.md
- modify: docs/design/directory/briefs/DIRECTORY-002.md
- modify: docs/design/directory/briefs/DIRECTORY-006.md
- modify: docs/design/decisions-words/DESIGN.md
- modify: docs/design/decisions-words/briefs/DECISIONSWORDS-001.md

**Checklist:**
- C14 — Every committed markdown file that renders ADR-005's decision is the output of scripts/design/render-cluster.py over the changed ledger.

**Stories:**
- S4 (Card lead, Landing a document change) — As a card lead landing the change, I want the design gate to pass with the schema untouched so that the ledger stays valid under the method it is measured by.

## Boundaries

- SHALL NOT change any file other than docs/design/decisions.json and the six rendered markdown files R4 names, and SHALL NOT edit a rendered markdown file by hand.
- SHALL NOT change scripts/design/schemas/decisions.schema.json, any script, any crate or any code.
- SHALL NOT add, remove, renumber, reorder or supersede a decision.
- SHALL NOT change the decided_by, date, status, title, scope, supersedes or superseded_by of any decision.
- SHALL NOT change any field of ADR-009, ADR-010 or ADR-012 to ADR-018, and SHALL NOT change any field of ADR-001 to ADR-004 or ADR-011 other than quote.
- SHALL NOT change docs/design/identity/STATEMENT-2026-09-22.md, docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md or any other file under docs/design/identity.
- SHALL NOT empty a quote, and SHALL NOT add a sentence, a Sources clause or a claim to any field.
- SHALL NOT re-serialise docs/design/decisions.json with any indentation, key order or escaping other than json.dump(indent=2, ensure_ascii=False) plus a trailing newline.

## Verification

- python3 scripts/design/validate.py docs/design/decisions.json exits 0.
- sh scripts/design/gate.sh exits 0.
- git diff --numstat 7b53625 -- docs/design/decisions.json prints 17 added and 17 removed lines, and git diff 7b53625 -- docs/design/decisions.json shows only the nine quote lines, the three ADR-005 lines, the ADR-006, ADR-007 and ADR-008 context lines, ADR-008's third consequence line and the updated line.
- A script loading docs/design/decisions.json and git show 7b53625:docs/design/decisions.json compares every field of every decision and reports differences only in the fields R1 and R2 name.
- git diff 7b53625 -- scripts/design/schemas/decisions.schema.json prints nothing.
- Re-running python3 scripts/design/render-cluster.py on docs/design/directory and docs/design/decisions-words leaves git status --porcelain empty.

