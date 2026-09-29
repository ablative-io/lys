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
    Auth, BEA, FILES, NOTES, TestResult, check, get, login, ok, op, post, refused, register,
    registered, registration, root, seeded, workspace_schema,
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
async fn approval_creates_the_client_shows_its_secret_once_and_a_sign_in_and_check_pass()
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
    let secret = approval["client"]["client_secret"]
        .as_str()
        .ok_or("no secret")?
        .to_owned();
    let credential = approval["client"]["credential"]
        .as_str()
        .ok_or("no credential")?
        .to_owned();
    assert_eq!(secret.len(), 64);

    let again = ok(post(&service, &path, Auth::Cookie(&admin), &body).await?)?;
    assert_eq!(
        again["client"],
        Value::Null,
        "the secret is shown once: {again}"
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

#[tokio::test]
async fn a_registrar_registers_apps_and_a_person_or_a_wrong_credential_does_not() -> TestResult {
    let (service, seeded) = identity_contract::harness::Service::start_judging(
        r#"{"version":1,"relations":{"editor":["view","edit"],"viewer":["view"]}}"#,
        None,
        |config| {
            Ok(lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, BEA],
            )?)
        },
    )
    .await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let account = op()?;
    let made = json!({"operation": account, "name": "registrar fixture"});
    ok(post(&service, "/service-accounts", Auth::Cookie(&admin), &made).await?)?;
    let registrar = json!({"operation": op()?, "service_account": account});
    let issued = ok(post(
        &service,
        "/apps/registrars",
        Auth::Cookie(&admin),
        &registrar,
    )
    .await?)?;
    let credential = issued["credential"]
        .as_str()
        .ok_or("no credential")?
        .to_owned();

    let probe = registration(NOTES, &workspace_schema(NOTES))?;
    let missing = post(&service, "/apps", Auth::Bearer(&credential), &probe).await?;
    assert_eq!(missing.1["refusal"], "NotHeld", "{}", missing.1);
    let owner = seeded.people[0].id.to_string();
    let root = ok(post(&service, "/grants/roots", Auth::Cookie(&admin), &json!({
        "operation":op()?,"route":"api","holder":owner,"resource":{"kind":"directory","id":"apps"},"relation":"editor",
        "pass_on":{"kind":"to","actions":["view","edit"],"recipients":["service_account"]},"window":{"starts_at":0,"ends_at":null}
    })).await?)?;
    ok(post(&service, "/grants", Auth::Cookie(&admin), &json!({
        "operation":op()?,"route":"api","source":root["grant"],"recipient":account,"responsible":owner,
        "resource":{"kind":"directory","id":"apps"},"relation":"editor","pass_on":{"kind":"use_only"},"window":{"starts_at":0,"ends_at":null}
    })).await?)?;

    let body = registration(NOTES, &workspace_schema(NOTES))?;
    let pending = ok(post(&service, "/apps", Auth::Bearer(&credential), &body).await?)?;
    assert_eq!(pending["state"], "pending");
    assert_eq!(
        pending["registered_by"]["kind"], "service_account",
        "{pending}"
    );
    assert_eq!(pending["service_account"], account.as_str());

    let mut refusals = 0;
    let bea = service.sign_in(login(BEA)).await?;
    let other = registration(FILES, &workspace_schema(FILES))?;
    refused(
        &post(&service, "/apps", Auth::Cookie(&bea), &other).await?,
        403,
        "NotAdmitted",
    )?;
    refusals += 1;
    let wrong = format!("lys-registrar.{account}.{}", "f".repeat(64));
    refused(
        &post(&service, "/apps", Auth::Bearer(&wrong), &other).await?,
        401,
        "credential_refused",
    )?;
    refusals += 1;
    let listed = ok(get(&service, "/apps", Auth::Cookie(&bea)).await?)?;
    let names: Vec<&str> = listed["apps"]
        .as_array()
        .ok_or("no apps")?
        .iter()
        .filter_map(|app| app["id"].as_str())
        .collect();
    assert_eq!(names, vec!["lys"], "a person sees approved apps only");
    refused(
        &get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&bea)).await?,
        404,
        "app_unknown",
    )?;
    refusals += 1;
    assert_eq!(refusals, 3);
    Ok(())
}

#[tokio::test]
async fn approval_binds_the_service_account_the_registration_names() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let account = op()?;
    let made = json!({"operation": account, "name": "app fixture"});
    ok(post(&service, "/service-accounts", Auth::Cookie(&admin), &made).await?)?;
    let mut body = registration(NOTES, &workspace_schema(NOTES))?;
    body["service_account"] = json!(account);
    ok(post(&service, "/apps", Auth::Cookie(&admin), &body).await?)?;
    let approval = ok(post(
        &service,
        &format!("/apps/{NOTES}/approve"),
        Auth::Cookie(&admin),
        &json!({"operation": op()?}),
    )
    .await?)?;
    assert_eq!(
        approval["app"]["service_account"],
        account.as_str(),
        "{approval}"
    );
    let credential = approval["client"]["credential"]
        .as_str()
        .ok_or("no credential")?;
    let me = ok(get(&service, "/apps/me", Auth::Bearer(credential)).await?)?;
    assert_eq!(me["service_account"], account.as_str());
    Ok(())
}
