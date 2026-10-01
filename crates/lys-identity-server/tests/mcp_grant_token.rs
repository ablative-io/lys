#![cfg(test)]
//! Grant header admission preserves the ordinary route holding checks.
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::OperationId;
use serde_json::{Value, json};
use std::error::Error;
type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Fixture {
    service: Service,
    cookie: String,
    person: String,
    other: String,
    agent: String,
    client: reqwest::Client,
}
fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}
impl Fixture {
    async fn new() -> TestResult<Self> {
        // Only the stores token admission and the profile route touch are opened.
        let (service, (person, agent)) = Service::start_adjusted(
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
                config.stops_dir = None;
                config.budgets_dir = None;
                config.policies_dir = None;
                config.goals_dir = None;
                config.reviews_dir = None;
            },
            |config| {
                use lys_identity::{
                    Actor, AuthMethod, Directory, IdentityId, LoginBinding, Profile, Provenance,
                    Transition,
                };
                lys_log_store::FileLeafStore::create(&config.log_dir, &config.log_origin)?;
                let path = config.log_dir.clone();
                let mut directory = Directory::open(
                    Box::new(move || lys_log_store::FileLeafStore::open(&path)),
                    lys_core::Ed25519Identity::load(&config.event_key_file)?,
                )?;
                let actor = Actor::new(
                    LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                    Provenance::new(AuthMethod::Oidc, 1),
                );
                let (person, _) = directory.setup_person(
                    actor.clone(),
                    OperationId::generate()?,
                    Profile::new("Owner")?,
                    1,
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
                Ok((person.to_string(), agent.to_string()))
            },
        )
        .await?;
        let cookie = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "owner@example.test".to_owned(),
            })
            .await?;
        let other = lys_identity::PersonId::from_bytes([99; 16]).to_string();
        Ok(Self {
            service,
            cookie,
            person,
            other,
            agent,
            client: reqwest::Client::new(),
        })
    }
    async fn holding(&self) -> TestResult {
        let team = operation()?;
        let (status, _) = self
            .service
            .post(
                "/teams",
                Some(&self.cookie),
                &json!({"operation":team,"name":"Shared"}),
            )
            .await?;
        assert_eq!(status, 200);
        for member in [&self.person, &self.agent] {
            let (status, _) = self
                .service
                .post(
                    &format!("/teams/{team}/members"),
                    Some(&self.cookie),
                    &json!({"operation":operation()?,"member":member}),
                )
                .await?;
            assert_eq!(status, 200);
        }
        Ok(())
    }

    async fn token(&self, resource: &str, relation: &str) -> TestResult<String> {
        let wire = json!({"kind":"person","id":resource});
        let (status, root) = self.service.post("/grants/roots",Some(&self.cookie),&json!({"operation":operation()?,"route":"api","holder":self.person,"resource":wire,"relation":"alpha","pass_on":{"kind":"to","actions":["read","write"],"recipients":["agent"]},"window":{"starts_at":0,"ends_at":null}})).await?;
        assert_eq!(status, 200);
        let (status, held) = self.service.post("/grants",Some(&self.cookie),&json!({"operation":operation()?,"route":"api","source":root["grant"],"recipient":self.agent,"responsible":self.person,"resource":wire,"relation":relation,"pass_on":{"kind":"use_only"},"window":{"starts_at":0,"ends_at":null}})).await?;
        assert_eq!(status, 200);
        let grant = held["grant"].as_str().ok_or("missing grant")?;
        let (status, issued) = self
            .service
            .post(
                &format!("/grants/{grant}/tokens"),
                Some(&self.cookie),
                &json!({"expires_at":lys_identity_server::session::now()+300}),
            )
            .await?;
        assert_eq!(status, 200);
        Ok(issued["token"].as_str().ok_or("missing token")?.to_owned())
    }
    async fn call(
        &self,
        token: &str,
        method: &str,
        path: &str,
        body: Value,
        cookie: bool,
    ) -> TestResult<(u16, Value)> {
        let mut request = self.client.post(format!("{}/mcp",self.service.base)).header("content-type","application/json").header("accept","application/json, text/event-stream").header(lys_identity_server::grant_tokens::HEADER,token).json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"change","arguments":{"method":method,"path":path,"body":body}}}));
        if cookie {
            request = request.header("cookie", &self.cookie);
        }
        let response = request.send().await?;
        let status = response.status().as_u16();
        Ok((status, response.json().await?))
    }
}

