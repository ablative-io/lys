#![cfg(test)]
//! A provider callback URL is not a browser credential. A different browser
//! holding the URL must not acquire the initiating person's session.
use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::Service;
use reqwest::{Client, Response, header};

type TestResult = Result<(), Box<dyn Error>>;

fn location(answer: &Response) -> Result<String, Box<dyn Error>> {
    Ok(answer
        .headers()
        .get(header::LOCATION)
        .ok_or("redirect")?
        .to_str()?
        .to_owned())
}

async fn callback_url(service: &Service, browser: &Client) -> Result<String, Box<dyn Error>> {
    let start = browser
        .get(format!(
            "{}/sign-in/providers/google-provider",
            service.base
        ))
        .send()
        .await?;
    assert_eq!(start.status(), 303);
    let provider_url = location(&start)?;
    let provider = browser.get(provider_url).send().await?;
    assert_eq!(provider.status(), 303);
    location(&provider)
}

#[tokio::test]
async fn a_callback_without_the_initiating_browser_cookie_never_creates_a_session() -> TestResult {
    let service = Service::start().await?;
    service.issuer.sign_in_as(Login {
        subject: "victim".to_owned(),
        email: "victim@example.test".to_owned(),
    })?;
    let browser = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let callback = callback_url(&service, &browser).await?;
    let answer = browser.get(callback).send().await?;
    assert_eq!(location(&answer)?, "/#/sign-in?refused=SignInStateUnknown");
    assert!(
        answer.headers().get(header::SET_COOKIE).is_none(),
        "no authenticated session on an unbound callback"
    );
    Ok(())
}

#[tokio::test]
async fn a_callback_with_another_browsers_cookie_never_creates_a_session() -> TestResult {
    let service = Service::start().await?;
    service.issuer.sign_in_as(Login {
        subject: "victim".to_owned(),
        email: "victim@example.test".to_owned(),
    })?;
    let browser = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let callback = callback_url(&service, &browser).await?;
    let answer = browser
        .get(callback)
        .header(header::COOKIE, "lys_provider=another-browser")
        .send()
        .await?;
    assert_eq!(location(&answer)?, "/#/sign-in?refused=SignInStateUnknown");
    assert!(answer.headers().get(header::SET_COOKIE).is_none());
    Ok(())
}

#[tokio::test]
async fn the_bound_browser_can_finish_once_and_a_replay_cannot_sign_in() -> TestResult {
    let service = Service::start().await?;
    service.issuer.sign_in_as(Login {
        subject: "person".to_owned(),
        email: "person@example.test".to_owned(),
    })?;
    let browser = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let start = browser
        .get(format!(
            "{}/sign-in/providers/google-provider",
            service.base
        ))
        .send()
        .await?;
    let set = start
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("browser binding cookie")?
        .to_str()?;
    assert!(set.contains("; HttpOnly"));
    assert!(set.contains("; SameSite=Lax"));
    assert!(set.contains("; Path=/auth/v1/providers/callback"));
    assert!(!set.contains("Domain="));
    let cookie = set.split(';').next().ok_or("cookie pair")?;
    let provider_url = location(&start)?;
    let provider = browser.get(provider_url).send().await?;
    let callback = location(&provider)?;
    let answer = browser
        .get(&callback)
        .header(header::COOKIE, cookie)
        .send()
        .await?;
    assert_eq!(location(&answer)?, "/#/me");
    assert!(answer.headers().get(header::SET_COOKIE).is_some());
    let replay = browser
        .get(&callback)
        .header(header::COOKIE, cookie)
        .send()
        .await?;
    assert_eq!(location(&replay)?, "/#/sign-in?refused=SignInStateUnknown");
    assert!(replay.headers().get(header::SET_COOKIE).is_none());
    Ok(())
}

#[tokio::test]
async fn duplicate_browser_cookies_are_refused_even_with_one_correct_value() -> TestResult {
    let service = Service::start().await?;
    service.issuer.sign_in_as(Login {
        subject: "person".to_owned(),
        email: "person@example.test".to_owned(),
    })?;
    let browser = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let start = browser
        .get(format!(
            "{}/sign-in/providers/google-provider",
            service.base
        ))
        .send()
        .await?;
    let set = start
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("browser binding cookie")?
        .to_str()?;
    let cookie = set.split(';').next().ok_or("cookie pair")?;
    let provider_url = location(&start)?;
    let provider = browser.get(provider_url).send().await?;
    let callback = location(&provider)?;
    let answer = browser
        .get(callback)
        .header(header::COOKIE, format!("{cookie}; lys_provider=other"))
        .send()
        .await?;
    assert_eq!(location(&answer)?, "/#/sign-in?refused=SignInStateUnknown");
    assert!(answer.headers().get(header::SET_COOKIE).is_none());
    Ok(())
}
