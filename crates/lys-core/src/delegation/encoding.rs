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
//!   caller ([`super::artifact::AnchorDelegation::from_cose_bytes`])
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
//! [`super::artifact::AnchorDelegation::from_cose_bytes`] the byte-compare is
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

use ciborium::value::Value;

use crate::cbor::{
    MAJOR_ARRAY, MAJOR_MAP, MAJOR_TAG, MAJOR_UNSIGNED, write_bytes, write_head, write_i64,
    write_text,
};
use crate::delegation::artifact::{DelegationClaim, DelegationRole, DelegationSubjectKind};
use crate::error::{TrustError, TrustResult};
use crate::keys::identity::is_usable_ed25519_public_key;

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

/// The fields extracted from a structurally valid delegation.
pub(crate) struct DecodedFields {
    /// Raw 32-byte Ed25519 root key from the protected `kid`. A **claim**, not
    /// an authority — see [`super::sign::verify_delegation`].
    pub(crate) root_public_key: [u8; KEY_LEN],
    /// The six signature-covered payload fields.
    pub(crate) claim: DelegationClaim,
    /// 64-byte Ed25519 signature (`COSE_Sign1` item 3).
    pub(crate) signature: [u8; 64],
}

/// Non-oracle failure for every rejected delegation (see module docs).
fn reject() -> TrustError {
    TrustError::DelegationVerification
}

/// Parse one CBOR value from `bytes` with ciborium. Trailing garbage is not
/// detected here; the caller's re-encode-and-compare gate covers it.
fn parse_value(bytes: &[u8]) -> TrustResult<Value> {
    ciborium::de::from_reader(bytes).map_err(|_err| reject())
}

/// Extract a fixed-size byte array from a CBOR bstr value.
fn fixed_bytes<const N: usize>(value: &Value) -> TrustResult<[u8; N]> {
    let Value::Bytes(bytes) = value else {
        return Err(reject());
    };
    bytes.as_slice().try_into().map_err(|_err| reject())
}

/// Require an integer value equal to `expected`, used for map-key and
/// header-value pins.
fn require_integer(value: &Value, expected: i128) -> TrustResult<()> {
    let Value::Integer(int) = value else {
        return Err(reject());
    };
    if i128::from(*int) == expected {
        Ok(())
    } else {
        Err(reject())
    }
}

/// Extract a non-negative integer that fits in a `u64`.
fn unsigned(value: &Value) -> TrustResult<u64> {
    let Value::Integer(int) = value else {
        return Err(reject());
    };
    u64::try_from(i128::from(*int)).map_err(|_err| reject())
}

/// Decode the protected bucket, returning the root key from `kid`.
///
/// Pins `alg = -8`, [`CONTENT_TYPE`], and a `bstr` `kid` of **exactly** 32
/// bytes **that is a point strict Ed25519 verification could accept**, in
/// exactly that order and with no additional entries. The content type is
/// checked against this module's own constant, so a receipt, a consistency
/// receipt or an attestation is refused here — before any signature is examined
/// and regardless of whether it would have verified.
///
/// # Why `kid` is point-validated and not merely length-checked
///
/// The payload's delegated key has been validated this way since an earlier
/// review; `kid` was not, and the module docs claimed the length check made it
/// unnecessary. It does not: `[0xff; 32]` is 32 bytes and is not a canonical
/// encoding of any point. Two 32-byte key slots in one artifact obeying two
/// different rules is a difference somebody will eventually read as meaningful,
/// so they now obey one.
///
/// This is consistency and a false invariant repaired, **not** a vulnerability
/// closed: [`super::sign::verify_delegation`] compares `kid` byte-for-byte
/// against the caller's expected key, so an unusable `kid` could never have
/// matched a key anyone trusts.
fn decode_protected(protected_raw: &[u8]) -> TrustResult<[u8; KEY_LEN]> {
    let Value::Map(protected) = parse_value(protected_raw)? else {
        return Err(reject());
    };
    let [(alg_key, alg), (ct_key, ct), (kid_key, kid)] = protected.as_slice() else {
        return Err(reject());
    };
    require_integer(alg_key, i128::from(HEADER_LABEL_ALG))?;
    require_integer(alg, i128::from(ALG_EDDSA))?;
    require_integer(ct_key, i128::from(HEADER_LABEL_CONTENT_TYPE))?;
    let Value::Text(content_type) = ct else {
        return Err(reject());
    };
    if content_type != CONTENT_TYPE {
        return Err(reject());
    }
    require_integer(kid_key, i128::from(HEADER_LABEL_KID))?;
    let root_public_key: [u8; KEY_LEN] = fixed_bytes(kid)?;
    if !is_usable_ed25519_public_key(&root_public_key) {
        return Err(reject());
    }
    Ok(root_public_key)
}

