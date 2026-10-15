//! The `lys runner` subcommands.

use std::path::PathBuf;

/// The scrollback each session keeps when none is given: one mebibyte.
pub const SCROLLBACK: usize = 1 << 20;

/// `lys runner` subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum RunnerCommand {
    /// Start the runner using this computer's saved connection, without a code.
    Start {
        /// Serve under the login service's readiness supervisor.
        #[arg(long, hide = true, conflicts_with = "restored")]
        worker: bool,
        /// Start the restored build without another update attempt.
        #[arg(long, hide = true)]
        restored: bool,
    },
    /// Connect this computer once and install its native login service.
    Install {
        /// The owner-only pending connection carried by the downloaded installer.
        #[arg(long, value_name = "PATH", conflicts_with_all = ["server", "machine", "server_ca"])]
        pending: Option<PathBuf>,
        /// The server's HTTPS API address; the connection code is read from stdin.
        #[arg(long, requires = "machine")]
        server: Option<String>,
        /// This computer's id from the Network page.
        #[arg(long, requires = "server")]
        machine: Option<String>,
        /// An additional PEM authority for the named server.
        #[arg(long, requires = "server")]
        server_ca: Option<PathBuf>,
    },
    /// End the current reconnect wait and dial Lys immediately.
    Reconnect {
        /// The runner socket; defaults to this user's installed runner.
        #[arg(long)]
        socket: Option<PathBuf>,
    },
    /// Remove the login service while keeping this computer's saved connection.
    Uninstall,
    /// Verify and install the server's release, keeping the previous runner.
    Update,
    /// Enter an isolated managed process session using a startup frame on stdin.
    #[command(hide = true)]
    ManagedEntry,
    /// Hold each agent the server starts in its own background
    /// pseudo-terminal.
    ///
    /// Listens on --socket, a Unix socket made readable and writable by its
    /// owner alone, and on no network address. Acts only on requests signed
    /// by the server key --server-key names. Prints `listening <socket> as
    /// runner <id>` once it answers; the id is what a dialled machine's
    /// record pins. Sessions keep running whether or not any screen is
    /// open. SIGTERM, SIGINT or SIGHUP stops the runner: it ends every
    /// session it holds and waits for each exit. A runner killed outright
    /// is followed by one that ends whatever its sessions left running and
    /// reports each as `ended_by_runner_restart`.
    Serve {
        /// The Unix socket to listen on.
        #[arg(long)]
        socket: PathBuf,

        /// The directory the record of held sessions is kept in.
        #[arg(long)]
        state: PathBuf,

        /// A file holding the server's Ed25519 public key as 64 hexadecimal
        /// characters. Public material only.
        #[arg(long)]
        server_key: PathBuf,

        /// Each session's scrollback, in bytes.
        #[arg(long, default_value_t = SCROLLBACK)]
        scrollback: usize,

        /// The Lys proxy's state directory on this machine: the one given
        /// to `lys proxy serve --state`. A run that goes through the proxy
        /// has its model calls counted from the usage file the proxy keeps
        /// there. Without it such a run is started and is not counted, and
        /// its coverage says so.
        #[arg(long)]
        proxy_state: Option<PathBuf>,
    },

    /// Relay the server's requests to this machine's runner, dialling the
    /// server with the machine's own key; the server never dials a runner
    /// on another machine.
    ///
    /// Ends, by name, when the server cannot be reached; the runner and its
    /// sessions keep running.
    Dial {
        /// This machine's runner's Unix socket.
        #[arg(long)]
        socket: PathBuf,

        /// The server's address: https://host:port and any path prefix.
        /// http:// is spoken only to this machine's loopback address.
        #[arg(long)]
        server: String,

        /// A certificate authority, in PEM, trusted for the server beside
        /// the public roots.
        #[arg(long)]
        server_ca: Option<PathBuf>,

        /// This machine's id, as the server's records name it.
        #[arg(long)]
        machine: String,

        /// The machine's own key file (raw 32-byte Ed25519 seed); its public
        /// half is the key the machine's runner record names.
        #[arg(long)]
        machine_key: PathBuf,
    },
    /// Join this computer to Lys as the runner of a computer Lys names, with
    /// the connection code Lys gave for it, then serve and dial as `lys
    /// runner serve` and `lys runner dial` do, for as long as this command
    /// runs.
    ///
    /// The code is read from standard input, never from the command line,
    /// so it stays out of shell history and the process list. This
    /// computer's own key is made here when it has none, and its private
    /// half never leaves it. The server's address must be one another
    /// computer can reach: https://, and not a loopback address; any other
    /// is refused `runner_join_unreachable` before anything is sent. Files
    /// are kept where `lys identity install` keeps the runner's, under
    /// `LYS_IDENTITY_HOME` when it is set.
    Join {
        /// The server's address: https://host:port and any path prefix, as
        /// Lys gave it.
        #[arg(long)]
        server: String,

        /// This computer's id, as Lys names it.
        #[arg(long)]
        machine: String,

        /// A certificate authority, in PEM, trusted for the server beside
        /// the public roots.
        #[arg(long)]
        server_ca: Option<PathBuf>,

        /// Each session's scrollback, in bytes.
        #[arg(long, default_value_t = SCROLLBACK)]
        scrollback: usize,
    },
    /// Judge one tool call for a session's harness: the `PreToolUse` hook
    /// command. Reads the hook's input on standard input, asks the runner on
    /// --socket, and writes the harness's answer on standard output. Any
    /// failure, from malformed input to a runner that cannot be reached, is
    /// written as a deny with its reason; the command itself exits zero so
    /// the harness reads that deny.
    Judge {
        /// The runner's Unix socket.
        #[arg(long)]
        socket: PathBuf,

        /// The harness whose hook wire is read and written.
        #[arg(long, value_enum)]
        harness: JudgeHarness,
    },
    /// Hand a run's status line to its runner: the command a run counted
    /// through the proxy is given as its harness's status line. Reads the
    /// harness's input on standard input, hands it to the runner on
    /// --socket, and writes one line for the harness to show: that Lys
    /// counts the run, or that its dollars and running time were not
    /// counted and why.
    StatusLine {
        /// The runner's Unix socket.
        #[arg(long)]
        socket: PathBuf,
    },
    /// Hand Claude Code's hook JSON from stdin to the runner. Every outcome
    /// exits zero with no stdout; failures are named on stderr.
    Hook {
        /// The runner's Unix socket.
        #[arg(long)]
        socket: PathBuf,

        /// Override `hook_event_name` for a settings entry that names its event.
        #[arg(long)]
        event: Option<String>,
    },
    /// Hand Codex's appended notification JSON to the runner. Only
    /// agent-turn-complete is sent; every outcome exits zero with no stdout.
    Notify {
        /// The runner's Unix socket.
        #[arg(long)]
        socket: PathBuf,

        /// The notification JSON appended by Codex.
        notification: String,
    },
    /// Serve as the independent owner of one supervised seat (AGENTS-004
    /// R1), started by the runner in a session of its own; its plan is in
    /// its directory.
    #[command(hide = true)]
    SeatOwner {
        /// The owner's directory under the runner's state.
        #[arg(long)]
        dir: PathBuf,
        /// The server's public key, as 64 hexadecimal characters.
        #[arg(long)]
        server_key: PathBuf,
        /// Scrollback for the one session.
        #[arg(long, default_value_t = SCROLLBACK)]
        scrollback: usize,
    },
}

/// The harnesses `lys runner judge` speaks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum JudgeHarness {
    /// Claude Code.
    Claude,
    /// Codex.
    Codex,
}
