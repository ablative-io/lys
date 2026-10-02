//! The administrator's setup roots can give an agent access, and no grant,
//! from them or any other, gives an agent an act withheld from agents.

use std::error::Error;

use identity_contract::apps::{Auth, get, login, op, post};
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::grants::{WITHHELD_FROM_AGENTS, shipped_model};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn ok((status, answer): (u16, Value)) -> Result<Value, Box<dyn Error>> {
    if status != 200 {
        return Err(format!("status {status}: {answer}").into());
    }
    Ok(answer)
}

/// A service on the shipped model whose administrator has finished setup,
/// with their signed-in cookie and person.
async fn set_up() -> Result<(Service, String, String), Box<dyn Error>> {
    let service = Service::start_adjusted(&shipped_model(), None, None, None, |_| {}, |_| Ok(()))
        .await?
        .0;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = ok(post(
        &service,
        "/setup",
        Auth::Cookie(&cookie),
        &json!({"operation": op()?, "display_name": "Administrator"}),
    )
    .await?)?;
    let person = person["person"].as_str().ok_or("no person")?.to_owned();
    Ok((service, cookie, person))
}

/// The administrator's grants passable to an agent.
async fn agent_roots(service: &Service, cookie: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let grants = ok(get(service, "/grants", Auth::Cookie(cookie)).await?)?;
    Ok(grants["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .filter(|grant| {
            grant["pass_on"]["recipients"]
                .as_array()
                .is_some_and(|recipients| recipients.contains(&json!("agent")))
        })
        .cloned()
        .collect())
}

#[tokio::test]
async fn setup_gives_the_administrator_roots_an_agent_can_be_given_access_through() -> TestResult
{
    let (service, cookie, person) = set_up().await?;
    let auth = Auth::Cookie(&cookie);
    let roots = agent_roots(&service, &cookie).await?;
    assert_eq!(roots.len(), 2, "{roots:?}");
    for root in &roots {
        assert_eq!(root["holder"], person.as_str());
        let passable = root["pass_on"]["actions"].as_array().ok_or("no actions")?;
        assert!(passable.contains(&json!("agent.start")), "{root}");
        for withheld in WITHHELD_FROM_AGENTS {
            assert!(!passable.contains(&json!(withheld)), "{withheld} in {root}");
        }
    }
    let again = ok(post(&service, "/grants/agent-roots", auth, &json!({})).await?)?;
    let mut ids: Vec<&str> = roots.iter().filter_map(|root| root["id"].as_str()).collect();
    ids.sort_unstable();
    let mut repeated: Vec<&str> = again["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    repeated.sort_unstable();
    assert_eq!(repeated, ids, "asking again records nothing new");

    let agent = ok(post(
        &service,
        "/agents",
        auth,
        &json!({"operation": op()?, "display_name": "Helper"}),
    )
    .await?)?;
    let agent = agent["agent"].as_str().ok_or("no agent")?.to_owned();
    ok(post(
        &service,
        &format!("/identities/{agent}/transitions"),
        auth,
        &json!({"operation": op()?, "transition": "activate", "reason": ""}),
    )
    .await?)?;
    let source = roots
        .iter()
        .find(|root| root["resource"]["id"] == "agents")
        .ok_or("no agents root")?;
    let give = |relation: &str, pass_on: Value| -> Result<Value, Box<dyn Error>> {
        Ok(json!({"operation": op()?, "route": "api", "source": source["id"], "recipient": agent, "responsible": person, "resource": source["resource"], "relation": relation, "pass_on": pass_on, "window": {"starts_at": 0, "ends_at": null}}))
    };
    let use_only = json!({"kind": "use_only"});
    ok(post(&service, "/grants", auth, &give("only.agent.start", use_only.clone())?).await?)?;

    // The roots pass on nothing withheld, so editor is outside them.
    let (status, refused) = post(&service, "/grants", auth, &give("editor", use_only.clone())?).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "ActionsOutside", "{refused}");

    // A root that does pass withheld acts on to agents still cannot give them.
    let wide = ok(post(
        &service,
        "/grants/roots",
        auth,
        &json!({"operation": op()?, "route": "api", "holder": person, "resource": source["resource"], "relation": "editor", "pass_on": {"kind": "to", "actions": ["agent.stop", "grant.delegate", "request.approve", "role.create"], "recipients": ["agent"]}, "window": {"starts_at": 0, "ends_at": null}}),
    )
    .await?)?;
    let give = |relation: &str, pass_on: Value| -> Result<Value, Box<dyn Error>> {
        Ok(json!({"operation": op()?, "route": "api", "source": wide["grant"], "recipient": agent, "responsible": person, "resource": source["resource"], "relation": relation, "pass_on": pass_on, "window": {"starts_at": 0, "ends_at": null}}))
    };
    let before = service.log_size().await?;
    for (relation, pass_on) in [
        ("only.grant.delegate", use_only.clone()),
        ("only.role.create", use_only),
        (
            "only.agent.stop",
            json!({"kind": "to", "actions": ["agent.stop", "request.approve"], "recipients": ["agent"]}),
        ),
    ] {
        let (status, refused) = post(&service, "/grants", auth, &give(relation, pass_on)?).await?;
        assert_eq!(status, 403, "{relation}: {refused}");
        assert_eq!(refused["refusal"], "WithheldFromAgents", "{relation}: {refused}");
    }
    assert_eq!(service.log_size().await?, before, "a refusal writes nothing");
    Ok(())
}
