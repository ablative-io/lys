use std::io::{Read, Write};
use std::net::TcpListener;

use super::Server;
use crate::commands::sign_in::Pass;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_personal_401_preserves_the_servers_name_and_prints_its_exact_sign_in_command() -> Result {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let base = format!("http://{}/api", listener.local_addr()?);
    let served = std::thread::spawn(move || -> std::result::Result<String, std::io::Error> {
        let (mut stream, _) = listener.accept()?;
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let length = stream.read(&mut buffer)?;
            if length == 0 {
                return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
            }
            request.extend_from_slice(&buffer[..length]);
        }
        let body = r#"{"refusal":"pass_revoked","reason":"this terminal's pass was ended"}"#;
        write!(
            stream,
            "HTTP/1.1 401 Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )?;
        String::from_utf8(request)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    });
    let server = Server::personal(Pass {
        server: base.clone(),
        person: "person-0123456789abcdef0123456789abcdef".to_owned(),
        expires_at: u64::MAX,
        access_token: "fixture-person-access".to_owned(),
        refresh_token: "fixture-person-refresh".to_owned(),
    })?;
    let error = server
        .get("/me")
        .err()
        .ok_or("revoked pass accepted")?
        .to_string();
    assert!(error.starts_with("pass_revoked:"), "{error}");
    assert!(error.contains("this terminal's pass was ended"));
    assert!(error.contains(&format!("lys sign-in --server {base}")));
    assert!(!error.contains("fixture-person-access"));
    assert!(!error.contains("fixture-person-refresh"));
    let request = served.join().map_err(|_| "fixture thread panicked")??;
    assert!(request.starts_with("GET /api/me HTTP/1.1"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer fixture-person-access")
    );
    assert!(!request.to_ascii_lowercase().contains("lys-operator"));
    Ok(())
}
