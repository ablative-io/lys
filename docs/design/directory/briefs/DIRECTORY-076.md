---
type: brief
id: DIRECTORY-076
cluster: directory
title: Lys offers the programs it can start and each program's models and modes as named choices, from its own descriptions and what each computer's runner finds installed
---

# DIRECTORY-076: Lys offers the programs it can start and each program's models and modes as named choices, from its own descriptions and what each computer's runner finds installed

> **Cluster:** directory
> **Design anchor:**
> - ADR-133 — The launch is everything Lys records for the seat, for Claude Code and our Codex build — A provisioning profile declares its harness build; the launch renders every field for that build or the profile is refused by name when it is recorded. Command MCP servers carry secrets only as handles. Claude Code permissions come from the profile and the Tool policy; Codex permissions are DIRECTORY-065's render of the same policy.
> - ADR-087 — The executable, its arguments and the working directory are fields of the reviewed profile version, never of a start request — The executable, the arguments and the working directory are read from the reviewed profile version's record, as the mock-up draws them, and a start request names only the agent, the profile version and the machine. Rejected: letting a start request set either, and taking either from the machine or a default.
> **Checklist:**
> - C474 — Lys answers the programs it can start, with their models and modes as named choices. (DIRECTORY-076 R1).
> - C475 — Each computer's runner reports where each described program is installed. (DIRECTORY-076 R2).
> **Stories:**
> - S190 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want to pick an agent's program, model and mode from named choices, so that I can set an agent up without typing paths, ids or JSON.

## Purpose

The profile editor asks a person to type a harness name, a program path, a package and a JSON build description, then to type model names and a permission mode (issue #120). Tom, 30 September 19:05: "None of these words align to the concepts you're trying to use them with"; the harness must be a choice, never typed, and nothing entered as JSON. A fresh install has no earlier profile to copy from, so the choices must come from what Lys itself knows: its own description of Claude Code and Codex, and what each computer's runner finds installed.

## Task

Keep one description per supported program as data in the repository: its plain name, one line saying what it is, its models (each an id and a plain label, the first its default), its permission modes (each an id and one plain line saying what it lets the agent do), and its MCP transports, channel policies and rendering contract as the declared-harness description already holds them. Answer them on one read with, for each computer, the program's absolute path and package as that computer's runner found it, and the builds already declared in reviewed profiles. The profile editor chooses from this read. Out: generating these descriptions from the published documentation, which is DIRECTORY-075 R1 and will replace the kept data.

## Requirements

### R1: Lys answers the programs it can start, with their models and modes as named choices

Behavioural. WHEN a signed-in person reads the programs, THE SYSTEM SHALL answer each program Lys describes whose rendering contract is registered in the home with its plain name and line, its models as id and label with the default first, its permission modes as id and plain meaning, and its declared-harness description, together with every build already declared in a reviewed profile under that program's name; a description file Lys cannot read SHALL fail the service's start naming the file.

**Acceptance:**
- A fresh install answers the named models and modes of every described program with a registered rendering contract before any profile exists; kept descriptions with unregistered contracts are absent.
- A build declared in a reviewed profile is answered under its program with its path and package.
- An unreadable description names its file at start.

**Files:**
- create: crates/lys-identity-server/src/harness_catalogue.rs
- create: crates/lys-identity-server/tests/harness_catalogue.rs
- create: docs/harness/catalogue/claude-code.json
- create: docs/harness/catalogue/codex.json
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/src/error_status.rs
- modify: crates/lys-identity-server/src/openapi_types.rs
- modify: crates/lys-home/src/harness/rendering.rs

**Checklist:**
- C474 — Lys answers the programs it can start, with their models and modes as named choices. (DIRECTORY-076 R1).

**Stories:**
- S190 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want to pick an agent's program, model and mode from named choices, so that I can set an agent up without typing paths, ids or JSON.

### R2: Each computer's runner reports where each described program is installed

Behavioural. WHEN a runner reports to Lys, THE SYSTEM SHALL include, for each program Lys describes, the absolute path the runner found for its command on its own search path and the version that program reports, or that it is not installed; the programs read SHALL answer each computer's install beside the program.

**Acceptance:**
- A computer with Claude Code installed is answered with its path and version under Claude Code.
- A computer without Codex is answered as not having it.
- A runner from before this change reports no installs and is answered as unknown, not as absent.

**Files:**
- create: crates/lys-runner/src/installed.rs
- create: crates/lys-runner/tests/installed.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-identity-server/src/runtime_api.rs
- modify: crates/lys-identity-server/src/harness_catalogue.rs

**Checklist:**
- C475 — Each computer's runner reports where each described program is installed. (DIRECTORY-076 R2).

**Stories:**
- S190 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want to pick an agent's program, model and mode from named choices, so that I can set an agent up without typing paths, ids or JSON.

## Boundaries

- SHALL NOT poll, sleep, add a timer, timeout, deadline, watchdog or background loop; every wait ends on the event it waits for, and an idle page and an idle service use no CPU.
- SHALL NOT add unsafe, an ignored test, an allow attribute, an underscore rename or a discarded result.
- SHALL NOT change a stored shape without a migration tested from an old install, and SHALL NOT edit an installed configuration by hand.
- SHALL NOT make Lys depend on Cambium or name a harness detail in code; harness details are data.
- SHALL NOT offer a choice a person must type as JSON, a path or an id: every choice is a name a person would recognise, held to Tom's test: would his dad know how to fill it in.
- SHALL NOT start, install or update a program on a computer; a runner only looks and reports.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Built directly by the assigned seat on its own branch in its registered worktree: red first, then green, with fmt, Clippy pedantic, tests and ast-grep.
- The full src_gate runs on Dean at the exact head that lands; the lead reviews that head and gives the landing word.
- Shown to Tom on the dev server at 127.0.0.1:5190 against the live Lys as one step of the night's demonstration.
