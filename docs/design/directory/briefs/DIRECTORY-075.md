---
type: brief
id: DIRECTORY-075
cluster: directory
title: Every flag, argument, environment variable and setting of Claude Code and Codex is a typed schema generated from their published docs, and approved MCP servers are a registry
---

# DIRECTORY-075: Every flag, argument, environment variable and setting of Claude Code and Codex is a typed schema generated from their published docs, and approved MCP servers are a registry

> **Cluster:** directory
> **Design anchor:**
> - ADR-133 — The launch is everything Lys records for the seat, for Claude Code and our Codex build — A provisioning profile declares its harness build; the launch renders every field for that build or the profile is refused by name when it is recorded. Command MCP servers carry secrets only as handles. Claude Code permissions come from the profile and the Tool policy; Codex permissions are DIRECTORY-065's render of the same policy.
> - ADR-087 — The executable, its arguments and the working directory are fields of the reviewed profile version, never of a start request — The executable, the arguments and the working directory are read from the reviewed profile version's record, as the mock-up draws them, and a start request names only the agent, the profile version and the machine. Rejected: letting a start request set either, and taking either from the machine or a default.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> **Checklist:**
> - C471 — Each harness's flags, arguments, environment variables and settings are Rust generated from its kept docs. (DIRECTORY-075 R1).
> - C472 — A profile version is checked against its harness's schema and its settings read lists every setting. (DIRECTORY-075 R2).
> - C473 — Approved MCP servers are kept in a registry that requests choose from. (DIRECTORY-075 R3).
> **Stories:**
> - S189 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want every setting an agent can have listed and checked, and a list of approved MCP servers, so that I can see an agent's whole configuration and a mistyped setting is refused before it starts.

## Purpose

A profile version records a program, its arguments and its MCP servers as free text (provisioning_store.rs), so the page for an agent cannot show all of its settings, cannot say which values are valid, and a mistyped flag is found only when the seat fails to start. Claude Code and Codex publish their command-line flags, environment variables, settings and MCP configuration in their documentation, which is available as markdown. Tom's direction is to take the definitive lists from there and generate Rust from them, not to copy another tool's code.

## Task

Keep the published markdown for each harness in the repository, with its source address and the date it was taken, and a generator that turns its tables into Rust types for that harness's flags, arguments, environment variables and settings, each with its documented type and description. A profile version is checked against the generated schema, and the agent's settings read answers every setting with its value or its default. Keep a registry of approved MCP servers, each with its name, its command or address and the configuration it needs, which DIRECTORY-072 requests choose from. Out: harnesses other than Claude Code and Codex.

## Requirements

### R1: Each harness's settings are Rust generated from its published docs

Behavioural. WHEN the generator runs over a harness's kept documentation, THE SYSTEM SHALL write Rust naming every flag, argument, environment variable and setting in its tables with its documented type and description, SHALL fail naming the table row it cannot read, and SHALL produce byte-identical output on a second run.

**Acceptance:**
- The generated Claude Code and Codex schemas name every flag and environment variable in the kept docs.
- A row the generator cannot read fails the run naming it.
- Running the generator twice gives the same bytes, and the committed Rust equals what it gives.

**Files:**
- create: crates/lys-harness-settings/Cargo.toml
- create: crates/lys-harness-settings/src/lib.rs
- create: crates/lys-harness-settings/src/claude_code.rs
- create: crates/lys-harness-settings/src/codex.rs
- create: crates/lys-harness-settings/tests/generated.rs
- create: scripts/harness-settings/generate.py
- create: docs/harness/claude-code.md
- create: docs/harness/codex.md
- modify: Cargo.toml

**Checklist:**
- C471 — Each harness's flags, arguments, environment variables and settings are Rust generated from its kept docs. (DIRECTORY-075 R1).

**Stories:**
- S189 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want every setting an agent can have listed and checked, and a list of approved MCP servers, so that I can see an agent's whole configuration and a mistyped setting is refused before it starts.

### R2: A profile version is checked against its harness's schema and its settings read in full

Behavioural. WHEN a profile version is written or read, THE SYSTEM SHALL refuse a flag, variable or setting its harness's schema does not name as harness_setting_unknown naming it, and SHALL answer the agent's settings as every setting of its harness with its value or its documented default.

**Acceptance:**
- A version naming an unknown flag is refused by name.
- An agent's settings read lists every setting of its harness, set or default.

**Files:**
- create: crates/lys-identity-server/tests/harness_settings.rs
- modify: crates/lys-identity-server/src/provisioning_store.rs

**Checklist:**
- C472 — A profile version is checked against its harness's schema and its settings read lists every setting. (DIRECTORY-075 R2).

**Stories:**
- S189 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want every setting an agent can have listed and checked, and a list of approved MCP servers, so that I can see an agent's whole configuration and a mistyped setting is refused before it starts.

### R3: Approved MCP servers are a registry

Behavioural. WHEN a person with the right adds or retires an approved MCP server, THE SYSTEM SHALL keep its name, its command or address and the configuration it needs, SHALL answer the registry to any agent under the person, and SHALL refuse a request for a server not in it as mcp_server_unknown.

**Acceptance:**
- The Dot server is added to the registry and read back with its configuration.
- A retired server can no longer be requested.

**Files:**
- create: crates/lys-identity-server/src/mcp_registry.rs
- create: crates/lys-identity-server/tests/mcp_registry.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C473 — Approved MCP servers are kept in a registry that requests choose from. (DIRECTORY-075 R3).

**Stories:**
- S189 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want every setting an agent can have listed and checked, and a list of approved MCP servers, so that I can see an agent's whole configuration and a mistyped setting is refused before it starts.

## Boundaries

- SHALL NOT poll, sleep, add a timer, timeout, deadline, watchdog or background loop; every wait ends on the event it waits for, and an idle page and an idle service use no CPU.
- SHALL NOT add unsafe, an ignored test, an allow attribute, an underscore rename or a discarded result.
- SHALL NOT change a stored shape without a migration tested from an old install, and SHALL NOT edit an installed configuration by hand.
- SHALL NOT make Lys depend on Cambium or name a harness detail in code; harness details are data.
- SHALL NOT copy another tool's source code; the documentation's tables are the only input, kept with their source address.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Built directly by the assigned seat on its own branch in its registered worktree: red first, then green, with fmt, Clippy pedantic, tests and ast-grep.
- The full src_gate runs on Dean at the exact head that lands; the lead reviews that head and gives the landing word.
- Shown to Tom on the dev server at 127.0.0.1:5190 against the live Lys as one step of the night's demonstration.
