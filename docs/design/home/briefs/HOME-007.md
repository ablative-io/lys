---
type: brief
id: HOME-007
cluster: home
title: Make the Claude Code render byte-deterministic for the same session head and version
---

# HOME-007: Make the Claude Code render byte-deterministic for the same session head and version

> **Cluster:** home
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-016 — A rendered file's derived uuids are a fixed, versioned contract — Derive every uuid a render needs and the record does not hold as UUIDv5 under the session's namespace and the name `<entry id>#<role>`, with a closed set of roles (`record` first); the session's namespace is UUIDv5 over one fixed lys namespace (32c05904-d1f1-550c-9eee-2f6c8f98b665, itself UUIDv5 of the RFC 9562 URL namespace over `lys/home/claude-code/render-uuid/v1`) and the id of the session being rendered, so the same entry id in two sessions never derives one uuid. Treat the namespace, the session namespace rule, the name form and the roles as frozen: a change is a new version alongside, never a mutation. The fork and launch cards derive by this scheme. Rejected: drawing fresh ids (nondeterministic), keying on a hash of the entry id alone without a namespace, a role and a session (two roles on one entry collide, two sessions with one hand-authored id collide, and it is not reproducible with standard UUID tooling), mixing in the target session id (the record does not hold it), and leaving the scheme mutable until a later card (every recorded hash would move with it).
> **Checklist:**
> - C31 — The Claude Code render derives the uuid of a record whose entry id is not uuid-shaped as UUIDv5 over the session's namespace, itself UUIDv5 of 32c05904-d1f1-550c-9eee-2f6c8f98b665 over the id of the session being rendered, and the name `<entry id>#record`, so two sessions with the same non-uuid entry id never collide; render.rs calls no random source and no clock.
> - C32 — A fixture session holding a tool record with two results and a tool record with one result beside user text renders twice, to two paths with the same target, into files of equal SHA-256.
> - C33 — A test pins the fixture's rendered SHA-256, and PROOF-RESUME.md records the same value with the commands that produce it.
> - C34 — The PROOF-RESUME source renders twice from one import with one recorded command into files of equal SHA-256, and PROOF-RESUME.md records that hash and the command.
> - C35 — PROOF-RESUME.md names the cause found, the namespace, name form and roles the multi-result case used, and a resume of the rendered file with --fork-session on the installed Claude Code, with the version `claude --version` printed and repeated_tool_use_ids 0.
> **Stories:**
> - S18 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the same session head rendered with the same lys-home version to give the same bytes every time, so that I can tell a rendered file by its hash and a receipted render event names what was written.
> - S19 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the proof document to name why two renders of a multi-result tool record differed, so that the fix is checked against its cause rather than its symptom.

## Purpose

A render of the same session head with the same target and the same lys-home version must give the same bytes every time, so a rendered file is told by its hash and a receipted render event can name what was written; the launch template's acceptance that two renders yield identical hashes stands on it. Today two renders differ whenever the context path holds an entry whose id is not uuid-shaped, which the importer writes for every tool record with more than one result. This brief removes that cause, derives the missing uuids from the record by the scheme docs/design/home/design.json fixes (CN9, ADR-016), pins the result with a fixture and a recorded hash, and re-measures the resume in PROOF-RESUME.md.

## Task

The cause is in crates/lys-home/src/harness/claude_code/render.rs: record_uuid gives an entry id that is not 36-character uuid-shaped a fresh random uuid from record::fresh_id on every render, and that value also becomes the next record's parentUuid, a summary's leafUuid and an assistant's `msg_` id. The importer (import.rs) writes such ids as `<uuid>-r<i>` for every tool result of a user record except a last one that stands alone, so any record with more than one result, and any single result beside user text, triggers it. The walk is otherwise already deterministic: entries in context-path order, parts in array order, serde_json objects with sorted keys (preserve_order is not enabled), timestamps copied from the entry, no clock read; keep it so. Replace fresh_id in record_uuid with UUIDv5 (RFC 9562) over the namespace and name form design.json fixes, add a committed synthetic fixture that carries both defect cases, gate determinism in render_tests.rs, pin the fixture's hash at the public API in tests/claude_code_round_trip.rs and record it with its commands in a new section of PROOF-RESUME.md, then complete that section with the cause and the measurements on the PROOF-RESUME source, including a resume of that source's render on the Claude Code installed where the proof runs, with the version `claude --version` prints. The rendered records' `version` field and the CLI's `--version` default of 2.1.281 stay as they are. HOME-001's render (R4) and CLI (R9) are on main; this brief changes their bytes for non-uuid entry ids only. Out of scope: the importer, the record's shape, anything a rendered file holds beyond the determinism, the loss account's fields, and any harness but Claude Code.

