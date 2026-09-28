//! `lys-home ship` (HOME-019 R5): a home pushed as the one ref
//! `refs/lys/home` to a bare repository at a path on this machine.
//!
//! Every check runs before anything is written, in this order: the remote
//! is a path on this machine; the home exists; it lists a session; the
//! remote path is absent or a bare repository; every session's lock is
//! free, and all are then held; every session has its index and its head;
//! every index is its file's and every head is indexed (checked strictly,
//! never rebuilt); the home's repository, if it has one, tracks nothing
//! outside the tracked set. Then the tracked set is staged exactly and
//! committed on `refs/lys/home` (no commit when the tree is the last
//! commit's), the locks are released, the remote is made when absent, and
//! the commit is pushed without force, after refusing a remote ref the
//! commit does not descend from.
//!
//! Ship writes only `<home>/.git`, the sessions' lock files and the remote:
//! no file of the tracked set, no index and no head. It does not scan the
//! tracked files for secrets and offers no way to leave a session out.

use crate::error::MoveError;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::HomeError;
use crate::home_move::git::Git;
use crate::home_move::remote::take_remote;
use crate::record::Home;
use crate::record::hold::hold_all;
use crate::record::index::Index;
use crate::record::tracked::tracked_set;
use crate::record::verify::{SessionReason, verify_sessions};

/// The one ref a home is shipped and fetched on.
pub const HOME_REF: &str = "refs/lys/home";

/// What a ship did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShipReport {
    /// The commit pushed.
    pub commit: String,
    /// The ref it was pushed to.
    #[serde(rename = "ref")]
    pub git_ref: String,
    /// The remote's absolute path.
    pub remote: PathBuf,
    /// Whether this ship made `<home>/.git`.
    pub initialised: bool,
    /// Whether this ship made the remote.
    pub remote_created: bool,
    /// Whether no new commit was made.
    pub unchanged: bool,
    /// Why no commit was made, or the empty string.
    pub note: String,
}

/// How a ship ended when it did not refuse by error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shipped {
    /// The home was pushed.
    Pushed(ShipReport),
    /// Some session's index is not its file's or its head is not indexed;
    /// nothing was written.
    StaleIndex(Vec<SessionReason>),
}

/// Ship the home at `home_dir` to `remote`, a path resolved against `base`,
/// running git with PATH from `inherited`. The remote and the home are both
/// made absolute before anything runs.
pub fn ship(
    home_dir: &Path,
    remote: &str,
    base: &Path,
    inherited: &[(OsString, OsString)],
) -> Result<Shipped, HomeError> {
    let remote = take_remote(remote, base)?;
    // Absolute before git is told where the repository is: git runs in the
    // home's directory, where a relative GIT_DIR would name another place.
    // Nothing after the checks below names the home but through `home`.
    let home_dir = std::path::absolute(home_dir)
        .map_err(|e| HomeError::io("resolving the home", home_dir, e))?;
    let home = Home::read(&home_dir)?;
    let sessions = home.session_ids()?;
    if sessions.is_empty() {
        return Err(HomeError::Move(MoveError::EmptyHome { path: home_dir }));
    }
    let remote_exists = check_remote(&remote)?;
    let held = hold_all(&home, &sessions)?;
    for session in &sessions {
        let file = home.session_path(session)?;
        if !Index::index_path(&file).is_file() {
            return Err(HomeError::Move(MoveError::IndexMissing {
                session: session.clone(),
                home: home_dir,
            }));
        }
        if !Index::head_path(&file).is_file() {
            return Err(HomeError::Move(MoveError::HeadMissing {
                session: session.clone(),
                home: home_dir,
            }));
        }
    }
    let stale = verify_sessions(&home)?;
    if !stale.is_empty() {
        return Ok(Shipped::StaleIndex(stale));
    }
    let tracked = tracked_set(&home)?;
    let git_dir = home.root().join(".git");
    let git = Git::new(inherited, &git_dir, Some(home.root().to_path_buf()));
    let initialised = !git_dir.exists();
    if !initialised {
        refuse_foreign(&git, &tracked)?;
    }
    if initialised {
        git.init(false)?;
    }
    let tree = git.stage_exactly(home.root(), &tracked)?;
    let last = git.commit_of(HOME_REF)?;
    let last_tree = match &last {
        Some(commit) => Some(git.text(&[&"rev-parse", &format!("{commit}^{{tree}}")])?),
        None => None,
    };
    let unchanged = last_tree.as_deref() == Some(tree.as_str());
    let commit = if let (Some(commit), true) = (&last, unchanged) {
        commit.clone()
    } else {
        let message = format!("lys-home ship: {} sessions", sessions.len());
        let commit = git.commit_tree(&tree, last.as_deref(), &message)?;
        let old = last.unwrap_or_else(|| "0".repeat(40));
        git.output(&[&"update-ref", &HOME_REF, &commit, &old], None)?;
        commit
    };
    drop(held);
    let remote_git = Git::new(inherited, &remote, None);
    if !remote_exists {
        remote_git.init(true)?;
    }
    if let Some(at) = remote_git.commit_of(HOME_REF)?
        && at != commit
        && !descends(&git, &at, &commit)?
    {
        return Err(HomeError::Move(MoveError::RefDiverged {
            git_ref: HOME_REF.to_owned(),
            remote_commit: at,
            commit,
        }));
    }
    let refspec = format!("{commit}:{HOME_REF}");
    git.output(
        &[&"push", &"--quiet", &"--no-verify", &remote, &refspec],
        None,
    )?;
    let note = if unchanged {
        format!("the home is unchanged since {commit}; no new commit was made")
    } else {
        String::new()
    };
    Ok(Shipped::Pushed(ShipReport {
        commit,
        git_ref: HOME_REF.to_owned(),
        remote,
        initialised,
        remote_created: !remote_exists,
        unchanged,
        note,
    }))
}

