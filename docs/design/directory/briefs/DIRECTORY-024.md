---
type: brief
id: DIRECTORY-024
cluster: directory
title: List everything a person cannot give on the delegation form, each with its one reason
---

# DIRECTORY-024: List everything a person cannot give on the delegation form, each with its one reason

> **Cluster:** directory
> **Depends on:** DIRECTORY-005, DIRECTORY-006
> **Blocked by:** Sign-off recorded on the card. The card is not built from this brief before that sign-off is recorded., DIRECTORY-006 landed on lys main, checked from a clone of lys by: git fetch origin main && git cat-file -e origin/main:crates/lys-identity/src/grants/lineage.rs && git grep -q UseOnly origin/main -- crates/lys-identity/src/grants/types.rs && git cat-file -e origin/main:surface/identity/src/features/grants/DelegateGrant.tsx — which exits 0 only once DIRECTORY-006's lineage decision, its PassOn type and its delegation form are on main. No separate ratification of the grant contract is waited for. Every modify path below is a file DIRECTORY-006 or DIRECTORY-005 creates, reconciled against its landed form before dispatch (CN12)., The one dependency on the grant representation: this brief reads the two fields DIRECTORY-006 builds, source (null for a root grant) and pass_on (use_only, or to with actions and recipient kinds). The grant representation stays recorded as open (C5); if a ratification renames or drops source or pass_on, this brief is amended to follow it.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> **Checklist:**
> - C181 — The delegation form's answer lists, for the chosen recipient, every grant in force the person holds on any resource that they cannot give, every relation on the source grant's resource that no grant they hold covers in any standing, their sign-in identity when the recipient is an agent, and each service account they hold to which a reason applies, marks the source grant when it is listed, and lists nothing they can give and nothing not in force.
> - C182 — Each item carries exactly one reason, the first applicable in the order sign_in_identity, above_what_you_hold, lent_to_you, use_only, people_only, agents_only; lent_to_you and use_only are told apart by whether the person's use-only grant names a source, and coverage is decided by the model's action sets, never by a rank of names.
> - C183 — The cannot-give answer comes from the one authenticated grant seam, is byte-identical for API, tool and browser callers, refuses an unknown reason on the typed contract, and discloses nothing the person cannot discover.
> - C184 — The delegation screen shows exactly the items and reasons the answer lists, asks again when the recipient changes and discards a superseded answer, and refuses an answer carrying an unknown reason by name, showing no item and no blank.
> - C185 — Conformance row 2.4 is carried by acceptance lines that name it, and its Brief column names DIRECTORY-024.
> **Stories:**
> - S76 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person giving part of my access, I want the form to list everything I cannot give to the recipient I chose, each with the one reason that stops it, so that I know what and why before I try, from the server's answer rather than my browser's guess.

## Purpose

Complete conformance row 2.4 of the grant and delegation work, under CONFORMANCE's build-order step 2 as an amendment beside DIRECTORY-006: the server's answer to the delegation form lists, for the chosen recipient, everything in force that the person cannot give, each item with exactly one reason from a closed set chosen by its fixed precedence, and the screen shows that list and nothing the answer did not list, because the browser is never the authority. DIRECTORY-006 refuses a forbidden delegation; this brief says, before the attempt, everything that would be refused and why.

## Task

