# DIRECTORY-003 file manifest

Every new file in the three new directories, with its one responsibility. A file not listed here is not written in this row; needing one stops the row and names it for a brief revision. Each file stays under 500 lines.

## crates/lys-identity (library: directory records, typed API, event projection)

| File | Responsibility |
| --- | --- |
| Cargo.toml | Package manifest. Depends on lys-core (verification only, unchanged), lys-log-store, serde, ciborium, sha2, ed25519-dalek, rand, thiserror, zeroize, time. |
| src/lib.rs | Declarations and re-exports only. |
| src/id.rs | PersonId and AgentId, enduring identifiers generated from the secure random source, with their text form and parse. Never derived from an email or display name (P1). |
| src/binding.rs | The external login binding, issuer plus subject, its normalisation and the rule that one binding names at most one person (P1). |
| src/profile.rs | The display profile and the validation of a profile change. A profile field never establishes identity (P1). |
| src/provenance.rs | The actor of a change: the authenticated human's binding and authentication provenance as the service attests it, and nothing claiming a person signed bytes (P8). |
| src/operation.rs | The operation id and retry rule: the same id with the same request returns the first receipt, and the same id with a different request is refused by name. |
| src/lifecycle.rs | The four states, the transition table of R5, and refusal by name of any transition outside it. Records, never enforces (CN11). |
| src/event.rs | The versioned event envelope and its typed payloads: register person, register agent under its responsible person, change profile, bind login, lifecycle transition, link-audit accepted. |
| src/encoding.rs | Canonical encoding of an event and its commitment, named as SHA-256 and never a BLAKE3 content address (R2). |
| src/signer.rs | The service's event signing key, held in Zeroizing memory, loaded from a key file the operator supplies, and the service attestation over each event. |
| src/log.rs | Commit of one signed event through lys-log-store as one leaf, and reconciliation of an uncertain append by reading the leaf back before any affected read answers (P4, P5). |
| src/projection.rs | The directory state rebuilt from the events at open, and advanced by each committed event. |
| src/directory.rs | The typed API over log and projection: register a person, register an agent, change a profile, bind a login, transition a state, read an identity. |
| src/receipt.rs | The receipt with the fields of P7, and the read-only inclusion verification of a receipt against the log. |
| src/link_audit.rs | The link-audit receiver's core (R4): deduplicate by source operation id, keep issuer observations apart from human-signed claims, and return a verifiable receipt. |
| src/error.rs | Typed errors naming the operation, identity and refusal, never a key or token byte. |

## crates/lys-identity-server (OIDC session handling, administrator admission)

| File | Responsibility |
| --- | --- |
| Cargo.toml | Package manifest. New workspace dependencies, each pinned: axum 0.8.9 and tokio 1.53.1 for the HTTP service, and openidconnect 4.0.1 with default features off and only reqwest and rustls-tls on, for token validation against the configured issuer. TLS is rustls, never native-tls. |
| src/main.rs | Binary entry: read the configuration, open the directory, serve. |
| src/lib.rs | Declarations and re-exports only. |
| src/config.rs | Typed configuration and its validation: listen address, log directory, event key file, OIDC issuer, and the administrator's configured issuer and subject (P9). No secret value in any diagnostic. |
| src/oidc.rs | Sign-in against the configured issuer, and validation of the returned token to an issuer and subject. |
| src/session.rs | The signed-in session, its cookie and its expiry. |
| src/admission.rs | Admission of the configured administrator for every mutation, fail closed for every other caller by name, and the step-1 authority statement shown to the caller (R3). |
| src/routes.rs | The HTTP routes mapping requests to the directory API, each mutation behind admission. |
| src/receipts_api.rs | The read-only receipt and inclusion-verification endpoints (R2). |
| src/link_audit_api.rs | The authenticated link-audit receiver endpoint over the core in lys-identity (R4). |
| src/error.rs | The mapping of typed errors to HTTP answers, each refusal by name. |

## tests/identity_contract (the contract tests, a workspace test crate)

| File | Responsibility |
| --- | --- |
| Cargo.toml | Test crate manifest, publish false, depending on both new crates. |
| src/lib.rs | Declarations only. |
| src/harness.rs | A directory on a temporary log, a service key, and a started server on a local port. |
| src/fake_issuer.rs | An in-process OIDC issuer that signs tokens for chosen issuer and subject pairs, so no live Rauthy is needed. |
| src/fixtures.rs | The reviewed typed link-audit contract fixtures (R4). |
| tests/registration.rs | R1: people and agents registered with enduring ids, each agent under its responsible person, and no running state or credential issued. |
| tests/events.rs | R2: one signed event per change, projection rebuilt at open equals the live one, an uncertain append reconciled before a read, and receipts verified. |
| tests/admission.rs | R3: the configured administrator admitted, every other caller refused by name, email and first visit never admitted. |
| tests/link_audit.rs | R4: duplicate source operation ids answered once, observations kept apart from claims, and actor provenance surviving replay. |
| tests/lifecycle.rs | R5: every allowed transition recorded as one signed event, every other refused by name with nothing recorded, and a retired identity never reactivated. |

## Outside the three directories

Cargo.toml and Cargo.lock gain the two crates, the test crate and the new dependencies. docs/design/identity/DIRECTORY-CONTRACT.md and docs/design/identity/IDENTITY-EVENTS.md are written as R1, R2 and R5 say. The deploy files under deploy/identity gain the directory service as R1 says, and they are not changed until DIRECTORY-002 has written them.

## Settled by the reviewer

Archie approved this manifest as lead. The three new workspace dependencies are pinned as listed above, and TLS is rustls, never native-tls. Token checks go through openidconnect, never by hand. tests/identity_contract is a workspace test crate with the in-process fake issuer. The event envelope in event.rs and encoding.rs signs durable bytes, so those two files are committed on their own first, for the joint review R2 asks for, before the rest of the row is built on them.
