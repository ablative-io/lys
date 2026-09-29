//! Real signed requests require an Active responsible person as well as an Active agent.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::harness::{ADMINISTRATOR, LINK_AUDIT_SOURCE, Service};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::{CertificateAuthority, create_certificate_request};
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
};
use lys_identity_server::agent_signature::{HEADER, payload};
use lys_identity_server::certificates_store::{CertificateStore, Issued};
use lys_identity_server::routes::open_directory;
use serde_json::json;

use super::support::{TestResult, inactive, stored};

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 15)],
            ]
        })
        .map(char::from)
        .collect()
}

async fn signed_write(person_active: bool) -> TestResult {
    let (service, (person, agent, key)) = Service::start_with(|config| {
        let mut directory = open_directory(config)?;
        let admin = Actor::new(
            LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
            Provenance::new(AuthMethod::Oidc, 1),
        );
        let binding = LoginBinding::new(&config.issuer, LINK_AUDIT_SOURCE)?;
        let (person, _) = directory.register_person(
            admin.clone(),
            OperationId::generate()?,
            Profile::new("Source person")?,
            1,
        )?;
        directory.bind_login(admin.clone(), OperationId::generate()?, person, binding, 2)?;
        let (agent, _) = directory.register_agent(
            admin.clone(),
            OperationId::generate()?,
            person,
            Profile::new("Source agent")?,
            3,
        )?;
        if person_active {
            directory.transition(
                admin.clone(),
                OperationId::generate()?,
                IdentityId::Person(person),
                Transition::Activate,
                "positive control",
                4,
            )?;
        }
        directory.transition(
            admin,
            OperationId::generate()?,
            IdentityId::Agent(agent),
            Transition::Activate,
            "active agent fixture",
            5,
        )?;
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &config.log_dir.with_file_name("signed-agent.key"),
        )?);
        let service_key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
        let request = create_certificate_request(&key, &agent.to_string())?;
        let certificate = CertificateAuthority::new(Arc::clone(&service_key))
            .issue_certificate_for_request(
                &request,
                &agent.to_string(),
                Duration::from_secs(600),
                vec![],
            )?;
        // The fixture holds a genuine certificate directly; it never grants a Registered
        // person an HTTP issuance exception or rewrites an Active record to Registered.
        CertificateStore::open(
            config
                .certificates_dir
                .as_ref()
                .ok_or("certificate store")?,
            service_key,
        )?
        .issue(Issued {
            serial: OperationId::generate()?.to_string(),
            agent: agent.to_string(),
            person: person.to_string(),
            claims: json!({}),
            der: STANDARD.encode(certificate.der_bytes),
            issued_at: 6,
        })?;
        Ok((person, agent, key))
    })
    .await?;
    let path = "/link-audit";
    let body = json!({
        "person": person.to_string(), "source_operation_id": OperationId::generate()?.to_string(),
        "change": "linked", "issuer": "https://accounts.fixture.test", "subject": "linked-account",
        "observer": service.issuer.issuer(), "observed_at": 1_790_000_200_u64,
    })
    .to_string()
    .into_bytes();
    let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let nonce = hex(&[17; 16]);
    let signed = payload("POST", path, &body, at, &nonce);
    let signature = sign_attestation(&signed, &key).to_cose_bytes();
    let header = format!("{agent} {at} {nonce} {}", hex(&signature));
    let before = stored(&service)?;
    let answer = service.post_signed(path, (HEADER, &header), body).await?;
    if person_active {
        assert_eq!(answer.0, 200, "{}", answer.1);
        assert_ne!(
            stored(&service)?,
            before,
            "Active signed control recorded nothing"
        );
    } else {
        assert_eq!(
            stored(&service)?,
            before,
            "Registered responsible person signed a write"
        );
        inactive(&answer, person);
    }
    Ok(())
}

#[tokio::test]
async fn signed_agent_of_registered_person_cannot_write() -> TestResult {
    signed_write(false).await
}

#[tokio::test]
async fn signed_agent_of_active_person_can_write() -> TestResult {
    signed_write(true).await
}
