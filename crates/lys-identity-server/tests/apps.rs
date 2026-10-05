#![cfg(test)]
//! An app is a record registered through the API and of no effect until the
//! administrator approves it: a pending app has no client and its kinds are
//! judged by nothing; approval creates its client, answers the secret once
//! and keeps it nowhere, and gives its kinds to the grants; a retired app's
//! credential and checks are refused while its grants stay readable. Every
//! refusal is asserted by name, and each test counts the refusals it met.

use std::error::Error;
use std::path::Path;

use identity_contract::apps::{
    Auth, BEA, FILES, NOTES, TestResult, approve, check, get, login, ok, op, post, refused,
    register, registered, registration, root, seeded, workspace_schema,
};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::{Value, json};

/// Whether any file under `dir` holds `needle`.
fn held_anywhere(dir: &Path, needle: &[u8]) -> Result<bool, Box<dyn Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let found = if path.is_dir() {
            held_anywhere(&path, needle)?
        } else {
            std::fs::read(&path)?
                .windows(needle.len())
                .any(|window| window == needle)
        };
        if found {
            return Ok(true);
        }
    }
    Ok(false)
}

fn action(kind: &str, id: &str, action: &str) -> Value {
    json!({"route": "api", "resource": {"kind": kind, "id": id}, "action": action})
}

#[tokio::test]
async fn a_pending_app_has_no_client_and_its_kinds_answer_app_not_approved() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let pending = register(&service, &admin, NOTES).await?;
    assert_eq!(pending["state"], "pending", "{pending}");
    assert_eq!(
        pending["client_id"],
        Value::Null,
        "registering creates no client"
    );
    assert_eq!(pending["version"], 0);

    let doc = format!("{NOTES}.doc");
    let mut refusals = 0;
    let checked = post(
        &service,
        "/grants/check",
        Auth::Cookie(&admin),
        &action(&doc, "1", "read"),
    )
    .await?;
    refused(&checked, 403, "app_not_approved")?;
    refusals += 1;
    refused(
        &root(&service, &admin, &bea, (&doc, "1"), "editor").await?,
        403,
        "app_not_approved",
    )?;
    refusals += 1;
    let guessed = format!("lys-app.{NOTES}.{}", "0".repeat(64));
    refused(
        &get(&service, "/apps/me", Auth::Bearer(&guessed)).await?,
        401,
        "credential_refused",
    )?;
    refusals += 1;
    assert_eq!(refusals, 3);
    Ok(())
}

#[tokio::test]
async fn approval_confirms_custody_without_returning_secrets_and_a_sign_in_and_check_pass()
-> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    register(&service, &admin, NOTES).await?;
    let operation = op()?;
    let body = json!({"operation": operation});
    let path = format!("/apps/{NOTES}/approve");
    let approval = ok(post(&service, &path, Auth::Cookie(&admin), &body).await?)?;
    assert_eq!(approval["app"]["state"], "approved", "{approval}");
    assert_eq!(approval["app"]["version"], 1);
    assert!(approval["client"].is_null());
    assert_eq!(approval["credentials"]["app"], NOTES);
    let secret = identity_contract::app_custody::secret();
    let credential = identity_contract::app_custody::credential(NOTES);
    assert!(!approval.to_string().contains(&secret));

    let again = ok(post(&service, &path, Auth::Cookie(&admin), &body).await?)?;
    assert_eq!(
        again["client"],
        Value::Null,
        "approval never returns the secret: {again}"
    );
    let other = json!({"operation": op()?});
    refused(
        &post(&service, &path, Auth::Cookie(&admin), &other).await?,
        409,
        "app_decided",
    )?;
    register(&service, &admin, FILES).await?;
    let reused = post(
        &service,
        &format!("/apps/{FILES}/approve"),
        Auth::Cookie(&admin),
        &body,
    )
    .await?;
    refused(&reused, 409, "app_operation_reused")?;
    assert!(
        !held_anywhere(service.dir.path(), secret.as_bytes())?,
        "no log, receipt or record holds the client secret"
    );

    let signed_in = ok(get(&service, "/apps/me", Auth::Bearer(&credential)).await?)?;
    assert_eq!(signed_in["id"], NOTES);
    let doc = format!("{NOTES}.doc");
    ok(root(&service, &admin, &bea, (&doc, "1"), "editor").await?)?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let permit = ok(post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &action(&doc, "1", "write"),
    )
    .await?)?;
    assert!(permit["grant"].is_string(), "{permit}");
    Ok(())
}

