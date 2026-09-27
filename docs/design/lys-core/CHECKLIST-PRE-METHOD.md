# Lys-Core — Checklist

> **Reconciled against the shipped code, 2026-07-30.** [ROADMAP.md](../../ROADMAP.md) marks Phase 1 (extract to `lys-core`) and Phase 2 (the `lys` CLI) **DONE**, and the boxes below now record that: an item is ticked only where it was verified against this repository's own bytes.
>
> Four are deliberately left open. **C61–C63** are the build gates — verified by CI on every change, not by reading source. **C64** is not met: tests live inline rather than in sibling `*_tests.rs` files in `merkle/leaf.rs`, both `seal/` files, `error.rs`, and six CLI modules (see [REVIEW-23-07.md](../../REVIEW-23-07.md) F12).
>
> Items amended by a ratified wire-format decision carry an annotation above them and are restated as-built; the decisions themselves live in [WIRE-FORMATS.md](../WIRE-FORMATS.md), which is authoritative wherever this file disagrees. This is the phase-1/2 acceptance list only — the `checkpoint` and `tlog` layers ratified afterwards (WIRE-FORMATS D1 and D2) are specified in [DESIGN.md](DESIGN.md) D4 and pinned by their own test suites, not by items here.

## Crate Setup

- [x] **C1** — Root Cargo.toml declares workspace members `crates/lys-core` and `crates/lys`
- [x] **C2** — lys-core Cargo.toml declares ed25519-dalek, x25519-dalek, aes-gcm, rcgen, ct-merkle, x509-parser, sha2, hkdf, rand, serde, thiserror, zeroize, base64, chrono, tracing dependencies — plus ciborium (COSE), postcard (typed leaf encoding), and time
- [x] **C3** — lys-core lib.rs declares public modules: keys, ca, merkle, checkpoint, tlog, attestation, seal, error — and the crate-internal `hex_lower` helper
- [x] **C4** — lib.rs carries `#![cfg_attr(not(test), forbid(unsafe_code))]`, over the workspace-wide `unsafe_code = "deny"`; `forbid` relaxes to `deny` in test builds only so the env-backed tests can call `std::env::set_var` (unsafe in edition 2024) under an explicit `#[allow]`
- [x] **C5** — TrustError enum defined with thiserror: CertificateGeneration, CertificateParsing, CertificateVerification, CertificateRevocation, MerkleTree, Seal, UnsealFailed, AttestationFailed, KeyManagement, Signing, InvalidSignature — plus CheckpointEncoding, CheckpointParsing, VerifierKey, NoteVerification, LogArtifactEncoding, LogArtifactVerification — with `TrustResult<T>` alias
- [x] **C6** — No Meridian reference anywhere in lys-core or lys sources: `grep -ri meridian crates/` returns nothing

## Key Management

- [x] **C7** — Ed25519Identity struct holds SigningKey + VerifyingKey; Debug output contains '[REDACTED]' for the signing key (test exists)
- [x] **C8** — Ed25519Identity::load_or_generate(path) loads a 32-byte seed file or generates and persists one; malformed-length files return KeyManagement errors
- [x] **C9** — Key generation is race-free: seed written to a unique temp file (pid + per-process counter in the name), published via no-clobber `hard_link`; on `AlreadyExists` the loser discards its candidate seed and loads the persisted key (first-writer-wins; concurrent-generation test exists)
- [x] **C10** — On Unix the key file is created mode 0o600; loading a key file with loose permissions emits a warning but still loads
- [x] **C11** — Ed25519Identity::from_env() reads a base64-encoded 32-byte seed from the `LYS_IDENTITY_KEY` environment variable; missing variable, invalid base64, and wrong decoded length each return KeyManagement errors
- [x] **C12** — All seed material in loading, generation, and decode paths is held in `Zeroizing` buffers
- [x] **C13** — Ed25519Identity::sign(message) returns [u8; 64]
- [x] **C14** — Ed25519Identity::verify(public_key, message, signature) uses ed25519-dalek `verify_strict`; no non-strict verification exists in library code anywhere in either crate — the only `verify` calls are inside tests that construct a small-order forgery, prove dalek's non-strict path accepts it, and prove lys rejects it
- [x] **C15** — Ed25519Identity::x25519_public_key() returns the Montgomery-form [u8; 32] and x25519_static_secret() returns the clamped-scalar StaticSecret; Diffie-Hellman between two identities' derived keys agrees from both sides (test exists)

## Certificate Authority

