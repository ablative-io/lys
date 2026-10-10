//! What the seat routes take and answer (AGENTS-002), and a seat's state
//! from its runner's own knowledge (R2).
//!
//! A seat with no session is `not-seen`; one whose session the runner holds
//! alive is `online-working` while the harness reports a turn and
//! `online-idle` between turns; one whose session the runner holds ended,
//! or does not hold, is `offline`. When its runner cannot be read the state
//! is `unknown`, and the list names the runner `unknown` with the reason,
//! never zero.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::seats_acts::{Live, SeatAdmission};
use crate::seats_state::Seat;

/// A seat to add.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SeatAddBody {
    /// The operation id it is added under.
    pub operation: String,
    /// Its name: 1 to 64 lowercase letters, digits and hyphens.
    pub name: String,
    /// The agent identity it runs as.
    pub agent: String,
    /// The reviewed profile version it starts from.
    pub profile_version: u32,
    /// The machine whose runner starts it.
    pub machine: String,
    /// The folder the harness starts in; absent, the profile's own default.
    #[serde(default)]
    pub working_folder: Option<String>,
    /// The account handle it runs under.
    #[serde(default)]
    pub account: Option<String>,
}

/// An act on a seat that names only its operation.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SeatOperationBody {
    /// The operation id the act is asked under.
    pub operation: String,
}

/// A stop or a restart.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SeatStopBody {
    /// The operation id the act is asked under.
    pub operation: String,
    /// End the session even while the harness reports a turn in progress.
    #[serde(default)]
    pub force: bool,
}

/// A message to a seat, delivered as a user turn.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SeatSendBody {
    /// The operation id it is sent under.
    pub operation: String,
    /// The text: not empty, and no control character but line breaks and tabs.
    pub text: String,
}

/// An attach, or a following read of one.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SeatAttachBody {
    /// Where the last read ended; absent, this call is the attach itself.
    #[serde(default)]
    pub cursor: Option<u64>,
    /// Wait for the session's next lines rather than answering at once.
    #[serde(default)]
    pub follow: bool,
}

/// A seat as the screens and the CLI show it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SeatView {
    /// Its name.
    pub name: String,
    /// The agent identity it runs as.
    pub agent: String,
    /// The harness its profile version declares.
    pub harness: String,
    /// The reviewed profile version it starts from.
    pub profile_version: u32,
    /// The machine whose runner starts it.
    pub machine: String,
    /// The folder the harness starts in; null for the profile's own default.
    pub working_folder: Option<String>,
    /// The account handle it runs under.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// The person responsible for its agent; null when nobody is.
    pub responsible: Option<String>,
    /// Its latest Lys session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The harness's own id for that session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_session: Option<String>,
    /// `online-working`, `online-idle`, `offline`, `not-seen` or `unknown`.
    pub state: String,
    /// The last hook or frame the runner saw, in milliseconds since the epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_signal_at: Option<u64>,
    /// Why the state is `unknown`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Who added it.
    pub created_by: String,
    /// When, in seconds since the epoch.
    pub created_at: u64,
    /// Raised by every change to it.
    pub revision: u64,
}

/// Every seat, and whether their runners could be read.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SeatList {
    /// The seats, in the order added.
    pub seats: Vec<SeatView>,
    /// `read` when every seat's runner answered, `unknown` otherwise.
    pub runner: String,
    /// Which runners could not be read, and why.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A seat started.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SeatStarted {
    /// The seat as it now stands.
    pub seat: SeatView,
    /// The Lys session.
    pub session: String,
    /// The harness's own session id, when the runner knew it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_session: Option<String>,
    /// The start the launch path kept and the runner's word on it.
    #[schema(value_type = Object)]
    pub start: Value,
    /// What admitted it.
    pub admission: SeatAdmission,
    /// What was not checked or not known, in words.
    pub notes: Vec<String>,
}

/// A seat stopped.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SeatStopped {
    /// The seat as it now stands.
    pub seat: SeatView,
    /// The session stopped.
    pub session: String,
    /// Whether the session's end was seen.
    pub ended: bool,
    /// What admitted it.
    pub admission: SeatAdmission,
    /// How it was ended, in words.
    pub notes: Vec<String>,
}

/// A seat restarted under a new session.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SeatRestarted {
    /// The seat as it now stands.
    pub seat: SeatView,
    /// The new session.
    pub session: String,
    /// The session stopped, when one was running.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped: Option<String>,
    /// What admitted it.
    pub admission: SeatAdmission,
    /// What was not checked or not known, and how the old session ended.
    pub notes: Vec<String>,
}

/// A message delivered to a seat.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SeatSent {
    /// The seat.
    pub seat: SeatView,
    /// The session it reached.
    pub session: String,
    /// Always true: a message not delivered is refused by name.
    pub delivered: bool,
    /// What admitted it.
    pub admission: SeatAdmission,
}

