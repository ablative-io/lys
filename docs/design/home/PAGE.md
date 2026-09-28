# home — what was asked, what it means, and what was written

## The words, as they were typed

lys-home records proxy calls as lys.call entries (ingest_call and ingest_outcome in crates/lys-home/src/record/call.rs) and harness events as lys.harness_event entries in the session JSONL. Its Cargo.toml depends on neither lys-core nor lys-log-store, so no entry is signed and none reaches a lys log. The repository has no server or runtime process: crates/lys is the verifier command line, and `lys log append` appends a leaf file without signing it. Opening a sealed record (`lys open`, seal::open_and_verify) leaves no record at all. The outcome: each lys.call entry, each lys.harness_event entry and each sealed-record open appends one leaf to a lys log under the identity. The leaf carries the session id, the entry id, the kind and the SHA-256 of the entry line, never content, and it is signed with the identity's Ed25519 key through sign_attestation, in the envelope the identity audit events use. A failed append is refused by name with the entry id, and the entry is never reported as recorded. Tests: two calls and one event give three leaves that verify under the identity's public key; one byte changed in an entry line makes verification refuse and name that entry; ingesting the same call_id again adds no leaf; a search of the log for a fixture message's text finds nothing. It builds after the identity event envelope (DIRECTORY-003 R2) and after the log grows past memory (F-BV2vqP) have landed. Not in scope: the door's proxy and the runtime, which live outside this repository, and anchoring, encryption at rest, and haematite as the store.

## What the survey found, and its angles

Today lys-home's lys.call and lys.harness_event entries sit only in the session JSONL. Nothing signs them and nothing puts them in a lys log, and opening a sealed envelope leaves no trace. The words ask that every such entry, and every sealed-record open, append one signed leaf to a lys log kept under the identity. The leaf holds only ids, the kind and the SHA-256 of the entry line, signed with the identity's Ed25519 key in the identity audit-event envelope. Tests must show the leaves verify, that tampering is caught and named, that re-ingest is idempotent, and that no content leaks. The work is sequenced after the DIRECTORY-003 R2 envelope and the F-BV2vqP log-scaling change.

### What the tree holds

- `crates/lys-home/Cargo.toml` — Its dependencies are clap, rand, serde, serde_json, sha1, sha2, thiserror and time. It has no lys-core and no lys-log-store, so both have to be added for signing and appending.
- `crates/lys-home/src/record/call.rs` — ingest_call (l.~347), ingest_call_files, ingest_outcome (l.~442), find_call (l.~332) and the already_recorded short-circuit (fn already, l.~493). The idempotency test and the refusal of a failed append land here. Its only append is session.append in finish_ingest_counted (l.~563).
- `crates/lys-home/src/record/mod.rs` — Session::append / append_entry / append_line (l.374-440) is the one durable write path. It writes the line with to_line, then the index row, then the head. The entry-line bytes whose SHA-256 the leaf carries exist here first, so this is the natural place to append a leaf and to order it against the line.
- `crates/lys-home/src/harness/claude_code/import.rs` — The Claude Code importer turns each harness-local record into a lys.harness_event (l.350). A whole transcript import therefore appends many events in one run, and each would need a leaf.
- `crates/lys-home/src/harness/claude_code/launch.rs` — Records the sixth harness_event kind, template_render, as a side leaf beside the context path (l.156, ADR-012). It is a lys.harness_event, so it falls under 'each lys.harness_event entry'.
- `crates/lys-home/src/harness/claude_code/events.rs` — Holds the six event kinds (hook, permission_mode, tool_completed, attachment, system, template_render) and the data shape {kind, harness, source_uuid, record, detail}. The record rides by block hash.
- `crates/lys-home/src/cli.rs` — The IngestCall subcommand (l.134-168) and the dispatch (l.297). A failed append has to surface here as a named refusal carrying the entry id, and any verify subcommand would be added here.
- `crates/lys/src/commands/seal.rs` — `lys open` wraps lys_core::seal::open_and_verify (l.188). It has no home, session, entry or log argument, and it deliberately collapses every failure to one non-oracle CliError::OpenFailed. It is the only place a 'sealed-record open' happens in the tree.
- `crates/lys/src/commands/log/append.rs` — `lys log append` appends raw leaf bytes without a key, as the words say. It shows how the CLI opens a log store (commands/log/store.rs).
- `crates/lys-core/src/attestation/sign.rs` — sign_attestation(payload, &Ed25519Identity) (l.76) produces a lys/attestation/v2 COSE_Sign1 over SHA-256(payload) plus a wall-clock unix-ms timestamp. verify_attestation / verify_attestation_bytes_by_signer is the verification path. The protected content type is the generic v2 attestation, so what kind of thing was signed must be carried inside the payload.
- `crates/lys-log-store/src/log.rs` — Log<S> holds every leaf in memory (`leaves: Vec<Vec<u8>>`, l.53-60), and its own doc says it is 'not suitable for an unbounded log'. That is the limit F-BV2vqP is named to lift.
- `crates/lys-log-store/src/file.rs` — The file-backed LeafStore that holds no leaf bytes in memory. It is where leaves physically land, and what the 'search the log for a fixture message's text' test would scan.
- `docs/design/home/DESIGN.md` — Non-goal l.66, 'Anchoring, signing or receipts into a lys log (CONTEXT-ROADMAP stage 6; when asked for)', is exactly what this asks for in part (signing plus a log, anchoring still out). The cluster design (design.json, rendered) must move it.
- `docs/design/home/RECORD.md` — States 'nothing is signed' (l.6) and names stage 6 as where signing arrives (l.119-120). It must record the leaf format once it exists.
- `docs/design/directory/briefs/DIRECTORY-003.json` — R2 defines the versioned identity event envelope: outside lys-core, typed payloads, actor, operation id and log coordinate in the receipt, reviewed with Archie before it signs durable bytes. It is to be written in docs/design/identity/IDENTITY-EVENTS.md, which does not exist.

### What was already decided

- home P1 — The original bytes are never rewritten. A failed log append cannot be undone by removing the session line.
- home P7 / CN3 — Transcript content never appears in output, logs or errors; hashes, counts and ids only. The leaf's 'never content' is this rule carried into the log.
- home CN4 — Every home file parses with Pi's parser. Any leaf pointer kept in the session must ride only inside custom entries.
- home non-goal 1 — 'Anchoring, signing or receipts into a lys log' was out of scope 'when asked for'. This asks for signing plus a log, with anchoring still excluded.
- ADR-012 — template_render is a lys.harness_event hung as a side leaf. It gets a log leaf too, without moving the session head.
- ADR-013 — lys.given is unsigned now and shaped as hashes so signing can be added later. The words do not list it, so it stays unsigned here.
- ADR-003 — Everything is pegged to a human authority, and an agent holds its own identity under a person. That bears on whose Ed25519 key 'the identity's key' is.
- DIRECTORY-003 R2 — The envelope is versioned, outside lys-core, with typed payloads, and reviewed jointly with Archie before any durable bytes are signed; the log is lys-log-store's file storage, not Haematite. It is not built: no crates/lys-identity, no IDENTITY-EVENTS.md.
- directory P4 — One signed committed event is both the change and its audit record, never a database change followed by a best-effort log append. That is the ordering this work's session line and leaf must answer to.
- directory P7 — A receipt carries version, operation id, actor, affected identity, operation, payload commitment and log coordinate. The home leaf's fields (session id, entry id, kind, line hash) must map onto that envelope.
- directory P8 — The service never claims a person signed bytes with a key they do not hold. Signing with 'the identity's key' must be a key that identity actually holds.
- lys CLAUDE.md: Wire formats are forever — A new leaf payload signed into a log is a frozen wire contract from its first durable signature, and it needs a versioned tag.
- lys CLAUDE.md: cryptographic changes — A cryptographic change needs an adversarial review (forgery, cross-protocol confusion, replay) before landing.
- lys CLAUDE.md: crates — crates/lys is a thin CLI over lys-core. lys 0.2.0 is published, and `lys open` recording to a log changes a published binary's behaviour.

### What was measured

- lys-home runtime dependencies on lys-core or lys-log-store: 0 of 9 dependencies
- lys.harness_event kinds: 6 (hook, permission_mode, tool_completed, attachment, system, template_render)
- harness events one real Claude Code import produces (PROOF-RESUME.md l.22): 124 of 205 entries, so 124 leaves per import of that session
- crates/lys-home/src/record/call.rs size: 589 lines, ~460 non-comment non-blank (limit 500 lines of code)
- crates/lys-home/src/record/mod.rs size: 630 lines, ~472 non-comment non-blank
- crates/lys-home/src/cli.rs size: 537 lines, ~433 non-comment non-blank
- crates/lys-home/src/harness/claude_code/import.rs size: 531 lines, ~473 non-comment non-blank
- crates/lys/src/commands/seal.rs size: 212 lines, ~123 non-comment non-blank
- #[test] functions in lys-home (src plus tests): 158
- crates in the workspace: 6 (lys, lys-anchor, lys-anchor-cli, lys-core, lys-home, lys-log-store); no lys-identity
- docs/design/identity/IDENTITY-EVENTS.md (DIRECTORY-003 R2's envelope): does not exist
- references to F-BV2vqP in this tree, cambium or ~/.aion outside clones: 0 found
- lys-log-store Log memory model: every leaf held in memory (Vec<Vec<u8>>, log.rs:60)
- roadmap rows: 11 (RM-001 to RM-010, RM-016); none carries signing home entries into a log
- workspace version: 0.2.0 (lys-core, lys, lys-log-store published at 0.2.0)

### What it means for the other projects

- cambium — The card and the F-BV2vqP prerequisite are tracked on Cambium's board, and this card must go through the brief_card → sign-off → card_build_v3 → src_pr → src_land chain. Nothing in Cambium's code changes.
- aion — The workflow chain runs this card, and gates go to Dean's laptop. The door's proxy that will call ingest_call lives outside this repo and is out of scope, but once the refusal lands it will receive a named refusal on a failed append.
- haematite — Explicitly not the store. The log stays lys-log-store's file storage, as DIRECTORY-003 R2 also requires.

### The decisions it stands on

- ADR-003 (honour) — The signing key must be the identity's own, pegged under a person, and the log is kept under that identity.
- ADR-012 (honour) — template_render stays a side leaf that does not move the session head. Its log leaf must not change the head hash either.
- ADR-013 (honour) — lys.given stays unsigned. The words do not include it, and its hash-only shape leaves signing for later.
- ADR-015 (honour) — Lantern notes remain outside the log and CN3's exception is untouched. Leaves carry hashes only.
- ADR-004 (honour) — lys-home must keep working as a standalone project: signing needs a key and a log path, not the door or manifold.
-  (new) — The home entry leaf is a new signed wire format: payload fields, versioned tag, envelope type within DIRECTORY-003 R2's envelope, and the ordering of line against leaf. It is frozen from its first signature and needs a recorded decision and an adversarial review.
-  (new) — The home design's non-goal 'Anchoring, signing or receipts into a lys log' must be narrowed to anchoring only. Signing into a lys log becomes in scope for lys.call, lys.harness_event and sealed opens.

### What it requires

- crates/lys-home/Cargo.toml depends on lys-core and lys-log-store.
- Every successful ingest_call, ingest_call_files and ingest_outcome that appends a lys.call entry appends exactly one leaf to the identity's lys log.
- Every lys.harness_event appended (import, template_render) appends exactly one leaf to the identity's lys log.
- Each sealed-record open appends exactly one leaf to the identity's lys log.
- Each leaf's payload holds the session id, entry id, kind and SHA-256 of the entry's exact line bytes, and no content.
- Each leaf is signed with the identity's Ed25519 key via lys_core::attestation::sign_attestation inside the DIRECTORY-003 R2 event envelope, and verifies with verify_attestation under the identity's public key.
- A failed log append returns a named error carrying the entry id, and the ingest report or CLI output never reports the entry as recorded.
- Test: two ingested calls and one harness event yield exactly three leaves, each verifying under the identity's public key, with the leaf count asserted.
- Test: changing one byte of one entry line in the session file makes verification fail with an error naming that entry's id, and only that entry.
- Test: ingesting the same call_id twice leaves the leaf count unchanged.
- Test: a byte search of every file under the log directory for a fixture message's text finds zero matches, over a log holding at least the expected leaves.
- The leaf format and its version are written in docs/design/home/RECORD.md, and the home design's non-goal on signing is revised.
- The adversarial review of the new signed format is recorded before it lands.
- All gate legs pass: fmt, clippy both feature shapes, tests --all-features, doc both shapes, design gate.

### What must not change

- No existing session file byte is rewritten (P1, CN1); the leaf is added beside, never by editing a line.
- Home files still parse with Pi's parser unchanged (CN4); any log pointer rides only in a custom entry.
- No transcript content appears in any leaf, log file, error or output (P7, CN3).
- lys-core's published wire formats (lys/attestation/v2, seal, anchor receipts) are not mutated; the home leaf is a new versioned format alongside.
- The session head and head hash do not move because of a leaf (ADR-012).
- `lys open`'s non-oracle failure behaviour (one CliError::OpenFailed) is kept.
- lys-log-store's LeafStore gains no fork, merge, delete or rewrite.
- No file exceeds 500 lines of code; call.rs (~460), mod.rs (~472) and import.rs (~473) cannot absorb much, so new logic goes in named sibling files.
- No unwrap, expect or panic in library code; seed buffers stay Zeroizing, and key material never appears in Debug or errors.

### What we must put in place first

- DIRECTORY-003 R2 landed: crates/lys-identity with the versioned event envelope, and docs/design/identity/IDENTITY-EVENTS.md written and reviewed with Archie. Neither exists today, and DIRECTORY-003 is itself blocked on a file manifest and Waffles' review.
- F-BV2vqP landed: lys-log-store's Log no longer holds every leaf in memory (log.rs:53-60). The card id must first be located, since nothing in the tree names it.
- An identity key the home can read: which Ed25519 key signs, and where it is kept. No key store for an identity exists in lys-home or the directory yet.
- A typed payload for home entries (and sealed opens) added to the identity event envelope, under the same joint review before any durable bytes are signed.

### The risks

- The leaf becomes a frozen wire format on its first signature. A field or ordering mistake found later means a v2 alongside for ever.
- Crash between the durable session line and the leaf: the already_recorded short-circuit in call.rs returns early, so a re-ingest would never add the missing leaf unless it checks the log.
- Hashing the entry line of a short, low-entropy entry lets a guesser confirm content by brute force. The line includes id and timestamp, but the envelope review should state this.
- A whole-transcript import writes one signed leaf per harness event (124 for the proof session), so import time and log size grow, and a failure midway leaves a partly logged import.
- sign_attestation reads the wall clock, so leaves are not byte-reproducible. Tests must verify, not compare bytes.
- Making the published `lys open` write to a log risks turning the verifier CLI into a stateful tool, and a recorded refusal could reintroduce the unsealing oracle seat.rs avoids.
- Starting before DIRECTORY-003 R2 means signing durable bytes under an unreviewed envelope, which R2 forbids.
- A search test that scans zero files passes vacuously unless it asserts the files and leaves it scanned.
- call.rs, record/mod.rs and import.rs are each within ~30-40 lines of the 500-line limit.

### Still open

- A sealed-record open happens in `lys open` (crates/lys), which has no home, session or entry. What does its leaf carry in place of session id and entry id, and which log and key does `lys open` write to? The sentence of the words it stands on: "The leaf carries the session id, the entry id, the kind and the SHA-256 of the entry line, never content, and it is signed with the identity's Ed25519 key through sign_attestation, in the envelope the identity audit events use.". Why only the lead can settle it: crates/lys/src/commands/seal.rs takes an envelope, attestation, sender key and recipient identity, and has no session, entry line or log argument. An open has none of the four leaf fields, so the leaf shape and what a stranger verifies differ for opens.
- Should the verifier CLI `lys open` (published in lys 0.2.0) start writing to a log, or should the recorded open live in lys-home (e.g. an open that goes through the home)? The sentence of the words it stands on: "The outcome: each lys.call entry, each lys.harness_event entry and each sealed-record open appends one leaf to a lys log under the identity.". Why only the lead can settle it: CLAUDE.md and the words call crates/lys 'the verifier command line' and 'a thin surface'. Making `lys open` mutate a log changes what a person running the published binary sees and needs.
- Is a refused open (wrong key, forged attestation) recorded too, or only a successful one? The sentence of the words it stands on: "Opening a sealed record (`lys open`, seal::open_and_verify) leaves no record at all.". Why only the lead can settle it: seal.rs collapses every failure to one non-oracle CliError::OpenFailed. Recording refusals changes what the log shows and must not reopen that oracle.
- Whose Ed25519 key is 'the identity's key' (the agent's own, the person's, or a home key), and where does lys-home read it from, given no identity key store exists yet? The sentence of the words it stands on: "The leaf carries the session id, the entry id, the kind and the SHA-256 of the entry line, never content, and it is signed with the identity's Ed25519 key through sign_attestation, in the envelope the identity audit events use.". Why only the lead can settle it: lys-home has no identity or key anywhere (grep finds none), and DIRECTORY-003 has not built crates/lys-identity. Directory P8 forbids signing with a key the identity does not hold, and ADR-003 gives an agent its own identity under a person. Which key signs decides who a stranger verifies the history against.
- Do entries already in a home (lys.call and lys.harness_event written before this lands) get leaves by a backfill, or does the log start only at entries appended after it? The sentence of the words it stands on: "The outcome: each lys.call entry, each lys.harness_event entry and each sealed-record open appends one leaf to a lys log under the identity.". Why only the lead can settle it: Existing homes hold such entries (e.g. PROOF-RESUME's 124 events). Without a backfill a verifier sees a history with unlogged entries it cannot tell from tampering.
- If the leaf append fails after the entry line is durable, what is kept: the unlogged line left in the session (P1 forbids removing it), or the leaf written before the line so a failed append leaves no line? The sentence of the words it stands on: "A failed append is refused by name with the entry id, and the entry is never reported as recorded.". Why only the lead can settle it: Session::append_line (record/mod.rs:407) makes the line durable first, and P1 says original bytes are never rewritten. Directory P4 refuses 'a change followed by a best-effort log append'. What a person finds in the session after a failure depends on the answer.
- Does a person get a lys-home command that verifies a session against its log, or is the verification the words test only a library function? The sentence of the words it stands on: "Tests: two calls and one event give three leaves that verify under the identity's public key; one byte changed in an entry line makes verification refuse and name that entry; ingesting the same call_id again adds no leaf; a search of the log for a fixture message's text finds nothing.". Why only the lead can settle it: No verify subcommand exists in crates/lys-home/src/cli.rs, and `lys log verify` checks tree proofs, not entry lines. Whether a stranger can run the check changes what a person does.
- DIRECTORY-003 R2 is not built, its envelope is not written, and it must be reviewed with Archie before signing durable bytes; is a new typed payload for home entries added to that envelope under the same review, and does this card wait for it? The sentence of the words it stands on: "It builds after the identity event envelope (DIRECTORY-003 R2) and after the log grows past memory (F-BV2vqP) have landed.". Why only the lead can settle it: docs/design/identity/IDENTITY-EVENTS.md does not exist, crates/lys-identity does not exist, and DIRECTORY-003 is blocked_by a file manifest and Waffles' review. The envelope is designed for directory changes (actor, affected identity, operation id), not session entries.
- What is F-BV2vqP, and is its change already on a board? No reference to it exists in this tree or the estate. The sentence of the words it stands on: "It builds after the identity event envelope (DIRECTORY-003 R2) and after the log grows past memory (F-BV2vqP) have landed.". Why only the lead can settle it: crates/lys-log-store/src/log.rs:53-60 still holds every leaf in memory, and no document, roadmap row or crate names F-BV2vqP, so the prerequisite cannot be confirmed or tracked from here.

### The units beyond the first

- Record each sealed-record open as a signed leaf under the identity — The open happens in crates/lys, the published verifier CLI, which has no home, session or entry. Its leaf fields, its log and whether refused opens count need a product decision and a change to a published binary, which is separate from the lys-home entry path.
- Backfill leaves for lys.call and lys.harness_event entries already in a home — Existing homes hold unlogged entries. Whether and how to log them after the fact is its own decision and migration, and the first unit works without it for new entries.
- A lys-home verify subcommand a stranger can run against a session and its log — The words require verification only in tests. A person-facing command is a surface decision of its own.

### The smallest complete shape

One lys-home unit, built once both prerequisites have landed. lys-home gains lys-core and lys-log-store, plus a home-entry leaf payload (session id, entry id, kind, SHA-256 of the line) inside the DIRECTORY-003 R2 envelope, signed with sign_attestation under the identity's key. One signing and append helper is called from Session's write path for lys.call and lys.harness_event entries (ingest, import, template_render). The failed-append refusal carries the entry id. Re-ingest reconciles a missing leaf. A verify function walks a session against its log and names the first entry whose line hash does not match. The unit also carries the four tests the words name, RECORD.md and the design's non-goal revised, and the adversarial review recorded. Sealed-record opens are held back for the second unit until their leaf shape is decided.

## The roadmap row

- **RM-017** — Sign each call, harness event and home-recorded open of a home into a lys log under the agent's identity (feature, idea)
- Summary: lys-home's lys.call and lys.harness_event entries, and a lys.open entry for each sealed record opened through a new lys-home open, each append one leaf to a lys log the home keeps under its agent's identity: a typed payload of the identity event envelope carrying the session id, the entry id, the kind and the SHA-256 of the entry line, never content, signed with the agent identity's Ed25519 key through sign_attestation; a line a fork copies gets its own copy leaf naming the parent's leaf, or the checkpoint leaf and the line's position when the parent entry predates the checkpoint. lys-home log-enable --agent names the home's agent in log/agent and starts the log with a signed checkpoint covering what the home held before it, and is refused without the agent's key; a home never enabled appends as today. A failed append is refused naming the entry and holds the session pending at log/pending/<session id>.json until its leaf is written; an append to a logging home whose key is absent is refused before its line; lys-home verify checks a session against the log. Built after DIRECTORY-003 R2's envelope and F-BV2vqP's lys-log-store 0.3.0 land.
- Asked by: tom on 2026-09-27T07:26:00+10:00
- Context: The signed entry log request for the home, surveyed against lys main; the lead of the home line answered that the agent's own identity key signs, that the payload joins DIRECTORY-003 R2's envelope under the same review and the card waits for it and for F-BV2vqP, that a failed append leaves the line, refuses by entry and holds the session pending, that a checkpoint leaf written when logging is enabled covers what the home held before the log, that enabling is refused without the agent's key and a logging home without it refuses the append before its line, that lys-home gains a verify subcommand, and that a sealed-record open is recorded through a lys-home open subcommand taking the envelope as a file, as a lys.open entry, only when it succeeds, with lys open unchanged. A second round of answers settled that the home names its agent by log-enable --agent, recorded in log/agent and never inferred from the home's path, and that a line a fork copies gets its own leaf under the child session id naming the parent session and the parent leaf's index, with verify checking the child alone and against the parent. A third answer settled that a copy of a line from before the checkpoint names the checkpoint leaf with the parent entry's position, the fork is not refused, and verify checks that checkpoint leaf and the parent's line byte for byte.
- Quote: lys-home records proxy calls as lys.call entries (ingest_call and ingest_outcome in crates/lys-home/src/record/call.rs) and harness events as lys.harness_event entries in the session JSONL. Its Cargo.toml depends on neither lys-core nor lys-log-store, so no entry is signed and none reaches a lys log. The repository has no server or runtime process: crates/lys is the verifier command line, and `lys log append` appends a leaf file without signing it. Opening a sealed record (`lys open`, seal::open_and_verify) leaves no record at all. The outcome: each lys.call entry, each lys.harness_event entry and each sealed-record open appends one leaf to a lys log under the identity. The leaf carries the session id, the entry id, the kind and the SHA-256 of the entry line, never content, and it is signed with the identity's Ed25519 key through sign_attestation, in the envelope the identity audit events use. A failed append is refused by name with the entry id, and the entry is never reported as recorded. Tests: two calls and one event give three leaves that verify under the identity's public key; one byte changed in an entry line makes verification refuse and name that entry; ingesting the same call_id again adds no leaf; a search of the log for a fixture message's text finds nothing. It builds after the identity event envelope (DIRECTORY-003 R2) and after the log grows past memory (F-BV2vqP) have landed. Not in scope: the door's proxy and the runtime, which live outside this repository, and anchoring, encryption at rest, and haematite as the store.
- Cluster: home; briefs: HOME-008
- Notes: Further units, not written: Put a sealed record into a session (the lead names it a later card; HOME-008 takes the envelope as a file). The survey's other units are carried by the lead's answers: Record each sealed-record open as a signed leaf under the identity is in HOME-008 as lys-home open; Backfill leaves for lys.call and lys.harness_event entries already in a home is replaced by the checkpoint leaf log-enable writes; A lys-home verify subcommand a stranger can run against a session and its log is in HOME-008.

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

Today a session exists only as its harness's file. Archie's Claude Code file measured 141,931,097 bytes on 24 September 2026: the conversation itself is under 3% of it, tool results 14%, thinking 6%, and about 60% is harness bookkeeping the model never sees; Waffles' file is 3.37 GB and Claude Code loads it whole. Nothing central holds a session, so nothing can render a smaller working session, move one to another device, swap provider and come back, or say what a session was given. Norn already has the canonical event model (SessionEvent, conversion to provider messages, compaction as a derived event, provider epoch boundaries) proved by its own tests; Claude Code has no importer to it and no renderer from it. Nothing a home holds is signed and none of it reaches a lys log: lys-home depends on neither lys-core nor lys-log-store, so a lys.call or lys.harness_event entry is only a line in a file anyone with the file can change, and no stranger can tell an honest history from an edited one.

## Solution

Adopt Pi's session tree as the home record (Tom, Dot 13:27 and 13:28: Pi's tree, not Norn): one append-only JSONL per session, a header line, then entries each carrying id, parentId and timestamp, a leaf pointer for the current position, forks by moving the pointer, compaction and branch summaries as entries that keep their originals. lys adds nothing to that grammar: harness events (approvals, tool completion) and proxy call records ride Pi's custom entry type under lys customType names, so a home file stays readable by Pi's own parser. Two captures feed it: the model traffic through the door's proxy (the same process that swaps the credential, SECRETS-002 R1), and the harness's own events from its transcript. Content blocks are stored once by hash; entries reference them. A harness resume file is a rendered projection of the root-to-leaf path: for Claude Code, a JSONL written under a chosen uuid at the harness's own path, then resumed by that id with --fork-session. Provider-native reasoning stays on the message with its provider, api and model and is rendered whole only to the same three; another model gets readable thinking as text and opaque blocks dropped, each named in a loss account. lys grants say who may read and resume. Every resume path is measured on a named harness version before anything relies on it. A session is launched on a harness from a launch template kept in the home (ADR-012): one JSON object per harness, schema in docs/design/home/launch-template.schema.json, stored under templates/ by its SHA-256 beside sessions/ and blocks/, so a template's version is its hash. Its named slots map the home onto the harness: the transcript (the one slot no template maps generically, so the template names how the harness fills it; Claude Code fills it by resuming the rendered file by path with --fork-session, since a bare resume writes onto the rendered file's own name), the MCP configuration, the environment, the secrets (use-only ones written as their handle; readable ones refused until the secrets rows build the broker reader, ADR-001) and the appended instructions. lys-home render-launch reads a template and a session, writes the R4 render, its loss account, an MCP file, an environment file and an instructions file into one directory, and prints the launch line in its report without running it (ADR-007). Each render is recorded on the session as a lys.harness_event of the sixth kind, template_render, hung as a side leaf beside the context path so the head and the session head hash (SHA-256 of the head entry's line) do not move; the written paths ride in a manifest block the event names by hash, under the 512-byte cap. At render, the launch template also records the context record: a lys.given custom entry after the render event naming, by path, byte length and SHA-256 only, the instruction documents Claude Code will load for the session's working directory and the files the render wrote, in the order measured on a named Claude Code version, with the environment variable names the template set and the kinds of document resolved and left unlisted (@-imports and .claude/rules, which a second lys.given entry at the first request records), so a reader sees what was measured and can check a file on disk against it without anyone reading its contents. Lanterns (stage 5, HOME-004) are the first thing that stands on the record: a lantern is a lys.lantern custom entry lit on purpose and appended at a session's head, naming an existing non-lantern entry of that session as its point (ADR-014); its note grows only by lys.lantern_epilogue entries appended after it, so a lantern's story is its entry followed by its epilogues and nothing is rewritten. Recall reads every session of the home without owning any of it: a read-only reader takes no lock and writes no index, seeks to lantern and epilogue rows by the index's custom column (CN7), and names any session it could not read rather than shortening the list in silence. A lantern lives in the home only; the Claude Code renderer already leaves custom entries out. A render is one function of the session head, the target (session id, cwd, model, version, canon) and the lys-home version: the same inputs give the same bytes (CN9), so a rendered file is told by its hash and a receipted render event can name what was written. The walk takes the order the record gives, entry order along the context path then part order within a message, never a map's or a set's order. A value the render needs that the record does not hold is derived from the record, never drawn fresh: a timestamp is the entry's own stamp, and a Claude Code uuid for an entry whose id is not already uuid-shaped (the importer's `<uuid>-r<i>` ids for split tool results, hand-authored ids, canon ids) is a UUID version 5 (RFC 9562, SHA-1 name-based) under the session's namespace over the name `<entry id>#<role>`. The session's namespace is UUIDv5 of one fixed lys namespace, 32c05904-d1f1-550c-9eee-2f6c8f98b665, over the id of the session being rendered; that lys namespace is itself UUIDv5 of the RFC 9562 URL namespace (6ba7b811-9dad-11d1-80b4-00c04fd430c8) over the name `lys/home/claude-code/render-uuid/v1`. The session in the namespace is the salt: the same hand-authored or split-result id in two sessions never derives one uuid, and the target session id, which the record does not hold, is never mixed in. The name is the entry's own id, then `#`, then the role the id plays in the rendered file, as UTF-8 bytes; roles carry no `#`, so the last `#` splits a name unambiguously and two roles on one entry never share a name. The roles are closed: `record`, the rendered record's `uuid`. The parentUuid of the next record, a summary's leafUuid and an assistant's `msg_` id follow from it unchanged. A uuid-shaped entry id passes through as it is, so an imported Claude Code session keeps its source's uuids. A derived uuid carries version nibble 5 and Claude Code's own uuids carry 4, so a derived uuid cannot equal an imported one. The namespace, the session namespace rule, the name form and the roles are fixed (ADR-016); the fork and launch cards derive their ids by this scheme, and a change to it is a new version alongside, never a mutation. The fork (stage 5b, HOME-006) is the pathway a lantern makes: lys-home fork resolves a lantern to the session it was lit in, cuts that session's root-to-point chain at the last assistant message at or before the point, reading through the index, and writes a child session under the parent's cwd whose header's parentSession is the parent file's path relative to the home, holding the parent's own line bytes for each cut entry, then a lys.forked_from entry naming the parent, the lantern, the point and the cut (ADR-017); the parent gains one lys.fork entry at its head naming the child. Nothing else is copied and no block is written. A user-message point is carried, not copied: the child's Claude Code render writes its text parts as a seed prompt beside the rendered file under an in-band marker line, and the template's launch line, printed by render-launch only, passes it as the first prompt (ADR-018). The child renders and launches like any session, through the launch template. The signed entry log (HOME-008, ADR-019) is where the home's history becomes verifiable by a stranger. A home logs its entries once lys-home log-enable --agent has run on it, naming the home's agent identity in log/agent under the home, which is refused unless the identity crate holds that agent's key; the agent is never inferred from the home's path; a home that never ran it appends as before, with no leaf and no log. On a logging home each lys.call, each lys.harness_event and each lys.open entry, written by any of the four writers of a session line (append_entry, append_beside, append_under and a fork's append_copied), appends one leaf to one lys log the home keeps at log/ beside sessions/, blocks/ and templates/, lys-log-store's file storage, serving every session of the home, since the home is its agent's and the log is kept under that agent's identity. A leaf is the identity event envelope's own encoding (DIRECTORY-003 R2) of a typed payload tagged lys/home-entry/v1, carrying the session id, the entry id, the kind and the SHA-256 of the entry's line, never content (CN11), with the COSE_Sign1 attestation sign_attestation makes over it under the agent identity's own Ed25519 key, read from where the identity crate holds it; there is no home key and no stand-in key, and an append of a logged kind to a logging home whose agent key is absent is refused before its line is written. The line comes first and is never removed or rewritten (P1); the leaf is derived from the durable line alone and appended after it, so a failed append is refused naming the entry, marks the session pending in the marker log/pending/<session id>.json naming the session and the entry, and refuses every further append to that session until the leaf is written, first thing at the session's next open or by lys-home verify --repair (CN12). A line a fork copies into a child gets its own leaf under the child's session id, tagged lys/home-entry-copy/v1 and naming the parent session, the parent entry's position in the parent's file and the index of the parent entry's leaf, so the child verifies without reading the parent's files and verify checks the parent's leaf in the same log. A copied line whose parent entry predates the checkpoint, and so has no leaf, names the checkpoint leaf in its place; verify accepts it when that checkpoint leaf is in the log and the parent's entry at that position is byte-equal to the copied line, and the fork is never refused for it. No entry records its leaf's coordinate: the leaf is found in the log by its session and entry id. log-enable writes the log's first leaf, one checkpoint tagged lys/home-checkpoint/v1, naming each session with its entry count and the SHA-256 of its file as it stood; entries before it are covered by that hash and every entry after it by its own leaf. A sealed record is opened through the home by lys-home open, which takes what lys open takes plus the session that records the open and the --out path, refuses an existing --out, an absent session, a pending session and a logging home without its agent's key before anything is opened, writes the plaintext there in a file created new with mode 0600 on Unix, and appends one lys.open entry naming the envelope by SHA-256, the sender's public key and the --out path, never the plaintext or the envelope; a refused open gives lys open's one failure message and records nothing, so the log is no oracle. lys-home verify walks one session against the log and the checkpoint and prints a report of counts and hashes, or names the first entry that fails. The verifier command line, crates/lys, is unchanged by this: lys open writes nothing.

