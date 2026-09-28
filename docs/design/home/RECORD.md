# The home record

Written for HOME-001 R1, R2 and R6, with HOME-002's render event,
HOME-003's context record, HOME-004's lanterns, HOME-007's rendered uuid,
HOME-006's forks and HOME-030's block rows, loss entry and Claude Code
compactions. Everything here is local state of a home, not a
wire contract; nothing is signed.

## Pi's grammar, as adopted

Read from the Pi checkout at `3d5cbe98`
(`packages/coding-agent/src/core/session-manager.ts`).

- **Header**, the first line: `{"type":"session","version":2,"id":…,"timestamp":…,"cwd":…,"parentSession"?:…}`.
- **Entry**, every later line: `type`, `id`, `parentId` (null for the first),
  `timestamp`, then the fields of its type. Types: `message` (Pi's
  `AgentMessage`, kept as JSON), `model_change`, `thinking_level_change`,
  `usage`, `compaction` (`summary`, `firstKeptEntryId`, `tokensBefore`),
  `branch_summary` (`fromId`, `summary`), `label` (`targetId`, `label`),
  `session_info`, `custom` (`customType`, `data`), `custom_message`,
  `context_edit`. Fields this crate does not act on are kept verbatim.
- **The head**: the current position. Appending writes a child of the head and
  advances it. Moving the head back rewrites nothing.
- **The context path**: the root-to-head path, with a compaction on it as Pi's
  `buildSessionContext` reads it: the compaction entry first, then the kept
  entries from `firstKeptEntryId` to the compaction, then everything after.

## What lys keeps beside the file

- `<id>.index.jsonl`: one row per entry `{id, parent, offset, len}`. Reading a
  path seeks to its entries only. An index whose last row does not reach the
  end of the file is stale: it is refused by name and rebuilt from the file,
  which is the one full read, and the session reports that a rebuild happened.
- `<id>.head`: the head entry id, written whole to a temporary file and renamed
  into place. Absent (a file Pi wrote), the last indexed entry is taken as the
  head and persisted on that open, so a later side leaf is never taken for it.
  Opening a session may therefore write beside the file before the command
  that opened it does anything else: a missing or stale index is rebuilt and
  a missing head is persisted, by every command that opens a session
  (`render`, `render-launch`, `ingest-call`, `canon add --from`), even when
  that command then refuses. The session file itself is never written by an
  open.
- `<id>.blocks.jsonl` (HOME-030 R2): one JSON row per content part stored at
  import, `{entry, part, hash}`. `entry` is the id of the entry the part went
  into (for a user record split into tool result messages, the tool result's
  own entry); `part` is the part's 0-based index in its source record's
  content, a string content being part 0; `hash` is the SHA-256 hex
  `BlockStore::put` returned for the part as the source record held it,
  never one recomputed from the Pi-shaped part. A row names no content, no
  text and no length of text. The file is appended only, is never part of
  Pi's grammar (no field is added to the header or to any entry for it), is
  durable before the import returns, and can be rebuilt from the original
  file by re-importing its parts through the store. A session imported before
  it existed has none; a reader then reports that session's blocks as
  unverified and never guesses them. It is not yet in the shipped tracked
  set, so a fetched home lists its blocks unverified.
- Durability order on append: entry line fsynced, then index row fsynced, then
  head renamed. A crash between any two leaves a file whose index is either
  complete or rebuildable, never one that claims an entry it does not hold.

## Blocks

`blocks/<hh>/<hash>`, SHA-256 hex. A put returns the hash and whether it was
new; a block already held is not written again; a block is never rewritten or
deleted. Written to a temporary file, fsynced, renamed, directory fsynced.

## The home as a git repository

`lys-home ship` (HOME-019) makes a home directory a git repository,
`<home>/.git`, tracking exactly the home's tracked set, enumerated by name
and never by a glob pattern or a `.gitignore`:

- for each session the home lists, `sessions/<id>.jsonl`,
  `sessions/<id>.index.jsonl` and `sessions/<id>.head`;
- each `blocks/<hh>/<hash>` and each `templates/<hh>/<hash>` whose name is 64
  lowercase hex digits under a directory named by its first two.

Never tracked: a session's lock file `sessions/<id>..lock` (the doubled dot
is the lock path's own spelling), a head or block temporary, any file at the
home root, anything under `.git`. The one ref is fixed, `refs/lys/home`, in
the home's own repository and on the remote; a ship pushes it without force
to a bare repository at a path on this machine. `fetch` pulls that ref into a
new, empty home, verifies every index, head, block and template strictly
(never rebuilding an index), appends one `arrival` per session, and commits
those appended lines as one commit whose only parent is the fetched commit,
with `refs/lys/home` and HEAD set to it, so the target's status is clean and
a later ship from the target carries the arrivals onward.

