# Codex: what is known, and from where

Archie, 3 October 2026. Written for the permissions page; nothing is to be built from it until Tom has read that page.

A row is **agreed** only when three things say the same: Codex's own page, Lys's code at a path and line, and something seen for real. Anything less is **open**, and the row says which leg is missing. **disagrees** means two legs contradict each other.

## The three legs

**Documentation.** Codex's own pages as fetched 20:22 on 3 October into this folder (a5425952). File names below are short: `approvals` is agent-approvals-security.md, `perm` is permissions.md, `sandbox` is sandboxing.md, `rules` is agent-configuration-rules.md, `hooks` is hooks.md, `basic`, `advanced`, `reference` are the config-file pages, `managed` is enterprise-managed-configuration.md, `env` is config-file-environment-variables.md.

Read whole: approvals, sandbox, permission-modes, rules, perm, hooks, sandboxing-auto-review, basic, env, advanced, managed, and reference lines 1 to 1500. **Not yet read: reference lines 1501 to 2883 (the rest of config.toml and all of requirements.toml), agents-md, models.** No row below leans on an unread page.

**Lys.** Main at a5425952. Read whole: `crates/lys-home/src/harness/codex/` launch.rs, launch_template.rs, zone.rs, leaf.rs, mod.rs; `crates/lys-home/src/harness/` rendering.rs (lines 1 to 200), rendering_permissions.rs, rendering_launch.rs (60 to 126); `crates/lys-runner/src/` codex_policy.rs, codex_policy_contract.rs, codex_policy_readback.rs, codex_judge.rs, codex_judge_client.rs, codex_refusals.rs, launch_config.rs; `crates/lys-identity-server/src/launch_permissions.rs` lines 1 to 140. **Not yet read: account.rs, beside.rs, parts.rs, rollout.rs** (they translate a session into a Codex rollout; they render no permission).

**Seen for real.** The installed program on Tom's Mac is `codex-cli 0.159.2` (`codex --version`, 20:24). Every observation ran it with `CODEX_HOME` pointed at an empty scratch folder, so Tom's own Codex folder was neither read nor written, no sign-in was used and no model was called.

- **O1** `codex --help`: the flags it takes.
- **O2** `codex sandbox -c ... -- /bin/sh -c <probe>`: a probe that tries to write in the working folder, in `.git` inside it, outside it, in `/tmp` and in `$TMPDIR`, to read a file outside the working folder and `/etc/hosts`, and to fetch https://example.com. Run under A, the exact settings launch.rs writes for workspace-write; B, read-only; C, workspace-write with Codex's defaults; D, danger-full-access.
- **O3** a scratch `config.toml` holding the profile codex_policy.rs writes (`lys-bound`), run with `codex sandbox -P lys-bound`.
- **O4** the installed runner's session folders, names only: nine sessions under `data/runner/sessions`, every one holding `settings.json` and `mcp.json`, which are Claude Code's files.

**What was not seen, and matters most:** no Codex agent has ever been started by the installed Lys (O4). Nobody has observed what a real Lys start hands Codex, or how it then behaves. Every row about a Lys Codex run is therefore open on that leg, however well the other two agree. O2 and O3 show how the program behaves under the settings Lys's code writes; they are not a Lys run.

## What Lys hands Codex today

