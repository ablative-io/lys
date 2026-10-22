---
type: brief
id: DIRECTORY-115
cluster: directory
title: A refused upgrade leaves no client release behind: releases are placed inside the upgrade's transaction, and a release the manifest does not name never blocks the next upgrade
---

# DIRECTORY-115: A refused upgrade leaves no client release behind: releases are placed inside the upgrade's transaction, and a release the manifest does not name never blocks the next upgrade

> **Cluster:** directory
> **Checklist:**
> - C829 — A refused upgrade removes the client releases it placed, a release the manifest does not name is replaced rather than refused, and the report lists every unreferenced release directory. DIRECTORY-115/R1-R3.
> **Stories:**
> - S468 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the installer of the identity stack, I want a refused upgrade to leave nothing behind and the next build at the same commit to install, so that I never move directories aside by hand to upgrade.

## Purpose

Measured 22 Oct 2026 on Tom's Mac while installing Lys 9861128f. `lys identity upgrade --from target/release --surface <pkg>` places the client releases first (crates/lys/src/identity/upgrade/command.rs: `clients.place(&layout)?` runs before `upgrade(...)`), writing clients/<platform>/<commit>/lys and lys-containment-entry under the identity root, outside the configuration transaction that the rest of the upgrade runs in. The first attempt was refused at a later step. The refusal rolled the transaction back, so clients.json still named the previous release, but the placed files stayed. The release was rebuilt at the same commit; a cargo release build is not byte-reproducible here, so the second `upgrade` was refused at once by ClientSet::place (install/layout/clients.rs): "client_release_changed: the same platform and commit already hold different bytes". The install could not proceed until the stranded directory was moved aside by hand (to /Users/tom/Developer/.scratch/waffles/final/aside-client-9861128f). The same root holds the directories of every earlier commit ever placed (seven and more under aarch64-apple-darwin alone) while clients.json names one: nothing prunes them, and nothing distinguishes a release that is served from bytes that were merely placed.

## Task

