# lys-core — what was asked, what it means, and what was written

## The words, as they were typed

An underscore-prefixed name is never used to quiet a lint in lys.

Tom's rule is that a binding is never renamed with a leading underscore to silence an unused warning.
The ast-grep brief 6747ce61 ruled this rule out of its own card, so it gets this one.

The card adds an ast-grep rule to the lys gate that refuses a new underscore-prefixed binding, and it handles each of the 272 such bindings in the tree today by what the binding does.
A guard held only to keep something alive gets a real name and is dropped by name where its life should end.
An unused trait parameter is either used or the trait is changed so it is not asked for.
A value that truly has no use is not bound at all.
The rule's own words say why it refuses, and the gate runs it with the other ast-grep legs.
Each change keeps behaviour the same, and the full lys test suite passes before and after.

## What the survey found, and its angles

The words ask for a lys card that adds an ast-grep rule to the lys gate refusing any binding whose name starts with an underscore, the usual way to silence an unused-variable warning. The same card clears every such binding in the tree by what it does. A guard kept only for its lifetime gets a real name and an explicit drop. A trait parameter an implementation ignores is either used or no longer asked for. A value with no use is not bound at all. Behaviour does not change, and the full test suite passes before and after. It is the rule that the draft ast-grep brief 6747ce61 (LYSCORE-001) left out for a card of its own, and it lands after that brief brings in lys's ast-grep configuration and gate leg.

### What the tree holds

- `Cargo.toml [workspace.lints.clippy]` — Line 87 sets map_err_ignore = "warn", which fails under -D warnings, so .map_err(|_| …) is refused. That is why 64 closures are written |_err|, and it conflicts with 'not bound at all'. Line 108 already sets used_underscore_binding = "warn", which catches only uses of an underscored binding, never the declaration itself.
- `crates/lys-core/src/{tlog/verify.rs, bundle/verify.rs, checkpoint/note.rs, receipt/*.rs, delegation/*.rs, attestation/encoding.rs, ca/*.rs, keys/identity.rs, seal/*.rs, merkle/*.rs}` — About 55 map_err(|_err| …) closures in library code. Most sit on verification paths that deliberately collapse every failure to one error (the non-oracle design), so the discarded error must not be 'used' by carrying it into the returned error.
- `crates/lys-core/src/seal/sealed_envelope.rs:235` — let (key_bytes, _derived_nonce) = derive_key_and_nonce(…). The derived nonce is intentionally not compared, and a comment explains why. This is a cryptographic file, so the CLAUDE.md adversarial-review rule sits beside the change.
- `crates/lys-anchor/src/admission/policy.rs:106 (AdmissionPolicy::admit) with trivial.rs:68,69,141 and certificate.rs:206` — The words' unused trait parameters. AcceptAll ignores both parameters, MaxSize ignores context and the certificate policy ignores submission. Each parameter is used by some implementation, so the trait cannot simply drop one.
- `crates/lys-log-store/src/store.rs:120 (LeafStore) and log_tests.rs:298-307` — The test fake LyingStore ignores _index, _bytes and _pin. LeafStore is the trait of the published lys-log-store 0.2.0, so changing it is a breaking public-API change.
- `crates/lys-anchor/src/keys/signer.rs:97 (Signer) with anchor/genesis_tests.rs:261; crates/lys-anchor-cli/src/commands/anchor/policy_tests.rs:18 (AnchorTask::run)` — More test fakes that ignore a trait parameter (_message, _policy).
- `crates/lys-log-store/src/file.rs:392 and crates/lys-core/src/keys/identity.rs:431` — #[cfg(not(unix))] stubs taking _dir and _path. Neither the Mac nor Dean's laptop compiles them, so no gate leg sees this change.
- `crates/lys-anchor/src/witness/observe_tests.rs:73 (staged) and the other fixtures returning (TempDir, …)` — The guard case, and the bulk of the work: 127 _dir plus about 40 other _tmp/_*_dir TempDir holders. Swapping them for the bare `_` pattern would drop the TempDir at the end of the statement and delete the directory under the test, which is a behaviour change.
- `crates/lys-core/src/keys/identity_tests.rs:855 (let _guard = EnvCleanup)` — Four environment-cleanup guards. LYSCORE-001 plans to remove the env mutation these guard, so the two cards overlap here.
- `crates/lys-core/src/merkle/tree.rs:68 (_marker: PhantomData)` — An underscore-prefixed struct field, not a binding. The rule's pattern has to be scoped so it neither refuses nor quietly skips this on a guess.
- `sgconfig.yml, rules/ast-grep/ (absent at 7b53625)` — The rule would live here, but neither exists yet. The draft brief 6747ce61 (LYSCORE-001) creates them.
- `docs/design/project.json, .land/gates.sh, .github/workflows/ci.yml` — The three places the lys gate is kept. None has an ast-grep leg today, so 'the other ast-grep legs' do not exist until LYSCORE-001 lands its `ast-grep scan --config sgconfig.yml` leg.
- `CLAUDE.md 'Coding standards'` — Already calls a _-prefixed unused variable a bypass, not a fix. The rule's message can cite it.
- `docs/design/lys-core/ (DESIGN.md, CHECKLIST.md, USER-STORIES.md)` — The cluster to continue. Its hand-written pre-method documents are renamed *-PRE-METHOD.md by LYSCORE-001 (draft ADR-045), and the design.json this card extends is that card's, still untracked.

