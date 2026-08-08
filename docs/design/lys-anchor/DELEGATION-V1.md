# `lys/anchor-delegation/v1` — the specification, written before the implementation

This is the briefing document for increment 11. It is deliberately written **before** any
Rust exists, so that the implementation has something it can disagree with. Anything below
that the code contradicts is a bug in one of them, and which one is a question worth asking
rather than a formality.

Derived from `docs/design/lys-anchor/BUILD-PLAN.md` §3.6, DP16, DP17, DP26, RFC 9052
(COSE_Sign1), RFC 8949 §4.2 (deterministic encoding), RFC 8032 (Ed25519).

---

## 0. Where it lives, and why that is not where the build plan put it

BUILD-PLAN §2.4 places `keys/delegation.rs` and `keys/delegation_encoding.rs` in
`lys-anchor`. **That is overridden.** The format lives in **`lys-core::delegation`, behind
the existing off-by-default `unstable-anchor` feature.**

The reason is a code fact, not a preference: `lys-core`'s `cbor` module — which owns
`write_head`, `write_bytes`, `write_text`, `write_i64` and `sig_structure_bytes` — is
`mod cbor;`, **private** (`lys-core/src/lib.rs:36`). A delegation entry is a tagged
`COSE_Sign1` over canonically-encoded CBOR, so building it in `lys-anchor` means
re-implementing RFC 8949 §4.2 heads and the RFC 9052 §4.4 `Sig_structure` a second time in
the workspace. `lys-anchor/src/keys/signer.rs` already names that outcome as the thing to
avoid: *"a second copy of a canonical encoder in the workspace, which is the failure this
repository guards against hardest: two encoders drift silently, and every round-trip test
on either side keeps passing while they do."*

The build plan's own objection to `lys-core` — §2.3, *"a format with no second
implementation and no operator ratification should not sit in the crate that gets
published"* — is answered rather than ignored:

- **Ratification** is what `unstable-anchor` exists for. CLAUDE.md: the gate holds draft
  wire formats *"until they are ratified"*, and `lys/anchor-receipt/v1` and
  `lys/verification-bundle/v1` already sit behind it for exactly this reason. A third draft
  in the drawer built for drafts is the gate working, not a hole in it.
- **A second implementation** is a deliverable of this increment, not a missing
  precondition: §6 below requires an independent encoder and a `go-cose` gate before the
  format lands.

Location changes nothing about *when* the format freezes. It freezes when a real anchor
writes leaf 0, because `LeafStore` has no insert — and that is DP17's, not this file's.

## 0.5 🔴 OPEN DECISION — the `origin` field may become a general `subject`, and this is the only free window

**Raised 2026-08-08, before publication, by the arrival of a real consumer. Tom's to rule.**

This document specifies `origin` as a DNS-style origin on DP15's reasoning — *pick the domain
you will still control in twenty years* — compared by raw byte equality against a value the
verifier was configured with. That is right for an anchor. It is wrong for the first consumer.

`manifold` is already a `lys-core` consumer (26 call sites, identity and attestation both
load-bearing). Its requirement, written in its own source at `compose/record.rs:160`, is that
*"passkey + lys certificate is what turns a claim into a countersigned delegation"* — and the
structural fact underneath it is that **its registry is keyed on the handle and nothing else,
so a seat has no registry entry to be checked against.** The gap is therefore not a missing
check but a **missing binding**, and the architecture that supplies one is exactly one hop:

> **The registry pins the handle. A delegation extends that pin one hop to a seat.**

A verifier walks envelope → delegation → registry pin: three links, each terminating in
something it already trusts.

**But this format cannot express it, and forcing it would be a lie in the signed bytes.** A
handle is not a domain; putting one in `origin` puts a value into a slot whose documented
meaning is something else, which is the "signed value that reads as something it is not"
failure this document argues against in §2.3 and §2.4. `role` has one inhabitant and rejects
unknown values *by design*, so "speaks-for" does not exist. `sequence` is scoped to
`(origin, role)` for rotation ordering in a log that path does not have.

**The two options, and why the timing is the whole point:**

1. **Generalise now** — one payload saying *"key A states key B holds role R for subject S,
   from time T, sequence N"*, with `subject` an **opaque typed string**: an anchor passes a
   domain, manifold passes a seat identifier. This is DP13 (domain-agnostic) applied to the
   payload rather than only to the crate.
2. **Mint a sibling later** — leave this format alone and add a second one.

⭐ **Publication is what forecloses option 1.** `lys-core 0.3.0` is deliberately held, so
generalising is free *today* and a `v2` after any release. A consumer arriving inside that
window is luck to spend rather than admire.

The honest argument against generalising: a format shaped by whichever consumer arrived first
is a format shaped by an accident. The mitigation is to keep `subject` opaque and typed rather
than trying to anticipate every future subject kind — describe the slot, not its occupants.

### 0.5.1 The `typed` half of "opaque typed subject" is load-bearing, and it is what preserves DP15

