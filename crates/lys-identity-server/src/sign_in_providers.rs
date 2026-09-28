//! The sign-in providers: Google, Microsoft and GitHub, set in the issuer
//! by the administrator from the Connections screen, through this service
//! and never by hand. The client id and secret entered go to the issuer and
//! nowhere else: they are not kept here, not logged and not answered back.
//! The list answers each provider's name, kind and client id.
//!
//! The service speaks to the issuer's administration API with the API key
//! the installation wrote for it, read once from its file at start. Setting
//! a provider looks its endpoints up at the issuer's own discovery document
//! (GitHub has none, so its endpoints are the fixed ones), then creates the
//! provider or replaces the one of the same name, so setting it again with
//! a new secret is the same act.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{AppState, signed_in};

/// Where the issuer's administration API is and what this service calls it with.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignInProvidersSettings {
    /// The issuer's API base: the URL its `providers` routes hang under.
    pub api: String,
    /// The file holding the API key as the issuer writes it, `name$secret`,
    /// mode 0600.
    pub api_key_file: PathBuf,
}

/// The issuer's administration API as this service calls it.
pub struct SignInProviders {
    api: String,
    authorization: String,
    client: reqwest::Client,
}

/// The most characters a client secret carries, the issuer's own limit.
const SECRET_MAX: usize = 256;

/// The sign-in providers this installation offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Google accounts.
    Google,
    /// Microsoft work and school accounts, of one tenant.
    Microsoft,
    /// GitHub accounts.
    GitHub,
}

/// Every provider offered, in the order shown.
pub const OFFERED: [Provider; 3] = [Provider::Google, Provider::Microsoft, Provider::GitHub];

impl Provider {
    /// The name the provider is kept under at the issuer.
    pub fn name(self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::Microsoft => "Microsoft",
            Self::GitHub => "GitHub",
        }
    }

    /// The provider kept under `name`, if it is one offered.
    pub fn from_name(name: &str) -> Option<Self> {
        OFFERED.into_iter().find(|provider| provider.name() == name)
    }
}

/// A provider as the issuer holds it, without its secret.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProviderView {
    /// The issuer's id for it.
    pub id: String,
    /// Which offered provider it is, null for one set some other way.
    pub provider: Option<Provider>,
    /// Its name at the issuer.
    pub name: String,
    /// Whether people may sign in with it.
    pub enabled: bool,
    /// The client id registered with the provider.
    pub client_id: String,
}

/// The answer of `GET /sign-in-providers`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProvidersView {
    /// Every provider the issuer holds.
    pub providers: Vec<ProviderView>,
    /// The providers this installation offers to set.
    #[schema(value_type = Vec<Provider>)]
    pub offered: [Provider; 3],
}

/// What the administrator sends to set a provider. Never printed.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = SignInProviderSetBody)]
pub(crate) struct SetBody {
    provider: Provider,
    client_id: String,
    client_secret: String,
    #[serde(default)]
    tenant: String,
}

/// A provider as the issuer lists it.
#[derive(Deserialize)]
struct Listed {
    id: String,
    name: String,
    enabled: bool,
    client_id: String,
}

/// What the issuer's lookup answers for a discovery document.
#[derive(Deserialize)]
struct Lookup {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    jwks_endpoint: Option<String>,
    scope: String,
    use_pkce: bool,
    client_secret_basic: bool,
    client_secret_post: bool,
}

impl SignInProviders {
    /// The API `settings` names, with the key read from its file.
    ///
    /// # Errors
    ///
    /// `ConfigInvalid` when the key file cannot be read or holds no key.
    pub fn open(settings: &SignInProvidersSettings) -> Result<Self, ServerError> {
        let text = std::fs::read_to_string(&settings.api_key_file).map_err(|error| {
            ServerError::ConfigInvalid {
                reason: format!(
                    "the sign-in providers API key file {} cannot be read: {}",
                    settings.api_key_file.display(),
                    error.kind()
                ),
            }
        })?;
        let key = text.trim();
        if key.is_empty() || !key.contains('$') || key.chars().any(char::is_whitespace) {
            return Err(ServerError::ConfigInvalid {
                reason:
                    "the sign-in providers API key file does not hold a key of the form name$secret"
                        .to_owned(),
            });
        }
        Ok(Self {
            api: settings.api.trim_end_matches('/').to_owned(),
            authorization: format!("API-Key {key}"),
            client: reqwest::Client::new(),
        })
    }

