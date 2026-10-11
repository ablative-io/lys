use super::{DOMAIN, endpoint, hex, read_key, signature};
use base64::Engine;
use lys_core::attestation::{Attestation, verify_attestation_bytes_by_signer};
use lys_core::keys::Ed25519Identity;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const PKCS8_PREFIX: [u8; 16] = super::PKCS8_ED25519_PREFIX;

fn pem(seed: [u8; 32]) -> String {
    let mut der = PKCS8_PREFIX.to_vec();
    der.extend_from_slice(&seed);
    format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
        base64::engine::general_purpose::STANDARD.encode(der)
    )
}

/// The header is the four words Lys's `agent_signature` reads, and its attestation
/// verifies, by the agent's own key, over exactly the payload Lys rebuilds.
#[test]
fn a_signed_message_carries_the_payload_lys_verifies() -> TestResult {
    let identity = Ed25519Identity::from_seed(&Zeroizing::new([7; 32]));
    let body = br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    let header = signature(
        &identity,
        "agent-00000000000000000000000000000001",
        "/mcp",
        body,
    );
    let [agent, millis, nonce, attestation] = header.split(' ').collect::<Vec<_>>()[..] else {
        return Err(format!("not four words: {header}").into());
    };
    assert_eq!(agent, "agent-00000000000000000000000000000001");
    assert!(nonce.len() >= 32, "a nonce of sixteen bytes in hex");
    let payload = format!(
        "{DOMAIN}\nPOST\n/mcp\n{}\n{millis}\n{nonce}",
        hex(&Sha256::digest(body))
    );
    let bytes = (0..attestation.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&attestation[at..at + 2], 16))
        .collect::<Result<Vec<_>, _>>()?;
    let verified: Result<Attestation, _> = verify_attestation_bytes_by_signer(
        &bytes,
        payload.as_bytes(),
        &identity.public_key_bytes(),
    );
    assert!(
        verified.is_ok(),
        "the attestation verifies over Lys's payload"
    );
    Ok(())
}

/// The key is read from a seat file's `privateKeyPem`, as the seat launcher passes it,
/// and from a bare PEM file; a file with neither is refused by name.
#[test]
fn the_key_is_read_from_a_seat_file_or_a_pem() -> TestResult {
    let dir = std::env::temp_dir().join(format!("lys-mcp-bridge-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let seat = dir.join("seat.json");
    std::fs::write(
        &seat,
        serde_json::json!({"handle":"h","privateKeyPem":pem([9; 32])}).to_string(),
    )?;
    let bare = dir.join("key.pem");
    std::fs::write(&bare, pem([9; 32]))?;
    let expected = Ed25519Identity::from_seed(&Zeroizing::new([9; 32])).public_key_bytes();
    assert_eq!(read_key(&seat)?.public_key_bytes(), expected);
    assert_eq!(read_key(&bare)?.public_key_bytes(), expected);
    let empty = dir.join("empty.json");
    std::fs::write(&empty, "{}")?;
    let refused = read_key(&empty)
        .err()
        .ok_or("a seat file with no key was read")?;
    assert!(
        refused.to_string().contains("mcp_key_unreadable"),
        "{refused}"
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// The bridge posts only to a numeric loopback address, at the path the URL names.
#[test]
fn only_a_loopback_url_is_reached() -> TestResult {
    let (authority, path) = endpoint("http://127.0.0.1:8490/api/mcp")?;
    assert_eq!((authority.port, path.as_str()), (8490, "/api/mcp"));
    assert!(endpoint("https://lys.example/mcp").is_err());
    assert!(endpoint("http://lys.example/mcp").is_err());
    Ok(())
}
