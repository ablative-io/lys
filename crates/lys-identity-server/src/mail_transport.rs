//! Each letter owns its socket. Kernel keepalive and dropping that letter
//! close it; no pool task or socket deadline survives the caller.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use lettre::Message;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::client::{AsyncSmtpConnection, AsyncTokioStream, TlsParameters};
use lettre::transport::smtp::extension::ClientId;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

use crate::mail::{MailSettings, Tls};

pub(super) struct Transport {
    host: String,
    port: u16,
    tls: Tls,
    credentials: Option<Credentials>,
}

impl Transport {
    pub(super) fn new(settings: &MailSettings, credentials: Option<Credentials>) -> Self {
        Self {
            host: settings.host.clone(),
            port: settings.port,
            tls: settings.tls,
            credentials,
        }
    }

    pub(super) fn peer(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub(super) async fn send(&self, message: &Message) -> Result<(), String> {
        let socket = TcpStream::connect((self.host.as_str(), self.port))
            .await
            .map_err(|error| error.to_string())?;
        crate::outbound::keepalive(&socket).map_err(|error| error.to_string())?;
        let socket: Box<dyn AsyncTokioStream> = match self.tls {
            Tls::Implicit => {
                let roots = rustls::RootCertStore::from_iter(
                    webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
                );
                let provider = Arc::new(rustls::crypto::ring::default_provider());
                let config = rustls::ClientConfig::builder_with_provider(provider)
                    .with_safe_default_protocol_versions()
                    .map_err(|error| error.to_string())?
                    .with_root_certificates(roots)
                    .with_no_client_auth();
                let name = rustls::pki_types::ServerName::try_from(self.host.clone())
                    .map_err(|error| error.to_string())?;
                let stream = tokio_rustls::TlsConnector::from(Arc::new(config))
                    .connect(name, socket)
                    .await
                    .map_err(|error| error.to_string())?;
                Box::new(Secured(stream))
            }
            Tls::Starttls | Tls::None => Box::new(socket),
        };
        let hello = ClientId::default();
        let mut connection = AsyncSmtpConnection::connect_with_transport(socket, &hello)
            .await
            .map_err(|error| error.to_string())?;
        if self.tls == Tls::Starttls {
            let tls = TlsParameters::new(self.host.clone()).map_err(|error| error.to_string())?;
            connection
                .starttls(tls, &hello)
                .await
                .map_err(|error| error.to_string())?;
        }
        if let Some(credentials) = &self.credentials {
            connection
                .auth(&[Mechanism::Plain, Mechanism::Login], credentials)
                .await
                .map_err(|error| error.to_string())?;
        }
        connection
            .send(message.envelope(), &message.formatted())
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[derive(Debug)]
struct Secured(tokio_rustls::client::TlsStream<TcpStream>);

impl AsyncTokioStream for Secured {
    fn peer_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.0.get_ref().0.peer_addr()
    }
}
impl AsyncRead for Secured {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(context, buffer)
    }
}
impl AsyncWrite for Secured {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(context, bytes)
    }
    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(context)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(context)
    }
}
