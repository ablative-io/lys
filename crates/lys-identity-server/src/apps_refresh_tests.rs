use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::sync::{Arc, Mutex, RwLock};

use lys_core::Ed25519Identity;
use lys_identity::grants::{Action, AppSchema, Model, Relation};

use crate::apps_state::{Applied, By, Decided, Held, Line, LysRecorded};
use crate::apps_store::AppStore;
use crate::error::ServerError;
use crate::grants::{GrantSetup, GrantState};
use crate::spicedb::SpiceDb;

type TestResult = Result<(), Box<dyn Error>>;

struct Table {
    storage: tempfile::TempDir,
    directory: Mutex<()>,
    apps: Mutex<AppStore>,
    grants: Mutex<Option<GrantState>>,
    setup: GrantSetup,
}

fn schema() -> Result<serde_json::Value, ServerError> {
    Ok(AppSchema::lys(BTreeMap::from([(
        Relation::new("viewer")?,
        BTreeSet::from([Action::new("read")?]),
    )]))
    .to_json())
}

fn next() -> Result<Line, ServerError> {
    Ok(Line::Applied(Applied {
        operation: "next".to_owned(),
        app: "lys".to_owned(),
        version: 2,
        schema: schema()?,
        proposal: None,
        by: By::Start,
        at: 2,
    }))
}

impl Table {
    fn new() -> Result<Self, Box<dyn Error>> {
        let storage = tempfile::tempdir()?;
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &storage.path().join("key"),
        )?);
        let mut apps = AppStore::open(&storage.path().join("apps"), key)?;
        apps.keep(Line::Lys(LysRecorded {
            operation: "initial".to_owned(),
            version: 1,
            schema: schema()?,
            at: 1,
        }))?;
        let setup = GrantSetup {
            log_dir: storage.path().join("grants"),
            log_origin: "test".to_owned(),
            key_file: storage.path().join("key"),
            model: RwLock::new(apps.model()?),
            spicedb: None,
            model_revision: std::sync::atomic::AtomicU64::new(apps.model_revision()),
            refresh: Mutex::new(()),
        };
        Ok(Self {
            storage,
            directory: Mutex::new(()),
            apps: Mutex::new(apps),
            grants: Mutex::new(None),
            setup,
        })
    }

    fn refresh(
        &self,
        publish: impl FnOnce(Option<&SpiceDb>, &Model) -> Result<(), ServerError>,
    ) -> Result<(), ServerError> {
        super::refresh(
            &self.directory,
            &self.apps,
            &self.grants,
            &self.setup,
            publish,
        )
    }

    fn advance(&self) -> Result<(), ServerError> {
        self.apps
            .lock()
            .map_err(|error| super::unavailable(error.to_string()))?
            .keep(next()?)?;
        Ok(())
    }
}

#[test]
fn schema_publication_holds_no_directory_apps_or_grants_lock() -> TestResult {
    let table = Table::new()?;
    table.refresh(|_, _| {
        assert!(
            table.directory.try_lock().is_ok(),
            "network publication held the directory lock"
        );
        assert!(
            table.apps.try_lock().is_ok(),
            "network publication held the apps lock"
        );
        assert!(
            table.grants.try_lock().is_ok(),
            "network publication held the grants lock"
        );
        Ok(())
    })?;
    Ok(())
}

#[test]
fn publication_refuses_a_model_that_moved_and_retry_publishes_the_current_one() -> TestResult {
    let table = Table::new()?;
    let error = table
        .refresh(|_, _| table.advance())
        .err()
        .ok_or("stale publication admitted")?;
    assert_eq!(error.name(), "apps_unavailable");
    assert_eq!(table.setup.model().version(), 1);
    assert!(table.setup.require_model(2).is_err());
    table.refresh(|_, _| Ok(()))?;
    assert_eq!(table.setup.model().version(), 2);
    table.setup.require_model(2)?;
    Ok(())
}

#[test]
fn a_failed_publication_keeps_decisions_refused_until_a_retry_succeeds() -> TestResult {
    let table = Table::new()?;
    table.advance()?;
    let error = table
        .refresh(|_, _| Err(super::unavailable("engine unavailable".to_owned())))
        .err()
        .ok_or("failed publication admitted")?;
    assert_eq!(error.name(), "apps_unavailable");
    assert!(table.setup.require_model(2).is_err());
    table.refresh(|_, _| Ok(()))?;
    table.setup.require_model(2)?;
    Ok(())
}

#[test]
fn concurrent_publication_is_refused_without_waiting() -> TestResult {
    let table = Table::new()?;
    table.refresh(|_, _| {
        assert!(table.refresh(|_, _| Ok(())).is_err());
        Ok(())
    })?;
    Ok(())
}

#[test]
fn an_old_apps_log_rebuilds_the_same_model_revision() -> TestResult {
    let table = Table::new()?;
    table.advance()?;
    let key = Arc::new(Ed25519Identity::load(&table.storage.path().join("key"))?);
    let reopened = AppStore::open(&table.storage.path().join("apps"), key)?;
    assert_eq!(reopened.model_revision(), 2);
    assert_eq!(reopened.model()?.version(), 2);
    Ok(())
}

#[test]
fn a_line_without_an_app_id_is_refused_by_name() {
    let mut held = Held::default();
    let result = held.hold(Line::Retired(Decided {
        operation: "retire".to_owned(),
        app: String::new(),
        reason: String::new(),
        by: By::Start,
        at: 1,
    }));
    assert_eq!(result, Err("this line names no app".to_owned()));
}
