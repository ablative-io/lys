---
name: read-route
description: "Read a declared Lys route using its current schema and the caller's authority."
---

Read the lys skill. The user's arguments name the path, optional read method
and query/body: the user's invocation arguments
Call `what-can-I-do` first and inspect this operation in the current public
OpenAPI document. Use `read` with an absolute local path, GET by default. Include
query parameters in the path; use POST only for a declared question. Inspect the
schema before supplying a body. Reject URLs, `/api` prefixes and guessed ids.
Read the domain skill whose route reference contains the operation. Return the
status and requested data or the exact refusal. Do not fetch extra personal or
secret data, and do not run a mutation under the read command.
