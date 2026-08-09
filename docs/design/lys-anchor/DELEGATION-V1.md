# `lys/delegation/v1` — the specification, written before the implementation

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

## 0.5 🟢 RULED 2026-08-08 — `origin` BECOMES a typed `subject`. Tom: *"we'll go with your recommendation there."*

**Decided inside the free window, which was the entire point.** The record of the decision and
its reasoning follows; §1.1, §1.2, §2, §3.3 and §6.1 are revised to match.

**Four consequences I decided rather than asked about, flagged so they can be vetoed:**

1. **The content type is renamed** — the old name was `vnd.lys.` + `anchor-delegation.v1+cbor`
   (45 bytes) and the new one is `application/vnd.lys.delegation.v1+cbor` (38 bytes, head
   `7826`). *(A blind search-and-replace across this file briefly rewrote both sides of that
   arrow to the new name, destroying the record of what changed — the exact "corrections are
   recorded, not silently applied" failure this document argues for elsewhere. Restored, and
   the old name is deliberately written in two pieces above so no future replace can eat it.)*
   A format that now serves seats should not be
   called *anchor*-delegation in a signature-covered field that freezes on publication. The
   Rust module was already named `lys_core::delegation`; only the wire tag said "anchor".
   Renaming is free today and a permanent misnomer tomorrow.

   **The format's prose name changes with it: `lys/anchor-` + `delegation/v1` becomes
   `lys/delegation/v1`.** Caught late, on a sweep after the content type had already been
   renamed — the title of this document and the freeze list in `CLAUDE.md` were both still using
   the old name, so **the wire name and the documented name had diverged**. That is worse than
   either name alone: `CLAUDE.md` is the authoritative record of which formats have frozen, and a
   reader comparing it against an artifact could not tell which of the two names was the one that
   froze. Unlike the content type this name is *not* on the wire, so renaming it costs nothing
   and carries no compatibility weight — the only thing at stake was whether the record is
   self-consistent, which is the entire job of a freeze list.
