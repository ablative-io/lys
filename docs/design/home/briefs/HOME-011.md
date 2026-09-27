---
type: brief
id: HOME-011
cluster: home
title: Record the given documents in the request's order, re-measured, with the old order named as superseded
---

# HOME-011: Record the given documents in the request's order, re-measured, with the old order named as superseded

> **Cluster:** home
> **Depends on:** HOME-003
> **Blocked by:** measure.py, the fixture PROOF-GIVEN was measured with (SHA-256 db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d), is not on main: it stands in the repository's origin on the branch fixture/proof-given-measure at commit 2f1baac9a0e11e643711ec8a49ee0f1357406e91, at docs/design/home/proof-given/measure.py. R1 takes it from that commit, checks its hash and commits it to the card branch by that path before anything else runs.
> **Design anchor:**
> - ADR-013 — The context record is a lys.given custom entry of document hashes, never copies — The context record is one lys.given custom entry, appended after the render event, whose data is the harness name, the Claude Code version the load order was measured on, the kinds as two lists, resolved (claude_md_chain, user_claude_md, memory_index, appended_instructions, mcp_config, environment_names) and unlisted (claude_md_imports and claude_rules, which this entry does not list and a later entry at the first request records), the config directory as its path and its source (template or home), the documents in the measured order each as kind, path, byte length and SHA-256, and the names of the environment variables the template set. It is not a copy of each document into the block store, and not a content-bearing record, because the entry must hold no content under home P7 and CN3. It is unsigned and unencrypted now, and because it names hashes only, signing and encryption at rest can be added later without changing what is recorded.
> - ADR-031 — The context record's cross-kind order is the order the harness's request gives — The context record's documents follow the order the harness's request gives wherever it and the read order differ: appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index. The MCP configuration sits straight after the appended instructions, both harness-side inputs ahead of the first message, until a card measures a real server's tools position. The within-directory order (CLAUDE.md, .claude/CLAUDE.md, CLAUDE.local.md) is unchanged. Entries already written under the old order stand as written and no data member is added to mark the order; the documents name the old order as superseded by the commit that lands HOME-011 and by that commit's date. Rejected: keeping the read order, since the request is what reached the model; rewriting or migrating entries already written, since the record is append-only; adding a data member or relying on harness_version to mark the order, since the old order was written under 2.1.283, the new one is written under the version the re-measurement ran on, which may be the same, and the harness version does not tell the two orders apart.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> **Checklist:**
> - C83 — resolve_given lists a session's documents in the request's order, appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index, with one directory's CLAUDE.md, .claude/CLAUDE.md and CLAUDE.local.md in that order and D/.claude/CLAUDE.md listed once, as user_claude_md, when D/.claude is the config directory.
> - C84 — The given_tests.rs unit test and the fixture test in tests/given_record.rs, whose config directory now holds a user CLAUDE.md, each assert that appended_instructions and mcp_config precede user_claude_md.
> - C85 — RECORD.md states one rule, that the request's order wins wherever it and the read order differ with the MCP configuration straight after the appended instructions, and names the old order as superseded by the commit that lands HOME-011, told from an entry by its recorded time and never by its harness_version.
> - C86 — PROOF-GIVEN.md keeps its earlier text unchanged and appends a re-measurement of the request's order by the committed, unchanged measure.py on the installed Claude Code, with the version `claude --version` printed, naming the old order as superseded.
> - C87 — S12 reads `in the order the request gives them`, HOME-003 carries an amendment naming its R2 cross-kind order as superseded, and every rendered markdown file of the home cluster is what its JSON renders to.
> **Stories:**
> - S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.
> - S13 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

## Purpose

The lys.given entry lists a session's documents in the order Claude Code reads the files, with the user CLAUDE.md first, while PROOF-GIVEN measured that the request the harness sends puts the appended instructions, in the system array, ahead of the first user message that carries every CLAUDE.md-family file and the memory index. The record should follow what reached the model. This brief makes the request's order the one rule (ADR-031), stated in RECORD.md, re-measures it with PROOF-GIVEN's own fixture on the installed Claude Code, moves the resolver and its two test files to it, and names the old order as superseded without touching any entry already written.

## Task

The cross-kind order is fixed in one place, resolve_given in crates/lys-home/src/harness/claude_code/given.rs, which pushes user_claude_md before the appended_instructions and mcp_config loop; move that push after the loop and correct the module docs. The new order is appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index; the within-directory order (CLAUDE.md, .claude/CLAUDE.md, CLAUDE.local.md) is out of scope and unchanged, as are the lys.given data shape (ADR-013), the template_render event, the manifest, the launch line and how the config directory is resolved. The MCP configuration's place straight after the appended instructions is provisional, from the read order, until a later card measures a real server's tools position; this card adds no MCP server and runs measure.py exactly as it is. Order: R1 commits measure.py and re-measures first, since a different order stops the card and a different installed version moves MEASURED_VERSION; then the resolver (R2), its unit tests (R3), the fixture test (R4), RECORD.md (R5) and the rendered design documents (R6). Entries already written under the old order stand; no data member is added, and the documents say the harness version does not tell the two orders apart and that an entry's recorded time against the date of the commit that lands HOME-011 does; that commit is named, not quoted by id, and no later commit writes its id or date in. The re-measurement runs on a machine with Claude Code installed; every compile, test battery and gate runs on the build machine. S12 and S13 are shared with HOME-003, which delivered them in the old order; this brief delivers them in the request's order, and C21 and C23 stay HOME-003's. The earlier real render and the Pi-reader fixture session recorded in PROOF-GIVEN.md were written under the old order and are not re-run. HOME-003's R2 is amended, not rewritten.

