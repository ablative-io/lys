# secrets — what was asked, what it means, and what was written

## The words, as they were typed

Write the brief for this card in the secrets cluster (docs/design/secrets/briefs, the next id not used on main or on any origin branch, which at 13:00 on 27 September was SECRETS-004, so take the id from the branches at write time), amending SECRETS-002 for two rows of docs/design/identity/CONFORMANCE.md, 7.6 in its test part and 7.8, and giving each row at least one acceptance line that names it. Row 7.6 says who may revoke a lease. The person acted for may revoke a lease they are acted for under at any time, and the revoke is the two facts SECRETS-002 R7 and R8 already state, issuing stops at once and the system behind confirms separately with pending shown until then. Anyone who is neither the person acted for nor the lease's holder is refused when they try. A caller who cannot discover the lease gets the not-found refusal SECRETS-002 R6 gives and nothing is named, and a caller who can see it gets a refusal that names the lease. The holder may give its own lease back, which is a relinquish, a separate act recorded as such that stops issuing at once. The refusal covers only the direct act of revoking a lease; withdrawal of an upstream grant under ADR-003 and SEC_REVOKE_CHAIN and emergency stop under row 3.4 end a lease as their own acts and are untouched. Whether the owner of a secret may revoke every credential derived from it is a policy choice not conferred by ownership alone, and this brief records it as a proposed ADR in decisions.json, recommendation that the person acted for revokes and ownership alone confers no revoke, rejected alternative that the owner revokes every derived credential, cited from the Decisions section by id, with no test and no code that grants it, so that until it is ruled the set of who may revoke beyond the person acted for is empty. Row 7.8 is the scope filter on the secrets list. The list takes one scope from a closed set of three, organisation, team and mine, the server applies the scope and answers only the secrets the caller may see within it, and the browser shows what came back and nothing else. Mine is the secrets whose scope is personal and whose owner is the caller. Team is the secrets whose scope is team and whose team is one the caller belongs to. Organisation is the secrets whose scope is organisation. Each value is keyed on the secret's own scope as the mock-up and SECRETS-002 R6 have it, and the list with no scope chosen is everything the caller may see, which is the mock-up's All and not a fourth value. A scope outside the set is refused by name. Done when a fixture of two people in two teams with secrets owned by each gives, for one caller, three lists whose members are exactly those the scope rule names, when a secret the caller may not see is absent from all three, when a revoke by the person acted for stops issuing at once and shows pending until confirmed, and when a revoke by a third person is refused naming the lease. Hold to ADR-001 to ADR-004, to R4 of DIRECTORY-002 and to ADR-009, so that every visibility and revoke check asks SpiceDB. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after Tom or the lead signs it off on the card. Rulings of the lead, Apollo, given on 27 September 2026 to run a6ce8a7a-7807-42bf-a0bb-9e74bb99672d in answer to its rounds 1 and 2, read by Archie as second reader. That run took the answers and then ended push_refused on its draft branch. They are settled here and the author reopens none of them. The two facts of a revoke are SEC_REVOKE_STATES in SECRETS-002 R7, and R8's in-flight rule stays open and outside this acceptance. The server's answer is this brief's whole of row 7.6, GET /leases/{lease_id} reading issuing stopped and upstream pending until confirmed; the lease screen with its Revoke control is outside this brief and named as the neighbour. Team ids for the signed-in person come from the identity provider's group claims on their token, one group per team, read through one function named in the brief, until a directory brief models teams and replaces that source in that one function; tests inject the claims. The host is crates/lys-secrets following ADR-019, and the requirement that touches it is blocked on crates/lys-secrets/Cargo.toml being present on origin/main by git cat-file -e. DESIGN.md's Solution and Intention sentences that put the broker inside the door are quoted as a question for the lead and not rewritten; they change when SECRETS-003 lands with ADR-019, by that brief's documents. Holding to R4 of DIRECTORY-002 means the narrow thing: no SpiceDB permission check is added in the step-1 paths R4 forbids, the checks are calls on the one evaluator seam the step-2 SpiceDB brief introduces, answering from the record's scope, team and owner fields until that brief lands, and the requirement that makes the seam ask SpiceDB is blocked on that brief's evaluator file by git cat-file -e, never on commit text. This brief is a new brief with depends_on SECRETS-002 whose rows name the SECRETS-002 requirements they extend; CHECKLIST.md C2 and DESIGN.md goal 2 widen from covered by a SECRETS-002 requirement to covered by a requirement of a SECRETS brief, one sentence each. Agents never use the secrets list, so an agent's team membership changes no list.

Rulings. The lead settled these in brief run 107557cf-3649-41b3-bd95-73296af454ff on 27 September 2026. Each is decided, so the brief takes it as given and does not ask it again.

The lead was asked this.
Main has no ADR-019, and three branches each claim ADR-019 for a different decision.
Should SECRETS-004 cite the host decision as ADR-019, in design_anchor or in prose, or should it cite it by the SECRETS-003 brief until the ledger on main carries it?
The lead ruled as follows.
Cite it by the brief, not by the number.
SECRETS-004 names the host decision in prose as the decision SECRETS-003 carries on brief/secrets/309540a4, that the broker's host is crates/lys-secrets, and puts no ADR id for it in design_anchor, since main's ledger ends at ADR-018 and validate.py refuses an anchor the ledger lacks.
My words following ADR-019 are corrected to following the host decision of SECRETS-003.
The number is taken from main's ledger by whoever lands first, and the block on crates/lys-secrets/Cargo.toml being present on origin/main by git cat-file -e stays as the only thing the requirement waits for.

The lead was asked this.
Does this brief build the secrets list screen with its scope control (All, Organisation, Team, Mine), or only the server's list answer, with the screen named as a neighbour the way the lease screen is?
The lead ruled as follows.
Only the server's list answer.
The secrets list screen with its scope control of All, Organisation, Team and Mine is outside this brief and is named as the neighbour, as the lease screen is, since surface/identity does not exist on main.
The sentence that the browser shows what came back and nothing else is kept in the brief's boundaries as the rule the neighbour is held to, with no requirement and no test of its own here, and every done-when line stays a server fixture.

The lead was asked this.
Does a relinquish also show upstream pending until the system behind confirms it, as a revoke does, or only stop issuing at once?
The lead ruled as follows.
Both facts, as a revoke.
A relinquish stops issuing at once and shows upstream pending until the system behind confirms, since the credential at the system behind exists whoever ended the lease.
GET /leases/{lease_id} after a relinquish reads ended by relinquish, issuing stopped, and upstream pending until confirmed and confirmed after.
One acceptance line asserts the three readings in that order.

The lead was asked this.
When a lease's holder calls revoke, rather than relinquish, on its own lease, is that refused and pointed at relinquish, or recorded as a relinquish?
The lead ruled as follows.
Refused and pointed at relinquish.
A revoke call by the lease's holder on its own lease is refused by name, the refusal names the lease and names relinquish as the holder's act, and nothing is recorded as a relinquish that the caller did not ask for, so that the record of each act says what was asked.
A holder who is also the person acted for under that lease revokes as the person acted for.
The sentence of the words that exempts the holder from the refusal is corrected by this answer to say that a revoke is refused for anyone but the person acted for, the holder with the refusal that points at relinquish and a third person with the refusal that names the lease.
One acceptance line covers the holder's revoke call.

The lead was asked this.
Who may discover a lease: its person acted for, its holder, and anyone who may see the secret it was issued from (so person_b, who sees a_org, gets 403 naming L1), or only the person acted for and the holder (so every third person gets the 404 that names nothing)?
The lead ruled as follows.
Only the person acted for and the holder discover a lease, and every third person gets the 404 that names nothing.
Seeing the secret a lease was issued from does not make the lease discoverable, so person_b, who sees a_org, gets the 404 for L1 and not a 403 that names it.
A lease says who acted with a secret and for whom, which is more than the secret's own visibility gives, and a 403 naming L1 would tell person_b that the lease exists.
It follows my round 1 ruling that visibility comes before naming.
The two who discover a lease are the two who can act on it in this brief, the person acted for by revoking it and the holder by relinquishing it.
Revocation by the secret's owner is the proposed decision already named as outside this brief, and if it is decided it brings the owner's discovery of the lease with it in its own brief.
The acceptance has a line for each of the three, the person acted for, the holder, and person_b who sees a_org and gets the 404.

The lead was asked this.
R5: an acceptance line is not a measurement — revoke.rs, api/leases.rs and tests/lease_revoke.rs are new; lease.rs and tests/support are SECRETS-003's.
Most lines are measured.
The 24-hour clock advance is a duration nobody traced to a source, and a timer longer than 24 hours would still pass it, so it does not measure 'no timer changes it'.
Which refusal person_b gets on L1 rests on the author's own rule for discovering a lease.
The lead ruled as follows.
Accepted.
R5's acceptance must measure the transition itself, as ruled under r5-confirmed-grep-counts-text.
The holder refusal `holder_relinquishes` and the 404 that names nothing for any third person, the owner included, stand unchanged.

The lead was asked this.
Renumber SECRETS-004's checklist items C19 to C25 and stories S15 to S17 to ids not used on main or on any origin branch.
SECRETS-003 claims C19 to C29 and S15 to S18 for different items on origin/brief/secrets/309540a4 and on its card, card-gate, hand and draft branches, and cites its C24 and C25 by id.
On today's branches that means C30 onward and S19 onward.
Update every reference in SECRETS-004.json, checklist.json, stories.json and design.json, then re-render.
The lead ruled as follows.
Accepted.
Renumber SECRETS-004's checklist items to C30 onward and its stories to S19 onward, after reading main and every origin branch at write time for ids already taken.
Update every reference in SECRETS-004.json, checklist.json, stories.json and design.json, then re-render.

