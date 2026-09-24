//! The `lys-home` binary.

use clap::Parser;

fn main() {
    let cli = lys_home::cli::Cli::parse();
    match lys_home::cli::run(cli) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string(&report).unwrap_or_else(|_| String::from("{}"))
            );
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
