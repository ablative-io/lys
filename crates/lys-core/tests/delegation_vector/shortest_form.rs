#![cfg(test)]
//! The frozen hex encodes the specification table in shortest form, and no
//! vector ties the protected and payload `bstr` heads.

use super::*;

// ---------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------

/// Every frozen vector's hex encodes the §6.1 table, in shortest form, and
/// carries nothing else.
///
/// # What this replaced, and why the old version could not fail
///
/// This was `the_frozen_literals_are_self_consistent`, and it passed over a
/// **stale** vector for as long as one existed. Its lengths (`86`, `66`, `169`,
/// `225`), its `0xa5` map head and its `100`/`90` offsets were genuine
/// specification facts rather than restatements of the hex — but nothing forced
/// them to move when the format did, so when the payload gained a typed subject
/// **both sides went stale together** and agreement survived. **A second party
/// that can go stale in lockstep with what it judges is not a second party.**
///
/// So there is no bare number here describing the hex. Every length is derived —
/// from a §6.1 table constant declared separately above, or from an RFC 8949
/// shortest head computed from the value — and each structure is walked with a
/// cursor that must land **exactly** on `len()`, so a field dropped, added,
/// re-typed or re-headed fails rather than going unexamined. Nothing in this
/// test calls `lys-core`.
///
/// Verified rather than argued: run against the pre-typed-subject vector it fails
/// at the content type. The old version passed.
///
/// # One residual hole, documented rather than hidden
///
/// [`expect_bstr`] derives the `bstr` head from the length of the value it is
/// given, so a truncated `HEX_ROOT_PUBLIC_KEY` is caught only because
/// `HEX_PROTECTED` is a *separate* literal still carrying `5820`. If both were
/// truncated identically this would pass. That is two coordinated paste errors
/// rather than one, and `the_seeds_derive_the_public_keys_the_vectors_name`
/// covers the key values independently.
#[test]
fn the_frozen_hex_encodes_the_specification_table_in_shortest_form() {
    let mut checked = 0;
    for v in all_vectors() {
        let name = v.name;
        let protected = from_hex(v.hex_protected);
        let payload = from_hex(v.hex_payload);
        let preimage = from_hex(v.hex_preimage);
        let signature = from_hex(v.hex_signature);
        let artifact = from_hex(v.hex_artifact);
        let root_key = from_hex(HEX_ROOT_PUBLIC_KEY);
        let delegated_key = from_hex(HEX_DELEGATED_PUBLIC_KEY);

        assert_eq!(
            root_key.len(),
            32,
            "{name}: spec §1.1, kid is a bstr .size 32"
        );
        assert_eq!(
            delegated_key.len(),
            32,
            "{name}: spec §1.2, the delegated key is a bstr .size 32"
        );
        assert_eq!(
            signature.len(),
            64,
            "{name}: RFC 8032, an Ed25519 signature is 64 bytes"
        );
        assert_ne!(
            v.subject_kind_wire, v.role_wire,
            "{name}: spec §0.5 consequence 4 — the two vocabularies must not \
             overlap, or a transposition of payload labels 1 and 4 is invisible \
             in these very bytes"
        );

        // ---- protected: {1: -8, 3: <content type>, 4: <kid>}   (spec §1.1)
        let mut at = 0;
        take(
            &protected,
            &mut at,
            &shortest_head(5, 3),
            &format!("{name}: protected map head — three entries"),
        );
        expect_uint(
            &protected,
            &mut at,
            1,
            &format!("{name}: protected label 1 (alg)"),
        );
        take(
            &protected,
            &mut at,
            &shortest_head(1, 7),
            &format!("{name}: alg = -8 (EdDSA): RFC 8949 major 1, argument -1-(-8) = 7"),
        );
        expect_uint(
            &protected,
            &mut at,
            3,
            &format!("{name}: protected label 3 (content type)"),
        );
        expect_text(
            &protected,
            &mut at,
            CONTENT_TYPE,
            &format!("{name}: the frozen content type"),
        );
        expect_uint(
            &protected,
            &mut at,
            4,
            &format!("{name}: protected label 4 (kid)"),
        );
        expect_bstr(
            &protected,
            &mut at,
            &root_key,
            &format!("{name}: kid — a bstr, not a tstr (RFC 9052 §3.1)"),
        );
        assert_eq!(
            at,
            protected.len(),
            "{name}: the protected bucket carries nothing else"
        );

        // ---- payload   (spec §1.2). The kind precedes the value it types.
        let mut at = 0;
        take(
            &payload,
            &mut at,
            &shortest_head(5, 6),
            &format!("{name}: payload map head — six entries"),
        );
        expect_uint(&payload, &mut at, 1, &format!("{name}: payload label 1"));
        expect_uint(
            &payload,
            &mut at,
            v.subject_kind_wire,
            &format!("{name}: subject_kind"),
        );
        expect_uint(&payload, &mut at, 2, &format!("{name}: payload label 2"));
        expect_text(
            &payload,
            &mut at,
            &v.subject_value,
            &format!("{name}: subject_value"),
        );
        expect_uint(&payload, &mut at, 3, &format!("{name}: payload label 3"));
        expect_bstr(
            &payload,
            &mut at,
            &delegated_key,
            &format!("{name}: the delegated public key"),
        );
        expect_uint(&payload, &mut at, 4, &format!("{name}: payload label 4"));
        expect_uint(&payload, &mut at, v.role_wire, &format!("{name}: role"));
        expect_uint(&payload, &mut at, 5, &format!("{name}: payload label 5"));
        expect_uint(
            &payload,
            &mut at,
            v.not_before_unix_ms,
            &format!("{name}: not_before_unix_ms"),
        );
        expect_uint(&payload, &mut at, 6, &format!("{name}: payload label 6"));
        expect_uint(&payload, &mut at, v.sequence, &format!("{name}: sequence"));
        assert_eq!(
            at,
            payload.len(),
            "{name}: the payload carries nothing else"
        );

        // ---- Sig_structure   (RFC 9052 §4.4, spec §1.0)
        let mut at = 0;
        take(
            &preimage,
            &mut at,
            &shortest_head(4, 4),
            &format!("{name}: Sig_structure is a four-element array"),
        );
        expect_text(
            &preimage,
            &mut at,
            "Signature1",
            &format!("{name}: the RFC 9052 §4.4 context string"),
        );
        expect_bstr(
            &preimage,
            &mut at,
            &protected,
            &format!("{name}: the protected bucket, bstr-wrapped"),
        );
        expect_bstr(
            &preimage,
            &mut at,
            &[],
            &format!("{name}: external_aad must be h'' — spec §1.0"),
        );
        expect_bstr(
            &preimage,
            &mut at,
            &payload,
            &format!("{name}: the embedded payload, bstr-wrapped"),
        );
        assert_eq!(
            at,
            preimage.len(),
            "{name}: the Sig_structure carries nothing else"
        );

        // ---- the tagged artifact   (spec §1)
        let mut at = 0;
        take(
            &artifact,
            &mut at,
            &shortest_head(6, 18),
            &format!("{name}: tag 18 is mandatory — spec §1.0"),
        );
        take(
            &artifact,
            &mut at,
            &shortest_head(4, 4),
            &format!("{name}: COSE_Sign1 is a four-element array"),
        );
        expect_bstr(
            &artifact,
            &mut at,
            &protected,
            &format!("{name}: the protected bucket"),
        );
        take(
            &artifact,
            &mut at,
            &shortest_head(5, 0),
            &format!("{name}: the unprotected bucket is the empty map — spec §1.3"),
        );
        expect_bstr(
            &artifact,
            &mut at,
            &payload,
            &format!("{name}: the embedded payload"),
        );
        expect_bstr(
            &artifact,
            &mut at,
            &signature,
            &format!("{name}: the signature"),
        );
        assert_eq!(
            at,
            artifact.len(),
            "{name}: the artifact carries nothing else, and no trailing bytes"
        );

        checked += 1;
    }
    assert_eq!(checked, 5, "every frozen vector must have been walked");
}

