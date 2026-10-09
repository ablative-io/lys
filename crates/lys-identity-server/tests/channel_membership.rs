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
    for (channel, restricted) in [("ward-a", false), ("ward-b", false), ("ward-secret", true)] {
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
