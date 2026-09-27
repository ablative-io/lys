//! Typed Rauthy API requests and responses over plain HTTP/1.1.
//!
//! Invariants:
//!
//! - Only the endpoints configure needs are spoken: the client list, one
//!   client, client creation and update, a client's secret, and a client's
//!   theme. Rauthy is reached at its published listener on this node; TLS
//!   for the public origin terminates at the reverse proxy in front of it.
//! - A transport failure is classified by whether the request left this
//!   process: [`WriteResult::Uncertain`] means it was sent and no response
//!   arrived, so the caller must read the resource back before it knows the
//!   outcome. A request that was never sent is [`IdentityError::Unreachable`].
//! - Every error status becomes [`IdentityError::RauthyStatus`] with a stable
//!   name. Rauthy's own message is kept, scrubbed of the API key secret this
//!   client holds, so a server that echoes a credential cannot put it into an
//!   error.
//! - The `Authorization` header is assembled in a zeroizing buffer and never
//!   formatted through `format!`.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::identity::credentials::{API_KEY_NAME, Secret};
use crate::identity::error::{IdentityError, IdentityResult};

/// How long any one request may take to connect, send or answer.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// An HTTP response.
#[derive(Debug)]
pub struct HttpResponse {
    /// The status code.
    pub status: u16,
    /// The body, de-chunked.
    pub body: Vec<u8>,
}

/// Why a request produced no response.
#[derive(Debug)]
pub enum TransportFailure {
    /// The request never left this process.
    NotSent(String),
    /// The request was sent; the response did not arrive.
    Uncertain(String),
}

/// The outcome of a write whose response may not have arrived.
#[derive(Debug, PartialEq, Eq)]
pub enum WriteResult {
    /// Rauthy answered 2xx.
    Applied,
    /// The request was sent and no response arrived.
    Uncertain(String),
}

/// One OIDC client as Rauthy reports it (the fields configure manages).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ClientRecord {
    /// Client id.
    pub id: String,
    /// Display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Whether the client is enabled.
    pub enabled: bool,
    /// Whether the client is confidential.
    pub confidential: bool,
    /// Redirect URIs.
    pub redirect_uris: Vec<String>,
    /// Post-logout redirect URIs.
    #[serde(default)]
    pub post_logout_redirect_uris: Option<Vec<String>>,
    /// Enabled grant types.
    pub flows_enabled: Vec<String>,
    /// Access-token signing algorithm.
    pub access_token_alg: String,
    /// ID-token signing algorithm.
    pub id_token_alg: String,
    /// Authorization-code lifetime, seconds.
    pub auth_code_lifetime: i32,
    /// Access-token lifetime, seconds.
    pub access_token_lifetime: i32,
    /// Allowed scopes.
    pub scopes: Vec<String>,
    /// Default scopes.
    pub default_scopes: Vec<String>,
    /// Allowed PKCE methods.
    #[serde(default)]
    pub challenges: Option<Vec<String>>,
    /// Whether MFA is forced.
    pub force_mfa: bool,
}

/// `POST /auth/v1/clients`.
#[derive(Debug, Serialize)]
pub struct NewClient<'a> {
    /// Client id.
    pub id: &'a str,
    /// Display name.
    pub name: &'a str,
    /// Confidential client.
    pub confidential: bool,
    /// Redirect URIs.
    pub redirect_uris: &'a [String],
    /// Post-logout redirect URIs.
    pub post_logout_redirect_uris: &'a [String],
}

/// `PUT /auth/v1/clients/{id}`: the full managed state of a client.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClientUpdate {
    /// Display name.
    pub name: String,
    /// Confidential client.
    pub confidential: bool,
    /// Redirect URIs.
    pub redirect_uris: Vec<String>,
    /// Post-logout redirect URIs.
    pub post_logout_redirect_uris: Option<Vec<String>>,
    /// Enabled.
    pub enabled: bool,
    /// Enabled grant types.
    pub flows_enabled: Vec<String>,
    /// Access-token signing algorithm.
    pub access_token_alg: String,
    /// ID-token signing algorithm.
    pub id_token_alg: String,
    /// Authorization-code lifetime, seconds.
    pub auth_code_lifetime: i32,
    /// Access-token lifetime, seconds.
    pub access_token_lifetime: i32,
    /// Allowed scopes.
    pub scopes: Vec<String>,
    /// Default scopes.
    pub default_scopes: Vec<String>,
    /// Allowed PKCE methods.
    pub challenges: Option<Vec<String>>,
    /// Forced MFA.
    pub force_mfa: bool,
}

