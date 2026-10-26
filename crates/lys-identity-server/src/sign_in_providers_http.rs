//! Issuer calls and provider proofs keep their wait until the response is read.
//! Refusal words retain service context without exposing the request's secrets.

use serde_json::Value;

use super::{Provider, SignInProviders, unnamed};
use crate::error::ServerError;
use crate::sign_in::unreached;

impl SignInProviders {
    pub(super) async fn send(
        &self,
        method: reqwest::Method,
        url: impl reqwest::IntoUrl,
        body: Option<&Value>,
    ) -> Result<Value, ServerError> {
        let url = url.into_url().map_err(|error| ServerError::SignInProvidersUnavailable { reason:error.to_string() })?;
        let mut call = crate::waits::Call::begin("issuer", &url.origin().ascii_serialization(), method.as_str(), url.path())?;
        call.until(self.send_untracked(method, url, body)).await
            .map_err(|reason| ServerError::SignInProvidersUnavailable { reason })?
    }

    async fn send_untracked(
        &self,
        method: reqwest::Method,
        url: reqwest::Url,
        body: Option<&Value>,
    ) -> Result<Value, ServerError> {
        let mut request = self
            .client
            .request(method, url)
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
                        unreached(error)
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
                        unreached(error)
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

    /// Prove `request`'s client id at the provider's own sign-in address,
    /// asked as a sign-in would ask it with `redirect`. A provider that
    /// answers with a refusal is refused with its own words.
    pub(super) async fn prove(
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
        let mut call = crate::waits::Call::begin("issuer", &url.origin().ascii_serialization(), "prove provider", provider.name())?;
        let proving = async {
        let answer = self.probe.get(url).send().await.map_err(|error| {
            ServerError::SignInProvidersUnavailable {
                reason: format!(
                    "{} could not be reached to check the client id: {}",
                    provider.name(),
                    unreached(error)
                ),
            }
        })?;
        let status = answer.status();
        if status.is_client_error() || status.is_server_error() {
            let text = answer.text().await.unwrap_or_default();
            return Err(refused(status.as_u16(), provider_words(&text)));
        }
        Ok(())
        };
        call.until(proving).await.map_err(|reason| ServerError::SignInProvidersUnavailable { reason })?
    }
}

/// The issuer's message from a refusal body, or its first line of text,
/// whole, with the issuer's product name said as Lys's sign-in service:
/// nothing a person reads names the issuer.
fn message(text: &str) -> String {
    let message = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|body| {
            body.get("message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| text.lines().next().unwrap_or_default().to_owned());
    unnamed(&message)
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
