---
type: brief
id: HOME-013
cluster: home
title: Move the home record's logic out of record/mod.rs into home.rs, session.rs and helpers.rs
---

# HOME-013: Move the home record's logic out of record/mod.rs into home.rs, session.rs and helpers.rs

> **Cluster:** home
> **Design anchor:**
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-014 — A lantern is a custom entry in its session, and its note grows only by epilogue entries — A lantern is a `lys.lantern` custom entry appended at its session's head, carrying in custom.data the entry id of its point (an existing entry of the same session that is not itself a lantern or an epilogue, the head or any entry the head has moved past), the note as written, who lit it and when. Its note grows only by `lys.lantern_epilogue` custom entries naming the lantern's entry id and carrying the further words, who added them and when; a lantern's story is its entry followed by its epilogues in order, and nothing is rewritten. Rejected: Pi's `label` entry on the target (it replaces or clears a label rather than growing one, and carries no author or time), a lantern store beside the session outside Pi's grammar (a lantern would stop travelling with its session), and editing the lantern's note in place (the record is append-only, P1).
> - ADR-017 — A fork is a child session cut from the parent's own lines at a lantern's point, with its ancestry on both sides — A fork resolves a lantern to the session it was lit in, read from the lys.lantern data's lit_in when the record carries it and otherwise by the older-record rule (one holder cuts, several refuse lantern_ambiguous until a session is named), and cuts that session's root-to-point chain at the last assistant message at or before the point, through the index. The child is a new session under the parent's cwd whose header's parentSession is the parent file's path relative to the home, holding each cut entry as the parent file's own line bytes, then one lys.forked_from custom entry as its head naming the parent session, the lantern, the point, the cut entry, whether the coordinate was carried and the carried entry; the parent gains one lys.fork custom entry at its head naming the child. Nothing else is copied and no block is written. Rejected: re-serialising the copied entries (the copy would stop hash-matching the parent's lines), a fork store beside the sessions outside Pi's grammar, cutting at a point no lantern names, and a header field beyond Pi's parentSession.
> - ADR-018 — A user-message point is carried as a seed prompt beside the rendered file, never copied into the child — When the point is a user message the cut stops at the assistant message before it and the message is carried, not copied: lys.forked_from records its id with coordinate_carried true and counts, by kind, the parts of it that are not text. The Claude Code render of such a child writes the message's text parts, in order, as a seed prompt beside the rendered file under an in-band marker line naming the parent session, the point and the lantern, and names it in the render report; the template's launch line, printed by render-launch only, passes that file as the resumed session's first prompt. A part that is not text never refuses a fork or a render and never enters the seed. Rejected: copying the user message into the child's chain, refusing a fork for a non-text part, and putting the seed's text in the report or the loss account.
> **Checklist:**
> - C45 — The shared helpers and constants safe_component, MAX_NAME_BYTES, PI_FORMAT_VERSION, now, fresh_id, json_len, write_durable and custom_type_of are defined in crates/lys-home/src/record/helpers.rs.
> - C46 — Session, its impl, take_lock, load_checked and to_line are defined in crates/lys-home/src/record/session.rs.
> - C47 — Home and its impl are defined in crates/lys-home/src/record/home.rs.
> - C48 — crates/lys-home/src/record/fork.rs imports write_durable and custom_type_of from crate::record::helpers and changes no other line.
> - C49 — crates/lys-home/src/record/mod.rs holds only module docs, pub mod and mod lines with their cfg(test) attributes, and pub use lines, and the item grep HOME-013 names prints nothing on it.
> - C50 — Every public path lys_home::record::{Home, Session, safe_component, now, fresh_id, json_len, MAX_NAME_BYTES, PI_FORMAT_VERSION} and lys_home::{Home, Session} resolves as before the move, and no crate outside lys-home changes.
> - C51 — Every test that passed before the move passes unchanged with an equal count, no test file changes beyond use lines, no non-test source file in crates/lys-home is over 500 lines of code, and the gate legs pass.
> **Stories:**
> - S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.
> - S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.

## Purpose

The repository's structure rule says a mod.rs carries only pub mod, pub use and module docs, with logic in named files and tests in sibling *_tests.rs files. crates/lys-home/src/record/mod.rs carries the record's logic: Home, Session, their impls, the shared helpers and two constants, 630 lines at 7b53625. The judged-rows card of the home line, whose brief is on brief/home/e35a7157-b4f6-447c-9988-4006d58ba102 at 8620ba6, records this as a finding on HOME-001 R1 and names this card as the act that answers it; that finding is not on main. This brief moves the code into named files and changes no behaviour and no public path, so the record module meets the rule and no caller notices.

## Task

Start from main as it stands when the build runs and move whatever crates/lys-home/src/record/mod.rs then holds; other home cards change files under record, so the item list below is mod.rs at 7b53625 and the build moves any item added since by the same rule. BASE names the main commit the build branch starts from (git merge-base HEAD origin/main). Order: helpers.rs first (R1), then session.rs (R2), which calls the helpers, then home.rs (R3), which calls Session, then fork.rs's import (R4), then mod.rs reduced to docs, mod lines and re-exports (R5), then the proof that nothing else changed (R6). Home goes to home.rs, Session and the helpers only Session calls (take_lock, load_checked, to_line) to session.rs, and the helpers and constants the record's files share (safe_component, MAX_NAME_BYTES, PI_FORMAT_VERSION, now, fresh_id, json_len, write_durable, custom_type_of) to helpers.rs. The three new modules are private and their public items are re-exported from mod.rs, so lys_home::record::{Home, Session, safe_component, now, fresh_id, json_len, MAX_NAME_BYTES, PI_FORMAT_VERSION} and lys_home::{Home, Session} resolve as before and no public path is added. Private fields and methods of Session and Home become pub(super), which reaches the record module and its descendants, exactly the reach private had in mod.rs; the impl Session blocks in beside.rs and fork.rs depend on it. Code moves byte for byte: a body's text does not change, and each new file imports what its bodies name. The 500-line measure is the repository's own: lines of code, without blank and comment lines, counted over non-test source; *_tests.rs files and files under crates/lys-home/tests are outside it, so record/call_tests.rs (519 lines of code at 7b53625) is not measured and is not split here. A test file changes only where the move requires a use path, and none is expected to. Out of scope: renaming or removing any public item, changing what any function does, splitting any test file, any other module's mod.rs (harness/claude_code/mod.rs included), and any crate outside lys-home.

## Requirements

### R1: Move the shared helpers and constants into record/helpers.rs

Create crates/lys-home/src/record/helpers.rs holding, moved from crates/lys-home/src/record/mod.rs as it stands at BASE, the constants MAX_NAME_BYTES and PI_FORMAT_VERSION, and the functions safe_component, now, fresh_id, json_len, write_durable and custom_type_of, each with its doc comment, attributes and body unchanged, and every other free function or constant mod.rs holds at BASE that is not Home's or Session's and is not used only by Session. MAX_NAME_BYTES, PI_FORMAT_VERSION, safe_component, now, fresh_id and json_len stay pub; write_durable and custom_type_of are declared pub(super), so they reach the record module and its descendants and nothing wider. The file opens with a //! module doc naming what it holds and imports what the moved bodies name (fresh_id's blocks::hex_of through a use of the record's blocks module), so no body's text changes. The file SHALL NOT hold Home, Session, take_lock, load_checked or to_line, SHALL NOT declare any item pub(crate) or pub(in ...), SHALL NOT rename any item, and SHALL NOT change what any function returns, writes or refuses.

**Acceptance:**
- `grep -nE '^(pub(\(super\))? )?(const|fn) (MAX_NAME_BYTES|PI_FORMAT_VERSION|safe_component|now|fresh_id|json_len|write_durable|custom_type_of)\b' crates/lys-home/src/record/helpers.rs` prints exactly 8 lines, one per name.
- `grep -nE '^pub\(super\) fn (write_durable|custom_type_of)\(' crates/lys-home/src/record/helpers.rs` prints exactly 2 lines.
- `grep -nE 'pub\(crate\)|pub\(in |struct (Home|Session)|fn (take_lock|load_checked|to_line)\b' crates/lys-home/src/record/helpers.rs` prints nothing.
- The first line of crates/lys-home/src/record/helpers.rs begins with `//!`.

**Files:**
- create: crates/lys-home/src/record/helpers.rs

**Checklist:**
- C45 — The shared helpers and constants safe_component, MAX_NAME_BYTES, PI_FORMAT_VERSION, now, fresh_id, json_len, write_durable and custom_type_of are defined in crates/lys-home/src/record/helpers.rs.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

### R2: Move Session and its own helpers into record/session.rs

Create crates/lys-home/src/record/session.rs holding, moved from mod.rs at BASE, the Session struct with its derive and field docs, its impl block whole, and the functions take_lock, load_checked and to_line, which only Session's impl calls; bodies, doc comments and attributes unchanged. Every field and every method of Session that is private at BASE is declared pub(super), so the impl Session blocks in record/beside.rs and record/fork.rs, and every other descendant of the record module, keep exactly the reach they have today; take_lock, load_checked and to_line stay private to the file. The file opens with a //! module doc naming what it holds and imports the helpers it calls from crate::record::helpers. It SHALL NOT make any field or method pub or pub(crate), SHALL NOT add, remove or rename a field or method, SHALL NOT change the lock, the durability order of line, index row and head, the reconcile path or any error variant, and SHALL NOT use an #[allow] to quiet a lint the move raises.

**Acceptance:**
- `grep -nE '^pub struct Session\b|^impl Session\b' crates/lys-home/src/record/session.rs` prints exactly 2 lines.
- `grep -nE '^fn (take_lock|load_checked|to_line)\b' crates/lys-home/src/record/session.rs` prints exactly 3 lines.
- `grep -cE '^[[:space:]]+pub\(super\) ' crates/lys-home/src/record/session.rs` prints the number of Session's fields and methods that carry no pub at BASE (11 at 7b53625: the 8 fields file, header, index, head, rebuilt, lock, stale and reconciliations, and the methods reconcile, fresh and append_line).
- `grep -nE 'pub\(crate\)|pub\(in |#\[allow|#!\[allow' crates/lys-home/src/record/session.rs` prints nothing.
- `git diff "$BASE" -- crates/lys-home/src/record/beside.rs` prints nothing.

**Files:**
- create: crates/lys-home/src/record/session.rs

**Checklist:**
- C46 — Session, its impl, take_lock, load_checked and to_line are defined in crates/lys-home/src/record/session.rs.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

### R3: Move Home into record/home.rs

Create crates/lys-home/src/record/home.rs holding, moved from mod.rs at BASE, the Home struct with its derive and doc and its impl block whole, bodies, doc comments and attributes unchanged. Every field and method of Home that is private at BASE is declared pub(super). The file opens with a //! module doc naming what it holds and imports Session, the helpers and the stores it names. It SHALL NOT hold Session or any helper, SHALL NOT add, remove or rename a field or method, and SHALL NOT change where a session, block or template file is placed or how a session id is checked.

**Acceptance:**
- `grep -nE '^pub struct Home\b|^impl Home\b' crates/lys-home/src/record/home.rs` prints exactly 2 lines.
- `grep -nE 'struct Session|impl Session|pub\(crate\)|pub\(in |#\[allow|#!\[allow' crates/lys-home/src/record/home.rs` prints nothing.

**Files:**
- create: crates/lys-home/src/record/home.rs

**Checklist:**
- C47 — Home and its impl are defined in crates/lys-home/src/record/home.rs.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

### R4: Point fork.rs's import of the private helpers at record::helpers

In crates/lys-home/src/record/fork.rs, import write_durable and custom_type_of from crate::record::helpers, and keep Home, Session and fresh_id imported from crate::record. No other line of fork.rs changes, and its impl Session block is unchanged. The change SHALL NOT re-export write_durable or custom_type_of from mod.rs and SHALL NOT widen either beyond pub(super).

**Acceptance:**
- `git diff -U0 "$BASE" -- crates/lys-home/src/record/fork.rs | grep -E '^[+-]' | grep -vE '^(\+\+\+|---) '` prints exactly the three lines `-use crate::record::{Home, Session, custom_type_of, fresh_id, write_durable};`, `+use crate::record::helpers::{custom_type_of, write_durable};` and `+use crate::record::{Home, Session, fresh_id};`, in any order.

**Files:**
- modify: crates/lys-home/src/record/fork.rs

**Checklist:**
- C48 — crates/lys-home/src/record/fork.rs imports write_durable and custom_type_of from crate::record::helpers and changes no other line.

**Stories:**
- S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.

### R5: Reduce record/mod.rs to module docs, mod lines and pub use lines

crates/lys-home/src/record/mod.rs keeps its //! module docs, every pub mod and mod line it holds at BASE with its cfg(test) attribute, and declares the three new modules as private `mod helpers;`, `mod home;` and `mod session;`. It re-exports with pub use exactly the public items it defined at BASE: Home from home, Session from session, and MAX_NAME_BYTES, PI_FORMAT_VERSION, safe_component, now, fresh_id and json_len from helpers, plus any other public item it defines at BASE from the file it moved to. Its five private use lines at BASE go, and its module doc's link to HomeError::SessionHeld keeps its text and names its target as crate::error::HomeError::SessionHeld, so it resolves without an import. mod.rs SHALL NOT hold any fn, struct, enum, union, impl, const, static, type or trait item, any private use line or any restricted re-export, SHALL NOT re-export any item that is not public at BASE, and SHALL NOT make helpers, home or session a pub mod, so no new public path is added. crates/lys-home/src/lib.rs does not change.

**Acceptance:**
- `grep -nE '^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?((const|async|unsafe|extern)[[:space:]]+)*(fn|struct|enum|union|impl|const|static|type|trait)\b|macro_rules!' crates/lys-home/src/record/mod.rs` prints nothing.
- `grep -nE '^[[:space:]]*use |pub\((crate|super|in )[^)]*\) use ' crates/lys-home/src/record/mod.rs` prints nothing.
- `grep -nxE 'mod (helpers|home|session);' crates/lys-home/src/record/mod.rs` prints exactly 3 lines.
- After `cargo doc --no-deps -p lys-home`, the 8 files target/doc/lys_home/record/struct.Home.html, struct.Session.html, fn.safe_component.html, fn.now.html, fn.fresh_id.html, fn.json_len.html, constant.MAX_NAME_BYTES.html and constant.PI_FORMAT_VERSION.html all exist, and none of the directories target/doc/lys_home/record/helpers, target/doc/lys_home/record/home and target/doc/lys_home/record/session exists.
- `git diff "$BASE" -- crates/lys-home/src/lib.rs` prints nothing.
- `cargo doc --no-deps` and `cargo doc --no-deps --all-features` each exit 0 with no warning line naming crates/lys-home/src/record.

**Files:**
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C49 — crates/lys-home/src/record/mod.rs holds only module docs, pub mod and mod lines with their cfg(test) attributes, and pub use lines, and the item grep HOME-013 names prints nothing on it.
- C50 — Every public path lys_home::record::{Home, Session, safe_component, now, fresh_id, json_len, MAX_NAME_BYTES, PI_FORMAT_VERSION} and lys_home::{Home, Session} resolves as before the move, and no crate outside lys-home changes.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.
- S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.

### R6: Prove the move changed no code, no test and no other crate

WHEN the moved items in helpers.rs, session.rs and home.rs are compared with mod.rs at BASE, ignoring blank lines, //! lines, use lines, mod lines, cfg(test) attributes and the text `pub(super) `, THE SYSTEM SHALL show the same lines. WHEN the workspace's tests run with all features, THE SYSTEM SHALL report the same passed, failed and ignored counts as at BASE, with none failed. THE SYSTEM SHALL NOT change any file outside crates/lys-home except the design documents that carry this brief and its roadmap row, SHALL NOT change any line of a test file other than a use line, SHALL NOT change any other module's mod.rs, and SHALL NOT leave any non-test source file in crates/lys-home over 500 lines of code, counted without blank lines and comment lines, with *_tests.rs files and files under crates/lys-home/tests left out of the count.

**Acceptance:**
- `bash -c 'strip() { sed -E "/^(pub )?use .*\{$/,/\};$/d" | grep -vE "^[[:space:]]*($|//!|(pub )?use |(pub(\(crate\))? )?mod |#\[cfg\(test\)\])" | sed -E "s/pub\(super\) //" | sort; }; diff <(git show "$BASE":crates/lys-home/src/record/mod.rs | strip) <(cat crates/lys-home/src/record/helpers.rs crates/lys-home/src/record/session.rs crates/lys-home/src/record/home.rs | strip)'` prints nothing.
- `cargo test --workspace --all-features 2>&1 | grep -E '^test result:' | awk '{p+=$4; f+=$6; i+=$8} END {print p, f, i}'` prints the same three numbers on the branch as on BASE, and the second number is 0.
- `git diff -U0 "$BASE" -- 'crates/lys-home/*_tests.rs' crates/lys-home/tests | grep -E '^[+-]' | grep -vE '^(\+\+\+|---) ' | grep -vE '^[+-][[:space:]]*(pub )?use '` prints nothing.
- `git diff --name-only "$BASE" -- . ':!crates/lys-home' ':!docs/design'` prints nothing.
- `git diff --name-only "$BASE" -- 'crates/lys-home/*mod.rs'` prints exactly crates/lys-home/src/record/mod.rs.
- `find crates/lys-home -name '*.rs' ! -name '*_tests.rs' ! -path 'crates/lys-home/tests/*' -exec sh -c 'n=$(grep -cvE "^[[:space:]]*(//|$)" "$1"); [ "$n" -gt 500 ] && echo "$1 $n"' _ {} \;` prints nothing.

**Checklist:**
- C51 — Every test that passed before the move passes unchanged with an equal count, no test file changes beyond use lines, no non-test source file in crates/lys-home is over 500 lines of code, and the gate legs pass.

**Stories:**
- S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.

## Boundaries

- SHALL NOT rename, remove or move to a new path any public item of lys-home, and SHALL NOT add a public path.
- SHALL NOT change the text of any moved function body, and SHALL NOT change any on-disk format, the session lock, the durability order of line, index row and head, or any error variant.
- SHALL NOT change any file of a crate other than lys-home.
- SHALL NOT change any test file except a use line the move requires, and SHALL NOT split or add a test file.
- SHALL NOT change any mod.rs other than crates/lys-home/src/record/mod.rs.
- SHALL NOT widen any private field, method or helper beyond pub(super).
- SHALL NOT silence a lint with #[allow], #[ignore], a _-prefixed name or #[cfg(any())].

## Verification

- Set and export BASE as the main commit the build branch starts from: `export BASE=$(git merge-base HEAD origin/main)`.
- Run `grep -nE '^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?((const|async|unsafe|extern)[[:space:]]+)*(fn|struct|enum|union|impl|const|static|type|trait)\b|macro_rules!' crates/lys-home/src/record/mod.rs` and confirm it prints nothing.
- Run `bash -c 'strip() { sed -E "/^(pub )?use .*\{$/,/\};$/d" | grep -vE "^[[:space:]]*($|//!|(pub )?use |(pub(\(crate\))? )?mod |#\[cfg\(test\)\])" | sed -E "s/pub\(super\) //" | sort; }; diff <(git show "$BASE":crates/lys-home/src/record/mod.rs | strip) <(cat crates/lys-home/src/record/helpers.rs crates/lys-home/src/record/session.rs crates/lys-home/src/record/home.rs | strip)'` and confirm it prints nothing.
- Run `cargo test --workspace --all-features 2>&1 | grep -E '^test result:' | awk '{p+=$4; f+=$6; i+=$8} END {print p, f, i}'` on BASE and on the branch and confirm the two outputs are equal with 0 failed.
- Run `find crates/lys-home -name '*.rs' ! -name '*_tests.rs' ! -path 'crates/lys-home/tests/*' -exec sh -c 'n=$(grep -cvE "^[[:space:]]*(//|$)" "$1"); [ "$n" -gt 500 ] && echo "$1 $n"' _ {} \;` and confirm it prints nothing.
- Run `cargo fmt --all` and confirm `git status --porcelain` shows no file it changed.
- Run `cargo clippy --all-targets --all-features -- -D warnings` and `cargo clippy --all-targets -- -D warnings` and confirm each exits 0.
- Run `cargo test --workspace --all-features` and confirm it exits 0.
- Run `cargo doc --no-deps --all-features` and `cargo doc --no-deps` and confirm each exits 0.
- Run `sh scripts/design/gate.sh` and confirm it exits 0.
