---
name: my-agents
description: "List the agents visible to the current Lys caller, with honest authentication limits."
---

Read the lys and identity skills and call `what-can-I-do` first.
For the person's own connection, use `read {"path":"/dashboard?limit=50"}` and
follow the returned page cursor with the same filters until that requested list
is complete. Report the agents and any page that refused.
`/apps/me/agents` is for a product person access pass, not connected-app OAuth.
For an OAuth app agent or signed seat, use only an admitted route from discovery
that lists the requested agents. If none exists, report that this connection
cannot list the person's agents. Do not read the administrator directory as a
substitute. An empty authorized list and a refusal are different outcomes.
