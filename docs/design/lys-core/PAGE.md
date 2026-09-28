# lys-core — what was asked, what it means, and what was written

## The words, as they were typed

The lys gate (docs/design/project.json, run by scripts/design/gate.sh) has no ast-grep leg, so the rules the estate holds (no unwrap, expect or panic outside tests, no #[allow], no _name renames, no #[ignore]) are only caught where clippy happens to overlap them. Cambium carries an sgconfig.yml and an ast-grep leg; lys has neither. Add sgconfig.yml and a rules directory to lys with the same rule set Cambium carries, and a gate leg `ast-grep scan --config sgconfig.yml` in docs/design/project.json requiring tool:ast-grep, so the leg fails the gate on a hit. Defect 5 of Waffles' 2026-09-25 22:35 audit.

Rulings of the lead, Archie, given on 27 September 2026 to the runs 04213a71 and 0d44242f in answer to their round 1. Both runs took the answers and then failed while writing, the second when its author session collided with state left by the first. They are settled here, and the author reopens none of them.

Neither rewrite nor exemption list. The rule recognises test code by a marker in the file itself, and the two fixture files get that marker. Each fixture.rs gains the inner attribute #![cfg(test)] as its first line, which is the truth of the file already, since its only declaration is a #[cfg(test)] mod. The rule treats a file whose first item is the inner attribute #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under tests/ as test code, and reports unwrap, expect and panic everywhere else. That is structure the file carries, not a list of names. The 22 calls then are not hits, and no call in either file is rewritten. The brief corrects the words' sentence. At this commit no such call exists in library code, so the rule's acceptance is zero hits over the tree with the two fixture files recognised by their own attribute, and one scratch file without the marker, never landed, that the rule reports, to prove the rule fires.

The no-std-mutex-in-async rule is recorded as not carried, beside the door-timer rules, for the same reason. The sentence was wrong on the tree. The lys tree has no async fn, no tokio and no std Mutex at this commit, so the rule could not fire and would only look like cover. The brief records it as not carried with that finding, and names the act that brings it back. The card that lands the first async code in lys carries no-std-mutex-in-async with it, and its brief names this ruling.

The hand-written lys-core documents are renamed to *-PRE-METHOD.md before this card renders the cluster, so the earlier design is kept and not replaced. This card is the one owner of docs/design/lys-core, and the lys-gate and roots cards keep out of it. Answered by Archie, lead for lys.

Rulings of the lead, Archie, given on 27 September 2026 to the run 6747ce61-490d-4e21-85f2-bed31eee945b in answer to its rounds. That run took every answer and then failed before writing, when the account pool refused every session. They are settled here, and the author reopens none of them.

Yes. Each of the 92 sibling *_tests.rs files gains the inner attribute #![cfg(test)] as its first line, the same marker the fixtures carry. It states what is already true of the file, because its only declaration is a #[cfg(test)] mod in its parent. The rule keeps recognising test code by structure the file carries and never by its name. The 205 helper calls are then not hits, and none of them is rewritten. The acceptance of zero hits over the tree stands, with the scratch file that proves the rule fires. Answered by Archie, lead for lys.

Carry no-lint-bypass-attributes as Cambium has it, and remove the 113 #[allow] lines in this card. The test opt-outs are replaced by clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests, which clippy applies to code it knows is test code, including every file carrying #![cfg(test)]. The lys CLAUDE.md sentence and the Cargo.toml lint comment are corrected to name clippy.toml in place of the per-module allow. An #[allow] outside tests is fixed at its cause. If one cannot be fixed in this card, the brief brings it back to the lead as a question with its line, and it never becomes an exemption. The acceptance is clippy with -D warnings green and zero #[allow] hits. Answered by Archie, lead for lys.

Fixed in this card, except the 47 hits in crates/lys-home/src/record/mod.rs. The 19 let-underscore hits are fixed at their cause. The three on infallible String writes use a form that returns no Result, and the rest handle or propagate the Result they drop. The mod-rs hits in the two tests/harness/mod.rs files and the other remaining file move their logic into named sibling files. The 47 hits in lys-home record/mod.rs are the mod.rs split card's (yrikrOgr, brief a3186728). This card is blocked on that card's landing, checked with a command that counts non-module lines in that file on origin/main, and the leg lands only when the whole tree is at zero hits. Answered by Archie, lead for lys.

Not in this card. Cambium carries no _name rule, so the words' 'same rule set' does not include it, and the brief corrects the words' sentence that lists it among the rules caught. It records the rule as not carried, with the 272 bindings counted at this commit, and names a card of its own that writes the rule and handles each binding by its act. A keep-alive guard is held by a real name and dropped by name, and an unused trait parameter is used or the trait is changed. That card is filed on this board from the brief's finding. Answered by Archie, lead for lys.

Both, and CI as well if lys has a CI file that runs the gate legs. The leg goes into docs/design/project.json and into .land/gates.sh, since that is what the landing runs as the whole gate, so a hit is refused at landing on every path. The brief names each file it adds the line to, and the acceptance runs .land/gates.sh on the scratch file and shows the leg red. Answered by Archie, lead for lys.

Yes. Each of the 22 files gains #![cfg(test)] as its first line and loses its #![allow]. An integration test root is only ever compiled as a test, so the attribute changes nothing about what is built, and it lets clippy.toml's allow-*-in-tests cover the helper functions as it already covers the #[test] functions. The two tests/harness/mod.rs files take the same line as an inner attribute of their module. No #[allow] of any kind replaces the removed lines. Acceptance is that both clippy legs are green on the final tree and the no-lint-bypass-attributes leg reports zero hits, with the file count measured and recorded at the commit the brief is written against. Answered by Archie, lead for the identity line.

This card corrects the comment, and only the comment. A comment that says something the code no longer does is a defect, and it lands with the change that made it false. R5 rewrites the lines above the attribute at crates/lys-core/src/lib.rs so they say what is true after R5, with no names or dates in it, and the attribute itself stays exactly as it is, as the reviewer ruled. Whether to tighten the attribute so tests forbid unsafe code too is named under further units not written. An acceptance line asserts that the diff to lib.rs touches comment lines only. Answered by Archie, lead for the identity line.

## What the survey found, and its angles

Give lys the ast-grep leg its gate lacks. Add sgconfig.yml and rules/ast-grep with the rules Cambium carries that can fire on lys (mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes), plus the ruled no-unwrap/expect/panic rule. That rule treats code as test code by structure the file carries (#![cfg(test)] first, #[cfg(test)] mod, #[test] fn, or a path under tests/). Wire `ast-grep scan --config sgconfig.yml` into docs/design/project.json, .land/gates.sh and CI so any hit fails the gate. Under the lead's rulings the card also brings the tree to zero hits first. That means #![cfg(test)] on 2 fixtures, 92 *_tests.rs files and the integration test roots; the 113 #[allow] lines replaced by clippy.toml's allow-*-in-tests; the 19 `let _ =` fixed at their cause; and the mod.rs logic moved into named files. It renders the lys-core cluster after renaming the hand-written documents to *-PRE-METHOD.md, and it lands only after the lys-home record/mod.rs split card lands.

### What the tree holds

- `docs/design/project.json` — Holds the tree's seven gate legs (fmt, two clippy, tests, two doc, design) and no ast-grep leg. The new leg goes here with requires ["tool:ast-grep"]. project.schema.json says a leg's command is 'run in the tree with no shell', so `ast-grep scan --config sgconfig.yml` fits as written.
- `.land/gates.sh` — repo_land runs this file as the whole gate. It has seven `leg` lines and no ast-grep. The rulings put the leg here too, and its header comment says it runs the gates 'exactly as CLAUDE.md lists them'.
- `.github/workflows/ci.yml` — lys has a CI file that runs fmt, clippy and test legs, so by the ruling the ast-grep step goes here as well. The ubuntu runner does not install ast-grep today.
- `sgconfig.yml (absent)` — New. Cambium's version is just `ruleDirs: [rules/ast-grep]`.
- `rules/ast-grep/ (absent)` — New. Carries mod-rs-declarations-only.yml, no-let-underscore-on-results.yml and no-lint-bypass-attributes.yml copied from Cambium, plus a new no-unwrap/expect/panic rule that Cambium does not have. Cambium relies on clippy for that.
- `clippy.toml (absent)` — New. allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests replace the 106 per-module #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] lines. Cambium's clippy.toml sets all three to false, so lys takes the opposite setting from Cambium on this.
- `/Users/tom/Developer/ablative/apps/cambium/rules/ast-grep/*.yml` — The source rule set, six rules at cambium 1be80d8ec. no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers are ruled not carried. There is no _name rule.
- `crates/lys-anchor/src/upward/fixture.rs, crates/lys-anchor/src/witness/fixture.rs` — First line today is #![allow(clippy::unwrap_used, ...)], which becomes #![cfg(test)]. Each is declared only as `#[cfg(test)] #[path = "fixture.rs"] pub(super) mod fixture;` in pin.rs:61-63 and report.rs:24-26. Together they hold 22 unwrap/expect/panic calls.
- `crates/**/*_tests.rs (92 files)` — Each gains #![cfg(test)] as its first line. They hold 205 unwrap/expect/panic calls in 47 files that fall outside a #[test] fn or a #[cfg(test)] mod, and 78 of them carry the #![allow] line that is removed.
- `crates/*/tests/*.rs and crates/{lys-core,lys-anchor}/tests/harness/mod.rs` — 26 files under tests/ carry #![allow]: 24 roots and 2 harness/mod.rs. Both harness/mod.rs files are also mod-rs-declarations-only hits (4 and 12), so their logic moves into named sibling files.
- `crates/lys-core/src/keys/identity_tests.rs:849-1068` — Holds 7 #[allow(unsafe_code)] around unsafe std::env::set_var/remove_var calls under #[serial]. With the workspace setting unsafe_code = "deny", removing those allows means the env tests must stop mutating the process environment. That in turn touches how Identity::from_env (keys/identity.rs:241) is exercised.
- `crates/lys-core/src/lib.rs:27-31 and :59` — Lines 27-30 are the comment that explains the relaxed forbid through the set_var tests' #[allow(unsafe_code)]. R5 rewrites them and line 31, the attribute, stays as it is. Line 59, `let _ = s.write_fmt(...)` in hex_lower, is a no-let-underscore-on-results hit on a code line.
- `crates/lys/src/commands/hex.rs:15, crates/lys-anchor-cli/src/commands/hex.rs:15` — The other two infallible String writes among the 19 let-underscore hits.
- `crates/lys-home/src/harness/claude_code/mod.rs` — 4 mod-rs-declarations-only hits. Its logic moves to a named sibling in this card.
- `crates/lys-home/src/record/mod.rs` — 47 mod-rs hits and 630 lines. This belongs to the mod.rs split card (yrikrOgr, brief a3186728), which blocks the leg from landing.
- `CLAUDE.md:35 and Cargo.toml:69-70` — Both say tests opt out per-module with #![allow(...)]. Both are corrected to name clippy.toml. The CLAUDE.md 'Gates before any commit' block and its 'All five clean' line also list no ast-grep.
- `docs/design/lys-core/{DESIGN,CHECKLIST,USER-STORIES}.md` — Hand-written and pre-method, with no design.json. They are renamed to *-PRE-METHOD.md before render-cluster.py writes DESIGN.md, CHECKLIST.md and USER-STORIES.md at those names (render-cluster.py:209-229). CHECKLIST.md C4 describes the set_var #[allow] that R5 removes.
- `scripts/design/gate.sh` — Validates every cluster that has a design.json, checks its coverage, and compares the rendered markdown byte for byte. Once lys-core gains design.json it is measured for the first time. The *-PRE-METHOD.md files pass the byte comparison because the tmp copy carries them unchanged.

### What was already decided

- CLAUDE.md Coding standards — No unwrap/expect/panic in library code, with tests opting out per-module (the sentence to correct). #[allow], #[ignore], _-prefixed unused variables and #[cfg(any())] count as bypasses, not fixes.
- CLAUDE.md 'A test needs a second party' — Count what fired, not what passed. That is the ground for the ruled scratch file, which proves the rule fires, beside the zero-hit acceptance.
- CLAUDE.md 'How a carded row is built' rule 2 — The full chain on every card, with ast-grep named among the gates, although the lys gate has no ast-grep leg today.
- CLAUDE.md 'Gates before any commit' — Lists six commands under 'All five clean' and no ast-grep. .land/gates.sh claims to mirror this list exactly.
- docs/design/lys-core (pre-method DESIGN/CHECKLIST/USER-STORIES) — The Phase 1/2 extraction design. CHECKLIST C4 records the forbid→deny relaxation for env-backed set_var tests 'under an explicit #[allow]'. These are kept as *-PRE-METHOD.md.
- scripts/design/schemas/project.schema.json — A leg is name, command (run with no shell), requires (kind:value such as tool:…, or a shorthand) and cadence.
- Cambium docs/design/project.json ast-grep leg — Cambium runs `ast-grep scan --json=compact` with requires tool:ast-grep and cadence round. The words ask lys for `ast-grep scan --config sgconfig.yml`.
- ADR-009 — lys pins vendor/rauthy as a submodule. Once initialised it is a Rust tree the scan must not walk.

### What was measured

- Commit the survey read: 7b53625 (HEAD == origin/main)
- Cambium's ast-grep rules: 6 at cambium 1be80d8ec: mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, no-std-mutex-in-async, no-timer-in-door-handlers, no-timer-import-in-door-handlers. No unwrap rule and no _name rule.
- Hits from Cambium's rule set run over lys crates/ with ast-grep 0.44.1: 199: no-lint-bypass-attributes 113, mod-rs-declarations-only 67, no-let-underscore-on-results 19
- mod-rs hits by file: lys-home record/mod.rs 47; lys-anchor tests/harness/mod.rs 12; lys-core tests/harness/mod.rs 4; lys-home harness/claude_code/mod.rs 4
- Contents of the #[allow] hits: 106 are clippy::unwrap_used+expect_used+panic and 7 are unsafe_code (all in lys-core keys/identity_tests.rs). grep finds 114 lines; the extra one is the comment at lib.rs:29.
- unwrap/expect/panic calls outside #[test] fns, #[cfg(test)] mods and tests/: 227: 205 in 47 *_tests.rs files and 22 in the 2 fixture.rs files, with 0 in library code
- *_tests.rs files: 92; 78 of them carry #![allow]
- Files already carrying #![cfg(test)]: 0
- Rust files under crates/*/tests/ at depth one: 28
- Files under tests/ carrying #![allow]: 26: 24 roots and 2 harness/mod.rs. The ruling says 22, which does not match.
- #[ignore] in crates: 0
- async fn, tokio or std Mutex in crates: 0 lines in .rs and 0 tokio entries in any Cargo.toml
- _name bindings by crude count: 7 `let _x` and 30 `_x:` annotations. The ruled 272 counts a wider binding set that this survey did not reproduce.
- vendor/rauthy .rs files in this clone: 0 (submodule not initialised)
- crates/lys-home/src/record/mod.rs: 630 lines, about 437 not a module declaration or comment on origin/main (crude grep), so the split has not landed
- Hand-written lys-core documents: 3 files, 411 lines (DESIGN 255, CHECKLIST 101, USER-STORIES 55), and no design.json
- ast-grep on this Mac: 0.44.1 at ~/.cargo/bin, exits 1 on error-severity hits
- Toolchain: rust 1.97.1, clippy 0.1.97
- Gate legs today: 7 in project.json, 7 in .land/gates.sh, 3 run steps in ci.yml; none is ast-grep

### What it means for the other projects

- cambium — The source of the rule set, unchanged by this card. lys takes 3 of its 6 rules and adds an unwrap/expect/panic rule Cambium lacks, so the two rule sets now differ. lys's clippy.toml allows unwrap, expect and panic in tests where Cambium's forbids them. Neither project is edited to match the other.
- aion — The chain places the gate by each leg's requires. A new tool:ast-grep requirement means the gate place, Dean's laptop, must provide ast-grep or the leg cannot be placed, and card_build_v3 and src_land will run it on every lys card from here on.
- method — lys's scripts/design copies the method's schemas. The new leg must validate against project.schema.json, and the first lys-core design.json, stories.json, checklist.json and brief must pass validate.py and check-coverage.py.
- argus — Not touched.
- haematite — Not touched.

### The decisions it stands on

- ADR-009 (honour) — vendor/rauthy is a pinned upstream fork. The scan must exclude it, and no rule may push edits into the fork.
-  (new) — lys recognises test code by structure the file carries (#![cfg(test)] first, #[cfg(test)] mod, #[test] fn, tests/ path), both for ast-grep and for clippy's allow-*-in-tests, instead of per-module #![allow]. This reverses the opt-out that CLAUDE.md and Cargo.toml state.
-  (new) — Cambium rules are carried only where they can fire. no-std-mutex-in-async and the door-timer rules are recorded as not carried, and the first async card brings no-std-mutex-in-async back.

### What it requires

- sgconfig.yml exists at the repository root and names rules/ast-grep as its rule directory.
- rules/ast-grep holds mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes as Cambium has them, plus an error-severity rule for unwrap, expect and panic outside test code.
- docs/design/project.json has a leg running `ast-grep scan --config sgconfig.yml` with requires ["tool:ast-grep"], and scripts/design/validate.py accepts the file.
- .land/gates.sh runs `leg ast-grep scan --config sgconfig.yml`, and ci.yml has a step that installs ast-grep and runs the same scan.
- `ast-grep scan --config sgconfig.yml` reports zero hits over the tree at the landed commit, and exits 0.
- A scratch file with an unwrap and no #![cfg(test)] marker makes .land/gates.sh show the ast-grep leg red, and the scratch file is never landed.
- Both fixture.rs files and all 92 *_tests.rs files begin with #![cfg(test)], and no unwrap, expect or panic call in them is rewritten.
- Every integration test root and both tests/harness/mod.rs files carry #![cfg(test)], with the file count measured and recorded in the brief.
- clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true, and zero #[allow]/#![allow]/#[expect]/#[ignore] remain in crates/.
- Both clippy legs are green with -D warnings, and the tests, doc, fmt and design legs stay green.
- The 19 `let _ =` hits are gone: the 3 hex writes use a form that returns no Result, and the other 16 handle or propagate their Result.
- The tests/harness/mod.rs files and lys-home harness/claude_code/mod.rs hold only module declarations and re-exports, with their logic in named sibling files.
- The lys-core env tests no longer call unsafe set_var/remove_var, and Identity::from_env behaves exactly as before.
- The comment above #![cfg_attr(not(test), forbid(unsafe_code))] in lys-core/src/lib.rs says what is true after the change and names no names or dates, and the attribute is byte-identical.
- CLAUDE.md:35 and the Cargo.toml lint comment name clippy.toml instead of the per-module #![allow].
- docs/design/lys-core's hand-written DESIGN, CHECKLIST and USER-STORIES are renamed to *-PRE-METHOD.md, the rendered cluster files sit beside them, and scripts/design/gate.sh passes.
- The brief records no-std-mutex-in-async and the two door-timer rules as not carried with their findings, and records the _name rule as not carried with its binding count and a follow-up card.
- The leg lands only after origin/main's crates/lys-home/src/record/mod.rs has zero non-module lines, checked by command.

### What must not change

- No unwrap, expect or panic call in a fixture or *_tests.rs file is rewritten.
- No exemption list of file names in any rule: test code is recognised by structure only.
- No #[allow] of any kind replaces a removed one, in tests or library code.
- #![cfg_attr(not(test), forbid(unsafe_code))] in lys-core/src/lib.rs stays exactly as it is.
- crates/lys-home/src/record/mod.rs is not split in this card; the split belongs to yrikrOgr.
- The lys-gate and roots cards do not write into docs/design/lys-core; this card is its one owner.
- No wire format, domain-separation tag, public API signature or behaviour of lys-core changes.
- vendor/rauthy is not scanned or edited.
- Cambium's rules and config are not edited.
- The existing seven gate legs and their commands are unchanged.

### What we must put in place first

- The lys-home record/mod.rs split card (yrikrOgr, brief a3186728) lands on origin/main, confirmed by a command counting that file's non-module lines as zero.
- ast-grep is installed on the gate place (Dean's laptop), ideally at the version this Mac runs (0.44.1), so tool:ast-grep can be placed.

### The risks

- An initialised vendor/rauthy submodule is Rauthy's Rust tree. A scan from the root without an ignore turns the gate red on code lys does not own.
- clippy's is-in-test detection may not treat an inner #![cfg(test)] in a file or an integration-test root as test code the way it treats a #[cfg(test)] mod. The unwrap, expect and panic lints would then fire across 92+ files once the #![allow] lines go, so this needs proving on one crate first.
- An inner #![cfg(test)] beside the parent's outer #[cfg(test)] on the same module may trip clippy::duplicated_attributes under pedantic -D warnings.
- Replacing the set_var env tests touches the key-loading path of Identity::from_env in a published crypto crate. If the change moves anything the adversarial-review rule covers, it needs that review.
- Two rulings conflict over lib.rs: the line-59 hex write fix against the comment-only lib.rs acceptance.
- The ruled count of 22 integration files disagrees with the measured 24 roots and 2 harness files carrying #![allow]. A brief that trusts the ruled number misses files.
- ast-grep versions may differ between this Mac, Dean's laptop and CI, and rule-matching semantics can change between versions.
- Structural test-code matching in ast-grep (the first item is #![cfg(test)], or code inside a #[cfg(test)] mod) is easy to write too broadly. The scratch-file proof and per-marker cases guard against a rule that never fires.
- Two earlier runs collided on shared state. The brief must start from a clean clone at a named commit, not a folder.

### Still open

- crates/lys-core/src/lib.rs:59 is one of the three infallible `let _ = s.write_fmt(...)` hits ruled fixed in this card. Does that code-line change go ahead, with the lib.rs acceptance widened to 'comment lines plus line 59', or does lib.rs stay comment-only and hex_lower get fixed another way? The sentence of the words it stands on: "An acceptance line asserts that the diff to lib.rs touches comment lines only.". Why only the lead can settle it: Two rulings collide on one file. The let-underscore ruling ('The three on infallible String writes use a form that returns no Result') needs a code change at crates/lys-core/src/lib.rs:59, and this sentence forbids any non-comment change to lib.rs.
- CHECKLIST.md C4 in the kept pre-method lys-core documents says the env tests call set_var under an explicit #[allow], which is false after R5. Is it corrected in the renamed CHECKLIST-PRE-METHOD.md, or left as the historical record? The sentence of the words it stands on: "The hand-written lys-core documents are renamed to *-PRE-METHOD.md before this card renders the cluster, so the earlier design is kept and not replaced.". Why only the lead can settle it: It changes what is kept. Editing the kept file alters the earlier design the lead ruled kept, while leaving it keeps a sentence the code no longer bears out (docs/design/lys-core/CHECKLIST.md C4).

### The units beyond the first

- A _name binding rule for lys, each binding handled by its act — Ruled out of this card because Cambium carries no such rule. Its own card writes the rule and handles each of the 272 bindings: keep-alive guards named and dropped by name, unused trait parameters used or the trait changed.
- Carry no-std-mutex-in-async with the first async code in lys — Ruled not carried because there is no async, tokio or std Mutex today. The card that lands lys's first async fn carries it and names this ruling.
- Tighten lys-core's unsafe attribute so tests forbid unsafe code too — Once the env tests no longer need unsafe, the cfg_attr(not(test)) relaxation has no reason to exist, but the ruling keeps the attribute unchanged in this card.
- Split crates/lys-home/src/record/mod.rs (yrikrOgr, brief a3186728) — Owns 47 of the 67 mod-rs hits and blocks this card's leg. It is already its own card.

### The smallest complete shape

One card, landed after yrikrOgr. It carries: the lys-core cluster rendered beside the renamed *-PRE-METHOD.md documents; sgconfig.yml and rules/ast-grep with the three carried Cambium rules and the structural unwrap/expect/panic rule; clippy.toml's allow-*-in-tests; #![cfg(test)] on the 2 fixtures, 92 *_tests.rs files, integration roots and harness mods, with all 113 #[allow] removed and the env tests fixed at their cause; the 19 `let _ =` fixed; the mod.rs logic in the two harness mods and claude_code moved into named files; and the CLAUDE.md, Cargo.toml and lib.rs comments corrected. It ends with the ast-grep leg in project.json, .land/gates.sh and ci.yml, zero hits over the tree, and a never-landed scratch file shown red.

## The roadmap row

- **RM-035** — Give the lys gate an ast-grep leg with the rule set it can carry (process, idea)
- Summary: The lys gate has no ast-grep leg. Add sgconfig.yml and rules/ast-grep carrying Cambium's mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes plus a structural no-unwrap-expect-panic-outside-tests rule; recognise test code by a first-line #![cfg(test)] and clippy.toml's allow-*-in-tests in place of 106 per-module #![allow]; fix the 7 unsafe_code allows, the `let _ =` discards and the mod.rs logic at their cause; and wire `ast-grep scan --config sgconfig.yml` into docs/design/project.json, .land/gates.sh and CI, landing it only at zero hits after the record/mod.rs split. Also renders the lys-core cluster for the first time beside the kept *-PRE-METHOD.md documents.
- Asked by: tom on 2026-09-27T14:26:00+10:00
- Context: The lys-gate card's words, defect 5 of the estate audit of 25 September 2026, carried with the lead's rulings to the earlier runs 04213a71, 0d44242f and 6747ce61 appended as the card holds them. Written by the words_to_brief author at 7b53625 after the survey and the lead's answers of 27 September 2026 on lib.rs line 59 (the change goes ahead, the lib.rs acceptance widened to comment lines plus line 59), on line 54 (its now-unused import goes with the line 59 fix, the acceptance reading comment lines plus lines 54 and 59) and on the pre-method C4 (left as the historical record).
- Quote: The lys gate (docs/design/project.json, run by scripts/design/gate.sh) has no ast-grep leg, so the rules the estate holds (no unwrap, expect or panic outside tests, no #[allow], no _name renames, no #[ignore]) are only caught where clippy happens to overlap them. Cambium carries an sgconfig.yml and an ast-grep leg; lys has neither. Add sgconfig.yml and a rules directory to lys with the same rule set Cambium carries, and a gate leg `ast-grep scan --config sgconfig.yml` in docs/design/project.json requiring tool:ast-grep, so the leg fails the gate on a hit. Defect 5 of Waffles' 2026-09-25 22:35 audit.

Rulings of the lead, Archie, given on 27 September 2026 to the runs 04213a71 and 0d44242f in answer to their round 1. Both runs took the answers and then failed while writing, the second when its author session collided with state left by the first. They are settled here, and the author reopens none of them.

Neither rewrite nor exemption list. The rule recognises test code by a marker in the file itself, and the two fixture files get that marker. Each fixture.rs gains the inner attribute #![cfg(test)] as its first line, which is the truth of the file already, since its only declaration is a #[cfg(test)] mod. The rule treats a file whose first item is the inner attribute #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under tests/ as test code, and reports unwrap, expect and panic everywhere else. That is structure the file carries, not a list of names. The 22 calls then are not hits, and no call in either file is rewritten. The brief corrects the words' sentence. At this commit no such call exists in library code, so the rule's acceptance is zero hits over the tree with the two fixture files recognised by their own attribute, and one scratch file without the marker, never landed, that the rule reports, to prove the rule fires.

The no-std-mutex-in-async rule is recorded as not carried, beside the door-timer rules, for the same reason. The sentence was wrong on the tree. The lys tree has no async fn, no tokio and no std Mutex at this commit, so the rule could not fire and would only look like cover. The brief records it as not carried with that finding, and names the act that brings it back. The card that lands the first async code in lys carries no-std-mutex-in-async with it, and its brief names this ruling.

The hand-written lys-core documents are renamed to *-PRE-METHOD.md before this card renders the cluster, so the earlier design is kept and not replaced. This card is the one owner of docs/design/lys-core, and the lys-gate and roots cards keep out of it. Answered by Archie, lead for lys.

Rulings of the lead, Archie, given on 27 September 2026 to the run 6747ce61-490d-4e21-85f2-bed31eee945b in answer to its rounds. That run took every answer and then failed before writing, when the account pool refused every session. They are settled here, and the author reopens none of them.

Yes. Each of the 92 sibling *_tests.rs files gains the inner attribute #![cfg(test)] as its first line, the same marker the fixtures carry. It states what is already true of the file, because its only declaration is a #[cfg(test)] mod in its parent. The rule keeps recognising test code by structure the file carries and never by its name. The 205 helper calls are then not hits, and none of them is rewritten. The acceptance of zero hits over the tree stands, with the scratch file that proves the rule fires. Answered by Archie, lead for lys.

Carry no-lint-bypass-attributes as Cambium has it, and remove the 113 #[allow] lines in this card. The test opt-outs are replaced by clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests, which clippy applies to code it knows is test code, including every file carrying #![cfg(test)]. The lys CLAUDE.md sentence and the Cargo.toml lint comment are corrected to name clippy.toml in place of the per-module allow. An #[allow] outside tests is fixed at its cause. If one cannot be fixed in this card, the brief brings it back to the lead as a question with its line, and it never becomes an exemption. The acceptance is clippy with -D warnings green and zero #[allow] hits. Answered by Archie, lead for lys.

Fixed in this card, except the 47 hits in crates/lys-home/src/record/mod.rs. The 19 let-underscore hits are fixed at their cause. The three on infallible String writes use a form that returns no Result, and the rest handle or propagate the Result they drop. The mod-rs hits in the two tests/harness/mod.rs files and the other remaining file move their logic into named sibling files. The 47 hits in lys-home record/mod.rs are the mod.rs split card's (yrikrOgr, brief a3186728). This card is blocked on that card's landing, checked with a command that counts non-module lines in that file on origin/main, and the leg lands only when the whole tree is at zero hits. Answered by Archie, lead for lys.

Not in this card. Cambium carries no _name rule, so the words' 'same rule set' does not include it, and the brief corrects the words' sentence that lists it among the rules caught. It records the rule as not carried, with the 272 bindings counted at this commit, and names a card of its own that writes the rule and handles each binding by its act. A keep-alive guard is held by a real name and dropped by name, and an unused trait parameter is used or the trait is changed. That card is filed on this board from the brief's finding. Answered by Archie, lead for lys.

Both, and CI as well if lys has a CI file that runs the gate legs. The leg goes into docs/design/project.json and into .land/gates.sh, since that is what the landing runs as the whole gate, so a hit is refused at landing on every path. The brief names each file it adds the line to, and the acceptance runs .land/gates.sh on the scratch file and shows the leg red. Answered by Archie, lead for lys.

Yes. Each of the 22 files gains #![cfg(test)] as its first line and loses its #![allow]. An integration test root is only ever compiled as a test, so the attribute changes nothing about what is built, and it lets clippy.toml's allow-*-in-tests cover the helper functions as it already covers the #[test] functions. The two tests/harness/mod.rs files take the same line as an inner attribute of their module. No #[allow] of any kind replaces the removed lines. Acceptance is that both clippy legs are green on the final tree and the no-lint-bypass-attributes leg reports zero hits, with the file count measured and recorded at the commit the brief is written against. Answered by Archie, lead for the identity line.

This card corrects the comment, and only the comment. A comment that says something the code no longer does is a defect, and it lands with the change that made it false. R5 rewrites the lines above the attribute at crates/lys-core/src/lib.rs so they say what is true after R5, with no names or dates in it, and the attribute itself stays exactly as it is, as the reviewer ruled. Whether to tighten the attribute so tests forbid unsafe code too is named under further units not written. An acceptance line asserts that the diff to lib.rs touches comment lines only. Answered by Archie, lead for the identity line.
- Cluster: lys-core; briefs: LYSCORE-001
- Notes: Further units, named and not written: A _name binding rule for lys, each binding handled by its act; Carry no-std-mutex-in-async with the first async code in lys; Tighten lys-core's unsafe attribute so tests forbid unsafe code too; Split crates/lys-home/src/record/mod.rs (yrikrOgr, brief a3186728). The leg lands only after that split, HOME-013 under RM-028 on brief/home/a3186728; RM-028 is not on main at 7b53625, so depends_on stays empty and LYSCORE-001 carries it in blocked_by. Ids: RM-035, ADR-054 and ADR-055 are the next after the highest on main and on every open branch at 7b53625 (RM-034, ADR-053). LYSCORE-001 is the id the method named; the open branches brief/lys-core/5f185fd4 and b1ff26fe also hold a LYSCORE-001, and by the lead's ruling this card is the one owner of docs/design/lys-core.

## The design

---
type: design
cluster: lys-core
title: lys-core — the ast-grep leg and test code by structure
---

# lys-core — the ast-grep leg and test code by structure

> **Cluster:** lys-core

## Intention

The rules lys holds itself to are enforced by the gate, not by whoever happens to be reading the diff. A reader of CLAUDE.md, the rule directory, clippy.toml and the gate should find one policy stated four ways that agree, and a stranger should be able to run the same scan the landing runs and get the same answer.

Test code is recognised by what the file itself says it is, never by a list of names someone keeps. A file that is only ever compiled as a test says so in its first line, and both clippy and ast-grep read that line. Nothing is exempted, nothing is rewritten to dodge a rule, and nothing is silenced: where the tree broke a rule, the cause is fixed.

A rule is carried only where it can fire. A rule that cannot fire on this tree would only look like cover, and every claim of zero hits is paired with a scratch case the rule reports, so the rule is shown to fire before its silence is believed.

## Problem

The lys gate (docs/design/project.json, run by scripts/design/gate.sh and by .land/gates.sh at landing) has no ast-grep leg, so unwrap, expect and panic outside tests, #[allow] and #[ignore] are caught only where clippy happens to overlap them, and mod.rs logic and `let _ =` discards are not caught at all. Cambium carries sgconfig.yml, a rule directory and an ast-grep leg; lys has none of them. Measured at 7b53625 with Cambium's rules and ast-grep 0.44.1 over crates/: 199 hits (113 no-lint-bypass-attributes, 67 mod-rs-declarations-only, 19 no-let-underscore-on-results). The 113 bypasses are 106 per-module #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] test opt-outs and 7 #[allow(unsafe_code)] around set_var and remove_var in lys-core's env-backed identity tests. A structural unwrap/expect/panic rule finds 224 calls outside a #[test] fn, a #[cfg(test)] mod and tests/, all in the 2 fixtures and 47 sibling *_tests.rs files, and none in library code. The tree's own documents say the opposite of the policy the gate should enforce: CLAUDE.md line 35 and Cargo.toml lines 69-70 tell tests to opt out per module with #![allow], and the lib.rs comment above the unsafe attribute explains it by a set_var #[allow] that this work removes. The lys-core cluster itself is three hand-written pre-method documents with no design.json, so scripts/design/gate.sh has never measured it.

## Solution

Cluster documents. In the brief's own commit, the three hand-written documents are renamed to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md, byte for byte, before the cluster is rendered, and the rendering is committed with the JSON, so the earlier design is kept beside the rendered DESIGN.md, CHECKLIST.md and USER-STORIES.md. gate.sh compares only the markdown render-cluster.py writes, and its temporary copy carries the *-PRE-METHOD.md files unchanged, so they pass the byte comparison. The kept files are not edited (P6 applies to comments in code, not to a kept record). The card itself checks that state rather than performing it.

Rule set. sgconfig.yml at the repository root names rules/ast-grep, as Cambium's does. Of Cambium's six rules at 1be80d8ec, three can fire on lys and are carried with their bodies unchanged: mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes (which also catches #[ignore]). no-std-mutex-in-async and the two door-timer rules are not carried (ADR-055). Cambium carries no _name rule and no unwrap rule; lys adds one rule of its own, no-unwrap-expect-panic-outside-tests, which reports `.unwrap()`, `.expect(..)` and `panic!(..)` except inside test code as ADR-054 defines it. Every rule carries an `ignores` entry for vendor/**, so an initialised vendor/rauthy is never scanned (ADR-009); target/ is already skipped because ast-grep honours .gitignore. The command stays exactly `ast-grep scan --config sgconfig.yml`, run from the root with no path.

Test code by structure (ADR-054). clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true, and every file that is only ever compiled as a test begins with #![cfg(test)]: the 2 fixtures, the 88 sibling *_tests.rs files under src/, the 28 integration roots under tests/ (4 of which are the *_tests.rs roots of crates/lys) and the 2 tests/harness/mod.rs files. Measured with clippy 0.1.97 on a probe crate: a #[cfg(test)] mod ancestor already covers a sibling file's helpers, the integration roots and harness modules are covered only once they carry the marker, and the outer #[cfg(test)] on the parent's `mod x;` beside the inner marker trips no lint, so the outer attribute stays. With that, all 106 #![allow] test opt-outs go. The 7 #[allow(unsafe_code)] go by fixing their cause: from_env reads the variable and passes the read's result to a private seam that holds today's decoding, and the tests feed the seam directly instead of mutating the process environment, so no test needs unsafe. The lib.rs comment is then rewritten to say what is true, and the attribute under it is left exactly as it is.

Hits fixed at their cause. The 19 `let _ =` discards: the three hex writers take a form that returns no Result, the 14 test-code writes handle their Result, and the two discards of values that are not Results are removed with what they held. The lys-core hex writer at lib.rs line 59 takes the same no-Result form, and line 54's `use std::fmt::Write;`, which that form leaves unused, goes with it; no other code line of lib.rs changes. The logic in the two tests/harness/mod.rs files and in lys-home's harness/claude_code/mod.rs moves into named sibling files, leaving declarations and re-exports. The 47 hits in lys-home's record/mod.rs belong to HOME-013 and are not touched here.

The leg. `ast-grep scan --config sgconfig.yml` requiring tool:ast-grep is added to docs/design/project.json, as a `leg` line to .land/gates.sh, and as a pinned install and scan step to .github/workflows/ci.yml, so a hit is refused on every path to main. CLAUDE.md's gates block gains the line. The leg lands only when the whole tree is at zero hits, which needs HOME-013 landed first; a scratch file without the marker, never landed, shows the leg red through .land/gates.sh while every other leg stays green.

## Principles

- **P1** — Test code is recognised by structure the file carries: a first-line #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory. No rule and no config holds a list of file names.
- **P2** — A rule is carried only where it can fire on this tree; a rule that cannot fire is recorded as not carried with the finding and the act that brings it back.
- **P3** — Count what fired: every zero-hit claim is paired with a never-landed scratch case the rule reports, and each structural marker has a case of its own.
- **P4** — A bypass is fixed at its cause and never replaced by another bypass; one that cannot be fixed goes back to the lead as a question with its line.
- **P5** — One leg on every landing path: project.json, .land/gates.sh and CI run the same command.
- **P6** — A comment the code no longer bears out is corrected in the change that made it false.

## Decisions

- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-054 — lys recognises test code by structure the file carries, for ast-grep and clippy alike — Test code is a file whose first line is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory; every file only ever compiled as a test carries the first-line marker, and clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests replace the per-module #![allow]. Rejected: an exemption list of file names in the rule, and rewriting the test helpers' calls.
- ADR-055 — Cambium's ast-grep rules are carried into lys only where they can fire — lys carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, and records no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers as not carried with the finding that they cannot fire; the _name rule is its own card. Rejected: copying all six rules, which would look like cover the scan cannot give.

## Goals

- `ast-grep scan --config sgconfig.yml` from the repository root reports zero hits and exits 0 at the landed commit.
- A never-landed scratch file with an unwrap and no #![cfg(test)] turns the ast-grep leg of .land/gates.sh red while every other leg stays green.
- grep finds zero #[allow], #![allow], #[expect] and #[ignore] under crates/, and both clippy legs pass with -D warnings.
- docs/design/project.json, .land/gates.sh and .github/workflows/ci.yml each run `ast-grep scan --config sgconfig.yml`.
- sh scripts/design/gate.sh measures lys-core for the first time and exits 0, with the three pre-method documents kept byte for byte.

## Non-Goals

- A _name binding rule and the handling of each underscore-prefixed binding — Cambium carries no such rule, so the same rule set does not include it; a card of its own writes the rule and handles each binding by its act.
- Carrying no-std-mutex-in-async — lys has no async fn, no tokio and no std Mutex, so it cannot fire (ADR-055); the card that lands lys's first async code carries it.
- Carrying no-timer-in-door-handlers and no-timer-import-in-door-handlers — lys has no door handlers, so they cannot fire (ADR-055).
- Tightening #![cfg_attr(not(test), forbid(unsafe_code))] so tests forbid unsafe code too — The attribute stays exactly as it is in this work; tightening it is a further unit.
- Splitting crates/lys-home/src/record/mod.rs — Its 47 hits are HOME-013's, which this work waits on.
- Correcting CHECKLIST-PRE-METHOD.md's C4 — The kept documents are the historical record and stay exactly as they were; the rendered CHECKLIST.md is the current truth.
- A rule for todo!, unimplemented! and unreachable! — The words and the survey name unwrap, expect and panic; the other three stay with clippy's workspace lints.
- ast-grep rule tests (`ast-grep test`) in the repository — No gate leg would run them; the never-landed scratch cases in the brief's acceptance are the measurement the words ask for.
- Editing Cambium's rules or config to match lys — Neither project is edited to match the other; lys takes the opposite clippy.toml setting on tests from Cambium's.

## Structure

| Path | Note | Brief |
|------|------|-------|
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

## Inventory

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


---
type: brief
id: LYSCORE-001
cluster: lys-core
title: Give lys an ast-grep gate leg with the rule set it can carry, and bring the tree to zero hits
---

# LYSCORE-001: Give lys an ast-grep gate leg with the rule set it can carry, and bring the tree to zero hits

> **Cluster:** lys-core
> **Blocked by:** HOME-013 (roadmap RM-028, brief branch brief/home/a3186728), the split of crates/lys-home/src/record/mod.rs, must land on origin/main first: the leg lands only when R9's count command prints 0 for that file on origin/main., ast-grep installed at the gate place, at 0.44.1 as this brief was measured with, so a leg requiring tool:ast-grep can be placed.
> **Design anchor:**
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-054 — lys recognises test code by structure the file carries, for ast-grep and clippy alike — Test code is a file whose first line is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory; every file only ever compiled as a test carries the first-line marker, and clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests replace the per-module #![allow]. Rejected: an exemption list of file names in the rule, and rewriting the test helpers' calls.
> - ADR-055 — Cambium's ast-grep rules are carried into lys only where they can fire — lys carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, and records no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers as not carried with the finding that they cannot fire; the _name rule is its own card. Rejected: copying all six rules, which would look like cover the scan cannot give.
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

## Purpose

lys holds itself to rules its gate does not enforce: no unwrap, expect or panic outside tests, no lint bypass, no logic in mod.rs, no `let _ =` discard. This brief gives the gate an ast-grep leg, carries the Cambium rules that can fire on lys, adds the one rule Cambium relies on clippy for, and brings the tree to zero hits by fixing causes, so the leg can land green and refuse the next hit on every path to main (see the design's solution, ADR-054 and ADR-055).

## Task

Start from a clean clone of the lys repository at the commit this brief lands at, never a shared folder; two earlier runs collided on shared state. The brief's own commit already carries the rename of the three hand-written lys-core documents to *-PRE-METHOD.md and the rendered DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md, so the design leg is green when the brief lands. In order: check that the kept and rendered cluster documents still hold (R1); add sgconfig.yml and the four rules (R2); add clippy.toml and the #![cfg(test)] first line to the 90 src test files (R3) and the 30 files under tests/ (R4), removing all 106 #![allow] test opt-outs; replace the env mutation in lys-core's identity tests with a seam and correct the lib.rs comment (R5); move the mod.rs logic into named files (R6); fix the `let _ =` discards (R7); correct CLAUDE.md and Cargo.toml (R8); and wire the leg into project.json, .land/gates.sh and CI (R9). Heavy builds and full gates run at the gate place; only a single crate's clippy or tests runs where the work is written.

Corrections to the words' sentences. The words list no unwrap, expect or panic outside tests, no #[allow], no _name renames and no #[ignore] as rules the estate holds and ask for the same rule set Cambium carries. Measured at Cambium 1be80d8ec, Cambium carries six rules and none for unwrap, expect or panic (it relies on clippy) and none for _name bindings. So the rule set carried is three of Cambium's six (mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, the last covering #[allow] and #[ignore]) plus a structural unwrap/expect/panic rule of lys's own; the _name rule is not carried and is its own card; no-std-mutex-in-async and the two door-timer rules are not carried because they cannot fire on lys. The 22 fixture calls and the helper calls in the sibling test files are not hits because their files carry #![cfg(test)], and none of them is rewritten.

Counts measured at 7b53625, which this brief follows where a ruling's count differs: 88 sibling *_tests.rs under src/ plus the 4 *_tests.rs roots of crates/lys make the 92 *_tests.rs files; 28 integration roots and 2 harness modules under tests/ (26 of them carrying #![allow], where the ruling counted 22); 106 #![allow] test opt-outs and 7 #[allow(unsafe_code)]; 224 unwrap/expect/panic calls outside the structural test markers before the markers, 0 in library code, 0 after them.

The kept CHECKLIST-PRE-METHOD.md's C4 says the env-backed tests call set_var under an explicit #[allow]; after R5 the code no longer bears that sentence out, and the kept file is not corrected. The rendered CHECKLIST.md is the current truth. The pre-method C-numbers and this cluster's C-numbers are different lists; this brief's C-numbers are the rendered checklist's.

Out of scope, each its own unit: the _name rule, no-std-mutex-in-async, tightening the unsafe attribute so tests forbid unsafe code too, and the record/mod.rs split.

## Requirements

### R1: Check that the kept *-PRE-METHOD.md documents and the rendered cluster hold on the card's tree

Structural. The brief's own commit renamed docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md with their bytes unchanged, and carries DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md as scripts/design/render-cluster.py renders them from the cluster's JSON. This requirement checks that this state holds on the card's tree; it performs no rename and no render of its own. The kept files are the historical record and are not edited: CHECKLIST-PRE-METHOD.md's C4 says the env-backed tests call std::env::set_var under an explicit #[allow], which the code no longer bears out after R5, and the rendered CHECKLIST.md is the current truth. THE SYSTEM SHALL NOT edit a *-PRE-METHOD.md file, SHALL NOT hand-edit a rendered markdown file, and SHALL NOT write under any docs/design directory other than lys-core and docs/design/project.json.

**Acceptance:**
- `git show 7b53625:docs/design/lys-core/DESIGN.md | cmp - docs/design/lys-core/DESIGN-PRE-METHOD.md` exits 0.
- `git show 7b53625:docs/design/lys-core/CHECKLIST.md | cmp - docs/design/lys-core/CHECKLIST-PRE-METHOD.md` exits 0.
- `git show 7b53625:docs/design/lys-core/USER-STORIES.md | cmp - docs/design/lys-core/USER-STORIES-PRE-METHOD.md` exits 0.
- `test -f docs/design/lys-core/briefs/LYSCORE-001.md` exits 0.
- `sh scripts/design/gate.sh` exits 0 and prints no line containing `rendered markdown differs`.
- `git diff --name-only origin/main...HEAD -- docs/design | grep -v '^docs/design/lys-core/' | grep -vx 'docs/design/project.json'` prints nothing.

**Checklist:**
- C1 — DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md under docs/design/lys-core are byte-identical to DESIGN.md, CHECKLIST.md and USER-STORIES.md at 7b53625.
- C2 — DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md under docs/design/lys-core are what render-cluster.py renders, and sh scripts/design/gate.sh exits 0.

**Stories:**
- S10 (Design reader, Reading the lys-core cluster) — As a reader of the lys-core design, I want the hand-written pre-method documents kept beside the rendered cluster so that the earlier design is not lost when the method's documents replace it.

### R2: Add sgconfig.yml and rules/ast-grep with the three carried Cambium rules and the structural unwrap/expect/panic rule

Structural. sgconfig.yml at the repository root reads `ruleDirs:` with the one entry `rules/ast-grep`, as Cambium's does. In this requirement's acceptance, `$cambium` is the path of a local clone of the Cambium repository in which commit 1be80d8ec is present, set by whoever takes the measurement; `git -C $cambium cat-file -e 1be80d8ec` exits 0 in it. rules/ast-grep holds exactly four rules. mod-rs-declarations-only.yml, no-let-underscore-on-results.yml and no-lint-bypass-attributes.yml are Cambium's files at 1be80d8ec with one addition each: an `ignores` list holding `vendor/**`. no-unwrap-expect-panic-outside-tests.yml (severity error, language rust) matches `$RECV.unwrap()`, `$RECV.expect($$$ARGS)` and `panic!($$$ARGS)`, and does not report a match that sits in test code, which is: inside a source file whose first child is `#![cfg(test)]`; inside a mod item preceded by `#[cfg(test)]`; inside a function item preceded by `#[test]`, other attributes and comments between them allowed; and any file under `crates/*/tests/`, written as an `ignores` entry beside `vendor/**`. WHEN code outside those four structures calls unwrap, expect or panic, THE SYSTEM SHALL report it as an error-severity hit. THE SYSTEM SHALL NOT name a file or a list of file names in any rule, SHALL NOT recognise test code by a file's name, SHALL NOT scan vendor/, SHALL NOT edit vendor/rauthy, and SHALL NOT edit Cambium's rules or config. Recorded as not carried, each with its finding: no-std-mutex-in-async, because lys has no async fn, no tokio entry and no std Mutex at 7b53625, so it could not fire and would only look like cover; the card that lands lys's first async code carries it and its brief names this ruling. no-timer-in-door-handlers and no-timer-import-in-door-handlers, for the same reason: lys has no door handlers. The _name rule, because Cambium carries none, so the same rule set does not include it: the ruling counts 272 underscore-prefixed bindings at 7b53625 (not reproduced here: an ast-grep count of identifiers beginning with an underscore finds 303, and a grep for `let _x` and `_x:` finds 37), and a card of its own writes the rule and handles each binding by its act. #[ignore] is caught by the carried no-lint-bypass-attributes.

**Acceptance:**
- `cat sgconfig.yml` prints exactly the two lines `ruleDirs:` and `  - rules/ast-grep`.
- `ls rules/ast-grep` prints exactly mod-rs-declarations-only.yml, no-let-underscore-on-results.yml, no-lint-bypass-attributes.yml and no-unwrap-expect-panic-outside-tests.yml.
- `git -C $cambium cat-file -e 1be80d8ec` exits 0, before either acceptance line that reads from $cambium is taken.
- For each of the three carried rules, deleting its `ignores:` line and its `  - "vendor/**"` line leaves a file byte-identical to `git -C $cambium show 1be80d8ec:rules/ast-grep/<name>.yml`.
- `grep -c 'vendor/\*\*' rules/ast-grep/*.yml` prints 1 for each of the four files.
- A scratch file crates/lys-core/src/scratch_markers.rs holding the 14 lines `pub fn a(x: Option<u8>) -> u8 { x.unwrap() }`, `pub fn b(x: Option<u8>) -> u8 { x.expect("b") }`, `pub fn c() { panic!("c") }`, `#[cfg(test)]`, `mod t { fn h(x: Option<u8>) -> u8 { x.unwrap() } }`, `#[test]`, `fn tt() { None::<u8>.unwrap(); }`, `#[cfg(unix)]`, `#[test]`, `// comment`, `fn tt2() { None::<u8>.unwrap(); }`, `#[cfg(not(test))]`, `mod n { fn h(x: Option<u8>) -> u8 { x.unwrap() } }`, `fn not_test_fn() { None::<u8>.expect("x"); }` gives exactly 5 no-unwrap-expect-panic-outside-tests hits from `ast-grep scan --config sgconfig.yml --json=stream crates/lys-core/src/scratch_markers.rs`, at lines 1, 2, 3, 13 and 14; the file is deleted after.
- A scratch file crates/lys-core/src/scratch_first.rs whose two lines are `#![cfg(test)]` and `pub fn a(x: Option<u8>) -> u8 { x.unwrap() }` gives 0 hits from the same scan; the file is deleted after.
- A scratch file crates/lys-core/src/scratch_second.rs whose three lines are `//! doc`, `#![cfg(test)]` and `pub fn a(x: Option<u8>) -> u8 { x.unwrap() }` gives exactly 1 hit, at line 3; the file is deleted after.
- A scratch file crates/lys-core/tests/scratch_path.rs whose one line is `fn h(x: Option<u8>) -> u8 { x.unwrap() }` gives 0 hits from `ast-grep scan --config sgconfig.yml --json=stream`; the file is deleted after.
- A scratch file vendor/scratch_vendor.rs whose one line is `pub fn a(x: Option<u8>) -> u8 { x.unwrap() }` gives 0 hits from `ast-grep scan --config sgconfig.yml --json=stream`; the file is deleted after.
- `git -C $cambium status --porcelain rules sgconfig.yml clippy.toml` prints nothing.

**Files:**
- create: sgconfig.yml
- create: rules/ast-grep/mod-rs-declarations-only.yml
- create: rules/ast-grep/no-let-underscore-on-results.yml
- create: rules/ast-grep/no-lint-bypass-attributes.yml
- create: rules/ast-grep/no-unwrap-expect-panic-outside-tests.yml

**Checklist:**
- C3 — sgconfig.yml at the repository root names rules/ast-grep as its only rule directory.
- C4 — rules/ast-grep carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes with the rule bodies Cambium carries at 1be80d8ec.
- C5 — rules/ast-grep carries the error-severity rule no-unwrap-expect-panic-outside-tests, which reports unwrap, expect and panic outside test code recognised by structure.
- C6 — Every rule in rules/ast-grep ignores vendor/**.
- C7 — The brief records no-std-mutex-in-async, no-timer-in-door-handlers, no-timer-import-in-door-handlers and the _name rule as not carried, each with its finding.

**Stories:**
- S1 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the gate to refuse unwrap, expect and panic in library code so that a panic path cannot land where clippy is silenced.
- S2 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want a lint bypass attribute refused at landing so that a lint is fixed at its cause instead of hidden.
- S4 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want every rule shown to fire on a scratch case so that a silent scan means clean code rather than a rule that never matches.

### R3: Add clippy.toml's test allowances and mark the fixtures and sibling test files with #![cfg(test)]

Structural. clippy.toml at the repository root sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true and nothing else. Each of the 2 fixture.rs files and the 88 sibling *_tests.rs files under crates/*/src gains `#![cfg(test)]` as its first line, and the 80 of them that carry `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` lose that line. The outer #[cfg(test)] on each parent's `mod x;` declaration stays: measured with clippy 0.1.97, the pair trips no lint. THE SYSTEM SHALL NOT rewrite any unwrap, expect or panic call in these files, SHALL NOT put any #[allow], #![allow] or #[expect] in place of a removed line, and SHALL NOT change what any of these files declares.

**Acceptance:**
- `cat clippy.toml` prints exactly the three lines `allow-unwrap-in-tests = true`, `allow-expect-in-tests = true` and `allow-panic-in-tests = true`.
- `for f in $(find crates -name '*_tests.rs' -path '*/src/*') crates/lys-anchor/src/upward/fixture.rs crates/lys-anchor/src/witness/fixture.rs; do head -1 "$f"; done | sort | uniq -c` prints exactly `  90 #![cfg(test)]`.
- `grep -l '#!\[allow' $(find crates -name '*_tests.rs' -path '*/src/*') crates/lys-anchor/src/upward/fixture.rs crates/lys-anchor/src/witness/fixture.rs` prints nothing.
- For each of these 90 files, `grep -oE '\.unwrap\(\)|\.expect\(|panic!\(' <file> | wc -l` at HEAD is not below the same count at origin/main.
- `ast-grep scan --config sgconfig.yml --filter no-unwrap-expect-panic-outside-tests --json=stream | wc -l` prints 0.
- `cargo clippy -p lys-anchor --all-targets --all-features -- -D warnings` exits 0.

**Files:**
- create: clippy.toml
- modify: crates/lys-anchor-cli/src/cli_tests.rs
- modify: crates/lys-anchor-cli/src/commands/anchor/open_tests.rs
- modify: crates/lys-anchor-cli/src/commands/anchor/policy_tests.rs
- modify: crates/lys-anchor-cli/src/commands/error_tests.rs
- modify: crates/lys-anchor-cli/src/commands/hex_tests.rs
- modify: crates/lys-anchor-cli/src/commands/output_tests.rs
- modify: crates/lys-anchor/src/admission/certificate_tests.rs
- modify: crates/lys-anchor/src/admission/context_tests.rs
- modify: crates/lys-anchor/src/admission/trivial_tests.rs
- modify: crates/lys-anchor/src/anchor/append_tests.rs
- modify: crates/lys-anchor/src/anchor/artifact_tests.rs
- modify: crates/lys-anchor/src/anchor/checkpoint_tests.rs
- modify: crates/lys-anchor/src/anchor/genesis_tests.rs
- modify: crates/lys-anchor/src/anchor/open_tests.rs
- modify: crates/lys-anchor/src/anchor/proof_nodes_tests.rs
- modify: crates/lys-anchor/src/anchor/read_only_tests.rs
- modify: crates/lys-anchor/src/anchor/status_tests.rs
- modify: crates/lys-anchor/src/anchor/submit_tests.rs
- modify: crates/lys-anchor/src/keys/file_signer_tests.rs
- modify: crates/lys-anchor/src/upward/bundle_tests.rs
- modify: crates/lys-anchor/src/upward/pin_tests.rs
- modify: crates/lys-anchor/src/witness/observe_tests.rs
- modify: crates/lys-anchor/src/witness/projection_tests.rs
- modify: crates/lys-core/src/attestation/artifact_tests.rs
- modify: crates/lys-core/src/attestation/encoding_tests.rs
- modify: crates/lys-core/src/attestation/sign_tests.rs
- modify: crates/lys-core/src/bundle/verify_tests.rs
- modify: crates/lys-core/src/ca/authority_tests.rs
- modify: crates/lys-core/src/ca/certificate_tests.rs
- modify: crates/lys-core/src/ca/extensions_tests.rs
- modify: crates/lys-core/src/ca/request_tests.rs
- modify: crates/lys-core/src/cbor_tests.rs
- modify: crates/lys-core/src/checkpoint/body_tests.rs
- modify: crates/lys-core/src/checkpoint/note_tests.rs
- modify: crates/lys-core/src/checkpoint/verifier_key_tests.rs
- modify: crates/lys-core/src/delegation/artifact_tests.rs
- modify: crates/lys-core/src/delegation/encoding_tests.rs
- modify: crates/lys-core/src/delegation/sign_tests.rs
- modify: crates/lys-core/src/error_tests.rs
- modify: crates/lys-core/src/keys/compare_tests.rs
- modify: crates/lys-core/src/keys/identity_tests.rs
- modify: crates/lys-core/src/keys/ssh_tests.rs
- modify: crates/lys-core/src/merkle/consistency_tests.rs
- modify: crates/lys-core/src/merkle/leaf_tests.rs
- modify: crates/lys-core/src/merkle/proof_tests.rs
- modify: crates/lys-core/src/merkle/reconstruct_tests.rs
- modify: crates/lys-core/src/merkle/tree_tests.rs
- modify: crates/lys-core/src/receipt/artifact_tests.rs
- modify: crates/lys-core/src/receipt/consistency_tests.rs
- modify: crates/lys-core/src/receipt/encoding_tests.rs
- modify: crates/lys-core/src/receipt/sign_tests.rs
- modify: crates/lys-core/src/seal/authenticated_tests.rs
- modify: crates/lys-core/src/seal/sealed_envelope_tests.rs
- modify: crates/lys-core/src/tlog/artifact_tests.rs
- modify: crates/lys-core/src/tlog/build_tests.rs
- modify: crates/lys-core/src/tlog/verify_tests.rs
- modify: crates/lys-home/src/harness/claude_code/events_tests.rs
- modify: crates/lys-home/src/harness/claude_code/given_tests.rs
- modify: crates/lys-home/src/harness/claude_code/import_tests.rs
- modify: crates/lys-home/src/harness/claude_code/paths_tests.rs
- modify: crates/lys-home/src/harness/claude_code/render_tests.rs
- modify: crates/lys-home/src/harness/claude_code/seed_tests.rs
- modify: crates/lys-home/src/harness/claude_code/template_tests.rs
- modify: crates/lys-home/src/record/beside_tests.rs
- modify: crates/lys-home/src/record/blocks_tests.rs
- modify: crates/lys-home/src/record/call_tests.rs
- modify: crates/lys-home/src/record/canon_tests.rs
- modify: crates/lys-home/src/record/epilogue_tests.rs
- modify: crates/lys-home/src/record/fork_cut_tests.rs
- modify: crates/lys-home/src/record/fork_tests.rs
- modify: crates/lys-home/src/record/given_tests.rs
- modify: crates/lys-home/src/record/lantern_tests.rs
- modify: crates/lys-home/src/record/reader_tests.rs
- modify: crates/lys-home/src/record/recall_tests.rs
- modify: crates/lys-home/src/record/record_tests.rs
- modify: crates/lys-home/src/record/templates_tests.rs
- modify: crates/lys-log-store/src/file_tests.rs
- modify: crates/lys-log-store/src/log_tests.rs
- modify: crates/lys/src/cli_tests.rs
- modify: crates/lys/src/commands/ca_tests.rs
- modify: crates/lys/src/commands/duration_tests.rs
- modify: crates/lys/src/commands/error_tests.rs
- modify: crates/lys/src/commands/files_tests.rs
- modify: crates/lys/src/commands/hex_tests.rs
- modify: crates/lys/src/commands/log/status_tests.rs
- modify: crates/lys/src/commands/log/store_tests.rs
- modify: crates/lys/src/commands/output_tests.rs
- modify: crates/lys/src/commands/pem_tests.rs
- modify: crates/lys-anchor/src/upward/fixture.rs
- modify: crates/lys-anchor/src/witness/fixture.rs

**Checklist:**
- C8 — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true.
- C9 — Both fixture.rs files and the 88 sibling *_tests.rs files under src/ begin with #![cfg(test)] and carry no #![allow].

**Stories:**
- S5 (Lys contributor, Writing tests and library code) — As a lys contributor writing tests, I want a test file recognised by the marker it carries so that its helpers need no per-file lint opt-out and no one keeps an exemption list.

### R4: Mark the integration test roots and the two harness modules with #![cfg(test)]

Structural. Measured at 7b53625: crates/*/tests holds 28 integration test roots, 24 of which carry `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, and 2 tests/harness/mod.rs files, both carrying it: 30 files and 26 lines, where the ruling counted 22; this brief follows the measured count. Each of the 30 files gains `#![cfg(test)]` as its first line, the harness modules as an inner attribute of their module, and the 26 #![allow] lines are removed. An integration root is only ever compiled as a test, so the marker changes nothing that is built; measured with clippy 0.1.97, a root or harness helper calling unwrap or expect is reported without the marker and passes with it. THE SYSTEM SHALL NOT rewrite any unwrap, expect or panic call in these files and SHALL NOT put any #[allow], #![allow] or #[expect] in place of a removed line.

**Acceptance:**
- `ls crates/*/tests/*.rs | wc -l` prints 28.
- `for f in crates/*/tests/*.rs crates/*/tests/harness/mod.rs; do head -1 "$f"; done | sort | uniq -c` prints exactly `  30 #![cfg(test)]`.
- `grep -l '#!\[allow' crates/*/tests/*.rs crates/*/tests/harness/*.rs` prints nothing.
- For each of these 30 files, `grep -oE '\.unwrap\(\)|\.expect\(|panic!\(' <file> | wc -l` at HEAD is not below the same count at origin/main.
- `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
- `cargo clippy --all-targets -- -D warnings` exits 0.

**Files:**
- modify: crates/lys-anchor-cli/tests/anchor_cli.rs
- modify: crates/lys-anchor/tests/anchor_receipt_conformance.rs
- modify: crates/lys-anchor/tests/cascade.rs
- modify: crates/lys-anchor/tests/checkpoint_note_conformance.rs
- modify: crates/lys-anchor/tests/standalone_is_complete.rs
- modify: crates/lys-anchor/tests/stranger_verification.rs
- modify: crates/lys-core/tests/bundle_conformance.rs
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
- modify: crates/lys-home/tests/cached_index.rs
- modify: crates/lys-home/tests/claude_code_round_trip.rs
- modify: crates/lys-home/tests/fork.rs
- modify: crates/lys-home/tests/given_record.rs
- modify: crates/lys-home/tests/lantern_cli.rs
- modify: crates/lys-home/tests/lantern_home.rs
- modify: crates/lys-home/tests/launch_template.rs
- modify: crates/lys/tests/certified_attestation_tests.rs
- modify: crates/lys/tests/cli_tests.rs
- modify: crates/lys/tests/json_output_tests.rs
- modify: crates/lys/tests/log_tests.rs
- modify: crates/lys-anchor/tests/harness/mod.rs
- modify: crates/lys-core/tests/harness/mod.rs

**Checklist:**
- C10 — The 28 integration test roots and both tests/harness/mod.rs files begin with #![cfg(test)] and carry no #![allow].

**Stories:**
- S5 (Lys contributor, Writing tests and library code) — As a lys contributor writing tests, I want a test file recognised by the marker it carries so that its helpers need no per-file lint opt-out and no one keeps an exemption list.

### R5: Test from_env through a seam instead of the process environment, and correct the lib.rs comment

Ed25519Identity::from_env keeps its signature `pub fn from_env() -> TrustResult<Self>` and becomes one call: it reads LYS_IDENTITY_KEY with std::env::var and hands the read's Result to a private associated function, from_env_value, that holds today's trimming, decoding, length check and error texts unchanged. The five env-backed tests in identity_tests.rs call from_env_value with the value they used to put in the environment, and with Err(std::env::VarError::NotPresent) for the missing-variable case; they lose #[serial_test::serial], and the EnvCleanup guard with its two #[allow(unsafe_code)] is removed. identity_tests.rs is the only user of serial_test in the tree, so the dev-dependency goes with it: the line `serial_test.workspace = true` under [dev-dependencies] in crates/lys-core/Cargo.toml and the line `serial_test = "3"` under [workspace.dependencies] in Cargo.toml are removed, and Cargo.lock is regenerated by cargo so it no longer lists serial_test or serial_test_derive. WHEN from_env_value is given Ok of the standard base64 of 32 bytes, THE SYSTEM SHALL return the identity whose seed is those bytes; WHEN it is given an Err, THE SYSTEM SHALL return TrustError::KeyManagement saying LYS_IDENTITY_KEY is not set. The comment above `#![cfg_attr(not(test), forbid(unsafe_code))]` in crates/lys-core/src/lib.rs (lines 27-30) is rewritten to say what is true after this change: library code has no unsafe, non-test builds forbid it, test builds fall back to the workspace-level deny, and no test in the crate uses unsafe code. It names no person and no date. THE SYSTEM SHALL NOT change the attribute line, SHALL NOT change any code line of lib.rs other than lines 54 and 59, which R7 changes, SHALL NOT make from_env_value public, SHALL NOT change the three KeyManagement reason texts, and SHALL NOT mutate the process environment in any test, and SHALL NOT change any other dependency line in either Cargo.toml. This touches the key-loading path of a published crate, so the change is reviewed adversarially before landing, as the repository requires for cryptographic changes.

**Acceptance:**
- `grep -cE 'set_var|remove_var|unsafe|serial' crates/lys-core/src/keys/identity_tests.rs` prints 0.
- `grep -c 'pub fn from_env() -> TrustResult<Self>' crates/lys-core/src/keys/identity.rs` prints 1, and the body of from_env is the single expression `Self::from_env_value(std::env::var(KEY_ENV_VAR))`.
- from_env_value(Ok(STANDARD.encode([9u8; 32]))) returns an identity whose public_key_bytes() equals ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]).verifying_key().to_bytes().
- from_env_value(Ok(URL_SAFE_NO_PAD.encode([3u8; 32]))) returns an identity whose public_key_bytes() equals ed25519_dalek::SigningKey::from_bytes(&[3u8; 32]).verifying_key().to_bytes().
- from_env_value(Err(std::env::VarError::NotPresent)) returns TrustError::KeyManagement whose message contains `LYS_IDENTITY_KEY` and `not set`.
- from_env_value(Ok("not-base64!!!@@".to_string())) returns TrustError::KeyManagement whose message contains `invalid base64`.
- from_env_value(Ok(STANDARD.encode([1u8; 16]))) returns TrustError::KeyManagement whose message contains `decoded to 16 bytes, expected 32`.
- `grep -cx '#!\[cfg_attr(not(test), forbid(unsafe_code))\]' crates/lys-core/src/lib.rs` prints 1, and `grep -cE 'set_var|#\[allow' crates/lys-core/src/lib.rs` prints 0.
- Every line `git diff -U0 origin/main...HEAD -- crates/lys-core/src/lib.rs` adds or removes begins with `//` after its leading whitespace, except the removal of original line 54 `use std::fmt::Write;`, the removal of original line 59 `let _ = s.write_fmt(format_args!("{b:02x}"));`, and the one line added in place of line 59.
- `grep -rc serial_test crates Cargo.toml | grep -vc ':0$'` prints 0.
- `grep -c 'name = "serial_test' Cargo.lock` prints 0.
- `cargo test -p lys-core --all-features` exits 0.

