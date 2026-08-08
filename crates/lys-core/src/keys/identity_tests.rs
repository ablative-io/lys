#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// The single environment variable read by [`Ed25519Identity::from_env`].
/// Every env-backed test mutates this and so must run serially.
const TEST_ENV_VAR: &str = "LYS_IDENTITY_KEY";

fn identity_from_seed(seed: [u8; 32]) -> Ed25519Identity {
    Ed25519Identity::from_seed(&Zeroizing::new(seed))
}

// ─── Debug redaction ──────────────────────────────────────────────

#[test]
fn debug_redacts_signing_key() {
    let seed = [7u8; 32];
    let id = identity_from_seed(seed);
    let dbg = format!("{id:?}");
    assert!(dbg.contains("[REDACTED]"), "got: {dbg}");
    assert!(
        !dbg.contains("07, 07, 07"),
        "raw seed bytes leaked in debug (hex): {dbg}"
    );
    assert!(
        !dbg.contains("7, 7, 7, 7"),
        "raw seed bytes leaked in debug (decimal array): {dbg}"
    );
    assert!(
        !dbg.contains("SigningKey("),
        "default SigningKey tuple debug leaked: {dbg}"
    );
    assert!(
        !dbg.contains("SigningKey {"),
        "default SigningKey struct debug leaked: {dbg}"
    );
}

#[test]
fn debug_includes_verifying_key_hex() {
    let seed = [7u8; 32];
    let id = identity_from_seed(seed);
    let dbg = format!("{id:?}");
    let expected_hex = id.public_key_bytes_hex();
    assert!(
        dbg.contains(&expected_hex),
        "verifying key hex missing from debug: {dbg}"
    );
}

// ─── public_key_bytes accessor ────────────────────────────────────

#[test]
fn public_key_bytes_returns_32_bytes() {
    let id = identity_from_seed([1u8; 32]);
    let bytes = id.public_key_bytes();
    assert_eq!(bytes.len(), 32);
}

#[test]
fn public_key_bytes_round_trips_through_verifying_key() {
    let id = identity_from_seed([2u8; 32]);
    let bytes = id.public_key_bytes();
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&bytes).unwrap();
    assert_eq!(vk.to_bytes(), bytes);
}

#[test]
fn public_key_bytes_stable_across_calls() {
    let id = identity_from_seed([3u8; 32]);
    assert_eq!(id.public_key_bytes(), id.public_key_bytes());
}

// ─── sign ─────────────────────────────────────────────────────────

#[test]
fn sign_produces_64_byte_signature() {
    let id = identity_from_seed([4u8; 32]);
    let sig = id.sign(b"hello");
    assert_eq!(sig.len(), 64);
}

#[test]
fn sign_then_verify_roundtrip() {
    let id = identity_from_seed([5u8; 32]);
    let msg = b"hello world";
    let sig = id.sign(msg);
    Ed25519Identity::verify(&id.public_key_bytes(), msg, &sig).unwrap();
}

#[test]
fn sign_different_messages_yields_different_signatures() {
    let id = identity_from_seed([6u8; 32]);
    let sig_a = id.sign(b"message A");
    let sig_b = id.sign(b"message B");
    assert_ne!(sig_a, sig_b);
}

#[test]
fn sign_empty_message() {
    let id = identity_from_seed([8u8; 32]);
    let sig = id.sign(b"");
    assert_eq!(sig.len(), 64);
    Ed25519Identity::verify(&id.public_key_bytes(), b"", &sig).unwrap();
}

// ─── verify ───────────────────────────────────────────────────────

