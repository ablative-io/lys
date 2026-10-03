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
`<id>.blocks.jsonl` beside an imported session holds one `{entry, part, hash}`
row per content part the import stored: the entry it went into, its index in
the source record, and the hash the block store returned; never content.

lys adds nothing to Pi's grammar. Its own data rides in Pi's `custom` entries:

| custom type         | what it is                                                        |
| ------------------- | ----------------------------------------------------------------- |
| `lys.call`          | one proxy call: request and response parts by hash, raw bodies by hash, status |
| `lys.harness_event` | a harness-local event: a hook, a system record, a permission mode or a tool completion, with the whole source record as a block |
| `lys.authored`      | this session is a hand-written demonstration, not history         |
| `lys.inherited`     | this entry came from the canon or another session, and says so    |
| `lys.given`         | what a rendered session was given: each instruction document Claude Code loads and each file the render wrote, by kind, path, length and SHA-256 in the measured order, with the config directory and the environment names; never a document's content |
| `lys.given_statement` | a render's signed given statement (`given`, `statement`): the `lys.given` entry it hangs under and the block holding a `lys/attestation/v2` over the RFC 8785 bytes of that entry's data; written only with `--key` |
| `lys.lantern`       | a lantern: a note on a point of this session (`point`, `note`, `lit_by`, `lit_at`, `lit_in`), lit on purpose at the head |
| `lys.lantern_epilogue` | further words on a lantern of this session (`lantern`, `words`, `added_by`, `added_at`), appended after it |
| `lys.forked_from`   | the child's ancestry after a fork (`parent_session`, `lantern`, `point`, `cut_at`, `coordinate_carried`, `carried`, `seed_left_out`), the first entry the fork writes after the copied chain |
| `lys.fork`          | one fork taken from this session (`child`), appended at the head |
| `lys.loss`          | what one compaction summarised, the line directly after it: the span's first and last ids and its entry, message, tool call, tool result and block counts, bytes and block digest; ids, counts and hashes only |

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
--cwd <dir> --model <id> --version <claude code version> --out <dir>
[--key <file>]` turns a
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
<out>/instructions.md --setting-sources= --strict-mcp-config` plus the
template's flags; the tool never runs it. A
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
The report carries `given_sha256`, the SHA-256 of the RFC 8785 bytes of
that entry's data, and `signing`, `signed` or `unsigned`. With `--key <file>`,
a raw 32-byte Ed25519 seed file such as `lys key generate` writes, the render
also signs those bytes as a `lys/attestation/v2` statement, keeps it as a
block named by one `lys.given_statement` entry under the `lys.given` entry,
writes `given-statement.cose` and `given-data.json` into `--out` for `lys
verify --attestation given-statement.cose --payload given-data.json`, and
reports `statement`, `statement_file` and `payload_file`. The key is loaded,
and the two files refused if they exist, before anything is written; an
unreadable key fails by its path and never prints a key byte.

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

`translate-codex --home <dir> --session <id> --out <dir> --codex-version <version> [--zone <iana name>]`
translates a session into a rollout in the shape Codex 0.156.0 writes for
its own threads, the one pair Claude Code to Codex. `--out` is a Codex home
directory: the rollout goes to
`sessions/YYYY/MM/DD/rollout-<local date and time>-<thread>.jsonl` under it,
the date and time the head entry's stamp in `--zone` (read from `TZ` when
the flag is absent, resolved through the time zone database bundled in the
build), with the loss account beside it as `<same stem>.loss.json`. The
thread id is the head's record uuid, so each head makes a new thread; the
thread is a fork of the session and says so in its first message. Every
text part, tool call and tool result on the context path is carried whole,
readable thinking as text, a base64 image as `input_image`; a compaction,
branch summary or custom message on the path becomes marked developer
text, and an `agent` sidechain beside the path one marked developer message.
The account names by entry id and hash what was kept, what changed and how,
and what was lost and why, and holds no content. Once both files are
written, one `lys.translation` side leaf records the translation beside the
context path; the head does not move. Only `0.156.0` is accepted, an
existing target is refused, and a stamp that is not RFC 3339 is refused
naming the entry. The report is the two paths and the counts; Codex is
never run.