Worth stating separately, because "opaque string" and "opaque **typed** string" are one word
apart and the word carries a security property.

**Generalising `origin` to a bare string would move DP15's protection out of the format and
into the caller.** Today the field's meaning is fixed, so "this is a domain, and a lapsed
domain orphans every proof" is a property of the artifact. Make it a bare string and an anchor
could pass a seat identifier, or a consumer a domain, and **nothing would notice** — the
delegation would verify perfectly and mean something no verifier could pin down. That is
precisely the §2.3 failure (a signed value whose semantics live nowhere) reintroduced through
the field that used to be the most tightly specified.

So the subject carries its **kind** alongside its value, and **a verifier states the kind it
expects**. Cross-kind confusion is then refused at decode rather than discovered downstream —
the same discipline §3.1 applies *between* formats, applied *within* one. A seat delegation
cannot be presented to an anchor verifier and a domain delegation cannot be presented to
manifold, for the same structural reason an inclusion receipt cannot be re-labelled as a
consistency receipt.

The kind set is closed and unknown kinds are rejected, on §2.3's reasoning exactly. Two kinds
are known to be wanted; a third is a `v3`, and that is what versioned wire contracts are for.

**This makes option 1 strictly better than a bare generalisation and answers the
"shaped-by-whoever-arrived-first" objection concretely**: the format is not shaped by manifold,
it is shaped to *distinguish* manifold's subject from an anchor's — which is a property the
current single-purpose field gets for free and a bare string would silently lose.

**What is already built stays useful either way.** The construction — tagged `COSE_Sign1`,
canonical CBOR, `kid`-is-a-claim, re-encode-and-byte-compare, non-oracle failure, an
independent second encoder and a `go-cose` gate — is payload-independent. Increment 11b
(genesis-as-delegation) changes a call site if the payload generalises, not a structure.

## 1. The artifact

A **tagged** `COSE_Sign1` (RFC 9052 tag 18), so byte 0 is `0xD2`.

```text
0xD2 / 18(COSE_Sign1) /
[
  protected   : bstr wrapping the canonical CBOR map below,
  unprotected : {} ,          <-- EMPTY. See §1.3.
  payload     : bstr wrapping the canonical CBOR map below,   <-- EMBEDDED, not detached
  signature   : bstr .size 64
]
```

### 1.0 `external_aad` is the empty byte string, and the tag is mandatory

**Both added after the independent encoder found them missing.** Each is inside, or decides,
the signed bytes, and neither was stated:

- **`external_aad` = `h''` (zero-length, encoded `0x40`)** — RFC 9052 §4.4's default. It sits
  *within* the `Sig_structure`, so an unpinned `external_aad` means two conforming
  implementations produce different signatures over the same claim and neither is wrong.
  This was the single most valuable finding of the exercise: it was in the briefing given to
  the implementers and **not in the specification**, which is precisely the kind of gap that
  survives review because everyone in the room happens to know the answer.
- **The tag is mandatory on the wire, and an untagged `COSE_Sign1` is REJECTED.** RFC 9052
  §4.2 permits either form "depending on the context", so the context must say. If both were
  accepted, one statement would have two valid encodings — the exact defect §3.4 exists to
  prevent — and §3.4's re-encode-and-compare only closes it because the canonical
  re-encoding emits the tag.

### 1.1 Protected header — signature-covered

```text
{
  1  : -8,                                                    / alg = EdDSA          /
  3  : tstr,                                                  / content type         /
       "application/vnd.lys.anchor-delegation.v1+cbor"        /   45 bytes           /
  4  : bstr .size 32                                          / kid = ROOT public key/
}
```

`kid` is a **`bstr`** carrying the raw 32-byte key (RFC 9052 §3.1), written in CDDL here
rather than prose because the prose form left the encoding to be inferred.

Keys ascending: `1, 3, 4`. All non-negative and single-byte-headed, so ascending numeric
label order and RFC 8949 §4.2 bytewise order coincide — the same argument
`receipt/encoding.rs` records for `{1, 3, 4, 395}`.

There is **no `395` (vds)** entry: that label declares a verifiable data structure and a
delegation proves nothing about a tree. Its absence is also what makes the protected bucket
a different length from a receipt's, though that is a consequence, not the defence — see
§3.1.

### 1.2 Payload — embedded, signature-covered

Deterministic CBOR map, keys ascending `1, 2, 3, 4, 5`:

```text
{
  1 : origin                  (tstr)         / e.g. "example.test"                    /
  2 : delegated public key    (bstr .size 32)/ the key being delegated TO             /
  3 : role                    (uint)         / 1 = operational. See §2.3.             /
  4 : not_before_unix_ms      (uint)         / a CLAIM by the signer, not an order key/
  5 : sequence                (uint)         / strictly increasing per (origin, role) /
}
```

