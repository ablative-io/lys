---
description: "Identify the caller of this Lys connection without confusing a person and an agent."
disable-model-invocation: true
---

Read the lys and identity skills. Call `what-can-I-do` with `{}` first.
For a personal connection, ask `read {"path":"/me"}`. Report only the caller's
returned identity and authentication kind needed to answer the user.
A signed agent or OAuth connected app is an agent, not its responsible person.
If this connection is an agent, report the identity supplied by the host's
connection or its admitted own-grant reading; discovery alone contains no holder
id. Do not claim `/me` works for an agent. Report its refusal if asked and do not
switch to a person's credential to make the command succeed.
