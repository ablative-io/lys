# The Operations canvas: what is still owed

Tom, 4 October 2026, 20:17, by voice: "if we can keep a list of those other things... can we throw the rest
of that canvas stuff in there?... we can do that as we go. That's fine." This is that list. An item leaves
it when it is written, built, installed and walked with pictures. Kept by Waffles.

## The canvas

- A box around each team, drawn for the person. Tom, 5 October, 07:4x: "draw a box around, yeah, each team,
  that's fine. Don't overthink it too much, but yeah, child boxes inside parents, that'd be good." Written
  5 October: one press on the tools bar (shown when a team has an agent running) draws a box named for each
  such team round its card and its agents' windows, a team's own teams inside it, and tidies; "Put it back"
  undoes it; the boxes are ordinary boxes after. An agent in several teams stands in the one furthest down.
  Installed in 0feba1e8 and walked there on 5 October with two teams made for it (Ablative, and Lys under
  it); it leaves the list.
- A box covers what is dropped in it, and is easier to hold. Tom, 5 October, 09:23: "the grouping boxes should
  probably dynamically resize when you drop things inside them, so they cover them... It's difficult to resize
  and move." Written 5 October: when anything is let go, each box grows (it never shrinks) to hold whatever has
  its middle in it, 20 clear at the sides and foot and 40 under its label, smaller boxes first so a box inside
  another carries the outer one with it. A box's whole outline is held: the left edge moves it, the right edge
  sizes its width, the foot its height, and the corner grip is larger. Walked on the developer server with a
  box and a note of my own. An edge that lies under a window cannot be pressed there; the window is in front.
- Widgets for everything Lys holds about an agent that has none yet: tasks, git branch, commits, worktrees,
  pull requests, reviews, jobs and tickets.
- The HUD on an agent's window: compact, set a goal, add a task.
- Layouts kept as project configuration, so a project opens on its own canvas.
- Free terminals on a computer, only on the computers a person has that permission for.
- The Canvas and Proxy swap at the top of Operations lay over a window that stood at the top of the canvas,
  and took the press meant for "Open terminal" and for the first add button; Tidy and home stood the first
  window exactly there. Written 5 October: the top 56 of the page is the swap's, and a view the canvas chooses
  itself (home, Tidy, a double press) keeps what it shows beneath it; a person may still move anything under
  it by hand. Installed in 0feba1e8 and walked there; it leaves the list.
- In a tidied column a chosen widget's remove control lay under the widget above it. Written 5 October: the
  widget chosen, or under the pointer, stands in front of its neighbours. Walked on the developer server.
- A drag begun on the rows of an opened widget moved the canvas, not the widget. Written 5 October: the whole
  card moves it; locked, it stays and the drag is the canvas's as before. Walked on the developer server.
- The terminal's background colour.
- Pinch to zoom by touch.
- What the canvas shows when nothing is running.
- The Agents table and the widgets say some of the same things; one of them goes.
- The You and Dashboard pages.
- "Claude mods": later, and to be talked through with Tom first. Tom, 5 October, about 08:40: "definitely
  think we should talk about Claude mods because there is a way to get Fable usage through that... a mod
  that worked exactly the same way that our hooks had, where it just sort of sent off all of the
  information... look at the slash usage command... the endpoint that that's getting to... we might actually
  be able to pretty fundamentally rewrite some of the aspects of Claude Code... wondering if we couldn't even
  potentially create like a unified harness inside Claude Code and get Claude Code running the codex models
  as well... dial down on one particular harness... that allows any given model." Said back by Waffles as
  ideas, unmeasured: the account's windows already reach the canvas through the proxy; Codex models inside
  Claude Code could be the proxy translating the calls. `docs/design/home/MODS.md` is the page to read first.

## Usage and the proxy

- A sub-agent's calls on the same model as the main thread set the agent's context for a moment. Codex
  marks them (`x-openai-subagent`, `x-codex-parent-thread-id`); Claude Code's mark is not yet found.
- Codex's account-window header names and its response stream are unmeasured until a Codex agent has run
  through an installed Lys (PROXY.md says what was measured).
- A call made on a joined computer is listed, and its body answers `CallKeptElsewhere`: the service reads
  only the proxy home on its own computer.
- A Chat Completions stream reports no usage to the proxy's reader.
- A call the provider refused (Claude Code's start-up probe is refused with HTTP 429 at a limit) carried no
  figures, and one call of unknown spend made the day's and the week's tokens "Not reported", and would
  refuse a start under a token cap. Written 5 October: the usage line carries the provider's HTTP status,
  and a call refused at its head is a spend of nought (installed in 25fe25d2 and walked: the probe of
  08:56 is kept with nought of each). Uses kept before that install stay as they were
  kept: an agent with an earlier refused probe shows its tokens from the next day and the next week.
- Shown on the install of 0feba1e8, 5 October: two calls with tokens on the Proxy screen and the account's
  two windows on the usage widget. A context percent still needs a window declared for the model.
- The proxy's admission write is on the call's path without a disk sync since a4f6eff3: 0.57 ms on the one
  call read. Whether that is "no latency" is Tom's to say; the figure is on every record (`admission_ns`).
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

## Installed and walked: a4f6eff3, 5 October 2026, 00:06

Left the list on this walk: Tidy and "Put it back" (a window moved by hand, tidied, put back, tidied again);
the double press on a window's bar in and out, and on the canvas to the whole; and the four defects below,
written after the walk of 1215fd1e. Tom's words of 4 October 17:01 on the double press, kept: "double
clicking should return you back... to your previous view... until you move again... Maybe double clicking
should just sort of take you back out to... view the whole thing or view the whole group that you're on."

- The service follows a runner's feed and holds its grant channel again when that runner next answers a
  request (`runner_links.rs`). Walked: after the upgrade, with no restart of the service, the first call of
  an agent started from the You page was listed on the Proxy screen.
- The proxy no longer holds a call for a disk sync. Walked: `admission_ns` 574,667 (0.57 ms) on the new
  call; 12,702,417 (12.7 ms) on the call of 4 October.
- The window in front stands over a line's handle. Walked: "add usage" pressed twice beside a window that
  already had one usage widget made three widgets and three lines.
- A service that stops closes the grant channels it holds (`grants_refusals.rs`). Not walked: the installed
  service ends with its process; `budget_feed.rs` holds the test.

Still not shown on an installed build:

- A call with tokens and a context percent. The agent walked with was refused by its provider both times
  (an error body, kept as `unrecorded`), so no call carries use yet, and no window is declared for its model.
- A Codex agent.
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
