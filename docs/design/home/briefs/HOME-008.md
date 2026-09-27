---
type: brief
id: HOME-008
cluster: home
title: Measure whether a template's CLAUDE_CONFIG_DIR moves the session's config directory, and record the directory the session uses
---

# HOME-008: Measure whether a template's CLAUDE_CONFIG_DIR moves the session's config directory, and record the directory the session uses

> **Cluster:** home
> **Depends on:** HOME-003
> **Design anchor:**
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-013 — The context record is a lys.given custom entry of document hashes, never copies — The context record is one lys.given custom entry, appended after the render event, whose data is the harness name, the Claude Code version the load order was measured on, the kinds as two lists, resolved (claude_md_chain, user_claude_md, memory_index, appended_instructions, mcp_config, environment_names) and unlisted (claude_md_imports and claude_rules, which this entry does not list and a later entry at the first request records), the config directory as its path and its source (template or home), the documents in the measured order each as kind, path, byte length and SHA-256, and the names of the environment variables the template set. It is not a copy of each document into the block store, and not a content-bearing record, because the entry must hold no content under home P7 and CN3. It is unsigned and unencrypted now, and because it names hashes only, signing and encryption at rest can be added later without changing what is recorded.
> - ADR-031 — The context record names the config directory the session reads from; a template's value --settings does not apply is recorded as given and not applied, in a new version — HOME-008 measures, through a rendered template on a named Claude Code version, whether the session reads its CLAUDE.md and memory index from the template's CLAUDE_CONFIG_DIR or from HOME/.claude. On a no answer the config directory is HOME/.claude only, with source home, never the renderer's own CLAUDE_CONFIG_DIR, and a render appends a new custom type lys.given.v2 alongside lys.given whose config_dir carries the applied path and source and the template's value as given_not_applied; given and given-check read both types in file order, each by its own shape. On a yes answer the record's shape does not change. On a split the answer is recorded per document and per-document config directories are a new card; no single applied directory is guessed. Entries already written stand as written and the correction lives in PROOF-GIVEN.md and RECORD.md. Rejected: changing lys.given's config_dir in place (entries of that shape are held), appending corrections to old sessions (the record is append-only), and reading the renderer's own CLAUDE_CONFIG_DIR (the launching shell is not the renderer).
> **Checklist:**
> - C83 — PROOF-GIVEN.md records three launches from render-launch's printed line through a template whose env slot alone sets CLAUDE_CONFIG_DIR to a fresh directory, with the measured machine's host and claude --version answer and, for CLAUDE.md and for the memory index, whether each was read from the template's directory or from HOME/.claude, as paths, counts and hashes only.
> - C84 — RECORD.md and PROOF-GIVEN.md state the answer with every Claude Code version and host measured or cited, and on a no or split answer name the affected entries as lys.given entries whose config_dir.source is template and whose harness_version is 2.1.283, standing as written. A build machine whose Claude Code version is not 2.1.283 is recorded as a finding naming both versions, and MEASURED_VERSION stays 2.1.283.
> - C85 — On a no answer a render resolves the config directory as HOME/.claude with source home and never takes the template's CLAUDE_CONFIG_DIR as the path. A template CLAUDE_CONFIG_DIR that is not absolute is refused by name before any file is written or entry appended.
> - C86 — On a no answer a render appends one lys.given.v2 entry carrying harness_version 2.1.283, whose config_dir is exactly {path, source, given_not_applied} and no lys.given entry, and lys.given's shape and its key-set test are unchanged.
> - C87 — On a no answer lys-home given lists lys.given and lys.given.v2 entries together in file order, each marked version 1 or 2, and given-check checks each entry by its own version's shape.
> - C88 — On a yes answer nothing under crates/ changes; on a split answer nothing under crates/ changes and RECORD.md and PROOF-GIVEN.md name each document with its directory and the per-document config directories card.
> **Stories:**
> - S35 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want whether a template's CLAUDE_CONFIG_DIR, reaching the session through --settings, moves its config directory measured on a named Claude Code version, so that the context record names the directory the session read from and not the one the template asked for.
> - S36 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the entries that name a directory the session did not use listed by type, source and Claude Code version in RECORD.md and PROOF-GIVEN.md, so that a reader of an old session can tell which of its entries to discount without any entry being rewritten.
> - S37 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want lys-home given to list both versions of the context record in file order, each marked with its version, and given-check to check each by its own shape, so that a session rendered after the change shows its record beside the old ones.

