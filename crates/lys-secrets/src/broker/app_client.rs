//! An app's virtual client credentials (DIRECTORY-081). An approved app's
//! client secret is sealed once, at approval, as `lys-app-{owner}-{app}-client`
//! and never leaves the broker. A product is issued a credential made for
//! it instead, `lys-client.{app}.{64 hex}`, answered once and kept here only
//! as its SHA-256, sealed beside the client entry as
//! `lys-app-{owner}-{app}-client-{credential id}`. The token exchange asks the
//! broker to confirm a presented credential; ending one retires its entry,
//! so its name is never sealed again and it is never confirmed again.

use crate::audit::AuditKind;
use crate::encoding::{hex, random_bytes, sha256};
use crate::{AppClientRefusal, Broker, EntryClass, PermissionCheck, Secret, SecretsError};

/// What every virtual client credential's value begins with.
pub const APP_CLIENT_PREFIX: &str = "lys-client.";

/// A credential as issued. Its value is answered once and kept nowhere.
pub struct IssuedAppClient {
    /// The credential's id, which names it everywhere but its value.
    pub credential_id: String,
    /// The value the product presents.
    pub value: Secret,
    /// The identity that owns the app's sealed client entry.
    pub owner: String,
    /// The confirmed current client-secret digest, used to settle missing custody.
    pub client_secret_sha256: String,
}

/// The app id `app`, refused by name unless it is one.
pub(super) fn app_named(app: &str) -> Result<(), SecretsError> {
    lys_identity::grants::schema::app_id(app).map_err(|error| match error {
        lys_identity::grants::schema::SchemaError::AppIdInvalid { id, reason } => {
            SecretsError::InvalidName {
                what: "app",
                name: id,
                reason,
            }
        }
        error @ lys_identity::grants::schema::SchemaError::Invalid { .. } => {
            SecretsError::Encoding {
                context: "validate app id",
                reason: error.to_string(),
            }
        }
    })?;
    Ok(())
}

/// Whether `left` and `right` are the same bytes, compared in constant time
/// for equal lengths.
fn same(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && left.iter().zip(right).fold(0_u8, |d, (a, b)| d | (a ^ b)) == 0
}

