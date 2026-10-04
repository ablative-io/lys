//! The `lys proxy` subcommands.

use std::path::PathBuf;

/// `lys proxy` subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum ProxyCommand {
    /// Forward each model call unchanged and record it under its run.
    ///
    /// A call to `http://<listen>/anthropic/...` goes to the Anthropic
    /// upstream, one to `/openai/...` to the `--openai` upstream, with its
    /// method, path, query, headers and body as sent. Each call is recorded
    /// as one `lys.call` entry in --home, under the session its request body
    /// names (`metadata.user_id`) or the day's `unlinked` session. The
    /// Anthropic upstream is --anthropic, else the one --upstream records
    /// (the install writes the login's own `ANTHROPIC_BASE_URL` there), else
    /// Anthropic's API; the proxy's environment is never read for it. Prints one JSON line
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

        /// Where an `/anthropic` path is forwarded, over any recorded one.
        #[arg(long)]
        anthropic: Option<String>,

        /// A file recording the Anthropic upstream and where it came from,
        /// as the install writes it.
        #[arg(long)]
        upstream: Option<PathBuf>,

        /// Where an `/openai` path is forwarded.
        #[arg(long, default_value = "https://api.openai.com")]
        openai: String,

        /// Where an `/openai` call is forwarded when it carries a chatgpt.com
        /// account, as a Codex signed in through chatgpt.com sends.
        #[arg(long, default_value = "https://chatgpt.com/backend-api/codex")]
        chatgpt: String,
    },
}
