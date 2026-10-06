#![cfg(test)]
//! A view that carries a secret never shows it in `Debug`.

use super::{ClientCredentialGiven, RegistrarIssued};

const SECRET: &str = "5ecre7f1a9d0c0ffee5ecre7f1a9d0c0ffee5ecre7f1a9d0c0ffee5ecre7f1a9";

#[test]
fn an_issued_client_credential_names_its_id_and_never_its_value() {
    let issued = ClientCredentialGiven {
        app: "fixture_notes".to_owned(),
        credential_id: "0a1b2c3d4e5f6a7b".to_owned(),
        credential: Some(format!("lys-client.fixture_notes.{SECRET}")),
    };
    let shown = format!("{issued:?}");
    assert!(shown.contains("0a1b2c3d4e5f6a7b"), "{shown}");
    assert!(!shown.contains(SECRET), "{shown}");
}

#[test]
fn an_issued_registrar_names_its_account_and_never_its_credential() {
    let issued = RegistrarIssued {
        service_account: "fixture_registrar".to_owned(),
        credential: Some(format!("lys-registrar.fixture_registrar.{SECRET}")),
    };
    let shown = format!("{issued:?}");
    assert!(shown.contains("fixture_registrar"), "{shown}");
    assert!(!shown.contains(SECRET), "{shown}");
}
