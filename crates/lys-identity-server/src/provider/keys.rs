//! The keys passes and ID tokens are signed with, and their rotation
//! (ACCESS-002 R3).
//!
//! Every token is signed with the current key and names it by its key id
//! (`kid`, the first eight bytes of its public key's SHA-256, in hex). The
//! administrator rotates the key (`POST /oauth/jwks/rotate`): a new seed is
//! made beside the install's own, the key it replaces is kept as retired
//! with the instant it was retired, and the current key and every retired
//! key some unexpired token may still name are published at `/oauth/jwks`.
//! A retired key is published for one lifetime after its retirement (the
//! longer of a pass's and an ID token's), and no longer, so a product
//! verifies any unexpired token with the published set alone.
//!
//! What a rotation keeps is one table beside the install's key file,
//! `<key file>.keys.json`, written whole and durably (`token_store::save`):
//! the current seed's file name, the operation that made it and each
//! retired key's public half. With no table the install's own key file is
//! the current key, as it was before rotation existed. The rotation's
//! operation is kept, so the same rotation sent again rotates once.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::endpoints::{malformed, provider};
use super::{held, random, unavailable};
use crate::error::ServerError;
use crate::routes::{AppState, administrator, hex, signed_in};
use crate::session::now;

const FORMAT: &str = "lys-provider-keys/v1";

/// The rotation's table, as written.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    format: String,
    /// The current seed's file name, beside the install's key file.
    current: String,
    /// The operation the current key was made by.
    operation: String,
    retired: Vec<Retired>,
}

/// A key no longer signing, published while a token it signed may live.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Retired {
    kid: String,
    /// The public key, base64url, as the key set publishes it.
    x: String,
    retired_at: u64,
}

/// The administrator's rotation of the signing key.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RotateBody {
    /// The operation id the rotation is kept under; sent again, the same
    /// rotation is answered and no second key is made.
    pub operation: String,
}

/// The keys after a rotation.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RotateAnswer {
    /// The key id every token is now signed with.
    pub current: String,
    /// The key ids published, the current one first.
    pub published: Vec<String>,
}

/// The current signing key and the retired keys still published.
pub(super) struct SigningKeys {
    key_file: PathBuf,
    current: Ed25519Identity,
    kid: String,
    current_file: PathBuf,
    retired: Vec<Retired>,
    operation: Option<String>,
}

fn kid_of(key: &Ed25519Identity) -> String {
    let digest = Sha256::digest(key.public_key_bytes());
    hex(digest.get(..8).unwrap_or_default())
}

fn table_of(key_file: &Path) -> PathBuf {
    key_file.with_extension("keys.json")
}

fn jwk(kid: &str, x: &str) -> Value {
    json!({
        "kty": "OKP",
        "crv": "Ed25519",
        "use": "sig",
        "alg": "EdDSA",
        "kid": kid,
        "x": x,
    })
}

fn load(path: &Path) -> Result<Ed25519Identity, ServerError> {
    Ed25519Identity::load(path).map_err(|error| ServerError::ConfigInvalid {
        reason: format!(
            "the provider's signing key {} cannot be read: {error}",
            path.display()
        ),
    })
}

impl SigningKeys {
    /// The keys beside `key_file`: the rotation's table when there is one,
    /// else the install's key file alone.
    pub(super) fn open(key_file: &Path) -> Result<Self, ServerError> {
        let table_file = table_of(key_file);
        match std::fs::read(&table_file) {
            Ok(bytes) => {
                let table: Table = serde_json::from_slice(&bytes).map_err(|error| {
                    unavailable(format!(
                        "the signing keys' table {} is malformed: {error}",
                        table_file.display()
                    ))
                })?;
                let named = Path::new(&table.current);
                if table.format != FORMAT || named.file_name() != Some(OsStr::new(&table.current)) {
                    return Err(unavailable(format!(
                        "the signing keys' table {} has an invalid format or names a key outside its folder",
                        table_file.display()
                    )));
                }
                let current_file = key_file.with_file_name(&table.current);
                let current = load(&current_file)?;
                Ok(Self {
                    key_file: key_file.to_owned(),
                    kid: kid_of(&current),
                    current,
                    current_file,
                    retired: table.retired,
                    operation: Some(table.operation),
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let current = load(key_file)?;
                Ok(Self {
                    key_file: key_file.to_owned(),
                    kid: kid_of(&current),
                    current,
                    current_file: key_file.to_owned(),
                    retired: Vec::new(),
                    operation: None,
                })
            }
            Err(error) => Err(unavailable(format!(
                "the signing keys' table {} could not be read: {error}",
                table_file.display()
            ))),
        }
    }

    /// The current key's id.
    pub(super) fn kid(&self) -> &str {
        &self.kid
    }

    /// `claims` as a compact JWS signed with the current key, naming it.
    pub(super) fn signed(&self, claims: &Value) -> String {
        let header = json!({ "alg": "EdDSA", "typ": "JWT", "kid": self.kid });
        let input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header.to_string()),
            URL_SAFE_NO_PAD.encode(claims.to_string())
        );
        let signature = URL_SAFE_NO_PAD.encode(self.current.sign(input.as_bytes()));
        format!("{input}.{signature}")
    }

