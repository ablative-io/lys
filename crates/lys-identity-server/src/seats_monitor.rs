//! Before a seat starts from Lys, the monitor's registry is asked whether it
//! still lists the seat's name online (AGENTS-002 R5), so no seat is watched
//! twice while it moves.
//!
//! The monitor is reached at the base `LYS_MONITOR_URL` names, by its seats
//! listing `GET {base}/api/seats`, which answers
//! `{"seats": [{"name", "online"}, ..]}` with `online` true, false, or null
//! while the monitor cannot read its terminal program. The collector's
//! secret, when `LYS_MONITOR_SECRET` names one, rides in the header
//! `LYS_MONITOR_SECRET_HEADER` names; a secret named with no header name, or
//! a header name that is not one, refuses the start `ConfigInvalid`, and no
//! header name is ever assumed. With no base named the check is skipped, and
//! the answer says so. A name the monitor lists online refuses the start
//! `seat_online_in_monitor`. The monitor unreachable, refusing, answering
//! what does not read, or not knowing whether the seat is online, does not
//! block the start: each is said in the answer's notes, in words.

use std::env::VarError;
use std::time::Duration;

use reqwest::header::HeaderName;
use serde_json::Value;

use crate::error::ServerError;
use crate::error_seat::SeatError;

/// The variable naming the monitor's base address.
pub const MONITOR_URL: &str = "LYS_MONITOR_URL";
/// The variable naming the monitor collector's secret.
pub const MONITOR_SECRET: &str = "LYS_MONITOR_SECRET";
/// The variable naming the header the collector's secret rides in.
pub const MONITOR_SECRET_HEADER: &str = "LYS_MONITOR_SECRET_HEADER";

/// How long the monitor is given to answer before it is said unreachable.
const ASKED_FOR: Duration = Duration::from_secs(5);

/// Ask the monitor, at the base the environment names, whether `name` is
/// online; answer the note the start carries, or refuse when it is online
/// or when its secret is named without the header it rides in.
pub(crate) async fn check(name: &str) -> Result<String, ServerError> {
    let base = match std::env::var(MONITOR_URL) {
        Ok(base) => base,
        Err(VarError::NotPresent) => {
            return Ok(format!(
                "the monitor was not asked whether seat {name} is online: {MONITOR_URL} names no monitor"
            ));
        }
        Err(VarError::NotUnicode(given)) => {
            return Ok(format!(
                "the monitor was not asked whether seat {name} is online: {MONITOR_URL} is not text ({})",
                given.to_string_lossy()
            ));
        }
    };
    let secret = match std::env::var(MONITOR_SECRET) {
        Ok(secret) => Some(secret),
        Err(VarError::NotPresent) => None,
        Err(VarError::NotUnicode(_)) => {
            return Ok(format!(
                "the monitor was not asked whether seat {name} is online: {MONITOR_SECRET} is not text"
            ));
        }
    };
    let Some(secret) = secret else {
        return check_at(&base, None, name).await;
    };
    let header = secret_header()?;
    check_at(&base, Some((&header, secret.as_str())), name).await
}

/// The header the environment names for the collector's secret, refused
/// `ConfigInvalid` when it names none or names no header.
fn secret_header() -> Result<HeaderName, ServerError> {
    let given = match std::env::var(MONITOR_SECRET_HEADER) {
        Ok(given) => given,
        Err(VarError::NotPresent) => {
            return Err(ServerError::ConfigInvalid {
                reason: format!(
                    "{MONITOR_SECRET} names a secret but {MONITOR_SECRET_HEADER} names no header for it to ride in"
                ),
            });
        }
        Err(VarError::NotUnicode(_)) => {
            return Err(ServerError::ConfigInvalid {
                reason: format!("{MONITOR_SECRET_HEADER} is not text"),
            });
        }
    };
    HeaderName::from_bytes(given.as_bytes()).map_err(|error| ServerError::ConfigInvalid {
        reason: format!("{MONITOR_SECRET_HEADER} `{given}` is not a header name: {error}"),
    })
}

/// Ask the monitor at `base` whether `name` is online, with the collector's
/// secret in its header when one is given.
pub(crate) async fn check_at(
    base: &str,
    secret: Option<(&HeaderName, &str)>,
    name: &str,
) -> Result<String, ServerError> {
    let address = format!("{}/api/seats", base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(ASKED_FOR)
        .build()
        .map_err(|error| ServerError::ConfigInvalid {
            reason: format!("the client that asks the monitor could not be built: {error}"),
        })?;
    let mut request = client.get(&address);
    if let Some((header, secret)) = secret {
        request = request.header(header, secret);
    }
    let answered = match request.send().await {
        Ok(answered) => answered,
        Err(error) => {
            return Ok(format!(
                "the monitor at {base} could not be reached, so it was not known whether seat {name} is online there: {error}"
            ));
        }
    };
    let status = answered.status();
    if !status.is_success() {
        return Ok(format!(
            "the monitor at {base} answered {status} to its seats listing, so it was not known whether seat {name} is online there"
        ));
    }
    let body = match answered.json::<Value>().await {
        Ok(body) => body,
        Err(error) => {
            return Ok(format!(
                "the monitor at {base} answered a seats listing that does not read: {error}"
            ));
        }
    };
    match online_in(&body, name) {
        Ok(Some(true)) => Err(SeatError::OnlineInMonitor {
            name: name.to_owned(),
            monitor: base.to_owned(),
        }
        .into()),
        Ok(Some(false)) => Ok(format!("the monitor at {base} lists seat {name} offline")),
        Ok(None) => Ok(format!(
            "the monitor at {base} does not know whether seat {name} is online, or does not list it"
        )),
        Err(reason) => Ok(format!(
            "the monitor at {base} answered a seats listing that does not read: {reason}"
        )),
    }
}

/// Whether the monitor's seats listing `body` names `name` online: none when
/// it does not list the name or does not know.
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
#[path = "seats_monitor_tests.rs"]
mod tests;
