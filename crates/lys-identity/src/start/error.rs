//! Every refusal of a start, each one typed variant with a stable name, and
//! the few failures that are not refusals.
//!
//! A refusal's words say what refused, and name the check that failed where
//! a check did. No variant holds a credential value: the fields are ids,
//! member and field names, states and the cards that make a missing record,
//! so neither the words nor the `Debug` form can carry one.
//!
//! The JSON a refusal is answered in is written here, by hand, so the route
//! and the command line answer the library's bytes and hold no start logic.

use std::fmt;

use crate::start::checks::Check;

/// Why a start, a start given again or a withdrawal was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The request carried a member beside the agent, the profile version
    /// and the machine.
    StartFieldNotAllowed {
        /// The member that is not allowed.
        member: String,
    },
    /// The request lacked one of its three members.
    StartMemberMissing {
        /// The missing member.
        member: &'static str,
    },
    /// No agent record exists for the agent named.
    AgentUnknown {
        /// The agent named.
        agent: String,
    },
    /// The caller is neither the agent's responsible person nor a directory
    /// administrator.
    StartRightMissing {
        /// The agent they asked to start.
        agent: String,
    },
    /// The agent's lifecycle state is not active.
    AgentNotActive {
        /// The state the lifecycle record holds.
        state: String,
    },
    /// The profile version has no review on record.
    ProfileVersionNotReviewed {
        /// The profile version.
        profile_version: String,
    },
    /// The machine is not one the agent's role may run on.
    MachineNotAllowedForRole {
        /// The machine asked for.
        machine: String,
        /// The roles the agent holds.
        roles: Vec<String>,
    },
    /// The agent holds no valid virtual credential.
    VirtualCredentialsNotValid {
        /// The agent.
        agent: String,
    },
    /// The machine may not reach a destination the profile needs.
    EgressNotReachable {
        /// The machine.
        machine: String,
        /// Each destination it may not reach.
        destinations: Vec<String>,
    },
    /// The record a check reads does not exist.
    CheckRecordMissing {
        /// The check that could not run.
        check: Check,
        /// The card that makes the record.
        card: &'static str,
    },
    /// The profile version's record does not hold a field the command needs.
    ProfileVersionFieldMissing {
        /// The profile version.
        profile_version: String,
        /// The field it does not hold.
        field: &'static str,
    },
    /// An assigned value is outside its id grammar.
    CommandValueOutsideGrammar {
        /// The assignment whose value is outside its grammar.
        assignment: &'static str,
    },
    /// A start for the agent stands unconfirmed.
    StartUnconfirmed {
        /// The launch record that stands.
        launch_record: String,
    },
    /// The handle record answered with a credential value.
    CredentialValueInAnswer {
        /// The credential whose record carried a value.
        record: String,
        /// The field that carried it.
        field: String,
    },
}

/// The fourteen refusal names, in the order the brief gives them.
pub const REFUSAL_NAMES: [&str; 14] = [
    "start_field_not_allowed",
    "start_member_missing",
    "agent_unknown",
    "start_right_missing",
    "agent_not_active",
    "profile_version_not_reviewed",
    "machine_not_allowed_for_role",
    "virtual_credentials_not_valid",
    "egress_not_reachable",
    "check_record_missing",
    "profile_version_field_missing",
    "command_value_outside_grammar",
    "start_unconfirmed",
    "credential_value_in_answer",
];

