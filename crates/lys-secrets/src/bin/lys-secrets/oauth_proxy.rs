//! OAuth at the proxy: an access token that has expired, or is about to,
//! is refreshed from the refresh token before the call is forwarded, and
//! the refreshed grant is resealed in the store. The holder sees neither
//! token. A drop may also revoke the grant with the provider.

use std::sync::PoisonError;

use axum::http::StatusCode;
use lys_secrets::{Broker, HandleId, OAuthGrant, Secret, SecretsError, Ticket};

use crate::files::{Layout, Route, now_ms};
use crate::serve::Shared;
use crate::spice::Grants;

type Failed = (StatusCode, SecretsError);

fn upstream(context: &'static str, reason: String) -> Failed {
    (
        StatusCode::BAD_GATEWAY,
        SecretsError::Encoding { context, reason },
    )
}

/// A form body, URL-encoded. It holds tokens, so it is a secret itself.
fn form(pairs: &[(&'static str, Secret)]) -> Result<Secret, Failed> {
    let mut url = reqwest::Url::parse("http://form.invalid/")
        .map_err(|error| upstream("form encoding", error.to_string()))?;
    {
        let mut query = url.query_pairs_mut();
        for (name, value) in pairs {
            let value = std::str::from_utf8(value.expose())
                .map_err(|_utf8| upstream("form encoding", format!("{name} is not UTF-8")))?;
            query.append_pair(name, value);
        }
    }
    Ok(Secret::from_slice(
        url.query().unwrap_or_default().as_bytes(),
    ))
}

async fn post_form(
    shared: &Shared,
    endpoint: &str,
    pairs: &[(&'static str, Secret)],
    hidden: &[&Secret],
) -> Result<(StatusCode, Vec<u8>), Failed> {
    let body = form(pairs)?;
    let answer = shared
        .client
        .post(endpoint)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body.expose().to_vec())
        .send()
        .await
        .map_err(|error| upstream("provider call", scrub(&error.to_string(), hidden)))?;
    let status = answer.status();
    let bytes = answer
        .bytes()
        .await
        .map_err(|error| upstream("provider answer", scrub(&error.to_string(), hidden)))?;
    Ok((status, bytes.to_vec()))
}

fn scrub(text: &str, hidden: &[&Secret]) -> String {
    hidden.iter().fold(text.to_owned(), |text, secret| {
        match std::str::from_utf8(secret.expose()) {
            Ok(token) if !token.is_empty() => text.replace(token, "[redacted]"),
            _ => text,
        }
    })
}

/// The grant with a live access token, refreshing it first when it needs
/// it, and every token it held before, for redaction.
pub async fn live(
    shared: &Shared,
    ticket: &Ticket,
    mut grant: OAuthGrant,
) -> Result<(OAuthGrant, Vec<Secret>), Failed> {
    let now = now_ms();
    if !grant.needs_refresh(now) {
        return Ok((grant, Vec::new()));
    }
    let retired: Vec<Secret> = grant
        .tokens()
        .into_iter()
        .map(|token| Secret::from_slice(token.expose()))
        .collect();
    let hidden: Vec<&Secret> = retired.iter().collect();
    let endpoint = grant.provenance().token_endpoint.clone();
    let (status, answer) = post_form(shared, &endpoint, &grant.refresh_form(), &hidden).await?;
    if !status.is_success() {
        return Err(upstream(
            "token endpoint",
            format!("the refresh was refused with {status}"),
        ));
    }
    grant
        .apply_refresh(&answer, now)
        .map_err(|error| (StatusCode::BAD_GATEWAY, error))?;
    {
        let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
        broker.refreshed(ticket, &grant)
    }
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    Ok((grant, retired))
}

/// Asks the provider to revoke `grant`. Answers whether it confirmed.
pub async fn revoke(client: &reqwest::Client, grant: &OAuthGrant) -> bool {
    let Some(endpoint) = grant.provenance().revocation_endpoint.clone() else {
        return false;
    };
    let Ok(body) = form(&grant.revocation_form()) else {
        return false;
    };
    client
        .post(endpoint)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body.expose().to_vec())
        .send()
        .await
        .is_ok_and(|answer| answer.status().is_success())
}

/// After the handle `id` is dropped, asks the provider to revoke the OAuth
/// grant behind it and records whether it confirmed.
///
/// # Errors
///
/// When the handle is not on an OAuth grant, and the broker's refusals.
pub fn revoke_after_drop(broker: &mut Broker<Grants>, id: &HandleId) -> Result<bool, SecretsError> {
    let grant = broker
        .oauth_grant_of(id)?
        .ok_or_else(|| SecretsError::Encoding {
            context: "revoking upstream",
            reason: format!("handle {id} is not on an OAuth grant"),
        })?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|source| SecretsError::Io {
            context: "starting the revocation call".to_owned(),
            source,
        })?;
    let confirmed = runtime.block_on(revoke(&reqwest::Client::new(), &grant));
    broker.record_upstream_revocation(id, confirmed)?;
    Ok(confirmed)
}

/// Seals the OAuth grant in `sealed` as `name` and routes it to
/// `upstream` in the authorization header. Answers the grant's provider
/// subject.
///
/// # Errors
///
/// When the grant does not read, and the broker's refusals.
pub fn seal(
    broker: &mut Broker<Grants>,
    layout: &Layout,
    name: &str,
    owner: &str,
    upstream: String,
    sealed: &Secret,
) -> Result<String, SecretsError> {
    let grant = OAuthGrant::from_sealed(sealed)?;
    broker.seal_oauth(name, owner, &grant)?;
    layout.add_route(
        name,
        Route {
            upstream,
            header: "authorization".to_owned(),
            prefix: "Bearer ".to_owned(),
            spend_header: None,
        },
    )?;
    Ok(grant.provenance().provider_subject.clone())
}