- [x] **C16** — CertificateAuthority::new(identity) wraps Ed25519Identity and exposes public_key_bytes()
- [x] **C17** — issue_certificate(subject, ttl, extensions) returns IssuedCertificate holding DER bytes, subject keypair, SHA-256 fingerprint ([u8; 32] of DER), expiry, and issuer public key
- [x] **C18** — rcgen signing goes through a RemoteKeyPair adapter with PKCS_ED25519 so the CA private seed is never serialised into rcgen's keypair representation
- [x] **C19** — IssuedCertificate Debug output redacts private key material (test exists)
- [x] **C20** — verify_certificate_chain(cert_der, issuer_public_key) extracts TBS bytes with x509-parser and verifies the signature with ed25519-dalek `verify_strict`
- [x] **C21** — Chain verification enforces the validity window: expired certificates and not-yet-valid certificates are both rejected (tests exist for each)
- [x] **C22** — verify_certificate_chain_at(cert_der, issuer_public_key, instant) verifies at an explicit instant; a cert expired now but valid at the given instant passes
- [x] **C23** — Self-signed certificates are rejected by verify_certificate_chain
- [x] **C24** — `LYS_OID_ARC` constant equals [1, 3, 6, 1, 4, 1, 66364] with a doc comment stating that 66364 is the IANA Private Enterprise Number assigned to lys and that the arc is permanent
- [x] **C25** — encode_extension / decode_extension round-trip an arbitrary DER payload under LYS_OID_ARC; decode of a cert without the extension returns Ok(None)
- [x] **C26** — Round-trip test: rcgen-generated Ed25519 keypair is loadable as ed25519-dalek SigningKey/VerifyingKey

## Merkle Transparency Log

- [x] **C27** — AppendOnlyTree<L> generic over leaf type L: Serialize; append(leaf) returns the new tree size
- [x] **C28** — No delete or modify operation exists on the tree — append-only enforced by API
- [x] **C29** — root() returns the current RootHash; the empty tree produces a deterministic empty root hash
- [x] **C30** — prove_inclusion(leaf_index) pre-checks bounds and returns TrustError::MerkleTree on out-of-range index — no panic path into the backing library
- [x] **C31** — prove_consistency(old_size, new_size) pre-checks the size pair (old ≤ new, new ≤ len, old ≥ 1) and returns TrustError::MerkleTree on violation
- [x] **C32** — verify_inclusion(root_hash, leaf, index, proof) and verify_consistency(old_root, new_root, proof) return Result; tampered proofs and mismatched roots fail
- [x] **C33** — RootHash::from_parts(root_hash, num_leaves) and to_parts() round-trip; from_parts requires no tree access
- [x] **C34** — InclusionProof and ConsistencyProof round-trip through as_bytes() / try_from_bytes()
- [x] **C35** — External-verifier round-trip test exists: a verifier holding only published root parts and proof bytes (never the tree) verifies inclusion and consistency
- [x] **C36** — reconstruct_from_leaves(leaves) rebuilds a tree with a root hash identical to the original (test exists)
- [x] **C37** — merkle module docs state the frozen-wire-contract rule: leaf encodings are canonical bytes, evolved only by introducing a new versioned leaf type

## Signed Attestations

*(C38–C43 amended by WIRE-FORMATS.md decision D4: the attestation artifact is the `lys/attestation/v2` tagged COSE_Sign1; the v1 JSON/preimage form was deleted unshipped.)*

- [x] **C38** — sign_attestation(payload, signing_key) signs the COSE `Sig_structure` `["Signature1", protected, h'', claims]` (RFC 9052 §4.4) with protected `{1: -8, 3: "application/vnd.lys.attestation.v2+cbor", 4: signer key}` — no meridian string, no v1 preimage constant remains anywhere
- [x] **C39** — Attestation { payload_hash: [u8; 32], signature: [u8; 64], signer_public_key: [u8; 32], timestamp: i64 } carries no serde; the only durable form is `to_cose_bytes()` / `from_cose_bytes()` (canonical-encoding-strict)
- [x] **C40** — verify_attestation(attestation, payload) rebuilds the `Sig_structure` from the attestation's own fields and verifies with `verify_strict`
- [x] **C41** — No legacy fallback exists: a signature over the bare payload hash and a signature over the deleted v1 preimage both fail verify_attestation (tests exist)
- [x] **C42** — Tampered payload fails verify_attestation
- [x] **C43** — Tampered timestamp fails verify_attestation — the timestamp is a signed claim inside the `Sig_structure` (test exists)

## Sealed Envelope