**Files:**
- modify: crates/lys-core/src/keys/identity.rs
- modify: crates/lys-core/src/keys/identity_tests.rs
- modify: crates/lys-core/src/lib.rs
- modify: crates/lys-core/Cargo.toml
- modify: Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C11 — The env-backed tests of Ed25519Identity::from_env call no set_var, remove_var or unsafe code, and from_env keeps its signature and its error texts.
- C12 — The comment above #![cfg_attr(not(test), forbid(unsafe_code))] in lys-core's lib.rs says what is true after the env tests change, and the attribute is byte-identical.

**Stories:**
- S6 (Lys contributor, Writing tests and library code) — As a lys contributor reading the tree's rules, I want CLAUDE.md, Cargo.toml and the lib.rs comment to state the policy the gate enforces so that the written rule and the enforced rule agree.
- S7 (Lys contributor, Writing tests and library code) — As a lys contributor changing key loading, I want the env-backed identity tests to run without unsafe code so that the test build needs no lint bypass.

### R6: Move the logic out of the two tests/harness/mod.rs files and lys-home's claude_code/mod.rs into named files

Structural. crates/lys-core/tests/harness/mod.rs keeps its marker, a short module doc and `mod go;` with the re-exports its roots use; find_go, go_or_skip, build_go_tool and run_built_tool move to tests/harness/go.rs, whose module doc opens with the phrase `Shared Go-toolchain harness` that lys-anchor's contract test reads. crates/lys-anchor/tests/harness/mod.rs keeps its marker, its module doc and the declarations of go, scaffold and scaffold_tests with the re-exports its roots use; find_go, go_or_skip, build_go_tool and run_built_tool move to go.rs; GO_ENV, GoScaffold, its impl, ALL_SCAFFOLDS and lys_core_harness move to scaffold.rs, and lys_core_harness names ../lys-core/tests/harness/go.rs; the two #[test] functions move to scaffold_tests.rs unchanged. crates/lys-home/src/harness/claude_code/mod.rs keeps its module doc and declarations; HARNESS, PROVIDER, API and AUTHORED move with their docs to names.rs and are re-exported from mod.rs, so crate::harness::claude_code::HARNESS and the other three resolve as before. THE SYSTEM SHALL NOT change what any moved item does, SHALL NOT loosen the contract test's clause checks, and SHALL NOT touch crates/lys-home/src/record/mod.rs.

