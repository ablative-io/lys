#![cfg(test)]
//! Signed passes are trusted only for their issuer, audience and lifetime.

use lys_pass::{Decision, KeySet, VerifiedPass};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn fixture(name: &str) -> Result<String> {
    Ok(std::fs::read_to_string(format!(
        "{}/fixtures/passes/{name}.jwt",
        env!("CARGO_MANIFEST_DIR")
    ))?)
}

fn keys() -> Result<KeySet> {
    Ok(KeySet::from_json(include_str!(
        "../fixtures/passes/keys.json"
    ))?)
}

#[test]
fn parent_reach_is_explicit_and_restricted_child_is_absent() -> Result {
    let token = fixture("valid")?;
    let keys = keys()?;
    let pass = VerifiedPass::verify(&token, &keys, "https://issuer.example", "sample", 100)?;
    assert!(matches!(
        pass.evaluate("sample.file", "child", "read", 100)?,
        Decision::Allowed {
            grant: "parent-grant"
        }
    ));
    assert_eq!(
        pass.evaluate("sample.file", "restricted", "read", 100)?,
        Decision::Refused
    );
    assert_eq!(
        pass.evaluate("sample.file", "child/unplaced", "read", 100)?,
        Decision::Refused
    );
    assert_eq!(pass.rights_in_prefix("sample.file").count(), 2);
    assert_eq!(pass.rights_in_prefix("sample.fi").count(), 0);
    Ok(())
}

#[test]
fn held_modes_are_never_hot_authority() -> Result {
    let token = fixture("valid")?;
    let keys = keys()?;
    let pass = VerifiedPass::verify(&token, &keys, "https://issuer.example", "sample", 100)?;
    assert!(matches!(
        pass.evaluate("sample.file", "held", "write", 100)?,
        Decision::Held { .. }
    ));
    assert!(!pass.hot_allowed("sample.file", "held", "write", 100)?);
    assert!(pass.hot_allowed("sample.file", "child", "read", 100)?);
    Ok(())
}

#[test]
fn hostile_passes_are_refused_by_name() -> Result {
    let keys = keys()?;
    for (name, expected) in [
        ("wrong-audience", "wrong_audience"),
        ("expired", "pass_expired"),
        ("unpublished-key", "unpublished_key"),
        ("tampered", "signature_refused"),
        ("future", "pass_not_yet_valid"),
        ("other-app", "rights_outside_audience"),
    ] {
        let token = fixture(name)?;
        let error = VerifiedPass::verify(&token, &keys, "https://issuer.example", "sample", 100)
            .err()
            .ok_or("hostile pass was accepted")?;
        assert_eq!(error.name(), expected, "{name}");
    }
    Ok(())
}

#[test]
fn previously_verified_pass_is_refused_at_expiry() -> Result {
    let token = fixture("valid")?;
    let keys = keys()?;
    let pass = VerifiedPass::verify(&token, &keys, "https://issuer.example", "sample", 100)?;
    assert_eq!(
        pass.evaluate("sample.file", "child", "read", 200)
            .err()
            .ok_or("expired pass accepted")?
            .name(),
        "pass_expired"
    );
    Ok(())
}

#[test]
fn truncated_rights_are_never_partial_authority() -> Result {
    let token = fixture("truncated")?;
    let keys = keys()?;
    let pass = VerifiedPass::verify(&token, &keys, "https://issuer.example", "sample", 100)?;
    assert_eq!(
        pass.evaluate("sample.file", "child", "read", 100)
            .err()
            .ok_or("truncated pass accepted")?
            .name(),
        "rights_truncated"
    );
    Ok(())
}

#[test]
fn connector_keeps_responsible_person_and_removed_key_refuses() -> Result {
    let token = fixture("connector")?;
    let keys = keys()?;
    let pass = VerifiedPass::verify(&token, &keys, "https://issuer.example", "sample", 100)?;
    assert_eq!(pass.claims().holder.responsible.as_deref(), Some("person"));
    let retired = KeySet::from_json("{\"keys\":[]}")?;
    assert_eq!(
        VerifiedPass::verify(&token, &retired, "https://issuer.example", "sample", 100)
            .err()
            .ok_or("retired key accepted")?
            .name(),
        "unpublished_key"
    );
    Ok(())
}

#[test]
fn malformed_keysets_and_unsigned_headers_are_refused() -> Result {
    assert!(
        KeySet::from_json(
            "{\"keys\":[{\"kty\":\"OKP\",\"crv\":\"Ed25519\",\"kid\":\"fixture\",\"x\":\"AA\"}]}"
        )
        .is_err()
    );
    let keys = keys()?;
    assert!(
        VerifiedPass::verify(
            "eyJhbGciOiJub25lIiwidHlwIjoiSldUIiwia2lkIjoiZml4dHVyZSJ9.e30.AA",
            &keys,
            "https://issuer.example",
            "sample",
            100
        )
        .is_err()
    );
    assert!(
        VerifiedPass::verify(
            "one.two.three.four",
            &keys,
            "https://issuer.example",
            "sample",
            100
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn wrong_issuer_never_trusts_a_pass_from_another_authority() -> Result {
    let token = fixture("valid")?;
    let keys = keys()?;
    assert_eq!(
        VerifiedPass::verify(&token, &keys, "https://other.example", "sample", 100)
            .err()
            .ok_or("wrong issuer accepted")?
            .name(),
        "wrong_issuer"
    );
    Ok(())
}
