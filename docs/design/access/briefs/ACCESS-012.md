---
type: brief
id: ACCESS-012
cluster: access
title: An app learns from the change stream when its schema version moves and how its proposal was decided
---

# ACCESS-012: An app learns from the change stream when its schema version moves and how its proposal was decided

> **Cluster:** access
> **Depends on:** DIRECTORY-089
> **Checklist:**
> - C873 — lys-identity's change stream offers a read following schemas (schemas, decided) and the frames schema_standing and schema_decided (outcome approved, declined or applied); ChangeFrame and ChangesRequest are non_exhaustive with constructors; a read that does not follow schemas is the same bytes as in 0.3.2; lys-identity is 0.4.0 (ACCESS-012 R1).
> - C874 — POST /grants/changes answers an app following schemas with its own schema standing on a first read and each later decision on its schema in order from its cursor, wakes a waiting read on the decision's signal, resets a cursor past the app's decisions, and refuses the administrator by name; the decisions are indexed once, never replayed per read (ACCESS-012 R2).
> - C875 — lys-identity 0.4.0 is published and tagged the day its gate is green, and liminal's lead is told the same minute (ACCESS-012 R3).
> **Stories:**
> - S508 (Tom, owner and administrator) — As the owner, when I approve or decline a product's schema change on the Apps screen, the product learns it from Lys's change stream as it happens, so its status never says it is waiting on me after I have decided.

## Purpose

Found on 31 October 2026 by liminal's lead while writing liminal's ACCESS-016 (liminal main 36329c27): liminal-server will propose its own access schema to Lys at start, and its /health reads `proposed` until the administrator decides. Nothing tells liminal the decision. The grant change stream (POST /grants/changes, DIRECTORY-089 R1; crates/lys-identity/src/grants/change_stream.rs) carries Baseline, Change, Watermark, Placement, PlacementMark, Ready, Unready and Reset, and "schema" appears nowhere in it; Lys has no other app-schema feed. So a proposal Tom approves stays `proposed` on liminal's /health until someone looks again by hand, which Tom's rule forbids (29 October: a change is an event applied in place, never a reload). liminal's ACCESS-016 R2 (5) says so plainly and names the frame it would consume: one visible to the app's own stream consumer, naming {app, version} on an applied change, and the operation with approved or declined on a decision. This brief adds it.

The decisions are already recorded. An app's change is kept as a `Proposed` line; the administrator's approval keeps an `Applied` line naming the proposal it approves (`proposal`), and a decline keeps a `ChangeDeclined` line, which clears the pending proposal (crates/lys-identity-server/src/apps_schema_api.rs, approve and decline; crates/lys-identity-server/src/apps_fold.rs:200-213). An administrator's own change is an `Applied` line with no proposal. The `ChangeDeclined` line does not name the proposal it declined; the fold knows it, because it is the app's pending proposal when the line is held.

## Task

Add a read following schemas to the change stream's wire contract in crates/lys-identity (change_stream.rs), with two frames, and make the published types non_exhaustive so the next frame is not a breaking release. Serve it from crates/lys-identity-server: index each app's schema decisions once in the apps index (apps_index.rs, apps_state.rs), build the frames in a new module crates/lys-identity-server/src/grant_changes/schemas.rs called from stream.rs, signal each decision from apps_schema_api.rs so a waiting read wakes on it, and publish the shapes in the OpenAPI document. Release lys-identity 0.4.0 the day it is green and tell liminal's lead.

The decisions this brief makes, each with its reason:

