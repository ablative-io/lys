//! Decoding a `lys/delegation/v1` artifact: the structural parse of the
//! envelope, the protected bucket and the payload, each refusal collapsed
//! to the one non-oracle error the parent module's docs describe.

use ciborium::value::Value;

use super::{
    ALG_EDDSA, CONTENT_TYPE, COSE_SIGN1_TAG, HEADER_LABEL_ALG, HEADER_LABEL_CONTENT_TYPE,
    HEADER_LABEL_KID, KEY_LEN, MAX_SEQUENCE, PAYLOAD_LABEL_DELEGATED_KEY, PAYLOAD_LABEL_NOT_BEFORE,
    PAYLOAD_LABEL_ROLE, PAYLOAD_LABEL_SEQUENCE, PAYLOAD_LABEL_SUBJECT_KIND,
    PAYLOAD_LABEL_SUBJECT_VALUE,
};
use crate::delegation::artifact::{DelegationClaim, DelegationRole, DelegationSubjectKind};
use crate::error::{TrustError, TrustResult};
use crate::keys::identity::is_usable_ed25519_public_key;

/// The fields extracted from a structurally valid delegation.
pub(crate) struct DecodedFields {
    /// Raw 32-byte Ed25519 root key from the protected `kid`. A **claim**, not
    /// an authority — see [`crate::delegation::sign::verify_delegation`].
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
/// closed: [`crate::delegation::sign::verify_delegation`] compares `kid` byte-for-byte
/// against the caller's expected key, so an unusable `kid` could never have
/// matched a key anyone trusts.
pub(super) fn decode_protected(protected_raw: &[u8]) -> TrustResult<[u8; KEY_LEN]> {
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
/// obligations, stated in [`crate::delegation`].
pub(super) fn decode_payload(payload_raw: &[u8]) -> TrustResult<DelegationClaim> {
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
/// `lys/delegation/v1` shape: tag 18 over a 4-array; the
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
