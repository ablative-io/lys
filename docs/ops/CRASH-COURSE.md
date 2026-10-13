# Lys crash course: bringing a person and their agent on

Written by Waffles, 13 October 2026 night, from the two Codex walks of the installed servers
(Lys ca2da3f9, Cambium b89f098b9). Every step below was exercised on the installed servers that
night unless it says "tonight's lane"; those steps land with the overnight installs and are read
back in the morning. The addresses are the public ones: https://lys.ablative.com.au and
https://cambium.ablative.com.au.

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
needed when the grant from step 1 exists. If the page says Offline with words about a live wire
address, reload once; tonight's door lane makes the surface say the real cause and recover itself.

## 3. The person gets an agent in Lys

Today an administrator registers the agent (**Agents**, add, the person as responsible), activates
it, and reviews a profile: which harness (Claude Code or Codex), which executable on which computer,
model, working folder, controls. Tonight's lane (AGENTS-008 R6) lets the person register and
certify their own agent. The profile's executable must be the absolute path on the computer the seat
runs on; today the catalogue only shows the server's own programs, tonight's lane (R2) asks the
chosen computer's runner instead.

## 4. The person's computer joins (their own Mac)

In Lys, **Network**, add the computer and choose **Another computer**; the Connect row shows one
command and a one-use code. Run the command on that Mac, paste the code when asked, and the runner
is up while the command runs (Control-C ends every session it holds). Today the add row is shown to
administrators only and the Mac needs a `lys` built from source; tonight's lane (R1, R3) shows the
row to every signed-in person and serves the built `lys` for download from the same row.

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
landed tonight (793d5fe9) makes it work for PTY-held seats too. From the person's own Mac these
verbs need tonight's lane (R4): `lys sign-in --server https://lys.ablative.com.au/api` once, then
the same `lys seat ...` verbs with `--server`, as the person, with no operator token anywhere.

## 6. The agent reaches Cambium

In Cambium, the **Invite** chip, **Agent seat**, a handle, **Add seat**. Today this mints a key and
a seat file that Lys knows nothing about, so the seat cannot get its Lys pass; tonight's Cambium
lane (ACCESS-008) makes Add seat pick one of the person's Lys agents, has Lys certify the seat's key,
writes `lysUrl` and `lysAgentId` into the seat file, and lets a Lys-started seat prove itself with
the run pass Lys gave it. Until that lands, agent seats on Cambium are the ones already enrolled.

## 7. What refuses, and what it means

- `lys_pass_absent` on Cambium: no Lys sign-in on this browser session; sign in through Lys.
- `NotHeld` from Lys: the person has no grant for that question; give the grant (step 1).
- `MachineNotForAgent`: allow the agent on that computer (step 5).
- `pass_expired`: the door's pass ran out and the refresh failed; reload once.
- `lys_could_not_be_asked`: the door's line to Lys failed (the 13 October outage); tonight's lane.
- `seat_unknown_to_lys` / `seat_key_not_lys_agent`: the seat's key is not a certificate Lys issued
  for that agent (step 6).

## 8. The Lys MCP

Waffles' seat gets the Lys MCP once Tom authorises it in the browser (the authorize link in the
night's last message); the same tools answer the same routes as the screens above, so the morning
crash course is the steps here, done by asking.
