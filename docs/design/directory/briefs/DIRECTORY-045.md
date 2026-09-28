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
> - C347 — Prove it on the live install (DIRECTORY-045 R4).
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
- How: Built in an earlier round; unchanged this round. crates/lys/build.rs stamps LYS_BUILD from `git rev-parse HEAD` and `git status --porcelain`, and every binary prints `NAME VERSION (LYS_BUILD)`. crates/lys/tests/version.rs holds the four build.rs files byte-identical and probes an exported tree and a committed tree. The gate's one failure of a_build_from_a_commit_names_it_and_its_dirty_state (line 149: after the first commit the probe still said 'not built from a git commit') did not reproduce here: 4 of 4 passed and the reviewer's run passed. I did not find its cause, so build.rs is unchanged rather than changed on a guess.
- Deviation: The exported-tree test builds a probe crate that uses the real build.rs, not a full workspace rebuild (as in round 1). The gate's single version-test failure is reported, not fixed: the cause was not found.
- Files changed:
  - modified: `crates/lys/build.rs` — Stamps LYS_BUILD with the 40-character head commit, adding '; dirty' when the tree has changes, or 'not built from a git commit'. Watches HEAD, the index, packed-refs, refs/heads and the branch ref.
  - modified: `crates/lys-identity-server/build.rs` — Byte-identical to crates/lys/build.rs.
  - modified: `crates/lys-secrets/build.rs` — Byte-identical to crates/lys/build.rs.
  - modified: `crates/lys-home/build.rs` — Byte-identical to crates/lys/build.rs.
  - modified: `crates/lys/tests/version.rs` — --version names the head commit; the four build scripts are equal; an exported tree and a committed tree are probed.
  - modified: `crates/lys-identity-server/tests/version.rs` — --version exits 0 and reads no file.
  - modified: `crates/lys-secrets/tests/version.rs` — --version names the build.
  - modified: `crates/lys-home/tests/version.rs` — --version names the build.
- Checklist delivery:
  - [x] C344 — Every Lys binary answers --version with its build commit (DIRECTORY-045 R1). — Each binary's --version names its build commit. Tests: crates/*/tests/version.rs.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — Every build is named, which upgrade and install compare.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] For each binary, `BIN --version` at the card's head prints its name, the crate version and the 40-character commit of that head, and a test builds from an exported tree without .git and shows the stated words. — crates/lys/build.rs:72-99 stamps LYS_BUILD from `git rev-parse HEAD` (40-hex filter) plus '; dirty', or 'not built from a git commit'. The four build.rs files are byte-identical (shasum 689f4206…), and every_stamped_crate_carries_the_same_build_script holds them equal. Tests: version_names_the_crate_version_and_the_head_commit in each crate's tests/version.rs, and a_build_from_an_exported_tree_says_it_has_no_commit (crates/lys/tests/version.rs:110). The gate's tests leg exited 0.
  - [x] `lys-identity-server --version` exits 0 and reads no file. — crates/lys-identity-server/tests/version.rs runs --version and -V in a directory holding a file named like the flag, and asserts success. The gate's tests leg exited 0.
