use std::error::Error;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, header};
use serde_json::json;

use crate::apps_state::{Approved, By, Client, Line, Registered};

#[tokio::test]
async fn the_model_route_reads_approved_kinds_even_when_the_published_model_has_none()
-> Result<(), Box<dyn Error>> {
    let (dir, state, actor, _, _, _) = crate::grant_settlement_tests::person_fixture().await?;
    let state = Arc::new(state);
    let cookie = state.sessions.begin(actor)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        cookie
            .split(';')
            .next()
            .ok_or("session cookie absent")?
            .parse()?,
    );
    let app = "fixture_model";
    let kind = format!("{app}.channel");
    {
        let mut apps = state.apps.lock().map_err(|error| error.to_string())?;
        apps.keep(Line::Registered(Registered {
            operation: "register-model".to_owned(),
            app: app.to_owned(),
            name: "Model fixture".to_owned(),
            redirects: vec![],
            schema: json!({"kinds": {&kind: {
                "actions": ["read", "send"], "agents": ["read", "send"],
                "relations": {"participant": ["read", "send"]}, "parents": []
            }}}),
            service_account: None,
            by: By::Start,
            at: 1,
        }))?;
        apps.keep(Line::Approved(Approved {
            operation: "approve-model".to_owned(),
            app: app.to_owned(),
            client: Client {
                client_id: app.to_owned(),
                secret_sha256: String::new(),
            },
            binding: None,
            by: By::Start,
            at: 2,
        }))?;
        assert!(apps.model()?.kinds().contains_key(&kind));
    }
    assert!(state.grant_setup.model()?.kinds().is_empty());

    let answered = super::model(State(Arc::clone(&state)), headers).await?.0;
    assert_eq!(
        serde_json::to_value(&answered.kinds)?,
        json!({&kind: {
            "agents": ["read", "send"],
            "relations": {"participant": ["read", "send"]}
        }})
    );
    assert!(
        state.grant_setup.model()?.kinds().is_empty(),
        "a read must not publish a permission model"
    );
    drop(state);
    dir.close()?;
    Ok(())
}
