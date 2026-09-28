---
type: brief
id: DIRECTORY-043
cluster: directory
title: Identity server hot paths: indexed runtime state, no snapshot clones, no blocking I/O on async workers, one pass per memory view, receipts without re-reading the log
---

# DIRECTORY-043: Identity server hot paths: indexed runtime state, no snapshot clones, no blocking I/O on async workers, one pass per memory view, receipts without re-reading the log

> **Cluster:** directory
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C334 — Runtime state is indexed (DIRECTORY-043 R1), proved by a counting test that fails at the base.
> - C335 — Snapshots serialise by reference (DIRECTORY-043 R2), proved by a counting test that fails at the base.
> - C336 — Directory writes never block an async worker (DIRECTORY-043 R3), proved by a counting test that fails at the base.
> - C337 — A session listing locks once and clones nothing it does not return (DIRECTORY-043 R4), proved by a counting test that fails at the base.
> - C338 — A memory view reads each session once, off the async worker (DIRECTORY-043 R5), proved by a counting test that fails at the base.
> - C339 — A receipt is rebuilt from the stored coordinate, not by re-reading leaves (DIRECTORY-043 R6), proved by a counting test that fails at the base.
> **Stories:**
> - S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Purpose

The identity server answers every agent's runtime reports, directory writes, memory views and agent views. It scans every session ever reported on each report, deep-copies whole stores to snapshot them, runs fsync on async workers under a mutex every route waits on, reads every session twice per memory view, and re-reads up to 1 024 leaf files to rebuild one receipt.

## Task

Fix the six findings below in crates/lys-identity-server and crates/lys-identity/src/restart.rs, each proved by a counting test that fails at the base. Where a file named here has moved or been split on main since 9712505, the fix follows the code to where it now lives and the dev record names the new path.

## Requirements

### R1: Runtime state is indexed

Behavioural. crates/lys-identity-server/src/runtime_state.rs:142, :148, :157, runtime_store.rs:205. Held::operation() walks every report of every session ever kept on each incoming report, and session() and hold() scan every session. Keep maps from operation id and session id to slot beside sessions, serde(skip), rebuilt once in decode(), fold() and settle(), updated in hold(); equality compares sessions only.

**Acceptance:**
- A test with 10 000 kept sessions shows one report visits 1 session, where the base visits all 10 000.
- The stored snapshot bytes equal the base's.

**Files:**
- modify: crates/lys-identity-server/src/runtime_state.rs
- modify: crates/lys-identity-server/src/runtime_store.rs

**Checklist:**
- C334 — Runtime state is indexed (DIRECTORY-043 R1), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The runtime state keeps a session-to-slot map and an operation-to-(slot, report) map beside the list it always had. Both maps are built when a state is decoded or a session is kept, so a report looks up its session instead of scanning every one. The serialised shape is the sessions list alone, so the stored bytes are unchanged. A visited counter counts the sessions a lookup touches, which proves the work is 1 and not N.
- Deviation: This round changed only the rendered markdown of the brief. The design gate failed with 'rendered markdown differs from the committed file: docs/design/directory/./briefs/DIRECTORY-043.md' because the JSON held review blocks the markdown lacked. The markdown was replaced with the render of the JSON.
- Files changed:
  - modified: `crates/lys-identity-server/src/runtime_state.rs` — Held keeps its sessions list and adds two indexes, by_session and by_operation, which are rebuilt on decode, never serialised and excluded from equality. It also has a visited counter. find/slot/operation answer by lookup, and keep writes into the slot find returned.
  - modified: `crates/lys-identity-server/src/runtime_store.rs` — report finds the session once through Held::find and appends to that slot; tracked(slot) and visited() are added.
  - created: `crates/lys-identity-server/src/runtime_store_tests.rs` — 10 000 kept sessions: one report visits 1.
  - created: `crates/lys-identity-server/src/runtime_state_tests.rs` — A state read back is indexed and seals the same bytes as a mirror of the base's {sessions} shape.
  - modified: `docs/design/directory/briefs/DIRECTORY-043.md` — Re-rendered from DIRECTORY-043.json so the committed markdown equals the render the design gate compares against.
