#![cfg(test)]
//! A pass's grant binding (DIRECTORY-089 R2) is trusted only for the exact
//! pass, issuer, holder, audience, lifetime and grant log it names, with
//! one complete, acyclic ancestry per right; each substitution is refused
//! by name and the pass itself verifies unchanged.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signer, SigningKey};
use lys_pass::binding::{
    BINDING_ANCESTRY, BINDING_LOG_MISMATCH, BINDING_MISMATCH, BINDING_TYPE, BINDING_UNSUPPORTED,
    VerifiedBinding, pass_digest, required,
};
use lys_pass::{GrantLog, KeySet, VerifiedPass};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const ISSUER: &str = "https://issuer.example";

fn key() -> SigningKey {
    SigningKey::from_bytes(&[9; 32])
}

fn keys() -> Result<KeySet> {
    let x = URL_SAFE_NO_PAD.encode(key().verifying_key().to_bytes());
    Ok(KeySet::from_json(
        &json!({"keys": [{"kty": "OKP", "crv": "Ed25519", "kid": "k1", "x": x}]}).to_string(),
    )?)
}

fn signed(typ: &str, claims: &Value) -> String {
    let header =
        URL_SAFE_NO_PAD.encode(json!({"alg": "EdDSA", "typ": typ, "kid": "k1"}).to_string());
    let payload = URL_SAFE_NO_PAD.encode(claims.to_string());
    let input = format!("{header}.{payload}");
    let signature = URL_SAFE_NO_PAD.encode(key().sign(input.as_bytes()).to_bytes());
    format!("{input}.{signature}")
}

fn pass_claims(holder: &str) -> Value {
    json!({
        "iss": ISSUER, "sub": holder, "aud": "sample", "iat": 100, "exp": 200,
        "holder": {"id": holder, "kind": "person", "responsible": null},
        "rights": [
            {"resource": {"kind": "sample.file", "id": "a"}, "actions": ["read"], "mode": "outright", "grant": "grant-child"},
            {"resource": {"kind": "sample.file", "id": "b"}, "actions": ["read"], "mode": "outright", "grant": "grant-other"}
        ]
    })
}

fn log() -> GrantLog {
    GrantLog {
        identity: "issuer.example/grants#1".to_owned(),
        epoch: 0,
    }
}

fn binding_claims(token: &str) -> Value {
    json!({
        "binding": 1, "iss": ISSUER, "sub": "person-1", "aud": "sample", "iat": 100, "exp": 200,
        "pass": pass_digest(token),
        "log": {"identity": "issuer.example/grants#1", "epoch": 0},
        "revision": 7,
        "dependencies": [
            {"grant": "grant-child", "path": ["grant-child", "grant-parent", "grant-root"]},
            {"grant": "grant-other", "path": ["grant-other"]}
        ]
    })
}

fn refusal(binding: &str, token: &str, pass: &VerifiedPass) -> Result<String> {
    Ok(
        VerifiedBinding::verify(binding, token, pass, &keys()?, &log())
            .err()
            .ok_or("a substituted binding was accepted")?
            .name()
            .to_owned(),
    )
}

#[test]
fn a_binding_maps_each_right_to_its_complete_ancestry() -> Result {
    let token = signed("JWT", &pass_claims("person-1"));
    let pass = VerifiedPass::verify(&token, &keys()?, ISSUER, "sample", 150)?;
    let binding = signed(BINDING_TYPE, &binding_claims(&token));
    let verified = VerifiedBinding::verify(&binding, &token, &pass, &keys()?, &log())?;
    assert_eq!(verified.revision(), 7);
    let child = verified.dependency("grant-child").ok_or("a dependency")?;
    assert_eq!(child.path, ["grant-child", "grant-parent", "grant-root"]);
    assert!(
        verified.dependency("grant-root").is_none(),
        "an ancestor is not a right"
    );
    Ok(())
}

#[test]
fn each_substitution_is_refused_by_name() -> Result {
    let token = signed("JWT", &pass_claims("person-1"));
    let pass = VerifiedPass::verify(&token, &keys()?, ISSUER, "sample", 150)?;
    let base = binding_claims(&token);
    let changed = |pointer: &str, value: Value| -> Result<String> {
        let mut claims = base.clone();
        *claims.pointer_mut(pointer).ok_or("a member")? = value;
        Ok(signed(BINDING_TYPE, &claims))
    };
    let other = signed("JWT", &pass_claims("person-2"));
    for (pointer, value, expected) in [
        ("/pass", json!(pass_digest(&other)), BINDING_MISMATCH),
        ("/sub", json!("person-2"), BINDING_MISMATCH),
        ("/aud", json!("other"), BINDING_MISMATCH),
        ("/iss", json!("https://elsewhere.example"), BINDING_MISMATCH),
        ("/exp", json!(300), BINDING_MISMATCH),
        ("/log/epoch", json!(1), BINDING_LOG_MISMATCH),
        ("/binding", json!(2), BINDING_UNSUPPORTED),
        (
            "/dependencies/0/path",
            json!(["grant-child", "grant-parent", "grant-child"]),
            BINDING_ANCESTRY,
        ),
        (
            "/dependencies/0/path",
            json!(["grant-parent"]),
            BINDING_ANCESTRY,
        ),
        (
            "/dependencies/1",
            json!({"grant": "grant-child", "path": ["grant-child"]}),
            BINDING_ANCESTRY,
        ),
        (
            "/dependencies/1",
            json!({"grant": "grant-extra", "path": ["grant-extra"]}),
            BINDING_ANCESTRY,
        ),
    ] {
        assert_eq!(
            refusal(&changed(pointer, value)?, &token, &pass)?,
            expected,
            "{pointer}"
        );
    }
    // A pass-typed binding is never a binding, and a binding is never a pass.
    let as_pass = signed("JWT", &base);
    assert_eq!(refusal(&as_pass, &token, &pass)?, "contract_refused");
    let as_binding = signed(BINDING_TYPE, &pass_claims("person-1"));
    assert!(VerifiedPass::verify(&as_binding, &keys()?, ISSUER, "sample", 150).is_err());
    assert_eq!(
        required(None).map_err(|error| error.name().to_owned()),
        Err("grant_binding_required".to_owned())
    );
    Ok(())
}
