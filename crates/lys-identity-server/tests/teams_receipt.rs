//! Every act on a team answers the team as it stands beside the line its
//! operation was first kept as, so a retry reads what it did the first time
//! however the team has changed since.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
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

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

struct Table {
    service: Service,
    seeded: Seeded,
    bea: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let bea = service.sign_in(login(BEA)).await?;
        Ok(Self {
            service,
            seeded,
            bea,
        })
    }

    fn agent(&self) -> String {
        self.seeded.people[1].agents[0].id.to_string()
    }

    async fn sent(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(&self.bea), body).await?;
        if status != 200 {
            return Err(format!("{path} answered {status}: {answer}").into());
        }
        Ok(answer)
    }
}

#[tokio::test]
async fn each_act_answers_what_its_operation_was_kept_as() -> TestResult {
    let table = Table::set().await?;
    let made = operation()?;
    let team = table
        .sent("/teams", &json!({ "operation": made, "name": "screens" }))
        .await?;
    assert_eq!(team["id"], made);
    assert_eq!(team["recorded"]["operation"], made);
    assert_eq!(team["recorded"]["act"], "created");
    assert_eq!(team["recorded"]["member"], Value::Null);
    assert_eq!(team["recorded"]["at"], team["created_at"]);
    assert_eq!(team["recorded"]["by"], team["created_by"]);

    let agent = table.agent();
    let adding = operation()?;
    let members = format!("/teams/{made}/members");
    let added = table
        .sent(&members, &json!({ "operation": adding, "member": agent }))
        .await?;
    assert_eq!(added["members"], json!([agent]));
    assert_eq!(added["recorded"]["operation"], adding);
    assert_eq!(added["recorded"]["act"], "added");
    assert_eq!(added["recorded"]["member"], agent);

    let removing = operation()?;
    let removed = table
        .sent(
            &format!("/teams/{made}/members/{agent}/remove"),
            &json!({ "operation": removing }),
        )
        .await?;
    assert_eq!(removed["members"], json!([]));
    assert_eq!(removed["recorded"]["act"], "removed");
    assert_eq!(removed["recorded"]["member"], agent);

    let retiring = operation()?;
    let retired = table
        .sent(
            &format!("/teams/{made}/retire"),
            &json!({ "operation": retiring }),
        )
        .await?;
    assert_eq!(retired["state"], "retired");
    assert_eq!(retired["recorded"]["operation"], retiring);
    assert_eq!(retired["recorded"]["act"], "retired");
    assert_eq!(retired["recorded"]["member"], Value::Null);
    assert_eq!(retired["recorded"]["at"], retired["retired_at"]);
    Ok(())
}

#[tokio::test]
async fn a_retry_reads_its_first_act_beside_the_team_as_it_stands_now() -> TestResult {
    let table = Table::set().await?;
    let made = operation()?;
    table
        .sent("/teams", &json!({ "operation": made, "name": "gates" }))
        .await?;
    let agent = table.agent();
    let adding = operation()?;
    let members = format!("/teams/{made}/members");
    let add = json!({ "operation": adding, "member": agent });
    let first = table.sent(&members, &add).await?;
    table
        .sent(
            &format!("/teams/{made}/members/{agent}/remove"),
            &json!({ "operation": operation()? }),
        )
        .await?;

    let again = table.sent(&members, &add).await?;
    assert_eq!(again["members"], json!([]));
    assert_eq!(again["recorded"], first["recorded"]);
    assert_eq!(again["recorded"]["act"], "added");
    assert_eq!(again["recorded"]["member"], agent);

    let (status, seen) = table
        .service
        .get(&format!("/teams/{made}"), Some(&table.bea))
        .await?;
    assert_eq!(status, 200, "{seen}");
    assert_eq!(seen["members"], json!([]));
    assert_eq!(seen.get("recorded"), None);
    Ok(())
}
