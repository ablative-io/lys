# CONTEXT-001 — Capture a running session's transcript into a home, byte for byte, with a durable status view

Rendered from `CONTEXT-001.json`; edit the JSON source, then regenerate this file.

Revision 1. Status: **draft_for_waffles_review**. Owner: Archie. Reviewer: Waffles. Date: 2026-09-22 17:10 Australia/Melbourne.

Source: [../CONTEXT-ROADMAP-2026-09-22.md](../CONTEXT-ROADMAP-2026-09-22.md).

A home directory holds a byte-for-byte, append-only copy of one real running session's transcript files, taken from a declared transcript root, filed under a stable capture ID; a status view shows what has been captured and the offset that is durably stored, and never shows transcript contents. The live session keeps running; nothing signs anything; no identity service is involved.

## Authority

- Tom 15:01: seven-step road; Archie steps 4-5 (memory, context, home); each step a brief to Waffles before work starts.
- Tom 16:27: capture the logs, that is what recreates a session; do not learn harness config layouts; identity and permissions first, signing when asked; do not over-complicate.
- Tom 16:29: more than one way to capture (a file watcher on the transcript tree is one); profiles specialise, the core stays flexible; write a roadmap.
- CONTEXT-ROADMAP-2026-09-22.md at lys a6c4fa5, reviewed by Chippy (99717d39, bc041636) and Waffles (874979c9, 364bd446, f0bc6df7), verified by Waffles 16:37 (81be9fb5): stage 1 as written there is this brief.
- Tom 17:00 in the room: concrete steps that put something on screen tonight; builders on Opus from the other account; Chippy advisory; Waffles and Archie reading and review.
- Chippy 17:06 in the room: the captured transcript stays private, Cambium gets status and an evidence reference, never transcript contents; the offset on screen means bytes durably stored, so a restart can never make the display claim more than is held.
- Waffles 17:06 in the room: stage 1 brief written now, next into the one build slot on this Mac after IDENTITY-001 row 02; its screen is the captured home beside the live session.

## Ceiling

- Estimate: 6 focused implementer hours (row 01: 4, row 02: 2); ceiling 8, contingency 2. Two hours is the measured stop point of the first block, not a claim the brief fits inside it; report overrun as soon as known and Waffles takes any ceiling change to Tom.
- One builder, one row and one gate at a time, in the one build slot on this Mac, editing the lys checkout on main by exact pathspec. No sub-agents. Builders never stage or commit another person's file and never rewind HEAD.
- The captured session is one of Archie's own on this Mac. Captured bytes never leave the machine and never appear in a post, a log line, an error, a test name or a rendered page.

## Shared contract

- A capture ID is generated once per home and is not the seat name, a session ID, a public key or an OAuth client ID. The seat name is stored as a label. Binding a capture ID to an enduring directory ID is a later explicit act (IDENTITY-001 row 04); nothing in this brief claims the platform identity exists.
- A profile is a declaration supplied to the tool, never discovered: a name, an absolute transcript root, and a capture method. This brief implements one method, a filesystem watcher, and one profile file for Claude Code whose root the operator writes in. The tool holds no Claude knowledge.
- Copy discipline: by offset, append-only; the copy is never rewritten. The recorded offset is a verified prefix through one source file generation and advances only after those bytes are fsynced in the home. A source that is replaced, truncated or rewritten starts a separately identified capture generation or is refused by name; it is never appended onto the old one.
- Privacy: captured bytes are data the tool moves and never interprets, prints, logs or renders. Status carries paths relative to the home, byte counts, offsets, generation IDs and timestamps only. Cambium receives a status and an evidence reference (home path and manifest hash), not transcript contents.
- Nothing here changes lys-core, lys-log-store, lys-anchor or their published formats. No signing, no hashing into a log, no encryption at rest, no sync; those are later stages and are not started here.
- This brief's home layout and manifest are local state, not a wire contract, and say so in their docs, as lys-log-store's file layout does.

## Rows

### Row 01 — lys-home: capture by offset from a declared transcript root

Depends on: nothing. Estimated hours: 4. Provenance: Roadmap stage 1: capture the logs; filing rule; profile declaration; copy discipline.

Wall:
- lys: `Cargo.toml`
- lys: `Cargo.lock`
- lys: `crates/lys-home/Cargo.toml`
- lys: `crates/lys-home/src/`
- lys: `crates/lys-home/README.md`
- lys: `docs/design/identity/HOME-LAYOUT.md`
- lys: `profiles/claude-code.example.toml`

