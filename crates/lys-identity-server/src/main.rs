//! The directory service binary: read the configuration, open the directory, serve.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

mod planned_stop;

use lys_identity_server::read_api::BUILD;
use lys_identity_server::{Config, ServerError, started_saying_with_clock};

/// What `--version` and `-V` print: the name, the crate version and the
/// commit this binary was built from.
const VERSION: &str = concat!(
    "lys-identity-server ",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("LYS_BUILD"),
    ")"
);

/// The instant a line is said, RFC 3339 in UTC with milliseconds, so every
/// line of identity.log can be placed in time (DIRECTORY-118 R1).
fn instant() -> String {
    let now = jiff::Timestamp::now();
    format!(
        "{}.{:03}Z",
        now.strftime("%Y-%m-%dT%H:%M:%S"),
        now.subsec_millisecond()
    )
}

/// Milliseconds since `since`, for the start's own timing line.
fn ms(since: Instant) -> u128 {
    since.elapsed().as_millis()
}

/// Serve, saying on the listening line how long the start took and where its
/// milliseconds went (Tom, 8 Oct 2026: a start in seconds is a defect, and a
/// start nobody timed cannot be shown to be in milliseconds).
async fn serve(config_path: PathBuf, began: Instant) -> Result<(), ServerError> {
    let config = Config::load(&config_path)?;
    planned_stop::retire_previous_plan(&config_path)?;
    let configured = ms(began);
    let say: lys_identity_server::routes::Say =
        Arc::new(|line: &str| println!("{} lys-identity-server {line}", instant()));
    // Warn and above reach identity.log through the same sink (DIRECTORY-095).
    lys_identity_server::warn_lines::install(Arc::clone(&say)).map_err(|refused| {
        ServerError::ConfigInvalid {
            reason: refused.to_string(),
        }
    })?;
    let service_began = Instant::now();
    let started = started_saying_with_clock(
        &config,
        Arc::clone(&say),
        lys_core::clock::ClockSource::System,
    )
    .await?;
    let app = started.router;
    let service = ms(service_began);
    let stop = planned_stop::PlannedStop::new();
    let signals = planned_stop::Signals::open()?;
    let app = planned_stop::router(app, stop.clone());
    let bind_began = Instant::now();
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(|error| ServerError::ConfigInvalid {
            reason: format!("{} could not be bound: {error}", config.listen),
        })?;
    match &started.refused {
        // A refused start never says it is listening: the installer's wait
        // ends on this line as a failure, naming the stores (DIRECTORY-107 R2).
        Some(refused) => println!(
            "lys-identity-server {refused} Serving only /health and the sign-in page on {}.",
            config.listen
        ),
        None => println!(
            "lys-identity-server listening on {}, ready in {} ms (configuration {configured} ms, service {service} ms, bind {} ms)",
            config.listen,
            ms(began),
            ms(bind_began)
        ),
    }
    // The connection's address is the person's own, carried to the issuer
    // so its failed sign-in count is per person.
    let (finished, stopped) = tokio::sync::oneshot::channel();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let outcome = signals.wait(stop, config_path).await;
        if finished.send(outcome).is_err() {
            eprintln!("planned_stop_result_lost: the service left before reading its stop result");
        }
    })
    .await
    .map_err(|error| ServerError::DirectoryUnavailable {
        reason: error.to_string(),
    })?;
    let outcome = stopped.await.map_err(|error| ServerError::ConfigInvalid {
        reason: format!("planned_stop_result_lost: {error}"),
    })?;
    // A clean stop writes the stores receipt from every store as it stands.
    if let Some(at_stop) = &started.at_stop {
        at_stop.write()?;
    }
    outcome
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

#[cfg(test)]
mod instant_tests {
    #[test]
    fn the_instant_is_rfc3339_utc_with_milliseconds() {
        let said = super::instant();
        let bytes = said.as_bytes();
        assert_eq!(said.len(), 24, "{said}");
        for (index, byte) in bytes.iter().enumerate() {
            let expected = match index {
                4 | 7 => b'-' == *byte,
                10 => b'T' == *byte,
                13 | 16 => b':' == *byte,
                19 => b'.' == *byte,
                23 => b'Z' == *byte,
                _ => byte.is_ascii_digit(),
            };
            assert!(expected, "{said} at {index}");
        }
    }
}
