# Agents — Checklist

## Words, variables and schedules

- [ ] **C701** — The five message slots (context warning, preparation, compaction, wake-up, scheduled reminder) resolve built-in, then workspace, then agent, then session, with named templates; an empty layer inherits (AGENTS-001 R1).
- [ ] **C702** — An agent's and a session's variables are JSON values with a revision, an author and an optional expiry; a stale revision is refused by name; the agent reads and sets its own through lys and the Lys MCP server (AGENTS-001 R2).
- [ ] **C703** — Delivery renders {{vars.key | fallback}}, {{goals}} and {{time_left}} at the moment of delivery and captures the rendered text with every contributing revision in the operation's receipt (AGENTS-001 R3).
- [ ] **C704** — A schedule has at, until, interval, max occurrences and recipients; missed intervals coalesce to one send; an uncertain delivery stops it for inspection; every state survives restart (AGENTS-001 R4).
- [ ] **C705** — The Usage screen shows and sets words, variables and schedules with named refusals (AGENTS-001 R5).

## Seats

- [ ] **C706** — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
- [ ] **C707** — The Sessions screen shows every seat online, offline, not seen by the runner, or seen but unregistered, from the runner's own liveness, never from a terminal program (AGENTS-002 R2).
- [ ] **C708** — Start, stop and restart are deliberate acts under Lys rights; stop goes through the harness's control channel and the runner, never typed keys (AGENTS-002 R3).
- [ ] **C709** — lys attach shows a seat's live terminal to its responsible person or an administrator and nobody else (AGENTS-002 R4).
- [ ] **C710** — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).

## Seat survival and handover

- [ ] **C711** — Supervised seats have an independent Lys process/descriptor owner and survive attaching-terminal exit or crash; manual Launch semantics remain unchanged (AGENTS-004 R1).
- [ ] **C712** — Runner and identity-server restarts rebind the same live seat/session/generation, preserve accepted hooks and uncertainty, and name unavailable authority (AGENTS-004 R2).
- [ ] **C713** — An upgrade hands an active turn, control descriptors and proxy stream over with one writer; refusal keeps the old live owner and never falls back to stop/start (AGENTS-004 R3).
- [ ] **C714** — Versioned owner/custody records migrate actual installed v3 state atomically and recover from an indexed bounded projection and tail (AGENTS-004 R4).
- [ ] **C715** — Proxy, hook, delivery, registry, reconnect and handover have measured deterministic count ratchets correlated with wall-clock work, with zero idle polling (AGENTS-004 R5).
- [ ] **C716** — Every survival behaviour has an observed main red and exact-change green, explicit fault/ready/exit signals and complete counts; tests over two seconds are defects (AGENTS-004 R6).
- [ ] **C717** — The actual roster N and 3N fixture keep per-seat work and memory bounded and unrelated seats responsive through one seat failure or handover (AGENTS-004 R7).
- [ ] **C718** — Intentional stop/restart retain fresh authority and stop-key retries; the registry reports proven owner/liveness/unknown state and the real install survives all four transitions (AGENTS-004 R8).

## Counted seat-loop performance

- [ ] **C719** — Every one of 41 census paths has an actual-code benchmark, reproducible counter vector, native-time correlation and non-increasing case ratchet; unmeasured values remain unqualified (AGENTS-005 R1/R10).
- [ ] **C720** — Capture admission, frames, worker, completion, usage and drain remove repeated history/journal work while preserving every byte and durable completion stage (AGENTS-005 R2).
- [ ] **C721** — Peer, hook, bind, status and source follow use indexed current identities/cursors instead of fleet, date and rotation history scans (AGENTS-005 R3).
- [ ] **C722** — Delivery/reminder and lifecycle costs are indexed and bounded, with existing DIRECTORY-064 and AGENTS-004 durability/retention contracts preserved (AGENTS-005 R4/R9).
- [ ] **C723** — Usage/feed enforcement touches only affected indexed memberships/standing and current immutable configuration, preserving all crossing actions (AGENTS-005 R5).
- [ ] **C724** — Registry/liveness pages use current indexed fields and proven owner events instead of copying history and polling all seats; exact totals/freshness remain unchanged (AGENTS-005 R6).
- [ ] **C725** — Terminal/pattern/transport paths count complete work, preserve every byte and match, and remove prefix re-search and shared network-wait holds (AGENTS-005 R7).
- [ ] **C726** — Approval, connector, MCP refusal and nonce paths remove per-request global scans/copies/thread growth without stale grants or weakened replay/audit (AGENTS-005 R8).
- [ ] **C734** — A message sent to a seat from the Sessions screen or `lys seat send` arrives in its Claude Code transcript as a user turn, never as typed keys (AGENTS-002 R6).