/// DIRECTORY-079 R2: an approved app's sign-in settings are set by the
/// administrator and nobody else; each address is judged as a registration's
/// is, a list with no address is refused in words that say the app could
/// sign nobody in, an app that is not approved is refused by its standing's
/// name, and an operation id reused answers the same and keeps nothing new.
#[tokio::test]
async fn sign_in_settings_are_the_administrators_judged_by_address_and_by_standing() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let path = format!("/apps/{NOTES}/sign_in");
    let settings = |redirects: Value, profile: bool| -> Result<Value, Box<dyn Error>> {
        Ok(json!({"operation": op()?, "redirects": redirects, "profile": profile}))
    };
    let listed = json!(["https://app.example.test/signed-in"]);
    register(&service, &admin, NOTES).await?;
    let mut refusals = 0;
    // Pending: refused by the standing's name.
    refused(
        &post(
            &service,
            &path,
            Auth::Cookie(&admin),
            &settings(listed.clone(), false)?,
        )
        .await?,
        403,
        "app_not_approved",
    )?;
    refusals += 1;
    // An approval with no return address is refused whole, and the app stays pending.
    let empty = json!({"operation": op()?, "redirects": [], "profile": false});
    let answer = post(
        &service,
        &format!("/apps/{NOTES}/approve"),
        Auth::Cookie(&admin),
        &empty,
    )
    .await?;
    refused(&answer, 400, "redirect_invalid")?;
    assert!(
        answer.1["reason"]
            .as_str()
            .is_some_and(|words| words.contains("can sign nobody in")),
        "{}",
        answer.1
    );
    assert_eq!(answer.1["fields"][0]["at"], "/redirects");
    refusals += 1;
    let still = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(still["state"], "pending");
    assert!(still["sign_in"].is_null());
    // Approved with its registration's addresses: the view carries them.
    approve(&service, &admin, NOTES).await?;
    let approved = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(approved["sign_in"]["redirects"], listed, "{approved}");
    assert_eq!(approved["sign_in"]["profile"], false);
    // Address rules: none, not absolute, a fragment, http away from this machine.
    let none = post(
        &service,
        &path,
        Auth::Cookie(&admin),
        &settings(json!([]), false)?,
    )
    .await?;
    refused(&none, 400, "redirect_invalid")?;
    assert!(
        none.1["reason"]
            .as_str()
            .is_some_and(|words| words.contains("can sign nobody in")),
        "{}",
        none.1
    );
    refusals += 1;
    for address in [
        "signed-in",
        "https://app.example.test/signed-in#fragment",
        "http://app.example.test/signed-in",
    ] {
        let answer = post(
            &service,
            &path,
            Auth::Cookie(&admin),
            &settings(json!([address]), false)?,
        )
        .await?;
        refused(&answer, 400, "redirect_invalid")?;
        assert!(
            answer.1["reason"]
                .as_str()
                .is_some_and(|words| words.contains(address)),
            "{}",
            answer.1
        );
        refusals += 1;
    }
    // A signed-in person who is not the administrator changes nothing.
    let bea = service.sign_in(login(BEA)).await?;
    let moved = json!(["https://app.example.test/moved"]);
    refused(
        &post(
            &service,
            &path,
            Auth::Cookie(&bea),
            &settings(moved.clone(), true)?,
        )
        .await?,
        403,
        "NotAdmitted",
    )?;
    refusals += 1;
    let unchanged = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(unchanged["sign_in"], approved["sign_in"]);
    // The administrator's change, and the same operation again: the same
    // answer, nothing new kept; the same operation for another app: refused.
    let change = settings(moved.clone(), true)?;
    let changed = ok(post(&service, &path, Auth::Cookie(&admin), &change).await?)?;
    assert_eq!(changed["sign_in"]["redirects"], moved);
    assert_eq!(changed["sign_in"]["profile"], true);
    let again = ok(post(&service, &path, Auth::Cookie(&admin), &change).await?)?;
    assert_eq!(again["sign_in"], changed["sign_in"]);
    register(&service, &admin, FILES).await?;
    let reused = post(
        &service,
        &format!("/apps/{FILES}/sign_in"),
        Auth::Cookie(&admin),
        &change,
    )
    .await?;
    refused(&reused, 409, "app_operation_reused")?;
    refusals += 1;
    // Declined and retired: refused by the standing's name.
    let decline = json!({"operation": op()?, "reason": "not this one"});
    ok(post(
        &service,
        &format!("/apps/{FILES}/decline"),
        Auth::Cookie(&admin),
        &decline,
    )
    .await?)?;
    refused(
        &post(
            &service,
            &format!("/apps/{FILES}/sign_in"),
            Auth::Cookie(&admin),
            &settings(listed.clone(), false)?,
        )
        .await?,
        403,
        "app_not_approved",
    )?;
    refusals += 1;
    let retire = json!({"operation": op()?, "reason": "done"});
    ok(post(
        &service,
        &format!("/apps/{NOTES}/retire"),
        Auth::Cookie(&admin),
        &retire,
    )
    .await?)?;
    refused(
        &post(
            &service,
            &path,
            Auth::Cookie(&admin),
            &settings(listed, false)?,
        )
        .await?,
        403,
        "app_retired",
    )?;
    refusals += 1;
    assert_eq!(refusals, 10);
    Ok(())
}

