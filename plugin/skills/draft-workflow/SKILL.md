---
name: draft-workflow
description: "Record, inspect and decide Lys directory and product drafts without confusing approval with execution."
---

# Lys draft-workflow

Read the sibling lys skill before calling a tool.
This skill covers 11 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

There are two draft families. The `drafts` tool accepts only `/drafts` paths.
Use `read` and `change` for `/product-drafts` routes. Their request schemas and
approval evidence differ; do not carry an id or hash from one family to the other.

Directory creation (`drafts`, POST `/drafts`) requires an agent signature and a
body with `operation`, `target` (`kind`, `id`, `action`), `method`, `path`, `body`
and `note`. The prepared body is exact JSON text in a string, not a JSON object.
An ordinary OAuth connected app cannot supply an agent signature through this
plugin. Report that boundary; do not claim its OAuth token is a signing key.

Directory listing (`drafts`, GET `/drafts?state=waiting`) is personal. Other states
are `decided` and `all`; `q`, `team`, `after`, `limit` retain the same paging scope.
An OAuth connection consented for Person reads can read only its person's own
directory drafts, including their raised drafts in `state=all`. Each reading has
a signed connection/person/route receipt. Agent mode is unchanged and does not
borrow the person's views. In Person reads, use
`read {"path":"/product-drafts?limit=50"}` for the person's own product drafts,
including those raised by them or their agents and those still waiting. Only
drafts held by that person or recorded with them as responsible are included,
filtered before paging. Follow `after` with the same limit. Use returned states
to select waiting rows; person product reads take no `app` or `state` query.
Administrator authority never adds another person's records.

Every decision as the person stays in Lys's browser. Person reads never decides,
approves, refuses, corrects, gives or signs for them. A decision call returns
`PersonPresenceRequired`: `deciding a draft needs you in Lys: <url of that draft>`.
Hand back that URL, the draft id, creation hash and exact prepared request.
The following decision bodies describe the browser API, not permission for an
AI to submit a person's approval.

Approve POST `/drafts/{id}/approve` with `operation`, `creation_hash`, `application`.
Refuse POST `/drafts/{id}/refuse` with `operation`, `creation_hash`, `reason`.
Correct POST `/drafts/{id}/correct` with those refusal fields plus `replacement`,
a complete DraftBody. These decisions are immutable and do not execute the change.
Read the list again and distinguish its decision receipt from application proof.

Product drafts expose create/list, approve/refuse, executed and refused-on-execution
routes. Read each live schema. An execution report needs actual product evidence;
never mark one executed merely because a draft was approved. Do not approve your
own work or manufacture the responsible person's authority.
