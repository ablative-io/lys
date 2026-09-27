//! Arguments several commands share: where permission comes from, which
//! relation a grant names, and a sealed record's class.

use std::path::PathBuf;

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
}

impl PermissionSource {
    pub fn grants(&self, layout: &Layout) -> Result<Grants, SecretsError> {
        Ok(match &self.directory_config {
            Some(config) => Grants::Directory(SpiceGrants::from_directory(
                config,
                &self.use_action,
                &self.read_action,
                &self.lend_action,
            )?),
            None => Grants::File(FileGrants::new(layout.grants())),
        })
    }
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum RelationArg {
    Use,
    Read,
    Lend,
}

impl From<RelationArg> for Relation {
    fn from(relation: RelationArg) -> Self {
        match relation {
            RelationArg::Use => Self::Use,
            RelationArg::Read => Self::Read,
            RelationArg::Lend => Self::Lend,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum RecordClass {
    Memory,
    Key,
}
