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
//! never retired. Every operation id names one line only.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::apps_binding::{Binding, Registrar};
use crate::read_views::Login;

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
    /// This is a new act kind; existing person records keep their exact shape.
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
    /// The SHA-256 of the client secret, in hex. The secret itself is
    /// shown once to the approving administrator and kept nowhere.
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
    /// Who placed it.
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
}

impl Line {
    /// The operation id the line was kept under.
    pub fn operation(&self) -> &str {
        match self {
            Self::Lys(line) => &line.operation,
            Self::Registered(line) => &line.operation,
            Self::Approved(line) => &line.operation,
            Self::Declined(line) | Self::Retired(line) | Self::ChangeDeclined(line) => {
                &line.operation
            }
            Self::Proposed(line) => &line.operation,
            Self::Applied(line) => &line.operation,
            Self::Placed(line) => &line.operation,
            Self::Registrar(line) => &line.operation,
        }
    }

    /// The app the line is about; a registrar line is about none.
    pub fn app(&self) -> Option<&str> {
        match self {
            Self::Lys(_) => Some(lys_identity::grants::LYS_APP),
            Self::Registered(line) => Some(&line.app),
            Self::Approved(line) => Some(&line.app),
            Self::Declined(line) | Self::Retired(line) | Self::ChangeDeclined(line) => {
                Some(&line.app)
            }
            Self::Proposed(line) => Some(&line.app),
            Self::Applied(line) => Some(&line.app),
            Self::Placed(line) => Some(&line.app),
            Self::Registrar(_) => None,
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
}

/// The apps as their log folds them, in the order registered.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Held {
    /// The apps.
    pub apps: Vec<App>,
    /// Every placement, in the order kept.
    pub placements: Vec<Placed>,
    /// Every registrar credential, in the order made; a service account's
    /// latest is the one it holds.
    pub registrars: Vec<Registrar>,
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
        self.apps.iter().find(|app| app.registered.app == id)
    }

    /// The parent a resource of `kind` and `id` is placed in.
    pub fn parent(&self, kind: &str, id: &str) -> Option<&Placed> {
        self.placements
            .iter()
            .find(|placed| placed.child_kind == kind && placed.child_id == id)
    }

    /// The line kept under `operation`, whichever kind it is.
    pub fn operation(&self, operation: &str) -> Option<Line> {
        for app in &self.apps {
            if app.registered.operation == operation {
                return Some(Line::Registered(app.registered.clone()));
            }
            if let Some(line) = app
                .history
                .iter()
                .find(|line| line.operation() == operation)
            {
                return Some(line.clone());
            }
        }
        let placed = self
            .placements
            .iter()
            .find(|line| line.operation == operation);
        if let Some(placed) = placed {
            return Some(Line::Placed(placed.clone()));
        }
        self.registrars
            .iter()
            .find(|line| line.operation == operation)
            .map(|registrar| Line::Registrar(registrar.clone()))
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
        if self.operation(line.operation()).is_some() {
            return Err(format!(
                "operation `{}` already names a line",
                line.operation()
            ));
        }
        self.allows(&line)
            .map_err(|refused| format!("line `{}` is refused: {refused:?}", line.operation()))?;
        match line {
            Line::Lys(lys) => self.apps.push(lys_app(lys)),
            Line::Registered(registered) => self.apps.push(App {
                registered,
                approved: None,
                declined: None,
                retired: None,
                versions: Vec::new(),
                pending: None,
                history: Vec::new(),
            }),
            Line::Placed(placed) => self.placements.push(placed),
            Line::Registrar(registrar) => self.registrars.push(registrar),
            other => {
                let id = other.app().unwrap_or_default().to_owned();
                let app = self
                    .apps
                    .iter_mut()
                    .find(|app| app.registered.app == id)
                    .ok_or_else(|| format!("app `{id}` was never registered"))?;
                apply(app, &other);
                app.history.push(other);
            }
        }
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
        serde_json::to_vec(&Sealed {
            format: FORMAT.to_owned(),
            held: self.clone(),
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

/// The app `lys` as its one line records it: approved at the model's version.
fn lys_app(lys: LysRecorded) -> App {
    let version = Version {
        version: lys.version,
        schema: lys.schema.clone(),
        operation: lys.operation.clone(),
        by: By::Start,
        at: lys.at,
    };
    App {
        registered: Registered {
            operation: lys.operation,
            app: lys_identity::grants::LYS_APP.to_owned(),
            name: "Lys".to_owned(),
            redirects: Vec::new(),
            schema: lys.schema,
            service_account: None,
            by: By::Start,
            at: lys.at,
        },
        approved: None,
        declined: None,
        retired: None,
        versions: vec![version],
        pending: None,
        history: Vec::new(),
    }
}

/// Whether `line` may be kept on `app` as it stands.
fn allows_on(app: &App, line: &Line, held: &Held) -> Result<(), Refused> {
    let standing = app.standing();
    let current = app.current().map_or(0, |version| version.version);
    match line {
        Line::Approved(_) | Line::Declined(_) if standing != Standing::Pending => {
            Err(Refused::Standing(standing))
        }
        Line::Retired(_) if app.registered.app == lys_identity::grants::LYS_APP => {
            Err(Refused::Lys)
        }
        Line::Retired(_) | Line::Proposed(_) | Line::Applied(_) | Line::Placed(_)
            if standing != Standing::Approved =>
        {
            Err(Refused::Standing(standing))
        }
        Line::Proposed(_) if app.pending.is_some() => Err(Refused::Pending),
        Line::Proposed(proposed) if proposed.replaces != current => Err(Refused::Moved(current)),
        Line::Applied(applied) if applied.version != current + 1 => Err(Refused::Moved(current)),
        Line::Applied(Applied {
            proposal: Some(proposal),
            ..
        }) if app.pending.as_ref().map(|pending| &pending.operation) != Some(proposal) => {
            Err(Refused::NothingPending)
        }
        Line::Applied(Applied { proposal: None, .. }) if app.pending.is_some() => {
            Err(Refused::Pending)
        }
        Line::ChangeDeclined(_) if app.pending.is_none() => Err(Refused::NothingPending),
        Line::Placed(placed) if held.parent(&placed.child_kind, &placed.child_id).is_some() => {
            Err(Refused::Placed)
        }
        _ => Ok(()),
    }
}

/// Apply `line`, already allowed, to `app`.
fn apply(app: &mut App, line: &Line) {
    match line {
        Line::Approved(approved) => {
            app.versions.push(Version {
                version: 1,
                schema: app.registered.schema.clone(),
                operation: approved.operation.clone(),
                by: approved.by.clone(),
                at: approved.at,
            });
            app.approved = Some(approved.clone());
        }
        Line::Declined(declined) => app.declined = Some(declined.clone()),
        Line::Retired(retired) => app.retired = Some(retired.clone()),
        Line::Proposed(proposed) => app.pending = Some(proposed.clone()),
        Line::ChangeDeclined(_) => app.pending = None,
        Line::Applied(applied) => {
            app.pending = None;
            app.versions.push(Version {
                version: applied.version,
                schema: applied.schema.clone(),
                operation: applied.operation.clone(),
                by: applied.by.clone(),
                at: applied.at,
            });
        }
        Line::Lys(_) | Line::Registered(_) | Line::Placed(_) | Line::Registrar(_) => {}
    }
}
