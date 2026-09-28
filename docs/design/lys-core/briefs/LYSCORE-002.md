---
type: brief
id: LYSCORE-002
cluster: lys-core
title: Refuse underscore-prefixed bindings in lys and clear each one by what it does
---

# LYSCORE-002: Refuse underscore-prefixed bindings in lys and clear each one by what it does

> **Cluster:** lys-core
> **Blocked by:** LYSCORE-001 (brief 6747ce61) has landed sgconfig.yml, rules/ast-grep and the `ast-grep scan --config sgconfig.yml` leg on main: `git cat-file -e origin/main:sgconfig.yml` succeeds. The build does not start until it does, and this card carries no copy of them.
> **Design anchor:**
> - ADR-047 — The given statement is a lys/attestation/v2 over the RFC 8785 bytes of lys.given's data, written only when render-launch is given a key — The given statement is lys-core's existing lys/attestation/v2 over the RFC 8785 (JSON Canonicalization Scheme) bytes of the lys.given entry's data, so its signed payload hash is the given hash render-launch reports as given_sha256. It is written only when render-launch is given a key: kept as a block named by a lys.given_statement entry hung under the lys.given entry, and as given-statement.cose and given-data.json under --out for lys verify. Rejected: serialising the record in struct field order, which a stranger cannot rebuild without the Rust type; signing the 64-hex hash string, whose attested hash would be SHA-256 of the hex and not the given hash; adding the given hash to the template_render event, which HOME-003 keeps unaltered; and changing lys verify's one failure message to name the file, which would change the published lys for every user.
> - ADR-048 — A trait parameter one implementation ignores is a bare `_` where the trait cannot drop it — An implementation that has no use for a parameter which another implementation of the same trait needs, or which a published trait asks for, writes the bare `_` pattern. The bare `_` binds nothing, so it is the words' 'not bound at all', and it is not the underscore-prefixed name the rule refuses. The trait is not changed. Rejected: changing AdmissionPolicy or LeafStore, adding a use to a parameter the implementation does not need, and keeping an underscore-prefixed name.
> **Checklist:**
> - C75 — rules/ast-grep/no-underscore-binding.yml holds one rule, id no-underscore-binding, at severity error with vendor/** ignored, whose message and note say it refuses because a leading underscore silences the unused warning instead of fixing its cause.
> - C76 — On an uncommitted scratch file with one underscore-prefixed binding in each of the 19 binding positions, the scan reports exactly 19 no-underscore-binding hits, and none on a bare `_`, a struct field declaration, a field initialiser or a field access.
> - C77 — `ast-grep scan --config sgconfig.yml` reports zero hits at the repository root of the landed tree.
> - C78 — Every guard whose directory or state the test still uses after the binding (a TempDir holder or an EnvCleanup guard) has a name without a leading underscore and is ended by drop(name) after its last use, with a tail result bound, the guard dropped and the binding returned.
> - C79 — Every discarded error in a closure or match arm is left unbound, map_err is rewritten as .ok().ok_or(…) when the error is already built and .ok().ok_or_else(…) when building it calls a function or a macro, and each site returns the same error as before.
> - C80 — Every value with no use is not bound: a bare `_` inside its pattern, including a TempDir a fixture returns beside a value it has already loaded into memory, an expression statement for a whole let, and no `let _ =` statement introduced.
> - C81 — Every trait implementation that ignores a parameter takes it as a bare `_`, and no trait declaration changes.
> - C82 — The two cfg(not(unix)) stubs, fsync_dir and warn_if_loose_permissions, take their parameter as a bare `_`.
> - C83 — cargo test --workspace --all-features exits 0 at the commit the build starts from and at its final commit, with the same number of tests passed at both.
> - C84 — Each of the eight test fixture structs that holds a TempDir guard names the field temp_dir and has a close method returning TempDir::close's error, and every test that builds one calls its close after the value's last use.
> **Stories:**
> - S24 (Card author, Landing work in lys) — As a card author landing work in lys, I want the gate to refuse an underscore-prefixed binding with a message saying why so that an unused warning is fixed at its cause rather than silenced.
> - S25 (Test writer, Writing tests in sibling files) — As a test writer, I want a temporary directory's guard named and dropped by name so that I can see where the directory's life ends.
> - S26 (Third-party verifier, Verifying lys artifacts) — As a third party verifying a lys artifact, I want every verification failure to keep returning the one uniform error so that the cleanup reveals nothing about which check failed.
> - S27 (Consumer of lys-log-store, Implementing LeafStore) — As a consumer implementing lys-log-store's LeafStore, I want the trait left unchanged so that my implementation still compiles against the next release.
> - S28 (Lead, Trusting the gate) — As the lead for lys, I want the rule shown to fire once for each binding position so that zero hits on the tree means the rule held and not that nothing was measured.
> - S29 (Lead, Trusting the gate) — As the lead for lys, I want the full suite to run the same tests before and after the card so that I know the cleanup changed no behaviour.

## Purpose

Turn CLAUDE.md's sentence that a _-prefixed unused variable is a bypass into a gate, and leave the tree with no such binding. The design's solution says how the rule sits beside LYSCORE-001's rule set and how each binding is cleared by what it does. The words counted 272 bindings; this card's rule counts 303 at 7b53625 in 70 files, and the acceptance is zero hits on the card's final tree, whatever the count on the tree the build starts from.

## Task

Clear every underscore-prefixed binding the rule reports, then add the rule. Guards whose directory or state the test still uses (R1) keep a real name and end with drop(name); a holder is classified by what happens after the binding, not by its name. The eight fixture structs that hold a TempDir in an underscore-prefixed field (R2) name it temp_dir and are closed by every test that builds one. Ignored trait parameters (R3) and the two non-unix stub parameters (R4) become a bare `_`. Discarded errors (R5) and unused values (R6), among them every TempDir a fixture returns beside a value it has already loaded into memory, are left unbound and return the same error or value as before. The rule (R7) is proved on scratch files and lands with zero hits, and the suite (R8) runs the same number of tests before and after. The counts in each requirement are measured at 7b53625. The build starts from main after LYSCORE-001 (brief 6747ce61) has landed, so re-measure with the rule on that commit, and take every before-and-after comparison against that commit: LYSCORE-001 may already have removed some sites, among them the EnvCleanup guards and their environment mutation. Clear whatever the rule reports there by the same requirement its kind falls under. In scope: every underscore-prefixed binding under crates/, the eight fixture guard fields, and the one rule file. Out of scope: PhantomData's `_marker` field; every trait declaration; sgconfig.yml, the leg and the three places the gate is kept, which LYSCORE-001 owns; Cargo.toml's lint table; Cambium. Heavy builds and full gate runs go to the gate venue that declares the rust-build and tool:ast-grep requirements.

## Requirements

### R1: Name each guard whose directory or state is still used and drop it by name where its test ends

WHEN a test binds a value that keeps something alive which the test still uses after the binding (a tempfile::TempDir whose directory holds a file, store or binary that a later statement reads, or an EnvCleanup guard whose drop resets the environment a later statement reads), THE SYSTEM SHALL bind it to a name without a leading underscore and SHALL end it with a `drop(name);` statement placed after the last statement that uses anything living in the directory or depending on the guard. In a test with no tail expression the drop is the test's last statement. In a function whose tail expression is its result, THE SYSTEM SHALL bind that result to a name, write `drop(name);`, and return the binding as the tail: `let result = <tail expression>; drop(name); result`. It SHALL NOT bind such a guard to a bare `_`, which drops it at the end of its own statement and deletes the directory under the test. It SHALL NOT move the guard into a struct, SHALL NOT change a fixture function's signature or return type, and SHALL NOT end a guard binding by any statement other than `drop(name);`. The name is the old name without its underscore (`_dir` becomes `dir`, `_control_dir` becomes `control_dir`, `_workdir` becomes `workdir`, `_bin_dir` becomes `bin_dir`, `_guard` becomes `guard`). Where that name is already bound in the same scope, the guard takes a name that says whose directory it holds. A holder is classified by what happens after the binding, not by its name. At 7b53625 the rule reports 59 guards whose directory or state is still used, in the 11 files listed: the TempDir from staged() in lys-anchor witness/observe_tests.rs (10, the Anchor's FileLeafStore lives in it), from fixture_home(), lit_fixture() and recall_fixture() in lys-home record/*_tests.rs (37, the Home's root is the directory), from two_entries() in lys-home tests/cached_index.rs (4, the returned paths are inside it), from cose_tool() in lys-core tests/delegation_conformance.rs (2, the built binary is inside it) and the bin_dir from cose_tool() in lys-anchor tests/anchor_receipt_conformance.rs (2, the built binary is inside it), and the four EnvCleanup guards in crates/lys-core/src/keys/identity_tests.rs, which are handled the same way wherever LYSCORE-001 has not already removed them. Every one of the 59 sits in a #[test] function with no tail expression at 7b53625. A TempDir returned beside a value the fixture has already loaded into memory keeps nothing the test uses alive, and falls under R6.

**Acceptance:**
- The scan `ast-grep scan --config sgconfig.yml` reports 0 no-underscore-binding hits in each of the 11 files this requirement modifies.
- In crates/lys-anchor/src/witness/observe_tests.rs, every test that at the commit the build starts from begins `let (_dir, mut anchor) = staged();` begins `let (dir, mut anchor) = staged();` at the card's final commit and holds exactly one `drop(dir);` statement, placed after its last use of `anchor`.
- In crates/lys-anchor/tests/anchor_receipt_conformance.rs, both `let (_gocache_dir, _bin_dir, bin) = cose_tool(&go);` lines read `let (_, bin_dir, bin) = cose_tool(&go);`, and each of the two tests holds exactly one `drop(bin_dir);` after its last use of `bin`.
- `git diff <the commit the build starts from> -- <each of the 11 files>` shows no change to the signature line of staged, cose_tool, fixture_home, lit_fixture, recall_fixture or two_entries.
- crates/lys-core/src/keys/identity_tests.rs holds no `_guard` identifier, and every `let guard = EnvCleanup;` it holds is followed later in the same test by exactly one `drop(guard);`.
- The number of `drop(` statements the card adds in the 11 files equals the number of guards whose directory or state is still used that the rule reports there on the commit the build starts from (59 measured at 7b53625), and none of those guards is bound to a bare `_`.

**Files:**
- modify: crates/lys-anchor/src/witness/observe_tests.rs
- modify: crates/lys-anchor/tests/anchor_receipt_conformance.rs
- modify: crates/lys-core/src/keys/identity_tests.rs
- modify: crates/lys-core/tests/delegation_conformance.rs
- modify: crates/lys-home/src/record/epilogue_tests.rs
- modify: crates/lys-home/src/record/fork_cut_tests.rs
- modify: crates/lys-home/src/record/fork_tests.rs
- modify: crates/lys-home/src/record/lantern_tests.rs
- modify: crates/lys-home/src/record/reader_tests.rs
- modify: crates/lys-home/src/record/recall_tests.rs
- modify: crates/lys-home/tests/cached_index.rs

**Checklist:**
- C78 — Every guard whose directory or state the test still uses after the binding (a TempDir holder or an EnvCleanup guard) has a name without a leading underscore and is ended by drop(name) after its last use, with a tail result bound, the guard dropped and the binding returned.

**Stories:**
- S25 (Test writer, Writing tests in sibling files) — As a test writer, I want a temporary directory's guard named and dropped by name so that I can see where the directory's life ends.

### R2: Name each fixture's directory guard field and close it where the test ends

WHEN a test fixture struct holds a tempfile::TempDir only to keep its directory alive, THE SYSTEM SHALL name that field `temp_dir` and SHALL give the struct a method `fn close(self) -> std::io::Result<()>` that calls `self.temp_dir.close()` and returns its result. WHEN a test has made its last use of a value of such a struct, built directly or held inside another value, THE SYSTEM SHALL call that value's `close()` and unwrap its result, so a directory that cannot be removed fails the test. This applies to eight structs: OpensslRequest in crates/lys-core/tests/openssl_csr_interop.rs; Party in crates/lys-core/tests/bundle_conformance.rs; Party in crates/lys-core/src/bundle/verify_tests.rs; ProvenLog in crates/lys/tests/log_tests.rs; Party in crates/lys-anchor/tests/cascade.rs; Fixture in crates/lys-anchor-cli/tests/anchor_cli.rs; Case in crates/lys-anchor/tests/stranger_verification.rs; Node in crates/lys-anchor/src/upward/fixture.rs, whose values are also built in crates/lys-anchor/src/upward/pin_tests.rs and crates/lys-anchor/src/upward/bundle_tests.rs. It SHALL NOT keep an underscore-prefixed field in any of the eight structs, SHALL NOT silence the field's dead_code warning with #[allow] or #[expect], SHALL NOT leave a fixture value to be dropped implicitly at the end of its test, and SHALL NOT discard the error close returns. It SHALL NOT rename or change PhantomData's `_marker` field in crates/lys-core/src/merkle/tree.rs, which is a type marker and not a held value; the marker field is kept as it is, and nothing else in tree.rs is promised unchanged, since R5 rewrites its two map_err closures.

**Acceptance:**
- `grep -nE '^\s*(pub )?_[A-Za-z0-9_]*\s*:' crates/lys-core/tests/openssl_csr_interop.rs crates/lys-core/tests/bundle_conformance.rs crates/lys-core/src/bundle/verify_tests.rs crates/lys/tests/log_tests.rs crates/lys-anchor/tests/cascade.rs crates/lys-anchor-cli/tests/anchor_cli.rs crates/lys-anchor/tests/stranger_verification.rs crates/lys-anchor/src/upward/fixture.rs` prints nothing.
- Each of the eight structs declares the field `temp_dir: TempDir` (written `temp_dir: tempfile::TempDir` where the file does not import TempDir) and has exactly one method named `close`, taking `self` and returning `std::io::Result<()>`, whose body is `self.temp_dir.close()`.
- For each of the eight structs in turn, an uncommitted edit replacing its close body with `panic!("close reached")` makes `cargo test --all-features` for that struct's crate fail every test whose body reaches one of the struct's constructors, counting calls made through helper functions, and no other test; the number of failed tests equals the number of such tests and is at least 1. With the edit reverted, `git status --porcelain crates` prints nothing.
- The line `    _marker: PhantomData<fn(L)>,` is present unchanged in crates/lys-core/src/merkle/tree.rs at the card's final commit, and `git diff <the commit the build starts from> -- crates/lys-core/src/merkle/tree.rs` shows no added or removed line containing `_marker`.

**Files:**
- modify: crates/lys-anchor-cli/tests/anchor_cli.rs
- modify: crates/lys-anchor/src/upward/bundle_tests.rs
- modify: crates/lys-anchor/src/upward/fixture.rs
- modify: crates/lys-anchor/src/upward/pin_tests.rs
- modify: crates/lys-anchor/tests/cascade.rs
- modify: crates/lys-anchor/tests/stranger_verification.rs
- modify: crates/lys-core/src/bundle/verify_tests.rs
- modify: crates/lys-core/tests/bundle_conformance.rs
- modify: crates/lys-core/tests/openssl_csr_interop.rs
- modify: crates/lys/tests/log_tests.rs

**Checklist:**
- C84 — Each of the eight test fixture structs that holds a TempDir guard names the field temp_dir and has a close method returning TempDir::close's error, and every test that builds one calls its close after the value's last use.

**Stories:**
- S25 (Test writer, Writing tests in sibling files) — As a test writer, I want a temporary directory's guard named and dropped by name so that I can see where the directory's life ends.

### R3: Take an ignored trait parameter as a bare `_`, leaving every trait unchanged

WHEN a trait implementation has no use for a parameter that another implementation of the same trait needs, or that a published trait asks for, THE SYSTEM SHALL write that parameter as a bare `_` with its type unchanged. This applies to AcceptAll::admit (both parameters), MaxSize::admit (context) and the certificate policy's admit (submission) under AdmissionPolicy; LyingStore's leaf, put_leaf and pin under the published LeafStore; DecliningSigner's sign under Signer; and RecordPolicy's run under AnchorTask. It SHALL NOT change the AdmissionPolicy, LeafStore, Signer or AnchorTask declaration. It SHALL NOT add a use of a parameter that the implementation does not need. It SHALL NOT write any parameter under an underscore-prefixed name.

**Acceptance:**
- `git diff <the commit the build starts from> -- crates/lys-anchor/src/admission/policy.rs crates/lys-log-store/src/store.rs crates/lys-anchor/src/keys/signer.rs crates/lys-anchor-cli/src/commands/anchor/policy.rs` prints nothing.
- In crates/lys-log-store/src/log_tests.rs, LyingStore's methods read `fn leaf(&self, _: u64)`, `fn put_leaf(&mut self, _: u64, _: &[u8])` and `fn pin(&mut self, _: PinnedRoot)`.
- In crates/lys-anchor/src/admission/trivial.rs, AcceptAll's admit takes `_: &Submission<'_>, _: &SubmitterContext<'_>` and MaxSize's admit takes `_: &SubmitterContext<'_>` as its context parameter; in crates/lys-anchor/src/admission/certificate.rs, admit takes `_: &Submission<'_>`.
- In crates/lys-anchor/src/anchor/genesis_tests.rs, DecliningSigner reads `fn sign(&self, _: &[u8])`; in crates/lys-anchor-cli/src/commands/anchor/policy_tests.rs, RecordPolicy reads `fn run<P: AdmissionPolicy>(self, _: P)`.
- The scan reports 0 no-underscore-binding hits in the five files this requirement modifies.

**Files:**
- modify: crates/lys-anchor-cli/src/commands/anchor/policy_tests.rs
- modify: crates/lys-anchor/src/admission/certificate.rs
- modify: crates/lys-anchor/src/admission/trivial.rs
- modify: crates/lys-anchor/src/anchor/genesis_tests.rs
- modify: crates/lys-log-store/src/log_tests.rs

**Checklist:**
- C81 — Every trait implementation that ignores a parameter takes it as a bare `_`, and no trait declaration changes.

**Stories:**
- S27 (Consumer of lys-log-store, Implementing LeafStore) — As a consumer implementing lys-log-store's LeafStore, I want the trait left unchanged so that my implementation still compiles against the next release.

### R4: Take the non-unix stubs' parameters as a bare `_`

The two #[cfg(not(unix))] stubs take their one parameter as a bare `_` with its type unchanged: `fn fsync_dir(_: &Path) -> StoreResult<()>` in crates/lys-log-store/src/file.rs and `fn warn_if_loose_permissions(_: &Path) {}` in crates/lys-core/src/keys/identity.rs. Their bodies, attributes and callers do not change, and their unix counterparts do not change. No gate host compiles non-unix code, so the change is checked by rustfmt, which parses both stubs whatever the target, and by the scan. No non-unix build target or leg is added.

**Acceptance:**
- The #[cfg(not(unix))] fsync_dir in crates/lys-log-store/src/file.rs reads `fn fsync_dir(_: &Path) -> StoreResult<()> {` and its body is `Ok(())`, unchanged.
- The #[cfg(not(unix))] warn_if_loose_permissions in crates/lys-core/src/keys/identity.rs reads `fn warn_if_loose_permissions(_: &Path) {}`.
- `cargo fmt --all --check` exits 0.
- The #[cfg(unix)] fsync_dir in crates/lys-log-store/src/file.rs and the #[cfg(unix)] warn_if_loose_permissions in crates/lys-core/src/keys/identity.rs are byte-identical before and after the card.

**Files:**
- modify: crates/lys-core/src/keys/identity.rs
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C82 — The two cfg(not(unix)) stubs, fsync_dir and warn_if_loose_permissions, take their parameter as a bare `_`.

**Stories:**
- S24 (Card author, Landing work in lys) — As a card author landing work in lys, I want the gate to refuse an underscore-prefixed binding with a message saying why so that an unused warning is fixed at its cause rather than silenced.

### R5: Leave every discarded error unbound and return the same error

WHEN code turns a failure into its own error and discards the original error, THE SYSTEM SHALL leave the discarded error unbound. A `.map_err(|_x| E)` becomes `.ok().ok_or(E)` when E's expression holds no function call, method call or macro invocation (a unit variant or a constant). It becomes `.ok().ok_or_else(F)` when E's expression holds one, such as `format!`, `.to_string()` or `reject()`, where F is a no-argument closure whose body is the old closure's body, or the called function itself when that body was a single call with no arguments (`reject()` becomes `ok_or_else(reject)`). An or_else closure writes `|_|`, and a match arm writes `Err(_)`. No lint the workspace enables checks this split, so the acceptance below measures it with an inline ast-grep rule. Every site SHALL return the same error, with the same variant, fields and message, as before. It SHALL NOT bind a discarded error under any name, and SHALL NOT pass `.ok_or(` an error expression that holds a call or a macro invocation. It SHALL NOT retire, relax or #[allow] clippy::map_err_ignore. It SHALL NOT merge, split or reorder the checks. On the verification paths in crates/lys-core/src/tlog/verify.rs, crates/lys-core/src/bundle/verify.rs and crates/lys-core/src/checkpoint/note.rs the discarded errors are not carried into the returned error, because a verifier must not say which check failed: every failure there returns the one uniform error it returns today. A closure whose error has a use carries it into the error it returns. At 7b53625 no site's returned error can take the discarded error without changing the error a caller sees, so no site carries one. The rule reports 68 such sites in the 28 files listed: 64 map_err closures, one or_else closure in keys/identity.rs, one match arm in keys/identity.rs and two match arms in merkle/consistency_tests.rs.

**Acceptance:**
- `grep -rnE '\|_[A-Za-z0-9]' crates --include='*.rs'` prints nothing.
- Cargo.toml still holds the line `map_err_ignore = "warn"`.
- In crates/lys-core/src/checkpoint/note.rs, parse_note's UTF-8 check reads `std::str::from_utf8(note_bytes).ok().ok_or(TrustError::NoteVerification)?`, and `verify_note(&[0xff], &verifier)` returns `Err(TrustError::NoteVerification)` for any verifier key.
- In crates/lys-core/src/bundle/verify.rs, the leaf decode reads `STANDARD.decode(&bundle.leaf).ok().ok_or_else(reject)?`.
- In crates/lys-core/src/tlog/verify.rs, each of the eight rewritten calls ends `.ok().ok_or(TrustError::LogArtifactVerification)?`.
- crates/lys-core/src/keys/identity.rs reads `.or_else(|_| STANDARD.decode(trimmed))` and `Err(_) => false`; crates/lys-core/src/merkle/consistency_tests.rs's two arms read `Err(_) => {}`.
- For each of the 64 map_err sites, the error expression in the rewritten call is the old closure's body unchanged, or the function the body called with no arguments: `git diff <the commit the build starts from>` shows no edited variant name, field, format string or argument inside any of them.
- `ast-grep scan --inline-rules '{id: eager-error, language: rust, rule: {pattern: $R.ok().ok_or($E)}, constraints: {E: {any: [{kind: call_expression}, {kind: macro_invocation}, {has: {stopBy: end, any: [{kind: call_expression}, {kind: macro_invocation}]}}]}}}' crates` reports 0 matches at the card's final commit, and the same command over an uncommitted scratch file holding exactly `fn s(x: Result<u8, ()>) -> Option<u8> { x.ok().ok_or(String::new()).ok() }` reports exactly 1 match.
- Each of these rewritten sites chains `.ok()` and then `.ok_or_else(|| …)`, the closure's body being the old closure's body: in crates/lys-core/src/ca/request.rs the two CertificateParsing length checks (subject public key, signature) and the two CertificateVerification checks (subject key point, proof-of-possession signature); in crates/lys-core/src/ca/authority.rs the three CertificateVerification checks (signature length, issuer key point, signature verification); in crates/lys-core/src/ca/certificate.rs the subject key length CertificateParsing check; in crates/lys-core/src/tlog/build.rs the two LogArtifactEncoding self-verification checks; and in crates/lys-core/src/checkpoint/body.rs the root hash length CheckpointParsing check.
- The scan reports 0 no-underscore-binding hits in the 28 files this requirement modifies.

**Files:**
- modify: crates/lys-anchor/src/admission/certificate.rs
- modify: crates/lys-anchor/src/anchor/append.rs
- modify: crates/lys-anchor/src/anchor/proof_nodes.rs
- modify: crates/lys-core/src/attestation/encoding.rs
- modify: crates/lys-core/src/bundle/verify.rs
- modify: crates/lys-core/src/ca/authority.rs
- modify: crates/lys-core/src/ca/certificate.rs
- modify: crates/lys-core/src/ca/extensions.rs
- modify: crates/lys-core/src/ca/request.rs
- modify: crates/lys-core/src/checkpoint/body.rs
- modify: crates/lys-core/src/checkpoint/note.rs
- modify: crates/lys-core/src/delegation/encoding.rs
- modify: crates/lys-core/src/delegation/sign.rs
- modify: crates/lys-core/src/keys/identity.rs
- modify: crates/lys-core/src/merkle/consistency.rs
- modify: crates/lys-core/src/merkle/consistency_tests.rs
- modify: crates/lys-core/src/merkle/tree.rs
- modify: crates/lys-core/src/receipt/consistency.rs
- modify: crates/lys-core/src/receipt/encoding.rs
- modify: crates/lys-core/src/receipt/sign.rs
- modify: crates/lys-core/src/seal/authenticated.rs
- modify: crates/lys-core/src/seal/sealed_envelope.rs
- modify: crates/lys-core/src/tlog/build.rs
- modify: crates/lys-core/src/tlog/verify.rs
- modify: crates/lys-log-store/src/file.rs
- modify: crates/lys/src/commands/duration.rs
- modify: crates/lys/src/commands/log/verify.rs
- modify: crates/lys/src/commands/seal.rs

**Checklist:**
- C79 — Every discarded error in a closure or match arm is left unbound, map_err is rewritten as .ok().ok_or(…) when the error is already built and .ok().ok_or_else(…) when building it calls a function or a macro, and each site returns the same error as before.

**Stories:**
- S26 (Third-party verifier, Verifying lys artifacts) — As a third party verifying a lys artifact, I want every verification failure to keep returning the one uniform error so that the cleanup reveals nothing about which check failed.

### R6: Leave every value with no use unbound

WHEN a pattern binds a value that nothing reads, THE SYSTEM SHALL write a bare `_` in that position of the pattern, for example `let (ok, _) = run_built_tool(…)` and `for (non_canonical, _) in non_canonical_spellings()`. WHEN a fixture returns a tempfile::TempDir beside a value it has already loaded into memory, and nothing after the binding reads the directory, THE SYSTEM SHALL write a bare `_` in the TempDir's position of the tuple pattern: `let (_, identity) = golden_identity();`, `let (_, root) = identity(&ROOT_SEED);`, `let (ca, _) = authority(…);`, and likewise for recipient_identity, test_authority, receipt_identity, anchor_identity and anchor, whose TempDir only held a seed file already read into an Ed25519Identity or a CertificateAuthority; the same holds for the gocache_dir cose_tool returns in lys-anchor tests/anchor_receipt_conformance.rs, which nothing reads once the tool is built. WHEN a whole `let _x = expr;` binds a value nothing reads and is not a guard, THE SYSTEM SHALL write the expression statement `expr;`. It SHALL NOT write `let _ = expr;`. It SHALL NOT bind the value under another name to use it in an assertion or a comment, SHALL NOT drop a call that has an effect, and SHALL NOT reorder statements. In crates/lys-core/src/seal/sealed_envelope.rs, `let (key_bytes, _derived_nonce) = derive_key_and_nonce(` becomes `let (key_bytes, _) = derive_key_and_nonce(`. The comment that follows, explaining why the envelope's own nonce is the operative one and the derived nonce is not compared, is kept byte-identical. Because sealed_envelope.rs is a cryptographic file, this change gets the adversarial review CLAUDE.md requires, even though it alters no behaviour. At 7b53625 the rule reports 164 such values in the 34 files listed: 4 in library code (sealed_envelope.rs, lys-log-store log.rs, and lys's commands/log/prove.rs) and 160 in test code, of which 105 are fixture TempDirs (57 from golden_identity in 8 files, 13 from authority, 12 from anchor, 8 from delegation_vector's identity, 5 from delegation_conformance's identity, 3 from test_authority, 2 from recipient_identity, 2 from anchor_identity, 1 from receipt_identity and 2 gocache_dir). It SHALL NOT change the signature line of any of these fixtures.

**Acceptance:**
- crates/lys-core/src/seal/sealed_envelope.rs reads `let (key_bytes, _) = derive_key_and_nonce(`, and `git diff <the commit the build starts from> -- crates/lys-core/src/seal/sealed_envelope.rs` changes no comment line.
- crates/lys/src/commands/log/prove.rs reads `let (old_root, _) = old_tree.root().to_parts();` and `let (new_root, _) = log.tree().root().to_parts();`; crates/lys-log-store/src/log.rs reads `let (prefix_root, _) = prefix.root().to_parts();`.
- crates/lys-home/src/record/record_tests.rs's test reopening_restores_the_persisted_head_not_the_last_entry holds the statements `s.append(message("assistant", "b")).unwrap();` and `s.append(message("user", "c")).unwrap();` in the same order as before, between the append of `a` and `s.move_head(Some(&a))`.
- crates/lys-core/src/keys/identity_tests.rs's test load_or_generate_creates_parent_dir holds the statement `Ed25519Identity::load_or_generate(&path).unwrap();` before `assert!(path.exists());`.
- `grep -rnE '^\s*let _ =' crates --include='*.rs'` prints the same lines at the final commit as at the commit the build starts from.
- At the card's final commit `grep -c 'let (_, identity) = golden_identity();'` summed over the eight files that define golden_identity is 57, the same as the count of `let (_dir, identity) = golden_identity();` at the commit the build starts from, and no golden_identity call binds a name in the TempDir's position.
- `git diff <the commit the build starts from> -- <each of the 34 files>` shows no change to the signature line of golden_identity, identity, recipient_identity, test_authority, receipt_identity, anchor_identity, anchor, authority or cose_tool, and adds no `drop(` statement in any of the 30 files this requirement modifies that R1 does not also modify.
- The scan reports 0 no-underscore-binding hits in the 34 files this requirement modifies.

**Files:**
- modify: crates/lys-anchor/src/admission/certificate_tests.rs
- modify: crates/lys-anchor/src/keys/file_signer_tests.rs
- modify: crates/lys-anchor/src/upward/pin_tests.rs
- modify: crates/lys-anchor/tests/anchor_receipt_conformance.rs
- modify: crates/lys-anchor/tests/checkpoint_note_conformance.rs
- modify: crates/lys-core/src/attestation/artifact_tests.rs
- modify: crates/lys-core/src/attestation/encoding_tests.rs
- modify: crates/lys-core/src/checkpoint/note_tests.rs
- modify: crates/lys-core/src/keys/identity_tests.rs
- modify: crates/lys-core/src/merkle/consistency_tests.rs
- modify: crates/lys-core/src/merkle/proof_tests.rs
- modify: crates/lys-core/src/merkle/tree_tests.rs
- modify: crates/lys-core/src/receipt/consistency_tests.rs
- modify: crates/lys-core/src/seal/authenticated_tests.rs
- modify: crates/lys-core/src/seal/sealed_envelope.rs
- modify: crates/lys-core/src/tlog/build_tests.rs
- modify: crates/lys-core/src/tlog/verify_tests.rs
- modify: crates/lys-core/tests/consistency_conformance.rs
- modify: crates/lys-core/tests/consistency_receipt_conformance.rs
- modify: crates/lys-core/tests/cose_conformance.rs
- modify: crates/lys-core/tests/delegation_conformance.rs
- modify: crates/lys-core/tests/delegation_vector.rs
- modify: crates/lys-core/tests/go_conformance.rs
- modify: crates/lys-core/tests/openssl_csr_interop.rs
- modify: crates/lys-core/tests/receipt_conformance.rs
- modify: crates/lys-core/tests/seal_derivation.rs
- modify: crates/lys-core/tests/signed_note_crosscheck.rs
- modify: crates/lys-home/src/record/record_tests.rs
- modify: crates/lys-home/tests/cached_index.rs
- modify: crates/lys-log-store/src/log.rs
- modify: crates/lys-log-store/src/log_tests.rs
- modify: crates/lys/src/commands/log/prove.rs
- modify: crates/lys/tests/cli_tests.rs
- modify: crates/lys/tests/log_tests.rs

**Checklist:**
- C80 — Every value with no use is not bound: a bare `_` inside its pattern, including a TempDir a fixture returns beside a value it has already loaded into memory, an expression statement for a whole let, and no `let _ =` statement introduced.

**Stories:**
- S24 (Card author, Landing work in lys) — As a card author landing work in lys, I want the gate to refuse an underscore-prefixed binding with a message saying why so that an unused warning is fixed at its cause rather than silenced.

### R7: Add the no-underscore-binding rule and prove it fires

rules/ast-grep/no-underscore-binding.yml holds one rule: id no-underscore-binding, language rust, severity error, `ignores: [vendor/**]`, no `files` entry. It reports an identifier matching `^_.` in every binding position: a let pattern, a function or closure parameter, a match arm, a for pattern, an if-let or while-let pattern, and the tuple, tuple-struct (excluding its path), struct (field pattern and shorthand), slice, or, ref, mut, reference and @ patterns inside them. It does not report a bare `_`, a struct field declaration, a field initialiser or a field access. Its message reads exactly: `Underscore-prefixed binding: the leading underscore silences the unused warning instead of fixing its cause.`. Its note reads exactly: `CLAUDE.md, Coding standards: a _-prefixed unused variable is a bypass, not a fix. Name a guard and drop it by name where its life should end; write a bare `_` where a value has no use; a trait parameter one implementation has no use for, which another implementation needs or a published trait asks for, is a bare `_`.`. The existing leg `ast-grep scan --config sgconfig.yml` runs it through sgconfig.yml's ruleDirs. WHEN the scan runs over a tree holding an underscore-prefixed binding, THE SYSTEM SHALL exit non-zero and report the binding under the id no-underscore-binding. It SHALL NOT report a bare `_`, and SHALL NOT be cleared by any ignores or files entry beyond vendor/**. The rule is shown to fire on uncommitted scratch files before its zero hits on the tree are trusted, and the scratch files are never committed.

**Acceptance:**
- rules/ast-grep/no-underscore-binding.yml parses as YAML with `id: no-underscore-binding`, `language: rust`, `severity: error` and `ignores` equal to the single entry `vendor/**`.
- An uncommitted file crates/lys-core/src/underscore_scratch.rs holding exactly `fn scratch(_: u8, _unused: u8) {}` makes `ast-grep scan --config sgconfig.yml` exit 1 with exactly one hit in total: rule no-underscore-binding at line 1 on `_unused`, and no hit on the bare `_`.
- That hit's printed message is `Underscore-prefixed binding: the leading underscore silences the unused warning instead of fixing its cause.`.
- An uncommitted file crates/lys-core/src/underscore_scratch.rs holding these lines (` / ` marks a line break): `struct Holder { _marker: u8, n: u8 } / struct Pair(u8, u8); / fn params(_a: u8, _: u8) {} / fn all(h: Holder, v: Vec<u8>, o: Option<u8>, p: Pair) { let _b = 1; let (_c, _) = (1, 2); let f = |_d| 0; let g = |_e: u8, _| 0; match o { Some(_f) => {}, None => {} } for _g in &v {} if let Some(_h) = o {} while let Some(_i) = o {} let Holder { _marker, .. } = h; let Holder { n: _j, .. } = Holder { _marker: 0, n: 0 }; let [_k, ..] = [1u8, 2]; let ref _l = 1; let mut _m = 1; let _n @ 1..=2 = 1u8; let &_o = &1u8; let Pair(_p, _) = p; let _q: u8 = 1; match o { Some(1) | Some(_r) => {}, _ => {} } let z = Holder { _marker: 1, n: 1 }; let w = z._marker; }` makes `ast-grep scan --config sgconfig.yml --json=stream` report exactly 19 hits with ruleId no-underscore-binding in that file, one each on _a, _b, _c, _d, _e, _f, _g, _h, _i, _marker (the shorthand pattern on the `let Holder { _marker, .. } = h;` line), _j, _k, _l, _m, _n, _o, _p, _q and _r. It reports none on the struct field `_marker: u8`, the initialiser `_marker: 0`, the initialiser `_marker: 1` or the access `z._marker`.
- With both scratch files removed, `git status --porcelain crates` prints nothing and `ast-grep scan --config sgconfig.yml` at the repository root exits 0 with zero hits.
- `git diff <the commit the build starts from> -- sgconfig.yml docs/design/project.json .land/gates.sh .github/workflows/ci.yml` prints nothing.

**Files:**
- create: rules/ast-grep/no-underscore-binding.yml

**Checklist:**
- C75 — rules/ast-grep/no-underscore-binding.yml holds one rule, id no-underscore-binding, at severity error with vendor/** ignored, whose message and note say it refuses because a leading underscore silences the unused warning instead of fixing its cause.
- C76 — On an uncommitted scratch file with one underscore-prefixed binding in each of the 19 binding positions, the scan reports exactly 19 no-underscore-binding hits, and none on a bare `_`, a struct field declaration, a field initialiser or a field access.
- C77 — `ast-grep scan --config sgconfig.yml` reports zero hits at the repository root of the landed tree.

**Stories:**
- S24 (Card author, Landing work in lys) — As a card author landing work in lys, I want the gate to refuse an underscore-prefixed binding with a message saying why so that an unused warning is fixed at its cause rather than silenced.
- S28 (Lead, Trusting the gate) — As the lead for lys, I want the rule shown to fire once for each binding position so that zero hits on the tree means the rule held and not that nothing was measured.

### R8: Show the suite runs the same tests before and after

WHEN the card's build starts, THE SYSTEM SHALL run `cargo test --workspace --all-features` on the commit it starts from and record the sum of the `N passed` counts over every `test result:` line. WHEN the card's final commit is built, THE SYSTEM SHALL run the same command and record the same sum. The two sums SHALL be equal and both runs SHALL report 0 failed. No test SHALL be added, removed, renamed or #[ignore]d to reach that equality.

**Acceptance:**
- `cargo test --workspace --all-features` exits 0 at the commit the build starts from and at the card's final commit.
- The sum of `N passed` over every `test result:` line is the same number at both commits, and every `test result:` line at both reports `0 failed`.
- `git diff <the commit the build starts from> -- crates` adds no `#[test]`, `#[ignore]`, `#[allow` or `#[expect` line and removes no `#[test]` line.

**Checklist:**
- C83 — cargo test --workspace --all-features exits 0 at the commit the build starts from and at its final commit, with the same number of tests passed at both.

**Stories:**
- S29 (Lead, Trusting the gate) — As the lead for lys, I want the full suite to run the same tests before and after the card so that I know the cleanup changed no behaviour.

## Boundaries

- SHALL NOT change any trait declaration, public signature or public type in lys-core, lys, lys-log-store, lys-anchor or lys-anchor-cli.
- SHALL NOT change the error, value or order of effects of any rewritten site; every verification path keeps its one uniform error.
- SHALL NOT change any wire format, domain-separation tag, test vector or signed fixture.
- SHALL NOT add #[allow], #[expect], #[ignore], #[cfg(any())], a `let _ =` statement, or an ignores or files entry beyond vendor/**.
- SHALL NOT change Cargo.toml, sgconfig.yml, docs/design/project.json, .land/gates.sh or .github/workflows/ci.yml.
- SHALL NOT rename PhantomData's `_marker` field in crates/lys-core/src/merkle/tree.rs.
- SHALL NOT commit either scratch file.
- SHALL NOT change Cambium's tree or the hand-written pre-method lys-core documents.

## Verification

- `ast-grep scan --config sgconfig.yml` at the repository root exits 0 with zero hits.
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps --all-features`, `cargo doc --no-deps` and `sh scripts/design/gate.sh` each exit 0.
- Adversarial review of the lys-core changes: for every rewritten site in tlog/verify.rs, bundle/verify.rs, checkpoint/note.rs, receipt/, delegation/, attestation/, seal/ and keys/, construct an input that fails at that site and show the returned error is the same value as at the commit the build starts from. For the sealed_envelope.rs nonce line, show that unseal still decrypts with the envelope's own nonce and that no derived value reaches the AEAD call.
- `git diff --stat <the commit the build starts from>` names only rules/ast-grep/no-underscore-binding.yml and files under crates/ listed in R1 to R6.
