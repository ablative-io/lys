//! DIRECTORY-047 R1 on a real install: the install is given a
//! `[password_policy]` of Lys's own that is not the sign-in service's
//! default ([`LYS_POLICY`]). The setup screen is answered that policy before
//! any password is sent, and the sign-in service enforces it: a password its
//! own default takes and Lys's policy does not is refused when the
//! directory service's key sets it, and one Lys's policy takes is set and
//! signs in through Lys. Nothing reads the sign-in service's policy back.
//!
//! The directory service's key is its own and narrower than the install's:
//! it is refused writing the password policy and rotating the sign-in
//! service's signing keys.

use std::path::Path;

use serde_json::{Value, json};

use super::identity_support::fixtures::TestResult;
use super::identity_support::server::request;
use super::{EMAIL, Heard, ask};

/// Lys's policy for this install, appended to its deployment configuration:
/// longer and with more digits than the sign-in service's default of 14
/// characters and one digit.
pub const LYS_POLICY: &str = "\n[password_policy]\nlength_min = 20\nlength_max = 64\ndigits = 3\n";

/// Taken by the sign-in service's default policy (14 or more characters, a
/// lower-case and an upper-case letter and a digit) and refused by Lys's:
/// 17 characters and two digits.
const DEFAULT_ONLY: &str = "Babbage-Engine-18";

/// Taken by Lys's policy: 22 characters and four digits.
const LYS_TAKES: &str = "Difference-Engine-1822";

/// The setup screen's answer carries Lys's policy, so the screen shows it
/// before a password is sent.
pub fn setup_shows_lys_policy(open: &Value) -> TestResult {
    let policy = &open["policy"];
    assert_eq!(policy["length_min"], 20, "{open}");
    assert_eq!(policy["length_max"], 64, "{open}");
    assert_eq!(policy["digits"], 3, "{open}");
    let words = policy["words"].as_str().ok_or("the policy has no words")?;
    assert!(
        words.contains("At least 20 and at most 64 characters"),
        "{words}"
    );
    Ok(())
}

/// The account `id`'s update as the sign-in service takes it, with
/// `password`, every other field as it was read.
fn update_with(user: &Value, password: &str) -> Value {
    let field = |name: &str| user.get(name).cloned().unwrap_or(Value::Null);
    json!({
        "email": field("email"),
        "given_name": field("given_name"),
        "family_name": field("family_name"),
        "language": field("language"),
        "password": password,
        "roles": user.get("roles").cloned().unwrap_or_else(|| json!([])),
        "groups": field("groups"),
        "enabled": true,
        "email_verified": field("email_verified"),
        "user_expires": field("user_expires"),
        "user_values": field("user_values"),
    })
}

/// The sign-in service at `rauthy_port` enforces Lys's policy on the
/// administrator's account, set with the key the directory service holds,
/// and that key may neither write the policy nor rotate signing keys.
pub fn issuer_enforces_lys_policy(
    root: &Path,
    rauthy_port: u16,
    service_port: u16,
    read: &mut Heard,
) -> TestResult {
    let held = std::fs::read_to_string(root.join("state").join("sign-in-providers-api-key"))?;
    assert!(
        held.starts_with("lys_directory$"),
        "the directory service holds its own key, never the install's"
    );
    let key = format!("API-Key {}", held.trim());
    let authorised = [("Authorization", key.as_str())];
    let address = format!("127.0.0.1:{rauthy_port}");

    let (status, body) = request(
        &address,
        "GET",
        &format!("/auth/v1/users/email/{EMAIL}"),
        &authorised,
        None,
    )?;
    assert_eq!(status, 200, "{body}");
    let user: Value = serde_json::from_str(&body)?;
    let id = user["id"].as_str().ok_or("the account has no id")?;
    let path = format!("/auth/v1/users/{id}");

    let refused = update_with(&user, DEFAULT_ONLY).to_string();
    let (status, body) = request(&address, "PUT", &path, &authorised, Some(&refused))?;
    assert_eq!(
        status, 400,
        "the sign-in service refuses a password only its own default takes: {body}"
    );
    assert!(body.contains("20"), "refused for Lys's length: {body}");

    let taken = update_with(&user, LYS_TAKES).to_string();
    let (status, body) = request(&address, "PUT", &path, &authorised, Some(&taken))?;
    assert_eq!(status, 200, "a password Lys's policy takes is set: {body}");
    let credentials = json!({ "email": EMAIL, "password": LYS_TAKES }).to_string();
    let signed_in = ask(
        service_port,
        "POST",
        "/api/sign-in",
        None,
        Some(&credentials),
    )?;
    assert_eq!(signed_in.status, 200, "{}", signed_in.body);
    read.answer("a sign-in with a password Lys's policy took", &signed_in);

    let policy = json!({
        "length_min": 8, "length_max": 128, "include_lower_case": null,
        "include_upper_case": null, "include_digits": null, "include_special": null,
        "valid_days": null, "not_recently_used": null,
    })
    .to_string();
    let (status, body) = request(
        &address,
        "PUT",
        "/auth/v1/password_policy",
        &authorised,
        Some(&policy),
    )?;
    assert_eq!(
        status, 403,
        "the service's key cannot write the policy: {body}"
    );
    let (status, body) = request(
        &address,
        "POST",
        "/auth/v1/oidc/rotate_jwk",
        &authorised,
        None,
    )?;
    assert_eq!(status, 403, "the service's key cannot rotate keys: {body}");
    Ok(())
}
