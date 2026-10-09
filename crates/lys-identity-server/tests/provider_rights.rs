#![cfg(test)]
//! The pass over HTTP (ACCESS-002 R1, R2): the access token the token route
//! answers is a pass lys-pass's own verifier accepts, carrying the holder's
//! rights for its audience app alone, reach and roles applied at issue; a
//! refresh token issues a new pass from the grants live at that moment, and
//! is refused by name once the holder is retired or the sign-in has ended.

use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use identity_contract::app_custody::secret;
use identity_contract::apps::{
    Auth, BEA, FILES, NOTES, TestResult, login, ok, op, post, put, registered, root,
    workspace_schema,
};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::provider::{CODE_SECONDS, PASS_SECONDS, ProviderSettings};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_pass::{Decision, KeySet, Mode, VerifiedPass};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The address every fixture app's registration lists.
const BACK: &str = "https://app.example.test/signed-in";
const VERIFIER: &str = "a-pass-verifier-of-enough-length-0123456789-abcdefghij";

/// The seeded service with Lys's provider on, its signing key written.
async fn world() -> Result<(Service, Seeded), Box<dyn Error>> {
    let broker = identity_contract::app_custody::start().await?;
    Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        move |config| {
            config.secrets = Some(SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            });
            config.provider = Some(ProviderSettings {
                key_file: config.log_dir.with_file_name("provider.key"),
                code_seconds: CODE_SECONDS,
                pass_seconds: PASS_SECONDS,
                rights_bytes: None,
            });
        },
        |config| {
            std::fs::write(config.log_dir.with_file_name("provider.key"), [6u8; 32])?;
            Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)
        },
    )
    .await
}

fn now() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

/// A code for `app`, as the browser signed in with `cookie` is handed it.
async fn code(service: &Service, cookie: &str, app: &str) -> Result<String, Box<dyn Error>> {
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let answer = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get(format!("{}/oauth/authorize", service.base))
        .query(&[
            ("client_id", app),
            ("redirect_uri", BACK),
            ("response_type", "code"),
            ("scope", "openid"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?;
    assert_eq!(answer.status(), 303);
    let back = answer
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or("the answer sends the browser nowhere")?
        .to_str()?;
    let back = reqwest::Url::parse(back)?;
    Ok(back
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or("the app is given a code")?)
}

/// The token route's answer to `app` for `form`, with the app's secret.
async fn token(
    service: &Service,
    app: &str,
    form: &[(&str, &str)],
) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!("{}/oauth/token", service.base))
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Basic {}", STANDARD.encode(format!("{app}:{}", secret()))),
        )
        .form(form)
        .send()
        .await?;
    let status = answer.status().as_u16();
    Ok((status, serde_json::from_str(&answer.text().await?)?))
}

/// The person signed in with `cookie` signs in to `app`: the token answer.
async fn signed_in(service: &Service, cookie: &str, app: &str) -> Result<Value, Box<dyn Error>> {
    let code = code(service, cookie, app).await?;
    let (status, answer) = token(
        service,
        app,
        &[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", BACK),
            ("code_verifier", VERIFIER),
        ],
    )
    .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(answer)
}

async fn refreshed(
    service: &Service,
    app: &str,
    answer: &Value,
) -> Result<(u16, Value), Box<dyn Error>> {
    let refresh = answer["refresh_token"]
        .as_str()
        .ok_or("a refresh token is issued")?;
    token(
        service,
        app,
        &[("grant_type", "refresh_token"), ("refresh_token", refresh)],
    )
    .await
}

/// The keys Lys publishes, as a product reads them.
async fn published(service: &Service) -> Result<KeySet, Box<dyn Error>> {
    let text = reqwest::get(format!("{}/oauth/jwks", service.base))
        .await?
        .text()
        .await?;
    Ok(KeySet::from_json(&text)?)
}

/// The answer's pass, verified by lys-pass for the audience `app`.
async fn verified(
    service: &Service,
    answer: &Value,
    app: &str,
) -> Result<VerifiedPass, Box<dyn Error>> {
    let pass = answer["access_token"].as_str().ok_or("a pass is issued")?;
    Ok(VerifiedPass::verify(
        pass,
        &published(service).await?,
        &service.base,
        app,
        now()?,
    )?)
}

/// Each right of `pass` as `kind id actions grant`.
fn rights(pass: &VerifiedPass) -> Vec<(String, String, Vec<String>, String)> {
    pass.claims()
        .rights
        .iter()
        .map(|right| {
            (
                right.resource.kind.clone(),
                right.resource.id.clone(),
                right.actions.clone(),
                right.grant.clone(),
            )
        })
        .collect()
}

fn grant_of(issued: &Value) -> Result<String, Box<dyn Error>> {
    Ok(issued["grant"]
        .as_str()
        .ok_or("the issue names its grant")?
        .to_owned())
}

