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

The signature is the directory service's. It attests that the service authenticated the actor the event names, by the method and at the time the event gives. It is never a person's signature. An agent registered by a person records that person. A request an agent signed is recorded under the person responsible for that agent, by the method agent signature, with the agent's id beside the person and never in their place (P8).

The whole message is one leaf of the lys-log-store log. The log coordinate is not in the leaf. The receipt returns it, because the leaf's bytes are fixed before the log gives it a place.

## The body

A canonical CBOR map (RFC 8949 section 4.2: shortest heads, definite lengths, keys ascending, no floats). Every key is an unsigned integer, and every map carries exactly its keys, in order.

| Key | Field | Value |
| --- | --- | --- |
| 1 | version | `1`; service-account bearer events use `2` |
| 2 | operation | the caller's operation id, 16 bytes |
| 3 | actor | map: `1` issuer (text), `2` subject (text), `3` method (`1` OIDC, `2` agent signature, `3` operator or service-account bearer), `4` authenticated-at (seconds since the Unix epoch), `5` the principal id (16 bytes), present for an agent signature or service-account bearer and absent for OIDC or operator; see the closed code-3 rule below |
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

The first-administrator extension (28 September 2026, reviewed by Archie in Cambium post 70165f54c05389a20f1b03770e7e05683a40c548a7f6724cb604c81504e7ccd3) adds kind `7`, setup person, with map `1` profile, naming only a person. Kinds 1 through 6 and their signature-covered bytes remain unchanged. Older readers refuse kind 7; deployment requires a reader that supports it. Before the first live setup leaf is written, every installed log reader, including the server and development seed binary, must support kind 7. Once written, rollback to a reader without kind 7 is not supported; restoring an older executable alone is not recovery.

Setup creates one new active person and binds exactly the event actor's issuer and subject in the same signed leaf. Its payload cannot nominate another login. The HTTP service admits only the configured administrator before calling this operation. It does not issue any resource grant. An existing binding refuses a new setup operation. Repeating the same operation, actor binding and profile returns the original receipt even after a new sign-in or restart; it does not change a subsequently suspended or retired identity. Reusing the operation for a different profile or actor refuses by name.

A profile is the map `1` display name (text).

Reporting registration uses kind `8`, naming an agent. Its map is `1` responsible person (16 bytes), `2` profile, `3` reporting target. A reporting target is the map `1` kind (`1` person, `2` agent), `2` id (16 bytes). Direct person registrations continue using kind `2` with unchanged bytes.

Reporting reassignment uses kind `9`, naming the changed agent. Its map is `1` previous target, `2` new target, `3` previous accountable person (16 bytes), `4` new accountable person (16 bytes). One leaf changes the edge and the affected descendants' accountable person. Each affected record names that leaf in its event provenance. Snapshot version `3` stores each agent's direct edge; the explicit version-2 migration materialises existing person edges without rewriting signed leaves. Readers must support both new event kinds before they are written.

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

## Launch records and withdrawals (DIRECTORY-029)

Two further kinds of signed directory event keep what a start gave and whether its request still stands. They are written by crates/lys-identity/src/start/: launch_record.rs holds both envelopes and the launch record's body, withdrawal.rs the withdrawal's body, and state.rs the log they are kept on.

### Status

Proposed, awaiting the joint review this document records for every kind before its first durable signature. Until the review is recorded here, launch records and withdrawals are signed only inside development isolation and tests. Neither kind changes a byte of `lys/identity-event/v1` or of kinds 1 to 7.

### The envelope

Each is a `COSE_Sign1` message in exactly the envelope above, signed by the directory service's key, with its own content type in the protected header:

| Kind | Content type |
| --- | --- |
| launch record | `application/vnd.lys.launch-record.v1+cbor` |
| withdrawal | `application/vnd.lys.launch-withdrawal.v1+cbor` |

So neither is ever read as the other or as an identity event: a reader takes the content type from the exact protected header it re-derives for the service key, and refuses any other. Each message is one leaf of the launch-record log (origin `lys/identity/launch-records`), kept beside the directory log, whose folded state is sealed in its signed snapshot under `lys/identity/launch-records-state/v1`: every leaf up to the snapshot, as a CBOR array of byte strings, checked at a start to fold to the log's own tree.

### The launch record's body

A canonical CBOR map with keys 1 to 12, every key present:

| Key | Field | Value |
| --- | --- | --- |
| 1 | version | `1`; service-account bearer events use `2` |
| 2 | launch record id | text: `launch-` and 32 lowercase hex digits from the secure random source |
| 3 | agent | text: the enduring agent id |
| 4 | machine | text: the machine's identifier |
| 5 | executable | text: the profile version's recorded executable |
| 6 | arguments | array of text: its recorded arguments, in order |
| 7 | working directory | text: its recorded working directory |
| 8 | profile version | text |
| 9 | credential ids | array of text: the ids the credentials check handed on, never a value |
| 10 | given by | text: the person who gave it |
| 11 | given at | seconds since the Unix epoch |
| 12 | copied from | text: the launch record it was given again from, or null |

### The withdrawal's body

A canonical CBOR map with keys 1 to 4: `1` version (`1`), `2` the launch record id withdrawn, `3` who withdrew it (text), `4` when (seconds since the Unix epoch). A withdrawal says the request no longer stands. It never says the agent did not start.

### Strict reading

A message is refused unless it is the exact canonical message over its three parts, its protected header is the one for its content type and the service key, its signature verifies, and re-encoding the body it names gives back the very bytes that were read. Every failure is the one refusal, whatever it was. A log is refused whole when it keeps a launch record id twice, or withdraws a record it does not keep or withdraws one twice.

### What reads them

A launch record's state is derived and never kept: running when the sessions record (the sessions brief of run d5055cc1, drafted as DIRECTORY-015) holds a verified `lys/session-start/v1` report, signed with the agent's own key, whose agent id and launch record id equal the record's; unconfirmed while no such report exists and no withdrawal names it; withdrawn when a withdrawal names it and no such report exists. The ruling that report rests on: the sessions brief adds the launch record id to `lys/session-start/v1` before its tag freezes, so its signed message holds five things, the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge; this card does not define that message.

## Historical authentication code 3 (29 September 2026)

Code 3 is closed. In the actor map, method 3 with no key 5 means the install operator. Method 3 with key 5 holding exactly 16 bytes means the service-account bearer identified by those bytes. Neither shape is renumbered. Every other key-5 shape is refused; there is no fallback between methods. A future authentication method requires a fresh code.

Operator events retain the v1 body and signed envelope. Service-account bearer events written by build `1b568cd90578f5ed5d7d438e628b23724eef7f12` retain their v2 body and signed envelope. Canonical re-encoding and the signed content type must agree with the decoded method. An operator write is refused while the reversible upgrade intent exists, because the previous binary cannot read that form.
