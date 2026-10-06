#![cfg(test)]
//! An app's virtual client credentials through the broker's routes
//! (DIRECTORY-081): issued once, confirmed only while live in the apps'
//! record and only for their own app, ended for good, and never written
//! into an answer but the issue's or into any audit line.
use crate::{
    files::{FileGrants, Layout, now_ms},
    serve::Shared,
    spice::Grants,
};
use axum::{
    body::Body,
    extract::{Request, State},
};
use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, OnBehalf, ServiceKey, ServiceWindow, new_operation_id, request_digest, to_hex,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

type Outcome = Result<(), Box<dyn std::error::Error>>;

struct Fixture {
    dir: tempfile::TempDir,
    key: Ed25519Identity,
    shared: Arc<Shared>,
}

fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let layout = Layout::new(&dir.path().join("broker"), &dir.path().join("keys"));
    layout.prepare()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("service.key"))?;
    layout.trust_service(ServiceKey {
        name: "identity".to_owned(),
        public_key: to_hex(&key.public_key_bytes()),
    })?;
    let grants = Grants::File(FileGrants::new(layout.grants()));
    let broker = Broker::create(&layout.paths(), grants.clone(), Box::new(now_ms))?;
    Ok(Fixture {
        dir,
        key,
        shared: Arc::new(Shared {
            broker: Mutex::new(broker),
            layout,
            client: reqwest::Client::new(),
            window: Mutex::new(ServiceWindow::new()),
            permissions: Arc::new(grants),
        }),
    })
}

fn signed(
    key: &Ed25519Identity,
    path: &str,
    body: &Value,
) -> Result<Request, Box<dyn std::error::Error>> {
    let body = serde_json::to_vec(body)?;
    let proof = OnBehalf::sign(
        "identity",
        "person-admin",
        &new_operation_id()?,
        now_ms(),
        request_digest("POST", path, &body)?,
        key,
    )?;
    let [service, person, op, at, signature] = proof.to_wire();
    Ok(Request::builder()
        .method("POST")
        .uri(path)
        .header("lys-service", service)
        .header("lys-on-behalf-of", person)
        .header("lys-operation", op)
        .header("lys-signed-at", at)
        .header("lys-service-signature", signature)
        .body(Body::from(body))?)
}

impl Fixture {
    /// Prepares `app`'s custody, answering its client secret's digest.
    async fn prepared(&self, app: &str) -> Result<String, Box<dyn std::error::Error>> {
        let body = json!({"app": app, "upstream": "http://127.0.0.1:8491/api"});
        let request = signed(&self.key, "/_lys/apps/prepare", &body)?;
        let answer = crate::save_app::prepare(State(Arc::clone(&self.shared)), request)
            .await
            .map_err(|e| format!("{} {}", e.0, e.1))?
            .0;
        Ok(answer["client_secret_sha256"]
            .as_str()
            .ok_or("no digest")?
            .to_owned())
    }

    async fn issue(&self, app: &str) -> Result<Value, String> {
        let request = signed(&self.key, "/_lys/apps/client/issue", &json!({"app": app}))
            .map_err(|e| e.to_string())?;
        crate::app_client::issue(State(Arc::clone(&self.shared)), request)
            .await
            .map(|answer| answer.0)
            .map_err(|e| format!("{} {}", e.0, e.1))
    }

    async fn authenticate(
        &self,
        app: &str,
        presented: &str,
        live: &[&str],
        digest: &str,
    ) -> Result<Value, String> {
        let body =
            json!({"app": app, "presented": presented, "live": live, "secret_sha256": digest});
        let request = signed(&self.key, "/_lys/apps/client", &body).map_err(|e| e.to_string())?;
        crate::app_client::authenticate(State(Arc::clone(&self.shared)), request)
            .await
            .map(|answer| answer.0)
            .map_err(|e| format!("{} {}", e.0, e.1))
    }

    async fn end(&self, app: &str, ids: &[&str]) -> Result<Value, String> {
        let body = json!({"app": app, "credential_ids": ids, "why": "revoked"});
        let request =
            signed(&self.key, "/_lys/apps/client/end", &body).map_err(|e| e.to_string())?;
        crate::app_client::end(State(Arc::clone(&self.shared)), request)
            .await
            .map(|answer| answer.0)
            .map_err(|e| format!("{} {}", e.0, e.1))
    }

    fn close(self) -> Outcome {
        self.dir.close()?;
        Ok(())
    }

    /// Every audit line, every field of each, as text.
    fn audit(&self) -> Result<String, Box<dyn std::error::Error>> {
        let broker = self.shared.broker.lock().map_err(|e| e.to_string())?;
        let lines = broker.audit().audit_every_line()?;
        Ok(format!(
            "{:?}",
            lines
                .iter()
                .map(|recorded| &recorded.line)
                .collect::<Vec<_>>()
        ))
    }
}

