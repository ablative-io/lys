# Claude Code under Lys: what is sourced, and what is not

Written by Vesper, 3 October 2026, without reading Waffles' or Archie's tables.
Second writing the same evening, after Lys's Claude Code folder was read whole
and after the three tables were put side by side: rows 1, 5, 9, 13, 16, 17 and
18 changed, rows 21 and 22 are new, and each change says whose observation it is.
Nothing is built from this page until Tom has read it.

A claim is **agreed** only when three things say the same: Claude Code's own
page (the copies in this folder, by file and line), Lys's code (path and line,
at commit 11f379c0; the lines changed in the second writing were read at df36a469), and something seen for real on Tom's Mac. A claim with
fewer is **open**, and the row says which leg is missing. **Disagrees** means
two legs say different things.

Page names: P = permissions.md, M = permission-modes.md, S = settings.md,
X = sandboxing.md, I = iam.md. I read the decisive lines myself; the rest of
each page was read whole by a reader working only from the text, and every
line number below was taken from the file, not from memory.

What was seen for real, and where:

- **The files nine runs were handed.** The installed Lys keeps each run's
  files under `~/Library/Application Support/lys/identity/data/runner/sessions/<run>/config/`.
  All nine hold `settings.json`, `mcp.json`, `instructions.txt`. Every
  `settings.json` has exactly two top members, `env` and `permissions`. I
  read `env` by variable name only.
- **Tonight's run of pancake**, `op-e85561b1c5386faffafe8a2b5a1d21cb`, started
  19:55:07, ended after 7 seconds with exit status 129. Its `permissions`:
  `defaultMode` `default`; `allow`, `ask`, `additionalDirectories` empty; `deny`
  = `Read`, `Write`, `Edit`, `Glob`, `Grep`, `WebFetch`, `Read(//)`, `Read(///**)`.
  Its `env` names: `ANTHROPIC_BASE_URL`, `LYS_AGENT`, `LYS_MACHINE`,
  `LYS_PROVISIONING_VERSION`, `LYS_SESSION`.
- **An earlier run of pancake**, `op-f0f55016…`, 19:19:54, 262 seconds. Claude
  Code wrote its own record of that run into Tom's own folder:
  `~/.claude/projects/-Users-tom-Developer-seats-people-pancake/ab7e825f-6019-43cb-91d5-5c3febf8606c.jsonl`.
  It says version 2.1.288, permission mode `default`, and records two hooks
  that ran and succeeded: `PreToolUse:Bash` (line 23) and `Stop` (line 33),
  both the command `${CLAUDE_PLUGIN_ROOT}/scripts/gate.sh`.
- **The installed policy store**, `data/policies/leaves/segments/00000000000000000000`:
  fifteen first-version policies. Ten hold the same seven hard rules; the
  five newest hold none.

## The three things asked

