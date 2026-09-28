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
- ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on the operator's workstation.
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
