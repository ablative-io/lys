//! What the apps' log folds to, and how that fold is sealed in the log's
//! signed snapshot so a start reads only the leaves after it.
//!
//! An app is a record: registered with its id, name, redirect addresses and
//! schema, and of no effect until an administrator approves it. Approval
//! makes the registered schema version 1 and records the app's sign-in
//! client by its id and the SHA-256 of its secret, never the secret. A schema
//! change is applied as the next version, or proposed and applied only when
//! an administrator approves it; every version stays in the record. A
//! retired app stays in the record, with its versions, and nothing it held
//! is removed. The app `lys` is recorded once, from Lys's own model, and is
//! never retired. Every operation id names one line only, except that an
//! approval's sign-in settings and its connector are kept beside it under the
//! approval's operation, so that an approval is complete without a second
//! act; an administrator's later change of those settings is a line of its
//! own, and the latest is what the provider reads at each request. An app
//! holds one connector or none (DIRECTORY-080 R1), and a connector line
//! written for an app approved before connectors has an operation of its own.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[path = "apps_index.rs"]
mod index;

#[path = "apps_fold.rs"]
mod fold;

#[path = "apps_client_lines.rs"]
mod client_lines;
pub use client_lines::{
    ClientCredential, ClientCredentialIssued, ClientCredentialRevoked, ClientCredentialsEnded,
    CustodyPrepared,
};

#[cfg(test)]
#[path = "apps_operation_tests.rs"]
mod operation_tests;

#[cfg(test)]
#[path = "apps_connector_tests.rs"]
mod connector_tests;

use crate::apps_binding::{Binding, Registrar};
use crate::read_views::Login;
use fold::{allows_on, apply, beside_its_approval, ids_read, lys_app};

/// The snapshot domain the apps' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/apps-state/v1";

const FORMAT: &str = "lys-apps-state/v1";

/// Who made an act on the apps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum By {
    /// A signed-in person, by their login.
    Person {
        /// The login.
        login: Login,
    },
    /// The install's operator token, acting under the administrator's login.
    /// Its provenance distinguishes authority from a person signing in.
    Operator {
        /// The administrator's login, not a claim that the person signed in.
        login: Login,
    },
    /// A service account, by its id.
    ServiceAccount {
        /// The service account's id.
        id: String,
    },
    /// The service itself, at start.
    Start,
}

/// An app as it was registered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registered {
    /// The operation id it was registered with.
    pub operation: String,
    /// The app's id, which is its kinds' prefix.
    pub app: String,
    /// Its name as people read it.
    pub name: String,
    /// The addresses its sign-in client may send a person back to.
    pub redirects: Vec<String>,
    /// Its schema as it was sent, already checked.
    pub schema: Value,
    /// The service account the app acts as, bound on approval.
    pub service_account: Option<String>,
    /// Who registered it.
    pub by: By,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// An app's sign-in client, by its id and the SHA-256 of its secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    /// The client id.
    pub client_id: String,
    /// The SHA-256 of the client secret, in hex. The secret itself is made
    /// and sealed by the secrets broker at approval and shown to nobody.
    pub secret_sha256: String,
}

/// An app's registration approved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approved {
    /// The operation id it was approved with.
    pub operation: String,
    /// The app.
    pub app: String,
    /// The client created at approval.
    pub client: Client,
    /// The binding of the registration's service account to the app, when it names one.
    pub binding: Option<Binding>,
    /// Who approved it.
    pub by: By,
    /// When.
    pub at: u64,
}

/// An approved app's sign-in settings: the exact addresses its client may
/// send a person back to, and whether its ID token carries the person's
/// name (the profile scope). Kept beside the approval under the approval's
/// operation, and under an operation of its own each time an administrator
/// changes them; the latest is in force at the provider's next request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignInSet {
    /// The operation id it was kept under: the approval's, or its own.
    pub operation: String,
    /// The app.
    pub app: String,
    /// The addresses the sign-in client may send a person back to, exactly.
    pub redirects: Vec<String>,
    /// Whether the app is given the person's name.
    pub profile: bool,
    /// Who set them.
    pub by: By,
    /// When.
    pub at: u64,
}

/// A registration or a change declined, or an app retired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decided {
    /// The operation id it was made with.
    pub operation: String,
    /// The app.
    pub app: String,
    /// Why, in the words of whoever made it; empty when they said nothing.
    pub reason: String,
    /// Who made it.
    pub by: By,
    /// When.
    pub at: u64,
}

/// A schema change waiting for an administrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposed {
    /// The operation id it was proposed with.
    pub operation: String,
    /// The app.
    pub app: String,
    /// The version it replaces.
    pub replaces: u64,
    /// The schema proposed, already checked.
    pub schema: Value,
    /// Who proposed it.
    pub by: By,
    /// When.
    pub at: u64,
}

