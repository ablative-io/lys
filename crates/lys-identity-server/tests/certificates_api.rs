//! The certificate route: the certificates entered for an agent are shown
//! to the administrator and to whoever answers for the agent, each with
//! an inclusion proof that verifies offline; a withdrawal is shown beside
//! the certificate it withdraws; and the claims are never shown as live.

use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::merkle::{InclusionProof, RootHash, verify_inclusion_raw};
use lys_identity::AgentId;
use lys_identity::signer::load_service_key;
use lys_identity_server::certificates_store::{CertificateStore, Issued, Withdrawn};
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

fn bytes(hex: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    (0..hex.len())
        .step_by(2)
        .map(|at| {
            Ok(u8::from_str_radix(
                hex.get(at..at + 2).ok_or("odd hex")?,
                16,
            )?)
        })
        .collect()
}

/// A service whose certificate log already holds two certificates for
/// Bea's agent, the second withdrawn, and one for Ada's.
async fn table() -> Result<(Service, Seeded, String, String), Box<dyn Error>> {
    let (service, seeded) = Service::start_with(|config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
        let dir = config
            .certificates_dir
            .as_deref()
            .ok_or("no certificates_dir")?;
        let key = Arc::new(load_service_key(&config.event_key_file)?);
        let mut store = CertificateStore::open(dir, key)?;
        let beas = seeded.people[1].agents[0].id.to_string();
        let adas = seeded.people[0].agents[0].id.to_string();
        for (serial, agent, person) in [
            ("serial-1", &beas, 1),
            ("serial-2", &beas, 1),
            ("serial-3", &adas, 0),
        ] {
            store.issue(Issued {
                serial: serial.to_owned(),
                agent: agent.clone(),
                person: seeded.people[person].id.to_string(),
                claims: json!({ "role": "builder", "role_version": 1 }),
                der: "AAEC".to_owned(),
                issued_at: 10,
            })?;
        }
        store.withdraw(Withdrawn {
            serial: "serial-2".to_owned(),
            by: seeded.people[0].id.to_string(),
            reason: "the key was replaced".to_owned(),
            withdrawn_at: 20,
        })?;
        Ok(seeded)
    })
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    Ok((service, seeded, ada, bea))
}

#[tokio::test]
async fn the_certificates_of_an_agent_are_shown_with_a_proof_that_verifies() -> TestResult {
    let (service, seeded, ada, bea) = table().await?;
    let agent = seeded.people[1].agents[0].id.to_string();
    let path = format!("/agents/{agent}/certificates");
    let (status, seen) = service.get(&path, Some(&bea)).await?;
    assert_eq!(status, 200, "{seen}");
    assert_eq!(seen["agent"], agent);
    assert_eq!(seen["claims_are_live"], false);
    let certificates = seen["certificates"].as_array().ok_or("no certificates")?;
    assert_eq!(certificates.len(), 2, "only this agent's: {seen}");
    assert_eq!(certificates[0]["serial"], "serial-1");
    assert_eq!(certificates[0]["withdrawn"], Value::Null);
    assert_eq!(
        certificates[0]["claims"],
        json!({ "role": "builder", "role_version": 1 })
    );
    assert_eq!(certificates[1]["serial"], "serial-2");
    assert_eq!(
        certificates[1]["withdrawn"]["reason"],
        "the key was replaced"
    );
    for certificate in certificates {
        let entry = &certificate["entry"];
        let size = entry["tree_size"].as_u64().ok_or("no tree size")?;
        let root: [u8; 32] = bytes(entry["root"].as_str().ok_or("no root")?)?
            .try_into()
            .map_err(|_short| "the root is not 32 bytes")?;
        let proof =
            InclusionProof::try_from_bytes(bytes(entry["proof"].as_str().ok_or("no proof")?)?)?;
        let leaf = bytes(entry["leaf_bytes"].as_str().ok_or("no leaf")?)?;
        verify_inclusion_raw(
            &RootHash::from_parts(root, size),
            &leaf,
            entry["leaf"].as_u64().ok_or("no leaf index")?,
            &proof,
        )?;
        let line: Value = serde_json::from_slice(&leaf)?;
        assert!(
            line.to_string()
                .contains(certificate["serial"].as_str().ok_or("no serial")?),
            "the leaf is the issuance: {line}"
        );
    }

    let (status, same) = service.get(&path, Some(&ada)).await?;
    assert_eq!(status, 200, "{same}");
    assert_eq!(same, seen, "the administrator reads the same");
    Ok(())
}

#[tokio::test]
async fn certificates_are_not_visible_to_anyone_else() -> TestResult {
    let (service, seeded, ada, bea) = table().await?;
    let adas = format!("/agents/{}/certificates", seeded.people[0].agents[0].id);
    let answer = service.get(&adas, None).await?;
    refused(&answer, 401, "NotSignedIn");
    let answer = service.get(&adas, Some(&bea)).await?;
    refused(&answer, 404, "AgentNotVisible");
    let stranger = format!("/agents/{}/certificates", AgentId::generate()?);
    let answer = service.get(&stranger, Some(&ada)).await?;
    refused(&answer, 404, "AgentNotVisible");
    Ok(())
}
