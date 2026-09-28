# PROOF-MOVE: a fixture home shipped, fetched, rendered and resumed (HOME-019 R9)

Measured 28 September 2026, 16:40 to 16:46 AEST, on this Mac, with the
lys-home binary built from card/HOME-019 at d41fa4b (a debug build, its target
directory outside the repository). `<scratch>` below is
`/Users/deanwhiting/archie-scratch/home019-proof`. Every lys-home process
ran with `LYS_FIXTURE_TOKEN` set to the fixture secret's value.

**Status: incomplete.** Every step up to the launch was measured and holds.
The launch line was run by hand once and Claude Code refused to authenticate
(exit 1, below), so the resume row of R9 is not yet met. It is to be
re-measured, with the same commands, once Claude Code on this machine is
logged in again.

| field | value |
| --- | --- |
| Claude Code version | `2.1.283 (Claude Code)` (`claude --version`, printed in this run) |
| git | `/usr/bin/git` |

## The fixture home

`crates/lys-home/tests/fixtures/launch/session.jsonl` copied byte for byte
to `<scratch>/source/sessions/fixture.jsonl`, then one render:

```
lys-home render-launch --home <scratch>/source --session fixture --template crates/lys-home/tests/fixtures/launch/template.json --uuid 7d3f1c2a-5b4e-4c6d-9a8b-0e1f2a3b4c5d --cwd <scratch>/run --model claude-opus-5 --version 2.1.283 --out <scratch>/first-out
```

Exit 0; session head hash `a552633ffeb98db0d02ef79f9800427dd5035c423efb13fd6018e9e360e87e42`;
manifest block `a2c4e50f475c667ff34fced06e953144c97e5559f15764b2371eb6e15cf2de79`;
template `a11ebde597009fe515861c51c32c2d0fd987db9dfa46a08ebf35744d93c646f4`.

The source's tracked set, 5 files, SHA-256 before the ship:

| path | sha256 |
| --- | --- |
| `blocks/a2/a2c4e50f475c667ff34fced06e953144c97e5559f15764b2371eb6e15cf2de79` | `a2c4e50f475c667ff34fced06e953144c97e5559f15764b2371eb6e15cf2de79` |
| `sessions/fixture.head` | `95a46cb0e2527126bca4966d7c106019819b301ceed4de9fc5a37f205c2707dd` |
| `sessions/fixture.index.jsonl` | `c4349cdcf92d76c3780279df210283328d91bdc854176c331dd81a4ac5e03ec6` |
| `sessions/fixture.jsonl` | `81027b8ae3032ff05dfac2d0242cb19208f2d8095c48715402309fca3fa5c5c3` |
| `templates/a1/a11ebde597009fe515861c51c32c2d0fd987db9dfa46a08ebf35744d93c646f4` | `a11ebde597009fe515861c51c32c2d0fd987db9dfa46a08ebf35744d93c646f4` |

## The ship

```
lys-home ship --home <scratch>/source --remote <scratch>/remote.git
```

| field | value |
| --- | --- |
| exit | 0 |
| commit | `18a24328786dd61f620bc241a814507279b5f2fc` |
| ref | `refs/lys/home` |
| remote | `<scratch>/remote.git` (absolute) |
| initialised / remote_created / unchanged / note | true / true / false / empty |
| `git --git-dir <scratch>/remote.git rev-parse refs/lys/home` | `18a24328786dd61f620bc241a814507279b5f2fc` |
| `ls-tree -r --name-only refs/lys/home` | the 5 paths above, in the same order |

## The fetch

```
lys-home fetch --remote <scratch>/remote.git --home <scratch>/target
```

| field | value |
| --- | --- |
| exit | 0 |
| commit | `18a24328786dd61f620bc241a814507279b5f2fc` |
| ref | `refs/lys/home` |
| sessions | 1: `fixture`, execution `72a0260f401ab1a5d3ac2759fe857b9b` |
| target HEAD | `5fb06bf9482840472144d0f55c88b3242de03117`, one parent, `18a24328786dd61f620bc241a814507279b5f2fc` |
| `git -C <scratch>/target status --porcelain` | 0 lines |
| arrival | last line of `<scratch>/target/sessions/fixture.jsonl`: `custom` of `lys.harness_event`, kind `arrival`, parentId the id the head file names, `source_commit` the ship's commit, `remote` `<scratch>/remote.git`, `ref` `refs/lys/home`, `execution` `72a0260f401ab1a5d3ac2759fe857b9b`; the session file went from 7 lines to 8 |

The SHA-256 of every file at the fetched commit (`git cat-file blob
18a24328…:<path>` in the target) equals the table above, 5 of 5. The
source's tracked set after the fetch equals the table above, 5 of 5.

