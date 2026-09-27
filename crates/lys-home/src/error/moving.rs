//! What ship and fetch refuse (HOME-019).

use std::path::PathBuf;

/// A refusal of ship or fetch.
#[derive(Debug, thiserror::Error)]
pub enum MoveError {
    /// A remote that is not a path on this machine: a URL of any scheme or
    /// an scp-style `host:path`.
    #[error(
        "remote_not_local: `{remote}` is not a path on this machine; shipping off this machine waits for stage 3's encryption from the secrets step"
    )]
    RemoteNotLocal {
        /// The remote as given.
        remote: String,
    },

    /// A git command exited non-zero. None of its output is carried.
    #[error("git_failed: git {subcommand} ended with {status}")]
    GitFailed {
        /// The git subcommand.
        subcommand: String,
        /// `exit code <n>`, or `a signal` when it had none.
        status: String,
    },

    /// Ship was asked to ship a home that lists no session.
    #[error(
        "empty_home: the home at {} holds no sessions, so there is nothing a fetch could arrive with; ship after a session has been captured", path.display()
    )]
    EmptyHome {
        /// The home directory.
        path: PathBuf,
    },

    /// The remote path exists and is not a bare repository.
    #[error(
        "remote_not_bare: {} is {found}; ship pushes only to a bare repository, and makes one where the path does not exist", path.display()
    )]
    RemoteNotBare {
        /// The remote path.
        path: PathBuf,
        /// What was found there.
        found: &'static str,
    },

    /// A session ship would snapshot is held by a live owner.
    #[error(
        "session_held: session `{session}` is held by a live owner; ship after that seat stops"
    )]
    HeldByOwner {
        /// The session id.
        session: String,
    },

    /// A session has no index file beside it.
    #[error(
        "index_missing: session `{session}` has no sessions/{session}.index.jsonl; run lys-home given --home {} --session {session}, which writes that session's index and head, then ship again", home.display()
    )]
    IndexMissing {
        /// The session id.
        session: String,
        /// The home directory.
        home: PathBuf,
    },

    /// A session has no head file beside it.
    #[error(
        "head_missing: session `{session}` has no sessions/{session}.head; run lys-home given --home {} --session {session}, which writes that session's index and head, then ship again", home.display()
    )]
    HeadMissing {
        /// The session id.
        session: String,
        /// The home directory.
        home: PathBuf,
    },

    /// The home's repository tracks paths outside the tracked set.
    #[error(
        "foreign_tracked: the home's repository tracks {} outside the tracked set; a shipped home carries only its sessions, indexes, heads, blocks and templates", paths.join(", ")
    )]
    ForeignTracked {
        /// The paths, in ascending byte order.
        paths: Vec<String>,
    },

    /// The remote's ref names a commit the commit to push does not descend from.
    #[error(
        "ref_diverged: the remote's {git_ref} is at {remote_commit}, and {commit} does not descend from it; ship never overwrites a ref and pushed nothing"
    )]
    RefDiverged {
        /// The ref.
        git_ref: String,
        /// The commit the remote's ref names.
        remote_commit: String,
        /// The commit ship would push.
        commit: String,
    },

    /// A fetch target already holds a home.
    #[error(
        "target_holds_home: {} already holds sessions/; fetch arrives only in a new, empty home", path.display()
    )]
    TargetHoldsHome {
        /// The target directory.
        path: PathBuf,
    },

    /// A fetch target exists and is not an empty directory.
    #[error(
        "target_not_empty: {} is not an empty directory; fetch arrives only in a new, empty home", path.display()
    )]
    TargetNotEmpty {
        /// The target directory.
        path: PathBuf,
    },

    /// Appending an arrival or committing the arrivals failed; everything
    /// fetch created was removed.
    #[error(
        "arrival_failed: {step} failed, so no arrival was recorded and everything fetch created was removed"
    )]
    ArrivalFailed {
        /// The step: `append to session <id>` or the git subcommand.
        step: String,
    },

    /// A fetch refused, and removing what it created failed too.
    #[error(
        "fetch into {} failed ({reason}), and removing what it created failed too ({cleanup}); remove the target by hand before fetching again", path.display()
    )]
    FetchHalfWritten {
        /// The target directory.
        path: PathBuf,
        /// The refusal that stopped the fetch, as displayed.
        reason: String,
        /// What the cleanup met.
        cleanup: String,
    },
}
