//! A model account's placeholder is found in the header a program carries
//! its token in, read whole or refused by name without being quoted, and
//! never taken for a draw when a call carries a token of its own.

use hyper::HeaderMap;
use hyper::header::HeaderValue;

use super::{Drawn, MARKER, placeholder};
use crate::proxy::error::ProxyError;

const HANDLE: &str = "0a1b2c3d4e5f60718293a4b5c6d7e8f9";
const TOKEN: &str = "d7a0f3c95be14e26a8c1f04d93b27e6c";

fn carrying(name: &'static str, value: &str) -> Result<HeaderMap, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(name, HeaderValue::from_str(value)?);
    Ok(headers)
}

#[test]
fn a_placeholder_is_read_from_either_carrier_and_a_real_token_is_not_a_draw()
-> Result<(), Box<dyn std::error::Error>> {
    let given = placeholder("model-account-op-1", HANDLE, TOKEN);
    let bearer = carrying("authorization", &format!("Bearer {given}"))?;
    let drawn = Drawn::carried(&bearer)?.ok_or("no draw read")?;
    assert_eq!(drawn.account(), "model-account-op-1");
    let keyed = carrying("x-api-key", &given)?;
    assert!(Drawn::carried(&keyed)?.is_some());
    let own = carrying("authorization", "Bearer a-token-of-its-own")?;
    assert!(Drawn::carried(&own)?.is_none());
    assert!(Drawn::carried(&HeaderMap::new())?.is_none());
    Ok(())
}

#[test]
fn a_placeholder_that_does_not_read_is_refused_without_being_quoted()
-> Result<(), Box<dyn std::error::Error>> {
    for broken in [
        format!("{MARKER}.only-an-account"),
        format!("{MARKER}.account.{HANDLE}.not-hex-{TOKEN}"),
        format!("{MARKER}.account with space.{HANDLE}.{TOKEN}"),
    ] {
        let headers = carrying("authorization", &format!("Bearer {broken}"))?;
        match Drawn::carried(&headers) {
            Err(ProxyError::ModelAccount { reason, .. }) => {
                assert!(!reason.contains(TOKEN) && !reason.contains(HANDLE), "{reason}");
            }
            Err(other) => return Err(format!("refused as {other}").into()),
            Ok(_) => return Err(format!("{broken} was read").into()),
        }
    }
    Ok(())
}