- Checklist delivery:
  - [x] C334 — Runtime state is indexed (DIRECTORY-043 R1), proved by a counting test that fails at the base. — one_report_among_ten_thousand_sessions_visits_one asserts visited == 1.
- Story delivery:
  - [x] S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows. — The report path is indexed; answers and stored bytes are unchanged.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test with 10 000 kept sessions shows one report visits 1 session, where the base visits all 10 000. — crates/lys-identity-server/src/runtime_store_tests.rs one_report_among_ten_thousand_sessions_visits_one asserts visited()-before == 1 for a report among KEPT=10_000. It passed in my cargo test --workspace --all-features run (exit 0). Round 1 recorded the base failure: 'left: 10000, right: 1'. Held::slot/find (runtime_state.rs) look up by HashMap, and RuntimeStore::report finds the slot once and appends into it.
  - [x] The stored snapshot bytes equal the base's. — runtime_state_tests.rs a_state_read_back_is_indexed_and_seals_the_same_bytes asserts encode() == BaseSealed{format, BaseHeld{sessions}}, a mirror of the base's declared shape, and that no index field appears in the JSON. The indexes are #[serde(skip)], and Held::decode rebuilds them via index().
- Checklist verified: C334
- Stories verified: S145

### R2: Snapshots serialise by reference

Behavioural. runtime_state.rs:200, requests_state.rs:70, certificates_store.rs:163, teams_state.rs:246, stops_state.rs:121, service_accounts_state.rs:169, reviews_state.rs:95. Each encode builds Sealed { held: self.clone() }, a deep copy of the whole folded state, only to serialise it. Serialise a borrowing SealedRef<'a> { format, held: &'a Held } and keep the owned Sealed for decode only.

**Acceptance:**
- For each of the seven stores a test shows the encoded bytes are identical to the base's for its existing fixture.
- grep over the seven files for 'held: self.clone()' prints nothing.

**Files:**
- modify: crates/lys-identity-server/src/runtime_state.rs
- modify: crates/lys-identity-server/src/requests_state.rs
- modify: crates/lys-identity-server/src/certificates_store.rs
- modify: crates/lys-identity-server/src/teams_state.rs
- modify: crates/lys-identity-server/src/stops_state.rs
- modify: crates/lys-identity-server/src/service_accounts_state.rs
- modify: crates/lys-identity-server/src/reviews_state.rs

**Checklist:**
- C335 — Snapshots serialise by reference (DIRECTORY-043 R2), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each of the seven stores used to seal a snapshot by cloning its whole held state into an owned Sealed struct. It now serialises a SealedRef that borrows the format and the held state. The field order and names are those of the owned struct, which stays for decoding. Each store has a test that its sealed bytes equal those of an owned copy of the base's shape.
- Deviation: The byte-identity tests live in sibling *_tests.rs files, which begin with #![cfg(test)] and are linked with #[cfg(test)] #[path] mod tests;, following CLAUDE.md's rule for test placement.
- Files changed:
  - modified: `crates/lys-identity-server/src/runtime_state.rs` — encode seals a borrowed SealedRef instead of cloning the held state.
  - modified: `crates/lys-identity-server/src/requests_state.rs` — The same borrowed seal; tests linked from a sibling file.
  - modified: `crates/lys-identity-server/src/certificates_store.rs` — The same borrowed seal; tests linked from a sibling file.
  - modified: `crates/lys-identity-server/src/teams_state.rs` — The same borrowed seal; tests linked from a sibling file.
  - modified: `crates/lys-identity-server/src/stops_state.rs` — The same borrowed seal; tests linked from a sibling file.
  - modified: `crates/lys-identity-server/src/service_accounts_state.rs` — The same borrowed seal; tests linked from a sibling file.
  - modified: `crates/lys-identity-server/src/reviews_state.rs` — The same borrowed seal; tests linked from a sibling file.
  - created: `crates/lys-identity-server/src/requests_state_tests.rs` — The sealed bytes equal an owned copy's.
  - created: `crates/lys-identity-server/src/certificates_store_tests.rs` — The sealed bytes equal an owned copy's.
  - created: `crates/lys-identity-server/src/teams_state_tests.rs` — The sealed bytes equal an owned copy's.
  - created: `crates/lys-identity-server/src/stops_state_tests.rs` — The sealed bytes equal an owned copy's.
  - created: `crates/lys-identity-server/src/service_accounts_state_tests.rs` — The sealed bytes equal an owned copy's.
  - created: `crates/lys-identity-server/src/reviews_state_tests.rs` — The sealed bytes equal an owned copy's.
