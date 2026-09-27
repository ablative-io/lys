//! `lys ca issue --log`: entering an issued certificate in a transparency log.
//!
//! The append is the one step that cannot be undone, so everything that can be
//! checked or written ahead of it is: the flags come together or not at all,
//! the log is opened and its size checked against what an inclusion proof can
//! carry, and the certificate, the issuer certificate and the leaf are staged
//! to disk and flushed. After the append only the renames and the
//! inclusion-proof artifact remain. If any of those fails, the error names the
//! entry that now stands (index, tree size and root) and the exact
//! `lys log prove inclusion` command that recovers the artifact, and nothing
//! is signed again.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::tlog::{MAX_JSON_TREE_SIZE, build_inclusion_artifact};
use lys_core::{Ed25519Identity, TrustError};

use crate::commands::error::{CliError, CliResult};
use crate::commands::files::StagedFile;
use crate::commands::hex::hex_lower;
use crate::commands::key::load_identity;
use crate::commands::log::prove::artifact_json;
use crate::commands::log::store::{self, LogStore};
use crate::commands::output::Emitter;

/// A transparency log an issued certificate is entered in, and where the
/// evidence of that entry is written.
#[derive(Debug)]
pub struct LogEntry<'a> {
    /// An initialized `lys log` directory.
    pub dir: &'a Path,
    /// The log operator's identity key, which signs the proof's checkpoint.
    pub key: &'a Path,
    /// Where the leaf is written: the certificate's DER bytes, exactly.
    pub leaf_out: &'a Path,
    /// Where the `lys/log-inclusion-proof/v1` artifact is written.
    pub artifact_out: &'a Path,
}

impl<'a> LogEntry<'a> {
    /// Reads the four log flags as one entry, or as no log at all.
    ///
    /// # Errors
    ///
    /// [`CliError::LogFlagsIncomplete`] naming the missing flags when some
    /// but not all four are given. A partial set is never read as "no log".
    pub fn from_flags(
        dir: Option<&'a Path>,
        key: Option<&'a Path>,
        leaf_out: Option<&'a Path>,
        artifact_out: Option<&'a Path>,
    ) -> CliResult<Option<Self>> {
        match (dir, key, leaf_out, artifact_out) {
            (None, None, None, None) => Ok(None),
            (Some(dir), Some(key), Some(leaf_out), Some(artifact_out)) => Ok(Some(Self {
                dir,
                key,
                leaf_out,
                artifact_out,
            })),
            _ => {
                let flags = [
                    ("--log", dir.is_none()),
                    ("--log-key", key.is_none()),
                    ("--leaf-out", leaf_out.is_none()),
                    ("--artifact-out", artifact_out.is_none()),
                ];
                let missing: Vec<&str> = flags
                    .iter()
                    .filter(|(_, absent)| *absent)
                    .map(|(flag, _)| *flag)
                    .collect();
                Err(CliError::LogFlagsIncomplete {
                    missing: missing.join(", "),
                })
            }
        }
    }

    /// Opens the log, loads the operator key, and refuses a log whose next
    /// leaf would take the tree to a size an inclusion proof cannot carry.
    /// Nothing is signed or appended here.
    ///
    /// # Errors
    ///
    /// The log directory and key errors of [`store::open`] and
    /// [`load_identity`], and [`TrustError::LogArtifactEncoding`] for a log
    /// with no room for one more provable leaf.
    pub(crate) fn open(&'a self) -> CliResult<OpenedLog<'a>> {
        let log = store::open(self.dir)?;
        let operator = load_identity(self.key)?;
        let size = log.tree().len();
        if size
            .checked_add(1)
            .is_none_or(|next| next >= MAX_JSON_TREE_SIZE)
        {
            return Err(CliError::Trust(TrustError::LogArtifactEncoding {
                reason: format!(
                    "log {} holds {size} leaves; one more would reach {MAX_JSON_TREE_SIZE}, \
                     beyond what an inclusion proof can carry, so nothing was signed or appended",
                    self.dir.display()
                ),
            }));
        }
        Ok(OpenedLog {
            log,
            operator,
            entry: self,
        })
    }
}

/// A log opened and checked for an entry, before anything is appended.
pub(crate) struct OpenedLog<'a> {
    log: LogStore,
    operator: Ed25519Identity,
    entry: &'a LogEntry<'a>,
}