    /// The retired keys still published at `at`, each retired less than
    /// `window` seconds before it, once each and never the current one.
    fn live_retired(&self, at: u64, window: u64) -> impl Iterator<Item = &Retired> {
        self.retired
            .iter()
            .enumerate()
            .filter_map(move |(index, retired)| {
                let earlier = self.retired.get(..index).unwrap_or_default();
                let fresh = retired.retired_at.saturating_add(window) > at
                    && retired.kid != self.kid
                    && !earlier.iter().any(|before| before.kid == retired.kid);
                fresh.then_some(retired)
            })
    }

    /// The published key set at `at`: the current key, then every retired
    /// key a token may still name.
    pub(super) fn published(&self, at: u64, window: u64) -> Value {
        let current = URL_SAFE_NO_PAD.encode(self.current.public_key_bytes());
        let mut keys = vec![jwk(&self.kid, &current)];
        keys.extend(
            self.live_retired(at, window)
                .map(|retired| jwk(&retired.kid, &retired.x)),
        );
        json!({ "keys": keys })
    }

    /// What a rotation answers: the current key id and every published one.
    pub(super) fn answer(&self, at: u64, window: u64) -> RotateAnswer {
        let mut published = vec![self.kid.clone()];
        published.extend(
            self.live_retired(at, window)
                .map(|retired| retired.kid.clone()),
        );
        RotateAnswer {
            current: self.kid.clone(),
            published,
        }
    }

    /// Make a new current key under `operation` at `at`, keeping the one it
    /// replaces published for `window` seconds. The table is written before
    /// anything signs with the new key; the replaced seed is then removed,
    /// unless it is the install's own key file. The same operation again
    /// rotates nothing.
    pub(super) fn rotate(
        &mut self,
        operation: &str,
        at: u64,
        window: u64,
    ) -> Result<(), ServerError> {
        if self.operation.as_deref() == Some(operation) {
            return Ok(());
        }
        let stem = self
            .key_file
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = format!("{stem}.{}.key", random::<12>()?);
        let next_file = self.key_file.with_file_name(&name);
        let next = Ed25519Identity::load_or_generate(&next_file).map_err(|error| {
            unavailable(format!("a new signing key could not be made: {error}"))
        })?;
        let mut retired: Vec<Retired> = self
            .retired
            .iter()
            .filter(|retired| retired.retired_at.saturating_add(window) > at)
            .cloned()
            .collect();
        retired.push(Retired {
            kid: self.kid.clone(),
            x: URL_SAFE_NO_PAD.encode(self.current.public_key_bytes()),
            retired_at: at,
        });
        let table = Table {
            format: FORMAT.to_owned(),
            current: name,
            operation: operation.to_owned(),
            retired,
        };
        let bytes = serde_json::to_vec(&table).map_err(|error| {
            unavailable(format!(
                "the signing keys' table could not be encoded: {error}"
            ))
        })?;
        super::token_store::save("the signing keys' table", &table_of(&self.key_file), &bytes)?;
        let replaced = std::mem::replace(&mut self.current_file, next_file);
        self.kid = kid_of(&next);
        self.current = next;
        self.retired = table.retired;
        self.operation = Some(table.operation);
        // Only the replaced key's public half is published from here on; its
        // seed is removed, except the install's own key file.
        if replaced != self.key_file
            && let Err(error) = std::fs::remove_file(&replaced)
        {
            tracing::warn!(
                "the replaced signing key {} could not be removed: {error}",
                replaced.display()
            );
        }
        Ok(())
    }
}

