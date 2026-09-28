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
