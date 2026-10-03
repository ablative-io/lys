//! The `lys proxy` subcommands.

use std::path::PathBuf;

/// `lys proxy` subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum ProxyCommand {
    /// Forward each model call unchanged and record it under its run.
    ///
    /// A call to `http://<listen>/anthropic/...` goes to the Anthropic
    /// upstream, one to `/openai/...` to the OpenAI upstream, with its
    /// method, path, query, headers and body as sent. Each call is recorded
    /// as one `lys.call` entry in --home, under the session its request body
    /// names (`metadata.user_id`) or the day's `unlinked` session. The
    /// Anthropic upstream is the login's own `ANTHROPIC_BASE_URL` when it has
    /// one, so a machine pointed at a gateway keeps it. Prints one JSON line
    /// naming where it listens and each upstream once it answers, then one
    /// JSON report line per call; the calls a run before it left open are
    /// recorded `lost` and reported first. SIGTERM or SIGINT stops it.
    Serve {
        /// The loopback address to listen on.
        #[arg(long)]
        listen: String,

        /// The home the calls are recorded in.
        #[arg(long)]
        home: PathBuf,

        /// Where the open-call journal and the capture spool live.
        #[arg(long)]
        state: PathBuf,

        /// Where an `/anthropic` path is forwarded. Absent, the login's own
        /// `ANTHROPIC_BASE_URL` when it has one, and Anthropic's API when not.
        #[arg(long)]
        anthropic: Option<String>,

        /// Where an `/openai` path is forwarded.
        #[arg(long, default_value = "https://api.openai.com")]
        openai: String,
    },
}
