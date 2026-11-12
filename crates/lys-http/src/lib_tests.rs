#![cfg(test)]
//! The client's TLS refuses what it must, and the refusals are rustls's own,
//! read by name from the error chain; a control accepts what it must, so a
//! refusal is shown to be about the certificate and not about the harness.
//! Each server is a rustls server on a loopback port, on certificates these
//! tests issue themselves.

use std::error::Error;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread::JoinHandle;

use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};

type TestResult<T> = Result<T, Box<dyn Error>>;

/// A root of the tests' own, and a server certificate it issued for `name`.
struct Issued {
    root: CertificateDer<'static>,
    leaf: CertificateDer<'static>,
    key: Vec<u8>,
}

fn issue(name: &str) -> TestResult<Issued> {
    let mut root_params = CertificateParams::new(Vec::<String>::new())?;
    root_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    root_params
        .distinguished_name
        .push(DnType::CommonName, "lys-http test root");
    let root_key = KeyPair::generate()?;
    let root = root_params.self_signed(&root_key)?;
    let issuer = Issuer::new(root_params, root_key);
    let leaf_key = KeyPair::generate()?;
    let leaf = CertificateParams::new(vec![name.to_owned()])?.signed_by(&leaf_key, &issuer)?;
    Ok(Issued {
        root: root.der().clone(),
        leaf: leaf.der().clone(),
        key: leaf_key.serialize_der(),
    })
}

/// Serves one TLS connection on 127.0.0.1 with `issued`'s server certificate,
/// answering a request with an empty 204 when the handshake completes. The
/// thread's result says whether a handshake completed.
fn serve(issued: &Issued) -> TestResult<(u16, JoinHandle<bool>)> {
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_no_client_auth()
    .with_single_cert(
        vec![issued.leaf.clone()],
        PrivatePkcs8KeyDer::from(issued.key.clone()).into(),
    )?;
    let config = Arc::new(config);
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let serving = std::thread::spawn(move || {
        let Ok((mut socket, _)) = listener.accept() else {
            return false;
        };
        let Ok(mut connection) = rustls::ServerConnection::new(config) else {
            return false;
        };
        while connection.is_handshaking() {
            // The handshake ends when the client refuses the certificate.
            if connection.complete_io(&mut socket).is_err() {
                return false;
            }
        }
        let mut stream = rustls::StreamOwned::new(connection, socket);
        let mut head = [0u8; 1024];
        if std::io::Read::read(&mut stream, &mut head).is_err() {
            return false;
        }
        let answered = std::io::Write::write_all(
            &mut stream,
            b"HTTP/1.1 204 No Content\r\nconnection: close\r\n\r\n",
        );
        stream.conn.send_close_notify();
        answered.is_ok() && std::io::Write::flush(&mut stream).is_ok()
    });
    Ok((port, serving))
}

/// Whether a certificate refusal of rustls's that `kind` accepts is anywhere
/// in `error`'s chain. An io::Error's `source()` skips the error it wraps, so
/// that one is read through `get_ref()`.
fn refused_by(
    error: &(dyn Error + 'static),
    kind: &dyn Fn(&rustls::CertificateError) -> bool,
) -> bool {
    let mut next = Some(error);
    while let Some(error) = next {
        if let Some(rustls::Error::InvalidCertificate(found)) =
            error.downcast_ref::<rustls::Error>()
            && kind(found)
        {
            return true;
        }
        if let Some(inner) = error
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::get_ref)
            && refused_by(inner, kind)
        {
            return true;
        }
        next = error.source();
    }
    false
}

fn trusting(root: &CertificateDer<'static>) -> TestResult<reqwest::Client> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(root.clone())?;
    Ok(super::builder_trusting(roots)?.build()?)
}

#[test]
fn a_client_builds_with_no_process_provider_installed() -> TestResult<()> {
    assert!(rustls::crypto::CryptoProvider::get_default().is_none());
    super::client()?;
    assert!(rustls::crypto::CryptoProvider::get_default().is_none());
    Ok(())
}

/// The roots are the bundled ones: a server under a root they do not hold is
/// refused as an unknown issuer, whatever the machine's own store holds.
#[tokio::test]
async fn a_server_outside_the_bundled_roots_is_refused_as_an_unknown_issuer() -> TestResult<()> {
    let issued = issue("127.0.0.1")?;
    let (port, serving) = serve(&issued)?;
    let Err(refused) = super::client()?
        .get(format!("https://127.0.0.1:{port}/"))
        .send()
        .await
    else {
        return Err("the client accepted a certificate no bundled root issued".into());
    };
    assert!(!serving.join().map_err(|_| "the server thread panicked")?);
    assert!(
        refused_by(&refused, &|kind| matches!(
            kind,
            rustls::CertificateError::UnknownIssuer
        )),
        "refused, but not as an unknown issuer: {refused:?}"
    );
    Ok(())
}

/// Under a root the client trusts, a certificate for another name is refused
/// as not valid for the name asked.
#[tokio::test]
async fn a_certificate_for_another_name_is_refused_under_a_trusted_root() -> TestResult<()> {
    let issued = issue("other.example")?;
    let (port, serving) = serve(&issued)?;
    let Err(refused) = trusting(&issued.root)?
        .get(format!("https://127.0.0.1:{port}/"))
        .send()
        .await
    else {
        return Err("the client accepted a certificate for another name".into());
    };
    assert!(!serving.join().map_err(|_| "the server thread panicked")?);
    assert!(
        refused_by(&refused, &|kind| matches!(
            kind,
            rustls::CertificateError::NotValidForName
                | rustls::CertificateError::NotValidForNameContext { .. }
        )),
        "refused, but not for its name: {refused:?}"
    );
    Ok(())
}

/// The control: the same construction, a trusted root and the right name,
/// completes the handshake and is answered. Without it, the two refusals
/// above could be the harness failing rather than the certificate.
#[tokio::test]
async fn a_certificate_for_the_name_under_a_trusted_root_is_answered() -> TestResult<()> {
    let issued = issue("127.0.0.1")?;
    let (port, serving) = serve(&issued)?;
    let answer = trusting(&issued.root)?
        .get(format!("https://127.0.0.1:{port}/"))
        .send()
        .await?;
    assert_eq!(answer.status(), reqwest::StatusCode::NO_CONTENT);
    assert!(serving.join().map_err(|_| "the server thread panicked")?);
    Ok(())
}
