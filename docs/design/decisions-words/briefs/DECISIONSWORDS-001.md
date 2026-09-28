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
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on the operator's workstation.
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

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each of the nine acceptance-given sentences is now the quote of its row in docs/design/decisions.json (one quote line per row in the ADR-001 to ADR-008 and ADR-011 objects), set by exact string, not paraphrase. I searched the file for each of the nine quote values held at 7b53625: every search finds 0 occurrences. Counting '…' and '...' across the quotes of ADR-001 to ADR-018 gives 0, and so does the count across all 78 decisions. Comparing field by field against HEAD, the quote of every other decision is unchanged. ADR-001 to ADR-011 are identical at HEAD and at 7b53625. ADR-012 to ADR-015 were changed between 7b53625 and HEAD, but only in their consequences, so their quotes still equal their values at 7b53625.
- Deviation: The ledger has grown since the brief was written: it now holds 78 decisions (up to ADR-111), not 18. The comparison of the other decisions' quotes was therefore run over all 78 against HEAD. The quotes of ADR-001 to ADR-018 were also checked against 7b53625.
- Files changed:
  - modified: `docs/design/decisions.json` — The quote of ADR-001 to ADR-008 and ADR-011 is now the exact declarative sentence the brief's acceptance gives; every other quote is unchanged.
- Checklist delivery:
  - [x] C1 — The quote of each of ADR-001 to ADR-008 in docs/design/decisions.json is one declarative sentence of its decision with no speaker, time of day, first person or ellipsis. — The quotes of ADR-001 to ADR-008 are the acceptance's exact sentences: one declarative sentence each, with no speaker, time of day, first person or ellipsis.
  - [x] C2 — The quote of ADR-011 in docs/design/decisions.json is one declarative sentence of its decision with no speaker, time of day, first person or ellipsis. — ADR-011's quote is the exact acceptance sentence.
  - [x] C3 — No quote value held by docs/design/decisions.json at 7b53625 for ADR-001 to ADR-008 or ADR-011 appears anywhere in the file. — Searching the file for each of the nine old quotes finds 0 occurrences.
- Story delivery:
  - [x] S1 (Ledger reader, Reading a decision) — As a ledger reader, I want each decision's quote to state the decision in the team's words so that the ledger holds no person's spoken words. — None of the nine quotes carries anyone's spoken words any more.
  - [x] S3 (Brief author, Anchoring a decision) — As a brief author anchoring a decision, I want its decision, context and consequences to mean exactly what was ruled so that a review against the ledger checks the same obligations as before. — Only quote fields changed under R1; decision, context and consequences are untouched by it.

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

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: ADR-005's context, decision and second consequence were set to the exact strings the acceptance gives, and its first consequence is unchanged. ADR-006's context was produced by the acceptance's own transform: re.sub('At \d{1,2}:\d{2} on ', 'On ') and then replace(decided_by, 'the operator'). ADR-007 used re.sub(' at \d{1,2}:\d{2}', ''), replace(decided_by, 'the operator') and replace(' he asked', ' the operator asked'). ADR-008 used re.sub(' at \d{1,2}:\d{2}', '') and replace(decided_by, 'the operator'), and its third consequence was set to the acceptance string. The field-by-field diff against HEAD shows changes only in: ADR-005 context, decision, consequences[1] and quote; ADR-006 context and quote; ADR-007 context and quote; ADR-008 context, consequences[2] and quote. The number of consequences in each row is unchanged. In the new text, the decided_by name 'Tom', the time-of-day matches and the matches of \bhe\b all count 0, as the git diff shows.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/decisions.json` — The context, decision and consequences of ADR-005 to ADR-008 now name the operator and the operator's workstation in place of a person, times of day and a personal machine.
- Checklist delivery:
  - [x] C4 — No context, decision or consequence of ADR-005 to ADR-008 contains its row's decided_by value. — 'Tom' no longer appears in the context, decision or consequences of ADR-005 to ADR-008.
  - [x] C5 — No context, decision or consequence of ADR-005 to ADR-008 contains a time of day. — The times 14:08, 18:54, 18:56 and 18:55 were removed; their dates are kept.
  - [x] C6 — ADR-005's context states the need the operator raised in place of what a person said or hoped. — ADR-005's context now reads 'the operator stated a need to get rid of PostgreSQL'.
  - [x] C7 — ADR-005's decision and its second consequence name the operator's workstation in place of a personal machine. — ADR-005's decision and its second consequence now say 'the operator's workstation'.