## Agents' passes to apps

- [ ] **C735** — An agent asks POST /agents/{id}/pass with its run pass or a grant credential for a live grant it holds, and is answered the rights_claim pass for an approved app carrying only its own grants, with no refresh token; every other ask is refused by name and each issue is logged (AGENTS-006 R1, R2, R4).
- [ ] **C736** — `lys agent pass` reads the grant credential from an owner-only file, asks over loopback with no operator token, and writes the pass to an owner-only file, printing neither (AGENTS-006 R3).

## The operations screen's network tree

- [ ] **C737** — The operations screen has a third view, the network tree: every machine the network knows as a branch, every seat on it as a leaf carrying the agent's display name and the seat's state as the runner last reported it; a stopped or never-started seat stays on the tree greyed, with its start act in place (AGENTS-007 R1). Functional component proof: NetworkTree.test.tsx — C737 draws every state and empty branch, excludes retired machines and names runner reasons; C737 starts an offline seat with a fresh operation and immediately uses the answered state; C737 reads all machine pages and refuses malformed or repeated pages. Not proven: the 50 ms rendering acceptance fails under shared load (57.60 ms in components-final; 228.71 ms in suite-last).
- [x] **C738** — Selecting a running leaf opens that seat's terminal beside the tree, fed by the same session stream the canvas uses, with the seat's send and stop acts; selecting another leaf swaps the terminal; a seat whose runner cannot be read is named with its reason, never shown blank (AGENTS-007 R2). Component proof: NetworkTreePane.test.tsx — C738 switches PTY terminals and aborts the previous stream exactly once; C738 follows managed attach lines, preserves the cursor, and names the stream end; C738 sends a user turn on Enter, clears only on delivery and names refusals; C738 offline and unknown leaves show their state and reason without mounting a terminal.
- [ ] **C739** — The network tree is the default view of the operations screen; a person's chosen view is kept with their canvas arrangement by the service and restored on their next visit; the canvas and the sessions table remain one click away (AGENTS-007 R3). Not proven on the served API: canvas_store.rs Arrangement denies unknown fields and has no operations member. Component tests prove the default, restoration from a fixture with operations, one write per change and a named refusal; they do not prove persistence on today's server.

## The runner's bridge under test

- [ ] **C751** — The bridge test's fixture treats a reply write refused BrokenPipe or ConnectionReset after the barrier as the bridge's correct close of a lane already refused on the other, proven red first with the second write ordered after the bridge's join; green 20 of 20 on Linux and macOS; no sleep, retry or serial marker (AGENTS-011 R1).

## The launcher when Lys is not answering (AGENTS-012)

- [ ] **C752** — Every server-rendered launch is kept on the device for its agent with the template hash, harness, proxy identity, run pass and the pass's expiry, written atomically, shown by lys agents (AGENTS-012 R1).
- [ ] **C753** — When the server's connect fails as the OS or TLS reports it, lys claude, lys codex and lys run start the agent from the kept launch with the proxy on the device recording every call, say so in the first line, and refuse by name with no kept launch or an expired pass; no clock anywhere (AGENTS-012 R2).
- [ ] **C754** — A run started while Lys was unreachable is handed to Lys whole on the next command that reaches it, accepted by launch hash and pass validity at start, idempotent by run id, never dropped, refused by name (AGENTS-012 R3).
- [ ] **C755** — The surface shows a handed-over session as started while Lys was not answering, from its device, counted in cost like any other (AGENTS-012 R4).
- [ ] **C756** — On a bare machine the first lys command asks for the Lys address once and keeps it, never answers config_unreadable; the first launch writes the device's Claude Code settings and lys agents lists them by path (AGENTS-012 R5).

