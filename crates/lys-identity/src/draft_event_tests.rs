#![cfg(test)]

use ciborium::Value;
use lys_core::Ed25519Identity;

use super::{cose_sign1, protected_header, sig_structure, verify_event};
use crate::OperationId;

pub(super) fn actor() -> Value {
    Value::Map(vec![
        (1.into(), Value::Text("https://issuer.example/".into())),
        (2.into(), Value::Text("subject".into())),
        (3.into(), 1.into()),
        (4.into(), 1.into()),
    ])
}

pub(super) fn created() -> Vec<u8> {
    created_on("team", "team.example", "write")
}

/// A created draft whose target is `kind`, `id` and `action`.
fn created_on(kind: &str, id: &str, action: &str) -> Vec<u8> {
    let value = Value::Array(vec![
        1.into(),
        0.into(),
        Value::Text(OperationId::from_bytes([1; 16]).to_string()),
        actor(),
        1.into(),
        Value::Array(vec![
            Value::Text(kind.into()),
            Value::Text(id.into()),
            Value::Text(action.into()),
        ]),
        Value::Text("POST".into()),
        Value::Text("/api/teams/example".into()),
        Value::Bytes(b"{\"operation\":\"exact\"}".to_vec()),
        Value::Text("review".into()),
        Value::Text(format!("person-{}", "02".repeat(16))),
        Value::Null,
        Value::Null,
    ]);
    let mut out = Vec::new();
    ciborium::into_writer(&value, &mut out).unwrap();
    out
}

fn leaf(body: &[u8], key: &Ed25519Identity) -> Vec<u8> {
    let protected = protected_header(
        &key.public_key_bytes(),
        "application/vnd.lys.identity-draft.v1+cbor",
    );
    let signature = key.sign(&sig_structure(&protected, body));
    cose_sign1(&protected, body, &signature)
}

#[test]
fn a_draft_names_its_resource_id_as_the_product_names_it() {
    let stream = "mHyUs0FppSFqSzfT5kjBzZk23asX6tgScUEKNwJawq0";
    let decoded = crate::draft_event::decode(&created_on("team", stream, "write")).unwrap();
    let crate::draft_event::DraftEvent::Created(event) = decoded else {
        panic!("a created draft decodes as created");
    };
    assert_eq!(event.target.id, stream);
    for bad in ["a/b", "a:b", "a=b", "a+b", "a b"] {
        assert!(
            crate::draft_event::decode(&created_on("team", bad, "write")).is_err(),
            "{bad:?}"
        );
    }
    assert!(crate::draft_event::decode(&created_on("Team", "one", "write")).is_err());
    assert!(crate::draft_event::decode(&created_on("team", "one", "Write")).is_err());
}

#[test]
fn a_draft_decodes_only_from_its_canonical_bytes() {
    let home = tempfile::tempdir().unwrap();
    let key = Ed25519Identity::load_or_generate(&home.path().join("key")).unwrap();
    let body = created();
    let signed = verify_event(&leaf(&body, &key), &key.public_key_bytes()).unwrap();
    assert_eq!(
        signed.payload_commitment(),
        crate::encoding::payload_commitment(&body)
    );
    let mut longer = body.clone();
    longer.splice(1..2, [0x18, 0x01]);
    assert!(verify_event(&leaf(&longer, &key), &key.public_key_bytes()).is_err());
    let mut padded = body;
    padded.push(0);
    assert!(verify_event(&leaf(&padded, &key), &key.public_key_bytes()).is_err());
}

#[test]
fn an_approval_over_another_hash_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let key = Ed25519Identity::load_or_generate(&home.path().join("key")).unwrap();
    let signed = verify_event(&leaf(&created(), &key), &key.public_key_bytes()).unwrap();
    let mut projection = crate::projection::Projection::new();
    projection.apply_entry(signed.entry(), 0).unwrap();
    let value = Value::Array(vec![
        1.into(),
        1.into(),
        Value::Text(OperationId::from_bytes([3; 16]).to_string()),
        actor(),
        2.into(),
        Value::Text(OperationId::from_bytes([1; 16]).to_string()),
        Value::Bytes(vec![0; 32]),
        Value::Text(OperationId::from_bytes([4; 16]).to_string()),
        Value::Null,
    ]);
    let mut body = Vec::new();
    ciborium::into_writer(&value, &mut body).unwrap();
    let approved = verify_event(&leaf(&body, &key), &key.public_key_bytes()).unwrap();
    let before = projection.shared();
    let failure = projection.apply_entry(approved.entry(), 1).unwrap_err();
    assert!(failure.to_string().starts_with("DraftHashMismatch:"));
    assert_eq!(projection, before);
}

pub(super) fn open(
    home: &std::sync::Arc<tempfile::TempDir>,
) -> crate::Directory<lys_log_store::FileLeafStore> {
    let home = std::sync::Arc::clone(home);
    let key = Ed25519Identity::load(&home.path().join("key")).unwrap();
    crate::Directory::open_with(
        Box::new(move || lys_log_store::FileLeafStore::open(&home.path().join("log"))),
        key,
        std::num::NonZeroU64::new(1).unwrap(),
    )
    .unwrap()
}

pub(super) fn home() -> std::sync::Arc<tempfile::TempDir> {
    let home = std::sync::Arc::new(tempfile::tempdir().unwrap());
    Ed25519Identity::load_or_generate(&home.path().join("key")).unwrap();
    lys_log_store::FileLeafStore::create(&home.path().join("log"), "example.com/lys/draft-test")
        .unwrap();
    home
}