/// R1, acceptance 1: a person with grants on two apps gets a pass for one
/// naming that app's rights only, which lys-pass verifies for that app, and
/// refuses as another app's.
#[tokio::test]
async fn a_pass_for_one_app_names_its_rights_only_and_lys_pass_verifies_it() -> TestResult {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    registered(&service, &admin, FILES).await?;
    let notes_doc = format!("{NOTES}.doc");
    let files_doc = format!("{FILES}.doc");
    let edits = grant_of(&ok(root(
        &service,
        &admin,
        &bea,
        (&notes_doc, "1"),
        "editor",
    )
    .await?)?)?;
    ok(root(&service, &admin, &bea, (&files_doc, "9"), "reader").await?)?;

    let bea_cookie = service.sign_in(login(BEA)).await?;
    let answer = signed_in(&service, &bea_cookie, NOTES).await?;
    let pass = verified(&service, &answer, NOTES).await?;
    let claims = pass.claims();
    assert_eq!(claims.sub, bea);
    assert_eq!(claims.holder.id, bea);
    assert_eq!(claims.holder.kind, "person");
    assert_eq!(claims.holder.responsible, None);
    assert_eq!(claims.rights_truncated, None);
    assert_eq!(
        rights(&pass),
        [(
            notes_doc.clone(),
            "1".to_owned(),
            vec!["read".to_owned(), "write".to_owned()],
            edits.clone()
        )]
    );
    assert_eq!(claims.rights[0].mode, Mode::Outright);
    assert!(
        claims.exp > claims.iat && claims.exp - claims.iat <= PASS_SECONDS,
        "the pass lives its lifetime at most: {} to {}",
        claims.iat,
        claims.exp
    );
    assert_eq!(
        pass.evaluate(&notes_doc, "1", "write", now()?)?,
        Decision::Allowed { grant: &edits }
    );
    let token = answer["access_token"].as_str().ok_or("a pass")?;
    let elsewhere = VerifiedPass::verify(
        token,
        &published(&service).await?,
        &service.base,
        FILES,
        now()?,
    );
    assert!(
        matches!(elsewhere, Err(lys_pass::Error::WrongAudience)),
        "a pass for one app is refused as another's"
    );
    Ok(())
}

/// R1, acceptance 3: a right held on a parent appears for the child placed
/// in it; a child placed restricted does not.
#[tokio::test]
async fn a_right_on_a_parent_reaches_the_placed_child_and_not_a_restricted_one() -> TestResult {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let (channel, workspace) = (format!("{NOTES}.channel"), format!("{NOTES}.workspace"));
    for (id, restricted) in [("general", false), ("ward_private", true)] {
        let place = json!({
            "operation": op()?,
            "child": {"kind": channel, "id": id},
            "parent": {"kind": workspace, "id": "team"},
            "restricted": restricted,
        });
        ok(post(
            &service,
            &format!("/apps/{NOTES}/placements"),
            Auth::Bearer(&credential),
            &place,
        )
        .await?)?;
    }
    let member = grant_of(&ok(root(
        &service,
        &admin,
        &bea,
        (&workspace, "team"),
        "member",
    )
    .await?)?)?;

    let bea_cookie = service.sign_in(login(BEA)).await?;
    let answer = signed_in(&service, &bea_cookie, NOTES).await?;
    let pass = verified(&service, &answer, NOTES).await?;
    let read = vec!["read".to_owned()];
    assert_eq!(
        rights(&pass),
        [
            (
                channel.clone(),
                "general".to_owned(),
                read.clone(),
                member.clone()
            ),
            (workspace.clone(), "team".to_owned(), read, member),
        ]
    );
    assert_eq!(
        pass.evaluate(&channel, "ward_private", "read", now()?)?,
        Decision::Refused,
        "a restricted child is not reached from its parent"
    );
    Ok(())
}

/// R1 with ACCESS-004: a grant naming a role is carried as the role's
/// current actions; no right names a role.
#[tokio::test]
async fn a_grant_naming_a_role_is_carried_as_the_roles_actions() -> TestResult {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let workspace = format!("{NOTES}.workspace");
    let mut schema = workspace_schema(NOTES);
    schema["kinds"][&workspace]["roles"] = json!({"administrator": ["read", "write"]});
    let roles = json!({"operation": op()?, "replaces": 1, "schema": schema});
    let applied = ok(put(
        &service,
        &format!("/apps/{NOTES}/schema"),
        Auth::Cookie(&admin),
        &roles,
    )
    .await?)?;
    assert_eq!(applied["applied"], true, "{applied}");
    let role = grant_of(&ok(root(
        &service,
        &admin,
        &bea,
        (&workspace, "ward"),
        "administrator",
    )
    .await?)?)?;

    let bea_cookie = service.sign_in(login(BEA)).await?;
    let pass = verified(
        &service,
        &signed_in(&service, &bea_cookie, NOTES).await?,
        NOTES,
    )
    .await?;
    assert_eq!(
        rights(&pass),
        [(
            workspace,
            "ward".to_owned(),
            vec!["read".to_owned(), "write".to_owned()],
            role
        )]
    );
    assert!(
        pass.claims()
            .rights
            .iter()
            .all(|right| !right.actions.iter().any(|action| action == "administrator")),
        "a pass carries actions, never a role's name"
    );
    Ok(())
}

