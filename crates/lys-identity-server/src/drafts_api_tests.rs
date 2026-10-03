//! HTTP admission and immutable evidence at the draft boundary.

use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::{CertificateAuthority, create_certificate_request};
use lys_identity::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use lys_log_store::FileLeafStore;
use serde_json::{Value, json};

type Failure = Box<dyn Error>;
type Result<T = ()> = std::result::Result<T, Failure>;

struct Table {
    service: Service,
    agent: String,
    person: String,
    owner: String,
    other: String,
    key: Arc<Ed25519Identity>,
}

fn operation() -> Result<String> {
    Ok(OperationId::generate()?.to_string())
}

impl Table {
    async fn start() -> Result<Self> {
        let (service, (agent, person, key, owner, other)) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            None,
            None,
            |config| {
                config.requests_dir = None;
                config.roles_file = None;
                config.provisioning_file = None;
                config.network_file = None;
                config.runtime_dir = None;
                config.service_accounts_dir = None;
                config.teams_dir = None;
                config.stops_dir = None;
                config.budgets_dir = None;
                config.policies_dir = None;
                config.goals_dir = None;
                config.reviews_dir = None;
            },
            |config| {
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let actor = Actor::new(
                    LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                    Provenance::new(AuthMethod::Oidc, 1),
                );
                FileLeafStore::create(&config.log_dir, &config.log_origin)?;
                let path = config.log_dir.clone();
                let mut directory = Directory::open(
                    Box::new(move || FileLeafStore::open(&path)),
                    Ed25519Identity::load(&config.event_key_file)?,
                )?;
                let (person, _) = directory.setup_person(
                    actor.clone(),
                    OperationId::generate()?,
                    Profile::new("Owner")?,
                    1,
                )?;
                let (other, _) = directory.register_person(
                    actor.clone(),
                    OperationId::generate()?,
                    Profile::new("Other")?,
                    1,
                )?;
                directory.bind_login(
                    actor.clone(),
                    OperationId::generate()?,
                    other,
                    LoginBinding::new(&config.issuer, "other-subject")?,
                    1,
                )?;
                directory.transition(
                    actor.clone(),
                    OperationId::generate()?,
                    IdentityId::Person(other),
                    Transition::Activate,
                    "",
                    2,
                )?;
                let (agent, _) = directory.register_agent(
                    actor.clone(),
                    OperationId::generate()?,
                    person,
                    Profile::new("Agent")?,
                    2,
                )?;
                directory.transition(
                    actor,
                    OperationId::generate()?,
                    IdentityId::Agent(agent),
                    Transition::Activate,
                    "",
                    3,
                )?;
                let sessions = crate::session::Sessions::open(
                    config.sessions_file.clone().ok_or("session file missing")?,
                    600,
                    false,
                )?;
                let owner = sessions.begin(Actor::new(
                    LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                    Provenance::new(AuthMethod::Oidc, crate::session::now()),
                ))?;
                let other_cookie = sessions.begin(Actor::new(
                    LoginBinding::new(&config.issuer, "other-subject")?,
                    Provenance::new(AuthMethod::Oidc, crate::session::now()),
                ))?;
                let certificate =
                    CertificateAuthority::new(Ed25519Identity::load(&config.event_key_file)?)
                        .issue_certificate_for_request(
                            &create_certificate_request(&key, &agent.to_string())?,
                            &agent.to_string(),
                            Duration::from_secs(3600),
                            vec![],
                        )?;
                crate::certificates_store::CertificateStore::open(
                    config
                        .certificates_dir
                        .as_deref()
                        .ok_or("certificate log missing")?,
                    Arc::clone(&key),
                )?
                .issue(crate::certificates_store::Issued {
                    serial: operation()?,
                    agent: agent.to_string(),
                    person: person.to_string(),
                    claims: json!({}),
                    der: base64::engine::general_purpose::STANDARD.encode(certificate.der_bytes),
                    issued_at: crate::session::now(),
                })?;
                Ok((
                    agent.to_string(),
                    person.to_string(),
                    key,
                    owner,
                    other_cookie,
                ))
            },
        )
        .await?;
        Ok(Self {
            service,
            agent,
            person,
            owner,
            other,
            key,
        })
    }

    fn body(&self, draft: OperationId) -> Value {
        json!({
            "operation": draft.to_string(),
            "target": {"kind": "person", "id": self.person, "action": "write"},
            "method": "POST",
            "path": format!("/identities/{}/profile", self.person),
            "body": "{ \"display_name\": \"Prepared\" }",
            "note": "Review this profile change",
        })
    }

    fn header(&self, body: &[u8]) -> Result<(String, Vec<u8>)> {
        let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce = crate::routes::hex(OperationId::generate()?.as_bytes()).to_uppercase();
        let payload = crate::agent_signature::payload("POST", "/drafts", body, at, &nonce);
        let signature = sign_attestation(&payload, &self.key).to_cose_bytes();
        Ok((
            format!(
                "{}  0{at}\t{nonce} {}",
                self.agent,
                crate::routes::hex(&signature)
            ),
            payload,
        ))
    }

    async fn create(&self) -> Result<(OperationId, Vec<u8>, String, Value)> {
        let draft = OperationId::generate()?;
        let body = self.body(draft).to_string().into_bytes();
        let (header, _) = self.header(&body)?;
        let (status, answer) = self
            .service
            .post_signed(
                "/drafts",
                (crate::agent_signature::HEADER, &header),
                body.clone(),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok((draft, body, header, answer))
    }

    fn directory(&self) -> Result<Directory<FileLeafStore>> {
        let path = self.service.dir.path().join("log");
        Ok(Directory::open(
            Box::new(move || FileLeafStore::open(&path)),
            Ed25519Identity::load(&self.service.dir.path().join("service.key"))?,
        )?)
    }
}

