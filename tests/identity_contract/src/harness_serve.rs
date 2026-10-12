//! Start the private HTTP service with contract refusal checking.

use std::error::Error;
use std::io;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Instant;

use lys_core::clock::ClockSource;
use lys_identity_server::{Config, Say, service, service_saying, service_with_clock, service_saying_with_clock};

/// Records a named elapsed interval for temporary fixture diagnosis.
pub struct StageTimer {
    name: &'static str,
    started: u128,
}

fn elapsed_ns() -> u128 {
    static CLOCK: OnceLock<Instant> = OnceLock::new();
    CLOCK
        .get_or_init(|| {
            eprintln!("pikelet_test_pid {}", std::process::id());
            Instant::now()
        })
        .elapsed()
        .as_nanos()
}

impl StageTimer {
    /// Starts an interval that is reported when its timer drops.
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            started: elapsed_ns(),
        }
    }
}

impl Drop for StageTimer {
    fn drop(&mut self) {
        eprintln!(
            "pikelet_stage {} {} {}",
            self.name,
            self.started,
            elapsed_ns()
        );
    }
}

pub(crate) struct DropMarker {
    name: &'static str,
    armed: bool,
}

impl DropMarker {
    pub(crate) fn new(name: &'static str) -> Self {
        Self { name, armed: false }
    }

    pub(crate) fn arm(&mut self) {
        self.armed = true;
    }
}

impl Drop for DropMarker {
    fn drop(&mut self) {
        if self.armed {
            eprintln!("pikelet_drop {} {}", self.name, elapsed_ns());
        }
    }
}

/// Fixture tasks run independently so synchronous drop can join their exit.
pub(crate) struct Serving {
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    worker: Option<std::thread::JoinHandle<io::Result<()>>>,
}

impl Serving {
    pub(crate) async fn start(
        servers: Vec<(tokio::net::TcpListener, axum::Router)>,
    ) -> io::Result<Self> {
        let timing = StageTimer::new("fixture.http_start");
        let servers = servers
            .into_iter()
            .map(|(listener, router)| Ok((listener.into_std()?, router)))
            .collect::<io::Result<Vec<_>>>()?;
        let (shutdown, stopped) = tokio::sync::oneshot::channel();
        let (ready, started) = tokio::sync::oneshot::channel();
        let worker = std::thread::Builder::new().name("fixture-http".to_owned()).spawn(move || {
            let runtime_timing = StageTimer::new("fixture.worker_runtime");
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
            drop(runtime_timing);
            runtime.block_on(async move {
                let ready_timing = StageTimer::new("fixture.worker_ready");
                let mut tasks = tokio::task::JoinSet::new();
                for (listener, router) in servers {
                    let listener = tokio::net::TcpListener::from_std(listener)?;
                    tasks.spawn(async move {
                        axum::serve(listener, router.into_make_service_with_connect_info::<std::net::SocketAddr>()).await
                    });
                }
                ready.send(()).map_err(|()| io::Error::other("fixture_start_cancelled"))?;
                drop(ready_timing);
                let mut failures = Vec::new();
                tokio::select! {
                    result = stopped => {
                        if result.is_err() {
                            failures.push("fixture_shutdown_sender_lost".to_owned());
                        }
                    }
                    result = tasks.join_next() => {
                        failures.push(format!("fixture_server_exited_before_shutdown: {result:?}"));
                    }
                }
                tasks.abort_all();
                while let Some(result) = tasks.join_next().await {
                    match result {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => failures.push(format!("fixture_server_failed: {error}")),
                        Err(error) if error.is_cancelled() => {}
                        Err(error) => failures.push(format!("fixture_task_failed: {error}")),
                    }
                }
                if failures.is_empty() { Ok(()) } else { Err(io::Error::other(failures.join("; "))) }
            })
        })?;
        let mut serving = Self {
            shutdown: Some(shutdown),
            worker: Some(worker),
        };
        if let Err(error) = started.await {
            let shutdown = serving.stop();
            return Err(io::Error::other(format!(
                "fixture_start_failed: {error}; shutdown: {shutdown:?}"
            )));
        }
        drop(timing);
        Ok(serving)
    }

    pub(crate) fn stop(&mut self) -> io::Result<()> {
        let timing = StageTimer::new("fixture.http_stop");
        if let Some(shutdown) = self.shutdown.take() {
            // A server that already exited reports its failure through the worker.
            match shutdown.send(()) {
                Ok(()) | Err(()) => {}
            }
        }
        let result = match self.worker.take() {
            Some(worker) => worker
                .join()
                .map_err(|error| io::Error::other(format!("fixture_worker_panicked: {error:?}")))?,
            None => Ok(()),
        };
        drop(timing);
        result
    }
}

impl Drop for Serving {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("fixture_shutdown_failed: {error}");
        }
    }
}

/// The service `config` describes, answering on `listener`, and a client
/// that follows no redirect.
pub(crate) async fn serve(
    listener: tokio::net::TcpListener,
    config: &Config,
    say: Option<Say>,
    clock: &ClockSource,
) -> Result<(Serving, reqwest::Client), Box<dyn Error>> {
    let timing = StageTimer::new("serve.total");
    let stage = StageTimer::new("serve.refusals_inventory");
    let documented = crate::refusals::listed();
    drop(stage);
    let stage = StageTimer::new("serve.service_open");
    let app = match (clock, say) {
        (ClockSource::System, Some(say)) => service_saying(config, say).await?,
        (ClockSource::System, None) => service(config).await?,
        (ClockSource::Supplied(clock), Some(say)) => service_saying_with_clock(config, say, ClockSource::Supplied(Arc::clone(clock))).await?,
        (ClockSource::Supplied(clock), None) => service_with_clock(config, ClockSource::Supplied(Arc::clone(clock))).await?,
    };
    drop(stage);
    let stage = StageTimer::new("serve.middleware");
    let app = app.layer(axum::middleware::from_fn(move |request, next| {
        let documented = Arc::clone(&documented);
        async move {
            let timing = StageTimer::new("http.server_request");
            let answer = crate::refusals::listed_only(documented, request, next).await;
            drop(timing);
            answer
        }
    }));
    drop(stage);
    let stage = StageTimer::new("serve.client_build");
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    drop(stage);
    let stage = StageTimer::new("serve.http_start");
    let server = Serving::start(vec![(listener, app)]).await?;
    drop(stage);
    drop(timing);
    Ok((server, client))
}
