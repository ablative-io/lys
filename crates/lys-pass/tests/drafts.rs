#![cfg(test)]
//! A product receipt prevents a second execution after acknowledgment loss.

use lys_pass::Target;
use lys_pass::drafts::{ApprovedDraft, Execution, Executor, execute_once};

#[derive(Default)]
struct Product {
    receipt: Option<Execution>,
    writes: usize,
}

impl Executor for Product {
    type Error = std::io::Error;

    fn receipt(&self, draft: &str) -> Result<Option<Execution>, Self::Error> {
        assert_eq!(draft, "draft");
        Ok(self.receipt.clone())
    }

    fn execute_and_record(&mut self, draft: &ApprovedDraft) -> Result<Execution, Self::Error> {
        self.writes += 1;
        let receipt = Execution::Executed {
            request_digest: draft.request_digest.clone(),
            receipt_digest: lys_pass::drafts::request_digest("receipt"),
        };
        self.receipt = Some(receipt.clone());
        Ok(receipt)
    }
}

#[test]
fn retry_after_acknowledgment_loss_does_not_execute_again() -> Result<(), Box<dyn std::error::Error>>
{
    let draft = ApprovedDraft {
        id: "draft".to_owned(),
        app: "sample".to_owned(),
        grant: "grant".to_owned(),
        target: Target::new("sample.file", "held", "write")?,
        request_digest: lys_pass::drafts::request_digest("prepared"),
        words: "prepared".to_owned(),
    };
    let mut product = Product::default();
    let first = execute_once(&draft, &mut product)?;
    assert_eq!(execute_once(&draft, &mut product)?, first);
    assert_eq!(product.writes, 1);
    let mut changed = draft;
    changed.request_digest = "1".repeat(64);
    assert!(execute_once(&changed, &mut product).is_err());
    assert_eq!(product.writes, 1);
    Ok(())
}

#[tokio::test]
async fn runner_pulls_approved_drafts_and_records_once_after_receipt_retry()
-> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        let digest = lys_pass::drafts::request_digest("prepared");
        let mut requests = Vec::new();
        for path in [
            "/drafts?app=sample&state=approved",
            "/drafts/draft/executed",
            "/drafts?app=sample&state=approved",
            "/drafts/draft/executed",
        ] {
            let (mut socket, _) = listener.accept().await?;
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 1024];
                let count = socket.read(&mut buffer).await?;
                if count == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "draft request ended",
                    ));
                }
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|value| value == b"\r\n\r\n") {
                    let headers = std::str::from_utf8(&bytes[..end]).map_err(|error| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, error)
                    })?;
                    let mut length = 0;
                    for header in headers.lines() {
                        if let Some((key, value)) = header.split_once(':')
                            && key.eq_ignore_ascii_case("content-length")
                        {
                            length = value.trim().parse::<usize>().map_err(|error| {
                                std::io::Error::new(std::io::ErrorKind::InvalidData, error)
                            })?;
                        }
                    }
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let request = String::from_utf8(bytes)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            assert!(
                request
                    .lines()
                    .next()
                    .is_some_and(|line| line.contains(path))
            );
            let body = if path.contains('?') {
                serde_json::json!({"drafts":[{"id":"draft","app":"sample","grant":"grant","target":{"kind":"sample.file","id":"held","action":"write"},"request_digest":digest,"words":"prepared"}],"total":1,"next":null}).to_string()
            } else {
                "{}".to_owned()
            };
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await?;
            socket.shutdown().await?;
            requests.push(request);
        }
        Ok::<_, std::io::Error>(requests)
    });
    let client = lys_pass::Client::new(
        reqwest::Client::builder().no_proxy().build()?,
        url::Url::parse(&format!("http://{address}"))?,
    )?;
    let mut product = Product::default();
    for attempt in 0..2 {
        let report = client
            .run_approved("connector-pass", "sample", None, &mut product)
            .await?;
        assert_eq!(report.acknowledged, 1);
        assert_eq!(product.writes, 1, "attempt {attempt}");
    }
    let requests = server.await??;
    assert!(requests.iter().all(|request| {
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer connector-pass")
    }));
    assert!(requests[1].contains("receipt_digest"));
    Ok(())
}

#[tokio::test]
async fn ordinary_refusal_cannot_become_a_draft() -> Result<(), Box<dyn std::error::Error>> {
    let client = lys_pass::Client::new(
        reqwest::Client::builder().no_proxy().build()?,
        url::Url::parse("http://127.0.0.1:1")?,
    )?;
    let draft = lys_pass::drafts::DraftRequest {
        operation: "operation".to_owned(),
        grant: "grant".to_owned(),
        target: Target::new("sample.file", "held", "write")?,
        request_digest: lys_pass::drafts::request_digest("prepared"),
        words: "prepared".to_owned(),
    };
    let refused: lys_pass::deliberate::CheckAnswer =
        serde_json::from_str("{\"allowed\":false,\"refusal\":\"not_held\"}")?;
    let error = client
        .create_draft("caller-pass", &draft, &refused)
        .await
        .err()
        .ok_or("ordinary refusal made a draft")?;
    assert_eq!(error.name(), "contract_refused");
    Ok(())
}
