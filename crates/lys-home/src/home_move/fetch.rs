//! `lys-home fetch` (HOME-019 R6): the shipped ref pulled into a new, empty
//! home, verified strictly, and one arrival hung beside each session's head.
//!
//! Before anything is written: the remote is a path on this machine, the
//! target holds no `sessions/` and is absent or an empty directory, and the
//! arrival data for this remote fits the event cap. Then every path fetch
//! creates is recorded as it is created ([`Created`]): the target when it
//! did not exist, `<dir>/.git`, each file and directory the checkout writes,
//! each lock file opening a session leaves. The ref is fetched, its tree is
//! written into the target with HEAD at the fetched commit, and the home is
//! verified: sessions, blocks and templates. A home that fails is removed
//! and reported by session and by hash; nothing arrives.
//!
//! A home that passes gets, per session in the order the home lists them,
//! one `arrival` event beside its head with a fresh execution id of its
//! own; the head does not move. The changed session and index files are
//! committed as one commit whose only parent is the fetched commit, and
//! both `refs/lys/home` and HEAD are set to it, so the target's status is
//! clean and a later ship from it carries the arrivals. A failure in the
//! appends or the commit removes what fetch created and refuses as
//! `arrival_failed`. Nothing is written to the remote, no block or template
//! is written, and no home-level id file exists.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::events::arrival;
use crate::home_move::git::Git;
use crate::home_move::remote::take_remote;
use crate::home_move::ship::HOME_REF;
use crate::home_move::undo::Created;
use crate::record::entries::{CUSTOM_HARNESS_EVENT, EntryBody};
use crate::record::index::Index;
use crate::record::verify::{Verification, verify_home};
use crate::record::{Home, fresh_id};

/// The line `<dir>/.git/info/exclude` holds, so the lock files a session
/// leaves are never shown as untracked.
pub const EXCLUDE_LOCKS: &str = "/sessions/*..lock\n";

/// One arrived session and its fresh execution id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Arrived {
    /// The session id.
    pub session: String,
    /// Its execution id, 32 lowercase hex digits.
    pub execution: String,
}

/// What a fetch did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FetchReport {
    /// The fetched commit.
    pub commit: String,
    /// The remote's absolute path.
    pub remote: PathBuf,
    /// The ref fetched.
    #[serde(rename = "ref")]
    pub git_ref: String,
    /// Each session and its execution id, in the order the home lists them.
    pub sessions: Vec<Arrived>,
}

/// How a fetch ended when it did not refuse by error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fetched {
    /// The home arrived.
    Arrived(FetchReport),
    /// The fetched home failed verification; everything fetch created was
    /// removed.
    VerificationFailed(Verification),
}

/// Fetch `refs/lys/home` from `remote`, a path resolved against `base`,
/// into the new home `home_dir`, running git with PATH from `inherited`.
pub fn fetch(
    remote: &str,
    home_dir: &Path,
    base: &Path,
    inherited: &[(OsString, OsString)],
) -> Result<Fetched, HomeError> {
    let remote = take_remote(remote, base)?;
    let target = std::path::absolute(home_dir)
        .map_err(|e| HomeError::io("resolving the target", home_dir, e))?;
    let existed = check_target(&target)?;
    let remote_text = remote.to_string_lossy().into_owned();
    arrival(&"0".repeat(40), &remote_text, HOME_REF, &"0".repeat(32)).data()?;
    let mut created = Created::new(&target, existed);
    match arrive(&mut created, &target, &remote, &remote_text, inherited) {
        Ok(Fetched::VerificationFailed(found)) => {
            undo_or_report(&created, &target, "verification_failed")?;
            Ok(Fetched::VerificationFailed(found))
        }
        Ok(arrived) => Ok(arrived),
        Err(e) => {
            undo_or_report(&created, &target, &e.to_string())?;
            Err(e)
        }
    }
}

/// Whether the target exists: one holding `sessions/` refuses as
/// `target_holds_home`, one that is not an empty directory as
/// `target_not_empty`.
fn check_target(target: &Path) -> Result<bool, HomeError> {
    if !target.exists() {
        return Ok(false);
    }
    if target.join("sessions").exists() {
        return Err(HomeError::TargetHoldsHome {
            path: target.to_path_buf(),
        });
    }
    let empty = target.is_dir()
        && std::fs::read_dir(target)
            .map_err(|e| HomeError::io("listing the target", target, e))?
            .next()
            .is_none();
    if empty {
        Ok(true)
    } else {
        Err(HomeError::TargetNotEmpty {
            path: target.to_path_buf(),
        })
    }
}

/// Remove what fetch created; a removal that fails too refuses as
/// `FetchHalfWritten`, carrying the reason fetch stopped.
fn undo_or_report(created: &Created, target: &Path, reason: &str) -> Result<(), HomeError> {
    created
        .undo()
        .map_err(|cleanup| HomeError::FetchHalfWritten {
            path: target.to_path_buf(),
            reason: reason.to_owned(),
            cleanup: cleanup.to_string(),
        })
}