- Checklist delivery:
  - [x] C335 — Snapshots serialise by reference (DIRECTORY-043 R2), proved by a counting test that fails at the base. — The seven byte-identity tests pass; 'held: self.clone()' no longer appears in the seven files.
- Story delivery:
  - [x] S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows. — Snapshots are sealed with no clone of the state, and the stored bytes are unchanged.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] For each of the seven stores a test shows the encoded bytes are identical to the base's for its existing fixture. — a_snapshot_seals_the_bytes_an_owned_copy_sealed is in certificates_store_tests.rs, requests_state_tests.rs, teams_state_tests.rs, stops_state_tests.rs, service_accounts_state_tests.rs and reviews_state_tests.rs. Those Held types are unchanged from base, so the owned Copied{format, held} is the base's Sealed serialisation. The runtime store is covered by runtime_state_tests.rs against the base-shape mirror. All pass in the workspace run (exit 0).
  - [x] grep over the seven files for 'held: self.clone()' prints nothing. — grep -n "held: self.clone()" over runtime_state.rs, requests_state.rs, certificates_store.rs, teams_state.rs, stops_state.rs, service_accounts_state.rs and reviews_state.rs printed nothing (exit 1).
- Checklist verified: C335
- Stories verified: S145

### R3: Directory writes never block an async worker

Behavioural. crates/lys-identity-server/src/routes.rs:231. with_directory() takes a std Mutex and runs the mutation, a leaf file write with fsync and on cadence a snapshot, inside the async handler, so every directory route waits behind one fsync on a tokio worker. Run the locked section on a single writer thread fed by a channel (or spawn_blocking), with reads served without waiting on a write in progress where the state allows it.

**Acceptance:**
- A test holds a directory write inside a blocked counting filesystem and shows a concurrent directory read route answers before the write is released.
- No std::sync::MutexGuard is held across an .await in routes.rs, shown by clippy's await_holding_lock lint enabled for the crate and green.

**Files:**
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C336 — Directory writes never block an async worker (DIRECTORY-043 R3), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Blocking directory I/O leaves the async worker by block_in_place, because the existing closures borrow and so cannot be sent to spawn_blocking. Readers no longer wait behind a writer holding the disk: they answer from the last settled projection. No MutexGuard is held across an await, and the crate-level deny of clippy's await_holding_lock keeps it that way.
- Deviation: R3 uses block_in_place instead of spawn_blocking, because the directory closures are not 'static. Only the list and read routes read from the published projection.
- Files changed:
  - created: `crates/lys-identity-server/src/directory_cell.rs` — DirectoryCell holds the writer behind one mutex and a published projection behind another. Writes run through block_in_place on a multi-thread runtime and inline otherwise. Reads answer from the published projection, which a write withdraws when it changes the folded count.
  - created: `crates/lys-identity-server/src/directory_cell_tests.rs` — A gated, counting file store holds a write on the disk while a read answers.
  - modified: `crates/lys-identity-server/src/routes.rs` — AppState holds a DirectoryCell. with_directory goes through write, and the list and read routes go through read_directory. The directory opens with its leaf-roots file.
  - modified: `crates/lys-identity-server/src/lib.rs` — Adds #![deny(clippy::await_holding_lock)] and the directory_cell module.
- Checklist delivery:
  - [x] C336 — Directory writes never block an async worker (DIRECTORY-043 R3), proved by a counting test that fails at the base. — a_read_answers_while_a_write_is_held_on_the_disk passes, and await_holding_lock is denied crate-wide.