## The lys custom entries

- `lys.call` (R6): `{provider, api, model, request:[hash], response:[hash],
  raw_request: hash, raw_response: hash, status, started_at, duration_ms,
  stream}`. `api` is one of `anthropic-messages`, `openai-chat-completions`,
  `openai-responses`. Request parts: Messages = `system` (string or its parts)
  then each `messages[].content` part; Chat Completions = each `messages[]`;
  Responses = `instructions` then each `input[]` item. Response parts:
  Messages `content[]`, Chat Completions `choices[].message`, Responses
  `output[]`; a raw event stream yields no parts and its bytes are kept as the
  raw block. `status`: `complete`, `cancelled`, `partial`, `unrecorded`,
  `lost`; only `complete` carries a full response list. No header is ever
  stored. The ingest report counts `part_blocks_new`, `part_blocks_reused` and
  `raw_blocks` separately.
- `lys.harness_event` (R8): `{kind, harness: "claude-code", source_uuid,
  record, detail}`. `kind` is `hook`, `attachment`, `system`,
  `permission_mode`, `tool_completed`, `template_render` or `arrival`; `record` is the
  whole source record as a block, by hash; `detail` holds names, ids, exit
  codes and counts only, never output or a body, and the serialised data is
  at most 512 bytes. A record that carries a uuid (`attachment`, `system`)
  sits at its exact place on the file's chain under that uuid, since a
  message's parentUuid may name it; a `permission-mode` record (no uuid) and
  each `tool_completed` (one per tool result) hang under the entry they
  followed as side leaves, off the context path.
- `template_render` (HOME-002 R5), the sixth kind: written by `render-launch`,
  not imported. Its `source_uuid` is null, its `record` is the SHA-256 of a
  manifest block, and its `detail` is `{template, session_head, files}`: the
  template hash, the session head hash (the SHA-256 of the head entry's line,
  newline included) and the count of files written. The manifest block, in
  the block store, is `{template, session_head, head, uuid, files}` with
  `head` the head entry id (or null), `uuid` the rendered session id and
  `files` a list of `{path, sha256}` in write order, so the paths never sit
  in the event and it stays under the cap. The event hangs beside the context
  path as a side leaf under the head (`append_beside`): the head does not
  move, the render walker never sees it, and a second render of the same
  session records the same session head hash in a second event.
- `arrival` (HOME-019 R4), the seventh kind: written by `fetch`, not
  imported, one per arriving session. Its `source_uuid` and `record` are both
  null (no block is stored for it), and its `detail` is exactly
  `{source_commit, remote, ref, execution}`: `source_commit` the 40-digit
  commit fetched, `remote` the absolute path of the bare repository it was
  fetched from, `ref` `refs/lys/home`, and `execution` a fresh execution id of
  that session's own, 32 lowercase hex digits (`record::fresh_id`), so each
  arrived session is a distinct execution with its ancestry on the record.
  It hangs beside the head as `template_render` does (`append_beside`): it
  moves no head, and the arrived head file is byte-identical to the source's.
  A remote whose path would carry the data over the 512-byte cap is refused
  before fetch writes anything.
