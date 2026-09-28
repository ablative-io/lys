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
