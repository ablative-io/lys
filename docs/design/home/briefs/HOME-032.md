---
type: brief
id: HOME-032
cluster: home
title: State the rendered-uuid rule with its test vector in RECORD.md, pin it by a test, amend the launch card and decide ADR-016
---

# HOME-032: State the rendered-uuid rule with its test vector in RECORD.md, pin it by a test, amend the launch card and decide ADR-016

> **Cluster:** home
> **Depends on:** HOME-007
> **Design anchor:**
> - ADR-016 — A rendered file's derived uuids are a fixed, versioned contract — Derive every uuid a render needs and the record does not hold as UUIDv5 under the session's namespace and the name `<entry id>#<role>`, with a closed set of roles (`record` first); the session's namespace is UUIDv5 over one fixed lys namespace (32c05904-d1f1-550c-9eee-2f6c8f98b665, itself UUIDv5 of the RFC 9562 URL namespace over `lys/home/claude-code/render-uuid/v1`) and the id of the session being rendered, so the same entry id in two sessions never derives one uuid. Treat the namespace, the session namespace rule, the name form and the roles as frozen: a change is a new version alongside, never a mutation. The fork and launch cards derive by this scheme. Rejected: drawing fresh ids (nondeterministic), keying on a hash of the entry id alone without a namespace, a role and a session (two roles on one entry collide, two sessions with one hand-authored id collide, and it is not reproducible with standard UUID tooling), mixing in the target session id (the record does not hold it), and leaving the scheme mutable until a later card (every recorded hash would move with it).
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> **Checklist:**
> - C45 — A Session asked by find_call for a call id builds, once per open, a map from call id to the first lys.call entry holding it in file order, keeps it current on every append and rebuilds it after a reconcile; once it is built, an ingest through ingest_call, ingest_call_files and ingest_outcome reads no lys.call entry, and a second ingest of a recorded call id records nothing and returns that entry's id.
> - C46 — A Session reports how many entries it has read from its file and how many syncs its own writes made (its line file, its index, its head and the sessions directory), and the block store reports its own syncs beside them, as counts a test reads.
> - C47 — The import command builds a new session under `<id>.jsonl.importing` with no per-entry sync and publishes it with one sync each of the line file, the index and the head and a sessions-directory sync before and after the rename to `<id>.jsonl`, five syncs whatever the record count; a crash before the rename leaves no `<id>.jsonl`, and the next import or open of that id removes what was left.
> - C48 — RECORD.md states the staged import's durability rule beside the per-append rule, as ADR-108 records it.
> - C49 — resume_check counts each transcript's tool_use ids in one pass and reports the same values as before.
> **Stories:**
> - S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.
> - S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.

## Purpose

A rendered Claude Code record's uuid for an entry id that is not uuid-shaped is derived from the home session's id and the entry id together, so the same id in two sessions rendered into one Claude Code directory never collides. HOME-007 R1 already derives it that way in render.rs; what is missing is the contract: RECORD.md states the rule but gives no test vector, no test ties the document to the code, the launch card HOME-002 still specifies the SHA-256 of the entry id alone, and ADR-016 is still proposed. This brief states the one rule with its vector in RECORD.md, pins that vector with a named test so the document and the render cannot drift, amends HOME-002 to name the rule, and moves ADR-016 to decided. A person sees no behaviour change: render.rs and every byte a render writes stay as they are.

## Task

No change to crates/lys-home/src/harness/claude_code/render.rs: record_uuid already returns, for an entry id that is not uuid-shaped, UUIDv5 under session_namespace(session id) over `<entry id>#record`, and its one call site passes session.header().id, the home session's own id. The work is the contract around it. (1) Rewrite the salt sentence of RECORD.md's `## The rendered uuid` section so it names the salt as the home session's own id (the id in the home session's header) and never the render target's session id that the rendered file's `sessionId` carries, and add a test vector table: session `one`, entry `e1`, uuid `83871c7a-20b7-5baa-8f66-8d8f4201d90d`; session `two`, entry `e1`, uuid `96533416-b648-5185-ac85-c3b7c51203c9`. (2) Add the test record_md_states_the_rendered_uuid_rule_with_the_vector_the_render_derives to render_tests.rs, reading RECORD.md at run time and keyed only on the values the document supplies, and prove it with the drift command and its control below, each on its own scratch copy. (3) Append an amendment to HOME-002.json naming the rule in place of R2's SHA-256-alone derivation and re-render HOME-002.md. (4) Move ADR-016 from proposed to decided in docs/design/decisions.json, touching only its status and the ledger's updated date, and show that diff in the dev record. (5) Re-render the cluster's markdown, run the design gate, and prove HOME-008 exists on no other branch; the earlier run's brief branch brief/home/4ce7e4a7-7e38-4649-8fd4-64ae9e6054ca is already deleted from origin by the lead, and the any-branch check excludes no branch by name. Out of scope: ids that are already uuid-shaped, which pass through unchanged; the importer; any harness but Claude Code.