**Acceptance:**
- `ast-grep scan --config sgconfig.yml --filter mod-rs-declarations-only --json=stream crates/lys-core/tests/harness/mod.rs crates/lys-anchor/tests/harness/mod.rs crates/lys-home/src/harness/claude_code/mod.rs | wc -l` prints 0.
- `git diff --name-only origin/main...HEAD -- crates/lys-home/src/cli.rs crates/lys-home/src/record/given.rs crates/lys-home/src/harness/claude_code/events.rs crates/lys-home/src/harness/claude_code/template.rs crates/lys-home/src/record/mod.rs` prints nothing.
- `cargo test -p lys-anchor --all-features the_go_environment_contract_matches_the_one_lys_core_wrote_down` passes in every test binary that declares `mod harness`.
- Changing `"GOPROXY", "off"` in crates/lys-core/tests/harness/go.rs to `"GOPROXY", "direct"` makes `cargo test -p lys-anchor --all-features` fail the_go_environment_contract_matches_the_one_lys_core_wrote_down in every binary that declares `mod harness`, with no other test failing; the change is reverted after.
- `cargo test --workspace --all-features` exits 0.

**Files:**
- create: crates/lys-core/tests/harness/go.rs
- create: crates/lys-anchor/tests/harness/go.rs
- create: crates/lys-anchor/tests/harness/scaffold.rs
- create: crates/lys-anchor/tests/harness/scaffold_tests.rs
- create: crates/lys-home/src/harness/claude_code/names.rs
- modify: crates/lys-core/tests/harness/mod.rs
- modify: crates/lys-anchor/tests/harness/mod.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C14 — The tests/harness/mod.rs files of lys-core and lys-anchor and lys-home's harness/claude_code/mod.rs hold only module declarations and re-exports.