/// A client's theme as Rauthy exports and accepts it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeDocument {
    /// The client the theme belongs to.
    pub client_id: String,
    /// Light mode colours.
    pub light: ThemeColours,
    /// Dark mode colours.
    pub dark: ThemeColours,
    /// Corner radius, a CSS value.
    pub border_radius: String,
}

/// One mode of a theme: seven HSL colours and three CSS values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeColours {
    /// Body text.
    pub text: [u16; 3],
    /// Emphasised text.
    pub text_high: [u16; 3],
    /// Page background.
    pub bg: [u16; 3],
    /// Raised background.
    pub bg_high: [u16; 3],
    /// Action colour.
    pub action: [u16; 3],
    /// Accent colour.
    pub accent: [u16; 3],
    /// Error colour.
    pub error: [u16; 3],
    /// Button text, a CSS value.
    pub btn_text: String,
    /// Light-mode switch icon, a CSS value.
    pub theme_sun: String,
    /// Dark-mode switch icon, a CSS value.
    pub theme_moon: String,
}

#[derive(Deserialize)]
struct ClientSecretResponse {
    secret: Option<String>,
}

#[derive(Deserialize)]
struct ErrorBody {
    message: String,
}

/// An authenticated Rauthy API client.
pub struct RauthyApi {
    address: String,
    authorization: Zeroizing<Vec<u8>>,
    scrub: Zeroizing<String>,
}

impl RauthyApi {
    /// A client for the Rauthy listener at `address` (`host:port`),
    /// authenticating with the bootstrap API key.
    pub fn new(address: &str, api_key_secret: &Secret) -> Self {
        let mut authorization = Zeroizing::new(Vec::new());
        authorization.extend_from_slice(b"API-Key ");
        authorization.extend_from_slice(API_KEY_NAME.as_bytes());
        authorization.push(b'$');
        authorization.extend_from_slice(api_key_secret.expose().as_bytes());
        Self {
            address: address.to_string(),
            authorization,
            scrub: Zeroizing::new(api_key_secret.expose().to_string()),
        }
    }

    /// The listener address this client speaks to.
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Every client Rauthy holds.
    pub fn list_clients(&self) -> IdentityResult<Vec<ClientRecord>> {
        self.read_json("GET", "list clients", "clients", "/auth/v1/clients")?
            .ok_or_else(|| self.status_error("list clients", "clients", 404, b""))
    }

    /// One client, or `None` when Rauthy has no client of that id.
    pub fn read_client(&self, id: &str) -> IdentityResult<Option<ClientRecord>> {
        self.read_json("GET", "read client", id, &format!("/auth/v1/clients/{id}"))
    }

    /// Create a client.
    pub fn create_client(&self, client: &NewClient<'_>) -> IdentityResult<WriteResult> {
        self.write("POST", "create client", client.id, "/auth/v1/clients", client)
    }

    /// Replace a client's managed state.
    pub fn update_client(&self, id: &str, update: &ClientUpdate) -> IdentityResult<WriteResult> {
        self.write("PUT", "update client", id, &format!("/auth/v1/clients/{id}"), update)
    }

    /// A client's theme (Rauthy's default when none was written).
    pub fn read_theme(&self, id: &str) -> IdentityResult<ThemeDocument> {
        self.read_json("POST", "read theme", id, &format!("/auth/v1/theme/{id}"))?
            .ok_or_else(|| self.status_error("read theme", id, 404, b""))
    }

    /// Replace a client's theme.
    pub fn write_theme(&self, theme: &ThemeDocument) -> IdentityResult<WriteResult> {
        let path = format!("/auth/v1/theme/{}", theme.client_id);
        self.write("PUT", "write theme", &theme.client_id, &path, theme)
    }

