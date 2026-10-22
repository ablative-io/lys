#![cfg(test)]

//! AGENTS-020: an agent with no identity asks for a pairing code and holds
//! its wait open until the person decides; the person's side registers,
//! certifies and answers it.

use std::sync::{Arc, Mutex};

use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use lys_core::Ed25519Identity;
use lys_core::ca::create_certificate_request;
use lys_core::pairing_words::WORDS;
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::pair_store::{Answer, Decision, Reach};
use crate::routes::AppState;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// A server with one signed-in person, its certificate log open.
struct Table {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    cookie: String,
    router: axum::Router,
}

impl Table {
    async fn new() -> TestResult<Self> {
        let (dir, mut state, actor, _person, _own, _other) =
            crate::grant_settlement_tests::person_fixture().await?;
        let key = Arc::new(Ed25519Identity::load(&state.grant_setup.key_file)?);
        state.certificates = Some(Mutex::new(
            crate::certificates_store::CertificateStore::open(
                &dir.path().join("certificates"),
                Arc::clone(&key),
            )?,
        ));
        state.policies = Some(Mutex::new(crate::agent_policy_store::PolicyStore::open(
            &dir.path().join("policies"),
            key,
        )?));
        let cookie = state.sessions.begin(actor)?;
        let state = Arc::new(state);
        let router = crate::routes_table::router(Arc::clone(&state))
            .merge(crate::certificates_api::routes().with_state(Arc::clone(&state)));
        Ok(Self {
            _dir: dir,
            state,
            cookie,
            router,
        })
    }

    /// Sends `body` to `path`, signed in as the person when `signed_in`.
    async fn send(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        signed_in: bool,
    ) -> TestResult<(u16, Value)> {
        let mut request = Request::builder().method(method).uri(path);
        if signed_in {
            request = request.header(header::COOKIE, self.cookie.clone());
        }
        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(body)?))?,
            None => request.body(Body::empty())?,
        };
        let response = self.router.clone().oneshot(request).await?;
        let status = response.status().as_u16();
        let bytes = to_bytes(response.into_body(), usize::MAX).await?;
        let answer = serde_json::from_slice(&bytes)
            .map_err(|error| format!("answer at {path}, status {status}: {error}"))?;
        Ok((status, answer))
    }

    /// Asks for a pairing over a fresh key, answering the code and wait id.
    async fn ask(&self, name: &str) -> TestResult<(String, String, Arc<Ed25519Identity>)> {
        let key = Arc::new(Ed25519Identity::ephemeral());
        let request = STANDARD.encode(create_certificate_request(&key, name)?);
        let (status, asked) = self
            .send(
                "POST",
                "/pair",
                Some(&json!({"request": request, "name": name})),
                false,
            )
            .await?;
        assert_eq!(status, 200, "{asked}");
        let code = asked["code"].as_str().ok_or("no code")?.to_owned();
        let wait = asked["wait"].as_str().ok_or("no wait")?.to_owned();
        Ok((code, wait, key))
    }
}

fn refused((status, answer): &(u16, Value), expected: u16, name: &str) {
    assert_eq!(*status, expected, "{answer}");
    assert_eq!(answer["refusal"], name, "{answer}");
}

#[tokio::test]
async fn pair_asks_answer_four_dictionary_words_a_wait_and_the_address() -> TestResult {
    let table = Table::new().await?;
    let key = Arc::new(Ed25519Identity::ephemeral());
    let request = STANDARD.encode(create_certificate_request(&key, "Waffles")?);
    let (status, asked) = table
        .send(
            "POST",
            "/pair",
            Some(&json!({"request": request, "name": "Waffles"})),
            false,
        )
        .await?;
    assert_eq!(status, 200, "{asked}");
    let code = asked["code"].as_str().ok_or("no code")?;
    let words: Vec<&str> = code.split(' ').collect();
    assert_eq!(words.len(), 4, "{code}");
    for word in &words {
        assert!(WORDS.contains(word), "{word} is not a dictionary word");
        assert!(word.bytes().all(|byte| byte.is_ascii_lowercase()), "{word}");
    }
    assert!(!asked["wait"].as_str().unwrap_or_default().is_empty());
    let address = asked["address"].as_str().ok_or("no address")?;
    assert!(
        address.starts_with("http") && address.ends_with("/pair"),
        "{address}"
    );
    let (second, _, _) = table.ask("Waffles").await?;
    assert_ne!(second, code, "the same code was given twice");
    Ok(())
}

