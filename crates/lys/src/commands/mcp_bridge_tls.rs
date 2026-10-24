//! The bridge's way to Lys: plain HTTP to a loopback address, or TLS to Lys on any
//! host, verified against this system's trust roots (AGENTS-015 R3).
//!
//! The roots are the system's own, read once by `rustls-native-certs` (the keychain's
//! trust settings on macOS, the platform store elsewhere), and every certificate is
//! verified by rustls-webpki under rustls with ring, the TLS stack lys-home and
//! lys-runner already carry. Nothing turns verification off and nothing accepts any
//! certificate: there is no such flag, option, variable or code path. The one seam is
//! a root added to the same trust path, compiled only for this crate's tests, so a test
//! certificate is trusted exactly the way a system root is.
//!
//! No step is bounded by a clock: the handshake ends on its own event, the peer's
//! answer or its closing, as the loopback exchange does.

use std::sync::Arc;

use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use crate::commands::error::CliResult;
use crate::commands::seat_client::refused;
use crate::identity::loopback_http::{
    Authority, Failure, Request, Response, exchange, exchange_over,
};

/// The verifying trust path: the system's roots, and nothing that turns it off.
pub(crate) struct TrustPath {
    config: Arc<ClientConfig>,
}

impl TrustPath {
    /// The system's trust roots, read once.
    ///
    /// # Errors
    /// Refuses `mcp_trust_roots_unreadable` when no system root can be read, naming
    /// what the platform store said.
    pub(crate) fn system() -> CliResult<Self> {
        Self::with_roots(true, &[])
    }

    /// The test's root alone on the same verifying path, for this crate's tests:
    /// what the bridge proves about verification does not depend on the machine's
    /// own trust store, which a detached gate cannot always read (24 October 2026).
    ///
    /// # Errors
    /// `mcp_trust_roots_unreadable` when `root` is not a usable trust anchor.
    #[cfg(test)]
    pub(crate) fn only(root: &CertificateDer<'static>) -> CliResult<Self> {
        Self::with_roots(false, std::slice::from_ref(root))
    }

    fn with_roots(system: bool, extra: &[CertificateDer<'static>]) -> CliResult<Self> {
        let mut roots = RootCertStore::empty();
        if system {
            let loaded = rustls_native_certs::load_native_certs();
            let (added, ignored) = roots.add_parsable_certificates(loaded.certs);
            let problems: Vec<String> = loaded.errors.iter().map(ToString::to_string).collect();
            if added == 0 {
                return Err(refused(
                    "mcp_trust_roots_unreadable",
                    format!(
                        "no trust root could be read from this system ({ignored} unusable): {}",
                        if problems.is_empty() {
                            "the store is empty".to_owned()
                        } else {
                            problems.join("; ")
                        }
                    ),
                ));
            }
            if !problems.is_empty() {
                eprintln!(
                    "mcp_trust_roots_partial: {added} system roots read; not read: {}",
                    problems.join("; ")
                );
            }
        }
        for root in extra {
            roots.add(root.clone()).map_err(|error| {
                refused(
                    "mcp_trust_roots_unreadable",
                    format!("the added root is not a trust anchor: {error}"),
                )
            })?;
        }
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|error| refused("mcp_tls_unavailable", error.to_string()))?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Self {
            config: Arc::new(config),
        })
    }
}

/// How the bridge reaches Lys.
pub(crate) enum Transport {
    /// Plain HTTP, to a numeric loopback address only.
    Loopback,
    /// TLS, verified on the trust path, to any host.
    Tls(TrustPath),
}

/// Why a message had no answer from Lys.
#[derive(Debug)]
pub(crate) enum Unanswered {
    /// The TLS handshake refused the server, with the host and the verifier's reason.
    TlsRefused(String),
    /// The exchange itself failed, as over loopback.
    Exchange(Failure),
}

/// Sends `request` to `authority` over `transport` and reads the whole response.
///
/// # Errors
/// [`Unanswered::TlsRefused`] when the server's certificate is not verified on the
/// trust path or the handshake is refused; [`Unanswered::Exchange`] when no connection
/// is made or no whole answer arrives.
pub(crate) fn post(
    transport: &Transport,
    authority: &Authority,
    request: &Request<'_>,
) -> Result<Response, Unanswered> {
    let Transport::Tls(trust) = transport else {
        return exchange(authority, request).map_err(Unanswered::Exchange);
    };
    let name = ServerName::try_from(authority.host.clone()).map_err(|error| {
        Unanswered::TlsRefused(format!(
            "{authority}: the host is not a TLS server name: {error}"
        ))
    })?;
    let mut connection = ClientConnection::new(Arc::clone(&trust.config), name)
        .map_err(|error| Unanswered::TlsRefused(format!("{authority}: {error}")))?;
    let mut socket = authority
        .connect()
        .map_err(|cause| Unanswered::Exchange(Failure::Unreachable(cause)))?;
    while connection.is_handshaking() {
        connection
            .complete_io(&mut socket)
            .map_err(|error| handshake_failure(authority, &error))?;
    }
    let mut stream = StreamOwned::new(connection, socket);
    exchange_over(&mut stream, authority, request).map_err(Unanswered::Exchange)
}

/// A handshake that failed: refused by TLS (the verifier or the peer's alert), or the
/// connection lost before any request was sent.
fn handshake_failure(authority: &Authority, error: &std::io::Error) -> Unanswered {
    match error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<rustls::Error>())
    {
        Some(refusal) => Unanswered::TlsRefused(format!("{authority}: {refusal}")),
        None => Unanswered::Exchange(Failure::Unreachable(format!(
            "{authority}: the connection was lost during the TLS handshake: {error}"
        ))),
    }
}