| # | Claim | Codex's page | Lys | Seen | Status |
|---|---|---|---|---|---|
| 1 | Codex has three sandbox modes: read-only, workspace-write, danger-full-access. | reference 217-220; sandbox 240-247 | launch_template.rs:57 accepts exactly these three; catalogue codex.json modes | O1: `--sandbox` possible values are these three | **agreed** |
| 2 | Lys sets the sandbox only when the profile names a mode. A Codex profile with no mode starts with no `--sandbox` at all. | With nothing set, Codex itself picks: Auto in a version-controlled folder, read-only otherwise, or whatever its config says (approvals 281-289; basic 32-40) | launch.rs:109 `if let Some(sandbox)`; the server demands a mode only for Claude Code (launch_template.rs:153-165 of the identity server) | Not seen. No Lys Codex start exists (O4) | **open**: nobody has seen what such a run gets. By the code, the login's own Codex config decides, and Lys does not know what that is |
| 3 | Lys never sets when Codex asks before acting (the approval policy). It is left to the machine's own Codex config. | The policy is `on-request` or `never`, from flags or config (reference 157-160; approvals 303-309) | launch.rs:88-92, the comment says so and no argument sets it | O1: the flag exists (`-a on-request` or `never`); Lys's argument list has none | **open** on a real run. Consequence either way: under `on-request` a Lys run stops at a prompt in a terminal nobody may be watching; under `never` it never asks |
| 4 | Lys does not give Codex its own config folder. The run reads the login's `~/.codex`: its config, rules, hooks, plugins, MCP servers and sign-in. | `CODEX_HOME` defaults to `~/.codex` and holds all of these (env 19; advanced 93-100; rules 48; hooks 44-49) | launch.rs:88-92; launch_template.rs:140 refuses a profile that sets `CODEX_HOME` | O4 only: no Codex run to look at | **open**: whether Tom's own Codex rules, hooks and plugins reach a Lys run has not been seen |
| 5 | Settings given on the command line win over project and user config. | Precedence: command-line flags and `--config` first (basic 32-40) | launch.rs:111 leans on this | Not tested (it would need a user config to lose to a flag; the scratch home had none in O2) | **open**: missing the observation |
| 6 | A managed machine can override even the command line. | `managed_config.toml` and MDM "override the user's local config.toml and any CLI `--config` overrides" (managed 874-876, 901) | Lys has no code that looks for managed config | Not seen | **open**: Lys does not account for it |
| 7 | Under read-only, commands cannot write anywhere, and cannot use the network. | approvals 365; sandbox 240-241 | launch.rs:122 passes `--sandbox read-only` and nothing else | O2 B: all five writes refused, network refused | **agreed** for the sandbox itself; a Lys run still open (O4) |
| 8 | Under every mode, commands can read the whole disk. Read-only does not mean "reads only the working folder". | The modes are described by where they write; none says reading is confined (approvals 364-366). Confining reads needs a permission profile with `:root = "deny"` (perm 558-584) | Lys writes no read restriction for Codex (launch.rs:109-126) | O2 A, B, C, D: reading a file outside the working folder and `/etc/hosts` allowed in all four | **agreed**. A person told "read-only" must be told it reads everything the login can read |
| 9 | Under workspace-write with Lys's settings, commands write only in the working folder; not `/tmp`, not `$TMPDIR`, not outside; no network. | `exclude_slash_tmp`, `exclude_tmpdir_env_var`, `writable_roots`, `network_access` (reference 222-245) | launch.rs:110-121 writes all four, and `web_search="disabled"` | O2 A: working folder allowed; `/tmp`, `$TMPDIR`, outside refused; network refused. O2 C (Codex's defaults) allowed `/tmp` and `$TMPDIR`, so Lys's settings are what closed them | **agreed** for the sandbox itself; a Lys run still open (O4) |
| 10 | `.git` inside the working folder stays read-only under workspace-write. | approvals 293-299 | Lys writes nothing about it | O2 A and C: write to `.git/x` refused; D allowed | **agreed**. An agent in workspace-write cannot commit without leaving the sandbox |
| 11 | Lys turns web search off only under workspace-write. Under read-only and danger-full-access it is left at Codex's default. | Default is `cached`; under full access it defaults to live (approvals 267-273; basic 110-115) | launch.rs:117 sits inside the workspace-write branch only | Not seen (needs a model call) | **open**. By page and code, a read-only Lys Codex agent has web search on |
| 12 | danger-full-access has no sandbox: writes anywhere, network on. | sandbox 245-247 | launch_template.rs:57 accepts it; launch.rs:122 passes it | O2 D: every write allowed, network allowed | **agreed** |
| 13 | Extra folders are passed as `--add-dir`, and are writable. | No line found in the pages read so far | launch.rs:124-126 | O1: "Additional directories that should be writable alongside the primary workspace" | **open**: missing the page; and Lys also passes `writable_roots=[]` under workspace-write (launch.rs:114), and which of the two wins has not been tested |
| 14 | Lys cannot give Codex any rule about a single tool, file or command. A Codex profile with any allow, deny or ask rule, or any hard policy rule on the agent, is refused at the start. | Codex has no per-tool allow or deny list in config; it has command-prefix rules in `.rules` files (rules 17-50) and permission profiles (perm) | launch_template.rs:44-53; launch_permissions.rs:125-135; catalogue `rule_forms: []` | Not seen as a refusal on the live install | **open** on the observation |
| 15 | A Codex profile that lists any tool is refused at the start. | n/a | launch_permissions.rs:138-140 puts the profile's tools into `allow`; launch_template.rs:45 refuses a non-empty `allow` | Not seen | **open**: read from code only; worth one test |

