#![cfg(test)]
//! The provider's query encoding, secret comparison, client credentials and
//! token signature, each checked against a value written apart from it.

use std::error::Error;

use axum::http::{HeaderMap, HeaderValue, header};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lys_core::Ed25519Identity;
use serde_json::json;
use sha2::Digest;

use super::endpoints::{Exchange, presented};
use super::{Access, Grant, OpenIdProvider, ProviderSettings, encoded, held, same};
use crate::error::ServerError;

#[test]
fn a_query_value_is_percent_encoded_except_its_unreserved_characters() {
    assert_eq!(encoded("aZ09-._~"), "aZ09-._~");
    assert_eq!(
        encoded("/oauth/authorize?a=b&c"),
        "%2Foauth%2Fauthorize%3Fa%3Db%26c"
    );
    assert_eq!(encoded("é"), "%C3%A9");
}

/// DIRECTORY-079 R1, acceptance 9: the provider's settings have no clients
/// member, so a configuration naming `provider.clients` is refused as it is
/// read, by the existing unknown-field rule, naming the key.
#[test]
fn a_configuration_naming_provider_clients_is_refused_naming_the_key() {
    let read = serde_json::from_value::<ProviderSettings>(json!({
        "key_file": "/srv/lys/state/provider.key",
        "clients": [],
    }));
    let refusal = match read {
        Ok(_) => panic!("a configuration naming provider.clients was read"),
        Err(error) => error.to_string(),
    };
    assert!(refusal.contains("unknown field `clients`"), "{refusal}");
    let without = serde_json::from_value::<ProviderSettings>(json!({
        "key_file": "/srv/lys/state/provider.key",
    }));
    assert!(without.is_ok(), "{without:?}");
}

#[test]
fn text_is_the_same_only_when_every_byte_is() {
    assert!(same("abc", "abc"));
    assert!(!same("abc", "abd"));
    assert!(!same("abc", "abcd"));
    assert!(same("", ""));
}

fn exchange(client_id: Option<&str>, client_secret: Option<&str>) -> Exchange {
    Exchange {
        grant_type: "authorization_code".to_owned(),
        code: "code".to_owned(),
        redirect_uri: "https://product.example.test/callback".to_owned(),
        code_verifier: "verifier".to_owned(),
        client_id: client_id.map(str::to_owned),
        client_secret: client_secret.map(str::to_owned),
    }
}

#[test]
fn client_credentials_are_read_from_basic_first_then_the_form() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Basic cHJvZHVjdDpzM2NyZXQ="),
    );
    assert_eq!(
        presented(&headers, &exchange(Some("other"), Some("x"))),
        Some(("product".to_owned(), "s3cret".to_owned()))
    );
    assert_eq!(
        presented(
            &HeaderMap::new(),
            &exchange(Some("product"), Some("s3cret"))
        ),
        Some(("product".to_owned(), "s3cret".to_owned()))
    );
    assert_eq!(
        presented(&HeaderMap::new(), &exchange(Some("product"), None)),
        None
    );
}

#[test]
fn an_id_token_verifies_under_the_key_its_jwks_names() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let key_file = dir.path().join("provider.key");
    std::fs::write(&key_file, [7u8; 32])?;
    let settings = ProviderSettings {
        key_file: key_file.clone(),
        code_seconds: 60,
    };
    let provider = OpenIdProvider::open(&settings, "http://localhost:8490".to_owned())?;
    let token = provider.signed(&json!({ "iss": "http://localhost:8490", "sub": "person-1" }));
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3);
    let header: serde_json::Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0])?)?;
    assert_eq!(header["alg"], "EdDSA");
    let keys = provider.keys();
    assert_eq!(header["kid"], keys["keys"][0]["kid"]);
    let x = URL_SAFE_NO_PAD.decode(keys["keys"][0]["x"].as_str().ok_or("x")?)?;
    let public: [u8; 32] = x.as_slice().try_into()?;
    assert_eq!(public, Ed25519Identity::load(&key_file)?.public_key_bytes());
    let signature = URL_SAFE_NO_PAD.decode(parts[2])?;
    let input = format!("{}.{}", parts[0], parts[1]);
    Ed25519Identity::verify(&public, input.as_bytes(), &signature)?;
    let mut forged = input.into_bytes();
    forged[0] ^= 1;
    assert!(Ed25519Identity::verify(&public, &forged, &signature).is_err());
    assert_eq!(provider.discovery()["issuer"], "http://localhost:8490");
    Ok(())
}

