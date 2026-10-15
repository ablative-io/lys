//! Person reads page only the product drafts their person holds or answers for.

#[path = "../src/mcp_endpoint/person_test_support.rs"]
mod support;

use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::draft_event::Target;
use lys_identity::grants::{GrantId, Mode};
use lys_identity::product_draft_event::{
    Created, ProductDraftEvent, draft_operation, words_digest,
};
use lys_identity::{IdentityId, PersonId};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

struct Person {
    id: PersonId,
    cookie: String,
    drafts: BTreeSet<String>,
}

struct Table {
    service: Service,
    people: Vec<Person>,
}

impl Table {
    async fn start() -> Result<Self> {
        let (service, seeded) = Service::start_with(|config| {
            let seeded = identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, "other"],
            )?;
            let mut directory =
                identity_contract::lys_identity_server::routes::open_directory(config)?;
            let mut people = Vec::new();
            for person in seeded.people {
                let mut drafts = BTreeSet::new();
                for (holder, mode) in [
                    (IdentityId::Person(person.id), Mode::ByDraft),
                    (IdentityId::Agent(person.agents[0].id), Mode::ByTwo),
                ] {
                    let client_operation = "prepare-the-product-change".to_owned();
                    let operation = draft_operation(holder, &client_operation);
                    let words = format!("Prepared for {}", person.display_name);
                    directory.record_product_draft(ProductDraftEvent::Created(Arc::new(
                        Created {
                            operation,
                            client_operation,
                            holder,
                            responsible: person.id,
                            recorded_at: lys_identity_server::session::now(),
                            app: "fixture_notes".to_owned(),
                            grant: GrantId::generate()?,
                            mode,
                            target: Target {
                                kind: "fixture_notes.doc".to_owned(),
                                id: "1".to_owned(),
                                action: "write".to_owned(),
                            },
                            request_digest: words_digest(&words),
                            words,
                        },
                    )))?;
                    drafts.insert(operation.to_string());
                }
                people.push((person.id, person.subject, drafts));
            }
            Ok(people)
        })
        .await?;
        let mut people = Vec::new();
        for (id, subject, drafts) in seeded {
            let cookie = service
                .sign_in(identity_contract::apps::login(&subject))
                .await?;
            people.push(Person { id, cookie, drafts });
        }
        Ok(Self { service, people })
    }
}

#[tokio::test]
async fn person_connection_reads_own_raised_and_waiting_product_drafts_before_paging() -> Result {
    let table = Table::start().await?;
    let (status, browser) = table
        .service
        .get("/product-drafts?limit=50", Some(&table.people[0].cookie))
        .await?;
    assert_eq!(status, 200);
    assert_eq!(
        browser["total"], 4,
        "the administrator browser fixture must include both people"
    );
    for person in &table.people {
        let (_, tokens) =
            support::connected_mode(&table.service, &person.cookie, Some("person_reads")).await?;
        let mut path = "/product-drafts?limit=1".to_owned();
        let mut seen = BTreeSet::new();
        loop {
            let answer =
                support::call(&table.service, &tokens, "read", json!({"path":path})).await?;
            assert_eq!(answer["status"], 200, "own product-draft read was refused");
            assert_eq!(
                answer["body"]["total"],
                person.drafts.len(),
                "paging counted another person's drafts"
            );
            let rows = answer["body"]["drafts"]
                .as_array()
                .ok_or("no product drafts")?;
            assert_eq!(rows.len(), 1, "filtering left an empty or oversized page");
            let row = &rows[0];
            let id = row["id"].as_str().ok_or("no product draft id")?;
            assert!(
                person.drafts.contains(id),
                "another person's product draft escaped"
            );
            assert!(
                seen.insert(id.to_owned()),
                "a product draft was paged twice"
            );
            assert_eq!(row["responsible"], person.id.to_string());
            assert_eq!(row["state"], "waiting");
            assert_eq!(answer["receipt"]["read"]["person"], person.id.to_string());
            let kept: Value =
                serde_json::from_slice(&support::unhex(&answer["receipt"]["message"])?)?;
            assert_eq!(kept["read"]["person"], person.id.to_string());
            if let Some(cursor) = answer["body"]["next"].as_str() {
                path = format!("/product-drafts?limit=1&after={cursor}");
            } else {
                break;
            }
        }
        assert_eq!(seen, person.drafts);
    }
    let (_, agent) = support::connected(&table.service, &table.people[0].cookie).await?;
    let refused = support::call(
        &table.service,
        &agent,
        "read",
        json!({"path":"/product-drafts"}),
    )
    .await?;
    assert_ne!(
        refused["status"], 200,
        "agent mode gained a person's product drafts"
    );
    Ok(())
}

#[tokio::test]
async fn person_connection_product_draft_decisions_still_need_the_browser() -> Result {
    let table = Table::start().await?;
    let person = &table.people[0];
    let (_, tokens) =
        support::connected_mode(&table.service, &person.cookie, Some("person_reads")).await?;
    let draft = person.drafts.first().ok_or("no own product draft")?;
    let before = table.service.log_size().await?;
    for tool in ["read", "change"] {
        for decision in ["approve", "refuse"] {
            let answer = support::call(
                &table.service,
                &tokens,
                tool,
                json!({
                    "method":"POST","path":format!("/product-drafts/{draft}/{decision}"),"body":{}
                }),
            )
            .await?;
            assert_eq!(answer["status"], 403);
            assert_eq!(answer["body"]["refusal"], "PersonPresenceRequired");
            assert_eq!(
                answer["body"]["reason"],
                format!(
                    "deciding a draft needs you in Lys: {}/#/access/drafts?draft={draft}",
                    table.service.base
                )
            );
        }
    }
    assert_eq!(
        table.service.log_size().await?,
        before,
        "a personal decision reached the directory"
    );
    Ok(())
}
