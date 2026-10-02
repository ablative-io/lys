//! A request for a relation that carries one action alone, approved under
//! the model Lys ships, issues a grant of exactly that action: the holder
//! may do it on that resource, and nothing beside it, on that resource or
//! any other. A second request for a second action issues a second grant,
//! and the two together still admit nothing a third action names.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity::grants::shipped_model;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const FAR: u64 = 4_102_444_800;

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) = Service::start_judging(&shipped_model(), None, |config| {
            Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)
        })
        .await?;
        let sign_in = |subject: &str| Login {
            subject: subject.to_owned(),
            email: "shared@example.test".to_owned(),
        };
        let ada = service.sign_in(sign_in(ADMINISTRATOR)).await?;
        let bea = service.sign_in(sign_in(BEA)).await?;
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
        })
    }

    /// Bea asks for `relation` on doc `resource`, Ada approves it, and the
    /// grant issued is answered as Bea reads it.
    async fn asked_and_approved(
        &self,
        resource: &str,
        relation: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let body = json!({
            "operation": operation()?,
            "resource": { "kind": "doc", "id": resource },
            "relation": relation,
            "ends_at": FAR,
            "why": "to stop the agent when it runs away",
        });
        let (status, asked) = self
            .service
            .post("/requests", Some(&self.bea), &body)
            .await?;
        assert_eq!(status, 200, "{asked}");
        let action = relation
            .strip_prefix("only.")
            .ok_or("not a one-action relation")?;
        assert_eq!(asked["actions"], json!([action]), "{asked}");
        let id = asked["id"].as_str().ok_or("no id")?;
        let approval =
            json!({ "operation": operation()?, "route": "api", "source": null, "note": "yes" });
        let (status, approved) = self
            .service
            .post(
                &format!("/requests/{id}/approve"),
                Some(&self.ada),
                &approval,
            )
            .await?;
        assert_eq!(status, 200, "{approved}");
        assert_eq!(approved["state"], "approved", "{approved}");
        let grant = approved["decision"]["grant"].as_str().ok_or("no grant")?;
        let (status, held) = self
            .service
            .get(&format!("/grants/{grant}"), Some(&self.bea))
            .await?;
        assert_eq!(status, 200, "{held}");
        Ok(held)
    }

    /// Whether Bea may do `action` on doc `resource`, as `/grants/check` answers.
    async fn may(&self, resource: &str, action: &str) -> Result<bool, Box<dyn Error>> {
        let question = json!({ "route": "api", "resource": { "kind": "doc", "id": resource }, "action": action });
        let (status, answer) = self
            .service
            .post("/grants/check", Some(&self.bea), &question)
            .await?;
        match status {
            200 => Ok(true),
            403 => {
                assert_eq!(
                    answer["refusal"], "NotHeld",
                    "{action} on {resource}: {answer}"
                );
                Ok(false)
            }
            _ => Err(format!("{action} on {resource} answered {status}: {answer}").into()),
        }
    }
}

#[tokio::test]
async fn an_approved_one_action_request_grants_that_action_and_nothing_wider() -> TestResult {
    let table = Table::set().await?;
    let bea = table.seeded.people[1].id.to_string();
    for action in ["agent.stop", "read", "view", "edit", "grant"] {
        assert!(
            !table.may("7", action).await?,
            "nothing is held before approval: {action}"
        );
    }

    let held = table.asked_and_approved("7", "only.agent.stop").await?;
    assert_eq!(held["holder"], bea);
    assert_eq!(held["resource"], json!({ "kind": "doc", "id": "7" }));
    assert_eq!(held["relation"], "only.agent.stop");
    assert_eq!(held["actions"], json!(["agent.stop"]));
    assert_eq!(held["pass_on"]["kind"], "use_only");
    assert_eq!(held["window"]["ends_at"], FAR);

    assert!(table.may("7", "agent.stop").await?);
    for action in [
        "read",
        "view",
        "edit",
        "grant",
        "agent.start",
        "agent.restart",
        "grant.delegate",
    ] {
        assert!(
            !table.may("7", action).await?,
            "approving agent.stop gave {action}"
        );
    }
    assert!(
        !table.may("8", "agent.stop").await?,
        "the grant reached another resource"
    );

    let second = table.asked_and_approved("7", "only.agent.start").await?;
    assert_ne!(
        second["id"], held["id"],
        "a second action is a second grant"
    );
    assert_eq!(second["actions"], json!(["agent.start"]));
    assert!(table.may("7", "agent.start").await?);
    assert!(table.may("7", "agent.stop").await?);
    for action in ["agent.restart", "read", "edit", "grant"] {
        assert!(
            !table.may("7", action).await?,
            "two one-action grants gave {action}"
        );
    }

    let (_, listed) = table.service.get("/grants", Some(&table.bea)).await?;
    assert_eq!(
        listed["grants"].as_array().map(Vec::len),
        Some(2),
        "{listed}"
    );
    Ok(())
}