- Checklist verified: C344
- Stories verified: S147
- Issues:
  - The cause of a_build_from_a_commit_names_it_and_its_dirty_state failing once in round 1 (line 149) is still unknown. It passed in this round's gate. If it recurs, find the cause in build.rs's rerun-if-changed set.

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
- How: upgrade() stops the service and the broker, moves bin/ to bin.previous/, and places each binary through a temporary name with a SHA-256 read-back. It then places the screens and starts the broker and the service, each waited on for ready through log and HTTP events. On failure, swap::back puts the previous build back and starts it, and the error says so. Unchanged this round apart from the adoption stop fix under R6, which this path uses for processes that have no exit lock.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys/src/identity/upgrade.rs` — upgrade(): recover, check the install, check each incoming --version, render, write the intent, swap forward, and swap back with the failure named when anything fails.
  - modified: `crates/lys/src/identity/upgrade_tests.rs` — Tests: A upgraded to B with A kept; a failed B puts A back running; data and credentials untouched.
  - created: `crates/lys/src/identity/upgrade/scratch_tests.rs` — Scratch-install fixture of /bin/sh stubs blocked on a fifo.
  - modified: `crates/lys/src/identity/cli.rs` — Upgrade and install help text.
  - modified: `crates/lys/src/identity/install/layout.rs` — upgrade_intent() and config_previous_dir().
  - modified: `crates/lys/src/identity/error.rs` — InstallBuildDiffers and UpgradeBuildDiffers.
  - modified: `crates/lys/src/identity/error_tests.rs` — Wire names of the two new kinds.
- Checklist delivery:
  - [x] C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7). — Tests: upgrade_tests.rs and the render, intent and adopt tests. The upgrade module's 19 tests pass here.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — A running install moves to a new build, and a failed new build comes back to the previous one.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test on a scratch root installs build A (two stub binaries answering --version and readiness), upgrades to build B, and shows B running and A kept in bin.previous/; a second test where B's service exits before ready shows A restored, running and ready, and the named failure. — upgrade_tests.rs:14 an_upgrade_runs_the_new_build_and_keeps_the_previous and :55 a_service_that_exits_before_ready_puts_the_previous_build_back. swap.rs:309-360 and upgrade.rs:391-417 name the failure as UpgradeFailed. Local identity run: 83 passed, 0 failed.
  - [x] A test shows data/ and every credential are byte-identical before and after both upgrades. — upgrade_tests.rs:96 neither_upgrade_touches_data_or_a_credential. render_tests.rs:25 also compares untouchable() across both upgrades.
  - [x] grep -nE 'sleep|timeout' over the new files prints nothing. — Ran `grep -nE 'sleep|timeout'` over upgrade.rs, upgrade/*, install/log_wait*.rs, */build.rs and */tests/version.rs: no output, exit 1.
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
- How: The server answers /authority with its build, and the surface renders it (both from an earlier round). install writes install/build.json through adopt::settle and record_build. A binary that cannot answer --version is recorded as 'built before build stamps', so build.json always names what runs. This round's gate clippy failure at install.rs:127 is fixed in the tree: `emitter.note(line);`. fmt --check and clippy on lys with --all-targets --all-features are clean here.
- Deviation: surface/identity/src/shell/About.tsx does not exist on main, so the build is rendered in surface/identity/src/features/signin/Gate.tsx, as the brief's rule for a missing file allows.
- Files changed:
  - modified: `crates/lys/src/identity/install.rs` — Recovers an unfinished upgrade, refuses a build other than the one placed, and records build.json through adopt::settle. Line 127's statement ends in a semicolon (clippy semicolon_if_nothing_returned).
  - modified: `crates/lys-identity-server/src/read_api.rs` — /authority answers `build` (earlier round).
  - modified: `surface/identity/src/features/signin/Gate.tsx` — Renders the build commit (earlier round).
- Checklist delivery:
  - [x] C346 — The install and the server say which build is running (DIRECTORY-045 R3). — The install and the server say which build runs.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — The running build can be read from build.json and from /authority.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test reads /authority from a running server built at the card's head and finds its commit in `build`. — crates/lys-identity-server/tests/authority_build.rs, with read_api.rs answering `build`. The gate's tests leg exited 0.
  - [x] A surface test renders the build commit from a stubbed /authority answer. — surface/identity/tests/build.test.tsx (2 tests), rendering in features/signin/Gate.tsx. `npm --prefix surface/identity install-ci-test` here: 325/325 passed, exit=0.
- Checklist verified: C346
- Stories verified: S147
- Issues:
  - The gate's surface leg failed because Vitest's default 5000 ms testTimeout killed tests/mockup.test.tsx under load. That is a clock bound, which the No time limits rule forbids.
- Fixes:
  - surface/identity/vite.config.ts: set testTimeout: 0 and hookTimeout: 0, which turns off Vitest's clock bounds. The surface suite then passed 325/325, exit 0.

### R4: Prove it on the live install

Evidence. Evidence. On the machine the live install runs on, build lys, lys-identity-server and lys-secrets at the card's head on the build machine through the card's own round, then run `lys identity upgrade --from` that build against the live install, and show the running server's /authority `build` equals the card's head. Record the before and after commits and the upgrade's printed lines in docs/design/directory/PROOF-UPGRADE.md. Secrets and passwords never appear in it. The live install was made before this card (R6), so the proof starts with its first move, and the sign-in page still keeps its session over plain http after the upgrade (the cookie setting carried by R5).

**Acceptance:**
- PROOF-UPGRADE.md names the installed commit before, the head commit after, and quotes the upgrade's lines; `curl -s http://localhost:8490/authority` shows the head commit.
- After the upgrade, signing in on the live install in a browser keeps the session, and the compose environment holds the cookie setting the new build renders.

