---
type: design
cluster: lys-core
title: Lys Core — no underscore-prefixed bindings
---

# Lys Core — no underscore-prefixed bindings

> **Cluster:** lys-core

## Intention

No name in lys starts with an underscore to quiet the compiler. Where a binding would go unused, the code says what the value is for. A guard that has to live is named and dropped by name where its life ends, and a test fixture that holds a temporary directory names that field and closes it where the test ends, so the directory's removal is checked. A value with no use is not bound at all. A trait parameter one implementation has no use for is written as a bare `_`, and only where another implementation needs it or the trait is published. The gate refuses a new underscore-prefixed binding, and the rule's message says why, so nobody has to remember it.

Nothing a caller, a verifier or a test sees changes. Every rewritten site returns the same value and the same error as before, every verification failure still collapses to the one error it collapses to today, and the full suite runs the same tests before and after.

## Problem

CLAUDE.md calls a _-prefixed unused variable a bypass, not a fix, but nothing enforces that. clippy's used_underscore_binding catches only uses of an underscored binding, never the declaration. Measured with this card's rule at 7b53625, the tree holds 303 underscore-prefixed bindings in 70 files. The words count 272; the rule counts 303, and the rule's count is the one this card clears. Of the 303, 59 are guards whose directory or state the test still uses (TempDir holders and four EnvCleanup guards), whose names hide the fact that they exist to hold a lifetime. 68 are discarded errors: 64 map_err closures written |_err|, |_refusal| or |_source| because the workspace's clippy::map_err_ignore refuses map_err(|_| …), plus one or_else closure and three match arms. 164 are values nobody reads, 105 of them TempDirs a fixture returns beside an identity or authority it has already loaded into memory, so the directory keeps nothing alive. 10 are trait parameters an implementation ignores, and 2 are parameters of cfg(not(unix)) stubs. Eight test fixture structs also hold a TempDir in an underscore-prefixed field only to keep its directory alive, where the underscore silences a dead_code warning; a struct field is not a binding, so the rule does not report them, and the words' guard sentence covers them. The ast-grep rule set that LYSCORE-001 (brief 6747ce61) brings to lys left this rule out for a card of its own.

## Solution

