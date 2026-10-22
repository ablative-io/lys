#![cfg(test)]
//! The delegation model reads the approved apps' current declarations.

use identity_contract::apps::{
    Auth, BEA, FILES, NOTES, TestResult, approve, get, login, ok, op, post, put, registration,
    seeded,
};
use identity_contract::harness::ADMINISTRATOR;
use identity_contract::harness::Service;
use serde_json::{Value, json};

#[tokio::test]
async fn the_grant_model_answers_both_approved_apps_and_only_their_current_kinds() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let caller = service.sign_in(login(BEA)).await?;
    let channel = format!("{NOTES}.channel");
    let document = format!("{FILES}.doc");
    let pending = "fixture_waiting";
    let pending_kind = format!("{pending}.channel");
    let notes = json!({"kinds": {&channel: {
        "actions": ["grant_delegate", "read", "send"],
        "agents": ["read", "send"],
        "relations": {
            "custodian": ["grant_delegate", "read", "send"],
            "participant": ["read", "send"]
        },
        "parents": []
    }}});
    let files = json!({"kinds": {&document: {
        "actions": ["delete", "read"],
        "agents": ["read"],
        "relations": {"editor": ["delete", "read"], "reader": ["read"]},
        "parents": []
    }}});
    let waiting = json!({"kinds": {&pending_kind: {
        "actions": ["read"], "agents": ["read"],
        "relations": {"reader": ["read"]}, "parents": []
    }}});
    for (app, schema) in [(NOTES, &notes), (FILES, &files), (pending, &waiting)] {
        ok(post(
            &service,
            "/apps",
            Auth::Cookie(&admin),
            &registration(app, schema)?,
        )
        .await?)?;
    }

    let before = ok(get(&service, "/grants/model", Auth::Cookie(&caller)).await?)?;
    assert_eq!(
        before["kinds"],
        json!({}),
        "pending apps declare no admitted kinds"
    );
    assert_eq!(before["version"], 1);
    assert_eq!(
        before["relations"],
        json!({"administrator": ["organisation.administer", "organisation.admit", "organisation.grant", "organisation.read"], "alpha": ["read", "write"], "beta": ["read"]})
    );
    approve(&service, &admin, NOTES).await?;
    approve(&service, &admin, FILES).await?;

    let model = ok(get(&service, "/grants/model", Auth::Cookie(&caller)).await?)?;
    assert_current_model(&service, &admin, &model, &channel, &document, &pending_kind).await?;

    let mut changed = notes;
    changed["kinds"][&channel]["agents"] = json!(["read"]);
    changed["kinds"][&channel]["relations"]["reader"] = json!(["read"]);
    let applied = ok(put(
        &service,
        &format!("/apps/{NOTES}/schema"),
        Auth::Cookie(&admin),
        &json!({"operation": op()?, "replaces": 1, "schema": changed}),
    )
    .await?)?;
    assert_eq!(applied["applied"], true, "{applied}");
    assert_eq!(applied["app"]["version"], 2, "{applied}");
    let current = ok(get(&service, "/grants/model", Auth::Cookie(&caller)).await?)?;
    assert_eq!(
        current["kinds"][&channel],
        json!({
            "agents": ["read"],
            "relations": {
                "custodian": ["grant_delegate", "read", "send"],
                "participant": ["read", "send"], "reader": ["read"]
            }
        }),
        "{current}"
    );
    assert_eq!(current["kinds"][&document], model["kinds"][&document]);
    assert_eq!(current["version"], before["version"]);
    assert_eq!(current["relations"], before["relations"]);

    ok(post(
        &service,
        &format!("/apps/{NOTES}/retire"),
        Auth::Cookie(&admin),
        &json!({"operation": op()?, "reason": "the app is retired"}),
    )
    .await?)?;
    let retired = ok(get(&service, "/grants/model", Auth::Cookie(&caller)).await?)?;
    assert_eq!(
        retired["kinds"],
        json!({&document: {
            "agents": ["read"],
            "relations": {"editor": ["delete", "read"], "reader": ["read"]}
        }}),
        "retired and pending apps supply no kinds: {retired}"
    );
    assert_eq!(retired["version"], before["version"]);
    assert_eq!(retired["relations"], before["relations"]);
    assert_eq!(get(&service, "/grants/model", Auth::Nobody).await?.0, 401);
    Ok(())
}

async fn assert_current_model(
    service: &Service,
    admin: &str,
    model: &Value,
    channel: &str,
    document: &str,
    pending_kind: &str,
) -> TestResult {
    assert_eq!(
        model["kinds"],
        json!({
            channel: {
                "agents": ["read", "send"],
                "relations": {
                    "custodian": ["grant_delegate", "read", "send"],
                    "participant": ["read", "send"]
                }
            },
            document: {
                "agents": ["read"],
                "relations": {"editor": ["delete", "read"], "reader": ["read"]}
            }
        }),
        "{model}"
    );
    assert!(model["kinds"].get(pending_kind).is_none(), "{model}");
    let withheld = model["withheld_from_agents"]
        .as_array()
        .ok_or("the model did not answer withheld_from_agents")?;
    // Only a person's credentials are withheld; authority may be given
    // (AGENTS-019 R1).
    assert!(withheld.contains(&json!("person.password.set")), "{model}");
    assert!(withheld.contains(&json!("person.sign-in.set")), "{model}");
    assert!(!withheld.contains(&json!("grant")), "{model}");
    // Passing on part of a grant held is an agent's to hold since AGENTS-015 R1.
    assert!(!withheld.contains(&json!("grant.delegate")), "{model}");
    for declared in model["kinds"]
        .as_object()
        .ok_or("the model did not answer kinds")?
        .values()
    {
        let agents = declared["agents"]
            .as_array()
            .ok_or("the kind did not answer agents")?;
        assert!(
            withheld.iter().all(|action| !agents.contains(action)),
            "a withheld action appeared among the agent actions: {declared}"
        );
    }
    for (app, kind) in [(NOTES, channel), (FILES, document)] {
        let registered = ok(get(service, &format!("/apps/{app}"), Auth::Cookie(admin)).await?)?;
        assert_eq!(
            model["kinds"][kind]["agents"],
            registered["schema"]["kinds"][kind]["agents"]
        );
        assert_eq!(
            model["kinds"][kind]["relations"],
            registered["schema"]["kinds"][kind]["relations"]
        );
    }

    Ok(())
}
