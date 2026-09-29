//! What the judge answers, what a refusal records, and the channel the
//! live grant authority answers grantable rules on.
//!
//! A refusal record names its source runner, the attempt, the proved
//! session, the instant, the tool, a safe summary of its target (a path, a
//! host or the tool alone, never the tool's input), the policy version, the
//! rule or check that refused and whether any grant could lift it. It
//! carries no secret and no tool body.
//!
//! A grantable rule is asked of the live grant authority on the grant
//! channel: one connection the server holds open, signed like every other
//! request, on which the runner writes each question and the server writes
//! its answer, in turn. The channel's own connection is the authority's
//! liveness: with none attached, a question is not asked and the call is
//! denied `grant_state_unavailable`; a channel that closes before it
//! answers denies every question it held, and so does an answer for
//! another attempt, rule or policy version. No answer is kept to stand for
//! a later call, and nothing here waits on a clock: a question ends on its
//! answer, on its channel closing, or on its caller leaving.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::judge::{NamedResource, Needed};
use crate::session::Sessions;

/// The version of every refusal record.
pub const REFUSAL_VERSION: u32 = 1;

/// One tool call the judge denied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefusalRecord {
    /// [`REFUSAL_VERSION`].
    pub version: u32,
    /// The runner that refused it.
    pub source: String,
    /// The attempt's stable identity.
    pub attempt: String,
    /// The proved session.
    pub session: String,
    /// The agent the policy constrains.
    pub agent: String,
    /// When, in milliseconds since the Unix epoch.
    pub at: u64,
    /// The tool.
    pub tool: String,
    /// A safe summary of its target.
    pub target: String,
    /// The policy version installed for the session's launch.
    pub policy_version: u64,
    /// The rule that refused it, when a rule did.
    pub rule: Option<String>,
    /// The check that refused it, by name.
    pub check: String,
    /// Whether a grant could lift it.
    pub grantable: bool,
    /// The permission it needed, when it was grantable.
    pub permission: Option<Needed>,
    /// Who could grant it, only where the grant authority named them.
    pub grantor: Option<String>,
    /// Why, in words.
    pub words: String,
}

/// What a judged call is answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verdict {
    /// Whether the call is denied. When it is not, the harness's own
    /// permission flow decides: there is no allow.
    pub deny: bool,
    /// The refusal, by name, when it is denied.
    pub refusal: Option<String>,
    /// Why, in words.
    pub reason: String,
    /// The policy version it was judged under.
    pub policy_version: Option<u64>,
    /// The rule that refused it.
    pub rule: Option<String>,
    /// Whether the denial was recorded: `recorded`, `reused`, `incomplete`
    /// or `not_attributed`; `none` when nothing was denied.
    pub audit: String,
}

impl Verdict {
    /// Not denied: the harness decides.
    pub fn pass(policy_version: Option<u64>) -> Self {
        Self {
            deny: false,
            refusal: None,
            reason: String::new(),
            policy_version,
            rule: None,
            audit: "none".to_owned(),
        }
    }

    /// Denied `refusal`, for `reason`, recorded as `audit` says.
    pub fn denied(refusal: &str, reason: String, audit: &str) -> Self {
        Self {
            deny: true,
            refusal: Some(refusal.to_owned()),
            reason,
            policy_version: None,
            rule: None,
            audit: audit.to_owned(),
        }
    }

    /// The verdict a kept refusal gives, recorded as `audit` says.
    pub fn of(record: &RefusalRecord, audit: &str) -> Self {
        Self {
            deny: true,
            refusal: Some(record.check.clone()),
            reason: record.words.clone(),
            policy_version: Some(record.policy_version),
            rule: record.rule.clone(),
            audit: audit.to_owned(),
        }
    }
}

/// A tool call a session's harness asks the judge about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JudgeAsk {
    /// The call's stable identity: the harness's tool-use id.
    #[serde(default)]
    pub attempt: Option<String>,
    /// The exact tool name.
    pub tool_name: String,
    /// Its structured input.
    pub tool_input: Value,
    /// The session the body names: never what the call is attributed to.
    #[serde(default)]
    pub claimed_session: Option<String>,
    /// Whether a subagent asks.
    #[serde(default)]
    pub subagent: bool,
}

/// A grantable rule asked of the live grant authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantQuestion {
    /// The attempt.
    pub attempt: String,
    /// The proved session.
    pub session: String,
    /// The session's agent.
    pub agent: String,
    /// The policy version installed for the launch.
    pub policy_version: u64,
    /// The rule.
    pub rule: String,
    /// The resource.
    pub resource: NamedResource,
    /// The action.
    pub action: String,
}

/// The live grant authority's answer to one question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantAnswer {
    /// The attempt it answers.
    pub attempt: String,
    /// The session it answers for.
    pub session: String,
    /// The policy version it was asked under.
    pub policy_version: u64,
    /// The rule it answers.
    pub rule: String,
    /// Whether the agent holds the permission now.
    pub permitted: bool,
    /// Who could grant it, where the authority establishes one.
    #[serde(default)]
    pub grantor: Option<String>,
    /// The authority's words.
    #[serde(default)]
    pub words: String,
}

