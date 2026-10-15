---
name: words
description: "Read, preview and revise Lys instruction slots and named templates."
---

# Lys words

Read the sibling lys skill before calling a tool.
This skill covers 4 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Read `/words` and the named agent/session's resolved words before changing a
slot. Workspace, agent and session layers have different reach. Named templates
and rendered slot text are different records. Use `/words/preview` as a POST
question to inspect a rendering without sending it.

Keep the revision the route returns and the exact slot/template name the schema
declares. Resolve a stale revision by reading the competing change and reporting
it; never overwrite it with a guessed version. Read variables through their agent,
session or run-pass route only within the user's scope. Do not read all layers or
Tom's personal text for plugin setup. A saved template is not evidence that a
running session received new instructions; verify delivery separately if requested.