/// Whether the remote path exists; one that exists and is not a bare
/// repository refuses as `remote_not_bare`, naming what was found.
fn check_remote(remote: &Path) -> Result<bool, HomeError> {
    let meta = match std::fs::metadata(remote) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(HomeError::io("reading the remote", remote, e)),
    };
    let found = if !meta.is_dir() {
        Some("a file")
    } else if remote.join(".git").exists() {
        Some("a non-bare repository")
    } else if remote.join("HEAD").is_file()
        && remote.join("objects").is_dir()
        && remote.join("refs").is_dir()
    {
        None
    } else {
        Some("a directory that is not a bare repository")
    };
    match found {
        Some(found) => Err(HomeError::Move(MoveError::RemoteNotBare {
            path: remote.to_path_buf(),
            found,
        })),
        None => Ok(true),
    }
}

/// Refuse as `foreign_tracked` when the home's index or its `refs/lys/home`
/// commit holds a path outside the tracked set.
fn refuse_foreign(git: &Git, tracked: &[String]) -> Result<(), HomeError> {
    let mut held = git.index_paths()?;
    if let Some(commit) = git.commit_of(HOME_REF)? {
        held.extend(git.tree_paths(&commit)?);
    }
    let mut foreign: Vec<String> = held
        .into_iter()
        .filter(|path| tracked.binary_search(path).is_err())
        .collect();
    foreign.sort_unstable();
    foreign.dedup();
    if foreign.is_empty() {
        Ok(())
    } else {
        Err(HomeError::Move(MoveError::ForeignTracked {
            paths: foreign,
        }))
    }
}

/// Whether `commit` descends from `ancestor`: `ancestor` is held in the
/// home's repository and is an ancestor of `commit`.
fn descends(git: &Git, ancestor: &str, commit: &str) -> Result<bool, HomeError> {
    if git.commit_of(ancestor)?.is_none() {
        return Ok(false);
    }
    let ran = git.run(&[&"merge-base", &"--is-ancestor", &ancestor, &commit], None)?;
    match ran.code {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        code => Err(HomeError::Move(MoveError::GitFailed {
            subcommand: "merge-base".to_owned(),
            status: code.map_or_else(|| "a signal".to_owned(), |code| format!("exit code {code}")),
        })),
    }
}
