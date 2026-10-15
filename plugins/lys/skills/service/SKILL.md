---
name: service
description: "Inspect Lys service contracts, health, configuration, receipts, setup and change signals."
---

# Lys service

Read the sibling lys skill before calling a tool.
This skill covers 32 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Public health, authority, public key and OpenAPI describe the installed service;
they do not prove a caller has workspace access. Public schema retrieval needs no
seat or personal data. Configuration writes carry a version and may change the
organization zone. Read the present version and scope before changing it.

Setup, sign-in providers, security events, estate import/apply and account recovery
are administration. Never run them as a plugin setup check. Login, callback and
MCP OAuth are host/browser protocols; do not collect passwords or tokens in chat.
Canvas arrangement is the caller's own saved view, not an authorization model.
Receipts are separate from the resulting product state and should name the same
operation and target as the requested act.

`GET /changes` returns `generation`; with the current value in `after` it holds
for the next signal. Subscribe before relying on a subsequent read, keep the
returned generation, and re-read admitted records after a wake. Signals carry no
records or identities. Do not repeatedly call health or lists for readiness.
`GET /mcp` advertises no stream (405); use POST Streamable HTTP, not legacy SSE.
The public MCP endpoint is `/api/mcp` on this install. Inside its four tools use
unprefixed API paths. See the lys skill for result and credential boundaries.
