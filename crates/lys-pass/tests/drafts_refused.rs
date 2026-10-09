#![cfg(test)]
//! A product's refusal on execution reaches Lys as its own name and reason
//! (ACCESS-001 R3): `{refusal, reason}`, any name the product gives, never a
//! permission refusal reshaped. A refusal without a name or a reason is not
//! sent.

use lys_pass::Target;
use lys_pass::drafts::{ApprovedDraft, Execution, request_digest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn approved() -> ApprovedDraft {
    ApprovedDraft {
        id: "draft".to_owned(),
        app: "sample".to_owned(),
        grant: "grant".to_owned(),
        target: Target {
            kind: "sample.file".to_owned(),
            id: "held".to_owned(),
            action: "write".to_owned(),
        },
        request_digest: request_digest("prepared"),
        words: "prepared".to_owned(),
    }
}

fn refused(refusal: &str, reason: &str) -> Execution {
    Execution::RefusedOnExecution {
        request_digest: request_digest("prepared"),
        refusal: refusal.to_owned(),
        reason: reason.to_owned(),
    }
}

/// Answers one request with 200 `{}` and returns its request line and body.
async fn one_request(
    listener: tokio::net::TcpListener,
) -> Result<(String, serde_json::Value), std::io::Error> {
    let invalid = |error: Box<dyn std::error::Error + Send + Sync>| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, error)
    };
    let (mut socket, _) = listener.accept().await?;
    let mut bytes = Vec::new();
    let (end, length) = loop {
        let mut buffer = [0; 1024];
        let count = socket.read(&mut buffer).await?;
        if count == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        bytes.extend_from_slice(&buffer[..count]);
        let Some(end) = bytes.windows(4).position(|value| value == b"\r\n\r\n") else {
            continue;
        };
        let headers = std::str::from_utf8(&bytes[..end]).map_err(|error| invalid(error.into()))?;
        let mut length = 0;
        for header in headers.lines() {
            if let Some((key, value)) = header.split_once(':')
                && key.eq_ignore_ascii_case("content-length")
            {
                length = value
                    .trim()
                    .parse()
                    .map_err(|error: std::num::ParseIntError| invalid(error.into()))?;
            }
        }
        if bytes.len() >= end + 4 + length {
            break (end, length);
        }
    };
    let head = String::from_utf8_lossy(&bytes[..end])
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    let body = serde_json::from_slice(&bytes[end + 4..end + 4 + length])
        .map_err(|error| invalid(error.into()))?;
    socket
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
        .await?;
    socket.shutdown().await?;
    Ok((head, body))
}

#[tokio::test]
async fn a_product_refusal_is_posted_as_its_own_name_and_reason()
-> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(one_request(listener));
    let client = lys_pass::Client::new(
        reqwest::Client::builder().no_proxy().build()?,
        url::Url::parse(&format!("http://{address}"))?,
    )?;
    let receipt = refused(
        "InvoiceClosed",
        "the invoice was closed before the draft was approved",
    );
    client
        .record_execution("connector-pass", &approved(), &receipt)
        .await?;
    let (head, body) = server.await??;
    assert!(
        head.starts_with("POST /product-drafts/draft/refused-on-execution "),
        "{head}"
    );
    assert_eq!(
        body,
        serde_json::json!({"refusal": "InvoiceClosed", "reason": "the invoice was closed before the draft was approved"})
    );
    Ok(())
}

#[tokio::test]
async fn a_refusal_without_a_name_or_a_reason_is_not_sent() -> Result<(), Box<dyn std::error::Error>>
{
    let client = lys_pass::Client::new(
        reqwest::Client::builder().no_proxy().build()?,
        url::Url::parse("http://127.0.0.1:9")?,
    )?;
    for receipt in [refused("", "a reason"), refused("InvoiceClosed", "")] {
        let answer = client
            .record_execution("connector-pass", &approved(), &receipt)
            .await;
        assert!(
            matches!(answer, Err(lys_pass::Error::Invalid(_))),
            "{answer:?}"
        );
    }
    Ok(())
}