- Story delivery:
  - [x] S2 (Ledger reader, Reading a decision) — As a ledger reader, I want a decision's context, decision and consequences to name roles instead of people, times of day and personal machines so that the ledger holds nothing personal. — These fields now name roles instead of a person, times of day or a personal machine.
  - [x] S3 (Brief author, Anchoring a decision) — As a brief author anchoring a decision, I want its decision, context and consequences to mean exactly what was ruled so that a review against the ledger checks the same obligations as before. — What each decision rules is unchanged; only who was named and the times of day are replaced.

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

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Before writing, I asserted that the file equalled json.dumps(parsed, indent=2, ensure_ascii=False) + '\n', then wrote it back in the same form. git diff --numstat against HEAD, the build's parent, gives 17 added and 17 removed lines: nine quotes, three ADR-005 lines, three context lines, ADR-008's consequence and the updated line. Decision ids and their order are the same as at HEAD. id, title, status, scope, date, decided_by, supersedes and superseded_by are unchanged for every decision. project is unchanged. updated is 2026-09-28, today's date, which the landing commit is expected to carry. scripts/design/schemas/decisions.schema.json was not touched. I did not run gate.sh or validate.py myself because the brief reserves them for the workflow. I did render every cluster into a temp copy, the same way gate.sh does, and every committed markdown file matched.
- Deviation: Several acceptance rows say 18 decisions and compare against 7b53625, but main has moved on. The ledger now holds 78 decisions, and ADR-012 to ADR-015 had their consequences changed by other cards after 7b53625. The brief's own verification steps say 17/17 lines against 7b53625; that count holds only against HEAD. Reverting other cards' landed work would break their rows and the boundary against changing other fields. So I kept HEAD's content and applied only this brief's changes: the diff against the parent commit is exactly 17/17, and ADR-001 to ADR-011 were identical at 7b53625 and HEAD. Also, updated was set to 2026-09-28 on the assumption that the landing commit carries today's date.
- Files changed:
  - modified: `docs/design/decisions.json` — updated is now 2026-09-28. The file is serialised as json.dumps(indent=2, ensure_ascii=False) plus a newline, and every field R1 and R2 do not name is unchanged.
- Checklist delivery:
  - [x] C8 — Every field of every decision that C1 to C7 do not name is byte-identical to its value at 7b53625. — Compared against HEAD, the only fields that differ are the ones C1 to C7 name.
  - [x] C9 — docs/design/decisions.json holds 18 decisions, ADR-001 to ADR-018 in order. — ADR-001 to ADR-018 are still the first 18 decisions, in order. The ledger now holds 78 decisions in total, added by other cards.
  - [x] C10 — The ledger's updated field is the date of the commit that makes the change. — updated is 2026-09-28, the expected commit date.
  - [x] C11 — docs/design/decisions.json keeps the serialisation of json.dump(indent=2, ensure_ascii=False) plus a trailing newline. — The file was written with json.dumps(indent=2, ensure_ascii=False) plus a newline, and the round-trip is byte-identical.
  - [x] C12 — scripts/design/schemas/decisions.schema.json is unchanged from 7b53625. — The schema file is untouched.
  - [x] C13 — sh scripts/design/gate.sh exits 0. — I expect gate.sh to pass: the temp-copy render of every cluster matches the committed markdown, and the schema and serialisation are unchanged. I did not run gate.sh itself.
