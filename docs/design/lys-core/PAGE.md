# lys-core — what was asked, what it means, and what was written

## The words, as they were typed

A certificate is judged self-signed by its keys and signature, never by a name that looks like the authority's key.

In lys-core's authority module, verify_certificate_chain_at treats a certificate whose subject equals the authority's hex public key common name as self-signed.
A certificate issued by an authority to a subject that happens to carry that name is then taken as a root it is not.
The revocation brief 1e30a4cb recorded this as a finding in its third round.

The outcome is that a certificate counts as self-signed only when its issuer and subject keys are the same key and its signature verifies under that key.
A name match alone never makes a certificate a root, and a test builds exactly that case and shows the chain is judged on its real issuer.
No published format changes, and every certificate that verifies today and is truly self-signed still verifies.

This card also ships the documentation caveat on verify_certificate_chain that the revocation brief's first round ruled is a documentation-only lys-core release of its own.
The caveat says in plain words what the function checks and what it leaves to the caller.

The review is adversarial, because this is cryptographic code.

## What the survey found, and its angles

The words ask for two things in lys-core's `ca/authority.rs`. First, `verify_certificate_chain_at` should stop deciding "self-signed" by comparing the raw subject and issuer DN bytes. A certificate should count as self-signed only when its subject key is the issuer key and its signature verifies under that key. A test must show that an authority-issued certificate whose subject is the authority's hex-public-key common name is judged on its real issuer. Second, the rustdoc of `verify_certificate_chain` should say in plain words what it checks and what it leaves to the caller. That caveat is the "documentation-only lys-core release" that revocation brief 1e30a4cb (DIRECTORY-013) recorded as its own card. No published format may change.

### What the tree holds

- `crates/lys-core/src/ca/authority.rs:368-374` — The DN check itself: `certificate.subject().as_raw() == certificate.issuer().as_raw()` returns CertificateVerification 'self-signed certificate rejected (issuer equals subject)' before the algorithm check and before the signature check. This is the line the card replaces.
- `crates/lys-core/src/ca/authority.rs:316-357` — Rustdoc of verify_certificate_chain_at. It already calls the DN check a heuristic, not a security boundary, and at 338-341 names this exact false positive. That text, the 'rejects self-signed certificates (issuer equal to subject)' sentence and the Errors section all have to be rewritten to match the key-based rule.
- `crates/lys-core/src/ca/authority.rs:303-314` — The rustdoc of the free verify_certificate_chain is two lines that defer to _at. This is where the words put the caveat on what is checked and what is left to the caller.
- `crates/lys-core/src/ca/authority.rs:194-207` — CertificateAuthority::verify_certificate_chain, the method form. Its docs point at the free function; the caveat either lands here too or is linked from here.
- `crates/lys-core/src/ca/authority.rs:219-235` — issuer_certificate sets the issuer DN to hex_lower(authority public key), so any leaf whose subject is that same hex string gets a byte-identical subject DN. This is where the false positive comes from.
- `crates/lys-core/src/ca/authority.rs:94-124 and 155-192` — The two issuance paths. issue_certificate(hex_lower(pk), ...) builds the test case the words ask for. issue_certificate_for_request over a CSR from the authority's own key gives a certificate whose subject key is the issuer key, which the new rule would call self-signed.
- `crates/lys-core/src/ca/certificate.rs:210` — certificate_subject_public_key already pulls the 32-byte SPKI out of a certificate. It is the existing helper for comparing the subject key with the issuer key.
- `crates/lys-core/src/ca/authority_tests.rs:162-175` — self_signed_certificate_is_rejected checks against issuer key [0u8;32] and asserts only the error variant. Under a key-based rule that certificate's subject key is not the zero key, so the test would pass through the signature failure while the self-signed check never runs.
- `crates/lys-core/src/ca/request_tests.rs:497` — This test passes a CSR to verify_certificate_chain under the identity's own key and expects an error. Its outcome may change for a different reason once the DN check is gone.
- `crates/lys/src/commands/verify.rs:139 and crates/lys/src/commands/ca.rs:286` — The published lys CLI calls verify_certificate_chain_at in `lys verify --cert` and `lys ca verify`, so its accept/reject behaviour changes with lys-core.
- `crates/lys/src/commands/error.rs:79 and crates/lys/src/commands/error_tests.rs:47` — The CLI error docs list 'self-signed certificate' as a refusal cause. A test asserts the rendered display does not contain 'self-signed'.
- `crates/lys-anchor/src/admission/certificate.rs:55-56 and 222` — Anchor admission calls verify_certificate_chain, and its module doc states that the call 'rejects self-signed certificates'. That doc has to stay true.
- `docs/design/lys-core/DESIGN.md:67 and docs/design/lys-core/CHECKLIST.md:39 (C23)` — The cluster this card continues. It states 'Self-signed certificates are rejected' with no definition, and C23 is ticked on the DN behaviour. Both need the key-based definition.
- `docs/design/lys-core/DESIGN.md:71 and 150` — The cluster names revocation as absent and consumer-side. The caveat has to say the same without contradicting it.
- `CHANGELOG.md:12 (Unreleased)` — A behaviour change and a rustdoc change to a published crate need an Unreleased entry. The section already carries lys-log-store and unstable-anchor additions not yet released.
- `~/.aion/clones/briefs/1e30a4cb-49b2-4b5d-86b5-9f78744a3501/docs/design/directory/briefs/DIRECTORY-013.md:39, 45 (11), 142, 192, 211` — The revocation brief, still a draft and not on main. It rules the caveat to be a separate documentation-only lys-core release. Its R5 and restrictions also say verify_certificate_chain(_at) 'SHALL NOT change, their documentation included', and its fold (R3) calls verify_certificate_chain_at at notBefore, so a behaviour change here changes what that fold issues.

### What was already decided

- docs/design/lys-core/DESIGN.md — Line 67: verify_certificate_chain verifies with verify_strict, enforces the validity window, and 'Self-signed certificates are rejected', with no definition of self-signed.
- docs/design/lys-core/CHECKLIST.md C23 — 'Self-signed certificates are rejected by verify_certificate_chain' is ticked, and it was built as a DN comparison.
- docs/design/lys-core/CHECKLIST.md C20-C22 — Signature via verify_strict; validity window enforced; _at verifies at an explicit instant. All must keep holding.
- docs/design/lys-core/USER-STORIES.md S5 — The Norn runtime gates cross-agent calls on verify_certificate_chain_at, so accept/reject changes reach consumers.
- docs/design/lys-core/DESIGN.md:71,150 — Revocation is deliberately absent from lys-core and stays consumer-side, which is one of the things the caveat must say is left to the caller.
- ADR-039 (draft, 1e30a4cb only) — lys-core's published verify_certificate_chain keeps its meaning and keeps passing with no log. The rustdoc gap is answered by a separate documentation-only lys-core release.
- DIRECTORY-013 (draft) — R1 (11) records the missing rustdoc caveat as a known gap owned by its own card. R5 and the restrictions forbid changing verify_certificate_chain(_at) or their documentation within that brief.
- ADR-008 — An agent's file shows its lys certificate and who signed it, so an authority's verifier must not misjudge who the issuer is.
- CLAUDE.md 'Wire formats are forever' / freeze paragraph — lys-core 0.2.0 is published, and publishing lys-core again freezes lys/delegation/v1, which is held back until the ordering fold exists.
- CLAUDE.md coding standards — Cryptographic changes need an adversarial review that builds actual attacks. Gates run in both feature shapes, and cargo doc runs in both.

### What was measured

- crates/lys-core/src/ca/authority.rs length: 455 lines total, including docs (under the 500-line code limit)
- crates/lys-core/src/ca/authority_tests.rs: 361 lines, 20 functions
- Self-signed check sites in lys-core: 1 (authority.rs:370, raw DN byte equality)
- Tests that exercise the self-signed refusal: 1 (authority_tests.rs:163), using issuer key [0u8;32] and asserting only the error variant, not the reason
- Tests that build an authority-issued certificate whose subject is the authority's hex CN: 0
- Production callers of verify_certificate_chain(_at) outside lys-core: 3: lys verify.rs:139, lys ca.rs:286, lys-anchor admission/certificate.rs:222
- lys-core workspace version: 0.2.0 (Cargo.toml:13), published 2026-07-30
- lys-core src commits on main since 0.2.0 (2026-07-30): 12, including lys/delegation/v1, lys/consistency-receipt/v1 and a keys::verify behaviour fix
- Git tags in the repository: 0 (no v0.2.0 tag to branch a 0.2.x release from)
- Design documents in docs/design/lys-core: 3 markdown files (411 lines) and no design.json, so scripts/design/gate.sh skips this cluster
- Revocation brief 1e30a4cb state: 2 draft commits on top of 7b53625, not on main. The round-three finding about the hex CN is not written in its tree

### What it means for the other projects