| # | Claim | Claude Code's page | Lys's code | Seen for real | Standing |
|---|---|---|---|---|---|
| 1 | A run started from the Lys screen or API never gets the judge hook. The hook is written only by the `lys-home render-launch` command, and only when both `--judge-socket` and `--judge-program` are given. | A PreToolUse hook runs before the permission prompt and can deny (P 560). A hook nobody installs is not discussed. | The server's start renders through `rendering_launch.rs:79`, which calls `env_settings`, the form with no hook. The form with the hook, `launch_env.rs:149-168`, is reached only from `claude_code/launch.rs:190-194`, behind the two flags declared at `:100-107`. No other code writes `PreToolUse` into a settings file. The judge's own file says "A hook the harness does not run protects nothing" (`lys-runner/src/claude_judge.rs:8-10`). | None of the nine runs' `settings.json` has a `hooks` member. | **Agreed.** The words in `launch_env.rs:112-113` ("sends each of the session's tool calls to the runner's judge") are true of one command and false of every run a person starts. |
| 2 | Policy rules a grant may lift ("stays with the judge hook", `launch_permissions.rs:6-8`) are therefore enforced by nothing in a run started from the screen. | Same as row 1. | Only rules with authority `hard` become settings denies (`launch_permissions.rs:150-157`); the rest are left for the judge, which row 1 shows is not installed. | No policy on the installed Lys holds a rule that is not hard, so no run has shown this. | **Open** on the real-life leg. Code alone says it. |
| 3 | The refusals on pancake came from pancake's own policy, version 1: seven hard rules, ids `deny-Read`, `deny-Write`, `deny-Edit`, `deny-Glob`, `deny-Grep`, `deny-WebFetch` and `deny-uninspectable` (Read under the path `/`). | A bare tool name in deny "removes the tool from Claude's context entirely" (P 68). | Hard rules become denies: a whole-tool rule is the tool's name, a path rule on Read is `Read(/<path>)` and `Read(/<path>/**)` (`rendering_permissions.rs:44-55`), so path `/` gives exactly `Read(//)` and `Read(///**)`. | The policy record for `agent-655c4cdb…` (the `LYS_AGENT` of tonight's run) holds those seven rules, and tonight's file holds exactly the eight denies they produce. | **Agreed.** |
| 4 | Nobody chose that policy. Lys wrote it for every agent added between 1 October 17:39 and 2 October 12:33, and has never taken it back. | Not a matter for the page. | `PolicyStore::ensure_default` seeded those rules from commit c2e667f1 until commit e06718bc removed them. Today it writes no rules (`agent_policy_store.rs:272-281`) and returns early when an agent already has a policy, so an agent added in that window keeps the old rules for ever. | Ten of the fifteen installed policies are that seed, word for word; the five newest are empty. | **Agreed** (code history and the store agree; the page has no part). A design row: an agent's restrictions today depend on the day it was added. |
| 5 | Tom's own Claude Code plugins and hooks run inside a Lys run. | Hooks, local MCP servers and plugin monitors "run with your full access", outside the sandbox (X 13, X 45). The pages do not say which settings sources' hooks run when a file is passed with `--settings`. | Lys sets no config folder: "The run uses the machine's own Claude Code setup: its config folder, settings, sign-in and MCP servers" (`rendering_launch.rs:96-98`); the arguments it builds from `:99` name its own MCP file and settings file and set no config folder. | The 19:19 run's record is in Tom's own `~/.claude/projects`, and it records a plugin's `gate.sh` running on `PreToolUse:Bash` and on `Stop`. Archie's observation, from the proxy's kept request for that same session: of the 103 tools the model was offered, 70 came from Tom's own setup (8 from a connector, 62 from a plugin). | **Agreed** by code and observation; the page is silent on the exact point, so the page leg is **open**. |
| 6 | Tom's own user settings (his allow, ask and deny rules) are also in force in a Lys run, merged with the file Lys passes. | `--settings` applies "above your user, project, and local files and below managed settings" and keeps lower values for keys it omits (S 568, S 635). List keys such as `permissions.allow` are combined across files (S 646). | Same as row 5: no config folder is set, nothing excludes a source (`--setting-sources` appears nowhere in Lys). | Not seen. The run's record shows hooks, not which rules decided a call. | **Open** on the real-life leg. If true, a person's own allow rules widen what a Lys agent may do, and Lys neither sets nor knows them. |
| 7 | Until some time on 3 October a run had its own config folder, apart from Tom's. | `CLAUDE_CONFIG_DIR` moves "your settings, session history, and plugins" (S 442). | The current code sets none (row 5). The earlier code is in history, not read for this table. | Run `op-c8315377…` (3 October 08:09) has `.claude.json`, `backups`, `cache`, `sessions` inside its config folder; no later run does. | **Open**: seen, and the page explains it, but the code leg is history I have not read. |

## What the permissions in the file mean