impl<P: PermissionCheck> Broker<P> {
    /// Rotate custody under an operation, returning owner, bearer reference,
    /// current digest and the value only on first creation. The sealed operation
    /// value reconciles a partial rotation after reopening.
    ///
    /// # Errors
    /// Refuses invalid names, stale custody and conflicting owners; propagates
    /// random-source, store and audit failures.
    pub fn issue_app_bearer(
        &mut self,
        app: &str,
        by: &str,
        operation: &str,
        expected_digest: &str,
    ) -> Result<(String, String, String, Option<Secret>), SecretsError> {
        app_named(app)?;
        operation.parse::<lys_identity::OperationId>()?;
        if expected_digest.len() != 64 || !expected_digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(SecretsError::InvalidName {
                what: "digest", name: expected_digest.to_owned(), reason: "expected a SHA-256 hex digest",
            });
        }
        let (client, owner) = match self.app_client_entry(app) {
            Ok(entry) => entry,
            Err(SecretsError::AppClient(AppClientRefusal::NoCustody { .. })) => (format!("lys-app-{by}-{app}-client"), by.to_owned()),
            Err(error) => return Err(error),
        };
        let prefix = format!("lys-app-{owner}-{app}");
        let staged = format!("{prefix}-custody-{operation}");
        let repeated = self.store.entry(&staged).is_some();
        let value = if let Some(entry) = self.store.entry(&staged) {
            if entry.owner != owner || entry.class != EntryClass::Key {
                return Err(SecretsError::SecretExists { name: staged });
            }
            self.store.open_for_use(&self.store_key, &staged, EntryClass::Key)?
        } else {
            Secret::new(hex(&random_bytes::<32>()?).into_bytes())
        };
        let digest = hex(&sha256(value.expose()));
        if let Some(entry) = self.store.entry(&client) {
            if entry.owner != owner || entry.class != EntryClass::Key {
                return Err(SecretsError::SecretExists { name: client });
            }
            let held = self.store.open_for_use(&self.store_key, &client, EntryClass::Key)?;
            let current = hex(&sha256(held.expose()));
            if !same(current.as_bytes(), expected_digest.as_bytes()) && !same(current.as_bytes(), digest.as_bytes()) {
                return Err(AppClientRefusal::CustodyMismatch { app: app.to_owned() }.into());
            }
        }
        self.seal_once(&staged, EntryClass::Key, &owner, &value)?;
        let mut bytes = zeroize::Zeroizing::new(format!("lys-app.{app}.").into_bytes());
        bytes.extend_from_slice(value.expose());
        let credential = Secret::from_slice(&bytes);
        let reference = format!("{prefix}-api-{operation}");
        self.seal_once(&reference, EntryClass::Credential, &owner, &credential)?;
        let api = format!("{prefix}-api");
        if let Some(entry) = self.store.entry(&api) {
            if entry.owner != owner || entry.class != EntryClass::Credential {
                return Err(SecretsError::SecretExists { name: api });
            }
            let held = self.store.open_for_use(&self.store_key, &api, EntryClass::Credential)?;
            if !same(held.expose(), credential.expose()) {
                self.store.replace(&self.store_key, &api, &credential)?;
            }
        } else {
            self.seal_once(&api, EntryClass::Credential, &owner, &credential)?;
        }
        if self.store.entry(&client).is_some() {
            let held = self.store.open_for_use(&self.store_key, &client, EntryClass::Key)?;
            if !same(held.expose(), value.expose()) {
                self.store.replace(&self.store_key, &client, &value)?;
            }
            self.store.confirm_index()?;
        } else {
            self.seal_once(&client, EntryClass::Key, &owner, &value)?;
        }
        self.record(AuditKind::Issue, (None, Some(by), Some(&reference)), None, None, "app custody confirmed")?;
        Ok((owner, reference, digest, if repeated { None } else { Some(credential) }))
    }

    /// The app's sealed client entry, by name, with its owner. App ids hold
    /// no hyphen, so the entry ending `-{app}-client` is this app's alone.
    fn app_client_entry(&self, app: &str) -> Result<(String, String), SecretsError> {
        let suffix = format!("-{app}-client");
        self.store
            .entries()
            .find(|entry| {
                entry.class == EntryClass::Key
                    && entry.name.starts_with("lys-app-")
                    && entry.name.ends_with(&suffix)
            })
            .map(|entry| (entry.name.clone(), entry.owner.clone()))
            .ok_or_else(|| {
                AppClientRefusal::NoCustody {
                    app: app.to_owned(),
                }
                .into()
            })
    }

    /// Issues `app` a new virtual client credential, as `by`. Only its
    /// SHA-256 is sealed; the value is answered once.
    ///
    /// # Errors
    /// `InvalidName` for an app id that is not one, `AppClientNoCustody`
    /// when the app's client secret is not sealed, and the store's, the
    /// random source's and the audit log's failures.
    pub fn issue_app_client(
        &mut self,
        app: &str,
        by: &str,
    ) -> Result<IssuedAppClient, SecretsError> {
        app_named(app)?;
        let (client, owner) = self.app_client_entry(app)?;
        let sealed_client = self.store.open_for_use(&self.store_key, &client, EntryClass::Key)?;
        let client_secret_sha256 = hex(&sha256(sealed_client.expose()));
        let credential_id = hex(&random_bytes::<8>()?);
        let value = Secret::new(
            format!("{APP_CLIENT_PREFIX}{app}.{}", hex(&random_bytes::<32>()?)).into_bytes(),
        );
        let digest = Secret::new(hex(&sha256(value.expose())).into_bytes());
        let name = format!("{client}-{credential_id}");
        self.seal_once(&name, EntryClass::Key, &owner, &digest)?;
        self.record(
            AuditKind::Issue,
            (None, Some(by), Some(&name)),
            None,
            None,
            &format!("issued app client credential {credential_id} for {app} by {by}"),
        )?;
        Ok(IssuedAppClient {
            credential_id,
            value,
            owner,
            client_secret_sha256,
        })
    }

    /// Confirms that `presented` is a credential issued for `app`, live in
    /// the apps' record (`live`), and that the app's sealed client secret is
    /// the one its approval records (`secret_sha256`). Answers the
    /// credential's id. A credential the apps' record no longer holds live
    /// is ended here and refused. Every answer writes one audit line, which
    /// holds no value.
    ///
    /// # Errors
    /// `AppClientNoCustody`, `AppClientCustodyMismatch`,
    /// `AppClientCredentialRefused`, and the store's and the audit log's
    /// failures.
    pub fn authenticate_app_client(
        &mut self,
        app: &str,
        presented: &Secret,
        live: &[String],
        secret_sha256: &str,
    ) -> Result<String, SecretsError> {
        app_named(app)?;
        match self.confirmed_app_client(app, presented, live, secret_sha256) {
            Ok((name, credential_id)) => {
                self.record(
                    AuditKind::Use,
                    (None, None, Some(&name)),
                    None,
                    None,
                    &format!("client authenticated for {app} with {credential_id}"),
                )?;
                Ok(credential_id)
            }
            Err(error) => {
                self.record(
                    AuditKind::Use,
                    (None, None, None),
                    None,
                    None,
                    &format!("refused {} for {app}", error.name()),
                )?;
                Err(error)
            }
        }
    }

    fn confirmed_app_client(
        &mut self,
        app: &str,
        presented: &Secret,
        live: &[String],
        secret_sha256: &str,
    ) -> Result<(String, String), SecretsError> {
        let (client, owner) = self.app_client_entry(app)?;
        let sealed = self
            .store
            .open_for_use(&self.store_key, &client, EntryClass::Key)?;
        if !same(
            hex(&sha256(sealed.expose())).as_bytes(),
            secret_sha256.as_bytes(),
        ) {
            return Err(AppClientRefusal::CustodyMismatch {
                app: app.to_owned(),
            }
            .into());
        }
        let refused = || -> SecretsError {
            AppClientRefusal::Refused {
                app: app.to_owned(),
            }
            .into()
        };
        if !presented
            .expose()
            .starts_with(format!("{APP_CLIENT_PREFIX}{app}.").as_bytes())
        {
            return Err(refused());
        }
        let digest = hex(&sha256(presented.expose()));
        let prefix = format!("{client}-");
        let names: Vec<String> = self
            .store
            .entries()
            .filter(|entry| entry.class == EntryClass::Key && entry.name.starts_with(&prefix))
            .map(|entry| entry.name.clone())
            .collect();
        let mut found = None;
        for name in names {
            let held = self
                .store
                .open_for_use(&self.store_key, &name, EntryClass::Key)?;
            if same(held.expose(), digest.as_bytes()) {
                found = Some(name);
            }
        }
        let name = found.ok_or_else(refused)?;
        let credential_id = name.strip_prefix(&prefix).ok_or_else(refused)?.to_owned();
        if !live.contains(&credential_id) {
            self.end_app_client(
                app,
                &name,
                &owner,
                &credential_id,
                "no longer live in the apps' record",
            )?;
            return Err(refused());
        }
        Ok((name, credential_id))
    }

    /// Ends each of `credential_ids` issued for `app`, as `by`, for the
    /// reason `why`. One already ended, or never issued, is passed over, so
    /// a repeated ending is answered the same. Answers the ids ended now.
    ///
    /// # Errors
    /// `InvalidName` for an app id that is not one, and the store's and the
    /// audit log's failures.
    pub fn end_app_clients(
        &mut self,
        app: &str,
        credential_ids: &[String],
        by: &str,
        why: &str,
    ) -> Result<Vec<String>, SecretsError> {
        app_named(app)?;
        let (client, owner) = match self.app_client_entry(app) {
            Ok(found) => found,
            Err(SecretsError::AppClient(AppClientRefusal::NoCustody { .. })) => {
                return Ok(Vec::new());
            }
            Err(error) => return Err(error),
        };
        let mut ended = Vec::new();
        for credential_id in credential_ids {
            let name = format!("{client}-{credential_id}");
            if self.store.entry(&name).is_some() {
                self.end_app_client(app, &name, &owner, credential_id, &format!("{why} by {by}"))?;
                ended.push(credential_id.clone());
            }
        }
        Ok(ended)
    }

    fn end_app_client(
        &mut self,
        app: &str,
        name: &str,
        owner: &str,
        credential_id: &str,
        why: &str,
    ) -> Result<(), SecretsError> {
        let at = (self.clock)();
        self.store.retire(name, owner, at)?;
        self.record(
            AuditKind::Drop,
            (None, Some(owner), Some(name)),
            None,
            None,
            &format!("ended app client credential {credential_id} for {app}: {why}"),
        )?;
        Ok(())
    }
}