#[test]
fn verify_rejects_tampered_message() {
    let id = identity_from_seed([10u8; 32]);
    let sig = id.sign(b"original");
    let result = Ed25519Identity::verify(&id.public_key_bytes(), b"tampered", &sig);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_wrong_public_key() {
    let id_a = identity_from_seed([11u8; 32]);
    let id_b = identity_from_seed([12u8; 32]);
    let sig = id_a.sign(b"msg");
    let result = Ed25519Identity::verify(&id_b.public_key_bytes(), b"msg", &sig);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_short_signature() {
    let id = identity_from_seed([13u8; 32]);
    let result = Ed25519Identity::verify(&id.public_key_bytes(), b"msg", &[0u8; 32]);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_long_signature() {
    let id = identity_from_seed([14u8; 32]);
    let result = Ed25519Identity::verify(&id.public_key_bytes(), b"msg", &[0u8; 128]);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_empty_signature() {
    let id = identity_from_seed([15u8; 32]);
    let result = Ed25519Identity::verify(&id.public_key_bytes(), b"msg", &[]);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_malformed_public_key() {
    // all-`0xff` is the easy non-canonical encoding: `y = p + 18` once the
    // sign bit is masked. It is now refused by the key rule rather than by
    // the signature check — see the canonical-encoding section below for why
    // that distinction is invisible to this assertion.
    let id = identity_from_seed([16u8; 32]);
    let sig = id.sign(b"msg");
    let result = Ed25519Identity::verify(&[0xff; 32], b"msg", &sig);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_small_order_public_key() {
    // [0u8; 32] encodes the point with y = 0, which lies on the curve and
    // has order 4. dalek's `VerifyingKey::from_bytes` accepts it (it is a
    // valid point encoding), so a small-order rule is the layer that must
    // reject it.
    //
    // That rule now fires in TWO places: `is_usable_ed25519_public_key`'s
    // third condition, checked by `verify` before it decodes the key, and
    // `verify_strict`'s own weak-key check behind it. Deleting either leaves
    // this test passing, so it proves neither alone — it is the end-to-end
    // statement, and the isolating tests are
    // `is_usable_ed25519_public_key_refuses_the_all_zero_key` (which pins
    // that the small-order clause and not the canonical-y clause is what
    // refuses it) and the dependency pin below.
    let weak_pk = [0u8; 32];
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&weak_pk)
        .expect("y=0 small-order point is a valid encoding dalek accepts");
    assert!(vk.is_weak(), "y=0 point must be classified as weak");

    let id = identity_from_seed([17u8; 32]);
    let sig = id.sign(b"msg");
    let result = Ed25519Identity::verify(&weak_pk, b"msg", &sig);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_identity_point_forgery_that_passes_non_strict() {
    use ed25519_dalek::Verifier;

    // The Edwards identity point (order 1) encodes as y = 1: [1, 0, ..., 0].
    // For a public key A equal to the identity, k·A is the identity for any
    // hash scalar k, so the verification equation s·B = R + k·A reduces to
    // s·B = R. The forged signature (R = basepoint, s = 1) therefore passes
    // NON-strict verification for ANY message — total forgery. Strict
    // verification rejects the small-order public key outright, which is why
    // Ed25519Identity::verify uses verify_strict.
    let mut weak_pk = [0u8; 32];
    weak_pk[0] = 1;

    // Compressed Ed25519 basepoint.
    let basepoint: [u8; 32] = [
        0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66,
    ];
    let mut forged_sig = [0u8; 64];
    forged_sig[..32].copy_from_slice(&basepoint);
    forged_sig[32] = 1; // s = 1, little-endian

    // Sanity: the forgery really does pass dalek's non-strict verify, for
    // two unrelated messages. This is the exact hole verify_strict closes.
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&weak_pk).unwrap();
    let sig = ed25519_dalek::Signature::from_bytes(&forged_sig);
    vk.verify(b"any message at all", &sig)
        .expect("non-strict verify accepts the small-order forgery");
    vk.verify(b"a completely different message", &sig)
        .expect("non-strict verify accepts the forgery for every message");

    // Our verify must reject it. Since the key rule was added this is also a
    // two-place rejection — `is_usable_ed25519_public_key` refuses the
    // identity point, and `verify_strict` refuses it again — so this test
    // states the outcome and proves neither layer on its own.
    let result = Ed25519Identity::verify(&weak_pk, b"any message at all", &forged_sig);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

// ─── canonical public-key encoding ────────────────────────────────
//
// `verify` accepts a public key iff `is_usable_ed25519_public_key` does. The
// clause that made that a *narrowing* is condition 1, canonical `y`: dalek
// reduces an out-of-range y-coordinate modulo `p` instead of rejecting it, so
// `y` and `y + p` were two 32-byte strings denoting one key.
//
// **Read this before adding a test here.** No test can distinguish the old
// `verify` from the new one through `verify`'s return value, and that is a
// theorem rather than a gap in the suite. A non-canonical encoding reduces to a
// point with `y <= 18`; for `verify` to have returned `Ok` under one, somebody
// must hold the discrete logarithm of that point. So the whole newly-refused
// set returned `Err` before the change and returns `Err` after it, and the
// change is observable only at the layer that decides key-ness. That layer is
// tested directly below, the dependency's behaviour — the thing that made the
// defect real — is pinned rather than described, and the indistinguishability
// itself is checked by `the_narrowing_is_unreachable_through_verify` rather
// than left as an argument in a comment.

/// Number of 32-byte strings whose masked y-coordinate is out of range:
/// `y ∈ {p, …, p + 18}`, each with the sign bit clear and set. `p + 18` is
/// `2²⁵⁵ − 1`, so this is the complete set — there is no nineteenth value.
const NON_CANONICAL_SPELLINGS: usize = 38;

/// How many of those `VerifyingKey::from_bytes` decodes at all. **Measured
/// against ed25519-dalek 2.2.0 / curve25519-dalek 4.1.3**, not derived: the
/// other 14 reduce to a `y` with no corresponding curve point.
const DALEK_DECODES: usize = 24;

/// How many of *those* dalek then reports as non-weak — the set `verify`
/// accepted as verifying keys before the canonical-`y` rule, and the exact set
/// the rule newly refuses. The remaining 4 (`y = p`, `y = p + 1`, both signs)
/// reduce to the order-4 and identity points and were already refused by
/// `verify_strict`.
const DALEK_DECODES_NON_WEAK: usize = 20;

/// `p + k` little-endian, for `k <= 18`. `p`'s least significant byte is `0xed`
/// and `0xed + 18 == 0xff`, so the addition touches byte 0 only.
fn non_canonical_y(k: u8, sign_bit: bool) -> [u8; 32] {
    let mut bytes = CURVE25519_FIELD_MODULUS_LE;
    bytes[0] += k;
    if sign_bit {
        bytes[31] |= 0x80;
    }
    bytes
}

/// The canonical spelling of the point `p + k` reduces to: `y = k`.
fn canonical_y(k: u8, sign_bit: bool) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[0] = k;
    if sign_bit {
        bytes[31] |= 0x80;
    }
    bytes
}

/// Every non-canonical spelling, paired with the canonical spelling of the
/// point it reduces to.
fn non_canonical_spellings() -> impl Iterator<Item = ([u8; 32], [u8; 32])> {
    (0u8..=18).flat_map(|k| {
        [false, true]
            .into_iter()
            .map(move |sign| (non_canonical_y(k, sign), canonical_y(k, sign)))
    })
}

/// **The positive control, and the second party.**
///
/// The defect is a property of ed25519-dalek, so this test asserts the
/// dependency's behaviour rather than ours: that it *accepts* the encodings the
/// rule above exists to refuse, and that each is genuinely a second spelling of
/// a key that already has one. Without this, the refusal tests below would be a
/// suite made only of refusals, which cannot tell a working rule from a `return
/// false`.
///
/// It is also a live pin. If a future dalek starts rejecting `y >= p` itself,
/// this test fails and says so, rather than the rule quietly becoming dead code
/// nobody re-examines.
#[test]
fn dalek_accepts_the_non_canonical_encodings_this_rule_exists_to_refuse() {
    let mut decoded = 0;
    let mut decoded_non_weak = 0;
    let mut seen = 0;

    for (non_canonical, canonical) in non_canonical_spellings() {
        seen += 1;
        assert_ne!(
            non_canonical, canonical,
            "the two spellings must be different byte strings"
        );
        let from_non_canonical = ed25519_dalek::VerifyingKey::from_bytes(&non_canonical);
        let from_canonical = ed25519_dalek::VerifyingKey::from_bytes(&canonical);
        assert_eq!(
            from_non_canonical.is_ok(),
            from_canonical.is_ok(),
            "the two spellings must decode alike, or they are not one point"
        );
        let (Ok(non_canonical_key), Ok(canonical_key)) = (from_non_canonical, from_canonical)
        else {
            continue;
        };
        decoded += 1;

        // Same point, two encodings — measured, not assumed. The Montgomery
        // u-coordinate is a function of y alone, so equality here is exactly
        // the statement that dalek reduced `p + k` to `k`.
        assert_eq!(
            non_canonical_key.to_montgomery().to_bytes(),
            canonical_key.to_montgomery().to_bytes(),
            "the non-canonical spelling must denote the same point as the canonical one"
        );
        // And dalek hands the non-canonical bytes straight back, so the second
        // spelling survives a round trip through the key type.
        assert_eq!(
            non_canonical_key.to_bytes(),
            non_canonical,
            "dalek preserves the non-canonical spelling verbatim"
        );

        if !non_canonical_key.is_weak() {
            decoded_non_weak += 1;
        }
    }

    assert_eq!(seen, NON_CANONICAL_SPELLINGS, "the sweep must be complete");
    assert_eq!(
        decoded, DALEK_DECODES,
        "ed25519-dalek's acceptance of out-of-range y-coordinates has changed"
    );
    assert_eq!(
        decoded_non_weak, DALEK_DECODES_NON_WEAK,
        "the set this rule newly refuses has changed size"
    );
}

/// The comparator, isolated at its boundary. `y == p` is the only case that
/// separates `<` from `<=`, and it is the encoding of zero that dalek accepts;
/// nothing else in this file fails if that one comparison is loosened.
#[test]
fn is_canonical_y_coordinate_is_exclusive_at_the_modulus() {
    let mut modulus_minus_one = CURVE25519_FIELD_MODULUS_LE;
    modulus_minus_one[0] -= 1;

    for sign_bit in [false, true] {
        let with_sign = |mut bytes: [u8; 32]| {
            if sign_bit {
                bytes[31] |= 0x80;
            }
            bytes
        };
        assert!(is_canonical_y_coordinate(&with_sign([0u8; 32])), "y = 0");
        assert!(
            is_canonical_y_coordinate(&with_sign(canonical_y(1, false))),
            "y = 1"
        );
        assert!(
            is_canonical_y_coordinate(&with_sign(modulus_minus_one)),
            "y = p - 1 is the largest canonical y"
        );
        assert!(
            !is_canonical_y_coordinate(&with_sign(CURVE25519_FIELD_MODULUS_LE)),
            "y = p is a non-canonical encoding of zero and must be refused"
        );
    }
}

/// Every non-canonical spelling is refused, including the 20 that dalek would
/// have handed to `verify_strict` as a perfectly good key.
///
/// This is the test the canonical-`y` clause exists for: delete that clause and
/// exactly the assertion below fires, on the first `k` whose point is on the
/// curve and not small-order.
#[test]
fn is_usable_ed25519_public_key_refuses_every_non_canonical_spelling() {
    let mut refused = 0;
    let mut refused_that_dalek_would_accept = 0;

    for (non_canonical, _canonical) in non_canonical_spellings() {
        assert!(
            !is_usable_ed25519_public_key(&non_canonical),
            "a non-canonical y-coordinate was accepted as a key: {}",
            crate::hex_lower(&non_canonical)
        );
        refused += 1;
        if let Ok(key) = ed25519_dalek::VerifyingKey::from_bytes(&non_canonical)
            && !key.is_weak()
        {
            refused_that_dalek_would_accept += 1;
        }
    }

    assert_eq!(
        refused, NON_CANONICAL_SPELLINGS,
        "every spelling must have been tried"
    );
    // Count what fired: without this, a sweep that only ever met encodings
    // dalek already rejects would satisfy every assertion above.
    assert_eq!(
        refused_that_dalek_would_accept, DALEK_DECODES_NON_WEAK,
        "this rule must be the only thing refusing these, or it proves nothing"
    );
}

/// The all-zero key, refused by the small-order clause and **not** by the
/// canonical-`y` one — asserted, so that this test cannot pass for the wrong
/// reason if the two clauses are ever confused.
#[test]
fn is_usable_ed25519_public_key_refuses_the_all_zero_key() {
    let all_zero = [0u8; 32];
    assert!(
        is_canonical_y_coordinate(&all_zero),
        "y = 0 is canonical; this key must be refused by the small-order clause"
    );
    assert!(
        ed25519_dalek::VerifyingKey::from_bytes(&all_zero).is_ok(),
        "dalek decodes the all-zero key, so decoding is not what refuses it either"
    );
    assert!(!is_usable_ed25519_public_key(&all_zero));
}

/// The identity point, likewise: canonical, decodable, small-order.
#[test]
fn is_usable_ed25519_public_key_refuses_the_identity_point() {
    let identity = canonical_y(1, false);
    assert!(is_canonical_y_coordinate(&identity), "y = 1 is canonical");
    assert!(
        ed25519_dalek::VerifyingKey::from_bytes(&identity).is_ok(),
        "dalek decodes the identity point"
    );
    assert!(!is_usable_ed25519_public_key(&identity));

    // The order-2 point, y = p - 1: the other small-order value with a
    // canonical encoding, and the one an off-by-one at the modulus would put
    // on the wrong side.
    let mut order_two = CURVE25519_FIELD_MODULUS_LE;
    order_two[0] -= 1;
    assert!(is_canonical_y_coordinate(&order_two));
    assert!(!is_usable_ed25519_public_key(&order_two));
}

/// A rule made only of refusals cannot tell a working predicate from `return
/// false`. Real keys must be accepted.
#[test]
fn is_usable_ed25519_public_key_accepts_real_keys() {
    let mut accepted = 0;
    for seed in 0u8..16 {
        let public_key = identity_from_seed([seed; 32]).public_key_bytes();
        assert!(
            is_usable_ed25519_public_key(&public_key),
            "a freshly derived public key must be usable"
        );
        accepted += 1;
    }
    assert_eq!(accepted, 16, "the control must have fired");
}

/// `verify` refuses every non-canonical spelling.
///
/// **This assertion held before the rule existed too**, for the reason set out
/// at the top of this section: no signature that verifies under any of these
/// keys is constructible, so the outcome was already `Err`. It is kept as the
/// end-to-end statement of the invariant — the thing a stranger reads `verify`
/// as promising — and not as evidence that the rule works. The evidence is
/// `is_usable_ed25519_public_key_refuses_every_non_canonical_spelling` above.
#[test]
fn verify_refuses_every_non_canonical_public_key_spelling() {
    let id = identity_from_seed([31u8; 32]);
    let signature = id.sign(b"msg");

    let mut refused = 0;
    for (non_canonical, _canonical) in non_canonical_spellings() {
        assert!(
            matches!(
                Ed25519Identity::verify(&non_canonical, b"msg", &signature),
                Err(TrustError::InvalidSignature)
            ),
            "verify accepted a non-canonically encoded public key: {}",
            crate::hex_lower(&non_canonical)
        );
        refused += 1;
    }
    assert_eq!(refused, NON_CANONICAL_SPELLINGS);
}

/// The narrowing changes no outcome anyone can reach — **checked, not argued**.
///
/// This is the load-bearing claim of the whole change, since `verify` is
/// shipped, ungated and semver-bound: the 20 encodings the rule newly refuses
/// were already refused *in practice*, one layer further down, because no
/// signature verifying under any of them is constructible. The test runs the
/// pre-rule path — `from_bytes` then `verify_strict`, which is exactly what
/// `verify` used to be — beside the current one and requires they agree on
/// every spelling. Prose claiming this would be prose that can quietly go
/// stale; a failure here would mean the narrowing had become reachable and the
/// release note is wrong.
#[test]
fn the_narrowing_is_unreachable_through_verify() {
    let mut compared = 0;
    for seed in 0u8..4 {
        let id = identity_from_seed([70 + seed; 32]);
        let message: &[u8] = b"a message somebody actually signed";
        let signature = id.sign(message);

        for (non_canonical, _canonical) in non_canonical_spellings() {
            // The pre-rule path, spelled out rather than referenced, so that
            // editing `verify` cannot silently edit the baseline it is being
            // compared against.
            let before = ed25519_dalek::VerifyingKey::from_bytes(&non_canonical)
                .ok()
                .is_some_and(|key| key.verify_strict(message, &signature.into()).is_ok());
            let after = Ed25519Identity::verify(&non_canonical, message, &signature).is_ok();
            assert!(
                !before,
                "a non-canonical key that the OLD path accepted now exists — the \
                 narrowing is reachable and this change is no longer behaviour-preserving \
                 on constructible input: {}",
                crate::hex_lower(&non_canonical)
            );
            assert_eq!(before, after, "old and new paths must agree");
            compared += 1;
        }
    }
    assert_eq!(
        compared,
        4 * NON_CANONICAL_SPELLINGS,
        "the comparison must have fired for every spelling under every signature"
    );
}

/// The other direction, and the one that *does* fail if the call site in
/// `verify` is wired up wrongly: narrowing must not have narrowed anything
/// else. A negated, misspelled or over-strict predicate at that call site
/// breaks every genuine verification, and this is the test that says so.
#[test]
fn verify_still_accepts_every_key_the_predicate_accepts() {
    let mut verified = 0;
    for seed in 0u8..16 {
        let id = identity_from_seed([seed; 32]);
        let public_key = id.public_key_bytes();
        assert!(is_usable_ed25519_public_key(&public_key));
        let signature = id.sign(b"the message");
        Ed25519Identity::verify(&public_key, b"the message", &signature)
            .expect("a genuine signature under a usable key must still verify");
        verified += 1;
    }
    assert_eq!(verified, 16, "the control must have fired");
}

// ─── load_or_generate ─────────────────────────────────────────────

#[test]
fn load_or_generate_creates_file_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    assert!(!path.exists());

    let id = Ed25519Identity::load_or_generate(&path).unwrap();
    assert!(path.exists());

    let contents = std::fs::read(&path).unwrap();
    assert_eq!(contents.len(), 32, "file must be exactly 32 bytes");

    let _ = id.public_key_bytes();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "generated key file must be 0600");
    }
}

#[test]
fn load_or_generate_loads_existing_valid_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    let seed = [5u8; 32];
    std::fs::write(&path, seed).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let id = Ed25519Identity::load_or_generate(&path).unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(id.public_key_bytes(), expected_pk);
}

// ─── load (load-only) ─────────────────────────────────────────────

#[test]
fn load_reads_existing_valid_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    let seed = [5u8; 32];
    std::fs::write(&path, seed).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let id = Ed25519Identity::load(&path).unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(id.public_key_bytes(), expected_pk);
}

