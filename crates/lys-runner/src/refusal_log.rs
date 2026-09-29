//! The runner's log of refusals: a denied call is appended, durably, before
//! its caller is answered.
//!
//! A call is answered only once it has been judged under the policy its
//! proved session was launched with. A peer the runner cannot prove is
//! denied and nothing is written in any session's name. A denial is kept in
//! the feed as one unit with its attempt, so the same attempt delivered
//! again, after a lost answer or a harness that runs its hook twice, reuses
//! the record and makes no second one. An append that fails never becomes a
//! permission: the call is denied all the same, by name, and the gap in the
//! audit is held and shown until it is recorded. A call no rule denies is
//! answered with no decision at all, so the harness's own permission flow
//! decides it: this log never holds, and the judge never gives, an allow.

use std::collections::BTreeMap;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::judge::{Asked, Judgement, Needed, judge};
use crate::peer::Leader;
use crate::refusals::{JudgeAsk, REFUSAL_VERSION, RefusalRecord, Verdict, ask, question};
use crate::session::{Guard, Sessions, now_ms};
use crate::tracking_store::{Body, Commit, attempt_key};

/// The attempt's identity: the harness's tool-use id when it gave one,
/// else a digest of the call itself, so a repeat is the same attempt.
pub fn attempt_of(session: &str, asked: &JudgeAsk) -> String {
    if let Some(attempt) = asked.attempt.as_deref().filter(|given| !given.is_empty()) {
        return format!("tool-use:{attempt}");
    }
    let mut hasher = Sha256::new();
    hasher.update(b"lys-runner/judge-attempt/v1\n");
    hasher.update(session.as_bytes());
    hasher.update(b"\n");
    hasher.update(asked.tool_name.as_bytes());
    hasher.update(b"\n");
    hasher.update(asked.tool_input.to_string().as_bytes());
    format!("digest:{}", crate::protocol::hex(&hasher.finalize()))
}

/// What a denial is made from.
struct Denial {
    check: String,
    rule: Option<String>,
    target: String,
    permission: Option<Needed>,
    grantor: Option<String>,
    words: String,
}

/// Judge `asked`, from the peer on `stream`, and answer its verdict.
pub fn answer(
    sessions: &Sessions,
    stream: &UnixStream,
    asked: &JudgeAsk,
    left: &AtomicBool,
) -> Verdict {
    let session = match crate::peer::prove(stream, &sessions.leaders()) {
        Ok(session) => session,
        Err(unproved) => {
            return Verdict::denied(
                "peer_unproved",
                format!("the asker is not proved to be a session of this runner: {unproved}"),
                "not_attributed",
            );
        }
    };
    let Some(guard) = sessions.guard(&session) else {
        return Verdict::pass(None);
    };
    let Some(policy) = guard.policy.clone() else {
        return Verdict::pass(None);
    };
    let attempt = attempt_of(&session, asked);
    match sessions
        .lock()
        .feed
        .attempt(&attempt_key(&session, &attempt))
    {
        Ok(Some(record)) => return Verdict::of(&record, "reused"),
        Ok(None) => {}
        Err(unread) => crate::error::said(&format!(
            "session {session}: the refusal kept for attempt {attempt} does not read: {unread}"
        )),
    }
    let judged = judge(
        &policy,
        &Asked {
            tool: &asked.tool_name,
            input: &asked.tool_input,
            cwd: Path::new(&guard.cwd),
            subagent: asked.subagent,
        },
    );
    let denial = match judged {
        Judgement::Pass => return Verdict::pass(Some(policy.version)),
        Judgement::Deny {
            refusal,
            rule,
            target,
            words,
        } => Denial {
            check: refusal.to_owned(),
            rule,
            target,
            permission: None,
            grantor: None,
            words,
        },
        Judgement::Ask { needed, target } => {
            let who = (attempt.as_str(), session.as_str(), policy.agent.as_str());
            match lifted(sessions, who, policy.version, &needed, left) {
                None => return Verdict::pass(Some(policy.version)),
                Some(denial) => Denial { target, ..denial },
            }
        }
    };
    let record = RefusalRecord {
        version: REFUSAL_VERSION,
        source: sessions.runner().to_owned(),
        attempt: attempt.clone(),
        session: session.clone(),
        agent: policy.agent.clone(),
        at: now_ms(),
        tool: asked.tool_name.clone(),
        target: denial.target,
        policy_version: policy.version,
        rule: denial.rule,
        check: denial.check,
        grantable: denial.permission.is_some(),
        permission: denial.permission,
        grantor: denial.grantor,
        words: denial.words,
    };
    keep(sessions, &session, &attempt, &record)
}

