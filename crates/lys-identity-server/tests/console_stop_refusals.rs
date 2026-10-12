#![cfg(test)]
//! The console stop's refusals that need only the service: every signature
//! that is not exactly one of the service's own over the exact body in the
//! console domain, a claim or reason of nothing, an oversized body with a
//! session and a run pass riding beside the signature, and a console signature offered
//! to the signed-in pull. Each is refused by name and pulls nothing. The
//! API description names the console signature and the shared body.
//!
//! The cases that stop a running session are in `stop_everything.rs`.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_runner::console_stop;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: format!("{subject}@example.test"),
    }
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

/// A service with its two people seeded and no runner: nothing here is
/// meant to reach a session.
struct Desk {
    service: Service,
}

impl Desk {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, _) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            None,
            None,
            |_| {},
            |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
        )
        .await?;
        Ok(Self { service })
    }

    fn request() -> Result<console_stop::Request, Box<dyn Error>> {
        Ok(console_stop::Request::new(
            OperationId::generate()?.to_string(),
            "an operator at this console".to_owned(),
            "end an unsafe run".to_owned(),
            true,
        ))
    }

    fn key(&self) -> Result<Ed25519Identity, Box<dyn Error>> {
        Ok(Ed25519Identity::load(
            &self.service.dir.path().join("service.key"),
        )?)
    }

    fn signed(&self, body: &[u8]) -> Result<String, Box<dyn Error>> {
        Ok(lys_runner::protocol::hex(
            &self.key()?.sign(&console_stop::signed_bytes(body)),
        ))
    }

    /// Post `body` to `path` carrying exactly `headers`, repeats kept.
    async fn send(
        &self,
        path: &str,
        body: Vec<u8>,
        headers: &[(&str, &str)],
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let mut request = reqwest::Client::new()
            .post(format!("{}{path}", self.service.base))
            .header("content-type", "application/json")
            .body(body);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let response = request.send().await?;
        let status = response.status().as_u16();
        Ok((status, serde_json::from_slice(&response.bytes().await?)?))
    }

    async fn console(
        &self,
        body: Vec<u8>,
        headers: &[(&str, &str)],
    ) -> Result<(u16, Value), Box<dyn Error>> {
        self.send(&console_stop::path(false), body, headers).await
    }

    async fn signed_console(
        &self,
        request: &console_stop::Request,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let body = serde_json::to_vec(request)?;
        let signature = self.signed(&body)?;
        self.console(body, &[(console_stop::SIGNATURE_HEADER, &signature)])
            .await
    }

    async fn untouched(&self) -> TestResult {
        let cookie = self.service.sign_in(login(ADMINISTRATOR)).await?;
        let (status, standing) = self
            .service
            .get("/runtime/stop-everything", Some(&cookie))
            .await?;
        assert_eq!(status, 200, "{standing}");
        assert!(standing["pulled"].is_null(), "{standing}");
        assert!(standing["last"].is_null(), "{standing}");
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_console_signatures_are_refused_without_a_pull() -> TestResult {
    let desk = Desk::set().await?;
    let body = serde_json::to_vec(&Desk::request()?)?;
    let signature = desk.signed(&body)?;
    refused(
        &desk
            .console(
                body,
                &[
                    (console_stop::SIGNATURE_HEADER, &signature),
                    (console_stop::SIGNATURE_HEADER, &signature),
                ],
            )
            .await?,
        401,
        "console_signature_refused",
    );
    desk.untouched().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_services_key_signing_another_domain_is_refused_without_a_pull() -> TestResult {
    let desk = Desk::set().await?;
    let body = serde_json::to_vec(&Desk::request()?)?;
    let mut other = b"lys-identity/another-act/v1\n".to_vec();
    other.extend_from_slice(&body);
    let signature = lys_runner::protocol::hex(&desk.key()?.sign(&other));
    refused(
        &desk
            .console(body, &[(console_stop::SIGNATURE_HEADER, &signature)])
            .await?,
        401,
        "console_signature_refused",
    );
    desk.untouched().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_signed_console_claim_of_nothing_is_refused_without_a_pull() -> TestResult {
    let desk = Desk::set().await?;
    let mut request = Desk::request()?;
    request.by = "   ".to_owned();
    refused(
        &desk.signed_console(&request).await?,
        400,
        "RequestMalformed",
    );
    desk.untouched().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_signed_console_reason_of_nothing_is_refused_without_a_pull() -> TestResult {
    let desk = Desk::set().await?;
    let mut request = Desk::request()?;
    request.reason = "   ".to_owned();
    refused(
        &desk.signed_console(&request).await?,
        400,
        "RequestMalformed",
    );
    desk.untouched().await
}

/// An oversized body that carries a session and a run pass beside the console
/// signature is refused on its headers alone: a run pass rides with no other
/// credential, so no credential is judged in full and the body is never kept.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cookie_and_a_run_pass_beside_a_console_signature_are_refused_before_the_body()
-> TestResult {
    let desk = Desk::set().await?;
    let cookie = desk.service.sign_in(login(ADMINISTRATOR)).await?;
    let body = vec![b' '; 2 * 1024 * 1024 + 1];
    let signature = desk.signed(&body)?;
    refused(
        &desk
            .console(
                body,
                &[
                    (console_stop::SIGNATURE_HEADER, &signature),
                    ("cookie", &cookie),
                    (lys_identity_server::agent_pass::HEADER, "unverified-pass"),
                ],
            )
            .await?,
        401,
        "AgentPassRefused",
    );
    desk.untouched().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_console_signature_does_not_admit_the_signed_in_pull() -> TestResult {
    let desk = Desk::set().await?;
    let body = serde_json::to_vec(&json!({
        "operation": OperationId::generate()?.to_string(),
        "reason": "not the console route",
    }))?;
    let signature = desk.signed(&body)?;
    refused(
        &desk
            .send(
                "/runtime/stop-everything",
                body,
                &[(console_stop::SIGNATURE_HEADER, &signature)],
            )
            .await?,
        401,
        "NotSignedIn",
    );
    desk.untouched().await
}

#[test]
fn the_api_description_names_the_console_signature_and_the_shared_body() -> TestResult {
    let document = lys_identity_server::openapi::document()?;
    let route = &document["paths"][console_stop::PATH]["post"];
    assert_eq!(
        route["security"],
        json!([{ "console_signature": [] }]),
        "{route}"
    );
    assert_eq!(
        route["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ConsoleStopBody",
        "{route}"
    );
    let scheme = &document["components"]["securitySchemes"]["console_signature"];
    assert_eq!(scheme["name"], console_stop::SIGNATURE_HEADER, "{scheme}");
    assert_eq!(
        document["components"]["schemas"]["ConsoleStopBody"]["additionalProperties"],
        false
    );
    Ok(())
}
