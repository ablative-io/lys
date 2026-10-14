//! Native sign-in refusals retain the provider reason and the correct origin.
use super::{Outcome, joined, pairs, serve};
use lys::commands::sign_in::{pass_file, sign_in_at};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;

#[test]
fn an_exchange_refusal_names_the_servers_reason_without_echoing_the_authorization_answer() -> Outcome
{
    let dir = tempfile::tempdir()?;
    let (base, worker) = serve(2, |request, origin| {
        if request.path.starts_with("/.well-known/") {
            Ok((
                200,
                json!({"issuer":origin, "authorization_endpoint":format!("{origin}/oauth/mcp/authorize"),"token_endpoint":format!("{origin}/oauth/mcp/token")}),
            ))
        } else {
            Ok((
                400,
                json!({"error":"invalid_grant", "error_description":"approval expired: fixture-private-answer; start sign-in again"}),
            ))
        }
    })?;
    let mut callback = None;
    let result = sign_in_at(&format!("{base}/api"), dir.path(), |authorize| {
        let query = pairs(
            authorize
                .split_once('?')
                .ok_or_else(|| lys::commands::seat_client::refused("fixture_invalid", "no query"))?
                .1,
        );
        let redirect = query
            .get("redirect_uri")
            .ok_or_else(|| lys::commands::seat_client::refused("fixture_invalid", "no redirect"))?
            .clone();
        let state = query
            .get("state")
            .ok_or_else(|| lys::commands::seat_client::refused("fixture_invalid", "no state"))?
            .clone();
        callback = Some(thread::spawn(move || {
            let rest = redirect
                .strip_prefix("http://")
                .ok_or("callback is not loopback HTTP")?;
            let (host, path) = rest.split_once('/').ok_or("callback path absent")?;
            let mut stream = TcpStream::connect(host).map_err(|error| error.to_string())?;
            write!(stream, "GET /{path}?state={state}&code=fixture-private-answer HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").map_err(|error| error.to_string())?;
            let mut response = Vec::new();
            stream
                .read_to_end(&mut response)
                .map_err(|error| error.to_string())?;
            Ok::<_, String>(())
        }));
        Ok(())
    });
    joined(worker)?;
    joined(callback.ok_or("callback never started")?)?;
    let error = result.err().ok_or("refused exchange accepted")?.to_string();
    assert!(
        error.contains("approval expired"),
        "server reason absent: {error}"
    );
    assert!(error.contains("invalid_grant"));
    assert!(!error.contains("fixture-private-answer"));
    assert!(!pass_file(dir.path()).exists());
    Ok(())
}

#[test]
fn a_loopback_alias_names_the_advertised_sign_in_origin_instead_of_a_generic_refusal() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (base, worker) = serve(1, |_, _| {
        Ok((
            200,
            json!({"issuer":"https://sign-in.example.test", "authorization_endpoint":"https://sign-in.example.test/oauth/mcp/authorize", "token_endpoint":"https://sign-in.example.test/oauth/mcp/token"}),
        ))
    })?;
    let error = sign_in_at(&format!("{base}/api"), dir.path(), |_| {
        Err(lys::commands::seat_client::refused(
            "fixture_invalid",
            "the wrong origin opened a browser",
        ))
    })
    .err()
    .ok_or("foreign origin accepted")?
    .to_string();
    joined(worker)?;
    assert!(
        error.contains("lys sign-in --server https://sign-in.example.test/api"),
        "correct origin absent: {error}"
    );
    assert!(!pass_file(dir.path()).exists());
    Ok(())
}
