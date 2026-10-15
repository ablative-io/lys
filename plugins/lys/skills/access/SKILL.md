---
name: access
description: "Give, inspect and explain Lys access through grants, roles, requests and reviews."
---

# Lys access

Read the sibling lys skill before calling a tool.
This skill covers 42 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Read `/grants/model`, the target resource and the caller's visible source grants
before preparing access. Read `/grants/why`, `/grants/who`, `/grants/reach`, batch
checks, membership and `/grants/which` with POST only when the live route describes
a question. `/grants/check` records an exercise; use `change` for a recorded act.

For an ordinary gift, `POST /grants` needs a held source, recipient, responsible
person, resource, relation, pass-on terms, window, route and operation. Resolve
each from current reads and the model. Use an administrator source or grant
mode only if the live schema and the caller's authority support it. Source code
for a newer server does not establish support in the installed service. A root grant, a role
holding, an access request and a derived grant have different admission rules.

Use the `give-access` command workflow. Read the recorded grant back by id and
check the recipient's discovery in its own connection before claiming usable
access. Revocation affects descendants; establish that scope before acting.
Grant tokens are secrets: do not issue one as an access check or print its value.
Bindings and checkpoints carry log identity and revision; a stale or degraded
answer is unresolved. `/grants/changes` is an event subscription, not a poll loop.
Use its live request schema and checkpoint; never reset a checkpoint to hide loss.