#[test]
fn load_refuses_missing_file_and_creates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    assert!(!path.exists());

    let err = Ed25519Identity::load(&path).unwrap_err();
    assert!(
        matches!(err, TrustError::KeyManagement { .. }),
        "got: {err:?}"
    );
    assert!(
        !path.exists(),
        "load-only constructor must never create a key file"
    );
}

#[test]
fn load_rejects_wrong_length() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [1u8; 16]).unwrap();

    let err = Ed25519Identity::load(&path).unwrap_err();
    assert!(
        matches!(err, TrustError::KeyManagement { .. }),
        "got: {err:?}"
    );
}

#[test]
fn load_or_generate_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");

    let id1 = Ed25519Identity::load_or_generate(&path).unwrap();
    let id2 = Ed25519Identity::load_or_generate(&path).unwrap();
    assert_eq!(id1.public_key_bytes(), id2.public_key_bytes());
}

#[test]
fn load_or_generate_rejects_wrong_length() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [0u8; 16]).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let err = Ed25519Identity::load_or_generate(&path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("invalid length"), "got: {msg}");
    assert!(msg.contains("expected 32 bytes, got 16"), "got: {msg}");
}

#[test]
fn load_or_generate_rejects_oversized() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [0u8; 64]).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let err = Ed25519Identity::load_or_generate(&path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("invalid length"), "got: {msg}");
    assert!(msg.contains("got 64"), "got: {msg}");
}