| head file | sha256 |
| --- | --- |
| source `sessions/fixture.head` | `95a46cb0e2527126bca4966d7c106019819b301ceed4de9fc5a37f205c2707dd` |
| target `sessions/fixture.head` | `95a46cb0e2527126bca4966d7c106019819b301ceed4de9fc5a37f205c2707dd` |

## The secret search

Every object `git --git-dir <scratch>/remote.git rev-list --objects
refs/lys/home` lists, read through `git cat-file <type> <id>` and searched
for the fixture secret's value by bytes:

| repository | objects read | matches |
| --- | --- | --- |
| `<scratch>/remote.git` at `refs/lys/home` | 12 (5 blobs, 6 trees with the root, 1 commit) | 0 |
| control: `<scratch>/scratch.git`, one blob holding the value planted, committed on `refs/lys/home` | 3 | 1 |

A byte search of every file under `<scratch>/source`, `<scratch>/target` and
`<scratch>/first-out` found the value 0 times.

## The renders of the target and the source

Run after the source's tracked set was measured unchanged (above), each into
its own empty out directory with the same arguments as the first render but
`--out`:

| render | exit | session_head | rendered file sha256 |
| --- | --- | --- | --- |
| target, `--out <scratch>/target-out` | 0 | `a552633ffeb98db0d02ef79f9800427dd5035c423efb13fd6018e9e360e87e42` | `2406a9fa14f687833a654eb58319ac0c226ec31054dda304d6489763e9fd1a42` |
| source, `--out <scratch>/source-out` | 0 | `a552633ffeb98db0d02ef79f9800427dd5035c423efb13fd6018e9e360e87e42` | `2406a9fa14f687833a654eb58319ac0c226ec31054dda304d6489763e9fd1a42` |

All five files of the two out directories are byte-identical pair by pair.
The target's launch line, with `<scratch>/target-out` written as `<out>`:

```
claude --resume <out>/7d3f1c2a-5b4e-4c6d-9a8b-0e1f2a3b4c5d.jsonl --fork-session --mcp-config <out>/mcp.json --settings <out>/env.json --append-system-prompt-file <out>/instructions.md --strict-mcp-config
```

## The launch, run by hand

The line above was run once from `<scratch>/run` by hand, never by lys-home.
It had `-p --output-format json` and one question appended (redacted: it
asks for the last word of the session's final assistant message), and stdin
was `/dev/null`. The rendered file holds 4 records (2 user, 2 assistant). The
expected word was taken from the final assistant record's text, trimmed,
stripped of one trailing full stop and lowercased (4 characters); its SHA-256
is `04efaf080f5a3e74e1c29d1ca6a48569382cbbcd324e8d59d2b83ef21c039f00`.

| field | value |
| --- | --- |
| exit | 1, in 2 s (16:44:25 to 16:44:27) |
| result | `is_error` true, `terminal_reason` `api_error`, `duration_api_ms` 0: Claude Code could not authenticate (its OAuth session had expired and could not be refreshed; `claude auth status` reports `loggedIn` false). No request reached the model |
| reply sha256, normalised | `0d7f15e1a2cf33a9ef9bf9eb7b80bca3d49ff327ab1ea35cab5d3b7378fe23ac` (the refusal text, not an answer): differs from the expected word's |
| `<out>` after | the same five files, the same hashes: nothing was written beside the rendered file |
| continuation | `~/.claude/projects/-Users-deanwhiting-archie-scratch-home019-proof-run/169dade6-a373-486d-9aeb-52a1cdab5e74.jsonl` (a new directory in the projects listing), 25 lines, sha256 `2374ed35e1792f84bc805edd05bbbe255376fa87edebe2143327c3d2dff65169`; read, hashed, left alone |

`lys-home resume-check <out>/7d3f1c2a-5b4e-4c6d-9a8b-0e1f2a3b4c5d.jsonl <continuation>`:

| field | value |
| --- | --- |
| rendered_records | 4 |
| forked_records | 25 |
| forked_new_records | 21 |
| repeated_tool_use_ids | 0 |
| new_tool_uses | 0 |
| exit | 0 |

## What this proves and what it does not

- Ship pushed exactly the tracked set as one ref to a bare repository at a
  path on this machine, and left the source byte-identical. Fetch arrived
  with every tracked file hash-equal, the head file identical, one arrival
  with its own execution id committed on the fetched commit, and a clean
  status.
- No object the shipped ref reaches holds the fixture secret's value, and
  the control finds it where it was planted.
- A render of the fetched session records the same session head as a render
  of the source, and writes the same bytes.
- It does not yet prove the resume. The printed launch line started, and
  resumed and forked the rendered file (the continuation copies it and
  repeats no tool use), but no turn was answered because Claude Code was not
  logged in. The reply-hash row and the exit-0 row of R9 stay open until
  the launch is run again after a login.
