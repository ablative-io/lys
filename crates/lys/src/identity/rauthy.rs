//! Typed Rauthy API requests and responses, over plain HTTP/1.1 to the
//! node-local listener.
//!
//! # Invariants
//!
//! - Only the endpoints configure and health use are spoken, each with a
//!   declared request or response type ([`NewClient`], [`Theme`],
//!   [`RauthyHealth`]). A client record is kept as Rauthy's own JSON object
//!   so that fields configure does not manage pass back unchanged.
//! - Every non-success status is a named error carrying the operation, the
//!   API path and Rauthy's own message ([`IdentityError::RauthyStatus`]).
//! - A request that never reached the listener is `rauthy_unreachable`. A
//!   request that was sent but whose answer did not arrive whole is
//!   uncertain: [`RauthyApi::create_client`] reads the client back and never
//!   sends a second create, and every other write reports
//!   [`Delivery::Uncertain`] so its caller confirms by reading back.
//! - The API key exists only inside the `Authorization` header, built in a
//!   `Zeroizing` buffer. [`RauthyApi`]'s `Debug` prints the origin alone.

use std::fmt;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use zeroize::Zeroizing;

use super::config::Origin;
use super::credentials::Secret;
use super::error::IdentityError;
use super::prepare::API_KEY_NAME;

/// One HTTP response.
#[derive(Debug)]
pub struct Response {
    /// Status code.
    pub status: u16,
    /// Body bytes, de-chunked.
    pub body: Vec<u8>,
}

/// Why an exchange produced no response.
#[derive(Debug)]
pub enum Transport {
    /// Nothing reached the listener: resolving, connecting or sending failed.
    NotSent(std::io::Error),
    /// The request was sent whole; the response did not arrive whole.
    Uncertain(String),
}

/// Whether a write's answer arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// Rauthy answered with success.
    Answered,
    /// The request was sent and its answer lost; read back to learn the outcome.
    Uncertain,
    /// The create's answer was lost, and reading back found the client.
    ResolvedByReadBack,
}

/// A new confidential client: `POST /auth/v1/clients`.
#[derive(Debug, Serialize)]
pub struct NewClient<'a> {
    /// Client id.
    pub id: &'a str,
    /// Display name.
    pub name: &'a str,
    /// Always true for the managed clients.
    pub confidential: bool,
    /// Exact redirect URIs.
    pub redirect_uris: &'a [String],
    /// Exact post-logout redirect URIs.
    pub post_logout_redirect_uris: &'a [String],
}

/// One mode of a client theme, exactly Rauthy's `ThemeCss`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeCss {
    /// Body text, HSL.
    pub text: [u16; 3],
    /// Emphasised text, HSL.
    pub text_high: [u16; 3],
    /// Page background, HSL.
    pub bg: [u16; 3],
    /// Raised background, HSL.
    pub bg_high: [u16; 3],
    /// Action colour, HSL.
    pub action: [u16; 3],
    /// Accent colour, HSL.
    pub accent: [u16; 3],
    /// Error colour, HSL.
    pub error: [u16; 3],
    /// Button text, a CSS value.
    pub btn_text: String,
    /// Light-mode toggle icon, a CSS value.
    pub theme_sun: String,
    /// Dark-mode toggle icon, a CSS value.
    pub theme_moon: String,
}

/// A client's whole theme, exactly Rauthy's `ThemeRequestResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    /// The client the theme belongs to.
    pub client_id: String,
    /// Light mode.
    pub light: ThemeCss,
    /// Dark mode.
    pub dark: ThemeCss,
    /// Corner radius, a CSS value.
    pub border_radius: String,
}

/// `GET /auth/v1/health`.
#[derive(Debug, Deserialize)]
pub struct RauthyHealth {
    /// Rauthy reaches its database.
    pub db_healthy: bool,
    /// Rauthy's Hiqlite cache is healthy.
    pub cache_healthy: bool,
}

#[derive(Deserialize)]
struct Discovery {
    issuer: String,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct Jwk {
    kid: String,
}

/// Rauthy's API at the node-local origin, authenticated by the bootstrap
/// API key prepare declared.
pub struct RauthyApi {
    origin: Origin,
    authorization: Zeroizing<String>,
}

impl fmt::Debug for RauthyApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RauthyApi")
            .field("origin", &self.origin.to_string())
            .field("authorization", &"[redacted]")
            .finish()
    }
}

impl RauthyApi {
    /// Binds the API to `origin` with the bootstrap key's secret.
    pub fn new(origin: Origin, secret: &Secret) -> Self {
        let mut authorization = Zeroizing::new(String::with_capacity(128));
        authorization.push_str("API-Key ");
        authorization.push_str(API_KEY_NAME);
        authorization.push('$');
        authorization.push_str(secret.expose());
        Self {
            origin,
            authorization,
        }
    }

