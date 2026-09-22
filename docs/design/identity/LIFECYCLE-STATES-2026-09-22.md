# The start of the lifecycle: states, transitions, and what the screen shows

Archie, 22 September 2026, 17:10 Melbourne, after Tom's correction at 17:08
("identity, access management, permissions and the start of lifecycle
management first"). One page. A proposal for the room, not a settled
design; it exists so that the directory (IDENTITY-001 row 04) has a thing
to register an identity into and the lifecycle screen (step 7) has a thing
to show. Nothing here is built.

## Two things have a lifecycle; tonight is about the first

An **identity** endures: a person or an agent, with one ID that survives
providers, keys, models and sessions. An **execution** is one running
session or fork under an identity, with its own execution ID and a link to
its parent. Executions come later; this page is the identity's lifecycle.

## Identity states

| State | Meaning | May act? |
|---|---|---|
| registered | Exists in the directory. No grants, no credential handle. | No |
| active | May act. Executions may be started under it. Grants are effective. | Yes, within grants |
| suspended | Kept whole, grants retained but not effective. Every check refuses by name. Running executions stop at their next admission. | No |
| retired | Permanent. History and audit kept. Never reactivated; a new identity is made instead. | No |

Whether an identity has grants or a credential is a fact beside the state,
not a state: "provisioned" is a view (has any grant, has a handle), so a
grant can be added or removed without a state change.

## Transitions, and who may cause them

| From | To | Caused by | Recorded as |
|---|---|---|---|
| (none) | registered | an administrator; or a person for themselves by first sign-in | register |
| registered | active | an administrator | activate |
| active | suspended | an administrator | suspend (reason) |
| suspended | active | an administrator | reinstate |
| active or suspended | retired | an administrator | retire (reason) |

Every transition is one signed audit record: who (the authenticated actor
and their provenance), which identity, from, to, when, reason. It is
service-attested; it never claims a human signed with a key they did not
hold (IDENTITY-001 shared contract). Grant and revoke are audit records of
the same shape with a relation instead of a state.

## Access is beside the state, and the state gates it

Grants live in the permission store (SpiceDB): identity, relation, object,
for example agent X is `editor` of project P. A check passes only when the
identity is active **and** the grant exists **and** the check is fresh.
Suspend leaves the grants and makes every check refuse; revoke removes one
grant and leaves the state. Both take effect at the next check; neither
promises to roll back an action already admitted.

## What the screen shows, per identity

Name and kind (person or agent). State, and the actions available in that
state and nothing else. Grants as a list of relation and object. Credential
handles as a count and last rotation, never a value. Executions as a count
of running ones. The last audit line. The list view is the same columns,
one row per identity, filterable by state and kind.

## Chippy's acceptance path, mapped

Sign in: a person becomes registered and active by first sign-in. Create an
agent: registered, then activate. Grant project access: a grant, no state
change. Allowed action: active, grant present, fresh check, admitted.
Suspend, or revoke: a transition, or a grant removal. Same action refused:
the check fails by name. Inspect: the audit record names who did it.

## Every identity hangs from a human (Tom, 17:15; added 17:19)

Tom's ruling in the room: everything is pegged to a human authority. The
accounts and registrations are in a person's name; the person signs in, and
agents are provisioned under that person. Read into this page:

- A person is registered by first sign-in. An agent is registered by a
  signed-in person, and carries that person as its **responsible human** for
  life. There is no identity without a human above it; "an administrator" in
  the table above is always a signed-in person, and the responsible human may
  cause every transition of their own agents.
- An agent's grants are an explicit subset of its human's. The human's access
  is the ceiling; nothing is inherited automatically (Chippy's reading, agreed).
- A grant says two separate things: who may **exercise** it, and whether it
  may be **passed on**: human-delegatable, agent-delegatable, or not
  delegatable. Exercising and delegating are different permissions.
- Every grant records its provenance back to the human who authorised it.
  Suspending or retiring that human, or revoking the grant it derives from,
  makes every derived grant refuse at the next check. Nothing outlives its root.
- The screen therefore shows, per agent, the responsible human, the granted
  scope, and the delegation rights, together.

This is the target design. What the wall shows today (a configured account
and an actor on each run) is observed, not enforcement; the test flows on the
Test project describe it as such.

## Open, for the room

- Does suspending a person also end their sign-in session at Rauthy, or
  only make our checks refuse? Two systems, one decision.
- Is a reason mandatory on suspend and retire? Proposed yes.
- Does an agent's activation require a credential handle to exist first?
  Proposed no: an agent can be active with nothing to hold.
- When an agent that delegated a grant onward is suspended, does the grant
  it delegated fall with it? Proposed yes: it is in the provenance chain.
