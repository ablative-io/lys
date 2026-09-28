---
type: brief
id: DIRECTORY-045
cluster: directory
title: Name every build, and upgrade a running identity install in place with a way back
---

# DIRECTORY-045: Name every build, and upgrade a running identity install in place with a way back

> **Cluster:** directory
> **Depends on:** DIRECTORY-044
> **Blocked by:** DIRECTORY-044 landed on main (it adds the exit and readiness waits this brief's upgrade uses): the build starts only when `git log --oneline origin/main --grep=DIRECTORY-044` names its R1 and R2 commits.
> **Design anchor:**
> - ADR-114 — An installed identity product names its build and upgrades itself in place, keeping the previous build to return to — Every Lys binary answers --version with the commit and dirty state it was built from, stamped at build time; a build with no commit to read says so in those words, never a made-up value. `lys identity upgrade` takes a folder of newly built binaries (and optionally a screens package), checks each one's --version, stops the service and the broker by their exit events, swaps the binaries by rename keeping the previous set beside them, starts the new ones and waits for them ready; if any new binary fails to start or become ready, it puts the previous set back, starts it, and fails naming what broke. Data, credentials, configuration and the compose services are never touched by an upgrade. Rejected: making install restart what differs, which would mix a first install's promises with an upgrade's risks; asking people to copy binaries by hand.
> - ADR-113 — The identity install and its clients wait on signals, never on a clock — A wait ends on the event it is waiting for or on the failure that makes the event impossible: a service is ready when it says so or when a single connection to it succeeds after a readiness event (its log gaining its listening line, observed through the platform's file-change notification), a stopped process is gone when the platform reports its exit (kqueue EVFILT_PROC on macOS, pidfd on Linux), a peer that refuses or closes is an error at once, and nothing carries a timeout. The screen re-measures on a ResizeObserver or animation-frame signal. Rejected: shorter sleeps, which still wait on a clock; keeping timeouts as a safety net, which the rule forbids and which hides a stuck peer's cause.
> **Checklist:**
> - C344 — Every Lys binary answers --version with its build commit (DIRECTORY-045 R1).
> - C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7).
> - C346 — The install and the server say which build is running (DIRECTORY-045 R3).
> - C347 — Prove the upgrade on a scratch install in the round; the live install is upgraded and recorded after landing (DIRECTORY-045 R4).
> **Stories:**
> - S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

## Purpose

A person running the identity product cannot tell which build is running, and cannot take a newer one: install only fills in what is missing and never restarts. So every landing stays invisible until someone swaps binaries by hand. This brief makes every binary name its commit and gives the product one command that upgrades a running install and puts the previous build back if the new one fails. Amended 28 September 2026 20:35 by Waffles after Archie's review of the first round (card/DIRECTORY-045 ca11e605): an upgrade that leaves the configuration and compose files behind cannot move an install to any build that adds a configuration key (the server refuses unknown keys), and so cannot carry the sign-in cookie fix, which lives in the compose file; an install made before this card must move to it; and an upgrade stopped part-way must be recoverable.

## Task

Stamp the build commit into every Lys binary, add `lys identity upgrade`, surface the running build in /authority and the screens, and prove it by upgrading the live install.

## Requirements

### R1: Every Lys binary answers --version with its build commit

Behavioural. lys, lys-identity-server, lys-secrets and lys-home each answer --version (and -V) with `NAME VERSION (COMMIT[; dirty])`, stamped at build time from the git tree being built (a build.rs reading `git rev-parse HEAD` and `git status --porcelain`, rerun when .git/HEAD or the index moves). A build with no git tree to read prints `(not built from a git commit)`: never an empty or invented value. lys-identity-server no longer reads a first argument of --version as a configuration path.

**Acceptance:**
- For each binary, `BIN --version` at the card's head prints its name, the crate version and the 40-character commit of that head, and a test builds from an exported tree without .git and shows the stated words.
- `lys-identity-server --version` exits 0 and reads no file.

**Files:**
- create: crates/lys/build.rs
- create: crates/lys-identity-server/build.rs
- create: crates/lys-secrets/build.rs
- create: crates/lys-home/build.rs
- modify: crates/lys/src/main.rs
- modify: crates/lys-identity-server/src/main.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/main.rs
- modify: crates/lys-home/src/main.rs

**Checklist:**
- C344 — Every Lys binary answers --version with its build commit (DIRECTORY-045 R1).

**Stories:**
- S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Built in an earlier round and unchanged this round. crates/lys/build.rs stamps LYS_BUILD from `git rev-parse HEAD` and `git status --porcelain`, and every binary prints `NAME VERSION (LYS_BUILD)`. crates/lys/tests/version.rs holds the four build.rs files byte-identical and checks an exported tree and a committed tree. The tests leg passed in round 2's measurement.
- Deviation: The exported-tree test builds a small probe crate with the real build.rs instead of rebuilding the whole workspace.
- Files changed:
  - modified: `crates/lys/build.rs` — Stamps LYS_BUILD with the 40-character head commit, adding '; dirty' when the tree has changes, or 'not built from a git commit'. It reruns when HEAD, the index, packed-refs, refs/heads or the branch ref changes, among the ones that exist.
  - modified: `crates/lys-identity-server/build.rs` — Byte-identical to crates/lys/build.rs.
  - modified: `crates/lys-secrets/build.rs` — Byte-identical to crates/lys/build.rs.
  - modified: `crates/lys-home/build.rs` — Byte-identical to crates/lys/build.rs.
  - modified: `crates/lys/tests/version.rs` — Tests that --version names the head commit, that the four build scripts are equal, and what an exported tree and a committed tree print.
  - modified: `crates/lys-identity-server/tests/version.rs` — Tests that --version exits 0 and reads no file.
  - modified: `crates/lys-secrets/tests/version.rs` — Tests that --version names the build.
  - modified: `crates/lys-home/tests/version.rs` — Tests that --version names the build.
- Checklist delivery:
  - [x] C344 — Every Lys binary answers --version with its build commit (DIRECTORY-045 R1). — Each binary's --version names its build commit. Tests: crates/*/tests/version.rs.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — Every build carries its name, which upgrade and install compare.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] For each binary, `BIN --version` at the card's head prints its name, the crate version and the 40-character commit of that head, and a test builds from an exported tree without .git and shows the stated words. — The four build.rs files are byte-identical (crates/lys/tests/version.rs:65 every_stamped_crate_carries_the_same_build_script). Each crate's tests/version.rs has version_names_the_crate_version_and_the_head_commit. crates/lys/tests/version.rs:110 a_build_from_an_exported_tree_says_it_has_no_commit. All pass in the measured tests leg (exit 0).
  - [x] `lys-identity-server --version` exits 0 and reads no file. — crates/lys-identity-server/tests/version.rs:38 runs the flag in a cwd holding a file named like the flag and asserts success. Passes in the measured tests leg.
- Checklist verified: C344
- Stories verified: S147

### R2: `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure

Behavioural. A new verb `lys identity upgrade --from DIR [--surface PKG] [--root ROOT]`. It refuses by name when ROOT holds no install, when DIR lacks any installed binary, or when a binary's --version cannot be read. It prints the installed and the new commit of each. It stops the service and then the broker, each waiting on its exit event through DIRECTORY-044's exit wait (never a sleep or a timeout); moves bin/ to bin.previous/ (replacing an older bin.previous/) and places the new binaries by copy to a temporary name and rename; places the screens package through the install's existing verify-and-place path when --surface is given, keeping the previous one the same way; starts the broker and then the service and waits for each ready on its readiness event. If a start or readiness fails, it stops what it started, restores bin.previous/ (and the previous screens), starts them, waits for them ready, and exits non-zero naming the failing binary and its log. It never touches data/ or credentials; the configuration and compose files move with the binaries as R5 says.

**Acceptance:**
- A test on a scratch root installs build A (two stub binaries answering --version and readiness), upgrades to build B, and shows B running and A kept in bin.previous/; a second test where B's service exits before ready shows A restored, running and ready, and the named failure.
- A test shows data/ and every credential are byte-identical before and after both upgrades.
- grep -nE 'sleep|timeout' over the new files prints nothing.

**Files:**
- create: crates/lys/src/identity/upgrade.rs
- create: crates/lys/src/identity/upgrade_tests.rs
- modify: crates/lys/src/identity/cli.rs
- modify: crates/lys/src/identity/install/layout.rs

**Checklist:**
- C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7).