/// The two `bstr` heads inside each `Sig_structure` are distinguishable.
///
/// ⚠️ **This is why vector B's `subject_value` is 32 bytes and not 31.** At 31 the
/// payload is 79 bytes — exactly the protected bucket's length — so both heads
/// read `584f`, and an implementation that derived one length from the other, or
/// wrapped the wrong bucket, would produce identical bytes. The specification's
/// "no value collides with another value" rule was written about payload *fields*
/// and did not reach the envelope; this is that rule applied one level up.
///
/// Vector A satisfies it incidentally (79 against 68). B now satisfies it by
/// construction, and this test is what stops a future edit to `B_SUBJECT_VALUE`
/// from quietly reintroducing the tie.
#[test]
fn no_vector_ties_the_protected_and_payload_bstr_heads() {
    let mut checked = 0;
    for v in all_vectors() {
        let protected = from_hex(v.hex_protected);
        let payload = from_hex(v.hex_payload);
        assert_ne!(
            shortest_head(2, u64::try_from(protected.len()).unwrap()),
            shortest_head(2, u64::try_from(payload.len()).unwrap()),
            "{}: the protected and payload bstr heads are identical ({} bytes \
             each), so a bucket swap or a length derived from the wrong bucket \
             would be invisible in this vector",
            v.name,
            protected.len()
        );
        checked += 1;
    }
    assert_eq!(checked, 5, "every frozen vector must have been checked");
}
