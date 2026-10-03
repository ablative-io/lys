# Claude Code Mods, read against the proxy: what a mod can reach that a hook cannot, and what it cannot

Waffles, 3 October 2026, from Tom's words on Dot 16:37 to 16:43 ("a more expressive, more in control
version of hooks"; "anything that gives us more control"; "maybe they're trying to close off the proxy
window"; "prepare for both eventualities"). Read in twenty minutes against the official reference and
API pages for v2.1.287 (installed here: 2.1.288), not the announcement. Bounded by that; the
TypeScript declarations Claude Code writes for a build are the complete reference and were not read.

## What a mod is

A plugin with a hooks module (`hooks/hooks.json` naming one `register.js` or `.ts`) that exports
`register(on, options)`. Each `on('event', matcher, async ($, e, next) => ...)` is middleware: `e` is
the event's input, frozen; `next(e)` runs the later hooks and then Claude Code's own behaviour and
resolves to the result; returning an object instead of calling `next` answers the event. `$` is the
mods API: `$.fs`, `$.process`, `$.http`, `$.model`, `$.session`, `$.prompt`, `$.tool`, `$.ui`, and
more. It runs in-process, in TypeScript, with the user's permissions, with no sandbox. Loaded with
`claude --plugin-dir`, `CLAUDE_CODE_PLUGIN_DIRS`, or installed as a plugin; managed settings can
prepend an organisation's mods before every user mod (`prependPlugins`) and refuse all others
(`allowManagedModsOnly`). A built-in guard, `sec-default@builtin`, loads first on managed machines.

## What it can intercept (the control Tom asked about)

| Seam | Event | What a hook can do |
| :- | :- | :- |
| A tool call about to run | `tool.call` | pass, change its input (`next({...e, ...})`), `{ deny: reason }`, or answer with `{ result }` without running it |
| The permission decision | `tool.check` | `{ decision: allow | ask | deny }` after rules, mode and PreToolUse hooks |
| The user's prompt | `prompt.submit` | rewrite text, add context, or `{ drop }` |
| The system prompt | `prompt.compose`, `prompt.section` | reorder sections, rewrite one, omit one |
| First-message context | `prompt.context` | `{ blocks }` for the conversation's first message |
| Claude Code's own reminders | `prompt.attachment` | rewrite or omit |
| Skill text | `skill.prompt` | rewrite |
| One request to the model | `turn.step` | change `model` or `effort` only; an async generator that can watch the step |
| The turn | `turn.start`, `turn.complete`, `$.turn.abort` | observe; show a line; abort |
| Stored transcript rows | `session.append` | rewrite a row's content before it is stored |
| Compaction | `session.compact` | `{ skip: reason }` |
| Cross-session messages | `session.receive`, `session.send` | consume, refuse, observe |
| Subagents | `agent.offer`, `agent.spawn` | withhold a type; change model; deny |
| Settings hooks | `classic.<Event>` | the old hooks' stdin JSON as an event |
| Other mods' calls | `fs.read`, `model.complete`, ... as events | an earlier mod can observe, rewrite or refuse a later mod's API call |
| Usage | `session.measure`, `$.session.usage()` | context tokens, window, percent; rate limits; cost |

A mod can also add commands and tools, call a model of its own (`$.model.complete`, `$.model.fork`
over the conversation with the session's credentials), run timers, submit a prompt from a background
job (`$.prompt.submit`, which starts a turn when the session is idle), and reach files, processes
and HTTP.

## What it cannot reach, which is why the proxy stands

- **The wire.** No event sees the request bytes or the response stream to the Claude API. `turn.step`
  changes the model and the effort of a request; it does not expose the body, the headers, the base
  URL or the transport. `$.model.complete` makes a new request; it does not sit on the conversation's.
- **The base URL.** Nothing in the mods API names it. `ANTHROPIC_BASE_URL` is read by the harness as
  before. The proxy is the only place that sees every call whole, times it, and stores it off the
  forwarding path (PROXY.md). Nothing in this release removes that; whether a later one does is the
  eventuality Tom named, and the answer is the same either way: the proxy is where capture lives,
  and a mod is a second seam, in-process, for things the wire cannot see.
- **Time.** A hook has ten seconds of its own execution per event (waiting on `next` or on API calls
  does not count); `session.end` hooks share 1.5 s. A mod cannot hold a call the way a proxy gate can.
- **Safety.** No sandbox; `$.fs.write` is not atomic; a mod's timer runs between turns. A mod is as
  trusted as the person's own shell.

## What this gives Lys, by place

1. **Policy at the tool seam, in-process.** `tool.call` and `tool.check` do what the PreToolUse hook
   does (the written-first gate on my seat) with a typed input, a result it can replace, and a
   decision it can set, and `plugin.register` lets an organisation mod refuse any later mod. The
   managed `prependPlugins` list is where a Lys policy mod would sit for every Lys-started run. That is
   the "managed settings" slice from today's proxy findings (finding 3), with a stronger hand.
2. **The memory index without touching the system prompt.** Tom's rule is never the system prompt (the
   prompt cache). `prompt.context` is the per-conversation first-message context, and
   `prompt.attachment` is Claude Code's own reminder channel; either carries the generated index
   in-process, and `session.compact` tells a mod when a compaction is about to happen. This is the
   same injection PROXY.md names, done inside the harness instead of on the wire; the proxy's version
   stays for harnesses that are not Claude Code.
3. **Waking a seat.** `$.prompt.submit({ text })` from a timer or an inbound message is exactly the
   wake Brisket's seat lacked today; `session.receive` is where a Cambium bridge would hand a post to
   the model, and `{ consumed }` is where it would keep notifications out of the transcript.
4. **Facts for Geiger.** `session.measure`, `turn.complete`, `session.append` and `tool.call` are the
   events a rule-checker mod would observe (RULES.md): facts, in-process, no scraping of scrollback.
5. **Not for capture.** Nothing of PROXY.md's capture, timing or storage moves into a mod; a mod sees
   text and decisions, the proxy sees bytes and time.

## Decisions for Tom

- Whether a Lys policy mod is written for Claude Code runs (tool seam, memory context, wake), as a
  slice after the proxy's, under the managed-settings line. My recommendation: yes, small, and only
  after the trust and working-folder slices are proven on the installed build.
- Whether to keep the Codex seats on the proxy-only path (no mods there) or wait for Codex's own
  extension surface before a second implementation.

Sources: the mods reference (code.claude.com/docs/en/plugins/mods/reference), the mods API page
(code.claude.com/docs/en/plugins/mods/api), the CHANGELOG entries for 2.1.287 and 2.1.288, and the
original proposal, issue 91870 on anthropics/claude-code (3 September 2026).
