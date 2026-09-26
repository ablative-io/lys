# lys-home

The home: a session held under its identity, in the shape of Pi's session tree.

A home is a directory. `sessions/<id>.jsonl` files are in the grammar Pi's
coding agent writes (a `session` header line, then entries with `id`,
`parentId` and `timestamp`), so a home file parses with Pi's own reader
unchanged. Beside each session file: `<id>.index.jsonl` (entry id, parent,
byte offset, length) so the path from the head to the root is read by seeking
to the entries on it, and `<id>.head` (the persisted head) so reopening
restores where the session stood. `blocks/<hh>/<hash>` holds content once by
SHA-256; entries reference blocks by hash. `templates/<hh>/<hash>` holds each
launch template the home has rendered, once by SHA-256, never rewritten.

lys adds nothing to Pi's grammar. Its own data rides in Pi's `custom` entries:

| custom type         | what it is                                                        |
| ------------------- | ----------------------------------------------------------------- |
| `lys.call`          | one proxy call: request and response parts by hash, raw bodies by hash, status |
| `lys.harness_event` | a harness-local event: a hook, a system record, a permission mode or a tool completion, with the whole source record as a block |
| `lys.authored`      | this session is a hand-written demonstration, not history         |
| `lys.inherited`     | this entry came from the canon or another session, and says so    |
| `lys.given`         | what a rendered session was given: each instruction document Claude Code loads and each file the render wrote, by kind, path, length and SHA-256 in the measured order, with the config directory and the environment names; never a document's content |

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

`render-launch --home <dir> --session <id> --template <file> --uuid <uuid>
--cwd <dir> --model <id> --version <claude code version> --out <dir>` turns a
session into the files a Claude Code launch needs, from a launch template
(schema: `docs/design/home/launch-template.schema.json`; `harness`, `flags`
and the five slots `transcript`, `mcp`, `env`, `secrets`, `instructions`; a
slot outside the five is refused by name). It writes `<uuid>.jsonl` and
`<uuid>.loss.json` (the render), `mcp.json`, `env.json` (a settings file
whose `env` holds the template's variables and each use-only secret as its
handle, never a value) and `instructions.md` into `--out`, refusing any that
exists; keeps the template under `templates/`; and appends one
`lys.harness_event` of kind `template_render` beside the session's context
path, naming the template hash, the session head hash and a manifest block
of the written paths, without moving the head. The report carries the launch
line, `claude --resume <out>/<uuid>.jsonl --fork-session --mcp-config
<out>/mcp.json --settings <out>/env.json --append-system-prompt-file
<out>/instructions.md` plus the template's flags; the tool never runs it. A
template marking a secret readable is refused naming `secret_reader_unbuilt`
and SECRETS-002, since no broker reader exists yet. The same template and
session write the same bytes twice. After the event, `render-launch` appends
one `lys.given` entry under it: the documents Claude Code 2.1.283 loads for
the rendered working directory (the user `CLAUDE.md` under the config
directory, the `CLAUDE.md` chain, the memory index) and the two written
files it is given (`instructions.md`, `mcp.json`, by their out-relative
paths), each as kind, path, byte length and SHA-256 in the measured order,
with the config directory (the template's `CLAUDE_CONFIG_DIR`, or
`HOME/.claude` from the rendering process's `HOME`) and the names the
environment file sets. Nothing of a document is kept, and a document the
harness would read that cannot be read fails the render by path.

`given --home <dir> --session <id>` reports every `lys.given` entry of the
session in file order, each with its entry id, harness and version, config
directory and source, the kinds resolved and unlisted, the environment names
and each document's kind, path, length and sha256. `given-check --home <dir>
--session <id> --entry <id> --path <listed path> --file <file>` hashes the
file and answers `matches` when its SHA-256 and length equal the listed
document's, `differs` otherwise, and exits as `diff` does: 0 on matches, 1
on differs, 2 on a refusal (a missing argument, a session or file that
cannot be read, an entry id or listed path not in the session). Neither
prints a byte of either file.

What the crate does not do: interpret, print or log transcript contents (errors
and reports carry ids, hashes, offsets and counts only); sign, hash into a lys
log or anchor; encrypt; move a home between devices; run or supervise an
agent; talk to Norn.

Design and briefs: `docs/design/home/` (HOME-001, HOME-002, HOME-003). Pi
reference: the checkout
at `3d5cbe98`, `packages/coding-agent/src/core/session-manager.ts`.
