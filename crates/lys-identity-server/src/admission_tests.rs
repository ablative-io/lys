use std::error::Error;
use std::sync::Arc;

use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};

use super::Admission;

#[test]
fn an_unavailable_administrator_store_is_refused_by_name() -> Result<(), Box<dyn Error>> {
    let login = LoginBinding::new("https://issuer.example.test", "administrator")?;
    let admission = Arc::new(Admission::new(Some(login.clone()), login.clone()));
    let failing = Arc::clone(&admission);
    let failure = std::thread::spawn(move || -> Result<(), String> {
        let guard = failing
            .administrator
            .write()
            .map_err(|error| error.to_string())?;
        assert!(guard.is_some());
        panic!("administrator store failed while held");
    })
    .join();
    assert!(failure.is_err());
    let actor = Actor::new(login, Provenance::new(AuthMethod::Oidc, 1));
    let refused = admission
        .is_administrator(&lys_identity::projection::Projection::default(), &actor)
        .err()
        .ok_or("unavailable store admitted administrator")?;
    assert_eq!(refused.name(), "DirectoryUnavailable");
    assert!(refused.to_string().contains("administrator"));
    Ok(())
}

#[test]
fn poisoned_administrator_store_neither_reads_nor_changes_the_administrator()
-> Result<(), Box<dyn Error>> {
    let login = LoginBinding::new("https://issuer.example.test", "administrator")?;
    let admission = Admission::new(Some(login.clone()), login.clone());
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let held = admission
            .administrator
            .write()
            .expect("fixture lock poisoned before injection");
        assert!(held.is_some());
        panic!("administrator state failure");
    }));
    assert!(poisoned.is_err());
    for refusal in [
        admission.administrator_login().map(|_| ()),
        admission.administrator_available(),
        admission.set_administrator(login),
    ] {
        let refused = refusal
            .err()
            .ok_or("poisoned administrator state was recovered")?;
        assert_eq!(refused.name(), "DirectoryUnavailable");
        assert!(refused.to_string().contains("administrator store"));
    }
    Ok(())
}
