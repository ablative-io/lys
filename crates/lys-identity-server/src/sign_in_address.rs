//! Forwarded addresses are authoritative only from explicitly named peers.
//! Walk X-Forwarded-For from the right, skipping trusted hops. An untrusted
//! peer's header is ignored, including malformed text. No proxy is trusted
//! by default. The proxy must append the actual peer it accepted.
use crate::error::ServerError;
use axum::http::HeaderMap;
use std::net::IpAddr;

fn refused() -> ServerError {
    ServerError::SignInFailed {
        reason: "a trusted proxy did not name a valid client address".to_owned(),
    }
}

pub(crate) fn resolve(
    peer: IpAddr,
    headers: &HeaderMap,
    trusted: &[IpAddr],
) -> Result<IpAddr, ServerError> {
    let is_trusted = |ip: IpAddr| {
        trusted
            .iter()
            .any(|allowed| allowed.to_canonical() == ip.to_canonical())
    };
    if !is_trusted(peer) {
        return Ok(peer);
    }
    let lines: Vec<_> = headers.get_all("x-forwarded-for").iter().collect();
    for line in lines.iter().rev() {
        let Ok(line) = line.to_str() else {
            return Err(refused());
        };
        for hop in line.rsplit(',') {
            let Ok(address) = hop.trim().parse::<IpAddr>() else {
                return Err(refused());
            };
            if !is_trusted(address) {
                return Ok(address);
            }
        }
    }
    Err(refused())
}

#[cfg(test)]
#[path = "sign_in_address_tests.rs"]
mod tests;
