//! DIRECTORY-047 R3 and DIRECTORY-079 R1 on a real install: a fixture
//! product is registered through the service's own routes by the signed-in
//! administrator and approved with its one return address, and the real
//! broker confirms custody of its credentials, showing its secret to nobody.
//! The product then discovers Lys at Lys's origin, sends a signed-in person
//! to Lys and is handed a code at its own registered address. The install is
//! run again, which restarts the service, and the app is still approved and
//! still handed a code. Every answer is kept for the scan, and every address
//! the browser is sent to is Lys's or the product's.
//!
//! The token, userinfo and keys steps are not here until BOX 16: no answer
//! hands a product its client secret, and the broker does not yet present
//! the app's client authentication for it. Until then the in-process tests
//! in lys-identity-server's tests/provider.rs prove them through the custody
//! fixture, among them
//! a_product_signs_in_through_lys_and_verifies_the_token_against_lys_keys
//! and an_app_is_a_client_from_its_approval_to_its_retirement_with_no_restart.

use std::path::Path;
use std::process::Command;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::identity_support::fixtures::{TestResult, succeeded};
use super::{EMAIL, Heard, PASSWORD, ask, operation};

/// The fixture product's client id, an app id.
const PRODUCT: &str = "fixture_product";
/// The one address the fixture product registered for its codes.
pub const CALLBACK: &str = "http://product.example.test/auth/callback";
/// The fixture product's PKCE verifier.
const VERIFIER: &str = "a-fixture-product-verifier-of-enough-length-0123456789";

/// The install the product is registered with.
pub struct Installed<'a> {
    /// The install root.
    pub root: &'a Path,
    pub service_port: u16,
    pub broker_port: u16,
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

/// Whether `text` holds a run of 64 hex digits, the shape of the client
/// secret the broker makes.
fn holds_a_secret(text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_hexdigit())
        .any(|run| run.len() >= 64)
}

/// Signs the administrator in, answering their session cookie.
fn sign_in(installed: &Installed<'_>, read: &mut Heard, what: &str) -> TestResult<String> {
    let credentials = json!({ "email": EMAIL, "password": PASSWORD }).to_string();
    let signed_in = ask(
        installed.service_port,
        "POST",
        "/api/sign-in",
        None,
        Some(&credentials),
    )?;
    assert_eq!(signed_in.status, 200, "{}", signed_in.body);
    read.answer(what, &signed_in);
    Ok(signed_in.cookie.ok_or("the sign-in began a session")?)
}

/// Registers the fixture product through `POST /api/apps` (an install serves
/// the service's routes under `/api`, beside its screens) as the signed-in
/// administrator and approves it with its one return address, as the Apps
/// screen does. The approval confirms broker custody: its credentials name
/// the app, no client is issued in the answer, and no answer holds a secret.
fn register(installed: &Installed<'_>, read: &mut Heard, cookie: &str) -> TestResult<Value> {
    let config: Value =
        serde_json::from_slice(&std::fs::read(installed.root.join("identity.json"))?)?;
    assert!(config["provider"].get("clients").is_none(), "{config}");
    let key_file = config["provider"]["key_file"]
        .as_str()
        .ok_or("the install names the provider's signing key")?;
    assert!(
        Path::new(key_file).starts_with(installed.root),
        "{key_file}"
    );
    let registration = json!({
        "operation": operation(),
        "id": PRODUCT,
        "name": "The fixture product",
        "redirects": [CALLBACK],
        "schema": {"kinds": {
            format!("{PRODUCT}.workspace"): {
                "actions": ["read"],
                "relations": {"member": ["read"]},
                "parents": [],
            },
        }},
    })
    .to_string();
    let registered = ask(
        installed.service_port,
        "POST",
        "/api/apps",
        Some(cookie),
        Some(&registration),
    )?;
    assert_eq!(registered.status, 200, "{}", registered.body);
    read.answer("the product's registration", &registered);
    let approval =
        json!({ "operation": operation(), "redirects": [CALLBACK], "profile": false }).to_string();
    let approved = ask(
        installed.service_port,
        "POST",
        &format!("/api/apps/{PRODUCT}/approve"),
        Some(cookie),
        Some(&approval),
    )?;
    assert_eq!(approved.status, 200, "{}", approved.body);
    read.answer("the product's approval", &approved);
    assert!(!holds_a_secret(&approved.body), "{}", approved.body);
    let answer: Value = serde_json::from_str(&approved.body)?;
    assert_eq!(answer["client"], Value::Null, "no secret is issued");
    assert_eq!(answer["credentials"]["app"], PRODUCT);
    for reference in ["client_secret_ref", "api_credential_ref"] {
        let named = answer["credentials"][reference].as_str().ok_or(reference)?;
        assert!(
            named.starts_with("lys-app-") && named.contains(PRODUCT),
            "{reference} is {named}"
        );
    }
    assert_eq!(answer["app"]["state"], "approved", "{answer}");
    assert_eq!(answer["app"]["client_id"], PRODUCT);
    assert_eq!(answer["app"]["sign_in"]["redirects"], json!([CALLBACK]));
    Ok(answer["app"]["sign_in"].clone())
}

