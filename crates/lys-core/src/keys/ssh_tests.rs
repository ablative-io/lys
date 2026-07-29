#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// A real Ed25519 public key and the exact line OpenSSH itself emitted for it.
///
/// Produced by `ssh-keygen -t ed25519 -N '' -C '' -f k`, then the 32-byte key
/// recovered from `k.pub` by decoding the base64 blob and reading its two
/// length-prefixed SSH strings. The pair is therefore an *external* vector:
/// the expected output was written by OpenSSH, not by this crate, so a match
/// proves agreement with the reference implementation rather than
/// self-consistency. Regenerate the same way if it ever needs replacing.
const REFERENCE_KEY: [u8; 32] = [
    0x2c, 0x30, 0x7a, 0x1b, 0xff, 0x56, 0x65, 0x53, 0xa1, 0x1b, 0x52, 0x52, 0xce, 0xff, 0x5b, 0x17,
    0x3d, 0x48, 0xc2, 0x93, 0x4d, 0xc2, 0xec, 0x22, 0xae, 0xa4, 0x8b, 0x3d, 0xc8, 0x23, 0x6b, 0xb1,
];

const REFERENCE_LINE: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICwwehv/VmVToRtSUs7/Wxc9SMKTTcLsIq6kiz3II2ux";

/// The test that actually matters: byte-identity with OpenSSH's own output.
#[test]
fn openssh_public_key_matches_the_openssh_reference_line() {
    assert_eq!(openssh_public_key(&REFERENCE_KEY), REFERENCE_LINE);
}

/// Cross-check against a constant every Ed25519 SSH key shares.
///
/// The fixed prefix encodes the length-prefixed algorithm name and the
/// leading length of the 32-byte key, so it is determined by the format
/// rather than by any particular key. Asserting it catches a broken length
/// prefix even if the reference vector above were ever replaced carelessly.
#[test]
fn every_encoded_key_carries_the_format_mandated_prefix() {
    for seed in [0x00u8, 0x01, 0x7f, 0xff] {
        let line = openssh_public_key(&[seed; 32]);
        assert!(
            line.starts_with("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI"),
            "got: {line}"
        );
    }
}

/// The blob must decode back to exactly the two declared SSH strings.
#[test]
fn encoded_blob_decodes_to_algorithm_name_and_key() {
    let line = openssh_public_key(&REFERENCE_KEY);
    let blob = STANDARD
        .decode(line.strip_prefix("ssh-ed25519 ").unwrap())
        .unwrap();

    let read_string = |at: usize| -> (Vec<u8>, usize) {
        let len = u32::from_be_bytes(blob[at..at + 4].try_into().unwrap()) as usize;
        (blob[at + 4..at + 4 + len].to_vec(), at + 4 + len)
    };

    let (algorithm, next) = read_string(0);
    let (key, end) = read_string(next);
    assert_eq!(algorithm, SSH_ED25519);
    assert_eq!(key, REFERENCE_KEY);
    assert_eq!(end, blob.len(), "no trailing bytes after the key");
}

#[test]
fn allowed_signers_line_is_namespace_scoped_and_carries_the_key() {
    let line = allowed_signers_line("tom@example.com", &REFERENCE_KEY).unwrap();
    assert_eq!(
        line,
        format!("tom@example.com namespaces=\"git\" {REFERENCE_LINE}")
    );
    // Whitespace-separated, four fields, in the order OpenSSH expects.
    let fields: Vec<&str> = line.split(' ').collect();
    assert_eq!(fields.len(), 4);
    assert_eq!(fields[0], "tom@example.com");
    assert_eq!(fields[1], "namespaces=\"git\"");
    assert_eq!(fields[2], "ssh-ed25519");
}

/// A principal with a space would shift every following field.
///
/// This is the quiet-corruption case: the file would still parse, just into
/// a different entry than the operator wrote. Rejected loudly instead.
#[test]
fn principal_containing_whitespace_is_rejected() {
    for bad in ["tom example", "tom\texample", "tom\nexample", " tom"] {
        let err = allowed_signers_line(bad, &REFERENCE_KEY).unwrap_err();
        assert!(
            matches!(err, TrustError::KeyManagement { .. }),
            "got: {err} for {bad:?}"
        );
        assert!(err.to_string().contains("whitespace"), "got: {err}");
    }
}

#[test]
fn empty_principal_is_rejected() {
    let err = allowed_signers_line("", &REFERENCE_KEY).unwrap_err();
    assert!(
        matches!(err, TrustError::KeyManagement { .. }),
        "got: {err}"
    );
    assert!(err.to_string().contains("empty"), "got: {err}");
}

/// No API in this module can emit private material.
#[test]
fn output_never_contains_anything_but_the_public_key() {
    let line = allowed_signers_line("p", &REFERENCE_KEY).unwrap();
    let blob = STANDARD
        .decode(line.split(' ').nth(3).unwrap())
        .expect("the fourth field is the base64 blob");
    // 4 + 11 + 4 + 32: the algorithm name and the public key, nothing else.
    assert_eq!(blob.len(), 51);
}
