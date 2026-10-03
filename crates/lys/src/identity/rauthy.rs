//! Typed requests to Rauthy's admin API over loopback HTTP, its statuses as
//! named errors, and read-back after an outcome lys cannot see.
//!
//! A request that was written but whose response never arrived may or may
//! not have taken effect. Such a failure is [`ErrorKind::RauthyUncertain`],
//! never a plain transport error, and every writing call here resolves it by
//! reading the resource back before deciding anything: a create is retried
//! only when the read-back proves the client does not exist.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

use super::credentials::Credential;
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::loopback_http::{self, Authority, Failure, Request, Response};
use super::prepare::API_KEY_NAME;

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

/// Rauthy's admin API at one loopback address.
pub struct RauthyApi {
    authority: Authority,
    credential: Option<Credential>,
}

impl RauthyApi {
    /// The API at `admin_url` (`http://host:port`), presenting the bootstrap
    /// API key secret when one is given.
    pub fn new(admin_url: &str, credential: Option<Credential>) -> IdentityResult<Self> {
        let authority = Authority::from_http_url(admin_url).map_err(|rule| {
            IdentityError::new(
                ErrorKind::IssuerInvalid,
                "reach Rauthy",
                "issuer.admin_url",
                rule,
            )
        })?;
        Ok(Self {
            authority,
            credential,
        })
    }

    fn exchange(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        resource: &str,
    ) -> IdentityResult<Response> {
        let mut authorization = Zeroizing::new(Vec::new());
        let mut headers: Vec<(&str, &[u8])> = vec![("Accept", b"application/json".as_slice())];
        if let Some(credential) = &self.credential {
            authorization.extend_from_slice(b"API-Key ");
            authorization.extend_from_slice(API_KEY_NAME.as_bytes());
            authorization.push(b'$');
            authorization.extend_from_slice(credential.expose().as_bytes());
            headers.push(("Authorization", authorization.as_slice()));
        }
        if body.is_some() {
            headers.push(("Content-Type", b"application/json".as_slice()));
        }
        let request = Request {
            method,
            path,
            headers: &headers,
            body: body.unwrap_or_default(),
        };
        loopback_http::exchange(&self.authority, &request).map_err(|failure| match failure {
            Failure::Unreachable(detail) => {
                IdentityError::new(ErrorKind::RauthyUnreachable, "connect", resource, detail)
            }
            Failure::Uncertain(detail) | Failure::Malformed(detail) => {
                IdentityError::new(ErrorKind::RauthyUncertain, "request", resource, detail)
            }
        })
    }

    fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        resource: &str,
    ) -> IdentityResult<Response> {
        let encoded = body.map(Value::to_string);
        let answer = self.exchange(
            method,
            path,
            encoded.as_deref().map(str::as_bytes),
            resource,
        )?;
        status_error(answer.status, &answer.body, resource).map_or(Ok(answer), Err)
    }

    fn json<T: for<'de> Deserialize<'de>>(answer: &Response, resource: &str) -> IdentityResult<T> {
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

    /// `PUT /auth/v1/password_policy`: the sign-in service's password
    /// policy set to `policy` whole, answering the policy it stored. The
    /// sign-in service answers the stored policy to this call alone, so the
    /// answer is the read-back.
    pub fn put_password_policy(&self, policy: &Value) -> IdentityResult<Value> {
        let answer = self.call(
            "PUT",
            "/auth/v1/password_policy",
            Some(policy),
            "password_policy",
        )?;
        Self::json(&answer, "password_policy")
    }

    /// `GET /auth/v1/api_keys`: every API key, by name and rights. Rauthy
    /// never answers a key's secret here.
    pub fn list_api_keys(&self) -> IdentityResult<Vec<Value>> {
        let answer = self.call("GET", "/auth/v1/api_keys", None, "api_keys")?;
        let listed: Value = Self::json(&answer, "api_keys")?;
        listed
            .get("keys")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                IdentityError::new(
                    ErrorKind::RauthyUnexpected,
                    "read API keys",
                    "api_keys",
                    "the answer lists no keys",
                )
            })
    }

    /// `POST /auth/v1/api_keys`: a new key made from `request`, answering
    /// its token, the key's name, a dollar sign and its secret.
    pub fn create_api_key(&self, name: &str, request: &Value) -> IdentityResult<Credential> {
        let answer = self.call("POST", "/auth/v1/api_keys", Some(request), name)?;
        Self::token(&answer, name)
    }

    /// `PUT /auth/v1/api_keys/{name}`: the key's rights set to `request`'s.
    /// What was stored is for the caller to read back.
    pub fn update_api_key(&self, name: &str, request: &Value) -> IdentityResult<()> {
        let path = format!("/auth/v1/api_keys/{name}");
        self.call("PUT", &path, Some(request), name).map(drop)
    }

    /// `PUT /auth/v1/api_keys/{name}/secret`: a new secret for the key,
    /// answering its token. The old secret stops working.
    pub fn renew_api_key_secret(&self, name: &str) -> IdentityResult<Credential> {
        let path = format!("/auth/v1/api_keys/{name}/secret");
        let answer = self.call("PUT", &path, None, name)?;
        Self::token(&answer, name)
    }

    /// An API key's token from an answer's plain body, checked to name
    /// the key it was asked for.
    fn token(answer: &Response, name: &str) -> IdentityResult<Credential> {
        let text = std::str::from_utf8(&answer.body).map_err(|error| {
            IdentityError::new(
                ErrorKind::RauthyUnexpected,
                "read API key",
                name,
                error.to_string(),
            )
        })?;
        let token = Zeroizing::new(text.trim().to_owned());
        if !token
            .strip_prefix(name)
            .is_some_and(|rest| rest.len() > 1 && rest.starts_with('$'))
        {
            return Err(IdentityError::new(
                ErrorKind::RauthyUnexpected,
                "read API key",
                name,
                "the answer is not this key's token",
            ));
        }
        Ok(Credential::received(&format!("{name}-api-key"), token))
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
        .unwrap_or_default();
    Some(IdentityError::new(
        kind,
        "call Rauthy",
        resource,
        format!("status {status} {message}").trim_end().to_string(),
    ))
}
