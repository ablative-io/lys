---
name: apps
description: "Manage Lys apps, schema registration, approvals, placements and OAuth connections."
---

# Lys apps

Read the sibling lys skill before calling a tool.
This skill covers 39 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

The `/apps` family describes app registration, approval, credentials, schema
versions, placements and benches. Read `/apps/me` or the named visible app before
changing it. Approval, credential issuance and schema adoption are different acts.
Check a schema before submitting its change, then obtain its required approval.
An app bench asks the product for evidence; a response is not approval by itself.

`/oauth/*` is the product issuer; `/oauth/mcp/*` is the connected-app OAuth door.
Let the host perform registration, PKCE S256, browser consent and token refresh.
The protected resource is `https://lys.ablative.com.au/api/mcp`; metadata is at
`https://lys.ablative.com.au/.well-known/oauth-protected-resource/api/mcp` and
`https://lys.ablative.com.au/.well-known/oauth-authorization-server`.
Do not manually paste tokens or impersonate the special `lys-cli` native client.

Connected-app consent creates/reuses an agent answering to the approving person;
it does not copy that person's grants. Give only the needed access to that agent.
Connector gifts and registry passes have their own schemas and authority. Never
issue credentials, rotate issuer keys, exchange connect codes or revoke an app
merely to test this plugin. Keep credential answers out of chat and logs.
