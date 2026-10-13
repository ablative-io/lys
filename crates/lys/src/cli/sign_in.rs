//! Native person sign-in arguments.

/// Sign in to the server from this person's terminal.
#[derive(Debug, clap::Args)]
pub struct SignInArgs {
    /// The server's HTTPS base, including /api; HTTP is allowed on loopback only.
    #[arg(long, value_name = "BASE")]
    pub server: String,
}
