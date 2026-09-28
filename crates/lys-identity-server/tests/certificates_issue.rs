//! Issuing and withdrawing a certificate: it is issued over the key the
//! agent's request proves, carries the claims the service keeps and none
//! the request names, verifies against the service's key, is entered once
//! however often it is sent, and is withdrawn beside its issuance.

use std::error::Error;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_core::ca::{
    LYS_OID_ARC, certificate_subject_public_key, create_certificate_request, decode_extension,
    verify_certificate_chain,
};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

/// A service with Ada as the administrator and Bea, their cookies, and the
/// key of Bea's agent.
struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
    key: Arc<Ed25519Identity>,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &service.dir.path().join("agent.key"),
        )?);
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
            key,
        })
    }

    fn agent(&self, person: usize) -> String {
        self.seeded.people[person].agents[0].id.to_string()
    }

    fn route(&self, person: usize) -> String {
        format!("/agents/{}/certificates", self.agent(person))
    }

    /// A request to issue under a new operation, over `key` for `subject`.
    fn request(key: &Arc<Ed25519Identity>, subject: &str) -> Result<Value, Box<dyn Error>> {
        Ok(json!({
            "operation": OperationId::generate()?.to_string(),
            "request": STANDARD.encode(create_certificate_request(key, subject)?),
        }))
    }
}

#[tokio::test]
async fn a_certificate_is_issued_over_the_proved_key_with_the_claims_kept() -> TestResult {
    let table = Table::set().await?;
    let agent = table.agent(1);
    let body = Table::request(&table.key, &agent)?;
    let (status, issued) = table
        .service
        .post(&table.route(1), Some(&table.bea), &body)
        .await?;
    assert_eq!(status, 200, "{issued}");
    assert_eq!(issued["recorded"], body["operation"]);
    assert_eq!(issued["claims_are_live"], false);
    let certificates = issued["certificates"].as_array().ok_or("no certificates")?;
    assert_eq!(certificates.len(), 1);
    let certificate = &certificates[0];
    assert_eq!(certificate["serial"], body["operation"]);
    assert_eq!(certificate["person"], table.seeded.people[1].id.to_string());
    assert_eq!(certificate["withdrawn"], Value::Null);
    let claims = &certificate["claims"];
    assert_eq!(claims["agent"], agent);
    assert_eq!(claims["person"], table.seeded.people[1].id.to_string());
    assert_eq!(claims["roles"], json!([]));
    assert_eq!(claims["grants"], json!([]));
    assert_eq!(claims["profile_version"], Value::Null);
    assert_eq!(claims["held_at"], certificate["issued_at"]);

    let der = STANDARD.decode(certificate["der"].as_str().ok_or("no der")?)?;
    assert_eq!(
        certificate_subject_public_key(&der)?,
        table.key.public_key_bytes(),
        "the certificate is over the key the request proved"
    );
    let service_key = Ed25519Identity::load(&table.service.dir.path().join("service.key"))?;
    verify_certificate_chain(&der, &service_key.public_key_bytes())?;
    let mut oid = LYS_OID_ARC.to_vec();
    oid.push(1);
    let carried = decode_extension(&der, &oid)?.ok_or("the certificate carries no claims")?;
    assert_eq!(&serde_json::from_slice::<Value>(&carried)?, claims);

    let (status, again) = table
        .service
        .post(&table.route(1), Some(&table.bea), &body)
        .await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again, issued, "sent again it is entered once");

    let (status, read) = table.service.get(&table.route(1), Some(&table.ada)).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["recorded"], Value::Null);
    assert_eq!(read["certificates"], issued["certificates"]);
    Ok(())
}

#[tokio::test]
async fn a_request_that_proves_nothing_for_this_agent_issues_nothing() -> TestResult {
    let table = Table::set().await?;
    let agent = table.agent(1);
    let path = table.route(1);

    let other = Table::request(&table.key, &table.agent(0))?;
    let answer = table.service.post(&path, Some(&table.bea), &other).await?;
    refused(&answer, 400, "RequestMalformed");
    let mut torn = Table::request(&table.key, &agent)?;
    torn["request"] = json!("AAEC");
    let answer = table.service.post(&path, Some(&table.bea), &torn).await?;
    refused(&answer, 400, "RequestMalformed");
    let mut named = Table::request(&table.key, &agent)?;
    named["claims"] = json!({ "roles": ["administrator"] });
    let answer = table.service.post(&path, Some(&table.bea), &named).await?;
    refused(&answer, 400, "RequestMalformed");

    let body = Table::request(&table.key, &agent)?;
    let answer = table.service.post(&path, None, &body).await?;
    refused(&answer, 401, "NotSignedIn");
    let answer = table
        .service
        .post(&table.route(0), Some(&table.bea), &body)
        .await?;
    refused(&answer, 404, "AgentNotVisible");

    let (status, issued) = table.service.post(&path, Some(&table.bea), &body).await?;
    assert_eq!(status, 200, "{issued}");
    let elsewhere = Arc::new(Ed25519Identity::load_or_generate(
        &table.service.dir.path().join("other.key"),
    )?);
    let mut reused = Table::request(&elsewhere, &agent)?;
    reused["operation"] = body["operation"].clone();
    let answer = table.service.post(&path, Some(&table.bea), &reused).await?;
    refused(&answer, 409, "CertificateReused");

    let (status, read) = table.service.get(&path, Some(&table.bea)).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(
        read["certificates"], issued["certificates"],
        "one was issued"
    );
    Ok(())
}

#[tokio::test]
async fn a_certificate_is_withdrawn_beside_its_issuance() -> TestResult {
    let table = Table::set().await?;
    let body = Table::request(&table.key, &table.agent(1))?;
    let (status, issued) = table
        .service
        .post(&table.route(1), Some(&table.bea), &body)
        .await?;
    assert_eq!(status, 200, "{issued}");
    let serial = body["operation"].as_str().ok_or("no operation")?;
    let path = format!("{}/{serial}/withdrawal", table.route(1));
    let reason = json!({ "reason": "the key was replaced" });

    let answer = table.service.post(&path, None, &reason).await?;
    refused(&answer, 401, "NotSignedIn");
    let answer = table
        .service
        .post(&path, Some(&table.ada), &json!({ "reason": " " }))
        .await?;
    refused(&answer, 400, "RequestMalformed");
    let elsewhere = format!("{}/{serial}/withdrawal", table.route(0));
    let answer = table
        .service
        .post(&elsewhere, Some(&table.ada), &reason)
        .await?;
    refused(&answer, 404, "CertificateUnknown");

    let (status, withdrawn) = table.service.post(&path, Some(&table.ada), &reason).await?;
    assert_eq!(status, 200, "{withdrawn}");
    assert_eq!(withdrawn["recorded"], serial);
    let certificate = &withdrawn["certificates"][0];
    assert_eq!(
        certificate["withdrawn"]["by"],
        table.seeded.people[0].id.to_string()
    );
    assert_eq!(certificate["withdrawn"]["reason"], "the key was replaced");
    assert_eq!(
        certificate["der"], issued["certificates"][0]["der"],
        "the certificate entered is not edited"
    );
    assert_eq!(
        certificate["entry"]["leaf"],
        issued["certificates"][0]["entry"]["leaf"]
    );

    let (status, again) = table.service.post(&path, Some(&table.ada), &reason).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(
        again["certificates"][0]["withdrawn"],
        certificate["withdrawn"]
    );
    let answer = table
        .service
        .post(&path, Some(&table.bea), &json!({ "reason": "another" }))
        .await?;
    refused(&answer, 409, "CertificateWithdrawn");
    Ok(())
}
