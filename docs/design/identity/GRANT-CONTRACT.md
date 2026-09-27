# The grant contract

What one grant says, how it is encoded, and how it is judged, as crates/lys-identity/src/grants gives it. DIRECTORY-006 R1 defines it; R2 to R4 enforce it.

## Status

The schema follows the lead's C5 rulings for DIRECTORY-006:

1. A grant is a SpiceDB relationship with an expiry caveat and a source relation.
2. Revoking a root deletes everything derived from it: no relationship of the revoked tree remains.
3. A delegation whose requested end is later than its source grant's end is refused by name before anything is committed. It is never clamped.
4. A committed event past that bound is refused by the projector by name, and no relationship is written for it.

The brief asks for independent review of the envelope, canonical encoding, root-authority bootstrap and lineage semantics before a durable grant is signed. That review is not recorded here yet; this document is what it reviews. Nothing in this contract changes lys-core or a published lys wire format, and `lys/delegation/v1` is not used as a capability token.

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

A token is 1 to 128 bytes of `a-z`, `0-9`, `_`, `-` and `.`.

Exercising and passing on are separate members. Pass-on is stated affirmatively: `0` is written out for use-only, and an absent key 8 is refused, never read as permission. Recipient kinds are a closed set, and an unknown code is refused `RecipientKindUnknown`.

A root grant is held by its responsible person: a root whose holder is not the responsible person is refused `LineageMalformed`, and so is a grant naming itself as its source.

The actions of key 7 are fixed when the grant is issued. A later model version, or an edit of the relation's actions, never widens a grant already held.

## The model

A model is versioned, and resolves each relation to the actions it carries. A relation's name says nothing about its actions. A requested relation lies within an authority only when every action the model resolves it to is an action of that authority. Names are never ranked, and no owner, editor or viewer order is read from a label. A relation the model does not define is refused `RelationUnknown`, and a relation carrying an action outside the authority is refused `ActionsOutside`, naming the actions; both name the model version, and so does every answer that permits.
