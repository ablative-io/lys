#![cfg(test)]
//! One origin reaches provider and API routes; unexpected token refusals retain status without credentials.

use lys_pass::{Client, Target};
use reqwest::Client as Http;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn answer(
    listener: tokio::net::TcpListener,
    replies: Vec<(&'static str, &'static str)>,
) -> Result<Vec<String>, std::io::Error> {
    let mut requests = Vec::new();
    for (status, body) in replies {
        let (mut stream, _) = listener.accept().await?;
        let mut bytes = Vec::new();
        loop {
            let mut chunk = [0; 1024];
            let count = stream.read(&mut chunk).await?;
            if count == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "incomplete fixture request",
                ));
            }
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(split) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&bytes[..split])
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
                let length = headers
                    .lines()
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .map(|(_, value)| value.trim().parse::<usize>())
                    .transpose()
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?
                    .unwrap_or(0);
                if bytes.len() >= split + 4 + length {
                    requests.push(
                        headers
                            .lines()
                            .next()
                            .ok_or_else(|| {
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "missing request line",
                                )
                            })?
                            .to_owned(),
                    );
                    break;
                }
            }
        }
        stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await?;
        stream.shutdown().await?;
    }
    Ok(requests)
}

fn client(origin: &str) -> Result<Client, Box<dyn std::error::Error>> {
    Ok(Client::new(
        Http::builder().no_proxy().build()?,
        Url::parse(origin)?,
    )?)
}

#[test]
fn an_api_mount_is_refused_as_an_origin_by_name() -> TestResult {
    for path in ["api", "api/", "prefix/api/", "other/"] {
        let error = Client::new(
            Http::builder().no_proxy().build()?,
            Url::parse(&format!("https://lys.example/{path}"))?,
        )
        .err()
        .ok_or("a mounted origin was accepted")?;
        assert_eq!(error.name(), "lys_origin_required");
    }
    Ok(())
}

#[tokio::test]
async fn one_client_uses_root_provider_routes_and_mounted_api_routes() -> TestResult {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let client = client(&format!("http://{}", listener.local_addr()?))?;
    let serving = tokio::spawn(answer(
        listener,
        vec![
            (
                "200 OK",
                "{\"access_token\":\"signed-pass\",\"token_type\":\"Bearer\",\"expires_in\":100}",
            ),
            ("200 OK", include_str!("../fixtures/passes/keys.json")),
            (
                "200 OK",
                "{\"revision\":9,\"results\":[{\"allowed\":true,\"grant\":\"grant-one\",\"mode\":\"outright\"}]}",
            ),
            ("200 OK", "{\"drafts\":[],\"next\":null,\"total\":0}"),
        ],
    ));
    client
        .refresh_token("sample", "syntheticsecret", "synthetic-refresh")
        .await?;
    client.fetch_keys().await?;
    client
        .check_batch(
            "pass",
            "holder",
            &[Target::new("sample.file", "one", "read")?],
            None,
        )
        .await?;
    client.approved_drafts("connector", "sample", None).await?;
    assert_eq!(
        serving.await??,
        [
            "POST /oauth/token HTTP/1.1",
            "GET /oauth/jwks HTTP/1.1",
            "POST /api/grants/check/batch HTTP/1.1",
            "GET /api/product-drafts?app=sample&state=approved HTTP/1.1",
        ]
    );
    Ok(())
}

async fn token_refusal(body: &'static str, expected: &str) -> TestResult {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let client = client(&format!("http://{}", listener.local_addr()?))?;
    let serving = tokio::spawn(answer(listener, vec![("404 Not Found", body)]));
    let error = client
        .refresh_token("sample", "syntheticsecret", "synthetic-refresh")
        .await
        .err()
        .ok_or("a route refusal was accepted")?;
    assert_eq!(error.name(), "lys_could_not_be_asked");
    assert_eq!(error.to_string(), expected);
    for credential in ["syntheticsecret", "synthetic-refresh"] {
        assert!(!error.to_string().contains(credential));
        assert!(!format!("{error:?}").contains(credential));
    }
    assert_eq!(serving.await??, ["POST /oauth/token HTTP/1.1"]);
    Ok(())
}

#[tokio::test]
async fn the_wrong_token_route_names_404_and_not_an_api_route() -> TestResult {
    token_refusal(
        "{\"error\":\"/api/oauth/token is not an API route\",\"refusal\":\"NotAnApiRoute\"}",
        "Lys could not be asked: token route answered 404 NotAnApiRoute",
    )
    .await
}

#[tokio::test]
async fn a_non_json_token_refusal_names_the_status_without_its_body() -> TestResult {
    token_refusal(
        "upstream unavailable: synthetic-secret synthetic-refresh",
        "Lys could not be asked: token route answered 404",
    )
    .await
}

#[tokio::test]
async fn a_refusal_field_that_is_not_a_name_is_never_echoed() -> TestResult {
    token_refusal(
        "{\"refusal\":\"synthetic-secret\"}",
        "Lys could not be asked: token route answered 404",
    )
    .await
}

#[tokio::test]
async fn a_credential_echo_in_a_plain_refusal_name_is_never_returned() -> TestResult {
    token_refusal(
        "{\"refusal\":\"syntheticsecret\"}",
        "Lys could not be asked: token route answered 404",
    )
    .await
}

#[tokio::test]
async fn a_token_refusal_echoing_the_credential_is_not_a_safe_contract_answer() -> TestResult {
    token_refusal(
        "{\"error\":\"invalid_grant\",\"error_description\":\"syntheticsecret was refused\",\"refusal\":\"credential_refused\",\"reason\":\"syntheticsecret was refused\"}",
        "Lys could not be asked: token route answered 404 credential_refused",
    ).await
}