**Files:**
- create: docs/design/directory/PROOF-UPGRADE.md

**Checklist:**
- C347 — Prove it on the live install (DIRECTORY-045 R4).

**Stories:**
- S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

#### R4 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked. There is no live identity install on this machine: nothing listens on localhost:8490 (lsof shows none), there is no lys directory under ~/Library/Application Support, and the only lys processes are test scratch installs. PROOF-UPGRADE.md cannot truthfully be written without a real upgrade, so it was not created. The steps for the machine that runs the live install, once the card's round builds the head: 1. Read the installed commit from install/build.json. 2. Run `<head build>/lys identity upgrade --from <head build>` (the lys built with the new binaries, or it refuses upgrade_build_differs). 3. Record the printed lines, and `curl -s http://localhost:8490/authority` naming the head commit, in docs/design/directory/PROOF-UPGRADE.md with no secrets. 4. Sign in in a browser, confirm the session survives a reload, and confirm the compose environment names the cookie setting without quoting any value. No command with a time limit is to be used.
- Deviation: Not performed: this machine has no live install. Round 1's attested crossings (a `curl -m 2` probe and a hand-run file-length check) were not repeated. This round's waits used `caffeinate -w` on a process exit and no command carried a time limit.
- Checklist delivery:
  - [ ] C347 — Prove it on the live install (DIRECTORY-045 R4). — Blocked on a live install; PROOF-UPGRADE.md not written.
- Story delivery:
  - [ ] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — The live proof is outstanding.

**Review (recorded):**

- Alignment: drifted
- Acceptance verdicts:
  - [ ] PROOF-UPGRADE.md names the installed commit before, the head commit after, and quotes the upgrade's lines; `curl -s http://localhost:8490/authority` shows the head commit. — docs/design/directory/PROOF-UPGRADE.md does not exist. `lsof -nP -iTCP:8490 -sTCP:LISTEN` prints nothing.
  - [ ] After the upgrade, signing in on the live install in a browser keeps the session, and the compose environment holds the cookie setting the new build renders. — There is no live install on this machine and no upgrade was performed.
- Issues:
  - Run the live upgrade on the machine holding the live identity install, with the lys built at the card's head: `lys identity upgrade --from <head build dir>`. Record the commit before and after, and quote the printed lines, in docs/design/directory/PROOF-UPGRADE.md with no secret. Show `curl -s http://localhost:8490/authority` naming the head commit. Confirm that a browser sign-in survives a reload and that compose.env names the cookie setting, without quoting any value.

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
- How: The new build's configuration and compose files are rendered before anything is stopped. Inside the swap they are kept in config.previous/, placed with a digest read-back and, when compose changed, applied through the Engine. On the way back they are restored and applied again. No credential is generated or written. Unchanged this round.
- Deviation: (none)
- Files changed:
  - created: `crates/lys/src/identity/upgrade/render.rs` — Templates renders compose.yaml, postgres-init.sql, compose.env and identity.json from the recorded choices. It refuses a build other than this lys, and its Debug shows lengths only.
  - created: `crates/lys/src/identity/upgrade/render_tests.rs` — Tests: B runs with its new key and value; a failed B puts A's files back byte for byte; rendering only reads; a missing credential is refused.
  - created: `crates/lys/src/identity/upgrade/swap.rs` — Keeps, places and restores the configuration and compose files beside the binaries.
  - modified: `crates/lys/src/identity/install/server_config.rs` — recorded_administrator reads the administrator identity.json records.
