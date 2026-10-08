---
type: brief
id: DIRECTORY-088
cluster: directory
title: Supply creation and session time without clock waits in CA and request tests
---

# DIRECTORY-088: Supply creation and session time without clock waits in CA and request tests

> **Cluster:** directory
> **Depends on:** DIRECTORY-040, DIRECTORY-046
> **Checklist:**
> - C212 — `lys ca issuer-cert --key <path> --out <path>` writes the issuer's self-signed CA certificate as one PEM block carrying no key material, and it and `lys ca issue --issuer-out` write the same bytes on every call, from one issuer certificate stored beside the CA key when it is first built.
> - C349 — A request is found by id, not by scanning every request (DIRECTORY-046 R2).
> **Stories:**
> - S84 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.
> - S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

## Purpose

Scone's fixed 49-test audit at source 71e44c78360d2b2228657dc5213026f920526fb9, report SHA256 bd725c0679735ecd7489974238284ee5ae000aa750a29327f982e24ed8f5618e, identifies rows 1/3/41 as two issuer-cert tests and one request expiry test. The CA helper sleeps two seconds three times; the request test sleeps three seconds. Completion signals do not advance a creation or expiry clock. On current local main 078dde2cc182708433e059223fcd87e5ba8b234e, authority.rs:262/306 and session.rs:33 still read wall time directly. CA tests execute the actual CLI, and requests.rs uses the real identity-contract HTTP service. This brief supplies those production owners with instance clocks so the same behavior can be tested immediately. C212/S84 are split with DIRECTORY-031/040 to preserve issuer reuse and external verification; C349/S148 are split with DIRECTORY-046 to preserve indexed request/replay behavior without extra time work. No broader identity, grant or clock policy is changed.

## Task

Write the injected-clock contract for both CA creation paths and the request/session owner, a test-only real-CLI injection road, and the three unchanged-assertion regressions. Specify positive controls that prove actual later supplied instants would expose a regenerated issuer and an expired fresh request. Preserve system time in production, old stored bytes, boundary comparisons and durable acknowledgments. Runtime implementation and remote proof follow approved bounded handovers; this box writes documents only.

## Requirements

### R1: Own the clock at construction and keep the production default

THE SYSTEM SHALL give CA certificate creation and the request/session owner an instance-owned Clock supplied through construction. Its read returns a typed UTC instant or a named clock failure; conversion to the existing unsigned Unix-second request/session representation is checked. SystemClock remains the default in every existing production constructor and executable. The provider is shared by one service and its Sessions, and by one CA command and its CertificateAuthority; different instances can hold different supplied clocks concurrently. A supplied clock is trusted fixture construction input, never an HTTP field, cookie, directory record, ordinary CLI flag, environment override or process-global mutable variable. Manual advancement assigns a value and acknowledges that assignment synchronously; it never schedules a timer or waits for wall time. Production needs no clock Mutex, per-read allocation, background task, disk write or history scan. Clock failure or an unrepresentable session instant is a named failure before mutation, never epoch zero, a cached instant or a system-clock fallback. Keep unrelated time owners outside this slice.

**Acceptance:**
- clock_instances_are_independent constructs two owners at distinct supplied instants, advances only one, and proves each creation/read sees exactly its own provider. Parallel tests share no mutable clock or global setting. The provider cannot be replaced by a request.
- clock_production_default uses the unchanged production constructors and executable and proves their source is SystemClock. The ordinary lys binary has no fixture-time option and reads no test-time environment variable, including when all features are compiled; the test helper is a separate, uninstalled target.
- clock_conversion_refuses_before_mutation supplies a failing provider and a UTC instant outside the request/session unsigned-second representation. Each returns a named clock-unavailable error before a new certificate file, session row or request is written; no zero timestamp or fallback is accepted. CA date-range and TTL validation retain their existing named refusals.

**Files:**
- create: crates/lys-core/src/clock.rs
- create: crates/lys-core/tests/clock.rs
- modify: crates/lys-core/src/lib.rs

