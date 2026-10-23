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
use lys_identity::{IdentityId, LifecycleState, OperationId, PersonId};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// A server with one signed-in person, its certificate log open.
struct Table {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    cookie: String,
    router: axum::Router,
    person: PersonId,
}

impl Table {
    async fn new() -> TestResult<Self> {
        let (dir, mut state, actor, person, _own, _other) =
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
            person,
        })
    }

    /// Holds `GET /pair/{wait}` open on its own task, answering what it
    /// answered.
    fn hold(&self, wait: &str) -> tokio::task::JoinHandle<HeldAnswer> {
        let router = self.router.clone();
        let path = format!("/pair/{wait}");
        tokio::spawn(async move {
            let request = Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::empty())?;
            let response = router.oneshot(request).await?;
            let status = response.status().as_u16();
            let bytes = to_bytes(response.into_body(), usize::MAX).await?;
            Ok((status, serde_json::from_slice(&bytes)?))
        })
    }

    /// Waits for the held `GET /pair/{wait}` to subscribe: the server's own
    /// event, no clock.
    async fn subscribed(&self) -> TestResult {
        while self
            .state
            .pairings
            .lock()
            .map_err(|e| e.to_string())?
            .waiting()
            == 0
        {
            tokio::task::yield_now().await;
        }
        Ok(())
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

    /// Sends `body` to `path`, signed in as the person when `signed_in`.
    async fn send(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        signed_in: bool,
    ) -> TestResult<(u16, Value)> {
        // The four words carry spaces; a URI carries them percent-encoded.
        let mut request = Request::builder()
            .method(method)
            .uri(path.replace(' ', "%20"));
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
}

type HeldAnswer = Result<(u16, Value), Box<dyn std::error::Error + Send + Sync>>;

fn op() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

/// The DER of a `CERTIFICATE` PEM as the store frames it.
fn der_of(pem: &str) -> TestResult<Vec<u8>> {
    let body: String = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();
    Ok(STANDARD.decode(body)?)
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

#[tokio::test]
async fn pair_approve_registers_under_the_person_certifies_the_key_and_answers_the_wait()
-> TestResult {
    let table = Table::new().await?;
    let (code, wait, key) = table.ask("Waffles").await?;
    let body = json!({"operation": op()?, "activate": op()?, "certificate": op()?,
        "display_name": "Waffles the Terrible", "grants": []});
    // Nobody signed in: refused on the lookup, the approval and the decline.
    for (method, path, body) in [
        ("GET", format!("/pair/{code}/waiting"), None),
        ("POST", format!("/pair/{code}/approve"), Some(&body)),
        ("POST", format!("/pair/{code}/decline"), None),
    ] {
        let (status, answer) = table.send(method, &path, body, false).await?;
        assert_eq!(status, 401, "{method} {path}: {answer}");
    }
    let (status, waiting) = table
        .send("GET", &format!("/pair/{code}/waiting"), None, true)
        .await?;
    assert_eq!(status, 200, "{waiting}");
    assert_eq!(waiting["code"], code, "{waiting}");
    assert_eq!(waiting["name"], "Waffles", "{waiting}");
    // The words typed with capitals and extra spaces still find it.
    let typed = format!("  {}  ", code.to_uppercase()).replace(' ', "  ");
    let (status, found) = table
        .send(
            "GET",
            &format!("/pair/{}/waiting", typed.replace(' ', "%20")),
            None,
            true,
        )
        .await?;
    assert_eq!(status, 200, "{found}");
    let held = table.hold(&wait);
    table.subscribed().await?;
    let (status, approved) = table
        .send("POST", &format!("/pair/{code}/approve"), Some(&body), true)
        .await?;
    assert_eq!(status, 200, "{approved}");
    let agent = approved["agent"].as_str().ok_or("no agent")?.to_owned();
    let serial = approved["serial"].as_str().ok_or("no serial")?.to_owned();
    assert_eq!(serial, body["certificate"], "{approved}");
    let (status, answer) = held.await?.map_err(|error| error.to_string())?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["agent"], agent, "{answer}");
    assert_eq!(answer["serial"], serial, "{answer}");
    assert_eq!(answer["handle"], "Waffles the Terrible", "{answer}");
    assert!(
        answer["person"]
            .as_str()
            .is_some_and(|name| !name.is_empty()),
        "{answer}"
    );
    assert!(
        answer["lys"]
            .as_str()
            .is_some_and(|lys| lys.ends_with("/api")),
        "{answer}"
    );
    assert_eq!(answer["apps"], json!([]), "{answer}");
    // The certificate is over the key the agent made.
    let pem = answer["certificate_pem"].as_str().ok_or("no pem")?;
    let der = der_of(pem)?;
    assert_eq!(
        lys_core::ca::certificate_subject_public_key(&der)?,
        key.public_key_bytes()
    );
    // Its serial is in the log.
    let (status, certificates) = table
        .send("GET", &format!("/agents/{agent}/certificates"), None, true)
        .await?;
    assert_eq!(status, 200, "{certificates}");
    assert!(
        certificates["certificates"]
            .as_array()
            .is_some_and(|all| all.iter().any(|one| one["serial"] == serial)),
        "{certificates}"
    );
    // Registered under the signed-in person, and active.
    let agent_id: lys_identity::AgentId = agent.parse()?;
    let person = table.person;
    crate::routes::with_directory(&table.state, |directory| {
        let record = directory
            .projection()?
            .record(IdentityId::Agent(agent_id))
            .ok_or(crate::error::ServerError::AgentNotVisible)?;
        assert_eq!(record.responsible(), Some(person));
        assert_eq!(record.state(), LifecycleState::Active);
        Ok(())
    })?;
    // Decided, the words find nothing.
    let (status, gone) = table
        .send("GET", &format!("/pair/{code}/waiting"), None, true)
        .await?;
    assert_eq!(status, 404, "{gone}");
    assert_eq!(gone["refusal"], "pair_unknown", "{gone}");
    Ok(())
}

#[tokio::test]
async fn pair_decline_answers_pair_declined_and_keeps_nothing_of_the_key() -> TestResult {
    let table = Table::new().await?;
    let (code, wait, _) = table.ask("Waffles").await?;
    let held = table.hold(&wait);
    table.subscribed().await?;
    let (status, declined) = table
        .send("POST", &format!("/pair/{code}/decline"), None, true)
        .await?;
    assert_eq!(status, 200, "{declined}");
    assert_eq!(declined["decision"], "declined", "{declined}");
    assert_eq!(declined["code"], code, "{declined}");
    let answered = held.await?.map_err(|error| error.to_string())?;
    refused(&answered, 409, "pair_declined");
    assert!(
        table
            .state
            .pairings
            .lock()
            .map_err(|e| e.to_string())?
            .undecided()
            .is_empty()
    );
    // Declined twice is unknown: nothing waits with these words.
    let again = table
        .send("POST", &format!("/pair/{code}/decline"), None, true)
        .await?;
    refused(&again, 404, "pair_unknown");
    // An unknown code is refused by name on approve too.
    let body = json!({"operation": op()?, "activate": op()?, "certificate": op()?,
        "display_name": "Nobody", "grants": []});
    let unknown = table
        .send(
            "POST",
            "/pair/no%20such%20words%20here/approve",
            Some(&body),
            true,
        )
        .await?;
    refused(&unknown, 404, "pair_unknown");
    Ok(())
}

/// AGENTS-020 R3, against the fixture service with apps: the answer lists the
/// approved apps with their addresses and not the pending one; the serial is
/// in the certificate log, proven against its root.
#[tokio::test]
async fn pair_answer_lists_the_approved_apps_with_addresses_and_not_the_pending_one() -> TestResult
{
    use identity_contract::apps::{FILES, NOTES, login, register, registered, seeded};
    use identity_contract::harness::ADMINISTRATOR;
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    registered(&service, &admin, FILES).await?;
    register(&service, &admin, "fixture_pending").await?;
    let key = Arc::new(Ed25519Identity::ephemeral());
    let request = STANDARD.encode(create_certificate_request(&key, "Pancake")?);
    let (status, asked) = service
        .post(
            "/pair",
            None,
            &json!({"request": request, "name": "Pancake"}),
        )
        .await?;
    assert_eq!(status, 200, "{asked}");
    let code = asked["code"].as_str().ok_or("no code")?.to_owned();
    let wait = asked["wait"].as_str().ok_or("no wait")?.to_owned();
    let base = service.base.clone();
    let held = tokio::spawn(async move {
        let response = reqwest::get(format!("{base}/pair/{wait}")).await?;
        let status = response.status().as_u16();
        let answer: Value = response.json().await?;
        Ok::<(u16, Value), reqwest::Error>((status, answer))
    });
    let body = json!({"operation": op()?, "activate": op()?, "certificate": op()?,
        "display_name": "Pancake", "grants": []});
    let (status, approved) = service
        .post(&format!("/pair/{code}/approve"), Some(&admin), &body)
        .await?;
    assert_eq!(status, 200, "{approved}");
    assert_eq!(approved["apps"], 2, "{approved}");
    let (status, answer) = held.await??;
    assert_eq!(status, 200, "{answer}");
    let apps = answer["apps"].as_array().ok_or("no apps")?;
    let ids: Vec<&str> = apps.iter().filter_map(|app| app["id"].as_str()).collect();
    assert_eq!(ids, [NOTES, FILES], "{answer}");
    for app in apps {
        assert_eq!(app["address"], "https://app.example.test", "{app}");
        assert!(
            app["name"].as_str().is_some_and(|name| !name.is_empty()),
            "{app}"
        );
    }
    assert!(!ids.contains(&"fixture_pending"), "{answer}");
    let pem = answer["certificate_pem"].as_str().ok_or("no pem")?;
    assert_eq!(
        lys_core::ca::certificate_subject_public_key(&der_of(pem)?)?,
        key.public_key_bytes()
    );
    let agent = answer["agent"].as_str().ok_or("no agent")?;
    let serial = answer["serial"].as_str().ok_or("no serial")?;
    let (status, certificates) = service
        .get(&format!("/agents/{agent}/certificates"), Some(&admin))
        .await?;
    assert_eq!(status, 200, "{certificates}");
    let entered = certificates["certificates"]
        .as_array()
        .and_then(|all| all.iter().find(|one| one["serial"] == serial))
        .ok_or_else(|| format!("serial {serial} is not in the log: {certificates}"))?;
    // Entered in the log: a leaf under its tree size, with the root the proof is against
    // (a one-leaf log's inclusion proof is empty, and still verifies).
    let entry = &entered["entry"];
    let (leaf, size) = (entry["leaf"].as_u64(), entry["tree_size"].as_u64());
    assert!(
        leaf.zip(size).is_some_and(|(leaf, size)| leaf < size)
            && entry["root"].as_str().is_some_and(|root| !root.is_empty())
            && entry["proof"].is_string(),
        "the entry is not in the log: {entered}"
    );
    Ok(())
}
