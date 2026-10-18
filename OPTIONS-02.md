# Domain 02 (people and agents): options, recommendation, second go

Each item from the Top ten of `lys-review/02-people-agents.md`, in order. "Cherie" is rung 1 (STAKEHOLDERS): first MacBook this weekend, never opens a terminal, never reads "seat" or "grant".

## 1. Nothing links Lys and Cambium

The agent is added, the person lands on its page, and nothing says where it goes next. The real route today (07) is Cambium's Invite panel, which reads the person's own Lys agents; Lys never says so.

| Option | What Cherie would see | Mistake it avoids | Purpose served | |
|---|---|---|---|---|
| A. A "Use {name} in Cambium" step on the agent's Overview, under its run controls: three numbered steps (open Cambium at its address, Invite, choose {name}; give it channels on its Access tab), or, before Cambium is connected, the one step that is missing | The next thing to do, on the page she already lands on, with the Cambium address as a link | "The person is never led to the next step" (merged ranking, shape 2); the dead end after Add (02 #1) | What problem: "I made an agent, now what?" Value: the agent is usable where she works | **Recommended** |
| B. A one-off page between Add and the agent's page: "{name} is added. Next: use it in Cambium" | A page she must click through; on reload or a later visit it is gone | Avoids crowding the Overview, but repeats the "lands somewhere, then a second page" pattern and is lost on reload | Purpose only once | |
| C. Pick or create the agent inside Lys for Cambium (`/apps/me/agents` already serves Cambium) | Nothing new in Lys; Cambium's own Invite panel already does this | Avoids a second copy of Cambium's picker, but changes nothing a person sees in Lys | Not a Lys surface change | |

Recommendation: A. The address is the administrator's view of the app (its sign-in return address); a person who is not the administrator is shown only the app's name and standing (`apps_read.rs:54-64`), so for them the step names Cambium without a link until the server serves its address (ask written in CONSENSUS, server asks).

Done: `src/features/file/CambiumNext.tsx`, shown on an active agent's Overview under its run controls. Test: `tests/cambium-next.test.tsx` (the vitest include does not cover `src/features/file`, so the test sits in `tests/`).

**Second go.** The first version showed the three steps on every active agent for ever, including one already working in Cambium, which turns a next step into wallpaper. Changed: once the agent holds standing access to anything in Cambium, the card folds to one line ("{name} has access in Cambium. To bring it into another channel, press Invite in that channel; change what it may use on its Access tab"). Still wrong and not mine to fix here: step 2 ends at Cambium's take command, which is a terminal command Cherie will never run (07, "the shortest honest journey"); a person who is not the administrator gets no Cambium link because Lys does not serve them its address (server ask written); and Lys cannot tell whether the agent already has a Cambium seat, only whether it holds access there.

## 2. "Add and run" is nine browser calls, and ends not running when no folder was chosen

The review's fix is one server act (`POST /agents` with `run`) and the folder required before the button is on. The server act is Rust, so it is an ask. Requiring the folder I tried first and backed out: on a fresh install the computer does not exist in Lys until the walk records it, and the folder chooser can only browse a recorded computer, so a required folder would make the first agent impossible to add.

