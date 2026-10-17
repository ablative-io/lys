//! The fixture serves this protocol surface from its shared issuer state.
use super::{
    CLIENT_ID, DIFFICULTY, Deserialize, Digest, FAILURES_BARRED, HeaderMap, Inner, IntoResponse,
    Json, Query, Response, SESSION_COOKIE, Sha256, Shared, State, StatusCode, header, held, json,
    refused,
};
#[derive(Deserialize)]
pub(super) struct PageRequest {
    client_id: String,
    code_challenge_method: String,
}

/// The start of the issuer's own sign-in: a session under a cookie, and its
/// request token in the page's template.
pub(super) async fn login_page(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Query(request): Query<PageRequest>,
) -> Response {
    fixture!(inner.record(&headers));
    if request.client_id != CLIENT_ID || request.code_challenge_method != "S256" {
        return refused("the authorization request is not one this issuer takes");
    }
    let serial = inner.next_serial();
    let session = format!("session{serial}");
    let token = format!("token{serial}");
    fixture!(held(&inner.sessions)).insert(session.clone(), token.clone());
    let page = format!(
        "<html><body><div hidden><template id=\"tpl_csrf_token\">{token}</template></div></body></html>"
    );
    (
        StatusCode::OK,
        [
            (
                header::SET_COOKIE,
                format!("{SESSION_COOKIE}={session}; Path=/; HttpOnly"),
            ),
            (header::CONTENT_TYPE, "text/html".to_owned()),
        ],
        page,
    )
        .into_response()
}

/// A proof-of-work challenge, remembered until it is answered once.
pub(super) async fn pow(State(inner): State<Shared>, headers: HeaderMap) -> Response {
    fixture!(inner.record(&headers));
    let serial = inner.next_serial();
    let challenge = format!("1:{DIFFICULTY}:4102444800:{serial:0>16}:{serial:0>43}:");
    fixture!(held(&inner.challenges)).insert(challenge.clone());
    challenge.into_response()
}

/// Whether `answer` answers a challenge this issuer gave and has not taken,
/// taking it.
pub(super) fn answered(inner: &Inner, answer: &str) -> std::io::Result<bool> {
    let Some((challenge, counter)) = answer.rsplit_once(':') else {
        return Ok(false);
    };
    let hash = Sha256::digest(answer.as_bytes());
    let zeros = hash[0] == 0 && hash[1] >> (16 - DIFFICULTY) == 0;
    Ok(!counter.is_empty() && zeros && held(&inner.challenges)?.remove(&format!("{challenge}:")))
}

#[derive(Deserialize)]
pub(super) struct Credentials {
    email: String,
    password: Option<String>,
    pow: String,
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

pub(super) fn session_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.to_owned())
}

pub(super) fn status(code: StatusCode, message: &str) -> Response {
    (
        code,
        Json(json!({ "error": "Unauthorized", "message": message })),
    )
        .into_response()
}

/// The credentials, with the session, its token and the answered challenge.
pub(super) async fn credentials(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Json(request): Json<Credentials>,
) -> Response {
    // The issuer refuses a sign-in that names no User-Agent (Rauthy v0.36.2,
    // "Empty User-Agent not allowed").
    let agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok());
    if agent.is_none_or(str::is_empty) {
        return status(StatusCode::BAD_REQUEST, "Empty User-Agent not allowed");
    }
    let address = fixture!(inner.record(&headers));
    let token = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok());
    let sessions = fixture!(held(&inner.sessions));
    let session = session_of(&headers).and_then(|id| sessions.get(&id).cloned());
    drop(sessions);
    if session.is_none() || session.as_deref() != token {
        return status(StatusCode::UNAUTHORIZED, "Unauthorized Session");
    }
    if !fixture!(answered(&inner, &request.pow)) {
        return status(StatusCode::BAD_REQUEST, "Invalid PoW");
    }
    if fixture!(held(&inner.failures))
        .get(&address)
        .copied()
        .unwrap_or(0)
        >= FAILURES_BARRED
    {
        return status(StatusCode::TOO_MANY_REQUESTS, "Too many failed logins");
    }
    let password = request.password.unwrap_or_default();
    let account = fixture!(held(&inner.accounts)).get(&request.email).cloned();
    let login = match account {
        Some(account) if account.password == password => {
            if account.second_factor {
                return (StatusCode::OK, Json(json!({ "code": "webauthn" }))).into_response();
            }
            Some(account.login)
        }
        Some(_) => None,
        None if password.is_empty() => None,
        None => fixture!(inner.chosen()),
    };
    let Some(login) = login else {
        *fixture!(held(&inner.failures)).entry(address).or_insert(0) += 1;
        return status(StatusCode::UNAUTHORIZED, "Invalid user credentials");
    };
    if request.client_id != CLIENT_ID || request.code_challenge_method.as_deref() != Some("S256") {
        return status(
            StatusCode::BAD_REQUEST,
            "the client or its challenge is not taken",
        );
    }
    let state = request.state.unwrap_or_default();
    let code = fixture!(inner.issue(
        login,
        &state,
        request.nonce.unwrap_or_default(),
        request.code_challenge.unwrap_or_default(),
    ));
    let location = format!("{}?code={code}&state={state}", request.redirect_uri);
    (StatusCode::ACCEPTED, [(header::LOCATION, location)]).into_response()
}
