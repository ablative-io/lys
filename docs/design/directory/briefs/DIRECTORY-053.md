---
type: brief
id: DIRECTORY-053
cluster: directory
title: Go back to the previous build after an upgrade that succeeded, and one build stamp for every binary
---

# DIRECTORY-053: Go back to the previous build after an upgrade that succeeded, and one build stamp for every binary

> **Cluster:** directory
> **Depends on:** DIRECTORY-045
> **Checklist:**
> - C393 — `lys identity upgrade --back` returns a running install to the build kept in bin.previous (and surface.previous), stopped, swapped, started and waited on for ready exactly as an upgrade is, and records the build now running (DIRECTORY-053 R1).
> - C394 — Every Lys binary takes its build stamp from one shared build-support crate; no build.rs is copied (DIRECTORY-053 R2).
> **Stories:**
> - S159 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who updated Lys, I want to go back to the build I had before with one command when the new one misbehaves after it started fine, so that a bad update never leaves me stuck.

## Purpose

DIRECTORY-045 upgrades a running install and returns to the previous build only when the upgrade itself fails. A build that starts ready and misbehaves an hour later has no way back but hand moves in the install folder, which an ordinary person cannot make. Its build stamp is also four byte-identical build.rs files that a test holds equal: one script copied four times, not one script. Amended 28 September 2026 21:05 by Waffles after Archie's read against the tree.

## Task

Add `lys identity upgrade --back`, which returns the install to the build kept beside it through the same stop, swap, start and ready path as an upgrade; and move the build stamp into one build-support crate that each binary's build.rs calls.

## Requirements

### R1: `lys identity upgrade --back`

Behavioural. `lys identity upgrade --back` (the same verb, so there is one path) reads the version of every binary in bin/ and in bin.previous/ before stopping anything, and refuses by name when there is no bin.previous/ (nothing_to_return_to) or a version cannot be read. It then stops the broker and the service through their exit locks, exchanges bin/ with bin.previous/, the configuration and compose files with config.previous/ (DIRECTORY-045 R5) and surface/ with surface.previous/ when that exists, recreating any compose service whose definition changes (so the build it leaves becomes the one kept, and --back again returns to it), starts both and waits on their readiness events, and writes install/build.json. A failure on the way puts back what it moved, exactly as an upgrade does, and names the binary and its log. It never writes data/ or a credential, and it reads and honours an unfinished upgrade's intent record first (DIRECTORY-045 R7). The Build line on the account screen shows the build now running. Each build records the newest data format it writes (the log event kinds it can read, as a format number in install/build.json and its --version detail). Before stopping anything, --back compares the kept build's format with the data's, and when the older build cannot read the data it refuses back_would_not_read, naming the first event kind the older build does not know. Data is never rolled back.

**Acceptance:**
- After an upgrade that succeeded, --back leaves the earlier commit running, answering /api/authority with that build, and --back again returns to the newer one.
- With no bin.previous/ it is refused nothing_to_return_to and nothing is stopped.
- A start failure during --back leaves the build it began from running and names the binary and its log.
- After an upgrade whose build writes a new event kind, --back is refused back_would_not_read naming that kind, and nothing is stopped.

**Files:**
- create: crates/lys/src/identity/upgrade_back_tests.rs
- modify: crates/lys/src/identity/upgrade.rs
- modify: crates/lys/src/identity/cli.rs
- modify: crates/lys/src/identity/error.rs

**Checklist:**
- C393 — `lys identity upgrade --back` returns a running install to the build kept in bin.previous (and surface.previous), stopped, swapped, started and waited on for ready exactly as an upgrade is, and records the build now running (DIRECTORY-053 R1).

**Stories:**
- S159 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who updated Lys, I want to go back to the build I had before with one command when the new one misbehaves after it started fine, so that a bad update never leaves me stuck.

### R2: One build stamp

Structural. A build-support crate, crates/lys-build-stamp, holds the one function that computes the stamp (head commit, `; dirty`, or `not built from a git commit`) and the rerun-if lines. Each of lys, lys-identity-server, lys-secrets and lys-home lists it under build-dependencies and its build.rs is one call. The byte-identity test is removed with the copies; the stamp's own tests (exported tree, fresh commit, dirty) move into the crate and run against it.

**Acceptance:**
- No two build.rs files in the workspace hold the stamp logic; each is one call.
- The crate's tests prove the three stamp forms.
- Every binary's --version still prints its name, version and commit.

**Files:**
- create: crates/lys-build-stamp/Cargo.toml
- create: crates/lys-build-stamp/src/lib.rs
- create: crates/lys-build-stamp/tests/stamp.rs
- modify: Cargo.toml
- modify: crates/lys/build.rs
- modify: crates/lys-identity-server/build.rs
- modify: crates/lys-secrets/build.rs
- modify: crates/lys-home/build.rs
- modify: crates/lys/Cargo.toml
- modify: crates/lys-identity-server/Cargo.toml
- modify: crates/lys-secrets/Cargo.toml
- modify: crates/lys-home/Cargo.toml
- modify: crates/lys/tests/version.rs

**Checklist:**
- C394 — Every Lys binary takes its build stamp from one shared build-support crate; no build.rs is copied (DIRECTORY-053 R2).

**Stories:**
- S159 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who updated Lys, I want to go back to the build I had before with one command when the new one misbehaves after it started fine, so that a bad update never leaves me stuck.

## Boundaries

- SHALL NOT change what an upgrade writes beyond bin/, bin.previous/, the configuration and compose files and config.previous/, the screens, the logs, the process files, install/upgrade.json and install/build.json; data is never rolled back.
- SHALL NOT add a timeout, deadline, poll, #[allow], #[ignore] or any bypass.
- SHALL NOT add a silent fallback: every failure is a named refusal.

## Verification

- The full Lys gate, ast-grep scan and surface checks exit 0 at the card's head, measured by the card round.
- On a scratch install: upgrade to a newer build, --back, and read the older commit from /api/authority; --back again and read the newer one.