impl<'a> OpenedLog<'a> {
    /// Stages the leaf, appends the certificate's DER, places every staged
    /// output, then builds and places the inclusion-proof artifact.
    ///
    /// # Errors
    ///
    /// Before the append, the staging error of the leaf file. After it,
    /// [`CliError::LoggedButUnwritten`], which names the entry and the command
    /// that recovers the artifact.
    pub(crate) fn enter(mut self, der: &[u8], staged: Vec<StagedFile>) -> CliResult<Entered<'a>> {
        let leaf = StagedFile::stage(self.entry.leaf_out, der, "leaf file")?;
        let (leaf_index, _) = self.log.append(der)?;
        let (root, tree_size) = self.log.tree().root().to_parts();
        let entered = Entered {
            entry: self.entry,
            leaf_index,
            tree_size,
            root,
        };
        match self.finish(der, leaf_index, staged.into_iter().chain([leaf])) {
            Ok(()) => Ok(entered),
            Err(cause) => Err(entered.unwritten(cause)),
        }
    }

    /// Everything after the append: the renames, then the artifact.
    fn finish(
        &self,
        der: &[u8],
        leaf_index: u64,
        staged: impl Iterator<Item = StagedFile>,
    ) -> CliResult<()> {
        for file in staged {
            file.place()?;
        }
        let artifact = build_inclusion_artifact(
            self.log.tree(),
            der,
            self.log.origin(),
            &self.operator,
            leaf_index,
        )?;
        let json = artifact_json(&artifact, "inclusion proof artifact")?;
        StagedFile::stage(
            self.entry.artifact_out,
            json.as_bytes(),
            "inclusion proof artifact",
        )?
        .place()
    }
}

/// What entering a certificate in a log produced.
pub(crate) struct Entered<'a> {
    entry: &'a LogEntry<'a>,
    leaf_index: u64,
    tree_size: u64,
    root: [u8; 32],
}

impl Entered<'_> {
    /// The root in standard base64, as the checkpoint carries it and as
    /// `scripts/verify_inclusion.py` takes it for its expected root.
    fn root_base64(&self) -> String {
        STANDARD.encode(self.root)
    }

    /// The error for a failure after the append: the entry stands, so it is
    /// named in full with the command that recovers its artifact.
    fn unwritten(&self, cause: CliError) -> CliError {
        let entry = self.entry;
        CliError::LoggedButUnwritten {
            log: entry.dir.to_path_buf(),
            leaf_index: self.leaf_index,
            tree_size: self.tree_size,
            root_base64: self.root_base64(),
            recover: format!(
                "lys log prove inclusion --dir {} --key {} --leaf-index {} --out {}",
                shell_word(entry.dir),
                shell_word(entry.key),
                self.leaf_index,
                shell_word(entry.artifact_out)
            ),
            cause: Box::new(cause),
        }
    }

    /// Reports the entry and where its evidence was written.
    pub(crate) fn report(&self, emit: &mut Emitter) {
        let entry = self.entry;
        emit.field("entered in log", "log_dir", entry.dir.display().to_string());
        emit.field("leaf index", "leaf_index", self.leaf_index);
        emit.field("tree size", "tree_size", self.tree_size);
        emit.field("root hash (sha256)", "root_hash", hex_lower(&self.root));
        emit.field("root hash (base64)", "root_base64", self.root_base64());
        emit.field(
            "leaf written",
            "leaf_path",
            entry.leaf_out.display().to_string(),
        );
        emit.field(
            "artifact written",
            "artifact_path",
            entry.artifact_out.display().to_string(),
        );
    }
}

/// A path as one POSIX shell word: verbatim when it holds only characters no
/// shell treats specially, otherwise single-quoted.
fn shell_word(path: &Path) -> String {
    let text = path.display().to_string();
    let plain = !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+=:,@%".contains(c));
    if plain {
        text
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
#[path = "ca_log_tests.rs"]
mod tests;
