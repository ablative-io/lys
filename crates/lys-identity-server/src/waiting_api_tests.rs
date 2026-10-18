#![cfg(test)]
//! `GET /waiting` (DIRECTORY-101 R1): one list of what waits on the signed-in
//! caller, oldest first, each part as its own route reads it, and a part the
//! caller may not read answered by its refusal in place of its entries.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use identity_contract::apps::{Auth, NOTES, get, ok, op, post, registered};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use identity_contract::lys_identity_server::config::Config;
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::{CertificateAuthority, create_certificate_request};
use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const BEA: &str = "bea-subject";
/// A login bound to no person: signed in, but nobody the directory holds.
const NOBODY: &str = "nobody-subject";
const DOC: &str = "fixture_notes.doc";

/// Ada (the administrator and root authority) with her first agent, Bea,
/// the fixture app approved, and a session for a login bound to no one.
struct Table {
    service: Service,
    ada: String,
    bea: String,
    nobody: String,
    ada_id: String,
    bea_id: String,
    agent: String,
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
    async fn set() -> Result<Self> {
        let broker = identity_contract::app_custody::start().await?;
        let (service, (seeded, key, nobody)) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            None,
            None,
            move |config| {
                config.secrets = Some(
                    identity_contract::lys_identity_server::secrets_api::SecretsSettings {
                        broker,
                        service: "identity".to_owned(),
                        service_key_file: config.event_key_file.clone(),
                    },
                );
            },
            |config| {
                let seeded = identity_contract::lys_identity_server::dev_seed::seed_configured(
                    config,
                    [ADMINISTRATOR, BEA],
                )?;
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let agent = seeded.people[0].agents[0].id.to_string();
                certify(config, &key, &agent, &seeded.people[0].id.to_string())?;
                let sessions = crate::session::Sessions::open(
                    config.sessions_file.clone().ok_or("session file missing")?,
                    600,
                    false,
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
            key,
        })
    }

    /// A root grant of `relation` on `kind:id` to `holder`, by Ada.
    async fn root(
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
    async fn agent_holds(&self) -> Result<String> {
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
                "recipient": self.agent, "responsible": self.ada_id,
                "resource": {"kind": "doc", "id": "quarter"}, "relation": "beta",
                "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null},
            }),
        )
        .await?)?;
        Ok(lent["grant"]
            .as_str()
            .ok_or("the lend names its grant")?
            .to_owned())
    }

    /// Bea asks for `beta` on `doc:quarter`; Ada, the root authority, decides it.
    async fn bea_asks(&self) -> Result<String> {
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
    async fn agent_drafts(&self) -> Result<String> {
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
    async fn bea_product_drafts(&self) -> Result<String> {
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

    async fn waiting(&self, cookie: &str) -> Result<Vec<Value>> {
        let answer = ok(get(&self.service, "/waiting", Auth::Cookie(cookie)).await?)?;
        Ok(answer["waiting"]
            .as_array()
            .ok_or_else(|| format!("no list: {answer}"))?
            .clone())
    }
}

fn kinds(items: &[Value]) -> Vec<&str> {
    let mut kinds: Vec<&str> = items
        .iter()
        .filter_map(|item| item["kind"].as_str())
        .collect();
    kinds.sort_unstable();
    kinds
}

#[tokio::test]
async fn one_list_of_what_waits_oldest_first_and_a_refused_part_in_place() -> Result {
    let table = Table::set().await?;
    let request = table.bea_asks().await?;
    let draft = table.agent_drafts().await?;
    let product = table.bea_product_drafts().await?;
    let review = table.agent_holds().await?;

    let ada = table.waiting(&table.ada).await?;
    assert_eq!(
        kinds(&ada),
        ["agent_draft", "product_draft", "request", "review"],
        "{ada:?}"
    );
    let entry = |kind: &str| {
        ada.iter()
            .find(|item| item["kind"] == kind)
            .cloned()
            .unwrap_or_default()
    };
    assert_eq!(entry("request")["id"], request.as_str());
    assert_eq!(entry("request")["asked_by"]["id"], table.bea_id.as_str());
    assert_eq!(entry("agent_draft")["id"], draft.as_str());
    assert_eq!(entry("agent_draft")["asked_by"]["id"], table.agent.as_str());
    assert_eq!(entry("product_draft")["id"], product.as_str());
    assert_eq!(
        entry("product_draft")["asked_by"]["id"],
        table.bea_id.as_str()
    );
    assert_eq!(entry("review")["id"], review.as_str());
    assert_eq!(entry("review")["asked_by"]["id"], table.agent.as_str());
    let mut seconds = Vec::new();
    for item in &ada {
        assert_eq!(item["can_decide"], true, "{item}");
        assert!(item["refusal"].is_null(), "{item}");
        assert!(
            !item["about"].as_str().unwrap_or_default().is_empty(),
            "{item}"
        );
        let at: jiff::Timestamp = item["asked_at"].as_str().ok_or("no asked_at")?.parse()?;
        seconds.push(at.as_second());
    }
    assert!(seconds.is_sorted(), "oldest first: {seconds:?}");
    assert_eq!(
        entry("request")["about"],
        "Bea (test person) asks for beta access to doc quarter"
    );

    let bea = table.waiting(&table.bea).await?;
    assert_eq!(
        bea,
        Vec::<Value>::new(),
        "the asker and holder decides none of it"
    );

    let nobody = table.waiting(&table.nobody).await?;
    assert_eq!(
        kinds(&nobody),
        ["agent_draft", "product_draft", "request", "review"]
    );
    for part in &nobody {
        assert_eq!(part["refusal"], "NoPerson", "{part}");
        assert!(part["reason"].is_string(), "{part}");
        assert!(
            part["id"].is_null(),
            "a refused part carries no entry: {part}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn waiting_is_for_a_signed_in_caller_only() -> Result {
    let table = Table::set().await?;
    let (status, answer) = get(&table.service, "/waiting", Auth::Nobody).await?;
    assert_eq!(status, 401, "{answer}");
    Ok(())
}

#[test]
fn entries_are_ordered_oldest_first_after_the_refused_parts() {
    let entry = |kind: &'static str, at: u64| crate::waiting_api::Dated {
        at,
        entry: crate::waiting_api::WaitingEntry {
            kind,
            id: format!("{kind}-{at}"),
            asked_by: crate::waiting_api::Asker {
                id: "a".to_owned(),
                name: None,
            },
            about: String::new(),
            asked_at: String::new(),
            can_decide: true,
        },
    };
    let refused = crate::waiting_api::WaitingRefused {
        kind: "review",
        refusal: "ReviewsUnavailable".to_owned(),
        reason: "closed".to_owned(),
    };
    let list = crate::waiting_api::ordered(
        vec![
            entry("request", 30),
            entry("agent_draft", 10),
            entry("product_draft", 20),
        ],
        vec![refused],
    );
    let ids: Vec<String> = list
        .iter()
        .map(|item| match item {
            crate::waiting_api::WaitingItem::Entry(entry) => entry.id.clone(),
            crate::waiting_api::WaitingItem::Refused(part) => part.refusal.clone(),
        })
        .collect();
    assert_eq!(
        ids,
        [
            "ReviewsUnavailable",
            "agent_draft-10",
            "product_draft-20",
            "request-30"
        ]
    );
}
