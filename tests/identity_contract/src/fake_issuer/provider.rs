//! The fixture serves this protocol surface from its shared issuer state.
use super::{
    CLIENT_ID, Deserialize, Digest, Engine, HeaderMap, Inner, IntoResponse, Json, PendingUpstream,
    Query, Response, Sha256, Shared, State, StatusCode, URL_SAFE_NO_PAD, answered, header, held,
    json, refused, session_of, status,
};
#[derive(Deserialize)]
pub(super) struct ProviderAsk {
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
}

/// The stand-in provider's sign-in address.
pub(super) async fn provider_authorize(
    State(inner): State<Shared>,
    Query(ask): Query<ProviderAsk>,
) -> Response {
    fixture!(held(&inner.provider_redirects)).push(ask.redirect_uri.clone());
    if ask.client_id.starts_with("rejected") {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "invalid_client",
                "error_description": "The OAuth client was not found.",
            })),
        )
            .into_response();
    }
    let Some(state) = ask.state else {
        return (StatusCode::OK, "<html><title>Sign in</title></html>").into_response();
    };
    let Some(login) = fixture!(inner.chosen()) else {
        return refused("the test chose no login at the provider");
    };
    let code = format!("upstream{}", inner.next_serial());
    fixture!(held(&inner.upstream_codes)).insert(code.clone(), login);
    let location = format!("{}?code={code}&state={state}", ask.redirect_uri);
    (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()
}

#[derive(Deserialize)]
pub(super) struct ProviderLogin {
    pow: String,
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    provider_id: String,
    pkce_challenge: String,
}

/// The session a request carries, when its request token is the session's.
pub(super) fn session_with_token(
    inner: &Inner,
    headers: &HeaderMap,
) -> std::io::Result<Option<String>> {
    let Some(token) = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(None);
    };
    let Some(session) = session_of(headers) else {
        return Ok(None);
    };
    Ok(
        (held(&inner.sessions)?.get(&session).map(String::as_str) == Some(token))
            .then_some(session),
    )
}

/// The issuer's start of a provider sign-in.
pub(super) async fn provider_login(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Json(request): Json<ProviderLogin>,
) -> Response {
    fixture!(inner.record(&headers));
    let Some(session) = fixture!(session_with_token(&inner, &headers)) else {
        return status(StatusCode::UNAUTHORIZED, "Unauthorized Session");
    };
    if !fixture!(answered(&inner, &request.pow)) {
        return status(StatusCode::BAD_REQUEST, "Invalid PoW");
    }
    if request.provider_id.is_empty() || request.provider_id.starts_with("unknown") {
        return status(StatusCode::NOT_FOUND, "no provider has that id");
    }
    if request.client_id != CLIENT_ID || request.code_challenge_method.as_deref() != Some("S256") {
        return status(
            StatusCode::BAD_REQUEST,
            "the client or its challenge is not taken",
        );
    }
    let serial = inner.next_serial();
    let callback_id = format!("callback{serial}");
    let xsrf = format!("xsrf{serial}");
    fixture!(held(&inner.upstream)).insert(
        callback_id.clone(),
        PendingUpstream {
            redirect_uri: request.redirect_uri,
            state: request.state.unwrap_or_default(),
            nonce: request.nonce.unwrap_or_default(),
            code_challenge: request.code_challenge.unwrap_or_default(),
            pkce_challenge: request.pkce_challenge,
            xsrf: xsrf.clone(),
            session,
        },
    );
    let callback = fixture!(held(&inner.public_callback)).clone();
    let Ok(mut location) = reqwest::Url::parse(&format!("{}/authorize", inner.provider_base))
    else {
        return status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the provider's address is not one",
        );
    };
    location
        .query_pairs_mut()
        .append_pair("client_id", "stand-in-client")
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &callback)
        .append_pair("state", &callback_id);
    (
        StatusCode::ACCEPTED,
        [
            (header::LOCATION, location.to_string()),
            (
                header::SET_COOKIE,
                format!("IssuerUpstream={callback_id}; Path=/"),
            ),
        ],
        xsrf,
    )
        .into_response()
}

#[derive(Deserialize)]
pub(super) struct ProviderBack {
    state: String,
    code: String,
    xsrf_token: String,
    pkce_verifier: String,
}

/// The issuer's finish of a provider sign-in.
pub(super) async fn provider_callback(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Json(back): Json<ProviderBack>,
) -> Response {
    fixture!(inner.record(&headers));
    let Some(session) = fixture!(session_with_token(&inner, &headers)) else {
        return status(StatusCode::UNAUTHORIZED, "Unauthorized Session");
    };
    let Some(pending) = fixture!(held(&inner.upstream)).remove(&back.state) else {
        return status(
            StatusCode::BAD_REQUEST,
            "no provider sign-in has that state",
        );
    };
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains(&format!("IssuerUpstream={}", back.state)));
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(back.pkce_verifier.as_bytes()));
    let matches = cookie
        && session == pending.session
        && back.xsrf_token == pending.xsrf
        && challenge == pending.pkce_challenge;
    if !matches {
        return status(
            StatusCode::BAD_REQUEST,
            "the provider sign-in does not match its start",
        );
    }
    let Some(login) = fixture!(held(&inner.upstream_codes)).remove(&back.code) else {
        return status(
            StatusCode::BAD_REQUEST,
            "the provider's code is unknown or used",
        );
    };
    let code = fixture!(inner.issue(login, &pending.state, pending.nonce, pending.code_challenge));
    let location = format!(
        "{}?code={code}&state={}",
        pending.redirect_uri, pending.state
    );
    (StatusCode::ACCEPTED, [(header::LOCATION, location)]).into_response()
}
