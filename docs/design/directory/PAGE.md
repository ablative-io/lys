# directory — what was asked, what it means, and what was written

## The words, as they were typed

Write the brief for this card in the directory cluster (docs/design/directory/briefs, next free id), as road step 2 of the directory design (docs/design/directory/DESIGN.md): the lys certificate gets a typed capability claim in place of today's opaque bytes, and an agent seat is issued one at spawn. This needs its own design round and its own format version, because a signed field nothing checks would be worse than none: name the new version alongside the shipped one, never a mutation of it (CLAUDE.md, Wire formats are forever), and name the verifier that reads the claim. Done when a certificate states what an agent was granted and a verifier reads it. Hold to DIRECTORY-006 R1 (no change to published cryptography) and to the adversarial-review rule for cryptographic changes. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after Tom or the lead signs it off on the card.

Rulings of the lead, Archie, given on 27 September 2026 to the run 858c0e71-e9b1-437e-9ca4-e4a4aa880284 in answer to its rounds 1 to 4. That run took every answer and then failed before writing, when Argus timed out on its session. They are settled here, and the author reopens none of them.

The non-goal 'Road step 2 onward' is amended narrowly, as on the SpiceDB brief (QZD2uagw). The certificate half of step 2, the typed capability claim and its verifier, leaves that non-goal and becomes a goal, and CN11 gets the same by one appended ruling line. Everything else in the non-goal stays. The amendment is recorded as this brief's own decision at the next free ADR id, citing STATEMENT-2026-09-22.md for where step 2 is defined.

CN1 is narrowed by one appended ruling line saying it binds DIRECTORY-001's documents-only work, and CN1 itself is not reworded. Whichever brief lands first writes that line and the others cite it.

The .1 transport and lys ca issue --claims stay exactly as shipped in 0.2.0, neither deprecated nor removed, because wire formats are forever. In place of opaque bytes means an agent's certificate carries a typed, versioned payload inside that same transport.

The claim type lives in lys-identity. The crate lys-core keeps carrying the claim as opaque bytes in its existing transport, with no change to its published formats or cryptography. The crate lys-identity defines, encodes, versions and verifies the typed payload, and the payload's version is its own, alongside, never a mutation.

The first vocabulary is narrow. The claim names the grant by its directory id, its holder, and the validity window it was issued under. The scope vocabulary waits on Tom's grant representation (DESIGN.md:58, C5), and the brief names the scope field as blocked on that decision rather than inventing it. There is one claim per certificate, as ruled on the revocation brief (zP1P2HLD).

A named verifier reads the claim and also refuses. It checks the chain, parses the claim, and refuses an action the claim does not cover, with refusal legs that assert the words, because a signed field nothing checks is worse than none. The per-call check at the door or in SpiceDB is not in this card, and belongs to QZD2uagw and to the Cambium side.

There is one certificate per enduring agent, keeping ADR-008's one certificate on the agent's file, and its subject names the agent's directory id. Issued at spawn is read as issued when the agent is provisioned and presented from its first spawn on. Per-execution credentials belong to the step-2 session card, as ruled on 6AWx1-JR.

The directory issues the certificate through lys-identity, under an issuer key the directory holds, and the brief states its custody. Neither lys (ADR-007) nor Cambium issues it. Which anchor it chains to is open for Tom (DESIGN.md:61). Tests use a test anchor, and production chaining is named as blocked on his choice.

This card delivers expiry. The validity window is in the claim, and the verifier refuses the claim past expiry, tested with an injected clock. Revocation is the DP26 fold in zP1P2HLD, and the claim carries what that fold needs, which is the certificate's identity in the log. This card names that card as its revocation. The live check is QZD2uagw's.

Issuance is a server route. The operator calls it with the agent's certificate request, and it issues one certificate for that enduring agent. Issuance never fires automatically when a grant is given. R5 names the route with its refusals for an agent that is not active, an agent that already holds a certificate, and a request whose key does not match the agent's.

The agent's key is registered with the agent and never supplied on the issuance call, because a key the caller supplies on the same call it is checked against proves nothing. DIRECTORY-003 owns the agent record's file set and is not edited here. This brief adds its own requirement, before R6, for an enrolled key, which is one Ed25519 public key per agent, keyed by the agent's id and held beside the agent record. The operator's call sets it once. Replacing it is a separate, audited act that names the old and new key, and it is never an overwrite. The issuance route compares the request's key with the enrolled key. It refuses a mismatch with key_mismatch, and an agent with no enrolled key with agent_key_not_enrolled, whose words say to enrol the agent's key first. Each refusal has its own acceptance line. The new requirement is blocked by DIRECTORY-003's agent record having landed, checked by git cat-file -e on its file, and R6 is blocked by the new requirement.

Sign-off does not ratify the format. WIRE-FORMATS.md requires a recorded human ratification, so the brief records it as Tom's and blocks the format row's build on it. The adversarial review is written into the brief before that.

The code rows wait. They are blocked on DIRECTORY-003 (the lys-identity crate existing on main) and on DIRECTORY-006 R1 (the grant), each with a check a stranger can run. The documents rows go ahead now.

The claim type is defined in crates/lys-identity/src/capability/claim.rs, because the SpiceDB brief keys its own blocker on that exact path. This brief keeps DIRECTORY-012 and RM-020, which it held first, if git ls-remote shows them still free immediately before writing. Otherwise it takes the next free id past every branch, and the brief that wrote an id first keeps it.

Rulings of the lead, Archie, given on 27 September 2026 to the run b38766d2-4b62-44ac-a335-abb04f33415d in answer to its rounds. That run took every answer and then failed before writing, when the account pool refused every session. They are settled here, and the author reopens none of them.

A new sub-component of the lys arc, never the shipped .1. The typed claim is a new format alongside the shipped one, and sharing .1 would make a hand-issued JSON claim indistinguishable from a typed one. The brief proposes 1.3.6.1.4.1.66364.2 and records it for Tom's ratification with the rest of the format, as already ruled, so the format row's build waits on that ratification. The .1 transport and lys ca issue --claims stay exactly as shipped. lys ca verify and lys inspect cert keep showing only .1, and the verifier in lys-identity is what reads the typed claim. Showing the typed claim in the lys CLI is named as its own card, not done here. Answered by Archie, lead for the identity line.

Each field the first vocabulary carries has its own refusal, and each refusal has its own acceptance leg asserting its words. A grant id other than the one the caller acts under is refused as claim_grant_mismatch. A holder other than the certificate's subject is refused as claim_holder_mismatch. An instant before the window is refused as claim_not_yet_valid, and an instant after it as claim_expired. A payload version the verifier does not know is refused as claim_version_unknown, and a payload that does not parse as claim_malformed. A refusal on scope is named as blocked on C5 and is not invented. Answered by Archie, lead for the identity line.

Issuance refuses an agent that holds a current certificate, and only that. A certificate that has expired, or whose key has been replaced, is not current, so issuance for that agent proceeds. Each issuance is its own audited event naming the certificate it follows, if there is one. The refusal for an agent holding a current certificate names that certificate and its expiry, so the operator knows when a new one can be issued. Answered by Archie, lead for the identity line.

This card builds it, because without it an operator who loses a key is stuck, and the reissue rule depends on it. Replacing an enrolled key is an operator act that records one audited event naming the agent, the old key and the new key. It never overwrites, and it marks the certificate issued under the old key as no longer current. It has its own acceptance lines, one for the replacement and one for a replacement that names an old key that is not the enrolled one, which is refused as key_replacement_mismatch. Answered by Archie, lead for the identity line.

Key the check on DIRECTORY-003's roadmap row on origin/main, since its file manifest is unwritten and this brief may not edit it. The check is a command a stranger can run, reading docs/design/roadmap.json at origin/main and requiring DIRECTORY-003's row to read landed. The brief writes that command exactly. When DIRECTORY-003's manifest names the agent record's file, a later card may re-key the check on that file. Answered by Archie, lead for the identity line.

Both are later units, and the brief names each. Presenting the certificate at spawn belongs to the step-2 session card, which builds the start command's presentation step. Showing the certificate on the agent's file under ADR-008 belongs to DIRECTORY-005, which owns the screens. This card delivers issuance, the enrolled key and its replacement, and the verifier, which is what the done line needs. Answered by Archie, lead for the identity line.

The capability claim takes .2, 1.3.6.1.4.1.66364.2, settled by Waffles at 12:12 on 27 September. A typed capability claim is an extension carried on agent certificates, so it falls within the purpose the register states for .2. The free-form claims keep .1 untouched, so a typed claim can never be read as a hand-written one and every certificate issued today reads exactly as before. This brief rewrites the register's .2 row itself, and no other card does. The row names the typed capability claim and keeps the rest of its stated purpose. The whole typed format, its fields and its encoding, is a technical decision and is not Tom's to ratify, as Waffles ruled at 12:13 on Tom's word of 12:12. The ratification WIRE-FORMATS.md asks for is the owning lead's with a second reader. When this brief lands, the lead sends its exact encoding, Apollo reads it as second reader, and Waffles records the ratification on the card with both names and the reasons. The format row then builds with no wait on Tom, and the brief does not block it on Tom. This brief also changes the WIRE-FORMATS.md line that asks for human ratification so that it names the owning lead with a second reader, in those words, so the rule and the practice agree. Answered by Archie, lead for the identity line.

The check keys on DIRECTORY-003's own execution record reading landed, not on RM-001. RM-001 links five cards, so reading it cannot say whether DIRECTORY-003 alone has landed. It is one command a stranger can run against lys main. It reads DIRECTORY-003's record and prints landed or not landed, naming the commit it read. This is the same ruling given to the provision brief in run 28d15c0d. Answered by Archie, lead for the identity line.

This brief writes no rotation requirement, and it says so in one sentence. The key's holding and reach are stated here, as the brief already does. Rotation waits on the anchor the issuer key chains to, which is open. The brief names issuer key rotation as its own card, to be written once the anchor is decided, and it names the certificates that card must keep verifiable across a rotation, which are every agent certificate issued under the current key. Answered by Archie, lead for the identity line.

A value the directory's configuration states. The configuration holds a default validity window and a maximum, with 30 days and 90 days as the shipped values. The operator may ask for a shorter window on an issuance call, and never a longer one. A request above the maximum is refused by name, naming the maximum, and a request with no window takes the default. The certificate records the window it was given. The brief records the two shipped values and their reason in an ADR at the next free id. A short window bounds how long a certificate outlives a withdrawn grant, and the maximum keeps an operator's typo from issuing a certificate that outlives its person's role. Acceptance lines cover a default issuance, a shorter request, and a longer request refused by name. One more ruling rides with this answer, from Apollo's review. The verifier takes a set of trusted issuer keys, never exactly one, and a certificate names the issuer key that signed it by that key's fingerprint, so a later rotation needs no change of format. One test gives the verifier two issuer keys and shows that it accepts a certificate under either and refuses one signed by a third. Answered by Archie, lead for the identity line.

Neither. The certificate names its issuer key in X.509's own Authority Key Identifier extension, whose keyIdentifier is the SHA-256 of the issuer key's SubjectPublicKeyInfo, the fingerprint the verifier's issuer-key set already uses, as RFC 7093 allows. That is where standard tools look, so a stranger reads it with openssl x509 -text, and the typed claim keeps only what the agent held, so its ratified format is not widened. The verifier compares the extension's key identifier with the fingerprint of the key that verified the chain, and refuses a difference or a missing extension by name as issuer_key_mismatch, naming both fingerprints. The issuer DN stays as lys-core writes it. Acceptance lines cover a matching certificate accepted, a certificate whose extension names another key refused, and one with no extension refused, each with the refusal's name. Answered by Archie, lead for the identity line.

Rulings of the lead, Archie, given on 27 September 2026 to the run 0f34978d-1555-4963-a7b9-96f12d4f0430 in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

The claim carries nothing for revocation. DIRECTORY-013's key stands: a certificate is named by the SHA-256 of its DER, computed by whoever checks it, never from anything the issuer chose. The words' sentence is corrected to say that the fold needs no field in the claim, because the certificate's identity in the log is the hash of the certificate itself. Answered by Archie, lead for the identity line.

Both sentences change, line 3 and line 208, so that each names the owning lead with a second reader and says the ratification is recorded on the card with both names and the reasons, never inferred from a build going green. D1 to D6's ratification records stay exactly as written, because they are history. Answered by Archie.

.2 stays a family arc, as docs/PEN-REGISTRATION.md:63 promises, and the typed capability claim takes 1.3.6.1.4.1.66364.2.1. This keeps the arc Waffles settled at 12:12 for the typed claim and leaves room under it for the issuer, runtime and session extensions. The brief records .2.1 in PEN-REGISTRATION.md, with that reason. The free-form claims keep .1 untouched. Answered by Archie.

Accepted. The first writer keeps an id, so ADR-062 stays with 09a6cc80 and ADR-063 with LYSCORE-001. The step-2 amendment ADR and the validity-window ADR move to the next two ADR ids free on every branch. Immediately before writing, fetch every origin ref and re-check with git ls-remote, then take the next free ids past main and every brief, draft and hand head. Record the heads read in the dev record. Every reference follows the new ids: decisions.json, design.json decisions, the goal, the non-goal, the CN1 and CN11 ruling lines, the brief's design_anchor, purpose and task, and RM-020's links and summary. The RM-020 note's claim about the highest ADR is corrected to name the heads actually read. Answered by Archie, lead for the identity line.

Accepted. C157 stays with 7494cb8b. The ratification-record item moves to the next directory-cluster C id free on every branch. Immediately before writing, fetch every origin ref and re-check with git ls-remote, then take the next free ids past main and every brief, draft and hand head. Record the heads read in the dev record. checklist.json, CHECKLIST.md, R3's checklist, the brief's checklist, the task's sentence and the RM-020 note all follow the new id, and the rendered files are re-rendered. Answered by Archie.

## What the survey found, and its angles

