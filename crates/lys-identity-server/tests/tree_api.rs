//! The tree is scoped to the caller and reports absence without inventing state.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "operator@example.test".to_owned(),
    }
}

#[tokio::test]
async fn the_tree_shows_only_owned_teams_and_never_turns_people_into_members() -> TestResult {
    let other = "another-subject";
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, other])?)).await?;
    let own_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let other_cookie = service.sign_in(login(other)).await?;
    let own = OperationId::generate()?.to_string();
    let elsewhere = OperationId::generate()?.to_string();
    for (id, cookie) in [(&own, &own_cookie), (&elsewhere, &other_cookie)] {
        let (status, answer) = service
            .post(
                "/teams",
                Some(cookie),
                &json!({
                    "operation": id, "name": "owned team",
                }),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
    }
    let agent = seeded.people[0].agents[0].id.to_string();
    let person = seeded.people[0].id.to_string();
    for member in [&agent, &person] {
        let (status, answer) = service
            .post(
                &format!("/teams/{own}/members"),
                Some(&own_cookie),
                &json!({
                    "operation": OperationId::generate()?.to_string(), "member": member,
                }),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
    }
    let (status, answer) = service.get("/tree", Some(&own_cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["root"]["id"], person);
    let teams = answer["teams"].as_array().ok_or("tree has no teams")?;
    assert_eq!(teams.len(), 1, "{answer}");
    assert_eq!(teams[0]["id"], own);
    assert_eq!(teams[0]["lead"], Value::Null);
    let members = teams[0]["members"]
        .as_array()
        .ok_or("team has no members")?;
    assert_eq!(members.len(), 1, "people are not tree nodes: {answer}");
    assert_eq!(members[0]["id"], agent);
    assert_eq!(members[0]["session"], "stopped");
    assert_eq!(members[0]["profile"], Value::Null);
    assert_eq!(members[0]["goals"], json!([]));
    assert_eq!(members[0]["budgets"], json!([]));
    assert_eq!(teams[0]["teams"], json!([]));
    let (status, other_tree) = service.get("/tree", Some(&other_cookie)).await?;
    assert_eq!(status, 200, "{other_tree}");
    assert_eq!(
        other_tree["teams"]
            .as_array()
            .ok_or("no other teams")?
            .len(),
        1
    );
    assert_eq!(other_tree["teams"][0]["id"], elsewhere);
    Ok(())
}

#[tokio::test]
async fn the_tree_requires_authentication() -> TestResult {
    let (service, seeded) = Service::start_with(|config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "another-subject"])?)
    })
    .await?;
    assert_eq!(seeded.people.len(), 2);
    let (status, answer) = service.get("/tree", None).await?;
    assert_eq!(status, 401, "{answer}");
    assert_eq!(answer["refusal"], "NotSignedIn");
    Ok(())
}

#[tokio::test]
async fn an_owned_team_tree_names_the_missing_roles_configuration() -> TestResult {
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.roles_file = None,
        |config| {
            seed_configured(config, [ADMINISTRATOR, "another-subject"])?;
            Ok(())
        },
    )
    .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    post(
        &service,
        &cookie,
        "/teams",
        json!({"operation": operation()?, "name": "owned team"}),
    )
    .await?;
    let (status, answer) = service.get("/tree", Some(&cookie)).await?;
    assert_eq!(status, 503, "{answer}");
    assert_eq!(answer["refusal"], "RolesUnavailable");
    assert!(
        answer["reason"]
            .as_str()
            .ok_or("no refusal reason")?
            .contains("the configuration names no roles_file"),
        "{answer}"
    );
    Ok(())
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

async fn post(
    service: &Service,
    cookie: &str,
    path: &str,
    body: Value,
) -> Result<Value, Box<dyn Error>> {
    let (status, answer) = service.post(path, Some(cookie), &body).await?;
    assert_eq!(status, 200, "{path}: {answer}");
    Ok(answer)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 0x0f)],
            ]
        })
        .map(char::from)
        .collect()
}

