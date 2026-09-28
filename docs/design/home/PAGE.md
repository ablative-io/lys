# home — what was asked, what it means, and what was written

## The words, as they were typed

A home is a directory of session files, their indexes and heads, and the blocks store, and today it lives on the machine that captured it and nowhere else. Roadmap stage 3 says a home moves by the same route as a build tree, a pushed ref that the target fetches, never a hand copy, and that the target gets a distinct execution id with its ancestry recorded. This card takes the first block of stage 3, the same-machine proof: a home shipped as a ref to a named remote and fetched into an isolated second home on this Mac, then rendered and resumed there through the Claude Code launch template. The home directory is a git repository whose tracked files are the session files, their index and head files and the blocks store; a lys-home subcommand commits the home's current state and pushes it as one ref to a remote named on the command line, refusing by name a home whose index is stale rather than shipping it, and reporting the commit and the ref it pushed. A second lys-home subcommand fetches that ref from the named remote into a new home directory that must not already hold a home, verifies every index against its file and every head against its index, and appends to each session a lys.harness_event recording the arrival: the source home's commit, the remote and ref it came from, and this home's own execution id, so the ancestry is on the record. Source files are never rewritten by either side, and no credential, token or secret value is ever tracked in the home. With the target home in place the launch template renders it and the printed launch line resumes it, which is the resume evidence stage 3 asks for, measured on this Mac on Claude Code 2.1.281 and written up as a proof document holding hashes, counts and paths only. Acceptance is that a fixture home with a synthetic session ships to a bare remote in a temporary directory and is fetched into a second directory whose session file, index and blocks hash-match the source; that the source home's files are byte-identical before and after; that the target session's last entry is an arrival event naming the source commit and a fresh execution id; that a fetch into a directory already holding a home is refused by name; that a home with a stale index is refused by name on ship and the report says so; that the launch template renders the target home and the launch line resumes it, recorded in the proof document; and that a search of the shipped ref for a fixture secret value finds nothing. Not in scope: the named second machine, which is stage 3's second block and waits on its two preconditions, read authority over the home from the directory step and encryption before bytes leave the machine from the secrets step, each a card on its own board, with this card's shipped ref as what that block fetches; harnesses other than Claude Code; encryption of the home; any transport other than a git ref to a named remote. Filed by Archie on Tom's roadmap stage 3 of 22 September 2026 and his 08:30 rule that a home moves as a pushed ref, on home DESIGN P1, P7 and P8 at lys main 0073b966, at 06:27 on 26 September 2026.

## What the survey found, and its angles

The card asks for the first, same-machine block of roadmap stage 3. A lys home (sessions/, their index and head files, and blocks/) becomes a git repository. A new lys-home subcommand commits the home and pushes it as one ref to a remote named on the command line, and it refuses a home whose index is stale. A second subcommand fetches that ref into a fresh directory, verifies each index and head, and appends an arrival lys.harness_event to each session carrying the source commit, the remote and ref, and a fresh execution id. The target home is then rendered through the existing Claude Code launch template and resumed, and the measurement is written up in a hashes-only proof document.

### What the tree holds

- `crates/lys-home/src/record/mod.rs` — Home::open makes only sessions/ and blocks/, and the session layout is sessions/<id>.jsonl with <id>.index.jsonl, <id>.head and <id>.lock beside it. Ship needs an explicit tracked set that leaves out the .lock files and the templates/ store. Fetch has to decide what 'already holds a home' means before Home::open silently creates the directories.
- `crates/lys-home/src/record/index.rs` — Index::load (line 83) rebuilds a stale or missing index and returns a rebuilt flag, and opening a session writes a missing head (RECORD.md). If ship and fetch reuse the normal open path, they would rewrite beside the source instead of refusing by name. They need a check-only path.
- `crates/lys-home/src/record/beside.rs` — append_beside and append_under hang an entry as a side leaf without moving the head, as ADR-012 requires. This is where the arrival event goes if the head and the session head hash must not change.
- `crates/lys-home/src/harness/claude_code/events.rs` — lys.harness_event has six closed kinds (hook, permission_mode, tool_completed, attachment, system, template_render) with harness 'claude-code', and MAX_DATA_BYTES is 512. An arrival is a seventh kind that is not a Claude Code record. Its detail (a 40-hex commit, a remote URL of any length, a ref and an execution id) has to fit under the cap.
- `crates/lys-home/src/harness/claude_code/launch.rs` — render-launch takes --template as a file path and stores it into the home's templates/ at render. So the target home can render without templates/ being shipped. Render appends template_render and lys.given beside the head, which the proof runs on the target home.
- `crates/lys-home/src/cli.rs` — The subcommand enum lives here, at 424 lines of code against the 500-line rule. given/given-check were already split into cli/given.rs, so ship and fetch belong in their own cli/ module.
- `crates/lys-home/src/error.rs` — HomeError has 29 variants. The new refusals by name (stale index on ship, target already a home, index or head mismatch on fetch, git failure) are added here.
- `crates/lys-home/Cargo.toml` — There is no git dependency today. The workspace rule says all dependencies are pure Rust, so the choice is between shelling out to the git binary and adding a pure-Rust git crate.
- `crates/lys-home/tests/fixtures/launch/` — The synthetic session (5 lines, 1307 bytes) and the handle-only template (use_only LYS_FIXTURE_TOKEN -> handle-fixture-0001) are the fixture home that the acceptance ships, fetches, renders and searches for a secret value.
- `docs/design/home/design.json / DESIGN.md` — Non-Goals lists 'Encryption at rest and moving a home between devices (stage 3 preconditions)'. That item has to be narrowed to the second machine, and Structure has to gain the new files and the HOME-004 brief.
- `docs/design/home/RECORD.md` — This is where each lys custom entry and each harness_event kind is written down. The arrival kind, the tracked set and the execution id are recorded here.
- `docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md:61-75` — Stage 3 text: the same-machine proof comes first, three preconditions, a pushed ref fetched by the target, 'the remote is named in the brief', a distinct execution ID with ancestry, and credentials never copied into the home.
- `docs/design/home/briefs/` — HOME-001..003 exist. This card is HOME-004, rendered by scripts/design/render-brief.py and checked by scripts/design/gate.sh.

### What was already decided

- home P1 — Original bytes are never rewritten. Ship and fetch must leave source session files untouched, and anything added is appended.
- home P6 — A resume path is a per-harness, per-version measurement, and the one recorded is Claude Code 2.1.281 on 24 September.
- home P7 — Transcript contents never appear in a post, log, error, test name or page, so the ship/fetch reports and the proof carry only hashes, counts and offsets.
- home P8 — Credentials are supplied at launch on the target and never carried in the home.
- home CN3 — Only hashes, counts, offsets and event ids appear in output, logs and errors.
- home CN4 — Every home file must still parse with Pi's parseSessionEntries, so the arrival event must be a custom entry.
- home CN7 — The head is persisted beside the file and never inferred. Fetch verifies the head against the index.
- home CN8 — No secret value is written to any file, report, launch line, error or entry. A use-only secret appears only as its handle.
- home Non-Goals — 'Encryption at rest and moving a home between devices (stage 3 preconditions)' is listed as a non-goal. This card moves a home on one machine, so the wording has to be narrowed.
- ADR-012 — Render events hang beside the context path so the session head hash does not move. Advancing the head was rejected, which is the precedent for where the arrival event sits.
- ADR-013 — lys.given is appended after each render and names the version the load order was measured on. The code pins it at 2.1.283 (given.rs MEASURED_VERSION).
- ADR-007 — render-launch prints the launch line and never runs it. The resume is run by hand for the proof.
- CONTEXT-ROADMAP stage 3 — The home moves as a pushed ref fetched by the target, never a hand copy. The remote is named in the brief. The target gets a distinct execution ID with ancestry. There are three preconditions, and the first block is the same-machine proof.
- RECORD.md index/head — A stale index is refused by name and rebuilt on open, and every command that opens a session may write the index or head beside it.

### What was measured

- lys-home source and test lines (src/**/*.rs): 5672 lines across 21 .rs files under src/record, src/cli, and the crate root, plus 22 harness files
- cli.rs code lines (excluding comments and blank lines): 424 of the 500 allowed (525 raw)
- record/mod.rs code lines: 438 of 500 (584 raw)
- HomeError variants: 29
- lys.harness_event kinds today: 6 (hook, permission_mode, tool_completed, attachment, system, template_render)
- harness_event data cap: 512 bytes (MAX_DATA_BYTES)
- Claude Code installed on this Mac: 2.1.283
- Claude Code version the words name for the proof: 2.1.281 (PROOF-RESUME measured 2.1.281, PROOF-GIVEN and given.rs MEASURED_VERSION use 2.1.283)
- git on this Mac: 2.47.1 at /opt/homebrew/bin/git
- git or remote code in lys-home today: 0 occurrences, and no git dependency in Cargo.toml
- execution id concept in lys-home code: none, only in identity design docs (PROVISIONING row 7, CONTEXT-ROADMAP stage 3 and 5b, STATEMENT fork)
- fixture session: 5 lines, 1307 bytes (tests/fixtures/launch/session.jsonl)
- fixture template: 15 lines, 467 bytes, one use-only secret as handle 'handle-fixture-0001'
- existing home briefs: 3 (HOME-001, HOME-002, HOME-003), next is HOME-004
- integration tests in crates/lys-home/tests: 4 files (cached_index 103, claude_code_round_trip 341, launch_template 349, given_record 541 lines)
- home checklist items: 23 (C1–C23), none for stage 3
- roadmap rows for stage 3: 0 (RM-005, RM-006 and RM-007 are the home rows, and none carries a move)
- commit 0073b966 the words cite: exists and is an ancestor of HEAD dfcca65
- files beside each session: 3 (<id>.index.jsonl, <id>.head, <id>.lock), plus dot-prefixed .tmp files in blocks/ during a put
- size of real homes' sources (DESIGN Problem): Archie's file 141,931,097 bytes; Waffles' file 3.37 GB

### What it means for the other projects

