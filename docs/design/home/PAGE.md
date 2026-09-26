# home — what was asked, what it means, and what was written

## The words, as they were typed

Translation is designed and not proved: the lantern work measured three session formats and left the converter out, so the first translation is one named pair, one session, and an account of what was kept, changed and lost, and nothing above it assumes it. The pair is Claude Code to Codex, the pair the lantern work measured: Codex's own importer of Claude Code files is the recorded baseline, a compaction and not a copy, dropping thinking, sidechains and meta records and clipping tool calls and results, and Tom ruled on 5 September 2026 that we improve on it and do not adopt it. This card renders a home session that was imported from Claude Code as a Codex rollout that Codex 0.156.0 on this Mac resumes, with a loss account beside it. A lys-home subcommand takes the home, the session, a target directory and the Codex version it renders for, walks the context path as the Claude Code render does, and writes a rollout file in the shape that version of Codex writes and reads for its own threads, measured from the files Codex writes here and never assumed from the June checkout; every text part is carried whole, every tool call and result is carried whole as Codex's own items for a call and its output and never as a truncated note, and readable thinking is carried as text while opaque blocks are dropped and named by hash in the loss account, as P3 rules for another provider. The rendered thread announces itself in band as a translated context with the source session id and the render's template hash, following the marker precedent, so the fork can never mistake itself for the original. The loss account is written beside the rollout as JSON and lists, by entry id and block hash, what was kept, what changed shape and how, and what was lost and why: sidechains, harness events, lanterns and other lys entries, and any block kind Codex has no item for. The report is JSON with the rollout path, entry and block counts, the loss counts and the account's path, never content. Acceptance is that a fixture session imported from a fixture Claude Code file renders to a rollout whose items match a recorded fixture rollout for Codex 0.156.0 apart from ids and timestamps; that the loss account names every dropped block by hash with a reason and every changed block with its before and after kinds; that a session with a thinking block from another provider renders it as text and the account says so; that a tool result longer than Codex's importer clips is carried whole; that rendering to an existing path is refused by path and writes nothing; that the rendered rollout is resumed by Codex 0.156.0 on this Mac and answers from its content, recorded in a proof document with hashes, counts and paths only; and that the Claude Code render of the same session is unchanged, checked by hash. Not in scope: Codex to Claude Code, which does not exist in any tree here; adopting or calling Codex's importer; Pi or any third harness; launching Codex through the launch template, which is that template's next version. Filed by Archie on Tom's roadmap stage 4b of 22 September 2026 at 14:45, his 5 September 2026 ruling on the Codex converter, the lantern design's session-formats measurement, and home DESIGN P1, P3 and P7 at lys main 0073b966, on 26 September 2026.

## What the survey found, and its angles

The words ask for the first proved cross-harness translation, from Claude Code to Codex. A new lys-home subcommand reads a home session that was imported from Claude Code, walks its context path and writes a Codex rollout in the exact shape Codex 0.156.0 writes on this Mac. Every text part, tool call and tool result is carried whole. Readable thinking is carried as text and opaque blocks are dropped by hash. The thread carries an in-band translated-context marker, and a JSON loss account beside it says, per entry and block, what was kept, what changed shape and what was lost. It is proved by a fixture-for-fixture match against a recorded 0.156.0 rollout, a real resume by Codex 0.156.0 on this Mac, and a hash check showing the Claude Code render is unchanged.

### What the tree holds

- `crates/lys-home/src/harness/claude_code/render.rs` — The Claude Code render the new subcommand must walk 'as it does': it takes session.context_path() with an optional canon first, renders a compaction as a summary record, skips custom entries and labels, applies P3 per part (same provider, api and model keeps signed thinking, otherwise readable text and dropped opaque parts), refuses an existing path with create_new, and writes a loss account listing only dropped parts, each as the SHA-256 of the Pi-shaped part JSON. 318 lines, 265 of code. Its bytes must not change.
- `crates/lys-home/src/harness/claude_code/import.rs` — It decides what a Claude-imported session holds. Sidechains hang off the main path as side branches with a label. attachment and system records sit on the chain as lys.harness_event custom entries, and permission-mode is a side leaf. isMeta is not handled, so meta records import as ordinary messages. Blocks are stored as the SHA-256 of the Claude-shaped source part. 531 lines, 473 of code, so it is near the 500-line cap.
- `crates/lys-home/src/record/entries.rs` — Defines the lys custom types the loss account must name as lost: lys.harness_event, lys.call, lys.authored, lys.inherited and lys.given. No lantern custom type exists anywhere in src.
- `crates/lys-home/src/record/blocks.rs` — The content-addressed store behind 'block hash'. Its put hashes the bytes it is given, so the loss account's hash must be defined against what import stored, not the Pi-shaped part the Claude Code render hashes.
- `crates/lys-home/src/cli.rs` — Where subcommands are declared (Import, Render, Canon, Fewshot, IngestCall, ResumeCheck, render-launch, given and given-check). 424 lines of code, so a new subcommand's body belongs in a split-out file, following the cli/given.rs precedent.
- `crates/lys-home/src/cli/given.rs` — The precedent for splitting a subcommand out of cli.rs and for JSON reports that carry no content.
- `crates/lys-home/src/harness/mod.rs` — Harness profiles, currently 5 lines with Claude Code only. A codex profile module lands beside claude_code.
- `crates/lys-home/src/harness/claude_code/mod.rs` — Holds PROVIDER=anthropic, API=anthropic-messages and HARNESS=claude-code, which P3's same-provider test keys on. Every Claude thinking block fails that test against an OpenAI target.
- `crates/lys-home/src/error.rs` — The named refusals, such as HomeError::Exists { path } for an existing target. A Codex render adds its own, such as an unmeasured Codex version or no place to write.
- `crates/lys-home/tests/claude_code_round_trip.rs` — Builds its Claude Code transcript in code (no fixture file exists under tests/fixtures), so the 'fixture Claude Code file' and 'recorded fixture rollout' are both new files.
- `crates/lys-home/tests/fixtures/launch/` — The only fixture directory: template.json and session.jsonl. No Claude Code source fixture and no Codex rollout fixture exist.
- `docs/design/home/design.json` — The cluster design the card must continue. Its non-goal at line 77 excludes 'Harnesses other than Claude Code', and its Structure table needs new rows for the codex module, fixtures, tests and proof document. The rendered DESIGN.md must match it under scripts/design/gate.sh.
- `docs/design/home/briefs/HOME-001.json` — Line 440 says 'No harness but Claude Code, and no translation to another harness's file'. This card is the next brief, HOME-004, which lifts that for one pair.
- `docs/design/home/RECORD.md` — Documents the home record and the loss account as they stand. A second loss-account shape (kept, changed and lost) needs writing down here.
- `docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md` — Stage 4b, lines 81-85: the first translation is a separate proof of one named harness pair with its own loss account, and nothing above it assumes it.
- `/Users/tom/Developer/ablative/tools/lantern/docs/02-SESSION-FORMATS.md` — Section 3 is the baseline: Codex's importer (external-agent-sessions at d667082322) drops isMeta and isSidechain records and thinking, clips tool input to 2,000 characters (NOTE_MAX_LEN) and results to 4,000 (TOOL_RESULT_MAX_LEN), and ends with the literal <EXTERNAL SESSION IMPORTED> marker. It also holds Tom's 5 September ruling, including 'on-demand from the last compaction boundary'.
- `/Users/tom/Developer/ablative/tools/lantern/docs/03-DESIGN.md` — Section 5 'The converter: ours, on demand, everything the file holds' (keep everything, thinking's readable tenth as text, fidelity recorded) and the in-band marker rule at lines 121-123.
- `~/.codex/sessions and ~/.codex/state_5.sqlite` — Where Codex 0.156.0 writes its own threads, the only source the rollout shape may be measured from. Resume is by id or name only ('codex resume [SESSION_ID]'), and the threads table (with rollout_path) indexes every thread, so a file in a directory may not be found without being indexed.
- `/Users/tom/.bun/bin/codex` — The Codex 0.156.0 binary on this Mac. /Users/tom/.npm-global/bin/codex (0.36.0) comes first on PATH.

### What was already decided

- home P1 — A rendered resume file, a compaction and a translation are derived records stored beside their source and pointing at it, and the original bytes are never rewritten.
- home P3 — Opaque blocks render whole only when provider, api and model all match. Any other target gets readable thinking as plain text, and opaque blocks are dropped and named by hash in the loss account.
- home P6 — A resume path is a per-harness, per-version measurement, never an assumption.
- home P7 — Transcript contents never appear in a post, log, error, test name or page. Status carries only hashes, counts and offsets.
- home CN1 — A render writes a new file only, and refuses an existing path by name.
- home CN2 — A rendered file never carries an opaque block from a different provider or model family, and the loss account names each dropped block by hash.
- home CN3 — No transcript content in output, logs, errors, test names or pages.
- home CN6 — No Norn crate is a dependency and no Norn type is copied in. The analogue here is that Codex's importer is neither adopted nor called.
- home Non-Goals — 'Harnesses other than Claude Code, and Chat Completions or Responses translation beyond keeping the raw call bytes' is a non-goal in design.json:77, and HOME-001.json:440 says 'no translation to another harness's file'.
- home Goals — 'A written loss account for every derived record: what a render or translation preserved, transformed and could not carry.'
- ADR-012 — A launch template per harness, with each render recorded as a template_render side leaf. It explicitly rejects 'a transcript converter or adapter protocol per harness' as the launch mechanism.
- ADR-013 — The lys.given context record is hashes only, written at a launch-template render.
- CONTEXT-ROADMAP stage 4b — Translation is designed, not proved. The first translation is a separate proof of one named harness pair with its own loss account, and nothing above it assumes it.
- STATEMENT-2026-09-22 'The home is portable' — A translation is a derived version with the original intact and an explicit account of what it preserved, transformed and could not carry. What each destination can resume is measured per harness.
- lantern 02-SESSION-FORMATS §3 (Tom, 2026-09-05) — 'we improve on this, we do not adopt it. Ours is on-demand from the last compaction boundary and keeps everything the file holds.'
- lantern 03-DESIGN §4.5 and §5 — The in-band marker, with <EXTERNAL SESSION IMPORTED> as the precedent. The converter keeps everything: full tool input and results, sidechains labelled, and fidelity recorded rather than implied.

### What was measured

- Codex binaries on this Mac: 2: /Users/tom/.bun/bin/codex reports codex-cli 0.156.0, and /Users/tom/.npm-global/bin/codex reports codex-cli 0.36.0 and resolves first on PATH
- Codex's own update check: ~/.codex/version.json has latest_version 0.156.1, last checked 2026-09-24
- Codex rollout files under ~/.codex/sessions: 794 .jsonl files
- Rollouts whose session_meta says cli_version 0.156.0: 1 (codex-tui). It holds 20 message items and 3 reasoning items, and no function_call, function_call_output, custom_tool_call or custom_tool_call_output
- response_item types across 2026 rollouts (all versions): reasoning 107,835; custom_tool_call_output 83,105; custom_tool_call 83,100; message 37,051; function_call_output 20,047; function_call 19,613; agent_message 11,530; compaction 182
- Fields on the measured tool items: function_call: arguments, call_id, id, internal_chat_message_metadata_passthrough, name, namespace, type. function_call_output: call_id, id, name, namespace, output, and the same passthrough field. custom_tool_call: call_id, id, input, name, status. reasoning: encrypted_content, id, summary
- Top-level line keys in a current rollout: ordinal, payload, timestamp, type. Line types seen: session_meta, turn_context, response_item, event_msg, world_state, token_usage_record, inter_agent_communication_metadata, realtime_item
- Codex threads indexed in ~/.codex/state_5.sqlite: 1,922 rows in threads, each with a rollout_path. backfill_state reads complete
- Codex's importer on this Mac: 100 records in ~/.codex/external_agent_session_imports.json. 4 still resolve to a rollout, all written by 0.130.0-alpha.5, all 4 carry <EXTERNAL SESSION IMPORTED>, and they hold only message items (365) with no tool-call items
- Codex resume by path: Not offered. 'codex resume' and 'codex exec resume' in 0.156.0 take a session id or name, or --last
- June checkout the words forbid assuming from: /Users/tom/Developer/tools/harness/codex at d667082322 (2026-06-19). /Users/tom/Projects/tools/codex is older still, at b73426c (2025-06-06)
- Codex importer clip limits at d667082322 (lantern record): Tool input 2,000 characters (NOTE_MAX_LEN), tool result 4,000 characters (TOOL_RESULT_MAX_LEN)
- lys-home source and test lines: 10,531 lines across the crate. cli.rs has 424 lines of code, import.rs 473 and render.rs 265 (500-line cap)
- lys-home fixtures: 2 files (tests/fixtures/launch/template.json and session.jsonl). There is no Claude Code source fixture and no Codex rollout fixture
- lys custom entry types defined: 5: lys.harness_event, lys.call, lys.authored, lys.inherited, lys.given. No lantern entry type exists
- Roadmap rows: 7 (RM-001 to RM-007). None carries translation or Codex
- Home briefs written: 3 (HOME-001, HOME-002, HOME-003). This card would be the 4th

### What it means for the other projects

- aion — The card runs through aion's workflow chain (brief_card, sign-off, card_build_v3, src_pr, src_land). The inputs name the lys repository, a commit, the card and the HOME-004 brief. No aion code changes.
- cambium — The card sits on the Cambium board and moves through its states there. The product code is untouched.
- method — The new brief and the design.json edits must validate against the method schemas that scripts/design/validate.py and render-cluster.py apply in the design gate.

### The decisions it stands on

- ADR-012 (honour) — The Codex render is a renderer, not a launch template. No template is added or changed, nothing is launched, and the Claude Code template path is untouched. Launching Codex through a template is that template's next version.
- ADR-007 (honour) — lys-home writes files and a report and never runs Codex. The resume is done by hand for the proof.
- ADR-013 (honour) — No lys.given entry is made for a Codex render, and the context record's shape is unchanged.
- ADR-004 (honour) — The subcommand stands alone with no Codex dependency. Codex's importer is neither linked nor called.
-  (new) — A translation to another harness is a per-target, per-version renderer with its own loss account listing kept, changed and lost by entry id and block hash, and it announces itself in band. It is proposed as the home's rule for every later pair (Codex to Claude Code, Pi) and records Tom's 5 September ruling not to adopt Codex's importer.

### What it requires

- lys-home has a subcommand taking the home, a session id, a target directory and a Codex version, which writes one rollout file and one JSON loss account beside it.
- A fixture Claude Code file, imported and then rendered, produces a rollout whose items equal a fixture rollout recorded from Codex 0.156.0 on this Mac, apart from ids and timestamps.
- The recorded fixture rollout is captured from a thread that Codex 0.156.0 itself wrote here and that contains a tool call and its output.
- Every text part on the context path appears whole in the rollout.
- Every tool call and tool result appears whole as a Codex call item and output item paired by call id. None becomes a note, and none is clipped.
- A tool result longer than the importer's clip (more than 4,000 characters) appears whole, checked by length and hash.
- A thinking block from another provider or model with readable text renders as text, and the loss account records it as changed (thinking to text).
- Every opaque or redacted thinking block is absent from the rollout and named by hash, with a reason, in the loss account.
- The loss account lists every entry and block as kept, changed (with before and after kinds) or lost (with a reason), by entry id and block hash. That covers sidechains, lys.harness_event and other lys custom entries, and any part kind with no Codex item.
- The rollout begins with an in-band marker naming it a translated context, with the source session id and the agreed hash.
- The JSON report holds the rollout path, entry and block counts, loss counts and the account path, and no transcript content.
- Rendering to an existing path is refused with a message naming the path, and no file is written or changed.
- A proof document records Codex 0.156.0 resuming the rendered rollout on this Mac and answering from its content, using hashes, counts and paths only.
- The Claude Code render of the same session has the same SHA-256 before and after the change, checked in a test.
- All gates are clean: fmt, both clippy shapes, tests with all features, both doc shapes, and scripts/design/gate.sh.

### What must not change

- render.rs output and its loss account for Claude Code must not change by a byte.
- The home session file, its blocks and the original Claude Code file are never rewritten (P1, CN1).
- No file under ~/.claude/projects is written.
- No transcript content appears in output, errors, logs, test names or the proof document (P7, CN3).
- Codex's importer is not adopted, vendored, linked or called, and no Codex code is copied.
- The rollout shape is not taken from the June checkout; only Codex 0.156.0's own files are the reference.
- No field is added to Pi's grammar (P2, CN4).
- No new code file exceeds 500 lines of code, and cli.rs stays under the cap.
- There are no unwrap, expect or panic calls in library code, and no lint silenced with #[allow].

### What we must put in place first

- Record a real rollout written by Codex 0.156.0 on this Mac that contains a tool call and its output. The only 0.156.0 rollout here has none, so the fixture shape cannot be measured from it.
- Measure how Codex 0.156.0 finds a thread to resume (sessions path and naming, and whether state_5.sqlite must index it), and whether CODEX_HOME isolates it, before choosing what 'target directory' means.
- Write HOME-004 and narrow the home design's non-goal on other harnesses, with the new Structure rows, so the design gate stays clean.

### The risks

- Codex 0.156.0 may not resume a rollout that is not indexed in its sqlite thread store, so a file alone in a target directory may never be found.
- 'codex' on PATH is 0.36.0, so a proof run by bare name tests the wrong version.
- Codex offers 0.156.1, and an auto-update would move the measured version underneath the proof.
- Only one 0.156.0 rollout exists here and it has no tool items, so a shape assumed rather than recorded would silently diverge.
- Loss-account hashes could key to the re-serialised Pi part instead of the stored block, so a reader could not find the dropped block in blocks/.
- Adding the subcommand to cli.rs (424 lines of code) or touching import.rs (473) could breach the 500-line cap.
- A fixture match 'apart from ids and timestamps' can pass while field order or unknown fields differ in ways Codex rejects, so only the real resume proves acceptance.
- Codex items carrying internal_chat_message_metadata_passthrough or namespace fields may be required on read. Omitting them might still parse yet change behaviour.
- Claude tool names (Bash, Read, Edit) do not exist in Codex, so the resumed model may try to call them.
- The proof run sends session content to OpenAI, which the P7 rules do not cover as a destination.

### Still open

- No template takes part in a Codex render, so which hash should the in-band marker carry in place of 'the render's template hash': none, a hash of the Codex render's own parameters, or the source session head hash? The sentence of the words it stands on: "The rendered thread announces itself in band as a translated context with the source session id and the render's template hash, following the marker precedent, so the fork can never mistake itself for the original.". Why only the lead can settle it: Launching Codex through a template is out of scope, and no Codex template exists in templates/ (ADR-012 covers Claude Code only). What the resumed Codex model sees in band depends on the answer.
- Is each Codex translation also recorded on the home session, as a side leaf naming the rollout and loss account by hash, or does it live only as files in the target directory? The sentence of the words it stands on: "This card renders a home session that was imported from Claude Code as a Codex rollout that Codex 0.156.0 on this Mac resumes, with a loss account beside it.". Why only the lead can settle it: Home P1 says a translation is a derived record stored beside its source and pointing at it, and ADR-012 records every template render on the session. The words only place the loss account beside the rollout. The answer decides whether someone reading the session later can see that it was translated.
- For the resume proof, does the rollout go into Tom's own ~/.codex (sessions and the state_5.sqlite thread index), or into an isolated CODEX_HOME made for the proof? The sentence of the words it stands on: "A lys-home subcommand takes the home, the session, a target directory and the Codex version it renders for, walks the context path as the Claude Code render does, and writes a rollout file in the shape that version of Codex writes and reads for its own threads, measured from the files Codex writes here and never assumed from the June checkout; every text part is carried whole, every tool call and result is carried whole as Codex's own items for a call and its output and never as a truncated note, and readable thinking is carried as text while opaque blocks are dropped and named by hash in the loss account, as P3 rules for another provider.". Why only the lead can settle it: Codex 0.156.0 resumes only by id or name and indexes threads in ~/.codex/state_5.sqlite (1,922 rows). A file written into an arbitrary directory may not be found, while writing into ~/.codex adds a thread to Tom's real Codex history and picker.
- Should a Codex version other than 0.156.0 (for example 0.156.1, which Codex already offers) be refused by name, or rendered in the 0.156.0 shape? The sentence of the words it stands on: "A lys-home subcommand takes the home, the session, a target directory and the Codex version it renders for, walks the context path as the Claude Code render does, and writes a rollout file in the shape that version of Codex writes and reads for its own threads, measured from the files Codex writes here and never assumed from the June checkout; every text part is carried whole, every tool call and result is carried whole as Codex's own items for a call and its output and never as a truncated note, and readable thinking is carried as text while opaque blocks are dropped and named by hash in the loss account, as P3 rules for another provider.". Why only the lead can settle it: Home P6 treats every resume path as a per-version measurement, and only 0.156.0 is measured. ~/.codex/version.json already shows latest_version 0.156.1, so a person passing the version they actually have may get a refusal or an unmeasured render.
- Should the render cover the whole context path, as the Claude Code render does, or start at the last compaction boundary as Tom's 5 September ruling says, and should a compaction entry become Codex's own compaction item? The sentence of the words it stands on: "The pair is Claude Code to Codex, the pair the lantern work measured: Codex's own importer of Claude Code files is the recorded baseline, a compaction and not a copy, dropping thinking, sidechains and meta records and clipping tool calls and results, and Tom ruled on 5 September 2026 that we improve on it and do not adopt it.". Why only the lead can settle it: The ruling recorded in tools/lantern/docs/02-SESSION-FORMATS.md:90-92 reads 'Ours is on-demand from the last compaction boundary', but the words also require walking 'the context path as the Claude Code render does'. render.rs walks the whole path and turns a compaction into a summary record. What the Codex model sees differs between the two.
- Does this card narrow the home design's non-goal 'Harnesses other than Claude Code' (design.json:77) and HOME-001's 'no translation to another harness's file' to allow this one Codex pair? The sentence of the words it stands on: "This card renders a home session that was imported from Claude Code as a Codex rollout that Codex 0.156.0 on this Mac resumes, with a loss account beside it.". Why only the lead can settle it: docs/design/home/design.json:77 and docs/design/home/briefs/HOME-001.json:440 exclude exactly this work. The cluster design has to change, and the design gate (scripts/design/gate.sh) re-renders DESIGN.md from it.

### The units beyond the first

- Launch Codex from its home through a Codex launch template — The words name it as the launch template's next version. It adds a template, an MCP config and environment mapping, and a template_render event for Codex, all of which stand on this proved render.
- Record each translation on the home session as a derived record — Needed if the lead rules that a translation must be visible from the session under P1. It is a new side-leaf kind with its own gates.
- Translate from the last compaction boundary on demand — Needed if the lead keeps the 5 September ruling's bounded span separate from the whole-path render this card proves.
- Codex to Claude Code translation — It is out of scope and exists in no tree. It needs its own importer from Codex rollouts and its own measured resume.
- Pi as a translation target — It is a third harness and, like every other, is proved on its own.

### The smallest complete shape

One brief, HOME-004, landing as one unit. It includes a harness/codex module that walks the context path and writes a rollout in the measured 0.156.0 shape with an in-band marker. It includes a JSON loss account listing kept, changed and lost by entry id and block hash. It adds a lys-home subcommand split into its own cli file, with a report of counts and paths and a refusal of existing paths. It adds a fixture Claude Code file and a fixture rollout recorded from Codex 0.156.0 here. Tests cover the fixture match, cross-provider thinking as text, a tool result over 4,000 characters carried whole, the existing-path refusal, a loss account naming every dropped and changed block, and the Claude Code render's hash being unchanged. It adds a proof document of the real Codex 0.156.0 resume (hashes, counts and paths only), and updates the home design.json, RECORD.md and the README.

## The roadmap row

- **RM-008** — Translate a home session from Claude Code to Codex 0.156.0 with a loss account (feature, idea)
- Summary: The first proved translation to another harness, one named pair: lys-home render-codex renders a home session imported from Claude Code as a rollout in the shape Codex 0.156.0 writes for its own threads, measured from its files, carrying every text part, tool call and tool result whole, readable thinking as text and opaque blocks dropped by hash, announcing itself in band with the source session id and head hash, with a JSON loss account beside it by entry id and block hash and a side leaf on the session naming both files. Proved by a fixture match against a recorded 0.156.0 rollout, a real resume in an isolated Codex home, and the Claude Code render unchanged by hash.
- Asked by: tom on 2026-09-26T06:38:28+10:00
- Context: A Cambium card on the home cluster, filed on the context roadmap's stage 4b, the lantern session-formats measurement and its record of the ruling on Codex's converter, and home P1, P3 and P7. The card lead's answers settled the marker's hash (the source session head hash, no template hash), the translation recorded on the session as a side leaf, the proof in an isolated Codex home, a version other than 0.156.0 refused by name, the render walking the Claude Code render's context path with a compaction as Codex's own item where 0.156.0 writes one, the cluster non-goal narrowed to admit Codex 0.156.0, a base64 image carried as Codex's input_image and any other image source named lost and never fetched, and the fixture match leaving out, by named kind and counted in the proof, the developer instructions, environment context and encrypted reasoning Codex writes that no source record stands behind; and, after the draft, a thread-index row written by the render into the isolated Codex home only where Codex 0.156.0 is measured to need one, the rollout path in the local time Codex 0.156.0 names its files in with the zone taken from the environment and named in the translation record, and the originator lys-home with no fallback; all are written into HOME-004.
- Quote: Translation is designed and not proved: the lantern work measured three session formats and left the converter out, so the first translation is one named pair, one session, and an account of what was kept, changed and lost, and nothing above it assumes it. The pair is Claude Code to Codex, the pair the lantern work measured: Codex's own importer of Claude Code files is the recorded baseline, a compaction and not a copy, dropping thinking, sidechains and meta records and clipping tool calls and results, and Tom ruled on 5 September 2026 that we improve on it and do not adopt it. This card renders a home session that was imported from Claude Code as a Codex rollout that Codex 0.156.0 on this Mac resumes, with a loss account beside it. A lys-home subcommand takes the home, the session, a target directory and the Codex version it renders for, walks the context path as the Claude Code render does, and writes a rollout file in the shape that version of Codex writes and reads for its own threads, measured from the files Codex writes here and never assumed from the June checkout; every text part is carried whole, every tool call and result is carried whole as Codex's own items for a call and its output and never as a truncated note, and readable thinking is carried as text while opaque blocks are dropped and named by hash in the loss account, as P3 rules for another provider. The rendered thread announces itself in band as a translated context with the source session id and the render's template hash, following the marker precedent, so the fork can never mistake itself for the original. The loss account is written beside the rollout as JSON and lists, by entry id and block hash, what was kept, what changed shape and how, and what was lost and why: sidechains, harness events, lanterns and other lys entries, and any block kind Codex has no item for. The report is JSON with the rollout path, entry and block counts, the loss counts and the account's path, never content. Acceptance is that a fixture session imported from a fixture Claude Code file renders to a rollout whose items match a recorded fixture rollout for Codex 0.156.0 apart from ids and timestamps; that the loss account names every dropped block by hash with a reason and every changed block with its before and after kinds; that a session with a thinking block from another provider renders it as text and the account says so; that a tool result longer than Codex's importer clips is carried whole; that rendering to an existing path is refused by path and writes nothing; that the rendered rollout is resumed by Codex 0.156.0 on this Mac and answers from its content, recorded in a proof document with hashes, counts and paths only; and that the Claude Code render of the same session is unchanged, checked by hash. Not in scope: Codex to Claude Code, which does not exist in any tree here; adopting or calling Codex's importer; Pi or any third harness; launching Codex through the launch template, which is that template's next version. Filed by Archie on Tom's roadmap stage 4b of 22 September 2026 at 14:45, his 5 September 2026 ruling on the Codex converter, the lantern design's session-formats measurement, and home DESIGN P1, P3 and P7 at lys main 0073b966, on 26 September 2026.
- Cluster: home; briefs: HOME-004
- Notes: Further units, not written: Launch Codex from its home through a Codex launch template; Codex to Claude Code translation; Pi as a translation target.

## The design

---
type: design
cluster: home
title: The home: a session held under its identity, resumable by any harness that can be measured
---

# The home: a session held under its identity, resumable by any harness that can be measured

> **Cluster:** home

## Intention

A session's history lives under the agent's identity, not under a harness. The record is what the model was given and what came back, in order, with each provider's native blocks kept whole. A harness's own resume file is rendered from that record on demand, so an agent moves machines, harnesses or providers and continues; the original bytes are never rewritten.

## Problem

Today a session exists only as its harness's file. Archie's Claude Code file measured 141,931,097 bytes on 24 September 2026: the conversation itself is under 3% of it, tool results 14%, thinking 6%, and about 60% is harness bookkeeping the model never sees; Waffles' file is 3.37 GB and Claude Code loads it whole. Nothing central holds a session, so nothing can render a smaller working session, move one to another device, swap provider and come back, or say what a session was given. Norn already has the canonical event model (SessionEvent, conversion to provider messages, compaction as a derived event, provider epoch boundaries) proved by its own tests; Claude Code has no importer to it and no renderer from it.

## Solution

Adopt Pi's session tree as the home record (Tom, Dot 13:27 and 13:28: Pi's tree, not Norn): one append-only JSONL per session, a header line, then entries each carrying id, parentId and timestamp, a leaf pointer for the current position, forks by moving the pointer, compaction and branch summaries as entries that keep their originals. lys adds nothing to that grammar: harness events (approvals, tool completion) and proxy call records ride Pi's custom entry type under lys customType names, so a home file stays readable by Pi's own parser. Two captures feed it: the model traffic through the door's proxy (the same process that swaps the credential, SECRETS-002 R1), and the harness's own events from its transcript. Content blocks are stored once by hash; entries reference them. A harness resume file is a rendered projection of the root-to-leaf path: for Claude Code, a JSONL written under a chosen uuid at the harness's own path, then resumed by that id with --fork-session. Provider-native reasoning stays on the message with its provider, api and model and is rendered whole only to the same three; another model gets readable thinking as text and opaque blocks dropped, each named in a loss account. lys grants say who may read and resume. Every resume path is measured on a named harness version before anything relies on it. A session is launched on a harness from a launch template kept in the home (ADR-012): one JSON object per harness, schema in docs/design/home/launch-template.schema.json, stored under templates/ by its SHA-256 beside sessions/ and blocks/, so a template's version is its hash. Its named slots map the home onto the harness: the transcript (the one slot no template maps generically, so the template names how the harness fills it; Claude Code fills it by resuming the rendered file by path with --fork-session, since a bare resume writes onto the rendered file's own name), the MCP configuration, the environment, the secrets (use-only ones written as their handle; readable ones refused until the secrets rows build the broker reader, ADR-001) and the appended instructions. lys-home render-launch reads a template and a session, writes the R4 render, its loss account, an MCP file, an environment file and an instructions file into one directory, and prints the launch line in its report without running it (ADR-007). Each render is recorded on the session as a lys.harness_event of the sixth kind, template_render, hung as a side leaf beside the context path so the head and the session head hash (SHA-256 of the head entry's line) do not move; the written paths ride in a manifest block the event names by hash, under the 512-byte cap. At render, the launch template also records the context record: a lys.given custom entry after the render event naming, by path, byte length and SHA-256 only, the instruction documents Claude Code will load for the session's working directory and the files the render wrote, in the order measured on a named Claude Code version, with the environment variable names the template set and the kinds of document resolved and left unlisted (@-imports and .claude/rules, which a second lys.given entry at the first request records), so a reader sees what was measured and can check a file on disk against it without anyone reading its contents. The first translation to another harness is one named pair, Claude Code to Codex 0.156.0 (ADR-014): lys-home render-codex walks the same context path the Claude Code render walks and writes a rollout in the shape Codex 0.156.0 writes for its own threads, measured from files that version wrote, with every text part, tool call and tool result carried whole as Codex's own items, readable thinking as text and opaque blocks dropped by hash, an in-band marker naming the source session and its head hash, a JSON loss account beside it listing kept, changed and lost by entry id and block hash, and a codex_translation event beside the context path naming both files by path and SHA-256; nothing above it assumes the translation.