async fn agent_tree(
    service: &Service,
    agent: &str,
    key: &lys_core::Ed25519Identity,
    nonce: u8,
) -> Result<(u16, Value), Box<dyn Error>> {
    use lys_core::attestation::sign_attestation;
    use lys_identity_server::agent_signature::{HEADER, payload};
    use std::time::{SystemTime, UNIX_EPOCH};

    let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let nonce = hex(&[nonce; 16]);
    let cose = sign_attestation(&payload("GET", "/tree", &[], at, &nonce), key).to_cose_bytes();
    let response = reqwest::Client::new()
        .get(format!("{}/tree", service.base))
        .header(HEADER, format!("{agent} {at} {nonce} {}", hex(&cose)))
        .send()
        .await?;
    let status = response.status().as_u16();
    let answer = serde_json::from_slice(&response.bytes().await?)?;
    Ok((status, answer))
}

#[tokio::test]
async fn an_agent_lead_reads_descendants_and_other_members_without_gaining_mutation_authority()
-> TestResult {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use lys_core::Ed25519Identity;
    use lys_core::ca::create_certificate_request;
    use std::sync::Arc;

    let other = "another-subject";
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, other])?)).await?;
    let owner = service.sign_in(login(ADMINISTRATOR)).await?;
    let other_cookie = service.sign_in(login(other)).await?;
    let root = operation()?;
    let middle = operation()?;
    let child = operation()?;
    let outside = operation()?;
    for (team, parent) in [
        (&root, None),
        (&middle, Some(root.as_str())),
        (&child, Some(middle.as_str())),
        (&outside, None),
    ] {
        post(
            &service,
            &owner,
            "/teams",
            json!({"operation": team, "name": "team", "parent": parent}),
        )
        .await?;
    }
    let lead = seeded.people[1].agents[0].id.to_string();
    let member = seeded.people[0].agents[0].id.to_string();
    for (team, agent) in [
        (&middle, &lead),
        (&middle, &member),
        (&child, &member),
        (&outside, &member),
    ] {
        post(
            &service,
            &owner,
            &format!("/teams/{team}/members"),
            json!({"operation": operation()?, "member":agent}),
        )
        .await?;
    }
    post(
        &service,
        &owner,
        &format!("/teams/{middle}/nesting"),
        json!({"operation": operation()?, "parent":root,"lead":lead}),
    )
    .await?;
    post(&service, &owner, &format!("/teams/{child}/goals"), json!({"operation": operation()?, "kind":"goal", "words":"the child's current goal", "deadline":u64::MAX})).await?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &service.dir.path().join("agent.key"),
    )?);
    post(&service, &other_cookie, &format!("/agents/{lead}/certificates"), json!({"operation":operation()?, "request":STANDARD.encode(create_certificate_request(&key,&lead)?)})).await?;
    let (status, answer) = agent_tree(&service, &lead, &key, 9).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["root"]["id"], lead);
    assert_eq!(answer["teams"].as_array().map(Vec::len), Some(1));
    assert_eq!(answer["teams"][0]["id"], middle);
    assert_eq!(answer["teams"][0]["lead"]["id"], lead);
    assert_eq!(
        answer["teams"][0]["members"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(answer["teams"][0]["teams"][0]["id"], child);
    assert_eq!(
        answer["teams"][0]["teams"][0]["members"][0]["goals"],
        json!(["the child's current goal"])
    );
    let (status, refused) = service
        .post(
            &format!("/teams/{middle}/nesting"),
            Some(&other_cookie),
            &json!({"operation":operation()?,"parent":null,"lead":null}),
        )
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted");
    post(
        &service,
        &owner,
        &format!("/teams/{middle}/members/{lead}/remove"),
        json!({"operation":operation()?}),
    )
    .await?;
    let (status, answer) = agent_tree(&service, &lead, &key, 10).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["teams"], json!([]));
    Ok(())
}

