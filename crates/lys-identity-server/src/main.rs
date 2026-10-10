//! The directory service binary: read the configuration, open the directory, serve.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use lys_identity_server::read_api::BUILD;
use lys_identity_server::{Config, ServerError, service_saying};

/// What `--version` and `-V` print: the name, the crate version and the
/// commit this binary was built from.
const VERSION: &str = concat!(
    "lys-identity-server ",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("LYS_BUILD"),
    ")"
);

async fn serve(config_path: PathBuf) -> Result<(), ServerError> {
    let config = Config::load(&config_path)?;
    let say: lys_identity_server::routes::Say =
        Arc::new(|line: &str| println!("lys-identity-server {line}"));
    // Warn and above reach identity.log through the same sink (DIRECTORY-095).
    lys_identity_server::warn_lines::install(Arc::clone(&say)).map_err(|refused| {
        ServerError::ConfigInvalid {
            reason: refused.to_string(),
        }
    })?;
    let app = service_saying(&config, say).await?;
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(|error| ServerError::ConfigInvalid {
            reason: format!("{} could not be bound: {error}", config.listen),
        })?;
    println!("lys-identity-server listening on {}", config.listen);
    // The connection's address is the person's own, carried to the issuer
    // so its failed sign-in count is per person.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .map_err(|error| ServerError::DirectoryUnavailable {
        reason: error.to_string(),
    })
}

#[tokio::main]
async fn main() -> ExitCode {
    let Some(first) = std::env::args_os().nth(1) else {
        eprintln!("usage: lys-identity-server <config.json> | --version");
        return ExitCode::from(2);
    };
    if first == "--version" || first == "-V" {
        println!("{VERSION}");
        return ExitCode::SUCCESS;
    }
    let config_path = PathBuf::from(first);
    println!("lys-identity-server build {BUILD}");
    match serve(config_path).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
