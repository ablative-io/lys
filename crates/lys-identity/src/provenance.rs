//! The actor of a change, as the service attests it.
//!
//! The directory service signs every event with its own key. What it attests
//! is that it authenticated this human, through this login, by this method, at
//! this time. It never claims the person signed anything: a person holds no key
//! the directory can speak for, and an agent registered by a person records
//! that person, never an invented agent signature (P8).

use crate::binding::LoginBinding;

/// How the service authenticated the actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthMethod {
    /// An OIDC sign-in whose token the service validated against the configured issuer.
    Oidc,
}

/// How and when the service authenticated the actor.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Provenance {
    method: AuthMethod,
    authenticated_at: u64,
}

impl Provenance {
    /// Authentication by `method` at `authenticated_at`, in seconds since the Unix epoch.
    pub fn new(method: AuthMethod, authenticated_at: u64) -> Self {
        Self {
            method,
            authenticated_at,
        }
    }

    /// How the actor was authenticated.
    pub fn method(&self) -> AuthMethod {
        self.method
    }

    /// When the actor was authenticated, in seconds since the Unix epoch.
    pub fn authenticated_at(&self) -> u64 {
        self.authenticated_at
    }
}

/// The authenticated human who made a change, as the service attests them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Actor {
    binding: LoginBinding,
    provenance: Provenance,
}

impl Actor {
    /// The human signed in through `binding`, authenticated as `provenance` says.
    pub fn new(binding: LoginBinding, provenance: Provenance) -> Self {
        Self {
            binding,
            provenance,
        }
    }

    /// The login the actor signed in through.
    pub fn binding(&self) -> &LoginBinding {
        &self.binding
    }

    /// How and when the actor was authenticated.
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}
