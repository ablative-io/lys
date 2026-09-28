# directory — what was asked, what it means, and what was written

## The words, as they were typed

Write the brief for this card in the directory cluster (docs/design/directory/briefs, next free id), building lys decision DP26, ruled and not built: revoking is an append to the log, and the live set of certificates is folded from it; the LeafStore exposes no delete and no rewrite, so revocation is a leaf, never a removal. Done when a revoked certificate fails verification and its history still verifies. Hold to DIRECTORY-006 R4 (revocation, inherited expiry and current permission decisions) and to lys-log-store's append-only contract. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after Tom or the lead signs it off on the card.

Rulings of the lead, Archie, given on 27 September 2026 to the run 82386e20-66d3-480c-b01b-438b63050fa2 in answer to its rounds. That run took every answer and then failed before writing, when the Argus gate refused every tool call of its session. They are settled here, and the author reopens none of them.

The certificate. To keep DP26's rule that the revocable unit and the claim unit are the same, the brief states that a certificate carries one capability claim. The typed-claim card (hhAN8h77) issues one claim per certificate, and the fold refuses to treat a multi-claim certificate as partially revoked. Revoking a claim means revoking its certificate and issuing a new one without it.

The log's inclusion and consistency proofs and the issuance record always verify. An attestation by the revoked certificate's key verifies if and only if its own log entry precedes the revocation leaf. The log order is the evidence of time, so a forgery made after compromise cannot be placed before the revocation. An attestation with no log entry before the revocation leaf fails, and the refusal names the revocation leaf.

The issuing CA key only. A person responsible under ADR-003, or an administrator, revokes by asking the directory, and the directory asks the CA to append. The fold refuses a revocation leaf not signed by the certificate's issuing CA, and the brief carries a refusal leg for a leaf signed by any other key.

Permanent. No leaf reinstates a revoked certificate. The fold refuses a reinstatement leaf by name, and the way back is a new certificate. The brief carries a leg for the last-wins hazard documented at delegation/artifact.rs:370-373.

Not in lys-core. Revocation stays a consumer-side non-goal there, and no lys-core code or published format changes. The fold lives in lys-identity, the consumer crate DIRECTORY-003 creates, so this brief is blocked on DIRECTORY-003 having landed, checked by the crate existing on main. CN1 binds DIRECTORY-001 only, as ruled on the SpiceDB brief this morning. The appended line comes from whichever brief lands first.

Beside DIRECTORY-006 R4, not discharging it. Certificate revocation and grant revocation are separate folds. DP26's shape is an input to the C25 review but does not close it. C25 stays open, and this brief names DP26 as the proposal the review reads.

The existing `lys ca verify` and the published verify_certificate_chain keep their meaning and keep passing with no log, because lys-core 0.2.0's API is frozen. Revocation is checked by a new form that takes the log (size N and tolerance as explicit inputs). The help text of both forms says plainly that verification without a log does not check revocation.

The library form only. The command goes to a later unit as a separate unpublished binary, and the published lys binary does not depend on lys-identity. Also a correction to the id. This draft's DIRECTORY-010 clashes with draft/directory/9bb544d1, which holds it. This brief is DIRECTORY-012, because DIRECTORY-011 goes to 858c0e71. Re-check with git ls-remote before the write.

Amend the non-goal narrowly. Append one line to the step-2 non-goal that names this brief's revocation fold and history check as step-2 work brought forward, and leave the rest of the non-goal as written. Remove the entry for it from blocked_by.

Refuse that revocation. The fold refuses a revocation leaf whose certificate has no earlier issuance leaf, naming it revocation_before_issuance. The later issuance is then valid. A revocation always names a certificate the log already holds.

No. An attestation by a certificate that is not revoked needs no log entry of its own. The history check verifies the signature and that no revocation leaf in the log covers the certificate, and answers not revoked. Log entries are required only for revocations, as ruled.

