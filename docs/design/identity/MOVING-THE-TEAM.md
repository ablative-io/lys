# Moving the team onto Lys: what it takes

Tom, 5 October 2026, about 08:10: "it'd be really good if we could start looking at... what we need to do
to move the team over. So maybe do a bit of an audit on that." This is that audit, by Waffles, the same
morning. It was read from the running seats, the seats folder and the installed Lys (0feba1e8); nothing
here was tried by moving a seat. Each line says whether it was read or is my inference.

## How a seat starts today (read from the running processes and /Users/tom/Developer/seats)

A Claude seat is one `claude` command carrying, per seat:

- `--resume <session id>`: the seat's one long conversation, kept across restarts.
- `--append-system-prompt-file`: the seat's own instructions (Waffles, Apollo).
- `--settings <seat>/settings.json`: model, permissions, environment (`ARGUS_URL` and others), the status
  line, enabled plugins (Argus, manifold, three more for Waffles), marketplaces.
- `--mcp-config <seat>/mcp.json`: Cambium, Dot, aion and others, each with the seat's identity in its env.
- `--dangerously-load-development-channels server:dot server:cambium plugin:argus@ablative`: how a
  Cambium post, a Dot turn and an Argus notice wake the seat.
- `--dangerously-skip-permissions` or a `--disallowedTools` list; `--model` for some.
- A secret in the seat's `env` file (`CLAUDE_CODE_OAUTH_TOKEN`), and for Waffles its own config folder.

A Codex seat is `codex resume <id> --profile <name>-channels`, on a patched Codex build
(`codex-channels`), with its profile in `~/.codex/<name>-channels.config.toml`. Seven are running.

## How Lys starts an agent today (read from pancake's own process and its profile on the install)

`claude --mcp-config <run>/mcp.json --settings <run>/settings.json --setting-sources= --strict-mcp-config
--model default`, in the agent's working folder, through Lys's proxy. The profile a person sets holds: the
program and installed copy, model, system prompt (keep or replace), tools, skills, MCP servers (http or
stdio; channel policy off or wake), permissions (seven modes, bypass among them; allow, deny and ask
rules; extra folders), and the working folder.

## What is already there

| A seat needs | Lys has it | How I know |
|---|---|---|
| Its own instructions | Yes: the profile's system prompt | read on the profile |
| Its MCP servers, with its identity | Yes: the profile's MCP servers, stdio and http | read on the profile; not tried with Cambium or Dot |
| Permission mode and denied tools | Yes: modes and deny rules | read on the profile |
| A model | Yes | read |
| A working folder | Yes, named per agent | read |
| A terminal a person can watch and type into | Yes: the canvas | walked |
| Every model call kept, with tokens and the account's windows | Yes: the proxy | walked 5 October: two calls, 1,178 and 2 tokens in, account at 30% of 5 hours |
| Start, stop, restart, stop everything | Yes | walked |
| Teams, with a lead and a parent | Yes; boxes on the canvas since 0feba1e8 | tests; not walked (no active team on the install) |

## What is missing, in the order I would do it

1. **Resume.** A seat is one conversation that survives a restart. A Lys start from the service is a fresh
   session each time: pancake's profile names no session and its launch line carries no `--resume`. The
   home crate can render a resume of a session it has written (`claude --resume <file> --fork-session`,
   `harness/claude_code/launch.rs`, proved in PROOF-RESUME.md), but the service's start does not use it
   yet, and "Session handover" is on the canvas list under the runner. Until a Lys agent can be started
   on its existing session, moving a seat means losing its context.
2. **Waking.** Seats are woken by Cambium, Dot and Argus through development channels. Lys renders
   `--channels server:<name>` for each MCP server whose channel policy is "wake" (`harness/rendering.rs`),
   so the piece is written. It has not been shown with a real server: pancake has no MCP server, and the
   seats' flag today is `--dangerously-load-development-channels`, which is a different flag. Whether
   `--channels` wakes a Lys agent from Cambium and Dot is the first thing to try.
3. **Hooks, plugins and the user's settings.** Lys starts Claude Code with `--setting-sources=` (none), so
   nothing from the machine's own setup reaches the run: no Argus plugin (context watch, compaction), no
   manifold plugin, no hooks (my own written-first gate is one). This is the "managed-settings slice"
   that has been an unwritten brief since 3 October. Your ruling of that day says a Lys-started AI uses
   the machine's own Claude Code setup; the launch line does the opposite today.
4. **Codex.** Seven seats are Codex. Lys has a Codex launch template in the home crate, but only Claude
   Code is offered as a harness on the install, no Codex agent has run through Lys, and the seats use a
   patched Codex build for channels.
5. **Accounts.** Each seat draws on an account by an OAuth token in its env file. Lys records the account
   a call was made on (seen: the organisation id and its two windows), but "who draws from which account,
   registered by a person" is still on the list, and so is moving between accounts at a limit in practice.
6. **Voice.** Dot is an MCP server with the seat's name in its env, and it needs the channel to hear you.
   It follows from 2 and the MCP row above; not tried.
7. **Context percent and compaction.** The usage widget shows no context figure until a window is declared
   for the model (Settings, Models), and Argus is what compacts a seat today. Lys has a compact route
   (`/runtime/sessions/{id}/compact`) and a context limit; not walked with a real seat.
8. **The seat's status line.** Dollars and running time come from it; next on the usage list already.

## What I found and fixed or wrote down this morning while reading

- The usage widget says "Tokens today: Not reported" although two calls reported tokens: the agent's
  start-up probe is refused by the provider with HTTP 429 and carries no figures, and one call of unknown
  spend makes the day's total unknown. I first wrote that a refused call spent nothing and counted it as
  nought; Tom refused that the same morning ("don't make any assumptions ever"), and it was taken out: the
  period says what was reported and how many calls reported none.
- pancake's "refused account" was only that start-up probe. A typed prompt made real calls.

## The smallest real move

One Claude seat that is not doing live work (Archie is stopped) started through Lys with its own system
prompt, its Cambium and Dot servers and its working folder, as a new session. That shows 2 and 6 for
real and costs no context. Resume (1) and settings (3) are what make it a move and not a copy.