#[tokio::test]
async fn summaries_use_reviewed_versions_reported_sessions_and_every_applicable_budget()
-> TestResult {
    use lys_core::Ed25519Identity;
    use lys_identity_server::budgets_state::{
        Act, Budget, Holder, HolderKind, Length, Measure, Period, Usage,
    };
    use lys_identity_server::budgets_store::BudgetStore;
    use lys_identity_server::provisioning_store::{ProvisioningStore, Review, Version};
    use lys_identity_server::runtime_state::{Report, Reported};
    use lys_identity_server::runtime_store::RuntimeStore;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    let team = operation()?;
    let (mut service, (seeded, reviewed)) = Service::start_with(|config| {
        let seeded = seed_configured(config,[ADMINISTRATOR,"another-subject"])?;
        let agent = seeded.people[0].agents[0].id.to_string();
        let other = seeded.people[1].agents[0].id.to_string();
        let person = seeded.people[0].id.to_string();
        let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
        let mut profiles = ProvisioningStore::open(config.provisioning_file.as_deref().ok_or("no profiles file")?)?;
        let description = json!({"models":{"minimum":0,"maximum":null,"further_encoding":{"kind":"array"}},
            "permissions":{"modes":[],"rule_forms":[]}, "mcp":{"transports":[],"working_directory":false,"handle_variables":false,"channel_policies":[]},
            "rendering_contract":"recorded/template-v1"});
        let first: Version = serde_json::from_value(json!({"number":1,"operation":operation()?,"set_by":person,"set_at":1,
            "settings":{"model_access":["primary-model"],"tools":[],"skills":[],"mcp_servers":[{"name":"kept-server","url":"https://server.invalid/mcp"}],
                "instructions":"private instructions","note":"","harness":{"name":"declared build","description":description,"program":"/opt/seat/bin/program","package":"pinned-build"}}}))?;
        let reviewed = first.operation.clone();
        assert_eq!(profiles.set(&agent,0,first.clone())?,1);
        profiles.review(&agent,1,Review {operation:operation()?,by:person.clone(),at:2})?;
        let second = Version {number:2, operation:operation()?, settings:lys_identity_server::provisioning_store::Settings {model_access:vec!["unreviewed-model".to_owned()], ..first.settings}, ..first};
        assert_eq!(profiles.set(&agent,1,second)?,2);
        let empty: Version = serde_json::from_value(json!({"number":1,"operation":operation()?,"set_by":person,"set_at":1,
            "settings":{"model_access":[],"tools":[],"skills":[],"mcp_servers":[],"instructions":"","note":""}}))?;
        assert_eq!(profiles.set(&other,0,empty)?,1);
        profiles.review(&other,1,Review {operation:operation()?,by:person.clone(),at:2})?;
        let mut runtime = RuntimeStore::open(config.runtime_dir.as_deref().ok_or("no runtime directory")?,Arc::clone(&key))?;
        for (id, session) in [(&agent,"running-session"),(&other,"unconfirmed-session")] {
            runtime.report(Report {operation:operation()?,session:session.to_owned(),agent:Some(id.clone()),machine:"recorded-machine".to_owned(),state:Reported::Starting,
                what:String::new(),confirmation:String::new(),reported_by:person.clone(),at:1,launch:None})?;
        }
        runtime.report(Report {operation:operation()?,session:"running-session".to_owned(),agent:Some(agent.clone()),machine:"recorded-machine".to_owned(),state:Reported::Running,
            what:String::new(),confirmation:String::new(),reported_by:person.clone(),at:2,launch:None})?;
        let mut budgets = BudgetStore::open(config.budgets_dir.as_deref().ok_or("no budgets directory")?,key)?;
        for (kind,id,measure,limit) in [(HolderKind::Agent,agent.clone(),Measure::Tokens,1000), (HolderKind::Person,person.clone(),Measure::RunningMs,9000),
            (HolderKind::Agent,agent.clone(),Measure::ContextPercent,80), (HolderKind::Agent,other,Measure::Tokens,300),
            (HolderKind::Team,team.clone(),Measure::Tokens,2000)] {
            budgets.set_confirmed(Budget {holder:Holder {kind,id},measure,limit,period:if measure == Measure::ContextPercent {None} else {Some(Period {length:Length::Week,zone:"UTC".to_owned()})},
                act:Act::Tell,version:1,by:person.clone(),at:1},0)?;
        }
        let at_ms = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        assert!(budgets.charge(Usage {event:operation()?,agent:agent.clone(),at_ms,tokens:42,running_ms:77,session:Some("running-session".to_owned()),context_percent:Some(55),crossed:Vec::new(),..Usage::default()})?);
        assert!(budgets.charge(Usage {event:operation()?,agent,at_ms,tokens:0,running_ms:0,session:Some("running-session".to_owned()),context_percent:Some(35),crossed:Vec::new(),..Usage::default()})?);
        Ok((seeded,reviewed))
    }).await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let agent = seeded.people[0].agents[0].id.to_string();
    let other = seeded.people[1].agents[0].id.to_string();
    post(
        &service,
        &cookie,
        "/teams",
        json!({"operation":team,"name":"team"}),
    )
    .await?;
    for member in [&agent, &other] {
        post(
            &service,
            &cookie,
            &format!("/teams/{team}/members"),
            json!({"operation":operation()?,"member":member}),
        )
        .await?;
    }
    for words in ["open goal", "closed goal"] {
        let goal = operation()?;
        post(
            &service,
            &cookie,
            &format!("/agents/{agent}/goals"),
            json!({"operation":goal,"kind":"goal","words":words,"deadline":u64::MAX}),
        )
        .await?;
        if words == "closed goal" {
            post(
                &service,
                &cookie,
                &format!("/goals/{goal}/mark"),
                json!({"operation":operation()?,"standing":"dropped","words":"closed"}),
            )
            .await?;
        }
    }
    let (status, answer) = service.get("/tree", Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    let member = &answer["teams"][0]["members"][0];
    assert_eq!(member["session"], "live");
    assert_eq!(answer["teams"][0]["members"][1]["session"], "unconfirmed");
    assert_eq!(member["goals"], json!(["open goal"]));
    let other_profile = &answer["teams"][0]["members"][1]["profile"];
    assert!(other_profile.is_object());
    for field in ["harness", "program", "model", "writable"] {
        assert_eq!(other_profile[field], Value::Null);
    }
    assert_eq!(member["profile"]["version"], reviewed);
    assert_eq!(member["profile"]["model"], "primary-model");
    assert_eq!(member["profile"]["harness"], "declared build");
    assert_eq!(member["profile"]["program"], "/opt/seat/bin/program");
    assert_eq!(member["profile"]["mcp_servers"], json!(["kept-server"]));
    assert_eq!(member["profile"]["writable"], Value::Null);
    let budgets = member["budgets"].as_array().ok_or("no budgets")?;
    assert_eq!(budgets.len(), 4);
    let team_budget = budgets
        .iter()
        .find(|budget| budget["holder"]["kind"] == "team")
        .ok_or("no team budget")?;
    assert_eq!(team_budget["holder"]["id"], team);
    assert_eq!(team_budget["spent"], 42);
    for (measure, spent) in [("tokens", 42), ("running_ms", 77), ("context_percent", 35)] {
        let budget = budgets
            .iter()
            .find(|budget| budget["measure"] == measure)
            .ok_or("missing measure")?;
        assert_eq!(budget["spent"], spent);
    }
    assert_eq!(answer["teams"][0]["members"][1]["budgets"][0]["spent"], 42);
    assert!(
        !member
            .as_object()
            .ok_or("no member object")?
            .contains_key("instructions")
    );
    service.restart().await?;
    assert_eq!(service.get("/tree", Some(&cookie)).await?.1, answer);
    Ok(())
}
