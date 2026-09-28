# The lifecycle contract

What an identity's lifecycle state is, who may move it, how it is read, and how it gates a check, as DIRECTORY-009 builds it. DIRECTORY-009 R1 writes it; R2 to R6 implement it.

## Status

This document writes down what is already settled and decides nothing new. The states and transitions are the lifecycle document's (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-37, ADR-011). The audit-record shape and the service-attestation rule are IDENTITY-001's (docs/design/identity/briefs/IDENTITY-001.json:41, docs/design/identity/briefs/IDENTITY-001.json:52). The table of refused transitions is DIRECTORY-003 R5's. The fold rule is ADR-027, the next-check rule is ADR-028 and the retired rule is CN21. No encoding is fixed here: a transition record's signed bytes are docs/design/identity/IDENTITY-EVENTS.md's, under DIRECTORY-003's joint review. Everything the design records as open stays open, and is named as open where this contract meets it.

## 1. The states

An identity is in exactly one of four states at a time (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:19-24).

| State | Meaning | May act? |
| --- | --- | --- |
| registered | Exists in the directory. No grants, no credential handle. | No |
| active | May act. Executions may be started under it. Grants are effective. | Yes, within grants |
| suspended | Kept whole, grants retained but not effective. Every check refuses by name. Running executions stop at their next admission. | No |
| retired | Permanent. History and audit kept. Never reactivated; a new identity is made instead. | No |

Whether an identity has grants or a credential handle is a fact beside the state, not a state: provisioned is a view (has any grant, has a handle) and not a state, so a grant can be added or removed without a state change (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:26-28).

## 2. The transitions and who may cause them

| Recorded as | From | To | Who may cause it |
| --- | --- | --- | --- |
| register | (none) | registered | an administrator; or a person for themselves by first sign-in, which in step 1 registers nobody (DIRECTORY-003 R1) and arrives with step 2's self-registration |
| activate | registered | active | an administrator; for an agent, also its responsible person |
| suspend | active | suspended | an administrator; for an agent, also its responsible person |
| reinstate | suspended | active | an administrator; for an agent, also its responsible person |
| retire | active or suspended | retired | an administrator; for an agent, also its responsible person |

An administrator is a signed-in person the directory's administrator policy admits (P9). An agent's responsible person is the person its registration record names, read from that record and never from the request (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:77-81). Every other actor is refused `lifecycle_actor_not_permitted`, and nothing is appended.

In this contract no policy causes a transition. The only automatic effect a suspension or retirement has on other identities is the grant-check one DIRECTORY-006 R4 enforces: every grant derived from a suspended or retired identity refuses at the next check (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:87-89). A policy that moves an identity between states would be a later revision of this contract.

## 3. The record

Every transition is one signed, service-attested audit record of IDENTITY-001's shape (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:40-44, docs/design/identity/briefs/IDENTITY-001.json:41, docs/design/identity/briefs/IDENTITY-001.json:52). It names the attested actor and their provenance, the identity, from, to, when and the reason given. The directory service signs it; it never claims a person signed bytes with a key they did not hold (P8). It is committed through the directory's event path (DIRECTORY-003 R2) and returned with its receipt, which carries the log coordinate the append yielded; the receipt is never itself appended.

The reason is recorded as given. Whether a reason is mandatory on suspend and retire stays the room's open question (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:101, ADR-011).

## 4. The fold

An identity's state is folded from the log and never set directly (ADR-027). The state at any moment is the fold of that identity's transition records in log order, starting from registered at its register record. No API, migration or repair sets it. A transition is validated against the folded state and the causer rule of section 2, its record is appended, and the state is read again by folding.

A read answers the fold, or refuses `lifecycle_fold_invalid` at the log coordinate of a record the transition table refuses: a transition from a state it cannot leave, or a record for an identity with no register record before it. The read never answers a state past such a record, never skips it and never repairs the log. A read opens no log for writing and commits nothing.

## 5. The check

An access check, whether this identity may perform this action on this resource now, passes only when the identity is active and the grant exists and the grant is fresh, in that order (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:48-53).

- The state leg is answered from the fold of section 4, and is asked first.
- The grant legs are answered by DIRECTORY-006's permission decision, with the refusal names that decision gives. SpiceDB is the answer to the grant question; Rauthy is the source of the sign-in event. Neither is asked by the state leg.

A registered, suspended or retired identity refuses every check by state alone, before the grant question is asked, whatever grants it holds. An unknown identity refuses the same way. An answered check commits no event and leaves no receipt.

## 6. The next check

A suspension or retirement takes effect at the next check (ADR-028). The directory's resolution of a Rauthy session to an identity, at sign-in and at refresh, is a check, so a refresh is a check: a session that outlives a suspension is refused at its next refresh, and at every access check before that, by state, whatever grants it holds. A refresh asks the state alone, so an active identity with no grant is admitted at refresh.

A suspension is not a revocation: nothing in this contract calls Rauthy to revoke, end or invalidate a session, and nothing keeps a list of sessions to end. An action already admitted is never rolled back. An at-once revocation would be a later revision of the suspend transition. Whether suspending a person also ends their sign-in session at Rauthy stays open, as the lifecycle document records it (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:99-100).

## 7. The retired identity

A retired identity's history and audit are kept (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:24). Its state, its transitions and the record that put it there are readable exactly as a live identity's: the same fields, the full history, its retire record as the record that put it there. Nothing deletes, hides, redacts or truncates a record of any identity (CN21). A retired identity is never reactivated; a new identity is made instead.

## 8. What the screen shows

Per identity (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:57-61):

- its name and kind, person or agent;
- its state, and the actions available in that state and nothing else: from registered, activate; from active, suspend and retire; from suspended, reinstate and retire; from retired, none;
- the record that put it there, as the last audit line: the operation id, the attested actor and their provenance, from, to, when, the reason given and the log coordinate;
- provisioned, as a view beside the state, never stored.

The list view is the same columns, one row per identity, filterable by state and by kind. A retired identity appears in it as a live one does.

The typed read of DIRECTORY-009 R6 is what the screen consumes. The screen itself is step 7's and is not built by DIRECTORY-009.

## 9. The refusals

Every refusal is named and carries the act that answers it.

| Refusal | When it fires | The act that answers it |
| --- | --- | --- |
| lifecycle_not_active | a check of an identity that is registered | activate the identity |
| lifecycle_suspended | a check, a sign-in or a refresh of an identity that is suspended | reinstate it |
| lifecycle_retired | a check, a sign-in or a refresh of an identity that is retired | none; make a new identity |
| lifecycle_unknown_identity | a check or a read of an identity with no register record | register it |
| lifecycle_actor_not_permitted | a transition requested by an actor section 2 does not name for it | have the administrator or the agent's responsible person cause the transition |
| lifecycle_fold_invalid | a read whose fold meets a record the transition table refuses, named by its log coordinate | verify the leaf at the named coordinate against its receipt and read the log by hand; the log is never repaired |

## 10. What the event vocabulary must carry

Inputs to DIRECTORY-003's joint review of docs/design/identity/IDENTITY-EVENTS.md, never edits made by this contract. Before DIRECTORY-009 R2 to R6 start, that document carries the five transition operations, register, activate, suspend, reinstate and retire, each with its from, its to and the reason given in the typed payload, so that the fold of section 4 is answered from the signed records alone. Where the reviewed vocabulary records register as its own change kinds rather than as a transition, the fold starts an identity at registered from that record, and the vocabulary is read as it stands; this contract asks no change of it.