**`sequence` was added by the adversarial review, and it is the reason the review happened.**
See §2.2 — without it this format is replayable by an attacker holding no key material.
It is a **`u64`, shortest-form head**, on exactly the reasoning §1.2 gives for `not_before`:
without the bound stated, two conforming implementations disagree again at ≥ 2⁶³.

**`sequence == u64::MAX` is REJECTED**, on both encode and decode. Strictly-increasing has no
successor at the maximum, so a signer who issues there can never supersede it and the key is
frozen for the life of the origin — the format would permit a delegation that permanently
disables its own rotation. Not attacker-reachable (only the root key signs), so this is a
self-inflicted foot-gun rather than a vulnerability; it is forbidden anyway because it costs
one comparison now and is impossible to add after the freeze. **Rejecting the maximum is what
makes "a successor always exists" a property of the format rather than of operator care.**

Two questions `sequence` leaves open, recorded so they are decided deliberately rather than
by whoever writes the code:

- **Whether the counter is shared across roles or restarts per role** cannot be tested today,
  because `role` has one inhabitant and the partition is effectively per-origin. It is a
  compatibility question about *already-signed* `v1` entries, so it must be decided **with**
  the `v2` role, never after it.
- **Nothing marks a genesis delegation.** `sequence = 0` is a convention, not a claim, so a
  fold reading from a non-zero log offset cannot tell whether it has seen the true minimum.
  Not exploitable alone — forging any delegation needs the root key — but it bears on §5 and
  on any partial-log verifier.

⚠️ **`sequence` is carried by `v1` and consumed by nothing in `v1`, and that must be said
plainly rather than left to be discovered.** The replay defence in §2.2 is a property of the
**fold**, and §2.4.1 defers the key-history artifact — and therefore the fold — to its own
version. So the artifact is still byte-identical across two issuances of one claim; replay is
demoted from a rollback to a no-op *only for a consumer that implements the ordering rule*.

This is structurally the shape §2.3 refuses for unknown roles — a signed value whose check
lives somewhere that does not exist yet — and it is admitted here rather than waved through.
The difference that makes it acceptable: the semantics **are** defined, in §2.2, and only the
implementation is pending, whereas `role: 7` has no defined meaning anywhere. **A field that
must exist before the freeze and cannot be consumed until after it is the one case where
carrying an unchecked value is the lesser fault** — the alternative is a format that can
never be made replay-safe at all.

`not_before_unix_ms` is a **`u64`**, encoded with the **shortest head that fits** (RFC 8949
§4.2). Both halves matter and an earlier draft of this line said `uint .size 8`, which was
ambiguous in a way the test vector could not expose:

- **`u64`-bounded**, because CBOR `uint` reaches 2⁶⁴−1 and an implementation modelling the
  field as `i64` would find values ≥ 2⁶³ wire-legal and undecodable — two conforming
  implementations disagreeing about a well-formed artifact. Unsigned also removes pre-1970
  timestamps from the format rather than from a validator.
- **Shortest-form, not fixed-width.** Read as "always emit the 8-byte head `0x1b`", the old
  wording contradicted §3.4's canonical encoding, and `not_before = 1` would then encode two
  ways. The §6.1 vector cannot catch this, because `1700000000000` needs the 8-byte head
  anyway. **A test vector that happens to sit on the safe side of an ambiguity hides it.**

**An empty origin is REJECTED.** Not because the empty string is malformed, but because of
what it does to a *misconfigured verifier*: §3.3 makes acceptance a comparison against the
caller's configured origin, so a verifier whose origin is unset would match a delegation with
an empty origin and accept it. DP15 already forbids a default origin; refusing the empty
string at decode is the same rule enforced from the other side, and it makes that
misconfiguration fail closed instead of silently succeeding.

**`origin` is bounded by the artifact size cap**, which is `4096` bytes, mirroring the
receipt's. This is a wire-visible decision rather than an implementation detail: an artifact
with a longer origin is well-formed under this specification's grammar, so another
implementation would accept what lys refuses. It is stated here so that divergence is a
specified limit rather than a surprise found by a stranger.

**Origin comparison is raw UTF-8 byte equality** (§3.3). Not case-folded, not
Unicode-normalised, no trailing-dot or port or scheme handling. The test vector is ASCII and
so cannot expose this; an IDN origin would. Byte equality is chosen because every other rule
is a normalisation rule that two implementations can apply differently, and a comparison
that differs between verifiers is a comparison an attacker picks the side of.

Embedded rather than detached because, unlike a receipt, **there is no value the verifier
independently recomputes.** A receipt's payload is a Merkle root the verifier derives from
leaves it holds; a delegation's payload is an assertion, and an assertion nobody can
recompute must travel with the signature that carries it.

### 1.3 Unprotected header is empty, and that is a requirement

`{}` — encoded `0xA0`. Nothing may be carried there and a decoder **must reject** a
non-empty unprotected bucket.

