# The directory contract

The directory of people and agents, as crates/lys-identity gives it. Every change is one signed event committed to the log before it is answered (P4, P5), and IDENTITY-EVENTS.md gives the bytes.

## Identifiers

A person's id is `person-` and 32 lowercase hex digits. An agent's id is `agent-` and 32 lowercase hex digits. Each is sixteen bytes from the operating system's secure random source, drawn when the identity is registered, and never changes. An id is never derived from an email, a display name or a login (P1).

## Logins

A login binding is an issuer and a subject, exactly as the issuer's token names them. The issuer is an http or https URL of at most 2048 bytes. The subject is at most 255 bytes. Neither may be empty, carry surrounding whitespace or a control character, and neither is rewritten. One binding names at most one person. A second binding of a login already bound is refused `BindingTaken`, naming the person it is bound to.

## Profiles

A profile is a display name of 1 to 200 characters, with no surrounding whitespace and no control character. It shows an identity and never establishes one. It carries no email.

## Provenance

Every change names its actor: a login of the person the service authenticated, the method and when. The actor is always a person. The method is one of two: the person's own OIDC sign-in, or a request signed by an agent the person is responsible for, whose signature the service verified, in which case the event also keeps the agent's id. The service signs the event. It attests the actor, and it never claims a person signed anything (P8).

## Registration

| Call | Answers | Records |
| --- | --- | --- |
| `Directory::register_person(actor, operation, profile, at)` | the new `PersonId` and the receipt | a person in state registered |
| `Directory::register_agent(actor, operation, responsible, profile, at)` | the new `AgentId` and the receipt | an agent in state registered, with `responsible` as its responsible person for life |

An agent's responsible person must be registered, or the call is refused `IdentityUnknown` and nothing is recorded. In step 1 the actor is always the configured administrator, and the responsible person is the administrator's person. Registering an agent starts nothing, and issues no login, credential, handle or certificate (P3). No call changes an agent's responsible person.

## Reading

| Call | Answers |
| --- | --- |
| `Directory::record(id)` | the identity's record, or none |
| `Directory::projection()` then `records()` | every identity, in id order |
| `Record::profile()`, `state()`, `responsible()`, `bindings()`, `registered_by()`, `events()` | the record's parts, and the log indices of its events |

A read answers only once any uncertain append is resolved. While one is held, every read and every change is refused `AppendUncertain` or `LogUnavailable`. Resolving it applies every leaf the log holds from that index on, whoever wrote it, before anything is answered. A leaf past the pin that is not a whole event this directory signed is refused `LeafNotAnEvent` before it is pinned, so it can be removed without equivocating. An event larger than the directory reads back is refused `EventTooLarge` before it is signed.

## Changes

| Call | Records |
| --- | --- |
| `Directory::change_profile(actor, operation, identity, profile, at)` | the new profile |
| `Directory::bind_login(actor, operation, person, binding, at)` | one more login bound to the person |
| `Directory::transition(actor, operation, identity, transition, reason, at)` | one lifecycle transition, from the state the identity is in |

Activate is registered to active, suspend is active to suspended, reinstate is suspended to active, and retire is active or suspended to retired. Suspend and retire name a reason. Any other transition is refused `TransitionRefused` and records nothing. The state is recorded and never enforced (CN11).

## Operation ids

A caller names each change with an operation id (`op-` and 32 hex digits) and keeps it across retries. The same id with the same request answers the first receipt again and records nothing. The same id with a different request is refused `OperationReused`.

## Receipts

Every change is answered with a receipt carrying the envelope version, the operation id, the actor, the identity, the change kind, the SHA-256 payload commitment and the log coordinate: leaf index, tree size, root and leaf hash. `receipt::verify_receipt` checks a receipt against the signed message, the service key, a checkpoint and an inclusion proof.

## Refusals

Every refusal is an `IdentityError` variant, named in its message's first word: `IdentifierMalformed`, `BindingMalformed`, `ProfileInvalid`, `ReasonRequired`, `TransitionRefused`, `ChangeMismatch`, `IdentityUnknown`, `AlreadyRegistered`, `BindingTaken`, `StateMismatch`, `OperationReused`, `LinkSourceSeen`, `AppendUncertain`, `AppendRefused`, `LogUnavailable`, and the envelope's own refusals in IDENTITY-EVENTS.md.

## Enduring agents and session credentials

This is road adjustment 3, from docs/design/identity/STATEMENT-2026-09-22.md:162-167, where the road and its adjustments live. The directory holds to these rules:

1. The agent record is the enduring identity, and it holds no session credential.
2. A session is a separate record that points at exactly one agent by that agent's enduring id.
3. Starting a session never creates an agent.
4. An agent may be registered before it ever runs, and while no session of it exists it holds no session credential.
5. A second session of the same agent presents the same enduring agent id with a new session credential.
6. A session credential is distinct from the agent's one lys certificate (ADR-008), and it never mints an agent.
7. A started agent reports back to the directory (ADR-007), and the directory checks what it presents; the presentation step is built by the card of roadmap row RM-029, not by this contract.

The form of the session credential is open for the road step 2 card. It has three readings: a per-session lys certificate, a SpeaksFor delegation and an ADR-001 handle. This contract chooses none of them. The proof of rule 5 is blocked on the road step 2 card that brings sessions, roadmap row RM-029. What this contract delivers is the written rules and a registered agent with no session credential, not a working second session.
