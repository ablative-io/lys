//! The token route's two grants: an authorization code exchanged for a pass,
//! an ID token and a refresh token, and a refresh token exchanged for a new
//! pass read from the grants live at that moment (ACCESS-002 R1, R2).
//!
//! Both are judged after the client authenticates (`client_auth`), so a
//! request with a refused credential is refused as that whatever grant it
//! asks for. No provider lock is held while the sessions, the directory, the
//! apps or the grants are read (`exchange.rs` keeps the lock order).

use lys_identity::IdentityId;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::endpoints::{Exchange, admitted_client, display_name, malformed, provider};
use super::grant_binding::asked;
use super::rights_claim::{Pass, pass};
use super::{Access, ID_TOKEN_SECONDS, Kind, held, random};
use crate::error::ServerError;
use crate::error_provider::ProviderError;
use crate::routes::{AppState, hex};
use crate::session::now;

/// The token route's answer to the app `client`, by the grant `form` asks for.
pub(super) fn exchange(
    state: &AppState,
    form: &Exchange,
    client: &str,
) -> Result<Value, ServerError> {
    match form.grant_type.as_str() {
        "authorization_code" => by_code(state, form, client),
        "refresh_token" => by_refresh(state, form, client),
        _ => Err(malformed(
            "a product exchanges an authorization code or a refresh token",
        )),
    }
}

/// The digest a kept token is found by.
fn digest(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

/// The exchange of a code for a pass, an ID token and a refresh token. The
/// app and its return address are judged again here, at this request.
fn by_code(state: &AppState, form: &Exchange, client: &str) -> Result<Value, ServerError> {
    let provider = provider(state)?;
    // An unsupported binding version is refused before the code is taken.
    let bind = asked(form.grant_binding.as_deref())?;
    let admitted = admitted_client(state, client, &form.redirect_uri)?;
    let client_id = admitted.app;
    let at = now();
    // The first act on the provider's state: the code verified and marked used.
    let taken = provider.take_grant(
        &form.code,
        &client_id,
        &form.redirect_uri,
        &form.code_verifier,
        at,
    )?;
    // Between the two acts, with no provider guard held: the session, the
    // setting, the name and the rights. The name goes into the ID token only
    // when the authorization asked for it and the app's setting still grants
    // it at this exchange; it is read from the directory now and kept nowhere.
    if !state.sessions.is_live(&taken.session_id)? {
        return Err(ServerError::Provider(ProviderError::CodeExpired));
    }
    let profile = taken.profile && admitted.profile;
    let name = if profile {
        display_name(state, taken.person)?
    } else {
        None
    };
    let ends_at = taken.expires_at;
    let mut claims = json!({
        "iss": provider.issuer,
        "sub": taken.subject,
        "aud": client_id,
        "iat": at,
        "exp": ends_at.min(at.saturating_add(ID_TOKEN_SECONDS)),
        "auth_time": taken.authenticated_at,
    });
    if let Some(nonce) = taken.nonce {
        claims["nonce"] = Value::String(nonce);
    }
    if let Some(name) = name {
        claims["name"] = Value::String(name);
    }
    let id_token = provider.signed(&claims)?;
    let issued = pass(
        state,
        provider,
        (IdentityId::Person(taken.person), client_id.as_str()),
        (at, ends_at),
        bind,
    )?;
    let refresh = random::<32>()?;
    let kept = |expires_at, kind| Access {
        session_id: taken.session_id.clone(),
        subject: taken.subject.clone(),
        app: client_id.clone(),
        profile,
        expires_at,
        kind,
    };
    // The second act: the pass and the refresh token kept together and
    // remembered by the code, or refused when the code was replayed between.
    provider.issue(
        &form.code,
        digest(&issued.token),
        kept(issued.expires_at, None),
        Some((digest(&refresh), kept(ends_at, Some(Kind::Refresh)))),
        at,
    )?;
    Ok(answered(
        json!({
            "access_token": issued.token,
            "token_type": "Bearer",
            "expires_in": issued.expires_at.saturating_sub(at),
            "id_token": id_token,
            "refresh_token": refresh,
        }),
        issued.binding,
    ))
}

/// `answer`, carrying the pass's grant binding beside it when one was made.
fn answered(mut answer: Value, binding: Option<String>) -> Value {
    if let Some(binding) = binding {
        answer["grant_binding"] = Value::String(binding);
    }
    answer
}

/// The exchange of a refresh token for a new pass, from the grants live now:
/// a grant revoked since the last pass is absent from this one. The refresh
/// token stands on the Lys sign-in it was issued in, and ends with it.
fn by_refresh(state: &AppState, form: &Exchange, client: &str) -> Result<Value, ServerError> {
    let provider = provider(state)?;
    let bind = asked(form.grant_binding.as_deref())?;
    let at = now();
    let lookup = digest(&form.refresh_token);
    let unknown = || ServerError::Provider(ProviderError::RefreshUnknown);
    let (session_id, subject, profile, ends_at) = {
        let tokens = held(&provider.tokens)?;
        let kept = tokens.get(&lookup, at).map_err(|error| match error {
            ServerError::Provider(ProviderError::TokenUnknown) => unknown(),
            other => other,
        })?;
        if kept.kind != Some(Kind::Refresh) || kept.app != client {
            return Err(unknown());
        }
        (
            kept.session_id.clone(),
            kept.subject.clone(),
            kept.profile,
            kept.expires_at,
        )
    };
    let person = subject
        .parse::<lys_identity::PersonId>()
        .map_err(ServerError::Identity)?;
    if !state.sessions.is_live(&session_id)? {
        held(&provider.tokens)?.revoke(&lookup)?;
        // Retiring or suspending a holder also ends its sign-ins: the
        // holder's state is named first, the ended sign-in otherwise.
        crate::grants::with_grants(state, |judged| {
            super::rights_claim::holder_of(judged.directory, IdentityId::Person(person)).map(drop)
        })?;
        return Err(ServerError::Provider(ProviderError::SessionEnded));
    }
    let issued: Pass = pass(
        state,
        provider,
        (IdentityId::Person(person), client),
        (at, ends_at),
        bind,
    )?;
    held(&provider.tokens)?.insert(
        digest(&issued.token),
        Access {
            session_id,
            subject,
            app: client.to_owned(),
            profile,
            expires_at: issued.expires_at,
            kind: None,
        },
        at,
    )?;
    Ok(answered(
        json!({
            "access_token": issued.token,
            "token_type": "Bearer",
            "expires_in": issued.expires_at.saturating_sub(at),
            "refresh_token": form.refresh_token,
        }),
        issued.binding,
    ))
}
