//! Typed requests to Rauthy's API: clients, themes and health. Every call
//! names its operation and resource in its error, carries a status as a
//! number, and never puts a body or the API key into a diagnostic. After an
//! uncertain outcome the caller reads the resource back rather than
//! repeating the write.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::credentials::Secret;
use super::error::{IdentityError, IdentityResult};

/// The Rauthy client of one deployment.
pub struct Rauthy {
    agent: ureq::Agent,
    origin: String,
    authorization: Option<Secret>,
}

/// A client as Rauthy answers it, in the fields the reconciliation owns.
/// Unknown fields are ignored on read; every field here is sent back whole
/// on update, so an update never blanks a value Rauthy holds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Client {
    /// The client id.
    pub id: String,
    /// Its display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Whether it holds a secret.
    pub confidential: bool,
    /// The exact redirect URIs.
    pub redirect_uris: Vec<String>,
    /// The post-logout redirect URIs.
    #[serde(default)]
    pub post_logout_redirect_uris: Option<Vec<String>>,
    /// The allowed CORS origins.
    #[serde(default)]
    pub allowed_origins: Option<Vec<String>>,
    /// Whether it may sign in.
    pub enabled: bool,
    /// The grant types it may use.
    pub flows_enabled: Vec<String>,
    /// The access token signing algorithm.
    pub access_token_alg: String,
    /// The id token signing algorithm.
    pub id_token_alg: String,
    /// Seconds an authorization code lives.
    pub auth_code_lifetime: i32,
    /// Seconds an access token lives.
    pub access_token_lifetime: i32,
    /// The scopes it may request.
    pub scopes: Vec<String>,
    /// The scopes it gets without asking.
    pub default_scopes: Vec<String>,
    /// The PKCE challenges it must use.
    #[serde(default)]
    pub challenges: Option<Vec<String>>,
    /// Whether every sign-in needs MFA.
    pub force_mfa: bool,
    /// Its home page.
    #[serde(default, rename = "client_uri")]
    pub uri: Option<String>,
    /// Its contacts.
    #[serde(default)]
    pub contacts: Option<Vec<String>>,
    /// Its back-channel logout URI.
    #[serde(default)]
    pub backchannel_logout_uri: Option<String>,
    /// The group prefix it is restricted to.
    #[serde(default)]
    pub restrict_group_prefix: Option<String>,
    /// Its custom claims.
    #[serde(default)]
    pub claims: Option<Value>,
    /// Whether custom claims sit at the token's root.
    #[serde(default)]
    pub claims_at_root: bool,
}

/// What `POST /clients` takes.
#[derive(Debug, Clone, Serialize)]
pub struct NewClient {
    /// The client id.
    pub id: String,
    /// Its display name.
    pub name: Option<String>,
    /// Whether it holds a secret.
    pub confidential: bool,
    /// The exact redirect URIs.
    pub redirect_uris: Vec<String>,
}

/// Seven HSL colours plus the button text and the two theme icons, as
/// Rauthy's `ThemeCss` takes them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeCss {
    /// Body text.
    pub text: [u16; 3],
    /// Emphasised text.
    pub text_high: [u16; 3],
    /// Page background.
    pub bg: [u16; 3],
    /// Raised background.
    pub bg_high: [u16; 3],
    /// The action colour.
    pub action: [u16; 3],
    /// The accent colour.
    pub accent: [u16; 3],
    /// The error colour.
    pub error: [u16; 3],
    /// The button text colour, a CSS value.
    pub btn_text: String,
    /// The sun icon colour, a CSS value.
    pub theme_sun: String,
    /// The moon icon colour, a CSS value.
    pub theme_moon: String,
}

/// A client's theme as Rauthy takes and answers it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Theme {
    /// The client the theme belongs to.
    pub client_id: String,
    /// The light palette.
    pub light: ThemeCss,
    /// The dark palette.
    pub dark: ThemeCss,
    /// The border radius, a CSS value.
    pub border_radius: String,
}

/// What `/auth/v1/health` answers.
#[derive(Debug, Clone, Deserialize)]
pub struct Health {
    /// Whether the database answers.
    pub db_healthy: bool,
    /// Whether the cache answers.
    pub cache_healthy: bool,
}

impl Theme {
    /// Whether two themes look the same: the palettes and the radius, not
    /// the client id Rauthy stamps on its default.
    #[must_use]
    pub fn same_look(&self, other: &Self) -> bool {
        self.light == other.light
            && self.dark == other.dark
            && self.border_radius == other.border_radius
    }
}