- `lys.given` (HOME-003 R3): the context record, what a rendered session was
  given, as hashes only. Data is exactly `{harness, harness_version, kinds,
  config_dir, documents, environment}`. `harness` is `claude-code` and
  `harness_version` the Claude Code version the load order was measured on
  (2.1.283, PROOF-GIVEN.md). `kinds` has two members: `resolved`, the list
  `claude_md_chain, user_claude_md, memory_index, appended_instructions,
  mcp_config, environment_names`, and `unlisted`, the list
  `claude_md_imports, claude_rules`, the two kinds that reach the request
  only by the harness reading a document, which this entry never parses;
  a second `lys.given` entry appended at the first request by the proxy's
  capture records those, so a reader can tell a kind this entry leaves out
  from one absent from the session. `config_dir` is `{path, source}`, with
  `source` `template` when the template's env slot set `CLAUDE_CONFIG_DIR`
  and `home` when it set none and the path is `HOME/.claude` from the
  rendering process's `HOME` (never that process's `CLAUDE_CONFIG_DIR`).
  `documents` is the ordered list, each `{kind, path, length, sha256}`, in
  one order, the order the harness's request gives (HOME-011, ADR-031):
  wherever the request's order and the read order, the order the harness
  reads the files in, differ, the request's order wins, since it is what
  reached the model. So the entry lists `appended_instructions`,
  `mcp_config`, `user_claude_md`, `claude_md_chain`, `memory_index`:
  first `appended_instructions` and `mcp_config` (the two files the render
  wrote, named by their path relative to the render's out directory, so two
  renders that write no per-render bytes give equal lists), both
  harness-side inputs ahead of the first message; the MCP configuration sits
  straight after the appended instructions, a place taken from the read
  order until a real server's tools position is measured; then
  `user_claude_md` (`<config>/CLAUDE.md`), then `claude_md_chain` for each
  directory from the outermost ancestor of the working directory down to it,
  its `CLAUDE.md`, `.claude/CLAUDE.md` and `CLAUDE.local.md` in that order
  within one directory, then `memory_index`
  (`<config>/projects/<slug>/memory/MEMORY.md`, the slug being every
  character outside ASCII letters and digits replaced by `-`); every path
  but the two written files is absolute. A position whose file is absent is
  omitted; when the config directory is `D/.claude` for a `D` on the chain,
  `D/.claude/CLAUDE.md` is listed once, as `user_claude_md`, in the user
  file's place and not again on the chain. The old order, `user_claude_md`,
  `appended_instructions`, `mcp_config`, `claude_md_chain`, `memory_index`,
  is superseded by the commit that lands HOME-011 and from that commit's
  date; entries written under it stand as written and are never rewritten.
  The old order was written under `harness_version` 2.1.283, and the
  harness version does not tell the two orders apart: a reader tells an
  entry's order by comparing its recorded time with the date of the commit
  that lands HOME-011, an entry written before it listing the old order.
  `environment` is the names of the variables the template set for the
  session, sorted, never a value or a handle. The
  entry hangs under the `template_render` event it follows, beside the
  context path, so the head does not move. The entry itself is unsigned and
  unencrypted and carries no document content; a render given a key signs
  its data as a separate `lys.given_statement`, and encryption at rest
  (stage 3) can be added later without changing what is recorded.
- `lys.given_statement` (HOME-018 R2, R3): `{given, statement}`. `given` is
  the id of the `lys.given` entry it signs, and it hangs under that entry,
  beside the context path, so the head does not move. `statement` is the
  hash of a block holding a `lys/attestation/v2` `COSE_Sign1`, lys-core's
  existing attestation, whose payload is the RFC 8785 bytes of the
  `lys.given` entry's data (keys ordered, no whitespace, no trailing
  newline); the report's `given_sha256` is the SHA-256 of those bytes. The
  same render writes the statement and its payload under `--out` as
  `given-statement.cose` and `given-data.json`, which `lys verify
  --attestation given-statement.cose --payload given-data.json` checks
  offline. It is written only when `render-launch` is given `--key`, a raw
  32-byte Ed25519 seed file; a render with no key writes no block, no entry
  and neither file, and its report's `signing` is `unsigned`. The entry holds
  no payload byte, no key byte and no document content.
- `lys.authored` (R3, R4): no data. Precedes the first authored message entry
  of a session; every authored assistant message carries provider, api and
  model `authored`, so a demonstration is never mistaken for history.