## Purpose

The context record (HOME-003) names the config directory a session read its CLAUDE.md and memory index from, and records source `template` whenever the launch template's env slot sets CLAUDE_CONFIG_DIR. That env slot reaches a launched session only through the settings file the launch line passes with --settings, and the earlier proof set CLAUDE_CONFIG_DIR in the process environment instead, so no one has shown the template's value moves the directory. This brief measures it through a rendered template on the installed Claude Code, names the version, records the answer in PROOF-GIVEN.md and RECORD.md, and, when the answer is no, makes the record name the directory the session does use, with the template's value kept as given and not applied in a new version beside lys.given (ADR-031, ADR-013).

## Task

R1 is a measurement, run on the machine the build runs on with a lys-home built there from the brief's base: render-launch through a template whose env slot alone sets CLAUDE_CONFIG_DIR to a fresh directory C, then run the printed launch line three times against a local listener with the launching environment scrubbed and CLAUDE_CONFIG_DIR absent from it, and read which of C and HOME/.claude each of CLAUDE.md and the memory index came from. ANTHROPIC_BASE_URL and the fixture key ride in the launching environment, not the template, so the capture does not depend on the question being measured. R2 writes the answer into RECORD.md and PROOF-GIVEN.md with every version and host measured or cited: the build machine's version, measured, and 2.1.283, the version entries were rendered under since the context record landed, cited from PROOF-LAUNCH.md and RECORD.md and not re-measured. MEASURED_VERSION stays 2.1.283, the version the load order was measured on; when the build machine's version differs, the two documents record that as a finding naming both versions and the constant does not move. If more than one version is measured and they answer differently, each version's answer is recorded and the code follows the answer of the version measured on the machine the build runs on. The answer decides the rest. `yes`: the card ends at R1 and R2; the record's shape does not change and nothing under crates/ changes. `split`: R1 and R2 record each document with its directory, name the follow-up card `Per-document config directories in the given resolver`, and the card stops; the resolver does not guess one applied directory. `no`: R3 to R7 land. The resolver (harness/claude_code/given.rs) resolves the config directory as HOME/.claude with source home and never takes the template's value as the path; a template CLAUDE_CONFIG_DIR that is not absolute is refused by name as it is today, and render-launch now refuses it before it writes any file or appends any entry, so given_not_applied only ever holds an absolute value; the renderer still never reads its own process's CLAUDE_CONFIG_DIR. A new custom type `lys.given.v2` (record/given_v2.rs, its constant beside CUSTOM_GIVEN in record/entries.rs) carries the same data as lys.given, harness_version still 2.1.283, with `config_dir` `{path, source, given_not_applied}`; render-launch appends it in place of lys.given; `given` and `given-check` read both types in file order, each by its own shape, marking `version` 1 or 2 on each listed record; and the unit and end-to-end tests assert the applied directory. lys.given's type string, struct, serialised shape and key-set test are untouched, and the `template` source variant stays because entries already written carry it. Entries already written stand as written: nothing is appended to an old session, and the correction lives in PROOF-GIVEN.md and RECORD.md. Every leg of the gate runs on the machine the build runs on. Out of scope: any other template slot, per-document config directories, the Claude Code version constant MEASURED_VERSION, and any change to the launch line, the environment file, the template_render event or the manifest.

## Requirements

### R1: Measure whether a template's CLAUDE_CONFIG_DIR, delivered through --settings, moves the session's config directory