    /// A confidential client's secret, as Rauthy holds it.
    pub fn read_client_secret(&self, id: &str) -> IdentityResult<Secret> {
        let path = format!("/auth/v1/clients/{id}/secret");
        let response = self.exchange("POST", "read client secret", id, &path, None)?;
        let parsed: ClientSecretResponse = serde_json::from_slice(&response.body)
            .map_err(|error| self.invalid("read client secret", id, &error.to_string()))?;
        parsed
            .secret
            .map(Secret::new)
            .ok_or_else(|| self.invalid("read client secret", id, "the client holds no secret"))
    }

    fn read_json<T: DeserializeOwned>(
        &self,
        method: &str,
        operation: &str,
        resource: &str,
        path: &str,
    ) -> IdentityResult<Option<T>> {
        let response = match self.exchange(method, operation, resource, path, None) {
            Err(IdentityError::RauthyStatus { status: 404, .. }) => return Ok(None),
            other => other?,
        };
        serde_json::from_slice(&response.body)
            .map(Some)
            .map_err(|error| self.invalid(operation, resource, &error.to_string()))
    }

    fn write<T: Serialize>(
        &self,
        method: &str,
        operation: &str,
        resource: &str,
        path: &str,
        body: &T,
    ) -> IdentityResult<WriteResult> {
        let body = serde_json::to_vec(body)
            .map_err(|error| self.invalid(operation, resource, &error.to_string()))?;
        match self.exchange(method, operation, resource, path, Some(&body)) {
            Ok(_) => Ok(WriteResult::Applied),
            Err(IdentityError::OutcomeUncertain { reason, .. }) => Ok(WriteResult::Uncertain(reason)),
            Err(error) => Err(error),
        }
    }

    fn exchange(
        &self,
        method: &str,
        operation: &str,
        resource: &str,
        path: &str,
        body: Option<&[u8]>,
    ) -> IdentityResult<HttpResponse> {
        let response = send(&self.address, method, path, Some(&self.authorization), body, REQUEST_TIMEOUT)
            .map_err(|failure| match failure {
                TransportFailure::NotSent(reason) => IdentityError::Unreachable {
                    operation: operation.to_string(),
                    resource: resource.to_string(),
                    address: self.address.clone(),
                    reason,
                },
                TransportFailure::Uncertain(reason) => IdentityError::OutcomeUncertain {
                    operation: operation.to_string(),
                    resource: resource.to_string(),
                    address: self.address.clone(),
                    reason,
                },
            })?;
        if (200..300).contains(&response.status) {
            Ok(response)
        } else {
            Err(self.status_error(operation, resource, response.status, &response.body))
        }
    }

    fn status_error(&self, operation: &str, resource: &str, status: u16, body: &[u8]) -> IdentityError {
        let message = serde_json::from_slice::<ErrorBody>(body)
            .ok()
            .map_or_else(|| String::from_utf8_lossy(body).into_owned(), |parsed| parsed.message);
        IdentityError::RauthyStatus {
            operation: operation.to_string(),
            resource: resource.to_string(),
            status,
            name: status_name(status),
            message: self.scrubbed(&message),
        }
    }

    fn invalid(&self, operation: &str, resource: &str, reason: &str) -> IdentityError {
        IdentityError::RauthyResponseInvalid {
            operation: operation.to_string(),
            resource: resource.to_string(),
            reason: self.scrubbed(reason),
        }
    }

    /// `text` with the API key secret removed and bounded in length.
    fn scrubbed(&self, text: &str) -> String {
        let cleaned = if self.scrub.is_empty() {
            text.to_string()
        } else {
            text.replace(self.scrub.as_str(), "<redacted>")
        };
        cleaned.chars().take(240).collect()
    }
}

/// The stable name of an HTTP status.
pub fn status_name(status: u16) -> &'static str {
    match status {
        400 => "bad_request",
        401 => "unauthorized",
        403 => "forbidden",
        404 => "not_found",
        409 => "conflict",
        429 => "too_many_requests",
        500..=599 => "server_error",
        _ => "unexpected_status",
    }
}

