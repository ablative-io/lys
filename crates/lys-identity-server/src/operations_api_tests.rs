#![cfg(test)]
//! `GET /operations/{operation}` (DIRECTORY-101 R2): whether Lys recorded a
//! change the caller made, read from the records that already carry operation
//! ids; another person's operation is refused, an unknown one is not recorded.

use identity_contract::apps::{Auth, get, login, ok, op, post};
use identity_contract::harness::{ADMINISTRATOR, Service};
use identity_contract::lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const BEA: &str = "bea-subject";

/// Ada, the administrator and root authority, and Bea, each signed in.
struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
}

impl Table {
    async fn set() -> Result<Self> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
        })
    }

    fn id(&self, person: usize) -> String {
        self.seeded.people[person].id.to_string()
    }

    async fn asked(&self, cookie: &str, operation: &str) -> Result<(u16, Value)> {
        get(
            &self.service,
            &format!("/operations/{operation}"),
            Auth::Cookie(cookie),
        )
        .await
    }
}

#[tokio::test]
async fn a_give_is_recorded_for_its_giver_refused_to_its_recipient_and_unknown_is_not_recorded()
-> Result {
    let table = Table::set().await?;
    let operation = op()?;
    let given = ok(post(
        &table.service,
        "/grants/roots",
        Auth::Cookie(&table.ada),
        &json!({
            "operation": operation, "route": "api", "holder": table.id(1),
            "resource": {"kind": "doc", "id": "quarter"}, "relation": "beta",
            "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null},
        }),
    )
    .await?)?;

    let answer = ok(table.asked(&table.ada, &operation).await?)?;
    assert_eq!(
        answer,
        json!({"recorded": true, "kind": "grant", "id": given["grant"]})
    );

    let (status, refused) = table.asked(&table.bea, &operation).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted", "{refused}");
    assert!(
        refused.get("kind").is_none() && refused.get("id").is_none(),
        "nothing of the record is said: {refused}"
    );

    let unknown = ok(table.asked(&table.ada, &op()?).await?)?;
    assert_eq!(unknown, json!({"recorded": false}));
    Ok(())
}

#[tokio::test]
async fn a_request_is_its_askers_and_an_approval_its_deciders() -> Result {
    let table = Table::set().await?;
    let request = op()?;
    ok(post(
        &table.service,
        "/requests",
        Auth::Cookie(&table.bea),
        &json!({
            "operation": request, "resource": {"kind": "doc", "id": "quarter"},
            "relation": "beta", "ends_at": null, "why": "to read the quarter's figures",
        }),
    )
    .await?)?;
    let asked = ok(table.asked(&table.bea, &request).await?)?;
    assert_eq!(
        asked,
        json!({"recorded": true, "kind": "request", "id": request})
    );
    let (status, refused) = table.asked(&table.ada, &request).await?;
    assert_eq!(
        (status, refused["refusal"].clone()),
        (403, json!("NotAdmitted")),
        "{refused}"
    );

    let decision = op()?;
    ok(post(
        &table.service,
        &format!("/requests/{request}/approve"),
        Auth::Cookie(&table.ada),
        &json!({
            "operation": decision, "route": "api", "source": null, "note": "for the quarter",
        }),
    )
    .await?)?;
    let decided = ok(table.asked(&table.ada, &decision).await?)?;
    assert_eq!(
        decided,
        json!({"recorded": true, "kind": "decision", "id": request})
    );
    let (status, refused) = table.asked(&table.bea, &decision).await?;
    assert_eq!(
        (status, refused["refusal"].clone()),
        (403, json!("NotAdmitted")),
        "{refused}"
    );
    Ok(())
}

#[tokio::test]
async fn a_job_made_is_its_makers_role_change() -> Result {
    let table = Table::set().await?;
    let operation = op()?;
    ok(post(&table.service, "/roles", Auth::Cookie(&table.ada), &json!({
        "operation": operation, "name": "Builder", "responsibilities": "Builds what the brief says.",
        "goals": "Green gates.", "practice": "Reads the brief first.", "profile": "builder.md",
        "grant_templates": [], "note": "The first version.",
    })).await?)?;
    let made = ok(table.asked(&table.ada, &operation).await?)?;
    assert_eq!(
        made,
        json!({"recorded": true, "kind": "role_change", "id": operation})
    );
    Ok(())
}

#[tokio::test]
async fn operations_are_for_a_signed_in_caller_and_a_malformed_id_is_refused() -> Result {
    let table = Table::set().await?;
    let (status, answer) = get(
        &table.service,
        &format!("/operations/{}", op()?),
        Auth::Nobody,
    )
    .await?;
    assert_eq!(status, 401, "{answer}");
    let (status, answer) = table.asked(&table.ada, "not-an-operation").await?;
    assert_eq!(status, 400, "{answer}");
    Ok(())
}
