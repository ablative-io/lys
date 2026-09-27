//! The contract tests' fake issuer, run on its own for a development
//! directory service that a browser signs in to. It is never a production
//! issuer: it signs every sign-in as the one login it was started with.
//!
//! Usage: `dev_issuer <bind> <issuer-url> <key-file> <subject> <email>`.

use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use identity_contract::fake_issuer::{FakeIssuer, Login};

async fn run(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [bind, issuer, key_file, subject, email] = arguments else {
        return Err("usage: dev_issuer <bind> <issuer-url> <key-file> <subject> <email>".into());
    };
    let bind: SocketAddr = bind.parse()?;
    let login = Login {
        subject: subject.clone(),
        email: email.clone(),
    };
    let started =
        FakeIssuer::start_at(bind, issuer.clone(), &PathBuf::from(key_file), login).await?;
    println!("dev issuer {} listening on {bind}", started.issuer());
    tokio::signal::ctrl_c().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match run(&arguments).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
