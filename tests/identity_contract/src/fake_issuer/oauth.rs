//! The fixture serves this protocol surface from its shared issuer state.
use super::{
    CLIENT_ID, CLIENT_SECRET, Deserialize, Digest, Engine, Form, HeaderMap, IntoResponse, Json,
    KEY_ID, Query, Response, STANDARD, Sha256, Shared, State, StatusCode, URL_SAFE_NO_PAD, Value,
    header, held, json,
};
pub(super) fn refused(reason: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": reason }))).into_response()
}

pub(super) async fn discovery(State(inner): State<Shared>) -> Json<Value> {
    let base = &inner.issuer;
    Json(json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/authorize"),
        "token_endpoint": format!("{base}/token"),
        "jwks_uri": format!("{base}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["EdDSA"],
        "code_challenge_methods_supported": ["S256"],
    }))
}

pub(super) async fn jwks(State(inner): State<Shared>) -> Json<Value> {
    Json(json!({ "keys": [{
        "kty": "OKP",
        "crv": "Ed25519",
        "use": "sig",
        "alg": "EdDSA",
        "kid": KEY_ID,
        "x": URL_SAFE_NO_PAD.encode(inner.key.public_key_bytes()),
    }]}))
}

#[derive(Deserialize)]
pub(super) struct AuthorizeRequest {
    redirect_uri: String,
    state: String,
    nonce: String,
    code_challenge: String,
    code_challenge_method: String,
}

pub(super) async fn authorize(
    State(inner): State<Shared>,
    Query(request): Query<AuthorizeRequest>,
) -> Response {
    if request.code_challenge_method != "S256" {
        return refused("only S256 is accepted");
    }
    let Some(login) = fixture!(inner.chosen()) else {
        return refused("the test chose no login");
    };
    let code = fixture!(inner.issue(login, &request.state, request.nonce, request.code_challenge));
    let location = format!(
        "{}?code={code}&state={}",
        request.redirect_uri, request.state
    );
    (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()
}

#[derive(Deserialize)]
pub(super) struct TokenRequest {
    code: String,
    code_verifier: String,
}

pub(super) fn client_admitted(headers: &HeaderMap) -> bool {
    let expected = format!(
        "Basic {}",
        STANDARD.encode(format!("{CLIENT_ID}:{CLIENT_SECRET}"))
    );
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == expected)
}

pub(super) async fn token(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Form(request): Form<TokenRequest>,
) -> Response {
    if !client_admitted(&headers) {
        return refused("the client credentials are wrong");
    }
    let Some(issued) = fixture!(held(&inner.codes)).remove(&request.code) else {
        return refused("the code is unknown or used");
    };
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(request.code_verifier.as_bytes()));
    if challenge != issued.challenge {
        return refused("the PKCE verifier does not match");
    }
    let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return refused("the clock is before the epoch");
    };
    let now = now.as_secs();
    let claims = json!({
        "iss": inner.issuer,
        "sub": issued.login.subject,
        "email": issued.login.email,
        "email_verified": true,
        "aud": [CLIENT_ID],
        "iat": now,
        "exp": now + 300,
        "auth_time": now,
        "nonce": issued.nonce,
    });
    let header = json!({ "alg": "EdDSA", "typ": "JWT", "kid": KEY_ID });
    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header.to_string()),
        URL_SAFE_NO_PAD.encode(claims.to_string())
    );
    let signature = URL_SAFE_NO_PAD.encode(inner.key.sign(signing_input.as_bytes()));
    Json(json!({
        "access_token": URL_SAFE_NO_PAD.encode(Sha256::digest(signing_input.as_bytes())),
        "token_type": "bearer",
        "expires_in": 300,
        "id_token": format!("{signing_input}.{signature}"),
    }))
    .into_response()
}