Yes. The rule is the next free id after main's highest and every open brief/* and draft/* branch's, the same rule that sent the brief id to DIRECTORY-012. Renumber to the next free across all of them, RM-020, ADR-032, C51 onward and S23 onward, unless a fresh check finds higher. Run git ls-remote on the lys origin immediately before writing, and fetch and read every brief/* and draft/* head. If any branch now holds one of these ids, take the next free past it. Record the ids checked and the branch heads read in the dev record. Every reference in the brief, the ledger and the rendered files follows the new numbers. Answered by Archie, lead for the identity line.

Move to DIRECTORY-013. From here on, the brief that has already written an id keeps it, and the later one takes the next free, with no renumbering back and forth. 858c0e71 keeps DIRECTORY-012 and RM-020 as its draft at efe4238 has them, and e5171dbd keeps DIRECTORY-011. This brief takes DIRECTORY-013, and its roadmap row, decisions, checklist and story ids are each the next free past every id on main and on every open brief/* and draft/* branch. Check them with git ls-remote and a fetch of every head immediately before writing, and record the heads read in the dev record. Every reference follows the new numbers. Answered by Archie, lead for the identity line.

## What the survey found, and its angles

Write the next directory brief, DIRECTORY-013, to build lys decision DP26. Revoking a certificate becomes a leaf appended to the log, signed by the certificate's issuing CA. The live set of certificates is folded from that log in the consumer crate lys-identity. A new verification form takes the log at size N with a tolerance: it refuses a revoked certificate and names the revocation leaf, while the certificate's proofs, its issuance record and every attestation logged before the revocation still verify. The lead's rulings of 27 September settle the claim unit, the signer, permanence, placement and ids, and the brief records them without reopening any. The brief is documents only, sits beside DIRECTORY-006 R4 rather than discharging it, and is blocked on DIRECTORY-003 landing the lys-identity crate.

### What the tree holds

- `docs/design/directory/briefs/` — The brief lands here. Main holds DIRECTORY-001 to -006 and -008 (8 briefs). Open drafts hold -007 (dc7fb926), -010 (9bb544d1, c7d95bf6), -011 (e5171dbd), and -012 on both 858c0e71 (typed claim) and 82386e20 (this card's earlier revocation draft), so this brief is DIRECTORY-013.
- `docs/design/directory/DESIGN.md:63 (design.json non_goals)` — The step-2 non-goal ('capability certificates, arbitrary grants and their enforcement…'). The lead ruled a one-line append naming this brief's revocation fold and history check as step-2 work brought forward, with the rest left as written. 9bb544d1 (DIRECTORY-010) appends its own line to the same non-goal.
- `docs/design/directory/DESIGN.md:185 (CN1)` — 'Documents only: nothing outside docs/design/directory/ and docs/design/decisions.json is created or modified' contradicts a brief whose rows create crates/lys-identity files. The lead ruled that CN1 binds DIRECTORY-001 only, and the appended line comes from whichever brief lands first. 9bb544d1 and 82386e20 each already carry a variant of that line.
- `docs/design/directory/DESIGN.md:181 (inventory row for docs/design/decisions.json)` — It says decisions.json is 'read here, never changed by a document row' and 'ADR-001 to ADR-018 at main'. A new ADR from this brief contradicts both halves.
- `docs/design/directory/DESIGN.md:191 (CN7) and :195 (CN11)` — CN7 caps IDENTITY-001 at 48 implementer hours. CN11 puts live capability policy and its enforcement in step 2. This brief adds hours to the lane and brings a step-2 piece forward.
- `docs/design/directory/briefs/DIRECTORY-006.json R4 and CHECKLIST.md C25` — R4 handles grant revocation, inherited expiry and freshness (GRANT_REVOKE, GRANT_FRESHNESS). C25's freshness mechanism is reviewed before dispatch. This brief sits beside R4, names DP26 as the proposal the C25 review reads, and leaves C25 open.
- `docs/design/directory/briefs/DIRECTORY-003.json` — It creates crates/lys-identity/ (R1, R2, R4, R5) and commits identity events through lys-log-store under IDENTITY-EVENTS.md. Its wall is a directory, so CN9 needs an exact file manifest for this brief's new files. The crate does not exist on main, which is the blocker.
- `crates/lys-log-store/src/store.rs:1-60,120-191` — The append-only contract. LeafStore has six methods (origin, extent, leaf, put_leaf, pin, pinned), with no fork, merge, delete, truncate or rewrite, and put_leaf refuses any index but extent(). Revocation must be a new leaf through put_leaf.
- `crates/lys-core/src/ca/authority.rs:94-99,205,309-314` — issue_certificate takes Vec<CustomExtension>. verify_certificate_chain(cert_der, issuer_public_key) takes no log, is published in lys-core 0.2.0, and must keep its meaning and pass unchanged.
- `crates/lys/src/cli.rs:277-296 and crates/lys/src/commands/ca.rs:55-65,179,299-323` — The `lys ca verify` help text, which gains the sentence that verification without a log does not check revocation. The CLI currently embeds one opaque claims blob under a single OID, and its output says 'capability claims', plural.
- `crates/lys-core/src/delegation/artifact.rs:360-379` — The last-wins replay hazard ruled as a refusal leg: a verbatim copy of an earlier issuance leaf appended after a revocation must not reinstate the certificate.
- `crates/lys-core/src/error.rs:60-70` — TrustError::CertificateRevocation is 'reserved for consumer-side operations'. Revocation stays a consumer-side non-goal in lys-core, so no lys-core change.
- `docs/design/lys-anchor/DECISIONS.md:400-425 (DP26)` — The ruling being built: revocation is an append and the live set a fold. The revocable unit is the claim unit. Freshness tolerance is an input and N is in the answer type. No default tolerance ships.
- `docs/design/identity/STATEMENT-2026-09-22.md:79,126` — Records DP26 as 'ruled, not built' for lys; this brief is what builds it.
- `docs/design/roadmap.json, docs/design/decisions.json, docs/design/directory/{checklist,stories,design}.json` — These gain the roadmap row, the proposed ADR, checklist items C65 onward, stories S31 onward and structure rows. All are rendered by render-cluster.py and checked by scripts/design/gate.sh.
- `scripts/design/gate.sh` — The design leg: validate.py on decisions.json, project.json and each cluster, check-coverage.py, then a render-cluster.py re-render compared byte for byte with the committed markdown.

### What was already decided

- DP26 — Revocation is an append and the live set is a fold. The revocable unit equals the claim unit. Tolerance is an input and N is in the answer type. No default tolerance. A derived view may refuse, never permit, on its own authority.
- ADR-003 — Everything is pegged to a human authority. The person responsible for an agent, or an administrator, asks the directory to revoke, and the directory asks the CA to append.
- ADR-008 — An agent's file shows its lys certificate with the signed receipts of changes made to it. A revocation is such a change, but the screen is a later unit.
- ADR-011 — Retired is permanent and a new identity is made instead. The certificate-permanence ruling mirrors it: the way back is a new certificate.
- DIRECTORY-003 — Creates crates/lys-identity and commits signed identity events through lys-log-store. This brief is blocked on it landing.
- DIRECTORY-006 R4 — Grant revocation, expiry and freshness. Certificate revocation is a separate fold beside it, not a discharge of it.
- C25 — Revocation stops derived authority, and the freshness mechanism is reviewed before dispatch. It stays open, and DP26 is named as the proposal that review reads.
- CN1 — Documents only. Ruled to bind DIRECTORY-001 only; the appended line comes from whichever brief lands first.
- CN9 — A directory wall for a new module needs an exact file manifest reviewed before its row starts.
- CN11 — Step 1 is records, sign-in and minimum audit. Live capability policy and enforcement are step 2's.
- Step-2 non-goal (DESIGN.md:63) — Capability certificates are out of step 1. Amended narrowly by one appended line.
- lys-log-store append-only contract — No fork, merge, delete or rewrite, and put_leaf only at extent.
- 858c0e71 DIRECTORY-012 (typed-claim card hhAN8h77) — lys/agent-capability/v1 typed claim, ratification pending (its ADR-033). It issues one claim per certificate.

### What was measured

- lys-identity crate on main (7b53625, which is origin/main's head): absent. crates/ holds lys, lys-anchor, lys-anchor-cli, lys-core, lys-home, lys-log-store (6 crates)
- LeafStore trait methods: 6 (origin, extent, leaf, put_leaf, pinned, pin); 0 delete, rewrite, fork or merge
- Open brief/* and draft/* heads on origin (git ls-remote, this session): 66 of 129 refs; 11 heads present locally, 55 missing locally and read through the GitHub contents API
- Highest DIRECTORY brief id across main and all heads: DIRECTORY-012 (858c0e71 at 9244584, and 82386e20 at 6ed93c8). Next free: DIRECTORY-013
- Highest RM id across main and all heads: RM-024 (draft/lys-gate/7dc46a76). RM-023 on draft/lys-core/95377829, RM-022 on 82386e20. Main's highest is RM-016. Next free: RM-025
- Highest ADR id across main and all heads: ADR-038 (on both draft/lys-gate/7dc46a76 and draft/lys-core/33d9ce6d). Main's highest is ADR-018. Next free: ADR-039
- Highest directory checklist id across main and all heads: C64 (858c0e71). Main's highest is C30. Next free: C65
- Highest directory story id across main and all heads: S30 (858c0e71). Main's highest is S12. Next free: S31
- Ids the words named as the floor: RM-020, ADR-032, C51, S23. The fresh check found higher on every one.
- This card's earlier draft branch: draft/directory/82386e20 at 6ed93c8 holds a 7-requirement revocation brief as DIRECTORY-012 (RM-022, ADR-037, C51-C57, S24-S27), about 50.5 KB rendered
- Directory cluster sizes on main: DESIGN.md 196 lines, design.json 770, CHECKLIST.md 49, checklist.json 185, stories.json 83, USER-STORIES.md 35
- lys-core CA source sizes: authority.rs 455 lines, certificate.rs 290, extensions.rs 97
- delegation/artifact.rs: 512 lines. The replay steps sit at 367-373.
- Capability-claim extensions the CLI embeds per certificate: 1 opaque blob under one OID (commands/ca.rs:179). Claims inside it are not typed on main.

### What it means for the other projects

- aion — The brief goes through brief_card, then sign-off on the card, then card_build_v3, src_pr and src_land. Sign-off by Tom or the lead gates card_build_v3. The earlier run 82386e20 left a stale draft branch holding DIRECTORY-012, which the chain should close as superseded.
- cambium — The card lives on the Cambium board beside the typed-claim card hhAN8h77 (858c0e71's DIRECTORY-012). This brief depends on that card's one-claim-per-certificate rule landing before a real certificate is multi-claim-safe.
- argus — The Argus gate refused every tool call of run 82386e20 and killed it before writing. The author's session must not hit the same refusal. If it does, that is a chain defect to name.
- method — validate.py, check-coverage.py, render-cluster.py and the schemas under scripts/design/ come from the design-system method. The brief must validate against brief.schema.json as vendored.

### The decisions it stands on

- ADR-003 (honour) — A person responsible for the agent, or an administrator, revokes by asking the directory, and only the issuing CA appends.
- ADR-008 (honour) — A revocation is a signed change to the certificate. The agent's file can later show it, and this brief does not build the screen.
- ADR-011 (honour) — Permanence mirrors retired: no reinstatement, and a new certificate instead.
- ADR-004 (honour) — lys-identity stands alone, and the published lys binary gains no dependency on it.
-  (new) — ADR-039 (proposed): certificate revocation is an issuing-CA-signed append to the log, folded in lys-identity, with one claim per certificate. It is permanent, log order is the evidence of time, and N and the tolerance are explicit inputs. It records DP26's build in the project ledger (subject to the DESIGN.md:181 question).
-  (new) — CN1 gains the ruled line binding it to DIRECTORY-001 only, unless 9bb544d1's line lands first. The step-2 non-goal gains one line naming this brief's work as brought forward.

### What it requires

- docs/design/directory/briefs/DIRECTORY-013.json and .md exist and validate, and scripts/design/gate.sh exits 0.
- The brief uses DIRECTORY-013, RM-025, ADR-039, C65 onward and S31 onward, or the next free past any higher id a re-check immediately before writing finds. The dev record lists the ls-remote heads read.
- depends_on names DIRECTORY-003, and blocked_by carries a check that crates/lys-identity exists on origin/main.
- blocked_by names sign-off by Tom or the lead on the card before card_build_v3, with no entry for the non-goal amendment.
- The brief states that a certificate carries exactly one capability claim, and the fold never treats a multi-claim certificate as partially revoked.
- A revoked certificate fails the new log-taking verification, and the refusal names its revocation leaf.
- The inclusion and consistency proofs and the issuance record of a revoked certificate verify.
- An attestation whose log entry precedes the revocation leaf verifies. One with no earlier entry fails, naming the revocation leaf.
- An attestation by an unrevoked certificate verifies with no log entry of its own.
- Refusal legs exist for a revocation leaf signed by a key other than the issuing CA's, a reinstatement leaf (refused by name), a verbatim replayed issuance leaf after a revocation (the last-wins hazard, artifact.rs:367-373), and revocation_before_issuance, where the later issuance is valid.
- N and the tolerance are explicit inputs to the new form, with no default tolerance.
- Every append goes through LeafStore::put_leaf at extent, and no row adds delete, rewrite, fork or merge.
- verify_certificate_chain and `lys ca verify` pass unchanged with no log, and the help text says verification without a log does not check revocation.
- The step-2 non-goal gains one appended line and the rest is unchanged. CN1 carries the ruled line unless another brief landed it first.
- The brief names DP26 as the proposal the C25 review reads, and C25 stays open.
- A constructed-attack review of the leaf formats is a gate before any code row.

### What must not change

- No lys-core code or published wire format changes, and lys-core 0.2.0's API stays frozen.
- The published lys binary gains no dependency on lys-identity, and no CLI revocation command is added in this unit.
- The LeafStore trait gains no delete, rewrite, fork or merge.
- DIRECTORY-006 R4 and C25 are not discharged or closed.
- The rest of the step-2 non-goal stays as written.
- The rulings of 27 September are not reopened.
- 858c0e71 keeps DIRECTORY-012 and RM-020, and e5171dbd keeps DIRECTORY-011. There is no renumbering back.
- The IDENTITY-001 files do not change.
- The brief does not rewrite the design around an open or contradicted sentence; it asks the lead.

### What we must put in place first

- DIRECTORY-003 landed on main with crates/lys-identity; it is absent at 7b53625. This gates code rows only, not writing the brief.
- A git ls-remote and a read of every brief/* and draft/* head immediately before writing. 55 of 66 heads are not present locally.

### The risks

- An id collision if another draft takes RM-025, ADR-039, C65 or S31 between the check and the write. ADR-038 is already duplicated across two branches.
- Merge conflicts on DESIGN.md's step-2 non-goal and CN1 lines with 9bb544d1 (DIRECTORY-010) and the stale 82386e20 draft.
- The one-claim rule depends on the unratified typed-claim format (858c0e71's lys/agent-capability/v1). On main, certificates carry an opaque claims blob.
- The leaf formats freeze once signed. Skipping the adversarial review would freeze a replay or cross-issuer confusion bug.
- If the tolerance or N can be ignored by a caller, revocation fails open, which is the obligation-on-caller shape DP26 forbids.
- The stale 82386e20 DIRECTORY-012 revocation brief could be mistaken for this brief or landed by accident.
- The Argus gate refusing tool calls could kill the author's run again before writing.

### Still open

- Does 'the help text of both forms' include the rustdoc of lys-core's published verify_certificate_chain, which would be a lys-core source change, or only `lys ca verify`'s CLI help in crates/lys, with the library caveat carried in lys-identity's docs? The sentence of the words it stands on: "The help text of both forms says plainly that verification without a log does not check revocation.". Why only the lead can settle it: The 'existing form' is both the `lys ca verify` command (crates/lys/src/cli.rs:277) and verify_certificate_chain (crates/lys-core/src/ca/authority.rs:309-314). Changing the second contradicts 'no lys-core code or published format changes'. A library consumer reading the published docs sees the caveat under one answer and not under the other.
- DESIGN.md:181 says decisions.json is 'read here, never changed by a document row' and lists 'ADR-001 to ADR-018 at main'. May this brief add ADR-039 and update that inventory sentence, or should DP26 stay out of the project ledger and be cited from docs/design/lys-anchor/DECISIONS.md? The sentence of the words it stands on: "If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it.". Why only the lead can settle it: docs/design/directory/DESIGN.md:181 conflicts with a brief that writes a new ADR, and the renumbering ruling names an ADR id. Whether the decision appears in the project ledger is what a reader of decisions.json sees.
- CN7 caps IDENTITY-001 at 48 implementer hours. Does DIRECTORY-013's estimate count against that ceiling, or sit outside it the way IDENTITY-002's 4 hours do? The sentence of the words it stands on: "If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it.". Why only the lead can settle it: docs/design/directory/DESIGN.md:191 says an overrun is reported and Waffles takes any ceiling change to Tom. Adding a brought-forward step-2 row to this lane either triggers that report or needs a stated exemption.
- CN11 says live capability policy and its enforcement are step 2's. Is the one-line amendment to the step-2 non-goal enough, or does CN11 need the same kind of appended line? The sentence of the words it stands on: "If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it.". Why only the lead can settle it: docs/design/directory/DESIGN.md:195 still reads as step-2-only after the non-goal amendment, so a reviewer could refuse the brief under CN11 even though the non-goal allows it.

### The units beyond the first

- A revocation command as a separate unpublished binary over lys-identity — Ruled to a later unit, so that the published lys binary never depends on lys-identity.
- Show a revoked certificate and its revocation receipt on the agent's file (ADR-008) — A screen change in surface/identity that depends on DIRECTORY-005 and this fold.
- The C25 freshness review, which reads DP26 as its proposal — Grant revocation freshness is a separate fold and decision. This brief only names DP26 as that review's input.
- Close the stale draft/directory/82386e20 branch as superseded by DIRECTORY-013 — It holds a colliding DIRECTORY-012 revocation brief and should not survive as an open draft.

### The smallest complete shape

One documents-only commit under docs/design. DIRECTORY-013.json and .md hold the contract row and the code rows: leaves, fold, append, log-taking verify, history check, and help text. Alongside them: the RM-025 roadmap row, the ADR-039 proposal, directory checklist C65 onward and stories S31 onward, structure rows for the named lys-identity files, the one-line step-2 non-goal append, the CN1 line if it has not landed, and re-rendered markdown. All pass scripts/design/gate.sh, and the dev record lists the heads read.

## The roadmap row

- **RM-025** — Revoke a certificate by appending a leaf and fold the live set from the log (feature, idea)
- Summary: Builds DP26 for certificates: revoking is one leaf appended to an append-only certificate log through lys-log-store, never a deletion or rewrite, and the live set is folded from the log. A revoked certificate fails revocation-aware verification, which takes the size N and a tolerance as explicit inputs with no default and names the revocation leaf, while its log proofs, its issuance record and the attestations logged before its revocation still verify, and an attestation by a certificate not revoked needs no log entry. A certificate carries one claim and is the revocable unit, only its issuing authority's key revokes it, a revocation is permanent, and a revocation naming a certificate with no earlier issuance is refused (ADR-039). The code lives in lys-identity; lys-core and every published format are unchanged, and lys ca verify keeps its meaning with help saying it does not check revocation. It stands beside DIRECTORY-006 R4 and leaves C25 open. DIRECTORY-013 carries it in seven rows.
- Asked by: tom on 2026-09-27T10:40:00+10:00
- Context: The certificate revocation card in the directory cluster, surveyed against main 7b53625. The lead for the identity line answered its rounds on 27 September 2026, and those answers are carried as settled in DIRECTORY-013 and ADR-039: a certificate carries one capability claim; history is the log's proofs, the issuance record, and attestations whose entries precede the revocation leaf, and an attestation by a certificate not revoked needs no entry; only the issuing authority's key revokes; revocation is permanent, and a revocation with no earlier issuance of its certificate is refused as revocation_before_issuance; the fold lives in lys-identity and the code rows block on DIRECTORY-003, with CN1 binding DIRECTORY-001 only; the brief stands beside DIRECTORY-006 R4 and C25 stays open with DP26 as the proposal its review reads; the published no-log verification keeps its meaning and a new library form taking the log checks revocation, its command line a later separate unpublished binary; the step-2 non-goal and CN11 each gain one appended line naming this brief's fold and history check as step-2 work brought forward; the decision enters the project ledger as ADR-039 with one line appended beneath DESIGN.md's decisions.json inventory row; the estimate sits outside CN7's ceiling; lys-core's rustdoc is untouched and the gap is answered by a documentation-only lys-core release, its own card.
- Quote: Write the brief for this card in the directory cluster (docs/design/directory/briefs, next free id), building lys decision DP26, ruled and not built: revoking is an append to the log, and the live set of certificates is folded from it; the LeafStore exposes no delete and no rewrite, so revocation is a leaf, never a removal. Done when a revoked certificate fails verification and its history still verifies. Hold to DIRECTORY-006 R4 (revocation, inherited expiry and current permission decisions) and to lys-log-store's append-only contract. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after Tom or the lead signs it off on the card.
- Cluster: directory; briefs: DIRECTORY-013
- Notes: Ids, under the rule that a brief's ids are each the next free past every id on main and on every open brief/ and draft/ branch, and that an id already written by another brief is kept by it: checked with git ls-remote origin and a fetch of all 68 brief/ and draft/ heads immediately before this write, the 68th being this card's own draft/directory/1e30a4cb at 1bdf660, re-checked for this round, against main 7b53625. Highest found: DIRECTORY-012 (draft/directory/858c0e71 at 9244584, and draft/directory/82386e20 at 6ed93c8), RM-024 (draft/lys-gate/7dc46a76 at c58c941), ADR-038 (draft/lys-gate/7dc46a76 and draft/lys-core/33d9ce6d at df7e5b0), C64 and S30 (draft/directory/858c0e71); so this brief is DIRECTORY-013, this row RM-025, its decision ADR-039, its checklist items C65 to C71 and its stories S31 to S34. Heads read: brief/decisions-words/0969edbd 36f87e5, brief/directory/63bd2f2e c6615d2, brief/directory/82882bd5-hand 0806037, brief/directory/d70a418b 3c1a3b9, brief/directory/lifecycle-hand 28e4603, brief/home/06103633 d857f0a, brief/home/31aa723f c6a7c8b, brief/home/54e28af6 af996bd, brief/home/5898fcb0 27a0088, brief/home/7d3f3efb db97e15, brief/home/99985c4a 2eb9c0a, brief/home/9e701467 d6e0898, brief/home/b9d7f174 1482864, brief/home/c322f3e6 7678280, brief/home/d388a9f7 3d8f685, brief/home/de72f0a4 978dd98, brief/home/e35a7157 8620ba6, brief/secrets/309540a4 c7553a7, draft/canon-words/e75b3567 8a8de27, draft/decisions-words/0969edbd 7bfe544, draft/directory/073049e0 f889f75, draft/directory/7e9506d8 a96eb99, draft/directory/82386e20 6ed93c8, draft/directory/82882bd5 c7d80e5, draft/directory/858c0e71 9244584, draft/directory/9bb544d1 13167ab, draft/directory/c7d95bf6 e12b825, draft/directory/dc7fb926 4a25814, draft/directory/e5171dbd fe72451, draft/home/06103633 1ce28e7, draft/home/0e229c29 9108ac6, draft/home/0eea471a e84e91a, draft/home/0efbd7ed 317d186, draft/home/128bb392 c617349, draft/home/24724e9e 0fe5698, draft/home/2bfc117e 2ad97ed, draft/home/31aa723f 1bf8bf3, draft/home/382db8b8 550dce4, draft/home/4c5d0ab9 1e49088, draft/home/54e28af6 2f255df, draft/home/5898fcb0 d470284, draft/home/65e82438 2753732, draft/home/7657952c 69fcb20, draft/home/7d3f3efb 0bb4bfe, draft/home/9635a408 1ff48e2, draft/home/99985c4a 7ab289d, draft/home/a13070d7 8813fd8, draft/home/a62aacfe 273d54e, draft/home/a6d7b86a 1ec12e2, draft/home/b69a5124 729e245 (moved from 348b95e; re-read, no id past the ones above), draft/home/b9d7f174 6212912, draft/home/c322f3e6 21fbd36, draft/home/d26d0f3b 78da410 (moved to deed152 during the write, re-read, highest RM-017 and ADR-019), draft/home/d388a9f7 f1444a7, draft/home/d5a29dc7 727869e, draft/home/de72f0a4 aa707aa, draft/home/e35a7157 e1aac1d, draft/lys-core/04213a71 65fad48, draft/lys-core/33d9ce6d df7e5b0, draft/lys-core/95377829 2d8336c, draft/lys-core/a31929b9 61e146f, draft/lys-gate/7dc46a76 c58c941, draft/lys-log-store/a1163a79 d7683e3, draft/rauthy-rebase/4365d91a 735b517, draft/secrets/309540a4 b7cd152, draft/secrets/604f95d0 b45bff6, draft/secrets/ad15e87e 66f70dd. Re-checked for the round that answered the lead's questions, with git ls-remote origin (82 brief/ and draft/ heads) and a fetch of every head added or moved since the check above: brief/home/d26d0f3b 1004c58, draft/home/4c46aabc 17c46c7, draft/directory/28d15c0d 252af42, draft/rauthy-rebase/be0c979d 3a3fed2, brief/directory/28d15c0d 3c517a0, draft/home/0e229c29 639e0e6, draft/home/b69a5124 7d34a4f, draft/roots/3fa3f51a 868eb04, brief/home/40b37c5f 8c6f5bb, brief/home/a3186728 9a5e50e, brief/home/0e229c29 9da214e, draft/home/533af05d ce6cedf, draft/lys-log-store/e8440f97 d3a819b, draft/home/d26d0f3b deed152, draft/directory/b38766d2 f45038c, brief/lys-anchor/6b1c7df6 f8a4712, brief/home/ea6ad356 f990cf5, draft/lys-gate/7dc46a76 fa9db04, and this card's own draft/directory/1e30a4cb 7baf039. None holds DIRECTORY-013, RM-025, ADR-039, C65 to C71 or S31 to S34. RM-026 to RM-029 and ADR-040 to ADR-042 now exist on later drafts, written after this brief took its ids, so this brief keeps its ids under the rule that the brief which wrote an id first keeps it. Superseded, recorded here only and referenced by no row of the brief: draft/directory/82386e20 at 6ed93c8 holds this card's earlier revocation draft as DIRECTORY-012 (RM-022, ADR-037, C51 to C57, S24 to S27); DIRECTORY-013 replaces it. Further units, not written: The revocation request path, blocked on DIRECTORY-006 R4 and on the directory server; A revocation command as a separate unpublished binary over lys-identity; Show a revoked certificate and its revocation receipt on the agent's file (ADR-008); The C25 freshness review, which reads DP26 as its proposal; A documentation-only lys-core release saying verification without a log does not check revocation; A lys-core card for the self-signed false positive of verify_certificate_chain_at; Close the stale draft/directory/82386e20 branch as superseded by DIRECTORY-013.

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
- ADR-039 — A certificate carrying one claim is revoked whole, by its issuing authority's key, permanently, by a leaf folded in lys-identity — A certificate carries one capability claim, so the revocable unit and the claim unit are the same; revoking a claim is revoking its certificate and issuing a new one without it, and the fold never treats a certificate carrying several claims as partially revoked. Only the issuing authority's key signs a revocation leaf; a person responsible for the agent under ADR-003, or an administrator, revokes by asking the directory, which asks the authority to append, and the fold refuses a revocation signed by any other key. A revocation is permanent: no leaf reinstates a certificate, the fold refuses a reinstatement leaf by name, a verbatim replay of an earlier issuance leaf included, and the way back is a new certificate. A revocation always names a certificate the log already holds: the fold refuses a revocation leaf with no earlier issuance leaf for its certificate as revocation_before_issuance, and a later issuance of that certificate is valid. A revoked certificate's inclusion and consistency proofs and its issuance record always verify, and an attestation by its key verifies if and only if its own log entry precedes the revocation leaf, the log order being the evidence of time; an attestation with no entry before the revocation leaf fails, naming the revocation leaf; an attestation by a certificate that is not revoked needs no log entry of its own. The fold and the revocation-aware verification live in lys-identity; lys-core keeps revocation a consumer-side non-goal, and its published verify_certificate_chain and the lys ca verify command keep their meaning and keep passing with no log, while a new library form taking the log, the size N and the tolerance as explicit inputs checks revocation; its command line is a later, separate unpublished binary, and the published lys binary does not depend on lys-identity. Certificate revocation and grant revocation are separate folds: this stands beside DIRECTORY-006 R4 and does not discharge it, and C25 stays open with DP26 as the proposal its review reads. Rejected: revoking claims one by one inside a certificate, letting any key but the issuing authority's sign a revocation, a later leaf reinstating a certificate, letting every attestation made before a revocation stand whatever its log position, a revocation standing for a certificate the log does not hold, building the fold in lys-core, and changing what the published no-log verification means.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.
- A revoked certificate fails verification against the certificate log while its history still verifies, with revocation an appended leaf and the live set folded from the log (DIRECTORY-013, ADR-039).

## Non-Goals

- Rows 06 (connect Cambium) and 07 (gate, install and demonstrate the release) — They change the Cambium repository and depend on IDENTITY-002, the upstream release rebase; they need a Cambium cluster or an agreed cross-repository arrangement first.
- Row 03's changes inside the Rauthy fork — The fork is its own repository under vendor/rauthy; a row whose files live there needs its own brief in the fork, which does not yet exist and blocks DIRECTORY-004. DIRECTORY-004 names the fork files as work with their owner, not as files of this repository.
- The grant representation: how one grant records who may exercise it, whether it may be passed on (person, agent or nobody), what it derives from, and whether it can be bounded. OPEN for Tom. — The statement leaves the exact delegation schema unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003); AGENT-PARITY-2026-09-23's questions (docs/design/identity/AGENT-PARITY-2026-09-23.md:17-23) are inputs to it, not answers.
- Suspension semantics: whether suspending a person also ends their sign-in session at Rauthy or only makes our checks refuse, and what else stops with a suspended identity. OPEN for Tom. — The lifecycle document asks it of the room (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:99-100, docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:104-105) and its states are only proposed (ADR-011); two systems, one decision.
- The name of the identity service. OPEN for Tom. — The statement lists it as not decided (docs/design/identity/STATEMENT-2026-09-22.md:192); no crate or directory name selects it.
- Which anchor the first agents pin to: our hosted anchor, a self-hosted one, or both. OPEN for Tom. — Lys leaves it as a product decision and so does the statement (docs/design/identity/STATEMENT-2026-09-22.md:193).
- A nightly Rauthy base versus waiting for an upstream release carrying #1696 and #1728. OPEN for Tom. — Waffles' ruling of 15:36:25: if upstream has published no such release by the row 05 showing, Tom decides (docs/design/identity/briefs/IDENTITY-001.json:18; docs/design/identity/STATEMENT-2026-09-22.md:177); rows 02 to 05 run on v0.36.2 meanwhile.
- Road step 2 onward: capability certificates, arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production. Brought forward as step-2 work under the lead's ruling of 27 September 2026: DIRECTORY-013's certificate revocation fold and its history check. — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working.
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
| `docs/design/directory/briefs/DIRECTORY-013.json` | certificate revocation as an appended leaf, the live set folded from the log, and the revoked certificate's history (DP26, ADR-039) | DIRECTORY-013 |
| `docs/design/directory/briefs/DIRECTORY-013.md` | rendered markdown | DIRECTORY-013 |
| `docs/design/identity/CERTIFICATE-REVOCATION.md` | the certificate revocation contract: leaf layouts, unit, signer, permanence, fold, N and tolerance, history, no-log forms, refusals | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/mod.rs` | declarations, re-exports and module docs only | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/leaf.rs` | the three certificate-log leaves and the revocation's signed bytes | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/error.rs` | the named revocation refusals, never carrying key material | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/fold.rs` | the live set folded from a LeafStore to its extent; reads only | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/append.rs` | issuance, revocation and attestation-entry leaves appended through Log::append | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/verify.rs` | revocation-aware certificate verification with N and the tolerance as inputs | DIRECTORY-013 |
| `crates/lys-identity/src/revocation/history.rs` | an attestation by a certificate's key judged against the revocation leaf's position | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_leaf.rs` | leaf encoding, malformed refusals and the revocation signer | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_fold.rs` | the fold's live set, refusals and read-only pass | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_append.rs` | append at the extent, reopen and write failure | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_verify.rs` | revoked, stale, unreadable, not-in-log, expired and no-log legs | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_history.rs` | attestations before and after revocation, proofs and issuance record | DIRECTORY-013 |
| `crates/lys-identity/tests/revocation_support/mod.rs` | test authorities, certificates and file-backed logs; generated keys only | DIRECTORY-013 |
| `crates/lys/src/cli_tests.rs` | the CLI's help tests; DIRECTORY-013 adds the no-log revocation sentence test | DIRECTORY-013 |

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
ADR-039 was added to the ledger by DIRECTORY-013, a brief recording a decision through the ledger and not a document row.

## Constraints

- **CN1** — Documents only: nothing outside docs/design/directory/ and docs/design/decisions.json is created or modified; the IDENTITY-001 files are not changed.
As the lead ruled, CN1 binds DIRECTORY-001 only.
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
Brought forward as step-2 work under the lead's ruling of 27 September 2026: DIRECTORY-013's certificate revocation fold and its history check.
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
id: DIRECTORY-013
cluster: directory
title: Revoke a certificate by appending a leaf, fold the live set from the log, and refuse a revoked certificate while its history still verifies
---

# DIRECTORY-013: Revoke a certificate by appending a leaf, fold the live set from the log, and refuse a revoked certificate while its history still verifies

> **Cluster:** directory
> **Depends on:** DIRECTORY-003
> **Blocked by:** Sign-off of this brief on its card by the project owner or the lead before card_build_v3 runs; a build started without it is a defect of the chain, not a row of this brief. The files this brief adds under crates/lys-identity/src/revocation/ and crates/lys-identity/tests/ are named in each row's files list and in the design's structure array, and this sign-off is their manifest review (CN9). Check: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory both exit 0, and the card records the sign-off before the build is dispatched., R2 to R7 wait on DIRECTORY-003 landed on main, checked by the crate existing there: git ls-tree -d --name-only origin/main crates/lys-identity prints crates/lys-identity. Until it prints, no code row starts, R7's change to the lys binary included. R1 is documents only., R2 to R6 wait on the constructed-attack review of R1's contract: the leaf layouts, the signed bytes of a revocation, the domain tags and the refusal table are a new wire format, frozen once a durable leaf is signed under them, so the adversarial review the repository requires for cryptographic changes (a forged revocation, a revocation signed by a key other than the issuing authority's, a replay of an issuance leaf after a revocation, cross-log and cross-issuer confusion, a revocation placed before its certificate's issuance, a leaf the fold cannot read) is recorded on the card with its verdict before R2 is dispatched. Check: the card carries the review's verdict against the R1 commit's hash.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-039 — A certificate carrying one claim is revoked whole, by its issuing authority's key, permanently, by a leaf folded in lys-identity — A certificate carries one capability claim, so the revocable unit and the claim unit are the same; revoking a claim is revoking its certificate and issuing a new one without it, and the fold never treats a certificate carrying several claims as partially revoked. Only the issuing authority's key signs a revocation leaf; a person responsible for the agent under ADR-003, or an administrator, revokes by asking the directory, which asks the authority to append, and the fold refuses a revocation signed by any other key. A revocation is permanent: no leaf reinstates a certificate, the fold refuses a reinstatement leaf by name, a verbatim replay of an earlier issuance leaf included, and the way back is a new certificate. A revocation always names a certificate the log already holds: the fold refuses a revocation leaf with no earlier issuance leaf for its certificate as revocation_before_issuance, and a later issuance of that certificate is valid. A revoked certificate's inclusion and consistency proofs and its issuance record always verify, and an attestation by its key verifies if and only if its own log entry precedes the revocation leaf, the log order being the evidence of time; an attestation with no entry before the revocation leaf fails, naming the revocation leaf; an attestation by a certificate that is not revoked needs no log entry of its own. The fold and the revocation-aware verification live in lys-identity; lys-core keeps revocation a consumer-side non-goal, and its published verify_certificate_chain and the lys ca verify command keep their meaning and keep passing with no log, while a new library form taking the log, the size N and the tolerance as explicit inputs checks revocation; its command line is a later, separate unpublished binary, and the published lys binary does not depend on lys-identity. Certificate revocation and grant revocation are separate folds: this stands beside DIRECTORY-006 R4 and does not discharge it, and C25 stays open with DP26 as the proposal its review reads. Rejected: revoking claims one by one inside a certificate, letting any key but the issuing authority's sign a revocation, a later leaf reinstating a certificate, letting every attestation made before a revocation stand whatever its log position, a revocation standing for a certificate the log does not hold, building the fold in lys-core, and changing what the published no-log verification means.
> **Checklist:**
> - C65 — docs/design/identity/CERTIFICATE-REVOCATION.md states the certificate-log leaves, the one-claim certificate as the revocable unit, the issuing-authority signer, permanence, revocation_before_issuance, the fold, N and the tolerance, history, the no-log forms and the refusal table.
> - C66 — The issuance leaf, the revocation leaf and the attestation entry encode and decode under lys-identity/certificate-log/v1, and a revocation verifies only under the issuing authority's key for the log's origin.
> - C67 — The live set is folded from the certificate log with its folded size; revocations not signed by the issuing authority, revocations of a certificate with no earlier issuance and reinstatements are refused by name at their index, and a leaf the fold cannot read blocks every permit.
> - C68 — A revocation is one leaf appended at the log's extent through lys-log-store, and the LeafStore trait gains no delete, rewrite, truncate, fork or merge.
> - C69 — Revocation-aware verification takes N and a tolerance with no default, carries the folded size in every answer, and refuses a revoked certificate naming its revocation leaf.
> - C70 — A revoked certificate's inclusion and consistency proofs and issuance record still verify, an attestation by its key verifies only when its own entry precedes the revocation leaf, and an attestation by a certificate not revoked needs no entry.
> - C71 — lys ca verify keeps its meaning and its help says verification without a log does not check revocation, with lys-core unchanged.
> **Stories:**
> - S31 (Certificate verifier, Checks an agent's certificate and its record against the certificate log without the issuer's cooperation) — As a certificate verifier, I want a revoked certificate to fail verification against the log with its revocation leaf named, so that revocation rests on the log and not on the issuer's word.
> - S32 (Certificate verifier, Checks an agent's certificate and its record against the certificate log without the issuer's cooperation) — As a certificate verifier, I want a revoked certificate's past record to still verify, so that revoking a certificate does not erase what it legitimately did before its revocation.
> - S33 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I want my agent's certificate revocable only by its issuing authority's key, so that nobody else can end it and nothing can quietly bring it back.
> - S34 (Certificate verifier, Checks an agent's certificate and its record against the certificate log without the issuer's cooperation) — As a certificate verifier using the command that takes no log, I want its help to say it does not check revocation, so that I never take its pass as proof a certificate is live.

## Purpose

Build DP26 (docs/design/lys-anchor/DECISIONS.md:400-425), ruled and not built, for certificates: revoking a certificate is one appended leaf in an append-only lys log, never a deletion and never a rewrite, and the set of live certificates is folded from that log. The done condition is that a revoked certificate fails verification against the log while its history still verifies. The fold and the revocation-aware verification live in lys-identity, the consumer crate DIRECTORY-003 creates, so revocation stays a consumer-side non-goal of lys-core and no lys-core code or published wire format changes (ADR-039). The log is held through lys-log-store's LeafStore, whose append-only contract (crates/lys-log-store/src/store.rs:1-60 and 120-191: six methods, origin, extent, leaf, put_leaf, pinned and pin, with put_leaf refusing any index but the extent, and no fork, merge, delete, truncate or rewrite) is held unchanged, so a revocation can only be a leaf written at the store's extent. A certificate carries one capability claim, so the revocable unit and the claim unit are the same unit as DP26 rules; only the issuing authority's key signs a revocation; a revocation is permanent; and a certificate's history is its log's inclusion and consistency proofs, its issuance record, and the attestations by its key whose own log entries precede the revocation leaf, the log order being the evidence of time (ADR-039). This brief stands beside DIRECTORY-006 R4's grant revocation and does not discharge it: certificate revocation and grant revocation are separate folds, and C25 stays open with DP26 named as the proposal its review reads.

## Task

Seven rows, in order: R1 the certificate revocation contract document (2 hours); R2 the three certificate-log leaves and the revocation signature (3 hours); R3 the fold of the live set at its log size (4 hours); R4 appending issuance, revocation and attestation-entry leaves through the log (2 hours); R5 revocation-aware verification with the log size N and the tolerance as inputs (3 hours); R6 a revoked certificate's history (3 hours); R7 the help text of the existing lys ca verify (1 hour): 18 focused implementer hours. This estimate sits outside CN7's IDENTITY-001 ceiling, as IDENTITY-002's 4 hours do, because this brief is step-2 work brought forward under the lead's ruling and not a row of IDENTITY-001's lane, so no hour is added to IDENTITY-001 and the ceiling is unchanged. R1 is documents only and lands first; the constructed-attack review of R1's contract follows; R2 to R6 start only when DIRECTORY-003 has landed and the review's verdict is on the card, and R7 starts only when DIRECTORY-003 has landed (blocked_by). In scope: the certificate-log leaf formats, the fold, the append, the verification call and the history call in crates/lys-identity/src/revocation/, their tests, the contract document and one help sentence of lys ca verify. Out of scope, each a later unit: the revocation request path, through which a person responsible under ADR-003 or an administrator asks the directory server and the issuing authority signs and appends, blocked on DIRECTORY-006 R4 and on the directory server, so that until it lands the only way to revoke is a caller that holds the issuing authority identity; a command line that revokes or verifies against a log (a separate unpublished binary; the published lys binary does not depend on lys-identity), the agent's file showing a revoked certificate and its revocation receipt (ADR-008), the C25 freshness review, a documentation-only lys-core release, and a lys-core card answering the known false positive of verify_certificate_chain_at's self-signed screen (crates/lys-core/src/ca/authority.rs:335-341), which rejects a certificate whose subject equals the issuing authority's hex public key common name; this brief records that false positive as a finding, lets the fold record certificate_chain_invalid for such a leaf, and does not do that card, while the typed-claim card's issuance refuses such a subject by name. That last one answers a gap this brief records and does not close: lys-core's published verify_certificate_chain keeps its rustdoc unchanged, because a rustdoc edit is a lys-core source change, so a library consumer reading lys-core's published documentation does not see that verification without a log does not check revocation; the caveat is carried by lys ca verify's help (R7) and lys-identity's documentation (R5). The existing lys ca verify and lys-core's verify_certificate_chain and verify_certificate_chain_at keep their meaning and keep passing with no log, because lys-core 0.2.0's API is frozen. DP26 is taken as it rules and as the lead settled it: a certificate carries one capability claim (the typed-claim card issues one claim per certificate), revoking a claim is revoking its certificate and issuing a new one without it, and the fold never treats a certificate carrying several claims as partially revoked; the tolerance is an input with no default; N is an input and the folded size is in every answer; the fold may refuse on its own authority and never permits beyond what it has walked. Log entries are required only for revocations and for the attestations of a certificate that is revoked: an attestation by a certificate that is not revoked needs no log entry of its own. It holds to DIRECTORY-006 R4 without discharging it: as R4 never lets reinstatement resurrect a revoked grant, no leaf reinstates a revoked certificate; as R4 never permits from a stale state, R5 and R6 never permit beyond the tolerance the caller gives. The fold does not detect a log shown truncated below the caller's own evidence of its size except by the caller's N (docs/design/lys-anchor/KEY-HISTORY-FOLD-QUESTIONS.md:100-140), and R1 says so. Every code row is red first: its first commit adds its test file alone and the named tests fail; its second commit makes them pass. Test names are snake_case and carry the criterion ids (d013_r3_ac1 and so on), so a search proves the mapping. No wall clock in the new library code: every instant is an input. No name of a person and no date inside code. Every path is relative to the repository root.

## Requirements

### R1: Write the certificate revocation contract: the leaves, the unit, the signer, permanence, the fold, N and the tolerance, history, and the refusals

Documents only. THE SYSTEM SHALL add docs/design/identity/CERTIFICATE-REVOCATION.md holding, in this order: (1) the premise: revoking a certificate is one leaf appended to an append-only certificate log through lys-log-store's LeafStore at its extent, and the live set is folded from the log; nothing deletes, rewrites, truncates, forks or merges a leaf, and the LeafStore trait is unchanged; (2) the unit: a certificate carries one capability claim, so the revocable unit and the claim unit are the same unit; revoking a claim is revoking its certificate and issuing a new one without it; no leaf revokes part of a certificate, and a certificate carrying several claim extensions is revoked whole and never treated as partially revoked; (3) the three leaf layouts of a certificate log, one log per issuing authority, as a table whose rows begin with the kind byte, each leaf beginning with the ASCII domain tag lys-identity/certificate-log/v1 and one zero byte, then one kind byte: 0x01 an issuance leaf carrying the certificate's DER, which runs to the end of the leaf with no length prefix; 0x02 a revocation leaf carrying the 32-byte SHA-256 of the revoked certificate's DER and a 64-byte Ed25519 signature; 0x03 an attestation entry carrying the 32-byte SHA-256 of the certificate's DER, then the attestation's COSE bytes, which run to the end of the leaf with no length prefix; a revocation leaf is exactly 129 bytes long; any other tag or kind, a revocation leaf of any other length, an issuance leaf with no DER byte, and an attestation entry with no COSE byte after its hash are malformed; (4) the signed bytes of a revocation: the ASCII domain tag lys-identity/certificate-revocation/v1, one zero byte, the 32-byte SHA-256 of the log's origin as its UTF-8 bytes, and the 32-byte certificate hash; (5) the signer rule: only the key of the authority that issued the certificate signs its revocation; a responsible person or an administrator revokes by asking the directory, which asks the authority to append, and that request path is a later unit, so until it lands the only way to revoke is a caller that holds the issuing authority identity; the fold refuses a revocation leaf whose signature does not verify under the issuing authority's key, whatever other key signed it; (6) permanence: no leaf reinstates a revoked certificate; an issuance leaf naming a certificate the fold already holds revoked, a verbatim replay included, is a reinstatement leaf and is refused by name, and the certificate stays revoked; the way back is a new certificate; the last-wins hazard documented at crates/lys-core/src/delegation/artifact.rs:370-373 is named and refused, never repeated; (7) a revocation always names a certificate the log already holds: a revocation leaf whose certificate has no earlier issuance leaf is refused as revocation_before_issuance and revokes nothing, and a later issuance leaf of that certificate is a valid issuance and not a reinstatement; (8) the fold: a pass over every leaf from index 0 to the store's extent; an issuance leaf is valid when its DER verifies under the issuing authority's public key with lys-core's verify_certificate_chain_at, unchanged, at the certificate's own notBefore instant read from that DER, so that the check is of the issuer's signature and reads no clock (lys-core's verify_certificate_chain reads the wall clock and is not used), the validity window at the caller's instant being the verification call's check; the log carries no instant and the fold never invents one, so a certificate used before its window begins is refused at the moment of use by the claim verifier, as claim_not_yet_valid on the typed-claim card (hhAN8h77), and not by the fold; a certificate whose subject common name equals the issuing authority's lowercase hex public key fails verify_certificate_chain_at's self-signed screen, a known false positive recorded at crates/lys-core/src/ca/authority.rs:335-341, so its issuance leaf records certificate_chain_invalid; the typed-claim card's issuance refuses such a subject by name, and the false positive is answered by its own lys-core card, not by this contract; an issuance leaf that is not valid is recorded as certificate_chain_invalid at its index, issues nothing, and does not count as an issuance for revocation_before_issuance, for a reinstatement or for certificate_not_in_log; the fold answers the folded size, the issued set, the revoked set with each revocation's index, and every refused leaf with its index and refusal name; a leaf the fold cannot read blocks every permit and never revokes or issues anything; (9) N and the tolerance: the verification call takes the size N the caller has evidence of and a tolerance counted in entries, with no default for either; every answer carries the size the fold walked; a refusal stands on the fold's own authority whatever its size; a permit is refused when the folded size is below N by more than the tolerance; and the statement of what the fold does NOT claim: a log shown truncated has a tip too, and the fold detects truncation only against the caller's N; (10) history: the log's inclusion and consistency proofs and a certificate's issuance record always verify after its revocation; an attestation by a revoked certificate's key verifies if and only if its own attestation entry precedes the revocation leaf, the log order being the evidence of time, so an attestation made after a compromise cannot be placed before the revocation; an attestation with no entry before the revocation leaf fails and the refusal names the revocation leaf; an attestation by a certificate that is not revoked needs no log entry of its own: the check verifies its signature and that no revocation leaf in the log covers the certificate, and answers not revoked; (11) the no-log forms: lys ca verify and lys-core's verify_certificate_chain and verify_certificate_chain_at keep their meaning, keep passing with no log and do not check revocation; lys ca verify's help and lys-identity's documentation say so; lys-core's published rustdoc of verify_certificate_chain does not say so, which is recorded here as a known gap whose answer is a documentation-only lys-core release, its own card, not done by this contract; the form that takes the log is a library call in lys-identity, and a command line for it is a separate unpublished binary on which the published lys binary does not depend; (12) the refusal table, one row per refusal beginning with its name, when it fires and the act that answers it: certificate_leaf_malformed, revocation_not_signed_by_issuer, revocation_before_issuance, certificate_reinstatement_refused, certificate_revoked, certificate_not_in_log, fold_unreadable_leaf, fold_stale, attestation_after_revocation, certificate_chain_invalid, attestation_signature_invalid, store_read_failed; (13) the statement that this contract stands beside DIRECTORY-006 R4 and does not discharge it, that certificate revocation and grant revocation are separate folds, that DP26 is the proposal C25's review reads and C25 stays open, and that the formats above need the repository's constructed-attack review before any leaf is signed outside a test. The document SHALL NOT change lys-core, lys-log-store or any published tag, SHALL NOT edit C25, and SHALL NOT edit any IDENTITY-001 file, docs/design/lys-anchor/DECISIONS.md or docs/design/WIRE-FORMATS.md. Estimate: 2 hours.

**Acceptance:**
- Leg 1 of 5: grep -c 'lys-identity/certificate-log/v1' docs/design/identity/CERTIFICATE-REVOCATION.md prints at least 1, grep -c 'lys-identity/certificate-revocation/v1' docs/design/identity/CERTIFICATE-REVOCATION.md prints at least 1, and grep -c '^| 0x01 \|^| 0x02 \|^| 0x03 ' docs/design/identity/CERTIFICATE-REVOCATION.md prints 3.
- Leg 2 of 5: grep -c '^| certificate_leaf_malformed \|^| revocation_not_signed_by_issuer \|^| revocation_before_issuance \|^| certificate_reinstatement_refused \|^| certificate_revoked \|^| certificate_not_in_log \|^| fold_unreadable_leaf \|^| fold_stale \|^| attestation_after_revocation \|^| certificate_chain_invalid \|^| attestation_signature_invalid \|^| store_read_failed ' docs/design/identity/CERTIFICATE-REVOCATION.md prints 12.
- Leg 3 of 5: each of grep -c 'one capability claim', grep -c 'no leaf reinstates a revoked certificate', grep -c 'a revocation always names a certificate the log already holds', grep -c 'only the key of the authority that issued the certificate', grep -c 'precedes the revocation leaf', grep -c 'at the certificate.s own notBefore', grep -c 'no length prefix', grep -c 'needs no log entry of its own', grep -c 'no default', grep -c 'detects truncation only against the caller', grep -c 'do not check revocation', grep -c 'documentation-only lys-core release', grep -c 'separate unpublished binary', grep -c 'claim_not_yet_valid', grep -c 'hhAN8h77' and grep -c 'authority.rs:335-341' over docs/design/identity/CERTIFICATE-REVOCATION.md prints at least 1.
- Leg 4 of 5: grep -c 'DIRECTORY-006 R4' docs/design/identity/CERTIFICATE-REVOCATION.md prints at least 1, grep -c 'DP26' docs/design/identity/CERTIFICATE-REVOCATION.md prints at least 1 and grep -c 'C25 stays open' docs/design/identity/CERTIFICATE-REVOCATION.md prints at least 1.
- Leg 5 of 5: git show --name-only --format= on the R1 commit prints exactly one line, docs/design/identity/CERTIFICATE-REVOCATION.md, and sh scripts/design/gate.sh exits 0 on that commit.

**Files:**
- create: docs/design/identity/CERTIFICATE-REVOCATION.md

**Checklist:**
- C65 — docs/design/identity/CERTIFICATE-REVOCATION.md states the certificate-log leaves, the one-claim certificate as the revocable unit, the issuing-authority signer, permanence, revocation_before_issuance, the fold, N and the tolerance, history, the no-log forms and the refusal table.

**Stories:**
- S33 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I want my agent's certificate revocable only by its issuing authority's key, so that nobody else can end it and nothing can quietly bring it back.

### R2: Encode and decode the three certificate-log leaves, and verify a revocation only under the issuing authority's key

THE SYSTEM SHALL encode and decode the issuance leaf, the revocation leaf and the attestation entry exactly as R1's contract lays them out, in crates/lys-identity/src/revocation/leaf.rs, with the refusals in crates/lys-identity/src/revocation/error.rs and crates/lys-identity/src/revocation/mod.rs carrying only module declarations, re-exports and module docs. A certificate is named by the SHA-256 of its DER, computed from the bytes the caller supplies and never from a serial or name the issuer chose. WHEN bytes are decoded, THE SYSTEM SHALL refuse certificate_leaf_malformed naming the reason for a wrong domain tag, a missing zero byte, an unknown kind byte, a revocation leaf that is not exactly 129 bytes long, an issuance leaf with no DER byte, and an attestation entry with no COSE byte after its hash, and SHALL NOT return a partial leaf. WHEN a revocation is signed, THE SYSTEM SHALL sign R1's signed bytes, which bind the SHA-256 of the log's origin and the certificate hash, with no length prefix anywhere, with the issuing authority's Ed25519 identity from lys-core. WHEN a revocation is checked against an issuer public key, THE SYSTEM SHALL verify the signature over R1's signed bytes rebuilt from the leaf and the log's origin, and SHALL refuse revocation_not_signed_by_issuer when it does not verify; a signature by any other key, a signature made for another log's origin and a signature over another certificate's hash SHALL all be refused. The leaf layer SHALL NOT carry a claim selector, a reinstatement kind or any field R1 does not lay out, and SHALL NOT change lys-core. No private key material appears in Debug, logs or refusals. Estimate: 3 hours.

**Acceptance:**
- Leg 1 of 5, d013_r2_ac1: one issuance leaf, one revocation leaf and one attestation entry built from fixed fixture bytes encode to bytes whose first 32 bytes are lys-identity/certificate-log/v1 followed by 0x00, whose 33rd byte is 0x01, 0x02 and 0x03 respectively, and decode back to equal values, the revocation leaf being 129 bytes long and the issuance leaf being 33 bytes plus the fixture DER's length; 3 round trips asserted; cargo test -p lys-identity --test revocation_leaf d013_r2_ac1 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 2 of 5, d013_r2_ac2: 7 malformed inputs, a wrong tag, a missing zero byte, kind byte 0x04, an issuance leaf of 33 bytes ending at its kind byte, a revocation leaf of 128 bytes, a revocation leaf of 130 bytes, and a revocation leaf carrying an extra 8-byte claim selector (137 bytes), are each refused certificate_leaf_malformed; the test asserts 7 refusals and 0 decoded leaves; cargo test -p lys-identity --test revocation_leaf d013_r2_ac2 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 3 of 5, d013_r2_ac3: a revocation signed by the issuing authority verifies under its public key; the same certificate's revocation signed by a second authority's key, one signed for another origin (the SHA-256 of a different origin string in the signed bytes), and one whose certificate hash is replaced by another certificate's are each refused revocation_not_signed_by_issuer; the test asserts 1 verified and 3 refused; cargo test -p lys-identity --test revocation_leaf d013_r2_ac3 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 4 of 5, d013_r2_ac4: the Debug rendering of every value the leaf layer returns and every refusal it raises, for the fixtures of legs 1 to 3, contains none of the issuing authority's 32 seed bytes as lowercase hex; cargo test -p lys-identity --test revocation_leaf d013_r2_ac4 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 5 of 5, the mapping, the wall and red first: rg -c 'fn \w*d013_r2_ac(1|2|3|4)(_|\b)' crates/lys-identity/tests/revocation_leaf.rs prints 4; git diff --stat $(git merge-base origin/main HEAD) HEAD -- crates/lys-core crates/lys-log-store prints nothing; on the row's first commit cargo test -p lys-identity --test revocation_leaf exits non-zero.

**Files:**
- create: crates/lys-identity/src/revocation/mod.rs
- create: crates/lys-identity/src/revocation/leaf.rs
- create: crates/lys-identity/src/revocation/error.rs
- create: crates/lys-identity/tests/revocation_leaf.rs
- create: crates/lys-identity/tests/revocation_support/mod.rs
- modify: crates/lys-identity/src/lib.rs
- modify: crates/lys-identity/Cargo.toml

**Checklist:**
- C66 — The issuance leaf, the revocation leaf and the attestation entry encode and decode under lys-identity/certificate-log/v1, and a revocation verifies only under the issuing authority's key for the log's origin.

**Stories:**
- S33 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I want my agent's certificate revocable only by its issuing authority's key, so that nobody else can end it and nothing can quietly bring it back.

### R3: Fold the live set from the certificate log, refusing unauthorised revocations, revocations before issuance and reinstatements by name

THE SYSTEM SHALL fold a certificate log in crates/lys-identity/src/revocation/fold.rs, given a store through lys-log-store's LeafStore trait and the issuing authority's public key, by reading every leaf from index 0 to the store's extent once, in order, and SHALL answer a live set carrying the folded size, the issued certificate hashes with the index of each one's first valid issuance leaf, the revoked certificate hashes with the index of each one's first valid revocation leaf, and every refused leaf with its index and refusal name. WHEN an issuance leaf's DER does not verify under the issuing authority's public key with lys-core's verify_certificate_chain_at, unchanged, at the certificate's own notBefore instant read from that DER (R1 (8)), THE SYSTEM SHALL record certificate_chain_invalid at its index, and that leaf SHALL NOT issue anything and SHALL NOT count as an issuance for revocation_before_issuance, for a reinstatement or for R5's certificate_not_in_log; a valid issuance leaf is one whose DER does verify so. The fold SHALL NOT call lys-core's verify_certificate_chain, which reads the wall clock. The log carries no instant and the fold SHALL NOT invent one: it checks only the issuer's signature at the certificate's own notBefore, and a certificate used before its window begins is refused at the moment of use by the claim verifier, as claim_not_yet_valid on the typed-claim card (hhAN8h77). WHEN an issuance leaf's certificate has a subject common name equal to the issuing authority's lowercase hex public key, THE SYSTEM SHALL record certificate_chain_invalid at its index, as verify_certificate_chain_at's self-signed screen answers today; that answer is a known false positive recorded at crates/lys-core/src/ca/authority.rs:335-341, answered by its own lys-core card, and the fold SHALL NOT work around it. WHEN a revocation leaf's signature does not verify under the issuing authority's key for the store's origin, THE SYSTEM SHALL record revocation_not_signed_by_issuer at its index and SHALL NOT revoke the certificate it names. WHEN a revocation leaf names a certificate with no valid issuance leaf at a lower index, THE SYSTEM SHALL record revocation_before_issuance at its index and SHALL NOT revoke the certificate it names, and a later valid issuance leaf of that certificate SHALL be an issuance, not a reinstatement. WHEN a valid issuance leaf names a certificate already revoked at a lower index, a verbatim replay of its earlier issuance leaf included, THE SYSTEM SHALL record certificate_reinstatement_refused at its index and the certificate SHALL stay revoked; the fold SHALL NOT let a later leaf win over a revocation, the last-wins hazard documented at crates/lys-core/src/delegation/artifact.rs:370-373. WHEN a leaf does not decode, THE SYSTEM SHALL record fold_unreadable_leaf at its index; that leaf SHALL NOT issue or revoke anything, and R5 and R6 SHALL NOT permit from a fold that holds one. A second valid revocation of a certificate already revoked and a second issuance leaf of a live certificate SHALL change nothing, and neither is a refusal. The fold SHALL NOT answer for part of a certificate: a certificate carrying several claim extensions is issued and revoked whole. A revoked certificate stays in the answer as revoked with its leaf index and is never dropped from it (ADR-008). IF the store fails to read a leaf, THEN THE SYSTEM SHALL refuse store_read_failed naming the index and SHALL NOT answer a partial set. The fold SHALL read only through LeafStore::extent, LeafStore::leaf and LeafStore::origin, and SHALL NOT call put_leaf, pin, Log::open or Log::append; a fold writes nothing. Estimate: 4 hours.

**Acceptance:**
- Leg 1 of 9, d013_r3_ac1, the last-wins leg (crates/lys-core/src/delegation/artifact.rs:370-373): a fixture log of 4 leaves, issuance of certificate A, issuance of certificate B, a revocation of A signed by the issuing authority, and a verbatim copy of A's issuance leaf at index 3, folds to folded size 4, issued A at 0 and B at 1, revoked A at 2, and exactly 1 refused leaf, certificate_reinstatement_refused at index 3; A is not live and B is live; cargo test -p lys-identity --test revocation_fold d013_r3_ac1 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 2 of 9, d013_r3_ac2, the other-signer leg: a fixture log of 3 leaves, issuance of A, a revocation of A signed by a second authority's key for the store's origin, and a revocation of A signed by the issuing authority for another origin, folds to 0 revoked certificates, A live, and exactly 2 refused leaves, revocation_not_signed_by_issuer at indexes 1 and 2; cargo test -p lys-identity --test revocation_fold d013_r3_ac2 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 3 of 9, d013_r3_ac3: a fixture log of 5 leaves, issuance of A, issuance of A again, a valid revocation of A, the same revocation leaf again, and 7 bytes that do not decode at index 4, folds to revoked A at index 2, 0 reinstatement refusals, exactly 1 refused leaf, fold_unreadable_leaf at index 4, and folded size 5; cargo test -p lys-identity --test revocation_fold d013_r3_ac3 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 4 of 9, d013_r3_ac4: a certificate issued with two custom extensions under the lys arc, its issuance leaf and one valid revocation leaf fold to that certificate revoked whole, and the answer holds exactly 1 revoked entry, keyed by the SHA-256 of that certificate's DER; cargo test -p lys-identity --test revocation_fold d013_r3_ac4 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 5 of 9, d013_r3_ac5: after 3 folds of leg 1's log the store's extent, its pinned tree size and root, and the SHA-256 of every file under its directory equal those recorded before the first fold; cargo test -p lys-identity --test revocation_fold d013_r3_ac5 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 6 of 9, d013_r3_ac6, the revocation_before_issuance leg: a fixture log of 2 leaves, a revocation of certificate A signed by the issuing authority for the store's origin at index 0 and the issuance of A at index 1, folds to folded size 2, issued A at 1, 0 revoked certificates, 0 reinstatement refusals and exactly 1 refused leaf, revocation_before_issuance at index 0; A is live; cargo test -p lys-identity --test revocation_fold d013_r3_ac6 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 7 of 9, d013_r3_ac7, the second-authority issuance leg: a fixture log of 3 leaves, the issuance leaf of certificate C issued by a second authority at index 0, a revocation of C signed by the issuing authority for the store's origin at index 1, and the issuance of certificate A by the issuing authority at index 2, folded with the issuing authority's public key, folds to folded size 3, issued A at 2 only, C not issued, 0 revoked certificates, and exactly 2 refused leaves, certificate_chain_invalid at index 0 and revocation_before_issuance at index 1; cargo test -p lys-identity --test revocation_fold d013_r3_ac7 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 8 of 9, d013_r3_ac8, the self-signed false-positive leg (crates/lys-core/src/ca/authority.rs:335-341): a fixture log of 2 leaves, the issuance leaf of certificate E issued by the issuing authority with the subject equal to the issuing authority's lowercase hex public key at index 0, and the issuance of certificate A at index 1, folds to folded size 2, issued A at 1 only, E not issued, 0 revoked certificates, and exactly 1 refused leaf, certificate_chain_invalid at index 0; cargo test -p lys-identity --test revocation_fold d013_r3_ac8 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 9 of 9, the mapping, the wall and red first: rg -c 'fn \w*d013_r3_ac(1|2|3|4|5|6|7|8)(_|\b)' crates/lys-identity/tests/revocation_fold.rs prints 8; rg -n 'put_leaf|\.pin\(|Log::open|\.append\(' crates/lys-identity/src/revocation/fold.rs prints nothing; rg -n 'verify_certificate_chain\(' crates/lys-identity/src/revocation/fold.rs prints nothing; on the row's first commit cargo test -p lys-identity --test revocation_fold exits non-zero.

**Files:**
- create: crates/lys-identity/src/revocation/fold.rs
- create: crates/lys-identity/tests/revocation_fold.rs
- modify: crates/lys-identity/src/revocation/mod.rs
- modify: crates/lys-identity/src/revocation/error.rs
- modify: crates/lys-identity/tests/revocation_support/mod.rs

**Checklist:**
- C67 — The live set is folded from the certificate log with its folded size; revocations not signed by the issuing authority, revocations of a certificate with no earlier issuance and reinstatements are refused by name at their index, and a leaf the fold cannot read blocks every permit.

**Stories:**
- S31 (Certificate verifier, Checks an agent's certificate and its record against the certificate log without the issuer's cooperation) — As a certificate verifier, I want a revoked certificate to fail verification against the log with its revocation leaf named, so that revocation rests on the log and not on the issuer's word.
- S33 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I want my agent's certificate revocable only by its issuing authority's key, so that nobody else can end it and nothing can quietly bring it back.

### R4: Append issuance, revocation and attestation-entry leaves only as new leaves at the log's extent

THE SYSTEM SHALL append a certificate's issuance leaf, its revocation leaf signed with the issuing authority's identity for the log's origin, and an attestation entry, in crates/lys-identity/src/revocation/append.rs, each through lys-log-store's Log::append, which writes with LeafStore::put_leaf at the store's extent, and SHALL return the index each leaf was written at. WHEN a revocation is appended, THE SYSTEM SHALL add exactly one leaf and SHALL NOT remove, rewrite, truncate or replace any stored leaf or any byte of one. IF lys-log-store refuses or fails a write, THEN THE SYSTEM SHALL return that refusal with the operation and the index it was attempted at, and SHALL NOT retry the write and SHALL NOT report the leaf as written. The append layer SHALL NOT decide who asked for the revocation: it signs with the issuing authority's identity it is given, and the request path through which a responsible person or an administrator asks the directory is a later unit, so until it lands the only way to revoke is a caller that holds the issuing authority identity. lys-log-store is not changed by this brief: the LeafStore trait keeps its six methods, origin, extent, leaf, put_leaf, pinned and pin, and gains no delete, rewrite, truncate, fork or merge. Estimate: 2 hours.

**Acceptance:**
- Leg 1 of 4, d013_r4_ac1: on a fresh file-backed log, appending issuance of A, issuance of B, a revocation of A and an attestation entry for B returns indexes 0, 1, 2 and 3; the store's extent is then 4; the bytes at indexes 0 and 1 are equal to the bytes read at those indexes before the revocation was appended; cargo test -p lys-identity --test revocation_append d013_r4_ac1 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 2 of 4, d013_r4_ac2: the log of leg 1 reopened from its directory folds (R3) to A revoked at index 2 and B live, and every leaf read after reopen is byte-identical to the leaf read before; cargo test -p lys-identity --test revocation_append d013_r4_ac2 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 3 of 4, d013_r4_ac3: a store whose put_leaf returns an I/O failure at index 1, the test's own LeafStore implementation, refuses the append of a revocation with the failure, the operation and index 1, and its extent stays 1; cargo test -p lys-identity --test revocation_append d013_r4_ac3 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 4 of 4, the wall and red first: rg -c 'fn \w*d013_r4_ac(1|2|3)(_|\b)' crates/lys-identity/tests/revocation_append.rs prints 3; git diff --stat $(git merge-base origin/main HEAD) HEAD -- crates/lys-log-store prints nothing; rg -n 'fn (delete|remove|rewrite|truncate|fork|merge)' crates/lys-log-store/src crates/lys-identity/src/revocation prints nothing; on the row's first commit cargo test -p lys-identity --test revocation_append exits non-zero.

**Files:**
- create: crates/lys-identity/src/revocation/append.rs
- create: crates/lys-identity/tests/revocation_append.rs
- modify: crates/lys-identity/src/revocation/mod.rs
- modify: crates/lys-identity/src/revocation/error.rs
- modify: crates/lys-identity/tests/revocation_support/mod.rs

**Checklist:**
- C68 — A revocation is one leaf appended at the log's extent through lys-log-store, and the LeafStore trait gains no delete, rewrite, truncate, fork or merge.

**Stories:**
- S33 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I want my agent's certificate revocable only by its issuing authority's key, so that nobody else can end it and nothing can quietly bring it back.

### R5: Verify a certificate against the log with N and the tolerance as inputs, and refuse a revoked certificate naming its revocation leaf

THE SYSTEM SHALL offer, in crates/lys-identity/src/revocation/verify.rs, a verification call taking a store through LeafStore, the issuing authority's public key, the certificate's DER, the instant to check the validity window at, the size N the caller has evidence of, and a tolerance counted in entries, every one of them a required input: there SHALL be no default tolerance, no default N and no default instant, and no Default implementation or constructor supplies one. WHEN called, THE SYSTEM SHALL check the certificate with lys-core's verify_certificate_chain_at, unchanged, and refuse certificate_chain_invalid when it fails, before any fold answer; SHALL fold the store (R3); and SHALL answer, in this order: certificate_revoked naming the revocation leaf's index and the folded size, when the fold holds the certificate revoked, whatever the folded size; fold_unreadable_leaf naming the index, when the fold holds a leaf it could not read; fold_stale naming the folded size, N and the tolerance, when the folded size is below N by more than the tolerance; certificate_not_in_log, when the fold holds no issuance leaf for the certificate; and otherwise live, carrying the folded size and N. Every answer, permit and refusal alike, SHALL carry the folded size. The call SHALL NOT permit a certificate the fold holds revoked, SHALL NOT permit from a fold holding an unreadable leaf, and SHALL NOT read a wall clock. Its documentation SHALL say that verification without a log, lys-core's verify_certificate_chain and verify_certificate_chain_at and the lys ca verify command, does not check revocation. lys-core's verify_certificate_chain and verify_certificate_chain_at SHALL NOT change, their documentation included, and SHALL keep passing a revoked certificate when no log is given. Estimate: 3 hours.

**Acceptance:**
- Leg 1 of 6, d013_r5_ac1: certificate A issued at index 0 verifies live at N equal to 1 with tolerance 0, the answer carrying folded size 1; after a valid revocation of A is appended at index 1 the same call with N equal to 2 and tolerance 0 refuses certificate_revoked naming index 1 and folded size 2; cargo test -p lys-identity --test revocation_verify d013_r5_ac1 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 2 of 6, d013_r5_ac2: on a log of 3 leaves with no revocation of A, the call with N equal to 5 refuses fold_stale naming folded size 3, N 5 and tolerance 1 when the tolerance is 1, and answers live carrying folded size 3 and N 5 when the tolerance is 2; on a second log of 3 leaves whose leaf at index 2 is a valid revocation of A, with N equal to 50 and tolerance 0, the call refuses certificate_revoked naming index 2 and not fold_stale; the test asserts 1 fold_stale, 1 live and 1 certificate_revoked; cargo test -p lys-identity --test revocation_verify d013_r5_ac2 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 3 of 6, d013_r5_ac3: each case called with an instant inside A's validity window, N equal to 2 and tolerance 0, on a log of exactly 2 leaves: a log holding A's issuance at index 0 and 7 bytes that do not decode at index 1 refuses fold_unreadable_leaf naming index 1 and folded size 2; a log holding the issuance of certificate B at index 0 and of certificate D at index 1, all three certificates issued by the issuing authority, refuses A certificate_not_in_log with folded size 2; a log holding A's issuance at index 0 and a revocation of A signed by a second authority's key at index 1 answers A live carrying folded size 2 and N 2; the test asserts 2 refusals and 1 live; cargo test -p lys-identity --test revocation_verify d013_r5_ac3 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 4 of 6, d013_r5_ac4: each case called with N equal to 2 and tolerance 0: on leg 1's 2-leaf log, A, revoked at index 1, verified at an instant one second after its notAfter refuses certificate_chain_invalid and not certificate_revoked; on a 2-leaf log holding A's issuance at index 0 and the issuance leaf of certificate C issued by a second authority at index 1, C checked against the first authority's key at an instant inside C's validity window refuses certificate_chain_invalid; the test asserts 2 certificate_chain_invalid and 0 certificate_revoked; cargo test -p lys-identity --test revocation_verify d013_r5_ac4 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 5 of 6, d013_r5_ac5: for the revoked certificate of leg 1, lys-core's verify_certificate_chain_at with the same issuer key and an instant inside its validity window returns Ok, and lys-core's verify_certificate_chain with the same issuer key returns Ok, so the no-log forms keep their meaning; cargo test -p lys-identity --test revocation_verify d013_r5_ac5 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 6 of 6, the mapping, the wall and red first: rg -c 'fn \w*d013_r5_ac(1|2|3|4|5)(_|\b)' crates/lys-identity/tests/revocation_verify.rs prints 5; rg -n 'SystemTime::now|Utc::now|Local::now|Instant::now|impl Default' crates/lys-identity/src/revocation prints nothing; rg -c 'does not check revocation' crates/lys-identity/src/revocation/verify.rs prints at least 1; git diff --stat $(git merge-base origin/main HEAD) HEAD -- crates/lys-core prints nothing; on the row's first commit cargo test -p lys-identity --test revocation_verify exits non-zero.

**Files:**
- create: crates/lys-identity/src/revocation/verify.rs
- create: crates/lys-identity/tests/revocation_verify.rs
- modify: crates/lys-identity/src/revocation/mod.rs
- modify: crates/lys-identity/src/revocation/error.rs
- modify: crates/lys-identity/tests/revocation_support/mod.rs

**Checklist:**
- C69 — Revocation-aware verification takes N and a tolerance with no default, carries the folded size in every answer, and refuses a revoked certificate naming its revocation leaf.

**Stories:**
- S31 (Certificate verifier, Checks an agent's certificate and its record against the certificate log without the issuer's cooperation) — As a certificate verifier, I want a revoked certificate to fail verification against the log with its revocation leaf named, so that revocation rests on the log and not on the issuer's word.

### R6: Keep a revoked certificate's history verifying: its proofs, its issuance record, and the attestations logged before its revocation

THE SYSTEM SHALL offer, in crates/lys-identity/src/revocation/history.rs, an attestation call taking a store through LeafStore, the issuing authority's public key, the certificate's DER, the attestation's COSE bytes, the payload, the size N the caller has evidence of and a tolerance counted in entries, all required. WHEN called, THE SYSTEM SHALL verify the attestation under the certificate's subject public key with lys-core's verify_attestation_bytes_by_signer, unchanged, and refuse attestation_signature_invalid when it fails; SHALL fold the store (R3); and, WHEN the fold holds the certificate revoked at index r, SHALL first refuse fold_unreadable_leaf naming the index when the fold holds any leaf it could not read, before any verified answer, and otherwise SHALL answer verified, naming the entry's index, r and the folded size, if and only if an attestation entry naming the certificate's hash with exactly these COSE bytes sits at an index below r, and otherwise SHALL refuse attestation_after_revocation naming r; an entry at an index above r SHALL NOT verify. WHEN the fold does not hold the certificate revoked, THE SYSTEM SHALL answer not revoked carrying the folded size, subject to R5's fold_unreadable_leaf and fold_stale refusals with the same N and tolerance, and SHALL NOT require an attestation entry for it. The log's inclusion and consistency proofs and the issuance record SHALL keep verifying after a revocation with lys-core's verify_inclusion_raw and verify_consistency, unchanged: nothing in this brief removes, rewrites or hides the issuance leaf or any leaf before the revocation. The call SHALL NOT read a wall clock and SHALL NOT change lys-core. Estimate: 3 hours.

**Acceptance:**
- Leg 1 of 6, d013_r6_ac1: a log of 5 leaves, issuance of A at 0, an attestation entry by A's key over payload P1 at 1, a valid revocation of A at 2, an attestation entry by A's key over payload P2 at 3, and issuance of B at 4, each call made with N 5 and tolerance 0: the P1 attestation verifies naming entry 1, revocation 2 and folded size 5; the P2 attestation refuses attestation_after_revocation naming index 2; an attestation by A's key over P3 with no entry refuses attestation_after_revocation naming index 2; the test asserts 1 verified and 2 refused; cargo test -p lys-identity --test revocation_history d013_r6_ac1 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 2 of 6, d013_r6_ac2: on leg 1's log, the inclusion proof of the issuance leaf at index 0 and of the revocation leaf at index 2, each built at tree size 5, verifies with lys-core's verify_inclusion_raw against the root of the 5-leaf tree, and the consistency proof from size 2 to size 5 verifies with lys-core's verify_consistency; 3 proofs asserted; cargo test -p lys-identity --test revocation_history d013_r6_ac2 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 3 of 6, d013_r6_ac3: on leg 1's log, the issuance leaf at index 0 decodes to DER whose SHA-256 is A's hash, and lys-core's verify_certificate_chain_at on that DER with the issuing authority's key at an instant inside its validity window returns Ok; cargo test -p lys-identity --test revocation_history d013_r6_ac3 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 4 of 6, d013_r6_ac4: the P1 attestation checked with the COSE bytes of an attestation by B's key refuses attestation_signature_invalid; B's own attestation over P1, with no attestation entry for B in the log and B not revoked, answers not revoked carrying folded size 5; cargo test -p lys-identity --test revocation_history d013_r6_ac4 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 5 of 6, d013_r6_ac5, the unreadable-below-revocation leg: a log of 4 leaves, issuance of A at 0, an attestation entry by A's key over payload P1 at 1, 7 bytes that do not decode at 2, and a valid revocation of A at 3, called for the P1 attestation with N 4 and tolerance 0, refuses fold_unreadable_leaf naming index 2 and does not answer verified; the test asserts 1 fold_unreadable_leaf and 0 verified; cargo test -p lys-identity --test revocation_history d013_r6_ac5 prints 'test result: ok. 1 passed; 0 failed'.
- Leg 6 of 6, the mapping and red first: rg -c 'fn \w*d013_r6_ac(1|2|3|4|5)(_|\b)' crates/lys-identity/tests/revocation_history.rs prints 5; on the row's first commit cargo test -p lys-identity --test revocation_history exits non-zero.

**Files:**
- create: crates/lys-identity/src/revocation/history.rs
- create: crates/lys-identity/tests/revocation_history.rs
- modify: crates/lys-identity/src/revocation/mod.rs
- modify: crates/lys-identity/src/revocation/error.rs
- modify: crates/lys-identity/tests/revocation_support/mod.rs

**Checklist:**
- C70 — A revoked certificate's inclusion and consistency proofs and issuance record still verify, an attestation by its key verifies only when its own entry precedes the revocation leaf, and an attestation by a certificate not revoked needs no entry.

**Stories:**
- S32 (Certificate verifier, Checks an agent's certificate and its record against the certificate log without the issuer's cooperation) — As a certificate verifier, I want a revoked certificate's past record to still verify, so that revoking a certificate does not erase what it legitimately did before its revocation.

### R7: Say in lys ca verify's help that verification without a log does not check revocation, and change nothing else it does

THE SYSTEM SHALL add to the help of the lys ca verify command, in the Verify variant's doc comment in crates/lys/src/cli.rs, the sentence: Verification without a log does not check revocation. The command SHALL keep its arguments, its exit codes and its output, and SHALL keep exiting 0 for a certificate whose chain and validity window verify, revoked or not, because lys-core's published verify_certificate_chain and verify_certificate_chain_at keep their meaning. The lys binary SHALL NOT gain a dependency on lys-identity, and this row SHALL NOT add a command, an argument or any change to lys-core, the rustdoc of verify_certificate_chain included. Estimate: 1 hour.

**Acceptance:**
- Leg 1 of 2, ca_verify_help_says_no_log_checks_no_revocation in crates/lys/src/cli_tests.rs: the rendered long help of lys ca verify contains 'Verification without a log does not check revocation.' exactly once; cargo test -p lys ca_verify_help_says_no_log_checks_no_revocation prints 'test result: ok. 1 passed; 0 failed'.
- Leg 2 of 2: git diff $(git merge-base origin/main HEAD) HEAD -- crates/lys/src/cli.rs shows added lines only inside the Verify variant's doc comment and no removed line; git diff --stat $(git merge-base origin/main HEAD) HEAD -- crates/lys/Cargo.toml crates/lys-core prints nothing; the existing tests of crates/lys/src/commands/ca_tests.rs pass unchanged under cargo test -p lys.

**Files:**
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/cli_tests.rs

**Checklist:**
- C71 — lys ca verify keeps its meaning and its help says verification without a log does not check revocation, with lys-core unchanged.

**Stories:**
- S34 (Certificate verifier, Checks an agent's certificate and its record against the certificate log without the issuer's cooperation) — As a certificate verifier using the command that takes no log, I want its help to say it does not check revocation, so that I never take its pass as proof a certificate is live.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and the lead approves a brief revision before that file is edited (CN9). Of DIRECTORY-003's files only crates/lys-identity/src/lib.rs (one module line) and crates/lys-identity/Cargo.toml (the dependencies R2 names) are touched.
- No code in lys-core or lys-log-store changes and no published wire format changes: lys/attestation/v2, lys/sealed-envelope/v1, lys/anchor-receipt/v1, lys/verification-bundle/v1, lys/consistency-receipt/v1 and lys/delegation/v1 stay byte-identical; lys-core's verify_certificate_chain and verify_certificate_chain_at keep their meaning and their documentation; TrustError::CertificateRevocation is not constructed by this brief; no lys-core release is made by this brief.
- Revocation is a leaf, never a removal: the LeafStore trait gains no delete, rewrite, truncate, fork or merge, its contract stays as written, and no row writes a leaf other than through Log::append at the extent.
- A certificate carries one capability claim and is the revocable unit; no leaf, field or call revokes, selects or answers for part of a certificate. No leaf and no call reinstates a revoked certificate.
- No default tolerance, no default N and no default instant anywhere; no wall clock in the library code of crates/lys-identity/src/revocation/.
- No revocation leaf is signed outside a test and no production key is generated or used by this brief or its build; the leaf formats are signed only in tests until the constructed-attack review's verdict is on the card.
- This brief stands beside DIRECTORY-006 R4 and does not discharge it: no grant, grant revocation or permission decision is built or changed here, C25 stays open, DP26 is the proposal C25's review reads and this brief does not close that review, and no file of DIRECTORY-006 is edited.
- No command is added to the lys binary and lys gains no dependency on lys-identity; the command that revokes or verifies against a log is a later unit, a separate unpublished binary; the agent's file showing a revoked certificate is a later unit and is not touched.
- The revocation request path is not built here: no row decides who may ask for a revocation and no row adds a directory-server call; the append layer signs with the issuing authority identity it is given, and until the request path lands the only way to revoke is a caller that holds that identity.
- C25 is not edited; no sentence of the design is rewritten by this brief's rows.
- No credential, token or key value in code, tests, fixtures, logs or documents; tests generate their keys. No name of a person and no date inside code.

## Verification

- From the repository root: sh scripts/design/gate.sh exits 0, and python3 scripts/design/validate.py docs/design/decisions.json and python3 scripts/design/validate.py docs/design/roadmap.json each exit 0.
- From the repository root, after each code row, at the gate venue (CN8): cargo fmt --all; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean, with git status showing no change from cargo fmt.
- From the repository root: rg -c 'fn \w*d013_r2_ac(1|2|3|4)(_|\b)' crates/lys-identity/tests/revocation_leaf.rs prints 4; rg -c 'fn \w*d013_r3_ac(1|2|3|4|5|6|7|8)(_|\b)' crates/lys-identity/tests/revocation_fold.rs prints 8; rg -c 'fn \w*d013_r4_ac(1|2|3)(_|\b)' crates/lys-identity/tests/revocation_append.rs prints 3; rg -c 'fn \w*d013_r5_ac(1|2|3|4|5)(_|\b)' crates/lys-identity/tests/revocation_verify.rs prints 5; rg -c 'fn \w*d013_r6_ac(1|2|3|4|5)(_|\b)' crates/lys-identity/tests/revocation_history.rs prints 5.
- For each code row, the row's first commit adds only its test file and cargo test for that file exits non-zero, and its second commit makes it pass; the review reads both commits.
- git diff --stat $(git merge-base origin/main HEAD) HEAD -- crates/lys-core crates/lys-log-store crates/lys-anchor crates/lys/Cargo.toml prints nothing.
- Each new file under crates/lys-identity/src/revocation/ holds under 500 lines of code excluding tests, comments and blank lines, and mod.rs holds only module declarations, re-exports and module docs.

