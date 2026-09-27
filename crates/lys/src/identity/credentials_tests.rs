use std::error::Error;

use super::*;

/// Whether `text` holds any eight consecutive characters of `secret`.
fn leaks(text: &str, secret: &str) -> bool {
    let bytes = secret.as_bytes();
    bytes.windows(8.min(bytes.len())).any(|window| {
        text.as_bytes()
            .windows(window.len())
            .any(|seen| seen == window)
    })
}

#[test]
fn alphanumeric_credentials_have_the_declared_length_and_alphabet() {
    let credential = Credential::generate("db-password", Shape::Alphanumeric(48));
    assert_eq!(credential.expose().len(), 48);
    assert!(
        credential
            .expose()
            .bytes()
            .all(|b| b.is_ascii_alphanumeric())
    );
    let other = Credential::generate("db-password", Shape::Alphanumeric(48));
    assert_ne!(credential.expose(), other.expose());
}

#[test]
fn the_encryption_key_is_an_id_and_32_bytes() -> Result<(), Box<dyn Error>> {
    let credential = Credential::generate("rauthy-encryption-key", Shape::EncryptionKey);
    let encoded = credential
        .expose()
        .strip_prefix("lys01/")
        .ok_or("the key id is missing")?;
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(encoded)?
            .len(),
        32
    );
    Ok(())
}

#[test]
fn a_stored_credential_is_reused_byte_for_byte() -> Result<(), Box<dyn Error>> {
    let generated = Credential::generate("api-secret", Shape::Alphanumeric(64));
    let mut stored = generated.expose().as_bytes().to_vec();
    let reread = Credential::from_stored("api-secret", &stored, Shape::Alphanumeric(64))?;
    assert_eq!(reread.expose(), generated.expose());
    stored.push(b'\n');
    let with_newline = Credential::from_stored("api-secret", &stored, Shape::Alphanumeric(64))?;
    assert_eq!(with_newline.expose(), generated.expose());
    Ok(())
}

#[test]
fn a_stored_credential_of_the_wrong_shape_is_refused_without_its_bytes() {
    let wrong = "short-but-secret-value";
    for shape in [Shape::Alphanumeric(48), Shape::EncryptionKey] {
        let result = Credential::from_stored("db-password", wrong.as_bytes(), shape);
        assert!(
            result.is_err_and(|error| error.kind() == ErrorKind::SecretMalformed
                && !leaks(&error.to_string(), wrong)
                && !leaks(&format!("{error:?}"), wrong))
        );
    }
    let short_key = "lys01/AAAA";
    let result = Credential::from_stored("key", short_key.as_bytes(), Shape::EncryptionKey);
    assert!(result.is_err_and(|error| error.kind() == ErrorKind::SecretMalformed));
}

#[test]
fn debug_and_display_never_show_a_generated_secret() {
    for shape in [
        Shape::Alphanumeric(32),
        Shape::Alphanumeric(64),
        Shape::EncryptionKey,
    ] {
        let credential = Credential::generate("generated", shape);
        let secret = credential.expose().to_string();
        let debug = format!("{credential:?}");
        let display = format!("{credential}");
        assert!(!leaks(&debug, &secret), "{debug}");
        assert!(!leaks(&display, &secret), "{display}");
        assert!(debug.contains("redacted") && display.contains("redacted"));
    }
    let source = Credential::generate("source", Shape::Alphanumeric(40));
    let received = Credential::received(
        "cambium-client-secret",
        Zeroizing::new(source.expose().to_string()),
    );
    assert!(!leaks(
        &format!("{received:?} {received}"),
        received.expose()
    ));
}