    /// Call `path` at the issuer's API; a refusal is named with its status
    /// and its message, never with what was sent.
    async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, ServerError> {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.api))
            .header(reqwest::header::AUTHORIZATION, &self.authorization)
            .header(reqwest::header::ACCEPT, "application/json");
        if let Some(body) = body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body.to_string());
        }
        let answer =
            request
                .send()
                .await
                .map_err(|error| ServerError::SignInProvidersUnavailable {
                    reason: format!("the issuer's API could not be reached: {error}"),
                })?;
        let status = answer.status().as_u16();
        let text =
            answer
                .text()
                .await
                .map_err(|error| ServerError::SignInProvidersUnavailable {
                    reason: format!("the issuer's answer could not be read: {error}"),
                })?;
        if !(200..300).contains(&status) {
            return Err(ServerError::SignInProvidersRefused {
                status,
                reason: message(&text),
            });
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|error| ServerError::SignInProvidersUnavailable {
            reason: format!("the issuer answered {status} with a body that is not JSON: {error}"),
        })
    }

    async fn listed(&self) -> Result<Vec<Listed>, ServerError> {
        let answer = self.call(reqwest::Method::POST, "/providers", None).await?;
        serde_json::from_value(answer).map_err(|error| ServerError::SignInProvidersUnavailable {
            reason: format!("the issuer's provider list could not be read: {error}"),
        })
    }

    async fn lookup(&self, issuer: &str) -> Result<Lookup, ServerError> {
        let asked = json!({ "issuer": issuer, "metadata_url": null });
        let answer = self
            .call(reqwest::Method::POST, "/providers/lookup", Some(&asked))
            .await?;
        serde_json::from_value(answer).map_err(|error| ServerError::SignInProvidersUnavailable {
            reason: format!("the issuer's lookup of {issuer} could not be read: {error}"),
        })
    }

    /// Create the provider `request` names, or replace the one of the same
    /// name, answering the list as it stands after.
    async fn upsert(&self, name: &str, request: &Value) -> Result<Vec<Listed>, ServerError> {
        let held = self.listed().await?;
        match held.into_iter().find(|listed| listed.name == name) {
            Some(listed) => {
                self.call(
                    reqwest::Method::PUT,
                    &format!("/providers/{}", listed.id),
                    Some(request),
                )
                .await?;
            }
            None => {
                self.call(reqwest::Method::POST, "/providers/create", Some(request))
                    .await?;
            }
        }
        self.listed().await
    }
}

/// The issuer's message from a refusal body, or its first line of text,
/// bounded so a refusal never carries a page.
fn message(text: &str) -> String {
    let message = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|body| {
            body.get("message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| text.lines().next().unwrap_or_default().to_owned());
    message.chars().take(300).collect()
}

/// The sign-in provider routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/sign-in-providers", get(list).post(set))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn api(state: &AppState) -> Result<&SignInProviders, ServerError> {
    state
        .sign_in_providers
        .as_ref()
        .ok_or_else(|| ServerError::SignInProvidersUnavailable {
            reason: "the configuration names no sign_in_providers".to_owned(),
        })
}

fn view(listed: Listed) -> ProviderView {
    ProviderView {
        provider: Provider::from_name(&listed.name),
        id: listed.id,
        name: listed.name,
        enabled: listed.enabled,
        client_id: listed.client_id,
    }
}

fn answer(listed: Vec<Listed>) -> ProvidersView {
    ProvidersView {
        providers: listed.into_iter().map(view).collect(),
        offered: OFFERED,
    }
}

/// Refuse a client id or secret the provider could not have issued: empty,
/// or carrying whitespace or a control character.
fn credential(name: &str, text: &str, most: usize) -> Result<(), ServerError> {
    if text.is_empty() {
        return Err(malformed(format!("{name} is empty")));
    }
    if text.chars().count() > most {
        return Err(malformed(format!(
            "{name} is longer than {most} characters"
        )));
    }
    if text.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(malformed(format!(
            "{name} carries whitespace or a control character"
        )));
    }
    Ok(())
}

