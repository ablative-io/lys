# Permissions facts, Waffles' own table

Started 3 October 2026, 20:25 AEST. Written independently of Archie's and Vesper's tables, to be reconciled
with them. A row is **agreed** only when the program's documentation, Lys's code and a real observation all
say the same thing. Anything less is **open**, and says what is missing. Nothing here is from memory.

Sources: D = the fetched pages in `docs/harness/reference/claude-code/` (page and line of the fetched copy).
L = Lys code at a path and line, at commit f34dece3. R = a real observation, named.
R1 = the settings file tonight's run of pancake was handed:
`~/Library/Application Support/lys/identity/data/runner/sessions/op-e85561b1c5386faffafe8a2b5a1d21cb/config/settings.json`
(keys: `env`, `permissions`; beside it `instructions.txt`, `mcp.json`).

## Claude Code

| # | Claim | D | L | R | State |
|---|---|---|---|---|---|
| 1 | Lys stores per agent: allow, deny, ask lists, extra folders, one mode | n/a | `lys-identity-server/src/launch_permissions.rs:20-35` | R1 has exactly these five keys under `permissions` | agreed |
| 2 | The screen's words for a mode are the catalogue's `meaning`, not Claude Code's words | permission-modes 17-24: `default` = "Reads only" run without asking | `docs/harness/catalogue/claude-code.json` modes; `harness_catalogue.rs:129-134` | Tom saw "Reads freely and asks before most changes and commands" on the installed screen, 20:09 | agreed |
| 3 | What R1's run was handed: mode `default`; allow, ask, extra folders empty; deny `Read, Write, Edit, Glob, Grep, WebFetch, Read(//), Read(///**)`; no `sandbox`; no `hooks` | n/a | n/a | R1 | observed; which policy or setting produced the deny list is **open** |
| 4 | A whole-tool deny removes the tool from the model's view; so R1's run had no Read, Write, Edit, Glob, Grep, WebFetch at all | permissions 66 | hard rule of kind tool renders as the bare tool name, `rendering_permissions.rs:44` | not observed in the run itself (nobody looked at which tools the model was offered) | **open**: needs the run's first model call from the proxy record |
| 5 | Order is deny, then ask, then allow; first match; a deny at any settings level beats an allow at any level | permissions 62-64, 671-673 | Lys only appends to deny, never removes: `rendering_permissions.rs:78-86` | none | **open**: no real observation |
| 6 | Rules on shell commands match the command's text and "aren't a security boundary" (`/bin/rm`, `sh -c` pass a `Bash(rm *)` deny) | permissions 251-261 | the judge refuses to treat shell text as authority: `lys-runner/src/judge.rs:21-24` | none | **open**: no real observation |
| 7 | Read and Edit deny rules do not stop a script the agent runs from opening the file; only the sandbox does | permissions 346 | n/a | none | **open** |
| 8 | The sandbox covers shell commands and their children only; Read, Edit, Write, WebFetch, hooks and local MCP servers run outside it | sandboxing 38-44 | n/a | none | **open** |
| 9 | In a file passed with `--settings`, a path rule with one leading slash is relative to that file's folder; two slashes is the disk root | permissions 353-369 | Lys passes `--settings settings.json` (`rendering_launch.rs:102`); a hard path rule is written `Tool(/{path})` (`rendering_permissions.rs:52`) | R1 has `Read(//)` and `Read(///**)`, which is that format with path `/` | agreed for hard rules whose path starts with `/`; a person-typed rule's form is **open** (nothing checks it) |
| 10 | Lys's `workspace-only` is not a Claude Code mode. Lys writes it as mode `dontAsk`, allow `Read(./**)`, `Edit(./**)`, deny `WebFetch`, `WebSearch`, reads outside working folders blocked, sandbox on, fail if unavailable, no unsandboxed commands, no network | permission-modes 17-24 lists six modes, none named workspace-only | `lys-home/src/harness/claude_code/launch_env.rs:57-108` | none (R1 was mode `default`) | **open**: no real run in this mode observed |
| 11 | Lys has a tool-boundary judge: a PreToolUse hook that asks the runner before each tool call; denies only, never allows; hard rules always deny; grantable rules are lifted only by a live grant | hooks run before the permission prompt and a deny holds: permissions 558-560 | `lys-runner/src/judge.rs:1-26`, `claude_judge.rs:1-10`, hook built at `lys-home/src/harness/claude_code/launch_env.rs:113-131` | R1 has no `hooks` key: the judge was **not installed** in tonight's run | code exists; whether any start path installs it is **open** (I found no caller passing `--judge-socket` outside `claude_code/launch.rs:190`) |
| 12 | The person's own Claude Code settings on the computer (user and project settings, hooks, plugins) also apply to a Lys run, because Lys does not replace them | permissions 667-673; settings page (not yet read whole by me) | Lys does not set `CLAUDE_CONFIG_DIR` (Tom's ruling, 3 Oct) | none | **open**: needs a real observation |
| 13 | A mod that handles `tool.check` can approve a call a deny rule refuses, unless managed settings or a Team or Enterprise plan | permissions 562-567 | n/a | none | **open** |
| 14 | `bypassPermissions`: nothing prompts except ask rules, tools needing a person, and removals of critical paths; deny rules still block; allow rules have no effect | permission-modes 26, 31-38; 563-565 | catalogue offers it as a mode | none | **open** |
| 15 | The profile's `tools` are added to the allow list at start | n/a | `launch_permissions.rs:136-139` | R1 allow is empty (pancake's profile names no tools; not checked) | **open** |

## Codex

Not read by me yet. The catalogue says three modes (`read-only`, `workspace-write`, `danger-full-access`) and
no rule forms (`docs/harness/catalogue/codex.json`). Lys renders a permissions profile named `lys-bound` from
a containment plan (`lys-runner/src/codex_policy.rs:1-16`). Every row is **open**.

## Not yet read by me

permission-modes 47-546 and 660 to the end; sandboxing 53 to the end; settings; iam; Lys's policy records
(the rules with hard and grantable authority) and `containment_policy.rs`; all of Codex.
