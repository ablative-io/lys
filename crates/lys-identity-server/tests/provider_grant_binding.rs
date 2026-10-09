#![cfg(test)]
//! A pass's grant binding over HTTP (DIRECTORY-089 R2): asked for, it is
//! answered beside the pass, verified by lys-pass against that exact pass,
//! names the identified grant log and the revision the rights were decided
//! at, and maps each right to its grant's ancestry; a revoke then denies
//! exactly the dependent right through the stream index, and a refresh
//! carries no revoked dependency. Not asked for, the answer is as before.

use std::error::Error;
use std::num::NonZeroUsize;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use identity_contract::app_custody::secret;
use identity_contract::apps::{
    Auth, BEA, NOTES, TestResult, login, ok, op, post, registered, root,
};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::grants::change_stream::ChangesPage;
use lys_identity::grants::channel_membership_index::{DEPENDENCY_REVOKED, MembershipIndex};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::provider::{CODE_SECONDS, PASS_SECONDS, ProviderSettings};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_pass::binding::{BINDING_LOG_MISMATCH, BINDING_MISMATCH, VerifiedBinding};
use lys_pass::{GrantLog, KeySet, VerifiedPass};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const BACK: &str = "https://app.example.test/signed-in";
const VERIFIER: &str = "a-pass-verifier-of-enough-length-0123456789-abcdefghij";

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
    let back = answer
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or("the answer sends the browser nowhere")?
        .to_str()?;
    Ok(reqwest::Url::parse(back)?
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or("the app is given a code")?)
}

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

async fn signed_in(
    service: &Service,
    cookie: &str,
    binding: Option<&str>,
) -> Result<(u16, Value), Box<dyn Error>> {
    let code = code(service, cookie, NOTES).await?;
    let mut form = vec![
        ("grant_type", "authorization_code"),
        ("code", code.as_str()),
        ("redirect_uri", BACK),
        ("code_verifier", VERIFIER),
    ];
    if let Some(binding) = binding {
        form.push(("grant_binding", binding));
    }
    token(service, NOTES, &form).await
}

async fn published(service: &Service) -> Result<KeySet, Box<dyn Error>> {
    let text = reqwest::get(format!("{}/oauth/jwks", service.base))
        .await?
        .text()
        .await?;
    Ok(KeySet::from_json(&text)?)
}

async fn changes(
    service: &Service,
    credential: &str,
    body: &Value,
) -> Result<ChangesPage, Box<dyn Error>> {
    let answer = ok(post(service, "/grants/changes", Auth::Bearer(credential), body).await?)?;
    Ok(serde_json::from_value(answer)?)
}

/// The pass and binding of `answer`, both verified for NOTES on `log`.
async fn bound(
    service: &Service,
    answer: &Value,
    log: &GrantLog,
) -> Result<(VerifiedPass, VerifiedBinding), Box<dyn Error>> {
    let token = answer["access_token"].as_str().ok_or("a pass")?;
    let binding = answer["grant_binding"].as_str().ok_or("a binding")?;
    let keys = published(service).await?;
    let pass = VerifiedPass::verify(token, &keys, &service.base, NOTES, now()?)?;
    let verified = VerifiedBinding::verify(binding, token, &pass, &keys, log)?;
    Ok((pass, verified))
}

