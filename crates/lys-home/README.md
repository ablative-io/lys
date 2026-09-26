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
| `lys.lantern`       | a lantern: a note on a point of this session (`point`, `note`, `lit_by`, `lit_at`), lit on purpose at the head |
| `lys.lantern_epilogue` | further words on a lantern of this session (`lantern`, `words`, `added_by`, `added_at`), appended after it |
| `lys.forked_from`   | the child's ancestry after a fork (`parent_session`, `lantern`, `point`, `cut_at`, `coordinate_carried`, `carried`, `seed_left_out`), the first entry the fork writes after the copied chain |
| `lys.fork`          | one fork taken from this session (`child`), appended at the head |

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
and a loss account beside the file; the same session head with the same
target writes the same bytes, since a record whose entry id is not
uuid-shaped, the importer's `<uuid>-r<i>` for a split tool result or a
hand-authored id, carries a UUIDv5 derived under the session's namespace
over `<entry id>#record`, never a fresh id, so two sessions with one such
entry id never derive one uuid and a rendered file is told by its hash),
`fewshot` writes a hand-authored file
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

`fork --home <dir> --lantern <id> [--session <id>]` forks a child session
from a lantern's point: it resolves the lantern to the session it was lit
in (its data's `lit_in`, or the one session holding an older record; several
holders are refused `lantern_ambiguous` until `--session` names one), cuts
that session's root-to-point chain at the last assistant message at or
before the point, and writes a child under the parent's `cwd` whose
header's `parentSession` is `sessions/<parent>.jsonl`, holding the parent
file's own line bytes for each cut entry, then `lys.forked_from` as its
head; the parent gains one `lys.fork` entry at its head. No block is
written and nothing else is copied. The report is ids and counts: the
child, the parent, the lantern, the point, the cut entry, the entries
copied, the distinct block hashes the copied entries name that the store
holds and those it does not, and whether and which entry was carried. A
lantern at a user message is carried, not copied: `render` and
`render-launch` of that child write the message's text parts beside the
rendered file as `<stem>.seed.txt` under an in-band marker line and report
its path; the printed launch line comes from the template's render only:
`render-launch` appends `"$(cat '<seed>')"` to the template's line as the
first prompt and lists the seed in its manifest, and `render` prints no
launch line.
A lantern before any assistant message is refused `nothing_to_fork`.

What the crate does not do: interpret, print or log transcript contents (errors
and reports carry ids, hashes, offsets and counts only; a lantern's note and
its epilogues are the one text the crate prints, and only `lantern recall`
prints them); sign, hash into a lys
log or anchor; encrypt; move a home between devices; run or supervise an
agent; talk to Norn.

Design and briefs: `docs/design/home/` (HOME-001, HOME-002, HOME-003,
HOME-004, HOME-006). Pi
reference: the checkout
at `3d5cbe98`, `packages/coding-agent/src/core/session-manager.ts`.