WHEN the proof measures the config directory, THE SYSTEM SHALL launch Claude Code by running, on the machine the build runs on, the launch line that `lys-home render-launch` printed, with a lys-home binary built from the brief's base on that machine, for a template whose env slot sets `CLAUDE_CONFIG_DIR` to a directory C created fresh by the proof for the measurement and named by no other file, variable or setting, and whose flags are `-p`, `Reply with the single word ok.`, `--output-format`, `json`, `--max-turns`, `1`. The launching process's environment SHALL hold only `PATH`, `TMPDIR`, `TERM`, `LANG`, `HOME` set to a fixture home H, `ANTHROPIC_BASE_URL` naming a local listener that writes each request body to a file, `ANTHROPIC_API_KEY` set to a fixture string, and the three telemetry-off variables PROOF-GIVEN.md's earlier method names; the settings file passed with `--settings` SHALL be the only carrier of `CLAUDE_CONFIG_DIR`. H/.claude and C SHALL each hold a `CLAUDE.md` and a memory index at `projects/<slug of the working directory>/memory/MEMORY.md`, the four files each carrying one marker token found in no other file. The proof SHALL run the launch three times, rewriting the four files before each run, and SHALL read, for `CLAUDE.md` and for the memory index, which of the two markers the captured request carries. It SHALL write a new section `## The config directory through --settings (HOME-008)` in PROOF-GIVEN.md recording: the host and the `claude --version` answer of the machine measured; the launch line as run, with scratch paths written `<scratch>`; the names (never the values) of the launching environment's variables; the settings file's member names and its env member's variable names; the SHA-256 of each of the four files; per run the exit status, the number of requests captured and, for each document, the number of occurrences of each marker; and per document the directory it was read from, `template` or `home`. The answer SHALL be `yes` when both documents were read from C, `no` when both were read from H/.claude, and `split` when they were read from different directories. IF a run's captured request carries neither marker of a document, THEN THE SYSTEM SHALL record that run as giving no answer and SHALL NOT count it toward any answer. THE SYSTEM SHALL NOT put `CLAUDE_CONFIG_DIR` in the launching process's environment, SHALL NOT put any marker token, any line of any document or any variable's value in the proof, SHALL NOT change any section of PROOF-GIVEN.md written before this one except to add the correction R2 names, and SHALL NOT add a way for lys-home to run the launch line.

**Acceptance:**
- PROOF-GIVEN.md holds a section headed `## The config directory through --settings (HOME-008)`.
- The section names the host measured and a `claude --version` answer for it, and the answer column of its per-machine table holds exactly one of `yes`, `no`, `split` for that host.
- The section's list of the launching environment's variable names does not contain `CLAUDE_CONFIG_DIR`, and its list of the settings file's env variable names contains `CLAUDE_CONFIG_DIR`.
- The section records the settings file's member names as exactly `env`.
- The section records four 64-hex-character SHA-256 values for the four marker files, and the four values are pairwise distinct.
- The section's run table has three rows; each row with an answer shows, for each document, 1 or more occurrences of exactly one of its two markers and 0 of the other.
- The section records for `CLAUDE.md` and for the memory index the directory each was read from, as `template` or `home`, and the recorded answer is `yes` when both are `template`, `no` when both are `home`, and `split` when they differ.
- A search of PROOF-GIVEN.md for each of the four marker tokens, kept in the proof's scratch tree, finds 0 occurrences.
- `git diff 7b53625 -- crates/lys-home/src/harness/claude_code/launch_env.rs crates/lys-home/src/harness/claude_code/template.rs` prints nothing.

**Files:**
- modify: docs/design/home/PROOF-GIVEN.md

**Checklist:**
- C83 — PROOF-GIVEN.md records three launches from render-launch's printed line through a template whose env slot alone sets CLAUDE_CONFIG_DIR to a fresh directory, with the measured machine's host and claude --version answer and, for CLAUDE.md and for the memory index, whether each was read from the template's directory or from HOME/.claude, as paths, counts and hashes only.

**Stories:**
- S35 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want whether a template's CLAUDE_CONFIG_DIR, reaching the session through --settings, moves its config directory measured on a named Claude Code version, so that the context record names the directory the session read from and not the one the template asked for.

### R2: State the answer per Claude Code version, and name the entries it affects, in RECORD.md and PROOF-GIVEN.md

