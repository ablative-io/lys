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

Stamp the build commit into every Lys binary, add `lys identity upgrade`, surface the running build in /authority and the screens, and prove the upgrade on a scratch install in the round, with the live install upgraded and recorded by the landing lead after landing.

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

### R4: Prove the upgrade on a scratch install in the round, and on the live install after landing

Evidence, in two parts. In the round: the upgrade tests drive `upgrade` end to end against a scratch install each test makes in its own temporary directory (build A's binaries running with the configuration and compose files A renders, then build B's), through the engine that brings the compose services to their definition, so neither a container runtime nor the live install is needed. The build machine has neither, and the round never reaches the live install or its state directory. After landing: the lead who lands the card upgrades the live install with the landed build's `lys identity upgrade --from`, and commits docs/design/directory/PROOF-UPGRADE.md to main naming the installed commit before, the landed commit after, the upgrade's printed lines, the `build` member of http://localhost:8490/api/authority, and the cookie setting in the install's compose environment. Secrets and passwords never appear in it. The live install was made before this card (R6), so that upgrade starts with its first move. The round does not write PROOF-UPGRADE.md.

**Acceptance:**
- A test starts build A on a scratch install in a temporary directory, upgrades it to build B, and asserts that the recorded build and every binary's --version name B, that the configuration key and compose environment value only B renders are in place, and that config.previous/ holds A's files.
- A test in which build B exits before it is ready asserts that A is running again, A is the recorded build, and A's configuration and compose files are back in place.
- No test or leg of the round needs a container runtime, a listener on port 8490 or the live install's state directory, and each test removes its temporary install when it ends.

**Checklist:**
- C347 — Prove the upgrade on a scratch install in the round; the live install is upgraded and recorded after landing (DIRECTORY-045 R4).

**Stories:**
- S147 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person running the identity product, I want to see which build is running and take a newer one with one command that puts the old one back if the new one fails, so that each landing reaches me and a bad build never leaves me without sign-in.

### R5: The configuration and compose files move with the binaries

Behavioural. An upgrade renders deployment.toml's derived files, the directory service's configuration and the compose files (compose.yaml and its environment) for the new build, from the install's recorded choices and the new build's templates, inside the swap: the previous ones are kept beside them (config.previous/) and put back with the binaries on failure and by --back (DIRECTORY-053). The compose services whose rendered definition changed are recreated through the engine, waited on for ready, and put back on failure. Running install from a build other than the one placed is refused install_build_differs, naming the placed and the new commit and the upgrade command, never restarting old binaries on a new configuration. The module note at the head of upgrade.rs says what this requirement makes true. It no longer says an upgrade never writes deployment.toml, identity.json or the compose services, and it names each file an upgrade renders and keeps in config.previous/.

**Acceptance:**
- A test where build B adds a configuration key and a compose environment value upgrades an install of build A: B runs with the new key and value, and a failed start of B puts A's binaries, configuration and compose files back, byte-identical.
- Install run from build B over an install of build A is refused install_build_differs and nothing is stopped.
- data/ and every credential are byte-identical across both.
- The module note of upgrade.rs names every file an upgrade renders, and no file it calls never written is one an upgrade writes.

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

Behavioural. An install with no install/build.json (made before this card) is upgraded, not re-installed: upgrade reads the placed binaries' versions (a binary that cannot answer --version is recorded as 'built before build stamps'), stops the processes its pid files name through their exit locks, or through the pid when a process has no exit lock, naming which, and proceeds as R2 and R5. Install run again never leaves a process running from a binary it replaced: it stops and restarts any process whose binary it changed, so install/build.json always names what runs. The binaries placed before this card have no build stamp. lys-identity-server reads --version as a configuration path and fails, and lys, lys-secrets and lys-home refuse it as an unexpected argument. Any installed binary whose --version exits non-zero or prints no stamp is recorded as 'built before build stamps' and the upgrade carries on. It is refused version_unreadable only for a binary in the new build folder.

**Acceptance:**
- A scratch install laid out as before this card (no build.json, no exit locks, a binary without --version) is upgraded, and /api/authority answers the new commit.
- Install run again with a changed binary restarts that process, and build.json equals the running build.
- A test places an installed lys-identity-server stub that fails when given --version as a configuration path and a lys stub that exits 2 with an unexpected argument, and the upgrade records both as 'built before build stamps' and carries on to the new build.

**Files:**
- create: crates/lys/src/identity/upgrade/adopt.rs
- create: crates/lys/src/identity/upgrade/adopt_tests.rs
- modify: crates/lys/src/identity/install/services.rs
- modify: crates/lys/src/identity/upgrade.rs

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
- The upgrade tests of R4 pass in the round's test leg on a build machine with no container runtime and no live install.
