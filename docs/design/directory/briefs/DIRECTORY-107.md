---
type: brief
id: DIRECTORY-107
cluster: directory
title: No Lys store is ever lost or emptied silently: a receipt per store, a put-back that removes nothing, and refusals that say what was lost
---

# DIRECTORY-107: No Lys store is ever lost or emptied silently: a receipt per store, a put-back that removes nothing, and refusals that say what was lost

> **Cluster:** directory
> **Depends on:** DIRECTORY-082, DIRECTORY-045
> **Design anchor:**
> - ADR-114 — An installed identity product names its build and upgrades itself in place, keeping the previous build to return to — Every Lys binary answers --version with the commit and dirty state it was built from, stamped at build time; a build with no commit to read says so in those words, never a made-up value. An exported build may be given LYS_BUILD_COMMIT as exactly 40 lowercase hexadecimal characters: its stamp is `<commit>; stated`, distinguishing the builder's statement from a commit read from Git; invalid values or disagreement with a tracked checkout's HEAD refuse the build, and changing or removing the variable refreshes the stamp. `lys identity upgrade` takes a folder of newly built binaries (and optionally a screens package), checks each one's --version, stops the service and the broker by their exit events, swaps the binaries by rename keeping the previous set beside them, starts the new ones and waits for them ready; if any new binary fails to start or become ready, it puts the previous set back, starts it, and fails naming what broke. Data, credentials, configuration and the compose services are never touched by an upgrade. Rejected: making install restart what differs, which would mix a first install's promises with an upgrade's risks; asking people to copy binaries by hand.
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> - ADR-113 — The identity install and its clients wait on signals, never on a clock — A wait ends on the event it is waiting for or on the failure that makes the event impossible: a service is ready when it says so or when a single connection to it succeeds after a readiness event (its log gaining its listening line, observed through the platform's file-change notification), a stopped process is gone when the platform reports its exit (kqueue EVFILT_PROC on macOS, pidfd on Linux), a peer that refuses or closes is an error at once, and nothing carries a timeout. The screen re-measures on a ResizeObserver or animation-frame signal. Rejected: shorter sleeps, which still wait on a clock; keeping timeouts as a safety net, which the rule forbids and which hides a stuck peer's cause.
> **Checklist:**
> - C499 — A put-back moves the window's data/ to data.rolled-back/<rfc3339>/ and removes none of it, and prints that folder with the count of leaves per store (DIRECTORY-082 R1).
> - C500 — After a put-back that follows writes in the window, the previous build reads every pre-window leaf, and every window leaf is found and counted in data.rolled-back (DIRECTORY-082 R1).
> - C777 — install/stores.json records every store the server opens, the SpiceDB projection and the install's keys and join files, each by the head it is honestly compared by (append-only logs by origin and head index, one-file stores by journal head, SpiceDB by projected grant revision, keys and join files by digest and presence) and by its class (whole or own), written by the server after its boot check, after every recorded removal and at clean stop in one durable write; a test fails when a store opened at startup has no entry or class.
> - C778 — Every boot and identity act compares the stores to the receipt before serving or changing anything: a lost whole store (directory log, grant log and identity, certificates, organisation configuration, service key, foreign SpiceDB projection) refuses the boot and the server serves only /health and the sign-in page saying refused_to_start with the store, both heads, the exact accept-loss command and the computer; a lost own store refuses only its routes with the loss sentence; a projection behind the log is re-projected; an honest shrink (revoke, a removal through the receipt's function) boots; a counted store is never created empty; the only way past a loss is lys identity accept-loss naming the store, recorded in install/stores.accepted.json.
> - C779 — keys/service.key, run/runner-server.pub, keys/runner-machine.key, run/runner-join.json, network.json, network.joins.json and data/runner/ are kept, moved and restored as one set by keep and put-back; install refuses runner_join_present rather than overwrite run/runner-server.pub under a saved join, and never mints a service key the receipt counts.
> - C780 — While a store has an accepted loss, NotHeld, the agent signature refusal, the runner refusals and machine_not_joined carry 'Lys lost its ... records at <rfc3339>' and what to do again; an accepted loss on the grant log advances its identity's epoch and the grant stream answers an old-epoch cursor Reset{Rollback} even after the head has grown.
> - C781 — ADR-114's decision says data is kept in data.previous, a put-back moves the window's data to data.rolled-back and removes nothing, and a shrunk store is refused by name until accepted; DESIGN.md renders the same.
> **Stories:**
> - S445 (Operator, Installs and runs the standalone identity product) — As the operator of a Lys install, I want every grant, certificate and computer join to survive every install, upgrade and put-back, or Lys to refuse to start and name the store that is short, so that I never again learn of a loss from the people it refused.
> - S446 (Operator, Installs and runs the standalone identity product) — As the operator of a Lys install that did lose records, I want every refusal that loss causes to say so, with when and what to do again, and every connected agent to be told its grant log was rolled back, so that nobody is sent to re-check grants, certificates or joins that Lys itself dropped.

## Purpose

On 16 October 2026 the Lys install on Tom's Mac was replaced and lost, without a word from Lys, liminal's grants, the liminal-seats agent's certificate (agent-d3b5f1169f352e59b569dcdec731db2a) and the Mac's runner join. The code at b542ea2e lets that happen at four points: a put-back deletes data/ before renaming the kept copy in (crates/lys/src/identity/upgrade/data_kept.rs:104); a missing grant log, certificate log or machines file is created empty at boot without a word (grants.rs:137-142, certificates_store.rs:204-207, network_store/journal.rs:125-128) and the grant log's identity is re-established at epoch 0 (grant_changes/identity.rs:135-189); install/build.json records binaries and settings but no store's count (upgrade/record.rs:14-29); and every refusal downstream (NotHeld, "the signature does not verify", "names no dialled runner", machine_not_joined) reads as the person's fault. Tom's word of 19 October (12:04): Lys is broken and ruining everything it touches; fix Lys first. This brief carries DIRECTORY-082 R1 (unlanded; C499 and C500 unticked), adds a receipt of every store's count and digest that boot and every identity act check before serving, makes the only way past a shrink a recorded operator act naming the store, keeps the server's and the computer's join records as one set, and makes every refusal a loss causes say so, with when and what to do again.

## Task

Land DIRECTORY-082 R1's put-back that removes nothing and counts what it moved aside; write a receipt of every store's count and digest outside data/ and refuse, by store name and before serving, any shrink, empty re-creation or changed grant log identity on every boot and identity act, with a recorded operator act as the only way past; keep the join set together and never overwrite a saved join; and make NotHeld, the signature refusal, the runner refusals and the grant stream name the loss and its epoch.

## Requirements

### R1: A put-back moves the window's data aside, removes nothing, and counts it (DIRECTORY-082 R1, carried here)

Behavioural. This requirement is DIRECTORY-082 R1 as written, carried here because it is the first thing that ruins what Lys touches and has not landed (C499 and C500 stand unticked; crates/lys/src/identity/upgrade/data_kept.rs:104 still removes data/ before renaming the kept copy in). WHEN a put-back restores the kept data (swap::back from any forward failure, swap.rs:408-421, and swap::recover while upgrade.json lacks Started, swap.rs:544-549, reached from install.rs:242, install/command.rs:40 and upgrade.rs:340-342), THE SYSTEM SHALL move data/ to data.rolled-back/<rfc3339>/ (the instant the put-back began, RFC 3339 UTC) before the kept copy is renamed into place, and SHALL remove nothing under data/; the move is a rename within the install root, a name already taken is refused by name and nothing is moved. THE SYSTEM SHALL then print the folder and, for each store under it, the store's name and its count of records, read from the store's own head (FileLeafStore::open_read_only, crates/lys-log-store/src/file.rs:243, and the record count of each one-file store) without reading its history; a store it cannot count is named with the reason, never skipped. data.rolled-back is never pruned by the installer. The kept copy in data.previous stays, as restore keeps it today. The same put-back records the rollback in the acceptance file R2 names (store, count before, count after, at), so the server's next boot refuses the shrunk stores by name until an operator accepts the loss. C499 and C500 are ticked by this brief's lane when its acceptance proves them; DIRECTORY-082's rows are not duplicated.

**Acceptance:**
- A put-back leaves data/ under data.rolled-back/<rfc3339>/ byte for byte as it stood, and removes no file under data/ (DIRECTORY-082's acceptance, in crates/lys/src/identity/upgrade/data_kept_tests.rs)
- The put-back's output names the folder and one record count per store; after writes in the window and a put-back, the previous build reads every pre-window record and every window record is found and counted in data.rolled-back
- A taken data.rolled-back name is refused by name and moves nothing
- The put-back writes one rollback entry per store whose count shrank into the acceptance file, read back in the test

**Files:**
- modify: crates/lys/src/identity/upgrade/data_kept.rs
- modify: crates/lys/src/identity/upgrade/data_kept_tests.rs
- modify: crates/lys/src/identity/upgrade/swap.rs

**Checklist:**
- C499 — A put-back moves the window's data/ to data.rolled-back/<rfc3339>/ and removes none of it, and prints that folder with the count of leaves per store (DIRECTORY-082 R1).
- C500 — After a put-back that follows writes in the window, the previous build reads every pre-window leaf, and every window leaf is found and counted in data.rolled-back (DIRECTORY-082 R1).

**Stories:**
- S445 (Operator, Installs and runs the standalone identity product) — As the operator of a Lys install, I want every grant, certificate and computer join to survive every install, upgrade and put-back, or Lys to refuse to start and name the store that is short, so that I never again learn of a loss from the people it refused.

### R2: A receipt names every store's head; boot and every identity act refuse a lost, re-identified or foreign store by name; a store that may shrink honestly is compared by head, never by size; a refused boot says why on /health and the sign-in page

Behavioural. The receipt is one file outside data/, install/stores.json beside install/build.json (Layout::install_dir, crates/lys/src/identity/install/layout.rs:142-149), with one entry per store: name, the head it is compared by, a digest, read_at, and the store's class (whole or own, below). Its shape lives once, in crates/lys-identity/src/stores_receipt.rs, used by the server and the lys binary both (both depend on lys-identity). How each store is compared is decided here, per store, because a count that may fall honestly must never refuse a boot: (a) an append-only log (the directory log, the organisation configuration log, the grant log, the certificates log, requests, mcp requests, and every lys-log-store store routes_startup.rs:93-128 opens) is compared by head: its origin digest and its head index (FileLeafStore / frontier_log len, crates/lys-log-store/src/frontier_log.rs:217); a head below the receipt's, or another origin, is a loss; a head at or above it with the same origin is fine; (b) a live set whose size falls by design (SpiceDB relations on every revoke; the one-file stores machines, roles, provisioning, apps, homes, runtime, service accounts, reviews, teams, stops, budgets, policies, whose records retire and will be pruned) is never compared by size: a one-file store is compared by its journal head (the sequence its journal already keeps, network_store/journal.rs read's third value) and its origin digest; (c) SpiceDB is a projection of the grant log, so it is compared by the grant-log revision it has projected (grants.rs:88 require_model and :121 hold_model already carry a model revision; the lane records the projected grant revision beside it) against the grant log's head: behind is re-projected from the log by the server's own projection path (GrantState open with Relationships::SpiceDb, grants.rs:131-150, and lys-identity's projection rebuild, grants/projection_live.rs:105) before serving; ahead of the head, or a projection of another log identity, is refused by name (spicedb_projection_foreign, naming both revisions); (d) the install's own files, keys/service.key's and keys/runner-machine.key's public halves, run/runner-join.json, run/runner-server.pub and the data/runner/ owners directory, are compared by digest, presence only (a present receipt entry with the file absent is a loss). An act that legitimately removes records, today none in the tree (no prune or compaction removes a record from any store at b542ea2e; goals compaction_holders is not one), and from now every prune, compaction or retirement that Tom's store-growth work lands, updates the receipt in the same durable operation that removes them, through the one function crates/lys-identity/src/stores_receipt.rs exports for it; a removal that bypasses it is a loss at the next boot, which is the point. The grant log's entry also carries the identity and epoch grant_changes/identity.rs records; another identity is refused (grant_log_identity_changed, naming both). A test fails when a store opened in routes_startup.rs has no entry and no class. The server writes the receipt after its boot check, after every removal (above) and at clean stop, each in one durable write (write, fsync, rename); the lys binary never writes it. Scope of a refusal, decided per store with the reason: a loss of a WHOLE store refuses the boot, because every route depends on it and serving without it would answer wrongly to everyone: the directory log (every identity), the grant log and its identity (every permission), the certificates log with agent-keys (every agent signature), the organisation configuration log (zones and sign-in), keys/service.key (signs every receipt, join and certificate) and a foreign SpiceDB projection. A loss of an OWN store refuses only its own routes, with the loss sentence of R4, while every other route serves, because a convenience store must never take Lys down and the person needs Lys up to repair it: machines with network.joins.json (the network routes and starts on machines refuse; people still sign in and reconnect computers), roles, provisioning, apps, homes, runtime, service accounts, reviews, teams, stops, budgets, policies, requests, mcp requests, and the runner's own files. At boot, before open_directory (routes_startup.rs:93), the server reads the receipt and compares every store it is about to open: a whole store lost (head below, origin changed, identity changed, projection foreign, or a counted store absent) refuses the boot; an own store lost is recorded and its routes refuse from then on. A counted store that is absent is never created empty: grants.rs:137-142, certificates_store.rs:204-207, network_store/journal.rs:125-128 and identity.rs establish's minting at epoch 0 run only for a store the receipt does not name. A refused boot is not Lys down silently: the server binds its listener and serves exactly two things, GET /health (health_api.rs:36), which answers refused_to_start with the store, what the receipt holds and what was found, the exact command (lys identity accept-loss --store <the store named>) and the computer to run it on (this host's name), and the sign-in page (surface/identity/src/features/sign-in/SignIn.tsx), which reads /health and shows those words instead of the form; every other route answers 503 with the same words; the startup lines say the same by store name. The only way past a loss is the recorded operator act: lys identity accept-loss --store <name> [--store <name>]..., which appends to install/stores.accepted.json one entry per named store (store, what the receipt held, what was found, at, the login's user name) and prints each; a store not named is not accepted. At the next boot a store with an acceptance newer than the receipt's read_at is served at what was found, the receipt is rewritten to it, and the acceptance's at becomes that store's loss epoch (R4). Starting a store empty is the same act at nothing found; there is no flag, environment variable or configuration key that skips the check. The lys binary makes the same comparison, for the stores it can read read-only, in every identity act (install, upgrade before data is kept, put-back, recover, stop, start) and refuses by the same names before changing anything. The receipt and acceptance file paths are two new server configuration keys written by the installer (install/server_config/rendering.rs, read in crates/lys-identity-server/src/config.rs); a missing key is written into the configuration by the program that reads it, never defaulted in code (Tom, 12 October 2026).

**Acceptance:**
- Red first, in a new crates/lys-identity-server/src/../tests/stores_receipt.rs against the crate's server fixture: after grants, a certificate and a machine join are written and the server stops, (a) data/ replaced by an empty directory makes the next boot refuse naming the directory log, the grant log and the certificates store as whole stores, creates no empty store, and serves GET /health as refused_to_start with the store, the receipt's head, what was found, the accept-loss command and this host's name, and the sign-in page's words say the same while every other route answers 503 with them; (b) the grant log truncated by one leaf by the test's own fixture makes the next boot refuse naming grant-log with both heads; (c) a grant log identity file rewritten makes the next boot refuse grant_log_identity_changed naming both identities
- In the same file, honest shrink: a revoke that lowers the SpiceDB relation count, then a boot, serves; a SpiceDB projection behind the grant log's head is re-projected before serving and serves; a projection recorded at a revision ahead of the head, or under another log identity, is refused spicedb_projection_foreign naming both; a record removed from a one-file store through the receipt's removal function, then a boot, serves, and the same removal made without it is refused at the next boot naming the store (the prune test Waffles asked for, driven through the one road every prune must take)
- In the same file, scope: the machines file absent with every whole store intact boots and serves sign-in, /me and grants; GET /network and a start on a machine refuse with the R4 loss sentence; after accept-loss --store network the next boot serves them
- In the same file: an acceptance entry for exactly the named store lets the next boot serve it and rewrites the receipt; a store not named in the acceptance still refuses
- In crates/lys-identity/src/stores_receipt_tests.rs: the receipt round-trips, a write is one rename, every store routes_startup.rs opens is named with a class (the test lists them from the server's own store-opening table), and the per-store comparison rules above are each exercised (head below, same head other origin, head above, projection behind, projection ahead, digest presence)
- In a new crates/lys/src/identity/upgrade/stores_receipt_tests.rs: lys identity install and upgrade on a root whose receipt names a store the disk lacks refuse by store name and change nothing; accept-loss writes exactly the named entries and prints them
- gate -stages nextest -crates lys-identity-server,lys-identity,lys -test 'package(lys-identity-server) | package(lys-identity) | package(lys)' green, counts in the report; surface/identity typecheck, biome and vitest green for SignIn.tsx

**Files:**
- create: crates/lys-identity/src/stores_receipt.rs
- create: crates/lys-identity/src/stores_receipt_tests.rs
- create: crates/lys-identity-server/src/stores_receipt.rs
- create: crates/lys-identity-server/src/boot_refused.rs
- create: crates/lys-identity-server/tests/stores_receipt.rs
- create: crates/lys/src/identity/accept_loss.rs
- create: crates/lys/src/identity/upgrade/stores_receipt_tests.rs
- modify: crates/lys-identity/src/lib.rs
- modify: crates/lys-identity-server/src/routes_startup.rs
- modify: crates/lys-identity-server/src/health_api.rs
- modify: crates/lys-identity-server/src/config.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/certificates_store.rs
- modify: crates/lys-identity-server/src/network_store/journal.rs
- modify: crates/lys-identity-server/src/grant_changes/identity.rs
- modify: surface/identity/src/features/sign-in/SignIn.tsx
- modify: crates/lys/src/identity/cli.rs
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys/src/identity/install/layout.rs
- modify: crates/lys/src/identity/install/server_config/rendering.rs
- modify: crates/lys/src/identity/upgrade/swap.rs

**Checklist:**
- C777 — install/stores.json records every store the server opens, the SpiceDB projection and the install's keys and join files, each by the head it is honestly compared by (append-only logs by origin and head index, one-file stores by journal head, SpiceDB by projected grant revision, keys and join files by digest and presence) and by its class (whole or own), written by the server after its boot check, after every recorded removal and at clean stop in one durable write; a test fails when a store opened at startup has no entry or class.
- C778 — Every boot and identity act compares the stores to the receipt before serving or changing anything: a lost whole store (directory log, grant log and identity, certificates, organisation configuration, service key, foreign SpiceDB projection) refuses the boot and the server serves only /health and the sign-in page saying refused_to_start with the store, both heads, the exact accept-loss command and the computer; a lost own store refuses only its routes with the loss sentence; a projection behind the log is re-projected; an honest shrink (revoke, a removal through the receipt's function) boots; a counted store is never created empty; the only way past a loss is lys identity accept-loss naming the store, recorded in install/stores.accepted.json.

**Stories:**
- S445 (Operator, Installs and runs the standalone identity product) — As the operator of a Lys install, I want every grant, certificate and computer join to survive every install, upgrade and put-back, or Lys to refuse to start and name the store that is short, so that I never again learn of a loss from the people it refused.

### R3: The server's join records and the computer's saved join are one set: kept, moved and restored together, and never overwritten under a saved join

Behavioural. The join set is keys/service.key, run/runner-server.pub (the service key's public half, which install.rs:261-262 writes from service_key(layout) through start_runner), keys/runner-machine.key, run/runner-join.json (crates/lys/src/commands/runner_join.rs:86), data/network.json and data/network.joins.json (network_join.rs:181-183), and data/runner/. An upgrade's keep (data_kept.rs keep) copies the whole set beside data.previous, and a put-back (R1) moves and restores the whole set together, so a machine's record on the server and the computer's saved connection to it never come from two different moments. start_runner (install.rs:168-188) writes run/runner-server.pub only when no run/runner-join.json exists on this computer; where one exists, the install refuses runner_join_present, naming the file and the server it joins, and changes nothing: a computer that joined another Lys is never silently turned into a server of its own. keys/service.key is never regenerated where one exists (load_or_generate, install.rs:119, kept); where the receipt (R2) counts a service key and none is on disk, the install refuses store_missing naming keys/service.key instead of minting one, because every joined computer's saved runner-server.pub names the old key.

**Acceptance:**
- In crates/lys/src/identity/upgrade/data_kept_tests.rs: after a keep and a put-back with writes to network.json and a new join code in the window, network.json, network.joins.json, run/runner-join.json and run/runner-server.pub are all from the kept moment, and the window's copies are all under data.rolled-back
- In a new crates/lys/src/identity/install/runner_join_guard_tests.rs: lys identity install on a root holding run/runner-join.json is refused runner_join_present naming the file and the server, and run/runner-server.pub is byte for byte as before; a root with no saved join writes it as today
- In the same file: a root whose receipt counts keys/service.key with the key removed is refused store_missing naming keys/service.key, and no key is generated

**Files:**
- create: crates/lys/src/identity/install/runner_join_guard_tests.rs
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys/src/identity/upgrade/data_kept.rs
- modify: crates/lys/src/identity/upgrade/data_kept_tests.rs

**Checklist:**
- C779 — keys/service.key, run/runner-server.pub, keys/runner-machine.key, run/runner-join.json, network.json, network.joins.json and data/runner/ are kept, moved and restored as one set by keep and put-back; install refuses runner_join_present rather than overwrite run/runner-server.pub under a saved join, and never mints a service key the receipt counts.

**Stories:**
- S445 (Operator, Installs and runs the standalone identity product) — As the operator of a Lys install, I want every grant, certificate and computer join to survive every install, upgrade and put-back, or Lys to refuse to start and name the store that is short, so that I never again learn of a loss from the people it refused.

### R4: Every refusal a lost store causes says so, with when, and what to do again; the grant stream resets a client whose log was rolled back

Behavioural. The server holds, from the acceptance file (R2), each store's loss epoch: the at of its newest acceptance. While a store has a loss epoch, the refusals its loss causes carry one more sentence naming it: NotHeld from the grant check (grants.rs:263-267) ends "Lys lost its grant records at <rfc3339>; ask for the grant again"; AgentSignatureRefused's "the signature does not verify" (agent_signature.rs:106-111), when the certificates store has a loss epoch, ends "Lys lost its certificate records at <rfc3339>; issue the agent's certificate again"; runner_dial_refused's "names no dialled runner" (runner_dial.rs:85 and :204) and the runner view GET /network/machines/{id}/runner, when the machines store has a loss epoch, carry lost_at, and machine_not_joined in crates/lys/src/commands/launch_device.rs:92-95 and :185-188 ends "Lys lost its computer records at <rfc3339>; connect this computer again" when the view carries it. The sentence is added to the words, never in place of the refusal's name, and is absent for a store with no loss. Accepting a loss on the grant log replaces the grant log identity through grant_changes/identity.rs reset (the next epoch, naming the one it replaces), and grant_changes/stream.rs answers a request whose log names the same identity at an earlier epoch with ChangeFrame::Reset{reason: Rollback, from: the asked log}, not OtherLog, whether or not the head has since grown past the request's cursor (today Rollback is answered only while after > head, stream.rs:155-161, so a client that slept through a put-back and new appends resumes a cursor into a log that is not the one it read).

**Acceptance:**
- In crates/lys-identity-server/tests/stores_receipt.rs: after an accepted loss on the grant log, a check for a grant written in the window is refused NotHeld with the loss sentence and its rfc3339; the same check on an install with no loss carries no such sentence
- In the same file: after the accepted loss and three new appends, a grant-changes request naming the old epoch with a cursor below the new head is answered Reset{Rollback} naming the asked log; a request naming the new epoch is answered from its cursor
- In the same file: an agent whose certificate was in the lost window is refused with the certificate loss sentence; a machine whose join was in the lost window answers lost_at in its runner view
- In crates/lys/src/commands/launch_device_tests.rs: a runner view carrying lost_at makes machine_not_joined end with the computer loss sentence

**Files:**
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/agent_signature.rs
- modify: crates/lys-identity-server/src/runner_dial.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: crates/lys-identity-server/src/openapi_runner_types.rs
- modify: crates/lys-identity-server/src/grant_changes/identity.rs
- modify: crates/lys-identity-server/src/grant_changes/stream.rs
- modify: crates/lys-identity-server/src/stores_receipt.rs
- modify: crates/lys/src/commands/launch_device.rs
- modify: crates/lys/src/commands/launch_device_tests.rs

**Checklist:**
- C780 — While a store has an accepted loss, NotHeld, the agent signature refusal, the runner refusals and machine_not_joined carry 'Lys lost its ... records at <rfc3339>' and what to do again; an accepted loss on the grant log advances its identity's epoch and the grant stream answers an old-epoch cursor Reset{Rollback} even after the head has grown.

**Stories:**
- S446 (Operator, Installs and runs the standalone identity product) — As the operator of a Lys install that did lose records, I want every refusal that loss causes to say so, with when and what to do again, and every connected agent to be told its grant log was rolled back, so that nobody is sent to re-check grants, certificates or joins that Lys itself dropped.

### R5: ADR-114 says what the code does

Documentary. ADR-114's decision (docs/design/decisions.json) says an upgrade never touches data and that the previous build comes back; since DIRECTORY-045 R7 and DIRECTORY-082 the upgrade keeps data/ whole in data.previous and a put-back restores that copy, and from this brief a put-back moves the window's data to data.rolled-back, removes nothing, and the server refuses to serve a shrunk or missing store until an operator accepts the loss by name. The ADR's decision text is amended to say exactly that, in one added sentence after "Data, credentials, configuration and the compose services are never touched by an upgrade", and a consequence is added: "A put-back never loses a record silently: the window's data is kept beside the install and every shrunk store is refused by name until accepted." DESIGN.md is re-rendered by render-cluster.py so its ADR-114 line (docs/design/directory/DESIGN.md:74) matches. Nothing else in the ADR changes. For Waffles, not for the lane: C393 and the structure rows at DESIGN.md:700 and :792 name `lys identity upgrade --back`, crates/lys/src/identity/upgrade/back.rs and upgrade_back_tests.rs; none of the three is in the tree at b542ea2e and cli.rs has no --back, so those rows describe work that never landed.

**Acceptance:**
- docs/design/decisions.json ADR-114 carries the added sentence and consequence; python3 scripts/design/validate.py docs/design/decisions.json prints no violation
- sh scripts/design/gate.sh prints no 'rendered markdown differs' line for docs/design/directory

**Files:**
- modify: docs/design/decisions.json
- modify: docs/design/directory/DESIGN.md

**Checklist:**
- C781 — ADR-114's decision says data is kept in data.previous, a put-back moves the window's data to data.rolled-back and removes nothing, and a shrunk store is refused by name until accepted; DESIGN.md renders the same.

**Stories:**
- S446 (Operator, Installs and runs the standalone identity product) — As the operator of a Lys install that did lose records, I want every refusal that loss causes to say so, with when and what to do again, and every connected agent to be told its grant log was rolled back, so that nobody is sent to re-check grants, certificates or joins that Lys itself dropped.

## Boundaries

- No store is ever created empty, re-identified or served short of its receipt without a recorded operator act that names it; no flag, environment variable or configuration key skips the check
- No store is compared by the size of a live set: append-only logs by head, projections by projected revision, sets by journal head and digest; a boot never refuses an honest shrink (revoke, prune, retirement recorded through the receipt)
- A refused boot is never silent: /health and the sign-in page say the store, both heads, the command and the computer; losing an own store never takes Lys down
- The installer prunes nothing: data.previous and data.rolled-back stay until a person removes them
- The receipt is written only by the server, in one durable write; the lys binary reads it and writes acceptances only
- No fixed wait, timer, poll or deadline anywhere, in code or test (Tom, 15 October 2026)
- No store's format moves: the receipt sits beside the stores and reads their existing heads
- Nothing installed by the lane; every file touched stays under 500 code lines (ADR-111, by scripts/file-length.sh)

## Verification

- gate -stages nextest -crates lys-identity-server,lys-identity,lys -test 'package(lys-identity-server) | package(lys-identity) | package(lys)'
- gate -stages clippy, then the whole declared gate
- sh scripts/design/gate.sh read for FAIL and 'rendered markdown differs' lines (DESIGN.md re-rendered for R5)
- python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory read for FAIL lines