impl Rauthy {
    /// A client for the deployment at `origin`, authenticating with the
    /// API key header when one is given.
    #[must_use]
    pub fn new(origin: &str, authorization: Option<Secret>) -> Self {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(10)))
            .build();
        Self {
            agent: ureq::Agent::new_with_config(config),
            origin: origin.trim_end_matches('/').to_string(),
            authorization,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/auth/v1{path}", self.origin)
    }

    /// The health answer.
    pub fn health(&self) -> IdentityResult<Health> {
        let operation = "read rauthy's health";
        let resource = "rauthy".to_string();
        let mut answer = self
            .agent
            .get(self.url("/health"))
            .call()
            .map_err(|error| transport(operation, &resource, &error))?;
        let status = answer.status().as_u16();
        if status != 200 {
            return Err(IdentityError::Status {
                operation,
                resource,
                status,
            });
        }
        answer
            .body_mut()
            .read_json::<Health>()
            .map_err(|error| IdentityError::Answer {
                operation,
                resource,
                what: "the health record",
                detail: kind_of(&error).to_string(),
            })
    }

    /// The client `id`, or `None` when Rauthy holds no such client.
    pub fn client(&self, id: &str) -> IdentityResult<Option<Client>> {
        let operation = "read the client";
        let mut answer = self.send(
            self.agent.get(self.url(&format!("/clients/{id}"))),
            operation,
            id,
        )?;
        match answer.status().as_u16() {
            200 => answer
                .body_mut()
                .read_json::<Client>()
                .map(Some)
                .map_err(|error| IdentityError::Answer {
                    operation,
                    resource: id.to_string(),
                    what: "a client record",
                    detail: kind_of(&error).to_string(),
                }),
            404 => Ok(None),
            status => Err(IdentityError::Status {
                operation,
                resource: id.to_string(),
                status,
            }),
        }
    }

    /// Create a client. A transport failure after the request was sent is
    /// returned as [`IdentityError::Transport`]; the caller reads the client
    /// back to learn whether it exists rather than creating it twice.
    pub fn create_client(&self, client: &NewClient) -> IdentityResult<()> {
        let operation = "create the client";
        let answer = self.send_json(
            self.agent.post(self.url("/clients")),
            operation,
            &client.id,
            client,
        )?;
        match answer.status().as_u16() {
            200 | 201 => Ok(()),
            status => Err(IdentityError::Status {
                operation,
                resource: client.id.clone(),
                status,
            }),
        }
    }

    /// Replace the client's settings with `client`.
    pub fn update_client(&self, client: &Client) -> IdentityResult<()> {
        let operation = "update the client";
        let answer = self.send_json(
            self.agent.put(self.url(&format!("/clients/{}", client.id))),
            operation,
            &client.id,
            client,
        )?;
        match answer.status().as_u16() {
            200 => Ok(()),
            status => Err(IdentityError::Status {
                operation,
                resource: client.id.clone(),
                status,
            }),
        }
    }

    /// The theme of client `id` as Rauthy holds it; Rauthy answers its
    /// default theme for a client that has none of its own. The read is a
    /// `POST` with no body, which is how Rauthy serves the record as JSON
    /// (its `GET` renders CSS).
    pub fn theme(&self, id: &str) -> IdentityResult<Theme> {
        let operation = "read the theme";
        let request = self.agent.post(self.url(&format!("/theme/{id}")));
        let request = match &self.authorization {
            Some(key) => request.header("Authorization", key.expose()),
            None => request,
        };
        let mut answer = request
            .send_empty()
            .map_err(|error| transport(operation, id, &error))?;
        match answer.status().as_u16() {
            200 => answer
                .body_mut()
                .read_json::<Theme>()
                .map_err(|error| IdentityError::Answer {
                    operation,
                    resource: id.to_string(),
                    what: "a theme record",
                    detail: kind_of(&error).to_string(),
                }),
            status => Err(IdentityError::Status {
                operation,
                resource: id.to_string(),
                status,
            }),
        }
    }

    /// Write the theme: a `PUT` that creates or replaces it.
    pub fn write_theme(&self, theme: &Theme) -> IdentityResult<()> {
        let operation = "write the theme";
        let answer = self.send_json(
            self.agent.put(self.url(&format!("/theme/{}", theme.client_id))),
            operation,
            &theme.client_id,
            theme,
        )?;
        match answer.status().as_u16() {
            200 | 201 => Ok(()),
            status => Err(IdentityError::Status {
                operation,
                resource: theme.client_id.clone(),
                status,
            }),
        }
    }

    fn send(
        &self,
        request: ureq::RequestBuilder<ureq::typestate::WithoutBody>,
        operation: &'static str,
        resource: &str,
    ) -> IdentityResult<ureq::http::Response<ureq::Body>> {
        let request = match &self.authorization {
            Some(key) => request.header("Authorization", key.expose()),
            None => request,
        };
        request
            .call()
            .map_err(|error| transport(operation, resource, &error))
    }

    fn send_json<T: Serialize>(
        &self,
        request: ureq::RequestBuilder<ureq::typestate::WithBody>,
        operation: &'static str,
        resource: &str,
        body: &T,
    ) -> IdentityResult<ureq::http::Response<ureq::Body>> {
        let request = match &self.authorization {
            Some(key) => request.header("Authorization", key.expose()),
            None => request,
        };
        request
            .send_json(body)
            .map_err(|error| transport(operation, resource, &error))
    }
}

/// A transport error, named by kind and never by body.
fn transport(operation: &'static str, resource: &str, error: &ureq::Error) -> IdentityError {
    let detail = match error {
        ureq::Error::Timeout(_) => "the request timed out".to_string(),
        ureq::Error::ConnectionFailed => "the connection could not be made".to_string(),
        ureq::Error::HostNotFound => "the host is not known".to_string(),
        ureq::Error::Io(io) => format!("the connection failed: {}", io.kind()),
        other => format!("the transport failed: {}", kind_of(other)),
    };
    IdentityError::Transport {
        operation,
        resource: resource.to_string(),
        detail,
    }
}

fn kind_of(error: &ureq::Error) -> &'static str {
    match error {
        ureq::Error::StatusCode(_) => "status",
        ureq::Error::Http(_) => "http",
        ureq::Error::BadUri(_) => "bad uri",
        ureq::Error::Protocol(_) => "protocol",
        ureq::Error::Tls(_) => "tls",
        ureq::Error::Json(_) => "json",
        ureq::Error::TooManyRedirects => "too many redirects",
        ureq::Error::RedirectFailed => "redirect failed",
        ureq::Error::Decompress(..) => "decompress",
        ureq::Error::BodyExceedsLimit(_) => "body exceeds limit",
        _ => "other",
    }
}