## Requirements

### R1: Commit PROOF-GIVEN's measure.py unchanged and append the re-measurement of the request's order to PROOF-GIVEN.md

Before any other requirement runs, THE SYSTEM SHALL take PROOF-GIVEN's measure fixture from commit 2f1baac9a0e11e643711ec8a49ee0f1357406e91 of the origin branch fixture/proof-given-measure, at docs/design/home/proof-given/measure.py, by `git show 2f1baac9a0e11e643711ec8a49ee0f1357406e91:docs/design/home/proof-given/measure.py` or by a fetch of that branch, check that its SHA-256 is db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d, and commit it, byte for byte, to the card branch at docs/design/home/proof-given/measure.py. IF the SHA-256 of the file taken is not db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d, THEN THE SYSTEM SHALL NOT commit it and SHALL NOT go on to any other requirement. THE SYSTEM SHALL NOT take the file from any path outside the repository's history, and SHALL NOT take it from the branch's head in place of that commit when the two differ. WHEN the order is re-measured, THE SYSTEM SHALL copy that file unchanged into a fresh scratch directory outside the repository, run it there once with python3, which makes its own three runs, on a machine where Claude Code is installed and a local port is free, and take `claude --version` on the same machine in the same session. Append to docs/design/home/PROOF-GIVEN.md, below its last existing line, one new section, headed as the re-measurement of the request's order and carrying the day it was measured, that records: the command run; the SHA-256 of the copy that was run; the string `claude --version` printed; for each run the request count, the number of system blocks and of messages, where the appended instructions sit (the system array) and the order of the CLAUDE.md-family files and the memory index inside the first user message; that the MCP server contributed no tool, so the MCP configuration's place is taken from the read order, straight after the appended instructions; the order the entry now records, appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index; and the old order, user_claude_md, appended_instructions, mcp_config, claude_md_chain, memory_index, named as superseded by the commit that lands HOME-011 and by that commit's date, with entries written before that commit standing as written. The section SHALL name 2.1.283 as the version of the earlier measurement and the version `claude --version` printed as the version of this one. IF the request places the kinds in any other order than the one above, THEN THE SYSTEM SHALL record what was measured in the section and SHALL NOT land the card, which goes back to its lead. IF the version printed is not 2.1.283 and the order is the one above, THEN the card goes on, and R2 moves MEASURED_VERSION to the version printed. THE SYSTEM SHALL NOT edit, reorder or remove any line PROOF-GIVEN.md held before, SHALL NOT change measure.py by a byte, SHALL NOT add an MCP server to the measurement, and SHALL NOT put any line of a fixture document, a request body or a marker string in the proof: paths, counts, orders, versions and hashes only.

**Acceptance:**
- `git show 2f1baac9a0e11e643711ec8a49ee0f1357406e91:docs/design/home/proof-given/measure.py | shasum -a 256` prints db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d.
- `git show 2f1baac9a0e11e643711ec8a49ee0f1357406e91:docs/design/home/proof-given/measure.py | cmp - docs/design/home/proof-given/measure.py` exits 0.
- The first commit the card branch adds on top of its base (git merge-base of the card branch and main when the build starts) changes docs/design/home/proof-given/measure.py and no other file.
- `shasum -a 256 docs/design/home/proof-given/measure.py` prints db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d.
- `git diff 7b53625 -- docs/design/home/PROOF-GIVEN.md` shows no removed line, and every added line comes after the file's line 209.
- The new section records a SHA-256 for the copy that was run equal to db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d.
- The new section records the string `claude --version` printed on the measuring run, names that string's version as this measurement's version, and names 2.1.283 as the earlier measurement's version.
- The new section records three runs, and for each the appended instructions in the system array and the user CLAUDE.md first among the CLAUDE.md-family files in the first user message.
- The new section lists the entry's order as appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index, in that order.
- The new section contains the word `superseded` beside the old order user_claude_md, appended_instructions, mcp_config, claude_md_chain, memory_index, and names HOME-011.
- `grep -cE 'MARK[A-Z]+ZQ|markmcpserverzq' docs/design/home/PROOF-GIVEN.md` prints 0.

