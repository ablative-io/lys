# Move a seat off herdr and Argus into Lys

AGENTS-002 R5 and R6 (C710, C734; S401, S418). This moves **one seat at a time**. The first seat moved is
**Waffles's own**. Move the next seat only once the seat before it has passed step 9.

When a seat has been moved, Lys starts it, sees it and warns it. herdr no longer holds a pane for it, and Argus no
longer delivers to it. The seat runs **managed**: its harness is started with stream-json input and output, and a
message reaches it as a user turn, never as typed keys (ADR-130).

In the steps below, `<seat>` is the seat's name in Lys: lowercase letters, digits and `-`, from 1 to 64 characters,
for example `waffles`. `<agent>` is the Lys agent identity the seat runs as. `<machine>` is the Lys machine record
whose joined runner holds the session, on this computer or another one. Every value comes from the
Lys server you signed into. None is written here.

## Join a second Mac

The Lys identity server stays on its existing host. Each device has its own joined runner, which
holds sessions on that device. A seat names an agent, a reviewed profile version and that computer.
Sign-in authorizes the person; joining identifies the computer. Neither replaces the other.

Follow [the crash course](CRASH-COURSE.md#1-install-the-two-device-clients) to download and install
`lys` and `lys-containment-entry` together from the signed-in Network Connect row. Choose the
client for this Mac's chip. A person can add and connect their own computer; an administrator is
not required for that step. If Lys holds no release for the platform, ask its operator to supply it.

Install the required Claude Code or Codex build on this Mac and use this device's own account.
Never copy another device's credentials or harness configuration. The reviewed agent profile must
name the build and command this runner will use. A plain command is resolved on this device at
start; an absolute path is used exactly. The catalogue asks the chosen computer's runner, so a
service-host executable path does not establish that an executable exists on this device.

Sign in from the device terminal:

```sh
lys sign-in --server https://lys.ablative.com.au/api
```

For another estate, use its exact API base. Complete the browser's sign-in and approval yourself.
The client saves that server address with the person pass. No operator token is handed to the device.

In **Network**, add the named computer with **Another computer**. Its **Connect** row shows
one command and a one-use code. Run the shown command on this Mac, preserving the full server
address, including `/api` when present:

```sh
lys runner join --server <server-from-the-command> --machine <computer-id>
```

It prompts `Paste the connection code Lys gave for this computer, then press Return:`.
Paste the code into stdin, never into an argument or shell history. Successful join names the
computer, then says the runner is listening. Leave this terminal running. Control-C stops the
runner and ends every session it holds. A later join needs **New code** from that computer's page;
the device keeps its own private key locally.

Allow the agent on this computer and review settings that name its program build, model, a
working folder on this device and managed controls. An administrator changes an existing
computer's agent permissions. Check that the reviewed version has the intended working folder:
it currently belongs to the profile, so selecting another computer does not supply another folder.

Run the `lys seat` and `lys attach` steps below from the signed-in device. They reach the saved
server as the person; they do not require an identity-service install on the device. To name the
server explicitly, use `lys seat --server <api-base> ...` and `lys attach --server <api-base> <seat>`.
On a host that also has a local server install, explicitly name the remote server for personal
operation rather than selecting the local operator connection.

For an ordinary interactive launch after joining, `lys agents` lists your agents on this computer;
`lys claude`, `lys codex` or `lys run --agent <name> <command>` starts one through the server and
attaches your terminal. A managed profile keeps its controls and selects one existing seat on
this computer pinned to its latest reviewed settings. If none exists, `launch_managed_seat_missing`
prints the exact `lys seat --server ... add ...` command; run it once, supplying a required working
folder if the command names one as missing, then launch again. Several matching seats refuse
`launch_managed_seat_ambiguous`; choose the intended one in Operations → Network. An already
running seat refuses `seat_already_running` with the command to attach to it. Keep controls enabled.

The wrapper's managed terminal submits complete lines to the exact run it started and shows
managed frames. `input admitted to session …` confirms input admission only; it does not confirm
that the managed turn finished. Control-C ends its attachment and asks to end that run.
`lys attach <seat> --type` instead reaches an existing named seat and leaves it running when its
attachment ends. The migration below uses named, managed seats: moving their supervision still
requires the old-pane guard and both proof lines in step 9. A wrapper launch is not evidence that
supervision was moved.

The operator must configure the model proxy before allowing starts. The wrapper validates it in
the returned template after the server starts the process and asks to end the session if it is
missing. A call may precede that check. Installed proof of proxy routing remains unrun.

The runner refuses an unqualified managed build as `control_adapter_unqualified`. Read its
named required build rather than assuming any locally installed version is qualified. Managed
seats use `lys runner managed-entry`; only a runner answer and a completed turn prove execution
on this device.

For a disposable proof, stop and retire its seat, retire its agent and computer, then stop only the
runner you started. Preserve the installed clients, other devices' state and existing caches.

## Before you start

- The Lys server is running, the chosen computer's runner is connected, and this terminal has signed in to
  that server. A service-host operator may instead use its local CLI connection; its token stays on numeric
  loopback and is never copied to a device.
- Argus is still running, so the old state can be checked against the new state. Its base is in `ARGUS_URL`
  (`http://127.0.0.1:4100` on this estate). The identity server makes its own check, refusing a start with
  `seat_online_in_monitor`, at the base its `LYS_MONITOR_URL` names: set it to the same base in the service's
  environment, or the check is skipped and the start says the monitor was not asked. When Argus wants its collector
  secret, set `LYS_MONITOR_SECRET` to it and `LYS_MONITOR_SECRET_HEADER` to the header Argus reads it from; a secret
  set with no header name refuses the start `ConfigInvalid`, and no header name is assumed.
- You know the seat's herdr pane and its Argus session id:

  ```sh
  herdr agent list
  curl -s "$ARGUS_URL/api/seats" | jq '.seats[] | select(.name == "<seat>") | {name, session_id, pane, online}'
  ```

  Write down `pane` and `session_id`. Steps 4 and 5 use them.

## Steps

1. **Make the agent's profile managed.** In Lys, open the agent's page, then its **Profile** tab. Tick **Require
   automatic context and reminder controls** (this sets the profile's `session.requires_controls` to true), then
   save. Saving makes a new profile version. Have that version reviewed under **Review**. Write down its version
   number as `<version>`. A seat whose profile does not require controls is refused `seat_not_managed` at step 6.

   Keep the profile otherwise the same as the herdr seat's: the same harness, the same model and the same working
   folder. A move changes where the seat runs from, never what it is.

2. **Record the seat in Lys.**

   ```sh
   lys seat add <seat> --agent <agent> --profile-version <version> --machine <machine> --working-folder <folder>
   lys seat list
   ```

   `lys seat list` shows `<seat>` as `not-seen`: it is registered, and no session is live yet. The possible refusals
   are `seat_name_taken`, `seat_name_invalid`, an unknown agent, a machine that names no runner, and a profile
   version that has not been reviewed. Each one is shown by its name. Fix what it names, then run the command again.

3. **Check that Lys refuses the start while Argus still sees the seat.** The pane is still running, so:

   ```sh
   lys seat start <seat>
   ```

   must be refused with `seat_online_in_monitor`, naming `<seat>`. This is the guard that stops a seat from being
   watched twice. If Argus cannot be reached, the start says so and does not block. In that case, check
   `online` by hand with the `curl` command above before going on.

4. **Stop the herdr pane.** End the harness cleanly first, so that its transcript is closed. Then close the pane:

   - Use Argus's `seat_stop` for `<seat>`. It ends the session the way Argus always has.
   - Run:

     ```sh
     herdr pane close <pane>
     herdr agent list
     ```

   The second command must no longer list the pane.

5. **Turn off Argus's delivery for that seat.** Set its transport to none:

   ```sh
   curl -s -X PUT "$ARGUS_URL/api/sessions/<argus-session-id>/target" \
     -H 'content-type: application/json' \
     -d '{"transport":"none","author":"<your name>"}'
   curl -s "$ARGUS_URL/api/seats" | jq '.seats[] | select(.name == "<seat>") | {name, online, transport}'
   ```

   The Argus MCP tool `target_set` with `transport: none` does the same. The second command must show the seat
   with `online` false and no transport.

6. **Start the seat from Lys.**

   ```sh
   lys seat start <seat>
   lys seat list
   ```

   The start answers with the Lys session id and, once the harness reports it, the harness session id. Write down
   the harness session id as `<session-uuid>`. `lys seat list` shows `<seat>` as `online-idle` or `online-working`,
   with its last signal time. In Lys, **Operations → Sessions** shows the **Seats** table.
   **Operations → Network** shows the selected seat's terminal beside the tree.

   If Lys refuses the start, the refusal is named; `seat_online_in_monitor` means step 4 or 5 has not finished.

7. **Send the seat a message from the screen and from the CLI.** Then read both messages back in the seat's
   transcript.

   - On the Sessions screen, type `move check: screen` in the seat's send box and press **Send**. The screen says
     **Delivered to session … as a user turn**.
   - From the CLI:

     ```sh
     lys seat send <seat> move check: cli
     ```

   - Read both back. The transcript is
     `<the seat's CLAUDE_CONFIG_DIR>/projects/<slug>/<session-uuid>.jsonl`, under the home Lys provisioned for the
     seat:

     ```sh
     t=$(find "$HOME" -path '*/projects/*' -name '<session-uuid>.jsonl' 2>/dev/null)
     echo "$t"
     jq -c 'select(.type == "user") | .message.content' "$t" | grep -F 'move check:'
     ```

     Both `move check: screen` and `move check: cli` must appear as user messages, and the seat must answer them.
     If `find` prints nothing, the transcript is not under `$HOME`. Search the seat's home instead. Do not take an
     empty result as proof that the message failed.

   A send to a seat that is not running is refused `seat_not_running`. Empty text, or text with a control
   character, is refused by name.

8. **Watch it live.**

   ```sh
   lys attach <seat>
   ```

   This shows the seat's live session: user turns, assistant text, tool calls and their results, and status. Send
   another message from the screen and watch it land. Leaving `lys attach` never ends the session. Each attach is
   kept as a record naming who attached.

9. **Prove the move.** The move is complete only when both of these are present:

   - **The first `[Lys context watch]` warning has been delivered by Lys.** When the seat's context reaches its
     warning point, Lys delivers the warning through the managed control channel (DIRECTORY-064). Lys's delivery
     log records it. On the Sessions screen, open the seat's running entry, press **Controls**, and read **Control
     operations**: there must be a receipt for the warning that the harness admitted. Read it back in the transcript
     too:

     ```sh
     jq -c 'select(.type == "user") | .message.content' "$t" | grep -F '[Lys context watch]'
     ```

   - **Argus has made no delivery for that seat since step 5.** Read Argus's intervention journal for
     `<argus-session-id>` with the Argus MCP tool `session_interventions`. Also run the `curl "$ARGUS_URL/api/seats"`
     command above. There must be no entry after the time of step 5. The seat must show no transport and `online`
     false.

   Write both proof lines (the Lys receipt's operation and time, and the last Argus entry's time) into the room post
   that says the seat was moved.

## If something goes wrong

- **To go back,** stop the seat in Lys with `lys seat stop <seat>`. If Lys refuses `seat_turn_in_progress`, the
  seat is in the middle of a turn. Wait for the turn to end, or use `lys seat stop <seat> --force` to end it
  mid-turn. Then start the pane in herdr again and set the Argus transport back to `herdr` with the pane id. Until
  that is done, the seat has nothing watching it. Say so in the room.
- **Never** run the seat in both places at once. If both Argus and Lys show it online, stop the Lys session first.
