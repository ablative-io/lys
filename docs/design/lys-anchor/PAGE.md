# lys-anchor — what was asked, what it means, and what was written

## The words, as they were typed

The test an_expired_certificate_is_refused in crates/lys-anchor/src/admission/certificate_tests.rs issues a certificate valid for one second and then sleeps 2.1 seconds so that the admission policy sees it expired. That wait is a clock. Under load the sleep proves nothing about the code, on a loaded host it is the kind of wait that makes a suite slow, and the suite pays it on every run. Tom's word of 27 September 2026, relayed by Waffles in the pipeline channel: tests must not take this long and things must not fail under load; a test that waits on the clock is a defect in the code, not a flake. The crate lys-core already carries verify_certificate_chain_at, which takes the instant to judge against, and its API is frozen at 0.2.0 and is not changed here.

The rule to meet: an admission policy judges at an instant its caller names. RecognisedCertificate gains admit_at, taking the statement, the asserted credential and the instant, built on verify_certificate_chain_at; admit keeps its signature and calls admit_at with the present, so nothing that admits a live request changes. The test issues the same one-second certificate and calls admit_at twice with no sleep: at an instant inside the validity window it asserts admitted, and at an instant two seconds past issuance it asserts NotAdmitted. The positive control stays, the refusal is still the only check that tells a chain verification that consults the window from one that does not, and the test runs in milliseconds.

Acceptance: the sleep is gone and both assertions stand on explicit instants; a grep for thread::sleep and tokio::time::sleep across crates/lys-anchor prints nothing; every other admission test is unchanged and green; no timeout is raised and no test is split; the change is one commit on the card branch. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). The cluster is lys-anchor, new under docs/design, holding only this card's documents; the brief id is the first for that cluster's prefix, and the roadmap row and any decision take the next free id after main's highest and every open brief/* and draft/* branch's, checked with git ls-remote immediately before writing. Not in scope: how notAfter is truncated at issuance, lys-core's public API, or any other test in the workspace. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie on 27 September 2026 on lys main 7b53625.

## What the survey found, and its angles

Remove the 2.1-second sleep in lys-anchor's an_expired_certificate_is_refused test. Instead, RecognisedCertificate gets an admit_at method that judges a certificate at an instant the caller names, built on lys-core's existing verify_certificate_chain_at. admit keeps its signature and calls admit_at with the current time. The test then asserts admitted at an instant inside the one-second window and NotAdmitted two seconds after issuance, with no wait. The documents are to be written as a new method cluster, lys-anchor, under docs/design. The roadmap row and any decision take the next free ids.

### What the tree holds

- `crates/lys-anchor/src/admission/certificate_tests.rs` — Line 202 holds the only std::thread::sleep in crates/lys-anchor: Duration::from_millis(2100) inside an_expired_certificate_is_refused (lines 180-208). The module doc's 'The passage of time' bullet (lines 16-19) says one case waits for a certificate to expire, so that prose goes out of date when the wait is gone.
- `crates/lys-anchor/src/admission/certificate.rs` — Where admit_at lands. Today the AdmissionPolicy::admit impl matches the three SubmitterContext arms (Unidentified refused, AssertedBySubmitter, AuthenticatedByTransport) and calls verify_certificate_chain(credential, &issuer), which uses Utc::now(). The module doc (lines 50-54) and the struct docs say 'inside its validity window right now' and need to name the instant instead. The file is 239 lines, well under the 500-line limit.
- `crates/lys-anchor/src/admission/policy.rs` — The AdmissionPolicy trait contract: admit(&self, &Submission, &SubmitterContext) -> Result<(), NotAdmitted>. It says the context 'arrives with its provenance attached' and that no accessor collapses the two arms. admit_at is an inherent method on RecognisedCertificate; the trait itself does not change.
- `crates/lys-anchor/src/admission/context.rs` — Defines SubmitterContext's three arms. The words say admit_at takes 'the asserted credential'; the trait takes a context. This is where that shape is decided.
- `crates/lys-anchor/src/anchor/append.rs:107` — The only non-test caller of .admit(&submission, &context) in lys-anchor and lys-anchor-cli. It must keep calling admit unchanged, so a live request is still judged at the present.
- `crates/lys-core/src/ca/authority.rs:358` — verify_certificate_chain_at(cert_der, issuer_public_key, at: DateTime<Utc>) already exists, validity boundaries inclusive. verify_certificate_chain (line 312) is a thin wrapper that passes Utc::now(). Issuance sets issued_at = Utc::now() (line 255), and to_offset_date_time (line 445) truncates to whole seconds. This is the frozen 0.2.0 API the words say is not changed.
- `crates/lys-core/src/ca/certificate.rs:49` — IssuedCertificate exposes expires_at but no issued_at. The test's 'instant two seconds past issuance' has to come from a timestamp captured around the issue call or be derived from expires_at.
- `crates/lys-anchor/Cargo.toml` — lys-anchor has no chrono dependency; chrono is only a workspace dependency (Cargo.toml:32) used by lys-core. An admit_at taking DateTime<Utc> means adding chrono.workspace = true here. The crate is publish = false.
- `docs/design/lys-anchor/` — Already exists on main with 8 legacy files (BUILD-PLAN, DECISIONS, DELEGATION-V1, HANDOFF-2026-08-08, KEY-HISTORY-FOLD-QUESTIONS, STRAWMAN-SESSION, STRAWMAN, WIRE-DRAFTS; 5171 lines) and no design.json. The words call the cluster 'new' and say it holds only this card's documents. DECISIONS.md holds the DP ledger (DP9, DP23).
- `scripts/design/gate.sh` — The design leg. It treats a directory with a design.json as a cluster, runs validate.py, check-coverage.py and render-cluster.py on it, and cmp-checks every rendered *.md against the committed copy. Adding design.json to docs/design/lys-anchor puts that directory under the gate for the first time.
- `scripts/design/schemas/brief.schema.json` — The brief id pattern is ^[A-Z]+-\d{3}$. A hyphenated prefix such as LYS-ANCHOR-001 is refused, so the prefix must be one uppercase word (for example ANCHOR).
- `docs/design/roadmap.json` — The roadmap is the v2 cluster registry (validate.py v2_clusters). The new row links cluster lys-anchor, which is what makes the gate validate it.

