#![cfg(test)]
//! Roles and restricted placements over HTTP (ACCESS-004).
//!
//! R1: a grant naming a role is judged as the role's actions under the
//! version current when it is judged; a widening an app makes waits for the
//! administrator, its diff naming the widening in words, and gives nothing
//! until approved; a narrowing is refused while a standing grant names the
//! role, and taken when none does.
//!
//! R2: a channel placed restricted in a workspace is reached by no member of
//! the workspace, only by a grant on the channel itself, and the reach view
//! names it restricted, apart from an unrestricted one.

use identity_contract::apps::{
    Auth, BEA, NOTES, TestResult, login, ok, op, post, put, refused, registered, root, seeded,
    workspace_schema,
};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::{Value, json};

/// The workspace schema with an `administrator` role carrying `actions` and
/// an `observer` role carrying read.
fn with_roles(actions: &[&str]) -> Value {
    let mut schema = workspace_schema(NOTES);
    schema["kinds"][format!("{NOTES}.workspace")]["roles"] =
        json!({"administrator": actions, "observer": ["read"]});
    schema
}

fn action(kind: &str, id: &str, action: &str) -> Value {
    json!({"route": "api", "resource": {"kind": kind, "id": id}, "action": action})
}

#[tokio::test]
async fn a_grant_naming_a_role_is_judged_as_the_role_and_an_apps_widening_waits() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let path = format!("/apps/{NOTES}/schema");
    let workspace = format!("{NOTES}.workspace");

    let roles = json!({"operation": op()?, "replaces": 1, "schema": with_roles(&["read"])});
    let applied = ok(put(&service, &path, Auth::Cookie(&admin), &roles).await?)?;
    assert_eq!(applied["applied"], true, "{applied}");
    assert_eq!(
        applied["diff"]["roles_added"],
        json!([
            {"kind": workspace, "name": "administrator"},
            {"kind": workspace, "name": "observer"},
        ])
    );
    let issued = ok(root(
        &service,
        &admin,
        &bea,
        (&workspace, "ward"),
        "administrator",
    )
    .await?)?;
    assert!(issued["grant"].is_string(), "{issued}");

    let bea_cookie = service.sign_in(login(BEA)).await?;
    let check = |verb: &'static str| action(&workspace, "ward", verb);
    ok(post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &check("read"),
    )
    .await?)?;
    let write = post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &check("write"),
    )
    .await?;
    assert_eq!(
        write.0, 403,
        "administrator carries read alone: {}",
        write.1
    );

    let wider =
        json!({"operation": op()?, "replaces": 2, "schema": with_roles(&["read", "write"])});
    let proposed = ok(put(&service, &path, Auth::Bearer(&credential), &wider).await?)?;
    assert_eq!(
        proposed["applied"], false,
        "an app's widening waits: {proposed}"
    );
    assert_eq!(
        proposed["diff"]["roles_changed"][0]["words"],
        json!(["adds write to administrator"])
    );
    let waiting = post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &check("write"),
    )
    .await?;
    assert_eq!(
        waiting.0, 403,
        "a widening waiting for the administrator gives nothing: {}",
        waiting.1
    );

    let approve = json!({"operation": op()?});
    let approved = ok(post(
        &service,
        &format!("{path}/approve"),
        Auth::Cookie(&admin),
        &approve,
    )
    .await?)?;
    assert_eq!(approved["app"]["version"], 3, "{approved}");
    let now = ok(post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &check("write"),
    )
    .await?)?;
    assert!(
        now["grant"].is_string(),
        "the grant is judged as the role at version 3: {now}"
    );
    Ok(())
}

#[tokio::test]
async fn a_narrowing_of_a_role_a_grant_names_is_refused_and_one_no_grant_names_is_taken()
-> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let path = format!("/apps/{NOTES}/schema");
    let workspace = format!("{NOTES}.workspace");
    let roles =
        json!({"operation": op()?, "replaces": 1, "schema": with_roles(&["read", "write"])});
    ok(put(&service, &path, Auth::Cookie(&admin), &roles).await?)?;
    ok(root(
        &service,
        &admin,
        &bea,
        (&workspace, "ward"),
        "administrator",
    )
    .await?)?;

    let narrower = json!({"operation": op()?, "replaces": 2, "schema": with_roles(&["read"])});
    let answer = put(&service, &path, Auth::Cookie(&admin), &narrower).await?;
    refused(&answer, 409, "schema_change_strands_grants")?;
    let dry = json!({"replaces": 2, "schema": with_roles(&["read"])});
    let checked = ok(post(
        &service,
        &format!("{path}/check"),
        Auth::Cookie(&admin),
        &dry,
    )
    .await?)?;
    assert_eq!(
        checked["stranded"],
        json!([{"kind": workspace, "relation": "administrator", "count": 1}]),
        "{checked}"
    );

    let mut observer_gone = with_roles(&["read", "write"]);
    observer_gone["kinds"][&workspace]["roles"] = json!({"administrator": ["read", "write"]});
    let taken = json!({"operation": op()?, "replaces": 2, "schema": observer_gone});
    let changed = ok(put(&service, &path, Auth::Cookie(&admin), &taken).await?)?;
    assert_eq!(
        changed["applied"], true,
        "removing a role no grant names strands nothing: {changed}"
    );
    Ok(())
}

#[tokio::test]
async fn a_restricted_place_is_reached_only_by_a_grant_on_it_and_shown_apart() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let (channel, workspace) = (format!("{NOTES}.channel"), format!("{NOTES}.workspace"));
    let placements = format!("/apps/{NOTES}/placements");
    for (id, restricted) in [("general", false), ("ward_private", true)] {
        let place = json!({
            "operation": op()?,
            "child": {"kind": channel, "id": id},
            "parent": {"kind": workspace, "id": "team"},
            "restricted": restricted,
        });
        let placed = ok(post(&service, &placements, Auth::Bearer(&credential), &place).await?)?;
        assert_eq!(placed["placed"]["restricted"], restricted, "{placed}");
    }
    ok(root(&service, &admin, &bea, (&workspace, "team"), "member").await?)?;

    let bea_cookie = service.sign_in(login(BEA)).await?;
    let read = |id: &str| action(&channel, id, "read");
    ok(post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &read("general"),
    )
    .await?)?;
    let private = post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &read("ward_private"),
    )
    .await?;
    assert_eq!(
        private.0, 403,
        "a member of the workspace does not reach a restricted place: {}",
        private.1
    );

    ok(root(&service, &admin, &bea, (&channel, "ward_private"), "poster").await?)?;
    let granted = ok(post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &read("ward_private"),
    )
    .await?)?;
    assert!(
        granted["grant"].is_string(),
        "a grant on the place reaches it: {granted}"
    );

    let reach = json!({"route": "api", "resources": [
        {"kind": channel, "id": "general", "actions": ["read"]},
        {"kind": channel, "id": "ward_private", "actions": ["read"]},
    ]});
    let answer = ok(post(&service, "/grants/reach", Auth::Cookie(&admin), &reach).await?)?;
    assert_eq!(answer["resources"][0]["restricted"], false, "{answer}");
    assert_eq!(answer["resources"][1]["restricted"], true, "{answer}");
    Ok(())
}
