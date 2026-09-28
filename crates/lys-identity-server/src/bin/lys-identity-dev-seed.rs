//! Development only: fill a disposable directory with two test people and
//! their agents, so the identity screens have real records to show.
//!
//! Usage: `lys-identity-dev-seed <config.json> <first-subject> <second-subject>`
//!
//! The configuration is the service's own. The seed writes to the log it
//! names, through the directory's typed API, and binds each test person to
//! the given subject at the configured issuer, so signing in as that subject
//! shows that person's records. It refuses a directory that already holds any
//! identity. Stop the service before seeding, and start it again after, since
//! the service reads the log when it opens. Never point it at a directory
//! whose records matter: every seeded event names the configured administrator
//! as its actor without a sign-in.

use std::path::PathBuf;
use std::process::ExitCode;

use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::{Config, ServerError};

fn run(config_path: &std::path::Path, first: &str, second: &str) -> Result<(), ServerError> {
    let config = Config::load(config_path)?;
    let seeded = seed_configured(&config, [first, second])?;
    for person in &seeded.people {
        println!(
            "{} {} signs in as {}",
            person.id, person.display_name, person.subject
        );
        for agent in &person.agents {
            println!("  {} {} {}", agent.id, agent.display_name, agent.state);
        }
    }
    println!("the log holds {} events", seeded.tree_size);
    Ok(())
}

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(config), Some(first), Some(second), None) =
        (args.next(), args.next(), args.next(), args.next())
    else {
        eprintln!("usage: lys-identity-dev-seed <config.json> <first-subject> <second-subject>");
        return ExitCode::from(2);
    };
    let (Some(first), Some(second)) = (first.to_str(), second.to_str()) else {
        eprintln!("the subjects are not UTF-8");
        return ExitCode::from(2);
    };
    match run(&PathBuf::from(config), first, second) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
