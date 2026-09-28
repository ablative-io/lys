//! `lys-home translate-codex --home <dir> --session <id> --out <dir>
//! --codex-version <version> [--zone <iana name>]` (HOME-009 R7): translate a
//! session of the home into a Codex rollout under `--out`, a Codex home
//! directory, with its loss account beside it, and print one JSON report,
//! `{"command": "translate-codex", "report": …}`, of the two paths and the
//! counts. `--codex-version` is required and has no default; `--zone` is
//! read from `TZ` when the flag is absent, so the library takes the zone as
//! an explicit input. The home is taken through [`Home::read`], which
//! creates nothing. A refusal is printed on stderr by the binary and exits 1
//! with nothing on stdout, and nothing is written. Codex is never run, and
//! no text, argument, output, summary or seed is printed.

use std::path::PathBuf;

use clap::Args;
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::codex::rollout::translate;
use crate::record::Home;

/// The arguments of `translate-codex`.
#[derive(Clone, Debug, PartialEq, Eq, Args)]
pub struct TranslateArgs {
    /// The home directory.
    #[arg(long)]
    pub home: PathBuf,
    /// The session to translate.
    #[arg(long)]
    pub session: String,
    /// The Codex home directory the rollout is written under.
    #[arg(long)]
    pub out: PathBuf,
    /// The Codex version whose rollout shape to write; only 0.156.0 is measured.
    #[arg(long = "codex-version")]
    pub codex_version: String,
    /// The IANA time zone the rollout's file name is written in; TZ when absent.
    #[arg(long, env = "TZ")]
    pub zone: Option<String>,
}

/// Run `translate-codex` and return its report.
pub fn run(args: &TranslateArgs) -> Result<Value, HomeError> {
    let home = Home::read(&args.home)?;
    let mut session = home.open_session(&args.session)?;
    let done = translate(
        &mut session,
        &args.out,
        &args.codex_version,
        args.zone.as_deref(),
    )?;
    Ok(json!({"command": "translate-codex", "report": {
        "rollout": done.rollout,
        "account": done.account,
        "entries": done.entries,
        "blocks": done.blocks,
        "kept": done.kept,
        "changed": done.changed,
        "lost": done.lost,
    }}))
}
