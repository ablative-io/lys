//! Before a seat starts from Lys, Argus's registry is asked whether it still
//! lists the seat's name online (AGENTS-002 R5), so no seat is watched
//! twice while it moves.
//!
//! Argus is reached at the base `LYS_ARGUS_URL` names, by its seats listing
//! `GET {base}/api/seats`, which answers `{"seats": [{"name", "online"}, ..]}`
//! with `online` true, false, or null while Argus cannot read its terminal
//! program; the collector's secret, when `LYS_ARGUS_SECRET` names one, rides
//! in its `x-argus-secret` header. With no base named the check is skipped,
//! and the answer says so. A name Argus lists online refuses the start
//! `seat_online_in_argus`. Argus unreachable, refusing, answering what does
//! not read, or not knowing whether the seat is online, does not block the
//! start: each is said in the answer's notes, in words.

use std::env::VarError;
use std::time::Duration;

use serde_json::Value;

use crate::error::ServerError;
use crate::error_seat::SeatError;

/// The variable naming Argus's base address.
pub const ARGUS_URL: &str = "LYS_ARGUS_URL";
/// The variable naming the Argus collector's secret.
pub const ARGUS_SECRET: &str = "LYS_ARGUS_SECRET";

/// How long Argus is given to answer before it is said unreachable.
const ASKED_FOR: Duration = Duration::from_secs(5);

/// Ask Argus, at the base the environment names, whether `name` is online;
/// answer the note the start carries, or refuse when it is online.
pub(crate) async fn check(name: &str) -> Result<String, ServerError> {
    let base = match std::env::var(ARGUS_URL) {
        Ok(base) => base,
        Err(VarError::NotPresent) => {
            return Ok(format!(
                "Argus was not asked whether seat {name} is online: {ARGUS_URL} names no Argus"
            ));
        }
        Err(VarError::NotUnicode(given)) => {
            return Ok(format!(
                "Argus was not asked whether seat {name} is online: {ARGUS_URL} is not text ({})",
                given.to_string_lossy()
            ));
        }
    };
    let secret = match std::env::var(ARGUS_SECRET) {
        Ok(secret) => Some(secret),
        Err(VarError::NotPresent) => None,
        Err(VarError::NotUnicode(_)) => {
            return Ok(format!(
                "Argus was not asked whether seat {name} is online: {ARGUS_SECRET} is not text"
            ));
        }
    };
    check_at(&base, secret.as_deref(), name).await
}

/// Ask the Argus at `base` whether `name` is online.
pub(crate) async fn check_at(
    base: &str,
    secret: Option<&str>,
    name: &str,
) -> Result<String, ServerError> {
    let address = format!("{}/api/seats", base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(ASKED_FOR)
        .build()
        .map_err(|error| ServerError::ConfigInvalid {
            reason: format!("the client that asks Argus could not be built: {error}"),
        })?;
    let mut request = client.get(&address);
    if let Some(secret) = secret {
        request = request.header("x-argus-secret", secret);
    }
    let answered = match request.send().await {
        Ok(answered) => answered,
        Err(error) => {
            return Ok(format!(
                "Argus at {base} could not be reached, so it was not known whether seat {name} is online there: {error}"
            ));
        }
    };
    let status = answered.status();
    if !status.is_success() {
        return Ok(format!(
            "Argus at {base} answered {status} to its seats listing, so it was not known whether seat {name} is online there"
        ));
    }
    let body = match answered.json::<Value>().await {
        Ok(body) => body,
        Err(error) => {
            return Ok(format!(
                "Argus at {base} answered a seats listing that does not read: {error}"
            ));
        }
    };
    match online_in(&body, name) {
        Ok(Some(true)) => Err(SeatError::OnlineInArgus {
            name: name.to_owned(),
            argus: base.to_owned(),
        }
        .into()),
        Ok(Some(false)) => Ok(format!("Argus at {base} lists seat {name} offline")),
        Ok(None) => Ok(format!(
            "Argus at {base} does not know whether seat {name} is online, or does not list it"
        )),
        Err(reason) => Ok(format!(
            "Argus at {base} answered a seats listing that does not read: {reason}"
        )),
    }
}

/// Whether Argus's seats listing `body` names `name` online: none when it
/// does not list the name or does not know.
pub(crate) fn online_in(body: &Value, name: &str) -> Result<Option<bool>, String> {
    let seats = body
        .get("seats")
        .and_then(Value::as_array)
        .ok_or("it names no seats list")?;
    for seat in seats {
        if seat.get("name").and_then(Value::as_str) == Some(name) {
            return match seat.get("online") {
                Some(Value::Bool(online)) => Ok(Some(*online)),
                Some(Value::Null) | None => Ok(None),
                Some(other) => Err(format!("seat {name}'s online is {other}, not a boolean")),
            };
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
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
    async fn a_seat_argus_lists_online_is_refused_and_an_absent_argus_is_said()
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
                Err(ServerError::Seat(SeatError::OnlineInArgus { .. }))
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
}
