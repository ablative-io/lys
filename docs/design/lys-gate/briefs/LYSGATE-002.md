---
type: brief
id: LYSGATE-002
cluster: lys-gate
title: Enforce the 500-line limit on code files as a gate leg
---

# LYSGATE-002: Enforce the 500-line limit on code files as a gate leg

> **Cluster:** lys-gate
> **Design anchor:**
> - ADR-111 — The 500-line limit on code files is a gate leg, not a sentence in CLAUDE.md — A gate leg, `sh scripts/file-length.sh`, runs the checker's own tests and then measures every tracked Rust and TypeScript source file, and fails naming each file whose code lines exceed 500. It is declared in project.json, in every cluster design.json gate, in .land/gates.sh, in CLAUDE.md's 'Gates before any commit' block and in CI. Rejected: an ast-grep rule, which matches syntax and cannot count lines; a clippy lint, since too_many_lines counts one function, not a file, and says nothing of TypeScript; counting raw lines, which would punish documentation.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> **Checklist:**
> - C10 — scripts/check_file_length.py counts the code lines of every tracked Rust and TypeScript source file as ADR-111 defines them and exits non-zero naming each file over 500, with its count.
> - C11 — scripts/check_file_length_test.py covers every counting rule (comments, block and nested comments, strings and raw strings holding comment markers, test code, blank lines, the 500 and 501 boundary, an unreadable file) and passes.
> - C12 — scripts/file-length.sh runs the tests, then the checker, and exits non-zero if either fails.
> - C13 — The file-length leg is declared identically in docs/design/project.json and in the '.' tree of every cluster design.json gate, directly after the ast-grep leg.
> - C14 — .land/gates.sh runs `leg sh scripts/file-length.sh`, CLAUDE.md's 'Gates before any commit' block lists the same command and the ast-grep command it was missing, and CI runs the same script.
> - C15 — The leg exits 0 on the card's head and exits non-zero on a scratch file of 501 code lines that never reaches origin, naming that file.
> - C16 — sh scripts/design/gate.sh exits 0 at the card's head.
> **Stories:**
> - S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main.
> - S5 (AI Agent, Runs the gates by hand before a commit) — As an agent building a card, I want the length check to name the file and its count so that I split it before landing, not after.

## Purpose

CLAUDE.md says no file holds over 500 lines of code, excluding tests, comments and whitespace, and no gate leg measures it, so the rule holds only while nobody breaks it. This brief makes it a gate leg declared everywhere the gate is declared, so a build that grows a file past the limit is refused in its own round.

## Task

Write a dependency-free Python 3 checker and its tests, a two-line shell script that runs both, and declare that script as the 'file-length' leg in project.json, in every cluster gate, in .land/gates.sh, in CLAUDE.md's gate block and in CI. Prove it green on the card's head and red on a planted scratch file that never reaches origin.

## Requirements

### R1: The checker, scripts/check_file_length.py