impl Refusal {
    /// The refusal's stable name.
    pub fn name(&self) -> &'static str {
        let index = match self {
            Self::StartFieldNotAllowed { .. } => 0,
            Self::StartMemberMissing { .. } => 1,
            Self::AgentUnknown { .. } => 2,
            Self::StartRightMissing { .. } => 3,
            Self::AgentNotActive { .. } => 4,
            Self::ProfileVersionNotReviewed { .. } => 5,
            Self::MachineNotAllowedForRole { .. } => 6,
            Self::VirtualCredentialsNotValid { .. } => 7,
            Self::EgressNotReachable { .. } => 8,
            Self::CheckRecordMissing { .. } => 9,
            Self::ProfileVersionFieldMissing { .. } => 10,
            Self::CommandValueOutsideGrammar { .. } => 11,
            Self::StartUnconfirmed { .. } => 12,
            Self::CredentialValueInAnswer { .. } => 13,
        };
        REFUSAL_NAMES[index]
    }

    /// The refusal as the JSON object the route answers inside a list.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"refusal\":");
        json_string(&mut out, self.name());
        out.push_str(",\"words\":");
        json_string(&mut out, &self.to_string());
        out.push('}');
        out
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        match self {
            Self::StartFieldNotAllowed { member } => write!(
                f,
                "{name}: a start request names only the agent, the profile version and the machine, and '{member}' is not one of them; the executable and the working directory come only from the reviewed profile version"
            ),
            Self::StartMemberMissing { member } => {
                write!(f, "{name}: the start request does not name its {member}")
            }
            Self::AgentUnknown { agent } => write!(
                f,
                "{name}: no agent record exists for {agent}; a start never creates an agent"
            ),
            Self::StartRightMissing { agent } => write!(
                f,
                "{name}: you lack the right to start this agent, {agent}; only its responsible person or a directory administrator may start it"
            ),
            Self::AgentNotActive { state } => write!(
                f,
                "{name}: the check '{}' failed: the agent is {state}",
                Check::AgentIsActive.name()
            ),
            Self::ProfileVersionNotReviewed { profile_version } => write!(
                f,
                "{name}: the check '{}' failed: profile version {profile_version} has no review on record",
                Check::ProfileVersionIsReviewed.name()
            ),
            Self::MachineNotAllowedForRole { machine, roles } => write!(
                f,
                "{name}: the check '{}' failed: machine {machine} is not one the agent's role may run on (roles held: {})",
                Check::MachineIsAllowedForRole.name(),
                listed(roles)
            ),
            Self::VirtualCredentialsNotValid { agent } => write!(
                f,
                "{name}: the check '{}' failed: {agent} holds no valid virtual credential",
                Check::VirtualCredentialsAreValid.name()
            ),
            Self::EgressNotReachable {
                machine,
                destinations,
            } => write!(
                f,
                "{name}: the check '{}' failed: machine {machine} may not reach {}",
                Check::MachineMayReach.name(),
                listed(destinations)
            ),
            Self::CheckRecordMissing { check, card } => write!(
                f,
                "{name}: the check '{}' cannot run, because the record it reads does not exist; {card} makes that record",
                check.name()
            ),
            Self::ProfileVersionFieldMissing {
                profile_version,
                field,
            } => write!(
                f,
                "{name}: profile version {profile_version} does not hold its {field} in the record Ink1H1Os keeps, and it is taken from nowhere else"
            ),
            Self::CommandValueOutsideGrammar { assignment } => write!(
                f,
                "{name}: the value of {assignment} is outside its id grammar, so no command is given"
            ),
            Self::StartUnconfirmed { launch_record } => write!(
                f,
                "{name}: launch record {launch_record} stands unconfirmed for this agent; wait for its report, or withdraw it first"
            ),
            Self::CredentialValueInAnswer { record, field } => write!(
                f,
                "{name}: the handle record answered a credential value for {record} in its field '{field}'; no credential id is taken from that answer"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// A start that did not give a command, and why.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StartError {
    /// Refused by name, with every check that ran.
    #[error("{0}")]
    Refused(Refused),
    /// No launch record has the id asked for.
    #[error("launch_record_unknown: no launch record has the id {launch_record}")]
    LaunchRecordUnknown {
        /// The id asked for.
        launch_record: String,
    },
    /// A record could not be read or written.
    #[error("launch_records_unavailable: {reason}")]
    Unavailable {
        /// What failed.
        reason: String,
    },
}

impl StartError {
    /// The error as the JSON body the route answers.
    pub fn to_json(&self) -> String {
        match self {
            Self::Refused(refused) => refused.to_json(),
            Self::LaunchRecordUnknown { .. } => error_json("launch_record_unknown", self),
            Self::Unavailable { .. } => error_json("launch_records_unavailable", self),
        }
    }

    /// The refusals, when this error is a refusal.
    pub fn refused(&self) -> Option<&Refused> {
        match self {
            Self::Refused(refused) => Some(refused),
            _ => None,
        }
    }
}

impl From<Refusal> for StartError {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(Refused {
            refusals: vec![refusal],
            checks: Vec::new(),
        })
    }
}

fn error_json(name: &str, error: &StartError) -> String {
    let mut out = String::from("{\"error\":");
    json_string(&mut out, name);
    out.push_str(",\"words\":");
    json_string(&mut out, &error.to_string());
    out.push('}');
    out
}

/// Every refusal of one start, and every check that ran before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The refusals, by name.
    pub refusals: Vec<Refusal>,
    /// Every check that ran, each with its result; empty when the start was
    /// refused before the checks.
    pub checks: Vec<crate::start::checks::CheckReport>,
}

impl Refused {
    /// Whether one of the refusals has `name`.
    pub fn names(&self, name: &str) -> bool {
        self.refusals.iter().any(|refusal| refusal.name() == name)
    }

    /// The refusal as the JSON body the route answers.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"refused\":[");
        for (at, refusal) in self.refusals.iter().enumerate() {
            if at > 0 {
                out.push(',');
            }
            out.push_str(&refusal.to_json());
        }
        out.push_str("],\"checks\":");
        crate::start::checks::checks_json(&mut out, &self.checks);
        out.push('}');
        out
    }
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (at, refusal) in self.refusals.iter().enumerate() {
            if at > 0 {
                f.write_str("; ")?;
            }
            refusal.fmt(f)?;
        }
        Ok(())
    }
}

fn listed(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

/// Append `text` to `out` as a JSON string.
pub(crate) fn json_string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                const DIGITS: &[u8; 16] = b"0123456789abcdef";
                let code = u8::try_from(u32::from(c)).unwrap_or(0);
                out.push_str("\\u00");
                out.push(char::from(DIGITS[usize::from(code >> 4)]));
                out.push(char::from(DIGITS[usize::from(code & 0x0f)]));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Append `items` to `out` as a JSON array of strings.
pub(crate) fn json_strings(out: &mut String, items: &[String]) {
    out.push('[');
    for (at, item) in items.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        json_string(out, item);
    }
    out.push(']');
}