WHEN R1's answer is recorded, THE SYSTEM SHALL state it in RECORD.md beside the `lys.given` description, and SHALL list in RECORD.md and in PROOF-GIVEN.md's `## The config directory through --settings (HOME-008)` every Claude Code version and host measured or cited, with the answer each gave: the version measured by R1 on the machine the build runs on, with its answer; and 2.1.283, the version `lys.given` entries have been rendered under since the context record landed, cited from PROOF-LAUNCH.md and RECORD.md and marked `cited, not re-measured`, its answer column holding R1's answer when the version R1 measured is 2.1.283 and `not measured` when it is not. IF the version R1 measured is not 2.1.283, THEN RECORD.md and PROOF-GIVEN.md SHALL each record a finding naming both versions, and THE SYSTEM SHALL NOT change `MEASURED_VERSION`, which names the version the load order was measured on. IF more than one version is measured and they answer differently, THEN both documents SHALL state each version's answer, and R3 to R7 SHALL follow the answer of the version measured on the machine the build runs on. IF the answer is `no` or `split`, THEN RECORD.md and PROOF-GIVEN.md SHALL name the affected entries as `lys.given` entries whose `config_dir.source` is `template` and whose `harness_version` is 2.1.283, name the Claude Code versions measured, and for `split` name each document with the directory it was read from; they SHALL state that those entries stand as written. IF the answer is `no`, THEN RECORD.md SHALL describe the `lys.given.v2` custom entry R4 defines, its data keys and its `config_dir` keys `path`, `source` and `given_not_applied`, and SHALL state that a render appends it in place of `lys.given`. IF the answer is `split`, THEN RECORD.md and PROOF-GIVEN.md SHALL name the card `Per-document config directories in the given resolver` as the follow-up, and THE SYSTEM SHALL NOT change any file under crates/. IF the answer is `yes`, THEN THE SYSTEM SHALL NOT change any file under crates/ and SHALL NOT change the record's shape. THE SYSTEM SHALL NOT rewrite, remove or append to any entry of any session to correct it, and SHALL NOT put any document content or variable value in either document.

**Acceptance:**
- RECORD.md holds the answer R1 recorded, and names the version R1 measured and 2.1.283, each with its answer; 2.1.283 is marked `cited, not re-measured`.
- When the version R1 measured is 2.1.283: the 2.1.283 row's answer equals R1's answer. When it is not 2.1.283: the 2.1.283 row's answer is `not measured`, and RECORD.md and PROOF-GIVEN.md each hold a finding naming both versions.
- With the answer `no` recorded: RECORD.md and PROOF-GIVEN.md each hold a paragraph naming the affected entries by custom type `lys.given`, `config_dir.source` `template` and `harness_version` 2.1.283, and stating they stand as written.
- With the answer `no` recorded: RECORD.md describes `lys.given.v2` with `config_dir` keys `path`, `source` and `given_not_applied`.
- With the answer `split` recorded: RECORD.md and PROOF-GIVEN.md each list `CLAUDE.md` and the memory index with the directory (`template` or `home`) each was read from, name `Per-document config directories in the given resolver`, and `git diff 7b53625 -- crates` prints nothing.
- With the answer `yes` recorded: `git diff 7b53625 -- crates` prints nothing.
- `git diff 7b53625 --stat` lists no session file under any home.
- `grep -n 'MEASURED_VERSION: &str' crates/lys-home/src/harness/claude_code/given.rs` prints a line whose value is `"2.1.283"`.

**Files:**
- modify: docs/design/home/RECORD.md
- modify: docs/design/home/PROOF-GIVEN.md

**Checklist:**
- C84 — RECORD.md and PROOF-GIVEN.md state the answer with every Claude Code version and host measured or cited, and on a no or split answer name the affected entries as lys.given entries whose config_dir.source is template and whose harness_version is 2.1.283, standing as written. A build machine whose Claude Code version is not 2.1.283 is recorded as a finding naming both versions, and MEASURED_VERSION stays 2.1.283.
- C88 — On a yes answer nothing under crates/ changes; on a split answer nothing under crates/ changes and RECORD.md and PROOF-GIVEN.md name each document with its directory and the per-document config directories card.

**Stories:**
- S36 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the entries that name a directory the session did not use listed by type, source and Claude Code version in RECORD.md and PROOF-GIVEN.md, so that a reader of an old session can tell which of its entries to discount without any entry being rewritten.

### R3: Resolve the config directory as HOME/.claude and hand the template's value on as given and not applied