/// DIRECTORY-079 R3: the name is kept nowhere. The access table holds the
/// token's app and whether it asked for the profile scope, never the name,
/// and a table carrying one is refused.
#[test]
fn the_access_table_keeps_whether_the_name_was_asked_for_and_never_the_name()
-> Result<(), Box<dyn Error>> {
    let access = Access {
        session_id: "session".to_owned(),
        subject: "person-00000000000000000000000000000000".to_owned(),
        app: "notes".to_owned(),
        profile: true,
        expires_at: 7,
    };
    let written = serde_json::to_value(&access)?;
    let mut keys: Vec<&str> = written
        .as_object()
        .ok_or("an object")?
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["app", "expires_at", "profile", "session_id", "subject"]
    );
    let mut with_name = written;
    with_name["name"] = json!("Ada");
    assert!(serde_json::from_value::<Access>(with_name).is_err());
    Ok(())
}

/// DIRECTORY-079 R3: discovery says only what is served.
#[test]
fn discovery_lists_the_openid_and_profile_scopes_and_the_name_claim() -> Result<(), Box<dyn Error>>
{
    let dir = tempfile::tempdir()?;
    let key_file = dir.path().join("provider.key");
    std::fs::write(&key_file, [9u8; 32])?;
    let settings = ProviderSettings {
        key_file,
        code_seconds: 60,
    };
    let provider = OpenIdProvider::open(&settings, "http://localhost:8490".to_owned())?;
    let discovery = provider.discovery();
    assert_eq!(discovery["scopes_supported"], json!(["openid", "profile"]));
    assert_eq!(
        discovery["claims_supported"],
        json!([
            "iss",
            "sub",
            "aud",
            "iat",
            "exp",
            "auth_time",
            "nonce",
            "name"
        ])
    );
    Ok(())
}

/// A provider with one code for the fixture client, exchangeable with the
/// verifier `v` at the address `back`.
fn provider_with_code(code: &str) -> Result<(tempfile::TempDir, OpenIdProvider), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let key_file = dir.path().join("provider.key");
    std::fs::write(&key_file, [3u8; 32])?;
    let settings = ProviderSettings {
        key_file,
        code_seconds: 60,
    };
    let provider = OpenIdProvider::open(&settings, "http://localhost:8490".to_owned())?;
    held(&provider.codes)?.insert(
        code.to_owned(),
        Grant {
            session_id: "a".repeat(32),
            client_id: "notes".to_owned(),
            redirect_uri: "https://notes.example.test/back".to_owned(),
            challenge: URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(b"v")),
            nonce: None,
            subject: format!("person-{}", "b".repeat(32)),
            person: format!("person-{}", "b".repeat(32)).parse()?,
            profile: false,
            authenticated_at: 1,
            expires_at: 1000,
            sign_in_ends_at: 1000,
            used: false,
            replayed: false,
            issued_access: None,
        },
    );
    // The table of tokens lives beside the key, so the directory stays.
    Ok((dir, provider))
}

fn access() -> Access {
    Access {
        session_id: "a".repeat(32),
        subject: format!("person-{}", "b".repeat(32)),
        app: "notes".to_owned(),
        profile: false,
        expires_at: 1000,
    }
}

/// A code exchanged a second time between the exchange's two acts ends with no
/// live token: the replay is refused `CodeUsed` and marks the code, and the
/// first exchange then keeps nothing and answers `CodeUsed` too.
#[test]
fn a_code_replayed_between_the_two_acts_of_an_exchange_leaves_no_live_token()
-> Result<(), Box<dyn Error>> {
    let (_dir, provider) = provider_with_code("code-1")?;
    let lookup = "c".repeat(64);
    provider.take_grant(
        "code-1",
        "notes",
        "https://notes.example.test/back",
        "v",
        10,
    )?;
    assert!(matches!(
        provider.take_grant(
            "code-1",
            "notes",
            "https://notes.example.test/back",
            "v",
            11
        ),
        Err(ServerError::CodeUsed)
    ));
    assert!(matches!(
        provider.issue("code-1", lookup.clone(), access(), 12),
        Err(ServerError::CodeUsed)
    ));
    assert!(matches!(
        held(&provider.tokens)?.get(&lookup, 12),
        Err(ServerError::TokenUnknown)
    ));
    Ok(())
}

/// A code exchanged once keeps its token and remembers it; a replay after the
/// issue revokes that token.
#[test]
fn a_code_replayed_after_its_exchange_revokes_the_token_it_issued() -> Result<(), Box<dyn Error>> {
    let (_dir, provider) = provider_with_code("code-2")?;
    let lookup = "c".repeat(64);
    provider.take_grant(
        "code-2",
        "notes",
        "https://notes.example.test/back",
        "v",
        10,
    )?;
    provider.issue("code-2", lookup.clone(), access(), 10)?;
    assert_eq!(held(&provider.tokens)?.get(&lookup, 11)?.app, "notes");
    assert!(matches!(
        provider.take_grant(
            "code-2",
            "notes",
            "https://notes.example.test/back",
            "v",
            12
        ),
        Err(ServerError::CodeUsed)
    ));
    assert!(matches!(
        held(&provider.tokens)?.get(&lookup, 12),
        Err(ServerError::TokenUnknown)
    ));
    Ok(())
}