- Story delivery:
  - [x] S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows. — Directory reads are not blocked behind a write held on the disk.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test holds a directory write inside a blocked counting filesystem and shows a concurrent directory read route answers before the write is released. — directory_cell_tests.rs a_read_answers_while_a_write_is_held_on_the_disk holds register_person inside Gated::put_leaf (asserts puts == 1). It reads 0 identities while !gate.released(), then reads 1 after the write returns. The list and read routes (routes.rs list/read) call read_directory → DirectoryCell::read, the function under test. It passed in the workspace run. Round 1 recorded that with the base behaviour (read through the writer lock) the test never completes.
  - [x] No std::sync::MutexGuard is held across an .await in routes.rs, shown by clippy's await_holding_lock lint enabled for the crate and green. — crates/lys-identity-server/src/lib.rs has #![deny(clippy::await_holding_lock)]. The round's clippy and clippy-all-features legs exited 0 with -D warnings.
- Checklist verified: C336
- Stories verified: S145

### R4: A session listing locks once and clones nothing it does not return

Behavioural. crates/lys-identity-server/src/runtime_api.rs:209, network_store.rs:146, runtime_store.rs:252. view() locks the network mutex, searches machines linearly and clones a whole Machine per session listed, and report() returns standing(), a clone of the whole Tracked with its launch JSON, which launch_api clones again. Lock the network store once per listing and build id to (name, runtime) once; have report() answer a view built from borrowed data.

**Acceptance:**
- A test listing 500 sessions shows 1 network lock acquisition, where the base takes 500.
- Listing and report answers equal the base's.

**Files:**
- modify: crates/lys-identity-server/src/runtime_api.rs
- modify: crates/lys-identity-server/src/network_store.rs
- modify: crates/lys-identity-server/src/runtime_store.rs
- modify: crates/lys-identity-server/src/launch_api.rs

**Checklist:**
- C337 — A session listing locks once and clones nothing it does not return (DIRECTORY-043 R4), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: A listing used to lock the machines store once per session to name its machine. It now takes one lock, copies every machine's name and runtime into a map, and releases the lock before building the views. The lock counts its acquisitions so the test can assert 1 for 500 sessions.
- Deviation: launch_api.rs and stop_api.rs are outside the brief's named files; they changed only to fit the new report and lock signatures.
- Files changed:
  - modified: `crates/lys-identity-server/src/network_store.rs` — NetworkLock counts the times it is taken. names() returns every machine's name and runtime in one pass.
  - modified: `crates/lys-identity-server/src/network_api.rs` — with_network takes the lock through NetworkLock.
  - modified: `crates/lys-identity-server/src/runtime_api.rs` — A listing takes the machines lock once and views each session from the resulting map; a report takes it once.
  - created: `crates/lys-identity-server/src/runtime_api_tests.rs` — 500 sessions take the lock once, and each view equals the base-shaped JSON.
  - modified: `crates/lys-identity-server/src/launch_api.rs` — Clones only the launch JSON.
  - modified: `crates/lys-identity-server/src/stop_api.rs` — Adapts to report returning the tracked session.
- Checklist delivery:
  - [x] C337 — A session listing locks once and clones nothing it does not return (DIRECTORY-043 R4), proved by a counting test that fails at the base. — a_listing_of_five_hundred_sessions_takes_the_machines_lock_once asserts taken == 1.
- Story delivery:
  - [x] S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows. — One lock per listing; answers are unchanged.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test listing 500 sessions shows 1 network lock acquisition, where the base takes 500. — runtime_api_tests.rs a_listing_of_five_hundred_sessions_takes_the_machines_lock_once asserts network.taken() == 1 after listing 501 sessions, and 2 after one report answer. NetworkLock::lock (network_store.rs) counts each acquisition. Round 1 recorded the base-shaped failure 'left: 501, right: 1'. It passed in the workspace run.
  - [x] Listing and report answers equal the base's. — The same test compares a whole SessionView JSON to the base-shaped object, with null machine_name/runtime for an unknown machine. names() keeps the first machine per id via or_insert, as machine()'s find did. The existing HTTP suites (runtime_reports, start_command, emergency_stop) pass unchanged in cargo test -p lys-identity-server --all-features (exit 0).
