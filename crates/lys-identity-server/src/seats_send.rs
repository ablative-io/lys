//! A message to a seat and an attach to it (AGENTS-002 R4, R6).
//!
//! A message reaches the seat's running session as a user turn, through
//! the runner's input, never as keys: text that is empty, or carries a
//! control character other than a line break or a tab, is refused by name,
//! and a seat with no running session is refused `seat_not_running`. A
//! message is kept under its operation id once delivered, so the same
//! message sent again answers as delivered and is not delivered twice. A
//! line typed in `lys attach --type` is a message kept as `attach_type`.
//!
//! An attach reads the seat's live session as lines, for its responsible
//! person or an administrator and refused `seat_attach_refused` for anyone
//! else; the attach itself is kept as a receipt naming who attached, and
//! detaching ends nothing.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{OriginalUri, Path, State};
use axum::http::HeaderMap;
use lys_identity::OperationId;
use lys_runner::{Act, Answer};

use crate::error::ServerError;
use crate::error_seat::SeatError;
use crate::routes::AppState;
use crate::runner_acts::Digested;
use crate::runner_sessions::{driven, kind};
use crate::seats_acts::{SEND, act, admitted, standing};
use crate::seats_api::{body, seat, shown, with_seats};
use crate::seats_state::{Line, Sent};
use crate::seats_views::{SeatAttachBody, SeatAttached, SeatSendBody, SeatSent};
use crate::session::now;

pub(crate) async fn send(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path(name): Path<String>,
    given: Result<Json<SeatSendBody>, JsonRejection>,
) -> Result<Json<SeatSent>, ServerError> {
    let given = body(given)?;
    Box::pin(deliver(&state, &headers, (&uri.to_string(), &name), given, "send")).await
}

/// A line typed in `lys attach --type`: a message, kept as typed from an attach.
pub(crate) async fn typed(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path(name): Path<String>,
    given: Result<Json<SeatSendBody>, JsonRejection>,
) -> Result<Json<SeatSent>, ServerError> {
    let given = body(given)?;
    Box::pin(deliver(
        &state,
        &headers,
        (&uri.to_string(), &name),
        given,
        "attach_type",
    ))
    .await
}

pub(crate) async fn attach(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    given: Result<Json<SeatAttachBody>, JsonRejection>,
) -> Result<Json<SeatAttached>, ServerError> {
    let given = body(given)?;
    let held = seat(&state, &name)?;
    let standing = standing(&state, &headers, &held)?;
    if !(standing.administrator || standing.is_responsible) {
        return Err(SeatError::AttachRefused { name }.into());
    }
    let session = held
        .session
        .clone()
        .ok_or_else(|| SeatError::NotRunning { name: name.clone() })?;
    let driven = driven(&state, &session)?;
    let read = Act::AttachRead {
        session: session.clone(),
        cursor: given.cursor,
        follow: given.follow,
    };
    // The attach itself is kept as a receipt naming who attached; the reads
    // that follow it are not each an attach.
    let answer = match given.cursor {
        None => {
            let caller = standing.asker.to_string();
            act(&state, (&driven, &caller, "attach"), None, read).await?.0
        }
        Some(_) => {
            crate::runner_client::ask(&state, &driven.machine, driven.runner.clone(), read)
                .await?
        }
    };
    let (lines, cursor, ended) = match answer {
        Answer::AttachLines {
            lines,
            cursor,
            ended,
        } => (lines, cursor, ended),
        other => {
            return Err(ServerError::Runner {
                refusal: "runner_reply_malformed".to_owned(),
                words: format!("an attach read was answered {}", kind(&other)),
            });
        }
    };
    Ok(Json(SeatAttached {
        seat: shown(&state, &held).await?,
        session,
        lines,
        cursor,
        ended,
    }))
}

/// Refuse a message that is empty or carries a control character other
/// than a line break or a tab.
fn checked_text(name: &str, text: &str) -> Result<(), ServerError> {
    if text.trim().is_empty() {
        return Err(SeatError::TextEmpty {
            name: name.to_owned(),
        }
        .into());
    }
    if let Some(control) = text
        .chars()
        .find(|character| character.is_control() && !matches!(character, '\n' | '\t'))
    {
        return Err(SeatError::TextControl {
            name: name.to_owned(),
            code: u32::from(control),
        }
        .into());
    }
    Ok(())
}

/// Deliver `given` to the seat named `name` as a user turn, kept as `how`.
async fn deliver(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    (url, name): (&str, &str),
    given: SeatSendBody,
    how: &str,
) -> Result<Json<SeatSent>, ServerError> {
    OperationId::from_str(&given.operation)?;
    let held = seat(state, name)?;
    let (admission, _) = admitted(state, headers, &held, SEND, url)?;
    checked_text(name, &given.text)?;
    let digest = Digested::of(&given.text);
    let asked = |session: &str| {
        Line::Sent(Sent {
            operation: given.operation.clone(),
            name: name.to_owned(),
            session: session.to_owned(),
            text: digest.clone(),
            act: how.to_owned(),
            by: admission.caller.clone(),
            at: now(),
        })
    };
    let kept = with_seats(state, |store| Ok(store.recorded(&given.operation).cloned()))?;
    if let Some(kept) = kept {
        return match kept {
            Line::Sent(sent) if kept_same(&sent, &asked(&sent.session)) => Ok(Json(SeatSent {
                seat: shown(state, &held).await?,
                session: sent.session,
                delivered: true,
                admission,
            })),
            other => Err(SeatError::OperationReused {
                operation: other.operation().to_owned(),
            }
            .into()),
        };
    }
    let running = if held.running {
        held.session.clone()
    } else {
        None
    };
    let session = running.ok_or_else(|| SeatError::NotRunning {
        name: name.to_owned(),
    })?;
    let driven = driven(state, &session)?;
    let input = Act::Input {
        session: session.clone(),
        text: given.text.clone(),
        enter: true,
    };
    match act(state, (&driven, &admission.caller, how), Some(digest.clone()), input).await {
        Ok(_) => {}
        Err(ServerError::Runner { refusal, .. })
            if matches!(refusal.as_str(), "session_ended" | "session_unknown") =>
        {
            return Err(SeatError::NotRunning {
                name: name.to_owned(),
            }
            .into());
        }
        Err(other) => return Err(other),
    }
    let line = asked(&session);
    with_seats(state, |store| store.keep(line))?;
    Ok(Json(SeatSent {
        seat: shown(state, &held).await?,
        session,
        delivered: true,
        admission,
    }))
}

fn kept_same(kept: &Sent, asked: &Line) -> bool {
    Line::Sent(kept.clone()).same_words(asked)
}
#[cfg(test)]
mod tests {
    use super::checked_text;
    use crate::error::ServerError;
    use crate::error_seat::SeatError;

    #[test]
    fn a_message_is_refused_empty_or_with_a_control_character() {
        assert!(checked_text("waffles", "hello\nthere\tyou").is_ok());
        assert!(matches!(
            checked_text("waffles", "  \n"),
            Err(ServerError::Seat(SeatError::TextEmpty { .. }))
        ));
        assert!(matches!(
            checked_text("waffles", "hello\u{1b}[2J"),
            Err(ServerError::Seat(SeatError::TextControl { code: 0x1b, .. }))
        ));
    }
}
