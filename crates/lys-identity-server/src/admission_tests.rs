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
