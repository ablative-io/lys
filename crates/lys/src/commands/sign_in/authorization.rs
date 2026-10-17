//! An advertised endpoint must stay on the origin that serves authorization.
use super::{CliResult, Value, base, refused};
pub(super) fn endpoint<'a>(metadata: &'a Value, origin: &str, name: &str) -> CliResult<&'a str> {
    let endpoint = metadata[name].as_str().ok_or_else(|| {
        refused(
            "sign_in_unavailable",
            format!("the authorization description names no {name}"),
        )
    })?;
    if !endpoint.starts_with(&format!("{origin}/")) {
        let advertised = metadata["issuer"].as_str().ok_or_else(|| refused("sign_in_unavailable", "the authorization endpoint has another origin but the service names no issuer to use"))?;
        let recommended = base(&format!("{}/api", advertised.trim_end_matches('/')))?;
        return Err(refused(
            "sign_in_origin_mismatch",
            format!(
                "this Lys serves authorization from another origin; run `lys sign-in --server {recommended}`"
            ),
        ));
    }
    Ok(endpoint)
}
