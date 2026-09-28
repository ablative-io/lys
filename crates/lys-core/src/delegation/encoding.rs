//! Byte-exact CBOR/COSE encoding for the `lys/delegation/v1` artifact.
//!
//! # Invariants
//!
//! - **Encoding is hand-assembled and infallible.** Every emitted byte comes
//!   from this module's fixed-shape writers over [`crate::cbor`]'s canonical
//!   heads — RFC 8949 §4.2 core deterministic by construction, immune to any
//!   serializer dependency's encoding choices across upgrades. There is
//!   deliberately no second canonical encoder in this workspace; two encoders
//!   drift silently while every round-trip test on either side keeps passing.
//! - **Decoding of untrusted input is never hand-rolled.** [`decode_fields`]
//!   parses with `ciborium` and then enforces the exact artifact shape; the
//!   caller ([`super::artifact::Delegation::from_cose_bytes`])
//!   additionally re-encodes the extracted fields and requires byte-identity
//!   with the input (canonical-encoding strictness).
//! - The protected header bucket is
//!   `{1: -8 (EdDSA), 3: <content type>, 4: <bstr .size 32 Ed25519 root key>}`
//!   in RFC 8949 §4.2 key order — exactly [`PROTECTED_LEN`] bytes. `kid` is a
//!   byte string of exactly 32 bytes (RFC 9052 §3.1) **and a point strict
//!   Ed25519 verification could accept**, so a key that is not an Ed25519 key
//!   cannot occupy the slot.
//!
//!   ⛔ **That sentence used to end at "a shorter or longer one is refused", and
//!   the conclusion did not follow from the premise.** Length is not keyness:
//!   `kid = [0xff; 32]` is exactly 32 bytes and is not a usable key — `y >= p`,
//!   so it is not a canonical encoding of any point — and it parsed. An
//!   adversarial review found the invariant false while the payload's *other*
//!   32-byte key slot had been validated for exactly this reason since the
//!   review before it. The two slots now obey one rule, applied by
//!   [`decode_protected`] and mirrored by [`check_encodable`].
//!
//!   Not a vulnerability in either state — [`super::sign::verify_delegation`]
//!   compares `kid` byte-for-byte against the key the caller named, so an
//!   unusable one can never match a key anybody trusts. It is fixed because a
//!   documented invariant that is false is worse than an absent one, and because
//!   tightening a decoder is free before publication and semver-bound after.
//! - **The tag is mandatory and an untagged `COSE_Sign1` is refused.** RFC 9052
//!   §4.2 permits either form "depending on the context", so the context has to
//!   say — and if both were accepted, one statement would have two valid
//!   encodings, which is the exact defect the re-encode gate exists to prevent.
//!   That gate only closes it because the canonical re-encoding emits the tag.
//! - **`not_before_unix_ms` is a `u64`.** CBOR `uint` reaches 2⁶⁴−1, so an
//!   implementation modelling the field as `i64` would find values ≥ 2⁶³
//!   wire-legal and undecodable, and two conforming implementations would
//!   disagree about a well-formed artifact. Unsigned also removes pre-1970
//!   timestamps from the format rather than from a validator.
//! - Ascending numeric label order and ascending bytewise-encoded-key order
//!   coincide for `{1, 3, 4}` because all three are non-negative and
//!   single-byte-headed; that is the same argument the receipt encoder records
//!   for `{1, 3, 4, 395}` and it is stated rather than assumed because it stops
//!   being true the moment a negative or multi-byte label is added. **A `v2`
//!   adding a negative label would break the coincidence** — COSE reserves
//!   negative labels for algorithm-specific parameters, and CBOR major type 1
//!   encodings sort *after* every major type 0 one, so such a label lands last
//!   under bytewise order and in numeric position under numeric order. Two
//!   implementations would then silently disagree.
//! - **There is no `395` (vds) entry.** That label declares a verifiable data
//!   structure, and a delegation proves nothing about a tree. Its absence also
//!   makes this bucket a different length from a receipt's, but length is a
//!   consequence and never the defence — the content type is.
//! - The payload map is `{1: subject_kind, 2: subject_value,
//!   3: delegated key, 4: role, 5: not_before_unix_ms, 6: sequence}` —
//!   **six** entries, labels ascending. The kind precedes the value it types.
//!   `sequence` is what orders delegations; see [`super`] for the replay defect
//!   it closes.
//! - **`subject_kind` and `role` are validated as a PAIR**, by
//!   [`DelegationSubjectKind::permits`], which is the single definition of the
//!   rule. `(1 domain, 2 operational)` and `(2 seat, 3 speaks-for)` are the only
//!   pairs this version defines; every other combination is refused at decode
//!   *and* at [`check_encodable`], including the two made entirely of
//!   individually valid values.
//! - **The two closed vocabularies are deliberately offset**, so that no valid
//!   pair has `subject_kind == role`. Both fields are `uint`s in one map, so
//!   numbering the roles from `1` would have made every valid `v1` payload carry
//!   the same byte at labels 1 and 4 — and an implementation that transposed the
//!   two fields would emit byte-identical artifacts, undetectable by any test or
//!   vector. Offset, a transposition yields a pair outside the table and is
//!   *refused* rather than merely different. See
//!   [`DelegationSubjectKind::permits`].
//! - **The payload is embedded**, not detached: there is no value a verifier
//!   independently recomputes, so the assertion must travel with the signature.
//!   See [`super`].
//! - **The unprotected bucket is the empty map `0xa0`**, and [`decode_fields`]
//!   refuses any other. Nothing unsigned may ride in this artifact.
//! - **An empty subject value is refused**, and the reason is about the
//!   *verifier* rather than about the string. Acceptance is a comparison against
//!   the caller's configured subject, so a verifier whose subject is unset would
//!   match an empty-subject delegation and accept it. Refusing the empty string
//!   at decode makes that misconfiguration fail closed.
//! - **The delegated key must be one `verify_strict` could accept** — canonical
//!   decompression, not small-order. See [`decode_payload`].
//! - Every decode failure collapses to
//!   [`TrustError::DelegationVerification`](crate::error::TrustError::DelegationVerification)
//!   (non-oracle; see the [`super`] module docs).
//!
//! # Canonical encoding is a requirement on the artifact, not a side effect
//!
//! **RFC 8949 §4.2 core deterministic encoding is normative on the wire for
//! this format.** A `lys/delegation/v1` artifact that is not canonically
//! encoded is not a valid artifact, whoever is reading it. That sentence is the
//! rule; everything below is about how this implementation enforces it and what
//! would go wrong without it.
//!
//! Stating it that way round matters more than it looks. RFC 9052 §9's own
//! restrictions are definite lengths, minimum-length arguments and no duplicate
//! labels — **map key *ordering* is not among them** — and they are scoped to
//! the `Sig_structure`, `Enc_structure` and `MAC_structure` rather than to the
//! headers. So canonicality here comes from *this format electing it*, and a
//! format's requirements have to be written down as requirements. A property
//! that holds only because of one implementation's internal strategy is a
//! property strangers do not have, and strangers verifying it is the entire
//! value of this crate.
//!
//! ## Why the byte-compare cannot be dropped — and the argument that does not work
//!
//! Two earlier arguments for the check were both wrong, in opposite directions,
//! and both are recorded because each is the kind that survives review.
//!
//! **Wrong argument #1:** *"the byte-compare is the only thing that refuses a
//! permuted map."* An adversarial review deleted the check: five tests failed
//! and neither permutation test was among them. [`decode_protected`] and
//! [`decode_payload`] use positional slice patterns with pinned integer labels,
//! so a permuted map dies at decode regardless.
//!
//! **Wrong argument #2:** *"non-canonicality inside the two `bstr`s is caught by
//! the signature check anyway, because [`super::sign::verify_delegation`]
//! re-derives the preimage from the parsed fields rather than from the wire's
//! bytes."* That is true **of this verifier and no other**. RFC 9052 §4.4 step 2
//! takes the protected attributes *from the body structure* — the wire bytes
//! verbatim — and a verifier built that way (`go-cose` is one) signs and checks
//! the *same* non-canonical bytes, so its signature check catches nothing. To
//! every conforming stranger a permuted protected map would be a second valid
//! artifact for one statement. Our preimage-derivation strategy is a detail of
//! ours; it cannot be what the guarantee rests on.
//!
//! **So the enforcement is deliberately layered, and each layer is load-bearing
//! for a different reader.** The positional decode pins refuse reordering; the
//! byte-compare refuses everything else, including the whole **envelope**, which
//! no signature covers under any verifier's strategy — tag head width,
//! indefinite-length forms in the outer array, the three `bstr`s and the
//! unprotected map, non-minimal length heads, and trailing garbage. At
//! [`super::artifact::Delegation::from_cose_bytes`] the byte-compare is
//! the only canonicality guard there is, since no signature is checked at all.
//!
//! ## What removing the byte-compare has been *measured* to admit
//!
//! `artifact_tests` exercises a malleability sweep — non-minimal tag head,
//! non-minimal outer-array head, indefinite outer array, indefinite protected
//! `bstr`, indefinite payload `bstr`, indefinite unprotected map (`bf ff`),
//! non-minimal length heads on each of the three `bstr`s, and trailing garbage.
//! **That is a lower bound on what the check guards, not an inventory of it.**
//! An earlier note here said "exactly four artifacts flip to accepted", which
//! read as an exhaustive count and was only ever the set the suite happened to
//! cover. CBOR admits more non-canonical spellings than any suite enumerates,
//! which is exactly why the rule is "byte-identical to the canonical
//! re-encoding" rather than a list of rejected forms.
//!
//! **A rule defended by the wrong argument survives exactly until someone checks
//! the argument.** The check stays; its justification has now been corrected
//! twice.
//!
//! [`decode_protected`]: decode::decode_protected
//! [`decode_payload`]: decode::decode_payload
//! [`DelegationSubjectKind::permits`]: crate::delegation::artifact::DelegationSubjectKind::permits

