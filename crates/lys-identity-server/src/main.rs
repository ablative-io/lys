//! The directory service binary: read the configuration, open the directory, serve.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

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

/// Milliseconds since `since`, for the start's own timing line.
fn ms(since: Instant) -> u128 {
    since.elapsed().as_millis()
}

/// Serve, saying on the listening line how long the start took and where its
/// milliseconds went (Tom, 8 Oct 2026: a start in seconds is a defect, and a
/// start nobody timed cannot be shown to be in milliseconds).
async fn serve(config_path: PathBuf, began: Instant) -> Result<(), ServerError> {
    let config = Config::load(&config_path)?;
    let configured = ms(began);
    let say: lys_identity_server::routes::Say =
        Arc::new(|line: &str| println!("lys-identity-server {line}"));
    // Warn and above reach identity.log through the same sink (DIRECTORY-095).
    lys_identity_server::warn_lines::install(Arc::clone(&say)).map_err(|refused| {
        ServerError::ConfigInvalid {
            reason: refused.to_string(),
        }
    })?;
    let service_began = Instant::now();
    let app = service_saying(&config, say).await?;
    let service = ms(service_began);
    let bind_began = Instant::now();
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(|error| ServerError::ConfigInvalid {
            reason: format!("{} could not be bound: {error}", config.listen),
        })?;
    println!(
        "lys-identity-server listening on {}, ready in {} ms (configuration {configured} ms, service {service} ms, bind {} ms)",
        config.listen,
        ms(began),
        ms(bind_began)
    );
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
    let began = Instant::now();
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
    match serve(config_path, began).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
