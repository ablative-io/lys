---
name: agents
description: "Manage Lys agent profiles, harnesses, skills, budgets, certificates and launches."
---

# Lys agents

Read the sibling lys skill before calling a tool.
This skill covers 48 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Agent routes include reporting, certificates, provisioning, MCP requests, policies,
usage, model calls, refusals, goals, words, variables and memory. Read only the
agent the user names and the metadata needed for the job. Model-call bodies,
memory and variables may contain personal or secret data; they are not setup probes.

Read the profile version and its review before changing provisioning or requesting
an MCP server. Review is a distinct person's decision when the route requires it.
Read budget limits before replacing a collection; PUT replaces the holder's limits,
not one incidental field. Use revision members for policy and variable updates.
Do not substitute a newer version on a stale-write refusal.

A start-command preview, a launch record, a running session and completed work
are separate evidence. Follow launch-record state and runtime receipts on signals.
Stop and restart routes affect access and sessions; use only when explicitly in
the user's job. An agent stop suspends access and withdraws certificates; ordinary
session end uses the runtime route. Audience passes are secret credentials; never
mint them as a connection test.