/// Decode the embedded payload map into a [`DelegationClaim`].
///
/// Five rules here refuse a *well-formed* value rather than a malformed one,
/// and each is a case of the same principle — **a signed unchecked value looks
/// checked**, so a field this version cannot act on must not be carried past
/// the signature that lends it authority.
///
/// 1. **Unknown roles.** The role goes through [`DelegationRole::from_wire`], so
///    `role: 7` is a decode failure. Accepting and passing it on would hand a
///    consumer a cryptographically perfect artifact whose meaning nobody in the
///    system defines. Adding a role is a `v2`.
/// 2. **Unknown subject kinds**, through
///    [`DelegationSubjectKind::from_wire`], on exactly the same reasoning:
///    `subject_kind: 3` names a namespace nothing in this version can interpret,
///    so a value carried past the signature would look checked and be nothing of
///    the sort. Adding a kind is a `v3`.
/// 3. **A `(subject_kind, role)` pair this version does not define**, through
///    [`DelegationSubjectKind::permits`]. This is the one rule here that refuses
///    a payload every *individual* field of which is valid — `(domain,
///    speaks-for)` and `(seat, operational)` are the two cases — and it is what
///    makes a subject's authority a property of the artifact rather than of
///    whoever reads it. It is also the only field rule at this level that the
///    caller's re-encode-and-byte-compare cannot mask: the canonical re-encoding
///    of an invalid pair is the invalid pair, so if this check were deleted the
///    artifact-level entry points would accept it.
/// 4. **An empty subject value.** Not because the empty string is malformed, but
///    because of what it does to a *misconfigured verifier*: acceptance is a
///    comparison against the caller's configured subject, so a verifier whose
///    subject is unset matches an empty-subject delegation and accepts it. This
///    is the "no default origin" rule enforced from the other side, and it makes
///    that misconfiguration fail closed rather than silently succeed.
/// 5. **A delegated key strict Ed25519 could never accept** — a non-canonical
///    encoding (`y >= p`, of which all-`0xff` is the easy example) or a
///    small-order point, the identity among them. Such a key cannot verify
///    anything, so a delegation naming it is a signed statement that can never
///    authorise a single artifact. All-zeros, all-`0xff` and the identity point
///    were all accepted before this check, and each produced a "valid"
///    delegation with no possible meaning. The predicate lives in
///    [`crate::keys::identity`] alongside the verification it has to agree with.
///
///    **The obvious implementation of this check does not work, and the reason
///    is a measured property of the dependency rather than a subtlety of the
///    format.** `ed25519_dalek::VerifyingKey::from_bytes([0xff; 32])`
///    **succeeds**, and `is_weak()` on the result returns **false** — dalek's
///    decompression reduces the y-coordinate modulo `p` instead of rejecting an
///    out-of-range one. So `from_bytes(..).is_ok() && !is_weak()`, which is what
///    the name `from_bytes` invites you to trust, accepts all-`0xff`; the first
///    version of this check did exactly that and four tests caught it. An
///    explicit `y < p` comparison is required and lives in
///    [`crate::keys::identity`].
///
///    ⛔ **This check used to be stricter than
///    [`crate::Ed25519Identity::verify`]**, and the note here recorded that as
///    deliberate: `verify_strict` checks the canonical `s` scalar and
///    small-order `R` and `A` but **not** `A`'s y-canonicality, so a key with
///    `y >= p` was refused by this decoder and accepted by the crate's own
///    verifier. That asymmetry is closed — `verify` now applies the same
///    predicate — so the delegation format's notion of a key and the crate's
///    are one notion rather than two that agreed by review.
///
/// A sixth rule bounds `sequence` at [`MAX_SEQUENCE`], so that a successor
/// always exists. Its *monotonicity* is not checked here and cannot be: this
/// function sees one delegation, and "strictly increasing per
/// `(subject_kind, subject_value, role)`" is a property of a set. That, and the equivocation rule, are a fold's
/// obligations, stated in [`super`].
fn decode_payload(payload_raw: &[u8]) -> TrustResult<DelegationClaim> {
    let Value::Map(payload) = parse_value(payload_raw)? else {
        return Err(reject());
    };
    let [
        (subject_kind_key, subject_kind),
        (subject_value_key, subject_value),
        (delegated_key_key, delegated_key),
        (role_key, role),
        (not_before_key, not_before),
        (sequence_key, sequence),
    ] = payload.as_slice()
    else {
        return Err(reject());
    };
    require_integer(subject_kind_key, i128::from(PAYLOAD_LABEL_SUBJECT_KIND))?;
    let subject_kind =
        DelegationSubjectKind::from_wire(unsigned(subject_kind)?).ok_or_else(reject)?;
    require_integer(subject_value_key, i128::from(PAYLOAD_LABEL_SUBJECT_VALUE))?;
    let Value::Text(subject_value) = subject_value else {
        return Err(reject());
    };
    if subject_value.is_empty() {
        return Err(reject());
    }
    require_integer(delegated_key_key, i128::from(PAYLOAD_LABEL_DELEGATED_KEY))?;
    let delegated_public_key: [u8; KEY_LEN] = fixed_bytes(delegated_key)?;
    if !is_usable_ed25519_public_key(&delegated_public_key) {
        return Err(reject());
    }
    require_integer(role_key, i128::from(PAYLOAD_LABEL_ROLE))?;
    let role = DelegationRole::from_wire(unsigned(role)?).ok_or_else(reject)?;
    // THE pair check, and it lives here because this is where a stranger's
    // artifact arrives. `check_encodable` refuses the same pair on the issuing
    // side; the rule itself is defined once, in `DelegationSubjectKind::permits`.
    if !subject_kind.permits(role) {
        return Err(reject());
    }
    require_integer(not_before_key, i128::from(PAYLOAD_LABEL_NOT_BEFORE))?;
    let not_before_unix_ms = unsigned(not_before)?;
    require_integer(sequence_key, i128::from(PAYLOAD_LABEL_SEQUENCE))?;
    let sequence = unsigned(sequence)?;
    if sequence > MAX_SEQUENCE {
        return Err(reject());
    }
    Ok(DelegationClaim {
        subject_kind,
        subject_value: subject_value.clone(),
        delegated_public_key,
        role,
        not_before_unix_ms,
        sequence,
    })
}