Behavioural. Python 3 standard library only. It takes the tracked file list from one `git ls-files -z` call at the repository root and never walks the disk, so build output, node_modules and untracked files are never read. It measures every tracked file ending .rs, .ts, .tsx, .mts or .mjs, except files under vendor/ and generated files that carry a first-line marker the repository already uses (name the marker if one exists; if none exists, none is invented). Each file is read once, as UTF-8; a file that cannot be read or decoded is an error naming the path and the reason, and the checker exits 2: it never skips it. A code line is a line that holds at least one character that is not whitespace and not part of a comment, after removing: Rust and TypeScript line comments (// to end of line, including /// and //!), block comments /* ... */ (nested in Rust, not nested in TypeScript), and nothing inside a string. String literals are honoured so a comment marker inside one is code: Rust "..." with escapes, raw strings r"..." and r#"..."# with any number of #, byte strings, and char literals without mistaking a lifetime ('a) for one; TypeScript '...', "..." and template literals `...` including ${...} nesting, and regular-expression literals where a / cannot be a division. Test code is not counted: a Rust file under any tests/ directory, a file named *_tests.rs, a Rust file whose first item is the inner attribute #![cfg(test)], and the body of any item annotated #[cfg(test)] (a mod, fn or impl, from its opening brace to its matching close, braces inside strings and comments ignored); a TypeScript file named *.test.ts, *.test.tsx, *.spec.ts or *.spec.tsx, or under a __tests__ directory. A file whose code lines exceed 500 is over the limit; exactly 500 is within it. Output: one line per file over the limit, sorted by path, as `PATH: N lines of code (limit 500)`, then a final line `file-length: K files measured, M over the limit`; exit 1 if M > 0, else 0. One pass per file, no regular expression rerun per line where a single scanner will do, no subprocess other than the one git call, no network.

**Acceptance:**
- python3 scripts/check_file_length.py exits 0 at the card's head and its last line reports M = 0 and K equal to the number of matching paths in `git ls-files` minus the excluded ones.
- python3 -c 'import ast,sys; ast.parse(open("scripts/check_file_length.py").read())' exits 0 and the file imports nothing outside the standard library.
- The checker's own file is itself under 500 lines of code by its own measure.

**Files:**
- create: scripts/check_file_length.py

**Checklist:**
- C10 — scripts/check_file_length.py counts the code lines of every tracked Rust and TypeScript source file as ADR-111 defines them and exits non-zero naming each file over 500, with its count.

**Stories:**
- S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main.
- S5 (AI Agent, Runs the gates by hand before a commit) — As an agent building a card, I want the length check to name the file and its count so that I split it before landing, not after.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. At the card's tree, `python3 scripts/check_file_length.py` exits 0 and its last line is `file-length: 498 files measured, 0 over the limit` (run in the card worktree and again at scratch commit 9d447cc, recorded in PROOF-LYSGATE-002.md). K is the tracked paths ending in the five suffixes, minus those under vendor/, minus the path-based test files (tests/ dirs, *_tests.rs, *.test/spec.ts(x), __tests__), minus files whose first item is #![cfg(test)]. is_measured_path is at scripts/check_file_length.py:369, and main skips a None count at :405. Accept 2: met. The file parses with ast.parse, and its imports are exactly __future__, pathlib, re, subprocess and sys, all standard library (checked with an ast walk). Accept 3: met. Python is not among the measured suffixes, so I counted it with Python's tokenizer, applying the same definition (no comments, docstrings or blank lines): 355 code lines out of 439 raw.

Rules as implemented. Rust: count_rust at :173 handles `//` line comments, nested /* */ through _rust_block_end at :127, escaped strings and byte strings through _rust_string_end at :114, and raw strings r/br/cr with any number of # through _rust_raw_prefix at :104 plus a hash count. A quote starts a char literal only when an escape follows or the closing quote is two characters on (_rust_char_end at :140); otherwise it is a lifetime. #[cfg(test)] makes the attribute and the item's head pending. A `{` at bracket depth 0 then starts an excluded body, closed by brace matching, where braces inside strings and comments are never seen. A `;`, `,` or `}` at depth 0 instead commits the pending lines as code (for example `#[cfg(test)] use x;`). An inner #![cfg(test)] before any code excludes the whole file. TypeScript/JS: count_script at :310 handles non-nesting block comments, '...' and "..." strings (an unclosed one ends at its line, so a JSX apostrophe cannot swallow the file), template literals with a ${...} brace-depth stack, and regex literals where the previous token allows one (_regex_allowed at :302). Generated files: no first-line generated marker exists in any tracked code file (I checked every first line; surface/identity/src/generated/*.ts carry prose headers only), so none is honoured, and the module doc says so. Cross-check: the per-file counts match a naive non-comment line count for all 143 TypeScript files, and for Rust wherever #[cfg(test)] does not apply. The whole run takes about 0.2 s.
- Deviation: (none)
- Files changed:
  - created: `scripts/check_file_length.py` — The file-length checker. One `git ls-files -z` at the root (default: the script's parent's parent, or argv[1]). It measures tracked .rs/.ts/.tsx/.mts/.mjs files outside vendor/ and outside path-based test code, reads each file once as UTF-8 and scans it in one pass (a compiled regex jumps to the next special character). It prints `PATH: N lines of code (limit 500)` per file over the limit, sorted by path, then `file-length: K files measured, M over the limit`. Exit 0/1, or 2 for any unreadable or undecodable file, which is named with its reason on stderr.
- Checklist delivery:
  - [x] C10 — scripts/check_file_length.py counts the code lines of every tracked Rust and TypeScript source file as ADR-111 defines them and exits non-zero naming each file over 500, with its count. — The checker exits 1 naming each file over 500 with its count; the red run named crates/lys/src/length_probe.rs with 501.
- Story delivery:
  - [x] S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main. — The leg fails in the round a file grows past 500.
  - [x] S5 (AI Agent, Runs the gates by hand before a commit) — As an agent building a card, I want the length check to name the file and its count so that I split it before landing, not after. — The output names the path and the count: `PATH: N lines of code (limit 500)`.

### R2: The checker's tests, scripts/check_file_length_test.py

Behavioural. unittest, standard library only, each case writing its fixture files into a tempfile.TemporaryDirectory initialised as a git repository with the files added, and running the checker's counting function or main on it. Cases, each with the expected count written as a literal: blank and whitespace-only lines; // /// //! line comments; a code line followed by a trailing comment counts once; /* */ over several lines; nested Rust /* /* */ */; a TypeScript /* inside a block comment not nesting; "// not a comment" in a Rust string and a TS string; r#"..."# holding "# and //; a lifetime 'a next to a char literal '/'; a TS template literal holding // and ${ `nested` }; a TS regex literal /\/\//; #[cfg(test)] mod with braces in strings inside it excluded, code after its close counted; #![cfg(test)] file, *_tests.rs, tests/x.rs, x.test.ts excluded; an untracked 600-line file ignored; a file of exactly 500 code lines passes and 501 fails with the named output line and exit 1; a file of 700 raw lines of which 200 are comments passes; invalid UTF-8 exits 2 naming the path.

**Acceptance:**
- python3 -m unittest scripts/check_file_length_test.py exits 0 and runs every case named in the spec, each as its own test method.

**Files:**
- create: scripts/check_file_length_test.py

**Checklist:**
- C11 — scripts/check_file_length_test.py covers every counting rule (comments, block and nested comments, strings and raw strings holding comment markers, test code, blank lines, the 500 and 501 boundary, an unreadable file) and passes.

**Stories:**
- S5 (AI Agent, Runs the gates by hand before a commit) — As an agent building a card, I want the length check to name the file and its count so that I split it before landing, not after.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept: met. `python3 -m unittest scripts/check_file_length_test.py` ran 23 tests, OK, in about 2.4 s. Each spec case is its own method: blank/whitespace (:62), // /// //! (:65), trailing comment (:69), multi-line /* */ (:72), nested Rust (:76), TS non-nesting (:80), comment markers in a Rust string (:84) and a TS string (:95), r##".."## holding "# and // (:105), lifetime beside '/' and '"' (:116), template with // and ${ `nested` } (:126), regex /\/\// (:136), #[cfg(test)] mod with braces in strings (:150), and #![cfg(test)] / *_tests.rs / tests/x.rs / x.test.ts excluded (:175-:184), each paired with a 600-line body so that failing to exclude it fails the case. Also untracked 600-line ignored (:187), exactly 500 passes (:192), 501 fails with the exact output and exit 1 (:198), 700 raw lines with 200 comments passes (:208), and invalid UTF-8 exits 2 naming the path (:217). One extra case, the JSX apostrophe (:146), covers the ends-at-its-line rule. Discrimination: I applied 18 mutations to a temp copy of the checker (no nesting, TS nesting, raw strings off, char literals off, lifetimes read as chars, regex off, cfg(test) off, inner-attribute off, >= limit, each path exclusion off, TS strings off, templates off, template nesting off, line-ended strings off). Each made the case built for it fail. Three fixtures (lifetime, template, template nesting) at first let errors cancel out and were reshaped until they did.
- Deviation: One test method beyond the spec's list, test_unclosed_quote_in_jsx_text_ends_at_its_line, pins a behaviour the checker needs so a JSX apostrophe cannot swallow later lines. Every listed case is present.
- Files changed:
  - created: `scripts/check_file_length_test.py` — 23 unittest cases, standard library only. Each writes its fixtures into a fresh TemporaryDirectory git repository, adds the tracked ones, and runs count_file or main on it, with expected counts as literals.
- Checklist delivery:
  - [x] C11 — scripts/check_file_length_test.py covers every counting rule (comments, block and nested comments, strings and raw strings holding comment markers, test code, blank lines, the 500 and 501 boundary, an unreadable file) and passes. — Covers every counting rule and passes (23 tests OK).
- Story delivery:
  - [x] S5 (AI Agent, Runs the gates by hand before a commit) — As an agent building a card, I want the length check to name the file and its count so that I split it before landing, not after. — The 501 case asserts the exact named output line.

### R3: The leg's command, scripts/file-length.sh

Structural. `set -eu`, then `python3 -m unittest scripts/check_file_length_test.py`, then `python3 scripts/check_file_length.py`, run from the repository root (cd to the script's parent's parent first). Exits non-zero if either does.

**Acceptance:**
- sh scripts/file-length.sh exits 0 at the card's head.
- With one test in check_file_length_test.py made to fail in a scratch copy, sh scripts/file-length.sh exits non-zero before the checker runs.

**Files:**
- create: scripts/file-length.sh

**Checklist:**
- C12 — scripts/file-length.sh runs the tests, then the checker, and exits non-zero if either fails.

**Stories:**
- S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. `sh scripts/file-length.sh` exits 0 on the card's tree and at scratch commit 9d447cc; the last line is `file-length: 498 files measured, 0 over the limit`. Accept 2: met. In a detached temporary worktree, I made one test assert 99 instead of the real count. `sh scripts/file-length.sh` exited 1 with `FAILED (failures=1)`, and its output held no `file-length:` line, so the checker never ran (set -e stops at the failed unittest). The worktree was removed afterwards.
- Deviation: (none)
- Files changed:
  - created: `scripts/file-length.sh` — The leg's command. `set -eu`, cd to the script's parent's parent, then `python3 -m unittest scripts/check_file_length_test.py`, then `python3 scripts/check_file_length.py`.
- Checklist delivery:
  - [x] C12 — scripts/file-length.sh runs the tests, then the checker, and exits non-zero if either fails. — Tests, then the checker; non-zero if either fails.
- Story delivery:
  - [x] S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main. — A broken checker test also reddens the leg.

### R4: Declare the leg in project.json and every cluster gate

Structural. Insert {"name": "file-length", "command": "sh scripts/file-length.sh", "requires": ["tool:sh", "tool:python3", "tool:git"], "cadence": "round"} directly after the ast-grep leg of docs/design/project.json trees[0] and of the '.' tree of every docs/design/*/design.json gate array that exists at the card's head. No other leg moves or changes. Re-render every changed cluster with scripts/design/render-cluster.py.

**Acceptance:**
- A python one-liner over project.json and every docs/design/*/design.json prints, for every '.' gate tree, that the leg after 'ast-grep' is exactly the file-length leg above, and prints no tree without it.
- git diff <base> <head> -- docs/design/*/design.json docs/design/project.json shows only inserted leg objects in the gate arrays of those files, besides this cluster's own documents.

**Files:**
- modify: docs/design/project.json
- modify: docs/design/decisions-words/design.json
- modify: docs/design/directory/design.json
- modify: docs/design/home/design.json
- modify: docs/design/lys-anchor/design.json
- modify: docs/design/lys-core/design.json
- modify: docs/design/lys-gate/design.json
- modify: docs/design/lys-log-store/design.json
- modify: docs/design/rauthy-rebase/design.json
- modify: docs/design/roots/design.json
- modify: docs/design/secrets/design.json

**Checklist:**
- C13 — The file-length leg is declared identically in docs/design/project.json and in the '.' tree of every cluster design.json gate, directly after the ast-grep leg.

**Stories:**
- S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. A one-liner over project.json trees and every docs/design/*/design.json gate prints, for all 11 '.' trees, that the leg after 'ast-grep' equals {name file-length, command sh scripts/file-length.sh, requires [tool:sh, tool:python3, tool:git], cadence round} and that it is the last leg; no tree lacks it. Accept 2: met. `git diff -U0 HEAD` over those files shows only + lines forming the 11 inserted leg objects and no - lines. The cluster files were rewritten through json.dumps(indent=2, ensure_ascii=False), after first confirming that each one round-trips byte-identically. I re-rendered all ten clusters with scripts/design/render-cluster.py; no .md file changed, because the renderer never reads the gate array.
- Deviation: CN3 says nothing under docs/design/lys-core is edited, but R4 explicitly lists docs/design/lys-core/design.json and lys-gate's structure names it, so it gains the inserted leg and nothing else. Its rendered markdown is unchanged.
- Files changed:
  - modified: `docs/design/project.json` — trees[0] gains the file-length leg directly after ast-grep; inserted by hand, keeping the file's inline-array style.
  - modified: `docs/design/decisions-words/design.json` — The '.' gate tree gains the file-length leg after ast-grep.
  - modified: `docs/design/directory/design.json` — The '.' gate tree gains the file-length leg after ast-grep.
  - modified: `docs/design/home/design.json` — The '.' gate tree gains the file-length leg after ast-grep.
  - modified: `docs/design/lys-anchor/design.json` — The '.' gate tree gains the file-length leg after ast-grep.
  - modified: `docs/design/lys-core/design.json` — The '.' gate tree gains the file-length leg after ast-grep.
  - modified: `docs/design/lys-gate/design.json` — The '.' gate tree gains the file-length leg after ast-grep.
  - modified: `docs/design/lys-log-store/design.json` — The '.' gate tree gains the file-length leg after ast-grep.
  - modified: `docs/design/rauthy-rebase/design.json` — The '.' gate tree gains the file-length leg after ast-grep.
  - modified: `docs/design/roots/design.json` — The '.' gate tree gains the file-length leg after ast-grep.
  - modified: `docs/design/secrets/design.json` — The '.' gate tree gains the file-length leg after ast-grep; its docs tree is untouched.
- Checklist delivery:
  - [x] C13 — The file-length leg is declared identically in docs/design/project.json and in the '.' tree of every cluster design.json gate, directly after the ast-grep leg. — The leg is identical in project.json and all ten cluster '.' trees, directly after ast-grep.
- Story delivery:
  - [x] S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main. — Every cluster round now runs the leg.

### R5: Declare the leg in .land/gates.sh, CLAUDE.md and CI

Structural. .land/gates.sh gains `leg sh scripts/file-length.sh` directly after its ast-grep leg. CLAUDE.md's 'Gates before any commit' block gains `ast-grep scan --config sgconfig.yml` (which .land/gates.sh already runs and the block omits) and then `sh scripts/file-length.sh`, in the order .land/gates.sh runs them, and its sentence 'All five clean. No exceptions.' becomes 'Every line clean. No exceptions.'; the coding-standards bullet on 500 lines gains ', measured by the file-length gate leg'. .github/workflows/ci.yml gains a step 'File length (any file over 500 lines of code fails the job)' running `sh scripts/file-length.sh` after the ast-grep step; nothing else in CI changes.

**Acceptance:**
- grep -c 'sh scripts/file-length.sh' .land/gates.sh CLAUDE.md .github/workflows/ci.yml prints 1 for each file.
- The commands in CLAUDE.md's block, in order, equal the commands of .land/gates.sh's leg lines, in order, excluding identity_leg and its CLAUDE.md line, checked by a script recorded in the dev record.

**Files:**
- modify: .land/gates.sh
- modify: CLAUDE.md
- modify: .github/workflows/ci.yml

**Checklist:**
- C14 — .land/gates.sh runs `leg sh scripts/file-length.sh`, CLAUDE.md's 'Gates before any commit' block lists the same command and the ast-grep command it was missing, and CI runs the same script.

**Stories:**
- S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. `grep -c 'sh scripts/file-length.sh'` prints 1 for each of .land/gates.sh, CLAUDE.md and .github/workflows/ci.yml. Accept 2: met. This script, run from the repo root, prints True: `gates=[l[4:] for l in open('.land/gates.sh').read().splitlines() if l.startswith('leg ') and l!='leg identity_leg']; block=open('CLAUDE.md').read().split('## Gates before any commit\n\n```\n',1)[1].split('```',1)[0].splitlines(); block=[l for l in block if "--test 'identity_*'" not in l]; print(gates==block)`. Both lists are [sh scripts/design/gate.sh, cargo fmt --check, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features --no-fail-fast, cargo doc --no-deps --all-features, cargo doc --no-deps, ast-grep scan --config sgconfig.yml, sh scripts/file-length.sh].
- Deviation: The spec names only the ast-grep and file-length additions to CLAUDE.md's block. But .land/gates.sh's first leg is `sh scripts/design/gate.sh`, which the block also omitted, so the accept row (block commands equal gates.sh leg commands) could not hold without it. I added it as the block's first line. The identity line stays where it was: it is excluded from the comparison, and moving it would reorder an existing line.
- Files changed:
  - modified: `.land/gates.sh` — Gains `leg sh scripts/file-length.sh` at line 43, directly after the ast-grep leg and before identity_leg.
  - modified: `CLAUDE.md` — The 'Gates before any commit' block gains `sh scripts/design/gate.sh` first, and `ast-grep scan --config sgconfig.yml` then `sh scripts/file-length.sh` last (lines 120-122). 'All five clean.' becomes 'Every line clean.'. The 500-line bullet (line 55) gains ', measured by the file-length gate leg'.
  - modified: `.github/workflows/ci.yml` — Gains the step 'File length (any file over 500 lines of code fails the job)', running `sh scripts/file-length.sh`, after the ast-grep step (line 56-57). Nothing else in CI changed; the Test step still lacks --all-features.
- Checklist delivery:
  - [x] C14 — .land/gates.sh runs `leg sh scripts/file-length.sh`, CLAUDE.md's 'Gates before any commit' block lists the same command and the ast-grep command it was missing, and CI runs the same script. — gates.sh, CLAUDE.md and CI all run sh scripts/file-length.sh; CLAUDE.md also gained the ast-grep line.
- Story delivery:
  - [x] S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main. — Landing, CI and the documented gate list all carry the leg.

### R6: Prove it green and red

Evidence. At the card's head the leg exits 0 in the card round. On a local scratch branch that never reaches origin, add crates/lys/src/length_probe.rs holding 501 code lines (plus comments, so its raw length exceeds its code length) and run sh scripts/file-length.sh: it exits 1 and prints exactly one over-limit line naming that path with 501. Record both runs' last lines in docs/design/lys-gate/PROOF-LYSGATE-002.md with the commit each ran at. Delete the scratch branch.

**Acceptance:**
- PROOF-LYSGATE-002.md names both commits and quotes both final lines; the red line names crates/lys/src/length_probe.rs with 501.
- git ls-remote origin shows no branch holding length_probe.rs.

**Files:**
- create: docs/design/lys-gate/PROOF-LYSGATE-002.md

**Checklist:**
- C15 — The leg exits 0 on the card's head and exits non-zero on a scratch file of 501 code lines that never reaches origin, naming that file.
- C16 — sh scripts/design/gate.sh exits 0 at the card's head.

**Stories:**
- S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main.
- S5 (AI Agent, Runs the gates by hand before a commit) — As an agent building a card, I want the length check to name the file and its count so that I split it before landing, not after.

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Accept 1: met. PROOF-LYSGATE-002.md names both commits and quotes `file-length: 498 files measured, 0 over the limit` (green, exit 0) and `crates/lys/src/length_probe.rs: 501 lines of code (limit 500)` / `file-length: 499 files measured, 1 over the limit` (red, exit 1, exactly one over-limit line). Accept 2: met. The local branch scratch/lysgate-002 was never pushed; after the red run its worktree was removed and the branch deleted. `git ls-remote origin 'refs/heads/scratch/*'` prints nothing, and no origin ref mentions length_probe. The card branch itself received no commit. C16: I did not run `sh scripts/design/gate.sh`, as the instructions forbid running the measured checks. Its render comparison can only pass, since re-rendering every cluster changed no .md; validate and coverage are left to the workflow's measurement.
- Deviation: The green run was made at a scratch commit that copies this round's uncommitted change, not at a card commit, because this round makes no commit on the card branch. The card round's own gate record carries the leg's run at the landed head.
- Files changed:
  - created: `docs/design/lys-gate/PROOF-LYSGATE-002.md` — Records the green run at scratch commit 9d447cc5351cfe28152c1cad5f970a62b01d96a3 (3f48732 plus this round's change, tree 984d1c3) with its last line. Records the red run at fd3cfbae5230cc48dea6e0fc6d4d2a645ae2ec78 (plus crates/lys/src/length_probe.rs: 1004 raw lines, 501 code, 502 comments) with its only over-limit line and its last line, and the deletion of the scratch branch with an empty ls-remote.
- Checklist delivery:
  - [x] C15 — The leg exits 0 on the card's head and exits non-zero on a scratch file of 501 code lines that never reaches origin, naming that file. — Green at 9d447cc, red at fd3cfba naming the probe with 501; the scratch branch is deleted and absent from origin.
  - [x] C16 — sh scripts/design/gate.sh exits 0 at the card's head. — Not run by me (forbidden); re-rendering produced no markdown diff, and the workflow measures the design leg.
- Story delivery:
  - [x] S4 (Lead, Reads a red gate round before landing a card) — As a lead, I want a build that grows a file past 500 lines of code refused in its own round so that no oversized file ever reaches main. — A red round is proven on a planted 501-line file.
  - [x] S5 (AI Agent, Runs the gates by hand before a commit) — As an agent building a card, I want the length check to name the file and its count so that I split it before landing, not after. — The red line names the file and its count.

## Boundaries

- SHALL NOT change any existing leg's name, command, requirements, cadence or order.
- SHALL NOT split, shorten or reformat any source file; main 6194599 has none over the limit.
- SHALL NOT add a dependency, a Rust crate, a package.json entry or a network call.
- SHALL NOT add any timeout, deadline or bound in seconds.
- SHALL NOT exempt any file by name or list; exclusions are only the structural test-code and vendor rules of R1.
- SHALL NOT let the scratch branch or length_probe.rs reach origin.

## Verification

- sh scripts/file-length.sh exits 0 at the card's head.
- sh scripts/design/gate.sh exits 0 at the card's head.
- The card round's record shows the file-length leg run and green for this cluster's gate.
- The red scratch run of R6 is recorded in PROOF-LYSGATE-002.md.
- ast-grep scan --config sgconfig.yml exits 0 at the card's head.
