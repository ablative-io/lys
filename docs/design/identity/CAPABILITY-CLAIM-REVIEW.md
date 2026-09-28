# Adversarial review of `lys/agent-capability/v1` (DIRECTORY-031 R2)

This is the review of [docs/design/identity/CAPABILITY-CLAIM.md](CAPABILITY-CLAIM.md)
(path relative to the repository root: `docs/design/identity/CAPABILITY-CLAIM.md`) that
DIRECTORY-031 R2 requires before ratification is asked for. Every attack below is built as
concrete bytes against the draft and names the draft's refusal that defeats it. It changes
no code.

## Parties

- **Draft's author:** the DIRECTORY-031 R1 drafting session of 2026-09-27, whose draft
  reached main in `f6f18d34f6210f60b623f492e28a13d933ff7b18` (committed by Tom Whiting in
  the fold of the brief and card branches).
- **Reviewer:** the DIRECTORY-031 round-1 development session of 2026-09-28 on the branch
  `card/DIRECTORY-031` (Claude Opus 5.5, a separate session from the draft's author, which
  did not write the draft and read it cold).
- **The axis of independence, said plainly:** the reviewer is a different session from the
  author, working from the draft's text, the RFCs and lys-core's source; it is **not** a
  different model or a different organisation. The ratification the draft asks for still
  needs the owning lead and a second reader, recorded on the card; this review does not
  stand in for either.

## The draft reviewed

- **Reviewed at commit:** `737461d` (`737461d docs: HOME-008, HOME-024 and HOME-033 ...`),
  the base commit of this round, at which the draft is byte-identical to its state at
  `f6f18d34f6210f60b623f492e28a13d933ff7b18`.
- **Amended, then every attack repeated:** three attacks found defects in the draft as it
  stood at that commit (entries 20, 22 and 25). The draft was amended (A1 to A3 below) and
  every entry was repeated against the amended draft, whose git blob id is
  `148b7307120ef86fe6bd7428cd073796f4eeab6d`. The amended draft lands in the same commit as
  this review; **the commit that holds the amended draft is recorded here once it exists**,
  and ratification is not asked for until it is.

### Amendments

