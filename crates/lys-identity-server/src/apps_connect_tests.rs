//! The connect code is single use and expires; a return outside the
//! manifest's origins is refused.

use super::{CODE_LIFETIME_MS, Exchanged, keep_code, return_admitted, take};
use crate::apps_binding::new_secret;

fn answer() -> Exchanged {
    Exchanged {
        app: "connect_fixture".to_owned(),
        issuer: "https://lys.example.test".to_owned(),
        client_id: "connect_fixture".to_owned(),
        client_secret: "lys-client.connect_fixture.x".to_owned(),
        workspace_credential: "lys-app.connect_fixture.y".to_owned(),
    }
}

#[test]
fn apps_connect_code_is_single_use() -> Result<(), Box<dyn std::error::Error>> {
    let (code, digest) = new_secret()?;
    keep_code(digest, 1_000, answer())?;
    let first = take(&code, 2_000)?.map_err(ToOwned::to_owned)?;
    assert_eq!(first.client_id, "connect_fixture");
    assert_eq!(take(&code, 3_000)?.err(), Some("connect_code_used"));
    assert_eq!(
        take("not-a-code", 3_000)?.err(),
        Some("connect_code_unknown")
    );
    Ok(())
}

#[test]
fn apps_connect_code_expires_after_ten_minutes() -> Result<(), Box<dyn std::error::Error>> {
    let (code, digest) = new_secret()?;
    keep_code(digest, 5_000, answer())?;
    assert_eq!(
        take(&code, 5_001 + CODE_LIFETIME_MS)?.err(),
        Some("connect_code_expired")
    );
    Ok(())
}

#[test]
fn apps_connect_return_outside_the_manifest_origin_is_refused() {
    let redirects = vec!["https://cambium.example.test/identity/oidc/callback".to_owned()];
    assert!(
        return_admitted(
            "https://cambium.example.test/identity/lys/connected",
            &redirects
        )
        .is_ok()
    );
    for outside in [
        "https://evil.example.test/identity/lys/connected",
        "http://cambium.example.test/identity/lys/connected",
        "https://cambium.example.test:8443/x",
        "not an address",
    ] {
        assert!(return_admitted(outside, &redirects).is_err(), "{outside}");
    }
}
