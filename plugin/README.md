# Lys for Claude Code

From a checkout of this repository, run:

```sh
claude plugin marketplace add ./
claude plugin install lys@lys-local
```

Open a new Claude Code session, use `/mcp` to find the Lys plugin server and
choose Authenticate. Complete consent at Lys. Then run `/lys:setup`.
The other commands are `/lys:whoami`, `/lys:capabilities`, `/lys:my-agents`,
`/lys:read-route`, `/lys:give-access` and `/lys:drafts`.
For development, `claude --plugin-dir ./plugin` loads this package without a
permanent plugin install. Do not also add a second manual Lys server for it.

Two companion agents, `access-reader` and `draft-preparer`, read or prepare only.
They do not grant access, decide drafts or impersonate a person. Their instructions
restrict behavior; they do not establish a server permission boundary.

The plugin connects directly to `https://lys.ablative.com.au/api/mcp` using
Streamable HTTP and the host's OAuth support. It needs no Lys binary, localhost
service, SSH tunnel, private key, prefilled token or dependency install.
`config/lys-mcp.json` is active through the manifest's `mcpServers` field.

The public metadata advertises that endpoint as its protected resource and Lys
as its authorization server. The host registers an OAuth client, uses PKCE S256,
and opens Lys for the person to sign in and consent. Agent mode remains the
default and uses the connected agent's own grants. Person reads is an explicit
choice for the person's own identity, agents, directory and product drafts; every reading
has a signed receipt naming connection, person and route. It never grants the
person's approval, giving or signing authority. Those acts need the browser.
Old tokens remain in agent mode. Never paste tokens
or a seat file into chat. Installed plugins do not modify or restart Lys or Cambium.

The server's four tools are `what-can-I-do`, `read`, `change`, `drafts`.
Commands are workflows over those tools, not extra server tool names. Host tool
prefixes may differ. Tool paths use `/me`, for example, while the remote endpoint
is `/api/mcp`. They never use `/api/me` or an arbitrary remote URL.

Thirteen operating skills cover all 305 operations in the live 0.3.1 schema,
including all 55 first path segments. Each domain has a route reference with
methods, paths, summaries and request-schema names. The current public schema at
`https://lys.ablative.com.au/api/openapi.json` is authoritative for bodies, query
parameters and refusals. `config/routes.json` records the coverage mapping.
Read the `lys` skill first; discovery does not grant the routes it lists.

| Workflow | Purpose |
| --- | --- |
| setup | Verify attachment, four tool names and discovery without personal reads. |
| whoami | Identify this caller; distinguish agent from responsible person. |
| capabilities | Read this connection's routes, resources and actions. |
| my-agents | Read a person's own dashboard if this connection is admitted. |
| read-route | Ask a declared reading route using its current schema. |
| give-access | Make an authorized scoped gift, read it back and verify recipient use. |
| drafts | Read or prepare drafts; hand personal decisions back to the browser. |

In Person reads, use `/me`, `/dashboard`, `/drafts` and `/product-drafts` for
the consenting person's own views. Directory drafts support `state=waiting`,
`decided` and `all`, including own raised drafts. Product drafts use
`read {"path":"/product-drafts?limit=50"}` and include only drafts held by the
person or recorded with them as responsible, filtered before paging. Follow
`after` cursors with the same limit; use each row's state to find waiting drafts.
Another person's drafts remain unavailable, including for administrators. A
decision is refused as `PersonPresenceRequired` with
`deciding a draft needs you in Lys:` and that draft's browser URL.
Agent mode is unchanged. Directory-draft creation requires an agent signature;
this HTTP OAuth plugin does not supply a signing key.
The installed `lys mcp` stdio bridge is a separate signed-agent connection and
currently accepts numeric loopback HTTP only. It cannot be pointed at the remote
HTTPS address. In agent mode, product drafts use `/product-drafts` through `read`/`change`;
the `drafts` tool itself accepts only `/drafts` routes.

Live verification found all four tools and successful discovery (92 routes, zero
resource grants) for the provided agent seat. Its attempted reads of `/health`
and `/me` returned `TokenScopeUndeclared`. Invalid `change` and `drafts` calls
returned JSON-RPC `-32602` before dispatch. These are verified refusals, not
successful workspace operations. No credential was printed or packaged.
OAuth browser consent and admitted end-to-end person/gift/draft operations still
need an appropriately authorized connection; this package does not bypass that.

For waiting, use an admitted `/changes` generation subscription or host notification.
A matching `after` holds for a signal. Re-read on the signal; never use an interval
or an elapsed-time readiness claim. An authenticated `GET /api/mcp` is 405; it is not a legacy SSE stream.

If setup reports `NotSignedIn`, authenticate this plugin server through the host.
If it reports `TokenScopeUndeclared`, `NotHeld`, `PermissionAbsent` or zero
resources, have the responsible authority give this connection its requested
access. Do not mint a token, widen a grant or retry with another credential.
Preserve uncertain operation ids and read their receipt before another mutation.

The optional validator requires Python 3.11 or later; the plugin runtime does not use Python.

Validate from the repository root:

```sh
claude plugin validate --strict ./.claude-plugin/marketplace.json
claude plugin validate --strict ./plugin
python3 -B plugin/scripts/validate.py
```

Uninstall with `claude plugin uninstall lys@lys-local`. Remove this marketplace
with `claude plugin marketplace remove lys-local` only if no other plugin needs
it. Do not remove shared server configuration, seats or identities.