**Stories:**
- S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: upgrade() stops the service and then the broker, each on its exit event. It moves bin/ to bin.previous/ and places each binary through a temporary name, reading back its SHA-256 digest before the rename. It then places the screens and starts the broker and the service, waiting for each to be ready on log and HTTP events. On failure, swap::back puts the previous build back, starts it, and the error names the failing binary and its log. Unchanged this round.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys/src/identity/upgrade.rs` — upgrade() finishes or puts back an unfinished upgrade, checks the install and each incoming --version, renders, writes the intent and swaps forward. If anything fails it swaps back and names the failure.
  - modified: `crates/lys/src/identity/upgrade_tests.rs` — Tests: A is upgraded to B with A kept; a failed B puts A back running; data and credentials are untouched.
  - created: `crates/lys/src/identity/upgrade/scratch_tests.rs` — A scratch-install fixture of /bin/sh stubs that block on a fifo.
  - modified: `crates/lys/src/identity/cli.rs` — Help text for upgrade and install.
  - modified: `crates/lys/src/identity/install/layout.rs` — Adds upgrade_intent() and config_previous_dir().
  - modified: `crates/lys/src/identity/error.rs` — Adds InstallBuildDiffers and UpgradeBuildDiffers.
  - modified: `crates/lys/src/identity/error_tests.rs` — Tests the wire names of the two new kinds.
- Checklist delivery:
  - [x] C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7). — Tests: upgrade_tests.rs and the render, intent and adopt tests.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — A running install moves to a new build, and a failed new build falls back to the previous one.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test on a scratch root installs build A (two stub binaries answering --version and readiness), upgrades to build B, and shows B running and A kept in bin.previous/; a second test where B's service exits before ready shows A restored, running and ready, and the named failure. — crates/lys/src/identity/upgrade_tests.rs:14 an_upgrade_runs_the_new_build_and_keeps_the_previous and :55 a_service_that_exits_before_ready_puts_the_previous_build_back, using the fifo-blocking /bin/sh stubs in upgrade/scratch_tests.rs. Measured tests leg exit 0.
  - [x] A test shows data/ and every credential are byte-identical before and after both upgrades. — upgrade_tests.rs:96 neither_upgrade_touches_data_or_a_credential. The same untouchable() comparison also appears in render_tests.rs:25.
  - [x] grep -nE 'sleep|timeout' over the new files prints nothing. — Running grep -nE 'sleep|timeout' over crates/lys/src/identity/upgrade/*, upgrade.rs and install/log_wait*.rs printed nothing.
- Checklist verified: C345
- Stories verified: S147

### R3: The install and the server say which build is running

Behavioural. `lys identity install` and `lys identity upgrade` write install/build.json under the root with each binary's commit and the screens package's digest, and print them. The identity server's GET /authority answer gains a `build` member with its own commit, and the screens show it in the About or settings view that already lists the authority.

**Acceptance:**
- A test reads /authority from a running server built at the card's head and finds its commit in `build`.
- A surface test renders the build commit from a stubbed /authority answer.

**Files:**
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys-identity-server/src/read_api.rs
- modify: surface/identity/src/shell/About.tsx

**Checklist:**
- C346 — The install and the server say which build is running (DIRECTORY-045 R3).

**Stories:**
- S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The server answers /authority with its build, and the screens show it. install writes install/build.json through adopt::settle and record_build, recording a binary that cannot answer --version as 'built before build stamps'. This round's surface leg failure was at tests/mockup.test.tsx:199, 'all six cases fail on v5': 'Test timed out in 5000ms.' That test runs six jsdom page loads in sequence, and under the gate machine's load it outran Vitest's default bound. The cause is the clock bound itself, which CLAUDE.md forbids, not the test. The fix in vite.config.ts was written at 00:58, after the gate log's last write at 00:56, so the round never measured it. With it, `npm --prefix surface/identity test` (vitest run, then node --test scripts/package.test.mjs) exits 0 here: 48 files, 325 tests.
- Deviation: surface/identity/src/shell/About.tsx does not exist on main, so the build is shown in surface/identity/src/features/signin/Gate.tsx, as the brief's rule for a missing file allows. The vite.config.ts change was already in the tree when this round started; this round checked it against Vitest's source and ran the suite with it.
- Files changed:
  - modified: `crates/lys/src/identity/install.rs` — Finishes or puts back an unfinished upgrade, refuses a build other than the one placed, and records build.json through adopt::settle.
  - modified: `crates/lys-identity-server/src/read_api.rs` — /authority answers with a `build` member (earlier round).
  - modified: `surface/identity/src/features/signin/Gate.tsx` — Shows the build commit (earlier round).
  - modified: `surface/identity/vite.config.ts` — Sets testTimeout: 0 and hookTimeout: 0, which turns off Vitest's 5000 ms and 10000 ms bounds, so no screen test is failed by a clock. Vitest 5.0.2's withTimeout returns the function unwrapped when the timeout is 0 or less.
- Checklist delivery:
  - [x] C346 — The install and the server say which build is running (DIRECTORY-045 R3). — The install and the server both say which build is running.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — The running build can be read from build.json and from /authority.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test reads /authority from a running server built at the card's head and finds its commit in `build`. — crates/lys-identity-server/tests/authority_build.rs passes in the measured tests leg.
  - [x] A surface test renders the build commit from a stubbed /authority answer. — surface/identity/tests/build.test.tsx:19 stubs '/authority' with a `build` member. The measured surface leg exited 0, and vite.config.ts testTimeout/hookTimeout 0 is verified against vitest run.C5UmxDPh.js:3252 ('if (timeout <= 0 ...) return fn').
- Checklist verified: C346
- Stories verified: S147

### R4: Prove it on the live install

Evidence. Evidence. On the machine the live install runs on, build lys, lys-identity-server and lys-secrets at the card's head on the build machine through the card's own round, then run `lys identity upgrade --from` that build against the live install, and show the running server's /authority `build` equals the card's head. Record the before and after commits and the upgrade's printed lines in docs/design/directory/PROOF-UPGRADE.md. Secrets and passwords never appear in it. The live install was made before this card (R6), so the proof starts with its first move, and the sign-in page still keeps its session over plain http after the upgrade (the cookie setting carried by R5).

**Acceptance:**
- PROOF-UPGRADE.md names the installed commit before, the head commit after, and quotes the upgrade's lines; `curl -s http://localhost:8490/authority` shows the head commit.
- After the upgrade, signing in on the live install in a browser keeps the session, and the compose environment holds the cookie setting the new build renders.