- `lys.inherited` (R11): `{authored, from_session, from_entries, provider, api,
  model, curated_at, curated_by, rule}`. One per example in the canon
  (`canon/canon.jsonl`), followed by the example's message entries copied
  whole with their ids, thinking blocks and signatures; an authored example
  has `authored: true`, no source, and provider, api and model `authored`.
  The canon is appended only, through the repository's review, and rendered
  first (before a session's own entries) when a render is given `--canon`;
  R4's thinking rule applies to every inherited thinking block. Every
  `lys.inherited` entry in the canon file is a canon example and states its
  rule: the canon loader refuses one whose data has no `rule` by name,
  `canon_example_without_rule`, naming its entry id, so `canon add` and a
  render with `--canon` both refuse it before writing anything.
  The handover (HOME-015, HOME-001 R12, ADR-058) uses the same custom type
  without a rule, since a letter is not a rule: `{authored, from_session,
  from_entries, provider, api, model, curated_at, curated_by}`, with
  `authored` false, `from_session` and `curated_by` the outgoing session's
  id, `from_entries` the letter's entry ids in path order, provider, api and
  model those of the first letter entry, and `curated_at` the last letter
  entry's timestamp. `lys-home handover` writes it as the first entry of the
  one session of a new successor home (fresh id, the outgoing header's cwd,
  no parentSession), followed by the letter's entries copied whole, each
  keeping its id, timestamp and message with only its parent link rewritten,
  then a `session_info` named `inherited from <outgoing session id>` with no
  other field. That first entry is what says the successor's first memory is
  inherited; the handover's report reads `inherited` from it. A rule-less
  `lys.inherited` entry in a session of a home is read without the canon's
  check. The outgoing session is read without being owned and nothing is
  written under its home. Before anything is created the handover refuses by
  name: `letter_not_assistant` (a letter entry that is not an assistant
  message), `letter_not_contiguous` (a letter entry off the root-to-head
  path, or not the child of the entry named before it), `letter_authored`
  (a letter entry whose message carries provider `authored`),
  `letter_without_thinking` (no letter entry holds a thinking block) and
  `successor_not_empty` (the successor path exists and is not an empty
  directory).
- `lys.lantern` (HOME-004 R1, R3): `{point, note, lit_by, lit_at}`. `point` is
  the entry id of an entry of the same session that is neither a lantern nor
  an epilogue (the head or any entry the head has moved past); `note` is the
  note byte for byte as written; `lit_by` is a self-declared name, as the
  canon's `curated_by` is, not a verified identity; `lit_at` is when, as the
  record's clock writes it. Lit only by `lantern light`, appended as a child
  of the head, and the head advances to it.
- `lys.lantern_epilogue` (HOME-004 R1, R4): `{lantern, words, added_by,
  added_at}`. `lantern` is the entry id of a `lys.lantern` entry of the same
  session; `words` are the further words byte for byte as written; `added_by`
  is a self-declared name; `added_at` is when. Added only by `lantern
  epilogue`, appended as a child of the head of the lantern's own session.
  A lantern's story is its entry followed by its epilogues in file order;
  nothing is rewritten. A lantern's note and its epilogues are the one text
  the crate prints, and only `lantern recall` prints them (ADR-015); errors
  and the light and epilogue reports carry ids, names and times only.

- `lys.forked_from` (HOME-006 R3): `{parent_session, lantern, point, cut_at,
  coordinate_carried, carried, seed_left_out}`. The first entry a fork writes
  in the child, directly after the copied chain, as the child of the cut
  entry; the child's head is it. `parent_session` is the parent's session
  id, `lantern` the lantern the fork was taken through, `point` its point,
  `cut_at` the cut entry (the last `message` entry whose role is `assistant`
  at or before the point, on the root-to-point chain), `coordinate_carried`
  whether the point is a user message carried as the child's first prompt
  rather than copied, `carried` that entry's id (or null), and
  `seed_left_out` an object counting, by each part's `type`, the parts of
  the carried message that are not `text` (`{}` when nothing is carried,
  when the content is a string, or when every part is text). Written only
  by `fork`.
- `lys.loss` (HOME-030 R4): what one compaction summarised, as ids, counts
  and hashes, never content: no text, thinking, tool input, tool result or
  summary, and no key named `text`, `content` or `body`. It is the line
  directly after its compaction entry in the file, with the compaction as its
  parent: a side leaf off the context path, so the path through the
  compaction and the context path are unchanged, and it never becomes the
  file chain's last entry. Its data is `{compaction, first_kept, kept_none,
  span_first, span_last, entries, messages, tool_calls, tool_results,
  blocks, entry_bytes, block_bytes, blocks_sha256, tokens_before}`:
  `compaction` the compaction's entry id; `first_kept` its
  `firstKeptEntryId`; `kept_none` true exactly when `first_kept` is the
  compaction's own id; `span_first` and `span_last` the first and last entry
  ids of the span, both null when the span is empty; `entries` the number of
  span entries; `messages` the span's message entries (user, assistant and
  `toolResult`); `tool_calls` the `toolCall` parts in the span's assistant
  messages; `tool_results` the span's `toolResult` messages; `blocks` the
  block rows whose entry is in the span; `entry_bytes` the sum of the span
  entries' line lengths as the index holds them; `block_bytes` the sum of
  the byte lengths of the blocks those rows name, as held; `blocks_sha256`
  the SHA-256 hex of those rows' hashes in span order then part order, each
  followed by one newline; `tokens_before` the compaction's `tokensBefore`.
  The span is the context the compaction summarised as that context stood,
  every entry its summary now stands in for: the entries of the compaction's
  ancestry that Pi's context reading covered at the compaction's parent,
  less the entries the compaction keeps. On the ancestry, root first, it
  starts at the root when no compaction entry sits earlier on it; otherwise
  at the nearest earlier compaction's first kept entry when that entry is on
  the ancestry (so entries an earlier compaction kept, and that earlier
  compaction entry itself, are in the span), and at that earlier compaction
  entry when its first kept entry is itself or is not on the ancestry. It
  ends at the entry whose child on the ancestry is the first kept entry, or
  at the compaction's parent when the compaction keeps nothing. For a
  completing compaction (one whose details carry `completes`) the span is
  the span of the compaction it completes, and every field but `compaction`
  and `tokens_before` equals that compaction's loss entry's. The span is one
  unbroken run of the ancestry, so `span_first`, `span_last` and the parent
  ids between them name every id in it. Counting: entries off that ancestry
  (side leaves such as `tool_completed` and permission-mode events, earlier
  loss entries, sidechains) are not in the span; custom entries on it
  (attachment and system events, `lys.authored`) count in `entries` only.
  The span is walked by seeking its entries through the index, never by
  reading the whole file, and no span entry or block is removed, rewritten
  or moved. This is not the render's `<uuid>.loss.json`, which accounts for
  what one render dropped; `lys.loss` accounts for what a compaction left
  out of the context, in the home.
- `lys.fork` (HOME-006 R3): `{child}`, the child's session id. Appended at
  the parent's head, which advances to it, once per fork; the parent gains
  this one line and no earlier byte of it changes.

## The rendered uuid

A Claude Code record's `uuid` is the entry's own id when that id is
uuid-shaped (36 characters, hex with `-` at 8, 13, 18 and 23), so an imported
session keeps its source's uuids. Any other id, the importer's `<uuid>-r<i>`
for a tool result split from a record with more than one, a hand-authored id,
a canon id, derives as UUID version 5 (RFC 9562) under the session's namespace
over the name `<entry id>#<role>`; the session's namespace is UUIDv5 of the
fixed lys render namespace `32c05904-d1f1-550c-9eee-2f6c8f98b665` (itself
UUIDv5 of the URL namespace over `lys/home/claude-code/render-uuid/v1`) over
the id of the session being rendered, so the same entry id in two sessions
never derives one uuid, and the target session id, which the record does not
hold, never enters it. The roles are closed: `record`, the record's `uuid`,
and `compact_boundary`, the uuid of the `compact_boundary` record a
compaction renders as beside its summary record (HOME-030 R6); the next
record's parentUuid and an assistant's `msg_` id follow from them. A derived uuid carries version nibble 5 where Claude
Code's own carry 4. Nothing random and no clock enters a render, every
timestamp is the entry's own, and the walk takes entry order then part
order, so the same session head with the same target writes the same bytes
(CN9, ADR-016); the namespace, the name form and the roles are fixed, and a
change is a new version alongside.
## Claude Code compactions

