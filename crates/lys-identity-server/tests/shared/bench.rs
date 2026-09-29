#![cfg(test)]
//! The bench's one scenario, run by `tests/apps_schema.rs` on the in-process
//! permission engine and by `tests/identity_spicedb.rs` on a live `SpiceDB`:
//! a draft asked "may X do Y to Z" answers as the real check answers once
//! the draft is registered, placed and granted, no standing check sees the
//! draft, and closing the bench removes its namespace.

use identity_contract::apps::{
    Auth, FILES, NOTES, TestResult, ok, op, post, refused, registered, root, workspace_schema,
};
use identity_contract::harness::Service;
use serde_json::json;

/// Run the scenario on `service`, as the administrator's session `admin`,
/// with `bea` the person granted once the draft is saved.
pub async fn answers_as_saved(service: &Service, admin: &str, bea: &str) -> TestResult {
    let (channel, workspace) = (format!("{NOTES}.channel"), format!("{NOTES}.workspace"));
    let open = json!({"app": NOTES, "schema": workspace_schema(NOTES)});
    let opened = ok(post(service, "/apps/bench", Auth::Cookie(admin), &open).await?)?;
    let bench = opened["bench"].as_str().ok_or("no bench")?.to_owned();
    let other = format!("{FILES}.doc");
    let ask_on = |action: &str, kind: &str| {
        json!({
            "holdings": [{"subject": "X", "relation": "member", "resource": {"kind": workspace, "id": "team"}}],
            "placements": [{"child": {"kind": channel, "id": "general"}, "parent": {"kind": workspace, "id": "team"}}],
            "question": {"subject": "X", "action": action, "resource": {"kind": kind, "id": "general"}},
        })
    };
    let ask = |action: &str| ask_on(action, &channel);
    let asked = format!("/apps/bench/{bench}/ask");
    let may_read = ok(post(service, &asked, Auth::Cookie(admin), &ask("read")).await?)?;
    assert_eq!(may_read["allowed"], true, "{may_read}");
    assert_eq!(
        may_read["path"],
        json!([
            format!("{channel}:general is placed in {workspace}:team"),
            format!("X holds member on {workspace}:team"),
            "member carries read",
        ])
    );
    let may_write = ok(post(service, &asked, Auth::Cookie(admin), &ask("write")).await?)?;
    assert_eq!(may_write["allowed"], false);
    assert_eq!(may_write["refusal"], "NotHeld");
    let undeclared = ok(post(service, &asked, Auth::Cookie(admin), &ask("delete")).await?)?;
    assert_eq!(undeclared["refusal"], "action_not_declared", "{undeclared}");
    let elsewhere = ok(post(
        service,
        &asked,
        Auth::Cookie(admin),
        &ask_on("read", &other),
    )
    .await?)?;
    assert_eq!(elsewhere["refusal"], "kind_not_registered", "{elsewhere}");

    let unseen =
        json!({"checks": [identity_contract::apps::check(bea, &channel, "general", "read")]});
    let before = ok(post(service, "/grants/check/batch", Auth::Cookie(admin), &unseen).await?)?;
    assert_eq!(
        before["results"][0]["refusal"], "kind_not_registered",
        "no standing check sees a draft"
    );

    let credential = registered(service, admin, NOTES).await?;
    let place = json!({
        "operation": op()?,
        "child": {"kind": channel, "id": "general"},
        "parent": {"kind": workspace, "id": "team"},
    });
    ok(post(
        service,
        &format!("/apps/{NOTES}/placements"),
        Auth::Bearer(&credential),
        &place,
    )
    .await?)?;
    ok(root(service, admin, bea, (&workspace, "team"), "member").await?)?;
    let both = json!({"checks": [
        identity_contract::apps::check(bea, &channel, "general", "read"),
        identity_contract::apps::check(bea, &channel, "general", "write"),
        identity_contract::apps::check(bea, &channel, "general", "delete"),
        identity_contract::apps::check(bea, &other, "general", "read"),
    ]});
    let after = ok(post(service, "/grants/check/batch", Auth::Cookie(admin), &both).await?)?;
    assert_eq!(
        after["results"][0]["allowed"], may_read["allowed"],
        "{after}"
    );
    assert_eq!(
        after["results"][1]["allowed"], may_write["allowed"],
        "{after}"
    );
    assert_eq!(after["results"][1]["refusal"], may_write["refusal"]);
    assert_eq!(after["results"][2]["refusal"], undeclared["refusal"]);
    assert_eq!(after["results"][3]["refusal"], elsewhere["refusal"]);

    ok(post(
        service,
        &format!("/apps/bench/{bench}/close"),
        Auth::Cookie(admin),
        &json!({}),
    )
    .await?)?;
    assert!(
        !service.dir.path().join("benches").join(&bench).exists(),
        "closing the bench removes its namespace"
    );
    refused(
        &post(service, &asked, Auth::Cookie(admin), &ask("read")).await?,
        404,
        "bench_unknown",
    )?;
    Ok(())
}