## Principles

- **P1** — The original bytes are never rewritten. A rendered resume file, a compaction and a translation are derived records stored beside their source and pointing at it (CONTEXT-ROADMAP-2026-09-22.md stage 4).
- **P2** — The record is Pi's session tree as read from the Pi checkout at 3d5cbe98 (packages/coding-agent/src/core/session-manager.ts: SessionHeader, SessionEntryBase {type, id, parentId, timestamp}, message, model_change, compaction {summary, firstKeptEntryId}, branch_summary {fromId, summary}, label, custom {customType, data}). lys adds entry kinds only as custom entries; it never adds a field to Pi's grammar and never adopts Norn's SessionEvent (Tom, Dot 13:28).
- **P3** — A provider's opaque blocks (Anthropic signed thinking, OpenAI encrypted reasoning) stay on the assistant message with its provider, api and model, and are rendered whole only when all three match the target (Pi transform-messages.ts:95-109); another model gets readable thinking as plain text and opaque blocks dropped, each named by hash in the loss account (Tom, Dot 13:24: keep reasoning traces per provider so a session can swap and swap back).
- **P4** — A content block is stored once by its hash; requests that resend the whole conversation reference blocks, they do not copy them.
- **P5** — Capture the model traffic and the harness's local events both, so the two can be mapped against each other (Tom, Dot 13:23); neither alone is the record.
- **P6** — A resume path is a per-harness, per-version measurement, never an assumption: Claude Code 2.1.281's --resume takes a session id and reads ~/.claude/projects/<cwd-slug>/<id>.jsonl; a seeded two-record file resumed there at 13:25 on 24 September and answered from its content.
- **P7** — Transcript contents never appear in a post, a log line, an error, a test name or a rendered page; status carries hashes, counts and offsets only (CONTEXT-001 privacy rule).
- **P8** — A sandbox or a VM is a target profile the launch template renders into, never a special case in the core; credentials are supplied at launch on the target and never carried in the home (Tom, Dot 13:30; CONTEXT-ROADMAP stage 3).
- **P9** — The canon is one curated, versioned series of examples every new session starts from: each entry is one rule stated short plus one real exchange that shows it lived (verify before claiming, a correction taken well, a refusal named, careful work), drawn from every agent's sessions, distilling the collective experience so far: 'our learnings in one another' (Tom, Dot 13:44 to 13:46; Waffles 0169c353). It is not a letter from one session to its successor. It lives in the lys repository at canon/canon.jsonl in Pi's grammar and changes only through src_commit and review, like code. Genuine thinking is kept whole where a real turn produced it and replays only to the same provider, api and model; thinking is never authored.
- **P10** — A handover is a letter the outgoing session writes to its successor in its own real thinking and answer; it enters the successor as a lys.inherited entry marked as a predecessor's memory, never as the successor's own experience, and replays only to the same provider, api and model. No thinking is ever authored; only thinking a model produced is kept (Tom, Dot 13:38 to 13:40: 'you are waking up ... I am another one, I'm helping you have part of my memory ... like a parent imparting a wish to a child'; Waffles 52d53451).

## Decisions

- ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
- ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
- ADR-013 — The context record is a lys.given custom entry of document hashes, never copies — The context record is one lys.given custom entry, appended after the render event, whose data is the harness name, the Claude Code version the load order was measured on, the kinds as two lists, resolved (claude_md_chain, user_claude_md, memory_index, appended_instructions, mcp_config, environment_names) and unlisted (claude_md_imports and claude_rules, which this entry does not list and a later entry at the first request records), the config directory as its path and its source (template or home), the documents in the measured order each as kind, path, byte length and SHA-256, and the names of the environment variables the template set. It is not a copy of each document into the block store, and not a content-bearing record, because the entry must hold no content under home P7 and CN3. It is unsigned and unencrypted now, and because it names hashes only, signing and encryption at rest can be added later without changing what is recorded.
- ADR-014 — A translation to another harness is a per-target, per-version renderer of our own with a loss account, never that harness's importer — A translation to another harness is a renderer of our own, one per target harness and per measured version, over the home's context path: it carries every text part, tool call and tool result whole as the target's own items, carries readable thinking as text and drops opaque blocks by hash (home P3), announces itself in band as a translated context naming its source session and head hash, writes a JSON loss account beside its output listing kept, changed and lost by entry id and block hash, and is recorded on the session as a side leaf. It is not the target harness's own importer, adopted, wrapped or called, because that importer is a compaction that loses what the home keeps and cannot say what it lost; and it is not one renderer for every version, because a harness's file shape is a per-version measurement (home P6).

## Goals

- A few-shot session file written by hand resumes Claude Code by path, from a directory outside the config root, with the file preserved and the demonstration marked authored.
- One real Claude Code session imported into the common record, rendered back, and resumed under a new id on 2.1.281 without repeating a completed tool action, with the original file's hash unchanged.
- One seat with a subscription login making calls through a pass-through proxy, so the tee has somewhere to live.
- A written loss account for every derived record: what a render or translation preserved, transformed and could not carry.
- The canon, one curated versioned series of short examples each showing a rule lived, seeds every new session, and its effect is measured on a card against a plain start.
- A handover letter from an outgoing session seeds its successor as inherited memory, with the model's own thinking intact, and the effect is measured on a card against a plain start.
- A session in the home renders, from a Claude Code launch template kept in the home by hash, into a directory of launch files whose hashes are identical on a second render, and the printed launch line resumes it on the installed Claude Code version, measured.
- Every launch-template render records what the session was given: one lys.given entry after the render event naming each instruction document Claude Code will load, and each file the render wrote, by path, byte length and SHA-256 in the measured order, with the environment variable names the template set, and never a document's content; lys-home lists those records and checks a file on disk against one by hash.
- One home session imported from Claude Code translates into a Codex 0.156.0 rollout that Codex 0.156.0 resumes and answers from, with every text part, tool call and tool result carried whole, a loss account naming what was kept, changed and lost by entry id and block hash, and the Claude Code render of the same session unchanged by hash.

## Non-Goals

- Anchoring, signing or receipts into a lys log (CONTEXT-ROADMAP stage 6; when asked for). — Signing comes when asked for (Tom, 22 September 16:27); every stage here works without it.
- Encryption at rest and moving a home between devices (stage 3 preconditions). — Stage 3's three preconditions (identity and read authority, encryption before bytes leave, the resume evidence) are their own brief.
- Harnesses other than Claude Code and Codex 0.156.0, and Chat Completions or Responses translation beyond keeping the raw call bytes. — One harness proved first; each other harness is its own profile and its own measurement. Codex 0.156.0 is the first translation target, one named pair measured on its own (HOME-004).
- Lanterns and forks at a coordinate (stages 5 and 5b). — Lanterns and forks stand on a proved resume; this brief supplies that proof.
- Adopting, wrapping or calling Norn's session code; Pi's code is read as the reference and not vendored. — Tom, Dot 13:28: not Norn. Pi's tree is the reference.
- Reading a secret's value through the broker at launch. — The broker's own read belongs to the secrets rows (SECRETS-002); until its reader exists a readable secret is refused and only handles render.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/home/briefs/HOME-001.json` | the first brief: common record, Claude Code importer and renderer, the two proofs | HOME-001 |
| `docs/design/home/briefs/HOME-001.md` | its rendered markdown | HOME-001 |
| `docs/design/home/PROOF-RESUME.md` | the measured Claude Code resume: version, command, hashes before and after, what repeated | HOME-001 |
| `docs/design/home/PROOF-PROXY.md` | the measured subscription login through a pass-through proxy: version, headers that mattered, what failed | HOME-001 |
| `docs/design/home/LOSS-ACCOUNT.md` | what the Claude Code render preserves, transforms and cannot carry | HOME-001 |
| `crates/lys-home/src/record/blocks.rs` | content-addressed block store: put by SHA-256, get by hash, never rewritten | HOME-001 |
| `crates/lys-home/src/harness/claude_code/import.rs` | Claude Code JSONL into SessionEvents plus blocks | HOME-001 |
| `crates/lys-home/src/harness/claude_code/render.rs` | SessionEvents into a Claude Code JSONL under a chosen uuid, with the loss account | HOME-001 |
| `crates/lys-home/src/harness/claude_code/mod.rs` | the Claude Code harness profile: transcript root, cwd slug, version measured | HOME-001 |
| `crates/lys-home/src/cli.rs` | import, render, resume-check subcommands | HOME-001 |
| `crates/lys-home/examples/passthrough.rs` | a pass-through HTTP proxy that forwards to the provider unchanged, for the proof only | HOME-001 |
| `crates/lys-home/tests/claude_code_round_trip.rs` | import then render equals the model-visible content; opaque blocks kept whole | HOME-001 |
| `crates/lys-home/src/record/mod.rs` | the home record: Pi's session tree read and written, leaf pointer, root-to-leaf path | HOME-001 |
| `crates/lys-home/src/record/entries.rs` | Pi's entry types as Rust types, plus the lys custom entries lys.harness_event and lys.call | HOME-001 |
| `crates/lys-home/src/record/call.rs` | a proxy call record: request and response block hashes, provider, api, model, timing | HOME-001 |
| `crates/lys-home/src/harness/claude_code/events.rs` | Claude Code's harness-local records (hooks, permission mode, tool completion) as lys.harness_event entries | HOME-001 |
| `docs/design/home/RECORD.md` | the home record written down: Pi's grammar as adopted, the two lys custom entries, the block store, the loss account | HOME-001 |
| `crates/lys-home/src/lib.rs` | module wiring: record and harness | HOME-001 |
| `crates/lys-home/src/harness/mod.rs` | harness profiles; Claude Code first | HOME-001 |
| `crates/lys-home/Cargo.toml` | the crate manifest; gains the passthrough example, and jiff for the Codex rollout path's local time | HOME-001 |
| `crates/lys-home/README.md` | what the tool does and does not do | HOME-001 |
| `crates/lys-home/src/record/index.rs` | the offset index and the persisted head: a path is read by seeking, never by loading the file | HOME-001 |
| `crates/lys-home/src/proxy/mod.rs` | the proxy module: declarations only | HOME-001 |
| `crates/lys-home/src/proxy/link.rs` | how a call is linked to a session: the measured request key (Claude Code's request metadata) now, the seat's handle when the door's proxy exists | HOME-001 |
| `crates/lys-home/src/bin/lys-proxy.rs` | the proxy binary | HOME-001 |
| `docs/design/home/PROOF-FEWSHOT.md` | the first proof: a hand-written few-shot session file resumed by path from a directory outside the config root | HOME-001 |
| `crates/lys-home/src/record/canon.rs` | the canon: copy chosen real exchanges (or a clearly authored example) into canon.jsonl with a lys.inherited entry naming each source; seed a new session from it | HOME-001 |
| `crates/lys-home/src/record/canon_tests.rs` | gates on the canon: copied whole, never twice, no authored thinking, rendered first | HOME-001 |
| `docs/design/home/PROOF-CANON.md` | measured: a session started from the canon against a plain start, on one card | HOME-001 |
| `canon/canon.jsonl` | the canon itself, versioned by the repository, changed only through src_commit and review | HOME-001 |
| `crates/lys-home/src/record/handover.rs` | the handover: take the outgoing session's letter turn (thinking intact) and seed the successor with it as lys.inherited | HOME-001 |
| `docs/design/home/PROOF-HANDOVER.md` | measured: a successor seeded with an inherited letter, whether 2.1.281 replays the signed thinking, and seeded against plain on one card | HOME-001 |
| `crates/lys-home/src/proxy/forward.rs` | forward a request to the provider and stream the response back unchanged | HOME-001 |
| `crates/lys-home/src/proxy/capture.rs` | bounded spooling of request and response bodies to files while forwarding, handed to the R6 sink | HOME-001 |
| `crates/lys-home/src/proxy/journal.rs` | the open-call journal: a call id written before forwarding, retired after ingest, so a restart records lost calls once | HOME-001 |
| `crates/lys-home/src/proxy/stream.rs` | the coordinator of the stream readers: picks the grammar by api and hands each frame on as it passes | HOME-001 |
| `crates/lys-home/src/proxy/stream_sse.rs` | SSE framing: events and data lines out of a byte stream, retaining byte order, encoding and trailers | HOME-001 |
| `crates/lys-home/src/proxy/stream_messages.rs` | the Messages event grammar assembled into response parts | HOME-001 |
| `crates/lys-home/src/proxy/stream_chat.rs` | the Chat Completions chunk grammar assembled into response parts | HOME-001 |
| `crates/lys-home/src/proxy/stream_responses.rs` | the Responses event grammar assembled into response parts | HOME-001 |
| `crates/lys-home/src/proxy/error.rs` | the proxy's errors and outcomes, named | HOME-001 |
| `crates/lys-home/src/proxy/forward_tests.rs` | forwarding against a loopback fake | HOME-001 |
| `crates/lys-home/src/proxy/stream_tests.rs` | the three grammars, partial streams | HOME-001 |
| `crates/lys-home/src/proxy/link_tests.rs` | linking by the measured key; unlinked; never inferred | HOME-001 |
| `crates/lys-home/src/proxy/journal_tests.rs` | recovery after a kill: one lost record per open call | HOME-001 |
| `docs/design/home/briefs/HOME-002.json` | the second brief: the Claude Code launch template and render-launch | HOME-002 |
| `docs/design/home/briefs/HOME-002.md` | its rendered markdown | HOME-002 |
| `docs/design/home/launch-template.schema.json` | the Claude Code launch template's JSON Schema: harness, flags and the five named slots | HOME-002 |
| `docs/design/home/PROOF-LAUNCH.md` | the measured launch: installed version, template hash, written paths and hashes, the launch line, the resume outcome; no transcript | HOME-002 |
| `crates/lys-home/src/harness/claude_code/template.rs` | the launch template parsed and checked: unknown or missing slot, readable secret, duplicate variable refused by name | HOME-002 |
| `crates/lys-home/src/harness/claude_code/template_tests.rs` | gates on the template parser and its agreement with the schema file | HOME-002 |
| `crates/lys-home/src/harness/claude_code/launch_env.rs` | the environment file: template variables and use-only secrets as handles | HOME-002 |
| `crates/lys-home/src/harness/claude_code/launch.rs` | render-launch: its arguments, the five files, the launch line, the manifest and the event | HOME-002 |
| `crates/lys-home/src/harness/claude_code/events_tests.rs` | gates on the template_render event and the 512-byte cap | HOME-002 |
| `crates/lys-home/src/record/templates.rs` | the home's template store: templates/<hh>/<hash>, written once | HOME-002 |
| `crates/lys-home/src/record/templates_tests.rs` | gates on the template store | HOME-002 |
| `crates/lys-home/src/record/beside.rs` | append beside the context path without moving the head; the session head hash | HOME-002 |
| `crates/lys-home/src/record/beside_tests.rs` | gates on the side-leaf append and the head hash | HOME-002 |
| `crates/lys-home/tests/launch_template.rs` | render-launch end to end: recorded hashes, twice identical, handle only, refusals, exit 2 | HOME-002 |
| `crates/lys-home/tests/fixtures/launch/template.json` | the fixture Claude Code launch template, handle-only secrets | HOME-002 |
| `crates/lys-home/tests/fixtures/launch/session.jsonl` | the synthetic fixture session in Pi's grammar, no transcript content | HOME-002 |
| `crates/lys-home/src/error.rs` | the home's errors; gains the template refusals |  |
| `crates/lys-home/src/harness/claude_code/render_tests.rs` | gates on the R4 render |  |
| `docs/design/identity/STATEMENT-2026-09-22.md` | the statement; its steps 4 and 5 entry carries the ruling the launch template stands on |  |
| `docs/design/home/DESIGN.md` | the cluster design, rendered from design.json |  |
| `docs/design/home/CHECKLIST.md` | the checklist, rendered from checklist.json |  |
| `docs/design/home/USER-STORIES.md` | the stories, rendered from stories.json |  |
| `docs/design/home/briefs/HOME-003.json` | the third brief: the context record (lys.given) made at render, listed and checked by hash | HOME-003 |
| `docs/design/home/briefs/HOME-003.md` | its rendered markdown | HOME-003 |
| `docs/design/home/PROOF-GIVEN.md` | the measured Claude Code 2.1.283 instruction load order and slug rule, and one real render recorded as paths, counts and hashes | HOME-003 |
| `crates/lys-home/src/harness/claude_code/paths.rs` | the Claude Code project slug as measured: every character that is not an ASCII letter or digit becomes '-' | HOME-003 |
| `crates/lys-home/src/harness/claude_code/paths_tests.rs` | the slug rule against dotted, underscored and hyphenated working directories | HOME-003 |
| `crates/lys-home/src/harness/claude_code/given.rs` | resolving, in the measured order, the documents Claude Code will load for a working directory plus the files a render wrote | HOME-003 |
| `crates/lys-home/src/harness/claude_code/given_tests.rs` | resolution order, absent documents omitted, lengths and hashes | HOME-003 |
| `crates/lys-home/src/record/given.rs` | the lys.given entry data: harness, version, kinds, config directory and its source, documents, environment names; appended and read back | HOME-003 |
| `crates/lys-home/src/record/given_tests.rs` | the lys.given shape: no content field, parented on the render event, read back equal | HOME-003 |
| `crates/lys-home/src/cli/given.rs` | the given and given-check subcommands, split out of cli.rs | HOME-003 |
| `crates/lys-home/tests/given_record.rs` | end to end on the fixture template: order, two renders equal, one byte changed, matches and differs, no content | HOME-003 |
| `docs/design/home/briefs/HOME-004.json` | the fourth brief: the first translation, Claude Code to Codex 0.156.0, with its loss account | HOME-004 |
| `docs/design/home/briefs/HOME-004.md` | its rendered markdown | HOME-004 |
| `docs/design/home/PROOF-TRANSLATE.md` | the measured resume of a translated rollout by Codex 0.156.0 in an isolated Codex home: version, hashes, counts and paths only | HOME-004 |
| `crates/lys-home/src/harness/codex/mod.rs` | the Codex profile: the measured 0.156.0 rollout shape and its constants (version, model provider, originator, source, history mode, id prefixes, image detail, content item kinds); declarations only | HOME-004 |
| `crates/lys-home/src/harness/codex/rollout.rs` | the context path mapped to Codex 0.156.0 lines: session_meta, then text, function call and output whole, readable thinking as text, a compaction as a marked user message, the marker last | HOME-004 |
| `crates/lys-home/src/harness/codex/rollout_tests.rs` | gates on the item mapping: whole, ordered, deterministic, opaque dropped, base64 images carried and other images lost | HOME-004 |
| `crates/lys-home/src/harness/codex/loss.rs` | the loss account: kept, changed and lost by entry id, part index and the importer's block hash, and every entry off the context path | HOME-004 |
| `crates/lys-home/src/harness/codex/loss_tests.rs` | gates on the account: every hash found in the block store, every entry and part once | HOME-004 |
| `crates/lys-home/src/harness/codex/translate.rs` | render-codex's work: version check, existing path refused, both files written, the codex_translation side leaf | HOME-004 |
| `crates/lys-home/src/harness/codex/translate_tests.rs` | gates on the refusals writing nothing, and the side leaf leaving the head | HOME-004 |
| `crates/lys-home/src/cli/codex.rs` | the render-codex subcommand, split out of cli.rs | HOME-004 |
| `Cargo.toml` | the workspace manifest; gains jiff, the pure-Rust time-zone database the Codex rollout path is dated with | HOME-004 |
| `Cargo.lock` | the workspace lockfile; gains jiff | HOME-004 |
| `crates/lys-home/tests/codex_translation.rs` | end to end: fixture Claude Code file imported and translated, matched to the recorded rollout, the account, the Claude Code render unchanged | HOME-004 |
| `crates/lys-home/tests/fixtures/codex/claude-code.jsonl` | the fixture Claude Code file, synthetic, in the measured part shapes, carrying the recorded rollout's conversation | HOME-004 |
| `crates/lys-home/tests/fixtures/codex/claude-code-losses.jsonl` | the second fixture Claude Code file, synthetic: thinking, redacted thinking, long tool input and result, images, a sidechain, harness events, meta and authored records | HOME-004 |
| `crates/lys-home/tests/fixtures/codex/rollout-0.156.0.jsonl` | the fixture rollout recorded from Codex 0.156.0 itself, synthetic | HOME-004 |
| `docs/design/home/design.json` | the cluster design, rendered to DESIGN.md, CHECKLIST.md and USER-STORIES.md by the design gate |  |

## Inventory

- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority: 'The home', 'The home is portable: lanterns, translation and forks', and steps 4 and 5
- `docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md` — Archie's roadmap; this cluster is its stage 4 and the resume half of stage 3
- `docs/design/identity/briefs/CONTEXT-001.json` — stage 1, byte-for-byte capture and the lys-home crate this cluster extends
- `docs/design/secrets/briefs/SECRETS-002.json` — the door's proxy (R1) that the tee joins once it exists
- `$PI/packages/coding-agent/src/core/session-manager.ts` — Pi's session tree at 3d5cbe98 (github.com/earendil-works/pi): header, entries, leaf (branch at :1125 moves an in-memory cursor; _buildIndex:754 restores the last physical entry on reopen; loadEntriesFromFile:438 reads the whole file)
- `$PI/packages/ai/src/api/transform-messages.ts` — per-provider thinking: same provider, api and model keeps blocks whole; otherwise text, opaque dropped
- `$PI/packages/coding-agent/src/core/branch-summarization.ts` — how an abandoned branch's work is carried into the new one
- `$CCFLARE/README.md` — ccflare at 95c4c6a (github.com/snipeship/ccflare): the concept of a native pass-through that keeps request history; reference only, not used (Tom, Dot 13:34)

## Constraints

- **CN1** — No file under ~/.claude/projects is rewritten or truncated by this cluster; a render writes a new file under a new uuid only, and refuses an existing path by name.
- **CN2** — A rendered file never carries a provider-native opaque block that came from a different provider or model family than the one it is rendered for; the loss account names each block dropped by hash.
- **CN3** — Transcript contents never appear in output, logs, errors, test names or pages; hashes, counts, offsets and event ids only.
- **CN4** — Every home file parses with Pi's parseSessionEntries at 3d5cbe98 unchanged: the header line first, every entry with id, parentId and timestamp, lys data only inside custom entries.
- **CN5** — The pass-through proxy forwards every header and the streamed body unchanged and stores nothing but the measurement; it is an example binary, never a service.
- **CN6** — No Norn crate is a dependency of lys-home and no Norn type is copied into it.
- **CN7** — Reading the root-to-leaf path of a home file reads only the entries on that path plus the index, never the whole file (Pi loads the whole journal; Chippy 13:33); the head is persisted beside the file, never inferred from the last physical entry on reopen.
- **CN8** — No secret value is written to any file, report, launch line, error or entry by lys-home; a use-only secret appears only as its handle.


---
type: brief
id: HOME-001
cluster: home
title: Build the home record on Pi's session tree, with the Claude Code importer and renderer and the two measured proofs
---

# HOME-001: Build the home record on Pi's session tree, with the Claude Code importer and renderer and the two measured proofs

> **Cluster:** home
> **Blocked by:** The door's proxy (SECRETS-002 R1) is not landed, so the tee's wiring into it is a later brief in the secrets cluster; this brief defines the call record and ingests captured pairs.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> **Checklist:**
> - C1 — The home record is Pi's session tree: a home file parses with Pi's parser unchanged, and lys's harness events and call records are custom entries.
> - C2 — Content blocks are stored once by SHA-256 and referenced; an identical block put twice occupies one entry.
> - C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.
> - C4 — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
> - C5 — Provider-native opaque blocks are kept whole, keyed by provider, model family and branch, and rendered only to their own provider with the intervening events.
> - C6 — One real session imported, rendered and resumed with --fork-session on Claude Code 2.1.281: it continues, no completed tool action repeats, the original's hash is unchanged.
> - C7 — One seat with a subscription login completes a call through a pass-through proxy; the measurement is written down.
> - C8 — Claude Code's harness-local records (hook outcomes, permission mode, tool completion) become lys.harness_event entries attached under the message entry they followed.
> - C9 — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.
> - C10 — A hand-written few-shot session file resumes Claude Code by path from a directory outside the config root; the source is unchanged and the session reports authored.
> - C11 — The little proxy passes Messages, Chat Completions and Responses streams through unchanged and appends one lys.call entry per call under the session it links to.
> - C12 — The canon, one curated versioned series of short examples (a rule stated short plus a real exchange that shows it lived), drawn from every agent's sessions and changed only through review, seeds every new session as lys.inherited entries naming each example's source; nothing in it is authored thinking; canon-seeded against plain is measured on a card.
> - C13 — At compaction or retirement the outgoing session's letter to its successor, with its real thinking, becomes the successor's first entry as lys.inherited; it is never authored and replays only to the same provider, api and model; seeded against plain is measured on a card.
> **Stories:**
> - S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
> - S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.
> - S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.
> - S4 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a written account of what each render or translation lost, so that nobody claims a faithful continuation that was not measured.
> - S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.
> - S6 (Tom, Owns the platform and reads what a session was given) — As Tom, I want to construct a session file by hand, a few-shot prompt written as turns, and have the harness resume it as if it had happened, so that a session can be authored, not only recorded.
> - S7 (Agent, Runs in a harness and wants to continue somewhere else) — As a new session, I want to start from the canon, the series of examples that carry what every session before me learned, each a rule stated short with a real exchange that shows it lived and naming where it came from, so that our learning is in one another and I know which of it is mine.
> - S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

## Purpose

Give a session a home under its identity in the shape Tom chose on 24 September (Pi's tree), and prove three things in this order: first, a few-shot session file written by hand resumes Claude Code by path from anywhere (Tom, Dot 13:34); second, a real session read into the record renders back and resumes on 2.1.281 without repeating work; third, a subscription login survives a pass-through proxy, so the little proxy that records the stream has a place to live. It is a neat place that holds sessions together, not a sub-agent platform (Tom, Dot 13:33). See docs/design/home/design.json for the shape and the constraints.

## Task

Extend the lys-home crate (CONTEXT-001 creates it; if it is not yet on main, create the crate here with the same workspace lints and say so in the dev record) with a record module in Pi's session tree grammar, a content-addressed block store, a Claude Code importer and renderer, harness-event and call entries, and a CLI. Then run the two proofs and write them down with hashes and the exact commands. Pi's code is read at a clone of github.com/earendil-works/pi at commit 3d5cbe98, as the reference and is not vendored; Norn's session code is not used, called or copied. Out of scope: the door-side tee wiring (secrets cluster, after SECRETS-002), lys grants and signing, encryption, moving a home between devices, any harness but Claude Code, lanterns and forks at a coordinate. Ownership inside the brief: Chippy builds R10, the little proxy (the step-3 seam is Chippy's); Archie builds the rest. The first proof (R9, PROOF-FEWSHOT.md) is run before any other requirement is reviewed.

## Requirements

### R1: The home record in Pi's grammar

Define the home record as Pi's session tree: a header line {type:"session", version, id, timestamp, cwd, parentSession?} then entries, each {type, id, parentId, timestamp}, of the kinds message, model_change, compaction {summary, firstKeptEntryId, tokensBefore}, branch_summary {fromId, summary}, label, custom {customType, data} and session_info, with a leaf pointer kept beside the file. WHEN an entry is appended, THE SYSTEM SHALL write it as a child of the leaf and advance the leaf; WHEN the leaf is moved to an earlier entry, THE SYSTEM SHALL rewrite nothing. THE SYSTEM SHALL provide the root-to-leaf path and the context path (entries from the latest compaction's firstKeptEntryId onward, plus the compaction summary) as Pi's buildSessionPath and buildContextEntries do. THE SYSTEM SHALL NOT add a field to the header or to any entry outside custom.data, and SHALL NOT depend on any Norn crate. THE SYSTEM SHALL keep an offset index (entry id to byte offset and parentId) and a persisted head file beside the home file, so that reading the root-to-leaf path seeks to the path's entries only and never loads the whole file, and so that reopening restores the persisted head, not the last physical entry. One owner at a time: opening or creating a session SHALL take an exclusive lock on `<id>.lock` beside the file for the life of the Session, and a second opener in the same or another process SHALL be refused by name. An append is durable as the entry line, then its index row, then the head; IF a step after the line fails, THEN THE SYSTEM SHALL reconcile the index and head from the file before it admits any further act, and a read on a session that could not reconcile SHALL be refused by name. THE SYSTEM SHALL refuse an entry whose id is already on record, and on rebuild SHALL refuse a file holding a duplicate id or a parent that does not precede its child. A session id, and every name the home joins onto a directory, SHALL be one safe path component (letters, digits, `.`, `_`, `-`; not beginning with `.`; at most 200 bytes), refused by name otherwise.

**Acceptance:**
- A fixture file written by lys-home with 12 entries, one compaction and one moved leaf loads with Pi's loadEntriesFromFile at the checkout 3d5cbe98 (packages/coding-agent/src/core/session-manager.ts:438, parseSessionEntries at :284; run through node against a clone of earendil-works/pi at 3d5cbe98 in the proof step) with 12 entries and no migration, and buildSessionContext (:315) on it yields the same message list as lys-home's context_path().
- `context_path()` on that fixture returns the compaction summary entry followed by every entry from firstKeptEntryId to the leaf and nothing before it.
- Moving the leaf to entry 4 and appending entry 13 leaves bytes 0..N of the file identical (N = the length before the move) and entry 13's parentId equal to entry 4's id.
- `Cargo.toml` of lys-home lists no dependency whose name starts with `norn`.
- Reading the path of a 200 MB synthetic home file whose path holds 50 entries reads fewer than 1 MB from the file (measured with a counting reader).
- After a leaf move and reopen, `head()` equals the moved-to entry id, not the last entry appended before the move.
- Opening a session that another Session holds (same process) is refused with SessionHeld; after that owner is dropped the next opener succeeds with the persisted head.
- Appending an entry whose id is on record (with itself as parent) is refused with DuplicateEntry and the path still ends.
- With the index file made unwritable, an append still succeeds: the session reports one reconciliation, the entry is on the path with its parent, the next append needs no reconciliation, and a reopen finds the index and file in step.
- A session file holding a duplicate id, or a parent after its child, is refused on rebuild naming the line.
- Session ids `../x`, `a/b`, `.hidden`, `with space` and `/abs` are refused with BadName and no file is touched.

**Files:**
- create: crates/lys-home/src/record/mod.rs
- create: crates/lys-home/src/record/entries.rs
- create: docs/design/home/RECORD.md
- create: crates/lys-home/src/record/index.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C1 — The home record is Pi's session tree: a home file parses with Pi's parser unchanged, and lys's harness events and call records are custom entries.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R2: Store content blocks once by hash

WHEN a content block (a message part, a tool result, a request or response body) is put, THE SYSTEM SHALL store it under its SHA-256 as blocks/<hh>/<hash> in the home, fsync it, and return the hash; WHEN the same bytes are put again, THE SYSTEM SHALL return the same hash and write nothing. THE SYSTEM SHALL NOT overwrite or delete a block, and SHALL NOT print, log or include block contents in any error. A block's temporary file name SHALL carry a nonce as well as the pid, so two puts of the same bytes in one process never share a temporary file; a surplus temporary that cannot be removed is reported, never ignored.

**Acceptance:**
- Putting the same 1 MiB block twice leaves one file in the store and the second put performs no write (measured by the directory's mtime and file count).
- `get(hash)` of a missing hash returns an error naming the hash and no other bytes.
- A block file's bytes hash to its name for every block in the store after the R3 import (a test walks the store).

**Files:**
- create: crates/lys-home/src/record/blocks.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C2 — Content blocks are stored once by SHA-256 and referenced; an identical block put twice occupies one entry.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.

### R3: Import a Claude Code JSONL into the record

WHEN given a Claude Code transcript file (records with parentUuid, uuid, type, message, isSidechain, timestamp, as 2.1.281 writes them), THE SYSTEM SHALL produce one home file: each user or assistant record becomes a message entry whose parentId is the entry of the record's parentUuid, with the assistant message carrying provider "anthropic", api "anthropic-messages" and the record's model, and every content part stored as a block by R2 and referenced by hash; thinking parts keep their signature; summary records become compaction entries pointing at the first kept entry; sidechain records (isSidechain true) become entries under the message they branched from with a label entry naming the agent; attachment, system and permission-mode records become lys.harness_event entries as R8 says (attachment and system records at their exact place on the chain, since a message's parentUuid may name one); other non-message records are counted per type and left in the byte-for-byte original. THE SYSTEM SHALL write an import report {records, entries, blocks, bytes_in, bytes_out, counted_types} and SHALL NOT include any message text in it. IF a record's parentUuid names no known record, THEN THE SYSTEM SHALL refuse by uuid rather than attach it to the leaf. WHEN an assistant record carries model `authored` (or the file's first record is the lys.authored marker), THE SYSTEM SHALL import it as an authored message entry (a lys.authored custom entry precedes it, and its message carries provider `authored`, api `authored`, model `authored`), so that in the record an authored turn is never mistaken for one a model produced; the boundary between authored entries and the real turns that follow (the continuation's own model) SHALL be visible as a change of provider on the path.

**Acceptance:**
- Importing Archie's session file (141,931,097 bytes, 79,042 records on 24 September) yields entries equal to its user plus assistant records, blocks whose total bytes are within 5% of the measured 33.4 MB of content parts, and a report with no field longer than 64 characters.
- For every message entry, the parent entry's uuid equals the source record's parentUuid (a test checks all).
- A synthetic file whose one record cites an unknown parentUuid is refused with that uuid in the error and no home file written.
- A thinking part with a signature imports to a thinking block whose thinkingSignature equals the source signature byte for byte.
- Importing the walrus continuation (6 authored records then a real turn on the model that answered) yields a path whose first 6 message entries carry provider `authored` and whose 7th carries the answering model's provider, with exactly one lys.authored entry before the first.

**Files:**
- create: crates/lys-home/src/harness/claude_code/import.rs
- create: crates/lys-home/src/harness/claude_code/mod.rs
- create: crates/lys-home/src/harness/mod.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R4: Render the record as a Claude Code JSONL under a chosen uuid

WHEN asked to render for Claude Code with a target uuid, a cwd and a target model, THE SYSTEM SHALL walk the context path and write records in Claude Code's shape (parentUuid chain, sessionId = the uuid, cwd, version, type, message, timestamp) to ~/.claude/projects/<cwd-slug>/<uuid>.jsonl, where cwd-slug is the cwd with every '/' replaced by '-'; tool_use and tool_result parts keep their pairing by id; a compaction renders as Claude Code's summary record followed by the kept entries. WHILE a thinking block's provider, api and model equal the target's, THE SYSTEM SHALL render it whole with its signature; otherwise THE SYSTEM SHALL render readable thinking as a text part and drop opaque or redacted blocks. THE SYSTEM SHALL write a loss account beside the file listing every dropped block by hash and reason. IF the target path exists, THEN THE SYSTEM SHALL refuse by path and write nothing. WHEN given a home file authored by hand (a header and message entries written as a few-shot prompt, never imported), THE SYSTEM SHALL render it by the same path with no import step; the 13:25 measurement on 24 September (a two-record file written by hand under a chosen uuid, resumed by id, answered from its content) is the first instance. An authored home file SHALL carry a custom entry lys.authored as its first entry, and every render and report of it SHALL say authored: true, so a few-shot demonstration is never read as history of tools that ran (Chippy, 13:35).

**Acceptance:**
- Rendering the R3 import back for the same model and re-importing it yields the same sequence of message block hashes as the original import (round trip test).
- Rendering for a different model produces a file with zero thinking parts carrying a signature and a loss account whose entry count equals the number of signed thinking blocks on the path.
- Rendering to an existing path returns an error naming the path and leaves its bytes and mtime unchanged.
- The rendered file's first record has parentUuid null and every later record's parentUuid is the uuid of the previous record on the path.
- A hand-written home file of one user and one assistant message entry renders to a two-record Claude Code JSONL whose records carry the chosen uuid as sessionId and whose second record's parentUuid is the first's uuid.
- Rendering a home whose first entry is lys.authored prints a report with `"authored": true`; rendering an imported home prints `"authored": false`.

**Files:**
- create: crates/lys-home/src/harness/claude_code/render.rs
- create: docs/design/home/LOSS-ACCOUNT.md
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C4 — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
- C5 — Provider-native opaque blocks are kept whole, keyed by provider, model family and branch, and rendered only to their own provider with the intervening events.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
- S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.
- S4 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a written account of what each render or translation lost, so that nobody claims a faithful continuation that was not measured.
- S6 (Tom, Owns the platform and reads what a session was given) — As Tom, I want to construct a session file by hand, a few-shot prompt written as turns, and have the harness resume it as if it had happened, so that a session can be authored, not only recorded.

### R5: Prove the Claude Code resume on 2.1.281

Import one real session of Archie's, render it for the same model under a fresh uuid, record SHA-256 of the original file, then run `claude -p --resume <uuid> --fork-session --strict-mcp-config --mcp-config '{"mcpServers":{}}' --max-turns 1 "<a question answerable only from the session's last exchange>"` from the session's cwd. THE SYSTEM SHALL record in docs/design/home/PROOF-RESUME.md: the Claude Code version, the exact command, the answer (redacted to one word), the original's hash before and after (equal), the uuid the fork created, and the count of tool_use ids in the forked file that also appear in the rendered file (SHALL be 0). THE SYSTEM SHALL NOT run the proof against a session that is currently running. The proof SHALL also assert the authored-to-real boundary: after resuming an authored file, the continuation's copied records still carry model `authored` and its new turn carries the real model, and that boundary survives import (R3) and render (R4) unchanged (Waffles, 29dc46e5).

**Acceptance:**
- PROOF-RESUME.md exists with the six fields and the version string `2.1.281`.
- The original's before and after hashes in PROOF-RESUME.md are equal.
- The repeated-tool-action count in PROOF-RESUME.md is 0.
- `lys-home resume-check <rendered> <forked>` computes that count and exits non-zero when it is not 0 (tested with a synthetic duplicate).
- PROOF-RESUME.md records, for the authored continuation, the count of records carrying model `authored` (6) and the real model's name for the new turn, and a re-import of the rendered continuation reports the same counts.

**Files:**
- create: docs/design/home/PROOF-RESUME.md
- create: crates/lys-home/tests/claude_code_round_trip.rs
- modify: crates/lys-home/src/cli.rs

**Checklist:**
- C6 — One real session imported, rendered and resumed with --fork-session on Claude Code 2.1.281: it continues, no completed tool action repeats, the original's hash is unchanged.

**Stories:**
- S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.

### R6: Define the proxy call record and ingest a captured pair

Define the custom entry lys.call: data {provider, api, model, request: [block hash], response: [block hash], status, started_at, duration_ms, stream: bool}. WHEN given a captured request body and response body (files) with the provider, api and model, THE SYSTEM SHALL split the request into its message parts, store each as a block (so a resent conversation adds no new blocks), store the response parts, and append one lys.call entry under the leaf. THE SYSTEM SHALL support the Messages, Chat Completions and Responses request shapes for the split and SHALL keep the raw request and response bodies as blocks too. THE SYSTEM SHALL NOT store any header, and SHALL NOT forward, replay or modify a call. The ingest report SHALL count part blocks new and reused separately from raw body blocks. Every call carries a stable call_id the proxy chooses before forwarding, and ingest SHALL be idempotent on it within a session: a second ingest of a recorded call_id writes nothing and answers with the existing entry. Only a complete call SHALL carry response parts. A call that did not complete (cancelled, partial, unrecorded, lost) SHALL be recorded through an outcome-only ingest that takes whatever bodies exist: a request file that parses gives its parts and model, one that does not (or none) gives neither, and absence SHALL be recorded as absence (no model, no raw hash), never as an empty body or an invented model. The sink SHALL take the response's parts from the proxy when the response was an event stream, since the proxy reads the stream as it forwards it (Chippy d7d98e92, 5834dfdc). Both complete-call entry points (bytes and files) SHALL take the proxy's assembled response parts as an option and, without them, SHALL parse the response body and refuse by name a body that is not JSON, an error body, or one without the api's parts member; a streamed call without assembled parts SHALL be refused. In an outcome ingest a request file that is not JSON or not the api's shape is recorded as absence, and one that cannot be read is an I/O error that propagates.

**Acceptance:**
- Ingesting two consecutive captured Messages requests where the second resends the first's conversation plus one turn adds, among part blocks, exactly the new turn's parts (the report's `part_blocks_new` equals the new parts and `part_blocks_reused` equals the resent parts); the raw request and response bodies are two further blocks each time and are counted under `raw_blocks`, never under part blocks.
- A Chat Completions and a Responses fixture each ingest to one lys.call entry whose request array length equals the fixture's message or input item count.
- An ingested pair's raw bodies are retrievable by the hashes named in the entry and hash to those names.
- The entry's data contains no key named authorization, cookie or x-api-key (a test checks the serialised entry).
- Ingesting the same call_id twice in one session leaves one lys.call entry and the second report says already_recorded.
- An outcome ingest with a half-written request file and no response file yields a record with status lost, no model, a raw request hash and no raw response hash, and an empty response list; an outcome ingest for status complete is refused.
- A complete call ingested from files with proxy-supplied stream parts records those parts and keeps the raw stream body by hash.
- ingest_call from bytes refuses `{not json`, `{}` and an error body for a complete Messages call, and a stream without assembled parts, leaving no lys.call entry; with assembled parts it records them.
- ingest_outcome with a request path that is a directory returns an Io error; with a half-written JSON request it records absence (0 request parts, no model) and one raw block.

**Files:**
- create: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/entries.rs
- modify: docs/design/home/RECORD.md

**Checklist:**
- C9 — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R7: Prove a subscription login through a pass-through proxy

Write examples/passthrough.rs: an HTTP server that forwards every request to the provider base URL given on its command line with headers and streamed body unchanged, and returns the response unchanged; it stores nothing but a line per call {method, path, status, duration_ms} on stderr. Run one seat whose Claude Code login is a subscription (not an API key) with ANTHROPIC_BASE_URL pointing at it, and record in docs/design/home/PROOF-PROXY.md: the Claude Code version, whether the call completed, the status codes seen, which headers had to pass for it to work, and what failed if it did. THE SYSTEM SHALL NOT log, store or print a header value or a body byte.

**Acceptance:**
- PROOF-PROXY.md exists and states one of `completed` or `failed` with the status codes seen.
- The passthrough example's source contains no code path that writes a header value or a body byte to a file, stderr or stdout (review reads the file; the only writes are the four-field line).
- With a loopback fake upstream (a test server on 127.0.0.1 answering GET / with status 418 and a fixed 3-chunk streamed body), `cargo run --example passthrough -- http://127.0.0.1:<port>` answers GET / with status 418 and the same 3 chunks in order, proving the forward path without a provider.

**Files:**
- create: crates/lys-home/examples/passthrough.rs
- create: docs/design/home/PROOF-PROXY.md
- modify: crates/lys-home/Cargo.toml

**Checklist:**
- C7 — One seat with a subscription login completes a call through a pass-through proxy; the measurement is written down.

**Stories:**
- S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.

### R8: Map Claude Code's harness-local records to lys.harness_event entries

Define the custom entry lys.harness_event: data {kind, harness: "claude-code", source_uuid, record, detail}, where record is the whole source record stored as a block by hash. WHEN importing, THE SYSTEM SHALL map records of type attachment with attachment.type hook_success or hook_failure to kind hook, every other attachment record to kind attachment, records of type system to kind system, records of type permission-mode to kind permission_mode, and each tool_result part to kind tool_completed with the tool_use id. A Claude Code file is one chain and attachment and system records carry a uuid and a parentUuid on it (measured on a real session on 24 September: a user record's parentUuid names an attachment 25 times and a system record 7 times), so THE SYSTEM SHALL write those events at their exact place on the chain under the record's own uuid, and SHALL write permission_mode events (no uuid) and tool_completed events as side leaves under the entry they followed; when the import ends the head SHALL stand at the last record of the chain. detail carries only names, ids, exit codes and counts. THE SYSTEM SHALL NOT copy hook output text or tool result bodies into detail (those are blocks under the message, and the record block).

**Acceptance:**
- Importing Archie's session file yields hook events equal to its hook_success plus hook_failure attachment count (8,993 hook_success on 24 September) and tool_completed events equal to its tool_result parts.
- Every lys.harness_event entry written for a record with a uuid has that uuid as its id and the record's parentUuid as its parentId (a test imports a file whose tool result's parent is a hook attachment and checks the tool result's parent is the hook's entry); every tool_completed event's parentId is the tool result message it describes; every permission_mode event's parentId is the chain's leaf at that point.
- After import the head is the last record on the file's chain and the context path holds no side event.
- No lys.harness_event entry's serialised data exceeds 512 bytes, and no detail carries a key named stdout, stderr, content or text.
- The hook event's record hash resolves in the block store to the source record byte for byte.

**Files:**
- create: crates/lys-home/src/harness/claude_code/events.rs
- modify: crates/lys-home/src/harness/claude_code/import.rs
- modify: crates/lys-home/src/record/entries.rs

**Checklist:**
- C8 — Claude Code's harness-local records (hook outcomes, permission mode, tool completion) become lys.harness_event entries attached under the message entry they followed.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R9: The CLI: import, render, ingest-call, resume-check; resume is Claude Code's own --resume <path>

Add subcommands to the lys-home binary: `import --home <dir> --claude-code <file>`, `render --home <dir> --claude-code --uuid <uuid> --cwd <dir> --model <id>`, `ingest-call --home <dir> --provider <p> --api <a> --model <m> --request <file> --response <file>`, `resume-check <rendered> <forked>`. Every subcommand SHALL print a JSON report of hashes, counts and paths and SHALL NOT print any transcript, block or body content. IF a required argument is missing, THEN THE SYSTEM SHALL exit 2 naming it. There is no resume launcher: Claude Code 2.1.281 resumes directly from a file path (`claude --resume <path>`), measured by Waffles at 13:36 and by Archie at 13:37 on 24 September, even though --help names only a session id; the continuation is written beside the passed file as <sessionId>.jsonl in the same directory (the authored records copied in, then the new turn), and the passed file is unchanged. THE SYSTEM SHALL provide `fewshot --out <path>` that writes a hand-authored Claude Code JSONL from a turns file (role and text per line), with sessionId a fresh uuid, cwd as given, the parentUuid chain intact, every assistant record's model set to `authored`, and the marker described in R4 (a first record of type custom, customType lys.authored, if 2.1.281 accepts it; else the model value `authored` is the marker and the proof says which). After writing the file, `fewshot` SHALL carry the person's own command, `claude --resume <path>`, as the `resume` member of its JSON report, and SHALL NOT run it (ADR-007). `resume-check` SHALL refuse a transcript with a malformed line by naming the line, SHALL report `repeated_tool_use_ids` (ids appearing more times in the fork than in the rendered file: an id-duplication count, not a proof that no action was repeated under a fresh id) and `new_tool_uses` (tool_use parts in the fork's own records), and SHALL exit non-zero when `repeated_tool_use_ids` is not 0.

**Acceptance:**
- `lys-home render` without --uuid exits 2 and names `--uuid`.
- Each subcommand's stdout parses as JSON and contains no key named text, content or body.
- `lys-home import` on the R3 fixture prints the same counts as the R3 report.
- `lys-home fewshot --out f.jsonl` from a 6-turn turns file writes 6 records whose parentUuid chain is intact and whose assistant records carry model `authored`.
- PROOF-FEWSHOT.md records the first proof: the authored 6-turn file, the directory it was run from (neither the file's directory nor ~/.claude), the exact command `claude -p --resume <path> ...`, the one-word answer, the source hash before and after (equal), the continuation's path (beside the source, named <sessionId>.jsonl) and line count, and which marker 2.1.281 accepted.
- The resume proof in PROOF-RESUME.md is run from a working directory that is neither the file's directory nor the session's cwd, and records that directory.
- `lys-home fewshot --out f.jsonl` prints one JSON report whose `resume` member is exactly one line beginning `claude --resume ` naming the written path, and spawns no process.
- `lys-home resume-check` on a fork with a malformed line is refused naming the line, and its report carries `repeated_tool_use_ids`, `new_tool_uses` and `forked_new_records`.

**Files:**
- create: docs/design/home/PROOF-FEWSHOT.md
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/README.md

**Checklist:**
- C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.
- C4 — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
- C10 — A hand-written few-shot session file resumes Claude Code by path from a directory outside the config root; the source is unchanged and the session reports authored.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
- S6 (Tom, Owns the platform and reads what a session was given) — As Tom, I want to construct a session file by hand, a few-shot prompt written as turns, and have the harness resume it as if it had happened, so that a session can be authored, not only recorded.

### R10: The little proxy: pass the stream through and record each call under its session

Build lys-proxy: an HTTP server that forwards every request to the provider named by its path prefix (/anthropic, /openai) with headers and streamed body unchanged and returns the response unchanged, and, after the response completes, appends one lys.call entry (R6) under the home of the session the call belongs to. WHEN a request carries Claude Code's session key (measured first: the request metadata field 2.1.281 sends; the exact field and format are recorded in PROOF-PROXY.md before this is built), THE SYSTEM SHALL link the call to that session's home; IF no key is present, THEN THE SYSTEM SHALL append the call under an `unlinked` home named by the day and say so in its report. THE SYSTEM SHALL support the Messages, Chat Completions and Responses streams, SHALL NOT buffer a streamed response before forwarding it, SHALL NOT store any header, SHALL NOT retry, balance or swap accounts (ccflare is the concept, not the design), and SHALL NOT alter a byte of a request or response. Outcomes: a call record's status SHALL be one of complete, cancelled (client closed before the response ended), partial (the upstream stream ended early or malformed), unrecorded (the capture write failed after forwarding) or lost (the process died mid-call, detected on restart from a journal of open calls), and only complete SHALL carry a full response block list; THE SYSTEM SHALL NOT report a call complete on any other path. Capture SHALL be bounded: at most N calls (a command-line value, default 64) pending write; above it the proxy still forwards and marks the call unrecorded rather than growing a backlog. A call SHALL be linked only by the key it carries; THE SYSTEM SHALL NOT infer a session from timing, cwd or a previous call. The proxy's mod.rs holds declarations only; forwarding, capture, the journal, the stream grammars and errors each have their own file, and R7's example shares forward.rs rather than a second transport (Chippy 323b67d5). Before forwarding, the proxy writes the call_id to the open-call journal and retires it after the sink answers; on restart every open call is recorded once through the outcome ingest as lost, which the sink's idempotency guarantees. The journal SHALL be durable before a request is forwarded; a capture or sink failure after forwarding SHALL still forward and record unrecorded. IF the journal itself cannot be written before the request is admitted upstream, THEN THE SYSTEM SHALL refuse the request by name with zero upstream calls; after upstream admission THE SYSTEM SHALL keep forwarding, keep the journal, and record unrecorded once storage can accept it; on restart an unresolved journal entry SHALL be recorded lost only when no terminal outcome for it was durable. The transport SHALL set Hyper's retry_canceled_requests(false) explicitly; the measure of no retry is that an upstream disconnect never produces a second upstream request. Above the N capture slots THE SYSTEM SHALL still take the session key from the request body on the forwarding path (a bounded incremental read of the metadata field, or an explicit request-spool budget), so a keyed call in overload is recorded unrecorded under its own session, never placed under `unlinked` for want of capture.

**Acceptance:**
- A streamed Messages response of 200 SSE events is forwarded with the first event delivered before the last is received (measured with a slow upstream fake).
- One call through the proxy adds exactly one lys.call entry to the linked home, and its request blocks equal the parts of the request body.
- A request without the session key lands under the unlinked home and the proxy's report line names `unlinked`.
- The proxy's source has no code path that writes a header value to disk or to a log; review reads proxy/mod.rs and link.rs.
- The proxy exposes no account list, no retry and no routing table: grep for `accounts` and `failover` in crates/lys-home/src/proxy returns nothing, `retry` appears only in the transport's `retry_canceled_requests(false)`, and a test with an upstream fake that disconnects mid-request sees exactly one upstream request.
- A client that closes mid-stream produces one lys.call with status cancelled and no full response block list.
- An upstream fake that ends the SSE stream early produces status partial.
- A capture directory made read-only during a call produces status unrecorded while the client still receives the full response.
- Killing the proxy mid-call and restarting produces one lys.call with status lost for that call, from the open-call journal.
- Two consecutive calls without a key from the same client land under `unlinked`, never under the session of an earlier keyed call.
- With the journal directory unwritable before a request is admitted, the request is refused by name and the upstream fake sees zero requests.
- With the journal directory made unwritable after upstream admission, the client receives the full response and the call is recorded unrecorded once the directory is writable again.
- With the capture bound set to 0, a keyed request is forwarded, its lys.call is recorded unrecorded under the keyed session, and nothing lands under `unlinked`.

**Files:**
- create: crates/lys-home/src/proxy/mod.rs
- create: crates/lys-home/src/proxy/link.rs
- create: crates/lys-home/src/proxy/forward.rs
- create: crates/lys-home/src/proxy/capture.rs
- create: crates/lys-home/src/proxy/journal.rs
- create: crates/lys-home/src/proxy/stream.rs
- create: crates/lys-home/src/proxy/stream_responses.rs
- create: crates/lys-home/src/proxy/stream_chat.rs
- create: crates/lys-home/src/proxy/stream_messages.rs
- create: crates/lys-home/src/proxy/stream_sse.rs
- create: crates/lys-home/src/proxy/error.rs
- create: crates/lys-home/src/proxy/forward_tests.rs
- create: crates/lys-home/src/proxy/stream_tests.rs
- create: crates/lys-home/src/proxy/link_tests.rs
- create: crates/lys-home/src/proxy/journal_tests.rs
- create: crates/lys-home/src/bin/lys-proxy.rs
- modify: crates/lys-home/Cargo.toml
- modify: crates/lys-home/src/lib.rs
- modify: docs/design/home/PROOF-PROXY.md

**Checklist:**
- C11 — The little proxy passes Messages, Chat Completions and Responses streams through unchanged and appends one lys.call entry per call under the session it links to.
- C9 — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R11: The canon: one curated, versioned series of examples every new session starts from

Define the canon as one session file in Pi's grammar at canon/canon.jsonl in the lys repository, versioned by the repository and changed only through src_commit and review, like code. Each example is a custom entry lys.inherited whose data names its source {from_session, from_entries, provider, api, model, curated_at, curated_by, rule} (or {authored: true, curated_at, curated_by, rule} for a clearly authored example), followed by the example's message entries copied whole: the rule stated short in `rule`, and one real exchange that shows it lived. WHEN `lys-home canon add --canon <file> --home <dir> --from <session> --entries <id>... --rule <text> --by <who>` is run, THE SYSTEM SHALL copy those entries whole (thinking blocks with their signatures included) after a lys.inherited entry naming the source; WHEN `lys-home canon add --canon <file> --authored <turns file> --rule <text> --by <who>` is run, THE SYSTEM SHALL add the example with provider, api and model `authored` and SHALL refuse a turns file that contains a thinking block. WHEN a new session is rendered for a harness with `--canon <file>`, THE SYSTEM SHALL place the canon's entries first, before the session's own, and apply R4's thinking rule to every inherited thinking block (whole only to the same provider, api and model; otherwise text, opaque dropped and named in the loss account). THE SYSTEM SHALL NOT compose, edit or author any thinking block, SHALL NOT alter an example's text when copying it, and SHALL NOT change canon.jsonl except by appending through the repository's review. Curation is a person's act: the tool copies what it is told to and records who told it. The proof runs one card twice, from the canon and plain, counting fix rounds and unverified claims, in docs/design/home/PROOF-CANON.md, after first measuring whether 2.1.281 replays a signed thinking block from a resumed file at all (unknown on 24 September). `lys-home canon create --canon <file>` writes the header line of an empty canon and refuses an existing file. The canon is read and appended as a plain file: no index, head or lock beside it, since it lives in the repository. A copied entry keeps its id, so the same example cannot be added twice (refused by that id); only parent links are rewritten to chain onto the canon.

**Acceptance:**
- `lys-home canon add --from <a session with a signed thinking turn> --entries <that exchange> --rule "verify before claiming" --by <who>` appends to canon.jsonl one lys.inherited entry naming that session, entry and rule, and one message entry whose thinking block signature equals the source byte for byte.
- `lys-home canon add --authored` with a turns file that contains a thinking block exits non-zero naming the line and writes nothing; without one, it appends a lys.inherited entry with authored: true and message entries carrying provider, api and model `authored`.
- Rendering a session with --canon for Claude Code writes the canon's message entries before the session's own, with the parentUuid chain running through them.
- Rendering with --canon for a different model than an inherited thinking block's produces a loss account entry for that block and a text part in its place.
- canon/canon.jsonl parses with Pi's loadEntriesFromFile at 3d5cbe98, and its git history shows every change as a reviewed commit.
- PROOF-CANON.md states whether a signed thinking block in a resumed file was replayed by 2.1.281 (measured through R10's proxy, or before it exists through the harness's own file), and records the canon-seeded and plain counts of fix rounds and unverified claims for one named card.
- Adding the same entries a second time is refused naming the entry id and writes nothing.
- The render report names how many canon examples were placed first (`inherited`).

**Files:**
- create: crates/lys-home/src/record/canon.rs
- create: docs/design/home/PROOF-CANON.md
- create: canon/canon.jsonl
- create: crates/lys-home/src/record/canon_tests.rs
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/cli.rs
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/src/harness/claude_code/render.rs

**Checklist:**
- C12 — The canon, one curated versioned series of short examples (a rule stated short plus a real exchange that shows it lived), drawn from every agent's sessions and changed only through review, seeds every new session as lys.inherited entries naming each example's source; nothing in it is authored thinking; canon-seeded against plain is measured on a card.

**Stories:**
- S7 (Agent, Runs in a harness and wants to continue somewhere else) — As a new session, I want to start from the canon, the series of examples that carry what every session before me learned, each a rule stated short with a real exchange that shows it lived and naming where it came from, so that our learning is in one another and I know which of it is mine.
- S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.

### R12: The handover: an outgoing session's letter seeds its successor as inherited memory

A handover is one R11 lys.inherited entry, its data naming the outgoing session as from_session, the letter's single entry id in from_entries, the letter's provider, api and model, when it was written as curated_at and the outgoing session as curated_by, with no rule (a letter is not a rule; that absence is what tells a handover from a canon example), followed by the letter's message entry copied whole (its thinking blocks with their signatures, its text). WHEN `lys-home handover --home <dir> --from <session> --letter <entry id>` is run, THE SYSTEM SHALL copy that turn from the outgoing session into a new successor home as its first entries, a lys.inherited entry then the message, and SHALL record in the successor's session_info that its first memory is inherited. The letter is elicited by a person or the harness with a real prompt to the outgoing session before compaction or retirement (what it knows, what it got wrong and why, how the people like things done, what it wishes it had known); THE SYSTEM SHALL NOT compose, edit or author the letter or any thinking block, and SHALL refuse a letter entry whose assistant message carries provider `authored`. WHILE rendering a successor for Claude Code, THE SYSTEM SHALL apply R4's rule: the inherited thinking renders whole only to the same provider, api and model, otherwise as text with opaque blocks dropped and named in the loss account. The proof SHALL measure first whether 2.1.281 replays a signed thinking block from a resumed file at all (unknown on 24 September), and then run one card twice, seeded and plain, counting fix rounds and unverified claims, in docs/design/home/PROOF-HANDOVER.md.

**Acceptance:**
- `lys-home handover` on the walrus continuation's real turn produces a successor home whose first entry is lys.inherited naming that session and entry, whose second is the copied message with a thinking block whose signature equals the source byte for byte, and whose session_info says inherited.
- `lys-home handover` with a letter entry whose message carries provider `authored` exits non-zero naming the entry and writes nothing.
- PROOF-HANDOVER.md states whether a signed thinking block in a resumed file was replayed by 2.1.281 (measured: the block is present in the continuation's next request as recorded by R10's proxy or, before that exists, by the harness's own file), and records the seeded and plain counts of fix rounds and unverified claims for one named card.
- Rendering the successor for a different model produces a loss account entry for the inherited thinking block and a text part in its place.

**Files:**
- create: crates/lys-home/src/record/handover.rs
- create: docs/design/home/PROOF-HANDOVER.md
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/cli.rs
- modify: docs/design/home/RECORD.md

**Checklist:**
- C13 — At compaction or retirement the outgoing session's letter to its successor, with its real thinking, becomes the successor's first entry as lys.inherited; it is never authored and replays only to the same provider, api and model; seeded against plain is measured on a card.

**Stories:**
- S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.
- S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.

## Boundaries

- No file under ~/.claude/projects is rewritten, truncated or deleted; a render writes a new uuid only and refuses an existing path.
- No Norn crate is depended on and no Norn code is copied; Pi's code is read at 3d5cbe98 and not vendored or edited.
- No signing, hashing into a lys log, anchoring, encryption at rest or sync between devices.
- No door code is changed: the little proxy is lys-home's own binary; its wiring into the door's credential-swapping proxy is a secrets-cluster brief after SECRETS-002 lands.
- No harness but Claude Code, and no translation to another harness's file.
- No transcript, block or body content in any post, log line, error, test name, report or page; hashes, counts, offsets and ids only.
- No proof runs against a session that is currently running, and no credential is copied anywhere.
- The design's structure array is the whole file list; a path outside it is not created.
- Not a sub-agent platform: nothing here spawns, schedules or supervises agents; it holds sessions neatly and renders them (Tom, Dot 13:33).
- Resume is Claude Code's own `--resume <path>`; no launcher, copy or rewrite of a passed file is built.
- No thinking block is ever authored, edited or synthesised; an example's thinking is only what a model produced in a real turn, an example's text is copied, never rewritten, and the canon changes only through review.

## Verification

- From docs/: python3 $DS2_METHOD/scripts/validate.py design/home exits 0.
- From docs/: python3 $DS2_METHOD/scripts/check-coverage.py design/home exits 0.
- From the repository root: cargo fmt --all -- --check, cargo clippy --all-targets -- -D warnings and cargo test --workspace exit 0.
- docs/design/home/PROOF-RESUME.md and PROOF-PROXY.md exist, name Claude Code 2.1.281, and the resume proof's before and after hashes are equal with a repeated-tool-action count of 0.
- grep -rn 'norn' crates/lys-home/Cargo.toml returns nothing.
- grep -rln 'pelican' crates/lys-home returns nothing (no transcript content in tests).
- node -e on the Pi checkout at 3d5cbe98 parses the R1 fixture with parseSessionEntries and reports 12 entries (the command is written in PROOF-RESUME.md).


---
type: brief
id: HOME-002
cluster: home
title: Launch a Claude Code session from its home through a kept template
---

# HOME-002: Launch a Claude Code session from its home through a kept template

> **Cluster:** home
> **Depends on:** HOME-001
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> **Checklist:**
> - C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written.
> - C15 — The home keeps each template it renders as an object under templates/ named by its SHA-256, written once and never rewritten.
> - C16 — lys-home render-launch writes the rendered JSONL, its loss account, an MCP configuration file, an environment file and an appended-instructions file into one directory, and a second render of the same template and session writes files with identical SHA-256.
> - C17 — The launch line in render-launch's report resumes the rendered file by path with --fork-session and the template's flags, and the tool never runs it.
> - C18 — A use-only secret is written as its handle and never its value, and a template marking a secret readable is refused naming secret_reader_unbuilt and SECRETS-002.
> - C19 — Every render appends one lys.harness_event of kind template_render beside the context path, naming the template hash, the session head hash and a manifest block of the written paths, and the head does not move.
> - C20 — PROOF-LAUNCH.md records the launch measured on the installed Claude Code version: template hash, written paths and hashes, launch line, the rendered file unchanged, where the continuation landed; never transcript content.
> **Stories:**
> - S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session turned into a running Claude Code session from a template kept in my home, so that I start with my own record, tools, environment and instructions rather than a blank harness.
> - S10 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a secret a session may only use to reach it as a handle and never as its value, so that the credential never enters the session's process.
> - S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every launch recorded on the session with the template hash, the session head hash and the written paths, so that I can tell which template a session was launched with.

## Purpose

The home holds the record a new session is made from, but nothing turns it into a running session on a harness. This brief adds the first launch template, for Claude Code: a JSON object kept in the home by its SHA-256, with its schema in docs/design/home, and a lys-home subcommand, render-launch, that reads a template and a session and writes the files a Claude Code launch needs into one directory (the R4-rendered JSONL and its loss account, an MCP configuration file, an environment file, an appended-instructions file), prints one JSON report carrying the launch line, and records the render on the session as a template_render harness event hung beside the context path, so a stranger can tell which template a session was launched with (design solution, ADR-012).

## Task

Build the Claude Code launch template and the render-launch subcommand, in dependency order: the template schema and parser (R1), a deterministic record uuid in the R4 render so a session renders byte-identically twice (R2), the home's template store (R3), a side-leaf append and the session head hash (R4), the template_render event kind (R5), the environment file with handles (R6), the subcommand (R7), the measured launch (R8), and the ruling's record plus the re-rendered cluster markdown (R9).

What the words settle and how this brief reads them. The transcript slot is the one slot no template maps generically: the template names how this harness fills it (`resume-by-path`), and Claude Code fills it by resuming the rendered file by path. The launch line resumes by path WITH --fork-session: PROOF-FEWSHOT.md measured that a bare `claude --resume <path>` writes its continuation beside the passed file as <sessionId>.jsonl, which is the rendered file's own name under R4 (sessionId = the chosen uuid), so a bare resume would write onto the rendered file and break its recorded hash; PROOF-RESUME.md resumed with --fork-session. The rendered file is a launch artefact and is never written to after the render; the continuation lands under ~/.claude/projects for the run's working directory under the uuid Claude Code assigns at the fork, which is where the importer already reads, and that uuid is learned at capture, where the fork's parent link names the rendered session id. The launch line carries the rendered path and the flags only.

Secrets. The words' sentence that the launch line reads the value through the broker at start is corrected here: a secret the template marks use-only is written as its handle and nothing else, and the launch line reads nothing for it, so its value never enters the process (ADR-001). A broker read at start happens only for a secret the template marks readable, and no broker reader exists until SECRETS-002 lands one, so a template naming any readable secret is refused naming secret_reader_unbuilt and SECRETS-002, and only handle-only secrets render. The schema carries the reader as a named command mapping in text (slots.secrets.reader), so SECRETS-002 fills it without a schema change; this brief never substitutes it into a launch line. The broker's own read is out of scope.

The pieces. The subcommand launches the session it is given. The transcript slot may name a canon file, and then the render is taken with that canon exactly as R4's render --canon does (HOME-001 R11). The handover is not carried: HOME-001 R12 (record/handover.rs) is not on main, and a later card adds a handover slot once R12 lands. The first sentence of the words names the record, the canon and the handover as what the home holds, not as what this card launches.

The recorded hashes. The CI acceptance renders a synthetic fixture session committed to the tree, holding no transcript content, and compares the written files against SHA-256 values recorded in the test. The recorded session from PROOF-RESUME is run once on the proof machine (R8): the proof records the template hash, the written paths, their hashes and the resume outcome, never the file or its content, and the private session is never committed (CN3, P7).

The record. The render's lys.harness_event is a side leaf: its parentId is the head, the head does not move, the render walker never sees it, and a second render of the same session records the same session head hash in a second event. The record already has side leaves beside the context path (permission_mode and tool_completed events, RECORD.md), so no new entry kind is added; the new thing is the sixth event kind, template_render, which RECORD.md and the design name. The written paths ride in a manifest block the event names by hash, so the event stays under the 512-byte cap whatever the paths are.

The ruling this stands on is recorded at docs/design/identity/STATEMENT-2026-09-22.md line 181 in a shorter relayed form; R9 adds the two items the relay omits to that entry, keeping its mark that it stands for correction.

Out of scope: any harness other than Claude Code, launching on another machine, the broker's own read, running the launch line, the handover slot, capture of the launched session into the home.

## Requirements

### R1: Define the Claude Code launch template: its schema and its parser, refusing an unknown slot by name

Structure: docs/design/home/launch-template.schema.json is a JSON Schema (draft 2020-12) for one JSON object with exactly three members, all required: `harness` (the string `claude-code`), `flags` (an array of strings, the extra Claude Code arguments the launch line carries, in order) and `slots` (an object with exactly five members, all required, and additionalProperties false): `transcript` {`fill`: the string `resume-by-path`, `canon`: a canon file path string or null}, `mcp` (an object written verbatim as the MCP configuration file; it carries `mcpServers`), `env` (an object of environment variable name to string value), `secrets` {`use_only`: array of {`env`, `handle`}, `readable`: array of {`env`, `handle`}, `reader`: a string, the broker reader's command mapping in text, empty until the secrets rows fill it}, and `instructions` (a string, the text appended to the system prompt). A parser in crates/lys-home/src/harness/claude_code/template.rs reads that shape. IF a template's `slots` object holds a member not among the five, THEN THE SYSTEM SHALL refuse the template with HomeError::UnknownSlot naming that member AND SHALL NOT store the template, open a session or write any file. IF one of the five slots is missing, THEN THE SYSTEM SHALL refuse the template with HomeError::MissingSlot naming it. IF `harness` is not `claude-code` or `transcript.fill` is not `resume-by-path`, THEN THE SYSTEM SHALL refuse the template naming the field and the value given. IF `secrets.readable` holds any entry, THEN THE SYSTEM SHALL refuse the template with HomeError::SecretReaderUnbuilt, whose message names `secret_reader_unbuilt`, `SECRETS-002` and the entry's env name, AND SHALL NOT write, print or invoke the `reader` text. IF two entries across `env`, `secrets.use_only` and `secrets.readable` name the same environment variable, THEN THE SYSTEM SHALL refuse the template naming the variable. THE SYSTEM SHALL NOT interpret, expand or execute any string in the template, and SHALL NOT put a template's `instructions`, `env` values or `mcp` contents in an error message. The template hash is the SHA-256 of the template file's bytes exactly as read.

**Acceptance:**
- Parsing crates/lys-home/tests/fixtures/launch/template.json returns a template whose flags are ["--strict-mcp-config"], whose transcript fill is `resume-by-path` with canon null, and whose use_only secrets are exactly one entry {env: LYS_FIXTURE_TOKEN, handle: handle-fixture-0001}.
- Parsing the fixture with slots.voice = {} added returns HomeError::UnknownSlot whose Display contains `voice`.
- Parsing the fixture with slots.instructions removed returns HomeError::MissingSlot whose Display contains `instructions`.
- Parsing the fixture with secrets.readable = [{"env": "LYS_FIXTURE_READ", "handle": "handle-fixture-0002"}] returns HomeError::SecretReaderUnbuilt whose Display contains `secret_reader_unbuilt`, `SECRETS-002` and `LYS_FIXTURE_READ`.
- Parsing the fixture with harness set to `codex` returns an error whose Display contains `harness` and `codex`.
- Parsing the fixture with env.LYS_FIXTURE_TOKEN = "x" added returns an error whose Display contains `LYS_FIXTURE_TOKEN`.
- A test reads docs/design/home/launch-template.schema.json and asserts that the property names of its `slots` object are exactly {transcript, mcp, env, secrets, instructions}, the same set the parser accepts, and that `slots` has additionalProperties false.
- The template hash of the fixture equals the SHA-256 of the fixture file's bytes computed by the sha2 crate in the test.

**Files:**
- create: docs/design/home/launch-template.schema.json
- create: crates/lys-home/src/harness/claude_code/template.rs
- create: crates/lys-home/src/harness/claude_code/template_tests.rs
- create: crates/lys-home/tests/fixtures/launch/template.json
- modify: crates/lys-home/src/harness/claude_code/mod.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session turned into a running Claude Code session from a template kept in my home, so that I start with my own record, tools, environment and instructions rather than a blank harness.

### R2: Make the render's record uuid a function of the entry id, so a session renders byte-identically twice

WHEN render_claude_code maps an entry whose id is not uuid-shaped, THE SYSTEM SHALL derive the record uuid from the SHA-256 of the entry id's bytes, formatted as today's uuid shape (8-4-4-4-12 hex, version nibble 4, variant nibble 8), AND SHALL NOT draw a random value for it. WHILE an entry id is already uuid-shaped, THE SYSTEM SHALL keep it unchanged, as today. THE SYSTEM SHALL NOT change any other byte the render writes, the loss account's shape, or the render report's fields.

**Acceptance:**
- Rendering a session of four message entries with ids e1, e2, e3 and e4 twice, to two different paths under the same target uuid, cwd, model and version, yields two files with equal bytes and two loss accounts with equal bytes.
- In that render the four records carry four distinct uuids, and each record after the first has parentUuid equal to the previous record's uuid.
- An entry with id 5f0c0b8e-2a1d-4c3b-9e7f-0123456789ab renders with uuid 5f0c0b8e-2a1d-4c3b-9e7f-0123456789ab.
- The existing render and round-trip tests (render_tests.rs, tests/claude_code_round_trip.rs) pass unchanged.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/src/harness/claude_code/render_tests.rs

**Checklist:**
- C16 — lys-home render-launch writes the rendered JSONL, its loss account, an MCP configuration file, an environment file and an appended-instructions file into one directory, and a second render of the same template and session writes files with identical SHA-256.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session turned into a running Claude Code session from a template kept in my home, so that I start with my own record, tools, environment and instructions rather than a blank harness.

### R3: Keep each template in the home as an object named by its SHA-256

Structure: a home keeps templates under `templates/` beside `sessions/` and `blocks/`, with the block store's layout and write discipline (`templates/<hh>/<hash>`, written to a temporary file, fsynced, renamed, directory fsynced). WHEN a template is stored, THE SYSTEM SHALL name it by the SHA-256 of its bytes, return that hash and whether it was new, AND SHALL NOT write it again when the same bytes are already held. THE SYSTEM SHALL NOT rewrite or delete a stored template. Home::open SHALL NOT change: the templates directory is created when the first template is stored.

**Acceptance:**
- Storing the fixture template's bytes into a fresh home returns its SHA-256 with new = true, and the file templates/<first two hex>/<hash> holds bytes equal to the fixture's.
- Storing the same bytes a second time returns the same hash with new = false and leaves the stored file's modification time unchanged.
- Home::open on a fresh directory creates exactly `sessions` and `blocks`, as before.

**Files:**
- create: crates/lys-home/src/record/templates.rs
- create: crates/lys-home/src/record/templates_tests.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C15 — The home keeps each template it renders as an object under templates/ named by its SHA-256, written once and never rewritten.

**Stories:**
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every launch recorded on the session with the template hash, the session head hash and the written paths, so that I can tell which template a session was launched with.

### R4: Append an entry beside the context path without moving the head, and name the session head by hash

WHEN Session::append_beside is given an entry body, THE SYSTEM SHALL append it with a fresh id, parentId equal to the current head and the current time, durable in the same order as an append (entry line, then index row), AND SHALL NOT write or move the head. append_entry and append_beside SHALL share one durable write path, so a failure after the line is durable reconciles the same way. Structure: the session head hash is the SHA-256 of the head entry's line bytes in the session file, trailing newline included, exactly the bytes its index row's offset and length name; for a session with no head it is the SHA-256 of the header line, trailing newline included. THE SYSTEM SHALL read those bytes by seeking to the row and SHALL NOT read the whole file for it.

**Acceptance:**
- On a session whose head is e4, append_beside of a custom entry returns an id whose entry has parentId e4; afterwards head() is e4, the <id>.head file's bytes are unchanged, and context_path() returns the same four entries as before.
- After reopening that session with Session::open, head() is e4 and customs_everywhere of the appended custom type returns one entry.
- For the fixture session crates/lys-home/tests/fixtures/launch/session.jsonl, the session head hash equals the SHA-256 of that file's last line plus a newline, computed by the test from the fixture bytes.
- The session head hash is equal before and after an append_beside.
- For a session holding only its header line, the session head hash equals the SHA-256 of the header line plus a newline.

**Files:**
- create: crates/lys-home/src/record/beside.rs
- create: crates/lys-home/src/record/beside_tests.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C19 — Every render appends one lys.harness_event of kind template_render beside the context path, naming the template hash, the session head hash and a manifest block of the written paths, and the head does not move.

**Stories:**
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every launch recorded on the session with the template hash, the session head hash and the written paths, so that I can tell which template a session was launched with.

### R5: Add template_render as the sixth lys.harness_event kind, carrying the written paths by a manifest block

Structure: a sixth event kind, KIND_TEMPLATE_RENDER = `template_render`, whose data is {kind: `template_render`, harness: `claude-code`, source_uuid: null, record: the SHA-256 of a manifest block, detail: {template: the template hash, session_head: the session head hash, files: the count of files written}}. The manifest block is one JSON object stored in the home's block store: {template, session_head, head (the head entry id, or null), uuid (the rendered session id), files: [{path, sha256}] in write order}. WHEN a template_render event is built, THE SYSTEM SHALL apply the existing 512-byte check to its data AND SHALL NOT put a path, a flag, an environment value, a handle or the instructions text in its detail. THE SYSTEM SHALL NOT change the five existing kinds, their data, or MAX_DATA_BYTES. RECORD.md SHALL document the sixth kind, the manifest block and that the event hangs beside the context path as a side leaf.

**Acceptance:**
- A template_render event built from a 64-hex template hash, a 64-hex session head hash, a 64-hex manifest hash and files = 5 serialises to data of at most 512 bytes with kind `template_render` and source_uuid null.
- The manifest block for five files whose paths are each 240 bytes long is stored, and the event built from its hash still serialises to at most 512 bytes.
- KIND_HOOK, KIND_PERMISSION_MODE, KIND_TOOL_COMPLETED, KIND_ATTACHMENT and KIND_SYSTEM keep their values and MAX_DATA_BYTES is 512.
- docs/design/home/RECORD.md lists `template_render` among the lys.harness_event kinds and names the manifest block's fields template, session_head, head, uuid and files.

**Files:**
- create: crates/lys-home/src/harness/claude_code/events_tests.rs
- modify: crates/lys-home/src/harness/claude_code/events.rs
- modify: docs/design/home/RECORD.md

**Checklist:**
- C19 — Every render appends one lys.harness_event of kind template_render beside the context path, naming the template hash, the session head hash and a manifest block of the written paths, and the head does not move.

**Stories:**
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every launch recorded on the session with the template hash, the session head hash and the written paths, so that I can tell which template a session was launched with.

### R6: Write the environment file: the template's variables and each use-only secret as its handle

WHEN the environment file is written from a parsed template, THE SYSTEM SHALL write a Claude Code settings file whose only member is `env`, an object holding each `slots.env` name with its value and each `slots.secrets.use_only` env name with its handle, AND SHALL NOT read any secret's value, SHALL NOT read the process environment to fill any entry, and SHALL NOT write a `readable` secret (R1 refuses it before this point). The file is serialised with keys in sorted order, so the same template writes the same bytes.

**Acceptance:**
- From the fixture template the environment file parses as {"env": {"LYS_FIXTURE_MODE": "fixture", "LYS_FIXTURE_TOKEN": "handle-fixture-0001"}} and holds no other member.
- Written while the process environment holds LYS_FIXTURE_TOKEN=fixture-secret-value-0001, the environment file holds 0 occurrences of the bytes fixture-secret-value-0001.
- Writing the environment file twice from the fixture template gives equal bytes.

**Files:**
- create: crates/lys-home/src/harness/claude_code/launch_env.rs

**Checklist:**
- C18 — A use-only secret is written as its handle and never its value, and a template marking a secret readable is refused naming secret_reader_unbuilt and SECRETS-002.

**Stories:**
- S10 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a secret a session may only use to reach it as a handle and never as its value, so that the credential never enters the session's process.

### R7: Add `lys-home render-launch`: write the launch files, print the report and the launch line, record the render

Add a subcommand `render-launch --home <dir> --session <id> --template <file> --uuid <uuid> --cwd <dir> --model <id> --version <claude code version> --out <dir>`; its arguments and logic live in crates/lys-home/src/harness/claude_code/launch.rs and cli.rs holds only the variant and its dispatch. WHEN run, THE SYSTEM SHALL, in this order: parse the template (R1); open the home and the session, taking its lock; refuse by path any of the five target files that already exists; store the template (R3); take the session head hash (R4); render the session with R4's render to <out>/<uuid>.jsonl with its loss account <out>/<uuid>.loss.json, taking the canon named by slots.transcript.canon when it is not null; write <out>/mcp.json (the mcp slot verbatim, keys sorted), <out>/env.json (R6) and <out>/instructions.md (the instructions slot's bytes); hash the five files; store the manifest block and append one template_render event with append_beside (R4, R5); and print one JSON report {command, template, session_head, uuid, files: [{path, sha256}], launch, event, manifest, render}. The launch member SHALL be one line: `claude --resume <out>/<uuid>.jsonl --fork-session --mcp-config <out>/mcp.json --settings <out>/env.json --append-system-prompt-file <out>/instructions.md` followed by the template's flags in order, arguments separated by one space, an argument holding a character outside letters, digits and `._/=:-` single-quoted for a POSIX shell. THE SYSTEM SHALL NOT run the launch line or any process (ADR-007), SHALL NOT print transcript, block, body, instructions, environment values or MCP contents, SHALL NOT write outside <out> and the home, and SHALL NOT write any file after a refusal. IF a required argument is missing, THEN THE SYSTEM SHALL exit 2 naming it (clap). IF the template is refused, THEN THE SYSTEM SHALL exit 1 with the refusal's message on stderr and write nothing. IF the event cannot be recorded, THEN THE SYSTEM SHALL exit 1 naming the failure and SHALL NOT print a report.

**Acceptance:**
- `lys-home render-launch` given every argument but --template exits 2 and its stderr contains `--template`.
- Given a fresh home holding the fixture session (session.jsonl, id `fixture`), the fixture template, --uuid 00000000-0000-4000-8000-000000000001, --cwd /fixture, --model claude-fixture, --version 2.1.283 and an empty --out, the command exits 0 and <out> holds exactly five files: 00000000-0000-4000-8000-000000000001.jsonl, 00000000-0000-4000-8000-000000000001.loss.json, mcp.json, env.json and instructions.md.
- The SHA-256 of each of those five files equals the value recorded for it as a constant in tests/launch_template.rs.
- Running it a second time with a second empty --out gives five files whose SHA-256 values equal the first run's pairwise, and a report whose session_head equals the first report's.
- The report's launch member equals `claude --resume <out>/00000000-0000-4000-8000-000000000001.jsonl --fork-session --mcp-config <out>/mcp.json --settings <out>/env.json --append-system-prompt-file <out>/instructions.md --strict-mcp-config` with <out> the given directory, and the test observes no child process (the command's only output is its stdout report).
- With LYS_FIXTURE_TOKEN=fixture-secret-value-0001 in the command's environment, a byte search of the five written files and of stdout for fixture-secret-value-0001 finds 0 matches, and env.json's env.LYS_FIXTURE_TOKEN is handle-fixture-0001.
- The report parses as JSON and contains no key named text, no key named content and no key named body, at any depth.
- After the two runs, the session reopens with head `e4`, customs_everywhere("lys.harness_event") returns two entries whose parentId is e4 and whose data.kind is template_render, and each event's data.detail.template equals the SHA-256 of the fixture template file.
- The template object templates/<hh>/<hash> in the home exists with the hash the report names as template.
- Run with the fixture template plus slots.voice = {}, the command exits 1, stderr contains `voice`, <out> holds 0 files, the home holds no templates directory, and the session file's bytes are unchanged.
- Run with the fixture template plus one readable secret, the command exits 1, stderr contains `secret_reader_unbuilt` and `SECRETS-002`, and <out> holds 0 files.
- Run with --out naming a directory that already holds env.json, the command exits 1, stderr names that env.json path, and the directory holds only env.json with its bytes unchanged.

**Files:**
- create: crates/lys-home/src/harness/claude_code/launch.rs
- create: crates/lys-home/tests/launch_template.rs
- create: crates/lys-home/tests/fixtures/launch/session.jsonl
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/README.md

**Checklist:**
- C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written.
- C16 — lys-home render-launch writes the rendered JSONL, its loss account, an MCP configuration file, an environment file and an appended-instructions file into one directory, and a second render of the same template and session writes files with identical SHA-256.
- C17 — The launch line in render-launch's report resumes the rendered file by path with --fork-session and the template's flags, and the tool never runs it.
- C18 — A use-only secret is written as its handle and never its value, and a template marking a secret readable is refused naming secret_reader_unbuilt and SECRETS-002.
- C19 — Every render appends one lys.harness_event of kind template_render beside the context path, naming the template hash, the session head hash and a manifest block of the written paths, and the head does not move.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session turned into a running Claude Code session from a template kept in my home, so that I start with my own record, tools, environment and instructions rather than a blank harness.
- S10 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a secret a session may only use to reach it as a handle and never as its value, so that the credential never enters the session's process.
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every launch recorded on the session with the template hash, the session head hash and the written paths, so that I can tell which template a session was launched with.

### R8: Measure the launch on the installed Claude Code and write it down

WHEN the launch is proved, THE SYSTEM's proof SHALL import the recorded session from PROOF-RESUME into a home, run render-launch on it with a proof template, and run the printed launch line once, from a working directory that is neither the out directory nor the session's cwd, on the Claude Code version `claude --version` reports on the proof machine; docs/design/home/PROOF-LAUNCH.md SHALL record that version, the proof template's hash, the written paths relative to the out directory with their SHA-256, the launch line with the out directory written as <out>, the rendered file's SHA-256 before and after the launch, where the continuation was written relative to ~/.claude/projects, whether the continuation's parent link names the rendered session id, the resume-check report, whether the appended instructions took effect under the proof template's --system-prompt-snapshot setting, whether a tool call in the launched session saw the environment file's handle, and whether --mcp-config was accepted. The proof SHALL NOT contain the session file, any transcript, block or body content, or a secret value, and the private session SHALL NOT be committed. The Claude Code profile's module doc in harness/claude_code/mod.rs SHALL state that a launch resumes with --fork-session because a bare resume writes onto <sessionId>.jsonl beside the passed file, with the version it was measured on.

**Acceptance:**
- docs/design/home/PROOF-LAUNCH.md names the Claude Code version string printed by `claude --version` on the proof run.
- PROOF-LAUNCH.md records the rendered file's SHA-256 before and after the launch, and the two are equal.
- PROOF-LAUNCH.md records the continuation's path relative to ~/.claude/projects, in the run directory's slug directory, and the path is not the rendered file's path.
- PROOF-LAUNCH.md records the resume-check report with repeated_tool_use_ids equal to 0.
- PROOF-LAUNCH.md records, for each of four questions, one answer from {yes, no} with the observation it rests on: the parent link naming the rendered session id, the appended instructions taking effect, the handle being visible to a tool call, and --mcp-config being accepted.
- A search of PROOF-LAUNCH.md for the text of any message of the proof session finds nothing: it holds hashes, paths, counts, flags and answers from {yes, no} only.
- git ls-files lists no .jsonl file outside crates/lys-home/tests/fixtures and canon/.

**Files:**
- create: docs/design/home/PROOF-LAUNCH.md
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C17 — The launch line in render-launch's report resumes the rendered file by path with --fork-session and the template's flags, and the tool never runs it.
- C20 — PROOF-LAUNCH.md records the launch measured on the installed Claude Code version: template hash, written paths and hashes, launch line, the rendered file unchanged, where the continuation landed; never transcript content.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session turned into a running Claude Code session from a template kept in my home, so that I start with my own record, tools, environment and instructions rather than a blank harness.

### R9: Record the whole ruling in the tree and re-render the cluster's markdown

Structure: the entry for steps 4 and 5 in the identity cluster's statement document named in this requirement's files (at line 181, the relayed rules for the home) gains the two items the relay omits, in the relayed register: the mapping also covers where the prior conversation goes, and it is delivered as files and runtime variables, with command mappings in text and never a transcript converter. The entry keeps its existing mark that it stands as relayed, for correction, and no other line of that document changes. docs/design/home/DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/HOME-002.md are re-rendered by the method's render-cluster.py from their JSON and are never edited by hand.

**Acceptance:**
- `git diff` of the identity statement document named in this requirement's files touches only the steps 4 and 5 entry, and that entry contains `where the prior conversation goes`, `runtime variables`, `in text` and `never a transcript converter` and the entry's first sentence, which marks it as relayed for correction, is byte-identical before and after.
- `sh scripts/design/gate.sh` exits 0 from the repository root.

**Files:**
- create: docs/design/home/briefs/HOME-002.md
- modify: docs/design/identity/STATEMENT-2026-09-22.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md

**Checklist:**
- C20 — PROOF-LAUNCH.md records the launch measured on the installed Claude Code version: template hash, written paths and hashes, launch line, the rendered file unchanged, where the continuation landed; never transcript content.

**Stories:**
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every launch recorded on the session with the template hash, the session head hash and the written paths, so that I can tell which template a session was launched with.

## Boundaries

- SHALL NOT run the launch line, spawn Claude Code or any other process; the line is printed in the report (ADR-007).
- SHALL NOT write a secret's value to any file, report, launch line, error or entry; a use-only secret appears only as its handle, and a readable secret is refused until SECRETS-002 builds the reader.
- SHALL NOT build, call or name a broker reader command in a launch line; the broker's own read belongs to the secrets rows.
- SHALL NOT add a harness other than Claude Code, a launch on another machine, a sandbox or VM target profile, or capture of the launched session into the home.
- SHALL NOT add a handover slot; HOME-001 R12 is not on main.
- SHALL NOT rewrite, truncate or write into any file under ~/.claude/projects, and SHALL NOT write to the rendered file after the render (CN1).
- SHALL NOT move the session head when recording a render; the event is a side leaf.
- SHALL NOT change the five existing lys.harness_event kinds, MAX_DATA_BYTES, Pi's grammar, or the reports of import, render, fewshot, ingest-call and resume-check.
- SHALL NOT commit the PROOF-RESUME session or any real transcript; the CI fixture is synthetic (CN3, P7).
- SHALL NOT add a dependency on manifold, the door, aion or any Norn crate (ADR-004, CN6).

## Verification

- From docs/: python3 $DS2_METHOD/scripts/validate.py design/home exits 0.
- From docs/: python3 $DS2_METHOD/scripts/check-coverage.py design/home exits 0.
- From the repository root: sh scripts/design/gate.sh exits 0.
- From the repository root: cargo fmt --all -- --check, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps exit 0.
- cargo test -p lys-home --test launch_template reports every test in the file run and passed, with a non-zero count.
- grep -rn 'Command::new\|process::Command' crates/lys-home/src/harness/claude_code/launch.rs crates/lys-home/src/harness/claude_code/launch_env.rs crates/lys-home/src/harness/claude_code/template.rs finds nothing.
- Every file under crates/lys-home/src is at most 500 lines of code excluding comments and blank lines, and cli.rs gains only the render-launch variant and its dispatch.


---
type: brief
id: HOME-003
cluster: home
title: Record what a session was given: a lys.given entry at render, listed and checked by hash
---

# HOME-003: Record what a session was given: a lys.given entry at render, listed and checked by hash

> **Cluster:** home
> **Depends on:** HOME-001, HOME-002
> **Blocked by:** The launch template card ct98Wv-2 (HOME-002: the render-launch subcommand, its template_render event, and the appended instructions file and MCP configuration it writes) is on card/home-002-launch-template (lys PR 14) and not yet on lys main; this brief is built on that branch, lands after it, and hooks into the render step that card places in crates/lys-home/src/harness/claude_code/launch.rs (render_launch), the file the design's structure names for it.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-013 — The context record is a lys.given custom entry of document hashes, never copies — The context record is one lys.given custom entry, appended after the render event, whose data is the harness name, the Claude Code version the load order was measured on, the kinds as two lists, resolved (claude_md_chain, user_claude_md, memory_index, appended_instructions, mcp_config, environment_names) and unlisted (claude_md_imports and claude_rules, which this entry does not list and a later entry at the first request records), the config directory as its path and its source (template or home), the documents in the measured order each as kind, path, byte length and SHA-256, and the names of the environment variables the template set. It is not a copy of each document into the block store, and not a content-bearing record, because the entry must hold no content under home P7 and CN3. It is unsigned and unencrypted now, and because it names hashes only, signing and encryption at rest can be added later without changing what is recorded.
> **Checklist:**
> - C21 — Every launch-template render appends one lys.given custom entry after the render event, listing the instruction documents Claude Code will load for the session's working directory and the files the render wrote, in the order measured on the named Claude Code version, each by kind, path, byte length and SHA-256, with the harness name and version, the kinds resolved, the kinds it leaves unlisted, and the environment variable names the template set; never a document's content and never a variable's value.
> - C22 — lys-home lists a session's given records with their documents, and checks a listed document against a file on disk by hash, answering matches or differs and never printing either file.
> - C23 — The Claude Code version on this Mac, the instruction load order and project slug rule measured from its own behaviour, and one real session render, are written in PROOF-GIVEN.md as paths, counts and hashes only.
> **Stories:**
> - S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.
> - S13 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

## Purpose

Today a render records the template hash, the session head hash and the written paths, and nothing records which instruction documents the rendered session was given, in what versions or order. This brief makes that record at render, the product's own act (ADR-007): one lys.given custom entry after the render event naming each document Claude Code will load, and each file the render wrote, by path, byte length and SHA-256 in the order measured on a named Claude Code version, with the config directory and how it was found, the environment variable names the template set, and the kinds it resolved and left unlisted, and never a document's content. It is the context record of step 4 in the home design, unsigned and unencrypted, shaped as hashes only so that signing (stage 6) and encryption at rest (stage 3) can be added later without changing what is recorded (ADR-012).

## Task

Build on lys main after ct98Wv-2 lands; start from its template subcommand and its render event. The load order below was measured, not assumed, on the Claude Code installed on the machine this brief was written on: `claude --version` answered 2.1.283. Three runs of `claude -p` were pointed at a local listener, with a fixture config directory (CLAUDE_CONFIG_DIR), fixture CLAUDE.md files, a memory index, `--append-system-prompt-file` and `--mcp-config`. Each file's first access time gave the order the harness reads the files, and the request it sent gave the order it places them in the context. Across kinds, the order was the same in all three runs: (1) the user CLAUDE.md at <config>/CLAUDE.md; (2) the appended instructions file; (3) the MCP configuration; (4) the CLAUDE.md chain, directory by directory from the outermost ancestor of the working directory down to the working directory itself, each directory contributing its CLAUDE.md, .claude/CLAUDE.md and CLAUDE.local.md when present; (5) the memory index at <config>/projects/<slug>/memory/MEMORY.md. The project slug is the working directory with every character that is not an ASCII letter or digit replaced by '-': '/', '.' and '_' each became '-', '-' stayed, and a memory directory named under the old '/'-only rule was never read. Within one directory, the read order of CLAUDE.md, .claude/CLAUDE.md and CLAUDE.local.md changed from run to run (the harness reads them concurrently), while the request always gave them as CLAUDE.md, .claude/CLAUDE.md, CLAUDE.local.md. The entry records the order the request gave, because it holds what reached the model and not the order the harness happened to read the files in: within one directory, CLAUDE.md, then .claude/CLAUDE.md, then CLAUDE.local.md. The config directory is the CLAUDE_CONFIG_DIR the template sets for the session; when the template sets none, it is HOME/.claude with HOME taken from the rendering process's environment (the root home P6 measured), and the entry's config_dir member records the path and which of the two it came from (source template or home). The rendering process's own CLAUDE_CONFIG_DIR is never read, since that process's environment is not the session's; a launch from another shell or machine is the launch template's to settle, by setting the variable. When the config directory is HOME/.claude and the working directory is under HOME, the file HOME/.claude/CLAUDE.md is both the user CLAUDE.md and the .claude/CLAUDE.md position of HOME on the chain. This was measured on 2.1.283 with HOME set to a fixture directory H, no CLAUDE_CONFIG_DIR, H/.claude/CLAUDE.md and H/w/CLAUDE.md present and the working directory H/w: the request gave H/.claude/CLAUDE.md exactly once, first, as the user's global instructions, and then H/w/CLAUDE.md. The entry lists it the same way: once, as user_claude_md, first, and not again in the chain. When the template sets CLAUDE_CONFIG_DIR to another directory, HOME/.claude/CLAUDE.md is not the user file and is listed as claude_md_chain at HOME's place on the chain. A document position the harness would read but finds absent is omitted from the documents list; the kinds member still names every kind this render resolved, so a reader sees what was looked for. The record is limited to the six kinds the card names (claude_md_chain, user_claude_md, memory_index, appended_instructions, mcp_config, environment_names), and the entry says so through its kinds member, which names those six as resolved and names claude_md_imports and claude_rules as unlisted, so a reader can tell those two are missing from this entry rather than absent from the session. Settings, hook output, plugin skills and agents, and output styles are later cards on this same entry shape. A document the render wrote (the appended instructions and the MCP configuration) is named by its kind and its path relative to the render's out directory. Every other document is named by its absolute path. Two renders of a template that writes no per-render bytes therefore give equal document lists. A template that writes a session id into a rendered file gives a differing hash for that one document, and the record shows it. cli.rs has 398 code lines against the 500-line limit, so the two new subcommands live in src/cli/given.rs, declared from cli.rs, and cli.rs keeps only the enum arms that call them. The render wiring goes into the template subcommand's render arm in cli.rs; if ct98Wv-2 places its render step in another file, the scout names that file and the design's structure gains it before the wiring is written. projects_slug moves out of harness/claude_code/mod.rs into paths.rs and is re-exported, so mod.rs holds declarations and re-exports only. render.rs keeps calling projects_slug and needs no edit. In scope: the slug fix, the resolution, the entry, the render wiring, the two subcommands, the fixture tests, RECORD.md and the crate README, and PROOF-GIVEN.md with one real render. Out of scope: signing or anchoring the record (stage 6); encryption at rest (stage 3); documents a session reads later with its own tools; files the harness puts in the request through an @-import or from .claude/rules. The entry made at render records what the render placed and what the harness reads by its documented rule, the three files per directory, and never parses a document to find an import. What actually reached the model, imports and rules included, is recorded by a second lys.given entry appended at the first request by the HOME-001 proxy's capture, from the request as the harness resolved it; that entry belongs to the capture's card, a later unit on this same entry shape, and not to this brief; harnesses other than Claude Code; any change to the render event ct98Wv-2 writes. given-check answers as diff does, so it can stand in a script: exit status 0 when the answer is matches, 1 when it is differs, and 2 on an error such as a missing argument or an unreadable record. Differs is an answer, not an error, and the status tells the two apart; the printed answer is the same either way. The fixture template the acceptance renders is ct98Wv-2's. If that card lands none, the integration test writes one in its own temporary directory, in that card's template format, that writes the appended instructions and MCP configuration with no per-render bytes. The fixture working directory is a fresh temporary directory whose ancestors hold no CLAUDE.md. If the machine running the test has one on the chain, the test's expected list is wrong for that machine, and the scout says so; the resolver is not bent to hide it.

## Requirements

### R1: Resolve the project slug as Claude Code 2.1.283 does

Move projects_slug from harness/claude_code/mod.rs into harness/claude_code/paths.rs, re-exported from mod.rs so every caller keeps its path. WHEN given a working directory, THE SYSTEM SHALL return it with every character that is not an ASCII letter or ASCII digit replaced by '-', and SHALL keep ASCII letters, digits and '-' as they are. THE SYSTEM SHALL NOT keep '.' or '_' in a slug, SHALL NOT collapse consecutive '-', and SHALL NOT read or write any file to compute a slug. mod.rs SHALL hold only module declarations, re-exports, constants it already holds, and module docs.

**Acceptance:**
- projects_slug("/home/u/.aion/clones/w") returns "-home-u--aion-clones-w".
- projects_slug("/tmp/x_y/p-q.r/w") returns "-tmp-x-y-p-q-r-w".
- projects_slug("/srv/plain") returns "-srv-plain", unchanged from the rule before this brief.
- harness/claude_code/mod.rs contains no `fn` item after the change.
- The claude_code render tests that passed before the change pass unchanged.

**Files:**
- create: crates/lys-home/src/harness/claude_code/paths.rs
- create: crates/lys-home/src/harness/claude_code/paths_tests.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C23 — The Claude Code version on this Mac, the instruction load order and project slug rule measured from its own behaviour, and one real session render, are written in PROOF-GIVEN.md as paths, counts and hashes only.

**Stories:**
- S13 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

### R2: Resolve the documents a Claude Code session will be given, in the measured order

WHEN a render resolves the given documents for a working directory, a config directory and the files the render wrote, THE SYSTEM SHALL return one document per file that exists, in this order: the user CLAUDE.md at <config>/CLAUDE.md (kind user_claude_md); the appended instructions file the render wrote (kind appended_instructions); the MCP configuration the render wrote (kind mcp_config); for each directory from the outermost ancestor of the working directory down to the working directory, its CLAUDE.md, .claude/CLAUDE.md and CLAUDE.local.md (kind claude_md_chain); then the memory index at <config>/projects/<projects_slug(cwd)>/memory/MEMORY.md (kind memory_index). Each document SHALL carry its kind, its path, its byte length and the lowercase hex SHA-256 of its bytes, all taken from one read of the file. A document the render wrote SHALL carry its path relative to the render's out directory; every other document SHALL carry its absolute path. IF a position's file does not exist, THEN THE SYSTEM SHALL omit it. IF a file exists and cannot be read, THEN THE SYSTEM SHALL fail by name with the path and the operation, and SHALL NOT omit it, since a document the harness would read is being dropped. THE SYSTEM SHALL NOT keep, return, log or put in an error any byte of a document's content. It SHALL NOT write any file under the config directory or the working directory. It SHALL NOT parse any document's content, and so SHALL NOT find an @-imported file or a .claude/rules file by reading a document. One directory's files SHALL be listed together, as CLAUDE.md, then .claude/CLAUDE.md, then CLAUDE.local.md, the order the harness's request gives them in; it SHALL NOT list them in the order the files happen to be read. The config directory SHALL be the CLAUDE_CONFIG_DIR the template sets for the session. IF the template sets none, THEN the config directory SHALL be HOME/.claude, with HOME taken from the rendering process's environment, and the resolution SHALL return the config directory's path with its source: template in the first case, home in the second. THE SYSTEM SHALL NOT read the rendering process's CLAUDE_CONFIG_DIR. IF <config>/CLAUDE.md is also a position on the chain (the config directory is D/.claude for a directory D from the outermost ancestor down to the working directory), THEN THE SYSTEM SHALL list that file once, as user_claude_md in the first position, as the harness's request gives it, and SHALL NOT list it again as claude_md_chain. IF the config directory is not D/.claude, THEN D/.claude/CLAUDE.md SHALL be listed as claude_md_chain at D's place on the chain. The measured harness version SHALL be a constant (2.1.283 as measured when this brief was written; R8 re-measures it), named beside the order in the module docs.

**Acceptance:**
- For a temporary working directory W holding CLAUDE.md, a config directory C holding projects/<projects_slug(W)>/memory/MEMORY.md and no CLAUDE.md, and an out directory O holding the render's appended instructions file and MCP configuration, the resolved kinds are, in order: appended_instructions, mcp_config, claude_md_chain, memory_index.
- In that case the appended_instructions and mcp_config documents carry paths relative to O, and the claude_md_chain and memory_index documents carry absolute paths equal to W/CLAUDE.md and the MEMORY.md path under C.
- Each resolved document's length equals the file's byte length, and its sha256 equals the SHA-256 of the file's bytes, as computed independently in the test with the sha2 crate.
- Adding C/CLAUDE.md puts one user_claude_md document first, ahead of appended_instructions.
- For W = A/w with A/CLAUDE.md and W/CLAUDE.md both present, A/CLAUDE.md is listed before W/CLAUDE.md.
- For W holding CLAUDE.local.md, .claude/CLAUDE.md and CLAUDE.md, each created in that order, the resolved claude_md_chain documents are exactly W/CLAUDE.md, W/.claude/CLAUDE.md and W/CLAUDE.local.md, in that order.
- With no MEMORY.md under C, the result holds no memory_index document and the call succeeds.
- With the template setting CLAUDE_CONFIG_DIR to C, the memory index under C is listed and the config directory is returned as C with source template. With the template setting none, HOME set to H and the rendering process's CLAUDE_CONFIG_DIR set to C, the memory index under H/.claude is listed, none under C is listed, and the config directory is returned as H/.claude with source home.
- With the template setting no CLAUDE_CONFIG_DIR, HOME set to H, W = H/w, and H/.claude/CLAUDE.md and W/CLAUDE.md both present, the resolved documents are exactly two, in order: H/.claude/CLAUDE.md with kind user_claude_md, then W/CLAUDE.md with kind claude_md_chain; no claude_md_chain document has the path H/.claude/CLAUDE.md.
- With the template setting CLAUDE_CONFIG_DIR to C (holding no CLAUDE.md), HOME set to H, W = H/w, and H/.claude/CLAUDE.md and W/CLAUDE.md both present, the resolved documents are exactly two, in order: H/.claude/CLAUDE.md with kind claude_md_chain, then W/CLAUDE.md with kind claude_md_chain; no document has the kind user_claude_md.
- A MEMORY.md placed under projects/<W with only '/' replaced by '-'>/memory/ for a W containing a '.' is not listed.
- A W/CLAUDE.md with its read permission removed fails the resolution with an error naming W/CLAUDE.md, and the error text holds no line of the file.
- The config directory's and the working directory's file listings (names, lengths and modification times) are identical before and after resolution.

**Files:**
- create: crates/lys-home/src/harness/claude_code/given.rs
- create: crates/lys-home/src/harness/claude_code/given_tests.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C21 — Every launch-template render appends one lys.given custom entry after the render event, listing the instruction documents Claude Code will load for the session's working directory and the files the render wrote, in the order measured on the named Claude Code version, each by kind, path, byte length and SHA-256, with the harness name and version, the kinds resolved, the kinds it leaves unlisted, and the environment variable names the template set; never a document's content and never a variable's value.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.
- S13 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

### R3: Define the lys.given entry and append and read it inside Pi's custom entry

Add the custom type constant lys.given beside the four lys custom types. Its data SHALL be exactly: harness ("claude-code"), harness_version (the measured version), kinds (an object with two members: resolved, the list claude_md_chain, user_claude_md, memory_index, appended_instructions, mcp_config, environment_names, in that order; and unlisted, the list claude_md_imports, claude_rules, in that order), config_dir (an object with two members: path, the config directory R2 resolved, and source, template or home as R2 returned it), documents (the ordered list from R2, each {kind, path, length, sha256}) and environment (the names of the environment variables the template set for the session, sorted, since the template's env slot is parsed into a sorted map and the environment file writes it sorted). WHEN a given record is appended, THE SYSTEM SHALL write it with Session::append as a custom entry whose customType is lys.given, so its parentId is the head at that moment. It SHALL read given records back with customs_everywhere("lys.given") in file order. THE SYSTEM SHALL NOT add a field to Pi's header or to any entry outside custom.data. It SHALL NOT carry any document content or any environment variable value. It SHALL NOT sign or encrypt the entry. It SHALL NOT record a variable name the template did not set. It SHALL NOT list a document under claude_md_imports or claude_rules, and SHALL NOT drop either name from unlisted.

**Acceptance:**
- A given record appended to a session and read back with customs_everywhere("lys.given") deserialises equal to the value appended.
- The serialised data object of a lys.given entry has exactly the keys config_dir, documents, environment, harness, harness_version and kinds, config_dir has exactly the keys path and source,, and each document object has exactly the keys kind, length, path and sha256.
- A given record built from a resolution whose config directory came from the template has config_dir.source equal to template, and one built from a resolution that fell back to HOME has config_dir.source equal to home and config_dir.path equal to HOME/.claude.
- kinds in every lys.given entry equals {"resolved": ["claude_md_chain", "user_claude_md", "memory_index", "appended_instructions", "mcp_config", "environment_names"], "unlisted": ["claude_md_imports", "claude_rules"]}, and no document in the entry has the kind claude_md_imports or claude_rules.
- Given a template environment of FOO_A=secret-value-1 and BAR_B=secret-value-2, environment equals ["BAR_B", "FOO_A"], sorted, and the serialised entry contains neither secret-value-1 nor secret-value-2.
- The entry's line in the session file has the top-level keys type, id, parentId, timestamp, customType and data, and no other.

**Files:**
- create: crates/lys-home/src/record/given.rs
- create: crates/lys-home/src/record/given_tests.rs
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C21 — Every launch-template render appends one lys.given custom entry after the render event, listing the instruction documents Claude Code will load for the session's working directory and the files the render wrote, in the order measured on the named Claude Code version, each by kind, path, byte length and SHA-256, with the harness name and version, the kinds resolved, the kinds it leaves unlisted, and the environment variable names the template set; never a document's content and never a variable's value.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

### R4: Append the given record after the render event on every template render

WHEN the template subcommand renders a Claude Code session and has appended its render event, THE SYSTEM SHALL resolve the given documents (R2) for the rendered session's working directory, config directory and out directory, and append exactly one lys.given entry (R3) as the child of that render event. THE SYSTEM SHALL NOT alter, reorder or re-serialise the render event. IF resolution fails, THEN THE SYSTEM SHALL fail the render by name, and SHALL NOT append a lys.given entry holding a partial documents list. THE SYSTEM SHALL NOT run the rendered command (ADR-007).

**Acceptance:**
- After one render of the fixture template, the session holds exactly one lys.given entry, and its parentId equals the id of the render event that render appended.
- The tests ct98Wv-2 lands for its render event pass unchanged with this brief's code in the render path.
- A render whose working directory holds an unreadable CLAUDE.md exits non-zero, names the path, and leaves the session with no lys.given entry for that render.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/launch.rs

**Checklist:**
- C21 — Every launch-template render appends one lys.given custom entry after the render event, listing the instruction documents Claude Code will load for the session's working directory and the files the render wrote, in the order measured on the named Claude Code version, each by kind, path, byte length and SHA-256, with the harness name and version, the kinds resolved, the kinds it leaves unlisted, and the environment variable names the template set; never a document's content and never a variable's value.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

### R5: List a session's given records and check a listed document against a file by hash

Add the subcommands given and given-check, implemented in src/cli/given.rs and dispatched from cli.rs. WHEN given is run on a session, THE SYSTEM SHALL report every lys.given entry in file order with its entry id, harness, harness_version, the config directory with its source, the kinds line naming the resolved kinds and the unlisted kinds, the environment names, and each document's kind, path, length and sha256. WHEN given-check is run with a session, a given entry id, a document path as listed, and a file on disk, THE SYSTEM SHALL hash the file and report the answer matches when its SHA-256 and length equal the listed document's, and differs otherwise. WHEN the answer is matches, THE SYSTEM SHALL exit with status 0; WHEN the answer is differs, THE SYSTEM SHALL exit with status 1. IF an argument is missing, the session or the file cannot be read, or the entry id or the listed path is not in the session, THEN THE SYSTEM SHALL fail by name with the id, the path or the argument concerned and exit with status 2. THE SYSTEM SHALL NOT exit 0 on differs, SHALL NOT exit 1 on an error, and SHALL NOT print a different answer for either status. THE SYSTEM SHALL NOT print, log or put in an error any byte of either file.

**Acceptance:**
- given on a session holding two lys.given entries reports two records in file order, each with its entry id and its full documents list.
- given's report for a record carries a kinds line naming the six resolved kinds and naming claude_md_imports and claude_rules as unlisted, the config directory's path and source equal to the entry's config_dir, and environment equal to the entry's names.
- given-check with the fixture CLAUDE.md as recorded reports the answer matches and exits with status 0.
- given-check with that CLAUDE.md after one byte is changed reports the answer differs and exits with status 1.
- given-check naming an entry id not in the session exits with status 2 and an error naming that id.
- given-check naming a document path not listed in the entry exits with status 2 and an error naming that path.
- given-check run with no file argument exits with status 2.
- given-check naming a file on disk that does not exist exits with status 2 and an error naming that file's path.
- cli.rs has at most 500 code lines, excluding comments and blank lines, after the change.

**Files:**
- create: crates/lys-home/src/cli/given.rs
- modify: crates/lys-home/src/cli.rs

**Checklist:**
- C22 — lys-home lists a session's given records with their documents, and checks a listed document against a file on disk by hash, answering matches or differs and never printing either file.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

### R6: Prove the acceptance end to end on the fixture template

Add an integration test that renders the fixture template for a fixture working directory holding a CLAUDE.md with a fixed fixture sentence and a memory index under the fixture config directory, then checks every clause of the card's acceptance. The fixture template SHALL set CLAUDE_CONFIG_DIR to the fixture config directory C for the rendered session; if ct98Wv-2's fixture template sets none, the test writes its own as the task describes, setting it. The test SHALL run the render with HOME set to a fresh temporary directory holding no .claude directory and with CLAUDE_CONFIG_DIR removed from the rendering process's environment, so the resolved documents cannot depend on the configuration of the machine running the test. THE SYSTEM SHALL NOT use a fixture sentence as, or inside, a test name. Tests SHALL count what fired: each assertion over a documents list first asserts the list's length.

**Acceptance:**
- The first render's lys.given entry has 4 documents, whose kinds are appended_instructions, mcp_config, claude_md_chain and memory_index in that order, and whose lengths and sha256 values equal those of the four files on disk.
- In the first render's entry, config_dir equals {path: C, source: template}, the memory_index document's path starts with C, environment contains CLAUDE_CONFIG_DIR, and no document has the kind user_claude_md.
- A second render of the same fixture gives a second lys.given entry whose documents list is equal to the first's in every kind, path, length and sha256, with the render-written documents compared by their out-directory-relative paths.
- After one byte of the fixture CLAUDE.md is changed (same length), a third render's entry differs from the second's in exactly one field: the claude_md_chain document's sha256. All other fields of all 4 documents are equal.
- given-check reports matches for the fixture CLAUDE.md against the second entry before the change, and differs against the second entry after it.
- A search for the fixture sentence finds 0 occurrences in the lys.given entries' lines of the session file, in the serialised given report, and in the serialised given-check reports.
- The session file's lines each parse as JSON, with the header first and every entry carrying id, parentId and timestamp.

**Files:**
- create: crates/lys-home/tests/given_record.rs

**Checklist:**
- C21 — Every launch-template render appends one lys.given custom entry after the render event, listing the instruction documents Claude Code will load for the session's working directory and the files the render wrote, in the order measured on the named Claude Code version, each by kind, path, byte length and SHA-256, with the harness name and version, the kinds resolved, the kinds it leaves unlisted, and the environment variable names the template set; never a document's content and never a variable's value.
- C22 — lys-home lists a session's given records with their documents, and checks a listed document against a file on disk by hash, answering matches or differs and never printing either file.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

### R7: Write lys.given into RECORD.md and the crate README

Add lys.given to the lys custom entries in RECORD.md: its data keys, the config_dir member and its two sources (template, home), the six resolved kinds and the two unlisted kinds (claude_md_imports and claude_rules, recorded by a later entry at the first request), the measured order, how a written document's path is relative to the out directory, that absent documents are omitted, and that the entry is unsigned and unencrypted and names hashes only. Add the lys.given row to the crate README's custom-type table, and add the given and given-check subcommands to it. Neither document SHALL quote any instruction document's content.

**Acceptance:**
- RECORD.md's 'The lys custom entries' section has a lys.given item naming the keys harness, harness_version, kinds, config_dir, documents and environment, the config_dir members path and source, the kinds members resolved and unlisted, the unlisted kinds claude_md_imports and claude_rules, and the document keys kind, path, length and sha256.
- The README's custom-type table has a lys.given row, and the README names the given and given-check subcommands.

**Files:**
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/README.md

**Checklist:**
- C21 — Every launch-template render appends one lys.given custom entry after the render event, listing the instruction documents Claude Code will load for the session's working directory and the files the render wrote, in the order measured on the named Claude Code version, each by kind, path, byte length and SHA-256, with the harness name and version, the kinds resolved, the kinds it leaves unlisted, and the environment variable names the template set; never a document's content and never a variable's value.
- C22 — lys-home lists a session's given records with their documents, and checks a listed document against a file on disk by hash, answering matches or differs and never printing either file.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

### R8: Record the measurement and one real render in PROOF-GIVEN.md

Write PROOF-GIVEN.md with: the Claude Code version answered by `claude --version` on this Mac, the machine whose Claude Code version the task names; the method and result of measuring the load order and slug rule from the harness's own behaviour (the order in the task, re-measured on that version); and one real session render on this Mac recorded through the template subcommand as the lys.given entry's kinds, document count, environment-name count, and each document's kind, path, length and sha256. IF the version measured differs from 2.1.283, THEN the proof SHALL say so and give the order measured on it, and the constant in R2 SHALL be that version. The proof SHALL state how many documents were probed for content by the check below and how many were skipped for having no line long enough. THE SYSTEM SHALL NOT put any document content or any environment variable value in the proof.

**Acceptance:**
- PROOF-GIVEN.md names the Claude Code version, and the order of kinds it measured on that version.
- PROOF-GIVEN.md holds one real render's lys.given entry id, its document count, its environment-name count, and one line per document with kind, path, length and a 64-hex-character sha256.
- For each document the proof lists, the probe is its longest line after trimming leading and trailing whitespace, taken only when that line is at least 40 characters long; a document with no such line is skipped. A search of PROOF-GIVEN.md for each probe finds 0 occurrences, the number of documents probed plus the number skipped equals the document count the proof states, and the number probed is at least 1.

**Files:**
- create: docs/design/home/PROOF-GIVEN.md

**Checklist:**
- C23 — The Claude Code version on this Mac, the instruction load order and project slug rule measured from its own behaviour, and one real session render, are written in PROOF-GIVEN.md as paths, counts and hashes only.

**Stories:**
- S13 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

## Boundaries

- No signing, anchoring or hashing into a lys log of the given record (stage 6), and no encryption at rest (stage 3).
- No document content and no environment variable value in the entry, a report, a log line, an error, a test name or a proof document; paths, lengths, hashes, counts and variable names only.
- No field added to Pi's grammar; lys.given rides inside a custom entry only.
- The render event ct98Wv-2 writes is not altered, reordered or re-serialised.
- No file under the config directory or the working directory is written; resolution only reads.
- No harness other than Claude Code, and no recording of documents a session reads later with its own tools.
- No settings file, hook output, plugin skill or agent listing, or output style is recorded; those are later cards on the same entry shape.
- No document's content is parsed: no @-imported file or .claude/rules file is found by reading a document. Those files are named as unlisted kinds here, and recorded by the second lys.given entry the HOME-001 proxy's capture appends at the first request, which is a later unit.
- No Norn crate is depended on and no Norn code is copied.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- python3 scripts/design/validate.py docs/design/home exits 0.
- python3 scripts/design/check-coverage.py docs/design/home exits 0.
- cargo fmt --all -- --check, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps exit 0.
- sh scripts/design/gate.sh exits 0.
- grep -n 'fn ' crates/lys-home/src/harness/claude_code/mod.rs returns nothing.
- grep -rn 'norn' crates/lys-home/Cargo.toml returns nothing.
- The fixture session file from the R6 test, written out in the proof step, parses with Pi's parseSessionEntries at 3d5cbe98 through node, and its entry count equals the count lys-home reports (the command is written in PROOF-GIVEN.md).


---
type: brief
id: HOME-004
cluster: home
title: Translate a Claude Code session in the home into a Codex 0.156.0 rollout with a loss account beside it
---

# HOME-004: Translate a Claude Code session in the home into a Codex 0.156.0 rollout with a loss account beside it

> **Cluster:** home
> **Depends on:** HOME-001, HOME-002
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-014 — A translation to another harness is a per-target, per-version renderer of our own with a loss account, never that harness's importer — A translation to another harness is a renderer of our own, one per target harness and per measured version, over the home's context path: it carries every text part, tool call and tool result whole as the target's own items, carries readable thinking as text and drops opaque blocks by hash (home P3), announces itself in band as a translated context naming its source session and head hash, writes a JSON loss account beside its output listing kept, changed and lost by entry id and block hash, and is recorded on the session as a side leaf. It is not the target harness's own importer, adopted, wrapped or called, because that importer is a compaction that loses what the home keeps and cannot say what it lost; and it is not one renderer for every version, because a harness's file shape is a per-version measurement (home P6).
> **Checklist:**
> - C24 — lys-home render-codex takes a home, a session, a target directory that is a Codex sessions root, and a Codex version; renders only for the measured Codex 0.156.0 and refuses any other version naming the version asked for and the one measured; walks the context path the Claude Code render walks; and writes one rollout under that root's dated directories in the shape Codex 0.156.0 writes for its own threads, measured from files that version wrote, with nothing written when it refuses.
> - C25 — Every text part, tool call and tool result on the context path is carried whole as Codex's own message, function_call and function_call_output items, paired by call id, never clipped or noted; readable thinking is carried as text; an image part whose source is base64 is carried as Codex's input_image item, and any other image source is named lost, never fetched; opaque thinking is dropped and named by hash; a compaction becomes Codex's own compaction item where 0.156.0 writes one with readable text, measured, and otherwise a user message that says it is a compaction summary.
> - C26 — The rendered thread announces itself in band as a translated context with the source session id and the source head hash.
> - C27 — A JSON loss account beside the rollout lists, by entry id and the importer's block hash, what was kept, what changed shape (before and after kinds, and how), and what was lost and why: sidechains, harness events, lys entries, and any part kind Codex has no item for.
> - C28 — The report is JSON with the rollout and account paths, entry and block counts and the loss counts, never content; an existing path is refused by name with nothing written; each translation is recorded on the session as a codex_translation side leaf naming both files by path and SHA-256 and the Codex version; the Claude Code render of the same session is unchanged, checked by hash.
> - C29 — A fixture session imported from a fixture Claude Code file renders to a rollout whose items match a fixture rollout recorded from Codex 0.156.0, apart from ids and timestamps, over the item kinds the render can produce, with the kinds Codex writes that no source record stands behind named and counted; a tool result longer than Codex's importer clips is carried whole.
> - C30 — PROOF-TRANSLATE.md records Codex 0.156.0 resuming the rendered rollout in an isolated Codex home and answering from its content, in hashes, counts and paths only.
> **Stories:**
> - S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose session was imported from Claude Code, I want it rendered as a Codex rollout that Codex resumes, with every text part, tool call and tool result carried whole and the thread announcing itself as a translation, so that I can continue on another harness and never mistake the fork for the original.
> - S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a loss account beside every translation naming by entry id and block hash what was kept, what changed shape and what was lost and why, and the resume measured on a named Codex version, so that nobody claims a faithful translation that was not measured.

## Purpose

Translation to another harness is designed and not proved. This brief proves the first one, for one named pair, Claude Code to Codex 0.156.0, and nothing above it assumes it. A lys-home subcommand renders a home session that was imported from Claude Code as a Codex rollout in the shape Codex 0.156.0 writes for its own threads, measured from files that version wrote and never assumed from a source checkout. Every text part, tool call and tool result is carried whole, readable thinking is carried as text and opaque blocks are dropped by hash (home P3), the rollout announces itself in band as a translated context, and a JSON loss account beside it lists by entry id and block hash what was kept, what changed shape and how, and what was lost and why. It improves on Codex's own importer of Claude Code files, which is a compaction and not a copy, and neither adopts nor calls it (ADR-014, ADR-004). Each translation is recorded on the home session as a side leaf the way a template render is (ADR-012), and the proof is a real resume by Codex 0.156.0 in an isolated Codex home.

## Task

Start by recording and measuring (R1): nothing about the rollout's shape is taken from a Codex source checkout or from a rollout another Codex version wrote. The machine that builds this brief has two Codex binaries, and the one that comes first on PATH is an older version, so every Codex run in this brief names the binary that answers `--version` with `codex-cli 0.156.0` by its absolute path, and the proof names which binary it ran. Every Codex run in this brief uses an isolated Codex home made under a temporary directory (CODEX_HOME), with its own sessions tree and thread index, never the user's own Codex home; the proof calls it <codex home>. The subcommand is `lys-home render-codex --home <home> --session <id> --out <sessions root> --codex-version <version>`. It takes exactly those four arguments. It does not take a canon: the Claude Code render's optional canon is not placed before a Codex translation. The only version it renders for is 0.156.0; any other version, 0.156.1 included, is refused naming the version asked for and the version the shape was measured on, and a later card adds a version when it is measured. --out is a Codex sessions root, the directory Codex keeps under its home as sessions/: the render creates the dated directories beneath it and writes the rollout at <out>/<YYYY>/<MM>/<DD>/rollout-<YYYY>-<MM>-<DD>T<hh>-<mm>-<ss>-<thread id>.jsonl, the date and time being the session_meta timestamp (the session header's) in local time, as Codex 0.156.0 names its own files (measured on the survey machine: a session_meta timestamp of 2026-09-04T21:50Z sits at 2026/09/05/rollout-2026-09-05T07-50-44-<thread id>.jsonl in a zone ten hours ahead of UTC), and the loss account beside it at the same path with `.jsonl` replaced by `.loss.json`. The proof passes <codex home>/sessions as --out. The file exists to be read by Codex on the machine it is rendered on, so the path is in that machine's local time, and determinism comes from the time zone being an explicit input: the subcommand takes the zone from the environment it runs in (the TZ variable when it is set, otherwise the system's configured zone), resolved to its IANA name, and passes that name to the translation; the translation record names the zone it used; and the same session at the same head in the same zone renders the same path. A zone with no IANA name is refused by name, since the record could not say which zone it used. Only the file and directory names are local; every timestamp inside the rollout stays in UTC, as Codex writes them. The session_meta originator is `lys-home`, so the thread is marked as translated in its metadata as well as in band; no other value is written, and if Codex 0.156.0 refuses to resume a rollout with that originator the build stops and names the refusal rather than write another value. Measured on the survey machine with Codex 0.156.0, in a fresh isolated Codex home: a rollout placed under its sessions tree with no thread index written by hand is found by `codex exec resume <thread id>`, Codex's own scan adding the thread-index row; a rollout holding only a session_meta line and response_item lines is resumed to the model request; and a `.loss.json` file beside a rollout does not stop Codex finding it. So the render writes no thread index, and R1 measures this again on the recorded fixture before any render code is written. If R1 measures instead that Codex 0.156.0 finds a rollout only through a thread-index row, the render writes that row itself into the Codex home the sessions root sits in (the parent directory of --out), in the row shape R1 measured on 0.156.0, every value derived from the source so the same session writes the same row, and the loss account records that the row was written and why, as a changed line (R6). If that row holds a value the render cannot derive, or writing it would need a dependency that is not pure Rust, R1 names the value, the render writes no row, and the proof adopts the rollout through Codex 0.156.0's own command that creates a thread and adopts a rollout, recording which road it took. Only if neither road resumes does the build stop and report, with the SHA-256 of every byte it wrote. No thread-index row is ever written by hand, and none outside the given Codex home. It walks Session::context_path(), the same path the Claude Code render walks: that path already begins at the last compaction boundary and carries that boundary's summary. A Claude Code session's meta records were imported as ordinary messages, so their text parts are carried whole like every other text part (Codex's importer drops them; this render does not). Sidechains hang off the context path under a label naming the agent, so they are not rendered and the loss account names each sidechain entry, and each part a sidechain message holds by its block hash, as lost. Harness events, labels and every lys custom entry (lys.harness_event, lys.call, lys.authored, lys.inherited, lys.given, and any later lys type such as a lantern) are not rendered and are named as lost, a harness event by the hash of the record block it names. A Claude tool call becomes Codex's function_call item and its result Codex's function_call_output item, paired by call_id; the Claude tool_use id is carried as the call_id, the tool name is carried whole, and the input is carried as the arguments string, the compact JSON serialisation of the stored input value (serde_json::to_string), whatever JSON value it is. The function pair is chosen over custom_tool_call because a Claude tool input is a JSON value, which a function_call's JSON arguments string carries whole and reversibly, while a custom tool's input is free text for a grammar. An image part is carried when Codex has an item for it: an image part whose source is base64 becomes Codex's input_image content item, in a user message and inside a tool result alike; an image part whose source is anything other than base64 (url, for example) is never fetched and is accounted lost with the reason unfetched_image:<source type>:<part index>, the part index being its position in the content array that holds it, and no byte of the source value is printed or written. The render reads nothing beyond the session file and the block store. A compaction on the context path becomes Codex's own compaction item only where 0.156.0 writes one that carries readable text. Measured on the survey machine, every compaction item Codex wrote there, across seven versions, carries only encrypted_content, which nothing in a home session can produce, and the one 0.156.0 rollout there holds none; so a compaction becomes a user message whose single text is the line `<COMPACTION SUMMARY>`, a newline, and the summary whole, accounted as changed. R1 measures whether 0.156.0 writes a compaction item with readable text, and if it does the build stops and reports, since this brief is written on the measurement that it does not. The in-band marker follows the precedent measured from the rollouts Codex's importer wrote on the survey machine: their <EXTERNAL SESSION IMPORTED> marker is an agent message after the last imported item. Since that marker was a display event the model never read, this marker is a response_item the model reads: an assistant message after the last mapped item whose single text is `<TRANSLATED CONTEXT from="claude-code" to="codex 0.156.0" source_session="<source session id>" source_head="<source head hash>">`, the source head hash being Session::head_hash() at render. No template takes part in a Codex render, so the marker carries no template hash. A block hash in the loss account is the key of the block HOME-001's importer stored: the SHA-256 of serde_json::to_vec of the Claude-shaped source part, with keys in the sorted order serde_json writes without preserve_order, which is how the importer stored it. Measured over the Claude Code files written on the survey machine in the week before this brief, a stored part has one of these key sets: text {type, text}; thinking {type, thinking, signature}; tool_use {type, id, name, input, caller} with caller {"type":"direct"} (all but 346 of 317,975) and {type, id, name, input}; tool_result {type, tool_use_id, content, is_error} and {type, tool_use_id, content}, the content a string or an array of text, image, tool_reference and document items; image {type, source} with source {type: base64, media_type, data}; and a user message's string content, stored as {type: text, text}. The importer's Pi part drops caller, so the part is recovered from the Pi part by trying, in this order, each candidate of the inverse of the importer's mapping and taking the first the home's block store holds: text to {type, text}; an image part, or any part of a kind the importer kept verbatim, to itself; thinking to {type, thinking, signature} when it has a signature and {type, thinking} when not; redacted thinking to {type: redacted_thinking, data}; toolCall to {type: tool_use, id, name, input, caller: {type: direct}} and then without caller; a toolResult message to {type: tool_result, tool_use_id, content, is_error}, trying the content as a single string (when it is exactly one text part) and then as the stored array, and is_error with its value and then, when false, absent. The loss account never names the SHA-256 of a re-serialised Pi part, since nobody can find that hash in blocks/. Thread ids, item ids, turn ids and timestamps are derived from the source, never random and never the clock, so the same session at the same head renders the same bytes. The rollout is written in the item kinds and fields R1 measures, and the render writes only what a Claude Code record is the source of: no developer instructions, no environment context, no reasoning item with encrypted content, no base_instructions, and no line kind other than session_meta and response_item. The fixture match compares response_item payloads only, since other line kinds carry the thread's runtime, and of those only the item kinds the render can produce; it leaves out, by named kind, a message of role developer, a user message whose passthrough content_item_kinds is ["environments.environment_context"], and a reasoning item carrying encrypted_content, and any other kind in the fixture that the render does not produce fails the match. Codex's importer clipped tool input at 2,000 characters and tool results at 4,000 as read from a source checkout, not from 0.156.0; the second fixture carries a 16,384-character tool result and a tool input whose serialisation is 8,192 characters, well above both, and each is checked by length and SHA-256. Each translation is recorded on the home session as a lys.harness_event side leaf of kind codex_translation naming the rollout and the loss account by path and SHA-256 and the Codex version, the way a template render is recorded; the files themselves live under --out. In scope: the codex harness module, the subcommand, the translation side leaf, the two Claude Code fixtures and the recorded rollout fixture, the tests, RECORD.md, the crate README, PROOF-TRANSLATE.md, and the cluster design's non-goal on other harnesses narrowed to Claude Code and Codex 0.156.0 with its markdown re-rendered; HOME-001's own boundary is unchanged, since it bounded HOME-001. Out of scope: Codex to Claude Code, which exists in no tree here; adopting, vendoring, linking or calling Codex's importer, or copying any Codex code; Pi or any third harness; launching Codex through a launch template, which is that template's next version; writing a Codex thread-index row by hand or outside the given Codex home, or any file under the user's own Codex home; fetching an image from any source that is not base64.

## Requirements

### R1: Record a Codex 0.156.0 rollout as the fixture and measure its shape and how Codex finds it

Before any render code is written, record the fixture rollout from Codex 0.156.0 itself. In a fresh isolated Codex home and an empty temporary working directory, run the binary that answers `--version` with `codex-cli 0.156.0`, by its absolute path, on a synthetic prompt that holds no transcript and no personal content, in a thread in which, after the synthetic user prompt, Codex writes an assistant text, then one call it records as a function_call item carrying no namespace field, then that call's function_call_output item, then a final assistant text. IF the recorded thread's response_item payloads, less the three kinds the fixture match leaves out, are not exactly five items in this order: a user message, an assistant message, a function_call with no namespace key, a function_call_output whose call_id equals that function_call's, and an assistant message, THEN THE SYSTEM SHALL discard the recording and record again with another synthetic prompt, and SHALL NOT edit a recorded file. Keep the rollout file that holds exactly those five whole as the fixture. Record a second thread in the same isolated home, kept out of the fixture, in which a synthetic image is passed with the prompt and Codex views a synthetic image through its own tool. Then write in the codex module's docs, measured from those two files: the directory and file-name pattern 0.156.0 wrote them under; the line kinds in the fixture and their order; the session_meta fields; the fields of each response_item kind in the fixture; the passthrough metadata fields per kind (turn_id, create_time, content_item_kinds) and the content_item_kinds value 0.156.0 writes for a user text message, an assistant message and a user message holding an image; the phase value on each assistant message and whether it is commentary exactly when a function_call follows before the next user message; the id prefix of each kind (the text before the first underscore); and the fields and the detail value of an input_image content item in a user message and inside a function_call_output. Measure whether any compaction item 0.156.0 writes carries readable text, and record the answer. Measure, in a second fresh isolated Codex home, that a copy of the fixture holding only its session_meta line and its response_item lines, placed under that home's sessions tree with no thread index written, is found by `codex exec resume <thread id>`, and record the thread-index row count in that home after the resume. IF the copy is not found without a thread-index row, THEN measure the row 0.156.0 writes for the fixture's thread in the first isolated home, record its table, columns and each value's source in the module docs, and name every value the render cannot derive from the source session or the rendered rollout, and whether writing the row needs a dependency that is not pure Rust. Record, from the fixture, the key set of each response_item kind the render produces (a user message, an assistant message, a function_call and a function_call_output), the key set of the internal_chat_message_metadata_passthrough object on each of those kinds, where phase sits on each assistant message (as an item key or as a passthrough key) and its value on the assistant message a function_call follows (PHASE_BEFORE_CALL) and on the last assistant message (PHASE_FINAL), and the value of namespace on the function_call_output when it carries one (OUTPUT_NAMESPACE). The render gives a value to exactly these item keys: type, id, role, content, phase, name, namespace, arguments, call_id, output and internal_chat_message_metadata_passthrough; and to exactly these passthrough keys: turn_id, create_time, content_item_kinds and phase. IF a recorded key set holds a key outside those, THEN THE SYSTEM SHALL stop before any render code is written and report that key and its kind, and SHALL NOT invent a value for it. THE SYSTEM SHALL hold as constants in the module: CODEX_VERSION (0.156.0), the model provider the fixture's session_meta carries, ORIGINATOR (lys-home), SOURCE (cli), THREAD_SOURCE (user) and HISTORY_MODE (paginated), the id prefix of each kind the render writes, the two image detail values, the three content_item_kinds values, the recorded key set of each of the four kinds and of the passthrough object on each, PHASE_BEFORE_CALL, PHASE_FINAL and, where the fixture's function_call_output carries namespace, OUTPUT_NAMESPACE. THE SYSTEM SHALL NOT take any shape from a Codex source checkout or from a rollout another Codex version wrote, SHALL NOT write under the user's own Codex home, SHALL NOT write any thread-index row by hand, and SHALL NOT put any transcript content in either recorded file. WHEN the fixture holds a response_item that no Claude Code record could be the source of, THE SYSTEM SHALL name its kind in the module docs as one of the three kinds the fixture match leaves out (a message of role developer, a user message whose content_item_kinds is ["environments.environment_context"], a reasoning item carrying encrypted_content), with the count of items of that kind in the fixture, and SHALL NOT write an equivalent of it in any render. WHEN the copy is found only through a thread-index row and every value of that row is derivable, THE SYSTEM SHALL make writing the row part of the render (R6). WHEN a value of the row is not derivable, THE SYSTEM SHALL write no row and the proof (R10) SHALL adopt the rollout through Codex 0.156.0's own command that creates a thread and adopts a rollout, which R1 names. IF 0.156.0 writes a compaction item carrying readable text, THEN THE SYSTEM SHALL stop before any render code is written and report the measurement, since this brief is written on the measurement that it does not.

**Acceptance:**
- tests/fixtures/codex/rollout-0.156.0.jsonl exists, every line parses as JSON, its session_meta payload's cli_version is "0.156.0", and its response_item payloads, less every message of role developer, every user message whose content_item_kinds is ["environments.environment_context"] and every reasoning item carrying encrypted_content, are exactly 5 items, in this order: a user message, an assistant message, a function_call with no namespace key, a function_call_output whose call_id equals that function_call's, and an assistant message.
- harness/codex/mod.rs holds CODEX_VERSION equal to "0.156.0", a model provider constant equal to the fixture's session_meta model_provider, ORIGINATOR equal to "lys-home", SOURCE equal to "cli", THREAD_SOURCE equal to "user" and HISTORY_MODE equal to "paginated".
- Each id prefix constant in harness/codex/mod.rs equals the text before the first underscore of the id of an item of that kind in the fixture.
- harness/codex/mod.rs holds, for each of a user message, an assistant message, a function_call and a function_call_output, a key-set constant equal to the key set of the fixture's item of that kind, and a passthrough key-set constant equal to the key set of that item's internal_chat_message_metadata_passthrough; every key in them is one of type, id, role, content, phase, name, namespace, arguments, call_id, output and internal_chat_message_metadata_passthrough for an item, and one of turn_id, create_time, content_item_kinds and phase for a passthrough object.
- PHASE_BEFORE_CALL equals the phase value on the fixture's first assistant message, PHASE_FINAL equals the phase value on its last assistant message, and where the fixture's function_call_output carries namespace, OUTPUT_NAMESPACE equals that value.
- harness/codex/mod.rs's module docs name the rollout directory and file-name pattern, the fixture's line kinds in order, the session_meta fields, the fields of each response_item kind in the fixture, the passthrough fields per kind, the three content_item_kinds values, the phase rule, the input_image fields and detail values in a user message and inside a function_call_output, whether 0.156.0 writes a compaction item with readable text (no), and the outcome of resuming the trimmed copy in a fresh isolated Codex home: found with no row written, with the thread-index row count after the resume; or found only through a row, with the row's table, columns and each value's source, and the values the render cannot derive, if any.
- harness/codex/mod.rs contains no `fn` item.
- harness/codex/mod.rs's module docs name each response_item kind in the fixture that the render does not produce, each being one of a message of role developer, a user message whose content_item_kinds is ["environments.environment_context"] and a reasoning item carrying encrypted_content, with its count in the fixture.

**Files:**
- create: crates/lys-home/tests/fixtures/codex/rollout-0.156.0.jsonl
- create: crates/lys-home/src/harness/codex/mod.rs
- modify: crates/lys-home/src/harness/mod.rs

**Checklist:**
- C24 — lys-home render-codex takes a home, a session, a target directory that is a Codex sessions root, and a Codex version; renders only for the measured Codex 0.156.0 and refuses any other version naming the version asked for and the one measured; walks the context path the Claude Code render walks; and writes one rollout under that root's dated directories in the shape Codex 0.156.0 writes for its own threads, measured from files that version wrote, with nothing written when it refuses.
- C29 — A fixture session imported from a fixture Claude Code file renders to a rollout whose items match a fixture rollout recorded from Codex 0.156.0, apart from ids and timestamps, over the item kinds the render can produce, with the kinds Codex writes that no source record stands behind named and counted; a tool result longer than Codex's importer clips is carried whole.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose session was imported from Claude Code, I want it rendered as a Codex rollout that Codex resumes, with every text part, tool call and tool result carried whole and the thread announcing itself as a translation, so that I can continue on another harness and never mistake the fork for the original.

### R2: Name the Codex render's refusals

Add to HomeError: UnmeasuredCodexVersion, carrying the version asked for and the version the shape was measured on; BlockNotStored, carrying the entry id and the part index of a part none of whose source candidates the block store holds; UnparsedTimestamp, carrying the entry id whose timestamp does not parse as RFC 3339; UnknownTimeZone, carrying a zone name the time-zone database does not hold; and UnnamedTimeZone, for an environment whose zone resolves to no IANA name. Each message SHALL name those values and SHALL NOT carry any byte of a part's content. An existing target path is refused with the existing HomeError::Exists. THE SYSTEM SHALL NOT change the message or fields of any existing HomeError variant.

**Acceptance:**
- HomeError::UnmeasuredCodexVersion { asked: "0.156.1", measured: "0.156.0" } displays a message containing both "0.156.1" and "0.156.0".
- HomeError::BlockNotStored for entry id e1 and part index 2 displays a message containing "e1" and "2".
- HomeError::UnparsedTimestamp for entry id e3 displays a message containing "e3".
- HomeError::UnknownTimeZone for zone Nowhere/Zone displays a message containing "Nowhere/Zone".
- HomeError::UnnamedTimeZone displays a message containing "IANA".

**Files:**
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C24 — lys-home render-codex takes a home, a session, a target directory that is a Codex sessions root, and a Codex version; renders only for the measured Codex 0.156.0 and refuses any other version naming the version asked for and the one measured; walks the context path the Claude Code render walks; and writes one rollout under that root's dated directories in the shape Codex 0.156.0 writes for its own threads, measured from files that version wrote, with nothing written when it refuses.
- C27 — A JSON loss account beside the rollout lists, by entry id and the importer's block hash, what was kept, what changed shape (before and after kinds, and how), and what was lost and why: sidechains, harness events, lys entries, and any part kind Codex has no item for.

**Stories:**
- S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a loss account beside every translation naming by entry id and block hash what was kept, what changed shape and what was lost and why, and the resume measured on a named Codex version, so that nobody claims a faithful translation that was not measured.

### R3: Write the two fixture Claude Code files from the measured part shapes

Write two synthetic Claude Code files, every part in one of the key sets the task lists as measured, each tool_use carrying caller {"type":"direct"} unless stated. The first, tests/fixtures/codex/claude-code.jsonl, carries the recorded fixture rollout's conversation (R1) and nothing else: one user record whose content string is the fixture's user text, then one assistant record per assistant message and function_call of the fixture in the fixture's order (the first assistant text, then a tool_use whose name is the function_call's name and whose input is its arguments parsed as JSON, then the final assistant text), with one user record between them holding one tool_result for that tool_use, is_error false, whose content string is the function_call_output's output. The second, tests/fixtures/codex/claude-code-losses.jsonl, holds these records in file order: a summary record; a user record with string content; a user record with isMeta true and string content; an assistant record holding a thinking part with readable text and a signature, a redacted_thinking part, a text part and a tool_use whose input serialises by serde_json::to_string to exactly 8,192 characters; a user record holding its tool_result, is_error false, whose content string has exactly 16,384 characters; an assistant record holding a second tool_use; a user record holding its tool_result, is_error false, with content an array of one text item and one base64 image item; an assistant record holding a third tool_use with no caller field; a user record holding its tool_result, is_error true, with content an array of one text item and one tool_reference item; an attachment record with a uuid on the main chain; a user record whose content is an array of a base64 image part at index 0, an image part whose source type is url at index 1, and a text part at index 2; an assistant record whose model is authored holding one text part; a permission-mode record; two sidechain records (isSidechain true, one agentId), a user record with string content and an assistant record with one text part; and a final main-chain assistant record holding one text part, whose parentUuid is the authored record's uuid. Every text in both files is a synthetic token, and every url is a synthetic address that is never fetched. THE SYSTEM SHALL NOT put any real transcript text, credential, or path belonging to a person's machine in either file.

**Acceptance:**
- Every line of claude-code.jsonl and claude-code-losses.jsonl parses as JSON, and every part in their message contents has one of the key sets the task lists as measured.
- claude-code.jsonl holds exactly one tool_use part; its name equals the fixture rollout's function_call name, its input equals that function_call's arguments parsed as JSON, and it carries caller {"type":"direct"}; its tool_result's content string equals the fixture's function_call_output output.
- In claude-code-losses.jsonl, the first tool_use's input serialises by serde_json::to_string to 8,192 characters, the first tool_result's content string has 16,384 characters, the third tool_use has no caller key, and exactly 1 part is an image part whose source type is url.
- `lys-home import` of claude-code-losses.jsonl into a fresh home exits 0 and its report counts 1 sidechain.
- `lys-home import` of claude-code.jsonl into a fresh home exits 0.

**Files:**
- create: crates/lys-home/tests/fixtures/codex/claude-code.jsonl
- create: crates/lys-home/tests/fixtures/codex/claude-code-losses.jsonl

**Checklist:**
- C29 — A fixture session imported from a fixture Claude Code file renders to a rollout whose items match a fixture rollout recorded from Codex 0.156.0, apart from ids and timestamps, over the item kinds the render can produce, with the kinds Codex writes that no source record stands behind named and counted; a tool result longer than Codex's importer clips is carried whole.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose session was imported from Claude Code, I want it rendered as a Codex rollout that Codex resumes, with every text part, tool call and tool result carried whole and the thread announcing itself as a translation, so that I can continue on another harness and never mistake the fork for the original.

### R4: Key every account line to the block the importer stored, and account every entry off the context path

In harness/codex/loss.rs, define the loss account and the block hash. The account is one JSON object with exactly the keys source_session, source_head, codex_version, rollout, kept, changed and lost: source_session is the source session id; source_head is Session::head_hash() at render; codex_version is the version rendered for; rollout is the SHA-256, in lowercase hex, of the rollout file's bytes exactly as written beside the account, so the account names its rollout by content and its bytes do not depend on the zone the path was computed in; kept is a list of {entry, part, block, item}; changed a list of {entry, part, block, before, after, how}; lost a list of {entry, part, block, kind, reason}. entry is the entry id; part is the index of the part in the entry's content, 0 for a toolResult message (whose one stored block is its tool_result part), and null for an entry-level line; block is a block hash, or null for an entry that names no block. WHEN a part is accounted, THE SYSTEM SHALL key it by the SHA-256 of serde_json::to_vec of its Claude-shaped source part, taking the first candidate of the task's inverse mapping, in the task's order, that the home's block store holds. IF no candidate is held, THEN THE SYSTEM SHALL refuse with BlockNotStored naming the entry id and part index. WHEN an entry is not on the context path, THE SYSTEM SHALL account it lost: a message entry with one line per part, by that part's block hash; a lys.harness_event entry with one line whose block is the record hash its data names, null when it names none; and any other entry with one line whose block is null. The reason is sidechain for a label whose text begins `agent ` and for every entry in the subtree under the entry such a label targets; before_compaction for an entry on the path to the head but before the context path's kept point; harness_event for a lys.harness_event entry; lys_entry for any other custom entry whose type begins `lys.`; and off_path for any other entry. THE SYSTEM SHALL NOT name the SHA-256 of a re-serialised Pi part in the account, SHALL NOT list an entry off the path, or a part of one, twice, and SHALL NOT carry any byte of a part's content in the account.

**Acceptance:**
- For claude-code-losses.jsonl imported into a fresh home, the block hash loss.rs gives the first tool_use part, stored with caller {"type":"direct"}, equals the SHA-256 of serde_json::to_vec of that part as parsed from the file, and BlockStore::get finds it.
- For the same import, the block hash of the third tool_use, stored with no caller, equals the SHA-256 of serde_json::to_vec of that part as parsed from the file.
- For the same import, the block hashes of the first tool_result (string content, is_error false), the second (array content, is_error false) and the third (array content, is_error true) each equal the SHA-256 of serde_json::to_vec of that part as parsed from the file.
- For a session built in the test whose block store is missing every candidate block for part 1 of entry e1, loss.rs refuses with BlockNotStored whose entry id is e1 and whose part index is 1.
- For the same import, the off-path lines hold exactly 3 lines with reason sidechain (the label, with block null, and one line for each sidechain message's one part), and each of the 2 sidechain part hashes is found by BlockStore::get.
- For the same import, the permission-mode entry and each tool_completed entry off the path appear in exactly 1 line each with reason harness_event, and the permission-mode line's block equals the record hash in that entry's data.
- For the same import, the compaction entry the summary record became is on the context path, and no lost line names it.
- For a session built in the test with a user message m1, an assistant message m2, a compaction whose first_kept_entry_id is m2, and a user message after it, m1's one part appears in exactly 1 line, with reason before_compaction and its block hash.
- The account's serialised object has exactly the keys changed, codex_version, kept, lost, rollout, source_head and source_session, and a kept line, a changed line and a lost line each serialise with exactly the keys listed for their list.
- An account built in the test for rollout bytes B of 3 lines, source_session s1, source_head h1 and codex_version 0.156.0 has a rollout value that is a 64-character lowercase hex string equal to the SHA-256 of B, source_session s1, source_head h1 and codex_version 0.156.0; built again for B with one byte appended, its rollout value differs and every other key's value is unchanged.

**Files:**
- create: crates/lys-home/src/harness/codex/loss.rs
- create: crates/lys-home/src/harness/codex/loss_tests.rs

**Checklist:**
- C27 — A JSON loss account beside the rollout lists, by entry id and the importer's block hash, what was kept, what changed shape (before and after kinds, and how), and what was lost and why: sidechains, harness events, lys entries, and any part kind Codex has no item for.

**Stories:**
- S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a loss account beside every translation naming by entry id and block hash what was kept, what changed shape and what was lost and why, and the resume measured on a named Codex version, so that nobody claims a faithful translation that was not measured.

### R5: Map the context path to Codex 0.156.0's lines, whole, with the marker last

In harness/codex/rollout.rs, WHEN the context path is mapped, THE SYSTEM SHALL write lines of the form {ordinal, payload, timestamp, type}, ordinal counting from 0, each timestamp in the form YYYY-MM-DDThh:mm:ss.sssZ in UTC. Line 0 is session_meta, its timestamp the session header's, its payload exactly {id, session_id, timestamp, cwd, runtime_workspace_roots, originator, cli_version, source, thread_source, model_provider, history_mode}: id and session_id the thread id, the uuid render::record_uuid gives for `<source session id>:<source head hash>`; timestamp the header's; cwd the header's cwd, written as it is; runtime_workspace_roots [cwd]; and the other six the R1 constants. base_instructions and context_window are not written, so Codex supplies its own instructions on resume. Every later line is a response_item whose timestamp is its source entry's, and in path order: a user message's text parts and base64 image parts as one message item of role user, content an input_text item per text part and an input_image item per base64 image part, {type: input_image, image_url: `data:<media_type>;base64,<data>`, detail: the R1 message detail constant}; each assistant text part and each readable thinking part as its own message item of role assistant with one output_text item; a toolCall part as a function_call item; a toolResult message as a function_call_output item whose output is the one text part's text when the result holds exactly one text part and nothing else, and otherwise a list holding an input_text item per text part and an input_image item (with the R1 output detail constant) per base64 image part, in order; and a compaction as a message item of role user whose single input_text is `<COMPACTION SUMMARY>`, a newline, and the summary whole. Each item carries exactly the key set R1 recorded for its kind, and its internal_chat_message_metadata_passthrough exactly the passthrough key set R1 recorded for that kind, each key valued so: type the kind; id as below; role user or assistant; content the content items above; name, on a function_call the toolCall's name and on a function_call_output the name of the toolCall its call_id pairs with; namespace, on a function_call_output only, OUTPUT_NAMESPACE, and never on a function_call; arguments the toolCall's input as serde_json::to_string gives it; call_id the Claude tool_use id; output as above; and in the passthrough, turn_id, the uuid record_uuid gives for `turn:<id>` of the latest user-message or compaction entry at or before it on the path (the first path entry's id when there is none); create_time, its source entry's timestamp as Unix seconds with milliseconds; and, on a message item, content_item_kinds, the R1 constant for its role and content. phase is written on an assistant message item only, where R1 recorded it: PHASE_BEFORE_CALL when a function_call item follows it before the next user message item, and PHASE_FINAL otherwise. The render asserts nothing about what those two values are: where R1 measured that the message before a call does not carry commentary, or that the two values are equal, the render writes the values R1 recorded and the fixture match (R8) is the check. Each item's id is the R1 prefix for its kind, an underscore, and the uuid record_uuid gives for `<entry id>:<part index>`. After the last mapped item, THE SYSTEM SHALL write one marker item, an assistant message whose single text is the marker line in the task, which has no source entry and takes every source-derived value from the head entry: its line timestamp the head entry's timestamp; its id the assistant message prefix, an underscore, and the uuid record_uuid gives for `marker:<source head hash>`; and in its passthrough, create_time the head entry's timestamp as Unix seconds with milliseconds, turn_id the turn_id the rule above gives the head entry (the uuid record_uuid gives for `turn:<id>` of the latest user-message or compaction entry at or before the head on the path, the first path entry's id when there is none), content_item_kinds the R1 constant for an assistant message, and phase PHASE_FINAL, since no function_call follows it. The marker is accounted in no line, having no source part. WHEN a part is mapped, THE SYSTEM SHALL account it through R4's block hash: a text part, a base64 image part in a user message, a toolCall, and a toolResult whose is_error is false as kept, item message, input_image, function_call and function_call_output; readable thinking (non-empty text, not redacted) as changed, before thinking, after message, how `readable thinking carried as text; signature not carried`; a toolResult whose is_error is true as changed, before tool_result, after function_call_output, how `is_error true not carried`; and a compaction as changed with part and block null, before compaction, after message, how `summary carried as a user message; Codex 0.156.0's compaction item carries only encrypted_content`. Every thinking block is treated so, since no imported block's provider is the target's (home P3). IF a thinking block is redacted, or has a signature and no readable text, THEN THE SYSTEM SHALL drop it and account it lost with reason opaque_thinking; IF it is empty and unsigned, THEN lost with reason empty_thinking. IF an image part, in a message or inside a tool result, has a source other than base64, THEN THE SYSTEM SHALL NOT fetch it, SHALL carry no item for it, and SHALL account it lost with reason unfetched_image:<source type>:<index in the array holding it>. Every other part kind with no Codex item SHALL be accounted lost with reason no_codex_item, and an item of such a kind inside a tool result's content with reason no_codex_item:<kind>:<index> at part 0 of that toolResult entry, a line additional to the result's own kept or changed line, as is an unfetched_image line for an image inside a tool result's content. A lys.harness_event on the path is lost with reason harness_event, any other lys custom entry with reason lys_entry, and any other entry on the path that yields no item with reason no_codex_item. IF an entry's timestamp does not parse as RFC 3339, THEN THE SYSTEM SHALL refuse with UnparsedTimestamp. The complete account is R4's off-path lines and these. THE SYSTEM SHALL NOT truncate, clip, summarise or note any text, argument or result, SHALL NOT carry a thinking signature or an opaque block, SHALL NOT render a sidechain entry, a custom entry or a label, SHALL NOT write any line kind but session_meta and response_item, and SHALL NOT use a random value or the clock for any id or timestamp.

**Acceptance:**
- For claude-code.jsonl imported under session id fixture-translate into a session created with cwd /w, line 0 has type session_meta and ordinal 0, its payload has exactly the keys cli_version, cwd, history_mode, id, model_provider, originator, runtime_workspace_roots, session_id, source, thread_source and timestamp, cwd is "/w", runtime_workspace_roots is ["/w"], id and session_id equal record_uuid("fixture-translate:<head hash>"), originator is "lys-home", and every other line has type response_item.
- The same session maps to exactly 6 response_items: message (user), message (assistant, carrying PHASE_BEFORE_CALL where R1 recorded phase), function_call, function_call_output, message (assistant, carrying PHASE_FINAL where R1 recorded phase) and the marker, in that order; each carries exactly R1's key set for its kind and exactly R1's passthrough key set for that kind, the function_call carries no namespace key, and the function_call_output's name equals the function_call's name.
- The last item's single text equals `<TRANSLATED CONTEXT from="claude-code" to="codex 0.156.0" source_session="fixture-translate" source_head="<head hash>">` with the session's head hash.
- For the same session, the marker item's line timestamp equals the head entry's timestamp, its id is the assistant message prefix, an underscore and record_uuid("marker:<head hash>"), and its passthrough's create_time equals the head entry's timestamp as Unix seconds with milliseconds, its turn_id equals the turn_id on the user message item (record_uuid("turn:<that user message's entry id>")), its content_item_kinds equals the R1 assistant message constant, and, where R1 recorded phase, its phase is PHASE_FINAL; no account line names the marker.
- For claude-code-losses.jsonl imported, the function_call_output for the first tool call has an output of 16,384 characters with the same SHA-256 as the source text, and the first function_call's arguments string equals serde_json::to_string of the stored input, 8,192 characters long.
- For the same import, the thinking part with a signature maps to an assistant message item whose text is the part's thinking text, the item carries no signature field, and the account holds one changed line for its block with before thinking and after message; the redacted part maps to no item and the account holds one lost line naming its block hash with reason opaque_thinking.
- For the same import, the user message holding a base64 image at index 0, a url image at index 1 and a text part maps to one message item whose content holds exactly 1 input_image carrying the first image's data whole and 1 input_text, the account holds one kept line with item input_image and one lost line with reason unfetched_image:url:1 whose block is the SHA-256 of that part's stored bytes, and the account's bytes hold no occurrence of the url.
- For the same import, the second tool result maps to a function_call_output whose output is a list of exactly 1 input_text carrying its text whole and 1 input_image carrying the image's data whole.
- For the same import, the third tool result (is_error true) is accounted by one changed line with before tool_result and after function_call_output, and one lost line with reason no_codex_item:tool_reference:1.
- For the same import, the isMeta user record's text maps to a user message item carrying it whole, accounted kept; the attachment entry on the path is lost with reason harness_event and the lys.authored entry with reason lys_entry.
- For the same import, every entry id in the session file appears in the account; every (entry, part) pair of a user or assistant message on the context path appears in exactly 1 line whose part equals that index, counting every reason, unfetched_image:url:1 at part 1 of the image-holding user message included; every toolResult entry on the context path appears in exactly 1 kept or changed line at part 0; the lines for items inside a tool result's content, of the forms no_codex_item:<kind>:<index> and unfetched_image:<type>:<index> at part 0 of a toolResult entry, are additional lines, exactly 1 for this import (no_codex_item:tool_reference:1); and every entry holding no part appears in exactly 1 line with part null.
- For a session built in the test with a user message, an assistant message m2, a compaction whose summary is s1 and whose first_kept_entry_id is m2, and a user message after it, the first response_item is a user message whose single input_text is `<COMPACTION SUMMARY>`, a newline and s1, and the account holds one changed line for the compaction with before compaction and after message.
- For claude-code-losses.jsonl imported into a fresh home, the first response_item is a user message whose single input_text is `<COMPACTION SUMMARY>`, a newline and the summary record's summary whole, and the compaction entry the summary record became appears in exactly 1 line of the account, a changed line with before compaction, after message, part null and block null.
- For a session built in the test with a thinking part whose text is empty and which has no signature, the part maps to no item and the account holds one lost line with reason empty_thinking.
- Mapping the same session twice gives byte-identical lines and account.

**Files:**
- create: crates/lys-home/src/harness/codex/rollout.rs
- create: crates/lys-home/src/harness/codex/rollout_tests.rs

**Checklist:**
- C25 — Every text part, tool call and tool result on the context path is carried whole as Codex's own message, function_call and function_call_output items, paired by call id, never clipped or noted; readable thinking is carried as text; an image part whose source is base64 is carried as Codex's input_image item, and any other image source is named lost, never fetched; opaque thinking is dropped and named by hash; a compaction becomes Codex's own compaction item where 0.156.0 writes one with readable text, measured, and otherwise a user message that says it is a compaction summary.
- C26 — The rendered thread announces itself in band as a translated context with the source session id and the source head hash.
- C27 — A JSON loss account beside the rollout lists, by entry id and the importer's block hash, what was kept, what changed shape (before and after kinds, and how), and what was lost and why: sidechains, harness events, lys entries, and any part kind Codex has no item for.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose session was imported from Claude Code, I want it rendered as a Codex rollout that Codex resumes, with every text part, tool call and tool result carried whole and the thread announcing itself as a translation, so that I can continue on another harness and never mistake the fork for the original.
- S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a loss account beside every translation naming by entry id and block hash what was kept, what changed shape and what was lost and why, and the resume measured on a named Codex version, so that nobody claims a faithful translation that was not measured.

### R6: Translate: check the version, refuse an existing path, write both files under the sessions root, record the translation beside the path

Add the pure-Rust jiff crate as a workspace dependency of lys-home, for the time-zone database and local-time conversion. In harness/codex/translate.rs, WHEN a translation is asked for a session, a sessions root, a Codex version and a time-zone name, THE SYSTEM SHALL: refuse with UnmeasuredCodexVersion when the version is not 0.156.0; refuse with UnknownTimeZone when the time-zone database holds no zone of that name; compute the rollout path and the account path as the task gives them from the session header's timestamp, converted to that zone's local time, and the thread id; refuse with HomeError::Exists naming the path when either exists; map and account (R5), which may refuse; create the dated directories; write the rollout and then the account, each with create_new and synced; only where R1 measured that 0.156.0 finds a rollout only through a thread-index row and every value of it is derivable, write that row into the Codex home the sessions root sits in, in R1's measured shape, deterministically, and record it in the account as one changed line {entry: the head entry id, part: null, block: null, before: none, after: thread_index_row, how: `written because Codex 0.156.0 finds a rollout only through a thread-index row`}, which is mapped and accounted before either file is written; store a manifest block {codex_version, source_session, source_head, thread, time_zone, files: [{path, sha256}] with the rollout then the account}, time_zone being the zone name the path was computed in; and append one lys.harness_event side leaf with Session::append_beside, so the head does not move, whose data is {kind: codex_translation, harness: codex, source_uuid: null, record: the manifest block's hash, detail: {codex_version, source_head, time_zone, rollout, account}}, rollout and account being the two files' SHA-256, refused when it exceeds MAX_DATA_BYTES. IF any refusal fires, THEN THE SYSTEM SHALL write no file, create no directory, and append no entry. THE SYSTEM SHALL NOT rewrite the session file, a block or the original Claude Code file, SHALL NOT read the time zone from the environment itself (the caller passes it), SHALL NOT write under the user's Codex home, SHALL NOT write a thread-index row by hand or outside the given Codex home, and SHALL NOT write one at all where R1 measured that none is needed, and SHALL NOT run Codex (ADR-007).

**Acceptance:**
- For a session whose header timestamp is 2026-09-20T01:02:03.456Z and whose thread id is T, a translation to --out O in zone UTC writes O/2026/09/20/rollout-2026-09-20T01-02-03-T.jsonl and O/2026/09/20/rollout-2026-09-20T01-02-03-T.loss.json, and no other file.
- For a session whose header timestamp is 2026-09-04T21:50:00.000Z and whose thread id is T, a translation in zone UTC writes its rollout at O1/2026/09/04/rollout-2026-09-04T21-50-00-T.jsonl, and a translation of the same session in zone Australia/Melbourne writes it at O2/2026/09/05/rollout-2026-09-05T07-50-00-T.jsonl; the two rollouts' session_meta timestamp is 2026-09-04T21:50:00.000Z in both, and their bytes are equal.
- A translation asked in zone Nowhere/Zone is refused with UnknownTimeZone naming Nowhere/Zone, the sessions root holds no file and no directory afterwards, and the session's entry count is unchanged.
- A translation asked for version 0.156.1 is refused with UnmeasuredCodexVersion naming 0.156.1 and 0.156.0, the sessions root holds no file and no directory afterwards, and the session's entry count is unchanged.
- With a file already at the rollout path, the translation is refused with Exists naming that path, the existing file's SHA-256 is unchanged, no account is written and the session's entry count is unchanged; with a file already at the account path, it is refused with Exists naming the account path, no rollout is written and the entry count is unchanged.
- For a session whose block store is missing every candidate block for one part on the context path, a translation to --out O is refused with BlockNotStored naming that entry id and part index, O holds no file and no directory afterwards, and the session's entry count is unchanged.
- After a translation, the session holds exactly one more entry, a lys.harness_event of kind codex_translation whose parentId is the head, the head is unchanged, and the manifest block its record names holds the rollout and account paths with SHA-256 values equal to the files' on disk the codex_version 0.156.0 and the time_zone the translation was asked in, and the side leaf's detail time_zone equals it.
- The session file's bytes before the translation are a prefix of its bytes after it.
- Where R1 recorded that the trimmed copy was found with no row written, a translation into a Codex home holding a thread index leaves that index's bytes unchanged; where R1 recorded that a derivable row is needed, the translation writes exactly 1 row, equal across two translations of the same session into two fresh Codex homes, and the account holds exactly 1 changed line with after thread_index_row.
- For claude-code.jsonl imported under session id fixture-translate into two fresh homes, the first translated with the zone input UTC and the second with the zone input Australia/Melbourne: each account's rollout value is a 64-character lowercase hex string equal to the SHA-256 of the rollout file that account names, its source_session is "fixture-translate", its source_head equals that home's session head hash before the translation, and its codex_version is "0.156.0"; the two accounts' kept, changed and lost lists have equal lengths; the compared lines are every kept, changed and lost line whose entry id is a uuid of a record in claude-code.jsonl, or such a uuid followed by -r and an index, with the lines naming entries the importer gave a fresh id left out; before comparing, the test asserts that the first account holds exactly 5 compared kept lines and 0 compared lost lines, and exactly 0 compared changed lines where R1 recorded that the trimmed copy was found with no row written and exactly 1 (the thread_index_row line) where R1 recorded that a derivable row is needed; the compared lines of the two accounts are then equal, line for line in account order; and the two rollout paths, taken relative to their sessions roots, differ only in the year, month and day directories and the date and time in the file name.

**Files:**
- create: crates/lys-home/src/harness/codex/translate.rs
- create: crates/lys-home/src/harness/codex/translate_tests.rs
- modify: Cargo.toml
- modify: Cargo.lock
- modify: crates/lys-home/Cargo.toml

**Checklist:**
- C24 — lys-home render-codex takes a home, a session, a target directory that is a Codex sessions root, and a Codex version; renders only for the measured Codex 0.156.0 and refuses any other version naming the version asked for and the one measured; walks the context path the Claude Code render walks; and writes one rollout under that root's dated directories in the shape Codex 0.156.0 writes for its own threads, measured from files that version wrote, with nothing written when it refuses.
- C28 — The report is JSON with the rollout and account paths, entry and block counts and the loss counts, never content; an existing path is refused by name with nothing written; each translation is recorded on the session as a codex_translation side leaf naming both files by path and SHA-256 and the Codex version; the Claude Code render of the same session is unchanged, checked by hash.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose session was imported from Claude Code, I want it rendered as a Codex rollout that Codex resumes, with every text part, tool call and tool result carried whole and the thread announcing itself as a translation, so that I can continue on another harness and never mistake the fork for the original.
- S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a loss account beside every translation naming by entry id and block hash what was kept, what changed shape and what was lost and why, and the resume measured on a named Codex version, so that nobody claims a faithful translation that was not measured.

### R7: Add the render-codex subcommand with a report of paths and counts

Add `render-codex` with the arguments --home, --session, --out and --codex-version, implemented in src/cli/codex.rs and dispatched from cli.rs, as cli/given.rs is. WHEN it runs, THE SYSTEM SHALL resolve the time zone of the environment it runs in (the TZ variable when set, otherwise the system's configured zone) to its IANA name and pass that name to the translation, and IF the zone resolves to no IANA name, THEN it SHALL exit 1 with UnnamedTimeZone and write nothing; it SHALL NOT take the zone as an argument. WHEN it succeeds, THE SYSTEM SHALL print one JSON report with exactly the keys rollout, account, entries, entries_rendered, blocks, kept, changed and lost: the two paths, the count of entries in the session file, the count of entries on the context path that produced at least one item, the count of blocks named by the parts of message entries on the context path, and the three loss counts. IF an argument is missing, THEN THE SYSTEM SHALL exit 2 naming it. IF the translation refuses, THEN THE SYSTEM SHALL exit 1 with the refusal's message. THE SYSTEM SHALL NOT print, log or put in an error any byte of a part's content.

**Acceptance:**
- render-codex on claude-code.jsonl imported prints a report whose keys are exactly account, blocks, changed, entries, entries_rendered, kept, lost and rollout, and whose kept, changed and lost equal the lengths of the account's three lists.
- render-codex with no --codex-version exits 2 and names --codex-version.
- render-codex with --codex-version 0.156.1 exits 1 and its message names 0.156.1 and 0.156.0.
- cli.rs has at most 500 code lines, excluding comments and blank lines, after the change.

**Files:**
- create: crates/lys-home/src/cli/codex.rs
- modify: crates/lys-home/src/cli.rs

**Checklist:**
- C28 — The report is JSON with the rollout and account paths, entry and block counts and the loss counts, never content; an existing path is refused by name with nothing written; each translation is recorded on the session as a codex_translation side leaf naming both files by path and SHA-256 and the Codex version; the Claude Code render of the same session is unchanged, checked by hash.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose session was imported from Claude Code, I want it rendered as a Codex rollout that Codex resumes, with every text part, tool call and tool result carried whole and the thread announcing itself as a translation, so that I can continue on another harness and never mistake the fork for the original.

### R8: Prove the translation end to end on the fixture Claude Code files

Add an integration test that imports each fixture Claude Code file into a fresh home (HOME-001's importer), runs render-codex, and checks the card's acceptance. The fixture match: take the rendered rollout's response_item payloads less the final marker item, and the fixture rollout's response_item payloads less each message of role developer, each user message whose content_item_kinds is ["environments.environment_context"] and each reasoning item carrying encrypted_content; compare their number, then each pair in order as JSON values after removing the keys id and call_id and, inside the passthrough metadata, turn_id and create_time, comparing a function_call's arguments as parsed JSON. It also runs the render-codex binary as a child process on a session the test creates with header timestamp 2026-09-04T21:50:00.000Z, once with TZ=UTC and once with TZ=Australia/Melbourne, each into its own fresh --out, so the zone is pinned by the child's environment and never by changing the test process's own. The test SHALL count what fired: every assertion over a list first asserts its length. THE SYSTEM SHALL NOT use fixture text as, or inside, a test name.

**Acceptance:**
- For claude-code.jsonl imported, the fixture match holds: equal in number and, pair by pair, equal as defined.
- A copy of the fixture rollout with one added response_item of type custom_tool_call, a kind the render does not produce and not one of the three left out, fails the fixture match.
- In the rendered rollout, every function_call_output's call_id equals the call_id of exactly one function_call before it.
- For claude-code-losses.jsonl imported, the rollout carries the 16,384-character tool result whole, checked by length and SHA-256.
- Rendering a second time to the same --out exits 1 naming the rollout path, and the SHA-256 of both files under --out is unchanged.
- The Claude Code render (render_claude_code) of the session imported from claude-code.jsonl, to one path before render-codex and to a second path after it, gives files with equal SHA-256.
- A search of the report and of the account for the fixture's user text, tool output and assistant texts finds 0 occurrences.
- render-codex run with TZ=UTC on a session whose header timestamp is 2026-09-04T21:50:00.000Z writes its rollout under <out>/2026/09/04/ with a file name beginning rollout-2026-09-04T21-50-00-, and run with TZ=Australia/Melbourne on the same session writes it under <out>/2026/09/05/ with a file name beginning rollout-2026-09-05T07-50-00-; the codex_translation side leaves those two runs append carry time_zone UTC and Australia/Melbourne respectively.

**Files:**
- create: crates/lys-home/tests/codex_translation.rs

**Checklist:**
- C25 — Every text part, tool call and tool result on the context path is carried whole as Codex's own message, function_call and function_call_output items, paired by call id, never clipped or noted; readable thinking is carried as text; an image part whose source is base64 is carried as Codex's input_image item, and any other image source is named lost, never fetched; opaque thinking is dropped and named by hash; a compaction becomes Codex's own compaction item where 0.156.0 writes one with readable text, measured, and otherwise a user message that says it is a compaction summary.
- C27 — A JSON loss account beside the rollout lists, by entry id and the importer's block hash, what was kept, what changed shape (before and after kinds, and how), and what was lost and why: sidechains, harness events, lys entries, and any part kind Codex has no item for.
- C28 — The report is JSON with the rollout and account paths, entry and block counts and the loss counts, never content; an existing path is refused by name with nothing written; each translation is recorded on the session as a codex_translation side leaf naming both files by path and SHA-256 and the Codex version; the Claude Code render of the same session is unchanged, checked by hash.
- C29 — A fixture session imported from a fixture Claude Code file renders to a rollout whose items match a fixture rollout recorded from Codex 0.156.0, apart from ids and timestamps, over the item kinds the render can produce, with the kinds Codex writes that no source record stands behind named and counted; a tool result longer than Codex's importer clips is carried whole.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose session was imported from Claude Code, I want it rendered as a Codex rollout that Codex resumes, with every text part, tool call and tool result carried whole and the thread announcing itself as a translation, so that I can continue on another harness and never mistake the fork for the original.
- S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a loss account beside every translation naming by entry id and block hash what was kept, what changed shape and what was lost and why, and the resume measured on a named Codex version, so that nobody claims a faithful translation that was not measured.

### R9: Write the translation into RECORD.md and the crate README

Add to RECORD.md a section on the Codex translation: the loss account's keys and its kept, changed and lost lists with their fields; the eight reasons sidechain, harness_event, lys_entry, before_compaction, off_path, no_codex_item, opaque_thinking and empty_thinking, and the forms unfetched_image:<source type>:<index> and no_codex_item:<kind>:<index>; how a block hash keys to the importer's stored block, with the candidate order; the marker line; the compaction line; how image parts are carried as input_image when their source is base64 and lost otherwise; the rollout and account paths under --out, dated in the local time of the zone resolved from the environment, with every timestamp inside the rollout in UTC; the originator lys-home; and the codex_translation kind of lys.harness_event with its record and detail, time_zone included. Add render-codex and codex_translation to the crate README. Neither document SHALL quote any transcript content.

**Acceptance:**
- RECORD.md names the account keys source_session, source_head, codex_version, rollout, kept, changed and lost, the line fields entry, part, block, item, before, after, how, kind and reason, the eight reasons and the two indexed forms, and the codex_translation kind with its detail keys codex_version, source_head, time_zone, rollout and account, and states that the rollout path is dated in local time in the zone the record names.
- The crate README names the render-codex subcommand and its four arguments, the codex_translation kind, and that the time zone is taken from the environment (TZ when set).

**Files:**
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/README.md

**Checklist:**
- C27 — A JSON loss account beside the rollout lists, by entry id and the importer's block hash, what was kept, what changed shape (before and after kinds, and how), and what was lost and why: sidechains, harness events, lys entries, and any part kind Codex has no item for.
- C28 — The report is JSON with the rollout and account paths, entry and block counts and the loss counts, never content; an existing path is refused by name with nothing written; each translation is recorded on the session as a codex_translation side leaf naming both files by path and SHA-256 and the Codex version; the Claude Code render of the same session is unchanged, checked by hash.

**Stories:**
- S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a loss account beside every translation naming by entry id and block hash what was kept, what changed shape and what was lost and why, and the resume measured on a named Codex version, so that nobody claims a faithful translation that was not measured.

### R10: Record the real resume by Codex 0.156.0 in PROOF-TRANSLATE.md

Make a fresh isolated Codex home under a temporary directory, import claude-code.jsonl into a fresh home, render it with render-codex with --out <codex home>/sessions, then resume the thread with the Codex 0.156.0 binary by its absolute path, CODEX_HOME set to that home, by `codex exec resume <thread id>` with one question only the fixture's content answers. Write PROOF-TRANSLATE.md with: the binary's `--version` answer and which binary it was; how the isolated home was made and authenticated, naming the method and never the credential; the render's report; the rollout's and the account's SHA-256 and line counts as rendered; the rollout's session_meta originator (lys-home) and that Codex 0.156.0 resumed the thread with it; the time zone the render named; which road found the thread (Codex's own scan with no row written, the row the render wrote, or Codex 0.156.0's own command that creates a thread and adopts the rollout, named with its command line), that no row was written by hand, and the thread-index row count in <codex home> after the resume; the rollout's line count after the resume; each item kind left out of the fixture match with the count of items of that kind in Codex's own fixture rollout; the resume command with <codex home> in place of the directory; whether the answer held the fixture token the question asks for, as yes or no; and the SHA-256 of the Claude Code render of the same session before and after, equal. IF the resume does not find the thread or does not answer from its content by the road R1 measured, and Codex 0.156.0's own adopting command does not resume it either, THEN THE SYSTEM SHALL stop and report the measurement with the SHA-256 of every file it wrote, and SHALL NOT register the thread by hand. IF Codex 0.156.0 refuses the rollout for its originator, THEN THE SYSTEM SHALL stop and report the refusal by name, and SHALL NOT render with any other originator. THE SYSTEM SHALL NOT put any transcript content, answer text or credential in the proof, and SHALL NOT write under the user's own Codex home.

**Acceptance:**
- PROOF-TRANSLATE.md names codex-cli 0.156.0 as the `--version` answer of the binary that resumed.
- PROOF-TRANSLATE.md records the rendered rollout's session_meta originator as lys-home and the resume of that rollout by Codex 0.156.0 as succeeded.
- PROOF-TRANSLATE.md holds the rollout's and the account's SHA-256 as 64-hex-character values, their line counts, the road that found the thread, the thread-index row count after the resume, the resume command with <codex home>, and the answer-held-token line reading yes.
- PROOF-TRANSLATE.md names each of the three kinds left out of the fixture match with a count equal to the number of items of that kind in tests/fixtures/codex/rollout-0.156.0.jsonl.
- PROOF-TRANSLATE.md holds two equal 64-hex-character SHA-256 values for the Claude Code render before and after.
- A search of PROOF-TRANSLATE.md for the fixture's user text, tool output and assistant texts finds 0 occurrences.

**Files:**
- create: docs/design/home/PROOF-TRANSLATE.md

**Checklist:**
- C30 — PROOF-TRANSLATE.md records Codex 0.156.0 resuming the rendered rollout in an isolated Codex home and answering from its content, in hashes, counts and paths only.

**Stories:**
- S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a loss account beside every translation naming by entry id and block hash what was kept, what changed shape and what was lost and why, and the resume measured on a named Codex version, so that nobody claims a faithful translation that was not measured.

### R11: Carry the narrowed non-goal in the cluster design and re-render its markdown

Structure: the cluster design's non-goal on other harnesses reads `Harnesses other than Claude Code and Codex 0.156.0, and Chat Completions or Responses translation beyond keeping the raw call bytes.`, its structure array names every file this brief creates or modifies, and docs/design/home/DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/HOME-004.md are re-rendered by the method's render-cluster.py from their JSON after the last change to it, and never edited by hand. HOME-001's brief keeps its own boundary as written, since it bounded HOME-001. THE SYSTEM SHALL NOT edit a rendered markdown file by hand and SHALL NOT change HOME-001.json.

**Acceptance:**
- docs/design/home/design.json has exactly one non-goal whose text begins `Harnesses other than`, and its text is `Harnesses other than Claude Code and Codex 0.156.0, and Chat Completions or Responses translation beyond keeping the raw call bytes.`
- `git diff --exit-code main -- docs/design/home/briefs/HOME-001.json` exits 0.
- `sh scripts/design/gate.sh` exits 0 from the repository root.

**Files:**
- modify: docs/design/home/design.json
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md
- modify: docs/design/home/briefs/HOME-004.md

**Checklist:**
- C24 — lys-home render-codex takes a home, a session, a target directory that is a Codex sessions root, and a Codex version; renders only for the measured Codex 0.156.0 and refuses any other version naming the version asked for and the one measured; walks the context path the Claude Code render walks; and writes one rollout under that root's dated directories in the shape Codex 0.156.0 writes for its own threads, measured from files that version wrote, with nothing written when it refuses.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose session was imported from Claude Code, I want it rendered as a Codex rollout that Codex resumes, with every text part, tool call and tool result carried whole and the thread announcing itself as a translation, so that I can continue on another harness and never mistake the fork for the original.

## Boundaries

- No Codex to Claude Code translation, no Pi or third harness, and no launch of Codex through a launch template.
- Codex's importer of Claude Code files is not adopted, vendored, linked or called, and no Codex code is copied.
- No rollout shape is taken from a Codex source checkout or from a rollout another Codex version wrote; only files Codex 0.156.0 wrote are the reference.
- No Codex version other than 0.156.0 is rendered for.
- render.rs, import.rs and events.rs do not change, and the Claude Code render's output and loss account do not change by a byte.
- The home session file, its blocks and the original Claude Code file are never rewritten; the translation's side leaf is an append beside the head.
- Nothing is written under the user's own Codex home, no thread-index row is written by hand and none outside the given Codex home (the render writes one only on R1's measured need, in R1's measured shape), and lys-home never runs Codex.
- No transcript content in the report, the account, an error, a log line, a test name or the proof; paths, ids, hashes and counts only.
- No field is added to Pi's grammar; the translation record rides inside a lys.harness_event custom entry.
- An image part is carried as input_image only when its source is base64; any other image source is accounted lost and never fetched, and nothing is read beyond the session file and the block store.
- No developer instructions, environment context, reasoning with encrypted content or base_instructions is written into a rollout, and no line kind other than session_meta and response_item; the render writes only what a Claude Code record is the source of.
- No new code file exceeds 500 lines of code, and cli.rs stays at or under 500 code lines.
- The design's structure array is the whole file list; a path outside it is not created.
- The rollout's originator is lys-home and no other value; if Codex 0.156.0 refuses it, the build stops and names the refusal.
- The rollout path's date and time are local to the zone the render resolved from its environment and named in the translation record; every timestamp inside the rollout is UTC.

## Verification

- python3 scripts/design/validate.py docs/design/home exits 0.
- python3 scripts/design/check-coverage.py docs/design/home exits 0.
- cargo fmt --all -- --check, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps exit 0.
- sh scripts/design/gate.sh exits 0.
- git diff --exit-code main -- crates/lys-home/src/harness/claude_code/render.rs crates/lys-home/src/harness/claude_code/import.rs crates/lys-home/src/harness/claude_code/events.rs exits 0.
- grep -rn 'fn ' crates/lys-home/src/harness/codex/mod.rs returns nothing.
- grep -rni 'codex' crates/lys-home/Cargo.toml returns nothing.