## Requirements

### R1: State the one rendered-uuid rule and its test vector in RECORD.md

The `## The rendered uuid` section of docs/design/home/RECORD.md states one rule: an entry id that is uuid-shaped (36 characters, hex with `-` at 8, 13, 18 and 23) is the record's `uuid`, and the section says so in the words `passes through unchanged`; any other entry id derives as UUID version 5 (RFC 9562) over the name `<entry id>#record` under the session's namespace, which is UUIDv5 of the lys render namespace `32c05904-d1f1-550c-9eee-2f6c8f98b665` over the home session's own id, the id in the home session's header that the render is called with. The section names that salt as the home session's own id and says the render target's session id, the one the rendered file's `sessionId` carries, never enters the derivation. The section gives a test vector as a markdown table with the columns session id, entry id and uuid and exactly two rows, each cell in backticks: `one`, `e1`, `83871c7a-20b7-5baa-8f66-8d8f4201d90d`; and `two`, `e1`, `96533416-b648-5185-ac85-c3b7c51203c9`. The section SHALL NOT state a second derivation as in force, SHALL NOT name the SHA-256 of the entry id alone as the rule, and SHALL NOT change the namespace, the name form or the role set it already states.

**Acceptance:**
- The section contains the strings `32c05904-d1f1-550c-9eee-2f6c8f98b665`, `<entry id>#record`, `home session's own id` and `sessionId`.
- The section holds exactly two table rows of three backticked cells: `one` | `e1` | `83871c7a-20b7-5baa-8f66-8d8f4201d90d` and `two` | `e1` | `96533416-b648-5185-ac85-c3b7c51203c9`.
- `python3 -c "import uuid;ns=uuid.uuid5(uuid.UUID('32c05904-d1f1-550c-9eee-2f6c8f98b665'),'one');print(uuid.uuid5(ns,'e1#record'))"` prints `83871c7a-20b7-5baa-8f66-8d8f4201d90d`, and with `two` in place of `one` prints `96533416-b648-5185-ac85-c3b7c51203c9`.
- The section contains the string `passes through unchanged`, in the sentence that names a uuid-shaped entry id.

**Files:**
- modify: docs/design/home/RECORD.md

**Checklist:**
- C45 — A Session asked by find_call for a call id builds, once per open, a map from call id to the first lys.call entry holding it in file order, keeps it current on every append and rebuilds it after a reconcile; once it is built, an ingest through ingest_call, ingest_call_files and ingest_outcome reads no lys.call entry, and a second ingest of a recorded call id records nothing and returns that entry's id.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

### R2: Pin the RECORD.md vector with a named render test and prove it with a drift command

Add to crates/lys-home/src/harness/claude_code/render_tests.rs a test named record_md_states_the_rendered_uuid_rule_with_the_vector_the_render_derives. WHEN the test runs, THE SYSTEM SHALL read docs/design/home/RECORD.md from the crate's manifest directory, take the vector rows from the `## The rendered uuid` section, assert that it found exactly two rows, and for each row render a one-entry session of a fresh home whose session id and entry id are the row's and assert the rendered record's `uuid` equals the row's uuid, and assert the two rows share one entry id and give two different uuids. The test SHALL take every session id, entry id and expected uuid from the document, SHALL NOT hold a copy of the vector in the test source, and SHALL NOT compute an expected value from the render it checks. No other test SHALL read the vector from RECORD.md. THE SYSTEM SHALL NOT change render.rs, any existing test in render_tests.rs, or tests/claude_code_round_trip.rs and its pinned fixture hash.

