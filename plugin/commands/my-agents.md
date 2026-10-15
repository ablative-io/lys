---
description: "List the agents visible to the current Lys caller, with honest authentication limits."
disable-model-invocation: true
---

Read the lys and identity skills and call `what-can-I-do` first.
When discovery identifies `connection.mode` as `person_reads`, the person chose
Person reads in browser consent. Use `read {"path":"/dashboard?limit=50"}` for
that person's own agents. This read returns only agent rows and paging fields.
Follow the returned page cursor with the same filters
until the requested list is complete. Keep each connection/person/route receipt
and report any page that refused. Changing agents, giving access and approvals
as the person need the browser; this connection can only read their views.
`/apps/me/agents` is for a product person access pass, not connected-app OAuth.
In agent mode, for an OAuth app agent or signed seat, use only an admitted route
from discovery that lists the requested agents. If none exists, report that this connection
cannot list the person's agents. Do not read the administrator directory as a
substitute. An empty authorized list and a refusal are different outcomes.
