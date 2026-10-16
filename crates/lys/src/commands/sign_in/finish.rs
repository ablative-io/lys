//! A pasted address reaches the same loopback callback as the browser.
use crate::commands::error::CliResult;
use crate::commands::seat_client::refused;
use lys_runner::dial::HttpClient;

fn validate(address: &str) -> CliResult<(&str, &str)> {
    let invalid = || {
        refused(
            "sign_in_finish_invalid",
            "name an http numeric loopback address with a port, /lys/sign-in and a query",
        )
    };
    let url = url::Url::parse(address).map_err(|_| invalid())?;
    let (authority, target) = address
        .strip_prefix("http://")
        .and_then(|s| s.split_once('/'))
        .ok_or_else(invalid)?;
    let (host, port) = authority.rsplit_once(':').ok_or_else(invalid)?;
    if !matches!(host, "127.0.0.1" | "[::1]")
        || port.parse::<u16>().is_err()
        || url.path() != "/lys/sign-in"
        || !target.starts_with("lys/sign-in?")
        || url.query().is_none_or(str::is_empty)
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(invalid());
    }
    let origin_end = "http://".len() + authority.len();
    Ok((&address[..origin_end], &address[origin_end..]))
}

pub(super) fn finish(address: &str) -> CliResult<()> {
    let (origin, target) = validate(address)?;
    let response = HttpClient::new(origin)?.send("GET", target, &[], b"")?;
    if response.status == 200 {
        return Ok(());
    }
    let name = std::str::from_utf8(&response.body)
        .ok()
        .filter(|name| {
            name.starts_with("sign_in_")
                && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        .ok_or_else(|| {
            refused(
                "sign_in_finish_refused",
                "the loopback receiver supplied no named answer",
            )
        })?;
    Err(refused(
        name,
        "the waiting sign-in refused this finishing address",
    ))
}

#[cfg(test)]
#[path = "finish_tests.rs"]
mod tests;
