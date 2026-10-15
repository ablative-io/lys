---
description: "Identify the caller of this Lys connection without confusing a person and an agent."
disable-model-invocation: true
---

Read the lys and identity skills. Call `what-can-I-do` with `{}` first.
When discovery identifies `connection.mode` as `person_reads`, the person chose
Person reads in Lys's browser consent. Ask `read {"path":"/me"}`. Report the
returned person as the person this connection reads on behalf of, and name the
connection from its read receipt. This never grants approval or signing authority.
In agent mode, a connected app or signed agent keeps its own identity. Report
only the host's supplied identity or an admitted own-grant reading; ordinary
agent discovery does not supply a holder id. Do not claim `/me` works in agent
mode or replace its credential. To enable own-person reads, reconnect and choose
Person reads in the browser. Agent mode remains the default for existing consent.
