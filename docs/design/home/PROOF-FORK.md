# PROOF-FORK: a real session forked from a lantern's point, rendered and launched (HOME-006 R9)

Measured 26 September 2026, 17:31 AEST, on this Mac, in one run end to end
(`<scratch>/run.sh`), with the lys-home binary built from
card/home-006-forks at 465ebe9 (the design commit, R1 to R9 and the three
review commits, on the line rebased onto main d30fd6d), node v26.4.0, and
the Pi checkout at `3d5cbe98` under `$PI`: the scratch clone
PROOF-LANTERN.md built (`git rev-parse HEAD` answers
`3d5cbe98c3bc67ef8433bdeee45fbe5f0d8a24db`, `git status --short` empty,
`packages/coding-agent/dist` built by its own tsgo). Every figure here is a
hash, a count, a path, a command, a version or an exit code; the scratch
home is not committed.

## How Pi reads `parentSession` at 3d5cbe98

`packages/coding-agent/src/core/session-manager.ts`, read before the
header value was chosen:

| line | what it says |
| --- | --- |
| 36 | `parentSession?: string;` on `SessionHeader` |
| 41 | `parentSession?: string;` on `NewSessionOptions` |
| 175 to 176 | `/** Path to the parent session (if this session was forked). */` `parentSessionPath?: string;` on the session listing's info |
| 598 | `const parentSessionPath = (header as SessionHeader).parentSession;` when a file is listed |
| 607 | `parentSessionPath,` carried into the listing |
| 739 | `parentSession: options?.parentSession,` written into a new session's header |
| 1191 | `parentSession: this.persist ? previousSessionFile : undefined,` on Pi's own fork: the previous session's file |
| 1345 | `parentSession: sourcePath,` on Pi's copy of a session into another directory: the source file's path |

Pi reads and writes the field as a session file path, nothing else, so the
child's header carries the parent file's path, written relative to the
home root (`sessions/<parent id>.jsonl`) so the home can move.

## The session, imported

The session PROOF-RESUME.md and PROOF-LAUNCH.md measured, named here by
its hash only: the source whose SHA-256 is
`793f4e87ea4608b2ddfb9d5dcbf1197d4245bde7ed1612630339a1cc1f6d15df`, under
`~/.claude/projects/<slug dir of the session cwd>/`, not running at the
time (`pgrep` found no process on it). Read, hashed and never written.