The lead was asked this.
Take the proposed ADR's id from the ids unused on main and on every origin branch at write time.
ADR-054 is now also claimed for other decisions on origin/draft/directory/49cc20db ('The delegation form's cannot-give answer…') and origin/draft/lys-core/12da4769 ('lys recognises test code by structure…').
Update decisions.json, design.json's decisions list, SECRETS-004's text and design_anchor, and the structure note, then re-render.
The lead ruled as follows.
Accepted.
Take the ADR id from the ids unused on main and on every origin branch at write time.
Update decisions.json, the decisions list in design.json, the text and design_anchor of SECRETS-004 and the structure note, then re-render.

The lead was asked this.
Replace R5's acceptance clause that `rg -n 'UpstreamState::Confirmed' crates/lys-secrets/src` prints exactly 1 line.
It counts mentions, not transitions: a match arm that maps the state to its word for GET /leases/{lease_id}, or an assertion in a src/*_tests.rs file, fails a correct build, and a transition made without that literal passes.
Measure the property itself.
For example, the upstream state's field or setter is private to revoke.rs and deliver_upstream_ack is its only writer, checked by a search anchored on the construction or assignment site.
Or a test drives every other public operation on an ended lease and asserts that upstream still reads `pending`.
The lead ruled as follows.
Accepted.
Replace the line-count clause with a test that measures the property.
The test ends a lease, drives every other public operation on it, and asserts that upstream still reads `pending` until deliver_upstream_ack runs, after which it reads `confirmed`.
The upstream state's field stays private to revoke.rs with deliver_upstream_ack as its only writer, and the build proves that by visibility, not by a text search.

The lead was asked this.
R2: an acceptance line is not a measurement — teams.rs and teams_tests.rs are creates. The file lib.rs is created by SECRETS-003 R3 on its branch.
The acceptance line requiring `rg -n 'fn team_ids' crates/lys-secrets/src` to print exactly 1 line also matches any test function in teams_tests.rs whose name starts with team_ids (such as `fn team_ids_empty_groups`), so a correct build fails it.
It counts text, not the one reader.
The lead ruled as follows.
Accepted.
The fix is ruled under r2-team-ids-grep-matches-test-names.

The lead was asked this.
R3: an acceptance line is not a measurement — store.rs, lease.rs, lib.rs and secret.rs are all SECRETS-003 R3 creates, and access.rs and access_tests.rs are new.
The two check-matrix tests are measured, with the number of asks asserted.
The line requiring `rg -n 'scope|team' crates/lys-secrets/src/secret.rs` to print 0 lines counts mentions, not fields.
SECRETS-003 makes secret.rs the file its SEC3_SECRET_SCOPE test (tests/secret_scope.rs) guards, so a doc comment naming that test, or the word scope, fails a correct build.
A field named, say, `visibility` would pass.
The lead ruled as follows.
Accepted.
The fix is ruled under r3-secret-rs-grep-counts-text.

The lead was asked this.
R5: the spec cannot be tested as written — The rulings are honoured: holder_relinquishes, the 404 for the owner and any third person, and L4 as a revoke.
The 8-step pending test measures the property.
The spec, though, says the upstream field is private to revoke.rs and that deliver_upstream_ack is its only writer.
The acceptance also requires deliver_upstream_ack to be the only function in revoke.rs that assigns it.
A revoke must set upstream to pending, and R6's relinquish.rs must too, yet relinquish.rs cannot write a field private to revoke.rs.
As written the rule is either false of a correct build or unimplementable for R6.
The clause 'the only function in revoke.rs that assigns it' is measured by nothing: the compile_fail doctest proves only that code outside the crate cannot write the field.
The lead ruled as follows.
Accepted.
The fix is ruled under r5-upstream-writer-contradicts-pending.

The lead was asked this.
Rewrite R5's rule on the upstream-state writer so that it holds in a correct build and covers R6.
Name the one function in crates/lys-secrets/src/revoke.rs that ends a lease with upstream `pending`, and have revoke and R6's relinquish.rs both call it.
State that deliver_upstream_ack is the only writer of `confirmed`.
Then drop the unmeasured acceptance clause 'deliver_upstream_ack is the only function in revoke.rs that assigns it', or replace it with a measurement: the compile_fail doctest together with the 8-step pending test, extended to run on a relinquished lease.
The lead ruled as follows.
Accepted.
Name one pub(crate) function in crates/lys-secrets/src/revoke.rs, `end_with_upstream_pending`, that ends a lease with upstream pending, and have revoke and R6's relinquish.rs both call it.
State that deliver_upstream_ack is the only writer of confirmed.
Drop the clause about the only function that assigns it, and measure the rule with the compile_fail doctest together with the 8-step pending test, extended to run on a relinquished lease.

The lead was asked this.
Replace R3's acceptance line requiring `rg -n 'scope|team' crates/lys-secrets/src/secret.rs` to print 0 lines.
It counts mentions, not fields, and secret.rs is the file SECRETS-003's SEC3_SECRET_SCOPE test guards.
Measure the type's shape instead, for example a test or compile check showing that the redacting type's fields are unchanged from SECRETS-003 and that scope and team are fields of the store's record type.
The lead ruled as follows.
Accepted.
Replace the line with a test that destructures the redacting type without a rest pattern, so it compiles only while that type's fields are exactly SECRETS-003's, and a test that builds the store's record type with its scope and team fields.

The lead was asked this.
Replace R2's acceptance clause requiring `rg -n 'fn team_ids' crates/lys-secrets/src` to print exactly 1 line.
Test functions in teams_tests.rs whose names start with team_ids also match it, so a correct build fails.
Anchor the check on the definition itself (for example `rg -n 'pub fn team_ids\(' crates/lys-secrets/src` prints exactly 1 line, in teams.rs), or measure the single reader another way.
The lead ruled as follows.
Accepted.
Anchor the check on the definition, so `rg -n 'pub fn team_ids\(' crates/lys-secrets/src` prints exactly 1 line, in teams.rs.

The lead was asked this.
When the person acted for revokes, or the holder relinquishes, a lease that has already ended by revoke or relinquish, does the call succeed and leave the first end record as the only one, or is it refused by name as already ended, with nothing recorded either way?
The lead ruled as follows.
It is refused by name as lease_already_ended, and nothing is recorded.
The refusal carries how and when the lease ended (revoked or relinquished, the instant, and who ended it), read from the first end record.
That first end record stays the only one.
A caller retrying after an uncertain reply reads the refusal and learns that the lease is ended, which is the same knowledge a quiet success would give, and the audit trail never shows a second end that did nothing.
The refusal is the same whichever of the two parties calls and whichever way the lease first ended.
A lease that ended by expiry is refused the same way, with expired as the way it ended.

## What the survey found, and its angles

Write a new brief in docs/design/secrets/briefs that depends on SECRETS-002 and amends it for two rows of docs/design/identity/CONFORMANCE.md. For row 7.6 (the test part): the person acted for revokes a lease, the holder relinquishes its own, a revoke by the holder is refused and pointed at relinquish, and everyone else gets the 404 that names nothing. Whether a secret's owner may revoke is recorded as a proposed ADR that grants nothing. For row 7.8: GET /secrets takes one scope from organisation, team and mine, the server applies it, and with no scope the list is everything the caller may see. The work is server answers in crates/lys-secrets only. Every check is a call on one seam that reads the record's fields until the step-2 SpiceDB evaluator exists, and both screens are named as neighbours.

### What the tree holds

- `docs/design/identity/CONFORMANCE.md` — The two rows being amended: 7.6 at line 85 ('test / open', SECRETS-002 amend) and 7.8 at line 87 ('Scope filter: organisation, team, mine.', test). Row 7.5 (line 84) holds the two facts of a revoke and row 3.4 (line 39) the emergency stop, which this brief does not touch. Line 9 says mock-up sample data is not behaviour.
- `docs/design/secrets/briefs/SECRETS-002.json` — The brief being extended, which must stay unedited. R6 covers personal, team and organisation scopes and denials that do not leak existence. R7 carries SEC_REVOKE_STATES and SEC_REVOKE_CHAIN in its acceptance. R8's in-flight rule stays open. R9 covers leases.
- `docs/design/secrets/design.json / DESIGN.md` — Goal 2 reads 'covered by a SECRETS-002 requirement' and widens to 'a requirement of a SECRETS brief'. The decisions list gains ADR-009 and the proposed ADR, and the structure table gains rows for every path the new brief names. The Solution sentence 'A broker in Rust inside the door' and the Intention sentence 'the door swaps' are the sentences to quote to the lead and leave unchanged.
- `docs/design/secrets/checklist.json / CHECKLIST.md` — C2 widens in the same way as goal 2. The new checklist items go after every id already claimed on main and on the origin branches.
- `docs/design/secrets/stories.json / USER-STORIES.md` — The new stories for the person acted for, the holder, and a person filtering the list.
- `docs/design/decisions.json` — The proposed ADR is added here: the person acted for revokes, and ownership alone confers no revoke. Main's ledger ends at ADR-018, and validate.py refuses a design_anchor the ledger lacks.
- `docs/design/roadmap.json` — RM-002 (secrets, briefed) links SECRETS-001 and SECRETS-002 only, and is the row that would carry this brief.
- `origin/brief/secrets/309540a4-4347-493e-bf0e-d686cfd349b4:docs/design/secrets/briefs/SECRETS-003.json` — The host decision (crates/lys-secrets, ADR-019 on that branch). Its R3 creates lib.rs, secret.rs, store.rs, lease.rs, tests/support/mod.rs and tests/secret_scope.rs (SEC3_SECRET_SCOPE), and R4 defines the revocation states. It has no HTTP layer and no personal/team/organisation field on the stored record. It claims C19–C29 and S15–S18, and cites C24 and C25 by id.
- `crates/lys-secrets/ (absent)` — The host for every code row. crates/lys-secrets/Cargo.toml does not exist on origin/main or on any origin branch, so every code requirement is blocked on `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml`.
- `crates/lys-identity-server/src/spicedb/check.rs (absent)` — The step-2 SpiceDB evaluator file. DIRECTORY-010, -014 and -020 (competing ids on different branches) each create it in their R3. It exists on no branch, and the requirement that makes the seam ask SpiceDB is blocked on it by `git cat-file -e`.
- `docs/design/directory/briefs/DIRECTORY-002.json R4` — No SpiceDB permission check, relationship write or schema write in crates/lys/src/identity/ in step 1. That directory does not exist on main.
- `docs/design/identity/mockup/index.v5.html:1286-1302, 1538` — The scope control's keys ('all', 'org', 'team', 'mine'; labels All, Organisation, Teams, Mine) and the inScope rule, which is keyed on the secret's own scope ('user' for personal). Line 1299's sample mayRev also lets the owner revoke, which this brief deliberately does not adopt. Line 1538 has the 'revoked here; upstream pending' wording.
- `scripts/design/gate.sh (validate.py, check-coverage.py, render-cluster.py)` — The method's gate. It must exit 0 after the new brief and the re-render.
- `origin/draft/secrets/107557cf-3649-41b3-bd95-73296af454ff` — The earlier draft for this same card (aeec4d0, 27 Sep 17:48). It holds SECRETS-004 R1–R7, C30–C36, S19–S21 and ADR-068, and is the only place those ids are claimed apart from ADR-068.

### What was already decided

- ADR-001 — The door holds the handle and swaps it, checking SpiceDB. On SECRETS-003's branch only its placement moves, to crates/lys-secrets.
- ADR-002 — The token revolver is the first consumer. This brief does not change it.
- ADR-003 — Every grant is pegged to a person, and withdrawing a grant stops everything derived from it. That withdrawal ends leases as its own act, which this brief leaves alone.
- ADR-004 — Every project stands alone, so no Cambium file and no required engine.
- ADR-009 — People sign in through the Rauthy fork. The group claims on that token are the source of team ids.
- SECRETS-002 R6 — Personal, team and organisation scopes are enforced when listing, and a denial must not leak whether a record exists.
- SECRETS-002 R7 / SEC_REVOKE_STATES — Local use stops and upstream stays unconfirmed, no timer confirms it, and the transition happens once, on the provider's acknowledgement.
- SECRETS-002 R8 — The in-flight cancellation rule is open and stays outside this acceptance.
- SECRETS-003 (brief/secrets/309540a4) — The broker's host is crates/lys-secrets. The crate is a library with no HTTP layer, and its R3 owns secret.rs, store.rs, lease.rs and the tests/support fixture.
- DIRECTORY-002 R4 — SpiceDB enforces nothing in step 1, so no check is added in crates/lys/src/identity/.
- CONFORMANCE 7.5 / 7.6 / 7.8 / 3.4 — A revoke is two facts. Who may revoke is partly a test and partly open. The list has a scope filter. Emergency stop is a separate act.
- secrets CN1–CN4 — No credential value appears in any document, paths are relative to the repository root, and the standalone host must be resolved before dispatch.

### What was measured

- origin branches read: 234
- HEAD against origin/main: HEAD 7b53625 is 2 commits behind origin/main fa3dd53. Both commits are crates/lys CA-log work with no docs/design change.
- SECRETS brief ids on main: 2 (SECRETS-001, SECRETS-002)
- branches carrying SECRETS-003: 22
- branches carrying SECRETS-004: 1: origin/draft/secrets/107557cf, this card's own earlier draft. The next id unused anywhere is SECRETS-005.
- highest ADR id on main: ADR-018
- highest ADR id on any origin branch: ADR-075, so the next unused id is ADR-076
- branches claiming ADR-068: 2: origin/draft/secrets/107557cf (owner revoke) and origin/draft/directory/526867e5 (access graph)
- secrets checklist and story ids on main: C1–C18 and S1–S14
- highest checklist and story ids on any origin branch: C36 and S21, both only on this card's draft. SECRETS-003 claims C19–C29 and S15–S18.
- crates/lys-secrets/Cargo.toml: absent on origin/main and on all 234 origin branches
- crates/lys-identity-server/src/spicedb/check.rs: absent on every branch. It is planned as R3 of DIRECTORY-010, DIRECTORY-014 and DIRECTORY-020, three competing ids.
- crates/lys/src/identity/ on main: absent
- SECRETS-002 size: 333 lines of JSON and 9 requirements. R7 has 8 acceptance lines.
- distinct SEC_* acceptance ids in SECRETS-002: 15
- CONFORMANCE.md: 118 lines. Row 7.6 is at line 85 and row 7.8 at line 87.
- secrets cluster documents on main: DESIGN.md 80 lines, design.json 239, checklist.json 110, stories.json 109
- sh scripts/design/gate.sh on HEAD: exit 0; coverage clean
- earlier draft's size: 11 files changed, 793 insertions and 40 deletions. SECRETS-004.json is 262 lines with 7 requirements.
- mock-up scope control: 4 buttons (All, Organisation, Teams, Mine). inScope keys on s.scope, whose values are org, team and user.

### What it means for the other projects

- cambium — Nothing is edited. SECRETS-002's door paths in the Cambium checkout stay named-only, and nothing makes Cambium a required runtime (ADR-004).
- method — The gate uses the method's scripts copied at 3c3bac7 (scripts/design/SOURCE.md). The brief must validate and render under that copy, and the method itself is not changed.
- aion — The card goes through brief_card, sign-off, card_build_v3, src_pr and src_land. The earlier run for it ended push_refused on its draft branch, so this write must push under a fresh id.

### The decisions it stands on

- ADR-001 (honour) — Handles and the store are unchanged. Revoke and relinquish only end leases.
- ADR-002 (honour) — The token revolver is not touched.
- ADR-003 (honour) — Withdrawing an upstream grant keeps ending leases as its own act. The refusal covers only the direct revoke.
- ADR-004 (honour) — The code lives in crates/lys-secrets, with no Cambium file and no required engine.
- ADR-009 (honour) — Team ids come from the group claims on the Rauthy token.
-  (new) — A proposed ADR. Recommended: the person acted for revokes, and ownership alone confers no revoke. Rejected: the owner revokes every derived credential. No test or code grants it.

### What it requires

- A new brief under docs/design/secrets/briefs, with depends_on [SECRETS-002], an id unused on main and on every origin branch, and each row naming the SECRETS-002 requirement it extends.
- At least one acceptance line names CONFORMANCE 7.6 and at least one names 7.8.
- A fixture of person_a in team_a and person_b in team_b, each owning personal, team and organisation secrets, gives person_a exactly: mine = a_personal; team = a_team; organisation = a_org and b_org; no scope = those four.
- b_personal and b_team are absent from all four lists.
- A scope outside organisation, team and mine (e.g. all, user) is refused by name, and no secret is answered.
- A revoke by the person acted for stops issuing at once, and GET /leases/{lease_id} reads pending, then confirmed only after deliver_upstream_ack. No other public operation and no timer changes it, and the same holds on a relinquished lease.
- A revoke by the holder is refused with a refusal that names the lease and relinquish, and nothing is recorded.
- person_b, and any third person including the secret's owner, gets the 404 that names nothing for L1.
- A relinquish by the holder is recorded as a relinquish. GET /leases/{lease_id} reads ended by relinquish, issuing stopped, upstream pending and then confirmed, in that order.
- A second revoke or relinquish is refused as lease_already_ended, carrying the first end's way, instant and actor, and no second end record is written. Expiry reads as expired.
- The proposed ADR is in decisions.json and cited from design.json's decisions, and no code or test grants the owner a revoke.
- `rg -n 'pub fn team_ids\(' crates/lys-secrets/src` prints exactly 1 line, in teams.rs, and tests inject the claims.
- Every visibility and revoke check is a call on one seam. The requirement that makes it ask SpiceDB is blocked on `git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/check.rs`.
- The code rows are blocked on `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml`.
- A test destructures the redacting type without a rest pattern, and a test builds the store record with its scope and team fields.
- In docs/design/secrets, C2 and goal 2 say 'a requirement of a SECRETS brief', one sentence each.
- sh scripts/design/gate.sh exits 0 after the re-render.

### What must not change

- docs/design/secrets/briefs/SECRETS-002.json and SECRETS-002.md are not edited.
- DESIGN.md's Solution and Intention sentences about the door are not rewritten.
- No SpiceDB permission check, relationship write or schema write goes in crates/lys/src/identity/ (DIRECTORY-002 R4).
- No ADR id for the host decision goes in design_anchor.
- Withdrawal of an upstream grant (ADR-003, SEC_REVOKE_CHAIN) and the emergency stop (row 3.4) are unchanged.
- SECRETS-002 R8's in-flight rule stays open.
- Neither screen is built.
- No Cambium file changes and the token revolver does not change.
- lys-core and published wire formats do not change.
- No credential, token or key value appears in any document, test or fixture.

### What we must put in place first

- Read main and every origin branch at write time for unused brief, checklist, story and ADR ids. Today's reads give SECRETS-005, C37, S22 and ADR-076 if this card's own draft ids count as used.
- Nothing else. The code blockers (crates/lys-secrets on main, and the evaluator file) are recorded in the brief, not put in place by it.

### The risks

- Id collisions keep recurring: ADR-068 was taken by another draft between runs. A race with other drafts at push time could refuse the push again.
- SECRETS-003 has not landed and may change the files it creates (lease.rs, secret.rs, store.rs, tests/support). Rows that modify them would then be stale.
- SECRETS-003 R4 also defines revocation states, so revoke.rs's upstream state could duplicate or conflict with them.
- The step-2 SpiceDB brief's id is contested (DIRECTORY-010, -014, -020) and its evaluator lives in a server crate. lys-secrets depending on it may invert the crate layering.
- The words' done-when line 'a revoke by a third person is refused naming the lease' conflicts with the discovery ruling. A builder reading the words and not the rulings would test the wrong refusal.
- The mock-up's own sample lets the owner revoke (line 1299) and labels the control 'Teams'. A reviewer comparing against the mock-up may read the brief as non-conforming.
- The agent holder's route to POST relinquish is unspecified, because agents do not sign in.

### Still open

- The Solution sentence ('A broker in Rust inside the door…') and the Intention sentence ('…a short-lived handle the door swaps…') in docs/design/secrets/DESIGN.md contradict the host decision that SECRETS-003 carries. Do they stay as they are until SECRETS-003 lands its own documents? The sentence of the words it stands on: "DESIGN.md's Solution and Intention sentences that put the broker inside the door are quoted as a question for the lead and not rewritten; they change when SECRETS-003 lands with ADR-019, by that brief's documents.". Why only the lead can settle it: The words ask for these sentences to be put to the lead as a question. On main, docs/design/secrets/DESIGN.md places the broker in the door, while this brief's code rows all land in crates/lys-secrets. A reader of the design sees one host and the brief builds in another.
- When an agent calls GET /secrets, is it refused by name, or answered like any other caller? The sentence of the words it stands on: "Agents never use the secrets list, so an agent's team membership changes no list.". Why only the lead can settle it: The sentence settles that team membership does not change an agent's list. It leaves open whether an agent caller is refused or answered, and an agent would see a refusal in one case and a list in the other. SECRETS-003 lets agents reach the broker by a signed presentation, so an agent can reach the route.

### The units beyond the first

- Build the scoped secrets list and the revoke, relinquish and lease answers in crates/lys-secrets — The code rows wait on crates/lys-secrets/Cargo.toml reaching origin/main through SECRETS-003, and go through card_build_v3 after sign-off.
- Make the visibility and revoke seam ask SpiceDB through the step-2 evaluator — Blocked separately on crates/lys-identity-server/src/spicedb/check.rs existing on main.
- Rule the proposed ADR on owner revocation — Only the lead or Tom can decide it. If decided, it brings the owner's discovery of a lease in its own brief.
- The secrets list screen and the lease screen with its Revoke control — These are the neighbours held to the rule that the browser shows only what the server answered. surface/identity does not exist on main.
- Replace the group-claim team source with a directory team model — A directory brief is named to replace team_ids' source in its one function.
- Rewrite DESIGN.md's Solution and Intention to the crates/lys-secrets host — This changes by SECRETS-003's documents when it lands, not in this brief.

### The smallest complete shape

One documents-only commit on main's docs/design: the new SECRETS brief (JSON and rendered markdown), with rows for the proposed ADR, team_ids, the seam, the scoped list, revoke, relinquish and the SpiceDB switch. The same commit adds the proposed ADR to decisions.json, the new checklist items and stories, the widened C2 and goal 2 sentences, the design.json decisions and structure rows, the RM-002 link, and a re-render that makes gate.sh exit 0. It must be signed off on the card before anything is built from it.

## The roadmap row

- **RM-047** — Say who may revoke a lease and filter the secrets list by scope (CONFORMANCE 7.6 and 7.8) (feature, idea)
- Summary: SECRETS-004 amends SECRETS-002 for CONFORMANCE rows 7.6 and 7.8. The person acted for revokes a lease, stopping issuing at once with upstream pending until confirmed; the holder relinquishes, recorded as its own act with the same two facts; a revoke by the holder is refused and pointed at relinquish; every other caller, a secret's owner among them, gets the not-found refusal that names nothing; and a revoke or relinquish of an already ended lease is refused as already ended. The secrets list takes one scope from organisation, team and mine, applied at the server, with no scope meaning everything the caller may see, and refuses an agent by name. Whether a secret's owner may revoke every derived credential is proposed ADR-095, and nothing grants it. Every visibility and revoke check goes through one seam that answers from the record's fields until the step-2 SpiceDB evaluator lands.
- Asked by: tom on 2026-09-27T13:07:00+10:00
- Context: The words of the SECRETS-004 card on the Lys board, carrying the lead's rulings to run a6ce8a7a, which ended push_refused, and to brief run 107557cf. The lead's answers before this round are written into SECRETS-004: the secrets design's Intention and Solution sentences stand until SECRETS-003's own documents change them, with the conflict named as an open finding against SECRETS-003; and an agent that asks for the secrets list is refused by name.
- Quote: Write the brief for this card in the secrets cluster (docs/design/secrets/briefs, the next id not used on main or on any origin branch, which at 13:00 on 27 September was SECRETS-004, so take the id from the branches at write time), amending SECRETS-002 for two rows of docs/design/identity/CONFORMANCE.md, 7.6 in its test part and 7.8, and giving each row at least one acceptance line that names it. Row 7.6 says who may revoke a lease. The person acted for may revoke a lease they are acted for under at any time, and the revoke is the two facts SECRETS-002 R7 and R8 already state, issuing stops at once and the system behind confirms separately with pending shown until then. Anyone who is neither the person acted for nor the lease's holder is refused when they try. A caller who cannot discover the lease gets the not-found refusal SECRETS-002 R6 gives and nothing is named, and a caller who can see it gets a refusal that names the lease. The holder may give its own lease back, which is a relinquish, a separate act recorded as such that stops issuing at once. The refusal covers only the direct act of revoking a lease; withdrawal of an upstream grant under ADR-003 and SEC_REVOKE_CHAIN and emergency stop under row 3.4 end a lease as their own acts and are untouched. Whether the owner of a secret may revoke every credential derived from it is a policy choice not conferred by ownership alone, and this brief records it as a proposed ADR in decisions.json, recommendation that the person acted for revokes and ownership alone confers no revoke, rejected alternative that the owner revokes every derived credential, cited from the Decisions section by id, with no test and no code that grants it, so that until it is ruled the set of who may revoke beyond the person acted for is empty. Row 7.8 is the scope filter on the secrets list. The list takes one scope from a closed set of three, organisation, team and mine, the server applies the scope and answers only the secrets the caller may see within it, and the browser shows what came back and nothing else. Mine is the secrets whose scope is personal and whose owner is the caller. Team is the secrets whose scope is team and whose team is one the caller belongs to. Organisation is the secrets whose scope is organisation. Each value is keyed on the secret's own scope as the mock-up and SECRETS-002 R6 have it, and the list with no scope chosen is everything the caller may see, which is the mock-up's All and not a fourth value. A scope outside the set is refused by name. Done when a fixture of two people in two teams with secrets owned by each gives, for one caller, three lists whose members are exactly those the scope rule names, when a secret the caller may not see is absent from all three, when a revoke by the person acted for stops issuing at once and shows pending until confirmed, and when a revoke by a third person is refused naming the lease. Hold to ADR-001 to ADR-004, to R4 of DIRECTORY-002 and to ADR-009, so that every visibility and revoke check asks SpiceDB. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after Tom or the lead signs it off on the card. Rulings of the lead, Apollo, given on 27 September 2026 to run a6ce8a7a-7807-42bf-a0bb-9e74bb99672d in answer to its rounds 1 and 2, read by Archie as second reader. That run took the answers and then ended push_refused on its draft branch. They are settled here and the author reopens none of them. The two facts of a revoke are SEC_REVOKE_STATES in SECRETS-002 R7, and R8's in-flight rule stays open and outside this acceptance. The server's answer is this brief's whole of row 7.6, GET /leases/{lease_id} reading issuing stopped and upstream pending until confirmed; the lease screen with its Revoke control is outside this brief and named as the neighbour. Team ids for the signed-in person come from the identity provider's group claims on their token, one group per team, read through one function named in the brief, until a directory brief models teams and replaces that source in that one function; tests inject the claims. The host is crates/lys-secrets following ADR-019, and the requirement that touches it is blocked on crates/lys-secrets/Cargo.toml being present on origin/main by git cat-file -e. DESIGN.md's Solution and Intention sentences that put the broker inside the door are quoted as a question for the lead and not rewritten; they change when SECRETS-003 lands with ADR-019, by that brief's documents. Holding to R4 of DIRECTORY-002 means the narrow thing: no SpiceDB permission check is added in the step-1 paths R4 forbids, the checks are calls on the one evaluator seam the step-2 SpiceDB brief introduces, answering from the record's scope, team and owner fields until that brief lands, and the requirement that makes the seam ask SpiceDB is blocked on that brief's evaluator file by git cat-file -e, never on commit text. This brief is a new brief with depends_on SECRETS-002 whose rows name the SECRETS-002 requirements they extend; CHECKLIST.md C2 and DESIGN.md goal 2 widen from covered by a SECRETS-002 requirement to covered by a requirement of a SECRETS brief, one sentence each. Agents never use the secrets list, so an agent's team membership changes no list.
- Cluster: secrets; briefs: SECRETS-004
- Notes: Further units, not written: Build the scoped secrets list and the revoke, relinquish and lease answers in crates/lys-secrets; Make the visibility and revoke seam ask SpiceDB through the step-2 evaluator; Rule the proposed ADR on owner revocation; The secrets list screen and the lease screen with its Revoke control; Replace the group-claim team source with a directory team model; Rewrite DESIGN.md's Solution and Intention to the crates/lys-secrets host. Ids read at write time against main and every origin branch: SECRETS-004, C30 to C38 and S19 to S21 are claimed only by this card's own earlier drafts and kept, other secrets drafts taking C39 to C49 and S22 to S27; ADR-095 is claimed by no other branch and kept, read from decisions.json on origin/main (a426b9a, ending at ADR-018) and on every one of the 273 heads `git ls-remote --heads origin` listed after a fetch: ADR-089 is claimed by origin/brief/directory/fb954264 and origin/draft/directory/fb954264, ADR-090 is named by the directory drafts, ADR-091 by origin/brief/directory/8122eb55, origin/draft/directory/8122eb55 and origin/draft/secrets/2d8c28b9, ADR-092 by origin/draft/directory/647a3946, ADR-093 by origin/brief/directory/8a60bd7a and origin/brief/home/8d27e91f, ADR-094 by origin/brief/directory/8a60bd7a, ADR-096 by origin/draft/directory/73184e72, origin/draft/home/d948c6a9 and origin/draft/secrets/0e6892c4, and ADR-097, the highest on any branch, by origin/draft/directory/844a587d and origin/draft/lys-log-store/6826e0cf, so the directory-claimed ids include ADR-089, ADR-090 and ADR-091 beside ADR-068, ADR-076 and ADR-085 to ADR-088; RM-047 is claimed only by this card's own earlier draft and kept.

## The design

---
type: design
cluster: secrets
title: Secrets broker: agents hold handles, never credentials
---

# Secrets broker: agents hold handles, never credentials

> **Cluster:** secrets

## Intention

Every identity, person or agent, uses credentials through a short-lived handle the door swaps for the real credential, so no credential ever reaches an agent and revoking is instant.

## Problem

Credentials live in files on each machine and in each worker's environment. An agent that holds a key cannot have it taken back, usage cannot be attributed in one place, and resting an account means copying a file to every machine.

## Solution

A broker in Rust inside the door: an encrypted store of real credentials, handles bound to identities, a proxy that checks SpiceDB, swaps the handle, forwards the call and writes one audit line, rotation across several accounts under one handle, OAuth refresh at the proxy, sealed records tagged in SpiceDB, and leases counted by uses, time window and spend. The token revolver is its first consumer.

## Principles

- **P1** — A handle, never a credential: the credential never leaves the server.
- **P2** — Revoking is dropping the handle; what was already handed to a process needs its own revocation story, stated per case.
- **P3** — Every use writes one audit line naming the seat, the handle, the real account and the time.
- **P4** — A key is used through the proxy, never read; only memories are read, in the smallest piece asked for.
- **P5** — Every grant traces back to the person who authorised it.
- **P6** — Permission to use never implies permission to lend. Pass-on rights are affirmative and recipient-specific; a missing prohibition is not a grant. Server-verified ownership is the affirmative may-lend route in docs/design/identity/CONFORMANCE.md at commit 1353c22 row 7.3; a display label is not proof of ownership.
- **P7** — Secret visibility, usage, delegation and revocation follow the same current human-rooted authority at every server seam, independent of how the caller reaches it.

## Decisions

- ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
- ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-095 — Ownership of a secret alone confers no revoke over the leases derived from it — The person acted for under a lease revokes it, and ownership of the secret the lease was issued from confers no revoke, over the owner revoking every credential derived from the secret, because who may revoke is a policy choice that ownership alone does not confer. Rejected: the owner revokes every credential derived from the secret.

## Goals

- An implementation brief, SECRETS-002, covers every part of the temporary key model in the statement with numbered requirements and acceptance criteria.
- Every checklist item and user story of the broker is covered by a requirement of a SECRETS brief.
- The token revolver can take its next account from the broker instead of its own list.

## Non-Goals

- OpenBao or another external secrets engine — Tom's model is built in Rust inside the door; an external engine is only reconsidered if credentials minted on demand are needed.
- The seat login through a proxy handle and base URL — Not tested with a subscription login; the statement keeps it open to prove later and nothing depends on it.
- The delegation schema — The statement says it is not settled.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/secrets/briefs/SECRETS-002.json` | the secrets broker implementation brief | SECRETS-001 |
| `docs/design/secrets/briefs/SECRETS-002.md` | its rendered markdown | SECRETS-001 |
| `docs/design/secrets/design.json` | this design; the structure gains a row for every path SECRETS-002 names | SECRETS-001 |
| `docs/design/secrets/DESIGN.md` | rendered design | SECRETS-001 |
| `docs/design/secrets/checklist.json` | checklist; gains the broker implementation items | SECRETS-001 |
| `docs/design/secrets/CHECKLIST.md` | rendered checklist | SECRETS-001 |
| `docs/design/secrets/stories.json` | stories; gains the broker implementation stories | SECRETS-001 |
| `docs/design/secrets/USER-STORIES.md` | rendered stories | SECRETS-001 |
| `crates/lys-core/src/delegation/mod.rs` | lys/delegation/v1; its docs record the handle and the lease's time window as applications of the delegation format (SECRETS-002 R1, R9) | SECRETS-002 |
| `docs/design/WIRE-FORMATS.md` | the lys wire-format register; records the handle, sealed-record and lease formats before any is signed (SECRETS-002 R1, R6, R9) | SECRETS-002 |
| `crates/lys-core/src/seal/mod.rs` | sealed envelopes; its docs record the sealed record as an application of lys/sealed-envelope/v1 (SECRETS-002 R6) | SECRETS-002 |
| `docs/design/secrets/briefs/SECRETS-004.json` | the brief amending SECRETS-002 for CONFORMANCE rows 7.6 and 7.8: who may revoke a lease, the relinquish, and the scoped secrets list | SECRETS-004 |
| `docs/design/secrets/briefs/SECRETS-004.md` | its rendered markdown | SECRETS-004 |
| `docs/design/decisions.json` | the project decision ledger; gains proposed ADR-095, ownership alone confers no revoke (SECRETS-004 R1) | SECRETS-004 |
| `Cargo.lock` | the workspace lockfile; gains the HTTP routing dependencies of crates/lys-secrets (SECRETS-004 R4) and the lys-secrets dependency of crates/lys-identity-server (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-secrets/Cargo.toml` | the broker crate's manifest, created by SECRETS-003; gains the HTTP routing dependencies (SECRETS-004 R4) | SECRETS-003 |
| `crates/lys-secrets/src/lib.rs` | the broker crate's module declarations, created by SECRETS-003; declares the SECRETS-004 modules (SECRETS-004 R3 to R7) and re-exports the lease record as lys_secrets::Lease (SECRETS-004 R5) | SECRETS-003 |
| `crates/lys-secrets/src/store.rs` | the encrypted store, created by SECRETS-003; each secret record, the entry that carries its owning identity, gains its scope and its team (SECRETS-004 R3) | SECRETS-003 |
| `crates/lys-secrets/src/lease.rs` | the lease record Lease, created by SECRETS-003; each lease carries its holder, its person acted for, its source secret and its end (SECRETS-004 R3), and its upstream field, holding SECRETS-003 R4's revocation state and not public outside the crate (SECRETS-004 R5) | SECRETS-003 |
| `crates/lys-secrets/src/secret.rs` | the redacting type for credential bytes, created by SECRETS-003; its fields unchanged, it gains only the declaration of its test module (SECRETS-004 R3) | SECRETS-003 |
| `crates/lys-secrets/schema/secrets.zed` | the SpiceDB relations, created by SECRETS-003; gains the relations the seam asks (SECRETS-004 R7) | SECRETS-003 |
| `crates/lys-secrets/tests/support/mod.rs` | shared test doubles, created by SECRETS-003; gains the two-people, two-teams fixture and the injected group claims (SECRETS-004 R4 to R7) | SECRETS-003 |
| `crates/lys-secrets/src/teams.rs` | team_ids: the signed-in person's team ids from the group claims on their token, the one place they are read (SECRETS-004 R2) | SECRETS-004 |
| `crates/lys-secrets/src/teams_tests.rs` | team_ids tests with injected claims (SECRETS-004 R2) | SECRETS-004 |
| `crates/lys-secrets/src/access.rs` | the one seam every secret visibility, lease discovery and lease revoke check calls; answers from the record's fields, then through the PermissionCheck it is given (SECRETS-004 R3, R7) | SECRETS-004 |
| `crates/lys-secrets/src/access_tests.rs` | seam tests answering from the record's fields (SECRETS-004 R3) | SECRETS-004 |
| `crates/lys-secrets/src/permission.rs` | PermissionCheck, the trait the seam answers through once SpiceDB is asked; declared in lys-secrets so it depends on no server crate (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-identity-server/src/spicedb/check.rs` | the step-2 SpiceDB evaluator, created by the step-2 SpiceDB brief whose id is not settled; implements PermissionCheck and is passed in where the routes are built (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-identity-server/Cargo.toml` | the identity server's manifest, landed by the directory briefs; gains the dependency on lys-secrets, so the server depends on the library and never the reverse (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-identity-server/src/routes.rs` | where the identity server builds its routes, landed by the directory briefs; passes the PermissionCheck implementation to the secrets routes (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-secrets/src/secret_tests.rs` | the test that destructures the redacting type with no rest pattern, so it compiles only while its fields are SECRETS-003's (SECRETS-004 R3) | SECRETS-004 |
| `crates/lys-secrets/src/secret_list.rs` | the scoped secrets list: organisation, team, mine, or no scope (SECRETS-004 R4) | SECRETS-004 |
| `crates/lys-secrets/src/api/mod.rs` | the broker's HTTP routes, module declarations only (SECRETS-004 R4) | SECRETS-004 |
| `crates/lys-secrets/src/api/secrets.rs` | GET /secrets with its scope parameter, and its refusal to an agent (SECRETS-004 R4) | SECRETS-004 |
| `crates/lys-secrets/src/api/leases.rs` | GET /leases/{lease_id}, POST /leases/{lease_id}/revoke and POST /leases/{lease_id}/relinquish (SECRETS-004 R5, R6) | SECRETS-004 |
| `crates/lys-secrets/src/revoke.rs` | the revoke by the person acted for, its refusals and the refusal of an already ended lease, end_with_upstream_pending, the one function that ends a lease with upstream pending for revoke and relinquish, and deliver_upstream_ack, the one writer of confirmed, both moving the lease through SECRETS-003 R4's revocation states with no second state of their own (SECRETS-004 R5, R6) | SECRETS-004 |
| `crates/lys-secrets/src/relinquish.rs` | the holder's relinquish, recorded as its own act (SECRETS-004 R6) | SECRETS-004 |
| `crates/lys-secrets/tests/secret_list.rs` | the CONFORMANCE 7.8 legs (SECRETS-004 R4) | SECRETS-004 |
| `crates/lys-secrets/tests/lease_revoke.rs` | the CONFORMANCE 7.6 revoke legs (SECRETS-004 R5) and the revoke leg against an always-no PermissionCheck double (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-secrets/tests/lease_relinquish.rs` | the CONFORMANCE 7.6 relinquish legs (SECRETS-004 R6) | SECRETS-004 |

## Inventory

- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority: 'Secrets: the temporary key model' through 'What revoking reaches', 'Two rules from Tom, 13:55', 'Lifecycle, budgets and leases' and 'Where it lives: lys'
- `docs/design/decisions.json` — the project decision ledger this cluster anchors to
- `crates/` — the five lys crates (lys, lys-core, lys-anchor, lys-anchor-cli, lys-log-store): sealed envelopes, the delegation format, the log. The door repository, where the statement places the store and the proxy, is the cambium checkout, apps/cambium in the ablative estate on this Mac; it is read, never written, by this brief
- `docs/design/identity/CONFORMANCE.md` — Committed behaviour source: docs/design/identity/CONFORMANCE.md at commit 1353c22. Bind the SEC_* acceptance IDs to its rows before dispatch; mock-up sample data is not enforcement evidence.

## Constraints

- **CN1** — No credential, token or key value is ever written, read or quoted in any document of this cluster.
- **CN2** — No row of SECRETS-002 is dispatched before Waffles has reviewed it.
- **CN3** — Every path written in a document of this cluster is relative to the repository root, whatever directory a session starts in; a command runs from its own tree and spells its paths from there.
- **CN4** — Resolve standalone broker-host ownership before dispatch. Earlier SECRETS-002 Cambium path proposals and empty door-owned file walls are not authority to make Cambium a required runtime service; ADR-004 remains binding.


---
type: brief
id: SECRETS-001
cluster: secrets
title: Write the secrets broker implementation brief
---

# SECRETS-001: Write the secrets broker implementation brief

> **Cluster:** secrets
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> **Checklist:**
> - C1 — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend.
> - C2 — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a requirement of a SECRETS brief.
> - C3 — The rendered markdown of this cluster matches its JSON.
> **Stories:**
> - S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time.

## Purpose

Agents get a short-lived, limited handle to a credential, never the credential itself, and the token revolver is the first user. This brief produces the implementation brief for that broker, so its rows can be reviewed and built one at a time. Documents only.

## Task

Read docs/design/identity/STATEMENT-2026-09-22.md, the sections 'Secrets: the temporary key model' through 'What revoking reaches', 'Two rules from Tom, 13:55', 'Lifecycle, budgets and leases: the discussion of 14:00' and 'Where it lives: lys', and the decisions ADR-001 to ADR-004 in docs/design/decisions.json. Read the source the design inventory names before assigning any file: the lys crates and the door repository. Write SECRETS-002 in the design-system brief form, add the broker's checklist items and user stories to this cluster, and add a structure row to design.json for every path SECRETS-002 names. Every path written in a document of this cluster is relative to the repository root, even though the session starts in docs/.

## Requirements

### R1: Author the implementation brief SECRETS-002

THE SYSTEM SHALL have a brief docs/design/secrets/briefs/SECRETS-002.json, valid against the design-system brief schema, with one requirement per part of the temporary key model named in C1. Each requirement SHALL state its behaviour in the statement's own terms, list acceptance criteria that can be checked against code and tests, and name the files it creates, modifies or deletes. The repository that owns each file SHALL be grounded in the statement and in the source the inventory names, cited by path and line, and never assumed: the statement places the store and proxy in the door ('Secrets: the temporary key model') and the handle and sealed-record formats in lys ('Where it lives: lys': lys sealed envelopes and the delegation format, applied). A file owned by lys is listed in the requirement's files and gains a structure row. A file owned by the door repository is named in the requirement's spec with its owner and citation and is NOT listed in files and gains NO structure row, because no root naming the door repository is set for this round; it moves into files when that root is set. A requirement whose owner the sources do not settle is recorded as open. Where the statement leaves a point open (the delegation schema; whether revoking a login token at the provider fails the seat's next call; the proxy-handle login path), SECRETS-002 SHALL record it as open and SHALL NOT settle it. design.json SHALL gain a structure row, with brief SECRETS-002, for every path SECRETS-002 names.

**Acceptance:**
- docs/design/secrets/briefs/SECRETS-002.json exists and validate.py reports it valid.
- SECRETS-002 has a requirement for each of the eight parts named in C1, each with at least one acceptance criterion and at least one file path.
- Every file SECRETS-002 names carries its owning repository with a citation to the statement or the source; lys-owned files are in files and structure, door-owned files are in the spec text only, and no structure row or files entry in this cluster carries a root token.
- The cancellation rule for calls already admitted by the proxy is a requirement of its own with its own acceptance criteria.
- Each point the statement leaves open appears in SECRETS-002 as open and is not decided there.
- check-coverage.py reports every SECRETS-002 path present in design.json structure.
- No file in the cluster contains a credential, token or key value.

**Files:**
- create: docs/design/secrets/briefs/SECRETS-002.json
- create: docs/design/secrets/briefs/SECRETS-002.md
- modify: docs/design/secrets/design.json
- modify: docs/design/secrets/DESIGN.md

**Checklist:**
- C1 — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend.

**Stories:**
- S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 met: SECRETS-002.json exists, and validate.py prints 'design/secrets/briefs/SECRETS-002.json: OK [brief.schema.json]'. Row 2 met: C1's eight parts map to R1 to R9, with the revocation part split into R7 (the three cases) and R8 (in-flight cancellation). Each requirement has 3 to 7 acceptance criteria. R1, R6 and R9 list lys files in files. R2, R3, R4, R5, R7 and R8 name only door paths (for example crates/cambium-door/src/http/secrets_proxy.rs and crates/cambium-store/src/traits/secrets.rs), in their spec text, as R1's own rule requires. Row 3 met: every path has an owner and a citation to the statement by line (for example :25, :27, :82) or to the source. Lys paths are in files and structure. Door paths are named in the spec with the door repository (cambium, read at cbcd2cc9d) and cited against crates/cambium-store/src/traits/mod.rs, crates/cambium-door/src/http/router.rs and crates/cambium-door/src/http/agent_seat.rs:13-15. No files entry or structure row carries a root token. Row 4 met: the cancellation rule is R8, with three acceptance criteria of its own. Row 5 met: the delegation schema is recorded OPEN in R1 and R9. Provider revocation of a login token is OPEN in R5 and R7. The proxy-handle login path is OPEN in R5. Points the source raised are also recorded open: the seat/speaks-for role has no consumer (crates/lys-core/src/delegation/artifact.rs:242-254); v1 has no expiry (crates/lys-core/src/delegation/mod.rs:218-221); sealed-envelope/v1 seals with an empty AAD (crates/lys-core/src/seal/sealed_envelope.rs:180-183, docs/design/WIRE-FORMATS.md:18); the audit line's signer (statement :80 against agent_seat.rs:13-15) and its schema (:187); the file for the revocation fold; the revolver's worker-side owner; and the cancellation rule itself until its review. Row 6 met: check-coverage.py exits 0 with no structure failure. Row 7 met: the cluster holds no credential values, and a grep for common token shapes found nothing.
- Deviation: (1) Six requirements (R2, R3, R4, R5, R7, R8) have empty files arrays. They touch only door-owned files, which R1's spec says must be named in the spec text and not in files until a door root is set. Their 'at least one file path' is the door path in their spec. (2) validate.py exits 1 for the cluster because of design.json's existing `gate` field, which is not mine to change. Every document of this round validates OK. (3) The worker side of the token revolver lives in manifold (crates/manifold-node/src/seat/launcher.rs:31-40 at 3df5ac5f64). The design inventory names no engine repository, so I read it only to show that its owner is open.
- Files changed:
  - created: `docs/design/secrets/briefs/SECRETS-002.json` — The implementation brief: nine requirements R1 to R9 in EARS form. Each carries acceptance criteria, owners with citations and open points. Blocked by Waffles' review, SpiceDB beside the door, the door root not being set, the delegation schema, and the lys-core release that freezes lys/delegation/v1.
  - created: `docs/design/secrets/briefs/SECRETS-002.md` — The brief rendered from its JSON by render-cluster.py.
  - modified: `docs/design/secrets/design.json` — Gains three structure rows with brief SECRETS-002, one per lys path the brief lists in files: crates/lys-core/src/delegation/mod.rs, docs/design/WIRE-FORMATS.md, crates/lys-core/src/seal/mod.rs. The existing gate field is untouched.
  - modified: `docs/design/secrets/DESIGN.md` — Re-rendered with the three new structure rows.
- Checklist delivery:
  - [x] C1 — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend. — R1 to R9 cover the eight parts. The cancellation rule is R8, on its own.
- Story delivery:
  - [x] S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time. — Nine numbered requirements with acceptance criteria and blockers, so a reviewer can take them one row at a time; CN2 is repeated in blocked_by and boundaries.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] docs/design/secrets/briefs/SECRETS-002.json exists and validate.py reports it valid. — With DS2_METHOD=/Users/tom/Developer/projects/deno_rust/meridian/.meridian/design-system-v2, validate.py design/secrets prints 'design/secrets/briefs/SECRETS-002.json: OK [brief.schema.json]' and 'All 5 document(s) valid.' It exits 0, before and after the harden edits.
  - [x] SECRETS-002 has a requirement for each of the eight parts named in C1, each with at least one acceptance criterion and at least one file path. — The parts map to R1 to R9: handle and proxy (R1), rotation (R2), revolver (R3), OAuth (R4), spawn login (R5), sealed records (R6), revocation (R7, plus R8 for in-flight calls) and leases (R9). Each has 3 to 7 acceptance criteria. R1, R6 and R9 list lys paths in files. R2, R3, R4, R5, R7 and R8 name door paths in their spec text, as SECRETS-001 R1's spec requires, for example crates/cambium-door/src/http/secrets_rotation.rs in R2 and secrets_in_flight.rs in R8.
  - [x] Every file SECRETS-002 names carries its owning repository with a citation to the statement or the source; lys-owned files are in files and structure, door-owned files are in the spec text only, and no structure row or files entry in this cluster carries a root token. — The lys paths delegation/mod.rs, seal/mod.rs and WIRE-FORMATS.md are in files and in design.json structure. Door paths appear only in the spec text, each under the prefix 'Door-owned, named here only (the door repository ... read at cbcd2cc9d)'. I checked cbcd2cc9d in apps/cambium: agent_seat.rs:13-15 says the private key is never persisted, and the store traits' mod.rs has no secrets module. A script check found no files entry beginning with /, $, < or ~ and none containing '..'. The design.json structure paths are all relative to the repository root.
  - [x] The cancellation rule for calls already admitted by the proxy is a requirement of its own with its own acceptance criteria. — SECRETS-002 R8, 'State and enforce the cancellation rule for calls already admitted', has 3 acceptance criteria of its own and cites the statement at :55.
  - [x] Each point the statement leaves open appears in SECRETS-002 as open and is not decided there. — The delegation schema (:21) is OPEN in R1(a) and R9. Provider revocation of a login token (:56) is OPEN in R5(b) and R7(a). The proxy-handle login path (:45) is OPEN in R5(a). The audit event schema (:187) is OPEN in R1(e). After the harden fix, R7 no longer chooses how an engine learns of a revocation.
  - [x] check-coverage.py reports every SECRETS-002 path present in design.json structure. — check-coverage.py design/secrets reports 'Coverage clean: all items covered, briefs consistent.' and exits 0. The 3 lys paths in SECRETS-002's files are structure rows with brief SECRETS-002.
  - [x] No file in the cluster contains a credential, token or key value. — A grep -rE across docs/design/secrets/ found no matches (exit 1). It looked for sk-, ghp_, eyJ, AKIA, BEGIN key blocks, xox tokens and hex runs of 40 or more characters.
- Checklist verified: C1
- Stories verified: S1
- Issues:
  - R8 cited statement :156 for Chippy's release 2, 'demonstrate revocation and recovery after a crash'. That quotation is on line 155; line 156 is release 3.
  - R9 acceptance fixed its one-use race test at 'over 100 repetitions of the race'. That count traces to nothing in the statement, the design or the brief, and a race that is not forced may never fire.
  - R7 acceptance 2 wrote '(the test double engine receives the end request)'. That decides that the door sends the engine an end request. The statement (:56) says only that the engine that runs the seat ends it on its own.
- Fixes:
  - Changed R8's citation to docs/design/identity/STATEMENT-2026-09-22.md:155.
  - Removed the repetition count of 100 from R9's race criterion. Both requests are now held at the use check until both have arrived, so the race fires on every run.
  - Removed the end-request parenthetical from R7 acceptance 2. The criterion now asserts only that the door names the seat and ends no process.
  - Re-rendered SECRETS-002.md with render-cluster.py; a second run was byte-identical.

### R2: Record the broker's checklist items and stories and cover them

THE SYSTEM SHALL add to checklist.json a section of the broker's implementation items and to stories.json the personas and stories of the people and agents who use the broker (a person granting an agent access, an agent using a handle, an operator resting an account, a reviewer reading the audit). Every item and story added SHALL be named by at least one SECRETS-002 requirement, and the rendered markdown SHALL match the JSON.

**Acceptance:**
- check-coverage.py on the cluster exits 0: no item or story is unassigned and no brief names an unknown id.
- render-cluster.py on the cluster leaves the rendered markdown unchanged after the commit.
- The stories include the token revolver asking for its next account.

**Files:**
- modify: docs/design/secrets/checklist.json
- modify: docs/design/secrets/CHECKLIST.md
- modify: docs/design/secrets/stories.json
- modify: docs/design/secrets/USER-STORIES.md

**Checklist:**
- C2 — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a requirement of a SECRETS brief.
- C3 — The rendered markdown of this cluster matches its JSON.

**Stories:**
- S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 met: check-coverage.py exits 0. It reports 'Coverage clean: all items covered, briefs consistent': 14 items and 11 stories, with C4 to C14 and S2 to S11 each named by at least one SECRETS-002 requirement and no unknown ids. Row 2 met: render-cluster.py exits 0, and a second run leaves the rendered markdown byte-identical (same checksum before and after), so the markdown matches the JSON. Row 3 met: S6, of the Token revolver persona, reads 'when a session prints its usage-limit words, I want to ask the broker for my next account instead of walking my own list', and R3 carries it.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/secrets/checklist.json` — Adds the section 'The broker's implementation' with items C4 to C14: one per part, plus C13 (open points stay open) and C14 (no engine dependency).
  - modified: `docs/design/secrets/CHECKLIST.md` — Re-rendered from checklist.json.
  - modified: `docs/design/secrets/stories.json` — Adds personas Person, AI Agent, Token revolver, Engine, Operator and a Reviewer who reads the audit, with stories S2 to S11.
  - modified: `docs/design/secrets/USER-STORIES.md` — Re-rendered from stories.json.
- Checklist delivery:
  - [x] C2 — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a requirement of a SECRETS brief. — C4 to C14 and S2 to S11 are recorded and each is covered by a SECRETS-002 requirement; coverage exits 0.
  - [x] C3 — The rendered markdown of this cluster matches its JSON. — Re-rendering changes nothing.
- Story delivery:
  - [x] S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time. — The rendered SECRETS-002.md shows every requirement next to the item and story ids it covers.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] check-coverage.py on the cluster exits 0: no item or story is unassigned and no brief names an unknown id. — check-coverage.py with the ledger's DS2_METHOD reports 14 checklist items, 11 user stories, 2 briefs and 'Coverage clean', and exits 0. C4 to C14 and S2 to S11 are each named by a SECRETS-002 requirement.
  - [x] render-cluster.py on the cluster leaves the rendered markdown unchanged after the commit. — render-cluster.py exits 0. The md5 of every .md file was identical before and after a second run, both before and after the harden edits. git status showed no further change.
  - [x] The stories include the token revolver asking for its next account. — stories.json has the persona 'Token revolver' with S6: 'I want to ask the broker for my next account instead of walking my own list'. SECRETS-002 R3 names S6.
- Checklist verified: C2, C3
- Stories verified: S1

## Boundaries

- Change no code: only paths under docs/design/secrets/ are created or modified.
- A credential, token or key never passes through an agent: no credential value is written, read or quoted.
- The statement is the authority: nothing it leaves open is decided here.
- Manifold is never a structural part: the engine that starts or ends a seat is whichever one runs the agent.
- No row of SECRETS-002 is dispatched until Waffles has reviewed it.

## Verification

- From docs/: python3 $DS2_METHOD/scripts/validate.py design/secrets exits 0.
- From docs/: python3 $DS2_METHOD/scripts/check-coverage.py design/secrets exits 0.
- From docs/: python3 $DS2_METHOD/scripts/render-cluster.py design/secrets exits 0 and git status shows no further change.


---
type: brief
id: SECRETS-002
cluster: secrets
title: Build the secrets broker: handles, the proxy, rotation, sealed records, revocation and leases
---

# SECRETS-002: Build the secrets broker: handles, the proxy, rotation, sealed records, revocation and leases

> **Cluster:** secrets
> **Blocked by:** Waffles' review of each row before it is dispatched (CN2), The live permission decision, SpiceDB beside the door, which the statement records as not started (docs/design/identity/STATEMENT-2026-09-22.md:127) and places in step 2 of the road (docs/design/identity/STATEMENT-2026-09-22.md:143); every proxy check asks it (docs/design/identity/STATEMENT-2026-09-22.md:17), A root naming the door repository, which is not set for this round; every door-owned file moves into files when it is, The delegation schema, which the statement leaves unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21); the handle (R1) and the lease's time window (R9) wait on it, The lys-core release that freezes lys/delegation/v1, which waits until the fold that enforces its ordering rule exists (CLAUDE.md:23); until then no delegation is signed outside tests (crates/lys-core/src/delegation/mod.rs:237-242), Standalone broker-host ownership and complete per-row implementation/test file walls: the prior draft maps the door to Cambium, while ADR-004 and IDENTITY-001 rows 04/05 require standalone identity operation. This discrepancy must be resolved by a reviewed ownership decision, not by making Cambium a mandatory server. No empty-wall row is ready for dispatch., Review of the proposed R7 freshness/refusal mechanism and the pending lifecycle-state policy in CONFORMANCE rows 3.2 and 3.4; accepted suspension/reinstatement behaviour in row 3.3 remains binding.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> **Checklist:**
> - C4 — A seat's call with its handle goes through the door's proxy, which checks SpiceDB, swaps the handle for the real credential, forwards the call and writes one audit line naming the seat, the handle, the real account and the time; the credential never leaves the server.
> - C5 — The store keeps a set of real accounts behind one handle, the proxy takes the next in turn and logs which one served, and resting an account is a store change with no file copied to any machine.
> - C6 — The token revolver asks the broker for its next account instead of walking its own list.
> - C7 — The proxy refreshes an OAuth access token itself from the refresh token in the store, and the seat never sees the refresh token.
> - C8 — The door puts the seat's own login into its environment at spawn, read from the store, rotating at spawn, and records which token went to which seat.
> - C9 — Sealed records are kept encrypted in the store, tagged in SpiceDB with which identities may read them, read by name with a relation check and one audit line, and a key is used through the proxy, never read.
> - C10 — Each of the three revocation cases has its own behaviour: a dropped handle refuses every new call, the login token's seat is ended by its engine, and read sealed knowledge is disclosed and recorded, never claimed back.
> - C11 — The proxy has an explicit cancellation rule for calls already admitted when their handle is dropped, written down before it is built.
> - C12 — Everything handed out is a lease counted by uses, a time window and a spend cap: the window in the signed delegation, uses and spend counted by the door, a one-use grant never spent twice.
> - C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
> - C14 — No broker row depends on any one engine: an engine without the broker reads its own pool file as it does today.
> - C15 — Real secret ownership or a valid human-rooted delegation permitting re-lending establishes affirmative may-lend; mere use and display labels do not. Applicable recipient policy is checked separately, including people-only refusal.
> - C16 — Personal, team and organisation secret boundaries are enforced in listing, metadata, read, use and lending; knowing another identity's record ID grants nothing.
> - C17 — Every derived handle stays inside its live ancestry, including shared use/spend budgets and expiry; revoking its source does not revoke an independently authorised sibling.
> - C18 — Issuance, account selection, retry and revocation preserve exact provenance and current authority; the screen distinguishes local refusal from unconfirmed provider action.
> **Stories:**
> - S2 (Person, Grants an agent provisioned under them access to an account) — As a person granting an agent access, I want to give it a handle under my own grant, limited by uses, time and spend, so that it can use the account without ever holding the credential and the grant traces back to me.
> - S3 (Person, Grants an agent provisioned under them access to an account) — As a person, I want to drop an agent's handle and have every new call on it refused at once, so that taking access back does not wait on rotating the real key.
> - S4 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to make my call with my handle and have the proxy swap in the credential, refreshing an expired OAuth token itself, so that I can do my work without ever seeing a credential.
> - S5 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to ask for one of my sealed memories by name and get back only the piece I am permitted, so that a secret never sits in plain text in my memory files.
> - S6 (Token revolver, The worker that runs Claude sessions and builders and turns to the next account on usage-limit words) — As the token revolver, when a session prints its usage-limit words, I want to ask the broker for my next account instead of walking my own list, so that the spread across accounts is the broker's and no pool file is copied to my machine.
> - S7 (Engine, Whichever runtime starts and ends a seat) — As the engine starting a seat, I want the door to give me the seat's own login from the store at spawn and record which token went to which seat, so that I read no pool file when the broker is there and still work from my own when it is not.
> - S8 (Operator, Keeps the accounts and revokes access) — As an operator, I want to rest an account by one change in the store, so that no worker gives it another call and no file is copied to any machine.
> - S9 (Operator, Keeps the accounts and revokes access) — As an operator revoking access, I want each case to say what it reaches: a handle refuses new calls, a call already admitted follows the stated cancellation rule, a login token's seat is ended by its engine, and read knowledge stays read, so that I am never told revocation took back what it cannot.
> - S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.
> - S11 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want each call that was in flight when its handle was dropped to show the outcome the cancellation rule gave it, so that no call's end is unaccounted for.
> - S12 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person allowed to use an account, I want the screen and server to distinguish that from permission to lend it, so I cannot accidentally grant my agent authority I do not hold.
> - S13 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As Dana, I want my private secrets and their metadata isolated from Tom and his agents unless I grant access, so knowing an identifier or signing in to the same installation does not disclose them.
> - S14 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person reviewing a grant or revocation, I want its source, selected service account and confirmed or unconfirmed outcome shown, so I know exactly what authority changed.

## Purpose

Agents get a short-lived, limited handle to a credential, never the credential itself (ADR-001), and the token revolver is the first user (ADR-002). This brief is the temporary key model of the statement, one requirement per part, so each row can be reviewed by Waffles and built on its own; it is step 3 of the road: 'Store, handle, proxy, rotation under 1 handle' (docs/design/identity/STATEMENT-2026-09-22.md:144). Amendment of 23 September 2026: the You and Secrets screens must distinguish permission to use an account from affirmative permission to lend it, with the responsible person and the source grant visible. This amendment is awaiting Waffles review; the implementation is not dispatched or claimed complete.

## Task

Build the broker the statement's 'Secrets: the temporary key model' describes, in Rust, inside the door (docs/design/identity/STATEMENT-2026-09-22.md:23-57), applying the lys formats 'Where it lives: lys' names (docs/design/identity/STATEMENT-2026-09-22.md:82). The store and the proxy are the door's; the handle's and the sealed record's formats are lys's. In: the handle and the proxy swap with its audit line (R1); rotation under one handle (R2); the token revolver as the first consumer (R3); OAuth refresh at the proxy (R4); the seat's own login at spawn (R5); sealed knowledge tagged in SpiceDB (R6); the three revocation cases (R7); the cancellation rule for calls in flight (R8); leases counted by uses, time window and spend (R9). Out: OpenBao or any external secrets engine; the proxy-handle login path; the delegation schema; anything the statement leaves open, each recorded open in the row it touches. Door-owned files are named in each spec with owner and citation and are not in files until a root naming the door repository is set. Every path is relative to its own repository's root. Conformance amendment source: docs/design/identity/CONFORMANCE.md at commit 1353c22, especially rows 3.3 and 7.1–7.8. Trace each executable acceptance identifier below to that committed behaviour map. Tom's 23 September 2026, 19:04:22 Melbourne request is context, not the checkable source. Existing SECRETS-001 execution records describe the earlier authoring task and are not evidence for this amendment. The broker-host ownership must be settled against the standalone identity product before the proposed Cambium paths below become executable walls.

## Requirements

### R1: Issue a handle and swap it at the proxy, with one audit line per use

WHEN a seat makes an outbound call carrying its handle, THE SYSTEM SHALL have the door's proxy check SpiceDB, swap the handle for the real credential, forward the call, and write one audit line naming which seat, which handle, which real account and when (docs/design/identity/STATEMENT-2026-09-22.md:27). A handle is a short-lived value bound to the seat's identity; the real credential sits in the door's encrypted store and never leaves the server (docs/design/identity/STATEMENT-2026-09-22.md:27). A handle is issued only under a grant that traces to a person (ADR-003; docs/design/identity/STATEMENT-2026-09-22.md:21). THE SYSTEM SHALL NOT return, log, or put into any error or Debug output a byte of the real credential, and SHALL NOT forward a call whose handle is unknown, dropped, bound to another identity, or whose SpiceDB check is not a permit.

Owners. The store and the proxy are the door's (docs/design/identity/STATEMENT-2026-09-22.md:25, 'We build this ourselves, in Rust, inside the door'; docs/design/identity/STATEMENT-2026-09-22.md:27). The handle's format is lys: 'Credential handover and sealed knowledge are lys sealed envelopes and the delegation format (lys/delegation/v1, a seat as a typed subject). The secrets broker's handle and the sealed record are these, applied' (docs/design/identity/STATEMENT-2026-09-22.md:82). The audit line is a lys log entry, 'not a row in a database' (docs/design/identity/STATEMENT-2026-09-22.md:80), appended through lys-log-store's Log (crates/lys-log-store/src/lib.rs:1-7).

Lys-owned, in files: crates/lys-core/src/delegation/mod.rs (lys; docs/design/identity/STATEMENT-2026-09-22.md:82; the module is the lys/delegation/v1 artifact, crates/lys-core/src/delegation/mod.rs:1-3), whose docs record the handle as an application of the format; docs/design/WIRE-FORMATS.md (lys; its section 1 is the register of frozen contracts, docs/design/WIRE-FORMATS.md:11-21), which records the handle's format before any handle is signed.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the encrypted store of real credentials (docs/design/identity/STATEMENT-2026-09-22.md:27); crates/cambium-door/src/http/secrets_handle.rs, issuing and resolving handles bound to an identity (docs/design/identity/STATEMENT-2026-09-22.md:27); crates/cambium-door/src/http/secrets_proxy.rs, the check, swap, forward and audit line (docs/design/identity/STATEMENT-2026-09-22.md:27). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: (a) the delegation schema, which the statement leaves unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003). (b) Which lys/delegation/v1 pair a handle would use: the only pair v1 defines for a seat is seat with speaks-for, and that role 'has defined semantics, no implementation and no consumer ... Do not invent a consumer for it' (crates/lys-core/src/delegation/artifact.rs:242-254), so whether the handle is that consumer or a new version alongside is Tom's to settle. (c) Delegation is behind unstable-anchor and 'No delegation may be signed outside tests until it is' ratified (crates/lys-core/src/delegation/mod.rs:237-242), and the lys-core release that freezes it waits for the fold (CLAUDE.md:23). (d) Whose key signs a proxy audit line: the statement says each proxy call is 'signed with the agent's key' (docs/design/identity/STATEMENT-2026-09-22.md:80), while the door holds only the seat's public key and never persists the private half (crates/cambium-door/src/http/agent_seat.rs:13-15). (e) The leaf schema of an audit line, which the statement leaves open as 'the event schema shared with the audit lines' (docs/design/identity/STATEMENT-2026-09-22.md:187).

Conformance amendment (ADR-003; STATEMENT-2026-09-22.md, Everything is pegged to a human authority; docs/design/identity/CONFORMANCE.md at commit 1353c22 row 7.3): WHEN a person or agent requests a derived handle, THE SYSTEM SHALL independently authorise both exercise and delegation against the current source grant. Permission to use a secret, possession of a handle, a display-only owner label or the absence of a prohibition SHALL NOT establish permission to lend. A person verified by the server as the actual owner has the affirmative may-lend route stated in docs/design/identity/CONFORMANCE.md at commit 1353c22, row 7.3; no separate borrowed pass-on grant is required for that owner route. A non-owner requires a valid human-rooted delegation explicitly permitting re-lending. Both routes enforce the requested recipient policy and bounds, identify their responsible person, and retain the actual ownership or source-grant evidence in the decision. A people-only account SHALL refuse every agent recipient, including the owner's own agent. The same check SHALL apply to direct API and agent-tool calls as to the screen; hiding a button is not enforcement. The refusal SHALL identify the blocking grant or policy without exposing another person's secret metadata or value. Stable operation identity SHALL survive a lost acknowledgement so retry cannot mint a second handle. The public record SHALL show holder, responsible person, source grant, permitted operations/resource, pass-on rights, expiry and current state; it SHALL contain no credential material.

**Acceptance:**
- A test sends one call through the proxy with a live handle to an upstream test double: the double receives exactly 1 request carrying the stored credential, and the seat-visible request and response contain no byte of that credential.
- A test sends one call each with an unknown handle, a dropped handle, and a handle bound to another seat's identity: each is refused, and the upstream test double's request count stays 0 across all 3.
- A test in which SpiceDB answers anything other than a permit for the call's relation: the call is refused and the upstream test double's request count is 0.
- A test forwards 3 calls: exactly 3 audit lines are appended to the lys log, each naming the seat, the handle, the real account and the time, and the log's size grows by exactly 3.
- A redaction test formats the store's credential type, the proxy's error type and the audit line with Debug and Display: none of the outputs contains a byte of the credential.
- A test asks the door to issue a handle for an agent whose grant does not trace to a person: no handle is issued.
- docs/design/WIRE-FORMATS.md and crates/lys-core/src/delegation/mod.rs name the handle's format before any handle is signed outside tests, and the points recorded open above are still recorded open, not decided, when the row is reviewed.
- SEC_USE_NOT_LEND: permit Tom to use Dana's finance-readonly account but give Tom no delegation right; Tom's proxied use succeeds, and both browser issuance and a direct API request for Tom's own agent are refused. Exactly zero derived handles, success-audit events or upstream calls result from the refused issuances.
- SEC_AFFIRMATIVE_LEND: verify both routes from CONFORMANCE row 7.3: the actual secret owner can issue a bounded handle without a separate borrowed pass-on grant; a non-owner can issue only with a valid human-rooted delegation permitting re-lending. Remove that delegation while retaining use-only permission and refuse the non-owner. A display-only owner label or ownership of a different secret cannot substitute for the real ownership record. Record which route authorised each success.
- SEC_PEOPLE_ONLY: Dana is the server-verified owner of the accounts-team credential, so ownership establishes her may-lend authority. An explicit applicable recipient policy permits people only. Issuance to her agent is refused by that recipient policy, not by denying Dana's ownership route. With an agent-permitting policy the owner route succeeds. Direct tool/API requests enforce the same decision.
- SEC_ISSUE_RETRY: lose the response after a committed issuance, repeat the exact operation ID and payload, then change the payload under that ID. The repeat returns the original handle identity with one logical issuance event; the changed payload is refused by name.
- SEC_AUTHORITY_TRACE: the visible handle record and audit carry the same holder, responsible person and source-grant IDs returned by the issuer. Altering a supplied parent, recipient kind, action set or resource cannot increase effective authority. Count the refused cases explicitly.

**Files:**
- modify: crates/lys-core/src/delegation/mod.rs
- modify: docs/design/WIRE-FORMATS.md

**Checklist:**
- C4 — A seat's call with its handle goes through the door's proxy, which checks SpiceDB, swaps the handle for the real credential, forwards the call and writes one audit line naming the seat, the handle, the real account and the time; the credential never leaves the server.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C15 — Real secret ownership or a valid human-rooted delegation permitting re-lending establishes affirmative may-lend; mere use and display labels do not. Applicable recipient policy is checked separately, including people-only refusal.
- C18 — Issuance, account selection, retry and revocation preserve exact provenance and current authority; the screen distinguishes local refusal from unconfirmed provider action.

**Stories:**
- S2 (Person, Grants an agent provisioned under them access to an account) — As a person granting an agent access, I want to give it a handle under my own grant, limited by uses, time and spend, so that it can use the account without ever holding the credential and the grant traces back to me.
- S4 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to make my call with my handle and have the proxy swap in the credential, refreshing an expired OAuth token itself, so that I can do my work without ever seeing a credential.
- S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.
- S12 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person allowed to use an account, I want the screen and server to distinguish that from permission to lend it, so I cannot accidentally grant my agent authority I do not hold.
- S14 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person reviewing a grant or revocation, I want its source, selected service account and confirmed or unconfirmed outcome shown, so I know exactly what authority changed.

### R2: Rotate across a set of real accounts under one handle

WHEN a call arrives on a handle whose store entry keeps a set of real accounts, THE SYSTEM SHALL have the proxy take the next one in turn for that call and log which one served it (docs/design/identity/STATEMENT-2026-09-22.md:33). The handle is the stable name (docs/design/identity/STATEMENT-2026-09-22.md:33). Resting an account SHALL be a change in the store with no file copied to any machine, and this SHALL replace the account pool file (docs/design/identity/STATEMENT-2026-09-22.md:33; ADR-002). THE SYSTEM SHALL NOT give a rested account a call, and SHALL NOT trust the spread across accounts to each worker: it is enforced at the proxy (docs/design/identity/STATEMENT-2026-09-22.md:33).

Owner: the door, which keeps the store and the proxy (docs/design/identity/STATEMENT-2026-09-22.md:25-27, docs/design/identity/STATEMENT-2026-09-22.md:33). No lys-owned file changes for rotation; the account that served each call is a field of the R1 audit line.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the set of real accounts behind a handle and the rested mark (docs/design/identity/STATEMENT-2026-09-22.md:33); crates/cambium-door/src/http/secrets_rotation.rs, taking the next account in turn (docs/design/identity/STATEMENT-2026-09-22.md:33). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

Retiring the pool file waits until its consumers have migrated (docs/design/identity/STATEMENT-2026-09-22.md:167), and an engine without the broker reads its own pool file as it does today (docs/design/identity/STATEMENT-2026-09-22.md:62; ADR-004).

Every account selected by rotation SHALL be eligible under the handle's same resource/action and owner boundaries. Rotation SHALL NOT turn a read-only handle into a write capability or select an account belonging to another person merely because it is in an available pool.

**Acceptance:**
- A test with 3 accounts behind one handle sends 6 calls: each account serves exactly 2, in turn, and each of the 6 audit lines names the account that served it.
- A test rests 1 of the 3 accounts in the store and sends 4 calls: the rested account serves 0 of them and the other 2 serve 2 each.
- Resting an account is a single store change: the test performs no file write outside the store and touches no pool file.
- A test with every account behind a handle rested sends 1 call: it is refused and the upstream test double's request count is 0.
- SEC_ROTATION_SCOPE: place one eligible account and one account outside the source grant in a test pool. Calls under that handle use only the eligible account; after it is rested the next call refuses instead of selecting the out-of-scope account. The audit identifies the selected account without disclosing its credential.

**Checklist:**
- C5 — The store keeps a set of real accounts behind one handle, the proxy takes the next in turn and logs which one served, and resting an account is a store change with no file copied to any machine.
- C17 — Every derived handle stays inside its live ancestry, including shared use/spend budgets and expiry; revoking its source does not revoke an independently authorised sibling.

**Stories:**
- S8 (Operator, Keeps the accounts and revokes access) — As an operator, I want to rest an account by one change in the store, so that no worker gives it another call and no file is copied to any machine.
- S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.
- S12 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person allowed to use an account, I want the screen and server to distinguish that from permission to lend it, so I cannot accidentally grant my agent authority I do not hold.

### R3: Serve the token revolver its next account from the broker

WHEN a worker that runs Claude sessions or builders sees a session print its usage-limit words, THE SYSTEM SHALL let it ask the broker for its next account instead of walking its own ordered list (docs/design/identity/STATEMENT-2026-09-22.md:35; ADR-002). The revolver is the first consumer of the handle (docs/design/identity/STATEMENT-2026-09-22.md:35). The broker's answer SHALL come from the same rotation as R2, and SHALL be attributed to the asking seat. THE SYSTEM SHALL NOT make any engine depend on the broker: an engine without it reads its own pool file as it does today (docs/design/identity/STATEMENT-2026-09-22.md:62; ADR-004), and manifold is never a structural part (docs/design/identity/STATEMENT-2026-09-22.md:61).

Owners. The broker's side is the door's (docs/design/identity/STATEMENT-2026-09-22.md:25-27). The worker's side is the engine's: the revolver read for this brief is manifold's (a launcher arms it, crates/manifold-node/src/seat/launcher.rs:31-40 in the manifold repository read at 3df5ac5f64, and it turns through the operator's pool <data-dir>/supervisor/accounts.json, docs/seat-document.md:569 in that repository), but the statement names no engine as the consumer and the design inventory names no engine repository.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-door/src/http/secrets_next_account.rs, the call a worker makes for its next account (docs/design/identity/STATEMENT-2026-09-22.md:35). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: the owner of the worker-side file. The sources do not settle which engine repository takes the first change, and no engine file is named until they do. Also open with R5: whether the answer is the account's login token at spawn (R5) or a handle through the proxy, which stays unproved (docs/design/identity/STATEMENT-2026-09-22.md:45).

**Acceptance:**
- A test asks for the next account 3 times for one seat against a handle with 2 accounts: the answers alternate between the 2 accounts, and each ask appends exactly 1 audit line naming the seat.
- A test asks for the next account when every account behind the handle is rested: the ask is refused and names no account.
- The door's tests for this call start no engine process and depend on no engine crate: the call is exercised with a test client alone.
- The worker-side owner is recorded open in this requirement when the row is reviewed, and no engine file is named.

**Checklist:**
- C6 — The token revolver asks the broker for its next account instead of walking its own list.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C14 — No broker row depends on any one engine: an engine without the broker reads its own pool file as it does today.

**Stories:**
- S6 (Token revolver, The worker that runs Claude sessions and builders and turns to the next account on usage-limit words) — As the token revolver, when a session prints its usage-limit words, I want to ask the broker for my next account instead of walking my own list, so that the spread across accounts is the broker's and no pool file is copied to my machine.

### R4: Refresh OAuth at the proxy; the seat never sees the refresh token

WHEN a call arrives on a handle whose credential is an OAuth grant, THE SYSTEM SHALL have the proxy swap the handle for a live access token, and refresh it itself when it expires (docs/design/identity/STATEMENT-2026-09-22.md:39). The refresh token sits in the store (docs/design/identity/STATEMENT-2026-09-22.md:39). THE SYSTEM SHALL NOT let the seat see the refresh token or the access token (docs/design/identity/STATEMENT-2026-09-22.md:39). Revoking SHALL drop the handle and MAY also revoke the grant upstream (docs/design/identity/STATEMENT-2026-09-22.md:39).

Owner: the door (docs/design/identity/STATEMENT-2026-09-22.md:25-27, docs/design/identity/STATEMENT-2026-09-22.md:39). No lys-owned file changes for refresh.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, holding the refresh token (docs/design/identity/STATEMENT-2026-09-22.md:39); crates/cambium-door/src/http/secrets_oauth.rs, the refresh at the proxy (docs/design/identity/STATEMENT-2026-09-22.md:39). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

A provider sign-in identity and an OAuth service-access grant SHALL remain distinct records. The account selected during service consent, its provider subject, client registration and consented scopes SHALL be recorded as that service grant's provenance; a sign-in email or the currently displayed person SHALL NOT substitute for the selected provider subject. Lending service access still requires R1's affirmative delegation decision.

**Acceptance:**
- A test with an expired access token sends 1 call: the proxy makes exactly 1 refresh request to the provider test double, forwards the call with the new access token, and the seat-visible response contains neither the refresh token nor either access token.
- A test with a live access token sends 2 calls: the provider test double's refresh request count is 0.
- A test drops an OAuth handle: the next call is refused, and when upstream revocation is asked for, the provider test double receives exactly 1 revocation request.
- A redaction test formats the store's OAuth grant type with Debug: the output contains neither token.
- SEC_OAUTH_ACCOUNT: use separate sign-in and service-consent provider fixtures, select a different account during service consent, and refresh after reopen. The recorded service grant and upstream call use the selected service account and original client; neither a matching email nor sign-in credentials are used to rebind or refresh it. A new client requires a named reconnect rather than an assumed refresh.

**Checklist:**
- C7 — The proxy refreshes an OAuth access token itself from the refresh token in the store, and the seat never sees the refresh token.
- C18 — Issuance, account selection, retry and revocation preserve exact provenance and current authority; the screen distinguishes local refusal from unconfirmed provider action.

**Stories:**
- S4 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to make my call with my handle and have the proxy swap in the credential, refreshing an expired OAuth token itself, so that I can do my work without ever seeing a credential.
- S14 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person reviewing a grant or revocation, I want its source, selected service account and confirmed or unconfirmed outcome shown, so I know exactly what authority changed.

### R5: Put the seat's own login into its environment at spawn

WHEN the engine that runs a seat starts it, THE SYSTEM SHALL have the door put the long-lived OAuth token Claude Code generates into the seat's environment, read from the store (docs/design/identity/STATEMENT-2026-09-22.md:43). This is the one place the credential reaches the process (docs/design/identity/STATEMENT-2026-09-22.md:43, docs/design/identity/STATEMENT-2026-09-22.md:167). It rotates at spawn, not per call (docs/design/identity/STATEMENT-2026-09-22.md:43). THE SYSTEM SHALL record which token went to which seat (docs/design/identity/STATEMENT-2026-09-22.md:56). THE SYSTEM SHALL NOT name any one engine: the engine that starts or ends a seat is whichever one runs the agent (docs/design/identity/STATEMENT-2026-09-22.md:61; ADR-004), and an engine without the broker reads its own pool file as it does today (docs/design/identity/STATEMENT-2026-09-22.md:62).

Owner: the door, which holds the store (docs/design/identity/STATEMENT-2026-09-22.md:43). No lys-owned file changes for the login at spawn.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the login tokens and the record of which went to which seat (docs/design/identity/STATEMENT-2026-09-22.md:43, docs/design/identity/STATEMENT-2026-09-22.md:56); crates/cambium-door/src/http/secrets_spawn_login.rs, the answer an engine receives at spawn (docs/design/identity/STATEMENT-2026-09-22.md:43). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: (a) the proxy-handle login path, where the seat holds a handle in the token variable and its base URL points at the proxy; it 'is not tested with a subscription login and nothing depends on it' (docs/design/identity/STATEMENT-2026-09-22.md:45), and this row builds nothing that depends on it. (b) Whether revoking the login token at the provider makes the seat's next call fail; it 'is proved with the provider before it is promised' (docs/design/identity/STATEMENT-2026-09-22.md:56), and this row promises nothing about it.

**Acceptance:**
- A test spawns 2 seats against a login set of 2 accounts: each spawn answer carries the login for exactly 1 account, the 2 answers differ, and the store's record names which account's token went to which seat.
- A test sends 3 calls from one spawned seat: the seat's login does not change between them (rotation is at spawn only).
- The spawn call is exercised by a test client with no engine crate in its dependencies.
- The proxy-handle login path and the provider-revocation question are recorded open in this requirement when the row is reviewed, and no acceptance criterion of this brief depends on either.

**Checklist:**
- C8 — The door puts the seat's own login into its environment at spawn, read from the store, rotating at spawn, and records which token went to which seat.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C14 — No broker row depends on any one engine: an engine without the broker reads its own pool file as it does today.

**Stories:**
- S7 (Engine, Whichever runtime starts and ends a seat) — As the engine starting a seat, I want the door to give me the seat's own login from the store at spawn and record which token went to which seat, so that I read no pool file when the broker is there and still work from my own when it is not.

### R6: Keep sealed knowledge in the store, tagged in SpiceDB

THE SYSTEM SHALL keep keys that cannot be rotated, and memories an identity wants kept secret, as sealed records in the same store, encrypted, each tagged in SpiceDB with which identities may read it (docs/design/identity/STATEMENT-2026-09-22.md:49). WHEN a seat asks for a sealed record by name, THE SYSTEM SHALL check the relation, return the text, and write one audit line (docs/design/identity/STATEMENT-2026-09-22.md:49). Each identity's sealed records are its own (docs/design/identity/STATEMENT-2026-09-22.md:49). THE SYSTEM SHALL NOT let a sealed record sit in plain text in a memory file (docs/design/identity/STATEMENT-2026-09-22.md:49), and SHALL NOT return a key by reading it: a key is used through the proxy even when it cannot rotate; only memories are read, in the smallest piece asked for (docs/design/identity/STATEMENT-2026-09-22.md:57; ADR-001).

Owners. The store, the SpiceDB tag and the read are the door's (docs/design/identity/STATEMENT-2026-09-22.md:49). The sealed record's format is lys sealed envelopes, applied (docs/design/identity/STATEMENT-2026-09-22.md:82).

Lys-owned, in files: crates/lys-core/src/seal/mod.rs (lys; docs/design/identity/STATEMENT-2026-09-22.md:82; the module is the sealed envelope, crates/lys-core/src/seal/mod.rs:1-10), whose docs record the sealed record as an application of it; docs/design/WIRE-FORMATS.md (lys; the lys/sealed-envelope/v1 row, docs/design/WIRE-FORMATS.md:18), which records the application.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the sealed records (docs/design/identity/STATEMENT-2026-09-22.md:49); crates/cambium-door/src/http/secrets_sealed.rs, the read by name with its relation check and audit line (docs/design/identity/STATEMENT-2026-09-22.md:49). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: lys/sealed-envelope/v1 seals with an empty AAD (docs/design/WIRE-FORMATS.md:18; crates/lys-core/src/seal/sealed_envelope.rs:180-183), so an envelope carries nothing that binds it to its name or its owning identity. Whether 'each identity's sealed records are its own' is held by a construction alongside v1 or by the door is not settled by the statement; any construction is a new version alongside, never a change to the shipped one, and takes an adversarial review (CLAUDE.md, coding standards).

Personal, team and organisation scopes SHALL be enforced at list, metadata, read, use and lending seams. A personal record belongs to its recorded person; another person's sign-in or agent ownership creates no access to it. Knowing a record ID or path SHALL NOT bypass its policy. Public denials SHALL avoid leaking the existence, title or owner of a record the caller cannot discover.

**Acceptance:**
- A test in which SpiceDB permits the asking identity reads one memory record by name: the text is returned and exactly 1 audit line is appended naming the seat, the record and the time.
- A test in which SpiceDB does not permit the asking identity: the read is refused, no text is returned, and the record is not decrypted.
- A test asks to read a record marked as a key: the read is refused, and the key is usable only through the proxy of R1.
- A test moves one identity's sealed record to another identity's name in the store and reads it as the second identity: the read returns no text.
- A test searches every memory file the door writes for a record's plaintext after sealing it: 0 matches.
- SEC_PRIVATE_SCOPE: create distinct personal memories and credentials for Tom and Dana. For each person, enumerate, read by known ID and request a derived handle as the other person and as that person's agent. Every unauthorised leg returns no protected metadata/plaintext, performs no decryption or credential forwarding and creates no handle; same-owner authorised controls prove each route actually ran.

**Files:**
- modify: crates/lys-core/src/seal/mod.rs
- modify: docs/design/WIRE-FORMATS.md

**Checklist:**
- C9 — Sealed records are kept encrypted in the store, tagged in SpiceDB with which identities may read them, read by name with a relation check and one audit line, and a key is used through the proxy, never read.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C16 — Personal, team and organisation secret boundaries are enforced in listing, metadata, read, use and lending; knowing another identity's record ID grants nothing.

**Stories:**
- S5 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want to ask for one of my sealed memories by name and get back only the piece I am permitted, so that a secret never sits in plain text in my memory files.
- S10 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.
- S13 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As Dana, I want my private secrets and their metadata isolated from Tom and his agents unless I grant access, so knowing an identifier or signing in to the same installation does not disclose them.

### R7: Revoke in the three cases the statement names

THE SYSTEM SHALL revoke in the 3 cases the statement splits (docs/design/identity/STATEMENT-2026-09-22.md:53-57), each with its own story. (1) A handle: WHEN a handle is dropped, THE SYSTEM SHALL refuse every new call on it, because the process never had the credential (docs/design/identity/STATEMENT-2026-09-22.md:55, docs/design/identity/STATEMENT-2026-09-22.md:29); a call already admitted follows the cancellation rule of R8. (2) The login token: it is in the process; THE SYSTEM SHALL record which token went to which seat, and the engine that runs the seat ends it on its own (docs/design/identity/STATEMENT-2026-09-22.md:56; docs/design/identity/STATEMENT-2026-09-22.md:61). (3) Sealed knowledge: once read it is in the process's context; permission controls the disclosure and the audit line records it, and neither takes back what was read (docs/design/identity/STATEMENT-2026-09-22.md:57). THE SYSTEM SHALL NOT describe revocation of a login token or of read knowledge as taking anything back. The real key is rotated upstream only when it is suspected leaked (docs/design/identity/STATEMENT-2026-09-22.md:29).

Owners: the door for dropping handles and refusing calls (docs/design/identity/STATEMENT-2026-09-22.md:27-29); the engine that runs the seat for ending it (docs/design/identity/STATEMENT-2026-09-22.md:56, docs/design/identity/STATEMENT-2026-09-22.md:61). Revocation in lys 'is itself an append with the live set folded from the log (DP26)' (docs/design/identity/STATEMENT-2026-09-22.md:79; docs/design/lys-anchor/DECISIONS.md:400), a fold 'ruled (DP26), not built' (docs/design/identity/STATEMENT-2026-09-22.md:126).

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-door/src/http/secrets_revoke.rs, dropping a handle and answering which seat holds which login token (docs/design/identity/STATEMENT-2026-09-22.md:55-56). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: (a) whether revoking the login token at the provider makes the seat's next call fail, proved with the provider before it is promised (docs/design/identity/STATEMENT-2026-09-22.md:56). (b) The file that carries the lys revocation fold: its owner is lys (docs/design/identity/STATEMENT-2026-09-22.md:126), and the key-history artifact that would carry a fold 'is its own future format' (crates/lys-core/src/delegation/mod.rs:208-210); no file is named until that format is designed.

Withdrawing a source grant SHALL stop fresh issuance and fresh use through every grant and handle derived from that source. A separate valid grant to the same holder or resource SHALL remain independent and SHALL NOT be revoked merely because it shares an identity or account. Proposal under review, not a settled source rule: if the broker cannot establish a permission decision at least as fresh as the revocation, refuse the affected admission by name rather than use a cached permit. The freshness mechanism and whether admission can await a fresh decision must be settled before dispatch. Local refusal of new proxy calls and upstream revocation confirmation SHALL be represented separately; a timeout SHALL remain unconfirmed, never displayed as completed. Unknown mutation outcomes retain their operation IDs for reconciliation.

The general identity-state policy is pending in docs/design/identity/CONFORMANCE.md at commit 1353c22, rows 3.2 and 3.4, and must be reviewed before this row is dispatched. The specific virtual-credential hold on suspension follows accepted row 3.3. Reinstatement SHALL re-evaluate current grants and leases; it SHALL NOT resurrect a separately revoked handle, an expired provisional grant, a retired identity or authority withdrawn by an ancestor. A suspension being lifted is not an issuance or renewal event.

**Acceptance:**
- A test drops a handle and then sends 3 new calls on it: all 3 are refused and the upstream test double's request count stays at its value before the drop.
- A test revokes a seat's login: the door's answer names the seat the token went to, and the door itself ends no process.
- A test revokes a sealed record's relation after one read: the next read is refused, the earlier audit line is still in the log, and the door's answer does not claim the earlier read is undone.
- The provider-revocation question and the fold's file are recorded open in this requirement when the row is reviewed.
- SEC_REVOKE_CHAIN: create a person-to-agent-to-agent chain with explicit pass-on grants and an independent sibling source. Revoke the chain root, then exercise every descendant. All chain-derived calls refuse and forward zero new upstream requests; the independently authorised control still works.
- SEC_REVOKE_FRESHNESS (proposed consistency acceptance; review required before dispatch): hold a permission replica behind the revocation revision. Fresh use must either await a sufficiently fresh decision or refuse by name; it never reaches upstream on the stale permit. Assert the selected behaviour and the exact zero-forward count.
- SEC_REVOKE_STATES: acknowledge local handle revocation while withholding provider acknowledgement. API and screen show local use stopped and upstream unconfirmed; no timer changes it to confirmed. Deliver the exact matching provider outcome and verify the state transitions once without a second revoke operation.
- SEC_REINSTATE_CURRENT: suspend a holder with three handles, independently revoke one and expire another while suspended, then reinstate the holder. Only the third still-authorised handle can resume; the revoked and expired handles stay refused and no new issuance/renewal is recorded.

**Checklist:**
- C10 — Each of the three revocation cases has its own behaviour: a dropped handle refuses every new call, the login token's seat is ended by its engine, and read sealed knowledge is disclosed and recorded, never claimed back.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C17 — Every derived handle stays inside its live ancestry, including shared use/spend budgets and expiry; revoking its source does not revoke an independently authorised sibling.
- C18 — Issuance, account selection, retry and revocation preserve exact provenance and current authority; the screen distinguishes local refusal from unconfirmed provider action.

**Stories:**
- S3 (Person, Grants an agent provisioned under them access to an account) — As a person, I want to drop an agent's handle and have every new call on it refused at once, so that taking access back does not wait on rotating the real key.
- S9 (Operator, Keeps the accounts and revokes access) — As an operator revoking access, I want each case to say what it reaches: a handle refuses new calls, a call already admitted follows the stated cancellation rule, a login token's seat is ended by its engine, and read knowledge stays read, so that I am never told revocation took back what it cannot.
- S14 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person reviewing a grant or revocation, I want its source, selected service account and confirmed or unconfirmed outcome shown, so I know exactly what authority changed.

### R8: State and enforce the cancellation rule for calls already admitted

WHILE a call already admitted by the proxy is in flight, a drop of its handle does not stop it; THE SYSTEM SHALL have the proxy carry an explicit cancellation rule for calls in flight, and that rule is part of the broker's design (docs/design/identity/STATEMENT-2026-09-22.md:55). THE SYSTEM SHALL write the rule down, in the door's broker module docs, before it is implemented, and SHALL give every call in flight at a drop an outcome the audit records. THE SYSTEM SHALL NOT leave the outcome of a call in flight at a drop undefined. What the rule is (let every admitted call finish, cancel at the next boundary, or another shape) is not stated by the statement; it is settled in this row's review (CN2) before any code, not here.

Owner: the door, whose proxy admits calls (docs/design/identity/STATEMENT-2026-09-22.md:27, docs/design/identity/STATEMENT-2026-09-22.md:55). Chippy's release 2 names 'demonstrate revocation and recovery after a crash' (docs/design/identity/STATEMENT-2026-09-22.md:155) and a retry needing 'a defined outcome' (docs/design/identity/STATEMENT-2026-09-22.md:70), which the rule must also answer for a call in flight when the door stops.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-door/src/http/secrets_in_flight.rs, the register of admitted calls and the rule applied to them at a drop (docs/design/identity/STATEMENT-2026-09-22.md:55). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: the rule itself.

**Acceptance:**
- The door's broker module docs state the cancellation rule in words before the row's first code commit.
- A test admits 2 calls against a slow upstream test double, drops the handle while both are in flight, then sends 1 new call: the new call is refused, and each of the 2 in-flight calls ends with the outcome the stated rule names, each with exactly 1 audit line recording that outcome.
- A test stops the door with 1 call in flight and restarts it: the call's outcome is recorded in the audit, and a retry of that call has the outcome the stated rule names.

**Checklist:**
- C11 — The proxy has an explicit cancellation rule for calls already admitted when their handle is dropped, written down before it is built.

**Stories:**
- S9 (Operator, Keeps the accounts and revokes access) — As an operator revoking access, I want each case to say what it reaches: a handle refuses new calls, a call already admitted follows the stated cancellation rule, a login token's seat is ended by its engine, and read knowledge stays read, so that I am never told revocation took back what it cannot.
- S11 (Reviewer, Reads the audit of what the broker did) — As a reviewer reading the audit, I want each call that was in flight when its handle was dropped to show the outcome the cancellation rule gave it, so that no call's end is unaccounted for.

### R9: Count leases by uses, time window and spend

THE SYSTEM SHALL treat everything handed out as a lease: a number of uses, a time window, a spend cap (docs/design/identity/STATEMENT-2026-09-22.md:68). The time window lives in the signed delegation; uses and spend are counted by the door, because a signed object cannot count (docs/design/identity/STATEMENT-2026-09-22.md:68). WHEN two requests arrive at once on a one-use grant, THE SYSTEM SHALL NOT spend it twice, and a retry SHALL have a defined outcome (docs/design/identity/STATEMENT-2026-09-22.md:70). A hard cap SHALL need a reservation before work starts and a settlement after, so two concurrent sessions cannot both spend the same remaining allowance (docs/design/identity/STATEMENT-2026-09-22.md:70); hard spending caps ship only where reservation and enforcement are proved (docs/design/identity/STATEMENT-2026-09-22.md:156). The proxy sees only the spending that passes through it (docs/design/identity/STATEMENT-2026-09-22.md:70), and THE SYSTEM SHALL NOT report a spend total as covering what did not pass through the proxy.

Owners: the door counts uses and spend (docs/design/identity/STATEMENT-2026-09-22.md:68). The time window lives in the signed delegation, whose format is lys (docs/design/identity/STATEMENT-2026-09-22.md:68, docs/design/identity/STATEMENT-2026-09-22.md:82).

Lys-owned, in files: crates/lys-core/src/delegation/mod.rs (lys; docs/design/identity/STATEMENT-2026-09-22.md:82), whose 'Expiry. There is no not_after' paragraph (crates/lys-core/src/delegation/mod.rs:218-221) must be answered for a lease's time window; docs/design/WIRE-FORMATS.md (lys; docs/design/WIRE-FORMATS.md:11-21), which records the delegation that carries it.

Door-owned, named here only (the door repository (the cambium checkout the design inventory names, read at cbcd2cc9d; paths below are relative to that repository's root)): crates/cambium-store/src/traits/secrets.rs, the use and spend counters and reservations (docs/design/identity/STATEMENT-2026-09-22.md:68, docs/design/identity/STATEMENT-2026-09-22.md:70); crates/cambium-door/src/http/secrets_lease.rs, the check, reservation and settlement at the proxy (docs/design/identity/STATEMENT-2026-09-22.md:68-70). Door-owned files are named here with their owner and citation and are not listed in files, because no root naming the door repository is set; each moves into files when that root is set. Door paths are proposed new files: the door has no secrets module today (the statement's table, docs/design/identity/STATEMENT-2026-09-22.md:128, records the broker as not started in the cambium door), and they sit where the door keeps its kinds of file today: store traits in crates/cambium-store/src/traits/ (crates/cambium-store/src/traits/mod.rs), HTTP handlers in crates/cambium-door/src/http/ registered in crates/cambium-door/src/http/router.rs.

OPEN, not decided here: the delegation schema (docs/design/identity/STATEMENT-2026-09-22.md:21), and with it how a time window is carried: lys/delegation/v1 has no expiry by design (crates/lys-core/src/delegation/mod.rs:218-221), so a window in the signed delegation is a new version alongside v1, never a change to it; its shape waits on the schema.

Derived leases SHALL be bounded by every live ancestor's resource, action, recipient-kind, time, use and spend restrictions. Splitting authority across two children SHALL NOT duplicate the parent's remaining use or spend budget. Delegation SHALL NOT renew a provisional grant or move its end date. A role-definition version change is distinct from the holder's grant and SHALL NOT silently extend that grant's lease. Expiry is checked against the named clock at admission, not merely displayed by the browser.

**Acceptance:**
- A test with a lease of 2 uses sends 3 calls: the first 2 are forwarded and the 3rd is refused, and the upstream test double's request count is 2.
- A test sends 2 simultaneous requests on a one-use grant: with both held at the use check until both have arrived, so the race is forced rather than hoped for, exactly 1 is forwarded and the other is refused.
- A test retries a call whose first attempt was forwarded: the retry has the outcome the row's docs state, and the use count moves at most once.
- A test runs 2 concurrent sessions against a hard cap with room for 1: exactly 1 reservation succeeds, and the settled spend never exceeds the cap.
- A test sends a call after its lease's time window has ended: it is refused.
- The door's spend report labels its total as the spending that passed through the proxy.
- The delegation schema and the carriage of the time window are recorded open in this requirement when the row is reviewed.
- SEC_LEASE_ATTENUATION: request a child lease ending after its parent, with broader actions/resource or an unpermitted recipient kind. Each request is refused naming the exceeded boundary; a strictly narrower control succeeds. An expired ancestor refuses even when the child's own displayed end date is later.
- SEC_SHARED_ALLOWANCE: derive two handles under a source with one use remaining; hold two calls at their shared reservation boundary and release them together. Exactly one upstream call occurs and the other refuses. Reopen and retry the accepted operation; the source allowance remains consumed once.
- SEC_PROVISIONAL_EXPIRY: advance a controlled clock to the holder's grant end and try both use and child issuance. Both refuse, including after editing the role definition or moving the holder to another role version; no action renews the expired grant without a separately authorised grant operation.

**Files:**
- modify: crates/lys-core/src/delegation/mod.rs
- modify: docs/design/WIRE-FORMATS.md

**Checklist:**
- C12 — Everything handed out is a lease counted by uses, a time window and a spend cap: the window in the signed delegation, uses and spend counted by the door, a one-use grant never spent twice.
- C13 — Every point the statement leaves open is recorded open in the row it touches and is not decided there.
- C17 — Every derived handle stays inside its live ancestry, including shared use/spend budgets and expiry; revoking its source does not revoke an independently authorised sibling.

**Stories:**
- S2 (Person, Grants an agent provisioned under them access to an account) — As a person granting an agent access, I want to give it a handle under my own grant, limited by uses, time and spend, so that it can use the account without ever holding the credential and the grant traces back to me.
- S12 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person allowed to use an account, I want the screen and server to distinguish that from permission to lend it, so I cannot accidentally grant my agent authority I do not hold.

## Boundaries

- No credential, token or key value is ever written, read or quoted in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- A credential never passes through an agent: the only credential that reaches a process is the seat's own login at spawn (R5).
- Nothing the statement leaves open is decided in a row: the delegation schema, provider revocation of a login token, the proxy-handle login path, the worker-side owner, the fold's file, the audit line's schema and signer, and the cancellation rule until its review.
- Manifold is never a structural part: the engine that starts or ends a seat is whichever one runs the agent, and no row depends on any one engine.
- Every project works without the others: an engine without the broker reads its own pool file as it does today, and the door without the broker signs people in as it does today.
- No OpenBao and no external secrets engine.
- A shipped wire format is never mutated: lys/sealed-envelope/v1 and lys/delegation/v1 evolve only by a new version alongside, and any cryptographic change takes an adversarial review before it lands.
- No door file is edited in a round without a root naming the door repository.
- No row is dispatched until Waffles has reviewed it.
- A mock-up owner label, an absent deny flag or a role name is never ownership evidence. Server-verified ownership of the secret is an affirmative may-lend route under docs/design/identity/CONFORMANCE.md at commit 1353c22, row 7.3; a non-owner needs a valid human-rooted delegation permitting re-lending. Policy and authority checks are performed at the server for every UI/API/MCP route; the UI renders that answer.

## Verification

- From the lys repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the door repository root: the door's own gates, all clean, once its root is set.
- From docs/ of the lys repository: python3 $DS2_METHOD/scripts/validate.py design/secrets and python3 $DS2_METHOD/scripts/check-coverage.py design/secrets exit 0.
- A search of every file a row touches finds no credential, token or key value.
- Map SEC_USE_NOT_LEND through SEC_PROVISIONAL_EXPIRY to the accepted mock-up conformance IDs before dispatch; the code evidence must include independent negative cases and exact exercised counts. A rendered mock-up is a specification artifact, never proof that broker enforcement exists.


---
type: brief
id: SECRETS-004
cluster: secrets
title: Say who may revoke a lease and let its holder relinquish it, and filter the secrets list by scope at the server (CONFORMANCE 7.6 and 7.8)
---

# SECRETS-004: Say who may revoke a lease and let its holder relinquish it, and filter the secrets list by scope at the server (CONFORMANCE 7.6 and 7.8)

> **Cluster:** secrets
> **Depends on:** SECRETS-002
> **Blocked by:** The lead's sign-off on the card before any row is built., R2 to R6 touch crates/lys-secrets, the broker's host under the host decision SECRETS-003 carries on brief/secrets/309540a4: they wait on `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0, and R5 and R6, which build on SECRETS-003 R4's proxy and its revocation states, also wait on `git cat-file -e origin/main:crates/lys-secrets/src/proxy.rs` exits 0; SECRETS-003 lands as one pull request, so R4 arrives with R3., R7 waits on the step-2 SpiceDB brief's evaluator file and on the server's route-building file: `git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/check.rs` exits 0 and `git cat-file -e origin/main:crates/lys-identity-server/src/routes.rs` exits 0. Each check is keyed on the file, because the briefs that land them have no settled id.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-095 — Ownership of a secret alone confers no revoke over the leases derived from it — The person acted for under a lease revokes it, and ownership of the secret the lease was issued from confers no revoke, over the owner revoking every credential derived from the secret, because who may revoke is a policy choice that ownership alone does not confer. Rejected: the owner revokes every credential derived from the secret.
> **Checklist:**
> - C30 — The person acted for under a lease may revoke it at any time: the revoke stops issuing at once, and GET /leases/{lease_id} reads upstream pending until the system behind confirms, with no timer confirming it (CONFORMANCE 7.6).
> - C31 — A revoke by anyone but the person acted for is refused: with the not-found refusal naming nothing when the caller cannot discover the lease, which only its person acted for and its holder can; for the lease's holder with a refusal naming the lease and relinquish; and with a refusal naming the lease for any other caller the seam lets discover it.
> - C32 — A lease's holder may relinquish it, recorded as a relinquish and not as a revoke, which stops issuing at once and reads upstream pending until the system behind confirms.
> - C33 — Whether a secret's owner may revoke every credential derived from it is recorded as a proposed ADR cited from this cluster's decisions, and no code and no test grants it.
> - C34 — The secrets list takes one scope from organisation, team and mine, applied at the server; with no scope it answers every secret the caller may see; any other scope is refused by name (CONFORMANCE 7.8).
> - C35 — The signed-in person's team ids are read from the group claims on their token, one group per team, through one function.
> - C36 — Every secret visibility and lease revoke check is a call on one seam, answering from the record's scope, team and owner fields until the step-2 SpiceDB evaluator exists, and asking SpiceDB through it once it does.
> - C37 — A revoke by the person acted for, or a relinquish by the holder, of a lease that has already ended is refused by name as already ended, carrying how, when and by whom it first ended, and records nothing; the first end record stays the only one (CONFORMANCE 7.6).
> - C38 — The secrets list is a person's view: an agent that asks for it is refused by name, the refusal naming the agent and saying that agents reach secrets only through their virtual credentials, with no list, not even an empty one, and nothing recorded (CONFORMANCE 7.8).
> **Stories:**
> - S19 (Person, Grants an agent provisioned under them access to an account) — As the person an agent acts for under a lease, I want to revoke the lease and see issuing stopped at once and the confirmation from the system behind shown pending until it comes, so that I know what has stopped and what is still unconfirmed.
> - S20 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent holding a lease, I want to give it back by a relinquish recorded as my own act, so that ending my own access is never recorded as a revoke nobody asked for.
> - S21 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person looking at the secrets list, I want to choose organisation, team or mine and have the server answer only what I may see within that scope, so that no filter ever shows me a secret I may not see.

## Purpose

Amends SECRETS-002 for two rows of docs/design/identity/CONFORMANCE.md: row 7.6, who may revoke a lease, in its test part, and row 7.8, the scope filter on the secrets list. The person acted for revokes, the holder relinquishes, and everyone else is refused without learning what they may not see; the secrets list answers one scope from a closed set of three, applied at the server. The open part of row 7.6, whether a secret's owner may revoke every credential derived from it, is recorded as proposed ADR-095, and nothing here grants it.

## Task

A new brief that depends on SECRETS-002 and leaves SECRETS-002's documents unedited. Each row names the SECRETS-002 requirement it extends: R6 for scopes and denials that do not leak, R7 for the revoke's two facts, SEC_REVOKE_STATES. SECRETS-002 R8's in-flight cancellation rule stays open and outside this acceptance. Split with SECRETS-002: C16 (scopes enforced in listing) and C18 (local refusal shown apart from unconfirmed upstream action) stay SECRETS-002's; C30 to C38 are this brief's and narrow them for rows 7.6 and 7.8.

Host: the broker's host is crates/lys-secrets, the host decision SECRETS-003 carries on brief/secrets/309540a4. No ADR id is cited for it: main's ledger ends at ADR-018, and the number is taken from main's ledger by whichever brief lands first. SECRETS-003 builds that crate as a library with no HTTP layer, so this brief adds the routes GET /secrets, GET /leases/{lease_id}, POST /leases/{lease_id}/revoke and POST /leases/{lease_id}/relinquish in crates/lys-secrets/src/api/, as a router the serving process mounts; the serving process and its sign-in are not this brief's. A signed-in person reaches a route from the mounting server's authentication; a lease's holder that is an agent, which does not sign in, reaches POST /leases/{lease_id}/relinquish and GET /leases/{lease_id} by the signed presentation SECRETS-003 defines for an agent's calls to the broker, verified against the key registered for the holder's identity, and no new way to authenticate is added. Tests insert the caller and their group claims directly.

Checks: every visibility and revoke check is a call on the one seam of R3, which answers from the record's scope, team and owner fields until the step-2 SpiceDB brief lands its evaluator, and R7 makes the seam ask SpiceDB through a `PermissionCheck` trait declared in crates/lys-secrets, which crates/lys-identity-server implements with the step-2 brief's one evaluator in crates/lys-identity-server/src/spicedb/check.rs and passes in where the routes are built, and through nothing else; crates/lys-secrets depends on no server crate. SECRETS-003's proxy use checks on a handle are not visibility or revoke checks, and this brief leaves them as they are. R5 and R6 build on SECRETS-003 R4: a lease's end moves through the revocation states R4 defines for SEC_REVOKE_STATES, with no second upstream state, and a use is measured at R4's proxy. Holding to DIRECTORY-002 R4 means the narrow thing: no SpiceDB permission check, relationship write or schema write is added under crates/lys/src/identity/.

In: the owner-revoke question as proposed ADR-095 (R1); team ids from the group claims through one function (R2); the one seam (R3); the scoped list (R4); the revoke and its refusals, and the refusal of an already ended lease (R5); the relinquish (R6); the refusal of the secrets list to an agent (R4); the seam asking SpiceDB (R7). Out, named as neighbours: the lease screen with its Revoke control, and the secrets list screen with its scope control of All, Organisation, Team and Mine. Out: the in-flight rule, upstream-grant withdrawal, emergency stop, a team model in the directory, and any grant to a secret's owner.

## Requirements

### R1: Record who may revoke beyond the person acted for as proposed ADR-095, and grant nothing under it

Extends SECRETS-002 R7, for the open part of CONFORMANCE row 7.6. The decision ledger carries ADR-095, status proposed, scope secrets: the person acted for under a lease revokes it, and ownership of the secret the lease was issued from confers no revoke; the rejected alternative is that the owner revokes every credential derived from the secret. The secrets design cites ADR-095 in its decisions by id. WHILE ADR-095 is proposed, THE SYSTEM SHALL keep the set of who may revoke a lease beyond the person acted for empty. THE SYSTEM SHALL NOT carry any code path, route or test that grants a secret's owner a revoke of a lease they are not acted for under, and this brief SHALL NOT mark ADR-095 decided or edit any other ledger entry.

**Acceptance:**
- docs/design/decisions.json holds exactly 1 entry with id ADR-095, its status reads `proposed`, and its decision names the owner revoking every credential derived from the secret as the rejected alternative.
- The decisions array of docs/design/secrets/design.json contains `ADR-095`.
- Every ledger entry from ADR-001 to ADR-018 is byte-identical to its text before this brief: `git diff` of docs/design/decisions.json shows added lines only, apart from the ledger's `updated` field.

**Files:**
- modify: docs/design/decisions.json
- modify: docs/design/secrets/design.json

**Checklist:**
- C33 — Whether a secret's owner may revoke every credential derived from it is recorded as a proposed ADR cited from this cluster's decisions, and no code and no test grants it.

### R2: Read the signed-in person's team ids from the group claims on their token, through one function

Extends SECRETS-002 R6. Blocked by: `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0. THE SYSTEM SHALL read the team ids of the signed-in person from the identity provider's group claims on their token (ADR-009), one group per team, through the one function `team_ids` in crates/lys-secrets/src/teams.rs, which takes the claims as its argument so that tests inject them. A directory brief that models teams replaces this source inside `team_ids` and nowhere else. THE SYSTEM SHALL NOT read the group claims anywhere but `team_ids`, SHALL NOT take a team id from a request's path, query or body, and SHALL NOT read an agent's team membership for any list: agents never use the secrets list, so an agent's team membership changes no list.

**Acceptance:**
- A test passes claims whose `groups` claim holds `team_a` and `team_b`: `team_ids` returns exactly the 2 ids `team_a` and `team_b`.
- A test passes claims whose `groups` claim is an empty array: `team_ids` returns 0 ids.
- `rg -n 'pub fn team_ids\(' crates/lys-secrets/src` prints exactly 1 line, in crates/lys-secrets/src/teams.rs.

**Files:**
- create: crates/lys-secrets/src/teams.rs
- create: crates/lys-secrets/src/teams_tests.rs
- modify: crates/lys-secrets/src/lib.rs

**Checklist:**
- C35 — The signed-in person's team ids are read from the group claims on their token, one group per team, through one function.

**Stories:**
- S21 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person looking at the secrets list, I want to choose organisation, team or mine and have the server answer only what I may see within that scope, so that no filter ever shows me a secret I may not see.

### R3: Answer every visibility and revoke check through one seam, from the record's scope, team and owner fields

Extends SECRETS-002 R6 and R7. Blocked by: `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0. Each secret record in the store of crates/lys-secrets/src/store.rs, the entry that already carries its owning identity, carries its scope, one of `personal`, `team` and `organisation`, and its team id when its scope is `team`; crates/lys-secrets/src/secret.rs keeps only the redacting type for credential bytes, and that type gains no field; secret.rs gains only the declaration of its test module. Each lease record in crates/lys-secrets/src/lease.rs carries its holder, its person acted for, and the secret it was issued from. THE SYSTEM SHALL answer every secret visibility check, every lease discovery check and every lease revoke check through the one seam in crates/lys-secrets/src/access.rs, which answers from those fields until R7 makes it ask SpiceDB: a caller may see a secret whose scope is `organisation`, a secret whose scope is `team` and whose team is one of the caller's team ids from R2, and a secret whose scope is `personal` and whose owner is the caller; a caller may discover a lease only when they are its person acted for or its holder, and seeing the secret it was issued from does not make a lease discoverable; a caller may revoke a lease only when they are its person acted for. The seam relates to SECRETS-003's proxy this way: the proxy's use checks on a handle stay where SECRETS-003 puts them; every secret visibility, lease discovery and lease revoke check this brief adds is a call on access.rs and on nothing else, and R7 makes access.rs answer through the `PermissionCheck` trait R7 declares in crates/lys-secrets, which crates/lys-identity-server implements with the step-2 evaluator in crates/lys-identity-server/src/spicedb/check.rs. THE SYSTEM SHALL NOT make a visibility, discovery or revoke check anywhere but through this seam, SHALL NOT answer a discovery or revoke check from the secret's owner field or from the secret's visibility (ADR-095), SHALL NOT add a field to the redacting type of secret.rs, and SHALL NOT add any SpiceDB permission check, relationship write or schema write under crates/lys/src/identity/ (DIRECTORY-002 R4).

**Acceptance:**
- A test in crates/lys-secrets/src/access_tests.rs builds its own fixture in that file: person_a in team_a and person_b in team_b, each by one group in injected group claims; secrets a_personal (scope personal, owner person_a), a_team (scope team, team team_a, owner person_a), a_org (scope organisation, owner person_a), b_personal (scope personal, owner person_b), b_team (scope team, team team_b, owner person_b) and b_org (scope organisation, owner person_b). It asks the see check for person_a over the 6 secrets: exactly 4 are answered yes, a_personal, a_team, a_org and b_org, and the test asserts it asked 6 times.
- A test in crates/lys-secrets/src/access_tests.rs, over the same in-file fixture and a lease issued from b_org, acted for person_a and held by agent_a, asks the discover check and the revoke check for the 4 callers person_a, agent_a, person_b (the secret's owner, who sees b_org) and person_c (a person in neither team_a nor team_b): the discover check answers yes for exactly 2, person_a and agent_a; the revoke check answers yes for exactly 1, person_a; and the test asserts it asked 8 times.
- A test in crates/lys-secrets/src/secret_tests.rs destructures the redacting type of crates/lys-secrets/src/secret.rs with a pattern that names every field that type declares at the commit where SECRETS-003 lands it on origin/main, and no rest pattern `..`, so the crate's tests compile only while that type's fields are exactly SECRETS-003's; `cargo test -p lys-secrets --all-features secret_fields_unchanged` reports exactly 1 passed.
- A test in crates/lys-secrets/src/access_tests.rs builds the store's secret record of crates/lys-secrets/src/store.rs for b_team with its scope field set to `team` and its team field set to team_b, reads both fields back, and asserts they are `team` and team_b.
- The diff of this brief's commits lists 0 paths under crates/lys/src/identity/.

**Files:**
- create: crates/lys-secrets/src/access.rs
- create: crates/lys-secrets/src/access_tests.rs
- create: crates/lys-secrets/src/secret_tests.rs
- modify: crates/lys-secrets/src/store.rs
- modify: crates/lys-secrets/src/lease.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/src/secret.rs

**Checklist:**
- C36 — Every secret visibility and lease revoke check is a call on one seam, answering from the record's scope, team and owner fields until the step-2 SpiceDB evaluator exists, and asking SpiceDB through it once it does.

### R4: Answer the secrets list for one scope from organisation, team and mine, applied at the server, to people only (CONFORMANCE 7.8)

Extends SECRETS-002 R6. Blocked by: `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0. WHEN a signed-in person asks GET /secrets with the query parameter `scope` set to one of `organisation`, `team` and `mine`, THE SYSTEM SHALL apply the scope at the server and answer only the secrets the caller may see through R3's seam within it: `mine` is the secrets whose scope is `personal` and whose owner is the caller; `team` is the secrets whose scope is `team` and whose team is one the caller belongs to by R2; `organisation` is the secrets whose scope is `organisation`. Each value is keyed on the secret's own scope. WHEN the request carries no `scope`, THE SYSTEM SHALL answer every secret the caller may see; that list is not a fourth value. IF `scope` carries any other value, THEN THE SYSTEM SHALL refuse with HTTP 400 and the error `unknown_scope` naming the value given, and answer no secret. IF the caller of GET /secrets is an agent, with or without a scope, THEN THE SYSTEM SHALL refuse with HTTP 403 and the error `agent_uses_virtual_credentials`, naming the agent and saying that agents reach secrets only through their virtual credentials, record nothing, and answer no list, not even an empty one; a person is answered as above. THE SYSTEM SHALL NOT answer a secret the caller may not see under any scope or under none, SHALL NOT widen `team` to team secrets of a team the caller does not belong to, SHALL NOT accept a fourth value for the unscoped list, and SHALL NOT let an agent's team membership change any list.

**Acceptance:**
- CONFORMANCE 7.8: with the fixture of tests/support: person_a in team_a and person_b in team_b, each by one group in the injected group claims on their token; secrets a_personal (scope personal, owner person_a), a_team (scope team, team team_a, owner person_a), a_org (scope organisation, owner person_a), b_personal (scope personal, owner person_b), b_team (scope team, team team_b, owner person_b) and b_org (scope organisation, owner person_b), GET /secrets?scope=mine as person_a answers exactly 1 secret, a_personal; GET /secrets?scope=team answers exactly 1 secret, a_team; GET /secrets?scope=organisation answers exactly 2 secrets, a_org and b_org.
- CONFORMANCE 7.8: with the same fixture, GET /secrets with no scope as person_a answers exactly 4 secrets, a_personal, a_team, a_org and b_org, and b_personal and b_team are absent from it and from each of the 3 scoped lists.
- CONFORMANCE 7.8: GET /secrets?scope=all as person_a answers HTTP 400 with error `unknown_scope` naming `all` and 0 secrets, and GET /secrets?scope=user answers HTTP 400 with error `unknown_scope` naming `user` and 0 secrets.
- CONFORMANCE 7.8: giving agent_a, held for person_a, the group `team_b` in its injected claims leaves each of person_a's 4 lists with the same members as before.
- CONFORMANCE 7.8: agent_a calls GET /secrets with no scope, and then with scope=team: each answers HTTP 403 with error `agent_uses_virtual_credentials`, the body names agent_a and says that agents reach secrets only through their virtual credentials, the body carries no list field, the audit log's line count is the same after both calls as before them, and in the same test GET /secrets with no scope as person_a answers exactly 4 secrets, a_personal, a_team, a_org and b_org.

**Files:**
- create: crates/lys-secrets/src/secret_list.rs
- create: crates/lys-secrets/src/api/mod.rs
- create: crates/lys-secrets/src/api/secrets.rs
- create: crates/lys-secrets/tests/secret_list.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/Cargo.toml
- modify: Cargo.lock
- modify: crates/lys-secrets/tests/support/mod.rs

**Checklist:**
- C34 — The secrets list takes one scope from organisation, team and mine, applied at the server; with no scope it answers every secret the caller may see; any other scope is refused by name (CONFORMANCE 7.8).
- C38 — The secrets list is a person's view: an agent that asks for it is refused by name, the refusal naming the agent and saying that agents reach secrets only through their virtual credentials, with no list, not even an empty one, and nothing recorded (CONFORMANCE 7.8).

**Stories:**
- S21 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person looking at the secrets list, I want to choose organisation, team or mine and have the server answer only what I may see within that scope, so that no filter ever shows me a secret I may not see.

### R5: Revoke a lease as the person acted for, and refuse the revoke to everyone else (CONFORMANCE 7.6)

Extends SECRETS-002 R7: the two facts of a revoke are SEC_REVOKE_STATES of SECRETS-002 R7, and SECRETS-002 R8's in-flight cancellation rule stays open and outside this acceptance. Extends SECRETS-002 R6 for the refusal that names nothing. Builds on SECRETS-003 R4: its proxy and the revocation states SEC_REVOKE_STATES names, which R4 defines. Blocked by: `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0 and `git cat-file -e origin/main:crates/lys-secrets/src/proxy.rs` exits 0. WHEN the person acted for under a lease calls POST /leases/{lease_id}/revoke, at any time, THE SYSTEM SHALL stop issuing under the lease at once, record the lease's end as a revoke by that caller, and show upstream pending until the system behind confirms separately; the revoke ends the lease by calling the one pub(crate) function `end_with_upstream_pending` in crates/lys-secrets/src/revoke.rs, which stops issuing, records the end with its act and its caller, and moves the lease's upstream into SECRETS-003 R4's revocation state for upstream unconfirmed, appending one audit line for the transition as R4 does, and is the one function any act that ends a lease with upstream pending calls. GET /leases/{lease_id} SHALL then read ended_by `revoke`, issuing `stopped` and upstream `pending`, and upstream `confirmed` only once the system behind's acknowledgement is delivered, by the one function `deliver_upstream_ack` in crates/lys-secrets/src/revoke.rs, which moves it into R4's revocation state for upstream confirmed and appends one audit line for that transition; `pending` and `confirmed` are the words GET /leases/{lease_id} reads for those two R4 states. A holder who is also the person acted for under that lease revokes as the person acted for. IF the caller cannot discover the lease through R3's seam, which every caller but its person acted for and its holder cannot, THEN THE SYSTEM SHALL refuse with HTTP 404 and the error `lease_not_found`, naming nothing, and SHALL answer GET /leases/{lease_id} the same way. IF the caller is the lease's person acted for and the lease has already ended, by a revoke or the end of its time window, THEN THE SYSTEM SHALL refuse with HTTP 409 and the error `lease_already_ended`, carrying the way it ended (`revoke` or `expired`), the instant it ended and, for a revoke, who ended it, all read from the first end record, and SHALL record nothing, so that the first end record stays the only one. IF the caller is the lease's holder and not its person acted for, THEN THE SYSTEM SHALL refuse with HTTP 403 and the error `holder_relinquishes`, naming the lease and naming relinquish as the holder's act. IF the caller can discover the lease through R3's seam, is not its holder, and the seam answers no to the revoke, THEN THE SYSTEM SHALL refuse with HTTP 403 and the error `revoke_not_permitted`, naming the lease. The upstream state is the field `upstream` of the lease record `Lease` that SECRETS-003 defines in crates/lys-secrets/src/lease.rs, the record its CONTRACT.md `Lease` section describes; its public path is `lys_secrets::Lease`, its re-export from crates/lys-secrets/src/lib.rs, and since SECRETS-003's brief names no such re-export, this row adds that `pub use` to lib.rs. The field `upstream` holds SECRETS-003 R4's revocation state and no second pending or confirmed state: revoke.rs declares no state values of its own. The field is not public outside the crate, so code outside the crate that writes it through `lys_secrets::Lease` fails to compile with E0616 and with no other error, the path `lys_secrets::Lease` being public; its type is a wrapper declared in crates/lys-secrets/src/revoke.rs whose one inner field, holding R4's state, is private to revoke.rs, so no code outside revoke.rs writes the state. `end_with_upstream_pending` is the only writer of upstream unconfirmed (`pending`) and `deliver_upstream_ack` is the only writer of upstream confirmed (`confirmed`). A use of a lease is measured at SECRETS-003 R4's proxy, whose upstream test double counts the requests forwarded to it, as R4's own tests do. THE SYSTEM SHALL NOT keep a second upstream pending or confirmed state beside R4's revocation states, SHALL NOT change upstream `pending` to `confirmed` anywhere but in `deliver_upstream_ack`, so that no timer and no other operation changes it, SHALL NOT let a refused revoke stop issuing, record an end or send any request to the system behind, SHALL NOT record a relinquish the caller did not ask for, SHALL NOT write a second end record for a lease that has already ended, and SHALL NOT let a secret's owner discover or revoke a lease as owner (ADR-095). R6's 9-step test on the revoked lease L1 measures this row's clause that no timer and no other operation changes upstream `pending` to `confirmed`, and R6 names C30 and S19 for it. The refusals cover only the direct act of revoking a lease: withdrawal of an upstream grant under ADR-003 and SEC_REVOKE_CHAIN, and emergency stop under CONFORMANCE row 3.4, end a lease as their own acts and are not changed.

**Acceptance:**
- CONFORMANCE 7.6: with lease L1 issued from a_org, acted for person_a and held by agent_a, person_a calls revoke on L1 while the upstream test double of SECRETS-003 R4's proxy withholds its acknowledgement: the next use of L1 through the proxy is refused, the double's count of forwarded use requests is the same as before the revoke, the audit log gains exactly 1 line for the move to upstream unconfirmed, and GET /leases/L1 reads ended_by `revoke`, issuing `stopped` and upstream `pending`; then `deliver_upstream_ack` runs once for L1 and GET /leases/L1 reads upstream `confirmed`, and the audit log gains exactly 1 more line, for the move to upstream confirmed.
- A `compile_fail,E0616` doctest in crates/lys-secrets/src/revoke.rs takes a `&mut lys_secrets::Lease` and assigns its `upstream` field; `lys_secrets::Lease` is the public re-export from crates/lys-secrets/src/lib.rs, so the path resolves and E0616 is the only error the doctest can hit, and the doctest passes under `cargo test --workspace --all-features`.
- CONFORMANCE 7.6: person_b, who sees a_org and is neither L1's person acted for nor its holder, calls revoke on L1: HTTP 404 with error `lease_not_found`, and the body is byte-identical to the answer for a lease id that does not exist, containing neither L1's id nor any secret title, holder or person; the next use of L1 through SECRETS-003 R4's proxy succeeds and the double's count of forwarded use requests rises by exactly 1, and the double receives 0 revoke requests.
- CONFORMANCE 7.6: agent_a, L1's holder, calls revoke on L1: HTTP 403 with error `holder_relinquishes`, and the body names L1 and names `relinquish`; L1's record carries no end, the next use of L1 through SECRETS-003 R4's proxy succeeds and the double's count of forwarded use requests rises by exactly 1, and the double receives 0 revoke requests.
- CONFORMANCE 7.6: person_b, the owner of b_org, calls revoke on lease L2, issued from b_org, acted for person_a and held by agent_a: HTTP 404 with error `lease_not_found` naming nothing, and L2 keeps issuing.
- CONFORMANCE 7.6: on lease L4, issued from a_org, whose holder and person acted for are both person_a, person_a's revoke is recorded as a revoke: GET /leases/L4 reads ended_by `revoke`.
- CONFORMANCE 7.6: person_a calls revoke on L1 a second time after revoking it: HTTP 409 with error `lease_already_ended`, the body carries the way `revoke`, the instant of the first revoke and person_a, L1 carries exactly 1 end record, and the double receives exactly 1 revoke request in all.
- CONFORMANCE 7.6: person_a calls revoke on lease L5, issued from a_org, acted for person_a and held by agent_a, after the injected clock passes the end of L5's time window: HTTP 409 with error `lease_already_ended`, the body carries the way `expired` and the instant L5's window ended, and L5 carries 0 revoke records and 0 relinquish records.

**Files:**
- create: crates/lys-secrets/src/revoke.rs
- create: crates/lys-secrets/src/api/leases.rs
- create: crates/lys-secrets/tests/lease_revoke.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/src/api/mod.rs
- modify: crates/lys-secrets/src/lease.rs
- modify: crates/lys-secrets/tests/support/mod.rs

**Checklist:**
- C30 — The person acted for under a lease may revoke it at any time: the revoke stops issuing at once, and GET /leases/{lease_id} reads upstream pending until the system behind confirms, with no timer confirming it (CONFORMANCE 7.6).
- C31 — A revoke by anyone but the person acted for is refused: with the not-found refusal naming nothing when the caller cannot discover the lease, which only its person acted for and its holder can; for the lease's holder with a refusal naming the lease and relinquish; and with a refusal naming the lease for any other caller the seam lets discover it.
- C37 — A revoke by the person acted for, or a relinquish by the holder, of a lease that has already ended is refused by name as already ended, carrying how, when and by whom it first ended, and records nothing; the first end record stays the only one (CONFORMANCE 7.6).

**Stories:**
- S19 (Person, Grants an agent provisioned under them access to an account) — As the person an agent acts for under a lease, I want to revoke the lease and see issuing stopped at once and the confirmation from the system behind shown pending until it comes, so that I know what has stopped and what is still unconfirmed.

### R6: Let a lease's holder relinquish it, recorded as its own act (CONFORMANCE 7.6)

Extends SECRETS-002 R7. Builds on SECRETS-003 R4, as R5 does. Blocked by: `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0 and `git cat-file -e origin/main:crates/lys-secrets/src/proxy.rs` exits 0. WHEN a lease's holder calls POST /leases/{lease_id}/relinquish, THE SYSTEM SHALL stop issuing under the lease at once, record the lease's end as a relinquish by the holder, a separate act from a revoke, by calling R5's `end_with_upstream_pending` in crates/lys-secrets/src/revoke.rs, and show upstream pending until the system behind confirms through R5's `deliver_upstream_ack`, moving through SECRETS-003 R4's revocation states exactly as a revoke does, since the credential at the system behind exists whoever ended the lease; GET /leases/{lease_id} SHALL then read ended_by `relinquish`, issuing `stopped`, and upstream `pending` until confirmed and `confirmed` after. IF the caller cannot discover the lease through R3's seam, which every caller but its person acted for and its holder cannot, THEN THE SYSTEM SHALL refuse with HTTP 404 and the error `lease_not_found`, naming nothing. IF the caller is the lease's holder and the lease has already ended, by a revoke, a relinquish or the end of its time window, THEN THE SYSTEM SHALL refuse with R5's HTTP 409 and error `lease_already_ended`, carrying the same way, instant and, for a revoke or a relinquish, who ended it, read from the first end record, and SHALL record nothing. IF the caller can discover the lease and is not its holder, THEN THE SYSTEM SHALL refuse with HTTP 403 and the error `relinquish_not_permitted`, naming the lease. IF the lease's person acted for calls revoke on a lease that has ended by a relinquish, THEN THE SYSTEM SHALL refuse with R5's HTTP 409 and error `lease_already_ended`, carrying the way `relinquish`, the instant of the relinquish and the holder who relinquished it, read from the first end record, and SHALL record nothing. THE SYSTEM SHALL NOT record a relinquish as a revoke, SHALL NOT write a second end record for a lease that has already ended, SHALL NOT let a timer change upstream `pending` to `confirmed`, and SHALL NOT let a refused relinquish stop issuing or record an end.

**Acceptance:**
- CONFORMANCE 7.6: agent_a, L1's holder, relinquishes L1 while the upstream test double of SECRETS-003 R4's proxy withholds its acknowledgement: the next use of L1 through the proxy is refused with the double's count of forwarded use requests unchanged, and GET /leases/L1 reads, in this order, ended_by `relinquish`, issuing `stopped`, and upstream `pending`, then upstream `confirmed` after the double delivers its acknowledgement once.
- CONFORMANCE 7.6: with lease L3 issued from a_org, acted for person_a and held by agent_a, relinquished by agent_a and the acknowledgement withheld, a test drives every other operation this brief exposes on L3 in turn, as 9 steps in this order: (1) a use of L3, (2) GET /leases/L3, (3) a second POST /leases/L3/relinquish by agent_a, (4) POST /leases/L3/revoke by person_a, (5) GET /secrets with no scope, (6) GET /secrets?scope=organisation, (7) GET /secrets?scope=team, (8) GET /secrets?scope=mine, (9) the injected clock advanced past the end of L3's own time window; after each of those 9 steps GET /leases/L3 reads upstream `pending`, and the test asserts it took 9 readings, one for each step. Then `deliver_upstream_ack` runs once for L3 and GET /leases/L3 reads upstream `confirmed`.
- CONFORMANCE 7.6: after agent_a's relinquish, L1's end record names the act `relinquish` and the caller agent_a, and L1 carries 0 revoke records.
- CONFORMANCE 7.6: person_a, L1's person acted for and not its holder, calls relinquish on L1: HTTP 403 with error `relinquish_not_permitted` naming L1, and L1 keeps issuing.
- CONFORMANCE 7.6: person_b, who sees a_org and is neither L1's person acted for nor its holder, calls relinquish on L1: HTTP 404 with error `lease_not_found` naming nothing, and L1 keeps issuing.
- CONFORMANCE 7.6: agent_a calls relinquish on L1 after person_a revoked it: HTTP 409 with error `lease_already_ended`, the body carries the way `revoke`, the instant of the revoke and person_a, and L1 carries exactly 1 end record, naming the act `revoke`.
- CONFORMANCE 7.6: agent_a calls relinquish on L3 a second time after relinquishing it: HTTP 409 with error `lease_already_ended`, the body carries the way `relinquish`, the instant of the first relinquish and agent_a, and L3 carries exactly 1 end record.
- CONFORMANCE 7.6: with L1 revoked by person_a and the acknowledgement still withheld, a test drives every other operation this brief exposes on L1 in turn, as 9 steps in this order: (1) a use of L1, (2) GET /leases/L1, (3) a second POST /leases/L1/revoke by person_a, (4) POST /leases/L1/relinquish by agent_a, (5) GET /secrets with no scope, (6) GET /secrets?scope=organisation, (7) GET /secrets?scope=team, (8) GET /secrets?scope=mine, (9) the injected clock advanced past the end of L1's own time window; after each of those 9 steps GET /leases/L1 reads upstream `pending`, and the test asserts it took 9 readings, one for each step. Then `deliver_upstream_ack` runs once for L1 and GET /leases/L1 reads upstream `confirmed`.
- With the 9-step test on the revoked lease L1 and the 9-step test on the relinquished lease L3 above, and R5's `compile_fail,E0616` doctest on the `upstream` field of `lys_secrets::Lease` passing under `cargo test --workspace --all-features`, upstream reads `confirmed` only after `deliver_upstream_ack` runs, on a revoked lease and on a relinquished one.
- CONFORMANCE 7.6: person_a calls revoke on lease L3 after agent_a relinquished it: HTTP 409 with error `lease_already_ended`, the body carries the way `relinquish`, the instant of the relinquish and agent_a, and L3 carries exactly 1 end record, naming the act `relinquish`.

**Files:**
- create: crates/lys-secrets/src/relinquish.rs
- create: crates/lys-secrets/tests/lease_relinquish.rs
- modify: crates/lys-secrets/src/api/leases.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/tests/support/mod.rs

**Checklist:**
- C30 — The person acted for under a lease may revoke it at any time: the revoke stops issuing at once, and GET /leases/{lease_id} reads upstream pending until the system behind confirms, with no timer confirming it (CONFORMANCE 7.6).
- C32 — A lease's holder may relinquish it, recorded as a relinquish and not as a revoke, which stops issuing at once and reads upstream pending until the system behind confirms.
- C37 — A revoke by the person acted for, or a relinquish by the holder, of a lease that has already ended is refused by name as already ended, carrying how, when and by whom it first ended, and records nothing; the first end record stays the only one (CONFORMANCE 7.6).

**Stories:**
- S19 (Person, Grants an agent provisioned under them access to an account) — As the person an agent acts for under a lease, I want to revoke the lease and see issuing stopped at once and the confirmation from the system behind shown pending until it comes, so that I know what has stopped and what is still unconfirmed.
- S20 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent holding a lease, I want to give it back by a relinquish recorded as my own act, so that ending my own access is never recorded as a revoke nobody asked for.

### R7: Make the seam ask SpiceDB through the step-2 evaluator

Extends SECRETS-002 R6 and R7. Blocked by: `git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/check.rs` exits 0 and `git cat-file -e origin/main:crates/lys-identity-server/src/routes.rs` exits 0, each keyed on the file and never on a brief id or commit text; and R3 landed. crates/lys-secrets declares, in crates/lys-secrets/src/permission.rs, a trait `PermissionCheck` with one method, which answers whether an identity may do an action on a secret or on a lease issued from it; crates/lys-secrets depends on no server crate, so crates/lys-secrets/Cargo.toml is not changed by this row; crates/lys-identity-server/Cargo.toml gains the dependency on lys-secrets, and Cargo.lock changes with it. crates/lys-identity-server implements `PermissionCheck` in crates/lys-identity-server/src/spicedb/check.rs with the step-2 brief's one evaluator in that file (ADR-001), and passes that implementation in where the routes of R4 are built, in crates/lys-identity-server/src/routes.rs. WHEN the step-2 SpiceDB evaluator exists, THE SYSTEM SHALL answer every check of R3's seam in crates/lys-secrets/src/access.rs by a call on the `PermissionCheck` it was given, with the relations the seam asks written in crates/lys-secrets/schema/secrets.zed. IF the `PermissionCheck` returns an error, THEN THE SYSTEM SHALL answer the request with HTTP 503 and the error `permission_engine_unavailable`, and answer no secret and change no lease. THE SYSTEM SHALL NOT answer any check from the record's scope, team and owner fields once this row lands, SHALL NOT fall back to those fields when the `PermissionCheck` returns an error, SHALL NOT call SpiceDB other than through the step-2 evaluator in check.rs, SHALL NOT add a second evaluator, SHALL NOT add a dependency from crates/lys-secrets on crates/lys-identity-server, and SHALL NOT add any check under crates/lys/src/identity/ (DIRECTORY-002 R4).

**Acceptance:**
- A test with a SpiceDB test double that implements `PermissionCheck` and answers no for person_a on a_org, which the record's fields allow: GET /secrets?scope=organisation as person_a answers exactly 1 secret, b_org, and the double records at least 1 check naming a_org.
- A test with the double answering yes to person_a's discovery of L1 and no to person_a's revoke of L1: HTTP 403 with error `revoke_not_permitted` and a body naming L1, and L1 keeps issuing.
- A test in crates/lys-secrets/tests/lease_revoke.rs substitutes a `PermissionCheck` double that answers no to every check and counts its calls, then calls POST /leases/L1/revoke as person_a: the double records exactly 1 check, the revoke answers HTTP 404 with error `lease_not_found` naming nothing, and the next use of L1 succeeds.
- A test with the double returning an error for every check: GET /secrets with no scope as person_a answers HTTP 503 with error `permission_engine_unavailable`, and the body contains 0 secrets.
- `cargo tree -p lys-secrets --all-features -e normal` prints 0 lines containing `lys-identity-server`.
- The diff of this row lists 0 paths under crates/lys/src/identity/.

**Files:**
- create: crates/lys-secrets/src/permission.rs
- modify: crates/lys-secrets/src/access.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/schema/secrets.zed
- modify: crates/lys-secrets/tests/support/mod.rs
- modify: crates/lys-secrets/tests/lease_revoke.rs
- modify: crates/lys-identity-server/src/spicedb/check.rs
- modify: crates/lys-identity-server/Cargo.toml
- modify: Cargo.lock
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C36 — Every secret visibility and lease revoke check is a call on one seam, answering from the record's scope, team and owner fields until the step-2 SpiceDB evaluator exists, and asking SpiceDB through it once it does.

## Boundaries

- No row is built before the lead signs this brief off on its card.
- The host is not rewritten in the secrets design. Its Intention reads "Every identity, person or agent, uses credentials through a short-lived handle the door swaps for the real credential, so no credential ever reaches an agent and revoking is instant." and its Solution reads "A broker in Rust inside the door: an encrypted store of real credentials, handles bound to identities, a proxy that checks SpiceDB, swaps the handle, forwards the call and writes one audit line, rotation across several accounts under one handle, OAuth refresh at the proxy, sealed records tagged in SpiceDB, and leases counted by uses, time window and spend." Both place the broker in the door, while this brief builds in crates/lys-secrets on the host decision SECRETS-003 carries: that conflict is an open finding against SECRETS-003, not against this brief. The two sentences stand as main's text and change only with SECRETS-003's own documents. R2 to R6 depend on SECRETS-003 landing, checked from a clone of lys by `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exiting 0, and R5 and R6 also by `git cat-file -e origin/main:crates/lys-secrets/src/proxy.rs` exiting 0.
- Only the server's answers are built. The lease screen with its Revoke control, and the secrets list screen with its scope control of All, Organisation, Team and Mine, are neighbours outside this brief. The rule the neighbour is held to, with no requirement and no test here: the browser shows what the server answered and nothing else.
- No code and no test grants a secret's owner a revoke of a lease they are not acted for under while ADR-095 is proposed; the set of who may revoke beyond the person acted for stays empty.
- The refusals cover only the direct act of revoking a lease: withdrawal of an upstream grant (ADR-003, SEC_REVOKE_CHAIN) and emergency stop (CONFORMANCE row 3.4) end a lease as their own acts and are not changed.
- SECRETS-002 R8's in-flight cancellation rule stays open; no row decides it.
- SECRETS-002's JSON and rendered markdown are not edited.
- No SpiceDB permission check, relationship write or schema write is added under crates/lys/src/identity/ (DIRECTORY-002 R4); every visibility and revoke check is a call on R3's seam.
- Team ids come only from the group claims `team_ids` reads; no team model is built here.
- The unscoped list is the list with no scope, never a fourth scope value.
- No cambium file is edited, and nothing makes Cambium or any engine a required runtime (ADR-004).
- The token revolver is not changed (ADR-002).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, each clean, and the test run reports 0 failed and 0 ignored.
- From the repository root: sh scripts/design/gate.sh exits 0.
- Before R2 to R6 are built, `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0; before R5 and R6 are built, `git cat-file -e origin/main:crates/lys-secrets/src/proxy.rs` exits 0 as well; before R7 is built, `git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/check.rs` exits 0 and `git cat-file -e origin/main:crates/lys-identity-server/src/routes.rs` exits 0.
- `rg -c 'conformance_7_6' crates/lys-secrets/tests/lease_revoke.rs crates/lys-secrets/tests/lease_relinquish.rs` prints at least 1 for each file, and `rg -c 'conformance_7_8' crates/lys-secrets/tests/secret_list.rs` prints at least 1.
- `git diff --name-only` over this brief's commits lists no path under crates/lys/src/identity/, and neither docs/design/secrets/briefs/SECRETS-002.json nor docs/design/secrets/briefs/SECRETS-002.md.

