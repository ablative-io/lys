//! The finishing address stays on the consent answer until its client takes it.
use super::{Apps, Asking, escaped, malformed};
use crate::ServerError;
use axum::extract::{ConnectInfo, Extension};
use axum::http::{HeaderMap, header};
use axum::response::{Html, IntoResponse, Response};
use std::net::{IpAddr, SocketAddr};

pub(super) fn address(
    apps: &Apps,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: &HeaderMap,
) -> Result<IpAddr, ServerError> {
    let peer = peer
        .ok_or_else(|| malformed("the consent request has no peer address"))?
        .0
        .0
        .ip();
    crate::sign_in_address::resolve(peer, headers, &apps.trusted).map(IpAddr::to_canonical)
}

pub(super) struct Return<'a> {
    pub(super) redirect: &'a str,
    pub(super) pairs: &'a [(&'a str, &'a str)],
    pub(super) name: &'a str,
    pub(super) native: bool,
    pub(super) same_host: bool,
    pub(super) expires_at: Option<u64>,
    pub(super) at: u64,
}

pub(super) fn elsewhere(name: &str, same_host: bool) {
    if !same_host {
        tracing::warn!(client = name, "sign-in finished elsewhere");
    }
}

pub(super) fn finish(
    apps: &Apps,
    asking: &Asking,
    browser: IpAddr,
    pairs: &[(&str, &str)],
    expires_at: Option<u64>,
) -> Result<Response, ServerError> {
    let native = asking.client_id == super::native::CLIENT;
    let (name, client_host) = super::native::finishing_client(apps, asking)?;
    let same_host = client_host == Some(browser);
    elsewhere(&name, same_host);
    page(Return {
        redirect: &asking.redirect_uri,
        pairs,
        name: &name,
        native,
        same_host,
        expires_at,
        at: apps.state.sessions.now()?,
    })
}

pub(super) fn page(given: Return<'_>) -> Result<Response, ServerError> {
    let mut url = reqwest::Url::parse(given.redirect)
        .map_err(|_| malformed("the registered redirect address is unreadable"))?;
    for (key, value) in given.pairs {
        url.query_pairs_mut().append_pair(key, value);
    }
    let address = escaped(url.as_str());
    let name = escaped(if given.native {
        "lys sign-in"
    } else {
        given.name
    });
    let refresh = if given.same_host {
        format!("<meta http-equiv=\"refresh\" content=\"0;url={address}\">")
    } else {
        String::new()
    };
    let field=given.expires_at.map_or_else(String::new, |expires_at|format!("<p>This address works once, for {} seconds, until the authorization code expires.</p><label>Finishing address <input readonly value=\"{address}\"></label>", expires_at.saturating_sub(given.at)));
    let instructions = if given.native {
        format!(
            "On the computer waiting for sign-in, run <code>lys sign-in --finish '{address}'</code>."
        )
    } else {
        "When this browser is on another computer, give this address to the Claude Code session that started the sign-in. Its complete_authentication tool asks for the URL from the page the sign-in opened. Keep it out of chat.".to_owned()
    };
    let policy = if given.native {
        "default-src 'none'; form-action 'self'"
    } else {
        "default-src 'none'; style-src 'unsafe-inline'; form-action 'self' https: http:"
    };
    Ok(([(header::CACHE_CONTROL,"no-store"),(header::X_FRAME_OPTIONS,"DENY"),
        (header::CONTENT_SECURITY_POLICY,policy)],
        Html(format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">{refresh}<title>Finish sign-in</title></head><body><h1>Finish sign-in</h1>{field}<p><a href=\"{address}\">Return to {name}</a></p><p>{instructions}</p></body></html>"))).into_response())
}
