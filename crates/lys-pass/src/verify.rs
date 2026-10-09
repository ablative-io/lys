//! Strict Ed25519 verification against a caller-owned published key cache.

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

use crate::Error;
use crate::rights::{Claims, Decision, Mode, Right, audience_owns, in_prefix};

#[derive(Deserialize)]
struct Header {
    alg: String,
    typ: String,
    kid: String,
    crit: Option<Vec<String>>,
    b64: Option<bool>,
}

#[derive(Deserialize)]
struct Jwk {
    kty: String,
    crv: String,
    kid: String,
    x: String,
    alg: Option<String>,
    #[serde(rename = "use")]
    usage: Option<String>,
    key_ops: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

/// The issuer's published Ed25519 verification keys, indexed by key id.
#[derive(Debug)]
pub struct KeySet {
    keys: BTreeMap<String, VerifyingKey>,
}

impl KeySet {
    /// Replace the cache only after the whole published keyset validates.
    pub fn from_json(json: &str) -> Result<Self, Error> {
        let published: Jwks = serde_json::from_str(json)?;
        let mut keys = BTreeMap::new();
        for key in published.keys {
            if key.kty != "OKP"
                || key.crv != "Ed25519"
                || key.kid.is_empty()
                || key.kid.chars().any(char::is_control)
                || key.alg.as_deref().is_some_and(|alg| alg != "EdDSA")
                || key.usage.as_deref().is_some_and(|usage| usage != "sig")
                || key
                    .key_ops
                    .as_ref()
                    .is_some_and(|ops| !ops.iter().any(|op| op == "verify"))
            {
                return Err(Error::Invalid(
                    "published key is not an Ed25519 signing key",
                ));
            }
            let bytes = URL_SAFE_NO_PAD.decode(&key.x)?;
            let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
                if bytes.is_empty() {
                    Error::Invalid("published key is empty")
                } else {
                    Error::Invalid("published Ed25519 key has the wrong length")
                }
            })?;
            let public = VerifyingKey::from_bytes(&bytes).map_err(Error::Signature)?;
            if public.is_weak() || keys.insert(key.kid, public).is_some() {
                return Err(Error::Invalid("published key is weak or duplicated"));
            }
        }
        Ok(Self { keys })
    }
}

type ActionIndex = BTreeMap<String, Vec<usize>>;
type ResourceIndex = BTreeMap<String, ActionIndex>;

/// Claims whose signature, issuer, audience and construction time were checked.
#[derive(Debug)]
pub struct VerifiedPass {
    claims: Claims,
    index: BTreeMap<String, ResourceIndex>,
}