- Checklist delivery:
  - [x] C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7). — Test: render_tests.rs.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — Configuration and compose move with the binaries.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test where build B adds a configuration key and a compose environment value upgrades an install of build A: B runs with the new key and value, and a failed start of B puts A's binaries, configuration and compose files back, byte-identical. — render_tests.rs:25 build_b_runs_with_its_new_key_and_value_and_a_failed_b_puts_a_back. swap.rs:211-254 keeps, places and restores the files.
  - [x] Install run from build B over an install of build A is refused install_build_differs and nothing is stopped. — adopt.rs:277-306 check_placed_build runs in install.rs before anything is stopped. adopt_tests.rs:206 install_from_another_build_is_refused_and_stops_nothing.
  - [x] data/ and every credential are byte-identical across both. — render_tests.rs:25 and adopt_tests.rs:206 compare untouchable() before and after. render.rs only reads credentials.
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
- How: The gate's failure of an_install_made_before_this_card_is_upgraded_and_answers_the_new_build reproduced here (2 of 5 runs). The error was 'process N still runs after caffeinate returned exit status: 0'. `caffeinate -w` returns on the kernel's exit event, but a `ps` right after can still list the exiting process. The exit event is now the answer: a waiter that returns success is the exit, and one that fails is refused with its status. The same test took up to 238 s because macOS was exec'ing the digest-checked copy of the adopted platform binary to read its --version. adopt_running now returns the file the running process was started from, and installed() reads the build from that file, which the copy equals byte for byte. After both fixes the test passed 5 of 5 and the upgrade module 19 of 19.
- Deviation: The gate's first message for this test ('bin/ holds no lys-secrets and none is running to adopt') did not reproduce in any run here. Its cause is not confirmed beyond the load-sensitive process checks fixed above.
- Files changed:
  - modified: `crates/lys/src/identity/upgrade/adopt.rs` — On macOS and the BSDs, stop_through_pid takes the waiter's success as the exit and refuses a waiter that fails, where it used to ask ps afterwards. adopt_running returns the source file, and installed() reads an adopted binary's build from it.
  - created: `crates/lys/src/identity/upgrade/adopt_tests.rs` — Tests: a pre-card install is adopted and upgraded, and /api/authority answers the new build; install run again restarts a process whose binary it placed; install from another build is refused.
  - modified: `crates/lys/src/identity/install/services.rs` — The log wait moved to log_wait.rs.
  - modified: `crates/lys/src/identity/install/exit_wait.rs` — Doc: only upgrade::adopt stops a process that has no exit lock.
- Checklist delivery:
  - [x] C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7). — Test: adopt_tests.rs, the pre-card upgrade.
  - [x] C346 — The install and the server say which build is running (DIRECTORY-045 R3). — build.json names what runs after install and after adoption.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — An install made before builds were named is upgraded in place.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A scratch install laid out as before this card (no build.json, no exit locks, a binary without --version) is upgraded, and /api/authority answers the new commit. — adopt_tests.rs:97 an_install_made_before_this_card_is_upgraded_and_answers_the_new_build asserts adoption, the stop-through-pid lines and /api/authority containing B. Local identity run: 83 passed.
  - [x] Install run again with a changed binary restarts that process, and build.json equals the running build. — adopt_tests.rs:152 install_run_again_restarts_a_process_whose_binary_it_placed. adopt.rs:321-360 settle.
