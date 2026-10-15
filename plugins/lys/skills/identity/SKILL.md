---
name: identity
description: "Read and manage Lys identities, own agents, accounts and sessions."
---

# Lys identity

Read the sibling lys skill before calling a tool.
This skill covers 30 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Use `/me` for a person connection's identity and `/dashboard` for that person's
own agents. A signed agent has no person session: use discovery and, if admitted,
`/agent/grants`; do not present its responsible person as the caller. An OAuth
connected app also acts as an agent. `/apps/me/agents` requires the product's
person access pass and is not interchangeable with connected-app OAuth.

Resolve ids from the caller-visible directory before changing profiles, reporting
edges, logins or lifecycle. Account enablement, passwords, admission and ending a
person's sessions need their declared authority. Read session metadata only within
the user's requested scope. Never retrieve account data for a plugin setup check.
`/dashboard` uses `after`, `limit`, `q` and `team`; follow its returned page cursor
with the same filters. Directory routes are administration, not a substitute for
own-agent visibility. Use the agents skill for provisioning and launch.