/// R2, acceptance 1: revoke a grant, refresh: the right is gone from the new
/// pass, which lys-pass verifies; the refresh grant is advertised.
#[tokio::test]
async fn a_refresh_reads_the_live_grants_so_a_revoked_right_is_gone() -> TestResult {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    let kept = grant_of(&ok(
        root(&service, &admin, &bea, (&doc, "1"), "reader").await?
    )?)?;
    let revoked = grant_of(&ok(
        root(&service, &admin, &bea, (&doc, "2"), "reader").await?
    )?)?;

    let (status, discovery) = service
        .get("/.well-known/openid-configuration", None)
        .await?;
    assert_eq!(status, 200, "{discovery}");
    assert_eq!(
        discovery["grant_types_supported"],
        json!(["authorization_code", "refresh_token"])
    );

    let bea_cookie = service.sign_in(login(BEA)).await?;
    let answer = signed_in(&service, &bea_cookie, NOTES).await?;
    let first = verified(&service, &answer, NOTES).await?;
    assert_eq!(first.claims().rights.len(), 2, "{:?}", rights(&first));

    let body = json!({"operation": op()?, "route": "api", "reason": "no longer needed"});
    let (status, ended) = service
        .post(&format!("/grants/{revoked}/revoke"), Some(&admin), &body)
        .await?;
    assert_eq!(status, 200, "{ended}");
    let (status, again) = refreshed(&service, NOTES, &answer).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["refresh_token"], answer["refresh_token"]);
    let next = verified(&service, &again, NOTES).await?;
    assert_eq!(
        rights(&next),
        [(doc, "1".to_owned(), vec!["read".to_owned()], kept)]
    );

    // A refresh token is no access token, and one Lys never gave is refused.
    let refresh = answer["refresh_token"].as_str().ok_or("a refresh token")?;
    let userinfo = reqwest::Client::new()
        .get(format!("{}/oauth/userinfo", service.base))
        .bearer_auth(refresh)
        .send()
        .await?;
    assert_eq!(userinfo.status(), 401);
    let body: Value = userinfo.json().await?;
    assert_eq!(body["refusal"], "TokenUnknown");
    let (status, unknown) = token(
        &service,
        NOTES,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", "not-one-lys-gave"),
        ],
    )
    .await?;
    assert_eq!(status, 400, "{unknown}");
    assert_eq!(unknown["refusal"], "RefreshUnknown");
    Ok(())
}

/// R2, acceptance 2: retire the holder, refresh: refused by name.
#[tokio::test]
async fn a_refresh_for_a_retired_holder_is_refused_by_name() -> TestResult {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let answer = signed_in(&service, &bea_cookie, NOTES).await?;
    let retire = json!({"operation": op()?, "transition": "retire", "reason": "left"});
    let (status, retired) = service
        .post(
            &format!("/identities/{bea}/transitions"),
            Some(&admin),
            &retire,
        )
        .await?;
    assert_eq!(status, 200, "{retired}");
    let (status, refused) = refreshed(&service, NOTES, &answer).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "HolderRetired", "{refused}");
    assert!(refused.get("access_token").is_none());
    Ok(())
}

/// R2: once the sign-in a refresh token stands on has ended, the refresh is
/// refused by name.
#[tokio::test]
async fn a_refresh_after_its_sign_in_ended_is_refused_by_name() -> TestResult {
    let (service, _seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let answer = signed_in(&service, &bea_cookie, NOTES).await?;
    let (status, sessions) = service.get("/sessions", Some(&bea_cookie)).await?;
    assert_eq!(status, 200, "{sessions}");
    let id = sessions["sessions"]
        .as_array()
        .ok_or("sessions is not an array")?
        .iter()
        .find(|session| session["current"] == true)
        .and_then(|session| session["id"].as_str())
        .ok_or("the current session is absent")?;
    let (status, ended) = service
        .post(
            &format!("/sessions/{id}/end"),
            Some(&bea_cookie),
            &json!({}),
        )
        .await?;
    assert_eq!(status, 200, "{ended}");
    let (status, refused) = refreshed(&service, NOTES, &answer).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "SessionEnded", "{refused}");
    let (status, again) = refreshed(&service, NOTES, &answer).await?;
    assert_eq!(status, 400, "{again}");
    assert_eq!(
        again["refusal"], "RefreshUnknown",
        "a refresh token whose sign-in ended is gone: {again}"
    );
    Ok(())
}
