# PROOF-LAUNCH: a session launched through a kept template (HOME-002 R8)

Measured 26 September 2026, 13:01 to 13:02 AEST, on this Mac, with the
lys-home binary built from card/home-002-launch-template at 338d3ba.

## The session, the template, the render

| field | value |
| --- | --- |
| Claude Code version | `2.1.283 (Claude Code)` (`claude --version`) |
| session | a synthetic session written in Pi's grammar in this session's scratchpad (`<scratch>/proof/home/sessions/proof.jsonl`): a header and 2 message entries (1 user, 1 assistant on `claude-opus-5-5`), the second setting a codeword; held nowhere in the tree and not the PROOF-RESUME session (see the note at the end) |
| session head hash | `85decc910e0497cbcff898feb84787a560bfa6abc4a3c3d34b91a296b15edd82` (the head entry's line, newline included) |
| proof template | `<scratch>/proof/template.json`, hash `38322da990c441add9066bf7f64f3df7633e23a0708193a01d8a0c2a0357e249`; flags `--strict-mcp-config --allowedTools 'Bash(printenv:*)'`; canon null; `mcp` = `{"mcpServers": {}}`; `env` = one variable; `secrets.use_only` = one entry, variable `LYS_PROOF_TOKEN`, handle `handle-proof-0001`; `secrets.readable` empty; `reader` empty; instructions = one line asking every reply to end with a fixed token |
| render-launch | `lys-home render-launch --home <scratch>/proof/home --session proof --template <scratch>/proof/template.json --uuid 7a5d0c2e-1b3f-4e6a-8c9d-2f4b6a8c0e1d --cwd /proof --model claude-opus-5-5 --version 2.1.283 --out <out>`, run with `LYS_PROOF_TOKEN` set in its environment to a value that appears in none of the files below (0 matches by byte search, and 0 in the report) |
| report | `records` 2, `thinking_kept` 0, `thinking_as_text` 0, `dropped` 0, `authored` false, `inherited` 0; manifest block `fbf4528bf71674067665a250ead281fbfe1d11cff1b7f848b343e025d3f72e63`; event id `740d28fb3dfec6943ffc01822c804e29`, hung under the head, which did not move |
| template kept | `<scratch>/proof/home/templates/38/38322da9…` (the hash the report names) |

The five files under `<out>`, in write order, with their SHA-256:

| file | sha256 |
| --- | --- |
| `7a5d0c2e-1b3f-4e6a-8c9d-2f4b6a8c0e1d.jsonl` | `129648fdc1b57f2d58115026dadfa0eb1eb1ad23e3c1bb285ec6a10f846354ab` |
| `7a5d0c2e-1b3f-4e6a-8c9d-2f4b6a8c0e1d.loss.json` | `afa0a3140955be201a62026b69ae77941d9889de6b12f509ee33712b8aadd3ef` |
| `mcp.json` | `d8e397af03b5b032f21d0aa967086f0c78b33c87b76f2e9898ae0a144df7de02` |
| `env.json` | `a6115969e666abfaff5ea366eceddc09b68cb0a347143bb75cb18432533ddc44` |
| `instructions.md` | `be1b6e8f168acaa52ad95aca38bc8defa1e378d0fe90ad496499b8beb9bc5d6e` |

`env.json` parses as `{"env": {"LYS_PROOF_MODE": …, "LYS_PROOF_TOKEN": "handle-proof-0001"}}`: the handle, never the value.

## The launch

The report's launch line, with the out directory written as `<out>`:

```
claude --resume <out>/7a5d0c2e-1b3f-4e6a-8c9d-2f4b6a8c0e1d.jsonl --fork-session --mcp-config <out>/mcp.json --settings <out>/env.json --append-system-prompt-file <out>/instructions.md --strict-mcp-config --allowedTools 'Bash(printenv:*)'
```

It was run once, by hand and not by the tool, from `<scratch>/proof/elsewhere` (neither `<out>` nor the session's cwd `/proof`), in print mode so it is non-interactive: the line above followed by `-p --max-turns 4 --output-format json` and one question asking the session to run `printenv LYS_PROOF_TOKEN` with the Bash tool and reply with the codeword from earlier in the conversation and the value printed. `LYS_PROOF_TOKEN` was set in the shell's environment to the same value as at the render.

| field | value |
| --- | --- |
| exit | 0, in 15 s (13:01:46 to 13:02:01); `num_turns` 2, `subtype` success, `is_error` false, `permission_denials` empty, `api_error_status` null |
| stderr | one warning that no stdin data arrived in 3 s (print mode with no pipe); nothing about MCP, settings or the prompt file |
| rendered file before | `129648fdc1b57f2d58115026dadfa0eb1eb1ad23e3c1bb285ec6a10f846354ab` |
| rendered file after | `129648fdc1b57f2d58115026dadfa0eb1eb1ad23e3c1bb285ec6a10f846354ab` (equal) |
| `<out>` after | the same five files, no sixth: nothing was written beside the rendered file |
| continuation | `~/.claude/projects/-private-tmp-claude-501--Users-tom-Developer-archie-34c4f280-dc08-4edc-9fe0-82f1f75bee3b-scratchpad-proof-elsewhere/b10dbe85-e232-44ce-a3ba-1a8a99fdd6bc.jsonl`: the run directory's slug directory (created by this run; the projects listing gained exactly that directory), under the uuid Claude Code assigned at the fork, not the rendered file's path; sha256 `181012a7e3241728facc6959fc916b8dedb937a8fced95882343c18a425ea7d0`; read, hashed, left alone |
| continuation lines | 36: the 2 rendered records copied in (same uuids), the new turn (2 user, 3 assistant, 1 tool_use and 1 tool_result), and Claude Code's own records (16 attachment, 3 mode, 3 atis-latch, 3 last-prompt, 2 queue-operation, 1 system, 1 cost-state) |
| assistant models | `claude-opus-5-5` on every assistant record, copied and new |

`lys-home resume-check <out>/7a5d0c2e-….jsonl <continuation>`:

| field | value |
| --- | --- |
| rendered_records | 2 |
| forked_records | 36 |
| forked_new_records | 34 |
| repeated_tool_use_ids | 0 |
| new_tool_uses | 1 (the Bash call the question asked for) |
| exit | 0 |

## The four questions

| question | answer | observation |
| --- | --- | --- |
| Does the continuation's parent link name the rendered session id? | no | The rendered session id `7a5d0c2e-1b3f-4e6a-8c9d-2f4b6a8c0e1d` appears in no field of any of the 36 records: every record, the 2 copied ones included, carries `sessionId` `b10dbe85-e232-44ce-a3ba-1a8a99fdd6bc`, and no record names a parent session. On 2.1.283 the fork is found by the copied records' uuids (2 of 2 present), not by a link to the rendered id; a capture that wants the rendered id must take it from the render's manifest block, not from the fork. |
| Did the appended instructions take effect? | yes | The reply ends with the exact token `instructions.md` asks for. The template does not set `--system-prompt-snapshot`, so Claude Code's default `on` applied: the continuation holds 2 `attachment` records of type `prompt_snapshot` and 1 of type `environment` that mention the snapshot, so the prompt with the appended file was rendered on this fork's first request and recorded there. |
| Did a tool call in the launched session see the environment file's handle? | yes | The one `tool_result` (of the one `Bash` `tool_use`) holds `handle-proof-0001`, and the reply repeats it. The secret value set in the shell's environment appears nowhere in the continuation, the reply or the five files (0 matches): `--settings` `env` replaced the shell's value with the handle in the tool's environment. |
| Was `--mcp-config` accepted? | yes | Exit 0 with `--mcp-config <out>/mcp.json --strict-mcp-config`; stderr and the JSON result say nothing about MCP; the result's `subtype` is `success`. |

The reply also held the codeword the session's earlier turn set (yes), so the rendered history was read.

## What this proves and what it does not

- On 2.1.283 the rendered launch line resumes the rendered file by path from an unrelated directory, forks, and writes its continuation under `~/.claude/projects/<slug of the run directory>/<fork uuid>.jsonl`; the rendered file is unchanged and nothing is written beside it, which is why the launch line carries `--fork-session` (PROOF-FEWSHOT.md measured the bare resume writing beside the passed file).
- The appended instructions, the settings file's environment and the MCP configuration each took effect through the flags the launch line carries.
- It does not prove the brief's premise that the fork's parent link names the rendered session id: on 2.1.283 it does not, and the answer above records it for the capture card to build on.
- Note on the session: R8's spec names the recorded session from PROOF-RESUME; this run used a small synthetic session on the build instruction for this card (print mode, one short question), so the private session was neither imported nor run. Everything the acceptance lists was measured on the launch line as rendered. Running the PROOF-RESUME session through the same line is one more run of the same command when wanted.