/// Runs the install again, which keeps what it wrote and restarts the
/// service on it.
fn install_again(installed: &Installed<'_>, read: &mut Heard) -> TestResult {
    let again = Command::new(installed.lys)
        .args(["identity", "install", "--root"])
        .arg(installed.root)
        .arg("--service-port")
        .arg(installed.service_port.to_string())
        .arg("--broker-port")
        .arg(installed.broker_port.to_string())
        .arg("--surface")
        .arg(installed.package)
        .env("PATH", installed.path)
        .output()?;
    succeeded(&again, "lys identity install, run again")?;
    read.install(&again);
    Ok(())
}

/// The product sends a person to Lys's authorize: not signed in, they are
/// sent on within Lys; signed in, the product is handed a code at its own
/// address with its state. Answers the code.
fn handed_a_code(
    installed: &Installed<'_>,
    read: &mut Heard,
    cookie: &str,
    when: &str,
) -> TestResult<String> {
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let authorize = format!(
        "/oauth/authorize?client_id={PRODUCT}&redirect_uri=http%3A%2F%2Fproduct.example.test%2Fauth%2Fcallback&response_type=code&scope=openid&state=product-state&nonce=product-nonce&code_challenge={challenge}&code_challenge_method=S256"
    );
    let unsigned = ask(installed.service_port, "GET", &authorize, None, None)?;
    assert_eq!(unsigned.status, 303, "{}", unsigned.body);
    read.answer(
        &format!("a product's person not yet signed in, {when}"),
        &unsigned,
    );
    let handed = ask(
        installed.service_port,
        "GET",
        &authorize,
        Some(cookie),
        None,
    )?;
    assert_eq!(handed.status, 303, "{}", handed.body);
    read.handed_back(&format!("a code handed to the product, {when}"), &handed);
    let back = handed.location.ok_or("the code is handed back")?;
    assert!(back.starts_with(CALLBACK), "{back}");
    assert_eq!(
        query_value(&back, "state").as_deref(),
        Some("product-state")
    );
    Ok(query_value(&back, "code").ok_or("a code")?)
}

/// A fixture product registered and approved on a real install is handed a
/// code for a signed-in person, before and after the install runs again.
pub fn signs_in_through_lys(installed: &Installed<'_>, read: &mut Heard) -> TestResult {
    let origin = format!("http://localhost:{}", installed.service_port);
    let cookie = sign_in(installed, read, "the administrator's sign-in")?;
    let sign_in_settings = register(installed, read, &cookie)?;

    let found = ask(
        installed.service_port,
        "GET",
        "/.well-known/openid-configuration",
        None,
        None,
    )?;
    assert_eq!(found.status, 200, "{}", found.body);
    read.answer("the product's discovery", &found);
    let discovery: Value = serde_json::from_str(&found.body)?;
    assert_eq!(discovery["issuer"], origin);
    for endpoint in [
        "authorization_endpoint",
        "token_endpoint",
        "userinfo_endpoint",
        "jwks_uri",
    ] {
        let address = discovery[endpoint].as_str().ok_or(endpoint)?;
        assert!(address.starts_with(&origin), "{endpoint} is {address}");
    }
    let first = handed_a_code(installed, read, &cookie, "at approval")?;

    install_again(installed, read)?;
    let cookie = sign_in(installed, read, "a sign-in after the restart")?;
    let kept = ask(
        installed.service_port,
        "GET",
        &format!("/api/apps/{PRODUCT}"),
        Some(&cookie),
        None,
    )?;
    assert_eq!(kept.status, 200, "{}", kept.body);
    read.answer("the product after the restart", &kept);
    assert!(!holds_a_secret(&kept.body), "{}", kept.body);
    let kept: Value = serde_json::from_str(&kept.body)?;
    assert_eq!(kept["state"], "approved", "{kept}");
    assert_eq!(kept["sign_in"], sign_in_settings);
    let second = handed_a_code(installed, read, &cookie, "after the restart")?;
    assert_ne!(first, second, "each authorize hands a fresh code");
    Ok(())
}
