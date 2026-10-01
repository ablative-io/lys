#![cfg(test)]
use super::draft_tests::{actor, approval, created, home, open};
use super::{cose_sign1, protected_header, sig_structure, verify_event};
use crate::{Entry, OperationId};
use ciborium::Value;
use lys_core::Ed25519Identity;

fn encoded(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::into_writer(value, &mut bytes).unwrap();
    bytes
}
fn op(byte: u8) -> Value {
    Value::Text(OperationId::from_bytes([byte; 16]).to_string())
}
fn event(body: &[u8], key: &Ed25519Identity) -> crate::draft_event::DraftEvent {
    let protected = protected_header(
        &key.public_key_bytes(),
        "application/vnd.lys.identity-draft.v2+cbor",
    );
    let signature = key.sign(&sig_structure(&protected, body));
    let signed = verify_event(
        &cose_sign1(&protected, body, &signature),
        &key.public_key_bytes(),
    )
    .unwrap();
    let Entry::Draft(event) = signed.entry() else {
        panic!("fixture must be a draft");
    };
    assert_eq!(crate::draft_event::encode(event), body);
    let mut padded = body.to_vec();
    padded.push(0);
    let signature = key.sign(&sig_structure(&protected, &padded));
    assert!(
        verify_event(
            &cose_sign1(&protected, &padded, &signature),
            &key.public_key_bytes()
        )
        .is_err()
    );
    let mut longer = body.to_vec();
    longer.splice(1..2, [0x18, 0x02]);
    let signature = key.sign(&sig_structure(&protected, &longer));
    assert!(
        verify_event(
            &cose_sign1(&protected, &longer, &signature),
            &key.public_key_bytes()
        )
        .is_err()
    );
    event.as_ref().clone()
}

#[test]
fn a_refusal_over_another_hash_is_refused_without_an_append() {
    let home = home();
    let key = Ed25519Identity::load(&home.path().join("key")).unwrap();
    let mut directory = open(&home);
    directory
        .record_draft(crate::draft_event::decode(&created()).unwrap())
        .unwrap();
    let bytes = encoded(&Value::Array(vec![
        2.into(),
        2.into(),
        op(6),
        actor(),
        2.into(),
        op(1),
        Value::Bytes(vec![0; 32]),
        Value::Text("refused".into()),
        Value::Null,
    ]));
    assert_eq!(
        directory.record_draft(event(&bytes, &key)).unwrap_err(),
        crate::IdentityError::DraftHashMismatch
    );
    assert!(directory.committed_at(1).unwrap().is_none());
    let hash = crate::encoding::payload_commitment(&created());
    let bytes = encoded(&Value::Array(vec![
        2.into(),
        2.into(),
        op(6),
        actor(),
        2.into(),
        op(1),
        Value::Bytes(hash.to_vec()),
        Value::Text("refused".into()),
        Value::Null,
    ]));
    directory.record_draft(event(&bytes, &key)).unwrap();
    assert!(
        directory
            .projection()
            .unwrap()
            .draft(OperationId::from_bytes([1; 16]))
            .unwrap()
            .is_refused()
    );
    drop(directory);
    let mut directory = open(&home);
    assert!(
        directory
            .record_draft(approval(hash))
            .unwrap_err()
            .to_string()
            .starts_with("DraftNotPending:")
    );
}

