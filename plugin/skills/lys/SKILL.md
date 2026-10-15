---
name: lys
description: "Operate Lys through its four MCP tools, live schemas, grants and named refusals. Use before Lys route work."
---

# Lys

Use the MCP server bundled with this plugin at `https://lys.ablative.com.au/api/mcp`.
Resolve its four bare tool names from the host's tool catalog; host prefixes vary.
Do not infer a tool name from a command name. The server offers exactly these tools:

| Tool | Arguments | Meaning |
| --- | --- | --- |
| `what-can-I-do` | `{}` only | This connection's declared routes, live resources and actions. |
| `read` | `path`; optional `method` (`GET` by default, or `POST`), optional `body` | Ask an existing reading route. A POST is a read only when that route describes a question. |
| `change` | explicit `method` (`POST`, `PUT`, `PATCH`, `DELETE`), `path`, optional `body` | Ask an existing mutation route under its own admission. |
| `drafts` | `path` under `/drafts`; optional `method` (`GET` by default, or `POST`), optional `body` | Record or decide an immutable directory draft. |

Call `what-can-I-do` first. It takes no path, body, filters or other members.
Its result lists routes even when their own rules will refuse this caller.
Zero resources means zero resource grants; a successful discovery is not access.
Fill path parameters with ids from this connection's reads, percent-encoding each
segment. Tool paths are local paths such as `/me`, never URLs or `/api/me`.
The HTTP endpoint itself includes `/api`; paths inside tools do not.

Read the current public schema at `https://lys.ablative.com.au/api/openapi.json`
without credentials, or use `read {"path":"/openapi.json"}` if admitted.
Look up `paths[path][method]`, its parameters, requestBody and referenced schemas
under `components.schemas`. Follow `$ref`, `oneOf`, required members and enums.
Use only fields the live schema declares. The route index in each domain skill's
`references/routes.md` is a coverage map, not a replacement for the current schema.
`config/routes.json` at the plugin root maps every shipped route to its skill.

Every tool result may contain `isError` and text JSON `{status, body}`. Check both.
JSON-RPC `error`, an HTTP refusal, a degraded result, and a recorded success are
different outcomes. Report `refusal`, `reason` and `fields` without credentials.
Do not retry an uncertain mutation: read its operation/receipt first. Keep the
operation id for that intended act; a different act needs a different id.
Read the changed record and its receipt after a mutation. A recorded approval
or launch is not evidence that an application or session has run.

OAuth browser consent offers Agent mode (the existing default) or Person reads.
Agent mode connects an app agent under its own grants. Person reads permits only
the consenting person's `/me`, `/dashboard` and own directory `/drafts` views,
with signed receipts naming the connection, person and route. Discovery names
that mode as `connection.mode: person_reads`. The `/dashboard` read returns only
agent rows and paging fields. Old tokens remain in agent mode.
Person reads never permits changes, decisions, approvals, giving or signing as
the person; those need Lys in the browser. Product-draft reads in this mode are
refused by name until an own-product-draft helper is callable. A draft decision
refuses with `PersonPresenceRequired` and the words
`deciding a draft needs you in Lys: <url of that draft>`.
Do not reuse the person's session, an operator credential or a different seat to
work around a refusal. Run passes, signed seats and grant tokens are alternative
credentials for their own purposes; never combine them with OAuth bearer tokens.
Never request or display a credential value in chat.

When waiting, use an admitted change signal or a host notification and return a
pending handoff. No repeated list calls on an interval. `/changes` answers a
`generation`; asking with `after` equal to it holds until the next signal. A wake
means re-read through ordinary admitted routes, not that the requested work passed.
If the host cannot keep the subscription open, report that limit and hand back.
