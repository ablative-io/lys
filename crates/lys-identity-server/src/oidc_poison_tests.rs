use super::{ClientId, ClientSecret, Flights, Mutex, Oidc, RedirectUrl};
use serde_json::json;
use std::error::Error;

fn relying_party() -> Result<Oidc, Box<dyn Error>> {
    Ok(Oidc {
        metadata: serde_json::from_value(json!({
            "issuer": "https://issuer.example.test",
            "authorization_endpoint": "https://issuer.example.test/authorize",
            "token_endpoint": "https://issuer.example.test/token",
            "jwks_uri": "https://issuer.example.test/jwks",
            "response_types_supported": ["code"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["EdDSA"],
        }))?,
        client_id: ClientId::new("fixture".to_owned()),
        secret: ClientSecret::new("fixture".to_owned()),
        redirect: RedirectUrl::new("https://lys.example.test/callback".to_owned())?,
        public_origin: "https://lys.example.test".to_owned(),
        http: reqwest::Client::new(),
        in_flight: Mutex::new(Flights::default()),
        provider_flights: Mutex::new(Flights::default()),
    })
}

#[tokio::test]
async fn poisoned_password_flights_refuse_start_abandon_and_finish() -> Result<(), Box<dyn Error>> {
    let oidc = relying_party()?;
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let held = oidc
            .in_flight
            .lock()
            .expect("fixture lock poisoned before injection");
        std::hint::black_box(&*held);
        panic!("password flight state failure");
    }));
    assert!(poisoned.is_err());
    let start = oidc.begin("192.0.2.1".parse()?).map(drop);
    let abandon = oidc.abandon("state");
    let finish = oidc.finish("code".to_owned(), "state").await.map(drop);
    for result in [start, abandon, finish] {
        let refusal = result
            .err()
            .ok_or("poisoned password flights admitted sign-in")?;
        assert_eq!(refusal.name(), "SignInFailed");
        assert!(refusal.to_string().contains("sign-in flights unavailable"));
    }
    Ok(())
}

#[tokio::test]
async fn poisoned_provider_flights_refuse_start_abandon_and_finish() -> Result<(), Box<dyn Error>> {
    let oidc = relying_party()?;
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let held = oidc
            .provider_flights
            .lock()
            .expect("fixture lock poisoned before injection");
        std::hint::black_box(&*held);
        panic!("provider flight state failure");
    }));
    assert!(poisoned.is_err());
    let start = oidc.begin_provider("192.0.2.1".parse()?).map(drop);
    let abandon = oidc.abandon("state");
    let finish = oidc.finish("code".to_owned(), "state").await.map(drop);
    for result in [start, abandon, finish] {
        let refusal = result
            .err()
            .ok_or("poisoned provider flights admitted sign-in")?;
        assert_eq!(refusal.name(), "SignInFailed");
        assert!(refusal.to_string().contains("sign-in flights unavailable"));
    }
    Ok(())
}