/// Everything after the first write: the fetch, the checkout, the
/// verification, the arrivals and their commit.
fn arrive(
    created: &mut Created,
    target: &Path,
    remote: &Path,
    remote_text: &str,
    inherited: &[(OsString, OsString)],
) -> Result<Fetched, HomeError> {
    created.make_dir(target)?;
    let git_dir = target.join(".git");
    created.repository(&git_dir);
    let git = Git::new(inherited, &git_dir, Some(target.to_path_buf()));
    git.init(false)?;
    let refspec = format!("{HOME_REF}:{HOME_REF}");
    git.output(
        &[&"fetch", &"--quiet", &"--no-tags", &remote, &refspec],
        None,
    )?;
    let commit = git
        .commit_of(HOME_REF)?
        .ok_or_else(|| HomeError::GitFailed {
            subcommand: "fetch".to_owned(),
            status: format!("no {HOME_REF} after the fetch"),
        })?;
    check_out(created, &git, target, &commit)?;
    let home = Home::read(target)?;
    let found = verify_home(&home)?;
    if !found.is_clean() {
        return Ok(Fetched::VerificationFailed(found));
    }
    let sessions = append_arrivals(created, &home, &commit, remote_text)?;
    commit_arrivals(&git, target, &commit, remote_text, &sessions)?;
    let exclude = git_dir.join("info").join("exclude");
    if let Some(info) = exclude.parent() {
        std::fs::create_dir_all(info)
            .map_err(|e| HomeError::io("creating the repository's info", info, e))?;
    }
    std::fs::write(&exclude, EXCLUDE_LOCKS)
        .map_err(|e| HomeError::io("writing the repository's exclude", &exclude, e))?;
    Ok(Fetched::Arrived(FetchReport {
        commit,
        remote: remote.to_path_buf(),
        git_ref: HOME_REF.to_owned(),
        sessions,
    }))
}

/// Write the commit's tree into the target, recording each file and
/// directory, and set HEAD to the commit.
fn check_out(
    created: &mut Created,
    git: &Git,
    target: &Path,
    commit: &str,
) -> Result<(), HomeError> {
    for path in git.tree_paths(commit)? {
        let file = target.join(&path);
        let dirs: Vec<PathBuf> = file
            .ancestors()
            .skip(1)
            .take_while(|dir| dir.starts_with(target) && *dir != target)
            .map(Path::to_path_buf)
            .collect();
        for dir in dirs.iter().rev() {
            created.dir(dir);
        }
        created.file(&file);
    }
    git.output(&[&"read-tree", &commit], None)?;
    git.output(&[&"checkout-index", &"--all"], None)?;
    git.output(&[&"update-ref", &"--no-deref", &"HEAD", &commit], None)?;
    Ok(())
}

/// Append one arrival beside each session's head, recording each lock
/// file; a failure names the session.
fn append_arrivals(
    created: &mut Created,
    home: &Home,
    commit: &str,
    remote_text: &str,
) -> Result<Vec<Arrived>, HomeError> {
    let mut sessions = Vec::new();
    for session in home.session_ids()? {
        let file = home.session_path(&session)?;
        created.file(&Index::lock_path(&file));
        let execution = fresh_id();
        let appended = arrival(commit, remote_text, HOME_REF, &execution)
            .data()
            .and_then(|data| {
                let mut open = home.open_session(&session)?;
                open.append_beside(EntryBody::Custom {
                    custom_type: CUSTOM_HARNESS_EVENT.to_owned(),
                    data: Some(data),
                })
            });
        if appended.is_err() {
            return Err(HomeError::ArrivalFailed {
                step: format!("append to session `{session}`"),
            });
        }
        sessions.push(Arrived { session, execution });
    }
    Ok(sessions)
}

/// Commit the changed session and index files as one commit whose only
/// parent is the fetched commit, and set `refs/lys/home` and HEAD to it; a
/// git failure refuses as `arrival_failed`, naming the subcommand.
fn commit_arrivals(
    git: &Git,
    target: &Path,
    fetched: &str,
    remote_text: &str,
    sessions: &[Arrived],
) -> Result<(), HomeError> {
    let mut changed = Vec::with_capacity(sessions.len() * 2);
    for arrived in sessions {
        changed.push(format!("sessions/{}.index.jsonl", arrived.session));
        changed.push(format!("sessions/{}.jsonl", arrived.session));
    }
    changed.sort_unstable();
    let message = format!("arrival from {remote_text} {HOME_REF} at {fetched}");
    let committed = git.stage(target, &changed).and_then(|()| {
        let tree = git.text(&[&"write-tree"])?;
        let commit = git.commit_tree(&tree, Some(fetched), &message)?;
        git.output(&[&"update-ref", &HOME_REF, &commit, &fetched], None)?;
        git.output(&[&"update-ref", &"--no-deref", &"HEAD", &commit], None)?;
        Ok(())
    });
    committed.map_err(|e| HomeError::ArrivalFailed {
        step: match e {
            HomeError::GitFailed { subcommand, .. } => subcommand,
            _ => "git".to_owned(),
        },
    })
}

/// The JSON `fetch` prints for a home that failed verification.
#[must_use]
pub fn refusal_report(found: &Verification) -> Value {
    json!({
        "command": "fetch",
        "refused": "verification_failed",
        "sessions": found.sessions,
        "bad_blocks": found.bad_blocks,
        "bad_templates": found.bad_templates,
    })
}
