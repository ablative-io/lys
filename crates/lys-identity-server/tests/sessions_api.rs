//! The session routes: a person's live sessions across every login bound to
//! them, ending one signs it out, another person's session refused exactly as
//! one never held, ending the current session clears the cookie, the
//! administrator's routes for any person, and no answer carrying a cookie
//! secret. Every answer is read into the route's own wire type.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{OperationId, PersonId};
use lys_identity_server::dev_seed::{Seeded, SeededPerson, seed_configured};
use lys_identity_server::session::COOKIE;
use lys_identity_server::sessions_api::{EndedView, SessionsView};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const ADA: &str = "ada-subject";
const ADA_SECOND: &str = "ada-second-subject";
const BEA: &str = "bea-subject";
const NEVER_HELD: &str = "00000000000000000000000000000000";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

async fn seeded() -> Result<(Service, Seeded), Box<dyn Error>> {
    Service::start_with(|config| Ok(seed_configured(config, [ADA, BEA])?)).await
}

fn person<'a>(seeded: &'a Seeded, subject: &str) -> Result<&'a SeededPerson, Box<dyn Error>> {
    Ok(seeded
        .people
        .iter()
        .find(|person| person.subject == subject)
        .ok_or("the seed holds no such person")?)
}

/// Bind a second login to Ada through the administrator's route.
async fn bind_second_login(service: &Service, seeded: &Seeded, administrator: &str) -> TestResult {
    let ada = person(seeded, ADA)?;
    let body = json!({
        "operation": OperationId::generate()?.to_string(),
        "issuer": service.issuer.issuer(),
        "subject": ADA_SECOND,
    });
    let path = format!("/people/{}/logins", ada.id);
    let (status, answer) = service.post(&path, Some(administrator), &body).await?;
    assert_eq!(status, 200, "{answer}");
    Ok(())
}

/// Read a 200 answer into the route's wire type, checking it carries no cookie secret.
fn typed<T: DeserializeOwned>(
    path: &str,
    (status, body): (u16, Value),
    cookies: &[&str],
) -> Result<T, Box<dyn Error>> {
    assert_eq!(status, 200, "{path}: {body}");
    assert_carries_no_secret(&body, cookies)?;
    Ok(serde_json::from_value(body)?)
}

/// GET `path` as `cookie`, which must answer 200.
async fn list(
    service: &Service,
    path: &str,
    cookie: &str,
    cookies: &[&str],
) -> Result<SessionsView, Box<dyn Error>> {
    typed(path, service.get(path, Some(cookie)).await?, cookies)
}

/// POST an end to `path` as `cookie`.
async fn end(service: &Service, path: &str, cookie: &str) -> Result<(u16, Value), Box<dyn Error>> {
    service.post(path, Some(cookie), &json!({})).await
}

/// The secret a `name=value` cookie carries.
fn secret(cookie: &str) -> Result<&str, Box<dyn Error>> {
    Ok(cookie
        .split_once('=')
        .map(|(_, value)| value)
        .ok_or("the cookie carries no value")?)
}

fn assert_carries_no_secret(body: &Value, cookies: &[&str]) -> TestResult {
    let text = body.to_string();
    for cookie in cookies {
        let value = secret(cookie)?;
        assert!(
            !text.contains(value),
            "an answer carried a cookie secret: {text}"
        );
    }
    Ok(())
}

/// The public id of the session signed in through `subject`, as `view` lists it.
fn id_of(view: &SessionsView, subject: &str, current: bool) -> Result<String, Box<dyn Error>> {
    Ok(view
        .sessions
        .iter()
        .find(|session| session.login.subject == subject && session.current == current)
        .ok_or("no such session is listed")?
        .id
        .clone())
}

