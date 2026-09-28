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
//!
//! Before a provider is saved it is proved: the provider's own sign-in
//! address is asked with the new client id and Lys's redirect address, and
//! a provider that refuses it is refused `ProviderRefused` with the
//! provider's own words, so a mistyped client id is found on the
//! Connections screen and not at a person's first sign-in. The redirect
//! address the screen shows to paste is the one the service sends, Lys's
//! own origin with the issuer's provider callback path.

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

/// Where the sign-in providers answer: their public origins, unless the
/// configuration names stand-ins, as a development or test service does.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderOrigins {
    /// Google's accounts origin.
    pub google: String,
    /// Microsoft's sign-in origin, under which each tenant answers.
    pub microsoft: String,
    /// GitHub's origin.
    pub github: String,
}

impl Default for ProviderOrigins {
    fn default() -> Self {
        Self {
            google: "https://accounts.google.com".to_owned(),
            microsoft: "https://login.microsoftonline.com".to_owned(),
            github: "https://github.com".to_owned(),
        }
    }
}

/// The issuer's administration API as this service calls it.
pub struct SignInProviders {
    api: String,
    authorization: String,
    client: reqwest::Client,
    origins: ProviderOrigins,
    probe: reqwest::Client,
}

/// The most characters a client secret carries, the issuer's own limit.
const SECRET_MAX: usize = 256;

/// The sign-in providers this installation offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
pub struct ProvidersView {
    /// Every provider the issuer holds.
    pub providers: Vec<ProviderView>,
    /// The providers this installation offers to set.
    pub offered: [Provider; 3],
    /// The redirect address to register with each provider: Lys's own.
    pub redirect_address: String,
}

/// What the administrator sends to set a provider. Never printed.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetBody {
    provider: Provider,
    client_id: String,
    client_secret: String,
    #[serde(default)]
    tenant: String,
}

