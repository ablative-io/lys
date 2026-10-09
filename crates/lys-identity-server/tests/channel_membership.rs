#![cfg(test)]
//! The served channel membership decision (ACCESS-006 R1): one route that
//! judges the shared lys-pass request through the grants' own decision at
//! one named revision, echoing the request it answers, and refuses by name a
//! foreign log, an unknown contract, a forged kind or workspace, a guessed
//! id and a revision ahead of the grants, without looking anything up.

use identity_contract::apps::{
    Auth, TestResult, approve, login, ok, op, post, refused, registration, root, seeded,
};
use identity_contract::harness::{ADMINISTRATOR, Service};
use serde_json::{Value, json};

const ROOMS: &str = "fixture_rooms";

/// A workspace whose members read its channels, and channels whose read and
/// post are held by separate relations.
fn rooms_schema() -> Value {
    json!({"kinds": {
        format!("{ROOMS}.workspace"): {
            "actions": ["read", "post"],
            "relations": {"member": ["read"]},
            "parents": []
        },
        format!("{ROOMS}.channel"): {
            "actions": ["read", "post"],
            "relations": {"reader": ["read"], "poster": ["post"]},
            "parents": [format!("{ROOMS}.workspace")]
        }
    }})
}

struct Rooms {
    service: Service,
    admin: String,
    credential: String,
    bea: String,
    ada: String,
    scribe: String,
    log: Value,
}

fn kind(name: &str) -> String {
    format!("{ROOMS}.{name}")
}

async fn place(rooms: &Rooms, channel: &str, workspace: &str, restricted: bool) -> TestResult {
    let body = json!({
        "operation": op()?,
        "child": {"kind": kind("channel"), "id": channel},
        "parent": {"kind": kind("workspace"), "id": workspace},
        "restricted": restricted,
    });
    ok(post(
        &rooms.service,
        &format!("/apps/{ROOMS}/placements"),
        Auth::Bearer(&rooms.credential),
        &body,
    )
    .await?)?;
    Ok(())
}

fn ask(rooms: &Rooms, subject: (&str, &str), channel: &str, action: &str) -> Value {
    json!({
        "contract": 1,
        "log": rooms.log,
        "workspace": {"kind": kind("workspace"), "id": "ward"},
        "subject": {"id": subject.0, "kind": subject.1},
        "resource": {"kind": kind("channel"), "id": channel},
        "action": action,
        "at_least": 0,
    })
}

async fn decide(rooms: &Rooms, request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let answer = ok(post(
        &rooms.service,
        "/grants/membership",
        Auth::Bearer(&rooms.credential),
        request,
    )
    .await?)?;
    assert_eq!(&answer["request"], request, "the answer echoes its request");
    assert_eq!(answer["contract"], 1);
    Ok(answer)
}

/// The rooms app, a ward of three channels (one restricted) and a channel
/// of another workspace, with the served grant log read from the route.
async fn rooms() -> Result<Rooms, Box<dyn std::error::Error>> {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let body = registration(ROOMS, &rooms_schema())?;
    ok(post(&service, "/apps", Auth::Cookie(&admin), &body).await?)?;
    approve(&service, &admin, ROOMS).await?;
    let credential = identity_contract::app_custody::credential(ROOMS);
    let mut rooms = Rooms {
        service,
        admin,
        credential,
        bea: seeded.people[1].id.to_string(),
        ada: seeded.people[0].id.to_string(),
        scribe: seeded.people[0].agents[0].id.to_string(),
        log: json!({"identity": "not-this-log", "epoch": 0}),
    };
    let probe = ask(&rooms, (rooms.bea.as_str(), "person"), "ward-a", "read");
    let refusal = decide(&rooms, &probe).await?;
    assert_eq!(refusal["verdict"]["refusal"], "membership_log_mismatch");
    assert!(refusal["revision"].is_null(), "{refusal}");
    rooms.log = refusal["log"].clone();
    for (channel, restricted) in [
        ("ward-a", false),
        ("ward-b", false),
        ("ward-c", false),
        ("ward-secret", true),
    ] {
        place(&rooms, channel, "ward", restricted).await?;
    }
    place(&rooms, "elsewhere-a", "elsewhere", false).await?;
    Ok(rooms)
}

