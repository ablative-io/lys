# lys-anchor — byte-exact wire drafts

> **STATUS: DRAFT. NOTHING HERE IS RATIFIED AND NOTHING HAS BEEN SIGNED UNDER IT.**
>
> The strawman defers to this document in three places ("byte-exact spec to be
> written and ratified per D3"). It exists so the design round ratifies or
> shoots down *text* rather than generating it live. No code implements any of
> it; no artifact exists under any tag named here. Every byte below is still
> free to change — which is exactly why it is written down now, before the
> freeze rule bites.
>
> **Update 2026-07-29:** the design decisions governing this file are now settled
> by derivation in [DECISIONS.md](DECISIONS.md) — receipts alongside JSON proofs,
> whole certificates in the issuance log, an unsigned bundle with mandatory
> chain-link checks, domain-scoped origins, and no timestamp inside receipts.
> This document stays DRAFT regardless: a format is frozen by the first durable
> artifact signed under it, not by a decision to adopt it, and no such artifact
> exists.
>
> **The freeze rule, restated because it is the whole reason for this file:**
> the moment the anchor signs one receipt under a tag, that tag is permanent.
> A receipt nobody can verify in five years is worse than no receipt, because
> it was believed.
>
> **Update 2026-07-30:** §1 is now **implemented** in `lys-core::receipt`, and
> building it corrected this document twice — the vdp's RFC 9942 wrappers
> (§1.2) and the size-1 conformance gap (§1.2.1) — plus surfaced one documented
> limitation (§1.3.1). Interop is earned rather than asserted: the vendored
> `veraison/go-cose` gate verifies lys receipts across every leaf of every tree
> size 2..=17, deriving the detached root from RFC 6962's *recursive*
> definitions while lys uses an *iterative* walk, and the two implementations
> produce byte-identical receipts.
>
> The document nonetheless **stays DRAFT**: code that could sign freezes
> nothing. Only a durable artifact signed under the tag does, and none exists —
> tests sign, nothing else may, and §2 (the bundle) is still unimplemented.

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
| `396` (vdp) | map keyed by proof type: inclusion `-1` → **array of** proofs, each an RFC 9942 `bstr .cbor [tree_size: uint, leaf_index: uint, inclusion_path: [+ bstr]]` |

> **Correction, 2026-07-30 (implementation).** An earlier revision of this table
> wrote the value at `-1` as a bare `[tree_size, leaf_index, inclusion_path]`
> array. That is **wrong**, and it is the kind of wrong that only shows up in
> someone else's parser. RFC 9942's CDDL nests the proof twice:
>
> ```
> verifiable-proofs = { &(inclusion-proof: -1) => inclusion-proofs }
> inclusion-proofs  = [ + inclusion-proof ]
> inclusion-proof   = bstr .cbor [ tree-size, leaf-index, inclusion-path ]
> ```
>
> Both wrappers — the array of proofs, and the `bstr` around each proof's CBOR
> — are load-bearing for conformance. Dropping either would have produced
> receipts that verify perfectly under lys and are unparseable by every
> conforming RFC 9942 implementation, which is precisely the failure this
> project exists to avoid. Caught before any artifact was signed, which is the
> entire argument for writing this file before building.

**lys issues and accepts exactly one proof in that array.** RFC 9942 permits
several; a receipt carries one signature over one root, so a multi-proof receipt
would need a rule for what disagreement between the proofs means. Checking only
the first while carrying others is a confusion attack waiting to happen — a
downstream reader could act on a proof the verifier never looked at. Widening
this means deciding the all-proofs-must-agree rule explicitly, and that is a v2
matter.

**Payload:** `nil` — detached.

### 1.2.1 Tree size 1 cannot be expressed, and the remedy is a genesis leaf

RFC 9942 types `inclusion-path` as `[+ bstr]` — *one or more*. The sole leaf of a
one-leaf tree has an **empty** path (RFC 6962's `PATH(0, {d0}) = {}`), so a
receipt for it cannot be a conforming artifact. The two facts are both correct
and jointly exclude a real state: a log's very first entry.

**Ruling:** issuance refuses `tree_size == 1`; the anchor's log is seeded with a
**genesis leaf** at initialisation, after which `tree_size >= 2` always holds and
every path has at least one node. Verification still *accepts* an empty path,
because refusing a mathematically true proof another implementation legitimately
made would be refusing a true statement — and nothing is admitted by accepting
it, since the root reconstruction independently requires the exact path length
that `(leaf_index, tree_size)` demands.

The asymmetry is deliberate: emit only what conforms, accept anything true. The
alternative — emitting one technically non-conforming receipt — would have made
it the *first* receipt in existence, and earliest artifacts are the ones others
reach for as interop test vectors.

### 1.2.2 Consistency proofs, `vdp` type `-2`

Not issued at launch (DP2 recommendation (b)), but specified here byte-exactly,
because "specified" previously meant a one-line sketch and the sketch was wrong.

RFC 9942 §5.3.1 ("Receipt of Consistency"):

```
RFC9162_SHA256_Verifiable_Consistency_Proofs = {
  &(consistency-proof: -2) => RFC9162_SHA256_Consistency_Proofs
}

RFC9162_SHA256_Consistency_Proofs = [
  + RFC9162_SHA256_Consistency_Proof
]

RFC9162_SHA256_Consistency_Proof =
  bstr .cbor RFC9162_SHA256_Consistency_Proof_Content

RFC9162_SHA256_Consistency_Proof_Content = [
  tree_size_1: uint,
  tree_size_2: uint,
  consistency_path: [ + bstr ]
]
```

Everything else matches §1.2: tagged `COSE_Sign1`, `alg -8`, `kid` in the
protected header, `vds 395 => 1`, detached `nil` payload, and **exactly one proof
in the array** for the same reason.

#### The content type MUST differ, or the two receipt kinds are confusable

`application/vnd.lys.consistency-receipt.v1+cbor` — **not** the inclusion
receipt's content type. The media type is lys's to choose (RFC 9942 does not
specify it), and this one has to be different, for a reason found by working out
the shape before writing any code:

With a shared content type, an inclusion receipt and a consistency receipt have a
**byte-identical protected header** — same `alg`, same content type, same `kid`,
same `vds` — and the payload is detached in both. The RFC 9052 §4.4
`Sig_structure` therefore covers *exactly the same bytes* for both kinds, and the
two are distinguished only by whether the **unprotected** header holds `-1` or
`-2`. Unprotected means not signature-covered, so that discriminator is free to
rewrite.

The consequence runs in the direction that matters. Take a valid *inclusion*
receipt: the anchor signed root `R` at size `S`. Rewrite its unprotected header to
`{-2: [<bstr .cbor [S1, S, path]>]}` and present it as a consistency receipt. The
signature still verifies — the detached payload is still `R`, which is exactly what
"the newer root at `tree_size_2`" is supposed to be. **The anchor never made a
consistency statement, and a verifier is now told that it did**, with a valid
signature to show.

The §1.3 argument for putting the proof in an unprotected header does not rescue
this. "Authenticated by consequence" works when tampering changes the
reconstruction; here the tampering changes *which reconstruction procedure the
verifier runs*, and both procedures can arrive at the same detached root.

**Ruling: the discriminator goes inside the signed bytes, as the content type.**
Same fix as the one already recorded for attestations — receipts and attestations
share the `0x84 0x6A "Signature1"` byte-0 prefix, so the protected bucket is what
separates them. Two of our own formats sharing a signature scope needs the same
treatment, and this is the third time that reasoning has applied.

Verifiers pin the content type they expect and refuse the other, so a
re-labelled artifact fails on the protected header before any proof is examined.
**A test must construct exactly this attack — an inclusion receipt re-labelled as
a consistency receipt — and prove it is refused.** Until that test exists this
section is a hypothesis, which is the standing rule in this file.

> **✅ NO LONGER A HYPOTHESIS.** `a_relabelled_inclusion_receipt_is_refused`
> (`receipt/consistency_tests.rs`) builds the attack rather than arguing it: a
> genuine inclusion receipt over the size-13 root, whose signature is then
> presented inside a consistency artifact whose `5 → 13` proof derives *exactly
> that root*. The signature is real and covers exactly the right 32 bytes, so
> only the content type stands in the way. It is refused, and the test asserts
> the spliced inclusion receipt is **independently valid on its own terms**, so
> the refusal is the re-labelling and not a broken fixture.
>
> Proven load-bearing by the drift that matters: collapsing
> `CONSISTENCY_CONTENT_TYPE` to `CONTENT_TYPE` — the one-token mistake, applied
> to *both* sides at once so honest issue-then-verify still round-trips — fails
> this test. A verifier-only version of the same drift breaks the honest path
> too and is therefore loud; **the both-sides version is the silent one, and it
> is the one this test exists for.**

##### The envelope has a Go gate, and building it found that the proof body's field order was pinned by nothing

`consistency_conformance.rs` gates the *derivation* — lys's iterative walk
against RFC 6962's recursive `SUBPROOF` and §2.1.4.2 in Go. It says nothing
about the COSE envelope those roots travel in, and a receipt whose root is right
but whose envelope no conforming library accepts is exactly as worthless as one
with the wrong root. `consistency_receipt_conformance.rs` closes that: over all
136 size pairs, lys signs and go-cose verifies, **and** go-cose signs and the
artifacts are compared **byte-for-byte**. Byte-identity is available here because
Ed25519 is deterministic and both sides emit RFC 8949 §4.2 core-deterministic
CBOR, and it is the stronger claim — it holds only if the two sides first agreed
on the detached payload, so equal bytes means equal roots, equal proof encodings
and equal header pins at once.

**⭐ THE FINDING, and it is a coverage hole this gate was the first thing to
see.** Swapping `tree-size-1` and `tree-size-2` in the wire encoder *and* the
decoder together — symmetric, so every round trip still succeeds — leaves **all
440 in-crate `lys-core` tests passing** and fails only this gate. The order of
the two sizes in the RFC 9942 §5.3.1 proof body was, until now, pinned by
nothing: the in-crate suite encodes and decodes with the same pair of functions,
so it has no way to disagree with itself about which field comes first. The
protected bucket has a hand-written golden vector; the proof body did not. This
is the one-party problem in its exact form, and the second party had to come from
another implementation reading the RFC's CDDL.

**⭐ AND THE GATE'S OWN NEGATIVE TEST WAS VACUOUS-CAPABLE.** Drifting the Go
content-type constant by one character made the sweep fail — and left
`go_cose_refuses_what_lys_refuses` **green**, because every assertion in it says
the Go tool *refused* something and a verifier that refuses everything satisfies
all of them at once. Its validity was borrowed entirely from a positive result in
a *different* test. Fixed by giving it its own positive control as its first
assertion; the same drift now fails both. **A test made only of refusals cannot
distinguish a working verifier from a broken one, and "the other test covers it"
is not a property the test itself has.**

##### The type is covered, and separately is not attacker-supplied — verified, with cites

A differing content type is only a discriminator if it is *inside the signed
message*. A type that sits beside the signature — an envelope field, a filename,
a header read before verifying — is decoration: the attacker rewrites the label
and the signature still covers the bytes it always did. For the shipped inclusion
receipt, four separate things hold, and each closes a different attack:

1. **It is in the protected header.** `receipt/encoding.rs:115-116` writes label
   `3` with `CONTENT_TYPE` into the protected map.
2. **The protected header is a covered input.** `cbor.rs:96` builds the RFC 9052
   §4.4 `Sig_structure` `["Signature1", protected, h'', payload]`; the signature
   is taken over exactly that (`receipt/sign.rs:156-157`).
3. **At verification the protected header is *re-derived*, not read from the
   wire** (`receipt/sign.rs:214-215`). So the discriminator is not merely
   covered — at signature-check time it is not attacker-supplied at all. A
   re-labelled artifact cannot even present its label to the check.
4. **The wire copy is independently pinned byte-exactly.** `decode_protected`
   (`receipt/encoding.rs:243-265`) destructures the map as exactly four entries
   in exactly that order and pins each value, and the re-encode identity gate
   (`receipt/artifact_tests.rs:42-47`) refuses a non-canonical encoding of the
   same value. Without (4), (3) would leave the wire header unconstrained and two
   distinct byte strings would both verify — malleability, not forgery.

**Consequence for how the test must be built.** The usual positive-control
recipe — *a fixture that passes today and fails once the type is covered* —
**cannot be constructed here, because there is nothing uncovered to flip.** Its
absence is not evidence the control is missing; it is evidence the defect it
screens for is already absent. Claiming that recipe as the standard would be
asserting a control that cannot fire, which is the failure this repo names
elsewhere.

##### "Covered" does not answer "chosen by whom" — the selection is caller-supplied

Re-derivation (3) means the verifier must decide *which* constant to re-derive
*before* it verifies. **If that branch were driven by a type field read off the
wire, re-derivation would buy nothing** — an attacker flips the label, the
verifier obligingly re-derives the matching constant, and the signature checks
out. So the question is not whether the type is covered but who selects it.

**It is the caller, by Rust type, and the wire has no vote.** There is no
dispatch on the content type anywhere in the workspace. `from_cose_bytes` is an
inherent associated function on the concrete `AnchorReceipt`
(`receipt/artifact.rs:123`) — naming the type *is* declaring the intent, and
there is deliberately no polymorphic `from_cose_bytes(bytes) -> AnyReceipt`
entry point. `decode_protected` then tests equality against **one** hardcoded
constant (`receipt/encoding.rs:257`), not membership in a set of known ones, so
property (4) pins against the re-derived constant rather than against
"any recognised type".

**This is a standing constraint, not an accident of the current shape.** A
convenience API that sniffs the type and dispatches would move selection to the
attacker while every one of the four properties above still held, and would read
as a usability improvement. If such an entry point is ever wanted, it must
dispatch to the per-type decoder and the dispatch must not be what authenticates
— the caller still has to say what it expected.

##### A false finding of my own, and what caught it

While checking the above I concluded the receipt content-type pin was **not
covered by any test**, on the strength of a grep for `CONTENT_TYPE` across the
receipt test modules that returned only the frozen-string test. That was wrong.
`every_protected_header_pin_is_enforced` (`receipt/encoding_tests.rs:285-321`)
mutates the content type's first byte at a verified offset (lines 303-308) and
asserts rejection. **Removing the pin fails exactly that test and nothing else —
confirmed by injection, which is the only reason the false finding was not
filed.**

The error was not a bad grep; the grep answered exactly what it was asked. **The
question was wrong: I searched for a name and concluded about a behaviour.** A
test that mutates a byte at an offset never spells the constant it is defending,
so name-search is structurally blind to precisely the best-built tests. Distinct
from a tool-level false zero (a mis-quoted glob), because here the tool was
correct and complete.

**The genuine residual it left behind is smaller and real.** That one test
bundles five independent pins — `alg -7`, `alg -19`, content type, `vds`, map
arity — so removing *any* of the five fails the same single test. The
exactly-one-test-fails standard is met at the test level and **not at the case
level: the failure cannot say which pin broke.** It matters because this test is
the template the consistency path would copy, and a five-in-one case would make
the re-label drift injection ambiguous exactly where it must be decisive. The
consistency pins get one case each.

**A further residual risk is narrower and is a one-token mistake.** Because (3)
re-derives the header from a *hardcoded* constant per code path, the separation
rests entirely on the consistency path passing its own constant. If
`receipt/consistency.rs` calls `protected_bytes` with the inclusion content type,
the two preimages coincide, the re-label attack succeeds, and **no test in the
suite today would fail** — nothing yet asserts that the consistency preimage
differs from the inclusion one. That is what the drift injection must pin: emit
the inclusion constant from the consistency path, and **exactly one** test may
fail, the re-label test. Assert the exact tag bytes of both protected buckets,
not merely that they are unequal.

**The detached payload is the *newer* root.** RFC 9942 §5.3.1: *"In a signed
consistency proof, the newer Merkle Tree root (proven to be consistent with an
older Merkle Tree root) is a detached payload and corresponds to the log at size
`tree-size-2`."*

#### A consistency receipt is NOT self-verifying, and the RFC does not say where the older root comes from

This is the important asymmetry with §1.2, and it is not a detail.

An **inclusion** receipt plus the leaf bytes is enough for a stranger: the
verifier recomputes the detached root from `(leaf, leaf_index, tree_size, path)`
and the signature either matches or does not. Nothing else is needed — no
checkpoint, no service, no prior state.

A **consistency** receipt cannot work that way. RFC 6962 consistency verification
relates *two* roots, and only one of them (the newer) is the detached payload. The
older root is not in the artifact, and **RFC 9942 does not state where a verifier
obtains it** — asked directly, the document simply does not address retrieval of
the `tree_size_1` root. §5.3.1's verification prose ("the consistency proof is
checked by applying a previous inclusion proof to the consistency proof") does not
close the gap either; it names no comparison.

That gap has a security consequence, and DECISIONS.md's governing principle
settles it: **a claim must not be settable by the party being judged.** If a
verifier accepts the older root from anywhere convenient — including anything the
anchor supplied alongside the receipt — then the anchor chooses both endpoints of
the statement it is being held to, and "consistent with an earlier version of my
log" becomes "consistent with whatever earlier version I nominate today". That is
precisely the equivocation a consistency proof exists to detect, re-admitted
through the verifier's front door.

**Ruling, matching §1.4's discipline for the anchor key: verification takes the
older root as a required argument from the caller, and there is no form of the
call that does not.** The caller is asserting *"this is the root I previously
held"*, and the receipt is then evidence about that specific prior belief. A
consistency receipt is not a self-contained artifact; it is an artifact that
attaches to a root the verifier already had, and the API must make that
impossible to forget rather than merely document it.

The same reasoning as `verify_receipt` requiring its anchor key, and the same as
the G1/G2 identity join: an artifact that carries its own trust anchor verifies
against whatever anchor it carries.

#### Verification derives the newer root; it does not accept one

Given the older root, there are two ways to check a consistency receipt, and the
choice decides what the artifact is *worth*.

**(a) Take both roots from the caller.** Use the existing
`merkle::verify_consistency(old_root, new_root, proof)`, then check the signature
over the caller's `new_root`. Sound — a wrong `new_root` fails the signature — and
it needs no new code. But the receipt then only confirms a statement the verifier
already held in full. It cannot *tell* you the anchor's current root, so it is
useless for the case that matters: learning where the log has got to, provably.

**(b) Derive the newer root from `(old_root, tree_size_1, tree_size_2, path)`** and
check the signature over the derived value. This is authentication by consequence,
exactly as for inclusion: alter the path or either size and the derived root
changes, so the signature over the root the anchor actually signed fails. The
caller supplies only what it legitimately owns — the root it previously held — and
*learns* the new root as an output.

**Ruling: (b).** It is what RFC 9942's detached payload is for; (a) makes the
detachment pointless.

**This requires a primitive lys does not have yet.** `merkle` exposes
`root_from_inclusion_path` (the inclusion analogue, cross-checked against
`ct-merkle` at every size 1..33) and `verify_consistency`, which takes both roots
and delegates. There is no `root_from_consistency_path`. Writing one means
implementing RFC 6962 §2.1.4.2's reconstruction directly, and it inherits the
standard this repo already applies to hand-written Merkle walks: swept against
`ct-merkle` across every `(old_size, new_size)` pair, plus the Go gate deriving the
same value from the RFC's *recursive* `SUBPROOF` definition so two independently
written algorithms must agree.

Until that exists and is cross-checked, `-2` stays specified and unimplemented.
That is the honest state, and it is recorded here rather than left as an absence
someone reads as "nearly done".

#### Two more things the RFC leaves open, ruled here

- **Size ordering is not specified.** RFC 9942 does not say whether
  `tree_size_1 < tree_size_2` is required or equal sizes are permitted. lys rules
  it below, and the ruling is lys's own, not the RFC's — recorded that way so a
  future reader does not go looking for RFC authority that is not there.
- **The verification order is underspecified.** lys applies §1.4's rule
  unchanged: signature first, and the proof is not examined at all until the
  signature over the detached newer root verifies against the named anchor key.

> **Correction, 2026-07-30 (fourth in this file).** The previous revision wrote
> this as `-2 → [size_1, size_2, consistency_path]` — a **bare array**, missing
> the array-of-proofs wrapper *and* the per-proof `bstr .cbor`. That is the
> identical defect corrected for `-1` in §1.2 above, repeated one paragraph
> below its own correction notice.
>
> The transferable part is not "check the CDDL". It is that **a correction
> recorded in prose does not propagate to its neighbours.** The `-1` fix was
> written as a note about `-1`, so the `-2` line beside it kept the same wrong
> shape and read as settled because the paragraph above it looked rigorous.
> When a defect is found in one instance of a pattern, the fix is to sweep every
> instance of that pattern in the same pass — the note is a record, not a
> repair.

#### An equal-size consistency proof cannot be expressed

`consistency-path` is `[ + bstr ]` — one or more. But **a consistency proof
between two equal sizes has an empty path**, and it is a true and useful
statement: *these two views of the log are the same log, unchanged.* Two correct
facts jointly excluding a real state, exactly as `[ + bstr ]` on
`inclusion-path` excludes a one-leaf tree (§1.2.1).

This is measured, not deduced. `merkle` sweeps all 153 pairs
`1 <= old_size <= new_size <= 17` and asserts the path is empty **iff** the
sizes are equal — 17 such cases, with the swept count asserted so a
zero-iteration loop cannot pass
(`a_consistency_path_is_empty_exactly_when_the_sizes_are_equal`).

**Ruling, following §1.2.1: issuance refuses `tree_size_1 == tree_size_2`;
verification still accepts an empty path.** Emit only what conforms, accept
anything true.

> **⛔ THE SECOND HALF OF THAT RULING IS WITHDRAWN — verification refuses equal
> sizes too. This is the FIFTH defect found in this draft by implementing it,
> and like the other four it was invisible to re-reading.**
>
> **The analogy to §1.2.1 does not hold, and writing
> `merkle::root_from_consistency_path` is what exposed the difference.** For a
> one-leaf inclusion path the empty path still leaves a derivation standing: the
> root is computed from the *leaf hash*, which the verifier supplies and which
> the receipt's claim is about. Authentication-by-consequence keeps working —
> substitute a different leaf and the derived root stops matching the signature.
>
> For equal sizes **there is no derivation left at all.** The newer root is the
> caller's `old_root` argument, returned untouched. So the signature check
> collapses to *"has this anchor ever signed this 32-byte value?"* — with the
> value entirely chosen by whoever presents the receipt.
>
> **Concretely: any inclusion receipt becomes an equal-size consistency receipt.**
> Its payload is a 32-byte root; present it as `old_root` with
> `tree_size_1 == tree_size_2` and an empty path, and the derived "newer root"
> equals it, so the anchor's real signature verifies over the real value. The
> content type is then the *only* thing refusing it. §1.2.2 already requires the
> content type to differ, and that check stands — but **domain separation should
> be the second line of this defence and here it would be the only one.**
>
> **Nothing is lost by refusing.** A verifier holding `old_root` learns exactly
> nothing from being told the log is still that size. *"Accept anything true"* is
> the right rule while the true statement carries information; it was never
> written for a vacuous statement whose acceptance degenerates a signature check
> into an existential query over the anchor's whole signing history.
>
> Enforced in `root_from_consistency_path` (`size_1 >= size_2` refused) and
> pinned by `equal_sizes_are_refused_even_though_the_statement_would_be_true`,
> drift-injected: permitting equal sizes fails that case and only that case.

No change is needed to make that hold. `tlog::build_consistency_artifact`
already requires `1 <= old_size < new_size` strictly, decided for its own
reasons before this limit was known. `merkle::prove_consistency` deliberately
*does* allow `old_size == new_size`, and should keep allowing it: the primitive's
job is to express a true proof, and the place a wire format's limit belongs is
the artifact layer that has to encode it.

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

### 1.3.1 The limit of "authenticated by consequence": `tree_size`

Authentication by consequence is exactly as strong as the consequence, and for
`tree_size` the consequence is incomplete. `tree_size` is pinned only as far as
it **changes the reconstruction**, and it does not always change it. At
`leaf_index = 0`, a tree of size 3 and a tree of size 4 produce the same sequence
of left/right combinations, so a valid receipt can be re-presented with the other
size: same leaf, same index, same root, same signature, different claimed size.
`receipt::sign_tests` proves this rather than leaving it to be discovered, and
also proves that sizes *outside* that class are refused — the equivalence is
narrow, not an absence of checking.

This is not a forgery and not a false inclusion claim. The leaf, the index and
the root are unaffected, and the information needed to distinguish the two sizes
is simply not present in an RFC 6962 inclusion path — which is why RFC 9162 tells
verifiers to check a proof against a *known* root at a *known* size.

**Ruling: conform and document.** Fixing it in the format would mean moving the
proof into the protected header or signing `root ‖ tree_size`, and both break
RFC 9942 conformance — a high price for a low-impact, externally detectable
malleability. A consumer relying on `tree_size` for anything load-bearing must
cross-check it against the anchor's published checkpoint at that size, where a
relabelled size mismatches immediately.

**Consequence for §2.3 (the bundle):** the chain-link check must compare *proven
leaf bytes*, never claimed sizes. Checkpoint bytes are self-describing and
signature-covered by the child's own note; `tree_size` in a receipt is not.

**The discharge is now tested, from both directions.** `bundle_conformance`
carries a receipt that claims size 3 while carrying a size-4 path — the exact
malleability above, at `leaf_index = 1`, where sizes 3 and 4 also share a walk —
and asserts that the bundle refuses it because the anchor's own note-signed
checkpoint says 4. Two facts came out of building that case, and both are worth
keeping:

- The malleability reproduces **identically in an independent implementation**.
  The Go cross-check rebuilds the root from RFC 6962's recursive structure rather
  than lys's iterative walk, and accepts the relabelled receipt too. That
  confirms the malleability belongs to the RFC's proof format and is not a defect
  in lys's walk — which is what "conform and document" assumed but had not shown.
- The same relabelled receipt on the **final** link is accepted by both, because
  there is no next checkpoint to contradict it. That is the honest residual limit
  and it is now pinned as a passing test rather than left as prose.

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
  "links": [
    {
      "checkpoint": "<the signed note this link's receipt proves, verbatim>",
      "receipt": "<base64 of tagged COSE_Sign1 receipt bytes>"
    }
  ],
  "counter_anchor": null
}
```

> **Correction, 2026-07-30 (implementation).** An earlier revision gave this as
> `"receipts": ["<base64 …>"]` — a flat array of receipts. **That is
> unverifiable for any chain longer than one link,** and the reason is the
> receipt design in §1 working exactly as intended: a receipt's payload is
> *detached*, so verifying receipt *i* requires the leaf bytes it proves.
>
> - Receipt 0's leaf is the log's signed note, which the bundle already carries
>   as `inclusion_proof.checkpoint`. Fine.
> - Receipt 1's leaf is **anchor 0's own signed note**, which appeared nowhere in
>   the bundle — and cannot be reconstructed from receipt 0 either. A receipt
>   yields a root and a tree size, but §1.5 deliberately keeps origins out of
>   receipts and a receipt carries no note signature.
>
> So `receipts` becomes **`links`**, each pairing the notarized checkpoint with
> the receipt over it. Both halves are load-bearing and neither is redundant.

### 2.2.1 `counter_anchor` is an opaque base64 string, not nested JSON

Typing the slot as arbitrary JSON would make `lys-core` depend on a JSON codec,
which it deliberately does not — the crate derives `Serialize`/`Deserialize` and
lets the consumer choose the codec (a stated `tlog` invariant). A counter-anchor
proof is binary anyway (an `.ots` file), so base64 matches how `leaf` and
`receipt` are already carried.

**A populated slot is refused** while nothing can verify one. Carrying a time
attestation that nothing checks is how a reader comes to believe it. The slot
exists so a future version needs no v2 — not so today's bundles can gesture at
time.

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
3. **`links[0].checkpoint` must be byte-identical to
   `inclusion_proof.checkpoint`.** This is the join — it makes the first
   notarization about *the log the leaf is in*, rather than about some other log
   whose checkpoint also happens to verify.
4. For each link *i*: verify its receipt against `links[i].checkpoint` as the
   proven leaf, under the anchor key the **caller** supplied for that link.
5. **The rung.** Where `links[i+1]` exists, its checkpoint must be anchor *i*'s
   own signed checkpoint, and its root and tree size must equal the root receipt
   *i* reconstructs and the size it claims. This forces two independently signed
   statements to agree — the anchor's *receipt* signature over a recomputed root,
   and the anchor's *note* signature over its published checkpoint. **It is also
   what makes §1.3.1's `tree_size` malleability harmless here:** a relabelled
   size no longer matches a note-signed one.
6. If `counter_anchor` is present, refuse (§2.2.1) until something can verify one.

**Steps 3 and 5 are the bundle's one interesting vulnerability** — they are what
turn a pile of valid receipts into an actual chain. Both are proven load-bearing
by drift injection: removing either fails exactly the one test built for it, and
nothing else.

**Correction found by drift-injecting the two halves of step 5 separately.** The
rung compares a root *and* a size, and the obvious attack against it — an anchor
that grows after issuing a receipt — changes both, so it cannot show either
comparison is individually necessary. Removing the root comparison left that
attack still caught by the size comparison, and the drift passed unnoticed. Each
half needed a case that trips only that half, and both now exist in
`bundle_conformance`:

| removed check | the only case that fails |
|---|---|
| step 3, the link-0 join | `link_zero_over_an_unrelated_log` |
| step 5's root comparison | `anchor_equivocates_at_the_same_tree_size` |
| step 5's size comparison | `relabelled_tree_size_contradicts_the_anchors_checkpoint` |

The lesson generalises past this format: **a drift injection that removes a check
and still sees the suite fail has proven nothing unless exactly one test fails,
and that test is the one built for that check.** An attack that trips two checks
at once hides the loss of either.

**Interop evidence (D6) now covers both formats, but not in the same way.** A
receipt is one signed artifact with a deterministic encoding, so its gate asserts
byte-identity against go-cose across 152 tree shapes. A bundle is a container
whose value is entirely in the relationships between the artifacts inside it, and
those live in checks rather than bytes — so its gate asserts **verdict parity
over 23 cases**, in both directions, against an independent Go verifier that
opens notes with `sumdb/note`, checks receipt signatures with go-cose, and
rebuilds roots from RFC 6962's recursive structure. Accepted cases must also
agree on every value derived: leaf, origin, size, and each reconstructed root.

**Trust inputs are the caller's.** The bundle names no keys — not the log's, not
any anchor's. One `NoteVerifierKey` per link serves both roles, and that is a
binding worth enforcing rather than a convenience: **an anchor's receipt-signing
key must be the same key its published checkpoints are signed with**, or nobody
could cross-check a receipt against the anchor's own log.

`anchors.len()` must equal `links.len()` exactly. A bundle claiming more
notarization than the verifier will check cannot be checked, and silently
verifying a prefix would report success for less than the bundle asserts.

**An empty chain is valid** and asserts something weaker: a leaf in a log nobody
notarized. The verifier reports it rather than letting a reader assume otherwise.
Truncating a chain is likewise not an attack — it removes a notarization rather
than fabricating one.

Every step uses artifacts that verify standalone. The bundle adds no step that
requires trusting the bundle.

---

## 3. Status of the five open items

All five are now settled — items 1, 2, 4 and 5 by derivation in
[DECISIONS.md](DECISIONS.md), and item 3 (§1.5, the anchor-origin
sub-question) by taking the lean: **the receipt carries no origin string.** The
anchor is identified by its key in label `4`, and the checkpoint being proven
already carries the child's origin. A second name for the same thing is a second
thing to disagree with itself, and D1 made origins mandatory precisely because
collisions are a security defect — adding an unvalidated origin field here would
reintroduce that surface for convenience. §1 is implemented that way.

What §1 still needs is not a decision but an act: authorisation to sign
something durable. Until then the tag is free.

### Original text, kept for the record

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