pub(super) fn approval(hash: [u8; 32]) -> crate::draft_event::DraftEvent {
    let crate::draft_event::DraftEvent::Created(created) =
        crate::draft_event::decode(&created()).unwrap()
    else {
        panic!("fixture must be a creation");
    };
    crate::draft_event::DraftEvent::Approved(std::sync::Arc::new(crate::draft_event::Approved {
        operation: OperationId::from_bytes([3; 16]),
        actor: created.actor.clone(),
        recorded_at: 2,
        draft: created.operation,
        draft_hash: hash,
        application: OperationId::from_bytes([4; 16]),
        evidence: None,
    }))
}

#[test]
fn a_mismatched_approval_never_appends_to_the_directory() {
    let home = home();
    let mut directory = open(&home);
    directory
        .record_draft(crate::draft_event::decode(&created()).unwrap())
        .unwrap();
    assert_eq!(
        directory.record_draft(approval([0; 32])).unwrap_err(),
        crate::IdentityError::DraftHashMismatch
    );
    assert!(directory.committed_at(1).unwrap().is_none());
    assert!(
        directory
            .projection()
            .unwrap()
            .draft(OperationId::from_bytes([1; 16]))
            .unwrap()
            .approved
            .is_none()
    );
    drop(directory);
    assert!(open(&home).committed_at(1).unwrap().is_none());
}

#[test]
fn drafts_survive_snapshot_reopen_and_exact_retries() {
    let home = home();
    let event = crate::draft_event::decode(&created()).unwrap();
    let hash = crate::encoding::payload_commitment(&created());
    let mut directory = open(&home);
    let coordinate = directory.record_draft(event.clone()).unwrap();
    assert_eq!(directory.record_draft(event.clone()).unwrap(), coordinate);
    drop(directory);
    let mut directory = open(&home);
    assert_eq!(
        directory
            .projection()
            .unwrap()
            .draft(event.operation())
            .unwrap()
            .hash,
        hash
    );
    let approval = approval(hash);
    let coordinate = directory.record_draft(approval.clone()).unwrap();
    assert_eq!(coordinate.index, 1);
    assert_eq!(
        directory.record_draft(approval.clone()).unwrap(),
        coordinate
    );
    drop(directory);
    let mut directory = open(&home);
    let held = directory
        .projection()
        .unwrap()
        .draft(event.operation())
        .unwrap();
    assert_eq!(held.hash, hash);
    assert!(held.approved.is_some());
    assert_eq!(directory.record_draft(approval).unwrap(), coordinate);
    assert!(directory.committed_at(2).unwrap().is_none());
}

#[test]
fn a_version_three_install_snapshot_migrates_without_rewriting_leaves() {
    old_install_migrates(3);
}

#[test]
fn a_version_four_draft_snapshot_migrates_without_rewriting_leaves() {
    old_install_migrates(4);
}

fn old_install_migrates(version: u8) {
    let home = home();
    let crate::draft_event::DraftEvent::Created(draft) =
        crate::draft_event::decode(&created()).unwrap()
    else {
        panic!("fixture must be a creation");
    };
    let mut directory = open(&home);
    let (person, _) = directory
        .setup_person(
            draft.actor.clone(),
            OperationId::from_bytes([9; 16]),
            crate::Profile::new("Example").unwrap(),
            1,
        )
        .unwrap();
    let before = directory
        .committed_at(0)
        .unwrap()
        .unwrap()
        .0
        .bytes()
        .to_vec();
    if version == 4 {
        directory
            .record_draft(crate::draft_event::DraftEvent::Created(
                std::sync::Arc::clone(&draft),
            ))
            .unwrap();
    }
    let fold = if version == 4 { 2 } else { 1 };
    let state = crate::directory_state::encode(directory.projection().unwrap(), fold).unwrap();
    drop(directory);
    let Value::Array(mut fields) = crate::state_value::decode(&state).unwrap() else {
        panic!("snapshot must be an array");
    };
    fields[0] = version.into();
    let Value::Array(projection) = &mut fields[2] else {
        panic!("projection must be an array");
    };
    if version == 3 {
        projection.pop();
    } else {
        let Value::Array(drafts) = &mut projection[3] else {
            panic!("drafts must be an array");
        };
        for draft in drafts {
            let Value::Array(row) = draft else {
                panic!("draft record must be an array");
            };
            row.pop();
        }
    }
    let state = crate::state_value::encode(&Value::Array(fields)).unwrap();
    let wrapped =
        crate::checkpoints::wrap(&crate::checkpoints::Checkpoints::default(), &state).unwrap();
    let key = Ed25519Identity::load(&home.path().join("key")).unwrap();
    let (mut log, _) = lys_log_store::FrontierLog::open(
        lys_log_store::FileLeafStore::open(&home.path().join("log")).unwrap(),
    )
    .unwrap();
    log.write_snapshot("lys/identity-directory/v1", &wrapped, &key)
        .unwrap();
    drop(log);
    assert!(
        crate::directory_migration::migrate(
            lys_log_store::FileLeafStore::open(&home.path().join("log")).unwrap(),
            &key
        )
        .unwrap()
    );
    let mut directory = open(&home);
    assert!(
        directory
            .record(crate::IdentityId::Person(person))
            .unwrap()
            .is_some()
    );
    assert_eq!(
        directory.committed_at(0).unwrap().unwrap().0.bytes(),
        before
    );
    directory
        .record_draft(crate::draft_event::DraftEvent::Created(draft))
        .unwrap();
    drop(directory);
    assert!(
        open(&home)
            .projection()
            .unwrap()
            .draft(OperationId::from_bytes([1; 16]))
            .is_some()
    );
}
