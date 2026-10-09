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
//! The administrator issues the product a client credential (DIRECTORY-081),
//! answered once, and the product exchanges each code with it, in its Basic
//! header before the restart and in its form after, the real broker
//! confirming it: the ID token verifies against the key Lys publishes, and
//! userinfo answers the same subject. A revoked credential is then refused
//! `credential_refused`, and once the app is retired its other credential is
//! refused `app_retired`.
//!
//! ACCESS-002 R2: build.json records the pass lifetime's stated default
//! after the first install, and a lifetime chosen in the deployment
//! configuration after the install runs again.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use lys_core::Ed25519Identity;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::identity_support::fixtures::{TestResult, succeeded};
use super::{EMAIL, Heard, PASSWORD, Seen, ask, operation, send};

/// The fixture product's client id, an app id.
const PRODUCT: &str = "fixture_product";
/// The one address the fixture product registered for its codes: https, as
/// the service requires of any address that is not this machine's.
pub const CALLBACK: &str = "https://product.example.test/auth/callback";
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
    /// The login the install is run with (`login`).
    pub login: &'a [(std::ffi::OsString, std::ffi::OsString)],
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
    assert!(
        answer.get("client").is_none(),
        "no secret is issued: {answer}"
    );
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

/// The pass lifetime `install/build.json` records (ACCESS-002 R2).
fn recorded_pass_lifetime(installed: &Installed<'_>) -> TestResult<u64> {
    let build: Value =
        serde_json::from_slice(&std::fs::read(installed.root.join("install/build.json"))?)?;
    Ok(build["settings"]["identity.pass_lifetime"]
        .as_u64()
        .ok_or("build.json records no pass lifetime")?)
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
        .env_clear()
        .envs(installed.login.iter().map(|(name, value)| (name, value)))
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
        "/oauth/authorize?client_id={PRODUCT}&redirect_uri=https%3A%2F%2Fproduct.example.test%2Fauth%2Fcallback&response_type=code&scope=openid&state=product-state&nonce=product-nonce&code_challenge={challenge}&code_challenge_method=S256"
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

/// Posts `body` to the app route `rest` of the fixture product as the
/// administrator, answering what was seen.
fn admin_act(
    installed: &Installed<'_>,
    cookie: &str,
    rest: &str,
    body: &Value,
) -> TestResult<Seen> {
    ask(
        installed.service_port,
        "POST",
        &format!("/api/apps/{PRODUCT}{rest}"),
        Some(cookie),
        Some(&body.to_string()),
    )
}

/// Issues the fixture product a client credential, answering its id and its
/// value, given this once.
fn issue(
    installed: &Installed<'_>,
    read: &mut Heard,
    cookie: &str,
) -> TestResult<(String, String)> {
    let issued = admin_act(
        installed,
        cookie,
        "/credentials/issue",
        &json!({ "operation": operation() }),
    )?;
    assert_eq!(issued.status, 200, "{}", issued.body);
    read.answer("the product's client credential, issued", &issued);
    let answer: Value = serde_json::from_str(&issued.body)?;
    let value = answer["credential"]
        .as_str()
        .ok_or("an issue answers its value")?;
    assert!(
        value.starts_with(&format!("lys-client.{PRODUCT}.")),
        "{value}"
    );
    let id = answer["credential_id"].as_str().ok_or("an id")?;
    Ok((id.to_owned(), value.to_owned()))
}

/// The product exchanges `code` with `credential`, in its Basic header or in
/// its form, answering what was seen.
fn exchange(
    installed: &Installed<'_>,
    code: &str,
    credential: &str,
    basic: bool,
) -> TestResult<Seen> {
    let mut form = format!(
        "grant_type=authorization_code&code={code}&redirect_uri=https%3A%2F%2Fproduct.example.test%2Fauth%2Fcallback&code_verifier={VERIFIER}"
    );
    let mut headers = Vec::new();
    if basic {
        let pair = STANDARD.encode(format!("{PRODUCT}:{credential}"));
        headers.push(format!("Authorization: Basic {pair}"));
    } else {
        write!(form, "&client_id={PRODUCT}&client_secret={credential}")?;
    }
    send(
        installed.service_port,
        "POST",
        "/oauth/token",
        &headers,
        Some(("application/x-www-form-urlencoded", &form)),
    )
}

/// The product signs the person in with `code`: the exchange answers an ID
/// token that verifies against the key Lys publishes, names Lys's origin and
/// the product, and userinfo answers the same subject for its access token.
fn signed_in(
    installed: &Installed<'_>,
    read: &mut Heard,
    code: &str,
    credential: &str,
    basic: bool,
    when: &str,
) -> TestResult {
    let origin = format!("http://localhost:{}", installed.service_port);
    let token = exchange(installed, code, credential, basic)?;
    assert_eq!(token.status, 200, "{}", token.body);
    let answer: Value = serde_json::from_str(&token.body)?;
    assert!(!token.body.contains(credential));
    read.answer(&format!("the product's token, {when}"), &token);
    let id_token = answer["id_token"].as_str().ok_or("an ID token")?;
    let parts: Vec<&str> = id_token.split('.').collect();
    let [header, claims, signature] = parts.as_slice() else {
        return Err(format!("an ID token has three parts: {id_token}").into());
    };
    let header: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(header)?)?;
    let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims)?)?;
    let keys = ask(installed.service_port, "GET", "/oauth/jwks", None, None)?;
    assert_eq!(keys.status, 200, "{}", keys.body);
    read.answer(&format!("Lys's keys, {when}"), &keys);
    let keys: Value = serde_json::from_str(&keys.body)?;
    let key = keys["keys"]
        .as_array()
        .and_then(|keys| keys.iter().find(|key| key["kid"] == header["kid"]))
        .ok_or("the token's key is published")?;
    assert_eq!(header["alg"], "EdDSA");
    let public: [u8; 32] = URL_SAFE_NO_PAD
        .decode(key["x"].as_str().ok_or("a public key")?)?
        .try_into()
        .map_err(|_unsized| "an Ed25519 key is 32 bytes")?;
    let signed = format!("{}.{}", parts[0], parts[1]);
    Ed25519Identity::verify(
        &public,
        signed.as_bytes(),
        &URL_SAFE_NO_PAD.decode(signature)?,
    )?;
    assert_eq!(claims["iss"], origin);
    assert_eq!(claims["aud"], PRODUCT);
    assert_eq!(claims["nonce"], "product-nonce");
    let access = answer["access_token"].as_str().ok_or("an access token")?;
    let info = send(
        installed.service_port,
        "GET",
        "/oauth/userinfo",
        &[format!("Authorization: Bearer {access}")],
        None,
    )?;
    assert_eq!(info.status, 200, "{}", info.body);
    read.answer(&format!("the product's userinfo, {when}"), &info);
    let info: Value = serde_json::from_str(&info.body)?;
    assert_eq!(info["sub"], claims["sub"]);
    Ok(())
}

