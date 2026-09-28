#![cfg(test)]
//! Byte-exactness and structural-rejection tests for
//! `lys/anchor-receipt/v1`.
//!
//! Expected bytes here are **hand-assembled from the specification**, not
//! captured from the encoder's own output. A golden file produced by the code
//! under test only proves the code is self-consistent; these literals prove it
//! agrees with RFC 9052, RFC 8949 and RFC 9942. If the encoder changes, these
//! fail — which is the point, because the format is frozen the moment an anchor
//! signs under it.

use ciborium::value::Value;

use super::*;

mod layout;
mod refusals;

/// A fixed anchor key. Value is irrelevant to encoding; only its length is.
const ANCHOR_KEY: [u8; 32] = [0xa1; 32];
/// A fixed signature.
const SIGNATURE: [u8; 64] = [0x5e; 64];
/// Two fixed path nodes.
const NODE_A: [u8; 32] = [0x11; 32];
const NODE_B: [u8; 32] = [0x22; 32];

/// A CBOR length that must fit a one-byte head, checked rather than truncated.
fn small(n: usize) -> u8 {
    u8::try_from(n).unwrap()
}

/// The protected bucket, written out byte by byte from the spec table.
fn expected_protected(key: &[u8; 32]) -> Vec<u8> {
    let mut out = vec![
        0xa4, // map(4)
        0x01, 0x27, // 1 (alg) => -8 (EdDSA)
        0x03, 0x78, 0x23, // 3 (content type) => text(35)
    ];
    out.extend_from_slice(b"application/vnd.lys.receipt.v1+cbor");
    out.extend_from_slice(&[0x04, 0x58, 0x20]); // 4 (kid) => bstr(32)
    out.extend_from_slice(key);
    out.extend_from_slice(&[0x19, 0x01, 0x8b, 0x01]); // 395 (vds) => 1
    out
}

/// The inner `bstr .cbor` inclusion proof, written out from the RFC 9942 CDDL.
fn expected_inner_proof(tree_size: u8, leaf_index: u8, nodes: &[[u8; 32]]) -> Vec<u8> {
    let mut out = vec![0x83, tree_size, leaf_index]; // array(3), two small uints
    out.push(0x80 | small(nodes.len())); // array(n), n < 24
    for node in nodes {
        out.extend_from_slice(&[0x58, 0x20]);
        out.extend_from_slice(node);
    }
    out
}

/// The unprotected bucket `{396: {-1: [<bstr .cbor proof>]}}`.
fn expected_unprotected(tree_size: u8, leaf_index: u8, nodes: &[[u8; 32]]) -> Vec<u8> {
    let proof = expected_inner_proof(tree_size, leaf_index, nodes);
    let mut out = vec![
        0xa1, // map(1)
        0x19, 0x01, 0x8c, // 396 (vdp)
        0xa1, // map(1)
        0x20, // -1 (inclusion)
        0x81, // array(1) — exactly one proof
    ];
    assert!(proof.len() < 256);
    out.extend_from_slice(&[0x58, small(proof.len())]);
    out.extend_from_slice(&proof);
    out
}

/// A canonical two-node receipt over tree size 4, leaf index 1.
fn sample() -> Vec<u8> {
    artifact_bytes(&ANCHOR_KEY, 4, 1, &[NODE_A, NODE_B], &SIGNATURE)
}
