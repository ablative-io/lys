---
type: brief
id: DIRECTORY-070
cluster: directory
title: An install's ports are its own configuration, and the install test runs on free ports in its own folder beside a live Lys
---

# DIRECTORY-070: An install's ports are its own configuration, and the install test runs on free ports in its own folder beside a live Lys

> **Cluster:** directory
> **Depends on:** DIRECTORY-045, DIRECTORY-047
> **Design anchor:**
> - ADR-114 — An installed identity product names its build and upgrades itself in place, keeping the previous build to return to — Every Lys binary answers --version with the commit and dirty state it was built from, stamped at build time; a build with no commit to read says so in those words, never a made-up value. `lys identity upgrade` takes a folder of newly built binaries (and optionally a screens package), checks each one's --version, stops the service and the broker by their exit events, swaps the binaries by rename keeping the previous set beside them, starts the new ones and waits for them ready; if any new binary fails to start or become ready, it puts the previous set back, starts it, and fails naming what broke. Data, credentials, configuration and the compose services are never touched by an upgrade. Rejected: making install restart what differs, which would mix a first install's promises with an upgrade's risks; asking people to copy binaries by hand.
> - ADR-113 — The identity install and its clients wait on signals, never on a clock — A wait ends on the event it is waiting for or on the failure that makes the event impossible: a service is ready when it says so or when a single connection to it succeeds after a readiness event (its log gaining its listening line, observed through the platform's file-change notification), a stopped process is gone when the platform reports its exit (kqueue EVFILT_PROC on macOS, pidfd on Linux), a peer that refuses or closes is an error at once, and nothing carries a timeout. The screen re-measures on a ResizeObserver or animation-frame signal. Rejected: shorter sleeps, which still wait on a clock; keeping timeouts as a safety net, which the rule forbids and which hides a stuck peer's cause.
> **Checklist:**
> - C459 — The service and broker ports are members of the deployment configuration (DIRECTORY-070 R1).
> - C460 — An earlier install keeps its ports through upgrade (DIRECTORY-070 R2).
> - C461 — The install test runs on free ports in its own folder, two at once, beside a live Lys (DIRECTORY-070 R3).
> **Stories:**
> - S184 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want its ports to be part of its own configuration and its install test to use ports and a folder of its own, so that testing an install never refuses, disturbs or depends on the Lys I am running.

## Purpose

The identity service's port and the secrets broker's port are constants of the product (layout.rs SERVICE_PORT 8490 and BROKER_PORT 8472), read by the service URL and the deployment it renders (layout.rs lines 178 and 231), the server configuration's listen and broker addresses (server_config.rs lines 88 and 120) and the upgrade's broker arguments and readiness checks (upgrade.rs lines 101, 114 and 124). The deployment file already carries the service port inside public_origin and the redirect addresses, but nothing reads a port back from it. So a second install on one machine cannot run, and the install test (crates/lys/tests/identity_install.rs) must use the same two ports: it refuses when anything listens on them (port_free, port_in_use), so it cannot run beside a live Lys and two copies cannot run at once. It also builds its binaries with --target-dir into target/identity-install, a second target directory that is never removed. The live upgrade gate reads the old ports by searching layout.rs for the constants (scripts/identity-gates/upgrade_live.py installation), so it breaks as soon as they move.

## Task

Make the two ports members of the install's deployment configuration, written by the install, read by the install, the server configuration and the upgrade, with an earlier install that names neither keeping the ports it has today. Rebuild the install test on ports the operating system hands it, in a folder of its own that is removed at the end, with its binaries built into the workspace's own target directory, so two copies pass at once beside a live Lys.

## Requirements

### R1: The service and broker ports are members of the deployment configuration

Behavioural. WHEN the install writes or reads an install's deployment configuration, THE SYSTEM SHALL carry the identity service's loopback port and the secrets broker's loopback port as members of its [deployment] table (config.rs Deployment), and SHALL take both from that configuration for the service URL and setup address it prints, the rendered public origin and redirect addresses, the server configuration's listen and broker addresses, the broker's --listen argument and the readiness checks of install and upgrade. A new install with no ports named writes 8490 and 8472 into its configuration. The two ports must differ from each other and from every other port the configuration names; a clash is refused by name before anything starts, naming both members. The constants SERVICE_PORT and BROKER_PORT are removed from layout.rs and no product code names 8490 or 8472 outside the template's default.

**Acceptance:**
- An install given a deployment configuration naming two free ports serves its screens and broker on those ports.
- The setup address it prints names the configured service port.
- A new install with no ports named records 8490 and 8472 in its configuration.
- A configuration naming the same port twice is refused by name before anything starts.
- No product source outside the deployment template names 8490 or 8472.

**Files:**
- create: crates/lys/src/identity/install/ports_tests.rs
- modify: crates/lys/src/identity/config.rs
- modify: crates/lys/src/identity/config/validate.rs
- modify: crates/lys/src/identity/install/layout.rs
- modify: crates/lys/src/identity/install/deployment.template.toml
- modify: crates/lys/src/identity/install/server_config.rs
- modify: crates/lys/src/identity/install/services.rs
- modify: crates/lys/src/identity/upgrade.rs
- modify: crates/lys/src/identity/install_tests.rs

**Checklist:**
- C459 — The service and broker ports are members of the deployment configuration (DIRECTORY-070 R1).

**Stories:**
- S184 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want its ports to be part of its own configuration and its install test to use ports and a folder of its own, so that testing an install never refuses, disturbs or depends on the Lys I am running.

### R2: An earlier install keeps its ports through upgrade

Behavioural. WHEN an install made by an earlier build, whose deployment configuration names neither port, is upgraded in place, THE SYSTEM SHALL read the absent members as 8490 and 8472, the ports that build served on, and SHALL record them into the configuration once as part of the upgrade, so the upgraded install answers on the same ports and a rollback to the earlier build still reads a configuration it accepts. Nobody edits an installed configuration by hand. The live upgrade gate (upgrade_live.py installation) takes the old install's ports from its own deployment configuration, or the earlier defaults when it names none, instead of searching layout.rs. This is tested from a real install made by build 1b568cd9, which can only serve on 8490 and 8472: after upgrade the screens and broker answer on those same ports, and the rollback leg starts the earlier build again on them.

**Acceptance:**
- An install made by build 1b568cd9 answers on the same service and broker ports after upgrade.
- The upgraded configuration names both ports, recorded once.
- A rollback to the earlier build starts and answers on the same ports.
- The live upgrade gate no longer reads layout.rs for ports.

**Files:**
- modify: crates/lys/src/identity/upgrade.rs
- modify: crates/lys/src/identity/config.rs
- modify: scripts/identity-gates/upgrade_live.py
- modify: scripts/identity-gates/test_upgrade_fixture.py
- modify: scripts/identity-gates/UPGRADE.md

**Checklist:**
- C460 — An earlier install keeps its ports through upgrade (DIRECTORY-070 R2).

**Stories:**
- S184 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want its ports to be part of its own configuration and its install test to use ports and a folder of its own, so that testing an install never refuses, disturbs or depends on the Lys I am running.

### R3: The install test runs on free ports in its own folder, two at once, beside a live Lys

Behavioural. WHEN the install test runs (identity_install.rs), THE SYSTEM SHALL take the service and broker ports from the operating system as it already takes the sign-in and database ports (identity_support fixtures free_port), write them into the estate's deployment configuration, and address every request and the origin from them; the fixed SERVICE_PORT, BROKER_PORT and ORIGIN constants and the port_free refusal are removed. Everything the test makes lives under temporary folders it owns, removed when the test ends whether it passes or fails, along with its compose project and volumes. Its binaries are built with the workspace's own cargo into the workspace's own target directory, with no --target-dir and no CARGO_TARGET_DIR, and target/identity-install is no longer made. The cli setup test (cli_tests/identity_setup.rs) takes its service port the same way. Proven by running two copies of the test at once on one machine while a listener the proof itself holds on 127.0.0.1:8490 and 127.0.0.1:8472 stays bound: both copies pass, and the held listeners are still bound and were never connected to.

**Acceptance:**
- The test serves and signs in on ports the operating system handed it.
- Two copies of the test run at once on one machine and both pass.
- A listener held on 8490 and 8472 during both runs is still bound afterwards and was never connected to.
- No folder, compose project or volume the test made remains after it passes or fails.
- The test build names no --target-dir and no target/identity-install directory is made.

**Files:**
- modify: crates/lys/tests/identity_install.rs
- modify: crates/lys/tests/cli_tests/identity_setup.rs
- modify: crates/lys/tests/identity_support/fixtures.rs

**Checklist:**
- C461 — The install test runs on free ports in its own folder, two at once, beside a live Lys (DIRECTORY-070 R3).

**Stories:**
- S184 (Installer and operator, Provision and upgrade the internal audit connection) — As the person running a Lys install, I want its ports to be part of its own configuration and its install test to use ports and a folder of its own, so that testing an install never refuses, disturbs or depends on the Lys I am running.

## Boundaries

- SHALL NOT refuse to run because another install holds 8490 or 8472, and SHALL NOT stop, connect to or change a live Lys.
- SHALL NOT use --target-dir, CARGO_TARGET_DIR or a cargo configuration target-dir, and SHALL NOT leave a second target directory behind.
- SHALL NOT edit an installed configuration by hand; an earlier install's ports are carried by the upgrade and tested from a real earlier install.
- SHALL NOT write anything outside the test's own temporary folders and the workspace target.
- SHALL NOT add a timeout, deadline, watchdog, poll, sleep, unsafe, ignored test or lint suppression.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Implementation follows card_build_v3, src_pr and src_land with Jev, fmt, Clippy pedantic, tests, ast-grep and the full gate on Dean.
- The concurrency proof runs two copies of the install test at once on Dean at normal width with 8490 and 8472 held by the proof, and keeps both logs.
- The upgrade leg starts from a real install made by build 1b568cd9 and checks both ports after upgrade and after rollback.