/// What an attach read of a seat's session showed.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SeatAttached {
    /// The seat.
    pub seat: SeatView,
    /// The session read.
    pub session: String,
    /// The lines since the cursor, one for each frame.
    pub lines: Vec<lys_runner::attach::AttachLine>,
    /// Where this read ended; send it back to read on.
    pub cursor: u64,
    /// Whether the session has ended.
    pub ended: bool,
}

/// A session a runner holds for no seat.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UnregisteredSession {
    /// The session.
    pub session: String,
    /// The machine whose runner holds it.
    pub machine: String,
    /// The agent the runtime reports keep it under, when they do.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Its process, when it has one.
    pub pid: Option<u32>,
    /// When it started, in milliseconds since the epoch.
    pub started_at: u64,
}

/// A runner that could not be read for the unregistered sessions.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SeatRunnerUnread {
    /// The machine.
    pub machine: String,
    /// The refusal's name.
    pub refusal: String,
    /// Why, in words.
    pub reason: String,
}

/// The sessions runners hold for no seat.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SeatUnregistered {
    /// The sessions, machine by machine.
    pub sessions: Vec<UnregisteredSession>,
    /// The runners that could not be read, each named; their sessions are unknown.
    pub unanswered: Vec<SeatRunnerUnread>,
}

/// How a seat's runner was read for its state.
pub(crate) enum Reading<'a> {
    /// The runner answered; the seat's session is this one, or none it holds.
    Read(Option<&'a Live>),
    /// The runner could not be read, for this reason.
    Unread(&'a str),
}

/// `seat` as shown, its state taken from `reading`.
pub(crate) fn view(seat: &Seat, responsible: Option<String>, reading: &Reading<'_>) -> SeatView {
    let (state, last_signal_at, harness_session, reason) = match (&seat.session, reading) {
        (None, _) => ("not-seen", None, None, None),
        (Some(_), Reading::Unread(reason)) => ("unknown", None, None, Some((*reason).to_owned())),
        (Some(_), Reading::Read(Some(live))) if live.alive && !live.ended => (
            if live.turn_active {
                "online-working"
            } else {
                "online-idle"
            },
            live.last_signal_at,
            live.harness_session.clone(),
            None,
        ),
        (Some(_), Reading::Read(live)) => (
            "offline",
            live.and_then(|live| live.last_signal_at),
            live.and_then(|live| live.harness_session.clone()),
            None,
        ),
    };
    let added = &seat.added;
    SeatView {
        name: added.name.clone(),
        agent: added.agent.clone(),
        harness: added.harness.clone(),
        profile_version: added.profile_version,
        machine: added.machine.clone(),
        working_folder: added.working_folder.clone(),
        account: added.account.clone(),
        responsible,
        session: seat.session.clone(),
        harness_session: seat.harness_session.clone().or(harness_session),
        state: state.to_owned(),
        last_signal_at,
        reason,
        created_by: added.by.clone(),
        created_at: added.at,
        revision: seat.revision,
    }
}

#[cfg(test)]
mod tests {
    use super::{Reading, view};
    use crate::seats_acts::Live;
    use crate::seats_state::{Added, Seat};

    fn seat(session: Option<&str>) -> Seat {
        Seat {
            added: Added {
                operation: "op-a".to_owned(),
                name: "waffles".to_owned(),
                agent: "agent".to_owned(),
                harness: "claude".to_owned(),
                profile_version: 1,
                machine: "op-1".to_owned(),
                working_folder: None,
                account: None,
                by: "person".to_owned(),
                at: 1,
            },
            session: session.map(str::to_owned),
            harness_session: None,
            running: session.is_some(),
            revision: 1,
        }
    }

    fn live(alive: bool, turn_active: bool) -> Live {
        Live {
            session: "op-b".to_owned(),
            pid: Some(7),
            alive,
            managed: true,
            harness_session: Some("uuid".to_owned()),
            turn_active,
            last_signal_at: Some(9),
            started_at: 1,
            ended: !alive,
        }
    }

    #[test]
    fn a_seat_is_shown_in_the_state_its_runner_knows() {
        let state = |seat: &Seat, reading: &Reading<'_>| view(seat, None, reading).state;
        let (working, idle, dead) = (live(true, true), live(true, false), live(false, false));
        assert_eq!(state(&seat(None), &Reading::Read(None)), "not-seen");
        assert_eq!(state(&seat(Some("op-b")), &Reading::Read(Some(&working))), "online-working");
        assert_eq!(state(&seat(Some("op-b")), &Reading::Read(Some(&idle))), "online-idle");
        assert_eq!(state(&seat(Some("op-b")), &Reading::Read(Some(&dead))), "offline");
        assert_eq!(state(&seat(Some("op-b")), &Reading::Read(None)), "offline");
        let unread = view(&seat(Some("op-b")), None, &Reading::Unread("runner_unreachable"));
        assert_eq!(unread.state, "unknown");
        assert_eq!(unread.reason.as_deref(), Some("runner_unreachable"));
        let shown = view(&seat(Some("op-b")), None, &Reading::Read(Some(&idle)));
        assert_eq!(shown.harness_session.as_deref(), Some("uuid"));
        assert_eq!(shown.last_signal_at, Some(9));
    }
}
