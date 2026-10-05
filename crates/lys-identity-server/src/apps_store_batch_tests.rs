//! An approval and its sign-in settings are one durable act: both leaves land
//! with one flush or neither does, so no app is ever approved without an
//! address to send a person back to. A store that refuses the write leaves
//! the app pending, and the same approval asked again lands whole.

use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;
use lys_log_store::{LeafStore, PinnedRoot, StoreError, StoreResult};
use serde_json::json;

use super::{AppStore, ORIGIN};
use crate::apps_bench_scratch::memory::{Kept, MemoryStore};
use crate::apps_state::{Approved, By, Client, Line, Registered, SignInSet, Standing};

const APP: &str = "fixture_notes";
const BACK: &str = "https://app.example.test/signed-in";

/// A leaf store that refuses every append while `refusing` is set, and is
/// the memory store otherwise.
struct Refusing {
    inner: MemoryStore,
    refusing: Arc<AtomicBool>,
}

impl LeafStore for Refusing {
    fn origin(&self) -> &str {
        self.inner.origin()
    }

    fn extent(&self) -> u64 {
        self.inner.extent()
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.inner.leaf(index)
    }

    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        if self.refusing.load(Ordering::SeqCst) {
            return Err(StoreError::Io {
                context: "the test refuses this write".to_owned(),
                source: std::io::Error::other("disk full"),
            });
        }
        self.inner.append(index, leaves, pin)
    }

    fn pinned(&self) -> PinnedRoot {
        self.inner.pinned()
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.inner.pin(pin)
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.inner.snapshot()
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.inner.put_snapshot(bytes)
    }
}

fn open(
    kept: &Arc<Mutex<Kept>>,
    refusing: &Arc<AtomicBool>,
) -> Result<AppStore<Refusing>, Box<dyn Error>> {
    let (kept, refusing) = (Arc::clone(kept), Arc::clone(refusing));
    Ok(AppStore::over(
        Box::new(move || {
            Ok(Refusing {
                inner: MemoryStore::open(Arc::clone(&kept), ORIGIN)?,
                refusing: Arc::clone(&refusing),
            })
        }),
        Arc::new(Ed25519Identity::ephemeral()),
    )?)
}

fn approval() -> (Approved, SignInSet) {
    (
        Approved {
            operation: "approve".to_owned(),
            app: APP.to_owned(),
            client: Client {
                client_id: APP.to_owned(),
                secret_sha256: "00".repeat(32),
            },
            binding: None,
            by: By::Start,
            at: 2,
        },
        SignInSet {
            operation: "approve".to_owned(),
            app: APP.to_owned(),
            redirects: vec![BACK.to_owned()],
            profile: false,
            by: By::Start,
            at: 2,
        },
    )
}

#[test]
fn an_approval_refused_by_the_store_leaves_the_app_pending_and_lands_whole_when_asked_again()
-> Result<(), Box<dyn Error>> {
    let kept = MemoryStore::empty();
    let refusing = Arc::new(AtomicBool::new(false));
    let mut store = open(&kept, &refusing)?;
    store.keep(Line::Registered(Registered {
        operation: "register".to_owned(),
        app: APP.to_owned(),
        name: "The notes fixture".to_owned(),
        redirects: vec![BACK.to_owned()],
        schema: json!({"kinds": {}}),
        service_account: None,
        by: By::Start,
        at: 1,
    }))?;
    assert_eq!(store.len(), 1);

    // The store refuses the write: neither leaf lands, the app stays pending.
    refusing.store(true, Ordering::SeqCst);
    let (approved, settings) = approval();
    let refused = store.keep_approval(approved, settings);
    assert!(
        refused.is_err(),
        "the refused write is not answered as kept"
    );
    assert_eq!(store.len(), 1, "no leaf landed");
    let app = store.app(APP).ok_or("no app")?;
    assert_eq!(app.standing(), Standing::Pending);
    assert!(app.approved.is_none());
    assert!(app.sign_in.is_none());
    assert!(
        store.held().operation("approve").is_none(),
        "the operation names nothing"
    );

    // Asked again once the store writes: both leaves land as one act.
    refusing.store(false, Ordering::SeqCst);
    let (approved, settings) = approval();
    store.keep_approval(approved, settings)?;
    assert_eq!(store.len(), 3, "the approval and its settings, one act");
    let app = store.app(APP).ok_or("no app")?;
    assert_eq!(app.standing(), Standing::Approved);
    assert_eq!(
        app.sign_in
            .as_ref()
            .map(|settings| settings.redirects.clone()),
        Some(vec![BACK.to_owned()])
    );
    assert!(matches!(
        store.held().operation("approve"),
        Some(Line::Approved(_))
    ));

    // The same approval again answers the same and keeps nothing new.
    let (approved, settings) = approval();
    store.keep_approval(approved, settings)?;
    assert_eq!(store.len(), 3);

    // What landed reads back whole from the leaves, in order.
    let reopened = open(&kept, &refusing)?;
    let app = reopened.app(APP).ok_or("no app after reopen")?;
    assert_eq!(app.standing(), Standing::Approved);
    assert!(app.sign_in.is_some());
    Ok(())
}