**Files:**
- create: docs/design/home/proof-given/measure.py
- modify: docs/design/home/PROOF-GIVEN.md

**Checklist:**
- C86 — PROOF-GIVEN.md keeps its earlier text unchanged and appends a re-measurement of the request's order by the committed, unchanged measure.py on the installed Claude Code, with the version `claude --version` printed, naming the old order as superseded.

**Stories:**
- S13 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: `git show 2f1baac9…:docs/design/home/proof-given/measure.py | shasum -a 256` printed db17cccb… (met). The object only became present after `git fetch origin fixture/proof-given-measure`. The branch head equals that commit, per ls-remote.
Row 2: cmp against the committed path reported the files equal (met).
Row 3: NOT met at dev stage (I make no commit). Moreover the card branch already carries 8c6f5bb, the brief commit, on top of the merge-base 7b53625 with origin/main. The landing step will have to arrange this row (see deviation).
Row 4: `shasum -a 256 docs/design/home/proof-given/measure.py` prints db17cccb… (met).
Row 5: `git diff 7b53625 -- PROOF-GIVEN.md` has 0 removed lines, and every added line is at line 210 or later (the new section header is at line 211) (met).
Row 6: the section's table records the SHA-256 of the copy run as db17cccb… (met).
Row 7: the table records `2.1.283 (Claude Code)` as printed, names 2.1.283 as this measurement's version, and names 2.1.283 as the earlier measurement's version (met).
Row 8: the per-run table has three rows. Each shows appended instructions in 'the `system` array, block 3 (`system[2]`)' and `user_claude_md` first in the first user message (met).
Row 9: the section lists `appended_instructions`, `mcp_config`, `user_claude_md`, `claude_md_chain`, `memory_index` (met).
Row 10: the section's last paragraph gives the old order beside 'is superseded by the commit that lands HOME-011' (met).
Row 11: `grep -cE 'MARK[A-Z]+ZQ|markmcpserverzq'` prints 0 (met).
Measurement details: at 13:32–13:33 AEST, 27 Sep 2026, I copied the file to /tmp/home011-measure.* and confirmed the copy's hash. I ran `python3 measure.py` once there, and it made its three runs. Each run: exit 0, one request to /v1/messages?beta=true, 3 system blocks, 2 messages, 0 stderr bytes. The order was identical in all three runs and matched the brief's expected order. The scratch directory was deleted afterwards.
- Deviation: Acceptance row 3 (the first commit on the branch touches only measure.py) cannot be met by this stage, because the instructions forbid committing. The branch also already has 8c6f5bb (the brief commit, 7 design files) on top of the merge-base 7b53625, so ordering measure.py first needs the landing step to reorder commits or read the row against a different base. The section also notes something the earlier run did not have: stderr was 0 bytes in every run, not one line of print-mode warning, and it records that as measured.
- Files changed:
  - created: `docs/design/home/proof-given/measure.py` — PROOF-GIVEN's measure fixture. It is byte for byte the blob at 2f1baac9a0e11e643711ec8a49ee0f1357406e91:docs/design/home/proof-given/measure.py, taken with git show after fetching origin fixture/proof-given-measure. SHA-256 db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d, mode 100644.
  - modified: `docs/design/home/PROOF-GIVEN.md` — Adds one section at line 211, after the old last line 209: 'The re-measurement of the request's order (HOME-011 R1), 27 September 2026'. It records the command, the copy's SHA-256, the version printed, a table for the three runs, the request's order, that the MCP server added no tool, the order the entry now records, and the old order named as superseded by the commit that lands HOME-011. No earlier line was changed.
- Checklist delivery:
  - [x] C86 — PROOF-GIVEN.md keeps its earlier text unchanged and appends a re-measurement of the request's order by the committed, unchanged measure.py on the installed Claude Code, with the version `claude --version` printed, naming the old order as superseded. — Earlier text unchanged; the re-measurement by the unchanged committed measure.py on 2.1.283 is appended, and the old order is named superseded. The commit-ordering row is left to the landing step.
- Story delivery:
  - [x] S13 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed. — The order was re-measured on the named version 2.1.283 and written in PROOF-GIVEN.md.

### R2: List the given documents in the request's order

