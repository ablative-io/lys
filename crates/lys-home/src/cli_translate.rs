//! `lys-home translate-codex --home <dir> --session <id> --out <dir>
//! --codex-version <version> [--zone <iana name>]` (HOME-009 R7): translate
//! a session of the home into a Codex rollout under `--out`, a Codex home
//! directory, with its loss account beside it, and print one JSON report,
//! `{"command": "translate-codex", "report": …}`, of the two paths and the
//! entry, block, kept, changed and lost counts.
//!
//! `--zone` is read from `TZ` by clap when the flag is absent, so the
//! library takes the zone as an explicit input and never reads the
//! environment itself. `--codex-version` has no default. The home is taken
//! through [`Home::read`], which creates nothing. The version and zone are
//! checked before the session is opened, so a refusal touches neither the
//! home nor `--out`; every refusal is printed on stderr by the binary and
//! exits 1 with nothing on stdout. Nothing of the transcript is printed and
//! Codex is never run (ADR-007).

use std::path::PathBuf;

use clap::Args;
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::codex::rollout::translate;
use crate::harness::codex::zone::{check_version, resolve_zone};
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
    /// The Codex home directory to write the rollout under.
    #[arg(long)]
    pub out: PathBuf,
    /// The Codex version the rollout is rendered for; only a measured one is accepted.
    #[arg(long = "codex-version")]
    pub codex_version: String,
    /// The IANA time zone the rollout's file name is written in; read from TZ when absent.
    #[arg(long, env = "TZ")]
    pub zone: Option<String>,
}

/// Run `translate-codex` and return its report.
pub fn run(args: &TranslateArgs) -> Result<Value, HomeError> {
    check_version(&args.codex_version)?;
    resolve_zone(args.zone.as_deref())?;
    let home = Home::read(&args.home)?;
    let mut session = home.open_session(&args.session)?;
    let done = translate(
        &mut session,
        &args.out,
        &args.codex_version,
        args.zone.as_deref(),
    )?;
    Ok(json!({
        "command": "translate-codex",
        "report": {
            "rollout": done.rollout,
            "account": done.account,
            "entries": done.entries,
            "blocks": done.blocks,
            "kept": done.kept,
            "changed": done.changed,
            "lost": done.lost,
        },
    }))
}
