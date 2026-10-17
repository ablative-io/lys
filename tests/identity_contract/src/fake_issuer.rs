//! An in-process OIDC issuer that signs ID tokens for a chosen issuer and
//! subject, so the admission tests need no live Rauthy.
//!
//! It serves discovery, its one Ed25519 key as a JWK set, an authorize
//! endpoint that answers at once for the login the test chose, and a token
//! endpoint that checks the PKCE verifier and the client credentials before it
//! signs. The ID token is built here by hand, apart from openidconnect, so the
//! service's validation is checked against a second writer of the format.
//!
//! It also serves the issuer's own server-side sign-in, the three steps Lys's
//! sign-in page is carried through: `GET /oidc/authorize`, which opens a
//! session under a cookie and names its request token in a `<template>`;
//! `POST /pow`, a proof-of-work challenge of difficulty 10; and
//! `POST /oidc/authorize`, which takes the credentials with the session, the
//! token and the answered challenge and answers 202 with the address it would
//! send a browser to. Accounts are held by email with a password; an email
//! it holds no account for signs in as the login the test chose. Failed
//! sign-ins are counted by the address in `X-Forwarded-For`, and an address
//! with [`FAILURES_BARRED`] of them is answered 429, so a test can see that
//! one person's failures bar only that person.
//!
//! A stand-in sign-in provider answers on a port of its own: its authorize
//! address refuses a client id beginning `rejected` with a provider's own
//! words, and otherwise, asked with a state, signs the person in as the
//! login the test chose and sends them back to the redirect address. The
//! issuer's own provider steps, `POST /providers/login` and
//! `POST /providers/callback`, send a person to that provider and back to
//! the public callback address the test names, which is Lys's.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use lys_core::Ed25519Identity;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The client the service is registered as.
pub const CLIENT_ID: &str = "lys-directory";
/// The client secret the service holds in its secret file.
pub const CLIENT_SECRET: &str = "contract-test-client-secret";
const KEY_ID: &str = "contract-issuer-key";
/// The cookie the issuer's sign-in session is held under.
pub const SESSION_COOKIE: &str = "IssuerSession";
/// How many failed sign-ins from one address bar that address.
pub const FAILURES_BARRED: u32 = 3;
/// The path an issuer started behind a public origin serves under and
/// names itself at, as the installed issuer does.
pub const PUBLIC_PATH: &str = "/auth/v1";
/// The difficulty of the issuer's proof-of-work challenge, in zero bits.
const DIFFICULTY: u32 = 10;

/// An account the issuer holds, by email.
#[derive(Debug, Clone)]
pub struct Account {
    /// The login it signs in as.
    pub login: Login,
    /// Its password.
    pub password: String,
    /// Whether it asks for a second factor, such as a passkey.
    pub second_factor: bool,
}

/// The login the issuer signs the next token for.
#[derive(Debug, Clone)]
pub struct Login {
    /// The subject claim.
    pub subject: String,
    /// The email claim, which the service must never admit on.
    pub email: String,
}

struct Issued {
    login: Login,
    nonce: String,
    challenge: String,
}

struct Inner {
    issuer: String,
    api: String,
    loopback: String,
    key: Ed25519Identity,
    standing: Option<Login>,
    next: Mutex<Option<Login>>,
    codes: Mutex<HashMap<String, Issued>>,
    accounts: Mutex<HashMap<String, Account>>,
    disabled: Mutex<HashMap<String, Account>>,
    sessions: Mutex<HashMap<String, String>>,
    challenges: Mutex<HashSet<String>>,
    failures: Mutex<HashMap<String, u32>>,
    forwarded: Mutex<Vec<Option<String>>>,
    serial: AtomicU64,
    provider_base: String,
    public_callback: Mutex<String>,
    upstream: Mutex<HashMap<String, PendingUpstream>>,
    upstream_codes: Mutex<HashMap<String, Login>>,
    provider_redirects: Mutex<Vec<String>>,
    /// The body the next provider callback is refused with, when one is set.
    callback_refusal: Mutex<Option<Value>>,
}

/// A provider sign-in the issuer began and has not finished.
struct PendingUpstream {
    redirect_uri: String,
    state: String,
    nonce: String,
    code_challenge: String,
    pkce_challenge: String,
    xsrf: String,
    session: String,
}

fn held<T>(slot: &Mutex<T>) -> std::io::Result<std::sync::MutexGuard<'_, T>> {
    slot.lock()
        .map_err(|error| std::io::Error::other(format!("fixture_lock_poisoned: {error}")))
}

macro_rules! fixture {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "fixture_lock_poisoned", "message": error.to_string() })),
            ).into_response(),
        }
    };
}

