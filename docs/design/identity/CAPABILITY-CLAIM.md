# `lys/agent-capability/v1` — the agent capability claim (PROPOSED)

**Status: PROPOSED, not ratified.** This is the design round's proposal for a new wire
format, written under DIRECTORY-031 R1. Nothing durable is signed under it until its
ratification is recorded (see [Status](#status)). It is a new format with its own version,
carried **alongside** the shipped certificate extension transport under
`1.3.6.1.4.1.66364.1`, and never a mutation of it.

Why it exists: today an agent's lys certificate can carry only opaque bytes under the
shipped capability-claims extension, which nothing interprets. A signed field nothing
checks is worse than none — it looks checked. This format gives the bytes a schema and
names the verifier that checks them, `verify_agent_capability`.

Every value below that a stranger could need is pinned as bytes. Every path is relative to
the repository root.

## Transport

- The claim is the **whole value of one non-critical X.509 extension** under the proposed
  OID **`1.3.6.1.4.1.66364.2.1`**.
- `1.3.6.1.4.1.66364.2.1` is the **first sub-arc** of `1.3.6.1.4.1.66364.2`. `.2` itself
  carries nothing: it stays the **family arc** for agent-certificate extensions (which
  issuer vouched, runtime identity, session binding), so that each of those extensions keeps
  room under it. The claim is never written under `.2` itself.
- It is written by lys-core's **unchanged** `encode_extension` (which marks it
  non-critical) and read by lys-core's **unchanged** `decode_extension` (which refuses a
  certificate carrying the OID more than once, so at most one claim can be read).
- The shipped `.1` extension (`1.3.6.1.4.1.66364.1`) and `lys ca issue --claims` stay
  **exactly as shipped**, neither deprecated nor removed. Because the typed claim lives only
  under `.2.1`, a claim hand-issued as operator JSON under `.1` can never be taken for a
  typed claim, and a typed claim placed under `.1` is never read as one: the verifier never
  reads `.1`.
- `lys ca verify`, `lys verify --cert` and `lys inspect cert` keep showing only `.1`.
  Showing the typed claim in the lys CLI is its own later card.
- The OID is **proposed for ratification with the rest of the format**; until then
  `docs/PEN-REGISTRATION.md` lists `.2` as `Reserved`.

## Assertion

- **Exactly one claim per certificate.**
- It names its **holder** by the agent's directory id, which equals the common name of the
  certificate's subject.
- It lists **every grant the agent holds at issuance**, each by its directory id with its
  time window **as it stood at issuance**, and nothing else.
- An agent holding no grant at issuance gets a claim whose list is **empty**. That is a
  valid claim: the certificate proves who the agent is, and what it may do is always asked
  of the access check.
- The list is **what held at issuance**. It is never presented as the agent's live grants,
  and a listed grant's window is a **record, not a live check**: whether a grant still
  stands is always the access check's answer, never the list's.
- **No validity window of its own.** The certificate's `notBefore` and `notAfter` govern
  its life.
- **No revocation member.** The revocation fold names a certificate by the SHA-256 of its
  DER, computed by whoever checks it and never from anything the issuer chose, so the fold
  needs no field in the claim.
