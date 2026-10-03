#![cfg(test)]
//! The snapshot format read back strictly: a sealed snapshot opens to what
//! was sealed, and each way a body can be wrong under a good signature is
//! refused by its own name.

use super::*;

const DOMAIN: &str = "example.com/lys/test-state/v1";
const ORIGIN: &str = "example.com/lys/snapshot-test";

fn key(dir: &tempfile::TempDir) -> Ed25519Identity {
    Ed25519Identity::load_or_generate(&dir.path().join("key")).unwrap()
}

fn frontier(count: u64) -> Frontier {
    Frontier::from_leaves((0..count).map(|index| format!("leaf-{index}").into_bytes()))
}

/// Signs `body` as a sealed snapshot, whatever it holds.
fn signed(body: &[u8], key: &Ed25519Identity) -> Vec<u8> {
    let mut sealed = Vec::new();
    field(&mut sealed, body);
    field(&mut sealed, &key.sign(body));
    sealed
}

fn body(size: u64, root: [u8; 32], nodes: &[u8], trailing: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    field(&mut body, SNAPSHOT_FORMAT.as_bytes());
    field(&mut body, DOMAIN.as_bytes());
    field(&mut body, ORIGIN.as_bytes());
    body.extend_from_slice(&size.to_be_bytes());
    body.extend_from_slice(&root);
    field(&mut body, nodes);
    field(&mut body, b"state");
    body.extend_from_slice(trailing);
    body
}

#[test]
fn a_sealed_snapshot_unseals_to_what_was_sealed() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir);
    let tree = frontier(11);
    let sealed = seal(DOMAIN, ORIGIN, &tree, b"the state", &key);
    let snapshot = unseal(&sealed, DOMAIN, ORIGIN, &key.public_key_bytes()).unwrap();
    assert_eq!(snapshot.frontier(), &tree);
    assert_eq!(snapshot.state(), b"the state");
}

#[test]
fn a_signed_body_whose_frontier_does_not_fold_to_its_root_is_the_wrong_root() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir);
    let tree = frontier(5);
    let sealed = signed(&body(5, [7u8; 32], &tree.nodes().concat(), b""), &key);
    let refusal = unseal(&sealed, DOMAIN, ORIGIN, &key.public_key_bytes()).unwrap_err();
    assert_eq!(refusal, SnapshotRefusal::WrongRoot { size: 5 });
}

#[test]
fn a_signed_body_with_the_wrong_node_count_is_malformed() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir);
    let tree = frontier(5);
    let sealed = signed(&body(4, tree.root(), &tree.nodes().concat(), b""), &key);
    let refusal = unseal(&sealed, DOMAIN, ORIGIN, &key.public_key_bytes()).unwrap_err();
    assert!(
        matches!(refusal, SnapshotRefusal::Malformed { .. }),
        "{refusal}"
    );
}

#[test]
fn a_signed_body_with_bytes_after_its_last_field_is_malformed() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir);
    let tree = frontier(3);
    let sealed = signed(&body(3, tree.root(), &tree.nodes().concat(), b"x"), &key);
    let refusal = unseal(&sealed, DOMAIN, ORIGIN, &key.public_key_bytes()).unwrap_err();
    assert!(
        matches!(refusal, SnapshotRefusal::Malformed { .. }),
        "{refusal}"
    );
}

#[test]
fn bytes_after_the_signature_are_malformed() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir);
    let mut sealed = seal(DOMAIN, ORIGIN, &frontier(2), b"s", &key);
    sealed.push(0);
    let refusal = unseal(&sealed, DOMAIN, ORIGIN, &key.public_key_bytes()).unwrap_err();
    assert!(
        matches!(refusal, SnapshotRefusal::Malformed { .. }),
        "{refusal}"
    );
}

#[test]
fn an_empty_signature_is_unsigned() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir);
    let tree = frontier(2);
    let mut sealed = Vec::new();
    field(
        &mut sealed,
        &body(2, tree.root(), &tree.nodes().concat(), b""),
    );
    field(&mut sealed, &[]);
    let refusal = unseal(&sealed, DOMAIN, ORIGIN, &key.public_key_bytes()).unwrap_err();
    assert_eq!(refusal, SnapshotRefusal::Unsigned);
}

#[test]
fn every_refusal_opens_with_its_name() {
    let refusals = [
        (SnapshotRefusal::Missing, "SnapshotMissing:"),
        (SnapshotRefusal::Unsigned, "SnapshotUnsigned:"),
        (
            SnapshotRefusal::SignatureInvalid,
            "SnapshotSignatureInvalid:",
        ),
        (SnapshotRefusal::WrongRoot { size: 1 }, "SnapshotWrongRoot:"),
    ];
    for (refusal, name) in &refusals {
        assert!(refusal.to_string().starts_with(name), "{refusal}");
    }
    assert_eq!(refusals.len(), 4);
}
