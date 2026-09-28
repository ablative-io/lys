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