## Principles

- **P1** — The original bytes are never rewritten. A rendered resume file, a compaction and a translation are derived records stored beside their source and pointing at it (CONTEXT-ROADMAP-2026-09-22.md stage 4).
- **P2** — The record is Pi's session tree as read from the Pi checkout at 3d5cbe98 (packages/coding-agent/src/core/session-manager.ts: SessionHeader, SessionEntryBase {type, id, parentId, timestamp}, message, model_change, compaction {summary, firstKeptEntryId}, branch_summary {fromId, summary}, label, custom {customType, data}). lys adds entry kinds only as custom entries; it never adds a field to Pi's grammar and never adopts Norn's SessionEvent (Tom, Dot 13:28).
- **P3** — A provider's opaque blocks (Anthropic signed thinking, OpenAI encrypted reasoning) stay on the assistant message with its provider, api and model, and are rendered whole only when all three match the target (Pi transform-messages.ts:95-109); another model gets readable thinking as plain text and opaque blocks dropped, each named by hash in the loss account (Tom, Dot 13:24: keep reasoning traces per provider so a session can swap and swap back).
- **P4** — A content block is stored once by its hash; requests that resend the whole conversation reference blocks, they do not copy them.
- **P5** — Capture the model traffic and the harness's local events both, so the two can be mapped against each other (Tom, Dot 13:23); neither alone is the record.
- **P6** — A resume path is a per-harness, per-version measurement, never an assumption: Claude Code 2.1.281's --resume takes a session id and reads ~/.claude/projects/<cwd-slug>/<id>.jsonl; a seeded two-record file resumed there at 13:25 on 24 September and answered from its content.
- **P7** — Transcript contents (message, tool and compaction content) never appear in a post, a log line, an error, a test name or a rendered page; status carries hashes, counts and offsets only (CONTEXT-001 privacy rule). A lantern's note and its epilogues are the one exception: they are the person's own annotations, not transcript, and only recall prints them (ADR-015).
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
- ADR-014 — A lantern is a custom entry in its session, and its note grows only by epilogue entries — A lantern is a `lys.lantern` custom entry appended at its session's head, carrying in custom.data the entry id of its point (an existing entry of the same session that is not itself a lantern or an epilogue, the head or any entry the head has moved past), the note as written, who lit it and when. Its note grows only by `lys.lantern_epilogue` custom entries naming the lantern's entry id and carrying the further words, who added them and when; a lantern's story is its entry followed by its epilogues in order, and nothing is rewritten. Rejected: Pi's `label` entry on the target (it replaces or clears a label rather than growing one, and carries no author or time), a lantern store beside the session outside Pi's grammar (a lantern would stop travelling with its session), and editing the lantern's note in place (the record is append-only, P1).
- ADR-015 — A lantern's note and epilogues are the person's annotations, not transcript — A lantern's note and its epilogues are the person's own annotations, not transcript. Recall prints them by design; they are the one exception to P7 and CN3, and only recall prints them. Transcript lines (message, tool and compaction content) still never appear in output, logs or errors, and a recall row never carries a line of the transcript around its point. Rejected: treating notes as transcript and printing only their hashes, which makes recall useless to the person who wrote them.
- ADR-016 — A rendered file's derived uuids are a fixed, versioned contract — Derive every uuid a render needs and the record does not hold as UUIDv5 under the session's namespace and the name `<entry id>#<role>`, with a closed set of roles (`record` first); the session's namespace is UUIDv5 over one fixed lys namespace (32c05904-d1f1-550c-9eee-2f6c8f98b665, itself UUIDv5 of the RFC 9562 URL namespace over `lys/home/claude-code/render-uuid/v1`) and the id of the session being rendered, so the same entry id in two sessions never derives one uuid. Treat the namespace, the session namespace rule, the name form and the roles as frozen: a change is a new version alongside, never a mutation. The fork and launch cards derive by this scheme. Rejected: drawing fresh ids (nondeterministic), keying on a hash of the entry id alone without a namespace, a role and a session (two roles on one entry collide, two sessions with one hand-authored id collide, and it is not reproducible with standard UUID tooling), mixing in the target session id (the record does not hold it), and leaving the scheme mutable until a later card (every recorded hash would move with it).
- ADR-017 — A fork is a child session cut from the parent's own lines at a lantern's point, with its ancestry on both sides — A fork resolves a lantern to the session it was lit in, read from the lys.lantern data's lit_in when the record carries it and otherwise by the older-record rule (one holder cuts, several refuse lantern_ambiguous until a session is named), and cuts that session's root-to-point chain at the last assistant message at or before the point, through the index. The child is a new session under the parent's cwd whose header's parentSession is the parent file's path relative to the home, holding each cut entry as the parent file's own line bytes, then one lys.forked_from custom entry as its head naming the parent session, the lantern, the point, the cut entry, whether the coordinate was carried and the carried entry; the parent gains one lys.fork custom entry at its head naming the child. Nothing else is copied and no block is written. Rejected: re-serialising the copied entries (the copy would stop hash-matching the parent's lines), a fork store beside the sessions outside Pi's grammar, cutting at a point no lantern names, and a header field beyond Pi's parentSession.
- ADR-018 — A user-message point is carried as a seed prompt beside the rendered file, never copied into the child — When the point is a user message the cut stops at the assistant message before it and the message is carried, not copied: lys.forked_from records its id with coordinate_carried true and counts, by kind, the parts of it that are not text. The Claude Code render of such a child writes the message's text parts, in order, as a seed prompt beside the rendered file under an in-band marker line naming the parent session, the point and the lantern, and names it in the render report; the template's launch line, printed by render-launch only, passes that file as the resumed session's first prompt. A part that is not text never refuses a fork or a render and never enters the seed. Rejected: copying the user message into the child's chain, refusing a fork for a non-text part, and putting the seed's text in the report or the loss account.
- ADR-019 — A home entry's leaf is a typed payload of the identity event envelope, signed by the agent's own key after its durable line — Each lys.call and lys.harness_event entry appends one leaf to one log the home keeps under its agent's identity: a typed payload of the identity event envelope (DIRECTORY-003 R2), tagged lys/home-entry/v1, carrying the session id, the entry id, the kind and the SHA-256 of the entry's line, signed through sign_attestation with the agent identity's own Ed25519 key as the identity crate holds it. The line is written first and never removed; a failed leaf append is refused naming the entry, marks the session pending and refuses further appends until the leaf is written at the next open or by verify --repair. The log starts with one checkpoint leaf, tagged lys/home-checkpoint/v1, naming each session with its entry count and file SHA-256. lys-home gains a verify subcommand. Rejected: a second envelope for home entries; a home key, a person's key or a stand-in key; writing the leaf before the line; a best-effort append that lets the session continue past an unlogged line; a per-entry backfill; verification as a library function only; and recording opens in the published lys open.
- ADR-020 — The home's non-goal on signing narrows to anchoring — Signing into a lys log is in scope for lys.call entries, lys.harness_event entries and sealed-record opens, and for nothing else; anchoring stays out of scope. Rejected: keeping the whole non-goal, and widening signing to every entry kind.

## Goals

- A few-shot session file written by hand resumes Claude Code by path, from a directory outside the config root, with the file preserved and the demonstration marked authored.
- One real Claude Code session imported into the common record, rendered back, and resumed under a new id on 2.1.281 without repeating a completed tool action, with the original file's hash unchanged.
- One seat with a subscription login making calls through a pass-through proxy, so the tee has somewhere to live.
- A written loss account for every derived record: what a render or translation preserved, transformed and could not carry.
- The canon, one curated versioned series of short examples each showing a rule lived, seeds every new session, and its effect is measured on a card against a plain start.
- A handover letter from an outgoing session seeds its successor as inherited memory, with the model's own thinking intact, and the effect is measured on a card against a plain start.
- A session in the home renders, from a Claude Code launch template kept in the home by hash, into a directory of launch files whose hashes are identical on a second render, and the printed launch line resumes it on the installed Claude Code version, measured.
- Every launch-template render records what the session was given: one lys.given entry after the render event naming each instruction document Claude Code will load, and each file the render wrote, by path, byte length and SHA-256 in the measured order, with the environment variable names the template set, and never a document's content; lys-home lists those records and checks a file on disk against one by hash.
- A lantern lit on purpose at a past entry of a session is recalled from the home by a word of its note and by its point, with its epilogues after it and never a line of the transcript around it.
- The same session head rendered twice with the same target and the same lys-home version gives files of equal SHA-256, a fixture holding a tool record with more than one result included, and the fixture's hash is pinned by a test and recorded in PROOF-RESUME.md.
- A session forked from a lantern's point holds the parent's own lines up to the last assistant message before the point, carries its ancestry on both sides, adds nothing to the block store, and is rendered and launched on the installed Claude Code version, measured, with a user-message point carried as the child's first prompt.
- Two ingested calls and one harness event give three leaves in the home's log that lys-home verify accepts under the agent identity's public key; one byte changed in one of those entry lines makes it refuse naming that entry; ingesting the same call id again adds no leaf; a sealed record opened through lys-home open adds one lys.open entry and one leaf; and a byte scan of the log directory for a fixture message's text finds nothing.

## Non-Goals

- Anchoring a home's log (CONTEXT-ROADMAP stage 6), receipts from an anchor, and signing any entry kind other than lys.call, lys.harness_event and lys.open. — Signing into a lys log was asked for with calls, harness events and sealed-record opens named (ADR-020), an open recorded through the home as a lys.open entry; anchoring was named out of scope with it, and lys.given (ADR-013), lanterns and forks are not among them.
- Encryption at rest and moving a home between devices (stage 3 preconditions). — Stage 3's three preconditions (identity and read authority, encryption before bytes leave, the resume evidence) are their own brief.
- Harnesses other than Claude Code, and Chat Completions or Responses translation beyond keeping the raw call bytes. — One harness proved first; each other harness is its own profile and its own measurement.
- Cutting at a point no lantern names, forks with tools, a lens or a briefing on what changed since the point, and leases, budgets and kill rules for a fork. — The fork through a lantern (stage 5b) is HOME-006; it takes a lantern id only, gives the child nothing but its own record, and what runs a fork and for how long belongs to the engine that launches it.
- Adopting, wrapping or calling Norn's session code; Pi's code is read as the reference and not vendored. — Tom, Dot 13:28: not Norn. Pi's tree is the reference.
- Reading a secret's value through the broker at launch. — The broker's own read belongs to the secrets rows (SECRETS-002); until its reader exists a readable secret is refused and only handles render.
- Recall by resonance, whispers or vectors, any ranking, dimming lanterns by code churn, Norn's tables and Cambium's lantern screen. — Out of scope of stage 5: recall is by an exact case-folded phrase or by point; each of these is its own retrieval or surface design.
- Making the Claude Code import deterministic for the same transcript (sidechain labels, the lys.authored mark, compaction ids and side-leaf harness events draw fresh ids). — The importer is out of scope for the deterministic render; determinism is claimed for a render of one session head, not for two imports of one transcript.
- Rendering tool parts from the stored block, and moving the render's default Claude Code version off 2.1.281. — Both change what a rendered file holds; the deterministic render changes nothing a file holds beyond the determinism.
- Haematite as the store of a home's log. — Named out of scope; the log is lys-log-store's file storage, as DIRECTORY-003 R2 also requires.
- The door's proxy and the runtime that will call ingest_call. — They live outside this repository; they receive the named refusal of a failed append once it lands.
- An entry kind that holds a sealed record in a session. — lys-home open takes the sealed envelope as a file; putting a sealed record into a session is a later card.

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
| `crates/lys-home/src/record/mod.rs` | the home record: Pi's session tree read and written, leaf pointer, root-to-leaf path; append_entry calls the entry log after the head, and Session::open writes a pending leaf first and it declares leaf, entry_log, log_checkpoint, pending, open and verify with their tests (HOME-008) | HOME-001 |
| `crates/lys-home/src/record/entries.rs` | Pi's entry types as Rust types, plus the lys custom entries lys.harness_event and lys.call | HOME-001 |
| `crates/lys-home/src/record/call.rs` | a proxy call record: request and response block hashes, provider, api, model, timing | HOME-001 |
| `crates/lys-home/src/harness/claude_code/events.rs` | Claude Code's harness-local records (hooks, permission mode, tool completion) as lys.harness_event entries | HOME-001 |
| `docs/design/home/RECORD.md` | the home record written down: Pi's grammar as adopted, the two lys custom entries, the block store, the loss account | HOME-001 |
| `crates/lys-home/src/lib.rs` | module wiring: record and harness; declares cli_entry_log and cli_open (HOME-008) | HOME-001 |
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
| `crates/lys-home/src/record/beside.rs` | append beside the context path without moving the head; the session head hash; calls the entry log at the end of append_beside and append_under (HOME-008) | HOME-002 |
| `crates/lys-home/src/record/beside_tests.rs` | gates on the side-leaf append and the head hash | HOME-002 |
| `crates/lys-home/tests/launch_template.rs` | render-launch end to end: recorded hashes, twice identical, handle only, refusals, exit 2 | HOME-002 |
| `crates/lys-home/tests/fixtures/launch/template.json` | the fixture Claude Code launch template, handle-only secrets | HOME-002 |
| `crates/lys-home/tests/fixtures/launch/session.jsonl` | the synthetic fixture session in Pi's grammar, no transcript content | HOME-002 |
| `crates/lys-home/src/error.rs` | the home's errors; gains the template refusals |  |
| `crates/lys-home/src/harness/claude_code/render_tests.rs` | gates on the R4 render, the deterministic render among them |  |
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
| `docs/design/home/briefs/HOME-004.json` | the lantern brief: light, epilogue and recall by note and by point | HOME-004 |
| `docs/design/home/briefs/HOME-004.md` | its rendered markdown | HOME-004 |
| `crates/lys-home/src/record/reader.rs` | a session read without owning it: no lock, no index or head written; the home's session ids | HOME-004 |
| `crates/lys-home/src/record/reader_tests.rs` | gates on the reader: reads a held session, writes nothing, lists every session | HOME-004 |
| `crates/lys-home/src/record/lantern.rs` | lighting a lantern: checks the point and the note, appends one lys.lantern entry at the head | HOME-004 |
| `crates/lys-home/src/record/lantern_tests.rs` | gates on lighting: earlier bytes kept, head is the lantern, each refusal by name | HOME-004 |
| `crates/lys-home/src/record/epilogue.rs` | adding an epilogue: names a lantern of the session, appends one lys.lantern_epilogue entry at the head | HOME-004 |
| `crates/lys-home/src/record/epilogue_tests.rs` | gates on epilogues: appended after the lantern, unknown lantern and blank words refused | HOME-004 |
| `crates/lys-home/src/record/recall.rs` | recall by note (exact phrase, case-folded) and by point, over every session, skipped sessions named | HOME-004 |
| `crates/lys-home/src/record/recall_tests.rs` | gates on recall: phrase in one text, both lanterns on a point, epilogues in order, no transcript | HOME-004 |
| `crates/lys-home/src/cli_lantern.rs` | the lantern subcommands: light, epilogue, recall, each printing one JSON report | HOME-004 |
| `crates/lys-home/tests/lantern_cli.rs` | the lantern subcommands run as the binary: reports, refusals, exit codes, no transcript in output | HOME-004 |
| `crates/lys-home/tests/lantern_home.rs` | a lantern lives in the home: the rendered Claude Code file carries no lantern line | HOME-004 |
| `docs/design/home/PROOF-LANTERN.md` | the dev record: Pi's parser run under node on a session holding lanterns and epilogues | HOME-004 |
| `crates/lys-home/tests/fixtures/multi_result.jsonl` | a synthetic Claude Code transcript with a two-result tool record and a one-result-plus-text record, the determinism fixture | HOME-007 |
| `Cargo.toml` | the workspace manifest; gains the sha1 dependency the render's UUIDv5 needs |  |
| `Cargo.lock` | the workspace lockfile |  |
| `docs/design/home/briefs/HOME-007.json` | the deterministic Claude Code render | HOME-007 |
| `docs/design/home/briefs/HOME-007.md` | its rendered markdown | HOME-007 |
| `docs/design/home/briefs/HOME-006.json` | the fork brief: cut at a lantern's point, ancestry on both sides, the seed and the launch line | HOME-006 |
| `docs/design/home/briefs/HOME-006.md` | its rendered markdown | HOME-006 |
| `crates/lys-home/src/record/fork_cut.rs` | resolving a lantern to its lit-in session and cutting the chain at the last assistant message, through the index | HOME-006 |
| `crates/lys-home/src/record/fork_cut_tests.rs` | gates on the names, the refusals and the cut: order, side leaves out, bytes read, each refusal by name | HOME-006 |
| `crates/lys-home/src/record/fork.rs` | writing the child from the parent's own lines, lys.forked_from in the child and lys.fork at the parent's head; append_copied appends a copy leaf on a logging home, naming the checkpoint leaf for a parent entry from before it (HOME-008) | HOME-006 |
| `crates/lys-home/src/record/fork_tests.rs` | gates on the fork: lines hash-equal, both ancestry entries, no block written, held parent refused, the report's counts | HOME-006 |
| `crates/lys-home/src/record/fork_report.rs` | the fork report: entries copied, block hashes held and unstored, ids only | HOME-006 |
| `crates/lys-home/src/cli_fork.rs` | the fork subcommand: one JSON report, refusals on stderr | HOME-006 |
| `crates/lys-home/src/harness/claude_code/seed.rs` | the seed prompt of a carried user message: the marker line, the text parts, the seed argument for the template's launch line | HOME-006 |
| `crates/lys-home/tests/fork.rs` | the fork run as the binary: five forks and five refusals, blocks unchanged, both ancestry sides | HOME-006 |
| `docs/design/home/PROOF-FORK.md` | the measured fork: Pi's parentSession read, a real session forked, rendered and launched, as hashes, counts and exit codes | HOME-006 |
| `crates/lys-identity/src/lib.rs` | the identity crate's root, brought by DIRECTORY-003 R2; declares the home payloads when its envelope lands without them (HOME-008 R1), changed only after it lands | DIRECTORY-003 |
| `crates/lys-identity/src/home_payloads.rs` | the lys/home-entry/v1, lys/home-entry-copy/v1 and lys/home-checkpoint/v1 payloads of the identity event envelope, created only when the envelope lands without them | HOME-008 |
| `docs/design/identity/IDENTITY-EVENTS.md` | the identity event envelope (DIRECTORY-003 R2); records the three home payloads and their adversarial review (HOME-008 R1, R12), changed only after DIRECTORY-003 R2 brings it | DIRECTORY-003 |
| `crates/lys-home/src/record/call_tests.rs` | call ingest tests |  |
| `docs/design/home/briefs/HOME-008.json` | the signed entry log | HOME-008 |
| `docs/design/home/briefs/HOME-008.md` | rendered HOME-008 | HOME-008 |
| `crates/lys-home/src/record/leaf.rs` | the encoding and decoding of a home leaf: the lys/home-entry/v1, lys/home-entry-copy/v1 and lys/home-checkpoint/v1 payloads in the identity envelope with their attestation | HOME-008 |
| `crates/lys-home/src/record/leaf_tests.rs` | leaf encoding and signature tests | HOME-008 |
| `crates/lys-home/src/record/entry_log.rs` | the home's log at log/ and its agent named in log/agent: sign and append a leaf with that agent's key, find a leaf and its index by session and entry | HOME-008 |
| `crates/lys-home/src/record/entry_log_tests.rs` | entry log tests | HOME-008 |
| `crates/lys-home/src/record/log_checkpoint.rs` | the checkpoint leaf written when logging is first enabled on a home | HOME-008 |
| `crates/lys-home/src/record/log_checkpoint_tests.rs` | checkpoint tests | HOME-008 |
| `crates/lys-home/src/record/pending.rs` | the pending marker log/pending/<session id>.json (session and entry ids) and the retry of its leaf at the session's next open, its next append, or verify --repair | HOME-008 |
| `crates/lys-home/src/record/pending_tests.rs` | pending marker tests | HOME-008 |
| `crates/lys-home/src/record/verify.rs` | verify one session against the log and the checkpoint, a fork's copy leaves against their parent leaves, or against the checkpoint and the parent's line when they name the checkpoint leaf | HOME-008 |
| `crates/lys-home/src/record/verify_tests.rs` | verify tests | HOME-008 |
| `crates/lys-home/src/cli_entry_log.rs` | the log-enable subcommand (--agent, written to log/agent) and the verify subcommand with its --repair | HOME-008 |
| `crates/lys-home/src/record/open.rs` | open a sealed envelope through the home and append its lys.open entry | HOME-008 |
| `crates/lys-home/src/record/open_tests.rs` | open-through-the-home tests | HOME-008 |
| `crates/lys-home/src/cli_open.rs` | the open subcommand | HOME-008 |
| `crates/lys-home/tests/entry_log.rs` | the words' four tests, log-enable, the checkpoint, the absent key, the failed append, the open and the wrong key, through the command line | HOME-008 |
| `crates/lys-home/tests/fixtures/entry_log/one_event.jsonl` | a Claude Code transcript of one permission-mode record and no message | HOME-008 |
| `crates/lys-home/tests/fixtures/entry_log/prelog/sessions/pre.jsonl` | the prelog home's session pre, written before logging: one lys.call and one lys.harness_event | HOME-008 |
| `crates/lys-home/tests/fixtures/entry_log/prelog/sessions/pre.index.jsonl` | the prelog home's index of session pre | HOME-008 |
| `crates/lys-home/tests/fixtures/entry_log/prelog/sessions/pre.head` | the prelog home's head of session pre | HOME-008 |

## Inventory

- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority: 'The home', 'The home is portable: lanterns, translation and forks', and steps 4 and 5
- `docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md` — Archie's roadmap; this cluster is its stage 4 and the resume half of stage 3
- `docs/design/identity/briefs/CONTEXT-001.json` — stage 1, byte-for-byte capture and the lys-home crate this cluster extends
- `docs/design/secrets/briefs/SECRETS-002.json` — the door's proxy (R1) that the tee joins once it exists
- `$PI/packages/coding-agent/src/core/session-manager.ts` — Pi's session tree at 3d5cbe98 (github.com/earendil-works/pi): header, entries, leaf (branch at :1125 moves an in-memory cursor; _buildIndex:754 restores the last physical entry on reopen; loadEntriesFromFile:438 reads the whole file)
- `$PI/packages/ai/src/api/transform-messages.ts` — per-provider thinking: same provider, api and model keeps blocks whole; otherwise text, opaque dropped
- `$PI/packages/coding-agent/src/core/branch-summarization.ts` — how an abandoned branch's work is carried into the new one
- `$CCFLARE/README.md` — ccflare at 95c4c6a (github.com/snipeship/ccflare): the concept of a native pass-through that keeps request history; reference only, not used (Tom, Dot 13:34)
- `crates/lys-home/src/harness/claude_code/render.rs` — on 0073b966 record_uuid gives an entry id that is not uuid-shaped a fresh random uuid (record::fresh_id) on every render, so two renders of a session holding the importer's `<uuid>-r<i>` ids differ; the launch template then shaped one from the SHA-256 of the entry id alone, outside any namespace, without a role or a session; it reads no clock and iterates no map or set
- `crates/lys-home/src/harness/claude_code/import.rs` — a user record's tool results each become an entry with id `<uuid>-r<i>`; only the last keeps the record's uuid, and only when the record has no parts of its own; unchanged by this cluster's determinism work
- `crates/lys-home/src/record/mod.rs` — Session::append_entry and append_beside both write through append_line (line, then index row; append_entry then the head); every lys.call and lys.harness_event line is written here, and nothing is signed or logged. About 472 lines of code, near the 500-line limit.
- `crates/lys-home/src/record/call.rs` — ingest_call, ingest_call_files and ingest_outcome append lys.call through finish_ingest_counted; find_call returns early with already_recorded for a call id on record. About 460 lines of code.
- `crates/lys-core/src/attestation/sign.rs` — sign_attestation(payload, &Ed25519Identity) makes a lys/attestation/v2 COSE_Sign1 over SHA-256(payload) with a wall-clock timestamp; verify_attestation and verify_attestation_bytes_by_signer check one. The protected content type is the generic attestation, so what was signed is carried in the payload.
- `crates/lys-log-store/src/file.rs` — FileLeafStore: log.json, leaves/<20-digit index> holding raw leaf bytes, state.json; every append fsynced with its directory.
- `crates/lys-log-store/src/log.rs` — Log over a LeafStore; at 0.2.0 it holds every leaf in memory (log.rs:53-60), the limit F-BV2vqP lifts in lys-log-store 0.3.0.
- `crates/lys/src/commands/seal.rs` — lys open wraps seal::open_and_verify and collapses every failure to one CliError::OpenFailed; it takes no home, session or log and writes nothing but the opened payload.
- `docs/design/directory/briefs/DIRECTORY-003.json` — R2 defines the versioned identity event envelope outside lys-core with typed payloads, written in docs/design/identity/IDENTITY-EVENTS.md and reviewed before it signs durable bytes; neither that file nor crates/lys-identity exists yet.

## Constraints

- **CN1** — No file under ~/.claude/projects is rewritten or truncated by this cluster; a render writes a new file under a new uuid only, and refuses an existing path by name.
- **CN2** — A rendered file never carries a provider-native opaque block that came from a different provider or model family than the one it is rendered for; the loss account names each block dropped by hash.
- **CN3** — Transcript lines (message, tool and compaction content) never appear in output, logs, errors, test names or pages; hashes, counts, offsets and event ids only. A lantern's note and its epilogues are the one exception, printed only by recall, and a recall row never carries a line of the transcript around its point (ADR-015).
- **CN4** — Every home file parses with Pi's parseSessionEntries at 3d5cbe98 unchanged: the header line first, every entry with id, parentId and timestamp, lys data only inside custom entries.
- **CN5** — The pass-through proxy forwards every header and the streamed body unchanged and stores nothing but the measurement; it is an example binary, never a service.
- **CN6** — No Norn crate is a dependency of lys-home and no Norn type is copied into it.
- **CN7** — Reading the root-to-leaf path of a home file reads only the entries on that path plus the index, never the whole file (Pi loads the whole journal; Chippy 13:33); the head is persisted beside the file, never inferred from the last physical entry on reopen.
- **CN8** — No secret value is written to any file, report, launch line, error or entry by lys-home; a use-only secret appears only as its handle.
- **CN9** — A render of the same session head with the same target and the same lys-home version writes the same bytes: the render walks entry order then part order and never a map's or a set's order, reads no clock and no random source, and derives any uuid the record does not hold as UUIDv5 under the session's namespace (UUIDv5 of 32c05904-d1f1-550c-9eee-2f6c8f98b665 over the session id) and the name `<entry id>#<role>` (ADR-016).
- **CN10** — A fork copies the parent file's own line bytes and writes no block; nothing of the parent's header but cwd, and no credential, handle or launch setting, enters the child; the parent gains one lys.fork entry at its head and no earlier byte of it changes.
- **CN11** — A home log leaf carries the session id, the entry id, the kind and the SHA-256 of the entry line (a fork's copy leaf also the parent session id, the parent leaf's index and the parent entry's position; the checkpoint, session ids, entry counts and file SHA-256s) and nothing else: no byte of transcript content, opened plaintext or sealed envelope enters the log, its errors or its reports.
- **CN12** — A failed leaf append never removes or rewrites the entry's line: the session is marked pending, the entry is not reported as recorded, and no further entry is appended to that session until the pending leaf is written. A logging home never holds an unlogged lys.call, lys.harness_event or lys.open line written while its agent key was absent.


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
title: Light a lantern at an entry of a session, grow its note by epilogues, and recall it by note and by point
---

# HOME-004: Light a lantern at an entry of a session, grow its note by epilogues, and recall it by note and by point

> **Cluster:** home
> **Depends on:** HOME-001
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-014 — A lantern is a custom entry in its session, and its note grows only by epilogue entries — A lantern is a `lys.lantern` custom entry appended at its session's head, carrying in custom.data the entry id of its point (an existing entry of the same session that is not itself a lantern or an epilogue, the head or any entry the head has moved past), the note as written, who lit it and when. Its note grows only by `lys.lantern_epilogue` custom entries naming the lantern's entry id and carrying the further words, who added them and when; a lantern's story is its entry followed by its epilogues in order, and nothing is rewritten. Rejected: Pi's `label` entry on the target (it replaces or clears a label rather than growing one, and carries no author or time), a lantern store beside the session outside Pi's grammar (a lantern would stop travelling with its session), and editing the lantern's note in place (the record is append-only, P1).
> - ADR-015 — A lantern's note and epilogues are the person's annotations, not transcript — A lantern's note and its epilogues are the person's own annotations, not transcript. Recall prints them by design; they are the one exception to P7 and CN3, and only recall prints them. Transcript lines (message, tool and compaction content) still never appear in output, logs or errors, and a recall row never carries a line of the transcript around its point. Rejected: treating notes as transcript and printing only their hashes, which makes recall useless to the person who wrote them.
> **Checklist:**
> - C24 — A lantern is a lys.lantern custom entry appended at the session's head, naming an existing entry of that session that is not a lantern or an epilogue as its point, with the note as written, lit-by and lit-at; an epilogue is a lys.lantern_epilogue custom entry naming the lantern's entry id with its words, author and time; both are documented beside the other lys custom entries.
> - C25 — Lighting only appends: the session's earlier bytes are unchanged and the head is the lantern; a point that is not an entry of the session, a lantern or epilogue as the point, an empty or whitespace-only note, and a session another owner holds are each refused by name with nothing written.
> - C26 — An epilogue is added to a named lantern of a session and appended at that session's head; a lantern the session does not hold and empty or whitespace-only words are each refused by name with nothing written.
> - C27 — Recall reads without owning: it takes no session lock and writes no file, and a session it cannot read is skipped and named with its reason while every other session is still listed.
> - C28 — Recall by note lists every lantern in the home whose note or one of whose epilogues contains the given words as one phrase, case-folded; recall by point lists every lantern marking a session and entry id, and refuses an unknown session or entry by name; every row is the lantern's id, session, point, lit-by, lit-at, note and its epilogues in order, and never a line of the transcript.
> - C29 — Three lys-home subcommands light a lantern, add an epilogue and recall, each printing one JSON report that carries no transcript line.
> - C30 — A lantern lives in the home: the rendered Claude Code file of a session carries no lantern or epilogue line, a home file holding them still parses with Pi's parser unchanged, and nothing lights a lantern automatically.
> **Stories:**
> - S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent at a moment of completion, success or learning, I want to light a lantern on a point of my session with a note, including a point I have already moved past, so that a later session, or a fork, can walk back to it.
> - S15 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent returning to a lantern, I want to add an epilogue to its note, so that its story grows without anything being rewritten.
> - S16 (Agent, Runs in a harness and wants to continue somewhere else) — As a later session, I want to recall lanterns by a word of their note or by the point they mark, and see each lantern with its epilogues and never the transcript around it, so that I find my way back without reading the session again.
> - S17 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a lantern kept in the home and out of every rendered resume file, with the home file still Pi's grammar, so that lighting one never changes what a harness resumes.

## Purpose

Stage 5 of the home: a lantern is a note plus a point in a session, declared on purpose, that a later session or a fork walks back to. It is stored in the session itself as a `lys.lantern` custom entry (ADR-014), its note grows only by `lys.lantern_epilogue` entries, and recall finds it by a phrase of its note or by its point without reading or printing the transcript around it. It stands on HOME-001's record and adds nothing to Pi's grammar. See docs/design/home/design.json for the shape, P1, P2, P7 and CN3, CN4 and CN7.

## Task

Extend lys-home with the two custom entries (R1), a read-only session reader and a listing of a home's sessions (R2), lighting (R3), epilogues (R4), recall by note and by point (R5), the `lantern` subcommands (R6) and the tests that keep a lantern in the home (R7), in that order. P7 and CN3 in docs/design/home/design.json are amended with this brief (ADR-015): transcript lines never appear in output, logs or errors, and a lantern's note and its epilogues are the one exception, printed only by recall. Recall matches the given words as one contiguous phrase, case-folded with Rust's `str::to_lowercase` on both sides, inside a single text (the note or one epilogue); no new dependency is added for case folding. Lit-by and an epilogue's author come from a required `--by` argument, a self-declared name as canon add's curator is (ADR-003). An epilogue is addressed by its session and the lantern's entry id together, because entry ids are unique only within a session, and is appended at the head of the lantern's own session. An epilogue's report carries the epilogue's entry id, the lantern's id, the session, who added it, when, and its ordinal among the lantern's epilogues counted from 1, never the words; the light report carries the lantern's id, session, point and time, never the note. Recall with empty or whitespace-only words is refused by name and lists nothing. Recall must not take the owner's lock, which is why R2 adds a reader that neither locks nor writes; lighting and epilogues append, so they take the lock as every append does. Out of scope: the fork through a lantern (stage 5b, its own card); resonance, whispers, vectors and any ranking; dimming by code churn; Norn's tables; Cambium's screen; lit-by from a directory identity; lighting from a live session under capture.

## Requirements

### R1: Define and document the lantern and epilogue custom entries

Structure: add the custom-type constants `CUSTOM_LANTERN = "lys.lantern"` and `CUSTOM_LANTERN_EPILOGUE = "lys.lantern_epilogue"` beside the existing lys custom types, and the two data shapes that ride in `custom.data`: a lantern's data is `{point, note, lit_by, lit_at}` (the point's entry id, the note as written, the self-declared name of who lit it, and when, RFC 3339 as the record's `now()` writes it), and an epilogue's data is `{lantern, words, added_by, added_at}` (the lantern's own entry id, the further words as written, who added them and when). Both serialise and deserialise with serde and reject an unknown field. Document both entries and their fields in the record's written contract and the crate README, beside the four existing lys custom types, and state there and in the README's list of what the crate does not do that a lantern's note and its epilogues are the one text the crate prints, and only recall prints them. THE SYSTEM SHALL NOT add a field to Pi's header or to any entry outside `custom.data`, SHALL NOT use Pi's `label` entry for a lantern, and SHALL NOT describe `lit_by` or `added_by` as a verified identity: each is a self-declared name until the directory issues identities.

**Acceptance:**
- `CUSTOM_LANTERN == "lys.lantern"` and `CUSTOM_LANTERN_EPILOGUE == "lys.lantern_epilogue"`.
- A lantern's data with point `e2`, note `The Fold Held Under Replay`, lit_by `fixture-lighter` and lit_at the string `now()` returned serialises to a JSON object whose keys are exactly `lit_at`, `lit_by`, `note`, `point`, and deserialising that object gives back an equal value.
- An epilogue's data serialises to a JSON object whose keys are exactly `added_at`, `added_by`, `lantern`, `words`.
- Deserialising lantern data carrying an extra key `anchors` is refused with an error.
- docs/design/home/RECORD.md has one bullet for `lys.lantern` naming the fields `point`, `note`, `lit_by`, `lit_at`, and one for `lys.lantern_epilogue` naming `lantern`, `words`, `added_by`, `added_at`.
- The custom-type table in crates/lys-home/README.md has a row for `lys.lantern` and a row for `lys.lantern_epilogue`, making six rows.

**Files:**
- modify: crates/lys-home/src/record/entries.rs
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/README.md

**Checklist:**
- C24 — A lantern is a lys.lantern custom entry appended at the session's head, naming an existing entry of that session that is not a lantern or an epilogue as its point, with the note as written, lit-by and lit-at; an epilogue is a lys.lantern_epilogue custom entry naming the lantern's entry id with its words, author and time; both are documented beside the other lys custom entries.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent at a moment of completion, success or learning, I want to light a lantern on a point of my session with a note, including a point I have already moved past, so that a later session, or a fork, can walk back to it.
- S15 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent returning to a lantern, I want to add an epilogue to its note, so that its story grows without anything being rewritten.

### R2: Read a session without owning it, and list a home's sessions

Add a read-only session reader. WHEN a session is read through it, THE SYSTEM SHALL use the index beside the file when it is valid for the file and otherwise build the index in memory by scanning the file, and SHALL read custom entries of a given type by seeking to the rows the index's custom column names (CN7). THE SYSTEM SHALL NOT take the session's lock, SHALL NOT create, write, rename or truncate any file (no index, head, lock or temporary file), and SHALL NOT refuse a session because another owner holds it. Add a listing of a home's session ids: every regular file directly under `sessions/` whose name ends `.jsonl`, except an index file, which is one whose name ends `.index.jsonl` and whose first line is not a session header; ids are returned in ascending byte order. IF a file under `sessions/` cannot be read while listing, THEN THE SYSTEM SHALL refuse by name naming the path. Split the index's rebuild from its write so the reader can rebuild without writing; the owner's `Index::load` keeps writing a rebuilt index exactly as it does today.

**Acceptance:**
- The fixture home used below holds session `fixture-lantern` created with `Home::create_session`, holding five message entries appended in order (called e1 to e5 here; the head is e5), where e3's message text is `FIXTURE-TRANSCRIPT-LINE-q7`. With a `Session` open on `fixture-lantern` (the lock held), the reader opens `fixture-lantern` and returns its five entries' ids in file order.
- With `fixture-lantern.index.jsonl` deleted, reading `fixture-lantern` through the reader returns the same custom entries an owner would, and afterwards the list of file names under `sessions/` and every file's bytes are identical to before the read (no index, head or lock file appears).
- With `fixture-lantern.lock` absent, a read through the reader leaves it absent.
- A home holding sessions `fixture-b`, `fixture-a` and `x.index` (each with its index, head and lock beside it) plus a file `notes.txt` lists exactly `["fixture-a", "fixture-b", "x.index"]`.
- After the reader has read a session, `Session::open` on it succeeds and appends as before (the one-owner lock is untouched).

**Files:**
- create: crates/lys-home/src/record/reader.rs
- create: crates/lys-home/src/record/reader_tests.rs
- modify: crates/lys-home/src/record/index.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C27 — Recall reads without owning: it takes no session lock and writes no file, and a session it cannot read is skipped and named with its reason while every other session is still listed.

**Stories:**
- S16 (Agent, Runs in a harness and wants to continue somewhere else) — As a later session, I want to recall lanterns by a word of their note or by the point they mark, and see each lantern with its epilogues and never the transcript around it, so that I find my way back without reading the session again.

### R3: Light a lantern at an entry of a session

WHEN a lantern is lit with a home, a session id, the entry id of the point, a note and a lighter's name, THE SYSTEM SHALL open the session as its one owner, check the point and the note, append one `lys.lantern` custom entry as a child of the head carrying the point, the note byte for byte as given, the lighter's name and the time, advance the head to it through the existing line, index row, head durability order, and return the lantern's entry id, session, point and time. The point may be any entry of the session, including one the head has moved past. IF the session id names no session file, THEN THE SYSTEM SHALL refuse with a new `UnknownSession` naming the session. IF the point is not an entry of the session, THEN THE SYSTEM SHALL refuse with `UnknownEntry` naming the session and the id. IF the point is a `lys.lantern` or `lys.lantern_epilogue` entry, THEN THE SYSTEM SHALL refuse with a new `PointIsLantern` naming the session and the id. IF the note is empty or only whitespace, THEN THE SYSTEM SHALL refuse with a new `EmptyNote` naming what was empty (`note`). IF another owner holds the session, THEN THE SYSTEM SHALL refuse with `SessionHeld`. On every refusal THE SYSTEM SHALL write nothing. THE SYSTEM SHALL NOT rewrite, truncate or reorder any earlier byte of the session file, SHALL NOT trim or otherwise alter the note it stores, SHALL NOT light a lantern from any act other than this one, and SHALL NOT put the note or any transcript text in an error.

**Acceptance:**
- The fixture home used below holds session `fixture-lantern` created with `Home::create_session`, holding five message entries appended in order (called e1 to e5 here; the head is e5), where e3's message text is `FIXTURE-TRANSCRIPT-LINE-q7`. Lighting at e2 with note `The Fold Held Under Replay` by `fixture-lighter` returns a lantern id L1; bytes 0..N of the session file are identical to the file before lighting (N its length before); the file has exactly one more line; `head()` equals L1; `entry(L1)` is a custom entry of type `lys.lantern` whose parentId is e5 and whose data has point `e2`, note `The Fold Held Under Replay` and lit_by `fixture-lighter`.
- A second lighting at e2 with note `second look` appends a second `lys.lantern` entry L2 whose point is e2, and L1 is still in the file unchanged.
- On a fresh copy of the fixture, a note `  kept as written  ` (two leading and two trailing spaces) is stored with exactly those bytes.
- Lighting at point `no-such-entry` is refused with `UnknownEntry { session: "fixture-lantern", id: "no-such-entry" }`, and the file length and `head()` are unchanged.
- Lighting with point L1 is refused with `PointIsLantern` naming L1; on a fresh copy of the fixture with a `lys.lantern_epilogue` custom entry appended through `Session::append`, lighting with that entry's id as the point is refused with `PointIsLantern` naming it; the file length is unchanged after each.
- Lighting with note `` and, separately, with note ` \n\t ` is refused with `EmptyNote` naming `note`; the file length is unchanged after each.
- With another `Session` open on `fixture-lantern`, lighting is refused with `SessionHeld` and the file length is unchanged.
- Lighting on session `no-such-session` is refused with `UnknownSession` naming `no-such-session`, and no file named `no-such-session.*` exists under `sessions/` afterwards.
- The display text of every refusal above contains neither `FIXTURE-TRANSCRIPT-LINE-q7` nor `The Fold Held Under Replay`.

**Files:**
- create: crates/lys-home/src/record/lantern.rs
- create: crates/lys-home/src/record/lantern_tests.rs
- modify: crates/lys-home/src/error.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C24 — A lantern is a lys.lantern custom entry appended at the session's head, naming an existing entry of that session that is not a lantern or an epilogue as its point, with the note as written, lit-by and lit-at; an epilogue is a lys.lantern_epilogue custom entry naming the lantern's entry id with its words, author and time; both are documented beside the other lys custom entries.
- C25 — Lighting only appends: the session's earlier bytes are unchanged and the head is the lantern; a point that is not an entry of the session, a lantern or epilogue as the point, an empty or whitespace-only note, and a session another owner holds are each refused by name with nothing written.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent at a moment of completion, success or learning, I want to light a lantern on a point of my session with a note, including a point I have already moved past, so that a later session, or a fork, can walk back to it.

### R4: Add an epilogue to a named lantern

WHEN an epilogue is added with a home, a session id, the lantern's entry id, the further words and an author's name, THE SYSTEM SHALL open that session as its one owner, check that the id names a `lys.lantern` entry of that session, append one `lys.lantern_epilogue` custom entry as a child of the session's head carrying the lantern's id, the words byte for byte as given, the author's name and the time, advance the head to it, and return the epilogue's entry id, the lantern's id, the session, the author's name, the time and the epilogue's ordinal: its position among the lantern's epilogues in file order, counted from 1. The return carries no words. IF the id is not a `lys.lantern` entry of that session (absent, a message, or an epilogue), THEN THE SYSTEM SHALL refuse with a new `UnknownLantern` naming the session and the id. IF the words are empty or only whitespace, THEN THE SYSTEM SHALL refuse with `EmptyNote` naming `epilogue`. IF the session names no session file, THEN THE SYSTEM SHALL refuse with `UnknownSession`; IF another owner holds it, THEN with `SessionHeld`. On every refusal THE SYSTEM SHALL write nothing. THE SYSTEM SHALL NOT modify the lantern entry or any earlier epilogue, SHALL NOT append the epilogue to any session but the lantern's own, and SHALL NOT put the words or any transcript text in an error.

**Acceptance:**
- On the R3 fixture after L1 and L2, adding to L1 the words `and the replay fold rings true` by `fixture-annotator` appends exactly one line; bytes 0..N are unchanged (N the length before); `head()` is the new entry E1, a custom entry of type `lys.lantern_epilogue` whose parentId is L2 and whose data has lantern L1, words `and the replay fold rings true` and added_by `fixture-annotator`.
- A second epilogue on L1 with words `a later word: cobalt` appends E2 after E1, and E1's line is unchanged.
- The report of E1 carries ordinal 1 and the report of E2 carries ordinal 2; neither report serialises to JSON containing `and the replay fold rings true` or `cobalt`.
- Naming `no-such-lantern`, then e2 (a message), then E1 (an epilogue) is each refused with `UnknownLantern` naming that id, and the file length is unchanged after each.
- Words `` and, separately, `   ` are refused with `EmptyNote` naming `epilogue`, and the file length is unchanged after each.
- With another `Session` open on `fixture-lantern`, adding an epilogue is refused with `SessionHeld` and the file length is unchanged.

**Files:**
- create: crates/lys-home/src/record/epilogue.rs
- create: crates/lys-home/src/record/epilogue_tests.rs
- modify: crates/lys-home/src/error.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C24 — A lantern is a lys.lantern custom entry appended at the session's head, naming an existing entry of that session that is not a lantern or an epilogue as its point, with the note as written, lit-by and lit-at; an epilogue is a lys.lantern_epilogue custom entry naming the lantern's entry id with its words, author and time; both are documented beside the other lys custom entries.
- C26 — An epilogue is added to a named lantern of a session and appended at that session's head; a lantern the session does not hold and empty or whitespace-only words are each refused by name with nothing written.

**Stories:**
- S15 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent returning to a lantern, I want to add an epilogue to its note, so that its story grows without anything being rewritten.

### R5: Recall lanterns by note and by point

Recall reads through the R2 reader only. WHEN recall by note is asked with some words, THE SYSTEM SHALL list every lantern in every session of the home for which the words, lowercased, occur as one contiguous substring of the lantern's note lowercased or of one of its epilogues' words lowercased. WHEN recall by point is asked with a session id and an entry id, THE SYSTEM SHALL list every lantern of that session whose point is that entry id, and an empty list only when the entry exists and no lantern marks it. Each row SHALL be the lantern's id, session, point, lit_by, lit_at, note and epilogues, the epilogues being every `lys.lantern_epilogue` entry of the same session naming the lantern, each as its added_by, added_at and words, in file order. Rows SHALL be ordered by session id ascending, then by the lantern's position in its file. The report SHALL carry the rows and a list of skipped sessions. IF a session cannot be read during recall by note, THEN THE SYSTEM SHALL skip it, name it in the skipped list with the reason (the error's display text), and still list every other session. IF the words of a recall by note are empty or only whitespace, THEN THE SYSTEM SHALL refuse with `EmptyNote` naming `words` and list nothing. IF recall by point names a session with no session file, THEN THE SYSTEM SHALL refuse with `UnknownSession`; IF the entry id is not an entry of that session, THEN with `UnknownEntry` naming the session and the id. THE SYSTEM SHALL NOT match a phrase split between the note and an epilogue or between two epilogues, SHALL NOT stem, rank, score or use vectors, SHALL NOT take a lock or write any file, SHALL NOT refuse the whole recall because one session could not be read, SHALL NOT return a partial list without naming what it skipped, and SHALL NOT carry in a row any line of the transcript: no message, tool or compaction content, and nothing from the entries around the point.

**Acceptance:**
- The recall fixture is the R4 fixture (L1 and L2 at e2 in `fixture-lantern`, epilogues E1 then E2 on L1) plus session `fixture-other` holding one message entry o1 and lantern L3 at o1 with note `nothing shared but the fold held`.
- Recall by note `fold` lists exactly [L1, L3], in that order, with an empty skipped list.
- Recall by note `FOLD HELD` lists exactly [L1, L3].
- Recall by note `cobalt`, a word only in E2, lists exactly [L1], and L1's row carries epilogues [E1's, E2's] in that order with words `and the replay fold rings true` then `a later word: cobalt`.
- Recall by note `under replay and`, which runs from L1's note into E1, lists no lantern; recall by note `rings true a later`, which runs from E1 into E2, lists no lantern.
- Recall by note `` and, separately, by note ` \t ` is each refused with `EmptyNote` naming `words`.
- Recall by point (`fixture-lantern`, e2) lists exactly [L1, L2]; recall by point (`fixture-lantern`, e4) lists no lantern and is not refused.
- Recall by point (`no-such-session`, e2) is refused with `UnknownSession` naming `no-such-session`; recall by point (`fixture-lantern`, `no-such-entry`) is refused with `UnknownEntry` naming `no-such-entry`.
- Every row serialises to a JSON object whose keys are exactly `epilogues`, `id`, `lit_at`, `lit_by`, `note`, `point`, `session`, and every epilogue in it to an object whose keys are exactly `added_at`, `added_by`, `words`.
- With a `Session` open on `fixture-lantern`, recall by note `fold` still lists exactly [L1, L3] with an empty skipped list.
- With a session `fixture-broken` whose first line is `not json`, recall by note `fold` lists exactly [L1, L3] and the skipped list has exactly one item, whose session is `fixture-broken`.
- After every recall above, the file names under `sessions/` and every file's bytes are identical to before it.
- The serialised report of every recall above contains no occurrence of `FIXTURE-TRANSCRIPT-LINE-q7`.

**Files:**
- create: crates/lys-home/src/record/recall.rs
- create: crates/lys-home/src/record/recall_tests.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C27 — Recall reads without owning: it takes no session lock and writes no file, and a session it cannot read is skipped and named with its reason while every other session is still listed.
- C28 — Recall by note lists every lantern in the home whose note or one of whose epilogues contains the given words as one phrase, case-folded; recall by point lists every lantern marking a session and entry id, and refuses an unknown session or entry by name; every row is the lantern's id, session, point, lit-by, lit-at, note and its epilogues in order, and never a line of the transcript.

**Stories:**
- S16 (Agent, Runs in a harness and wants to continue somewhere else) — As a later session, I want to recall lanterns by a word of their note or by the point they mark, and see each lantern with its epilogues and never the transcript around it, so that I find my way back without reading the session again.

### R6: Add the lantern subcommands to lys-home

Add a nested `lantern` subcommand to lys-home with three actions, in their own module so cli.rs stays under the 500-line limit. `lantern light --home <dir> --session <id> --point <entry id> --note <text> --by <name>` lights through R3 and prints one JSON object whose keys are exactly `id`, `session`, `point` and `lit_at`. `lantern epilogue --home <dir> --session <id> --lantern <entry id> --words <text> --by <name>` adds through R4 and prints one JSON object whose keys are exactly `added_at`, `added_by`, `id` (the epilogue's entry id), `lantern`, `ordinal` and `session`, never the words. `lantern recall --home <dir>` with exactly one of `--note <words>` or the pair `--session <id> --point <entry id>` recalls through R5 and prints one JSON object whose keys are exactly `lanterns` (the rows) and `skipped` (each an object with keys exactly `session` and `reason`). WHEN an action succeeds, THE SYSTEM SHALL exit 0; IF it is refused, THEN THE SYSTEM SHALL print the refusal's display text on stderr, print nothing on stdout and exit 1. THE SYSTEM SHALL NOT print a transcript line on stdout or stderr, SHALL NOT change the arguments or report shape of import, render, canon, fewshot, ingest-call or resume-check, and SHALL NOT take `--by` from the environment: it is a required argument, as canon add's is.

**Acceptance:**
- On the R3 fixture, `lys-home lantern light --home <home> --session fixture-lantern --point <e2> --note "The Fold Held Under Replay" --by fixture-lighter` exits 0 and prints one JSON object with keys exactly `id`, `lit_at`, `point`, `session`, where `session` is `fixture-lantern`, `point` is e2 and `id` equals the session's head afterwards.
- The same command with `--point no-such-entry` exits 1, prints nothing on stdout, and its stderr contains `no-such-entry`.
- The same command with `--note ""` exits 1 and prints nothing on stdout.
- The light command's printed line does not contain `The Fold Held Under Replay`.
- `lys-home lantern epilogue --home <home> --session fixture-lantern --lantern <L1> --words "a later word: cobalt" --by fixture-annotator` exits 0, prints one JSON object with keys exactly `added_at`, `added_by`, `id`, `lantern`, `ordinal`, `session`, where `lantern` is L1, `ordinal` is 1 and `id` equals the session's head afterwards, which is a `lys.lantern_epilogue` entry whose data's lantern is L1; the printed line does not contain `cobalt`.
- `lys-home lantern recall --home <home> --note cobalt` exits 0 and prints one JSON object with keys exactly `lanterns`, `skipped`, whose `lanterns` holds one row with `id` L1.
- `lys-home lantern recall --home <home> --session fixture-lantern --point <e2>` exits 0 and its `lanterns` holds the rows for every lantern lit at e2.
- `lys-home lantern recall --home <home> --note ""` exits 1, prints nothing on stdout, and its stderr names `words`.
- `lys-home lantern recall --home <home> --note fold --session fixture-lantern --point <e2>` is refused by clap with exit code 2.
- `lys-home lantern light` without `--by` is refused by clap with exit code 2.
- Across every command run above, stdout and stderr together contain no occurrence of `FIXTURE-TRANSCRIPT-LINE-q7`.

**Files:**
- create: crates/lys-home/src/cli_lantern.rs
- create: crates/lys-home/tests/lantern_cli.rs
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C29 — Three lys-home subcommands light a lantern, add an epilogue and recall, each printing one JSON report that carries no transcript line.

**Stories:**
- S14 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent at a moment of completion, success or learning, I want to light a lantern on a point of my session with a note, including a point I have already moved past, so that a later session, or a fork, can walk back to it.
- S15 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent returning to a lantern, I want to add an epilogue to its note, so that its story grows without anything being rewritten.
- S16 (Agent, Runs in a harness and wants to continue somewhere else) — As a later session, I want to recall lanterns by a word of their note or by the point they mark, and see each lantern with its epilogues and never the transcript around it, so that I find my way back without reading the session again.

### R7: Keep the lantern in the home

WHILE a session holds lanterns and epilogues, WHEN it is rendered for Claude Code, THE SYSTEM SHALL render no record for a `lys.lantern` or `lys.lantern_epilogue` entry: custom entries already do not render, and this requirement pins that with a test rather than changing the renderer. A session file holding lanterns and epilogues SHALL parse with Pi's `parseSessionEntries` at 3d5cbe98 unchanged; the command and its output are written in docs/design/home/PROOF-LANTERN.md, the dev record of this brief. THE SYSTEM SHALL NOT change the renderer, SHALL NOT write a lantern or an epilogue into any rendered resume file, and SHALL NOT append a `lys.lantern` or `lys.lantern_epilogue` entry from import, render, canon, fewshot, ingest-call, resume-check or any code path other than R3's lighting and R4's epilogue.

**Acceptance:**
- On the R4 fixture, rendering `fixture-lantern` for Claude Code to a fresh path produces a file in which 0 lines contain `lys.lantern`, 0 lines contain `The Fold Held Under Replay`, and 0 lines contain `cobalt`.
- The rendered file of the R4 fixture has the same number of lines as the rendered file of a copy of `fixture-lantern` taken before L1 was lit, rendered to another fresh path.
- `parseSessionEntries` from a clone of github.com/earendil-works/pi at 3d5cbe98, run through node on the R4 fixture's session file, returns the header and 9 entries, of which exactly 2 have customType `lys.lantern` and exactly 2 have customType `lys.lantern_epilogue`; the command and its output are written in docs/design/home/PROOF-LANTERN.md.
- `grep -rln 'CUSTOM_LANTERN' crates/lys-home/src` lists only files among record/entries.rs, record/lantern.rs, record/epilogue.rs, record/recall.rs and their sibling `_tests.rs` files.

**Files:**
- create: crates/lys-home/tests/lantern_home.rs
- create: docs/design/home/PROOF-LANTERN.md

**Checklist:**
- C30 — A lantern lives in the home: the rendered Claude Code file of a session carries no lantern or epilogue line, a home file holding them still parses with Pi's parser unchanged, and nothing lights a lantern automatically.

**Stories:**
- S17 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a lantern kept in the home and out of every rendered resume file, with the home file still Pi's grammar, so that lighting one never changes what a harness resumes.

## Boundaries

- SHALL NOT fork, cut or seed a session at a lantern's point; that is stage 5b.
- SHALL NOT add resonance, whispers, vectors, embeddings, stemming, scoring, ranking or dimming by code churn to recall.
- SHALL NOT use, call or copy any Norn crate, type or table, and SHALL NOT build any Cambium screen.
- SHALL NOT add a field to Pi's header or to any entry outside custom.data, and SHALL NOT use Pi's label entry for a lantern.
- SHALL NOT rewrite, truncate, reorder or delete any byte of a session file, index, head, lock or block; lighting and epilogues only append.
- SHALL NOT light a lantern or add an epilogue automatically or from any act other than R3 and R4.
- SHALL NOT render a lantern or an epilogue into a Claude Code resume file, and SHALL NOT change the renderer.
- SHALL NOT print, log or put in an error or a test name any transcript line (message, tool or compaction content); a lantern's note and epilogues are printed only by recall.
- SHALL NOT take a session lock or write any file during recall.
- SHALL NOT change the arguments or report shape of import, render, canon, fewshot, ingest-call or resume-check.
- SHALL NOT weaken the one-owner lock, the line, index row, head durability order, or the safe-component name rule.
- SHALL NOT present lit_by or added_by as a verified identity.
- SHALL NOT add a dependency to lys-home.

## Verification

- cargo fmt --all -- --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo clippy --all-targets -- -D warnings
- cargo test --workspace --all-features, and confirm the tests of R2 to R7 ran by name (reader_tests, lantern_tests, epilogue_tests, recall_tests, lantern_cli, lantern_home) with a non-zero count each
- cargo doc --no-deps --all-features and cargo doc --no-deps, both with zero warnings
- sh scripts/design/gate.sh exits 0
- Every non-test file under crates/lys-home/src has at most 500 lines of code, cli.rs included (the repository's rule excludes test files, comments and whitespace), where a line of code is a non-blank line that is not a comment, counted per file as `grep -vcE '^\s*$|^\s*//' <file>`; the counts, test files included, are written in docs/design/home/PROOF-LANTERN.md
- git diff of crates/lys-home/Cargo.toml adds no dependency
- git diff of crates/lys-home/src/harness/claude_code/render.rs is empty
- The node run of R7's third acceptance line, against a clone of earendil-works/pi at 3d5cbe98, is written in docs/design/home/PROOF-LANTERN.md with its output


---
type: brief
id: HOME-006
cluster: home
title: Fork a session from a lantern's point, write its ancestry on both sides, and render and launch the child
---

# HOME-006: Fork a session from a lantern's point, write its ancestry on both sides, and render and launch the child

> **Cluster:** home
> **Depends on:** HOME-001, HOME-002, HOME-004
> **Blocked by:** The lantern card 'Light a lantern and recall it' (board card YUTHf4z0, brief 7657952c) must land first: it delivers the `lys.lantern` custom entry (data `point`, `note`, `lit_by` and `lit_in`, lit at the head of the session holding the point, its `lit_at` the entry's timestamp as recall reports it), the home's session listing and lock-free session view (`record/view.rs`), and `lys-home lantern light`. This brief is built on the main that lantern card lands on, and is refreshed onto it (its ids renumbered past the lantern card's) before the build starts., The fork reads the session a lantern was lit in (its lit-in session) from the `lit_in` key of the lantern card's `lys.lantern` data, beside `point`, `note` and `lit_by`; the lantern card is named here for that `lit_in` read only. Until the lantern card lands recording `lit_in`, every lantern resolves by R2's older-record rule: a `lys.lantern` entry whose data carries no `lit_in` is an older record., The lantern card's `lit_in` round must add the field to `LanternData` in `record/entries.rs`, which refuses an unknown field: until it lands, a `lys.lantern` entry carrying `lit_in` is one the light act does not write and recall refuses by shape, so the fixtures here write `L2` by hand with `lit_in` and the fork reads the key from the entry's raw data.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-014 — A lantern is a custom entry in its session, and its note grows only by epilogue entries — A lantern is a `lys.lantern` custom entry appended at its session's head, carrying in custom.data the entry id of its point (an existing entry of the same session that is not itself a lantern or an epilogue, the head or any entry the head has moved past), the note as written, who lit it and when. Its note grows only by `lys.lantern_epilogue` custom entries naming the lantern's entry id and carrying the further words, who added them and when; a lantern's story is its entry followed by its epilogues in order, and nothing is rewritten. Rejected: Pi's `label` entry on the target (it replaces or clears a label rather than growing one, and carries no author or time), a lantern store beside the session outside Pi's grammar (a lantern would stop travelling with its session), and editing the lantern's note in place (the record is append-only, P1).
> - ADR-017 — A fork is a child session cut from the parent's own lines at a lantern's point, with its ancestry on both sides — A fork resolves a lantern to the session it was lit in, read from the lys.lantern data's lit_in when the record carries it and otherwise by the older-record rule (one holder cuts, several refuse lantern_ambiguous until a session is named), and cuts that session's root-to-point chain at the last assistant message at or before the point, through the index. The child is a new session under the parent's cwd whose header's parentSession is the parent file's path relative to the home, holding each cut entry as the parent file's own line bytes, then one lys.forked_from custom entry as its head naming the parent session, the lantern, the point, the cut entry, whether the coordinate was carried and the carried entry; the parent gains one lys.fork custom entry at its head naming the child. Nothing else is copied and no block is written. Rejected: re-serialising the copied entries (the copy would stop hash-matching the parent's lines), a fork store beside the sessions outside Pi's grammar, cutting at a point no lantern names, and a header field beyond Pi's parentSession.
> - ADR-018 — A user-message point is carried as a seed prompt beside the rendered file, never copied into the child — When the point is a user message the cut stops at the assistant message before it and the message is carried, not copied: lys.forked_from records its id with coordinate_carried true and counts, by kind, the parts of it that are not text. The Claude Code render of such a child writes the message's text parts, in order, as a seed prompt beside the rendered file under an in-band marker line naming the parent session, the point and the lantern, and names it in the render report; the template's launch line, printed by render-launch only, passes that file as the resumed session's first prompt. A part that is not text never refuses a fork or a render and never enters the seed. Rejected: copying the user message into the child's chain, refusing a fork for a non-text part, and putting the seed's text in the report or the loss account.
> **Checklist:**
> - C36 — The two fork custom types, lys.forked_from (parent_session, lantern, point, cut_at, coordinate_carried, carried, seed_left_out) and lys.fork (child), are named beside the other lys custom entries and documented in RECORD.md, and neither adds a field to Pi's header or outside custom.data.
> - C37 — A fork refuses by name before any file is created: a lantern id no session holds as a lys.lantern entry, an older-record lantern several sessions hold with no session named (lantern_ambiguous, listing them in ascending byte order), a named session that is not the lantern's lit-in session (lantern_not_lit_here), a lantern that sits before any assistant message (nothing_to_fork), and a parent another owner holds; no refusal carries a note, a part's text or any entry's data.
> - C38 — A fork resolves a lantern to the session it was lit in, read from its data's lit_in or by the older-record rule when the data carries none, and cuts that session's root-to-point chain, read through the index and never by loading the file, at the last assistant message at or before the point, in file order with every side leaf left out and no entry after the cut.
> - C39 — The child session holds each cut entry as the parent file's own line bytes, under the parent's cwd, with the header's parentSession the parent file's path relative to the home; nothing is re-serialised.
> - C40 — Ancestry is written on both sides: lys.forked_from is the first entry the fork writes in the child, directly after the copy, and its head; lys.fork is appended at the parent's head naming the child; a user-message point is not copied but carried, its id recorded with coordinate_carried true and its parts that are not text counted by kind.
> - C41 — The fork report carries the child, the parent, the lantern, the point, the cut entry, the count of entries copied, the distinct block hashes the copied entries name that the store holds and those it does not, whether the coordinate was carried and the carried entry; never a part's text, a note or any entry's data.
> - C42 — A fork writes no block, rewrites no earlier byte of the parent, and copies nothing from the parent but its cwd: no credential, handle or launch setting enters the child.
> - C43 — lys-home fork --home --lantern [--session] prints one JSON report on success, exits 1 with the refusal on stderr and nothing on stdout, and takes no point or entry id.
> - C44 — Rendering a child whose coordinate was carried writes the carried message's text parts as a seed prompt beside the rendered file under an in-band marker line, the render report names it, the template's launch line (printed by render-launch only) passes it as the first prompt and is never run, and PROOF-FORK.md records a fork launched on the installed Claude Code version as hashes, counts, paths, commands, versions and exit codes only.
> **Stories:**
> - S20 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern's point, carrying everything said up to that point and nothing after it, and launch the child like any session, so that I can go back and talk with the self that lit the lantern.
> - S21 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a fork at a user message to carry that message as the child's first prompt beside the rendered file, with nothing from the parent's header but its working directory, so that the child starts at the coordinate and no credential, handle or launch setting is copied from the parent.
> - S22 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the ancestry written on both sides, the child's header naming the parent file and a lys.forked_from entry naming the lantern, the point and the cut, and a lys.fork entry at the parent's head naming the child, so that a stranger can tell a fork from its parent from the record alone.
> - S23 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a fork's report to carry ids and counts only, with the parent's earlier bytes and the block store unchanged and the whole thing proved through the binary, so that a fork never quietly copies or rewrites anything.

## Purpose

A lantern is a pathway back to a conversation with the self that lit it, and Claude Code resumes a session only from its end. This brief delivers the fork that makes the pathway: a new session in the home cut from the parent's chain at a lantern's point, carrying its ancestry on both sides, adding nothing to the block store and rewriting nothing in the parent, then rendered and launched like any session. It is stage 5b of the home and stands on the lantern card's lys.lantern entry (the design's solution and ADR-012).

## Task

Add `lys-home fork --home <dir> --lantern <id> [--session <id>]`. It resolves the lantern id to the session the lantern was lit in, read from its `lys.lantern` data's `lit_in` (a copy of a lantern's line inside a fork's child is a copy, not a second lantern), cuts that session's raw root-to-point chain (read through the index) back to the last assistant message at or before the lantern's point, creates a child session whose header's `parentSession` is the parent file's path relative to the home, copies the chain's lines byte for byte, appends `lys.forked_from` to the child directly after the copy and `lys.fork` at the parent's head, and prints one JSON report of ids and counts. When the point is a user message, that message is not copied: the cut stops at the assistant message before it, `lys.forked_from` records the message's id with `coordinate_carried` true, and the child's render writes its text parts, in order, as a seed prompt beside the rendered file under an in-band marker line, while `lys.forked_from` counts the parts left out by kind; the render names the seed file in its report and prints no launch line; the launch line comes from the template's render only, with the seed as the first prompt when there is one. Nothing is copied from the parent: the child launches on whatever the launch supplies. The launch template (HOME-002, on main) is what supplies it: `render-launch` prints the template's line with the seed as the first prompt, so the child launches with the template's handles and not the bare seat login, and the proof says which line it ran. In scope: the fork, its report, the seed on render and the seed on the template's launch line, RECORD.md, PROOF-FORK.md and the cluster's rendered markdown. A lantern that sits before any assistant message is refused by name (`nothing_to_fork`): a point before the first reply would carry only the seed, which is a new session and not a fork. Out of scope: lighting, holding or recalling lanterns (the lantern card); cutting at a point without a lantern; forks with tools or a lens; leases, budgets and kill rules; the launch template and handles; harnesses other than Claude Code. Before the header value is written, read how Pi reads `parentSession` in `packages/coding-agent/src/core/session-manager.ts` at 3d5cbe98 and record the lines in PROOF-FORK.md; if Pi there reads it as anything other than a session file path, stop and name it to the lead rather than choose a form. The fixture home used below holds one session `parent`, built with `Session::append_entry` under fixed ids, in this order: `e1` user message with one text part `fixture-text-1`; `e2` assistant message with one text part `fixture-text-2`; `s2` a `lys.harness_event` of kind `permission_mode` whose parent is `e2` (a side leaf, off the chain), its `record` naming a block of its own; then the head moved back to `e2` with `Session::move_head` and the session closed; then the lantern `L2` at point `e2`, a `lys.lantern` entry written with `Session::append_entry` as a child of `e2` whose data carries `lit_in` `parent` beside `point`, `note`, `lit_by` and `lit_at` (written by hand until the lantern card's `lit_in` round lands, since the light act on main records none); then, reopened, the older-record lantern `O2`, a `lys.lantern` entry written with `Session::append_entry` as the child of `L2`, whose data is exactly `point` `e2`, `note` and `lit_by` and carries no `lit_in`; `e3` a `lys.harness_event` of kind `attachment` whose parent is `O2`, its `record` naming another block; `e4` user `fixture-text-4`; `e5` assistant `fixture-text-5`; `e6` user with one text part `fixture-text-6` followed by one image part; `e7` assistant `fixture-text-7`; `e8` user `fixture-text-8`; `e9` assistant `fixture-text-9`, each the child of the one before. Every text part of `e1` to `e9` is put through `BlockStore::put` as `serde_json::to_vec` of the part as it stands in the entry, as the importer stores a text part, and so are the `record` blocks of `s2` and `e3`; `e6`'s image part is not put. With the session closed, three more lanterns are then lit with the light act at the session's head: `L5` with point `e5`, `L6` with point `e6`, and `L1` with point `e1`. A second session `compacted` holds `e1` user, `e2` assistant, `c3` a compaction with `firstKeptEntryId` `e2`, `e4` user, `e5` assistant, in one chain, with a lantern `C5` lit at point `e5`. `L1`, `L2`, `L5`, `L6` and `C5` stand for the entry ids the light act returns. The eight `fixture-text-*` strings (`fixture-text-1`, `-2`, `-4`, `-5`, `-6`, `-7`, `-8`, `-9`) are the content sentinels: none may appear in a report, an error, a log line or a test name.

## Requirements

### R1: Name the two fork custom types and the fork's refusals

Structural. `record/entries.rs` declares `CUSTOM_FORK` = `lys.fork` and `CUSTOM_FORKED_FROM` = `lys.forked_from` beside the lys custom types already there. `error.rs` gains four named refusals: `HomeError::NoSuchLantern { lantern }`, whose message names the lantern id (HOME-004 already owns `HomeError::UnknownLantern { session, id }`, which the fork keeps for a named session that does not hold the lantern); `HomeError::LanternAmbiguous { lantern, sessions }`, whose message is prefixed `lantern_ambiguous` and names the lantern id and every session holding it; `HomeError::LanternNotLitHere { lantern, session, lit_in }`, whose message is prefixed `lantern_not_lit_here` and names the lantern id, the session asked for and the lit-in session; and `HomeError::NothingToFork { lantern }`, whose message is prefixed `nothing_to_fork` and says the lantern sits before any assistant message. THE SYSTEM SHALL NOT add a Pi entry type, a header field, or any field outside a custom entry's `data`, and SHALL NOT carry transcript content, a note, or any entry's data in a refusal.

**Acceptance:**
- `CUSTOM_FORK == "lys.fork"` and `CUSTOM_FORKED_FROM == "lys.forked_from"`.
- `HomeError::NoSuchLantern { lantern: "no-such-lantern".into() }.to_string()` contains `no-such-lantern`.
- `HomeError::LanternAmbiguous { lantern: "x".into(), sessions: vec!["a".into(), "b".into()] }.to_string()` begins with `lantern_ambiguous` and contains `x`, `a` and `b`.
- `HomeError::LanternNotLitHere { lantern: "x".into(), session: "a".into(), lit_in: "b".into() }.to_string()` begins with `lantern_not_lit_here` and contains `x`, `a` and `b`.
- `HomeError::NothingToFork { lantern: "x".into() }.to_string()` begins with `nothing_to_fork` and contains `x` and `before any assistant message`.
- `git diff` of `record/entries.rs` adds no field to `SessionHeader`, `EntryBase` or any `EntryBody` variant.

**Files:**
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C36 — The two fork custom types, lys.forked_from (parent_session, lantern, point, cut_at, coordinate_carried, carried, seed_left_out) and lys.fork (child), are named beside the other lys custom entries and documented in RECORD.md, and neither adds a field to Pi's header or outside custom.data.
- C37 — A fork refuses by name before any file is created: a lantern id no session holds as a lys.lantern entry, an older-record lantern several sessions hold with no session named (lantern_ambiguous, listing them in ascending byte order), a named session that is not the lantern's lit-in session (lantern_not_lit_here), a lantern that sits before any assistant message (nothing_to_fork), and a parent another owner holds; no refusal carries a note, a part's text or any entry's data.

**Stories:**
- S22 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the ancestry written on both sides, the child's header naming the parent file and a lys.forked_from entry naming the lantern, the point and the cut, and a lys.fork entry at the parent's head naming the child, so that a stranger can tell a fork from its parent from the record alone.

### R2: Resolve a lantern to the session it was lit in and cut that session's chain at the last assistant message

WHEN a fork is asked for with a home, a lantern id and, optionally, a session id, THE SYSTEM SHALL read every session the home lists through a lock-free view and find each session whose index holds a `lys.lantern` row whose entry id equals the lantern id. IF no session holds a `lys.lantern` entry with that id (including an id that names an entry of another kind), THEN THE SYSTEM SHALL refuse with `HomeError::NoSuchLantern` naming the id. WHERE the lantern entry's data carries `lit_in` (the lit-in session's id), THE SYSTEM SHALL cut from the lit-in session and no other, and IF a session id is given that is not the lit-in session, THEN THE SYSTEM SHALL refuse with `HomeError::LanternNotLitHere` naming the lantern, the given session and the lit-in session. WHERE the lantern entry's data carries no `lit_in` (an older record), THE SYSTEM SHALL cut from the one session holding it when exactly one does and no session id is given; IF more than one session holds it and no session id is given, THEN THE SYSTEM SHALL refuse with `HomeError::LanternAmbiguous` naming the lantern and every session holding it, in ascending byte order; WHEN a session id is given, THE SYSTEM SHALL cut from that session, and IF that session does not hold the lantern, THEN THE SYSTEM SHALL refuse with `HomeError::UnknownLantern`. WHEN the cut is taken, THE SYSTEM SHALL read the chosen session's lantern entry by its index row, take the lantern's `point` (and `lit_in`) from it, and read the point's ancestry from the root to the point through the index (`Index::ancestry` and `read_rows_from`), reading the point's entry once, as the last row of that ancestry, and SHALL end the cut at the last entry on that chain, at or before the point, that is a `message` entry whose `message.role` is `assistant`; the cut is the chain from the root to that entry in chain order, with any compaction entry and any custom entry at its own place. IF no entry on that chain at or before the point is an assistant message, THEN THE SYSTEM SHALL refuse with `HomeError::NothingToFork` naming the lantern. WHERE the point is a `message` entry whose role is `user`, THE SYSTEM SHALL record the point as the carried entry with `coordinate_carried` true; otherwise `coordinate_carried` is false and no entry is carried. Every refusal comes before any file is created or written. THE SYSTEM SHALL NOT include any entry after the cut entry, SHALL NOT include an entry off the chain (a side leaf), SHALL NOT reorder the chain as `context_path()` does, SHALL NOT read the point's entry a second time apart from its ancestry, SHALL NOT load a whole session file, SHALL NOT take a lock or write anything while resolving and cutting, SHALL NOT treat a copy of a lantern's line in another session as the lantern when `lit_in` is recorded, SHALL NOT pick one of several holders of an older-record lantern by itself, and SHALL NOT accept a point that is not a lantern's.

**Acceptance:**
- On the fixture, the cut for `L5` is the ids `[e1, e2, L2, O2, e3, e4, e5]` in that order, the cut entry is `e5`, `coordinate_carried` is false and no entry is carried.
- On the fixture, the cut for `L6` is the ids `[e1, e2, L2, O2, e3, e4, e5]`, the cut entry is `e5`, the carried entry is `e6` and `coordinate_carried` is true.
- On the fixture, the cut for `L2` is the ids `[e1, e2]` and the cut entry is `e2`.
- No cut on the fixture contains `s2`.
- On the `compacted` session, the cut for `C5` is the ids `[e1, e2, c3, e4, e5]` in that order.
- The bytes of `parent` the `L5` resolve and cut read equal the sum of the index row lengths of `L5` (the lantern entry, read for its point) and of `e1`, `e2`, `L2`, `O2`, `e3`, `e4` and `e5` (the point `e5` counted once, as the last row of its ancestry), and are fewer than the file's length.
- With a session `A` created in the test's fixture home by hand, first holding copies of `e1` and `e2` under their own ids, as a fork's copy holds the ancestry, then an entry with the id and data of `L2` whose parent is `A`'s copy of `e2`, then an entry with the id and data of `O2` whose parent is `A`'s `L2`, all four written with `Session::append_entry`, which accepts each of them: resolving `L2` cuts from `parent`; resolving `L2` with session `A` returns `HomeError::LanternNotLitHere` naming `L2`, `A` and `parent`; resolving `O2` returns `HomeError::LanternAmbiguous` naming `O2` with the sessions `A` and `parent` in ascending byte order; resolving `O2` with session `parent` cuts `[e1, e2]` from `parent`.
- Resolving `no-such-lantern` and resolving `e5` each return `HomeError::NoSuchLantern` naming that id, and resolving `O2` with session `compacted` (which does not hold it) returns `HomeError::UnknownLantern` naming `compacted` and `O2`; resolving `L1` returns `HomeError::NothingToFork` naming `L1`; the test asserts 6 refusals from 6 refusal cases, and the `sessions` directory's file count and the SHA-256 of every session file are unchanged after them.

**Files:**
- create: crates/lys-home/src/record/fork_cut.rs
- create: crates/lys-home/src/record/fork_cut_tests.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C37 — A fork refuses by name before any file is created: a lantern id no session holds as a lys.lantern entry, an older-record lantern several sessions hold with no session named (lantern_ambiguous, listing them in ascending byte order), a named session that is not the lantern's lit-in session (lantern_not_lit_here), a lantern that sits before any assistant message (nothing_to_fork), and a parent another owner holds; no refusal carries a note, a part's text or any entry's data.
- C38 — A fork resolves a lantern to the session it was lit in, read from its data's lit_in or by the older-record rule when the data carries none, and cuts that session's root-to-point chain, read through the index and never by loading the file, at the last assistant message at or before the point, in file order with every side leaf left out and no entry after the cut.

**Stories:**
- S20 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern's point, carrying everything said up to that point and nothing after it, and launch the child like any session, so that I can go back and talk with the self that lit the lantern.
- S21 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a fork at a user message to carry that message as the child's first prompt beside the rendered file, with nothing from the parent's header but its working directory, so that the child starts at the coordinate and no credential, handle or launch setting is copied from the parent.

### R3: Write the child from the parent's own lines and the ancestry on both sides

WHEN a fork is made from a cut, THE SYSTEM SHALL first open the parent session as its owner, then create a child session with a fresh id, the parent header's `cwd`, and `parentSession` equal to the parent session file's path relative to the home root (`sessions/<parent id>.jsonl`). THE SYSTEM SHALL append each entry of the cut to the child as the exact bytes of that entry's line in the parent file, read by its index row's offset and length, updating the child's index and head as any append does. THE SYSTEM SHALL then append to the child one `custom` entry of `customType` `lys.forked_from` whose parent is the cut entry, with data exactly `parent_session` (the parent's session id), `lantern` (the lantern id), `point` (the lantern's point), `cut_at` (the cut entry's id), `coordinate_carried` (boolean), `carried` (the carried entry's id, or null) and `seed_left_out` (an object counting, by each part's `type`, the parts of the carried message that are not `text` parts; `{}` when nothing is carried, when the carried message's content is a string, or when every part is text), and the child's head moves to it. THE SYSTEM SHALL read the carried entry by its index row to count its parts, and SHALL NOT refuse a fork for a part that is not text. THE SYSTEM SHALL then append to the parent, as the child of the parent's head, one `custom` entry of `customType` `lys.fork` with data exactly `child` (the child's session id), and the parent's head moves to it. IF another owner holds the parent, THEN THE SYSTEM SHALL refuse with `HomeError::SessionHeld` and create no child. THE SYSTEM SHALL NOT re-serialise a copied entry, SHALL NOT rewrite, truncate or reorder any earlier byte of the parent, SHALL NOT write any block to the block store, SHALL NOT copy the carried user message into the child, and SHALL NOT copy anything else from the parent into the child: no header field but `cwd`, no credential, no handle and no launch setting.

**Acceptance:**
- Forking `L5`: the child's entry lines 1 to 7 (after the header) each have the same SHA-256 as the parent's lines for `e1`, `e2`, `L2`, `O2`, `e3`, `e4`, `e5` respectively, and the child holds exactly 8 entries.
- Forking `L5`: the child's 8th entry is `custom` with `customType` `lys.forked_from`, `parentId` `e5`, and data equal to `{"parent_session": "parent", "lantern": <L5's id>, "point": "e5", "cut_at": "e5", "coordinate_carried": false, "carried": null, "seed_left_out": {}}`; the child's head is that entry's id.
- Forking `L5`: the child header's `parentSession` is `sessions/parent.jsonl`, the child's id differs from `parent`, and the child's header `cwd` equals the parent's.
- Forking `L6`: the `lys.forked_from` data is `{"parent_session": "parent", "lantern": <L6's id>, "point": "e6", "cut_at": "e5", "coordinate_carried": true, "carried": "e6", "seed_left_out": {"image": 1}}` and no child entry has id `e6`.
- With N the parent file's length before a fork: after it, the SHA-256 of the parent's bytes 0..N equals the SHA-256 of the whole file before, exactly one line follows byte N, that line is `custom` `lys.fork` with data `{"child": <child id>}` and `parentId` equal to the parent's head before the fork, and the parent's head is that entry's id.
- The count of files and the total bytes under the home's `blocks` directory are equal before and after a fork.
- With a `Session` held open on `parent` in the test, a fork returns `HomeError::SessionHeld`, and the `sessions` directory's file count and the SHA-256 of `parent` are unchanged.

**Files:**
- create: crates/lys-home/src/record/fork.rs
- create: crates/lys-home/src/record/fork_tests.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C39 — The child session holds each cut entry as the parent file's own line bytes, under the parent's cwd, with the header's parentSession the parent file's path relative to the home; nothing is re-serialised.
- C40 — Ancestry is written on both sides: lys.forked_from is the first entry the fork writes in the child, directly after the copy, and its head; lys.fork is appended at the parent's head naming the child; a user-message point is not copied but carried, its id recorded with coordinate_carried true and its parts that are not text counted by kind.
- C42 — A fork writes no block, rewrites no earlier byte of the parent, and copies nothing from the parent but its cwd: no credential, handle or launch setting enters the child.

**Stories:**
- S20 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern's point, carrying everything said up to that point and nothing after it, and launch the child like any session, so that I can go back and talk with the self that lit the lantern.
- S21 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a fork at a user message to carry that message as the child's first prompt beside the rendered file, with nothing from the parent's header but its working directory, so that the child starts at the coordinate and no credential, handle or launch setting is copied from the parent.
- S22 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the ancestry written on both sides, the child's header naming the parent file and a lys.forked_from entry naming the lantern, the point and the cut, and a lys.fork entry at the parent's head naming the child, so that a stranger can tell a fork from its parent from the record alone.
- S23 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a fork's report to carry ids and counts only, with the parent's earlier bytes and the block store unchanged and the whole thing proved through the binary, so that a fork never quietly copies or rewrites anything.

### R4: Count the fork's entries and blocks into a report that carries no content

Structural: `record/fork_report.rs` defines the fork report with exactly the keys `child`, `parent`, `lantern`, `point`, `cut_at`, `entries`, `blocks`, `unstored`, `coordinate_carried` and `carried`. `entries` is the number of entries copied into the child. The candidate hashes are the block hashes the copied custom entries name in their data (`record` of `lys.harness_event`; `request`, `response`, `raw_request` and `raw_response` of `lys.call`) and the SHA-256 of each content part of the copied message entries, serialised with `serde_json::to_vec` as it stands in the entry. `blocks` is the number of distinct candidate hashes that name a block the home's store holds, each checked with `BlockStore::contains` (the block's path is a file); `unstored` is the number of distinct candidate hashes that name no block in the store (the importer stores a Claude Code part in its source form and writes a mapped part inline, so an inline thinking, tool-call or tool-result part names no block). WHEN a fork completes, THE SYSTEM SHALL return this report. THE SYSTEM SHALL NOT count in `blocks` a hash whose block the store does not hold, SHALL NOT read a block's bytes to count it, and SHALL NOT put any part's text, any note, any entry's data or any other transcript content in the report.

**Acceptance:**
- Forking `L5` reports `entries` 7, `blocks` 5 (the four text parts of `e1`, `e2`, `e4`, `e5` and the `record` of `e3`; the lantern entries `L2` and `O2` name no block) and `unstored` 0.
- On a session `mixed` added to the fixture home, built with `Session::append_entry`: `m1` a user message with one text part `fixture-text-1`, put through `BlockStore::put` as `serde_json::to_vec` of the part; `m2` an assistant message whose inline parts are a Pi `thinking` part (thinking `fixture-text-2`, `thinkingSignature` `sig-m2`) and a Pi `toolCall` part, with only their Claude Code source forms (a `thinking` part with `signature` `sig-m2`, and a `tool_use` part) put through `BlockStore::put`, as the importer stores them; and `M2` a `lys.lantern` entry at point `m2` carrying no `lit_in`, the child of `m2`. Forking `M2` reports `entries` 2, `blocks` 1 and `unstored` 2, and the test, hashing the child's copied parts itself, finds a file under the home's `blocks` directory by path for the one hash counted and none for the two unstored.
- Forking `L6` reports `cut_at` `e5`, `point` `e6`, `coordinate_carried` true and `carried` `e6`.
- Two forks of `L5` report two different `child` ids, and the test, hashing each child's copied parts and the block hashes named by its copied custom entries itself, finds the two sets equal with 5 hashes each; the report carries counts and ids only, and no hash is read from it.
- The report serialised to JSON contains none of the eight content sentinels; the test asserts it checked 8 sentinels.

**Files:**
- create: crates/lys-home/src/record/fork_report.rs
- modify: crates/lys-home/src/record/fork_tests.rs

**Checklist:**
- C41 — The fork report carries the child, the parent, the lantern, the point, the cut entry, the count of entries copied, the distinct block hashes the copied entries name that the store holds and those it does not, whether the coordinate was carried and the carried entry; never a part's text, a note or any entry's data.
- C43 — lys-home fork --home --lantern [--session] prints one JSON report on success, exits 1 with the refusal on stderr and nothing on stdout, and takes no point or entry id.

**Stories:**
- S23 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a fork's report to carry ids and counts only, with the parent's earlier bytes and the block store unchanged and the whole thing proved through the binary, so that a fork never quietly copies or rewrites anything.

### R5: Give lys-home the fork subcommand

Structural: `lys-home fork --home <dir> --lantern <id> [--session <id>]` lives in `src/cli_fork.rs`; `cli.rs` gains only the `Fork` variant and its dispatch, and `lib.rs` declares the module. `--session` names the session to cut from, as R2 reads it. WHEN `fork` succeeds, THE SYSTEM SHALL print one JSON object `{"command": "fork", "report": <the R4 report>}` on stdout and exit 0. IF a fork is refused, THEN THE SYSTEM SHALL exit 1 with the refusal on stderr and print nothing on stdout. THE SYSTEM SHALL NOT take a point or an entry id as an argument, SHALL NOT render or launch the child from `fork`, and SHALL NOT print transcript content.

**Acceptance:**
- `lys-home fork --home <fixture> --lantern <L5's id>` exits 0 and prints one JSON object whose keys are exactly `command` and `report`, with `command` equal to `fork` and `report`'s keys exactly those of R4.
- `lys-home fork --home <fixture> --lantern no-such-lantern` exits 1, its stderr contains `no-such-lantern`, and its stdout is empty.
- `lys-home fork --home <fixture> --lantern <L1's id>` exits 1, its stderr contains `nothing_to_fork` and L1's id, and its stdout is empty.
- `lys-home fork --home <fixture>` (no `--lantern`) exits 2, and `lys-home fork --home <fixture> --lantern <L5's id> --point e5` exits 2.
- `cli.rs` has at most 500 lines of code after the change, counted without comments and blank lines.

**Files:**
- create: crates/lys-home/src/cli_fork.rs
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C37 — A fork refuses by name before any file is created: a lantern id no session holds as a lys.lantern entry, an older-record lantern several sessions hold with no session named (lantern_ambiguous, listing them in ascending byte order), a named session that is not the lantern's lit-in session (lantern_not_lit_here), a lantern that sits before any assistant message (nothing_to_fork), and a parent another owner holds; no refusal carries a note, a part's text or any entry's data.
- C43 — lys-home fork --home --lantern [--session] prints one JSON report on success, exits 1 with the refusal on stderr and nothing on stdout, and takes no point or entry id.

**Stories:**
- S20 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern's point, carrying everything said up to that point and nothing after it, and launch the child like any session, so that I can go back and talk with the self that lit the lantern.

### R6: Render the child with its seed prompt, named in the report

WHEN a session is rendered for Claude Code, THE SYSTEM SHALL add to the render report `seed`, null, and SHALL NOT print a launch line from `render`: the printed launch line comes from the template's render only. WHEN the session's path carries a `lys.forked_from` entry with `coordinate_carried` true, THE SYSTEM SHALL also read the carried entry from the parent session named by `parent_session`, through a lock-free view by its index row, and write `<rendered path without .jsonl>.seed.txt` beside the rendered file holding the marker line `<FORKED FROM SESSION <parent id> AT ENTRY <point> BY LANTERN <lantern id>>`, a newline, the heading line `The message at the coordinate, which the transcript does not hold`, a newline, then the carried message's text (its `content` when that is a string; otherwise the `text` of each `text` part in order, joined by a newline, with every part that is not `text` left out, as the child's `lys.forked_from` counts them); `seed` is that file's path. WHEN the child is rendered through `render-launch`, THE SYSTEM SHALL print the template's launch line (the resume by path with `--fork-session`, the three files and the template's flags) with `"$(cat '<seed path>')"` appended as the first prompt when there is one, and list the seed file in the render's manifest, so the child launches with the template's handles and not the bare seat login. IF the seed path already exists, THEN THE SYSTEM SHALL refuse with `HomeError::Exists` naming it and write nothing. THE SYSTEM SHALL NOT write the carried text into the rendered JSONL, the loss account or the report, SHALL NOT refuse a render for a carried part that is not text, SHALL NOT write any byte of a part that is not text into the seed, SHALL NOT run the launch line, SHALL NOT write a credential, a handle or anything taken from the parent's header into the launch line or the seed, and SHALL NOT change how any record renders.

**Acceptance:**
- Rendering the `L5` child to `<out>/a.jsonl` writes 4 records (the messages `e1`, `e2`, `e4`, `e5`), reports `seed` null and no `launch`, and writes no `a.seed.txt`.
- Rendering the `L6` child to `<out>/b.jsonl` writes 4 records, writes `<out>/b.seed.txt` whose bytes are exactly the line `<FORKED FROM SESSION parent AT ENTRY e6 BY LANTERN <L6's id>>`, a newline, the line `The message at the coordinate, which the transcript does not hold`, a newline, and `fixture-text-6`, with no trailing newline (the image part of `e6` left out), and reports `seed` equal to `<out>/b.seed.txt` and no `launch`.
- `<out>/b.jsonl` contains each of `fixture-text-1`, `-2`, `-4` and `-5` (the copied messages' texts) and no `fixture-text-6` (the carried text, which lives only in `<out>/b.seed.txt`); `<out>/b.loss.json` and the render's printed report each contain none of the eight content sentinels; the test asserts it checked 8 sentinels in each.
- Rendering the `L6` child again to `<out>/c.jsonl` with `<out>/c.seed.txt` already present returns `HomeError::Exists` naming `c.seed.txt`, and `<out>/c.jsonl` does not exist afterwards.
- Rendering `parent` itself reports `seed` null and no `launch`, and no render report carries the text `claude --resume`.
- `render-launch` on the `L6` child reports `launch` equal to the template's line followed by `"$(cat '<out>/<uuid>.seed.txt')"`, its manifest lists six files with the seed last, and on the `L5` child the line and the manifest are as HOME-002 measured them.

**Files:**
- create: crates/lys-home/src/harness/claude_code/seed.rs
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs
- modify: crates/lys-home/src/harness/claude_code/launch.rs

**Checklist:**
- C42 — A fork writes no block, rewrites no earlier byte of the parent, and copies nothing from the parent but its cwd: no credential, handle or launch setting enters the child.
- C44 — Rendering a child whose coordinate was carried writes the carried message's text parts as a seed prompt beside the rendered file under an in-band marker line, the render report names it, the template's launch line (printed by render-launch only) passes it as the first prompt and is never run, and PROOF-FORK.md records a fork launched on the installed Claude Code version as hashes, counts, paths, commands, versions and exit codes only.

**Stories:**
- S20 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern's point, carrying everything said up to that point and nothing after it, and launch the child like any session, so that I can go back and talk with the self that lit the lantern.
- S21 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a fork at a user message to carry that message as the child's first prompt beside the rendered file, with nothing from the parent's header but its working directory, so that the child starts at the coordinate and no credential, handle or launch setting is copied from the parent.

### R7: Prove the words' acceptance end to end through the binary

Structural: `tests/fork.rs` builds the fixture home in a temporary directory, lights `L5`, `L6` and `L1` with `lys-home lantern light`, writes `L2` (its data carrying `lit_in` `parent`, by hand until the lantern card's `lit_in` round lands) and `O2` (carrying none) with `Session::append_entry`, and runs the `lys-home` binary (`env!("CARGO_BIN_EXE_lys-home")`) for every fork and render, asserting the count of forks and of refusals, not only successes. THE SYSTEM SHALL NOT name a test after, or print, any content sentinel or note.

**Acceptance:**
- Two `fork --lantern <L5's id>` runs exit 0 with two different `child` ids, the block store's file count and bytes are unchanged across both, and the two children's copied lines have equal SHA-256 line for line.
- `fork --lantern <L6's id>` exits 0 and its report's `cut_at` is `e5`, differing from its `point` `e6`.
- After those forks, `fork --lantern <L2's id>` exits 0 with `parent` as the report's `parent`, and `fork --lantern <O2's id> --session parent` exits 0 with `parent` as the report's `parent`.
- `fork --lantern no-such-lantern`, `fork --lantern e5`, `fork --lantern <L1's id>`, `fork --lantern <O2's id>` and `fork --lantern <L2's id> --session <the first L5 child's id>` each exit 1 naming the lantern id on stderr, the last three with `nothing_to_fork`, `lantern_ambiguous` and `lantern_not_lit_here` respectively; the test asserts 5 forks succeeded and 5 were refused.
- After the five forks, the parent's head is the last `lys.fork` entry written and the parent holds exactly 5 `lys.fork` entries, whose `child` values are the five reported child ids.

**Files:**
- create: crates/lys-home/tests/fork.rs

**Checklist:**
- C37 — A fork refuses by name before any file is created: a lantern id no session holds as a lys.lantern entry, an older-record lantern several sessions hold with no session named (lantern_ambiguous, listing them in ascending byte order), a named session that is not the lantern's lit-in session (lantern_not_lit_here), a lantern that sits before any assistant message (nothing_to_fork), and a parent another owner holds; no refusal carries a note, a part's text or any entry's data.
- C40 — Ancestry is written on both sides: lys.forked_from is the first entry the fork writes in the child, directly after the copy, and its head; lys.fork is appended at the parent's head naming the child; a user-message point is not copied but carried, its id recorded with coordinate_carried true and its parts that are not text counted by kind.
- C41 — The fork report carries the child, the parent, the lantern, the point, the cut entry, the count of entries copied, the distinct block hashes the copied entries name that the store holds and those it does not, whether the coordinate was carried and the carried entry; never a part's text, a note or any entry's data.

**Stories:**
- S23 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a fork's report to carry ids and counts only, with the parent's earlier bytes and the block store unchanged and the whole thing proved through the binary, so that a fork never quietly copies or rewrites anything.

### R8: Write the fork entries into the record document and render the cluster

Structural: `docs/design/home/RECORD.md` gains a section for `lys.forked_from` (data `parent_session`, `lantern`, `point`, `cut_at`, `coordinate_carried`, `carried`, `seed_left_out`; the child's first entry written by the fork, directly after the copied chain) and one for `lys.fork` (data `child`; appended at the parent's head), states that a child's header `parentSession` is the parent file's path relative to the home, that the copied lines are the parent's own bytes, defines the report's `blocks` and `unstored` counts as R4 does, including why an imported part's inline form can name no block, and states that a fork cuts from the session a lantern was lit in, read from the `lys.lantern` data key `lit_in` (a lantern whose data carries no `lit_in` resolves by the older-record rule), and that a copy of a lantern's line in a child is a copy. The cluster's rendered markdown (`DESIGN.md`, `CHECKLIST.md`, `USER-STORIES.md`, `briefs/HOME-006.md`) is regenerated with `scripts/design/render-cluster.py` from the JSON as it stands. THE SYSTEM SHALL NOT edit a rendered markdown file by hand, SHALL NOT change the cluster's JSON documents in this requirement, and SHALL NOT describe either entry as a signed or frozen wire format.

**Acceptance:**
- RECORD.md holds one list item beginning `` `lys.forked_from` `` that names all seven of its data keys, and one list item beginning `` `lys.fork` `` that names `child`.
- `sh scripts/design/gate.sh` exits 0.

**Files:**
- create: docs/design/home/briefs/HOME-006.md
- modify: docs/design/home/RECORD.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md

**Checklist:**
- C36 — The two fork custom types, lys.forked_from (parent_session, lantern, point, cut_at, coordinate_carried, carried, seed_left_out) and lys.fork (child), are named beside the other lys custom entries and documented in RECORD.md, and neither adds a field to Pi's header or outside custom.data.

**Stories:**
- S22 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the ancestry written on both sides, the child's header naming the parent file and a lys.forked_from entry naming the lantern, the point and the cut, and a lys.fork entry at the parent's head naming the child, so that a stranger can tell a fork from its parent from the record alone.

### R9: Measure a conversation with a lantern's self through a fork

Structural: `docs/design/home/PROOF-FORK.md` records, as hashes, counts, paths, commands, versions and exit codes only: the lines where Pi at 3d5cbe98 reads `parentSession`; the Claude Code version printed by `claude --version` on the machine the proof runs on; a real Claude Code session imported into a scratch home with its source hash before and after; a lantern lit with `lys-home lantern light` at a past assistant entry; the fork's report; the child file loaded with Pi's `loadEntriesFromFile` at 3d5cbe98 and its entry count; the child rendered under a fresh uuid; the launch line run, the launch template's line from `render-launch` run by hand, with `-p` and one question that asks for a one-word answer, a word that appears in the parent's chain only at or before the point; the run's exit code, the continuation file's path, SHA-256 and record counts, and `lys-home resume-check` on it; the SHA-256 of the answer beside the SHA-256 of the expected answer, where the answer is the run's printed reply and both it and the expected word are trimmed of surrounding whitespace, stripped of one trailing full stop and lowercased before hashing; the parent's pre-fork prefix hash before and after; the block store's file count and bytes before and after; and the same for a second fork from a lantern at a user entry launched with its seed; and the number of text parts the leak check compared and the number it left out as empty after trimming or holding no letter and no digit, with no length bound on a compared part. THE SYSTEM SHALL NOT write a line of transcript content, a note, a question's or an answer's text into the proof, and SHALL NOT record a credential or a handle.

**Acceptance:**
- PROOF-FORK.md names the Claude Code version the launches ran on, and the version string it names is the one `claude --version` printed in the run it records.
- The assistant-point fork's launch exits 0, the SHA-256 of its reply (trimmed of surrounding whitespace, stripped of one trailing full stop, lowercased) equals the SHA-256 of the expected word normalised the same way, and `resume-check` reports 0 repeated tool uses.
- The recorded parent prefix hash after each fork equals the parent's whole-file hash before it, and the block store's file count and bytes are equal before and after.
- The Pi load of the child reports as many entries as the fork's `entries` plus 1.
- The user-point fork's report shows `coordinate_carried` true, and its seeded launch exits 0.
- PROOF-FORK.md contains none of the source session's message texts: each text part of the imported session's messages is trimmed of surrounding whitespace, every part that is empty after trimming or holds no letter and no digit is left out, each remaining part is compared with each line of PROOF-FORK.md trimmed of surrounding whitespace, and 0 are equal; the question's text occurs 0 times in the file and the expected word occurs 0 times as a whole whitespace-separated token; the file records the number of parts compared and, beside it, the number of parts left out.

**Files:**
- create: docs/design/home/PROOF-FORK.md

**Checklist:**
- C44 — Rendering a child whose coordinate was carried writes the carried message's text parts as a seed prompt beside the rendered file under an in-band marker line, the render report names it, the template's launch line (printed by render-launch only) passes it as the first prompt and is never run, and PROOF-FORK.md records a fork launched on the installed Claude Code version as hashes, counts, paths, commands, versions and exit codes only.

**Stories:**
- S20 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern's point, carrying everything said up to that point and nothing after it, and launch the child like any session, so that I can go back and talk with the self that lit the lantern.

## Boundaries

- SHALL NOT define, light, hold or recall a lantern, or change the lantern card's entries, view or subcommands; the fork reads `lys.lantern` as that card writes it, including the lit-in session it records under `lit_in`.
- SHALL NOT cut at a point that no lantern names; `fork` takes a lantern id only.
- SHALL NOT give a fork tools, a lens or a briefing on what changed since the point.
- SHALL NOT build a lease, a budget, a kill rule, the Claude Code launch template or a handle broker; the launch line is printed, never run by lys-home.
- SHALL NOT support a harness other than Claude Code.
- SHALL NOT rewrite, truncate or delete any byte of a session file, a block, or any file under Claude Code's own projects directory.
- SHALL NOT add a field to Pi's grammar; ancestry rides in Pi's `parentSession` and in `custom` entries only.
- SHALL NOT put transcript content in a report, an error, a log line, a test name or a proof document.
- SHALL NOT depend on a Norn crate, and SHALL NOT touch lys-core's wire formats.

## Verification

- From the repository root: `python3 scripts/design/validate.py docs/design/home` and `python3 scripts/design/check-coverage.py docs/design/home` exit 0.
- From the repository root: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --all-features`, `cargo doc --no-deps --all-features` and `cargo doc --no-deps` exit 0.
- From the repository root: `sh scripts/design/gate.sh` and `sh .land/gates.sh` exit 0.
- Every new source file under `crates/lys-home/src` has at most 500 lines of code, counted without comments and blank lines, and `record/mod.rs` gains only `pub mod` and test-module lines.
- `cargo test -p lys-home --all-features --test fork` passes and its output shows the counted case and refusal assertions.
- `grep -c 'fixture-text-' crates/lys-home/tests/fork.rs` counts only fixture construction and sentinel checks; no `#[test]` function name contains `fixture-text`.
- Read PROOF-FORK.md against R9's acceptance: every figure is a hash, a count, a path, a command, a version or an exit code.


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


---
type: brief
id: HOME-008
cluster: home
title: Sign each call, harness event and home-recorded open of a home into a lys log under the agent's identity, and verify a session against it
---

# HOME-008: Sign each call, harness event and home-recorded open of a home into a lys log under the agent's identity, and verify a session against it

> **Cluster:** home
> **Depends on:** HOME-001, HOME-002, HOME-003, HOME-004, HOME-006
> **Blocked by:** DIRECTORY-003 R2 landed, with the home-entry, home-entry-copy and home checkpoint payloads reviewed with its envelope before any durable byte is signed: checked by `git cat-file -e main:docs/design/identity/IDENTITY-EVENTS.md` and `git cat-file -e main:crates/lys-identity/src/lib.rs` both succeeding on lys main., F-BV2vqP landed: the Lys card for lys-log-store's change that stores subtree hashes instead of holding every leaf in memory (today at crates/lys-log-store/src/log.rs:53-60), released as lys-log-store 0.3.0; checked on lys main by `cargo metadata --format-version 1 --no-deps` reporting version 0.3.0 for the package named lys-log-store (its Cargo.toml takes its version from the workspace, so the version is read as cargo resolves it).
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-019 — A home entry's leaf is a typed payload of the identity event envelope, signed by the agent's own key after its durable line — Each lys.call and lys.harness_event entry appends one leaf to one log the home keeps under its agent's identity: a typed payload of the identity event envelope (DIRECTORY-003 R2), tagged lys/home-entry/v1, carrying the session id, the entry id, the kind and the SHA-256 of the entry's line, signed through sign_attestation with the agent identity's own Ed25519 key as the identity crate holds it. The line is written first and never removed; a failed leaf append is refused naming the entry, marks the session pending and refuses further appends until the leaf is written at the next open or by verify --repair. The log starts with one checkpoint leaf, tagged lys/home-checkpoint/v1, naming each session with its entry count and file SHA-256. lys-home gains a verify subcommand. Rejected: a second envelope for home entries; a home key, a person's key or a stand-in key; writing the leaf before the line; a best-effort append that lets the session continue past an unlogged line; a per-entry backfill; verification as a library function only; and recording opens in the published lys open.
> - ADR-020 — The home's non-goal on signing narrows to anchoring — Signing into a lys log is in scope for lys.call entries, lys.harness_event entries and sealed-record opens, and for nothing else; anchoring stays out of scope. Rejected: keeping the whole non-goal, and widening signing to every entry kind.
> **Checklist:**
> - C45 — lys-home depends on lys-core, lys-log-store and lys-identity, and the home-entry leaf (session id, entry id, kind, SHA-256 of the entry line, tag lys/home-entry/v1), the home-entry-copy leaf (those four, the parent session id, the parent leaf's index and the parent entry's position, tag lys/home-entry-copy/v1) and the home checkpoint leaf (tag lys/home-checkpoint/v1) are typed payloads of the identity event envelope, written in docs/design/identity/IDENTITY-EVENTS.md.
> - C46 — On a home that logs its entries, every lys.call, lys.harness_event and lys.open entry appended to a session through append_entry, append_beside or append_under appends exactly one leaf to the home's log at log/ under the home, signed with the Ed25519 key of the agent log/agent names through lys_core::attestation::sign_attestation, and no other entry kind appends a leaf.
> - C47 — No leaf and no file under the home's log directory carries a byte of transcript content: a byte scan of every file under it for a fixture message's text finds nothing.
> - C48 — lys-home log-enable starts a home's log with one checkpoint leaf naming each session of the home with its entry count and the SHA-256 of its file as they stand, and a log never holds a second checkpoint.
> - C49 — A failed leaf append is refused naming the entry id, the entry is never reported as recorded, the session is marked pending by log/pending/<session id>.json naming the session and the entry, every further append to it is refused until the pending leaf is written, and the leaf is written first at the session's next open or by lys-home verify --repair.
> - C50 — Ingesting a call_id already recorded adds no leaf, and writes the entry's one leaf when the log holds none for it.
> - C51 — lys-home verify --home <dir> --session <id> checks every logged entry line against its one leaf under the agent identity's public key and the pre-log history against the checkpoint, prints one JSON report of counts and hashes, and otherwise exits 1 naming the first entry, or the session for the checkpoint, that fails.
> - C52 — docs/design/home/RECORD.md records the three leaf formats (home-entry, home-entry-copy and checkpoint), their tags, the log's place in the home, the pending marker and the order of line against leaf.
> - C53 — The adversarial review of the three leaf formats (forgery, cross-protocol confusion, replay, a leaf under another key, guessing a short line from its hash) is recorded in docs/design/identity/IDENTITY-EVENTS.md before any durable byte is signed under them.
> - C54 — A home that never ran log-enable appends every entry as before, with no leaf and no log; lys-home log-enable --agent names the home's agent in log/agent, and is refused by name, creating nothing, when --agent is missing or the identity crate holds no key for that agent.
> - C55 — On a home that logs its entries and whose agent's key the identity crate does not hold, a lys.call, lys.harness_event or lys.open append is refused before its line is written, naming the home and the entry kind and the act that answers it, with nothing written.
> - C56 — lys-home open opens a sealed envelope with open_and_verify, writes the plaintext to --out, created new with mode 0600 on Unix, and appends one lys.open entry (the envelope's SHA-256, the sender's public key and the --out path) to the named session with its leaf; an existing --out is refused by name, and a failed open gives lys open's one failure message and writes no plaintext, entry or leaf.
> - C57 — A fork on a logging home gives each lys.call, lys.harness_event and lys.open line it copies its own leaf under the child session id, tagged as a copy and naming the parent session id, the parent entry's position and the parent leaf's index, or the checkpoint leaf's index when the parent entry predates the checkpoint, and is never refused for such a line; the child whose copies name parent leaves verifies without the parent's files, and verify refuses a copy whose parent leaf is absent or differs, and a copy naming the checkpoint whose parent line is absent or differs.
> **Stories:**
> - S24 (Agent, Keeps a history a stranger can verify) — As an agent, I want each call and each harness event of my session signed into a log under my own identity, so that a stranger can verify my history without trusting the home's files.
> - S25 (Agent, Keeps a history a stranger can verify) — As an agent whose home held entries before logging began, I want those entries covered by one signed checkpoint, so that my earlier history can be told apart from tampering.
> - S26 (Reviewer, Verifies a session against its log) — As the reviewer, I want one command that checks a session's entry lines against the log and names the first entry that fails, so that tampering is found by entry rather than suspected.
> - S27 (Reviewer, Verifies a session against its log) — As the reviewer, I want a failed log append refused with its entry named and the session held until the leaf is written, so that no entry is ever reported as recorded without its leaf.
> - S28 (Agent, Keeps a history a stranger can verify) — As an agent, I want each sealed record I open through my home recorded as an entry with its own signed leaf, naming the envelope by hash and never its content, so that a stranger can see what I opened without learning what it said.

## Purpose

A home's lys.call and lys.harness_event entries today are lines in a file: nothing signs them and none reaches a lys log, so a stranger cannot tell an honest history from an edited one, and opening a sealed record leaves no record at all. This brief gives each such entry, each line a fork copies of those kinds (one written before the log included), and each sealed record opened through the home, one signed leaf in a lys log the home keeps under its agent's identity, carrying ids, the kind and the line's hash and never content. Logging is enabled on a home by one act that names the home's agent, records it in the home and writes a signed checkpoint covering what the home held before it; a home never enabled appends as today. A failed append is refused by entry without removing the line and holds the session pending under a named marker, an append to a logging home whose agent key is absent is refused before its line is written, and lys-home gains open and verify commands. The design's signed entry log paragraph, CN11, CN12 and ADR-019 carry the architecture.

## Task

Build after both blockers land, on the lys main they land on. Add lys-core, lys-log-store and lys-identity to lys-home; define the three payloads in the identity event envelope (R1); keep one log for the home at log/, naming its agent in log/agent, and sign with that agent identity's own key as the identity crate holds it (R2); enable logging on a home by lys-home log-enable --agent, which writes log/agent and the checkpoint and is refused without the agent's key (R3); hook the leaf append into the end of Session::append_entry, Session::append_beside, Session::append_under and Session::append_copied for lys.call, lys.harness_event and lys.open only, on a logging home only, with a copy leaf for a line a fork copies, naming the parent entry's leaf, or the checkpoint leaf when the parent entry predates it (R4), which covers ingest_call, ingest_call_files, ingest_outcome, the Claude Code importer, render-launch's template_render, lys-home open and lys-home fork; refuse an append to a logging home whose agent key is absent before its line is written (R5); refuse a failed append by entry, mark the session pending at log/pending/<session id>.json and write the pending leaf first at the session's next open (R6); reconcile a missing leaf on re-ingest (R7); open a sealed record through lys-home open, checking the session, its pending state and the agent key before anything is opened, and record it as a lys.open entry (R8); verify a session, a forked child included (R9), through a new lys-home verify subcommand (R10); run the words' tests through the binary (R11); record the formats and their adversarial review (R12). record/mod.rs, record/call.rs and harness/claude_code/import.rs are each within about 40 lines of the 500-line limit: new logic goes in the named new files and those files gain only the calls into them. crates/lys-identity/src/lib.rs and docs/design/identity/IDENTITY-EVENTS.md are not in the tree today; the DIRECTORY-003 R2 blocker brings them, and this brief changes them only after it lands. Out of scope: putting a sealed record into a session (no entry kind holds one, and none is added); the verifier command line crates/lys, whose lys open writes nothing and keeps its single OpenFailed; the door's proxy and the runtime; anchoring; encryption at rest; Haematite as the store; any entry kind but lys.call, lys.harness_event and lys.open; a per-entry backfill of entries written before the checkpoint.

## Requirements

### R1: Add the three home payloads to the identity event envelope

Structural. crates/lys-home/Cargo.toml gains lys-core, lys-log-store and lys-identity as path dependencies of the workspace, and nothing else. The identity event envelope that DIRECTORY-003 R2 lands carries three typed payloads for the home, and no second envelope: the home-entry payload, tagged lys/home-entry/v1, with exactly four fields in this order: session (the session id, UTF-8), entry (the entry id, UTF-8), kind (the entry's customType, UTF-8: lys.call, lys.harness_event or lys.open) and line_sha256 (the 32-byte SHA-256 of the entry's line as written, excluding its terminating newline); the home-entry-copy payload, tagged lys/home-entry-copy/v1, for a line a fork copies into a child session, with exactly seven fields in this order: session (the child session id), entry, kind, line_sha256 as the home-entry payload has them, then parent_session (the parent session id, UTF-8), parent_leaf (the unsigned 64-bit index in the home's log of the parent entry's leaf, or of the log's checkpoint leaf when the parent entry has no leaf because it was written before the checkpoint) and parent_position (the unsigned 64-bit position of the parent entry among the entry lines of the parent session file, counted from 1, header excluded, present on every copy leaf whichever parent leaf it names, never optional and never a byte offset); and the home checkpoint payload, tagged lys/home-checkpoint/v1, holding one row per session of the home, sorted by session id bytewise, each with exactly session (UTF-8), entries (the count of entry lines, header excluded) and file_sha256 (the 32-byte SHA-256 of the file's bytes). crates/lys-home/src/record/leaf.rs encodes and decodes a leaf: its bytes are one encoding, the envelope's own serialised form carrying the tagged payload and the COSE_Sign1 bytes that lys_core::attestation::sign_attestation produces over that payload's encoded bytes. crates/lys-identity/src/lib.rs and docs/design/identity/IDENTITY-EVENTS.md are not in the tree today: the DIRECTORY-003 R2 blocker brings them, and this requirement modifies them only after it lands. WHERE the envelope as landed carries none of these payloads, THE SYSTEM SHALL add them as typed payloads of that same envelope in crates/lys-identity/src/home_payloads.rs, declared from crates/lys-identity/src/lib.rs, and name them in docs/design/identity/IDENTITY-EVENTS.md; WHERE the envelope as landed already carries them under these tags and fields, THE SYSTEM SHALL use them as landed and SHALL NOT create home_payloads.rs. THE SYSTEM SHALL NOT define a second envelope, SHALL NOT put any field in any of the three payloads beyond those named, SHALL NOT carry a message, tool, compaction, plaintext or envelope byte in any, SHALL NOT mutate lys/attestation/v2 or any other format lys-core publishes, and SHALL NOT sign any durable byte under any of the three tags before R12's review is recorded. record/mod.rs declares leaf and, under cfg(test), leaf_tests in this requirement.

**Acceptance:**
- A unit test encodes a home-entry payload for session `s1`, entry `e1`, kind `lys.call` and line_sha256 of the bytes `abc`, decodes it, and gets back exactly those four values with line_sha256 equal to ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad.
- A unit test encodes a home-entry-copy payload for session `child`, entry `e1`, kind `lys.call`, line_sha256 of `abc`, parent_session `parent`, parent_leaf 7 and parent_position 3, decodes it, and gets back exactly those seven values.
- A unit test signs a home-entry leaf with one Ed25519 identity and verifies its attestation with lys_core::attestation::verify_attestation_bytes_by_signer under that identity's public key: Ok.
- The same leaf verified under a second identity's public key is refused.
- A checkpoint payload built from sessions `b` then `a` encodes its rows in the order `a`, `b`.
- Each of the six pairs of a leaf of one of the three tags decoded as a payload of another of the three is refused: the test asserts 6 refusals.
- `cargo tree -p lys-home --depth 1` lists lys-core, lys-log-store and lys-identity.
- docs/design/identity/IDENTITY-EVENTS.md names the three tags and the fields of each payload in the order above.

**Files:**
- create: crates/lys-home/src/record/leaf.rs
- create: crates/lys-home/src/record/leaf_tests.rs
- create: crates/lys-identity/src/home_payloads.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/Cargo.toml
- modify: Cargo.lock
- modify: crates/lys-identity/src/lib.rs
- modify: docs/design/identity/IDENTITY-EVENTS.md

**Checklist:**
- C45 — lys-home depends on lys-core, lys-log-store and lys-identity, and the home-entry leaf (session id, entry id, kind, SHA-256 of the entry line, tag lys/home-entry/v1), the home-entry-copy leaf (those four, the parent session id, the parent leaf's index and the parent entry's position, tag lys/home-entry-copy/v1) and the home checkpoint leaf (tag lys/home-checkpoint/v1) are typed payloads of the identity event envelope, written in docs/design/identity/IDENTITY-EVENTS.md.

**Stories:**
- S24 (Agent, Keeps a history a stranger can verify) — As an agent, I want each call and each harness event of my session signed into a log under my own identity, so that a stranger can verify my history without trusting the home's files.

### R2: Keep one signed log for the home, name its agent, and append a leaf to it

crates/lys-home/src/record/entry_log.rs holds the home's log at log/ under the home root, a lys-log-store FileLeafStore behind lys-log-store's Log, one log for every session of the home. The home's agent is named by the file log/agent under the home root, which holds the agent identity as given to log-enable (R3) followed by one newline and nothing else; a session finds its home, and so its log, from its own file's place in the home (sessions/<id>.jsonl under the home root). WHEN a leaf is appended, THE SYSTEM SHALL read the agent from log/agent, sign the payload with the Ed25519 key the identity crate (DIRECTORY-003 R2) holds for that agent identity, through lys_core::attestation::sign_attestation, and append the leaf as one log leaf. IF log/ exists and log/agent does not, THEN THE SYSTEM SHALL refuse with a message beginning `home <home> logs its entries and names no agent in log/agent`, and write no leaf. WHEN asked whether the log holds a leaf for a session and entry id, THE SYSTEM SHALL answer from the leaves in the log alone, with that leaf's index when it does. THE SYSTEM SHALL NOT sign with any key but the named agent identity's, SHALL NOT infer the agent from the home's path, SHALL NOT make, generate or store a key of its own, SHALL NOT write a leaf's coordinate into any session file, SHALL NOT hold the log's leaves in memory beyond what lys-log-store's Log holds, and SHALL NOT put key material in any Debug output, error or report. record/mod.rs declares entry_log and, under cfg(test), entry_log_tests in this requirement.

**Acceptance:**
- A unit test on a fresh home whose log/agent names agent `agent-fixture`, whose key the identity crate holds, appends a home-entry leaf for session `s1` entry `e1`: the home then holds log/log.json and log/leaves/00000000000000000000, and the log's size is 1.
- After that append, asking the log for session `s1` entry `e1` answers present at index 0, and for session `s1` entry `e2` answers absent.
- The leaf read back from log/leaves/00000000000000000000 verifies under the public key of `agent-fixture`, and is refused under the public key of a second identity the identity crate holds.
- A unit test on a home whose log/ exists and whose log/agent was removed appends a leaf: refused with a message beginning `home ` and holding `names no agent in log/agent`, and the log's size is unchanged.
- `grep -rn 'generate\|from_seed\|SigningKey::' crates/lys-home/src/record/entry_log.rs` prints nothing.
- The Debug output of the entry log's value, formatted with {:?} in a test, does not contain the hex of the identity's seed.

**Files:**
- create: crates/lys-home/src/record/entry_log.rs
- create: crates/lys-home/src/record/entry_log_tests.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C46 — On a home that logs its entries, every lys.call, lys.harness_event and lys.open entry appended to a session through append_entry, append_beside or append_under appends exactly one leaf to the home's log at log/ under the home, signed with the Ed25519 key of the agent log/agent names through lys_core::attestation::sign_attestation, and no other entry kind appends a leaf.

**Stories:**
- S24 (Agent, Keeps a history a stranger can verify) — As an agent, I want each call and each harness event of my session signed into a log under my own identity, so that a stranger can verify my history without trusting the home's files.

### R3: Enable logging on a home, writing the checkpoint leaf

lys-home log-enable --home <dir> --agent <agent identity>. A home logs its entries when its log/ directory holds a log; a home whose log/ does not exist has never had logging enabled. WHEN log-enable runs on a home that does not log and the identity crate holds a key for the agent named by --agent, THE SYSTEM SHALL write log/agent naming that agent (R2), create the log and append as its leaf 0 one lys/home-checkpoint/v1 leaf naming every session of the home with its entry count and the SHA-256 of its file as they stand, signed with that agent's key, print one JSON object with exactly the fields command (`log-enable`), agent, sessions (the number of rows in the checkpoint), log_size and log_root (lowercase hex), and exit 0. IF --agent is not given, THEN THE SYSTEM SHALL exit 2 with the argument parser's message naming `--agent` on stderr and create nothing. IF the identity crate holds no key for the named agent (an agent it does not know included), THEN THE SYSTEM SHALL exit 1 with a message on stderr beginning `log-enable refused: the identity crate holds no key for agent ` followed by the agent, then ` of home ` and the home, and create nothing. THE SYSTEM SHALL NOT write a leaf for any entry on record before the checkpoint, SHALL NOT write a second checkpoint to a log that holds one, SHALL NOT enable logging by any act but log-enable, SHALL NOT take the agent from anything but --agent, and SHALL NOT read any line's content into the checkpoint beyond hashing the file's bytes. record/mod.rs declares log_checkpoint and, under cfg(test), log_checkpoint_tests, and lib.rs declares cli_entry_log, in this requirement.

**Acceptance:**
- A unit test on a home holding session `pre` with 2 entries and session `other` with 3 entries runs log-enable with agent `agent-fixture`: log/agent holds exactly the bytes `agent-fixture` and one newline, the log holds 1 leaf, a checkpoint with rows (`other`, 3, SHA-256 of other's file) then (`pre`, 2, SHA-256 of pre's file) that verifies under the public key of `agent-fixture`, and the report carries agent `agent-fixture`, sessions 2 and log_size 1.
- Appending one lys.call to `pre` after that adds exactly one leaf, a home-entry leaf, and the log holds 2 leaves of which exactly 1 is a checkpoint.
- Running log-enable a second time on that home leaves the log holding exactly 1 checkpoint leaf.
- On a fresh home holding no session, log-enable writes a checkpoint with 0 rows and a log of size 1.
- lys-home log-enable --home H without --agent exits 2 with stderr holding `--agent`, and H has no log/ directory afterwards.
- On a home, lys-home log-enable --agent `agent-unheld`, an agent whose key the identity crate does not hold, exits 1 with stderr beginning `log-enable refused: the identity crate holds no key for agent agent-unheld of home `, and the home has no log/ directory afterwards.

**Files:**
- create: crates/lys-home/src/record/log_checkpoint.rs
- create: crates/lys-home/src/record/log_checkpoint_tests.rs
- create: crates/lys-home/src/cli_entry_log.rs
- modify: crates/lys-home/src/lib.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/cli.rs

**Checklist:**
- C48 — lys-home log-enable starts a home's log with one checkpoint leaf naming each session of the home with its entry count and the SHA-256 of its file as they stand, and a log never holds a second checkpoint.
- C54 — A home that never ran log-enable appends every entry as before, with no leaf and no log; lys-home log-enable --agent names the home's agent in log/agent, and is refused by name, creating nothing, when --agent is missing or the identity crate holds no key for that agent.

**Stories:**
- S25 (Agent, Keeps a history a stranger can verify) — As an agent whose home held entries before logging began, I want those entries covered by one signed checkpoint, so that my earlier history can be told apart from tampering.

### R4: Append one leaf for every lys.call, lys.harness_event and lys.open line of a logging home

The hook is one call into the entry log made at the end of each of the four writers of a session line, after the last step each makes durable: Session::append_entry (which Session::append calls, and through which ingest_call, ingest_call_files, ingest_outcome and the Claude Code importer write) after the head; Session::append_beside (template_render's side leaf) after the index row; Session::append_under after the index row; and Session::append_copied (a fork's copy of a parent line) after the head. WHEN any of the four writes an entry whose customType is lys.call, lys.harness_event or lys.open to a session of a home that logs its entries, THE SYSTEM SHALL append that entry's one leaf, taking the line's bytes from what was written: a home-entry leaf for append_entry, append_beside and append_under, and for append_copied a home-entry-copy leaf naming the parent session id, the parent entry's position in the parent session file and, as parent_leaf, the index of the parent entry's leaf in the home's log. WHEN append_copied copies a lys.call, lys.harness_event or lys.open line whose parent entry has no leaf in the log because it is among the entries the checkpoint counts for the parent session, THE SYSTEM SHALL name as parent_leaf the index of the log's checkpoint leaf, with the parent entry's position, and SHALL NOT refuse the fork for it. WHEN the home has never had logging enabled, THE SYSTEM SHALL append the entry as it did before this brief, with no leaf and no log. THE SYSTEM SHALL NOT append a leaf for any other entry kind (message, lys.given, lys.lantern, lys.lantern_epilogue, lys.fork, lys.forked_from, lys.authored, lys.inherited, compaction and the rest), SHALL NOT append a leaf before the line is durable, SHALL NOT write a line of a logged kind by any path but these four, SHALL NOT create a log for a home that does not log, SHALL NOT move the head or change the session head hash because of a leaf, and SHALL NOT change any byte a session file held before. Logic beyond the call into the entry log stays out of record/mod.rs, which stays under 500 lines of code.

**Acceptance:**
- A unit test appends one message entry, one lys.given entry and one lys.lantern entry to a session of a logging home: the log's size does not change.
- Session::append of one lys.call entry to a session of a logging home grows the log by exactly 1 leaf, a home-entry leaf of kind lys.call naming that entry, and the head is that entry afterwards.
- Importing a Claude Code transcript of one `permission-mode` record into a logging home, which writes through Session::append_entry, grows the log by exactly 1 leaf, whose kind is lys.harness_event.
- Session::append_beside of one lys.harness_event entry to a session of a logging home grows the log by exactly 1 leaf of kind lys.harness_event, and the session's head before and after is the same entry id.
- Session::append_under of one lys.open entry under a named entry of a session of a logging home grows the log by exactly 1 leaf of kind lys.open, and the session's head before and after is the same entry id.
- A unit test on a logging home appends to session `parent` a user message, a lys.call and an assistant message, lights a lantern at the assistant message and forks it: the log grows by exactly 1 leaf for the child, a home-entry-copy leaf of kind lys.call whose session is the child's id, whose entry is the lys.call's id, whose parent_session is `parent`, whose parent_position is 2 and whose parent_leaf is the index of the parent's lys.call leaf; the copied messages, lys.forked_from and lys.fork add no leaf.
- A unit test writes to session `early` of a home a user message, a lys.call and an assistant message, runs log-enable, then lights a lantern at the assistant message and forks it: the fork succeeds, and the log grows by exactly 1 leaf for the child, a home-entry-copy leaf of kind lys.call whose parent_session is `early`, whose parent_leaf is 0 (the checkpoint leaf) and whose parent_position is 2.
- The leaf of an appended lys.call has line_sha256 equal to the SHA-256 of that entry's line in the session file, read back from the file, excluding its newline.
- The bytes of the session file before an append are a prefix of its bytes after it.
- On a home that never ran log-enable, appending one lys.call writes its line, and the home has no log/ directory afterwards.

**Files:**
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/record/beside.rs
- modify: crates/lys-home/src/record/fork.rs
- modify: crates/lys-home/src/record/entry_log_tests.rs

**Checklist:**
- C46 — On a home that logs its entries, every lys.call, lys.harness_event and lys.open entry appended to a session through append_entry, append_beside or append_under appends exactly one leaf to the home's log at log/ under the home, signed with the Ed25519 key of the agent log/agent names through lys_core::attestation::sign_attestation, and no other entry kind appends a leaf.
- C54 — A home that never ran log-enable appends every entry as before, with no leaf and no log; lys-home log-enable --agent names the home's agent in log/agent, and is refused by name, creating nothing, when --agent is missing or the identity crate holds no key for that agent.
- C57 — A fork on a logging home gives each lys.call, lys.harness_event and lys.open line it copies its own leaf under the child session id, tagged as a copy and naming the parent session id, the parent entry's position and the parent leaf's index, or the checkpoint leaf's index when the parent entry predates the checkpoint, and is never refused for such a line; the child whose copies name parent leaves verifies without the parent's files, and verify refuses a copy whose parent leaf is absent or differs, and a copy naming the checkpoint whose parent line is absent or differs.

**Stories:**
- S24 (Agent, Keeps a history a stranger can verify) — As an agent, I want each call and each harness event of my session signed into a log under my own identity, so that a stranger can verify my history without trusting the home's files.

### R5: Refuse an append to a logging home whose agent key is absent, before its line is written

IF an entry whose customType is lys.call, lys.harness_event or lys.open is to be appended to a session of a home that logs its entries and the identity crate holds no key for the agent log/agent names (an agent it does not know included), THEN THE SYSTEM SHALL refuse before the line is written with a message beginning `home <home> logs its entries and the identity crate holds no key for its agent: refused <kind>` and ending `hold the agent's key in the identity crate for this home`, and write no line, no index row, no head and no leaf. IF log/agent is missing, THEN THE SYSTEM SHALL refuse the same append before its line with R2's message and write nothing. THE SYSTEM SHALL NOT write an unlogged lys.call, lys.harness_event or lys.open line to a session of a logging home, SHALL NOT sign with any other key in its place, and SHALL NOT carry content in the refusal.

**Acceptance:**
- In the end-to-end test, on a logging home whose agent's key the identity crate no longer holds, lys-home ingest-call for call id `k1` exits 1 with stderr beginning `home ` and holding `refused lys.call`, and the bytes of the session file and every file under log/ are the same before and after.
- With the key held again, lys-home ingest-call for call id `k1` exits 0, the session file holds one lys.call line for `k1`, and the log holds exactly one home-entry leaf for that entry.
- A unit test on a logging home whose log/agent was removed appends one lys.call: refused with a message holding `names no agent in log/agent`, and the session file's bytes are unchanged.

**Files:**
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C55 — On a home that logs its entries and whose agent's key the identity crate does not hold, a lys.call, lys.harness_event or lys.open append is refused before its line is written, naming the home and the entry kind and the act that answers it, with nothing written.

**Stories:**
- S27 (Reviewer, Verifies a session against its log) — As the reviewer, I want a failed log append refused with its entry named and the session held until the leaf is written, so that no entry is ever reported as recorded without its leaf.

### R6: Refuse a failed append by entry and hold the session until its leaf is written

The pending marker of a session is the file log/pending/<session id>.json under the home root, holding one JSON object with exactly two fields, session (the session id) and entry (the entry id whose leaf is pending), and nothing else. IF the leaf append for an entry fails after its line is durable, THEN THE SYSTEM SHALL write that session's pending marker naming the entry, and return a refusal whose message begins `leaf append failed for entry <entry id> of session <session id>`. WHEN a session with a pending marker is opened for writing (Home::open_session, Session::open), THE SYSTEM SHALL first append the pending entry's leaf, unless the log already holds a leaf for it, and remove the marker, before the open returns; IF that fails, THEN THE SYSTEM SHALL leave the marker as it is and the open returns. WHILE a session has a pending marker, WHEN any entry is to be appended to it, THE SYSTEM SHALL first try the pending leaf in the same way; IF that fails, THEN THE SYSTEM SHALL refuse the append with a message beginning `session <session id> is pending the leaf of entry <entry id>` and write no line. THE SYSTEM SHALL NOT retry a pending leaf when a session is only read (SessionReader, and verify without --repair), SHALL NOT remove or rewrite the pending entry's line, SHALL NOT report the entry as recorded in any report, SHALL NOT append a second leaf for an entry the log already holds, and SHALL NOT carry content in the marker or either refusal. record/mod.rs declares pending and, under cfg(test), pending_tests in this requirement, and calls into pending from Home::open_session, Session::open and the append path.

**Acceptance:**
- In the end-to-end test, with the home's log/leaves directory made read-only, lys-home ingest-call for call id `c3` exits 1, prints nothing on stdout, and prints on stderr a line beginning `leaf append failed for entry ` followed by the entry id of the lys.call holding `c3` in the session file.
- Directly after, the session file holds that lys.call line, the log's size is unchanged, and log/pending/logged.json holds exactly the JSON object with session `logged` and entry the id of that lys.call.
- Still read-only, lys-home ingest-call for call id `c4` exits 1, prints on stderr a line beginning `session logged is pending the leaf of entry ` followed by the same entry id, and the session file holds no entry for `c4`.
- A unit test leaves session `s1` pending the leaf of entry `e1` with log/leaves read-only, makes log/leaves writable, and calls Home::open_session on `s1` and appends nothing: the log then holds exactly one leaf for `e1` and log/pending/s1.json does not exist.
- With log/leaves writable again, lys-home ingest-call for call id `c4` exits 0, and the log then holds exactly one leaf for the `c3` entry and exactly one for the `c4` entry, and log/pending/logged.json does not exist.
- A unit test that finds the log already holding the pending entry's leaf removes the marker and appends no leaf.

**Files:**
- create: crates/lys-home/src/record/pending.rs
- create: crates/lys-home/src/record/pending_tests.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C49 — A failed leaf append is refused naming the entry id, the entry is never reported as recorded, the session is marked pending by log/pending/<session id>.json naming the session and the entry, every further append to it is refused until the pending leaf is written, and the leaf is written first at the session's next open or by lys-home verify --repair.

**Stories:**
- S27 (Reviewer, Verifies a session against its log) — As the reviewer, I want a failed log append refused with its entry named and the session held until the leaf is written, so that no entry is ever reported as recorded without its leaf.

### R7: Keep re-ingest idempotent and reconcile a missing leaf

WHEN ingest_call, ingest_call_files or ingest_outcome finds its call id already recorded, THE SYSTEM SHALL check the log for that entry's leaf, append it if the log holds none, and report already_recorded as today. THE SYSTEM SHALL NOT append a second leaf for an entry the log already holds and SHALL NOT write a second lys.call line.

**Acceptance:**
- In the end-to-end test, a second lys-home ingest-call with call id `c1` exits 0 with already_recorded true and the log's size unchanged.
- A unit test that appends a lys.call line with its leaf failing, then ingests the same call id with the log writable, leaves the log holding exactly one leaf for that entry and the session holding exactly one lys.call line for that call id.

**Files:**
- modify: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/call_tests.rs

**Checklist:**
- C50 — Ingesting a call_id already recorded adds no leaf, and writes the entry's one leaf when the log holds none for it.

**Stories:**
- S24 (Agent, Keeps a history a stranger can verify) — As an agent, I want each call and each harness event of my session signed into a log under my own identity, so that a stranger can verify my history without trusting the home's files.

### R8: Open a sealed record through the home and record the open

lys-home open --home <dir> --session <id> --key <recipient key file> --sender-public-key <hex> --envelope <file> --attestation <file> --out <file>, taking the recipient key file, the sender's Ed25519 public key, the sealed envelope and its attestation as lys open takes them, since lys_core::seal::open_and_verify needs them. Before open_and_verify runs and before any plaintext is written, THE SYSTEM SHALL check in this order: IF --out names an existing path, THEN refuse with a message beginning `open refused: --out ` naming that path; IF the named session is not in the home, THEN refuse with the error opening that session gives, naming the session; IF the session is pending and its pending leaf cannot be written (R6), THEN refuse with R6's pending message; IF the home logs its entries and the identity crate holds no key for its agent, or log/agent is missing, THEN refuse with R5's message for kind lys.open; each such refusal exits 1 and writes nothing. WHEN those checks pass and open_and_verify succeeds, THE SYSTEM SHALL write the plaintext to --out in a file it creates new with mode 0600 on Unix in the same call that creates it, as lys open does, then append to the named session one lys.open custom entry whose data holds exactly envelope_sha256 (the lowercase hex SHA-256 of the sealed envelope file's bytes), sender_public_key (lowercase hex) and out (the --out path as given), and that entry's leaf by R4; the report on stdout names the entry id, the envelope_sha256 and the payload byte count. IF open_and_verify fails for any reason, THEN THE SYSTEM SHALL refuse with the one message lys open gives for every such failure, `sealed envelope open failed: invalid attestation or undecryptable envelope`, exit 1, write no plaintext, append no entry and append no leaf. THE SYSTEM SHALL NOT put a byte of the plaintext or of the envelope in the entry, the leaf, the report or any error, SHALL NOT record a refused open in the session or the log, SHALL NOT tell one cryptographic failure from another, SHALL NOT run open_and_verify before the checks above pass, SHALL NOT create the --out file with any wider mode, even for a moment before narrowing it, and SHALL NOT change crates/lys. record/mod.rs declares open and, under cfg(test), open_tests, and lib.rs declares cli_open, in this requirement.

**Acceptance:**
- In the end-to-end test, a record sealed with lys-core to a recipient key and opened by lys-home open into session `opened` of a logging home exits 0, the --out file's bytes equal the sealed plaintext, the session gains exactly one lys.open entry whose envelope_sha256 equals the SHA-256 of the envelope file, and the log grows by exactly 1 leaf, of kind lys.open.
- The same open with a second recipient key exits 1 with stderr holding `sealed envelope open failed: invalid attestation or undecryptable envelope`, the --out path does not exist afterwards, the session file's bytes are unchanged, and the log's size is unchanged.
- The same open with --out naming an existing file exits 1 with stderr beginning `open refused: --out `, that file's bytes are unchanged, and the session file's bytes and the log's size are unchanged.
- The same open with --session `absent`, a session the home does not hold, exits 1 with stderr holding `absent`, and the --out path does not exist afterwards.
- The same open on the logging home after the identity crate no longer holds its agent's key exits 1 with stderr beginning `home ` and holding `refused lys.open`, the session file's bytes are unchanged, and the --out path does not exist afterwards.
- The same open into session `opened` while it is pending the leaf of an entry and log/leaves is read-only exits 1 with stderr beginning `session opened is pending the leaf of entry `, the session file's bytes are unchanged, and the --out path does not exist afterwards.
- On Unix, after the successful open, the --out file's permission bits read 0600.
- A byte scan of the session file and every file under log/ for the sealed plaintext's text finds 0 matches after the successful open.

**Files:**
- create: crates/lys-home/src/record/open.rs
- create: crates/lys-home/src/record/open_tests.rs
- create: crates/lys-home/src/cli_open.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C56 — lys-home open opens a sealed envelope with open_and_verify, writes the plaintext to --out, created new with mode 0600 on Unix, and appends one lys.open entry (the envelope's SHA-256, the sender's public key and the --out path) to the named session with its leaf; an existing --out is refused by name, and a failed open gives lys open's one failure message and writes no plaintext, entry or leaf.

**Stories:**
- S28 (Agent, Keeps a history a stranger can verify) — As an agent, I want each sealed record I open through my home recorded as an entry with its own signed leaf, naming the envelope by hash and never its content, so that a stranger can see what I opened without learning what it said.

### R9: Verify a session against the log and the checkpoint

Verify reads the session through SessionReader and the log, and writes nothing. WHEN a session is verified, THE SYSTEM SHALL check, over the leaves of the home's log for that session and the checkpoint: every leaf's attestation verifies over its payload and its signer is the public key of the agent log/agent names; each lys.call, lys.harness_event and lys.open entry after the checkpoint's count has exactly one leaf (home-entry or home-entry-copy) whose kind equals the entry's customType and whose line_sha256 equals the SHA-256 of its line as the file holds it; every such leaf names an entry the session holds; for each home-entry-copy leaf whose parent_leaf is not the checkpoint's index, the log's leaf at its parent_leaf index exists and is a home-entry or home-entry-copy leaf whose session is its parent_session and whose entry and line_sha256 equal the copy's; for each home-entry-copy leaf whose parent_leaf is the checkpoint's index, the checkpoint leaf exists in the log and names parent_session with an entry count of at least parent_position, and the entry line at parent_position in the parent session file is byte-equal to the copied line; and the SHA-256 of the file's bytes through the end of the checkpoint's counted entry lines equals the checkpoint's file_sha256. IF a home-entry-copy leaf's parent_position is 0, or is greater than the number of entry lines in the parent session file (header excluded) whenever that file is in the home, THEN THE SYSTEM SHALL refuse naming the copied entry as for any failing entry. THE SYSTEM SHALL check a copy that names a parent entry's leaf against that leaf in the log without requiring any file of the parent session, and SHALL require the parent session file only for a copy that names the checkpoint leaf. WHEN every check passes, THE SYSTEM SHALL return a result carrying exactly session (the session id), entries (the entry lines in the session file, header excluded), logged (the lys.call, lys.harness_event and lys.open entries after the checkpoint's count), leaves (the number of home-entry and home-entry-copy leaves in the log whose session is this session), checkpoint (the checkpoint's entries and file_sha256 for this session, entries 0 and no file hash when the checkpoint does not name it), log_size and log_root. IF an entry fails, THEN THE SYSTEM SHALL refuse naming the first such entry in file order with a message beginning `verify refused: entry <entry id> of session <session id>`; IF the pre-log history fails, THEN THE SYSTEM SHALL refuse with a message beginning `verify refused: session <session id> does not match the checkpoint`. THE SYSTEM SHALL NOT accept a leaf signed by any other key, SHALL NOT pass a logged entry that has no leaf or two leaves, SHALL NOT pass a copy whose parent leaf is absent or differs, SHALL NOT pass a copy naming the checkpoint leaf whose parent entry line is absent or differs from the copied line, SHALL NOT pass a copy whose parent_position is 0 or past the parent file's last entry line, SHALL NOT print any line content, and SHALL NOT write any file unless asked to repair. record/mod.rs declares verify and, under cfg(test), verify_tests in this requirement.

**Acceptance:**
- A unit test signs one leaf of a logged session with a second Ed25519 identity: verify refuses with a message beginning `verify refused: entry ` and naming that entry's id.
- A unit test appends a duplicate leaf for one entry: verify refuses naming that entry.
- A unit test with a leaf naming entry `ghost`, which the session does not hold, refuses naming `ghost`.
- A unit test whose logged entry has no leaf and no pending marker refuses naming that entry.
- On R4's forked home, verifying the child with the parent's files sessions/parent.jsonl, sessions/parent.index.jsonl and sessions/parent.head removed returns a passing result whose leaves is 1 and whose logged is 1.
- On R4's forked home with the parent's files in place, verifying the child returns a passing result whose leaves is 1; with the copy leaf re-signed to name as parent_leaf the index of the checkpoint leaf, keeping parent_position 2, verify refuses naming the copied lys.call's entry id, since the checkpoint counts 0 entries of `parent`.
- On R4's home forked from `early`, with the parent's files in place, verify of the child returns a passing result whose leaves is 1, whose logged is 1 and whose checkpoint entries is 0.
- On that home, with the `m` of the model in the lys.call line of sessions/early.jsonl changed to `n` in place, verify of the child refuses with a message beginning `verify refused: entry ` and naming the copied lys.call's entry id.
- On that home, with sessions/early.jsonl, sessions/early.index.jsonl and sessions/early.head removed, verify of the child refuses with a message beginning `verify refused: entry ` and naming the copied lys.call's entry id.
- A unit test on a home holding session `edge` of exactly three entry lines, a lys.call, a user message and a lys.call, written before log-enable, runs log-enable, then copies all three entry lines, in file order, into a new session `edge-child` through Session::append_copied, so that each copied line's parent is already in `edge-child` and no copy is refused with UnknownParent: the copied user message adds no leaf, and the log grows by exactly 2 home-entry-copy leaves with parent_leaf 0 and parent_position 1 and 3, and verify of `edge-child` returns a passing result whose leaves is 2.
- On that home, with the copy leaf of the third line re-signed to name parent_position 4, verify of `edge-child` refuses with a message beginning `verify refused: entry ` and naming that copied entry's id.
- On that home, with the copy leaf of the first line re-signed to name parent_position 0, verify of `edge-child` refuses with a message beginning `verify refused: entry ` and naming that copied entry's id.

**Files:**
- create: crates/lys-home/src/record/verify.rs
- create: crates/lys-home/src/record/verify_tests.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C51 — lys-home verify --home <dir> --session <id> checks every logged entry line against its one leaf under the agent identity's public key and the pre-log history against the checkpoint, prints one JSON report of counts and hashes, and otherwise exits 1 naming the first entry, or the session for the checkpoint, that fails.

**Stories:**
- S26 (Reviewer, Verifies a session against its log) — As the reviewer, I want one command that checks a session's entry lines against the log and names the first entry that fails, so that tampering is found by entry rather than suspected.

### R10: Give lys-home a verify subcommand

lys-home verify --home <dir> --session <id> [--repair]. WHEN every check of R9 passes, THE SYSTEM SHALL print one JSON object on stdout with exactly the fields command (`verify`), then session, entries, logged, leaves (home-entry and home-entry-copy leaves whose session is this session), checkpoint (an object of entries and file_sha256), log_size and log_root (lowercase hex) as R9's result carries them, and exit 0. WHEN --repair is given and the session is pending, THE SYSTEM SHALL write the pending leaf first, then verify, and the report SHALL carry repaired with the count of leaves it wrote. IF verification refuses, THEN THE SYSTEM SHALL exit 1 with R9's message on stderr and print nothing on stdout. THE SYSTEM SHALL NOT write any file without --repair and SHALL NOT print any line content.

**Acceptance:**
- lys-home verify --home H --session logged, on the end-to-end test's home after two calls and one event, prints a report with entries 3, logged 3, leaves 3, checkpoint entries 0, log_size 4, and exits 0.
- Without --repair, the bytes of every file under H are the same before and after the command, a pending marker included.
- On a home pending the leaf of one entry, lys-home verify --repair with the log writable prints a report with repaired 1 and exits 0, and log/pending/<session id>.json is gone.

**Files:**
- modify: crates/lys-home/src/cli_entry_log.rs
- modify: crates/lys-home/src/cli.rs

**Checklist:**
- C51 — lys-home verify --home <dir> --session <id> checks every logged entry line against its one leaf under the agent identity's public key and the pre-log history against the checkpoint, prints one JSON report of counts and hashes, and otherwise exits 1 naming the first entry, or the session for the checkpoint, that fails.

**Stories:**
- S26 (Reviewer, Verifies a session against its log) — As the reviewer, I want one command that checks a session's entry lines against the log and names the first entry that fails, so that tampering is found by entry rather than suspected.

### R11: Run the words' tests through the command line

crates/lys-home/tests/entry_log.rs drives the lys-home binary on a fresh temporary home whose agent identity `agent-fixture` has its key held by the identity crate. It runs lys-home log-enable --agent agent-fixture on it, imports crates/lys-home/tests/fixtures/entry_log/one_event.jsonl, a Claude Code transcript of one `permission-mode` record and no message, as session `logged`, then ingests two calls with call ids `c1` and `c2`, each with model `m-fixture`, whose request bodies each carry one user message holding the text `lys-fixture-sentinel-4f1c`. crates/lys-home/tests/fixtures/entry_log/prelog/ is a home written by lys-home at the base commit, before logging, of exactly three files: sessions/pre.jsonl, holding session `pre` with one lys.call entry of model `m-fixture` and one lys.harness_event entry, its index sessions/pre.index.jsonl and its head sessions/pre.head. THE SYSTEM SHALL pass each check below and SHALL NOT pass a check by running zero cases: each asserts the count it ran over.

**Acceptance:**
- After log-enable, the import and the two ingests, the log holds exactly 4 leaves: 1 checkpoint of 0 rows and 3 home-entry leaves (two of kind lys.call, one of kind lys.harness_event), and lys-home verify exits 0 with leaves 3.
- Changing the `m` of `m-fixture` in the `c2` entry's line in the session file to `n`, in place, makes lys-home verify exit 1 with stderr beginning `verify refused: entry <id of the c2 entry> of session logged`, and names no other entry.
- Ingesting `c1` a second time leaves the log at 4 leaves.
- A byte scan of every file under the home's log/ directory for `lys-fixture-sentinel-4f1c` scans at least 6 files (log.json, agent and 4 leaves) and finds 0 matches, while the same scan of the home's blocks/ directory finds at least 1.
- On a copy of the prelog fixture, lys-home ingest-call into `pre` without log-enable exits 0, writes its line, and leaves the home with no log/ directory.
- On a second copy of the prelog fixture, lys-home log-enable --agent agent-fixture then one lys-home ingest-call into `pre` gives a log of 2 leaves whose leaf 0 is a checkpoint naming `pre` with 2 entries, and lys-home verify --session pre exits 0 with checkpoint entries 2.
- On that second copy, changing the `m` of `m-fixture` in the pre-log lys.call line to `n`, in place, makes lys-home verify --session pre exit 1 with stderr beginning `verify refused: session pre does not match the checkpoint`.

**Files:**
- create: crates/lys-home/tests/entry_log.rs
- create: crates/lys-home/tests/fixtures/entry_log/one_event.jsonl
- create: crates/lys-home/tests/fixtures/entry_log/prelog/sessions/pre.jsonl
- create: crates/lys-home/tests/fixtures/entry_log/prelog/sessions/pre.index.jsonl
- create: crates/lys-home/tests/fixtures/entry_log/prelog/sessions/pre.head

**Checklist:**
- C46 — On a home that logs its entries, every lys.call, lys.harness_event and lys.open entry appended to a session through append_entry, append_beside or append_under appends exactly one leaf to the home's log at log/ under the home, signed with the Ed25519 key of the agent log/agent names through lys_core::attestation::sign_attestation, and no other entry kind appends a leaf.
- C47 — No leaf and no file under the home's log directory carries a byte of transcript content: a byte scan of every file under it for a fixture message's text finds nothing.
- C48 — lys-home log-enable starts a home's log with one checkpoint leaf naming each session of the home with its entry count and the SHA-256 of its file as they stand, and a log never holds a second checkpoint.
- C50 — Ingesting a call_id already recorded adds no leaf, and writes the entry's one leaf when the log holds none for it.
- C51 — lys-home verify --home <dir> --session <id> checks every logged entry line against its one leaf under the agent identity's public key and the pre-log history against the checkpoint, prints one JSON report of counts and hashes, and otherwise exits 1 naming the first entry, or the session for the checkpoint, that fails.
- C54 — A home that never ran log-enable appends every entry as before, with no leaf and no log; lys-home log-enable --agent names the home's agent in log/agent, and is refused by name, creating nothing, when --agent is missing or the identity crate holds no key for that agent.

**Stories:**
- S24 (Agent, Keeps a history a stranger can verify) — As an agent, I want each call and each harness event of my session signed into a log under my own identity, so that a stranger can verify my history without trusting the home's files.
- S25 (Agent, Keeps a history a stranger can verify) — As an agent whose home held entries before logging began, I want those entries covered by one signed checkpoint, so that my earlier history can be told apart from tampering.
- S26 (Reviewer, Verifies a session against its log) — As the reviewer, I want one command that checks a session's entry lines against the log and names the first entry that fails, so that tampering is found by entry rather than suspected.

### R12: Record the formats and their adversarial review

docs/design/home/RECORD.md gains a section on the signed entry log: the log at log/ under the home and log/agent naming its agent, the three tags and their fields, which entry kinds are logged (lys.call, lys.harness_event and lys.open) and what a lys.open entry's data holds, a fork's copy leaf, naming the parent entry's leaf or, for a parent entry from before the checkpoint, the checkpoint leaf and the entry's position, log-enable and what a home that never ran it does, the line-then-leaf order, the pending marker at log/pending/<session id>.json and its retry, the checkpoint, and how verify reads them; its opening sentence that nothing is signed is corrected to name what is. docs/design/identity/IDENTITY-EVENTS.md, which the DIRECTORY-003 R2 blocker brings and which this requirement modifies only after it lands, records the adversarial review of the three payloads, held jointly with the review of the envelope DIRECTORY-003 R2 lands, covering a forged leaf, a leaf signed by another key, cross-protocol confusion among the three tags and with the envelope's other payloads, a replayed leaf for another entry or session (a copy leaf offered for its parent's entry included), and guessing a short line from its SHA-256, each with the attack tried and why it fails. THE SYSTEM SHALL NOT sign a durable byte under any of the three tags before that review is recorded, and SHALL NOT change the rules those documents already state for other formats.

**Acceptance:**
- `grep -c 'lys/home-entry/v1\|lys/home-entry-copy/v1\|lys/home-checkpoint/v1' docs/design/home/RECORD.md` prints a number of at least 3.
- RECORD.md no longer says nothing is signed without naming lys.call, lys.harness_event and lys.open as signed on a logging home.
- IDENTITY-EVENTS.md holds a review record naming the five attacks above, each with its outcome.

**Files:**
- modify: docs/design/home/RECORD.md
- modify: docs/design/identity/IDENTITY-EVENTS.md

**Checklist:**
- C52 — docs/design/home/RECORD.md records the three leaf formats (home-entry, home-entry-copy and checkpoint), their tags, the log's place in the home, the pending marker and the order of line against leaf.
- C53 — The adversarial review of the three leaf formats (forgery, cross-protocol confusion, replay, a leaf under another key, guessing a short line from its hash) is recorded in docs/design/identity/IDENTITY-EVENTS.md before any durable byte is signed under them.

**Stories:**
- S24 (Agent, Keeps a history a stranger can verify) — As an agent, I want each call and each harness event of my session signed into a log under my own identity, so that a stranger can verify my history without trusting the home's files.

## Boundaries

- No change to crates/lys: lys open writes to no log and keeps its single non-oracle OpenFailed.
- No second envelope: the three payloads are typed payloads of the identity event envelope DIRECTORY-003 R2 lands, and no field beyond those R1 names.
- No key but the agent identity's own as the identity crate holds it, for the agent log/agent names: no home key, no person's key, no generated or stand-in key, and no agent inferred from the home's path.
- No durable byte signed under any of the three tags before R12's review is recorded.
- No change to crates/lys-identity/src/lib.rs or docs/design/identity/IDENTITY-EVENTS.md before the DIRECTORY-003 R2 blocker brings them.
- No existing session byte removed or rewritten, and no leaf coordinate written into a session file; the head and the session head hash never move because of a leaf.
- No leaf for any entry kind but lys.call, lys.harness_event and lys.open, no per-entry backfill, no log created but by log-enable, and no unlogged line of those kinds in a logging home.
- No entry kind that holds a sealed record, no refused open recorded anywhere, and no open_and_verify run before the session, pending and key checks pass.
- No transcript content, plaintext or envelope byte in any leaf, log file, pending marker, entry, error, report or test name.
- No change to lys-log-store's LeafStore: no fork, merge, delete or rewrite.
- No anchoring, no encryption at rest, no Haematite, and nothing of the door's proxy or the runtime.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- From the repository root: cargo fmt --all leaves the tree unchanged.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0.
- cargo test --workspace --all-features exits 0 and lists R1 to R11's tests as passed.
- cargo doc --no-deps --all-features and cargo doc --no-deps exit 0 with no warnings.
- sh scripts/design/gate.sh exits 0.
- git diff <base> -- crates/lys prints nothing.
- Replacing the leaf append in record/mod.rs with nothing makes R11's first test fail on the leaf count, and only tests that count leaves fail, recorded as a drift injection in the dev record.
- Each of record/mod.rs, record/call.rs and harness/claude_code/import.rs has under 500 lines of code.