use crate::cbor::{
    MAJOR_ARRAY, MAJOR_MAP, MAJOR_TAG, MAJOR_UNSIGNED, write_bytes, write_head, write_i64,
    write_text,
};
use crate::delegation::artifact::DelegationClaim;
use crate::error::{TrustError, TrustResult};
use crate::keys::identity::is_usable_ed25519_public_key;

mod decode;

pub(crate) use decode::decode_fields;

/// The `lys/delegation/v1` domain discriminator: the protected content
/// type (COSE header label 3). Signature-covered.
///
/// This string is a frozen wire contract — evolving the artifact means a new
/// `v2` media type, never a mutation of this one. It is also the *only* thing
/// separating a delegation from a receipt, a consistency receipt or an
/// attestation, all of which are `COSE_Sign1` messages signed by the same kind
/// of Ed25519 key and whose signing preimages share their first twelve bytes.
pub(crate) const CONTENT_TYPE: &str = "application/vnd.lys.delegation.v1+cbor";

/// Length of a raw Ed25519 public key, and so of both `kid` and the delegated
/// key in the payload.
pub(crate) const KEY_LEN: usize = 32;

/// Exact length of the protected bucket: `map(3)` head, `1 => -8` (2 bytes),
/// `3 => text(38)` (3 + 38), `4 => bstr(32)` (3 + 32).
///
/// Fixed because every component is fixed — the content type is a constant and
/// the key is always 32 bytes. Pinned as a constant so a change to any of them
/// is a visible change here rather than a silent one on the wire.
pub(crate) const PROTECTED_LEN: usize = 79;

