---
description: "List and prepare Lys drafts; hand personal decisions back to the browser."
disable-model-invocation: true
---

Read the lys and draft-workflow skills. The requested draft act is: $ARGUMENTS
Call `what-can-I-do` first. When `connection.mode` is `person_reads`, the person
chose Person reads in browser consent. List their own waiting directory drafts
with `drafts {"path":"/drafts?state=waiting"}`. Use `state=all` to include their
own raised and decided directory drafts. Follow cursors with the same filters
and keep the connection/person/route receipts. Another person's drafts are never
available through this consent, including when its person is an administrator.

Read their own product drafts with `read {"path":"/product-drafts?limit=50"}`.
These include drafts held by the person or recorded with them as responsible,
including their own raised and waiting drafts. Filtering happens before paging.
Follow `after` cursors with the same limit. Use each returned row's state to
select waiting drafts; this person read takes no `app` or `state` query.
Keep every read receipt. The `drafts` tool itself accepts only `/drafts` paths.

Deciding, approving, refusing, correcting, giving or signing as the person needs
the browser. A decision call returns `PersonPresenceRequired` with
`deciding a draft needs you in Lys: <url of that draft>`. Give the person that
URL; never submit a browser decision or substitute a credential on their behalf.
Never present an administrator's browser product list as the connection's own list.

In agent mode, existing agent admission is unchanged. Directory creation needs
an agent signature; this OAuth plugin supplies no signing key. Use `read` for an
admitted `/product-drafts` reading in that mode. Prepare only the requested act,
inspect its live schema and preserve exact bytes, id and creation hash. Hand
back the prepared request when the connection cannot submit it. Never approve
as the person. Approval records a decision only; execution needs its own receipt
and state read-back.