## What Lys has written for Codex and does not use

| # | Claim | Codex's page | Lys | Seen | Status |
|---|---|---|---|---|---|
| 16 | Lys has code that writes a Codex permission profile (`lys-bound`: whole disk denied, named folders readable or writable, named hosts) and code that checks Codex loaded it. Neither is called by any start. | Profiles are how Codex confines reads (perm 47-57) | codex_policy.rs:54 and codex_policy_readback.rs:34 are called only from `crates/lys-runner/tests/codex_policy.rs` (grep across `crates`, 3 October) | O4: no Codex start exists to contradict it | **agreed** that it is unused. Nothing on a screen may describe it as what an agent gets |
| 17 | That profile as written does not run a command at all. | A profile needs `:minimal` read for common tools (perm 106-107, 575-578) | codex_policy.rs:58-64 writes `"/" = "deny"` and the plan's reads, no `:minimal` | O3: `/bin/echo hi` under it died with exit 134; with `":minimal" = "read"` added it ran | **disagrees** with its own purpose; unused, so no harm today |
| 18 | That profile's comment says an empty host list means no destination is permitted. It does not. | "Network on, proxy off: direct, unrestricted network access. Domain rules are not enforced" (perm 353-361; approvals 144-152) | codex_policy.rs:88-99 sets `enabled = true`, never sets `features.network_proxy`, and says "An empty domain map still means no permitted destination" | O3: with the profile's network table and no hosts, the fetch of example.com succeeded; with `features.network_proxy=true` added it was refused | **disagrees**: page and observation agree with each other and contradict the code's comment. If this code were ever used as written, the agent would have the whole internet |
| 19 | That profile writes `mode = "full"` under network. | Not in the profile table (perm 173-200), not in reference 1-1500 | codex_policy.rs:90 | O3: the installed Codex accepted the file without complaint | **open**: undocumented key, meaning unknown |
| 20 | Under that profile (with `:minimal`), a file in the home folder could not be read, but `/private/tmp` outside the named folders could be written. | `:workspace` grants temp folders; this profile does not extend it (perm 140-146, 580-583) | codex_policy.rs:58-64 | O3: read of a file under `/Users/tom` refused; writes under `/private/tmp` outside both named folders allowed | **open**: the write is unexplained. The probe folders were themselves under `/private/tmp`; needs repeating from a folder elsewhere |
| 21 | Lys pins one Codex build by the hash of its bytes and refuses any other. Nothing calls it. | n/a | codex_policy_contract.rs:76 `verify_package` has no caller (same grep) | Installed is 0.159.2; zone.rs:22 measured 0.156.0 for rollouts; tracking.rs:45 lists 0.156.0 and 0.161.0-alpha.3 | **open**: three different version lists, none matching what is installed |

## The judge and hooks