WHILE the answer PROOF-GIVEN.md's HOME-008 section records for the Claude Code version measured on the machine the build runs on is `no`, WHEN a render resolves the config directory, THE SYSTEM SHALL resolve it as `HOME/.claude` with `HOME` from the rendering process's environment and source `home`, whether or not the template's env slot sets `CLAUDE_CONFIG_DIR`, and SHALL hand the template's `CLAUDE_CONFIG_DIR` value, exactly as the env slot holds it, to the record as given and not applied (none when the slot sets none). IF the template's env slot sets `CLAUDE_CONFIG_DIR` to a value that is not an absolute path (empty, relative, or beginning with `~`), THEN THE SYSTEM SHALL refuse with `HomeError::NotAbsolute` naming `CLAUDE_CONFIG_DIR` and the value's shape, as it refuses today, before any use of `HOME`, and SHALL hand no value on as given and not applied: only an absolute value that was not applied is handed on. IF `HOME` is absent or empty, THEN THE SYSTEM SHALL refuse with the existing refusal `HomeError::NoConfigDir`, whether or not the template sets `CLAUDE_CONFIG_DIR`. THE SYSTEM SHALL NOT take the template's value as the path the documents are resolved under, SHALL NOT yield source `template` from a render, SHALL NOT read the rendering process's own `CLAUDE_CONFIG_DIR`, and SHALL NOT remove the `template` variant of the source, which `lys.given` entries already written carry. The module docs of given.rs SHALL state this rule and the PROOF-GIVEN.md section it stands on. IF that answer is `yes` or `split`, THEN THE SYSTEM SHALL NOT change crates/lys-home/src/harness/claude_code/given.rs or crates/lys-home/src/harness/claude_code/given_tests.rs.

**Acceptance:**
- With the answer `no` recorded: a unit test resolving with a template value C (an absolute path) and HOME h asserts the path is `h/.claude`, the source is `home`, and the value handed on as given and not applied is C.
- With the answer `no` recorded: a unit test resolving with no template value and HOME h asserts the path `h/.claude`, the source `home`, and nothing handed on as given and not applied.
- With the answer `no` recorded: a unit test resolving with a template value C and no HOME asserts the refusal `HomeError::NoConfigDir`.
- With the answer `no` recorded: a unit test resolving with the template value `~/c` and HOME h asserts the refusal `HomeError::NotAbsolute` whose `what` is `CLAUDE_CONFIG_DIR`, and that its displayed text contains `CLAUDE_CONFIG_DIR`.
- With the answer `no` recorded: a unit test where C holds a memory index for the working directory and h/.claude holds none asserts the resolved documents hold no path under C.
- With the answer `no` recorded: `grep -n 'ConfigSource::Template' crates/lys-home/src/harness/claude_code/given.rs` prints nothing.
- With the answer `yes` recorded: `git diff 7b53625 -- crates/lys-home/src/harness/claude_code/given.rs crates/lys-home/src/harness/claude_code/given_tests.rs` prints nothing.
- With the answer `split` recorded: `git diff 7b53625 -- crates/lys-home/src/harness/claude_code/given.rs crates/lys-home/src/harness/claude_code/given_tests.rs` prints nothing.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/given.rs
- modify: crates/lys-home/src/harness/claude_code/given_tests.rs

**Checklist:**
- C85 — On a no answer a render resolves the config directory as HOME/.claude with source home and never takes the template's CLAUDE_CONFIG_DIR as the path. A template CLAUDE_CONFIG_DIR that is not absolute is refused by name before any file is written or entry appended.

**Stories:**
- S35 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want whether a template's CLAUDE_CONFIG_DIR, reaching the session through --settings, moves its config directory measured on a named Claude Code version, so that the context record names the directory the session read from and not the one the template asked for.

### R4: Define lys.given.v2 alongside lys.given, with the template's value named as given and not applied

WHILE the answer PROOF-GIVEN.md's HOME-008 section records for the Claude Code version measured on the machine the build runs on is `no`, THE SYSTEM SHALL declare the custom type `lys.given.v2` beside `lys.given` in entries.rs, and define its record in record/given_v2.rs with data exactly `{harness, harness_version, kinds, config_dir, documents, environment}`, the five keys other than `config_dir` holding what `lys.given` holds, `harness_version` being `MEASURED_VERSION`, 2.1.283, whatever version the machine the build runs on has installed, and `config_dir` exactly `{path, source, given_not_applied}`: `path` and `source` the applied config directory and its source, and `given_not_applied` the template's `CLAUDE_CONFIG_DIR` value as a string, or JSON null when the template set none. The record SHALL be appended as a `lys.given.v2` entry under the event it follows, leaving the head where it stands, SHALL be read back from an entry of that custom type, SHALL refuse an entry of any other type by id, and SHALL list every `lys.given.v2` entry of a session in file order. THE SYSTEM SHALL NOT change `lys.given`'s custom type string, record struct, serialised shape or readers in record/given.rs, and SHALL NOT change the test in record/given_tests.rs that pins `config_dir`'s keys as exactly `path` and `source`. IF that answer is `yes` or `split`, THEN THE SYSTEM SHALL NOT change crates/lys-home/src/record/entries.rs, crates/lys-home/src/record/mod.rs or crates/lys-home/src/record/given.rs, and SHALL NOT create crates/lys-home/src/record/given_v2.rs or crates/lys-home/src/record/given_v2_tests.rs.