### What was already decided

- lys-core DESIGN.md Constraints — No unwrap/expect/panic in library code, no file over 500 lines, every public item documented, cryptographic changes need an adversarial review, and this cluster changes behaviour only for the four deliberate breaks.
- lys-core CHECKLIST.md C61-C64 — The build gates are verified by CI rather than by reading source. C64 (tests in sibling *_tests.rs files) is still open.
- CLAUDE.md Coding standards — 'Silencing a lint with #[allow], an #[ignore]d test, a _-prefixed unused variable, or #[cfg(any())] is a bypass, not a fix. Fix the code.' This card turns that sentence into a gate.
- CLAUDE.md 'A test needs a second party' — Count what fired, not what passed. A drift injection proves nothing unless exactly one test fails, and it must be the test built for that check. The new rule needs a fire-proof.
- LYSCORE-001 draft (brief 6747ce61) design.json non_goals — 'A _name-rename rule and the 272 _-prefixed bindings: Cambium carries no such rule. A card of its own writes the rule and handles each binding by its act.' This is where the words come from.
- LYSCORE-001 draft solution — sgconfig.yml names rules/ast-grep with four rules at severity error. The leg is `ast-grep scan --config sgconfig.yml`, run in project.json, .land/gates.sh and CI, with vendor/** ignored.
- LYSCORE-001 draft CN3 and C6 — No hit is cleared by a _-prefixed rename, and no `let _ =` statement remains under crates/. So 'not bound at all' cannot become a `let _ = …;` statement.
- LYSCORE-001 draft CN2 — The public API of lys-core, lys and lys-log-store does not change. That fences off changes to LeafStore.
- LYSCORE-001 draft ADR-043/044/045 — Rules at severity error; carry only rules that can fire; pre-method documents renamed. They are proposed in that brief and not yet in this project's ledger.
- Cargo.toml workspace lints — pedantic at warn under -D warnings; map_err_ignore and used_underscore_binding enabled.

### What was measured

- Underscore-prefixed bindings found by an ast-grep identifier rule (let, fn param, closure param, match, for, tuple-struct pattern) at 7b53625: 303 unique nodes on 299 lines in 70 files
- Underscore-prefixed bindings the ast-grep rule missed and grep found (sealed_envelope.rs:235 _derived_nonce, cli_tests.rs:1664 _issuer_pub): 2, so at least 305 bindings in total
- The words' own count: 272, not reproduced; the measured count is 305
- Every underscore-prefixed identifier node, uses and fields included: 321 nodes, 43 distinct names, 76 files
- Bindings by crate: lys-core 188, lys-anchor 52, lys-home 44, lys-log-store 9, lys 9, lys-anchor-cli 1
- Bindings in test code vs non-test code: 227 test, 76 non-test
- Bindings by syntactic place: let patterns 220, closure params 65, fn params 12, match arms 3, for patterns 3, tuple-struct patterns 3
- Most common names: _dir 127, _err 64, _stdout 20, _index 9, _tmp 8, _size 6, _root_dir 6, _delegated_dir 6, _path 5, _guard 4
- map_err(|_err| …) or or_else(|_err| …) closures in non-test code: about 58 across lys-core, lys-anchor, lys-log-store and lys
- Unused trait parameters in non-test code: 4 (AdmissionPolicy::admit in trivial.rs ×3, certificate.rs ×1)
- Unused trait parameters in test fakes: 6 (LyingStore ×4, DecliningSigner ×1, RecordPolicy ×1)
- cfg(not(unix)) stubs with an underscored parameter: 2
- Underscore-prefixed struct fields: 1 (_marker in merkle/tree.rs)
- `let _x =` statements with a single identifier (not a tuple): 7
- ast-grep legs in the lys gate today (project.json, .land/gates.sh, ci.yml): 0
- sgconfig.yml / rules/ directory in the lys tree: absent / absent
- State of brief 6747ce61's lys-core design.json, checklist.json and stories.json: untracked in its clone at 7b53625, not committed
- Cambium ast-grep rules: 6 rules, none refusing underscore-prefixed bindings
- ast-grep installed on this Mac: 0.44.1
- Rust files under crates/: 275

### What it means for the other projects

- cambium — The rule set lys copies comes from Cambium, which has no rule against underscore-prefixed bindings. The Cambium tree is read-only for this card. The new rule is a candidate for Cambium to adopt later.
- aion — The card goes through the brief_card → sign-off → card_build_v3 → src_pr → src_land chain. The build's gate run needs ast-grep available wherever the leg runs, including Dean's laptop.

### The decisions it stands on

-  (new) — lys refuses underscore-prefixed bindings with an ast-grep rule at severity error, beside the LYSCORE-001 rule set. Cambium has no such rule, so this is lys's own policy and needs its own entry.
-  (new) — Only if the lead retires clippy::map_err_ignore to allow map_err(|_| …): that reverses a deliberate lint choice in Cargo.toml and should be recorded.
-  (new) — How a trait parameter ignored by some implementations is handled, especially for the published LeafStore where the trait cannot change. The ledger has no decision on this.

### What it requires

- rules/ast-grep holds one rule at severity error that reports any underscore-prefixed binding in let, parameter, closure, match, for and pattern positions under crates/, with vendor/** ignored.
- The rule's message says why it refuses.
- `ast-grep scan --config sgconfig.yml` reports zero hits from the new rule on the landed tree.
- An uncommitted scratch file with one underscore-prefixed binding makes the scan exit 1 with exactly one hit, from the new rule.
- The new rule runs through the same ast-grep leg in docs/design/project.json, .land/gates.sh and CI.
- Every TempDir, EnvCleanup or other guard binding has a name without a leading underscore and is dropped by an explicit drop(name) where its lifetime should end.
- No trait implementation declares an underscore-prefixed parameter.
- No binding under crates/ has a name beginning with an underscore.
- cargo test --workspace --all-features passes before and after, with the same count of tests run.
- Both clippy legs, fmt, both doc legs and the design gate pass.

### What must not change

- No behaviour change: every verification failure still collapses to the same single error variant as today.
- No wire format, domain-separation tag, test vector or signed fixture changes.
- The public API of lys-core, lys and lys-log-store (including the LeafStore trait) does not change.
- No hit is cleared by #[allow], #[expect], #[ignore], #[cfg(any())] or an ignores/files entry beyond vendor/**.
- No `let _ =` statement is introduced under crates/.
- Cambium's tree is not changed.
- The seven existing project.json legs keep their names, commands, requirements, cadence and order.
- The hand-written pre-method lys-core documents are not edited.

### What we must put in place first

- LYSCORE-001 (brief 6747ce61) lands sgconfig.yml, rules/ast-grep and the `ast-grep scan --config sgconfig.yml` leg in project.json, .land/gates.sh and CI; it lands after the mod.rs split card (brief a3186728).
- ast-grep is installed where the gate runs: Dean's laptop and CI.
- The lead answers the map_err_ignore question and the unused-trait-parameter question before the ~64 closures and 10 trait parameters are rewritten.

### The risks

- Replacing a guard binding with `_` drops the TempDir at the end of the statement and deletes the directory under the test. That changes behaviour and could make a test pass vacuously.
- Using a discarded error to satisfy the rule on a non-oracle verification path would leak which check failed, which is a security regression in trust code.
- Rewriting |_err| as .ok().ok_or(…) or .or(Err(…)) passes the gate but may be read as sidestepping map_err_ignore: the same bypass the card forbids, in another form.
- The cfg(not(unix)) stubs are compiled by no gate host, so a mistake there ships unseen.
- A rule pattern that under-matches (the first ast-grep rule used in this survey missed 2 of 305) would leave hits the gate never reports.
- A rule that over-matches, catching field names or `_` itself, would force needless edits.
- The 272 in the words does not match the 305 measured, so an acceptance check against 272 would pass or fail on the wrong number.
- The card overlaps LYSCORE-001 in identity_tests.rs, where the EnvCleanup guards may disappear, and in the lys-core cluster documents. Landing out of order conflicts.
- Changes to cryptographic files such as sealed_envelope.rs may trigger the adversarial-review requirement even though behaviour is unchanged.
- About 300 edits in 70 files make a large diff for review, mostly mechanical.

### Still open

- The workspace sets clippy::map_err_ignore under -D warnings, which refuses map_err(|_| …). For the ~64 |_err| closures, should the card keep that lint and rewrite each closure without a binding (for example .or(Err(…)) or .ok().ok_or(…)), or should it retire map_err_ignore? The sentence of the words it stands on: "A value that truly has no use is not bound at all.". Why only the lead can settle it: Cargo.toml:87 enables map_err_ignore, so the plain unbound form fails clippy. Most of these closures sit on non-oracle verification paths (tlog/verify.rs, bundle/verify.rs, checkpoint/note.rs), where 'using' the error would leak which check failed. Either way, someone reading the lint table or a verifier's code sees a different policy.
- When a trait parameter is ignored by one implementation but needed by another, or the trait is the published LeafStore, may that implementation write the bare `_` pattern instead of either using the parameter or changing the trait? The sentence of the words it stands on: "An unused trait parameter is either used or the trait is changed so it is not asked for.". Why only the lead can settle it: AdmissionPolicy::admit (lys-anchor/src/admission/policy.rs:113) needs both parameters across its implementations, so neither can be dropped. LeafStore (lys-log-store/src/store.rs:120) is in the published lys-log-store 0.2.0, where changing it breaks consumers. The sentence offers only 'used' or 'trait changed', and the tree allows neither without changing behaviour or API.
- Is the card's scope every underscore-prefixed binding the rule refuses (305 measured at 7b53625), not the 272 the words state? The sentence of the words it stands on: "The card adds an ast-grep rule to the lys gate that refuses a new underscore-prefixed binding, and it handles each of the 272 such bindings in the tree today by what the binding does.". Why only the lead can settle it: An ast-grep scan plus grep at 7b53625 finds at least 305 bindings in 70 files. The rule must land with zero hits, so the acceptance count a person checks against differs from the words.
- Does this card wait for LYSCORE-001 (brief 6747ce61) to land sgconfig.yml, rules/ast-grep and the ast-grep leg, or does it carry that configuration and leg itself if it lands first? The sentence of the words it stands on: "The rule's own words say why it refuses, and the gate runs it with the other ast-grep legs.". Why only the lead can settle it: The lys gate has no ast-grep leg: docs/design/project.json, .land/gates.sh and .github/workflows/ci.yml name none, and sgconfig.yml and rules/ are absent. The brief that adds them is still an untracked draft, so the landing order decides what reaches the gate.

### The units beyond the first

- Offer the underscore-binding rule to Cambium's rule set — Cambium carries no such rule and its tree is read-only to this card, so adopting it there is Cambium's own gated change.
- Retire or restate clippy::map_err_ignore in the lys workspace lints — Only if the lead chooses that route. Changing a deliberate lint choice is its own decision and record, separate from clearing the bindings.

### The smallest complete shape

One card, landing after LYSCORE-001. It adds rules/ast-grep/<rule>.yml at severity error, picked up by the existing `ast-grep scan --config sgconfig.yml` leg, with a message saying why it refuses. In the same diff it clears every underscore-prefixed binding (305 at 7b53625): guards named and dropped by name, ignored trait parameters handled as the lead rules, and unused values left unbound. The leg lands green, a scratch file shows the rule fires exactly once, and the full gate passes on Dean's laptop before and after.

## The roadmap row

- **RM-032** — Refuse underscore-prefixed bindings in lys and clear each one by what it does (fix, idea)
- Summary: lys calls a _-prefixed unused variable a bypass, not a fix, and nothing enforces it. This item adds rules/ast-grep/no-underscore-binding.yml at severity error, run by the ast-grep leg LYSCORE-001 lands, with a message saying why it refuses. It clears every underscore-prefixed binding the rule reports (303 in 70 files at 7b53625, where the words count 272) by what the binding does. Guards whose directory or state is still used are named and dropped by name, the eight test fixtures that hold a TempDir in an underscore-prefixed field name it and close it where the test ends, discarded errors and unused values, among them TempDirs returned beside an identity already in memory, are left unbound with clippy::map_err_ignore kept, and ignored trait parameters become a bare `_` with every trait unchanged. Behaviour does not change, and the full suite runs the same tests before and after.
- Asked by: tom on 2026-09-27T12:10:45+10:00
- Context: The underscore-binding card on the Lys board: the rule that the ast-grep brief 6747ce61 (LYSCORE-001) ruled out of its own card. The lead's five answers to this run's survey (map_err_ignore kept, with the unbound .ok().ok_or form; the bare `_` for ignored trait parameters; the scope is every binding the rule reports, not 272; the card waits for LYSCORE-001; the eight fixture guard fields named and closed in this card) are written into LYSCORE-002.
- Quote: An underscore-prefixed name is never used to quiet a lint in lys.

Tom's rule is that a binding is never renamed with a leading underscore to silence an unused warning.
The ast-grep brief 6747ce61 ruled this rule out of its own card, so it gets this one.

The card adds an ast-grep rule to the lys gate that refuses a new underscore-prefixed binding, and it handles each of the 272 such bindings in the tree today by what the binding does.
A guard held only to keep something alive gets a real name and is dropped by name where its life should end.
An unused trait parameter is either used or the trait is changed so it is not asked for.
A value that truly has no use is not bound at all.
The rule's own words say why it refuses, and the gate runs it with the other ast-grep legs.
Each change keeps behaviour the same, and the full lys test suite passes before and after.
- Cluster: lys-core; briefs: LYSCORE-002
- Notes: Further units, not written: 'Offer the underscore-binding rule to Cambium's rule set'. LYSCORE-002 waits on LYSCORE-001 (brief 6747ce61, roadmap RM-029 on draft/lys-core/6747ce61), which lands sgconfig.yml, rules/ast-grep and the ast-grep leg. RM-029 is not in this ledger, so the dependency is carried in the brief's blocked_by rather than in depends_on. Ids: RM-032, ADR-047, ADR-048 and LYSCORE-002 are the next after the highest in every lys brief clone and draft branch checked against 7b53625 (RM-031 and ADR-046 in the home clone c265f6c9; LYSCORE-001 on draft/lys-core/04213a71, 5f185fd4 and 6747ce61). The cluster's C, S, P and CN numbers start after the pre-method documents and both lys-core draft clusters (C75, S24, P5, CN9), so they merge without renumbering.

## The design

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
- Renaming PhantomData's `_marker` field in merkle/tree.rs — It is a type marker, not a held value, and a struct field declaration is not a binding the rule refuses. The marker field is kept as it is; nothing else in tree.rs is promised unchanged, and its two map_err closures are rewritten with the other discarded errors.
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
> - ADR-047 — lys refuses an underscore-prefixed binding with an ast-grep rule at severity error — lys carries its own ast-grep rule, no-underscore-binding, at severity error beside the rule set LYSCORE-001 lands. It refuses an underscore-prefixed identifier in every binding position, admits the bare `_`, and its message says why it refuses. Each existing binding is cleared by what it does rather than renamed. Rejected: relying on used_underscore_binding and review, which never see the declaration, and clearing hits by renaming, #[allow] or an ignores entry.
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
> - S25 (Test writer, Reading a test's fixtures) — As a test writer, I want a temporary directory's guard named and dropped by name so that I can see where the directory's life ends.
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
- S25 (Test writer, Reading a test's fixtures) — As a test writer, I want a temporary directory's guard named and dropped by name so that I can see where the directory's life ends.

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
- S25 (Test writer, Reading a test's fixtures) — As a test writer, I want a temporary directory's guard named and dropped by name so that I can see where the directory's life ends.

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

