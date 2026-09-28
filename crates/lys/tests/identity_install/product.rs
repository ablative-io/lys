//! DIRECTORY-047 R3 on a real install: a fixture product configured from
//! what the install wrote. The product is registered in the service
//! configuration install wrote, as a product registration adds it, and the
//! install is run again, which keeps it and restarts the service on it. The
//! product then discovers Lys at Lys's origin, sends a signed-in person to
//! Lys, is handed a code at its own registered address, exchanges it with
//! PKCE and its secret, and reads the person through the access token. Every
//! answer is kept for the scan, and every address the browser is sent to is
//! Lys's or the product's.

use std::path::Path;
use std::process::Command;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::identity_support::fixtures::{TestResult, succeeded};
use super::{EMAIL, Heard, ORIGIN, PASSWORD, ask, hex, send};

/// The fixture product's client id.
const PRODUCT: &str = "fixture-product";
/// The fixture product's client secret; only its SHA-256 is registered.
const SECRET: &str = "fixture-product-secret";
/// The one address the fixture product registered for its codes.
pub const CALLBACK: &str = "http://product.example.test/auth/callback";
/// The fixture product's PKCE verifier.
const VERIFIER: &str = "a-fixture-product-verifier-of-enough-length-0123456789";

/// The install the product is registered with.
pub struct Installed<'a> {
    /// The install root.
    pub root: &'a Path,
    /// The `lys` binary.
    pub lys: &'a Path,
    /// The screens package the install was given.
    pub package: &'a Path,
    /// The `PATH` with no browser on it.
    pub path: &'a str,
}

/// The value `name` has in the query of `address`.
fn query_value(address: &str, name: &str) -> Option<String> {
    let (_, query) = address.split_once('?')?;
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.to_owned())
}

/// Registers the fixture product in the service configuration the install
/// wrote and runs the install again, which keeps the product.
fn register(installed: &Installed<'_>, read: &mut Heard) -> TestResult {
    let file = installed.root.join("identity.json");
    let mut config: Value = serde_json::from_slice(&std::fs::read(&file)?)?;
    assert_eq!(config["provider"]["clients"], json!([]), "{config}");
    let key_file = config["provider"]["key_file"]
        .as_str()
        .ok_or("the install names the provider's signing key")?;
    assert!(
        Path::new(key_file).starts_with(installed.root),
        "{key_file}"
    );
    config["provider"]["clients"] = json!([{
        "client_id": PRODUCT,
        "secret_sha256": hex(&Sha256::digest(SECRET.as_bytes())),
        "redirect_uris": [CALLBACK],
    }]);
    // Written compactly, as a registration writes it; the install writes its
    // own layout back, so the configuration changes and the service restarts.
    std::fs::write(&file, serde_json::to_vec(&config)?)?;
    let again = Command::new(installed.lys)
        .args(["identity", "install", "--root"])
        .arg(installed.root)
        .arg("--surface")
        .arg(installed.package)
        .env("PATH", installed.path)
        .output()?;
    succeeded(&again, "lys identity install, run again")?;
    read.install(&again);
    let kept: Value = serde_json::from_slice(&std::fs::read(&file)?)?;
    assert_eq!(kept["provider"]["clients"], config["provider"]["clients"]);
    Ok(())
}

/// A fixture product signs a person in through Lys, configured from what the
/// install wrote.
pub fn signs_in_through_lys(installed: &Installed<'_>, read: &mut Heard) -> TestResult {
    register(installed, read)?;
    let credentials = json!({ "email": EMAIL, "password": PASSWORD }).to_string();
    let signed_in = ask("POST", "/api/sign-in", None, Some(&credentials))?;
    assert_eq!(signed_in.status, 200, "{}", signed_in.body);
    read.answer("a sign-in after the restart", &signed_in);
    let cookie = signed_in.cookie.ok_or("the sign-in began a session")?;

    let found = ask("GET", "/.well-known/openid-configuration", None, None)?;
    assert_eq!(found.status, 200, "{}", found.body);
    read.answer("the product's discovery", &found);
    let discovery: Value = serde_json::from_str(&found.body)?;
    assert_eq!(discovery["issuer"], ORIGIN);
    for endpoint in [
        "authorization_endpoint",
        "token_endpoint",
        "userinfo_endpoint",
        "jwks_uri",
    ] {
        let address = discovery[endpoint].as_str().ok_or(endpoint)?;
        assert!(address.starts_with(ORIGIN), "{endpoint} is {address}");
    }

    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let authorize = format!(
        "/oauth/authorize?client_id={PRODUCT}&redirect_uri=http%3A%2F%2Fproduct.example.test%2Fauth%2Fcallback&response_type=code&scope=openid&state=product-state&nonce=product-nonce&code_challenge={challenge}&code_challenge_method=S256"
    );
    let unsigned = ask("GET", &authorize, None, None)?;
    assert_eq!(unsigned.status, 303, "{}", unsigned.body);
    read.answer("a product's person not yet signed in", &unsigned);
    let handed = ask("GET", &authorize, Some(&cookie), None)?;
    assert_eq!(handed.status, 303, "{}", handed.body);
    read.handed_back("a code handed to the product", &handed);
    let back = handed.location.ok_or("the code is handed back")?;
    assert_eq!(
        query_value(&back, "state").as_deref(),
        Some("product-state")
    );
    let code = query_value(&back, "code").ok_or("a code")?;

    let form = format!(
        "grant_type=authorization_code&code={code}&redirect_uri=http%3A%2F%2Fproduct.example.test%2Fauth%2Fcallback&code_verifier={VERIFIER}"
    );
    let basic = STANDARD.encode(format!("{PRODUCT}:{SECRET}"));
    let exchanged = send(
        "POST",
        "/oauth/token",
        &[format!("Authorization: Basic {basic}")],
        Some(("application/x-www-form-urlencoded", &form)),
    )?;
    assert_eq!(exchanged.status, 200, "{}", exchanged.body);
    read.answer("the product's tokens", &exchanged);
    let tokens: Value = serde_json::from_str(&exchanged.body)?;
    let id_token = tokens["id_token"].as_str().ok_or("an ID token")?;
    let payload = id_token.split('.').nth(1).ok_or("an ID token's claims")?;
    let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload)?)?;
    assert_eq!(claims["iss"], ORIGIN);
    assert_eq!(claims["aud"], PRODUCT);
    assert_eq!(claims["nonce"], "product-nonce");
    let access = tokens["access_token"].as_str().ok_or("an access token")?;

    let info = send(
        "GET",
        "/oauth/userinfo",
        &[format!("Authorization: Bearer {access}")],
        None,
    )?;
    assert_eq!(info.status, 200, "{}", info.body);
    read.answer("the product's user information", &info);
    let person: Value = serde_json::from_str(&info.body)?;
    assert_eq!(person["sub"], claims["sub"]);
    assert_ne!(
        claims["sub"], EMAIL,
        "the subject is the person's directory id"
    );
    let keys = ask("GET", "/oauth/jwks", None, None)?;
    assert_eq!(keys.status, 200, "{}", keys.body);
    read.answer("the product's keys", &keys);
    Ok(())
}