#[tokio::test]
async fn an_agents_draft_round_trips_with_original_evidence() -> Result {
    let mut table = Table::start().await?;
    let (draft, body, header, answer) = table.create().await?;
    table.service.restart().await?;
    let mut directory = table.directory()?;
    let held = directory
        .projection()?
        .draft(draft)
        .ok_or("draft not retained")?;
    let proof = held.created.evidence.as_ref().ok_or("evidence missing")?;
    let signature = held
        .created
        .request_signature
        .as_ref()
        .ok_or("header missing")?;
    assert_eq!(
        held.created
            .actor
            .provenance()
            .agent()
            .map(|id| id.to_string()),
        Some(table.agent)
    );
    assert_eq!(proof.body, body);
    assert_eq!(signature.header, header.as_bytes());
    assert_eq!(
        signature.payload,
        crate::agent_signature::payload("POST", "/drafts", &body, proof.signed_at_ms, &proof.nonce,)
    );
    assert_eq!(answer["creation_hash"], crate::routes::hex(&held.hash));
    assert_eq!(held.created.body, b"{ \"display_name\": \"Prepared\" }");
    Ok(())
}

#[tokio::test]
async fn another_person_cannot_approve_a_draft() -> Result {
    let table = Table::start().await?;
    let (draft, _, _, created) = table.create().await?;
    let before = table.service.log_size().await?;
    let (status, answer) = table.service.post(
        &format!("/drafts/{draft}/approve"), Some(&table.other),
        &json!({"operation": operation()?, "creation_hash": created["creation_hash"], "application": operation()?}),
    ).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted");
    assert_eq!(table.service.log_size().await?, before);
    Ok(())
}

#[tokio::test]
async fn a_wrong_creation_hash_writes_no_leaf() -> Result {
    let table = Table::start().await?;
    let (draft, _, _, _) = table.create().await?;
    let before = table.service.log_size().await?;
    let (status, answer) = table.service.post(
        &format!("/drafts/{draft}/approve"), Some(&table.owner),
        &json!({"operation": operation()?, "creation_hash": "00".repeat(32), "application": operation()?}),
    ).await?;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["refusal"], "DraftHashMismatch");
    assert_eq!(table.service.log_size().await?, before);
    Ok(())
}

#[tokio::test]
async fn a_signature_plus_a_cookie_is_refused_before_consuming_the_nonce() -> Result {
    let table = Table::start().await?;
    let body = table
        .body(OperationId::generate()?)
        .to_string()
        .into_bytes();
    let (header, _) = table.header(&body)?;
    let before = table.service.log_size().await?;
    let (status, answer) = table
        .service
        .post_carrying(
            "/drafts",
            &[
                (crate::agent_signature::HEADER, &header),
                ("cookie", &table.owner),
            ],
            body.clone(),
        )
        .await?;
    assert_eq!(status, 401, "{answer}");
    assert_eq!(answer["refusal"], "AgentSignatureRefused");
    assert_eq!(table.service.log_size().await?, before);
    let (status, answer) = table
        .service
        .post_signed("/drafts", (crate::agent_signature::HEADER, &header), body)
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(())
}