An unprotected header is unsigned by definition. A receipt puts its inclusion proof there
because the proof is checked by reconstruction, so tampering with it is caught by the check
it feeds. A delegation has no such check: anything placed in the unprotected bucket would be
attacker-controllable data sitting inside an artifact whose whole purpose is to be trusted.
**Refusing the empty-map violation is the only way "there is nothing unsigned here" is a
property rather than a hope.**

---

## 2. Semantics, including what this format does NOT do

### 2.1 What it says

*"The holder of root key `K_root` states that, for origin `O`, key `K_op` holds role `R`
from time `T` onward."* That is all. It is one signed claim, and DP26's revocation model —
revocation is an append — means the claim is never mutated, only superseded by a later
entry in the log.

### 2.2 ⛔ CORRECTED — `sequence` orders delegations. Log position CANNOT, and this was an exploitable defect

**The original text of this section said the opposite, and the adversarial review broke it.**
It read: *"What orders delegations is their position in the append-only log … It must never
be used to order two delegations"* of `not_before`. That is replayable.

**The attack, demonstrated executably.** Every payload field is chosen by the signer and
Ed25519 is deterministic, so **two issuances of the same claim are byte-identical** — the
implementation's own `issuance_is_deterministic` test establishes this for an unrelated
reason. So:

1. Leaf 0: root `R` delegates to `K1`.
2. `K1` is compromised. Per DP26 the operator appends leaf *N*: `R` delegates to `K2`.
3. An attacker **holding no key material whatsoever** appends leaf *N+1*, a verbatim copy of
   leaf 0. Those bytes are public — they are in the log, which is the point of the log.
4. Under "ordered by log position, last wins", the revoked `K1` is current again.

A replay and a legitimate re-delegation to `K1` produce **identical bytes**, so no fold over
the log can distinguish them. The `AcceptAll` policy ships (`admission/trivial.rs`), and even
under a restrictive one the compromised operational key is a legitimate submitter.

**Why the original reasoning failed.** It rejected `not_before` because *"a signer can write
any number it likes"* — true, but the signer is the **offline root key**, i.e. the trusted
party. The rule removed the format's only replay-resistant ordering key and replaced it with
one the attacker can append to. Ordering by a monotonic value the trusted signer chose is
what a certificate serial number has always been.

**The rule, corrected:**

- **`sequence` (label 5) orders delegations**, strictly increasing per `(origin, role)`. A
  replay carries a sequence already superseded, so it loses; a replay of the *current*
  delegation is byte-identical to what is already current and changes nothing. **Replay
  becomes a no-op rather than a rollback.**
- **The log establishes existence and publication, not order.** DP16's "the key history *is*
  log history" survives intact: the log is still where the history is published and proven.
  It is simply not the sort key.
- **Two delegations at the same `(origin, role, sequence)`:** if the payloads are
  **byte-identical**, ignore the later one as a duplicate. If they **differ**, that is
  root-key equivocation and a fold must **refuse**, never pick — DP26's rule that a derived
  view may refuse on its own authority and never permit on it. This turns a conflicting
  sequence into a *detected* fault instead of an ambiguity resolved by luck.

  ⛔ **The bytes-identical branch is not a nicety, and an earlier draft of this rule omitted
  it and was a keyless denial of service.** The draft said "two *distinct* delegations …
  refuse", with the whole weight on *distinct*. But a replay is byte-identical **by
  construction** — that is the property `sequence` exists because of. So a fold keying on
  `(origin, role, sequence)` and refusing on *any* duplicate hands the same keyless attacker
  a permanent refusal in place of a rollback: re-append the **current** delegation and the
  fold refuses forever.

  **This is the second-order form of the defect the review had just fixed**, in the sentence
  written to fix it — the attacker keeps the same capability (append public bytes) and the
  outcome merely changes from rollback to denial. A remedy that preserves the attacker's
  reach has moved the failure, not removed it.
- **`not_before_unix_ms` still orders nothing.** It remains an effectivity claim, so a
  delegation can be prepared offline and take effect later.

**This is why the format was built last and reviewed before freezing.** The defect is in the
payload, so it is free to fix now and impossible to fix afterwards.

### 2.3 Unknown roles are REJECTED, not carried

`role` decodes to a closed enum; `1` is `Operational` and every other value is a decode
failure.

The alternative — accept and pass through — produces the exact failure recorded in this
repo's live state: **a signed unchecked value looks checked.** A consumer that sees a
verified delegation with `role: 7` has been handed a cryptographically perfect artifact
whose meaning nobody in the system defines, and the signature raises its apparent
authority while its semantics stay empty. Adding a role later is a `v2`; that is what
versioned wire contracts are for.

### 2.4 Four things this format deliberately does not answer

Written here so their absence cannot later read as "nearly done":

1. **Which delegation is current.** That is a fold over the log, and BUILD-PLAN F4
   establishes there is no slot in `lys/verification-bundle/v1` for a delegation or its
   inclusion proof. The key-history *artifact* is deferred to its own v1.
2. **Whether the root key is trustworthy.** Nothing attests it. DP14 is explicit: a verifier
   is always *told* whom to trust, or we have rebuilt a CA. §3.2 is where this becomes a
   hard API requirement.
