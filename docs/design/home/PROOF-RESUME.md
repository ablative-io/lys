# PROOF-RESUME: a real session imported, rendered, and resumed by path (HOME-001 R5)

Measured 24 September 2026, 15:29 to 15:31 AEST, on this Mac, with the lys-home
binary built from card/home-001 at 19e3ffe8.

## The session

| field | value |
| --- | --- |
| Claude Code version | `2.1.281 (Claude Code)` |
| source file | `~/.claude/projects/-Users-tom-Developer-archie/e1c27f5d-b418-472d-8ea8-b69bea49882f.jsonl`, an Archie session of 11 September 2026 (written by 2.1.268), not running at the time (`pgrep` found no process on it) |
| source size | 562,932 bytes, 242 records: 32 user, 49 assistant, 74 attachment, 16 system, 10 permission-mode, 10 mode, 10 atis-latch, 10 last-prompt, 18 queue-operation, 11 bridge-session, 2 cost-state |
| source hash before | `793f4e87ea4608b2ddfb9d5dcbf1197d4245bde7ed1612630339a1cc1f6d15df` |
| source hash after | `793f4e87ea4608b2ddfb9d5dcbf1197d4245bde7ed1612630339a1cc1f6d15df` (equal, checked after the fork ran) |

## Import (R3, R8)

`lys-home import --home <scratch>/proof5/resume/home --claude-code <source> --session real1`

| field | value |
| --- | --- |
| entries | 205: 81 messages plus 124 harness events (42 attachment, 32 hook, 16 system, 10 permission_mode, 24 tool_completed) |
| hook events | 32 = the file's `hook_success` attachments (32), no `hook_failure` |
| tool_completed | 24 = the file's tool_result parts |
| blocks | 81 content parts, all new to the store |
| bytes in / out | 562,932 / 219,884 |
| counted, left in the original | mode 10, atis-latch 10, last-prompt 10, queue-operation 18, bridge-session 11, cost-state 2 |
| what the first import refused | before R8, this file was refused by uuid: a user record's parentUuid names an attachment record 25 times and a system record 7 times; those records are now entries at their exact place |

## Render for the same model (R4)

`lys-home render --home <home> --session real1 --uuid 478d9617-9050-453d-bb94-b84385905aed --cwd "<the session's cwd>" --model claude-opus-5 --out <scratch>/proof5/resume/rendered/478d9617-9050-453d-bb94-b84385905aed.jsonl`

| field | value |
| --- | --- |
| records | 81 (32 user, 49 assistant), one chain, every assistant record on `claude-opus-5` |
| thinking | 17 blocks rendered whole with their signatures (`thinking_kept` 17, `thinking_as_text` 0, `dropped` 0) |
| loss account | `478d9617-….loss.json`: `dropped: []` |
| tool_use parts | 24 |

## Resume by path with a fork

Run from `<scratch>/proof5/resume/elsewhere`, a directory that is neither the rendered file's directory nor the session's cwd (recorded in `fork-run.log`).

```
claude -p --resume <scratch>/proof5/resume/rendered/478d9617-9050-453d-bb94-b84385905aed.jsonl --fork-session --strict-mcp-config --mcp-config '{"mcpServers":{}}' --max-turns 1 "How many messages did you say were carried over? Reply with only the number."
```

