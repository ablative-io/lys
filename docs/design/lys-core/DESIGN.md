---
type: design
cluster: lys-core
title: Lys Core — the ast-grep rule set and its gate leg
---

# Lys Core — the ast-grep rule set and its gate leg

> **Cluster:** lys-core

## Intention

The rules lys holds about its own code are enforced by a machine at every place a land is gated, not by whichever clippy lint happens to overlap them and not by a reviewer remembering them. A card author learns of an unwrap in library code, a lint bypass, a dropped Result or logic in a mod.rs from the gate, which names the rule and the line.

Test code is recognised by what the file itself carries, never by a list of names kept somewhere else, so the rule and the file cannot drift apart. Each rule is shown to fire before it is trusted, and a rule that cannot fire on this tree is not carried, because a control that never fires is indistinguishable from one that passed.

## Problem

The lys gate has no ast-grep leg. lys has no sgconfig.yml and no rules directory, so its rules against unwrap, expect and panic outside tests, against #[allow] and #[ignore], against dropped Results and against logic in mod.rs are caught only where a clippy lint overlaps them. Measured at 4dd1c33, the rules that would be carried report 113 lint-bypass hits, 19 let-underscore hits, 20 mod-rs hits and 224 unwrap/expect/panic hits, so a leg added alone would turn every lys round red. The per-module test opt-out that CLAUDE.md sanctions is itself an #[allow]. The gate is held in three places that do not follow each other: docs/design/project.json, .land/gates.sh, which repo_land runs as the whole gate, and CI; and scripts/design/gate.sh validates project.json without running any leg.

## Solution

