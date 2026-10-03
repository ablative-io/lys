//! The pass-through proxy, for the subscription proof only (HOME-001 R7).
//!
//! `cargo run -p lys-home --example passthrough -- <provider base URL> [listen address]`
//!
//! Every request is forwarded to the base with its headers and streamed body
//! unchanged and its response returned unchanged, through the one transport
//! `lys proxy serve` uses (`lys_home::proxy::forward`). The only thing written is
//! one line per call on stderr, `{method, path, status, duration_ms}`; no
//! header value and no body byte is written, printed or stored. It is an
//! example binary, never a service: point one harness at it with
//! `ANTHROPIC_BASE_URL=http://<listen address>`, measure, and stop it.

use std::process::ExitCode;
use std::sync::Arc;

use lys_home::proxy::error::ProxyError;
use lys_home::proxy::forward::{Base, Upstream, pass_through, serve};

/// Where the example listens when no address is given.
const DEFAULT_LISTEN: &str = "127.0.0.1:8485";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(base) = args.next() else {
        eprintln!("usage: passthrough <provider base URL> [listen address]");
        return ExitCode::from(2);
    };
    let listen = args.next().unwrap_or_else(|| DEFAULT_LISTEN.to_owned());
    match run(&base, &listen) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("passthrough: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(base: &str, listen: &str) -> Result<(), ProxyError> {
    let base = Base::parse(base)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|source| ProxyError::Accept { source })?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind(listen)
            .await
            .map_err(|source| ProxyError::Accept { source })?;
        let transport = Arc::new((Upstream::new()?, base));
        serve(listener, move |request| {
            let transport = Arc::clone(&transport);
            async move { pass_through(&transport.0, &transport.1, request).await }
        })
        .await
    })
}
