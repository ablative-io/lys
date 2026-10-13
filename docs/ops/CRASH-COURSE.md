# Lys crash course: bringing a person and their agent on

Written by Waffles, 13 October 2026 night, from the two Codex walks of the installed servers
(Lys ca2da3f9, Cambium b89f098b9). Every step below was exercised on the installed servers that
night unless it says "tonight's lane"; those steps land with the overnight installs and are read
back in the morning. Updated 14 October 03:1x: Lys 3fac9c99 is installed (the AGENTS-008 rows below are
in it; its full-suite reruns are still pending, so each of those steps is read back in the morning before
anyone is sent through it); the Cambium door is d4d4f2014 (ACCESS-007); ACCESS-008 is on Cambium main at
dee2861c7 and installs with the PERF read-boundary work. The addresses are the public ones: https://lys.ablative.com.au and
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
needed when the grant from step 1 exists. If the page says Offline, read the words: since the door of 14 October 00:43 (d4d4f2014) they name
the real refusal (the issuer's name, or "Lys could not be asked") instead of "no live wire address", and
the door retries Lys once on a fresh connection before it says so. Reload once; if it stays, send the words.

## 3. The person gets an agent in Lys

Today an administrator registers the agent (**Agents**, add, the person as responsible), activates
it, and reviews a profile: which harness (Claude Code or Codex), which executable on which computer,
model, working folder, controls. Since Lys 3fac9c99 the person registers and certifies their own agent
(AGENTS-008 R6). The profile's executable must be the absolute path on the computer the seat
runs on; since 3fac9c99 the catalogue asks the chosen computer's runner (R2); before it only showed the
server's own programs.

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

In Cambium, the **Invite** chip, **Agent seat**, a handle, **Add seat**. Today this mints a key and
a seat file that Lys knows nothing about, so the seat cannot get its Lys pass; ACCESS-008 (on Cambium main
dee2861c7, installs with the next door) makes Add seat pick one of the person's Lys agents, has Lys certify the seat's key,
writes `lysUrl` and `lysAgentId` into the seat file, and lets a Lys-started seat prove itself with
the run pass Lys gave it. Until that install, agent seats on Cambium are the ones already enrolled.

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

Waffles' seat gets the Lys MCP once Tom authorises it in the browser (the authorize link in the
night's last message); the same tools answer the same routes as the screens above, so the morning
crash course is the steps here, done by asking.
