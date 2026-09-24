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
  into place. Absent, the last indexed entry is the head (a file Pi wrote).
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
- `lys.harness_event` (R8), `lys.authored` (R3, R4), `lys.inherited` (R11):
  written up with their requirements.

## The loss account

Written with R4: what a render preserved, transformed and could not carry,
each dropped block named by hash and reason.