#[tokio::test]
async fn mcp_grant_token_acts_only_on_its_resource_and_action() -> TestResult {
    let fixture = Fixture::new().await?;
    fixture.holding().await?;
    let writer = fixture.token(&fixture.person, "alpha").await?;
    let path = format!("/identities/{}/profile", fixture.person);
    let body =
        || -> TestResult<Value> { Ok(json!({"operation":operation()?,"display_name":"Changed"})) };
    let (status, answer) = fixture.call(&writer, "POST", &path, body()?, false).await?;
    assert_eq!(status, 200);
    assert_eq!(answer["result"]["structuredContent"]["status"], 200);
    let (status, answer) = fixture
        .call(
            &writer,
            "POST",
            &format!("/identities/{}/profile", fixture.other),
            body()?,
            false,
        )
        .await?;
    assert_eq!(status, 200);
    assert_eq!(
        answer["result"]["structuredContent"]["body"]["refusal"],
        "GrantTokenScopeMismatch"
    );
    let reader = fixture.token(&fixture.person, "beta").await?;
    let (status, answer) = fixture.call(&reader, "POST", &path, body()?, false).await?;
    assert_eq!(status, 200);
    assert_eq!(
        answer["result"]["structuredContent"]["body"]["refusal"],
        "GrantTokenScopeMismatch"
    );
    let (_, current) = fixture.service.get("/me", Some(&fixture.cookie)).await?;
    assert_eq!(current["person"]["display_name"], "Changed");
    Ok(())
}

#[tokio::test]
async fn mcp_grant_token_refuses_an_undeclared_route() -> TestResult {
    let fixture = Fixture::new().await?;
    let token = fixture.token(&fixture.person, "alpha").await?;
    let (status, answer) = fixture
        .call(&token, "GET", "/directory/people", Value::Null, false)
        .await?;
    assert_eq!(status, 200);
    assert_eq!(
        answer["result"]["structuredContent"]["body"]["refusal"],
        "TokenScopeUndeclared"
    );
    Ok(())
}

#[tokio::test]
async fn mcp_grant_token_refuses_a_personal_cookie() -> TestResult {
    let fixture = Fixture::new().await?;
    let token = fixture.token(&fixture.person, "alpha").await?;
    let (status, answer) = fixture
        .call(
            &token,
            "POST",
            &format!("/identities/{}/profile", fixture.person),
            json!({"operation":operation()?,"display_name":"Cookie borrowed"}),
            true,
        )
        .await?;
    assert_eq!(status, 400);
    assert_eq!(answer["refusal"], "GrantTokenCookieConflict");
    Ok(())
}

#[tokio::test]
async fn mcp_grant_token_refuses_a_person_holder() -> TestResult {
    let fixture = Fixture::new().await?;
    let (status,root) = fixture.service.post("/grants/roots",Some(&fixture.cookie),&json!({"operation":operation()?,"route":"api","holder":fixture.person,"resource":{"kind":"person","id":fixture.person},"relation":"alpha","pass_on":{"kind":"use_only"},"window":{"starts_at":0,"ends_at":null}})).await?;
    assert_eq!(status, 200);
    let grant = root["grant"].as_str().ok_or("missing grant")?;
    let (status, issued) = fixture
        .service
        .post(
            &format!("/grants/{grant}/tokens"),
            Some(&fixture.cookie),
            &json!({"expires_at":lys_identity_server::session::now()+300}),
        )
        .await?;
    assert_eq!(status, 200);
    let token = issued["token"].as_str().ok_or("missing token")?;
    let (status, answer) = fixture
        .call(
            token,
            "POST",
            &format!("/identities/{}/profile", fixture.person),
            json!({"operation":operation()?,"display_name":"Person fallback"}),
            false,
        )
        .await?;
    assert_eq!(status, 200);
    assert_eq!(
        answer["result"]["structuredContent"]["body"]["refusal"],
        "TokenHolderNotAgent"
    );
    Ok(())
}