- Story delivery:
  - [x] S3 (Brief author, Anchoring a decision) — As a brief author anchoring a decision, I want its decision, context and consequences to mean exactly what was ruled so that a review against the ledger checks the same obligations as before. — The meaning of every decision is preserved.
  - [x] S4 (Card lead, Landing a document change) — As a card lead landing the change, I want the design gate to pass with the schema untouched so that the ledger stays valid under the method it is measured by. — The schema is untouched, and every rendered cluster matches its committed markdown.

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

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: I ran python3 scripts/design/render-cluster.py on docs/design/directory, docs/design/decisions-words and docs/design/rauthy-rebase; no markdown was edited by hand. Each changed markdown file differs from HEAD by exactly one line added and one removed. That line is the ADR-005 entry in the '- ADR-005 — The identity database is PostgreSQL, possibly on a network device — …' form, which now ends '…rather than on the operator's workstation.' No cluster's JSON changed. I then rendered every cluster that has a design.json into a temp copy, the way gate.sh does, and found no differing markdown, so no 'rendered markdown differs' line is expected.
- Deviation: The brief names six rendered files, but on the current main, five newer directory briefs (DIRECTORY-021, 025, 030, 031, 041) and two rauthy-rebase files (DESIGN.md and RAUTHYREBASE-001.md) also render ADR-005's decision. Leaving them stale would make gate.sh report 'rendered markdown differs', which would break CN5 and R4's gate acceptance. So they were regenerated by the renderer, one line each, and the landing's file list is thirteen markdown files plus decisions.json, not six plus one. The PAGE.md files of directory, roots, rauthy-rebase and decisions-words still mention the old machine name, but render-cluster.py does not produce them and the boundary forbids editing them, so they were left unchanged.
- Files changed:
  - modified: `docs/design/directory/DESIGN.md` — Re-rendered; its ADR-005 decision line now names the operator's workstation.
  - modified: `docs/design/directory/briefs/DIRECTORY-001.md` — Re-rendered ADR-005 line.
  - modified: `docs/design/directory/briefs/DIRECTORY-002.md` — Re-rendered ADR-005 line.
  - modified: `docs/design/directory/briefs/DIRECTORY-006.md` — Re-rendered ADR-005 line.
  - modified: `docs/design/directory/briefs/DIRECTORY-021.md` — Re-rendered ADR-005 line; this brief postdates the one I was given.
  - modified: `docs/design/directory/briefs/DIRECTORY-025.md` — Re-rendered ADR-005 line; this brief postdates the one I was given.
  - modified: `docs/design/directory/briefs/DIRECTORY-030.md` — Re-rendered ADR-005 line; this brief postdates the one I was given.
  - modified: `docs/design/directory/briefs/DIRECTORY-031.md` — Re-rendered ADR-005 line; this brief postdates the one I was given.
  - modified: `docs/design/directory/briefs/DIRECTORY-041.md` — Re-rendered ADR-005 line; this brief postdates the one I was given.
  - modified: `docs/design/decisions-words/DESIGN.md` — Re-rendered ADR-005 line.
  - modified: `docs/design/decisions-words/briefs/DECISIONSWORDS-001.md` — Re-rendered ADR-005 line.
  - modified: `docs/design/rauthy-rebase/DESIGN.md` — Re-rendered ADR-005 line; this cluster also cites ADR-005.
  - modified: `docs/design/rauthy-rebase/briefs/RAUTHYREBASE-001.md` — Re-rendered ADR-005 line.
- Checklist delivery:
  - [x] C14 — Every committed markdown file that renders ADR-005's decision is the output of scripts/design/render-cluster.py over the changed ledger. — Every committed markdown file that renders ADR-005's decision is now the renderer's output over the changed ledger, confirmed by a render of every cluster into a temp copy.
- Story delivery:
  - [x] S4 (Card lead, Landing a document change) — As a card lead landing the change, I want the design gate to pass with the schema untouched so that the ledger stays valid under the method it is measured by. — No rendered markdown differs from what its JSON renders to, and the schema is untouched.

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