Place client releases inside the upgrade's transaction so a refused upgrade leaves nothing behind, and treat a directory at the same commit that the manifest does not name as replaceable rather than as a changed release. In: install/layout/clients.rs ClientSet::place and its callers in upgrade/command.rs and the install command, the configuration transaction's rollback, and the tests of both. Out: pruning releases of older commits that the manifest no longer names (a deletion is Tom's call; this brief only names the pile), the surface package, any change to what a release contains.

## Requirements

### R1: Client releases are placed inside the upgrade's transaction and removed when it is refused

WHEN `lys identity upgrade` runs THE SYSTEM SHALL place the client releases as a step of the same transaction that writes clients.json and the rest of the configuration, after the preparation checks and before the swap, and WHEN that transaction is refused or rolled back at any later step THE SYSTEM SHALL remove every client file and directory the attempt placed (and only those: a directory that existed before the attempt is untouched), so that after a refused upgrade the identity root holds exactly the client releases it held before. The removal is part of the rollback and is reported by name in the upgrade's notes.

**Acceptance:**
- crates/lys/src/identity/upgrade/runner_recovery_tests.rs (or a new clients_rollback_tests.rs beside it): an upgrade whose step after placement is made to fail leaves clients/<platform>/<commit> absent and clients.json unchanged, and the notes carry a line naming the removed release directory.
- The same test with the release directory already present before the attempt (same bytes): the directory is still present after the refused attempt.
- A successful upgrade still ends with clients/<platform>/<commit> placed and clients.json naming it (existing install and upgrade tests stay green).

**Files:**
- create: crates/lys/src/identity/upgrade/clients_rollback_tests.rs
- modify: crates/lys/src/identity/upgrade/command.rs
- modify: crates/lys/src/identity/upgrade/runner.rs
- modify: crates/lys/src/identity/install/layout/clients.rs

**Checklist:**
- C829 — A refused upgrade removes the client releases it placed, a release the manifest does not name is replaced rather than refused, and the report lists every unreferenced release directory. DIRECTORY-115/R1-R3.

**Stories:**
- S468 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the installer of the identity stack, I want a refused upgrade to leave nothing behind and the next build at the same commit to install, so that I never move directories aside by hand to upgrade.

### R2: A directory at the same commit that the manifest does not name is replaced, not refused

WHEN ClientSet::place finds a file at clients/<platform>/<commit>/<name> holding different bytes THE SYSTEM SHALL refuse with client_release_changed only if clients.json names that commit for that platform (the bytes are a served release); otherwise THE SYSTEM SHALL replace the file and note "client_release_replaced: <path> was placed by an earlier attempt that did not finish". A served release at the same commit with different bytes stays refused by name, as today.

**Acceptance:**
- crates/lys/src/identity/install/layout/clients.rs tests: a stranded directory at the commit (manifest names an older commit) with different bytes is replaced and the note is emitted; a served directory (manifest names this commit) with different bytes is refused with client_release_changed; same bytes in either case is a no-op.
- absent_clients_use_the_host_pair_and_never_replace_a_release_at_the_same_commit stays green, reading the manifest to decide.

**Files:**
- modify: crates/lys/src/identity/install/layout/clients.rs

**Checklist:**
- C829 — A refused upgrade removes the client releases it placed, a release the manifest does not name is replaced rather than refused, and the report lists every unreferenced release directory. DIRECTORY-115/R1-R3.

**Stories:**
- S468 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the installer of the identity stack, I want a refused upgrade to leave nothing behind and the next build at the same commit to install, so that I never move directories aside by hand to upgrade.

### R3: The upgrade's report names every release directory the root holds that the manifest does not

WHEN `lys identity upgrade` finishes THE SYSTEM SHALL list, in its notes and in the JSON report under clients_unreferenced, every clients/<platform>/<commit> directory that clients.json does not name, so the operator can see the pile (seven and more on Tom's Mac on 22 Oct). Nothing is deleted by this brief.

**Acceptance:**
- crates/lys/src/identity/upgrade/clients_rollback_tests.rs: with two unreferenced commit directories present, the JSON report lists both paths under clients_unreferenced and the notes carry one line per directory; with none, the list is empty and no line is printed.

**Files:**
- modify: crates/lys/src/identity/upgrade/command.rs
- modify: crates/lys/src/identity/upgrade/clients_rollback_tests.rs

**Checklist:**
- C829 — A refused upgrade removes the client releases it placed, a release the manifest does not name is replaced rather than refused, and the report lists every unreferenced release directory. DIRECTORY-115/R1-R3.

**Stories:**
- S468 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the installer of the identity stack, I want a refused upgrade to leave nothing behind and the next build at the same commit to install, so that I never move directories aside by hand to upgrade.

## Boundaries

- No client release directory is deleted except the ones the refused attempt itself placed; the unreferenced pile is listed, never pruned.
- No change to what a release holds, to the surface package, or to the install command's own first placement beyond sharing the transaction.
- No sleep, timeout, poll or fixed wait; every step is judged by its own event or result.
- Every file stays under the 500-line gate; new tests go in the named new file, not into runner_tests.rs.

## Verification

- Red first: the new tests written and shown red at the brief commit through `cargo nextest run -p lys -E 'test(clients_rollback) | test(client_release)'` on Dean's laptop, green at the fix commit, judged by the run's output lines and never by its exit code.
- The whole gate (tokei, ast-grep, fmt, clippy pedantic, nextest, cargo test --doc, tsc, vitest) at the landing sha, judged by its output.
- Walked on the Mac: an upgrade made to refuse after placement leaves clients/aarch64-apple-darwin without the new commit's directory, and the next upgrade at the same commit proceeds.
- Hospital line in the handback, never in a commit: if this code were used in a hospital, the worst credible failure is an identity server that cannot be upgraded to a security fix until someone finds and moves a stranded directory by hand, and it could harm about everyone that server signs in.

## Amendments

### Amendment 1: R1's file list: where the transaction lives

- **Date:** 2026-10-23
- **By:** Waffles

R1 named crates/lys/src/identity/upgrade/runner.rs as the transaction; that file is the process runner (Restart), untouched. The transaction is upgrade.rs (Parts gains `clients: Option<&ClientSet>`, placement after the intent record and before the swap as `place_clients`), swap.rs (`back` removes what the intent names) and intent.rs (`Step::ClientsPlaced`, `clients_placed: PlacedClients`), so those three are modified in runner.rs's place. install/layout.rs exports PlacedClients. Every fixture that builds Parts or Intent takes the new field: examples/upgrade_window.rs, scratch_tests.rs, runner_tests.rs, runner_recovery_tests.rs, render_tests.rs, stores_receipt_tests.rs, companion_tests.rs, intent_tests.rs, data_kept_tests.rs and upgrade_tests.rs (which also mounts clients_rollback_tests.rs and now passes its client set through Parts). The install command's own first placement keeps its place outside a transaction, as the boundary says, and only takes the new signature. A replaced stranded file (R2) counts as placed by the attempt and is removed on rollback; a directory that existed before is left, as R1 says.

### Amendment 2: R3's file list: where the walk lives

- **Date:** 2026-10-23
- **By:** Waffles

The walk of clients/<platform>/<commit> against clients.json is `unreferenced_releases` in crates/lys/src/identity/install/layout/clients.rs, beside `client_releases` which reads the manifest, exported through install/layout.rs; upgrade/command.rs keeps the saying (`say_unreferenced`, one line per directory) and the JSON field (`report_unreferenced`, `clients_unreferenced`, an empty list when every release is named). Both files are modified in addition to R3's two.