#[tokio::test]
async fn decisions_are_recorded_without_applying_the_prepared_action() -> Result {
    let table = Table::start().await?;
    for decision in ["approve", "refuse", "correct"] {
        let (draft, _, _, created) = table.create().await?;
        let replacement = OperationId::generate()?;
        let body = match decision {
            "approve" => json!({
                "operation": operation()?, "creation_hash": created["creation_hash"],
                "application": operation()?,
            }),
            "refuse" => json!({
                "operation": operation()?, "creation_hash": created["creation_hash"],
                "reason": "Prepared words are refused",
            }),
            _ => json!({
                "operation": operation()?, "creation_hash": created["creation_hash"],
                "reason": "Saving my own correction", "replacement": table.body(replacement),
            }),
        };
        let before = table.service.log_size().await?;
        let (status, answer) = table
            .service
            .post(
                &format!("/drafts/{draft}/{decision}"),
                Some(&table.owner),
                &body,
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        assert_eq!(table.service.log_size().await?, before + 1);
        let mut directory = table.directory()?;
        let projection = directory.projection()?;
        let held = projection.draft(draft).ok_or("decided draft missing")?;
        assert!(!held.is_pending());
        match decision {
            "approve" => assert!(held.approved.is_some()),
            "refuse" => assert!(held.refused.is_some()),
            _ => {
                assert!(held.correction.is_some());
                let own = projection.draft(replacement).ok_or("replacement missing")?;
                assert_eq!(own.replacement_of, Some(draft));
                assert!(own.created.evidence.is_none());
                assert!(own.created.actor.provenance().agent().is_none());
                assert_eq!(answer["replacement"], replacement.to_string());
            }
        }
        assert_eq!(
            projection
                .record(IdentityId::Person(table.person.parse()?))
                .ok_or("person missing")?
                .profile()
                .display_name(),
            "Owner"
        );
    }
    Ok(())
}

#[test]
fn draft_routes_advertise_their_request_and_response_schemas() -> Result {
    let api = crate::openapi::api();
    for path in [
        "/drafts",
        "/drafts/{id}/approve",
        "/drafts/{id}/refuse",
        "/drafts/{id}/correct",
    ] {
        let route = api
            .routes()
            .iter()
            .find(|route| route.path == path)
            .ok_or("route missing")?;
        assert_eq!(route.method, lys_openapi::Method::Post);
        assert!(route.request.is_some());
        assert!(route.response.is_some());
    }
    Ok(())
}

#[tokio::test]
async fn an_unknown_a_decided_and_an_invalid_draft_are_refused_and_a_large_one_kept() -> Result {
    let table = Table::start().await?;
    let owner = &table;
    let approve = |draft: String, hash: Value| async move {
        owner
            .service
            .post(
                &format!("/drafts/{draft}/approve"),
                Some(&owner.owner),
                &json!({"operation": operation()?, "creation_hash": hash, "application": operation()?}),
            )
            .await
    };
    let before = table.service.log_size().await?;
    let (status, answer) =
        approve(OperationId::generate()?.to_string(), json!("00".repeat(32))).await?;
    assert_eq!(
        (status, answer["refusal"].clone()),
        (404, json!("DraftNotFound")),
        "{answer}"
    );
    assert_eq!(table.service.log_size().await?, before);

    let (draft, _, _, created) = table.create().await?;
    let (status, answer) = approve(draft.to_string(), created["creation_hash"].clone()).await?;
    assert_eq!(status, 200, "{answer}");
    let decided = table.service.log_size().await?;
    let (status, answer) = approve(draft.to_string(), created["creation_hash"].clone()).await?;
    assert_eq!(
        (status, answer["refusal"].clone()),
        (409, json!("DraftNotPending")),
        "{answer}"
    );
    assert_eq!(table.service.log_size().await?, decided);

    let mut invalid = table.body(OperationId::generate()?);
    invalid["target"]["action"] = json!("");
    let body = invalid.to_string().into_bytes();
    let (header, _) = table.header(&body)?;
    let (status, answer) = table
        .service
        .post_signed("/drafts", (crate::agent_signature::HEADER, &header), body)
        .await?;
    assert_eq!(
        (status, answer["refusal"].clone()),
        (409, json!("DraftChangeInvalid")),
        "{answer}"
    );
    assert_eq!(table.service.log_size().await?, decided);

    // A draft whose leaf, holding the request and the prepared body both, is
    // over 64 KiB is kept, and its prepared body comes back whole.
    let large_draft = OperationId::generate()?;
    let mut large = table.body(large_draft);
    let prepared = format!("{{ \"display_name\": \"{}\" }}", "x".repeat(35_000));
    large["body"] = json!(prepared);
    let body = large.to_string().into_bytes();
    let (header, _) = table.header(&body)?;
    let (status, answer) = table
        .service
        .post_signed("/drafts", (crate::agent_signature::HEADER, &header), body)
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(table.service.log_size().await?, decided + 1);
    let mut directory = table.directory()?;
    let projection = directory.projection()?;
    let held = projection
        .draft(large_draft)
        .ok_or("large draft not kept")?;
    assert_eq!(held.created.body, prepared.as_bytes());
    Ok(())
}

#[path = "drafts_mcp_tests.rs"]
mod mcp;