Implement R1 to R4 in order after DIRECTORY-006 (R1, R2, R4 and R5 for this brief's R1 and R2; R6 for R3) and DIRECTORY-005. R1 computes the list in lys-identity from DIRECTORY-006's authority and lineage decisions; R2 carries it on the one authenticated seam as a typed operation of the grant contract; R3 renders it in the delegation form; R4 names this brief in row 2.4's Brief column. In scope: the per-recipient cannot-give answer over every grant in force the person holds on any resource, the relations above what they hold on the source grant's resource, the person's own sign-in identity, and each service account they hold under row 1.3 to which a reason applies; the closed six reason values and their precedence, the mark on the source grant, the screen's rendering and its refusal by name of an unknown reason. Out: grants not in force (they show on their own grant card as void, under DIRECTORY-006 R6), any standing on an item, what a suspended ancestor does to its descendants, any change to DIRECTORY-006's admission rules or text, lending from a secret in the secrets broker (CONFORMANCE 7.3, SECRETS-002), CONFORMANCE row 2.4's Behaviour text, and any published lys wire format. Split with DIRECTORY-006: C29 and C30 stay DIRECTORY-006's, and this brief takes row 2.4 through C181 to C185.

## Requirements

### R1: Compute the cannot-give list with one reason per item from the closed set and its precedence

The reason set is a closed enumeration of six values, declared in precedence order: sign_in_identity, above_what_you_hold, lent_to_you, use_only, people_only, agents_only. WHEN asked for the cannot-give list for a source grant the caller holds and a chosen recipient, THE SYSTEM SHALL list: (a) every grant in force that the person holds on any resource that is not a service account, that they cannot give to that recipient, as a grant item; (b) every relation the model declares on the source grant's resource that no grant the person holds on that resource covers, in any standing, as a relation item; (c) the person's own sign-in identity as one standing item, WHEN the recipient is an agent; and (d) each service account the person holds under conformance row 1.3, through a grant in force whose resource is that service account, to which one of the six reasons applies, as a service-account item. A grant whose resource is a service account is listed once, as a service-account item under (d), and never also as a grant item under (a). A grant is in force exactly when DIRECTORY-006's evaluator answers that it is in force, for the grant itself and for its ancestry; this brief carries no test of its own for revocation, expiry or suspension. A grant covers a relation when the relation's action set is a subset of the grant's. For a grant or service-account item the reasons that apply are read from the fields of the person's own grant: use_only when its pass_on is use_only, whoever issued it, the directory administrator included; lent_to_you when its pass_on is use_only and its source is not null; people_only when its pass_on admits only people as recipients and the recipient is an agent; agents_only when its pass_on admits only agents as recipients and the recipient is a person. A relation item's reason is above_what_you_hold; the sign-in identity item's reason is sign_in_identity. The order puts every reason that no choice of recipient can change before the two reasons about the chosen recipient, people_only and agents_only, so that the list never suggests that picking another recipient would make an item givable when it would not. THE SYSTEM SHALL give each item exactly one reason, the first applicable reason in precedence order. An item SHALL be marked as the source WHEN it is the grant the form was opened from, and no other item is marked. A grant that is not in force SHALL NOT be listed, and no item SHALL carry a standing. A grant or service account whose pass_on admits the recipient's kind SHALL NOT be listed. A relation covered by a grant the person holds, in any standing, SHALL NOT be listed as a relation item. An ancestor's end date that has not passed SHALL NOT give any reason. Coverage SHALL be decided by the model's action and resource sets and SHALL NOT use a rank of relation names or display strings. The computation SHALL read the person's grants, their ancestry and the recipient's kind through DIRECTORY-006's authority and lineage decisions and SHALL NOT carry a second admission check. THE SYSTEM SHALL NOT enumerate anything from records the person cannot discover: every item comes from the person's own grants and accounts or from the model's relations on the source grant's resource, and no item carries an ancestor's id, holder or label. Items are ordered as follows: grant items and service-account items together by the bytes of their grant id, each grant appearing once; then relation items in the model's declared relation order; then the sign-in identity item.

**Acceptance:**
- CANNOT_GIVE_FIXTURE (conformance row 2.4): the fixture model declares four relations on project P whose names do not imply their action sets: alder {view}, birch {view, comment}, cedar {view, edit}, damson {view, comment, edit, grant}. Fixture person P1 holds, all in force: G1, alder on P, source null, pass_on to {view} for recipient kinds {person, agent}; G2, birch on P, source null, pass_on use_only; G3, cedar on P, source G0 (held by fixture person P3), pass_on use_only. P1 holds nothing covering damson and has one linked sign-in identity. For source G1 and fixture agent A1 the list holds exactly 4 items, in this order: G2 use_only, G3 lent_to_you, damson above_what_you_hold, sign-in identity sign_in_identity; G1 and alder are not listed; each item carries exactly one reason, none carries a standing and none is marked as the source.
- CANNOT_GIVE_PERSON_RECIPIENT (conformance row 2.4): the same fixture, source G1 and fixture person P2 as recipient, gives exactly 3 items, in this order: G2 use_only, G3 lent_to_you, damson above_what_you_hold; no item has reason sign_in_identity.
- CANNOT_GIVE_PRECEDENCE (conformance row 2.4): in the fixture both use_only and lent_to_you apply to G3 (pass_on use_only, source G0). For source G1 with recipient A1, and again with recipient P2 (2 legs, counted), G3's item carries exactly one reason, lent_to_you, and no item carries use_only for G3.
- CANNOT_GIVE_ORDER_TABLE: for each of the 63 non-empty subsets of the six reasons the reason chosen is the subset's first member in the order sign_in_identity, above_what_you_hold, lent_to_you, use_only, people_only, agents_only; the test asserts it ran exactly 63 cases.
- CANNOT_GIVE_PEOPLE_ONLY (conformance row 2.4): the fixture plus G11, a grant on resource U, source null, in force, pass_on to its actions for recipient kinds {person} only. Source G1, recipient A1: the list holds exactly 5 items and G11 carries exactly one reason, people_only. Source G1, recipient P2: the list holds exactly the 3 items of CANNOT_GIVE_PERSON_RECIPIENT and no item names G11.
- CANNOT_GIVE_AGENTS_ONLY (conformance row 2.4): the fixture plus G10, a grant on resource S, source null, in force, pass_on to its actions for recipient kinds {agent} only. Source G1, recipient P2: the list holds exactly 4 items and G10 carries exactly one reason, agents_only. Source G1, recipient A1: the list holds exactly the 4 items of CANNOT_GIVE_FIXTURE and no item names G10.
- CANNOT_GIVE_IN_FORCE_ONLY (conformance row 2.4): the fixture plus G5, a grant on resource R, source null, pass_on use_only, revoked; G9, damson on P, source null, pass_on to its actions for recipient kinds {person, agent}, its end passed under a controlled clock; and G14, a grant on resource V, source G13 (held by P3), pass_on use_only, where G13 is revoked. Source G1, recipient A1: the list holds exactly 3 items, in this order: G2 use_only, G3 lent_to_you, sign-in identity sign_in_identity; no item names G5, G9, G13 or G14, no item is the relation damson, and no item carries a standing.
- CANNOT_GIVE_SOURCE_MARK (conformance row 2.4): the fixture, source G2, recipient A1: the list holds the same 4 items as CANNOT_GIVE_FIXTURE, G2's item is marked as the source, and exactly 1 item is marked.
- CANNOT_GIVE_SERVICE_ACCOUNT (conformance row 2.4): the fixture plus G7, a grant on service account SA1, source null, in force, pass_on use_only. Source G1, recipient A1: the list holds exactly 5 items, in this order: G2 use_only, G3 lent_to_you, SA1 use_only, damson above_what_you_hold, sign-in identity sign_in_identity; SA1 appears once, as a service-account item, and no grant item names G7.
- CANNOT_GIVE_SERVICE_ACCOUNT_GIVABLE (conformance row 2.4): the fixture plus G8, a grant on service account SA2, source null, in force, pass_on to its actions for recipient kinds {person, agent}. Source G1, recipient A1: the list holds exactly the 4 items of CANNOT_GIVE_FIXTURE and no item names SA2 or G8.
- CANNOT_GIVE_NO_RANK: the fixture with the relation names alder and damson swapped, so alder is {view, comment, edit, grant} and damson is {view}, and G1 holds damson. Source G1, recipient A1: alder carries above_what_you_hold and damson is not listed.
- CANNOT_GIVE_ADMISSION_AGREES: for each of G1, G2 and G3 and each of the 2 recipients A1 and P2 (6 legs, counted), the grant is absent from the list exactly when DIRECTORY-006's delegate admission permits delegating it to that recipient.
- CANNOT_GIVE_DETERMINISTIC: the fixture built with P1's grants inserted in reverse order gives a list serialised byte-identical to the fixture's.

**Files:**
- create: crates/lys-identity/src/grants/cannot_give.rs
- create: crates/lys-identity/tests/grant_cannot_give.rs
- modify: crates/lys-identity/src/grants/mod.rs

**Checklist:**
- C181 — The delegation form's answer lists, for the chosen recipient, every grant in force the person holds on any resource that they cannot give, every relation on the source grant's resource that no grant they hold covers in any standing, their sign-in identity when the recipient is an agent, and each service account they hold to which a reason applies, marks the source grant when it is listed, and lists nothing they can give and nothing not in force.
- C182 — Each item carries exactly one reason, the first applicable in the order sign_in_identity, above_what_you_hold, lent_to_you, use_only, people_only, agents_only; lent_to_you and use_only are told apart by whether the person's use-only grant names a source, and coverage is decided by the model's action sets, never by a rank of names.

**Stories:**
- S76 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person giving part of my access, I want the form to list everything I cannot give to the recipient I chose, each with the one reason that stops it, so that I know what and why before I try, from the server's answer rather than my browser's guess.

### R2: Answer the cannot-give list on the one authenticated seam with a closed wire enumeration

THE SYSTEM SHALL expose the cannot-give list as a typed operation of DIRECTORY-006 R5's authenticated grant seam in the standalone identity server, taking the source grant id and the recipient id, calling R1 and nothing else to compute it, so API, tool and browser callers get the same answer. The answer SHALL carry the source grant id, the recipient id and the ordered items; each item SHALL carry its subject kind (grant, relation, service_account or sign_in_identity), the person's own grant id for a grant or service-account item and the relation name for a relation item, exactly one reason spelled as one of the six snake_case values of R1, and the source mark; no item SHALL carry a standing. The contract type SHALL refuse by name, and SHALL NOT default, an answer whose reason is outside the six. IF the caller is unauthenticated, THEN THE SYSTEM SHALL refuse by name with no items. IF the source grant is not one the caller holds, or the recipient is unknown, THEN THE SYSTEM SHALL refuse by name and SHALL NOT disclose whether a grant or identity the caller cannot discover exists. The answer SHALL NOT include another identity's private grants, labels or ids. The operation reads only and SHALL NOT create a grant or an event.

**Acceptance:**
- CANNOT_GIVE_ROUTES (conformance row 2.4): R1's fixture, caller P1, source G1, recipient A1, asked through the API route, the tool route and the browser route (3 routes, counted): the three answers are byte-identical and their items equal R1's CANNOT_GIVE_FIXTURE list.
- CANNOT_GIVE_WIRE (conformance row 2.4): the answer for R1's fixture serialises the reasons as the strings use_only, lent_to_you, above_what_you_hold and sign_in_identity and carries no standing member on any item; the answer for CANNOT_GIVE_PEOPLE_ONLY with recipient A1 serialises G11's reason as people_only; the answer for CANNOT_GIVE_AGENTS_ONLY with recipient P2 serialises G10's reason as agents_only.
- CANNOT_GIVE_WIRE_UNKNOWN_REASON (conformance row 2.4): parsing an answer whose one item has reason "borrowed" fails with the named error for an unknown cannot-give reason and yields no answer value.
- CANNOT_GIVE_AUTH (conformance row 2.4): an unauthenticated request is refused by name with zero items; P2 asking with source G1 is refused with the same refusal body as P2 asking with a grant id that exists nowhere; the grant and event counts are unchanged after every request.
- CANNOT_GIVE_UNKNOWN_RECIPIENT (conformance row 2.4): P1 asks with source G1 and recipient id X0, which exists nowhere, and again with source G1 and recipient X1, a fixture identity outside what P1 may discover under DIRECTORY-006 R5 and R6: both requests are refused by name, the two refusal bodies are byte-identical, each carries zero items, and the grant and event counts are unchanged after each request.
- CANNOT_GIVE_NO_LEAK (conformance row 2.4): adding P2's private grant of damson on P to R1's fixture leaves P1's answer for source G1 and recipient A1 byte-identical.

**Files:**
- create: crates/lys-identity-server/tests/grant_cannot_give.rs
- modify: crates/lys-identity-server/src/grant_contract/mod.rs
- modify: crates/lys-identity-server/src/grant_contract/requests.rs
- modify: crates/lys-identity-server/src/grant_contract/views.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C183 — The cannot-give answer comes from the one authenticated grant seam, is byte-identical for API, tool and browser callers, refuses an unknown reason on the typed contract, and discloses nothing the person cannot discover.

**Stories:**
- S76 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person giving part of my access, I want the form to list everything I cannot give to the recipient I chose, each with the one reason that stops it, so that I know what and why before I try, from the server's answer rather than my browser's guess.

### R3: Show on the delegation form exactly the list the answer gives, and refuse an unknown reason by name

WHEN the delegation form is opened from a source grant, and again WHEN its To choice changes, THE SYSTEM SHALL ask R2's operation with the source grant and the chosen recipient and SHALL render the answer's items in the answer's order, each with its reason and the source mark. WHEN an answer arrives whose recipient is not the current To choice, THE SYSTEM SHALL discard it and SHALL NOT render any of its items. The screen SHALL NOT compute, add, drop, merge or reorder an item, and SHALL NOT derive a reason from grant data it holds. WHEN an item's reason is agents_only, THE SYSTEM SHALL show it with the words "This can be passed on only to an agent." IF an answer carries a reason outside the six, THEN THE SYSTEM SHALL refuse the whole answer with a refusal named unknown_cannot_give_reason, and SHALL NOT render any item of that answer or a blank reason.

**Acceptance:**
- CANNOT_GIVE_SCREEN (conformance row 2.4): rendered from R2's answer for R1's fixture (source G1, recipient A1), the list shows exactly 4 rows, the answer holds exactly 4 items, and the rows' (subject, reason) pairs in order equal the answer's: G2 use_only, G3 lent_to_you, damson above_what_you_hold, sign-in identity sign_in_identity.
- CANNOT_GIVE_SCREEN_NO_OTHER (conformance row 2.4): rendered from that answer with the damson item removed, the list shows exactly 3 rows and no damson row, although the fixture grants the screen holds would place damson above what P1 holds.
- CANNOT_GIVE_SCREEN_RECIPIENT (conformance row 2.4): with the form open for A1, changing To to P2 sends exactly one new request carrying recipient P2, and the list then shows exactly the 3 rows of R1's CANNOT_GIVE_PERSON_RECIPIENT answer, with no sign-in identity row.
- CANNOT_GIVE_SCREEN_AGENTS_ONLY (conformance row 2.4): rendered from R2's answer for CANNOT_GIVE_AGENTS_ONLY with recipient P2, the list shows exactly 4 rows and G10's row, the only row with reason agents_only, shows the words "This can be passed on only to an agent."
- CANNOT_GIVE_SCREEN_STALE (conformance row 2.4): with the form open for A1, changing To to P2 and back to A1 before the P2 reply arrives, then delivering the A1 reply before the P2 reply, leaves the list showing exactly the 4 rows of the A1 answer.
- CANNOT_GIVE_SCREEN_UNKNOWN_REASON (conformance row 2.4): an answer of 4 items whose third has reason "borrowed" renders 0 list rows and one refusal named unknown_cannot_give_reason.

**Files:**
- create: surface/identity/src/features/grants/CannotGiveList.tsx
- create: surface/identity/src/features/grants/cannotGiveAnswer.ts
- create: surface/identity/tests/cannot_give.test.tsx
- modify: surface/identity/src/features/grants/DelegateGrant.tsx
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C184 — The delegation screen shows exactly the items and reasons the answer lists, asks again when the recipient changes and discards a superseded answer, and refuses an answer carrying an unknown reason by name, showing no item and no blank.

**Stories:**
- S76 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a person giving part of my access, I want the form to list everything I cannot give to the recipient I chose, each with the one reason that stops it, so that I know what and why before I try, from the server's answer rather than my browser's guess.

### R4: Name this brief in conformance row 2.4

Change the Brief column of row 2.4 in docs/design/identity/CONFORMANCE.md from "DIRECTORY (new row)" to "DIRECTORY-024". No other cell, row or line of that file changes, and no other file under docs/design/identity changes.

**Acceptance:**
- CANNOT_GIVE_CONFORMANCE_ROW (conformance row 2.4): in docs/design/identity/CONFORMANCE.md the row whose first cell is 2.4 has DIRECTORY-024 as its Brief cell, its other four cells are unchanged, and git diff of that file against the brief's base shows exactly one line removed and one line added, both that row.

**Files:**
- modify: docs/design/identity/CONFORMANCE.md

**Checklist:**
- C185 — Conformance row 2.4 is carried by acceptance lines that name it, and its Brief column names DIRECTORY-024.

## Boundaries

- DIRECTORY-006 R6 shows the server's reason for a refused choice, while this brief defines the closed reason set and the full per-recipient list that R6's screen consumes; DIRECTORY-006's text does not change.
- The sentence of docs/design/directory/DESIGN.md at line 63, "Road step 2 onward: capability certificates, arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production", stands as written and, by the lead's answer, does not exclude this brief, as it does not exclude DIRECTORY-006; no sentence of DESIGN.md is edited by hand, and DESIGN.md gains only the Structure rows render-cluster.py produces from design.json.
- CN1 governs the planning documents of the directory design round and nothing else; this brief's build walls are its own R1 to R4 files, CN1 is no bar on its code paths or on its one-column change to CONFORMANCE.md row 2.4, and no file outside those walls changes.
- In docs/design/identity/CONFORMANCE.md only row 2.4's Brief column changes; the row's Behaviour text is not edited.
- The reason set is the six values sign_in_identity, above_what_you_hold, lent_to_you, use_only, people_only, agents_only, in that precedence; no value is added, renamed or made open-ended, and no free-text reason stands in for one.
- The list holds only what is in force, as the pinned mock-up draws it: a grant that is not in force is not listed and carries no reason here; it shows on its own grant card as void with its named reason, under DIRECTORY-006 R6, and no item carries a standing.
- This brief decides no suspension policy (DESIGN.md line 58 and ADR-011 stay open): whether a grant is in force is DIRECTORY-006's evaluator's answer, and what a suspended ancestor does to its descendants is left to the lifecycle decision.
- The fields of the person's own grant decide its reason: pass_on, the presence of a source and the recipient kinds pass_on admits; the identity of the issuer never does. The grant representation stays recorded as open (DESIGN.md line 57, C5); if a ratification renames or drops source or pass_on, this brief is amended to follow it.
- The browser computes no authority: it renders the server's answer, asks again when the recipient changes, and never adds, drops or re-derives an item or a reason.
- No second evaluator: the list is computed from DIRECTORY-006's authority and lineage decisions and exposed only through its one authenticated seam; DIRECTORY-006 R2's delegation rules and R5's seam are held as written.
- Nothing in crates/lys/src/identity/ checks or writes SpiceDB (DIRECTORY-002 R4); this brief adds no permission enforcement.
- A sign-in identity is never offered as givable to an agent (CONFORMANCE 1.2); sign-in identities are the Rauthy fork's provider links (ADR-009) and nothing in vendor/rauthy changes.
- The list never discloses another identity's private grants, labels, ids or existence to say no (DIRECTORY-006 R6).
- No lys-core code and no published or frozen wire format changes; lys/delegation/v1 is not used as the grant format.
- The card is not built from this brief before sign-off is recorded on the card.

## Verification

- From the repository root: sh scripts/design/gate.sh exits 0, and python3 scripts/design/render-cluster.py docs/design/directory run twice leaves git status unchanged after the first run.
- DESIGN.md is byte-identical to render-cluster.py's output, and git diff docs/design/directory/DESIGN.md against the brief's base touches only the Structure table.
- At implementation time run the repository battery from the exact revision: cargo fmt --check, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps, plus the surface's strict type check and tests; report each CANNOT_GIVE_* case with its counted legs.
- Confirm by search of crates/lys-identity, crates/lys-identity-server and surface/identity/src that the strings new to this brief, sign_in_identity, above_what_you_hold, lent_to_you, people_only, agents_only and unknown_cannot_give_reason, appear only in crates/lys-identity/src/grants/cannot_give.rs, crates/lys-identity-server/src/grant_contract/requests.rs, crates/lys-identity-server/src/grant_contract/views.rs, the generated types, cannotGiveAnswer.ts, CannotGiveList.tsx and the tests; use_only is left out of the search because DIRECTORY-006's grant contract already carries it.
- Confirm git diff docs/design/identity/ touches only CONFORMANCE.md row 2.4, and git diff docs/design/directory/briefs/DIRECTORY-006.json docs/design/directory/briefs/DIRECTORY-006.md is empty.
