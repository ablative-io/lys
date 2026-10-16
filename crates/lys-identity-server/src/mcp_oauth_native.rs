//! The built-in native client carries its approving session's person authority.

use axum::http::{HeaderMap, header};
use lys_identity::Actor;

use super::{digest, held};
use crate::error::ServerError;
use crate::error_provider::ProviderError;
use crate::mcp_oauth_store::{Kind, Store};
use crate::routes::AppState;

pub(super) const CLIENT: &str = "lys-cli";
pub(super) const PREFIX: &str = "lys-cli.";

pub(super) fn redirect(uri: &str) -> bool {
    reqwest::Url::parse(uri).is_ok_and(|url| {
        url.scheme() == "http"
            && matches!(url.host_str(), Some("127.0.0.1" | "[::1]"))
            && url.port().is_some()
            && url.path() == "/lys/sign-in"
            && url.query().is_none()
            && url.fragment().is_none()
            && url.username().is_empty()
            && url.password().is_none()
    })
}

pub(super) fn session(state: &AppState, subject: &str) -> Result<Actor, ServerError> {
    let id = subject
        .strip_prefix("cli-session:")
        .ok_or(ServerError::Provider(ProviderError::TokenUnknown))?;
    state
        .sessions
        .live(|_| true)?
        .into_iter()
        .find(|session| session.id == id)
        .map(|session| session.actor)
        .ok_or(ServerError::Provider(ProviderError::TokenUnknown))
}

pub(crate) fn actor(state: &AppState, headers: &HeaderMap) -> Result<Option<Actor>, ServerError> {
    if let Some(actor) = crate::mcp_endpoint::reading_actor() {
        return Ok(Some(actor));
    }
    let Some(text) = headers
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
    else {
        return Ok(None);
    };
    let Some(pass) = text
        .strip_prefix("Bearer ")
        .filter(|pass| pass.starts_with(PREFIX))
    else {
        return Ok(None);
    };
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1
        || [
            header::COOKIE.as_str(),
            "lys-operator",
            "lys-agent-pass",
            "lys-agent-signature",
            "lys-grant-token",
        ]
        .iter()
        .any(|name| headers.contains_key(*name))
    {
        return Err(ServerError::NotSignedIn);
    }
    let path = state.estate_plan_file.with_file_name("connected-apps.json");
    let store = Store::open(&path)?;
    let issued = store
        .token(&digest(pass), Kind::Access, crate::session::now())
        .filter(|issued| issued.client_id == CLIENT)
        .ok_or(ServerError::NotSignedIn)?;
    session(state, &issued.agent)
        .map(Some)
        .map_err(|error| match error {
            ServerError::Provider(ProviderError::TokenUnknown) => ServerError::NotSignedIn,
            other => other,
        })
}

pub(super) fn person(apps: &super::Apps, subject: &str) -> Result<String, ServerError> {
    let actor = session(&apps.state, subject)?;
    crate::routes::with_directory(&apps.state, |directory| {
        crate::read_api::own_person(directory.projection()?, &actor)
            .map(|person| person.to_string())
    })
}

pub(super) fn known(apps: &super::Apps, client: &str, uri: &str) -> Result<String, ServerError> {
    if client == CLIENT {
        if !redirect(uri) {
            return Err(ServerError::Provider(ProviderError::RedirectUnregistered));
        }
        return Ok("Lys CLI".to_owned());
    }
    let store = held(&apps.store)?;
    let app = store
        .app(client)
        .ok_or(ServerError::Provider(ProviderError::RedirectUnregistered))?;
    if !app.redirect_uris.iter().any(|kept| kept == uri) {
        return Err(ServerError::Provider(ProviderError::RedirectUnregistered));
    }
    Ok(app.name.clone())
}

pub(super) fn finishing_client(
    apps: &super::Apps,
    asking: &super::Asking,
) -> Result<(String, Option<std::net::IpAddr>), ServerError> {
    if asking.client_id == CLIENT {
        return Ok((CLIENT.to_owned(), asking.client_host));
    }
    let store = held(&apps.store)?;
    let app = store
        .app(&asking.client_id)
        .ok_or_else(|| super::malformed("the registered client is no longer held"))?;
    Ok((app.name.clone(), app.client_host))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_builtin_cli_accepts_only_a_numeric_loopback_redirect() {
        assert!(super::redirect("http://127.0.0.1:12345/lys/sign-in"));
        assert!(!super::redirect("https://another.example/lys/sign-in"));
        assert!(!super::redirect("http://localhost:12345/lys/sign-in"));
        assert!(!super::redirect(
            "http://127.0.0.1:12345/lys/sign-in?outside=1"
        ));
    }
}