/// Hard input cap for [`decode_fields`]. A canonical delegation is the 79-byte
/// protected bucket, a payload of roughly 55 bytes plus the subject value, a
/// 64-byte signature and a handful of heads. This bound is far above that and
/// rejects oversize input before parsing; it is also the only bound on the
/// subject value, which this format otherwise treats as opaque text.
pub(crate) const MAX_ARTIFACT_LEN: usize = 4096;

/// CBOR tag number for `COSE_Sign1` (RFC 9052 §2). The artifact is always
/// tagged — byte 0 is `0xd2` — and the verifier requires the tag.
const COSE_SIGN1_TAG: u64 = 18;

/// COSE header label `alg`.
const HEADER_LABEL_ALG: u64 = 1;
/// COSE header label `content type`.
const HEADER_LABEL_CONTENT_TYPE: u64 = 3;
/// COSE header label `kid`.
const HEADER_LABEL_KID: u64 = 4;

/// The `alg` value: `EdDSA`.
///
/// `-8` rather than RFC 9864's preferred `-19`, deliberately and for the same
/// reason as the shipped attestation and receipt: `go-cose` ships only `-8`,
/// and an artifact no off-the-shelf library verifies is worthless. A move to
/// `-19` is a `v2` matter, triggered when the Go and Python COSE ecosystems
/// both accept it.
const ALG_EDDSA: i64 = -8;