2. **Both subject kinds ship in v1, not one.** Unknown kinds must be rejected (§2.3's rule),
   so **adding a kind later is a compatibility event** — an old verifier would refuse a seat
   delegation. Shipping the mechanism with a single kind would therefore still require a v2 for
   the second one, defeating the whole reason for deciding now.
3. **`role`'s vocabulary is scoped per kind, and `(kind, role)` is validated as a pair.** Two
   roles ship — one defined per kind — so `role` carries no information a reader could not get
   from `kind`, which makes the field *look* redundant. It is not: the axes are genuinely
   independent (a domain could later want a witness or a revocation role), so the field stays
   and the **pair** is what is checked. That is §0.5.1's cross-kind argument made executable
   rather than documented.
4. ⛔ **The role vocabulary starts at `2`, not at `1`, and this is a freeze-time decision found
   by the independent encoder.** With kinds `{1 = domain, 2 = seat}` and roles numbered from
   `1`, **every valid pair would have `subject_kind == role`** — `(1,1)` and `(2,2)`. An
   implementation that wired label `1` to its role field and label `4` to its kind field would
   then emit **byte-identical bytes for every valid `v1` artifact**, so no test, no golden
   vector and no second encoder could detect the swap. It would stay latent until a `v2`
   introduced a pair whose halves differ, at which point every `v1`-era implementation would be
   revealed to have had the fields crossed all along.

   **This is not fixable by choosing a better test vector** — it is a property of the numbering,
   and the numbering freezes. So roles are `2 = operational` and `3 = speaks_for`, making the
   valid pairs `(1,2)` and `(2,3)`. Two properties follow: no valid pair has `kind == role`, so
   a swap is visible in the bytes of *every* case rather than none; and each swap yields an
   **invalid** pair (`(2,1)`, `(3,2)`), so it is refused at decode rather than merely encoded
   differently. **The gap between the two vocabularies is load-bearing and must not be "tidied"
   back to a tenth of a byte of elegance.**

**The admitted cost, in the same terms §2.2 admits `sequence`'s:** the seat role has defined
semantics and **no implementation and no consumer** — manifold confirmed it is building
nothing and set no deadline. That is the §2.3 shape (a value whose check lives elsewhere), and
it is accepted for the same reason `sequence` was: it must exist *before* the freeze and cannot
be consumed until after, so the alternative is a format that can never express the case at all.
What makes it admissible rather than speculative is that the semantics are written here, and
that the second shape was **stated by the operator** — agents and people sharing a member —
rather than guessed at by me.

**And the requirement is real but not urgent, which changes why this is being done.** manifold
needs nothing from lys and is waiting on nothing. So this generalisation is justified on lys's
own merits plus one operator-stated future consumer — not by a dependency. If the sibling
format had been cleaner, that was a defensible answer.

### The superseded reasoning, kept because deleting it hides that the design moved

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
       "application/vnd.lys.delegation.v1+cbor"               /   38 bytes           /
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

Deterministic CBOR map, keys ascending `1, 2, 3, 4, 5, 6`:

```text
{
  1 : subject_kind            (uint)         / closed enum: 1 = domain, 2 = seat      /
  2 : subject_value           (tstr)         / "example.test" | a seat identifier     /
  3 : delegated public key    (bstr .size 32)/ the key being delegated TO             /
  4 : role                    (uint)         / closed enum, scoped per kind. §2.3.    /
  5 : not_before_unix_ms      (uint)         / a CLAIM by the signer, not an order key/
  6 : sequence                (uint)         / strictly incr. per (kind,value,role)/
}
```

**The payload is renumbered rather than appended to**, because nothing is published and a map
whose reading order does not match its meaning is a permanent small tax on every reader. The
kind precedes the value it types.

**`subject_kind` and `role` are validated as a PAIR, not independently:**

| `subject_kind` | valid `role` | meaning |
|---|---|---|
| `1` domain | `2` operational | the key signs checkpoints and receipts for this domain |
| `2` seat | `3` speaks-for | the key may sign on behalf of this seat |

Any other pair is **rejected at decode**, and any unrecognised value of either field likewise
(§2.3). A domain delegation therefore cannot be presented to a seat verifier and vice versa —
the same discipline §3.1 applies *between* formats, applied *within* one, and structural rather
than documented.

⛔ **The two vocabularies deliberately do not overlap: there is no `role = 1`, and a wire `role`
of `1` is a decode failure.** §0.5 consequence 4 carries the argument — numbering the roles from
`1` would make `subject_kind == role` in every valid pair and render a swap of the two fields
undetectable in the encoded bytes, permanently. The consequence to keep in view here is that the
column above is not merely a list of accepted pairs: **each single-field error and each field
swap lands outside the table**, so the pair check catches wiring mistakes with the same
machinery that catches semantic ones.

⚠️ **`role = 3` (speaks-for) has defined semantics, no implementation and no consumer.** See
§0.5's admitted cost. It is here because adding it later would be a compatibility event, not
because anything reads it today.

#### `subject_value` is scoped by the AUTHORITY key, not by a global namespace

Worth stating because the opposite assumption is the natural one and it would send somebody
off to build a registry that is not needed.

**A subject value does not have to be globally unique.** For `kind = seat` it will typically be
a local name — a seat within a member, where several seats share one member because agents and
people do. Two different members may each have a seat called the same thing, and both
delegations are well-formed.

That is safe, and it is safe for §3.2's reason rather than by luck: **`verify_delegation`
requires the expected authority key as well as the expected subject**, so a delegation for
member B's seat `x` fails against a verifier holding member A's key, whatever the subject says.
The uniqueness that matters is of the **pair** `(authority key, subject value)`, and the
authority half is pinned outside this format by whatever the verifier already trusts — a
registry, a configuration file, an operator's decision.

Two consequences:

- **This format defines no structure for `subject_value` and will not.** It is opaque UTF-8
  compared byte-for-byte (§1.2). A consumer that wants member-qualified names builds them in
  the string; lys neither imposes nor parses a separator, because a delimiter in a signed
  identifier is a parsing decision that would then be frozen alongside it.
- **A verifier that checks the subject and not the authority has no security at all** — it
  would accept anyone's delegation for a name it recognises. Which is why the API takes both
  and offers no way to pass only one.

**`sequence` was added by the adversarial review, and it is the reason the review happened.**
See §2.2 — without it this format is replayable by an attacker holding no key material.
It is a **`u64`, shortest-form head**, on exactly the reasoning §1.2 gives for `not_before`:
without the bound stated, two conforming implementations disagree again at ≥ 2⁶³.

**`sequence == u64::MAX` is REJECTED**, on both encode and decode. Strictly-increasing has no
successor at the maximum, so a signer who issues there can never supersede it and the key is
frozen for the life of the subject — the format would permit a delegation that permanently
disables its own rotation. Not attacker-reachable (only the root key signs), so this is a
self-inflicted foot-gun rather than a vulnerability; it is forbidden anyway because it costs
one comparison now and is impossible to add after the freeze. **Rejecting the maximum is what
makes "a successor always exists" a property of the format rather than of operator care.**

Two questions `sequence` leaves open, recorded so they are decided deliberately rather than
by whoever writes the code:

- **The counter restarts per role — decided, not left open — and nothing exercises the second
  role.** The partition is `(subject_kind, subject_value, role)`: a `speaks_for` delegation and
  an `operational` one for the same subject do not share a counter. The **kind** is in the
  partition deliberately, because a domain and a seat whose identifier strings happen to collide
  would otherwise share one counter, which is the exact collision the typed subject exists to
  stop leaving to chance. This was an open question while `role` had one inhabitant; with two it
  is a compatibility fact about *already-signed* `v1` entries and had to be settled before the
  freeze rather than alongside a later role. **It is settled here in prose and enforced
  nowhere** — the fold that reads it does not exist yet (§2.4.1), and no consumer emits a
  `speaks_for` delegation to disagree with it.
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

**An empty `subject_value` is REJECTED.** Not because the empty string is malformed, but
because of what it does to a *misconfigured verifier*: §3.3 makes acceptance a comparison
against the caller's configured subject, so a verifier whose subject is unset would match a
delegation carrying an empty value and accept it. DP15 already forbids a default origin;
refusing the empty string at decode is the same rule enforced from the other side, and it makes
that misconfiguration fail closed instead of silently succeeding. **The rule generalises to
seats without weakening**: an unset seat identifier is exactly as dangerous as an unset origin,
and arguably likelier, since a seat name has no DNS registration to make its absence obvious.

**The `4096`-byte cap is on the ARTIFACT, not on `subject_value`**, mirroring the receipt's. It
is enforced on **both encode and decode**, on the same reasoning §1.2 gives for `u64::MAX`.
`subject_value` is bounded only transitively.

⛔⛔ **CORRECTED 2026-08-09 — this paragraph used to say "the derived maximum is `3885` bytes —
every other field is fixed-width", and BOTH HALVES ARE FALSE.** `not_before_unix_ms` and
`sequence` are `u64` in shortest form, so each occupies **1 to 9 bytes** depending on its
value. They are not fixed-width, and the maximum `subject_value` is therefore **not a
constant**. `3885` is the maximum *for vector A's field values specifically*, and the sentence
promoted one instance to a law.

**The maximum is a function of the other fields' encoded widths:**

```text
artifact = 150 + head(payload_len) + payload_len
payload_len = 45 + width(not_before) + width(sequence) + head(L) + L
```

where `width(v)` is the shortest-form encoded size of `v` (1, 2, 3, 5 or 9 bytes) and `head(n)`
is the CBOR head size for a length of `n`. The largest `L` satisfying `artifact ≤ 4096` is the
maximum, and it **moves with the other two fields**:

| `not_before` / `sequence` | widths | max `subject_value` |
|---|---|---|
| vector A (`1700000000000`, `300`) | 9 + 3 | **3885** |
| vector D (`23`, `0`) — both inline | 1 + 1 | **3895** |
| both near `u64::MAX` | 9 + 9 | **3879** |

⭐ **Sixteen lengths on which two conforming implementations disagree**, in *both* directions —
which is the same defect class the correction below was written to fix, reintroduced by the
sentence that fixed it. An implementation enforcing `L ≤ 3885` as a constant:

- **refuses `3886`–`3895`** — ten lengths this crate issues valid artifacts at;
- **accepts `3879`–`3884`** — six lengths at which the artifact overflows the cap, reaching up
  to `4102` bytes, and then fails every verification afterwards.

**`lys-core` is CORRECT and only the specification was wrong.** `check_encodable` derives the
bound by encoding the artifact and measuring it (`encoding.rs:399` — *"Derived, never
hardcoded: the bound shifts whenever the payload gains a field"*), so no shipped code carries
the constant. That is the opposite direction from this document's worst previous error, where
the prose was laxer than the code — here the prose is **wrong in a way that makes a faithful
implementer diverge from us**.

⚠️ **An independent party instantiated the predicted defect while believing it was following
the specification.** One of the two vector parties listed *"`subject_value` ≤ 3885 bytes (§1.2
derived max)"* among the rules it enforced during construction — it read this paragraph, took
`3885` as a constant, and hard-coded it. The prediction and its confirmation arrived in the
same exercise, from parties that never communicated.

**A conforming implementation MUST derive this bound rather than hard-code any number in the
table above**, including `3885`. The table exists to make the variation visible, not to be
copied.

⛔ **The referent used to be ambiguous, and the ambiguity was wire-visible — in the paragraph
written to prevent exactly that.** The old wording, *"`subject_value` is bounded by the artifact
size cap, which is 4096 bytes"*, reads either as capping the artifact or as capping the value.
The two readings disagree about **211 lengths** (`3886`–`4096`): one conforming implementation
accepts what the other refuses, for artifacts that are well-formed under this grammar either way.
The paragraph's own stated purpose was that any divergence be *"a specified limit rather than a
surprise found by a stranger"* — which it cannot achieve while the limit's referent is unstated.
Found by the independent encoder, which computed both boundaries rather than reading the sentence
twice; **an ambiguity is invisible to a reader who resolves it on first pass, and every reader
resolves it the same way they wrote it.**

⛔ **This paragraph used to end: *"The maximum is stated as a number above rather than left as a
derivation, because a limit each implementation computes for itself is a limit each
implementation gets to compute differently."* That sentence is the direct cause of the error
corrected above**, and it is kept because the reasoning is seductive and half right.

It is true that an under-specified derivation lets implementations diverge. It does not follow
that a constant is the remedy, and **here the quantity simply is not constant** — so stating a
number did not remove the divergence, it *guaranteed* one, in the direction of a faithful
implementer disagreeing with us. The correct remedy for an ambiguous derivation is a
**specified** derivation: the formula and the worked table above, which any implementation can
evaluate and none can resolve two ways.

⭐ **Replacing a derivation with a constant is only safe when you have checked that the
quantity is invariant** — and the sentence that did it asserted the invariance ("every other
field is fixed-width") in the same breath, from the same author, with nothing able to
disagree. The `211 lengths` figure in the correction above has the same flaw for the same
reason: its `3886` lower bound is A-specific, so the true disputed range moves with
`not_before` and `sequence` too.

**`subject_value` comparison is raw UTF-8 byte equality** (§3.3). Not case-folded, not
Unicode-normalised, no trailing-dot or port or scheme handling for a domain, and no
case-insensitivity for a seat. The test vector is ASCII and so cannot expose this; an IDN
origin would. Byte equality is chosen because every other rule is a normalisation rule that two
implementations can apply differently, and a comparison that differs between verifiers is a
comparison an attacker picks the side of. **Typing the subject does not soften this**: a seat
identifier is arbitrary text chosen by a consumer, so it has *more* room for two
normalisations to disagree than a domain does, not less.

#### Two well-formedness questions this section previously left open

Both were found by the independent encoder, both are constructible, and both are the same defect
class: **a value the grammar admits and the prose never rules on, so two conforming
implementations disagree about a well-formed artifact.** Neither is exploitable; both freeze.

**1. `subject_value` MUST be valid UTF-8, and invalid UTF-8 is a decode failure.** CBOR major
type 3 is *defined* as UTF-8, but RFC 8949 §4.2 does not require a decoder to validate it and
RFC 9052 §9 does not either. So "opaque UTF-8 compared byte-for-byte" (above) invites a
byte-oriented decoder to skip validation entirely: it would accept an artifact that a decoder
reading into a Rust `String` rejects, and **§3.4's re-encode-and-byte-compare passes in both
cases**, because re-encoding invalid bytes reproduces them faithfully. Requiring validity picks
the reading that matches CBOR's own definition of the type. *Opaque* constrains what a verifier
may **interpret** — it never licensed skipping the type's own validity rule.

**2. BOTH 32-byte key slots MUST be usable Ed25519 public keys, and a decoder must reject one
that is not.** That means the payload's delegated key (label 3) *and* the protected header's
`kid`. Concretely, three classes are refused at decode: a **non-canonical `y`** (a value `>= p`),
the **all-zero** string, and any **small-order** point including the identity.

⛔ **CORRECTED SAME DAY — this subsection first said the exact opposite, and it was the most
dangerous thing written into this document.** It stated that the delegated key "is an opaque
32-byte string and is NOT validated as an Ed25519 point at decode," and justified that with:
*"the available validation is `dalek`'s, which reduces `y` mod `p` rather than rejecting
`y >= p`."*

**The premise was false, and the code disproving it was already committed.**
`encoding.rs` calls `is_usable_ed25519_public_key`, which performs an explicit canonical-`y`
check **precisely because** `dalek`'s `from_bytes([0xff; 32])` succeeds and `is_weak()` returns
false. The paragraph declined a rule on the grounds that the check could not be built, in a crate
whose sibling module documents having built it. Found by the adversarial review, which measured
all three classes refused.

**Three things went wrong, and the third is the general one:**

1. **A real finding was generalised past its subject.** `Ed25519Identity::verify` does accept
   non-canonical keys — that is measured, recorded and still open. But it is a *different
   function*, and "dalek is the only available validation" was inferred rather than checked.
2. **The direction was the dangerous one.** A stranger implementing the ratified text would
   **accept** artifacts lys **refuses** — "two conforming implementations disagreeing about a
   well-formed artifact", the exact defect class the sibling half of this subsection (invalid
   UTF-8) was written to eliminate. That half matched the code; only this one was backwards.
3. ⭐ **A specification asserting what the code does, written without reading the code, is a
   second party that has quietly become a first one.** This document's whole authority is that it
   was written *before* the implementation and can therefore disagree with it. A paragraph
   describing shipped behaviour from memory keeps that authority in its tone and loses it in
   substance — and it is *more* dangerous than an ordinary error, because it is normative on the
   wire and readers have no reason to check it against the code.

The now-deleted consequence paragraph was false in the same direction: it told consumers that two
artifacts carrying different bytes for one point are two distinct valid delegations, and that a
consumer must canonicalise before comparing. **Against the shipped decoder the non-canonical
spelling never parses**, so that advice defended against a case the decoder already refuses.

Why validation is the right rule rather than merely the shipped one: a delegation naming an
unusable key is a claim that can never be acted on, so admitting it buys nothing and costs a
`v2` to remove later. And validating **both** slots keeps one rule over two identically-shaped
fields — the asymmetry was itself a finding (`kid` went unvalidated while label 3 did not, while
`encoding.rs`'s stated invariant claimed no non-Ed25519 key could occupy the slot; `[0xff; 32]`
did).

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

*"The holder of root key `K_root` states that, for subject `S`, key `K_op` holds role `R`
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

- **`sequence` (label 6) orders delegations**, strictly increasing per `(subject_kind, subject_value, role)`. A
  replay carries a sequence already superseded, so it loses; a replay of the *current*
  delegation is byte-identical to what is already current and changes nothing. **Replay
  becomes a no-op rather than a rollback.**
- **The log establishes existence and publication, not order.** DP16's "the key history *is*
  log history" survives intact: the log is still where the history is published and proven.
  It is simply not the sort key.
- **Two delegations at the same `(subject_kind, subject_value, role, sequence)`:** if the payloads are
  **byte-identical**, ignore the later one as a duplicate. If they **differ**, that is
  root-key equivocation and a fold must **refuse**, never pick — DP26's rule that a derived
  view may refuse on its own authority and never permit on it. This turns a conflicting
  sequence into a *detected* fault instead of an ambiguity resolved by luck.

  ⛔ **The bytes-identical branch is not a nicety, and an earlier draft of this rule omitted
  it and was a keyless denial of service.** The draft said "two *distinct* delegations …
  refuse", with the whole weight on *distinct*. But a replay is byte-identical **by
  construction** — that is the property `sequence` exists because of. So a fold keying on
  `(subject_kind, subject_value, role, sequence)` and refusing on *any* duplicate hands the same keyless attacker
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

### 2.3 Unknown values, and invalid `(kind, role)` pairs, are REJECTED — not carried

**Both fields are closed enums, and the pair is a closed set.** Three distinct decode failures,
and each has to be stated because each admits a different mistake:

1. `subject_kind` outside `{1 = domain, 2 = seat}` is a decode failure.
2. `role` outside `{2 = operational, 3 = speaks_for}` is a decode failure. **`1` is not a role**
   — see §0.5 consequence 4 for why the vocabularies are disjoint.
3. **A pair outside the §1.2 table is a decode failure even when both halves are individually
   recognised.** `(domain, speaks_for)` and `(seat, operational)` are the two such pairs, and
   this is the clause that makes cross-kind confusion structural rather than documented (§3.3).

⛔ **The earlier version of this section said "`1` is `Operational` and every other value is a
decode failure", which forbade the very role §0.5 ships.** It survived the generalisation
because renaming `origin` was the visible half of that change and renumbering the enums was
not — a stale normative sentence in the section that *defines* rejection, which is the worst
place for one, because an implementer following it would build a verifier that refuses valid
artifacts and passes every test written from the same stale text. **A stale document is a
confident source.**

The alternative to rejection — accept and pass through — produces the exact failure recorded in
this repo's live state: **a signed unchecked value looks checked.** A consumer that sees a
verified delegation with `role: 7` has been handed a cryptographically perfect artifact whose
meaning nobody in the system defines, and the signature raises its apparent authority while its
semantics stay empty. The same holds, more sharply, for a *recognised* value in the wrong pair:
`(seat, operational)` would read to a careless consumer as an operational key for something that
has no operations. Adding a kind, a role, or a pair later is a `v2`; that is what versioned wire
contracts are for.

### 2.4 Four things this format deliberately does not answer

Written here so their absence cannot later read as "nearly done":

1. **Which delegation is current.** That is a fold over the log, and BUILD-PLAN F4
   establishes there is no slot in `lys/verification-bundle/v1` for a delegation or its
   inclusion proof. The key-history *artifact* is deferred to its own v1.
2. **Whether the root key is trustworthy.** Nothing attests it. DP14 is explicit: a verifier
   is always *told* whom to trust, or we have rebuilt a CA. §3.2 is where this becomes a
   hard API requirement.
3. **Root-key succession.** `v1`'s two roles are `operational` and `speaks_for`; neither can say
   "this new root key succeeds the old one", and a `Root` role is **deliberately not reserved**
   — note that reserving one is now a *smaller* change than it was when the enum had a single
   inhabitant, which makes stating the refusal more necessary rather than less. Two
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
`application/vnd.lys.delegation.v1+cbor` before it examines anything else, and no
delegation is accepted by `verify_receipt` / attestation verification, or vice versa.

### 3.2 ⚠️ THE CENTRAL TRAP — `kid` is not an authority, it is a claim

A delegation carries the root key in its own protected header, so it verifies against
whatever key it carries. **An attacker signs a delegation for subject `S` with their own
root key, puts their own key in `kid`, and it is cryptographically perfect.** It vouches for
nothing, and a verifier that reads the key out of the artifact will accept it.

Requirement: **`verify_delegation` takes the expected root key as a required argument** and
enforces `kid == expected`. There is deliberately no "just verify this" entry point. A
parse-only path may exist and must be named for parsing, exactly as
`AnchorReceipt::from_cose_bytes` is.

### 3.3 Cross-subject and cross-kind replay must fail

Without a subject check, a delegation issued for anchor A is a valid delegation for anchor
B whenever B's operator holds the same root key — or, worse, whenever a verifier neglects
to notice which subject it is looking at.

Requirement: **`verify_delegation` takes the expected subject kind AND the expected subject
value as required arguments**, and enforces both against the signature-covered payload. The
value comparison is raw UTF-8 byte equality (§1.2). The subject is in the payload, which is
also DP15 working as intended: a runtime-configured value reaching signed bytes, never a
committed constant.

**The kind argument is not redundant with the value, and that is the point of typing it.** A
verifier that checked only the value would accept a seat delegation whose seat identifier
happened to equal an origin string — and since a seat identifier is arbitrary text chosen
elsewhere, that collision is an attacker's choice rather than an accident. Requiring the kind
makes the two namespaces disjoint by construction instead of by the hope that they never
overlap.

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
`kid` is not the root key I trust" / "your subject is not mine" and "your signature is bad" —
because the first two return before `Ed25519Identity::verify` runs, and Ed25519 verification
dominates everything preceding it. Round-robin sampling, medians, with the first arm repeated
last as a control that reproduced to 1.001×.

A delegation is examined by strangers, so this answers *"is this verifier configured to trust
root key K, for subject S?"* to anyone who can hand it bytes and a stopwatch. It does not
brute-force a key; it **confirms a guess**, and for a public anchor the candidate set is
small.

**Requirement: the signature verification runs unconditionally**, and the `kid`, subject-kind,
subject-value and signature results are combined into a single decision at the end. Returning
early is the optimisation that costs the property.

⚠️ **The typed subject widened this requirement**, so the requirement is stated over the whole
comparison set rather than over the two arms originally instrumented — **a measured leak on one
field is an argument about the shape of the function, not an inventory of its fields.**

✅ **Re-measured after the generalisation, including the new field.** Round-robin, 6000 rounds ×
20 iterations, medians, release build, with a **positive control built by re-implementing the
early return out-of-tree** so nothing shipped had to be perturbed:

| arm | ratio |
|---|---|
| wrong `kid` · wrong `subject_kind` · wrong `subject_value` · bad signature | **1.000× – 1.003×** |
| control repeat | 1.001× |
| **injected early return on `kid`** | **8.57×** |

The instrument resolves ~0.4%, so a leak of the injected magnitude is excluded by about three
orders of magnitude. **The subject-kind arm is 1.003× — the new field does not leak.** The control
separation is 8.57× rather than the original 32.8× because both arms here still pay the ~5.7 µs
parse; the earlier baseline evidently did not, which is a reminder that a ratio is only
comparable against its own control.

⚠️ **Two caveats on the instrument, because the requirement is stronger than what any test
proves.** First, the counter that makes an early return visible is `#[cfg(test)]`: in a test build
it gives signature verification an observable side effect, so the optimiser **cannot** sink the
call under a branch — in the shipped build the result merely feeds a `&` chain, and nothing
structurally forbids sinking it. **The instrument guarantees the property only in the build where
the instrument exists**, and the release-build measurement above is the sole evidence for the
shape consumers get, on one compiler version. Second, the timing test's arms are enumerated by
hand, so **a comparison added later is unguarded until someone adds its arm** — which is exactly
what happened to `subject_kind` and was caught by review rather than by the suite.

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
delegation_preimage(&root_public_key, &claim)            -> the Sig_structure bytes to be signed
assemble_delegation(&root_public_key, &claim, &signature) -> the tagged COSE_Sign1 bytes
sign_delegation(&Ed25519Identity, &claim)                 -> tagged COSE_Sign1 bytes   [wrapper]
verify_delegation(bytes, &expected_root_key, expected_subject_kind, expected_subject_value)
                                                          -> AnchorDelegation
```

⛔ **`verify_delegation` takes the expected subject as TWO arguments — the kind and the value —
and an earlier draft of this block listed only `expected_origin`.** That stale signature is not
a cosmetic slip: §3.3 argues that requiring the *kind* is what makes the domain and seat
namespaces disjoint by construction, so a signature omitting it **removes the defence the
adjacent section argues for**, and does so in the block an implementer is most likely to read
instead of the prose. The kind is a typed enum rather than an integer, so a caller cannot pass a
value outside the closed set.

**The root key is an argument, not a claim field.** `delegation_preimage` takes the root key
*separately* from the six payload fields the claim carries, rather than bundling all seven into
one struct. The
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

- ⛔ **Genesis is `(subject_kind = domain, role = operational)` — the pair `(1, 2)` — and neither
  is a parameter.** This section previously never said so, which was the most consequential of
  its silences: leaf 0 is *the one leaf that can never be corrected* (`LeafStore` has no insert
  and no rewrite), so a constructor that took the kind from a caller would let a single
  mis-passed argument produce an anchor whose genesis permanently claims to delegate a **seat**.
  There is no recovery path and no verifier that would flag it, because the artifact would be
  perfectly signed and internally valid — the pair `(2, 3)` is in the §1.2 table. **Fixing the
  kind in the constructor is what makes "an anchor's genesis is a domain delegation" a property
  of the code rather than of the caller.**
- **`sequence` is `0` for genesis**, fixed by the constructor, not a parameter. It was stated
  only as an aside in §6.1's *test-vector* rationale, which is not where a constructor's
  behaviour belongs. Not a parameter because genesis is the first delegation for its
  `(subject_kind, subject_value, role)` by construction, so any nonzero start opens a range below
  the first entry that nothing can ever write into — and a caller-chosen start is the same
  foot-gun §1.2 closes at `u64::MAX`.
- **`not_before_unix_ms` is a parameter and no clock is read.** It is a claim by the signer,
  and the signer is the caller holding the root key, not the library. The argument that
  settles it is the field's own purpose: `not_before` exists so a delegation can be prepared
  **offline to take effect later**, which a clock read at creation time forbids outright.
- **`subject_value` comes from `LeafStore::origin()`.** No new config field, no constant (DP15).
  The store's origin is a domain, which is why the fixed kind above is the *consistent* choice
  rather than merely the safe one — a `seat` genesis would put a domain string in a field typed
  as a seat identifier, the exact "signed value that reads as something it is not" failure §2.3
  refuses.
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
| `subject_kind` | `1` (domain) |
| `subject_value` | `example.test` |
| `role` | `2` (operational) |
| `not_before_unix_ms` | `1700000000000` |
| `sequence` | `300` |

⚠️ **The vector must be regenerated by both parties independently whenever ANY row above, or the
content type, changes — and it has now been invalidated three times.** Round 2 by adding
`sequence`; round 3 by the content-type rename; round 4 by the role renumbering (`1` → `2`),
which changes the payload, the preimage, the signature and the artifact while leaving the
protected header alone. **Stating the cause per round was itself a defect** — this warning twice
named a superseded reason and so read as already-handled, which is precisely how a stale vector
survives. The rule replaces the list: *any change to a signature-covered byte invalidates it.*

A vector regenerated by one side and copied by the other is one party agreeing with itself. The
encoder party has never read the Rust, and that is the whole value of the exercise.

**Every field value is distinct, and that is a requirement rather than an aesthetic.** Two
payload fields sharing a value mask a swap between them — the §0.5-consequence-4 failure in
miniature, and the reason the role vocabulary starts at `2` is the same reason stated at the
level of the format instead of the vector. So `subject_kind = 1` and `role = 2` differ, and
`sequence` is neither.

`sequence = 300` is chosen to expose bugs rather than to look natural. It is **not** `0`, because
a zero field is indistinguishable from a field an implementation forgot to write and defaulted;
it is **not** `1` or `2`, because those are `subject_kind`'s and `role`'s values; and `300` needs
**two argument bytes** — `19012c`, and ⛔ **an earlier draft of this line wrote `190130`, which is
304.** I hand-wrote that literal and never recomputed it; the independent encoder caught it. It
is the golden-literal-nobody-rechecks failure this whole exercise exists to catch, and it landed
in the one field chosen *because* it is head-width-sensitive. That gives a third argument width
alongside `not_before`'s eight and the two inline enums' zero.

📐 **Terminology, fixed here because these sections contradicted each other.** A CBOR head is an
**initial byte** plus zero or more **argument bytes**. Widths below are always stated as the
number of *argument* bytes: `19012c` is two, `781f` is one, `1b…` is eight, and an inline value
such as `02` is zero. §6.1 previously called `19012c` a "2-byte head" (counting arguments) while
§6.1.1 called `781f` a "2-byte head" (counting total bytes) — **the same phrase meaning two
different things, in the two sections whose entire job is pinning head widths.** The concrete
cost, and it is not hypothetical: a reviewer checking "the 2-byte case is covered" sees both and
reads them as one class when they are different widths, which is how the gaps in §6.1.1 below
went unnoticed by their own author. A genesis delegation would in practice carry `sequence = 0`; the vector is not obliged to
be a genesis one, and is more useful for not being.

`example.test` is a reserved name and **not** the configured production origin. DP15 forbids a
committed origin constant; a test vector carrying the real origin would be one.

### 6.1.1 Vector B — the seat arm, and the three things vector A structurally cannot test

**Vector A above is retained unchanged** — it is the domain arm, the one genesis actually uses.
Vector B is **added alongside**, because a second vector is free (vectors are not frozen; only
the format is) and A leaves three specific gaps that no amount of care about A can close:

1. **The seat arm has no coverage anywhere.** `(kind, role) = (2, 3)` is half the format and no
   golden vector exercises it, so a bug reachable only through a seat delegation would ship.
2. **A cannot distinguish "wrote the value" from "wrote the label twice" at label 1**, because
   `subject_kind = 1` sits at label `1` and encodes `0101`. §6.1 requires distinct *values*; it
   never considered **label/value** collision. B has none: no label equals its own value.
3. ⭐ **A cannot test shortest-form encoding at all, and §1.2 says so in as many words** —
   `not_before = 1700000000000` "needs the 8-byte head anyway", so an implementation emitting a
   fixed 8-byte head passes A. **B fixes the gap that vector A's own rationale identified and
   left open**, which is the more useful half of adding it.

| field | value | why |
|---|---|---|
| root seed | **unchanged** (`00 01 … 1f`) | identical protected header, so B isolates the payload |
| delegated public key | **unchanged** (derived from `20 21 … 3f`) | same reason |
| `subject_kind` | `2` (seat) | the untested arm |
| `subject_value` | `lys-seat-00000000000000000000001` (32 bytes) | `tstr` head `7820` — **one** argument byte; A's 12-byte value is inline. 32 rather than 31 or 33 — see below |
| `role` | `3` (speaks_for) | the untested role; pair `(2,3)` |
| `not_before_unix_ms` | `7` | **inline** — zero argument bytes, the case A cannot reach. A fixed-eight-byte implementation fails here and only here |
| `sequence` | `24` | encodes `1818`: the exact boundary where CBOR stops inlining. Off-by-one head logic fails at `24` and passes at both `23` and `300` |

**No value collides with another value, and no value collides with its own label.** `not_before`
is `7` rather than `1` for the second reason: `1` is inline too, but it equals **label 1**, and a
positional decoder reading a shifted label-value stream is exactly the bug class this vector
exists to expose. `7` is inline, distinct from `2`, `3` and `24`, and collides with **no label at
all**. Two thirds of the residual cannot be removed — kinds are frozen at `{1,2}` and roles at
`{2,3}` while labels run `1..6`, so those must collide with some label — but this third was free.

⛔ **`subject_value` is 33 bytes, and the three values it passed through are the argument for the
rule rather than an anecdote about indecision.** Each length collides at a *different level of
nesting*, and each collision was found by a different party:

- **31 bytes** → payload is exactly **79**, the same length as the protected bucket, so inside
  the `Sig_structure` both buckets carry the identical `bstr` head `584f`. An implementation
  deriving the payload's length head from the protected bucket's is invisible in B. **Collides at
  the envelope level.**
- **32 bytes** → payload is 80 and the envelope tie is gone, but 32 is **exactly the length of the
  delegated public key** at label 3, so two entries inside the payload map have equal-length
  bodies whose heads differ only by major type (`7820` against `5820`). **Collides one level
  inward.** Weaker — a swap of labels 2 and 3 puts a `bstr` where a `tstr` belongs, so a
  positional decoder with pinned types catches it on *type* — but it leans on that disjointness
  rather than on the length being distinct.
- **33 bytes** → payload 81: distinct from the protected bucket *and* from the key. Collides at
  neither level, and was the recommendation on that basis.

⭐ **32 was kept, and the reason is that the second "collision" is not one.** A swap of labels 2
and 3 puts a `bstr` where a `tstr` is expected and vice versa, so a **positional decoder with
pinned types catches it on type**, not on length — the guard is structural rather than
coincidental. And the equal lengths buy something no other vector has: labels 2 and 3 sit adjacent
carrying `7820` and `5820`, **the same argument width under two different major types**, so an
implementation that computes the right length under the wrong major type is caught within one
field of itself. That is the one axis C's `tstr`-versus-`bstr` widths cannot test at close range.

So the equal-length case is *guarded* at the payload level and *productive* besides, where the
31-byte case was unguarded at the envelope level — nothing but the length distinguished the two
buckets there. **Same surface fact, opposite verdicts, and the difference is whether anything
else is already separating the two values.**

⭐ **The lesson §6.1's distinctness rule keeps re-teaching: it is scoped to values *within* the
payload map, while the envelope above and the field lengths beside it can collide too. A rule
scoped to one nesting level says nothing about the next** — and fixing the level you noticed moved
the collision to the level you did not, twice in a row here. The rule to carry forward is not
"make every length distinct" but **"for each collision, name what else separates the two values —
and if the answer is nothing, change one."**

#### ⚠️ What A and B together do NOT cover — three argument widths, stated because I claimed otherwise

This subsection previously ended *"between the two vectors every head width the format can
produce appears at least once."* **That was false**, and the independent encoder refuted it by
walking both artifacts and both preimages, collecting every `(major type, argument width)` pair
actually emitted, and differencing against what §1.1/§1.2 plus the 4096-byte cap can produce.
Three producible widths appear in neither vector:

| missing | reachable by | witness |
|---|---|---|
| `uint`, **four** argument bytes (`1a`) | `not_before` or `sequence` in `[65536, 2³²−1]` | `sequence = 70000` → `1a00011170` |
| `tstr`, **two** argument bytes (`79`) | `subject_value` of 256 bytes up to the derived maximum (§1.2 — **not** a constant `3885`) | 300 bytes → `79012c` |
| `bstr`, **two** argument bytes (`59`) | the payload `bstr` once `subject_value` passes 255 | → `59016f` |

⛔ **The `uint` four-byte gap is the one that matters**, because A and B between them cover zero,
one, two and eight argument bytes and **jump straight over four** — a genuinely broken branch in
an implementation's head logic would pass both vectors. And note the second and third: the
two-argument-byte width is exercised in **major type 0 only** (A's `sequence`), never in a
*length* head, so an implementation correct for integers and wrong for lengths is invisible to
both.

**Vector C closes all three at once** and is therefore specified rather than deferred: a
`subject_value` of **300 bytes** drags the `tstr` head to `79012c` *and* the payload `bstr` head
past 255, while `sequence = 70000` supplies the four-byte `uint`. Inputs otherwise as A.

#### ⛔ A+B+C complete the width axis — and the axis is complete because it is COARSE

Both parties confirm that all twelve producible `(major type, argument width)` pairs are covered
by A, B and C. **That is the setup, not the conclusion.** Head width is a property of a **range**;
shortest-form encoding is a property of a **boundary**. The three vectors carry eight distinct
`uint` values and three distinct lengths, and between them they cross exactly **one** of the
format's ten boundaries — and only from above.

⭐ **The sharpest instance: B pins `sequence = 24`, the first value needing an argument byte, and
the realistic off-by-one is in the other direction.** An encoder switching at `>= 23` emits
`1817` for 23, and *none* of `1, 2, 3, 7, 24, 300, 70000` catches it. The gap sits **inside the
boundary B was written to pin** — pinning one side of a boundary reads as pinning the boundary.

The boundaries no vector crosses, with witnesses, listed rather than summarised so they cannot
read as handled:

| uncovered | witness | note |
|---|---|---|
| `uint` 23 (low side) | `17` vs `1817` | see above; closed by **vector D** |
| `tstr` 23 / 24 | `77…` vs `7818…` | the same boundary in the **length** path — C proved the 2-byte *width* transfers across major types, never that the inline boundary does. Closed by **D** |
| ⛔ ~~`uint` 2⁶³~~ — **MISFILED, removed from this table** | `1b8000000000000000` | **Nothing changes head width at 2⁶³.** `1b7fff…` and `1b8000…` are both eight argument bytes, so this is not a shortest-form boundary at all — it is a **type-modelling** boundary (`u64` vs `i64`), which belongs to §1.2 and not to a table of encoding widths. Listing it here made this axis look better covered than it is: a reader counting closed rows counts one that was never on the axis. Vector E still exists and is still worth having — see below for what it does and does not measure |
| `uint` 255/256, 65535/65536, 2³²−1/2³² | `18ff`/`190100`, `19ffff`/`1a00010000`, `1affffffff`/`1b…` | argument-width transitions. **Left uncovered**, recorded here rather than closed |
| `tstr` 255/256 | `78ff`/`790100` | argument-width transition. **Left uncovered** |
| ⛔ the cap boundary, wherever it falls | `790f2d`/`790f2e` for A's field values | **This row used to read "`3885`/`3886` … the newly-pinned cap", inheriting §1.2's false constant.** The boundary is a *function* of `not_before` and `sequence`, so there is no single pair of lengths to cover — a vector pinning `3885/3886` would pin one point on a curve and read as pinning the rule. **Left uncovered, and now for a second reason: what to cover is itself undecided** |

**Vector D — the cheap low-side boundaries.** Inputs as A except

```text
subject_value       = "twenty-three-bytes.test"   / exactly 23 bytes, tstr head 0x77 /
not_before_unix_ms  = 23                          / uint head 0x17, inline /
sequence            = 0
```

⛔ **The value is written out because the previous version of this line said only "a **23-byte**
value" and the dispatch that acted on it supplied `lys-seat-00000000000` — twenty bytes.** Both
independent parties measured it, both refused to adjust silently, and both computed the 23-byte
and 20-byte readings side by side rather than picking one. The 20-byte value has `tstr` head
`0x74`: inside the inline range, nowhere near its edge, and therefore **already covered by
vector A's 12-byte value**. The integer half still works, so a naive check goes green and
§6.1.1 records the `tstr` boundary as closed by an artifact that never crosses it.
⭐ **A specified length is not a specified value.** Where the point of a vector *is* a length,
the length must be checkable by counting the literal.

⛔⛔ **And the literal I chose to correct it with was ALSO wrong, in a worse way — caught by
the same two parties, independently, from two different sections.** `lys-seat-…` under
`subject_kind = 1 (domain)` is **a seat-shaped string in a domain slot**: precisely the
confusable pairing §0.5.1 and §3.3 spend paragraphs defending against, about to be frozen into
the vector most likely to be copy-pasted as a starting point. It would have taught every
implementer the shape the format exists to prevent. The replacement is a plain domain whose
text states the property the vector is *for*, so a reader who miscounts is corrected by the
string itself.

**Vector E — the `u64` boundary the specification argues.** Inputs as A except
`not_before_unix_ms = 9223372036854775808` (2⁶³), head `1b8000000000000000`.

⚠️ **E does not measure what it was written to measure, and that has to be said here rather
than discovered.** Its stated purpose is §1.2's *interoperability* claim — that a `u64`-vs-`i64`
disagreement is possible at ≥ 2⁶³. But an `i64` implementation **cannot represent 2⁶³ at all**,
so it can never be asked to encode E; every available encoder emits the same eight bytes by
construction, and the one foreign decoder in the exercise (`fxamacker/cbor`) models label 5 as
`uint64` and passes trivially. **The vector pins our bytes and leaves the claim untested.**

The discriminating test, specified so it is not re-derived: decode E into a struct with label 5
as `uint64` and assert equality, **and** decode it into one with `int64` and assert that it
*fails*. Only the second half measures the claim, and no golden vector can contain it — it is a
property of the decoder, not of the bytes.

⭐ **A vector can pin an artifact perfectly and still not test the sentence it was written
for.** "Untested claim" and "unpinned bytes" are different defects, and closing the second is
what makes the first stop looking urgent.

#### 🔨 RULING — D is SPLIT into D and F, because one artifact cannot isolate two boundaries

The two parties disagreed, and the disagreement is the useful part.

D as first specified put `not_before_unix_ms = 23` next to a `subject_value` of **23 bytes** —
the same number in two fields, in the one artifact written to pin the value `23`.

- **Party 1 recorded it as an explicit non-finding**: the major type separates the two, so a
  positional decoder with pinned types refuses a label-2/label-5 swap on type. That is the
  *guarded* case.
- **Party 2 recorded it as a finding**: an implementation that wrote `subject_value`'s length
  into label 5, or derived either from the other, produces D's exact bytes. Only the label byte
  and the position separate them. That is the *unguarded* case, and it is a different bug from
  the swap party 1 answered.

Both are right about the case each considered. **The ruling follows this document's own rule —
"for each collision, name what else separates the two values; if the answer is nothing, change
one"** — and the honest answer here is that the collision is *forced*: `23` is the only value at
the top of the inline range, so both boundaries want the same number, and neither can move.

**So the artifact moves instead. One boundary each:**

| | `subject_value` | `not_before_unix_ms` | `sequence` |
|---|---|---|---|
| **D** — the `tstr` inline boundary | `"twenty-three-bytes.test"` (**23**) | `1700000000000` (as A) | `0` |
| **F** — the `uint` inline boundary | `"example.test"` (12, as A) | **23** | `1` |

No two fields in either vector now share a value, so a mismatch names its own cause. D keeps
`sequence = 0` and therefore the genesis shape; F carries `sequence = 1` so that its three
numbers — 12, 23, 1 — are pairwise distinct.

⚠️ **And a limit on what either proves, which party 2 could see and party 1 could not.** In
`lys-core` both crossings go through a **single `cbor::write_head`**. For this implementation
they are one rule exercised twice, so D's `tstr` coverage is *inference from a shared code
path*, not an independent measurement — and splitting the vectors does not change that, because
the split separates the artifacts and not the function. It is an independent measurement only
against an implementation that encodes lengths and integers separately. **Claim the axis you
have:** these vectors pin bytes for everyone and isolate rules only for implementations built
differently from ours.

That asymmetry is worth naming: party 2's was an *implementation fact* it obtained by reading
the crate, party 1's a *methodology argument* from the spec alone — the party forbidden to read
`crates/` could not have found it, and the finding is not a mark against it.

⭐ **The lesson is about the claim, not the gap.** §6.1.1 was written to close a hole vector A
admitted, and it asserted completeness in the same breath — **claiming coverage immediately after
closing one hole is what lets the remaining holes read as handled.** State which widths are
covered and let the gaps be visible, or a reader checks the sentence instead of the set.

#### Vector D and genesis — ⛔ the claim that it "pins genesis's exact payload" was OVERSTATED

D carries `sequence = 0`, and §6.1's refusal of a zero sequence for its own vector — *a zero
field is indistinguishable from one an implementation forgot to write* — **was correct with one
vector and is now spent**: A, B and C all carry nonzero sequences, so a defaulting bug already
fails three times and a `sequence = 0` vector no longer risks hiding one. That much stands, and
it is why D is the right place to put the zero.

⛔ **What does NOT stand is "pins genesis's exact payload", and both parties refuted it by
different routes.** §5 fixes **three of the six payload fields** — `subject_kind = 1`,
`role = 2`, `sequence = 0`. The other three are free:

- `subject_value` is the operator's own origin, supplied at creation;
- `not_before_unix_ms` is a caller parameter;
- `delegated_public_key` is whatever key the operator generated.

So no vector can pin leaf 0's bytes, **and DP15 forbids committing a real origin anyway.** D
pins the *shape* genesis takes — the fixed three, plus a zero sequence encoded inline — and that
is worth having. It is not the same claim, and the stronger one is what makes leaf 0 read as
covered when it is not. ⭐ **"Pins the fields that are fixed" and "pins the payload" differ by
exactly the fields an operator chooses, which are the ones a vector was never able to reach.**

⚠️ **A gap that can never be closed, and should stop being listed as open.** §6.1.1's
label-versus-value ambiguity at `a6 01 01` (does an implementation write the value, or the label
twice?) is closed by B — but **every genesis payload that will ever exist opens `a6 01 01`**,
since §5 freezes `subject_kind = 1` and §1.2 freezes it at label 1. For the one position
`LeafStore` can never correct, that gap is permanent and no vector can close it. Recording it as
"closed by B" would let the case that matters most read as handled.

⚠️ **B must be generated by the same two independent parties as A, and neither may copy the
other.** It is a new vector, not a variant — the fact that A matched proves nothing about B.

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

#### ⚠️ Amended again — a verify-level injection MUST be re-signed, and byte-tampering cannot model either §3.2 or §3.3

Found by the independent encoder, executably, and it invalidated two rows of the table below —
including one written the same day, in the pass that was fixing this section.

**A tampered byte is not the attack these rules describe.** Flip a byte in `kid` and the artifact
now fails **both** §3.2 (`kid ≠ expected`) and §3.6 (the signature no longer verifies), so the
injection isolates nothing. Worse than non-isolation: **§3.2's entire premise is that the
attacker's artifact is *cryptographically perfect*** — "an attacker signs a delegation with their
own root key, puts their own key in `kid`, and it is cryptographically perfect." A tampered
artifact does not exercise that trap at all; it exercises the signature check, which was never in
doubt. So the injection must **re-sign with the attacker's key**, after which the signature
verifies and only §3.2 can fire.

The same holds for §3.3, and there the re-signing is not a technicality but *the literal scenario
the section describes* — "a delegation issued for anchor A is a valid delegation for anchor B
whenever B's operator holds the same root key." The injection must therefore be re-signed **by
the legitimate root key**.

⛔ **And the cross-kind row was wrong in a second, sharper way.** As written it changed
`subject_kind` to `2` and left `role` at `2`, producing the pair **`(2, 2)`** — which §2.3 clause
3 refuses **at decode**, before §3.3's comparison is ever reached. It therefore proved clause 3
a second time and left §3.3's cross-kind arm unproven. The pair space is small enough to settle
exhaustively: `(2,2)` and `(1,3)` are the *only* individually-valid-but-invalid pairs, so the
`(1,3)` row already covers clause 3 completely. Isolating §3.3 requires a **well-formed** seat
delegation, `(2, 3)`, which decodes cleanly and can only fail at the kind comparison.

**The general form, and it is §6.2's own lesson turned on §6.2:** the amendment above moved
decode-level tests *below* the byte-compare so each rule stands alone — but §3.2 and §3.3 are
**verify**-level comparisons against caller-supplied arguments, so they cannot be moved there.
For those, isolation comes from making the artifact *valid* rather than from lowering the test.
**An injection has to be well-formed in every respect except the one rule it targets**, and
"tamper with a byte" quietly violates that everywhere the signature covers — which is everywhere.

| injection | must be caught by |
|---|---|
| content type changed to the receipt's | §3.1 |
| `kid` replaced with the attacker's root key, **and the artifact re-signed with the attacker's root key** | §3.2 |
| payload `subject_value` changed, **and re-signed by the legitimate root key** | §3.3 — the value arm |
| a well-formed **seat** delegation, `(kind, role) = (2, 3)`, presented to a **domain**-expecting verifier | §3.3 — the cross-*kind* arm, and it is a separate rule from the value arm: a verifier that checked only the value would accept this |
| ~~payload map keys permuted~~ → **indefinite-length outer array** | §3.4 — retargeted; the permuted case is guarded by positional decode, not by the byte-compare |
| a byte flipped in the signature | §3.6 |
| unprotected header carries one entry | §1.3 |
| `role` set to `7` | §2.3 clause 2 — an unrecognised role |
| `role` set to `1` | §2.3 clause 2 — `1` is `subject_kind`'s domain value and deliberately **not** a role; this is the row that proves the vocabularies are disjoint rather than merely documented as such |
| **`(subject_kind, role)` = `(1, 3)` — both halves individually valid, the pair is not** | §2.3 clause 3. This case is required because the two single-field rows above cannot reach it: each half decodes, so **only** the pair check can fire |
| **labels `1` and `4` swapped, giving `(2, 1)`** | §2.3 clause 2, *not* clause 3 — `1` fails role decode before any pair is formed. Recorded as its own row because the whole point of §0.5 consequence 4 is that this injection is *reachable at all*: under roles-from-`1` the swapped artifact would have been byte-identical to a valid one and no injection could have existed to write |
| **`sequence` set to `u64::MAX`** | §1.2 — refused on **both** encode and decode |
| **`subject_value` set to the empty string** | §1.2 — the fail-closed rule for a verifier whose subject is unset |
| **`subject_value` one byte past the derived maximum** (artifact 4097) | §1.2 — one byte over the cap, on both encode and decode. ⚠️ The injection must **derive** that length from the `not_before` and `sequence` it uses, not hard-code `3886`: with both fields inline the first refused length is `3896`, so a `3886`-byte injection is a *valid* artifact and the row would silently stop testing anything |
| **`subject_value` containing invalid UTF-8** | §1.2 — the byte-oriented decoder that skips validation; §3.4's re-encode passes it |
| **delegated key (label 3) with a non-canonical `y`** (`[0xff; 32]`) | §1.2 — key-slot validation |
| **`kid` with a non-canonical `y`** | §1.2 — the *other* key slot, which went unvalidated while label 3 did not |
| **the early return on a `subject_kind` mismatch is restored** | §3.5.1 — the arm the timing test did not have |
| **a replayed earlier delegation supersedes the current one** | §2.2 |
| **the early return on a `kid` mismatch is restored** | §3.5.1 |

The last two are new and neither is an encoding rule, which is the point: **the two
severest findings of the review were a semantic defect and a channel outside the return
value.** An injection table built only from encoding rules would have found neither.

⛔ **Four of the rows above were MISSING until an enumeration went looking for them, and their
absence is a different failure from the one this section was amended twice to fix.** §1.2 states
four normative rejections — `sequence == u64::MAX`, an empty `subject_value`, the size cap, and
invalid UTF-8 — and none had an injection row. Both amendments above concern rules guarded
**twice**, where the count cannot say which guard did the work. **These were rules guarded
zero times: normative prose with nothing exercising it.** A rule with two guards is
over-determined; a rule with none is a sentence.

Found by enumerating §1.2's rules and differencing against this table — **not** by reading the
table, which cannot work: **an injection table is a list of what somebody thought of, so it can
never be audited against itself.** The audit has to run from the rules inward, and the rules live
in §1.2 and §2.3.

⭐ **The swap row is the only entry in this table whose value is that it can be written.** Every
other injection tests whether a guard fires; that one tests whether the *format* leaves room for
a guard to exist. A table of injections can only ever cover distinctions the encoding makes — so
a numbering choice that erases a distinction is invisible to this whole method, which is why it
had to be caught by a second party reasoning about the enum space rather than by any test.
