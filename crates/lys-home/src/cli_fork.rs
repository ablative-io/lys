//! `lys-home fork --home <dir> --lantern <id> [--session <id>]` (HOME-006
//! R5): fork a child session from a lantern's point and print one JSON
//! report, `{"command": "fork", "report": …}`, of ids and counts. `--session`
//! names the session to cut from, as the resolve reads it: the lit-in
//! session of a lantern whose data records one, or one holder of an older
//! record. The home is taken through [`Home::read`], which creates nothing,
//! so a mistyped path never makes a home. A refusal is printed on stderr by
//! the binary and exits 1 with nothing on stdout; a missing or unknown
//! argument is refused by clap with exit code 2. No point or entry id is
//! taken, the child is neither rendered nor launched here, and nothing of
//! the transcript is printed.

use std::path::PathBuf;

use clap::Args;
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::record::Home;
use crate::record::fork::fork;

/// The arguments of `fork`.
#[derive(Clone, Debug, PartialEq, Eq, Args)]
pub struct ForkArgs {
    /// The home directory.
    #[arg(long)]
    pub home: PathBuf,
    /// The lantern's entry id, as a light report printed it.
    #[arg(long)]
    pub lantern: String,
    /// The session to cut from, when the lantern is an older record several sessions hold.
    #[arg(long)]
    pub session: Option<String>,
}

/// Run `fork` and return its report.
pub fn run(args: &ForkArgs) -> Result<Value, HomeError> {
    let home = Home::read(&args.home)?;
    let report = fork(&home, &args.lantern, args.session.as_deref())?;
    let report = serde_json::to_value(&report).map_err(|source| HomeError::Json {
        context: "the fork report could not be serialised",
        source,
    })?;
    Ok(json!({"command": "fork", "report": report}))
}