impl VerifiedPass {
    /// Verify the complete signed token before trusting any permission claim.
    pub fn verify(
        token: &str,
        keys: &KeySet,
        issuer: &str,
        audience: &str,
        now: u64,
    ) -> Result<Self, Error> {
        if issuer.is_empty() || audience.is_empty() {
            return Err(Error::Invalid("trusted issuer or audience is missing"));
        }
        let mut parts = token.split('.');
        let header = parts.next().ok_or(Error::Invalid("missing header"))?;
        let payload = parts.next().ok_or(Error::Invalid("missing claims"))?;
        let signature = parts.next().ok_or(Error::Invalid("missing signature"))?;
        if parts.next().is_some() {
            return Err(Error::Invalid("too many token segments"));
        }
        let decoded: Header = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(header)?)?;
        if decoded.alg != "EdDSA"
            || decoded.typ != "JWT"
            || decoded.kid.is_empty()
            || decoded.kid.chars().any(char::is_control)
            || decoded
                .crit
                .as_ref()
                .is_some_and(|values| !values.is_empty())
            || decoded.b64 == Some(false)
        {
            return Err(Error::Invalid("unsupported signed token header"));
        }
        let key = keys
            .keys
            .get(&decoded.kid)
            .ok_or_else(|| Error::UnpublishedKey(decoded.kid.clone()))?;
        let signature =
            Signature::from_slice(&URL_SAFE_NO_PAD.decode(signature)?).map_err(Error::Signature)?;
        key.verify_strict(format!("{header}.{payload}").as_bytes(), &signature)
            .map_err(Error::Signature)?;
        let claims: Claims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload)?)?;
        if claims.iss != issuer {
            return Err(Error::WrongIssuer);
        }
        if claims.aud != audience {
            return Err(Error::WrongAudience);
        }
        if claims.sub.is_empty()
            || claims.sub != claims.holder.id
            || !matches!(
                claims.holder.kind.as_str(),
                "person" | "agent" | "connector" | "service_account" | "machine"
            )
            || (claims.holder.kind != "person"
                && claims
                    .holder
                    .responsible
                    .as_deref()
                    .is_none_or(str::is_empty))
        {
            return Err(Error::Invalid("holder or responsibility is missing"));
        }
        if claims.exp <= claims.iat {
            return Err(Error::Invalid("pass lifetime is empty"));
        }
        if claims.exp <= now {
            return Err(Error::Expired);
        }
        if claims.iat > now || claims.nbf.is_some_and(|nbf| nbf > now) {
            return Err(Error::NotYetValid);
        }
        if claims
            .rights_truncated
            .as_deref()
            .is_some_and(|app| app != audience)
        {
            return Err(Error::RightsOutsideAudience);
        }
        let mut index: BTreeMap<String, ResourceIndex> = BTreeMap::new();
        for (position, right) in claims.rights.iter().enumerate() {
            if !audience_owns(&right.resource.kind, audience) {
                return Err(Error::RightsOutsideAudience);
            }
            if right.resource.kind.is_empty()
                || right.resource.id.is_empty()
                || right.grant.is_empty()
                || right.actions.is_empty()
                || right.resource.kind.chars().any(char::is_control)
                || right.resource.id.chars().any(char::is_control)
            {
                return Err(Error::Invalid("effective right is incomplete"));
            }
            for action in &right.actions {
                if action.is_empty() || action.chars().any(char::is_control) {
                    return Err(Error::Invalid("effective action is invalid"));
                }
                index
                    .entry(right.resource.kind.clone())
                    .or_default()
                    .entry(right.resource.id.clone())
                    .or_default()
                    .entry(action.clone())
                    .or_default()
                    .push(position);
            }
        }
        Ok(Self { claims, index })
    }

    /// The verified claims, borrowed without cloning the pass.
    #[must_use]
    pub const fn claims(&self) -> &Claims {
        &self.claims
    }

    /// Decide an exact target from the index; reach is never recomputed locally.
    pub fn evaluate(
        &self,
        kind: &str,
        id: &str,
        action: &str,
        now: u64,
    ) -> Result<Decision<'_>, Error> {
        if self.claims.exp <= now {
            return Err(Error::Expired);
        }
        if self.claims.iat > now || self.claims.nbf.is_some_and(|nbf| nbf > now) {
            return Err(Error::NotYetValid);
        }
        if let Some(app) = &self.claims.rights_truncated {
            return Err(Error::Truncated(app.clone()));
        }
        let Some(entries) = self
            .index
            .get(kind)
            .and_then(|ids| ids.get(id))
            .and_then(|actions| actions.get(action))
        else {
            return Ok(Decision::Refused);
        };
        let mut held = None;
        for position in entries {
            let right = self
                .claims
                .rights
                .get(*position)
                .ok_or(Error::Invalid("right index is inconsistent"))?;
            if right.mode == Mode::Outright {
                return Ok(Decision::Allowed {
                    grant: &right.grant,
                });
            }
            if held.is_none() {
                held = Some(Decision::Held {
                    grant: &right.grant,
                    mode: right.mode,
                });
            }
        }
        Ok(held.unwrap_or(Decision::Refused))
    }

    /// Hot routes accept only an outright, unexpired and complete grant.
    pub fn hot_allowed(&self, kind: &str, id: &str, action: &str, now: u64) -> Result<bool, Error> {
        Ok(matches!(
            self.evaluate(kind, id, action, now)?,
            Decision::Allowed { .. }
        ))
    }

    /// Enumerate explicitly signed rights under a kind prefix, without granting descendants.
    pub fn rights_in_prefix<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a Right> + 'a {
        self.claims
            .rights
            .iter()
            .filter(move |right| in_prefix(&right.resource.kind, prefix))
    }
}
