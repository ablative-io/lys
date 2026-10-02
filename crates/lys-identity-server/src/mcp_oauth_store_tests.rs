//! The connected-apps store keeps apps, approvals and token digests across
//! a reopen, and answers only live tokens of the kind asked for.

use super::{App, Issued, Kind, Limits, Store};
use crate::error::ServerError;

const LIMITS: Limits = Limits {
    most: 3,
    per_minute: 2,
    unapproved_seconds: 1000,
};

fn app(registered_at: u64) -> App {
    App {
        name: "Notes".to_owned(),
        redirect_uris: vec!["https://app.example/callback".to_owned()],
        registered_at,
    }
}

fn issued(kind: Kind, expires_at: u64) -> Issued {
    Issued {
        agent: "agent".to_owned(),
        client_id: "client".to_owned(),
        kind,
        expires_at,
    }
}

#[test]
fn what_is_kept_is_read_back_after_a_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::TempDir::new()?;
    let path = dir.path().join("connected-apps.json");
    let mut store = Store::open(&path)?;
    store.register("client".to_owned(), app(1), LIMITS)?;
    store.connect("client", "person", "agent".to_owned())?;
    store.issue(
        vec![
            ("access".to_owned(), issued(Kind::Access, 100)),
            ("refresh".to_owned(), issued(Kind::Refresh, 200)),
        ],
        None,
        10,
    )?;
    let store = Store::open(&path)?;
    assert_eq!(
        store.app("client").map(|app| app.name.as_str()),
        Some("Notes")
    );
    assert_eq!(store.connection("client", "person"), Some("agent"));
    assert!(store.token("access", Kind::Access, 50).is_some());
    assert!(store.token("access", Kind::Refresh, 50).is_none());
    assert!(store.token("access", Kind::Access, 100).is_none());
    assert!(store.token("refresh", Kind::Refresh, 150).is_some());
    Ok(())
}

#[test]
fn a_spent_refresh_token_ends_when_its_successors_are_issued()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::TempDir::new()?;
    let mut store = Store::open(&dir.path().join("connected-apps.json"))?;
    store.issue(
        vec![("first".to_owned(), issued(Kind::Refresh, 200))],
        None,
        10,
    )?;
    store.issue(
        vec![("second".to_owned(), issued(Kind::Refresh, 300))],
        Some("first"),
        20,
    )?;
    assert!(store.token("first", Kind::Refresh, 30).is_none());
    assert!(store.token("second", Kind::Refresh, 30).is_some());
    Ok(())
}

#[test]
fn a_store_file_that_does_not_read_is_refused_by_name() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::TempDir::new()?;
    let path = dir.path().join("connected-apps.json");
    std::fs::write(&path, b"not json")?;
    let Err(refused) = Store::open(&path) else {
        return Err("a torn store opened".into());
    };
    assert!(refused.to_string().contains("connected-apps store"));
    Ok(())
}

#[test]
fn registrations_are_throttled_held_to_a_most_and_let_go_unapproved()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::TempDir::new()?;
    let mut store = Store::open(&dir.path().join("connected-apps.json"))?;
    store.register("first".to_owned(), app(0), LIMITS)?;
    store.register("second".to_owned(), app(10), LIMITS)?;
    assert!(matches!(
        store.register("third".to_owned(), app(20), LIMITS),
        Err(ServerError::RegistrationThrottled)
    ));
    assert!(store.app("third").is_none(), "a throttled app is not kept");
    store.connect("first", "person", "agent".to_owned())?;
    store.register("third".to_owned(), app(70), LIMITS)?;
    assert!(matches!(
        store.register("fourth".to_owned(), app(200), LIMITS),
        Err(ServerError::RequestMalformed { .. })
    ));
    store.register("fourth".to_owned(), app(1010), LIMITS)?;
    assert!(store.app("first").is_some(), "an approved app is kept");
    assert!(
        store.app("second").is_none(),
        "an app no one approved within its window is let go"
    );
    assert!(store.app("third").is_some() && store.app("fourth").is_some());
    Ok(())
}
