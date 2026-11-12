//! The one way Lys builds an HTTP client.
//!
//! Every reqwest client in Lys is built here. Its TLS is rustls on ring, the
//! provider the rest of Lys's rustls runs on, with the bundled webpki-roots as
//! its trust anchors: a machine's own certificate store neither widens nor
//! narrows what Lys trusts, and a machine without one still connects. This is
//! what reqwest 0.12's `rustls-tls` gave. reqwest 0.13 would instead verify
//! through the operating system's store (rustls-platform-verifier), and, built
//! without a crypto provider as it is here, refuses to build a client at all
//! unless one is installed for the process. Lys installs none: the
//! configuration below names its provider itself.
//!
//! `tests/one_builder.rs` refuses a reqwest client built anywhere else in the
//! workspace.

use std::sync::Arc;

/// Why a client could not be built.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// rustls refused the TLS configuration.
    #[error("the TLS configuration could not be built: {0}")]
    Tls(#[from] rustls::Error),
    /// reqwest refused the client.
    #[error("the HTTP client could not be built: {0}")]
    Client(#[from] reqwest::Error),
}

/// A client builder carrying Lys's TLS, for a caller that sets more on it
/// (a redirect policy, keepalive) before building.
///
/// # Errors
///
/// Refuses when rustls cannot build the TLS configuration.
pub fn builder() -> Result<reqwest::ClientBuilder, Error> {
    Ok(builder_trusting(bundled_roots())?)
}

/// A client with Lys's TLS and reqwest's other defaults.
///
/// # Errors
///
/// Refuses when rustls cannot build the TLS configuration or reqwest the client.
pub fn client() -> Result<reqwest::Client, Error> {
    Ok(builder()?.build()?)
}

/// The bundled webpki-roots, as rustls's root store.
fn bundled_roots() -> rustls::RootCertStore {
    rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    }
}

/// A builder whose TLS trusts `roots` and nothing else. [`builder`] passes the
/// bundled roots; the tests pass a root of their own, to show a name the
/// certificate does not carry is refused under a root the client trusts.
fn builder_trusting(roots: rustls::RootCertStore) -> Result<reqwest::ClientBuilder, rustls::Error> {
    Ok(reqwest::Client::builder().tls_backend_preconfigured(tls(roots)?))
}

/// rustls on ring, `roots`, rustls's safe default protocol versions, and
/// HTTP/1.1 offered by ALPN: reqwest uses a preconfigured configuration as it
/// is given, ALPN included, and HTTP/1.1 is the only version Lys builds
/// reqwest with.
fn tls(roots: rustls::RootCertStore) -> Result<rustls::ClientConfig, rustls::Error> {
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(config)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
