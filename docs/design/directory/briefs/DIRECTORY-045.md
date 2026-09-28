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
- How: Row 1 (each binary prints its name, crate version and the 40-character head commit, and an exported-tree build shows the stated words): met, not yet run. The stamp is written at crates/lys/build.rs:64-69 and emitted at build.rs:82; the no-commit words are at build.rs:20 and 46-48. The version strings are at crates/lys/src/main.rs:27, crates/lys-secrets/src/bin/lys-secrets/main.rs (VERSION const before main), crates/lys-home/src/main.rs:8 and crates/lys-identity-server/src/main.rs:11-17. The tests are crates/{lys,lys-identity-server,lys-secrets,lys-home}/tests/version.rs::version_names_the_crate_version_and_the_head_commit. The exported-tree test is crates/lys/tests/version.rs::a_build_from_an_exported_tree_says_it_has_no_commit, and the commit and dirty test is a_build_from_a_commit_names_it_and_its_dirty_state. Row 2 (lys-identity-server --version exits 0 and reads no file): met, not yet run. The flag is handled before any path is formed (crates/lys-identity-server/src/main.rs, `if first == "--version" || first == "-V"`), and the lys-identity-server version test runs in a folder where files named after the flags hold no configuration.
- Deviation: The exported-tree test builds a small probe crate whose build script is the real build.rs, copied into a temporary tree with no .git (GIT_CEILING_DIRECTORIES stops git from finding a parent repository). It does not rebuild the whole workspace from a git archive. The words come only from build.rs, and the four build.rs files are held byte-identical by a test, so the probe measures the same code. A full workspace rebuild inside the test suite would have been a second heavy build.
- Files changed:
  - created: `crates/lys/build.rs` — Sets LYS_BUILD to `git rev-parse HEAD` plus `; dirty` when `git status --porcelain` lists changes. It uses --no-optional-locks so the index is never rewritten. It says `not built from a git commit` when there is no git tree, or when the tree found does not track this crate's Cargo.toml. It is rerun when HEAD, the index, packed-refs or the branch's ref moves. Standard library only.
  - created: `crates/lys-identity-server/build.rs` — Byte-identical copy of crates/lys/build.rs.
  - created: `crates/lys-secrets/build.rs` — Byte-identical copy of crates/lys/build.rs.
  - created: `crates/lys-home/build.rs` — Byte-identical copy of crates/lys/build.rs.
  - modified: `crates/lys/src/main.rs` — Parses with Cli::command().version(VERSION), where VERSION is `CARGO_PKG_VERSION (LYS_BUILD)`. Dispatches the new identity Upgrade verb.
  - modified: `crates/lys-identity-server/src/main.rs` — A first argument of --version or -V prints `lys-identity-server VERSION (BUILD)` and exits 0 before any configuration is read. It prints its build line when it starts serving.
  - modified: `crates/lys-secrets/src/bin/lys-secrets/main.rs` — Parses with the stamped VERSION, so --version and -V print `lys-secrets VERSION (COMMIT)`.
  - modified: `crates/lys-home/src/main.rs` — Parses with the stamped VERSION, so --version and -V print `lys-home VERSION (COMMIT)`.
  - created: `crates/lys/tests/version.rs` — Checks that lys --version and -V equal `lys VERSION (HEAD[; dirty])`, with HEAD read from git at test time. Checks that the four build.rs files are byte-identical. Builds a probe crate that uses this build.rs from an exported tree with no .git and asserts the words. Builds it again from a fresh git commit and asserts the 40-character commit, then `; dirty` after a change.
  - created: `crates/lys-identity-server/tests/version.rs` — Checks that --version and -V exit 0 with the head commit. They run in a folder holding files literally named `--version` and `-V` that are not configuration, so any attempt to read them as configuration would fail.
  - created: `crates/lys-secrets/tests/version.rs` — Checks that --version and -V print `lys-secrets VERSION (HEAD[; dirty])`.
  - created: `crates/lys-home/tests/version.rs` — Checks that --version and -V print `lys-home VERSION (HEAD[; dirty])`.
- Checklist delivery:
  - [x] C344 — Every Lys binary answers --version with its build commit (DIRECTORY-045 R1). — Every one of the four binaries answers --version and -V with its build commit. The tests are written but have not been run.