**Acceptance:**
- With the answer `no` recorded: entries.rs declares a constant whose value is `lys.given.v2`, and the constant whose value is `lys.given` is unchanged.
- With the answer `no` recorded: a unit test in given_v2_tests.rs serialises a record whose template value is `/c` and asserts the data's keys are exactly `config_dir, documents, environment, harness, harness_version, kinds` and `config_dir`'s keys are exactly `given_not_applied, path, source`, with `given_not_applied` equal to `/c`, `source` equal to `home` and `harness_version` equal to `2.1.283`.
- With the answer `no` recorded: a unit test serialises a record with no template value and asserts `config_dir.given_not_applied` is JSON null.
- With the answer `no` recorded: a unit test appends a record under an event, reads it back from the entry, and asserts it equals the record written; reading a `lys.given` entry as `lys.given.v2` is refused naming the entry's id.
- With the answer `no` recorded: `git diff 7b53625 -- crates/lys-home/src/record/given.rs crates/lys-home/src/record/given_tests.rs` prints nothing, and the tests in record/given_tests.rs pass.
- With the answer `yes` recorded: crates/lys-home/src/record/given_v2.rs does not exist and `git diff 7b53625 -- crates/lys-home/src/record/entries.rs crates/lys-home/src/record/mod.rs` prints nothing.
- With the answer `split` recorded: crates/lys-home/src/record/given_v2.rs does not exist and `git diff 7b53625 -- crates/lys-home/src/record/entries.rs crates/lys-home/src/record/mod.rs` prints nothing.

**Files:**
- create: crates/lys-home/src/record/given_v2.rs
- create: crates/lys-home/src/record/given_v2_tests.rs
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C86 — On a no answer a render appends one lys.given.v2 entry carrying harness_version 2.1.283, whose config_dir is exactly {path, source, given_not_applied} and no lys.given entry, and lys.given's shape and its key-set test are unchanged.

**Stories:**
- S35 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want whether a template's CLAUDE_CONFIG_DIR, reaching the session through --settings, moves its config directory measured on a named Claude Code version, so that the context record names the directory the session read from and not the one the template asked for.

### R5: Append a lys.given.v2 entry, and no lys.given entry, at every render

WHILE the answer PROOF-GIVEN.md's HOME-008 section records for the Claude Code version measured on the machine the build runs on is `no`, WHEN `render-launch` renders a session, THE SYSTEM SHALL append exactly one `lys.given.v2` entry under the `template_render` event it follows, built from R3's resolution and the template's `CLAUDE_CONFIG_DIR` value, and SHALL report that entry's id as the report's `given`. IF the template's env slot sets `CLAUDE_CONFIG_DIR` to a value that is not an absolute path, THEN `render-launch` SHALL refuse with R3's refusal naming `CLAUDE_CONFIG_DIR` before it writes any file under `--out` and before it appends any entry, and SHALL exit 1. THE SYSTEM SHALL NOT append a `lys.given` entry, SHALL NOT change the `template_render` event, the manifest, the launch line, the environment file's bytes, the environment names the record lists or any other member of the report. IF that answer is `yes` or `split`, THEN THE SYSTEM SHALL NOT change crates/lys-home/src/harness/claude_code/launch.rs.