/// Send one HTTP/1.1 request to `address` (`host:port`) and read the whole
/// response. `authorization`, when given, is the `Authorization` header value.
pub fn send(
    address: &str,
    method: &str,
    path: &str,
    authorization: Option<&[u8]>,
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<HttpResponse, TransportFailure> {
    let mut stream = connect(address, timeout)?;
    let mut request = Zeroizing::new(Vec::new());
    request.extend_from_slice(method.as_bytes());
    request.push(b' ');
    request.extend_from_slice(path.as_bytes());
    request.extend_from_slice(b" HTTP/1.1\r\nHost: ");
    request.extend_from_slice(address.as_bytes());
    request.extend_from_slice(
        b"\r\nConnection: close\r\nAccept: application/json\r\nAccept-Encoding: identity\r\n",
    );
    if let Some(value) = authorization {
        request.extend_from_slice(b"Authorization: ");
        request.extend_from_slice(value);
        request.extend_from_slice(b"\r\n");
    }
    let payload = body.unwrap_or_default();
    if body.is_some() {
        request.extend_from_slice(b"Content-Type: application/json\r\n");
    }
    request.extend_from_slice(format!("Content-Length: {}\r\n\r\n", payload.len()).as_bytes());
    request.extend_from_slice(payload);
    stream
        .write_all(&request)
        .and_then(|()| stream.flush())
        .map_err(|error| TransportFailure::Uncertain(format!("sending the request failed: {error}")))?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|error| TransportFailure::Uncertain(format!("reading the response failed: {error}")))?;
    parse_response(&raw).map_err(|reason| TransportFailure::Uncertain(reason.to_string()))
}

fn connect(address: &str, timeout: Duration) -> Result<TcpStream, TransportFailure> {
    let resolved = address
        .to_socket_addrs()
        .map_err(|error| TransportFailure::NotSent(format!("resolving the address failed: {error}")))?;
    let mut last = String::from("the address resolved to nothing");
    for candidate in resolved {
        match TcpStream::connect_timeout(&candidate, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .and_then(|()| stream.set_write_timeout(Some(timeout)))
                    .map_err(|error| TransportFailure::NotSent(format!("setting timeouts failed: {error}")))?;
                return Ok(stream);
            }
            Err(error) => last = format!("connecting to {candidate} failed: {error}"),
        }
    }
    Err(TransportFailure::NotSent(last))
}

/// Parse a complete HTTP/1.1 response read to end of stream.
pub fn parse_response(raw: &[u8]) -> Result<HttpResponse, &'static str> {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("the response ended before its headers did")?;
    let head = std::str::from_utf8(&raw[..split])
        .ok()
        .ok_or("the response headers are not UTF-8")?;
    let rest = &raw[split + 4..];
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or("the response has no status line")?;
    let mut chunked = false;
    let mut length = None;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            let value = value.trim();
            if name.eq_ignore_ascii_case("transfer-encoding") {
                chunked = value.eq_ignore_ascii_case("chunked");
            } else if name.eq_ignore_ascii_case("content-length") {
                length = value.parse::<usize>().ok();
            }
        }
    }
    let body = if chunked {
        dechunk(rest)?
    } else {
        match length {
            Some(length) if rest.len() >= length => rest[..length].to_vec(),
            Some(_) => return Err("the response ended before its body did"),
            None => rest.to_vec(),
        }
    };
    Ok(HttpResponse { status, body })
}

fn dechunk(mut rest: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut body = Vec::new();
    loop {
        let line_end = rest
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or("a chunk size line is unterminated")?;
        let size_text = std::str::from_utf8(&rest[..line_end])
            .ok()
            .ok_or("a chunk size is not UTF-8")?;
        let size_hex = size_text.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_hex, 16)
            .ok()
            .ok_or("a chunk size is not hexadecimal")?;
        rest = &rest[line_end + 2..];
        if size == 0 {
            return Ok(body);
        }
        if rest.len() < size + 2 {
            return Err("the response ended inside a chunk");
        }
        body.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}
