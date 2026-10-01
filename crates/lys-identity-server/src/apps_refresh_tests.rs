use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::sync::{Arc, Mutex, RwLock};

use lys_core::Ed25519Identity;
use lys_identity::grants::{Action, AppSchema, Relation};

use crate::apps_state::{By, Decided, Held, Line, LysRecorded};
use crate::apps_store::AppStore;
use crate::grants::GrantSetup;

#[test]
fn schema_publication_holds_no_directory_apps_or_grants_lock() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&temp.path().join("key"))?);
    let mut apps = AppStore::open(&temp.path().join("apps"), key)?;
    apps.keep(Line::Lys(LysRecorded {
        operation: "initial".to_owned(),
        version: 1,
        schema: AppSchema::lys(BTreeMap::from([(
            Relation::new("viewer")?,
            BTreeSet::from([Action::new("read")?]),
        )]))
        .to_json(),
        at: 1,
    }))?;
    let setup = GrantSetup {
        log_dir: temp.path().join("grants"),
        log_origin: "test".to_owned(),
        key_file: temp.path().join("key"),
        model: RwLock::new(apps.model()?),
        spicedb: None,
    };
    let apps = Mutex::new(apps);
    let directory = Mutex::new(());
    let grants = Mutex::new(None);
    super::refresh(&directory, &apps, &grants, &setup, |_, _| {
        assert!(
            directory.try_lock().is_ok(),
            "network publication held the directory lock"
        );
        assert!(
            apps.try_lock().is_ok(),
            "network publication held the apps lock"
        );
        assert!(
            grants.try_lock().is_ok(),
            "network publication held the grants lock"
        );
        Ok(())
    })?;
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
