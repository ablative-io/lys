//! A change wait authenticates before waiting and again before answering.
use identity_contract::apps::{Auth, get, login, post};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::json;
use std::error::Error;
use std::future::Future;
use std::task::{Context, Waker};
#[path = "support/agent_policy.rs"]
mod support;

#[tokio::test]
async fn ending_a_session_wakes_its_wait_and_refuses_the_ended_cookie() -> Result<(), Box<dyn Error>>
{
    let (service, _) = support::table(false).await?;
    let (status, refused) = get(&service, "/changes", Auth::Nobody).await?;
    assert_eq!(status, 401, "{refused}");
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, initial) = get(&service, "/changes", Auth::Cookie(&cookie)).await?;
    assert_eq!(status, 200, "{initial}");
    let generation = initial["generation"].as_str().ok_or("no generation")?;
    let path = format!("/changes?after={generation}");
    let (status, sessions) = get(&service, "/sessions", Auth::Cookie(&cookie)).await?;
    assert_eq!(status, 200, "{sessions}");
    let id = sessions["sessions"][0]["id"].as_str().ok_or("no session")?;
    let mut waiting = Box::pin(get(&service, &path, Auth::Cookie(&cookie)));
    assert!(
        waiting
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let (status, ended) = post(
        &service,
        &format!("/sessions/{id}/end"),
        Auth::Cookie(&cookie),
        &json!({}),
    )
    .await?;
    assert_eq!(status, 200, "{ended}");
    let (status, refused) = waiting.await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "NotSignedIn");
    Ok(())
}