WHEN resolve_given resolves the documents for a working directory, a config directory and an out directory, THE SYSTEM SHALL return them in this order: the appended instructions file the render wrote (appended_instructions); the MCP configuration the render wrote (mcp_config); the user CLAUDE.md at <config>/CLAUDE.md (user_claude_md); for each directory from the outermost ancestor of the working directory down to it, its CLAUDE.md, .claude/CLAUDE.md and CLAUDE.local.md in that order (claude_md_chain); then the memory index (memory_index). WHILE the config directory is D/.claude for a directory D on the chain, THE SYSTEM SHALL list D/.claude/CLAUDE.md exactly once, as user_claude_md in the user file's place, and SHALL NOT list it as claude_md_chain. Rewrite the module docs of given.rs to state that this is the order the harness's request gives, that the request's order wins wherever it and the order the harness reads the files in differ, that the MCP configuration's place straight after the appended instructions comes from the read order until a real server's tools position is measured, that MEASURED_VERSION is the version the order was re-measured on, and to word the D/.claude/CLAUDE.md rule as listed once, as user_claude_md. Set MEASURED_VERSION to the version R1's section records `claude --version` printing, and make the harness_version literal asserted in crates/lys-home/src/record/given_tests.rs that same version. In crates/lys-home/README.md, set the version the `lys.given` paragraph names in `the documents Claude Code <version> loads` to the string MEASURED_VERSION holds, in the same change that sets MEASURED_VERSION, so the README never names a version the code no longer measures. THE SYSTEM SHALL NOT change any other line of the README, and SHALL NOT change the within-directory order, CHAIN_FILES, MEASURED_VERSION to any version other than the one R1 measured, how the config directory is resolved, how a document is read, hashed, named or omitted, or any error; SHALL NOT add a data member to lys.given; and SHALL NOT rewrite, migrate or re-read any lys.given entry already written.

**Acceptance:**
- A resolution for a fixture with an out directory holding instructions.md and mcp.json, a config directory c holding CLAUDE.md and the memory index, and a working directory w holding CLAUDE.md returns 5 documents whose kinds in order are appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index.
- In that resolution documents[0].path is `instructions.md`, documents[1].path is `mcp.json` and documents[2].path is c/CLAUDE.md.
- With HOME h, no template CLAUDE_CONFIG_DIR, h/.claude/CLAUDE.md and h/w/CLAUDE.md present, working directory h/w and an empty out directory, the resolution returns 2 documents: user_claude_md at h/.claude/CLAUDE.md, then claude_md_chain at h/w/CLAUDE.md.
- `grep -n 'once, first' crates/lys-home/src/harness/claude_code/given.rs` prints nothing.
- The module docs of given.rs list the five kinds in the order appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index and contain the words `read order`.
- `git diff 7b53625 -- crates/lys-home/src/record/given.rs crates/lys-home/src/record/entries.rs crates/lys-home/src/harness/claude_code/launch.rs` prints nothing.
- The line defining CHAIN_FILES in given.rs is unchanged from 7b53625.
- The string MEASURED_VERSION holds in given.rs is the version R1's section records `claude --version` printing.
- The harness_version literal asserted in crates/lys-home/src/record/given_tests.rs equals the string MEASURED_VERSION holds, and `cargo test -p lys-home --lib record::given_tests` reports 7 tests run, all passed.
- `grep -n 'the documents Claude Code' crates/lys-home/README.md` prints one line, and the version it names is the string MEASURED_VERSION holds in given.rs.
- When MEASURED_VERSION holds 2.1.283, `grep -c '2\.1\.283' crates/lys-home/README.md` prints 1; when it holds any other version, the same command prints 0.
- `git diff 7b53625 -- crates/lys-home/README.md` changes no line other than the one naming the version.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/given.rs
- modify: crates/lys-home/src/record/given_tests.rs
- modify: crates/lys-home/README.md

**Checklist:**
- C83 — resolve_given lists a session's documents in the request's order, appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index, with one directory's CLAUDE.md, .claude/CLAUDE.md and CLAUDE.local.md in that order and D/.claude/CLAUDE.md listed once, as user_claude_md, when D/.claude is the config directory.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (5 documents, kinds appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index): given.rs lines 217-230 push instructions/mcp before user_claude_md. This is exercised by given_tests.rs a_config_claude_md_is_listed_after_the_appended_instructions_and_mcp_config at line 150 (met by code; not run by me).
Row 2 (paths in that resolution): the same test asserts documents[2].path == c/CLAUDE.md. Paths 0 and 1 are the relative written names, as before (met).
Row 3 (HOME h fallback returns 2 documents, user then chain): home_dot_claude_claude_md_is_listed_once_as_the_user_file_when_home_is_the_config, whose body is unchanged; with an empty out directory the new order gives the same two documents (met).
Row 4: `grep -n 'once, first' given.rs` prints nothing (met).
Row 5: module docs lines 9-17 list the five kinds in the new order, and lines 24 and 27 contain 'read order' (met).
Row 6: `git diff 7b53625` on record/given.rs, record/entries.rs and launch.rs prints nothing (met).
Row 7: the CHAIN_FILES line is unchanged (met).
Row 8: MEASURED_VERSION stays "2.1.283", the version R1 recorded (met).
Row 9: record/given_tests.rs already asserts harness_version "2.1.283" at line 113, so no change is needed. Its 7 tests are untouched (met by content; not run).
Row 10: README line 72 names 'the documents Claude Code 2.1.283 loads', equal to MEASURED_VERSION (met).
Row 11: `grep -c '2\.1\.283' README.md` counts 1 (met; line 72 is the only occurrence).
Row 12: README is unchanged (met).
- Deviation: crates/lys-home/src/record/given_tests.rs and crates/lys-home/README.md are listed for modification but were not changed. R1 measured 2.1.283, which both already name, so the brief's rule (set them to the measured version) needs no edit.
- Files changed:
  - modified: `crates/lys-home/src/harness/claude_code/given.rs` — resolve_given now pushes appended_instructions and mcp_config first, then user_claude_md, then the chain (skipping a path equal to the user file), then the memory index. The module docs state the request's order, say it wins wherever it and the read order differ, give the MCP configuration's provisional place from the read order, name MEASURED_VERSION as the re-measured version, and word the D/.claude/CLAUDE.md rule as 'listed once, as user_claude_md'.