#[cfg(unix)]
#[test]
fn load_or_generate_warns_but_loads_on_loose_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    let seed = [9u8; 32];
    std::fs::write(&path, seed).unwrap();
    chmod(&path, 0o644);

    let id = Ed25519Identity::load_or_generate(&path).unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(id.public_key_bytes(), expected_pk);
}

#[test]
fn load_or_generate_creates_parent_dir() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("deep").join("identity.key");
    assert!(!path.parent().unwrap().exists());

    let _id = Ed25519Identity::load_or_generate(&path).unwrap();
    assert!(path.exists());
    assert!(path.parent().unwrap().exists());
}

#[test]
fn load_or_generate_rejects_no_filename_path() {
    let err = Ed25519Identity::load_or_generate(Path::new("")).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no filename component"), "got: {msg}");
}

#[test]
fn load_or_generate_concurrent_threads_agree_on_persisted_seed() {
    // The concurrency invariant: racing generators in the same process must
    // all return the identity of the single seed that ends up on disk. No
    // caller may hold an identity that diverges from the persisted file, and
    // no tmp files may be left behind.
    const THREADS: usize = 8;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");

    let barrier = std::sync::Barrier::new(THREADS);
    let public_keys: Vec<[u8; 32]> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    Ed25519Identity::load_or_generate(&path)
                        .expect("concurrent load_or_generate must succeed")
                        .public_key_bytes()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("thread panicked"))
            .collect()
    });

    // Every thread must agree with the seed that was actually persisted.
    let persisted = std::fs::read(&path).unwrap();
    assert_eq!(persisted.len(), 32);
    let mut persisted_seed = [0u8; 32];
    persisted_seed.copy_from_slice(&persisted);
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&persisted_seed)
        .verifying_key()
        .to_bytes();
    for (i, pk) in public_keys.iter().enumerate() {
        assert_eq!(
            *pk, expected_pk,
            "thread {i} returned an identity that diverges from the persisted seed"
        );
    }

    // No temp files may survive the race.
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| *name != *"identity.key")
        .collect();
    assert!(
        leftovers.is_empty(),
        "tmp files left behind after concurrent generation: {leftovers:?}"
    );
}

