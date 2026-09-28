//! The service account routes: a person creates and retires service accounts
//! they own and sees only their own, the administrator names any person as
//! owner and sees every one, `GET /me` lists the caller's own that are in
//! use, the same act sent again answers what was recorded, and each refusal
//! is by name.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::read_views::{MeView, ServiceAccountView, ServiceAccountsView};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const ADA: &str = "ada-subject";
const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

struct Table {
    service: Service,
    seeded: Seeded,
    admin: String,
    ada: String,
    bea: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADA, BEA])?)).await?;
        let admin = service.sign_in(login(ADMINISTRATOR)).await?;
        let ada = service.sign_in(login(ADA)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        Ok(Self {
            service,
            seeded,
            admin,
            ada,
            bea,
        })
    }

    fn person(&self, index: usize) -> Result<String, Box<dyn Error>> {
        Ok(self
            .seeded
            .people
            .get(index)
            .ok_or("the seed holds two people")?
            .id
            .to_string())
    }

    async fn create(&self, cookie: &str, body: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .post("/service-accounts", Some(cookie), body)
            .await
    }

    async fn made(&self, cookie: &str, body: &Value) -> Result<ServiceAccountView, Box<dyn Error>> {
        let (status, answer) = self.create(cookie, body).await?;
        assert_eq!(status, 200, "{answer}");
        Ok(serde_json::from_value(answer)?)
    }

    async fn list(&self, cookie: &str) -> Result<ServiceAccountsView, Box<dyn Error>> {
        let (status, answer) = self.service.get("/service-accounts", Some(cookie)).await?;
        assert_eq!(status, 200, "{answer}");
        Ok(serde_json::from_value(answer)?)
    }
}

#[tokio::test]
async fn a_person_creates_their_own_and_sees_only_their_own() -> TestResult {
    let table = Table::set().await?;
    let (ada_id, bea_id) = (table.person(0)?, table.person(1)?);

    let ada_own = table
        .made(
            &table.ada,
            &json!({ "operation": operation()?, "name": "docs deploy", "description": "publishes the site" }),
        )
        .await?;
    assert_eq!(ada_own.owner, ada_id);
    assert_eq!(ada_own.state, "active");
    assert_eq!(ada_own.created_by.subject, ADA);
    let bea_own = table
        .made(
            &table.bea,
            &json!({ "operation": operation()?, "name": "backup reader" }),
        )
        .await?;
    assert_eq!(bea_own.owner, bea_id);
    let named = table
        .made(
            &table.admin,
            &json!({ "operation": operation()?, "name": "ci runner", "owner": ada_id }),
        )
        .await?;
    assert_eq!(named.owner, ada_id, "the administrator names any owner");
    assert_eq!(named.created_by.subject, ADMINISTRATOR);

    let bea_sees = table.list(&table.bea).await?;
    assert_eq!(bea_sees.scope, "personal");
    assert_eq!(bea_sees.service_accounts, vec![bea_own.clone()]);
    let ada_sees = table.list(&table.ada).await?;
    assert_eq!(
        ada_sees.service_accounts,
        vec![ada_own.clone(), named.clone()]
    );
    let admin_sees = table.list(&table.admin).await?;
    assert_eq!(admin_sees.scope, "directory");
    assert_eq!(admin_sees.service_accounts, vec![ada_own, bea_own, named]);
    Ok(())
}

#[tokio::test]
async fn me_lists_the_callers_own_in_use() -> TestResult {
    let table = Table::set().await?;
    let kept = table
        .made(
            &table.bea,
            &json!({ "operation": operation()?, "name": "backup reader" }),
        )
        .await?;
    let ended = table
        .made(
            &table.bea,
            &json!({ "operation": operation()?, "name": "old uploader" }),
        )
        .await?;
    table
        .made(
            &table.ada,
            &json!({ "operation": operation()?, "name": "docs deploy" }),
        )
        .await?;
    let (status, retired) = table
        .service
        .post(
            &format!("/service-accounts/{}/retire", ended.id),
            Some(&table.bea),
            &json!({ "operation": operation()? }),
        )
        .await?;
    assert_eq!(status, 200, "{retired}");
    assert_eq!(retired["state"], "retired");

    let (status, body) = table.service.get("/me", Some(&table.bea)).await?;
    assert_eq!(status, 200, "{body}");
    let me: MeView = serde_json::from_value(body)?;
    assert_eq!(
        me.service_accounts,
        vec![kept],
        "only the caller's own, and only those in use"
    );
    Ok(())
}

#[tokio::test]
async fn the_same_request_again_answers_what_was_recorded() -> TestResult {
    let table = Table::set().await?;
    let body = json!({ "operation": operation()?, "name": "docs deploy", "description": "publishes the site" });
    let first = table.made(&table.ada, &body).await?;
    let again = table.made(&table.ada, &body).await?;
    assert_eq!(again, first);
    assert_eq!(table.list(&table.ada).await?.service_accounts.len(), 1);

    let retire = json!({ "operation": operation()? });
    let path = format!("/service-accounts/{}/retire", first.id);
    let (status, ended) = table.service.post(&path, Some(&table.ada), &retire).await?;
    assert_eq!(status, 200, "{ended}");
    let (status, ended_again) = table.service.post(&path, Some(&table.ada), &retire).await?;
    assert_eq!(status, 200, "{ended_again}");
    assert_eq!(
        ended_again, ended,
        "the same retirement answers what was recorded"
    );
    Ok(())
}

#[tokio::test]
async fn every_refusal_is_by_name() -> TestResult {
    let table = Table::set().await?;
    let bea_id = table.person(1)?;
    let op = operation()?;

    let unsigned = table
        .service
        .post(
            "/service-accounts",
            None,
            &json!({ "operation": op, "name": "docs deploy" }),
        )
        .await?;
    refused(&unsigned, 401, "NotSignedIn");
    let nameless = table
        .create(&table.ada, &json!({ "operation": op, "name": "  " }))
        .await?;
    refused(&nameless, 400, "RequestMalformed");
    let for_another = table
        .create(
            &table.ada,
            &json!({ "operation": op, "name": "docs deploy", "owner": bea_id }),
        )
        .await?;
    refused(&for_another, 403, "NotAdmitted");

    let kept = table
        .made(
            &table.ada,
            &json!({ "operation": op, "name": "docs deploy" }),
        )
        .await?;
    let other_words = table
        .create(
            &table.ada,
            &json!({ "operation": op, "name": "another name" }),
        )
        .await?;
    refused(&other_words, 409, "ServiceAccountReused");

    let path = format!("/service-accounts/{}/retire", kept.id);
    let by_another = table
        .service
        .post(
            &path,
            Some(&table.bea),
            &json!({ "operation": operation()? }),
        )
        .await?;
    refused(&by_another, 404, "ServiceAccountUnknown");
    let unknown = table
        .service
        .post(
            &format!("/service-accounts/{}/retire", operation()?),
            Some(&table.ada),
            &json!({ "operation": operation()? }),
        )
        .await?;
    refused(&unknown, 404, "ServiceAccountUnknown");
    let (status, ended) = table
        .service
        .post(
            &path,
            Some(&table.admin),
            &json!({ "operation": operation()? }),
        )
        .await?;
    assert_eq!(status, 200, "the administrator retires any: {ended}");
    let twice = table
        .service
        .post(
            &path,
            Some(&table.ada),
            &json!({ "operation": operation()? }),
        )
        .await?;
    refused(&twice, 409, "ServiceAccountRetired");
    Ok(())
}