/// `value` with its last character changed.
fn changed(value: &str) -> String {
    let mut changed = value.to_owned();
    let last = if changed.pop() == Some('0') { '1' } else { '0' };
    changed.push(last);
    changed
}

#[tokio::test]
async fn an_issued_credential_is_confirmed_while_live_and_never_after_it_is_ended() -> Outcome {
    let fixture = fixture()?;
    let digest = fixture.prepared("notes_app").await?;
    let issued = fixture.issue("notes_app").await?;
    let value = issued["value"].as_str().ok_or("no value")?.to_owned();
    let id = issued["credential_id"].as_str().ok_or("no id")?.to_owned();
    assert!(value.starts_with("lys-client.notes_app."), "{value}");
    assert_eq!(issued["owner"], "person-admin");
    let confirmed = fixture
        .authenticate("notes_app", &value, &[&id], &digest)
        .await?;
    assert_eq!(confirmed["credential_id"], id.as_str());
    let second = fixture.issue("notes_app").await?;
    let other = second["value"].as_str().ok_or("no value")?.to_owned();
    let other_id = second["credential_id"].as_str().ok_or("no id")?.to_owned();
    assert_ne!(other_id, id);
    let ended = fixture.end("notes_app", &[&id]).await?;
    assert_eq!(ended["ended"], json!([id]));
    let refused = fixture
        .authenticate("notes_app", &value, &[&id, &other_id], &digest)
        .await
        .err()
        .ok_or("an ended credential was confirmed")?;
    assert!(refused.contains("AppClientCredentialRefused"), "{refused}");
    // The second is untouched by the first's ending.
    fixture
        .authenticate("notes_app", &other, &[&other_id], &digest)
        .await?;
    // A repeated ending answers none ended and refuses nothing.
    assert_eq!(fixture.end("notes_app", &[&id]).await?["ended"], json!([]));
    let audit = fixture.audit()?;
    assert!(!audit.contains(&value) && !audit.contains(&other));
    assert!(audit.contains(&format!("client authenticated for notes_app with {id}")));
    assert!(audit.contains("refused AppClientCredentialRefused for notes_app"));
    assert!(audit.contains(&format!("issued app client credential {id} for notes_app")));
    fixture.close()
}

#[tokio::test]
async fn a_credential_the_apps_record_no_longer_holds_live_is_refused_and_ended() -> Outcome {
    let fixture = fixture()?;
    let digest = fixture.prepared("notes_app").await?;
    let issued = fixture.issue("notes_app").await?;
    let value = issued["value"].as_str().ok_or("no value")?.to_owned();
    let id = issued["credential_id"].as_str().ok_or("no id")?.to_owned();
    let refused = fixture
        .authenticate("notes_app", &value, &[], &digest)
        .await
        .err()
        .ok_or("a credential not live was confirmed")?;
    assert!(refused.contains("AppClientCredentialRefused"), "{refused}");
    // Ended for good: named live again, it is still refused.
    assert!(
        fixture
            .authenticate("notes_app", &value, &[&id], &digest)
            .await
            .is_err()
    );
    assert!(
        fixture
            .audit()?
            .contains("no longer live in the apps' record")
    );
    fixture.close()
}

#[tokio::test]
async fn another_apps_credential_a_changed_one_and_a_wrong_custody_are_refused() -> Outcome {
    let fixture = fixture()?;
    let digest = fixture.prepared("notes_app").await?;
    let other_digest = fixture.prepared("diary_app").await?;
    let notes = fixture.issue("notes_app").await?;
    let diary = fixture.issue("diary_app").await?;
    let notes_value = notes["value"].as_str().ok_or("no value")?;
    let notes_id = notes["credential_id"].as_str().ok_or("no id")?;
    let diary_value = diary["value"].as_str().ok_or("no value")?;
    let diary_id = diary["credential_id"].as_str().ok_or("no id")?;
    for (presented, live) in [
        (diary_value.to_owned(), diary_id),
        (changed(notes_value), notes_id),
        (
            notes_value.replacen("lys-client.", "lys-other.", 1),
            notes_id,
        ),
        ("not-a-credential".to_owned(), notes_id),
    ] {
        let refused = fixture
            .authenticate("notes_app", &presented, &[live], &digest)
            .await
            .err()
            .ok_or("a credential not this app's was confirmed")?;
        assert!(refused.contains("AppClientCredentialRefused"), "{refused}");
        assert!(!refused.contains(&presented), "{refused}");
    }
    let mismatch = fixture
        .authenticate("notes_app", notes_value, &[notes_id], &other_digest)
        .await
        .err()
        .ok_or("a custody the approval does not record was confirmed")?;
    assert!(mismatch.contains("AppClientCustodyMismatch"), "{mismatch}");
    let none = fixture
        .issue("unknown_app")
        .await
        .err()
        .ok_or("an app with no custody was issued a credential")?;
    assert!(none.contains("AppClientNoCustody"), "{none}");
    fixture.close()
}