/// Payload map label 1: `subject_kind` — what kind of thing label 2 is. A
/// closed enum, and the kind **precedes** the value it types.
const PAYLOAD_LABEL_SUBJECT_KIND: u64 = 1;
/// Payload map label 2: the subject the delegation is scoped to.
const PAYLOAD_LABEL_SUBJECT_VALUE: u64 = 2;
/// Payload map label 3: the raw 32-byte key being delegated **to**.
const PAYLOAD_LABEL_DELEGATED_KEY: u64 = 3;
/// Payload map label 4: the role, a closed enum whose vocabulary is scoped per
/// subject kind.
const PAYLOAD_LABEL_ROLE: u64 = 4;
/// Payload map label 5: `not_before_unix_ms` — a claim by the signer, never an
/// ordering key.
const PAYLOAD_LABEL_NOT_BEFORE: u64 = 5;
/// Payload map label 6: `sequence` — the value that *does* order delegations.
const PAYLOAD_LABEL_SEQUENCE: u64 = 6;

/// The largest `sequence` this format accepts: `u64::MAX - 1`.
///
/// **`u64::MAX` is refused so that a successor always exists.** `sequence` is
/// strictly increasing per `(subject_kind, subject_value, role)`, and the maximum
/// has no successor —
/// a signer who issued there would permanently disable rotation for that
/// subject, with no in-band way out.
///
/// It is a foot-gun rather than a vulnerability: nothing attacker-reachable
/// leads here, since issuing any delegation needs the offline root key. It is
/// forbidden anyway because the check costs one comparison today and is
/// impossible to add after the format freezes. That turns "a successor always
/// exists" into a property of the format rather than a property of operator
/// care.
pub(crate) const MAX_SEQUENCE: u64 = u64::MAX - 1;

/// Build the protected header map `{1: -8, 3: content_type, 4: root_key}` in
/// canonical key order.
///
/// **`content_type` is the caller's declaration of which artifact kind it is
/// building, and is never read from an artifact.** At verification the header is
/// re-derived through this function from the constant belonging to the code path
/// doing the verifying, so the discriminator that separates a delegation from a
/// receipt is not attacker-supplied at all. Reading the wire's type and
/// comparing it against itself would accept anything; reading it and
/// *dispatching* on it would hand the choice to whoever wrote the artifact.
///
/// The parameter exists rather than the constant being inlined so that the tests
/// can assemble a cryptographically perfect artifact bearing a *foreign* content
/// type and prove it is refused — a mutant built by splicing bytes would fail
/// its signature check first and prove nothing about the pin.
pub(crate) fn protected_bytes(content_type: &str, root_public_key: &[u8; KEY_LEN]) -> Vec<u8> {
    let mut out = Vec::with_capacity(PROTECTED_LEN);
    write_head(&mut out, MAJOR_MAP, 3);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_ALG);
    write_i64(&mut out, ALG_EDDSA);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_CONTENT_TYPE);
    write_text(&mut out, content_type);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_KID);
    write_bytes(&mut out, root_public_key);
    out
}