- Checklist verified: C337
- Stories verified: S145

### R5: A memory view reads each session once, off the async worker

Behavioural. crates/lys-identity-server/src/memory_api.rs:203, lys-home recall.rs:160, given.rs:244. GET /agents/{id}/memory calls recall_all and then last_given, each opening and parsing every session of the home with blocking std fs calls in the async handler. Add a lys-home function that derives both lanterns and last-given from one reader per session in one pass, and run it on spawn_blocking.

**Acceptance:**
- A test with a home of 20 sessions shows the route opens each session file once, where the base opens each twice.
- The view's answer equals the base's.

**Files:**
- modify: crates/lys-identity-server/src/memory_api.rs
- modify: crates/lys-home/src/recall.rs
- modify: crates/lys-home/src/given.rs

**Checklist:**
- C338 — A memory view reads each session once, off the async worker (DIRECTORY-043 R5), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The memory view used to read the home twice, once for recall_all and once for last_given. It now makes one pass that folds every session into both answers, and it runs on the blocking pool instead of an async worker.
- Deviation: The change reaches into lys-home, the crate that owns the readers.
- Files changed:
  - modified: `crates/lys-home/src/record/recall.rs` — recall_all_and_last_given reads each session once and answers both views. recall_and_given_with takes the opener, so the test can count opens.
  - modified: `crates/lys-home/src/record/given.rs` — given_of is crate-visible, and keep_last is added.
  - modified: `crates/lys-home/src/lib.rs` — Re-exports recall_all_and_last_given.
  - modified: `crates/lys-home/src/record/given_last_tests.rs` — 20 sessions plus one broken: each is opened once, and the answers equal recall_all and last_given.
  - modified: `crates/lys-identity-server/src/memory_api.rs` — The memory view runs the one-pass reader on spawn_blocking.
- Checklist delivery:
  - [x] C338 — A memory view reads each session once, off the async worker (DIRECTORY-043 R5), proved by a counting test that fails at the base. — one_pass_opens_each_session_once_and_answers_as_the_two_readings passes.
- Story delivery:
  - [x] S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows. — One pass per memory view, with answers unchanged.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test with a home of 20 sessions shows the route opens each session file once, where the base opens each twice. — crates/lys-home/src/record/given_last_tests.rs one_pass_opens_each_session_once_and_answers_as_the_two_readings counts 21 sessions (20 plus one broken), each opened once. memory_api.rs runs recall_all_and_last_given, which is recall_and_given_with over home.read_session, inside tokio::task::spawn_blocking. Round 1 recorded the injected-second-open failure at given_last_tests.rs:125.
  - [x] The view's answer equals the base's. — The same test asserts recalled == recall_all(&home) and last == last_given(&home), including the broken session skipped by both. tests/memory.rs passes in the workspace run.
- Checklist verified: C338
- Stories verified: S145

### R6: A receipt is rebuilt from the stored coordinate, not by re-reading leaves

Behavioural. crates/lys-identity/src/restart.rs:328, read_api.rs:233, directory.rs:165 and :395, commit.rs:158. Ledger::entry() reads every leaf file from the nearest 1 024-leaf checkpoint and rehashes each to rebuild one receipt, on every agent view, every retried operation (twice in transition()) and for grants. Keep each leaf's coordinate (index, tree size, root, 32 bytes) when it is folded, beside the log, so entry() reads one leaf and verifies it; in transition() reuse the first answered() result.

**Acceptance:**
- A test with a 5 000-leaf ledger shows entry() for leaf 4 000 reads 1 leaf file, where the base reads up to 1 024.
- The receipts equal the base's byte for byte, and a coordinate store removed from disk is rebuilt from the log on open and gives the same receipts.

**Files:**
- modify: crates/lys-identity/src/restart.rs
- modify: crates/lys-identity-server/src/read_api.rs
- modify: crates/lys-identity/src/directory.rs