Work:
- Add workspace crate lys-home (library plus a lys-home binary) under the workspace lints (pedantic set, unsafe forbidden, no unwrap/expect/panic in library code, tests in sibling *_tests.rs files). Name every module in the manifest before writing: home (create/open, capture ID, label), profile (TOML declaration: name, transcript_root, method), source (source identity and generation: device, inode, size, mtime at open; generation changes on replace/truncate), capture (offset copy, fsync, manifest advance), manifest (per-file record: relative path, generation ID, durable offset, last seen source length, timestamps), watch (filesystem watcher over the declared root, fallback poll), cli (init, watch, status), error.
- lys-home init --home <dir> --label <seat name> creates the home with a fresh capture ID and writes capture.json; refuses an existing home. lys-home watch --home <dir> --profile <file> copies every file under the declared transcript root as it grows: read from the recorded durable offset, append to the home copy, fsync the copy and its directory, then advance the manifest; restart resumes from the manifest and never duplicates bytes.
- Source generation: on open and on every change, compare device, inode and length with the manifest. Length shrank, inode changed or bytes at the recorded offset differ from the copy: start a new generation ID for that path (the old copy is kept, closed and marked) or refuse by name when the operator passed --strict. Never append new-generation bytes onto an old-generation copy.
- HOME-LAYOUT.md documents captures/<capture-id>/<generation-id>/<relative path>, capture.json and manifest.json, and states that the layout is local state and not a wire contract. profiles/claude-code.example.toml carries a placeholder root; the operator writes the real one. The README states the privacy rule and what the tool does not do (no signing, no hashing, no encryption, no sync, no parsing).

Acceptance:
- CTX001_PREFIX: a source file appended in steps while watch runs; after each step, the home copy is a byte-identical prefix of the source up to the manifest's durable offset, checked with cmp on the recorded length, at every step and not only at the end.
- CTX001_RESTART: kill watch between a copy and its manifest advance (fault injection point named in code); on restart the copy is re-verified against the source from the recorded offset and the missing bytes are appended once; no byte duplicated, no byte lost; count the exercised crash points.
- CTX001_GENERATION: truncate a source, then replace it by inode; each yields a new generation directory with the old copy intact and closed; with --strict each is refused by name and nothing is written.
- CTX001_PRIVACY: a test transcript containing a marker string; the marker never appears in stdout, stderr, logs, error text, manifest, Debug output or test names; the copy contains it exactly once per generation.
- CTX001_PROFILE: the tool refuses a missing or relative transcript root and an unknown method by name; nothing in crates/lys-home mentions .claude, projects or jsonl outside the example profile.

Acceptance greps:
- `rg -n 'CTX001_PREFIX|CTX001_RESTART|CTX001_GENERATION|CTX001_PRIVACY|CTX001_PROFILE' crates/lys-home`
- `rg -n '\.claude|jsonl' crates/lys-home/src && false || true`

### Row 02 — The screen: a status view of the captured home

Depends on: 01. Estimated hours: 2. Provenance: Roadmap stage 1 proof and stage 7 note: a small capture-status view arrives with the stage that makes it useful; Chippy 17:06 on what the offset must mean.

Wall:
- lys: `crates/lys-home/src/`
- lys: `crates/lys-home/README.md`

Work:
- lys-home status --home <dir> prints one line per captured file: relative path, generation ID, durable offset, last seen source length, last change time, plus the capture ID and label at the top. Values come only from manifest.json; the display can never claim more than the manifest records, and the manifest advances only after fsync.
- lys-home status --render <file.html> writes a single static page with the same table, this product's accent from the estate colour guide (identity #D4975A, deep #A86B2E, wash #3D2A17) on the shared foundation, no scripts, no external assets, nothing but manifest values. It is opened in a browser beside the live session; that is the screen for tonight.
- Evidence a builder returns to the Cambium task: the status text, the rendered page's path, and SHA-256 of manifest.json; never a transcript path outside the home and never transcript bytes.

Acceptance:
- CTX001_STATUS: status output equals the manifest for a home with two generations; after a crash injected before the manifest advance, status shows the pre-crash offset, not the copied length.
- CTX001_RENDER: the rendered page contains every manifest value and no byte from any captured file; it loads with no network access.

Acceptance greps:
- `rg -n 'CTX001_STATUS|CTX001_RENDER' crates/lys-home`

## Required venue checks

- Lys: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps.
- The live proof: watch running against one of Archie's real Claude Code sessions on this Mac for at least one full turn, then status and the rendered page shown to Tom; the captured home is not posted, only its status and manifest hash.
- Dev install shown first, gated after, per Tom's rule for this box; the Argus gate timeout is Heimdall's and no gate is submitted until it is resolved.

## Current evidence

- Nothing built. No crate, no file, no command has been created for this brief. The roadmap it derives from is on lys main at a6c4fa5.
- Prior measured facts relied on: Claude Code session files are append-only JSONL trees with uuid/parentUuid (Lantern 02-SESSION-FORMATS); compaction writes a boundary record and keeps everything before it. The tool depends on neither fact; they only say why byte capture is enough for stage 1.

## Source evidence

- lys-log-store/src/file.rs: durability by fsync of file and parent directory, F_FULLFSYNC on Apple; the copy discipline here follows it.
- lys-log-store/src/store.rs: reported gaps are attributable, silent gaps are not; the refuse-by-name rule here follows it.