- aion — The card runs through aion's chain (brief_card → sign-off → card_build_v3 → src_pr → src_land). The words take the pattern from how aion moves build trees ('the same route as a build tree'): aion pushes refs/staged-rounds/{run_id}/..., which is the precedent for the home's ref namespace. aion's code is not changed.
- cambium — The card sits on a Cambium board. Stage 3's second block waits on two cards on other boards (directory read authority, secrets encryption), which fetch this card's shipped ref, so the ref name and report shape become what those cards depend on.

### The decisions it stands on

- ADR-012 (honour) — The arrival event, like template_render, should hang beside the context path so the head and the session head hash do not move and a render of the target matches a render of the source.
- ADR-013 (honour) — Rendering the target home appends lys.given unchanged, and the recorded harness_version (2.1.283) has to match the version the proof measures.
- ADR-007 (honour) — The launch line is printed and never run. The resume for the proof is run by hand and recorded.
- ADR-001 (honour) — Only handles ever appear in a home or a render, and no credential is tracked or carried to the target (home P8, CN8).
- ADR-004 (honour) — Ship and fetch must work with git alone, without manifold, aion or the broker.
-  (new) — A home is a git repository with an allowlisted tracked set (sessions/*.jsonl, *.index.jsonl, *.head, blocks/), with lock and temporary files never tracked, and it moves only as one pushed ref to a named remote.
-  (new) — An arrival is a seventh lys.harness_event kind, written by fetch and not imported, recording the source commit, the remote and ref, and the target home's execution id. It needs its own decision because it extends the closed kind set and adds execution identity to the home.

### What it requires

- lys-home has a ship subcommand that commits the tracked set of a home and pushes it as one ref to the remote named on the command line, and its JSON report names the commit and the ref.
- Ship refuses a home with any stale index by name, exits 1, writes nothing to the home or the remote, and the report says which session was stale.
- Ship tracks only session files, their index and head files, and blocks/. No .lock file, .tmp file, render output or environment file is in the pushed tree.
- lys-home has a fetch subcommand that fetches the named ref from the named remote into a new directory, and refuses by name a directory that already holds a home.
- Fetch verifies every index against its session file and every head against its index without rebuilding either, and refuses by name on a mismatch.
- After fetch, each target session's last physical entry is a lys.harness_event arrival naming the source commit, the remote, the ref and a fresh execution id, and the data is at most 512 bytes.
- The fixture home's source files are byte-identical before and after ship, measured by SHA-256 of every file.
- The fetched session file, index and blocks hash-match the source (measured as the lead settles relative to the arrival append).
- A search of every object reachable from the shipped ref for the fixture secret value finds zero matches, and a positive control finds a planted value.
- render-launch renders the target home, and the printed launch line resumes it on this Mac's Claude Code. A proof document in docs/design/home records the version, hashes, counts and paths only.
- HOME-004 brief, design.json Structure and Non-Goals, RECORD.md, CHECKLIST and the crate README describe ship, fetch and the arrival kind, and the design gate passes.
- All gates pass: fmt, clippy with and without --all-features, tests --all-features, doc in both shapes, and the design gate.

### What must not change

- No existing session file, index, head or block in the source home is rewritten, truncated or rebuilt by ship or fetch (P1, CN1).
- No credential, token, secret value or remote userinfo is written to any tracked file, entry, report or error (CN8, P8).
- No transcript content in reports, errors, test names or the proof document (P7, CN3).
- Pi's grammar stays unchanged: the arrival is a custom entry, and every home file still parses with parseSessionEntries at 3d5cbe98 (CN4).
- render-launch, template_render and lys.given behaviour and wire shape are unchanged.
- No transport other than git, no encryption, no second machine and no harness other than Claude Code.
- No new non-pure-Rust dependency, no unsafe, no file over 500 lines of code, and no unwrap/expect/panic in library code.

### What we must put in place first

- The lead's answers to the product decisions on the hash-match versus arrival append, the Claude Code version, remote locality, the secret guarantee and templates/.
- A HOME-004 brief in docs/design/home/briefs and a roadmap row for stage 3 block one, going through the aion chain (brief_card, sign-off).

### The risks

- Calling the existing open path (Index::load, Session open) on ship would silently rebuild a stale index or persist a missing head in the source, which breaks both 'byte-identical' and 'refused by name'.
- The user's global git config (hooks, commit signing, core.autocrlf, a missing user.name) could make commit fail or alter bytes. The git invocation must be isolated.
- A remote URL with embedded credentials could be recorded in the arrival event, or appear in a git error echoed into a report.
- Pushing a real home (up to GB-scale blocks) to a remote off the machine ships plaintext transcripts before stage 3's encryption precondition is met.
- Secrets pasted into a conversation live in blocks/ and would be shipped. An allowlist of paths cannot catch them.
- The arrival event changes the target's session file and index, so an acceptance measured naively after fetch cannot hash-match the source.
- Where the arrival sits could move the head and change the session head hash, which diverges renders of source and target (ADR-012).
- A long remote path or URL could push the arrival data past the 512-byte cap and fail on fetch.
- The proof run on 2.1.283 against words naming 2.1.281 leaves the proof and lys.given disagreeing on version.
- git objects are not written with the home's fsync discipline, so an interrupted fetch can leave a partial target that a retry must refuse or clean up by name.
- cli.rs is at 424 code lines, and adding the subcommands inline would breach the 500-line rule.

### Still open

- Should the byte-for-byte match between target and source be checked on the fetched commit before the arrival event is appended, or only as a prefix of the target's session file and index, given that the arrival event makes the target's session file and index differ from the source? The sentence of the words it stands on: "Acceptance is that a fixture home with a synthetic session ships to a bare remote in a temporary directory and is fetched into a second directory whose session file, index and blocks hash-match the source; that the source home's files are byte-identical before and after; that the target session's last entry is an arrival event naming the source commit and a fresh execution id; that a fetch into a directory already holding a home is refused by name; that a home with a stale index is refused by name on ship and the report says so; that the launch template renders the target home and the launch line resumes it, recorded in the proof document; and that a search of the shipped ref for a fixture secret value finds nothing.". Why only the lead can settle it: Fetch appends an arrival lys.harness_event to each target session (record/beside.rs), which adds a line to <id>.jsonl and a row to <id>.index.jsonl. After fetch the target's session file and index cannot hash-match the source, so the acceptance cannot pass as worded. The lead chooses what the match is measured on.
- Should the proof be measured on the installed Claude Code 2.1.283, or must 2.1.281 be installed for it? The sentence of the words it stands on: "With the target home in place the launch template renders it and the printed launch line resumes it, which is the resume evidence stage 3 asks for, measured on this Mac on Claude Code 2.1.281 and written up as a proof document holding hashes, counts and paths only.". Why only the lead can settle it: `claude --version` on this Mac answers 2.1.283. PROOF-GIVEN.md and harness/claude_code/given.rs MEASURED_VERSION pin 2.1.283, and lys.given entries record 2.1.283. A proof on 2.1.281 needs a downgrade and would contradict the version the render records.
- Should ship refuse a remote that is not on this machine (for example a local path or file:// only) until encryption exists? The sentence of the words it stands on: "The home directory is a git repository whose tracked files are the session files, their index and head files and the blocks store; a lys-home subcommand commits the home's current state and pushes it as one ref to a remote named on the command line, refusing by name a home whose index is stale rather than shipping it, and reporting the commit and the ref it pushed.". Why only the lead can settle it: CONTEXT-ROADMAP stage 3 requires encryption before any bytes leave the machine, and the words defer encryption. A remote named on the command line could be a hosted forge, and then plaintext transcripts and blocks would leave the Mac. Whether ship refuses such a remote changes what a person can do with it.
- Is 'no secret value is ever tracked' met by the allowlisted path set (sessions and blocks only, no env or render files), or must ship also scan block and session content and refuse a match? The sentence of the words it stands on: "Source files are never rewritten by either side, and no credential, token or secret value is ever tracked in the home.". Why only the lead can settle it: blocks/ holds raw request and response bodies (lys.call raw_request/raw_response, record/call.rs), and a secret pasted into a conversation lives there. An allowlist cannot keep it out of the shipped ref. A scan needs a list of secret values, which lys-home does not have (secrets stay behind the broker, ADR-001). What a person can safely ship depends on the answer.
- Should templates/ be left out of the shipped home, so the target has none of the source's template objects that its template_render events name by hash? The sentence of the words it stands on: "The home directory is a git repository whose tracked files are the session files, their index and head files and the blocks store; a lys-home subcommand commits the home's current state and pushes it as one ref to a remote named on the command line, refusing by name a home whose index is stale rather than shipping it, and reporting the commit and the ref it pushed.". Why only the lead can settle it: ADR-012 and record/templates.rs keep each rendered template in the home under templates/ by SHA-256. The words' tracked set leaves templates/ out, so a template_render event carried on a shipped session names a template that the target home cannot produce. render-launch still works because it takes --template as a file.

### The units beyond the first

- Stage 3 block two: a home fetched and resumed on a named second machine — This block waits on two preconditions owned by other boards, read authority over the home (directory step) and encryption before bytes leave the machine (secrets step). It fetches the ref this card ships.

### The smallest complete shape

One landable unit, HOME-004, containing:
- lys-home ship and fetch subcommands in their own cli module, with git invoked in isolation, the allowlisted tracked set, a stale-index refusal on ship, and an already-a-home refusal and index/head verification on fetch, none of which writes to the source.
- The arrival event as a seventh lys.harness_event kind hung beside the head, carrying the source commit, the remote, the ref and a fresh per-home execution id.
- An end-to-end test on the launch fixture home: ship to a bare remote in a tempdir, fetch into a second tempdir, hash matches, source byte-identity, the arrival as last entry, both refusals, and a secret search of the pushed objects with a positive control.
- A proof document of the target home rendered through render-launch and resumed by the printed line on this Mac's Claude Code.
- Design, RECORD, checklist and README updates.

## The roadmap row

- **RM-008** — Move a home as a pushed ref: ship, fetch and the same-machine resume proof (stage 3, first block) (feature, idea)
- Summary: A home lives only on the machine that captured it. This item takes stage 3's first block: the home directory becomes a git repository with an allowlisted tracked set, `lys-home ship` commits it and pushes it as one ref to a remote named on the command line, refusing a stale index, a remote off this machine, a named secret value and a match of the five standard secret patterns, and `lys-home fetch` fetches the ref into a new home, checks every index and head without rebuilding, and records an arrival event on each session naming the source commit, the remote, the ref and the new home's execution id. The fetched home is rendered through the Claude Code launch template and resumed by the printed line, measured on the installed Claude Code and written up as hashes, counts and paths.
- Asked by: tom on 2026-09-26T06:27:00+10:00
- Context: A Cambium card on the home cluster for roadmap stage 3's first block, the same-machine proof. The lead's answers settled that the hash match is taken at the fetched commit before the arrival and as byte prefixes after it, that the proof is measured on the installed Claude Code 2.1.283 rather than the 2.1.281 the words name, that ship refuses a remote off this machine until encryption at rest lands, that no secret is tracked by an allowlisted tracked set plus a scan for the named values file's values and five standard patterns, and that templates/ is shipped.
- Quote: A home is a directory of session files, their indexes and heads, and the blocks store, and today it lives on the machine that captured it and nowhere else. Roadmap stage 3 says a home moves by the same route as a build tree, a pushed ref that the target fetches, never a hand copy, and that the target gets a distinct execution id with its ancestry recorded. This card takes the first block of stage 3, the same-machine proof: a home shipped as a ref to a named remote and fetched into an isolated second home on this Mac, then rendered and resumed there through the Claude Code launch template. The home directory is a git repository whose tracked files are the session files, their index and head files and the blocks store; a lys-home subcommand commits the home's current state and pushes it as one ref to a remote named on the command line, refusing by name a home whose index is stale rather than shipping it, and reporting the commit and the ref it pushed. A second lys-home subcommand fetches that ref from the named remote into a new home directory that must not already hold a home, verifies every index against its file and every head against its index, and appends to each session a lys.harness_event recording the arrival: the source home's commit, the remote and ref it came from, and this home's own execution id, so the ancestry is on the record. Source files are never rewritten by either side, and no credential, token or secret value is ever tracked in the home. With the target home in place the launch template renders it and the printed launch line resumes it, which is the resume evidence stage 3 asks for, measured on this Mac on Claude Code 2.1.281 and written up as a proof document holding hashes, counts and paths only. Acceptance is that a fixture home with a synthetic session ships to a bare remote in a temporary directory and is fetched into a second directory whose session file, index and blocks hash-match the source; that the source home's files are byte-identical before and after; that the target session's last entry is an arrival event naming the source commit and a fresh execution id; that a fetch into a directory already holding a home is refused by name; that a home with a stale index is refused by name on ship and the report says so; that the launch template renders the target home and the launch line resumes it, recorded in the proof document; and that a search of the shipped ref for a fixture secret value finds nothing. Not in scope: the named second machine, which is stage 3's second block and waits on its two preconditions, read authority over the home from the directory step and encryption before bytes leave the machine from the secrets step, each a card on its own board, with this card's shipped ref as what that block fetches; harnesses other than Claude Code; encryption of the home; any transport other than a git ref to a named remote. Filed by Archie on Tom's roadmap stage 3 of 22 September 2026 and his 08:30 rule that a home moves as a pushed ref, on home DESIGN P1, P7 and P8 at lys main 0073b966, at 06:27 on 26 September 2026.
- Cluster: home; briefs: HOME-004
- Notes: Further units, not written: Stage 3 block two: a home fetched and resumed on a named second machine.

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

Adopt Pi's session tree as the home record (Tom, Dot 13:27 and 13:28: Pi's tree, not Norn): one append-only JSONL per session, a header line, then entries each carrying id, parentId and timestamp, a leaf pointer for the current position, forks by moving the pointer, compaction and branch summaries as entries that keep their originals. lys adds nothing to that grammar: harness events (approvals, tool completion) and proxy call records ride Pi's custom entry type under lys customType names, so a home file stays readable by Pi's own parser. Two captures feed it: the model traffic through the door's proxy (the same process that swaps the credential, SECRETS-002 R1), and the harness's own events from its transcript. Content blocks are stored once by hash; entries reference them. A harness resume file is a rendered projection of the root-to-leaf path: for Claude Code, a JSONL written under a chosen uuid at the harness's own path, then resumed by that id with --fork-session. Provider-native reasoning stays on the message with its provider, api and model and is rendered whole only to the same three; another model gets readable thinking as text and opaque blocks dropped, each named in a loss account. lys grants say who may read and resume. Every resume path is measured on a named harness version before anything relies on it. A session is launched on a harness from a launch template kept in the home (ADR-012): one JSON object per harness, schema in docs/design/home/launch-template.schema.json, stored under templates/ by its SHA-256 beside sessions/ and blocks/, so a template's version is its hash. Its named slots map the home onto the harness: the transcript (the one slot no template maps generically, so the template names how the harness fills it; Claude Code fills it by resuming the rendered file by path with --fork-session, since a bare resume writes onto the rendered file's own name), the MCP configuration, the environment, the secrets (use-only ones written as their handle; readable ones refused until the secrets rows build the broker reader, ADR-001) and the appended instructions. lys-home render-launch reads a template and a session, writes the R4 render, its loss account, an MCP file, an environment file and an instructions file into one directory, and prints the launch line in its report without running it (ADR-007). Each render is recorded on the session as a lys.harness_event of the sixth kind, template_render, hung as a side leaf beside the context path so the head and the session head hash (SHA-256 of the head entry's line) do not move; the written paths ride in a manifest block the event names by hash, under the 512-byte cap. At render, the launch template also records the context record: a lys.given custom entry after the render event naming, by path, byte length and SHA-256 only, the instruction documents Claude Code will load for the session's working directory and the files the render wrote, in the order measured on a named Claude Code version, with the environment variable names the template set and the kinds of document resolved and left unlisted (@-imports and .claude/rules, which a second lys.given entry at the first request records), so a reader sees what was measured and can check a file on disk against it without anyone reading its contents. A home moves by the route a build tree takes (ADR-014): the home directory is a git repository whose tracked set is an allowlist (sessions/<id>.jsonl, its index and head files, blocks/ and templates/, never a lock, temporary, environment or render file), `lys-home ship` commits that set and pushes it as one ref, refs/lys-home/<commit>, to a remote named on the command line, and `lys-home fetch` fetches the ref into a new home, checks every index and head without rebuilding either, and records the arrival on each session as a seventh lys.harness_event kind beside the head (ADR-015), naming the source commit, the remote, the ref and the target home's own execution id. Until encryption at rest exists, ship refuses a remote off this machine and any tracked file holding a named secret value.

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
- ADR-014 — A home moves as one pushed git ref of an allowlisted tracked set, never a hand copy — A home is a git repository whose tracked set is an allowlist: sessions/<id>.jsonl, <id>.index.jsonl and <id>.head, the files under blocks/ and the files under templates/, never a lock, temporary, environment, render or execution-id file. `lys-home ship` commits exactly that set with git run apart from the person's configuration and pushes it without force as one ref, refs/lys-home/<commit>, to a remote named on the command line; `lys-home fetch` fetches that ref into a new home. Rejected: a hand copy or an archive of the directory, which names no commit and records no ancestry; tracking the whole directory, which ships whatever else is in it; and a pure-Rust git crate, which adds a dependency where the git binary suffices. Until encryption at rest exists, ship refuses a remote off this machine and any tracked file holding a value of the named secret values file or matching one of five standard secret patterns (private key header, sk- token, GitHub token, AWS access key id, JWT).
- ADR-015 — An arrival is a seventh lys.harness_event kind, hung beside the head and written by fetch — Fetch appends to each session one lys.harness_event of kind `arrival`, with source_uuid and record null and a detail of exactly source_commit, remote, ref and execution_id, hung beside the head with append_beside so the head and the session head hash do not move; the execution id is one per target home, fresh at fetch, kept in the untracked file <home>/execution-id. The remote is recorded as a canonical path, a file:/// URL, or a URL without userinfo, and the data stays within the 512-byte cap. Rejected: appending the arrival on the context path, which moves the head (ADR-012); a new custom entry type, which adds a type where the harness event carries the same shape; and one execution id per session, where the words give the home one.

## Goals

- A few-shot session file written by hand resumes Claude Code by path, from a directory outside the config root, with the file preserved and the demonstration marked authored.
- One real Claude Code session imported into the common record, rendered back, and resumed under a new id on 2.1.281 without repeating a completed tool action, with the original file's hash unchanged.
- One seat with a subscription login making calls through a pass-through proxy, so the tee has somewhere to live.
- A written loss account for every derived record: what a render or translation preserved, transformed and could not carry.
- The canon, one curated versioned series of short examples each showing a rule lived, seeds every new session, and its effect is measured on a card against a plain start.
- A handover letter from an outgoing session seeds its successor as inherited memory, with the model's own thinking intact, and the effect is measured on a card against a plain start.
- A session in the home renders, from a Claude Code launch template kept in the home by hash, into a directory of launch files whose hashes are identical on a second render, and the printed launch line resumes it on the installed Claude Code version, measured.
- Every launch-template render records what the session was given: one lys.given entry after the render event naming each instruction document Claude Code will load, and each file the render wrote, by path, byte length and SHA-256 in the measured order, with the environment variable names the template set, and never a document's content; lys-home lists those records and checks a file on disk against one by hash.
- A home ships as one pushed git ref to a named remote on this machine and is fetched into a second home whose tracked files match the source at the fetched commit, with an arrival on each session naming the source commit and a fresh execution id; the fetched home renders through the launch template and the printed launch line resumes it on the installed Claude Code version, measured.

## Non-Goals

- Anchoring, signing or receipts into a lys log (CONTEXT-ROADMAP stage 6; when asked for). — Signing comes when asked for (Tom, 22 September 16:27); every stage here works without it.
- Encryption at rest, and moving a home to a named second machine (stage 3's second block and its preconditions). — Stage 3's second block waits on read authority over the home and on encryption before bytes leave the machine, each a card on its own board; this cluster's move is the same-machine proof, whose shipped ref that block fetches.
- Harnesses other than Claude Code, and Chat Completions or Responses translation beyond keeping the raw call bytes. — One harness proved first; each other harness is its own profile and its own measurement.
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
| `crates/lys-home/Cargo.toml` | the crate manifest; gains the passthrough example | HOME-001 |
| `crates/lys-home/Cargo.toml` | the crate manifest; gains regex for the five standard secret patterns | HOME-004 |
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
| `docs/design/home/briefs/HOME-004.json` | the fourth brief: ship and fetch a home as one git ref, the arrival event, the same-machine resume proof | HOME-004 |
| `docs/design/home/briefs/HOME-004.md` | its rendered markdown | HOME-004 |
| `docs/design/home/PROOF-MOVE.md` | the measured move: the fixture home shipped, fetched, rendered and resumed on the installed Claude Code, as hashes, counts and paths | HOME-004 |
| `crates/lys-home/src/record/verify.rs` | an index and a head checked against their session file without rebuilding or writing anything | HOME-004 |
| `crates/lys-home/src/record/verify_tests.rs` | gates on the check-only verification: stale, missing and unindexed refused, nothing written | HOME-004 |
| `crates/lys-home/src/moves/mod.rs` | the move module: declarations only | HOME-004 |
| `crates/lys-home/src/moves/git.rs` | the git binary run with the person's git configuration, hooks, signing and prompts shut out | HOME-004 |
| `crates/lys-home/src/moves/git_tests.rs` | gates on the isolated git runner | HOME-004 |
| `crates/lys-home/src/moves/remote.rs` | the remote named: ship stays on this machine, no remote carries userinfo, the form the arrival records | HOME-004 |
| `crates/lys-home/src/moves/remote_tests.rs` | gates on the remote rules | HOME-004 |
| `crates/lys-home/src/moves/tracked.rs` | the tracked set by allowlist, and the scan for the named secret values and the five standard patterns | HOME-004 |
| `crates/lys-home/src/moves/tracked_tests.rs` | gates on the tracked set, the value scan and each pattern with its near miss | HOME-004 |
| `crates/lys-home/src/moves/ship.rs` | ship: check, scan, commit the tracked set, push one ref; the table of the five standard secret patterns | HOME-004 |
| `crates/lys-home/src/moves/fetch.rs` | fetch: into a staging directory, verify, record the arrival, rename into place | HOME-004 |
| `crates/lys-home/src/cli/moves.rs` | the ship and fetch subcommands, split out of cli.rs | HOME-004 |
| `crates/lys-home/tests/home_ship.rs` | ship through the built binary: the report, the pushed tree, each refusal with nothing written | HOME-004 |
| `crates/lys-home/tests/home_fetch.rs` | fetch through the built binary: the arrival, the execution id, each refusal with nothing left behind | HOME-004 |
| `crates/lys-home/tests/home_move.rs` | the move end to end on the fixture home: hash match at the fetched commit, prefixes after, source unchanged, secret search with its control | HOME-004 |
| `Cargo.toml` | the workspace manifest; gains regex among the workspace dependencies | HOME-004 |
| `Cargo.lock` | the lockfile; gains regex and what it resolves to | HOME-004 |

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
title: Move a home as one pushed git ref: ship, fetch with an arrival event, and the same-machine resume proof
---

# HOME-004: Move a home as one pushed git ref: ship, fetch with an arrival event, and the same-machine resume proof

> **Cluster:** home
> **Depends on:** HOME-001, HOME-002, HOME-003
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-013 — The context record is a lys.given custom entry of document hashes, never copies — The context record is one lys.given custom entry, appended after the render event, whose data is the harness name, the Claude Code version the load order was measured on, the kinds as two lists, resolved (claude_md_chain, user_claude_md, memory_index, appended_instructions, mcp_config, environment_names) and unlisted (claude_md_imports and claude_rules, which this entry does not list and a later entry at the first request records), the config directory as its path and its source (template or home), the documents in the measured order each as kind, path, byte length and SHA-256, and the names of the environment variables the template set. It is not a copy of each document into the block store, and not a content-bearing record, because the entry must hold no content under home P7 and CN3. It is unsigned and unencrypted now, and because it names hashes only, signing and encryption at rest can be added later without changing what is recorded.
> - ADR-014 — A home moves as one pushed git ref of an allowlisted tracked set, never a hand copy — A home is a git repository whose tracked set is an allowlist: sessions/<id>.jsonl, <id>.index.jsonl and <id>.head, the files under blocks/ and the files under templates/, never a lock, temporary, environment, render or execution-id file. `lys-home ship` commits exactly that set with git run apart from the person's configuration and pushes it without force as one ref, refs/lys-home/<commit>, to a remote named on the command line; `lys-home fetch` fetches that ref into a new home. Rejected: a hand copy or an archive of the directory, which names no commit and records no ancestry; tracking the whole directory, which ships whatever else is in it; and a pure-Rust git crate, which adds a dependency where the git binary suffices. Until encryption at rest exists, ship refuses a remote off this machine and any tracked file holding a value of the named secret values file or matching one of five standard secret patterns (private key header, sk- token, GitHub token, AWS access key id, JWT).
> - ADR-015 — An arrival is a seventh lys.harness_event kind, hung beside the head and written by fetch — Fetch appends to each session one lys.harness_event of kind `arrival`, with source_uuid and record null and a detail of exactly source_commit, remote, ref and execution_id, hung beside the head with append_beside so the head and the session head hash do not move; the execution id is one per target home, fresh at fetch, kept in the untracked file <home>/execution-id. The remote is recorded as a canonical path, a file:/// URL, or a URL without userinfo, and the data stays within the 512-byte cap. Rejected: appending the arrival on the context path, which moves the head (ADR-012); a new custom entry type, which adds a type where the harness event carries the same shape; and one execution id per session, where the words give the home one.
> **Checklist:**
> - C24 — lys-home ship commits the home's tracked set (session files, their index and head files, blocks and templates) and pushes it as one ref, refs/lys-home/<commit>, to the remote named on the command line, and its report names the commit and the ref.
> - C25 — Ship refuses by name a home whose index is stale or whose head file is missing, exits 1, and writes nothing to the home or the remote; no index is rebuilt and no head persisted.
> - C26 — Ship refuses by name any remote that is not a path on this machine or a file:/// URL, naming its scheme and the encryption-at-rest precondition, and writes nothing.
> - C27 — No lock, temporary, environment or render file is in a shipped tree, and ship refuses by file and offset a tracked file holding any value of the --secret-values file or matching one of the five standard patterns, naming the pattern and printing no value or matched byte.
> - C28 — lys-home fetch fetches the named ref into a new directory, refuses by name a directory that already holds a home (a sessions, blocks or templates directory, a .git entry, or an execution-id file) and accepts one holding only other entries, and checks every index against its file and every head against its index without rebuilding either, leaving nothing behind on a refusal.
> - C29 — Fetch appends to each session, beside the head, one lys.harness_event of kind arrival naming the source commit, the remote without userinfo, the ref and the target home's execution id, within the 512-byte cap.
> - C30 — On the fixture home, every tracked file at the fetched commit equals its source byte for byte, the source session file and index are byte prefixes of the target's after the arrival, the source home's files are unchanged by ship and fetch, and a search of every object in the remote finds no fixture secret value while a planted one is found.
> - C31 — PROOF-MOVE.md records the fetched home rendered by render-launch and resumed by the printed launch line on Claude Code 2.1.283, as hashes, counts and paths only.
> **Stories:**
> - S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home carried to a new home as a pushed ref that the new home fetches, so that I continue from my own record rather than from a hand copy.
> - S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every fetched session to record the commit, remote and ref it came from and the new home's execution id, so that a moved home's ancestry is on the record.
> - S16 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a ship to refuse a home holding a named secret value, so that no credential leaves in a shipped ref.
> - S17 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the moved home's resume measured on a named Claude Code version and written in a proof document, so that the move is shown to continue a session rather than assumed to.

## Purpose

A home lives only on the machine that captured it. Stage 3 of the context roadmap moves a home by the route a build tree takes, a pushed ref the target fetches, never a hand copy, and gives the target a distinct execution id with its ancestry on the record (ADR-014, ADR-015). This brief takes stage 3's first block, the same-machine proof: `lys-home ship` commits the home's tracked set and pushes it as one ref to a remote named on the command line, refusing a stale index, a remote off this machine and a named secret value; `lys-home fetch` fetches that ref into a new home, checks every index and head without rebuilding, and records an arrival on each session beside the head; and the fetched home is rendered through the Claude Code launch template and resumed by the printed line, measured and written up as hashes, counts and paths.

## Task

Build, in dependency order: a check-only index and head verification (R1), an isolated git runner (R2), the remote rules (R3), the tracked set and the secret-value scan (R4), the arrival event kind (R5), the ship subcommand (R6), the fetch subcommand (R7), the end-to-end test on the fixture home (R8), the measured proof (R9), and the record, README and re-rendered cluster markdown (R10).

What the lead's answers settle and how this brief reads them. The hash match is taken on the fetched files before the arrival is appended: every tracked file at the fetched commit equals the source byte for byte; after the arrival, the source session file and index are a byte prefix of the target's and the blocks are unchanged (R8). The proof is measured on the installed Claude Code 2.1.283, the version MEASURED_VERSION pins and lys.given records; the 2.1.281 in the words is superseded, with no downgrade (R9). Until encryption at rest lands with stage 3's precondition card, ship refuses by name any remote that is not a path on this machine or a file:// URL, naming the remote's scheme and that card, and the acceptance ships to a bare local remote (R3, R6). No secret value is tracked by two mechanisms and no third: the tracked set is an allowlist of session files, their index and head files, blocks and templates, never an environment or render file (R4); and ship scans every tracked file's bytes, blocks included, for the values of the file named by --secret-values and for the five standard patterns, refusing on a hit by file and offset (R4, R6). templates/ is shipped, so the target resolves every template_render event's template by its hash. Fetch refuses only a directory that holds a home, by its markers: a sessions, blocks or templates directory, a .git entry, or an execution-id file, which is the first thing a home writes and so marks a home on its own; an empty directory, or one holding only other entries such as a lone note file, is accepted and the home is created beside them, the refusal names the entry that made the directory a home, except that a directory marked only by its execution-id file is refused naming the directory and not the file, and nothing is written under a refused target (R7). A refusal for a rule of the home is never HomeError::Io, which stays for the file system failing.

The five standard patterns are named: private_key_header `-----BEGIN [A-Z ]*PRIVATE KEY-----`, sk_token `sk-[A-Za-z0-9_-]{20,}`, github_token `gh[pousr]_[A-Za-z0-9]{36,}`, aws_access_key_id `AKIA[0-9A-Z]{16}` and jwt `eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}`, byte regexes run case-sensitive over every tracked file's bytes. They are a constant table in the ship module, one row per pattern with its name and regex, so a sixth is one row; a hit refuses by file path, byte offset and pattern name and never prints the matched bytes, and the named values file's values are matched as exact bytes alongside the five (R4, R6).

What the words settle. Ship refuses a stale index by name and does not rebuild it, and its report says so: a stale refusal prints a report naming each stale session file under `stale_index` (R6); fetch refuses a directory that already holds a home by name; the execution id is one per target home, carried on each session's arrival; and the only transport is a git ref pushed to and fetched from a named remote. Out of scope: the named second machine (stage 3's second block, which waits on read authority over the home and on encryption before bytes leave the machine, and fetches this brief's ref), harnesses other than Claude Code, encryption of the home, and any other transport.

The ordinary choices this brief makes. git is the git binary spawned with the person's configuration shut out, not a new crate. The five patterns are compiled with the pure-Rust regex crate's byte regexes, added as a workspace dependency, which is the one dependency this brief adds. When one tracked file holds more than one hit, the refusal names the smallest offset, and a value's hit is named `named_value` in place of a pattern name. The ref is refs/lys-home/<commit>, named by the commit it points at, so two homes never collide on one remote and no push needs force; the home's own branch refs/heads/lys-home chains its ships. A session whose head file is missing is refused by name on ship, since fetch verifies every head. The execution id is record::fresh_id (32 lowercase hex) kept in <home>/execution-id, which is not in the tracked set. The arrival hangs beside the head as template_render does (ADR-012), so the head and the session head hash do not move, and carries the remote as ship_remote or fetch_remote records it: a canonical path, a file:/// URL, and never userinfo. Fetch works in a staging directory beside the target and moves the staged entries into place only when every check has passed, so a refused fetch leaves nothing a retry must clean up.

## Requirements

### R1: Check a session's index and head without writing anything

Add `verify_session(session_file: &Path) -> Result<VerifiedSession, HomeError>` in crates/lys-home/src/record/verify.rs, declared from record/mod.rs, where VerifiedSession carries the session id, the entry count and the head (`Option<String>`). WHEN called, THE SYSTEM SHALL read the session file's header line, the index file beside it and the head file beside it, and SHALL return Ok only when the index rows are this file's under the rules a cached index is already held to by Index::load (each row starts where the one before ended, has a nonzero length, an id not yet seen and a parent already indexed that is not itself, ends on a newline byte of the session file, and the last row ends where the file ends) and the head file names an id the index holds, or is empty for a session whose head is the header. IF the index file is missing, unreadable as rows, or disagrees with the file under those rules, THEN THE SYSTEM SHALL refuse with HomeError::StaleIndex naming the session file. IF the head file is missing, THEN THE SYSTEM SHALL refuse with a new HomeError::HeadMissing whose message contains `head_missing` and the session file. IF the head file names an id the index does not hold, THEN THE SYSTEM SHALL refuse with a new HomeError::HeadNotIndexed whose message contains `head_not_indexed`, the session file and that id. THE SYSTEM SHALL NOT rebuild an index, persist a head, create a lock file or write any file; SHALL NOT call Index::load, Session::open or Home::open; SHALL NOT read an entry line beyond the byte at each row's end; and SHALL NOT put any entry content in an error.

**Acceptance:**
- On a home holding the fixture session tests/fixtures/launch/session.jsonl as sessions/fixture.jsonl, opened once with Session::open so its index and head exist, verify_session(sessions/fixture.jsonl) returns entries 4 and head Some("e4").
- After the 8 bytes `{"x":1}` plus a newline are appended to that session file, verify_session returns Err(HomeError::StaleIndex) whose path is that session file, and the SHA-256 of fixture.jsonl, fixture.index.jsonl and fixture.head and the sorted listing of sessions/ are the same before and after the call.
- With fixture.index.jsonl deleted, verify_session returns Err(HomeError::StaleIndex) and sessions/ holds no fixture.index.jsonl after the call.
- With fixture.head deleted, verify_session returns Err(HomeError::HeadMissing) and sessions/ holds no fixture.head after the call.
- With fixture.head holding `e9` plus a newline, verify_session returns Err(HomeError::HeadNotIndexed) whose Display contains `head_not_indexed` and `e9`.
- The verify tests run five cases and assert that exactly four of them are refused.

**Files:**
- create: crates/lys-home/src/record/verify.rs
- create: crates/lys-home/src/record/verify_tests.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C25 — Ship refuses by name a home whose index is stale or whose head file is missing, exits 1, and writes nothing to the home or the remote; no index is rebuilt and no head persisted.
- C28 — lys-home fetch fetches the named ref into a new directory, refuses by name a directory that already holds a home (a sessions, blocks or templates directory, a .git entry, or an execution-id file) and accepts one holding only other entries, and checks every index against its file and every head against its index without rebuilding either, leaving nothing behind on a refusal.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home carried to a new home as a pushed ref that the new home fetches, so that I continue from my own record rather than from a hand copy.

### R2: Run git isolated from the person's git configuration

Add the module crates/lys-home/src/moves/ (mod.rs holding only module docs and `pub mod` lines, declared from lib.rs) and in moves/git.rs a `Git` runner for one directory that spawns the `git` binary found on PATH. EVERY git invocation SHALL carry the environment GIT_CONFIG_GLOBAL=/dev/null, GIT_CONFIG_NOSYSTEM=1 and GIT_TERMINAL_PROMPT=0, and SHALL begin its arguments with `-c core.hooksPath=/dev/null -c commit.gpgSign=false -c core.autocrlf=false -c user.name=lys-home -c user.email=lys-home@localhost`. IF git cannot be spawned, or exits with a status other than 0, THEN THE SYSTEM SHALL refuse with a new HomeError::Git whose message is one line containing `git_failed`, the step name the caller gave (for example `push`) and the exit status. THE SYSTEM SHALL NOT carry git's stdout or stderr into an error or a report, SHALL NOT read the person's global or system git configuration, SHALL NOT run a hook, SHALL NOT sign, SHALL NOT prompt, and SHALL NOT add a dependency for git to crates/lys-home/Cargo.toml.

**Acceptance:**
- The std::process::Command a Git runner builds for step `rev-parse` has get_envs() holding GIT_CONFIG_GLOBAL=/dev/null, GIT_CONFIG_NOSYSTEM=1 and GIT_TERMINAL_PROMPT=0, and its first ten get_args() are `-c core.hooksPath=/dev/null -c commit.gpgSign=false -c core.autocrlf=false -c user.name=lys-home -c user.email=lys-home@localhost`.
- Running step `rev-parse` with arguments `--verify HEAD` in an empty temporary directory that is not a repository returns Err(HomeError::Git) whose Display contains `git_failed` and `rev-parse` and holds no newline.
- `grep -n 'fn ' crates/lys-home/src/moves/mod.rs` prints nothing.
- `grep -nE '^(git2|gix)' crates/lys-home/Cargo.toml` prints nothing, and `grep -nE '^name = "(git2|gix)"' Cargo.lock` prints nothing.

**Files:**
- create: crates/lys-home/src/moves/mod.rs
- create: crates/lys-home/src/moves/git.rs
- create: crates/lys-home/src/moves/git_tests.rs
- modify: crates/lys-home/src/lib.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C24 — lys-home ship commits the home's tracked set (session files, their index and head files, blocks and templates) and pushes it as one ref, refs/lys-home/<commit>, to the remote named on the command line, and its report names the commit and the ref.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home carried to a new home as a pushed ref that the new home fetches, so that I continue from my own record rather than from a hand copy.

### R3: Name the remote: ship stays on this machine, and no remote carries userinfo

In crates/lys-home/src/moves/remote.rs add `ship_remote(text) -> Result<Remote, HomeError>` and `fetch_remote(text) -> Result<Remote, HomeError>`, where Remote carries the argument handed to git and the form recorded in the arrival event. For ship: WHEN the text begins `file:///` (a file URL with an empty authority), THE SYSTEM SHALL accept it and record it as given; WHEN the text holds `://` with any other scheme, or begins `file://` with a nonempty authority, THE SYSTEM SHALL refuse with a new HomeError::RemoteOffMachine whose message contains `remote_off_machine`, the scheme (the text before `://`), and that ship accepts only a path on this machine or a file:// URL until encryption at rest lands with stage 3's encryption precondition card; WHEN the text holds no `://` and a `:` stands before its first `/`, THE SYSTEM SHALL refuse with RemoteOffMachine naming the scheme `ssh`, as git reads host:path as ssh; OTHERWISE the text is a path on this machine, and THE SYSTEM SHALL resolve it with std::fs::canonicalize, record the canonical absolute path, and refuse a path that does not resolve with HomeError::Io naming it. For fetch: WHEN a URL's authority holds `@`, or the text holds no `://` and an `@` stands before a `:` that stands before its first `/`, THE SYSTEM SHALL refuse with a new HomeError::RemoteUserinfo whose message contains `remote_userinfo` and the scheme; a path is resolved and recorded as for ship; every other remote is recorded as given. THE SYSTEM SHALL NOT put a remote's authority, userinfo, host or any part after the scheme into an error; SHALL NOT run git or contact a remote while naming one; and SHALL NOT accept an `https`, `http`, `ssh` or `git` remote for ship.

**Acceptance:**
- ship_remote on the path of an existing bare repository in a temporary directory returns a Remote whose recorded form equals std::fs::canonicalize of that path.
- ship_remote("file:///tmp/lys-remote.git") returns a Remote whose recorded form is `file:///tmp/lys-remote.git`.
- ship_remote("https://example.invalid/home.git") returns Err(HomeError::RemoteOffMachine) whose Display contains `remote_off_machine`, `https` and `encryption at rest`.
- ship_remote("ssh://example.invalid/home.git") and ship_remote("example.invalid:home.git") each return Err(HomeError::RemoteOffMachine) whose Display contains `ssh`.
- ship_remote("file://user:tok3n@example.invalid/home.git") returns Err(HomeError::RemoteOffMachine) whose Display contains `file` and contains neither `tok3n` nor `example.invalid`.
- fetch_remote("https://user:tok3n@example.invalid/home.git") returns Err(HomeError::RemoteUserinfo) whose Display contains `remote_userinfo` and `https` and does not contain `tok3n`.
- fetch_remote("git@example.invalid:home.git") returns Err(HomeError::RemoteUserinfo) whose Display contains `ssh` and does not contain `example.invalid`.
- fetch_remote("https://example.invalid/home.git") returns a Remote whose recorded form is `https://example.invalid/home.git`.
- ship_remote on a relative path that does not exist returns Err(HomeError::Io).
- The remote tests assert that exactly seven of their ten cases are refused.

**Files:**
- create: crates/lys-home/src/moves/remote.rs
- create: crates/lys-home/src/moves/remote_tests.rs
- modify: crates/lys-home/src/moves/mod.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C26 — Ship refuses by name any remote that is not a path on this machine or a file:/// URL, naming its scheme and the encryption-at-rest precondition, and writes nothing.
- C29 — Fetch appends to each session, beside the head, one lys.harness_event of kind arrival naming the source commit, the remote without userinfo, the ref and the target home's execution id, within the 512-byte cap.

**Stories:**
- S16 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a ship to refuse a home holding a named secret value, so that no credential leaves in a shipped ref.

### R4: Name the tracked set and scan it for the named values and the five standard patterns

In crates/lys-home/src/moves/tracked.rs add `tracked_set(home_root) -> Result<Vec<PathBuf>, HomeError>` returning home-relative paths sorted bytewise: for every sessions/<id>.jsonl whose name does not end `.index.jsonl`, that file, sessions/<id>.index.jsonl and sessions/<id>.head; every file under blocks/<hh>/; and every file under templates/<hh>/; where no file name beginning `.` is taken. THE SYSTEM SHALL NOT include a session's lock file (the path Index::lock_path names), a `.tmp` file, a file at the home's root, the execution-id file, or any file under a directory other than sessions/, blocks/ and templates/ (render output, an environment file, an MCP file, the .git directory). In crates/lys-home/src/moves/ship.rs add the constant table STANDARD_PATTERNS, one row per pattern holding its name and its regex, with exactly these five rows in this order: private_key_header `-----BEGIN [A-Z ]*PRIVATE KEY-----`; sk_token `sk-[A-Za-z0-9_-]{20,}`; github_token `gh[pousr]_[A-Za-z0-9]{36,}`; aws_access_key_id `AKIA[0-9A-Z]{16}`; jwt `eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}`. Each regex is compiled as a case-sensitive byte regex with the regex crate (regex::bytes), added to the workspace's dependencies and to crates/lys-home/Cargo.toml as `regex.workspace = true`; a row that does not compile is refused with a new HomeError::SecretPattern naming the row's name, never with a panic. In tracked.rs add `read_values(file) -> Result<Vec<Vec<u8>>, HomeError>` taking one value per line with the line ending removed and empty lines skipped, and `scan(home_root, set, values, patterns) -> Result<(), HomeError>`. WHEN any value's bytes occur in a tracked file, or any pattern's regex matches a tracked file's bytes, THE SYSTEM SHALL refuse with a new HomeError::SecretInHome whose message contains `secret_in_home`, the home-relative path of the first tracked file in set order holding a hit, the byte offset of the earliest hit in that file, and what hit there: the pattern's name for a pattern, `named_value` for a value (a value before a pattern, and patterns in table order, when two hits start at one offset). THE SYSTEM SHALL NOT put a value, the matched bytes, their length or the bytes around a hit into an error, a report or a log line; SHALL NOT scan for anything but the values and the table's patterns; and SHALL NOT write any file.

**Acceptance:**
- `git diff <base> -- crates/lys-home/Cargo.toml`, where <base> is the brief's base commit, adds exactly one line, `regex.workspace = true`, and removes none.
- On the fixture home after one render-launch of tests/fixtures/launch/template.json, tracked_set returns exactly sessions/fixture.jsonl, sessions/fixture.index.jsonl and sessions/fixture.head, one path per file under blocks/ whose name does not begin `.`, and one path under templates/, in bytewise order.
- After the test creates blocks/ab/.tmp-x, env.json at the home root, render/env.json and the path Index::lock_path names for sessions/fixture.jsonl, tracked_set returns the same list as before they were created.
- read_values on a file holding `a`, a newline, a newline, `b` and a newline returns [b"a", b"b"].
- STANDARD_PATTERNS holds exactly five rows, and their names in order are private_key_header, sk_token, github_token, aws_access_key_id and jwt, each with the regex this requirement gives it, character for character.
- scan over that home's tracked set with the values [b"fixture-secret-value-0001"] and STANDARD_PATTERNS returns Ok(()).
- After the test writes blocks/zz/plant holding `0123fixture-secret-value-0001` into a copy of that home, scan over the copy's tracked set returns Err(HomeError::SecretInHome) whose Display contains `blocks/zz/plant`, `offset 4` and `named_value` and does not contain `fixture-secret-value-0001`.
- For each of the five plants `-----BEGIN OPENSSH PRIVATE KEY-----`, `sk-` followed by 20 × `A`, `ghp_` followed by 36 × `A`, `AKIA` followed by 16 × `A`, and `eyJ` + 10 × `a` + `.eyJ` + 10 × `a` + `.` + 10 × `a`, written as `0123` followed by the plant into blocks/zz/plant of a fresh copy of that home, scan with no values returns Err(HomeError::SecretInHome) whose Display contains `blocks/zz/plant`, `offset 4` and, respectively, private_key_header, sk_token, github_token, aws_access_key_id and jwt, and does not contain the plant.
- For each of the five near misses `-----BEGIN PUBLIC KEY-----`, `sk-` followed by 19 × `A`, `ghp_` followed by 35 × `A`, `AKIA` followed by 15 × `A`, and `eyJ` + 10 × `a` + `.eyJ` + 10 × `a` + `.` + 9 × `a`, written the same way, scan with no values returns Ok(()).
- The pattern tests run ten plant cases and assert that exactly five of them are refused, one naming each of the five patterns.

**Files:**
- create: crates/lys-home/src/moves/tracked.rs
- create: crates/lys-home/src/moves/tracked_tests.rs
- create: crates/lys-home/src/moves/ship.rs
- modify: crates/lys-home/src/moves/mod.rs
- modify: crates/lys-home/src/error.rs
- modify: crates/lys-home/Cargo.toml
- modify: Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C27 — No lock, temporary, environment or render file is in a shipped tree, and ship refuses by file and offset a tracked file holding any value of the --secret-values file or matching one of the five standard patterns, naming the pattern and printing no value or matched byte.

**Stories:**
- S16 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a ship to refuse a home holding a named secret value, so that no credential leaves in a shipped ref.

### R5: Add the arrival kind of lys.harness_event

In crates/lys-home/src/harness/claude_code/events.rs add the seventh kind KIND_ARRIVAL = `arrival` and `arrival(source_commit, remote, reference, execution_id) -> HarnessEvent` with source_uuid None, record None, and a detail holding exactly `source_commit`, `remote`, `ref` and `execution_id`. Its data is the shape every lys.harness_event already carries, `{kind, harness, source_uuid, record, detail}`, refused by the existing HomeError::EventTooLarge when it exceeds MAX_DATA_BYTES (512). The arrival is written by fetch only: THE SYSTEM SHALL NOT map any Claude Code record to the arrival kind in event_of, SHALL NOT change the six existing kinds, their constants or template_render's shape, and SHALL NOT put content, userinfo or a path other than the recorded remote into an arrival.

**Acceptance:**
- arrival(40 × `a`, `/r/remote.git`, `refs/lys-home/` + 40 × `a`, 32 × `b`).data() returns Ok of the JSON object {"kind":"arrival","harness":"claude-code","source_uuid":null,"record":null,"detail":{"source_commit":40 × `a`,"remote":"/r/remote.git","ref":"refs/lys-home/" + 40 × `a`,"execution_id":32 × `b`}}.
- The same call with a remote of 400 characters returns Err(HomeError::EventTooLarge).
- The events tests that passed before this brief pass unchanged.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/events.rs
- modify: crates/lys-home/src/harness/claude_code/events_tests.rs

**Checklist:**
- C29 — Fetch appends to each session, beside the head, one lys.harness_event of kind arrival naming the source commit, the remote without userinfo, the ref and the target home's execution id, within the 512-byte cap.

**Stories:**
- S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every fetched session to record the commit, remote and ref it came from and the new home's execution id, so that a moved home's ancestry is on the record.

### R6: Add `lys-home ship`: commit the tracked set and push it as one ref

Add the subcommand `ship --home <dir> --remote <remote> --secret-values <file>`; its arguments and dispatch live in crates/lys-home/src/cli/moves.rs and its logic in crates/lys-home/src/moves/ship.rs, and cli.rs holds only the variant, its dispatch and the module doc naming it. WHEN run, THE SYSTEM SHALL, in this order: name the remote with ship_remote (R3); check every sessions/*.jsonl whose name does not end `.index.jsonl` with verify_session (R1); take the tracked set (R4) and scan it for the values read from --secret-values and for STANDARD_PATTERNS (R4); and only then initialise the home as a git repository when it holds no .git, build a commit of exactly the tracked set through a temporary index file under .git (the previous ship commit, refs/heads/lys-home, as its parent when there is one), set refs/heads/lys-home to it, push `<commit>:refs/lys-home/<commit>` to the remote without force (R2), and print one JSON report {command: "ship", commit, ref}. IF any session's index is stale, THEN THE SYSTEM SHALL check every session before refusing, print one JSON report {command: "ship", refused: "stale_index", stale: [the home-relative path of each stale session file, sorted bytewise]}, exit 1 with HomeError::StaleIndex on stderr, and write nothing to the home and nothing to the remote. IF any other refusal fires, THEN THE SYSTEM SHALL exit 1 with the refusal on stderr, print no report, and write nothing to the home and nothing to the remote. THE SYSTEM SHALL NOT write any file of the home outside .git/, SHALL NOT check out, reset, clean or stash, SHALL NOT force a push or push any ref but the one, SHALL NOT open a session, take its lock, rebuild an index or persist a head, and SHALL NOT print a secret value, a matched byte or a transcript byte.

**Acceptance:**
- `lys-home ship --home <dir> --remote <remote>` exits 2 and its stderr contains `--secret-values`.
- In tests/home_ship.rs, on the fixture home after one render-launch of the fixture template, with a bare remote made by `git init --bare` in a temporary directory and a values file holding `fixture-secret-value-0001`, ship exits 0 and prints a report whose keys are exactly command, commit and ref, whose commit is 40 lowercase hex characters, and whose ref equals `refs/lys-home/` followed by the commit.
- `git --git-dir <remote> rev-parse <ref>` prints the reported commit, and `git --git-dir <remote> for-each-ref` prints exactly one line.
- `git --git-dir <remote> ls-tree -r --name-only <commit>` prints exactly the paths tracked_set returns for the home, in the same order.
- After the 8 bytes `{"x":1}` plus a newline are appended to sessions/fixture.jsonl of a second fixture home, ship exits 1, its stdout is one JSON object whose keys are exactly command, refused and stale, with command `ship`, refused `stale_index` and stale equal to ["sessions/fixture.jsonl"], its stderr contains `stale` and `fixture.jsonl`, that home holds no .git, and `git --git-dir <remote2> for-each-ref` on a fresh bare remote prints nothing.
- Ship with --remote https://example.invalid/home.git exits 1, its stderr contains `remote_off_machine` and `https`, and the home holds no .git.
- On a fixture home holding blocks/zz/plant with the bytes `0123fixture-secret-value-0001`, ship exits 1, its stderr contains `secret_in_home`, `blocks/zz/plant` and `offset 4` and does not contain `fixture-secret-value-0001`, and the fresh bare remote's for-each-ref prints nothing.
- On a fixture home holding blocks/zz/plant with the bytes `0123ghp_` followed by 36 × `A`, ship exits 1, its stderr contains `secret_in_home`, `blocks/zz/plant`, `offset 4` and `github_token` and does not contain `ghp_`, and the fresh bare remote's for-each-ref prints nothing.
- The ship tests assert that exactly four of their five ship runs over a valid argument set are refused.

**Files:**
- create: crates/lys-home/src/cli/moves.rs
- create: crates/lys-home/tests/home_ship.rs
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/src/moves/mod.rs
- modify: crates/lys-home/src/moves/ship.rs

**Checklist:**
- C24 — lys-home ship commits the home's tracked set (session files, their index and head files, blocks and templates) and pushes it as one ref, refs/lys-home/<commit>, to the remote named on the command line, and its report names the commit and the ref.
- C25 — Ship refuses by name a home whose index is stale or whose head file is missing, exits 1, and writes nothing to the home or the remote; no index is rebuilt and no head persisted.
- C26 — Ship refuses by name any remote that is not a path on this machine or a file:/// URL, naming its scheme and the encryption-at-rest precondition, and writes nothing.
- C27 — No lock, temporary, environment or render file is in a shipped tree, and ship refuses by file and offset a tracked file holding any value of the --secret-values file or matching one of the five standard patterns, naming the pattern and printing no value or matched byte.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home carried to a new home as a pushed ref that the new home fetches, so that I continue from my own record rather than from a hand copy.
- S16 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a ship to refuse a home holding a named secret value, so that no credential leaves in a shipped ref.

### R7: Add `lys-home fetch`: fetch the ref into a new home, verify it, and record the arrival

Add the subcommand `fetch --remote <remote> --ref <ref> --home <dir>`; its arguments and dispatch live in crates/lys-home/src/cli/moves.rs and its logic in crates/lys-home/src/moves/fetch.rs. WHEN run, THE SYSTEM SHALL, in this order: refuse with a new HomeError::TargetHoldsHome when <dir> exists and holds a home, that is an entry named `sessions`, `blocks` or `templates` that is a directory, an entry named `.git` of any kind, or an entry named `execution-id` of any kind, before running git, before Home::open and before writing anything; when one of the first four is present its message contains `target_holds_home`, <dir> and the name of the first such entry in the order sessions, blocks, templates, .git; when only `execution-id` is present its message contains `target_holds_home` and <dir> and does not name the file; an empty <dir>, or one holding only other entries, is not a home and is accepted; name the remote with fetch_remote (R3); make a staging directory beside <dir> whose name contains `.lys-fetch-`; initialise it as a git repository, fetch <ref> from the remote into the same ref name, and check out its commit detached (R2); check every sessions/*.jsonl of the checkout whose name does not end `.index.jsonl` with verify_session (R1); build every session's arrival event (R5) and refuse on one that exceeds the cap before appending any; take a fresh execution id with record::fresh_id; append to each session, in session id order, one lys.harness_event custom entry holding the arrival's data with append_beside so the head does not move; then place the home in <dir>: create <dir> when it does not exist, write the execution id with a newline to <dir>/execution-id as a new file that never replaces one (when that file already exists, the write refuses with HomeError::TargetHoldsHome naming <dir>, not with HomeError::Io; any other failure of that write refuses with HomeError::Io naming it; either refusal comes before anything else is written under <dir>), rename the staged .git, sessions, blocks and templates into <dir>, and remove the staging directory; and print one JSON report {command: "fetch", home, commit, ref, execution_id, arrivals: [{session, event}]}. IF any refusal fires, THEN THE SYSTEM SHALL remove the staging directory, leave <dir> exactly as it was, or absent when it was absent, print no report and exit 1 with the refusal on stderr. THE SYSTEM SHALL NOT write into <dir> before the verification and the arrivals have passed in staging, SHALL NOT touch, move or remove an entry <dir> already held, SHALL NOT call a directory that holds no home marker a home, SHALL NOT refuse a home marker through HomeError::Io, SHALL NOT rebuild an index or persist a head before the verification passes, SHALL NOT append any entry but the one arrival per session, SHALL NOT move a head, and SHALL NOT write anything to the remote.

**Acceptance:**
- `lys-home fetch --remote <remote> --home <dir>` exits 2 and its stderr contains `--ref`.
- In tests/home_fetch.rs, from a bare remote holding the ref a ship of the fixture home (after one render-launch) pushed, fetch into a path that does not exist exits 0 and prints a report whose commit and ref equal the ship report's and whose execution_id is 32 lowercase hex characters, and <dir>/execution-id holds that execution_id followed by a newline.
- The last line of <dir>/sessions/fixture.jsonl is a custom entry with customType `lys.harness_event`, parentId `e4`, data.kind `arrival`, data.detail.source_commit equal to the commit, data.detail.ref equal to the ref, data.detail.remote equal to std::fs::canonicalize of the remote path, and data.detail.execution_id equal to the report's execution_id, and its serialised data is at most 512 bytes.
- <dir>/sessions/fixture.head holds `e4` followed by a newline after the fetch.
- Two fetches of the same ref into two new paths report two different execution_id values.
- A second fetch into the same <dir> exits 1, its stderr contains `target_holds_home` and `sessions`, the SHA-256 of every file under <dir> is the same before and after, and <dir>'s parent holds no entry whose name contains `.lys-fetch-`.
- A fetch into an existing directory holding one empty file named `note` exits 0, `note` is still there and still empty afterwards, and the directory then holds .git, sessions/fixture.jsonl and execution-id beside it.
- A fetch into an existing directory holding only an empty directory named `templates` exits 1, its stderr contains `target_holds_home` and `templates`, and the directory holds only that empty `templates` afterwards.
- A fetch into an existing directory holding only one empty file named `execution-id` exits 1, its stderr contains `target_holds_home` and the directory's path and does not contain the string `execution-id`, and the directory holds only that empty `execution-id` file afterwards.
- From a ref whose commit the test builds with plain git from a copy of the fixture home whose sessions/fixture.jsonl had the 8 bytes `{"x":1}` plus a newline appended after its index was written, fetch exits 1, its stderr contains `stale` and `fixture.jsonl`, <dir> does not exist afterwards, and its parent holds no entry whose name contains `.lys-fetch-`.
- From a ref whose commit the test builds with plain git from a copy of the fixture home whose sessions/fixture.head holds `e9` plus a newline, fetch exits 1, its stderr contains `head_not_indexed` and `e9`, and <dir> does not exist afterwards.
- Fetch with --remote https://user:tok3n@example.invalid/home.git exits 1, its stderr contains `remote_userinfo` and does not contain `tok3n`, and <dir> does not exist afterwards.
- The fetch tests assert that exactly six of their nine fetch runs over a valid argument set are refused.

**Files:**
- create: crates/lys-home/src/moves/fetch.rs
- create: crates/lys-home/tests/home_fetch.rs
- modify: crates/lys-home/src/cli/moves.rs
- modify: crates/lys-home/src/moves/mod.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C28 — lys-home fetch fetches the named ref into a new directory, refuses by name a directory that already holds a home (a sessions, blocks or templates directory, a .git entry, or an execution-id file) and accepts one holding only other entries, and checks every index against its file and every head against its index without rebuilding either, leaving nothing behind on a refusal.
- C29 — Fetch appends to each session, beside the head, one lys.harness_event of kind arrival naming the source commit, the remote without userinfo, the ref and the target home's execution id, within the 512-byte cap.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home carried to a new home as a pushed ref that the new home fetches, so that I continue from my own record rather than from a hand copy.
- S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every fetched session to record the commit, remote and ref it came from and the new home's execution id, so that a moved home's ancestry is on the record.

### R8: Prove the move end to end on the fixture home

Add crates/lys-home/tests/home_move.rs, which builds the source home from the fixture session and one render-launch of the fixture template with LYS_FIXTURE_TOKEN=fixture-secret-value-0001 in the command's environment, records the SHA-256 of every file under the source home, ships it with a values file holding `fixture-secret-value-0001` to a bare remote in a temporary directory, and fetches the pushed ref into a second temporary directory, all through the built binary. The match is taken where the arrival cannot touch it: THE SYSTEM SHALL show every tracked file at the fetched commit equal to the source file byte for byte, and after the arrival is appended SHALL show the source session file and index as byte prefixes of the target's and the blocks and templates unchanged. The search of the shipped ref SHALL read every object in the bare remote and count what it searched and what it found, with a positive control that the same search finds a planted value. The source home in this test SHALL also hold blocks/zz/crlf with the 6 bytes `a`, CR, LF, `b`, CR, LF. The ship in this test SHALL run with HOME and XDG_CONFIG_HOME set to a directory whose .gitconfig turns on commit.gpgSign, sets core.hooksPath to a directory holding an executable pre-push hook that exits 1, sets core.autocrlf true, and sets user.name and user.email so a plain commit fails on signing alone, and the test SHALL show each of the three settings firing on plain git under that same HOME, so the ship passing is the isolation and not a setting that never fired. THE SYSTEM SHALL NOT name a test after transcript content, SHALL NOT print a secret value, and SHALL NOT write any file outside the test's temporary directories.

**Acceptance:**
- For every path tracked_set returns for the source home, the SHA-256 of `git -C <target> show <commit>:<path>` equals the SHA-256 of the source file, and the test asserts the number of paths compared equals the tracked set's length, with at least one path under blocks/ and exactly one under templates/.
- After the fetch, the source sessions/fixture.jsonl bytes are a strict prefix of the target's and the target has exactly one line more; the source sessions/fixture.index.jsonl bytes are a strict prefix of the target's and the target has exactly one row more.
- The sorted lists of files under blocks/ and templates/ are equal between source and target, and each pair of files has equal SHA-256.
- Every file present in the source home before ship has the same SHA-256 after ship and after fetch, and every path added to the source home by ship begins with `.git/`.
- The target session's last line is an arrival event whose detail.source_commit equals the ship report's commit and whose detail.execution_id equals the contents of <target>/execution-id without its newline.
- A byte search for `fixture-secret-value-0001` over the contents of every object `git --git-dir <remote> cat-file --batch-all-objects --batch` prints finds 0 matches, and the test asserts the number of objects searched is at least the tracked set's length.
- The same search over a scratch bare repository into which the test wrote one blob holding `fixture-secret-value-0001` with `git hash-object -w` finds exactly 1 match.
- The ship run under the hostile .gitconfig exits 0, `git --git-dir <remote> rev-parse <ref>` prints the reported commit (the pre-push hook did not run), `git --git-dir <remote> cat-file -p <commit>` prints no line beginning `gpgsig`, and `git --git-dir <remote> cat-file blob <commit>:blocks/zz/crlf` prints exactly the 6 bytes `a`, CR, LF, `b`, CR, LF.
- Under the same HOME and XDG_CONFIG_HOME, with no GIT_CONFIG_GLOBAL set, plain `git push` of a commit from a scratch repository to a fresh bare repository exits nonzero and that bare repository's for-each-ref prints nothing; plain `git commit` in a scratch repository exits nonzero (the signing key does not exist); and plain `git add` of a file holding `a`, CR, LF, `b`, CR, LF followed by `git cat-file blob :<file>` prints the 4 bytes `a`, LF, `b`, LF.
- The hostile-configuration tests assert that exactly three plain-git controls fired and that the one isolated ship passed.

**Files:**
- create: crates/lys-home/tests/home_move.rs

**Checklist:**
- C24 — lys-home ship commits the home's tracked set (session files, their index and head files, blocks and templates) and pushes it as one ref, refs/lys-home/<commit>, to the remote named on the command line, and its report names the commit and the ref.
- C27 — No lock, temporary, environment or render file is in a shipped tree, and ship refuses by file and offset a tracked file holding any value of the --secret-values file or matching one of the five standard patterns, naming the pattern and printing no value or matched byte.
- C30 — On the fixture home, every tracked file at the fetched commit equals its source byte for byte, the source session file and index are byte prefixes of the target's after the arrival, the source home's files are unchanged by ship and fetch, and a search of every object in the remote finds no fixture secret value while a planted one is found.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home carried to a new home as a pushed ref that the new home fetches, so that I continue from my own record rather than from a hand copy.
- S16 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a ship to refuse a home holding a named secret value, so that no credential leaves in a shipped ref.

### R9: Render and resume the fetched home on the installed Claude Code and write it down

WHEN the move is proved, THE SYSTEM's proof SHALL build the source home from the fixture session and one render-launch of the fixture template, ship it to a bare remote in a scratch directory, fetch the ref into a second scratch directory, run render-launch on the target home with the fixture template, and run the printed launch line once from a working directory that is neither the out directory nor the session's cwd, on Claude Code 2.1.283 as `claude --version` reports it. docs/design/home/PROOF-MOVE.md SHALL record: the version string; the ship report's commit and ref; the fetch report's execution_id; the SHA-256 of the source and target session file, index and head at the fetched commit; the counts of block and template files in source and target; the session_head of the source render's template_render event and of the target render's; the five written paths relative to <out> with their SHA-256; the launch line with the out directory written as <out>; the rendered file's SHA-256 before and after the launch; where the continuation was written relative to the Claude Code projects directory; whether the continuation's parent link names the rendered session id, answered yes or no with the observation it rests on; and the resume-check report. The proof SHALL NOT contain transcript content, a secret value, or a path that is not written relative to <scratch>, <out> or the projects directory.

**Acceptance:**
- docs/design/home/PROOF-MOVE.md contains the string `2.1.283 (Claude Code)` as the version `claude --version` printed on the proof run.
- PROOF-MOVE.md records the ship commit and the ref, and the ref equals `refs/lys-home/` followed by the commit.
- PROOF-MOVE.md records, for the session file, the index and the head, the source SHA-256 and the target SHA-256 at the fetched commit, and each pair is equal.
- PROOF-MOVE.md records the session_head of the source render and of the target render, and the two are equal.
- PROOF-MOVE.md records the rendered file's SHA-256 before and after the launch, and the two are equal.
- PROOF-MOVE.md records the answer yes, with its observation, to whether the continuation's parent link names the rendered session id.
- PROOF-MOVE.md records the resume-check report with repeated_tool_use_ids equal to 0.
- A search of PROOF-MOVE.md for `fixture turn one` and for `fixture-secret-value-0001` finds 0 matches.

**Files:**
- create: docs/design/home/PROOF-MOVE.md

**Checklist:**
- C31 — PROOF-MOVE.md records the fetched home rendered by render-launch and resumed by the printed launch line on Claude Code 2.1.283, as hashes, counts and paths only.

**Stories:**
- S17 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the moved home's resume measured on a named Claude Code version and written in a proof document, so that the move is shown to continue a session rather than assumed to.

### R10: Write the move down and re-render the cluster

RECORD.md SHALL write down the home as a git repository, the tracked set by path pattern, that a session's lock file, dot-prefixed files and the execution-id file are never tracked, the ref namespace refs/lys-home/<commit> and the branch refs/heads/lys-home, the arrival kind's data shape with its four detail keys, and that ship and fetch check an index and a head without rebuilding either. crates/lys-home/README.md SHALL document both subcommands with their arguments, their reports and their refusals by name. The cluster's markdown (DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/HOME-004.md) SHALL be re-rendered with scripts/design/render-cluster.py, never hand-edited. THE SYSTEM SHALL NOT change the written description of any existing entry kind, and SHALL NOT change the rendered markdown of HOME-001, HOME-002 or HOME-003.

**Acceptance:**
- docs/design/home/RECORD.md lists `arrival` among the lys.harness_event kinds with the detail keys source_commit, remote, ref and execution_id.
- RECORD.md names sessions/<id>.jsonl, sessions/<id>.index.jsonl, sessions/<id>.head, blocks/<hh>/<hash> and templates/<hh>/<hash> as tracked, and names the session lock file and `execution-id` as never tracked.
- crates/lys-home/README.md contains `ship --home <dir> --remote <remote> --secret-values <file>` and `fetch --remote <remote> --ref <ref> --home <dir>`.
- `python3 scripts/design/render-cluster.py docs/design/home` followed by `git diff --exit-code docs/design/home` exits 0.
- `sh scripts/design/gate.sh` exits 0.

**Files:**
- create: docs/design/home/briefs/HOME-004.md
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/README.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md

**Checklist:**
- C24 — lys-home ship commits the home's tracked set (session files, their index and head files, blocks and templates) and pushes it as one ref, refs/lys-home/<commit>, to the remote named on the command line, and its report names the commit and the ref.
- C29 — Fetch appends to each session, beside the head, one lys.harness_event of kind arrival naming the source commit, the remote without userinfo, the ref and the target home's execution id, within the 512-byte cap.

**Stories:**
- S15 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every fetched session to record the commit, remote and ref it came from and the new home's execution id, so that a moved home's ancestry is on the record.

## Boundaries

- No second machine, no encryption, no harness other than Claude Code, and no transport other than a git ref pushed to and fetched from a named remote.
- Ship accepts no remote off this machine: only a path on this machine or a file:/// URL.
- No source session file, index, head, block or template is written, truncated, rebuilt or re-serialised by ship or fetch; ship writes into the home only under .git/.
- No secret value, remote userinfo, transcript content, block content or git output in a report, an error, an entry, a test name or the proof document.
- The scan is for the values of the --secret-values file and the five standard patterns of STANDARD_PATTERNS, and nothing else.
- No field added to Pi's grammar; the arrival rides inside a lys.harness_event custom entry.
- render-launch, the template_render event and lys.given keep their behaviour and wire shape.
- No force push, no deletion of a remote ref, and no ref pushed outside refs/lys-home/.
- No dependency added but the pure-Rust regex crate, no unsafe, no file over 500 lines of code, and no unwrap, expect or panic in library code.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- python3 scripts/design/validate.py docs/design/home exits 0.
- python3 scripts/design/check-coverage.py docs/design/home exits 0.
- cargo fmt --all -- --check, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps exit 0.
- sh scripts/design/gate.sh exits 0.
- grep -n 'fn ' crates/lys-home/src/moves/mod.rs returns nothing.
- git diff of crates/lys-home/Cargo.toml against the brief's base commit adds exactly the one line `regex.workspace = true` and removes none.
- The target session file from the R8 test, written out in the proof step, parses with Pi's parseSessionEntries at 3d5cbe98 through node, and its entry count equals the source's plus one (the command is written in PROOF-MOVE.md).
- git ls-files lists no .jsonl file outside crates/lys-home/tests/fixtures and canon/.