#[tokio::test]
async fn a_second_registration_of_an_id_and_of_lys_are_refused_app_exists() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    register(&service, &admin, NOTES).await?;
    let mut refusals = 0;
    for app in [NOTES, "lys"] {
        let body = registration(app, &workspace_schema(NOTES))?;
        refused(
            &post(&service, "/apps", Auth::Cookie(&admin), &body).await?,
            409,
            "app_exists",
        )?;
        refusals += 1;
    }
    assert_eq!(refusals, 2);
    Ok(())
}

#[tokio::test]
async fn ids_with_a_hyphen_a_dot_or_under_three_characters_are_refused_app_id_invalid() -> TestResult
{
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let mut refusals = 0;
    for id in ["fixture-notes", "fixture.notes", "fx"] {
        let body = registration(id, &workspace_schema(id))?;
        let answer = post(&service, "/apps", Auth::Cookie(&admin), &body).await?;
        refused(&answer, 400, "app_id_invalid")?;
        assert_eq!(answer.1["fields"][0]["at"], "/id", "{}", answer.1);
        refusals += 1;
    }
    assert_eq!(refusals, 3);
    Ok(())
}

#[tokio::test]
async fn a_retired_apps_checks_answer_app_retired_and_its_grants_stay_readable() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    let issued = ok(root(&service, &admin, &bea, (&doc, "1"), "editor").await?)?;
    let grant = issued["grant"].as_str().ok_or("no grant")?.to_owned();
    let retire = json!({"operation": op()?, "reason": "the fixture is done"});
    let retired = ok(post(
        &service,
        &format!("/apps/{NOTES}/retire"),
        Auth::Cookie(&admin),
        &retire,
    )
    .await?)?;
    assert_eq!(retired["state"], "retired");

    let bea_cookie = service.sign_in(login(BEA)).await?;
    let mut refusals = 0;
    let checked = post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &action(&doc, "1", "read"),
    )
    .await?;
    refused(&checked, 403, "app_retired")?;
    refusals += 1;
    refused(
        &get(&service, "/apps/me", Auth::Bearer(&credential)).await?,
        403,
        "app_retired",
    )?;
    refusals += 1;
    let lys = json!({"operation": op()?});
    refused(
        &post(&service, "/apps/lys/retire", Auth::Cookie(&admin), &lys).await?,
        403,
        "app_is_lys",
    )?;
    refusals += 1;
    assert_eq!(refusals, 3);
    let read = ok(get(&service, &format!("/grants/{grant}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(read["id"], grant.as_str(), "{read}");
    Ok(())
}

#[tokio::test]
async fn a_declined_app_never_takes_effect() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    register(&service, &admin, NOTES).await?;
    let decline = json!({"operation": op()?, "reason": "not this one"});
    let declined = ok(post(
        &service,
        &format!("/apps/{NOTES}/decline"),
        Auth::Cookie(&admin),
        &decline,
    )
    .await?)?;
    assert_eq!(declined["state"], "declined");
    let approve = json!({"operation": op()?});
    let answer = post(
        &service,
        &format!("/apps/{NOTES}/approve"),
        Auth::Cookie(&admin),
        &approve,
    )
    .await?;
    refused(&answer, 409, "app_decided")?;
    let checked = post(
        &service,
        "/grants/why",
        Auth::Cookie(&admin),
        &action(&format!("{NOTES}.doc"), "1", "read"),
    )
    .await?;
    refused(&checked, 403, "app_not_approved")?;
    Ok(())
}

#[tokio::test]
async fn a_kind_no_approved_app_declares_is_refused_kind_not_registered() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let mut refusals = 0;
    for kind in ["fixture_nobody.doc", "fixture_notes.nothing"] {
        refused(
            &root(&service, &admin, &bea, (kind, "1"), "editor").await?,
            403,
            "kind_not_registered",
        )?;
        refusals += 1;
    }
    let undeclared = action(&format!("{NOTES}.doc"), "1", "delete");
    refused(
        &post(&service, "/grants/check", Auth::Cookie(&admin), &undeclared).await?,
        400,
        "action_not_declared",
    )?;
    refusals += 1;
    assert_eq!(refusals, 3);
    Ok(())
}

