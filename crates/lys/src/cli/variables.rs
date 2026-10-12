//! Argument definitions for the variables commands.

use std::path::PathBuf;

use clap::Subcommand;

/// `lys variables`: which map, and what to do with it.
#[derive(Debug, clap::Args)]
pub struct VariablesArgs {
    /// The launch's working folder, where its mcp.json carries the run
    /// pass; the current folder when absent.
    #[arg(long)]
    pub folder: Option<PathBuf>,

    /// The variable map operation to perform.
    #[command(subcommand)]
    pub command: VariablesCommand,
}

/// `lys variables` subcommands.
#[derive(Debug, Subcommand)]
pub enum VariablesCommand {
    /// Read the run's agent map, or with --session its session map: each
    /// variable with its revision, author and expiry, and the scope's
    /// revision to carry on the next set.
    Get {
        /// The session's map instead of the agent's.
        #[arg(long)]
        session: bool,
    },
    /// Patch the map: each key=value sets a key (a JSON value when it
    /// parses as one, else the text), key= alone removes it; the revision
    /// read must be given, and a stale one is refused by name.
    Set {
        /// The session's map instead of the agent's.
        #[arg(long)]
        session: bool,

        /// The scope revision read; 0 for a map never patched.
        #[arg(long)]
        revision: u64,

        /// When the keys set expire, in seconds since the Unix epoch.
        #[arg(long)]
        expires_at: Option<u64>,

        /// The keys, as key=value or key= to remove.
        #[arg(required = true)]
        values: Vec<String>,
    },
}
