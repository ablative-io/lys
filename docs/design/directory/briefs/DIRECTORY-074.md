---
type: brief
id: DIRECTORY-074
cluster: directory
title: A seat's reviewed profile names its writable folders, the Launch carries them, and the runner confines the session to them
---

# DIRECTORY-074: A seat's reviewed profile names its writable folders, the Launch carries them, and the runner confines the session to them

> **Cluster:** directory
> **Design anchor:**
> - ADR-128 — The runner applies OS containment from the same Lys policy and reports native denials — Compile one Lys policy into Seatbelt on macOS and Landlock with a private network namespace on Linux. A policy-bound egress service enforces hostnames while OS rules prevent direct bypass. A runner applies containment before untrusted exec and records its policy digest and session incarnation. Native kernel events feed the same authenticated refusal stream as051, with distinct provenance from tool and proxy denials. Missing enforcement or required audit support refuses launch. gaps are visible and end affected sessions through the existing ownership mechanism. A writable outside-root fixture that succeeds without confinement is the filesystem control. Never infer sandbox enforcement from a failure to write /etc/x. Both native platforms require real tests and receipts.
> - ADR-133 — The launch is everything Lys records for the seat, for Claude Code and our Codex build — A provisioning profile declares its harness build; the launch renders every field for that build or the profile is refused by name when it is recorded. Command MCP servers carry secrets only as handles. Claude Code permissions come from the profile and the Tool policy; Codex permissions are DIRECTORY-065's render of the same policy.
> - ADR-087 — The executable, its arguments and the working directory are fields of the reviewed profile version, never of a start request — The executable, the arguments and the working directory are read from the reviewed profile version's record, as the mock-up draws them, and a start request names only the agent, the profile version and the machine. Rejected: letting a start request set either, and taking either from the machine or a default.
> **Checklist:**
> - C469 — The signed Launch carries a profile's writable folders. (DIRECTORY-074 R1).
> - C470 — A confined session writes only inside its folders and a write outside is refused by name. (DIRECTORY-074 R2).
> **Stories:**
> - S188 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want each seat confined to its own folders, so that no seat can write anywhere else on my machine.

## Purpose

The runner already builds OS containment from a Lys policy and reports native denials (containment_paths.rs, containment_policy.rs, containment_macos.rs, ADR-128), but the launch never asks for it, so every seat runs with the whole machine open. Tom's ruling is that YOLO mode ends: each seat works only in its own folders.

## Task

Let a profile version name the folders a seat may write. Carry them in the signed runner Launch and have the runner apply containment from them, so a write outside is refused and reported by name. A Launch with no folders behaves as today, so no running seat changes until its profile is moved. Out: moving existing seats onto confinement, which is done one team at a time on Tom's word. Confinement is set per seat in its profile, never for everyone: leads keep the reach their profile gives them, and changing a seat's folders is a new reviewed profile version and a restart.

## Requirements

### R1: The Launch carries the profile's writable folders

Behavioural. WHEN a profile version names writable folders, THE SYSTEM SHALL carry them in the signed runner Launch rendered from it; a Launch with none SHALL behave exactly as today, and a runner from before this change SHALL refuse a Launch that carries them rather than ignore them.

**Acceptance:**
- The Launch rendered from a version with folders names them.
- A version with none renders a Launch that behaves as today.
- A runner built before this change refuses a Launch naming folders.

**Files:**
- create: crates/lys-runner/tests/launch_folders.rs
- create: crates/lys-identity-server/tests/profile_folders.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-identity-server/src/launch_template.rs
- modify: crates/lys-identity-server/src/provisioning_store.rs
- modify: crates/lys-identity-server/src/provisioning_api.rs
- modify: crates/lys-identity-server/tests/provisioning.rs

**Checklist:**
- C469 — The signed Launch carries a profile's writable folders. (DIRECTORY-074 R1).

**Stories:**
- S188 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want each seat confined to its own folders, so that no seat can write anywhere else on my machine.

### R2: The runner confines the session to those folders

Behavioural. WHEN a Launch names writable folders, THE SYSTEM SHALL start the session under the runner's containment with those folders writable and nothing else in the home writable, and SHALL report a write outside them as a native denial naming the path.

**Acceptance:**
- A confined seat's write inside its folder succeeds.
- Its write outside is refused and reported naming the path.
- An unconfined seat is unchanged.

**Files:**
- modify: crates/lys-runner/src/session/lifecycle.rs
- modify: crates/lys-runner/src/containment_paths.rs
- modify: crates/lys-runner/src/containment_policy.rs
- modify: crates/lys-runner/tests/launch_folders.rs

**Checklist:**
- C470 — A confined session writes only inside its folders and a write outside is refused by name. (DIRECTORY-074 R2).

**Stories:**
- S188 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want each seat confined to its own folders, so that no seat can write anywhere else on my machine.

## Boundaries

- SHALL NOT poll, sleep, add a timer, timeout, deadline, watchdog or background loop; every wait ends on the event it waits for, and an idle page and an idle service use no CPU.
- SHALL NOT add unsafe, an ignored test, an allow attribute, an underscore rename or a discarded result.
- SHALL NOT change a stored shape without a migration tested from an old install, and SHALL NOT edit an installed configuration by hand.
- SHALL NOT make Lys depend on Cambium or name a harness detail in code; harness details are data.
- SHALL NOT move any running seat onto confinement; that is done on Tom's word, one team at a time.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Built directly by the assigned seat on its own branch in its registered worktree: red first, then green, with fmt, Clippy pedantic, tests and ast-grep.
- The full src_gate runs on Dean at the exact head that lands; the lead reviews that head and gives the landing word.
- Shown to Tom on the dev server at 127.0.0.1:5190 against the live Lys as one step of the night's demonstration.
