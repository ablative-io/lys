//! A person's own secrets, by their word through a trusted screen service:
//! add one with its value and where it is used, replace its value, and
//! retire it. Each change is carried under an operation id the person's
//! screen made once for it (see the library's `values`), so a resend of the
//! same change answers what was recorded and applies nothing.
//!
//! A value travels only in the signed body of the request that seals it,
//! and is never returned, logged or written anywhere but the sealed store.
//! A body that does not read is refused in fixed words, never with the
//! parser's own, which can quote what it could not read.

use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::{HeaderName, StatusCode};
use lys_secrets::{EntryClass, OwnerChanged, Secret, SecretsError};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::callers::{Caller, caller, refused};
use crate::files::Route;
use crate::serve::{MAX_BODY, Shared, on_broker};

pub(crate) type Answer = Result<Json<Value>, (StatusCode, String)>;

/// The words a body that does not read is refused with.
fn malformed(expected: &str) -> (StatusCode, String) {
    (
        StatusCode::BAD_REQUEST,
        format!("RequestMalformed: the body is not {expected}\n"),
    )
}

/// The person a trusted screen service vouches for, and the body read in
/// the shape `expected` names; any other caller is refused.
pub(crate) async fn vouched<T: DeserializeOwned>(
    shared: &Arc<Shared>,
    request: Request,
    expected: &str,
) -> Result<(Caller, T), (StatusCode, String)> {
    let (parts, body) = request.into_parts();
    // Guarded: read before the caller is known, as the caller's signature covers it.
    let body: Bytes = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|_unread| malformed(expected))?;
    let who = caller(shared, &parts, &body).await?;
    if who.via.is_none() {
        return Err((
            StatusCode::FORBIDDEN,
            "NotAdmitted: a value is given only by a person, through a trusted screen service\n"
                .to_owned(),
        ));
    }
    let asked = serde_json::from_slice(&body).map_err(|_unread| malformed(expected))?;
    Ok((who, asked))
}

/// Whether the change was applied now, or answered from an earlier one.
pub(crate) fn repeated(changed: &OwnerChanged) -> bool {
    matches!(changed, OwnerChanged::Repeated { .. })
}

/// The route a secret is used at, checked: an `https` origin, or `http` on
/// this machine's loopback address, with no user, password, query or
/// fragment; a header a request may carry that is not the broker's own.
pub(crate) fn checked_route(
    upstream: &str,
    header: &str,
    prefix: &str,
) -> Result<Route, (StatusCode, String)> {
    let bad = |reason: &str| (StatusCode::BAD_REQUEST, format!("RouteInvalid: {reason}\n"));
    let url = reqwest::Url::parse(upstream)
        .map_err(|_unread| bad("where it is used is not an address"))?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "localhost"));
    let scheme = url.scheme() == "https" || (url.scheme() == "http" && loopback);
    if !scheme || url.host_str().is_none() {
        return Err(bad(
            "where it is used is an https address, or http on this computer's own loopback",
        ));
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(bad(
            "where it is used carries no user, password, query or fragment",
        ));
    }
    let lowered = header.to_ascii_lowercase();
    if HeaderName::from_bytes(lowered.as_bytes()).is_err()
        || lowered == "host"
        || lowered.starts_with("lys-")
    {
        return Err(bad(
            "the header is a request header's name, not host and not one beginning lys-",
        ));
    }
    if prefix.chars().any(char::is_control) {
        return Err(bad("the text before the value holds a control character"));
    }
    Ok(Route {
        upstream: upstream.trim_end_matches('/').to_owned(),
        header: lowered,
        prefix: prefix.to_owned(),
        spend_header: None,
    })
}

/// A route in the words an operation id is held to.
pub(crate) fn route_words(route: &Route) -> String {
    format!("{} {} {:?}", route.upstream, route.header, route.prefix)
}

const ADD: &str = "{operation, name, class, upstream, header, prefix, value}, each a string, \
                   class credential or key";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Add {
    operation: String,
    name: String,
    class: String,
    upstream: String,
    header: String,
    prefix: String,
    value: String,
}