impl Inner {
    /// Record the forwarded address a sign-in step carried, answering it.
    fn record(&self, headers: &HeaderMap) -> std::io::Result<String> {
        let forwarded = headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        held(&self.forwarded)?.push(forwarded.clone());
        Ok(forwarded.unwrap_or_default())
    }

    fn next_serial(&self) -> u64 {
        self.serial.fetch_add(1, Ordering::SeqCst)
    }

    /// Issue a code for `login`, answering it.
    fn issue(
        &self,
        login: Login,
        state: &str,
        nonce: String,
        challenge: String,
    ) -> std::io::Result<String> {
        let code = URL_SAFE_NO_PAD.encode(Sha256::digest(state.as_bytes()));
        held(&self.codes)?.insert(
            code.clone(),
            Issued {
                login,
                nonce,
                challenge,
            },
        );
        Ok(code)
    }

    /// The login the test chose for the next sign-in, or the standing one.
    fn chosen(&self) -> std::io::Result<Option<Login>> {
        Ok(held(&self.next)?.take().or_else(|| self.standing.clone()))
    }
}

/// A running issuer.
#[derive(Clone)]
pub struct FakeIssuer {
    inner: Arc<Inner>,
    serving: Arc<Mutex<crate::harness_serve::Serving>>,
}