**Stories:**
- S8 (Lys contributor, Writing tests and library code) — As a lys contributor reading a module tree, I want mod.rs files to hold only declarations and re-exports so that logic is found in a named file.

### R7: Fix the `let _ =` discards at their cause

The three hex writers (crates/lys/src/commands/hex.rs:15, crates/lys-anchor-cli/src/commands/hex.rs:15 and crates/lys-core/src/lib.rs:59) take a form that returns no Result: each byte pushes its two lowercase digits from a 16-character digit table, and the `use std::fmt::Write;` and the deliberate-discard comment go. `s.push_str(&format!(..))` is not the form, because clippy's format_push_string refuses it under the workspace's pedantic lints. The 14 test-code writes to a String handle their Result with `.expect("writing to a String cannot fail")`: anchor_receipt_conformance.rs:163, sealed_envelope_tests.rs:286, bundle_conformance.rs:673 and :692, consistency_conformance.rs:52, consistency_receipt_conformance.rs:80, delegation_conformance.rs:188, delegation_vector.rs:616, receipt_conformance.rs:48, signed_note_crosscheck.rs:55, given_tests.rs:29, given_record.rs:47, cli_tests.rs:52 and log_tests.rs:81. The two discards of values that are not Results are removed with what they held: identity_tests.rs:589 `let _ = id.public_key_bytes();` is deleted; json_output_tests.rs:375 `let _ = recipient_pub;` is deleted with the binding at line 92, and the key-generate call at line 87 stays as a statement, since json_ok asserts its success. In crates/lys-core/src/lib.rs the new form is the one line that replaces line 59, the deliberate-discard comment above it goes, and line 54's `use std::fmt::Write;`, which the new form leaves unused, is removed; no other code line of lib.rs changes. IF a hex writer is given the bytes 0x00, 0x0f, 0xab, 0xff, THEN THE SYSTEM SHALL return `000fabff`. Each writer's measurement is a new test named hex_lower_writes_000fabff asserting that hex_lower(&[0x00, 0x0f, 0xab, 0xff]) equals "000fabff": in crates/lys/src/commands/hex_tests.rs and crates/lys-anchor-cli/src/commands/hex_tests.rs, beside the existing 0xa5 vectors, which stay as they are; and, because lib.rs gains no test, in crates/lys-core/src/ca/authority_tests.rs, the sibling test file of ca/authority.rs, which calls hex_lower, reaching it through crate::hex_lower. THE SYSTEM SHALL NOT discard a Result, SHALL NOT use `_ =` in place of `let _ =`, and SHALL NOT add any #[allow].

