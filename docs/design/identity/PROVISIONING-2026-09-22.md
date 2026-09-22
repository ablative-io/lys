# Provisioning an agent under a human

Archie, 22 September 2026, 17:22 Melbourne. Prep while the test flows are
readied; a proposal for the room, not a settled design. Follows Tom's ruling
of 17:15 (everything pegged to a human authority) as written into
LIFECYCLE-STATES-2026-09-22.md, and answers his question from the private
room: "how do I provision a new agent?" Nothing here is built.

## The path, in order

Each step produces one record and is useful before the next exists.

| # | Step | Who | Produces | Exists today |
|---|---|---|---|---|
| 1 | The human signs in | the person | a person identity, registered and active by first sign-in | The door signs people in with Google. Rauthy is not installed (IDENTITY-001 rows 02 to 05 pending). |
| 2 | Register the agent | the signed-in person | an agent identity, state registered, responsible human = that person, for life | Seats exist as registry ids in cambium's roster and as Argus seats. Neither carries a responsible human or a state. |
| 3 | Grant | the responsible human, or an admin under one | grant records: relation, object, exercisable-by, delegatable (human, agent, none), derived-from | Nothing. Access today is whatever the seat's environment holds. |
| 4 | Credential handle | the secrets broker (step 3 of the road) | a handle under the identity, never a value in the seat | The pool file. The login exception stands: the harness login token still reaches its process. |
| 5 | Activate | the responsible human or an admin | transition registered → active, one signed audit record | Nothing. A seat is "active" when its process runs. |
| 6 | Render the home onto a harness | the launcher, from the identity's profile | the command template: working directory, appended prompt, MCP configuration, environment, secrets by handle | Aion's claude-worker takes a working directory, a config directory and a session name per run (proved 21 September). No profile record, no identity behind it. |
| 7 | Start an execution | the launcher | an execution id linked to the agent identity and the human above it; audit line | Aion runs and Argus sessions have ids. Neither links to an enduring identity. |

Steps 1, 2, 3 and 5 are Chippy's line (steps 1 and 2 of the road) and are
listed so the whole path is visible; steps 6 and 7 are mine (step 5 of the
road). Step 4 is the broker. The screen (step 7 of the road) shows the path
as it stands for one agent: which of the seven have happened, and the audit
line for each.

## What one grant says

One row, five facts: **who** may exercise it (the agent identity), **what**
(relation on object, as SpiceDB expresses it), **from whom** it derives (the
human, or a grant that itself derives from the human), **whether it may be
passed on** (human-delegatable, agent-delegatable, or not delegatable), and
**the audit record** that created it. A check reads active state, the grant,
and freshness; nothing else. Revoking the root revokes the chain at the next
check. This is the shape; the store and schema are Chippy's to settle.

## The first useful screen on this path

Sign in; create an agent (steps 1 and 2); give it one grant on one project
(3); activate it (5); see a permitted action admitted and, after revoke or
suspend, the same action refused by name; read the audit line for each
change. That is Chippy's concrete first screen of 17:08 with the responsible
human shown on it. Steps 4, 6 and 7 are not on that screen and are not
needed for it.

## Observed, not enforced

Every "exists today" cell above is observed on this Mac on 22 September.
None of it enforces the ruling: no seat has a responsible human, no grant
record exists, and a run's actor is a configured account rather than an
identity. The test flows on the Test project describe their actor the same
way.

## Open, for the room

- May a person register an agent for another person to be responsible for?
  Proposed no: the registering person is the responsible human, transfer is a
  later transition.
- Does step 6 require step 4, or may a development agent run with the pool
  file until the broker lands? Proposed: allowed in development, and the
  audit line says which.
