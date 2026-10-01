use std::cell::Cell;
use std::error::Error;
use std::sync::Arc;

use super::{Applied, Approved, By, Client, Held, Line, LysRecorded, Placed, Records, Registered};
use crate::apps_binding::Registrar;

thread_local! {
    static VISITS: Cell<usize> = const { Cell::new(0) };
    static COPIES: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn visited() {
    VISITS.set(VISITS.get() + 1);
}

pub(super) fn copied() {
    COPIES.set(COPIES.get() + 1);
}

fn registered(id: &str) -> Registered {
    Registered {
        operation: format!("register-{id}"),
        app: id.to_owned(),
        name: id.to_owned(),
        redirects: Vec::new(),
        schema: serde_json::json!({"kinds": []}),
        service_account: None,
        by: By::Start,
        at: 1,
    }
}

fn applied(id: &str, version: u64) -> Applied {
    Applied {
        operation: format!("apply-{id}-{version}"),
        app: id.to_owned(),
        version,
        schema: serde_json::json!({"kinds": []}),
        proposal: None,
        by: By::Start,
        at: 2,
    }
}

#[test]
fn apps_operation_lookups_touch_only_the_named_record_after_restore_and_tail_writes()
-> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    for n in 0..256 {
        let id = format!("app-{n}");
        held.hold(Line::Registered(registered(&id)))?;
        held.hold(Line::Approved(Approved {
            operation: format!("approve-{id}"),
            app: id.clone(),
            client: Client {
                client_id: id.clone(),
                secret_sha256: "digest".to_owned(),
            },
            binding: None,
            by: By::Start,
            at: 1,
        }))?;
        held.hold(Line::Applied(applied(&id, 2)))?;
    }
    held.hold(Line::Placed(Placed {
        operation: "placement".to_owned(),
        app: "app-255".to_owned(),
        child_kind: "app-255/child".to_owned(),
        child_id: "child".to_owned(),
        parent_kind: "app-255/parent".to_owned(),
        parent_id: "parent".to_owned(),
        by: By::Start,
        at: 2,
    }))?;
    held.hold(Line::Registrar(Registrar {
        operation: "registrar".to_owned(),
        service_account: "account".to_owned(),
        secret_sha256: "digest".to_owned(),
        by: By::Start,
        at: 2,
    }))?;
    let bytes = held.encode()?;
    let mut restored = Held::decode(&bytes)?;
    assert_eq!(restored.encode()?, bytes);
    let copied = restored.clone();
    assert!(Arc::ptr_eq(&restored.index, &copied.index));
    VISITS.set(0);
    restored.hold(Line::Applied(applied("app-0", 3)))?;
    assert_eq!(VISITS.get(), 0, "tail write visited previous history");
    assert!(matches!(
        restored.operation("apply-app-255-2"),
        Some(Line::Applied(_))
    ));
    assert!(matches!(
        restored.operation("register-app-255"),
        Some(Line::Registered(_))
    ));
    assert!(matches!(
        restored.operation("apply-app-0-3"),
        Some(Line::Applied(_))
    ));
    assert!(matches!(
        restored.operation("placement"),
        Some(Line::Placed(_))
    ));
    assert!(matches!(
        restored.operation("registrar"),
        Some(Line::Registrar(_))
    ));
    assert!(restored.operation("missing").is_none());
    assert!(copied.operation("apply-app-0-3").is_none());
    assert_eq!(VISITS.get(), 5, "lookup visited unrelated records");
    assert_eq!(
        restored
            .parent("app-255/child", "child")
            .map(|line| line.parent_id.as_str()),
        Some("parent")
    );
    assert!(restored.parent("app-255/child", "missing").is_none());
    assert_eq!(
        restored
            .app("app-255")
            .map(|app| app.registered.app.as_str()),
        Some("app-255")
    );
    assert!(restored.hold(Line::Applied(applied("app-0", 3))).is_err());
    Ok(())
}

#[test]
fn apps_seal_borrows_the_records_and_keeps_the_old_snapshot_shape() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(Line::Lys(LysRecorded {
        operation: "model".to_owned(),
        version: 1,
        schema: serde_json::json!({"kinds": []}),
        at: 1,
    }))?;
    COPIES.set(0);
    let bytes = held.encode()?;
    assert_eq!(COPIES.get(), 0, "seal copied the complete apps fold");
    let old = serde_json::json!({
        "format": super::FORMAT,
        "held": {
            "apps": held.apps,
            "placements": held.placements,
            "registrars": held.registrars,
        },
    });
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&bytes)?, old);
    let restored = Held::decode(&serde_json::to_vec(&old)?)?;
    assert_eq!(restored, held);
    assert!(matches!(
        restored.operation("model"),
        Some(Line::Registered(_))
    ));
    Ok(())
}

#[test]
fn old_duplicate_operations_keep_the_first_app_and_nested_record_order()
-> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(Line::Registered(registered("first")))?;
    held.hold(Line::Registered(registered("second")))?;
    let mut first = applied("first", 2);
    first.operation = "duplicate".to_owned();
    held.apps[0].history.push(Line::Applied(first.clone()));
    held.apps[1].registered.operation = "duplicate".to_owned();
    let restored = Held::from(Records {
        apps: held.apps,
        placements: Vec::new(),
        registrars: Vec::new(),
    });
    assert_eq!(restored.operation("duplicate"), Some(Line::Applied(first)));
    let bytes = restored.encode()?;
    assert_eq!(
        Held::decode(&bytes)?.operation("duplicate"),
        restored.operation("duplicate")
    );
    Ok(())
}