/// The tenant a Microsoft provider signs in against: its id or its domain.
fn tenant(text: &str) -> Result<&str, ServerError> {
    let tenant = text.trim();
    if tenant.is_empty() {
        return Err(malformed(
            "a Microsoft provider names its tenant: the directory (tenant) id or the tenant's domain",
        ));
    }
    if !tenant
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.'))
    {
        return Err(malformed("the tenant is letters, digits, hyphens and dots"));
    }
    Ok(tenant)
}

/// The provider as the issuer's API takes it, from what was entered and
/// what the issuer's lookup found.
fn request_of(provider: Provider, body: &SetBody, found: Option<Lookup>) -> Value {
    let (typ, found) = match (provider, found) {
        (Provider::Google, Some(found)) => ("google", found),
        (Provider::Microsoft, Some(found)) => ("oidc", found),
        (Provider::GitHub, _) | (Provider::Google | Provider::Microsoft, None) => (
            "github",
            Lookup {
                issuer: "https://github.com".to_owned(),
                authorization_endpoint: "https://github.com/login/oauth/authorize".to_owned(),
                token_endpoint: "https://github.com/login/oauth/access_token".to_owned(),
                userinfo_endpoint: "https://api.github.com/user".to_owned(),
                jwks_endpoint: None,
                scope: "user:email".to_owned(),
                use_pkce: false,
                client_secret_basic: false,
                client_secret_post: true,
            },
        ),
    };
    json!({
        "name": provider.name(),
        "typ": typ,
        "enabled": true,
        "issuer": found.issuer,
        "authorization_endpoint": found.authorization_endpoint,
        "token_endpoint": found.token_endpoint,
        "userinfo_endpoint": found.userinfo_endpoint,
        "jwks_endpoint": found.jwks_endpoint,
        "use_pkce": found.use_pkce,
        "client_secret_basic": found.client_secret_basic,
        "client_secret_post": found.client_secret_post,
        "auto_onboarding": true,
        "auto_link": false,
        "client_id": body.client_id,
        "client_secret": body.client_secret,
        "scope": found.scope,
        "admin_claim_path": null,
        "admin_claim_value": null,
        "mfa_claim_path": null,
        "mfa_claim_value": null,
    })
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ProvidersView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let listed = api(&state)?.listed().await?;
    Ok(Json(answer(listed)))
}

async fn set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<SetBody>, JsonRejection>,
) -> Result<Json<ProvidersView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    credential("client_id", &body.client_id, SECRET_MAX)?;
    credential("client_secret", &body.client_secret, SECRET_MAX)?;
    let api = api(&state)?;
    let found = match body.provider {
        Provider::Google => Some(api.lookup("https://accounts.google.com").await?),
        Provider::Microsoft => {
            let tenant = tenant(&body.tenant)?;
            Some(
                api.lookup(&format!("https://login.microsoftonline.com/{tenant}/v2.0"))
                    .await?,
            )
        }
        Provider::GitHub => {
            if !body.tenant.trim().is_empty() {
                return Err(malformed("a tenant is named for Microsoft only"));
            }
            None
        }
    };
    let request = request_of(body.provider, &body, found);
    let listed = api.upsert(body.provider.name(), &request).await?;
    Ok(Json(answer(listed)))
}
