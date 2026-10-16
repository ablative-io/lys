//! Native person sign-in arguments.

/// Sign in to the server from this person's terminal.
#[derive(Debug, clap::Args)]
pub struct SignInArgs {
    /// The server's HTTPS base, including /api; HTTP is allowed on loopback only.
    #[arg(
        long,
        value_name = "BASE",
        required_unless_present = "finish",
        conflicts_with = "finish"
    )]
    pub server: Option<String>,
    /// Hand the finishing address to the sign-in waiting on this computer.
    #[arg(long, value_name = "ADDRESS")]
    pub finish: Option<String>,
}
