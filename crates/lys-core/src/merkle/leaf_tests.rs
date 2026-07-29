#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use serde::Serialize;

#[derive(Serialize)]
struct Sample {
    id: u32,
    label: String,
}

#[test]
fn serialize_leaf_is_deterministic_for_identical_inputs() {
    let a = Sample {
        id: 7,
        label: "alpha".to_string(),
    };
    let b = Sample {
        id: 7,
        label: "alpha".to_string(),
    };
    let bytes_a = serialize_leaf(&a).unwrap();
    let bytes_b = serialize_leaf(&b).unwrap();
    assert_eq!(bytes_a.as_bytes(), bytes_b.as_bytes());
}

#[test]
fn serialize_leaf_differs_between_distinct_inputs() {
    let a = Sample {
        id: 7,
        label: "alpha".to_string(),
    };
    let c = Sample {
        id: 8,
        label: "alpha".to_string(),
    };
    let bytes_a = serialize_leaf(&a).unwrap();
    let bytes_c = serialize_leaf(&c).unwrap();
    assert_ne!(bytes_a.as_bytes(), bytes_c.as_bytes());
}

#[test]
fn serialized_leaf_exposes_bytes_through_as_ref() {
    let leaf = serialize_leaf(&123u64).unwrap();
    let as_ref: &[u8] = leaf.as_ref();
    assert_eq!(as_ref, leaf.as_bytes());
}
