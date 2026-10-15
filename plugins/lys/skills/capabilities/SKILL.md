---
name: capabilities
description: "Show the routes, resources and actions this Lys connection currently holds."
---

Read the lys skill. Call `what-can-I-do` with exactly `{}`.
Report the listed resources and their actions, route count and any degraded or
refused reading. If there are no resources, say this connection has no resource
grants. Listing a route does not grant its authority. Do not exercise a route to
claim all listed routes work. Keep the response scoped to the user's question.