**Acceptance:**
- `cargo test -p lys-home --all-features record_md_states_the_rendered_uuid_rule_with_the_vector_the_render_derives` reports 1 passed and 0 failed.
- The source of record_md_states_the_rendered_uuid_rule_with_the_vector_the_render_derives contains neither `83871c7a-20b7-5baa-8f66-8d8f4201d90d` nor `96533416-b648-5185-ac85-c3b7c51203c9`.
- On a scratch copy of the landed head, `scratch=$(mktemp -d) && git archive HEAD | tar -x -C "$scratch" && sed -i.orig 's/83871c7a-20b7-5baa-8f66-8d8f4201d90d/83871c7a-20b7-5baa-8f66-8d8f4201d90e/' "$scratch/docs/design/home/RECORD.md" && grep -c '83871c7a-20b7-5baa-8f66-8d8f4201d90e' "$scratch/docs/design/home/RECORD.md" && (cd "$scratch" && cargo test -p lys-home --all-features --no-fail-fast 2>&1) | grep -E '^test .* \.\.\. FAILED$'` prints `1` and then exactly one line, `test harness::claude_code::render_tests::record_md_states_the_rendered_uuid_rule_with_the_vector_the_render_derives ... FAILED`.
- The control, on a fresh scratch copy with no sed and no grep -c step: `scratch=$(mktemp -d) && git archive HEAD | tar -x -C "$scratch" && (cd "$scratch" && cargo test -p lys-home --all-features --no-fail-fast 2>&1) > "$scratch.out"; grep -c -F 'test harness::claude_code::render_tests::record_md_states_the_rendered_uuid_rule_with_the_vector_the_render_derives ... ok' "$scratch.out"; grep -c -E '^test .* \.\.\. FAILED$' "$scratch.out"` prints `1` and then `0`: the named test ran and passed, and no test failed.
- `git diff 1756688 -- crates/lys-home/src/harness/claude_code/render.rs crates/lys-home/tests/claude_code_round_trip.rs` prints nothing.
- `git diff 1756688 -- crates/lys-home/src/harness/claude_code/render_tests.rs` removes no line.
- `cargo test --workspace --all-features` reports every test that passed at 1756688 as passing, the_same_entry_id_in_two_sessions_derives_two_uuids_salted_by_the_session_id among them.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/render_tests.rs

**Checklist:**
- C46 — A Session reports how many entries it has read from its file and how many syncs its own writes made (its line file, its index, its head and the sessions directory), and the block store reports its own syncs beside them, as counts a test reads.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

### R3: Amend the launch card to name the rendered-uuid rule

Append one entry to the `amendments` array of docs/design/home/briefs/HOME-002.json whose subject names the rendered uuid and whose ruling states that R2's derivation from the SHA-256 of the entry id's bytes is replaced by the rule docs/design/home/RECORD.md states: UUIDv5 over `<entry id>#record` under UUIDv5 of `32c05904-d1f1-550c-9eee-2f6c8f98b665` over the home session's own id, never the render target's session id, with a uuid-shaped entry id passing through unchanged (ADR-016). Then regenerate docs/design/home/briefs/HOME-002.md with scripts/design/render-brief.py. THE SYSTEM SHALL NOT rewrite HOME-002's requirements, acceptance, boundaries, verification or its existing amendment.