#[test]
fn generate_and_persist_lost_race_loads_winner_seed() {
    // Deterministic replay of the publish race: the key file appears after
    // the `path.exists()` check in load_or_generate but before the
    // no-clobber publish. Calling the private generate_and_persist with the
    // file already present exercises exactly that window — the caller must
    // get the WINNER's identity back, and the file must not be clobbered.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    let winner_seed = [42u8; 32];
    std::fs::write(&path, winner_seed).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let id = generate_and_persist(&path).unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&winner_seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(
        id.public_key_bytes(),
        expected_pk,
        "loser of the publish race must return the persisted (winner) identity"
    );

    // The winner's file must be untouched and no tmp files left behind.
    assert_eq!(std::fs::read(&path).unwrap(), winner_seed);
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| *name != *"identity.key")
        .collect();
    assert!(
        leftovers.is_empty(),
        "tmp files left behind after lost publish race: {leftovers:?}"
    );
}

#[test]
fn generate_and_persist_lost_race_surfaces_invalid_winner_file() {
    // If the concurrently persisted file is corrupt (wrong length), the
    // losing generator must surface that loudly instead of silently
    // returning its own unpersisted identity.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [7u8; 16]).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let err = generate_and_persist(&path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("invalid length"), "got: {msg}");
    assert!(msg.contains("expected 32 bytes, got 16"), "got: {msg}");
}

