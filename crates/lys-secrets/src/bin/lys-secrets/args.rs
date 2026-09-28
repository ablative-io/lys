//! Arguments several commands share: where permission comes from, which
//! relation a grant names, a sealed record's class, and a page of the audit
//! log.

use std::num::NonZeroU64;
use std::path::PathBuf;
use std::sync::Arc;

use lys_secrets::{Relation, SecretsError};

use crate::files::{FileGrants, Layout};
use crate::spice::{Grants, SpiceGrants};

/// Where permission comes from: the broker's grants file, or the Lys
/// directory's grants when its configuration is named.
#[derive(clap::Args)]
pub struct PermissionSource {
    /// The Lys directory's configuration; when named, every use and read is
    /// judged by the directory's grants on `secret/<name>`.
    #[arg(long)]
    directory_config: Option<PathBuf>,
    /// The action the directory must give on the secret for a use.
    #[arg(long, default_value = "edit")]
    use_action: String,
    /// The action the directory must give on a sealed record for a read.
    #[arg(long, default_value = "view")]
    read_action: String,
    /// The action the directory must give on a secret for lending it on.
    #[arg(long, default_value = "grant")]
    lend_action: String,
    /// The action the directory must give on a scope (`person/<id>`,
    /// `team/<name>` or `organisation/<name>`) for standing inside it.
    #[arg(long, default_value = "view")]
    member_action: String,
}

impl PermissionSource {
    pub fn grants(&self, layout: &Layout) -> Result<Grants, SecretsError> {
        Ok(match &self.directory_config {
            Some(config) => Grants::Directory(Arc::new(SpiceGrants::from_directory(
                config,
                &self.use_action,
                &self.read_action,
                [&self.lend_action, &self.member_action],
            )?)),
            None => Grants::File(FileGrants::new(layout.grants())),
        })
    }
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum RelationArg {
    Use,
    Read,
    Lend,
    Member,
}

impl From<RelationArg> for Relation {
    fn from(relation: RelationArg) -> Self {
        match relation {
            RelationArg::Use => Self::Use,
            RelationArg::Read => Self::Read,
            RelationArg::Lend => Self::Lend,
            RelationArg::Member => Self::Member,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum RecordClass {
    Memory,
    Key,
}

/// A page of the audit log: the last `--most` lines, ending at the tail or
/// before `--before`. Only those lines are read.
#[derive(clap::Args)]
pub struct Page {
    /// How many lines to print (required; there is no default). The page
    /// ends at the log's last line, or before `--before`.
    #[arg(long)]
    pub most: NonZeroU64,
    /// The index to end before, as the previous page printed it; without
    /// it the page ends at the log's last line.
    #[arg(long)]
    pub before: Option<u64>,
}