- Checklist delivery:
  - [x] C83 — resolve_given lists a session's documents in the request's order, appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index, with one directory's CLAUDE.md, .claude/CLAUDE.md and CLAUDE.local.md in that order and D/.claude/CLAUDE.md listed once, as user_claude_md, when D/.claude is the config directory. — The request's order is in resolve_given. The within-directory order and D/.claude/CLAUDE.md-once are unchanged.
- Story delivery:
  - [x] S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents. — The record lists documents in the order the request gives them.

### R3: Assert the request's order in the given_tests.rs unit tests

In given_tests.rs, replace a_config_claude_md_is_listed_first_as_user_claude_md with a_config_claude_md_is_listed_after_the_appended_instructions_and_mcp_config, which, on the module's fixture with a CLAUDE.md written in the config directory, asserts the documents' length, then that appended_instructions and mcp_config come before user_claude_md and user_claude_md before every claude_md_chain document. Rename home_dot_claude_claude_md_is_listed_once_first_as_the_user_file_when_home_is_the_config to home_dot_claude_claude_md_is_listed_once_as_the_user_file_when_home_is_the_config, keeping its body. THE SYSTEM SHALL NOT change any other test in given_tests.rs, SHALL NOT put a fixture document's line in a test name, and SHALL NOT remove an assertion that counts the documents before indexing them.

**Acceptance:**
- a_config_claude_md_is_listed_after_the_appended_instructions_and_mcp_config asserts documents.len() == 5 and the kinds appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index in that order, and passes.
- home_dot_claude_claude_md_is_listed_once_as_the_user_file_when_home_is_the_config passes, and `grep -c 'listed_once_first' crates/lys-home/src/harness/claude_code/given_tests.rs` prints 0.
- one_directory_lists_claude_md_then_dot_claude_then_local_whatever_the_creation_order passes, and its body is unchanged from 7b53625.
- `cargo test -p lys-home --lib harness::claude_code::given_tests` reports 15 tests run, all passed.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/given_tests.rs

**Checklist:**
- C84 — The given_tests.rs unit test and the fixture test in tests/given_record.rs, whose config directory now holds a user CLAUDE.md, each assert that appended_instructions and mcp_config precede user_claude_md.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: the test at given_tests.rs:150 asserts documents.len() == 5 and then the kinds array [AppendedInstructions, McpConfig, UserClaudeMd, ClaudeMdChain, MemoryIndex] (met by code; not run).
Row 2: the renamed test is at line 323, and `grep -c listed_once_first` prints 0 (met).
Row 3: the one_directory_lists_… body is unchanged (met).
Row 4: the file still has 15 #[test] functions (counted by grep) (met by count; not run).
- Deviation: (none)
- Files changed:
  - modified: `crates/lys-home/src/harness/claude_code/given_tests.rs` — a_config_claude_md_is_listed_first_as_user_claude_md is replaced by a_config_claude_md_is_listed_after_the_appended_instructions_and_mcp_config, which asserts len 5, the five kinds in order and documents[2].path == c/CLAUDE.md. The home_dot_claude test is renamed to drop 'first' and keeps its body.
- Checklist delivery:
  - [x] C84 — The given_tests.rs unit test and the fixture test in tests/given_record.rs, whose config directory now holds a user CLAUDE.md, each assert that appended_instructions and mcp_config precede user_claude_md. — The unit test asserts appended_instructions and mcp_config precede user_claude_md.
- Story delivery:
  - [x] S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents. — The unit test gates the request's order.

### R4: Give the fixture test a user CLAUDE.md in its config directory and assert the request's order

In tests/given_record.rs, Fixture::new SHALL write a user CLAUDE.md into the config directory c, holding one fixed fixture line of its own. The first-render test, renamed the_first_render_records_five_documents_in_the_request_s_order_under_the_render_event, SHALL assert given_documents 5, documents.len() 5, the kinds appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index in that order, and each document's length and SHA-256 equal to the file on disk it names (out/instructions.md, out/mcp.json, c/CLAUDE.md, w/CLAUDE.md, the memory index), with c/CLAUDE.md at index 2, and harness_version equal, as a literal, to the string MEASURED_VERSION holds; KINDS SHALL hold those five names. The two-renders test and the given listing SHALL assert 5 documents per record, and the one-byte change to w/CLAUDE.md SHALL still change exactly the claude_md_chain hash, asserted at that document's index, 3. The fallback test SHALL assert the kinds appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index with HOME's .claude/CLAUDE.md at index 2 and the memory index at index 4. THE SYSTEM SHALL NOT keep any assertion that user_claude_md occurs 0 times, SHALL NOT put the fixture line in a test name, and SHALL NOT change what the rendering process's environment holds for any test.

