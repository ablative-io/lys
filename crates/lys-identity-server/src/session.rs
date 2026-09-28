//! The signed-in session, its cookie and its expiry.
//!
//! A session's cookie secret is 32 bytes from the secure random source,
//! carried in an `HttpOnly`, `SameSite=Lax` cookie, `Secure` when configured.
//! The session holds the actor the service authenticated, when it started and
//! when it ends, and a public id of 16 further random bytes. The public id is
//! what every answer names a session by; the cookie secret never leaves this
//! module except in the Set-Cookie header that begins the session. Sessions
//! are kept in memory: a restart signs everyone out, which is the safe
//! direction.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
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

/// One live session as an answer may name it: its public id, never its cookie secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    /// The session's public id, apart from its cookie secret.
    pub id: String,
    /// The actor the service authenticated.
    pub actor: Actor,
    /// When the session began, in seconds since the Unix epoch.
    pub started_at: u64,
    /// When the session ends, in seconds since the Unix epoch.
    pub ends_at: u64,
}

/// The live sessions.
pub struct Sessions {
    live: Mutex<HashMap<String, SessionEntry>>,
    seconds: u64,
    secure: bool,
}

/// `N` bytes from the secure random source, as lowercase hex.
fn random_hex<const N: usize>() -> Result<String, ServerError> {
    let mut bytes = [0u8; N];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| ServerError::SignInFailed {
            reason: format!("the secure random source failed: {error}"),
        })?;
    Ok(crate::routes::hex(&bytes))
}

/// The cookie secret a Cookie header names, if it names one.
fn cookie_secret(cookie_header: Option<&str>) -> Result<&str, ServerError> {
    cookie_header
        .into_iter()
        .flat_map(|header| header.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE)
        .map(|(_, value)| value)
        .ok_or(ServerError::NotSignedIn)
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

    /// The live sessions, with every expired one removed.
    fn pruned(&self) -> MutexGuard<'_, HashMap<String, SessionEntry>> {
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        let at = now();
        live.retain(|_, entry| entry.ends_at > at);
        live
    }

    /// The Set-Cookie header value carrying `value` for `max_age` seconds.
    fn cookie(&self, value: &str, max_age: u64) -> String {
        let secure = if self.secure { "; Secure" } else { "" };
        format!("{COOKIE}={value}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age}{secure}")
    }

    /// The Set-Cookie header value that clears the session cookie, with the
    /// same attributes the cookie was set with.
    pub fn clear_cookie(&self) -> String {
        self.cookie("", 0)
    }

    /// Begin a session for `actor`, answering the Set-Cookie header value.
    pub fn begin(&self, actor: Actor) -> Result<String, ServerError> {
        let secret = random_hex::<32>()?;
        let started_at = now();
        let entry = SessionEntry {
            id: random_hex::<16>()?,
            actor,
            started_at,
            ends_at: started_at.saturating_add(self.seconds),
        };
        self.pruned().insert(secret.clone(), entry);
        Ok(self.cookie(&secret, self.seconds))
    }

    /// The live session named by a Cookie header.
    fn entry(&self, cookie_header: Option<&str>) -> Result<SessionEntry, ServerError> {
        let secret = cookie_secret(cookie_header)?;
        self.pruned()
            .get(secret)
            .cloned()
            .ok_or(ServerError::NotSignedIn)
    }

    /// The actor of the session named by a Cookie header, if it is live.
    pub fn actor(&self, cookie_header: Option<&str>) -> Result<Actor, ServerError> {
        self.entry(cookie_header).map(|entry| entry.actor)
    }

    /// The public id of the live session named by a Cookie header.
    pub fn current(&self, cookie_header: Option<&str>) -> Result<String, ServerError> {
        self.entry(cookie_header).map(|entry| entry.id)
    }

    /// Every live session whose actor `belongs` admits.
    pub fn live(&self, belongs: impl Fn(&Actor) -> bool) -> Vec<SessionEntry> {
        self.pruned()
            .values()
            .filter(|entry| belongs(&entry.actor))
            .cloned()
            .collect()
    }

    /// End the live session with public id `id`, if `belongs` admits its
    /// actor, answering the ended session. A session that is not live, or
    /// whose actor `belongs` does not admit, is refused `SessionUnknown`
    /// alike, and stays as it was.
    pub fn end(
        &self,
        id: &str,
        belongs: impl FnOnce(&Actor) -> bool,
    ) -> Result<SessionEntry, ServerError> {
        let mut live = self.pruned();
        let secret = live
            .iter()
            .find(|(_, entry)| entry.id == id)
            .map(|(secret, entry)| (secret.clone(), belongs(&entry.actor)));
        match secret {
            Some((secret, true)) => live.remove(&secret).ok_or(ServerError::SessionUnknown),
            Some((_, false)) | None => Err(ServerError::SessionUnknown),
        }
    }
}