**Acceptance:**
- `ast-grep scan --config sgconfig.yml --filter no-let-underscore-on-results --json=stream | wc -l` prints 0.
- `cargo test -p lys --all-features --bin lys commands::hex::tests::hex_lower_writes_000fabff -- --exact` prints `test commands::hex::tests::hex_lower_writes_000fabff ... ok` and exits 0.
- `cargo test -p lys-anchor-cli --all-features --lib commands::hex::tests::hex_lower_writes_000fabff -- --exact` prints `test commands::hex::tests::hex_lower_writes_000fabff ... ok` and exits 0.
- `cargo test -p lys-core --all-features --lib ca::authority::tests::hex_lower_writes_000fabff -- --exact` prints `test ca::authority::tests::hex_lower_writes_000fabff ... ok` and exits 0.
- `grep -c 'write_fmt\|std::fmt::Write' crates/lys/src/commands/hex.rs crates/lys-anchor-cli/src/commands/hex.rs crates/lys-core/src/lib.rs` prints 0 for each file.
- `grep -c 'recipient_pub' crates/lys/tests/json_output_tests.rs` prints 0.

**Files:**
- modify: crates/lys/src/commands/hex.rs
- modify: crates/lys-anchor-cli/src/commands/hex.rs
- modify: crates/lys-core/src/lib.rs
- modify: crates/lys/src/commands/hex_tests.rs
- modify: crates/lys-anchor-cli/src/commands/hex_tests.rs
- modify: crates/lys-core/src/ca/authority_tests.rs
- modify: crates/lys-core/src/keys/identity_tests.rs
- modify: crates/lys/tests/json_output_tests.rs
- modify: crates/lys-anchor/tests/anchor_receipt_conformance.rs
- modify: crates/lys-core/src/seal/sealed_envelope_tests.rs
- modify: crates/lys-core/tests/bundle_conformance.rs
- modify: crates/lys-core/tests/consistency_conformance.rs
- modify: crates/lys-core/tests/consistency_receipt_conformance.rs
- modify: crates/lys-core/tests/delegation_conformance.rs
- modify: crates/lys-core/tests/delegation_vector.rs
- modify: crates/lys-core/tests/receipt_conformance.rs
- modify: crates/lys-core/tests/signed_note_crosscheck.rs
- modify: crates/lys-home/src/harness/claude_code/given_tests.rs
- modify: crates/lys-home/tests/given_record.rs
- modify: crates/lys/tests/cli_tests.rs
- modify: crates/lys/tests/log_tests.rs

