---
type: brief
id: HOME-031
cluster: home
title: Sign each call, harness event and home-recorded open of a home into a lys log under the agent's identity, and verify a session against it
---

# HOME-031: Sign each call, harness event and home-recorded open of a home into a lys log under the agent's identity, and verify a session against it

> **Cluster:** home
> **Depends on:** HOME-001, HOME-002, HOME-003, HOME-004, HOME-006
> **Blocked by:** DIRECTORY-003 R2 landed, with the home-entry, home-entry-copy and home checkpoint payloads reviewed with its envelope before any durable byte is signed: checked by `git cat-file -e main:docs/design/identity/IDENTITY-EVENTS.md` and `git cat-file -e main:crates/lys-identity/src/lib.rs` both succeeding on lys main., F-BV2vqP landed: the Lys card for lys-log-store's change that stores subtree hashes instead of holding every leaf in memory (today at crates/lys-log-store/src/log.rs:53-60), released as lys-log-store 0.3.0; checked on lys main by `cargo metadata --format-version 1 --no-deps` reporting version 0.3.0 for the package named lys-log-store (its Cargo.toml takes its version from the workspace, so the version is read as cargo resolves it).
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-019 — The secrets broker lives in lys, as crates/lys-secrets; Cambium consumes it and is never its home (amends ADR-001) — The broker lives in the lys repository as the crate crates/lys-secrets, part of the standalone identity platform, built in Rust. A person installs Lys to hold secrets; Cambium reaches the broker through the door as a consumer and is never its home. Rejected: the broker inside the cambium door that ADR-001 and SECRETS-002 name, which would make Cambium a runtime a person must install to keep a credential.
> - ADR-020 — A lease is a broker record under the audit log, not a signed format — The lease is a broker record under the audit log: it counts uses, time and spend against a grant and is read only by the broker, so nothing a stranger verifies changes. The limit on a lease's time is the window of the lys-identity grant it counts against (DIRECTORY-006 R1): the lease's own not_after never extends past it. Rejected: a signed lease format, a new version beside lys/delegation/v1 with its own adversarial review; and reading the window as one the signed delegation keeps, which v1 cannot carry.
> **Checklist:**
> - C45 — A Session asked by find_call for a call id builds, once per open, a map from call id to the first lys.call entry holding it in file order, keeps it current on every append and rebuilds it after a reconcile; once it is built, an ingest through ingest_call, ingest_call_files and ingest_outcome reads no lys.call entry, and a second ingest of a recorded call id records nothing and returns that entry's id.
> - C46 — A Session reports how many entries it has read from its file and how many syncs its own writes made (its line file, its index, its head and the sessions directory), and the block store reports its own syncs beside them, as counts a test reads.
> - C47 — The import command builds a new session under `<id>.jsonl.importing` with no per-entry sync and publishes it with one sync each of the line file, the index and the head and a sessions-directory sync before and after the rename to `<id>.jsonl`, five syncs whatever the record count; a crash before the rename leaves no `<id>.jsonl`, and the next import or open of that id removes what was left.
> - C48 — RECORD.md states the staged import's durability rule beside the per-append rule, as ADR-108 records it.
> - C49 — resume_check counts each transcript's tool_use ids in one pass and reports the same values as before.
> - C50 — The canon keeps the id set load builds, and adding an example refuses a repeated id by that set, never by walking the loaded entries.
> - C51 — Opening a session from its cached index checks every row in memory, reads the final byte of at most three rows (the first, the middle and the last by position) through one buffered reader, and returns a read error other than an unexpected end of file as that error, never as a stale index that is rebuilt.
> - C52 — context_path moves entries out of the path it read, customs reads only the path's entries of the custom type asked for, and the render's assistant arm and the importer's assistant content iterate a content array by reference, with no rendered or recorded byte changed.
> - C53 — One function, uuid_string, writes every uuid's 8-4-4-4-12 form: the fewshot's ids are 16 random bytes with the version nibble 4 and the variant nibble 8 set, formatted by it, and every render hash pinned in the tree is unchanged.
> - C54 — The line bytes of the multi_result fixture imported through the import command hash, under SHA-256 after each fresh 32-hex id is replaced by its order of first appearance and the header timestamp by a fixed token, to the value the same test gives at the parent commit.
> - C55 — On a home that logs its entries and whose agent's key the identity crate does not hold, a lys.call, lys.harness_event or lys.open append is refused before its line is written, naming the home and the entry kind and the act that answers it, with nothing written.
> - C56 — lys-home open opens a sealed envelope with open_and_verify, writes the plaintext to --out, created new with mode 0600 on Unix, and appends one lys.open entry (the envelope's SHA-256, the sender's public key and the --out path) to the named session with its leaf; an existing --out is refused by name, and a failed open gives lys open's one failure message and writes no plaintext, entry or leaf.
> - C57 — A fork on a logging home gives each lys.call, lys.harness_event and lys.open line it copies its own leaf under the child session id, tagged as a copy and naming the parent session id, the parent entry's position and the parent leaf's index, or the checkpoint leaf's index when the parent entry predates the checkpoint, and is never refused for such a line; the child whose copies name parent leaves verifies without the parent's files, and verify refuses a copy whose parent leaf is absent or differs, and a copy naming the checkpoint whose parent line is absent or differs.
> **Stories:**
> - S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.
> - S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.
> - S26 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want a disk error while a session opens reported as that error, so that a failing disk is seen and never papered over by a rebuilt index.
> - S27 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.
> - S28 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose session was imported from Claude Code, I want it rendered as a Codex thread that Codex resumes, with every text, tool call and tool result carried whole, so that I continue on Codex knowing what the session knew rather than a clipped summary of it.

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
- C45 — A Session asked by find_call for a call id builds, once per open, a map from call id to the first lys.call entry holding it in file order, keeps it current on every append and rebuilds it after a reconcile; once it is built, an ingest through ingest_call, ingest_call_files and ingest_outcome reads no lys.call entry, and a second ingest of a recorded call id records nothing and returns that entry's id.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

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
- C46 — A Session reports how many entries it has read from its file and how many syncs its own writes made (its line file, its index, its head and the sessions directory), and the block store reports its own syncs beside them, as counts a test reads.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

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
- C48 — RECORD.md states the staged import's durability rule beside the per-append rule, as ADR-108 records it.
- C54 — The line bytes of the multi_result fixture imported through the import command hash, under SHA-256 after each fresh 32-hex id is replaced by its order of first appearance and the header timestamp by a fixed token, to the value the same test gives at the parent commit.