#[tokio::test]
async fn an_apps_credential_naming_another_apps_kind_is_refused_not_your_app() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let notes = registered(&service, &admin, NOTES).await?;
    registered(&service, &admin, FILES).await?;
    let files_doc = format!("{FILES}.doc");
    let batch = json!({"checks": [check(&bea, &files_doc, "1", "read")]});
    let answer = ok(post(
        &service,
        "/grants/check/batch",
        Auth::Bearer(&notes),
        &batch,
    )
    .await?)?;
    assert_eq!(answer["results"][0]["refusal"], "not_your_app", "{answer}");
    assert!(
        answer["results"][0]["reason"]
            .as_str()
            .is_some_and(|words| words.contains("`fixture_files`"))
    );
    let which = json!({"subject": bea, "kind": files_doc, "action": "read", "page_size": 10});
    refused(
        &post(&service, "/grants/which", Auth::Bearer(&notes), &which).await?,
        403,
        "not_your_app",
    )?;
    let place = json!({
        "operation": op()?,
        "child": {"kind": format!("{FILES}.channel"), "id": "general"},
        "parent": {"kind": format!("{FILES}.workspace"), "id": "team"},
    });
    let placed = post(
        &service,
        &format!("/apps/{FILES}/placements"),
        Auth::Bearer(&notes),
        &place,
    )
    .await?;
    refused(&placed, 403, "NotAdmitted")?;
    Ok(())
}