HOME-030 R3 and R6, measured on Claude Code 2.1.281 files. This corrects
HOME-001's import, which read only a `summary` record, by measurement: every
other compacted file holds the pair below.

**Import.** A compaction is a `system` record with subtype
`compact_boundary` (parentUuid null, `logicalParentUuid` the record before
it, `compactMetadata` with `trigger`, `preTokens` and, when part of the
conversation is kept, `preservedSegment` with `headUuid`, `anchorUuid` and
`tailUuid`), and a `user` record with `isCompactSummary: true` whose
parentUuid is the boundary's uuid and whose message content is the summary.
The pair is matched by that parentUuid, never by adjacency, and becomes one
Pi compaction entry when the summary record is read: its id is the summary
record's uuid, so a later record naming the summary attaches under it; its
parent is the boundary's `preservedSegment.tailUuid` when that names an
entry on record, otherwise its `logicalParentUuid` when that does,
otherwise the file chain's last entry, so a `tailUuid` naming no entry on
record is never refused; its summary is the summary record's content;
`firstKeptEntryId` is the first entry the record named by
`preservedSegment.headUuid` produced; `tokensBefore` is `preTokens`; the
timestamp is the summary record's; and its Pi `details` field is
`{boundaryUuid, summaryUuid}`. The summary's content is stored as a block
with a row under the compaction's id, part 0. Neither record becomes a
harness event or a user message; the import report counts compaction
entries in `compactions` and the records they came from in
`compaction_sources`. A boundary without `preservedSegment` keeps nothing:
`firstKeptEntryId` is the compaction's own id. A boundary whose
`isCompactSummary` record has not been read when a later record names the
boundary as its parent, or at the end of the file, is imported at that point
under the boundary's uuid with an empty summary, the boundary's timestamp,
the same parent, first kept and tokens rules, and details `{boundaryUuid,
summary_missing: true}`; the file is not refused. An `isCompactSummary`
record read after that compaction was written is imported as a second
compaction entry under the summary's uuid whose parent is the first, with
the first's `firstKeptEntryId` and `tokensBefore` and details
`{boundaryUuid, summaryUuid, completes}` naming the first in `completes`;
the first is left unrewritten and keeps `summary_missing`. A `type:"summary"`
record, the earlier shape, is imported as before: a fresh id under the last
main-path message, keeping nothing, `tokensBefore` 0. Every compaction entry,
by any of these paths, becomes the file chain's last entry, so the next
record without an on-record parent attaches under it and the head is set to
it when nothing follows; its `lys.loss` entry is written on the next line.
The one refusal: a `preservedSegment.headUuid` that names no record on
record when the compaction is written refuses the import naming that uuid,
as an unknown parentUuid is refused, and nothing of the compaction is
appended.

