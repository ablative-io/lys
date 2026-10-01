//! Controlled issuer writes expose stale updates without timing assumptions.

use std::cell::RefCell;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::sync::{Mutex, Notify, mpsc};

use crate::sign_in_providers::{SignInProviders, SignInProvidersSettings};

thread_local! {
    static STARTED: RefCell<Option<mpsc::UnboundedSender<bool>>> = const { RefCell::new(None) };
}

pub(crate) fn started(queued: bool) {
    STARTED.with(|held| {
        if let Some(sender) = held.borrow().as_ref() {
            assert!(sender.send(queued).is_ok(), "change observer disappeared");
        }
    });
}

struct Issuer {
    user: Mutex<Value>,
    email_put: Notify,
    release_email: Notify,
    paused_email: AtomicBool,
}

struct Fixture {
    api: Arc<SignInProviders>,
    issuer: Arc<Issuer>,
    serving: tokio::task::JoinHandle<Result<(), std::io::Error>>,
    keys: tempfile::TempDir,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.serving.abort();
        STARTED.with(|held| *held.borrow_mut() = None);
    }
}

impl Fixture {
    async fn open() -> Result<Self, Box<dyn Error>> {
        let keys = tempfile::tempdir()?;
        let key = keys.path().join("api.key");
        std::fs::write(&key, "test$synthetic-key")?;
        let issuer = Arc::new(Issuer {
            user: Mutex::new(json!({
                "id": "account", "email": "old@example.test", "language": "en", "roles": [],
                "enabled": true, "email_verified": true, "user_values": {}
            })),
            email_put: Notify::new(),
            release_email: Notify::new(),
            paused_email: AtomicBool::new(false),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = format!("http://{}", listener.local_addr()?);
        let router = Router::new()
            .route("/users/{id}", get(read).put(write))
            .route("/users/email/{email}", get(by_email))
            .with_state(Arc::clone(&issuer));
        let serving = tokio::spawn(async move { axum::serve(listener, router).await });
        let api = Arc::new(SignInProviders::open(
            &SignInProvidersSettings {
                api: address,
                api_key_file: key,
            },
            None,
        )?);
        Ok(Self {
            api,
            issuer,
            serving,
            keys,
        })
    }
}

async fn read(State(issuer): State<Arc<Issuer>>) -> Json<Value> {
    Json(issuer.user.lock().await.clone())
}

async fn write(State(issuer): State<Arc<Issuer>>, Json(update): Json<Value>) -> Json<Value> {
    if update["email"] == "new@example.test" && !issuer.paused_email.swap(true, Ordering::SeqCst) {
        issuer.email_put.notify_one();
        issuer.release_email.notified().await;
    }
    *issuer.user.lock().await = update;
    Json(Value::Null)
}

async fn by_email(Path(email): Path<String>) -> Json<Value> {
    Json(json!({ "email": email }))
}

#[tokio::test]
async fn disabling_and_email_changes_cannot_replace_each_others_fields()
-> Result<(), Box<dyn Error>> {
    let fixture = Fixture::open().await?;
    assert!(fixture.keys.path().exists());
    let api = Arc::clone(&fixture.api);
    let email = tokio::spawn(async move {
        super::change(&api, "account", |update| {
            update["email"] = json!("new@example.test");
        })
        .await
    });
    fixture.issuer.email_put.notified().await;
    let (observer, mut started) = mpsc::unbounded_channel();
    STARTED.with(|held| *held.borrow_mut() = Some(observer));
    let api = Arc::clone(&fixture.api);
    let disable = tokio::spawn(async move {
        super::change(&api, "account", |update| update["enabled"] = json!(false)).await
    });
    let queued = started
        .recv()
        .await
        .ok_or("change did not enter or queue")?;
    if queued {
        fixture.issuer.release_email.notify_one();
        email.await??;
        disable.await??;
    } else {
        disable.await??;
        fixture.issuer.release_email.notify_one();
        email.await??;
    }
    let user = fixture.issuer.user.lock().await;
    assert_eq!(
        user["enabled"], false,
        "an email write re-enabled the disabled account"
    );
    assert_eq!(
        user["email"], "new@example.test",
        "disable lost the email change"
    );
    Ok(())
}

#[tokio::test]
async fn email_lookup_keeps_the_complete_address_in_one_path_segment() -> Result<(), Box<dyn Error>>
{
    let fixture = Fixture::open().await?;
    let email = "member@example.test/other?query#fragment%";
    let user = super::find_by_email(&fixture.api, email)
        .await?
        .ok_or("email was split into path or query")?;
    assert_eq!(user["email"], email);
    Ok(())
}
