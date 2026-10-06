//! The lines an app's virtual client credentials leave in the apps' log
//! (DIRECTORY-081). A credential's value is answered once by its issue and
//! kept nowhere; the broker holds its digest beside the app's sealed client
//! secret. The apps' record says which credentials are live: issued, not
//! revoked, and of an app not retired. Retiring an app ends every credential
//! it holds by the retirement's one line, whatever the broker answers; the
//! broker's own ending of them is recorded when it confirms it.

use serde::{Deserialize, Serialize};

use super::{App, By, Line};

/// A virtual client credential issued for an approved app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientCredentialIssued {
    /// The operation id it was issued with.
    pub operation: String,
    /// The app.
    pub app: String,
    /// The credential's id, as the broker named it.
    pub credential_id: String,
    /// The identity that owns the app's sealed client entry, on whose
    /// behalf the broker is asked to confirm the credential.
    pub owner: String,
    /// Who issued it.
    pub by: By,
    /// When.
    pub at: u64,
}

/// A client credential revoked by an administrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientCredentialRevoked {
    /// The operation id it was revoked with.
    pub operation: String,
    /// The app.
    pub app: String,
    /// The credential.
    pub credential_id: String,
    /// Why, in the administrator's words; may be empty.
    pub reason: String,
    /// Who revoked it.
    pub by: By,
    /// When.
    pub at: u64,
}

/// The broker's confirmation that it ended these credentials of the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientCredentialsEnded {
    /// The operation id the confirmation was kept under.
    pub operation: String,
    /// The app.
    pub app: String,
    /// The credentials the broker ended.
    pub credential_ids: Vec<String>,
    /// When.
    pub at: u64,
}

/// One client credential as the app's lines make it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCredential {
    /// Its issue.
    pub issued: ClientCredentialIssued,
    /// Its revocation, once it has one.
    pub revoked: Option<ClientCredentialRevoked>,
    /// Whether the broker has confirmed ending it.
    pub ended_at_broker: bool,
}

impl ClientCredential {
    /// Whether the credential is live on `app`: not revoked, and the app not
    /// retired.
    pub fn live_on(&self, app: &App) -> bool {
        self.revoked.is_none() && app.retired.is_none()
    }
}

impl App {
    /// Every client credential issued for the app, in the order issued.
    pub fn client_credentials(&self) -> Vec<ClientCredential> {
        let mut credentials: Vec<ClientCredential> = Vec::new();
        for line in &self.history {
            match line {
                Line::ClientCredentialIssued(issued) => credentials.push(ClientCredential {
                    issued: issued.clone(),
                    revoked: None,
                    ended_at_broker: false,
                }),
                Line::ClientCredentialRevoked(revoked) => {
                    if let Some(credential) = credentials
                        .iter_mut()
                        .find(|held| held.issued.credential_id == revoked.credential_id)
                    {
                        credential.revoked = Some(revoked.clone());
                    }
                }
                Line::ClientCredentialsEnded(ended) => {
                    for credential in &mut credentials {
                        if ended
                            .credential_ids
                            .contains(&credential.issued.credential_id)
                        {
                            credential.ended_at_broker = true;
                        }
                    }
                }
                _ => {}
            }
        }
        credentials
    }

    /// The ids of the app's live client credentials.
    pub fn live_client_credentials(&self) -> Vec<String> {
        self.client_credentials()
            .into_iter()
            .filter(|credential| credential.live_on(self))
            .map(|credential| credential.issued.credential_id)
            .collect()
    }

    /// The ids of the credentials no longer live that the broker has not
    /// yet confirmed ending.
    pub fn client_credentials_to_end(&self) -> Vec<String> {
        self.client_credentials()
            .into_iter()
            .filter(|credential| !credential.live_on(self) && !credential.ended_at_broker)
            .map(|credential| credential.issued.credential_id)
            .collect()
    }

    /// The custody owner the app's credentials were issued under.
    pub fn client_custody_owner(&self) -> Option<String> {
        self.history.iter().rev().find_map(|line| match line {
            Line::ClientCredentialIssued(issued) => Some(issued.owner.clone()),
            _ => None,
        })
    }
}