3. **Root-key succession.** `DelegationRole` has one inhabitant, so `v1` cannot say "this new
   root key succeeds the old one", and a `Root` role is **deliberately not reserved**. Two
   reasons, and the first is the one that makes the omission correct rather than merely
   accepted: **root compromise is unrecoverable by any in-band mechanism** — a delegation
   signed by a compromised root naming a successor is one the attacker can also write, so
   succession after compromise always requires out-of-band trust. And *planned* succession
   needs the deferred key-history fold to define its consumption; reserving a role whose
   semantics nothing implements would be a signed value that looks checked and is not, which
   is exactly what §2.3 refuses for unknown roles. A `v2` defines the role and its fold
   together, or neither.
4. **Expiry.** There is no `not_after`. DP26 rules freshness tolerance an **input to the
   verify call** with no default — an expiry baked into the artifact would be exactly the
   default DP26 refuses, frozen into bytes that outlive the decision.

---

## 3. The adversarial requirements — these are the security content

Each is a rule; §6 requires one drift injection per rule, isolated so that exactly the
check built for it fails.

### 3.1 Cross-format confusion must fail

The signing preimage begins `0x84 0x6A "Signature1"` — **identical** to a receipt's, a
consistency receipt's and a `lys/attestation/v2` attestation's. Byte-0 disjointness does
*not* separate them; `receipt/sign.rs` says so in as many words and it would be a mistake
to re-derive the comfort.

What separates them is the content type *inside* the signed bytes, and each verifier
pinning its own. Requirement: `verify_delegation` pins
`application/vnd.lys.anchor-delegation.v1+cbor` before it examines anything else, and no
delegation is accepted by `verify_receipt` / attestation verification, or vice versa.

### 3.2 ⚠️ THE CENTRAL TRAP — `kid` is not an authority, it is a claim

A delegation carries the root key in its own protected header, so it verifies against
whatever key it carries. **An attacker signs a delegation for origin `O` with their own
root key, puts their own key in `kid`, and it is cryptographically perfect.** It vouches for
nothing, and a verifier that reads the key out of the artifact will accept it.

Requirement: **`verify_delegation` takes the expected root key as a required argument** and
enforces `kid == expected`. There is deliberately no "just verify this" entry point. A
parse-only path may exist and must be named for parsing, exactly as
`AnchorReceipt::from_cose_bytes` is.

### 3.3 Cross-origin replay must fail

Without an origin check, a delegation issued for anchor A is a valid delegation for anchor
B whenever B's operator holds the same root key — or, worse, whenever a verifier neglects
to notice which origin it is looking at.

Requirement: `verify_delegation` takes the expected origin as a required argument and
enforces equality against the **payload** origin. The origin is in the signature-covered
payload, which is also DP15 working as intended: a runtime-configured value reaching signed
bytes, never a committed constant.

### 3.4 Canonical-encoding strictness

Decode with `ciborium`, enforce the exact shape, **re-encode the extracted fields and
require byte-identity with the input.** This is the shipped pattern
(`receipt/encoding.rs`), and it is what makes "deterministic CBOR" enforced rather than
merely intended: without it, a non-canonical re-encoding of the same fields is a second
valid artifact for one statement.

RFC 9052 §9's own restrictions are definite lengths, minimum-length arguments, and no
duplicate labels — **map key *ordering* is not among them**, and they are scoped to the
`Sig_structure`, `Enc_structure` and `MAC_structure` rather than to the headers. Canonical
ordering here comes from lys electing RFC 8949 §4.2, so a decoder conforming to RFC 9052
alone would accept a permuted protected map as valid COSE.

#### ⛔ CORRECTED — what the byte-compare actually guards, measured by removing it

An earlier draft of this section concluded from the paragraph above that the
re-encode-and-compare is *"the only thing that refuses a permuted map."* **That is false for
this implementation, and the adversarial review proved it by deleting the check.** With the
byte-compare disabled, exactly five tests fail — and `a_permuted_payload_map_is_refused` and
`a_reordered_protected_map_is_refused`, the two tests §6.2 designates for this rule, **both
still pass.** `decode_protected` and `decode_payload` use positional slice patterns with
pinned integer labels, so a permuted map dies at decode regardless.

**The real mechanism, which nobody had written down and which is a better argument:**
`verify_delegation` re-derives the preimage from the **parsed fields**, not from the wire's
protected and payload bytes. So any non-canonicality *inside* the two `bstr`s is already
caught by the signature check. The byte-compare's actual and only job is **envelope**
canonicality — the tag head, indefinite-length forms in the outer array and the `bstr`s, and
trailing garbage. Removing it flips exactly four artifacts from refused to accepted: a
non-minimal tag head, an indefinite outer array, an indefinite payload `bstr`, and trailing
garbage.