/// A schema version made current.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Applied {
    /// The operation id it was applied with.
    pub operation: String,
    /// The app.
    pub app: String,
    /// The version it is.
    pub version: u64,
    /// The schema.
    pub schema: Value,
    /// The proposal it approves, when it approves one.
    pub proposal: Option<String>,
    /// Who applied it.
    pub by: By,
    /// When.
    pub at: u64,
}

/// A resource of an app kind placed in its parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placed {
    /// The operation id it was placed with.
    pub operation: String,
    /// The app whose kinds they are.
    pub app: String,
    /// The child's kind.
    pub child_kind: String,
    /// The child's id.
    pub child_id: String,
    /// The parent's kind.
    pub parent_kind: String,
    /// The parent's id.
    pub parent_id: String,
    /// Whether the placement is restricted (ACCESS-004 R2): the parent's
    /// relations do not flow to the child, and only a grant on the child
    /// reaches it. Absent from a line kept before restriction existed, which
    /// keeps its bytes, and from every unrestricted placement.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub restricted: bool,
    /// Who placed it.
    pub by: By,
    /// When.
    pub at: u64,
}

/// An approved app's connector (DIRECTORY-080 R1): the identity the app gives
/// and holds grants as, answering to the administrator who approved it, by
/// that person's id as resolved from their login when the line was written.
/// Kept beside the approval under the approval's operation when the approval
/// writes it, and under an operation of its own when an administrator gives
/// one to an app approved before connectors were.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connected {
    /// The operation id it was kept under.
    pub operation: String,
    /// The app.
    pub app: String,
    /// The connector's id: `connector-` and 32 hex digits.
    pub connector: String,
    /// The approving administrator's person id.
    pub approver: String,
    /// Who wrote it.
    pub by: By,
    /// When.
    pub at: u64,
}

/// Lys's own model recorded as the schema of the app `lys`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LysRecorded {
    /// The operation id it was recorded with.
    pub operation: String,
    /// The model's version, which is the schema's first.
    pub version: u64,
    /// The model as the schema of the app `lys`.
    pub schema: Value,
    /// When.
    pub at: u64,
}

/// One line of the log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum Line {
    /// Lys's own model recorded as the app `lys`.
    Lys(LysRecorded),
    /// An app registered.
    Registered(Registered),
    /// A registration approved.
    Approved(Approved),
    /// An approved app's sign-in settings set.
    SignInSet(SignInSet),
    /// A registration declined.
    Declined(Decided),
    /// An app retired.
    Retired(Decided),
    /// A schema change proposed.
    Proposed(Proposed),
    /// A schema version made current.
    Applied(Applied),
    /// A proposed change declined.
    ChangeDeclined(Decided),
    /// A resource placed in its parent.
    Placed(Placed),
    /// A service account made a registrar.
    Registrar(Registrar),
    /// An approved app's connector recorded.
    Connector(Connected),
    /// A virtual client credential issued for an approved app.
    ClientCredentialIssued(ClientCredentialIssued),
    /// A client credential revoked.
    ClientCredentialRevoked(ClientCredentialRevoked),
    /// The broker's confirmation that it ended an app's credentials.
    ClientCredentialsEnded(ClientCredentialsEnded),
    /// An approved app's current credential digest and sealed references.
    CustodyPrepared(CustodyPrepared),
}

impl Line {
    /// The operation id the line was kept under.
    pub fn operation(&self) -> &str {
        match self {
            Self::Lys(line) => &line.operation,
            Self::Registered(line) => &line.operation,
            Self::Approved(line) => &line.operation,
            Self::SignInSet(line) => &line.operation,
            Self::Declined(line) | Self::Retired(line) | Self::ChangeDeclined(line) => {
                &line.operation
            }
            Self::Proposed(line) => &line.operation,
            Self::Applied(line) => &line.operation,
            Self::Placed(line) => &line.operation,
            Self::Registrar(line) => &line.operation,
            Self::Connector(line) => &line.operation,
            Self::ClientCredentialIssued(line) => &line.operation,
            Self::ClientCredentialRevoked(line) => &line.operation,
            Self::ClientCredentialsEnded(line) => &line.operation,
            Self::CustodyPrepared(line) => &line.operation,
        }
    }

