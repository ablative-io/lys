//! The `lys-home` binary.

use clap::Parser;

fn main() {
    let cli = lys_home::cli::Cli::parse();
    let refused = cli.command.refusal_status();
    match lys_home::cli::run_with_status(cli) {
        Ok(outcome) => {
            println!(
                "{}",
                serde_json::to_string(&outcome.report).unwrap_or_else(|_| String::from("{}"))
            );
            if outcome.status != 0 {
                std::process::exit(outcome.status);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(refused);
        }
    }
}