## Requirements

### R1: Derive the uuid of a record whose entry id is not uuid-shaped from the record

WHEN the Claude Code render writes a record for a message entry whose id is not 36 characters of hex digits with dashes at positions 8, 13, 18 and 23, THE SYSTEM SHALL write as the record's `uuid` the UUID version 5 (RFC 9562: SHA-1 over the 16 namespace bytes then the name, version nibble 5, variant bits 10) over the session's namespace and the UTF-8 name formed by the entry id, then `#`, then the role `record`, in lowercase hyphenated form, where the session's namespace is the UUID version 5 over the lys render namespace 32c05904-d1f1-550c-9eee-2f6c8f98b665 and the UTF-8 id of the session being rendered (the id in the home session's header), so the same non-uuid entry id in two sessions never derives the same uuid; the next record's parentUuid, a summary's leafUuid and an assistant record's `msg_` id SHALL follow from that value exactly as they follow from a uuid-shaped entry id's uuid. WHEN the entry id is uuid-shaped, THE SYSTEM SHALL write it unchanged. THE SYSTEM SHALL take every timestamp from the entry's own stamp. THE SYSTEM SHALL NOT call record::fresh_id, any random source or any clock from render.rs, SHALL NOT derive a uuid from the target session id, the out path or the time, SHALL NOT change the value of any other field of any record, and SHALL NOT change import.rs or the ids it writes. SHA-1 comes from the RustCrypto `sha1` crate at 0.10 added as a workspace dependency; no other dependency is added.

**Acceptance:**
- A unit test renders a home session with id `one` whose context path holds a user entry with id `u1` and asserts the rendered record's uuid is `8614322e-bd70-5b0c-baa8-571010e52a8c` (computed independently by Python's uuid.uuid5 over uuid.uuid5 of the namespace and `one`, then `u1#record`).
- The namespace constant in render.rs equals the UUIDv5 of 6ba7b811-9dad-11d1-80b4-00c04fd430c8 over `lys/home/claude-code/render-uuid/v1`, asserted in a test as the string `32c05904-d1f1-550c-9eee-2f6c8f98b665`.
- The same entry id `u1` rendered from a second session with id `two` gives `9c843336-82b9-5a39-b7cd-2889252f8e4e`, not the first session's value: the derived uuid is salted with the session id, so two sessions with the same non-uuid entry id never collide.
- A message entry with id `11111111-1111-4111-8111-111111111111` renders with uuid `11111111-1111-4111-8111-111111111111`.
- `grep -nE 'fresh_id|rand::|now\(|SystemTime|OffsetDateTime|Instant' crates/lys-home/src/harness/claude_code/render.rs` prints nothing.
- `git diff dfcca65 -- crates/lys-home/src/harness/claude_code/import.rs crates/lys-home/src/record/entries.rs` prints nothing.
- The two existing tests in render_tests.rs pass unchanged.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/Cargo.toml
- modify: Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C31 — The Claude Code render derives the uuid of a record whose entry id is not uuid-shaped as UUIDv5 over the session's namespace, itself UUIDv5 of 32c05904-d1f1-550c-9eee-2f6c8f98b665 over the id of the session being rendered, and the name `<entry id>#record`, so two sessions with the same non-uuid entry id never collide; render.rs calls no random source and no clock.

**Stories:**
- S18 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the same session head rendered with the same lys-home version to give the same bytes every time, so that I can tell a rendered file by its hash and a receipted render event names what was written.

### R2: Gate that a multi-result fixture renders twice to the same bytes

Add crates/lys-home/tests/fixtures/multi_result.jsonl, a synthetic Claude Code transcript of six records carrying no real transcript content: a user question (11111111-1111-4111-8111-111111111111), an assistant record with two tool_use parts (22222222-2222-4222-8222-222222222222), a user record holding exactly two tool_result parts and no other part (33333333-3333-4333-8333-333333333333), an assistant record with one tool_use part (44444444-4444-4444-8444-444444444444), a user record holding one tool_result part and one text part (55555555-5555-4555-8555-555555555555), and an assistant answer (66666666-6666-4666-8666-666666666666), each with a fixed timestamp. WHEN that fixture is imported once into a session of the home named `multi` and the session is rendered twice with the same target to two different out paths, THE SYSTEM SHALL write two files of equal bytes and two loss accounts of equal bytes. THE SYSTEM SHALL NOT write two records with the same uuid in one rendered file.

