# PROOF-FEWSHOT: a hand-authored file resumed by path (HOME-001 R5, R9)

Measured 24 September 2026, 14:38 to 14:39 AEST, on this Mac.

| field | value |
| --- | --- |
| Claude Code version | `2.1.281 (Claude Code)` (`claude --version`) |
| lys-home | the worktree build of card/home-001 after dda6daa9 (the R9 command line) |
| turns file | 6 lines: 3 `user:` and 3 `assistant:` turns; the last exchange sets a second codeword |
| authored file | `<scratch>/proof5/fewshot/authored.jsonl`, 6 records, sessionId `60d9c850-fa7a-458e-82f1-dd060ac51bca`, cwd `/authored` |
| written by | `lys-home fewshot --out <that path> --turns turns.txt --cwd /authored` |
| fewshot report | `{"authored":true,"command":"fewshot","file":"<that path>","records":6}`, and the line `claude --resume <that path>` (now the `resume` member of the report) |
| parentUuid chain | intact: each record's parentUuid is the previous record's uuid, the first is null |
| assistant models | `authored`, `authored`, `authored` |
| run from | `<scratch>/proof5/elsewhere`, a directory that is neither the file's directory nor under `~/.claude` |
| command | `claude -p --resume <scratch>/proof5/fewshot/authored.jsonl --strict-mcp-config --mcp-config '{"mcpServers":{}}' --max-turns 1 "What is the second codeword?"` |
| answer | `basalt` (one word, the value only the file's last exchange holds) |
| exit | 0, in 7 s |
| source hash before | `4ef6c70e27f76e6a70b4743351606fb70993dcb762a3fd686cb90bbcd0636c47` |
| source hash after | `4ef6c70e27f76e6a70b4743351606fb70993dcb762a3fd686cb90bbcd0636c47` (equal: the passed file is unchanged) |
| continuation | `<scratch>/proof5/fewshot/60d9c850-fa7a-458e-82f1-dd060ac51bca.jsonl`, beside the source, named `<sessionId>.jsonl` |
| continuation lines | 34: the 6 authored records copied in (same uuids), then the new turn (1 user, 1 assistant on `claude-opus-5-5`), plus Claude Code's own records (13 attachment, 3 atis-latch, 3 last-prompt, 2 queue-operation, 2 system, 2 mode, 1 cost-state) |
| authored boundary | the 3 copied assistant records still carry model `authored`; the new assistant record carries `claude-opus-5-5` |

`<scratch>` is this session's scratchpad directory; the proof holds no transcript text beyond the codewords the turns file was written with.

## Which marker 2.1.281 accepts

R4 and R9 allow two markers for an authored file: a first record of type `custom` with `customType` `lys.authored`, or the model value `authored` on every assistant record. Both were run.

| variant | file | answer | continuation |
| --- | --- | --- | --- |
| model `authored` only (what `fewshot` writes) | `fewshot/authored.jsonl`, hash `4ef6c70e…` | `basalt`, exit 0 | 34 lines, models as above |
| a first `custom` record `lys.authored` before the same 6 records | `marker/authored-marked.jsonl`, hash `c86cf07b954346039f3a43b3371746780d71c03210eb85f6aeafc1f006cdae11` (7 lines), sessionId `7a1c2e3f-4b5d-4e6f-8a9b-0c1d2e3f4a5b` | `heron` to "What is the first codeword?", exit 0 | 34 lines; the `custom` record is **not** copied into the continuation (no record of type `custom` in it; its first record is `atis-latch`) |

So 2.1.281 tolerates a leading `custom` record (it resumes and answers) but does not carry it forward. The marker that survives the resume is the model value `authored` on the assistant records, which the continuation keeps byte for byte. `fewshot` therefore writes only that marker, and the importer (R3) recognises an authored turn by it. The source hash of the marked variant was also unchanged after the run.

## What this proves and what it does not

- Claude Code 2.1.281 resumes from a file path given to `--resume`, from a working directory unrelated to the file, and writes its continuation beside the file. Nothing under `~/.claude/projects` was written for this cwd.
- A hand-authored exchange is read as history: the answer came from the file's last exchange and from nowhere else.
- The authored records are never mistaken for a model's: they keep model `authored` in the continuation, and R3 imports them behind one `lys.authored` entry with provider, api and model `authored`.
- It does not prove anything about tools: the authored file holds no tool actions, so the repeated-tool-action question belongs to PROOF-RESUME.md.