**Acceptance:**
- `grep -c 'user_claude_md").count(), 0' crates/lys-home/tests/given_record.rs` prints 0.
- the_first_render_records_five_documents_in_the_request_s_order_under_the_render_event asserts report["given_documents"] == 5 and documents.len() == 5 and passes.
- The same test asserts documents[2].path == c/CLAUDE.md and that its length and SHA-256 equal those of the file written there.
- The same test's harness_version literal equals the string MEASURED_VERSION holds in given.rs.
- the_rendering_process_s_config_dir_is_never_read_and_home_dot_claude_is_the_fallback asserts the kinds appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index and documents[2].path == h/.claude/CLAUDE.md, and passes.
- two_renders_give_equal_lists_and_one_changed_byte_changes_exactly_one_hash asserts 5 documents in each record, differing == ["claude_md_chain"] and after.documents[3].sha256 equal to the changed file's SHA-256, and passes.
- Moving the user_claude_md document back ahead of the appended instructions in resolve_given makes exactly three tests fail: a_config_claude_md_is_listed_after_the_appended_instructions_and_mcp_config, the_first_render_records_five_documents_in_the_request_s_order_under_the_render_event and the_rendering_process_s_config_dir_is_never_read_and_home_dot_claude_is_the_fallback, recorded as a drift injection in the dev record with the failing test names and the count of passing tests.

**Files:**
- modify: crates/lys-home/tests/given_record.rs

**Checklist:**
- C84 — The given_tests.rs unit test and the fixture test in tests/given_record.rs, whose config directory now holds a user CLAUDE.md, each assert that appended_instructions and mcp_config precede user_claude_md.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: grep for 'user_claude_md").count(), 0' prints 0 (met).
Row 2: the renamed test asserts report["given_documents"] == 5 and documents.len() == 5 (met by code; not run).
Row 3: it asserts documents[2].path == c/CLAUDE.md, plus its length and SHA-256 against USER_SENTENCE, the bytes written there. The zip over on_disk, with c/CLAUDE.md third and a length guard, checks the same against the file on disk (met).
Row 4: the harness_version literal is "2.1.283" == MEASURED_VERSION (met).
Row 5: the fallback test asserts the kinds in the new order and documents[2].path == h/.claude/CLAUDE.md (met by code; not run).
Row 6: the two-renders test asserts 5 per record, differing == ["claude_md_chain"], documents[3] is claude_md_chain, and documents[3].sha256 equals the changed file's hash (met by code; not run).
Row 7 (drift injection): NOT RUN. The instructions forbid me running any test command. Tracing the code under the old push order predicts that exactly three tests fail: a_config_claude_md_is_listed_after_the_appended_instructions_and_mcp_config (kinds array), the_first_render_records_five_documents_in_the_request_s_order_under_the_render_event (KINDS), and the_rendering_process_s_config_dir_is_never_read_and_home_dot_claude_is_the_fallback (kinds). Two tests pass under both orders: two_renders (index 3 is claude_md_chain in both orders, and it compares records to each other) and home_dot_claude_…listed_once… (empty out directory). The workflow's drift run must supply the failing names and the pass count.
- Deviation: The drift-injection record this row asks for (failing test names and the count of passing tests) is a prediction from the code, not a measured run: this stage may not run tests. The measured record must come from the workflow's run.
- Files changed:
  - modified: `crates/lys-home/tests/given_record.rs` — Fixture::new writes c/CLAUDE.md with a fixed line (USER_SENTENCE). KINDS holds the five kinds in the request's order. The first-render test is renamed the_first_render_records_five_documents_in_the_request_s_order_under_the_render_event and asserts 5 documents, the length and hash of each file on disk (c/CLAUDE.md at index 2) and harness_version "2.1.283". The two-renders test and the given listing assert 5 documents per record, and the changed hash is checked at index 3. The fallback test asserts the new kinds with h/.claude/CLAUDE.md at index 2 and the memory index at index 4. The count-of-0 user_claude_md assertion is removed.
- Checklist delivery:
  - [x] C84 — The given_tests.rs unit test and the fixture test in tests/given_record.rs, whose config directory now holds a user CLAUDE.md, each assert that appended_instructions and mcp_config precede user_claude_md. — The fixture test's config directory now holds a user CLAUDE.md, and the test asserts appended_instructions and mcp_config precede it.
- Story delivery:
  - [x] S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents. — The end-to-end binary test gates the request's order by path, length and hash.

### R5: State the one rule and the superseded order in RECORD.md