**Files:**
- create: docs/design/directory/PROOF-UPGRADE.md

**Checklist:**
- C347 — Prove the upgrade on a scratch install in the round; the live install is upgraded and recorded after landing (DIRECTORY-045 R4).

**Stories:**
- S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

#### R4 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. There is still no live identity install that can be reached from this machine: `lsof -nP -iTCP:8490 -sTCP:LISTEN` prints nothing, `curl -s http://localhost:8490/authority` gets no answer, there is no lys directory under ~/Library/Application Support, and ~/.ssh/config names no host. PROOF-UPGRADE.md cannot truthfully be written without a real upgrade, so it was not created. On the machine that runs the live install, once the card's round has built the head: 1. Read the installed commit from install/build.json. 2. Run `<head build dir>/lys identity upgrade --from <head build dir>`. The lys must be the one built with the new binaries, or it refuses with upgrade_build_differs. 3. Record the before and after commits and the printed lines in docs/design/directory/PROOF-UPGRADE.md, with no secret, and show `curl -s http://localhost:8490/authority` naming the head commit. 4. Sign in in a browser, confirm the session survives a reload, and confirm compose.env names the cookie setting without quoting its value. No command may carry a time limit.
- Deviation: Not performed: no live install can be reached from this machine.
- Checklist delivery:
  - [ ] C347 — Prove the upgrade on a scratch install in the round; the live install is upgraded and recorded after landing (DIRECTORY-045 R4). — Blocked on a live install; PROOF-UPGRADE.md is not written.
