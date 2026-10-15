---
name: identity
description: "Read and manage Lys identities, own agents, accounts and sessions."
---

# Lys identity

Read the sibling lys skill before calling a tool.
This skill covers 30 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Use `/me` for a person connection's identity and `/dashboard` for that person's
own agents when the person chose Person reads in browser consent. Discovery
names this as `connection.mode: person_reads`. Each read returns a signed receipt
naming the connection, person and route. This mode cannot change, approve, give
or sign as the person; those acts need the browser. In agent mode, an OAuth app
or signed agent keeps its own identity: use discovery and, if admitted,
`/agent/grants`; do not present its responsible person as the caller.
`/apps/me/agents` requires the product's person access pass and is not
interchangeable with connected-app OAuth.

Resolve ids from the caller-visible directory before changing profiles, reporting
edges, logins or lifecycle. Account enablement, passwords, admission and ending a
person's sessions need their declared authority. Read session metadata only within
the user's requested scope. Never retrieve account data for a plugin setup check.
`/dashboard` uses `after`, `limit`, `q` and `team`; follow its returned page cursor
with the same filters. Directory routes are administration, not a substitute for
own-agent visibility. Use the agents skill for provisioning and launch.