- aion — The card runs through aion's chain (brief_card, sign-off, card_build_v3, src_pr, src_land). The revocation brief 1e30a4cb, still a draft in aion's brief clones, needs its DIRECTORY-013 restrictions reconciled with this card.
- cambium — The card lives on Cambium's board. If a crates.io release is ruled in, publishing is an outward step the chain must carry explicitly.

### The decisions it stands on

- ADR-008 (honour) — An agent's file shows who signed its certificate, so verification must judge the real issuer key and never a name.
- ADR-003 (honour) — Authority chains are pegged to real keys. A name-only root would let a subject's name stand in for authority.
-  (new) — Define self-signed in lys-core as 'subject key equals the issuer key and the signature verifies under it', replacing the DN-byte heuristic that C23 was built on.
-  (new) — Rule whether a documentation-only lys-core release is cut from main (which would freeze lys/delegation/v1) or from a 0.2.0-based branch, or is not published by this card.

### What it requires

- verify_certificate_chain_at no longer compares subject and issuer DN bytes anywhere (a search of authority.rs for `subject().as_raw() == ` finds nothing).
- A certificate issued by CertificateAuthority::issue_certificate with subject hex_lower(authority public key) verifies with verify_certificate_chain under the authority's key and is refused under any other key.
- A certificate whose subject key equals the supplied issuer key and whose signature verifies under it is refused with a self-signed reason, and the test asserts that reason, not only the error variant.
- The existing self_signed_certificate_is_rejected test is rebuilt so that the self-signed branch fires, not the signature or weak-key branch.
- Every existing lys-core, lys and lys-anchor test and the openssl_csr_interop test pass unchanged in behaviour, apart from the ruled self-signed cases.
- The rustdoc of verify_certificate_chain states what it checks (Ed25519 strict signature under the supplied key, validity window, self-signed refusal by key) and what it leaves to the caller (revocation, trust in the issuer key, subject-key possession, one level only, extension meaning).
- The rustdoc of verify_certificate_chain_at no longer describes a DN heuristic or its false positive.
- DESIGN.md:67 and CHECKLIST.md C23 state the key-based definition, and CHANGELOG Unreleased records the change.
- Every repository gate is clean in both feature shapes: fmt, clippy with and without --all-features, tests with --all-features, cargo doc with and without --all-features, and the design gate.
- A constructed-attack adversarial review is recorded on the card before landing.

### What must not change

- No published wire format or domain tag changes: lys/attestation/v2, lys/sealed-envelope/v1, lys/anchor-receipt/v1, lys/verification-bundle/v1 and lys/consistency-receipt/v1 stay byte-identical.
- Signature verification stays ed25519-dalek verify_strict. x509-parser's verify_signature is never called.
- The signatures of verify_certificate_chain, verify_certificate_chain_at and CertificateAuthority::verify_certificate_chain stay as they are.
- The issuer DN that issuance writes (hex_lower of the authority key) does not change.
- lys-core gains no revocation check. It stays consumer-side.
- No unwrap, expect or panic in library code, and no #[allow] or #[ignore] bypass.
- lys/delegation/v1 is not published as a side effect of any release this card makes.

### What we must put in place first

- The lead rules whether the words mean the refusal defect, and what happens to truly self-signed certificates that verify today with differing DNs.
- The lead rules whether this card publishes to crates.io and from which base, given that main holds the unpublished lys/delegation/v1 and there is no v0.2.0 tag.
- The order against DIRECTORY-013 is ruled, and that draft brief's 'SHALL NOT change … documentation included' lines are amended accordingly.

### The risks