**Checklist:**
- C212 — `lys ca issuer-cert --key <path> --out <path>` writes the issuer's self-signed CA certificate as one PEM block carrying no key material, and it and `lys ca issue --issuer-out` write the same bytes on every call, from one issuer certificate stored beside the CA key when it is first built.

**Stories:**
- S84 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.

### R2: Use the supplied clock in both CA creation paths

WHEN the authority creates an issuer or leaf certificate, THE SYSTEM SHALL read its owned clock in place of the direct Utc::now calls in ca/authority.rs:262 and :306. Both issue_certificate and issue_certificate_for_request use the existing shared validity_window calculation, with the supplied creation instant passed explicitly; issuer_certificate_der uses the same provider. Preserve whole-second DER conversion, TTL validation and overflow checks, issuer notAfter, subjects, extensions, Ed25519 signing and proof-of-possession behavior. Stored issuer reuse is still decided by the existing CLI stored_issuer_certificate path, before creation; advancing a clock never regenerates a valid stored issuer. Verification is outside this creation seam: the existing verify_certificate_chain_at remains the explicit verifier for a supplied verification instant. No frozen verification clock, longer certificate window or changed expiry comparison is introduced.

**Acceptance:**
- issuer_creation_obeys_supplied_time creates issuer DER with a known key at T0 and in a separate fresh authority at T0+2 seconds. Independent X.509 parsing finds exactly those notBefore values, the same public key and the unchanged notAfter; the two DER values differ. A provider ignored in favor of Utc::now must fail this assertion, rather than accidentally pass a byte-reuse test.
- both_leaf_creation_paths_obey_supplied_time exercises generated-subject and verified-request issuance at supplied instants. The existing TTL, reported issued_at/expires_at, DER truncation, chain, extensions and proof-of-possession assertions remain, with exactly the existing time-boundary comparisons.
- issuer_cache_does_not_read_creation_time exports an already valid stored issuer after the provider advances, then after a creation-only provider is set to fail. The stored/exported bytes are unchanged and no issuer creation is attempted; invalid stored bytes retain their named refusal and are never rebuilt as a fallback.

**Files:**
- modify: crates/lys-core/src/ca/authority.rs
- modify: crates/lys/src/commands/ca.rs

**Checklist:**
- C212 — `lys ca issuer-cert --key <path> --out <path>` writes the issuer's self-signed CA certificate as one PEM block carrying no key material, and it and `lys ca issue --issuer-out` write the same bytes on every call, from one issuer certificate stored beside the CA key when it is first built.

**Stories:**
- S84 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.

### R3: Keep real CLI journeys and replace their three clock waits

THE SYSTEM SHALL supply time to the two existing issuer-cert integration tests through a separate test-only CLI target, not a production clock override. The helper parses the actual CLI arguments and calls one shared CA dispatcher and the existing issuer-cert/issue, stored-file, log, staging and error paths. The normal main calls that same dispatcher with SystemClock; no copied CLI implementation, simulated Output, fake certificate writer or mock log is sufficient. The helper takes its supplied instant explicitly at process construction, is behind a non-default clock-fixture feature and is never installed. Only the named test journeys select it. Remove let_a_second_pass and its three calls, update the obsolete pause explanation, and keep every original assertion and real child completion. The OpenSSL verification helper receives the supplied verification instant, so a fixed historical test clock does not cause the preserved standard-tool assertion to expire as wall time advances. Its assertion still checks the actual generated files with the external tool.

**Acceptance:**
- issuer_cert_writes_the_stored_certificate_and_every_later_call_writes_the_same_bytes retains every assertion from issuer_cert.rs:84: one PEM block, public-key correspondence and lengths, output path, stored-byte equality, subject/basicConstraints, absence of private material, repeated-export equality, issue --issuer-out equality and successful external verification. First creation is at T0; later issuer-cert is at T0+2 and issue is at T0+4. No sleep, timeout, deadline, polling or busy loop remains.
- an_issuer_certificate_first_built_by_issue_is_the_one_issuer_cert_writes_later retains every assertion from issuer_cert.rs:151. The real issue command first creates and stores the issuer at T0; the real issuer-cert command at T0+2 exports exactly those bytes. No library-only replacement or omitted store path qualifies.
- issuer_regeneration_control uses the same key in two separately owned fresh stored-issuer fixtures at T0 and T0+2. Their independently parsed creation seconds and different DER bytes prove the advanced-time fixture can detect a rebuild. The original tests still require equality when the original store is reused. A control that only compares one cached file to itself fails qualification.

