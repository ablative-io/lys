//! Start the private HTTP service with contract refusal checking.

use std::error::Error;
use std::io;
use std::sync::Arc;

use lys_identity_server::{Config, Say, service, service_saying};

/// Fixture tasks run independently so synchronous drop can join their exit.
pub(crate) struct Serving {
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    worker: Option<std::thread::JoinHandle<io::Result<()>>>,
}

impl Serving {
    pub(crate) async fn start(
        servers: Vec<(tokio::net::TcpListener, axum::Router)>,
    ) -> io::Result<Self> {
        let servers = servers
            .into_iter()
            .map(|(listener, router)| Ok((listener.into_std()?, router)))
            .collect::<io::Result<Vec<_>>>()?;
        let (shutdown, stopped) = tokio::sync::oneshot::channel();
        let (ready, started) = tokio::sync::oneshot::channel();
        let worker = std::thread::Builder::new().name("fixture-http".to_owned()).spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
            runtime.block_on(async move {
                let mut tasks = tokio::task::JoinSet::new();
                for (listener, router) in servers {
                    let listener = tokio::net::TcpListener::from_std(listener)?;
                    tasks.spawn(async move {
                        axum::serve(listener, router.into_make_service_with_connect_info::<std::net::SocketAddr>()).await
                    });
                }
                ready.send(()).map_err(|()| io::Error::other("fixture_start_cancelled"))?;
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
        Ok(serving)
    }

    pub(crate) fn stop(&mut self) -> io::Result<()> {
        if let Some(shutdown) = self.shutdown.take() {
            // A server that already exited reports its failure through the worker.
            match shutdown.send(()) {
                Ok(()) | Err(()) => {}
            }
        }
        match self.worker.take() {
            Some(worker) => worker
                .join()
                .map_err(|error| io::Error::other(format!("fixture_worker_panicked: {error:?}")))?,
            None => Ok(()),
        }
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
) -> Result<(Serving, reqwest::Client), Box<dyn Error>> {
    let documented = crate::refusals::listed();
    let app = match say {
        Some(say) => service_saying(config, say).await?,
        None => service(config).await?,
    };
    let app = app.layer(axum::middleware::from_fn(move |request, next| {
        crate::refusals::listed_only(Arc::clone(&documented), request, next)
    }));
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let server = Serving::start(vec![(listener, app)]).await?;
    Ok((server, client))
}
