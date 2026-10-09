//! A pass's grant binding (DIRECTORY-089 R2): the signed statement, separate
//! from the pass, of the grant log, the revision and the complete ancestry
//! each of the pass's rights was decided from.
//!
//! The binding is its own compact JWS, typed [`BINDING_TYPE`], signed by the
//! same issuer key as the pass and published beside it in the token answer
//! only when the product asks for it. The pass's bytes are never changed:
//! the binding names the pass by the SHA-256 of its exact compact bytes, its
//! issuer, holder, audience and lifetime, so it cannot be moved to another
//! pass. A product maps each right to its dependency, keys its cache by
//! (grant log, grant id), and admits only once its grant stream stands at
//! [`BindingClaims::revision`] or later. A revocation names one grant; its
//! descendants are resolved here, from the paths.
//!
//! A product that requires bindings refuses a pass without one by
//! [`BINDING_REQUIRED`] and never assumes a revision of zero; a product
//! that does not ask keeps verifying passes exactly as before.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Error;
use crate::membership::GrantLog;
use crate::verify::{KeySet, VerifiedPass, signed_payload};

/// The binding version this crate reads and the issuer produces.
pub const BINDING_VERSION: u32 = 1;
/// The token request form value asking for a binding of this version.
pub const BINDING_REQUEST: &str = "1";
/// The JWS `typ` a binding is signed under, never a pass's `JWT`.
pub const BINDING_TYPE: &str = "lys-grant-binding+jwt";

/// A binding is required and absent.
pub const BINDING_REQUIRED: &str = "grant_binding_required";
/// The binding is of a version this crate does not read.
pub const BINDING_UNSUPPORTED: &str = "grant_binding_unsupported";
/// The binding names another pass, issuer, holder, audience or lifetime.
pub const BINDING_MISMATCH: &str = "grant_binding_mismatch";
/// The binding names another grant log or reset epoch.
pub const BINDING_LOG_MISMATCH: &str = "grant_binding_log_mismatch";
/// A dependency's ancestry is missing, repeated or not the pass's rights.
pub const BINDING_ANCESTRY: &str = "grant_binding_ancestry_refused";

/// One right's grant and its complete ancestry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(as = PassGrantDependency))]
pub struct Dependency {
    /// The grant a right of the pass names.
    pub grant: String,
    /// That grant and every ancestor, from the grant to its root.
    pub path: Vec<String>,
}

/// A binding's signed claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(as = PassGrantBinding))]
pub struct BindingClaims {
    /// The binding version, [`BINDING_VERSION`].
    pub binding: u32,
    /// The issuer, the pass's.
    pub iss: String,
    /// The holder, the pass's.
    pub sub: String,
    /// The audience, the pass's.
    pub aud: String,
    /// The pass's issue instant.
    pub iat: u64,
    /// The pass's first invalid instant.
    pub exp: u64,
    /// The SHA-256, lowercase hex, of the pass's exact compact bytes.
    pub pass: String,
    /// The grant log every dependency belongs to.
    pub log: GrantLog,
    /// The receipt revision the rights were decided at: a product admits
    /// only once its grant stream stands here or later.
    pub revision: u64,
    /// One dependency per grant the pass's rights name, in grant order.
    pub dependencies: Vec<Dependency>,
}

fn refused(name: &'static str, reason: &'static str) -> Error {
    Error::Binding { name, reason }
}

/// The SHA-256, lowercase hex, of a pass's exact compact bytes.
#[must_use]
pub fn pass_digest(token: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(token.as_bytes());
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// The binding a revocation-enabled product requires, or its absence by name.
pub fn required(binding: Option<&str>) -> Result<&str, Error> {
    binding
        .filter(|binding| !binding.is_empty())
        .ok_or_else(|| refused(BINDING_REQUIRED, "the pass carries no grant binding"))
}

/// A binding whose signature, pass, issuer, holder, audience, lifetime,
/// grant log and ancestry were checked against one verified pass.
#[derive(Debug)]
pub struct VerifiedBinding {
    claims: BindingClaims,
    by_grant: BTreeMap<String, usize>,
}

impl VerifiedBinding {
    /// Verify `binding` as the binding of the pass `token`, already verified
    /// as `pass`, on the grant log `log` the product's stream reads.
    pub fn verify(
        binding: &str,
        token: &str,
        pass: &VerifiedPass,
        keys: &KeySet,
        log: &GrantLog,
    ) -> Result<Self, Error> {
        let payload: serde_json::Value =
            serde_json::from_slice(&signed_payload(binding, keys, BINDING_TYPE)?)?;
        if payload.get("binding").and_then(serde_json::Value::as_u64)
            != Some(u64::from(BINDING_VERSION))
        {
            return Err(refused(
                BINDING_UNSUPPORTED,
                "the binding is not of a version this verifier reads",
            ));
        }
        let claims: BindingClaims = serde_json::from_value(payload)?;
        let signed = pass.claims();
        if claims.pass != pass_digest(token) {
            return Err(refused(BINDING_MISMATCH, "the binding names another pass"));
        }
        if (
            &claims.iss,
            &claims.sub,
            &claims.aud,
            claims.iat,
            claims.exp,
        ) != (
            &signed.iss,
            &signed.sub,
            &signed.aud,
            signed.iat,
            signed.exp,
        ) {
            return Err(refused(
                BINDING_MISMATCH,
                "the binding names another issuer, holder, audience or lifetime",
            ));
        }
        if &claims.log != log {
            return Err(refused(
                BINDING_LOG_MISMATCH,
                "the binding names another grant log or reset epoch",
            ));
        }
        let mut by_grant = BTreeMap::new();
        for (position, dependency) in claims.dependencies.iter().enumerate() {
            if dependency.path.first() != Some(&dependency.grant) {
                return Err(refused(
                    BINDING_ANCESTRY,
                    "a dependency's path does not begin at its grant",
                ));
            }
            let mut seen = BTreeSet::new();
            if dependency
                .path
                .iter()
                .any(|grant| grant.is_empty() || !seen.insert(grant))
            {
                return Err(refused(
                    BINDING_ANCESTRY,
                    "a dependency's path is empty or names a grant twice",
                ));
            }
            if by_grant
                .insert(dependency.grant.clone(), position)
                .is_some()
            {
                return Err(refused(BINDING_ANCESTRY, "a grant has two dependencies"));
            }
        }
        let rights: BTreeSet<&String> = signed.rights.iter().map(|right| &right.grant).collect();
        if rights.len() != by_grant.len()
            || rights.iter().any(|grant| !by_grant.contains_key(*grant))
        {
            return Err(refused(
                BINDING_ANCESTRY,
                "the dependencies are not exactly the grants the pass's rights name",
            ));
        }
        Ok(Self { claims, by_grant })
    }

    /// The verified claims.
    #[must_use]
    pub const fn claims(&self) -> &BindingClaims {
        &self.claims
    }

    /// The revision a product's stream must stand at before admitting.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.claims.revision
    }

    /// The dependency of the right resting on `grant`, as a decision of the
    /// pass names it.
    #[must_use]
    pub fn dependency(&self, grant: &str) -> Option<&Dependency> {
        self.by_grant
            .get(grant)
            .and_then(|position| self.claims.dependencies.get(*position))
    }
}