    /// `GET /auth/v1/clients`: every client, as Rauthy's JSON objects.
    pub fn list_clients(&self) -> Result<Vec<Map<String, Value>>, IdentityError> {
        let path = "/auth/v1/clients";
        let response = self.call("list clients", "GET", path, None)?;
        decode(
            "list clients",
            path,
            &ok_body("list clients", path, response)?,
        )
    }

    /// `GET /auth/v1/clients/{id}`, or `None` when Rauthy has no such client.
    pub fn find_client(&self, id: &str) -> Result<Option<Map<String, Value>>, IdentityError> {
        let path = format!("/auth/v1/clients/{id}");
        let response = self.call("read client", "GET", &path, None)?;
        if response.status == 404 {
            return Ok(None);
        }
        decode(
            "read client",
            &path,
            &ok_body("read client", &path, response)?,
        )
        .map(Some)
    }

    /// `POST /auth/v1/clients`. An answer lost after sending is resolved by
    /// reading the client back; a second create is never sent.
    pub fn create_client(&self, client: &NewClient<'_>) -> Result<Delivery, IdentityError> {
        let path = "/auth/v1/clients";
        let body = to_value("create client", path, client)?;
        match self.write("create client", "POST", path, &body)? {
            Delivery::Uncertain => match self.find_client(client.id)? {
                Some(_) => Ok(Delivery::ResolvedByReadBack),
                None => Err(IdentityError::UncertainUnresolved {
                    operation: "create client",
                    resource: format!("/auth/v1/clients/{}", client.id),
                }),
            },
            delivery => Ok(delivery),
        }
    }

    /// `PUT /auth/v1/clients/{id}` with a whole client record.
    pub fn update_client(
        &self,
        id: &str,
        record: &Map<String, Value>,
    ) -> Result<Delivery, IdentityError> {
        let path = format!("/auth/v1/clients/{id}");
        self.write(
            "update client",
            "PUT",
            &path,
            &Value::Object(record.clone()),
        )
    }

    /// `POST /auth/v1/theme/{id}`: the client's theme, or Rauthy's default
    /// when none has been written.
    pub fn read_theme(&self, client_id: &str) -> Result<Theme, IdentityError> {
        let path = format!("/auth/v1/theme/{client_id}");
        let response = self.call("read theme", "POST", &path, None)?;
        decode(
            "read theme",
            &path,
            &ok_body("read theme", &path, response)?,
        )
    }

    /// `PUT /auth/v1/theme/{id}` with a whole theme.
    pub fn write_theme(&self, theme: &Theme) -> Result<Delivery, IdentityError> {
        let path = format!("/auth/v1/theme/{}", theme.client_id);
        let body = to_value("write theme", &path, theme)?;
        self.write("write theme", "PUT", &path, &body)
    }

    fn write(
        &self,
        operation: &'static str,
        method: &'static str,
        path: &str,
        body: &Value,
    ) -> Result<Delivery, IdentityError> {
        match self.call(operation, method, path, Some(body)) {
            Ok(response) => ok_body(operation, path, response).map(|_| Delivery::Answered),
            Err(IdentityError::UncertainOutcome { .. }) => Ok(Delivery::Uncertain),
            Err(error) => Err(error),
        }
    }

    fn call(
        &self,
        operation: &'static str,
        method: &'static str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Response, IdentityError> {
        let bytes = body.map(Value::to_string);
        let headers = [("Authorization", self.authorization.as_str())];
        exchange(
            &self.origin,
            method,
            path,
            &headers,
            bytes.as_deref().map(str::as_bytes),
        )
        .map_err(|transport| transport_error(transport, operation, path, &self.origin))
    }
}

/// `GET /auth/v1/health`. Rauthy answers 500 with the same body when a
/// check fails, so both statuses are read.
pub fn rauthy_health(origin: &Origin) -> Result<RauthyHealth, IdentityError> {
    let path = "/auth/v1/health";
    let response = exchange(origin, "GET", path, &[], None)
        .map_err(|transport| transport_error(transport, "read health", path, origin))?;
    if response.status != 200 && response.status != 500 {
        return Err(status_error("read health", path, &response));
    }
    decode("read health", path, &response.body)
}

/// The issuer Rauthy publishes, and the key ids of its signing keys: the
/// public identity a restart or restore must preserve.
pub fn issuer_identity(origin: &Origin) -> Result<(String, Vec<String>), IdentityError> {
    let discovery_path = "/auth/v1/.well-known/openid-configuration";
    let response = exchange(origin, "GET", discovery_path, &[], None)
        .map_err(|transport| transport_error(transport, "read issuer", discovery_path, origin))?;
    let discovery: Discovery = decode(
        "read issuer",
        discovery_path,
        &ok_body("read issuer", discovery_path, response)?,
    )?;
    let certs_path = "/auth/v1/oidc/certs";
    let response = exchange(origin, "GET", certs_path, &[], None)
        .map_err(|transport| transport_error(transport, "read signing keys", certs_path, origin))?;
    let jwks: Jwks = decode(
        "read signing keys",
        certs_path,
        &ok_body("read signing keys", certs_path, response)?,
    )?;
    let mut kids: Vec<String> = jwks.keys.into_iter().map(|key| key.kid).collect();
    kids.sort();
    Ok((discovery.issuer, kids))
}

