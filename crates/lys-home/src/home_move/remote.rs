//! The one rule for a remote (HOME-019 R3): in this stage a remote is a
//! path on this machine and nothing else.
//!
//! A remote holding `://` (an ssh, https, git or file URL, or any other
//! scheme) or a `:` before its first `/` (git's scp-style `host:path`) is
//! refused as `remote_not_local` before any process starts and before any
//! file is written: moving a home off this machine waits for stage 3's
//! encryption from the secrets step. No host list, estate file or ssh
//! configuration is read. Anything else is a path, joined to the base
//! directory it is given (ship and fetch give the process's current
//! directory) and made absolute, so git is always handed a path that
//! begins with `/` and can never read it as a host.

use std::path::{Path, PathBuf};

use crate::error::HomeError;

/// Take a remote argument as an absolute path on this machine, resolving a
/// relative one against `base`; anything else refuses as
/// `remote_not_local`.
pub fn take_remote(remote: &str, base: &Path) -> Result<PathBuf, HomeError> {
    let before_slash = remote.split('/').next().unwrap_or(remote);
    if remote.contains("://") || before_slash.contains(':') {
        return Err(HomeError::RemoteNotLocal {
            remote: remote.to_owned(),
        });
    }
    let joined = base.join(remote);
    std::path::absolute(&joined).map_err(|e| HomeError::io("resolving the remote", &joined, e))
}