| # | Claim | Claude Code's page | Lys's code | Seen for real | Standing |
|---|---|---|---|---|---|
| 8 | Deny is judged first, then ask, then allow; the first match decides; a deny from any file beats an allow from any file. | P 64, P 66, P 675. | Lys writes all three lists (`rendering_permissions.rs:87-88`) and relies on this order. | Not seen: no run has been watched refusing or allowing a call. | **Open** on the real-life leg. |
| 9 | With `Read`, `Write`, `Edit`, `Glob`, `Grep` and `WebFetch` denied by bare name, the agent does not have those tools at all. | P 68; for `WebFetch`, P 489-492. | Row 3. | The denies were in the file. Archie's observation, from the proxy's kept request for session ab7e825f: `Read`, `Write`, `Edit` and `WebFetch` were absent from the tools the model was offered, and `Bash` was still offered. He then looked for `Glob` and `Grep`: neither is in that run's list of 103, but neither is in the list of 107 offered to a run at 14:54 that was offered `Read`, `Write`, `Edit` and `WebFetch`. | **Agreed** for those four, on Archie's observation and not mine. For `Glob` and `Grep` the refusal is not what removed them: this Claude Code (2.1.288) offered no tools by those names in either run, so a rule refusing them by name may refuse nothing, while `Bash`, which searches and reads files as well, was offered in both. **Open**: no other version was seen, and the page (P 68) still lists them as tools. This is the opposite of "Reads freely and asks before most changes", the sentence the screen showed. |
| 10 | A bare `Read` deny may not stop a shell command such as `cat` from reading a file. | The page says Read deny rules reach `cat`, `head`, `tail`, `sed`, `tee` and redirections (P 348) but does not say whether that holds for a deny with no path. Bash itself was not denied. | Lys does not deny `Bash` in that seed. | Not seen. | **Open**: the page does not say, and it was not tried. |
| 11 | `Read(//)` means the top of the disk; a path beginning `//` is absolute, and a path with one leading `/` is relative to the folder of the file passed with `--settings`. | P 355, P 357, P 361, P 371. | Lys writes path rules as `Tool(/<path>)` where `<path>` begins with `/`, so they come out with `//` and are absolute (`rendering_permissions.rs:52`). | The two rules were in the file. Whether Claude Code read `Read(///**)` as a rule or skipped it was not seen. | **Open.** The page says a rule it cannot use is skipped, and a `-p` run says nothing about it (S 610, S 616). |
| 12 | In mode `default` a call that no rule decides asks the person. | P 84; M 21. What happens to such a call when nobody can answer is not said for `default`. | Lys passes the profile's mode through as `defaultMode` (`rendering_permissions.rs:89-91`) and refuses a profile with none (`launch_env.rs:26-41`, `launch_template.rs:150-162`). | Tonight's file says `default`; the 19:19 record says `default`. The run is in a terminal a person can type into, so a prompt can be answered there. | **Agreed** on what was set. What a prompt does when nobody is at the terminal is **open**. |
| 13 | Lys's own mode `workspace-only` is not a Claude Code mode. Lys turns it into `dontAsk`, allows `Read(./**)` and `Edit(./**)`, denies `WebFetch` and `WebSearch`, sets `blockReadsOutsideWorkingDirectories`, and turns the sandbox on with no network and no way out. | `dontAsk` denies every call that would ask, and still runs reads in the working folders, read-only shell commands and allow matches (M 551). The sandbox covers shell commands only (X 13); `failIfUnavailable` and `allowUnsandboxedCommands: false` are as Lys uses them (X 103, X 225); `strictAllowlist` is honoured from `--settings` (X 566); the read block is X 295 and M 45. | `launch_env.rs:74-103`. The same lines refuse the mode when the allow list or the extra folders are not empty (`:75-86`), and every tool a profile lists is put on the allow list first (`launch_permissions.rs:137-140`), so a profile that lists any tool cannot be `workspace-only`. | No installed run used it: every observed file has only `env` and `permissions`, and the modes seen are `default`. | **Open** on the real-life leg. |
| 14 | Hooks sit outside every limit in the file: a hook a plugin installs runs with the person's full access whatever the permissions say. | X 13, X 45. A hook's "allow" does not beat a deny or ask rule (P 562), but a hook is itself a program and is not judged by them. | Lys installs none and removes none (rows 1 and 5). | Row 5: `gate.sh` ran. | **Agreed.** |
| 15 | On a machine with no managed settings and no Team or Enterprise sign-in, an installed mod can approve a call a deny rule refuses. | P 569. M 32 says deny rules block "in every mode" without that exception; the two pages differ. | Lys knows nothing of mods. | Not seen; whether Tom's Mac has a mod was not looked at. | **Open**, and the pages **disagree** with each other. |
| 16 | A run's shell commands inherit the run's environment, secrets included, unless a setting scrubs them. | X 36, X 424. | Lys puts handles, not secret values, in the variables it adds (`launch_env.rs:1-7`, `:49-56`). But those are set "beside the runner's own environment" (`lys-runner/src/pty.rs:39`): the run inherits everything the runner itself was started with, and the environment is cleared only for an isolated start (`pty.rs:137-140`). | The names Lys adds hold no credential name. The names the run inherits from the runner were not read, and no value was read. | **Open.** The first writing called this agreed; it had looked only at what Lys adds. What the runner's own environment holds on Tom's Mac is not known. |