**Files:**
- create: crates/lys/src/commands/ca_dispatch.rs
- create: crates/lys/tests/clock_cli.rs
- modify: crates/lys/Cargo.toml
- modify: crates/lys/src/main.rs
- modify: crates/lys/src/commands/mod.rs
- modify: crates/lys/src/commands/ca.rs
- modify: crates/lys/tests/ca_log/issuer_cert.rs
- modify: crates/lys/tests/ca_log/support.rs
- modify: crates/lys/tests/ca_log_tests.rs

**Checklist:**
- C212 — `lys ca issuer-cert --key <path> --out <path>` writes the issuer's self-signed CA certificate as one PEM block carrying no key material, and it and `lys ca issue --issuer-out` write the same bytes on every call, from one issuer certificate stored beside the CA key when it is first built.

**Stories:**
- S84 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.

### R4: Share request time with the service session owner

THE SYSTEM SHALL construct the request routes and Sessions with the same instance-owned clock. Add a construction seam for the real service/harness; existing service, service_saying, Sessions::new and Sessions::open retain their SystemClock defaults. The request routes that currently import session::now use the owned provider, and Sessions uses it for opening/pruning, beginning, checking and listing sessions. Sample at the existing logical boundary and pass an already sampled value through an operation rather than adding repeated reads inside a store loop. Do not change session duration, cookie attributes, request expiry rules, existing replay lookup order or the indexed request-by-id seam. A kept operation still returns its original id/asked_at after its requested end; a fresh operation whose ends_at is less than or equal to the sampled instant still refuses RequestMalformed. Test advancement never expires the caller cookie unintentionally: fixture session duration covers all supplied request instants, and a separate boundary case proves expiry at equality. The clock is transient construction state only; no stored session/request shape changes. This includes all five current request time reads: requests_api.rs:310/382 and requests_decide.rs:144/261/285 for list, ask, approve, settle and decline. Each uses the same owned provider; no request decision keeps a hidden session::now read.

**Acceptance:**
- a_kept_request_is_answered_again_after_the_end_it_asked_for retains every assertion from requests.rs:334 through the same actual authenticated HTTP route. Set T0, ask with ends_at=T0+2, await the first 200 reply, set T0+3, then replay the identical operation/body. Assert 200 and unchanged id/asked_at. A fresh operation with that same ended window refuses 400/RequestMalformed. No sleep or wall-clock read creates the test window.
- request_clock_boundaries queries with ends_at=T0+1, T0 and T0-1 in separate fresh operations. Only the future window is admitted; equality and past refuse with unchanged store contents. A stored identical replay keeps its original fields, while changed content under the same id retains RequestReused. Approve, settle and decline also observe that same supplied instant while preserving their existing authorization/decision/refusal rules; advancing one service never changes another service's answers.
- session_clock_is_shared begins and checks a real fixture session using the same provider as requests, with a deliberately longer session lifetime. At its exact session end it is not signed in; before it, it is live. Two services with separate clocks do not affect one another. No wall-clock-derived auth-session fallback masks an expired or invalid supplied time.

**Files:**
- create: crates/lys-identity-server/tests/clock.rs
- modify: crates/lys-identity-server/src/session.rs
- modify: crates/lys-identity-server/src/requests_api.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/routes_startup.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/src/error_names.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/tests/requests.rs
- modify: tests/identity_contract/src/harness.rs
- modify: tests/identity_contract/src/harness_serve.rs
- modify: crates/lys-identity-server/src/requests_decide.rs

**Checklist:**
- C349 — A request is found by id, not by scanning every request (DIRECTORY-046 R2).

**Stories:**
- S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

### R5: Prove no clock waits and count only completed work