#[tokio::test]
async fn pair_wait_holds_until_the_decision_is_written_then_answers_it() -> TestResult {
    let table = Table::new().await?;
    let (code, wait, _) = table.ask("Waffles").await?;
    let router = table.router.clone();
    let path = format!("/pair/{wait}");
    let held = tokio::spawn(async move {
        let request = Request::builder()
            .method("GET")
            .uri(path)
            .body(Body::empty())?;
        let response = router.oneshot(request).await?;
        let status = response.status().as_u16();
        let bytes = to_bytes(response.into_body(), usize::MAX).await?;
        Ok::<(u16, Value), Box<dyn std::error::Error + Send + Sync>>((
            status,
            serde_json::from_slice(&bytes)?,
        ))
    });
    // The server's own event: the wait has subscribed to the decision.
    while table
        .state
        .pairings
        .lock()
        .map_err(|e| e.to_string())?
        .waiting()
        == 0
    {
        tokio::task::yield_now().await;
    }
    assert!(!held.is_finished(), "the wait answered before any decision");
    table
        .state
        .pairings
        .lock()
        .map_err(|e| e.to_string())?
        .decide(&code, Decision::Declined)?;
    let answered = held.await?.map_err(|error| error.to_string())?;
    refused(&answered, 409, "pair_declined");
    // Answered once, the wait is gone.
    let again = table
        .send("GET", &format!("/pair/{wait}"), None, false)
        .await?;
    refused(&again, 404, "pair_unknown");

    let (code, wait, _) = table.ask("Pancake").await?;
    let answer = Answer {
        agent: "agent-1".to_owned(),
        handle: "pancake".to_owned(),
        certificate_pem: "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n"
            .to_owned(),
        serial: "serial-1".to_owned(),
        lys: "http://lys.test/api".to_owned(),
        person: "tom".to_owned(),
        apps: vec![Reach {
            id: "app-1".to_owned(),
            name: "Fixture product".to_owned(),
            address: "https://fixture.test".to_owned(),
        }],
    };
    table
        .state
        .pairings
        .lock()
        .map_err(|e| e.to_string())?
        .decide(
            &code,
            Decision::Approved {
                answer: Box::new(answer.clone()),
            },
        )?;
    let (status, got) = table
        .send("GET", &format!("/pair/{wait}"), None, false)
        .await?;
    assert_eq!(status, 200, "{got}");
    assert_eq!(got, serde_json::to_value(&answer)?);
    Ok(())
}

#[tokio::test]
async fn pair_unknown_wait_is_refused_at_once() -> TestResult {
    let table = Table::new().await?;
    let answer = table.send("GET", "/pair/no-such-wait", None, false).await?;
    refused(&answer, 404, "pair_unknown");
    Ok(())
}

#[tokio::test]
async fn pair_request_that_is_not_pkcs10_over_ed25519_is_refused_before_anything_is_kept()
-> TestResult {
    let table = Table::new().await?;
    for request in ["invalid", &STANDARD.encode(b"not a request")] {
        let answer = table
            .send(
                "POST",
                "/pair",
                Some(&json!({"request": request, "name": "Waffles"})),
                false,
            )
            .await?;
        refused(&answer, 400, "pair_request_invalid");
    }
    assert!(
        table
            .state
            .pairings
            .lock()
            .map_err(|e| e.to_string())?
            .undecided()
            .is_empty()
    );
    Ok(())
}
