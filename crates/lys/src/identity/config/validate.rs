//! What a deployment's addresses and clients must be: origins, the admin
//! address and each client checked, each refusal named.

use std::net::Ipv4Addr;

use super::{Client, ClientRole, LOOPBACK_HOSTS, TOKEN_ALGORITHMS, refuse};
use crate::identity::error::{ErrorKind, IdentityResult};

pub(super) fn authority_of(origin: &str) -> &str {
    origin
        .split_once("://")
        .map_or(origin, |(_, rest)| rest.trim_end_matches('/'))
}

fn host_of(authority: &str) -> &str {
    if authority.starts_with('[') {
        return authority
            .find(']')
            .map_or(authority, |end| &authority[..=end]);
    }
    authority.split(':').next().unwrap_or(authority)
}

/// Splits an absolute URI into scheme, authority and the rest, refusing
/// anything lys does not accept as an origin or redirect.
fn split_uri(uri: &str) -> Result<(&str, &str, &str), &'static str> {
    if uri.contains(|c: char| c.is_whitespace() || c.is_control()) {
        return Err("contains whitespace or control characters");
    }
    let (scheme, rest) = uri.split_once("://").ok_or("expected scheme://host")?;
    if scheme != "http" && scheme != "https" {
        return Err("expected an http or https scheme");
    }
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    if authority.is_empty() || authority.contains('@') {
        return Err("expected a host, with no credentials in the authority");
    }
    if let Some((_, port)) = authority.rsplit_once(':')
        && !authority.ends_with(']')
        && port.parse::<u16>().map_or(true, |p| p == 0)
    {
        return Err("the port is not a number from 1 to 65535");
    }
    if scheme == "http" && !LOOPBACK_HOSTS.contains(&host_of(authority)) {
        return Err("plain http is accepted only for a loopback host; use https");
    }
    Ok((scheme, authority, tail))
}

/// The gateway of `network`, a private IPv4 range written from its first
/// address. The range leaves room for the gateway and the four services at
/// the least, so its prefix is at most /29.
pub(super) fn network_gateway(network: &str) -> IdentityResult<Ipv4Addr> {
    let invalid = |detail: &str| {
        refuse(
            ErrorKind::ConfigInvalid,
            "deployment.network",
            format!(
                "expected a private IPv4 range written from its first address, at most /29, as 172.29.47.0/24 ({detail})"
            ),
        )
    };
    let (address, prefix) = network
        .split_once('/')
        .ok_or_else(|| invalid("no /prefix"))?;
    let address: Ipv4Addr = address
        .parse()
        .map_err(|error: std::net::AddrParseError| invalid(&error.to_string()))?;
    let prefix: u32 = prefix
        .parse()
        .map_err(|error: std::num::ParseIntError| invalid(&error.to_string()))?;
    if !address.is_private() {
        return Err(invalid("not a private range"));
    }
    if prefix > 29 {
        return Err(invalid(
            "too few addresses for the gateway and the services",
        ));
    }
    let first = u32::from(address);
    if first & (u32::MAX >> prefix) != 0 {
        return Err(invalid("not the range's first address"));
    }
    // A range that starts private can still run past its private block, as
    // 10.0.0.0/7 runs into 11.0.0.0/8: its last address must be private too.
    if !Ipv4Addr::from(first | (u32::MAX >> prefix)).is_private() {
        return Err(invalid("the range runs past its private block"));
    }
    Ok(Ipv4Addr::from(first | 1))
}

pub(super) fn validate_origin(origin: &str) -> IdentityResult<()> {
    let (_, _, tail) = split_uri(origin)
        .map_err(|rule| refuse(ErrorKind::IssuerInvalid, "issuer.public_origin", rule))?;
    if !tail.is_empty() && tail != "/" {
        return Err(refuse(
            ErrorKind::IssuerInvalid,
            "issuer.public_origin",
            "an origin carries no path, query or fragment",
        ));
    }
    Ok(())
}

pub(super) fn validate_admin_url(url: &str) -> IdentityResult<()> {
    let (scheme, _, tail) = split_uri(url)
        .map_err(|rule| refuse(ErrorKind::IssuerInvalid, "issuer.admin_url", rule))?;
    if scheme != "http" || (!tail.is_empty() && tail != "/") {
        return Err(refuse(
            ErrorKind::IssuerInvalid,
            "issuer.admin_url",
            "lys reaches the admin API over loopback http, as http://127.0.0.1:port",
        ));
    }
    Ok(())
}

pub(super) fn validate_client(role: ClientRole, client: &Client) -> IdentityResult<()> {
    let resource = format!("clients.{}", role.key());
    let id_ok = (2..=256).contains(&client.id.len())
        && client
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte));
    if !id_ok || client.id == "rauthy" {
        return Err(refuse(
            ErrorKind::ClientInvalid,
            &resource,
            "the id is 2 to 256 of letters, digits, . _ - and is not Rauthy's own client",
        ));
    }
    if !(2..=128).contains(&client.name.chars().count()) {
        return Err(refuse(
            ErrorKind::ClientInvalid,
            &resource,
            "the name is 2 to 128 characters",
        ));
    }
    if !TOKEN_ALGORITHMS.contains(&client.token_alg.as_str()) {
        return Err(refuse(
            ErrorKind::ClientInvalid,
            &resource,
            "token_alg is RS256, RS384, RS512 or EdDSA",
        ));
    }
    if client.challenges.iter().any(|method| method != "S256") {
        return Err(refuse(
            ErrorKind::ClientInvalid,
            &resource,
            "the only PKCE challenge method accepted is S256",
        ));
    }
    if client.redirect_uris.is_empty() {
        return Err(refuse(
            ErrorKind::RedirectInvalid,
            &resource,
            "at least one exact redirect URI is required",
        ));
    }
    for uri in client
        .redirect_uris
        .iter()
        .chain(&client.post_logout_redirect_uris)
    {
        let (_, _, tail) = split_uri(uri).map_err(|rule| {
            refuse(
                ErrorKind::RedirectInvalid,
                &resource,
                format!("{uri}: {rule}"),
            )
        })?;
        if tail.contains('#') || tail.contains('*') {
            return Err(refuse(
                ErrorKind::RedirectInvalid,
                &resource,
                format!("{uri}: an exact redirect URI has no fragment and no wildcard"),
            ));
        }
    }
    Ok(())
}
