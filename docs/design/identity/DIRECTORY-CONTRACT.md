# The directory contract

The directory of people and agents, as crates/lys-identity gives it. Every change is one signed event committed to the log before it is answered (P4, P5), and IDENTITY-EVENTS.md gives the bytes.

## Identifiers

A person's id is `person-` and 32 lowercase hex digits. An agent's id is `agent-` and 32 lowercase hex digits. Each is sixteen bytes from the operating system's secure random source, drawn when the identity is registered, and never changes. An id is never derived from an email, a display name or a login (P1).

## Logins

A login binding is an issuer and a subject, exactly as the issuer's token names them. The issuer is an http or https URL. The subject is at most 255 bytes, the bound OpenID Connect Core 1.0 section 2 sets on `sub`. Neither may be empty, carry surrounding whitespace or a control character, and neither is rewritten. One binding names at most one person. A second binding of a login already bound is refused `BindingTaken`, naming the person it is bound to.

## Profiles

A profile is a display name that is not empty, with no surrounding whitespace and no control character. It shows an identity and never establishes one. It carries no email.

## Provenance

Every change names its actor: a login of the person the service authenticated, the method and when. The actor is always a person. The method is one of two: the person's own OIDC sign-in, or a request signed by an agent the person is responsible for, whose signature the service verified, in which case the event also keeps the agent's id. The service signs the event. It attests the actor, and it never claims a person signed anything (P8).

## Registration

| Call | Answers | Records |
| --- | --- | --- |
| `Directory::register_person(actor, operation, profile, at)` | the new `PersonId` and the receipt | a person in state registered |
| `Directory::register_agent(actor, operation, responsible, profile, at)` | the new `AgentId` and the receipt | an agent in state registered, with a direct reporting edge to `responsible` |

An agent's responsible person must be registered, or the call is refused `IdentityUnknown` and nothing is recorded. In step 1 the actor is always the configured administrator, and the responsible person is the administrator's person. Registering an agent starts nothing, and issues no login, credential, handle or certificate (P3). `Directory::register_reporting_agent` accepts a person or agent target and resolves an active chain to its accountable person. `Directory::change_reports_to` changes that edge and the affected descendants' accountability in one signed event; earlier signed leaves remain unchanged. Inactive reporting links remain visible as a named gap and block acting authority until restored or reassigned. Snapshot version 3 materialises direct edges for existing agents through an explicit version-2 migration before startup opens the directory.

## Reading

| Call | Answers |
| --- | --- |
| `Directory::record(id)` | the identity's record, or none |
| `Directory::projection()` then `records()` | every identity, in id order |
| `Record::profile()`, `state()`, `responsible()`, `reports_to()`, `reporting_gap()`, `bindings()`, `registered_by()`, `events()` | the record's parts, and the log indices of its events |

A read answers only once any uncertain append is resolved. While one is held, every read and every change is refused `AppendUncertain` or `LogUnavailable`. Resolving it applies every leaf the log holds from that index on, whoever wrote it, before anything is answered. A leaf past the pin that is not a whole event this directory signed is refused `LeafNotAnEvent` before it is pinned, so it can be removed without equivocating. An event of any size is signed, and is read back whole.

## Changes

| Call | Records |
| --- | --- |
| `Directory::change_profile(actor, operation, identity, profile, at)` | the new profile |
| `Directory::bind_login(actor, operation, person, binding, at)` | one more login bound to the person |
| `Directory::transition(actor, operation, identity, transition, reason, at)` | one lifecycle transition, from the state the identity is in |

Activate is registered to active, suspend is active to suspended, reinstate is suspended to active, and retire is active or suspended to retired. Suspend and retire name a reason. Any other transition is refused `TransitionRefused` and records nothing. The state is recorded and never enforced (CN11).

## The grant path, recorded open for Tom

DIRECTORY-003 R5 records this question and decides nothing. The registration, profile, login and lifecycle calls above ask SpiceDB for no decision and write no grant.

The path: a person signs in, creates an agent under themselves, grants it one project, the agent's action on that project is allowed, the grant is revoked or the agent suspended, and the same action is refused by name, with the audit record naming who made each change (docs/design/identity/STATEMENT-2026-09-22.md:183; docs/design/identity/PROVISIONING-2026-09-22.md:39-46).

The question for Tom: does the grant path land in DIRECTORY-003, in a new row of the directory cluster before DIRECTORY-005, or in the first brief of road step 2? Revision 5 keeps arbitrary grants and live capability enforcement in road step 2 (docs/design/identity/briefs/IDENTITY-001.json:29-30), Chippy's first screen of 17:08 puts the path on step 1's screen (docs/design/identity/STATEMENT-2026-09-22.md:183), and PROVISIONING places the grant in steps 1 and 2 of the road without naming a row (docs/design/identity/PROVISIONING-2026-09-22.md:23-24).

Whichever row the path lands in carries these criteria, drafted here so none is lost. None of them is an acceptance criterion of DIRECTORY-003.

- (a) Allowed before revoke: with the agent active and one grant on project P, the agent's action on P is admitted.
- (b) Refused after revoke: once the grant is revoked, the same action is refused by name at the next check and nothing on P changes.
- (c) Refused after suspend: once the agent is suspended, the same action is refused by name while its grant stays recorded, under whatever suspension semantics Tom settles.
- (d) Every grant, revoke and transition is one signed audit record naming the actor.

A later brief carrying grants, such as the DIRECTORY-006 amendment (docs/design/directory/briefs/DIRECTORY-006.md:37), states its own authority for doing so; this contract records the question as DIRECTORY-003 left it and settles no row for it.

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
