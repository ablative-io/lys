use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;

use super::*;
use crate::identity::rauthy::{Theme, ThemeCss};
use crate::identity::themes::rauthy_dark_defaults;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const STOP: &[u8] = b"STOP / HTTP/1.1\r\nContent-Length: 0\r\n\r\n";

fn read_request(stream: &mut TcpStream) -> TestResult<(String, Vec<u8>)> {
    let mut raw = Vec::new();
    let mut buffer = [0_u8; 4096];
    let head_end = loop {
        if let Some(at) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
            break at;
        }
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err("the request ended before its head".into());
        }
        raw.extend_from_slice(&buffer[..read]);
    };
    let head = String::from_utf8(raw[..head_end].to_vec())?;
    let length = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map_or(Ok(0), |(_, value)| value.trim().parse::<usize>())?;
    let mut body = raw[head_end + 4..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err("the request ended before its body".into());
        }
        body.extend_from_slice(&buffer[..read]);
    }
    let request_line = head.lines().next().unwrap_or_default().to_string();
    Ok((request_line, body))
}

/// A stand-in for Rauthy's theme endpoints: `POST` answers the stored theme,
/// `PUT` replaces it. Returns every request line it served once stopped.
fn fake_rauthy(stored: &Theme) -> TestResult<(String, JoinHandle<TestResult<Vec<String>>>)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}", listener.local_addr()?);
    let mut theme = serde_json::to_vec(stored)?;
    let handle = std::thread::spawn(move || -> TestResult<Vec<String>> {
        let mut seen = Vec::new();
        for stream in listener.incoming() {
            let mut stream = stream?;
            let (line, body) = read_request(&mut stream)?;
            if line.starts_with("STOP") {
                return Ok(seen);
            }
            let answer = if line.starts_with("PUT ") {
                theme = body;
                b"{}".to_vec()
            } else {
                theme.clone()
            };
            seen.push(line);
            stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                    answer.len()
                )
                .as_bytes(),
            )?;
            stream.write_all(&answer)?;
        }
        Ok(seen)
    });
    Ok((url, handle))
}

fn stop<T>(url: &str, handle: JoinHandle<TestResult<T>>) -> TestResult<T> {
    let mut stream = TcpStream::connect(url.trim_start_matches("http://"))?;
    stream.write_all(STOP)?;
    handle
        .join()
        .map_err(|panic| format!("the fake Rauthy panicked: {panic:?}"))?
}

fn theme_with_dark(client_id: &str, dark: ThemeCss) -> Theme {
    Theme {
        client_id: client_id.to_string(),
        light: rauthy_dark_defaults(),
        dark,
        border_radius: "5px".to_string(),
    }
}

fn app_client() -> Client {
    Client {
        id: "app".to_string(),
        name: "App".to_string(),
        redirect_uris: vec!["https://cambium.example/callback".to_string()],
        post_logout_redirect_uris: Vec::new(),
        token_alg: "EdDSA".to_string(),
        challenges: Vec::new(),
    }
}

#[test]
fn a_theme_that_fails_contrast_is_refused_before_it_is_written() -> TestResult {
    let mut dark = rauthy_dark_defaults();
    dark.text_high = dark.bg;
    let (url, handle) = fake_rauthy(&theme_with_dark("app", dark))?;
    let api = RauthyApi::new(&url, None)?;
    let mapping = ThemeMapping::declared()?;
    let outcome = reconcile_theme(&api, &mapping, ClientRole::App, &app_client());
    let seen = stop(&url, handle)?;
    let error = outcome.err().ok_or("a failing theme was accepted")?;
    assert_eq!(error.kind(), ErrorKind::ThemeInvalid);
    assert!(error.to_string().contains("text_high over ink"), "{error}");
    assert_eq!(seen, ["POST /auth/v1/theme/app HTTP/1.1"]);
    Ok(())
}

#[test]
fn a_theme_that_meets_contrast_is_written_and_read_back() -> TestResult {
    let (url, handle) = fake_rauthy(&theme_with_dark("app", rauthy_dark_defaults()))?;
    let api = RauthyApi::new(&url, None)?;
    let mapping = ThemeMapping::declared()?;
    let outcome = reconcile_theme(&api, &mapping, ClientRole::App, &app_client());
    let seen = stop(&url, handle)?;
    assert_eq!(outcome?.outcome, "applied");
    assert_eq!(
        seen,
        [
            "POST /auth/v1/theme/app HTTP/1.1",
            "PUT /auth/v1/theme/app HTTP/1.1",
            "POST /auth/v1/theme/app HTTP/1.1",
        ]
    );
    Ok(())
}