/// Ask each permission in turn; answer `None` when every one is given now,
/// else the first refusal, its target left for the caller to name.
fn lifted(
    sessions: &Sessions,
    who: (&str, &str, &str),
    version: u64,
    needed: &[Needed],
    left: &AtomicBool,
) -> Option<Denial> {
    for need in needed {
        let (check, grantor, words) = match ask(sessions, question(who, version, need), left) {
            Ok(answer) if answer.permitted => continue,
            Ok(answer) => (
                "grant_refused",
                answer.grantor,
                format!(
                    "rule `{}` denies it without {} on {}:{}: {}",
                    need.rule, need.action, need.resource.kind, need.resource.id, answer.words
                ),
            ),
            Err(reason) => (
                "grant_state_unavailable",
                None,
                format!(
                    "rule `{}` needs {} on {}:{}, and the live grant authority did not answer: {reason}",
                    need.rule, need.action, need.resource.kind, need.resource.id
                ),
            ),
        };
        return Some(Denial {
            check: check.to_owned(),
            rule: Some(need.rule.clone()),
            target: String::new(),
            permission: Some(need.clone()),
            grantor,
            words,
        });
    }
    None
}

/// Append `record` before answering; a failed append denies all the same,
/// and holds the gap for the page to show.
fn keep(sessions: &Sessions, session: &str, attempt: &str, record: &RefusalRecord) -> Verdict {
    let mut table = sessions.lock();
    let commit = Commit {
        source: None,
        attempt: Some(attempt.to_owned()),
        control: None,
    };
    let appended = table.feed.append(
        session,
        record.at,
        vec![Body::Refusal(record.clone())],
        commit,
    );
    match appended {
        Ok(_) => {
            drop(table);
            sessions.wake();
            Verdict::of(record, "recorded")
        }
        Err(error) => {
            let gap = table.gaps.entry(session.to_owned()).or_default();
            gap.lost += 1;
            gap.since.get_or_insert(record.at);
            gap.words = format!("a refusal was not recorded: {error}");
            drop(table);
            crate::error::said(&format!(
                "session {session}: audit_incomplete: the refusal of attempt {attempt} was not appended: {error}"
            ));
            let mut verdict = Verdict::of(record, "incomplete");
            verdict.reason = format!(
                "{} (audit_incomplete: the refusal could not be recorded)",
                verdict.reason
            );
            verdict
        }
    }
}

/// Where a session's audit has a gap.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditGap {
    /// How many refusals were not recorded.
    pub lost: u64,
    /// Since when, in milliseconds since the Unix epoch.
    pub since: Option<u64>,
    /// Why, in words.
    pub words: String,
}

impl Sessions {
    /// The leaders of every running session, by session.
    pub fn leaders(&self) -> BTreeMap<String, Leader> {
        self.lock()
            .sessions
            .iter()
            .filter(|(_, session)| session.ended.is_none())
            .filter_map(|(id, session)| Some((id.clone(), session.guard.leader.clone()?)))
            .collect()
    }

    /// What session `id` was started with beside its launch.
    pub fn guard(&self, id: &str) -> Option<Guard> {
        self.lock()
            .sessions
            .get(id)
            .map(|session| session.guard.clone())
    }

    /// Each session whose audit has a gap, and the gap.
    pub fn gaps(&self) -> BTreeMap<String, AuditGap> {
        self.lock().gaps.clone()
    }
}
