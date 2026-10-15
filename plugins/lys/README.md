# Lys for Codex

Use a plugin-capable Codex CLI from the repository root:

```sh
codex plugin marketplace add .
codex plugin add lys@lys-local
```

Open a new session to load the plugin and authenticate its Lys server when the
host offers connection setup. The marketplace requests authentication on install.
Use `$setup`, then `$whoami`, `$capabilities`, `$my-agents`, `$read-route`,
`$give-access` or `$drafts` from this plugin. If another package has the same short
skill name, select the Lys package in the skill picker. The seven command
workflows are exported as native skills; Codex does not load Claude command files.

`agents/lys-access-reader.toml` and `agents/lys-draft-preparer.toml` are optional
native companion roles. The plugin manifest does not automatically register
roles in this host. If wanted, copy them to your Codex `agents/` directory after
checking that the destinations do not already contain different files. Start a
new session and invoke the full role name. They use a read-only filesystem sandbox;
remote authority is still enforced by Lys, and their instructions permit reads
and preparation only. The plugin's skills and MCP connection need no role copy.

The plugin connects directly to `https://lys.ablative.com.au/api/mcp` using
Streamable HTTP and the host's OAuth support. It needs no Lys binary, localhost
service, SSH tunnel, private key, prefilled token or dependency install.
`config/lys-mcp.json` is active through the manifest's `mcpServers` field.

The public metadata advertises that endpoint as its protected resource and Lys
as its authorization server. The host registers an OAuth client, uses PKCE S256,
and opens Lys for the person to sign in and approve the connection. Consent makes
an app agent answering to that person. It does not copy the person's authority.
Give that agent the specific resource access it needs in Lys. Never paste tokens
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
| drafts | List, prepare or decide the correct draft family under its own authority. |

Personal `/me`, `/dashboard` and directory-draft decisions require the person
connection their routes admit. Connected-app OAuth is an agent connection. It
cannot be presented as the approving person's session. Directory-draft creation
requires an agent signature; this HTTP OAuth plugin does not supply a signing key.
The installed `lys mcp` stdio bridge is a separate signed-agent connection and
currently accepts numeric loopback HTTP only. It cannot be pointed at the remote
HTTPS address. Product drafts use `/product-drafts` through `read`/`change`;
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

This uses the compatibility manifest shape that OpenAI documents at
https://developers.openai.com/api/docs/guides/agents-api/tools/plugins and
https://developers.openai.com/plugins/build/plugins : `skills` and `mcpServers`
resolve inside this plugin root. No path depends on the author's home directory.

The optional validator requires Python 3.11 or later; the plugin runtime does not use Python.

Validate both distributions from the repository root:

```sh
python3 -B plugin/scripts/validate.py
```

With saved public schema and tools/list answers, also verify live coverage:

```sh
python3 -B plugin/scripts/validate.py --openapi schema.json --tools-list tools.json
```

The optional inputs are the public OpenAPI object and the `result` object of a
live tools/list response. They contain schemas, not credentials.
Uninstall with `codex plugin remove lys@lys-local`. Remove the marketplace with
`codex plugin marketplace remove lys-local` only if no other plugin uses it.
Remove a manually copied companion role only if it still matches this package.
Leave shared server configuration, seats and identities alone.
