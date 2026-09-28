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

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1, met: crates/lys-anchor/Cargo.toml line 71 is `chrono.workspace = true`, directly after the [dependencies] header and before [dev-dependencies]. It is the only line of that form. Row 2, met: in Cargo.lock the lys-anchor dependencies are base64 0.22.1, chrono, lys-core, lys-log-store, serde_json, sha2, tempfile, thiserror 2.0.19. Row 3, met: git diff --stat shows 1 insertion and 0 deletions for crates/lys-anchor/Cargo.toml. Row 4, met: the root Cargo.toml is untouched and the workspace chrono entry at line 38 is unchanged.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys-anchor/Cargo.toml` — [dependencies] now lists chrono.workspace = true, the workspace's existing chrono 0.4 entry, as the first line of the table
  - modified: `Cargo.lock` — The lys-anchor package's dependency list now includes "chrono", the already-resolved 0.4.45
- Checklist delivery:
  - [x] C1 — crates/lys-anchor/Cargo.toml lists chrono.workspace = true under [dependencies], and no other dependency is added. — One added line under [dependencies]; nothing else added or changed.
- Story delivery:
  - [x] S3 (Developer, Embeds RecognisedCertificate in code that must judge admission at a known instant) — As a developer, I want to ask RecognisedCertificate whether a submission is admitted at an instant I name, given the same submitter context admit takes, so that I can judge admission without reading the clock. — chrono is available, so admit_at can take DateTime<Utc>.

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

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1, met: certificate.rs line 214 has `pub fn admit_at(&self, _submission: &Submission<'_>, context: &SubmitterContext<'_>, at: DateTime<Utc>) -> Result<(), NotAdmitted>`, the only such line. Row 2, met in source, not run: admit_at's /// doc (lines 196-213) has an # Errors section. Its intra-doc links point at public items ([`AdmissionPolicy::admit`], [`NotAdmitted`]); the module doc links [`AdmissionPolicy::admit`], which is in scope through the existing `use`. The cargo doc 0-warning half was not run, per the instructions. Row 3, met: `grep -nw verify_certificate_chain` matches nothing. The only `verify_certificate_chain_at(` call is at line 232 inside admit_at: `verify_certificate_chain_at(credential, &self.issuer_public_key, at)`. Row 4, met: grep -c 'Utc::now()' prints 1, at line 270 inside the AdmissionPolicy impl: `self.admit_at(submission, context, Utc::now())`. Doc prose says 'the present' so the grep count stays exact. Row 5, met: admit's signature is still (&self, &Submission<'_>, &SubmitterContext<'_>) -> Result<(), NotAdmitted>. Row 6, met: lines 225-229 match Unidentified => return Err(NotAdmitted), AssertedBySubmitter(bytes) => *bytes and AuthenticatedByTransport(peer) => peer.certificate(), with no _ arm. Row 7, met: grep -nE 'right now|currently' prints nothing. Row 8, met: no file under crates/lys-core, and neither policy.rs, context.rs nor anchor/append.rs, is modified (git diff --stat lists only the four intended files). admit_at reads no clock and no extension. NotAdmitted, the trait and the constructors are untouched.
- Deviation: The statement parameter of admit_at is bound as `_submission`, because the policy never reads the statement. This is the same binding the base admit impl carried, moved rather than added: admit now binds `submission` and passes it on, so the workspace's count of underscore-prefixed bindings is unchanged. It is disclosed here because the brief bars adding one.
- Files changed:
  - modified: `crates/lys-anchor/src/admission/certificate.rs` — Imports chrono::{DateTime, Utc} and verify_certificate_chain_at. Adds a public, documented RecognisedCertificate::admit_at with an # Errors section: it matches the three SubmitterContext arms with no wildcard, verifies the credential at the caller's instant, then checks the subject allow-list. AdmissionPolicy::admit now delegates to admit_at at Utc::now(). The module, issued_by and admit docs say the certificate is judged at the instant its caller names (the present through admit) and name verify_certificate_chain_at.
- Checklist delivery:
  - [x] C2 — RecognisedCertificate has a public, documented admit_at taking the submission, the SubmitterContext and a DateTime<Utc>, which verifies the chain and validity window at that instant through lys_core::ca::verify_certificate_chain_at. — admit_at at certificate.rs:214, verifying through verify_certificate_chain_at at :232.
  - [x] C3 — RecognisedCertificate's AdmissionPolicy::admit keeps its signature and calls admit_at with Utc::now(). — admit keeps its signature and calls self.admit_at(submission, context, Utc::now()) at :270.
  - [x] C4 — certificate.rs's docs say a certificate is judged at the instant its caller names, and none says 'right now' or 'currently'. — Module doc item 1, the 'What the chain check covers' section, issued_by and admit docs rewritten; no 'right now' or 'currently' remains.
- Story delivery:
  - [x] S3 (Developer, Embeds RecognisedCertificate in code that must judge admission at a known instant) — As a developer, I want to ask RecognisedCertificate whether a submission is admitted at an instant I name, given the same submitter context admit takes, so that I can judge admission without reading the clock. — admit_at takes the same submission and context as admit, plus the instant.
  - [x] S4 (Operator, Serves a live anchor) — As an operator of a live anchor, I want each live submission still judged at the present, so that adding admit_at changes nothing a live request sees. — admit is admit_at at Utc::now(), with the same checks in the same order, so a live request is judged exactly as before.

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

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1, met: grep -rnE 'thread::sleep|tokio::time::sleep' crates/lys-anchor prints nothing (checked). Row 2, met: the test body (from certificate_tests.rs:185) calls policy.admit_at twice, admit zero times, and contains no Utc::now(). Row 3, met: the first assertion is admit_at(&statement(), &asserted(&short_lived.der_bytes), issued_at + TimeDelta::milliseconds(500)) == Ok(()), where issued_at = short_lived.expires_at - TimeDelta::seconds(1). Row 4, met: the second assertion is admit_at(..., issued_at + TimeDelta::seconds(2)) == Err(NotAdmitted). Row 5, not run, per the instructions: there is no sleep and there are two in-memory verifications, so the test should finish well under a second. The instants hold because lys-core's validity_window truncates notAfter to whole seconds and notBefore is issued_at at whole-second DER precision, so expires_at − 1 s equals notBefore. Row 6, reasoned, not run, per the instructions: given Utc::now(), the positive control still passes (now is inside the one-second window just after issuance) and the refusal assertion fails. The other 18 admission tests already go through admit, which is Utc::now(), so they are unaffected and exactly one test fails, at its Err(NotAdmitted) assertion. Row 7, met: grep -c 'waits for a certificate to expire' prints 0. Row 8, met: the diff touches only the module-doc bullet, the use block (the added chrono import) and the test body; context_tests.rs and trivial_tests.rs are untouched. Row 9, not run: 19 admission tests are expected to pass, since only this test changed.
- Deviation: Rows 5, 6 and 9 need cargo runs, and the brief says to run no build or test command. They are argued from source here and left to the workflow's gate. I did not perform the drift injection, so there is nothing to revert.
- Files changed:
  - modified: `crates/lys-anchor/src/admission/certificate_tests.rs` — Adds `use chrono::TimeDelta;`. The 'The passage of time' module-doc bullet now describes judging at two named instants. an_expired_certificate_is_refused issues the same one-second certificate, takes issued_at = expires_at − 1 s, asserts admit_at at issued_at + 500 ms is Ok(()) and then admit_at at issued_at + 2 s is Err(NotAdmitted). There is no sleep.
- Checklist delivery:
  - [x] C5 — an_expired_certificate_is_refused contains no sleep and asserts Ok(()) from admit_at at an instant inside the one-second window and Err(NotAdmitted) at issuance plus 2 seconds. — No sleep; Ok(()) at issuance + 500 ms, Err(NotAdmitted) at issuance + 2 s.
  - [x] C6 — grep -rnE 'thread::sleep|tokio::time::sleep' crates/lys-anchor prints nothing. — The grep prints nothing.
  - [x] C7 — certificate_tests.rs's module doc describes the expiry case as judged at named instants, not as waiting for expiry. — The bullet now reads 'The passage of time, at named instants' and describes two named instants, with nothing about waiting.
  - [x] C8 — The other 18 admission tests are byte-unchanged and pass. — The other 18 tests are byte-unchanged; passing is left to the gate.
- Story delivery:
  - [x] S1 (Operator, Runs the workspace's gate on a shared, loaded build host) — As an operator running the gate on a loaded host, I want the expired-certificate test to finish without waiting on the clock, so that the suite stays fast and its outcome does not depend on load. — The 2100 ms sleep is gone and the outcome no longer depends on host load.
  - [x] S2 (Reviewer, Checks that the certificate gate's tests can tell a correct gate from a broken one) — As a reviewer, I want the expiry refusal and its positive control judged at explicit instants, so that the refusal still proves the chain verification consults the validity window. — The positive control and the refusal are both judged at explicit instants, so the refusal still requires the window to be consulted.

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