#[tokio::test]
async fn a_persons_sessions_span_every_login_bound_to_them_newest_first() -> TestResult {
    let (service, seeded) = seeded().await?;
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    bind_second_login(&service, &seeded, &administrator).await?;
    let first = service.sign_in(login(ADA)).await?;
    let second = service.sign_in(login(ADA_SECOND)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let cookies = [
        first.as_str(),
        second.as_str(),
        bea.as_str(),
        &administrator,
    ];
    let ada = person(&seeded, ADA)?;
    let from_first = list(&service, "/sessions", &first, &cookies).await?;
    assert_eq!(from_first.person, ada.id.to_string());
    assert_eq!(from_first.sessions.len(), 2, "{from_first:?}");
    let mut subjects = from_first
        .sessions
        .iter()
        .map(|session| session.login.subject.as_str())
        .collect::<Vec<_>>();
    subjects.sort_unstable();
    assert_eq!(subjects, [ADA_SECOND, ADA]);
    for session in &from_first.sessions {
        assert_eq!(session.login.issuer, service.issuer.issuer());
        assert!(session.ends_at > session.started_at);
        assert_eq!(session.id.len(), 32, "a public id is 16 bytes as hex");
    }
    assert!(
        from_first
            .sessions
            .windows(2)
            .all(|pair| pair[0].started_at >= pair[1].started_at),
        "newest first: {from_first:?}"
    );
    let current = id_of(&from_first, ADA, true)?;
    let other = id_of(&from_first, ADA_SECOND, false)?;
    let from_second = list(&service, "/sessions", &second, &cookies).await?;
    assert_eq!(id_of(&from_second, ADA, false)?, current);
    assert_eq!(id_of(&from_second, ADA_SECOND, true)?, other);
    let from_bea = list(&service, "/sessions", &bea, &cookies).await?;
    assert_eq!(from_bea.sessions.len(), 1, "{from_bea:?}");
    assert!(from_bea.sessions[0].current);
    Ok(())
}

#[tokio::test]
async fn an_ended_session_is_signed_out_and_ends_only_once() -> TestResult {
    let (service, _) = seeded().await?;
    let kept = service.sign_in(login(ADA)).await?;
    let ended = service.sign_in(login(ADA)).await?;
    let cookies = [kept.as_str(), ended.as_str()];
    let view = list(&service, "/sessions", &kept, &cookies).await?;
    let id = view
        .sessions
        .iter()
        .find(|session| !session.current)
        .ok_or("the second session is not listed")?
        .id
        .clone();
    let path = format!("/sessions/{id}/end");
    let answered: EndedView = typed(&path, end(&service, &path, &kept).await?, &cookies)?;
    assert_eq!(answered.ended, id);
    let (status, body) = service.get("/me", Some(&ended)).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    let (status, _) = service.get("/me", Some(&kept)).await?;
    assert_eq!(status, 200, "the other session is untouched");
    let (status, again) = end(&service, &path, &kept).await?;
    assert_eq!(status, 404, "{again}");
    assert_eq!(again["refusal"], "SessionUnknown");
    let (_, never) = end(&service, &format!("/sessions/{NEVER_HELD}/end"), &kept).await?;
    assert_eq!(again, never, "ended answers exactly as never held");
    let view = list(&service, "/sessions", &kept, &cookies).await?;
    assert_eq!(view.sessions.len(), 1, "{view:?}");
    Ok(())
}

#[tokio::test]
async fn another_persons_session_is_refused_as_one_never_held() -> TestResult {
    let (service, _) = seeded().await?;
    let ada = service.sign_in(login(ADA)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let cookies = [ada.as_str(), bea.as_str()];
    let bea_view = list(&service, "/sessions", &bea, &cookies).await?;
    let bea_id = id_of(&bea_view, BEA, true)?;
    let (status, refused) = end(&service, &format!("/sessions/{bea_id}/end"), &ada).await?;
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["refusal"], "SessionUnknown");
    let (_, never) = end(&service, &format!("/sessions/{NEVER_HELD}/end"), &ada).await?;
    assert_eq!(refused, never, "not yours answers exactly as never held");
    assert!(!refused.to_string().contains(&bea_id));
    assert_carries_no_secret(&refused, &cookies)?;
    let ada_view = list(&service, "/sessions", &ada, &cookies).await?;
    assert!(ada_view.sessions.iter().all(|session| session.id != bea_id));
    let (status, _) = service.get("/me", Some(&bea)).await?;
    assert_eq!(status, 200, "Bea is still signed in");
    Ok(())
}

#[tokio::test]
async fn ending_the_current_session_clears_the_cookie() -> TestResult {
    let (service, _) = seeded().await?;
    let cookie = service.sign_in(login(ADA)).await?;
    let view = list(&service, "/sessions", &cookie, &[&cookie]).await?;
    let id = id_of(&view, ADA, true)?;
    let answer = reqwest::Client::new()
        .post(format!("{}/sessions/{id}/end", service.base))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await?;
    assert_eq!(answer.status().as_u16(), 200);
    let cleared = answer
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .ok_or("ending the current session set no cookie")?
        .to_str()?
        .to_owned();
    let attributes = cleared.split("; ").collect::<Vec<_>>();
    assert_eq!(attributes[0], format!("{COOKIE}="), "{cleared}");
    for attribute in ["HttpOnly", "SameSite=Lax", "Path=/", "Max-Age=0"] {
        assert!(attributes.contains(&attribute), "{cleared}");
    }
    let body: Value = serde_json::from_str(&answer.text().await?)?;
    assert_carries_no_secret(&body, &[&cookie])?;
    let ended: EndedView = serde_json::from_value(body)?;
    assert_eq!(ended.ended, id);
    let (status, body) = service.get("/sessions", Some(&cookie)).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    Ok(())
}

#[tokio::test]
async fn the_administrator_lists_and_ends_any_persons_sessions() -> TestResult {
    let (service, seeded) = seeded().await?;
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let ada_cookie = service.sign_in(login(ADA)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let cookies = [
        administrator.as_str(),
        ada_cookie.as_str(),
        bea_cookie.as_str(),
    ];
    let ada = person(&seeded, ADA)?;
    let path = format!("/directory/people/{}/sessions", ada.id);
    let view = list(&service, &path, &administrator, &cookies).await?;
    assert_eq!(view.person, ada.id.to_string());
    assert_eq!(view.sessions.len(), 1, "{view:?}");
    assert!(!view.sessions[0].current, "not the administrator's session");
    let id = id_of(&view, ADA, false)?;
    let end_path = format!("{path}/{id}/end");
    let answered: EndedView = typed(
        &end_path,
        end(&service, &end_path, &administrator).await?,
        &cookies,
    )?;
    assert_eq!(answered.ended, id);
    let (status, _) = service.get("/me", Some(&ada_cookie)).await?;
    assert_eq!(status, 401, "Ada is signed out");
    let (status, _) = service.get("/me", Some(&bea_cookie)).await?;
    assert_eq!(status, 200, "Bea is untouched");
    let bea = person(&seeded, BEA)?;
    let bea_view = list(
        &service,
        &format!("/directory/people/{}/sessions", bea.id),
        &administrator,
        &cookies,
    )
    .await?;
    let bea_id = id_of(&bea_view, BEA, false)?;
    let (status, body) = end(&service, &format!("{path}/{bea_id}/end"), &administrator).await?;
    assert_eq!(status, 404, "Bea's session is not Ada's: {body}");
    assert_eq!(body["refusal"], "SessionUnknown");
    let unknown = PersonId::generate()?;
    let (status, body) = service
        .get(
            &format!("/directory/people/{unknown}/sessions"),
            Some(&administrator),
        )
        .await?;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["refusal"], "IdentityUnknown");
    let (status, body) = end(
        &service,
        &format!("/directory/people/{unknown}/sessions/{bea_id}/end"),
        &administrator,
    )
    .await?;
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["refusal"], "IdentityUnknown");
    Ok(())
}

