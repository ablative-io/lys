# A clean start for Claude Code: what Lys switches off, and how that is known

Archie, 3 October 2026. Asked by Tom at 20:47: a run Lys starts must not load the person's own settings, hooks, plugins, MCP servers or connectors, and must still be signed in. Every row names the page line, the Lys code and what was seen. Claude Code here is 2.1.288 (`claude --version` on this Mac).

## How it was seen

No model was called and nothing was sent anywhere. `claude -p hi --output-format stream-json --verbose --include-hook-events` prints, before its first request, one line listing the tools, MCP servers, plugins, skills, agents and commands it has loaded. The request itself was pointed at a closed port on this Mac (`ANTHROPIC_BASE_URL=http://127.0.0.1:9`) and the program was stopped once that line was out. Run in an empty scratch folder, as Tom's login, so the person's setup was really there to be loaded.

- **B0**, 20:50: no extra flags. The person's setup as it loads today.
- **A2**, 20:50: `--setting-sources "" --strict-mcp-config --mcp-config mcp.json`, the file naming one probe server.
- **A3**, 20:50: `--mcp-config mcp.json --settings s.json --setting-sources= --strict-mcp-config`, with `s.json` setting the mode to `plan`. This is the form Lys writes.

## What the two flags do

| # | Claim | Page | Lys code | Seen |
|---|---|---|---|---|
| 1 | With an empty list of setting sources, the person's settings, the folder's and the folder's local ones are not loaded. | cli-reference.md 129: "Comma-separated list of setting sources to load (`user`, `project`, `local`)". The page does not say what an empty list does. | rendering_launch.rs, `NO_SETTING_SOURCES` | B0: 13 plugins, 10 of them the person's. A2 and A3: 3, all Claude Code's own (`cc-plugin-*`). |
| 2 | The person's hooks do not run. | settings.md: hooks are a settings key; plugins-reference.md: plugins carry hooks. No page line says the empty list stops them. | same flag | B0: 6 hook events at start (SessionStart, UserPromptSubmit, InstructionsLoaded). A2 and A3: none. |
| 3 | The person's plugins, and the skills and agents they bring, are not loaded. | plugins-reference.md 206: a plugin is on by the person's `enabledPlugins`, which is a settings key. | same flag | B0: 46 skills, 23 agents. A2: 19 skills, all Claude Code's own; 5 agents, none from a plugin. The person's own skill folder (`norn` in B0) is gone too. |
| 4 | Only the MCP servers Lys passes are used: none of the person's, none from a plugin, no claude.ai connector. | cli-reference.md 131: "Only use MCP servers from `--mcp-config`, ignoring all other MCP configurations"; mcp.md 598 the same. | rendering_launch.rs, `ONLY_GIVEN_MCP` | B0: servers `plugin:argus:argus` and `claude.ai Claude Docs`, 70 MCP tools. A2 and A3: the probe server only, no MCP tools. |
| 5 | The settings file Lys passes still applies. | cli-reference.md 125 names `--settings` apart from the user, project and local files. | rendering_launch.rs: `--settings settings.json` | A3: the start line reports `permissionMode` `plan`, which only `s.json` set. |
| 6 | The empty list works written as one word, `--setting-sources=`. | not on the page | `NO_SETTING_SOURCES` is that word, so no empty argument has to survive a command line | A3 used that form and matched A2. |
| 7 | The sign-in is kept. | iam.md 206 to 209: on macOS credentials are kept in the Keychain, else in `.credentials.json`, and both move with `CLAUDE_CONFIG_DIR`; neither is a settings file. `--bare` is the flag that drops them (cli-reference.md 74 and `claude --help`: "OAuth and keychain are never read"); neither flag Lys adds says so. | Lys sets no `CLAUDE_CONFIG_DIR` (the test asserts it) | The key names of the person's settings file were read: none is a credential or a key helper, so leaving that file out removes no sign-in. **Not seen**: a signed-in request under the two flags, because no model was called. The first Lys-started run after the install shows it in the proxy's record. |

## What the two flags do not switch off

| # | Thing | What was found | What would do it |
|---|---|---|---|
| 8 | The person's `CLAUDE.md` and the folder's. | Not seen either way: the start line does not list instruction files, and no model was called. cli-reference.md 129 does not say. | `--safe-mode` drops it (cli-reference.md 127), but also drops skills and MCP servers, and the page does not say it spares the ones Lys passes; `--bare` drops it and the sign-in with it. To be read from the proxy's record of the first run after the install. |
| 9 | Auto memory. | A2 still names a memory folder under the login's `~/.claude/projects/`. | `--safe-mode` or `--bare`, with the costs in row 8. Not written. |
| 10 | Claude Code's own three plugins and 19 skills. | A2, A3. They are the program's, not the person's. | Nothing; they are part of the program. |
| 11 | Managed settings. | Not on this Mac. cli-reference.md 125 and 127 say managed policy always applies. | Nothing, by design of the program. |
| 12 | The trust row Lys writes in `~/.claude.json`. | Not checked under the flags: `-p` skips the trust question. | If the terminal run stops at a trust question after the install, this is why. |
| 13 | The second launch line, used when a seat is forked (`claude_code/launch.rs` `launch_line`). | It does not carry the two flags and was not changed tonight. | The same two words, in that line and its tests. |

## Codex

Not written tonight: it is not one or two arguments.

| # | Thing | Why an argument does not do it |
|---|---|---|
| 14 | The login's config, hooks and MCP servers | They load from the Codex home. Pointing `CODEX_HOME` elsewhere leaves them out and leaves the sign-in out with them, since the sign-in file lives in the same folder (codex/FACTS.md rows 37 to 40). |
| 15 | `AGENTS.md` from the home and the folder | They reach the model even in replace mode (codex/FACTS.md rows 37, 39, seen in O5). |
| 16 | The login's skills | Listed to the model from `~/.agents/skills` even with the Codex home elsewhere (codex/FACTS.md row 40). No page read names a switch. |

What would do it: a Codex home that Lys writes, holding only the sign-in and Lys's config, plus a documented switch for skills and for `AGENTS.md`. Neither switch has been found on the pages read.