**Checklist:**
- C15 — The `let _ =` hits in the three hex writers, the 14 test-code writes, identity_tests.rs and json_output_tests.rs are gone.

**Stories:**
- S9 (Lys contributor, Writing tests and library code) — As a lys contributor, I want no `let _ =` discard in the tree so that no error is swallowed without a decision.

### R8: Correct CLAUDE.md and the Cargo.toml lint comment to name clippy.toml and the ast-grep leg

Structural. The coding-standards sentence at CLAUDE.md:35 that tells tests to opt out per module with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` is replaced by one naming clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests and the #![cfg(test)] first line that marks a file as test code. The comment at Cargo.toml:69-70 is corrected the same way. CLAUDE.md's `Gates before any commit` block gains the line `ast-grep scan --config sgconfig.yml` after `cargo doc --no-deps`, and its count sentence `All five clean.` becomes `All seven clean.`, matching the seven lines the block then lists. THE SYSTEM SHALL NOT change any other rule CLAUDE.md states and SHALL NOT change any line inside Cargo.toml's [workspace.lints] tables other than those comment lines; the serial_test removal from [workspace.dependencies] is R5's.

**Acceptance:**
- `grep -c 'Tests opt out per-module' CLAUDE.md` prints 0, and `grep -c 'clippy.toml' CLAUDE.md` prints at least 1.
- `grep -c '#!\[allow' Cargo.toml` prints 0, and `grep -c 'clippy.toml' Cargo.toml` prints 1.
- `sed -n '/^## Gates before any commit/,/^All /p' CLAUDE.md | grep -c '^ast-grep scan --config sgconfig.yml$'` prints 1, and the block's last line is `All seven clean. No exceptions.`
- `diff <(git show origin/main:Cargo.toml | sed -n '/^\[workspace\.lints/,$p' | grep -v '^#') <(sed -n '/^\[workspace\.lints/,$p' Cargo.toml | grep -v '^#')` prints nothing, over the [workspace.lints.rust] and [workspace.lints.clippy] tables that end the file.

**Files:**
- modify: CLAUDE.md
- modify: Cargo.toml

**Checklist:**
- C16 — CLAUDE.md's coding-standards sentence and the Cargo.toml lint comment name clippy.toml's allow-*-in-tests in place of the per-module #![allow].
- C17 — CLAUDE.md's gates block lists `ast-grep scan --config sgconfig.yml` and its count sentence matches the lines it lists.

**Stories:**
- S6 (Lys contributor, Writing tests and library code) — As a lys contributor reading the tree's rules, I want CLAUDE.md, Cargo.toml and the lib.rs comment to state the policy the gate enforces so that the written rule and the enforced rule agree.

### R9: Wire `ast-grep scan --config sgconfig.yml` into project.json, .land/gates.sh and CI, and land it only at zero hits

docs/design/project.json gains, after the design leg, the leg named `ast-grep` with command `ast-grep scan --config sgconfig.yml`, requires ["tool:ast-grep"] and cadence round. .land/gates.sh gains the line `leg ast-grep scan --config sgconfig.yml` after its last cargo doc leg. .github/workflows/ci.yml's test job gains a step running `cargo install ast-grep --version 0.44.1 --locked` and a step running `ast-grep scan --config sgconfig.yml`. WHEN any rule in rules/ast-grep reports a hit, THE SYSTEM SHALL fail the ast-grep leg in each of the three places. WHILE origin/main's crates/lys-home/src/record/mod.rs has any non-module line, THE SYSTEM SHALL NOT land this leg; the count is taken by `git show origin/main:crates/lys-home/src/record/mod.rs | ast-grep scan --rule rules/ast-grep/mod-rs-declarations-only.yml --stdin --json=stream | python3 -c "import sys,json; print(len({n for h in map(json.loads,sys.stdin) for n in range(h['range']['start']['line'],h['range']['end']['line']+1)}))"`, which prints 521 at 7b53625 and must print 0. THE SYSTEM SHALL NOT change the seven existing legs or their commands, SHALL NOT pass a path to the scan, and SHALL NOT land any scratch file.

**Acceptance:**
- `python3 scripts/design/validate.py docs/design/project.json` exits 0, and the file's last leg is {"name": "ast-grep", "command": "ast-grep scan --config sgconfig.yml", "requires": ["tool:ast-grep"], "cadence": "round"}.
- `git diff origin/main...HEAD -- docs/design/project.json .land/gates.sh | grep '^-[^-]'` prints nothing.
- `git diff --name-only origin/main...HEAD -- docs/design | grep -v '^docs/design/lys-core/'` prints exactly `docs/design/project.json`.
- `grep -cx 'leg ast-grep scan --config sgconfig.yml' .land/gates.sh` prints 1.
- `grep -c 'cargo install ast-grep --version 0.44.1 --locked' .github/workflows/ci.yml` prints 1, and `grep -c 'run: ast-grep scan --config sgconfig.yml' .github/workflows/ci.yml` prints 1.
- The record/mod.rs count command above prints 0 against origin/main before the leg lands.
- `ast-grep scan --config sgconfig.yml` from the repository root prints no hit and exits 0.
- With a scratch file crates/lys-core/src/scratch_red.rs whose one line is `pub fn scratch(x: Option<u8>) -> u8 { x.unwrap() }`, `sh .land/gates.sh` exits 1, prints `--- status 1: ast-grep scan --config sgconfig.yml ---`, and prints `--- status 0:` for each of its other seven legs; the file is deleted after and is in no commit.
- `grep -rnE '#!?\[(allow|expect)\(|#\[ignore' crates --include='*.rs'` prints nothing.

**Files:**
- modify: docs/design/project.json
- modify: .land/gates.sh
- modify: .github/workflows/ci.yml

**Checklist:**
- C13 — grep finds zero #[allow], #![allow], #[expect] and #[ignore] under crates/.
- C18 — docs/design/project.json has a leg running `ast-grep scan --config sgconfig.yml` requiring tool:ast-grep, and validate.py accepts the file.
- C19 — .land/gates.sh runs `leg ast-grep scan --config sgconfig.yml`.
- C20 — .github/workflows/ci.yml installs ast-grep 0.44.1 and runs `ast-grep scan --config sgconfig.yml`.
- C21 — `ast-grep scan --config sgconfig.yml` reports zero hits and exits 0 at the landed commit.
- C22 — A never-landed scratch file with an unwrap and no #![cfg(test)] makes .land/gates.sh show the ast-grep leg red and every other leg green.
- C23 — The leg lands only after origin/main's crates/lys-home/src/record/mod.rs has zero non-module lines, checked by command.

**Stories:**
- S1 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the gate to refuse unwrap, expect and panic in library code so that a panic path cannot land where clippy is silenced.
- S2 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want a lint bypass attribute refused at landing so that a lint is fixed at its cause instead of hidden.
- S3 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the same scan run by the design gate, the landing gate and CI so that no path to main skips it.
- S4 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want every rule shown to fire on a scratch case so that a silent scan means clean code rather than a rule that never matches.

## Boundaries

- No unwrap, expect or panic call in a fixture, a *_tests.rs file or a file under tests/ is rewritten.
- No rule, config or script names a file or a list of file names to exempt; test code is recognised by structure only.
- No #[allow], #![allow], #[expect] or #[ignore] of any kind is added, in tests or in library code; an #[allow] that cannot be fixed at its cause goes back to the lead as a question with its line.
- #![cfg_attr(not(test), forbid(unsafe_code))] in crates/lys-core/src/lib.rs stays byte-identical, and no code line of lib.rs changes other than lines 54 and 59.
- crates/lys-home/src/record/mod.rs is not changed.
- Nothing is written under any docs/design directory other than lys-core, except the one ast-grep leg in docs/design/project.json.
- No wire format, domain-separation tag, public API signature or public behaviour of any crate changes.
- vendor/rauthy is neither scanned nor edited, and no file under vendor/ is committed.
- Cambium's rules and config are not edited.
- The seven existing gate legs and their commands are unchanged.
- No scratch file is committed.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps; each exits 0.
- From the repository root: `ast-grep scan --config sgconfig.yml` prints no hit and exits 0.
- From the repository root: `sh scripts/design/gate.sh` exits 0.
- From the repository root: `sh .land/gates.sh` exits 0 with eight `--- status 0:` lines, then again with the R9 scratch file, exiting 1 with the ast-grep leg the only `--- status 1:` line; the scratch file is deleted after.
- `grep -rnE '#!?\[(allow|expect)\(|#\[ignore' crates --include='*.rs'` prints nothing.
- An adversarial review of the crates/lys-core/src/keys/identity.rs diff shows the seed still decoded into Zeroizing buffers, the three KeyManagement reason texts unchanged, and no key material in any error or Debug output.
- `git status --porcelain` after the scratch cases prints nothing that is not part of the card's diff.

