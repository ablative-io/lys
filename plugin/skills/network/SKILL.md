---
name: network
description: "Read Lys computers, runners, client artifacts and join evidence."
---

# Lys network

Read the sibling lys skill before calling a tool.
This skill covers 20 declared operations. Use [the route reference](references/routes.md)
to find their exact methods and paths, then inspect the live OpenAPI operation.

Resolve the machine's owner, owning team and allowed agents. Computer ownership
and agent ownership are separate requirements. Runner absent, unreachable, retired
and protocol-mismatch are distinct refusals; never repair an adjacent machine.
Folder enumeration and folder choice are POST questions with the declared body.
Do not invoke a visible folder dialog without the user's task requiring it.

Join codes, installer downloads and dial passes are credentials or install acts,
not health probes. Never print their bodies. Do not install, upgrade, restart or
join a runner unless that is explicitly the job. Client artifacts name a commit
and SHA-256; verify those before an authorized install. A download is not install
proof. Dial-next is an event-held request for the runner, not an assistant polling
interface. Read runner receipts separately from service health.
