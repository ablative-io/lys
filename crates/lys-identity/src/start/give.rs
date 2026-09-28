//! Give a start: check it, keep its launch record, and render the command
//! from that record. Never run it.
//!
//! In order, and each refusal by name: the request is parsed; the agent is
//! resolved to its enduring record; the caller is admitted; a start already
//! standing unconfirmed for the agent refuses the new one as
//! `start_unconfirmed`, on any machine; the five checks run; the command's
//! fields are read from the reviewed profile version; the assigned values
//! are held to their id grammars. Only then is one launch record kept, and
//! the command given is the one rendered from it, with the recorded working
//! directory beside it.
//!
//! A start given again from a kept record runs the admission and the checks
//! again and keeps a new record with its own id, copied from the kept one
//! and naming it, without reading any running process; a record is never
//! issued twice. A start of an agent that already reads running is given as
//! a new launch record naming the same enduring agent id: the session its
//! report makes is the sessions brief's to keep, and nothing about the first
//! session changes. No start writes a session record or creates or changes
//! an agent record.

use lys_log_store::LeafStore;

use crate::start::active::Lifecycles;
use crate::start::authority::{Admission, admit};
use crate::start::checks::{self, CheckReport, checks_json};
use crate::start::command::{self, Grammars};
use crate::start::credentials::HandleRecords;
use crate::start::egress::{EgressLists, ProfileNeeds};
use crate::start::error::{Refusal, Refused, StartError, json_string, json_strings};
use crate::start::launch_record::{LaunchRecord, new_launch_record_id};
use crate::start::machine_role::RoleMachines;
use crate::start::profile_command::{self, ProfileCommand, ProfileVersionRecords};
use crate::start::profile_review::ProfileReviews;
use crate::start::request::{AgentRecords, StartRequest, resolve};
use crate::start::state::{LaunchRecords, LaunchState, SessionReports, derive, standing};

/// Every owner a start reads, each through its own seam. None is written.
#[derive(Clone, Copy)]
pub struct Owners<'a> {
    /// The agent records DIRECTORY-011 keeps.
    pub agents: &'a dyn AgentRecords,
    /// The directory's admission.
    pub admission: &'a dyn Admission,
    /// The lifecycle record DIRECTORY-003 keeps.
    pub lifecycles: &'a dyn Lifecycles,
    /// The review record `Ink1H1Os` keeps.
    pub reviews: &'a dyn ProfileReviews,
    /// The role's machines in `Ink1H1Os`'s record.
    pub role_machines: &'a dyn RoleMachines,
    /// The handle record SECRETS-002 keeps.
    pub handles: &'a dyn HandleRecords,
    /// What each profile version needs, from its record.
    pub needs: &'a dyn ProfileNeeds,
    /// The egress lists of the network record of CONFORMANCE row 8.5.
    pub egress: &'a dyn EgressLists,
    /// The profile version record `Ink1H1Os` keeps, read for the command.
    pub profiles: &'a dyn ProfileVersionRecords,
    /// The sessions record the sessions brief d5055cc1 keeps.
    pub sessions: &'a dyn SessionReports,
    /// The id grammars of the assigned values.
    pub grammars: Grammars,
}

/// A start given: the launch record kept, and the command rendered from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Given {
    /// The launch record kept for it.
    pub record: LaunchRecord,
    /// The command line, rendered from the record.
    pub command: String,
    /// Every check, each passed.
    pub checks: Vec<CheckReport>,
    /// The record's state as it was given: unconfirmed.
    pub state: LaunchState,
}

impl Given {
    /// The working directory the command runs in.
    pub fn working_directory(&self) -> &str {
        &self.record.working_directory
    }

    /// The start as the JSON body the route answers.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"command\":");
        json_string(&mut out, &self.command);
        out.push_str(",\"working_directory\":");
        json_string(&mut out, self.working_directory());
        out.push_str(",\"launch_record\":");
        record_json(&mut out, &self.record);
        out.push_str(",\"state\":");
        json_string(&mut out, self.state.name());
        out.push_str(",\"words\":");
        json_string(&mut out, &self.state.words());
        out.push_str(",\"checks\":");
        checks_json(&mut out, &self.checks);
        out.push('}');
        out
    }
}