**Acceptance:**
- A test in render_tests.rs imports the fixture once into a session named `multi`, renders it with session id aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa, cwd /elsewhere, model claude-opus-5-5 and version 2.1.281 to `a.jsonl` and to `b.jsonl`, and asserts SHA-256(a.jsonl) == SHA-256(b.jsonl) and bytes(a.loss.json) == bytes(b.loss.json).
- Each render reports 8 records; the rendered uuids in order are exactly 11111111-1111-4111-8111-111111111111, 22222222-2222-4222-8222-222222222222, d0426444-d38e-5376-aef6-035f7c3634e1, 33333333-3333-4333-8333-333333333333, 44444444-4444-4444-8444-444444444444, dcc867fa-10a3-5142-89a0-38367729ae4b, 55555555-5555-4555-8555-555555555555, 66666666-6666-4666-8666-666666666666.
- The 8 uuids are 8 distinct strings, each 36 characters with `-` at positions 8, 13, 18 and 23 and a hex digit everywhere else.
- Record 1's parentUuid is null and record n's parentUuid equals record n-1's uuid for n = 2 to 8.
- Replacing the derivation in record_uuid with record::fresh_id makes this test fail on unequal SHA-256, recorded as a drift injection in the dev record.

**Files:**
- create: crates/lys-home/tests/fixtures/multi_result.jsonl
- modify: crates/lys-home/src/harness/claude_code/render_tests.rs

**Checklist:**
- C32 — A fixture session holding a tool record with two results and a tool record with one result beside user text renders twice, to two paths with the same target, into files of equal SHA-256.

**Stories:**
- S18 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the same session head rendered with the same lys-home version to give the same bytes every time, so that I can tell a rendered file by its hash and a receipted render event names what was written.

### R3: Pin the fixture's rendered hash at the public API and record it in the proof document

WHEN the fixture is imported and rendered through lys_home::cli::run with the target of R2, THE SYSTEM SHALL write a file whose SHA-256 equals a hex constant pinned in tests/claude_code_round_trip.rs. Add a new section to docs/design/home/PROOF-RESUME.md, headed as the deterministic render, that records that constant as the fixture's rendered SHA-256, measured with `shasum -a 256`, beside the exact lys-home import and render commands that produce it and the lys-home commit built from. The test SHALL read PROOF-RESUME.md and assert that it contains the constant. The test SHALL NOT compute the expected value from the render it checks and SHALL NOT read the constant from the proof document. The section SHALL NOT carry any transcript content.

**Acceptance:**
- A test in tests/claude_code_round_trip.rs runs the import and render commands through `run`, and asserts the rendered file's SHA-256 equals the pinned constant.
- The same test asserts docs/design/home/PROOF-RESUME.md contains the pinned constant.
- The new section of PROOF-RESUME.md records the fixture's SHA-256 equal to the pinned constant, the import command, the render command and a commit id.
- Running the recorded import and render commands at the landed commit and then `shasum -a 256` on the rendered file prints the pinned constant.
- Changing one hex digit of the recorded value in PROOF-RESUME.md makes exactly this test fail, recorded as a drift injection in the dev record.

**Files:**
- modify: crates/lys-home/tests/claude_code_round_trip.rs
- modify: docs/design/home/PROOF-RESUME.md

**Checklist:**
- C33 — A test pins the fixture's rendered SHA-256, and PROOF-RESUME.md records the same value with the commands that produce it.

**Stories:**
- S18 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the same session head rendered with the same lys-home version to give the same bytes every time, so that I can tell a rendered file by its hash and a receipted render event names what was written.

### R4: Record the cause, the PROOF-RESUME source's hashes and its resume in the proof document

