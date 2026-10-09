//! What the product draft tests share: the service with Ada (the
//! administrator), Bea and Cy seeded, the fixture apps registered, and the
//! grants a draft rests on and is decided under. Lys's provider is on, so a
//! person signs in to the fixture app and holds the pass it issues.

use std::error::Error;

use identity_contract::apps::{Auth, NOTES, ok, op, post, registered};
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{Actor, AuthMethod, IdentityId, LoginBinding, Profile, Provenance, Transition};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::routes::open_directory;
use serde_json::{Value, json};

/// The second seeded person's subject.
pub const BEA: &str = "bea-subject";
/// A third person, who may grant `write` on the fixture documents.
pub const CY: &str = "cy-subject";
/// The fixture app's document kind.
pub const DOC: &str = "fixture_notes.doc";

/// The seeded service and each person's session and id.
pub struct Table {
    pub service: Service,
    pub admin: String,
    pub bea: String,
    pub bea_id: String,
    pub cy: String,
    pub notes: String,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Cy, registered, bound to `CY` at the configured issuer and active.
fn third_person(config: &lys_identity_server::config::Config) -> Result<String, Box<dyn Error>> {
    let mut directory = open_directory(config)?;
    let actor = Actor::new(
        config.administrator_binding()?,
        Provenance::new(AuthMethod::Oidc, now()),
    );
    let (cy, _) = directory.register_person(
        actor.clone(),
        lys_identity::OperationId::generate()?,
        Profile::new("Cy (test person)")?,
        now(),
    )?;
    directory.bind_login(
        actor.clone(),
        lys_identity::OperationId::generate()?,
        cy,
        LoginBinding::new(&config.issuer, CY)?,
        now(),
    )?;
    directory.transition(
        actor,
        lys_identity::OperationId::generate()?,
        IdentityId::Person(cy),
        Transition::Activate,
        "",
        now(),
    )?;
    Ok(cy.to_string())
}

/// A login at the fake issuer.
pub fn login(subject: &str) -> identity_contract::fake_issuer::Login {
    identity_contract::apps::login(subject)
}

impl Table {
    /// Ada, Bea and Cy signed in, `fixture_notes` registered and approved,
    /// and Cy holding `editor` on `fixture_notes.doc:1` with `write` to pass on.
    pub async fn set() -> Result<Self, Box<dyn Error>> {
        let broker = identity_contract::app_custody::start().await?;
        let (service, (seeded, cy_id)) = Service::start_adjusted(
            identity_contract::harness::GRANT_MODEL,
            None,
            None,
            None,
            move |config| {
                config.secrets = Some(lys_identity_server::secrets_api::SecretsSettings {
                    broker,
                    service: "identity".to_owned(),
                    service_key_file: config.event_key_file.clone(),
                });
                config.provider = Some(lys_identity_server::provider::ProviderSettings {
                    key_file: config.log_dir.with_file_name("provider.key"),
                    code_seconds: lys_identity_server::provider::CODE_SECONDS,
                    pass_seconds: lys_identity_server::provider::PASS_SECONDS,
                    rights_bytes: None,
                });
            },
            |config| {
                std::fs::write(config.log_dir.with_file_name("provider.key"), [8u8; 32])?;
                let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
                let cy = third_person(config)?;
                Ok((seeded, cy))
            },
        )
        .await?;
        let admin = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let cy = service.sign_in(login(CY)).await?;
        let notes = registered(&service, &admin, NOTES).await?;
        let table = Self {
            service,
            admin,
            bea,
            bea_id: seeded.people[1].id.to_string(),
            cy,
            notes,
        };
        table
            .root(
                &cy_id,
                "1",
                "outright",
                json!({"kind": "to", "actions": ["write"], "recipients": ["person"]}),
            )
            .await?;
        Ok(table)
    }

    /// A root grant of `editor` on `fixture_notes.doc:id` to `holder` in `mode`.
    pub async fn root(
        &self,
        holder: &str,
        id: &str,
        mode: &str,
        pass_on: Value,
    ) -> Result<String, Box<dyn Error>> {
        let body = json!({
            "operation": op()?,
            "route": "api",
            "holder": holder,
            "resource": {"kind": DOC, "id": id},
            "relation": "editor",
            "pass_on": pass_on,
            "window": {"starts_at": 0, "ends_at": null},
            "mode": mode,
        });
        let issued = ok(post(
            &self.service,
            "/grants/roots",
            Auth::Cookie(&self.admin),
            &body,
        )
        .await?)?;
        Ok(issued["grant"]
            .as_str()
            .ok_or("the issue names its grant")?
            .to_owned())
    }

    /// The pass Lys issues Bea for the fixture app: she signs in to it
    /// through Lys's provider, and the app exchanges the code with its
    /// secret (ACCESS-002).
    pub async fn bea_pass(&self) -> Result<String, Box<dyn Error>> {
        use base64::Engine;
        use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
        use sha2::Digest;
        const BACK: &str = "https://app.example.test/signed-in";
        const VERIFIER: &str = "a-product-draft-verifier-of-enough-length-0123456789-ab";
        let challenge = URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(VERIFIER.as_bytes()));
        let answer = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?
            .get(format!("{}/oauth/authorize", self.service.base))
            .query(&[
                ("client_id", NOTES),
                ("redirect_uri", BACK),
                ("response_type", "code"),
                ("scope", "openid"),
                ("code_challenge", challenge.as_str()),
                ("code_challenge_method", "S256"),
            ])
            .header(reqwest::header::COOKIE, &self.bea)
            .send()
            .await?;
        let back = answer
            .headers()
            .get(reqwest::header::LOCATION)
            .ok_or("authorize sends the browser nowhere")?
            .to_str()?;
        let code = reqwest::Url::parse(back)?
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned())
            .ok_or("the app is given a code")?;
        let secret = identity_contract::app_custody::secret();
        let token: Value = reqwest::Client::new()
            .post(format!("{}/oauth/token", self.service.base))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Basic {}", STANDARD.encode(format!("{NOTES}:{secret}"))),
            )
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code.as_str()),
                ("redirect_uri", BACK),
                ("code_verifier", VERIFIER),
            ])
            .send()
            .await?
            .json()
            .await?;
        Ok(token["access_token"]
            .as_str()
            .ok_or_else(|| format!("no pass is issued: {token}"))?
            .to_owned())
    }

    /// A grant held by Bea on `fixture_notes.doc:id` in `mode`, use only.
    pub async fn bea_holds(&self, id: &str, mode: &str) -> Result<String, Box<dyn Error>> {
        self.root(&self.bea_id, id, mode, json!({"kind": "use_only"}))
            .await
    }
}

/// lys-pass's `DraftRequest` for `words` on `fixture_notes.doc:id`.
pub fn request(operation: &str, grant: &str, id: &str, words: &str) -> Value {
    json!({
        "operation": operation,
        "grant": grant,
        "target": {"kind": DOC, "id": id, "action": "write"},
        "request_digest": lys_pass::drafts::request_digest(words),
        "words": words,
    })
}

/// A decision body naming the words decided.
pub fn decision(words: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({"operation": op()?, "request_digest": lys_pass::drafts::request_digest(words)}))
}

/// Refuse the test unless `answer` is a refusal named `name`.
pub fn refused_as(answer: &(u16, Value), name: &str) -> Result<(), Box<dyn Error>> {
    if answer.0 == 200 || answer.1["refusal"] != name {
        return Err(format!("expected {name}, got {} {}", answer.0, answer.1).into());
    }
    Ok(())
}