So the check stays, its justification changes, and §6.2's row for it is retargeted from
"permuted map keys" — a case it does not guard — to "indefinite-length outer array". **A
rule defended by the wrong argument survives exactly until someone checks the argument.**

Two corrections to the correction, both from the independent encoder, and the first is the
more important thing on this page:

- ⛔ **The argument above is true of *lys's* verifier and does not transfer to a stranger's,
  which is the only kind that matters here.** "Non-canonicality inside the two `bstr`s is
  caught by the signature check" holds because `verify_delegation` re-derives the preimage
  from *parsed fields*. **A verifier built the RFC-literal way does the opposite:** RFC 9052
  §4.4 step 2 takes the protected attributes *from the body structure* — the wire bytes,
  verbatim — so it signs and checks the same non-canonical bytes and the signature check
  catches nothing. `go-cose` is such a verifier. To every conforming stranger, then, a
  permuted protected map **is** a second valid artifact for one statement, and lys refuses it
  only because of an implementation strategy this document never mandated.

  **Therefore canonical encoding is normative ON THE WIRE**, stated here as a requirement on
  the artifact rather than inferred from how lys happens to verify. §1.2 already took this
  care with the 4096-byte cap; the same care is owed here, and for a stronger reason —
  a divergence in canonicality is a malleability, not just an interop limit. **An
  implementation strategy is not a specification, and a property that only holds because of
  one is a property strangers do not have.**
- The claim "removing it flips **exactly four** artifacts from refused to accepted" is the
  set the current suite *exercises*, not a proven exhaustive set of what the check guards.
  Untested neighbours include an indefinite-length protected `bstr`, an indefinite-length
  unprotected map (`bf ff`), non-minimal length heads on any of the three `bstr`s, and a
  non-minimal outer-array head. Stated as a bound on the evidence, because "exactly four"
  reads as exhaustive and nothing established that.

A caveat for any future `v2`: §1.1's ordering argument holds because every label is
non-negative. **A negative label would break the coincidence** — COSE reserves negative
labels for algorithm-specific parameters, and CBOR major type 1 encodings sort *after* every
major type 0 one, so a negative label lands last under bytewise order and in numeric
position under numeric order. Two implementations would then silently disagree.

### 3.5 Non-oracle failure

Every decode and verification failure collapses to a single error value carrying no
information about which check fired. A delegation is examined by strangers; a distinct
variant per failure is a free oracle for probing what a verifier knows.

### 3.5.1 ⛔ Non-oracle means the CALL, not just the returned value

**Added after the adversarial review measured a 32.8× timing separation** between "your
`kid` is not the root key I trust" / "your origin is not mine" and "your signature is bad" —
because the first two return before `Ed25519Identity::verify` runs, and Ed25519 verification
dominates everything preceding it. Round-robin sampling, medians, with the first arm repeated
last as a control that reproduced to 1.001×.

A delegation is examined by strangers, so this answers *"is this verifier configured to trust
root key K, for origin O?"* to anyone who can hand it bytes and a stopwatch. It does not
brute-force a key; it **confirms a guess**, and for a public anchor the candidate set is
small.

**Requirement: the signature verification runs unconditionally**, and the `kid`, origin and
signature results are combined into a single decision at the end. Returning early is the
optimisation that costs the property.

The general rule, which this repo had stated only about error *values*: **a collapsed error
type is not a non-oracle if the amount of work done differs per cause.** An error enum is
observable in one channel; a function is observable in several.

### 3.6 Signature must be strict Ed25519

Verification goes through `Ed25519Identity::verify` (strict), inheriting the existing
malleability and small-order rejections rather than restating them.

---

## 4. API shape

Two-phase, with a convenience wrapper implemented **in terms of** the two phases:

```text
delegation_preimage(fields)            -> the Sig_structure bytes to be signed
assemble_delegation(fields, signature) -> the tagged COSE_Sign1 bytes
sign_delegation(&Ed25519Identity, fields) -> tagged COSE_Sign1 bytes   [wrapper]
verify_delegation(bytes, expected_root_key, expected_origin) -> AnchorDelegation
```

**The root key is an argument, not a claim field.** `delegation_preimage` takes the root key
*separately* from the four payload fields, rather than bundling all five into one struct. The
root key is the signer's identity; the claim is what the signer says. Carrying it in both
places would create a value whose `kid` and whose signing identity could disagree, and a
struct that can be internally inconsistent is one somebody will eventually make inconsistent.

The split is not a third pattern for its own sake. **The root key is the offline key** —
DP16 gives it exactly one job, signing delegations into the log, precisely so it can live
somewhere the operational key cannot. Genesis is therefore the one signing operation an
operator would most want to perform on an air-gapped machine, and a two-phase API is what
makes that possible without `lys-core` becoming signer-generic.

`assemble_delegation` **must verify the supplied signature before returning**. Assembling an
artifact whose signature does not match its own bytes produces a file that fails verification
at some later, less debuggable moment.

---

## 5. Genesis-as-delegation (the second half of increment 11)

