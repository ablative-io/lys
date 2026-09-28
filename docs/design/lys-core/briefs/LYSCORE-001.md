---
type: brief
id: LYSCORE-001
cluster: lys-core
title: Carry the hand-written lys-core design into its three JSON documents
---

# LYSCORE-001: Carry the hand-written lys-core design into its three JSON documents

> **Cluster:** lys-core
> **Checklist:**
> - C1 — DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md under docs/design/lys-core are byte-identical to DESIGN.md, CHECKLIST.md and USER-STORIES.md at 7b53625.
> - C2 — DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md under docs/design/lys-core are what render-cluster.py renders, and sh scripts/design/gate.sh exits 0.
> - C3 — sgconfig.yml at the repository root names rules/ast-grep as its only rule directory.
> - C4 — rules/ast-grep carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes with the rule bodies Cambium carries at 1be80d8ec.
> - C5 — rules/ast-grep carries the error-severity rule no-unwrap-expect-panic-outside-tests, which reports unwrap, expect and panic outside test code recognised by structure.
> - C6 — Every rule in rules/ast-grep ignores vendor/**.
> - C7 — The brief records no-std-mutex-in-async, no-timer-in-door-handlers, no-timer-import-in-door-handlers and the _name rule as not carried, each with its finding.
> - C8 — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true.
> - C9 — Both fixture.rs files and the 88 sibling *_tests.rs files under src/ begin with #![cfg(test)] and carry no #![allow].
> - C10 — The 28 integration test roots and both tests/harness/mod.rs files begin with #![cfg(test)] and carry no #![allow].
> - C11 — The env-backed tests of Ed25519Identity::from_env call no set_var, remove_var or unsafe code, and from_env keeps its signature and its error texts.
> - C12 — The comment above #![cfg_attr(not(test), forbid(unsafe_code))] in lys-core's lib.rs says what is true after the env tests change, and the attribute is byte-identical.
> - C13 — grep finds zero #[allow], #![allow], #[expect] and #[ignore] under crates/.
> - C14 — The tests/harness/mod.rs files of lys-core and lys-anchor and lys-home's harness/claude_code/mod.rs hold only module declarations and re-exports.
> - C15 — The `let _ =` hits in the three hex writers, the 14 test-code writes, identity_tests.rs and json_output_tests.rs are gone.
> - C16 — CLAUDE.md's coding-standards sentence and the Cargo.toml lint comment name clippy.toml's allow-*-in-tests in place of the per-module #![allow].
> - C17 — CLAUDE.md's gates block lists `ast-grep scan --config sgconfig.yml` and its count sentence matches the lines it lists.
> - C18 — docs/design/project.json has a leg running `ast-grep scan --config sgconfig.yml` requiring tool:ast-grep, and validate.py accepts the file.
> - C19 — .land/gates.sh runs `leg ast-grep scan --config sgconfig.yml`.
> - C20 — .github/workflows/ci.yml installs ast-grep 0.44.1 and runs `ast-grep scan --config sgconfig.yml`.
> - C21 — `ast-grep scan --config sgconfig.yml` reports zero hits and exits 0 at the landed commit.
> - C22 — A never-landed scratch file with an unwrap and no #![cfg(test)] makes .land/gates.sh show the ast-grep leg red and every other leg green.
> - C23 — The leg lands only after origin/main's crates/lys-home/src/record/mod.rs has zero non-module lines, checked by command.
> - C24 — `LYS_OID_ARC` constant equals [1, 3, 6, 1, 4, 1, 66364] with a doc comment stating that 66364 is the IANA Private Enterprise Number assigned to lys and that the arc is permanent
> - C25 — encode_extension / decode_extension round-trip an arbitrary DER payload under LYS_OID_ARC; decode of a cert without the extension returns Ok(None)
> - C26 — Round-trip test: rcgen-generated Ed25519 keypair is loadable as ed25519-dalek SigningKey/VerifyingKey
> - C27 — AppendOnlyTree<L> generic over leaf type L: Serialize; append(leaf) returns the new tree size
> - C28 — No delete or modify operation exists on the tree — append-only enforced by API
> - C29 — root() returns the current RootHash; the empty tree produces a deterministic empty root hash
> - C30 — prove_inclusion(leaf_index) pre-checks bounds and returns TrustError::MerkleTree on out-of-range index — no panic path into the backing library
> - C31 — prove_consistency(old_size, new_size) pre-checks the size pair (old ≤ new, new ≤ len, old ≥ 1) and returns TrustError::MerkleTree on violation
> - C32 — verify_inclusion(root_hash, leaf, index, proof) and verify_consistency(old_root, new_root, proof) return Result; tampered proofs and mismatched roots fail
> - C33 — RootHash::from_parts(root_hash, num_leaves) and to_parts() round-trip; from_parts requires no tree access
> - C34 — InclusionProof and ConsistencyProof round-trip through as_bytes() / try_from_bytes()
> - C35 — External-verifier round-trip test exists: a verifier holding only published root parts and proof bytes (never the tree) verifies inclusion and consistency
> - C36 — reconstruct_from_leaves(leaves) rebuilds a tree with a root hash identical to the original (test exists)
> - C37 — merkle module docs state the frozen-wire-contract rule: leaf encodings are canonical bytes, evolved only by introducing a new versioned leaf type
> - C38 — sign_attestation(payload, signing_key) signs the COSE `Sig_structure` `["Signature1", protected, h'', claims]` (RFC 9052 §4.4) with protected `{1: -8, 3: "application/vnd.lys.attestation.v2+cbor", 4: signer key}` — no meridian string, no v1 preimage constant remains anywhere
> - C39 — Attestation { payload_hash: [u8; 32], signature: [u8; 64], signer_public_key: [u8; 32], timestamp: i64 } carries no serde; the only durable form is `to_cose_bytes()` / `from_cose_bytes()` (canonical-encoding-strict)
> - C40 — verify_attestation(attestation, payload) rebuilds the `Sig_structure` from the attestation's own fields and verifies with `verify_strict`
> - C41 — No legacy fallback exists: a signature over the bare payload hash and a signature over the deleted v1 preimage both fail verify_attestation (tests exist)
> - C42 — Tampered payload fails verify_attestation
> - C43 — Tampered timestamp fails verify_attestation — the timestamp is a signed claim inside the `Sig_structure` (test exists)
> - C44 — seal(payload, recipient_public_key) returns SealedEnvelope { ephemeral_public_key, ciphertext, nonce } using a fresh ephemeral X25519 keypair per call (two seals of the same payload to the same recipient differ)
> - C45 — HKDF-SHA256 info input is `b"lys-sealed-envelope/v1" || ephemeral_public_key || recipient_public_key` — the hyphen-form HKDF domain tag, deliberately distinct from the slash-form attestation context tag `lys/sealed-envelope/v1`
> - C46 — Both seal and open reject non-contributory Diffie-Hellman: a low-order public key fails via `was_contributory` before any key derivation (test exists)
> - C47 — Seal/open roundtrip succeeds: sealed with the recipient's X25519 public key, opened with the recipient's static secret
> - C48 — Wrong private key, tampered ciphertext, and tampered nonce all return exactly TrustError::UnsealFailed — a single undifferentiated failure through the AES-GCM arbiter, with no early return distinguishing causes
> - C49 — SealedEnvelope::attestation_bytes() covers every wire byte of the envelope (ephemeral key, nonce, ciphertext)
> - C50 — sign_and_seal(payload, sender_identity, recipient_x25519_public_key) returns (SealedEnvelope, Attestation) where the attestation signs attestation_bytes()
> - C51 — open_and_verify verifies the attestation before any decryption: an invalid sender signature is rejected without the cipher being touched, and a valid signature over a tampered envelope also fails (tests exist)
> - C52 — `lys` binary crate exists; main.rs is a thin entry (parse, dispatch, exit codes) with clap definitions isolated in cli.rs; no `anyhow` anywhere — the CLI carries its own thiserror type
> - C53 — `lys key` generates an identity at a path and inspects one (public key, fingerprint); no subcommand, flag, or output format prints private key material (test asserts output contains no seed bytes in any encoding)
> - C54 — `lys ca issue` issues a certificate signed by an issuer identity file, embedding a caller-supplied capability-claim payload as a LYS_OID_ARC extension, and writes the PEM out
> - C55 — `lys ca verify` verifies a certificate against an issuer public key, and accepts an explicit verification instant flag routing to verify_certificate_chain_at
> - C56 — `lys attest` signs a payload file and emits the COSE_Sign1 artifact; `lys verify` checks an artifact against a payload and reports success/failure via exit code. (File paths only as built — the stdin path this item originally anticipated was not implemented, and ROADMAP Phase 2 records the file-only surface.)
> - C57 — `lys seal` seals a payload file for a recipient public key and writes the sender attestation alongside it; `lys open` opens it with the recipient identity, verifying the attestation first; the pair round-trips
> - C58 — `lys log init` pins the log's origin exactly once and refuses to re-initialize; `lys log append` appends a leaf file's raw bytes and prints the new root; `lys log checkpoint` signs a C2SP tlog-checkpoint in the signed-note envelope over the current root; `lys log prove` emits a self-contained JSON proof artifact with the relevant signed checkpoint(s) embedded verbatim
> - C59 — `lys log verify` verifies an inclusion or consistency claim from **only** the artifact, the leaf, and the verifier key — no access to the leaf sequence, the store, or the tree; declared sizes are checked against the signature-verified checkpoint and roots are recomputed, never trusted; every tamper class collapses to one identical message
> - C60 — Cross-process CLI test exists: a log produced by one process is verified end-to-end by the CLI in another process with no access to the original tree
> - C61 — cargo fmt --check passes clean *(gate — verified by CI, not from source)*
> - C62 — cargo clippy --all-targets -- -D warnings passes clean *(gate — verified by CI, not from source)*
> - C63 — cargo test --workspace passes green *(gate — verified by CI, not from source)*
> - C64 — No file exceeds 500 lines of code; every mod.rs carries only pub mod / pub use / module docs; tests live in sibling *_tests.rs files *(not met: `merkle/leaf.rs`, both `seal/` files, `error.rs`, and six CLI modules carry inline `mod tests` — REVIEW-23-07.md F12)*
> - C65 — lys-core builds standalone with zero meridian-* dependencies in its Cargo.toml and Cargo.lock
> **Stories:**
> - S1 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the gate to refuse unwrap, expect and panic in library code so that a panic path cannot land where clippy is silenced.
> - S2 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want a lint bypass attribute refused at landing so that a lint is fixed at its cause instead of hidden.
> - S3 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the same scan run by the design gate, the landing gate and CI so that no path to main skips it.
> - S4 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want every rule shown to fire on a scratch case so that a silent scan means clean code rather than a rule that never matches.
> - S5 (Lys contributor, Writing tests and library code) — As a lys contributor writing tests, I want a test file recognised by the marker it carries so that its helpers need no per-file lint opt-out and no one keeps an exemption list.
> - S6 (Lys contributor, Writing tests and library code) — As a lys contributor reading the tree's rules, I want CLAUDE.md, Cargo.toml and the lib.rs comment to state the policy the gate enforces so that the written rule and the enforced rule agree.
> - S7 (Lys contributor, Writing tests and library code) — As a lys contributor changing key loading, I want the env-backed identity tests to run without unsafe code so that the test build needs no lint bypass.
> - S8 (Lys contributor, Writing tests and library code) — As a lys contributor reading a module tree, I want mod.rs files to hold only declarations and re-exports so that logic is found in a named file.
> - S9 (Lys contributor, Writing tests and library code) — As a lys contributor, I want no `let _ =` discard in the tree so that no error is swallowed without a decision.
> - S10 (Design reader, Reading the lys-core cluster) — As a reader of the lys-core design, I want the hand-written pre-method documents kept beside the rendered cluster so that the earlier design is not lost when the method's documents replace it.
> - S11 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys ca verify` with an explicit instant so that I can check whether a certificate was valid at the time a disputed action occurred, not just at the time of my audit.
> - S12 (Lys CLI, Operator and Auditor) — As an operator, I want `lys attest` to sign a file or stdin so that I can hand a third party a detached, self-contained attestation over any artifact.
> - S13 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys verify` to check an attestation with only the payload and the signer's public key so that verification requires nothing from the party who produced the record.
> - S14 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys log verify` to prove inclusion from only a published root and proof bytes so that I can confirm a challenged entry was logged without the operator's cooperation and without seeing any other entry.
> - S15 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys log verify` to check consistency between two published roots so that I can detect any rewrite of history between two points in time.
> - S16 (Lys CLI, Operator and Auditor) — As an operator, I want `lys seal` and `lys open` so that I can move a credential file to a specific recipient with per-envelope forward secrecy instead of pasting secrets into a chat.
> - S17 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to reconstruct submitted roots via `RootHash::from_parts` so that I can verify consistency proofs between an instance's successive submissions while seeing only roots, never contents.
> - S18 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to verify the submitter's v1 domain-separated attestation over each submitted root so that only the holder of the registered instance key can extend that instance's anchored history.
> - S19 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to append anchored roots to my own `AppendOnlyTree` and serve inclusion proofs so that my receipt for an anchoring event is itself independently verifiable.
> - S20 (Lys-Anchor Service, (future notary)) — As the anchor service, I want the wire tags and preimage layouts frozen at `v1` so that a receipt issued today still verifies against signatures produced years from now.
> - S21 (Haematite, Commit Attestation) — As haematite, I want to attest a BLAKE3 commit root — 32 opaque bytes signed with the instance identity — so that a whole database state becomes attestable without lys knowing anything about haematite's hash world.
> - S22 (Haematite, Commit Attestation) — As haematite, I want to append commit attestations to an append-only log so that my flat timestamped commit list gains the hash-chained lineage it structurally lacks.
> - S23 (Haematite, Commit Attestation) — As haematite, I want inclusion proofs over the commit log so that any party can verify a historical commit belongs to the canonical lineage without replaying the database.

## Purpose

The lys-core design on main is three hand-written documents, DESIGN.md, CHECKLIST.md and USER-STORIES.md, with no JSON sources. The design gate renders every cluster that has a design.json and compares the render with the committed markdown, so the first lys-core brief to add a design.json would replace the hand-written documents with whatever that JSON holds and the older design would leave the record. This brief carries the whole hand-written design into design.json, checklist.json and stories.json first, so that the render reproduces it: every C id and text, every S id and text, and every section of DESIGN.md in the field the design schema gives it. Nothing new is designed and no code changes.

## Task

Carry main's docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md at 7b536253 into docs/design/lys-core/design.json, checklist.json and stories.json (R1 to R3), render the cluster with python3 scripts/design/render-cluster.py docs/design/lys-core, commit the rendered DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md from that render (R4), and pass sh scripts/design/gate.sh. Every path is relative to the repository root.

The three JSON documents are written on the brief branch with this brief, as the method writes a cluster's design beside its first brief. The build starts from them: it runs the measurements in R1 to R3 against main's bytes at 7b536253, corrects any carried text the measurements show differs from main so that it matches main exactly, and renders. Write each JSON document with json.dumps(doc, indent=2, ensure_ascii=False) and one trailing newline, so an en dash stays an en dash and a backtick stays a backtick.

What is carried, and where. Nothing is reworded, merged, dropped, renumbered or reordered, and stale text is carried as it stands (S12's stdin, S14's root parts, and the structure tree's missing delegation/, receipt/, bundle/, ca/request.rs and keys/ssh.rs are corrected by a later unit, not here). The constraints take ids CN1 to CN9 in main's order because the schema requires an id; the goals render as bullets in main's order because the renderer writes goals that way; the fenced structure tree becomes one structure row per entry, 75 rows (73 entries under the two roots crates/lys-core/ and crates/lys/, and the two roots), with each path spelled as the tree spells it (directory entries keep their trailing slash, *_tests.rs globs stay globs), continuation lines joined into the note with one space, and every brief field empty because every one of those paths exists on main; the D1 to D8 subsections stay inside the solution text, not in decisions, because they are not ledger ADRs.

Sentences with no field of their own, named: (a) the paragraph under main's structure tree that begins 'Tests live in sibling' has no field after a structure table, so it goes into the solution text, as its own paragraph after the last D8 paragraph; (b) CHECKLIST.md's three-paragraph intro blockquote and its two italic annotations, the one above C38 that covers C38 to C43 and the one above C58 that covers C58 to C59, have no field in checklist.json, so they go into design.json as one section at the end of the solution text headed '### Checklist notes', carried verbatim, each annotation opening with the ids it covers as main's does; they do not go into any checklist section name, and the eight checklist headings stay exactly as main has them; (c) three persona headings have no ' — ' between a name and a role, and the renderer always writes one, so each is split into name and role at the point given in R2. The dev record says each of these, where it went, and why.

Differences the render makes against main, which the R3 dev record lists by name with its reason. Layout: the frontmatter title loses its quotes; the renderer adds the '> **Cluster:** lys-core' line under the heading; the 'Tests live in sibling' paragraph moves from under the structure tree to the end of the D8 text; goals change from '1.' to '6.' numbering to bullets; the structure tree becomes a Path, Note, Brief table; each constraint gains its '**CNn** —' prefix. Content, the two the card's lead allows: the '### Checklist notes' section in the solution (carrying (b) above, so the rendered DESIGN.md holds it and the rendered CHECKLIST.md does not), and the eight structure rows for the cluster's own documents and this brief's own files, listed as the directory cluster lists its own, so that check-coverage.py passes and R1 to R4 name real files. Every other difference between the rendered DESIGN.md and main's is layout only. The dev record also lists the rendered CHECKLIST.md's eleven missing lines and the rendered USER-STORIES.md's three changed headings.

Coverage. This brief claims C1 to C65 and S1 to S23 because check-coverage.py fails every checklist item and story that no brief in the cluster claims. It carries them; it did not build them. The done flags record main's ticks unchanged: C1 to C60 and C65 true, C61 to C64 false. The later lys-core card that waits on this one adds its own brief, items and stories after the carried ones; none of them is added here.

Out of scope: any new checklist item, user story, principle, ADR, goal, non-goal or constraint; any change to code, to crates/, to scripts/design/ (renderer, validator, schemas) or to any other cluster; a note field for checklists and personas in the method's renderer and schemas.

## Requirements

### R1: Carry CHECKLIST.md into checklist.json and render it

docs/design/lys-core/checklist.json holds cluster 'lys-core' and main's eight CHECKLIST.md sections at 7b536253, each named exactly as main names it and in main's order: Crate Setup, Key Management, Certificate Authority, Merkle Transparency Log, Signed Attestations, Sealed Envelope, CLI Surface, Integration Verification. Each of main's item lines '- [x] **Cn** — text' and '- [ ] **Cn** — text' becomes one item {id, text, done} in its section and in main's order, where text is every character after the first ' — ' byte for byte (C61 to C64 keep their italic tails inside the text) and done is true for '[x]' and false for '[ ]'. The document SHALL NOT reword, merge, drop, renumber, reorder or add any item, SHALL NOT change any section name, and SHALL NOT hold the intro blockquote and the two italic annotations in any section name or item text. WHEN python3 scripts/design/render-cluster.py docs/design/lys-core runs, THE SYSTEM SHALL write docs/design/lys-core/CHECKLIST.md whose eight headings and 65 item lines equal main's byte for byte, and SHALL NOT write any line main's CHECKLIST.md does not hold.

**Acceptance:**
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/lys-core/checklist.json')); ids=[i['id'] for s in d['sections'] for i in s['items']]; print(len(ids), len(set(ids)), ids==['C%d' % n for n in range(1, 66)])" prints 65 65 True.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/lys-core/checklist.json')); print([i['id'] for s in d['sections'] for i in s['items'] if not i['done']])" prints ['C61', 'C62', 'C63', 'C64'].
- From the repository root: python3 -c "import json; print([s['name'] for s in json.load(open('docs/design/lys-core/checklist.json'))['sections']])" prints ['Crate Setup', 'Key Management', 'Certificate Authority', 'Merkle Transparency Log', 'Signed Attestations', 'Sealed Envelope', 'CLI Surface', 'Integration Verification'].
- From the repository root: the C and S text comparison in this brief's verification prints 'compared 88 differences 0' as its last line.
- From the repository root: git show 7b536253:docs/design/lys-core/CHECKLIST.md | diff - docs/design/lys-core/CHECKLIST.md | grep -c '^<' prints 11 (the five intro blockquote lines, their following blank line, and the two annotations with their three blank lines), and the same pipeline with grep -c '^>' prints 0.

**Files:**
- modify: docs/design/lys-core/checklist.json
- modify: docs/design/lys-core/CHECKLIST.md

**Checklist:**
- C1 — DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md under docs/design/lys-core are byte-identical to DESIGN.md, CHECKLIST.md and USER-STORIES.md at 7b53625.
- C2 — DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md under docs/design/lys-core are what render-cluster.py renders, and sh scripts/design/gate.sh exits 0.
- C3 — sgconfig.yml at the repository root names rules/ast-grep as its only rule directory.
- C4 — rules/ast-grep carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes with the rule bodies Cambium carries at 1be80d8ec.
- C5 — rules/ast-grep carries the error-severity rule no-unwrap-expect-panic-outside-tests, which reports unwrap, expect and panic outside test code recognised by structure.
- C6 — Every rule in rules/ast-grep ignores vendor/**.
- C7 — The brief records no-std-mutex-in-async, no-timer-in-door-handlers, no-timer-import-in-door-handlers and the _name rule as not carried, each with its finding.
- C8 — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true.
- C9 — Both fixture.rs files and the 88 sibling *_tests.rs files under src/ begin with #![cfg(test)] and carry no #![allow].
- C10 — The 28 integration test roots and both tests/harness/mod.rs files begin with #![cfg(test)] and carry no #![allow].
- C11 — The env-backed tests of Ed25519Identity::from_env call no set_var, remove_var or unsafe code, and from_env keeps its signature and its error texts.
- C12 — The comment above #![cfg_attr(not(test), forbid(unsafe_code))] in lys-core's lib.rs says what is true after the env tests change, and the attribute is byte-identical.
- C13 — grep finds zero #[allow], #![allow], #[expect] and #[ignore] under crates/.
- C14 — The tests/harness/mod.rs files of lys-core and lys-anchor and lys-home's harness/claude_code/mod.rs hold only module declarations and re-exports.
- C15 — The `let _ =` hits in the three hex writers, the 14 test-code writes, identity_tests.rs and json_output_tests.rs are gone.
- C16 — CLAUDE.md's coding-standards sentence and the Cargo.toml lint comment name clippy.toml's allow-*-in-tests in place of the per-module #![allow].
- C17 — CLAUDE.md's gates block lists `ast-grep scan --config sgconfig.yml` and its count sentence matches the lines it lists.
- C18 — docs/design/project.json has a leg running `ast-grep scan --config sgconfig.yml` requiring tool:ast-grep, and validate.py accepts the file.
- C19 — .land/gates.sh runs `leg ast-grep scan --config sgconfig.yml`.
- C20 — .github/workflows/ci.yml installs ast-grep 0.44.1 and runs `ast-grep scan --config sgconfig.yml`.
- C21 — `ast-grep scan --config sgconfig.yml` reports zero hits and exits 0 at the landed commit.
- C22 — A never-landed scratch file with an unwrap and no #![cfg(test)] makes .land/gates.sh show the ast-grep leg red and every other leg green.
- C23 — The leg lands only after origin/main's crates/lys-home/src/record/mod.rs has zero non-module lines, checked by command.
- C24 — `LYS_OID_ARC` constant equals [1, 3, 6, 1, 4, 1, 66364] with a doc comment stating that 66364 is the IANA Private Enterprise Number assigned to lys and that the arc is permanent
- C25 — encode_extension / decode_extension round-trip an arbitrary DER payload under LYS_OID_ARC; decode of a cert without the extension returns Ok(None)
- C26 — Round-trip test: rcgen-generated Ed25519 keypair is loadable as ed25519-dalek SigningKey/VerifyingKey
- C27 — AppendOnlyTree<L> generic over leaf type L: Serialize; append(leaf) returns the new tree size
- C28 — No delete or modify operation exists on the tree — append-only enforced by API
- C29 — root() returns the current RootHash; the empty tree produces a deterministic empty root hash
- C30 — prove_inclusion(leaf_index) pre-checks bounds and returns TrustError::MerkleTree on out-of-range index — no panic path into the backing library
- C31 — prove_consistency(old_size, new_size) pre-checks the size pair (old ≤ new, new ≤ len, old ≥ 1) and returns TrustError::MerkleTree on violation
- C32 — verify_inclusion(root_hash, leaf, index, proof) and verify_consistency(old_root, new_root, proof) return Result; tampered proofs and mismatched roots fail
- C33 — RootHash::from_parts(root_hash, num_leaves) and to_parts() round-trip; from_parts requires no tree access
- C34 — InclusionProof and ConsistencyProof round-trip through as_bytes() / try_from_bytes()
- C35 — External-verifier round-trip test exists: a verifier holding only published root parts and proof bytes (never the tree) verifies inclusion and consistency
- C36 — reconstruct_from_leaves(leaves) rebuilds a tree with a root hash identical to the original (test exists)
- C37 — merkle module docs state the frozen-wire-contract rule: leaf encodings are canonical bytes, evolved only by introducing a new versioned leaf type
- C38 — sign_attestation(payload, signing_key) signs the COSE `Sig_structure` `["Signature1", protected, h'', claims]` (RFC 9052 §4.4) with protected `{1: -8, 3: "application/vnd.lys.attestation.v2+cbor", 4: signer key}` — no meridian string, no v1 preimage constant remains anywhere
- C39 — Attestation { payload_hash: [u8; 32], signature: [u8; 64], signer_public_key: [u8; 32], timestamp: i64 } carries no serde; the only durable form is `to_cose_bytes()` / `from_cose_bytes()` (canonical-encoding-strict)
- C40 — verify_attestation(attestation, payload) rebuilds the `Sig_structure` from the attestation's own fields and verifies with `verify_strict`
- C41 — No legacy fallback exists: a signature over the bare payload hash and a signature over the deleted v1 preimage both fail verify_attestation (tests exist)
- C42 — Tampered payload fails verify_attestation
- C43 — Tampered timestamp fails verify_attestation — the timestamp is a signed claim inside the `Sig_structure` (test exists)
- C44 — seal(payload, recipient_public_key) returns SealedEnvelope { ephemeral_public_key, ciphertext, nonce } using a fresh ephemeral X25519 keypair per call (two seals of the same payload to the same recipient differ)
- C45 — HKDF-SHA256 info input is `b"lys-sealed-envelope/v1" || ephemeral_public_key || recipient_public_key` — the hyphen-form HKDF domain tag, deliberately distinct from the slash-form attestation context tag `lys/sealed-envelope/v1`
- C46 — Both seal and open reject non-contributory Diffie-Hellman: a low-order public key fails via `was_contributory` before any key derivation (test exists)
- C47 — Seal/open roundtrip succeeds: sealed with the recipient's X25519 public key, opened with the recipient's static secret
- C48 — Wrong private key, tampered ciphertext, and tampered nonce all return exactly TrustError::UnsealFailed — a single undifferentiated failure through the AES-GCM arbiter, with no early return distinguishing causes
- C49 — SealedEnvelope::attestation_bytes() covers every wire byte of the envelope (ephemeral key, nonce, ciphertext)
- C50 — sign_and_seal(payload, sender_identity, recipient_x25519_public_key) returns (SealedEnvelope, Attestation) where the attestation signs attestation_bytes()
- C51 — open_and_verify verifies the attestation before any decryption: an invalid sender signature is rejected without the cipher being touched, and a valid signature over a tampered envelope also fails (tests exist)
- C52 — `lys` binary crate exists; main.rs is a thin entry (parse, dispatch, exit codes) with clap definitions isolated in cli.rs; no `anyhow` anywhere — the CLI carries its own thiserror type
- C53 — `lys key` generates an identity at a path and inspects one (public key, fingerprint); no subcommand, flag, or output format prints private key material (test asserts output contains no seed bytes in any encoding)
- C54 — `lys ca issue` issues a certificate signed by an issuer identity file, embedding a caller-supplied capability-claim payload as a LYS_OID_ARC extension, and writes the PEM out
- C55 — `lys ca verify` verifies a certificate against an issuer public key, and accepts an explicit verification instant flag routing to verify_certificate_chain_at
- C56 — `lys attest` signs a payload file and emits the COSE_Sign1 artifact; `lys verify` checks an artifact against a payload and reports success/failure via exit code. (File paths only as built — the stdin path this item originally anticipated was not implemented, and ROADMAP Phase 2 records the file-only surface.)
- C57 — `lys seal` seals a payload file for a recipient public key and writes the sender attestation alongside it; `lys open` opens it with the recipient identity, verifying the attestation first; the pair round-trips
- C58 — `lys log init` pins the log's origin exactly once and refuses to re-initialize; `lys log append` appends a leaf file's raw bytes and prints the new root; `lys log checkpoint` signs a C2SP tlog-checkpoint in the signed-note envelope over the current root; `lys log prove` emits a self-contained JSON proof artifact with the relevant signed checkpoint(s) embedded verbatim
- C59 — `lys log verify` verifies an inclusion or consistency claim from **only** the artifact, the leaf, and the verifier key — no access to the leaf sequence, the store, or the tree; declared sizes are checked against the signature-verified checkpoint and roots are recomputed, never trusted; every tamper class collapses to one identical message
- C60 — Cross-process CLI test exists: a log produced by one process is verified end-to-end by the CLI in another process with no access to the original tree
- C61 — cargo fmt --check passes clean *(gate — verified by CI, not from source)*
- C62 — cargo clippy --all-targets -- -D warnings passes clean *(gate — verified by CI, not from source)*
- C63 — cargo test --workspace passes green *(gate — verified by CI, not from source)*
- C64 — No file exceeds 500 lines of code; every mod.rs carries only pub mod / pub use / module docs; tests live in sibling *_tests.rs files *(not met: `merkle/leaf.rs`, both `seal/` files, `error.rs`, and six CLI modules carry inline `mod tests` — REVIEW-23-07.md F12)*
- C65 — lys-core builds standalone with zero meridian-* dependencies in its Cargo.toml and Cargo.lock

### R2: Carry USER-STORIES.md into stories.json and render it

docs/design/lys-core/stories.json holds cluster 'lys-core' and main's four USER-STORIES.md persona headings at 7b536253 as four personas in main's order, each split into name and role: 'Norn Agent Runtime — Signing Session History (primary consumer)' at its ' — ' into name 'Norn Agent Runtime' and role 'Signing Session History (primary consumer)'; 'Lys CLI Operator and Auditor' into name 'Lys CLI' and role 'Operator and Auditor'; 'Lys-Anchor Service (future notary)' into name 'Lys-Anchor Service' and role '(future notary)'; 'Haematite Commit Attestation' into name 'Haematite' and role 'Commit Attestation'. Each of main's story lines '**Sn.** text' becomes one story {id, text} under its persona and in main's order, where text is every character after '**Sn.** ' byte for byte. The document SHALL NOT reword, merge, drop, renumber, reorder or add any story, and SHALL NOT drop or add any word of a persona heading. WHEN python3 scripts/design/render-cluster.py docs/design/lys-core runs, THE SYSTEM SHALL write docs/design/lys-core/USER-STORIES.md whose 23 story lines equal main's byte for byte and whose only changed lines are the three headings that gain ' — '.

**Acceptance:**
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/lys-core/stories.json')); ids=[s['id'] for p in d['personas'] for s in p['stories']]; print(len(ids), len(set(ids)), ids==['S%d' % n for n in range(1, 24)])" prints 23 23 True.
- From the repository root: python3 -c "import json; print([(p['name'], p['role']) for p in json.load(open('docs/design/lys-core/stories.json'))['personas']])" prints [('Norn Agent Runtime', 'Signing Session History (primary consumer)'), ('Lys CLI', 'Operator and Auditor'), ('Lys-Anchor Service', '(future notary)'), ('Haematite', 'Commit Attestation')].
- From the repository root: the C and S text comparison in this brief's verification prints 'compared 88 differences 0' as its last line.
- From the repository root: git show 7b536253:docs/design/lys-core/USER-STORIES.md | diff - docs/design/lys-core/USER-STORIES.md | grep '^>' prints exactly the three lines '> ## Lys CLI — Operator and Auditor', '> ## Lys-Anchor Service — (future notary)' and '> ## Haematite — Commit Attestation', and the same pipeline with grep -c '^<' prints 3.

**Files:**
- modify: docs/design/lys-core/stories.json
- modify: docs/design/lys-core/USER-STORIES.md

**Stories:**
- S1 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the gate to refuse unwrap, expect and panic in library code so that a panic path cannot land where clippy is silenced.
- S2 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want a lint bypass attribute refused at landing so that a lint is fixed at its cause instead of hidden.
- S3 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the same scan run by the design gate, the landing gate and CI so that no path to main skips it.
- S4 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want every rule shown to fire on a scratch case so that a silent scan means clean code rather than a rule that never matches.
- S5 (Lys contributor, Writing tests and library code) — As a lys contributor writing tests, I want a test file recognised by the marker it carries so that its helpers need no per-file lint opt-out and no one keeps an exemption list.
- S6 (Lys contributor, Writing tests and library code) — As a lys contributor reading the tree's rules, I want CLAUDE.md, Cargo.toml and the lib.rs comment to state the policy the gate enforces so that the written rule and the enforced rule agree.
- S7 (Lys contributor, Writing tests and library code) — As a lys contributor changing key loading, I want the env-backed identity tests to run without unsafe code so that the test build needs no lint bypass.
- S8 (Lys contributor, Writing tests and library code) — As a lys contributor reading a module tree, I want mod.rs files to hold only declarations and re-exports so that logic is found in a named file.
- S9 (Lys contributor, Writing tests and library code) — As a lys contributor, I want no `let _ =` discard in the tree so that no error is swallowed without a decision.
- S10 (Design reader, Reading the lys-core cluster) — As a reader of the lys-core design, I want the hand-written pre-method documents kept beside the rendered cluster so that the earlier design is not lost when the method's documents replace it.
- S11 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys ca verify` with an explicit instant so that I can check whether a certificate was valid at the time a disputed action occurred, not just at the time of my audit.
- S12 (Lys CLI, Operator and Auditor) — As an operator, I want `lys attest` to sign a file or stdin so that I can hand a third party a detached, self-contained attestation over any artifact.
- S13 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys verify` to check an attestation with only the payload and the signer's public key so that verification requires nothing from the party who produced the record.
- S14 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys log verify` to prove inclusion from only a published root and proof bytes so that I can confirm a challenged entry was logged without the operator's cooperation and without seeing any other entry.
- S15 (Lys CLI, Operator and Auditor) — As an auditor, I want `lys log verify` to check consistency between two published roots so that I can detect any rewrite of history between two points in time.
- S16 (Lys CLI, Operator and Auditor) — As an operator, I want `lys seal` and `lys open` so that I can move a credential file to a specific recipient with per-envelope forward secrecy instead of pasting secrets into a chat.
- S17 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to reconstruct submitted roots via `RootHash::from_parts` so that I can verify consistency proofs between an instance's successive submissions while seeing only roots, never contents.
- S18 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to verify the submitter's v1 domain-separated attestation over each submitted root so that only the holder of the registered instance key can extend that instance's anchored history.
- S19 (Lys-Anchor Service, (future notary)) — As the anchor service, I want to append anchored roots to my own `AppendOnlyTree` and serve inclusion proofs so that my receipt for an anchoring event is itself independently verifiable.
- S20 (Lys-Anchor Service, (future notary)) — As the anchor service, I want the wire tags and preimage layouts frozen at `v1` so that a receipt issued today still verifies against signatures produced years from now.
- S21 (Haematite, Commit Attestation) — As haematite, I want to attest a BLAKE3 commit root — 32 opaque bytes signed with the instance identity — so that a whole database state becomes attestable without lys knowing anything about haematite's hash world.
- S22 (Haematite, Commit Attestation) — As haematite, I want to append commit attestations to an append-only log so that my flat timestamped commit list gains the hash-chained lineage it structurally lacks.
- S23 (Haematite, Commit Attestation) — As haematite, I want inclusion proofs over the commit log so that any party can verify a historical commit belongs to the canonical lineage without replaying the database.

### R3: Carry DESIGN.md into design.json and render it

docs/design/lys-core/design.json holds cluster 'lys-core' and main's DESIGN.md at 7b536253 in the fields the design schema gives it: title 'Lys Core — Trust Primitives and CLI Surface' (the frontmatter title without its quotes); intention, the two Intention paragraphs; problem, the whole Problem section (opening paragraph, four bullets, closing paragraph); solution, the whole Solution section from '### D1: Domain-agnostic boundary' to the paragraph beginning 'The phase proof', then the paragraph under main's structure tree beginning 'Tests live in sibling', then a section headed '### Checklist notes' holding main's CHECKLIST.md intro blockquote (its five lines as main writes them) and its two italic annotations, the C38 to C43 one first, each carried verbatim and separated by one blank line; goals, the six goals in main's order without their '1. ' to '6. ' numbering; non_goals, the seven bullets in main's order, each whole bullet after '- ' as text and reason empty; structure, 75 rows for the fenced tree's two roots and 73 entries in the tree's order, each path the entry's full repository-relative path as the tree spells it, each note the entry's comment after '— ' with continuation lines joined by one space (empty when the entry has none), each brief empty, followed by eight rows, in this order and exactly as written here (path 'docs/design/lys-core/design.json' with note 'the lys-core design, carried from main's DESIGN.md' and brief 'LYSCORE-001'; path 'docs/design/lys-core/DESIGN.md' with note 'rendered markdown' and brief empty; path 'docs/design/lys-core/checklist.json' with note 'the lys-core checklist, carried from main's CHECKLIST.md' and brief 'LYSCORE-001'; path 'docs/design/lys-core/CHECKLIST.md' with note 'rendered markdown' and brief empty; path 'docs/design/lys-core/stories.json' with note 'the lys-core user stories, carried from main's USER-STORIES.md' and brief 'LYSCORE-001'; path 'docs/design/lys-core/USER-STORIES.md' with note 'rendered markdown' and brief empty; path 'docs/design/lys-core/briefs/LYSCORE-001.json' with note 'the brief that carries the hand-written lys-core design into the three JSON documents' and brief 'LYSCORE-001'; path 'docs/design/lys-core/briefs/LYSCORE-001.md' with note 'rendered markdown' and brief 'LYSCORE-001'); constraints, the nine bullets in main's order as CN1 to CN9, each whole bullet after '- ' as text; principles, decisions and inventory empty; and gate, the project's one tree entry for '.' with its shorthand and its seven legs fmt, clippy-all-features, clippy, tests, doc-all-features, doc and design. It SHALL NOT reword, merge, drop, reorder or add any sentence of main's DESIGN.md, SHALL NOT correct stale carried text, SHALL NOT change any relative link (../../ROADMAP.md, ../WIRE-FORMATS.md, ../../PEN-REGISTRATION.md, ../../REVIEW-23-07.md, DESIGN.md), and SHALL NOT add any principle, ADR, goal, non-goal, constraint, inventory row, structure row beyond the eight named, and SHALL NOT add any word beyond the '### Checklist notes' heading and the eight rows' paths, notes and brief values as stated here. WHEN python3 scripts/design/render-cluster.py docs/design/lys-core runs, THE SYSTEM SHALL write docs/design/lys-core/DESIGN.md, and the build's dev record SHALL list every difference between it and main's DESIGN.md with its reason: six of layout (unquoted title, the added '> **Cluster:** lys-core' line, the moved 'Tests live in sibling' paragraph, goals as bullets, the structure tree as a table, the CN prefixes) and two of content that the card's lead allows (the '### Checklist notes' section, the eight rows for the cluster's own documents and this brief's own files).

**Acceptance:**
- From the repository root: the field comparison in this brief's verification prints 'carried fields 7 differences []'.
- From the repository root: the structure tree comparison in this brief's verification prints '75 75 True True'.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/lys-core/design.json')); s=d['structure']; print(len(s), [r['brief'] for r in s[:75]] == [''] * 75)" prints 83 True.
- From the repository root: python3 -c "import json; print([(r['path'], r['note'], r['brief']) for r in json.load(open('docs/design/lys-core/design.json'))['structure'][75:]] == [('docs/design/lys-core/design.json', \"the lys-core design, carried from main's DESIGN.md\", 'LYSCORE-001'), ('docs/design/lys-core/DESIGN.md', 'rendered markdown', ''), ('docs/design/lys-core/checklist.json', \"the lys-core checklist, carried from main's CHECKLIST.md\", 'LYSCORE-001'), ('docs/design/lys-core/CHECKLIST.md', 'rendered markdown', ''), ('docs/design/lys-core/stories.json', \"the lys-core user stories, carried from main's USER-STORIES.md\", 'LYSCORE-001'), ('docs/design/lys-core/USER-STORIES.md', 'rendered markdown', ''), ('docs/design/lys-core/briefs/LYSCORE-001.json', 'the brief that carries the hand-written lys-core design into the three JSON documents', 'LYSCORE-001'), ('docs/design/lys-core/briefs/LYSCORE-001.md', 'rendered markdown', 'LYSCORE-001')])" prints True, the eight rows' paths, notes and brief values being exactly those stated in the spec, in its order.
- From the repository root: python3 -c "import json; d=json.load(open('docs/design/lys-core/design.json')); print(d['principles'], d['decisions'], d['inventory'], [l['name'] for g in d['gate'] for l in g['legs']])" prints [] [] [] ['fmt', 'clippy-all-features', 'clippy', 'tests', 'doc-all-features', 'doc', 'design'].
- From the repository root: rg -c '^### Checklist notes$' docs/design/lys-core/DESIGN.md prints 1, and rg -c '^\*\(C38–C43 amended|^\*\(C58–C59 superseded' docs/design/lys-core/DESIGN.md prints 2.
- From the repository root: git show 7b536253:docs/design/lys-core/DESIGN.md | diff - docs/design/lys-core/DESIGN.md | grep -c '^<' prints 104, and the same pipeline with grep -c '^>' prints 117.
- The R3 dev record names each of the eight differences listed in the spec, one entry each, with its reason, and names no other difference.

**Files:**
- modify: docs/design/lys-core/design.json
- modify: docs/design/lys-core/DESIGN.md

### R4: Render the brief's own markdown and pass the design gate

WHEN python3 scripts/design/render-cluster.py docs/design/lys-core runs after R1 to R3, THE SYSTEM SHALL write docs/design/lys-core/briefs/LYSCORE-001.md from docs/design/lys-core/briefs/LYSCORE-001.json, and the committed DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md SHALL each be byte-identical to that render. WHEN sh scripts/design/gate.sh runs on the card's final tree, THE SYSTEM SHALL exit 0 with lys-core among the measured clusters. The build SHALL NOT change any file outside docs/design/lys-core, SHALL NOT hand-edit any rendered markdown, and SHALL NOT change the authored fields of briefs/LYSCORE-001.json.

**Acceptance:**
- From the repository root: sh scripts/design/gate.sh exits 0 and prints no line containing 'differs'.
- From the repository root: python3 scripts/design/render-cluster.py docs/design/lys-core prints 'Done. 4 file(s) rendered.', and git status --porcelain docs/design/lys-core prints nothing after it.
- From the repository root: python3 scripts/design/check-coverage.py docs/design/lys-core exits 0 and reports 65 checklist items, 23 user stories and 1 brief.
- From the repository root, on the build branch: git diff --no-ext-diff --name-only <base>..HEAD -- . ':!docs/design/lys-core' prints nothing, where <base> is the commit the build started from.

**Files:**
- create: docs/design/lys-core/briefs/LYSCORE-001.md

## Boundaries

- No change to code: no file under crates/ and no file under scripts/ changes, including scripts/design/render-cluster.py, scripts/design/validate.py, scripts/design/check-coverage.py and scripts/design/schemas/.
- No new checklist item, user story, principle, ADR, goal, non-goal or constraint: the checklist holds C1 to C65 and the stories S1 to S23 and nothing else; the CN ids name the nine constraints main already has.
- No carried text is reworded, merged, dropped, renumbered, reordered or corrected, even where it is stale; reconciling the design with the tree as built is a later unit.
- No done flag changes: C1 to C60 and C65 stay true and C61 to C64 stay false, exactly as main ticks them.
- The eight checklist section names stay exactly as main has them; no checklist note is placed in a section name.
- The directory, home, secrets, identity, lys-anchor and lys-log-store clusters, docs/design/decisions.json, docs/design/project.json and docs/design/WIRE-FORMATS.md are not touched.
- No note slot is added to the renderer and the schemas; that is a change to the shared design tooling and a later unit.
- The later lys-core card's brief, items and stories are not added here; they come after the carried ones once this brief lands.
- Claiming C1 to C65 and S1 to S23 records that this brief carries them into JSON; it does not claim to have built them.

## Verification

- From the repository root: sh scripts/design/gate.sh exits 0.
- From the repository root: python3 scripts/design/validate.py docs/design/lys-core and python3 scripts/design/check-coverage.py docs/design/lys-core both exit 0.
- C and S text comparison, keyed on main's bytes at 7b536253 and counting every id either side holds; from the repository root run the following, which prints 'compared 88 differences 0' as its last line:
python3 - <<'EOF'
import re, subprocess
base = '7b536253'
pats = {'CHECKLIST.md': r'^- \[[ x]\] \*\*(C\d+)\*\* — (.*)$', 'USER-STORIES.md': r'^\*\*(S\d+)\.\*\* (.*)$'}
compared = differences = 0
for name, pat in pats.items():
    old = re.findall(pat, subprocess.run(['git', 'show', base + ':docs/design/lys-core/' + name], capture_output=True, text=True, check=True).stdout, re.M)
    new = re.findall(pat, open('docs/design/lys-core/' + name).read(), re.M)
    old_d, new_d = dict(old), dict(new)
    if len(old_d) != len(old) or len(new_d) != len(new):
        differences += 1
        print('duplicate id in', name)
    for i in sorted(set(old_d) | set(new_d), key=lambda s: int(s[1:])):
        compared += 1
        if old_d.get(i) != new_d.get(i):
            differences += 1
            print('differs:', i)
print('compared', compared, 'differences', differences)
EOF
- Field comparison of design.json against main's DESIGN.md and CHECKLIST.md at 7b536253; from the repository root run the following, which prints 'carried fields 7 differences []':
python3 - <<'EOF'
import json, subprocess
def main_lines(name):
    return subprocess.run(['git', 'show', '7b536253:docs/design/lys-core/' + name], capture_output=True, text=True, check=True).stdout.split('\n')
D, C = main_lines('DESIGN.md'), main_lines('CHECKLIST.md')
d = json.load(open('docs/design/lys-core/design.json'))
notes = '\n'.join(C[2:7]) + '\n\n' + C[59] + '\n\n' + C[88]
checks = {
    'title': d['title'] == D[3][len('title: "'):-1],
    'intention': d['intention'] == '\n'.join(D[10:13]),
    'problem': d['problem'] == '\n'.join(D[16:24]),
    'solution': d['solution'] == '\n'.join(D[27:133]) + '\n\n' + D[242] + '\n\n### Checklist notes\n\n' + notes,
    'goals': d['goals'] == [l.split('. ', 1)[1] for l in D[136:142]],
    'non_goals': d['non_goals'] == [{'text': l[2:], 'reason': ''} for l in D[145:152]],
    'constraints': d['constraints'] == [{'id': 'CN%d' % i, 'text': l[2:]} for i, l in enumerate(D[246:255], 1)],
}
print('carried fields', len(checks), 'differences', [k for k, v in checks.items() if not v])
EOF
- Structure tree comparison, the entry names and every word of the tree's notes against the first 75 structure rows; from the repository root run the following, which prints '75 75 True True':
python3 - <<'EOF'
import json, re, subprocess
t = subprocess.run(['git', 'show', '7b536253:docs/design/lys-core/DESIGN.md'], capture_output=True, text=True, check=True).stdout
fence = t.split('## Structure\n\n```\n')[1].split('\n```\n')[0]
names, words = [], []
for line in fence.split('\n'):
    m = re.match(r'^(crates/\S+)|^[│ ]*[├└]── (\S+)', line)
    if m:
        names.append(m.group(1) or m.group(2))
        words += line.split('— ', 1)[1].split() if '— ' in line else []
    else:
        words += line.replace('│', ' ').split()
rows = json.load(open('docs/design/lys-core/design.json'))['structure'][:75]
print(len(names), len(rows), all(r['path'].endswith(n) for r, n in zip(rows, names)), ' '.join(words) == ' '.join(w for r in rows for w in r['note'].split()))
EOF
- From the repository root: git diff --no-ext-diff --name-only <base>..HEAD -- crates scripts prints nothing, where <base> is the commit the build started from.
- From the repository root: python3 scripts/design/render-cluster.py docs/design/lys-core run a second time changes no file (git status --porcelain docs/design/lys-core prints nothing after it).