**Acceptance:**
- With the answer `no` recorded: after one render of a session holding no given entry, the session holds exactly one entry of custom type `lys.given.v2`, its parent is the `template_render` event's id, its id equals the report's `given`, and the session holds 0 entries of custom type `lys.given`.
- With the answer `no` recorded: `grep -n 'GivenRecord::claude_code' crates/lys-home/src/harness/claude_code/launch.rs` prints nothing.
- With the answer `no` recorded: in the given_record.rs test R7 adds, render-launch with a template whose `CLAUDE_CONFIG_DIR` is `~/c` exits 1, its stderr contains `CLAUDE_CONFIG_DIR`, its stdout is empty, the `--out` directory holds 0 files, and the session's line count equals its count before the render.
- With the answer `yes` recorded: `git diff 7b53625 -- crates/lys-home/src/harness/claude_code/launch.rs` prints nothing.
- With the answer `split` recorded: `git diff 7b53625 -- crates/lys-home/src/harness/claude_code/launch.rs` prints nothing.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/launch.rs

**Checklist:**
- C86 — On a no answer a render appends one lys.given.v2 entry carrying harness_version 2.1.283, whose config_dir is exactly {path, source, given_not_applied} and no lys.given entry, and lys.given's shape and its key-set test are unchanged.

**Stories:**
- S35 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want whether a template's CLAUDE_CONFIG_DIR, reaching the session through --settings, moves its config directory measured on a named Claude Code version, so that the context record names the directory the session read from and not the one the template asked for.

### R6: List and check both versions of the context record, each by its own version

WHILE the answer PROOF-GIVEN.md's HOME-008 section records for the Claude Code version measured on the machine the build runs on is `no`, WHEN `lys-home given` lists a session, THE SYSTEM SHALL list every `lys.given` and every `lys.given.v2` entry together in the order they stand in the session file, each record carrying `version` 1 for `lys.given` and 2 for `lys.given.v2`, with `count` the number of both. WHEN `lys-home given-check` checks an entry, THE SYSTEM SHALL read it by its own custom type's shape and answer by that record's listed document, as it answers today. THE SYSTEM SHALL NOT read a `lys.given` entry by the `lys.given.v2` shape or the reverse, SHALL NOT refuse a `lys.given` entry for lacking `given_not_applied`, SHALL NOT change the exit statuses or the refusals `given-check` gives, and SHALL NOT print any byte of any file. IF that answer is `yes` or `split`, THEN THE SYSTEM SHALL NOT change crates/lys-home/src/cli/given.rs, and SHALL NOT create crates/lys-home/tests/given_versions.rs.

**Acceptance:**
- With the answer `no` recorded: an integration test in given_versions.rs builds a session holding one `lys.given` entry appended through the library and then renders it once, runs `lys-home given`, and asserts `count` is 2, `records[0].version` is 1 with `records[0].entry` the `lys.given` entry's id, and `records[1].version` is 2 with `records[1].entry` the render report's `given`.
- With the answer `no` recorded: in that test, `given-check` on the `lys.given` entry's id with a listed path and the file at that path exits 0 with `answer` `matches`, and on the `lys.given.v2` entry's id with a listed path and the file at that path exits 0 with `answer` `matches`.
- With the answer `no` recorded: in that test, `given-check` on the `lys.given.v2` entry's id with a path it does not list exits 2.
- With the answer `yes` recorded: `git diff 7b53625 -- crates/lys-home/src/cli/given.rs` prints nothing and crates/lys-home/tests/given_versions.rs does not exist.
- With the answer `split` recorded: `git diff 7b53625 -- crates/lys-home/src/cli/given.rs` prints nothing and crates/lys-home/tests/given_versions.rs does not exist.

**Files:**
- create: crates/lys-home/tests/given_versions.rs
- modify: crates/lys-home/src/cli/given.rs

**Checklist:**
- C87 — On a no answer lys-home given lists lys.given and lys.given.v2 entries together in file order, each marked version 1 or 2, and given-check checks each entry by its own version's shape.

**Stories:**
- S37 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want lys-home given to list both versions of the context record in file order, each marked with its version, and given-check to check each by its own shape, so that a session rendered after the change shows its record beside the old ones.

### R7: Make the end-to-end context record test assert the applied directory and the template's value as given and not applied

