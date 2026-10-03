# The words on the screens, file by file: kept, rewritten or removed

Waffles, 3 October 2026. Item 6 of SCREENS-PLAN.md. The test is Tom's: would his dad, his sister, a
90-year-old be able to do this; and "not plain words, words that say what it's actually doing". Rules for
every sentence: one name per thing; say what happens, not what the system calls it; a reason is always
visible and ends with the button that fixes it; a refusal's code name goes last, small, never first.
This first part covers the start flow, the files Archie and Vesper are in tonight. The rest follows by file
from the walk of all 104 screens.

## One name per thing, everywhere

| Today, in various places | From now on |
|:-|:-|
| registered, "needs to be switched on", "turn this agent on", AgentNotActive | **Not turned on yet** (button: Turn on) |
| active | **Ready** |
| suspended, "access is suspended", "reinstated" | **Paused** (button: Turn back on) |
| retired | **Retired** |
| identity file, file, record | this person's page, this agent's page |
| provisioning, profile, saved settings | **Settings** |
| machine, computer, runs_on | **Computer** |
| runner, Lys runner, runner reports | "Lys on that computer" |
| session, runtime session | "a run" (Running since 14:02) |
| retained request, pending start | never shown; see below |
| admitted, admission, may_run | "allowed to run on" |

## features/runtime/StartAgent.tsx

| Today | Becomes |
|:-|:-|
| Start this agent | **Start** |
| Computer this agent runs on / Choose a computer | Leaves the start. In Settings: "Runs on", with the last computer chosen. |
| This agent cannot start yet. + a closed Details fold | One open sentence and one button, as below. |
| AgentNotActive: turn this agent on before starting it. | "Pancake is not turned on yet." **Turn on** |
| (suspended) | "Pancake is paused." **Turn back on** |
| MachineUnavailable: no computer with a Lys runner admits this agent. | "No computer is allowed to run Pancake yet." **Allow on this computer** |
| (no folder, today refused by the server as WorkingFolderUnnamed) | "Pancake has no folder to work in." **Choose a folder** |
| This start has no confirmed answer. Its original request is retained. Press Start this agent to check that same request. | "Lys did not hear back. Nothing was started twice." **Try again** |
| Lys admitted the start, but no runner ran it. The same request is retained. | "Lys on that computer did not start it." **Try again** |
| Started on X. Its Lys runner has it running. / Open its terminal | The terminal opens in place. No sentence. |
| Not carried into this start: | "Left out of this run:" |
| every "The retained ..." sentence (fourteen of them) | One sentence: "An earlier start was never confirmed. **Check it**", which resolves it and says the outcome. The fourteen cases stay in the code and in the small print. |
| An administrator must save this agent's settings. | "Only the person who runs this Lys can change these settings." |

## features/provisioning/ProfileEditor.tsx (Settings)

| Today | Becomes |
|:-|:-|
| Program this agent uses / Installed copy this agent uses | "Runs with" (Claude Code, Codex), one chooser. The installed copy is chosen for the person and shown small; a second chooser appears only if there are two. |
| Program path / Another program… / Enter the program's full path, starting with / | Removed from the form. Lys lists what is installed. |
| Model this agent uses | "Model" |
| What this agent may do + the program's mode names | "Before it changes things": **Asks first every time** / **Works freely in its own folder, asks before anything else** / **Never asks**. Under each, one line saying what that means. |
| Works in its own folder; internet tools are off. Shell commands can also write to the program's temporary folder. | Under the second choice: "It can read and change files in its folder. It cannot use the internet." |
| Lys makes this agent's own folder when it starts. | No longer true. "Works in" with a folder chooser; empty shows "Choose a folder". |
| System prompt this agent uses / Prompt this agent uses instead / Words added to this agent's prompt | "Instructions": "Add to the usual instructions" (a text box) and, behind "More", "Replace the usual instructions". |
| This Lys is too old to list programs. It gets the list when Lys is updated. | "This Lys needs updating before it can list programs." |
| Save rules for the next start (AgentPolicy) | "Save. Takes effect the next time it starts." |

## features/file/IdentityFile.tsx and AgentOverview.tsx (an agent's page)

| Today | Becomes |
|:-|:-|
| Identity: / Access status: … This says whether its identity may be used; it does not say a process is running. | One line: "Ready · not running" or "Ready · running on Tom's Mac since 14:02" or "Paused". |
| This agent still needs to be switched on before it can start. | "Not turned on yet." **Turn on** |
| This agent's access is suspended. A new start needs it to be reinstated. | "Paused." **Turn back on** |
| Running state is unknown until the runner answers. / No runner has reported a session. Running state is unknown. | "Checking…", then "Not running." or "Lys on Tom's Mac is not answering." |
| Running, as its runner last reported / Stopped, as its runner confirmed / Running state is unconfirmed | "Running" / "Not running" / "Lys on that computer has not answered since 14:02" |
| Start → (link to settings) with "Choose its computer and model, then start this agent." | **Start**, the button itself. |
| Set limits: Set how much this agent may use and when it must stop. | kept |
| Give access: Choose what this agent may reach from access you can give. | "Give access: choose what it can reach." |
| See all runner reports | "Every run" |
| registered → active ⇄ suspended → retired | Removed. The history below it already says what happened and when. |
| Evidence … signed receipt, the log checkpoint and its inclusion proof | Behind "Proof", for whoever audits. |

## features/people/People.tsx and Preview.tsx (today's front page)

| Today | Becomes |
|:-|:-|
| Directory / People and agents / Everyone who works here, human or not. Every agent answers to a person. | The tree on the front page. No heading. |
| Prepare start… | **Start** |
| Open file | "Open" |
| Read runtime reports | Removed; the tree's dot says running. |
| Your own records: you and the agents that answer to you. A directory administrator sees everyone through the directory's own routes. | Removed. |

## features/people/AddAgent.tsx

| Today | Becomes |
|:-|:-|
| Who this agent answers to | kept |
| What this agent may do + "Agent access choices are unavailable until Lys declares which actions are withheld from agents. Nothing is selected." | The same three choices as Settings. The second sentence is a fault in Lys, not a thing to tell a person: it is fixed, not shown. |
| Some actions cannot be selected because the model has no relation carrying that action alone. | Removed; the choices that cannot be made are not offered. |
| Team this agent joins / No team | kept |
| Computer / Computer name / Type a name for this computer. | "Runs on", this computer chosen already. |
| (no folder field) | "Works in": a folder chooser. Required. |
| every "saved request" and "earlier registration" sentence | As for a start: one sentence and **Check it**. |

## Still to list

The other ninety-odd files, in this order because of who meets them first: sign-in and setup; Sessions and
Terminal; Network and adding a computer; Access, grants and roles; secrets; usage and limits; requests and
reviews; apps; settings. Each gets the same table.
