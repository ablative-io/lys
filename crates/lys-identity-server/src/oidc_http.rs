//! openidconnect's HTTP calls, made through lys-http's client.
//!
//! openidconnect's own "reqwest" feature ties it to reqwest 0.12, and Lys is on
//! reqwest 0.13 with every client built by lys-http (rustls on ring, the bundled
//! webpki-roots). openidconnect takes any client that implements its
//! `AsyncHttpClient`, so the service hands it this one. A call is translated the
//! way openidconnect's own reqwest client translates it: the request is sent as
//! openidconnect built it, and the answer's status, version, headers and whole
//! body are handed back unchanged. Nothing here reads, retries or follows a
//! redirect: what the client does with a redirect is the policy it was built
//! with, and every token check stays openidconnect's.

use std::future::Future;
use std::pin::Pin;

use openidconnect::{AsyncHttpClient, HttpClientError, HttpRequest, HttpResponse};

/// A reqwest client, as openidconnect's HTTP client.
#[derive(Clone, Debug)]
pub struct OidcHttp(reqwest::Client);

impl OidcHttp {
    /// Wraps `client`, which lys-http built.
    #[must_use]
    pub fn new(client: reqwest::Client) -> Self {
        Self(client)
    }

    /// The client underneath, for the calls the service makes itself.
    #[must_use]
    pub fn client(&self) -> &reqwest::Client {
        &self.0
    }
}

impl<'c> AsyncHttpClient<'c> for OidcHttp {
    type Error = HttpClientError<reqwest::Error>;
    type Future = Pin<Box<dyn Future<Output = Result<HttpResponse, Self::Error>> + Send + 'c>>;

    fn call(&'c self, request: HttpRequest) -> Self::Future {
        Box::pin(async move {
            let request = reqwest::Request::try_from(request).map_err(Box::new)?;
            let response = self.0.execute(request).await.map_err(Box::new)?;
            let mut answer = openidconnect::http::Response::builder()
                .status(response.status())
                .version(response.version());
            for (name, value) in response.headers() {
                answer = answer.header(name, value);
            }
            let body = response.bytes().await.map_err(Box::new)?;
            answer.body(body.to_vec()).map_err(HttpClientError::Http)
        })
    }
}