/// d089_r2_binding_and_dependency_counts: two independent grants, one
/// binding at a coherent revision; revoking one denies exactly its right,
/// the other stays admitted; a refresh carries no revoked dependency; a
/// binding moved to another pass or log is refused by name.
#[tokio::test]
async fn a_binding_maps_each_right_to_its_ancestry_and_one_revoke_denies_one() -> TestResult {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    let mut grants = Vec::new();
    for id in ["1", "2"] {
        let issued = ok(root(&service, &admin, &bea, (&doc, id), "reader").await?)?;
        grants.push(issued["grant"].as_str().ok_or("a grant")?.to_owned());
    }
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let (status, answer) = signed_in(&service, &bea_cookie, Some("1")).await?;
    assert_eq!(status, 200, "{answer}");
    let stream = changes(&service, &credential, &json!({"limit": 64})).await?;
    let log = GrantLog {
        identity: stream.log.identity.clone(),
        epoch: stream.log.epoch,
    };
    let (pass, binding) = bound(&service, &answer, &log).await?;
    assert_eq!(pass.claims().rights.len(), 2);
    for grant in &grants {
        let dependency = binding
            .dependency(grant)
            .ok_or("each right has its dependency")?;
        assert_eq!(
            dependency.path,
            vec![grant.clone()],
            "a root grant is its own ancestry"
        );
    }
    let mut index = MembershipIndex::new(
        stream.log.clone(),
        lys_identity::signer::load_service_key(&service.dir.path().join("service.key"))?
            .public_key_bytes(),
        NonZeroUsize::new(16).ok_or("a window")?,
    );
    index.apply_page(&stream.log, &stream.frames)?;
    assert!(index.cursor() >= Some(binding.revision()));
    for grant in &grants {
        index.admit(
            binding.revision(),
            &binding.dependency(grant).ok_or("dep")?.path,
        )?;
    }

    let revoke = json!({"operation": op()?, "route": "api", "reason": "left"});
    ok(post(
        &service,
        &format!("/grants/{}/revoke", grants[0]),
        Auth::Cookie(&admin),
        &revoke,
    )
    .await?)?;
    let cursor = index.cursor().ok_or("a cursor")?;
    let next = changes(
        &service,
        &credential,
        &json!({"log": stream.log, "after": cursor, "limit": 64}),
    )
    .await?;
    index.apply_page(&next.log, &next.frames)?;
    let lost = index.admit(
        binding.revision(),
        &binding.dependency(&grants[0]).ok_or("dep")?.path,
    );
    assert_eq!(
        lost.map_err(|refusal| refusal.name),
        Err(DEPENDENCY_REVOKED)
    );
    index.admit(
        binding.revision(),
        &binding.dependency(&grants[1]).ok_or("dep")?.path,
    )?;

    let refresh = answer["refresh_token"].as_str().ok_or("a refresh token")?;
    let (status, renewed) = token(
        &service,
        NOTES,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh),
            ("grant_binding", "1"),
        ],
    )
    .await?;
    assert_eq!(status, 200, "{renewed}");
    let (_, rebound) = bound(&service, &renewed, &log).await?;
    assert!(
        rebound.dependency(&grants[0]).is_none(),
        "the revoked dependency is gone"
    );
    assert!(rebound.revision() > binding.revision());

    // Substitutions, one at a time, each refused by name.
    let keys = published(&service).await?;
    let other_token = renewed["access_token"].as_str().ok_or("a pass")?;
    let other_pass = VerifiedPass::verify(other_token, &keys, &service.base, NOTES, now()?)?;
    let moved = VerifiedBinding::verify(
        answer["grant_binding"].as_str().ok_or("a binding")?,
        other_token,
        &other_pass,
        &keys,
        &log,
    );
    assert_eq!(
        moved.map_err(|error| error.name().to_owned()),
        Err(BINDING_MISMATCH.to_owned())
    );
    let foreign = GrantLog {
        identity: "elsewhere".to_owned(),
        epoch: 0,
    };
    let token = answer["access_token"].as_str().ok_or("a pass")?;
    let elsewhere = VerifiedBinding::verify(
        answer["grant_binding"].as_str().ok_or("a binding")?,
        token,
        &pass,
        &keys,
        &foreign,
    );
    assert_eq!(
        elsewhere.map_err(|error| error.name().to_owned()),
        Err(BINDING_LOG_MISMATCH.to_owned())
    );
    Ok(())
}

/// d089_r2_compatibility_and_refusal_counts: not asked for, no binding is
/// answered and the pass verifies as before; an unknown binding version is
/// refused by name before the code is spent.
#[tokio::test]
async fn an_unasked_binding_is_absent_and_an_unknown_version_is_refused() -> TestResult {
    let (service, _) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let (status, plain) = signed_in(&service, &bea_cookie, None).await?;
    assert_eq!(status, 200, "{plain}");
    assert!(plain.get("grant_binding").is_none(), "{plain}");
    let token = plain["access_token"].as_str().ok_or("a pass")?;
    VerifiedPass::verify(
        token,
        &published(&service).await?,
        &service.base,
        NOTES,
        now()?,
    )?;
    assert_eq!(
        lys_pass::binding::required(None).map_err(|error| error.name().to_owned()),
        Err("grant_binding_required".to_owned())
    );
    let (status, refused) = signed_in(&service, &bea_cookie, Some("2")).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "grant_binding_unsupported");
    Ok(())
}