## Things found on the way

| # | Claim | Source | Standing |
|---|---|---|---|
| 17 | Six of the nine runs ended within 7 seconds, five of them with exit status 129. | The runner's `sessions.json` on the installed Lys. | Seen. **Closed by Waffles** for all but one: status 129 is how the runner ends a session, a hang-up to the whole process group (`lys-runner/src/pty.rs:336`), and the 7-second runs were his own readback script starting and ending them. The run at 11:54:34 is not explained and stays **open**. |
| 18 | The copies of the five pages said at their top "fetched 3 October 2026 20:40 AEST" while already in the repository before 20:27. | Line 1 of each page; the Mac's clock. | **Closed.** Waffles corrected the headers to 20:19, the fetched files' own timestamp, and each header now says the earlier time was a guess. |
| 19 | Built-in default mode for a run with no mode set is no longer always `default`: it is `auto` in a terminal from version 2.1.283. Lys refuses a profile with no mode, so this does not reach a Lys run. | M 87-88; `launch_template.rs:150-162`. | Page and code; not seen. **Open.** |
| 20 | The pages do not name `allowManagedHooksOnly`, do not say which sources' hooks run, and do not say whether `~/.claude.json` moves with `CLAUDE_CONFIG_DIR`. | S and X read whole; P 679 mentions only "the `allowManaged*Only` locks". | **Open**: to fetch the hooks page and the settings reference before any design leans on them. |
| 21 | Lys writes into the person's own Claude Code file. Before every Claude Code start the runner records the run's folder as trusted: one row, `projects.<folder>.hasTrustDialogAccepted`, in `.claude.json` under `CLAUDE_CONFIG_DIR` when the launch sets it, else under the login's home. | `lys-runner/src/trust.rs:1-17` (the ruling, "Tom, 3 October 2026"), `:28-40` (which file), `:57` (the row). The file's own words: "this one row is the only write Lys makes there". Row 20 already says the pages do not cover where `.claude.json` lives. | Code only. Tom's `~/.claude.json` was not opened. **Open** on the real-life leg and on the page leg. A design row: a folder chosen on the screen becomes a folder Claude Code trusts, for Tom's own sessions too. |
| 22 | The agent's policy is handed to the runner only on one of the two start paths. | `lys-identity-server/src/launch_record_config.rs:93` passes it; `launch_api.rs:498` passes `policy: None`. | Code only; which path each of the nine runs took was not looked at. **Open.** With no judge hook installed (row 1), what the runner does with a policy it is handed reaches no tool call either way. |

## What was read for the second writing

Lys's Claude Code folder (`crates/lys-home/src/harness/claude_code/`) was read
whole by a reader working only from the code, and I then read every line cited
in the changed rows myself. That reading contradicted row 1 in its command
name and one line number, row 5 in its line numbers, and row 16 in its
standing. It also said the judge never answers "allow" and that the catalogue
lists seven modes; I have not read those lines myself, so they are not rows.