**Stories:**
- S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.

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
- C46 — A Session reports how many entries it has read from its file and how many syncs its own writes made (its line file, its index, its head and the sessions directory), and the block store reports its own syncs beside them, as counts a test reads.
- C54 — The line bytes of the multi_result fixture imported through the import command hash, under SHA-256 after each fresh 32-hex id is replaced by its order of first appearance and the header timestamp by a fixed token, to the value the same test gives at the parent commit.
- C57 — A fork on a logging home gives each lys.call, lys.harness_event and lys.open line it copies its own leaf under the child session id, tagged as a copy and naming the parent session id, the parent entry's position and the parent leaf's index, or the checkpoint leaf's index when the parent entry predates the checkpoint, and is never refused for such a line; the child whose copies name parent leaves verifies without the parent's files, and verify refuses a copy whose parent leaf is absent or differs, and a copy naming the checkpoint whose parent line is absent or differs.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

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
- S27 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.

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
- C49 — resume_check counts each transcript's tool_use ids in one pass and reports the same values as before.

**Stories:**
- S27 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.

### R7: Keep re-ingest idempotent and reconcile a missing leaf

WHEN ingest_call, ingest_call_files or ingest_outcome finds its call id already recorded, THE SYSTEM SHALL check the log for that entry's leaf, append it if the log holds none, and report already_recorded as today. THE SYSTEM SHALL NOT append a second leaf for an entry the log already holds and SHALL NOT write a second lys.call line.

**Acceptance:**
- In the end-to-end test, a second lys-home ingest-call with call id `c1` exits 0 with already_recorded true and the log's size unchanged.
- A unit test that appends a lys.call line with its leaf failing, then ingests the same call id with the log writable, leaves the log holding exactly one leaf for that entry and the session holding exactly one lys.call line for that call id.

**Files:**
- modify: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/call_tests.rs

**Checklist:**
- C50 — The canon keeps the id set load builds, and adding an example refuses a repeated id by that set, never by walking the loaded entries.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

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
- S28 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose session was imported from Claude Code, I want it rendered as a Codex thread that Codex resumes, with every text, tool call and tool result carried whole, so that I continue on Codex knowing what the session knew rather than a clipped summary of it.

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
- C51 — Opening a session from its cached index checks every row in memory, reads the final byte of at most three rows (the first, the middle and the last by position) through one buffered reader, and returns a read error other than an unexpected end of file as that error, never as a stale index that is rebuilt.

**Stories:**
- S26 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want a disk error while a session opens reported as that error, so that a failing disk is seen and never papered over by a rebuilt index.

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
- C51 — Opening a session from its cached index checks every row in memory, reads the final byte of at most three rows (the first, the middle and the last by position) through one buffered reader, and returns a read error other than an unexpected end of file as that error, never as a stale index that is rebuilt.

**Stories:**
- S26 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want a disk error while a session opens reported as that error, so that a failing disk is seen and never papered over by a rebuilt index.

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
- C46 — A Session reports how many entries it has read from its file and how many syncs its own writes made (its line file, its index, its head and the sessions directory), and the block store reports its own syncs beside them, as counts a test reads.
- C47 — The import command builds a new session under `<id>.jsonl.importing` with no per-entry sync and publishes it with one sync each of the line file, the index and the head and a sessions-directory sync before and after the rename to `<id>.jsonl`, five syncs whatever the record count; a crash before the rename leaves no `<id>.jsonl`, and the next import or open of that id removes what was left.
- C48 — RECORD.md states the staged import's durability rule beside the per-append rule, as ADR-108 records it.
- C50 — The canon keeps the id set load builds, and adding an example refuses a repeated id by that set, never by walking the loaded entries.
- C51 — Opening a session from its cached index checks every row in memory, reads the final byte of at most three rows (the first, the middle and the last by position) through one buffered reader, and returns a read error other than an unexpected end of file as that error, never as a stale index that is rebuilt.
- C54 — The line bytes of the multi_result fixture imported through the import command hash, under SHA-256 after each fresh 32-hex id is replaced by its order of first appearance and the header timestamp by a fixed token, to the value the same test gives at the parent commit.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.
- S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.
- S26 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want a disk error while a session opens reported as that error, so that a failing disk is seen and never papered over by a rebuilt index.

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
- C52 — context_path moves entries out of the path it read, customs reads only the path's entries of the custom type asked for, and the render's assistant arm and the importer's assistant content iterate a content array by reference, with no rendered or recorded byte changed.
- C53 — One function, uuid_string, writes every uuid's 8-4-4-4-12 form: the fewshot's ids are 16 random bytes with the version nibble 4 and the variant nibble 8 set, formatted by it, and every render hash pinned in the tree is unchanged.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

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