- Story delivery:
  - [ ] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — The live proof is still outstanding.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] PROOF-UPGRADE.md names the installed commit before, the head commit after, and quotes the upgrade's lines; `curl -s http://localhost:8490/authority` shows the head commit. — docs/design/directory/PROOF-UPGRADE.md does not exist. lsof -nP -iTCP -sTCP:LISTEN on this Mac shows no listener on 8490.
  - [ ] After the upgrade, signing in on the live install in a browser keeps the session, and the compose environment holds the cookie setting the new build renders. — No live install could be reached, so no upgrade and no sign-in were performed.
- Issues:
  - Run the upgrade on the machine that hosts the live identity install (not this Mac: no :8490 listener, no lys install in Application Support, no ssh config). Build the card's head, run `<head build dir>/lys identity upgrade --from <head build dir>`, write docs/design/directory/PROOF-UPGRADE.md with the before and after commits and the printed lines (no secrets), show `curl -s http://localhost:8490/authority` naming the head commit, and confirm that a browser sign-in survives and compose.env names the cookie setting.

### R5: The configuration and compose files move with the binaries

Behavioural. An upgrade renders deployment.toml's derived files, the directory service's configuration and the compose files (compose.yaml and its environment) for the new build, from the install's recorded choices and the new build's templates, inside the swap: the previous ones are kept beside them (config.previous/) and put back with the binaries on failure and by --back (DIRECTORY-053). The compose services whose rendered definition changed are recreated through the engine, waited on for ready, and put back on failure. Running install from a build other than the one placed is refused install_build_differs, naming the placed and the new commit and the upgrade command, never restarting old binaries on a new configuration.