#[tokio::test]
async fn every_other_caller_is_refused_by_name() -> TestResult {
    let (service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADA)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let stranger = service.sign_in(login("stranger")).await?;
    let cookies = [ada.as_str(), bea.as_str(), stranger.as_str()];
    let bea_person = person(&seeded, BEA)?;
    let bea_id = id_of(
        &list(&service, "/sessions", &bea, &cookies).await?,
        BEA,
        true,
    )?;
    let listing = format!("/directory/people/{}/sessions", bea_person.id);
    let ending = format!("{listing}/{bea_id}/end");
    for cookie in [&ada, &stranger] {
        let (status, body) = service.get(&listing, Some(cookie)).await?;
        assert_eq!(status, 403, "{body}");
        assert_eq!(body["refusal"], "NotAdmitted");
        assert_carries_no_secret(&body, &cookies)?;
        let (status, body) = end(&service, &ending, cookie).await?;
        assert_eq!(status, 403, "{body}");
        assert_eq!(body["refusal"], "NotAdmitted");
        assert!(!body.to_string().contains(&bea_id));
    }
    let (status, body) = service.get("/sessions", Some(&stranger)).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NoPerson");
    let (status, body) = end(&service, &format!("/sessions/{bea_id}/end"), &stranger).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NoPerson");
    for path in ["/sessions", listing.as_str()] {
        for cookie in [None, Some("lys_directory_session=00")] {
            let (status, body) = service.get(path, cookie).await?;
            assert_eq!(status, 401, "{path}: {body}");
            assert_eq!(body["refusal"], "NotSignedIn");
        }
    }
    let (status, _) = service.get("/me", Some(&bea)).await?;
    assert_eq!(status, 200, "no refusal ended Bea's session");
    Ok(())
}