### What was already decided

- DP9 (docs/design/lys-anchor/DECISIONS.md:179) — The write path is certificate-gated, and the gate must not be built on a request's signature. RecognisedCertificate is that gate, and admit_at keeps it.
- DP23 (docs/design/lys-anchor/DECISIONS.md:359) — Admission is a policy object and no default ships. admit_at must not add a Default or a defaulting constructor, and the trait stays the policy boundary.
- policy.rs AdmissionPolicy contract — A policy sees the submission and the submitter context with its provenance attached, and nothing else. Every refusal is one fieldless NotAdmitted.
- certificate.rs module docs — The policy asks exactly two questions lys-core already answers: chain plus validity window, then an optional subject allow-list read only after the chain verifies. It reads no extension.
- CLAUDE.md 'A test needs a second party' — Count what fired, and keep the positive control. The expired case is named as the only check that tells a chain verification that consults the window from one that does not.
- CLAUDE.md coding standards — No unwrap in library code, every public item documented, no file over 500 lines, fmt, clippy in both feature shapes, tests with --all-features, and cargo doc in both shapes.
- lys-core 0.2.0 published API — verify_certificate_chain_at is exported from lys_core::ca (ca/mod.rs:11) and is frozen at 0.2.0, which the words say is not changed.

### What was measured