sgconfig.yml at the repository root names one rule directory, rules/ast-grep, as Cambium's does. It holds four rules at severity error (ADR-063). Three are Cambium's mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, carried with the same id, severity, files and rule, a message that cites lys's CLAUDE.md where Cambium's cites its own AGENTS.md, and vendor/** ignored so an initialised Rauthy submodule is never scanned. The fourth is lys's own no-unwrap-expect-panic-outside-tests. It treats four structures as test code: a file whose first item is the inner attribute #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, and a path under tests/. It reports unwrap, expect and panic everywhere else. The leg is one command, `ast-grep scan --config sgconfig.yml`, run from the repository root. It exits non-zero on any hit and zero on none, and it runs in docs/design/project.json, in .land/gates.sh and in CI.

The tree is brought to zero hits in the same card, so the leg is green when it lands. Every sibling *_tests.rs file and both fixture.rs files gain #![cfg(test)] as their first line, which states what their parent's #[cfg(test)] mod already makes true. Every integration test root and the two tests/harness/mod.rs files gain it too: an integration test root is only ever compiled as a test, so the attribute changes nothing about what is built and lets clippy treat its helper fns as test code. The per-module #![allow] opt-outs are replaced by clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests, which clippy applies to code it knows is test code. The seven #[allow(unsafe_code)] lines in lys-core's identity tests go with the environment mutation that needed them: the from_env tests exercise the rules applied to the variable's value directly, and the process environment is never written, and the comment above lib.rs's unsafe_code attribute is corrected to say so. Dropped Results are handled, and the three infallible String writes in library and CLI code use a form that returns no Result. Logic in three mod.rs files moves into named sibling files. The 47 mod-rs hits in lys-home's record/mod.rs on main belong to HOME-013's split, which this card is built on. The proof that the unwrap rule fires is one scratch file without the marker, reported by the scan and never committed.

no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers are not carried (ADR-064). No _name-rename rule is written, because Cambium carries none. Before the cluster is rendered, the hand-written lys-core documents are renamed to *-PRE-METHOD.md (ADR-065), so the method's render takes the names DESIGN.md, CHECKLIST.md and USER-STORIES.md without overwriting the earlier design.

## Principles

- **P1** — Test code is recognised by structure the file carries: a first-line #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory. No rule and no config holds a list of file names.
- **P2** — A rule is carried only where it can fire on this tree; a rule that cannot fire is recorded as not carried with the finding and the act that brings it back.
- **P3** — Count what fired: every zero-hit claim is paired with a never-landed scratch case the rule reports, and each structural marker has a case of its own.
- **P4** — A bypass is fixed at its cause and never replaced by another bypass; one that cannot be fixed goes back to the lead as a question with its line.
- **P5** — One leg on every landing path: project.json, .land/gates.sh and CI run the same command.
- **P6** — A comment the code no longer bears out is corrected in the change that made it false.
- **P7** — A verifier never says which check failed. The one uniform error on a verification path is the design, so its discarded errors are not carried.
- **P8** — A bare `_` binds nothing and is admitted; an underscore-prefixed name is a binding and is refused.
- **P9** — Count what fired: the rule is shown to fire once for each binding position before zero hits on the tree is trusted.
- **P10** — Test code is recognised by structure the file carries: its first item, an enclosing #[cfg(test)] mod or #[test] fn, or a path under tests/. Never by a list of file names.
- **P11** — A rule is trusted only once shown to fire, and a rule that cannot fire on this tree is not carried.
- **P12** — The leg lands green: every hit a carried rule reports is fixed at its cause in the same card, never exempted, narrowed or silenced.
- **P13** — Every place a land is gated runs the same command, so no route to main skips the rules.

## Decisions

- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-008 — An agent's file shows its lys certificate — An agent's file shows its lys certificate once one is issued: what it claims, who signed it, when it was issued and when it expires, with the signed receipts of the changes made to it. An agent registered before any key or proof of possession was supplied shows its certificate as not issued, never a placeholder.
- ADR-044 — A certificate is self-signed when its subject key is the issuer key and its signature verifies under it, never by its names — Self-signed is judged by keys: a certificate is self-signed when its subject public key is the supplied issuer key, compared as decoded Ed25519 points, and its signature verifies under that key with verify_strict, and such a certificate is refused whatever its names. A certificate issued to a subject whose name equals the issuer's hex-key common name is judged on its real issuer and verifies. Rejected: keeping the DN-byte comparison as a heuristic screen; keeping truly self-signed certificates with differing names verifying.
- ADR-045 — The self-signed fix and the verify_certificate_chain caveat land on main with no lys-core release, after DIRECTORY-013 — This card lands the key-based self-signed rule and the rustdoc caveat on main and cuts no crates.io release; a release is its own act after the ordering fold. DIRECTORY-013 lands first and is not amended; this card's build is blocked until DIRECTORY-013's roadmap row reads landed on origin/main. Rejected: a documentation-only release cut from main or from a 0.2.0 base in this card; landing this card first and amending DIRECTORY-013.
- ADR-047 — The given statement is a lys/attestation/v2 over the RFC 8785 bytes of lys.given's data, written only when render-launch is given a key — The given statement is lys-core's existing lys/attestation/v2 over the RFC 8785 (JSON Canonicalization Scheme) bytes of the lys.given entry's data, so its signed payload hash is the given hash render-launch reports as given_sha256. It is written only when render-launch is given a key: kept as a block named by a lys.given_statement entry hung under the lys.given entry, and as given-statement.cose and given-data.json under --out for lys verify. Rejected: serialising the record in struct field order, which a stranger cannot rebuild without the Rust type; signing the 64-hex hash string, whose attested hash would be SHA-256 of the hex and not the given hash; adding the given hash to the template_render event, which HOME-003 keeps unaltered; and changing lys verify's one failure message to name the file, which would change the published lys for every user.
- ADR-048 — A trait parameter one implementation ignores is a bare `_` where the trait cannot drop it — An implementation that has no use for a parameter which another implementation of the same trait needs, or which a published trait asks for, writes the bare `_` pattern. The bare `_` binds nothing, so it is the words' 'not bound at all', and it is not the underscore-prefixed name the rule refuses. The trait is not changed. Rejected: changing AdmissionPolicy or LeafStore, adding a use to a parameter the implementation does not need, and keeping an underscore-prefixed name.
- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-054 — lys recognises test code by structure the file carries, for ast-grep and clippy alike — Test code is a file whose first line is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory; every file only ever compiled as a test carries the first-line marker, and clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests replace the per-module #![allow]. Rejected: an exemption list of file names in the rule, and rewriting the test helpers' calls.
- ADR-055 — The Claude Code render refuses by name what it cannot shape and never writes a default in place of a value the record did not carry — Every value the render copies from a message entry must be present and of the type the target field takes; a missing field or a field of another type refuses the render by the session, the entry id, the field and the expected type, and nothing is written. The value itself is never named, except an unmapped stopReason, which is named because it is a protocol value and not transcript. Every serialisation the render needs, the records, the dropped parts it hashes and the loss account, is done before any directory or file is created, and a failure refuses by name. Only three values are written that the record does not carry, and they are format constants Claude Code's file requires on every record: gitBranch "", usage {input_tokens 0, output_tokens 0} and stop_sequence null. Only one field reads a meaning from its absence: a thinking part with no `redacted` field is not redacted, since in Pi's grammar the field is optional and its absence is the source's own statement; a `redacted` present and not a boolean refuses. stopReason is mapped by one explicit arm each for toolUse, length, stop, error and aborted, and a wildcard arm: toolUse maps to tool_use, length to max_tokens and stop to end_turn; the error arm and the aborted arm refuse by name, naming the value, because Claude Code's own session files carry no value for an errored or interrupted turn (measured on Claude Code 2.1.283 over every JSONL file of its projects directory, 3379 files, with `grep -rhoE '"stop_reason":("[a-z_]+"|null)' --include='*.jsonl' <projects directory> | sort | uniq -c`: tool_use 547470, null 172397, end_turn 53290, stop_sequence 2253, refusal 22, max_tokens 1, error 0, aborted 0); and the wildcard arm refuses by name, naming the value, and maps to no Claude Code value. A present role the render does not know is still skipped. render-launch stores its template in the home only after the render has succeeded. Rejected: rendering a default and naming it in the loss account, refusing only missing fields and copying wrongly typed ones, mapping every unknown stopReason to end_turn, and keeping the template a refused launch had already stored.
- ADR-063 — lys enforces its code rules with ast-grep, recognising test code by structure the file carries — Carry Cambium's mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, and add lys's own no-unwrap-expect-panic-outside-tests. That rule treats as test code a file whose first item is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, and a path under tests/. Every *_tests.rs file, both fixture.rs files, every integration test root and both tests/harness/mod.rs files gain #![cfg(test)] as their first line. The per-module #![allow] opt-outs are replaced by clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests. Every existing hit is fixed at its cause in the same card, so the leg is green when it lands. The leg `ast-grep scan --config sgconfig.yml` runs in docs/design/project.json, .land/gates.sh and CI. Rejected: an exemption list of file names; rewriting the calls in test files; narrowing no-lint-bypass-attributes to spare the test opt-outs; carrying only the rules that were clean at the time.
- ADR-064 — A rule that cannot fire on the lys tree is not carried — no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers are recorded as not carried, with that finding. The card that lands the first async code in lys carries no-std-mutex-in-async with it, and its brief names this decision. Rejected: carrying all six of Cambium's rules because the words said the same rule set.
- ADR-065 — The hand-written lys-core documents are kept as *-PRE-METHOD.md, not replaced by the method's render — Before this cluster is rendered, the three documents are renamed to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md with their bytes unchanged, and the method's render takes the old names. The live citations of the old paths are re-pointed to the new ones. LYSCORE-003's card is the one owner of docs/design/lys-core. Rejected: overwriting the documents with the render, and deleting them.

## Goals

- crates/lys-core/src/ca/authority.rs contains no call to .subject() or .issuer() on a parsed certificate.
- A certificate the authority issues to the subject named by its own lowercase-hex public key verifies under the authority's key and is refused under any other key, in lys-core and through lys ca verify and lys verify --cert.
- A certificate whose subject key is the supplied issuer key and whose signature verifies under it is refused with the reason 'self-signed certificate rejected (subject key is the issuer key)'.
- The rustdoc of verify_certificate_chain names every check it makes and every question it leaves to the caller, and both cargo doc shapes build with no warning.
- DIRECTORY-013's fold issues a legitimately issued hex-common-name leaf and records certificate_chain_invalid for a leaf self-signed by keys.
- docs/design/lys-core/checklist.json holds C1 to C74 in order, the carried C1 to C65 unchanged but for C23, and sh scripts/design/gate.sh exits 0.
- `lys-core` compiles standalone in this repository with zero Meridian dependencies and zero Meridian references, behaviour-identical to the hardened source crate except the deliberate breaks (D5 legacy strip, D7 tag renames, `LYS_IDENTITY_KEY`, `LYS_OID_ARC`).
- All hardening commitments hold in the ported code: `verify_strict` everywhere, validity-window enforcement with an `_at` variant, `RootHash::from_parts` external verification, authenticated timestamps with domain separation, contributory-DH rejection, seed zeroization, single-arbiter unsealing, race-free key generation.
- Attestation verification is v2-only (WIRE-FORMATS.md D4): no legacy code path exists in the crate — neither the Meridian preimage nor the deleted-unshipped `lys/attestation/v1` form verifies.
- An external verifier round-trips: inclusion and consistency proofs verify from published root parts and proof bytes alone.
- The `lys` CLI covers every primitive, and a log produced in one process verifies end-to-end via the CLI in another with no access to the original tree.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --workspace` all pass clean.
- `ast-grep scan --config sgconfig.yml` reports 0 no-underscore-binding hits at the repository root of the landed tree.
- An uncommitted scratch file holding one underscore-prefixed binding in each of the 19 binding positions gets exactly 19 no-underscore-binding hits. An uncommitted file holding `fn scratch(_: u8, _unused: u8) {}` gets exactly one hit, at `_unused`.
- In each of the eight fixture files, the TempDir field reads `temp_dir`, and every test that builds one of those fixtures calls its close method.
- `cargo test --workspace --all-features` exits 0 at the commit the build starts from and at its final commit, and reports the same number of tests passed at both.
- The trait declarations of AdmissionPolicy, LeafStore, Signer and AnchorTask are byte-identical before and after the card.
- Both clippy legs pass with -D warnings while Cargo.toml still sets map_err_ignore = "warn".
- `ast-grep scan --config sgconfig.yml` from the repository root reports zero hits and exits 0 at the landed commit.
- A never-landed scratch file with an unwrap and no #![cfg(test)] turns the ast-grep leg of .land/gates.sh red while every other leg stays green.
- grep finds zero #[allow], #![allow], #[expect] and #[ignore] under crates/, and both clippy legs pass with -D warnings.
- docs/design/project.json, .land/gates.sh and .github/workflows/ci.yml each run `ast-grep scan --config sgconfig.yml`.
- sh scripts/design/gate.sh measures lys-core for the first time and exits 0, with the three pre-method documents kept byte for byte.
- `ast-grep scan --config sgconfig.yml` exits 0 at the repository root of the landed tree and reports 0 hits.
- One scratch file holding an unwrap outside test code, added under crates/ and never committed, makes the scan exit 1 with exactly one hit, from no-unwrap-expect-panic-outside-tests.
- The command `ast-grep scan --config sgconfig.yml` appears once in each of docs/design/project.json, .land/gates.sh and .github/workflows/ci.yml.
- Both clippy legs pass with -D warnings, and no #[allow] line remains under crates/.
- docs/design/lys-core holds DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md byte-equal to the hand-written files, and sh scripts/design/gate.sh exits 0.

## Non-Goals

- Publishing a lys-core release to crates.io — Publishing lys-core freezes the unpublished lys/delegation/v1, which waits for the ordering fold; cutting a release is its own act afterwards (ADR-045).
- Amending DIRECTORY-013's brief — Its 'SHALL NOT change' lines bind that card's own build; it lands first and is not edited here (ADR-045).
- A revocation check in lys-core — Revocation stays consumer-side; the caveat names it as the caller's.
- Walking a chain above one level, or checking the issuer's basic constraints or key usage — The function verifies one signature under one supplied key; the caveat names this as the caller's.
- A distinct CLI message for the self-signed refusal — The CLI refusal is deliberately non-oracle (P4).
- **Domain-specific semantics.** No agent, session, or claim vocabulary in the crate. Canonical agent claim schemas are phase 5.
- **Storage traits.** In-memory operations only; persistence is the consumer's concern. The CLI persists leaf sequences as files, using `reconstruct_from_leaves` — that is CLI policy, not a library trait.
- **Network operations.** `lys-core` is a pure library; the CLI is local-only. Transport belongs to `lys-anchor` (phase 4).
- **Anchoring, receipts, SCITT/COSE.** The notary layer is `lys-anchor`; nothing in this cluster emits or verifies COSE receipts.
- **Revocation infrastructure.** No CRLs, no OCSP, no revocation flag. Consumer-side today; a first-class answer is an open product question.
- **MCP surface.** `lys-mcp` is a later phase.
- **Zero-knowledge proofs.** Selective disclosure via salted-hash leaves + inclusion proofs is the v1 privacy story; ZK is a research direction.
- Offering the no-underscore-binding rule to Cambium's rule set — Cambium's tree is read only to this card; adopting the rule there is Cambium's own gated change.
- Retiring or relaxing clippy::map_err_ignore — The lint stays; a discarded error is left unbound by .ok().ok_or(…) and .ok().ok_or_else(…) instead.
- Renaming PhantomData's `_marker` field in merkle/tree.rs — It is a type marker, not a held value, and a struct field declaration is not a binding the rule refuses. The marker field is kept as it is; nothing else in tree.rs is promised unchanged, and its two map_err closures are rewritten with the other discarded errors.
- Changing the AdmissionPolicy, LeafStore, Signer or AnchorTask trait — Each parameter is needed by some implementation, and LeafStore is in the published lys-log-store 0.2.0 (ADR-048).
- Carrying sgconfig.yml, the rule directory or the ast-grep leg — LYSCORE-001 (brief 6747ce61) lands them; this card adds one rule to them.
- Changing how a verification path collapses its failures — The single error is the design (P7); this card keeps it exactly.
- A _name binding rule and the handling of each underscore-prefixed binding — Cambium carries no such rule, so the same rule set does not include it; a card of its own writes the rule and handles each binding by its act.
- Carrying no-std-mutex-in-async — lys has no async fn, no tokio and no std Mutex, so it cannot fire (ADR-055); the card that lands lys's first async code carries it.
- Carrying no-timer-in-door-handlers and no-timer-import-in-door-handlers — lys has no door handlers, so they cannot fire (ADR-055).
- Tightening #![cfg_attr(not(test), forbid(unsafe_code))] so tests forbid unsafe code too — The attribute stays exactly as it is in this work; tightening it is a further unit.
- Splitting crates/lys-home/src/record/mod.rs — Its 47 hits are HOME-013's, which this work waits on.
- Correcting CHECKLIST-PRE-METHOD.md's C4 — The kept documents are the historical record and stay exactly as they were; the rendered CHECKLIST.md is the current truth.
- A rule for todo!, unimplemented! and unreachable! — The words and the survey name unwrap, expect and panic; the other three stay with clippy's workspace lints.
- ast-grep rule tests (`ast-grep test`) in the repository — No gate leg would run them; the never-landed scratch cases in the brief's acceptance are the measurement the words ask for.
- Editing Cambium's rules or config to match lys — Neither project is edited to match the other; lys takes the opposite clippy.toml setting on tests from Cambium's.
- Carrying no-std-mutex-in-async — lys has no async fn, no tokio and no std Mutex, so it could not fire (ADR-064). The card that lands the first async code carries it and names ADR-064.
- Carrying no-timer-in-door-handlers and no-timer-import-in-door-handlers — Their files are Cambium door-handler paths that lys does not have, so they could not fire (ADR-064).
- A _name-rename rule and the existing _-prefixed bindings — Cambium carries no such rule. A card of its own writes the rule and handles each binding by its act.
- Splitting crates/lys-home/src/record/mod.rs — HOME-013 owns that split; this card is built on it and lands after it.
- Tightening lys-core's unsafe_code attribute so test builds forbid unsafe code too — The attribute stays exactly as it is; only the comment above it is corrected. Tightening it is a further unit.
- Changing Cambium's rules or policy — Cambium's tree is read only; its policy of no unwrap even in tests is not imported.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/lys-core/design.json` | this brief's cluster record; R7 restates the carried design's self-signed sentence and appends this record's principles, constraints, goals, non-goals and inventory under the next free numbers | LYSCORE-001 |
| `docs/design/lys-core/checklist.json` | the checklist rows LYSCORE-001 delivers; R7 adds them to the carried checklist | LYSCORE-001 |
| `docs/design/lys-core/stories.json` | the stories LYSCORE-001 serves | LYSCORE-001 |
| `docs/design/lys-core/briefs/LYSCORE-001.json` | the brief: self-signed by keys, the verifier's caveat, and their consumers | LYSCORE-001 |
| `docs/design/lys-core/DESIGN.md` | rendered from design.json; R7 commits the render | LYSCORE-001 |
| `docs/design/lys-core/CHECKLIST.md` | rendered from checklist.json; R7 commits the render | LYSCORE-001 |
| `docs/design/lys-core/USER-STORIES.md` | rendered from stories.json; R7 commits the render | LYSCORE-001 |
| `crates/lys-core/src/ca/authority.rs` | issuance and chain verification; the self-signed judgement and the verifier rustdoc |  |
| `crates/lys-core/src/ca/authority_tests.rs` | unit tests of authority.rs |  |
| `crates/lys/tests/certified_attestation_tests.rs` | CLI tests of lys verify --cert and the certificate it joins |  |
| `crates/lys-anchor/src/admission/certificate.rs` | anchor admission by certificate; its module doc names what verify_certificate_chain rejects |  |
| `CHANGELOG.md` | release record; the Unreleased section |  |
| `crates/lys-identity/tests/revocation_fold.rs` | the fold's tests, created by DIRECTORY-013 in the directory cluster | DIRECTORY-013 |
| `crates/lys-core/` |  |  |
| `crates/lys-core/Cargo.toml` |  |  |
| `crates/lys-core/tests/` | cross-implementation conformance suites |  |
| `crates/lys-core/tests/cose_conformance.rs` | round-trip against veraison/go-cose |  |
| `crates/lys-core/tests/go_conformance.rs` | round-trip against Go sumdb/note |  |
| `crates/lys-core/tests/signed_note_crosscheck.rs` | crosscheck against Cloudflare signed_note |  |
| `crates/lys-core/src/` |  |  |
| `crates/lys-core/src/lib.rs` | pub mod + re-exports, hex_lower helper (D1) |  |
| `crates/lys-core/src/error.rs` | TrustError enum, TrustResult<T> (D1) |  |
| `crates/lys-core/src/keys/` |  |  |
| `crates/lys-core/src/keys/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/keys/identity.rs` | Ed25519Identity: load_or_generate, from_env, sign, verify_strict, X25519 derivation, redaction (D2) |  |
| `crates/lys-core/src/keys/identity_tests.rs` |  |  |
| `crates/lys-core/src/ca/` |  |  |
| `crates/lys-core/src/ca/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/ca/authority.rs` | CertificateAuthority: issue, verify chain, _at variant (D3) |  |
| `crates/lys-core/src/ca/certificate.rs` | IssuedCertificate, Debug redaction (D3) |  |
| `crates/lys-core/src/ca/extensions.rs` | LYS_OID_ARC, encode/decode extension (D3) |  |
| `crates/lys-core/src/ca/*_tests.rs` |  |  |
| `crates/lys-core/src/merkle/` |  |  |
| `crates/lys-core/src/merkle/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/merkle/tree.rs` | AppendOnlyTree<L>: append, root, proofs, reconstruct (D4) |  |
| `crates/lys-core/src/merkle/proof.rs` | RootHash from_parts/to_parts, Inclusion/ConsistencyProof, verify_inclusion, verify_consistency, raw-leaf path (D4) |  |
| `crates/lys-core/src/merkle/leaf.rs` | leaf hashing, RawLeaf, frozen-wire-contract docs (D4) |  |
| `crates/lys-core/src/merkle/*_tests.rs` |  |  |
| `crates/lys-core/src/checkpoint/` | WIRE-FORMATS D1: signed tree heads |  |
| `crates/lys-core/src/checkpoint/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/checkpoint/body.rs` | CheckpointBody encode/parse (C2SP tlog-checkpoint) |  |
| `crates/lys-core/src/checkpoint/note.rs` | sign_note / verify_note / verify_checkpoint (C2SP signed-note; origin == key-name enforced) |  |
| `crates/lys-core/src/checkpoint/verifier_key.rs` | verifier-key strings and RFC 6962-style key IDs |  |
| `crates/lys-core/src/checkpoint/*_tests.rs` |  |  |
| `crates/lys-core/src/tlog/` | WIRE-FORMATS D2: self-contained proof artifacts |  |
| `crates/lys-core/src/tlog/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/tlog/artifact.rs` | frozen JSON artifact shapes and format strings |  |
| `crates/lys-core/src/tlog/build.rs` | self-verifying inclusion/consistency builders |  |
| `crates/lys-core/src/tlog/verify.rs` | third-party verification, single non-oracle error |  |
| `crates/lys-core/src/tlog/*_tests.rs` |  |  |
| `crates/lys-core/src/attestation/` |  |  |
| `crates/lys-core/src/attestation/mod.rs` | pub mod / pub use, invariant docs |  |
| `crates/lys-core/src/attestation/artifact.rs` | Attestation type; to_cose_bytes / canonical-strict from_cose_bytes (D5) |  |
| `crates/lys-core/src/attestation/encoding.rs` | private byte-exact COSE_Sign1 encode + shape-pinned decode, Sig_structure assembly (D5) |  |
| `crates/lys-core/src/attestation/sign.rs` | sign_attestation, verify_attestation, verify_attestation_bytes over the v2 Sig_structure (D5) |  |
| `crates/lys-core/src/attestation/*_tests.rs` | sibling tests incl. golden vectors and mutants A–F |  |
| `crates/lys-core/src/seal/` |  |  |
| `crates/lys-core/src/seal/mod.rs` | pub mod / pub use only |  |
| `crates/lys-core/src/seal/sealed_envelope.rs` | seal/open, HKDF binding, contributory checks, single failure arbiter (D6) |  |
| `crates/lys-core/src/seal/authenticated.rs` | sign_and_seal / open_and_verify (D6) |  |
| `crates/lys/` |  |  |
| `crates/lys/Cargo.toml` |  |  |
| `crates/lys/tests/` |  |  |
| `crates/lys/tests/cli_tests.rs` | key / attest / verify / ca / seal / open / inspect |  |
| `crates/lys/tests/log_tests.rs` | log lifecycle incl. the cross-process third-party path |  |
| `crates/lys/src/` |  |  |
| `crates/lys/src/main.rs` | thin entry: parse args, dispatch, exit codes (D8) |  |
| `crates/lys/src/cli.rs` | clap definitions and help text only (D8) |  |
| `crates/lys/src/commands/` |  |  |
| `crates/lys/src/commands/mod.rs` | pub mod only |  |
| `crates/lys/src/commands/key.rs` | lys key generate / inspect (D8) |  |
| `crates/lys/src/commands/ca.rs` | lys ca issue / verify (D8) |  |
| `crates/lys/src/commands/attest.rs` | lys attest (D8) |  |
| `crates/lys/src/commands/verify.rs` | lys verify (D8) |  |
| `crates/lys/src/commands/inspect.rs` | lys inspect attestation / cert (D8) |  |
| `crates/lys/src/commands/seal.rs` | lys seal / open (D8) |  |
| `crates/lys/src/commands/error.rs` | CLI error type, non-oracle failure messages (D8) |  |
| `crates/lys/src/commands/files.rs` | file I/O incl. owner-only plaintext writes (D8) |  |
| `crates/lys/src/commands/hex.rs` | hex parsing/formatting helpers (D8) |  |
| `crates/lys/src/commands/pem.rs` | PEM encode/decode helpers (D8) |  |
| `crates/lys/src/commands/log/` |  |  |
| `crates/lys/src/commands/log/mod.rs` | pub mod only |  |
| `crates/lys/src/commands/log/init.rs` | lys log init (origin pinned once) (D8) |  |
| `crates/lys/src/commands/log/append.rs` | lys log append (D8) |  |
| `crates/lys/src/commands/log/checkpoint.rs` | lys log checkpoint (D8) |  |
| `crates/lys/src/commands/log/prove.rs` | lys log prove inclusion / consistency (D8) |  |
| `crates/lys/src/commands/log/verify.rs` | lys log verify inclusion / consistency (D8) |  |
| `crates/lys/src/commands/log/store.rs` | leaf-sequence store: O_EXCL leaf writes, atomic tmp+rename state, rebuild on open (D8) |  |
| `docs/design/lys-core/design.json` | the lys-core design, carried from main's DESIGN.md | LYSCORE-001 |
| `docs/design/lys-core/DESIGN.md` | rendered markdown |  |
| `docs/design/lys-core/checklist.json` | the lys-core checklist, carried from main's CHECKLIST.md | LYSCORE-001 |
| `docs/design/lys-core/CHECKLIST.md` | rendered markdown |  |
| `docs/design/lys-core/stories.json` | the lys-core user stories, carried from main's USER-STORIES.md | LYSCORE-001 |
| `docs/design/lys-core/USER-STORIES.md` | rendered markdown |  |
| `docs/design/lys-core/briefs/LYSCORE-001.json` | the brief that carries the hand-written lys-core design into the three JSON documents | LYSCORE-001 |
| `docs/design/lys-core/briefs/LYSCORE-001.md` | rendered markdown | LYSCORE-001 |
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
| `docs/design/lys-core/design.json` | This design | LYSCORE-001 |
| `docs/design/lys-core/checklist.json` | The rows LYSCORE-001 delivers | LYSCORE-001 |
| `docs/design/lys-core/stories.json` | The stories LYSCORE-001 serves | LYSCORE-001 |
| `docs/design/lys-core/briefs/LYSCORE-001.json` | The ast-grep leg brief | LYSCORE-001 |
| `docs/design/lys-core/briefs/LYSCORE-001.md` | Its rendered markdown | LYSCORE-001 |
| `docs/design/lys-core/DESIGN.md` | The rendering of design.json, committed with the brief; R1 checks it | LYSCORE-001 |
| `docs/design/lys-core/CHECKLIST.md` | The rendering of checklist.json, committed with the brief; R1 checks it | LYSCORE-001 |
| `docs/design/lys-core/USER-STORIES.md` | The rendering of stories.json, committed with the brief; R1 checks it | LYSCORE-001 |
| `docs/design/lys-core/DESIGN-PRE-METHOD.md` | The hand-written lys-core design, renamed in the brief's own commit and kept byte for byte as it stood at 7b53625 | LYSCORE-001 |
| `docs/design/lys-core/CHECKLIST-PRE-METHOD.md` | The hand-written lys-core checklist (C1 to C65 of the extraction), renamed in the brief's own commit and kept byte for byte; its C4 sentence is not borne out by the code after R5 | LYSCORE-001 |
| `docs/design/lys-core/USER-STORIES-PRE-METHOD.md` | The hand-written lys-core user stories, renamed in the brief's own commit and kept byte for byte | LYSCORE-001 |
| `sgconfig.yml` | ast-grep project config naming rules/ast-grep as its one rule directory | LYSCORE-001 |
| `rules/ast-grep/mod-rs-declarations-only.yml` | Carried from Cambium: mod.rs holds no function, struct, enum, trait, impl, const or static | LYSCORE-001 |
| `rules/ast-grep/no-let-underscore-on-results.yml` | Carried from Cambium: no `let _ =` discard | LYSCORE-001 |
| `rules/ast-grep/no-lint-bypass-attributes.yml` | Carried from Cambium: no #[allow], #![allow], #[expect] or #[ignore] | LYSCORE-001 |
| `rules/ast-grep/no-unwrap-expect-panic-outside-tests.yml` | New: unwrap, expect and panic reported everywhere except test code recognised by structure | LYSCORE-001 |
| `clippy.toml` | allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests set to true | LYSCORE-001 |
| `crates/lys-core/tests/harness/go.rs` | The Go-toolchain logic moved out of lys-core's tests/harness/mod.rs | LYSCORE-001 |
| `crates/lys-anchor/tests/harness/go.rs` | The Go-toolchain logic moved out of lys-anchor's tests/harness/mod.rs | LYSCORE-001 |
| `crates/lys-anchor/tests/harness/scaffold.rs` | GO_ENV, GoScaffold, ALL_SCAFFOLDS and the path to lys-core's harness, moved out of lys-anchor's tests/harness/mod.rs | LYSCORE-001 |
| `crates/lys-anchor/tests/harness/scaffold_tests.rs` | The two contract tests moved out of lys-anchor's tests/harness/mod.rs | LYSCORE-001 |
| `crates/lys-home/src/harness/claude_code/names.rs` | HARNESS, PROVIDER, API and AUTHORED, moved out of claude_code/mod.rs | LYSCORE-001 |
| `docs/design/project.json` | The tree's gate legs; gains the ast-grep leg |  |
| `.land/gates.sh` | The whole gate the landing runs; gains `leg ast-grep scan --config sgconfig.yml` |  |
| `.github/workflows/ci.yml` | CI; gains a pinned ast-grep install and the same scan |  |
| `CLAUDE.md` | The test opt-out sentence and the gates block, corrected |  |
| `Cargo.toml` | The workspace lint comment, corrected to name clippy.toml; the serial_test workspace dev-dependency, removed |  |
| `crates/lys-core/Cargo.toml` | The serial_test dev-dependency, removed with the last #[serial_test::serial] |  |
| `Cargo.lock` | Regenerated by cargo without serial_test and serial_test_derive |  |
| `crates/lys-core/src/lib.rs` | The comment above #![cfg_attr(not(test), forbid(unsafe_code))], rewritten; the attribute stays byte-identical; hex_lower's line 59 takes a form that returns no Result and line 54's import goes |  |
| `crates/lys-core/src/keys/identity.rs` | Ed25519Identity::from_env reads the variable and hands the result to a private seam the tests feed |  |
| `crates/lys-core/src/keys/identity_tests.rs` | The env-backed tests call the seam; no set_var, remove_var, unsafe or #[serial] |  |
| `crates/lys/src/commands/hex.rs` | hex_lower writes without a Result |  |
| `crates/lys-anchor-cli/src/commands/hex.rs` | hex_lower writes without a Result |  |
| `crates/lys-home/src/harness/claude_code/mod.rs` | Declarations and re-exports only after R6 |  |
| `crates/lys-core/tests/harness/mod.rs` | Declarations and re-exports only after R6, first line #![cfg(test)] |  |
| `crates/lys-anchor/tests/harness/mod.rs` | Declarations and re-exports only after R6, first line #![cfg(test)] |  |
| `crates/lys-anchor/src/upward/fixture.rs` | Test fixture; first line becomes #![cfg(test)] |  |
| `crates/lys-anchor/src/witness/fixture.rs` | Test fixture; first line becomes #![cfg(test)] |  |
| `crates/lys/src` | lys sources; its 10 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `crates/lys/tests` | lys integration tests; its 4 roots gain #![cfg(test)] as their first line |  |
| `crates/lys-anchor/src` | lys-anchor sources; its 17 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `crates/lys-anchor/tests` | lys-anchor integration tests; its 5 roots gain #![cfg(test)] as their first line |  |
| `crates/lys-anchor-cli/src` | lys-anchor-cli sources; its 6 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `crates/lys-anchor-cli/tests` | lys-anchor-cli integration tests; its 1 roots gain #![cfg(test)] as their first line |  |
| `crates/lys-core/src` | lys-core sources; its 33 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `crates/lys-core/tests` | lys-core integration tests; its 11 roots gain #![cfg(test)] as their first line |  |
| `crates/lys-home/src` | lys-home sources; its 20 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `crates/lys-home/tests` | lys-home integration tests; its 7 roots gain #![cfg(test)] as their first line |  |
| `crates/lys-log-store/src` | lys-log-store sources; its 2 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `docs/design/roadmap.json` | RM-035 carries this work |  |
| `docs/design/decisions.json` | ADR-054 and ADR-055 |  |
| `sgconfig.yml` | ast-grep root config; ruleDirs names rules/ast-grep | LYSCORE-003 |
| `rules/ast-grep` | lys's ast-grep rule directory, one <rule id>.yml per carried rule | LYSCORE-003 |
| `rules/ast-grep/mod-rs-declarations-only.yml` | Cambium's rule, its message citing lys's CLAUDE.md and vendor/** ignored | LYSCORE-003 |
| `rules/ast-grep/no-let-underscore-on-results.yml` | Cambium's rule, vendor/** ignored | LYSCORE-003 |
| `rules/ast-grep/no-lint-bypass-attributes.yml` | Cambium's rule, its message citing lys's CLAUDE.md and vendor/** ignored | LYSCORE-003 |
| `rules/ast-grep/no-unwrap-expect-panic-outside-tests.yml` | lys's rule: unwrap, expect and panic outside the four recognised test structures | LYSCORE-003 |
| `clippy.toml` | allow-unwrap-in-tests, allow-expect-in-tests, allow-panic-in-tests | LYSCORE-003 |
| `crates/lys-core/tests/harness/go.rs` | lys-core's Go-toolchain harness, moved whole out of harness/mod.rs | LYSCORE-003 |
| `crates/lys-anchor/tests/harness/go.rs` | lys-anchor's Go-toolchain harness and its two contract tests, moved whole out of harness/mod.rs | LYSCORE-003 |
| `crates/lys-home/src/harness/claude_code/names.rs` | the HARNESS, PROVIDER, API and AUTHORED constants, moved out of claude_code/mod.rs | LYSCORE-003 |
| `docs/design/lys-core/DESIGN-PRE-METHOD.md` | the hand-written pre-method design, renamed with its bytes unchanged | LYSCORE-003 |
| `docs/design/lys-core/CHECKLIST-PRE-METHOD.md` | the hand-written pre-method checklist C85 to C65, renamed with its bytes unchanged | LYSCORE-003 |
| `docs/design/lys-core/USER-STORIES-PRE-METHOD.md` | the hand-written pre-method stories S30 to S23, renamed with its bytes unchanged | LYSCORE-003 |
| `docs/design/lys-core/design.json` | this design | LYSCORE-003 |
| `docs/design/lys-core/checklist.json` | the rows LYSCORE-003 delivers | LYSCORE-003 |
| `docs/design/lys-core/stories.json` | the stories LYSCORE-003 serves | LYSCORE-003 |
| `docs/design/lys-core/briefs/LYSCORE-003.json` | the brief | LYSCORE-003 |
| `docs/design/lys-core/briefs/LYSCORE-003.md` | the brief, rendered | LYSCORE-003 |
| `docs/design/lys-core/DESIGN.md` | rendered from design.json once the hand-written file is renamed away |  |
| `docs/design/lys-core/CHECKLIST.md` | rendered from checklist.json once the hand-written file is renamed away |  |
| `docs/design/lys-core/USER-STORIES.md` | rendered from stories.json once the hand-written file is renamed away |  |
| `docs/design/project.json` | the project's gate trees; gains the ast-grep leg |  |
| `.land/gates.sh` | the landing gate repo_land runs as the whole gate; gains the ast-grep leg |  |
| `.github/workflows/ci.yml` | CI; its test job gains an ast-grep install and the scan |  |
| `CLAUDE.md` | coding standards; the test opt-out sentence names clippy.toml |  |
| `Cargo.toml` | workspace lint table, whose unwrap/expect/panic comment names clippy.toml, and workspace dependencies, which lose serial_test |  |
| `crates/lys-core/Cargo.toml` | lys-core's manifest; its dev-dependencies lose serial_test |  |
| `Cargo.lock` | the resolved dependency graph, without serial_test and the packages only it pulled in |  |
| `docs/PEN-REGISTRATION.md` | cites the pre-method lys-core design and checklist by path |  |
| `crates/lys-core/src/lib.rs` | crate root; hex_lower, and the comment above the unsafe_code attribute |  |
| `crates/lys-core/src/keys/identity.rs` | Ed25519Identity, including from_env |  |
| `crates/lys-core/tests/harness/mod.rs` | lys-core's Go-toolchain harness, logic in a mod.rs today |  |
| `crates/lys-anchor/tests/harness/mod.rs` | lys-anchor's Go-toolchain harness, logic in a mod.rs today |  |
| `crates/lys-home/src/harness/claude_code/mod.rs` | the Claude Code profile module; four constants in a mod.rs today |  |
| `crates/lys/src/commands/hex.rs` | the CLI's hex helper |  |
| `crates/lys-anchor-cli/src/commands/hex.rs` | the anchor CLI's hex helper |  |
| `crates/lys/src` | the CLI crate; sibling *_tests.rs files |  |
| `crates/lys/tests` | the CLI's integration tests |  |
| `crates/lys-anchor/src` | lys-anchor; sibling *_tests.rs files and the two fixture.rs files |  |
| `crates/lys-anchor/tests` | lys-anchor's integration tests |  |
| `crates/lys-anchor-cli/src` | the anchor CLI crate; sibling *_tests.rs files |  |
| `crates/lys-core/src` | lys-core; sibling *_tests.rs files |  |
| `crates/lys-core/tests` | lys-core's integration tests |  |
| `crates/lys-home/src` | lys-home; sibling *_tests.rs files |  |
| `crates/lys-home/tests` | lys-home's integration tests |  |
| `crates/lys-log-store/src` | lys-log-store; sibling *_tests.rs files |  |
| `crates/lys-home/src/harness/claude_code` | the Claude Code profile; sibling *_tests.rs files |  |
| `tests/identity_contract/tests` | the directory contract's integration test roots |  |

## Inventory

- `crates/lys-core/src/ca/authority.rs` — 455 lines with docs. Lines 368-374 refuse a certificate whose raw subject and issuer DN bytes are equal, before the algorithm and signature checks; lines 316-357 document that as a heuristic with a known hex-common-name false positive; lines 303-314 are verify_certificate_chain's two-line rustdoc; lines 194-207 the method form; lines 219-235 write the issuer DN as the lowercase hex of the authority key.
- `crates/lys-core/src/ca/authority_tests.rs` — 361 lines, 20 test functions; self_signed_certificate_is_rejected (line 163) checks an rcgen self-signed certificate under key [0u8; 32] and asserts only the error variant, so it passes through the signature branch; no test builds a hex-common-name certificate.
- `crates/lys-core/src/ca/certificate.rs` — certificate_subject_public_key (line 210) reads a certificate's 32-byte Ed25519 subject key: Ed25519 algorithm, no parameters, no unused bits, exactly 32 bytes.
- `crates/lys-core/src/ca/request_tests.rs` — a_request_is_not_accepted_as_a_certificate (line 489) passes a PKCS#10 request to verify_certificate_chain and expects CertificateParsing or CertificateVerification; a request does not parse as a certificate.
- `crates/lys/src/commands/ca.rs` — lys ca verify calls verify_certificate_chain_at (line 286) and maps every CertificateVerification to the one CertificateVerificationFailed message.
- `crates/lys/src/commands/verify.rs` — lys verify --cert calls verify_certificate_chain_at (line 139).
- `crates/lys/src/commands/error_tests.rs` — line 47 asserts the CertificateVerificationFailed display does not contain 'self-signed'.
- `crates/lys/tests/certified_attestation_tests.rs` — 380 lines; its Fixture generates a CA key, holders, presented-key certificates via lys ca request and lys ca issue --request, attestations, and runs lys verify --cert.
- `crates/lys-anchor/src/admission/certificate.rs` — module doc lines 55-56 say verify_certificate_chain 'rejects self-signed certificates'; line 222 calls it.
- `CHANGELOG.md` — the [Unreleased] section (line 12) carries lys-log-store and unstable-anchor additions not yet released.
- `docs/design/lys-core/DESIGN.md` — the earlier markdown design of this cluster; line 67 says 'Self-signed certificates are rejected' with no definition.
- `docs/design/lys-core/CHECKLIST.md` — the earlier markdown checklist, C1 to C65; C23 'Self-signed certificates are rejected by verify_certificate_chain' is ticked on the DN behaviour.
- `docs/design/lys-core/USER-STORIES.md` — the earlier markdown stories, S1 to S23; S5 and S11 are the consumers of chain verification this change reaches.
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
- `docs/design/lys-core/DESIGN-PRE-METHOD.md` — Renamed byte for byte from DESIGN.md in the brief's own commit. Hand-written pre-method design of the Phase 1/2 extraction (255 lines); no design.json beside it
- `docs/design/lys-core/CHECKLIST-PRE-METHOD.md` — Renamed byte for byte from CHECKLIST.md in the brief's own commit. Hand-written checklist C1 to C65 (101 lines); C4 records the forbid-to-deny relaxation for set_var tests under an explicit #[allow]
- `docs/design/lys-core/USER-STORIES-PRE-METHOD.md` — Renamed byte for byte from USER-STORIES.md in the brief's own commit. Hand-written stories S1 to S23 (55 lines)
- `docs/design/project.json` — Seven gate legs (fmt, clippy-all-features, clippy, tests, doc-all-features, doc, design); no ast-grep leg
- `.land/gates.sh` — Seven `leg` lines, run by the landing as the whole gate; no ast-grep
- `.github/workflows/ci.yml` — fmt, clippy and test steps on ubuntu-latest; ast-grep not installed
- `scripts/design/gate.sh` — Validates, checks coverage of, and byte-compares the rendering of every cluster with a design.json; lys-core has none yet
- `sgconfig.yml` — Absent
- `rules/ast-grep` — Absent
- `clippy.toml` — Absent
- `Cargo.toml` — Lines 69-70 say tests opt out per-module with #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]; unsafe_code = "deny"; unwrap_used, expect_used and panic warn
- `CLAUDE.md` — Line 35 says tests opt out per-module with #![allow]; the gates block lists six commands under 'All five clean' and no ast-grep
- `crates/lys-core/src/lib.rs` — Lines 27-30 explain the relaxed forbid through the set_var tests' #[allow(unsafe_code)]; line 31 is the attribute; line 59 is `let _ = s.write_fmt(...)` in hex_lower, and line 54 is the `use std::fmt::Write;` that write needs
- `crates/lys-core/src/keys/identity.rs` — from_env (line 241) reads LYS_IDENTITY_KEY with std::env::var and decodes it in place
- `crates/lys-core/src/keys/identity_tests.rs` — Five #[serial] env tests and the EnvCleanup guard carry the 7 #[allow(unsafe_code)] around set_var and remove_var (lines 847-1068)
- `crates/lys-anchor/src/upward/fixture.rs` — First line #![allow(clippy::unwrap_used, ...)]; declared only as #[cfg(test)] mod from pin.rs; 7 unwrap/expect/panic calls
- `crates/lys-anchor/src/witness/fixture.rs` — First line #![allow(clippy::unwrap_used, ...)]; declared only as #[cfg(test)] mod from report.rs; 15 unwrap/expect/panic calls
- `crates/*/src/**/*_tests.rs` — 88 sibling test files, 78 carrying #![allow]; none carries #![cfg(test)]
- `crates/*/tests/*.rs` — 28 integration test roots, 24 carrying #![allow], among them the 4 *_tests.rs roots of crates/lys
- `crates/lys-core/tests/harness/mod.rs` — 127 lines, #![allow] at line 18, 4 functions (mod-rs-declarations-only hits); lys-anchor's contract test reads this file's text
- `crates/lys-anchor/tests/harness/mod.rs` — 252 lines, #![allow] at line 45, 12 mod-rs-declarations-only hits including two #[test] fns
- `crates/lys-home/src/harness/claude_code/mod.rs` — 4 const items (HARNESS, PROVIDER, API, AUTHORED), mod-rs-declarations-only hits
- `crates/lys-home/src/record/mod.rs` — 630 lines, 47 mod-rs-declarations-only hits covering 521 lines on origin/main at 7b53625; split by HOME-013, not here
- `crates/lys/src/commands/hex.rs` — Line 15 `let _ = s.write_fmt(...)`
- `crates/lys-anchor-cli/src/commands/hex.rs` — Line 15 `let _ = s.write_fmt(...)`
- `vendor/rauthy` — Pinned Rauthy submodule (ADR-009), not initialised in a fresh clone
- `$cambium/rules/ast-grep/mod-rs-declarations-only.yml` — Source of the carried rule at cambium 1be80d8ec
- `$cambium/rules/ast-grep/no-let-underscore-on-results.yml` — Source of the carried rule at cambium 1be80d8ec
- `$cambium/rules/ast-grep/no-lint-bypass-attributes.yml` — Source of the carried rule at cambium 1be80d8ec
- `$cambium/rules/ast-grep/no-std-mutex-in-async.yml` — Not carried: lys has no async fn, tokio or std Mutex
- `$cambium/rules/ast-grep/no-timer-in-door-handlers.yml` — Not carried: lys has no door handlers
- `$cambium/rules/ast-grep/no-timer-import-in-door-handlers.yml` — Not carried: lys has no door handlers
- `docs/design/project.json` — One tree '.' with seven legs (fmt, clippy-all-features, clippy, tests, doc-all-features, doc, design); no ast-grep leg.
- `scripts/design/gate.sh` — Validates decisions.json and project.json and, for every cluster with a design.json, validates it, checks its coverage and compares its rendered markdown byte for byte. It runs no leg's command.
- `.land/gates.sh` — The gate repo_land runs as the whole gate: gate.sh, cargo fmt --check, both clippies, the tests and both cargo doc runs. No ast-grep line.
- `.github/workflows/ci.yml` — Job test runs fmt, clippy and tests; job audit runs cargo-deny. No ast-grep install and no scan.
- `sgconfig.yml` — Absent.
- `rules` — Absent.
- `clippy.toml` — Absent.
- `Cargo.toml` — Workspace lints: unwrap_used, expect_used and panic at warn, under a comment saying tests opt out per module with #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)].
- `CLAUDE.md` — Coding standards: tests opt out per module with #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]; the same file calls #[allow], #[ignore], _-prefixed variables and #[cfg(any())] a bypass.
- `crates` — 333 Rust files under crates/ and tests/ in eight workspace members; no async fn, no tokio and no std Mutex or RwLock. At 4dd1c33 the lys rules report 113 lint-bypass hits (106 test opt-outs and seven #[allow(unsafe_code)] in lys-core keys/identity_tests.rs, none outside test code), 19 let-underscore hits in 18 files, 20 mod-rs hits in three files and 224 unwrap/expect/panic hits in 49 files (22 in the two fixture.rs files, 202 in helper fns of 47 *_tests.rs files, none in library code).
- `crates/**/*_tests.rs` — 104 files: sibling test files under src/ included by a #[cfg(test)] mod in their parent, and four integration test roots under crates/lys/tests. None begins with #![cfg(test)]; 82 carry the unwrap/expect/panic opt-out.
- `crates/lys-anchor/src/witness/fixture.rs` — Line 1 is the opt-out; 15 unwrap/expect/panic calls; declared from report.rs as a #[cfg(test)] #[path] mod.
- `crates/lys-anchor/src/upward/fixture.rs` — Line 1 is the opt-out; 7 unwrap/expect/panic calls; declared from pin.rs as a #[cfg(test)] #[path] mod.
- `crates/*/tests and tests/*/tests (31 roots and two harness modules not named *_tests.rs)` — Integration test roots and the two tests/harness/mod.rs files. 22 of them carry the opt-out, and their helper fns are not test code to clippy unless the file is marked.
- `crates/lys-home/src/record/mod.rs` — 47 mod-rs hits on main at 7b53625; 0 at 4dd1c33, where HOME-013 moved its logic into home.rs, session.rs and helpers.rs.
- `crates/lys-anchor/tests/harness/mod.rs` — Reads ../lys-core/tests/harness/mod.rs as text in a contract test that checks lys-core's Go environment clauses.
- `crates/lys-core/src/keys/identity_tests.rs` — Five from_env tests mutate LYS_IDENTITY_KEY with std::env::set_var/remove_var under seven #[allow(unsafe_code)] lines, #[serial_test::serial] and an EnvCleanup guard.
- `docs/design/lys-core` — Pre-method cluster: DESIGN.md (255 lines), CHECKLIST.md (C85 to C65), USER-STORIES.md (S30 to S23); no design.json before this design.
- `crates/lys-core/tests/seal_derivation.rs` — Lines 7-8 cite docs/design/lys-core/DESIGN.md (D6) and CHECKLIST.md (C45).
- `docs/PEN-REGISTRATION.md` — Line 50 cites docs/design/lys-core/DESIGN.md and docs/design/lys-core/CHECKLIST.md.
- `crates/lys-core/src/lib.rs` — Lines 27-30 say test builds relax forbid because the env-backed tests call set_var under #[allow(unsafe_code)]; line 59 discards the Result of a String write.
- `vendor/rauthy` — A git submodule, empty until initialised; once initialised its Rust sources sit under the scan root, and ast-grep walks into it.
- `$cambium/sgconfig.yml` — ruleDirs: rules/ast-grep. Read only.
- `$cambium/rules/ast-grep` — Six error-severity rules at 1be80d8ec, unchanged since df4dca5: mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers. No unwrap/expect/panic rule and no _name rule. Read only.

## Constraints

- **CN1** — No unwrap, expect or panic call in a fixture or *_tests.rs file is rewritten: each such file's count of `.unwrap()`, `.expect(` and `panic!(` is not below its count at the base.
- **CN2** — No rule and no config names a file or a list of file names to exempt; test code is recognised by structure only (P1).
- **CN3** — No #[allow], #![allow], #[expect] or #[ignore] of any kind replaces a removed one, in tests or in library code.
- **CN4** — #![cfg_attr(not(test), forbid(unsafe_code))] in crates/lys-core/src/lib.rs stays byte-identical, and the card's diff to lib.rs changes comment lines plus lines 54 and 59 only.
- **CN5** — crates/lys-home/src/record/mod.rs is not changed by this work.
- **CN6** — No wire format, domain-separation tag, public API signature or public behaviour of lys-core changes; Ed25519Identity::from_env keeps its signature and its three error texts.
- **CN7** — vendor/rauthy is neither scanned nor edited, and no file under vendor/ is committed (ADR-009).
- **CN8** — Cambium's rules and config are not edited.
- **CN9** — The seven existing gate legs and their commands are unchanged in docs/design/project.json and .land/gates.sh.
- **CN10** — Only this card writes under docs/design/lys-core.
- **CN11** — The public API of lys-core, lys and lys-log-store does not change, and the AdmissionPolicy, LeafStore, Signer and AnchorTask declarations are byte-identical.
- **CN12** — No hit is cleared by #[allow], #[expect], #[ignore], #[cfg(any())], a rename to another underscore-prefixed name, or an ignores or files entry beyond vendor/**.
- **CN13** — No `let _ =` statement is introduced under crates/.
- **CN14** — Cargo.toml keeps map_err_ignore = "warn" and used_underscore_binding = "warn".
- **CN15** — Cambium's tree is not changed.
- **CN16** — docs/design/project.json, .land/gates.sh, .github/workflows/ci.yml and sgconfig.yml are not changed.
- **CN17** — The hand-written pre-method lys-core documents are not edited.
- **CN18** — The rule is at severity error.
- **CN19** — No wire format, domain-separation tag, test vector or signed fixture changes.
- **CN20** — The public API of lys-core, lys and lys-log-store does not change.
- **CN21** — No hit is cleared by an #[allow], an #[expect], an #[ignore], a _-prefixed rename, #[cfg(any())], or an ignores or files entry beyond tests/** in the unwrap rule and vendor/** in every rule.
- **CN22** — No unwrap, expect or panic call in a fixture.rs or *_tests.rs file is rewritten, moved out of its file or removed.
- **CN23** — Cambium's tree is not changed.
- **CN24** — The seven existing legs of docs/design/project.json keep their names, commands, requirements, cadence and order.
- **CN25** — Only this card writes under docs/design/lys-core. The pre-method documents are renamed, never edited or deleted.
- **CN26** — Every carried rule is at severity error.
- **CN27** — The attribute #![cfg_attr(not(test), forbid(unsafe_code))] in crates/lys-core/src/lib.rs does not change; the lines above it are comment lines and say what is true.