/// `POST /oauth/jwks/rotate`: the administrator rotates the signing key.
pub(super) async fn rotate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<RotateBody>, JsonRejection>,
) -> Result<Json<RotateAnswer>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    administrator(&state, &actor)?;
    let Json(body) = body.map_err(|refused| malformed(&refused.body_text()))?;
    body.operation
        .parse::<OperationId>()
        .map_err(|_malformed| malformed("operation is an operation id"))?;
    let provider = provider(&state)?;
    let at = now();
    let window = provider.published_seconds();
    let mut keys = held(&provider.keys)?;
    keys.rotate(&body.operation, at, window)?;
    Ok(Json(keys.answer(at, window)))
}

#[cfg(test)]
mod tests {
    use lys_pass::{Claims, Holder, KeySet, VerifiedPass};
    use serde_json::Value;

    use super::SigningKeys;

    const ISSUER: &str = "http://localhost:8490";
    const APP: &str = "notes";
    const PERSON: &str = "person-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    /// One pass lifetime, as the provider would publish a retired key for.
    const WINDOW: u64 = 600;

    fn claims(iat: u64, exp: u64) -> Result<Value, serde_json::Error> {
        serde_json::to_value(Claims {
            iss: ISSUER.to_owned(),
            sub: PERSON.to_owned(),
            aud: APP.to_owned(),
            iat,
            exp,
            nbf: None,
            holder: Holder {
                id: PERSON.to_owned(),
                kind: "person".to_owned(),
                responsible: None,
            },
            rights: Vec::new(),
            rights_truncated: None,
        })
    }

    fn set(keys: &SigningKeys, at: u64) -> Result<KeySet, lys_pass::Error> {
        KeySet::from_json(&keys.published(at, WINDOW).to_string())
    }

    /// ACCESS-002 R3: rotate; a pass from before the rotation verifies with
    /// the published set until its expiry and not after; the old key leaves
    /// the set one lifetime after the rotation; the table read again says
    /// the same; the same rotation sent again makes no second key.
    #[test]
    fn a_pass_from_before_a_rotation_verifies_until_its_expiry_and_its_key_leaves_after_one_lifetime()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let key_file = dir.path().join("provider.key");
        std::fs::write(&key_file, [4u8; 32])?;
        let mut keys = SigningKeys::open(&key_file)?;
        let before = keys.signed(&claims(100, 700)?);
        let old = keys.kid().to_owned();
        keys.rotate("operation-1", 200, WINDOW)?;
        assert_ne!(keys.kid(), old);
        let after = keys.signed(&claims(200, 800)?);
        assert_eq!(
            keys.answer(300, WINDOW).published,
            [keys.kid(), old.as_str()]
        );

        let published = set(&keys, 300)?;
        VerifiedPass::verify(&before, &published, ISSUER, APP, 300)?;
        VerifiedPass::verify(&after, &published, ISSUER, APP, 300)?;
        assert!(matches!(
            VerifiedPass::verify(&before, &set(&keys, 700)?, ISSUER, APP, 700),
            Err(lys_pass::Error::Expired)
        ));
        let later = set(&keys, 800)?;
        assert!(matches!(
            VerifiedPass::verify(&before, &later, ISSUER, APP, 650),
            Err(lys_pass::Error::UnpublishedKey(kid)) if kid == old
        ));
        VerifiedPass::verify(&after, &later, ISSUER, APP, 799)?;
        assert_eq!(keys.answer(800, WINDOW).published, [keys.kid()]);

        let reopened = SigningKeys::open(&key_file)?;
        assert_eq!(reopened.kid(), keys.kid());
        assert_eq!(reopened.published(300, WINDOW), keys.published(300, WINDOW));
        let current = keys.kid().to_owned();
        keys.rotate("operation-1", 250, WINDOW)?;
        assert_eq!(keys.kid(), current);
        assert!(key_file.is_file(), "the install's own key file is kept");
        Ok(())
    }
}