| # | Claim | Codex's page | Lys | Seen | Status |
|---|---|---|---|---|---|
| 22 | Codex has hooks, including one before each tool call that can refuse it. | hooks 750-782 | codex_judge.rs:109-125 writes that exact refusal shape; `lys runner judge --harness codex` exists (commands/runner.rs:72-78) | Not seen | **open** on the observation |
| 23 | Lys never installs that hook for a Codex run. | Hooks come from `hooks.json` or `[hooks]` next to a config layer (hooks 34-49) | No code writes a Codex hook: the only writer of a judge hook is Claude Code's launch_env.rs:127 (grep for `PreToolUse` and `runner judge`). tracking.rs:54 says "no pre-tool hook" | O4 | **agreed** that none is installed, as far as code and the absence of any run can show |
| 24 | A profile that requires the judge does not start on Codex. | n/a | tracking.rs:122-126 refuses with `policy_not_supported` | Not seen | **open** on the observation |
| 25 | If Lys did install its hook, Codex would skip it until a person trusted it, unless started with `--dangerously-bypass-hook-trust` or delivered as managed. | hooks 64-80 | No code handles hook trust | O1: the flag exists | **open**; a design fact for later, not a present behaviour |
| 26 | A Codex hook is not a wall. A hook that errors, times out or answers badly lets the tool call through; web search and some tool paths never reach it; a later `write_stdin` is not asked again. | hooks 462-467, 521-524, 828-830; managed 711 | codex_judge.rs:7-9 and :75-79 say the same | Not seen | **open** on the observation. Page and code agree: the hook cannot be the only thing between an agent and harm |
| 27 | Codex reports a refused command or file change without saying why. | Not found in the pages read | codex_refusals.rs:30-36, :59-65 | Not seen | **open**: one leg only |

## What Codex offers that Lys does not use

These are the program's own granular controls. The page and O1 are the only legs; Lys has no code for any of them, so each is **open** and is listed so the design does not assume them away.

| # | Control | Codex's page |
|---|---|---|
| 28 | Permission profiles: per path `read`, `write` or `deny`, with globs for deny; per host `allow` or `deny`. Beta. They do not combine with `--sandbox`: if a sandbox mode is given anywhere, the profile is ignored. | perm 7-21, 173-200 |
| 29 | Lys's `--sandbox` flag therefore switches permission profiles off for that run. | perm 9-14; launch.rs:122 |
| 30 | Command rules: allow, prompt or forbid by command prefix, strictest wins; apply to commands leaving the sandbox. Experimental. Read from `rules/` beside every config layer, including the login's. | rules 7-9, 48, 66-69 |
| 31 | Granular approval: five named kinds of prompt, each allowed to surface or auto-rejected. | reference 157-190; approvals 309 |
| 32 | Automatic review: a second agent answers approval prompts in place of a person. Not a guarantee. | sandboxing-auto-review 7-15, 267-275 |
| 33 | Requirements an administrator sets and a user cannot loosen: allowed modes, allowed profiles, deny-read paths, forbidden commands, managed hooks, allowed MCP servers. Read from `/etc/codex/requirements.toml`. | managed 58-69, 93-99, 685-705, 755-770 |
| 34 | The environment handed to commands: by default names containing KEY, SECRET or TOKEN are **not** removed. | basic 171-173; advanced 419-421 |

## Where the page and the program disagree

| # | Claim | Codex's page | Seen | Status |
|---|---|---|---|---|
| 35 | The sandbox test command is `codex sandbox macos ...`. | approvals 414-421 | O1: 0.159.2 takes `codex sandbox [OPTIONS] [COMMAND]`; `codex sandbox macos --help` tried to run a program called `macos` | **disagrees** |
| 36 | The flag to pick a profile for that command is `--permissions-profile`. | approvals 416 | O1: it is `-P, --permission-profile` | **disagrees** |

## The worst reading, for a ward

If a Codex agent were started by Lys today in a children's intensive care unit: with no mode chosen it runs under whatever the login's own Codex config says, which Lys neither sets nor reads (rows 2, 3, 4). With "read-only" chosen it still reads every file the login can read, and may still search the web (rows 8, 11). No Lys rule about a tool, a file or a host reaches it (row 14), and no judge sees its tool calls (row 23). The confinement Lys has written for Codex is not wired in, and as written would either not run or leave the network open (rows 16 to 18). None of this has been watched on a real run, because there has never been one (O4).