WHILE the answer PROOF-GIVEN.md's HOME-008 section records for the Claude Code version measured on the machine the build runs on is `no`, THE SYSTEM SHALL change given_record.rs so that, for its fixture template setting `CLAUDE_CONFIG_DIR` to C with the rendering process's HOME h, it asserts the render's entry is `lys.given.v2` with `config_dir` `{path: h/.claude, source: home, given_not_applied: C}` and that no listed document's path lies under C. The test that proves the rendering process's own `CLAUDE_CONFIG_DIR` is never read SHALL keep asserting that. THE SYSTEM SHALL add one test to given_record.rs that renders the fixture with a template whose env slot sets `CLAUDE_CONFIG_DIR` to `~/c` and asserts R5's refusal: exit 1, stderr naming `CLAUDE_CONFIG_DIR`, an empty stdout, no file under `--out` and no entry appended. THE SYSTEM SHALL NOT remove a test from given_record.rs, and SHALL NOT keep any assertion that a render records source `template`. IF that answer is `yes` or `split`, THEN THE SYSTEM SHALL NOT change crates/lys-home/tests/given_record.rs.

**Acceptance:**
- With the answer `no` recorded: given_record.rs asserts `config_dir` equal to `{"path": h/.claude, "source": "home", "given_not_applied": C}` for the fixture render.
- With the answer `no` recorded: given_record.rs asserts no listed document path starts with C.
- With the answer `no` recorded: `grep -n '"source": "template"' crates/lys-home/tests/given_record.rs` prints nothing.
- With the answer `no` recorded: `cargo test -p lys-home --test given_record` lists every test name it lists at 7b53625 plus exactly one more, all passed.
- With the answer `yes` recorded: `git diff 7b53625 -- crates/lys-home/tests/given_record.rs` prints nothing.
- With the answer `split` recorded: `git diff 7b53625 -- crates/lys-home/tests/given_record.rs` prints nothing.

**Files:**
- modify: crates/lys-home/tests/given_record.rs

**Checklist:**
- C85 — On a no answer a render resolves the config directory as HOME/.claude with source home and never takes the template's CLAUDE_CONFIG_DIR as the path. A template CLAUDE_CONFIG_DIR that is not absolute is refused by name before any file is written or entry appended.
- C86 — On a no answer a render appends one lys.given.v2 entry carrying harness_version 2.1.283, whose config_dir is exactly {path, source, given_not_applied} and no lys.given entry, and lys.given's shape and its key-set test are unchanged.

**Stories:**
- S35 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want whether a template's CLAUDE_CONFIG_DIR, reaching the session through --settings, moves its config directory measured on a named Claude Code version, so that the context record names the directory the session read from and not the one the template asked for.

## Boundaries

- No template slot other than the env slot's CLAUDE_CONFIG_DIR is examined or changed, and launch_env.rs, template.rs, the launch line, the environment file's bytes, the template_render event and the manifest do not change.
- lys.given's custom type string, its record struct, its serialised shape, its readers and the key-set test in record/given_tests.rs do not change; a change of shape is lys.given.v2 alongside, never a mutation.
- No entry of any session in any home is rewritten, removed or appended to in order to correct it.
- The renderer never reads its own process's CLAUDE_CONFIG_DIR.
- On a `split` answer no resolver or record change lands and no single applied directory is chosen; per-document config directories are a separate card.
- On a `yes` answer nothing under crates/ changes.
- MEASURED_VERSION stays 2.1.283 and the harness_version value a record carries is not changed by this brief, whatever version the build machine runs; a difference is recorded as a finding naming both versions.
- No document content, marker token or environment variable value appears in the proof, RECORD.md, test names, output or errors, except the template's CLAUDE_CONFIG_DIR value, which lys.given.v2's given_not_applied records, `given` lists, and the not-absolute refusal names as it does today.
- lys-home gains no act of running a session; the proof runs the printed launch line itself.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- From the repository root: cargo fmt --all leaves the tree unchanged.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0.
- cargo test --workspace --all-features exits 0.
- cargo doc --no-deps --all-features and cargo doc --no-deps exit 0 with no warnings.
- sh scripts/design/gate.sh exits 0.
- Read the answer in PROOF-GIVEN.md's `## The config directory through --settings (HOME-008)`: with `yes` or `split`, `git diff 7b53625 -- crates` prints nothing; with `no`, `cargo test -p lys-home --test given_versions` and `cargo test -p lys-home --test given_record` pass.
- `git diff 7b53625 -- crates/lys-home/src/record/given.rs crates/lys-home/src/record/given_tests.rs crates/lys-home/src/harness/claude_code/launch_env.rs crates/lys-home/src/harness/claude_code/template.rs` prints nothing.
