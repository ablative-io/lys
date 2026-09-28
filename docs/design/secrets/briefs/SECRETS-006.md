---
type: brief
id: SECRETS-006
cluster: secrets
title: The broker signs with a key it holds and returns the signature only, and the key never leaves it
---

# SECRETS-006: The broker signs with a key it holds and returns the signature only, and the key never leaves it

> **Cluster:** secrets
> **Depends on:** SECRETS-005, SECRETS-007
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-124 — A signing key is used inside the secrets broker, which signs typed members under a purpose's domain and never gives the key out — A secret may be sealed as a signing key with one purpose from a closed list. The broker signs for a live, admitted handle, builds the bytes to sign itself from typed members with the same function the verifier uses, and answers a COSE_Sign1 signature and the public key only.
> **Checklist:**
> - C411 — A signing key is stored as a secret with one named purpose; no route that carries a value serves it, and listing shows its public key and never its seed (SECRETS-006 R1).
> - C412 — The signing route admits the use as any other, builds the bytes to sign from typed members, and returns the signature only; it signs no raw digest (SECRETS-006 R2).
> - C413 — Every signing use and every refused one is on the audit record, and a revoked handle signs nothing (SECRETS-006 R3).
> **Stories:**
> - S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key.

## Purpose

The broker admits a use of a secret and never gives its value out. A signing key is the one secret that cannot be used that way today. To sign, a caller would have to be given the key. DIRECTORY-049 R7 first had `lys mcp` read an agent's certificate key out of the broker at each call, which the broker's own rule forbids. This card gives the broker a signing use, so a caller that holds a handle to a key gets signatures and never the key. It builds after SECRETS-007 lands, because both change crates/lys-secrets/src/bin/lys-secrets/main.rs and crates/lys-secrets/src/audit.rs. DIRECTORY-045 and DIRECTORY-053 also change main.rs, so the card round rebases on main and reads each cite again before it builds.

## Task

Let a secret be sealed as a signing key with the one purpose it signs for. Add a signature route that takes a handle, its signed presentation and the typed members of the thing to be signed. The broker builds the bytes to sign itself from one function shared with the verifier, signs them as a COSE_Sign1 and answers the signature under the broker's ordinary admission and audit. The broker never signs a digest or bytes a caller composed.

## Requirements

### R1: A signing key is a secret with one named purpose

Behavioural. EntryClass (crates/lys-secrets/src/store.rs line 43) gains a new variant SigningKey. It is apart from Key, which stays a key used only through the proxy (store.rs line 47). A signing key's entry holds an Ed25519 seed of 32 bytes and the one purpose it signs for, in a new purpose member of the entry beside its class. It is sealed once by a new subcommand seal-signing-key in crates/lys-secrets/src/bin/lys-secrets/cli.rs, dispatched in main.rs, taking --name, --owner and --purpose and reading the seed from standard input. Seal (cli.rs line 31) keeps its --upstream, and RecordClass (args.rs line 69) is unchanged. Purposes are a closed enum SigningPurpose in the new crates/lys-secrets/src/broker/signing.rs, each with its own domain label. The first is agent_request, whose domain label is the one lys-identity-server signs under today (crates/lys-identity-server/src/agent_signature.rs line 35). A stored signing key has no value-bearing use. Two routes and two commands carry a value today, and each refuses a signing key not_a_value_secret. The routes are the credential proxy (the fallback at serve.rs line 118 and its handler proxy at line 175) and the OAuth proxy (bin/lys-secrets/oauth_proxy.rs). The commands are SpawnLogin (main.rs line 324, which calls spawn_login at broker/spawn.rs line 24) and ReadRecord (main.rs line 407, which reads through broker/records.rs). The four refusals of this card are signing_key_invalid, signing_purpose_unknown, signing_purpose_mismatch and not_a_value_secret. They are one new enum SigningRefusal in crates/lys-secrets/src/error/signing.rs. SecretsError (error.rs) gains one variant Signing holding it by #[from]. Its name comes through error/name.rs, and refused in callers.rs (line 42) maps its status. The refusals signing_key_invalid and signing_purpose_unknown answer 400. The refusals signing_purpose_mismatch and not_a_value_secret answer 403. The listing (GET /_lys/secrets, view.rs) shows a signing key's handle, scope, holder, purpose and public key, never the seed. The tests of R1 are in crates/lys-secrets/src/broker/signing_tests.rs, declared in signing.rs as its tests module, and the route tests are in crates/lys-secrets/tests/signing.rs.

