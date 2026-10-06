#![cfg(test)]
//! The canonical-encoding rule for Ed25519 public keys: the non-canonical
//! spellings dalek accepts, the predicate that refuses them, and `verify`
//! agreeing with the predicate.

use super::*;

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
                .is_ok_and(|key| key.verify_strict(message, &signature.into()).is_ok());
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
