---
type: brief
id: DIRECTORY-082
cluster: directory
title: A put-back keeps the window's writes beside the data it restores, and the upgrade proof reads segment stores
---

# DIRECTORY-082: A put-back keeps the window's writes beside the data it restores, and the upgrade proof reads segment stores

> **Cluster:** directory
> **Depends on:** DIRECTORY-053
> **Design anchor:**
> - ADR-114 — An installed identity product names its build and upgrades itself in place, keeping the previous build to return to — Every Lys binary answers --version with the commit and dirty state it was built from, stamped at build time; a build with no commit to read says so in those words, never a made-up value. An exported build may be given LYS_BUILD_COMMIT as exactly 40 lowercase hexadecimal characters: its stamp is `<commit>; stated`, distinguishing the builder's statement from a commit read from Git; invalid values or disagreement with a tracked checkout's HEAD refuse the build, and changing or removing the variable refreshes the stamp. `lys identity upgrade` takes a folder of newly built binaries (and optionally a screens package), checks each one's --version, stops the service and the broker by their exit events, swaps the binaries by rename keeping the previous set beside them, starts the new ones and waits for them ready; if any new binary fails to start or become ready, it puts the previous set back, starts it, and fails naming what broke. Data, credentials, configuration and the compose services are never touched by an upgrade. Rejected: making install restart what differs, which would mix a first install's promises with an upgrade's risks; asking people to copy binaries by hand.
> **Checklist:**
> - C499 — A put-back moves the window's data/ to data.rolled-back/<rfc3339>/ and removes none of it, and prints that folder with the count of leaves per store (DIRECTORY-082 R1).
> - C500 — After a put-back that follows writes in the window, the previous build reads every pre-window leaf, and every window leaf is found and counted in data.rolled-back (DIRECTORY-082 R1).
> - C501 — A confirmed upgrade's data/ carries every leaf in segment stores, and data.previous keeps the pre-upgrade copy unchanged (DIRECTORY-082 R2).
> - C502 — A put-back or a keep killed mid-copy, at data.restoring or data.previous.partial, leaves no partial copy served, and a second run completes it (DIRECTORY-082 R2).
> - C503 — The upgrade proof reads segment stores: every leaf read digests the whole store directory, and old leaves are proved against the kept v1 copy (DIRECTORY-082 R3).
> **Stories:**
> - S159 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who updated Lys, I want to go back to the build I had before with one command when the new one misbehaves after it started fine, so that a bad update never leaves me stuck.

## Purpose

BOX 17's first real old-install upgrade proof, run by Waffles on 6 October in the BOX 19 install window, stopped on its first baseline (1b568cd9 to 4b718b02). The upgrade itself ran; the proof's window check raised "operator proof requires a nonempty identity log" (upgrade_window.py line 124). The cause is the harness: since LYSLOGSTORE-008 a writable open migrates a v1 store to segments (file.rs open, migrate.rs migrate_v1), so a store's leaves sit under leaves/segments/ (segment.rs segments_dir), and the harness still keeps only plain files directly in leaves/ (upgrade_window.py lines 119 to 121, upgrade_live.py app_leaves and preserve_leaves, upgrade_restart.py lines 52 and 69 to 75). Reading the put-back on the way showed it sound: an upgrade copies data/ whole to data.previous before anything is placed (data_kept.rs keep, called from swap.rs) and a put-back puts that copy back before the previous build starts (data_kept.rs restore). One loss remains: restore removes data/, so everything written in the window, and its audit, is deleted on a put-back.

## Task

Keep the window's data on a put-back instead of removing it, and say where it went. Prove the put-back, the confirm and a kill mid-copy. Fix every v1-only leaf read in the upgrade proof together, so one window reruns all four baselines. Out of scope: any bound on data.rolled-back, which is Tom's word; the live install's data.previous, which nobody touches; and the migration itself, which stays as LYSLOGSTORE-008 built it.

## Requirements

### R1: A put-back moves the window's data aside and counts it

Behavioural. WHEN a put-back restores the kept data, THE SYSTEM SHALL move data/ to data.rolled-back/<rfc3339>/, the instant the put-back began in RFC 3339 UTC, before the kept copy is renamed into place, and SHALL remove nothing under data/. The move is a rename within the install root; a name already taken is refused by name and nothing is moved. THE SYSTEM SHALL then print the folder and, for each store under it, the store's name and its count of leaves, read from the store's own head without reading its history. A store it cannot count is named with the reason, never skipped. data.rolled-back is never pruned by the installer; whether it is ever bounded is left to Tom. The kept copy in data.previous stays, as restore keeps it today.

**Acceptance:**
- A put-back leaves data/ under data.rolled-back/<rfc3339>/ byte for byte as it stood.
- A put-back removes no file under data/.
- The put-back's output names the folder and one leaf count per store.
- After writes in the window and a put-back, the previous build reads every pre-window leaf.
- Every window leaf is found in data.rolled-back, and the counts printed match it.
- A taken data.rolled-back name is refused by name and moves nothing.