Rewrite the documents part of the lys.given paragraph in docs/design/home/RECORD.md to state one rule: the documents follow the order the harness's request gives, which wins wherever it and the order the harness reads the files in differ, so the entry lists appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index; the MCP configuration sits straight after the appended instructions, both harness-side inputs ahead of the first message, until a real server's tools position is measured; within one directory the order stays CLAUDE.md, .claude/CLAUDE.md, CLAUDE.local.md; and D/.claude/CLAUDE.md is listed once, as user_claude_md. Name the old order, user_claude_md, appended_instructions, mcp_config, claude_md_chain, memory_index, as superseded by the commit that lands HOME-011 and by that commit's date; say that entries written under it stand as written, that the old order was written under harness_version 2.1.283, that the harness version does not tell the two orders apart, and that a reader tells an entry's order by comparing its recorded time with that commit's date, an entry written before it listing the old order. THE SYSTEM SHALL NOT claim that harness_version marks the order, SHALL NOT add a data member to the paragraph's data list, and SHALL NOT change any other paragraph of RECORD.md.

**Acceptance:**
- The lys.given paragraph names appended_instructions, mcp_config, user_claude_md, claude_md_chain and memory_index as the documents' order, in that order.
- The paragraph contains the words `request` and `read order` in the sentence stating which order wins.
- The paragraph contains `superseded` and `HOME-011`, and the old order user_claude_md, appended_instructions, mcp_config, claude_md_chain, memory_index.
- The paragraph states that harness_version does not tell the two orders apart, and that an entry's order is told by its recorded time against the landing commit's date.
- `grep -n 'listed once, first' docs/design/home/RECORD.md` prints nothing.
- The paragraph's data list is still exactly `{harness, harness_version, kinds, config_dir, documents, environment}`.

**Files:**
- modify: docs/design/home/RECORD.md

