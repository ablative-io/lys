# lys-anchor — byte-exact wire drafts

> **STATUS: DRAFT. NOTHING HERE IS RATIFIED AND NOTHING HAS BEEN SIGNED UNDER IT.**
>
> The strawman defers to this document in three places ("byte-exact spec to be
> written and ratified per D3"). It exists so the design round ratifies or
> shoots down *text* rather than generating it live. No code implements any of
> it; no artifact exists under any tag named here. Until Tom ratifies, every
> byte below is free to change — which is exactly why it is written down now,
> before the freeze rule bites.
>
> **The freeze rule, restated because it is the whole reason for this file:**
> the moment the anchor signs one receipt under a tag, that tag is permanent.
> A receipt nobody can verify in five years is worse than no receipt, because
> it was believed.

---

## 0. What is being specified, and what is deliberately not

**In scope:** two wire contracts a stranger must parse — the COSE receipt the
anchor issues, and the verification bundle that carries a whole provenance
chain in one file.

**Out of scope here:** the storage layout (tile format, DP10), the write-path
gate (DP9), the key ceremony (DP6). Those are operational and can change
without invalidating a historical artifact. These two cannot.

That split is the useful one: **specify permanently what a stranger parses;
leave operationally free what only the operator touches.**

---

## 1. Receipt — `lys/anchor-receipt/v1`

### 1.1 Naming, and the one-character lesson

- Attestation context tag: `lys/anchor-receipt/v1` (slash form).
- Content type: `application/vnd.lys.receipt.v1+cbor`.

Both follow the shipped convention (`lys/attestation/v2`,
`application/vnd.lys.attestation.v2+cbor`). **There is no hyphen form of this
tag and no KDF anywhere in receipts** — stated explicitly because F2 was a
one-character confusion between the slash context tag and the hyphen HKDF
info of the sealed envelope, and a wrong crypto value in a public reference is
permanent in a way a code bug is not. If a future revision introduces a
derived key here, the hyphen form is reserved for it and MUST be decided in
the same change, not later.

### 1.2 Structure

Tagged `COSE_Sign1` (RFC 9052), CBOR tag 18, array of four:

```
18([ protected: bstr, unprotected: map, payload: nil, signature: bstr ])
```

**Protected header** — exactly four entries, canonical (ascending label) order:

| Label | Value | Why |
|---|---|---|
| `1` (alg) | `-8` (EdDSA) | Deployed practice. RFC 9864 prefers `-19`, but go-cose still ships only `-8`; a receipt no off-the-shelf library verifies is worthless. Same reasoning and same code point as D4. `-19` is a v2 matter with a documented trigger. |
| `3` (content type) | `"application/vnd.lys.receipt.v1+cbor"` | Domain discriminator, signature-covered. Stops cross-protocol confusion with `lys/attestation/v2`, which is the same COSE shape. |
| `4` (kid) | raw 32-byte Ed25519 anchor public key | **Signature-covered, deliberately.** Attestation v1's defect was leaving the signer key outside the signed bytes; v2 fixed it. Receipts inherit the fix rather than rediscovering it. This is also how a verifier knows *which anchor* signed. |
| `395` (vds) | `1` (`RFC9162_SHA256`) | The same RFC 6962 SHA-256 tree lys already implements and conformance-tests. A re-encoding of identical semantics, not a new proof system. |

**Unprotected header** — one entry:

| Label | Value |
|---|---|
| `396` (vdp) | map keyed by proof type: inclusion `-1` → `[tree_size: uint, leaf_index: uint, inclusion_path: [+ bstr]]` |

Consistency proofs (`-2` → `[size_1, size_2, consistency_path]`) are specified
but **not issued at launch** (DP2 recommendation (b)).

**Payload:** `nil` — detached.

### 1.3 What is signed, and why an unprotected proof is safe

The `Sig_structure` is the standard `["Signature1", protected, h'', payload]`
with empty `external_aad`, where **`payload` is the anchor's 32-byte Merkle
root at `tree_size`** — the value the verifier *recomputes* rather than reads.

The proof living in the *unprotected* header looks alarming and is not. The
verifier's algorithm is:

1. Compute the leaf hash: `SHA-256(0x00 ‖ leaf-bytes)` (RFC 6962 raw path —
   the same one every `lys log` artifact uses).
2. Walk `inclusion_path` from that leaf hash to a candidate root, using
   `leaf_index` and `tree_size`.
3. Verify the signature over `Sig_structure` with that candidate root as the
   detached payload.

A tampered proof yields a different candidate root, and the signature fails.
**The proof is authenticated by consequence rather than by coverage** — it
cannot be altered without producing bytes the anchor never signed. This is why
RFC 9942 puts it there, and stating the argument is worth more than asserting
the safety.

In words, the anchor's signature asserts: *"the leaf that hashes to this
path's base was included at index N in my tree of size S, whose root I
vouch for."*

### 1.4 Verifier discipline

Same as the shipped attestation verifier, for the same reasons:

- **Canonical-encoding-strict.** Re-encode the parsed structure and require
  byte-identity with the input. This rejects valid-signature mutants that
  vanilla go-cose, coset and pycose accept — the differentiator D4 already
  bought, extended rather than abandoned.
- **`verify_strict` Ed25519**, rejecting malleable signatures and
  small-order/torsion keys. Non-repudiation requires a unique valid signature.
- **Non-oracle failure.** Every rejected check collapses to one
  indistinguishable message. A receipt verifier is exactly the network-exposed
  surface where a distinguishable error becomes a parsing oracle.
- **The leaf being proven is the child's checkpoint bytes**, which are
  self-describing (origin, size, root). So verifying the receipt
  simultaneously establishes *which* child root was notarized — no separate
  binding needed.

### 1.5 Open sub-question for the round

The anchor's own origin string does not appear in the receipt; the anchor is
identified only by its key in label `4`. Options: leave it (keys are the
identity, and the checkpoint being proven already carries the child's origin),
or add the anchor origin as a fifth protected entry for human diagnosability.
**Lean: leave it.** A second name for the same thing is a second thing to
disagree with itself, and D1 made origins mandatory precisely because
collisions are a security defect — adding an unvalidated origin field here
reintroduces that surface for convenience.

---

## 2. Verification bundle — `lys/verification-bundle/v1`

### 2.1 What it is, and what it is not

The bundle is the artifact a stranger actually receives. It carries frozen
artifacts **verbatim** and adds nothing cryptographic: **it is packaging, and
it must never become a trust statement.** Every security property comes from
the artifacts inside; the container's only job is to not lose or reorder them.

Consequence, stated so it cannot be quietly violated later: **the bundle is
never signed.** A signature over the container would invite verifiers to check
the wrapper and skip the contents — the exact failure mode where something
reports success having verified nothing.

### 2.2 Shape

JSON, in the D2 self-contained-file spirit (a `lys log` proof artifact is
already a self-contained JSON file a 15-line script can check):

```json
{
  "format": "lys/verification-bundle/v1",
  "leaf": "<base64 standard, padded — the leaf bytes verbatim>",
  "inclusion_proof": { "...D2 inclusion artifact, embedded verbatim..." },
  "receipts": [
    "<base64 of tagged COSE_Sign1 receipt bytes>"
  ],
  "counter_anchor": null
}
```

- **`format`** — first field, mandatory, exact string match. A bundle whose
  format string is unrecognised is rejected before any other field is read.
- **`leaf`** — base64 rather than a nested string, because leaf bytes are
  arbitrary binary and JSON string escaping is not a byte-preserving channel.
- **`inclusion_proof`** — the existing D2 artifact embedded as-is, not
  re-encoded. Re-encoding a frozen artifact inside a container is how
  byte-identity gets lost.
- **`receipts`** — **ordered, ascending the chain**: index 0 is the receipt
  over the child checkpoint, index 1 the parent's receipt over *that* anchor's
  checkpoint, and so on. Order is load-bearing and MUST be validated, not
  assumed: each receipt's proven leaf must be the previous link's checkpoint
  bytes. An unvalidated chain of individually-valid receipts proves nothing
  about their relationship.
- **`counter_anchor`** — the OpenTimestamps-style slot, `null` at launch.
  **Present in v1 from day one even while unused**, because adding a field to
  a frozen container means a v2, and DP11 already commits to designing for it
  now.

### 2.3 Verification order

1. Reject unrecognised `format`.
2. Verify the D2 inclusion proof against its embedded checkpoint (existing
   `lys log verify inclusion` path — unchanged).
3. For each receipt in order: verify it, then require its proven leaf bytes to
   equal the previous link's checkpoint bytes. **Failure to check this link is
   the bundle's one interesting vulnerability** — it is what turns a pile of
   valid receipts into an actual chain.
4. If `counter_anchor` is present, verify it independently.

Every step uses artifacts that verify standalone. The bundle adds no step that
requires trusting the bundle.

---

## 3. What this draft still needs from the round

1. **Ratify or reject the two tag names.** They freeze on first signature.
2. **Ratify `-8` over `-19`**, with the migration trigger written down (DP2 —
   the strawman's reasoning stands, but the choice should be minuted).
3. **Decide the anchor-origin sub-question** (§1.5).
4. **Confirm the bundle is never signed** (§2.1). This is a posture, and
   posture drifts unless recorded.
5. **Confirm the receipt-chain link check is mandatory**, not advisory (§2.3
   step 3).

Nothing here is buildable until those five are settled, and all five are
cheap to settle because the text exists.
