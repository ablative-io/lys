//! Fixtures owned only by the reviews test binary.
use super::*;
/// A service with Ada as the root authority and Bea, each with their agents,
/// and the cookies of Ada and Bea.
pub(super) struct Table {
    pub(super) service: Service,
    pub(super) seeded: Seeded,
    pub(super) ada: String,
    pub(super) bea: String,
    pub(super) clock: std::sync::Arc<identity_contract::harness::ManualClock>,
    reviews_dir: std::path::PathBuf,
    key: std::sync::Arc<lys_core::Ed25519Identity>,
}

impl Table {
    pub(super) async fn set() -> Result<Self, Box<dyn Error>> {
        let clock =
            std::sync::Arc::new(identity_contract::harness::ManualClock::new(1_800_000_000));
        let supplied = std::sync::Arc::clone(&clock);
        let broker = identity_contract::app_custody::start().await?;
        let (service, (seeded, reviews_dir, key)) = Service::start_clock_adjusted(
            supplied,
            move |config| {
                config.session_seconds = 100 * 86_400;
                config.secrets = Some(lys_identity_server::secrets_api::SecretsSettings {
                    broker,
                    service: "identity".to_owned(),
                    service_key_file: config.event_key_file.clone(),
                });
            },
            None,
            |config| {
                Ok((
                    seed_configured(config, [ADMINISTRATOR, BEA])?,
                    config.reviews_dir.clone().ok_or("reviews directory")?,
                    std::sync::Arc::new(lys_core::Ed25519Identity::load(&config.event_key_file)?),
                ))
            },
        )
        .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
            clock,
            reviews_dir,
            key,
        })
    }

    /// Ada issues `person` a root grant on `resource` that may be lent to
    /// agents, and `person` lends it to `agent`. Answers both grant ids.
    pub(super) async fn lend(
        &self,
        person: usize,
        cookie: &str,
        resource: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        self.lend_to(person, person, cookie, resource).await
    }

    /// As `lend`, with the grant lent to the agent of `recipient`.
    pub(super) async fn lend_to(
        &self,
        person: usize,
        recipient: usize,
        cookie: &str,
        resource: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        self.lend_at(person, recipient, cookie, resource, 0).await
    }

    pub(super) async fn lend_at(
        &self,
        person: usize,
        recipient: usize,
        cookie: &str,
        resource: &str,
        starts_at: u64,
    ) -> Result<(String, String), Box<dyn Error>> {
        let holder = &self.seeded.people[person];
        let receiver = &self.seeded.people[recipient];
        let root = json!({
            "operation": operation()?,
            "route": "api",
            "holder": holder.id.to_string(),
            "resource": { "kind": "doc", "id": resource },
            "relation": "alpha",
            "pass_on": { "kind": "to", "actions": ["read"], "recipients": ["agent"] },
            "window": { "starts_at": 0, "ends_at": null },
        });
        let (status, issued) = self
            .service
            .post("/grants/roots", Some(&self.ada), &root)
            .await?;
        assert_eq!(status, 200, "{issued}");
        let source = issued["grant"].as_str().ok_or("no grant")?.to_owned();
        let lent = json!({
            "operation": operation()?,
            "route": "api",
            "source": source,
            "recipient": receiver.agents[0].id.to_string(),
            "responsible": receiver.id.to_string(),
            "resource": { "kind": "doc", "id": resource },
            "relation": "beta",
            "pass_on": { "kind": "use_only" },
            "window": { "starts_at": starts_at, "ends_at": null },
        });
        let (status, issued) = self.service.post("/grants", Some(cookie), &lent).await?;
        assert_eq!(status, 200, "{issued}");
        Ok((
            source,
            issued["grant"].as_str().ok_or("no grant")?.to_owned(),
        ))
    }

    pub(super) async fn keep(
        &self,
        cookie: &str,
        grant: &str,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .post(&format!("/reviews/{grant}/keep"), Some(cookie), body)
            .await
    }

    pub(super) async fn reviews(&self, cookie: &str) -> Result<Value, Box<dyn Error>> {
        let (status, body) = self.service.get("/reviews", Some(cookie)).await?;
        assert_eq!(status, 200, "{body}");
        Ok(body)
    }
}

impl Table {
    pub(super) fn store(
        &self,
    ) -> Result<lys_identity_server::reviews_store::ReviewStore, Box<dyn Error>> {
        Ok(lys_identity_server::reviews_store::ReviewStore::open(
            &self.reviews_dir,
            std::sync::Arc::clone(&self.key),
        )?)
    }

    pub(super) async fn waiting(&self) -> Result<Vec<Value>, Box<dyn Error>> {
        let (status, view) = self.service.get("/waiting", Some(&self.ada)).await?;
        assert_eq!(status, 200, "{view}");
        Ok(view["waiting"].as_array().ok_or("waiting list")?.clone())
    }

    pub(super) async fn keep_all(
        &self,
        cookie: &str,
        agent: &str,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .post(&format!("/reviews/agents/{agent}/keep"), Some(cookie), body)
            .await
    }
}
