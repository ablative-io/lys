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

Adopt Pi's session tree as the home record (Tom, Dot 13:27 and 13:28: Pi's tree, not Norn): one append-only JSONL per session, a header line, then entries each carrying id, parentId and timestamp, a leaf pointer for the current position, forks by moving the pointer, compaction and branch summaries as entries that keep their originals. lys adds nothing to that grammar: harness events (approvals, tool completion) and proxy call records ride Pi's custom entry type under lys customType names, so a home file stays readable by Pi's own parser. Two captures feed it: the model traffic through the door's proxy (the same process that swaps the credential, SECRETS-002 R1), and the harness's own events from its transcript. Content blocks are stored once by hash; entries reference them. A harness resume file is a rendered projection of the root-to-leaf path: for Claude Code, a JSONL written under a chosen uuid at the harness's own path, then resumed by that id with --fork-session. Provider-native reasoning stays on the message with its provider, api and model and is rendered whole only to the same three; another model gets readable thinking as text and opaque blocks dropped, each named in a loss account. lys grants say who may read and resume. Every resume path is measured on a named harness version before anything relies on it.

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
- ADR-012 — A home moves as a pushed git ref that the target fetches, and the target records its own execution id and ancestry — A home moves by the same route as a build tree: its tracked files are committed in the home's own git repository and pushed as one ref to a remote named on the command line, and the target fetches that ref into a new home. This over a hand copy (cp, rsync or an archive carried between directories), which leaves no commit to name, no tree hash to check the arrival against and no ancestry. The target mints its own execution id and records its arrival (source commit, remote, ref and execution id) in each session; source artifacts are never rewritten, and credentials are supplied at launch on the target, never tracked in the home.

## Goals

- A few-shot session file written by hand resumes Claude Code by path, from a directory outside the config root, with the file preserved and the demonstration marked authored.
- One real Claude Code session imported into the common record, rendered back, and resumed under a new id on 2.1.281 without repeating a completed tool action, with the original file's hash unchanged.
- One seat with a subscription login making calls through a pass-through proxy, so the tee has somewhere to live.
- A written loss account for every derived record: what a render or translation preserved, transformed and could not carry.
- The canon, one curated versioned series of short examples each showing a rule lived, seeds every new session, and its effect is measured on a card against a plain start.
- A handover letter from an outgoing session seeds its successor as inherited memory, with the model's own thinking intact, and the effect is measured on a card against a plain start.
- A home shipped as one git ref to a named remote, only when every index is fresh, every file is a home file and no named credential value or standard credential pattern is in it, and fetched into an isolated second home on the same machine; the fetched tree equals the source's, each session then carries one appended arrival event with the source commit and a fresh execution id, and the home is rendered through the Claude Code launch template and resumed on the installed Claude Code with its version and the measurement written down.

## Non-Goals