- **Opt-in, like placements.** A read asks for schema frames with `schemas: true`. ChangeFrame is a tagged enum with `deny_unknown_fields`, so a consumer on 0.3.2 (liminal-server today) would refuse a page carrying a frame it does not know; a frame it never asked for is never sent to it. From this brief on, every new frame is asked for by a request field, so an older consumer is never sent one.
- **An app's own schema, with one cursor per app.** The cursor `decided` is the count of decisions on the asker's own app's schema. An app is shown only its own schema, as it is shown only changes on its own kinds; one app's count is one number, so the cursor needs no second order across apps. The administrator decides these changes and reads them on the Apps screen and GET /apps; a read following schemas by the administrator is refused by name (`schemas_need_an_app`), not answered with every app under a cursor that cannot name them.
- **Decisions are not interleaved with grant revisions.** Placements are, because a placement changes what a grant reaches. A schema version does not change what a pass carries (its rights carry their actions expanded, ACCESS-002 R1), so no consumer decision depends on a schema frame's position among grant changes. A schema frame is stated as of the page's hold, the same hold its Ready is read under, and stands after the grant and placement frames and before Ready or Unready.
- **Reconnecting after a decision, or after a reset.** A consumer with a cursor learns every decision after it on its next read: the read is not idle while the app's count is past the cursor, so it answers at once even if it asked to wait. A consumer with no cursor (a first read, or after a Reset) is given a `schema_standing` frame after the Baseline: the app's current version, its pending proposal if one waits, and the count it stands at. Standing rides its own frame rather than a new Baseline field, because Baseline is a grant frame that every 0.3.2 reader parses with `deny_unknown_fields`.
- **The declined proposal is named.** The index records, as each `ChangeDeclined` line is held, the operation of the proposal it cleared; a start from the sealed snapshot recovers it by walking each app's history once (Proposed sets pending, Applied and ChangeDeclined clear it). No stored line changes, and the snapshot format stays lys-apps-state/v1.
- **non_exhaustive now.** ACCESS-010 R3 adds a holder_cut_off frame and is not yet built. Marking ChangeFrame and ChangesRequest `#[non_exhaustive]` in this one breaking release (0.4.0) lets that frame, and every later one, land as a minor release. ChangesRequest gains constructors, since a non_exhaustive struct cannot be built by literal outside its crate.

## Requirements

### R1: The wire contract has a read following schemas and two schema frames