/// A provider as the issuer lists it.
#[derive(Deserialize)]
pub(crate) struct Listed {
    /// The issuer's id for it.
    pub(crate) id: String,
    /// Its name at the issuer.
    pub(crate) name: String,
    /// Whether people may sign in with it.
    pub(crate) enabled: bool,
    /// The client id registered with the provider.
    pub(crate) client_id: String,
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
    pub fn open(
        settings: &SignInProvidersSettings,
        origins: Option<ProviderOrigins>,
    ) -> Result<Self, ServerError> {
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
        let probe = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| ServerError::ConfigInvalid {
                reason: format!("the provider check could not be made: {error}"),
            })?;
        Ok(Self {
            api: settings.api.trim_end_matches('/').to_owned(),
            authorization: format!("API-Key {key}"),
            client: reqwest::Client::new(),
            origins: origins.unwrap_or_default(),
            probe,
        })
    }

    /// Call `path` at the issuer's API; a refusal is named with its status
    /// and its message, never with what was sent.
    pub(crate) async fn call(
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
                    reason: format!(
                        "the issuer's API could not be reached: {}",
                        error.without_url()
                    ),
                })?;
        let status = answer.status().as_u16();
        let text =
            answer
                .text()
                .await
                .map_err(|error| ServerError::SignInProvidersUnavailable {
                    reason: format!(
                        "the issuer's answer could not be read: {}",
                        error.without_url()
                    ),
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

    pub(crate) async fn listed(&self) -> Result<Vec<Listed>, ServerError> {
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

    /// Prove `request`'s client id at the provider's own sign-in address,
    /// asked as a sign-in would ask it with `redirect`. A provider that
    /// answers with a refusal is refused with its own words.
    async fn prove(
        &self,
        provider: Provider,
        request: &Value,
        redirect: &str,
    ) -> Result<(), ServerError> {
        let field = |name: &str| {
            request
                .get(name)
                .and_then(Value::as_str)
                .unwrap_or_default()
        };
        let refused = |status: u16, reason: String| ServerError::ProviderRefused {
            provider: provider.name(),
            status,
            reason,
        };
        let mut url = reqwest::Url::parse(field("authorization_endpoint"))
            .map_err(|error| refused(0, format!("its sign-in address is not one: {error}")))?;
        url.query_pairs_mut()
            .append_pair("client_id", field("client_id"))
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", redirect)
            .append_pair("scope", field("scope"));
        let answer = self.probe.get(url).send().await.map_err(|error| {
            ServerError::SignInProvidersUnavailable {
                reason: format!(
                    "{} could not be reached to check the client id: {}",
                    provider.name(),
                    error.without_url()
                ),
            }
        })?;
        let status = answer.status();
        if status.is_client_error() || status.is_server_error() {
            let text = answer.text().await.unwrap_or_default();
            return Err(refused(status.as_u16(), provider_words(&text)));
        }
        Ok(())
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
/// bounded so a refusal never carries a page, with the issuer's product name
/// said as Lys's sign-in service: nothing a person reads names the issuer.
fn message(text: &str) -> String {
    let message = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|body| {
            body.get("message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| text.lines().next().unwrap_or_default().to_owned());
    let bounded: String = message.chars().take(300).collect();
    unnamed(&bounded)
}

/// `text` with every spelling of the issuer's product name said as Lys's
/// sign-in service.
pub(crate) fn unnamed(text: &str) -> String {
    const NAME: &str = "rauthy";
    let lower = text.to_ascii_lowercase();
    let mut said = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(found) = lower.get(at..).and_then(|rest| rest.find(NAME)) {
        let start = at + found;
        said.push_str(text.get(at..start).unwrap_or_default());
        said.push_str("the sign-in service");
        at = start + NAME.len();
    }
    said.push_str(text.get(at..).unwrap_or_default());
    said
}

/// A provider's own words from a refusal page: its JSON error description,
/// or the page's title, or its first line.
fn provider_words(text: &str) -> String {
    let from_json = serde_json::from_str::<Value>(text).ok().and_then(|body| {
        ["error_description", "message", "error"]
            .iter()
            .find_map(|name| body.get(*name).and_then(Value::as_str).map(str::to_owned))
    });
    let from_title = || {
        let start = text.find("<title>")? + "<title>".len();
        let end = text.get(start..)?.find("</title>")?;
        text.get(start..start + end)
            .map(|title| title.trim().to_owned())
    };
    from_json
        .or_else(from_title)
        .unwrap_or_else(|| text.lines().next().unwrap_or_default().trim().to_owned())
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

pub(crate) fn api(state: &AppState) -> Result<&SignInProviders, ServerError> {
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

fn answer(listed: Vec<Listed>, redirect: &str) -> ProvidersView {
    ProvidersView {
        providers: listed.into_iter().map(view).collect(),
        offered: OFFERED,
        redirect_address: redirect.to_owned(),
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
fn request_of(provider: Provider, body: &SetBody, found: Option<Lookup>, github: &str) -> Value {
    let (typ, found) = match (provider, found) {
        (Provider::Google, Some(found)) => ("google", found),
        (Provider::Microsoft, Some(found)) => ("oidc", found),
        (Provider::GitHub, _) | (Provider::Google | Provider::Microsoft, None) => (
            "github",
            Lookup {
                issuer: github.to_owned(),
                authorization_endpoint: format!("{github}/login/oauth/authorize"),
                token_endpoint: format!("{github}/login/oauth/access_token"),
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
    Ok(Json(answer(listed, state.sign_in.callback())))
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
    let origins = &api.origins;
    let found = match body.provider {
        Provider::Google => Some(api.lookup(&origins.google).await?),
        Provider::Microsoft => {
            let tenant = tenant(&body.tenant)?;
            Some(
                api.lookup(&format!("{}/{tenant}/v2.0", origins.microsoft))
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
    let request = request_of(body.provider, &body, found, &origins.github);
    let redirect = state.sign_in.callback();
    api.prove(body.provider, &request, redirect).await?;
    let listed = api.upsert(body.provider.name(), &request).await?;
    Ok(Json(answer(listed, redirect)))
}
