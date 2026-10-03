# The grant contract

What one grant says, how it is encoded, and how it is judged, as crates/lys-identity/src/grants gives it. DIRECTORY-006 R1 defines it; R2 to R4 enforce it.

## Status

The schema follows the lead's C5 rulings for DIRECTORY-006:

1. A grant is a SpiceDB relationship with an expiry caveat and a source relation.
2. Revoking a root deletes everything derived from it: no relationship of the revoked tree remains.
3. A delegation whose requested end is later than its source grant's end is refused by name before anything is committed. It is never clamped.
4. A committed event past that bound is refused by the projector by name, and no relationship is written for it.

The brief asks for independent review of the envelope, canonical encoding, root-authority bootstrap and lineage semantics before a durable grant is signed. That review is recorded as ADR-078 (docs/design/decisions.json, decided 2026-09-27, recorded with DIRECTORY-025), which reads: "This ruling, together with DIRECTORY-006 R1 to R5 as built and accepted on hand/DIRECTORY-006-R5 at e525a91 after a second reader's review, stands as the ratification DIRECTORY-006 R1 and DIRECTORY-006's blocked_by ask for; no separate contract review is waited on." It decides the grant representation with Apollo as second reader, and the root-authority bootstrap: one person fixed when the grants are opened issues every root grant, and no other bootstrap is added. It settles the revision-freshness mechanism of R4 (C25) as built: a check is answered only from a projection that has applied every committed revocation it depends on, and is otherwise refused `StaleDecision`. The grant-representation non-goal stays recorded as open for Tom, and a later ruling by Tom amends this contract. Nothing in this contract changes lys-core or a published lys wire format, and `lys/delegation/v1` is not used as a capability token.

## Envelope

A signed grant event names the application envelope `application/vnd.lys.grant-event.v1+cbor` in its protected header. It is distinct from `lys/identity-event/v1` (`application/vnd.lys.identity-event.v1+cbor`) and from `lys/delegation/v1` (`application/vnd.lys.delegation.v1+cbor`). A message naming either of those is never read as a grant, and a grant is never read as either.

## One grant

A canonical CBOR map (RFC 8949 section 4.2) with integer keys 1 to 12. Every key is required. A key outside 1 to 12 is refused `MemberUnknown`; a missing key is refused `MemberMissing`, naming the member. No member has a default.

| Key | Member | Value |
| --- | --- | --- |
| 1 | id | the grant id, 16 bytes; its text form is `grant-` and 32 hex digits |
| 2 | issuer | identity map: `1` kind (`1` person, `2` agent), `2` id (16 bytes) |
| 3 | holder | identity map, as the issuer |
| 4 | responsible person | the person id responsible for the holder, 16 bytes |
| 5 | resource | map: `1` kind (token), `2` id (token) |
| 6 | relation | the relation it was requested as (token) |
| 7 | actions | the actions the holder may exercise: a non-empty array of tokens, ascending |
| 8 | pass-on | `0` for use-only, or a map: `1` actions (a non-empty subset of key 7), `2` recipient kinds (a non-empty array of `1` person, `2` agent) |
| 9 | source | `0` for a root, or the 16-byte id of the grant it derives from |
| 10 | window | map: `1` starts-at (seconds), `2` ends-at (seconds, or null for no end of its own) |
| 11 | model version | the model version it was judged under, 1 or more |
| 12 | authorising operation | the operation id of the signed event that authorised it, 16 bytes |

A token is one or more bytes of `a-z`, `0-9`, `_`, `-` and `.`, and at most the permission engine's (SpiceDB's) own bound for what it becomes there: an action (a permission) and a relation 64 bytes, a resource kind (an object type) 128 bytes, a resource id (an object id) 1024 bytes.

Exercising and passing on are separate members. Pass-on is stated affirmatively: `0` is written out for use-only, and an absent key 8 is refused, never read as permission. Recipient kinds are a closed set, and an unknown code is refused `RecipientKindUnknown`.

A root grant is held by its responsible person: a root whose holder is not the responsible person is refused `LineageMalformed`, and so is a grant naming itself as its source.

The actions of key 7 are fixed when the grant is issued. A later model version, or an edit of the relation's actions, never widens a grant already held.

## The model

A model is versioned, and resolves each relation to the actions it carries. A relation's name says nothing about its actions. A requested relation lies within an authority only when every action the model resolves it to is an action of that authority. Names are never ranked, and no owner, editor or viewer order is read from a label. A relation the model does not define is refused `RelationUnknown`, and a relation carrying an action outside the authority is refused `ActionsOutside`, naming the actions; both name the model version, and so does every answer that permits.

## Grant events

A grant change is recorded only as one signed grant event, committed as one leaf of the grant log (lys-log-store) before it is answered. The event body is a canonical CBOR map: `1` version (`1`), `2` operation id (16 bytes), `3` caller (identity map), `4` recorded-at (seconds), `5` change kind (`1` issue, `2` revoke, `3` use) and `6` the change: the grant's own map; for a revocation a map of `1` grant id and `2` reason (any length, not blank); for a use a map of `1` grant id and `2` route (`1` browser, `2` api, `3` tool). The signed message is a `COSE_Sign1` (tag 18) whose protected header is `1: -8` (EdDSA), `3:` the grant envelope, `4:` the service's 32-byte Ed25519 key. A message naming another envelope is refused `EnvelopeMismatch`.

The grant book and the permission relationships are both derived from these events and from nothing else. The same operation id with the same request answers the first receipt again; with a different request it is refused `OperationReused`. An append whose outcome is unknown is held and named `OperationUnresolved` until the log is read back; a committed event not yet in the relationships is named `ProjectionPending`. Neither is answered as success, and neither mints a fresh operation.

A receipt carries the event version, operation id, caller, grant, change kind, SHA-256 payload commitment and log coordinate, and `verify_grant_receipt` checks it against the signed message, the service key, a checkpoint and an inclusion proof.

## Relationships

Each issued grant is written as three `SpiceDB` relationships: `grant:<id>#holder@<person|agent>:<id>` with the `unexpired` caveat carrying its end; the source relation `grant:<id>#source@grant:<source>`, or `@person:<responsible>` for a root; and `<resource kind>:<resource id>#<relation>@grant:<id>#holder`. Revoking a grant deletes every relationship of that grant and of every grant derived from it. The store's revision is the number of grant events it reflects, and a decision requires the relationships to stand at or after every change on the authority it rests on. A committed event the book refuses, such as one ending later than its source, is kept as a named refusal and no relationship is written for it.

## Observed use

When a check permits an exercise, the grants append a use event naming the grant, its holder as the caller, the route and the time. A use changes no relationship. The book keeps each grant's latest use and its count of recorded uses, both rebuilt from the log on reopen. A grant with no use event reads as not seen: that no exercise was observed, never that it was never used. A refused check writes no use. A permitted check whose use could not be appended is still answered, and names why in its `use_event`. The grants then hold that exercise as unreported, with its count, latest time, route and reason, and a grant's read answers its use reporting as `missing` rather than `reported`, so a missing report is never read as a count of zero. An unreported exercise is in no log, so it is known only until the grants are reopened, and a later recorded use does not make the count whole.