/// The product's exchange of `code` with `credential` is refused `refusal`
/// with `status`.
fn refused(
    installed: &Installed<'_>,
    read: &mut Heard,
    code: &str,
    credential: &str,
    (status, refusal): (u16, &str),
) -> TestResult {
    let token = exchange(installed, code, credential, true)?;
    read.answer(&format!("the product's token, refused {refusal}"), &token);
    assert_eq!(token.status, status, "{}", token.body);
    let answer: Value = serde_json::from_str(&token.body)?;
    assert_eq!(answer["refusal"], refusal, "{answer}");
    assert!(!token.body.contains(credential));
    Ok(())
}

/// A fixture product registered and approved on a real install signs a
/// person in with its client credential, before and after the install runs
/// again; revoked, the credential is refused, and retired, the app is.
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
    let (credential_id, credential) = issue(installed, read, &cookie)?;
    let first = handed_a_code(installed, read, &cookie, "at approval")?;
    signed_in(installed, read, &first, &credential, true, "at approval")?;

    assert_eq!(
        recorded_pass_lifetime(installed)?,
        600,
        "build.json records the pass lifetime's stated default"
    );
    // A chosen pass lifetime (ACCESS-002 R2): named in the deployment
    // configuration, it is what the run again records.
    let deployment = installed.root.join("deployment.toml");
    let mut chosen = std::fs::read_to_string(&deployment)?;
    chosen.push_str("\n[identity]\npass_lifetime = 900\n");
    std::fs::write(&deployment, chosen)?;
    install_again(installed, read)?;
    assert_eq!(
        recorded_pass_lifetime(installed)?,
        900,
        "build.json records the chosen pass lifetime"
    );
    let config: Value =
        serde_json::from_slice(&std::fs::read(installed.root.join("identity.json"))?)?;
    assert_eq!(config["provider"]["pass_seconds"], 900, "{config}");
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
    assert_eq!(kept["client_credentials"][0]["live"], true, "{kept}");
    let second = handed_a_code(installed, read, &cookie, "after the restart")?;
    assert_ne!(first, second, "each authorize hands a fresh code");
    signed_in(
        installed,
        read,
        &second,
        &credential,
        false,
        "after the restart",
    )?;

    let third = handed_a_code(installed, read, &cookie, "before the revocation")?;
    let revoked = admin_act(
        installed,
        &cookie,
        &format!("/credentials/{credential_id}/revoke"),
        &json!({ "operation": operation(), "reason": "rotated" }),
    )?;
    assert_eq!(revoked.status, 200, "{}", revoked.body);
    read.answer("the product's credential, revoked", &revoked);
    let revoked: Value = serde_json::from_str(&revoked.body)?;
    assert_eq!(revoked["client_credentials"][0]["live"], false);
    assert_eq!(revoked["client_credentials"][0]["ended_at_broker"], true);
    refused(
        installed,
        read,
        &third,
        &credential,
        (401, "credential_refused"),
    )?;

    let (other_id, other) = issue(installed, read, &cookie)?;
    let fourth = handed_a_code(installed, read, &cookie, "before the retirement")?;
    let retired = admin_act(
        installed,
        &cookie,
        "/retire",
        &json!({ "operation": operation(), "reason": "done" }),
    )?;
    assert_eq!(retired.status, 200, "{}", retired.body);
    read.answer("the product, retired", &retired);
    let retired: Value = serde_json::from_str(&retired.body)?;
    let ended = &retired["client_credentials"][1];
    assert_eq!(ended["credential_id"], other_id.as_str(), "{retired}");
    assert_eq!(ended["live"], false);
    assert_eq!(ended["ended_by_retirement"], true);
    refused(installed, read, &fourth, &other, (403, "app_retired"))?;
    Ok(())
}
