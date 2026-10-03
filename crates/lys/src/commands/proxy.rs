//! `lys proxy`: parse, then hand to `lys-home`'s proxy, which holds all of
//! the proxy's logic.

use std::io::Write;
use std::path::Path;

use lys_home::proxy::error::ProxyError;
use lys_home::proxy::forward::{Base, Proxy, ProxyConfig};
use lys_home::proxy::journal::CallReport;
use serde_json::json;

use crate::cli::ProxyCommand;
use crate::commands::error::{CliError, CliResult};
use crate::identity::install::proxy::{ANTHROPIC, Upstream, shown};

/// The Anthropic upstream and where it came from: `--anthropic`, else the
/// record `--upstream` names, else Anthropic's API. Never the environment.
pub fn anthropic_upstream(
    flag: Option<String>,
    record: Option<&Path>,
) -> CliResult<(String, String)> {
    if let Some(flag) = flag {
        return Ok((flag, "flag".to_owned()));
    }
    let Some(path) = record else {
        return Ok((ANTHROPIC.to_owned(), "default".to_owned()));
    };
    let bytes = std::fs::read(path).map_err(|source| CliError::Io {
        context: format!("reading the proxy's upstream record {}", path.display()),
        source,
    })?;
    let upstream: Upstream =
        serde_json::from_slice(&bytes).map_err(|source| CliError::JsonParse {
            what: "proxy upstream",
            path: path.to_path_buf(),
            source,
        })?;
    Ok((upstream.anthropic, upstream.from))
}

/// Whether the parsed `base` names `listen` itself, which would send each
/// call back into the proxy.
pub fn names_itself(base: &Base, listen: &str) -> bool {
    let shown = shown(base.as_str());
    let authority = shown
        .split_once("://")
        .map_or(shown.as_str(), |(_, rest)| rest)
        .split('/')
        .next()
        .unwrap_or_default();
    let Some((_, port)) = listen.rsplit_once(':') else {
        return false;
    };
    ["127.0.0.1", "localhost", "[::1]", "0.0.0.0"]
        .iter()
        .any(|host| authority == format!("{host}:{port}"))
}

/// Parses an upstream, naming it in any refusal without the user, password
/// or query it carried.
fn parsed(base: &str) -> Result<Base, ProxyError> {
    Base::parse(base).map_err(|error| match error {
        ProxyError::BadUpstream { reason, .. } => ProxyError::BadUpstream {
            base: shown(base),
            reason,
        },
        other => other,
    })
}

/// Runs `lys proxy`.
pub fn run(command: ProxyCommand) -> CliResult<()> {
    let ProxyCommand::Serve {
        listen,
        home,
        state,
        anthropic,
        upstream,
        openai,
    } = command;
    // Every prompt and reply of every run passes into the proxy's home and
    // state: what it writes is its owner's alone.
    #[cfg(unix)]
    rustix::process::umask(rustix::fs::Mode::from_raw_mode(0o077));
    let (anthropic, source) = anthropic_upstream(anthropic, upstream.as_deref())?;
    // An unparseable upstream is refused by name first; only a parsed one is
    // asked whether it names the proxy itself.
    let anthropic_base = parsed(&anthropic)?;
    if names_itself(&anthropic_base, &listen) {
        return Err(ProxyError::BadUpstream {
            base: shown(&anthropic),
            reason: format!(
                "the Anthropic upstream (from {source}) is this proxy's own address; \
                 the login's ANTHROPIC_BASE_URL must name the provider or gateway, not Lys's proxy"
            ),
        }
        .into());
    }
    let config = ProxyConfig {
        home,
        state,
        anthropic: anthropic_base,
        openai: parsed(&openai)?,
        capture_slots: 64,
    };
    let start = json!({
        "proxy": "listening",
        "listen": listen,
        "anthropic": shown(&anthropic),
        "anthropic_from": source,
        "openai": shown(&openai),
    });
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(8)
        .enable_all()
        .build()
        .map_err(|source| ProxyError::io("starting the runtime", &config.state, source))?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind(&listen)
            .await
            .map_err(|source| {
                ProxyError::io("binding the listen address", &config.state, source)
            })?;
        let report_state = config.state.clone();
        let started = Proxy::start(config)?;
        for report in &started.lost {
            print_report(report);
        }
        let mut out = std::io::stdout();
        writeln!(out, "{start}")
            .and_then(|()| out.flush())
            .map_err(|source| CliError::Io {
                context: "writing that the proxy listens".to_owned(),
                source,
            })?;
        let reports = started.reports;
        std::thread::Builder::new()
            .name(String::from("lys-proxy-reports"))
            .spawn(move || {
                for report in reports {
                    print_report(&report);
                }
            })
            .map_err(|source| {
                ProxyError::io("starting the report worker", &report_state, source)
            })?;
        started.proxy.serve(listener).await.map_err(CliError::from)
    })
}

fn print_report(report: &CallReport) {
    match serde_json::to_string(report) {
        Ok(line) => println!("{line}"),
        Err(error) => eprintln!("lys proxy: a call report could not be serialised: {error}"),
    }
}

#[cfg(test)]
#[path = "proxy_tests.rs"]
mod tests;