## Adding an agent is one act (AGENTS-013)

- [ ] **C770** — POST /agents with run registers, turns on, saves and approves settings, records a new computer or allows on a named one, and starts only with a working folder, under one operation with one receipt naming each part and the first refusal.
- [ ] **C771** — AddAgent sends one request and keeps no walk in the browser; the act says Add or Add and run by whether a folder is chosen; a refused part says what was done and the next act.

## A test's runner and owners end with the test (AGENTS-014)

- [ ] **C772** — lys runner serve and lys runner seat-owner started with --ends-with-fd end, the way an intentional stop ends them, when that descriptor reaches end of file; the runner passes it to every owner it starts; without it nothing changes and the installed Lys never passes it.
- [ ] **C773** — Every lys-runner integration test that starts a runner holds the write end of its lifeline for the test's life, and the Mac's process list read before and after the crate's test run shows no runner or owner the run started outliving it.

## An agent sets up and starts another agent under grants, from any device (AGENTS-015)

- [x] **C774** — The agent, seat, machine-runner and grant pass-on routes admit an agent's signed call held to its own live grants, refused not_permitted by name otherwise; an agent's new agent answers to the agent's person; no agent approves a profile version it set.
- [x] **C775** — An agent that registers an agent holds, under the person's agent.create grant, exactly the rights to set its profile, start, start-command, stop and restart it, withdrawn when that grant is revoked; the agent is registered and activated before its grants are appended, and the grant append is idempotent on retry, so a crash never leaves rights on an agent that does not exist (AGENTS-015 R2, ordered by AGENTS-016 R3).
- [x] **C776** — lys mcp reaches Lys's MCP over verified https from any device, http only on loopback, and says a connection or TLS failure by name.
- [x] **C782** — The agent file's Give form offers 'Create agents for me', 'Run my agents on this computer' and 'Set up and review this kind of profile' as one choice each in plain words, each posting exactly its grant, absent where the person has nothing to give.

## The Lys computer is a device, and the agent's file shows its connection lines (AGENTS-016)

- [x] **C783** — On the computer that runs Lys, a launch matches the machine whose runner is the install's own through the runner key the view carries, and every advice sentence in launch_device names only labels the Computers page renders.
- [x] **C784** — An agent's file shows the complete claude mcp add and codex mcp add lines for this install and agent with a copy act each, and says which origin works now and which waits on AGENTS-015 R3.
- [x] **C785** — An agent holding agent.create issues the first certificate of an agent whose record names it as registrar, and only while that agent holds none; a second certificate, one for an agent it did not register, one after the person revoked its agent.create, and every withdrawal are refused by name (certificate_not_first, agent_not_yours, agent_create_revoked, not_permitted), and the certificate acts stay withheld from grants.
- [x] **C786** — GET /dashboard lists every agent an agent registered and every start an agent made for the signed-in person, and the dashboard shows them under one heading in plain words with Stop beside each, posting the agent's stop as the agent file does; no lines, no heading.

## An agent is enrolled in one act, stopped without harm, started without a second review, rooted in one choice, seated where it runs (AGENTS-017)