`ship --home <dir> --remote <path>` makes the home a git repository
(`<home>/.git`, on first use) tracking exactly its tracked set: for each
session, `sessions/<id>.jsonl`, `sessions/<id>.index.jsonl` and
`sessions/<id>.head`; each `blocks/<hh>/<hash>`; each
`templates/<hh>/<hash>`; never a lock file, a temporary or a file at the
home root. It commits that set on the one ref `refs/lys/home` (no new
commit when the home is unchanged since the last ship) and pushes it
without force to a bare repository at `--remote`, a path on this machine,
making one there when the path does not exist. Git runs as the binary on
`PATH` with a pinned environment: no system or global configuration, no
hooks, no filters, no line-ending conversion, the identity
`lys-home <lys-home@invalid>` and the file protocol only. The report is
`{"command": "ship", "report": {commit, ref, remote, initialised,
remote_created, unchanged, note}}`. Its refusals, each by name and each
pushing nothing: `remote_not_local` (a URL of any scheme or an scp-style
`host:path`; shipping off this machine waits for stage 3's encryption from
the secrets step), `remote_not_bare` (naming the path and whether it found a
file, a non-bare repository or a directory that is not a bare repository),
`empty_home`, `session_held` (a live owner holds a session; ship after that
seat stops), `index_missing` and `head_missing` (naming the session, the
missing file and `lys-home given`, which writes it), `stale_index` (printed
on stdout as a JSON report naming each session and its reason,
`index_not_this_file` or `head_not_indexed`), `foreign_tracked` (the home's
repository tracks a path outside the set), `ref_diverged` (the remote's ref
names a commit the new one does not descend from; the remote is left as it
was) and `git_failed` (a git command exited non-zero, named with its exit
code and none of its output). Ship writes no file of the tracked set, no
index and no head, and offers no way to leave a session out.

`fetch --remote <path> --home <dir>` pulls `refs/lys/home` into a new
home: `--home` is absent or an empty directory. It verifies every session's
index and head strictly (never rebuilding an index) and hashes every block
and template, then appends one `lys.harness_event` of kind `arrival` beside
each session's head, `{source_commit, remote, ref, execution}` with a fresh
execution id of that session's own, and commits those lines as one commit
whose only parent is the fetched commit, so `git status` in the new home is
clean and a later ship from it carries the arrivals. The report is
`{"command": "fetch", "report": {commit, remote, ref, sessions: [{session,
execution}]}}`. Its refusals: `remote_not_local`, `target_holds_home` (the
target holds `sessions/`), `target_not_empty`, an arrival over the 512-byte
event cap, all before anything is written; `verification_failed` (printed
on stdout as a JSON report naming each session and its reason and each bad
block and template by hash, never by content); `arrival_failed` (an append
or a step of the arrival commit failed, named by the session or the git
subcommand); and `git_failed`. A fetch that refuses after its first write
removes exactly the paths it created, deepest first: a target it made is
gone, and an empty target that stood before is left existing and empty.

`lys proxy serve --listen <addr> --home <dir> --state <dir> [--upstream <file>]
[--anthropic <url>] [--openai <url>]` is the little proxy (HOME-001 R10).
A harness pointed at `http://<listen>/anthropic` (or `/openai`) has every
call forwarded to the provider with its headers and streamed body unchanged
(only `host` is set to the upstream's), frame by frame, through one transport
that never sends a request twice. Each model call (a `POST` to
`/v1/messages`, `/v1/chat/completions` or `/v1/responses`) is journalled
under `<state>/journal` before it is sent, and a journal that cannot be
written refuses the call by name with nothing sent. The call's bodies are
spooled under `<state>/capture` while they pass, without a capture-slot limit.
When a call ends, it is recorded as one
`lys.call` in the home. The session is the one named by the key in the
call's own body (`metadata.user_id`, read once as the body passes),
or `unlinked-<day>` when the body carries no key; the proxy never infers a
session. The status is `complete`, `cancelled`, `partial`, `unrecorded` or
`lost`, and only `complete` carries response parts, which are assembled from
the event stream by the api's grammar. A call a previous run left open is
recorded `lost` once at the next start. One JSON report line per call goes
to stdout: ids, the session, the status and counts. The pass-through
`examples/passthrough.rs` shares the transport, for the subscription proof
in `docs/design/home/PROOF-PROXY.md`, which is still to be run.

Secrets: no credential lys-home holds is ever written into the home; a
use-only secret appears only as its handle. That is the one secret guarantee
ship makes. Ship does not scan the tracked files for secrets: blocks hold
imported tool results verbatim and a template's env and mcp slots are
free strings, so a secret a tool printed or a person typed can be in a
home, and no scan can promise to find every such value. That is why ship takes only a
path on this machine.

`compactions --home <dir> --session <id>` lists a session's compactions in
path order: for each, whether its summary is present, the compaction it
completes or is completed by, its first kept entry, its `lys.loss` entry, and
the span that entry names read back entry by entry through the index with
its blocks checked as held by hash. It prints one JSON report
`{"command": "compactions", session, blocks_verified, unreadable,
compactions: [...]}` of ids, counts and hashes, then exits 1 when an entry
could not be read, a block is missing, the session has no block rows file
(`blocks_verified` false), or a compaction has no loss entry (its reason says
so); 0 only when every check held. It writes nothing and starts nothing.

What the crate does not do: interpret, print or log transcript contents (errors
and reports carry ids, hashes, offsets and counts only; a lantern's note and
its epilogues are the one text the crate prints, and only `lantern recall`
prints them); sign, hash into a lys
log or anchor; encrypt; move a home between devices; run or supervise an
agent; talk to Norn.

Design and briefs: `docs/design/home/` (HOME-001, HOME-002, HOME-003,
HOME-004, HOME-006, HOME-009, HOME-019, HOME-030). Pi
reference: the checkout
at `3d5cbe98`, `packages/coding-agent/src/core/session-manager.ts`.