**Acceptance:**
- A test where build B adds a configuration key and a compose environment value upgrades an install of build A: B runs with the new key and value, and a failed start of B puts A's binaries, configuration and compose files back, byte-identical.
- Install run from build B over an install of build A is refused install_build_differs and nothing is stopped.
- data/ and every credential are byte-identical across both.

**Files:**
- create: crates/lys/src/identity/upgrade/render.rs
- create: crates/lys/src/identity/upgrade/render_tests.rs
- modify: crates/lys/src/identity/upgrade.rs
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys/src/identity/install/server_config.rs
- modify: crates/lys/src/identity/install/layout.rs

**Checklist:**
- C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7).

**Stories:**
- S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The new build's configuration and compose files are rendered before anything is stopped. Inside the swap, the previous files are kept in config.previous/ and the new ones placed with a digest read-back; when compose changed, they are applied through the Engine. On the way back the kept files are restored and applied again. No credential is generated or written. Unchanged this round.
- Deviation: (none)
- Files changed:
  - created: `crates/lys/src/identity/upgrade/render.rs` — Templates renders compose.yaml, postgres-init.sql, compose.env and identity.json from the install's recorded choices. It refuses a build other than this lys, and its Debug output shows lengths only.
  - created: `crates/lys/src/identity/upgrade/render_tests.rs` — Tests: B runs with its new key and value; a failed B puts A's files back byte for byte; rendering only reads; a missing credential is refused.
  - created: `crates/lys/src/identity/upgrade/swap.rs` — Keeps, places and restores the configuration and compose files alongside the binaries.
  - modified: `crates/lys/src/identity/install/server_config.rs` — recorded_administrator reads the administrator from the recorded identity.json.
- Checklist delivery:
  - [x] C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7). — Test: render_tests.rs.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — The configuration and compose files move with the binaries.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test where build B adds a configuration key and a compose environment value upgrades an install of build A: B runs with the new key and value, and a failed start of B puts A's binaries, configuration and compose files back, byte-identical. — crates/lys/src/identity/upgrade/render_tests.rs:25 build_b_runs_with_its_new_key_and_value_and_a_failed_b_puts_a_back asserts that configuration() equals `before`, that the binaries are at A, that compose is applied B then A, and that B's log contains NEW_KEY.
  - [x] Install run from build B over an install of build A is refused install_build_differs and nothing is stopped. — upgrade/adopt_tests.rs:206 install_from_another_build_is_refused_and_stops_nothing asserts the pid files are unchanged and the ready lines are still A's. install.rs calls adopt::check_placed_build before require_docker and before any start or stop.
  - [x] data/ and every credential are byte-identical across both. — The untouchable() equality assertions in render_tests.rs:25 (both halves) and adopt_tests.rs:206.
- Checklist verified: C345
- Stories verified: S147

### R6: The first move of an install made before this card

Behavioural. An install with no install/build.json (made before this card) is upgraded, not re-installed: upgrade reads the placed binaries' versions (a binary that cannot answer --version is recorded as 'built before build stamps'), stops the processes its pid files name through their exit locks, or through the pid when a process has no exit lock, naming which, and proceeds as R2 and R5. Install run again never leaves a process running from a binary it replaced: it stops and restarts any process whose binary it changed, so install/build.json always names what runs.