impl FakeIssuer {
    /// Start an issuer on a local port, signing with the seed in `key_file`.
    ///
    /// # Errors
    /// Propagates fixture key, listener or server-start failures.
    pub async fn start(key_file: &Path) -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let issuer = format!("http://{}", listener.local_addr()?);
        Self::serve(listener, (issuer, ""), key_file, None).await
    }

    /// Start an issuer on a local port that names itself on `public`, the
    /// origin a browser uses, at [`PUBLIC_PATH`], as the installed issuer
    /// does behind Lys: its name and every endpoint its discovery document
    /// gives are on `public`, and it answers only on its own loopback port,
    /// under the same path.
    ///
    /// # Errors
    /// Propagates fixture key, listener or server-start failures.
    pub async fn start_behind(public: &str, key_file: &Path) -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let issuer = format!("{public}{PUBLIC_PATH}");
        Self::serve(listener, (issuer, PUBLIC_PATH), key_file, None).await
    }

    /// Start an issuer listening on `bind` that names itself `issuer`, for a
    /// development service a browser signs in to. Every sign-in the caller
    /// has not chosen with [`FakeIssuer::sign_in_as`] is signed as `standing`.
    ///
    /// # Errors
    /// Propagates fixture key, listener or server-start failures.
    pub async fn start_at(
        bind: SocketAddr,
        issuer: String,
        key_file: &Path,
        standing: Login,
    ) -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind(bind).await?;
        Self::serve(listener, (issuer, ""), key_file, Some(standing)).await
    }

    /// Serve on `listener` under the name and the path `(issuer, path)`.
    async fn serve(
        listener: tokio::net::TcpListener,
        (issuer, path): (String, &str),
        key_file: &Path,
        standing: Option<Login>,
    ) -> Result<Self, Box<dyn Error>> {
        let provider = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let provider_base = format!("http://{}", provider.local_addr()?);
        let loopback = format!("http://{}", listener.local_addr()?);
        let inner = Arc::new(Inner {
            issuer,
            api: format!("{loopback}{path}"),
            loopback,
            key: Ed25519Identity::load(key_file)?,
            standing,
            next: Mutex::new(None),
            codes: Mutex::new(HashMap::new()),
            accounts: Mutex::new(HashMap::new()),
            disabled: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            challenges: Mutex::new(HashSet::new()),
            failures: Mutex::new(HashMap::new()),
            forwarded: Mutex::new(Vec::new()),
            serial: AtomicU64::new(1),
            provider_base,
            public_callback: Mutex::new(String::new()),
            upstream: Mutex::new(HashMap::new()),
            upstream_codes: Mutex::new(HashMap::new()),
            provider_redirects: Mutex::new(Vec::new()),
            callback_refusal: Mutex::new(None),
        });
        let provider_router = Router::new()
            .route("/authorize", get(provider_authorize))
            .route("/login/oauth/authorize", get(provider_authorize))
            .route("/{tenant}/v2.0/authorize", get(provider_authorize))
            .with_state(Arc::clone(&inner));
        let router = Router::new()
            .route("/.well-known/openid-configuration", get(discovery))
            .route("/jwks", get(jwks))
            .route("/authorize", get(authorize))
            .route("/token", post(token))
            .route("/oidc/authorize", get(login_page).post(credentials))
            .route("/pow", post(pow))
            .route("/providers/login", post(provider_login))
            .route("/providers/callback", post(provider_callback))
            .with_state(Arc::clone(&inner));
        let router = if path.is_empty() {
            router
        } else {
            Router::new().nest(path, router)
        };
        let serving = crate::harness_serve::Serving::start(vec![
            (provider, provider_router),
            (listener, router),
        ])
        .await?;
        Ok(Self {
            inner,
            serving: Arc::new(Mutex::new(serving)),
        })
    }

    pub(crate) fn shutdown(&self) -> std::io::Result<()> {
        held(&self.serving)?.stop()
    }

    /// The issuer URL, exactly as it signs it.
    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.inner.issuer
    }

    /// Where the issuer answers: its loopback origin and the path it
    /// serves under, the address a service reaches it at.
    #[must_use]
    pub fn api(&self) -> &str {
        &self.inner.api
    }

    /// The issuer's loopback origin, which no browser is ever given.
    #[must_use]
    pub fn loopback(&self) -> &str {
        &self.inner.loopback
    }

    /// Sign the next sign-in as `login`.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn sign_in_as(&self, login: Login) -> std::io::Result<()> {
        *held(&self.inner.next)? = Some(login);
        Ok(())
    }

    /// Hold `account` under its login's email, replacing any held there.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn hold(&self, account: Account) -> std::io::Result<()> {
        held(&self.inner.accounts)?.insert(account.login.email.clone(), account);
        Ok(())
    }

    /// Follow a change the administration API made to account `subject`:
    /// its email, a new password when one was set, and whether it may sign
    /// in. An account never given a password is not held.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn follow(
        &self,
        subject: &str,
        email: &str,
        password: Option<String>,
        enabled: bool,
    ) -> std::io::Result<()> {
        let mut accounts = held(&self.inner.accounts)?;
        let mut disabled = held(&self.inner.disabled)?;
        let before = accounts
            .iter()
            .find(|(_, account)| account.login.subject == subject)
            .map(|(key, account)| (key.clone(), account.clone()));
        let before = match before {
            Some((key, account)) => {
                accounts.remove(&key);
                Some(account)
            }
            None => disabled.remove(subject),
        };
        let earlier = before.as_ref().map(|account| account.password.clone());
        let Some(password) = password.or(earlier) else {
            return Ok(());
        };
        let account = Account {
            login: Login {
                subject: subject.to_owned(),
                email: email.to_owned(),
            },
            password,
            second_factor: before.is_some_and(|account| account.second_factor),
        };
        if enabled {
            accounts.insert(email.to_owned(), account);
        } else {
            disabled.insert(subject.to_owned(), account);
        }
        Ok(())
    }

    /// The account held under `email`, if one is.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn account(&self, email: &str) -> std::io::Result<Option<Account>> {
        Ok(held(&self.inner.accounts)?.get(email).cloned())
    }

    /// The stand-in provider's origin.
    #[must_use]
    pub fn provider_base(&self) -> &str {
        &self.inner.provider_base
    }

    /// Send people back from the provider to `callback`, the issuer's
    /// public provider callback address.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn set_public_callback(&self, callback: &str) -> std::io::Result<()> {
        callback.clone_into(&mut *held(&self.inner.public_callback)?);
        Ok(())
    }

    /// Refuse the next provider callback with 403 and `body`, as the issuer
    /// refuses a person the provider signed in whom it will not admit.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn refuse_provider_callback(&self, body: Value) -> std::io::Result<()> {
        *held(&self.inner.callback_refusal)? = Some(body);
        Ok(())
    }

    /// Every redirect address the stand-in provider was asked with, in order.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn provider_redirects(&self) -> std::io::Result<Vec<String>> {
        Ok(held(&self.inner.provider_redirects)?.clone())
    }

    /// Every `X-Forwarded-For` value a sign-in step carried, in order, none
    /// where a step carried none.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn forwarded(&self) -> std::io::Result<Vec<Option<String>>> {
        Ok(held(&self.inner.forwarded)?.clone())
    }
}

type Shared = Arc<Inner>;

mod oauth;
mod provider;
mod sign_in;
use oauth::{authorize, discovery, jwks, refused, token};
use provider::{provider_authorize, provider_callback, provider_login};
use sign_in::{answered, credentials, login_page, pow, session_of, status};

#[cfg(test)]
mod poison_tests {
    use super::held;

    #[test]
    fn poisoned_fixture_state_fails_instead_of_returning_a_partial_value() {
        let slot = std::sync::Mutex::new(1);
        let poison = std::panic::catch_unwind(|| {
            let mut state = held(&slot).expect("fixture is initially healthy");
            *state = 2;
            panic!("injected partial fixture update");
        });
        assert!(poison.is_err());
        match held(&slot) {
            Err(error) => assert!(error.to_string().contains("fixture_lock_poisoned")),
            Ok(value) => panic!("poisoned fixture value was returned: {value}"),
        }
    }
}