// ─── from_env ─────────────────────────────────────────────────────

#[test]
#[serial_test::serial]
#[allow(unsafe_code)]
fn from_env_loads_valid_base64_standard() {
    let seed = [9u8; 32];
    let encoded = STANDARD.encode(seed);
    // SAFETY: env mutation serialised via #[serial]; cleaned up by the guard.
    unsafe { std::env::set_var(TEST_ENV_VAR, &encoded) };
    let _guard = EnvCleanup;

    let id = Ed25519Identity::from_env().unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(id.public_key_bytes(), expected_pk);

    let sig = id.sign(b"test");
    Ed25519Identity::verify(&id.public_key_bytes(), b"test", &sig).unwrap();
}

#[test]
#[serial_test::serial]
#[allow(unsafe_code)]
fn from_env_loads_valid_base64_urlsafe() {
    let seed = [3u8; 32];
    let encoded = URL_SAFE_NO_PAD.encode(seed);
    // SAFETY: env mutation serialised via #[serial]; cleaned up by the guard.
    unsafe { std::env::set_var(TEST_ENV_VAR, &encoded) };
    let _guard = EnvCleanup;

    let id = Ed25519Identity::from_env().unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(id.public_key_bytes(), expected_pk);
}

#[test]
#[serial_test::serial]
#[allow(unsafe_code)]
fn from_env_missing_var_returns_key_management_error() {
    // SAFETY: env mutation serialised via #[serial].
    unsafe { std::env::remove_var(TEST_ENV_VAR) };

    let err = Ed25519Identity::from_env().unwrap_err();
    assert!(matches!(err, TrustError::KeyManagement { .. }));
    let msg = err.to_string();
    assert!(msg.contains(TEST_ENV_VAR), "got: {msg}");
    assert!(msg.contains("not set"), "got: {msg}");
}

