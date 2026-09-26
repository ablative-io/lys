# PROOF-GIVEN: the instruction load order measured, and one real render recorded (HOME-003 R8)

Measured 26 September 2026, 13:43 to 13:55 AEST, on this Mac, with the
lys-home binary built from card/home-003-context-record at b196569 (the R6
commit, rebased onto lys main a1160e4; the commits after it change documents
only) and the Claude Code installed here. Scratch files live under
`<scratch>/proof-given` (`<scratch>` is this session's scratchpad); none is
committed. Nothing here quotes a line of any document: paths, counts, orders
and hashes only.

## The version

| field | value |
| --- | --- |
| Claude Code version | `2.1.283 (Claude Code)` (`claude --version`, this Mac) |
| the version the brief measured | 2.1.283: the same, so the constant `MEASURED_VERSION` in `harness/claude_code/given.rs` stays 2.1.283 and the order below is the re-measurement on it |

## The method

Three runs of `claude -p` pointed at a local listener, from a fixture tree
under `<scratch>/proof-given/measure` written fresh before each run by
`measure.py` there, with every other input removed from the environment
(`env` held only `PATH`, `TMPDIR`, `TERM`, `LANG`, `HOME`,
`CLAUDE_CONFIG_DIR`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_API_KEY` set to a
fixture string, and the three telemetry-off variables):

- `HOME` = `measure/home`, empty, so `HOME/.claude` holds nothing;
- `CLAUDE_CONFIG_DIR` = `measure/config`, holding `CLAUDE.md` (the user file)
  and a memory index under **two** slugs of the working directory, one under
  the rule this card ships (every character outside ASCII letters and digits
  becomes `-`) and one under the earlier `/`-only rule;
- the working directory `W` = `measure/anc.dot/w_us`, chosen so its path holds
  a `.` and a `_`; `W` holds `CLAUDE.md`, `.claude/CLAUDE.md` and
  `CLAUDE.local.md`, and its parent `anc.dot` holds `CLAUDE.md`;
- `--append-system-prompt-file measure/instr.md` and
  `--mcp-config measure/mcp.json --strict-mcp-config`, the MCP file naming one
  stdio server whose command is `/usr/bin/true`;
- `-p "Reply with the single word ok." --output-format json --max-turns 1`.

Each fixture file carries one unique marker token. The listener answers every
`POST` with a fixed six-event stream and writes the request body to
`measure/run<n>/req-<k>.json`; the bodies stay there and are not printed. Two
things were read after each run: each file's access time (this volume updates
`atime` on the first read after a write, so the files are rewritten before
every run), which gives the order the harness read the files; and the request,
which gives the order the harness placed them in what it sent (the `system`
array first, then `messages`, first occurrence of each marker).

| run | exit | requests captured | seconds | stderr |
| --- | --- | --- | --- | --- |
| 1 | 0 | 1 (`POST /v1/messages?beta=true`, model `claude-opus-5-5`, 3 system blocks, 2 messages) | 6.3 | one line, the no-stdin warning of print mode |
| 2 | 0 | 1 (same) | 6.2 | same |
| 3 | 0 | 1 (same) | 5.3 | same |

## The order measured

Identical in all three runs.

By read time (first access), the files in order:

1. `<config>/CLAUDE.md` (`user_claude_md`);
2. `W/.claude/CLAUDE.md`, read early, before the files below and out of its
   directory's turn, in every run;
3. `instr.md` (`appended_instructions`);
4. `mcp.json` (`mcp_config`), within 12 microseconds of the instructions file;
5. `anc.dot/CLAUDE.md`, then `W/CLAUDE.md`, then `W/CLAUDE.local.md`
   (`claude_md_chain`, outermost ancestor first);
6. `<config>/projects/<slug>/memory/MEMORY.md` (`memory_index`) under the
   slug `…-measure-anc-dot-w-us`. The index under the `/`-only slug
   `…-measure-anc.dot-w_us` was never read in any run (its access time never
   moved), which is the slug rule R1 ships.

By the request: the appended instructions are the third `system` block; every
`CLAUDE.md`-family file and the memory index are inside the first text block
of the first user message, in this order: `<config>/CLAUDE.md`, then
`anc.dot/CLAUDE.md`, `W/CLAUDE.md`, `W/.claude/CLAUDE.md`, `W/CLAUDE.local.md`,
then `MEMORY.md`. The MCP server's name appears nowhere in the request (a
server that exits at once contributes no tool), so the MCP configuration's
position is known from the read order only.

What this settles for the entry, and how it stands against the brief's order:

- Within one directory the request gives `CLAUDE.md`, `.claude/CLAUDE.md`,
  `CLAUDE.local.md`, while the read order differs (`.claude/CLAUDE.md` first
  here); the entry records the request's order, as the brief rules.
- Across kinds the read order is the brief's list exactly: user file,
  appended instructions, MCP configuration, chain, memory index; the entry
  lists them so, and `MEASURED_VERSION` stays 2.1.283.
- The request differs from that list in one place: it places the appended
  instructions (in `system`) ahead of the user `CLAUDE.md` (in the first
  message), whereas the brief's list puts the user file first. This proof
  records that difference for the lead; the entry follows the brief's list
  as signed off.
- The user file under `CLAUDE_CONFIG_DIR` is given first and once; with
  `HOME` empty there was no `HOME/.claude/CLAUDE.md` to double it.

Fixture file sizes and hashes (the files are the fixture's own, not any
person's documents):

| file | length | sha256 |
| --- | --- | --- |
| `config/CLAUDE.md` | 78 | `f181b149125fd9d4260e5a6ef1300e7160f4522756d0f3cd0e83f17e255e057b` |
| `instr.md` | 86 | `67ba51d7f115e3f44a65b114261566a1477bf1cd26065295f077c3f54dfdf328` |
| `mcp.json` | 78 | `ac34918f22abb327d57bbfa1538fe98a26d3f15e5168b8a48087f125c8987dad` |
| `anc.dot/CLAUDE.md` | 102 | `3149586f084fab87a8e811815d279bea708c149b117706408ed3e61a7a8a1d08` |
| `W/CLAUDE.md` | 88 | `b72dfac32712cc04af48a47cd17bcf2854036a5d0642bf615fb66726b80a6958` |
| `W/.claude/CLAUDE.md` | 97 | `d5e95cd1cc043fc231ddc69b3d8eb25a2f3fb7c997b1fad5e83799b30e9af41d` |
| `W/CLAUDE.local.md` | 91 | `f0f4c76a5c1afc0a382ccac33c901a57934fed0f67785e2c1f6968fb19f5b289` |
| `MEMORY.md` under the shipped slug | 86 | `99eb375f9cde7cd4a8d1ac2217f3387be44339f8f4f807c6c71e69d0aee54ce4` |
| `MEMORY.md` under the `/`-only slug (never read) | 86 | `6976eb10d16ad460bc43bf37f0fb3276656bba50b8c7c2e6dac812dff4caf78b` |

## One real render

The session is the one PROOF-RESUME.md and PROOF-LAUNCH.md measured, the
Archie session of 11 September 2026 at
`~/.claude/projects/<slug dir of the session cwd>/e1c27f5d-b418-472d-8ea8-b69bea49882f.jsonl`,
the file PROOF-RESUME.md names, not running at the time; read, hashed and never written
(`793f4e87ea4608b2ddfb9d5dcbf1197d4245bde7ed1612630339a1cc1f6d15df` before
and after), not committed.

| field | value |
| --- | --- |
| import | `lys-home import --home <scratch>/proof-given/real/home --claude-code <source> --session real1`: 205 entries, 81 blocks, as the two earlier proofs recorded |
| template | `<scratch>/proof-given/real/template.json`, hash `2ba3098cd284e30bb5650554701a27830724a418a3553aa4e1b084b20101ce79`; flags `--strict-mcp-config`; `mcp` `{"mcpServers": {}}`; `env` one variable `LYS_PROOF_MODE`; `secrets.use_only` one entry, variable `LYS_PROOF_TOKEN`, handle `handle-proof-0001`; no `CLAUDE_CONFIG_DIR` set, so the config directory comes from `HOME` |
| render-launch | `lys-home render-launch --home <scratch>/proof-given/real/home --session real1 --template <scratch>/proof-given/real/template.json --uuid 7a2c9e41-5b3d-4f6a-9c8e-2d1b0a4f6e83 --cwd <session cwd> --model claude-opus-5 --version 2.1.283 --out <scratch>/proof-given/real/out`, run with this Mac's own `HOME`; exit 0 |
| report | session head `15f4c163ee20cc01bb666edf4e99da1986eb62be2969dcb6cba47a0c9d7af743` (as PROOF-LAUNCH.md); event `c0187253a0bb6d23fc2688d988b8a959`; manifest `3a7b5fc31b31b9265c34936f16ae334be59a714a55a832001f8f5f2338585e43`; `given` `762dd1a1105cfae493d8b55f7bd126f1`; `given_documents` 5; render records 81, dropped 0 |
| the five written files | `7a2c9e41-….jsonl` `fccd686dd6ffaeda61a197ffca2044092e6704307ef3b498c882a77799ea2b55`; `7a2c9e41-….loss.json` `42cfe6480c618534443ef147a25af53a39287d6f1e0fe3d45e3bb94200d23abd`; `mcp.json` `d8e397af03b5b032f21d0aa967086f0c78b33c87b76f2e9898ae0a144df7de02`; `env.json` `a6115969e666abfaff5ea366eceddc09b68cb0a347143bb75cb18432533ddc44`; `instructions.md` `ab1467d379c4161b16707b68c063ed2a3edc00b18c510798e3365641e7bbf907` |

`lys-home given --home <scratch>/proof-given/real/home --session real1`: one
record.

| field | value |
| --- | --- |
| entry id | `762dd1a1105cfae493d8b55f7bd126f1`, a `lys.given` custom entry whose parentId is the render event `c0187253…`; the session head file still names `e28eef93-…` |
| harness, version | `claude-code`, `2.1.283` |
| config_dir | `~/.claude` (this Mac's `HOME/.claude`), source `home` |
| kinds | resolved `claude_md_chain, user_claude_md, memory_index, appended_instructions, mcp_config, environment_names`; unlisted `claude_md_imports, claude_rules` |
| environment | 2 names: `LYS_PROOF_MODE`, `LYS_PROOF_TOKEN` (the handle and the mode value appear in no member) |
| document count | 5 |

| kind | path | length | sha256 |
| --- | --- | --- | --- |
| `user_claude_md` | `~/.claude/CLAUDE.md` | 1219 | `b34af9bd90f2e32a367a95d71fd8c740cbc7339370a215bbc80bce00f7a2a017` |
| `appended_instructions` | `instructions.md` | 67 | `ab1467d379c4161b16707b68c063ed2a3edc00b18c510798e3365641e7bbf907` |
| `mcp_config` | `mcp.json` | 23 | `d8e397af03b5b032f21d0aa967086f0c78b33c87b76f2e9898ae0a144df7de02` |
| `claude_md_chain` | `<session cwd>/CLAUDE.md` | 1620 | `6b374b1b488edbfa597fdeec042b2473cfe1b1fffccbbf9a8a81f6a5133b7f76` |
| `memory_index` | `~/.claude/projects/<slug dir of the session cwd>/memory/MEMORY.md` | 5636 | `3ba1041425efc73c5788edcdc33d0a8b08baa1ea80d9f724d2d77fbfbbd2a852` |

The two written documents carry their paths relative to `<out>`, and their
hashes equal the manifest's for the same files. The session cwd is four
directories below `/`, and no `CLAUDE.md`, `.claude/CLAUDE.md` or
`CLAUDE.local.md` exists at `/` or at any of its three ancestors other than
the user file itself (listed once, first), so the chain holds one document;
the paths are written as `~` and `<session cwd>` as PROOF-LAUNCH.md writes
them. `HOME/.claude/CLAUDE.md` is the user file here
and is not listed again on the chain.

## The content check

For each of the five documents the probe is its longest line after trimming
leading and trailing whitespace, taken only when that line is at least 40
characters long. 4 documents were probed (longest trimmed lines of 287, 66,
531 and 309 characters) and 1 was skipped, `mcp.json`, whose longest trimmed
line is 16 characters; 4 + 1 = 5, the document count above. A search of this
file for each of the 4 probes finds 0 occurrences (the probes were written to
`<scratch>/proof-given/real/probes.txt` and searched from there; the search
is `grep -c -F -f probes.txt PROOF-GIVEN.md`, answering 0).

## The fixture session under Pi's reader

The R6 fixture (the fixture template with `CLAUDE_CONFIG_DIR` set to a
fixture config directory, a working directory holding one `CLAUDE.md`, a
memory index under that config directory, `HOME` a fresh directory) was
rendered three times through the built binary into
`<scratch>/proof-given/fixture`, giving a session file of 11 lines (the
header and 10 entries: 4 messages, 3 `template_render` events, 3 `lys.given`
entries), sha256
`6681e629b5441252e9662f774d6488f1f963ced8bd1ba1cdb14a8603cbf245c6`.
`lys-home given` on it reports `count` 3, each record's 4 documents equal in
kind, path, length and sha256 across the three.

Pi's own reader at `3d5cbe98` (`loadEntriesFromFile` from
`packages/coding-agent/src/core/session-manager.ts`, which calls
`parseSessionEntries`), run as
`bun run <scratch>/proof-given/pi-parse.ts <scratch>/proof-given/fixture/home/sessions/fixture.jsonl`
(the script prints counts only), answers: header type `session`, id
`fixture`; 10 entries, of which 6 `custom`, 3 with `customType` `lys.given`,
4 `message`; 10 entries carrying `id`, `parentId` and `timestamp`. The 3
equals the count `lys-home given` reports, and the 10 equals the file's
entry lines.

## What this proves and what it does not

- On 2.1.283 the harness reads the user `CLAUDE.md` under `CLAUDE_CONFIG_DIR`,
  the appended instructions file, the MCP configuration, the `CLAUDE.md`
  chain from the outermost ancestor down and the memory index under the
  slug this card ships, in that order; the memory index under the `/`-only
  slug is never read.
- The request places one directory's three files as `CLAUDE.md`,
  `.claude/CLAUDE.md`, `CLAUDE.local.md` whatever order they were read in,
  and places the appended instructions in `system`, ahead of the message
  that carries the instruction files; the entry's cross-kind order is the
  brief's list, which is the read order, and this difference is recorded
  here rather than resolved.
- It does not measure `@`-imports, `.claude/rules`, settings, hook output,
  plugin skills and agents or output styles; the entry names the first two
  as unlisted and the rest are later cards on the same shape.
- The real render's entry names hashes only; the search above finds no line
  of any document in this file.