    /// The app the line is about; a registrar line is about none.
    pub fn app(&self) -> Option<&str> {
        match self {
            Self::Lys(_) => Some(lys_identity::grants::LYS_APP),
            Self::Registered(line) => Some(&line.app),
            Self::Approved(line) => Some(&line.app),
            Self::SignInSet(line) => Some(&line.app),
            Self::Declined(line) | Self::Retired(line) | Self::ChangeDeclined(line) => {
                Some(&line.app)
            }
            Self::Proposed(line) => Some(&line.app),
            Self::Applied(line) => Some(&line.app),
            Self::Placed(line) => Some(&line.app),
            Self::Connector(line) => Some(&line.app),
            Self::Registrar(_) => None,
            Self::ClientCredentialIssued(line) => Some(&line.app),
            Self::ClientCredentialRevoked(line) => Some(&line.app),
            Self::ClientCredentialsEnded(line) => Some(&line.app),
            Self::CustodyPrepared(line) => Some(&line.app),
        }
    }
}

/// Where an app stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// Registered, and waiting for an administrator.
    Pending,
    /// Approved: its client is enabled and its kinds are judged.
    Approved,
    /// Declined: it never took effect.
    Declined,
    /// Retired: its client is disabled and every check on its kinds is refused.
    Retired,
}

/// One schema version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    /// Its number, from 1.
    pub version: u64,
    /// The schema.
    pub schema: Value,
    /// The operation that made it current.
    pub operation: String,
    /// Who made it current.
    pub by: By,
    /// When.
    pub at: u64,
}

/// One app, as its lines make it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct App {
    /// How it was registered.
    pub registered: Registered,
    /// Its approval, once it has one.
    pub approved: Option<Approved>,
    /// Its sign-in settings as last set: the addresses the provider admits and
    /// whether the app is given the person's name. Set with the approval.
    pub sign_in: Option<SignInSet>,
    /// Its connector, once it has one. Absent from the sealed snapshot while
    /// it has none, so a record of apps without connectors seals to the same
    /// lys-apps-state/v1 bytes it always did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<Connected>,
    /// Its registration declined, once it is.
    pub declined: Option<Decided>,
    /// Its retirement, once it has one.
    pub retired: Option<Decided>,
    /// Every schema version, in order.
    pub versions: Vec<Version>,
    /// The change waiting for an administrator, if one is.
    pub pending: Option<Proposed>,
    /// Every line kept on it after its registration, in order.
    pub history: Vec<Line>,
}

impl App {
    /// Where the app stands. The app `lys`, recorded once at start from the
    /// model file, stands approved without an approval; every other app
    /// stands approved only once an administrator's approval is kept, whoever
    /// its registration says made it.
    pub fn standing(&self) -> Standing {
        let recorded_at_start =
            self.registered.app == lys_identity::grants::LYS_APP && self.registered.by == By::Start;
        if self.retired.is_some() {
            Standing::Retired
        } else if self.approved.is_some() || recorded_at_start {
            Standing::Approved
        } else if self.declined.is_some() {
            Standing::Declined
        } else {
            Standing::Pending
        }
    }

    /// The current schema version, once the app has one.
    pub fn current(&self) -> Option<&Version> {
        self.versions.last()
    }

    /// The schema version numbered `version`.
    pub fn version(&self, version: u64) -> Option<&Version> {
        self.versions.iter().find(|held| held.version == version)
    }
}

/// Why a line cannot be kept on the apps as they stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// An app by that id is already registered.
    Exists,
    /// No app by that id is registered.
    Unknown,
    /// The act is not one the app as it stands takes, and why.
    Standing(Standing),
    /// A change already waits for approval.
    Pending,
    /// No change waits for approval.
    NothingPending,
    /// The version named is not the current one, which is this.
    Moved(u64),
    /// The resource is already placed.
    Placed,
    /// The app is `lys`, which is never retired.
    Lys,
    /// No live client credential of the app has that id.
    Credential,
}

/// The apps as their log folds them, in the order registered.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, from = "Records")]
pub struct Held {
    /// The apps.
    pub apps: Vec<App>,
    /// Every placement, in the order kept.
    pub placements: Vec<Placed>,
    /// Every registrar credential, in the order made; a service account's
    /// latest is the one it holds.
    pub registrars: Vec<Registrar>,
    #[serde(skip)]
    index: Arc<index::Index>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Records {
    apps: Vec<App>,
    placements: Vec<Placed>,
    registrars: Vec<Registrar>,
}

impl From<Records> for Held {
    fn from(records: Records) -> Self {
        let index = Arc::new(index::Index::of(
            &records.apps,
            &records.placements,
            &records.registrars,
        ));
        Self {
            apps: records.apps,
            placements: records.placements,
            registrars: records.registrars,
            index,
        }
    }
}

impl Clone for Held {
    fn clone(&self) -> Self {
        #[cfg(test)]
        operation_tests::copied();
        Self {
            apps: self.apps.clone(),
            placements: self.placements.clone(),
            registrars: self.registrars.clone(),
            index: Arc::clone(&self.index),
        }
    }
}

#[derive(Serialize)]
struct Sealing<'a> {
    format: &'static str,
    held: &'a Held,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

impl Held {
    /// The app named `id`.
    pub fn app(&self, id: &str) -> Option<&App> {
        self.apps.get(*self.index.apps.get(id)?)
    }

