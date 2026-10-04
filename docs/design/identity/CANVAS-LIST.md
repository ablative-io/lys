# The Operations canvas: what is still owed

Tom, 4 October 2026, 20:17, by voice: "if we can keep a list of those other things... can we throw the rest
of that canvas stuff in there?... we can do that as we go. That's fine." This is that list. An item leaves
it when it is written, built, installed and walked with pictures. Kept by Waffles.

## The canvas

- Tidy: lay the windows and widgets out automatically. Written 4 October, 21:11, and walked on the developer
  server (one press from the tools bar; "Put it back" beside it until the next change); leaves this list when
  it is installed and walked there.
- A box around each team, drawn for the person.
- Widgets for everything Lys holds about an agent that has none yet: tasks, git branch, commits, worktrees,
  pull requests, reviews, jobs and tickets.
- The HUD on an agent's window: compact, set a goal, add a task.
- Layouts kept as project configuration, so a project opens on its own canvas.
- Free terminals on a computer, only on the computers a person has that permission for.
- Double-click out to group. Tom, 4 October 17:01: "double clicking should return you back... to your previous
  view... until you move again... Maybe double clicking should just sort of take you back out to... view the
  whole thing or view the whole group that you're on." Written 21:18 as: a double press on a window's bar or a
  box's bar brings the view in to it and the same double press goes back, until the view is moved; a double
  press on the canvas itself brings everything into view. Walked on the developer server; leaves this list when
  installed and walked there.
- The Canvas and Proxy swap at the top of Operations lies over a window that stands at the top of the canvas.
- The terminal's background colour.
- Pinch to zoom by touch.
- What the canvas shows when nothing is running.
- The Agents table and the widgets say some of the same things; one of them goes.
- The You and Dashboard pages.
- "Claude mods": later.

## Usage and the proxy

- A sub-agent's calls on the same model as the main thread set the agent's context for a moment. Codex
  marks them (`x-openai-subagent`, `x-codex-parent-thread-id`); Claude Code's mark is not yet found.
- Codex's account-window header names and its response stream are unmeasured until a Codex agent has run
  through an installed Lys (PROXY.md says what was measured).
- A call made on a joined computer is listed, and its body answers `CallKeptElsewhere`: the service reads
  only the proxy home on its own computer.
- A Chat Completions stream reports no usage to the proxy's reader.
- The proxy's admission write is on the call's path; its cost (`admission_ns`) is on every record and has
  not been read against Tom's "no latency".
- Dollars and running time. Tom, 4 October 20:20: "The cost in US dollars is definitely there... I'm pretty
  sure it's available through the status line alone." It is: Claude Code's status line reports the cost in US
  dollars and the running time, and Lys has the adapter that reads it. A run counted through the proxy starts
  with that adapter off (the start gives the runner one way of tracking, not both). Next round: both together,
  the proxy for calls and the status line for dollars and time. Codex's own figure is still to be found.
- Claude Code mods are not needed for this. Tom, 4 October 20:25: "if it's just those things, they're also
  available in the status line... we don't need to dive into mods unless there's anything extra available."
  What a mod would add beyond the status line is turn boundaries and sub-agent spawns as events; neither is
  asked for.
- Any local program can send a run key, or none.
- Model accounts end to end: who draws from which account, registered by a person.

## Found on the walk of 1215fd1e, 4 October 2026, and written for the next install

- The service followed a runner's feed, and held its grant channel, only from its own start. An upgrade
  starts the service before the runner, so after every upgrade no use reached the service until it was
  started again. Written: each link is begun again when that runner next answers a request
  (`runner_links.rs`); a link that ended on a refusal stays ended, and every end is said.
- The proxy held each call for two disk syncs, about 12 ms each: at admission, and when the request named
  its session. Written: the call waits only for the record to be put in place; the syncs are made beside it.
- A short line's own handle lay over the first add button beside a window, so pressing "add usage" a
  second time took the first widget's line away. Written: the window in front stands over the handles.
- Not yet shown on an installed build: a call with tokens and a context percent (pancake's account was
  refused by its provider, HTTP 429, when walked), and a Codex agent.
- Six machine records on Tom's Mac name the one runner socket; five follows end, as designed, on a use
  that names a session of another machine. Clutter from earlier joins, to be cleared with Tom.

## The runner

- A runner restarted while it drains a run's usage.
- The drain has no test of its own.
- A settling stop that reaches a runner before the person's own first stop makes that first stop kill.
- The console stop route, and the broker's console actor.
- Session handover.
- The audit, intent first.

## Other products through Lys

Tom, 4 October 20:26: "what else winds up needing to go through Lys... Apollo will probably need some help
getting haematite's permissions and sign in. We also need to look at Cambium's permissions and sign in."

- Haematite. Talked through with Apollo in the Lys room the same evening; his design page comes before any
  code. Lys's two pieces: a claim naming the responsible person on an agent's token, and a read that gives
  everything one subject holds on one app, with the grant log's position as its revision, so the service
  evaluates rows itself and asks again when `/changes` answers. Rule both sides hold: Lys opens its own
  store through the library in its own process, never through `haem serve`.
- The embedding and query workers are children of the haematite service and hold no credential of their
  own (Apollo, 20:34); nothing of theirs goes through Lys.
- Cambium's sign-in and permissions: not yet looked at.