- [x] **C44** — seal(payload, recipient_public_key) returns SealedEnvelope { ephemeral_public_key, ciphertext, nonce } using a fresh ephemeral X25519 keypair per call (two seals of the same payload to the same recipient differ)
- [x] **C45** — HKDF-SHA256 info input is `b"lys-sealed-envelope/v1" || ephemeral_public_key || recipient_public_key` — the hyphen-form HKDF domain tag, deliberately distinct from the slash-form attestation context tag `lys/sealed-envelope/v1`
- [x] **C46** — Both seal and open reject non-contributory Diffie-Hellman: a low-order public key fails via `was_contributory` before any key derivation (test exists)
- [x] **C47** — Seal/open roundtrip succeeds: sealed with the recipient's X25519 public key, opened with the recipient's static secret
- [x] **C48** — Wrong private key, tampered ciphertext, and tampered nonce all return exactly TrustError::UnsealFailed — a single undifferentiated failure through the AES-GCM arbiter, with no early return distinguishing causes
- [x] **C49** — SealedEnvelope::attestation_bytes() covers every wire byte of the envelope (ephemeral key, nonce, ciphertext)
- [x] **C50** — sign_and_seal(payload, sender_identity, recipient_x25519_public_key) returns (SealedEnvelope, Attestation) where the attestation signs attestation_bytes()
- [x] **C51** — open_and_verify verifies the attestation before any decryption: an invalid sender signature is rejected without the cipher being touched, and a valid signature over a tampered envelope also fails (tests exist)

## CLI Surface

- [x] **C52** — `lys` binary crate exists; main.rs is a thin entry (parse, dispatch, exit codes) with clap definitions isolated in cli.rs; no `anyhow` anywhere — the CLI carries its own thiserror type
- [x] **C53** — `lys key` generates an identity at a path and inspects one (public key, fingerprint); no subcommand, flag, or output format prints private key material (test asserts output contains no seed bytes in any encoding)
- [x] **C54** — `lys ca issue` issues a certificate signed by an issuer identity file, embedding a caller-supplied capability-claim payload as a LYS_OID_ARC extension, and writes the PEM out
- [x] **C55** — `lys ca verify` verifies a certificate against an issuer public key, and accepts an explicit verification instant flag routing to verify_certificate_chain_at
- [x] **C56** — `lys attest` signs a payload file and emits the COSE_Sign1 artifact; `lys verify` checks an artifact against a payload and reports success/failure via exit code. (File paths only as built — the stdin path this item originally anticipated was not implemented, and ROADMAP Phase 2 records the file-only surface.)
- [x] **C57** — `lys seal` seals a payload file for a recipient public key and writes the sender attestation alongside it; `lys open` opens it with the recipient identity, verifying the attestation first; the pair round-trips

*(C58–C59 superseded by WIRE-FORMATS.md decisions D1 and D2: `lys log` emits C2SP signed-note checkpoints and self-contained JSON proof artifacts, and third-party verification takes the artifact, the leaf, and the verifier key rather than raw root parts plus proof bytes. Restated as-built.)*

- [x] **C58** — `lys log init` pins the log's origin exactly once and refuses to re-initialize; `lys log append` appends a leaf file's raw bytes and prints the new root; `lys log checkpoint` signs a C2SP tlog-checkpoint in the signed-note envelope over the current root; `lys log prove` emits a self-contained JSON proof artifact with the relevant signed checkpoint(s) embedded verbatim
- [x] **C59** — `lys log verify` verifies an inclusion or consistency claim from **only** the artifact, the leaf, and the verifier key — no access to the leaf sequence, the store, or the tree; declared sizes are checked against the signature-verified checkpoint and roots are recomputed, never trusted; every tamper class collapses to one identical message
- [x] **C60** — Cross-process CLI test exists: a log produced by one process is verified end-to-end by the CLI in another process with no access to the original tree

## Integration Verification

- [ ] **C61** — cargo fmt --check passes clean *(gate — verified by CI, not from source)*
- [ ] **C62** — cargo clippy --all-targets -- -D warnings passes clean *(gate — verified by CI, not from source)*
- [ ] **C63** — cargo test --workspace passes green *(gate — verified by CI, not from source)*
- [ ] **C64** — No file exceeds 500 lines of code; every mod.rs carries only pub mod / pub use / module docs; tests live in sibling *_tests.rs files *(not met: `merkle/leaf.rs`, both `seal/` files, `error.rs`, and six CLI modules carry inline `mod tests` — REVIEW-23-07.md F12)*
- [x] **C65** — lys-core builds standalone with zero meridian-* dependencies in its Cargo.toml and Cargo.lock
