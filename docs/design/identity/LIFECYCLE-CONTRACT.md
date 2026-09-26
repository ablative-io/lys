# The identity lifecycle contract

This document writes down, in one place, the lifecycle contract as it already
stands on main: the states and transitions of
docs/design/identity/LIFECYCLE-STATES-2026-09-22.md, the audit-record shape
and service-attestation rule of IDENTITY-001, DIRECTORY-003 R5's table of
refused transitions, and the directory decisions ADR-011, ADR-027 and
ADR-028. It restates; it decides nothing new, fixes no encoding, and leaves
open everything the design records as open. It is the contract that
DIRECTORY-009 rows R2 to R6 are built and reviewed against
(docs/design/directory/briefs/DIRECTORY-009.json). Every path in it is
relative to the repository root.

## 1. The four states

An identity, a person or an agent, is in exactly one of four states at a time
(docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:19-24, ADR-011).

| State | Meaning | May act? |
|---|---|---|
| registered | Exists in the directory. No grants, no credential handle. | No |
| active | May act. Executions may be started under it. Grants are effective. | Yes, within grants |
| suspended | Kept whole, grants retained but not effective. Every check refuses by name. Running executions stop at their next admission. | No |
| retired | Permanent. History and audit kept. Never reactivated; a new identity is made instead. | No |

There is no fifth state: provisioned is a view (has any grant, has a handle)
and not a state, so a grant can be added or removed without a state change
(docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:26-28).

## 2. The transitions, and who may cause them

| Recorded as | From | To | Who may cause it |
|---|---|---|---|
| register | (none) | registered | An administrator. A person for themselves by first sign-in is the lifecycle document's second causer; in step 1 first sign-in registers nobody (DIRECTORY-003 R1), and that path arrives with step 2's self-registration. |
| activate | registered | active | An administrator; when the identity is an agent, also its responsible person. |
| suspend | active | suspended | An administrator; when the identity is an agent, also its responsible person. |
| reinstate | suspended | active | An administrator; when the identity is an agent, also its responsible person. |
| retire | active or suspended | retired | An administrator; when the identity is an agent, also its responsible person. |

An administrator is always a signed-in person whom the directory's
administrator policy admits (P9). An agent's responsible person is the person
its registration record names, for life; the rule reads them from that
record and never from a request
(docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:77-81). Every other
actor is refused lifecycle_actor_not_permitted and nothing is appended.

In this contract no policy causes a transition. The only automatic effect a
suspension or retirement has on other identities is the grant-check one
DIRECTORY-006 R4 enforces: every grant derived from the suspended or retired
identity refuses at the next check
(docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:87-89). A policy that
moves an identity between states would be a later revision of this contract.

## 3. The record rule

Every transition is one signed, service-attested audit record of
IDENTITY-001's shape, naming the attested actor and their provenance, the
identity, from, to, when and the reason given
(docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:40-44,
docs/design/identity/briefs/IDENTITY-001.json:41 and
docs/design/identity/briefs/IDENTITY-001.json:52). It is service-attested: it
never claims a person signed with a key they did not hold. It is committed
through the directory's event path (DIRECTORY-003 R2) and returned with its
receipt. The reason is recorded as given; whether a reason is mandatory on
suspend and retire stays the room's open question
(docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:101, ADR-011).

## 4. The fold rule