/// Append `record` to `out` as a JSON object.
pub fn record_json(out: &mut String, record: &LaunchRecord) {
    let texts = [
        ("id", &record.id),
        ("agent", &record.agent),
        ("machine", &record.machine),
        ("executable", &record.executable),
        ("working_directory", &record.working_directory),
        ("profile_version", &record.profile_version),
        ("given_by", &record.given_by),
    ];
    out.push('{');
    for (name, value) in texts {
        json_string(out, name);
        out.push(':');
        json_string(out, value);
        out.push(',');
    }
    out.push_str("\"arguments\":");
    json_strings(out, &record.arguments);
    out.push_str(",\"credential_ids\":");
    json_strings(out, &record.credential_ids);
    out.push_str(",\"given_at\":");
    out.push_str(&record.given_at.to_string());
    out.push_str(",\"copied_from\":");
    match &record.copied_from {
        Some(source) => json_string(out, source),
        None => out.push_str("null"),
    }
    out.push('}');
}

/// Give a start for the request `members` name, as `caller`, at `now`.
pub fn give<S, I>(
    store: &mut LaunchRecords<S>,
    owners: &Owners<'_>,
    caller: &str,
    members: I,
    now: u64,
) -> Result<Given, StartError>
where
    S: LeafStore,
    I: IntoIterator<Item = (String, String)>,
{
    let request = StartRequest::from_members(members)?;
    give_request(store, owners, (caller, now), &request, None)
}

/// Give a start again from the kept launch record `source`, as `caller`, at
/// `now`: a new record with its own id, naming `source` as the record it
/// was copied from.
pub fn give_again<S: LeafStore>(
    store: &mut LaunchRecords<S>,
    owners: &Owners<'_>,
    caller: &str,
    source: &str,
    now: u64,
) -> Result<Given, StartError> {
    store.settle()?;
    let kept = store
        .record(source)
        .cloned()
        .ok_or_else(|| StartError::LaunchRecordUnknown {
            launch_record: source.to_owned(),
        })?;
    let request = StartRequest::new(&kept.agent, &kept.profile_version, &kept.machine);
    give_request(store, owners, (caller, now), &request, Some(&kept))
}

fn give_request<S: LeafStore>(
    store: &mut LaunchRecords<S>,
    owners: &Owners<'_>,
    (caller, now): (&str, u64),
    request: &StartRequest,
    source: Option<&LaunchRecord>,
) -> Result<Given, StartError> {
    store.settle()?;
    let agent = resolve(owners.agents, request.agent())?;
    let admitted = admit(owners.admission, caller, &agent)?;
    if let Some(launch_record) = standing(store, &agent.id, owners.sessions) {
        return Err(Refusal::StartUnconfirmed { launch_record }.into());
    }
    let checked = checks::run(owners, &agent.id, request)?;
    let after_checks = |refusal: Refusal| {
        StartError::Refused(Refused {
            refusals: vec![refusal],
            checks: checked.reports.clone(),
        })
    };
    if !checked.passed() {
        return Err(StartError::Refused(Refused {
            refusals: checked.refusals(),
            checks: checked.reports.clone(),
        }));
    }
    let fields = match source {
        Some(kept) => ProfileCommand {
            executable: kept.executable.clone(),
            arguments: kept.arguments.clone(),
            working_directory: kept.working_directory.clone(),
        },
        None => profile_command::read(owners.profiles, request.profile_version())
            .map_err(after_checks)?,
    };
    let record = LaunchRecord {
        id: new_launch_record_id()?,
        agent: agent.id,
        machine: request.machine().to_owned(),
        executable: fields.executable,
        arguments: fields.arguments,
        working_directory: fields.working_directory,
        profile_version: request.profile_version().to_owned(),
        credential_ids: checked.credential_ids.clone(),
        given_by: admitted.caller,
        given_at: now,
        copied_from: source.map(|kept| kept.id.clone()),
    };
    command::check_grammar(&record, owners.grammars).map_err(after_checks)?;
    store.keep(&record)?;
    let state = derive(&record, None, owners.sessions);
    Ok(Given {
        command: command::render(&record),
        record,
        checks: checked.reports,
        state,
    })
}
