#![cfg(test)]
//! `GET /waiting` (DIRECTORY-101 R1): one list of what waits on the signed-in
//! caller, oldest first, each part as its own route reads it, and a part the
//! caller may not read answered by its refusal in place of its entries.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use identity_contract::apps::{Auth, NOTES, get, ok, op, post, registered};
use identity_contract::harness::{ADMINISTRATOR, ManualClock, Service};
use identity_contract::lys_identity_server::config::Config;
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::{CertificateAuthority, create_certificate_request};
use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use serde_json::{Value, json};

pub(super) type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const BEA: &str = "bea-subject";
/// A login bound to no person: signed in, but nobody the directory holds.
const NOBODY: &str = "nobody-subject";
const DOC: &str = "fixture_notes.doc";

/// Ada (the administrator and root authority) with her first agent, Bea,
/// the fixture app approved, and a session for a login bound to no one.
pub(super) struct Table {
    pub(super) service: Service,
    pub(super) ada: String,
    pub(super) bea: String,
    pub(super) nobody: String,
    pub(super) ada_id: String,
    pub(super) bea_id: String,
    pub(super) agent: String,
    pub(super) other_agent: String,
    pub(super) clock: Arc<ManualClock>,
    key: Arc<Ed25519Identity>,
}

fn now() -> u64 {
    crate::session::now()
}

/// The agent's certificate, kept where the service reads certificates.
fn certify(config: &Config, key: &Arc<Ed25519Identity>, agent: &str, person: &str) -> Result {
    let certificate = CertificateAuthority::new(Ed25519Identity::load(&config.event_key_file)?)
        .issue_certificate_for_request(
            &create_certificate_request(key, agent)?,
            agent,
            Duration::from_secs(3600),
            vec![],
        )?;
    let dir = config
        .certificates_dir
        .as_deref()
        .ok_or("certificate log missing")?;
    crate::certificates_store::CertificateStore::open(dir, Arc::clone(key))?.issue(
        crate::certificates_store::Issued {
            serial: op()?,
            agent: agent.to_owned(),
            person: person.to_owned(),
            claims: json!({}),
            der: base64::engine::general_purpose::STANDARD.encode(certificate.der_bytes),
            issued_at: now(),
        },
    )?;
    Ok(())
}

impl Table {
    pub(super) async fn set() -> Result<Self> {
        let broker = identity_contract::app_custody::start().await?;
        let clock = Arc::new(ManualClock::new(1_800_000_000));
        let supplied = Arc::clone(&clock);
        let sessions_clock = Arc::clone(&clock);
        let (service, (seeded, key, nobody)) = Service::start_clock_adjusted(
            supplied,
            move |config| {
                config.session_seconds = 100 * 86_400;
                config.secrets = Some(
                    identity_contract::lys_identity_server::secrets_api::SecretsSettings {
                        broker,
                        service: "identity".to_owned(),
                        service_key_file: config.event_key_file.clone(),
                    },
                );
            },
            None,
            move |config| {
                let seeded = identity_contract::lys_identity_server::dev_seed::seed_configured(
                    config,
                    [ADMINISTRATOR, BEA],
                )?;
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let agent = seeded.people[0].agents[0].id.to_string();
                certify(config, &key, &agent, &seeded.people[0].id.to_string())?;
                let sessions = crate::session::Sessions::open_with_clock(
                    config.sessions_file.clone().ok_or("session file missing")?,
                    600,
                    false,
                    lys_core::clock::ClockSource::Supplied(sessions_clock),
                )?;
                let nobody = sessions.begin(Actor::new(
                    LoginBinding::new(&config.issuer, NOBODY)?,
                    Provenance::new(AuthMethod::Oidc, now()),
                ))?;
                Ok((seeded, key, nobody))
            },
        )
        .await?;
        let ada = service
            .sign_in(identity_contract::apps::login(ADMINISTRATOR))
            .await?;
        let bea = service.sign_in(identity_contract::apps::login(BEA)).await?;
        registered(&service, &ada, NOTES).await?;
        Ok(Self {
            service,
            ada,
            bea,
            nobody,
            ada_id: seeded.people[0].id.to_string(),
            bea_id: seeded.people[1].id.to_string(),
            agent: seeded.people[0].agents[0].id.to_string(),
            other_agent: seeded.people[0].agents[1].id.to_string(),
            clock,
            key,
        })
    }

