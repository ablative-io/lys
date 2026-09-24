# lys-home

The home: a session held under its identity, in the shape of Pi's session tree.

A home is a directory. `sessions/<id>.jsonl` files are in the grammar Pi's
coding agent writes (a `session` header line, then entries with `id`,
`parentId` and `timestamp`), so a home file parses with Pi's own reader
unchanged. Beside each session file: `<id>.index.jsonl` (entry id, parent,
byte offset, length) so the path from the head to the root is read by seeking
to the entries on it, and `<id>.head` (the persisted head) so reopening
restores where the session stood. `blocks/<hh>/<hash>` holds content once by
SHA-256; entries reference blocks by hash.

lys adds nothing to Pi's grammar. Its own data rides in Pi's `custom` entries:

| custom type         | what it is                                                        |
| ------------------- | ----------------------------------------------------------------- |
| `lys.call`          | one proxy call: request and response parts by hash, raw bodies by hash, status |
| `lys.harness_event` | a harness-local event: a hook, a system record, a permission mode or a tool completion, with the whole source record as a block |
| `lys.authored`      | this session is a hand-written demonstration, not history         |
| `lys.inherited`     | this entry came from the canon or another session, and says so    |

One owner at a time: a session file is opened under an exclusive lock on
`<id>.lock` beside it, held while the `Session` lives, so a second opener in
this process or another is refused by name. An append is durable in three
steps (line, index row, head); a failure after the line is durable makes the
session reconcile itself from the file before it admits anything else. A
session id, and every name joined onto a directory, must be one safe path
component (letters, digits, `.`, `_`, `-`; not beginning with `.`).

The Claude Code profile (`harness/claude_code`): `import` turns a transcript
into a session, with `attachment` and `system` records as events at their
exact place on the chain (a message's parentUuid may name one), `render`
writes a session back as a transcript for a model (same model keeps thinking
whole with its signature; a different model gets readable thinking as text
and a loss account beside the file), `fewshot` writes a hand-authored file
that `claude --resume <path>` takes (the command is in the JSON report, never
run), and `resume-check` counts tool_use ids a fork repeats and the tool
actions in the fork's own records.

What the crate does not do: interpret, print or log transcript contents (errors
and reports carry ids, hashes, offsets and counts only); sign, hash into a lys
log or anchor; encrypt; move a home between devices; run or supervise an
agent; talk to Norn.

Design and brief: `docs/design/home/` (HOME-001). Pi reference: the checkout
at `3d5cbe98`, `packages/coding-agent/src/core/session-manager.ts`.