THE SYSTEM SHALL preserve a before/after assertion manifest for the three named tests and a complete fixture/source receipt. Source at the fixed base has three calls to the CA two-second helper and one request three-second sleep: four clock waits, nine seconds of deliberate waiting in one execution of all three tests. Scone reports 2.103, 4.156 and 3.576 seconds on the qualification source; these are historical observations, not a new run or a causal performance measurement. Add a runnable bounded no-clock-wait check over the exact named tests and reachable helper, with an introduced-wait positive control that goes red. Missing injection API or failed compilation is not a behavioral red. New supplied-time assertions must execute and fail if the real creation/route path ignores the provider, and pass only after it uses it. Count real provider reads per completed command/HTTP operation, certificate creations, request appends/replays/refusals, new locks, full-state clones, sync stages and history visits; timing ends at actual command exit or HTTP reply with durable evidence, not at enqueue. Zero waits is measured; zero overhead is never assumed. The no-clock-wait source check is a source-constraint red, reported separately from the creation-time and HTTP behavioral reds. Its fixed call graph includes the two named CA tests and their reachable helper and the named request test; unknown wait indirection refuses qualification rather than reporting zero.

**Acceptance:**
- clock_wait_ratchet proves the source check detects each of the four existing wait calls on the base and an intentionally introduced wait in its independent fixture. After the change all three tests and their reachable helpers have zero clock waits; the controlled-clock advancement is synchronous and uses no watchdog. The original assertion manifest is complete and unweakened.
- clock_work_counts pairs actual counts and native completion times at the exact source and change: issuer creation has one required creation-time read, direct leaf issuance has the two existing creation/validity reads, and a warm stored-issuer export has zero creation reads. Request/session counts are reported at their existing distinct decision boundaries and must not grow per call. No added production locks, per-read allocation, disk sync or history traversal is accepted. Physical dependency sync totals remain unmeasured until instrumented; they are never labelled zero.
- clock_red_green_receipts names each observed assertion, run id, exact SHA/tree, complete run/pass/fail/skip counts and elapsed test times. All three existing tests, regeneration controls, conversion/error controls and parallel-isolation controls run. A skipped subset or historical pass is not the new proof; any test over 2 seconds is a defect whose remaining cause is reported and repaired within its owning slice rather than hidden.

**Files:**
- create: crates/lys-core/tests/clock.rs
- create: crates/lys-identity-server/tests/clock.rs
- modify: crates/lys/tests/ca_log/issuer_cert.rs
- modify: crates/lys/tests/ca_log/support.rs
- modify: crates/lys-identity-server/tests/requests.rs

**Checklist:**
- C212 — `lys ca issuer-cert --key <path> --out <path>` writes the issuer's self-signed CA certificate as one PEM block carrying no key material, and it and `lys ca issue --issuer-out` write the same bytes on every call, from one issuer certificate stored beside the CA key when it is first built.
- C349 — A request is found by id, not by scanning every request (DIRECTORY-046 R2).

**Stories:**
- S84 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.
- S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

### R6: Qualify preservation and deliver bounded code slices later

THE SYSTEM SHALL keep this brief separate from implementation. Its later code handovers name exact owned files, base, remote seat checkout and immutable source road. Split the work into clock contract, CA/CLI fixture wiring, request/session wiring and qualification slices; each has its own smallest write/check scope and observed reds before correct behavior. Format the actual files and run focused nextest plus strict touched-crate Clippy in default and all-target/all-feature configurations on the approved remote venue; no Cargo on this Mac and no whole suite or final gate by the writer. Include lys-core, lys, lys-identity-server and identity-contract when touched. The lead owns the one assembled gate after all required code is written. Keep the installed issuer PEM and session/request records byte-compatible: injection adds no persisted field, schema version or migration. Prove an existing stored issuer and persisted request/session survive close/reopen under unchanged production defaults, including kept replay and expiry at the supplied test boundary. If implementation needs a stored-shape change, different expiry/security policy or a wider time owner, stop for a separate decision rather than use a shim, old-format refusal or silent fallback.