**Checklist:**
- C85 — RECORD.md states one rule, that the request's order wins wherever it and the read order differ with the MCP configuration straight after the appended instructions, and names the old order as superseded by the commit that lands HOME-011, told from an entry by its recorded time and never by its harness_version.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: the paragraph lists the five kinds in the new order (met).
Row 2: 'wherever the request's order and the read order, the order the harness reads the files in, differ, the request's order wins' contains both 'request' and 'read order' (met).
Row 3: the paragraph contains 'superseded', 'HOME-011' and the old order (met).
Row 4: it states 'the harness version does not tell the two orders apart' and 'a reader tells an entry's order by comparing its recorded time with the date of the commit that lands HOME-011' (met).
Row 5: `grep -n 'listed once, first'` prints nothing (met).
Row 6: the data list `{harness, harness_version, kinds, config_dir, documents, environment}` is unchanged (met).
No other paragraph changed.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/home/RECORD.md` — The documents part of the lys.given paragraph now states one rule: the request's order wins over the read order. It gives the order appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index, the MCP configuration's place as provisional, the within-directory order unchanged and D/.claude/CLAUDE.md listed once, as user_claude_md. It names the old order as superseded by the commit that lands HOME-011, says entries written under it stand, and says harness_version (2.1.283 for both orders) does not tell the orders apart; an entry's recorded time against that commit's date does.
- Checklist delivery:
  - [x] C85 — RECORD.md states one rule, that the request's order wins wherever it and the read order differ with the MCP configuration straight after the appended instructions, and names the old order as superseded by the commit that lands HOME-011, told from an entry by its recorded time and never by its harness_version. — One rule is stated, the old order is named superseded, and the reader is told to use recorded time, never harness_version.
- Story delivery:
  - [x] S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents. — The record's documented order is the request's.

### R6: Record the supersession on HOME-003 and render its markdown

Append one amendment to docs/design/home/briefs/HOME-003.json, as a new `amendments` member holding one entry, dated the day it is written and attributed to HOME-011, whose ruling names R2's cross-kind order (user_claude_md first) as superseded by HOME-011 and ADR-031, the documents now following the request's order appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index, with the within-directory order unchanged. Then run scripts/design/render-cluster.py on docs/design/home so briefs/HOME-003.md is what HOME-003.json renders to; S12's wording and every other rendered file of the cluster are already committed as rendered, so the render has nothing else to change. THE SYSTEM SHALL NOT change any key of HOME-003.json other than adding `amendments`, SHALL NOT edit a rendered markdown file by hand and SHALL NOT change any authored JSON of the cluster other than HOME-003.json.

**Acceptance:**
- Loaded as JSON, docs/design/home/briefs/HOME-003.json at 7b53625 and after the change are equal in every key except `amendments`, and after the change `amendments` holds exactly one entry.
- That entry's ruling contains `HOME-011`, `ADR-031` and `superseded`.
- docs/design/home/USER-STORIES.md contains `in the order the request gives them` and does not contain `in the order the harness reads them`.
- `sh scripts/design/gate.sh` exits 0.

**Files:**
- modify: docs/design/home/briefs/HOME-003.json
- modify: docs/design/home/briefs/HOME-003.md

**Checklist:**
- C87 — S12 reads `in the order the request gives them`, HOME-003 carries an amendment naming its R2 cross-kind order as superseded, and every rendered markdown file of the home cluster is what its JSON renders to.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: a Python check confirmed that HOME-003.json before and after, loaded as JSON, are equal in every key but `amendments`, and that `amendments` holds one entry (met).
Row 2: the ruling contains 'HOME-011', 'ADR-031' and 'superseded' (met).
Row 3: USER-STORIES.md now contains 'in the order the request gives them' and no longer the old wording (met).
Row 4: gate.sh not run by me. render-cluster.py docs/design/home was run and all 10 rendered files are now its output, which is what gate.sh compares. The amendment carries exactly the schema's four required keys.
- Deviation: The brief says the render would change only HOME-003.md, but the brief commit 8c6f5bb had left the cluster's rendered files stale. It changed stories.json, checklist.json and design.json and added HOME-011.json without re-rendering. So render-cluster.py also rewrote USER-STORIES.md, DESIGN.md and CHECKLIST.md and created briefs/HOME-011.md. All four are in the design's structure array. None was edited by hand, and gate.sh requires them to match their JSON.
- Files changed:
  - modified: `docs/design/home/briefs/HOME-003.json` — Adds only an `amendments` member with one entry: date 2026-09-27, by HOME-011. Its ruling names R2's cross-kind order (user_claude_md first) as superseded by HOME-011 and ADR-031, gives the new order, and says the within-directory order is unchanged.
  - modified: `docs/design/home/briefs/HOME-003.md` — Re-rendered by render-cluster.py: carries the amendment and S12's new wording.
  - modified: `docs/design/home/USER-STORIES.md` — Re-rendered: S12 reads 'in the order the request gives them'.
  - modified: `docs/design/home/DESIGN.md` — Re-rendered from design.json as committed in 8c6f5bb: the ADR-031 line and three HOME-011 structure rows.
  - modified: `docs/design/home/CHECKLIST.md` — Re-rendered from checklist.json as committed in 8c6f5bb: C83 to C87.
  - created: `docs/design/home/briefs/HOME-011.md` — The rendered markdown of HOME-011.json (a path in the design's structure array).
- Checklist delivery:
  - [x] C87 — S12 reads `in the order the request gives them`, HOME-003 carries an amendment naming its R2 cross-kind order as superseded, and every rendered markdown file of the home cluster is what its JSON renders to. — S12's wording is updated, the HOME-003 amendment is recorded, and every rendered file is re-rendered.
- Story delivery:
  - [x] S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the request gives them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents. — The rendered story reads 'in the order the request gives them'.

## Boundaries

- The within-directory order (CLAUDE.md, .claude/CLAUDE.md, CLAUDE.local.md) and CHAIN_FILES do not change.
- No data member is added to lys.given; its six keys, its kinds lists and its config_dir shape stay as they are, and record/given.rs and record/entries.rs are not changed.
- No lys.given entry already written is rewritten, migrated or re-read to change it; the record is append-only.
- measure.py is committed and run byte for byte as it is, and no MCP server is added to the measurement.
- No line PROOF-GIVEN.md already holds is edited, reordered or removed; the new measurement is appended below it.
- No document content, request body or fixture marker appears in PROOF-GIVEN.md, a test name, an error or output: paths, counts, orders, versions and hashes only.
- The template_render event, the manifest, the launch line and the config directory's resolution do not change, and nothing in lys-home launches Claude Code (ADR-007).
- MEASURED_VERSION holds the version the re-measurement ran on: 2.1.283 when that is what is installed, the installed version otherwise; no other version is written.
- No commit after the one that lands HOME-011 writes that commit's id or date into RECORD.md or PROOF-GIVEN.md; a reader finds both in git.
- Measuring a real MCP server's tools position is not part of this brief.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- From the repository root: cargo fmt --all leaves the tree unchanged.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0.
- cargo test --workspace --all-features exits 0 and lists R3's and R4's named tests as passed.
- cargo doc --no-deps --all-features and cargo doc --no-deps exit 0 with no warnings.
- sh scripts/design/gate.sh exits 0.
- shasum -a 256 docs/design/home/proof-given/measure.py prints db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d.
- git show 2f1baac9a0e11e643711ec8a49ee0f1357406e91:docs/design/home/proof-given/measure.py | cmp - docs/design/home/proof-given/measure.py exits 0.
- git diff 7b53625 -- docs/design/home/PROOF-GIVEN.md shows added lines only, all after line 209.
- git diff 7b53625 -- crates/lys-home/src/record/given.rs crates/lys-home/src/record/entries.rs prints nothing.
- With the user_claude_md push moved back ahead of the appended instructions in resolve_given, cargo test -p lys-home fails exactly the three tests R4's drift acceptance names; restored, it passes.