- **A1 — the issuer-key fingerprint's citation.** The draft called its fingerprint RFC 7093
  section 2 method 1. Method 1 hashes only the `subjectPublicKey` BIT STRING's value; the
  draft's pinned bytes, and rcgen 0.13's `SubjectKeyIdentifier`, hash the whole DER
  SubjectPublicKeyInfo. The bytes are kept (they are what `openssl verify -CAfile` needs to
  find lys-core's issuer certificate); the text now defines the fingerprint by its bytes,
  names it RFC 7093 method 4 truncated to 160 bits, says it is not method 1, and pins the
  method 1 value for the `0x07` key as refused. The decision-log row D7 of
  `docs/design/WIRE-FORMATS.md` carried the same citation and is corrected with it.
- **A2 — the Authority Key Identifier is compared as whole bytes.** Step 2 compares the
  whole extension value with `30 16 80 14` followed by the signing key's fingerprint, byte
  for byte, and names the refusal when `decode_extension` refuses a duplicated extension
  (`issuer_key_mismatch` for the identifier, `claim_malformed` for the claim).
- **A3 — refusal precedence.** A canonical map whose key 1 is a text string other than
  `lys/agent-capability/v1` is `claim_version_unknown` whatever its other members are.

## How the bytes were built

Every hex string below was built by a stand-alone Python program: DER written by hand, and
Ed25519 keys and signatures by a pure-Python RFC 8032 implementation (no lys code, no
rcgen, no cargo run). Its public key for the `0x07` seed equals the draft's pinned
`ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c`, and its fingerprints for
the `0x07` and `0x09` keys equal the brief's pinned Authority Key Identifier values; its
signatures were checked only by the same program, so their independence is of authorship,
not of platform. Signatures are deterministic, so the bytes are reproducible.

Common to the certificates: version 3, serial 1, signature algorithm Ed25519, issuer DN
`CN=lys test anchor`, subject public key the Ed25519 key of the 32-byte seed of `0x21`
bytes, notBefore 1800000000 (`270115080000Z`) and notAfter 1800086400 (`270116080000Z`),
signed by the `0x07` test anchor unless the entry says otherwise, and carrying the
Authority Key Identifier `30168014324be2dea8bc44461b0233e51fa48902ed6b1cc6` and the fixture
claim unless the entry says otherwise. The verifier's set is `{0x07}` unless the entry says
otherwise, the instant is 1800000060 (notBefore plus 60 seconds) and the grant acted under
is `grant-01`.

The fixture claim (84 bytes), from the draft:
```
a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382a301686772616e742d30
31021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f00
```

## Entries

### 1. Forgery

**Attack.** The first certificate's exact contents (subject `agent-01`, the fixture claim under `1.3.6.1.4.1.66364.2.1`, the Authority Key Identifier naming the `0x07` test anchor) signed instead by the `0x0b` key, which is not in the verifier's set `{0x07}`. The attacker copies the honest Authority Key Identifier to point at a trusted key.

**Bytes — certificate DER, signed by the `0x0b` key:**

```
3082016530820117a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba38188308185301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc63062060a2b
0601040184863c02010454a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d303103
82a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f0030
0506032b6570034100e63e82c9da4430e1452e34bbdbb73a3308a075b2ed28ea8751f19b6b3d99a844588ad312ca9b52
0453a1c253399305eddf0491930cd8b5e9b6dd8aeffeb9f201
```

**Refusal that defeats it:** `certificate_chain_invalid`

**Why.** Step 1 finds the signing key by running lys-core's `verify_certificate_chain_at` under each key of the set at the certificate's own notBefore; the Ed25519 `verify_strict` fails under `0x07`, no key accepts, and the verifier refuses before either extension is read. The Authority Key Identifier is never consulted to choose a key, so naming a trusted key in it buys nothing.

**Outcome:** `defeated`

### 2. Operator JSON under 1.3.6.1.4.1.66364.2.1

**Attack.** The operator JSON `{"role":"admin"}`, as `lys ca issue --claims` embeds it, placed as the whole value under `.2.1` of a certificate the trusted issuer signed (the attacker is an issuer-side operator using the `.1` tool's payload habits).

**Bytes — extension value:**

```
7b22726f6c65223a2261646d696e227d
```

**Bytes — certificate DER:**

```
3082011e3081d1a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e6368
6f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c0861
67656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b420
1d9d0ba3433041301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc6301e060a2b060104
0184863c020104107b22726f6c65223a2261646d696e227d300506032b6570034100dc7d882b387e888ca93d73128cab
e560cb4c6cfa0389ed784013db1692aa7fb5776fffbf2ce9f6b9df92290e102cc17bd74c1cd8641958069ae0249e770d
e50c
```

**Refusal that defeats it:** `claim_malformed`

**Why.** `0x7b` is CBOR major type 3 (text string) with length 27, but only 15 bytes follow: the input is not one well-formed CBOR item. Even had it parsed, a text string is not the required map. No byte of JSON is a map head `0xa3`.

**Outcome:** `defeated`

### 3. A v1 claim only under .1

**Attack.** The fixture claim's exact bytes placed only under the shipped `.1` extension (`1.3.6.1.4.1.66364.1`), with nothing under `.2.1`, hoping a verifier falls back to `.1`.

**Bytes — certificate DER:**

```
3082016430820116a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba38187308184301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc6306106092b
0601040184863c010454a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382
a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f003005
06032b6570034100faac5bc82f3b08f3d75fe3cdc292202b436686ef6cdb598e4b7a471024c66a2bd3c4f36da5721c88
969129f9f90ff4fdbeff72044216fa4436044bf663438a02
```

**Refusal that defeats it:** `claim_malformed`

**Why.** The verifier reads only `.2.1` and never reads `.1`; the `.2.1` extension is absent, which the draft names `claim_malformed`. Conversely the `.1` readers (`lys ca verify`, `lys inspect cert`) keep showing these bytes as opaque unverified claims, never as a typed claim.

**Outcome:** `defeated`

### 4. lys/delegation/v1 bytes

**Attack.** The complete frozen `lys/delegation/v1` vector A artifact (a tagged `COSE_Sign1`, `crates/lys-core/tests/delegation_vector/frozen_hex.rs`) placed as the value under `.2.1`; and, separately, its bare 68-byte payload map.

**Bytes — extension value (the artifact, 220 bytes):**

```
d284584fa301270378266170706c69636174696f6e2f766e642e6c79732e64656c65676174696f6e2e76312b63626f72
04582003a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8a05844a60101026c6578616d70
6c652e7465737403582029acbae141bccaf0b22e1a94d34d0bc7361e526d0bfe12c89794bc9322966dd70402051b0000
018bcfe568000619012c5840478bf10b8ff704eab09a24efba6728eca75d9924a4cd006b46ba202d9f43fd198e64363a
36ad4433918ab9e96956ead3ecb22da4bca69ef95ab0f9ec782f340f
```

**Bytes — extension value (the payload map alone):**

```
a60101026c6578616d706c652e7465737403582029acbae141bccaf0b22e1a94d34d0bc7361e526d0bfe12c89794bc93
22966dd70402051b0000018bcfe568000619012c
```

**Bytes — certificate DER carrying the artifact:**

```
308201f1308201a3a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba38201133082010f301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc63081eb
060a2b0601040184863c02010481dcd284584fa301270378266170706c69636174696f6e2f766e642e6c79732e64656c
65676174696f6e2e76312b63626f7204582003a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc86641255
31b8a05844a60101026c6578616d706c652e7465737403582029acbae141bccaf0b22e1a94d34d0bc7361e526d0bfe12
c89794bc9322966dd70402051b0000018bcfe568000619012c5840478bf10b8ff704eab09a24efba6728eca75d9924a4
cd006b46ba202d9f43fd198e64363a36ad4433918ab9e96956ead3ecb22da4bca69ef95ab0f9ec782f340f300506032b
6570034100a594daa709253edf86f5a6383ae8a668a17d4d5a7663db3b37295feb4915e2f3132f252bb839abcf962ca9
84a0e83a0734e6990ada52314e251523afc62c6d0b
```

**Refusal that defeats it:** `claim_malformed`

**Why.** The artifact begins `d2`, CBOR tag 18: not a map. The payload map has six entries and its key 1 is the unsigned integer `1`, not a text string, so it is not a canonical map whose key 1 is text (which would have been `claim_version_unknown`) but a map with a wrong-typed member and unknown keys 4 to 6. Both are refused; neither is ever read as a capability.

**Outcome:** `defeated`

### 5. lys/attestation/v2 payload bytes

**Attack.** The golden `lys/attestation/v2` payload map (`crates/lys-core/src/attestation/encoding_tests.rs`, inside `GOLDEN_ARTIFACT_HEX`) placed as the value under `.2.1`.

**Bytes — extension value (46 bytes):**

```
a20158209404f8b8cec8ad98a88b106d9345d518c273f012c1306b8af5103e865997191e021b0000018bcfe56800
```

**Bytes — certificate DER:**

```
3082013c3081efa003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e6368
6f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c0861
67656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b420
1d9d0ba361305f301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc6303c060a2b060104
0184863c0201042ea20158209404f8b8cec8ad98a88b106d9345d518c273f012c1306b8af5103e865997191e021b0000
018bcfe56800300506032b65700341002a81e21016c6c5345f6004da6a5ea26e88bed8022dfc9d4bc67dbac6a2b9e0a1
5628d74e83588d1c54763097a088f395a0c113a3d04fc45970826e0829da770d
```

**Refusal that defeats it:** `claim_malformed`

**Why.** A two-entry map whose key 1 is a 32-byte byte string (`58 20`), not a text string, whose key 2 is an integer, not a text string, and which lacks key 3. The content-type slot of every lys COSE artifact lives in its protected header, never in its payload's key 1, so no lys payload can present `lys/agent-capability/v1` at key 1 by accident.

**Outcome:** `defeated`

### 6. Malleability by integer form

**Attack.** The fixture claim with grant-01's not-before 1800000000 written in the 9-byte form `1b000000006b49d200` in place of `1a6b49d200`: the same value, different bytes.

**Bytes — claim bytes (88 bytes):**

```
a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382a301686772616e742d30
31021b000000006b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f00
```

**Refusal that defeats it:** `claim_malformed`

**Why.** RFC 8949 section 4.2.1 requires the shortest form; the draft refuses any integer not in shortest form. The certificate hash (the revocation fold's name for it) would differ, but no second encoding of the same claim is ever accepted.

**Outcome:** `defeated`

### 7. Malleability by grant order

**Attack.** The fixture claim with its two grants in the order grant-02, grant-01.

**Bytes — claim bytes (84 bytes):**

```
a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382a301686772616e742d30
32021a6b49d200031a6b715f00a301686772616e742d3031021a6b49d200031a6b4b2380
```

**Refusal that defeats it:** `claim_malformed`

**Why.** The grants array must be ascending by the grant id's UTF-8 bytes; `grant-02` > `grant-01`.

**Outcome:** `defeated`

### 8. Malleability by a grant listed twice

**Attack.** The fixture claim with grant-01's entry listed twice in place of grant-02.

**Bytes — claim bytes (84 bytes):**

```
a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382a301686772616e742d30
31021a6b49d200031a6b4b2380a301686772616e742d3031021a6b49d200031a6b4b2380
```

**Refusal that defeats it:** `claim_malformed`

**Why.** No grant id twice; strictly ascending order also excludes equal neighbours.

**Outcome:** `defeated`

### 9. Malleability by a null end

**Attack.** The open-ended claim (grant-01 with not-before 1800000000 and no end) re-encoded with key 3 present as CBOR `null` (`f6`).

**Bytes — claim bytes (57 bytes):**

```
a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310381a301686772616e742d30
31021a6b49d20003f6
```

**Refusal that defeats it:** `claim_malformed`

**Why.** Key 3 is absent, never null, for a grant with no end; present as anything but an unsigned integer is refused, so the open-ended grant has exactly one encoding.

**Outcome:** `defeated`

### 10. Transposition

**Attack.** The holder id and a grant id swapped: a claim naming holder `grant-01` and listing one grant `agent-01` (not-before 1800000000, no end), in a certificate whose subject is `agent-01`, the trusted issuer having been induced to sign it. The type-level transposition (key 2 given the array, key 3 given the text) is the second byte string.

**Bytes — claim bytes (55 bytes):**

```
a301776c79732f6167656e742d6361706162696c6974792f763102686772616e742d30310381a201686167656e742d30
31021a6b49d200
```

**Bytes — certificate DER:**

```
308201453081f8a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e6368
6f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c0861
67656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b420
1d9d0ba36a3068301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc63045060a2b060104
0184863c02010437a301776c79732f6167656e742d6361706162696c6974792f763102686772616e742d30310381a201
686167656e742d3031021a6b49d200300506032b657003410085f7cf0611acf0fb5b2624cfa75e84e8397fb00f1bc27a
70953d9d124a9404c90d0140a0a956c2d55f4766415dbf3be887bcf5465952ce9d233bd17f5ca78401
```

**Bytes — key 2 and key 3 swapped in type:**

```
a301776c79732f6167656e742d6361706162696c6974792f7631028003686167656e742d3031
```

**Refusal that defeats it:** `claim_holder_mismatch`

**Why.** Step 4 compares the holder with the subject's common name, `grant-01` against `agent-01`, and refuses. The type swap is refused as `claim_malformed` (key 2 must be text, key 3 an array). Directory ids are 16 random bytes (`crates/lys-identity/src/id.rs`), so an agent id equal to a grant id does not occur, and holder and grant ids sit under different integer keys at different depths.

**Outcome:** `defeated`

### 11. Authority Key Identifier naming another trusted key

**Attack.** A certificate signed by the `0x07` test anchor whose Authority Key Identifier names the second trusted issuer key (`0x09`, fingerprint `3dba97edb866520d36063a0d9f79576893f3b130`), verified against the set `{0x07, 0x09}`.

**Bytes — extension value:**

```
301680143dba97edb866520d36063a0d9f79576893f3b130
```

**Bytes — certificate DER:**

```
3082016530820117a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba38188308185301f0603551d230418301680143dba97edb866520d36063a0d9f79576893f3b1303062060a2b
0601040184863c02010454a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d303103
82a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f0030
0506032b65700341009da3a0196619e0b630dd96e7cb977862db0044a5426add24272a05b02b0b23c84e651290b6c4f3
e074e9732ac3dff296b3a734ba59df2da50e5f8c2088665a09
```

**Refusal that defeats it:** `issuer_key_mismatch`

**Why.** Step 1 finds the signing key by signature, `0x07`; step 2 compares the whole Authority Key Identifier value with `30168014324be2dea8bc44461b0233e51fa48902ed6b1cc6` and refuses, naming both fingerprints. The identifier can never redirect which key is believed to have signed.

**Outcome:** `defeated`

### 12. Authority Key Identifier absent

**Attack.** The first certificate with no extension under `2.5.29.35`.

**Bytes — certificate DER:**

```
308201413081f4a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e6368
6f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c0861
67656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b420
1d9d0ba36630643062060a2b0601040184863c02010454a301776c79732f6167656e742d6361706162696c6974792f76
3102686167656e742d30310382a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d303202
1a6b49d200031a6b715f00300506032b65700341004e17f9d03ce61649343ab9a92a98858c0f4382b46faecfb19371ff
ec57c9c5139f62d6df038d9b4f3bd498eaadc90400a4f807bfe33578fcf96cd56313e4b301
```

**Refusal that defeats it:** `issuer_key_mismatch`

**Why.** Step 2 refuses an absent identifier, naming the signing key's fingerprint and `absent`.

**Outcome:** `defeated`

### 13. Authority Key Identifier of 32 bytes

**Attack.** The first certificate whose Authority Key Identifier carries the full 32-byte SHA-256 of the signing key's SubjectPublicKeyInfo (the right key, the wrong length).

**Bytes — extension value (36 bytes):**

```
30228020324be2dea8bc44461b0233e51fa48902ed6b1cc671e7739af2551e0bfe68f54e
```

**Bytes — certificate DER:**

```
3082017130820123a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba38194308191302b0603551d23042430228020324be2dea8bc44461b0233e51fa48902ed6b1cc671e7739af2
551e0bfe68f54e3062060a2b0601040184863c02010454a301776c79732f6167656e742d6361706162696c6974792f76
3102686167656e742d30310382a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d303202
1a6b49d200031a6b715f00300506032b6570034100089422f87bca429e2953b2a0870593dd6bd392304da7c0675cacc8
8df50a8941b3476e26d214657164fca0adc5e4e6602de50a11d4791656803083f736237500
```

**Refusal that defeats it:** `issuer_key_mismatch`

**Why.** Only a keyIdentifier of exactly 20 bytes in exactly the 24-byte form is accepted; a prefix match is not a match, because the comparison is of the whole value.

**Outcome:** `defeated`

### 14. Downgrade to v0

**Attack.** The fixture claim with its content type `lys/agent-capability/v0`.

**Bytes — claim bytes (84 bytes):**

```
a301776c79732f6167656e742d6361706162696c6974792f763002686167656e742d30310382a301686772616e742d30
31021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f00
```

**Refusal that defeats it:** `claim_version_unknown`

**Why.** Key 1 is a text string other than `lys/agent-capability/v1`; the refusal names `lys/agent-capability/v0`. No version other than v1 is read by a v1 verifier, older or newer.

**Outcome:** `defeated`

### 15. Downgrade by absent key 1

**Attack.** The fixture claim with key 1 removed (a two-entry map of holder and grants), hoping a verifier assumes a default version.

**Bytes — claim bytes (59 bytes):**

```
a202686167656e742d30310382a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d303202
1a6b49d200031a6b715f00
```

**Refusal that defeats it:** `claim_malformed`

**Why.** A missing member is refused and no member is filled by default.

**Outcome:** `defeated`

### 16. Replay under another agent's certificate

**Attack.** The fixture claim (holder `agent-01`), a claim the issuer really did sign once, carried in a certificate whose subject is `agent-02`.

**Bytes — certificate DER, subject agent-02:**

```
3082016530820117a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3032302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba38188308185301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc63062060a2b
0601040184863c02010454a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d303103
82a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f0030
0506032b65700341005fa16c0c6f41536e18c51d92852fcbec0029ab1b952784c28b704e2ffe392b9a1b6dfb8854be02
1834934bd89569b689f35d1b42367637f4d36cc652d70f5607
```

**Refusal that defeats it:** `claim_holder_mismatch`

**Why.** The claim is bound to its certificate by the issuer's signature over the whole TBS, and inside it the holder must equal the subject's common name; `agent-01` against `agent-02` is refused, naming both.

**Outcome:** `defeated`

### 17. Replay for an unlisted grant

**Attack.** The first certificate presented with the grant acted under `grant-03`, which the claim does not list.

**Bytes — certificate DER (the first certificate):**

```
3082016530820117a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba38188308185301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc63062060a2b
0601040184863c02010454a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d303103
82a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f0030
0506032b65700341006ff611ad3b9ed6e6a1a08f69ef30b5cc53897f5143d35816ac2d00d5c2dcf0400c27554195461e
acac19f7e4f6c4205def7587a28aac4ad1004f19ceeca56203
```

**Bytes — grant acted under, UTF-8:**

```
6772616e742d3033
```

**Refusal that defeats it:** `claim_grant_mismatch`

**Why.** Step 5 refuses a grant not among the listed ids, naming `grant-03`, `grant-01` and `grant-02`. A claim listing no grant refuses every grant acted under.

**Outcome:** `defeated`

### 18. Expiry one second past the certificate's notAfter

**Attack.** The first certificate (notBefore 1800000000, notAfter 1800086400) verified at the instant 1800086401.

**Bytes — certificate DER (the first certificate):**

```
3082016530820117a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba38188308185301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc63062060a2b
0601040184863c02010454a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d303103
82a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f0030
0506032b65700341006ff611ad3b9ed6e6a1a08f69ef30b5cc53897f5143d35816ac2d00d5c2dcf0400c27554195461e
acac19f7e4f6c4205def7587a28aac4ad1004f19ceeca56203
```

**Bytes — instant, seconds since the Unix epoch, as a CBOR unsigned integer:**

```
1a6b4b2381
```

**Refusal that defeats it:** `certificate_chain_invalid`

**Why.** Step 6 runs `verify_certificate_chain_at` under the signing key at the given instant; `at > notAfter` returns lys-core's `certificate expired` reason. The claim has no window of its own to disagree with the certificate's, and step 1's search at the certificate's own notBefore never substitutes for this check, because step 6 always runs last and always at the caller's instant.

**Outcome:** `defeated`

### 19. Timing oracle

Each comparison `verify_agent_capability` makes, and the signature check, taken in turn.
The question for each is whether its running time can tell an attacker something secret.

- **Authority Key Identifier against the signing key's fingerprint.** Both operands are
  public: the Authority Key Identifier is read from the certificate, which the presenter
  holds, and the fingerprint is the SHA-256 of a public key in the verifier's trusted set,
  which a verifier publishes by design (a stranger must be able to verify). An early-exit
  byte comparison can only reveal how many leading bytes of a public value match another
  public value. No secret to time.
- **Holder against subject.** Both operands are public: the holder is read from the claim
  and the subject's common name from the same certificate, both bytes the presenter
  supplied. No secret to time.
- **Grant acted under against the listed grant ids.** Both operands are public: the grant
  acted under is the caller's own input, and the listed ids are in the certificate. Grant
  ids are directory identifiers, not bearer secrets; knowing one grants nothing without
  the access check. No secret to time.
- **Instant against the certificate's own window.** Both operands are public: the instant
  is the caller's input and the window is the certificate's notBefore and notAfter. No
  secret to time.
- **The signature check** (lys-core's unchanged `verify_certificate_chain_at`, Ed25519
  `verify_strict`, run under each key of the set). Every operand is public: the
  certificate's TBS bytes and signature, and the trusted public keys. Signature
  verification involves no private key, so there is nothing secret for its timing to leak.
  What the timing of the search over the set can reveal — which key of the set, if any,
  verified, and the size of the set — is itself public: the set is published, and the
  Authority Key Identifier names the signer. No private key material is ever in the
  verifier's hands.

**Outcome:** `defeated`

## Further entries built by the reviewer

### 20. RFC 7093 method 1 identifier (the draft's own citation)

**Attack.** An issuer implemented by a stranger from the draft as it stood at the reviewed commit: the draft said the fingerprint was RFC 7093 section 2 method 1. Method 1 hashes only the value of the `subjectPublicKey` BIT STRING, the raw 32-byte key, giving `fe812c12f3ab4ce6ac5db69ac352f906cb1b11ef` for the `0x07` key, while the draft's pinned bytes, and rcgen 0.13's `SubjectKeyIdentifier`, hash the whole 44-byte SubjectPublicKeyInfo (`324be2dea8bc44461b0233e51fa48902ed6b1cc6`). Two conforming readings of one draft produced different identifiers for every key: the stranger's certificates would be refused by lys and lys's would be refused by the stranger, and `openssl verify` would stop finding the issuer for the stranger's certificates.

**Bytes — extension value by method 1:**

```
30168014fe812c12f3ab4ce6ac5db69ac352f906cb1b11ef
```

**Bytes — certificate DER carrying it, signed by 0x07:**

```
3082016530820117a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba38188308185301f0603551d23041830168014fe812c12f3ab4ce6ac5db69ac352f906cb1b11ef3062060a2b
0601040184863c02010454a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d303103
82a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f0030
0506032b657003410044b2cc35e40aa8fb165a3a3d74554d8932b8d275457b4dc58818fe1aa78a390d7b0f13ce9c0d72
d2e8a2743e670252f34c0cd73559a06b8435daa4980f2ef80e
```

**Refusal that defeats it:** `issuer_key_mismatch`

**Why.** This stood against the draft at the reviewed commit as a specification defect: a refusal fired, but the draft named the refused value conformant. Amendment A1 makes the bytes the definition (the leftmost 160 bits of the SHA-256 of the whole DER SubjectPublicKeyInfo), states that this is RFC 7093 method 4 truncated as rcgen writes it and not method 1, and pins the method 1 value above as refused. Repeated against the amended draft: the value is refused by name and the draft says so.

**Outcome:** `defeated`

### 21. Authority Key Identifier with further fields

**Attack.** The right 20-byte keyIdentifier followed by an `authorityCertIssuer` (the test anchor's DN) and an `authorityCertSerialNumber` of 1.

**Bytes — extension value:**

```
30398014324be2dea8bc44461b0233e51fa48902ed6b1cc6a11ea41c301a3118301606035504030c0f6c797320746573
7420616e63686f72820101
```

**Refusal that defeats it:** `issuer_key_mismatch`

**Why.** The draft requires an AuthorityKeyIdentifier holding only a keyIdentifier; amendment A2 makes the comparison the whole value byte for byte against `30 16 80 14` and the fingerprint, so extra fields cannot be read two ways by two parsers.

**Outcome:** `defeated`

### 22. Authority Key Identifier in a non-DER form

**Attack.** The right keyIdentifier inside a SEQUENCE whose length is written in long form (`30 81 16`), valid BER, not DER.

**Bytes — extension value (25 bytes):**

```
3081168014324be2dea8bc44461b0233e51fa48902ed6b1cc6
```

**Refusal that defeats it:** `issuer_key_mismatch`

**Why.** Before amendment A2 the draft said 'an AuthorityKeyIdentifier holding only a 20-byte keyIdentifier', which a lenient BER parser reads as satisfied; the draft's own text said DER, so a refusal was required but was not pinned to bytes. A2 compares the whole value byte for byte, and this value is refused.

**Outcome:** `defeated`

### 23. Duplicate extensions

**Attack.** A certificate signed by the trusted issuer carrying the fixture claim twice under `.2.1`, and a second carrying the Authority Key Identifier twice, hoping two parsers pick different copies.

**Bytes — certificate DER, claim twice:**

```
308201c93082017ba003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba381ec3081e9301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc63062060a2b
0601040184863c02010454a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d303103
82a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f0030
62060a2b0601040184863c02010454a301776c79732f6167656e742d6361706162696c6974792f763102686167656e74
2d30310382a301686772616e742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b
715f00300506032b6570034100429683e11e2fd4738adf5433187616afc03b269d4f2a45c4c610272067868be886fdba
ad112c7e7dec4dcf5323c654516c74cf776efde40c08b55eef9b162b06
```

**Bytes — certificate DER, Authority Key Identifier twice:**

```
3082018630820138a003020102020101300506032b6570301a3118301606035504030c0f6c7973207465737420616e63
686f72301e170d3237303131353038303030305a170d3237303131363038303030305a30133111300f06035504030c08
6167656e742d3031302a300506032b6570032100884b8857f4eaa1613c61504db34d4beaf346517a0e31de3cddd4d9b4
201d9d0ba381a93081a6301f0603551d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc6301f060355
1d23041830168014324be2dea8bc44461b0233e51fa48902ed6b1cc63062060a2b0601040184863c02010454a301776c
79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382a301686772616e742d3031021a6b
49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f00300506032b6570034100e91a392122bd
1395927954742579d91cad258fe09cabd6f276ed7a638c40f16c2b8cba7c6440fd372eec8390c805cfb5ce9f6e53cd8b
e9a30af44731fb68f10e
```

**Refusal that defeats it:** `claim_malformed`

**Why.** x509-parser 0.16 accepts duplicate extensions at parse, so step 1 passes; lys-core's unchanged `decode_extension` looks up by `get_extension_unique` and refuses a duplicated OID. Amendment A2 names the refusal for each: the claim twice is `claim_malformed`, the Authority Key Identifier twice is `issuer_key_mismatch` (read at step 2, before the claim).

**Outcome:** `defeated`

### 24. Self-describe tag and non-shortest length

**Attack.** The fixture claim wrapped in the RFC 8949 self-described CBOR tag (`d9d9f7`), and the fixture claim with the content type's length written `78 17` (one-byte length argument) in place of `77`.

**Bytes — tagged:**

```
d9d9f7a301776c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382a301686772616e
742d3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f00
```

**Bytes — long string length:**

```
a30178176c79732f6167656e742d6361706162696c6974792f763102686167656e742d30310382a301686772616e742d
3031021a6b49d200031a6b4b2380a301686772616e742d3032021a6b49d200031a6b715f00
```

**Refusal that defeats it:** `claim_malformed`

**Why.** The top item must be a map, not a tag; lengths, like integers, must be in shortest form under RFC 8949 section 4.2.1.

**Outcome:** `defeated`

### 25. Refusal precedence under a later version

**Attack.** A canonical map whose key 1 is `lys/agent-capability/v2` and which also carries a key 4, as a later version might: before amendment A3 one sentence made it `claim_version_unknown` and another (unknown key) `claim_malformed`, so two conforming verifiers could name the same bytes differently.

**Bytes — claim bytes (40 bytes):**

```
a401776c79732f6167656e742d6361706162696c6974792f763202686167656e742d303103800401
```

**Refusal that defeats it:** `claim_version_unknown`

**Why.** Refused either way, so nothing was ever accepted; the defect was that the refusal name was not determined by the bytes. Amendment A3 states the decoder reads key 1 once the bytes are one canonical map and names any other text version `claim_version_unknown` whatever the other members are; a v1 verifier does not judge a later version's members.

**Outcome:** `defeated`

## Result

Every entry above records the outcome `defeated`: 19 required entries and 6 further
entries, 25 in all. Three of them (20, 22 and 25) found defects in the draft at the reviewed
commit; each was repaired by amendment and repeated against the amended draft. No attack
stands against the amended draft. Ratification may be asked for once the commit holding the
amended draft is recorded above; this review does not ratify the format.