⛔ **CORRECTED — this section said "`Anchor::create` … changes to build leaf 0 as a
delegation", and that substitution is impossible. §0 and §5 could not both be satisfied, and
§5 never noticed.** Found by building it.

§0 puts the format behind `unstable-anchor` **in order to keep it changeable**. But
**genesis is the one code path a default build cannot do without** — `LeafStore` has no
insert, so a log created without leaf 0 can never be given one. So the gate that protects the
format *forbids* the substitution: replacing `create`'s body with a delegation build leaves a
default-features build unable to create any anchor at all, which is a worse form of the
"reachable API over an unreachable state" trap this build already fell into once.

**The resolution, and it is a genuine design finding rather than a workaround: the two
feature shapes need different genesis, and a default build cannot create a DP16-conformant
anchor. By construction, until the format is ratified and the gate comes off.**

- `Anchor::create(store, genesis: &[u8], …)` stays **ungated and byte-for-byte unchanged**.
- `Anchor::create_with_delegated_genesis(store, root_signer, not_before_unix_ms, …)` is a
  **second, differently-named** constructor behind `unstable-anchor`.

Two names, not one name with a `#[cfg]`-varying signature — that would be two functions
wearing one name, forking every call site and `standalone_is_complete.rs`, which is
deliberately `#[cfg]`-free so it compiles in both shapes. And emphatically not a constructor
that accepts a root signer and writes *something else* when the feature is off: that puts a
durable leaf 0 that looks like an intended delegation and is not one at the single position
that can never be corrected.

**Normative decisions this section previously left to a parenthetical or to silence:**

- **`sequence` is `0` for genesis**, fixed by the constructor, not a parameter. It was stated
  only as an aside in §6.1's *test-vector* rationale, which is not where a constructor's
  behaviour belongs. Not a parameter because genesis is the first delegation for its
  `(origin, role)` by construction, so any nonzero start opens a range below the first entry
  that nothing can ever write into — and a caller-chosen start is the same foot-gun §1.2
  closes at `u64::MAX`.
- **`not_before_unix_ms` is a parameter and no clock is read.** It is a claim by the signer,
  and the signer is the caller holding the root key, not the library. The argument that
  settles it is the field's own purpose: `not_before` exists so a delegation can be prepared
  **offline to take effect later**, which a clock read at creation time forbids outright.
- **The origin comes from `LeafStore::origin()`.** No new config field, no constant (DP15).
- **Ordering inside the constructor is an invariant:** check-empty → build → sign → assemble
  (which verifies) → append. Leaf 0 cannot be replaced, so a signer that declines must leave
  an **empty** log that can still be given genesis later.

⭐ **An unexpected result worth keeping: the root-signer bound is `Signer`, not
`InProcessSigner`.** Because the constructor goes through `delegation_preimage` +
`Signer::sign` + `assemble_delegation` rather than the `sign_delegation` convenience, **an
offline or remote root signer can issue an anchor's genesis today**, with no
`Ed25519Identity` anywhere in the path. `keys/signer.rs` had said "nothing in this crate
calls `sign` yet"; that is now false and was corrected. The asymmetry *is* the custody story:
the key that must stay online is the one the crate has to hold, and the root key — the one an
operator most wants air-gapped — is the one it does not. **An entry point designed for absent
key material passes a custody boundary for free.**

**No delegation may be signed outside tests (DP17)** — and the exposure is now one flag wide:
nothing in `lys-anchor-cli` reaches the new constructor, so the moment `anchor init` grows a
`--root-key`, DP17 is violated by a command rather than by a library call.

**Known limit, named rather than papered over: the DP16 invariant holds only at creation.**
`Anchor::open` never inspects leaf 0, so a store created by a default build and later opened
by an all-features build is a non-DP16 anchor and nothing flags it. Checking at open would
need a root key argument and, done properly, the deferred key-history fold.

---

## 6. The second parties — required before this lands

A round trip through our own encoder and decoder proves nothing about the wire: any
symmetric change is invisible. Three independent parties, each on a named axis:

1. **An independent encoder, written from this document and never from the Rust.** It emits
   the protected bytes, the payload bytes and the `Sig_structure` preimage as hex for a
   fixed vector. Axis: *encoding*. This is the party that catches a field order, a head
   width, or a map-vs-array mistake.
2. **`veraison/go-cose`** verifying the assembled artifact, and returning the signed bytes
   **verbatim**. Axis: *envelope + algorithm*. Verbatim return is not decoration — it is
   what caught the increment-3 defect where an anchor signed `body ‖ "extension\n"` and all
   18 in-crate tests stayed green. Reproduced here: appending `0x0a` to the preimage left
   **44 of 46 in-crate delegation tests green**, and neither failure was about the preimage;
   go-cose failed it instantly.

   ⚠️ **But a verbatim return is structurally blind to drift *inside* the bytes both parties
   read**, and this was found the only way it could be — by injecting the drift and watching
   the gate pass. A permuted payload map reaches the artifact *and* the preimage the gate
   compares against, so go-cose hands the same permuted payload back and **both sides agree
   about the wrong encoding**. Verbatim return proves the two parties read the same bytes;
   it cannot prove those bytes are canonical, because canonicality is a property of the
   encoding rather than of the agreement.

   The gate therefore also re-encodes both signature-covered maps through
   `fxamacker/cbor`'s `CoreDetEncOptions` and requires byte-identity — an external second
   party for §3.4, which RFC 9052 §9 does not mandate and so no COSE library enforces.
   **The general form: "two implementations agreed" is only as strong as the property the
   agreement is sensitive to.**
