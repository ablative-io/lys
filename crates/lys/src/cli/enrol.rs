//! Argument definitions for `lys agent enrol` (AGENTS-017 R1, R5).

use std::path::PathBuf;

/// `lys agent enrol`: make a new agent's key on this computer, have Lys
/// certify it, and write its seat file.
#[derive(Debug, clap::Args)]
pub struct EnrolArgs {
    /// Lys's API address: `https://<lys host>/api` from any computer, its
    /// certificate always verified against this system's trust roots; `http://`
    /// only on a loopback address, e.g. `http://127.0.0.1:8490/api`.
    #[arg(long)]
    pub url: String,

    /// The agent whose key is made and certified (`agent-…`).
    #[arg(long)]
    pub agent: String,

    /// The registering agent's seat file or PKCS#8 PEM: the request is signed
    /// as that agent, for the first certificate of an agent it registered.
    #[arg(long = "as", value_name = "SEAT_FILE", conflicts_with = "pass")]
    pub as_file: Option<PathBuf>,

    /// The registering agent's id, when the `--as` file does not name it.
    #[arg(long, value_name = "ID", requires = "as_file")]
    pub as_agent: Option<String>,

    /// The responsible person's app pass, read from this file: the request is
    /// the person's, for any certificate of their agent.
    #[arg(long, value_name = "PASS_FILE")]
    pub pass: Option<PathBuf>,

    /// Where the seat file is written, mode 0600: the path the agent's reviewed
    /// profile names as its seat file. A path that exists is never replaced.
    #[arg(long)]
    pub out: PathBuf,

    /// The app's door the seat answers at; with --handle and --device the
    /// seat file carries the door's members, so a door seat's key is made on
    /// the computer it runs on.
    #[arg(long, value_name = "DOOR_URL", requires_all = ["handle", "device"])]
    pub door: Option<String>,

    /// The registry handle the door seat answers as.
    #[arg(long, requires = "door")]
    pub handle: Option<String>,

    /// The device label the door seat's session carries.
    #[arg(long, requires = "door")]
    pub device: Option<String>,
}
