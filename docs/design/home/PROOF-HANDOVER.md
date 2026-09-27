# PROOF-HANDOVER: an elicited letter handed to a successor home (HOME-015 R8)

The handover measured on a letter elicited for the purpose, not the walrus
turn. Every figure below is an id, a hash, a count, a version, a command, a
path or an exit code. No text of the letter, its thinking or any reply is
written here.

## Where it ran

| field | value |
| --- | --- |
| Claude Code version | `2.1.283 (Claude Code)` measured, beside `2.1.281`, the version the row named |
| lys-home | the `lys-home` binary built with `cargo +1.97.1 build -p lys-home` at `c3a4053` on the build machine; `<proof>` below is a fresh scratch directory there |
| Claude Code on the build machine | `claude -p` exit 1, `is_error` true, `terminal_reason` `api_error`: its sign-in had expired, so no session could be elicited there |
| Claude Code runs | the elicitation and the resume ran on the development Mac with the same version, `2.1.283 (Claude Code)`; the session file was copied to the build machine by `scp` and the rendered file copied back, each compared by SHA-256 on both sides |

## The letter (R8 step 2)

Elicited with thinking on, from a fresh directory `<letter>`, not a running
session:

```
MAX_THINKING_TOKENS=6000 claude -p --strict-mcp-config --mcp-config '{"mcpServers":{}}' --max-turns 1 --output-format json "<a request for a letter to its successor: what it knows, what it got wrong and why, how the people like things done, what it wishes it had known>"
```

| field | value |
| --- | --- |
| exit | 0 |
| session id | `58e5ebb5-5d4d-429d-afec-6f3ab513f9ef` |
| model | `claude-opus-5-5` |
| session file | `~/.claude/projects/<letter slug>/58e5ebb5-5d4d-429d-afec-6f3ab513f9ef.jsonl`, 27 records, SHA-256 `632973d765a27c4fd9227cd298841194ed1bf37be456cb738ea95f08e35f2a68` on both machines |
| the final turn | 2 assistant records after the last user record: `5bf1b291-d9dd-49ca-a056-a173251a352a` (1 thinking part, 0 bytes of thinking text, a signature of 1980 bytes) and `67365cbb-b826-48d0-b649-f3628ca20ee6` (1 text part) |

2.1.283 wrote the turn's thinking signed, with its thinking text empty: the
signature is the only content of the thinking part, so the byte-equal line
below is measured on that signature and on the part as a whole.

`lys-home import --home <proof>/home --claude-code <proof>/source.jsonl --session letter1`: exit 0, 27 records, 18 entries, 3 blocks. On the root-to-head path the two letter entries are adjacent, `67365cbb-…` the child of `5bf1b291-…`.

## The handover (R8 step 2)

```
lys-home handover --home <proof>/home --from letter1 --letter 5bf1b291-d9dd-49ca-a056-a173251a352a 67365cbb-b826-48d0-b649-f3628ca20ee6 --successor <proof>/successor
```

| field | value |
| --- | --- |
| exit | 0 |
| successor session id | `d28d8e772b87c7c848c43fbde6757d51` |
| successor file | `<proof>/successor/sessions/d28d8e772b87c7c848c43fbde6757d51.jsonl`, 5 lines: the header, `custom` `lys.inherited`, 2 `message`, `session_info` |
| report | `inherited` true; keys `successor_home`, `session`, `file`, `inherited` |
| `lys.inherited` data | 8 keys, no `rule` |
| `session_info` | 5 keys (`type`, `id`, `parentId`, `timestamp`, `name`), `parentId` `67365cbb-b826-48d0-b649-f3628ca20ee6` |
| outgoing home | 22 files; the SHA-256 of every file equal before and after a second handover of the same letter to `<proof>/successor2` (exit 0), the listing's own SHA-256 `1f19be8ce1027248fe77b5552ac37593f16381c3581275e3f472725c83a67877` |
| a letter of `67365cbb-…` alone | exit 1, `letter_without_thinking`, `<proof>/successor3` absent afterwards |

Each copied thinking part, hashed as its compact JSON (the loss account's
hash) and its `thinkingSignature` as UTF-8 bytes:

