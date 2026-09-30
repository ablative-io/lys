# Lys-Core — Checklist

## Certificate Authority

- [ ] **C66** — verify_certificate_chain_at compares no subject or issuer distinguished-name bytes: crates/lys-core/src/ca/authority.rs calls neither .subject() nor .issuer() on a parsed certificate.
- [ ] **C67** — A certificate issued by CertificateAuthority::issue_certificate to the subject named by the authority's lowercase-hex public key verifies under the authority's key and is refused under any other key.
- [ ] **C68** — A certificate issued by issue_certificate_for_request over a request made with the authority's own key is refused by verify_certificate_chain as self-signed.
- [ ] **C69** — The rustdoc of verify_certificate_chain states what it checks and what it leaves to the caller.
- [ ] **C70** — The rustdoc of verify_certificate_chain_at and CertificateAuthority::verify_certificate_chain states the key-based self-signed rule and mentions no distinguished-name heuristic.
- [x] **C24** — `LYS_OID_ARC` constant equals [1, 3, 6, 1, 4, 1, 66364] with a doc comment stating that 66364 is the IANA Private Enterprise Number assigned to lys and that the arc is permanent
- [x] **C25** — encode_extension / decode_extension round-trip an arbitrary DER payload under LYS_OID_ARC; decode of a cert without the extension returns Ok(None)
- [x] **C26** — Round-trip test: rcgen-generated Ed25519 keypair is loadable as ed25519-dalek SigningKey/VerifyingKey

## CLI Surface

- [ ] **C71** — lys ca verify and lys verify --cert exit 0 for a certificate issued to the subject named by the issuer's lowercase-hex public key, and exit 1 for a certificate self-signed by keys.
- [x] **C52** — `lys` binary crate exists; main.rs is a thin entry (parse, dispatch, exit codes) with clap definitions isolated in cli.rs; no `anyhow` anywhere — the CLI carries its own thiserror type
- [x] **C53** — `lys key` generates an identity at a path and inspects one (public key, fingerprint); no subcommand, flag, or output format prints private key material (test asserts output contains no seed bytes in any encoding)
- [x] **C54** — `lys ca issue` issues a certificate signed by an issuer identity file, embedding a caller-supplied capability-claim payload as a LYS_OID_ARC extension, and writes the PEM out
- [x] **C55** — `lys ca verify` verifies a certificate against an issuer public key, and accepts an explicit verification instant flag routing to verify_certificate_chain_at
- [x] **C56** — `lys attest` signs a payload file and emits the COSE_Sign1 artifact; `lys verify` checks an artifact against a payload and reports success/failure via exit code. (File paths only as built — the stdin path this item originally anticipated was not implemented, and ROADMAP Phase 2 records the file-only surface.)
- [x] **C57** — `lys seal` seals a payload file for a recipient public key and writes the sender attestation alongside it; `lys open` opens it with the recipient identity, verifying the attestation first; the pair round-trips
- [x] **C58** — `lys log init` pins the log's origin exactly once and refuses to re-initialize; `lys log append` appends a leaf file's raw bytes and prints the new root; `lys log checkpoint` signs a C2SP tlog-checkpoint in the signed-note envelope over the current root; `lys log prove` emits a self-contained JSON proof artifact with the relevant signed checkpoint(s) embedded verbatim
- [x] **C59** — `lys log verify` verifies an inclusion or consistency claim from **only** the artifact, the leaf, and the verifier key — no access to the leaf sequence, the store, or the tree; declared sizes are checked against the signature-verified checkpoint and roots are recomputed, never trusted; every tamper class collapses to one identical message
- [x] **C60** — Cross-process CLI test exists: a log produced by one process is verified end-to-end by the CLI in another process with no access to the original tree

## Consumers and Records

- [ ] **C72** — lys-anchor's admission module doc says verify_certificate_chain rejects certificates whose subject key is the issuer key.
- [ ] **C73** — CHANGELOG.md's Unreleased section records that a hex-common-name certificate now verifies, that a certificate self-signed by keys is now refused, and the new rustdoc caveat on verify_certificate_chain.
- [ ] **C74** — DIRECTORY-013's fold issues a legitimately issued hex-common-name leaf and records certificate_chain_invalid for a leaf self-signed by keys.

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