- Anchoring, signing or receipts into a lys log (CONTEXT-ROADMAP stage 6; when asked for). — Signing comes when asked for (Tom, 22 September 16:27); every stage here works without it.
- Encryption at rest, and moving a home between devices beyond the same-machine proof of HOME-004 (stage 3 second block and later). — The second block waits on its two preconditions, read authority over the home from the directory step and encryption before bytes leave the machine from the secrets step, each a card on its own board; HOME-004 takes only the same-machine block, and its shipped ref is what the second block fetches.
- Harnesses other than Claude Code, and Chat Completions or Responses translation beyond keeping the raw call bytes. — One harness proved first; each other harness is its own profile and its own measurement.
- Lanterns and forks at a coordinate (stages 5 and 5b). — Lanterns and forks stand on a proved resume; this brief supplies that proof.
- Adopting, wrapping or calling Norn's session code; Pi's code is read as the reference and not vendored. — Tom, Dot 13:28: not Norn. Pi's tree is the reference.
- Any transport for a home other than a git ref pushed to a named remote. — Stage 3 moves a home by the same route as a build tree, a pushed ref the target fetches, never a hand copy (ADR-012).

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
| `docs/design/home/briefs/HOME-004.json` | the brief that ships a home as a git ref, fetch it into a second home with an arrival event, and the same-machine resume proof | HOME-004 |
| `docs/design/home/briefs/HOME-004.md` | its rendered markdown | HOME-004 |
| `docs/design/home/DESIGN.md` | the cluster design rendered from design.json | HOME-004 |
| `docs/design/home/CHECKLIST.md` | the checklist rendered from checklist.json | HOME-004 |
| `docs/design/home/USER-STORIES.md` | the stories rendered from stories.json | HOME-004 |
| `crates/lys-home/src/record/verify.rs` | read-only check of a session's index against its file and its head against its index; a missing head is legal; never rebuilds or writes | HOME-004 |
| `crates/lys-home/src/record/verify_tests.rs` | the read-only check's tests: stale index, missing index, missing head, head naming no entry, nothing written | HOME-004 |
| `crates/lys-home/src/record/mod.rs` | declares the verify module | HOME-004 |
| `crates/lys-home/src/error.rs` | named refusals for ship and fetch: stale index naming the index step, head mismatch, unshippable files, occupied target, arrival too large, a git step that failed; session held reuses the existing SessionHeld | HOME-004 |
| `crates/lys-home/src/ship/mod.rs` | module declarations only: git, scan, push, fetch, arrival, cli and their sibling test modules | HOME-004 |
| `crates/lys-home/src/ship/git.rs` | the git binary run with system and global config, hooks, signing, line-ending conversion and prompts shut off | HOME-004 |
| `crates/lys-home/src/ship/git_tests.rs` | the git runner's tests: the machine's git configuration ignored, a failed push refused by name without git's output | HOME-004 |
| `crates/lys-home/src/ship/scan.rs` | sorts a home's files into tracked, left behind and foreign, and scans the tracked ones for the named credential values and the five standard credential patterns, by file and offset | HOME-004 |
| `crates/lys-home/src/ship/scan_tests.rs` | the classification and scan tests: the three sets, one hit at its offset, a test per pattern, the excluded shapes not matched, no value in any report | HOME-004 |
| `crates/lys-home/src/ship/push.rs` | ship: lock every session, check every index, classify and scan, commit exactly the tracked set in the home's own repository, push one ref | HOME-004 |
| `crates/lys-home/src/ship/push_tests.rs` | ship's tests: exactly the tracked set on one ref, the source unchanged, the stale-index, foreign-file, credential-hit and held-session refusals with no .git and no ref after, a missing head shipped as it is, a second ship parented on the first | HOME-004 |
| `crates/lys-home/src/ship/fetch.rs` | fetch: refuse an occupied target, fetch one ref, check the tree and every index and head | HOME-004 |
| `crates/lys-home/src/ship/fetch_tests.rs` | fetch's tests: commit and tree equal to the ship's, the files hash-matching before the arrival, the occupied-target refusals, a head naming no entry refused after fetch, source and remote unchanged | HOME-004 |
| `crates/lys-home/src/ship/arrival.rs` | the arrival lys.harness_event: source commit, remote without userinfo, ref, execution id | HOME-004 |
| `crates/lys-home/src/ship/arrival_tests.rs` | the arrival tests: the byte prefix, the index row and head, one execution id per fetch, userinfo removed, the 512-byte cap | HOME-004 |
| `crates/lys-home/src/ship/cli.rs` | the ship, fetch and index subcommand arguments and their JSON reports | HOME-004 |
| `crates/lys-home/src/cli.rs` | the ship, fetch and index subcommands joined to the command enum, delegating to ship/cli.rs | HOME-004 |
| `crates/lys-home/src/main.rs` | a refused ship or fetch prints its JSON report and exits 1 | HOME-004 |
| `crates/lys-home/src/lib.rs` | declares the ship module | HOME-004 |
| `crates/lys-home/tests/ship_fetch.rs` | ship to a bare remote in a temporary directory and fetch into a second home: tree equality, the byte prefix after the arrival, source byte-identity, the refusals, the index step, the fixture secret | HOME-004 |
| `docs/design/home/RECORD.md` | the git-backed home: its tracked set, what is left behind and refused, the ref, the index step and the arrival kind of lys.harness_event | HOME-004 |
| `crates/lys-home/README.md` | what ship, fetch and index do and do not do | HOME-004 |
| `docs/design/home/PROOF-SHIP.md` | the measured same-machine move: ship, fetch, render through the launch template and resume on the installed Claude Code with its version recorded, hashes, lengths, counts and paths only | HOME-004 |

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