**Acceptance:**
- clock_old_state_preserved opens the actual supported old issuer PEM and session/request bytes. The issuer is exported byte-equal, valid unexpired sessions retain their ids/actors/windows, and a kept request retains id/asked_at on replay after its requested end. Supplied test time is never persisted. All existing refusal and durable-before-success assertions remain.
- clock_delivery_manifest lists every changed hot path with locks held, whole-state clones, explicit disk syncs per completed call and all-history loops. It reports exact commands/exits and full focused counts, with default/all-feature strict Clippy exit 0 before publication under the standing rule. Remote source identity, full lead gate, landing and installation are distinct evidence; no code or runtime proof is claimed by this document gate.

**Files:**
- create: crates/lys-core/tests/clock.rs
- create: crates/lys-identity-server/tests/clock.rs

**Checklist:**
- C212 — `lys ca issuer-cert --key <path> --out <path>` writes the issuer's self-signed CA certificate as one PEM block carrying no key material, and it and `lys ca issue --issuer-out` write the same bytes on every call, from one issuer certificate stored beside the CA key when it is first built.
- C349 — A request is found by id, not by scanning every request (DIRECTORY-046 R2).

**Stories:**
- S84 (Stranger, Checks an issued certificate and its log entry offline, holding nothing from lys) — As a stranger holding the issuer's certificate, an issued certificate, its leaf and its inclusion artifact, I want to check the certificate and its entry in the log offline with openssl and a standard-library script, so that I rely on nothing from lys and on no one's word that the certificate was logged.
- S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

## Boundaries

- Only DIRECTORY-088.json/.md and necessary additions-only structure declarations in directory/design.json plus rendered DESIGN.md are written in this box. Existing checklist/story entries and every other brief stay unchanged; no source/test/Cargo file is edited.
- Creation time and request/session time are the only production owners changed by the later implementation. Verification policy, issuer notAfter, TTL/session duration, cookie attributes, stored formats, replay lookup, grant semantics and unrelated time owners are preserved.
- No production CLI/HTTP/environment clock override, global frozen clock, elapsed-time watchdog, sleep, polling or busy loop is introduced. The separate clock fixture target is uninstalled and default-off; the normal executable always selects SystemClock.
- The three original tests keep every assertion and their real CLI/HTTP/store paths. Independent later-time regeneration and fresh-expiry controls prevent removal of the sleeps from making byte equality or expiry vacuous. Standard external certificate verification uses the supplied verification instant.
- No observed runtime red/green, new performance baseline, physical sync total, gate, publication, landing or installed hospital readiness follows from this brief. A missing injection API is an implementation prerequisite, not an observed behavioral red.
- No persisted shape changes are planned, so no migration is invented. An actual need for a shape/policy/wider-owner change is a stop, with exact source evidence and a separate decision.

## Verification

- Read the fixed audit report's three relevant rows, source identity and summary counts, then the three exact tests, CA creation/stored-file/CLI dispatcher seams, request route and session owner, and actual service fixture construction. Verify all create/modify paths at the frozen local-main SHA; created targets must be absent and existing targets present.
- Format the JSON sources, render through scripts/design/render-cluster.py, preserve every preexisting design object and original source byte outside additions, and verify every new JSON string leaf is represented in Markdown. Check exact four-path scope and preservation of foreign WIP/staged entries without treating index stat-cache bytes as staged-content identity.
- Run scripts/design/gate.sh in an owned base-plus-four-path docs/scripts scratch candidate under the existing Python-docs permission. Read complete stdout/stderr and record exit, valid documents, clean clusters, warnings and parsed failures; require exit 0 and zero failures. The source/remote runtime gate does not run in this box.
- After Crumpet's final-readback signal, commit only the four paths straight to local main. Verify exact parent/SHA/tree/path list and all four committed blobs equal the gated candidate. Hold push, retain full command receipts and remove only the owned scratch archive/candidate.
- The later remote implementation records the no-wait red and positive controls separately from missing APIs/compile errors, then runs every original and added behavioral assertion. Every test is under 2 seconds, or remains a named owning-slice defect; no timer is raised and no assertion or case is removed.
- Report the system-clock default, per-instance isolation, failure-before-mutation, old-state/reopen preservation, completed-work counts and native-time measurements. Numeric baseline/correlation remains unqualified until run. The lead's assembled gate, publication, installation and runtime use remain separate.