async fn grant(
    rooms: &Rooms,
    holder: &str,
    resource: (&str, &str),
    relation: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let kind = kind(resource.0);
    ok(root(
        &rooms.service,
        &rooms.admin,
        holder,
        (&kind, resource.1),
        relation,
    )
    .await?)
}

#[tokio::test]
async fn read_and_post_are_separate_and_stay_in_their_channel() -> TestResult {
    let rooms = rooms().await?;
    let reader = grant(&rooms, &rooms.bea, ("channel", "ward-a"), "reader").await?;
    let bea = (rooms.bea.as_str(), "person");

    let read = decide(&rooms, &ask(&rooms, bea, "ward-a", "read")).await?;
    assert_eq!(read["verdict"]["outcome"], "allowed", "{read}");
    assert_eq!(read["verdict"]["grant"], reader["grant"], "{read}");
    assert_eq!(
        read["verdict"]["scope"],
        json!({"kind": kind("channel"), "id": "ward-a"})
    );
    assert!(
        read["revision"]
            .as_u64()
            .is_some_and(|revision| revision >= 1),
        "{read}"
    );

    for (channel, action) in [("ward-a", "post"), ("ward-b", "read"), ("ward-b", "post")] {
        let refused = decide(&rooms, &ask(&rooms, bea, channel, action)).await?;
        assert_eq!(
            refused["verdict"]["outcome"], "refused",
            "{channel} {action}"
        );
        assert_eq!(refused["verdict"]["refusal"], "NotHeld", "{refused}");
        assert!(
            refused["revision"].is_u64(),
            "decided at a revision: {refused}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn a_workspace_member_reaches_its_channels_but_not_a_restricted_one() -> TestResult {
    let rooms = rooms().await?;
    let member = grant(&rooms, &rooms.bea, ("workspace", "ward"), "member").await?;
    let bea = (rooms.bea.as_str(), "person");

    let reached = decide(&rooms, &ask(&rooms, bea, "ward-b", "read")).await?;
    assert_eq!(reached["verdict"]["outcome"], "allowed", "{reached}");
    assert_eq!(reached["verdict"]["grant"], member["grant"]);
    assert_eq!(
        reached["verdict"]["scope"],
        json!({"kind": kind("workspace"), "id": "ward"})
    );
    let post = decide(&rooms, &ask(&rooms, bea, "ward-b", "post")).await?;
    assert_eq!(post["verdict"]["refusal"], "NotHeld", "{post}");
    let restricted = decide(&rooms, &ask(&rooms, bea, "ward-secret", "read")).await?;
    assert_eq!(restricted["verdict"]["refusal"], "NotHeld", "{restricted}");
    Ok(())
}

#[tokio::test]
async fn an_agent_is_judged_as_itself_never_as_its_responsible_person() -> TestResult {
    let rooms = rooms().await?;
    grant(&rooms, &rooms.ada, ("channel", "ward-a"), "reader").await?;
    let scribe = (rooms.scribe.as_str(), "agent");
    let inherited = decide(&rooms, &ask(&rooms, scribe, "ward-a", "read")).await?;
    assert_eq!(inherited["verdict"]["refusal"], "NotHeld", "{inherited}");
    let as_person = decide(
        &rooms,
        &ask(&rooms, (rooms.scribe.as_str(), "person"), "ward-a", "read"),
    )
    .await?;
    assert_eq!(
        as_person["verdict"]["refusal"], "membership_subject_kind_mismatch",
        "{as_person}"
    );
    assert!(as_person["revision"].is_null());
    Ok(())
}

#[tokio::test]
async fn a_revoked_grant_is_refused_at_the_revision_that_revoked_it() -> TestResult {
    let rooms = rooms().await?;
    let reader = grant(&rooms, &rooms.bea, ("channel", "ward-a"), "reader").await?;
    let grant_id = reader["grant"]
        .as_str()
        .ok_or("the issue names its grant")?;
    let revoke = json!({"operation": op()?, "route": "api", "reason": "left the ward"});
    let revoked = ok(post(
        &rooms.service,
        &format!("/grants/{grant_id}/revoke"),
        Auth::Cookie(&rooms.admin),
        &revoke,
    )
    .await?)?;
    let fence = revoked["receipt"]["revision"]
        .as_u64()
        .ok_or("the revoke names its revision")?;
    let mut request = ask(&rooms, (rooms.bea.as_str(), "person"), "ward-a", "read");
    request["at_least"] = json!(fence);
    let after = decide(&rooms, &request).await?;
    assert_eq!(after["verdict"]["outcome"], "refused", "{after}");
    assert!(
        after["revision"]
            .as_u64()
            .is_some_and(|revision| revision >= fence),
        "{after}"
    );
    Ok(())
}

#[tokio::test]
async fn bindings_are_refused_by_name_before_any_lookup() -> TestResult {
    let rooms = rooms().await?;
    grant(&rooms, &rooms.bea, ("channel", "ward-a"), "reader").await?;
    let bea = (rooms.bea.as_str(), "person");

    let mut future = ask(&rooms, bea, "ward-a", "read");
    future["contract"] = json!(2);
    let mut reset = ask(&rooms, bea, "ward-a", "read");
    reset["log"]["epoch"] = json!(rooms.log["epoch"].as_u64().unwrap_or(0) + 1);
    let mut forged_workspace = ask(&rooms, bea, "ward-a", "read");
    forged_workspace["workspace"]["id"] = json!("elsewhere");
    let mut ahead = ask(&rooms, bea, "ward-a", "read");
    ahead["at_least"] = json!(u64::MAX);
    let mut empty_action = ask(&rooms, bea, "ward-a", "read");
    empty_action["action"] = json!("");
    for (request, name) in [
        (future, "membership_contract_unsupported"),
        (reset, "membership_log_mismatch"),
        (forged_workspace, "membership_outside_workspace"),
        (
            ask(&rooms, bea, "ward-z", "read"),
            "membership_outside_workspace",
        ),
        (
            ask(&rooms, bea, "elsewhere-a", "read"),
            "membership_outside_workspace",
        ),
        (ahead, "StaleDecision"),
        (empty_action, "membership_request_malformed"),
    ] {
        let answer = decide(&rooms, &request).await?;
        assert_eq!(answer["verdict"]["refusal"], name, "{answer}");
        assert!(
            answer["revision"].is_null(),
            "refused before a decision: {answer}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn a_claimed_label_or_follow_is_no_member_of_the_request() -> TestResult {
    let rooms = rooms().await?;
    for extra in ["label", "follow", "tag", "cursor"] {
        let mut request = ask(&rooms, (rooms.bea.as_str(), "person"), "ward-a", "read");
        request[extra] = json!(true);
        refused(
            &post(
                &rooms.service,
                "/grants/membership",
                Auth::Bearer(&rooms.credential),
                &request,
            )
            .await?,
            400,
            "RequestMalformed",
        )?;
    }
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_and_the_owning_app_may_ask() -> TestResult {
    let rooms = rooms().await?;
    let bea_cookie = rooms
        .service
        .sign_in(login(identity_contract::apps::BEA))
        .await?;
    let request = ask(&rooms, (rooms.bea.as_str(), "person"), "ward-a", "read");
    refused(
        &post(
            &rooms.service,
            "/grants/membership",
            Auth::Cookie(&bea_cookie),
            &request,
        )
        .await?,
        403,
        "NotAdmitted",
    )?;
    let admin = ok(post(
        &rooms.service,
        "/grants/membership",
        Auth::Cookie(&rooms.admin),
        &request,
    )
    .await?)?;
    assert_eq!(admin["request"], request);
    Ok(())
}

/// ACCESS-006 R3: a page of the resources one subject may act on.
fn resources_page(rooms: &Rooms, subject: (&str, &str), rows: u32, after: Value) -> Value {
    json!({
        "contract": 1,
        "log": rooms.log,
        "workspace": {"kind": kind("workspace"), "id": "ward"},
        "subject": {"id": subject.0, "kind": subject.1},
        "kind": kind("channel"),
        "action": "read",
        "at_least": 0,
        "bounds": {"rows": rows, "bytes": 65536},
        "after": after,
    })
}

/// ACCESS-006 R3: a page of the subjects who may act on one channel.
fn recipients_page(rooms: &Rooms, channel: &str, rows: u32, after: Value) -> Value {
    json!({
        "contract": 1,
        "log": rooms.log,
        "workspace": {"kind": kind("workspace"), "id": "ward"},
        "resource": {"kind": kind("channel"), "id": channel},
        "action": "read",
        "at_least": 0,
        "bounds": {"rows": rows, "bytes": 65536},
        "after": after,
    })
}

async fn page(
    rooms: &Rooms,
    path: &str,
    request: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let answer = ok(post(
        &rooms.service,
        &format!("/grants/membership/{path}"),
        Auth::Bearer(&rooms.credential),
        request,
    )
    .await?)?;
    assert_eq!(answer["contract"], 1);
    assert_eq!(answer["log"], rooms.log);
    Ok(answer)
}

fn ids(page: &Value, field: &str) -> Vec<String> {
    page["outcome"]["rows"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row[field]["id"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn resources_page_through_the_ward_and_skip_the_restricted_child() -> TestResult {
    let rooms = rooms().await?;
    grant(&rooms, &rooms.bea, ("channel", "ward-a"), "reader").await?;
    grant(&rooms, &rooms.bea, ("workspace", "ward"), "member").await?;
    let bea = (rooms.bea.as_str(), "person");

    let first = page(
        &rooms,
        "resources",
        &resources_page(&rooms, bea, 2, Value::Null),
    )
    .await?;
    assert_eq!(first["outcome"]["outcome"], "page", "{first}");
    assert_eq!(ids(&first, "resource"), ["ward-a", "ward-b"], "{first}");
    assert_eq!(first["outcome"]["returned"], 2);
    assert_eq!(first["outcome"]["complete"], false);
    assert!(first["revision"].is_u64());
    let next = first["outcome"]["next"].clone();
    assert!(next.is_string(), "{first}");

    let second = page(&rooms, "resources", &resources_page(&rooms, bea, 2, next)).await?;
    assert_eq!(ids(&second, "resource"), ["ward-c"], "{second}");
    assert_eq!(
        second["outcome"]["skipped"], 1,
        "ward-secret is decided and left out"
    );
    assert_eq!(second["outcome"]["complete"], true);
    assert!(second["outcome"]["next"].is_null());
    Ok(())
}

#[tokio::test]
async fn a_revocation_between_pages_is_seen_by_the_next_page() -> TestResult {
    let rooms = rooms().await?;
    let member = grant(&rooms, &rooms.bea, ("workspace", "ward"), "member").await?;
    let bea = (rooms.bea.as_str(), "person");
    let first = page(
        &rooms,
        "resources",
        &resources_page(&rooms, bea, 1, Value::Null),
    )
    .await?;
    assert_eq!(ids(&first, "resource"), ["ward-a"]);
    let grant_id = member["grant"]
        .as_str()
        .ok_or("the issue names its grant")?;
    let revoke = json!({"operation": op()?, "route": "api", "reason": "left the ward"});
    let revoked = ok(post(
        &rooms.service,
        &format!("/grants/{grant_id}/revoke"),
        Auth::Cookie(&rooms.admin),
        &revoke,
    )
    .await?)?;
    let mut request = resources_page(&rooms, bea, 1, first["outcome"]["next"].clone());
    request["at_least"] = revoked["receipt"]["revision"].clone();
    let after = page(&rooms, "resources", &request).await?;
    assert!(ids(&after, "resource").is_empty(), "{after}");
    assert_eq!(after["outcome"]["returned"], 0);
    assert_eq!(after["outcome"]["complete"], true);
    Ok(())
}

#[tokio::test]
async fn a_cursor_is_bound_to_its_question() -> TestResult {
    let rooms = rooms().await?;
    grant(&rooms, &rooms.bea, ("workspace", "ward"), "member").await?;
    let bea = (rooms.bea.as_str(), "person");
    let first = page(
        &rooms,
        "resources",
        &resources_page(&rooms, bea, 1, Value::Null),
    )
    .await?;
    let next = first["outcome"]["next"].clone();
    let ada = (rooms.ada.as_str(), "person");
    for (request, name) in [
        (
            resources_page(&rooms, ada, 1, next.clone()),
            "membership_cursor_foreign",
        ),
        (
            resources_page(&rooms, bea, 1, json!("not-a-cursor")),
            "membership_cursor_malformed",
        ),
        (
            resources_page(&rooms, bea, 0, Value::Null),
            "membership_page_bound_invalid",
        ),
    ] {
        let refused = page(&rooms, "resources", &request).await?;
        assert_eq!(refused["outcome"]["refusal"], name, "{refused}");
        assert!(refused["outcome"].get("rows").is_none());
    }
    let substituted = page(
        &rooms,
        "recipients",
        &recipients_page(&rooms, "ward-a", 1, next),
    )
    .await?;
    assert_eq!(
        substituted["outcome"]["refusal"], "membership_cursor_foreign",
        "{substituted}"
    );
    let mut tiny = resources_page(&rooms, bea, 1, Value::Null);
    tiny["bounds"]["bytes"] = json!(1);
    let over = page(&rooms, "resources", &tiny).await?;
    assert_eq!(
        over["outcome"]["refusal"], "membership_page_row_over_budget",
        "{over}"
    );
    Ok(())
}

#[tokio::test]
async fn recipients_are_each_listed_once_whatever_their_chains() -> TestResult {
    let rooms = rooms().await?;
    grant(&rooms, &rooms.bea, ("channel", "ward-a"), "reader").await?;
    grant(&rooms, &rooms.bea, ("workspace", "ward"), "member").await?;
    grant(&rooms, &rooms.ada, ("workspace", "ward"), "member").await?;
    grant(&rooms, &rooms.ada, ("channel", "elsewhere-a"), "reader").await?;
    let mut expected = vec![rooms.ada.clone(), rooms.bea.clone()];
    expected.sort();

    let mut listed = Vec::new();
    let mut after = Value::Null;
    let mut pages = 0;
    loop {
        let answer = page(
            &rooms,
            "recipients",
            &recipients_page(&rooms, "ward-a", 1, after),
        )
        .await?;
        pages += 1;
        listed.extend(ids(&answer, "subject"));
        if answer["outcome"]["complete"] == true {
            break;
        }
        after = answer["outcome"]["next"].clone();
    }
    assert_eq!(listed, expected, "one row per recipient, in a stable order");
    assert_eq!(pages, 2);
    let restricted = page(
        &rooms,
        "recipients",
        &recipients_page(&rooms, "ward-secret", 5, Value::Null),
    )
    .await?;
    assert!(ids(&restricted, "subject").is_empty(), "{restricted}");
    let outside = page(
        &rooms,
        "recipients",
        &recipients_page(&rooms, "elsewhere-a", 5, Value::Null),
    )
    .await?;
    assert_eq!(
        outside["outcome"]["refusal"], "membership_outside_workspace",
        "{outside}"
    );
    Ok(())
}

/// ACCESS-006 R4: whether a subject holds a current grant within the ward.
fn admission(rooms: &Rooms, subject: (&str, &str)) -> Value {
    json!({
        "contract": 1,
        "log": rooms.log,
        "workspace": {"kind": kind("workspace"), "id": "ward"},
        "subject": {"id": subject.0, "kind": subject.1},
        "at_least": 0,
    })
}

async fn admit(rooms: &Rooms, request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let answer = ok(post(
        &rooms.service,
        "/grants/membership/admission",
        Auth::Bearer(&rooms.credential),
        request,
    )
    .await?)?;
    assert_eq!(&answer["request"], request, "the answer echoes its request");
    Ok(answer)
}

#[tokio::test]
async fn one_channel_grant_admits_a_guest_and_nothing_wider() -> TestResult {
    let rooms = rooms().await?;
    let reader = grant(&rooms, &rooms.bea, ("channel", "ward-a"), "reader").await?;
    let bea = (rooms.bea.as_str(), "person");
    let admitted = admit(&rooms, &admission(&rooms, bea)).await?;
    assert_eq!(admitted["verdict"]["outcome"], "allowed", "{admitted}");
    assert_eq!(admitted["verdict"]["grant"], reader["grant"]);
    assert_eq!(
        admitted["verdict"]["scope"],
        json!({"kind": kind("channel"), "id": "ward-a"})
    );
    for (channel, action) in [("ward-b", "read"), ("ward-a", "post")] {
        let refused = decide(&rooms, &ask(&rooms, bea, channel, action)).await?;
        assert_eq!(refused["verdict"]["refusal"], "NotHeld", "{refused}");
    }
    let roster = page(
        &rooms,
        "resources",
        &resources_page(&rooms, bea, 10, Value::Null),
    )
    .await?;
    assert_eq!(
        ids(&roster, "resource"),
        ["ward-a"],
        "admission widened nothing"
    );
    Ok(())
}

#[tokio::test]
async fn no_current_grant_refuses_and_the_last_revocation_closes_the_door() -> TestResult {
    let rooms = rooms().await?;
    let ada = (rooms.ada.as_str(), "person");
    let none = admit(&rooms, &admission(&rooms, ada)).await?;
    assert_eq!(
        none["verdict"]["refusal"], "membership_no_current_grant",
        "{none}"
    );
    assert!(none["revision"].is_u64(), "decided at a revision: {none}");
    grant(&rooms, &rooms.ada, ("channel", "elsewhere-a"), "reader").await?;
    let elsewhere = admit(&rooms, &admission(&rooms, ada)).await?;
    assert_eq!(
        elsewhere["verdict"]["refusal"], "membership_no_current_grant",
        "a grant in another workspace admits nothing here: {elsewhere}"
    );

    let only = grant(&rooms, &rooms.ada, ("channel", "ward-b"), "reader").await?;
    let admitted = admit(&rooms, &admission(&rooms, ada)).await?;
    assert_eq!(admitted["verdict"]["outcome"], "allowed", "{admitted}");
    let grant_id = only["grant"].as_str().ok_or("the issue names its grant")?;
    let revoke = json!({"operation": op()?, "route": "api", "reason": "visit over"});
    let revoked = ok(post(
        &rooms.service,
        &format!("/grants/{grant_id}/revoke"),
        Auth::Cookie(&rooms.admin),
        &revoke,
    )
    .await?)?;
    let mut request = admission(&rooms, ada);
    request["at_least"] = revoked["receipt"]["revision"].clone();
    let closed = admit(&rooms, &request).await?;
    assert_eq!(
        closed["verdict"]["refusal"], "membership_no_current_grant",
        "{closed}"
    );
    Ok(())
}

#[tokio::test]
async fn an_agent_is_admitted_only_by_its_own_grants() -> TestResult {
    let rooms = rooms().await?;
    grant(&rooms, &rooms.ada, ("workspace", "ward"), "member").await?;
    let scribe = (rooms.scribe.as_str(), "agent");
    let refused = admit(&rooms, &admission(&rooms, scribe)).await?;
    assert_eq!(
        refused["verdict"]["refusal"], "membership_no_current_grant",
        "{refused}"
    );
    let forged = admit(
        &rooms,
        &admission(&rooms, (rooms.scribe.as_str(), "person")),
    )
    .await?;
    assert_eq!(
        forged["verdict"]["refusal"], "membership_subject_kind_mismatch",
        "{forged}"
    );
    Ok(())
}
