//! Approval belongs to the person and client, across that person's browser sessions.

use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};

use super::{Apps, Asking, held, malformed};
use crate::error::ServerError;
use crate::session::{SessionEntry, now};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Agent,
    PersonReads,
}

pub(super) struct Answer {
    pub(super) asking: String,
    pub(super) decision: String,
    pub(super) grants: Vec<String>,
    pub(super) mode: Mode,
}

pub(super) fn answer(body: &[u8]) -> Result<Answer, ServerError> {
    let text = std::str::from_utf8(body).map_err(|_unread| malformed("an approval is a form"))?;
    let url = reqwest::Url::parse(&format!("http://form.invalid/?{text}"))
        .map_err(|_unread| malformed("an approval is a form"))?;
    let (mut asking, mut decision, mut mode, mut grants) = (None, None, None, Vec::new());
    for (name, value) in url.query_pairs() {
        match name.as_ref() {
            "asking" if asking.is_none() => asking = Some(value.into_owned()),
            "decision" if decision.is_none() => decision = Some(value.into_owned()),
            "mode" if mode.is_none() => {
                mode = Some(match value.as_ref() {
                    "agent" => Mode::Agent,
                    "person_reads" => Mode::PersonReads,
                    _ => {
                        return Err(malformed(
                            "choose agent or person_reads for this connection",
                        ));
                    }
                });
            }
            "grant" => grants.push(value.into_owned()),
            "asking" | "decision" | "mode" => {
                return Err(malformed("an approval carries each choice once"));
            }
            _ => return Err(malformed("an approval carries only its own fields")),
        }
    }
    Ok(Answer {
        asking: asking.ok_or_else(|| malformed("an approval names what it answers"))?,
        decision: decision.ok_or_else(|| malformed("an approval carries its decision"))?,
        grants,
        mode: mode.unwrap_or(Mode::Agent),
    })
}

pub(super) enum Approval {
    Ready(Asking),
    Refused(Response),
}

pub(super) fn take(
    apps: &Apps,
    key: &str,
    session: &SessionEntry,
) -> Result<Approval, ServerError> {
    let person = crate::routes::with_directory(&apps.state, |directory| {
        crate::read_api::own_person(directory.projection()?, &session.actor)
    })?;
    let mut waiting = held(&apps.asking)?;
    let cause = match waiting.get(key) {
        None => Some(("asking_missing", String::new())),
        Some(asking) if asking.expires_at <= now() => {
            Some(("asking_expired", asking.person.to_string()))
        }
        Some(asking) if asking.person != person => {
            Some(("person_mismatch", asking.person.to_string()))
        }
        Some(_) => None,
    };
    if let Some((cause, prior)) = cause {
        if cause == "asking_expired" {
            waiting.remove(key);
        }
        drop(waiting);
        tracing::warn!(refusal = "CodeUnknown", cause, person = %person, asking_person = prior, "CLI or app approval refused");
        let words = match cause {
            "person_mismatch" => {
                "This approval belongs to another person. Sign in as the person who opened it, or start sign-in again from your terminal."
            }
            "asking_expired" => {
                "This approval expired. Start sign-in again from your terminal, then approve its new page."
            }
            _ => {
                "This approval is no longer held by Lys. The service may have restarted or the request was already answered. Start sign-in again from your terminal, then approve its new page."
            }
        };
        return Ok(Approval::Refused((StatusCode::BAD_REQUEST,
            [(header::CACHE_CONTROL, "no-store"), (header::CONTENT_SECURITY_POLICY, "default-src 'none'")],
            Html(format!("<!doctype html><html lang=\"en\"><title>Sign-in needs a new approval</title><h1>Sign-in needs a new approval</h1><p>{words}</p><p>Run <code>lys sign-in</code> again.</p><details><summary>Error details</summary><p>CodeUnknown: {cause}</p></details></html>"))
        ).into_response()));
    }
    waiting
        .remove(key)
        .map(Approval::Ready)
        .ok_or_else(|| ServerError::ConfigInvalid {
            reason: "the admitted approval disappeared while its lock was held".to_owned(),
        })
}

#[cfg(test)]
mod tests {
    use super::{Mode, answer};

    #[test]
    fn consent_defaults_to_agent_and_person_reads_needs_an_explicit_choice()
    -> Result<(), crate::error::ServerError> {
        assert!(matches!(
            answer(b"asking=held&decision=approve")?.mode,
            Mode::Agent
        ));
        assert!(matches!(
            answer(b"asking=held&decision=approve&mode=person_reads")?.mode,
            Mode::PersonReads
        ));
        Ok(())
    }

    #[test]
    fn consent_refuses_unknown_and_ambiguous_mode_choices() {
        for form in [
            "asking=held&decision=approve&mode=administrator",
            "asking=held&decision=approve&mode=agent&mode=person_reads",
            "asking=held&decision=approve&decision=refuse",
            "asking=held&asking=another&decision=approve",
        ] {
            assert!(
                answer(form.as_bytes()).is_err(),
                "ambiguous consent was admitted"
            );
        }
    }
}
