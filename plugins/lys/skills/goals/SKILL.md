---
name: goals
description: "Manage Lys goals, alarms and scheduled delivery with occurrence evidence."
---

# Lys goals

Read the sibling lys skill before calling a tool.
This skill covers 13 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Read the named goal, alarm or schedule and its responsible person before acting.
A goal mark needs evidence and its authorized judge. Rewording or suspending a
standing aim does not prove it achieved. A schedule keeps occurrence counts when
changed; do not replace it with a fresh schedule to reset history.

Follow deliveries by occurrence and receipt. An uncertain goal resend requires a
distinct authorized occurrence and the prior delivery id. Acknowledging an alarm
is personal and does not repair its cause. Schedule activation, stopping and goal
judgment are changes; do not perform them as connection checks.

Use notifications or the admitted change signal for the next occurrence. No
interval-based list loop, no fixed sleep and no elapsed-time claim of delivery.
Return a pending handoff with the goal/schedule/occurrence and expected signal
when the next act belongs to another person or process.