**Render.** A context path whose first entry is a compaction renders in the
shape Claude Code 2.1.281 writes: a `system` record with subtype
`compact_boundary`, content `Conversation compacted`, level `info`,
`compactMetadata` `{preTokens}` from `tokensBefore` and parentUuid null;
then a `user` record with `isCompactSummary` and `isVisibleInTranscriptOnly`
true and message `{role: user, content: <summary>}` under the boundary; then
the kept and later records, the first under the summary record. Both carry
`sessionId`, `cwd`, `version`, `userType`, `isSidechain` and `timestamp` as
every rendered record does. No `type:"summary"` line is written (HOME-001
wrote one before the shape was measured, and it left the summary in no
record the model reads), no custom entry (`lys.loss` included) and no
`preservedSegment`.

## Forks

A fork (HOME-006) cuts from the session a lantern was lit in, read from
the `lys.lantern` data key `lit_in`; a lantern whose data carries no
`lit_in` is an older record and resolves by the sessions holding it (one
holder cuts, several refuse `lantern_ambiguous` until one is named with
`--session`). A copy of a lantern's line in a child (copied lines keep
their ids) is a copy, not a second lantern: with `lit_in` recorded it is
never cut from. The chain is read through the index, never by loading the
file, and the cut ends at the last assistant message at or before the
point; a point before any assistant message is refused `nothing_to_fork`.

The child is a new session under the parent's `cwd` whose header's
`parentSession` is the parent file's path relative to the home root
(`sessions/<parent id>.jsonl`), the field Pi reads as the parent session's
file path. Its copied lines are the parent file's own bytes for each cut
entry, read by the index row's offset and length and never re-serialised,
so each copied line hashes equal to the parent's; nothing else of the
parent enters the child, no block is written, and the child's parts name
the same block hashes as the parent's.

The fork report carries ids and counts only: `child`, `parent`,
`lantern`, `point`, `cut_at`, `entries` (the entries copied), `blocks`,
`unstored`, `coordinate_carried` and `carried`. The candidate hashes are
the block hashes the copied custom entries name in their data (`record` of
a `lys.harness_event`; `request`, `response`, `raw_request` and
`raw_response` of a `lys.call`) and the SHA-256 of each content part of
the copied message entries, serialised as it stands in the entry. `blocks`
counts the distinct candidates that name a block the store holds, each
checked by path and never read; `unstored` counts those that name none.
The importer stores a Claude Code part in its source form and writes the
mapped Pi part inline (a `thinking` part with `thinkingSignature`, a
`toolCall` part), so an inline thinking, tool-call or tool-result part
hashes to no block and counts as unstored; a text part is stored as it
stands and counts as held.

A carried user message is not copied: the Claude Code render of such a
child writes its text parts, in order, beside the rendered file as
`<stem>.seed.txt` under the in-band marker line `<FORKED FROM SESSION
<parent> AT ENTRY <point> BY LANTERN <lantern>>` and a heading line, and
the printed launch line passes that file as the resumed session's first
prompt; `render-launch` does the same on the template's line and lists
the seed in its manifest. Neither entry is a signed or frozen wire format:
both are local state of a home, as every lys custom entry here is.

## The loss account

Written with R4: what a render preserved, transformed and could not carry,
each dropped block named by hash and reason.
