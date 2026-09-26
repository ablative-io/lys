use std::error::Error;

use super::*;

type TestResult = Result<(), Box<dyn Error>>;

/// Every generated value, for asserting none leaks.
fn values(credentials: &Credentials) -> Vec<String> {
    credentials
        .iter()
        .map(|(_, secret, _)| secret.expose().to_string())
        .collect()
}

#[test]
fn a_fresh_directory_generates_every_declared_credential_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let credentials = Credentials::load_or_generate(dir.path(), true)?;
    let mut generated = 0;
    for (spec, secret, provenance) in credentials.iter() {
        assert_eq!(provenance, Provenance::Generated, "{}", spec.name);
        assert_eq!(
            check_form(spec.form, secret.expose()),
            Ok(()),
            "{}",
            spec.name
        );
        assert!(
            dir.path().join(spec.name).is_file(),
            "{} was not written",
            spec.name
        );
        generated += 1;
    }
    assert_eq!(generated, DECLARED.len());
    let mut distinct = values(&credentials);
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        DECLARED.len(),
        "two credentials share a value"
    );
    Ok(())
}

#[test]
fn a_second_load_reuses_every_value_unchanged() -> TestResult {
    let dir = tempfile::tempdir()?;
    let first = values(&Credentials::load_or_generate(dir.path(), true)?);
    let again = Credentials::load_or_generate(dir.path(), true)?;
    assert!(
        again
            .iter()
            .all(|(_, _, provenance)| provenance == Provenance::Reused)
    );
    assert_eq!(values(&again), first);
    let refusing = Credentials::load_or_generate(dir.path(), false)?;
    assert_eq!(values(&refusing), first);
    Ok(())
}

#[test]
fn a_missing_credential_is_refused_by_name_and_not_generated() -> TestResult {
    let dir = tempfile::tempdir()?;
    Credentials::load_or_generate(dir.path(), true)?;
    let removed = dir.path().join(API_KEY_SECRET);
    std::fs::remove_file(&removed)?;
    let error = Credentials::load_or_generate(dir.path(), false)
        .err()
        .ok_or("a missing credential was accepted")?;
    let message = error.to_string();
    assert!(
        message.starts_with("secret_missing: rauthy_api_key_secret"),
        "got {message}"
    );
    assert!(
        !removed.exists(),
        "the missing credential was generated again"
    );
    Ok(())
}

#[test]
fn a_credential_of_the_wrong_form_is_refused_without_its_value() -> TestResult {
    let dir = tempfile::tempdir()?;
    Credentials::load_or_generate(dir.path(), true)?;
    let tampered = "short-and-not-alphanumeric!";
    private_files::write_private(&dir.path().join(ENC_KEY), tampered.as_bytes())?;
    let message = Credentials::load_or_generate(dir.path(), false)
        .err()
        .ok_or("a malformed key was accepted")?
        .to_string();
    assert!(
        message.starts_with("secret_invalid: rauthy_enc_key"),
        "got {message}"
    );
    assert!(
        !message.contains(tampered),
        "the value reached the message: {message}"
    );
    Ok(())
}

#[test]
fn debug_and_display_of_every_credential_type_carry_no_secret() -> TestResult {
    let dir = tempfile::tempdir()?;
    let credentials = Credentials::load_or_generate(dir.path(), true)?;
    let secrets = values(&credentials);
    let mut rendered = vec![format!("{credentials:?}"), format!("{credentials:#?}")];
    for (spec, secret, provenance) in credentials.iter() {
        rendered.push(format!(
            "{secret:?} {secret:#?} {secret} {spec:?} {provenance:?}"
        ));
    }
    let origin = super::super::config::Origin::parse("http://127.0.0.1:8080")?;
    let api =
        super::super::rauthy::RauthyApi::new(origin, credentials.get(API_KEY_SECRET, dir.path())?);
    rendered.push(format!("{api:?} {api:#?}"));
    let mut checked = 0;
    for text in &rendered {
        for secret in &secrets {
            assert!(!text.contains(secret.as_str()), "a secret reached: {text}");
            checked += 1;
        }
    }
    assert_eq!(checked, rendered.len() * DECLARED.len());
    assert_eq!(rendered.len(), 2 + DECLARED.len() + 1);
    Ok(())
}

#[test]
fn the_encryption_key_names_its_id_and_nothing_else() -> TestResult {
    let dir = tempfile::tempdir()?;
    let credentials = Credentials::load_or_generate(dir.path(), true)?;
    let key = credentials.get(ENC_KEY, dir.path())?;
    let id = key.key_id().ok_or("the key has no id")?;
    assert_eq!(id.len(), 8);
    assert!(key.expose().starts_with(&format!("{id}/")));
    assert!(credentials.get("not_declared", dir.path()).is_err());
    Ok(())
}
