# The home record

Written for HOME-001 R1, R2 and R6. Everything here is local state of a home,
not a wire contract; nothing is signed.

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
  `permission_mode`, `tool_completed` or `template_render`; `record` is the
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
  R4's thinking rule applies to every inherited thinking block.

## The loss account

Written with R4: what a render preserved, transformed and could not carry,
each dropped block named by hash and reason.
