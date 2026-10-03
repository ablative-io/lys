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
| 3 | What R1's run was handed: mode `default`; allow, ask, extra folders empty; deny `Read, Write, Edit, Glob, Grep, WebFetch, Read(//), Read(///**)`; no `sandbox`; no `hooks` | n/a | n/a | R1 | **agreed** as to its source, see row 16 |
| 4 | A whole-tool deny removes the tool from the model's view; so R1's run had no Read, Write, Edit, Glob, Grep, WebFetch at all | permissions 66 | hard rule of kind tool renders as the bare tool name, `rendering_permissions.rs:44` | not observed in the run itself (nobody looked at which tools the model was offered) | **open**: needs the run's first model call from the proxy record |
| 5 | Order is deny, then ask, then allow; first match; a deny at any settings level beats an allow at any level | permissions 62-64, 671-673 | Lys only appends to deny, never removes: `rendering_permissions.rs:78-86` | none | **open**: no real observation |
| 6 | Rules on shell commands match the command's text and "aren't a security boundary" (`/bin/rm`, `sh -c` pass a `Bash(rm *)` deny) | permissions 251-261 | the judge refuses to treat shell text as authority: `lys-runner/src/judge.rs:21-24` | none | **open**: no real observation |
| 7 | Read and Edit deny rules do not stop a script the agent runs from opening the file; only the sandbox does | permissions 346 | n/a | none | **open** |
| 8 | The sandbox covers shell commands and their children only; Read, Edit, Write, WebFetch, hooks and local MCP servers run outside it | sandboxing 38-44 | n/a | none | **open** |
| 9 | In a file passed with `--settings`, a path rule with one leading slash is relative to that file's folder; two slashes is the disk root | permissions 353-369 | Lys passes `--settings settings.json` (`rendering_launch.rs:102`); a hard path rule is written `Tool(/{path})` (`rendering_permissions.rs:52`) | R1 has `Read(//)` and `Read(///**)`, which is that format with path `/` | agreed for hard rules whose path starts with `/`; a person-typed rule's form is **open** (nothing checks it) |
| 10 | Lys's `workspace-only` is not a Claude Code mode. Lys writes it as mode `dontAsk`, allow `Read(./**)`, `Edit(./**)`, deny `WebFetch`, `WebSearch`, reads outside working folders blocked, sandbox on, fail if unavailable, no unsandboxed commands, no network | permission-modes 17-24 lists six modes, none named workspace-only | `lys-home/src/harness/claude_code/launch_env.rs:57-108` | none (R1 was mode `default`) | **open**: no real run in this mode observed |
| 11 | Lys has a tool-boundary judge: a PreToolUse hook that asks the runner before each tool call; denies only, never allows; hard rules always deny; grantable rules are lifted only by a live grant | hooks run before the permission prompt and a deny holds: permissions 558-560 | `lys-runner/src/judge.rs:1-26`, `claude_judge.rs:1-10`, hook built at `lys-home/src/harness/claude_code/launch_env.rs:113-131` | R1 has no `hooks` key: the judge was **not installed** in tonight's run | **agreed** that a start made through the server does not install it: that path writes the settings file with `env_settings(&template)` (`lys-home/src/harness/rendering_launch.rs:79`), which has no hook; only the command-line launch (`claude_code/launch.rs:190-194`, flags `--judge-socket` and `--judge-program`) calls `settings(template, judge)`. So the judge protects no agent started from the screen. Whether anything else passes those flags is still **open** |
| 12 | The person's own Claude Code settings on the computer (user and project settings, hooks, plugins) also apply to a Lys run, because Lys does not replace them | permissions 667-673; settings page (not yet read whole by me) | Lys does not set `CLAUDE_CONFIG_DIR` (Tom's ruling, 3 Oct) | none | **open**: needs a real observation |
| 13 | A mod that handles `tool.check` can approve a call a deny rule refuses, unless managed settings or a Team or Enterprise plan | permissions 562-567 | n/a | none | **open** |
| 14 | `bypassPermissions`: nothing prompts except ask rules, tools needing a person, and removals of critical paths; deny rules still block; allow rules have no effect | permission-modes 26, 31-38; 563-565 | catalogue offers it as a mode | none | **open** |
| 15 | The profile's `tools` are added to the allow list at start | n/a | `launch_permissions.rs:136-139` | R1 allow is empty (pancake's profile names no tools; not checked) | **open** |
| 16 | R1's deny list is pancake's own tool-boundary policy, version 1: seven hard rules (whole tools Read, Write, Edit, Glob, Grep, WebFetch; and Read under path `/`, id `deny-uninspectable`), written out as deny rules at start. The agent's stored permissions are empty lists and mode `default` | a deny in the settings file blocks in every mode: permission-modes 26 | policy model `lys-runner/src/judge.rs:59-115`; hard rules to deny `launch_permissions.rs:153-160`, `rendering_permissions.rs:41-86` | R2: `GET /api/agents/agent-655c…/policy` on the installed Lys, 20:24 by the clock, answered those seven rules, digest 09ec8e24…; `GET …/provisioning` answered permissions allow [], deny [], ask [], mode default, tools [], version 2 | **agreed** |
| 17 | Lys already has a screen for an agent's policy (add a rule: id, tool, kind, target, who may lift it) and one for its refusals, on the agent's file, not on the settings form and not on the front page | n/a | `surface/identity/src/features/file/AgentPolicy.tsx`, shown from `sections.tsx:222` | not looked at in a browser | **open**: needs the browser |
| 18 | The provisioning answer for pancake says `enforced: false` | n/a | field on `ProvisioningAnswer` (`Provisioning.tsx:18`); what sets it not read | R2 | **open**: meaning not read |
| 19 | Who made pancake's policy, and when | n/a | n/a | not looked up | **open** |

