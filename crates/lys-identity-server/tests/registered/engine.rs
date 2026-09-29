//! Registered lifecycle regression coverage.
use super::support::{PERSON, TestResult, fixture_judging, inactive, login, stored};
use lys_identity_server::spicedb::SpiceDbSettings;
use serde_json::json;
use std::error::Error;

struct CountedEngine {
    address: std::net::SocketAddr,
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    key: tempfile::TempDir,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    server: Option<std::thread::JoinHandle<std::io::Result<()>>>,
}

impl CountedEngine {
    fn start() -> Result<Self, Box<dyn Error>> {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let router = axum::Router::new()
            .fallback(counted_engine_answer)
            .with_state(std::sync::Arc::clone(&calls));
        let (stop, stopped) = tokio::sync::oneshot::channel();
        // The production gateway client blocks on TCP: its stand-in needs independent I/O.
        let server = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener)?;
                axum::serve(listener, router)
                    .with_graceful_shutdown(async {
                        let _ = stopped.await;
                    })
                    .await
            })
        });
        let key = tempfile::TempDir::new()?;
        std::fs::write(key.path().join("key"), "local-regression-key")?;
        Ok(Self {
            address,
            calls,
            key,
            stop: Some(stop),
            server: Some(server),
        })
    }

    fn settings(&self) -> SpiceDbSettings {
        SpiceDbSettings {
            endpoint: self.address.to_string(),
            key_file: self.key.path().join("key"),
            mirror: "registered_admission".to_owned(),
        }
    }

    fn count(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for CountedEngine {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(server) = self.server.take() {
            assert!(
                matches!(server.join(), Ok(Ok(()))),
                "engine server did not shut down cleanly"
            );
        }
    }
}

async fn counted_engine_answer(
    axum::extract::State(calls): axum::extract::State<
        std::sync::Arc<std::sync::atomic::AtomicUsize>,
    >,
    request: axum::extract::Request,
) -> (axum::http::StatusCode, &'static str) {
    calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    match (request.method().as_str(), request.uri().path()) {
        ("POST", "/v1/schema/read") => (axum::http::StatusCode::OK, "{\"schemaText\":\"\"}"),
        ("POST", "/v1/schema/write") => (axum::http::StatusCode::OK, "{}"),
        ("POST", "/v1/relationships/read") => (axum::http::StatusCode::OK, ""),
        _ => (
            axum::http::StatusCode::NOT_FOUND,
            "unexpected test engine request",
        ),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registered_caller_is_refused_before_any_permission_engine_request() -> TestResult {
    let engine = CountedEngine::start()?;
    let (service, person, _) = fixture_judging(PERSON, false, Some(engine.settings())).await?;
    let cookie = service.sign_in(login(PERSON)).await?;
    let before = engine.count();
    let bytes = stored(&service)?;
    let answer = service.get("/grants", Some(&cookie)).await?;
    assert_eq!(
        engine.count(),
        before,
        "Registered request reached permission engine"
    );
    assert_eq!(stored(&service)?, bytes);
    inactive(&answer, person);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_control_reaches_the_permission_engine_and_reads_grants() -> TestResult {
    let engine = CountedEngine::start()?;
    let (service, _, _) = fixture_judging(PERSON, true, Some(engine.settings())).await?;
    let cookie = service.sign_in(login(PERSON)).await?;
    let before = engine.count();
    let answer = service.get("/grants", Some(&cookie)).await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    assert_eq!(answer.1["grants"], json!([]));
    assert!(
        engine.count() > before,
        "Active control never exercised engine counter"
    );
    Ok(())
}
