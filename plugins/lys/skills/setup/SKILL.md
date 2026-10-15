---
name: setup
description: "Verify the Lys plugin connection, live tool list and admitted discovery without changing state."
---

Resolve this plugin's MCP server in the host. If absent, report attachment missing
and point to the plugin README; do not edit host configuration automatically.
Read its live tools/list through the host's MCP inventory and verify the four bare
names `what-can-I-do`, `read`, `change`, `drafts`. Report any difference.
Call `what-can-I-do` with `{}`. Report route count and resource count, the exact
refusal if any, and whether authentication and resource access were each verified.
A route list with zero resources is not workspace access. Do not call `/me`, read
personal data, create a draft, issue a token, grant access or restart anything.
Use the host's Authenticate action for OAuth when needed. Return one outcome per
check. Read the sibling lys skill for the result envelope and the README for
installation. Stop on a refusal and name the person/authority the next act needs.
