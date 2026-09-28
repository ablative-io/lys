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