One rule file joins the rule directory LYSCORE-001 lands. rules/ast-grep/no-underscore-binding.yml runs at severity error with vendor/** ignored. The existing leg `ast-grep scan --config sgconfig.yml` picks it up through sgconfig.yml's ruleDirs, so docs/design/project.json, .land/gates.sh and CI run it with the other ast-grep rules and none of the three changes (CN16). The rule reports an identifier that starts with an underscore and has at least one more character, wherever it binds: a let pattern, a function or closure parameter, a match arm, a for pattern, an if-let or while-let pattern, and the tuple, tuple-struct, struct (field and shorthand), slice, or, ref, mut, reference and @ patterns inside them. It never reports a bare `_`, which binds nothing (ADR-048), a struct field declaration, a field initialiser or a field access. Its message and note say why it refuses (ADR-047).

Each binding is cleared by what it does. A holder is classified by what happens after the binding, not by its name. A guard whose directory or state the test still uses is renamed without the underscore and ended with drop(name) after its last use; where a function's tail expression is its result, the result is bound, the guard dropped, and the binding returned. It is never made a bare `_`, which would drop it at the end of its statement and delete the directory under the test. Each of the eight fixture structs that holds a TempDir in an underscore-prefixed field names the field temp_dir and gains a close method returning TempDir::close's error, and every test closes each fixture value it builds after the value's last use, so the directory's removal is checked rather than silent. PhantomData's `_marker` in merkle/tree.rs is a type marker, not a held value, and stays. A discarded error is left unbound. A map_err becomes .ok().ok_or(E) when E is already built, and .ok().ok_or_else(…) when building E calls a function or a macro. No enabled clippy lint checks that split, so the brief measures it with an inline ast-grep rule. An or_else closure or a match arm writes the bare `_`. clippy::map_err_ignore stays (CN14). On the verification paths the single error is the design: a verifier must not say which check failed (P7). A value with no use becomes a bare `_` inside its pattern, including a TempDir a fixture returns beside a value it has already loaded into memory. A whole `let _x = expr;` becomes the expression statement `expr;`, never `let _ = expr;` (CN13). A trait parameter an implementation ignores becomes a bare `_` when another implementation of the same trait needs it or the trait is published, and no trait declaration changes (ADR-048, CN11). The two non-unix stubs take a bare `_`.

The card lands after LYSCORE-001 (brief 6747ce61), which lands sgconfig.yml, rules/ast-grep and the leg. It carries no second copy of any of them. The rule is shown to fire on a scratch file, with one hit for each binding position and none for the non-binding forms, before its zero hits on the tree are trusted (P9).

## Principles

- **P5** — Clear a binding by what it does: a guard is named and dropped by name, a value with no use is not bound at all, and an ignored trait parameter is a bare `_` only where the trait cannot drop it.
- **P6** — Behaviour is held: each rewritten site returns the same value and the same error as before, and the suite runs the same tests before and after.
- **P7** — A verifier never says which check failed. The one uniform error on a verification path is the design, so its discarded errors are not carried.
- **P8** — A bare `_` binds nothing and is admitted; an underscore-prefixed name is a binding and is refused.
- **P9** — Count what fired: the rule is shown to fire once for each binding position before zero hits on the tree is trusted.

## Decisions

- ADR-047 — lys refuses an underscore-prefixed binding with an ast-grep rule at severity error — lys carries its own ast-grep rule, no-underscore-binding, at severity error beside the rule set LYSCORE-001 lands. It refuses an underscore-prefixed identifier in every binding position, admits the bare `_`, and its message says why it refuses. Each existing binding is cleared by what it does rather than renamed. Rejected: relying on used_underscore_binding and review, which never see the declaration, and clearing hits by renaming, #[allow] or an ignores entry.
- ADR-048 — A trait parameter one implementation ignores is a bare `_` where the trait cannot drop it — An implementation that has no use for a parameter which another implementation of the same trait needs, or which a published trait asks for, writes the bare `_` pattern. The bare `_` binds nothing, so it is the words' 'not bound at all', and it is not the underscore-prefixed name the rule refuses. The trait is not changed. Rejected: changing AdmissionPolicy or LeafStore, adding a use to a parameter the implementation does not need, and keeping an underscore-prefixed name.

## Goals

- `ast-grep scan --config sgconfig.yml` reports 0 no-underscore-binding hits at the repository root of the landed tree.
- An uncommitted scratch file holding one underscore-prefixed binding in each of the 19 binding positions gets exactly 19 no-underscore-binding hits. An uncommitted file holding `fn scratch(_: u8, _unused: u8) {}` gets exactly one hit, at `_unused`.
- In each of the eight fixture files, the TempDir field reads `temp_dir`, and every test that builds one of those fixtures calls its close method.
- `cargo test --workspace --all-features` exits 0 at the commit the build starts from and at its final commit, and reports the same number of tests passed at both.
- The trait declarations of AdmissionPolicy, LeafStore, Signer and AnchorTask are byte-identical before and after the card.
- Both clippy legs pass with -D warnings while Cargo.toml still sets map_err_ignore = "warn".

## Non-Goals

- Offering the no-underscore-binding rule to Cambium's rule set — Cambium's tree is read only to this card; adopting the rule there is Cambium's own gated change.
- Retiring or relaxing clippy::map_err_ignore — The lint stays; a discarded error is left unbound by .ok().ok_or(…) and .ok().ok_or_else(…) instead.
- Renaming PhantomData's `_marker` field in merkle/tree.rs — It is a type marker, not a held value, and a struct field declaration is not a binding the rule refuses.
- Changing the AdmissionPolicy, LeafStore, Signer or AnchorTask trait — Each parameter is needed by some implementation, and LeafStore is in the published lys-log-store 0.2.0 (ADR-048).
- Carrying sgconfig.yml, the rule directory or the ast-grep leg — LYSCORE-001 (brief 6747ce61) lands them; this card adds one rule to them.
- Changing how a verification path collapses its failures — The single error is the design (P7); this card keeps it exactly.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `rules/ast-grep/no-underscore-binding.yml` | lys's rule refusing an underscore-prefixed binding, severity error, vendor/** ignored | LYSCORE-002 |
| `sgconfig.yml` | ast-grep root config naming rules/ast-grep; landed by LYSCORE-001 (brief 6747ce61), unchanged here |  |
| `rules/ast-grep` | lys's ast-grep rule directory; landed by LYSCORE-001 (brief 6747ce61) |  |
| `docs/design/lys-core/design.json` | this design | LYSCORE-002 |
| `docs/design/lys-core/checklist.json` | the rows LYSCORE-002 delivers | LYSCORE-002 |
| `docs/design/lys-core/stories.json` | the stories LYSCORE-002 serves | LYSCORE-002 |
| `docs/design/lys-core/briefs/LYSCORE-002.json` | the brief | LYSCORE-002 |
| `docs/design/lys-core/DESIGN.md` | rendered from design.json | LYSCORE-002 |
| `docs/design/lys-core/CHECKLIST.md` | rendered from checklist.json | LYSCORE-002 |
| `docs/design/lys-core/USER-STORIES.md` | rendered from stories.json | LYSCORE-002 |
| `docs/design/lys-core/briefs/LYSCORE-002.md` | rendered from the brief | LYSCORE-002 |
| `docs/design/lys-core/DESIGN-PRE-METHOD.md` | the hand-written pre-method design, renamed by LYSCORE-001 (brief 6747ce61, ADR-045), content unchanged |  |
| `docs/design/lys-core/CHECKLIST-PRE-METHOD.md` | the hand-written pre-method checklist, renamed by LYSCORE-001 (brief 6747ce61, ADR-045), content unchanged |  |
| `docs/design/lys-core/USER-STORIES-PRE-METHOD.md` | the hand-written pre-method stories, renamed by LYSCORE-001 (brief 6747ce61, ADR-045), content unchanged |  |
| `Cargo.toml` | workspace lint table; map_err_ignore and used_underscore_binding at warn, unchanged |  |
| `crates/lys-core/src` | lys-core library and sibling *_tests.rs files |  |
| `crates/lys-core/tests` | lys-core integration and conformance tests |  |
| `crates/lys-anchor/src` | lys-anchor library and sibling *_tests.rs files |  |
| `crates/lys-anchor/tests` | lys-anchor integration and conformance tests |  |
| `crates/lys-anchor-cli/src` | the anchor CLI and its sibling *_tests.rs files |  |
| `crates/lys-anchor-cli/tests` | the anchor CLI's integration tests |  |
| `crates/lys-home/src` | lys-home library and sibling *_tests.rs files |  |
| `crates/lys-home/tests` | lys-home integration tests |  |
| `crates/lys-log-store/src` | lys-log-store library, its published LeafStore trait and sibling *_tests.rs files |  |
| `crates/lys/src` | the lys CLI |  |
| `crates/lys/tests` | the lys CLI's integration tests |  |

## Inventory

- `crates` — At 7b53625 this card's rule reports 303 underscore-prefixed bindings in 70 files: 59 guards whose directory or state is still used, in 11 files (55 TempDir holders and four EnvCleanup guards in lys-core keys/identity_tests.rs), 68 discarded errors in 28 files (64 map_err closures, one or_else closure, three match arms), 164 unused values in 34 files (105 of them TempDirs returned beside an identity or authority already loaded into memory, 57 from golden_identity), 10 ignored trait parameters in 5 files and 2 cfg(not(unix)) stub parameters. Every underscore-prefixed identifier in the tree that is not a struct field is one of the 303; nine underscore-prefixed struct fields are not, the eight fixture TempDir fields and PhantomData's _marker.
- `Cargo.toml` — [workspace.lints.clippy]: pedantic at warn, map_err_ignore = "warn" (line 87), used_underscore_binding = "warn" (line 108), all failing under -D warnings.
- `crates/lys-core/src/tlog/verify.rs` — Eight map_err(|_err| TrustError::LogArtifactVerification) closures; every failure is that one error.
- `crates/lys-core/src/bundle/verify.rs` — Six map_err(|_err| reject()) closures; reject() builds the file's one error.
- `crates/lys-core/src/checkpoint/note.rs` — Three map_err(|_err| TrustError::NoteVerification) closures; every structural failure is that one error.
- `crates/lys-core/src/seal/sealed_envelope.rs` — Line 235 binds (key_bytes, _derived_nonce) from derive_key_and_nonce; the comment after it says why the derived nonce is not compared. A cryptographic file.
- `crates/lys-anchor/src/admission/policy.rs` — AdmissionPolicy::admit(&self, submission, context); AcceptAll ignores both parameters, MaxSize ignores context, the certificate policy ignores submission.
- `crates/lys-log-store/src/store.rs` — LeafStore, public in the published lys-log-store 0.2.0; the LyingStore fake in log_tests.rs ignores index, bytes and pin.
- `crates/lys-anchor/src/keys/signer.rs` — Signer::sign(&self, message); DecliningSigner in anchor/genesis_tests.rs ignores message.
- `crates/lys-anchor-cli/src/commands/anchor/policy.rs` — AnchorTask::run(self, policy); RecordPolicy in policy_tests.rs ignores policy.
- `crates/lys-log-store/src/file.rs` — Line 392: #[cfg(not(unix))] fn fsync_dir(_dir: &Path); no gate host compiles it.
- `crates/lys-core/src/keys/identity.rs` — Line 431: #[cfg(not(unix))] fn warn_if_loose_permissions(_path: &Path) {}; no gate host compiles it. Also seven discarded-error sites.
- `crates/lys-core/src/keys/identity_tests.rs` — Four `let _guard = EnvCleanup;` guards over the from_env tests; LYSCORE-001 plans to remove the environment mutation they guard.
- `crates (test fixture structs)` — Eight test fixture structs hold a TempDir in an underscore-prefixed field only to keep the directory alive: OpensslRequest (lys-core tests/openssl_csr_interop.rs:166), Party (lys-core tests/bundle_conformance.rs:107), Party (lys-core src/bundle/verify_tests.rs:27), ProvenLog (lys tests/log_tests.rs:96), Party (lys-anchor tests/cascade.rs:100), Fixture (lys-anchor-cli tests/anchor_cli.rs:35), Case (lys-anchor tests/stranger_verification.rs:228) and Node (lys-anchor src/upward/fixture.rs:48, used by upward/pin_tests.rs and upward/bundle_tests.rs). None has a close method.
- `crates/lys-core/src/merkle/tree.rs` — Line 68: the struct field _marker: PhantomData, not a binding.
- `sgconfig.yml` — Absent at 7b53625; LYSCORE-001 (brief 6747ce61) creates it.
- `rules` — Absent at 7b53625; LYSCORE-001 (brief 6747ce61) creates rules/ast-grep.
- `docs/design/project.json` — Seven legs; no ast-grep leg at 7b53625. LYSCORE-001 adds `ast-grep scan --config sgconfig.yml` here, in .land/gates.sh and in CI.
- `CLAUDE.md` — Coding standards: silencing a lint with #[allow], an #[ignore]d test, a _-prefixed unused variable or #[cfg(any())] is a bypass, not a fix.
- `docs/design/lys-core` — Hand-written pre-method DESIGN.md, CHECKLIST.md (C1 to C65) and USER-STORIES.md (S1 to S23), renamed *-PRE-METHOD.md with their content unchanged as LYSCORE-001 (ADR-045) does, so this cluster's rendered DESIGN.md, CHECKLIST.md and USER-STORIES.md take the plain names.
- `$cambium/rules/ast-grep` — Six rules, none refusing underscore-prefixed bindings. Read only.

## Constraints

- **CN9** — Every rewritten site returns the same value and the same error (variant, fields and message) as before; every failure in tlog/verify.rs, bundle/verify.rs and checkpoint/note.rs still returns the one error it returns today.
- **CN10** — No wire format, domain-separation tag, test vector or signed fixture changes.
- **CN11** — The public API of lys-core, lys and lys-log-store does not change, and the AdmissionPolicy, LeafStore, Signer and AnchorTask declarations are byte-identical.
- **CN12** — No hit is cleared by #[allow], #[expect], #[ignore], #[cfg(any())], a rename to another underscore-prefixed name, or an ignores or files entry beyond vendor/**.
- **CN13** — No `let _ =` statement is introduced under crates/.
- **CN14** — Cargo.toml keeps map_err_ignore = "warn" and used_underscore_binding = "warn".
- **CN15** — Cambium's tree is not changed.
- **CN16** — docs/design/project.json, .land/gates.sh, .github/workflows/ci.yml and sgconfig.yml are not changed.
- **CN17** — The hand-written pre-method lys-core documents are not edited.
- **CN18** — The rule is at severity error.
