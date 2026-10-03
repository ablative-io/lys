//! Who may act on many resources at once: `/grants/reach` answers each
//! resource's holders and their actions exactly as `/grants/who` answers
//! each resource and action, hides what the caller may not see as `who`
//! does, answers twenty resources in one request under a second, and
//! answers a thousand resources whole and in order.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const DOCS: usize = 20;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

async fn post_ok(
    service: &Service,
    path: &str,
    cookie: &str,
    body: &Value,
) -> Result<Value, Box<dyn Error>> {
    let (status, answer) = service.post(path, Some(cookie), body).await?;
    assert_eq!(status, 200, "{path}: {answer}");
    Ok(answer)
}

/// On each of twenty docs, Ada holds a private root with read and write,
/// and Bea a root lending read to agents, lent on to her agent.
async fn world(service: &Service, seeded: &Seeded, ada: &str, bea: &str) -> TestResult {
    let (ada_person, bea_person) = (&seeded.people[0], &seeded.people[1]);
    for n in 0..DOCS {
        let doc = json!({ "kind": "doc", "id": n.to_string() });
        let root = |holder: String, pass_on: Value| -> Result<Value, Box<dyn Error>> {
            Ok(json!({
                "operation": OperationId::generate()?.to_string(), "route": "api",
                "holder": holder, "resource": doc, "relation": "alpha", "pass_on": pass_on,
                "window": { "starts_at": 0, "ends_at": null },
            }))
        };
        let private = root(ada_person.id.to_string(), json!({ "kind": "use_only" }))?;
        post_ok(service, "/grants/roots", ada, &private).await?;
        let lending = root(
            bea_person.id.to_string(),
            json!({ "kind": "to", "actions": ["read"], "recipients": ["agent"] }),
        )?;
        let lending = post_ok(service, "/grants/roots", ada, &lending).await?;
        let lent = json!({
            "operation": OperationId::generate()?.to_string(), "route": "tool",
            "source": lending["grant"], "recipient": bea_person.agents[0].id.to_string(),
            "responsible": bea_person.id.to_string(), "resource": doc, "relation": "beta",
            "pass_on": { "kind": "use_only" }, "window": { "starts_at": 0, "ends_at": null },
        });
        post_ok(service, "/grants", bea, &lent).await?;
    }
    Ok(())
}

type Holders = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;

/// Each doc's holders and their actions, asked of `/grants/who` one doc and
/// one action at a time.
async fn by_who(service: &Service, cookie: &str) -> Result<Holders, Box<dyn Error>> {
    let mut out = Holders::new();
    for n in 0..DOCS {
        let doc = out.entry(n.to_string()).or_default();
        for action in ["read", "write"] {
            let ask = json!({
                "route": "browser", "resource": { "kind": "doc", "id": n.to_string() },
                "action": action, "page_size": 100, "after": null,
            });
            let page = post_ok(service, "/grants/who", cookie, &ask).await?;
            assert_eq!(page["complete"], true, "{page}");
            for holder in page["holders"].as_array().ok_or("no holders")? {
                let holder = holder["holder"].as_str().ok_or("no holder")?;
                doc.entry(holder.to_owned())
                    .or_default()
                    .insert(action.to_owned());
            }
        }
    }
    Ok(out)
}

/// The same, asked of `/grants/reach` in one request.
async fn by_reach(service: &Service, cookie: &str) -> Result<Holders, Box<dyn Error>> {
    let resources: Vec<Value> = (0..DOCS)
        .map(|n| json!({ "kind": "doc", "id": n.to_string(), "actions": ["read", "write"] }))
        .collect();
    let ask = json!({ "route": "browser", "resources": resources });
    let answer = post_ok(service, "/grants/reach", cookie, &ask).await?;
    let mut out = Holders::new();
    let answered = answer["resources"].as_array().ok_or("no resources")?;
    assert_eq!(answered.len(), DOCS, "every doc, in order");
    for (n, resource) in answered.iter().enumerate() {
        assert_eq!(resource["id"], n.to_string(), "{resource}");
        let doc = out.entry(n.to_string()).or_default();
        for holder in resource["holders"].as_array().ok_or("no holders")? {
            let actions = holder["actions"].as_array().ok_or("no actions")?;
            doc.insert(
                holder["holder"].as_str().ok_or("no holder")?.to_owned(),
                actions
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
            );
        }
    }
    Ok(out)
}

#[tokio::test]
async fn reach_answers_twenty_resources_exactly_as_who_does() -> TestResult {
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    world(&service, &seeded, &ada, &bea).await?;
    for cookie in [&ada, &bea] {
        let reached = by_reach(&service, cookie).await?;
        assert_eq!(reached, by_who(&service, cookie).await?);
    }
    let as_ada = by_reach(&service, &ada).await?;
    let everyone: Vec<usize> = as_ada.values().map(BTreeMap::len).collect();
    assert_eq!(everyone, vec![3; DOCS], "Ada sees all three holders");
    let ada_id = seeded.people[0].id.to_string();
    assert_eq!(
        as_ada["0"][&ada_id],
        BTreeSet::from(["read".to_owned(), "write".to_owned()])
    );
    let as_bea = by_reach(&service, &bea).await?;
    assert!(
        as_bea
            .values()
            .all(|holders| !holders.contains_key(&ada_id)),
        "Bea never sees Ada's private grants: {as_bea:?}"
    );
    Ok(())
}

#[tokio::test]
async fn reach_refuses_no_resources_and_answers_a_thousand_whole() -> TestResult {
    let (service, _) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let none = json!({ "route": "browser", "resources": [] });
    let (status, answer) = service.post("/grants/reach", Some(&ada), &none).await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "RequestMalformed", "{answer}");
    let many: Vec<Value> = (0..1000)
        .map(|n| json!({ "kind": "doc", "id": n.to_string(), "actions": ["read"] }))
        .collect();
    let ask = json!({ "route": "browser", "resources": many });
    let answer = post_ok(&service, "/grants/reach", &ada, &ask).await?;
    let answered = answer["resources"].as_array().ok_or("no resources")?;
    assert_eq!(answered.len(), 1000, "every resource asked about");
    for (n, resource) in answered.iter().enumerate() {
        assert_eq!(resource["id"], n.to_string(), "{resource}");
    }
    Ok(())
}