**Acceptance:**
- HOME-002.json's `amendments` array holds 2 entries, and the first is equal field for field to the one at 1756688.
- The second amendment's ruling contains `32c05904-d1f1-550c-9eee-2f6c8f98b665`, `<entry id>#record`, `home session's own id` and `RECORD.md`.
- HOME-002.json's `requirements` array is equal field for field to the one at 1756688.
- `sh scripts/design/gate.sh` prints no line naming docs/design/home/briefs/HOME-002.md.

**Files:**
- modify: docs/design/home/briefs/HOME-002.json
- modify: docs/design/home/briefs/HOME-002.md

**Checklist:**
- C47 — The import command builds a new session under `<id>.jsonl.importing` with no per-entry sync and publishes it with one sync each of the line file, the index and the head and a sessions-directory sync before and after the rename to `<id>.jsonl`, five syncs whatever the record count; a crash before the rename leaves no `<id>.jsonl`, and the next import or open of that id removes what was left.

**Stories:**
- S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.

### R4: Move ADR-016 from proposed to decided with a two-line ledger diff

In docs/design/decisions.json set ADR-016's `status` from `proposed` to `decided`, and set the ledger's top-level `updated` to the committer date, as `git log -1 --format=%cs` prints it, of the commit that makes the change; when that date equals the value `updated` holds at 1756688, the `updated` line stays as it is. THE SYSTEM SHALL NOT change ADR-016's title, scope, date, decided_by, context, decision, quote, consequences, supersedes or superseded_by, and SHALL NOT change any other decision or any other top-level field. The dev record shows the output of `git diff 1756688 -- docs/design/decisions.json`.

**Acceptance:**
- In docs/design/decisions.json, ADR-016's `status` is `decided`.
- Let D0 be what `git show 1756688:docs/design/decisions.json | python3 -c 'import json,sys;print(json.load(sys.stdin)["updated"])'` prints, and D1 what `git log -1 --format=%cs -- docs/design/decisions.json` prints on the landed head. When D1 equals D0, `git diff -U0 1756688 -- docs/design/decisions.json | grep -E '^[-+][^-+]'` prints exactly two lines, `-      "status": "proposed",` and `+      "status": "decided",`. When D1 differs from D0, it prints exactly four lines: `-  "updated": "D0",` and `+  "updated": "D1",` with D0 and D1 written in, then the same two status lines.
- `python3 scripts/design/validate.py docs/design/decisions.json` prints `All 1 document(s) valid.`
- The dev record holds the output of `git diff 1756688 -- docs/design/decisions.json`.

**Files:**
- modify: docs/design/decisions.json

**Checklist:**
- C48 — RECORD.md states the staged import's durability rule beside the per-append rule, as ADR-108 records it.

**Stories:**
- S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.

### R5: Render the cluster's markdown and prove HOME-008 exists once

Precondition: the superseded brief branch of the earlier run, brief/home/4ce7e4a7-7e38-4649-8fd4-64ae9e6054ca, which carried an earlier HOME-008.json at 50135be, is deleted from origin by the lead before the build; the build does not delete it and does not exclude it. Regenerate the cluster's markdown with scripts/design/render-cluster.py so docs/design/home/DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/HOME-008.md are what their JSON renders to. WHEN every ref of the repository's remote has been fetched, THE SYSTEM SHALL show that every commit touching docs/design/home/briefs/HOME-008.json on any branch is an ancestor of the landed head, so a HOME-008.json on any other branch fails this check. THE SYSTEM SHALL NOT write a second brief under HOME-008, SHALL NOT write any other brief file, and SHALL NOT delete another branch's commits to pass the check.

**Acceptance:**
- `git ls-remote origin refs/heads/brief/home/4ce7e4a7-7e38-4649-8fd4-64ae9e6054ca` prints nothing.
- `sh scripts/design/gate.sh` exits 0.
- After `git fetch origin '+refs/heads/*:refs/remotes/origin/*'`, `for c in $(git log --all --format=%H -- docs/design/home/briefs/HOME-008.json); do git merge-base --is-ancestor "$c" HEAD || echo "$c"; done` prints nothing, and `git log --all --format=%H -- docs/design/home/briefs/HOME-008.json | wc -l` prints at least 1.
- `ls docs/design/home/briefs/ | grep -c '^HOME-008\.'` prints `2` (HOME-008.json and HOME-008.md).

**Files:**
- create: docs/design/home/briefs/HOME-008.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md

**Checklist:**
- C49 — resume_check counts each transcript's tool_use ids in one pass and reports the same values as before.

**Stories:**
- S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.

## Boundaries

- No change to crates/lys-home/src/harness/claude_code/render.rs: its derivation, RENDER_NAMESPACE, the name form `<entry id>#<role>`, the role set and the call site passing the home session's own id stay as they are.
- The render target's session id never enters a derived uuid, in the code or in any rule this brief writes.
- Uuid-shaped entry ids pass through unchanged; this brief does not touch them.
- No change to tests/claude_code_round_trip.rs, the fixture hash it pins, PROOF-RESUME.md, import.rs or the ids it writes.
- decisions.json changes by ADR-016's status and the ledger's updated date only.
- HOME-002's requirements and its existing amendment are not rewritten, only added to.
- No person sees a behaviour change: no rendered byte, report field or CLI output changes.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- From the repository root: cargo fmt --all leaves the tree unchanged.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0.
- cargo test --workspace --all-features exits 0 and lists record_md_states_the_rendered_uuid_rule_with_the_vector_the_render_derives as passed.
- cargo doc --no-deps --all-features and cargo doc --no-deps exit 0 with no warnings.
- sh scripts/design/gate.sh exits 0.
- Run R2's drift command on a scratch copy of the landed head and read exactly one FAILED line, naming the test R2 adds; run R2's control on a fresh scratch copy and read the named test's `... ok` line and no FAILED line.
- git diff 1756688 -- crates/lys-home/src/harness/claude_code/render.rs prints nothing.