/// What the policy stand-in saw once stopped: every request line and the
/// last body it was sent.
type PolicySeen = TestResult<(Vec<String>, Vec<u8>)>;

/// A stand-in for Rauthy's password policy endpoint. It answers `status`;
/// with 200 and no `fixed` answer it stores the body and answers the policy
/// it stored as Rauthy does, the fields left absent left out. Returns every
/// request line and the last body it was sent, once stopped.
fn fake_policy(
    status: u16,
    fixed: Option<&'static str>,
) -> TestResult<(String, JoinHandle<PolicySeen>)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}", listener.local_addr()?);
    let handle = std::thread::spawn(move || -> PolicySeen {
        let (mut seen, mut sent) = (Vec::new(), Vec::new());
        for stream in listener.incoming() {
            let mut stream = stream?;
            let (line, body) = read_request(&mut stream)?;
            if line.starts_with("STOP") {
                break;
            }
            seen.push(line);
            sent.clone_from(&body);
            let answer = if let Some(fixed) = fixed {
                fixed.as_bytes().to_vec()
            } else {
                let mut stored: Map<String, Value> = serde_json::from_slice(&body)?;
                stored.retain(|_, value| !value.is_null());
                serde_json::to_vec(&stored)?
            };
            stream.write_all(
                format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                    answer.len()
                )
                .as_bytes(),
            )?;
            stream.write_all(&answer)?;
        }
        Ok((seen, sent))
    });
    Ok((url, handle))
}

/// Lys's configured policy, no default of Lys's or of Rauthy's.
fn lys_policy() -> PasswordPolicy {
    PasswordPolicy {
        length_min: 12,
        length_max: 48,
        lower_case: None,
        upper_case: Some(1),
        digits: Some(2),
        special: None,
        not_recently_used: Some(4),
    }
}

#[test]
fn lys_password_policy_is_written_whole_and_rauthy_stores_that_policy() -> TestResult {
    let (url, handle) = fake_policy(200, None)?;
    let api = RauthyApi::new(&url, None)?;
    let outcome = reconcile_password_policy(&api, &lys_policy());
    let (seen, sent) = stop(&url, handle)?;
    let operation = outcome?;
    assert_eq!(operation.kind, "password_policy");
    assert_eq!(operation.outcome, "applied");
    assert_eq!(seen, ["PUT /auth/v1/password_policy HTTP/1.1"]);
    let sent: Value = serde_json::from_slice(&sent)?;
    assert_eq!(
        sent,
        json!({
            "length_min": 12,
            "length_max": 48,
            "include_lower_case": null,
            "include_upper_case": 1,
            "include_digits": 2,
            "include_special": null,
            "valid_days": null,
            "not_recently_used": 4,
        }),
        "Rauthy is sent Lys's policy and nothing of its own"
    );
    Ok(())
}

#[test]
fn a_policy_stored_otherwise_than_written_is_a_read_back_mismatch() -> TestResult {
    let (url, handle) = fake_policy(200, Some(r#"{"length_min":8,"length_max":48}"#))?;
    let api = RauthyApi::new(&url, None)?;
    let outcome = reconcile_password_policy(&api, &lys_policy());
    stop(&url, handle)?;
    let error = outcome
        .err()
        .ok_or("a policy stored otherwise was accepted")?;
    assert_eq!(error.kind(), ErrorKind::ReadBackMismatch);
    Ok(())
}

#[test]
fn a_key_that_cannot_write_the_policy_is_refused_naming_the_right() -> TestResult {
    let forbidden = r#"{"error":"Forbidden","message":"missing access rights"}"#;
    let (url, handle) = fake_policy(403, Some(forbidden))?;
    let api = RauthyApi::new(&url, None)?;
    let outcome = reconcile_password_policy(&api, &lys_policy());
    let (seen, _) = stop(&url, handle)?;
    let error = outcome.err().ok_or("a forbidden write was accepted")?;
    assert_eq!(error.kind(), ErrorKind::RauthyForbidden);
    assert!(error.to_string().contains("Secrets update"), "{error}");
    assert_eq!(seen.len(), 1, "one write was asked, and nothing else");
    Ok(())
}
