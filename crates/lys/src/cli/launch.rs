//! Personal launch arguments; harness arguments remain separate from wrapper options.

/// The signed-in server and an explicitly chosen agent.
#[derive(Debug, clap::Args)]
pub struct LaunchOptions {
    /// The signed-in server's exact API base; otherwise read the sign-in state.
    #[arg(long, value_name = "BASE")]
    pub server: Option<String>,
    /// Your agent's name or id, overriding your recorded harness default.
    #[arg(long, value_name = "NAME")]
    pub agent: Option<String>,
}

/// Launch the agent for a named harness.
#[derive(Debug, clap::Args)]
pub struct HarnessArgs {
    /// Lys's launch selection.
    #[command(flatten)]
    pub options: LaunchOptions,
    /// Arguments for the harness, kept as separate words.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub arguments: Vec<String>,
}

/// Launch the reviewed program of an agent through this device's runner.
#[derive(Debug, clap::Args)]
pub struct RunArgs {
    /// Lys's launch selection; give these options before the command.
    #[command(flatten)]
    pub options: LaunchOptions,
    /// The declared program or harness name to select.
    pub command: String,
    /// Arguments for the program, kept as separate words.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub arguments: Vec<String>,
}

/// Read your agents on this machine or explicitly choose a harness default.
#[derive(Debug, clap::Args)]
pub struct AgentsArgs {
    /// The signed-in server's exact API base; otherwise read the sign-in state.
    #[arg(long, value_name = "BASE")]
    pub server: Option<String>,
    /// Show only this harness or declared program.
    #[arg(long, value_name = "COMMAND")]
    pub harness: Option<String>,
    /// Record your default for this harness on this machine.
    #[arg(long, requires = "harness", value_name = "NAME")]
    pub default: Option<String>,
}

#[cfg(test)]
#[path = "launch_tests.rs"]
mod tests;