fn class(text: &str) -> Result<EntryClass, (StatusCode, String)> {
    match text {
        "credential" => Ok(EntryClass::Credential),
        "key" => Ok(EntryClass::Key),
        _ => Err(malformed(ADD)),
    }
}

/// Adds a secret as the person: sealed, then routed to where it is used.
/// A resend of the same operation writes the same route again.
pub async fn add(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked) = vouched::<Add>(&shared, request, ADD).await?;
    let value = Zeroizing::new(asked.value);
    if value.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "ValueEmpty: a secret is added with its value\n".to_owned(),
        ));
    }
    let class = class(&asked.class)?;
    let route = checked_route(&asked.upstream, &asked.header, &asked.prefix)?;
    let words = route_words(&route);
    let layout = shared.layout.clone();
    let (name, operation) = (asked.name, asked.operation);
    on_broker(&shared, move |broker| {
        let sealed = Secret::from_slice(value.as_bytes());
        let changed = broker.add_secret(
            &who.identity,
            (&name, class, &sealed),
            &words,
            (who.via.as_deref(), Some(operation.as_str())),
        )?;
        layout.add_route(&name, route)?;
        let entry = broker
            .store()
            .entry(&name)
            .cloned()
            .ok_or_else(|| SecretsError::SecretUnknown { name: name.clone() })?;
        Ok::<_, SecretsError>(Json(json!({
            "secret": name,
            "class": entry.class,
            "owner": entry.owner,
            "sequence": entry.sequence,
            "operation": operation,
            "repeated": repeated(&changed),
        })))
    })
    .await
    .map_err(|error| refused(&error))?
    .map_err(|error| refused(&error))
}

const REPLACE: &str = "{operation, secret, value}, each a string";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Replace {
    operation: String,
    secret: String,
    value: String,
}

/// Replaces a secret's value, as its owner.
pub async fn replace(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked) = vouched::<Replace>(&shared, request, REPLACE).await?;
    let value = Zeroizing::new(asked.value);
    if value.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "ValueEmpty: a value is replaced with a new value\n".to_owned(),
        ));
    }
    let (secret, operation) = (asked.secret, asked.operation);
    on_broker(&shared, move |broker| {
        let sealed = Secret::from_slice(value.as_bytes());
        let (changed, sequence) = broker.replace_secret(
            &who.identity,
            (&secret, &sealed),
            (who.via.as_deref(), Some(operation.as_str())),
        )?;
        Ok::<_, SecretsError>(Json(json!({
            "secret": secret,
            "sequence": sequence,
            "operation": operation,
            "repeated": repeated(&changed),
        })))
    })
    .await
    .map_err(|error| refused(&error))?
    .map_err(|error| refused(&error))
}

const RETIRE: &str = "{operation, secret}, each a string";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Retire {
    operation: String,
    secret: String,
}

/// Retires `secret` as `who`, under `operation`, and takes its route out.
/// Answers when it was retired.
pub(crate) async fn retired(
    shared: &Arc<Shared>,
    who: Caller,
    secret: String,
    operation: String,
) -> Result<(OwnerChanged, i64), (StatusCode, String)> {
    let layout = shared.layout.clone();
    on_broker(shared, move |broker| {
        let answered = broker.retire_secret(
            &who.identity,
            &secret,
            (who.via.as_deref(), Some(operation.as_str())),
        )?;
        layout.remove_route(&secret)?;
        Ok::<_, SecretsError>(answered)
    })
    .await
    .map_err(|error| refused(&error))?
    .map_err(|error| refused(&error))
}

/// Retires a secret, as its owner: its handles end, its value leaves the
/// store, and its name is never used again.
pub async fn retire(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked) = vouched::<Retire>(&shared, request, RETIRE).await?;
    let (changed, at) =
        retired(&shared, who, asked.secret.clone(), asked.operation.clone()).await?;
    Ok(Json(json!({
        "secret": asked.secret,
        "retired_at": at,
        "operation": asked.operation,
        "repeated": repeated(&changed),
    })))
}
