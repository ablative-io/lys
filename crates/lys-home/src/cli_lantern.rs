//! The `lys-home lantern` subcommands (HOME-004 R6): `light`, `epilogue`
//! and `recall`, each printing one JSON report. `light` prints the lantern's
//! id, session, point and time, never the note; `epilogue` prints the
//! epilogue's id, lantern, session, author, time and ordinal, never the
//! words; `recall` prints the rows and the sessions skipped, and is the one
//! command that prints a note or an epilogue (ADR-015). `--by` is a required
//! argument, a self-declared name as `canon add`'s is, never read from the
//! environment. A refusal is printed on stderr by the binary and exits 1;
//! a missing or conflicting argument is refused by clap with exit code 2.

use std::path::PathBuf;

use clap::{ArgGroup, Args, Subcommand};
use serde::Serialize;
use serde_json::Value;

use crate::error::HomeError;
use crate::record::Home;
use crate::record::epilogue::add_epilogue;
use crate::record::lantern::light;
use crate::record::recall::{recall_by_note, recall_by_point};

/// The lantern commands.
#[derive(Debug, Subcommand)]
pub enum LanternAction {
    /// Light a lantern at an entry of a session with a note.
    Light {
        /// The home directory.
        #[arg(long)]
        home: PathBuf,
        /// The session the entry is in.
        #[arg(long)]
        session: String,
        /// The entry id of the point to mark.
        #[arg(long)]
        point: String,
        /// The note, stored as written.
        #[arg(long)]
        note: String,
        /// Who is lighting it, as they name themselves.
        #[arg(long)]
        by: String,
    },
    /// Add further words to a lantern of a session.
    Epilogue {
        /// The home directory.
        #[arg(long)]
        home: PathBuf,
        /// The session the lantern is in.
        #[arg(long)]
        session: String,
        /// The lantern's entry id.
        #[arg(long)]
        lantern: String,
        /// The further words, stored as written.
        #[arg(long)]
        words: String,
        /// Who is adding them, as they name themselves.
        #[arg(long)]
        by: String,
    },
    /// List lanterns by a phrase of their note or epilogues, or by the entry they mark.
    Recall(RecallArgs),
}

/// The arguments of `lantern recall`: exactly one of `--note` or the pair
/// `--session` and `--point`.
#[derive(Clone, Debug, PartialEq, Eq, Args)]
#[command(group = ArgGroup::new("how").required(true).args(["note", "session"]))]
pub struct RecallArgs {
    /// The home directory.
    #[arg(long)]
    pub home: PathBuf,
    /// The words to find as one phrase, case-folded, in a note or an epilogue.
    #[arg(long, conflicts_with_all = ["session", "point"])]
    pub note: Option<String>,
    /// The session whose entry is marked.
    #[arg(long, requires = "point")]
    pub session: Option<String>,
    /// The entry id marked.
    #[arg(long, requires = "session")]
    pub point: Option<String>,
}

fn report<T: Serialize>(value: &T) -> Result<Value, HomeError> {
    serde_json::to_value(value).map_err(|source| HomeError::Json {
        context: "the lantern report could not be serialised",
        source,
    })
}

/// Run a lantern command and return its report.
pub fn run(action: LanternAction) -> Result<Value, HomeError> {
    match action {
        LanternAction::Light {
            home,
            session,
            point,
            note,
            by,
        } => {
            let home = Home::open(home)?;
            report(&light(&home, &session, &point, &note, &by)?)
        }
        LanternAction::Epilogue {
            home,
            session,
            lantern,
            words,
            by,
        } => {
            let home = Home::open(home)?;
            report(&add_epilogue(&home, &session, &lantern, &words, &by)?)
        }
        LanternAction::Recall(args) => {
            let home = Home::open(args.home)?;
            let found = match (args.note, args.session, args.point) {
                (Some(words), _, _) => recall_by_note(&home, &words)?,
                (None, Some(session), Some(point)) => recall_by_point(&home, &session, &point)?,
                (None, _, _) => {
                    return Err(HomeError::BodyShape {
                        api: "lantern recall",
                        reason: "recall is by --note, or by --session with --point",
                    });
                }
            };
            report(&found)
        }
    }
}
