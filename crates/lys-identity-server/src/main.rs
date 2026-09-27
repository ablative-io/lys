//! The directory service binary: read the configuration, open the directory, serve.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use lys_identity::Directory;
use lys_identity::signer::load_service_key;
use lys_identity_server::admission::Admission;
use lys_identity_server::oidc::Oidc;
use lys_identity_server::session::Sessions;
use lys_identity_server::{AppState, Config, ServerError, router};
use lys_log_store::FileLeafStore;

async fn serve(config_path: PathBuf) -> Result<(), ServerError> {
    let config = Config::load(&config_path)?;
    if !config.log_dir.exists() {
        FileLeafStore::create(&config.log_dir, &config.log_origin).map_err(|error| {
            ServerError::ConfigInvalid {
                reason: format!("the log could not be created: {error}"),
            }
        })?;
    }
    let log_dir = config.log_dir.clone();
    let reopen = Box::new(move || FileLeafStore::open(&log_dir));
    let directory = Directory::open(reopen, load_service_key(&config.event_key_file)?)?;
    let state = Arc::new(AppState {
        directory: Mutex::new(directory),
        oidc: Oidc::discover(&config).await?,
        sessions: Sessions::new(config.session_seconds, config.secure_cookie),
        admission: Admission::new(
            config.administrator_binding()?,
            config.link_audit_binding()?,
        ),
    });
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(|error| ServerError::ConfigInvalid {
            reason: format!("{} could not be bound: {error}", config.listen),
        })?;
    axum::serve(listener, router(state))
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