- **No issuer member.** The certificate names its issuer key in its Authority Key
  Identifier (see [Issuer key identifier](#issuer-key-identifier)).
- **No scope member** (see [Scope](#scope)).

## Encoding

A deterministic CBOR map under **RFC 8949 section 4.2.1** (core deterministic encoding:
shortest-form integers and lengths, definite lengths only, map keys in bytewise
lexicographic order of their encodings). Integer keys:

| Key | Member | Type |
|---|---|---|
| `1` | content type | text string, exactly `lys/agent-capability/v1` |
| `2` | holder id | text string, the agent's directory id |
| `3` | grants | array of grant maps, possibly empty |

Each grant map has integer keys:

| Key | Member | Type |
|---|---|---|
| `1` | grant id | text string, the grant's directory id |
| `2` | not-before | unsigned integer, seconds since the Unix epoch in UTC |
| `3` | not-after | unsigned integer, seconds since the Unix epoch in UTC; **present only when the grant's window has an end** |

Rules, each a refusal when broken:

- The content type is the payload's own version and its **domain separation** from every
  other lys artifact. A canonical map whose key 1 is any other text string is refused as
  `claim_version_unknown`, naming the content type it carries, **whatever its other
  members are**: the decoder reads key 1 once the bytes are one canonical CBOR map, and a
  later version's members are not v1's to judge. Every other defect is `claim_malformed`.
- All three top-level members are required; no other key is allowed at the top or in a
  grant map. Keys 1 and 2 of a grant map are required.
- Key 3 of a grant map is **absent, never null**, when the grant's window has no end (the
  grant contract's window allows an absent end). Key 3 present as anything but an unsigned
  integer — `null` (`f6`) included — is refused.
- When key 3 is present, not-after is **strictly after** not-before.
- The grants array is ordered by the grant id's UTF-8 bytes, **ascending**, with **no grant
  id twice**.
- The input is exactly one CBOR item: **no trailing bytes**.
- Any other defect — not CBOR, not canonical (keys out of order, an integer or length not in
  shortest form, an indefinite length), an unknown key, a missing member, a wrong type,
  grants out of order or duplicated, trailing bytes, not-after at or before not-before — is
  refused as `claim_malformed`, naming the defect. No partial claim is returned and no
  member is filled by default.
- An encoder emits the grants ordered by grant id, omits key 3 for a grant with no end, and
  emits no scope, revocation, issuer or validity-window member.

### Vectors

**Fixture claim** — holder id `agent-01`; grant `grant-01` with not-before 1800000000 and
not-after 1800086400; grant `grant-02` with not-before 1800000000 and not-after
1802592000 (84 bytes):

```
a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f00
```

Annotated:

```
a3                                      map(3)
  01                                    key 1: content type
  77 6c79732f6167656e742d6361706162696c6974792f7631    "lys/agent-capability/v1" (23)
  02                                    key 2: holder id
  68 6167656e742d3031                   "agent-01"
  03                                    key 3: grants
  82                                    array(2)
    a3                                  map(3)
      01 68 6772616e742d3031            1: "grant-01"
      02 1a 6b49d200                    2: 1800000000
      03 1a 6b4b2380                    3: 1800086400
    a3                                  map(3)
      01 68 6772616e742d3032            1: "grant-02"
      02 1a 6b49d200                    2: 1800000000
      03 1a 6b715f00                    3: 1802592000
```

**No grant** — holder id `agent-01`, grants empty (38 bytes):

```
a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310380
```

**Open-ended grant** — holder id `agent-01`; only grant `grant-01` with not-before
1800000000 and no end, so its map has two entries and no key 3 (55 bytes):

```
a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310381a201686772616e742d3031021a6b49d200
```

## Issuer key identifier

- The certificate carries X.509's **Authority Key Identifier** extension, OID
  **`2.5.29.35`**, **non-critical**.
- Its value is the DER of an `AuthorityKeyIdentifier` holding **only** a `keyIdentifier`
  (`[0] IMPLICIT OCTET STRING`) of **exactly 20 bytes** — no `authorityCertIssuer`, no
  `authorityCertSerialNumber`.
- Those 20 bytes are the **issuer-key fingerprint**: the first 20 bytes of the SHA-256 of
  the issuer key's **whole DER `SubjectPublicKeyInfo`** (the 44 bytes for an Ed25519 key,
  algorithm identifier included). This is the keyid rcgen 0.13 writes as the
  `SubjectKeyIdentifier` of lys-core's issuer certificate (`KeyIdMethod::Sha256`, which
  hashes the whole `SubjectPublicKeyInfo` and keeps the leftmost 160 bits).
- **The bytes above are the definition; the RFC is cited only for its family.** This is
  **not** RFC 7093 section 2 method 1, which hashes only the value of the
  `subjectPublicKey` BIT STRING (the raw 32-byte key) and gives a different identifier for
  the same key (see the vector below). It is RFC 7093 section 2 method 4 (the hash of the
  DER encoding of the `SubjectPublicKeyInfo`) with SHA-256, truncated to its leftmost 160
  bits as methods 1 to 3 truncate. An identifier computed by method 1 is refused as
  `issuer_key_mismatch`.
- **The issuer-key fingerprint is new, defined here.** It is **not** lys's existing
  certificate fingerprint, which is the SHA-256 of a certificate's DER. It fingerprints a
  key, not a certificate, and is 20 bytes, not 32. It is the fingerprint by which a
  verifier's set of trusted issuer keys knows each key.
- Because it equals the `SubjectKeyIdentifier` of lys-core's issuer certificate,
  `openssl verify -CAfile` finds the issuer, and a stranger reads the identifier with
  `openssl x509 -text`.
- It is added through lys-core's **existing extensions parameter** (with `encode_extension`
  under `2.5.29.35`), so lys-core's issuance, its leaf parameters and its issuer certificate
  stay unchanged, and rcgen's own Authority Key Identifier is not switched on. The issuer DN
  stays as lys-core writes it.

### Vector

The Ed25519 issuer key derived from the 32-byte seed of `0x07` bytes has public key
`ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c`; its 44-byte DER
`SubjectPublicKeyInfo` is `302a300506032b6570032100` followed by that key. The first 20
bytes of the SHA-256 of that `SubjectPublicKeyInfo` are
`324be2dea8bc44461b0233e51fa48902ed6b1cc6`, so the Authority Key Identifier extension
value is (24 bytes):

```
30168014324be2dea8bc44461b0233e51fa48902ed6b1cc6
```

```
30 16                                   SEQUENCE (22)
  80 14                                 [0] keyIdentifier (20)
    324be2dea8bc44461b0233e51fa48902ed6b1cc6
```

For the same key, RFC 7093 method 1 (SHA-256 of the raw 32-byte key, leftmost 160 bits)
would give `fe812c12f3ab4ce6ac5db69ac352f906cb1b11ef`. That value is **not** this key's
issuer-key fingerprint, and an Authority Key Identifier carrying it is refused.

## Scope

v1 has **no scope member and no scope refusal.** Both wait on the grant representation
(C5), and none is invented here. Adding one is a **new version alongside v1**, never a
mutation of it.

## Verifier check

`verify_agent_capability` in lys-identity takes the certificate's DER, a **set** of trusted
issuer public keys (never exactly one), the instant, and the grant id the caller acts
under. It reads no clock itself. In this order:

1. **Find the signing key**: the key of the set under which lys-core's
   `verify_certificate_chain_at` accepts the certificate at the certificate's own
   `notBefore`. This happens **before any byte of either extension is read**. An empty set,
   or a certificate that verifies under no key of the set, is refused as
   `certificate_chain_invalid`.
2. **Compare the Authority Key Identifier**: read the extension under `2.5.29.35` with
   `decode_extension`; its `keyIdentifier` must equal the signing key's issuer-key
   fingerprint. The comparison is of the **whole extension value, byte for byte**, against
   the 24 bytes `30 16 80 14` followed by that fingerprint, so a BER form, a long-form length
   or any further field of the `AuthorityKeyIdentifier` cannot be read two ways. An absent
   extension, the extension carried more than once (which `decode_extension` refuses), a
   value that is not an `AuthorityKeyIdentifier` holding only a 20-byte `keyIdentifier` in
   exactly that form, or a fingerprint other than the signing key's is refused as
   `issuer_key_mismatch`, naming the signing key's fingerprint and the one named (or
   `absent`).
