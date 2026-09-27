# Identity events, `lys/identity-event/v1`

One signed committed directory event is both the identity change and its audit record (P4). This document is the envelope's exact encoding. The code that writes it is crates/lys-identity: src/event.rs holds the typed event and its wire codes, src/encoding.rs the canonical body and its commitment, and src/signer.rs the signed envelope.

## Status

Reviewed jointly as R2 asks, and accepted. The review is recorded below. Durable bytes may be signed under this envelope from the commit that records it.

## The envelope

An event is a `COSE_Sign1` message (RFC 9052, tag 18) in the shape lys-core's attestation v2 uses.

| Part | Value |
| --- | --- |
| Protected header | A canonical map: `1` (alg) is `-8`, EdDSA. `3` (content type) is `application/vnd.lys.identity-event.v1+cbor`. `4` (kid) is the directory service's 32-byte Ed25519 public key. |
| Unprotected header | The empty map. |
| Payload | The event body below. |
| Signature | 64 bytes of Ed25519 over `["Signature1", protected, h'', payload]`. |

The signature is the directory service's. It attests that the service authenticated the actor the event names, by the method and at the time the event gives. It is never a person's signature, and an agent registered by a person records that person, never an agent signature (P8).

The whole message is one leaf of the lys-log-store log. The log coordinate is not in the leaf. The receipt returns it, because the leaf's bytes are fixed before the log gives it a place.

## The body

A canonical CBOR map (RFC 8949 section 4.2: shortest heads, definite lengths, keys ascending, no floats). Every key is an unsigned integer, and every map carries exactly its keys, in order.

| Key | Field | Value |
| --- | --- | --- |
| 1 | version | `1` |
| 2 | operation | the caller's operation id, 16 bytes |
| 3 | actor | map: `1` issuer (text), `2` subject (text), `3` method (`1` OIDC), `4` authenticated-at (seconds since the Unix epoch) |
| 4 | identity | map: `1` kind (`1` person, `2` agent), `2` id (16 bytes) |
| 5 | recorded-at | seconds since the Unix epoch, by the service's clock |
| 6 | change kind | a code from the table below |
| 7 | change | the change's map, below |

| Kind | Change | Its map | Identity it may name |
| --- | --- | --- | --- |
| 1 | register person | `1` profile | a person |
| 2 | register agent | `1` responsible person (16 bytes), `2` profile | an agent |
| 3 | change profile | `1` profile | either |
| 4 | bind login | `1` issuer (text), `2` subject (text) | a person |
| 5 | lifecycle transition | `1` transition, `2` from, `3` to, `4` reason (text) | either |
| 6 | link audit accepted | `1` source operation id (text), `2` change (`1` linked, `2` unlinked), `3` issuer, `4` subject, `5` observer (text), `6` observed-at (seconds) | a person |

A profile is the map `1` display name (text).

Lifecycle states are `1` registered, `2` active, `3` suspended, `4` retired. Transitions are `1` activate, `2` suspend, `3` reinstate, `4` retire. A transition event is refused unless its `to` is the state the table gives from its `from`: activate is registered to active, suspend is active to suspended, reinstate is suspended to active, and retire is active or suspended to retired. Suspend and retire name a reason.

A link-audit event records the issuer's observation as the receiver accepted it, under the source's own operation id. It is an observation, kept apart from any claim a person made. Its fields follow R4 as written. Row 01's typed link-audit contract must equal this payload: a wire format is kept for ever, so the contract follows the wire and never the other way round.

## Strict reading

A message is refused unless it decodes to exactly this shape, the event it names passes the same checks as an event being written, and re-encoding that event gives back the very bytes that were read. So each event has one byte string. A padded, reordered or long-form message never verifies. A message over 65536 bytes is refused before it is parsed. A message naming a key other than the service key it is checked against is refused as a signer mismatch. A bad signature is refused with no further detail.

## The two hashes

Two SHA-256 hashes touch an event, and they are named apart. Neither is a BLAKE3 content address.

| Hash | Over | Used as |
| --- | --- | --- |
| SHA-256, named `sha-256` | the body bytes | the payload commitment a receipt carries (P7) |
| SHA-256, RFC 6962 leaf hash | the whole signed message | the log's leaf, and what inclusion is proved for |

## Review

Reviewed jointly by Archie and Buckley on DIRECTORY-003 at 6590854, and accepted. The rulings:

1. lys-identity keeps its own canonical head writer, and lys-core does not change. A test pins the writer to RFC 8949 Appendix A's head vectors at every width boundary (0, 23, 24, 255, 256, 65535, 65536, 4294967295 and 4294967296), so a drift from the shortest form fails on its own line, not only through a round trip that could be wrong both ways.
2. The actor is the login binding plus its provenance. The step-1 administrator is configured by issuer and subject. A reader resolves a binding to a person through the directory when it reads, never when the event is written.
3. The link-audit payload is taken from R4 now, and row 01's typed contract must equal it.
