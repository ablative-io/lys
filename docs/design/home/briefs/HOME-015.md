---
type: brief
id: HOME-015
cluster: home
title: Hand an outgoing session's letter to a new successor home as inherited memory
---

# HOME-015: Hand an outgoing session's letter to a new successor home as inherited memory

> **Cluster:** home
> **Depends on:** HOME-001, HOME-004
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-016 — A rendered file's derived uuids are a fixed, versioned contract — Derive every uuid a render needs and the record does not hold as UUIDv5 under the session's namespace and the name `<entry id>#<role>`, with a closed set of roles (`record` first); the session's namespace is UUIDv5 over one fixed lys namespace (32c05904-d1f1-550c-9eee-2f6c8f98b665, itself UUIDv5 of the RFC 9562 URL namespace over `lys/home/claude-code/render-uuid/v1`) and the id of the session being rendered, so the same entry id in two sessions never derives one uuid. Treat the namespace, the session namespace rule, the name form and the roles as frozen: a change is a new version alongside, never a mutation. The fork and launch cards derive by this scheme. Rejected: drawing fresh ids (nondeterministic), keying on a hash of the entry id alone without a namespace, a role and a session (two roles on one entry collide, two sessions with one hand-authored id collide, and it is not reproducible with standard UUID tooling), mixing in the target session id (the record does not hold it), and leaving the scheme mutable until a later card (every recorded hash would move with it).
> - ADR-017 — A fork is a child session cut from the parent's own lines at a lantern's point, with its ancestry on both sides — A fork resolves a lantern to the session it was lit in, read from the lys.lantern data's lit_in when the record carries it and otherwise by the older-record rule (one holder cuts, several refuse lantern_ambiguous until a session is named), and cuts that session's root-to-point chain at the last assistant message at or before the point, through the index. The child is a new session under the parent's cwd whose header's parentSession is the parent file's path relative to the home, holding each cut entry as the parent file's own line bytes, then one lys.forked_from custom entry as its head naming the parent session, the lantern, the point, the cut entry, whether the coordinate was carried and the carried entry; the parent gains one lys.fork custom entry at its head naming the child. Nothing else is copied and no block is written. Rejected: re-serialising the copied entries (the copy would stop hash-matching the parent's lines), a fork store beside the sessions outside Pi's grammar, cutting at a point no lantern names, and a header field beyond Pi's parentSession.
> - ADR-058 — A handover is a rule-less lys.inherited entry and the letter's turn copied whole into a new successor home, its inheritance read from its first entry — The letter is one or more entry ids, a run of assistant message entries standing next to each other on the outgoing session's path, in path order. Each entry is copied whole, keeping its id, its timestamp and its message, with only its parent link rewritten to chain onto the successor, and from_entries lists every id in that order. The successor is a new home directory, absent or empty, holding one new session with a fresh id, the outgoing header's cwd and no parentSession. Its first entry is a lys.inherited entry with no rule, then the copied entries, then a session_info whose name is `inherited from <outgoing session id>`, and no field is added to Pi's grammar. The first entry is what says the successor's first memory is inherited; the handover's own report reads it from there and prints inherited true, and the render report and its count of canon examples are unchanged. The canon loader newly refuses by name a canon example whose lys.inherited entry has no rule; a lys.inherited entry in a session of a home is read without that check. Rejected: merging the run into one message (composing the letter), copying one entry (half a letter), a session_info flag (breaks CN4), writing into the outgoing home, and setting parentSession, which ADR-017 keeps for a fork's ancestry.
> **Checklist:**
> - C45 — The shared helpers and constants safe_component, MAX_NAME_BYTES, PI_FORMAT_VERSION, now, fresh_id, json_len, write_durable and custom_type_of are defined in crates/lys-home/src/record/helpers.rs.
> - C46 — Session, its impl, take_lock, load_checked and to_line are defined in crates/lys-home/src/record/session.rs.
> - C47 — Home and its impl are defined in crates/lys-home/src/record/home.rs.
> - C48 — crates/lys-home/src/record/fork.rs imports write_durable and custom_type_of from crate::record::helpers and changes no other line.
> - C49 — crates/lys-home/src/record/mod.rs holds only module docs, pub mod and mod lines with their cfg(test) attributes, and pub use lines, and the item grep HOME-013 names prints nothing on it.
> - C50 — Every public path lys_home::record::{Home, Session, safe_component, now, fresh_id, json_len, MAX_NAME_BYTES, PI_FORMAT_VERSION} and lys_home::{Home, Session} resolves as before the move, and no crate outside lys-home changes.
> - C51 — Every test that passed before the move passes unchanged with an equal count, no test file changes beyond use lines, no non-test source file in crates/lys-home is over 500 lines of code, and the gate legs pass.
> - C52 — PROOF-HANDOVER.md records the handover of an elicited letter by ids, hashes and the signature comparison, whether the inherited signed block appears in a resumed continuation's own file on the installed Claude Code beside 2.1.281, and the seeded and plain card counts as not run.
> **Stories:**
> - S2 (Agent, Continues a session imported from Claude Code on Codex) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.
> - S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.
> - S8 (Agent, Continues a session imported from Claude Code on Codex) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

## Purpose

An outgoing session writes one letter as its last turn, in its own real thinking and answer, to the session that wakes up after it (P10). This brief builds HOME-001 R12, the handover, which carries that letter into a new successor home as inherited memory, so the successor starts from what the outgoing session learned and never from a summary someone else wrote. The letter enters under a lys.inherited entry with no rule, since a letter is not a rule, and is copied whole with its signed thinking; R4's rule decides what the inherited thinking becomes when rendered for another model (ADR-058).

## Task

Add `lys-home handover --home <dir> --from <session> --letter <entry id>... --successor <dir>`. It reads the outgoing session without owning it and takes the letter as one or more entry ids, a run of assistant message entries standing next to each other on the path, in path order; HOME-001 R12's single entry id is corrected to that list. It refuses by name before anything is written: a letter entry that is not an assistant message, a run that is not contiguous on the root-to-head path, an entry whose message carries provider `authored`, a letter with no thinking block, and a successor path that exists and is not an empty directory. It then writes a new successor home holding one session with a fresh id, the outgoing header's cwd and no parentSession: a rule-less lys.inherited entry, then each letter entry copied whole with only its parent link rewritten, then a session_info named `inherited from <outgoing session id>`. No field is added to session_info (CN4): the first entry is what says the successor's first memory is inherited, and R12's `session_info says inherited` is corrected to that. The command prints one JSON report naming the successor home, the session id and its file, with `inherited` true read from the successor's first entry; the render report and its count of canon examples stay unchanged. `Inherited.rule` becomes optional. Today the canon reader never reads a lys.inherited entry's data, so a canon example without a rule is not refused now; this card adds that refusal to `canon::load`, which `canon add` and a render with `--canon` both reach, and only a handover's own entry in a session of a home may omit the rule. In scope: error.rs, record/entries.rs, record/canon.rs, record/canon_tests.rs (exactly two lines: `inherited.rule.as_str()` becomes `inherited.rule.as_deref()`, and the expected `"verify before claiming"` becomes `Some("verify before claiming")`), record/mod.rs (exactly the two lines `pub mod handover;` and `mod handover_tests;`, and no other code), record/handover.rs, record/handover_tests.rs, cli.rs, RECORD.md, PROOF-HANDOVER.md, design.json and the cluster's rendered markdown. A parallel card is splitting record/mod.rs; whichever lands second carries the other's module lines forward. The acceptance runs on a letter elicited for the purpose on the build machine, not the walrus turn; the unit tests use a synthetic fixture. If the build machine's harness writes no signed thinking for that turn, the proof records that as the finding, the byte-equal line is not met, and the card is not closed on it. The replay measurement is carry-over into the continuation's own file only; whether the block was sent to the model waits on R10's proxy. The seeded and plain card counts are recorded as not run, because HOME-002 adds no handover slot to the launch template. Out of scope: the little proxy (R10); a handover slot in the launch template; eliciting the letter at compaction or retirement from the harness; the seeded and plain card runs; any change to render.rs or its report; any canon change beyond the optional rule and its refusal; any change to how thinking blocks are signed or verified. C13 stays with HOME-001, and this brief delivers the handover through C45 to C52 instead; stories S2, S5 and S8 are shared with HOME-001, and this brief serves them for the handover only. The fixture home, used by R2 to R6, holds one session `outgoing` with cwd `/work/outgoing`, built with `Session::append_entry` under fixed ids, each entry's timestamp the string `fixture-time-<id>`, appended in this order. `u1`: user message, no parent, one text part `fixture-text-u1`. `a1`: assistant message, parent `u1`, provider `anthropic`, api `anthropic-messages`, model `claude-fixture-model`, content one part `{"type":"thinking","thinking":"fixture-thinking-a1","thinkingSignature":"fixture-signature-a1"}`. `a2`: assistant, parent `a1`, the same provider, api and model, one text part `fixture-text-a2`. `u2`: user, parent `a2`, one text part `fixture-text-u2`. `a3`: assistant, parent `u2`, the same provider, api and model, one text part `fixture-text-a3`. `b1`: assistant, parent `a1`, the same provider, api and model, content one part `{"type":"thinking","thinking":"fixture-thinking-b1","thinkingSignature":"fixture-signature-b1"}`, a side branch off the path. `x1`: assistant, parent `a3`, provider, api and model `authored`, one text part `fixture-text-x1`. `x1` is appended last, so the head is `x1` and the root-to-head path is `u1`, `a1`, `a2`, `u2`, `a3`, `x1`, with `b1` off it. No test name contains `fixture-`.

## Requirements

### R1: Name the handover's refusals and the successor's session name

Structural. `error.rs` gains six named refusals. Each message begins with its name, names what the refusal names, and carries no transcript content. `HomeError::LetterNotAssistant { session, id }` begins `letter_not_assistant` and names the entry. `HomeError::LetterNotContiguous { session, id }` begins `letter_not_contiguous` and names the letter entry that does not follow the one before it on the path. `HomeError::LetterAuthored { session, id }` begins `letter_authored` and names the entry. `HomeError::LetterWithoutThinking { session, ids }` begins `letter_without_thinking` and names every letter entry. `HomeError::SuccessorNotEmpty { path }` begins `successor_not_empty` and names the path. `HomeError::CanonExampleWithoutRule { id }` begins `canon_example_without_rule`, names the example's entry id and names the missing field `rule`. `record/entries.rs` documents `CUSTOM_INHERITED` as the type of both a canon example and a handover, and declares `INHERITED_FROM`, the session_info name prefix a handover writes, as `inherited from ` with one trailing space. THE SYSTEM SHALL NOT add a Pi entry type, a header field or any field outside a custom entry's `data`. THE SYSTEM SHALL NOT carry a letter's text, thinking or signature in a refusal.

**Acceptance:**
- `HomeError::LetterNotAssistant { session: "outgoing".into(), id: "u1".into() }.to_string()` begins with `letter_not_assistant` and contains `u1`.
- `HomeError::LetterNotContiguous { session: "outgoing".into(), id: "a3".into() }.to_string()` begins with `letter_not_contiguous` and contains `a3`.
- `HomeError::LetterAuthored { session: "outgoing".into(), id: "x1".into() }.to_string()` begins with `letter_authored` and contains `x1`.
- `HomeError::LetterWithoutThinking { session: "outgoing".into(), ids: vec!["a3".into(), "a4".into()] }.to_string()` begins with `letter_without_thinking` and contains `a3` and `a4`.
- `HomeError::SuccessorNotEmpty { path: PathBuf::from("successor-dir") }.to_string()` begins with `successor_not_empty` and contains `successor-dir`.
- `HomeError::CanonExampleWithoutRule { id: "k1".into() }.to_string()` begins with `canon_example_without_rule` and contains `k1` and `` `rule` ``.
- `crate::record::entries::INHERITED_FROM == "inherited from "`.
- `git diff 7b53625 -- crates/lys-home/src/record/entries.rs` adds no field to `SessionHeader`, `EntryBase` or any `EntryBody` variant.

**Files:**
- modify: crates/lys-home/src/error.rs
- modify: crates/lys-home/src/record/entries.rs

**Checklist:**
- C46 — Session, its impl, take_lock, load_checked and to_line are defined in crates/lys-home/src/record/session.rs.

**Stories:**
- S8 (Agent, Continues a session imported from Claude Code on Codex) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each refusal carries only session ids, entry ids or a path; the gate each_refusal_begins_with_its_name_and_names_what_it_refuses checks every acceptance line and INHERITED_FROM.
- Deviation: Built by hand on hand/HOME-015 at the lead's direction, not through card_build_v3; clippy and tests ran on the build machine.
- Files changed:
  - modified: `crates/lys-home/src/error.rs` — six named refusals, each beginning with its name and naming ids or the path only
  - modified: `crates/lys-home/src/record/entries.rs` — CUSTOM_INHERITED documented for a canon example and a handover; INHERITED_FROM declared; no field added to the header, EntryBase or any EntryBody variant
- Checklist delivery:
  - [x] C46 — A handover refuses by name before any file is created: a letter entry that is not an assistant message, a letter run whose entries do not stand next to each other on the outgoing path in the order given, a letter entry whose message carries provider `authored`, a letter holding no thinking block, and a successor path that exists and is not an empty directory. — met; see how
- Story delivery:
  - [x] S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs. — served for the handover

### R2: Let a handover's lys.inherited entry carry no rule, and make the canon refuse an example without one

Structural, then behaviour. In `record/canon.rs`, `Inherited.rule` becomes `Option<String>`, defaulted when absent and skipped when `None`, its place among the fields unchanged; `add_from` and `add_authored` write `Some` of the rule they are given. In `record/canon_tests.rs` exactly two lines change so the existing gate compiles against the optional rule: the line that reads `inherited.rule.as_str()` becomes `inherited.rule.as_deref()`, and the line of the expected tuple holding `"verify before claiming"` becomes `Some("verify before claiming")`. `record/mod.rs` gains exactly two lines, `pub mod handover;` and `mod handover_tests;`, and no other code; `record/handover.rs` is created holding only its `//!` module documentation, which states the handover's invariants (the letter copied whole, only its parent link rewritten, nothing written under the outgoing home), so the `pub mod handover;` line compiles here and the later requirements of this brief fill it; `record/handover_tests.rs` opens with the inner attribute `#![cfg(test)]` so that its declaration needs no attribute line in `record/mod.rs`. The new gates of this requirement live in `record/handover_tests.rs`. Today `canon::load` never reads a `lys.inherited` entry's data; this requirement adds that read. WHEN the canon is loaded, IF a `lys.inherited` entry's data carries no `rule` key, THEN THE SYSTEM SHALL refuse with `HomeError::CanonExampleWithoutRule` naming that entry's id, as the sixth named refusal, and SHALL write nothing, so `canon add` and a render with `--canon` both refuse through it. The reader tells the two kinds apart by where the entry stands: every `lys.inherited` entry in the canon file is a canon example and needs a rule; a `lys.inherited` entry in a session of a home is read without that check, so only a handover's own entry may omit `rule`. THE SYSTEM SHALL NOT change the bytes a canon entry with a rule serialises to. THE SYSTEM SHALL NOT change `canon/canon.jsonl`, any line of `record/canon_tests.rs` other than those two, or any other canon behaviour. THE SYSTEM SHALL NOT refuse a rule-less `lys.inherited` entry read from a session of a home. THE SYSTEM SHALL NOT put any code other than those two module lines into `record/mod.rs`.

**Acceptance:**
- `serde_json::to_string(&Inherited { authored: false, from_session: Some("s".into()), from_entries: vec!["e".into()], provider: "p".into(), api: "a".into(), model: "m".into(), curated_at: "t".into(), curated_by: "b".into(), rule: Some("r".into()) })` equals `{"authored":false,"from_session":"s","from_entries":["e"],"provider":"p","api":"a","model":"m","curated_at":"t","curated_by":"b","rule":"r"}`.
- The same value with `rule: None` serialises to `{"authored":false,"from_session":"s","from_entries":["e"],"provider":"p","api":"a","model":"m","curated_at":"t","curated_by":"b"}`.
- A canon file holding a header line and one `custom` `lys.inherited` entry with id `k1`, whose data is the rule-less object above, makes `canon::load` return `Err(HomeError::CanonExampleWithoutRule { id })` with `id == "k1"`.
- `canon::add_authored` on that canon file with a turns file of the two lines `user: q` and `assistant: a`, rule `r` and curator `b` returns `Err(HomeError::CanonExampleWithoutRule { id })` with `id == "k1"`, and the canon file's SHA-256 is the same before and after.
- `render_claude_code` of the fixture session `outgoing` with that canon file as the target's canon and an absent `out` path returns `Err(HomeError::CanonExampleWithoutRule { id })` with `id == "k1"`, and the `out` path is still absent afterwards.
- A session `inheritor` created in a fresh home, holding one `custom` `lys.inherited` entry with id `h1` and the rule-less data above, is opened with `SessionReader`, and `SessionReader::entry("h1")` returns an entry whose data deserialises to `Inherited` with `rule == None`.
- `git diff 7b53625 -- canon/canon.jsonl` prints nothing.
- `git diff 7b53625 --numstat -- crates/lys-home/src/record/canon_tests.rs` prints `2`, a tab, `2`, a tab and that path; the two added lines contain `inherited.rule.as_deref()` and `Some("verify before claiming")`, one each.
- `git diff 7b53625 --numstat -- crates/lys-home/src/record/mod.rs` prints `2`, a tab, `0`, a tab and that path; the two added lines are `pub mod handover;` and `mod handover_tests;`.
- After this requirement, `crates/lys-home/src/record/handover.rs` exists and every non-blank line of it begins with `//!`.
- `cargo test -p lys-home --all-features canon` passes.

**Files:**
- create: crates/lys-home/src/record/handover.rs
- create: crates/lys-home/src/record/handover_tests.rs
- modify: crates/lys-home/src/record/canon.rs
- modify: crates/lys-home/src/record/canon_tests.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C45 — The shared helpers and constants safe_component, MAX_NAME_BYTES, PI_FORMAT_VERSION, now, fresh_id, json_len, write_durable and custom_type_of are defined in crates/lys-home/src/record/helpers.rs.

**Stories:**
- S8 (Agent, Continues a session imported from Claude Code on Codex) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The rule check sits in canon::load, which canon add and a render with --canon both reach before writing; a session of a home is read by SessionReader, which never checks it. Gates: an_inherited_rule_is_written_when_present_and_skipped_when_absent, the_canon_refuses_an_example_without_a_rule_on_load_add_and_render, a_ruleless_inherited_entry_in_a_session_of_a_home_reads_cleanly. A rule that is present but null is refused as absent.
- Deviation: Built by hand on hand/HOME-015 at the lead's direction, not through card_build_v3; clippy and tests ran on the build machine.
- Files changed:
  - modified: `crates/lys-home/src/record/canon.rs` — Inherited.rule is Option<String>, defaulted and skipped when None; canon::load refuses a lys.inherited entry with no rule by name
  - modified: `crates/lys-home/src/record/canon_tests.rs` — exactly the two named lines
  - modified: `crates/lys-home/src/record/mod.rs` — exactly pub mod handover; and mod handover_tests;
  - created: `crates/lys-home/src/record/handover.rs` — module documentation of the handover invariants
  - created: `crates/lys-home/src/record/handover_tests.rs` — opens with #![cfg(test)]; the canon and serialisation gates
- Checklist delivery:
  - [x] C45 — A lys.inherited entry's rule may be absent in the type and is skipped when absent, a canon entry with a rule serialises to the same bytes as before, and the canon loader refuses by name a canon example without a rule, naming its entry id and the missing field, which canon add and a render with --canon both reach; a rule-less lys.inherited entry in a session of a home reads cleanly. — met; see how
- Story delivery:
  - [x] S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs. — served for the handover

### R3: Take a letter from the outgoing session and refuse by name before anything is written

WHEN a handover is asked for with an outgoing home, a from session id, a list of letter entry ids and a successor path, THE SYSTEM SHALL check, in this order, and refuse at the first check that fails. (1) IF the successor path exists and is not an empty directory, THEN THE SYSTEM SHALL refuse with `HomeError::SuccessorNotEmpty` naming the path. (2) The outgoing home is taken through `Home::read`; IF the home has no session with the from id, THEN THE SYSTEM SHALL refuse with `HomeError::UnknownSession`, and the session is otherwise read through the read-only `SessionReader`. (3) For each letter id in the order given: IF the session holds no entry with that id, THEN THE SYSTEM SHALL refuse with `HomeError::UnknownEntry`; IF the entry is not a `message` entry whose `message.role` is `assistant`, THEN THE SYSTEM SHALL refuse with `HomeError::LetterNotAssistant` naming it. (4) The outgoing session's path is the chain of entries from its root to its head, where the head is read with `index::read_head` over the session's own index and the chain is walked from the head through each entry's `parentId` with `SessionReader::entry`, reading only. The letter's entries stand next to each other on the path in path order when every letter entry lies on that path and each letter entry after the first names the entry before it in the list as its `parentId`. IF a letter entry, taken in the order given, does not lie on the path, THEN THE SYSTEM SHALL refuse with `HomeError::LetterNotContiguous` naming the first such entry; otherwise, IF a letter entry after the first does not name the entry before it as its `parentId`, THEN THE SYSTEM SHALL refuse with `HomeError::LetterNotContiguous` naming the first such entry. (5) IF any letter entry's message carries `provider` `authored`, THEN THE SYSTEM SHALL refuse with `HomeError::LetterAuthored` naming the first such entry. (6) IF no letter entry's message content holds a part whose `type` is `thinking`, THEN THE SYSTEM SHALL refuse with `HomeError::LetterWithoutThinking` naming every letter entry; an empty list reaches this check and is refused here. Every check runs before any file or directory is created. THE SYSTEM SHALL NOT take a lock on the outgoing session, or write, create or rename any file under the outgoing home. THE SYSTEM SHALL NOT create the successor path, or write anything under it, when it refuses. THE SYSTEM SHALL NOT merge letter entries, accept a letter out of path order, or accept a letter entry that stands on a side branch off the root-to-head path, even when its `parentId` names the entry before it. THE SYSTEM SHALL NOT put a letter's text, thinking or signature in a refusal.

**Acceptance:**
- On the fixture, the letter `[u1]` is refused with `LetterNotAssistant` naming `u1`.
- On the fixture, the letter `[a1, a3]` is refused with `LetterNotContiguous` naming `a3`.
- On the fixture, the letter `[a2, a1]` is refused with `LetterNotContiguous` naming `a1`.
- On the fixture, the letter `[a1, b1]`, where `b1` names `a1` as its `parentId` but stands on a side branch off the root-to-head path, is refused with `LetterNotContiguous` naming `b1`.
- On the fixture, the letter `[x1]` is refused with `LetterAuthored` naming `x1`.
- On the fixture, the letter `[a3]` is refused with `LetterWithoutThinking` whose `ids` equal `["a3"]`, and the empty letter `[]` is refused with `LetterWithoutThinking` whose `ids` are empty.
- On the fixture, the letter `[nope]` is refused with `UnknownEntry` naming `nope`, and the from session `no-such-session` with the letter `[a1, a2]` is refused with `UnknownSession` naming `no-such-session`.
- On the fixture, the letter `[a1, a2]` with a successor directory that holds one file `keep` is refused with `SuccessorNotEmpty` naming that directory, and afterwards the directory holds exactly `keep` with the same SHA-256 as before.
- In every refusal case above but the `keep` case, the successor path, absent before, is absent afterwards; in every case the file listing and every file's SHA-256 under the outgoing home are equal before and after; the test asserts that ten refusal cases ran.

**Files:**
- modify: crates/lys-home/src/record/handover.rs
- modify: crates/lys-home/src/record/handover_tests.rs

**Checklist:**
- C46 — Session, its impl, take_lock, load_checked and to_line are defined in crates/lys-home/src/record/session.rs.

**Stories:**
- S8 (Agent, Continues a session imported from Claude Code on Codex) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Every check runs before Home::open touches the successor; the outgoing session is read through SessionReader and Index::read, which write nothing. Gate: every_refusal_is_named_before_anything_is_written counts ten cases and compares every file hash under the outgoing home.
- Deviation: Built by hand on hand/HOME-015 at the lead's direction, not through card_build_v3; clippy and tests ran on the build machine.
- Files changed:
  - modified: `crates/lys-home/src/record/handover.rs` — the checks in order: successor path, session, assistant entries, contiguity on the root-to-head path read through read_head and SessionReader, authored, thinking
  - modified: `crates/lys-home/src/record/handover_tests.rs` — the ten refusal cases
- Checklist delivery:
  - [x] C46 — A handover refuses by name before any file is created: a letter entry that is not an assistant message, a letter run whose entries do not stand next to each other on the outgoing path in the order given, a letter entry whose message carries provider `authored`, a letter holding no thinking block, and a successor path that exists and is not an empty directory. — met; see how
- Story delivery:
  - [x] S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs. — served for the handover

### R4: Write the successor home: the lys.inherited entry, the letter copied whole, then the session_info name

WHEN every check of R3 passes, THE SYSTEM SHALL open the successor path as a new home with `Home::open`, holding `sessions/` and `blocks/`, and create one session in it with a fresh id from `fresh_id`, the outgoing session header's `cwd`, and no `parentSession`. It SHALL then append, in order: (1) one `custom` entry of `customType` `lys.inherited` with a fresh id, no `parentId` and the current time, whose data is the `Inherited` value `authored` false, `from_session` the outgoing session id, `from_entries` every letter id in the order given, `provider`, `api` and `model` from the first letter entry's message, `curated_at` the last letter entry's timestamp, `curated_by` the outgoing session id, and no `rule`; (2) each letter entry, with its id, timestamp and `message` exactly as the source holds them, its `parentId` rewritten to the entry before it in the successor; (3) one `session_info` entry, the child of the last letter entry, whose `name` is `INHERITED_FROM` followed by the outgoing session id and which carries no other field. The head moves to it. THE SYSTEM SHALL then return a `HandoverReport` of `successor_home`, `session`, `file` and `inherited`, where `inherited` is read back from the successor's first entry through `SessionReader` and is true when that entry is `custom` `lys.inherited`. THE SYSTEM SHALL NOT compose, merge, edit or re-order a letter entry's message, or any thinking block or signature in it. THE SYSTEM SHALL NOT write a `rule` key into the handover's data. THE SYSTEM SHALL NOT set `parentSession`. THE SYSTEM SHALL NOT add a field to the header, to `session_info` or to any entry outside `custom.data`. THE SYSTEM SHALL NOT write a block to either home's block store. THE SYSTEM SHALL NOT put transcript content in the report.

**Acceptance:**
- On the fixture, the letter `[a1, a2]` into an absent successor path writes `sessions/<id>.jsonl` under it, where `<id>` is 32 lowercase hex characters, the header's `cwd` is `/work/outgoing`, and the header has no `parentSession` key.
- The successor's entries in file order are exactly four. Entry 1 is `custom` `lys.inherited` with `parentId` null and data equal to `{"authored":false,"from_session":"outgoing","from_entries":["a1","a2"],"provider":"anthropic","api":"anthropic-messages","model":"claude-fixture-model","curated_at":"fixture-time-a2","curated_by":"outgoing"}`, with no `rule` key.
- Entry 2 has id `a1`, `parentId` equal to entry 1's id and timestamp `fixture-time-a1`; its `message` equals the source `a1` message as a JSON value, and the bytes of its `content[0].thinkingSignature` equal the source's bytes, `fixture-signature-a1`.
- Entry 3 has id `a2`, `parentId` `a1`, timestamp `fixture-time-a2`, and a `message` equal to the source `a2` message as a JSON value.
- Entry 4 is `session_info` with `parentId` `a2` and `name` `inherited from outgoing`; its keys are exactly `type`, `id`, `parentId`, `timestamp` and `name`, and the session's head is its id.
- The report's keys are exactly `successor_home`, `session`, `file` and `inherited`; `inherited` is true, `session` equals the header id, `file` equals the successor session file, and its serialised JSON contains none of `fixture-text-`, `fixture-thinking-` and `fixture-signature-`.
- With the successor path an existing empty directory, the same letter gives the same four entries in the same order, and the successor's `blocks/` directory holds no file.
- The file listing and every file's SHA-256 under the outgoing home are equal before and after the handover.

**Files:**
- modify: crates/lys-home/src/record/handover.rs
- modify: crates/lys-home/src/record/handover_tests.rs

**Checklist:**
- C47 — Home and its impl are defined in crates/lys-home/src/record/home.rs.
- C48 — crates/lys-home/src/record/fork.rs imports write_durable and custom_type_of from crate::record::helpers and changes no other line.
- C49 — crates/lys-home/src/record/mod.rs holds only module docs, pub mod and mod lines with their cfg(test) attributes, and pub use lines, and the item grep HOME-013 names prints nothing on it.

**Stories:**
- S8 (Agent, Continues a session imported from Claude Code on Codex) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Gates: the_letter_is_copied_whole_into_a_new_successor_home and an_empty_successor_directory_takes_the_same_four_entries.
- Deviation: Built by hand on hand/HOME-015 at the lead's direction, not through card_build_v3; clippy and tests ran on the build machine.
- Files changed:
  - modified: `crates/lys-home/src/record/handover.rs` — write_successor: the rule-less lys.inherited entry, the letter entries with only parentId rewritten, the session_info name, and the report read back from the first entry
  - modified: `crates/lys-home/src/record/handover_tests.rs` — the four-entry gates for an absent and an empty successor
- Checklist delivery:
  - [x] C47 — A handover writes a new successor home holding one session with a fresh id, the outgoing header's cwd and no parentSession, and leaves every file of the outgoing home byte for byte as it was. — met; see how
  - [x] C48 — The successor session's first entry is a lys.inherited entry with no rule naming the outgoing session as from_session and curated_by, every letter id in path order as from_entries, the letter's provider, api and model, and the last letter entry's timestamp as curated_at. — met; see how
  - [x] C49 — Each letter entry follows in the successor with its id, timestamp and message equal to the source, every thinkingSignature byte for byte, and then one session_info entry named `inherited from <outgoing session id>` with no other field. — met; see how
- Story delivery:
  - [x] S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs. — served for the handover

### R5: Give lys-home the handover subcommand

WHEN `lys-home handover --home <dir> --from <session> --letter <entry id>... --successor <dir>` is run, THE SYSTEM SHALL run R3 and R4. `--letter` takes one or more ids in path order. On success it SHALL print one JSON report `{"command": "handover", "report": …}` holding R4's report. On a refusal it SHALL exit 1 with the refusal on stderr and nothing on stdout. The module doc of `cli.rs` lists `handover` among the commands. THE SYSTEM SHALL NOT render, launch or run the successor. THE SYSTEM SHALL NOT take a `--rule` argument. THE SYSTEM SHALL NOT print transcript content. THE SYSTEM SHALL NOT bring `cli.rs` over 500 lines of code.

**Acceptance:**
- `cli::run(Cli::parse_from(["lys-home", "handover", "--home", <fixture home>, "--from", "outgoing", "--letter", "a1", "a2", "--successor", <absent dir>]))` returns a value whose `command` is `handover`, whose `report.inherited` is true, and whose `report.session` names a file that exists at `report.file`.
- The same call with `--letter x1` returns an `Err` whose `to_string()` begins with `letter_authored` and contains `x1`, and the successor path is still absent.
- `Command::Handover(..).refusal_status()` is 1.
- `Cli::try_parse_from(["lys-home", "handover", "--home", "h", "--from", "s", "--successor", "d"])` is an error naming `--letter`.
- `crates/lys-home/src/cli.rs` has at most 500 lines when blank lines and comment lines are not counted.

**Files:**
- modify: crates/lys-home/src/cli.rs

**Checklist:**
- C50 — Every public path lys_home::record::{Home, Session, safe_component, now, fresh_id, json_len, MAX_NAME_BYTES, PI_FORMAT_VERSION} and lys_home::{Home, Session} resolves as before the move, and no crate outside lys-home changes.

**Stories:**
- S8 (Agent, Continues a session imported from Claude Code on Codex) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Gate: the_handover_subcommand_prints_its_report_and_refuses_by_name.
- Deviation: Built by hand on hand/HOME-015 at the lead's direction, not through card_build_v3; clippy and tests ran on the build machine.
- Files changed:
  - modified: `crates/lys-home/src/cli.rs` — the handover subcommand and HandoverArgs; the module doc lists handover; 454 code lines
- Checklist delivery:
  - [x] C50 — lys-home handover prints one JSON report naming the successor home, the session id and its file, with inherited true read from the successor's first entry, and no transcript content. — met; see how
- Story delivery:
  - [x] S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs. — served for the handover

### R6: Render the successor for its own model and for another

WHILE rendering a successor for Claude Code, THE SYSTEM SHALL apply R4's rule as `render_claude_code` already does: for a target whose model is the letter's, with the letter's provider `anthropic` and api `anthropic-messages`, an inherited signed thinking block renders whole with its signature; for any other model it renders as a text part with one loss-account entry naming the part by hash. This requirement adds gates in `record/handover_tests.rs` on a successor written by R4 and changes no render code. THE SYSTEM SHALL NOT change `harness/claude_code/render.rs` or the render report. THE SYSTEM SHALL NOT change how a thinking block is signed or verified. THE SYSTEM SHALL NOT render a signature to a model other than the letter's.

**Acceptance:**
- The fixture successor of `[a1, a2]` rendered with model `claude-other-model` and an `out` path reports `thinking_as_text` 1, `thinking_kept` 0 and `dropped` 1, and writes exactly two records.
- In that render, the first record's `message.content` equals `[{"type":"text","text":"fixture-thinking-a1"}]`, and no record carries a `signature` key.
- In that render, the loss account's `dropped` holds one entry, whose `hash` equals `Hash::of(&serde_json::to_vec(<source a1 content[0]>)).to_string()` and whose `reason` is `signed thinking rendered as text: different provider, api or model`.
- The same successor rendered with model `claude-fixture-model` reports `thinking_kept` 1 and `dropped` 0, and its first record's `content[0]` has `type` `thinking` and `signature` `fixture-signature-a1`.
- `git diff 7b53625 -- crates/lys-home/src/harness/claude_code/render.rs` prints nothing.

**Files:**
- modify: crates/lys-home/src/record/handover_tests.rs

**Checklist:**
- C51 — Every test that passed before the move passes unchanged with an equal count, no test file changes beyond use lines, no non-test source file in crates/lys-home is over 500 lines of code, and the gate legs pass.

**Stories:**
- S2 (Agent, Continues a session imported from Claude Code on Codex) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: No render code changed. Gates: a_successor_rendered_for_another_model_carries_its_thinking_as_text and a_successor_rendered_for_its_own_model_keeps_its_signed_thinking.
- Deviation: Built by hand on hand/HOME-015 at the lead's direction, not through card_build_v3; clippy and tests ran on the build machine.
- Files changed:
  - modified: `crates/lys-home/src/record/handover_tests.rs` — the two render gates on a successor written by the handover
- Checklist delivery:
  - [x] C51 — A successor rendered for another model carries each inherited signed thinking block as a text part with one loss-account entry naming it by hash, and rendered for the letter's own model keeps it whole with its signature. — met; see how
- Story delivery:
  - [x] S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it. — served for the handover

### R7: Write the handover into the record document and render the cluster

Structural. RECORD.md's `lys.inherited` item gains the handover form: the data `{authored, from_session, from_entries, provider, api, model, curated_at, curated_by}` with no `rule`; the lys.inherited entry as a successor's first entry, followed by the letter entries copied whole with only their parent links rewritten, then a `session_info` named `inherited from <outgoing session id>`; inheritance read from the first entry; the canon refusing an example with no rule; and the handover's refusals by name. The cluster's markdown is re-rendered with `scripts/design/render-cluster.py`, so DESIGN.md, CHECKLIST.md and briefs/HOME-015.md are what their JSON renders to. THE SYSTEM SHALL NOT describe a field outside `custom.data` for the handover. THE SYSTEM SHALL NOT edit rendered markdown by hand.

**Acceptance:**
- RECORD.md's `lys.inherited` item contains `{authored, from_session, from_entries, provider, api, model, curated_at, curated_by}`, the words `inherited from`, and each of `letter_not_assistant`, `letter_not_contiguous`, `letter_authored`, `letter_without_thinking`, `successor_not_empty` and `canon_example_without_rule`.
- From the repository root, `sh scripts/design/gate.sh` exits 0, and `docs/design/home/briefs/HOME-015.md` exists.

**Files:**
- create: docs/design/home/briefs/HOME-015.md
- modify: docs/design/home/RECORD.md
- modify: docs/design/home/design.json
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md

**Checklist:**
- C45 — The shared helpers and constants safe_component, MAX_NAME_BYTES, PI_FORMAT_VERSION, now, fresh_id, json_len, write_durable and custom_type_of are defined in crates/lys-home/src/record/helpers.rs.
- C47 — Home and its impl are defined in crates/lys-home/src/record/home.rs.
- C48 — crates/lys-home/src/record/fork.rs imports write_durable and custom_type_of from crate::record::helpers and changes no other line.
- C49 — crates/lys-home/src/record/mod.rs holds only module docs, pub mod and mod lines with their cfg(test) attributes, and pub use lines, and the item grep HOME-013 names prints nothing on it.

**Stories:**
- S8 (Agent, Continues a session imported from Claude Code on Codex) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

#### R7 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The cluster was re-rendered with render-cluster.py and sh scripts/design/gate.sh exits 0.
- Deviation: Built by hand on hand/HOME-015 at the lead's direction, not through card_build_v3; clippy and tests ran on the build machine.
- Files changed:
  - modified: `docs/design/home/RECORD.md` — the handover form of lys.inherited, the canon refusal and the five handover refusals
  - modified: `docs/design/home/design.json` — the notes of the handover, proof and record rows
  - modified: `docs/design/home/DESIGN.md` — re-rendered
- Checklist delivery:
  - [x] C45 — A lys.inherited entry's rule may be absent in the type and is skipped when absent, a canon entry with a rule serialises to the same bytes as before, and the canon loader refuses by name a canon example without a rule, naming its entry id and the missing field, which canon add and a render with --canon both reach; a rule-less lys.inherited entry in a session of a home reads cleanly. — met; see how
  - [x] C47 — A handover writes a new successor home holding one session with a fresh id, the outgoing header's cwd and no parentSession, and leaves every file of the outgoing home byte for byte as it was. — met; see how
  - [x] C48 — The successor session's first entry is a lys.inherited entry with no rule naming the outgoing session as from_session and curated_by, every letter id in path order as from_entries, the letter's provider, api and model, and the last letter entry's timestamp as curated_at. — met; see how
  - [x] C49 — Each letter entry follows in the successor with its id, timestamp and message equal to the source, every thinkingSignature byte for byte, and then one session_info entry named `inherited from <outgoing session id>` with no other field. — met; see how
- Story delivery:
  - [x] S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs. — served for the handover

### R8: Prove the handover on an elicited letter and measure the carry-over of its signed thinking

WHEN the build is complete, THE SYSTEM SHALL write PROOF-HANDOVER.md from runs on the build machine. (1) The version: the output of `claude --version` on the build machine, recorded beside 2.1.281, the version the row named. (2) The letter: one short Claude Code session with thinking on, asked for a letter to its successor in the session's own words, imported with `lys-home import` into a scratch home, whose final assistant turn, the run of assistant entries after its last user message, is handed over with `lys-home handover` into an absent successor path. Recorded: the session id, the letter entry ids, the successor session id, the SHA-256 of each copied thinking part in the source and in the successor, the SHA-256 of each `thinkingSignature` in both, and whether each pair is equal. IF the harness wrote no signed thinking for that turn, THEN the proof SHALL record that as the finding and state that the byte-equal line is not met. (3) The carry-over: the successor rendered for the letter's own model with `--out`, then resumed with `claude -p --resume <rendered file> --fork-session --max-turns 1` and a short prompt. Recorded: whether the continuation file Claude Code wrote holds a thinking part whose signature's SHA-256 equals the inherited signature's, with that hash and the count of such parts, beside PROOF-RESUME.md's earlier 0 of 17. One line says that whether the block was sent to the model is left open, waiting on R10's proxy. (4) The seeded and plain runs of one card: recorded as not run, naming the handover slot in the launch template as the later unit that makes the two counts measurable. THE SYSTEM SHALL NOT write the text of the letter, its thinking or any reply into the proof. THE SYSTEM SHALL NOT rewrite any file under Claude Code's own projects directory, and SHALL NOT run the proof against a running session. THE SYSTEM SHALL NOT claim a sent-to-model measurement.

**Acceptance:**
- PROOF-HANDOVER.md names the Claude Code version measured and 2.1.281 on one line.
- PROOF-HANDOVER.md records the elicited session's id, its letter entry ids, the successor session id, and for each copied thinking part the source and successor SHA-256 of the part and of its signature, with an equal or not-equal verdict per pair.
- PROOF-HANDOVER.md records the carry-over as present or absent with the signature hash and a count, cites PROOF-RESUME.md's 0 of 17, and holds one line saying that sent-to-model waits on R10's proxy.
- PROOF-HANDOVER.md records the seeded and plain counts of fix rounds and unverified claims as not run, naming the handover slot in the launch template as the later unit.
- Every figure in PROOF-HANDOVER.md is an id, a hash, a count, a version, a command, a path or an exit code.

**Files:**
- create: docs/design/home/PROOF-HANDOVER.md

**Checklist:**
- C52 — PROOF-HANDOVER.md records the handover of an elicited letter by ids, hashes and the signature comparison, whether the inherited signed block appears in a resumed continuation's own file on the installed Claude Code beside 2.1.281, and the seeded and plain card counts as not run.

**Stories:**
- S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.
- S2 (Agent, Continues a session imported from Claude Code on Codex) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.

#### R8 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The signatures compare equal byte for byte; the carry-over is absent (0 parts); sent-to-model waits on R10.
- Deviation: Built by hand on hand/HOME-015 at the lead's direction, not through card_build_v3; clippy and tests ran on the build machine. Claude Code on the build machine was signed out (exit 1, api_error), so the elicitation and the resume ran on the development Mac with the same Claude Code version, 2.1.283, and lys-home ran on the build machine; the files moved between them by scp and were compared by SHA-256. The letter's thinking was written signed with empty thinking text, so rendered for another model it is dropped and named (empty thinking dropped), not carried as a text part; the fixture gate measures the text-part path.
- Files changed:
  - created: `docs/design/home/PROOF-HANDOVER.md` — the elicited letter, the handover by ids and hashes, the carry-over, and the card counts as not run
- Checklist delivery:
  - [x] C52 — PROOF-HANDOVER.md records the handover of an elicited letter by ids, hashes and the signature comparison, whether the inherited signed block appears in a resumed continuation's own file on the installed Claude Code beside 2.1.281, and the seeded and plain card counts as not run. — met; see how
- Story delivery:
  - [x] S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught. — served for the handover
  - [x] S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it. — served for the handover

## Boundaries

- SHALL NOT compose, merge, edit or author a letter entry or any thinking block; a letter entry is copied with only its parent link rewritten.
- SHALL NOT change how thinking blocks are signed or verified, the render's thinking rule, `harness/claude_code/render.rs` or the render report.
- SHALL NOT build the little proxy (R10), a handover slot in the launch template, or a harness hook that elicits the letter.
- SHALL NOT change the canon beyond making `Inherited.rule` optional and refusing a canon example without a rule; `canon/canon.jsonl` stays unchanged.
- SHALL NOT write under the outgoing home: no lock, index, head, entry or block.
- SHALL NOT add a field to Pi's header, to session_info or to any entry outside `custom.data`, and SHALL NOT set the successor's parentSession.
- SHALL NOT change `record/mod.rs` beyond the two module lines, and SHALL NOT change `record/canon_tests.rs` beyond its two named lines.
- SHALL NOT put the letter's text, thinking or signature, or any transcript content, in a report, an error, a log line, a test name or the proof.
- SHALL NOT rewrite any file under Claude Code's own projects directory, and SHALL NOT run a proof against a running session.

## Verification

- From the repository root: `python3 scripts/design/validate.py docs/design/home` and `python3 scripts/design/check-coverage.py docs/design/home` exit 0.
- From the repository root: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --all-features`, `cargo doc --no-deps --all-features` and `cargo doc --no-deps` exit 0.
- From the repository root: `sh scripts/design/gate.sh` exits 0.
- Every changed or new source file under `crates/lys-home/src` has at most 500 lines of code, counted without comments and blank lines.
- `git diff 7b53625 --stat -- crates canon` names only files this brief's requirements list.
- `cargo test -p lys-home --all-features handover` passes, and the refusal test asserts ten cases ran.
- `grep -rln pelican crates/lys-home` prints nothing.
- Read PROOF-HANDOVER.md against R8's acceptance: every figure is an id, a hash, a count, a version, a command, a path or an exit code.