## Codex

Written 20:28 by the clock, before reading Archie's table. D = the fetched pages in
`docs/harness/reference/codex/` (a5425952). I have no real observation of my own for any Codex row, so
every row is **open** on that leg.

| # | Claim | D | L | R | State |
|---|---|---|---|---|---|
| C1 | Lys refuses to start a Codex agent whose settings carry any allow, deny or ask rule, or whose policy has any hard rule: "native tool rules and hard policy rules are unsupported" | n/a | `lys-home/src/harness/codex/launch_template.rs:44-53` | none | **open**. Consequence if true: pancake's kind of policy cannot be put on a Codex agent at all |
| C2 | The only modes Lys accepts for Codex are `read-only`, `workspace-write`, `danger-full-access`, passed as `--sandbox <mode>` | agent-approvals-security 360-371 names `--sandbox` with those values | `launch_template.rs:54-60`; `launch.rs:109-122` | none | **open** |
| C3 | When the agent's settings name no mode, Lys passes no `--sandbox`; Codex then takes its sandbox from the computer's own Codex config, or its launch default (a version-controlled folder: workspace write with on-request approvals; otherwise read-only) | agent-approvals-security 279-288 | `launch.rs:109` (`if let Some`), comment at 88-92 | none | **open** |
| C4 | Two paths, and they differ. The home's launch (`lys-home/src/harness/codex/launch.rs`) never sets Codex's approval policy: it is whatever the computer's own Codex config says. The runner's `lys-bound` profile (`lys-runner/src/codex_policy.rs:81-82`) writes `approval_policy = "never"` and `default_permissions = "lys-bound"` with a filesystem table and a network domain table | two layers, sandbox mode and approval policy: agent-approvals-security 60-62 | `launch.rs:88-89` (comment), no approval flag in that file; `codex_policy.rs:54-111`; the only caller of `codex_policy::render` I found is `codex_policy_readback.rs:6` | none | **open**: which path a start made through the server takes is unread. Corrected 20:29: my first wording said no approval policy was set anywhere; my own search, run in the same command as the row was written, showed `codex_policy.rs:81`. I wrote the row before reading the output |
| C5 | For `workspace-write` Lys also sets: command network off, no extra writable roots, temp folders excluded, web search disabled | keys not yet checked by me against the configuration reference | `launch.rs:110-121` | none | **open**, D unread |
| C6 | Extra folders are passed as `--add-dir` | not read by me | `launch.rs:124-126` | none | **open**, D unread |
| C7 | Codex's sandbox is enforced by the operating system (Seatbelt on macOS), and on macOS Codex refuses a command rather than run it unsandboxed when the policy cannot be enforced | agent-approvals-security 425-428; permissions 511-514 | n/a | none | **open** |
| C8 | The sandbox governs local commands only. MCP servers, connectors, web search, the browser and computer use have their own controls and are not held by it | permissions 467-507 | n/a | none | **open** |
| C9 | In workspace write, `.git`, `.agents` and `.codex` under a writable root stay read-only | agent-approvals-security 291-298 | n/a | none | **open** |
| C10 | Lys's runner can render a Codex permissions profile named `lys-bound` from a containment plan; its own comment calls it "a config fragment ... not an enforcement receipt" | permission profiles exist: permissions 45-200 (headings only read) | `lys-runner/src/codex_policy.rs:1-16` | none | **open**: whether any start made through the server uses it is unread |

## Reconciled with Vesper's and Archie's tables, 20:30 by the clock

Read after mine was written. No row of mine contradicts one of theirs. Vesper's rows 1, 3, 5, 8, 9, 11, 13, 15 are my 11, 16, 12, 5,
4, 9, 10, 13. What theirs add that mine lacked: Vesper 4 (nobody chose pancake's policy: Lys seeded it for agents added between
1 October 17:39 and 2 October 12:33 and never took it back), Vesper 5 (a plugin's hook ran inside a Lys run; seen in the run's own
record), Vesper 14 (hooks sit outside every limit in the file). Archie's Codex table has real observations; mine had none.

One row of Vesper's I can close. Her 17: six of nine runs ended within seven seconds, five with exit status 129, cause open.
All nine runs in the runner's `sessions.json` ended with 129 or `Hangup: 1`. The runner ends a session by sending its process
group the hangup signal (`lys-runner/src/pty.rs:336`), and 129 is how a program killed by hangup reports. The seven-second runs
started 10:57:16, 11:54:34, 14:46:48 and 19:55:07 on 3 October; my installs finished 10:57:07, 14:46:30 and 19:55:01 by my own
record, and each install is followed by my readback script, which starts a run and ends it. So those are my readbacks ending
their own runs, not an agent failing to start. The 11:54 run has no install beside it in my record: **open**.

## Not yet read by me

permission-modes 47-546 and 660 to the end; sandboxing 53 to the end; settings; iam; Lys's policy records
(the rules with hard and grantable authority) and `containment_policy.rs`. Codex: the configuration reference,
managed configuration, hooks, rules, auto-review, sandboxing.md, permissions.md 1-446; Lys's Codex files other than
`launch.rs` 85-128 and `launch_template.rs` 30-110.
