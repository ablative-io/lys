---
name: teams
description: "Manage Lys team membership, nesting and the caller-visible team tree."
---

# Lys teams

Read the sibling lys skill before calling a tool.
This skill covers 13 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Read `/tree` for the caller's remit and `/teams/{id}` for current membership,
parent and lead before changes. Resolve people, agents, teams and machines by id;
they are not interchangeable member kinds. Membership confirmation and removal
have separate routes. Replacing nesting changes both parent and lead under the
route's rules; do not model a tree change as a display-only rename.

Team budget and goals live below `/teams/{id}` and use the same actual team id.
Read limits before replacement and preserve the goal's evidence and judge. Retiring
a team can affect visibility and inherited authority. Read membership admission
through the grant routes when asked why a member may act; membership alone is not
a grant. Read changes back through the same authorized tree, with page cursors
where served.
