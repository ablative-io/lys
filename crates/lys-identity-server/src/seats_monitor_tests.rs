//! The seats monitor's own tests: what a listing says, and the refusal
//! and the notes `check_at` answers. A child of `seats_monitor` by path,
//! so the service's route scan never reads its fake monitor's route.

use crate::error_agents::AgentsError;
use axum::Router;
use axum::routing::get;
use serde_json::json;

use super::{check_at, online_in};
use crate::error::ServerError;
use crate::error_seat::SeatError;

#[test]
fn the_listing_says_online_offline_or_unknown() -> Result<(), String> {
    let body = json!({"seats": [
        {"name": "waffles", "online": true},
        {"name": "gaia", "online": false},
        {"name": "hermes", "online": null},
    ]});
    assert_eq!(online_in(&body, "waffles")?, Some(true));
    assert_eq!(online_in(&body, "gaia")?, Some(false));
    assert_eq!(online_in(&body, "hermes")?, None);
    assert_eq!(online_in(&body, "absent")?, None);
    assert!(online_in(&json!({}), "waffles").is_err());
    let malformed = json!({"seats": [{"name": "waffles", "online": "yes"}]});
    assert!(online_in(&malformed, "waffles").is_err());
    Ok(())
}

#[tokio::test]
async fn a_seat_the_monitor_lists_online_is_refused_and_an_absent_monitor_is_said()
-> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let routes = Router::new().route(
        "/api/seats",
        get(|| async { axum::Json(json!({"seats": [{"name": "waffles", "online": true}]})) }),
    );
    tokio::spawn(async move { axum::serve(listener, routes).await });
    let refused = check_at(&base, None, "waffles").await;
    assert!(
        matches!(
            refused,
            Err(ServerError::Agents(AgentsError::Seat(
                SeatError::OnlineInMonitor { .. }
            )))
        ),
        "{refused:?}"
    );
    let note = check_at(&base, None, "gaia").await?;
    assert!(note.contains("does not list it"), "{note}");
    let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let gone = format!("http://{}", closed.local_addr()?);
    drop(closed);
    let note = check_at(&gone, None, "waffles").await?;
    assert!(note.contains("could not be reached"), "{note}");
    Ok(())
}
