//! Argument definitions for `lys agent pair` (AGENTS-020 R4).

use std::path::PathBuf;

/// `lys agent pair`: make this agent's key on this computer, ask Lys for
/// four words to tell a person, wait for their decision, and write the seat
/// file. The printed words are the only thing the person needs to hear.
#[derive(Debug, clap::Args)]
pub struct PairArgs {
    /// Lys's API address: `https://<lys host>/api` from any computer, its
    /// certificate always verified against this system's trust roots; `http://`
    /// only on a loopback address, e.g. `http://127.0.0.1:8490/api`.
    #[arg(long)]
    pub lys: String,

    /// The name the agent proposes for itself; the person may change it.
    #[arg(long)]
    pub name: String,

    /// Where the seat file is written, mode 0600. A path that exists is never
    /// replaced.
    #[arg(long)]
    pub seat: PathBuf,
}