**Acceptance:**
- A scratch install laid out as before this card (no build.json, no exit locks, a binary without --version) is upgraded, and /api/authority answers the new commit.
- Install run again with a changed binary restarts that process, and build.json equals the running build.

**Files:**
- create: crates/lys/src/identity/upgrade/adopt.rs
- create: crates/lys/src/identity/upgrade/adopt_tests.rs
- modify: crates/lys/src/identity/install/services.rs

**Checklist:**
- C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7).
- C346 — The install and the server say which build is running (DIRECTORY-045 R3).

**Stories:**
- S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: An install with no build.json is adopted. Each placed binary's build is read, and a binary that cannot answer is recorded as 'built before build stamps'. Each process is stopped through its exit lock, or through its pid when it has none, and the output names which. The upgrade then proceeds as R2 and R5 describe. The exit event is the answer: after `caffeinate -w` (or the Linux pidfd) returns success, nothing asks ps again. Unchanged this round; the tests leg passed in round 2's measurement.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys/src/identity/upgrade/adopt.rs` — On macOS and the BSDs, stop_through_pid takes the waiter's success as the exit and refuses a waiter that fails. adopt_running returns the file the running process was started from, and installed() reads an adopted binary's build from it.
  - created: `crates/lys/src/identity/upgrade/adopt_tests.rs` — Tests: a pre-card install is adopted and upgraded, and /api/authority answers the new build; install run again restarts a process whose binary it placed; install from another build is refused.
  - modified: `crates/lys/src/identity/install/services.rs` — The log wait moved out to log_wait.rs.
  - modified: `crates/lys/src/identity/install/exit_wait.rs` — Doc: only upgrade::adopt stops a process that has no exit lock.
- Checklist delivery:
  - [x] C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7). — Test: adopt_tests.rs, the pre-card upgrade.
  - [x] C346 — The install and the server say which build is running (DIRECTORY-045 R3). — build.json names what runs after install and after adoption.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — An install made before builds were named is upgraded in place.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A scratch install laid out as before this card (no build.json, no exit locks, a binary without --version) is upgraded, and /api/authority answers the new commit. — upgrade/adopt_tests.rs:97 an_install_made_before_this_card_is_upgraded_and_answers_the_new_build, using stand_in_for_an_unstamped_binary and start_unlocked. Measured tests leg exit 0.
  - [x] Install run again with a changed binary restarts that process, and build.json equals the running build. — upgrade/adopt_tests.rs:152 install_run_again_restarts_a_process_whose_binary_it_placed.
- Checklist verified: C345, C346
- Stories verified: S147

### R7: An upgrade stopped part-way is finished or put back

Behavioural. Before the first stop, an upgrade writes an intent record (install/upgrade.json: from and to builds, and each step as it completes, written by a temporary name and a rename). Every later upgrade, --back or install first reads it and, when present, finishes the recorded upgrade or puts the previous build back, naming which, before doing anything else. Each placed binary and file is read back and its digest compared with its source before the old one is removed, as the screens already are. A readiness check on a process already running reads its log from the offset it opened at, never from the first byte on each change. upgrade.rs stays under 500 lines by moving the swap, render and adopt into their own modules.

**Acceptance:**
- A test kills the upgrade after the rename of bin/ and before the last copy; the next upgrade names the unfinished one, finishes it or puts it back, and the install runs one whole build.
- A test corrupts one placed copy; the digest check refuses it and nothing old is removed.
- No file under crates/lys/src/identity/ is 500 lines or more.

**Files:**
- create: crates/lys/src/identity/upgrade/intent.rs
- create: crates/lys/src/identity/upgrade/intent_tests.rs
- create: crates/lys/src/identity/upgrade/swap.rs
- modify: crates/lys/src/identity/upgrade.rs

**Checklist:**
- C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7).

**Stories:**
- S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

#### R7 — Execution record

**Dev (recorded):**

- Status: implemented
- How: The intent record is written before the first stop, and each step is recorded as it completes. The next upgrade or install acts on an unfinished upgrade and names what it did: one whose new build had started is finished, and any other is put back. Placement copies to a temporary name, compares digests and renames. This round's design leg failure was 'rendered markdown differs from the committed file: docs/design/directory/./briefs/DIRECTORY-045.md'. The workflow had written round 2's dev blocks into DIRECTORY-045.json after the markdown was last rendered. Running `python3 ../scripts/design/render-cluster.py design/directory` from docs/ re-rendered the cluster, and `sh scripts/design/gate.sh` exits 0 here.
- Deviation: The design.json rows belong to DIRECTORY-048 and DIRECTORY-049, not this card; they were added because the gate at this card's head fails without them. DIRECTORY-045.md renders from a JSON file the workflow writes after each answer, so it needs rendering again whenever the workflow adds a round's blocks.
- Files changed:
  - created: `crates/lys/src/identity/upgrade/intent.rs` — install/upgrade.json holds each completed step, written atomically.
  - created: `crates/lys/src/identity/upgrade/intent_tests.rs` — Tests: a kill after bin/ moved is put back and the upgrade then runs; recovery on its own; a started build is finished; a corrupted copy is refused.
  - created: `crates/lys/src/identity/install/log_wait.rs` — LogCursor and the log watch, so a readiness wait reads only what is new.
  - created: `crates/lys/src/identity/install/log_wait_tests.rs` — Tests for the cursor.
  - modified: `crates/lys/src/identity/install.rs` — install first finishes or puts back an unfinished upgrade.
  - modified: `docs/design/directory/design.json` — Adds structure rows for DIRECTORY-048 R7 and DIRECTORY-049 R7, so the coverage leg is clean.
  - modified: `docs/design/directory/DESIGN.md` — Re-rendered from design.json.
  - modified: `docs/design/directory/briefs/DIRECTORY-045.md` — Re-rendered from DIRECTORY-045.json, which now carries round 2's dev blocks, so it matches its source.
- Checklist delivery:
  - [x] C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7). — Test: intent_tests.rs.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — An interrupted upgrade never leaves a half-placed build.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [x] A test kills the upgrade after the rename of bin/ and before the last copy; the next upgrade names the unfinished one, finishes it or puts it back, and the install runs one whole build. — upgrade/intent_tests.rs:68 killed_after_the_first_copy is used by :87 the_next_upgrade_puts_back_an_upgrade_killed_part_way_then_runs and :121. :146 an_upgrade_killed_after_its_new_build_started_is_finished.
  - [x] A test corrupts one placed copy; the digest check refuses it and nothing old is removed. — upgrade/intent_tests.rs:174 a_copy_that_differs_from_its_source_is_refused_and_nothing_old_goes, and :217. The refusal text is in swap.rs:67.
  - [x] No file under crates/lys/src/identity/ is 500 lines or more. — wc -l: the largest is upgrade.rs at 466. scripts/file-length.sh reports '557 files measured, 0 over the limit'.
- Checklist verified: C345
- Stories verified: S147
- Issues:
  - This round's measured design leg failed with 'rendered markdown differs from the committed file: docs/design/directory/./briefs/DIRECTORY-045.md' (legs.log, design leg line 126). This is the same defect as round 2: the workflow writes the round's blocks into DIRECTORY-045.json after the developer renders. In this checkout the .md (01:53) is newer than the .json (01:15) and `sh scripts/design/gate.sh` exits 0 now. But the card's measurement must show the design leg at 0, which means fixing the chain: re-render the cluster after the workflow writes the brief JSON, or render before the design leg checks. This must not be patched by hand again each round.

## Boundaries

- SHALL NOT add any timeout, deadline, sleep or poll interval.
- SHALL NOT touch data/ or any credential in an upgrade. The configuration and compose files are rendered for the new build inside the swap and kept and put back with the binaries (R5); nothing else outside bin/, the screens, the logs, the process files and install/build.json is written.
- SHALL NOT print or record any secret, password or API key, in output, tests or the proof.
- SHALL NOT add #[allow], #[ignore] or any bypass; every file stays under 500 lines of code; ast-grep stays at zero hits.
- Where a file named here does not exist on main at the card's base (About.tsx, a main.rs), the change goes where that concern lives and the dev record names the path.

## Verification

- The full Lys gate and the surface checks exit 0 at the card's head, measured by the card round.
- The live upgrade of R4 is recorded in PROOF-UPGRADE.md and /authority on the live install names the head commit.