#[test]
#[serial_test::serial]
#[allow(unsafe_code)]
fn from_env_invalid_base64_returns_key_management_error() {
    // SAFETY: env mutation serialised via #[serial]; cleaned up by the guard.
    unsafe { std::env::set_var(TEST_ENV_VAR, "not-base64!!!@@") };
    let _guard = EnvCleanup;

    let err = Ed25519Identity::from_env().unwrap_err();
    assert!(matches!(err, TrustError::KeyManagement { .. }));
    let msg = err.to_string();
    assert!(msg.contains("invalid base64"), "got: {msg}");
}

#[test]
#[serial_test::serial]
#[allow(unsafe_code)]
fn from_env_wrong_length_decoded_returns_key_management_error() {
    let encoded = STANDARD.encode([1u8; 16]);
    // SAFETY: env mutation serialised via #[serial]; cleaned up by the guard.
    unsafe { std::env::set_var(TEST_ENV_VAR, &encoded) };
    let _guard = EnvCleanup;

    let err = Ed25519Identity::from_env().unwrap_err();
    assert!(matches!(err, TrustError::KeyManagement { .. }));
    let msg = err.to_string();
    assert!(
        msg.contains("decoded to 16 bytes, expected 32"),
        "got: {msg}"
    );
}

// ─── X25519 derivation ────────────────────────────────────────────

