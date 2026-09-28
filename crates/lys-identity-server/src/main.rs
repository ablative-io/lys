//! The directory service binary: read the configuration, open the directory, serve.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use lys_identity_server::{Config, ServerError, service_saying};

async fn serve(config_path: PathBuf) -> Result<(), ServerError> {
    let config = Config::load(&config_path)?;
    let say = Arc::new(|line: &str| println!("lys-identity-server {line}"));
    let app = service_saying(&config, say).await?;
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(|error| ServerError::ConfigInvalid {
            reason: format!("{} could not be bound: {error}", config.listen),
        })?;
    println!("lys-identity-server listening on {}", config.listen);
    axum::serve(listener, app)
        .await
        .map_err(|error| ServerError::DirectoryUnavailable {
            reason: error.to_string(),
        })
}

#[tokio::main]
async fn main() -> ExitCode {
    let Some(config_path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: lys-identity-server <config.json>");
        return ExitCode::from(2);
    };
    match serve(config_path).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
