---
type: brief
id: HOME-014
cluster: home
title: LanternData carries lit_in, the light act records it, recall reports it and the fork reads the typed field
---

# HOME-014: LanternData carries lit_in, the light act records it, recall reports it and the fork reads the typed field

> **Cluster:** home
> **Depends on:** HOME-004, HOME-006
> **Design anchor:**
> - ADR-014 — A lantern is a custom entry in its session, and its note grows only by epilogue entries — A lantern is a `lys.lantern` custom entry appended at its session's head, carrying in custom.data the entry id of its point (an existing entry of the same session that is not itself a lantern or an epilogue, the head or any entry the head has moved past), the note as written, who lit it and when. Its note grows only by `lys.lantern_epilogue` custom entries naming the lantern's entry id and carrying the further words, who added them and when; a lantern's story is its entry followed by its epilogues in order, and nothing is rewritten. Rejected: Pi's `label` entry on the target (it replaces or clears a label rather than growing one, and carries no author or time), a lantern store beside the session outside Pi's grammar (a lantern would stop travelling with its session), and editing the lantern's note in place (the record is append-only, P1).
> - ADR-015 — A lantern's note and epilogues are the person's annotations, not transcript — A lantern's note and its epilogues are the person's own annotations, not transcript. Recall prints them by design; they are the one exception to P7 and CN3, and only recall prints them. Transcript lines (message, tool and compaction content) still never appear in output, logs or errors, and a recall row never carries a line of the transcript around its point. Rejected: treating notes as transcript and printing only their hashes, which makes recall useless to the person who wrote them.
> - ADR-017 — A fork is a child session cut from the parent's own lines at a lantern's point, with its ancestry on both sides — A fork resolves a lantern to the session it was lit in, read from the lys.lantern data's lit_in when the record carries it and otherwise by the older-record rule (one holder cuts, several refuse lantern_ambiguous until a session is named), and cuts that session's root-to-point chain at the last assistant message at or before the point, through the index. The child is a new session under the parent's cwd whose header's parentSession is the parent file's path relative to the home, holding each cut entry as the parent file's own line bytes, then one lys.forked_from custom entry as its head naming the parent session, the lantern, the point, the cut entry, whether the coordinate was carried and the carried entry; the parent gains one lys.fork custom entry at its head naming the child. Nothing else is copied and no block is written. Rejected: re-serialising the copied entries (the copy would stop hash-matching the parent's lines), a fork store beside the sessions outside Pi's grammar, cutting at a point no lantern names, and a header field beyond Pi's parentSession.
> - ADR-046 — A lantern records the session it was lit in as an optional typed lit_in; a lantern without it is an older record resolved by its holders — LanternData gains an optional lit_in, the id of the session that held the point when the lantern was lit, and keeps refusing unknown keys; a present null and a present non-string value are kept distinct from an absent key. The light act writes it as the id of the session at whose head it appends the lantern and reports it; recall reports it beside point, note and lit_by, as null for a lantern whose data has no lit_in, and gives a lantern whose lit_in is present and not a session id the fork's lit_in_not_a_session refusal with the fork's reason text, skipped by note and refused by point, never a row and never entry_shape. The fork reads lit_in through a narrower typed view carrying point, lit_by and the optional lit_in, which requires neither lit_at nor note: a lantern whose data has no lit_in is an older record and resolves by its holders as ADR-017 rules (one holder cuts, several refuse lantern_ambiguous until a session is named), and a lit_in that is present but is null, not a string, not a safe session name, or names no session of the home is refused lit_in_not_a_session naming which, never read as any session. Rejected: making lit_in required (every lantern lit before it would become the wrong shape, skipped by recall by note and refused by recall by point); refusing a lantern with no lit_in by name (every lantern lit before it would become unforkable, retiring ADR-017's older-record rule); reading a null lit_in as an absent one (a silent fallback to the holder rule); the fork reading the full LanternData (the older records without lit_at would stop being forkable); and rewriting earlier lanterns to carry it (P1).
> **Checklist:**
> - C100 — LanternData carries an optional lit_in, the session that held the point when the lantern was lit, keeping a present null and a present non-string value distinct from an absent key, and still refuses an unknown key.
> - C101 — The light act writes lit_in as the id of the session at whose head it appends the lantern, and its report carries lit_in beside id, session, point and lit_at.
> - C102 — Recall rows carry lit_in beside point, note and lit_by, the lighting session for a lantern the light act lit and null for a lantern whose data has no lit_in, and a lantern whose lit_in is present and not a session id is never listed as a row but skipped by note and refused by point as lit_in_not_a_session with the fork's reason text.
> - C103 — The fork reads lit_in through a typed view that does not require lit_at or note and never from a raw JSON key: a lantern with no lit_in resolves by its holders, and a present lit_in that is null, not a string, not a safe session name, or names no session of the home refuses lit_in_not_a_session naming which.
> - C104 — The fork's fixtures light L2 with the light act and write by hand only the lanterns the light act cannot produce (O2, M2, N3, the copy N2 and N1 to N4), each named in its test as standing for such a record.
> - C105 — RECORD.md and the lys-home README document lit_in, and PROOF-FORK.md keeps its measured older-record sentence with a note that the light act now records lit_in.
> **Stories:**
> - S41 (Agent, Continues a session imported from Claude Code on Codex) — As an agent that lights a lantern, I want it to record the session I lit it in and recall to show that session beside the point and the note, so that I know which session the lantern leads back to.
> - S42 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a lantern lit before lit_in was recorded to stay readable and forkable by its holders, and a lit_in that is not a session id refused by name, so that no lantern is ever read as a session it does not name.
> - S43 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fork's tests to light their lanterns with the light act wherever it can produce them, so that the fork is proved on the record the light act really writes.

## Purpose

A fork has to know the session a lantern was lit in, because a copied line keeps its id and a fork's child holds a copy of every lantern on the copied chain. HOME-006 reads that session from a `lit_in` key the light act does not write, looked up in the entry's raw data as a stopgap, and its fixtures write the `lit_in` lantern `L2` by hand. This brief makes `lit_in` part of the lantern's typed shape: `LanternData` carries it, the light act records it, the light report and recall report it, and the fork reads the typed field, while every lantern lit before it stays readable and forkable by its holders (ADR-017, ADR-046).

## Task

Add an optional `lit_in` to `LanternData` in `crates/lys-home/src/record/entries.rs`, typed so that an absent key, a present JSON null and a present non-string value stay three distinct things. `light()` writes `lit_in` as the id of the session at whose head it appends the lantern, and its `Lit` report, printed by `lys-home lantern light`, carries `lit_in` beside `id`, `session`, `point` and `lit_at`. Recall's `LanternRow` carries `lit_in` beside `point`, `note` and `lit_by`, as JSON null for a lantern whose data has no `lit_in`. `fork_cut.rs` reads `point` and `lit_in` through a narrower typed view in place of raw `serde_json::Value` lookups. Both fork fixtures light `L2` with the light act. In scope: `entries.rs`, `lantern.rs`, `recall.rs`, `fork_cut.rs`, their tests, `fork_tests.rs`, `tests/lantern_cli.rs`, `tests/fork.rs`, `docs/design/home/RECORD.md`, the lys-home README, a note in `docs/design/home/PROOF-FORK.md`, and the cluster's rendered markdown.

A correction to the request, as ruled: the request says a lantern record without `lit_in` is refused by name as it is today, and the tree does not refuse it. A lantern whose data has no `lit_in` key is an older record and resolves by its holders as ADR-017 and HOME-006's third amendment rule: one holder cuts, several refuse `lantern_ambiguous` until `--session` names one. Only a `lit_in` that is present and not a session id is refused by name (`lit_in_not_a_session`), never read as any session. ADR-017 is not retired, and every lantern lit before this brief stays forkable. `lit_in` is optional: a lantern lit before this brief is read as it is, never as the wrong shape; recall shows its `lit_in` as null; recall by note skips nothing new and recall by point refuses nothing new for want of `lit_in`.

The fork's typed read, as ruled: the fork reads a lantern through a narrower typed view carrying `point`, `lit_by` and an optional `lit_in`, which does not require `lit_at` or `note`, so the older records `O2`, `M2` and `N3`, whose data is exactly `{point, note, lit_by}`, read as they do today. `LanternData` keeps its full shape for lighting and recall.

The fixtures, as ruled: the rule that the fork's fixtures contain no hand-written lantern entry covers every lantern the light act can produce, so `L2` is lit with the light act in both fork fixtures. The records the light act can no longer write stay hand-written, and this is the rule's one exception: the older records `O2` (in both fixtures), `M2` and `N3` with no `lit_in`, the copy `N2` whose `lit_in` names a session other than the one it stands in, and `N1` to `N4` whose `lit_in` is null, 5, `../elsewhere` and `no-such-session`. Each is named in its test as standing for a record the light act cannot produce.

The tests that change, as ruled: the request's rule that no other existing lantern or fork test changes means that no test changes except those named here and those whose fixtures this brief changes. `lantern_data_round_trips_with_exactly_its_four_keys` becomes a five-key round trip with `lit_in`; `a_row_carries_exactly_its_fields` gains `lit_in`; the light report's key pin in `tests/lantern_cli.rs` gains `lit_in`. A test whose `L2` is now lit by the light act asserts the id the light act returns in place of the literal `"L2"`, at each of the twelve sites in `fork_cut_tests.rs`, `fork_tests.rs` and `tests/fork.rs`, and the module docs of `fork_cut_tests.rs` and `tests/fork.rs` say that `L2` is lit with the light act; what each of those tests asserts is otherwise unchanged. New tests are added for what this brief delivers.

Recall's refusal, as ruled: a lantern whose `lit_in` is present and not a session id is refused in recall as `lit_in_not_a_session`, with the fork's four reason texts, both as the skip reason by note and as the refusal by point, and is never listed as a row. Recall's other `entry_shape` cases keep their name.

Out of scope: re-measuring PROOF-FORK on a real session whose lanterns carry `lit_in`; recall telling a lantern lit in a session from a copy a fork carried in; and retiring the older-record holder rule. The brief is ordered after HOME-006, which is on main, so it starts on the current main.

## Requirements

### R1: Give LanternData an optional lit_in that keeps null and non-string values distinct from an absent key

Structural. `LanternData` in `record/entries.rs` gains a documented fifth field `lit_in: Option<LitIn>`: the session that held the point when the lantern was lit. `LitIn` is a documented public enum declared beside it, `#[serde(untagged)]`, with `LitIn::Session(String)` for a JSON string and `LitIn::Other(serde_json::Value)` for any other JSON value, null included. The field carries `#[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = ...)]` naming a crate function that wraps whatever value is present in `Some`, so an absent key reads as `None` and a present null reads as `Some(LitIn::Other(Value::Null))`. `LanternData` keeps `#[serde(deny_unknown_fields)]`, and `EpilogueData` does not change. THE SYSTEM SHALL NOT read a present null as an absent key, SHALL NOT make `lit_in` required, SHALL NOT refuse a lantern whose data has no `lit_in`, and SHALL NOT rename or retype `point`, `note`, `lit_by` or `lit_at`.

**Acceptance:**
- `serde_json::from_value::<LanternData>(json!({"point": "e2", "note": NOTE, "lit_by": LIGHTER, "lit_at": now()}))` is `Ok` with `lit_in == None`.
- The same object with `"lit_in": null` added is `Ok` with `lit_in == Some(LitIn::Other(Value::Null))`.
- The same object with `"lit_in": 5` added is `Ok` with `lit_in == Some(LitIn::Other(json!(5)))`.
- The same object with `"lit_in": "parent"` added is `Ok` with `lit_in == Some(LitIn::Session("parent".to_owned()))`.
- `lantern_data_round_trips_with_exactly_its_five_keys` builds a `LanternData` with `lit_in: Some(LitIn::Session(FIXTURE.to_owned()))`; its sorted serialised keys equal `["lit_at", "lit_by", "lit_in", "note", "point"]` and it deserialises back equal; no test named `lantern_data_round_trips_with_exactly_its_four_keys` remains.
- A `LanternData` with `lit_in: None` serialises to exactly the keys `["lit_at", "lit_by", "note", "point"]`.
- `lantern_data_with_an_extra_key_is_refused` passes unchanged: data carrying `"anchors": []` is `Err`.
- `git diff` of `record/entries.rs` changes no line of `EpilogueData`.

**Files:**
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/record/lantern_tests.rs

**Checklist:**
- C100 — LanternData carries an optional lit_in, the session that held the point when the lantern was lit, keeping a present null and a present non-string value distinct from an absent key, and still refuses an unknown key.

**Stories:**
- S41 (Agent, Continues a session imported from Claude Code on Codex) — As an agent that lights a lantern, I want it to record the session I lit it in and recall to show that session beside the point and the note, so that I know which session the lantern leads back to.

### R2: Record lit_in in the light act, print it in the light report, and check a recorded lit_in in one place

WHEN `light(home, session, point, note, by)` appends a lantern, THE SYSTEM SHALL write its data's `lit_in` as `Some(LitIn::Session(session))`, the id of the session at whose head the lantern is appended, and SHALL return a `Lit` report carrying a documented `lit_in: String` field equal to that id beside `id`, `session`, `point` and `lit_at`; `lys-home lantern light` prints that report. `record/lantern.rs` also holds the one `pub(crate)` check of a recorded `lit_in`, taking the home, the lantern id and a `&LitIn` and returning the session id it names. IF the recorded `lit_in` is null, not a string, not a safe session name, or names no session file of the home THEN the check SHALL return `HomeError::LitInNotASession` naming the lantern with `what` equal to `is null`, `is not a string`, `is not a safe session name` and `names no session of the home` respectively. THE SYSTEM SHALL NOT take `lit_in` from an argument other than the session `light` owns, from the point entry, or from any other session, SHALL NOT change the order of `light`'s checks or its refusals, SHALL NOT put the note in the report, and SHALL NOT change a variant, a field or an `#[error]` text in `error.rs`.

**Acceptance:**
- A new test in `lantern_tests.rs`: `light(&home, FIXTURE, "e2", NOTE, LIGHTER)` returns a `Lit` whose `lit_in == FIXTURE`, and the appended entry's data read as `LanternData` has `lit_in == Some(LitIn::Session(FIXTURE.to_owned()))`.
- In `tests/lantern_cli.rs`, the light report's sorted keys equal `["id", "lit_at", "lit_in", "point", "session"]` and `report["lit_in"] == FIXTURE`; the test's other assertions are unchanged.
- `git diff` of `crates/lys-home/src/error.rs` is empty.
- Every other test in `lantern_tests.rs` and `tests/lantern_cli.rs` passes unchanged.

**Files:**
- modify: crates/lys-home/src/record/lantern.rs
- modify: crates/lys-home/src/record/lantern_tests.rs
- modify: crates/lys-home/tests/lantern_cli.rs

**Checklist:**
- C101 — The light act writes lit_in as the id of the session at whose head it appends the lantern, and its report carries lit_in beside id, session, point and lit_at.

**Stories:**
- S41 (Agent, Continues a session imported from Claude Code on Codex) — As an agent that lights a lantern, I want it to record the session I lit it in and recall to show that session beside the point and the note, so that I know which session the lantern leads back to.

### R3: Report lit_in in recall's rows, null for a lantern without it, and refuse a lit_in that is not a session id by name

`LanternRow` in `record/recall.rs` gains a documented `lit_in: Option<String>`, serialised without `skip_serializing_if`, so every row carries the key beside `point`, `note` and `lit_by`; `rows_of` takes the home so it can run R2's check. WHEN recall by note or by point lists a lantern whose data has no `lit_in`, THE SYSTEM SHALL list it with `lit_in` null. WHEN it lists a lantern whose `lit_in` is present, THE SYSTEM SHALL run R2's check and list the session id it returns. IF that check refuses THEN recall by note SHALL skip the lantern's session and name it in `skipped` with that refusal's display text, and recall by point SHALL return that refusal. THE SYSTEM SHALL NOT give such a lantern `entry_shape`, SHALL NOT change the name of recall's other `entry_shape` refusals, SHALL NOT list a lantern whose `lit_in` is present and not a session id as a row, SHALL NOT skip or refuse a lantern for having no `lit_in`, and SHALL NOT print a line of the transcript (ADR-015).

**Acceptance:**
- `a_row_carries_exactly_its_fields` asserts row keys `["epilogues", "id", "lit_at", "lit_by", "lit_in", "note", "point", "session"]`, and still counts 2 rows and 2 epilogues.
- A new test: in `recall_fixture`, `recall_by_point(&home, OTHER, "o1")` lists one row whose `lit_in == Some(OTHER.to_owned())`, and every row `recall_by_note(&home, "fold")` lists has `lit_in == Some(row.session.clone())`.
- A new test: a lantern appended with `Session::append_entry` whose data is exactly `{point, note, lit_by, lit_at}` recalls by its point as one row with `lit_in == None`, serialised as `"lit_in": null`, and `recall_by_note` on a phrase of its note lists it with `skipped` empty.
- A new test: four sessions each holding one lantern whose `lit_in` is null, 5, `"../elsewhere"` and `"no-such-session"`; `recall_by_note` on the lanterns' note lists those four sessions in `skipped` and no row from them, each skip's `reason` starting `lit_in_not_a_session` and containing `is null`, `is not a string`, `is not a safe session name` and `names no session of the home` respectively; `recall_by_point` at each lantern's point is `Err(HomeError::LitInNotASession { .. })` with `what` equal to those four texts respectively; the test counts 4 skips and 4 refusals, and no skip reason starts `entry_shape`.
- `a_lantern_whose_data_is_not_the_shape_is_named_not_dropped` passes unchanged, its refusal still `HomeError::EntryShape`.
- Every other test in `recall_tests.rs` passes unchanged.

**Files:**
- modify: crates/lys-home/src/record/recall.rs
- modify: crates/lys-home/src/record/recall_tests.rs

**Checklist:**
- C102 — Recall rows carry lit_in beside point, note and lit_by, the lighting session for a lantern the light act lit and null for a lantern whose data has no lit_in, and a lantern whose lit_in is present and not a session id is never listed as a row but skipped by note and refused by point as lit_in_not_a_session with the fork's reason text.

**Stories:**
- S41 (Agent, Continues a session imported from Claude Code on Codex) — As an agent that lights a lantern, I want it to record the session I lit it in and recall to show that session beside the point and the note, so that I know which session the lantern leads back to.
- S42 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a lantern lit before lit_in was recorded to stay readable and forkable by its holders, and a lit_in that is not a session id refused by name, so that no lantern is ever read as a session it does not name.

### R4: Read lit_in in the fork through a narrower typed view, keeping the holder rule and the named refusals

`read_lantern` in `record/fork_cut.rs` deserialises the lantern entry's data into a documented public view declared in `record/entries.rs` beside `LanternData`, carrying `point: String`, `lit_by: String` and a `lit_in` declared with R1's `LitIn` type and R1's deserialiser; the view does not deny unknown keys and does not require `note` or `lit_at`, so the older records whose data is exactly `{point, note, lit_by}` read as they do today. A present `lit_in` is checked with R2's check. IF the data has no `lit_in` THEN THE SYSTEM SHALL resolve by the holders exactly as today: one holder cuts, several refuse `lantern_ambiguous` until a session is named. IF `lit_in` is present and the check refuses THEN THE SYSTEM SHALL return that `lit_in_not_a_session` refusal. IF the data does not deserialise into the view THEN THE SYSTEM SHALL refuse `EntryShape` naming the session, the lantern and `lys.lantern`. The module doc of `fork_cut.rs` describes the typed read. THE SYSTEM SHALL NOT look up a `lit_in` key in a raw `serde_json::Value`, SHALL NOT read a present `lit_in` that is not a session id as any session, SHALL NOT refuse a lantern for having no `lit_in`, and SHALL NOT change the cut, the copied line bytes, the ancestry entries, the fork report, or the names and display prefixes of `lit_in_not_a_session`, `lantern_not_lit_here` and `lantern_ambiguous`.

**Acceptance:**
- `grep -c '"lit_in"' crates/lys-home/src/record/fork_cut.rs` prints `0`, and `grep -c 'Value::Null' crates/lys-home/src/record/fork_cut.rs` prints `0`.
- `a_lit_in_that_is_not_a_session_id_is_refused_by_name` passes with the refusals it counts on the base commit: `N1` gives `is null`, `N2` `is not a string`, `N3` `is not a safe session name` and `N4` `names no session of the home`, each display text starting `lit_in_not_a_session`.
- `a_lit_in_session_that_holds_no_copy_refuses_naming_the_holder_read` passes with the refusals it counts on the base commit, each `LanternNotLitHere` with `lit_in == "elsewhere"`.
- In `a_lit_in_lantern_cuts_from_its_session_and_an_older_record_needs_one_named`, `O2` with no session named refuses `LanternAmbiguous` with `sessions == ["A", PARENT]`, and with `PARENT` named cuts from `PARENT` with `lit_in == None`.
- `fork(&home, "M2", None)` still reports `(entries, blocks, unstored) == (2, 1, 2)`, and every test in `fork_tests.rs` that forks `N3` passes, though neither lantern's data carries `lit_at` and neither carries `lit_in`.

**Files:**
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/record/fork_cut.rs

**Checklist:**
- C103 — The fork reads lit_in through a typed view that does not require lit_at or note and never from a raw JSON key: a lantern with no lit_in resolves by its holders, and a present lit_in that is null, not a string, not a safe session name, or names no session of the home refuses lit_in_not_a_session naming which.

**Stories:**
- S42 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a lantern lit before lit_in was recorded to stay readable and forkable by its holders, and a lit_in that is not a session id refused by name, so that no lantern is ever read as a session it does not name.

### R5: Light L2 with the light act in both fork fixtures and name every hand-written lantern as a record the light act cannot produce

In `fixture_home` of `record/fork_cut_tests.rs`, `L2` is lit with `light(&home, PARENT, "e2", NOTE, LIGHTER)` while `e2` is the parent's head and no owner holds the session, and in `fixture_home` of `tests/fork.rs` with `lys-home lantern light` at `--session parent --point e2`; `Lanterns.l2` holds the returned id and `O2` is appended as its child. The lanterns the fork fixtures write by hand are exactly those the light act cannot produce: `O2` in both fixtures, `M2` and `N3` in `fork_tests.rs`, `N2` and `N1` to `N4` in `fork_cut_tests.rs`, each call site carrying a comment containing `the light act cannot produce`. Each of the twelve sites that asserted the literal `"L2"` asserts the id the light act returned. The module docs of `fork_cut_tests.rs` and `tests/fork.rs` say that `L2` is lit with the light act. THE SYSTEM SHALL NOT write by hand any lantern the light act can produce, SHALL NOT drop or weaken a hand-written refusal or older-record case, and SHALL NOT change any assertion, count or refusal beyond the literal-id replacement.

**Acceptance:**
- `grep -c '"L2"'` prints `0` for each of `crates/lys-home/src/record/fork_cut_tests.rs`, `crates/lys-home/src/record/fork_tests.rs` and `crates/lys-home/tests/fork.rs`.
- `grep -c 'lantern_entry('` prints `4` for `fork_cut_tests.rs` (the helper and the `O2`, `N2` and `N1` to `N4` sites), `2` for `fork_tests.rs` (`M2`, `N3`) and `2` for `tests/fork.rs` (the helper and `O2`).
- `grep -c 'the light act cannot produce'` prints `3` for `fork_cut_tests.rs`, `2` for `fork_tests.rs` and `1` for `tests/fork.rs`.
- `five_forks_succeed_and_five_are_refused_with_the_ancestry_on_the_parent` passes with 5 children, 5 refusals and 5 `lys.fork` marks, its `lantern_not_lit_here` refusal naming `lanterns.l2`.
- The count of `#[test]` functions in `fork_cut_tests.rs`, `fork_tests.rs` and `tests/fork.rs` equals the base commit's: 8, 12 and 3.

**Files:**
- modify: crates/lys-home/src/record/fork_cut_tests.rs
- modify: crates/lys-home/src/record/fork_tests.rs
- modify: crates/lys-home/tests/fork.rs

**Checklist:**
- C104 — The fork's fixtures light L2 with the light act and write by hand only the lanterns the light act cannot produce (O2, M2, N3, the copy N2 and N1 to N4), each named in its test as standing for such a record.

**Stories:**
- S43 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fork's tests to light their lanterns with the light act wherever it can produce them, so that the fork is proved on the record the light act really writes.

### R6: Document lit_in in the record, the README and the fork proof, and render the cluster

Structural. The `lys.lantern` bullet of `docs/design/home/RECORD.md` lists `{point, note, lit_by, lit_at, lit_in}` and states that `lit_in` is the id of the session the light act appended the lantern to, absent from a lantern lit before HOME-014, which recall shows as null; its Forks section states that a lantern with no `lit_in` resolves by its holders, that a present `lit_in` that is not a session id is refused `lit_in_not_a_session` by the fork and by recall, and that a fork's child lists a copy of the lantern whose `lit_in` names the parent. The `lys.lantern` row of the custom-entry table in `crates/lys-home/README.md` lists `lit_in`. `docs/design/home/PROOF-FORK.md` keeps its measured sentence that the light act on the measured tree records no `lit_in` byte for byte, and gains a note directly after it that from HOME-014 the light act records `lit_in`, and that lanterns lit before it, as those measured were, resolve by the older-record rule. `HOME-014.md`, `DESIGN.md`, `CHECKLIST.md` and `USER-STORIES.md` are rendered from their JSON with the repository's `scripts/design/render-cluster.py`. THE SYSTEM SHALL NOT rewrite any measured figure, hash, command or result in PROOF-FORK.md, and SHALL NOT edit any rendered markdown by hand.

**Acceptance:**
- `grep -c 'point, note, lit_by, lit_at, lit_in' docs/design/home/RECORD.md` prints `1`.
- `grep -c 'lit_in_not_a_session' docs/design/home/RECORD.md` prints a number greater than `0`.
- The README line beginning `| \`lys.lantern\`` contains `lit_in`.
- `git diff` of `docs/design/home/PROOF-FORK.md` deletes no line, and its added lines contain `HOME-014` and `lit_in`.
- `sh scripts/design/gate.sh` exits 0.

**Files:**
- create: docs/design/home/briefs/HOME-014.md
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/README.md
- modify: docs/design/home/PROOF-FORK.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md

**Checklist:**
- C105 — RECORD.md and the lys-home README document lit_in, and PROOF-FORK.md keeps its measured older-record sentence with a note that the light act now records lit_in.

**Stories:**
- S42 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a lantern lit before lit_in was recorded to stay readable and forkable by its holders, and a lit_in that is not a session id refused by name, so that no lantern is ever read as a session it does not name.

## Boundaries

- SHALL NOT rewrite, truncate or delete any byte of an existing session line; a lantern lit before this brief stays as it is (P1, ADR-014).
- SHALL NOT refuse, skip or read as the wrong shape a lantern whose data has no lit_in, and SHALL NOT retire ADR-017's older-record rule.
- SHALL NOT change the names or display prefixes of lit_in_not_a_session, lantern_not_lit_here, lantern_ambiguous or entry_shape, SHALL NOT change the four lit_in_not_a_session reason texts, and SHALL NOT change error.rs.
- SHALL NOT change the fork's cut, its copied line bytes, its lys.forked_from and lys.fork data, or its report keys.
- SHALL NOT change EpilogueData or the epilogue act, and SHALL NOT remove deny_unknown_fields from LanternData.
- SHALL NOT drop or weaken the hand-written O2, M2, N3, N2 and N1 to N4 cases.
- SHALL NOT put a note, epilogue words or any transcript content in a refusal, a report other than recall's rows, a log line, a test name or a proof document (ADR-015).
- SHALL NOT mark or filter in recall a lantern copied into a fork's child, and SHALL NOT re-measure PROOF-FORK on a real session.
- SHALL NOT edit rendered markdown by hand, and SHALL NOT add a file beyond HOME-014.md.

## Verification

- From the repository root: `python3 scripts/design/validate.py docs/design/home` and `python3 scripts/design/check-coverage.py docs/design/home` exit 0.
- From the repository root: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --all-features`, `cargo doc --no-deps --all-features` and `cargo doc --no-deps` exit 0.
- From the repository root: `sh scripts/design/gate.sh` exits 0.
- `crates/lys-home/src/record/entries.rs`, `lantern.rs`, `recall.rs` and `fork_cut.rs` each have at most 500 lines of code, counted without comments and blank lines.
- `cargo test -p lys-home --all-features --test fork --test lantern_cli` passes, and its output lists the five-forks test and the light report test as passed.
