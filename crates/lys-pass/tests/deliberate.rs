#![cfg(test)]
//! Deliberate routes never use cached rights to answer a live refusal.

use lys_pass::{Client, Target};
use reqwest::Client as Http;
use url::Url;

async fn serve(listener: tokio::net::TcpListener, body: &'static str) -> Result<String, std::io::Error> {
    serve_status(listener, body, "200 OK").await
}

async fn serve_status(listener: tokio::net::TcpListener, body: &'static str, status: &'static str) -> Result<String, std::io::Error> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (mut stream, _) = listener.accept().await?;
    let mut bytes = Vec::new();
    loop {
        let mut chunk = [0; 1024];
        let read = stream.read(&mut chunk).await?;
        if read == 0 { return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "request ended before body")); }
        bytes.extend_from_slice(&chunk[..read]);
        if let Some(split) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&bytes[..split]).map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            let mut length = None;
            for line in headers.lines() {
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("content-length") {
                    length = Some(value.trim().parse::<usize>().map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?);
                }
            }
            let length = length.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "request has no content length"))?;
            if bytes.len() >= split + 4 + length { break; }
        }
    }
    let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    String::from_utf8(bytes).map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

#[tokio::test]
async fn unreachable_lys_is_named_and_never_offline_allowed() -> Result<(), Box<dyn std::error::Error>> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    drop(listener);
    let client = Client::new(Http::builder().no_proxy().build()?, Url::parse(&format!("http://{address}"))?)?;
    let error = client.check_batch("pass", "holder", &[Target::new("sample.file", "child", "read")?], None).await.err().ok_or("unreachable issuer allowed")?;
    assert_eq!(error.name(), "lys_could_not_be_asked");
    Ok(())
}

#[tokio::test]
async fn live_batch_preserves_held_mode_holder_and_revision() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(serve(listener, "{\"revision\":9,\"results\":[{\"allowed\":false,\"mode\":\"by_two\",\"grant\":\"grant\"}]}"));
    let client = Client::new(Http::builder().no_proxy().build()?, Url::parse(&format!("http://{address}"))?)?;
    let answer = client.check_batch("caller-pass", "holder", &[Target::new("sample.file", "held", "write")?], Some(9)).await?;
    assert_eq!(answer.revision, 9);
    assert_eq!(answer.results[0].mode, Some(lys_pass::Mode::ByTwo));
    assert!(!answer.results[0].allowed);
    let request = server.await??;
    assert!(request.starts_with("POST /grants/check/batch "));
    assert!(request.to_ascii_lowercase().contains("authorization: bearer caller-pass"));
    let (_, body) = request.split_once("\r\n\r\n").ok_or("missing body")?;
    let body: serde_json::Value = serde_json::from_str(body)?;
    assert_eq!(body["checks"][0]["subject"], "holder");
    assert_eq!(body["at_least"], 9);
    Ok(())
}

#[tokio::test]
async fn incomplete_live_population_is_a_named_refusal() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(serve(listener, "{\"revision\":9,\"results\":[]}"));
    let client = Client::new(Http::builder().no_proxy().build()?, Url::parse(&format!("http://{address}"))?)?;
    let error = client.check_batch("pass", "holder", &[Target::new("sample.file", "child", "read")?], None).await.err().ok_or("incomplete batch accepted")?;
    assert_eq!(error.name(), "lys_could_not_be_asked");
    server.await??;
    Ok(())
}

#[tokio::test]
async fn token_fetch_and_refresh_send_their_distinct_grants() -> Result<(), Box<dyn std::error::Error>> {
    for refresh in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = tokio::spawn(serve(listener, "{\"access_token\":\"signed-pass\",\"token_type\":\"Bearer\",\"expires_in\":100,\"refresh_token\":\"refresh\"}"));
        let client = Client::new(Http::builder().no_proxy().build()?, Url::parse(&format!("http://{address}"))?)?;
        let answer = if refresh { client.refresh_token("sample", "synthetic", "refresh").await? }
        else { client.fetch_token(&lys_pass::TokenRequest { client_id: "sample", client_secret: "synthetic", code: "code", redirect_uri: "https://product.example/return", code_verifier: "verifier" }).await? };
        assert_eq!(answer.access_token, "signed-pass");
        let request = server.await??;
        assert!(request.starts_with("POST /oauth/token "));
        assert!(request.contains(if refresh { "grant_type=refresh_token" } else { "grant_type=authorization_code" }));
        assert_eq!(answer.refresh_token.as_deref(), Some("refresh"));
    }
    Ok(())
}

#[tokio::test]
async fn ended_holder_refresh_preserves_the_issuer_refusal_name() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(serve_status(listener, "{\"error\":\"invalid_grant\",\"error_description\":\"holder ended\",\"refusal\":\"HolderRetired\",\"reason\":\"holder ended\"}", "400 Bad Request"));
    let client = Client::new(Http::builder().no_proxy().build()?, Url::parse(&format!("http://{address}"))?)?;
    let error = client.refresh_token("sample", "synthetic", "refresh").await.err().ok_or("retired holder refresh accepted")?;
    assert_eq!(error.name(), "HolderRetired");
    server.await??;
    Ok(())
}