| Option | What Cherie would see | Mistake it avoids | Purpose served | |
|---|---|---|---|---|
| A. Require the folder; the button stays off until one is chosen | On a fresh install, a button that never turns on, because the folder cannot be browsed yet | Avoids the false promise, but makes the right thing impossible, the opposite of "easy first" (WHAT-I-THINK 11:0x) | Blocks the purpose | Tried, backed out |
| B. The button says what the press will do: "Add and run" when a folder is chosen; "Add" with one line ("Without a folder, Clover is added and allowed on Ward computer now, and its page asks for the folder before it starts") when not | A button that does what it says, and the reason the run waits | The label that lies (02 #2, "the button labelled Add and run ends on a page that says the agent has no folder"); never a greyed button without its reason | What problem: "I pressed run and it isn't running." Value: trust in the one button | **Recommended** |
| C. Keep the words, explain on the landing page | Still pressed "run" and got "not running" | Nothing; the page it lands on already explains | | |

Done: `src/features/people/AddAgent.tsx` (`runsWith`, the note `.add-agent-later`, the button's word and name). Three tests updated to the honest words; the four availability tests that choose `/srv` first still see "Add and run", so both sides of the rule are asserted.

**Second go.** The words are now true, but the walk underneath is unchanged: without a folder it still sends the start, the real service answers `WorkingFolderUnnamed`, and the form catches that and lands on the agent's page. Nobody sees it, but it is a refused start in the record for every folderless first agent, and the nine-call chain and its session-storage locks are still the browser's. Stopping the walk before the start when there is no folder is the right next change; it moves every folderless test's expected stages (about thirty), so I wrote it into the server ask (one act, which removes the chain altogether) rather than half-do it here.

## 3. The menus contradict each other

The rail's own names (Operations, Network) are domain 01's and already decided in CONSENSUS (Running agents, Computers). What is mine: Operations has a tab also called "Network", and People and agents has a "Found" tab that is about what is running, not about people.

| Option | What Cherie would see | Mistake it avoids | Purpose served | |
|---|---|---|---|---|
| A. Running agents' tree tab is "By computer"; "Found" leaves People and is listed in Running agents › Sessions as "Sessions with no agent"; the People stats link there; the old address redirects | One word per place: Computers is a page, "By computer" is a way of looking at running agents; People is only people, agents and their teams | Two places with one name (02 #3); a tab whose word means something else on another page ("Found" vs "sessions with no agent" vs "Sessions with no seat") | What problem: "where is the thing I clicked last time". Value: the names can be trusted | **Recommended** |
| B. Tab "Seats" (the review's word) | A word she will never read (STAKEHOLDERS rung 1 "never reads seat") | Avoids the clash, but trades it for jargon | | |
| C. Tab "Computers" | The same word as the rail entry beside it, for a different page | Repeats the mistake being fixed | | |

Service accounts belongs in Settings (02 #3), but Settings is not my folder: asked of 01 below, and the tab stays until it has a home, so nothing goes missing.

Done: `SessionCanvas.tsx` (tab word and its nav's name "Running agents views"), `RuntimeSessions.tsx` (a heading "Sessions with no agent"), `People.tsx` (no Found tab; `/people/view/found` goes to `/canvas?view=sessions`), `RuntimeCounts.tsx` (the two stats are links to Running agents).

**Second go.** Running agents › Sessions now stacks three lists: who is running, Seats with its own "Sessions with no seat", and "Sessions with no agent". The last two are different reads (`/seats/unregistered` and `/runtime/found`) with nearly the same name, so a person still cannot tell them apart; they should be one list once the server says whether a session has neither. I did not merge them here because they come from two different services' views and a merged list would have to guess which one is right. The tab bar still says "Proxy" beside the three views, which is a word rung 1 never reads; that one is domain 01/04's.

## 4. Start, Stop and Restart exist four times, each with its own words

| Option | What Cherie would see | Mistake it avoids | Purpose served | |
|---|---|---|---|---|
| A. One `RunConfirm` (the question, the act, "Keep it running") and one `useRunActs` (the two agent requests, the restart's kept operation) in `team/RunConfirm.tsx`, used by the agent's page, the dashboard's Stop and the terminal; one word table `RUN_WORDS` that the seat screens use too | The same question before Stop wherever she presses it: "Stop Clover? This ends its run on Ward computer. What it has not saved is lost. You can start it again afterwards." and the same "Keep it running" | Four vocabularies for one act ("Keep running", "Keep it running", "the machine it runs on"); one place asks first and another stops at once | What problem: "will this lose my work?" answered the same way everywhere. Value: one thing to learn | **Recommended** |
| B. One `RunControls` component for both the agent routes and the seat routes | The same, if it worked; it means writing one component over two route families (`/agents/…` and `/seats/…`) with different receipts and turn rules | Avoids the duplication fully, but is the large half-done change Tom asked us not to make | | |
| C. Words only, four components kept | Same words, four implementations still drifting | Avoids nothing structural | | |

Done: `team/RunConfirm.tsx` (new), `team/AgentRun.tsx` (its own Stop and Restart replaced; `Stop` kept as the export the dashboard imports), `runtime/Terminal.tsx` (its stop, restart and two dialogs replaced), `sessions/Seats.tsx` (word table from `RUN_WORDS`). A refusal now stays inside the question it answers, so the person sees why the stop did not happen where they pressed it.

**Second go.** After the first pass the tree pane under Running agents › By computer still stopped an agent on one press, while every other Stop asked first, so the one rule I was making was broken in the most-used place. Changed: its Stop now asks with the same `RunConfirm`. Still wrong: the Seats table's Stop and Restart do not ask first (they are the troubleshooting table, and asking there means a dialog inside a table cell, which is the phone-width problem of item 10); and the seat routes and the agent routes are still two families, which only the server can make one (the review's "one route family").

## 5. "This computer" means the Lys host, not the device in hand

CONSENSUS: "This computer" always means the device the browser is on; the Lys host is named by its name.

| Option | What Cherie would see | Mistake it avoids | Purpose served | |
|---|---|---|---|---|
| A. Say the computer by its name wherever the screen knows it ("Lys found Claude Code on Front desk", "Allow on Ward computer"); where it does not, "the chosen computer" or "its computer"; the new-computer field is "Name of the computer Lys runs on", with "Clover runs there. It calls itself Tom's MacBook Pro; change the name if you like." | A name she recognises, or a phrase that is true from any device | "This computer" meaning the server on three forms (merged ranking, shape 1); the phone user told "on this computer" about a Mac across town | What problem: "which computer?" Value: the page is true from a phone | **Recommended** |
| B. "the server" | A word rung 1 does not use | Avoids the false "this", adds jargon | | |
| C. Leave "this computer" and add a note that it means the Lys host | Two meanings for one phrase, with a footnote | Nothing | | |

The review also says "never default a new computer's name to the server's". I did not apply that part: on the Add form the new computer *is* the Lys host (the walk records the host's own runner), so its own name is the right default. What was wrong was calling it "this computer".

Done: `provisioning/ProfileEditor.tsx` (`whereName`, `onWhere`, a `computerName` prop for a computer not yet added), `people/AddAgent.tsx` (label, hint, reason line, passes the name), `runtime/CannotStart.tsx` (Allow names the computer), `people/Words.tsx`, `people/add-and-run.ts`, `provisioning/ModeWords.tsx`, `provisioning/McpServers.tsx`, `team/Start.tsx`, `sessions/Owned.tsx`. Tests updated to the new sentences, including three in domain 04's files that assert sentences from `people/Words.tsx`.

**Second go.** The refusal words in `people/Words.tsx` now say "the chosen computer" because the table is keyed by refusal code and has no name to hand; that is true but vague, and the next change is for the callers that know the computer (the Add form, Start, Settings) to pass its name into the words. The help text that says "starts it at once on this computer" (`src/shell/help/pages.ts:113`) is domain 01's and is asked below. The tree's node title "no seat on this computer" I left: there "this" points at the computer row it is on, which is true from any device.

## 6. One state has three names

| Option | What Cherie would see | Mistake it avoids | Purpose served | |
|---|---|---|---|---|
| A. The plain words everywhere, from the one table in `people/Words.tsx`: states Not turned on yet, Active, Paused, Retired; acts Turn on, Pause, Turn back on, Retire (the full form says "Retire for good"); the emergency stop is "Pause now" | The same word in the list, on the file's head, in the reason an agent will not start, and in its record ("Paused: active → paused") | Three vocabularies for one state (02 #6); IAM words rung 1 never reads ("Access suspended", "Restore access", "reinstated") | What problem: "is my helper on?" Value: one word to learn | **Recommended** |
| B. The directory's words everywhere (Awaiting activation, Access suspended, Permanently retired, Restore access) | IAM words on her helper | Avoids the split, keeps the jargon | | |
| C. Two tables, one for people and one for agents | Two names again, now by design | Repeats the mistake | | |

Done: `people/Words.tsx` (`STATUS`, `ACTION`, `ACTION_SHORT`, the `AgentNotActive` words), `runtime/CannotStart.tsx` (reads them instead of its own), `file/IdentityFile.tsx` (the head's line, the menu's "Pause now", the form's descriptions, the result line), `file/EmergencyStop.tsx`, `file/receipt-words.ts` (record lines), `people/AddAgent.tsx` ("Show retired and paused"). Tests moved to the new words.

**Second go.** "Turn on" on a person's file reads a little like a machine; I kept it, because the head already says "Not turned on yet" beside it and one word for both kinds is the point. Seats' "Online, idle / Offline / Not seen" are left as they are: they say whether a run is live, which is a different question from whether the agent is allowed to work, and folding the two together would make one word mean two things. Help still says "awaiting activation, suspended" (`src/shell/help/pages.ts:71`, `concepts/trouble.ts:51`), asked of 01. `src/features/me/You.tsx:32` has its own table for service accounts ("Awaiting activation", "Registered" for active), which is domain 05's.

## 7. "Add person" is shown to everyone, and leads a non-administrator to a dead end

| Option | What a rung-2 helper or a contractor would see | Mistake it avoids | Purpose served | |
|---|---|---|---|---|
| A. Offer Add person only to the administrator (the directory page already knows the scope, so it is drawn with the page, not popped in); the address `#/people/new` reached any other way says why it is not open and what they can do instead, with "Add an agent" as the link | Only the acts that work for them; never a button that leads to "no" | A dead-end button (02 #7); "every error names the cause and the next act"; "every form ends with the next step as a link" | What problem: "can I add my colleague?" Value: no wasted press | **Recommended** |
| B. Draw it disabled with the reason beside it | A greyed button on every visit, for an act most of them will never have | Avoids hiding, but puts the administrator's job in front of everyone (STAKEHOLDERS: "nothing else visible" for a contractor) | | |
| C. Keep it, improve only the page it leads to | The same dead end, said better | Half the mistake | | |

Done: `people/People.tsx` (`PeopleHead` takes `admin`, or reads the scope itself on the Teams and Service accounts views), `people/AddPerson.tsx` (the cause, the next act and a link). Test: `tests/people-add-person.test.tsx`, both sides (offered to the administrator on two views, not offered to Bea, and Bea's page says why with the link).

**Second go.** On the Teams and Service accounts views the head reads the scope itself, so for the administrator "Add person" appears a moment after "Add agent"; that is the pop-in the rail decision warns against, small here but real. The cure is for the page to read the scope once above all its views; that is a restructure of `People.tsx` I would rather do with item 3's move of Service accounts, when that view leaves. "Send the administrator their name and email" still does not say who the administrator is; Lys does not serve that name to a person, so it is a server ask if it matters.

## 8. "Every agent answers to a person", yet agents can answer to agents, and the column shows the owner

The server already serves each agent's `reports_to` (`AgentSummary` in `read_api.rs:121`); the surface's generated type drops it.

| Option | What a rung-2 owner would see | Mistake it avoids | Purpose served | |
|---|---|---|---|---|
| A. "Answers to" shows the agent it reports to when that is an agent, with the person beside it: "Scribe (agent), for Ada"; a person-reporting agent still shows its person; the subtitle says what is always true: "Every agent has a person who answers for it." The agent's own page says the same, with a link | The chain she set up, and the person who is accountable, in one cell | A page sentence that the form beside it contradicts (02 #8); a column whose name says one thing and whose cell shows another | What problem: "who is this helper working for?" Value: accountability she can read | **Recommended** |
| B. Two columns, "Reports to" and "Answers for it" | Two columns, most rows identical | Avoids the contradiction, costs width on a page already crowded on a phone (item 10) | | |
| C. Change only the subtitle | A true sentence above a column that still hides the chain | Half the fix | | |

Done: `people/directory.ts` (`reportsTo`, read from the served edge, an agent only), `people/People.tsx` (the column, the subtitle, and its two inline warning colours moved to the new `people/people.css`), `file/AgentOverview.tsx` (the same on the agent's page). Test: `tests/people-answers-to.test.tsx` (an agent-reporting agent and a person-reporting one, in the list and on the page). While here: `runtime/NetworkTree.test.tsx` had a 50 ms bound that failed under load; it now counts React commits instead (one, the mount), since the repository allows no time limits in a test.

**Second go.** The first pass fixed only the list; the agent's own page still said "Answers to: Ada" for an agent that reports to Scribe, which is the same contradiction one click later, so it now says the same as the list. Still to do, outside my folders: `src/generated/index.ts` `AgentSummary` and `AgentView` should carry `reports_to` (asked below) so the surface stops reading it defensively.

## 9. `team/` is the agent's run controls; `teams/` is organisational teams

| Option | What a person would see | Mistake it avoids | Purpose served | |
|---|---|---|---|---|
| A. Move `team/` to `run/` (AgentRun, Start, RunConfirm, `run.css`), fix the stale comments; leave three one-line re-exports in `team/` for the dashboard's imports until domain 06 moves them, then the folder goes | Nothing changes on screen; the next builder finds the run controls under a name that says what they are | Two folders one letter apart for unrelated jobs (02 #9); stale comments that describe a terminal multiplexer and the wrong page | Purpose: the "one run-controls component used from one place" of CONSENSUS has an honest home | **Recommended** |
| B. Merge into `runtime/` | Same, but `runtime/` is already 40 files of canvas, terminal and tree | Avoids a new folder, makes the biggest folder bigger | | |
| C. Leave the folder, fix only the comments | The same trap for the next reader | | | |

Done: `git mv` of the four files; imports updated in `file/`, `provisioning/`, `runtime/`, `sessions/` and two tests; the stale comments in `run.css` (was `team.css:1,10`) and `runtime/NetworkAgentPane.tsx:3` (it named `/agents/{id}/start`; Start sends `/start-command`) corrected. `team/AgentRun.tsx`, `team/Start.tsx` and `team/team.css` are one-line re-exports, each saying why it is there.

**Second go.** The class names are still `team-start`, `team-stop`, `team-start-line`, and the dashboard's own CSS and markup use them (`dashboard/dashboard.css:101`, `dashboard/AgentTree.tsx:135`), so renaming them to `run-*` here would quietly unstyle domain 06's rows. That rename belongs with 06's import change; asked below as one change.

## 10. Phone width breaks or is unsupported

| Option | What Cherie would see on her phone | Mistake it avoids | Purpose served | |
|---|---|---|---|---|
| A. The small, certain fixes in the run screens: the `.network-pane` rule that cancelled its own phone padding is merged above the media query; the terminal gets the line keys a phone keyboard cannot send (Left, Right, Line start, Line end, named as the runner publishes them); every run control is a 44 px target under 600 px; the Seats table becomes one card per seat; message boxes are 16 px so the phone does not zoom; dead terminal CSS removed | A terminal she can edit a line in with her thumb; buttons she can hit; no table cut off at the right | "Phones: 44 px touch targets, 16 px inputs; tables become cards under 600 px" (CONSENSUS); the cascade bug (02 #10) | What problem: "I'm away from the Mac and it needs a nudge." Value: Lys works from the device in hand | **Recommended** |
| B. Touch select mode and pointer-event pinch on the canvas as well | The same, plus a canvas usable by touch | Fixes the canvas too, but `CanvasSurface.tsx` is the largest file here and the canvas is Tom's Tuesday decision (STAKEHOLDERS, "Terminal canvas"); half-done there is worse than not started | | |
| C. Hide the canvas below 600 px and open Running agents on By computer | A phone that never shows the canvas | Avoids the canvas's touch problems by not offering it; it decides Tom's open question for him | | |

Done: `runtime/network-tree.css` (the cascade fix, 16 px input, 44 px acts), `runtime/terminal.css` (keys wrap, 44 px under 600 px, the dead `.runner-sessions` and `.terminal-open` rules removed), `runtime/Terminal.tsx` (four line keys), `run/run.css` (44 px acts, the question wraps), `sessions/Seats.tsx` with the new `sessions/seats.css` (cards under 600 px, each cell labelled). The keys test lists each key by the name the runner publishes.

**Second go.** The first pass stopped at the cascade bug and the keys; the Seats table, six columns with a send box in the last cell, was still the worst thing on a phone in my folders, so it became cards in the second pass. Still not done: the canvas (multi-select needs Meta, Ctrl or Shift; pinch is wheel or Safari gestures only), left for Tom's decision on the canvas; the Mac-only Command and Option editing keys in `GpuTerminal.tsx` stay as they are (the on-screen keys are the phone's way in). I have not seen any of this on a phone: these are CSS rules checked by reading the cascade, not by a device.

