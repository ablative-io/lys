#![cfg(test)]
//! Future issue and not-before instants have one bounded tolerance at every decision.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signer, SigningKey};
use lys_pass::{Decision, Error, KeySet, VerifiedPass};
use serde_json::json;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const ISSUER: &str = "https://issuer.example";
const NOW: u64 = 100;

fn key() -> SigningKey {
    SigningKey::from_bytes(&[9; 32])
}

fn keys() -> Result<KeySet> {
    let x = URL_SAFE_NO_PAD.encode(key().verifying_key().to_bytes());
    Ok(KeySet::from_json(
        &json!({"keys": [{"kty": "OKP", "crv": "Ed25519", "kid": "clock", "x": x}]}).to_string(),
    )?)
}

fn token(iat: u64, nbf: Option<u64>, exp: u64) -> String {
    let claims = json!({
        "iss": ISSUER, "sub": "person", "aud": "sample", "iat": iat, "nbf": nbf, "exp": exp,
        "holder": {"id": "person", "kind": "person", "responsible": null},
        "rights": [{"resource": {"kind": "sample.file", "id": "a"},
            "actions": ["read"], "mode": "outright", "grant": "grant"}]
    });
    let header =
        URL_SAFE_NO_PAD.encode(json!({"alg": "EdDSA", "typ": "JWT", "kid": "clock"}).to_string());
    let payload = URL_SAFE_NO_PAD.encode(claims.to_string());
    let input = format!("{header}.{payload}");
    let signature = URL_SAFE_NO_PAD.encode(key().sign(input.as_bytes()).to_bytes());
    format!("{input}.{signature}")
}

fn verify(iat: u64, nbf: Option<u64>, now: u64) -> Result<VerifiedPass> {
    Ok(VerifiedPass::verify(
        &token(iat, nbf, 200),
        &keys()?,
        ISSUER,
        "sample",
        now,
    )?)
}

fn allowed(pass: &VerifiedPass, now: u64) -> Result {
    assert_eq!(
        pass.evaluate("sample.file", "a", "read", now)?,
        Decision::Allowed { grant: "grant" }
    );
    assert!(pass.hot_allowed("sample.file", "a", "read", now)?);
    Ok(())
}

fn refused(error: &Error, skew_ms: u128) {
    assert_eq!(error.name(), "pass_not_yet_valid");
    assert_eq!(
        error.to_string(),
        format!("pass_not_yet_valid: clock skew {skew_ms} ms exceeds 5000 ms")
    );
}

#[test]
fn issued_now_verifies_at_once() -> Result {
    let pass = verify(NOW, Some(NOW), NOW)?;
    assert_eq!(pass.claims().iat, NOW);
    allowed(&pass, NOW)
}

#[test]
fn iat_four_seconds_ahead_verifies_and_allows() -> Result {
    allowed(&verify(NOW + 4, None, NOW)?, NOW)
}

#[test]
fn not_before_four_seconds_ahead_verifies_and_allows() -> Result {
    allowed(&verify(NOW, Some(NOW + 4), NOW)?, NOW)
}

#[test]
fn iat_six_seconds_ahead_refuses_naming_6000_ms() -> Result {
    let error = VerifiedPass::verify(&token(NOW + 6, None, 200), &keys()?, ISSUER, "sample", NOW)
        .err()
        .ok_or("a pass beyond the skew was accepted")?;
    refused(&error, 6000);
    Ok(())
}

#[test]
fn not_before_six_seconds_ahead_refuses_naming_6000_ms() -> Result {
    let error = VerifiedPass::verify(
        &token(NOW, Some(NOW + 6), 200),
        &keys()?,
        ISSUER,
        "sample",
        NOW,
    )
    .err()
    .ok_or("a pass before its validity was accepted")?;
    refused(&error, 6000);
    Ok(())
}

#[test]
fn five_second_boundary_is_inclusive() -> Result {
    allowed(&verify(NOW + 5, Some(NOW + 5), NOW)?, NOW)
}

#[test]
fn evaluating_after_clock_rollback_accepts_four_seconds() -> Result {
    for (iat, nbf) in [(NOW + 4, None), (NOW, Some(NOW + 4))] {
        allowed(&verify(iat, nbf, NOW + 4)?, NOW)?;
    }
    Ok(())
}

#[test]
fn evaluating_after_clock_rollback_refuses_six_seconds() -> Result {
    for (iat, nbf) in [(NOW + 6, None), (NOW, Some(NOW + 6))] {
        let pass = verify(iat, nbf, NOW + 6)?;
        let error = pass
            .evaluate("sample.file", "a", "read", NOW)
            .err()
            .ok_or("evaluation accepted a pass beyond the skew")?;
        refused(&error, 6000);
        let error = pass
            .hot_allowed("sample.file", "a", "read", NOW)
            .err()
            .ok_or("a hot decision accepted a pass beyond the skew")?;
        refused(&error, 6000);
    }
    Ok(())
}

#[test]
fn greatest_future_claim_determines_reported_skew() -> Result {
    let error = VerifiedPass::verify(
        &token(NOW + 6, Some(NOW + 9), 200),
        &keys()?,
        ISSUER,
        "sample",
        NOW,
    )
    .err()
    .ok_or("a future pass was accepted")?;
    refused(&error, 9000);
    Ok(())
}

#[test]
fn extreme_timestamp_reports_skew_without_overflow() -> Result {
    let error = VerifiedPass::verify(
        &token(u64::MAX - 1, None, u64::MAX),
        &keys()?,
        ISSUER,
        "sample",
        0,
    )
    .err()
    .ok_or("an extreme future pass was accepted")?;
    refused(&error, u128::from(u64::MAX - 1) * 1000);
    Ok(())
}

#[test]
fn clock_skew_does_not_extend_expiry() -> Result {
    let pass = verify(NOW, None, NOW)?;
    let error = pass
        .evaluate("sample.file", "a", "read", 200)
        .err()
        .ok_or("evaluation extended expiry")?;
    assert_eq!(error.name(), "pass_expired");
    let error = VerifiedPass::verify(&token(NOW, None, 200), &keys()?, ISSUER, "sample", 200)
        .err()
        .ok_or("verification extended expiry")?;
    assert_eq!(error.name(), "pass_expired");
    Ok(())
}
