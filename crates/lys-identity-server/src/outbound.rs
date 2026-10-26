//! All HTTP sockets carry SDK-023's kernel keepalive. There is no client
//! request or connection clock; a call is ended by its transport or caller.

use std::time::Duration;

pub(crate) fn http() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .tcp_keepalive(Duration::from_secs(30))
        .tcp_keepalive_interval(Duration::from_secs(10))
        .tcp_keepalive_retries(5)
}

pub(crate) fn keepalive(stream: &impl std::os::fd::AsFd) -> std::io::Result<()> {
    let socket = socket2::SockRef::from(stream);
    socket.set_keepalive(true)?;
    socket.set_tcp_keepalive(
        &socket2::TcpKeepalive::new()
            .with_time(Duration::from_secs(30))
            .with_interval(Duration::from_secs(10))
            .with_retries(5),
    )
}

pub(crate) fn body(response: reqwest::Response, call: crate::waits::Call) -> axum::body::Body {
    axum::body::Body::from_stream(futures_util::stream::try_unfold(
        (response, call),
        |(mut response, mut call)| async move {
            let chunk = call
                .until(response.chunk())
                .await
                .map_err(std::io::Error::other)?
                .map_err(std::io::Error::other)?;
            Ok::<_, std::io::Error>(chunk.map(|chunk| (chunk, (response, call))))
        },
    ))
}
