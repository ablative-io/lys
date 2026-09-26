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

Add `lys-home fork --home <dir> --lantern <id> [--session <id>]`. It resolves the lantern id to the session the lantern was lit in, read from its `lys.lantern` data's `lit_in` (a copy of a lantern's line inside a fork's child is a copy, not a second lantern), cuts that session's raw root-to-point chain (read through the index) back to the last assistant message at or before the lantern's point, creates a child session whose header's `parentSession` is the parent file's path relative to the home, copies the chain's lines byte for byte, appends `lys.forked_from` to the child directly after the copy and `lys.fork` at the parent's head, and prints one JSON report of ids and counts. When the point is a user message, that message is not copied: the cut stops at the assistant message before it, `lys.forked_from` records the message's id with `coordinate_carried` true, and the child's render writes its text parts, in order, as a seed prompt beside the rendered file under an in-band marker line, while `lys.forked_from` counts the parts left out by kind; the render prints the launch line, `claude --resume` by path, passing the seed as the first prompt when there is one. Nothing is copied from the parent: the child launches on whatever the launch supplies. The launch template (HOME-002, on main) is what supplies it: `render-launch` prints the template's line with the seed as the first prompt, so the child launches with the template's handles and not the bare seat login, and the proof says which line it ran. In scope: the fork, its report, the seed and launch line on render, RECORD.md, PROOF-FORK.md and the cluster's rendered markdown. A lantern that sits before any assistant message is refused by name (`nothing_to_fork`): a point before the first reply would carry only the seed, which is a new session and not a fork. Out of scope: lighting, holding or recalling lanterns (the lantern card); cutting at a point without a lantern; forks with tools or a lens; leases, budgets and kill rules; the launch template and handles; harnesses other than Claude Code. Before the header value is written, read how Pi reads `parentSession` in `packages/coding-agent/src/core/session-manager.ts` at 3d5cbe98 and record the lines in PROOF-FORK.md; if Pi there reads it as anything other than a session file path, stop and name it to the lead rather than choose a form. The fixture home used below holds one session `parent`, built with `Session::append_entry` under fixed ids, in this order: `e1` user message with one text part `fixture-text-1`; `e2` assistant message with one text part `fixture-text-2`; `s2` a `lys.harness_event` of kind `permission_mode` whose parent is `e2` (a side leaf, off the chain), its `record` naming a block of its own; then the head moved back to `e2` with `Session::move_head` and the session closed; then the lantern `L2` at point `e2`, a `lys.lantern` entry written with `Session::append_entry` as a child of `e2` whose data carries `lit_in` `parent` beside `point`, `note`, `lit_by` and `lit_at` (written by hand until the lantern card's `lit_in` round lands, since the light act on main records none); then, reopened, the older-record lantern `O2`, a `lys.lantern` entry written with `Session::append_entry` as the child of `L2`, whose data is exactly `point` `e2`, `note` and `lit_by` and carries no `lit_in`; `e3` a `lys.harness_event` of kind `attachment` whose parent is `O2`, its `record` naming another block; `e4` user `fixture-text-4`; `e5` assistant `fixture-text-5`; `e6` user with one text part `fixture-text-6` followed by one image part; `e7` assistant `fixture-text-7`; `e8` user `fixture-text-8`; `e9` assistant `fixture-text-9`, each the child of the one before. Every text part of `e1` to `e9` is put through `BlockStore::put` as `serde_json::to_vec` of the part as it stands in the entry, as the importer stores a text part, and so are the `record` blocks of `s2` and `e3`; `e6`'s image part is not put. With the session closed, three more lanterns are then lit with the light act at the session's head: `L5` with point `e5`, `L6` with point `e6`, and `L1` with point `e1`. A second session `compacted` holds `e1` user, `e2` assistant, `c3` a compaction with `firstKeptEntryId` `e2`, `e4` user, `e5` assistant, in one chain, with a lantern `C5` lit at point `e5`. `L1`, `L2`, `L5`, `L6` and `C5` stand for the entry ids the light act returns. The eight `fixture-text-*` strings (`fixture-text-1`, `-2`, `-4`, `-5`, `-6`, `-7`, `-8`, `-9`) are the content sentinels: none may appear in a report, an error, a log line or a test name.

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

### R6: Render the child with its seed prompt and print its launch line

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

Structural: `docs/design/home/PROOF-FORK.md` records, as hashes, counts, paths, commands, versions and exit codes only: the lines where Pi at 3d5cbe98 reads `parentSession`; the Claude Code version printed by `claude --version` on the machine the proof runs on; a real Claude Code session imported into a scratch home with its source hash before and after; a lantern lit with `lys-home lantern light` at a past assistant entry; the fork's report; the child file loaded with Pi's `loadEntriesFromFile` at 3d5cbe98 and its entry count; the child rendered under a fresh uuid; the launch line run, stating whether it is the launch template's line or the printed resume-by-path line run by hand, with `-p` and one question that asks for a one-word answer, a word that appears in the parent's chain only at or before the point; the run's exit code, the continuation file's path, SHA-256 and record counts, and `lys-home resume-check` on it; the SHA-256 of the answer beside the SHA-256 of the expected answer, where the answer is the run's printed reply and both it and the expected word are trimmed of surrounding whitespace, stripped of one trailing full stop and lowercased before hashing; the parent's pre-fork prefix hash before and after; the block store's file count and bytes before and after; and the same for a second fork from a lantern at a user entry launched with its seed; and the number of text parts the leak check compared and the number it left out as empty after trimming or holding no letter and no digit, with no length bound on a compared part. THE SYSTEM SHALL NOT write a line of transcript content, a note, a question's or an answer's text into the proof, and SHALL NOT record a credential or a handle.

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