/// Golden vector for the Ed25519→X25519 derivation.
///
/// Every other test in this section is self-consistent — deterministic,
/// matches its own secret, differs from another identity, agrees under DH —
/// so all of them would pass unchanged if the derivation itself moved. This
/// one pins it to a constant, and it is the guard that makes touching this
/// code safe: the X25519 public key is what senders seal to, so a change
/// here silently renders every previously sealed envelope undecryptable.
///
/// A failure means the derivation moved. Do not update the constant to make
/// it pass — that would be recording the break rather than catching it.
#[test]
fn x25519_public_key_matches_the_pinned_golden_vector() {
    let id = identity_from_seed([7u8; 32]);
    assert_eq!(
        crate::hex_lower(&id.x25519_public_key()),
        "761d88ec830413919dfe9d4d1d56f17e653c8c994082df5b137b90a0ae6edf74",
        "the Ed25519-to-X25519 derivation changed; every existing sealed \
         envelope addressed to this identity would become undecryptable"
    );
}

/// A sealed payload survives the round trip through the derived secret.
///
/// The golden vector above pins the public half; this pins that the private
/// half still opens what that public half seals. Together they cover both
/// directions of the derivation, which is what a zeroize refactor could
/// plausibly disturb without changing any signature.
#[test]
fn payload_sealed_to_the_derived_public_key_still_opens() {
    let id = identity_from_seed([7u8; 32]);
    let envelope = crate::seal::seal(b"round trip", &id.x25519_public_key()).unwrap();
    let opened = crate::seal::open(&envelope, &id.x25519_static_secret()).unwrap();
    assert_eq!(opened.as_slice(), b"round trip");
}

#[test]
fn x25519_public_key_matches_derived_static_secret() {
    let id = identity_from_seed([21u8; 32]);
    let secret = id.x25519_static_secret();
    let expected = x25519_dalek::PublicKey::from(&secret).to_bytes();
    assert_eq!(id.x25519_public_key(), expected);
}

#[test]
fn x25519_public_key_is_deterministic() {
    let id = identity_from_seed([22u8; 32]);
    assert_eq!(id.x25519_public_key(), id.x25519_public_key());
}

#[test]
fn different_identities_produce_different_x25519_public_keys() {
    let id_a = identity_from_seed([23u8; 32]);
    let id_b = identity_from_seed([24u8; 32]);
    assert_ne!(id_a.x25519_public_key(), id_b.x25519_public_key());
}

#[test]
fn x25519_static_secret_agrees_across_identities() {
    // Diffie–Hellman sanity: two identities derive the same shared secret
    // from each other's X25519 public keys.
    let id_a = identity_from_seed([25u8; 32]);
    let id_b = identity_from_seed([26u8; 32]);

    let a_secret = id_a.x25519_static_secret();
    let b_secret = id_b.x25519_static_secret();

    let b_public = x25519_dalek::PublicKey::from(id_b.x25519_public_key());
    let a_public = x25519_dalek::PublicKey::from(id_a.x25519_public_key());

    let shared_ab = a_secret.diffie_hellman(&b_public);
    let shared_ba = b_secret.diffie_hellman(&a_public);
    assert_eq!(shared_ab.as_bytes(), shared_ba.as_bytes());
}

// ─── IO failure paths ─────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn load_or_generate_read_permission_denied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [1u8; 32]).unwrap();
    chmod(&path, 0o000);

    let err = Ed25519Identity::load_or_generate(&path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("failed to read"), "got: {msg}");

    chmod(&path, 0o600);
}

#[cfg(unix)]
#[test]
fn load_or_generate_write_permission_denied() {
    let dir = tempfile::tempdir().unwrap();
    let restricted = dir.path().join("noaccess");
    std::fs::create_dir(&restricted).unwrap();
    chmod(&restricted, 0o000);

    let path = restricted.join("identity.key");
    let err = Ed25519Identity::load_or_generate(&path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("failed to write"), "got: {msg}");

    chmod(&restricted, 0o755);

    // The failed generation must not leave an orphaned tmp key file behind.
    let leftovers: Vec<_> = std::fs::read_dir(&restricted)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(
        leftovers.is_empty(),
        "orphaned files left after failed key generation: {leftovers:?}"
    );
}

// ─── test helpers ─────────────────────────────────────────────────

#[cfg(unix)]
fn chmod(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(path, perms).unwrap();
}

/// Removes [`TEST_ENV_VAR`] on drop so env-backed tests leave no residue.
#[allow(unsafe_code)]
struct EnvCleanup;

#[allow(unsafe_code)]
impl Drop for EnvCleanup {
    fn drop(&mut self) {
        // SAFETY: env mutation serialised via #[serial].
        unsafe { std::env::remove_var(TEST_ENV_VAR) };
    }
}
