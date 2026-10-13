# Move a seat off herdr and Argus into Lys

AGENTS-002 R5 and R6 (C710, C734; S401, S418). This moves **one seat at a time**. The first seat moved is
**Waffles's own**. Move the next seat only once the seat before it has passed step 9.

When a seat has been moved, Lys starts it, sees it and warns it. herdr no longer holds a pane for it, and Argus no
longer delivers to it. The seat runs **managed**: its harness is started with stream-json input and output, and a
message reaches it as a user turn, never as typed keys (ADR-130).

In the steps below, `<seat>` is the seat's name in Lys: lowercase letters, digits and `-`, from 1 to 64 characters,
for example `waffles`. `<agent>` is the Lys agent identity the seat runs as. `<machine>` is the Lys machine record
whose runner is this install's own (kind `lys`). Every value comes from your own install. None is written here.

## Join a second Mac

The second Mac needs the device clients, its own machine key and its own harness installation and account.
The identity service stays on its existing host. A runner is the process that holds sessions; a computer record
names where an agent may run; joining makes a machine identity from the device's public key. A seat names an
agent, a reviewed profile version and the computer whose runner starts it.

The following client commands are a source-derived procedure, not a completed device proof. Authenticate Git
for this private repository and its private dependencies first. Run them in a new directory you own. The
checkout uses the toolchain pinned in `rust-toolchain.toml`; an existing Rust 1.94.1 does not satisfy that pin.
Check `sysctl -n hw.logicalcpu` before building: four build jobs leave two slots free only on a device with at
least six logical CPUs. Keep the default Cargo target directory in this checkout.

```sh
caffeinate -i nice -n 19 git clone https://github.com/ablative-io/lys.git lys-device
cd lys-device
git rev-parse HEAD
export CARGO_NET_GIT_FETCH_WITH_CLI=true CARGO_BUILD_JOBS=4 NEXTEST_TEST_THREADS=4
caffeinate -i nice -n 19 rustup toolchain install 1.98.1 --component rustfmt --component clippy
caffeinate -i nice -n 9 sh -c 'nice -n 10 cargo build --locked --release -p lys'
caffeinate -i nice -n 9 sh -c 'nice -n 10 cargo build --locked --release -p lys-runner'
mkdir bin
install -m 755 target/release/lys target/release/lys-containment-entry bin/
export PATH="$PWD/bin:$PATH"
export LYS_IDENTITY_HOME="$PWD/device-state"
lys --version
lys runner join --help
```

The outer nice increment of nine and Cargo's increment of ten give Cargo an effective niceness of nineteen.
`lys-runner` provides the `lys-containment-entry` executable; its serve and dial commands are subcommands of
`lys`. Keep both executables together. Record the harness's absolute executable path and version on this Mac
with `command -v claude` / `claude --version` or `command -v codex` / `codex --version`. Sign in to the harness
on this Mac with its own account. Never copy another device's credentials or harness configuration.

In the browser, the person opens the issuer's `/#/sign-in`, enters their own **Email** and **Password** and
clicks **Sign in**, or clicks the offered provider sign-in link and completes it themselves. In **Network**,
an administrator uses **Computer**, enters the computer's **Name**, chooses **Another computer**, and clicks
**Add**. The **Connect** row shows a command and a one-time code with separate **Copy** buttons. The form is
currently displayed only to administrators; a new person needs that administrator's assistance even though
the server serves an own-computer join-code route. Do not describe an operator token as a personal login.

Run the command the browser gave on the second Mac, under `caffeinate -i nice -n 19`. Preserve its full server
address, including `/api` when present:

```sh
caffeinate -i nice -n 19 lys runner join --server <server-from-the-command> --machine <computer-id>
```

It prompts `Paste the connection code Lys gave for this computer, then press Return:`. Paste the code into
stdin, never into an argument or shell history. Successful join prints `This computer joined Lys at ... as
computer ...`, then `listening ... as runner ...`. Leave this command running. Control-C stops it and ends
the sessions it holds. A later `join` needs a new one-time code; the device keeps its private key locally.

Have the administrator allow the agent on this computer and review a profile that names the executable's
absolute path on this Mac, the intended model, a working folder on this Mac and managed controls. The
catalogue's installed builds are read on the service host, so a path shown there is not proof of a path on
this device. Record and start the seat from the service host with `lys seat add` and `lys seat start`, as in
steps 2 and 6 below, using the joined computer's id. The fresh device's `lys seat` and `lys attach` commands
require a local identity-service configuration and operator token; they cannot log in to the remote issuer.
Use the browser's Operations screen to send a turn, read its answer and stop the seat, or use the service
host's CLI as in steps 7 and 8. Only the runner's answer and a completed turn prove device execution.

At source revision `307f6a6e`, managed starts accept only the launched-binary qualifications in
`crates/lys-runner/src/harness_control/process/qualified.rs`: Claude Code `2.1.296` and Codex
`0.161.0-alpha.3`. A different device build is refused `control_adapter_unqualified`; installing a harness
without checking its version does not qualify a seat. Managed seats use `lys runner managed-entry`.

For a disposable proof, stop and retire its seat, retire its agent and computer, stop the owned runner and
remove only the checkout and device-state directory you made. Remove a newly installed Rust toolchain only
when it was absent before this proof and no other work uses it. Preserve existing installations and caches.

## Before you start

- Lys is installed and running on this computer, and `lys` reaches it (the CLI reads the operator token itself and
  never prints it).
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
   with its last signal time. The Sessions screen in Lys shows the same: under **Operations**, the **Seats** table is on the page when nothing is running, and in the **Agents** panel otherwise.

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
