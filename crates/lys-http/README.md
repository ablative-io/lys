# lys-http

The one way Lys builds an HTTP client. Every reqwest client in Lys, and in the products that take
lys-pass's `http` feature, is built by `lys_http::builder()` or `lys_http::client()`.

Its TLS is rustls on ring, with the bundled webpki-roots as its only trust anchors. A machine's own
certificate store neither widens nor narrows what Lys trusts, and a machine without one still
connects. No process-wide crypto provider is installed: the configuration names its provider
itself.

## Why rustls-platform-verifier is in the lock, and why it never verifies anything

reqwest 0.13.5 is built with its `rustls-no-provider` feature, the only public rustls feature that
does not bring aws-lc-rs. That feature is `["dep:rustls-platform-verifier", "__rustls"]` in
reqwest's own manifest. So rustls-platform-verifier 0.7.1 is compiled into every binary that links
reqwest. reqwest has no public rustls feature without it, and lys does not depend on reqwest's
private `__rustls` feature.

It is compiled in and unreachable from a Lys client:

- reqwest constructs the platform verifier only in its default TLS arm, `TlsBackend::Rustls`
  (reqwest 0.13.5, `src/async_impl/client.rs`, lines 759 and 767).
- lys-http hands reqwest a finished `rustls::ClientConfig` through `tls_backend_preconfigured`,
  which sets `TlsBackend::BuiltRustls` (line 2226). That arm uses the configuration as given (ring,
  the bundled roots, HTTP/1.1 by ALPN) and never the verifier.
- The default arm cannot run in a working Lys process in any case. It first asks
  `CryptoProvider::get_default()`, and with none installed calls `default_rustls_crypto_provider`,
  which panics in a build without aws-lc (lines 2499 to 2507). Lys installs no provider:
  `a_client_builds_with_no_process_provider_installed` (`src/lib_tests.rs`) asserts there is none
  before and after a client is built.
- No client is built any other way: `tests/one_builder.rs` reads every Rust file under `crates/`
  and `tests/` and refuses a reqwest client built anywhere but in this crate.

What the client trusts is held by tests too: `production_trusts_exactly_the_bundled_roots`, and
the refusals of a server outside the bundled roots (an unknown issuer) and of a certificate for
another name, each against a control that is answered.
