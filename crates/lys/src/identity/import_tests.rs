#![cfg(test)]

use super::{authority, credential, verify_receipts};
use serde_json::json;

const ACCOUNT: &str = "op-01010101010101010101010101010101";

#[test]
fn bearer_can_only_go_to_numeric_loopback() {
    for allowed in ["127.0.0.1:8490", "[::1]:8490"] {
        assert!(authority(allowed).is_ok());
    }
    for refused in [
        "identity.example:8490",
        "192.0.2.1:8490",
        "localhost:8490",
        "127.0.0.1:0",
        "127.0.0.1:8490/remote",
    ] {
        assert!(authority(refused).is_err());
    }
}

#[test]
fn credential_refusal_never_prints_the_credential() {
    let secret = "secret-do-not-echo";
    for input in [
        format!("lys-operator.{secret}"),
        format!("lys-registrar.{ACCOUNT}.{secret}\r\nCookie: forged"),
    ] {
        let error = credential(input.as_bytes()).unwrap_err();
        assert!(!error.to_string().contains(secret));
    }
    let good = format!("lys-registrar.{ACCOUNT}.{}", "ab".repeat(32));
    assert_eq!(credential(good.as_bytes()).unwrap(), good);
}

#[test]
fn success_must_name_every_requested_operation_and_the_actual_account() {
    let plan =
        lys_identity::import_document::parse(br#"{"agents":[{"display_name":"Gypsy"}]}"#, ACCOUNT)
            .unwrap();
    let response = json!({"by":{"kind":"service_account","id":ACCOUNT},"completed":[{
        "entry":"agents/Gypsy", "operation":plan[0].operation.to_string(), "result":{"agent":"agent-01010101010101010101010101010101"}
    }]});
    assert!(verify_receipts(&plan, ACCOUNT, &response).is_ok());
    for (pointer, wrong) in [
        ("/by/kind", json!("person")),
        ("/by/id", json!("another-account")),
        ("/completed/0/operation", json!("another-operation")),
        ("/completed/0/entry", json!("agents/Apollo")),
        ("/completed", json!([])),
    ] {
        let mut changed = response.clone();
        *changed.pointer_mut(pointer).unwrap() = wrong;
        assert!(
            verify_receipts(&plan, ACCOUNT, &changed).is_err(),
            "{pointer}"
        );
    }
}
