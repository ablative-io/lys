//! Typed requests to Rauthy's admin API over loopback HTTP, its statuses as
//! named errors, and read-back after an outcome lys cannot see.
//!
//! A request that was written but whose response never arrived may or may
//! not have taken effect. Such a failure is [`ErrorKind::RauthyUncertain`],
//! never a plain transport error, and every writing call here resolves it by
//! reading the resource back before deciding anything: a create is retried
//! only when the read-back proves the client does not exist.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

use super::credentials::Credential;
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::prepare::API_KEY_NAME;

const TIMEOUT: Duration = Duration::from_secs(10);

/// One mode of a Rauthy client theme, exactly as the admin API carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeCss {
    /// Body text, as hue, saturation and lightness.
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
    /// Button text, as a CSS value.
    pub btn_text: String,
    /// Theme switch sun icon, as a CSS value.
    pub theme_sun: String,
    /// Theme switch moon icon, as a CSS value.
    pub theme_moon: String,
}

/// A client's whole theme, as `POST /auth/v1/theme/{id}` returns it and
/// `PUT /auth/v1/theme/{id}` takes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theme {
    /// The client the theme belongs to.
    pub client_id: String,
    /// Light mode.
    pub light: ThemeCss,
    /// Dark mode.
    pub dark: ThemeCss,
    /// Border radius, as a CSS value.
    pub border_radius: String,
}

/// `POST /auth/v1/clients`.
#[derive(Debug, Clone, Serialize)]
pub struct NewClient {
    /// Client id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Whether the client holds a secret.
    pub confidential: bool,
    /// Exact redirect URIs.
    pub redirect_uris: Vec<String>,
    /// Exact post-logout redirect URIs.
    pub post_logout_redirect_uris: Vec<String>,
}

/// Rauthy's `/auth/v1/health` answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Health {
    /// Whether Rauthy reaches its database.
    pub db_healthy: bool,
    /// Whether Rauthy's cache answers.
    pub cache_healthy: bool,
}

#[derive(Debug, Deserialize)]
struct SecretResponse {
    secret: Option<String>,
}

/// A raw HTTP answer.
struct Answer {
    status: u16,
    body: Vec<u8>,
}

/// Rauthy's admin API at one loopback address.
pub struct RauthyApi {
    host: String,
    port: u16,
    credential: Option<Credential>,
}

impl RauthyApi {
    /// The API at `admin_url` (`http://host:port`), presenting the bootstrap
    /// API key secret when one is given.
    pub fn new(admin_url: &str, credential: Option<Credential>) -> IdentityResult<Self> {
        let authority = admin_url
            .strip_prefix("http://")
            .map(|rest| rest.trim_end_matches('/'))
            .ok_or_else(|| {
                IdentityError::new(
                    ErrorKind::IssuerInvalid,
                    "reach Rauthy",
                    "issuer.admin_url",
                    "expected http://host:port",
                )
            })?;
        let (host, port) = authority
            .rsplit_once(':')
            .and_then(|(host, port)| Some((host, port.parse::<u16>().ok()?)))
            .unwrap_or((authority, 80));
        Ok(Self {
            host: host.to_string(),
            port,
            credential,
        })
    }

    fn exchange(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        resource: &str,
    ) -> IdentityResult<Answer> {
        let unreachable = |detail: String| {
            IdentityError::new(ErrorKind::RauthyUnreachable, "connect", resource, detail)
        };
        let address = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|error| unreachable(error.to_string()))?
            .next()
            .ok_or_else(|| unreachable(format!("{} resolves to no address", self.host)))?;
        let mut stream = TcpStream::connect_timeout(&address, TIMEOUT)
            .map_err(|error| unreachable(format!("{address}: {error}")))?;
        let uncertain = |detail: String| {
            IdentityError::new(ErrorKind::RauthyUncertain, "request", resource, detail)
        };
        stream
            .set_read_timeout(Some(TIMEOUT))
            .and_then(|()| stream.set_write_timeout(Some(TIMEOUT)))
            .map_err(|error| unreachable(error.to_string()))?;
        let mut request = Zeroizing::new(Vec::new());
        request.extend_from_slice(
            format!(
                "{method} {path} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\nAccept: application/json\r\n",
                self.host, self.port
            )
            .as_bytes(),
        );
        if let Some(credential) = &self.credential {
            request.extend_from_slice(b"Authorization: API-Key ");
            request.extend_from_slice(API_KEY_NAME.as_bytes());
            request.push(b'$');
            request.extend_from_slice(credential.expose().as_bytes());
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
            .map_err(|error| uncertain(format!("writing the request: {error}")))?;
        let mut raw = Vec::new();
        stream
            .read_to_end(&mut raw)
            .map_err(|error| uncertain(format!("reading the response: {error}")))?;
        parse_answer(&raw).ok_or_else(|| uncertain("the response was cut short".to_string()))
    }

    fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        resource: &str,
    ) -> IdentityResult<Answer> {
        let encoded = body.map(Value::to_string);
        let answer = self.exchange(
            method,
            path,
            encoded.as_deref().map(str::as_bytes),
            resource,
        )?;
        status_error(answer.status, &answer.body, resource).map_or(Ok(answer), Err)
    }

    fn json<T: for<'de> Deserialize<'de>>(answer: &Answer, resource: &str) -> IdentityResult<T> {
        serde_json::from_slice(&answer.body).map_err(|error| {
            IdentityError::new(
                ErrorKind::RauthyUnexpected,
                "read response",
                resource,
                error.to_string(),
            )
        })
    }

    /// `GET /auth/v1/health`. Rauthy answers 500 with the same body when a
    /// part is unhealthy, so both statuses are read.
    pub fn health(&self) -> IdentityResult<Health> {
        let answer = self.exchange("GET", "/auth/v1/health", None, "rauthy")?;
        if answer.status != 200 && answer.status != 500 {
            return Err(IdentityError::new(
                ErrorKind::RauthyUnexpected,
                "read health",
                "rauthy",
                format!("status {}", answer.status),
            ));
        }
        Self::json(&answer, "rauthy")
    }

    /// `GET /auth/v1/clients`.
    pub fn list_clients(&self) -> IdentityResult<Vec<Value>> {
        let answer = self.call("GET", "/auth/v1/clients", None, "clients")?;
        Self::json(&answer, "clients")
    }

    /// `GET /auth/v1/clients/{id}`, `None` when Rauthy has no such client.
    pub fn get_client(&self, id: &str) -> IdentityResult<Option<Value>> {
        let path = format!("/auth/v1/clients/{id}");
        let answer = self.exchange("GET", &path, None, id)?;
        if answer.status == 404 {
            return Ok(None);
        }
        if let Some(error) = status_error(answer.status, &answer.body, id) {
            return Err(error);
        }
        Self::json(&answer, id).map(Some)
    }

    /// Creates a client once. A lost response is resolved by reading the
    /// client back; the create is repeated only when the read-back proves it
    /// did not happen. Returns the client and whether read-back decided it.
    pub fn create_client(&self, request: &NewClient) -> IdentityResult<(Value, bool)> {
        let body = serde_json::to_value(request).map_err(|error| {
            IdentityError::new(
                ErrorKind::RenderFailed,
                "encode client",
                &request.id,
                error.to_string(),
            )
        })?;
        for attempt in 0..2 {
            match self.call("POST", "/auth/v1/clients", Some(&body), &request.id) {
                Ok(answer) => return Self::json(&answer, &request.id).map(|v| (v, attempt > 0)),
                Err(error) if error.kind() == ErrorKind::RauthyUncertain => {
                    if let Some(existing) = self.get_client(&request.id)? {
                        return Ok((existing, true));
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Err(IdentityError::new(
            ErrorKind::RauthyUncertain,
            "create client",
            &request.id,
            "two creates went unanswered and read-back found no client",
        ))
    }

    /// `PUT /auth/v1/clients/{id}`, confirmed by reading the client back.
    pub fn update_client(&self, id: &str, body: &Value) -> IdentityResult<Value> {
        let path = format!("/auth/v1/clients/{id}");
        match self.call("PUT", &path, Some(body), id) {
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::RauthyUncertain => {}
            Err(error) => return Err(error),
        }
        self.get_client(id)?.ok_or_else(|| {
            IdentityError::new(
                ErrorKind::ReadBackMismatch,
                "update client",
                id,
                "the client is gone",
            )
        })
    }

    /// `POST /auth/v1/clients/{id}/secret`: the confidential client's secret.
    pub fn client_secret(&self, id: &str) -> IdentityResult<Credential> {
        let path = format!("/auth/v1/clients/{id}/secret");
        let answer = self.call("POST", &path, None, id)?;
        let response: SecretResponse = Self::json(&answer, id)?;
        let secret = response.secret.ok_or_else(|| {
            IdentityError::new(
                ErrorKind::RauthyUnexpected,
                "read client secret",
                id,
                "the client holds no secret; it is not confidential",
            )
        })?;
        Ok(Credential::received(
            &format!("{id}-client-secret"),
            Zeroizing::new(secret),
        ))
    }

    /// `POST /auth/v1/theme/{id}`: the client's theme, or Rauthy's default
    /// when none is set.
    pub fn get_theme(&self, id: &str) -> IdentityResult<Theme> {
        let path = format!("/auth/v1/theme/{id}");
        let answer = self.call("POST", &path, None, id)?;
        Self::json(&answer, id)
    }

    /// `PUT /auth/v1/theme/{id}`, confirmed by reading the theme back.
    pub fn put_theme(&self, theme: &Theme) -> IdentityResult<Theme> {
        let path = format!("/auth/v1/theme/{}", theme.client_id);
        let body = serde_json::to_value(theme).map_err(|error| {
            IdentityError::new(
                ErrorKind::RenderFailed,
                "encode theme",
                &theme.client_id,
                error.to_string(),
            )
        })?;
        match self.call("PUT", &path, Some(&body), &theme.client_id) {
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::RauthyUncertain => {}
            Err(error) => return Err(error),
        }
        let stored = self.get_theme(&theme.client_id)?;
        if &stored != theme {
            return Err(IdentityError::new(
                ErrorKind::ReadBackMismatch,
                "set theme",
                &theme.client_id,
                "the theme read back differs from the theme written",
            ));
        }
        Ok(stored)
    }
}

pub(super) fn status_error(status: u16, body: &[u8], resource: &str) -> Option<IdentityError> {
    let kind = match status {
        200..=299 => return None,
        400 => ErrorKind::RauthyBadRequest,
        401 => ErrorKind::RauthyUnauthorized,
        403 => ErrorKind::RauthyForbidden,
        500..=599 => ErrorKind::RauthyServerError,
        _ => ErrorKind::RauthyUnexpected,
    };
    let message = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .map(|text| text.chars().take(200).collect::<String>())
        .unwrap_or_default();
    Some(IdentityError::new(
        kind,
        "call Rauthy",
        resource,
        format!("status {status} {message}").trim_end().to_string(),
    ))
}

fn parse_answer(raw: &[u8]) -> Option<Answer> {
    let split = raw.windows(4).position(|window| window == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&raw[..split]).ok()?;
    let rest = &raw[split + 4..];
    let mut lines = head.split("\r\n");
    let status = lines.next()?.split(' ').nth(1)?.parse::<u16>().ok()?;
    let mut chunked = false;
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':')?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding") && value.eq_ignore_ascii_case("chunked") {
            chunked = true;
        } else if name.eq_ignore_ascii_case("content-length") {
            length = Some(value.parse::<usize>().ok()?);
        }
    }
    let body = if chunked {
        dechunk(rest)?
    } else if let Some(length) = length {
        rest.get(..length)?.to_vec()
    } else {
        rest.to_vec()
    };
    Some(Answer { status, body })
}

fn dechunk(mut rest: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let line_end = rest.windows(2).position(|window| window == b"\r\n")?;
        let size_text = std::str::from_utf8(&rest[..line_end]).ok()?;
        let size = usize::from_str_radix(size_text.split(';').next()?.trim(), 16).ok()?;
        rest = &rest[line_end + 2..];
        if size == 0 {
            return Some(body);
        }
        body.extend_from_slice(rest.get(..size)?);
        rest = rest.get(size + 2..)?;
    }
}
