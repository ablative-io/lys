use std::io::{Read, Write};
use std::net::TcpListener;

use super::Server;
use crate::commands::sign_in::Pass;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_personal_401_preserves_the_servers_name_and_prints_its_exact_sign_in_command() -> Result {
    personal_401(
        r#"{"refusal":"pass_revoked","reason":"this terminal's pass was ended"}"#,
        "pass_revoked",
        "this terminal's pass was ended",
    )
}

#[test]
fn a_non_json_401_still_names_sign_in_because_a_proxy_can_answer_before_lys() -> Result {
    personal_401("<html>Sign in</html>", "person_pass_expired", "HTTP 401")
}

fn personal_401(body: &'static str, name: &str, reason: &str) -> Result {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let base = format!("http://{}/api", listener.local_addr()?);
    let response = std::thread::spawn(move || -> std::result::Result<String, std::io::Error> {
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
    assert!(error.starts_with(&format!("{name}:")), "{error}");
    assert!(error.contains(reason));
    assert!(error.contains(&format!("lys sign-in --server {base}")));
    assert!(!error.contains("fixture-person-access"));
    assert!(!error.contains("fixture-person-refresh"));
    let request = response
        .join()
        .map_err(|panic| format!("fixture thread panicked: {panic:?}"))??;
    assert!(request.starts_with("GET /api/me HTTP/1.1"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer fixture-person-access")
    );
    assert!(!request.to_ascii_lowercase().contains("lys-operator"));
    Ok(())
}
