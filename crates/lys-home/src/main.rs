//! The `lys-home` binary.

use clap::{CommandFactory, FromArgMatches};
use lys_home::cli::Cli;

/// What `--version` prints after the name: the crate version and the commit
/// the binary was built from, stamped by `build.rs`.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("LYS_BUILD"), ")");

/// The arguments, with `--version` answering [`VERSION`].
fn parse() -> Cli {
    let matches = Cli::command().version(VERSION).get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
}

fn main() {
    let cli = parse();
    let refused = cli.command.refusal_status();
    match lys_home::cli::run_with_status(cli) {
        Ok(outcome) => match serde_json::to_string(&outcome.report) {
            Ok(line) => {
                println!("{line}");
                if outcome.status != 0 {
                    std::process::exit(outcome.status);
                }
            }
            Err(source) => {
                let error = lys_home::HomeError::Json {
                    context: "the report could not be serialised",
                    source,
                };
                eprintln!("{error}");
                std::process::exit(refused);
            }
        },
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(refused);
        }
    }
}