- Checklist verified: C345, C346
- Stories verified: S147
- Issues:
  - The round-1 gate's first message for the adoption test ('bin/ holds no lys-secrets and none is running to adopt') was never reproduced or explained. It passed in this round and locally.

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
- How: The intent is written before the first stop, and each step is recorded as it completes. The next upgrade or install names an unfinished upgrade: one that had started its new build is finished, and any other is put back, so the install runs one whole build. Placement copies to a temporary name, compares digests and renames, and a differing copy is removed with nothing old replaced. This round also fixed the design gate's coverage leg, which was failing on paths from commit b39890e. The four structure rows were added, DESIGN.md was re-rendered, and render, validate and check-coverage on design/directory all passed ('Coverage clean').
- Deviation: The design.json rows belong to DIRECTORY-048 and DIRECTORY-049, not this card. They were added because the gate at this card's head fails without them.
- Files changed:
  - created: `crates/lys/src/identity/upgrade/intent.rs` — install/upgrade.json holds each completed step, written atomically.
  - created: `crates/lys/src/identity/upgrade/intent_tests.rs` — Tests: a kill after bin/ moved is put back, then the upgrade runs; recovery alone; a started build is finished; a corrupted copy is refused.
  - created: `crates/lys/src/identity/install/log_wait.rs` — LogCursor and the log watch, so a readiness wait reads only what is new.
  - created: `crates/lys/src/identity/install/log_wait_tests.rs` — Tests for the cursor.
  - modified: `crates/lys/src/identity/install.rs` — install recovers an unfinished upgrade first.
  - modified: `docs/design/directory/design.json` — Adds structure rows for DIRECTORY-048 R7 (rules/no-app-names.yml, sgconfig.yml) and DIRECTORY-049 R7 (crates/lys/src/cli/mcp.rs, crates/lys/tests/mcp_stdio.rs), so the coverage leg is clean.
  - modified: `docs/design/directory/DESIGN.md` — Re-rendered from design.json.
- Checklist delivery:
  - [x] C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7). — Test: intent_tests.rs.
- Story delivery:
  - [x] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — An interrupted upgrade never leaves a half build.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] A test kills the upgrade after the rename of bin/ and before the last copy; the next upgrade names the unfinished one, finishes it or puts it back, and the install runs one whole build. — intent_tests.rs:67 killed_after_the_first_copy, :83 the_next_upgrade_puts_back_an_upgrade_killed_part_way_then_runs, :121 recovery alone, :144 a started build finished. The cut is simulated in the same process.
  - [x] A test corrupts one placed copy; the digest check refuses it and nothing old is removed. — intent_tests.rs a_copy_that_differs_from_its_source_is_refused_and_nothing_old_goes, with swap.rs:53-106.
  - [x] No file under crates/lys/src/identity/ is 500 lines or more. — The largest is upgrade.rs at 466 lines. file-length.sh: 557 files measured, 0 over the limit.
- Checklist verified: C345
- Stories verified: S147

## Boundaries

- SHALL NOT add any timeout, deadline, sleep or poll interval.
- SHALL NOT touch data/ or any credential in an upgrade. The configuration and compose files are rendered for the new build inside the swap and kept and put back with the binaries (R5); nothing else outside bin/, the screens, the logs, the process files and install/build.json is written.
- SHALL NOT print or record any secret, password or API key, in output, tests or the proof.
- SHALL NOT add #[allow], #[ignore] or any bypass; every file stays under 500 lines of code; ast-grep stays at zero hits.
- Where a file named here does not exist on main at the card's base (About.tsx, a main.rs), the change goes where that concern lives and the dev record names the path.

## Verification

- The full Lys gate and the surface checks exit 0 at the card's head, measured by the card round.
- The live upgrade of R4 is recorded in PROOF-UPGRADE.md and /authority on the live install names the head commit.
