//! Every personal exception names a real route, including the provider's external callback.

use std::error::Error;

use axum::http::Method;
use identity_contract::harness::Service;

use super::OWN_ACCOUNT_ROUTES;

#[tokio::test]
async fn every_personal_exception_names_a_registered_method_and_template()
-> Result<(), Box<dyn Error>> {
    assert_eq!(
        OWN_ACCOUNT_ROUTES.len(),
        14,
        "review the complete exception set"
    );
    let service = Service::start().await?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    for (method, template) in OWN_ACCOUNT_ROUTES {
        let path = format!("/{template}");
        if path == crate::sign_in::PROVIDER_CALLBACK_PATH {
            // This browser callback is outside the API table. Exercise its actual
            // registration, without following a redirect to a different handler.
            assert_eq!(*method, Method::GET);
            let response = client.get(format!("{}{path}", service.base)).send().await?;
            assert_ne!(response.status().as_u16(), 404, "missing callback {path}");
        } else {
            assert!(
                crate::openapi_table::TABLE.iter().any(|entry| {
                    entry.1 == path && method.as_str().eq_ignore_ascii_case(entry.0.word())
                }),
                "stale exception {method} {path}"
            );
        }
    }
    // Prove the callback probe would detect an absent route, rather than a
    // wildcard or automatic redirect making every request appear registered.
    let missing = client
        .get(format!("{}/no-such-personal-exception", service.base))
        .send()
        .await?;
    assert_eq!(missing.status().as_u16(), 404);
    Ok(())
}