| field | value |
| --- | --- |
| answer | `167` (redacted to one token: the number the session's last assistant turn reported, present nowhere else) |
| exit | 0, in 4 s (15:29:51 to 15:29:55) |
| the fork's uuid | `744ae223-de98-4abb-9b98-ac7409ca0a13` |
| where the fork was written | `~/.claude/projects/-private-tmp-…-proof5-resume-elsewhere/744ae223-de98-4abb-9b98-ac7409ca0a13.jsonl`: with `--fork-session` the continuation goes to Claude Code's own directory for the run's cwd under the fork's uuid, not beside the passed file (PROOF-FEWSHOT.md measured the other case: without `--fork-session` it is written beside the passed file as `<sessionId>.jsonl`) |
| fork lines | 93: 64 records copied from the rendered file (same uuids), the new turn (1 user, 1 assistant on `claude-opus-5`), and 27 of Claude Code's own records (14 attachment, 3 mode, 3 atis-latch, 3 last-prompt, 2 queue-operation, 1 system, 1 cost-state) |
| rendered file after | unchanged |

`lys-home resume-check <rendered> <fork>`:

| field | value |
| --- | --- |
| rendered_records | 81 |
| forked_records | 93 |
| forked_new_records | 29 |
| repeated_tool_use_ids | 0 (an id-duplication count: tool_use ids appearing more times in the fork than in the rendered file; it says nothing about an action repeated under a fresh id) |
| new_tool_uses | 0 (the fork's own records hold no tool_use part at all, which is the stronger statement for a one-turn question answerable without tools) |
| exit | 0 (non-zero when repeated_tool_use_ids is not 0, tested with a synthetic duplicate in `tests/claude_code_round_trip.rs`) |

## The fork imported back (R3)

`lys-home import` of the fork into the same home: 93 records, 105 entries (35 messages, 13 attachment, 1 hook, 1 system, 24 tool_completed events), 66 content parts of which 16 were already held and 50 were new. The 64 copied records' message content is identical to the rendered file's (checked record by record), so the 50 new blocks are not Claude Code's doing: they are the 24 tool_result and 24 tool_use parts, which the renderer writes in its own shape from Pi's toolResult and toolCall messages rather than in the source record's bytes, plus the new turn's 2 parts. The 16 reused are the user and assistant text parts. A renderer that emitted a tool part from the block the importer stored for it would make a round trip add only the new turn; that is a change to R4 for review, not made here.

## What 2.1.281 did with the signed thinking

The rendered file carried 17 assistant records whose content is one signed thinking block each (Claude Code writes thinking as its own assistant record). The fork copied 64 of the 81 records: every user and text record, and none of the 17 thinking records. So 2.1.281 does not carry signed thinking blocks from a resumed file into its continuation. Whether it sent them to the model on the first turn cannot be seen from the files; that is R10's measurement (through the proxy) and PROOF-CANON.md records it as open.

## The authored boundary (R3 acceptance, R5)

From PROOF-FEWSHOT.md's continuation (`<scratch>/proof5/fewshot/60d9c850-….jsonl`, 34 lines): the 6 copied records keep their uuids; the 3 copied assistant records carry model `authored`; the new turn's assistant record carries `claude-opus-5-5`. `tests/claude_code_round_trip.rs` imports a continuation of that shape and checks the path's providers read `authored, authored, authored` then `anthropic`, with exactly one `lys.authored` entry before the first, and that a render of it keeps the same models in the same order.

## Pi's reader on the record (R1 acceptance)

Recorded in PROOF-CANON.md with the canon file, since both are the same check: `parseSessionEntries` from the checkout at `3d5cbe98`, run under node against `home/sessions/real1.jsonl` and the canon.

## The deterministic render (HOME-007)

The render is one function of the session head, the target and the lys-home
version: the same inputs give the same bytes, so a rendered file is told by
its hash. This section pins that on a synthetic fixture and re-measures the
session above.

### The fixture's hash (R3)

`crates/lys-home/tests/fixtures/multi_result.jsonl`, six synthetic records
(SHA-256 `5e9c37f955dee54b215c30adeec521ae4f5827fb80d1b9bb0d395f3daee497fc`):
a question, an assistant record with two tool_use parts, a user record of
exactly two tool_result parts, an assistant record with one tool_use, a user
record of one tool_result beside its own text, and the answer. Built from
lys-home at `396f2b6` (the derivation landed at `8bc7f04`) and run from the
repository root, `<scratch>` a fresh directory:

```
lys-home import --home <scratch>/home --claude-code crates/lys-home/tests/fixtures/multi_result.jsonl --session multi
lys-home render --home <scratch>/home --session multi --uuid aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa --cwd /elsewhere --model claude-opus-5-5 --version 2.1.281 --out <scratch>/a.jsonl
lys-home render --home <scratch>/home --session multi --uuid aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa --cwd /elsewhere --model claude-opus-5-5 --version 2.1.281 --out <scratch>/b.jsonl
shasum -a 256 <scratch>/a.jsonl <scratch>/b.jsonl
```

| field | value |
| --- | --- |
| import | 6 records, 11 entries, 9 blocks new |
| each render | 8 records, `dropped` 0 |
| `a.jsonl` | `798a64be2164b48c30e4e64fa80c4a307687f7808c256e7606be52b2c9f03074` |
| `b.jsonl` | equal to `a.jsonl` |
| `a.loss.json`, `b.loss.json` | `4d9fd3712492331879273558a96c18eecbeb77aa42215100e7d14faff1542f2e`, equal |

`tests/claude_code_round_trip.rs` runs the same import and render through
`lys_home::cli::run` and asserts the rendered file's SHA-256 equals that
constant, written in the test and never computed from the render it checks,
and that this document contains it once. Drift injection: one hex digit of
the value above changed makes exactly that test fail.