- thread::sleep / tokio::time::sleep call sites under crates/lys-anchor: 1 (certificate_tests.rs:202, 2100 ms)
- sleep call sites elsewhere in the workspace (out of scope): 2 (crates/lys-home/src/record/templates_tests.rs:31 and blocks_tests.rs:22, 20 ms each)
- #[test] count in lys-anchor admission test files: certificate_tests.rs 10, context_tests.rs 4, trivial_tests.rs 5 (19 total)
- certificate.rs / certificate_tests.rs line counts: 239 / 387 lines
- non-test callers of .admit( in lys-anchor and lys-anchor-cli: 1 (anchor/append.rs:107)
- existing callers of verify_certificate_chain_at outside lys-core: crates/lys commands/ca.rs and verify.rs, plus lys ca_tests; none in lys-anchor
- files already in docs/design/lys-anchor: 8 markdown files, 5171 lines, no design.json and no briefs/
- highest roadmap id across main and all 67 origin brief/*, draft/* and main refs (ls-remote on 2026-09-27): RM-018, so the next free id is RM-019 (main alone tops at RM-016)
- highest decision id across the same refs: ADR-029, so the next free id is ADR-030 (main alone tops at ADR-018)
- origin branches whose name contains 'anchor': 0
- chrono in lys-anchor's Cargo.toml: absent (workspace dependency only, Cargo.toml:32)
- workspace version: 0.2.0; only tag present is lys-core-0.1.0

### What it means for the other projects

- cambium — The card goes through the board's chain (brief_card, sign-off, card_build_v3, src_pr, src_land). Nothing is hand-built.
- method — The documents must pass the method's scripts vendored in scripts/design (validate.py, check-coverage.py, render-cluster.py through gate.sh). The method itself does not change.
- aion — The workflow chain runs the build and landing. Heavy gates go to Dean's laptop; only warm single-crate checks run on this Mac.

### The decisions it stands on

- ADR-004 (honour) — lys-anchor stays standalone. The change adds no dependency on another estate project; only chrono, already in the workspace, is added.
-  (new) — If the author records it, an admission policy judges at an instant its caller names, and the live path passes the present (ADR-030, the next free id as of this survey's ls-remote). DP9 and DP23 in docs/design/lys-anchor/DECISIONS.md are honoured: the certificate gate and the no-default policy object are unchanged.

### What it requires

- RecognisedCertificate has a public, documented admit_at that judges the certificate at a caller-supplied instant via lys_core::ca::verify_certificate_chain_at.
- RecognisedCertificate's AdmissionPolicy::admit keeps its exact signature and calls admit_at with the present instant.
- an_expired_certificate_is_refused contains no sleep and asserts Ok(()) from admit_at at an instant inside the window and Err(NotAdmitted) at issuance plus 2 s.
- grep -rnE 'thread::sleep|tokio::time::sleep' crates/lys-anchor prints nothing.
- The other 9 tests in certificate_tests.rs and the 9 in context_tests.rs and trivial_tests.rs are byte-unchanged and pass.
- crates/lys-core is byte-unchanged.
- fmt, clippy (both feature shapes, -D warnings), cargo test --workspace --all-features, cargo doc (both shapes) and sh scripts/design/gate.sh all pass.
- The code change is one commit on the card branch.
- docs/design/lys-anchor gains a method cluster (design.json, checklist.json, stories.json, rendered markdown, briefs/<PREFIX>-001) that the gate validates, and roadmap.json gains the next free RM id linked to cluster lys-anchor.
- The test's module doc and certificate.rs docs no longer say the check waits on, or is judged 'right now' by, the clock alone.

### What must not change

- lys-core's public API and source do not change.
- The AdmissionPolicy trait signature does not change, and no Default or defaulting constructor is added (DP23).
- NotAdmitted stays fieldless, and every refusal stays the same value.
- The live admission path in anchor/append.rs is untouched.
- No other test in the workspace changes, including the lys-home sleeps.
- No timeout is raised and no test is split.
- notAfter truncation at issuance is not touched.
- The existing legacy documents in docs/design/lys-anchor are not rewritten unless the lead rules otherwise.

### What we must put in place first

- The lead's ruling on whether docs/design/lys-anchor, which already exists, may hold the new cluster beside its legacy documents.
- The lead's ruling on admit_at's credential parameter shape: SubmitterContext or bytes.
- A fresh git ls-remote immediately before writing ids. As of this survey the next free ids are RM-019 and ADR-030.

### The risks

- Adding design.json to docs/design/lys-anchor puts the directory under gate.sh for the first time. check-coverage or validate may object to the directory's shape, and a render that emits DESIGN.md, CHECKLIST.md or USER-STORIES.md must not collide with the legacy files (today it does not).
- An admit_at taking bare bytes would give a public method that collapses provenance, against the context.rs and policy.rs contracts.
- Choosing an 'inside' instant carelessly (for example issuance + 1 s against a truncated notAfter) could make the positive control depend on sub-second timing. Issuance + 0 s, or a timestamp captured before issue, keeps it inside the inclusive window.
- If admit_at took a clock rather than an instant, the fix would spread; the words ask for an instant.
- Branch ids can move between this survey and the write. RM-018 and ADR-029 are already taken on open branches, so reusing main's numbering would collide.
- Adding chrono to lys-anchor's dependencies changes the dependency list of a publish = false crate. Harmless, but reviewers should see it named.

### Still open

- The cluster directory docs/design/lys-anchor already exists on main with 8 legacy documents (5171 lines). Should this card add design.json and briefs/ beside them, move the legacy files elsewhere, or use a different cluster name so the cluster holds only this card's documents? The sentence of the words it stands on: "The cluster is lys-anchor, new under docs/design, holding only this card's documents; the brief id is the first for that cluster's prefix, and the roadmap row and any decision take the next free id after main's highest and every open brief/* and draft/* branch's, checked with git ls-remote immediately before writing.". Why only the lead can settle it: docs/design/lys-anchor/ on main 7b53625 already holds BUILD-PLAN.md, DECISIONS.md, DELEGATION-V1.md, HANDOFF-2026-08-08.md, KEY-HISTORY-FOLD-QUESTIONS.md, STRAWMAN-SESSION.md, STRAWMAN.md and WIRE-DRAFTS.md, so the directory is neither new nor able to hold only this card's documents without moving them. Where those documents live afterwards changes what a reader finds there.
- Should admit_at take the SubmitterContext, as admit does (keeping the Unidentified, asserted and authenticated arms), or only credential bytes, as 'the asserted credential' reads? The sentence of the words it stands on: "RecognisedCertificate gains admit_at, taking the statement, the asserted credential and the instant, built on verify_certificate_chain_at; admit keeps its signature and calls admit_at with the present, so nothing that admits a live request changes.". Why only the lead can settle it: admit takes &SubmitterContext, which has three arms (context.rs), and policy.rs's contract says provenance arrives attached, with no accessor collapsing the two. An admit_at taking only asserted bytes cannot receive what admit receives. It would have to collapse provenance before the call, which the tree's design refuses, or it could not represent Unidentified or AuthenticatedByTransport. An operator calling the public admit_at sees a different API depending on the answer.

### The units beyond the first

- Remove the 20 ms sleeps from lys-home's templates and blocks record tests — The same defect class (a test waiting on the clock) in crates/lys-home/src/record/templates_tests.rs:31 and blocks_tests.rs:22. The words put every other test in the workspace out of scope, so it needs its own card.

### The smallest complete shape

One commit on the card branch: RecognisedCertificate::admit_at, documented, taking the submission, the credential (in the shape the lead rules) and a DateTime<Utc>, and calling verify_certificate_chain_at. admit is rewritten to call admit_at(…, Utc::now()), with chrono added to lys-anchor's dependencies. an_expired_certificate_is_refused is rewritten to assert admitted inside the window and NotAdmitted at issuance + 2 s with no sleep, and the two doc passages that describe waiting are updated. Beside it on the brief branch: the lys-anchor method cluster (design.json, checklist, stories, rendered markdown, brief <PREFIX>-001) and roadmap row RM-019, all passing gate.sh.

## The roadmap row

- **RM-026** — Judge certificate admission at an instant its caller names, and take the sleep out of its expiry test (fix, idea)
- Summary: RecognisedCertificate gains admit_at, judging a submission at an instant its caller names on lys-core's existing verify_certificate_chain_at; admit keeps its signature and calls it with the present, so a live request is judged as before. an_expired_certificate_is_refused stops sleeping 2.1 seconds and asserts admitted inside the one-second window and NotAdmitted two seconds past issuance at explicit instants, so the suite no longer pays a wait on every run or depends on a host's load. lys-core is not changed.
- Asked by: tom on 2026-09-27T10:36:00+10:00
- Context: The card filed on the lys board against main 7b53625, relaying the rule given in the pipeline channel that a test waiting on the clock is a defect in the code. Its survey asked the lys lead two questions, answered in round 1: the method documents are added beside the eight pre-method documents already in docs/design/lys-anchor, which stay unrenamed and unchanged; and admit_at takes the statement, the SubmitterContext and the instant, exactly what admit takes plus the instant.
- Quote: The test an_expired_certificate_is_refused in crates/lys-anchor/src/admission/certificate_tests.rs issues a certificate valid for one second and then sleeps 2.1 seconds so that the admission policy sees it expired. That wait is a clock. Under load the sleep proves nothing about the code, on a loaded host it is the kind of wait that makes a suite slow, and the suite pays it on every run. Tom's word of 27 September 2026, relayed by Waffles in the pipeline channel: tests must not take this long and things must not fail under load; a test that waits on the clock is a defect in the code, not a flake. The crate lys-core already carries verify_certificate_chain_at, which takes the instant to judge against, and its API is frozen at 0.2.0 and is not changed here.

The rule to meet: an admission policy judges at an instant its caller names. RecognisedCertificate gains admit_at, taking the statement, the asserted credential and the instant, built on verify_certificate_chain_at; admit keeps its signature and calls admit_at with the present, so nothing that admits a live request changes. The test issues the same one-second certificate and calls admit_at twice with no sleep: at an instant inside the validity window it asserts admitted, and at an instant two seconds past issuance it asserts NotAdmitted. The positive control stays, the refusal is still the only check that tells a chain verification that consults the window from one that does not, and the test runs in milliseconds.

Acceptance: the sleep is gone and both assertions stand on explicit instants; a grep for thread::sleep and tokio::time::sleep across crates/lys-anchor prints nothing; every other admission test is unchanged and green; no timeout is raised and no test is split; the change is one commit on the card branch. Keep to the method (scripts/design/validate.py, check-coverage.py, render-cluster.py, run by scripts/design/gate.sh). The cluster is lys-anchor, new under docs/design, holding only this card's documents; the brief id is the first for that cluster's prefix, and the roadmap row and any decision take the next free id after main's highest and every open brief/* and draft/* branch's, checked with git ls-remote immediately before writing. Not in scope: how notAfter is truncated at issuance, lys-core's public API, or any other test in the workspace. If a sentence here is open or contradicted by the repository as it stands, the survey quotes it whole as a question for the lead. Filed by Archie on 27 September 2026 on lys main 7b53625.
- Cluster: lys-anchor; briefs: LYSANCHOR-001
- Notes: Further units, not written: Remove the 20 ms sleeps from lys-home's templates and blocks record tests. Ids: RM-026 and ADR-040 are the next free after main's highest (RM-016, ADR-018 at 7b53625) and every open brief/* and draft/* branch's (RM-025, ADR-039), checked with git ls-remote over origin's 69 main, brief/* and draft/* refs immediately before writing; no ref held a LYSANCHOR brief. The render writes DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSANCHOR-001.md, none of which a pre-method document holds, so nothing is renamed.

## The design

---
type: design
cluster: lys-anchor
title: lys-anchor: admission judged at an instant its caller names
---

# lys-anchor: admission judged at an instant its caller names

> **Cluster:** lys-anchor

## Intention

The anchor's certificate gate is judged at an instant its caller names. A live request is still judged at the present, exactly as it is today, while a test states the instants it judges at instead of waiting for the clock to reach them, so the expiry refusal is proved in milliseconds and cannot fail because a host is loaded.

The gate keeps every property it has: the same one-value refusal, the same two questions put to lys-core, provenance arriving attached, and a positive control beside the refusal so that the refusal is still the check that tells a chain verification that consults the validity window from one that does not.

## Problem

an_expired_certificate_is_refused in crates/lys-anchor/src/admission/certificate_tests.rs issues a certificate valid for one second, asserts it admitted, then sleeps 2100 milliseconds before asserting it refused. It is the only sleep in crates/lys-anchor. The suite pays the wait on every run, and on a loaded host a wait on the clock is what makes a suite slow and its outcome depend on scheduling rather than on the code. The test has to wait because RecognisedCertificate can only judge at the present: its AdmissionPolicy::admit calls lys-core's verify_certificate_chain, which reads the clock itself, although lys-core already exports verify_certificate_chain_at, which takes the instant to judge against.

## Solution

RecognisedCertificate gains a public, documented inherent method, admit_at, that takes what admit takes, the submission and the SubmitterContext, plus the instant to judge at, as a chrono UTC date-time, the type verify_certificate_chain_at takes. Its body is the body admit has today with verify_certificate_chain replaced by verify_certificate_chain_at at that instant: the three context arms matched with no wildcard (Unidentified refused, the asserted bytes and the transport-authenticated peer's certificate each read as the credential), the chain verified first, and the optional subject allow-list read only after the chain verifies. Every refusal is the same fieldless NotAdmitted. AdmissionPolicy::admit keeps its signature and calls admit_at with the present, so anchor/append.rs, the only non-test caller of admit, and every live request are unchanged. The AdmissionPolicy trait does not change (DP23, policy.rs), and admit_at is inherent to RecognisedCertificate rather than a trait method (ADR-040). lys-anchor gains chrono through the workspace's existing entry (ADR-004: nothing outside the workspace is added).

The words say admit_at takes the asserted credential; the lead ruled that this is read as the SubmitterContext that carries it, so the Unidentified, asserted and authenticated arms are kept and nothing collapses provenance before the call, as policy.rs's contract says.

The expiry test issues the same one-second certificate from the same authority and takes its issuance instant from the certificate itself, as its expires_at less the one-second lifetime: lys-core truncates both validity bounds to whole seconds, so that instant is exactly the certificate's notBefore and the window runs one second from it, bounds inclusive. It asserts admitted at half a second past issuance, strictly inside the window, and NotAdmitted at two seconds past issuance, strictly after it. Neither instant is read from the clock, so the outcome does not depend on how long issuance took. The module docs of certificate.rs and certificate_tests.rs are rewritten where they say the certificate is judged right now or that a case waits for expiry.

This cluster's documents sit beside the eight documents docs/design/lys-anchor already held, which are its pre-method record and are read, not changed (CN6): BUILD-PLAN.md, DECISIONS.md, DELEGATION-V1.md, HANDOFF-2026-08-08.md, KEY-HISTORY-FOLD-QUESTIONS.md, STRAWMAN-SESSION.md, STRAWMAN.md and WIRE-DRAFTS.md. DECISIONS.md is cited by path from elsewhere in the tree, which is why none is renamed. The method's render writes DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSANCHOR-001.md, and none of the eight holds any of those names.

## Principles

- **P1** — An admission policy judges at an instant its caller names; the live path names the present.
- **P2** — A test states the instants it judges at and never waits on the clock.
- **P3** — Provenance arrives attached: admit_at takes the SubmitterContext admit takes, and nothing collapses its arms before the call.
- **P4** — The positive control stands beside the refusal, and the refusal remains the only check that tells a chain verification that consults the validity window from one that does not.

## Decisions

- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-040 — An admission policy judges at an instant its caller names; the live path names the present — RecognisedCertificate gains an inherent admit_at taking the submission, the SubmitterContext and the instant, built on verify_certificate_chain_at; AdmissionPolicy::admit keeps its signature and calls admit_at with the present, so a live request is judged exactly as before, and tests state the instants they judge at. Rejected: keeping the sleep; raising a timeout or splitting the test; injecting a clock into the policy instead of passing an instant; changing the AdmissionPolicy trait; and an admit_at taking only the asserted credential bytes, which would have to collapse the context's provenance before the call and could not represent the unidentified or transport-authenticated arms.

## Goals

- an_expired_certificate_is_refused contains no sleep and asserts Ok(()) from admit_at at issuance plus 500 milliseconds and Err(NotAdmitted) at issuance plus 2 seconds.
- grep -rnE 'thread::sleep|tokio::time::sleep' crates/lys-anchor prints nothing.
- The 18 other admission tests (9 in certificate_tests.rs, 4 in context_tests.rs, 5 in trivial_tests.rs) are byte-unchanged and pass.
- crates/lys-core, crates/lys-anchor/src/admission/policy.rs, crates/lys-anchor/src/admission/context.rs and crates/lys-anchor/src/anchor/append.rs are byte-unchanged.
- Every leg of the gate passes, and the code change is one commit on the card branch.

## Non-Goals

- How notAfter is truncated at issuance. — Out of scope by the words; the test reads the truncated bound, it does not change it.
- Any change to lys-core, its public API included. — Out of scope by the words; verify_certificate_chain_at is frozen at 0.2.0 and used as published.
- The 20 ms sleeps in crates/lys-home/src/record/templates_tests.rs and blocks_tests.rs. — The words put every other test in the workspace out of scope; they are a further unit on the roadmap row.
- A clock injected into the policy in place of an instant. — The words ask for an instant the caller names.
- A change to the AdmissionPolicy trait. — admit keeps its signature (DP23 keeps the trait as the policy boundary); admit_at is inherent to RecognisedCertificate.
- Raising a timeout or splitting a test. — Excluded by the words.
- Renaming, moving or rewriting the eight pre-method documents in docs/design/lys-anchor. — The lead ruled they stay where they are, unchanged; the render writes no name any of them holds.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `crates/lys-anchor/Cargo.toml` | lys-anchor's manifest; gains chrono from the workspace in [dependencies] (LYSANCHOR-001 R1) |  |
| `Cargo.lock` | the workspace lockfile; lys-anchor's dependency list gains chrono (LYSANCHOR-001 R1) |  |
| `crates/lys-anchor/src/admission/certificate.rs` | RecognisedCertificate; gains admit_at, admit calls it with the present, docs name the instant (LYSANCHOR-001 R2) |  |
| `crates/lys-anchor/src/admission/certificate_tests.rs` | RecognisedCertificate's tests; an_expired_certificate_is_refused judges at two named instants with no sleep, module doc rewritten (LYSANCHOR-001 R3) |  |
| `docs/design/lys-anchor/design.json` | this design | LYSANCHOR-001 |
| `docs/design/lys-anchor/DESIGN.md` | rendered design | LYSANCHOR-001 |
| `docs/design/lys-anchor/checklist.json` | the checklist | LYSANCHOR-001 |
| `docs/design/lys-anchor/CHECKLIST.md` | rendered checklist | LYSANCHOR-001 |
| `docs/design/lys-anchor/stories.json` | the user stories | LYSANCHOR-001 |
| `docs/design/lys-anchor/USER-STORIES.md` | rendered stories | LYSANCHOR-001 |
| `docs/design/lys-anchor/briefs/LYSANCHOR-001.json` | the brief | LYSANCHOR-001 |
| `docs/design/lys-anchor/briefs/LYSANCHOR-001.md` | the rendered brief | LYSANCHOR-001 |

## Inventory

- `crates/lys-anchor/src/admission/certificate.rs` — 239 lines. RecognisedCertificate { issuer_public_key, subject_keys }, constructors issued_by and issued_by_to, no Default (a compile_fail doctest checks it). Its AdmissionPolicy::admit matches Unidentified (refused), AssertedBySubmitter and AuthenticatedByTransport with no wildcard, calls verify_certificate_chain(credential, &issuer) at the present, then reads the subject key only if an allow-list is set. Module doc question 1 says 'inside its validity window right now'; issued_by and admit docs say 'currently'.
- `crates/lys-anchor/src/admission/certificate_tests.rs` — 387 lines, 10 tests. an_expired_certificate_is_refused (lines 182-208) issues from seed b"lys-anchor-admission-ca-seed-05!" with a one-second TTL, asserts admit Ok(()), sleeps 2100 ms (line 202, the only sleep in crates/lys-anchor), asserts Err(NotAdmitted). The module doc bullet 'The passage of time' (lines 16-19) says one case waits for a certificate to expire.
- `crates/lys-anchor/src/admission/policy.rs` — The AdmissionPolicy trait: admit(&self, &Submission, &SubmitterContext) -> Result<(), NotAdmitted>; the context arrives with its provenance attached; NotAdmitted is fieldless. Unchanged.
- `crates/lys-anchor/src/admission/context.rs` — SubmitterContext: Unidentified, AssertedBySubmitter(&[u8]), AuthenticatedByTransport(AuthenticatedPeer). Unchanged.
- `crates/lys-anchor/src/admission/context_tests.rs` — 4 tests. Unchanged.
- `crates/lys-anchor/src/admission/trivial_tests.rs` — 5 tests. Unchanged.
- `crates/lys-anchor/src/anchor/append.rs` — Line 107 is the only non-test caller of admit in lys-anchor and lys-anchor-cli. Unchanged.
- `crates/lys-anchor/Cargo.toml` — publish = false; [dependencies] lys-core, lys-log-store, thiserror from the workspace; no chrono.
- `Cargo.toml` — The workspace; chrono 0.4 with serde is a workspace dependency (line 32). Unchanged.
- `crates/lys-core/src/ca/authority.rs` — verify_certificate_chain (line 312) wraps verify_certificate_chain_at (line 358) with Utc::now(); the window check is inclusive at both bounds. Issuance truncates notBefore and notAfter to whole seconds. Frozen at 0.2.0; unchanged.
- `crates/lys-core/src/ca/certificate.rs` — IssuedCertificate exposes expires_at (whole seconds, equal to notAfter) and no issuance instant. Unchanged.
- `docs/design/lys-anchor/BUILD-PLAN.md` — Pre-method record read by this cluster, unrenamed and unchanged: the anchor's build plan, written before the method.
- `docs/design/lys-anchor/DECISIONS.md` — Pre-method record read by this cluster, unrenamed and unchanged: the DP decision ledger: DP9 (the write path is certificate-gated, never on a request's signature) at line 179 and DP23 (admission is a policy object and no default ships) at line 359; DP26 is cited by path from the revocation brief.
- `docs/design/lys-anchor/DELEGATION-V1.md` — Pre-method record read by this cluster, unrenamed and unchanged: the lys/delegation/v1 draft.
- `docs/design/lys-anchor/HANDOFF-2026-08-08.md` — Pre-method record read by this cluster, unrenamed and unchanged: the anchor's handoff record.
- `docs/design/lys-anchor/KEY-HISTORY-FOLD-QUESTIONS.md` — Pre-method record read by this cluster, unrenamed and unchanged: the open questions on the key-history fold.
- `docs/design/lys-anchor/STRAWMAN-SESSION.md` — Pre-method record read by this cluster, unrenamed and unchanged: the strawman session record.
- `docs/design/lys-anchor/STRAWMAN.md` — Pre-method record read by this cluster, unrenamed and unchanged: the anchor strawman.
- `docs/design/lys-anchor/WIRE-DRAFTS.md` — Pre-method record read by this cluster, unrenamed and unchanged: the anchor's wire-format drafts.

## Constraints

- **CN1** — crates/lys-core is byte-unchanged, its public API included.
- **CN2** — The AdmissionPolicy trait is unchanged; no Default and no defaulting constructor is added to any policy (DP23); NotAdmitted stays fieldless and every refusal is that one value.
- **CN3** — crates/lys-anchor/src/anchor/append.rs is unchanged and admit keeps its signature, so a live request is judged at the present.
- **CN4** — No test other than an_expired_certificate_is_refused changes anywhere in the workspace; no timeout is raised; no test is split.
- **CN5** — No file over 500 lines of code; no unwrap, expect, panic, todo, unimplemented or unreachable in library code; every public item documented; no #[allow], #[ignore], _-prefixed unused binding added, or #[cfg(any())].
- **CN6** — The eight pre-method documents in docs/design/lys-anchor are not renamed, moved or changed.
- **CN7** — The code change is one commit on the card branch.
- **CN8** — Every path in this cluster's documents is relative to the repository root.


---
type: brief
id: LYSANCHOR-001
cluster: lys-anchor
title: Judge RecognisedCertificate at an instant its caller names, and take the sleep out of its expiry test
---

# LYSANCHOR-001: Judge RecognisedCertificate at an instant its caller names, and take the sleep out of its expiry test

> **Cluster:** lys-anchor
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-040 — An admission policy judges at an instant its caller names; the live path names the present — RecognisedCertificate gains an inherent admit_at taking the submission, the SubmitterContext and the instant, built on verify_certificate_chain_at; AdmissionPolicy::admit keeps its signature and calls admit_at with the present, so a live request is judged exactly as before, and tests state the instants they judge at. Rejected: keeping the sleep; raising a timeout or splitting the test; injecting a clock into the policy instead of passing an instant; changing the AdmissionPolicy trait; and an admit_at taking only the asserted credential bytes, which would have to collapse the context's provenance before the call and could not represent the unidentified or transport-authenticated arms.
> **Checklist:**
> - C1 — crates/lys-anchor/Cargo.toml lists chrono.workspace = true under [dependencies], and no other dependency is added.
> - C2 — RecognisedCertificate has a public, documented admit_at taking the submission, the SubmitterContext and a DateTime<Utc>, which verifies the chain and validity window at that instant through lys_core::ca::verify_certificate_chain_at.
> - C3 — RecognisedCertificate's AdmissionPolicy::admit keeps its signature and calls admit_at with Utc::now().
> - C4 — certificate.rs's docs say a certificate is judged at the instant its caller names, and none says 'right now' or 'currently'.
> - C5 — an_expired_certificate_is_refused contains no sleep and asserts Ok(()) from admit_at at an instant inside the one-second window and Err(NotAdmitted) at issuance plus 2 seconds.
> - C6 — grep -rnE 'thread::sleep|tokio::time::sleep' crates/lys-anchor prints nothing.
> - C7 — certificate_tests.rs's module doc describes the expiry case as judged at named instants, not as waiting for expiry.
> - C8 — The other 18 admission tests are byte-unchanged and pass.
> **Stories:**
> - S1 (Operator, Runs the workspace's gate on a shared, loaded build host) — As an operator running the gate on a loaded host, I want the expired-certificate test to finish without waiting on the clock, so that the suite stays fast and its outcome does not depend on load.
> - S2 (Reviewer, Checks that the certificate gate's tests can tell a correct gate from a broken one) — As a reviewer, I want the expiry refusal and its positive control judged at explicit instants, so that the refusal still proves the chain verification consults the validity window.
> - S3 (Developer, Embeds RecognisedCertificate in code that must judge admission at a known instant) — As a developer, I want to ask RecognisedCertificate whether a submission is admitted at an instant I name, given the same submitter context admit takes, so that I can judge admission without reading the clock.
> - S4 (Operator, Serves a live anchor) — As an operator of a live anchor, I want each live submission still judged at the present, so that adding admit_at changes nothing a live request sees.

## Purpose

Delivers RecognisedCertificate::admit_at, which judges a submission at an instant its caller names on lys-core's existing verify_certificate_chain_at, routes admit through it with the present so every live request is judged as before, and rewrites an_expired_certificate_is_refused to judge at two explicit instants instead of sleeping 2100 milliseconds. The suite stops paying a wait on every run and stops depending on the host's load, and the refusal stays the check that tells a window-consulting chain verification from one that does not (ADR-040).

## Task

Do R1 to R3 in order, as one commit on the card branch. In scope: chrono added to lys-anchor from the workspace; admit_at on RecognisedCertificate; admit calling it with Utc::now(); the docs in certificate.rs that say 'right now' or 'currently'; an_expired_certificate_is_refused and the module-doc bullet in certificate_tests.rs that describes it. Out of scope: lys-core (source and public API), the AdmissionPolicy trait, NotAdmitted, anchor/append.rs, how notAfter is truncated at issuance, every other test in the workspace (the two 20 ms sleeps in crates/lys-home included), and the eight pre-method documents in docs/design/lys-anchor. Two sentences of the card's words are corrected by the lead's answers and are read here as corrected. First, where the words say admit_at takes 'the asserted credential', admit_at takes the statement, the SubmitterContext and the instant, exactly what admit takes plus the instant: the asserted credential is read as the context that carries it, so the Unidentified, asserted and authenticated arms are kept and provenance arrives attached. Second, where the words call the lys-anchor cluster new and holding only this card's documents, the directory is not new: it holds eight pre-method documents, unrenamed and unchanged, and this cluster's documents are added beside them. The test's issuance instant is the certificate's expires_at less its one-second lifetime, which is exactly its notBefore because issuance truncates both bounds to whole seconds; its inside instant is issuance plus 500 milliseconds and its refusal instant is issuance plus 2 seconds, so neither depends on how long issuance took. Heavy builds and every gate leg run on the build machine.

## Requirements

### R1: Add chrono to lys-anchor's dependencies from the workspace

Structure: crates/lys-anchor/Cargo.toml's [dependencies] table gains chrono.workspace = true, the workspace's existing chrono entry, and Cargo.lock's lys-anchor package gains chrono in its dependency list. It SHALL NOT add, remove or change any other dependency, dev-dependency, feature or version, and SHALL NOT change the workspace's chrono entry.

**Acceptance:**
- grep -c '^chrono.workspace = true$' crates/lys-anchor/Cargo.toml prints 1, and the matching line lies between the [dependencies] header and the [dev-dependencies] header.
- In Cargo.lock, the dependencies list of the package named lys-anchor is exactly: base64 0.22.1, chrono, lys-core, lys-log-store, serde_json, sha2, tempfile, thiserror 2.0.19.
- git diff --numstat <base> -- crates/lys-anchor/Cargo.toml prints 1 added line and 0 removed lines, where <base> is the commit the build started from.
- git diff <base> -- Cargo.toml prints nothing.

**Files:**
- modify: crates/lys-anchor/Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C1 — crates/lys-anchor/Cargo.toml lists chrono.workspace = true under [dependencies], and no other dependency is added.

**Stories:**
- S3 (Developer, Embeds RecognisedCertificate in code that must judge admission at a known instant) — As a developer, I want to ask RecognisedCertificate whether a submission is admitted at an instant I name, given the same submitter context admit takes, so that I can judge admission without reading the clock.

### R2: Add RecognisedCertificate::admit_at and route admit through it with the present

Structure: RecognisedCertificate gains a public, documented inherent method admit_at taking &self, the submission (&Submission<'_>), the submitter context (&SubmitterContext<'_>) and the instant (chrono DateTime<Utc>), returning Result<(), NotAdmitted>. Behaviour: WHEN admit_at is called, THE SYSTEM SHALL match the context's three arms with no wildcard arm, SHALL return Err(NotAdmitted) for Unidentified, SHALL take the credential from AssertedBySubmitter's bytes and from AuthenticatedByTransport's peer certificate alike, SHALL verify the credential with lys_core::ca::verify_certificate_chain_at against the configured issuer public key at the given instant, and, only after that verification succeeds and only where a subject allow-list is configured, SHALL return Err(NotAdmitted) for a subject key outside it; otherwise it SHALL return Ok(()). WHEN AdmissionPolicy::admit is called on a RecognisedCertificate, THE SYSTEM SHALL return what admit_at returns for the same submission and context at Utc::now(). IF any check refuses, THEN THE SYSTEM SHALL return the one fieldless NotAdmitted. THE SYSTEM SHALL NOT read the clock inside admit_at, SHALL NOT collapse the context's provenance before the call or add an accessor that does, SHALL NOT change admit's parameter types or return type, SHALL NOT change the AdmissionPolicy trait, SHALL NOT add Default or a defaulting constructor, SHALL NOT add a field to NotAdmitted, and SHALL NOT read any certificate extension. certificate.rs's module doc, its issued_by doc and its admit doc are rewritten to say the certificate is judged at the instant its caller names, the present when called through admit, and to name verify_certificate_chain_at; they SHALL NOT say 'right now' or 'currently'.

**Acceptance:**
- grep -c 'pub fn admit_at' crates/lys-anchor/src/admission/certificate.rs prints 1, and that method's parameters after &self are, in order, &Submission<'_>, &SubmitterContext<'_> and DateTime<Utc>, and its return type is Result<(), NotAdmitted>.
- admit_at carries a /// doc comment with an # Errors section, and cargo doc --no-deps -p lys-anchor emits 0 warnings.
- grep -nw 'verify_certificate_chain' crates/lys-anchor/src/admission/certificate.rs prints nothing, and grep -c 'verify_certificate_chain_at(' crates/lys-anchor/src/admission/certificate.rs prints 1, inside admit_at, with the instant parameter as its third argument.
- grep -c 'Utc::now()' crates/lys-anchor/src/admission/certificate.rs prints 1, and that line is inside the AdmissionPolicy::admit impl, as the third argument of its call to self.admit_at.
- The AdmissionPolicy::admit impl's parameter types are &Submission<'_> and &SubmitterContext<'_> and its return type is Result<(), NotAdmitted>, as on <base>.
- admit_at's match on the context names SubmitterContext::Unidentified, SubmitterContext::AssertedBySubmitter and SubmitterContext::AuthenticatedByTransport and has no _ arm, and its Unidentified arm returns Err(NotAdmitted).
- grep -nE 'right now|currently' crates/lys-anchor/src/admission/certificate.rs prints nothing.
- git diff <base> -- crates/lys-core crates/lys-anchor/src/admission/policy.rs crates/lys-anchor/src/admission/context.rs crates/lys-anchor/src/anchor/append.rs prints nothing.

**Files:**
- modify: crates/lys-anchor/src/admission/certificate.rs

**Checklist:**
- C2 — RecognisedCertificate has a public, documented admit_at taking the submission, the SubmitterContext and a DateTime<Utc>, which verifies the chain and validity window at that instant through lys_core::ca::verify_certificate_chain_at.
- C3 — RecognisedCertificate's AdmissionPolicy::admit keeps its signature and calls admit_at with Utc::now().
- C4 — certificate.rs's docs say a certificate is judged at the instant its caller names, and none says 'right now' or 'currently'.

**Stories:**
- S3 (Developer, Embeds RecognisedCertificate in code that must judge admission at a known instant) — As a developer, I want to ask RecognisedCertificate whether a submission is admitted at an instant I name, given the same submitter context admit takes, so that I can judge admission without reading the clock.
- S4 (Operator, Serves a live anchor) — As an operator of a live anchor, I want each live submission still judged at the present, so that adding admit_at changes nothing a live request sees.

### R3: Judge the expiry test at two named instants with no sleep

WHEN an_expired_certificate_is_refused runs, THE SYSTEM SHALL issue the same certificate as on the base commit (seed b"lys-anchor-admission-ca-seed-05!", subject "submitter-a", a one-second lifetime, no extensions), SHALL take its issuance instant as the certificate's expires_at less one second, SHALL assert that admit_at at issuance plus 500 milliseconds returns Ok(()) as the positive control, and SHALL then assert that admit_at at issuance plus 2 seconds returns Err(NotAdmitted). THE SYSTEM SHALL NOT sleep, SHALL NOT read the clock to choose either instant, SHALL NOT drop or reorder the positive control after the refusal, SHALL NOT raise any timeout, SHALL NOT split the test, and SHALL NOT change any other test. The module doc bullet 'The passage of time' is rewritten to say this case judges the certificate at two named instants, one inside its validity window and one two seconds past issuance, and that its refusal is still the only check here that tells a chain verification that consults the validity window from one that does not; the test's own comments SHALL NOT speak of a wait.

**Acceptance:**
- grep -rnE 'thread::sleep|tokio::time::sleep' crates/lys-anchor prints nothing.
- an_expired_certificate_is_refused calls admit_at exactly 2 times, calls admit 0 times and contains no Utc::now().
- Its first assertion is that admit_at(&statement(), &asserted(&short_lived.der_bytes), short_lived.expires_at - 1 s + 500 ms) equals Ok(()).
- Its second assertion is that admit_at(&statement(), &asserted(&short_lived.der_bytes), short_lived.expires_at - 1 s + 2 s) equals Err(NotAdmitted).
- cargo test -p lys-anchor --all-features --lib admission::certificate::tests::an_expired_certificate_is_refused reports 1 passed and 0 failed, and the run's 'finished in' time is under 1 second.
- With admit_at's call to verify_certificate_chain_at temporarily given Utc::now() in place of the instant parameter, cargo test -p lys-anchor --all-features --lib admission:: reports exactly 1 failed test, an_expired_certificate_is_refused, failing at its Err(NotAdmitted) assertion; the change is reverted before the commit.
- grep -c 'waits for a certificate to expire' crates/lys-anchor/src/admission/certificate_tests.rs prints 0.
- git diff <base> -- crates/lys-anchor/src/admission/certificate_tests.rs changes lines only within the module doc bullet 'The passage of time', the use declarations and the body of an_expired_certificate_is_refused; git diff <base> -- crates/lys-anchor/src/admission/context_tests.rs crates/lys-anchor/src/admission/trivial_tests.rs prints nothing.
- cargo test -p lys-anchor --all-features --lib admission:: reports 19 passed and 0 failed.

**Files:**
- modify: crates/lys-anchor/src/admission/certificate_tests.rs

**Checklist:**
- C5 — an_expired_certificate_is_refused contains no sleep and asserts Ok(()) from admit_at at an instant inside the one-second window and Err(NotAdmitted) at issuance plus 2 seconds.
- C6 — grep -rnE 'thread::sleep|tokio::time::sleep' crates/lys-anchor prints nothing.
- C7 — certificate_tests.rs's module doc describes the expiry case as judged at named instants, not as waiting for expiry.
- C8 — The other 18 admission tests are byte-unchanged and pass.

**Stories:**
- S1 (Operator, Runs the workspace's gate on a shared, loaded build host) — As an operator running the gate on a loaded host, I want the expired-certificate test to finish without waiting on the clock, so that the suite stays fast and its outcome does not depend on load.
- S2 (Reviewer, Checks that the certificate gate's tests can tell a correct gate from a broken one) — As a reviewer, I want the expiry refusal and its positive control judged at explicit instants, so that the refusal still proves the chain verification consults the validity window.

## Boundaries

- SHALL NOT change any file under crates/lys-core, its public API included.
- SHALL NOT change the AdmissionPolicy trait, SHALL NOT add a field to NotAdmitted, and SHALL NOT add Default or a defaulting constructor to any policy.
- SHALL NOT change crates/lys-anchor/src/anchor/append.rs or admit's parameter types or return type.
- SHALL NOT change any test other than an_expired_certificate_is_refused, the lys-home sleeps included; SHALL NOT raise a timeout; SHALL NOT split a test.
- SHALL NOT change how notAfter is truncated at issuance.
- SHALL NOT add a dependency other than chrono from the workspace, and SHALL NOT inject a clock into the policy.
- SHALL NOT rename, move or change the eight pre-method documents in docs/design/lys-anchor.
- SHALL NOT leave any file over 500 lines of code, and SHALL NOT use unwrap, expect, panic, todo, unimplemented or unreachable in library code.
- SHALL NOT silence a lint with #[allow], #[ignore] a test, add a _-prefixed unused binding, or use #[cfg(any())].
- SHALL NOT land the code change as more than one commit on the card branch.

## Verification

- On the build machine, from the repository root: cargo fmt --all -- --check, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo doc --no-deps --all-features and cargo doc --no-deps each exit 0 with 0 warnings.
- On the build machine, from the repository root: cargo test --workspace --all-features exits 0 with 0 failed tests.
- From the repository root: sh scripts/design/gate.sh exits 0.
- git rev-list --count <base>..HEAD on the card branch prints 1, where <base> is the commit the build started from.
- git diff --name-only <base>..HEAD prints exactly Cargo.lock, crates/lys-anchor/Cargo.toml, crates/lys-anchor/src/admission/certificate.rs and crates/lys-anchor/src/admission/certificate_tests.rs.