/// Build the embedded payload map
/// `{1: subject_kind, 2: subject_value, 3: delegated_key, 4: role,
/// 5: not_before_unix_ms, 6: sequence}` in canonical key order.
///
/// The subject kind and the role are written from
/// [`DelegationSubjectKind::wire_value`] and [`DelegationRole::wire_value`]
/// rather than from numbers the caller supplied, so no value outside either
/// closed enum can be emitted by this crate at all.
///
/// **Encoding stays infallible and total over the field types.** The field
/// constraints this format adds — a non-empty subject value, a usable delegated
/// key, and a `(subject_kind, role)` pair this version defines — are enforced by
/// [`check_encodable`] on the issuing path rather than here, so that this
/// function keeps the property the whole byte-exactness argument rests on: it
/// cannot fail, so there is no encoding failure mode to reason about and
/// canonicality is a property of the construction.
///
/// [`DelegationSubjectKind::wire_value`]: crate::delegation::artifact::DelegationSubjectKind::wire_value
/// [`DelegationRole::wire_value`]: crate::delegation::artifact::DelegationRole::wire_value
pub(crate) fn payload_bytes(claim: &DelegationClaim) -> Vec<u8> {
    let mut out = Vec::with_capacity(claim.subject_value.len() + 80);
    write_head(&mut out, MAJOR_MAP, 6);
    write_head(&mut out, MAJOR_UNSIGNED, PAYLOAD_LABEL_SUBJECT_KIND);
    write_head(&mut out, MAJOR_UNSIGNED, claim.subject_kind.wire_value());
    write_head(&mut out, MAJOR_UNSIGNED, PAYLOAD_LABEL_SUBJECT_VALUE);
    write_text(&mut out, &claim.subject_value);
    write_head(&mut out, MAJOR_UNSIGNED, PAYLOAD_LABEL_DELEGATED_KEY);
    write_bytes(&mut out, &claim.delegated_public_key);
    write_head(&mut out, MAJOR_UNSIGNED, PAYLOAD_LABEL_ROLE);
    write_head(&mut out, MAJOR_UNSIGNED, claim.role.wire_value());
    write_head(&mut out, MAJOR_UNSIGNED, PAYLOAD_LABEL_NOT_BEFORE);
    write_head(&mut out, MAJOR_UNSIGNED, claim.not_before_unix_ms);
    write_head(&mut out, MAJOR_UNSIGNED, PAYLOAD_LABEL_SEQUENCE);
    write_head(&mut out, MAJOR_UNSIGNED, claim.sequence);
    out
}

