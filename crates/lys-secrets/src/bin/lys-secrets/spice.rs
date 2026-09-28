//! Permission from the Lys directory's grants, as `SpiceDB` holds them: an
//! identity may use a secret when the directory gives it the chosen action
//! on the resource `secret/<name>`. Every check asks the engine afresh.

use std::path::Path;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use lys_identity::grants::{Action, Resource};
use lys_identity::{AgentId, IdentityId, PersonId};
use lys_identity_server::Config;
use lys_identity_server::spicedb::SpiceDb;
use lys_secrets::{Denied, PermissionCheck, Permitted, SecretsError};

/// The resource kind a secret is granted under in the directory.
pub const SECRET_KIND: &str = "secret";

/// Permission read from the directory's permission engine.
pub struct SpiceGrants {
    engine: SpiceDb,
    action: Action,
    read_action: Action,
    lend_action: Action,
    member_action: Action,
}

impl std::fmt::Debug for SpiceGrants {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpiceGrants")
            .field("engine", &self.engine)
            .finish_non_exhaustive()
    }
}

fn refused(reason: String) -> SecretsError {
    SecretsError::Encoding {
        context: "directory permission",
        reason,
    }
}

impl SpiceGrants {
    /// Reaches the engine the directory's configuration names, asking for
    /// `action` on each use and `read_action` on each sealed-record read.
    ///
    /// # Errors
    ///
    /// When the configuration does not read, names no engine, or the engine
    /// cannot be reached.
    pub fn from_directory(
        config: &Path,
        action: &str,
        read_action: &str,
        [lend_action, member_action]: [&str; 2],
    ) -> Result<Self, SecretsError> {
        let config = Config::load(config)
            .map_err(|error| refused(format!("the directory configuration: {error}")))?;
        let settings = config.spicedb.clone().ok_or_else(|| {
            refused("the directory configuration names no permission engine".to_owned())
        })?;
        let model = config
            .grant_model()
            .map_err(|error| refused(error.to_string()))?;
        let engine =
            SpiceDb::open(&settings, &model).map_err(|error| refused(error.to_string()))?;
        let action = Action::new(action).map_err(|error| refused(error.to_string()))?;
        let read_action = Action::new(read_action).map_err(|error| refused(error.to_string()))?;
        let lend_action = Action::new(lend_action).map_err(|error| refused(error.to_string()))?;
        let member_action =
            Action::new(member_action).map_err(|error| refused(error.to_string()))?;
        Ok(Self {
            engine,
            action,
            read_action,
            lend_action,
            member_action,
        })
    }
}

fn identity(text: &str) -> Option<IdentityId> {
    PersonId::from_str(text)
        .map(IdentityId::Person)
        .or_else(|_person| AgentId::from_str(text).map(IdentityId::Agent))
        .ok()
}

impl PermissionCheck for SpiceGrants {
    fn may_use(&self, holder: &str, secret: &str) -> Result<Permitted, Denied> {
        self.check(&self.action, holder, secret)
    }

    fn may_read(&self, identity: &str, record: &str) -> Result<Permitted, Denied> {
        self.check(&self.read_action, identity, record)
    }

    fn may_lend(&self, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        self.check(&self.lend_action, identity, secret)
    }

    fn member_of(&self, identity: &str, target: &str) -> Result<Permitted, Denied> {
        let Some((kind, name)) = target.split_once('/') else {
            return Err(Denied {
                reason: format!("{target} is not a scope"),
                no_person_root: false,
            });
        };
        self.check_on(&self.member_action, identity, kind, name)
    }
}

impl SpiceGrants {
    fn check(&self, action: &Action, holder: &str, secret: &str) -> Result<Permitted, Denied> {
        self.check_on(action, holder, SECRET_KIND, secret)
    }

    fn check_on(
        &self,
        action: &Action,
        holder: &str,
        kind: &str,
        secret: &str,
    ) -> Result<Permitted, Denied> {
        let denied = |reason: String| Denied {
            reason,
            no_person_root: false,
        };
        let subject = identity(holder)
            .ok_or_else(|| denied(format!("{holder} is not a directory identity id")))?;
        let resource = Resource::new(kind, secret).map_err(|error| denied(error.to_string()))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        match self.engine.check(&resource, action, subject, now) {
            Ok(true) => Ok(Permitted {
                person: "the directory's grant chain".to_owned(),
            }),
            Ok(false) => Err(denied(format!(
                "the directory gives no {} on {kind}/{secret}",
                action.as_str()
            ))),
            Err(error) => Err(denied(format!(
                "the permission engine did not answer: {error}"
            ))),
        }
    }
}

/// The permission source the broker runs with.
#[derive(Debug)]
pub enum Grants {
    /// The broker's own grants file.
    File(crate::files::FileGrants),
    /// The Lys directory's grants, through its permission engine.
    Directory(SpiceGrants),
}

impl Grants {
    /// Every grant the broker can list, as (identity, secret, granted by).
    /// Directory grants are listed by the directory itself.
    pub fn list(&self) -> Result<Vec<crate::files::GrantView>, SecretsError> {
        match self {
            Self::File(file) => file.list(),
            Self::Directory(_) => Ok(Vec::new()),
        }
    }
}

impl PermissionCheck for Grants {
    fn may_use(&self, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        match self {
            Self::File(file) => file.may_use(identity, secret),
            Self::Directory(engine) => engine.may_use(identity, secret),
        }
    }

    fn may_read(&self, identity: &str, record: &str) -> Result<Permitted, Denied> {
        match self {
            Self::File(file) => file.may_read(identity, record),
            Self::Directory(engine) => engine.may_read(identity, record),
        }
    }

    fn may_lend(&self, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        match self {
            Self::File(file) => file.may_lend(identity, secret),
            Self::Directory(engine) => engine.may_lend(identity, secret),
        }
    }

    fn member_of(&self, identity: &str, target: &str) -> Result<Permitted, Denied> {
        match self {
            Self::File(file) => file.member_of(identity, target),
            Self::Directory(engine) => engine.member_of(identity, target),
        }
    }
}