**Acceptance:**
- A seed of 31 bytes is refused signing_key_invalid naming the length 31.
- A purpose outside SigningPurpose is refused signing_purpose_unknown naming it.
- The credential proxy refuses a signing key not_a_value_secret.
- The OAuth proxy refuses a signing key not_a_value_secret.
- The login hand-over at spawn refuses a signing key not_a_value_secret.
- The sealed record read refuses a signing key not_a_value_secret.
- The listing of a signing key holds its public key.
- The listing of a signing key holds its purpose.
- The listing of a signing key holds neither the seed's bytes nor their hex, checked by a test that searches the answer for both.

**Files:**
- create: crates/lys-secrets/src/broker/signing.rs
- create: crates/lys-secrets/src/broker/signing_tests.rs
- create: crates/lys-secrets/src/error/signing.rs
- modify: crates/lys-secrets/src/broker.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/src/store.rs
- modify: crates/lys-secrets/src/error.rs
- modify: crates/lys-secrets/src/error/name.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/cli.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/main.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/callers.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/view.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/serve.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/oauth_proxy.rs
- modify: crates/lys-secrets/src/broker/spawn.rs
- modify: crates/lys-secrets/src/broker/records.rs

**Checklist:**
- C411 — A signing key is stored as a secret with one named purpose; no route that carries a value serves it, and listing shows its public key and never its seed (SECRETS-006 R1).