#[test]
fn a_correction_refuses_the_original_with_a_link_and_saves_the_correctors_change() {
    let home = home();
    let key = Ed25519Identity::load(&home.path().join("key")).unwrap();
    let hash = crate::encoding::payload_commitment(&created());
    let mut directory = open(&home);
    directory
        .record_draft(crate::draft_event::decode(&created()).unwrap())
        .unwrap();
    let Value::Array(mut corrected) =
        ciborium::from_reader::<Value, _>(created().as_slice()).unwrap()
    else {
        panic!("creation must be an array");
    };
    corrected[2] = op(5);
    let Value::Map(by) = &mut corrected[3] else {
        panic!("actor must be a map");
    };
    by[1].1 = Value::Text("corrector".into());
    corrected[4] = 3.into();
    corrected[8] = Value::Bytes(b"{\"changed\":true}".to_vec());
    corrected[11] = op(1);
    let corrector = corrected[3].clone();
    let corrected = encoded(&Value::Array(corrected));
    let bytes = encoded(&Value::Array(vec![
        2.into(),
        3.into(),
        op(6),
        corrector,
        3.into(),
        op(1),
        Value::Bytes(hash.to_vec()),
        Value::Text("corrected".into()),
        Value::Bytes(corrected.clone()),
        Value::Bytes(crate::encoding::payload_commitment(&corrected).to_vec()),
    ]));
    directory.record_draft(event(&bytes, &key)).unwrap();
    assert!(
        directory
            .record_draft(approval(hash))
            .unwrap_err()
            .to_string()
            .starts_with("DraftNotPending:")
    );
    let own = directory
        .projection()
        .unwrap()
        .draft(OperationId::from_bytes([5; 16]))
        .unwrap();
    assert_eq!(own.created.actor.binding().subject(), "corrector");
    assert_eq!(own.created.corrects, Some(OperationId::from_bytes([1; 16])));
    drop(directory);
    let mut directory = open(&home);
    assert!(
        directory
            .record_draft(approval(hash))
            .unwrap_err()
            .to_string()
            .starts_with("DraftNotPending:")
    );
    assert!(
        directory
            .projection()
            .unwrap()
            .draft(OperationId::from_bytes([5; 16]))
            .is_some()
    );
}

#[test]
fn the_agents_request_signature_is_kept_with_the_leaf() {
    let home = home();
    let key = Ed25519Identity::load(&home.path().join("key")).unwrap();
    let agent = crate::AgentId::from_bytes([7; 16]);
    let body = b"{\"jsonrpc\":\"2.0\",\"id\":7}";
    let nonce = "AB".repeat(16);
    let payload = format!(
        "lys-identity/agent-request/v1\nPOST\n/mcp\n{}\n1\n{nonce}",
        crate::id::to_hex(&crate::encoding::payload_commitment(body))
    )
    .into_bytes();
    let cose = lys_core::attestation::sign_attestation(&payload, &key).to_cose_bytes();
    let header = format!("{agent}  0001\t{nonce} {}", crate::id::to_hex(&cose)).into_bytes();
    let Value::Array(mut fields) = ciborium::from_reader::<Value, _>(created().as_slice()).unwrap()
    else {
        panic!("creation must be an array");
    };
    fields[0] = 2.into();
    let Value::Map(by) = &mut fields[3] else {
        panic!("actor must be a map");
    };
    by[2].1 = 2.into();
    by.push((5.into(), Value::Bytes(agent.as_bytes().to_vec())));
    fields[12] = Value::Array(vec![
        Value::Text(agent.to_string()),
        Value::Text("POST".into()),
        Value::Text("/mcp".into()),
        Value::Bytes(body.to_vec()),
        1.into(),
        Value::Text(nonce),
        Value::Bytes(cose),
    ]);
    fields.push(Value::Array(vec![
        Value::Bytes(header.clone()),
        Value::Bytes(payload.clone()),
    ]));
    let bytes = encoded(&Value::Array(fields));
    let event = event(&bytes, &key);
    let crate::draft_event::DraftEvent::Created(creation) = &event else {
        panic!("fixture must be a creation");
    };
    let mut changed = creation.as_ref().clone();
    changed
        .request_signature
        .as_mut()
        .unwrap()
        .payload
        .push(b'\n');
    assert!(
        crate::sign_draft_event(
            crate::draft_event::DraftEvent::Created(std::sync::Arc::new(changed)),
            &key
        )
        .is_err()
    );
    assert_eq!(crate::draft_event::encode(&event), bytes);
    let mut directory = open(&home);
    directory.record_draft(event).unwrap();
    let message = directory
        .committed_at(0)
        .unwrap()
        .unwrap()
        .0
        .bytes()
        .to_vec();
    assert!(message.windows(header.len()).any(|part| part == header));
    assert!(message.windows(payload.len()).any(|part| part == payload));
    drop(directory);
    assert_eq!(
        open(&home).committed_at(0).unwrap().unwrap().0.bytes(),
        message
    );
}
