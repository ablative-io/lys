---
name: runtime
description: "Operate Lys seats and runtime sessions with delivery receipts and event signals."
---

# Lys runtime

Read the sibling lys skill before calling a tool.
This skill covers 40 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Resolve the seat, its selected reviewed profile and current session/control ids.
Read the runner's own seat state; unreachable is not stopped. Seat version changes,
retirement and imports require the stopped state their routes declare.

For an authorized message or control act, preserve operation ids and read control
receipts. Uncertain delivery needs its reconcile route and responsible decision;
do not send again with a new operation to make uncertainty disappear. A goal
resend is a distinct authorized occurrence that names possible prior delivery.
Use exact-byte routes for a byte stream and screen routes for a screen. Read/wait
POST routes are questions; send/type/input/keys/end are acts through `change`.

Attach and wait follow runner signals. Do not use timer loops or repeatedly list
sessions for readiness. Keep `/changes` generation or the route's supplied stream
cursor; on wake re-read only the requested state. Import starts with its dry-run
plan; confirm exactly that plan after the required person approves it. A stale
plan is a refusal to resolve, not permission to rebuild and confirm silently.
Never stop everything, restart a service, or control another seat as a setup check.