Structural. THE SYSTEM SHALL add to `ChangesRequest` (crates/lys-identity/src/grants/change_stream.rs) `schemas: bool` (`#[serde(default, skip_serializing_if = "std::ops::Not::not")]`) and `decided: Option<u64>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`), the count of decisions on the asker's own schema it has applied, given with `after` whenever `schemas` is. THE SYSTEM SHALL add to `ChangeFrame`:
- `SchemaStanding { app: String, version: u64, pending: Option<String>, decided: u64 }`, wire tag `schema_standing`: the app's current schema version, the operation of the proposal waiting for the administrator if one is, and the count of decisions it stands at.
- `SchemaDecided { app: String, decision: u64, outcome: SchemaOutcome, proposal: Option<String>, version: u64 }`, wire tag `schema_decided`: the decision at position `decision` (from one) on the app's schema. `outcome` is `approved` (the administrator approved the app's proposal named by `proposal`; `version` is the version it made current), `declined` (the administrator declined the proposal named by `proposal`; `version` is the current version, unchanged), or `applied` (the administrator's own change, `proposal` absent; `version` is the version it made current).

`SchemaOutcome` SHALL be a new snake_case enum beside `ChangeKind`. `ChangeFrame` and `ChangesRequest` SHALL be `#[non_exhaustive]`, and `ChangesRequest` SHALL offer `first(limit)`, `in_log(log)` (a first read that names the log it expects, as liminal's first read does today: log given, after absent), `after(log, revision, limit)`, `waiting()`, `following_placements(placed)` and `following_schemas(decided)`, where `decided` is `None` on a first read. Every request shape the 0.3.2 server accepts SHALL be buildable from them, read from every literal ChangesRequest in this workspace and in liminal (crates/liminal-server's grant stream reader), and every literal in the workspace moves to them. The module comment SHALL say what a read following schemas is given and that every frame added from now on is asked for by a request field. `MembershipIndex` (channel_membership_index.rs) SHALL apply a schema frame as nothing, with the cursor unchanged, since a schema decision changes no membership. lys-identity SHALL take its own version, 0.4.0, in crates/lys-identity/Cargo.toml (not the workspace's), and the workspace dependency line SHALL name 0.4.0. CHANGELOG.md SHALL carry a `## [lys-identity 0.4.0]` section naming the read, the frames, the non_exhaustive types and their constructors, and what a consumer changes to move.

**Acceptance:**
- Red first (cargo nextest, lys-identity, tests/change_stream_schemas.rs): `a_read_not_following_schemas_is_the_same_bytes_as_before`: `ChangesRequest::first(10)`, a first read built with `first(10).in_log(..)` (log given, after absent), and a resumed read built with `after` and `waiting` serialise to the JSON 0.3.2 writes for the same reads, pinned as strings in the test.
- Red: `schema_frames_carry_their_wire_names`: a `schema_standing`, a `schema_decided` of each outcome, and a request following schemas serialise with the tags and field names above, and parse back equal.
- Red: `a_frame_from_an_older_reader_still_parses`: every frame kind 0.3.2 writes, pinned as strings, parses unchanged.
- Red (lys-identity, channel_membership_index_tests.rs): `a_schema_frame_moves_no_membership`: an index fed a schema_standing and a schema_decided between two changes holds the same members and cursor as one fed the changes alone, and stays Ready on the next Ready.

**Files:**
- modify: crates/lys-identity/src/grants/change_stream.rs
- modify: crates/lys-identity/src/grants/channel_membership_index.rs
- modify: crates/lys-identity/src/grants/channel_membership_index_tests.rs
- create: crates/lys-identity/tests/change_stream_schemas.rs
- modify: crates/lys-identity/Cargo.toml
- modify: Cargo.toml
- modify: CHANGELOG.md

**Checklist:**
- C873 — lys-identity's change stream offers a read following schemas (schemas, decided) and the frames schema_standing and schema_decided (outcome approved, declined or applied); ChangeFrame and ChangesRequest are non_exhaustive with constructors; a read that does not follow schemas is the same bytes as in 0.3.2; lys-identity is 0.4.0 (ACCESS-012 R1).

**Stories:**
- S508 (Tom, owner and administrator) — As the owner, when I approve or decline a product's schema change on the Apps screen, the product learns it from Lys's change stream as it happens, so its status never says it is waiting on me after I have decided.

### R2: Lys answers an app following schemas from an index, and wakes it on each decision

Behavioural. THE SYSTEM SHALL keep, in the apps index (crates/lys-identity-server/src/apps_index.rs), each app's schema decisions in the order held: the history position of every `Applied` line and every `ChangeDeclined` line on it, and for a `ChangeDeclined` line the operation of the pending proposal it cleared, read from the app before the line clears it (apps_state.rs, `Held::hold`). `Index::of` SHALL rebuild them from each app's history in one pass at start, so a start from the sealed snapshot answers the same. A read SHALL cost the decisions after its cursor, never a walk of the history.

WHEN POST /grants/changes is read with `schemas` by an app (the asker named by its credential), THE SYSTEM SHALL answer as follows. Every frame it answers today stays the same and in the same order.
- On a first read, a `schema_standing` frame after the Baseline, for the asker's app.
- On a read with `decided`, one `schema_decided` frame for each decision on the asker's app after `decided`, in order, each built from its line: an `Applied` line with a proposal is `approved`, without one `applied`, and a `ChangeDeclined` line is `declined` naming the indexed proposal.
- Schema frames stand after the grant and placement frames and before Ready or Unready.
- The read is idle only when no grant change, placement or decision follows its cursor.
- A `decided` past the app's count is answered with Reset (`rollback`), the same way a grant cursor past the log is.

The request's shape SHALL be checked as placements' is, each refusal named in words: `decided` without `schemas`; a resumed read following schemas without `decided`; a first read naming `decided`. A read with `schemas` by the administrator SHALL be refused `schemas_need_an_app`, a new GrantStreamError variant, listed for the route in the OpenAPI table.

`GrantChanges` SHALL hold a schema signal beside its revision and placement signals: a count of decisions across every app. The schema routes SHALL send it once a decision is kept, from approve, decline and an administrator's change. An applied version SHALL be sent after `refresh` has published the model, so a reader woken by it never reads a version the service is not yet deciding with. A waiting read following schemas SHALL be woken by the signal as it is by the others, subscribed before the hold. It never waits on a clock.

The published shapes (`GrantChangesRequest`, `GrantChangeFrame`) SHALL carry the new field and frames. They move with the shapes beside them into crates/lys-identity-server/src/grant_changes/published.rs, so grant_changes.rs stays under the file-length gate. If any other file this brief touches passes the gate's limit, its new code moves to a module of its own, never to an allow.

**Acceptance:**
- Red first (cargo nextest, lys-identity-server, tests/grant_changes_schemas.rs; every wait ends on the read's answer, never a sleep): `an_apps_first_schema_read_states_its_version_and_pending_proposal`: an approved app with a proposal waiting reads schema_standing with its current version, the proposal's operation and decided 0.
- Red: `an_approval_reaches_a_waiting_reader_as_approved_with_the_new_version`: the app's read, following schemas with its cursor and wait, and the administrator's approve are run together. The read answers one schema_decided (approved, the proposal's operation, the new version) before Ready.
- Red: `a_decline_reaches_a_waiting_reader_naming_the_declined_proposal`: as above, with decline: declined, the proposal's operation, the version unchanged.
- Red: `an_administrators_own_change_reaches_the_app_as_applied`: applied, no proposal, the new version.
- Red: `a_reader_that_was_away_learns_the_decision_from_its_cursor`: the approval is kept while no read is open. The next read, asking to wait, answers at once with the decision.
- Red: `a_reset_reader_learns_the_current_version_from_its_standing`: a first read after an approval states the new version with no pending proposal.
- Red: `another_apps_decision_never_reaches_this_app`: a decision on app B leaves app A's reads with no schema frame and A's count unchanged.
- Red: `a_schema_cursor_past_the_apps_decisions_is_reset`: decided above the count answers Reset rollback.
- Red: `the_administrator_following_schemas_is_refused_by_name`: schemas_need_an_app. Each malformed shape is refused with its words.
- Red: `a_read_not_following_schemas_is_answered_exactly_as_before`: with decisions kept, a read without schemas answers the same frames as before them.
- Red: `a_restart_answers_the_same_decisions`: after a restart from the sealed snapshot, a read from decided 0 answers the same decisions, the declined one still naming its proposal.
- The OpenAPI contract tests pass with the new field, frames and refusal.

**Files:**
- modify: crates/lys-identity-server/src/apps_index.rs
- modify: crates/lys-identity-server/src/apps_state.rs
- modify: crates/lys-identity-server/src/apps_schema_api.rs
- modify: crates/lys-identity-server/src/grant_changes.rs
- create: crates/lys-identity-server/src/grant_changes/published.rs
- create: crates/lys-identity-server/src/grant_changes/schemas.rs
- modify: crates/lys-identity-server/src/grant_changes/stream.rs
- modify: crates/lys-identity-server/src/error_grant_stream.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- create: crates/lys-identity-server/tests/grant_changes_schemas.rs

**Checklist:**
- C874 — POST /grants/changes answers an app following schemas with its own schema standing on a first read and each later decision on its schema in order from its cursor, wakes a waiting read on the decision's signal, resets a cursor past the app's decisions, and refuses the administrator by name; the decisions are indexed once, never replayed per read (ACCESS-012 R2).

**Stories:**
- S508 (Tom, owner and administrator) — As the owner, when I approve or decline a product's schema change on the Apps screen, the product learns it from Lys's change stream as it happens, so its status never says it is waiting on me after I have decided.

### R3: Released the day it is green, and liminal told

Behavioural. WHEN R1 and R2 pass the gate at one commit THE SYSTEM SHALL have lys-identity 0.4.0 published the same day, from a MacBook export of that commit (the laptop holds no crates.io credential). It SHALL be checked three ways: the crates.io index checksum, the static.crates.io sha256, and the published files diffed against the commit's tree. It SHALL be tagged lys-identity-v0.4.0 (Tom, 15 October 2026: written, gated and green means published and tagged now). liminal's lead SHALL be told the same minute, with the version and the move: liminal-server's stream reader takes 0.4.0, builds its reads with the constructors, asks `schemas`, and moves `proposed` to `current` or `declined` on schema_decided (liminal's further brief after ACCESS-016).

**Acceptance:**
- The publish record names the three checks and the tag, and the message to liminal's lead names the version, the constructors and the two frames.

**Files:**
- modify: CHANGELOG.md

**Checklist:**
- C875 — lys-identity 0.4.0 is published and tagged the day its gate is green, and liminal's lead is told the same minute (ACCESS-012 R3).

**Stories:**
- S508 (Tom, owner and administrator) — As the owner, when I approve or decline a product's schema change on the Apps screen, the product learns it from Lys's change stream as it happens, so its status never says it is waiting on me after I have decided.

## Boundaries

- SHALL NOT change a frame or a field a 0.3.2 reader parses, or answer a read that does not follow schemas differently.
- SHALL NOT change a stored apps line, the sealed snapshot's format, or a schema route's answer.
- SHALL NOT show an app anything of another app's schema.
- SHALL NOT replay an app's history per read; the index is built once at start and kept as lines are held.
- SHALL NOT move liminal's reader; liminal moves in its own brief after this is published.
- SHALL NOT build ACCESS-010's holder_cut_off frame; it lands after this as a minor release.
- SHALL NOT wait a set time anywhere, in the server or the tests: every wait ends on a signal, the read's answer, a closed connection or an exit.

## Verification

- gate -stages nextest -crates lys-identity,lys-identity-server -test 'test(schema) | test(grant_changes) | test(membership)'; every red first with its FAIL line quoted.
- python3 scripts/design/validate.py docs/design/access and python3 scripts/design/check-coverage.py docs/design/access, read for FAIL lines, never the exit code.
- The whole declared gate on the MacBook at the final commit, with START and END posted with df and the swap line.
- If this code were used in a hospital, the worst credible failure is a product showing a schema change as waiting after it was declined, or as current when it was not, so staff act on a status that is wrong; R2's tests pin each outcome and the reconnect cases, so about zero people are harmed.
