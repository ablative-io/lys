# The screens do not let a person start an agent: what is wrong and the plan of action

Waffles, 3 October 2026, 20:20, after Tom tried the installed build (33280af9) and could not start an agent
(Dot 20:02 to 20:08): "there's no way from my screen to start it"; "that button's greyed out"; "I said the
landing page. You should have access to all the things that you need. So that's the front page"; "what this
agent may do ... is idiotic"; "it says an agent needs to be turned on before you can start it ... I don't
know what that means"; "a lot of these things I specifically asked for"; "I've already paid for this and it
has not been done and you have told me many times that it is being done."

He is right on every count. I read every screen: 104 files under surface/identity/src, about 1,250 visible
sentences and every link, kept by file in my scratchpad as screen-walk.txt. This page is what that read
found, against what he asked for on 30 September, and what we do.

## What he asked for and what is on main

On 30 September, 18:39 to 19:12, Tom ruled the team screen: names in a tree down the left (you, your teams,
your agents, a dot when one is running), the terminal filling the rest, a stopped agent showing Start in
place, "where it runs is a setting, not a start step", settings in a drawer, no headers, nothing shown
twice, real data. And for every form: no free text where Lys knows the choices, labels that say what the
thing actually does, and the test "would a 90-year-old woman, my sister, my dad be able to do this".

That screen was built (features/team, 437 lines with its tests) and deleted from main the same night:
commit bb630ab0, 30 September 19:53, "Delete the team page; a different approach follows". What followed
was an agent canvas and a save, review, start sequence on the settings page. Nobody since has put a start
in front of a person, and nothing in any battery or readback tries one: mine tonight used a token and a
script, so I reported "installed and read back clean" on a build its owner cannot use. That is the failure
in how I check, and item 7 fixes it.

## What a person meets today, in order

1. **The front page is the People list.** It has no start. Its only way toward one is a link called
   "Prepare start…".
2. **That link, the agent page's "Start" and the address /start all land on the settings tab.** The start
   button is at the bottom, under every setting (ProfileEditor.tsx renders the fields, then the start). The
   code's own note says an agent has one start "first on its settings page". It is last.
3. **The button is grey and the reason is folded away.** An agent that is not active gets "This agent
   cannot start yet." and a closed "Details" fold holding "AgentNotActive: turn this agent on before
   starting it." Nothing there turns it on. The act lives on another page (Manage directory, Change), under
   the words "reinstate" and "status". The page's header says "still needs to be switched on" and "access
   is suspended. A new start needs it to be reinstated", three names for one thing and no button beside
   any of them.
4. **Today's working-folder rule is on no screen.** The server now refuses a start when no folder is named.
   The words "working folder" appear nowhere in the front end or its generated client. A person cannot
   name a folder, so any agent they add is refused, and the form still says "Lys makes this agent's own
   folder when it starts", which stopped being true today.
5. **"What this agent may do" is the program's permission mode with a new label.** "Reads freely and asks
   before most changes and commands" describes a flag of Claude Code, not what a person is deciding.
6. **The words fail the test throughout.** "Retained request", "admitted", "runner reports", "provisioning",
   "identity file", "Access status ... does not say a process is running", "registered → active ⇄
   suspended → retired", refusal codes such as "MachineUnavailable:" at the front of sentences, and reasons
   hidden in "Details" folds on most forms. The dock offers "Assistant, not built yet".

## The plan of action

1. **The front page is the team screen, and it holds everything a person needs.** The tree of names on the
   left; the chosen agent on the right. Running: its terminal. Stopped: one Start button, there. Add an
   agent or a person from the tree. Settings, limits and access open as a drawer over it. People, roles,
   access and the rest stay reachable from the menu, and none of them is needed to start work. We begin
   from the deleted screen (git show bb630ab0^), not from nothing.
2. **Start is one press.** Where it runs is a saved setting with the last computer already chosen. The
   save, review, start sequence stays in the code as what makes a start safe to retry, and leaves the
   screen: a person sees Starting, then the terminal, or one sentence saying what went wrong and a
   Try again that sends the same request.
3. **When it cannot start, the screen says why in one sentence and offers the one button that fixes it,
   right there.** Not turned on: "Turn on". Paused: "Turn back on". No folder: a folder chooser. No
   computer allows it: "Allow on this computer". Retired says so and offers nothing. One word per state
   everywhere: Ready, Paused, Retired.
4. **The working folder is on the screen today's rule needs it on:** when adding an agent and in its
   settings, chosen with a folder chooser under the person's Developer folder, shown on the front page as
   "Works in". The generated client is rebuilt from the server so the field exists.
5. **"What this agent may do" becomes three choices that say what happens:** it asks before changing
   anything; it changes files in its own folder without asking and asks before anything else; it does
   anything without asking. Each maps to the program's own mode underneath.
6. **Every reason is shown, never folded, and every sentence is rewritten against the test.** One pass
   over all 1,250 sentences, by file, each kept, rewritten or removed. Refusal names move to the end of
   the sentence in small type for whoever debugs. "Not built yet" items come off the screen.
7. **No screen work is called done until a person's walk passes on the installed build:** sign in, land,
   add an agent with a folder, start it, type in its terminal, stop it, start it again, with a screenshot
   of each step posted. The walk is done by a seat that did not write the change, in a real browser, and
   it becomes a leg of the battery. A readback by token and script no longer counts for a screen.

## Who, and in what order

- **Archie:** items 1 and 2, from the deleted team screen, against today's start and restart routes.
- **Vesper:** items 3, 4 and 5, and the server's client regenerated for item 4.
- **Waffles:** item 6's list, file by file, handed to both as they go; and item 7, the walk, myself.
- Everything is written and read before anything is built; then one battery, one install, then the walk.
  The walk's screenshots go to Tom before any word that it works.

## What I have changed tonight, so it is known

I turned the test agent pancake back on at 20:07 (it was paused because both readbacks tonight stop it when
they finish), so its start button is live. Nothing else on the installed build has been touched.

If these screens were used in a hospital, the worst credible failure is a person who cannot start or stop
the agent they are responsible for because the control is hidden or refuses without saying why; about 10
to 100 people.
