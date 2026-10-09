//! Signing a person in to a product through Lys, server-side.
//!
//! The authorization code flow with PKCE runs in the product's server, never
//! in the browser. The authorization, token and key addresses are read from
//! the issuer's discovery document, never written here. The app's issued
//! credential is held by [`SignIn`] and sent only in the token request; it
//! is never printed, and no error carries it or the issuer's response body.
//! A pass is trusted only after it verifies against the issuer's published
//! keys, for the product's audience, at the instant the caller names.

use std::fmt;
use std::fs::File;
use std::io::Read;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::Client as Http;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use url::Url;

use crate::{Error, KeySet, TokenResponse, VerifiedPass, read_token};

/// The discovery document's place under the issuer.
const DISCOVERY: &str = ".well-known/openid-configuration";

/// The scope every sign-in asks for.
const SCOPE: &str = "openid";

/// An app's issued client credential. Its text is never shown.
pub struct Secret(String);

impl Secret {
    /// The credential `text`, as the product read it from its own file.
    ///
    /// # Errors
    ///
    /// `contract_refused` when the text is empty or holds a control
    /// character, so a trailing newline is the file's mistake, said by name.
    pub fn new(text: String) -> Result<Self, Error> {
        if text.is_empty() || text.chars().any(char::is_control) {
            return Err(Error::Invalid("client secret is empty or not one line"));
        }
        Ok(Self(text))
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret(<redacted>)")
    }
}

/// What a sign-in was started with: where to send the browser, and what the
/// product keeps server-side until the code comes back.
#[derive(Debug, Clone)]
pub struct Started {
    /// The Lys authorization address the browser is sent to.
    pub url: String,
    /// The state Lys returns with the code.
    pub state: String,
    /// The PKCE verifier, kept by the product and sent only in the exchange.
    pub verifier: String,
    /// The value of the short-lived cookie binding the state to the browser.
    pub browser: String,
}

/// A signed-in person's verified pass.
#[derive(Debug, Clone)]
pub struct SignedIn {
    /// The pass's text, presented by the product on the person's behalf.
    pub token: String,
    /// Whose pass it is.
    pub holder: String,
    /// The pass's first invalid instant, in Unix seconds.
    pub expires_at: u64,
    /// The refresh token, when Lys issued one.
    pub refresh: Option<String>,
}

/// The addresses an issuer's discovery document names.
#[derive(Debug, Clone)]
struct Endpoints {
    authorize: Url,
    token: Url,
    keys: Url,
}

#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
}

/// One app's server-side sign-in through one Lys issuer.
#[derive(Debug)]
pub struct SignIn {
    http: Http,
    issuer: String,
    audience: String,
    client_id: String,
    secret: Secret,
    redirect: Url,
    endpoints: Endpoints,
}

impl SignIn {
    /// Read the discovery document of the Lys at `issuer` and sign people in
    /// to the app `audience` as the client `client_id` with `secret`, Lys
    /// sending each browser back to `redirect`.
    ///
    /// # Errors
    ///
    /// `endpoint_refused` or `contract_refused` when `issuer` is not an HTTP
    /// origin or the document names another issuer or an address that is not
    /// HTTP; `lys_could_not_be_asked` when the document cannot be read.
    pub async fn connect(
        http: Http,
        issuer: &Url,
        audience: &str,
        client_id: &str,
        secret: Secret,
        redirect: Url,
    ) -> Result<Self, Error> {
        if audience.is_empty() || client_id.is_empty() || client_id.chars().any(char::is_control) {
            return Err(Error::Invalid("audience or client id is missing"));
        }
        if !matches!(redirect.scheme(), "http" | "https") || redirect.host_str().is_none() {
            return Err(Error::Invalid("redirect is not an HTTP address"));
        }
        let base = crate::Client::new(http.clone(), issuer.clone())?;
        let response = http
            .get(base.endpoint(DISCOVERY)?)
            .send()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        if !response.status().is_success() {
            return Err(Error::CannotAsk("discovery returned a refusal status"));
        }
        let discovery: Discovery = response
            .json()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        if discovery.issuer.trim_end_matches('/') != issuer.as_str().trim_end_matches('/') {
            return Err(Error::WrongIssuer);
        }
        let endpoints = Endpoints {
            authorize: http_address(&discovery.authorization_endpoint)?,
            token: http_address(&discovery.token_endpoint)?,
            keys: http_address(&discovery.jwks_uri)?,
        };
        Ok(Self {
            http,
            issuer: discovery.issuer,
            audience: audience.to_owned(),
            client_id: client_id.to_owned(),
            secret,
            redirect,
            endpoints,
        })
    }

    /// Start a sign-in: a fresh state, PKCE verifier and browser value, and
    /// the authorization address carrying the state and the verifier's S256
    /// challenge.
    ///
    /// # Errors
    ///
    /// `contract_refused` when the system's random source cannot be read.
    pub fn start(&self) -> Result<Started, Error> {
        let state = random()?;
        let verifier = random()?;
        let browser = random()?;
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let mut url = self.endpoints.authorize.clone();
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", self.redirect.as_str())
            .append_pair("scope", SCOPE)
            .append_pair("state", &state)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256");
        Ok(Started {
            url: url.into(),
            state,
            verifier,
            browser,
        })
    }