    /// A root grant of `relation` on `kind:id` to `holder`, by Ada.
    pub(super) async fn root(
        &self,
        holder: &str,
        kind: &str,
        id: &str,
        relation: &str,
        mode: &str,
    ) -> Result<String> {
        let issued = ok(post(&self.service, "/grants/roots", Auth::Cookie(&self.ada), &json!({
            "operation": op()?, "route": "api", "holder": holder,
            "resource": {"kind": kind, "id": id}, "relation": relation,
            "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null}, "mode": mode,
        })).await?)
        .map_err(|refused| format!("{relation} on {kind}:{id} to {holder}: {refused}"))?;
        Ok(issued["grant"]
            .as_str()
            .ok_or("the issue names its grant")?
            .to_owned())
    }

    /// Ada gives her agent `beta` on `doc:quarter` from a root grant of her own; the agent's grant is due her review.
    pub(super) async fn agent_holds(&self) -> Result<String> {
        self.agent_holds_at(&self.agent, 0).await
    }

    pub(super) async fn agent_holds_at(&self, agent: &str, starts_at: u64) -> Result<String> {
        let (status, issued) = self
            .service
            .post(
                "/grants/roots",
                Some(&self.ada),
                &json!({
                    "operation": op()?, "route": "api", "holder": self.ada_id,
                    "resource": {"kind": "doc", "id": "quarter"}, "relation": "alpha",
                    "pass_on": {"kind": "to", "actions": ["read"], "recipients": ["agent"]},
                    "window": {"starts_at": 0, "ends_at": null},
                }),
            )
            .await?;
        assert_eq!(status, 200, "{issued}");
        let lent = ok(post(
            &self.service,
            "/grants",
            Auth::Cookie(&self.ada),
            &json!({
                "operation": op()?, "route": "api", "source": issued["grant"],
                "recipient": agent, "responsible": self.ada_id,
                "resource": {"kind": "doc", "id": "quarter"}, "relation": "beta",
                "pass_on": {"kind": "use_only"}, "window": {"starts_at": starts_at, "ends_at": null},
            }),
        )
        .await?)?;
        Ok(lent["grant"]
            .as_str()
            .ok_or("the lend names its grant")?
            .to_owned())
    }

    /// Bea asks for `beta` on `doc:quarter`; Ada, the root authority, decides it.
    pub(super) async fn bea_asks(&self) -> Result<String> {
        let operation = op()?;
        ok(post(
            &self.service,
            "/requests",
            Auth::Cookie(&self.bea),
            &json!({
                "operation": operation, "resource": {"kind": "doc", "id": "quarter"},
                "relation": "beta", "ends_at": null, "why": "to read the quarter's figures",
            }),
        )
        .await?)?;
        Ok(operation)
    }

    /// Ada's agent prepares a change to Ada's profile; Ada decides it.
    pub(super) async fn agent_drafts(&self) -> Result<String> {
        let draft = op()?;
        let body = json!({
            "operation": draft,
            "target": {"kind": "person", "id": self.ada_id, "action": "person.profile.set"},
            "method": "POST", "path": format!("/identities/{}/profile", self.ada_id),
            "body": "{ \"display_name\": \"Prepared\" }", "note": "Review this profile change",
        })
        .to_string()
        .into_bytes();
        let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce =
            crate::routes::hex(lys_identity::OperationId::generate()?.as_bytes()).to_uppercase();
        let payload = crate::agent_signature::payload("POST", "/drafts", &body, at, &nonce);
        let signature = sign_attestation(&payload, &self.key).to_cose_bytes();
        let header = format!(
            "{}  0{at}\t{nonce} {}",
            self.agent,
            crate::routes::hex(&signature)
        );
        let (status, answer) = self
            .service
            .post_signed("/drafts", (crate::agent_signature::HEADER, &header), body)
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(draft)
    }

    /// Bea holds a by-draft grant on the fixture document and drafts an act on it.
    pub(super) async fn bea_product_drafts(&self) -> Result<String> {
        let grant = self
            .root(&self.bea_id, DOC, "1", "editor", "by_draft")
            .await?;
        let words = "{\"stop\":\"seat-7\"}";
        let made = ok(post(
            &self.service,
            "/product-drafts",
            Auth::Cookie(&self.bea),
            &json!({
                "operation": op()?, "grant": grant,
                "target": {"kind": DOC, "id": "1", "action": "write"},
                "request_digest": lys_pass::drafts::request_digest(words), "words": words,
            }),
        )
        .await?)?;
        Ok(made["draft"].as_str().ok_or("a draft id")?.to_owned())
    }

    pub(super) async fn waiting(&self, cookie: &str) -> Result<Vec<Value>> {
        let answer = ok(get(&self.service, "/waiting", Auth::Cookie(cookie)).await?)?;
        Ok(answer["waiting"]
            .as_array()
            .ok_or_else(|| format!("no list: {answer}"))?
            .clone())
    }
}
