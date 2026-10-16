#![cfg(test)]
use clap::Parser;
#[test]
fn free_run_arguments_name_the_reviewed_harness_description_and_provisioning_page()
-> Result<(), Box<dyn std::error::Error>> {
    let cli = crate::cli::Cli::try_parse_from(["lys", "run", "agent", "--", "--config", "x"])?;
    let crate::cli::Command::Run(args) = cli.command else {
        return Err("run command was not parsed".into());
    };
    let error = super::arguments(&args.arguments)
        .err()
        .ok_or("free arguments were accepted")?;
    let words = error.to_string();
    assert!(words.contains("launch_arguments_unavailable"), "{words}");
    assert!(words.contains("harness description"), "{words}");
    assert!(words.contains("#/provisioning"), "{words}");
    assert!(words.contains("no session was started"), "{words}");
    Ok(())
}