**Stories:**
- S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each acceptance row, met in code; tests written but never run:
(1) A 31-byte seed is refused: seed_of in broker/signing.rs refuses signing_key_invalid naming 'a seed of 31 bytes, not 32'. Test: signing_tests.rs a_seed_of_31_bytes_is_refused_signing_key_invalid_naming_31 (line 84).
(2) An unknown purpose is refused: SigningPurpose::parse refuses signing_purpose_unknown naming it, and is used by the CLI and the route. Test: signing_tests.rs line 99.
(3) Credential proxy: admission refuses not_a_value_secret (using.rs current_for). Tests: tests/signing.rs the_credential_proxy_refuses_a_signing_key_not_a_value_secret (line 193), served binary, 403, use count unchanged; library test at signing_tests.rs line 113.
(4) OAuth proxy: Ticket::oauth and oauth_grant_of refuse. Test: signing_tests.rs the_oauth_proxy_refuses_a_signing_key_not_a_value_secret (line 135).
(5) Spawn: signing_tests.rs line 166.
(6) Sealed record read: signing_tests.rs line 179.
(7) Listing holds the public key: tests/signing.rs line 207, checked against the key made from the test's own seed.
(8) Listing holds the purpose: tests/signing.rs line 217.
(9) Listing holds neither the seed's bytes nor their hex: tests/signing.rs line 227 searches the whole answer for the raw bytes, lower-case hex and upper-case hex.
- Deviation: (a) crates/lys-secrets/src/bin/lys-secrets/oauth_proxy.rs was not modified. Both of its value reads go through the library (Ticket::oauth in forward, oauth_grant_of in revoke_after_drop), so the refusal lives in crates/lys-secrets/src/broker/oauth_grants.rs, which the brief does not list. (b) sign_for does not call admit_use_for itself. admit_use_for now refuses a signing key as a value use, because its Ticket::credential would carry the seed. sign_for calls the same shared admission, admit_use_as, with UseFor::Signature, so the checks and use lines are identical. (c) The credential-proxy route test writes a hand-set route for the key into routes.json. seal-signing-key adds no route, and without one the fallback answers 404 before admission ever runs. (d) A value use naming a signing key is refused even when an account added under it is current, so the refusal holds whatever account is in use.
- Files changed:
  - created: `crates/lys-secrets/src/broker/signing.rs` — SigningPurpose (closed enum; agent_request with label/domain/parse), AgentRequest typed members with validation, Signable, Signed/Signing, Broker::seal_signing_key (32-byte seed check, purpose and seed sealed together under lys-secrets/signing-key/v1, public key recorded), Broker::sign_for, and gives_value, the one not_a_value_secret check that every value-bearing read calls. Declares signing_tests.rs as its tests module.
  - created: `crates/lys-secrets/src/broker/signing_tests.rs` — R1 library tests: 31-byte seed, unknown purpose, value use refused before admission, OAuth reads, spawn login, sealed read.
  - created: `crates/lys-secrets/src/error/signing.rs` — SigningRefusal with KeyInvalid, PurposeUnknown, PurposeMismatch and NotAValueSecret, named signing_key_invalid, signing_purpose_unknown, signing_purpose_mismatch and not_a_value_secret. No message carries a seed byte.
  - modified: `crates/lys-secrets/src/error.rs` — SecretsError::Signing(#[from] SigningRefusal); declares and re-exports the module.
  - modified: `crates/lys-secrets/src/error/name.rs` — Signing(refusal) takes its name from refusal.name().
  - modified: `crates/lys-secrets/src/store.rs` — EntryClass::SigningKey (label signing_key). EntryView gains purpose and public_key (serde default, skipped when none). The public add refuses the signing-key class; add_signing_key is the only way to seal one.
  - modified: `crates/lys-secrets/src/broker.rs` — Declares the signing module and re-exports its types; issue_capped accepts a SigningKey entry for a handle.
  - modified: `crates/lys-secrets/src/lib.rs` — Re-exports AgentRequest, Signable, Signed, Signing, SigningPurpose and SigningRefusal.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/cli.rs` — New seal-signing-key subcommand taking --name, --owner and --purpose; Seal and RecordClass are unchanged.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/main.rs` — Dispatches SealSigningKey: parses the purpose, reads the seed raw from standard input without trimming, prints the public key and never the seed. Declares signature_route.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/callers.rs` — refused() maps signing_key_invalid and signing_purpose_unknown to 400, signing_purpose_mismatch and not_a_value_secret to 403.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/view.rs` — GET /_lys/secrets entries carry purpose and public_key.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/serve.rs` — Module doc says the proxy refuses a signing key at admission; the signature route is registered (R2).
  - modified: `crates/lys-secrets/src/broker/using.rs` — UseFor {Value, Signature}. admit_use_as is the one admission, and every public admit passes Value. current_for refuses a signing key not_a_value_secret for a Value use, on its use line, before the use is admitted.
  - modified: `crates/lys-secrets/src/broker/oauth_grants.rs` — Ticket::oauth and oauth_grant_of (the OAuth proxy's two value reads) refuse a signing key not_a_value_secret.
  - modified: `crates/lys-secrets/src/broker/spawn.rs` — spawn_login refuses a signing key not_a_value_secret, on its spawn_login audit line.
  - modified: `crates/lys-secrets/src/broker/records.rs` — read_record refuses a signing key not_a_value_secret, on its sealed_read audit line.
- Checklist delivery:
  - [x] C411 — A signing key is stored as a secret with one named purpose; no route that carries a value serves it, and listing shows its public key and never its seed (SECRETS-006 R1). — Stored with one purpose; the proxy, the OAuth reads, spawn and read each refuse it; the listing shows the public key and purpose and never the seed. Tests written, never run.
- Story delivery:
  - [x] S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key. — An agent's handle gets signatures and never the key. Tests not yet run.

### R2: The broker builds what it signs and returns the signature only

Behavioural. POST /_lys/signature is a new route in serve.rs, handled in the new crates/lys-secrets/src/bin/lys-secrets/signature_route.rs. Its name is kept apart from the Sign subcommand (cli.rs line 217, main.rs line 341), which signs a holder's presentation and is unchanged. The request is JSON holding the handle, the purpose and the purpose's typed members. It carries the handle's signed presentation in the headers every handle route reads (caller in callers.rs line 85). For agent_request the members are method, path, body_digest, signed_at_ms and nonce. The member body_digest is the SHA-256 of the request body as 64 lower-case hex characters, and nonce is at least 16 bytes as hex. The request type denies unknown members, so a request carrying a digest to sign or composed bytes fails to parse and is refused 400 before any admission. The route calls one new library function, Broker::sign_for in broker/signing.rs, which does all that follows, so the tests can drive signing through the library Broker as well as over the route. It admits the use through admit_use_for (broker/using.rs line 135) exactly as it admits any other, and settles the Ticket it returns with settle_unmetered (using.rs line 305) once the signature is made, or with settle_failed (using.rs line 316) when signing is refused after admission, so no operation stays open (lineage.rs line 57). The presentation is signed, the handle is live, the lease covers the named key and the caller is permitted. It then checks that the purpose asked is the key's own. It builds the bytes to sign with a new function lys_core::agent_request::payload in the new crates/lys-core/src/agent_request.rs, which takes the five members and holds the domain label. It signs them as a COSE_Sign1 with lys_core::attestation::sign_attestation (crates/lys-core/src/attestation/sign.rs line 76), under an Ed25519Identity made from the seed by from_seed (crates/lys-core/src/keys/identity.rs line 288). The function from_seed becomes public and still takes the seed only wrapped in Zeroizing. Its comment at identity.rs lines 283 to 285, which says no constructor taking private key material is exposed, is rewritten to say that this one constructor is exposed for a holder that keeps the seed sealed, as the broker does. ADR-124 records that the rule is lifted for this constructor only. It answers the COSE_Sign1 bytes from Attestation::to_cose_bytes (attestation/artifact.rs line 78) as hex, and the key's public key as hex. In lys-identity-server, payload (agent_signature.rs lines 51 to 53) becomes a call to lys_core::agent_request::payload with the digest it computes from the body. DOMAIN moves from agent_signature.rs line 35 into agent_request.rs, so the signer and the verifier build the same bytes from one function. The key that signs a presentation stays with the presenter as it does today, and this route does not move it into the broker. Each acceptance line of R2 and R3 is one test function in crates/lys-secrets/tests/signing.rs, using the fixtures of crates/lys-secrets/tests/support. Served in tests/support/served.rs gains seal_signing_key, which runs seal-signing-key with the seed on standard input. It also gains start_logged, which starts the broker with its standard error written to a file in the served root in place of inheriting it (served.rs line 210). The log-line test starts the broker with start_logged and reads that file after its calls.

**Acceptance:**
- A signature the route returns for agent_request verifies with lys_core::attestation::verify_attestation_bytes_by_signer over lys_core::agent_request::payload of the same members, which are the two calls signed_agent makes (agent_signature.rs lines 95 and 99).
- The text `lys-identity/agent-request/v1` appears in exactly one source file under crates, which is crates/lys-core/src/agent_request.rs.
- A call naming a purpose other than the key's own is refused signing_purpose_mismatch naming both purposes.
- A request holding any member outside the purpose's typed members is refused 400 and leaves the handle's use count unchanged.
- An unsigned presentation is refused PresentationUnsigned.
- A lease that does not cover the key is refused OutsideScope (error/bounds.rs line 39).
- A caller that is not permitted is refused PermissionDenied.
- No answer of the route holds the seed's bytes or their hex.
- No log line the broker writes during the test holds the seed's bytes or their hex.

**Files:**
- create: crates/lys-secrets/src/bin/lys-secrets/signature_route.rs
- create: crates/lys-secrets/tests/signing.rs
- create: crates/lys-core/src/agent_request.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/serve.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/main.rs
- modify: crates/lys-core/src/lib.rs
- modify: crates/lys-core/src/keys/identity.rs
- modify: crates/lys-identity-server/src/agent_signature.rs
- modify: crates/lys-secrets/tests/support/served.rs

**Checklist:**
- C412 — The signing route admits the use as any other, builds the bytes to sign from typed members, and returns the signature only; it signs no raw digest (SECRETS-006 R2).

**Stories:**
- S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each acceptance row, met in code; tests written but never run:
(1) The route's signature verifies: tests/signing.rs a_signature_from_the_route_verifies_over_the_payload_of_its_members (line 237). It checks with verify_attestation_bytes_by_signer against the public key made from the test's own seed. The payload comes from lys_identity_server::agent_signature::payload (the verifier's own call) and is asserted equal to lys_core::agent_request::payload. A payload with different members does not verify.
(2) The domain label is in one file: the label lives only in crates/lys-core/src/agent_request.rs, and identity-server's DOMAIN is removed. Test at line 262 walks every .rs file under crates and asserts that file is the only holder; the test builds the label at runtime so it does not contain it.
(3) Purpose mismatch: signing.rs signature() refuses signing_purpose_mismatch naming the purpose held and the purpose asked. Test at line 297, then settle_failed leaves upstream_failed.
(4) An unknown member is refused 400 with the use count unchanged: deny_unknown_fields on both request levels. Test at line 323 uses a digest member and a top-level bytes member, asserts used == 0, then used == 1 after a good call.
(5) PresentationUnsigned: test at line 350, 400.
(6) OutsideScope: test at line 362, a handle on the credential asked for the key, 403.
(7) PermissionDenied: test at line 373, in process, grant revoked after a successful sign.
(8) No route answer holds the seed: test at line 405 searches four answers (200, 400, 403, 400) for the bytes and both hex cases.
(9) No log line holds the seed: test at line 425 starts with start_logged, makes calls including a revoke, then searches the log file.
- Deviation: (a) The request names the signing key in a member `key`, where the brief says 'holding the handle'. The handle token and id already travel in the presentation headers, and admission needs the key's name to give OutsideScope. (b) With one purpose in the closed list, no signing key can have a purpose other than the one asked. The mismatch test therefore uses a handle on a credential, whose own purpose is 'none'; the refusal names both 'none' and 'agent_request'. (c) Added, not in the brief: sign_for refuses a signing time more than PRESENTATION_SKEW_MS from the broker's clock, as PresentationStale, before admission. Without it a holder could obtain signatures for requests dated ahead, which the identity server would still accept after the lease was revoked or its window closed. Test at line 385. The served tests therefore sign for the wall clock's time. (d) seal_signing_key and start_logged live in the new tests/support/keyed.rs, not tests/support/served.rs. Every item of a support file must be used by every test crate that includes it, or clippy -D warnings refuses it as dead code. screen_routes.rs uses served.rs whole and would not use these two, and signing.rs cannot use served.rs's screen-route items. (e) The member validation (method and path free of spaces and control characters, lower-case 64-hex body digest, nonce of at least 16 bytes) stops a member from forging a line of the payload. (f) Added a PresentationStale control test for (c).
- Files changed:
  - created: `crates/lys-secrets/src/bin/lys-secrets/signature_route.rs` — POST /_lys/signature. The JSON body {key, purpose, members} denies unknown members at both levels. The purpose is parsed and members are read per purpose into AgentRequest before the presentation is read or anything is admitted. Calls Broker::sign_for and answers {signature (COSE_Sign1 hex), public_key (hex), uses_left}. A retry answers 409.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/serve.rs` — Registers /_lys/signature.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/main.rs` — mod signature_route.
  - created: `crates/lys-core/src/agent_request.rs` — DOMAIN and payload(method, path, body_digest, signed_at_ms, nonce), byte-identical to the old identity-server layout. The module doc states the one-function invariant and that the format is frozen.
  - modified: `crates/lys-core/src/lib.rs` — pub mod agent_request.
  - modified: `crates/lys-core/src/keys/identity.rs` — from_seed is public and still takes Zeroizing<[u8; 32]>. Its comment now says this one constructor is exposed for a holder that keeps the seed sealed, as the broker does, citing ADR-124.
  - modified: `crates/lys-core/tests/seal_derivation.rs` — Comment only: it no longer says from_seed is private.
  - modified: `crates/lys-identity-server/src/agent_signature.rs` — DOMAIN removed; payload computes the body digest and calls lys_core::agent_request::payload.
  - created: `crates/lys-secrets/tests/signing.rs` — One test function per acceptance line of R2 and R3, plus the R1 route tests and one control for the signing-time bound.
  - created: `crates/lys-secrets/tests/support/keyed.rs` — Served-binary fixture. Keyed::new seeds a broker with the product's own commands; seal_signing_key runs seal-signing-key with the seed on standard input. Served::start and Served::start_logged (standard error written to a file in the served root; Keyed::log reads it back). held, sign and ask send holder-signed or service-signed requests; audit_bytes reads the audit log's files whole.
  - modified: `crates/lys-secrets/tests/support/world.rs` — paths() is public. broker() now calls a new broker_on(grants, clock), so a test can move the clock by hand.
  - created: `crates/lys-secrets/src/broker/signing.rs` — sign_for: signing-time bound, the shared admission, purpose check, sealed binding checks (domain, purpose, 32-byte seed, public key), sign_attestation under from_seed, then settle_unmetered, or settle_failed on a refusal after admission.
- Checklist delivery:
  - [x] C412 — The signing route admits the use as any other, builds the bytes to sign from typed members, and returns the signature only; it signs no raw digest (SECRETS-006 R2). — Same admission; bytes built from typed members by the shared function; the answer holds the signature and public key only; unknown members refused 400. Tests not yet run.
- Story delivery:
  - [x] S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key. — The broker signs agent requests with the key it holds. Tests not yet run.

### R3: Every signature is on the audit record

Behavioural. A signing use is audited as every use is, by the Use lines admit_use_for writes itself (broker/using.rs lines 208, 231 and 241). No new AuditKind is added, so the replay of admitted Use lines (broker/folded.rs line 149) counts a signing use against the lease's max_uses after a reopen, and keeps its operation id. AuditLine (audit.rs lines 95 to 119) and its rule that no field is a raw handle, a digest, a credential or a key byte (audit.rs lines 92 to 93) are unchanged. A refused signing use leaves the refusal line admit_use_for writes, with its refusal's name as the outcome. A revoke through POST /_lys/leases/{lease_id}/revoke (serve.rs line 111) ends the lease with a drop line that marks its handle dropped (broker/ending.rs line 219), so the next signing use is refused HandleDropped, as a drop through POST /_lys/drop is. A lease whose window has closed refuses the next signing use LeaseWindowClosed (admit.rs line 112). That line is driven through Broker::sign_for on the Leases fixture of tests/support/leases.rs, whose clock the test moves past the window (leases.rs lines 89 and 136), with no sleep.

**Acceptance:**
- One admitted signing use appears once at GET /_lys/audit as a line of kind use whose outcome is admitted.
- One refused signing use appears once at GET /_lys/audit as a line of kind use whose outcome names its refusal.
- After a reopen of the broker, a lease of max_uses 2 that has signed twice refuses a third signing use LeaseExhausted.
- After the lease is revoked through POST /_lys/leases/{lease_id}/revoke the next signing call is refused HandleDropped.
- After the lease is revoked the next signing call answers no signature.
- After the Leases fixture's clock passes the lease's window, the next Broker::sign_for is refused LeaseWindowClosed.
- After the handle is dropped through POST /_lys/drop the next signing call is refused HandleDropped.
- No audit line holds the key's public key in hex.
- No audit line holds the hex SHA-256 of the bytes signed.
- No audit line holds the seed's bytes or their hex.

**Files:**
- modify: crates/lys-secrets/src/broker/using.rs
- modify: crates/lys-secrets/src/broker/admit.rs
- modify: crates/lys-secrets/tests/support/leases.rs

**Checklist:**
- C413 — Every signing use and every refused one is on the audit record, and a revoked handle signs nothing (SECRETS-006 R3).

**Stories:**
- S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each acceptance row, met in code; tests written but never run:
(1) One admitted use line: test at line 445 counts kind use, outcome admitted: 0 before, 1 after one signature, read at GET /_lys/audit.
(2) One refused use line: test at line 456 counts kind use, outcome PresentationUnsigned: exactly 1, and no admitted line.
(3) LeaseExhausted after a reopen: test at line 470, in process, a lease of max_uses 2 signs twice, Broker::open over the same paths, and the third sign is refused. Admitted Use lines are replayed by folded.rs.
(4) HandleDropped after revoke: test at line 485, POST /_lys/leases/{id}/revoke as the owner through the trusted service, 403.
(5) No signature after revoke: test at line 496, 200 with a signature before, non-200 non-JSON without one after.
(6) LeaseWindowClosed: test at line 512, Broker::sign_for on a World broker whose AtomicI64 clock the test moves past the window, no sleep.
(7) HandleDropped after POST /_lys/drop: test at line 528.
(8) to (10) Nothing in the audit log: tests at lines 563, 575 and 588 read every file of the audit-log folder whole, after a sign, a refusal and a revoke, and search for the public-key hex, the hex SHA-256 of the exact payload signed (at its recorded signing time), and the seed bytes and hex, each in both cases. The setup first asserts the log names the key, so the search is over lines that exist.
- Deviation: (a) The window test drives Broker::sign_for through the World fixture (tests/support/world.rs, with a new broker_on taking a clock the test moves), not the Leases fixture in tests/support/leases.rs. Including leases.rs obliges signing.rs to use every item of it and of fixture.rs. A sign method added there would be dead code in lease_revoke.rs and lease_relinquish.rs under clippy -D warnings. So leases.rs is unmodified. (b) crates/lys-secrets/src/broker/admit.rs is unmodified: admission needed no change. (c) crates/lys-secrets/src/broker/scope.rs, not in the brief, was changed so '<secret>@primary' use lines are discoverable. Without it the admitted-line acceptance row cannot be met at the route, for signing and for every other use.
- Files changed:
  - modified: `crates/lys-secrets/src/broker/using.rs` — The shared admission, with the use named Value or Signature, writes the same Use lines for a signing use. No new AuditKind; AuditLine is unchanged.
  - modified: `crates/lys-secrets/src/broker/scope.rs` — discovers() reads '<secret>@primary', the name an admitted Use line gives a secret's first account, as the secret itself (only when that name is not itself sealed). Without this, no admitted use line was ever visible at GET /_lys/audit.
  - created: `crates/lys-secrets/tests/signing.rs` — R3 tests, served and in process.
  - modified: `crates/lys-secrets/tests/support/world.rs` — broker_on(grants, clock) and a public paths() for the window and reopen tests.
- Checklist delivery:
  - [x] C413 — Every signing use and every refused one is on the audit record, and a revoked handle signs nothing (SECRETS-006 R3). — Signing uses and refusals are on the audit record through the shared admission; revoke and drop stop signing; the audit log holds no key material. Tests not yet run.
- Story delivery:
  - [x] S165 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent, I want the broker to sign my requests with the key it holds for me, so that I can prove who I am without ever holding the key. — Every signature is audited and ends with its lease. Tests not yet run.

## Boundaries

- SHALL NOT return, log or write a signing key's seed on any path, or accept a request that would make the broker return it.
- SHALL NOT sign a digest or bytes a caller composed. The broker builds what it signs from typed members under a purpose's domain label.
- SHALL NOT add a second admission path. A signing use is admitted, leased, audited and revoked as every other use.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass.
- SHALL NOT add a silent fallback. Every failure is a named refusal.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- `cargo test -p lys-secrets --test signing` exits 0 on Dean's laptop at the card's head.
- `cargo test -p lys-secrets --lib broker::signing` exits 0 on Dean's laptop at the card's head.
