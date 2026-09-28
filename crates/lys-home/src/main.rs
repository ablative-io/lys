//! The `lys-home` binary.

use clap::Parser;

fn main() {
    let cli = lys_home::cli::Cli::parse();
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
