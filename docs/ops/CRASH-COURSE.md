# Lys crash course: bringing a person and their agent on

Use https://lys.ablative.com.au and https://cambium.ablative.com.au. These instructions include
the evening source changes. Waffles installs them and reads the steps back on the installed
servers before the morning room uses them.

## 1. Admit the person (an administrator, once per person)

In Lys, open **People and agents**, click **Add person**, give the display name, the email and the
first grant. The one grant that opens Cambium is `member` on `cambium.workspace:ablative`: with it
the person reads the workspace, posts, and can be given seats. `reader` opens reading only. The
same act over the API is `POST /api/people/admit`, which registers, makes the issuer account, binds,
activates and grants in one call and answers the five steps by name.

## 2. The person signs in

Open https://cambium.ablative.com.au, click **Sign in**, then **Sign in with Google** on the Lys
page. The person completes Google themselves; Lys hands Cambium the person's Lys id, Cambium asks Lys
one question (`read` on the workspace) and lets them in. Nothing is pasted and no invitation is
needed when the grant from step 1 exists. If the page says Offline, read the words: since the door of 14 October 00:43 (d4d4f2014) they name
the real refusal (the issuer's name, or "Lys could not be asked") instead of "no live wire address", and
the door retries Lys once on a fresh connection before it says so. Reload once; if it stays, send the words.

## 3. The person gets an agent in Lys

In Lys, open **Agents**, add the agent with yourself as responsible, and choose its harness
(Claude Code or Codex), computer, model, working folder and controls. The profile names the command
to run: `claude`, `codex`, `cdx`, or the absolute path to your fork. A plain command is resolved on
the chosen computer at each start. An absolute path is launched exactly as written. The catalogue
asks that computer's runner which programs it can use. Give a working folder on that computer
before saving and starting the profile.

The add-agent form's **Access** choices are described in step 6. Select them before adding the agent
if it will use Cambium straight away.

## 4. The person's computer joins (their own Mac)

In Lys, **Network**, add the computer and choose **Another computer**; the Connect row shows one
command and a one-use code. Run the command on that Mac, paste the code when asked, and the runner
is up while the command runs (Control-C ends every session it holds). Since 3fac9c99 the row is
shown to every signed-in person and serves the built `lys` for download from the same row (R1, R3);
before, it was administrators only and the Mac needed a `lys` built from source.

The `lys` on Tom's Mac: the installed one is linked at `~/.local/bin/lys`; `lys --version` must
name the install's commit. A `lys` that answers "unrecognized subcommand 'seat'" is the stale cargo
copy.

## 5. Start the seat, send it a turn, watch it

On the server's host, as the operator:

```
lys seat add <name> --agent <agent-id> --profile-version <n> --machine <machine-id> --working-folder <dir>
lys seat start <name>
lys seat send <name> 'Reply with exactly OK.'
lys attach <name>
lys seat stop <name>
```

`start` is refused `MachineNotForAgent` until the computer lists the agent among those that may
run on it (Network, the computer, allow the agent). `attach` follows the live session; the fix
landed tonight (793d5fe9) makes it work for PTY-held seats too. From the person's own Mac, since
3fac9c99 (R4): `lys sign-in --server https://lys.ablative.com.au/api` once, then
the same `lys seat ...` verbs with `--server`, as the person, with no operator token anywhere.

## 6. The agent reaches Cambium

In Lys's add-agent form, under **Access**, find the Cambium resource you want the agent to use and
tick the acts it needs. Lys offers the acts Cambium declares under `agents` for that kind: read and
send on channels and topics, and the declared acts on the other kinds. A combined relation gives
every act named in its label. **Everything here** selects only the offered acts. An act Cambium
withholds stays grey with its reason. The person must hold access they may pass on to an agent;
ask the administrator for that access if no Cambium choices are offered.

For an agent already added, give it the same Cambium access from its agent page before asking it
to act there. Giving access and enrolling a Cambium seat are separate steps:

In Cambium, open **Invite**, choose **Agent seat**, choose your Lys agent, give the seat a handle,
and press **Add seat**. Cambium asks Lys to certify that seat's key and writes `lysUrl` and
`lysAgentId` into the seat file. A Lys-started seat proves itself with the run pass Lys gave it.

## 7. What refuses, and what it means

- `lys_pass_absent` on Cambium: no Lys sign-in on this browser session; sign in through Lys.
- `NotHeld` from Lys: the person has no grant for that question; give the grant (step 1).
- `MachineNotForAgent`: allow the agent on that computer (step 5).
- `pass_expired`: the door's pass ran out and the refresh failed; reload once.
- `lys_could_not_be_asked`: the door's line to Lys failed on the first try and on one retry over a fresh
  connection (installed 14 Oct 00:43). Seeing it now means the line to Lys is down, not a blip.
- `pass_expired` on a delivery (door log, "recipient grant projection's pass has expired"): that person's
  session pass ran out and no refresh has succeeded since; they get no live pushes until they reload and the
  refresh succeeds. History still shows everything when they do.
- `seat_unknown_to_lys` / `seat_key_not_lys_agent`: the seat's key is not a certificate Lys issued
  for that agent (step 6).

## 8. The Lys MCP

A harness started by Lys receives its Lys MCP connection and run pass at launch. To connect an
external MCP app, add `https://lys.ablative.com.au/api/mcp` in that app and follow its sign-in link.
Sign in through Lys. The consent screen groups its choices by resource; tick what the app may do,
then press **Connect**. **Don't connect** refuses the connection. Ticking nothing connects nothing:
the app connects with no permissions.

Ask **what-can-I-do** to read the routes and resources that connection may reach. With no selected
access, it answers "nothing: connect again and tick what the app may do". Reconnect and choose the
needed access before sending a change. The MCP uses the same permissions and refusals as Lys's
screens.
