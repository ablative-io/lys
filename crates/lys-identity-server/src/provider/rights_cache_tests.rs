use super::{CAPACITY, Cache, Cached, Key, Read};
use lys_identity::{IdentityId, PersonId, projection::Projection};
use lys_pass::{Holder, Mode, Right, rights::Resource};
use std::collections::BTreeMap;

fn key(app: &str, revision: u64) -> Key {
    Key {
        holder: IdentityId::Person(PersonId::from_bytes([1; 16])),
        app: app.to_owned(),
        revision,
        bind: true,
    }
}

fn value() -> Cached {
    Cached {
        asserted: Holder {
            id: key("notes", 1).holder.to_string(),
            kind: "person".to_owned(),
            responsible: None,
        },
        read: Read {
            rights: vec![Right {
                resource: Resource {
                    kind: "notes.doc".to_owned(),
                    id: "1".to_owned(),
                },
                actions: vec!["read".to_owned()],
                mode: Mode::Outright,
                grant: "grant-1".to_owned(),
            }],
            paths: BTreeMap::from([(
                "grant-1".to_owned(),
                vec!["grant-1".to_owned(), "root".to_owned()],
            )]),
        },
        directory: Projection::default(),
        apps_revision: 1,
        read_at: 10,
        valid_until: Some(20),
    }
}

#[test]
fn cache_hit_returns_identical_rights_paths_and_holder() -> Result<(), Box<dyn std::error::Error>> {
    let mut cache = Cache::default();
    let original = value();
    cache.replace(key("notes", 1), original.clone());
    let hit = cache
        .get(&key("notes", 1), &Projection::default(), 1, 10)
        .ok_or("cache miss")?;
    assert_eq!(hit.read, original.read);
    assert_eq!(hit.asserted, original.asserted);
    Ok(())
}

#[test]
fn moved_revision_invalidates_and_replaces_the_old_entry() {
    let mut cache = Cache::default();
    cache.replace(key("notes", 1), value());
    assert!(
        cache
            .get(&key("notes", 2), &Projection::default(), 1, 10)
            .is_none()
    );
    cache.replace(key("notes", 2), value());
    assert!(
        cache
            .get(&key("notes", 2), &Projection::default(), 1, 10)
            .is_some()
    );
    assert_eq!(cache.entries.len(), 1);
}

#[test]
fn the_bound_evicts_the_oldest_entry_even_when_it_was_read() {
    let mut cache = Cache::default();
    for index in 0..CAPACITY {
        cache.replace(key(&index.to_string(), 1), value());
    }
    assert!(
        cache
            .get(&key("0", 1), &Projection::default(), 1, 10)
            .is_some()
    );
    cache.replace(key("new", 1), value());
    assert_eq!(cache.entries.len(), CAPACITY);
    assert!(
        cache
            .get(&key("0", 1), &Projection::default(), 1, 10)
            .is_none()
    );
    assert!(
        cache
            .get(&key("1", 1), &Projection::default(), 1, 10)
            .is_some()
    );
}

#[test]
fn binding_app_changes_and_time_boundaries_invalidate() {
    let mut cache = Cache::default();
    cache.replace(key("notes", 1), value());
    let mut unbound = key("notes", 1);
    unbound.bind = false;
    assert!(cache.get(&unbound, &Projection::default(), 1, 10).is_none());
    assert!(
        cache
            .get(&key("notes", 1), &Projection::default(), 2, 10)
            .is_none()
    );
    assert!(
        cache
            .get(&key("notes", 1), &Projection::default(), 1, 20)
            .is_none()
    );
    assert!(
        cache
            .get(&key("notes", 1), &Projection::default(), 1, 9)
            .is_none()
    );
}

#[test]
fn exchange_summary_reports_all_acts_and_the_actual_cache_outcome()
-> Result<(), Box<dyn std::error::Error>> {
    use super::super::exchange_timing::{Act, Timing};
    use std::sync::{Arc, Mutex};

    let heard: Arc<Mutex<Vec<String>>> = Arc::default();
    let captured = Arc::clone(&heard);
    let say: crate::routes::Say = Arc::new(move |line| match captured.lock() {
        Ok(mut lines) => lines.push(line.to_owned()),
        Err(error) => panic!("fixture log poisoned: {error}"),
    });
    tracing::subscriber::with_default(crate::warn_lines::WarnLines::new(say), || {
        for hit in [true, false] {
            let mut timing = Timing::new("agent", "cambium");
            for act in [Act::TakeGrant, Act::IsLive, Act::Pass, Act::Issue] {
                timing.measure(act, || ());
            }
            timing.cache(hit);
        }
    });
    let lines = heard.lock().map_err(|error| error.to_string())?;
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("cache=hit"));
    assert!(lines[1].contains("cache=miss"));
    for line in lines.iter() {
        for field in [
            "holder_kind=agent",
            "app=cambium",
            "take_grant_elapsed_ms=Some(",
            "is_live_elapsed_ms=Some(",
            "pass_elapsed_ms=Some(",
            "issue_elapsed_ms=Some(",
        ] {
            assert!(line.contains(field), "missing field {field}");
        }
    }
    Ok(())
}