- Removing the DN screen without a key-based replacement that actually fires leaves C23 true only by accident. A test that checks against key [0u8;32] passes through the signature branch instead.
- Comparing raw SPKI bytes can miss a non-canonical encoding of the issuer point, so the adversarial review must show that verify_strict and the key decoding close this.
- Certificates that verify today (authority-issued over the authority's own CSR key) would start being refused, a behaviour change for lys ca verify, lys verify --cert, anchor admission and the Norn S5 boundary.
- A release cut from main freezes lys/delegation/v1 before its ordering fold exists, which breaks the rule recorded in CLAUDE.md.
- Calling the release 'documentation-only' while it carries a verification behaviour change misstates what consumers receive.
- The draft DIRECTORY-013 fold's issued set changes for hex-CN certificates, and its restriction lines conflict with this card.
- Intra-doc links in the new caveat that point at gated items break the default cargo doc build.

### Still open

- The tree rejects that certificate as self-signed rather than accepting it as a root. Is the defect to fix the wrongful refusal (a legitimately issued certificate is refused), rather than a wrongful acceptance? The sentence of the words it stands on: "A certificate issued by an authority to a subject that happens to carry that name is then taken as a root it is not.". Why only the lead can settle it: authority.rs:368-374 returns CertificateVerification for DN equality, and authority.rs:338-341 documents this as a known false positive: such a certificate is refused, never accepted as a root. After the fix it would verify where today it fails, so `lys ca verify` and `lys verify --cert` turn from exit-nonzero to exit-0 for it.
- A certificate whose subject key is the issuer key and whose signature verifies under it, but whose issuer and subject DNs differ, verifies today and is truly self-signed. Under the new definition it would be refused. Should it now be refused (keeping C23 'self-signed are rejected'), or keep verifying as this sentence says? The sentence of the words it stands on: "No published format changes, and every certificate that verifies today and is truly self-signed still verifies.". Why only the lead can settle it: Today no certificate with equal DNs verifies at all. The only truly self-signed certificates that verify today are ones with differing DNs, including one issued by issue_certificate_for_request over a CSR from the authority's own key. The outcome sentence would reclassify these as self-signed, and C23 (CHECKLIST.md:39) says self-signed are rejected, so this sentence and the outcome conflict for exactly those certificates.
- Does this card publish a lys-core release to crates.io, and if so, cut from what? Main carries 12 unreleased lys-core commits, including the unpublished lys/delegation/v1, and this card also changes behaviour. The sentence of the words it stands on: "This card also ships the documentation caveat on verify_certificate_chain that the revocation brief's first round ruled is a documentation-only lys-core release of its own.". Why only the lead can settle it: CLAUDE.md says publishing lys-core freezes lys/delegation/v1 and that release waits for the ordering fold. There is no v0.2.0 tag to branch from. A release carrying the self-signed fix is not documentation-only. Whether a person sees a new crates.io version, and what it freezes, depends on this answer.
- The draft DIRECTORY-013 says verify_certificate_chain(_at) and their documentation SHALL NOT change, and its fold calls verify_certificate_chain_at. Does this card land first and amend DIRECTORY-013, or wait until that brief is settled? The sentence of the words it stands on: "The revocation brief 1e30a4cb recorded this as a finding in its third round.". Why only the lead can settle it: DIRECTORY-013.md:142, 192 and 211 (in clone 1e30a4cb) forbid changing these functions or their rustdoc, and its R3 fold's issued set changes when hex-CN certificates start verifying. The two cards conflict on the same lines unless their order is ruled.

### The units beyond the first

- Publish a lys-core release carrying the verify_certificate_chain caveat and the self-signed fix — Publishing is an outward, irreversible step. From main it would also freeze lys/delegation/v1, so it needs its own ruling on base and version and its own gated row.
- Amend DIRECTORY-013 (revocation brief 1e30a4cb) to stand on the changed verify_certificate_chain_at and its new documentation — The draft brief forbids changing these functions and their docs, and its R1 (11) names this caveat as an open gap. It is a separate brief with its own reviews and sign-off.

### The smallest complete shape

One lys-core change in crates/lys-core/src/ca/authority.rs, landed whole through the chain. The DN-byte screen is replaced by the key-based self-signed rule (subject SPKI equals the issuer key and the signature verifies under it), placed so it fires. authority_tests.rs gains the hex-CN test, which verifies under the authority and is refused under another key, and a self-signed test that asserts its reason and that the branch fired. The rustdoc of verify_certificate_chain, verify_certificate_chain_at and the CertificateAuthority method is rewritten with the plain caveat on what is checked and what is left to the caller. DESIGN.md:67, CHECKLIST.md C23, the lys-anchor admission doc and CHANGELOG Unreleased are updated to match. All gates run in both feature shapes, and the adversarial review is recorded. Publishing to crates.io stays out of this unit unless the lead rules otherwise.

## The roadmap row

- **RM-030** — Judge a certificate self-signed by its keys, and say what verify_certificate_chain leaves to the caller (fix, idea)
- Summary: lys-core's verify_certificate_chain_at decides self-signed by comparing distinguished-name bytes, so it refuses a certificate the authority legitimately issued to a subject named by the authority's hex public key, and accepts a certificate whose subject key is the issuer's own key when the names differ. LYSCORE-001 makes a certificate self-signed only when its subject key is the issuer key, compared as decoded Ed25519 points, and its signature verifies under that key (ADR-044); shows the change through lys ca verify, lys verify --cert and DIRECTORY-013's fold; writes the rustdoc caveat on verify_certificate_chain naming what it checks and what it leaves to the caller; corrects lys-anchor's admission doc; and records the change in CHANGELOG's Unreleased section. No wire format changes and no crates.io release is cut (ADR-045).
- Asked by: tom on 2026-09-27T12:10:45+10:00
- Context: The lys-core self-signed card on the Lys board, through brief_card: surveyed against lys main 7b53625, and the lead for lys answered its four questions in round 1, and the lead for the identity line answered the author's question on the older design. The answers, carried as settled in LYSCORE-001, ADR-044 and ADR-045: the defect is the wrongful refusal of a legitimately issued hex-common-name certificate, which verifies after the fix; a certificate self-signed by keys is refused whatever its names, keeping C23; the card cuts no crates.io release, because publishing lys-core freezes lys/delegation/v1 before its ordering fold; and the revocation card DIRECTORY-013 lands first, unamended, with this card's build blocked until it has; and the hand-written lys-core design (C1 to C65, S1 to S23) is carried into JSON by its own card first, with this card's build blocked until origin/main's checklist.json holds C1 to C65, after which this card adds its rows and commits a render that keeps the older text; and this record's own principles, constraints, goals, non-goals and inventory are appended to the carried design under the next free P and CN numbers, never replacing it, while the carried title, intention, problem and solution stay the cluster's. In the next round the lead accepted three review findings, carried as settled: R1 gains a test whose subject key is an ECDSA P-256 key signed validly by K, which verifies (Ok(()), read by running it against the tree) and is not refused as self-signed; the five caveat bullets of R2 begin with one wording used in both spec and acceptance; and R6 names none of DIRECTORY-013's fold interface, which cannot be read before it lands, binds its test to that landed interface by name, observes one issuance, one revocation and one leaf refused as certificate_chain_invalid at index 1, and is blocked until crates/lys-identity/tests/revocation_fold.rs exists on origin/main.
- Quote: A certificate is judged self-signed by its keys and signature, never by a name that looks like the authority's key.

In lys-core's authority module, verify_certificate_chain_at treats a certificate whose subject equals the authority's hex public key common name as self-signed.
A certificate issued by an authority to a subject that happens to carry that name is then taken as a root it is not.
The revocation brief 1e30a4cb recorded this as a finding in its third round.

The outcome is that a certificate counts as self-signed only when its issuer and subject keys are the same key and its signature verifies under that key.
A name match alone never makes a certificate a root, and a test builds exactly that case and shows the chain is judged on its real issuer.
No published format changes, and every certificate that verifies today and is truly self-signed still verifies.

This card also ships the documentation caveat on verify_certificate_chain that the revocation brief's first round ruled is a documentation-only lys-core release of its own.
The caveat says in plain words what the function checks and what it leaves to the caller.

The review is adversarial, because this is cryptographic code.
- Cluster: lys-core; briefs: LYSCORE-001
- Notes: Ids: RM-030, ADR-044 and ADR-045 follow the highest ids on main (RM-016, ADR-018 at 7b53625), on origin's brief/* branches (RM-026 and ADR-040 on brief/lys-anchor/6b1c7df6) and in the local brief clones' drafts (RM-029 and ADR-043), checked on 2026-09-27. LYSCORE-001 is the brief id this run was given; the local draft in brief clone a31929b9 (the estate-roots card, RM-019, not pushed) also writes a docs/design/lys-core/design.json and a LYSCORE-001, so whichever lands second renumbers its brief and merges the cluster files. Ordering: LYSCORE-001 depends on DIRECTORY-013 (RM-025 in its draft, not on main), so depends_on stays empty until that row is on main; the brief's blocked_by carries the command that reads DIRECTORY-013's roadmap row on origin/main as landed. The older design: with design.json in docs/design/lys-core, scripts/design/gate.sh renders the cluster and compares DESIGN.md, CHECKLIST.md and USER-STORIES.md, which hold the hand-written design (C1 to C65, S1 to S23). Ruled: a separate card, carded by the lead on the lys board, carries them into the JSON first; LYSCORE-001's blocked_by carries the command that reads origin/main's checklist.json for C1 to C65, and its R7 adds C23's restatement and C66 to C74, appends this record's design rows after the carried ones under the next free P and CN numbers, and commits the render. Until the carry card lands, this draft's cluster files are not the whole lys-core record, so the design leg of the gate cannot pass on a tree holding this draft alone. Further units, not written: Publish a lys-core release carrying the verify_certificate_chain caveat and the self-signed fix; Amend DIRECTORY-013 (revocation brief 1e30a4cb) to stand on the changed verify_certificate_chain_at and its new documentation.

## The design

---
type: design
cluster: lys-core
title: Lys Core — a certificate is self-signed by its keys, and the chain verifier says what it leaves to the caller
---

# Lys Core — a certificate is self-signed by its keys, and the chain verifier says what it leaves to the caller

> **Cluster:** lys-core

## Intention

A stranger holding a lys certificate and an authority's public key should get the same answer from lys-core as from reading the certificate by hand: who really signed it, and whether it is one the authority issued. Whether a certificate is self-signed is a fact about keys and a signature, never about a name, so a subject that happens to carry the authority's hex-key common name is judged on the key that really signed it.

The verifier should also say plainly what it proves and what it does not. A caller reading the documentation of verify_certificate_chain should learn, without reading the code, which checks ran and which questions (revocation, trust in the issuer key, possession of the subject key, anything above one level, what extensions mean) are still theirs to answer.

## Problem

verify_certificate_chain_at (crates/lys-core/src/ca/authority.rs) decides "self-signed" by comparing the raw subject and issuer distinguished-name bytes, before the algorithm and signature checks. The authority writes its issuer name as the lowercase hex of its public key, so a certificate the authority legitimately issues to a subject named with that same hex string has byte-identical names and is refused, even though its signature verifies under the authority's key. The rustdoc already records this as a known false positive of a heuristic. In the other direction, a certificate whose subject key is the issuer's own key and whose signature verifies under it (for example one issued by issue_certificate_for_request over a request made with the authority's own key) carries different names and verifies today, although it is self-signed by keys. The published lys CLI (lys ca verify, lys verify --cert), lys-anchor admission and the Norn runtime's MCP boundary all inherit both errors.

Separately, verify_certificate_chain's rustdoc is two lines that defer to the _at form and never says what is left to the caller. The revocation brief (DIRECTORY-013) records that gap as owned by its own card, and its fold calls verify_certificate_chain_at, so the order of the two cards matters.

## Solution

One change in lys-core's ca/authority.rs, landed whole through the chain after DIRECTORY-013 has landed (ADR-045).

The distinguished-name comparison is removed. After the signature has verified strictly under the supplied issuer key, the verifier reads the certificate's subject public key and compares it with the supplied issuer key as decoded Ed25519 curve points; when they are the same point the certificate is self-signed and is refused with a CertificateVerification reason that says so (ADR-044). Placing the judgement after verify_strict makes it exactly the words' definition (same key and the signature verifies under that key), and comparing decoded points rather than raw bytes means a non-canonical encoding of the issuer's point is still the issuer's key. A subject key that is not a readable 32-byte Ed25519 key, or does not decode to a point, is not the issuer's key. The check order is otherwise unchanged: parse, Ed25519 algorithm, signature length, issuer-key decoding, strict signature, self-signed, validity window.

The rustdoc of the free verify_certificate_chain carries the caveat in full: what it checks, and what it leaves to the caller. verify_certificate_chain_at and CertificateAuthority::verify_certificate_chain describe the key-based rule, drop every mention of the name heuristic, and point to that caveat. The doc links only to items present in both feature shapes.

Consumers see the change directly. lys ca verify and lys verify --cert exit 0 for the hex-common-name certificate and exit 1 for a certificate self-signed by keys; their failure message stays the one non-oracle line. lys-anchor's admission module doc keeps saying it rejects self-signed certificates, now defined by keys. DIRECTORY-013's fold issues a legitimately issued hex-common-name leaf and still records certificate_chain_invalid for a leaf self-signed by keys. CHANGELOG's Unreleased section records both changes. No crates.io release is cut (ADR-045). No wire format, domain tag or issuance field changes (CN1, CN4).

The record follows the older design rather than replacing it. The hand-written DESIGN.md, CHECKLIST.md and USER-STORIES.md (C1 to C65, S1 to S23) are first carried into design.json, checklist.json and stories.json by their own card, and this brief's build waits for it. The brief then restates C23 in the carried checklist, adds C66 to C74 after C65, finds S5 and S11 already carried, and restates the carried design's self-signed sentence with the key-based definition. The carried title, intention, problem and solution stay the cluster's; this record's own stay here and in the brief. This record's principles, constraints, goals, non-goals and inventory rows are appended after the carried ones, each principle and constraint renumbered to the next free P and CN number after the carried design's highest, so P1 to P4 and CN1 to CN8 here are this record's numbers and not the cluster's. The render it commits keeps every other carried item as it was.

## Principles

- **P1** — A certificate's role is judged by keys and signatures, never by a name: no distinguished-name comparison decides whether a certificate is accepted or refused.
- **P2** — The self-signed judgement is only reached once the signature has verified under the supplied key, so it can never stand in for the signature check.
- **P3** — Documentation states what is proven and what is not; a caller must not have to read the code to learn what is still theirs to check.
- **P4** — The CLI's refusal stays non-oracle: one message for every failed check, the self-signed refusal included.

## Decisions

- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
- ADR-044 — A certificate is self-signed when its subject key is the issuer key and its signature verifies under it, never by its names — Self-signed is judged by keys: a certificate is self-signed when its subject public key is the supplied issuer key, compared as decoded Ed25519 points, and its signature verifies under that key with verify_strict, and such a certificate is refused whatever its names. A certificate issued to a subject whose name equals the issuer's hex-key common name is judged on its real issuer and verifies. Rejected: keeping the DN-byte comparison as a heuristic screen; keeping truly self-signed certificates with differing names verifying.
- ADR-045 — The self-signed fix and the verify_certificate_chain caveat land on main with no lys-core release, after DIRECTORY-013 — This card lands the key-based self-signed rule and the rustdoc caveat on main and cuts no crates.io release; a release is its own act after the ordering fold. DIRECTORY-013 lands first and is not amended; this card's build is blocked until DIRECTORY-013's roadmap row reads landed on origin/main. Rejected: a documentation-only release cut from main or from a 0.2.0 base in this card; landing this card first and amending DIRECTORY-013.

## Goals

- crates/lys-core/src/ca/authority.rs contains no call to .subject() or .issuer() on a parsed certificate.
- A certificate the authority issues to the subject named by its own lowercase-hex public key verifies under the authority's key and is refused under any other key, in lys-core and through lys ca verify and lys verify --cert.
- A certificate whose subject key is the supplied issuer key and whose signature verifies under it is refused with the reason 'self-signed certificate rejected (subject key is the issuer key)'.
- The rustdoc of verify_certificate_chain names every check it makes and every question it leaves to the caller, and both cargo doc shapes build with no warning.
- DIRECTORY-013's fold issues a legitimately issued hex-common-name leaf and records certificate_chain_invalid for a leaf self-signed by keys.
- docs/design/lys-core/checklist.json holds C1 to C74 in order, the carried C1 to C65 unchanged but for C23, and sh scripts/design/gate.sh exits 0.

## Non-Goals

- Publishing a lys-core release to crates.io — Publishing lys-core freezes the unpublished lys/delegation/v1, which waits for the ordering fold; cutting a release is its own act afterwards (ADR-045).
- Amending DIRECTORY-013's brief — Its 'SHALL NOT change' lines bind that card's own build; it lands first and is not edited here (ADR-045).
- A revocation check in lys-core — Revocation stays consumer-side; the caveat names it as the caller's.
- Walking a chain above one level, or checking the issuer's basic constraints or key usage — The function verifies one signature under one supplied key; the caveat names this as the caller's.
- A distinct CLI message for the self-signed refusal — The CLI refusal is deliberately non-oracle (P4).

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/lys-core/design.json` | this brief's cluster record; R7 restates the carried design's self-signed sentence and appends this record's principles, constraints, goals, non-goals and inventory under the next free numbers | LYSCORE-001 |
| `docs/design/lys-core/checklist.json` | the checklist rows LYSCORE-001 delivers; R7 adds them to the carried checklist | LYSCORE-001 |
| `docs/design/lys-core/stories.json` | the stories LYSCORE-001 serves | LYSCORE-001 |
| `docs/design/lys-core/briefs/LYSCORE-001.json` | the brief: self-signed by keys, the verifier's caveat, and their consumers | LYSCORE-001 |
| `docs/design/lys-core/DESIGN.md` | rendered from design.json; R7 commits the render | LYSCORE-001 |
| `docs/design/lys-core/CHECKLIST.md` | rendered from checklist.json; R7 commits the render | LYSCORE-001 |
| `docs/design/lys-core/USER-STORIES.md` | rendered from stories.json; R7 commits the render | LYSCORE-001 |
| `crates/lys-core/src/ca/authority.rs` | issuance and chain verification; the self-signed judgement and the verifier rustdoc |  |
| `crates/lys-core/src/ca/authority_tests.rs` | unit tests of authority.rs |  |
| `crates/lys/tests/certified_attestation_tests.rs` | CLI tests of lys verify --cert and the certificate it joins |  |
| `crates/lys-anchor/src/admission/certificate.rs` | anchor admission by certificate; its module doc names what verify_certificate_chain rejects |  |
| `CHANGELOG.md` | release record; the Unreleased section |  |
| `crates/lys-identity/tests/revocation_fold.rs` | the fold's tests, created by DIRECTORY-013 in the directory cluster | DIRECTORY-013 |

## Inventory

- `crates/lys-core/src/ca/authority.rs` — 455 lines with docs. Lines 368-374 refuse a certificate whose raw subject and issuer DN bytes are equal, before the algorithm and signature checks; lines 316-357 document that as a heuristic with a known hex-common-name false positive; lines 303-314 are verify_certificate_chain's two-line rustdoc; lines 194-207 the method form; lines 219-235 write the issuer DN as the lowercase hex of the authority key.
- `crates/lys-core/src/ca/authority_tests.rs` — 361 lines, 20 test functions; self_signed_certificate_is_rejected (line 163) checks an rcgen self-signed certificate under key [0u8; 32] and asserts only the error variant, so it passes through the signature branch; no test builds a hex-common-name certificate.
- `crates/lys-core/src/ca/certificate.rs` — certificate_subject_public_key (line 210) reads a certificate's 32-byte Ed25519 subject key: Ed25519 algorithm, no parameters, no unused bits, exactly 32 bytes.
- `crates/lys-core/src/ca/request_tests.rs` — a_request_is_not_accepted_as_a_certificate (line 489) passes a PKCS#10 request to verify_certificate_chain and expects CertificateParsing or CertificateVerification; a request does not parse as a certificate.
- `crates/lys/src/commands/ca.rs` — lys ca verify calls verify_certificate_chain_at (line 286) and maps every CertificateVerification to the one CertificateVerificationFailed message.
- `crates/lys/src/commands/verify.rs` — lys verify --cert calls verify_certificate_chain_at (line 139).
- `crates/lys/src/commands/error_tests.rs` — line 47 asserts the CertificateVerificationFailed display does not contain 'self-signed'.
- `crates/lys/tests/certified_attestation_tests.rs` — 380 lines; its Fixture generates a CA key, holders, presented-key certificates via lys ca request and lys ca issue --request, attestations, and runs lys verify --cert.
- `crates/lys-anchor/src/admission/certificate.rs` — module doc lines 55-56 say verify_certificate_chain 'rejects self-signed certificates'; line 222 calls it.
- `CHANGELOG.md` — the [Unreleased] section (line 12) carries lys-log-store and unstable-anchor additions not yet released.
- `docs/design/lys-core/DESIGN.md` — the earlier markdown design of this cluster; line 67 says 'Self-signed certificates are rejected' with no definition.
- `docs/design/lys-core/CHECKLIST.md` — the earlier markdown checklist, C1 to C65; C23 'Self-signed certificates are rejected by verify_certificate_chain' is ticked on the DN behaviour.
- `docs/design/lys-core/USER-STORIES.md` — the earlier markdown stories, S1 to S23; S5 and S11 are the consumers of chain verification this change reaches.

## Constraints

- **CN1** — No published wire format or domain tag changes: lys/attestation/v2, lys/sealed-envelope/v1, lys/anchor-receipt/v1, lys/verification-bundle/v1 and lys/consistency-receipt/v1 stay byte-identical, and lys/delegation/v1 is untouched.
- **CN2** — Certificate signatures are verified only with ed25519-dalek verify_strict; x509-parser's verify_signature is never called.
- **CN3** — The signatures of verify_certificate_chain, verify_certificate_chain_at and CertificateAuthority::verify_certificate_chain do not change.
- **CN4** — Issuance does not change: the issuer DN stays the lowercase hex of the authority's public key and both issuance paths write the same fields.
- **CN5** — lys-core gains no revocation check.
- **CN6** — No unwrap, expect, panic, todo, unimplemented or unreachable in library code, and no #[allow], #[ignore], _-prefixed unused binding or #[cfg(any())] bypass.
- **CN7** — No lys-core version bump and no crates.io publish; lys/delegation/v1 is not frozen as a side effect.
- **CN8** — Rustdoc outside the unstable-anchor feature links no item gated by it, so cargo doc --no-deps builds with no warning in the default shape.


---
type: brief
id: LYSCORE-001
cluster: lys-core
title: Judge a certificate self-signed by its keys, and say what verify_certificate_chain leaves to the caller
---

# LYSCORE-001: Judge a certificate self-signed by its keys, and say what verify_certificate_chain leaves to the caller

> **Cluster:** lys-core
> **Depends on:** DIRECTORY-013
> **Blocked by:** DIRECTORY-013 landed on main: the build starts only when this command prints landed: git fetch -q origin main && git show origin/main:docs/design/roadmap.json | python3 -c "import json,sys; rows=[i for i in json.load(sys.stdin)['items'] if 'DIRECTORY-013' in i['links']['briefs']]; print(rows[0]['status'] if len(rows)==1 else 'absent')", The lys-core carry card landed on main (it carries every item of the hand-written docs/design/lys-core DESIGN.md, CHECKLIST.md and USER-STORIES.md into design.json, checklist.json and stories.json, keeping each C and S id and its text): the build starts only when this command prints carried: git fetch -q origin main && git show origin/main:docs/design/lys-core/checklist.json | python3 -c "import json,sys; ids={i['id'] for s in json.load(sys.stdin)['sections'] for i in s['items']}; print('carried' if all('C%d' % n in ids for n in range(1, 66)) else 'absent')", crates/lys-identity/tests/revocation_fold.rs exists on main, created by DIRECTORY-013 (R6 adds to it and does not create it): the build starts only when this command exits 0: git fetch -q origin main && git cat-file -e origin/main:crates/lys-identity/tests/revocation_fold.rs
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
> - ADR-044 — A certificate is self-signed when its subject key is the issuer key and its signature verifies under it, never by its names — Self-signed is judged by keys: a certificate is self-signed when its subject public key is the supplied issuer key, compared as decoded Ed25519 points, and its signature verifies under that key with verify_strict, and such a certificate is refused whatever its names. A certificate issued to a subject whose name equals the issuer's hex-key common name is judged on its real issuer and verifies. Rejected: keeping the DN-byte comparison as a heuristic screen; keeping truly self-signed certificates with differing names verifying.
> - ADR-045 — The self-signed fix and the verify_certificate_chain caveat land on main with no lys-core release, after DIRECTORY-013 — This card lands the key-based self-signed rule and the rustdoc caveat on main and cuts no crates.io release; a release is its own act after the ordering fold. DIRECTORY-013 lands first and is not amended; this card's build is blocked until DIRECTORY-013's roadmap row reads landed on origin/main. Rejected: a documentation-only release cut from main or from a 0.2.0 base in this card; landing this card first and amending DIRECTORY-013.
> **Checklist:**
> - C23 — Self-signed certificates are rejected by verify_certificate_chain, where a certificate is self-signed when its subject public key is the supplied issuer key, compared as decoded Ed25519 points, and its signature verifies under that key.
> - C66 — verify_certificate_chain_at compares no subject or issuer distinguished-name bytes: crates/lys-core/src/ca/authority.rs calls neither .subject() nor .issuer() on a parsed certificate.
> - C67 — A certificate issued by CertificateAuthority::issue_certificate to the subject named by the authority's lowercase-hex public key verifies under the authority's key and is refused under any other key.
> - C68 — A certificate issued by issue_certificate_for_request over a request made with the authority's own key is refused by verify_certificate_chain as self-signed.
> - C69 — The rustdoc of verify_certificate_chain states what it checks and what it leaves to the caller.
> - C70 — The rustdoc of verify_certificate_chain_at and CertificateAuthority::verify_certificate_chain states the key-based self-signed rule and mentions no distinguished-name heuristic.
> - C71 — lys ca verify and lys verify --cert exit 0 for a certificate issued to the subject named by the issuer's lowercase-hex public key, and exit 1 for a certificate self-signed by keys.
> - C72 — lys-anchor's admission module doc says verify_certificate_chain rejects certificates whose subject key is the issuer key.
> - C73 — CHANGELOG.md's Unreleased section records that a hex-common-name certificate now verifies, that a certificate self-signed by keys is now refused, and the new rustdoc caveat on verify_certificate_chain.
> - C74 — DIRECTORY-013's fold issues a legitimately issued hex-common-name leaf and records certificate_chain_invalid for a leaf self-signed by keys.
> **Stories:**
> - S5 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to verify a counterparty's certificate chain at the MCP boundary — at the dispatch instant, via `verify_certificate_chain_at` — so that cross-agent tool calls are gated on legitimate, unexpired, capability-scoped identity.
> - S11 (Lys CLI Operator and Auditor, Checking certificates and certified statements) — As an auditor, I want `lys ca verify` with an explicit instant so that I can check whether a certificate was valid at the time a disputed action occurred, not just at the time of my audit.

## Purpose

lys-core's chain verifier decides self-signed by comparing names, so it refuses a certificate the authority legitimately issued to a subject carrying the authority's hex-key name, and accepts a certificate whose subject key is the issuer's own key when the names differ. This brief makes the judgement one of keys and signature (ADR-044), shows it through the published CLI and through DIRECTORY-013's fold, and writes the caveat on verify_certificate_chain that tells a caller what is proven and what is still theirs. It lands on main with no crates.io release (ADR-045).

## Task

Seven rows, in order. R1 replaces the distinguished-name screen in verify_certificate_chain_at with the key-based self-signed rule, placed after strict signature verification, and adds the tests that show it: the hex-common-name certificate judged on its real issuer, the rebuilt self-signed test that makes the self-signed branch fire and asserts its reason, the certificate whose subject key is not a readable Ed25519 key, the two kinds of certificate this reclassifies from verifying to refused (one issued by issue_certificate_for_request over a request made with the authority's own key, and one signed by a key over that same key as subject key under differing names), and the decoded-point comparison. R2 rewrites the rustdoc of verify_certificate_chain, verify_certificate_chain_at and CertificateAuthority::verify_certificate_chain. R3 adds two CLI tests. R4 corrects lys-anchor's admission module doc. R5 adds the CHANGELOG entry. R6 adds one test to DIRECTORY-013's fold tests, calling the fold by the names its landed interface carries. R7 adds this brief's checklist rows to the carried lys-core checklist, restates the carried design's self-signed sentence, appends this brief's principles, constraints, goals, non-goals and inventory rows after the carried ones under the next free P and CN numbers, and commits the render, which keeps the older text. This is a behaviour change in both directions and is intended: a hex-common-name certificate refused today verifies, and a certificate self-signed by keys that verifies today when its names differ is refused. Every certificate that verifies today and is not self-signed by keys still verifies. C23 keeps its promise under the key-based definition of self-signed; its earlier ticked state rested on the name comparison, so it is restated here and ticks again only on review. The build is blocked until DIRECTORY-013 has landed on main with its fold tests file crates/lys-identity/tests/revocation_fold.rs, and until the lys-core carry card has landed on main (blocked_by, one command each); DIRECTORY-013's brief is not amended. In acceptance lines, <base> is the main commit the build branched from, after both have landed. Out of scope, each a later unit: publishing a lys-core release to crates.io; amending DIRECTORY-013. Out of scope entirely: a revocation check in lys-core, chain walking, a distinct CLI message for the self-signed refusal, carrying the older lys-core design into JSON (the carry card's work), and any hand edit of docs/design/lys-core's markdown files.

## Requirements

### R1: Judge a certificate self-signed by its keys and signature, never by its names

In crates/lys-core/src/ca/authority.rs, verify_certificate_chain_at SHALL check, in this order: the DER parses as an X.509 certificate; its signature algorithm is Ed25519; its signature is 64 bytes; the supplied issuer key decodes to an Ed25519 point; the signature verifies under the supplied issuer key with ed25519-dalek verify_strict; the certificate is not self-signed; the instant lies inside notBefore and notAfter, both inclusive. WHEN the signature has verified under the supplied issuer key AND the certificate's subject public key is the same Ed25519 key as the supplied issuer key, THE SYSTEM SHALL refuse the certificate with TrustError::CertificateVerification whose reason is exactly 'self-signed certificate rejected (subject key is the issuer key)'. Two keys are the same key when both decode to Ed25519 points and the decoded points are equal, so a non-canonical encoding of the issuer's point is the issuer's key; the comparison is made by a private function of authority.rs over two 32-byte keys. The subject key is read as certificate_subject_public_key reads it (Ed25519 algorithm, no parameters, no unused bits, exactly 32 bytes). IF the subject key cannot be read that way, or does not decode to an Ed25519 point, THEN THE SYSTEM SHALL treat it as not the issuer's key and SHALL NOT refuse the certificate as self-signed for that reason. THE SYSTEM SHALL NOT compare the subject and issuer distinguished names, or any name, to decide whether a certificate is self-signed, and a name match SHALL NOT refuse or accept a certificate. IF the signature does not verify under the supplied issuer key, THEN THE SYSTEM SHALL refuse with the reason 'certificate signature did not verify against the issuer public key' whatever the subject key is, and SHALL NOT report the certificate as self-signed. THE SYSTEM SHALL NOT call x509-parser's verify_signature, SHALL NOT change the signature of verify_certificate_chain, verify_certificate_chain_at or CertificateAuthority::verify_certificate_chain, SHALL NOT change either issuance path or the issuer DN they write, and SHALL NOT use unwrap, expect or panic in library code. This is a behaviour change on both sides: a certificate the authority legitimately issues to the subject named by its own lowercase-hex public key, refused today, verifies; a certificate whose subject key is the issuer key and whose signature verifies under it, accepted today when its names differ, is refused. The certificates reclassified from verifying to refused are of two kinds, each with its own acceptance line: one issued by issue_certificate_for_request over a request made with the authority's own key, and one any other tool signs with a key over that same key as subject key under a subject name that differs from its issuer name. issue_certificate generates a fresh subject key, so it produces neither. Every certificate that verifies today and is not self-signed by keys SHALL still verify.

**Acceptance:**
- rg -n '\.subject\(\)|\.issuer\(\)' crates/lys-core/src/ca/authority.rs prints nothing.
- Test hex_common_name_certificate_is_judged_on_its_real_issuer in crates/lys-core/src/ca/authority_tests.rs: authority A issues a certificate with issue_certificate to the subject hex_lower(&A.public_key_bytes()) with a 1-hour TTL; the test asserts the parsed certificate's raw subject DN equals its raw issuer DN; verify_certificate_chain(der, &A.public_key_bytes()) returns Ok(()); verify_certificate_chain(der, &B.public_key_bytes()) for a second authority B returns Err(TrustError::CertificateVerification) whose reason equals 'certificate signature did not verify against the issuer public key'; cargo test -p lys-core --lib hex_common_name_certificate_is_judged_on_its_real_issuer prints 'test result: ok. 1 passed; 0 failed'.
- Test self_signed_certificate_is_rejected in crates/lys-core/src/ca/authority_tests.rs, rebuilt: an rcgen certificate self-signed by Ed25519 key K, checked with verify_certificate_chain under K's 32-byte public key, returns Err(TrustError::CertificateVerification) whose reason equals 'self-signed certificate rejected (subject key is the issuer key)'; the same certificate checked under a second key's public key returns Err(TrustError::CertificateVerification) whose reason equals 'certificate signature did not verify against the issuer public key'; cargo test -p lys-core --lib self_signed_certificate_is_rejected prints 'test result: ok. 1 passed; 0 failed'.
- Test ca_own_key_certificate_is_refused_as_self_signed in crates/lys-core/src/ca/authority_tests.rs: authority A issues with issue_certificate_for_request over create_certificate_request(A's own identity, "agent-self") with a 1-hour TTL; the test asserts the parsed certificate's raw subject DN differs from its raw issuer DN; verify_certificate_chain(der, &A.public_key_bytes()) returns Err(TrustError::CertificateVerification) whose reason equals 'self-signed certificate rejected (subject key is the issuer key)'; cargo test -p lys-core --lib ca_own_key_certificate_is_refused_as_self_signed prints 'test result: ok. 1 passed; 0 failed'.
- Test key_self_signed_certificate_with_differing_names_is_refused in crates/lys-core/src/ca/authority_tests.rs: an rcgen issuer certificate with common name issuer-x self-signed by Ed25519 key K, and a leaf with common name leaf-y and subject key K signed by K under that issuer; the test asserts the leaf's raw subject DN differs from its raw issuer DN; verify_certificate_chain(leaf der, K's 32-byte public key) returns Err(TrustError::CertificateVerification) whose reason equals 'self-signed certificate rejected (subject key is the issuer key)'; cargo test -p lys-core --lib key_self_signed_certificate_with_differing_names_is_refused prints 'test result: ok. 1 passed; 0 failed'.
- Test subject_key_as_supplied_key_without_its_signature_fails_the_signature in crates/lys-core/src/ca/authority_tests.rs: authority A issues with issue_certificate_for_request over a request from identity K; verify_certificate_chain(der, &K.public_key_bytes()) returns Err(TrustError::CertificateVerification) whose reason equals 'certificate signature did not verify against the issuer public key'; cargo test -p lys-core --lib subject_key_as_supplied_key_without_its_signature_fails_the_signature prints 'test result: ok. 1 passed; 0 failed'.
- Test self_signed_judgement_compares_decoded_points in crates/lys-core/src/ca/authority_tests.rs, calling the private same-key function of authority.rs: the 32 bytes 0x03 followed by 31 bytes 0x00 and the 32 bytes 0xf0 followed by 30 bytes 0xff and one byte 0x7f (the canonical and a non-canonical encoding of the point with y = 3) are judged the same key; 0x03 followed by 31 bytes 0x00 and 0x04 followed by 31 bytes 0x00 are judged different keys; 0x02 followed by 31 bytes 0x00 (no point has y = 2) compared with itself is judged not the same key; the test asserts 1 same and 2 not-same; cargo test -p lys-core --lib self_signed_judgement_compares_decoded_points prints 'test result: ok. 1 passed; 0 failed'.
- Test unreadable_subject_key_is_not_judged_self_signed in crates/lys-core/src/ca/authority_tests.rs: an rcgen issuer certificate with common name issuer-x self-signed by Ed25519 key K, and a leaf with common name leaf-y whose subject public key is an ECDSA P-256 key (SubjectPublicKeyInfo algorithm 1.2.840.10045.2.1, not Ed25519) signed by K under that issuer, with rcgen's default validity; the test asserts certificate_subject_public_key(leaf der) returns Err(TrustError::CertificateParsing); verify_certificate_chain(leaf der, K's 32-byte public key) returns Ok(()), the result the tree at the base commit gives for this input, and so is not refused with the reason 'self-signed certificate rejected (subject key is the issuer key)'; cargo test -p lys-core --lib unreadable_subject_key_is_not_judged_self_signed prints 'test result: ok. 1 passed; 0 failed'.
- rg -c 'fn (hex_common_name_certificate_is_judged_on_its_real_issuer|self_signed_certificate_is_rejected|ca_own_key_certificate_is_refused_as_self_signed|key_self_signed_certificate_with_differing_names_is_refused|subject_key_as_supplied_key_without_its_signature_fails_the_signature|self_signed_judgement_compares_decoded_points|unreadable_subject_key_is_not_judged_self_signed)\(' crates/lys-core/src/ca/authority_tests.rs prints 7.
- cargo test -p lys-core --all-features --test openssl_csr_interop exits 0, and a_request_is_not_accepted_as_a_certificate in crates/lys-core/src/ca/request_tests.rs passes unchanged.

**Files:**
- modify: crates/lys-core/src/ca/authority.rs
- modify: crates/lys-core/src/ca/authority_tests.rs

**Checklist:**
- C23 — Self-signed certificates are rejected by verify_certificate_chain, where a certificate is self-signed when its subject public key is the supplied issuer key, compared as decoded Ed25519 points, and its signature verifies under that key.
- C66 — verify_certificate_chain_at compares no subject or issuer distinguished-name bytes: crates/lys-core/src/ca/authority.rs calls neither .subject() nor .issuer() on a parsed certificate.
- C67 — A certificate issued by CertificateAuthority::issue_certificate to the subject named by the authority's lowercase-hex public key verifies under the authority's key and is refused under any other key.
- C68 — A certificate issued by issue_certificate_for_request over a request made with the authority's own key is refused by verify_certificate_chain as self-signed.

**Stories:**
- S5 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to verify a counterparty's certificate chain at the MCP boundary — at the dispatch instant, via `verify_certificate_chain_at` — so that cross-agent tool calls are gated on legitimate, unexpired, capability-scoped identity.

### R2: Say in verify_certificate_chain's rustdoc what it checks and what it leaves to the caller

Documentation only, in crates/lys-core/src/ca/authority.rs. The doc comment of the free verify_certificate_chain SHALL carry two sections. '# What this checks' lists, one bullet each: the bytes parse as an X.509 certificate; its signature algorithm is Ed25519; its signature verifies with strict Ed25519 verification under the issuer key the caller supplies; its subject key is not that issuer key (a certificate whose subject key is the issuer key and whose signature verifies under it is self-signed and refused, whatever its names); the current instant lies inside notBefore and notAfter, both inclusive. '# What this leaves to the caller' lists five bullets, in this order, each beginning with the words given here: 'Revocation' (lys-core has no revocation check); 'Trust in the supplied issuer key' (the function believes whatever key it is given); 'Possession of the subject key' (proven only at issuance, by CertificateAuthority::issue_certificate_for_request); 'Anything beyond one level' (it verifies one signature under one key, walks no chain, and checks no issuer certificate, basic constraints or key usage); 'The meaning of any extension or capability claim' (none is read). The doc comment of verify_certificate_chain_at SHALL state the key-based rule and link to verify_certificate_chain for what is left to the caller; the doc comment of CertificateAuthority::verify_certificate_chain SHALL link to the same caveat. THE SYSTEM SHALL NOT keep any sentence describing a distinguished-name comparison, a heuristic screen or its false positive. The documentation SHALL NOT link any item gated by unstable-anchor, and SHALL NOT change any function signature or body.

**Acceptance:**
- rg -n '^/// # What this checks$|^/// # What this leaves to the caller$' crates/lys-core/src/ca/authority.rs prints exactly 2 lines, both inside the doc comment directly above the line beginning 'pub fn verify_certificate_chain(cert_der'.
- The '# What this leaves to the caller' section of verify_certificate_chain's doc comment holds exactly 5 bullets, whose first words are, in order: 'Revocation', 'Trust in the supplied issuer key', 'Possession of the subject key', 'Anything beyond one level', 'The meaning of any extension or capability claim'.
- The doc comment of verify_certificate_chain_at contains the sentence 'A certificate is self-signed when its subject public key is the supplied issuer key and its signature verifies under that key.' and the intra-doc link [`verify_certificate_chain`].
- rg -in 'heuristic|false positive|issuer equal to subject|issuer equals subject' crates/lys-core/src/ca/authority.rs prints nothing.
- cargo doc -p lys-core --no-deps and cargo doc -p lys-core --no-deps --all-features each exit 0 and print no line containing 'warning'.

**Files:**
- modify: crates/lys-core/src/ca/authority.rs

**Checklist:**
- C69 — The rustdoc of verify_certificate_chain states what it checks and what it leaves to the caller.
- C70 — The rustdoc of verify_certificate_chain_at and CertificateAuthority::verify_certificate_chain states the key-based self-signed rule and mentions no distinguished-name heuristic.

**Stories:**
- S5 (Norn Agent Runtime, Signing Session History (primary consumer)) — As the Norn runtime, I want to verify a counterparty's certificate chain at the MCP boundary — at the dispatch instant, via `verify_certificate_chain_at` — so that cross-agent tool calls are gated on legitimate, unexpired, capability-scoped identity.

### R3: Show lys ca verify and lys verify --cert judging both cases on keys

Tests only, in crates/lys/tests/certified_attestation_tests.rs, using its Fixture. WHEN lys ca verify or lys verify --cert is given a certificate the fixture's CA issued over a holder's request to the subject named by the CA's lowercase-hex public key, THE SYSTEM SHALL exit 0. WHEN either is given a certificate the fixture's CA issued over a request made with the CA's own key, THE SYSTEM SHALL exit 1 with the existing single refusal message, and SHALL NOT print any text naming the self-signed check. No file under crates/lys/src changes.

**Acceptance:**
- Test hex_common_name_certificate_verifies_through_both_commands: holder key holder.key is certified by certify_presented with the subject equal to the fixture's ca_public_key (64 lowercase hex characters) into hexcn.pem; lys --json ca verify --cert hexcn.pem --issuer-public-key <ca_public_key> exits 0; after holder.key attests the payload, verify_certified on that attestation and hexcn.pem exits 0 with verified true; cargo test -p lys --test certified_attestation_tests hex_common_name_certificate_verifies_through_both_commands prints 'test result: ok. 1 passed; 0 failed'.
- Test authority_own_key_certificate_is_refused_by_both_commands: certify_presented with holder ca.key and subject agent-self into self.pem; lys ca verify --cert self.pem --issuer-public-key <ca_public_key> exits 1 and its stderr contains 'certificate verification failed' and does not contain 'self-signed'; after ca.key attests the payload, verify_certified on that attestation and self.pem exits 1; cargo test -p lys --test certified_attestation_tests authority_own_key_certificate_is_refused_by_both_commands prints 'test result: ok. 1 passed; 0 failed'.
- cargo test -p lys --bin lys certificate_verification_failed_display_is_single_and_generic prints 'test result: ok. 1 passed; 0 failed', and git diff --stat <base>..HEAD -- crates/lys/src prints nothing.

**Files:**
- modify: crates/lys/tests/certified_attestation_tests.rs

**Checklist:**
- C71 — lys ca verify and lys verify --cert exit 0 for a certificate issued to the subject named by the issuer's lowercase-hex public key, and exit 1 for a certificate self-signed by keys.

**Stories:**
- S11 (Lys CLI Operator and Auditor, Checking certificates and certified statements) — As an auditor, I want `lys ca verify` with an explicit instant so that I can check whether a certificate was valid at the time a disputed action occurred, not just at the time of my audit.

### R4: Keep lys-anchor's admission doc true under the key-based rule

Documentation only, in the module doc of crates/lys-anchor/src/admission/certificate.rs. The sentence naming what verify_certificate_chain rejects SHALL say it rejects a certificate whose subject key is the issuer key, and SHALL NOT describe self-signed by names. No code in lys-anchor changes.

**Acceptance:**
- rg -n 'rejects self-signed certificates' crates/lys-anchor/src/admission/certificate.rs prints nothing.
- rg -c 'whose subject key is the issuer key' crates/lys-anchor/src/admission/certificate.rs prints 1.
- cargo test -p lys-anchor --all-features exits 0.

**Files:**
- modify: crates/lys-anchor/src/admission/certificate.rs

**Checklist:**
- C72 — lys-anchor's admission module doc says verify_certificate_chain rejects certificates whose subject key is the issuer key.

### R5: Record the change in CHANGELOG's Unreleased section

Documentation only. Under ## [Unreleased] in CHANGELOG.md, a subsection headed '### Changed — `lys-core` certificate verification' SHALL hold three bullets: a certificate issued to the subject named by the issuer's lowercase-hex public key now verifies, where it was refused; a certificate whose subject key is the issuer key and whose signature verifies under it is now refused as self-signed, whatever its names, where it verified when its names differed; the rustdoc of verify_certificate_chain now states what it checks and what it leaves to the caller. The entry SHALL NOT name a version number or a release date, and no other section of CHANGELOG.md changes.

**Acceptance:**
- rg -n '^### Changed — `lys-core` certificate verification$' CHANGELOG.md prints exactly 1 line, and that line sits after the line '## [Unreleased]' and before the next line beginning '## ['.
- That subsection holds exactly 3 bullets, and they contain, in order, the phrases 'now verifies', 'now refused as self-signed' and 'what it leaves to the caller'.
- git diff <base>..HEAD -- CHANGELOG.md removes no line and adds lines only inside that subsection.

**Files:**
- modify: CHANGELOG.md

**Checklist:**
- C73 — CHANGELOG.md's Unreleased section records that a hex-common-name certificate now verifies, that a certificate self-signed by keys is now refused, and the new rustdoc caveat on verify_certificate_chain.

### R6: Show DIRECTORY-013's fold issuing a hex-common-name leaf and refusing a leaf self-signed by keys

Tests only, one test added to crates/lys-identity/tests/revocation_fold.rs, which DIRECTORY-013 creates and this row adds to; this row starts only once DIRECTORY-013 has landed and that file exists on origin/main (blocked_by). The fold's public entry point, how an issuance leaf and a revocation leaf are built, and the accessors for the folded size, the issued set, the revoked certificates and the refused leaves with their reason cannot be read from any tree reachable when this brief was written, because DIRECTORY-013 has not landed; this row invents none of them. The test calls them by the names DIRECTORY-013's landed interface gives them on origin/main when the row is started. WHEN the fold reads an issuance leaf whose certificate the issuing authority issued to the subject named by its own lowercase-hex public key, THE SYSTEM SHALL count it issued. WHEN the fold reads an issuance leaf whose certificate's subject key is the issuing authority's key, THE SYSTEM SHALL record certificate_chain_invalid at its index and SHALL NOT count it issued. WHEN the fold reads a revocation leaf for the issued hex-common-name certificate, THE SYSTEM SHALL count that certificate revoked. No file under crates/lys-identity/src changes and no other test of revocation_fold.rs changes.

**Acceptance:**
- Test lyscore001_fold_judges_self_signed_by_keys in crates/lys-identity/tests/revocation_fold.rs, calling DIRECTORY-013's landed fold interface by the names it carries on origin/main: a fixture log of 3 leaves, at index 0 the issuance leaf of certificate H issued by the issuing authority with issue_certificate to the subject hex_lower of the issuing authority's public key, at index 1 the issuance leaf of certificate K issued by the issuing authority with issue_certificate_for_request over a request made with the issuing authority's own identity, at index 2 the revocation leaf of H, folded with the issuing authority's public key, shows exactly 1 issuance (H), exactly 1 revocation (H), and exactly 1 refused leaf, certificate_chain_invalid at index 1; cargo test -p lys-identity --test revocation_fold lyscore001_fold_judges_self_signed_by_keys prints 'test result: ok. 1 passed; 0 failed'.
- git diff --stat <base>..HEAD -- crates/lys-identity/src prints nothing, and git diff <base>..HEAD -- crates/lys-identity/tests/revocation_fold.rs removes no line.

**Files:**
- modify: crates/lys-identity/tests/revocation_fold.rs

**Checklist:**
- C74 — DIRECTORY-013's fold issues a legitimately issued hex-common-name leaf and records certificate_chain_invalid for a leaf self-signed by keys.

### R7: Append this brief's rows to the carried lys-core record and commit the render that keeps the older text

Structural, in docs/design/lys-core, on the files the carry card landed. The carried files are the cluster's record of what main already says: this row appends to them and removes or rewords nothing in them except C23 and the one design sentence named below. In checklist.json: C23 keeps its section and position, its text becomes this brief's C23 text and its done becomes false; C66 to C74 are added after C65, in id order, each under the section name this brief's checklist gives it (Certificate Authority for C66 to C70, CLI Surface for C71, Consumers and Records for C72 to C74), a section the carried file lacks being appended after its last section. In stories.json nothing changes: S5 and S11, the stories this brief serves, are already carried with the same text, and this brief adds no story. In design.json the carried title, intention, problem and solution stay the carried ones, apart from one sentence: the carried sentence 'Self-signed certificates are rejected.' becomes 'A certificate whose subject public key is the supplied issuer key and whose signature verifies under that key is self-signed and is rejected, whatever its names.' This brief's own title, intention, problem and solution stay in its cluster record and are not written into the carried design. This brief's design rows are appended after the carried rows of the same field, in the order this brief's record gives them, and each keeps its text: its 4 principles after the carried principles; its 8 constraints after the carried constraints; its 6 goals after the carried goals; its 5 non-goals after the carried non-goals; its 13 inventory rows after the carried inventory; and, so that the design gate's coverage check finds them, each decision id ADR-003, ADR-044 and ADR-045 and each path R1 to R7 name that the carried decisions and structure lack. Immediately before writing, the highest P number and the highest CN number in the carried design.json are read (0 where it has none), and the appended rows take the next free numbers in order: the record's P1, P2, P3 and P4 become P(p+1) to P(p+4), and its CN1 to CN8 become CN(c+1) to CN(c+8), where p and c are those highest numbers. Every reference to one of these ids in an appended row, and in the review record of this brief, follows the new number, so the non-goal whose reason cites P4 cites P(p+4). DESIGN.md, CHECKLIST.md and USER-STORIES.md are then written by scripts/design/render-cluster.py docs/design/lys-core and committed as rendered. The build SHALL NOT remove, reorder or reword any carried checklist item other than C23, any carried story, or any carried design row or text other than that one sentence, SHALL NOT replace the carried title, intention, problem or solution, SHALL NOT reuse a P or CN number the carried design already holds, and SHALL NOT edit any of the three markdown files by hand.

**Acceptance:**
- python3 -c "import json; ids=[i['id'] for s in json.load(open('docs/design/lys-core/checklist.json'))['sections'] for i in s['items']]; print(ids==['C%d' % n for n in range(1, 75)])" prints True.
- For every checklist id from C1 to C65 other than C23, its text and done in docs/design/lys-core/checklist.json equal those in git show <base>:docs/design/lys-core/checklist.json, and git diff <base>..HEAD -- docs/design/lys-core/stories.json prints nothing.
- Every carried design row is present and unchanged apart from the one sentence: with O the design.json of git show <base>:docs/design/lys-core/design.json and N the committed docs/design/lys-core/design.json, N's title, intention and problem equal O's; N's solution equals O's solution with 'Self-signed certificates are rejected.' replaced by the new sentence; and for each of principles, constraints, goals, non_goals, inventory, structure and decisions, N's list begins with O's list, element for element.
- The appended rows carry the new numbers: with p and c the highest P and CN numbers in O (0 where it has none), N's principles after O's are exactly 4 rows with ids P(p+1) to P(p+4), N's constraints after O's are exactly 8 rows with ids CN(c+1) to CN(c+8), N's goals after O's are exactly 6, N's non_goals after O's are exactly 5 and the last of them cites P(p+4), N's inventory after O's is exactly 13 rows, and python3 scripts/design/check-coverage.py docs/design/lys-core prints 'Coverage clean'.
- rg -c 'C23\*\* — Self-signed certificates are rejected by verify_certificate_chain, where a certificate is self-signed when its subject public key is the supplied issuer key' docs/design/lys-core/CHECKLIST.md prints 1, and rg -c '^- \[ \] \*\*C74\*\*' docs/design/lys-core/CHECKLIST.md prints 1.
- rg -c 'is self-signed and is rejected, whatever its names' docs/design/lys-core/DESIGN.md prints 1, and rg -c 'Self-signed certificates are rejected\.' docs/design/lys-core/DESIGN.md prints 0.
- Rendering a fresh copy of docs/design/lys-core with python3 scripts/design/render-cluster.py leaves DESIGN.md, CHECKLIST.md and USER-STORIES.md byte-identical to the committed files, and sh scripts/design/gate.sh exits 0.

**Files:**
- modify: docs/design/lys-core/checklist.json
- modify: docs/design/lys-core/design.json
- modify: docs/design/lys-core/DESIGN.md
- modify: docs/design/lys-core/CHECKLIST.md
- modify: docs/design/lys-core/USER-STORIES.md

**Checklist:**
- C23 — Self-signed certificates are rejected by verify_certificate_chain, where a certificate is self-signed when its subject public key is the supplied issuer key, compared as decoded Ed25519 points, and its signature verifies under that key.

## Boundaries

- Only the files the requirements name change; a row that needs any other file stops and names it.
- No published wire format or domain tag changes: lys/attestation/v2, lys/sealed-envelope/v1, lys/anchor-receipt/v1, lys/verification-bundle/v1, lys/consistency-receipt/v1 and lys/delegation/v1 stay byte-identical.
- No lys-core version bump, no crates.io publish and no release tag; the workspace version stays 0.2.0.
- Signature verification stays ed25519-dalek verify_strict; x509-parser's verify_signature is never called.
- The signatures of verify_certificate_chain, verify_certificate_chain_at and CertificateAuthority::verify_certificate_chain do not change; neither issuance path and neither DN they write changes.
- lys-core gains no revocation check, and no chain walking.
- The CLI's refusal message and exit codes do not change; no file under crates/lys/src changes.
- DIRECTORY-013's brief and every DIRECTORY-013 file other than one added test in crates/lys-identity/tests/revocation_fold.rs are untouched.
- No intra-doc link from ungated documentation to an item gated by unstable-anchor.
- No unwrap, expect, panic, todo, unimplemented or unreachable in library code; no #[allow], #[ignore], _-prefixed unused binding or #[cfg(any())] bypass.
- docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md change only through R7's render; no carried item other than C23 and the one design sentence R7 names is removed, reordered or reworded, and this brief's design rows are only appended after the carried ones.

## Verification

- Before the first row: git fetch -q origin main && git show origin/main:docs/design/roadmap.json | python3 -c "import json,sys; rows=[i for i in json.load(sys.stdin)['items'] if 'DIRECTORY-013' in i['links']['briefs']]; print(rows[0]['status'] if len(rows)==1 else 'absent')" prints landed, the lys-core carry command in blocked_by prints carried, and git cat-file -e origin/main:crates/lys-identity/tests/revocation_fold.rs exits 0.
- cargo fmt --all leaves git diff empty.
- cargo clippy --all-targets --all-features -- -D warnings exits 0.
- cargo clippy --all-targets -- -D warnings exits 0.
- cargo test --workspace --all-features exits 0, and git diff --stat <base>..HEAD lists only the files R1 to R7 name; of existing test functions only self_signed_certificate_is_rejected changes.
- cargo doc --no-deps --all-features and cargo doc --no-deps each exit 0 with no line containing 'warning'.
- sh scripts/design/gate.sh exits 0 and prints no 'rendered markdown differs' line.
- git diff <base>..HEAD -- Cargo.toml crates/lys-core/Cargo.toml prints nothing.
- The brief's review record carries a constructed-attack review of R1, one entry per attack with the result observed: a subject named by the authority's hex key (verifies under the authority, refused under another key); the authority's own key certified over a request (refused as self-signed); a certificate signed by a key over that same key as subject key under differing names (refused as self-signed); a non-canonical encoding of the issuer's point as subject key (judged the issuer's key); a subject key equal to the supplied key with a signature by another key (refused on the signature); a small-order issuer key (refused, small_order_issuer_key_forgery_is_rejected unchanged); a subject key that is not a readable Ed25519 key under a valid signature (an ECDSA P-256 subject key: verifies, Ok(()), not refused as self-signed).