/// A two-hop chain over HTTP: the administrator's root, passed on to Bea.
/// The binding names the whole ancestry, the stream names only the revoked
/// root, and the index resolves the descendant from the binding's path.
#[tokio::test]
async fn a_delegated_right_is_lost_when_its_ancestor_is_revoked() -> TestResult {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let (ada, bea) = (
        seeded.people[0].id.to_string(),
        seeded.people[1].id.to_string(),
    );
    let credential = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    let root_body = json!({
        "operation": op()?, "route": "api", "holder": ada,
        "resource": {"kind": doc, "id": "1"}, "relation": "reader",
        "pass_on": {"kind": "to", "actions": ["read"], "recipients": ["person"]},
        "window": {"starts_at": 0, "ends_at": null},
    });
    let root = ok(post(&service, "/grants/roots", Auth::Cookie(&admin), &root_body).await?)?;
    let root = root["grant"].as_str().ok_or("a root grant")?.to_owned();
    let passed_body = json!({
        "operation": op()?, "route": "api", "source": root, "recipient": bea,
        "responsible": bea, "resource": {"kind": doc, "id": "1"}, "relation": "reader",
        "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null},
    });
    let passed = ok(post(&service, "/grants", Auth::Cookie(&admin), &passed_body).await?)?;
    let passed = passed["grant"].as_str().ok_or("a passed grant")?.to_owned();

    let bea_cookie = service.sign_in(login(BEA)).await?;
    let (status, answer) = signed_in(&service, &bea_cookie, Some("1")).await?;
    assert_eq!(status, 200, "{answer}");
    let stream = changes(&service, &credential, &json!({"limit": 64})).await?;
    let log = GrantLog {
        identity: stream.log.identity.clone(),
        epoch: stream.log.epoch,
    };
    let (_, binding) = bound(&service, &answer, &log).await?;
    let path = binding
        .dependency(&passed)
        .ok_or("the passed right")?
        .path
        .clone();
    assert_eq!(
        path,
        vec![passed.clone(), root.clone()],
        "the whole ancestry"
    );

    let mut index = MembershipIndex::new(
        stream.log.clone(),
        lys_identity::signer::load_service_key(&service.dir.path().join("service.key"))?
            .public_key_bytes(),
        NonZeroUsize::new(16).ok_or("a window")?,
    );
    index.apply_page(&stream.log, &stream.frames)?;
    index.admit(binding.revision(), &path)?;
    let revoke = json!({"operation": op()?, "route": "api", "reason": "left"});
    ok(post(
        &service,
        &format!("/grants/{root}/revoke"),
        Auth::Cookie(&admin),
        &revoke,
    )
    .await?)?;
    let cursor = index.cursor().ok_or("a cursor")?;
    let next = changes(
        &service,
        &credential,
        &json!({"log": stream.log, "after": cursor, "limit": 64}),
    )
    .await?;
    let revoked: Vec<&str> = next
        .frames
        .iter()
        .filter_map(|frame| match frame {
            lys_identity::grants::change_stream::ChangeFrame::Change {
                grant,
                change: lys_identity::grants::change_stream::ChangeKind::Revoke,
                ..
            } => Some(grant.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        revoked,
        [root.as_str()],
        "only the revoked ancestor is named"
    );
    index.apply_page(&next.log, &next.frames)?;
    assert_eq!(
        index
            .admit(binding.revision(), &path)
            .map_err(|refusal| refusal.name),
        Err(DEPENDENCY_REVOKED)
    );
    Ok(())
}

/// The registry-side binding (`POST /grants/bindings`): a pass Lys did not
/// issue is bound to the grants it names after Lys judges each; the same
/// verifier reads it; a revoked or foreign grant refuses the whole binding.
#[tokio::test]
async fn a_registry_pass_is_bound_only_to_grants_lys_judges_live() -> TestResult {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let (ada, bea) = (
        seeded.people[0].id.to_string(),
        seeded.people[1].id.to_string(),
    );
    let credential = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    let held = ok(root(&service, &admin, &bea, (&doc, "1"), "reader").await?)?;
    let held = held["grant"].as_str().ok_or("a grant")?.to_owned();
    let other = ok(root(&service, &admin, &ada, (&doc, "2"), "reader").await?)?;
    let other = other["grant"].as_str().ok_or("a grant")?.to_owned();
    let digest = lys_pass::binding::pass_digest("registry-issued pass bytes");
    let ask = |grants: Vec<&str>| {
        json!({
            "binding": 1, "pass": digest, "iss": "https://registry.example.test",
            "sub": bea, "aud": NOTES, "iat": 100, "exp": 200, "grants": grants,
        })
    };
    let answer = ok(post(
        &service,
        "/grants/bindings",
        Auth::Bearer(&credential),
        &ask(vec![held.as_str()]),
    )
    .await?)?;
    let stream = changes(&service, &credential, &json!({"limit": 64})).await?;
    let log = GrantLog {
        identity: stream.log.identity.clone(),
        epoch: stream.log.epoch,
    };
    let grants = std::collections::BTreeSet::from([held.as_str()]);
    let verified = VerifiedBinding::verify_bound(
        answer["grant_binding"].as_str().ok_or("a binding")?,
        &lys_pass::binding::Bound {
            pass: &digest,
            iss: "https://registry.example.test",
            sub: &bea,
            aud: NOTES,
            iat: 100,
            exp: 200,
            grants: &grants,
        },
        &published(&service).await?,
        &log,
    )?;
    assert_eq!(Some(verified.revision()), answer["revision"].as_u64());
    assert_eq!(
        verified.dependency(&held).ok_or("dep")?.path,
        vec![held.clone()]
    );

    let foreign = post(
        &service,
        "/grants/bindings",
        Auth::Bearer(&credential),
        &ask(vec![other.as_str()]),
    )
    .await?;
    assert_eq!(foreign.1["refusal"], "NotHolder", "{foreign:?}");
    let revoke = json!({"operation": op()?, "route": "api", "reason": "left"});
    ok(post(
        &service,
        &format!("/grants/{held}/revoke"),
        Auth::Cookie(&admin),
        &revoke,
    )
    .await?)?;
    let revoked = post(
        &service,
        "/grants/bindings",
        Auth::Bearer(&credential),
        &ask(vec![held.as_str()]),
    )
    .await?;
    assert_eq!(revoked.1["refusal"], "Revoked", "{revoked:?}");
    Ok(())
}