/// One plain HTTP/1.1 exchange with `Connection: close`.
pub fn exchange(
    origin: &Origin,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> Result<Response, Transport> {
    let address = (origin.host.as_str(), origin.effective_port())
        .to_socket_addrs()
        .map_err(Transport::NotSent)?
        .next()
        .ok_or_else(|| {
            Transport::NotSent(std::io::Error::other("the host resolved to no address"))
        })?;
    let mut stream = TcpStream::connect(address).map_err(Transport::NotSent)?;
    let authority = origin.authority();
    let mut head = Zeroizing::new(String::with_capacity(512));
    for piece in [method, " ", path, " HTTP/1.1\r\nHost: ", authority.as_str()] {
        head.push_str(piece);
    }
    head.push_str("\r\nConnection: close\r\nAccept: application/json\r\n");
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    if let Some(body) = body {
        head.push_str("Content-Type: application/json\r\nContent-Length: ");
        head.push_str(&body.len().to_string());
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    // A send that fails leaves the request incomplete, and Rauthy acts on
    // no incomplete request: that is still "not sent".
    stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(body.unwrap_or_default()))
        .and_then(|()| stream.flush())
        .map_err(Transport::NotSent)?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|error| Transport::Uncertain(error.to_string()))?;
    parse_response(&raw).map_err(Transport::Uncertain)
}

fn parse_response(raw: &[u8]) -> Result<Response, String> {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("no complete response head arrived")?;
    let head = std::str::from_utf8(&raw[..split])
        .map_err(|error| format!("response head is not UTF-8: {error}"))?;
    let rest = &raw[split + 4..];
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or("no status line")?;
    let mut chunked = false;
    let mut length = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding") {
            chunked = value.eq_ignore_ascii_case("chunked");
        } else if name.eq_ignore_ascii_case("content-length") {
            let parsed = value
                .parse::<usize>()
                .map_err(|error| format!("content-length {value:?}: {error}"))?;
            length = Some(parsed);
        }
    }
    let body = match (chunked, length) {
        (true, _) => dechunk(rest)?,
        (false, Some(length)) => rest
            .get(..length)
            .ok_or("the body ended before its content-length")?
            .to_vec(),
        (false, None) => rest.to_vec(),
    };
    Ok(Response { status, body })
}

fn dechunk(mut rest: &[u8]) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    loop {
        let end = rest
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or("a chunk size line is unterminated")?;
        let line = std::str::from_utf8(&rest[..end]).map_err(|error| error.to_string())?;
        let digits = line.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(digits, 16)
            .map_err(|error| format!("chunk size {digits:?}: {error}"))?;
        rest = &rest[end + 2..];
        if size == 0 {
            return Ok(body);
        }
        body.extend_from_slice(rest.get(..size).ok_or("a chunk ended early")?);
        rest = rest.get(size + 2..).ok_or("a chunk is unterminated")?;
    }
}

fn transport_error(
    transport: Transport,
    operation: &'static str,
    path: &str,
    origin: &Origin,
) -> IdentityError {
    match transport {
        Transport::NotSent(source) => IdentityError::Unreachable {
            service: "rauthy",
            operation,
            resource: path.to_string(),
            address: origin.authority(),
            source,
        },
        Transport::Uncertain(reason) => IdentityError::UncertainOutcome {
            operation,
            resource: path.to_string(),
            reason,
        },
    }
}

fn ok_body(
    operation: &'static str,
    path: &str,
    response: Response,
) -> Result<Vec<u8>, IdentityError> {
    if (200..300).contains(&response.status) {
        Ok(response.body)
    } else {
        Err(status_error(operation, path, &response))
    }
}

/// Rauthy's error body is `{"error": ..., "message": ...}`; the message is
/// carried whole, and never includes what the request sent.
fn status_error(operation: &'static str, path: &str, response: &Response) -> IdentityError {
    let message = serde_json::from_slice::<Value>(&response.body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| String::from_utf8_lossy(&response.body).into_owned());
    IdentityError::RauthyStatus {
        operation,
        resource: path.to_string(),
        status: response.status,
        message,
    }
}

fn decode<T: DeserializeOwned>(
    operation: &'static str,
    path: &str,
    body: &[u8],
) -> Result<T, IdentityError> {
    serde_json::from_slice(body).map_err(|error| IdentityError::RauthyResponse {
        operation,
        resource: path.to_string(),
        reason: error.to_string(),
    })
}

fn to_value<T: Serialize>(
    operation: &'static str,
    path: &str,
    body: &T,
) -> Result<Value, IdentityError> {
    serde_json::to_value(body).map_err(|error| IdentityError::RauthyResponse {
        operation,
        resource: path.to_string(),
        reason: format!("request body did not serialise: {error}"),
    })
}