**Files:**
- modify: crates/lys/src/identity/upgrade/data_kept.rs
- modify: crates/lys/src/identity/upgrade/data_kept_tests.rs
- modify: crates/lys/src/identity/upgrade/swap.rs

**Checklist:**
- C499 — A put-back moves the window's data/ to data.rolled-back/<rfc3339>/ and removes none of it, and prints that folder with the count of leaves per store (DIRECTORY-082 R1).
- C500 — After a put-back that follows writes in the window, the previous build reads every pre-window leaf, and every window leaf is found and counted in data.rolled-back (DIRECTORY-082 R1).

**Stories:**
- S159 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who updated Lys, I want to go back to the build I had before with one command when the new one misbehaves after it started fine, so that a bad update never leaves me stuck.

### R2: The confirm and a kill mid-copy are proved

Behavioural. WHEN an upgrade is confirmed, THE SYSTEM SHALL leave data/ carrying every leaf it held, in segment stores, and data.previous holding the pre-upgrade copy unchanged. WHEN a keep or a put-back is killed while copying, at data.previous.partial or data.restoring, THE SYSTEM SHALL serve no partial copy, and the next run SHALL clear the partial name and complete the copy from the start. The proof kills at each copy boundary, not on a timer.

**Acceptance:**
- A confirmed upgrade's data/ reads every leaf it held before the upgrade.
- A confirmed upgrade leaves data.previous unchanged.
- A keep killed mid-copy leaves data/ as it was, and a second run completes data.previous.
- A put-back killed mid-copy serves no partial data/, and a second run completes it.

**Files:**
- modify: crates/lys/src/identity/upgrade/data_kept.rs
- modify: crates/lys/src/identity/upgrade/data_kept_tests.rs

**Checklist:**
- C501 — A confirmed upgrade's data/ carries every leaf in segment stores, and data.previous keeps the pre-upgrade copy unchanged (DIRECTORY-082 R2).
- C502 — A put-back or a keep killed mid-copy, at data.restoring or data.previous.partial, leaves no partial copy served, and a second run completes it (DIRECTORY-082 R2).

**Stories:**
- S159 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who updated Lys, I want to go back to the build I had before with one command when the new one misbehaves after it started fine, so that a bad update never leaves me stuck.

### R3: The upgrade proof reads segment stores

Behavioural. WHEN the upgrade proof reads a store's leaves after the candidate has opened it, THE SYSTEM SHALL digest every file under the store's directory, segments and offsets included, so any write is seen, and SHALL still refuse a store holding no leaf by name. WHEN it proves that old leaves survived the upgrade, it SHALL read them from the kept v1 copy (the store's "<dir>.v1" sibling, or data.previous on a put-back) and SHALL prove the segment store reads back the same leaves through the candidate's own answers. Every stale read is fixed in one change: upgrade_window.py leaves() (lines 119 to 121), upgrade_live.py app_leaves and preserve_leaves (lines 59 to 76) and their callers, and upgrade_restart.py budget_family and settle (lines 52 and 69 to 75). The proof's Python units run on Dean under Python 3.9 before the window is asked for.

**Acceptance:**
- refuse_operator reads a migrated directory log as nonempty and sees an admitted write.
- An empty store is still refused by name.
- Old app leaves are proved against the kept v1 copy after the upgrade.
- The restart checks read segment stores and still find a rewritten old leaf.
- Each fixed read has a unit over a segment store and over a v1 store.

**Files:**
- modify: scripts/identity-gates/upgrade_window.py
- modify: scripts/identity-gates/upgrade_live.py
- modify: scripts/identity-gates/upgrade_restart.py
- modify: scripts/identity-gates/test_upgrade_window.py
- modify: scripts/identity-gates/test_upgrade_restart.py
- modify: scripts/identity-gates/test_upgrade_fixture.py

**Checklist:**
- C503 — The upgrade proof reads segment stores: every leaf read digests the whole store directory, and old leaves are proved against the kept v1 copy (DIRECTORY-082 R3).

**Stories:**
- S159 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who updated Lys, I want to go back to the build I had before with one command when the new one misbehaves after it started fine, so that a bad update never leaves me stuck.

## Boundaries

- SHALL NOT remove, prune or rewrite anything under data.rolled-back or data.previous.
- SHALL NOT touch the live install's data.previous.
- SHALL NOT change the migration, the store layout or what any store answers.
- SHALL NOT read any store's whole history to count it.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.

## Verification

- cargo nextest run for lys whole, cargo test --doc, fmt, clippy pedantic in both configurations, ast-grep and the file-length check exit 0 at the head, on Dean.
- The identity-gates Python units pass on Dean under /usr/bin/python3 3.9.
- The BOX 17 upgrade proof passes all four baselines in one window, its lines posted.
