# Lys in ten minutes: sign in and launch your agent

Use [Lys](https://lys.ablative.com.au) for your account, computers and agents.
The server decides which agent may start. Lys on your computer runs it and connects your terminal.

An administrator must first add you in **People and agents → Add person**. If you need Cambium,
the grant is `member` on `cambium.workspace:ablative`; `reader` permits reading only.
Complete your own browser sign-in. Nobody needs to send you a token or certificate.

## 1. Install the two device clients

In Lys, open **Network**, enter your computer's name in the add row and choose **Another computer**.
The **Connect** row offers **Download lys for this Mac** and **Download its companion**.
Choose Apple silicon or Intel to match your Mac. Download both files into Downloads, then run:

```sh
mkdir -p "$HOME/.local/bin" && install -m 755 "$HOME/Downloads/lys" "$HOME/Downloads/lys-containment-entry" "$HOME/.local/bin/"
export PATH="$HOME/.local/bin:$PATH"
lys --version
```

Keep both executables together. The page names their source commit. If it says Lys holds no client
for your Mac, the operator must supply that platform's release before you can continue.
Claude Code or Codex must also be installed here. The agent's settings declare its program and
build. Lys checks that declaration when starting it.

## 2. Sign in from this terminal

```sh
lys sign-in --server https://lys.ablative.com.au/api
```

Finish the sign-in and approval in the browser this command opens. Lys keeps your terminal's pass
in an owner-only file on this computer. Do not open, copy or paste that file.

The exact server address is saved with the sign-in. Later launch commands use that address.
You can name it explicitly with `--server`; Lys never invents a missing server address.
Signing in authorizes your terminal. Connecting the computer is the next step.

## 3. Connect this computer once

Return to the computer's **Connect** row. Copy its command and run it in a second terminal.
It has this form; use the actual computer id and full server address from the row:

```sh
lys runner join --server https://lys.ablative.com.au/api --machine <computer-id>
```

When asked, paste the one-use connection code into the prompt. Keep the code out of command
arguments and shell history. Successful join names the computer, then says its runner is listening.
Leave this terminal running while you use your agents. Control-C in this terminal ends its runner
and every session it holds. To connect again later, ask for **New code** on the computer's page.

Sign-in and join each belong to this computer. Do both on another computer rather than copying
the first computer's files.

## 4. Choose your agent

```sh
lys agents
```

The list contains your agents allowed on this computer, with their harness, identity state and
running state. An unknown running state means the runner did not answer; its reason is shown.
It does not mean the agent stopped.

If there is no Claude Code agent, open [Add an agent](https://lys.ablative.com.au/#/agents/new).
Choose yourself as responsible, Claude Code, this computer, its program build, model and working
folder. Give only the access it needs. Save the settings and complete their review. An empty folder
is refused; choose a folder that exists on this computer.

Choose the agent you want `lys claude` to use by default:

```sh
lys agents --harness claude --default <agent-name>
```

For Codex, add a Codex agent and use `--harness codex`. This choice is kept by the CLI on this
device for your signed-in person, server, computer and harness. It does not change the agent's
server settings. Lys does not choose the first agent in a list for you.

## 5. Launch it here

The agent's reviewed settings decide its terminal transport. An interactive profile gets the
harness's terminal. A managed profile keeps its controls and uses one named seat on this computer
pinned to the latest reviewed settings version.

```sh
lys claude
```

If it refuses `launch_managed_seat_missing`, run the exact `lys seat --server ... add ...` command
it prints once, then run `lys claude` again. The command names the real agent, computer and reviewed
version. If it contains `<absolute-working-folder>`, replace only that required value with the
folder this seat must work in; none was guessed. Keep managed controls enabled.

If several matching seats exist, use **Operations → Network** to choose the intended seat.
If the seat is already running, use the printed `lys attach ... --type` command to reach it.
`lys attach` leaves an existing seat running when you leave; a new run owned by `lys claude`
is asked to end when you leave its attachment.

Your current terminal attaches to the new run. The server starts your selected agent on this
computer through its runner, using the agent's reviewed settings. The server supplies the run's
proxy address and pass. No provider key or Cambium certificate is copied into the command.

The operator must configure the model proxy before allowing starts. The current CLI checks the
proxy in the returned launch template after the server has started the process, then asks to end
the session if it is missing. A process may already have made a call before that check. The
installed routing proof remains unrun.

To choose another agent for one run:

```sh
lys claude --agent <agent-name>
```

The other forms are:

```sh
lys codex
lys run claude
lys run codex
lys run --agent <agent-name> <command>
```

`<command>` selects an agent whose declared program matches it. It does not run an arbitrary
local program outside Lys. Put Lys options before the command in `lys run`.

When the harness exits or this attachment ends, Lys asks the server to end the session and
restores your terminal. A failed end is printed with its refusal; do not assume the session ended
unless Lys confirmed it. Check **Operations → Sessions** and stop that run there if necessary.
For an interactive profile, keyboard Control-C is passed to the harness. For a managed profile,
type a complete line and press Return to submit input to this run. `input admitted to session …`
confirms input admission only; it does not confirm that the managed turn finished. The terminal
shows the harness's managed frames. Control-C ends that attachment and asks to end the run.

Additional harness arguments currently refuse `launch_arguments_unavailable`: the server's
start request has no argument field. Set supported choices in the agent's settings instead.
Do not bypass Lys by running the harness directly to evade this refusal.

## 6. Find its terminal and usage in Lys

Open **Operations → Network** at
[the Network view](https://lys.ablative.com.au/#/canvas?view=tree), choose your running agent in the tree,
and its terminal fills the right side. **Operations → Sessions** lists running sessions.
The agent's own Overview also has **Open its terminal**, **Stop** and **Restart**.

Open **Operations → Canvas** for its windows and usage widgets. A **Usage** widget's
**Open its model calls** goes to **Operations → Proxy**. There, choose your agent to see its
model calls, their reported token usage and response details. Budgets and limits live on the
agent's **Budgets** tab. No usage yet means no usage has been reported, not zero usage.

## 7. Use Cambium through Lys

In [Cambium](https://cambium.ablative.com.au), choose **Sign in** and complete the Lys sign-in.
For your agent, give it Cambium access from its Lys page, then in Cambium choose
**Invite → Agent seat**, select the Lys agent and add its handle.

Access and enrolling the seat are separate acts. A person may give an agent only access they
hold and may pass on. Cambium asks Lys to certify the seat's key; the Lys-started agent uses
the run pass Lys supplied. Never hand its certificate or key to an independently started agent
as a substitute for starting through Lys.

## 8. Read a refusal and do its next act

Lys prints the named reason and next command or surface page. Keep the refusal's name when
reporting a problem. An unanswered start is not proof that nothing started: check its session
before trying another start.

| Name | Meaning and next act |
| --- | --- |
| `server_address_missing` | This terminal has no recorded server address. For this estate, run `lys sign-in --server https://lys.ablative.com.au/api`; for another estate, obtain its exact API base and name it. |
| `person_pass_missing`, `person_pass_expired`, `person_pass_wrong_server` | This terminal has no accepted sign-in for that server. Run `lys sign-in --server https://lys.ablative.com.au/api` for this estate; use the explicitly named address for another estate. |
| `machine_not_joined` | This device has no joined computer record. Open its Network Connect row and run the exact `lys runner join` command shown there. |
| `MachineRetired`, `machine_retired` | This computer record is retired. Add and connect a new computer record in Network. |
| `agent_for_harness_missing` | None of your agents on this computer declares that harness or program. Open Add an agent and choose that harness and computer. |
| `agent_default_missing` | Matching agents exist, but you have chosen no default. Run `lys agents --harness claude --default <agent-name>` or the Codex equivalent. |
| `agent_unknown`, `agent_name_ambiguous`, `agent_harness_mismatch` | The named agent is absent, its name is shared, or it declares another harness. Run `lys agents` and use an unambiguous matching name or id with `--agent`. |
| `launch_command_invalid` | The command is not a plain program name or absolute executable path. Run `lys agents --json` and use a listed program. |
| `AgentNotActive` | The agent is not ready to run. Open its Overview and use Turn on or Turn back on as offered. A retired agent cannot be turned back on. |
| `ProfileNotReviewed` | The latest settings have no recorded review. Complete their review on the agent's page before launching. |
| `WorkingFolderUnnamed` | The settings name no working folder. Choose one on this computer in the agent's settings and review the new version. |
| `MachineNotForAgent` | The computer does not allow this agent, or its reviewed settings choose another computer. Allow it here through the administrator, or choose this computer in the agent's settings and review the new version, as the printed reason says. |
| `MachineUnavailable` | Lys cannot use the selected computer's runner. Reconnect it using that computer's Network Connect row. |
| `HarnessUndeclared`, `HarnessBuildDiffers`, `ModelUnrepresentable`, `PolicyUnrepresentable` | The reviewed settings do not describe a program/build/model/policy the runner can render. Correct the named setting on the agent's page and review the new version. |
| `launch_arguments_unavailable` | This server cannot receive extra harness arguments. Put supported settings in the agent's profile and launch without extra arguments. |
| `launch_managed_seat_missing` | This managed agent has no seat here pinned to the latest reviewed settings. Run the exact `lys seat --server ... add ...` command printed, supplying the required working folder if it is missing, then launch again. Keep managed controls enabled. |
| `launch_managed_seat_ambiguous` | More than one matching seat exists. Choose the intended seat in Operations → Network and use its terminal there. |
| `seat_already_running` | The matching seat is running already. Use the exact `lys attach ... --type` command printed to reach it. Leaving that attachment keeps the existing run alive. |
| `seat_state_unknown` | The runner has not confirmed the matching seat's state. Inspect it in Operations → Network before starting another run. |
| `model_proxy_missing` | The returned launch template names no model proxy. The process may already have started; Lys asks to end its session. The operator must configure the proxy before another launch. |
| `launch_profile_changed` | The settings changed after the CLI selected them. Lys asks to end that session. Review the current settings before launching again. |
| `launch_cleanup_failed`, `launch_terminal_cleanup_failed`, `launch_terminal_end_unconfirmed` | Session cleanup or terminal restoration failed or was not confirmed. Read every named failure, check the printed session id in Operations → Sessions, and stop it there if still running. Report an unconfirmed stop to the operator. |
| `launch_terminal_required`, `launch_terminal_background`, `launch_terminal_mismatch`, `launch_terminal_size_missing` | This launch needs a foreground terminal with its own input, output and known window size. Run it directly in a terminal without `--json`, pipes, redirection or backgrounding. |
| `launch_catalog_invalid` | The server's list or settings answer is incomplete or malformed. Report its named field and refusal to the operator; no launch choice was guessed. |
| `NotHeld` | Your authority does not include the requested act. Ask the administrator for the named grant. |
| `lys_pass_absent`, `pass_expired` in Cambium | Cambium has no current Lys sign-in for you. Sign in through Lys again. |
| `seat_unknown_to_lys`, `seat_key_not_lys_agent` | That Cambium seat's key does not identify the chosen Lys agent. Enrol the correct Lys agent through Invite → Agent seat. |
| `lys_could_not_be_asked` | Cambium could not reach Lys. Report the words to the operator; signing in again cannot repair the server connection. |

For a separate MCP app, add `https://lys.ablative.com.au/api/mcp`, complete its Lys sign-in,
and select access on its consent page. **Don't connect** refuses. Selecting nothing grants
nothing. **what-can-I-do** lists routes and resources that connection may reach.

This guide describes the LYS-LAUNCH source path. Installation, the browser walk, the device launch,
proxy usage and session cleanup must be verified on the installed release before the morning room
uses it. They were not run in the write-only lane.