    /// The parent a resource of `kind` and `id` is placed in.
    pub fn parent(&self, kind: &str, id: &str) -> Option<&Placed> {
        self.placements.get(self.index.parent(kind, id)?)
    }

    /// The line kept under `operation`, whichever kind it is.
    pub fn operation(&self, operation: &str) -> Option<Line> {
        let location = *self.index.operations.get(operation)?;
        #[cfg(test)]
        operation_tests::visited();
        match location {
            index::Location::Registered(app) => self
                .apps
                .get(app)
                .map(|app| Line::Registered(app.registered.clone())),
            index::Location::History(app, line) => self.apps.get(app)?.history.get(line).cloned(),
            index::Location::Placed(placed) => {
                self.placements.get(placed).cloned().map(Line::Placed)
            }
            index::Location::Registrar(registrar) => {
                self.registrars.get(registrar).cloned().map(Line::Registrar)
            }
        }
    }

    /// Whether `line` may be kept on the apps as they stand, by reason.
    pub fn allows(&self, line: &Line) -> Result<(), Refused> {
        let app = line.app().and_then(|id| self.app(id));
        match line {
            Line::Lys(_) | Line::Registered(_) => match app {
                Some(_) => Err(Refused::Exists),
                None => Ok(()),
            },
            Line::Registrar(_) => Ok(()),
            _ => {
                let app = app.ok_or(Refused::Unknown)?;
                allows_on(app, line, self)
            }
        }
    }

    /// Fold one line. A line the lines before it do not allow is refused by
    /// reason, since every kept line was checked against what came before.
    pub fn hold(&mut self, line: Line) -> Result<(), String> {
        if line.app() == Some("") {
            return Err("this line names no app".to_owned());
        }
        if let Line::Connector(connected) = &line {
            ids_read(connected)?;
        }
        if let Some(kept) = self.operation(line.operation())
            && !beside_its_approval(&kept, &line)
        {
            return Err(format!(
                "operation `{}` already names a line",
                line.operation()
            ));
        }
        self.allows(&line)
            .map_err(|refused| format!("line `{}` is refused: {refused:?}", line.operation()))?;
        let operation = line.operation().to_owned();
        let location = match line {
            Line::Lys(lys) => {
                let position = self.apps.len();
                let app = lys_app(lys);
                Arc::make_mut(&mut self.index).app(&app.registered.app, position);
                self.apps.push(app);
                index::Location::Registered(position)
            }
            Line::Registered(registered) => {
                let position = self.apps.len();
                Arc::make_mut(&mut self.index).app(&registered.app, position);
                self.apps.push(App {
                    registered,
                    approved: None,
                    sign_in: None,
                    connector: None,
                    declined: None,
                    retired: None,
                    versions: Vec::new(),
                    pending: None,
                    history: Vec::new(),
                });
                index::Location::Registered(position)
            }
            Line::Placed(placed) => {
                let position = self.placements.len();
                Arc::make_mut(&mut self.index).placement(&placed, position);
                self.placements.push(placed);
                index::Location::Placed(position)
            }
            Line::Registrar(registrar) => {
                let position = self.registrars.len();
                self.registrars.push(registrar);
                index::Location::Registrar(position)
            }
            other => {
                let id = other
                    .app()
                    .ok_or_else(|| "this line names no app".to_owned())?;
                let position = self
                    .index
                    .apps
                    .get(id)
                    .copied()
                    .ok_or_else(|| format!("app `{id}` was never registered"))?;
                let app = self
                    .apps
                    .get_mut(position)
                    .ok_or_else(|| format!("app `{id}` was never registered"))?;
                apply(app, &other);
                let history = app.history.len();
                app.history.push(other);
                index::Location::History(position, history)
            }
        };
        Arc::make_mut(&mut self.index).operation(&operation, location);
        Ok(())
    }

    /// Fold every leaf of `tail`, in order.
    pub fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let line = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not an apps line: {error}"))?;
            self.hold(line)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealing {
            format: FORMAT,
            held: self,
        })
        .map_err(|error| format!("apps state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed =
            serde_json::from_slice(bytes).map_err(|error| format!("apps state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "apps state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}
