//! Start the private HTTP service with contract refusal checking.

use std::error::Error;
use std::sync::Arc;

use lys_identity_server::{Config, Say, service, service_saying};

/// The service `config` describes, answering on `listener`, and a client
/// that follows no redirect.
pub(crate) async fn serve(
    listener: tokio::net::TcpListener,
    config: &Config,
    say: Option<Say>,
) -> Result<
    (
        tokio::task::JoinHandle<std::io::Result<()>>,
        reqwest::Client,
    ),
    Box<dyn Error>,
> {
    let documented = crate::refusals::listed();
    let app = match say {
        Some(say) => service_saying(config, say).await?,
        None => service(config).await?,
    };
    let app = app.layer(axum::middleware::from_fn(move |request, next| {
        crate::refusals::listed_only(Arc::clone(&documented), request, next)
    }));
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
    });
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    Ok((server, client))
}