#[tokio::test]
async fn a_workspace_member_may_read_a_channel_placed_in_it_and_no_more() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let (channel, workspace) = (format!("{NOTES}.channel"), format!("{NOTES}.workspace"));
    let place = json!({
        "operation": op()?,
        "child": {"kind": channel, "id": "general"},
        "parent": {"kind": workspace, "id": "team"},
    });
    ok(post(
        &service,
        &format!("/apps/{NOTES}/placements"),
        Auth::Bearer(&credential),
        &place,
    )
    .await?)?;
    let backwards = json!({
        "operation": op()?,
        "child": {"kind": workspace, "id": "team"},
        "parent": {"kind": channel, "id": "general"},
    });
    let refused_place = post(
        &service,
        &format!("/apps/{NOTES}/placements"),
        Auth::Bearer(&credential),
        &backwards,
    )
    .await?;
    refused(&refused_place, 400, "placement_invalid")?;
    ok(root(&service, &admin, &bea, (&workspace, "team"), "member").await?)?;

    let bea_cookie = service.sign_in(login(BEA)).await?;
    let read = ok(post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &action(&channel, "general", "read"),
    )
    .await?)?;
    assert!(read["grant"].is_string(), "{read}");
    let write = post(
        &service,
        "/grants/check",
        Auth::Cookie(&bea_cookie),
        &action(&channel, "general", "write"),
    )
    .await?;
    assert_eq!(
        write.0, 403,
        "a member carries read, not write: {}",
        write.1
    );
    let batch = json!({"checks": [check(&bea, &channel, "general", "read"), check(&bea, &channel, "elsewhere", "read")]});
    let answer = ok(post(
        &service,
        "/grants/check/batch",
        Auth::Bearer(&credential),
        &batch,
    )
    .await?)?;
    assert_eq!(answer["results"][0]["allowed"], true, "{answer}");
    assert_eq!(answer["results"][0]["via"], format!("{workspace}:team"));
    assert_eq!(
        answer["results"][1]["allowed"], false,
        "an unplaced channel reaches no workspace"
    );
    Ok(())
}

#[tokio::test]
async fn two_apps_with_the_same_kind_name_are_checked_each_under_its_own_prefix() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    registered(&service, &admin, FILES).await?;
    let (notes_doc, files_doc) = (format!("{NOTES}.doc"), format!("{FILES}.doc"));
    ok(root(&service, &admin, &bea, (&notes_doc, "1"), "editor").await?)?;
    let batch = json!({"checks": [
        check(&bea, &notes_doc, "1", "write"),
        check(&bea, &files_doc, "1", "write"),
    ]});
    let answer = ok(post(
        &service,
        "/grants/check/batch",
        Auth::Cookie(&admin),
        &batch,
    )
    .await?)?;
    assert_eq!(answer["results"][0]["allowed"], true, "{answer}");
    assert_eq!(answer["results"][1]["allowed"], false, "{answer}");
    assert_eq!(answer["results"][1]["refusal"], "NotHeld", "{answer}");
    Ok(())
}

#[tokio::test]
async fn a_schema_fault_is_refused_at_its_pointer_and_a_bad_redirect_by_name() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let mut schema = workspace_schema(NOTES);
    schema["kinds"]["fixture_files.doc"] =
        json!({"actions": ["read"], "relations": {"reader": ["read"]}});
    let answer = post(
        &service,
        "/apps",
        Auth::Cookie(&admin),
        &registration(NOTES, &schema)?,
    )
    .await?;
    refused(&answer, 400, "schema_invalid")?;
    assert_eq!(
        answer.1["fields"][0]["at"], "/schema/kinds/fixture_files.doc",
        "{}",
        answer.1
    );
    assert!(
        answer.1["reason"]
            .as_str()
            .is_some_and(|words| words.contains("`fixture_files`"))
    );
    let mut body = registration(NOTES, &workspace_schema(NOTES))?;
    body["redirects"] = json!(["http://app.example.test/signed-in"]);
    refused(
        &post(&service, "/apps", Auth::Cookie(&admin), &body).await?,
        400,
        "redirect_invalid",
    )?;
    Ok(())
}

#[path = "shared/app_registration.rs"]
mod app_registration;
