# PROOF-LAUNCH: the recorded session launched through a kept template (HOME-002 R8)

Measured 26 September 2026, 13:10 to 13:12 AEST, on this Mac, with the
lys-home binary built from card/home-002-launch-template at 338d3ba.

## The session, imported

The session is the one PROOF-RESUME.md measured: the Archie session of 11
September 2026 at `~/.claude/projects/-Users-tom-Developer-archie/e1c27f5d-b418-472d-8ea8-b69bea49882f.jsonl`,
not running at the time (`pgrep` found no process on it). It is read, hashed
and never written; it is not committed.

| field | value |
| --- | --- |
| Claude Code version | `2.1.283 (Claude Code)` (`claude --version`) |
| source size | 562,932 bytes, 242 records |
| source hash before | `793f4e87ea4608b2ddfb9d5dcbf1197d4245bde7ed1612630339a1cc1f6d15df` |
| source hash after | `793f4e87ea4608b2ddfb9d5dcbf1197d4245bde7ed1612630339a1cc1f6d15df` (equal, checked after the launch ran) |
| import | `lys-home import --home <scratch>/proof-real/home --claude-code <source> --session real1`: 205 entries (81 messages, 42 attachment, 32 hook, 16 system, 10 permission_mode, 24 tool_completed events), 81 blocks all new, bytes in / out 562,932 / 219,884, as PROOF-RESUME.md recorded |
| session head | entry `e28eef93-7551-473a-b881-123c826e0b46`; session head hash `15f4c163ee20cc01bb666edf4e99da1986eb62be2969dcb6cba47a0c9d7af743` (the head entry's line, newline included) |

## The template and the render

| field | value |
| --- | --- |
| proof template | `<scratch>/proof-real/template.json`, hash `38322da990c441add9066bf7f64f3df7633e23a0708193a01d8a0c2a0357e249`; `harness` `claude-code`; flags `--strict-mcp-config --allowedTools 'Bash(printenv:*)'`; `transcript` fill `resume-by-path`, canon null; `mcp` = `{"mcpServers": {}}`; `env` = one variable `LYS_PROOF_MODE`; `secrets.use_only` = one entry, variable `LYS_PROOF_TOKEN`, handle `handle-proof-0001`; `secrets.readable` empty; `reader` empty; `instructions` = one line asking every reply to end with a fixed token; `--system-prompt-snapshot` not set |
| render-launch | `lys-home render-launch --home <scratch>/proof-real/home --session real1 --template <scratch>/proof-real/template.json --uuid 9c1e4a7b-3d5f-4a2c-8e6b-1f0d2c4a6e8b --cwd /Users/tom/Developer/archie --model claude-opus-5 --version 2.1.283 --out <out>`, with `LYS_PROOF_TOKEN` set in its environment to a value that appears in none of the five files and not in the report (0 matches by byte search in each of the six) |
| report | `records` 81 (32 user, 49 assistant), `thinking_kept` 17, `thinking_as_text` 0, `dropped` 0, `authored` false, `inherited` 0; loss account `dropped: []`; manifest block `b5c540ddc6f691c767fd0360e21646fe5d827f364c2e5596e91aab03a0642479`; event `21cc155252be1c9bfe06da50d90855e6`, a `template_render` entry whose parentId is the head `e28eef93-…`; the head file still names `e28eef93-…` after the render |
| template kept | `<scratch>/proof-real/home/templates/38/38322da9…`, the hash the report names |

The five files under `<out>` (`<scratch>/proof-real/out`), in write order:

| file | sha256 |
| --- | --- |
| `9c1e4a7b-3d5f-4a2c-8e6b-1f0d2c4a6e8b.jsonl` | `46b6b9e741f05dc089848de20562e9b6cf2a7d49fb355a707b3945e38bf97879` |
| `9c1e4a7b-3d5f-4a2c-8e6b-1f0d2c4a6e8b.loss.json` | `fcf927ae1adacecd173572b707db98cd1bacaae5bf4564475938ba2cd0edf342` |
| `mcp.json` | `d8e397af03b5b032f21d0aa967086f0c78b33c87b76f2e9898ae0a144df7de02` |
| `env.json` | `a6115969e666abfaff5ea366eceddc09b68cb0a347143bb75cb18432533ddc44` |
| `instructions.md` | `be1b6e8f168acaa52ad95aca38bc8defa1e378d0fe90ad496499b8beb9bc5d6e` |

`env.json` parses as `{"env": {"LYS_PROOF_MODE": …, "LYS_PROOF_TOKEN": "handle-proof-0001"}}`: the handle, never the value.

## The launch

The report's launch line, with the out directory written as `<out>`:

```
claude --resume <out>/9c1e4a7b-3d5f-4a2c-8e6b-1f0d2c4a6e8b.jsonl --fork-session --mcp-config <out>/mcp.json --settings <out>/env.json --append-system-prompt-file <out>/instructions.md --strict-mcp-config --allowedTools 'Bash(printenv:*)'
```

Run once, by hand and never by the tool, from `<scratch>/proof-real/elsewhere`
(neither `<out>` nor the session's cwd), in print mode so it is
non-interactive: the line above followed by `-p --max-turns 2 --output-format json`
and one short question asking the session to run `printenv LYS_PROOF_TOKEN`
with the Bash tool and then reply on one line with the number of messages it
had said were carried over (the question PROOF-RESUME.md asked) and the value
printed. Two turns because the third question below needs the tool call and
the answer after it; with one turn the run ends on the call. `LYS_PROOF_TOKEN`
was set in the shell's environment to the same value as at the render.

| field | value |
| --- | --- |
| exit | 0, in 15 s (13:11:24 to 13:11:39); `num_turns` 2, `subtype` success, `is_error` false, `permission_denials` empty, `api_error_status` null |
| stderr | one warning that no stdin data arrived in 3 s (print mode with no pipe); nothing about MCP, settings or the prompt file |
| rendered file before | `46b6b9e741f05dc089848de20562e9b6cf2a7d49fb355a707b3945e38bf97879` |
| rendered file after | `46b6b9e741f05dc089848de20562e9b6cf2a7d49fb355a707b3945e38bf97879` (equal) |
| `<out>` after | the same five files, no sixth: nothing was written beside the rendered file |
| continuation | `~/.claude/projects/-private-tmp-claude-501--Users-tom-Developer-archie-34c4f280-dc08-4edc-9fe0-82f1f75bee3b-scratchpad-proof-real-elsewhere/1a83f02d-7e36-4005-98a9-b0c4daa911e9.jsonl`: the run directory's slug directory (the projects listing gained exactly that directory), under the uuid Claude Code assigned at the fork, not the rendered file's path; sha256 `5e27c5320036e25d038fd707052912f8e7cd89f4c32dab2bb1b0ed3ebc6aeb73`; read, hashed, left alone |
| continuation lines | 98: 64 records copied from the rendered file (same uuids: every user and text record, none of the 17 thinking records, as PROOF-RESUME.md measured on 2.1.281), the new turn (2 user, 2 assistant, 1 tool_use and 1 tool_result), and Claude Code's own records (17 attachment, 3 mode, 3 atis-latch, 3 last-prompt, 2 queue-operation, 1 system, 1 cost-state) |
| assistant models | `claude-opus-5` on every copied and every new assistant record |
| answer | redacted: it held the same number PROOF-RESUME.md recorded as the session's last answer (yes), the handle (yes), the instruction token (yes), the secret value (no); 85 characters |

`lys-home resume-check <out>/9c1e4a7b-….jsonl <continuation>`:

| field | value |
| --- | --- |
| rendered_records | 81 |
| forked_records | 98 |
| forked_new_records | 34 |
| repeated_tool_use_ids | 0 |
| new_tool_uses | 1 (the Bash call the question asked for) |
| exit | 0 |

## The four questions

| question | answer | observation |
| --- | --- | --- |
| Does the continuation's parent link name the rendered session id? | no | The rendered session id `9c1e4a7b-3d5f-4a2c-8e6b-1f0d2c4a6e8b` appears in no field of any of the 98 records: every record, the 64 copied ones included, carries `sessionId` `1a83f02d-7e36-4005-98a9-b0c4daa911e9`, and no record names a parent session. On 2.1.283 the fork is tied to its source by the copied records' uuids (64 of the 81 rendered uuids present), not by a link to the rendered id; a capture that wants the rendered id takes it from the render's manifest block and matches uuids, not from the fork. |
| Did the appended instructions take effect? | yes | The reply ends with the exact token `instructions.md` asks for. The template does not set `--system-prompt-snapshot`, so Claude Code's default `on` applied: the continuation holds 2 `attachment` records of type `prompt_snapshot` and 1 of type `environment`, so the prompt with the appended file was rendered on this fork's first request and recorded there. |
| Did a tool call in the launched session see the environment file's handle? | yes | The one new `tool_result` (of the one `Bash` `tool_use`) holds `handle-proof-0001`, and the reply repeats it. The secret value set in the shell's environment appears nowhere in the continuation, the reply or the five files (0 matches): `--settings` `env` put the handle in the tool's environment in place of the shell's value. |
| Was `--mcp-config` accepted? | yes | Exit 0 with `--mcp-config <out>/mcp.json --strict-mcp-config`; stderr and the JSON result say nothing about MCP; the result's `subtype` is `success`. |

## What this proves and what it does not

- On 2.1.283 the rendered launch line resumes the rendered file by path from an unrelated directory, forks, and writes its continuation under `~/.claude/projects/<slug of the run directory>/<fork uuid>.jsonl`; the rendered file and the source file are unchanged and nothing is written beside either, which is why the launch line carries `--fork-session` (PROOF-FEWSHOT.md measured the bare resume writing beside the passed file).
- The appended instructions, the settings file's environment and the MCP configuration each took effect through the flags the launch line carries, and the session answered from the rendered history without repeating a tool action.
- It does not bear out the premise that the fork's parent link names the rendered session id: on 2.1.283 it does not, and the first answer above records that for the capture card to build on.
- As on 2.1.281, the 17 signed thinking records were not carried into the continuation; whether they were sent to the model on the first turn is not visible from the files (PROOF-CANON.md leaves it open).
