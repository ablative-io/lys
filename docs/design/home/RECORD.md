# The home record

Written for HOME-001 R1, R2 and R6, with HOME-002's render event and
HOME-003's context record. Everything here is local state of a home, not a
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
  the measured order: `user_claude_md` (`<config>/CLAUDE.md`), then
  `appended_instructions` and `mcp_config` (the two files the render wrote,
  named by their path relative to the render's out directory, so two renders
  that write no per-render bytes give equal lists), then `claude_md_chain`
  for each directory from the outermost ancestor of the working directory
  down to it, its `CLAUDE.md`, `.claude/CLAUDE.md` and `CLAUDE.local.md` in
  that order, then `memory_index`
  (`<config>/projects/<slug>/memory/MEMORY.md`, the slug being every
  character outside ASCII letters and digits replaced by `-`); every path
  but the two written files is absolute. A position whose file is absent is
  omitted; when the config directory is `D/.claude` for a `D` on the chain,
  `D/.claude/CLAUDE.md` is listed once, first, as `user_claude_md`.
  `environment` is the names of the variables the template set for the
  session, sorted, never a value or a handle. The
  entry hangs under the `template_render` event it follows, beside the
  context path, so the head does not move. It is unsigned and unencrypted
  and carries no document content: signing (stage 6) and encryption at rest
  (stage 3) can be added later without changing what is recorded.
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
(CN9, ADR-014); the namespace, the name form and the roles are fixed, and a
change is a new version alongside.

## The loss account

Written with R4: what a render preserved, transformed and could not carry,
each dropped block named by hash and reason.
