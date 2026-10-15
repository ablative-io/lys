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

Deciding, approving, refusing, correcting, giving or signing as the person needs
the browser. A decision call returns `PersonPresenceRequired` with
`deciding a draft needs you in Lys: <url of that draft>`. Give the person that
URL; never submit a browser decision or substitute a credential on their behalf.
Product-draft reads through Person reads are currently refused by name because
the own-product-draft helper is unavailable. Hand back the browser drafts page
for that family; never present an administrator's product list as an own list.

In agent mode, existing agent admission is unchanged. Directory creation needs
an agent signature; this OAuth plugin supplies no signing key. Use `read` for an
admitted `/product-drafts` reading in that mode. Prepare only the requested act,
inspect its live schema and preserve exact bytes, id and creation hash. Hand
back the prepared request when the connection cannot submit it. Never approve
as the person. Approval records a decision only; execution needs its own receipt
and state read-back.
