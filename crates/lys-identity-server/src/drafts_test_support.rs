//! HTTP admission and immutable evidence at the draft boundary.

pub(super) use std::error::Error;
pub(super) use std::sync::Arc;
pub(super) use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) use base64::Engine;
pub(super) use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
pub(super) use lys_core::Ed25519Identity;
pub(super) use lys_core::attestation::sign_attestation;
pub(super) use lys_core::ca::{CertificateAuthority, create_certificate_request};
pub(super) use lys_identity::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
pub(super) use lys_log_store::FileLeafStore;
pub(super) use serde_json::{Value, json};

pub(super) type Failure = Box<dyn Error>;
pub(super) type Result<T = ()> = std::result::Result<T, Failure>;

pub(super) struct Table {
    pub(super) service: Service,
    pub(super) agent: String,
    pub(super) person: String,
    pub(super) owner: String,
    pub(super) other: String,
    pub(super) key: Arc<Ed25519Identity>,
}

pub(super) fn operation() -> Result<String> {
    Ok(OperationId::generate()?.to_string())
}

impl Table {
    pub(super) async fn start() -> Result<Self> {
        let (service, (agent, person, key, owner, other)) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            None,
            None,
            |config| {
                config.requests_dir = None;
                config.roles_file = None;
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

    pub(super) fn body(&self, draft: OperationId) -> Value {
        json!({
            "operation": draft.to_string(),
            "target": {"kind": "person", "id": self.person, "action": "person.profile.set"},
            "method": "POST",
            "path": format!("/identities/{}/profile", self.person),
            "body": "{ \"display_name\": \"Prepared\" }",
            "note": "Review this profile change",
        })
    }

    pub(super) fn header(&self, body: &[u8]) -> Result<(String, Vec<u8>)> {
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

    pub(super) async fn create(&self) -> Result<(OperationId, Vec<u8>, String, Value)> {
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

    pub(super) fn directory(&self) -> Result<Directory<FileLeafStore>> {
        let path = self.service.dir.path().join("log");
        Ok(Directory::open(
            Box::new(move || FileLeafStore::open(&path)),
            Ed25519Identity::load(&self.service.dir.path().join("service.key"))?,
        )?)
    }
}

