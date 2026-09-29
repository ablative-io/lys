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

#[cfg(unix)]
#[test]
fn server_error_is_uncertain_and_sent_once_without_retry() -> Result<(), Box<dyn std::error::Error>>
{
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;
    let temp = tempfile::tempdir()?;
    let key = temp.path().join("credential");
    std::fs::write(&key, format!("lys-registrar.{ACCOUNT}.{}", "ab".repeat(32)))?;
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
    let file = temp.path().join("import.json");
    std::fs::write(&file, r#"{"agents":[{"display_name":"fixture"}]}"#)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    std::fs::write(
        temp.path().join("identity.json"),
        serde_json::to_vec(&json!({"listen":address,"surface_dir":temp.path().join("surface")}))?,
    )?;
    let fixture = std::thread::spawn(move || -> Result<TcpListener, String> {
        let (mut socket, _) = listener.accept().map_err(|e| e.to_string())?;
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(|e| e.to_string())?;
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            socket.read_exact(&mut byte).map_err(|e| e.to_string())?;
            head.push(byte[0]);
        }
        let head = String::from_utf8(head).map_err(|e| e.to_string())?;
        assert!(head.starts_with("POST /api/identity/import HTTP/1.1"));
        let length = head
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_owned)
            })
            .ok_or("no length")?
            .trim()
            .parse::<usize>()
            .map_err(|e| e.to_string())?;
        let mut body = vec![0; length];
        socket.read_exact(&mut body).map_err(|e| e.to_string())?;
        let answer = r#"{"refusal":"ServiceAccountsUnavailable","reason":"fixture refusal","entry":"agents/fixture","completed":[]}"#;
        write!(socket,"HTTP/1.1 503 Service Unavailable\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",answer.len()).map_err(|e|e.to_string())?;
        Ok(listener)
    });
    let error = super::run(&file, Some(temp.path().to_owned()), Some(key), None, true).unwrap_err();
    assert_eq!(error.kind(), super::ErrorKind::ImportUncertain);
    assert!(error.to_string().contains("ServiceAccountsUnavailable"));
    assert!(error.to_string().contains("no retry was made"));
    let listener = fixture.join().map_err(|_panic| "fixture panicked")??;
    listener.set_nonblocking(true)?;
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn installation_preserves_existing_private_bearer_and_refuses_open_modes()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir()?;
    let layout = super::Layout::at(temp.path().to_owned());
    super::prepare_credential(&layout)?;
    let path = super::credential_path(&layout);
    let before = std::fs::read(&path)?;
    super::prepare_credential(&layout)?;
    assert_eq!(std::fs::read(&path)?, before);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
    assert!(super::prepare_credential(&layout).is_err());
    assert_eq!(std::fs::read(&path)?, before);
    Ok(())
}

#[test]
fn endpoint_follows_installed_surface_and_headless_layout_before_any_request()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let layout = super::Layout::at(temp.path().to_owned());
    for (surface, expected) in [
        (json!("/fixture/surface"), "/api/identity/import"),
        (json!(null), "/identity/import"),
    ] {
        std::fs::write(
            layout.service_config(),
            serde_json::to_vec(&json!({"listen":"127.0.0.1:17890", "surface_dir":surface}))?,
        )?;
        let (address, route) = super::endpoint(&layout, None)?;
        assert_eq!(address.port, 17890);
        assert_eq!(route, expected);
        assert_eq!(
            super::endpoint(&layout, Some("127.0.0.1:17891"))?.0.port,
            17891
        );
        assert!(super::endpoint(&layout, Some("192.0.2.1:17891")).is_err());
    }
    Ok(())
}