/// Decode a delegation into its fields, enforcing the exact
/// `lys/delegation/v1` shape: the input cap; tag 18 over a 4-array; the
/// protected map pinned to `{1: -8, 3: CONTENT_TYPE, 4: usable bstr(32) key}`;
/// an
/// **empty** unprotected map; an embedded `bstr` payload pinned to
/// `{1: known subject kind, 2: non-empty tstr, 3: usable bstr(32) key,
/// 4: known role the kind permits, 5: uint, 6: uint}`; a 64-byte signature.
///
/// Canonical-encoding strictness is the caller's byte-compare — this function
/// accepts what ciborium parses.
///
/// # Errors
///
/// Every failure collapses to [`TrustError::DelegationVerification`].
pub(crate) fn decode_fields(bytes: &[u8]) -> TrustResult<DecodedFields> {
    if bytes.len() > MAX_ARTIFACT_LEN {
        return Err(reject());
    }
    // The tag is mandatory. RFC 9052 §4.2 permits an untagged `COSE_Sign1`
    // "depending on the context"; this context says tagged, because accepting
    // both would give one statement two valid encodings.
    let Value::Tag(COSE_SIGN1_TAG, boxed) = parse_value(bytes)? else {
        return Err(reject());
    };
    let Value::Array(items) = *boxed else {
        return Err(reject());
    };
    let [
        protected_item,
        unprotected_item,
        payload_item,
        signature_item,
    ] = items.as_slice()
    else {
        return Err(reject());
    };

    // The unprotected bucket must be present *and* empty. Present, because a
    // `COSE_Sign1` always has four items; empty, because an unsigned bucket in
    // an artifact that exists to be trusted is attacker-controlled data with no
    // check downstream of it.
    let Value::Map(unprotected) = unprotected_item else {
        return Err(reject());
    };
    if !unprotected.is_empty() {
        return Err(reject());
    }

    let signature: [u8; 64] = fixed_bytes(signature_item)?;

    let Value::Bytes(protected_raw) = protected_item else {
        return Err(reject());
    };
    let root_public_key = decode_protected(protected_raw)?;

    // Embedded, not detached: `nil` here is not a delegation.
    let Value::Bytes(payload_raw) = payload_item else {
        return Err(reject());
    };
    let claim = decode_payload(payload_raw)?;

    Ok(DecodedFields {
        root_public_key,
        claim,
        signature,
    })
}

#[cfg(test)]
#[path = "encoding_tests.rs"]
mod tests;
