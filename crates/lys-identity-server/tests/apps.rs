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
async fn an_app_bearer_issue_answers_once_and_rotation_ends_the_old_value() -> TestResult {
    let (mut service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    register(&service, &admin, NOTES).await?;
    approve(&service, &admin, NOTES).await?;
    let path = format!("/apps/{NOTES}/bearer/issue");
    let body = json!({"operation": op()?});
    let first = ok(post(&service, &path, Auth::Cookie(&admin), &body).await?)?;
    let credential = first["credential"].as_str().ok_or("no issued bearer")?;
    assert!(credential.starts_with(&format!("lys-app.{NOTES}.")));
    let reference = first["reference"].as_str().ok_or("no issued reference")?;
    let repeated = ok(post(&service, &path, Auth::Cookie(&admin), &body).await?)?;
    assert_eq!(repeated["reference"], reference);
    assert!(repeated["credential"].is_null());
    ok(get(&service, "/apps/me", Auth::Bearer(credential)).await?)?;
    let rotation = json!({"operation": op()?});
    let second = ok(post(&service, &path, Auth::Cookie(&admin), &rotation).await?)?;
    let rotated = second["credential"].as_str().ok_or("no rotated bearer")?;
    assert_ne!(rotated, credential);
    refused(
        &get(&service, "/apps/me", Auth::Bearer(credential)).await?,
        401,
        "credential_refused",
    )?;
    ok(get(&service, "/apps/me", Auth::Bearer(rotated)).await?)?;
    assert!(!held_anywhere(service.dir.path(), credential.as_bytes())?);
    assert!(!held_anywhere(service.dir.path(), rotated.as_bytes())?);
    service.restart().await?;
    refused(
        &get(&service, "/apps/me", Auth::Bearer(credential)).await?,
        401,
        "credential_refused",
    )?;
    ok(get(&service, "/apps/me", Auth::Bearer(rotated)).await?)?;
    let replayed = ok(post(&service, &path, Auth::Cookie(&admin), &rotation).await?)?;
    assert_eq!(replayed["reference"], second["reference"]);
    assert!(replayed["credential"].is_null());
    assert!(!held_anywhere(service.dir.path(), rotated.as_bytes())?);
    Ok(())
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
    let body = json!({"operation": operation, "redirects": ["https://notes.example.test/signed-in"], "profile": false});
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
    let other = json!({"operation": op()?, "redirects": ["https://notes.example.test/signed-in"], "profile": false});
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
    let approve = json!({"operation": op()?, "redirects": ["https://notes.example.test/signed-in"], "profile": false});
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

/// DIRECTORY-080 R1 (box 12.2): approving an app writes its connector line
/// in the approval's own batch, answering to the person who holds the
/// approving administrator's login.
#[tokio::test]
async fn an_approval_writes_its_connector_answering_to_the_approving_person() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = seeded
        .people
        .iter()
        .find(|person| person.subject == ADMINISTRATOR)
        .ok_or("the administrator is a seeded person")?
        .id;
    register(&service, &admin, NOTES).await?;
    approve(&service, &admin, NOTES).await?;
    let dir = service.dir.path();
    assert!(held_anywhere(dir, br#""line":"connector""#)?);
    assert!(held_anywhere(dir, br#""connector":"connector-"#)?);
    assert!(held_anywhere(
        dir,
        format!(r#""approver":"{person}""#).as_bytes()
    )?);
    Ok(())
}

/// DIRECTORY-080 R1 (box 12.2): an approving login no person holds is
/// refused `connector_needs_a_person`, and the app stays pending with nothing
/// written for it.
#[tokio::test]
async fn an_approver_whose_login_no_person_holds_is_refused_and_nothing_is_approved() -> TestResult
{
    let broker = identity_contract::app_custody::start().await?;
    let (service, _) = identity_contract::harness::Service::start_adjusted(
        identity_contract::harness::GRANT_MODEL,
        None,
        None,
        None,
        move |config| {
            config.secrets = Some(lys_identity_server::secrets_api::SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            });
        },
        // Registering through the service judges a grant under the
        // administrator's person, which this administrator has not; the app
        // is registered in the store, as one registered before it was.
        |config| {
            use lys_identity_server::apps_state::{By, Line, Registered};
            let seeded =
                lys_identity_server::dev_seed::seed_configured(config, [BEA, "cara-subject"])?;
            let key = std::sync::Arc::new(lys_core::Ed25519Identity::load(&config.event_key_file)?);
            let mut apps =
                lys_identity_server::apps_store::AppStore::open(&config.apps_dir(), key)?;
            apps.keep(Line::Registered(Registered {
                operation: op()?,
                app: NOTES.to_owned(),
                name: "notes".to_owned(),
                redirects: vec!["https://notes.example.test/signed-in".to_owned()],
                schema: workspace_schema(NOTES),
                service_account: None,
                by: By::Operator {
                    login: lys_identity_server::read_views::Login {
                        provider: config.issuer.clone(),
                        subject: ADMINISTRATOR.to_owned(),
                    },
                },
                at: 1,
            }))?;
            Ok(seeded)
        },
    )
    .await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let body = json!({"operation": op()?, "redirects": ["https://notes.example.test/signed-in"], "profile": false});
    let answer = post(
        &service,
        &format!("/apps/{NOTES}/approve"),
        Auth::Cookie(&admin),
        &body,
    )
    .await?;
    refused(&answer, 403, "connector_needs_a_person")?;
    let app = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(app["state"], "pending", "{app}");
    let dir = service.dir.path();
    assert!(!held_anywhere(dir, br#""line":"approved""#)?);
    assert!(!held_anywhere(dir, br#""line":"connector""#)?);
    Ok(())
}

/// A service whose apps log holds `NOTES` approved by the line an approval
/// wrote before connectors, with no connector line, and the seeded people.
async fn approved_before_connectors() -> Result<
    (
        identity_contract::harness::Service,
        lys_identity_server::dev_seed::Seeded,
    ),
    Box<dyn Error>,
> {
    use lys_identity_server::apps_state::{Approved, By, Client, Line, Registered};
    identity_contract::harness::Service::start_adjusted(
        identity_contract::harness::GRANT_MODEL,
        None,
        None,
        None,
        |_| {},
        |config| {
            let seeded =
                lys_identity_server::dev_seed::seed_configured(config, [ADMINISTRATOR, BEA])?;
            let key = std::sync::Arc::new(lys_core::Ed25519Identity::load(&config.event_key_file)?);
            let mut apps =
                lys_identity_server::apps_store::AppStore::open(&config.apps_dir(), key)?;
            let by = By::Operator {
                login: lys_identity_server::read_views::Login {
                    provider: config.issuer.clone(),
                    subject: ADMINISTRATOR.to_owned(),
                },
            };
            apps.keep(Line::Registered(Registered {
                operation: op()?,
                app: NOTES.to_owned(),
                name: "notes approved before connectors".to_owned(),
                redirects: vec!["https://notes.example.test/signed-in".to_owned()],
                schema: workspace_schema(NOTES),
                service_account: None,
                by: by.clone(),
                at: 1,
            }))?;
            apps.keep(Line::Approved(Approved {
                operation: op()?,
                app: NOTES.to_owned(),
                client: Client {
                    client_id: NOTES.to_owned(),
                    secret_sha256: "cd".repeat(32),
                },
                binding: None,
                by,
                at: 2,
            }))?;
            Ok(seeded)
        },
    )
    .await
}

/// DIRECTORY-080 R1 (box 12.3): an app approved before connectors holds
/// none until the administrator gives it one, answering to the person who
/// holds the administrator's login; the same act sent again answers the
/// same, and a second connector is refused by name.
#[tokio::test]
async fn an_app_approved_before_connectors_is_given_one_connector_by_the_administrator()
-> TestResult {
    let (service, seeded) = approved_before_connectors().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = seeded
        .people
        .iter()
        .find(|person| person.subject == ADMINISTRATOR)
        .ok_or("the administrator is a seeded person")?
        .id
        .to_string();
    let path = format!("/apps/{NOTES}/connector");
    let before = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(before["state"], "approved", "{before}");
    assert_eq!(before["connector"], Value::Null, "{before}");

    let bea = service.sign_in(login(BEA)).await?;
    let body = json!({"operation": op()?});
    refused(
        &post(&service, &path, Auth::Cookie(&bea), &body).await?,
        403,
        "NotAdmitted",
    )?;
    let given = ok(post(&service, &path, Auth::Cookie(&admin), &body).await?)?;
    let connector = &given["connector"];
    let id = connector["id"]
        .as_str()
        .ok_or("the view names its connector")?;
    let hex = id.strip_prefix("connector-").ok_or("a connector id")?;
    assert!(
        hex.len() == 32 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "{id}"
    );
    assert_eq!(connector["approver"], person.as_str(), "{given}");
    assert_eq!(connector["operation"], body["operation"], "{given}");

    let again = ok(post(&service, &path, Auth::Cookie(&admin), &body).await?)?;
    assert_eq!(
        again["connector"], given["connector"],
        "the same act answers the same"
    );
    refused(
        &post(
            &service,
            &path,
            Auth::Cookie(&admin),
            &json!({"operation": op()?}),
        )
        .await?,
        409,
        "connector_exists",
    )?;
    let after = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(after["connector"], given["connector"], "{after}");
    let lines = br#""line":"connector""#;
    let dir = service.dir.path();
    assert!(held_anywhere(dir, lines)?);
    Ok(())
}

/// DIRECTORY-080 R1 (box 12.3): a connector is given only to an approved
/// app that is not Lys and holds none, each refusal by its name; an
/// operation that names another act is refused, and nothing is written.
#[tokio::test]
async fn a_connector_is_given_only_to_an_approved_app_without_one() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let give = |app: &str| format!("/apps/{app}/connector");
    let mut refusals = 0;
    for (app, status, name) in [
        ("fixture_unknown", 404, "app_unknown"),
        ("lys", 403, "app_is_lys"),
    ] {
        let body = json!({"operation": op()?});
        refused(
            &post(&service, &give(app), Auth::Cookie(&admin), &body).await?,
            status,
            name,
        )?;
        refusals += 1;
    }
    register(&service, &admin, NOTES).await?;
    let body = json!({"operation": op()?});
    refused(
        &post(&service, &give(NOTES), Auth::Cookie(&admin), &body).await?,
        403,
        "app_not_approved",
    )?;
    refusals += 1;
    let approval = json!({"operation": op()?, "redirects": ["https://notes.example.test/signed-in"], "profile": false});
    ok(post(
        &service,
        &format!("/apps/{NOTES}/approve"),
        Auth::Cookie(&admin),
        &approval,
    )
    .await?)?;
    let approved = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert!(approved["connector"]["id"].is_string(), "{approved}");
    // The give act under the approval's operation: one operation is one act.
    let reused = json!({"operation": approval["operation"]});
    refused(
        &post(&service, &give(NOTES), Auth::Cookie(&admin), &reused).await?,
        409,
        "app_operation_reused",
    )?;
    refusals += 1;
    let body = json!({"operation": op()?});
    refused(
        &post(&service, &give(NOTES), Auth::Cookie(&admin), &body).await?,
        409,
        "connector_exists",
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
    let body = json!({"operation": op()?});
    refused(
        &post(&service, &give(NOTES), Auth::Cookie(&admin), &body).await?,
        403,
        "app_retired",
    )?;
    refusals += 1;
    assert_eq!(refusals, 6);
    let after = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(after["connector"], approved["connector"], "{after}");
    Ok(())
}

/// DIRECTORY-080 R1 (box 12.5): an approved app acts in the engine as its
/// connector. Holding nothing it may do nothing; lent `read` by the person
/// who approved it, it may read and no more, and the grant answers to that
/// person: one naming anyone else as responsible is refused by name.
#[tokio::test]
async fn an_approved_apps_connector_is_checked_on_what_its_approver_lent_it() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = seeded
        .people
        .iter()
        .find(|person| person.subject == ADMINISTRATOR)
        .ok_or("the administrator is a seeded person")?
        .id
        .to_string();
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let app = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    let connector = app["connector"]["id"]
        .as_str()
        .ok_or("an approved app holds its connector")?
        .to_owned();
    assert_eq!(app["connector"]["approver"], person.as_str(), "{app}");
    let doc = format!("{NOTES}.doc");
    let checks = json!({"checks": [
        check(&connector, &doc, "1", "read"),
        check(&connector, &doc, "1", "write"),
    ]});
    let checked = |answer: &Value, index: usize| answer["results"][index]["allowed"].clone();
    let before = ok(post(
        &service,
        "/grants/check/batch",
        Auth::Cookie(&admin),
        &checks,
    )
    .await?)?;
    assert_eq!(checked(&before, 0), false, "nothing granted: {before}");
    assert_eq!(before["results"][0]["refusal"], "NotHeld", "{before}");

    let lending = json!({
        "operation": op()?, "route": "api", "holder": person,
        "resource": {"kind": doc, "id": "1"}, "relation": "reader",
        "pass_on": {"kind": "to", "actions": ["read"], "recipients": ["connector"]},
        "window": {"starts_at": 0, "ends_at": null},
    });
    let lending = ok(post(&service, "/grants/roots", Auth::Cookie(&admin), &lending).await?)?;
    let lent = |responsible: &str| -> Result<Value, Box<dyn Error>> {
        Ok(json!({
            "operation": op()?, "route": "api", "source": lending["grant"],
            "recipient": connector, "responsible": responsible,
            "resource": {"kind": doc, "id": "1"}, "relation": "reader",
            "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null},
        }))
    };
    refused(
        &post(&service, "/grants", Auth::Cookie(&admin), &lent(&bea)?).await?,
        403,
        "ResponsibleMismatch",
    )?;
    ok(post(&service, "/grants", Auth::Cookie(&admin), &lent(&person)?).await?)?;
    let after = ok(post(
        &service,
        "/grants/check/batch",
        Auth::Cookie(&admin),
        &checks,
    )
    .await?)?;
    assert_eq!(checked(&after, 0), true, "lent read: {after}");
    assert_eq!(checked(&after, 1), false, "lent no write: {after}");
    Ok(())
}

/// DIRECTORY-080 R1 (box 12.5): an identity is read by its prefix, and a
/// prefix that names no kind of identity is refused by name.
#[tokio::test]
async fn an_identity_of_no_known_kind_is_refused_by_name() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let unknown = format!("robot-{}", "ab".repeat(16));
    let answer = get(
        &service,
        &format!("/identities/{unknown}"),
        Auth::Cookie(&admin),
    )
    .await?;
    refused(&answer, 400, "IdentifierMalformed")?;
    assert!(
        answer
            .1
            .to_string()
            .contains("person, agent, service account, connector or machine"),
        "{}",
        answer.1
    );
    Ok(())
}

#[path = "shared/app_registration.rs"]
mod app_registration;