3. **Parse the claim**: read the extension under `1.3.6.1.4.1.66364.2.1` with
   `decode_extension` and decode it as [Encoding](#encoding) states. An absent extension,
   the extension carried more than once (which `decode_extension` refuses), or one that does
   not parse is refused as `claim_malformed`; a content type it does not know as
   `claim_version_unknown`.
4. **Check the holder** against the common name of the certificate's subject:
   `claim_holder_mismatch`, naming both.
5. **Check the grant acted under** against the listed grant ids: `claim_grant_mismatch`,
   naming the grant acted under and the listed ids. An empty list lists no grant, so every
   grant acted under is refused.
6. **Check the instant** against the certificate's own window with
   `verify_certificate_chain_at` under the signing key: `certificate_chain_invalid`,
   carrying lys-core's reason — `certificate expired` past `notAfter`,
   `certificate not yet valid` before `notBefore` (both boundaries inclusive).

The instant is checked **last**, so every claim refusal fires by name on a certificate
inside its window. On success the claim is returned **as what held at issuance**.

The verifier never reads the extension under `1.3.6.1.4.1.66364.1`, never checks the instant
against a listed grant's window, and never answers whether a grant still stands.

## Rendering consumer

A consumer that shows a claim — first, the agent's file of ADR-008 — shows it **only after
`verify_agent_capability` accepts it**, and shows the list **as held at issuance**, never as
the agent's live grants. What the agent may do now is the access check's answer.

## Revocation

- v1 stops by **expiry of the certificate**.
- Revocation is the DP26 fold of the revocation card zP1P2HLD (DIRECTORY-013), which names
  a certificate by the SHA-256 of its DER and needs **no member of the claim**.
- The directory appends every certificate it issues to the certificate log as that card's
  issuance leaf, so the fold can revoke it.
- The live per-call check belongs to the card QZD2uagw.

## Issuer and anchor

- The directory issues through lys-identity under an issuer key the directory service
  holds.
- A verifier holds a **set** of trusted issuer keys, and the certificate names its signer in
  its Authority Key Identifier, so a rotation of the issuer key needs **no change of
  format**.
- Which anchor that issuer key chains to is **open**.
- Tests use a **test anchor** (the issuer built from the 32-byte seed of `0x07` bytes),
  never shipped in any configuration.

## Status

**Proposed, not ratified.** Sign-off of the brief that carries this draft does not ratify
it, and neither does a green build. Ratification of the format and of its OID
`1.3.6.1.4.1.66364.2.1` is the **owning lead's with a second reader**, recorded on the card
with both names and the reasons, after an adversarial review by a party other than this
draft's author has landed with every attack defeated
(`docs/design/identity/CAPABILITY-CLAIM-REVIEW.md`). The ratification is then written into
the decision log of `docs/design/WIRE-FORMATS.md` (row D7) and the `.2` row of
`docs/PEN-REGISTRATION.md`.
