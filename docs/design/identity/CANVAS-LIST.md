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

## Tom's walk of People and agents, 5 October, 16:35 to 16:40, by voice

His words, in the order said, on a person's page (he had Waffles' open):

- "the buttons up in the top right hand side. We've got edit name, which is a button, which I don't think needs
  to be a button at all. That should just be in the settings, the overview kind of thing for editing their
  names. We can probably get rid of that."
- "it's got three buttons that say suspend access, retire permanently or emergency stop. I think the suspend and
  retire button should be under maybe the drop down and the emergency stop button should be just like a stop
  symbol button and... probably to the left of the active badge."
- Buttons everywhere: "where... we've used like text label buttons. I really don't like big text label buttons.
  They look really stupid. I'd really like you to find symbolic buttons with like, you know, labels beside...
  Even things like... add this limit and set goal. Like you're in the [goal] thing, you could probably just have
  a button that just says set rather than set goal... a plus button for the add this limit kind of thing...
  suspend access and retire permanently. Like you could probably just have suspend and retire."
- The budget screen: "I don't think there's a great use of space."
- "there's a thing where something's duplicated, so it says like there's no dollar spent has been reported for
  Waffles the Terrible."
- "walk it in the browser, have a look, see what else you're seeing."
- The page's head, 16:40: "it's got like people and agents slash Waffles the Terrible and then underneath that
  it says Waffles the Terrible... and then added 29th of September... it's just using up way too much vertical
  space before you actually get to the useful stuff. I'd probably just have Waffles the Terrible... We've got
  people and agents selected in the sidebar, so we don't really need to have that. We certainly don't need
  Waffles the Terrible a second time. Get rid of all the unnecessary buttons... the header really shouldn't
  need to be that big before you get to the tabs... the tabs and the active slash suspend retire... drop down
  hamburger kind of thing and the emergency stop button, then just with the name. And I don't think we really
  needed like added 29th of September... You can just go in the overview."
- The system prompt, 16:41: "the options are to keep this agent's own prompt or add to it. Those aren't mutually
  exclusive options. You can keep it unchanged. You can... add to it... you can replace it altogether. And yeah,
  I really suspect you should have a bit of a better look at all of this stuff."
- Settings, 16:42: "a few things are a bit cramped up and not aligned properly, like the rules part. And you're
  saying connected tools this agent can use... if we're just talking about MCP servers... just call them MCP
  servers. If we're talking about more than that... we should just break things out into separate sort of
  things. So tools, plugins... MCP servers, plugins, skills."
- The working folder, 16:43: "The Choose folder is pretty stupid... it's not giving you... a place to go. Like you
  can't type in a path or select... a path for anything."
- Every button, 16:43: "I want you to do an audit for all of these button labels. Like you're on the settings
  page and... the label is like save these settings. Like I just want you to go through and just like get rid
  of everything that's written in words, replace it with a symbol where we can... [a full audit of] that, cause
  it's really, really pretty shit. Like throughout the entire site, like those kinds of buttons are really kind
  of dumb."

Seen by Waffles on the walk he asked for (installed a1678ae6, pictures w40, w41), beside his own:

- The head stands about 180 points tall before the tabs: the path line, the name, "Added 29 Sep.", then four
  word buttons and the badge on a row of their own height.
- Overview says the agent's whole name twice in two lines ("Waffles the Terrible is not running." "Waffles the
  Terrible has no program chosen yet.") above a large word button, "Choose its program".
- Overview, Computer and Model: "Choose a computer when you start" with "Saved choice for the next start."
  under it, which says a choice is saved when none is.
- Limits and goals: the two lines he saw are "No dollar spend has been reported for Waffles the Terrible." and
  "No plan window has been reported for Waffles the Terrible.", under a banner that already says no usage has
  been reported; three sentences for one fact. The add-a-limit row lays nine choices and four actions out in a
  table of their own with "Add this limit" at its end.

## The canvas as wiring (Tom, the same walk)

- An emergency stop widget: "that emergency stop would be a good widget to have... a really common sense kind of
  widget... here's... an emergency stop button and I've wired it up to these things. So if I need to, I can just
  hit the emergency stop on there."
- "I wonder even if maybe we could sort of think about sort of like the wiring diagram that the operations
  canvas becomes... it could almost be like a permissions thing in itself really... These things can go there,
  can't go here."
- "having things like a shared prompt field where you can send stuff in."
- Flows: "it's not [Lys's] primary purpose, but... pretty cool for it nonetheless... running commands in little
  computational flows, like the output of this one runs to the input of this one... a templating kind of thing
  where we could wire variables from one thing to the next... flows from one terminal process feeding... back
  out into other little data processing widgets or templating widgets and then back out into a new terminal
  would be really interesting... it would give people another reason to use it in their day to day."

None of this section is written yet.

## Sign in with ChatGPT (Tom, 5 October, 16:50, by voice)

"one of the things during OpenAI's Dev Day... they announced... a sign in with ChatGPT option, which is like...
sign in with Google, sign in with GitHub... presumably over the next year or so, there'll be a sign in with
Anthropic as well... if you use AI services in your site... you can offer users the option to sign in with
ChatGPT and then [it] draws from their inference budget so they can use your service with their inference...
I wonder if we can get that into our [Rauthy fork]... so you can sign in with your various different ChatGPT
accounts to use the subscription budget. Cause like I've got for instance, three at the moment that would be
great to rotate through."

Not read by Waffles: what OpenAI announced and what it offers a relying service. To read on OpenAI's own
developer pages before anything is said about fitting it to the issuer inside Lys. It sits beside "The proxy
swaps the account" below: several registered accounts, rotated at the proxy, each call attributed.