- [x] **C787** — lys agent enrol, run where the agent will run, generates its key, posts the request as the PEM the CLI writes, receives the certificate and writes the seat file at the reviewed profile's path, mode 0600, never overwriting one that exists.
- [x] **C788** — The dashboard's Stop beside a started line ends that run through POST /runtime/sessions/{id}/end and keeps the certificate; Withdraw is the emergency act behind its confirmation, final, and its receipt says the registrar cannot certify again.
- [x] **C789** — A profile version without its own review starts when its responsible person or the administrator reviewed the same settings content on any agent of theirs, the answer naming the carried review; a changed digest needs review again; an agent never carries a review it made.
- [x] **C790** — One standing choice gives an agent its roots, agent-roots first for an administrator without them, then four grants in order with one receipt; the install names the computer Lys runs on, by the administrator with the lys runner, idempotently.
- [x] **C791** — The enrol command writes the one seat file shape both Lys's bridge and Cambium's door read, with the door's fields when asked, so a Cambium seat's key is born on the computer it runs on and no seat file is carried by hand.

## An owner a test starts ends with the test, and an owner whose folder is gone ends itself (AGENTS-018)

- [ ] **C814** — Every test fixture that starts a seat owner ends that owner's process group and waits for its exit when the fixture is dropped, on return, on panic and on close, with the pid and group read from the runner's record of the start (AGENTS-018 R1).
- [ ] **C815** — A seat owner watches its own folder as an event (kqueue EVFILT_VNODE on macOS, inotify on Linux) and, when the folder is removed or renamed away, ends its harness's group, writes one line to stderr and exits 0 (AGENTS-018 R2).
- [ ] **C816** — A seat owner whose folder cannot be watched refuses to start by name, owner_dir_unwatched, so no owner runs unwatched (AGENTS-018 R2).

## Everything but a person's credentials can be given to an AI (AGENTS-019)

- [ ] **C829** — WITHHELD_FROM_AGENTS names exactly person.password.set, person.sign-in.set and person.login.bind; the other twenty-nine acts it named are in AGENT_MAY_HOLD and in AGENTS_NOT_MACHINES, so a person can give them to an agent and no machine ever holds them (AGENTS-019 R1).
- [ ] **C830** — A grant to an agent of an act moved by R1 stands only on the giving person's own live grant of that act, and withdrawing the giver's grant ends the agent's with it; an agent never approves or declines a request it made, refused own_request (AGENTS-019 R1).
- [ ] **C831** — An agent holding organisation.admit on its organisation admits a person through POST /people/admit by its relayed signature or by an approved app's agent pass, the first grant asked held to the agent's own chain, every leaf signed as the agent and naming its responsible person; without the grant it is refused NotHeld naming who can give it (AGENTS-019 R2).
- [ ] **C832** — An agent holding person.profile.set or person.email.set changes another person's profile or email through MCP; POST /directory/people/{id}/account/password and the person's own sign-in routes refuse an agent WithheldFromAgents by name (AGENTS-019 R3).
- [ ] **C833** — Every Lys route that is not public either admits an agent's signature judged by its declared scope, or is on HUMAN_ONLY with its reason, or is a kept responsibility; tests/agent_route_parity.rs fails a route on none of the three or on two (AGENTS-019 R4).
- [ ] **C834** — what-can-I-do lists for an agent every route whose declared scope its live grants reach, administrator-body routes included (AGENTS-019 R4).
- [ ] **C835** — A person's dashboard acted-by-agents list carries every act their agents did under an act moved by R1 and every admission an agent made for them, newest first, in plain words, beside the registrations and starts it lists today (AGENTS-019 R5).

## AGENTS-020: an agent with no identity pairs with its person by a spoken code

- [ ] **C836** — An unknown key asks Lys for four words, a signed-in person enters them, names the agent and gives its roots, Lys certifies the key and answers the approved apps, and `lys agent pair` does the agent's side. AGENTS-020/R1-R4.

## AGENTS-021: one Lys plugin for both hosts, installed by one line that names the Lys it reaches

- [ ] **C837** — One plugin tree plugins/lys serves Claude Code and Codex (plugin/ retired), `npx lys-plugin install [--lys <url>]` adds the marketplace and the plugin to every host on PATH through the hosts' own commands judged by their words, and every skill matches the four served tools and routes.json. AGENTS-021/R1-R4.
