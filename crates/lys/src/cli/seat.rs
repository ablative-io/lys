//! Argument definitions for the seat commands.

use std::path::PathBuf;

use clap::Subcommand;

use super::SeatImportArgs;

/// `lys seat`: what to do, and the server it is asked of.
#[derive(Debug, clap::Args)]
pub struct SeatArgs {
    /// The identity server's base address, `http://<loopback>:<port>` with
    /// `/api` after it when the server serves its screens, over the
    /// installed service's own. The operator token is still the install's,
    /// and is sent only to a numeric loopback address.
    #[arg(long, global = true, value_name = "BASE")]
    pub server: Option<String>,

    /// What to do.
    #[command(subcommand)]
    pub command: SeatCommand,
}

/// `lys seat` subcommands. Each prints human lines, or one JSON object
/// with --json; a refusal is shown by the server's name and words.
#[derive(Debug, Subcommand)]
pub enum SeatCommand {
    /// Add a seat: a name, the agent it runs as, a reviewed profile
    /// version, and the machine whose runner holds it.
    Add {
        /// The seat's name: lowercase letters, digits and '-', 1 to 64,
        /// unique in the install.
        name: String,

        /// The agent identity the seat runs as.
        #[arg(long)]
        agent: String,

        /// The reviewed version of the agent's profile the seat starts
        /// with.
        #[arg(long)]
        profile_version: u64,

        /// The machine whose runner holds the seat.
        #[arg(long)]
        machine: String,

        /// The folder the seat works in, on that machine.
        #[arg(long)]
        working_folder: Option<String>,
    },

    /// List every seat with its state and last signal, and whether the
    /// runner could be read; a runner that could not is named with its
    /// reason.
    List,

    /// Start a seat's session through the server's own start path with
    /// the reviewed profile's transport.
    Start {
        /// The seat.
        name: String,
    },

    /// Stop a seat's session: the harness is asked to end when idle, and
    /// a turn in progress is refused `seat_turn_in_progress` unless
    /// --force.
    Stop {
        /// The seat.
        name: String,

        /// End the session even while a turn is in progress.
        #[arg(long)]
        force: bool,
    },

    /// Stop a seat's session, then start it again.
    Restart {
        /// The seat.
        name: String,

        /// End the session even while a turn is in progress.
        #[arg(long)]
        force: bool,
    },

    /// Send a message to a running seat as a user turn, never as typed
    /// keys. Text that is empty or carries control characters is refused
    /// by name.
    Send {
        /// The seat.
        name: String,

        /// The message; its words are joined with single spaces.
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        text: Vec<String>,
    },

    /// Import one seat's existing configuration by a confirmed,
    /// restartable plan: a dry run that changes nothing, then the person's
    /// confirmation of that exact plan. Importing never starts, stops or
    /// restarts a seat.
    Import(SeatImportArgs),
}

/// `lys attach`: the seat or session to show, and how.
#[derive(Debug, clap::Args)]
pub struct AttachArgs {
    /// The seat whose session to show.
    #[arg(required_unless_present = "session", conflicts_with = "session")]
    pub seat: Option<String>,

    /// Send each line typed on standard input to the seat as a user turn,
    /// through the same path as `lys seat send`, never as keys. An empty
    /// line sends nothing.
    #[arg(long = "type", conflicts_with = "session")]
    pub typing: bool,

    /// Show this session, one not started managed, read-only as the exact
    /// bytes of its pseudo-terminal.
    #[arg(long, value_name = "ID")]
    pub session: Option<String>,

    /// The identity server's base address, `http://<loopback>:<port>` with
    /// `/api` after it when the server serves its screens, over the
    /// installed service's own. The operator token is still the install's,
    /// and is sent only to a numeric loopback address.
    #[arg(long, value_name = "BASE")]
    pub server: Option<String>,
}

/// `lys seat import` subcommands, each of one seat.
#[derive(Debug, Subcommand)]
pub enum SeatImportCommand {
    /// Read the seat's declared sources at one instant and show the whole
    /// plan: its sources and revisions, every record it writes, credential
    /// references, replacements, schedule counts, exclusions,
    /// prerequisites and refusals. Nothing is sent, started or changed.
    DryRun {
        /// The seat.
        name: String,

        /// The JSON manifest naming the seat's sources.
        #[arg(long, value_name = "PATH")]
        manifest: PathBuf,
    },

    /// Confirm the exact plan a dry run showed, as the person responsible,
    /// and apply it under one operation that resumes after a stop.
    Confirm {
        /// The seat.
        name: String,

        /// The plan id the dry run showed.
        #[arg(long, value_name = "ID")]
        plan: String,

        /// The plan revision the dry run showed.
        #[arg(long, value_name = "REVISION")]
        revision: String,
    },

    /// Show every import of the seat and the one selected.
    Status {
        /// The seat.
        name: String,
    },
}
