---
type: brief
id: DIRECTORY-072
cluster: directory
title: A seat asks for a declared MCP server, a lead approves it within their remit, and the approval makes a new reviewed profile version
---

# DIRECTORY-072: A seat asks for a declared MCP server, a lead approves it within their remit, and the approval makes a new reviewed profile version

> **Cluster:** directory
> **Design anchor:**
> - ADR-087 — The executable, its arguments and the working directory are fields of the reviewed profile version, never of a start request — The executable, the arguments and the working directory are read from the reviewed profile version's record, as the mock-up draws them, and a start request names only the agent, the profile version and the machine. Rejected: letting a start request set either, and taking either from the machine or a default.
> - ADR-089 — A role is a set of grant templates copied at grant time, not a live group — A role is a template copied at grant time, not a live group. A holding names the role and the version it holds, the grants it carries are made from that version's templates when the holding is granted, and an edit to the role changes none of them. Who holds a role remains answerable as a query over holdings. Only the versioned grant templates make a version; a title edit is recorded and makes no version, and conformance 4.1's job and profile stay proposed and outside. Rejected: a live group, whose membership carries the role's current grants, because a live group would change every holder's permissions at the moment of the edit, which ADR-006 forbids.
> - ADR-133 — The launch is everything Lys records for the seat, for Claude Code and our Codex build — A provisioning profile declares its harness build; the launch renders every field for that build or the profile is refused by name when it is recorded. Command MCP servers carry secrets only as handles. Claude Code permissions come from the profile and the Tool policy; Codex permissions are DIRECTORY-065's render of the same policy.
> **Checklist:**
> - C464 — A seat's request for a declared MCP server is recorded against its latest reviewed version. (DIRECTORY-072 R1).
> - C465 — An approval within the approver's remit makes a new reviewed version; beyond it is refused by name. (DIRECTORY-072 R2).
> **Stories:**
> - S186 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want a seat's request for a tool approved only by someone whose remit covers it, so that permissions flow down my tree and never widen on the way.

## Purpose

A profile version already declares MCP servers (provisioning_store.rs McpServer, line 31, and mcp_servers, line 115), and grant delegation already refuses anything beyond what the delegator may pass on (lys-identity grants/admission.rs line 237). Nothing joins them: a seat cannot ask for a server, a lead cannot approve one, and adding a server today means a person editing a profile by hand. Tom's example is Pikelet asking for the Dot server and Archie approving it.

## Task

Let an agent ask for an MCP server declared in the estate by name. Let a person or lead agent approve the request only when they hold that server within their own remit for that agent; the approval writes a new profile version, the latest reviewed one plus that server, recorded as reviewed by the approver, keeping the previous version. Out: restarting the seat onto it (DIRECTORY-073) and the screen.

## Requirements

### R1: A seat asks for a declared MCP server by name

Behavioural. WHEN an agent asks for an MCP server by name, THE SYSTEM SHALL record the request against that agent's latest reviewed profile version, SHALL refuse a server the estate does not declare as mcp_server_unknown naming it, and SHALL refuse a server the version already carries as mcp_server_held.

**Acceptance:**
- Pikelet's request for the Dot server is recorded and read back pending.
- A request for an undeclared server is refused by name.
- A request for a server the version carries is refused by name.

**Files:**
- create: crates/lys-identity-server/src/mcp_requests_api.rs
- create: crates/lys-identity-server/tests/mcp_requests.rs
- modify: crates/lys-identity-server/src/routes_table.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C464 — A seat's request for a declared MCP server is recorded against its latest reviewed version. (DIRECTORY-072 R1).

**Stories:**
- S186 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want a seat's request for a tool approved only by someone whose remit covers it, so that permissions flow down my tree and never widen on the way.

### R2: An approval within remit makes a new reviewed profile version

Behavioural. WHEN a person or agent approves a pending request, THE SYSTEM SHALL check through the existing grant admission that the approver holds that server passable for that agent, SHALL refuse otherwise as mcp_beyond_remit naming the approver, the agent and the server, and on success SHALL write a new profile version equal to the latest reviewed version plus that server, recorded as reviewed by the approver, with the previous version kept unchanged.

**Acceptance:**
- Archie's approval of the Dot server for Pikelet writes a new version naming it, reviewed by Archie.
- Waffles approving a server outside her remit is refused as mcp_beyond_remit.
- The previous version reads back unchanged.
- The launch rendered from the new version names the server in its MCP configuration.

**Files:**
- modify: crates/lys-identity-server/src/provisioning_store.rs
- modify: crates/lys-identity-server/src/mcp_requests_api.rs
- modify: crates/lys-identity-server/tests/mcp_requests.rs

**Checklist:**
- C465 — An approval within the approver's remit makes a new reviewed version; beyond it is refused by name. (DIRECTORY-072 R2).

**Stories:**
- S186 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As the person running a team of agents, I want a seat's request for a tool approved only by someone whose remit covers it, so that permissions flow down my tree and never widen on the way.

## Boundaries

- SHALL NOT poll, sleep, add a timer, timeout, deadline, watchdog or background loop; every wait ends on the event it waits for, and an idle page and an idle service use no CPU.
- SHALL NOT add unsafe, an ignored test, an allow attribute, an underscore rename or a discarded result.
- SHALL NOT change a stored shape without a migration tested from an old install, and SHALL NOT edit an installed configuration by hand.
- SHALL NOT make Lys depend on Cambium or name a harness detail in code; harness details are data.
- SHALL NOT widen any grant, and SHALL NOT let an approver approve for an agent outside their own reach.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Built directly by the assigned seat on its own branch in its registered worktree: red first, then green, with fmt, Clippy pedantic, tests and ast-grep.
- The full src_gate runs on Dean at the exact head that lands; the lead reviews that head and gives the landing word.
- Shown to Tom on the dev server at 127.0.0.1:5190 against the live Lys as one step of the night's demonstration.
