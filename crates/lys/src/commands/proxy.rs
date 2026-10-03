//! `lys proxy`: parse, then hand to `lys-home`'s proxy, which holds all of
//! the proxy's logic.

use std::io::Write;

use lys_home::proxy::error::ProxyError;
use lys_home::proxy::forward::{Base, Proxy, ProxyConfig};
use lys_home::proxy::journal::CallReport;
use serde_json::json;

use crate::cli::ProxyCommand;
use crate::commands::error::{CliError, CliResult};

/// The login's own Anthropic base, which the proxy keeps as its upstream.
pub const LOGIN_BASE: &str = "ANTHROPIC_BASE_URL";

/// Anthropic's API, the upstream when nothing names another.
const ANTHROPIC: &str = "https://api.anthropic.com";

/// Where the Anthropic upstream came from, as the start line names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// `--anthropic` on the command line.
    Flag,
    /// The login's own `ANTHROPIC_BASE_URL`.
    Login,
    /// Neither: Anthropic's API.
    Default,
}

impl Source {
    fn word(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::Login => "login",
            Self::Default => "default",
        }
    }
}

/// The Anthropic upstream: the flag, else the login's own base, else
/// Anthropic's API. An empty login value is no value.
pub fn anthropic_upstream(flag: Option<String>, login: Option<String>) -> (String, Source) {
    match (flag, login.filter(|value| !value.is_empty())) {
        (Some(flag), _) => (flag, Source::Flag),
        (None, Some(login)) => (login, Source::Login),
        (None, None) => (ANTHROPIC.to_owned(), Source::Default),
    }
}

/// A base as the start line names it: scheme, host, port and path, without
/// any user, password or query it carried.
pub fn shown(base: &str) -> String {
    let (scheme, rest) = base.split_once("://").unwrap_or(("", base));
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let (authority, path) = rest.find('/').map_or((rest, ""), |at| rest.split_at(at));
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    format!("{scheme}://{host}{path}")
}

/// Whether `base` names `listen` itself, which would send each call back
/// into the proxy.
pub fn names_itself(base: &str, listen: &str) -> bool {
    let shown = shown(base);
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

/// Runs `lys proxy`.
pub fn run(command: ProxyCommand) -> CliResult<()> {
    let ProxyCommand::Serve {
        listen,
        home,
        state,
        anthropic,
        openai,
    } = command;
    let (anthropic, source) = anthropic_upstream(anthropic, std::env::var(LOGIN_BASE).ok());
    if names_itself(&anthropic, &listen) {
        return Err(ProxyError::BadUpstream {
            base: shown(&anthropic),
            reason: format!(
                "the Anthropic upstream ({}) is this proxy's own address; \
                 the login's {LOGIN_BASE} must name the provider or gateway, not Lys's proxy",
                source.word()
            ),
        }
        .into());
    }
    let config = ProxyConfig {
        home,
        state,
        anthropic: Base::parse(&anthropic)?,
        openai: Base::parse(&openai)?,
        capture_slots: 64,
    };
    let start = json!({
        "proxy": "listening",
        "listen": listen,
        "anthropic": shown(&anthropic),
        "anthropic_from": source.word(),
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