The words ask for one new brief in the directory cluster, carrying road step 2's certificate half. An enduring agent gets one lys certificate, issued by the directory through lys-identity. The certificate carries a typed, versioned capability claim (grant id, holder and validity window) under a new OID, 1.3.6.1.4.1.66364.2.1, beside the shipped opaque .1 transport, which does not change. A named verifier in lys-identity checks the chain, the issuer key, the claim's fields and its window, and refuses each failure by name. The brief also covers the enrolled agent key and its audited replacement, the issuance route with its refusals, the configured validity window, the adversarial review, the lead-plus-second-reader ratification (with WIRE-FORMATS.md's rule rewritten to match), the .2.1 register row, and the narrow amendments to the step-2 non-goal, CN1 and CN11. All of it follows the method and the lead's rulings, which are not reopened.

### What the tree holds

- `docs/design/directory/design.json and docs/design/directory/DESIGN.md` — The cluster this brief continues. It has 96 structure rows and CN1 to CN12. DESIGN.md:63 is the 'Road step 2 onward' non-goal the brief amends. CN1 (DESIGN.md:185, documents only) and CN11 (DESIGN.md:195, capability policy is step 2's) each get one appended ruling line. DESIGN.md:58 (grant representation, C5) and DESIGN.md:61 (anchor) are the open items the brief names as blockers. Every new file needs a structure row.
- `docs/design/directory/checklist.json, CHECKLIST.md, stories.json, USER-STORIES.md` — The brief's checklist and story ids must exist here. On this HEAD there are C1–C30 and 12 stories, but other branches reach C192, so the ratification-record item takes the next C id free on every branch.
- `docs/design/decisions.json` — Gets the two new ADRs: the step-2 amendment citing STATEMENT-2026-09-22.md:143, and the 30-day default / 90-day maximum validity window. HEAD holds ADR-001 to ADR-018; the highest id on any branch is ADR-078.
- `docs/design/roadmap.json` — The roadmap row (RM-020 as held, or the next free) links the brief and the ADRs. The DIRECTORY-003 blocker also reads this repository's brief execution record at origin/main.
- `docs/design/WIRE-FORMATS.md` — Line 3 ('Ratification is a deliberate human decision') and line 208 ('ratification remains a recorded human decision') are rewritten to name the owning lead with a second reader. The D1–D6 records stay as they are. The new format gets its decision-log row here.
- `docs/PEN-REGISTRATION.md` — Line 63 is the '.2 Reserved' row that .2.1 is recorded under (the arc stays a family arc). Line 62 is the .1 row, which stays untouched.
- `crates/lys-core/src/ca/extensions.rs` — The shipped opaque transport: encode_extension/decode_extension and LYS_OID_ARC. lys-core keeps carrying the typed payload as opaque bytes, and the file must not change.
- `crates/lys-core/src/ca/authority.rs (at origin/main fa3dd53)` — issue_certificate_for_request is the issuance path lys-identity calls. leaf_params sets no Authority Key Identifier (rcgen use_authority_key_identifier_extension defaults to false), so the ruled AKI has to travel as a custom extension. validity_window takes Utc::now(), so the issuance time cannot be injected. verify_certificate_chain_at is the injectable-clock chain check the verifier stands on. Row 6.4 (fa3dd53) made issuer_certificate_der and `openssl verify -CAfile` a stated property.
- `~/.cargo/registry/.../rcgen-0.13.2/src/lib.rs:516-531 and src/certificate.rs:766-805` — rcgen writes a SubjectKeyIdentifier on the issuer certificate equal to the first 20 bytes of SHA-256(SPKI) (KeyIdMethod::Sha256, truncated). The ruled AKI uses the full SHA-256 of the SPKI, so it will not match that SKI.
- `crates/lys/src/commands/ca.rs:62-70, crates/lys/src/cli.rs` — The shipped .1 OID (CAPABILITY_CLAIMS_COMPONENT = 1), lys ca issue --claims, and `lys inspect cert`. These stay exactly as shipped and do not show the typed claim.
- `crates/lys-identity/src/capability/claim.rs (does not exist)` — The path the SpiceDB brief (QZD2uagw) keys its blocker on. The claim type must be defined at exactly this path. crates/lys-identity is absent at origin/main.
- `docs/design/directory/briefs/DIRECTORY-003.json` — Owns the agent record and the lys-identity crate. It has no execution block at origin/main, so the ruled check prints 'not landed'. This brief must not edit it.
- `docs/design/directory/briefs/DIRECTORY-006.json R1` — The grant contract that supplies the grant's directory id. It is a proposal, not dispatched, and blocked on C5. Its 'no change to published cryptography' rule is the one this brief holds to.
- `hand/DIRECTORY-013 docs/design/directory/briefs/DIRECTORY-013.json` — The revocation card (zP1P2HLD). A certificate is named by SHA-256 of its DER. It defines an issuance leaf, and its R3 fold refuses 'revocations before issuance', so whether this card's issuance appends that leaf decides whether its certificates can be revoked.
- `scripts/design/validate.py, check-coverage.py, render-cluster.py, gate.sh, schemas/brief.schema.json` — The method gate. A brief is valid only against brief.schema.json. The DIRECTORY-003 'landed' check reads execution.status, which is one of in_flight, landed or failed.
- `docs/design/identity/STATEMENT-2026-09-22.md:78, :125, :143` — Line 143 defines step 2 ('The typed capability claim in the lys certificate, an agent seat gets one at spawn'). Line 78 says an instance CA issues a certificate when the agent is born and that rotation is an append (DP16). Line 125 names the typed claim as missing.

### What was already decided

- DIRECTORY-001 non-goal 'Road step 2 onward' (DESIGN.md:63) — Capability certificates are out of step 1. The ruled amendment moves only the typed claim and its verifier into the goals.
- CN1 — Documents only, nothing outside docs/design/directory/ and decisions.json. It gets a ruling line binding it to DIRECTORY-001's work, because this brief edits WIRE-FORMATS.md and PEN-REGISTRATION.md.
- CN11 — Live capability policy and its enforcement are step 2's. It gets the same appended ruling line as the non-goal.
- CN9 — A new module's wall needs an exact file manifest reviewed before its row starts. Sign-off serves as that review for the lys-identity/capability files.
- C5 / DESIGN.md:58 — The grant representation is OPEN for Tom, so the claim's scope field is named as blocked, not invented.
- DESIGN.md:61 — Which anchor the first agents pin to is OPEN for Tom. Production chaining and issuer-key rotation are named as blocked on it.
- P3 — Registration issues no capability certificate, so issuance is a separate operator route.
- ADR-008 — An agent's file shows its one lys certificate, or 'not issued' when no key has been supplied. Showing it on the file is DIRECTORY-005's work.
- ADR-007 — The product never runs an agent, so presentation at spawn is the step-2 session card's.
- ADR-011 — Issuance refuses an agent that is not active.
- ADR-003 — Every grant is rooted in a person, and the claim names a grant under that rule.
- DIRECTORY-006 R1 — Defines the grant contract in lys-identity without changing published cryptography or lys/delegation/v1.
- DIRECTORY-013 (hand/DIRECTORY-013) — Revocation by an appended leaf. The certificate is named by SHA-256 of its DER, there is an issuance leaf, and the fold refuses a revocation before issuance.
- WIRE-FORMATS.md:3,208 and D1–D6 — Proposed → ratified → frozen, with ratification currently worded as a human decision. D1–D6 are history and stay as written.
- PEN-REGISTRATION.md:60-64 — .1 is in use for opaque claims. .2 is reserved for agent-certificate identity and issuer extensions. Arcs are never reused.
- CLAUDE.md 'Wire formats are forever' — A new version alongside the shipped one, never a mutation. A format freezes when the first durable artifact is signed under it.

### What was measured

- Local HEAD compared with origin/main: HEAD 7b53625 is 2 commits behind origin/main fa3dd53 (row 6.4 and its hardening, 16 files, +1659/−15, including crates/lys-core/src/ca/authority.rs)
- Remote heads read (main, brief/, draft/, hand/, card/): 199 of 284 refs on origin
- Highest ids on any of those 199 heads: ADR-078, RM-048, C192, DIRECTORY-025
- DIRECTORY-012 holders: 4 drafts: 858c0e71 (this card, first written 2026-09-26T22:24:54Z), 82386e20 (revocation draft, 22:27:11Z), b38766d2, 0f34978d
- RM-020 holders: 10 heads. This card's 858c0e71 roadmap commit is dated 2026-09-26T22:59:35Z; the others (home, lys-log-store, directory/67e5c3fc) are 2026-09-27T01:12Z to 03:21Z
- ADR-062/ADR-063 holders: ADR-062 on 09a6cc80 and 0f34978d. ADR-063 on LYSCORE-001 and 4 other hand heads, plus 0f34978d
- C157 holders: 7494cb8b and 0f34978d
- crates/lys-identity at origin/main: absent (git cat-file -e fails)
- DIRECTORY-003 execution record at origin/main: none, so the ruled check prints 'not landed'
- Decisions in docs/design/decisions.json at HEAD: 18 (ADR-001 to ADR-018)
- Directory cluster at HEAD: 7 briefs, 30 checklist items, 12 stories, 96 structure rows, 12 constraints; validate and coverage both exit 0
- File sizes: DESIGN.md 196 lines, design.json 770, WIRE-FORMATS.md 208, PEN-REGISTRATION.md 67, lys-core/src/ca total 2428 (authority.rs 455, extensions.rs 97)
- rcgen version and its key-identifier length: rcgen 0.13.2. SKI and AKI key ids are the first 20 bytes of SHA-256(SPKI). Leaf AKI is off by default, and lys-core leaf_params does not turn it on
- Existing fingerprint convention in lys-core: SHA-256 over the certificate DER (certificate.rs:58,108). No issuer-key-set or SPKI fingerprint exists
- Earlier draft of this card (0f34978d): 8 requirements R1–R8. Checklist C249–C256 and C157. Anchors ADR-062 and ADR-063

### What it means for the other projects

- cambium — The per-call check at the door or in SpiceDB belongs to QZD2uagw and the Cambium side. Cambium later consumes this issuer. The card runs through Cambium's board chain: brief_card, sign-off, card_build_v3.
- aion — The workflow chain carries the card. Presenting the certificate at a seat's first spawn belongs to the step-2 session card, and whichever engine starts the seat is the one that presents it (ADR-004, ADR-007).
- method — The brief must validate against the method's brief.schema.json and pass validate.py, check-coverage.py and render-cluster.py through scripts/design/gate.sh.

### The decisions it stands on

- ADR-003 (honour) — The claim names a grant rooted in a person, and a withdrawn grant is bounded by the short window.
- ADR-007 (honour) — The directory issues but never runs an agent. Presentation at spawn is left to the session card.
- ADR-008 (honour) — One certificate per enduring agent, whose subject is the agent's directory id. Display on the file is left to DIRECTORY-005.
- ADR-011 (honour) — Issuance refuses an agent that is not active.
- ADR-004 (honour) — Issuance and verification live in lys-identity and stand without Cambium or Manifold.
-  (new) — Step-2 amendment: the certificate half of step 2 (typed claim and verifier) leaves the 'Road step 2 onward' non-goal and becomes a goal, citing STATEMENT-2026-09-22.md:143. It takes the next free ADR past ADR-078.
-  (new) — Validity window: 30-day default and 90-day maximum in the directory's configuration, with shorter requests allowed and longer ones refused by name. It takes the next free ADR after the amendment.
-  (new) — WIRE-FORMATS.md's ratification rule (lines 3 and 208) changes from 'a deliberate human decision' to the owning lead with a second reader, recorded on the card. D1–D6 are kept.

### What it requires

- The brief exists at docs/design/directory/briefs/<id>.json with its rendered .md, and scripts/design/gate.sh exits 0.
- design.json's non-goal 'Road step 2 onward' no longer covers the typed claim and its verifier, a goal names them, and CN1 and CN11 each end with one appended ruling line; neither is reworded otherwise.
- decisions.json holds the step-2 amendment ADR and the validity-window ADR at the next free ids, and every reference (goal, non-goal, ruling lines, design_anchor, purpose, task, roadmap links) uses those ids.
- PEN-REGISTRATION.md records .2.1 for the typed capability claim with its reason, keeps .2 as a family arc, and leaves the .1 row byte-identical.
- WIRE-FORMATS.md lines 3 and 208 name the owning lead with a second reader, with the ratification recorded on the card with both names and reasons; the D1–D6 rows are unchanged.
- A documents row writes the format proposal and a separate adversarial-review document constructing forgery, confusion and malleability attacks, and the format row's build is blocked on the ratification record.
- The claim type is defined in crates/lys-identity/src/capability/claim.rs, carrying grant id, holder and validity window, with scope named as blocked on C5.
- The verifier accepts a certificate signed by either of two trusted issuer keys and refuses one signed by a third.
- The verifier refuses, each by name with its own acceptance leg asserting the words: claim_grant_mismatch, claim_holder_mismatch, claim_not_yet_valid, claim_expired (tested with an injected clock), claim_version_unknown, claim_malformed, and issuer_key_mismatch for an AKI naming another key and for a missing AKI.
- An enrolled-key requirement comes before issuance: one Ed25519 key per agent, set once by the operator, with refusals key_mismatch and agent_key_not_enrolled each having its own acceptance line.
- Key replacement is one audited event naming the agent, the old key and the new key; it never overwrites, it marks the old certificate not current, and it refuses key_replacement_mismatch.
- The issuance route refuses an agent that is not active, and an agent holding a current certificate (naming that certificate and its expiry). It proceeds for an expired or key-replaced certificate, and each issuance is an audited event naming the certificate it follows.
- A default issuance gets 30 days, a shorter request is honoured, and a request over 90 days is refused naming the maximum.
- The code rows are blocked by a stranger-runnable command reading DIRECTORY-003's execution record at origin/main and printing 'landed' or 'not landed' with the commit read, and by a check on DIRECTORY-006 R1.
- The brief names its further units: CLI display of the typed claim, presentation at spawn (session card), display on the agent's file (DIRECTORY-005), and issuer-key rotation once the anchor is decided.
- The dev record lists the remote heads read when the ids were chosen.

### What must not change

- No change to lys-core's published formats or cryptography, including crates/lys-core/src/ca/extensions.rs and authority.rs.
- The .1 OID, lys ca issue --claims, lys ca verify and lys inspect cert stay exactly as shipped in 0.2.0.
- DIRECTORY-003.json and the agent record's file set are not edited.
- The D1–D6 ratification records in WIRE-FORMATS.md are not changed.
- The issuer DN stays as lys-core writes it.
- Issuance never fires automatically when a grant is given.
- No scope vocabulary is invented.
- No production anchor chaining is used; tests use a test anchor.
- No id already written first by another brief is taken (ADR-062 stays with 09a6cc80, ADR-063 with LYSCORE-001, C157 with 7494cb8b).
- CN1 and CN11 are not reworded beyond one appended ruling line each.

### What we must put in place first

- Immediately before writing: run git ls-remote on origin and re-check ADR, C, DIRECTORY and RM ids across main and every brief/, draft/ and hand/ head (199 at survey time).
- Get the lead's answer on the AKI key-identifier length, because it decides whether `openssl verify -CAfile` keeps working.
- Get the lead's answer on whether issuance appends DIRECTORY-013's issuance leaf, because it decides a dependency on DIRECTORY-013.

### The risks

- A full 32-byte AKI keyid mismatches the issuer certificate's 20-byte SKI, so `openssl verify -CAfile` (promised by row 6.4) rejects directory-issued certificates.
- The claim window and the X.509 window overlap: lys-core's chain check refuses expiry first, so claim_expired or claim_not_yet_valid may never fire on a real certificate. That makes an untested control unless each is isolated.
- Issuance time is Utc::now() inside lys-core (validity_window), so issuance tests cannot inject a clock without changing lys-core.
- Any durable certificate signed under .2.1 before ratification freezes a draft format.
- If issuance does not log a DIRECTORY-013 issuance leaf, directory certificates become unrevocable by the fold.
- The code rows are transitively blocked on Tom's C5 grant representation through DIRECTORY-006 R1, so the verifier may wait a long time for a grant id type.
- This tree is 2 commits behind origin/main, so a brief written against HEAD could miss row 6.4's lys-core issuer-certificate changes.
- Id collisions: RM-020 and DIRECTORY-012 are claimed on other heads, and dozens of parallel drafts move the next free id.

### Still open

- The ruled AKI holds the full 32-byte SHA-256 of the issuer SPKI, but lys-core's issuer certificate carries a 20-byte truncated SKI, so `openssl verify -CAfile` will stop finding the issuer. Should the AKI use the 20-byte truncated SHA-256 (RFC 7093 method 1, matching the SKI), or stay full-length and accept that openssl verify fails? The sentence of the words it stands on: "The certificate names its issuer key in X.509's own Authority Key Identifier extension, whose keyIdentifier is the SHA-256 of the issuer key's SubjectPublicKeyInfo, the fingerprint the verifier's issuer-key set already uses, as RFC 7093 allows.". Why only the lead can settle it: rcgen-0.13.2 src/lib.rs:531 truncates to 20 bytes, and certificate.rs:766-777 writes that as the issuer certificate's SKI. OpenSSL rejects an issuer whose SKI differs from the leaf's AKI keyid. Row 6.4 on main (crates/lys-core/src/ca/authority.rs:214, crates/lys/src/cli.rs:277) promises `openssl verify -CAfile` works. Also, no 'issuer-key set' exists yet, and lys's existing fingerprints are SHA-256 of the certificate DER, not of the SPKI.
- Should issuance append DIRECTORY-013's issuance leaf to the certificate log, so that the revocation fold can later revoke a directory-issued certificate? The sentence of the words it stands on: "This card names that card as its revocation.". Why only the lead can settle it: DIRECTORY-013 (hand/DIRECTORY-013) R3 refuses 'revocations before issuance' and R4 appends issuance leaves. If this card's route issues without logging, none of its certificates can be revoked by that fold. If it logs, this card depends on DIRECTORY-013's leaf format landing first. Either way, what is kept for the operator changes.
- An agent holds one certificate and the certificate holds one claim naming one grant. When an agent holds two grants, which grant does its certificate name, and how does the operator choose it on the issuance call? The sentence of the words it stands on: "There is one claim per certificate, as ruled on the revocation brief (zP1P2HLD).". Why only the lead can settle it: The issuance call carries only the certificate request, and a second certificate is refused while one is current. So an agent granted two projects could present only one, and the verifier would refuse the other grant with claim_grant_mismatch.
- Is the claim's validity window the certificate's own window (the 30/90-day issuance window), or the grant's window? The sentence of the words it stands on: "The claim names the grant by its directory id, its holder, and the validity window it was issued under.". Why only the lead can settle it: If it is the grant's window, a grant ending before the certificate makes claim_expired fire earlier than X.509 expiry. If it is the certificate's window, it duplicates notBefore/notAfter, and lys-core's own 'certificate expired' check (authority.rs verify_certificate_chain_at) fires first, so claim_expired can never be observed on a real certificate.
- DIRECTORY-012 and RM-020 are not free: later drafts (82386e20 and six other heads) also carry them, though this card wrote both first. Does this brief keep them under 'first writer keeps', or take DIRECTORY-026 and RM-049? The sentence of the words it stands on: "This brief keeps DIRECTORY-012 and RM-020, which it held first, if git ls-remote shows them still free immediately before writing.". Why only the lead can settle it: git ls-remote at survey time shows DIRECTORY-012 on draft/directory/82386e20 (written 2026-09-26T22:27:11Z) and RM-020 on brief/home/0e229c29, brief/home/879bf06e, brief/home/ea6ad356, brief/directory/67e5c3fc, draft/home/d5a29dc7 and draft/lys-log-store/e8440f97. That fails the 'still free' condition while the 'held first' condition holds. The earlier runs also left drafts, although the words say they 'failed before writing'.

### The units beyond the first

- Show the typed capability claim in the lys CLI — Ruled as its own card. lys ca verify and lys inspect cert keep showing only .1 here.
- Present the agent's certificate at spawn (step-2 session card) — This is the start command's presentation step and per-execution credentials, ruled to the session card (6AWx1-JR).
- Show the certificate on the agent's file (DIRECTORY-005) — ADR-008's screen belongs to DIRECTORY-005, which owns the screens.
- Rotate the directory's issuer key — Waits on Tom's anchor choice (DESIGN.md:61) and must keep every agent certificate issued under the current key verifiable.
- Chain the issuer key to the production anchor — Blocked on Tom's anchor decision. Tests use a test anchor until then.
- Add the scope field and its refusal to the claim — Blocked on Tom's grant representation (C5, DESIGN.md:58). It needs a new payload version alongside v1.
- Check the claim per call at the door or in SpiceDB (QZD2uagw) — The live check is the SpiceDB brief's and the Cambium side's, not this card's.

### The smallest complete shape

One brief in the directory cluster (DIRECTORY-012 or the next free id), with its rendered markdown, the design.json amendments (goal, narrowed non-goal, CN1 and CN11 ruling lines, structure rows), two ADRs in decisions.json, the checklist item and stories, and a roadmap row. The brief's documents rows can land after sign-off: the format proposal with the WIRE-FORMATS.md rule change and the .2.1 register row, then the adversarial review, then the ratification record. The code rows sit in the same brief, blocked by checks: the claim type at capability/claim.rs, the enrolled key, the issuance route with the window configuration, key replacement, and the named verifier with every refusal leg.

## The roadmap row

- **RM-056** — Carry a typed capability claim in an agent's certificate and read it with a named verifier (feature, idea)
- Summary: Road step 2's certificate half (docs/design/identity/STATEMENT-2026-09-22.md, The road), brought into the directory cluster by ADR-093: a design round proposes lys/agent-capability/v1, a typed payload with its own version under the proposed OID 1.3.6.1.4.1.66364.2.1, the first sub-arc of the .2 family arc, alongside the shipped .1 transport and never a mutation of it; a second party attacks it before the owning lead ratifies it with a second reader, and the ratification is written into the decision log and the register's .2 row; then lys-identity defines the claim (the holder and every grant held at issuance, each with its window as it stood then, and nothing for revocation, the issuer or a window of its own), the operator enrols one Ed25519 key per agent, the directory issues an enduring agent its one certificate through an operator-called route under an issuer key in its custody, named in the certificate's Authority Key Identifier by its 20-byte RFC 7093 keyid, for a configured validity window of 30 days by default and 90 at most (ADR-094), and appends it to the certificate log as DIRECTORY-013's issuance leaf so that the revocation fold can revoke it, the operator can replace an enrolled key by an audited act, and verify_agent_capability checks the chain against a set of trusted issuer keys, reads the claim and refuses what it does not cover, and refuses the certificate past its own expiry. lys-core's published formats and cryptography do not change.
- Asked by: tom on 2026-09-27T14:28:00+10:00
- Context: The capability-claim card on the Lys board, written into DIRECTORY-031 by the design-system author rounds at main 7b53625 (origin/main at fa3dd53 when the ids were chosen). The quote is the card's words with the rulings of the lead, Archie, given on 27 September 2026 to runs 858c0e71, b38766d2 and 0f34978d, carried in the brief as settled; the later rulings supersede the earlier ones where they differ. His answers to this round are carried the same way: the Authority Key Identifier's keyIdentifier is the 20-byte truncated SHA-256 of the issuer's SubjectPublicKeyInfo (RFC 7093 method 1), matching the SubjectKeyIdentifier lys-core's issuer certificate carries so that openssl verify -CAfile keeps finding the issuer, and that 20-byte keyid is a new issuer-key fingerprint, not lys's existing SHA-256 of a certificate's DER; issuance appends DIRECTORY-013's issuance leaf, so this brief is blocked by DIRECTORY-013 landing on lys main, and an issuance whose leaf cannot be appended is refused by name and returns no certificate; the claim names every grant the agent holds at issuance, one claim per certificate, the operator choosing nothing on the issuance call, a verifier refusing a grant the list does not hold with claim_grant_mismatch, and the list never presented as live grants; the claim carries no validity window of its own and claim_expired is removed, the certificate's notBefore and notAfter governing its life under lys-core's certificate-expired check, and each listed grant carries its window as it stood at issuance as a record, not a live check; and this brief takes the next free ids, DIRECTORY-031 and RM-056, the sentence about keeping DIRECTORY-012 and RM-020 being withdrawn. His answers to the round after carry the same way: claim_not_yet_valid goes with claim_expired, an instant before notBefore being refused only as certificate_chain_invalid with lys-core's `certificate not yet valid` reason; an agent holding no grant at issuance is issued a certificate whose claim lists no grants, an empty array, because the certificate proves who the agent is and what it may do is always asked of Access; a listed grant whose window has no end encodes its end as absent, starts_at always present and ends_at present only when the grant has an end, following DIRECTORY-006's GrantWindow; and a certificate that DIRECTORY-013's fold holds revoked is not current, so issuance may proceed for that agent, as after expiry or key replacement.
- Quote: Write the brief for this card in the directory cluster (docs/design/directory/briefs, next free id), as road step 2 of the directory design (docs/design/directory/DESIGN.md): the lys certificate gets a typed capability claim in place of today's opaque bytes, and an agent seat is issued one at spawn. This needs its own design round and its own format version, because a signed field nothing checks would be worse than none: name the new version alongside the shipped one, never a mutation of it (CLAUDE.md, Wire formats are forever), and name the verifier that reads the claim. Done when a certificate states what an agent was granted and a verifier reads it. Hold to DIRECTORY-006 R1 (no change to published cryptography) and to the adversarial-review rule for cryptographic changes. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). If a sentence of DESIGN.md or CHECKLIST.md is open or contradicted by the repository as it stands, quote it as a question for the lead rather than rewriting the design around it. The card is built from this brief only after Tom or the lead signs it off on the card.

Rulings of the lead, Archie, given on 27 September 2026 to the run 858c0e71-e9b1-437e-9ca4-e4a4aa880284 in answer to its rounds 1 to 4. That run took every answer and then failed before writing, when Argus timed out on its session. They are settled here, and the author reopens none of them.

The non-goal 'Road step 2 onward' is amended narrowly, as on the SpiceDB brief (QZD2uagw). The certificate half of step 2, the typed capability claim and its verifier, leaves that non-goal and becomes a goal, and CN11 gets the same by one appended ruling line. Everything else in the non-goal stays. The amendment is recorded as this brief's own decision at the next free ADR id, citing STATEMENT-2026-09-22.md for where step 2 is defined.

CN1 is narrowed by one appended ruling line saying it binds DIRECTORY-001's documents-only work, and CN1 itself is not reworded. Whichever brief lands first writes that line and the others cite it.

The .1 transport and lys ca issue --claims stay exactly as shipped in 0.2.0, neither deprecated nor removed, because wire formats are forever. In place of opaque bytes means an agent's certificate carries a typed, versioned payload inside that same transport.

The claim type lives in lys-identity. The crate lys-core keeps carrying the claim as opaque bytes in its existing transport, with no change to its published formats or cryptography. The crate lys-identity defines, encodes, versions and verifies the typed payload, and the payload's version is its own, alongside, never a mutation.

The first vocabulary is narrow. The claim names the grant by its directory id, its holder, and the validity window it was issued under. The scope vocabulary waits on Tom's grant representation (DESIGN.md:58, C5), and the brief names the scope field as blocked on that decision rather than inventing it. There is one claim per certificate, as ruled on the revocation brief (zP1P2HLD).

A named verifier reads the claim and also refuses. It checks the chain, parses the claim, and refuses an action the claim does not cover, with refusal legs that assert the words, because a signed field nothing checks is worse than none. The per-call check at the door or in SpiceDB is not in this card, and belongs to QZD2uagw and to the Cambium side.

There is one certificate per enduring agent, keeping ADR-008's one certificate on the agent's file, and its subject names the agent's directory id. Issued at spawn is read as issued when the agent is provisioned and presented from its first spawn on. Per-execution credentials belong to the step-2 session card, as ruled on 6AWx1-JR.

The directory issues the certificate through lys-identity, under an issuer key the directory holds, and the brief states its custody. Neither lys (ADR-007) nor Cambium issues it. Which anchor it chains to is open for Tom (DESIGN.md:61). Tests use a test anchor, and production chaining is named as blocked on his choice.

This card delivers expiry. The validity window is in the claim, and the verifier refuses the claim past expiry, tested with an injected clock. Revocation is the DP26 fold in zP1P2HLD, and the claim carries what that fold needs, which is the certificate's identity in the log. This card names that card as its revocation. The live check is QZD2uagw's.

Issuance is a server route. The operator calls it with the agent's certificate request, and it issues one certificate for that enduring agent. Issuance never fires automatically when a grant is given. R5 names the route with its refusals for an agent that is not active, an agent that already holds a certificate, and a request whose key does not match the agent's.

The agent's key is registered with the agent and never supplied on the issuance call, because a key the caller supplies on the same call it is checked against proves nothing. DIRECTORY-003 owns the agent record's file set and is not edited here. This brief adds its own requirement, before R6, for an enrolled key, which is one Ed25519 public key per agent, keyed by the agent's id and held beside the agent record. The operator's call sets it once. Replacing it is a separate, audited act that names the old and new key, and it is never an overwrite. The issuance route compares the request's key with the enrolled key. It refuses a mismatch with key_mismatch, and an agent with no enrolled key with agent_key_not_enrolled, whose words say to enrol the agent's key first. Each refusal has its own acceptance line. The new requirement is blocked by DIRECTORY-003's agent record having landed, checked by git cat-file -e on its file, and R6 is blocked by the new requirement.

Sign-off does not ratify the format. WIRE-FORMATS.md requires a recorded human ratification, so the brief records it as Tom's and blocks the format row's build on it. The adversarial review is written into the brief before that.

The code rows wait. They are blocked on DIRECTORY-003 (the lys-identity crate existing on main) and on DIRECTORY-006 R1 (the grant), each with a check a stranger can run. The documents rows go ahead now.

The claim type is defined in crates/lys-identity/src/capability/claim.rs, because the SpiceDB brief keys its own blocker on that exact path. This brief keeps DIRECTORY-012 and RM-020, which it held first, if git ls-remote shows them still free immediately before writing. Otherwise it takes the next free id past every branch, and the brief that wrote an id first keeps it.

Rulings of the lead, Archie, given on 27 September 2026 to the run b38766d2-4b62-44ac-a335-abb04f33415d in answer to its rounds. That run took every answer and then failed before writing, when the account pool refused every session. They are settled here, and the author reopens none of them.

A new sub-component of the lys arc, never the shipped .1. The typed claim is a new format alongside the shipped one, and sharing .1 would make a hand-issued JSON claim indistinguishable from a typed one. The brief proposes 1.3.6.1.4.1.66364.2 and records it for Tom's ratification with the rest of the format, as already ruled, so the format row's build waits on that ratification. The .1 transport and lys ca issue --claims stay exactly as shipped. lys ca verify and lys inspect cert keep showing only .1, and the verifier in lys-identity is what reads the typed claim. Showing the typed claim in the lys CLI is named as its own card, not done here. Answered by Archie, lead for the identity line.

Each field the first vocabulary carries has its own refusal, and each refusal has its own acceptance leg asserting its words. A grant id other than the one the caller acts under is refused as claim_grant_mismatch. A holder other than the certificate's subject is refused as claim_holder_mismatch. An instant before the window is refused as claim_not_yet_valid, and an instant after it as claim_expired. A payload version the verifier does not know is refused as claim_version_unknown, and a payload that does not parse as claim_malformed. A refusal on scope is named as blocked on C5 and is not invented. Answered by Archie, lead for the identity line.

Issuance refuses an agent that holds a current certificate, and only that. A certificate that has expired, or whose key has been replaced, is not current, so issuance for that agent proceeds. Each issuance is its own audited event naming the certificate it follows, if there is one. The refusal for an agent holding a current certificate names that certificate and its expiry, so the operator knows when a new one can be issued. Answered by Archie, lead for the identity line.

This card builds it, because without it an operator who loses a key is stuck, and the reissue rule depends on it. Replacing an enrolled key is an operator act that records one audited event naming the agent, the old key and the new key. It never overwrites, and it marks the certificate issued under the old key as no longer current. It has its own acceptance lines, one for the replacement and one for a replacement that names an old key that is not the enrolled one, which is refused as key_replacement_mismatch. Answered by Archie, lead for the identity line.

Key the check on DIRECTORY-003's roadmap row on origin/main, since its file manifest is unwritten and this brief may not edit it. The check is a command a stranger can run, reading docs/design/roadmap.json at origin/main and requiring DIRECTORY-003's row to read landed. The brief writes that command exactly. When DIRECTORY-003's manifest names the agent record's file, a later card may re-key the check on that file. Answered by Archie, lead for the identity line.

Both are later units, and the brief names each. Presenting the certificate at spawn belongs to the step-2 session card, which builds the start command's presentation step. Showing the certificate on the agent's file under ADR-008 belongs to DIRECTORY-005, which owns the screens. This card delivers issuance, the enrolled key and its replacement, and the verifier, which is what the done line needs. Answered by Archie, lead for the identity line.

The capability claim takes .2, 1.3.6.1.4.1.66364.2, settled by Waffles at 12:12 on 27 September. A typed capability claim is an extension carried on agent certificates, so it falls within the purpose the register states for .2. The free-form claims keep .1 untouched, so a typed claim can never be read as a hand-written one and every certificate issued today reads exactly as before. This brief rewrites the register's .2 row itself, and no other card does. The row names the typed capability claim and keeps the rest of its stated purpose. The whole typed format, its fields and its encoding, is a technical decision and is not Tom's to ratify, as Waffles ruled at 12:13 on Tom's word of 12:12. The ratification WIRE-FORMATS.md asks for is the owning lead's with a second reader. When this brief lands, the lead sends its exact encoding, Apollo reads it as second reader, and Waffles records the ratification on the card with both names and the reasons. The format row then builds with no wait on Tom, and the brief does not block it on Tom. This brief also changes the WIRE-FORMATS.md line that asks for human ratification so that it names the owning lead with a second reader, in those words, so the rule and the practice agree. Answered by Archie, lead for the identity line.

The check keys on DIRECTORY-003's own execution record reading landed, not on RM-001. RM-001 links five cards, so reading it cannot say whether DIRECTORY-003 alone has landed. It is one command a stranger can run against lys main. It reads DIRECTORY-003's record and prints landed or not landed, naming the commit it read. This is the same ruling given to the provision brief in run 28d15c0d. Answered by Archie, lead for the identity line.

This brief writes no rotation requirement, and it says so in one sentence. The key's holding and reach are stated here, as the brief already does. Rotation waits on the anchor the issuer key chains to, which is open. The brief names issuer key rotation as its own card, to be written once the anchor is decided, and it names the certificates that card must keep verifiable across a rotation, which are every agent certificate issued under the current key. Answered by Archie, lead for the identity line.

A value the directory's configuration states. The configuration holds a default validity window and a maximum, with 30 days and 90 days as the shipped values. The operator may ask for a shorter window on an issuance call, and never a longer one. A request above the maximum is refused by name, naming the maximum, and a request with no window takes the default. The certificate records the window it was given. The brief records the two shipped values and their reason in an ADR at the next free id. A short window bounds how long a certificate outlives a withdrawn grant, and the maximum keeps an operator's typo from issuing a certificate that outlives its person's role. Acceptance lines cover a default issuance, a shorter request, and a longer request refused by name. One more ruling rides with this answer, from Apollo's review. The verifier takes a set of trusted issuer keys, never exactly one, and a certificate names the issuer key that signed it by that key's fingerprint, so a later rotation needs no change of format. One test gives the verifier two issuer keys and shows that it accepts a certificate under either and refuses one signed by a third. Answered by Archie, lead for the identity line.

Neither. The certificate names its issuer key in X.509's own Authority Key Identifier extension, whose keyIdentifier is the SHA-256 of the issuer key's SubjectPublicKeyInfo, the fingerprint the verifier's issuer-key set already uses, as RFC 7093 allows. That is where standard tools look, so a stranger reads it with openssl x509 -text, and the typed claim keeps only what the agent held, so its ratified format is not widened. The verifier compares the extension's key identifier with the fingerprint of the key that verified the chain, and refuses a difference or a missing extension by name as issuer_key_mismatch, naming both fingerprints. The issuer DN stays as lys-core writes it. Acceptance lines cover a matching certificate accepted, a certificate whose extension names another key refused, and one with no extension refused, each with the refusal's name. Answered by Archie, lead for the identity line.

Rulings of the lead, Archie, given on 27 September 2026 to the run 0f34978d-1555-4963-a7b9-96f12d4f0430 in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

The claim carries nothing for revocation. DIRECTORY-013's key stands: a certificate is named by the SHA-256 of its DER, computed by whoever checks it, never from anything the issuer chose. The words' sentence is corrected to say that the fold needs no field in the claim, because the certificate's identity in the log is the hash of the certificate itself. Answered by Archie, lead for the identity line.

Both sentences change, line 3 and line 208, so that each names the owning lead with a second reader and says the ratification is recorded on the card with both names and the reasons, never inferred from a build going green. D1 to D6's ratification records stay exactly as written, because they are history. Answered by Archie.

.2 stays a family arc, as docs/PEN-REGISTRATION.md:63 promises, and the typed capability claim takes 1.3.6.1.4.1.66364.2.1. This keeps the arc Waffles settled at 12:12 for the typed claim and leaves room under it for the issuer, runtime and session extensions. The brief records .2.1 in PEN-REGISTRATION.md, with that reason. The free-form claims keep .1 untouched. Answered by Archie.

Accepted. The first writer keeps an id, so ADR-062 stays with 09a6cc80 and ADR-063 with LYSCORE-001. The step-2 amendment ADR and the validity-window ADR move to the next two ADR ids free on every branch. Immediately before writing, fetch every origin ref and re-check with git ls-remote, then take the next free ids past main and every brief, draft and hand head. Record the heads read in the dev record. Every reference follows the new ids: decisions.json, design.json decisions, the goal, the non-goal, the CN1 and CN11 ruling lines, the brief's design_anchor, purpose and task, and RM-020's links and summary. The RM-020 note's claim about the highest ADR is corrected to name the heads actually read. Answered by Archie, lead for the identity line.

Accepted. C157 stays with 7494cb8b. The ratification-record item moves to the next directory-cluster C id free on every branch. Immediately before writing, fetch every origin ref and re-check with git ls-remote, then take the next free ids past main and every brief, draft and hand head. Record the heads read in the dev record. checklist.json, CHECKLIST.md, R3's checklist, the brief's checklist, the task's sentence and the RM-020 note all follow the new id, and the rendered files are re-rendered. Answered by Archie.
- Cluster: directory; briefs: DIRECTORY-031
- Notes: Ids, re-checked immediately before this round's write by running git ls-remote origin (249 refs under refs/heads), then fetching every origin branch (git fetch origin '+refs/heads/*'), and reading the 207 heads it listed under main, brief/, draft/, hand/ and card/ (1, 46, 101, 32 and 27 heads), each of whose sha matched the ls-remote listing, with origin/main at fa3dd53: the highest ids on any of them were ADR-088 (draft/directory/8122eb55), RM-051, C228 and DIRECTORY-028 (draft/directory/73184e72). DIRECTORY-026, ADR-079, ADR-080, C193 and RM-049, which this card's earlier draft wrote, were already held by the earlier drafts draft/directory/07bad7e0 and draft/directory/73184e72, so they were given up, and, under the lead's ruling that this brief takes the next free ids and the first writer keeps an id, this brief is DIRECTORY-031, this row RM-056, the step-2 amendment ADR-093, the validity-window decision ADR-094 and the ratification record's item C251. ADR-062 stays with 09a6cc80, ADR-063 with LYSCORE-001 and C157 with 7494cb8b; DIRECTORY-012 and RM-020 stay with the heads that hold them. C249 to C256 and S113 to S115 appear only on this card's own draft heads (858c0e71, b38766d2, 0f34978d) and are kept. Every row main holds is unchanged. Further units, left for later and not written: Show the typed capability claim in the lys CLI; Present the agent's certificate at spawn (step-2 session card); Show the certificate on the agent's file (DIRECTORY-005); Rotate the directory's issuer key; Chain the issuer key to the production anchor; Add the scope field and its refusal to the claim; Check the claim per call at the door or in SpiceDB (QZD2uagw).

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
- ADR-093 — The directory cluster carries the certificate half of road step 2 — Amend narrowly, as on the SpiceDB brief (QZD2uagw): the certificate half of step 2, the typed capability claim and its verifier, leaves the 'Road step 2 onward' non-goal and becomes a goal of this cluster, and CN11 gains one appended ruling line saying the same; everything else in the non-goal stays out. CN1 gains one appended ruling line saying it binds DIRECTORY-001's documents-only work, and its own words are not changed. Rejected: leaving the brief under the unamended non-goal, which would make the cluster's design deny the work its own brief carries; widening the amendment to the rest of step 2 (SpiceDB, the per-call door check, per-execution credentials); and rewording CN1 or CN11 themselves.
- ADR-094 — An agent certificate's validity window comes from the directory's configuration: 30 days by default, 90 days at most — The directory's configuration holds a default validity window and a maximum, shipped as 30 days (2592000 seconds) and 90 days (7776000 seconds). An issuance call with no window takes the default; the operator may ask for a shorter window, never a longer one, and a request above the maximum is refused by name, lifetime_above_maximum, naming the maximum. The certificate records the window it was given as its own notBefore and notAfter. A verifier takes a set of trusted issuer keys, never exactly one, and the certificate names the issuer key that signed it in X.509's Authority Key Identifier by its issuer-key fingerprint, the first 20 bytes of the SHA-256 of that key's SubjectPublicKeyInfo (RFC 7093 method 1, matching the SubjectKeyIdentifier of lys-core's issuer certificate), so a later rotation of the issuer key needs no change of format. Rejected: an operator-supplied lifetime with no default and no bound, which lets a typo outlive a withdrawn grant; a window fixed in code with no configuration; and a verifier bound to exactly one issuer key, which would make every rotation a format change.

## Goals

- Every open row of IDENTITY-001 (02, 04, 03, 05) exists as a valid design-system brief in this cluster, DIRECTORY-002 to DIRECTORY-005, in its dependency order, every ID001 acceptance identifier kept.
- The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement of the row that owns it, with acceptance criteria; where the sources do not settle the row, it is recorded open for Tom in DIRECTORY-003 with its criteria drafted.
- Every decision still open for Tom is recorded as open and decided nowhere in this cluster.
- The two live demonstrations to Tom, ID001_LINK_LIVE and ID001_DIRECTORY_LIVE, stay hold points a loop completion never replaces (CN6).
- DIRECTORY-006 makes the grant/refusal journey enforceable and binds its acceptance to the reviewed mock-up, without rewriting the historical IDENTITY-001 record.
- Road step 2's certificate half (defined in docs/design/identity/STATEMENT-2026-09-22.md, The road; brought into this cluster by ADR-093): an enduring agent's certificate states what it was granted at issuance in a typed, versioned claim, lys/agent-capability/v1, a new format under its own proposed OID 1.3.6.1.4.1.66364.2.1 alongside the unchanged shipped .1 extension transport, and a named verifier, verify_agent_capability, checks the chain against a set of trusted issuer keys, reads the claim and refuses what it does not cover (DIRECTORY-031).

## Non-Goals

- Rows 06 (connect Cambium) and 07 (gate, install and demonstrate the release) — They change the Cambium repository and depend on IDENTITY-002, the upstream release rebase; they need a Cambium cluster or an agreed cross-repository arrangement first.
- Row 03's changes inside the Rauthy fork — The fork is its own repository under vendor/rauthy; a row whose files live there needs its own brief in the fork, which does not yet exist and blocks DIRECTORY-004. DIRECTORY-004 names the fork files as work with their owner, not as files of this repository.
- The grant representation: how one grant records who may exercise it, whether it may be passed on (person, agent or nobody), what it derives from, and whether it can be bounded. OPEN for Tom. — The statement leaves the exact delegation schema unsettled (docs/design/identity/STATEMENT-2026-09-22.md:21; ADR-003); AGENT-PARITY-2026-09-23's questions (docs/design/identity/AGENT-PARITY-2026-09-23.md:17-23) are inputs to it, not answers.
- Suspension semantics: whether suspending a person also ends their sign-in session at Rauthy or only makes our checks refuse, and what else stops with a suspended identity. OPEN for Tom. — The lifecycle document asks it of the room (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:99-100, docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:104-105) and its states are only proposed (ADR-011); two systems, one decision.
- The name of the identity service. OPEN for Tom. — The statement lists it as not decided (docs/design/identity/STATEMENT-2026-09-22.md:192); no crate or directory name selects it.
- Which anchor the first agents pin to: our hosted anchor, a self-hosted one, or both. OPEN for Tom. — Lys leaves it as a product decision and so does the statement (docs/design/identity/STATEMENT-2026-09-22.md:193).
- A nightly Rauthy base versus waiting for an upstream release carrying #1696 and #1728. OPEN for Tom. — Waffles' ruling of 15:36:25: if upstream has published no such release by the row 05 showing, Tom decides (docs/design/identity/briefs/IDENTITY-001.json:18; docs/design/identity/STATEMENT-2026-09-22.md:177); rows 02 to 05 run on v0.36.2 meanwhile.
- Road step 2 onward, except its certificate half, the typed capability claim and its verifier, which ADR-093 makes a goal: arbitrary grants and their enforcement, session launch and stop, credential handles, memory, context assembly, lanterns and anchoring in production — Revision 5 keeps them out of step 1 (docs/design/identity/briefs/IDENTITY-001.json:29-30); ADR-007 (the start command) and ADR-008 (the certificate on an agent's file) govern what the step-1 screens do not present as working. Step 2 is defined in docs/design/identity/STATEMENT-2026-09-22.md (The road); only its certificate half left this non-goal, under ADR-093.
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
| `docs/design/directory/briefs/DIRECTORY-031.json` | the typed capability claim in an agent's certificate and its verifier: road step 2's certificate half | DIRECTORY-031 |
| `docs/design/directory/briefs/DIRECTORY-031.md` | rendered markdown | DIRECTORY-031 |
| `docs/design/identity/CAPABILITY-CLAIM.md` | the design round's proposal for lys/agent-capability/v1 under the proposed OID 1.3.6.1.4.1.66364.2.1: transport, assertion, encoding, issuer key identifier, scope, verifier check, rendering consumer, revocation, issuer and anchor, status | DIRECTORY-031 |
| `docs/design/identity/CAPABILITY-CLAIM-REVIEW.md` | the adversarial review of the capability-claim draft, by a party other than its author, before the owning lead's ratification with a second reader | DIRECTORY-031 |
| `docs/design/WIRE-FORMATS.md` | the wire-format register; gains one line pointing at the capability-claim proposal and its PROPOSED decision-log row, which reads RATIFIED once the ratification is recorded on the card (R3), and both of its ratification sentences name the owning lead with a second reader; D1 to D6 and every other existing line unchanged | DIRECTORY-031 |
| `docs/PEN-REGISTRATION.md` | the sub-arc register under 1.3.6.1.4.1.66364; its .2 row keeps .2 a family arc for agent-certificate extensions, names .2.1 for the typed capability claim lys/agent-capability/v1 and keeps the rest of its stated purpose; its status cell stays Reserved until the ratification is recorded and then reads In use (.2.1) (R3); the .1 and .3+ rows unchanged | DIRECTORY-031 |
| `crates/lys-identity/Cargo.toml` | the directory crate's manifest; gains the capability claim's encoding dependency from the workspace lockfile | DIRECTORY-031 |
| `crates/lys-identity/src/capability/mod.rs` | declarations and re-exports only | DIRECTORY-031 |
| `crates/lys-identity/src/capability/claim.rs` | the typed lys/agent-capability/v1 claim: holder id and every grant held at issuance, each with its grant id and its window as it stood then | DIRECTORY-031 |
| `crates/lys-identity/src/capability/encoding.rs` | canonical encode and strict decode of the claim: claim_malformed, claim_version_unknown | DIRECTORY-031 |
| `crates/lys-identity/src/capability/error.rs` | named capability errors, never key or seed bytes | DIRECTORY-031 |
| `crates/lys-identity/src/capability/verify.rs` | verify_agent_capability: the signing key from the trusted set first, then the Authority Key Identifier, the claim, holder, listed grant and the certificate's own window, each refusal named | DIRECTORY-031 |
| `crates/lys-identity/src/capability/issue.rs` | the directory's issuance of an enduring agent's one certificate carrying one claim and the Authority Key Identifier of its issuer key, appended to the certificate log as the revocation card's issuance leaf, refusing an agent that holds a current certificate, recorded as one directory event naming the certificate it follows | DIRECTORY-031 |
| `crates/lys-identity/tests/capability_encoding.rs` | the pinned vector and the counted decode refusals | DIRECTORY-031 |
| `crates/lys-identity/tests/capability_verify.rs` | the verifier's acceptance, counted refusal legs and drift injections | DIRECTORY-031 |
| `crates/lys-identity/tests/capability_issue.rs` | issuance, reissue after expiry and its counted refusals | DIRECTORY-031 |
| `crates/lys-identity/src/agent_key.rs` | an agent's one enrolled Ed25519 public key, keyed by its directory id beside the agent record, enrolled once by the operator | DIRECTORY-031 |
| `crates/lys-identity/tests/agent_key.rs` | enrolment and its counted refusals | DIRECTORY-031 |
| `crates/lys-identity-server/src/lib.rs` | the directory service's crate root; declares the issuer key, key enrolment, key replacement and issuance route modules | DIRECTORY-031 |
| `crates/lys-identity-server/src/issuer_key.rs` | custody of the directory's issuer key: restricted private file, Zeroizing seed, redacted Debug | DIRECTORY-031 |
| `crates/lys-identity-server/tests/issuer_key.rs` | issuer key loading, counted refusals and redaction | DIRECTORY-031 |
| `crates/lys-identity-server/src/agent_key_route.rs` | the operator's enrol route POST /identity/agents/{agent_id}/key and its named refusals | DIRECTORY-031 |
| `crates/lys-identity-server/tests/agent_key_route.rs` | the enrol route's acceptance and counted refusals | DIRECTORY-031 |
| `crates/lys-identity-server/src/certificate_route.rs` | the directory's issuance route POST /identity/agents/{agent_id}/certificate: the operator's call with the agent's certificate-signing request, its named refusals | DIRECTORY-031 |
| `crates/lys-identity-server/tests/certificate_route.rs` | the issuance route's acceptance and counted refusals | DIRECTORY-031 |
| `crates/lys-identity/src/agent_key_replacement.rs` | the audited replacement of an agent's enrolled key, one event naming the old and new key, never an overwrite | DIRECTORY-031 |
| `crates/lys-identity/tests/agent_key_replacement.rs` | replacement, reissue under the new key and the counted refusals | DIRECTORY-031 |
| `crates/lys-identity-server/src/agent_key_replacement_route.rs` | the operator's replacement route POST /identity/agents/{agent_id}/key/replacement and its named refusals | DIRECTORY-031 |
| `crates/lys-identity-server/tests/agent_key_replacement_route.rs` | the replacement route's acceptance and counted refusals | DIRECTORY-031 |

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
- `crates/lys-core/src/ca/extensions.rs` — the shipped extension transport: LYS_OID_ARC, encode_extension and decode_extension carry opaque bytes; DIRECTORY-031 uses it unchanged; read, never changed
- `crates/lys-core/src/ca/authority.rs` — issue_certificate_for_request and verify_certificate_chain_at, which DIRECTORY-031 issues and verifies through; read, never changed

## Constraints

- **CN1** — Documents only: nothing outside docs/design/directory/ and docs/design/decisions.json is created or modified; the IDENTITY-001 files are not changed. Ruling (ADR-093): CN1 binds DIRECTORY-001's documents-only work.
- **CN2** — Development isolation: rows 02 to 05 use only disposable test identities and test provider registrations; no production tokens, real business sign-in or live Cambium participant migration.
- **CN3** — Every path written in a document of this cluster is relative to the repository root, whatever directory a session starts in; a command runs from its own tree and spells its paths from there.
- **CN4** — No structure row or files entry carries a root token; a file in another repository is named in a requirement's spec with its owner.
- **CN5** — A live demonstration to Tom is never an acceptance criterion of a loop requirement; it is a verification step a person performs after the row lands.
- **CN6** — The live demonstrations ID001_LINK_LIVE (after row 03) and ID001_DIRECTORY_LIVE (after row 05) are mandatory operator hold points: the brief that follows each is blocked by it until Tom's demonstration receipt is recorded; a loop completion never stands in for one.
- **CN7** — Revision 5's ceiling stands: 48 focused implementer hours for IDENTITY-001 (row 01 1.5, closed; 02 8; 04 10; 03 10; 05 6; 06 4; 07 5; total 44.5, contingency 3.5), with IDENTITY-002's 4 hours outside it. An overrun is reported as soon as it is known, and Waffles takes any ceiling change to Tom (docs/design/identity/briefs/IDENTITY-001.json:24).
- **CN8** — One implementer, one row in implementation and one gate invocation at a time in this lane; release builds, checks and tests run through the gate workflow at the venue, and a development exception never bypasses it (docs/design/identity/briefs/IDENTITY-001.json:25-27, docs/design/identity/briefs/IDENTITY-001.json:135).
- **CN9** — A row that needs a file outside its wall stops and names it, and the reviewer approves a brief revision before that file is edited; a directory wall for a wholly new module allows only its named responsibility and needs an exact file manifest reviewed before its row starts (docs/design/identity/briefs/IDENTITY-001.json:28).
- **CN10** — Rows 02 to 05 run on Rauthy v0.36.2 under Waffles' ruling of 15:36:25; each development install checks current releases and advisories and records the accepted exception; IDENTITY-001-UPSTREAM-AUTH-STATE binds real sign-in and install, rows 06 and 07 (docs/design/identity/briefs/IDENTITY-001.json:18, docs/design/identity/briefs/IDENTITY-001.json:46).
- **CN11** — Step 1 is directory records, sign-in and the minimum signed identity audit. SpiceDB is installed and checked in row 02 and enforces nothing in step 1; live capability policy and its enforcement are step 2's, and running SpiceDB is not permission enforcement (docs/design/identity/briefs/IDENTITY-001.json:29-30). Ruling (ADR-093): the certificate half of step 2, the typed capability claim and its verifier, is a goal of this cluster, carried by DIRECTORY-031; the rest of step 2 stays out.
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
id: DIRECTORY-031
cluster: directory
title: Carry a typed capability claim in an agent's certificate and read it with a named verifier
---

# DIRECTORY-031: Carry a typed capability claim in an agent's certificate and read it with a named verifier

> **Cluster:** directory
> **Depends on:** DIRECTORY-003, DIRECTORY-006
> **Blocked by:** Sign-off of this brief on its card by Tom or the lead before any requirement is built. Sign-off does not ratify the format., R3, the ratification record: the ratification of lys/agent-capability/v1 and of its OID 1.3.6.1.4.1.66364.2.1 by the owning lead with a second reader, recorded on the card with both names and the reasons, asked for only after R2's review has landed and records every attack defeated. Sign-off of this brief is not that ratification, and the format is a technical decision whose ratification does not wait on Tom. Checks a stranger can run: `git show origin/main:docs/design/identity/CAPABILITY-CLAIM-REVIEW.md` exits 0, and the card carries a recorded ratification naming the owning lead and the second reader, each with their reasons., R4, the format row, and R6 and R8, which encode and read the claim: R3 landed on main, so that the decision log records the format RATIFIED and the register records .2.1 in use. Checks a stranger can run: `git show origin/main:docs/design/WIRE-FORMATS.md | grep -c -E '^\| D[0-9]+ \|.*lys/agent-capability/v1.*RATIFIED'` prints 1, and `git show origin/main:docs/PEN-REGISTRATION.md | grep -c -F '| In use (.2.1) |'` prints 1., R4 to R8, the code rows: DIRECTORY-003 built on main, the lys-identity and lys-identity-server crates existing. Check a stranger can run: `git cat-file -e origin/main:crates/lys-identity/Cargo.toml && git cat-file -e origin/main:crates/lys-identity-server/Cargo.toml` exits 0. Every modify path under crates/lys-identity, crates/lys-identity-server and deploy/identity is reconciled against DIRECTORY-002's and DIRECTORY-003's landed manifests before dispatch (CN9, CN12)., R4 to R8, the code rows: DIRECTORY-006 R1 built on main, the grant contract existing. Check a stranger can run: `git cat-file -e origin/main:crates/lys-identity/src/grants/types.rs` exits 0 and `cargo test -p lys-identity --test grant_contract` passes at origin/main., R6: DIRECTORY-013 (card zP1P2HLD, this card's named revocation) landed on lys main, because every certificate R6 issues is appended to the certificate log as DIRECTORY-013's issuance leaf so that its fold can revoke it. DIRECTORY-013 is carried in pull request 28 of the lys repository. Checks a stranger can run, from a clone of lys: `git fetch -q origin pull/28/head && git show FETCH_HEAD:docs/design/identity/CERTIFICATE-REVOCATION.md | grep -c '^## 8. The fold'` prints 1, finding its fold there; and `git fetch -q origin main && git cat-file -e origin/main:crates/lys-identity/src/revocation/append.rs && git cat-file -e origin/main:crates/lys-identity/src/revocation/fold.rs` exits 0 once it has landed on main., R5: DIRECTORY-003's agent record landed on main, keyed on DIRECTORY-003's own execution record reading landed, not on RM-001, because DIRECTORY-003's file manifest is unwritten and RM-001 links other briefs besides it, so reading RM-001 cannot say whether DIRECTORY-003 alone has landed (a later card may re-key this check on the agent record's file once DIRECTORY-003's manifest names it). Check a stranger can run, from a clone of lys: `git fetch -q origin main && git show origin/main:docs/design/directory/briefs/DIRECTORY-003.json | python3 -c "import json,subprocess,sys; e=json.load(sys.stdin).get('execution') or {}; c=subprocess.run(['git','rev-parse','origin/main'],capture_output=True,text=True).stdout.strip(); ok=e.get('status')=='landed'; print(('landed' if ok else 'not landed')+' at '+c); sys.exit(0 if ok else 1)"` prints `landed at` followed by the commit it read, and exits 0; it prints `not landed at` followed by that commit, and exits 1, while the record is absent or not landed. R6 is blocked by R4 and R5, and R7 by R5 and R6., R5's, R6's and R7's routes: the directory service's router existing on main. Check a stranger can run: `git cat-file -e origin/main:crates/lys-identity-server/src/routes.rs` exits 0 (DIRECTORY-005 R1 names it; reconciled against DIRECTORY-003's landed manifest before dispatch)., R6's configuration example: deploy/identity/config.example.toml, which is DIRECTORY-002's and which R6 modifies, existing on main. Check a stranger can run: `git cat-file -e origin/main:deploy/identity/config.example.toml` exits 0., Production chaining of the directory's issuer key: which anchor it chains to is OPEN for Tom (design non-goal, the first agents' anchor; C5). Tests use a test anchor, and no requirement here chooses the production one., The claim's scope member and a refusal on scope: blocked on Tom's grant representation (design non-goal, the grant representation; C5). v1 carries no scope member and no requirement here invents one.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-093 — The directory cluster carries the certificate half of road step 2 — Amend narrowly, as on the SpiceDB brief (QZD2uagw): the certificate half of step 2, the typed capability claim and its verifier, leaves the 'Road step 2 onward' non-goal and becomes a goal of this cluster, and CN11 gains one appended ruling line saying the same; everything else in the non-goal stays out. CN1 gains one appended ruling line saying it binds DIRECTORY-001's documents-only work, and its own words are not changed. Rejected: leaving the brief under the unamended non-goal, which would make the cluster's design deny the work its own brief carries; widening the amendment to the rest of step 2 (SpiceDB, the per-call door check, per-execution credentials); and rewording CN1 or CN11 themselves.
> - ADR-094 — An agent certificate's validity window comes from the directory's configuration: 30 days by default, 90 days at most — The directory's configuration holds a default validity window and a maximum, shipped as 30 days (2592000 seconds) and 90 days (7776000 seconds). An issuance call with no window takes the default; the operator may ask for a shorter window, never a longer one, and a request above the maximum is refused by name, lifetime_above_maximum, naming the maximum. The certificate records the window it was given as its own notBefore and notAfter. A verifier takes a set of trusted issuer keys, never exactly one, and the certificate names the issuer key that signed it in X.509's Authority Key Identifier by its issuer-key fingerprint, the first 20 bytes of the SHA-256 of that key's SubjectPublicKeyInfo (RFC 7093 method 1, matching the SubjectKeyIdentifier of lys-core's issuer certificate), so a later rotation of the issuer key needs no change of format. Rejected: an operator-supplied lifetime with no default and no bound, which lets a typo outlive a withdrawn grant; a window fixed in code with no configuration; and a verifier bound to exactly one issuer key, which would make every rotation a format change.
> **Checklist:**
> - C249 — docs/design/identity/CAPABILITY-CLAIM.md proposes lys/agent-capability/v1 under the proposed OID 1.3.6.1.4.1.66364.2.1, alongside the unchanged .1 extension transport, stating its transport, assertion, encoding, issuer key identifier (the 20-byte RFC 7093 method 1 keyid, a new issuer-key fingerprint), scope, verifier check, rendering consumer, revocation, issuer and anchor, and status; docs/design/WIRE-FORMATS.md carries it as PROPOSED and both of its ratification sentences name the owning lead with a second reader, with D1 to D6 unchanged; and the .2 row of docs/PEN-REGISTRATION.md keeps .2 a family arc and records .2.1 for the typed capability claim.
> - C250 — docs/design/identity/CAPABILITY-CLAIM-REVIEW.md records an adversarial review of the draft by a party other than its author, holding at least the forgery, cross-format confusion, malleability, transposition, issuer substitution, downgrade, replay and expiry attacks, each built as bytes and defeated by a named refusal of the draft, and a timing-oracle entry that, for each comparison the verifier makes, builds a timing attack and names what defeats it or states why both operands are public.
> - C251 — Once the owning lead and a second reader have ratified lys/agent-capability/v1 on the card, the format's row in docs/design/WIRE-FORMATS.md's decision log reads RATIFIED with the date, both names and the reasons the card records, and the .2 row of docs/PEN-REGISTRATION.md reads In use (.2.1), with no other line of either file changed.
> - C252 — lys-identity encodes the claim of holder id and every grant held at issuance, each with its grant id and its window as it stood then, the list empty when the agent held none and a grant's end absent when it has none, to the vectors pinned in the draft, refuses every non-canonical, unordered, duplicated, unknown-member, missing-member and trailing-byte payload with claim_malformed, and refuses another content type with claim_version_unknown.
> - C253 — The operator enrols one Ed25519 public key per agent, keyed by the agent's directory id and held beside the agent record, once, as one signed directory event; a second enrolment is refused and no key is taken from an issuance call.
> - C254 — When the operator calls the directory's issuance route with an active agent's certificate-signing request, the directory issues that enduring agent a certificate, subject its directory id, carrying exactly one lys/agent-capability/v1 claim listing every grant it holds at issuance, an empty list when it holds none, under an issuer key held in the service's custody and named in the certificate's Authority Key Identifier by its 20-byte issuer-key fingerprint so that `openssl verify -CAfile` finds the issuer, for the window asked for or the configured default of 30 days and never past the configured maximum of 90 days, appended to the certificate log as the revocation card's issuance leaf, as one directory event naming the certificate it follows; it refuses an agent that is not active, an agent holding a current certificate, one not expired, not under a replaced key and not held revoked by the revocation fold (naming it and its expiry), an agent with no enrolled key (agent_key_not_enrolled), a request whose key does not match the enrolled key (key_mismatch) and an issuance whose leaf is not appended; nothing issues when a grant is given.
> - C255 — Replacing an agent's enrolled key is one audited directory event naming the old and new key, never an overwrite; an old key that is not the enrolled one is refused with key_replacement_mismatch, and a certificate under the replaced key is no longer current.
> - C256 — verify_agent_capability finds the signing key in a set of trusted issuer keys before it reads the claim and refuses an Authority Key Identifier that is absent or names another key (issuer_key_mismatch), a malformed claim, an unknown version, a holder other than the subject, a grant the claim does not list, and an instant outside the certificate's own window, each by a named refusal, with the refusal legs counted; a listed grant's window is never checked as live.
> **Stories:**
> - S113 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to read from an agent's certificate what it was granted at issuance and have a holder, a grant or an instant outside that refused, so that a signed claim is never taken as checked when nothing checked it.
> - S114 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a stranger verifying lys artifacts, I want the capability claim format specified, attacked and ratified before anything is signed under it, with every shipped format left byte-identical, so that no historical verification breaks.
> - S115 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want my agent issued one certificate listing the grants it holds and naming its holder, against a key enrolled for it, so that it carries proof of those grants from its first spawn and a lost key or an expired certificate can be replaced.

## Purpose

Road step 2's certificate half (docs/design/identity/STATEMENT-2026-09-22.md, The road; in this cluster under ADR-093): today an agent's lys certificate can carry only opaque bytes under the shipped capability-claims extension, which nothing interprets, and a signed field nothing checks is worse than none. This brief runs the design round for a new format with its own version, lys/agent-capability/v1, carried under its own proposed OID 1.3.6.1.4.1.66364.2.1, the first sub-arc of the .2 family arc of agent-certificate extensions, alongside the shipped .1 transport and never a mutation of it, records .2.1 in the register's .2 row, and makes both of WIRE-FORMATS.md's ratification sentences name the owning lead with a second reader (R1), has it attacked by a second party before ratification is asked for (R2), writes the recorded ratification into the decision log and the register's .2 row (R3), and, once that record is on main, defines the typed claim in lys-identity, naming its holder and every grant the agent holds at issuance with each grant's window as it stood then (R4), has the operator enrol one Ed25519 public key per agent beside its agent record (R5), has the directory issue an enduring agent its one certificate carrying one claim when the operator calls its issuance route, under an issuer key in the directory service's custody that the certificate names in X.509's Authority Key Identifier, for a configured validity window, appending the certificate to the certificate log as DIRECTORY-013's issuance leaf so that its revocation fold can revoke it (R6), lets the operator replace an enrolled key by an audited act that is never an overwrite (R7), and names verify_agent_capability as the verifier that checks the chain against a set of trusted issuer keys, parses the claim, refuses what the claim does not cover and refuses the certificate past its own expiry (R8). Done when a certificate states what an agent was granted and a verifier reads it. lys-core's published formats and cryptography do not change (DIRECTORY-006 R1).

## Task

Implement R1 to R8 in order. R1 and R2 are documents and go ahead after sign-off; R3 is a documents row that goes ahead once the ratification is recorded on the card; R1 and R3 are the only rows that edit docs/PEN-REGISTRATION.md and docs/design/WIRE-FORMATS.md, R1 proposing and R3 recording the ratification, and no other card rewrites the register's .2 row. R4 to R8 wait for DIRECTORY-003 and DIRECTORY-006 R1 on main; R4, R6 and R8 also wait for R3 on main; R5 waits for DIRECTORY-003's agent record; R6 waits for R4, R5, DIRECTORY-013 and deploy/identity/config.example.toml on main; R7 waits for R5 and R6; R8 waits for R4 and R6 (blocked_by). Estimate, as a new increment outside IDENTITY-001's 48-hour ceiling, which stays as CN7 states: R1 2h, R2 3h, R3 1h, R4 2h, R5 3h, R6 6h, R7 3h, R8 3h, total 23 hours; estimates are proposals for review. The first vocabulary is narrow: one claim per certificate, naming its holder by the agent's directory id and listing every grant the agent holds at issuance, each by its directory id with its time window as it stood at issuance, and nothing else; an agent holding no grant at issuance is issued a certificate whose claim lists no grant, since the certificate proves who the agent is and what it may do is always asked of the access check, and a listed grant whose window has no end is encoded with its end absent, its start always present, following DIRECTORY-006's grant window, whose end may be null. The list is what held at issuance (conformance 6.2): it is never presented as live grants, a listed grant's window is a record and not a live check, and whether a grant still stands is always asked of the access check, never of the list; the operator chooses nothing about the list on the issuance call, and a grant given later needs a new certificate, while whether a grant change reissues a certificate stays open under conformance 6.3. The claim carries no validity window of its own: the certificate's notBefore and notAfter govern its life, and lys-core's certificate-expired check, run by the verifier against the instant it is given, refuses a certificate past expiry. The claim carries no member for revocation: the revocation fold of DIRECTORY-013 (card zP1P2HLD, this card's named revocation) names a certificate by the SHA-256 of its DER, computed by whoever checks it and never from anything the issuer chose, so the certificate's identity in the log is the hash of the certificate itself and the fold needs no field in the claim; R6 appends every certificate it issues to the certificate log as DIRECTORY-013's issuance leaf, so that the fold can revoke it. The claim carries no issuer member either: the certificate names the issuer key that signed it in X.509's Authority Key Identifier extension (OID 2.5.29.35, non-critical), whose keyIdentifier is the issuer-key fingerprint, a new fingerprint defined here as the first 20 bytes of the SHA-256 of the issuer key's DER SubjectPublicKeyInfo (RFC 7093 section 2, method 1), which is the keyid rcgen writes as the SubjectKeyIdentifier of lys-core's issuer certificate; it is not lys's existing certificate fingerprint, the SHA-256 of a certificate's DER; the extension is added through lys-core's existing extensions parameter with lys-core's issuance and its leaf parameters untouched, it matches the SubjectKeyIdentifier of lys-core's issuer certificate so that `openssl verify -CAfile` keeps finding the issuer, the issuer DN stays as lys-core writes it, and a stranger reads the identifier with `openssl x509 -text`. The scope member is blocked on the grant representation (C5) and is not invented. The .1 transport and `lys ca issue --claims` stay exactly as shipped, neither deprecated nor removed; `lys ca verify`, `lys verify --cert` and `lys inspect cert` keep showing only .1, and showing the typed claim in the lys CLI is its own later card. The certificate is per enduring agent: issued when the agent is provisioned, presented from its first spawn on, and its subject is the agent's directory id, which must also be the common name of the agent's certificate-signing request, since lys-core refuses a request whose common name differs from the subject. Issuance is invoked only by the operator calling the directory's issuance route with the agent's certificate-signing request (R6), never automatically when a grant is given. The issuer key's custody: its 32-byte seed lives in a restricted private file on the directory service's host, named by the service's configuration, loaded only by crates/lys-identity-server/src/issuer_key.rs and held in Zeroizing buffers; it is reached only by the issuance route and never by lys (the CLI), Cambium or any other process. The same configuration holds the agent certificate's default validity window and its maximum, shipped as 30 days and 90 days (ADR-094), and the path of the certificate log; the operator may ask for a shorter window, never a longer one, and the certificate records the window it was given as its own notBefore and notAfter. The configuration's example, deploy/identity/config.example.toml, and the restricted-file helper crates/lys/src/identity/private_files.rs are DIRECTORY-002's; R6 modifies the example file only once it exists on main (blocked_by), and it does not reuse or change private_files.rs, because no file under crates/lys changes. The verifier authenticates the certificate under a key of its set before it reads the claim, and checks the instant against the certificate's own window last, so that every claim refusal fires by name on a certificate inside its window. Issuance refuses an agent holding a current certificate and only that: a certificate that has expired, whose key has been replaced (R7), or that DIRECTORY-013's fold holds revoked is not current, and each issuance is its own audited event naming the certificate it follows. The directory's events are written in DIRECTORY-003's signed event envelope under the kinds agent_key_enrolled (R5), agent_certificate_issued (R6) and agent_key_replaced (R7); if that envelope's register, docs/design/identity/IDENTITY-EVENTS.md, must list them, the row stops and names that file under CN9, because DIRECTORY-003's files are not edited here. In scope: the draft and its WIRE-FORMATS.md entry, the adversarial review, the ratification's record in the decision log and the register, the typed claim, the enrolled key and its audited replacement, issuance through the directory's route with the issuer key's custody, its Authority Key Identifier and its issuance leaf, and the verifier with its expiry leg. Out, each a later unit: presenting the certificate at spawn (the step-2 session card, 6AWx1-JR, which also owns per-execution credentials), showing the certificate on the agent's file (ADR-008, DIRECTORY-005), showing the typed claim in the lys CLI, the scope member, the revocation fold itself and its request path (DIRECTORY-013), the live per-call check at the door and in SpiceDB (QZD2uagw and the Cambium side), the production anchor, and any change to lys-core, the lys CLI or lys/delegation/v1. This brief writes no rotation requirement for the directory's issuer key: rotation waits on the anchor that key chains to, which is open, and is its own card, which must keep every agent certificate issued under the current issuer key verifiable across a rotation. This brief adds checklist items C249 to C256 and C251 and stories S113 to S115 and splits none.

## Requirements

### R1: Propose lys/agent-capability/v1 under 1.3.6.1.4.1.66364.2.1 alongside the shipped transport

Structural. Write docs/design/identity/CAPABILITY-CLAIM.md as the design round's proposal for the agent capability claim format lys/agent-capability/v1, a new format with its own version alongside the shipped certificate extension transport and never a mutation of it. It SHALL have these sections, in this order: Transport (the claim is the whole value of one non-critical extension under the proposed OID 1.3.6.1.4.1.66364.2.1, the first sub-arc of 1.3.6.1.4.1.66364.2, which stays the family arc for agent-certificate extensions (which issuer vouched, runtime identity, session binding) so that those extensions keep room under it; written by the unchanged encode_extension and read by the unchanged decode_extension; the shipped .1 extension and `lys ca issue --claims` stay exactly as shipped, so a claim hand-issued as operator JSON under .1 can never be taken for a typed claim; `lys ca verify` and `lys inspect cert` keep showing only .1; the OID is proposed for ratification with the rest of the format); Assertion (exactly one claim per certificate, naming the holder by the agent's directory id, which equals the certificate subject's common name, and listing every grant the agent holds at issuance, each by its directory id with its time window as it stood at issuance, and nothing else; an agent holding no grant at issuance gets a claim whose list is empty, because the certificate proves who the agent is and what it may do is always asked of the access check; the list is what held at issuance, never presented as live grants, and a listed window is a record, not a live check, so the current answer is always the access check's; the claim carries no validity window of its own, because the certificate's notBefore and notAfter govern its life; no revocation member, because the revocation fold names a certificate by the SHA-256 of its DER, computed by whoever checks it and never from anything the issuer chose; and no issuer member, because the certificate names its issuer key in its Authority Key Identifier); Encoding (a deterministic CBOR map under RFC 8949 section 4.2.1 with integer keys 1 content type, the text string lys/agent-capability/v1, which is the payload's own version and its domain separation from every other lys artifact; 2 holder id, text; 3 grants, an array of maps, possibly empty, each with integer keys 1 grant id, text, 2 not-before and 3 not-after, unsigned integer seconds since the Unix epoch in UTC, not-after after not-before, key 3 present only when the grant's window has an end and absent, never null, when it has none, as the grant contract's window allows an absent end, the array ordered by the grant id's UTF-8 bytes ascending with no grant id twice; and the full hex vector for the fixture claim (holder id `agent-01`; grant `grant-01` with not-before 1800000000 and not-after 1800086400; grant `grant-02` with not-before 1800000000 and not-after 1802592000), for the claim of holder `agent-01` listing no grant, and for the claim of holder `agent-01` listing only grant `grant-01` with not-before 1800000000 and no end); Issuer key identifier (the certificate carries X.509's Authority Key Identifier extension, OID 2.5.29.35, non-critical, whose value is the DER of an AuthorityKeyIdentifier holding only a keyIdentifier of exactly 20 bytes, the issuer-key fingerprint, a new fingerprint defined here as the first 20 bytes of the SHA-256 of the issuer key's DER SubjectPublicKeyInfo (RFC 7093 section 2, method 1), which is the keyid rcgen writes as the SubjectKeyIdentifier of lys-core's issuer certificate; it is not lys's existing certificate fingerprint, the SHA-256 of a certificate's DER, and which is the fingerprint by which a verifier's set of trusted issuer keys knows each key; because it equals the SubjectKeyIdentifier of lys-core's issuer certificate, `openssl verify -CAfile` finds the issuer; it is added through lys-core's existing extensions parameter, so lys-core's issuance, its leaf parameters and its issuer certificate stay unchanged; the issuer DN stays as lys-core writes it; and the full hex of that extension value for the Ed25519 issuer key derived from the 32-byte seed of 0x07 bytes); Scope (v1 has no scope member and no scope refusal: both wait on the grant representation, and adding one is a new version alongside v1); Verifier check (verify_agent_capability in lys-identity takes the certificate's DER, a set of trusted issuer public keys, never exactly one, the instant and the grant id the caller acts under, and in this order: finds the signing key, the key of the set under which lys-core's verify_certificate_chain_at accepts the certificate at the certificate's own notBefore, before any byte of the claim is read; compares the Authority Key Identifier's keyIdentifier with the signing key's issuer-key fingerprint; parses the claim; checks the holder against the subject and the grant acted under against the listed grant ids; then checks the instant against the certificate's own window with verify_certificate_chain_at under the signing key; each refusal named: certificate_chain_invalid, issuer_key_mismatch, claim_version_unknown, claim_malformed, claim_holder_mismatch, claim_grant_mismatch; a certificate past its notAfter is refused as certificate_chain_invalid carrying lys-core's `certificate expired` reason; the verifier never checks the instant against a listed grant's window and never answers whether a grant still stands); Rendering consumer (a consumer that shows a claim, first the agent's file of ADR-008, shows it only after verify_agent_capability accepts it, and shows the list as held at issuance, never as live grants); Revocation (v1 stops by expiry of the certificate; revocation is the DP26 fold of the revocation card zP1P2HLD, which needs no member of the claim; the directory appends every certificate it issues to the certificate log as that card's issuance leaf, so the fold can revoke it; the live per-call check belongs to the card QZD2uagw); Issuer and anchor (the directory issues through lys-identity under an issuer key the directory service holds; a verifier holds a set of trusted issuer keys and the certificate names its signer in its Authority Key Identifier, so a rotation of the issuer key needs no change of format; which anchor that key chains to is open; tests use a test anchor); Status (proposed, not ratified; sign-off of this brief does not ratify it; ratification is the owning lead's with a second reader, recorded on the card with both names and the reasons, and R3 then writes it into the decision log of docs/design/WIRE-FORMATS.md). The same requirement SHALL add to docs/design/WIRE-FORMATS.md one line directly after the paragraph that says a capability format is not started and not scheduled, pointing at the proposal, and one decision-log row for lys/agent-capability/v1 and its OID under the next D number, marked PROPOSED; SHALL change the sentence of its opening paragraph that reads `Ratification is a deliberate human decision, recorded in the decision log at the bottom of this file.` to read `Ratification is a deliberate decision of the owning lead with a second reader, recorded on the card with both names and the reasons and in the decision log at the bottom of this file, never inferred from a build going green.`; and SHALL change the words `ratification remains a recorded human decision, never inferred from a build going green.` at the end of the paragraph under the decision log to read `ratification remains a decision of the owning lead with a second reader, recorded on the card with both names and the reasons, never inferred from a build going green.` The same requirement SHALL rewrite the .2 row of docs/PEN-REGISTRATION.md's sub-arc table so that its purpose keeps its stated family (identity and issuer extensions for agent certificates: which issuer vouched, runtime identity, session binding), states that .2 is a family arc whose extensions take sub-arcs beneath it so that each keeps room under .2, and names .2.1 for the typed capability claim lys/agent-capability/v1, and whose status cell stays `Reserved` while the format is proposed; R3 changes that status cell once the ratification is recorded, and this requirement does not. It SHALL NOT change or delete any other line of docs/design/WIRE-FORMATS.md, SHALL NOT change the rows D1 to D6, SHALL NOT mark the format ratified, SHALL NOT change any line of docs/PEN-REGISTRATION.md other than the .2 row, SHALL NOT change the .1 row or the .3+ row, SHALL NOT mark .2 `In use` or allocated, SHALL NOT place the typed claim under 1.3.6.1.4.1.66364.1 or under 1.3.6.1.4.1.66364.2 itself, SHALL NOT name lys/delegation/v1 as a carrier of the claim, SHALL NOT describe the claim's list as live grants, and SHALL NOT change any file under crates/.

**Acceptance:**
- docs/design/identity/CAPABILITY-CLAIM.md exists, and its second-level headings, read in order, are exactly: Transport, Assertion, Encoding, Issuer key identifier, Scope, Verifier check, Rendering consumer, Revocation, Issuer and anchor, Status.
- The draft contains each of the literal strings `lys/agent-capability/v1`, `1.3.6.1.4.1.66364.2.1`, `2.5.29.35`, `RFC 7093`, `verify_agent_capability`, `certificate_chain_invalid`, `issuer_key_mismatch`, `claim_version_unknown`, `claim_malformed`, `claim_holder_mismatch`, `claim_grant_mismatch` and `certificate expired`, and contains neither `claim_expired` nor `claim_not_yet_valid`.
- The draft's hex vector for the fixture claim (holder id `agent-01`; grant `grant-01` with not-before 1800000000 and not-after 1800086400; grant `grant-02` with not-before 1800000000 and not-after 1802592000) is exactly a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f00 (84 bytes).
- The draft's hex vector for the claim of holder `agent-01` listing no grant is exactly a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310380 (38 bytes), and its hex vector for the claim of holder `agent-01` listing only grant `grant-01` with not-before 1800000000 and no end is exactly a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310381a201686772616e742d3031021a6b49d200 (55 bytes).
- The draft's hex Authority Key Identifier extension value for the Ed25519 issuer key derived from the 32-byte seed of 0x07 bytes is exactly 30168014324be2dea8bc44461b0233e51fa48902ed6b1cc6 (24 bytes), whose last 20 bytes are the first 20 bytes of the SHA-256 of that key's 44-byte DER SubjectPublicKeyInfo, and the draft states that this issuer-key fingerprint is new and is not the SHA-256 of a certificate's DER.
- `git diff --numstat` of docs/design/WIRE-FORMATS.md against the brief's base commit reports 4 added lines and 2 deleted lines, the deleted lines being the opening paragraph's line and the paragraph under the decision log at the base commit.
- docs/design/WIRE-FORMATS.md contains the sentence `Ratification is a deliberate decision of the owning lead with a second reader, recorded on the card with both names and the reasons and in the decision log at the bottom of this file, never inferred from a build going green.` and does not contain `Ratification is a deliberate human decision`.
- docs/design/WIRE-FORMATS.md contains `ratification remains a decision of the owning lead with a second reader, recorded on the card with both names and the reasons, never inferred from a build going green.` and does not contain `ratification remains a recorded human decision`.
- The rows D1 to D6 of docs/design/WIRE-FORMATS.md's decision log are byte-identical to the base commit's.
- The line added after the not-started paragraph of docs/design/WIRE-FORMATS.md contains `docs/design/identity/CAPABILITY-CLAIM.md`.
- docs/design/WIRE-FORMATS.md's decision log gains exactly one row containing `lys/agent-capability/v1`; that row contains `1.3.6.1.4.1.66364.2.1` and `PROPOSED` and does not contain `RATIFIED`, and its D number is one more than the highest D number the log held at the base commit.
- `git diff --numstat` of docs/PEN-REGISTRATION.md against the brief's base commit reports 1 added line and 1 deleted line; the deleted line is the sub-arc table's row whose first cell is .2, the added line is that row rewritten, whose first cell is still .2, it contains `.2.1`, `lys/agent-capability/v1`, `family arc` and `which issuer vouched, runtime identity, session binding`, and its last cell reads exactly `Reserved`, as the deleted row's did; and `git diff --stat` against the base commit for crates/ prints nothing.

**Files:**
- create: docs/design/identity/CAPABILITY-CLAIM.md
- modify: docs/design/WIRE-FORMATS.md
- modify: docs/PEN-REGISTRATION.md

**Checklist:**
- C249 — docs/design/identity/CAPABILITY-CLAIM.md proposes lys/agent-capability/v1 under the proposed OID 1.3.6.1.4.1.66364.2.1, alongside the unchanged .1 extension transport, stating its transport, assertion, encoding, issuer key identifier (the 20-byte RFC 7093 method 1 keyid, a new issuer-key fingerprint), scope, verifier check, rendering consumer, revocation, issuer and anchor, and status; docs/design/WIRE-FORMATS.md carries it as PROPOSED and both of its ratification sentences name the owning lead with a second reader, with D1 to D6 unchanged; and the .2 row of docs/PEN-REGISTRATION.md keeps .2 a family arc and records .2.1 for the typed capability claim.

**Stories:**
- S114 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a stranger verifying lys artifacts, I want the capability claim format specified, attacked and ratified before anything is signed under it, with every shipped format left byte-identical, so that no historical verification breaks.

### R2: Attack the draft before ratification is asked for

WHEN R1's draft is complete, THE SYSTEM SHALL have it reviewed adversarially by a party other than its author, before ratification is asked for, and record the review in docs/design/identity/CAPABILITY-CLAIM-REVIEW.md with the commit of the draft it reviewed. The review SHALL hold at least the following entries, building each attack as concrete bytes against the draft and naming the draft's refusal that defeats it, and SHALL enter any further attack its reviewer builds in the same way: forgery (a claim in a certificate not signed by any key of the verifier's set); cross-format confusion (the operator JSON `{"role":"admin"}`, as `lys ca issue --claims` embeds it, placed under 1.3.6.1.4.1.66364.2.1; a v1 claim's bytes placed only under 1.3.6.1.4.1.66364.1; a lys/delegation/v1 artifact's bytes and a lys/attestation/v2 payload's bytes each placed under 1.3.6.1.4.1.66364.2.1); malleability (the fixture claim re-encoded with a grant's not-before in a longer integer form, and re-encoded with its two grants in the other order and with one grant listed twice, and a grant with no end re-encoded with key 3 present as null); field transposition (the holder id placed as a listed grant id and a grant id placed as the holder id); issuer substitution (an Authority Key Identifier naming a second trusted issuer key inside a certificate signed by the first, a certificate carrying no Authority Key Identifier, and an Authority Key Identifier whose keyIdentifier is the full 32-byte SHA-256 of the signing key's SubjectPublicKeyInfo); downgrade (a content type naming lys/agent-capability/v0, and a map with key 1 absent); replay (a valid claim for one agent presented in a certificate whose subject is another agent, and a valid claim presented for a grant it does not list); expiry (an instant one second past the certificate's own notAfter); and timing oracle (for each comparison verify_agent_capability makes, the Authority Key Identifier against the signing key's fingerprint, the holder against the subject, the grant acted under against the listed grant ids and the instant against the certificate's own window, either a timing attack built as bytes and the refusal or property that defeats it, or the statement, with its reason, that both operands are public so the comparison exposes no secret to time; and the same for the signature check, which is lys-core's unchanged verify_certificate_chain_at). IF an attack succeeds against the draft, THEN THE SYSTEM SHALL amend the draft and repeat that attack against the amended draft, and SHALL NOT ask for ratification while any attack stands. The review SHALL NOT be written by the draft's author, SHALL NOT be a light-model pass, and SHALL NOT change code.

**Acceptance:**
- docs/design/identity/CAPABILITY-CLAIM-REVIEW.md names its reviewer and the draft's author, and the two are different parties.
- The review holds at least these nineteen named entries: forgery; operator JSON under 1.3.6.1.4.1.66364.2.1; a v1 claim only under .1; lys/delegation/v1 bytes; lys/attestation/v2 payload bytes; malleability by integer form; malleability by grant order; malleability by a grant listed twice; malleability by a null end; transposition; Authority Key Identifier naming another trusted key; Authority Key Identifier absent; Authority Key Identifier of 32 bytes; downgrade to v0; downgrade by absent key 1; replay under another agent's certificate; replay for an unlisted grant; expiry one second past the certificate's notAfter; timing oracle. Each entry but the timing oracle carries its attack bytes in hex and the name of the draft's refusal that defeats it.
- The timing-oracle entry names each of the four comparisons verify_agent_capability makes (Authority Key Identifier against the signing key's fingerprint, holder against subject, grant acted under against the listed grant ids, instant against the certificate's own window) and the signature check, and for each either carries a timing attack's bytes in hex with what defeats it, or states that both operands are public values and gives the reason.
- Every entry of the review, each of the nineteen named entries and any further entry its reviewer adds, records the outcome `defeated`, and no entry records any other outcome.
- The review records a commit id, and docs/design/identity/CAPABILITY-CLAIM.md at that commit is byte-identical to the draft at the commit where the review lands.

**Files:**
- create: docs/design/identity/CAPABILITY-CLAIM-REVIEW.md

**Checklist:**
- C250 — docs/design/identity/CAPABILITY-CLAIM-REVIEW.md records an adversarial review of the draft by a party other than its author, holding at least the forgery, cross-format confusion, malleability, transposition, issuer substitution, downgrade, replay and expiry attacks, each built as bytes and defeated by a named refusal of the draft, and a timing-oracle entry that, for each comparison the verifier makes, builds a timing attack and names what defeats it or states why both operands are public.

**Stories:**
- S114 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a stranger verifying lys artifacts, I want the capability claim format specified, attacked and ratified before anything is signed under it, with every shipped format left byte-identical, so that no historical verification breaks.

### R3: Record the format's ratification in the decision log and the register's .2 row

WHEN the ratification of lys/agent-capability/v1 and of its OID 1.3.6.1.4.1.66364.2.1 by the owning lead with a second reader is recorded on the card with both names and the reasons, after R2's review has landed with every entry defeated, THE SYSTEM SHALL write that ratification into the repository in one commit: in docs/design/WIRE-FORMATS.md, the status cell of the decision-log row for lys/agent-capability/v1 that R1 added SHALL change from PROPOSED to RATIFIED followed by the date of the recorded ratification, the names of the owning lead and of the second reader, and their reasons, each as the card records it, with every other cell of that row unchanged; and in docs/PEN-REGISTRATION.md, the status cell of the .2 row SHALL change from `Reserved` to `In use (.2.1)`, with every other cell of that row unchanged, so that .2 stays the family arc and .2.1 is in use for the typed capability claim. IF no ratification is recorded on the card, or R2's review is not on main or records any entry not defeated, THEN THE SYSTEM SHALL NOT make this change. THE SYSTEM SHALL NOT change any other line of docs/design/WIRE-FORMATS.md or docs/PEN-REGISTRATION.md, SHALL NOT change the rows D1 to D6, the .1 row or the .3+ row, SHALL NOT change the format's bytes, its OID or docs/design/identity/CAPABILITY-CLAIM.md, SHALL NOT record a ratifier or reason the card does not carry, and SHALL NOT change any file under crates/.

**Acceptance:**
- `git diff --numstat` of docs/design/WIRE-FORMATS.md between R3's commit and its parent reports 1 added line and 1 deleted line; the deleted line is the decision-log row containing `lys/agent-capability/v1` and `PROPOSED`; the added line has the same D number, contains `lys/agent-capability/v1`, `1.3.6.1.4.1.66364.2.1` and `RATIFIED`, does not contain `PROPOSED`, and contains the date, the two ratifiers' names and their reasons as the card records them; every cell of the added row other than its status cell is byte-identical to the deleted row's.
- At R3's commit, `git show <commit>:docs/design/WIRE-FORMATS.md | grep -c -E '^\| D[0-9]+ \|.*lys/agent-capability/v1.*RATIFIED'` prints 1, and the same command with PROPOSED in place of RATIFIED prints 0.
- The rows D1 to D6 of docs/design/WIRE-FORMATS.md's decision log at R3's commit are byte-identical to its parent's.
- `git diff --numstat` of docs/PEN-REGISTRATION.md between R3's commit and its parent reports 1 added line and 1 deleted line; the deleted line is the sub-arc table's row whose first cell is .2 and whose last cell reads exactly `Reserved`; the added line's first cell is .2, its last cell reads exactly `In use (.2.1)`, and every cell other than the last is byte-identical to the deleted row's; `git show <commit>:docs/PEN-REGISTRATION.md | grep -c -F '| In use (.2.1) |'` prints 1.
- `git diff --stat` between R3's commit and its parent names only docs/design/WIRE-FORMATS.md and docs/PEN-REGISTRATION.md.
- At R3's parent, `git show <parent>:docs/design/identity/CAPABILITY-CLAIM-REVIEW.md` exits 0, and the review at that commit has no entry whose outcome is other than `defeated`.

**Files:**
- modify: docs/design/WIRE-FORMATS.md
- modify: docs/PEN-REGISTRATION.md

**Checklist:**
- C251 — Once the owning lead and a second reader have ratified lys/agent-capability/v1 on the card, the format's row in docs/design/WIRE-FORMATS.md's decision log reads RATIFIED with the date, both names and the reasons the card records, and the .2 row of docs/PEN-REGISTRATION.md reads In use (.2.1), with no other line of either file changed.

**Stories:**
- S114 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a stranger verifying lys artifacts, I want the capability claim format specified, attacked and ratified before anything is signed under it, with every shipped format left byte-identical, so that no historical verification breaks.

### R4: Define the typed claim and its strict encoding in lys-identity

Define in crates/lys-identity/src/capability/claim.rs the claim of lys/agent-capability/v1 exactly as ratified: a typed claim with the holder id and the list of grants held at issuance, which is empty when the agent held none, each with its grant id, its not-before and, only when the grant's window has an end, its not-after, encoded canonically as the draft states. The content type is the payload's own version, alongside the .1 transport's opaque bytes and never a mutation of them. WHEN bytes are decoded, THE SYSTEM SHALL return a claim only if they are the canonical encoding of exactly the three members with the content type lys/agent-capability/v1, and of keys 1 and 2 and at most key 3 in every grant entry, an absent key 3 meaning the grant has no end; a claim whose grants array is empty is returned as a claim listing no grant. IF the bytes are a canonical map whose key 1 is a text string other than lys/agent-capability/v1, THEN THE SYSTEM SHALL refuse with claim_version_unknown naming that content type. IF the bytes are not CBOR, are not canonical (keys out of order, an integer not in shortest form, an indefinite length), carry an unknown key at the top or in a grant entry, lack a member (key 1, 2 or 3 at the top, key 1 or 2 in a grant entry), carry a grant's key 3 as anything but an unsigned integer, null included, carry trailing bytes, list grants out of order or one grant id twice, or carry a grant whose not-after is at or before its not-before, THEN THE SYSTEM SHALL refuse with claim_malformed naming the defect, and SHALL NOT return a partial claim or fill any member by default. The encoder SHALL emit the grants ordered by grant id, SHALL omit key 3 for a grant with no end and SHALL NOT emit null for it, and SHALL NOT emit a scope, revocation, issuer or validity-window member. No file under crates/lys-core SHALL change, and no dependency absent from the workspace Cargo.lock SHALL be added.

**Acceptance:**
- Encoding the fixture claim (holder id `agent-01`; grant `grant-01` with not-before 1800000000 and not-after 1800086400; grant `grant-02` with not-before 1800000000 and not-after 1802592000) yields exactly the hex a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f00; the test holds that hex as a literal and does not import it from the crate.
- Encoding the fixture claim built with its grants given in the order grant-02, grant-01 yields the same hex, and decoding that vector returns a claim equal to the fixture claim.
- Encoding the claim of holder `agent-01` listing no grant yields exactly the hex a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310380, and encoding the claim of holder `agent-01` listing only grant `grant-01` with not-before 1800000000 and no end yields exactly the hex a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310381a201686772616e742d3031021a6b49d200; decoding each returns a claim equal to the one encoded, the first listing no grant and the second's grant-01 having no end; the test holds both hex literals itself.
- Thirteen malformed inputs are each refused with claim_malformed: the bytes 7b7d (JSON `{}`), the vector with keys 1 and 2 in the order 2, 1, grant-01's not-before 1800000000 written in the 9-byte form 1b000000006b49d200, an indefinite-length map (first byte bf), an added top-level key 4, key 3 absent, key 1 absent, one trailing byte 00, grant-01's not-after equal to its not-before, the two grants in the order grant-02, grant-01, grant-01 listed twice, an added key 4 in grant-01's entry, and the open-ended vector with key 3 written as null (a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310381a301686772616e742d3031021a6b49d20003f6); the test asserts that thirteen claim_malformed cases ran.
- The vector with content type lys/agent-capability/v2 and the vector with content type lys/agent-capability/v0 are each refused with claim_version_unknown, and each message contains its content type; the test asserts two claim_version_unknown cases ran.
- A drift injection that removes only the grant-order check makes exactly one test fail, the malformed-input test, on its out-of-order case.

**Files:**
- create: crates/lys-identity/src/capability/mod.rs
- create: crates/lys-identity/src/capability/claim.rs
- create: crates/lys-identity/src/capability/encoding.rs
- create: crates/lys-identity/src/capability/error.rs
- create: crates/lys-identity/tests/capability_encoding.rs
- modify: crates/lys-identity/src/lib.rs
- modify: crates/lys-identity/Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C252 — lys-identity encodes the claim of holder id and every grant held at issuance, each with its grant id and its window as it stood then, the list empty when the agent held none and a grant's end absent when it has none, to the vectors pinned in the draft, refuses every non-canonical, unordered, duplicated, unknown-member, missing-member and trailing-byte payload with claim_malformed, and refuses another content type with claim_version_unknown.

**Stories:**
- S113 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to read from an agent's certificate what it was granted at issuance and have a holder, a grant or an instant outside that refused, so that a signed claim is never taken as checked when nothing checked it.

### R5: Enrol one Ed25519 public key per agent beside its agent record

WHEN the admitted operator (DIRECTORY-003 R3) calls POST /identity/agents/{agent_id}/key with one Ed25519 public key for a registered agent that has no enrolled key, THE SYSTEM SHALL enrol that key as the agent's one public key, keyed by the agent's directory id and held beside the agent record in the directory's store, as one signed directory event of kind agent_key_enrolled naming the agent and the key, committed before the call returns (P4, P5). IF the caller is not admitted, the agent id names no registered agent, the key is not a valid 32-byte Ed25519 public key, or the agent already has an enrolled key, THEN THE SYSTEM SHALL refuse with not_admitted, agent_not_registered, invalid_agent_key or agent_key_already_enrolled respectively, record no event and leave any enrolled key unchanged. The enrol route SHALL NOT overwrite an enrolled key, SHALL NOT accept private key material, and SHALL NOT change the files of DIRECTORY-003's agent record; THE SYSTEM SHALL NOT take an agent's key from a certificate-signing request or from any issuance call.

**Acceptance:**
- For a registered agent agent-01 with no enrolled key, the admitted operator's enrol call with the 32-byte Ed25519 public key K1 makes the directory's enrolled key for agent-01 read K1 and adds exactly one event to the directory, an event of kind agent_key_enrolled naming agent-01 and K1; after reopening the directory, the enrolled key still reads K1.
- A second enrol call for agent-01 with the key K2 refuses with agent_key_already_enrolled, records no event, and the enrolled key for agent-01 still reads K1.
- An enrol call by an unauthenticated caller refuses with not_admitted and records no event.
- An enrol call for agent-99, which is not registered, refuses with agent_not_registered and records no event.
- An enrol call for registered agent agent-02 with a 31-byte key refuses with invalid_agent_key, records no event, and agent-02 still has no enrolled key; the test asserts four refusal cases ran across this line and the three before it.
- A drift injection that removes only the already-enrolled check makes exactly one test fail, the second-enrol case.

**Files:**
- create: crates/lys-identity/src/agent_key.rs
- create: crates/lys-identity/tests/agent_key.rs
- create: crates/lys-identity-server/src/agent_key_route.rs
- create: crates/lys-identity-server/tests/agent_key_route.rs
- modify: crates/lys-identity/src/lib.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C253 — The operator enrols one Ed25519 public key per agent, keyed by the agent's directory id and held beside the agent record, once, as one signed directory event; a second enrolment is refused and no key is taken from an issuance call.

**Stories:**
- S115 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want my agent issued one certificate listing the grants it holds and naming its holder, against a key enrolled for it, so that it carries proof of those grants from its first spawn and a lost key or an expired certificate can be replaced.

### R6: Issue an enduring agent its certificate through the directory's issuance route, under an issuer key in the service's custody, and log it as an issuance leaf

THE SYSTEM SHALL hold the directory's issuer key only inside the directory service: read at start by crates/lys-identity-server/src/issuer_key.rs from the private file the service's configuration names, its 32-byte Ed25519 seed held in Zeroizing buffers, and used only to construct the lys-core certificate authority this route issues with and the issuing identity DIRECTORY-013's append takes; the same configuration holds the agent certificate's default validity window and its maximum, shipped as 2592000 seconds (30 days) and 7776000 seconds (90 days) (ADR-094), and the path of the certificate log, and its example, deploy/identity/config.example.toml, gains the four entries without any secret value. IF the seed file is missing, is not exactly 32 bytes, or has any group or other permission bit set, THEN THE SYSTEM SHALL refuse to start issuing with issuer_key_missing, issuer_key_length or issuer_key_permissions respectively, naming the path and never the bytes. WHEN the admitted operator calls POST /identity/agents/{agent_id}/certificate with the agent's certificate-signing request and optionally a lifetime in seconds, THE SYSTEM SHALL take the call's instant from its injected clock; take the lifetime asked for, or the configured default when none is asked for; build the claim (the agent's directory id as holder, and every grant the directory records the agent as holding at the call's instant, each with its grant id and its time window as it stands then, the window's end omitted when the grant has none; an empty list when the agent holds no grant, the issuance proceeding); encode it with R4; place it with encode_extension under 1.3.6.1.4.1.66364.2.1; build the Authority Key Identifier extension value, the DER of an AuthorityKeyIdentifier holding only a keyIdentifier of the directory issuer key's issuer-key fingerprint (the first 20 bytes of the SHA-256 of its DER SubjectPublicKeyInfo), and place it with encode_extension under 2.5.29.35, non-critical; issue the certificate with lys-core's issue_certificate_for_request from that request, with the agent's directory id as subject, the lifetime as its ttl and those two extensions; append the certificate's DER to the certificate log as DIRECTORY-013's issuance leaf with DIRECTORY-013's append; and record the issuance as one signed directory event of kind agent_certificate_issued naming the agent, the SHA-256 of the new certificate's DER, its notAfter and the SHA-256 of the certificate it follows when there is one, committed before the certificate's DER is returned (P4, P5). A certificate is current while its notAfter is at or after the call's instant, its subject public key is the agent's enrolled key and DIRECTORY-013's fold over the certificate log does not hold it revoked. IF the caller is not admitted, the agent is not active (registered, suspended or retired, ADR-011), the agent has no enrolled key, the request's common name is not the agent's directory id, the request's public key does not equal the agent's enrolled key, the lifetime is zero, the lifetime exceeds the configured maximum, the agent holds a current certificate, the issuance leaf is not appended, or the issuance event is not committed, THEN THE SYSTEM SHALL refuse with not_admitted, agent_not_active, agent_key_not_enrolled (whose message tells the operator to enrol the agent's key first), request_subject_mismatch, key_mismatch, lifetime_invalid, lifetime_above_maximum (whose message names the configured maximum in seconds), agent_certificate_current (whose message names the current certificate's SHA-256 and its notAfter), issuance_leaf_not_appended or issuance_not_recorded respectively, and return no certificate; every refusal before the append leaves no issuance leaf and no issuance event, a refusal at the append leaves no issuance event, and a refusal at the event's commit leaves the appended issuance leaf in the log, since the log is append-only, with no certificate returned. THE SYSTEM SHALL NOT issue a certificate automatically when a grant is given, SHALL NOT take a grant, a list of grants or a window of a grant from the issuance call, SHALL NOT issue a window longer than the configured maximum, SHALL NOT take the default or the maximum from the issuance call, SHALL NOT take the key it compares against from the issuance call, SHALL NOT refuse an agent because it holds no grant, SHALL NOT take an execution id, SHALL NOT put more than one claim, a scope member, a credential value, a secret handle or any extension under 1.3.6.1.4.1.66364.1 in the certificate, SHALL NOT return a certificate whose issuance leaf is not in the log, SHALL NOT remove or rewrite a leaf of the certificate log, SHALL NOT switch on rcgen's own Authority Key Identifier or change any file under crates/lys-core, SHALL NOT let the seed appear in Debug output, logs, errors, events, certificates, responses, start commands or the configuration example, SHALL NOT ship a test anchor in the service's configuration, and SHALL NOT expose an issuing path to lys (the CLI) or Cambium.

**Acceptance:**
- Reading a 32-byte seed file with mode 0600 yields an issuer whose public key equals the Ed25519 public key the test derives from those 32 bytes itself.
- A file with mode 0640 refuses with issuer_key_permissions, a file with mode 0604 refuses with issuer_key_permissions, a 31-byte file with mode 0600 refuses with issuer_key_length, and a missing path refuses with issuer_key_missing; each message contains the path and not the seed's hex, and the test asserts four refusal cases ran.
- The Debug output of the loaded issuer and of each of the four refusals contains neither the seed's lowercase hex nor its standard base64.
- An active agent agent-01 holding grants grant-01 and grant-02, whose enrolled key is the key of a certificate-signing request with common name agent-01, issued through POST /identity/agents/agent-01/certificate by the admitted operator asking for lifetime 86400, shorter than the default, returns a certificate whose subject common name is agent-01, which lys-core's verify_certificate_chain_at accepts at its own notBefore plus 60 seconds against the directory's issuer public key, whose notAfter minus its notBefore is 86400 seconds, whose extension under 1.3.6.1.4.1.66364.2.1 decodes with R4 to holder agent-01 listing grant-01 and grant-02, each with the time window the directory records for it, whose extension under 2.5.29.35 is non-critical and whose value is 30168014 followed by the first 20 bytes of the SHA-256 of the directory issuer key's DER SubjectPublicKeyInfo, which the test computes itself from that key's public bytes, and which carries no extension under 1.3.6.1.4.1.66364.1; the directory records exactly one issuance event, of kind agent_certificate_issued, naming agent-01 and the SHA-256 of the returned DER.
- The certificate log after that call holds exactly one more leaf than before, an issuance leaf whose certificate DER is the returned DER byte for byte; DIRECTORY-013's fold over the log holds that certificate issued and not revoked; and after a revocation leaf for it is appended with DIRECTORY-013's append under the directory's issuing identity, the fold holds it revoked.
- Writing that certificate and the directory issuer's lys-core issuer_certificate_der each as PEM, `openssl verify -CAfile` with the issuer certificate on the agent certificate exits 0, and `openssl x509 -text` on the agent certificate prints an Authority Key Identifier whose hex equals the Subject Key Identifier it prints for the issuer certificate.
- The configuration example deploy/identity/config.example.toml names the issuer key file's path, the certificate log's path, the default lifetime 2592000 and the maximum lifetime 7776000, and contains no 64-character hexadecimal string.
- For active agent agent-08, holding no grant, with an enrolled key and no certificate, a call with no lifetime asked for returns a certificate whose extension under 1.3.6.1.4.1.66364.2.1 decodes with R4 to holder agent-08 listing no grant, and records one issuance event.
- For active agent agent-09, holding only grant grant-09 whose window the directory records with a start and no end, with an enrolled key and no certificate, a call returns a certificate whose extension under 1.3.6.1.4.1.66364.2.1 decodes with R4 to holder agent-09 listing grant-09 with that start and no end.
- With the shipped configuration, issuing to active agent agent-05, holding grant grant-05 with an enrolled key and no certificate, with no lifetime asked for returns a certificate whose notAfter minus its notBefore is 2592000 seconds.
- For active agent agent-06, holding grant grant-06 with an enrolled key and no certificate, a call asking for lifetime 7776001 refuses with lifetime_above_maximum, whose message contains `7776000`, returns no certificate bytes, appends no leaf and records no event, and a following call for agent-06 asking for lifetime 7776000 returns a certificate whose notAfter minus its notBefore is 7776000 seconds.
- Calling the route for agent-01 while registered, while suspended and while retired each refuses with agent_not_active, returns no certificate bytes and leaves the directory's event count and the certificate log's leaf count unchanged; the test asserts three cases ran.
- For active agent agent-04 holding grant grant-04 with no enrolled key, a call with a request whose common name is agent-04 refuses with agent_key_not_enrolled, whose message contains `enrol the agent's key first`, returns no certificate bytes, appends no leaf and records no event.
- A request with common name agent-01 signed by an Ed25519 key other than agent-01's enrolled key refuses with key_mismatch, returns no certificate bytes, appends no leaf and records no event.
- A second call for agent-01 with the injected clock at the first certificate's notBefore plus 3600 seconds refuses with agent_certificate_current, whose message contains the first certificate's SHA-256 in lowercase hex and its notAfter in decimal seconds since the Unix epoch, returns no certificate bytes and leaves one issuance event and one issuance leaf.
- A call for agent-01 with the injected clock at the first certificate's notAfter plus one second issues a new certificate, and its issuance event names the first certificate's SHA-256 as the certificate it follows.
- For active agent agent-07, holding grant grant-07 with an enrolled key and a current certificate, after a revocation leaf for that certificate is appended with DIRECTORY-013's append under the directory's issuing identity, a call for agent-07 with the injected clock at that certificate's notBefore plus 3600 seconds issues a new certificate, and its issuance event names the revoked certificate's SHA-256 as the certificate it follows.
- A request with common name agent-02 presented for agent-01 refuses with request_subject_mismatch; lifetime 0 refuses with lifetime_invalid; an unauthenticated caller refuses with not_admitted; each appends no leaf and records no event.
- Granting grant-01 to active agent agent-03, which has an enrolled key and no certificate, records no issuance event, appends no leaf and issues no certificate.
- A failure injected at the certificate log's append refuses with issuance_leaf_not_appended and no certificate bytes, and after reopening, the directory holds no issuance event for that call.
- A failure injected at the issuance event's commit refuses with issuance_not_recorded and no certificate bytes, and after reopening, the directory holds no issuance event for that call.
- A drift injection that removes only the current-certificate check makes exactly one test fail, the second-call case.

**Files:**
- create: crates/lys-identity/src/capability/issue.rs
- create: crates/lys-identity/tests/capability_issue.rs
- create: crates/lys-identity-server/src/issuer_key.rs
- create: crates/lys-identity-server/tests/issuer_key.rs
- create: crates/lys-identity-server/src/certificate_route.rs
- create: crates/lys-identity-server/tests/certificate_route.rs
- modify: crates/lys-identity/src/capability/mod.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: deploy/identity/config.example.toml

**Checklist:**
- C254 — When the operator calls the directory's issuance route with an active agent's certificate-signing request, the directory issues that enduring agent a certificate, subject its directory id, carrying exactly one lys/agent-capability/v1 claim listing every grant it holds at issuance, an empty list when it holds none, under an issuer key held in the service's custody and named in the certificate's Authority Key Identifier by its 20-byte issuer-key fingerprint so that `openssl verify -CAfile` finds the issuer, for the window asked for or the configured default of 30 days and never past the configured maximum of 90 days, appended to the certificate log as the revocation card's issuance leaf, as one directory event naming the certificate it follows; it refuses an agent that is not active, an agent holding a current certificate, one not expired, not under a replaced key and not held revoked by the revocation fold (naming it and its expiry), an agent with no enrolled key (agent_key_not_enrolled), a request whose key does not match the enrolled key (key_mismatch) and an issuance whose leaf is not appended; nothing issues when a grant is given.

**Stories:**
- S115 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want my agent issued one certificate listing the grants it holds and naming its holder, against a key enrolled for it, so that it carries proof of those grants from its first spawn and a lost key or an expired certificate can be replaced.

### R7: Replace an enrolled key by an audited act that is never an overwrite

WHEN the admitted operator calls POST /identity/agents/{agent_id}/key/replacement naming the agent's enrolled key as the old key and one Ed25519 public key as the new key, THE SYSTEM SHALL make the new key the agent's enrolled key as one signed directory event of kind agent_key_replaced naming the agent, the old key and the new key, committed before the call returns (P4, P5), keeping the enrolment event and every earlier replacement event in the directory's history; from that event on, a certificate whose subject public key is the old key is not current, so issuance for the agent proceeds (R6). IF the caller is not admitted, the agent has no enrolled key, the new key is not a valid 32-byte Ed25519 public key, or the named old key is not the enrolled key, THEN THE SYSTEM SHALL refuse with not_admitted, agent_key_not_enrolled, invalid_agent_key or key_replacement_mismatch respectively, record no event and leave the enrolled key unchanged. THE SYSTEM SHALL NOT replace a key through the enrol route, SHALL NOT delete or rewrite the enrolment event or an earlier replacement event, and SHALL NOT accept private key material.

**Acceptance:**
- For agent-01 enrolled with K1, a replacement call naming old key K1 and new key K2 makes the enrolled key read K2 and adds exactly one event of kind agent_key_replaced naming agent-01, K1 and K2; after reopening the directory, the enrolled key reads K2 and the enrolment event naming K1 is still in its history.
- A replacement call for agent-01 naming old key K3, which is not its enrolled key, refuses with key_replacement_mismatch, records no event and leaves the enrolled key unchanged.
- A replacement call for registered agent agent-02, which has no enrolled key, refuses with agent_key_not_enrolled and records no event.
- A replacement call by an unauthenticated caller refuses with not_admitted, and one naming a 31-byte new key refuses with invalid_agent_key; each records no event, and the test asserts four refusal cases ran across this line and the two before it.
- For active agent agent-01 holding grant grant-01 and a current certificate issued under K1 with lifetime 86400, after the replacement from K1 to K2, an issuance call with the injected clock at that certificate's notBefore plus 3600 seconds and a request under K2 issues a certificate whose issuance event names the first certificate's SHA-256 as the certificate it follows, and an issuance call at the same instant with a request under K1 refuses with key_mismatch.
- A drift injection that removes only the old-key comparison makes exactly one test fail, the K3 case.

**Files:**
- create: crates/lys-identity/src/agent_key_replacement.rs
- create: crates/lys-identity/tests/agent_key_replacement.rs
- create: crates/lys-identity-server/src/agent_key_replacement_route.rs
- create: crates/lys-identity-server/tests/agent_key_replacement_route.rs
- modify: crates/lys-identity/src/lib.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C255 — Replacing an agent's enrolled key is one audited directory event naming the old and new key, never an overwrite; an old key that is not the enrolled one is refused with key_replacement_mismatch, and a certificate under the replaced key is no longer current.

**Stories:**
- S115 (Grant holder and reviewer, Exercises or delegates current authority and verifies its exact origin) — As a responsible person, I want my agent issued one certificate listing the grants it holds and naming its holder, against a key enrolled for it, so that it carries proof of those grants from its first spawn and a lost key or an expired certificate can be replaced.

### R8: Name verify_agent_capability and make it refuse what the claim does not cover

WHEN verify_agent_capability is given a certificate's DER, a set of trusted issuer public keys, an instant and the grant id the caller acts under, THE SYSTEM SHALL, in this order: find the signing key, the key of the set under which lys-core's verify_certificate_chain_at accepts the certificate at the certificate's own notBefore; read the extension under 2.5.29.35 with decode_extension and compare its keyIdentifier with the signing key's issuer-key fingerprint, the first 20 bytes of the SHA-256 of its DER SubjectPublicKeyInfo; read the extension under 1.3.6.1.4.1.66364.2.1 with decode_extension; decode it with R4; check the claim's holder against the certificate subject's common name; check that the grant acted under is one of the claim's listed grant ids; check the instant against the certificate's own window with verify_certificate_chain_at under the signing key; and return the claim when every check passes, as what held at issuance. IF the set is empty or the certificate verifies under no key of the set, THEN THE SYSTEM SHALL refuse with certificate_chain_invalid without reading either extension. IF the Authority Key Identifier is absent, is not an AuthorityKeyIdentifier holding only a 20-byte keyIdentifier, or names a fingerprint other than the signing key's, THEN THE SYSTEM SHALL refuse with issuer_key_mismatch, its message naming the signing key's fingerprint and the fingerprint named, or `absent` when there is none. IF the claim's extension is absent or does not parse, the content type is not one it knows, the holder differs from the subject, the grant acted under is not listed, or the instant is outside the certificate's own window, THEN THE SYSTEM SHALL refuse with claim_malformed, claim_version_unknown, claim_holder_mismatch, claim_grant_mismatch or certificate_chain_invalid respectively, each message naming the values compared, and the last carrying lys-core's reason, `certificate expired` past the notAfter and `certificate not yet valid` before the notBefore. A refusal on scope is blocked on the grant representation (C5) and is not built. THE SYSTEM SHALL NOT read the extension under 1.3.6.1.4.1.66364.1, SHALL NOT read the claim before the signing key is found, SHALL NOT return a claim from a certificate that failed any check, SHALL NOT check the instant against a listed grant's window, SHALL NOT answer whether a grant still stands, SHALL NOT read a clock itself, SHALL NOT take a single issuer key in place of the set, and SHALL NOT change `lys ca verify`, `lys verify --cert` or `lys inspect cert`.

**Acceptance:**
- A certificate from the test anchor (an issuer identity built in the test from 32 bytes of 0x07, never in any configuration) for subject agent-01, issued with ttl 86400, carrying the fixture claim of R4 under 1.3.6.1.4.1.66364.2.1 and the Authority Key Identifier value 30168014324be2dea8bc44461b0233e51fa48902ed6b1cc6 under 2.5.29.35, verified at its own notBefore plus 60 seconds against the set holding only the test anchor's key, returns that claim for grant grant-01 and returns it for grant grant-02; call this the first certificate, and nb and na its own notBefore and notAfter as read from it.
- A test-anchor certificate like the first whose claim lists grant-01 with not-before 1 and not-after 1000, verified at nb+60 for grant grant-01, returns that claim, because a listed grant's window is never checked as live.
- A certificate issued by R6's issuance function in crates/lys-identity/src/capability/issue.rs under the test anchor, for agent-01 holding grant-01, verified at its own notBefore plus 60 seconds against the set holding only the test anchor's key for grant grant-01, returns a claim whose holder is agent-01 and whose listed grant ids are exactly grant-01.
- The first certificate verified against the set holding only another Ed25519 key refuses with certificate_chain_invalid, and so does the first certificate verified against the empty set, and so does a certificate from another key whose extensions under 2.5.29.35 and 1.3.6.1.4.1.66364.2.1 are each the single byte 00, proving neither extension was read.
- A test-anchor certificate like the first whose Authority Key Identifier value is 301680143dba97edb866520d36063a0d9f79576893f3b130, the fingerprint of a second issuer key built in the test from 32 bytes of 0x09, verified at nb+60 against the set holding both keys, refuses with issuer_key_mismatch, and its message contains 324be2dea8bc44461b0233e51fa48902ed6b1cc6 and 3dba97edb866520d36063a0d9f79576893f3b130.
- A test-anchor certificate like the first with no extension under 2.5.29.35 refuses with issuer_key_mismatch, and its message contains 324be2dea8bc44461b0233e51fa48902ed6b1cc6 and `absent`; one whose Authority Key Identifier value is 30228020324be2dea8bc44461b0233e51fa48902ed6b1cc671e7739af2551e0bfe68f54e, a 32-byte keyIdentifier, also refuses with issuer_key_mismatch.
- A test-anchor certificate whose extension under 1.3.6.1.4.1.66364.2.1 is the operator JSON `{"role":"admin"}` refuses with claim_malformed, and so does one that carries a valid v1 claim only under 1.3.6.1.4.1.66364.1.
- A test-anchor certificate whose claim carries content type lys/agent-capability/v2 refuses with claim_version_unknown, and its message contains `lys/agent-capability/v2`.
- A test-anchor certificate with subject agent-01 whose claim names holder agent-02 refuses with claim_holder_mismatch, and its message contains `agent-01` and `agent-02`; a claim under subject agent-01 whose holder is `grant-01` and whose one listed grant id is `agent-01` also refuses with claim_holder_mismatch.
- The first certificate verified at nb+60 for grant grant-03 refuses with claim_grant_mismatch, and its message contains `grant-03`, `grant-01` and `grant-02`.
- The first certificate verified at na+1 refuses with certificate_chain_invalid, and its message contains `certificate expired`; verified at nb-1 it refuses with certificate_chain_invalid, and its message contains `certificate not yet valid`; the test asserts that fourteen refusal cases ran across this line and the seven before it and none was accepted.
- Given the set holding the test anchor's key and the second issuer key built from 32 bytes of 0x09, a certificate from the test anchor and one from the second issuer, each for agent-01 listing grant-01 with an Authority Key Identifier naming its own signer, are each accepted at their own notBefore plus 60 seconds and return their claims, and a like certificate from a third issuer built from 32 bytes of 0x0b refuses with certificate_chain_invalid; the test asserts two acceptances and one refusal ran.
- A test-anchor certificate like the first whose claim lists no grant, verified at nb+60 for grant grant-01, refuses with claim_grant_mismatch, and its message contains `grant-01`; one whose claim lists only grant-01 with not-before 1 and no end, verified at nb+60 for grant grant-01, returns that claim; the test asserts one refusal and one acceptance ran.
- A drift injection that removes only the final check of the instant against the certificate's own window makes exactly one test fail, the expiry test holding the na+1 and nb-1 cases.
- A drift injection that removes only the listed-grant check makes exactly one test fail, the grant-03 case.
- A drift injection that removes only the comparison of a present keyIdentifier with the signing key's fingerprint makes exactly one test fail, the 0x09 Authority Key Identifier case.
- `git diff --stat` against the brief's base commit for crates/lys-core and crates/lys prints nothing, and `cargo test --workspace --all-features` passes every existing lys-core wire-vector and conformance test unchanged.

**Files:**
- create: crates/lys-identity/src/capability/verify.rs
- create: crates/lys-identity/tests/capability_verify.rs
- modify: crates/lys-identity/src/capability/mod.rs

**Checklist:**
- C256 — verify_agent_capability finds the signing key in a set of trusted issuer keys before it reads the claim and refuses an Authority Key Identifier that is absent or names another key (issuer_key_mismatch), a malformed claim, an unknown version, a holder other than the subject, a grant the claim does not list, and an instant outside the certificate's own window, each by a named refusal, with the refusal legs counted; a listed grant's window is never checked as live.

**Stories:**
- S113 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a verifier, I want to read from an agent's certificate what it was granted at issuance and have a holder, a grant or an instant outside that refused, so that a signed claim is never taken as checked when nothing checked it.

## Boundaries

- No file under crates/lys-core changes: LYS_OID_ARC, encode_extension, decode_extension, the certificate authority, its leaf parameters, its issuer certificate, verify_certificate_chain_at, the certificate request functions and every published wire vector stay byte-identical, and rcgen's own Authority Key Identifier is not switched on.
- The .1 transport and `lys ca issue --claims` stay exactly as shipped, neither deprecated nor removed; no file under crates/lys changes, so `lys ca verify`, `lys verify --cert` and `lys inspect cert` keep showing only .1.
- The typed claim is never written under 1.3.6.1.4.1.66364.1 and never under 1.3.6.1.4.1.66364.2 itself; 1.3.6.1.4.1.66364.2.1 is proposed for ratification with the format, .2 stays the family arc for agent-certificate extensions, and docs/PEN-REGISTRATION.md changes only in its .2 row, which R1 rewrites to name .2.1 for the typed capability claim while keeping the rest of that row's stated purpose and its status cell `Reserved`, and whose status cell R3 alone changes, to `In use (.2.1)`, once the ratification is recorded on the card.
- D1 to D6 in docs/design/WIRE-FORMATS.md's decision log stay exactly as written, and the lys/agent-capability/v1 row reads RATIFIED only through R3, after the ratification is recorded on the card.
- lys/delegation/v1 is not mutated, reinterpreted or used as a capability token.
- Nothing durable is signed under lys/agent-capability/v1 before its ratification is recorded; test certificates are minted afresh on every run and none is committed.
- Sign-off of this brief does not ratify the format, no code is built before sign-off, and the format's ratification is the owning lead's with a second reader, never inferred from a sign-off or a green build.
- The claim carries no revocation member, no issuer member, no scope member and no validity window of its own; a listed grant's window is a record of what held at issuance and is never checked as live.
- No scope vocabulary, scope refusal, per-execution credential, revocation fold or revocation request path, SpiceDB or per-call door check, automatic issuance on a grant, presentation at spawn, screen or agent-file change, CLI display of the typed claim, issuer key rotation or production anchor is added here, and DIRECTORY-013's files, leaf formats and lys-log-store are not changed.
- No private key material, credential value or secret handle appears in any certificate, claim, event, command, log, error or response.
- Neither lys (the CLI) nor Cambium issues the certificate or holds an issuing path.
- No agent public key is taken from an issuance call, an enrolled key is never overwritten, and DIRECTORY-003's brief and agent record files do not change.
- The IDENTITY-001 files and every other brief's ids, estimates and order stay unchanged, and CN7's ceiling does not change.

## Verification

- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory; render with python3 scripts/design/render-cluster.py docs/design/directory, render again, and confirm the second render is byte-identical; then sh scripts/design/gate.sh exits 0.
- After R1, R2 and R3: `git diff --stat` against the base commit touches only docs/design/identity/CAPABILITY-CLAIM.md, docs/design/identity/CAPABILITY-CLAIM-REVIEW.md, docs/design/WIRE-FORMATS.md, docs/PEN-REGISTRATION.md and the cluster's own documents.
- Before R3 is dispatched: run the review and card checks in blocked_by. Before R4 is dispatched: run the R3, DIRECTORY-003 and DIRECTORY-006 R1 checks in blocked_by, each with its stated result; before R5 is dispatched, run the DIRECTORY-003 execution-record check in blocked_by and confirm it prints `landed at` and the commit it read.
- At implementation, on the build venue from the exact revision: cargo fmt --all, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps; report every refusal case count and each drift injection's single failing test by name.
- `openssl x509 -inform DER -text -noout` on a certificate R6 issues under the test anchor prints an X509v3 Authority Key Identifier section whose value is the 20-byte identifier the brief pins, equal to the X509v3 Subject Key Identifier the same command prints for the test anchor's lys-core issuer certificate, and `openssl verify -CAfile` with that issuer certificate as PEM on the issued certificate as PEM exits 0.
- `git diff --stat` against the base commit for crates/lys-core and crates/lys prints nothing.