- Story delivery:
  - [ ] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — The part about seeing the build is delivered. The story as a whole also rides on R4, the live upgrade, which is blocked.

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
- How: Row 1 (A installed, upgraded to B, B running with A kept in bin.previous/; a B service that exits before ready gets A restored, running and ready, with the failure named): met, not yet run. The tests are crates/lys/src/identity/upgrade_tests.rs::an_upgrade_runs_the_new_build_and_keeps_the_previous and a_service_that_exits_before_ready_puts_the_previous_build_back. In crates/lys/src/identity/upgrade.rs:
- `upgrade` (lines 356-413) reads every version before it stops anything, then stops the service and then the broker through services::stop, which waits on the exit lock (stop_all, line 330).
- swap_in (line 286) moves bin/ to bin.previous/, replacing an older one, and places each binary through place_binary (line 153). The screens go through surface::place, keeping surface.previous/.
- start_all (line 339) starts the broker and then the service. Each is waited on through services::wait_until, which checks once on each log-change event and refuses when the process exits (launch, line 136).
- On failure, put_back (line 314) and a restart run, and the error is UpgradeFailed naming the binary and its log (lines 342-346, 407-411).
Row 2 (data/, identity.json and deployment.toml byte-identical): met, not yet run. The test is upgrade_tests.rs::neither_upgrade_touches_data_or_configuration, which asserts three files are compared. The module has no write path to those files (upgrade.rs:1-22 states the invariant). Row 3 (grep -nE 'sleep|timeout' over the new files prints nothing): met. I grepped every new file and it printed nothing. The refusals are tested in refusals_are_named_and_stop_nothing: not_installed, binary_missing and version_unreadable, with A still running and no bin.previous/ made.
- Deviation: Four things differ from the brief, all visible in the diff:
1. Files outside the R2 list were edited: error.rs and error_tests.rs (the named refusal kinds), mod.rs (the module declaration) and services.rs (wait_answering deleted once it had no caller).
2. Readiness now needs the process's own `listening on` line in its log since it started, and then the port answering. Before, only the port answering was checked. This lets the tests use stubs that answer no HTTP, avoids a dead-code variant used only by tests, and follows ADR-113's 'ready when it says so'.
3. Upgrade refuses a root with no bin/ as not_installed, telling the operator to run `lys identity install` from the new build first, because an install made before this change has no bin/ to keep.
4. CN1 says documents only, but this brief's requirements name code files, so I followed the requirements.
- Files changed:
  - created: `crates/lys/src/identity/upgrade.rs` — The new `lys identity upgrade` module, with the pieces the install now shares: Unit (the broker and the service), Ready (the log line, then the port answering), launch, place_binary (copy to a temporary name, then rename), version (reads `NAME VERSION (COMMIT)`), record_build (install/build.json) and the upgrade itself with its return to the previous build.
  - created: `crates/lys/src/identity/upgrade_tests.rs` — Scratch-root tests with /bin/sh stub builds. A is installed and running; B is upgraded to and runs, with A kept in bin.previous/. A B service that exits before ready gets A restored, running and ready, with the failure named. data/, identity.json and deployment.toml stay byte-identical across a successful and a failed upgrade. The three refusals are checked by name, and version parsing is checked.
  - modified: `crates/lys/src/identity/cli.rs` — Adds the Upgrade verb with --from, --surface and --root.
  - modified: `crates/lys/src/identity/install/layout.rs` — Adds BINARIES (lys-secrets, lys-identity-server) and the paths bin/, bin.previous/, surface.previous/, install/ and install/build.json.
  - modified: `crates/lys/src/identity/mod.rs` — Declares the upgrade module.
  - modified: `crates/lys/src/identity/error.rs` — Adds four refusal kinds: NotInstalled (not_installed), BinaryMissing (binary_missing), VersionUnreadable (version_unreadable) and UpgradeFailed (upgrade_failed).
  - modified: `crates/lys/src/identity/error_tests.rs` — The distinct-names test lists the four new kinds.
  - modified: `crates/lys/src/identity/install/services.rs` — Removes wait_answering, which had no caller left: readiness now goes through upgrade::launch, which calls wait_until with the log line and the port.
- Checklist delivery:
  - [x] C345 — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7). — Upgrade swaps the binaries and screens and returns to the previous build on failure. The tests are written but have not been run.