    /// Exchange `code` for a pass, when `state` and `browser` are the ones
    /// `started` holds, and verify the pass at `now`.
    ///
    /// # Errors
    ///
    /// `sign_in_state_refused` when the state or the browser differs, before
    /// anything is sent; the issuer's refusal by name; or the pass's own
    /// refusal when it does not verify.
    pub async fn finish(
        &self,
        code: &str,
        state: &str,
        browser: &str,
        started: &Started,
        now: u64,
    ) -> Result<SignedIn, Error> {
        if !same(state, &started.state) || !same(browser, &started.browser) {
            return Err(Error::TokenRefused {
                name: "sign_in_state_refused".to_owned(),
                reason: "the sign-in's state is not this browser's".to_owned(),
            });
        }
        if code.is_empty() {
            return Err(Error::Invalid("authorization code is missing"));
        }
        let answer = self
            .token(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", self.redirect.as_str()),
                ("code_verifier", &started.verifier),
            ])
            .await?;
        self.verified(answer, now).await
    }

    /// A fresh verified pass for `refresh`, at `now`.
    ///
    /// # Errors
    ///
    /// `lys_could_not_be_asked` when Lys cannot be reached; the issuer's
    /// refusal by name; or the pass's own refusal when it does not verify.
    pub async fn refresh(&self, refresh: &str, now: u64) -> Result<SignedIn, Error> {
        if refresh.is_empty() {
            return Err(Error::Invalid("refresh token is missing"));
        }
        let answer = self
            .token(&[("grant_type", "refresh_token"), ("refresh_token", refresh)])
            .await?;
        self.verified(answer, now).await
    }

    /// The token request, with the client credential in the form
    /// (`client_secret_post`), the method the existing exchange uses.
    async fn token(&self, form: &[(&str, &str)]) -> Result<TokenResponse, Error> {
        let mut fields = vec![
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.secret.0.as_str()),
        ];
        fields.extend_from_slice(form);
        let response = self
            .http
            .post(self.endpoints.token.clone())
            .form(&fields)
            .send()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        read_token(response).await
    }

    async fn verified(&self, answer: TokenResponse, now: u64) -> Result<SignedIn, Error> {
        let keys = self.keys().await?;
        let pass = VerifiedPass::verify(
            &answer.access_token,
            &keys,
            &self.issuer,
            &self.audience,
            now,
        )?;
        let claims = pass.claims();
        Ok(SignedIn {
            holder: claims.holder.id.clone(),
            expires_at: claims.exp,
            token: answer.access_token,
            refresh: answer.refresh_token,
        })
    }

    async fn keys(&self) -> Result<KeySet, Error> {
        let response = self
            .http
            .get(self.endpoints.keys.clone())
            .send()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        let text = response
            .error_for_status()
            .map_err(|error| Error::Transport(Box::new(error)))?
            .text()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        KeySet::from_json(&text)
    }
}

/// The `Set-Cookie` value of a signed-in session: the opaque `id` only,
/// `HttpOnly`, `Secure`, `SameSite=Strict`, for the whole origin.
///
/// # Errors
///
/// `contract_refused` when the name or the id is not a cookie token.
pub fn session_cookie(name: &str, id: &str, max_age: u64) -> Result<String, Error> {
    cookie(name, id, "Strict", max_age)
}

/// The `Set-Cookie` value binding a started sign-in to its browser:
/// `HttpOnly`, `Secure`, `SameSite=Lax` so it rides Lys's redirect back, and
/// short-lived.
///
/// # Errors
///
/// `contract_refused` when the name or the value is not a cookie token.
pub fn state_cookie(name: &str, browser: &str, max_age: u64) -> Result<String, Error> {
    cookie(name, browser, "Lax", max_age)
}

fn cookie(name: &str, value: &str, same_site: &str, max_age: u64) -> Result<String, Error> {
    let token = |text: &str| {
        !text.is_empty()
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    };
    if !token(name) || !token(value) {
        return Err(Error::Invalid("cookie name or value is not a token"));
    }
    Ok(format!(
        "{name}={value}; HttpOnly; Secure; SameSite={same_site}; Path=/; Max-Age={max_age}"
    ))
}

fn http_address(text: &str) -> Result<Url, Error> {
    let url = Url::parse(text)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(Error::Invalid("discovery names an address that is not HTTP"));
    }
    Ok(url)
}

/// 32 bytes of the system's random source, as unpadded base64url: 43
/// characters, all in PKCE's verifier alphabet.
fn random() -> Result<String, Error> {
    let mut bytes = [0_u8; 32];
    let read = File::open("/dev/urandom").and_then(|mut source| source.read_exact(&mut bytes));
    if read.is_err() {
        return Err(Error::Invalid("the random source cannot be read"));
    }
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// Whether two values are equal, comparing every byte whatever differs first.
fn same(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .fold(0_u8, |differs, (a, b)| differs | (a ^ b))
            == 0
}