## Integration Verification

- [ ] **C61** — cargo fmt --check passes clean *(gate — verified by CI, not from source)*
- [ ] **C62** — cargo clippy --all-targets -- -D warnings passes clean *(gate — verified by CI, not from source)*
- [ ] **C63** — cargo test --workspace passes green *(gate — verified by CI, not from source)*
- [ ] **C64** — No file exceeds 500 lines of code; every mod.rs carries only pub mod / pub use / module docs; tests live in sibling *_tests.rs files *(not met: `merkle/leaf.rs`, both `seal/` files, `error.rs`, and six CLI modules carry inline `mod tests` — REVIEW-23-07.md F12)*
- [x] **C65** — lys-core builds standalone with zero meridian-* dependencies in its Cargo.toml and Cargo.lock

## The underscore-binding rule

- [ ] **C75** — rules/ast-grep/no-underscore-binding.yml holds one rule, id no-underscore-binding, at severity error with vendor/** ignored, whose message and note say it refuses because a leading underscore silences the unused warning instead of fixing its cause.
- [ ] **C76** — On an uncommitted scratch file with one underscore-prefixed binding in each of the 19 binding positions, the scan reports exactly 19 no-underscore-binding hits, and none on a bare `_`, a struct field declaration, a field initialiser or a field access.
- [ ] **C77** — `ast-grep scan --config sgconfig.yml` reports zero hits at the repository root of the landed tree.

## Bindings cleared by what they do

- [ ] **C78** — Every guard whose directory or state the test still uses after the binding (a TempDir holder or an EnvCleanup guard) has a name without a leading underscore and is ended by drop(name) after its last use, with a tail result bound, the guard dropped and the binding returned.
- [ ] **C84** — Each of the eight test fixture structs that holds a TempDir guard names the field temp_dir and has a close method returning TempDir::close's error, and every test that builds one calls its close after the value's last use.
- [ ] **C79** — Every discarded error in a closure or match arm is left unbound, map_err is rewritten as .ok().ok_or(…) when the error is already built and .ok().ok_or_else(…) when building it calls a function or a macro, and each site returns the same error as before.
- [ ] **C80** — Every value with no use is not bound: a bare `_` inside its pattern, including a TempDir a fixture returns beside a value it has already loaded into memory, an expression statement for a whole let, and no `let _ =` statement introduced.
- [ ] **C81** — Every trait implementation that ignores a parameter takes it as a bare `_`, and no trait declaration changes.
- [ ] **C82** — The two cfg(not(unix)) stubs, fsync_dir and warn_if_loose_permissions, take their parameter as a bare `_`.

## Behaviour held

- [ ] **C83** — cargo test --workspace --all-features exits 0 at the commit the build starts from and at its final commit, with the same number of tests passed at both.

## Cluster documents

- [ ] **C1** — DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md under docs/design/lys-core are byte-identical to DESIGN.md, CHECKLIST.md and USER-STORIES.md at 7b53625.
- [ ] **C2** — DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md under docs/design/lys-core are what render-cluster.py renders, and sh scripts/design/gate.sh exits 0.

## Rule set

- [ ] **C3** — sgconfig.yml at the repository root names rules/ast-grep as its only rule directory.
- [ ] **C4** — rules/ast-grep carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes with the rule bodies Cambium carries at 1be80d8ec.
- [ ] **C5** — rules/ast-grep carries the error-severity rule no-unwrap-expect-panic-outside-tests, which reports unwrap, expect and panic outside test code recognised by structure.
- [ ] **C6** — Every rule in rules/ast-grep ignores vendor/**.
- [ ] **C7** — The brief records no-std-mutex-in-async, no-timer-in-door-handlers, no-timer-import-in-door-handlers and the _name rule as not carried, each with its finding.

## Test code by structure

- [ ] **C8** — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true.
- [ ] **C9** — Both fixture.rs files and the 88 sibling *_tests.rs files under src/ begin with #![cfg(test)] and carry no #![allow].
- [ ] **C10** — The 28 integration test roots and both tests/harness/mod.rs files begin with #![cfg(test)] and carry no #![allow].
- [ ] **C11** — The env-backed tests of Ed25519Identity::from_env call no set_var, remove_var or unsafe code, and from_env keeps its signature and its error texts.
- [ ] **C12** — The comment above #![cfg_attr(not(test), forbid(unsafe_code))] in lys-core's lib.rs says what is true after the env tests change, and the attribute is byte-identical.
- [ ] **C13** — grep finds zero #[allow], #![allow], #[expect] and #[ignore] under crates/.

## Hits fixed at their cause

- [ ] **C14** — The tests/harness/mod.rs files of lys-core and lys-anchor and lys-home's harness/claude_code/mod.rs hold only module declarations and re-exports.
- [ ] **C15** — The `let _ =` hits in the three hex writers, the 14 test-code writes, identity_tests.rs and json_output_tests.rs are gone.

## Documents naming the policy

- [ ] **C16** — CLAUDE.md's coding-standards sentence and the Cargo.toml lint comment name clippy.toml's allow-*-in-tests in place of the per-module #![allow].
- [ ] **C17** — CLAUDE.md's gates block lists `ast-grep scan --config sgconfig.yml` and its count sentence matches the lines it lists.

## The gate leg

- [ ] **C18** — docs/design/project.json has a leg running `ast-grep scan --config sgconfig.yml` requiring tool:ast-grep, and validate.py accepts the file.
- [ ] **C19** — .land/gates.sh runs `leg ast-grep scan --config sgconfig.yml`.
- [ ] **C20** — .github/workflows/ci.yml installs ast-grep 0.44.1 and runs `ast-grep scan --config sgconfig.yml`.
- [ ] **C21** — `ast-grep scan --config sgconfig.yml` reports zero hits and exits 0 at the landed commit.
- [ ] **C22** — A never-landed scratch file with an unwrap and no #![cfg(test)] makes .land/gates.sh show the ast-grep leg red and every other leg green.
- [ ] **C23** — The leg lands only after origin/main's crates/lys-home/src/record/mod.rs has zero non-module lines, checked by command.

## Pre-method documents

- [ ] **C85** — docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md are renamed to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md with their bytes unchanged, and the two live citations name the new paths.
- [ ] **C86** — The lys-core cluster's DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-003.md are rendered from its JSON, and sh scripts/design/gate.sh exits 0.

## Test code recognition

- [ ] **C87** — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true, and CLAUDE.md and Cargo.toml name it in place of the per-module #![allow].
- [ ] **C88** — Every *_tests.rs file, both fixture.rs files, every integration test root and the two tests/harness/mod.rs files begin with #![cfg(test)], and no #![allow] line remains in any of them.
- [ ] **C89** — crates/lys-core/src/keys/identity_tests.rs holds no #[allow(unsafe_code)] and no unsafe block, and the comment above lib.rs's unsafe_code attribute says what is true after that.

## Existing hits cleared

- [ ] **C90** — No `let _ =` statement remains under crates/.
- [ ] **C91** — crates/lys-core/tests/harness/mod.rs, crates/lys-anchor/tests/harness/mod.rs and crates/lys-home/src/harness/claude_code/mod.rs hold no fn, struct, enum, trait, impl, const or static item.

## Rules and leg

- [ ] **C92** — sgconfig.yml names rules/ast-grep, which holds mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes and no-unwrap-expect-panic-outside-tests, each at severity error.
- [ ] **C93** — The leg `ast-grep scan --config sgconfig.yml` is in docs/design/project.json requiring tool:ast-grep, in .land/gates.sh and in CI.
- [ ] **C94** — The scan reports zero hits over the landed tree, and exactly one hit on an uncommitted scratch file holding an unwrap outside test code.