Complete the section of docs/design/home/PROOF-RESUME.md that R3 adds with: the cause found (record_uuid gave a fresh random uuid through record::fresh_id to every entry id that is not uuid-shaped, which the importer's `<uuid>-r<i>` ids for split tool results are) and that no map or set order and no clock was a cause; the derivation used (namespace 32c05904-d1f1-550c-9eee-2f6c8f98b665, the session's namespace as its UUIDv5 over the session id, name `<entry id>#<role>`) and the role the multi-result case used (`record`, for the fixture's two `-r0` entries); the PROOF-RESUME source (SHA-256 793f4e87ea4608b2ddfb9d5dcbf1197d4245bde7ed1612630339a1cc1f6d15df) imported once and rendered twice with one recorded render command to two out paths, and both hashes; the counts of that source's user records with more than one tool_result and of its user records mixing a tool_result with other parts, and the count of entries on its context path whose id is not uuid-shaped, with a plain statement that where these are 0 the render holds no derived uuid, so its equal hashes and its resume show no regression and do not exercise the derivation; and the first of those two rendered files, resumed as the earlier proof was, with `claude -p --resume <rendered> --fork-session` from a directory that is neither the rendered file's directory nor the session's cwd, with the string `claude --version` prints on the same run recorded beside it, the exit status, and `lys-home resume-check`'s report. The section SHALL NOT carry any transcript content, only hashes, counts, ids and commands, and the PROOF-RESUME source SHALL NOT be modified. Then regenerate the cluster's rendered markdown with render-cluster.py.

**Acceptance:**
- The section contains the strings `record_uuid`, `fresh_id`, `-r<i>`, `32c05904-d1f1-550c-9eee-2f6c8f98b665` and `#record`.
- The section records two hashes for the PROOF-RESUME source's two renders and they are equal.
- The section records the three counts of the PROOF-RESUME source named in the spec, each as 0, and the statement that its render holds no derived uuid.
- The section names the resumed file as the first of the PROOF-RESUME source's two renders, by the hash it records for that render.
- The section records the output of `claude --version` taken on the resume run, beside that run's command, and the resume's exit status as 0.
- The section records resume-check's repeated_tool_use_ids for the resumed file as 0.
- The source hash the section records after the resume is 793f4e87ea4608b2ddfb9d5dcbf1197d4245bde7ed1612630339a1cc1f6d15df.
- `sh scripts/design/gate.sh` exits 0, so DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/HOME-007.md are what their JSON renders to.

**Files:**
- create: docs/design/home/briefs/HOME-007.md
- modify: docs/design/home/PROOF-RESUME.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md
- modify: crates/lys-home/README.md
- modify: docs/design/home/RECORD.md

**Checklist:**
- C34 — The PROOF-RESUME source renders twice from one import with one recorded command into files of equal SHA-256, and PROOF-RESUME.md records that hash and the command.
- C35 — PROOF-RESUME.md names the cause found, the namespace, name form and roles the multi-result case used, and a resume of the rendered file with --fork-session on the installed Claude Code, with the version `claude --version` printed and repeated_tool_use_ids 0.

**Stories:**
- S19 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the proof document to name why two renders of a multi-result tool record differed, so that the fix is checked against its cause rather than its symptom.

## Boundaries

- No change to crates/lys-home/src/harness/claude_code/import.rs or the ids it writes, and no change to the home record's shape (record/entries.rs, Pi's grammar).
- No change to what a rendered record holds apart from the uuid derived for a non-uuid-shaped entry id and the parentUuid, leafUuid and `msg_` id that follow from it; the `version` field and the CLI's `--version` default of 2.1.281 stay as they are.
- No change to the loss account's fields, and no harness other than Claude Code.
- The namespace, the session's namespace as its UUIDv5 over the session id, the name form `<entry id>#<role>` and the role `record` are exactly as design.json names them; no other role is added and the target session id is not mixed into the name or the namespace.
- No transcript content in the proof document, test names, fixture or output; the PROOF-RESUME source is not modified and not committed.
- The resume is run by hand with Claude Code's own `--resume <path>`; nothing in lys-home launches Claude Code.
- No dependency other than the RustCrypto `sha1` crate at 0.10 is added.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- From the repository root: cargo fmt --all leaves the tree unchanged.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0.
- cargo test --workspace --all-features exits 0 and the output lists R1's, R2's and R3's tests as passed.
- cargo doc --no-deps --all-features and cargo doc --no-deps exit 0 with no warnings.
- sh scripts/design/gate.sh exits 0.
- python3 -c "import uuid;ns=uuid.UUID('32c05904-d1f1-550c-9eee-2f6c8f98b665');print(uuid.uuid5(uuid.uuid5(ns,'multi'),'33333333-3333-4333-8333-333333333333-r0#record'))" prints d0426444-d38e-5376-aef6-035f7c3634e1, the value R2's test asserts.
- Import crates/lys-home/tests/fixtures/multi_result.jsonl into a scratch home with lys-home import and render it twice with the command PROOF-RESUME.md records; shasum -a 256 prints the same hash for both files and it equals the hash PROOF-RESUME.md records.
- git diff dfcca65 -- crates/lys-home/src/harness/claude_code/import.rs crates/lys-home/src/record/entries.rs prints nothing.