| entry | what | source SHA-256 | successor SHA-256 | verdict |
| --- | --- | --- | --- | --- |
| `5bf1b291-d9dd-49ca-a056-a173251a352a` | part 0 | `48e9c243152b99c98943bf07d35d89ea7cbbfe0f6b7a5f2db93ebe10b13f0c8e` | `48e9c243152b99c98943bf07d35d89ea7cbbfe0f6b7a5f2db93ebe10b13f0c8e` | equal |
| `5bf1b291-d9dd-49ca-a056-a173251a352a` | part 0 signature | `3091e1622513ce7e832e571b7b193aa4c1ae2fb34a6a59ebe2e6f02f3f1b76a5` | `3091e1622513ce7e832e571b7b193aa4c1ae2fb34a6a59ebe2e6f02f3f1b76a5` | equal |

The text entry `67365cbb-…` holds no thinking part; its message is equal to
the source as a JSON value.

## The carry-over (R8 step 3)

Rendered for the letter's own model on the build machine:

```
lys-home render --home <proof>/successor --session d28d8e772b87c7c848c43fbde6757d51 --uuid 5a1e0b0c-1d2e-4f30-8a41-526374859607 --cwd <proof>/elsewhere --model claude-opus-5-5 --out <proof>/rendered/5a1e0b0c-1d2e-4f30-8a41-526374859607.jsonl
```

| field | value |
| --- | --- |
| exit | 0 |
| report | `records` 2, `thinking_kept` 1, `thinking_as_text` 0, `dropped` 0 |
| rendered file | SHA-256 `a8b4b3e03c21969a19a9344ce7f327ad17da4a045e8bc43c2575bc706fe5626f` on both machines, and unchanged after the resume |
| rendered signature | SHA-256 `3091e1622513ce7e832e571b7b193aa4c1ae2fb34a6a59ebe2e6f02f3f1b76a5`, equal to the inherited signature |

Resumed from `<elsewhere>`, a fresh directory that is neither the rendered
file's directory nor the letter's:

```
claude -p --resume <rendered>/5a1e0b0c-1d2e-4f30-8a41-526374859607.jsonl --fork-session --strict-mcp-config --mcp-config '{"mcpServers":{}}' --max-turns 1 --output-format json "<a one-word reply>"
```

| field | value |
| --- | --- |
| exit | 0 |
| continuation id | `6c439bed-cc43-46e5-b828-d569f955d1da` |
| continuation file | `~/.claude/projects/<elsewhere slug>/6c439bed-cc43-46e5-b828-d569f955d1da.jsonl`, 31 lines, SHA-256 `93bd3a1588e5ba43f4eea4104e33475cfd2c4f9c248fbf77d660e34f362a7e30` |
| records copied from the rendered file | 1 of 2: the text record; the thinking record was not copied |
| thinking parts whose signature's SHA-256 is `3091e1622513ce7e832e571b7b193aa4c1ae2fb34a6a59ebe2e6f02f3f1b76a5` | 0 |
| carry-over | absent: the inherited signed block is not in the continuation's own file, as PROOF-RESUME.md measured 0 of 17 on 2.1.281 |
| thinking parts in the file | 1, the new turn's own, SHA-256 of its signature `221e32c14ce80eabff7e588ca2ae790f4af6f83f52d361b6dafee7815c971d3a` |

Whether the inherited block was sent to the model is left open: it waits on R10's proxy, which records the request itself.

Rendered for another model (`--model claude-other-model --uuid 6b2f1c1d-2e3f-4041-9b52-637485960718`): exit 0, `records` 2, `thinking_kept` 0, `thinking_as_text` 0, `dropped` 1, the loss account naming hash `48e9c243152b99c98943bf07d35d89ea7cbbfe0f6b7a5f2db93ebe10b13f0c8e` with reason `empty thinking dropped`. The render's rule turns readable thinking into a text part; this letter's thinking text is empty, so the rule drops the part and names it, and the synthetic fixture's gate (`a_successor_rendered_for_another_model_carries_its_thinking_as_text`) is what measures the text-part path.

## The seeded and plain card runs (R8 step 4)

Not run. The seeded and plain counts of fix rounds and unverified claims for
one card need a way to launch a session seeded with the successor; HOME-002
adds no handover slot to the launch template, and that slot is the later unit
that makes the two counts measurable.