impl GrantAnswer {
    /// Whether this answers `question`: the same attempt, session, policy
    /// version and rule.
    pub fn answers(&self, question: &GrantQuestion) -> bool {
        self.attempt == question.attempt
            && self.session == question.session
            && self.policy_version == question.policy_version
            && self.rule == question.rule
    }
}

/// Where the grant channel stands.
#[derive(Debug, Default)]
pub(crate) struct Desk {
    next: u64,
    live: BTreeSet<u64>,
    asked: VecDeque<GrantQuestion>,
    taken: BTreeMap<String, u64>,
    answers: BTreeMap<String, Result<GrantAnswer, String>>,
}

fn key(question: &GrantQuestion) -> String {
    format!(
        "{}\n{}\n{}",
        question.session, question.attempt, question.rule
    )
}

/// Build the question for `needed` on the proved session.
pub fn question(
    (attempt, session, agent): (&str, &str, &str),
    policy_version: u64,
    needed: &Needed,
) -> GrantQuestion {
    GrantQuestion {
        attempt: attempt.to_owned(),
        session: session.to_owned(),
        agent: agent.to_owned(),
        policy_version,
        rule: needed.rule.clone(),
        resource: needed.resource.clone(),
        action: needed.action.clone(),
    }
}

/// Ask `question` of the live grant authority and wait for its answer:
/// refused, with the reason, when no channel is attached, when the channel
/// that took it closes first, or when the caller leaves.
pub fn ask(
    sessions: &Sessions,
    question: GrantQuestion,
    left: &AtomicBool,
) -> Result<GrantAnswer, String> {
    let named = key(&question);
    let mut table = sessions.lock();
    if table.desk.live.is_empty() {
        return Err("no grant authority is attached to this runner".to_owned());
    }
    table.desk.asked.push_back(question);
    drop(table);
    sessions.wake();
    let answered = sessions.until_any(left, |table| {
        if let Some(answer) = table.desk.answers.remove(&named) {
            return Some(answer);
        }
        let lost = match table.desk.taken.get(&named) {
            Some(channel) => !table.desk.live.contains(channel),
            None => table.desk.live.is_empty(),
        };
        lost.then(|| Err("the grant authority's channel closed before it answered".to_owned()))
    });
    let mut table = sessions.lock();
    table.desk.taken.remove(&named);
    table.desk.asked.retain(|waiting| key(waiting) != named);
    drop(table);
    answered.unwrap_or_else(|left| Err(format!("the question ended unanswered: {left}")))
}

/// Serve the grant channel on `stream`: write each question as one line,
/// read the authority's answer line to it, and hand it to the call that
/// asked, until the connection closes.
pub fn serve(sessions: &Arc<Sessions>, stream: &UnixStream) {
    let channel = {
        let mut table = sessions.lock();
        let channel = table.desk.next;
        table.desk.next += 1;
        table.desk.live.insert(channel);
        channel
    };
    crate::error::said("the grant authority attached its channel");
    let never = AtomicBool::new(false);
    let reason = match stream.try_clone() {
        Ok(reading) => carry(sessions, stream, BufReader::new(reading), channel, &never),
        Err(error) => error.to_string(),
    };
    let mut table = sessions.lock();
    table.desk.live.remove(&channel);
    drop(table);
    sessions.wake();
    crate::error::said(&format!("the grant authority's channel closed: {reason}"));
}

fn carry(
    sessions: &Sessions,
    mut writer: &UnixStream,
    mut reader: BufReader<UnixStream>,
    channel: u64,
    never: &AtomicBool,
) -> String {
    loop {
        let taken = sessions.until_any(never, |table| {
            let question = table.desk.asked.pop_front()?;
            table.desk.taken.insert(key(&question), channel);
            Some(question)
        });
        let question = match taken {
            Ok(question) => question,
            Err(error) => return error.to_string(),
        };
        let named = key(&question);
        let written = serde_json::to_string(&question)
            .map_err(|error| error.to_string())
            .and_then(|line| {
                writer
                    .write_all(line.as_bytes())
                    .and_then(|()| writer.write_all(b"\n"))
                    .and_then(|()| writer.flush())
                    .map_err(|error| error.to_string())
            });
        let mut line = String::new();
        let read = written.and_then(|()| match reader.read_line(&mut line) {
            Ok(0) => Err("the authority closed the channel".to_owned()),
            Ok(_) => Ok(()),
            Err(error) => Err(error.to_string()),
        });
        if let Err(reason) = read {
            return reason;
        }
        let answer = serde_json::from_str::<GrantAnswer>(line.trim_end())
            .map_err(|error| format!("the authority's answer does not read: {error}"))
            .and_then(|answer| {
                if answer.answers(&question) {
                    Ok(answer)
                } else {
                    Err("the authority answered another attempt, rule or policy version".to_owned())
                }
            });
        sessions.lock().desk.answers.insert(named, answer);
        sessions.wake();
    }
}