3. **A golden vector hardcoded as a literal in the test**, never imported from the constant
   it checks, so it cannot move with the code. This is the `tests/seal_derivation.rs`
   pattern and WIRE-FORMATS records why: reversing two public keys in the seal `info`, and
   changing the domain tag, each left the entire suite green.

### 6.1 The fixed test vector

Chosen here so that both implementations receive identical inputs from a third document:

| field | value |
|---|---|
| root seed (Ed25519, RFC 8032) | `00 01 02 … 1f` (bytes 0–31) |
| root **public** key (= `kid`) | `03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8` |
| delegated public key | `29acbae141bccaf0b22e1a94d34d0bc7361e526d0bfe12c89794bc9322966dd7` |
| — derived from seed | `20 21 22 … 3f` (bytes 32–63) |
| origin | `example.test` |
| role | `1` (operational) |
| `not_before_unix_ms` | `1700000000000` |
| `sequence` | `300` |

⚠️ **The published vector is INVALIDATED by `sequence` and must be regenerated by both
parties independently, exactly as the first one was.** A vector regenerated by one side and
copied by the other is one party agreeing with itself.

`sequence = 300` is chosen to expose bugs rather than to look natural. It is **not** `0`,
because a zero field is indistinguishable from a field an implementation forgot to write and
defaulted; it is **not** `1`, because `role` is `1` and two fields sharing a value mask a
field-swap; and `300` needs a **2-byte** head — `19012c`, and ⛔ **an earlier draft of this line wrote
`190130`, which is 304.** I hand-wrote that literal and never recomputed it; the independent
encoder caught it. It is the golden-literal-nobody-rechecks failure this whole exercise
exists to catch, and it landed in the one field chosen *because* it is head-width-sensitive.
A third head width
alongside `not_before`'s 8-byte and `role`'s inline — so a head-width bug has nowhere to
hide. A genesis delegation would in practice carry `sequence = 0`; the vector is not
obliged to be a genesis one, and is more useful for not being.

`example.test` is a reserved name and **not** the configured production origin. DP15 forbids
a committed origin constant; a test vector carrying the real origin would be one.

### 6.2 Drift injections — one case per rule

Each must fail **exactly one** check, and it must be the check built for it. A drift that
fails five tests proves the suite is alive, not that the rule is guarded.

#### ⚠️ Amended: at `verify_delegation` level this is NOT achievable, and the reason is structural

Found by the implementation, and it is a better observation than the requirement it
corrects. **§3.4's re-encode-and-byte-compare covers for four of the seven injections.**
The canonical re-encoding always emits the right content type, an empty unprotected bucket,
a known role and the tag — so deleting the content-type pin entirely leaves the suite green,
because the byte-compare rejects the artifact anyway, for its own reason.

That is a rule guarded twice and therefore, by this repo's own standard, **proven by
neither**: the obvious case cannot distinguish which check is doing the work. The remedy is
not to weaken the requirement but to move the isolating tests *below* the byte-compare, at
`decode_fields` level, where each rule is the only thing that can fire.

The mirror image also holds and is worth stating so nobody re-derives it: the **permuted-map**
injection is itself double-guarded — positional decode pins it as well as the byte-compare —
so §3.4's isolation actually comes from trailing garbage, indefinite lengths and non-minimal
heads, *not* from the permutation case this table names.

The general form, already in this repo's memory: **"exactly one test fails" counts tests,
while the noun that matters is rules.** A rule with two guards and a guard with two rules
both defeat the count, in opposite directions.

| injection | must be caught by |
|---|---|
| content type changed to the receipt's | §3.1 |
| `kid` replaced with the attacker's root key | §3.2 |
| payload origin changed | §3.3 |
| ~~payload map keys permuted~~ → **indefinite-length outer array** | §3.4 — retargeted; the permuted case is guarded by positional decode, not by the byte-compare |
| a byte flipped in the signature | §3.6 |
| unprotected header carries one entry | §1.3 |
| `role` set to `7` | §2.3 |
| **a replayed earlier delegation supersedes the current one** | §2.2 |
| **the early return on a `kid` mismatch is restored** | §3.5.1 |

The last two are new and neither is an encoding rule, which is the point: **the two
severest findings of the review were a semantic defect and a channel outside the return
value.** An injection table built only from encoding rules would have found neither.
