//! The signed-in session, its cookie and its expiry.
//!
//! A session id is 32 bytes from the secure random source, carried in an
//! `HttpOnly`, `SameSite=Lax` cookie, `Secure` when configured. The session holds
//! the actor the service authenticated and when it ends. Sessions are kept in
//! memory: a restart signs everyone out, which is the safe direction.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use lys_identity::Actor;
use rand::TryRngCore;
use rand::rngs::OsRng;

use crate::error::ServerError;

/// The cookie's name.
pub const COOKIE: &str = "lys_directory_session";

/// Seconds since the Unix epoch, now.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// The live sessions.
pub struct Sessions {
    live: Mutex<HashMap<String, (Actor, u64)>>,
    seconds: u64,
    secure: bool,
}

impl Sessions {
    /// No sessions, each to live `seconds`.
    pub fn new(seconds: u64, secure: bool) -> Self {
        Self {
            live: Mutex::new(HashMap::new()),
            seconds,
            secure,
        }
    }

    /// Begin a session for `actor`, answering the Set-Cookie header value.
    pub fn begin(&self, actor: Actor) -> Result<String, ServerError> {
        let mut bytes = [0u8; 32];
        OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(|error| ServerError::SignInFailed {
                reason: format!("the secure random source failed: {error}"),
            })?;
        let id = crate::routes::hex(&bytes);
        let ends = now().saturating_add(self.seconds);
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        live.retain(|_, (_, until)| *until > now());
        live.insert(id.clone(), (actor, ends));
        let secure = if self.secure { "; Secure" } else { "" };
        Ok(format!(
            "{COOKIE}={id}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}{secure}",
            self.seconds
        ))
    }

    /// The actor of the session named by a Cookie header, if it is live.
    pub fn actor(&self, cookie_header: Option<&str>) -> Result<Actor, ServerError> {
        let id = cookie_header
            .into_iter()
            .flat_map(|header| header.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == COOKIE)
            .map(|(_, value)| value.to_owned())
            .ok_or(ServerError::NotSignedIn)?;
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        match live.get(&id) {
            Some((actor, until)) if *until > now() => Ok(actor.clone()),
            Some(_) => {
                live.remove(&id);
                Err(ServerError::NotSignedIn)
            }
            None => Err(ServerError::NotSignedIn),
        }
    }
}