/// Refuse, on the **issuing** side, every claim the decoder would refuse.
///
/// # Why this exists rather than being left to verification
///
/// Encode and decode were allowed to disagree once, and the shape of the
/// resulting defect is worth keeping in front of anyone who edits either side.
/// The artifact cap was enforced at decode only, so a subject value of 3884
/// bytes signed and verified while 3885 signed *successfully* and then failed
/// every verification afterwards. That is exactly the "file that fails verification at
/// some later, less debuggable moment" that
/// [`super::sign::assemble_delegation`]'s signature check exists to prevent,
/// arriving through the one door that check does not cover.
///
/// So the rule is: **every constraint the decoder enforces is refused here
/// too**, and the two lists are kept together in one function so a new decode
/// rule that is not mirrored is a visible omission rather than an invisible one.
/// The `kid` point check was added here in the same change that added it to
/// [`decode_protected`], for exactly that reason: a decode rule landing without
/// its encode mirror would have made *this* invariant false while repairing
/// another.
///
/// **A rule enforced in two places is proven by neither of the obvious cases**,
/// which is why the `(subject_kind, role)` pair has exactly one *definition* —
/// [`DelegationSubjectKind::permits`] — called from here and from
/// [`decode_payload`]. Deleting either call site leaves the other, so the
/// isolating tests are written at each site directly rather than through an
/// entry point that passes both.
///
/// The size bound is *derived* from the encoded artifact rather than from a
/// precomputed subject-value length, because the derived limit moves whenever a payload
/// field is added — `sequence` moved it — and a hardcoded bound would have gone
/// stale silently.
///
/// # Errors
///
/// Returns [`TrustError::DelegationEncoding`] naming the constraint. Descriptive
/// rather than non-oracle: the caller supplied every input and holds the key.
///
/// [`decode_protected`]: decode::decode_protected
/// [`decode_payload`]: decode::decode_payload
/// [`DelegationSubjectKind::permits`]: crate::delegation::artifact::DelegationSubjectKind::permits
pub(crate) fn check_encodable(
    root_public_key: &[u8; KEY_LEN],
    claim: &DelegationClaim,
) -> TrustResult<()> {
    if !is_usable_ed25519_public_key(root_public_key) {
        return Err(TrustError::DelegationEncoding {
            reason: "the root public key going into the protected `kid` is not one strict \
                     Ed25519 verification could ever accept (non-canonical encoding, or a \
                     small-order point): no signature could verify under it, so the artifact \
                     would be refused by its own decoder"
                .to_string(),
        });
    }
    if claim.subject_value.is_empty() {
        return Err(TrustError::DelegationEncoding {
            reason: "the subject value is empty: a verifier whose configured subject is unset \
                     would match it, so the format refuses it at both ends"
                .to_string(),
        });
    }
    if !claim.subject_kind.permits(claim.role) {
        return Err(TrustError::DelegationEncoding {
            reason: format!(
                "subject kind {:?} does not confer role {:?}: the two are validated as a pair, \
                 because a subject combined with an authority nothing defines over it is a \
                 signed statement with no meaning",
                claim.subject_kind, claim.role
            ),
        });
    }
    if !is_usable_ed25519_public_key(&claim.delegated_public_key) {
        return Err(TrustError::DelegationEncoding {
            reason: "the delegated public key is not one strict Ed25519 verification could ever \
                     accept (non-canonical encoding, or a small-order point): a delegation \
                     naming it could never authorise anything"
                .to_string(),
        });
    }
    if claim.sequence > MAX_SEQUENCE {
        return Err(TrustError::DelegationEncoding {
            reason: format!(
                "sequence {} is the maximum a u64 can hold, and sequence must strictly \
                 increase — issuing here would leave no successor and permanently disable \
                 rotation for this subject; the largest issuable value is {MAX_SEQUENCE}",
                claim.sequence
            ),
        });
    }
    // Derived, never hardcoded: the bound shifts whenever the payload gains a
    // field, and the last time one was added it shifted by nine bytes.
    let encoded_len = artifact_bytes(root_public_key, claim, &[0u8; 64]).len();
    if encoded_len > MAX_ARTIFACT_LEN {
        return Err(TrustError::DelegationEncoding {
            reason: format!(
                "the encoded artifact would be {encoded_len} bytes, over the \
                 {MAX_ARTIFACT_LEN}-byte cap the decoder enforces — shorten the subject value"
            ),
        });
    }
    Ok(())
}

/// Wrap `protected`, `payload` and `signature` in the tagged `COSE_Sign1`
/// envelope `18([protected, {}, payload, signature])`, all definite-length.
///
/// Split out from [`artifact_bytes`] so the same envelope writer serves the
/// content-type-confusion test, which needs a genuine envelope around a
/// deliberately foreign protected bucket.
pub(crate) fn envelope_bytes(protected: &[u8], payload: &[u8], signature: &[u8; 64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(protected.len() + payload.len() + 96);
    write_head(&mut out, MAJOR_TAG, COSE_SIGN1_TAG);
    write_head(&mut out, MAJOR_ARRAY, 4);
    write_bytes(&mut out, protected);
    // The unprotected bucket: the empty map, and nothing else is ever emitted
    // or accepted here.
    write_head(&mut out, MAJOR_MAP, 0);
    write_bytes(&mut out, payload);
    write_bytes(&mut out, signature);
    out
}

/// Build the complete tagged `COSE_Sign1` delegation.
pub(crate) fn artifact_bytes(
    root_public_key: &[u8; KEY_LEN],
    claim: &DelegationClaim,
    signature: &[u8; 64],
) -> Vec<u8> {
    envelope_bytes(
        &protected_bytes(CONTENT_TYPE, root_public_key),
        &payload_bytes(claim),
        signature,
    )
}

#[cfg(test)]
#[path = "encoding_tests.rs"]
mod tests;
