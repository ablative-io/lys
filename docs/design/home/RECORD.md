# The home record

Written for HOME-001 R1, R2 and R6, with HOME-002's render event,
HOME-003's context record, HOME-004's lanterns, HOME-007's rendered uuid
and HOME-006's forks. Everything here is local state of a home, not a
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
  Whether a template's `CLAUDE_CONFIG_DIR` moves the session's config
  directory, when it reaches the session only through the settings file the
  launch line passes with `--settings`, was measured for HOME-026 (written
  under its draft id HOME-008; PROOF-GIVEN.md, `## The config directory
  through --settings (HOME-008)`): three runs of the printed launch line
  with `CLAUDE_CONFIG_DIR` absent from the launching environment read both
  the user `CLAUDE.md` and the memory index from the template's directory
  and neither from `HOME/.claude`. The answer is **yes**, for every version
  and host measured or cited:

  | host | Claude Code version | measured or cited | answer |
  | --- | --- | --- | --- |
  | Dean's laptop (`Mac.modem`), the machine the build runs on | 2.1.283 | measured, 28 September 2026 | yes |
  | the version `lys.given` entries have been rendered under since the context record landed (PROOF-LAUNCH.md, this file) | 2.1.283 | cited, not re-measured | yes |

  The version measured is the version entries are rendered under, so there
  is no finding of a version difference and `MEASURED_VERSION` stays
  2.1.283. `lys.given` entries whose `config_dir.source` is `template` and
  whose `harness_version` is 2.1.283 name the directory the session read
  from; they stand as written, and the record's shape does not change.
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
hold, never enters it. The roles are closed: `record`, the record's `uuid`;
the next record's parentUuid, a summary's leafUuid and an assistant's `msg_`
id follow from it. A derived uuid carries version nibble 5 where Claude
Code's own carry 4. Nothing random and no clock enters a render, every
timestamp is the entry's own, and the walk takes entry order then part
order, so the same session head with the same target writes the same bytes
(CN9, ADR-016); the namespace, the name form and the roles are fixed, and a
change is a new version alongside.
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