- Story delivery:
  - [ ] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — The upgrade and the return to the previous build are delivered. The story also rides on R4, which is blocked.

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
- How: Row 1 (a running server built at the card's head answers /authority with its commit in `build`): met, not yet run. The route is registered in crates/lys-identity-server/src/read_api.rs (routes) and answered by the `authority` handler with json!({"authority": AUTHORITY, "build": BUILD}). The test is crates/lys-identity-server/tests/authority_build.rs::the_authority_names_the_build_that_answers. Row 2 (a surface test renders the build commit from a stubbed /authority): met, not yet run. The rendering is in surface/identity/src/features/signin/Gate.tsx inside SignIn's details, and the test is surface/identity/tests/build.test.tsx. The spec's install/build.json is written by upgrade::record_build (crates/lys/src/identity/upgrade.rs:205-254) with each binary's commit and the screens' manifest SHA-256. It is called from install.rs after the units start and from upgrade after success and after a return to the previous build.
- Deviation: surface/identity/src/shell/About.tsx does not exist on main. The brief's boundary sends the change to where the concern lives, and the only view that lists the authority is SignIn in surface/identity/src/features/signin/Gate.tsx, so the build is shown there. Other files outside the list were edited: api.ts, fixtures.ts, and routes.rs (the /authority route moved into read_api.rs). /authority changed from plain text to JSON so it can carry `build`. The only reader of it was the screens, and they are updated.
- Files changed:
  - modified: `crates/lys/src/identity/install.rs` — Places lys-secrets and lys-identity-server into bin/ from beside lys when they are absent, never replacing one already there. Starts both from bin/ through upgrade::launch. Writes install/build.json and prints each binary's commit and the screens' commit and manifest SHA-256; under --json they come back as a `build` field.
  - modified: `crates/lys-identity-server/src/read_api.rs` — Now owns GET /authority and answers it as JSON {authority, build}, where BUILD is env!("LYS_BUILD"). The module docs state that the value is never empty or invented.
  - modified: `crates/lys-identity-server/src/routes.rs` — Drops its plain-text /authority route and handler, which moved to read_api.rs; axum refuses a route registered twice.
  - created: `crates/lys-identity-server/tests/authority_build.rs` — Starts the service through the identity_contract harness, reads /authority, and checks that `authority` is AUTHORITY and that `build` equals the head commit git names (allowing `; dirty`).
  - modified: `surface/identity/src/api.ts` — api.authority reads the JSON answer as AuthorityAnswer {authority, build} through the shared request path, so refusals and unreadable answers are named as elsewhere.
  - modified: `surface/identity/src/features/signin/Gate.tsx` — The sign-in view's 'Advanced: how access is managed' section, the one view that lists the authority, now shows `Build <commit>` below it.
  - modified: `surface/identity/tests/fixtures.ts` — The stubbed /authority answer is {authority, build: BUILD}, with a BUILD constant.
  - created: `surface/identity/tests/build.test.tsx` — Renders the sign-in view from a stubbed /authority answer and finds `Build <commit>`. A second case shows the no-commit words exactly as the service gives them.
- Checklist delivery:
  - [x] C346 — The install and the server say which build is running (DIRECTORY-045 R3). — The install, the upgrade, the server and the screens all name the running build. The tests are written but have not been run.
- Story delivery:
  - [ ] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — Seeing the running build is delivered. The story also rides on R4, which is blocked.

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
- How: Row 1 (PROOF-UPGRADE.md names the commit before and after and quotes the upgrade's lines; curl http://localhost:8490/authority shows the head commit): not met. This session's machine has no live identity install: nothing answers on localhost:8490, and there is no data root at ~/Library/Application Support/lys/identity. The brief also says to run no build, so no binaries exist at the card's head to upgrade to. The proof has to be made on the machine the live install runs on, after the card's round builds the head. The steps there:
1. That live install predates bin/, so first run `lys identity install` from the new build. It places bin/ from the new siblings and, with the configuration unchanged, restarts nothing.
2. Run `lys identity upgrade --from <that build's target dir>`.
3. Record the printed lines and `curl -s http://localhost:8490/api/authority`, or /authority when the install has no screens, in docs/design/directory/PROOF-UPGRADE.md.
I did not create PROOF-UPGRADE.md, so that no evidence is invented.
- Deviation: Not performed: there is no live install on this machine and no head build to use.
- Checklist delivery:
  - [ ] C347 — Prove it on the live install (DIRECTORY-045 R4). — Blocked: the live upgrade and its proof are for the machine that runs the live install, after the card's round builds the head.
- Story delivery:
  - [ ] S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in. — Not demonstrated on the live install.

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

## Boundaries

- SHALL NOT add any timeout, deadline, sleep or poll interval.
- SHALL NOT touch data/ or any credential in an upgrade. The configuration and compose files are rendered for the new build inside the swap and kept and put back with the binaries (R5); nothing else outside bin/, the screens, the logs, the process files and install/build.json is written.
- SHALL NOT print or record any secret, password or API key, in output, tests or the proof.
- SHALL NOT add #[allow], #[ignore] or any bypass; every file stays under 500 lines of code; ast-grep stays at zero hits.
- Where a file named here does not exist on main at the card's base (About.tsx, a main.rs), the change goes where that concern lives and the dev record names the path.

## Verification

- The full Lys gate and the surface checks exit 0 at the card's head, measured by the card round.
- The live upgrade of R4 is recorded in PROOF-UPGRADE.md and /authority on the live install names the head commit.