**Checklist:**
- C339 — A receipt is rebuilt from the stored coordinate, not by re-reading leaves (DIRECTORY-043 R6), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each appended leaf's tree root is written to a roots file beside the log, and the file is synced before a snapshot. A receipt reads its one leaf and that stored root instead of re-folding from the nearest checkpoint. The file is local derived state, never signed or on the wire, and every refusal is logged by name and rebuilt from the log.
- Deviation: (none)
- Files changed:
  - created: `crates/lys-identity/src/roots.rs` — A local file of 32-byte leaf roots beside the log, checked against the last checkpoint and the snapshot's root on open. A file that is missing, short or refused is rebuilt from the log, by name.
  - modified: `crates/lys-identity/src/restart.rs` — The ledger keeps the roots. entry() rebuilds a receipt from one leaf and its stored root when the root is held. append and adopt push roots, and the roots are synced before each snapshot.
  - modified: `crates/lys-identity/src/log.rs` — Adds EventLog::open_beside.
  - modified: `crates/lys-identity/src/directory.rs` — Adds open_beside, folded() and is_settled().
  - modified: `crates/lys-identity/src/lib.rs` — Adds mod roots.
  - created: `crates/lys-identity/tests/receipt_roots.rs` — Leaf 4 000 of 5 000 reads 1 leaf with roots and 929 without, with receipts equal to those the appends returned. Removed, tampered and short roots files are rebuilt, and the leaves read are counted.
- Checklist delivery:
  - [x] C339 — A receipt is rebuilt from the stored coordinate, not by re-reading leaves (DIRECTORY-043 R6), proved by a counting test that fails at the base. — Both receipt_roots tests pass: read counts of 1 and 929, and rebuild counts of 4096+904 and 3072+904.
- Story delivery:
  - [x] S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows. — Receipts are rebuilt without re-reading the log, byte for byte equal to the base's.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test with a 5 000-leaf ledger shows entry() for leaf 4 000 reads 1 leaf file, where the base reads up to 1 024. — crates/lys-identity/tests/receipt_roots.rs a_receipt_reads_one_leaf_and_a_removed_roots_file_is_rebuilt asserts 1 leaf read with roots, and 929 on the base checkpoint path opened without roots. Ledger::entry (restart.rs) goes to entry_at when Beside::root(index) is held. It passed in the workspace run.
  - [x] The receipts equal the base's byte for byte, and a coordinate store removed from disk is rebuilt from the log on open and gives the same receipts. — Each receipt is compared == with the Receipt the append returned, and after remove_file the roots file is rebuilt to 5000*32 bytes. a_refused_or_short_roots_file_is_rebuilt_from_the_log counts the rebuild reads for a tampered checkpoint record, a tampered snapshot record and a short file, and checks the receipts equal after each. roots.rs logs RootsRefused/RootsRebuilt/RootsNotKept by name. Both tests passed.
- Checklist verified: C339
- Stories verified: S145

## Boundaries

- SHALL NOT change any answer, refusal, record, wire format, stored byte or audit line: every existing test stays as it is and green; a changed expected value is a defect, not an update.
- SHALL NOT add any timeout, deadline, sleep, poll interval or bound in seconds, and SHALL remove any this brief names (CLAUDE.md, No time limits).
- SHALL NOT add #[allow], #[ignore], a _-prefixed binding or any other bypass; clippy with -D warnings, ast-grep scan and the file-length leg stay green, and no file passes 500 lines of code.
- SHALL NOT add a dependency that is not already in Cargo.lock unless the dev record names it and why nothing in the lock serves.
- SHALL NOT measure speed with a clock in any test: each requirement is proved by counting the work done (reads, parses, scans, syscalls through a counting double, lock acquisitions), never by elapsed time.

## Verification

- cargo fmt --all --check, both clippy legs with -D warnings, cargo test --workspace --all-features --no-fail-fast, both cargo doc legs, ast-grep scan --config sgconfig.yml, sh scripts/design/gate.sh and (once LYSGATE-002 has landed) sh scripts/file-length.sh all exit 0 at the card's head, measured by the card round, never by the builder's own run.
- Every counting test named in an acceptance line fails at the card's base commit and passes at its head: the dev record names each test and quotes its failing assertion at the base.