Tom, a few minutes later (about 16:55, by voice): "we can do the one-time password kind of sign-in... to get the
authentication tokens that... Codex already uses so we don't need to over-complicate things. So yeah, that flow
is already sort of available to us." So the first way in is the sign-in Codex already has, giving the tokens
Codex already uses; nothing new is needed in the issuer for it. Not read by Waffles: how Codex's sign-in hands
its tokens over and where it keeps them.

## The Hub

- A third view beside Canvas and Proxy. Tom, 5 October, 16:20, by voice: "it just makes me think that we could
  maybe have a third thing in here. So currently we've got... Canvas and Proxy. Perhaps we could have Canvas,
  hub and proxy and the hub could be sort of like just a swap between the chats for each one kind of thing...
  more nicely, you know, like just rendered sort of view of the chats that you got running... Sort of like the
  proxy is kind of like the raw view. And this would be like just the... streaming, nice looking view. Would be
  handy, I reckon." Said back by Waffles, unmeasured: the proxy already keeps each call's messages and its
  response stream per run, so the Hub reads the same records and draws them as a conversation, live, with a
  switcher between the agents running. Not written.

## Usage and the proxy

- Header values. The proxy kept every header's name and the values of a short list. Tom, 5 October, 16:23, by
  voice, reading a call on the Proxy screen: "there's all these values not kept... there were a fair few things
  that I would have thought would be worth keeping that we aren't keeping there. You know, like runtime,
  runtime version... Package version, OS, language, architecture... Claude Code Session ID, like all of these
  things are incredibly valuable." Written 5 October, not yet built: no list chooses; every header's value is
  kept unless the header carries a credential, by the list or by how it is named (`proxy/headers.rs`).
- The proxy swaps the account. Tom, the same minute: "Things like authorization, it'd be great if we could sort
  of swap out the authorization so we could rotate through our tokens without having to restart sessions... And
  you can see... what requests were attributed to which account." Not written. Said back by Waffles, unmeasured:
  today a run that reaches an account's window is ended and started again on the next account; the proxy sits
  on every call and already knows the run, so it could put the next registered account's credential on the call
  itself, as a gate, and write which account each call drew from on its record. To design against PROXY.md
  (a gate holds the call; the credential comes from the broker, never from a file or the record).

- A sub-agent's calls on the same model as the main thread set the agent's context for a moment. Codex
  marks them (`x-openai-subagent`, `x-codex-parent-thread-id`); Claude Code's mark is not yet found.
- Codex's account-window header names and its response stream are unmeasured until a Codex agent has run
  through an installed Lys (PROXY.md says what was measured).
- A call made on a joined computer is listed, and its body answers `CallKeptElsewhere`: the service reads
  only the proxy home on its own computer.
- A Chat Completions stream reports no usage to the proxy's reader.
- A call the provider refused (Claude Code's start-up probe, a one-token message, is answered HTTP 429
  `rate_limit_error` each time an agent starts) carries no figures, and one call of unknown spend made the
  day's and the week's tokens "Not reported". On 5 October I made such a call count as nought (installed in
  25fe25d2). Tom, 10:08 the same day: "No, calls counts as the tokens that they've made. Like don't make any
  assumptions ever... we get the reporting back, so why assume?" Written after: the nought is taken out; a
  call that reported nothing is unknown again, its record naming the HTTP status; and a period with such calls
  says what was reported and how many reported none ("1,180 tokens reported, 1 call reported none"), never a
  total. A token cap over such a period is still unjudged, as before 25fe25d2; whether a refused probe should
  hold a start under a cap is Tom's to say. Why the provider refuses the probe is not known; to compare with
  a start outside Lys.
- The Proxy screen. Tom, 5 October 10:10 and 10:13: "A lot of things are coming back as unrecorded, or
  something like that... it would be good if we could get syntax highlighting on the JSON... a few things
  that we could do to make the proxy slightly nicer, easier to navigate, like, you know, see the input, see
  the output." Written 5 October: a call opens on Read, what went in (settings, system prompt, tools by name,
  each turn folded, the last open) and what came out (words, tools asked for, why it stopped, or the
  provider's error by name); the JSON is in colour; the list says "No complete response" where it said
  "unrecorded". Still owed: the HTTP status on the list's row, which the service does not hand the list yet.
- Shown on the install of 0feba1e8, 5 October: two calls with tokens on the Proxy screen and the account's
  two windows on the usage widget. A context percent still needs a window declared for the model.
- The proxy's admission write is on the call's path without a disk sync since a4f6eff3: 0.57 ms on the one
  call read. Whether that is "no latency" is Tom's to say; the figure is on every record (`admission_ns`).
- Dollars and running time. Tom, 4 October 20:20: "The cost in US dollars is definitely there... I'm pretty
  sure it's available through the status line alone." It is: Claude Code's status line reports the cost in US
  dollars and the running time, and Lys has the adapter that reads it. A run counted through the proxy starts
  with that adapter off (the start gives the runner one way of tracking, not both). Written 5 October, not yet
  built or installed: a Claude Code run counted through the proxy is given a status line in the settings file
  Lys already hands it (`lys runner status-line`, reaching the runner through two variables the runner sets,
  `LYS_PROGRAM` and `LYS_RUNNER_SOCKET`); the runner keeps the dollars, as what was added since the last
  report, and the running time, and writes them with the run's next calls and when it ends. Tokens and context
  are still counted only from the calls. The run's status line says "Lys: counted", or that the figures were
  not counted and why. A run handed to another runner keeps the first runner's socket in its environment
  until it is started again; not yet looked at. Codex's own figure is still to be found.
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
