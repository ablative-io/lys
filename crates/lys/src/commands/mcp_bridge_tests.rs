use super::{DOMAIN, endpoint, hex, read_key, signature};
use base64::Engine;
use lys_core::attestation::{Attestation, verify_attestation_bytes_by_signer};
use lys_core::keys::Ed25519Identity;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// PKCS#8 v1 DER for an ed25519 private key, before its seed (RFC 8410 section 7),
/// written out here so the reader is checked against the RFC, not against itself.
const PKCS8_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

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
        serde_json::json!({"version":1,"handle":"h","privateKeyPem":pem([9; 32])}).to_string(),
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

/// AGENTS-017 R5 guard: a bare PEM key file, no JSON, is still read by the
/// bridge after the seat file reader moved to lys-core.
#[test]
fn a_bare_pem_key_file_is_still_read_by_the_bridge() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bare = dir.path().join("agent.pem");
    std::fs::write(&bare, pem([11; 32]))?;
    let expected = Ed25519Identity::from_seed(&Zeroizing::new([11; 32])).public_key_bytes();
    assert_eq!(read_key(&bare)?.public_key_bytes(), expected);
    Ok(())
}

/// AGENTS-017 R5: a JSON seat file with no `version` is refused, naming
/// `seat_file_version_unknown` and the version this lys reads; a file that is
/// not JSON at all after its brace stays a refusal.
#[test]
fn a_json_seat_file_without_version_is_refused_naming_the_version_wanted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let versionless = dir.path().join("old.seat.json");
    std::fs::write(
        &versionless,
        serde_json::json!({"handle":"h","privateKeyPem":pem([12; 32])}).to_string(),
    )?;
    let refused = read_key(&versionless)
        .err()
        .ok_or("a seat file without version was read")?
        .to_string();
    assert!(refused.contains("seat_file_version_unknown"), "{refused}");
    assert!(
        refused.contains(&format!(
            "reads version {}",
            lys_core::seat_file::SEAT_FILE_VERSION
        )),
        "{refused}"
    );
    let torn = dir.path().join("torn.seat.json");
    std::fs::write(&torn, "{\"version\": 1, \"privateKeyPem\": ")?;
    let unparsed = read_key(&torn)
        .err()
        .ok_or("an unparseable seat file was read")?
        .to_string();
    assert!(unparsed.contains("seat_file_unreadable"), "{unparsed}");
    Ok(())
}

/// An https URL to Lys's MCP route is accepted on any host, by name or address
/// (AGENTS-015 R3): a seat reaches Lys from any device.
#[test]
fn an_https_url_on_any_host_is_accepted() {
    for url in [
        "https://lys.example.test/api/mcp",
        "https://203.0.113.7:8443/mcp",
        "https://[2001:db8::1]/api/mcp",
    ] {
        let checked = endpoint(url);
        assert!(
            checked.is_ok(),
            "{url} was refused: {:?}",
            checked.err().map(|refusal| refusal.to_string())
        );
    }
}

/// A plain http URL off loopback is refused, and the refusal says to use https.
#[test]
fn an_http_url_off_loopback_is_refused_naming_https() -> TestResult {
    for url in [
        "http://lys.example.test/api/mcp",
        "http://203.0.113.7:8490/api/mcp",
        "http://localhost:8490/api/mcp",
    ] {
        let refusal = endpoint(url)
            .err()
            .ok_or(format!("{url} was accepted"))?
            .to_string();
        assert!(refusal.contains("https://"), "{url}: {refusal}");
    }
    Ok(())
}

/// An http URL on a numeric loopback address is accepted, as before R3.
#[test]
fn an_http_loopback_url_is_accepted_as_today() {
    for url in ["http://127.0.0.1:8490/api/mcp", "http://[::1]:8490/api/mcp"] {
        let checked = endpoint(url);
        assert!(
            checked.is_ok(),
            "{url} was refused: {:?}",
            checked.err().map(|refusal| refusal.to_string())
        );
    }
}

/// The URL's host, port and path are the ones posted to; the signature is made over
/// the router's path, `/api` stripped; https names port 443 when none is written and
/// is reached over TLS, http over loopback.
#[test]
fn the_endpoint_carries_the_port_path_and_signed_path() -> TestResult {
    let loopback = endpoint("http://127.0.0.1:8490/api/mcp")?;
    assert_eq!(
        (
            loopback.authority.port,
            loopback.path.as_str(),
            loopback.signed_path.as_str(),
            loopback.secure
        ),
        (8490, "/api/mcp", "/mcp", false)
    );
    let remote = endpoint("https://lys.example.test/api/mcp")?;
    assert_eq!(
        (
            remote.authority.host.as_str(),
            remote.authority.port,
            remote.path.as_str(),
            remote.signed_path.as_str(),
            remote.secure
        ),
        ("lys.example.test", 443, "/api/mcp", "/mcp", true)
    );
    let refusal = endpoint("ftp://lys.example.test/api/mcp")
        .err()
        .ok_or("an ftp URL was accepted")?
        .to_string();
    assert!(refusal.contains("mcp_url_invalid"), "{refusal}");
    let insecure = endpoint("http://lys.example.test/api/mcp")
        .err()
        .ok_or("an http URL off loopback was accepted")?
        .to_string();
    assert!(insecure.contains("mcp_url_insecure"), "{insecure}");
    Ok(())
}

#[path = "mcp_bridge_tls_tests.rs"]
mod tls;