The state is folded from the log and never set directly (ADR-027). The state
of an identity at any moment is the fold of that identity's transition
records in log order, from registered. No API, migration or repair sets it: a
transition validates against the folded state (DIRECTORY-003 R5's table) and
the cause rule of section 2, appends its record through the directory's event
path, and the state is read again by folding. Any stored copy of the state,
such as the field DIRECTORY-003 R5's projection keeps, is filled from the fold
and from nothing else, rebuilt from it at open and equal to it after every
append.

A read answers the fold or refuses by name at the coordinate of a record the
table refuses: a transition from a state it cannot leave, or a record for an
identity with no register record before it, is refused lifecycle_fold_invalid
naming that record's log coordinate. The read never answers a state past the
record, never repairs the log and never skips the record. A read writes
nothing.

## 5. The check rule

An access check, whether this identity may perform this action on this
resource now, passes only when the identity is active and the grant exists
and the grant is fresh, in that order
(docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:48-53). The state leg is
answered from the fold of section 4; the grant legs are answered by
DIRECTORY-006's permission decision, with SpiceDB the answer to the grant
question and Rauthy the source of the sign-in event, and their refusals pass
through under the names that decision gives. A suspended or retired identity
refuses every check by state alone, before the grant question is asked,
whatever grants it holds. An answered check commits no event and appends
nothing.

## 6. The next-check rule

A suspension or retirement takes effect at the next check (ADR-028). The
directory's resolution of a Rauthy-authenticated session at sign-in and at
refresh is a check, so a refresh is a check: a session that outlives a
suspension is refused at its next refresh and at every access check before
that, by state, whatever grants it holds. A refresh checks state alone, so an
active identity holding no grant is admitted at refresh.

A session is refused only by the directory's own check:
nothing in this contract calls Rauthy to revoke, end or invalidate a session,
and nothing keeps a list of sessions to end. An action already admitted
before the transition is never rolled back. An at-once revocation of the
Rauthy session would be a later revision of the suspend transition, and
whether suspending a person also ends their sign-in session at Rauthy stays
open as the design records it
(docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:99-100).

## 7. The retired rule

Retirement closes the identity and never its record. A retired identity's
history and audit are kept: its state, its transitions and the record that
put it there are readable exactly as a live identity's are. Nothing deletes,
hides, redacts or truncates an audit record of any identity. A retired
identity is never reactivated; a new identity is made instead
(docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:24, ADR-011, ADR-027).

## 8. What the screen shows

Per identity (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:57-61):

- its name and kind, person or agent;
- its state, and the actions available in that state and nothing else:
  in registered, activate; in active, suspend and retire; in suspended,
  reinstate and retire; in retired, none;
- the record that put it in that state, as the last audit line: the
  operation, the attested actor and their provenance, from, to, when, the
  reason given and the log coordinate;
- whether it is provisioned, computed as a view beside the state and never
  stored.

A list view carries the same columns, one row per identity, filterable by
state and by kind. A retired identity appears in both exactly as a live one
does, with its retire record as the record that put it there and its actions
empty.

The typed read of DIRECTORY-009 R6 is what the screen consumes. Building the
screen is step 7's work and not DIRECTORY-009's.

## 9. The refusals

| Name | When it fires | The act that answers it |
|---|---|---|
| lifecycle_not_active | A check or session resolution for an identity whose folded state is registered. | Activate the identity. |
| lifecycle_suspended | A check or session resolution for an identity whose folded state is suspended. | Reinstate it. |
| lifecycle_retired | A check or session resolution for an identity whose folded state is retired. | None; make a new identity. |
| lifecycle_unknown_identity | A check or read naming an identity with no record in the log. | Register it. |
| lifecycle_actor_not_permitted | A transition requested by an actor section 2 does not name for it, or by an actor who is not an attested person. | Have the administrator or the agent's responsible person cause the transition. |
| lifecycle_fold_invalid | The fold meets a record the transition table refuses, named at its log coordinate. | Verify the leaf at the named coordinate against its receipt and read the log by hand; the log is never repaired. |

## 10. What IDENTITY-EVENTS.md must carry before the code rows start

docs/design/identity/IDENTITY-EVENTS.md, which DIRECTORY-003 owns under its
joint review, must carry before DIRECTORY-009 R2 to R6 start: the five
transition operations, register, activate, suspend, reinstate and retire, in
its event vocabulary, each with from, to and reason in its typed payload.
This is an input to that joint review and never an edit made from here; the
signed encoding of the transition record stays with that document, and this
contract fixes none.