| field | value |
| --- | --- |
| Claude Code version | `2.1.283 (Claude Code)` (`claude --version`, the run's own output in `<scratch>/claude-version.txt`) |
| source size | 562,932 bytes, 242 records |
| source hash before | `793f4e87ea4608b2ddfb9d5dcbf1197d4245bde7ed1612630339a1cc1f6d15df` |
| source hash after | `793f4e87ea4608b2ddfb9d5dcbf1197d4245bde7ed1612630339a1cc1f6d15df` (equal, checked after both launches ran) |
| import | `lys-home import --home <scratch>/home --claude-code <source> --session real1`: exit 0; 205 entries (81 messages, 42 attachment, 32 hook, 16 system, 10 permission_mode, 24 tool_completed events), 81 blocks all new, bytes in / out 562,932 / 219,884, as the two earlier proofs recorded |

## Two lanterns lit

`lys-home lantern light --home <scratch>/home --session real1 --point <entry> --note <note> --by proof-lighter`, twice, exit 0 each; the notes are not carried here (ADR-015 lets only recall print them).

| lantern | point | kind of point | lit_at |
| --- | --- | --- | --- |
| `c4cd8a075eb19455585814a3e29f0a59` (A) | `7389b620-3244-447a-816a-36e831893aa6` | an assistant message, the session's 223rd record | `2026-09-26T07:31:28.96Z` |
| `01d4b17735b59fe99b6b85a8744e4646` (B) | `35b5364e-7496-46db-aa3c-b863416b5237` | a user message, the session's 78th record | `2026-09-26T07:31:28.997Z` |

The light act on this tree records no `lit_in`, so both resolve by the
older-record rule: one holder, `real1`, cuts.

## Fork A: the assistant point

`lys-home fork --home <scratch>/home --lantern c4cd8a075eb19455585814a3e29f0a59`, exit 0:

```
{"command":"fork","report":{"blocks":74,"carried":null,"child":"72dca69599258c2e3227a02aacb89812","coordinate_carried":false,"cut_at":"7389b620-3244-447a-816a-36e831893aa6","entries":139,"lantern":"c4cd8a075eb19455585814a3e29f0a59","parent":"real1","point":"7389b620-3244-447a-816a-36e831893aa6","unstored":66}}
```

| field | value |
| --- | --- |
| parent length before, N | 220,664 bytes |
| parent whole-file hash before | `b360220f931cfaf089efa6ece7b5198b655f15e19b045d3990b44e5f28a07a9d` |
| parent prefix hash after (bytes 0..N) | `b360220f931cfaf089efa6ece7b5198b655f15e19b045d3990b44e5f28a07a9d` (equal) |
| block store before | 172 files, 413,162 bytes |
| block store after | 172 files, 413,162 bytes (equal) |
| the cut | 139 entries of the 205: the chain from the root to the point, which is itself the last assistant message, so `cut_at` equals `point`; 79 messages and 60 harness events on the chain; the 66 entries after the point and the side leaves left out |
| `blocks` 74, `unstored` 66 | 74 distinct hashes named by the copied entries that are files under `blocks/` (text parts and event records); 66 that are not: the inline thinking, tool-call and tool-result parts, whose Claude Code source forms are the stored blocks |

Pi's loader on the child, `PI=<scratch>/proof-lantern/pi node <scratch>/pi-load-fork.mjs <scratch>/home/sessions/72dca69599258c2e3227a02aacb89812.jsonl` (the script imports `loadEntriesFromFile` from `$PI/packages/coding-agent/dist/core/session-manager.js` and prints counts only), exit 0:

```
{"header_type":"session","header_id":"72dca69599258c2e3227a02aacb89812","header_version":2,"header_parent_session":"sessions/real1.jsonl","entries":140,"entries_with_id_parent_timestamp":140,"message":79,"custom":61,"forked_from":1,"fork":0,"lantern":0}
```

140 entries: the fork's `entries` 139 plus the one `lys.forked_from`.

### Rendered through the launch template

`lys-home render-launch --home <scratch>/home --session 72dca69599258c2e3227a02aacb89812 --template <scratch>/template.json --uuid 6f0c2b7e-4a1d-4c3e-8b5f-1e2d3c4b5a60 --cwd <session cwd> --model claude-opus-5 --version 2.1.283 --out <scratch>/out-a`, exit 0, with `LYS_PROOF_TOKEN` set in its environment to a value that appears in none of the written files, the report or either continuation (0 matches by byte search in each).

| field | value |
| --- | --- |
| template | `<scratch>/template.json`, hash `00159b76e9188d541596fe3a9cd98886b5aaf2f55bfcac15783bdb2fe767cfbb`: `harness` `claude-code`; flags `--strict-mcp-config --allowedTools 'Bash(printenv:*)'`; `transcript` fill `resume-by-path`, canon null; `mcp` `{"mcpServers": {}}`; `env` one variable `LYS_PROOF_MODE`; `secrets.use_only` one entry, variable `LYS_PROOF_TOKEN`, handle `handle-proof-0006`; `readable` empty; `instructions` one line asking for the answer and nothing else |
| render | `records` 79 (the child's 79 messages), `thinking_kept` 17, `thinking_as_text` 0, `dropped` 0, `seed` null; the render report carries no launch line |
| session head | `c0c3da7f08ec6b676bd4ab700328184afa9ce1a5c1cc556dd5b930503bc355b0`; event `2df0911f6e20e0672f8d6ea38aa56159`; manifest `046de2dbdbd5550bd40a228daef2b8308233718fd180364247b08c92d3c2667f`; given `f431df95edc5763123eb140de00d47f0`, 3 documents |
| files, in manifest order | `6f0c2b7e-….jsonl` `ec27f1854febfe19a80326e63eb0bba294252b060e4f4e89ece9ed42dceea81f`; `6f0c2b7e-….loss.json` `6f628ca5c4a74974034a64fb0d28e6f95bb2bf27127d277e89ce3abce20e9664`; `mcp.json` `d8e397af03b5b032f21d0aa967086f0c78b33c87b76f2e9898ae0a144df7de02`; `env.json` `ec1d4d943d026a0d0c66ae59ed80b8d1b70b11fa9533671a9c2086132b5078bb`; `instructions.md` `a5cc916f3f258b64e56bab747c6b9c9b147e7cb0bb3ea07e0361f49723296966` (five files, no seed) |

### The launch line, run

The line run is the launch template's line from the `render-launch`
report, the one launch line the tool prints, by hand and never by the
tool, from `<scratch>/elsewhere` (neither `<scratch>/out-a` nor the
session's cwd), in print mode with one question appended. The question
(redacted, as every message of the session is) has a one-word answer that
the parent's chain holds only at or before the point, in records 121 to
223 of the source and in none after; `--max-turns 1` since no tool is
needed.

```
claude --resume <scratch>/out-a/6f0c2b7e-4a1d-4c3e-8b5f-1e2d3c4b5a60.jsonl --fork-session --mcp-config <scratch>/out-a/mcp.json --settings <scratch>/out-a/env.json --append-system-prompt-file <scratch>/out-a/instructions.md --strict-mcp-config --allowedTools 'Bash(printenv:*)' -p --max-turns 1 --output-format json '<question>'
```

| field | value |
| --- | --- |
| exit | 0, in 3,556 ms (`duration_api_ms` 3,052); `subtype` success, `is_error` false, `num_turns` 1, `permission_denials` empty, `stop_reason` end_turn, `terminal_reason` completed |
| stderr | empty |
| rendered file before | `ec27f1854febfe19a80326e63eb0bba294252b060e4f4e89ece9ed42dceea81f` |
| rendered file after | `ec27f1854febfe19a80326e63eb0bba294252b060e4f4e89ece9ed42dceea81f` (equal) |
| `<scratch>/out-a` after | the same five files, no sixth |
| continuation | `~/.claude/projects/<slug dir>/3bf71d5c-c8e4-451c-902d-be8abcde1d87.jsonl`, `<slug dir>` being the run directory's slug (the projects listing gained that file and a `memory/` directory of Claude Code's own); sha256 `17ee075f31defd995338aa86b2f2b936e65d8f2d993a72b15425fc4c5e79eb22`; read, hashed, left alone |
| continuation lines | 92: 62 records copied from the rendered file (same uuids: every user and text record, none of the 17 thinking records, as PROOF-LAUNCH.md measured on 2.1.283), the new turn (1 user, 1 assistant), and Claude Code's own records (15 attachment, 3 mode, 3 atis-latch, 3 last-prompt, 2 queue-operation, 1 system, 1 cost-state); every record carries the fork's own `sessionId`, and the rendered uuid appears in no record |
| answer | the reply's SHA-256, trimmed of surrounding whitespace, stripped of one trailing full stop and lowercased, is `c78961d3d782d8a85d9344eedae027f43ce6b9fd35c8f355861a39e0d0ddecc5` |
| expected | the expected word normalised the same way: `c78961d3d782d8a85d9344eedae027f43ce6b9fd35c8f355861a39e0d0ddecc5` (equal) |

`lys-home resume-check <scratch>/out-a/6f0c2b7e-….jsonl <continuation>`, exit 0:

```
{"command":"resume-check","report":{"forked_new_records":30,"forked_records":92,"new_tool_uses":0,"rendered_records":79,"repeated_tool_use_ids":0}}
```

## Fork B: the user point, launched with its seed

Taken after fork A, so the parent now holds A's `lys.fork` line at its head.

`lys-home fork --home <scratch>/home --lantern 01d4b17735b59fe99b6b85a8744e4646`, exit 0:

```
{"command":"fork","report":{"blocks":32,"carried":"35b5364e-7496-46db-aa3c-b863416b5237","child":"337541de7a39bd77ed4d773fbd8bac43","coordinate_carried":true,"cut_at":"d2594984-87f5-4abe-93e8-7dccaf9283fa","entries":46,"lantern":"01d4b17735b59fe99b6b85a8744e4646","parent":"real1","point":"35b5364e-7496-46db-aa3c-b863416b5237","unstored":15}}
```

| field | value |
| --- | --- |
| parent length before, N | 220,883 bytes (220,664 plus A's one `lys.fork` line) |
| parent whole-file hash before | `f3054b1eef5e7ac60cdaaf51e1b68ae3110d9087a9326ab7c0d5cbe3b27b5bb8` |
| parent prefix hash after (bytes 0..N) | `f3054b1eef5e7ac60cdaaf51e1b68ae3110d9087a9326ab7c0d5cbe3b27b5bb8` (equal) |
| parent head after both forks | `0e3c8ea3562eb8fed5661b621df33e5b`, B's `lys.fork` entry (`<scratch>/home/sessions/real1.head`) |
| block store before | 173 files, 414,575 bytes (172 plus the manifest block of A's render) |
| block store after | 173 files, 414,575 bytes (equal) |
| the cut | 46 entries: the chain to `cut_at`, the assistant message before the point (the source's 73rd record); the point is carried (`coordinate_carried` true, `carried` its id) and not copied |

Pi's loader on the child, exit 0: `{"header_type":"session","header_id":"337541de7a39bd77ed4d773fbd8bac43","header_version":2,"header_parent_session":"sessions/real1.jsonl","entries":47,"entries_with_id_parent_timestamp":47,"message":20,"custom":27,"forked_from":1,"fork":0,"lantern":0}`: 47, the fork's 46 plus one.

`lys-home render-launch … --session 337541de7a39bd77ed4d773fbd8bac43 --uuid 6f0c2b7e-4a1d-4c3e-8b5f-1e2d3c4b5a61 … --out <scratch>/out-b`, exit 0:

| field | value |
| --- | --- |
| render | `records` 20, `thinking_kept` 2, `thinking_as_text` 0, `dropped` 0; `seed` `<scratch>/out-b/6f0c2b7e-….seed.txt`, 4 lines, 399 bytes, sha256 `edd93209910a95f9a0e1d447057c95f4fc10cdc8aed852a0bc97f3f1ff65ca22`: its first line is `<FORKED FROM SESSION real1 AT ENTRY 35b5364e-7496-46db-aa3c-b863416b5237 BY LANTERN 01d4b17735b59fe99b6b85a8744e4646>`, its second the heading, then the carried message's text, which is not carried here; the render report carries no launch line |
| session head | `038fe26d84c1e8381de304c6541fed481310f38af926132b3471237464974191`; event `349f8fbdc2d62c1008e5b60a64436501`; manifest `7088d1a8ccea87b81110bc9fb40db1a6110a9abca593074eb4257ea166d6d94e`; given `0dbf2900fd5cd68c72c3a04f1085802f`, 3 documents |
| files, in manifest order | `6f0c2b7e-…61.jsonl` `ff0372d15d4c8eb6c688304b7c0c37ba87bd030405b985c37b1de3a12db6f495`; `6f0c2b7e-…61.loss.json` `bae2a4b7772a2e29d9566f3ac0e3bc2849a0fd643b64c11c1432a83de595f889`; `mcp.json` `d8e397af…`; `env.json` `ec1d4d94…` (the same bytes as A's); `instructions.md` `a5cc916f…`; `6f0c2b7e-…61.seed.txt` `edd93209…` (six files, the seed last) |

The launch line the report printed, the template's line with the seed as the first prompt:

```
claude --resume <scratch>/out-b/6f0c2b7e-4a1d-4c3e-8b5f-1e2d3c4b5a61.jsonl --fork-session --mcp-config <scratch>/out-b/mcp.json --settings <scratch>/out-b/env.json --append-system-prompt-file <scratch>/out-b/instructions.md --strict-mcp-config --allowedTools 'Bash(printenv:*)' "$(cat '<scratch>/out-b/6f0c2b7e-4a1d-4c3e-8b5f-1e2d3c4b5a61.seed.txt')"
```

Run by hand from `<scratch>/elsewhere` with `-p --max-turns 4 --output-format json` inserted before the seed argument, nothing else changed; four turns are allowed because the seed is a turn of the parent's conversation and not a question, so a tool call before the reply has room.

| field | value |
| --- | --- |
| exit | 0, in 6,965 ms (`duration_api_ms` 6,519); `subtype` success, `is_error` false, `num_turns` 2, `permission_denials` empty, `stop_reason` end_turn, `terminal_reason` completed |
| stderr | empty |
| rendered file before / after | `ff0372d15d4c8eb6c688304b7c0c37ba87bd030405b985c37b1de3a12db6f495` both (equal) |
| continuation | `~/.claude/projects/<slug dir>/139da1dc-f37c-409e-bab7-9151a4e99f25.jsonl`, sha256 `2ceb3b7181fb581193708a5dbf2df357497fb04ba873962e23f5eefb32ab7d61`; 53 lines: 18 copied from the rendered file (of 20; the 2 thinking records not carried), the new turns (2 user, 3 assistant, holding one `tool_use` and its `tool_result`; no server is configured under `--strict-mcp-config` with an empty `mcp.json`), and 30 of Claude Code's own records (17 attachment, 3 mode, 3 atis-latch, 3 last-prompt, 2 queue-operation, 1 system, 1 cost-state); every record carries the fork's own `sessionId`, and the rendered uuid appears in no record |
| the seed as the first prompt | the first new `user` record's content is byte for byte the seed file (the marker line, the heading and the carried text), so the carried message reached the child as its first prompt |
| resume-check | `{"command":"resume-check","report":{"forked_new_records":35,"forked_records":53,"new_tool_uses":1,"rendered_records":20,"repeated_tool_use_ids":0}}`, exit 0 |

## The leak check

`python3 <scratch>/leak-check.py <source> docs/design/home/PROOF-FORK.md '<question>' '<expected word>'`: every text part of the imported session's messages (a `content` string, or each `text` part) is trimmed of surrounding whitespace; every part that is empty after trimming or holds no letter and no digit is left out; each remaining part is compared whole, with no length bound, against each line of this file trimmed of surrounding whitespace; the question's text and the expected word are searched for as well.

| field | value |
| --- | --- |
| text parts | 16 |
| compared | 16 |
| left out (empty, or no letter and no digit) | 0 |
| parts equal to a line of this file | 0 |
| occurrences of the question's text | 0 |
| occurrences of the expected word as a whole whitespace-separated token | 0 |

## The scripts the proof ran

`<scratch>/pi-load-fork.mjs`:

```js
// Load session files with Pi's own loader at 3d5cbe98 (the checkout under $PI, built with its own tsgo), printing counts only: never an entry's content.
import { readFileSync } from "node:fs";
const pi = process.env.PI;
if (!pi) { console.error("PI is not set: the Pi checkout at 3d5cbe98"); process.exit(2); }
const { loadEntriesFromFile } = await import(`${pi}/packages/coding-agent/dist/core/session-manager.js`);
for (const file of process.argv.slice(2)) {
  const entries = loadEntriesFromFile(file);
  const header = entries[0];
  const rest = entries.slice(1);
  const custom = rest.filter((e) => e.type === "custom");
  console.log(JSON.stringify({
    header_type: header?.type, header_id: header?.id, header_version: header?.version,
    header_parent_session: header?.parentSession,
    entries: rest.length,
    entries_with_id_parent_timestamp: rest.filter((e) => "id" in e && "parentId" in e && "timestamp" in e).length,
    message: rest.filter((e) => e.type === "message").length,
    custom: custom.length,
    forked_from: custom.filter((e) => e.customType === "lys.forked_from").length,
    fork: custom.filter((e) => e.customType === "lys.fork").length,
    lantern: custom.filter((e) => e.customType === "lys.lantern").length,
  }));
}
```

`<scratch>/leak-check.py`:

```python
#!/usr/bin/env python3
"""Compare every text part of the imported session's messages with every line of PROOF-FORK.md, printing counts only."""
import json, sys
source, proof, question, word = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
parts = []
for line in open(source):
    line = line.strip()
    if not line:
        continue
    r = json.loads(line)
    if r.get("type") not in ("user", "assistant"):
        continue
    c = r.get("message", {}).get("content")
    if isinstance(c, str):
        parts.append(c)
    elif isinstance(c, list):
        for p in c:
            if p.get("type") == "text":
                parts.append(p.get("text", ""))
compared, left_out, equal = 0, 0, 0
proof_text = open(proof).read()
lines = [l.strip() for l in proof_text.splitlines()]
for part in parts:
    t = part.strip()
    if not t or not any(ch.isalnum() for ch in t):
        left_out += 1
        continue
    compared += 1
    equal += sum(1 for l in lines if l == t)
tokens = proof_text.split()
print(json.dumps({
    "text_parts": len(parts), "compared": compared, "left_out": left_out, "equal_lines": equal,
    "question_occurrences": proof_text.count(question),
    "word_whole_token_occurrences": sum(1 for t in tokens if t == word),
}))
```

`<scratch>/normalise-hash.py`, fed the run's `result` field, or the expected word, on stdin:

```python
#!/usr/bin/env python3
"""SHA-256 of a reply or an expected word, trimmed, one trailing full stop stripped, lowercased."""
import hashlib, sys
text = sys.stdin.read().strip()
if text.endswith("."):
    text = text[:-1]
print(hashlib.sha256(text.lower().encode()).hexdigest(), len(text))
```

## Lines of code per file (the brief's verification)

Counted with `grep -vcE '^\s*$|^\s*//' <file>` on the tree at 465ebe9.
Every non-test file is at most 500; the one over it,
`src/record/call_tests.rs` at 519, is a test file the repository's rule
excludes, held by main before this card.

```
$ cd crates/lys-home && for f in $(find src -name '*.rs' | sort); do printf "%s %s\n" "$(grep -vcE '^\s*$|^\s*//' $f)" "$f"; done | sort -rn | head -8
519 src/record/call_tests.rs
473 src/harness/claude_code/import.rs
472 src/record/mod.rs
460 src/record/call.rs
451 src/harness/claude_code/render_tests.rs
433 src/cli.rs
407 src/record/fork_cut_tests.rs
407 src/harness/claude_code/given_tests.rs
```

The files this card adds: `src/record/fork.rs` 205, `src/record/fork_cut.rs`
186, `src/harness/claude_code/seed.rs` 128, `src/record/fork_report.rs` 102,
`src/cli_fork.rs` 24; `src/cli.rs` 433 after its variant and dispatch.

## What this proves and what it does not

- On 2.1.283 a child cut from a real session at a lantern's point loads
  with Pi's own loader, renders through the launch template, and the
  template's line resumes it from an unrelated directory and answers, in
  one turn without a tool, from what the parent said at or before the
  point; the parent's earlier bytes, the source file, the rendered file and
  the block store are unchanged, and the parent gained exactly one line per
  fork.
- A child forked at a user message carries that message as its first
  prompt through the seed file the render wrote, and the seeded launch
  exits 0 with the seed byte for byte as the fork's first user record
  (`new_tool_uses` 1, `repeated_tool_use_ids` 0, four turns allowed).
- The line run was the launch template's, the one launch line the tool
  prints, so the child ran with the template's flags, settings and handle,
  not the bare seat login.
- It does not measure whether the copied thinking blocks reached the model:
  as in the two earlier proofs, the 17 and 2 thinking records were not
  carried into the continuations. It does not exercise `lit_in`, since the
  light act on this tree records none; the fixture gates exercise it on a
  hand-written lantern.
