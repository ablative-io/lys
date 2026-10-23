//! `lys agent pair` (AGENTS-020 R4): the agent's side of a pairing, for an
//! agent that is not a product's seat. Run on the computer the agent will
//! run on, it makes the agent's key here (as `lys agent enrol` does), asks
//! Lys for a pairing code over a PKCS#10 request on that key, prints the
//! four words and the address for the person, holds `GET /pair/{wait}` open
//! until the person decides, and writes the seat file with the certificate,
//! its serial, the agent id and the Lys address.
//!
//! The hold is Lys's own: the request is answered when the person decides,
//! and not before. No poll interval and no deadline (Tom: no fixed waits,
//! ever). A decline is printed by its name, `pair_declined`, and nothing is
//! written; a path that already names anything is refused `seat_file_exists`
//! before a key is made, and again at the link, never overwritten. What is
//! printed is the words, the address, the path, the agent, the serial and
//! the apps the agent may reach; never key material.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use lys_core::ca::{certificate_subject_public_key, create_certificate_request};
use lys_core::keys::Ed25519Identity;
use lys_core::seat_file::SeatFile;
use rand::RngCore;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::cli::PairArgs;
use crate::commands::enrol::{absent, write_whole};
use crate::commands::error::CliResult;
use crate::commands::mcp_bridge::tls::{Transport, TrustPath, post};
use crate::commands::mcp_bridge::{endpoint, named};
use crate::commands::output::Emitter;
use crate::commands::pem;
use crate::commands::seat_client::refused;
use crate::identity::loopback_http::Request;

/// One pairing: where Lys is, the name proposed and where the seat file goes.
pub struct Pair<'a> {
    /// Lys's API address: `https://<lys host>/api`, or `http://` on loopback.
    pub lys: &'a str,
    /// The name the agent proposes.
    pub name: &'a str,
    /// The seat file's path.
    pub seat: &'a Path,
}

/// What the person is told, as soon as Lys gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Told {
    /// The four words.
    pub code: String,
    /// Where the person enters them.
    pub address: String,
}

/// One app the agent may reach, as Lys answered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    /// The app's id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its address.
    pub address: String,
}

/// What a pairing wrote and learned: printed, never the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paired {
    /// The seat file.
    pub path: PathBuf,
    /// The agent.
    pub agent: String,
    /// The name the person settled on.
    pub handle: String,
    /// The responsible person's name.
    pub person: String,
    /// The certificate's serial in Lys's certificate log.
    pub serial: String,
    /// The approved apps the agent may reach.
    pub apps: Vec<Reach>,
}

/// Runs `lys agent pair`.
///
/// # Errors
/// `seat_file_exists` naming the path; `pair_declined` when the person
/// declined; Lys's refusal by its own name with the route;
/// `pair_answer_unreadable` when Lys answered no certificate over this key;
/// the transport's own refusals as `lys mcp` names them.
pub fn run(args: &PairArgs, json: bool) -> CliResult<()> {
    let paired = pair(
        &Pair {
            lys: &args.lys,
            name: &args.name,
            seat: &args.seat,
        },
        &mut |told| {
            if json {
                println!("{}", json!({"code": told.code, "address": told.address}));
            } else {
                println!("Say these four words to your person: {}", told.code);
                println!("They enter them at: {}", told.address);
                println!("Waiting for their decision.");
            }
        },
    )?;
    let mut emitter = Emitter::new(json);
    emitter.field("seat file", "path", paired.path.display().to_string());
    emitter.field("agent", "agent", paired.agent);
    emitter.field("name", "handle", paired.handle);
    emitter.field("person", "person", paired.person);
    emitter.field("serial", "serial", paired.serial);
    let apps: Vec<Value> = paired
        .apps
        .iter()
        .map(|app| json!({"id": app.id, "name": app.name, "address": app.address}))
        .collect();
    emitter.field("apps", "apps", Value::Array(apps));
    emitter.finish();
    Ok(())
}

/// Make the key, ask for the words, tell them, hold for the decision, and
/// write the seat file.
///
/// # Errors
/// As [`run`].
pub fn pair(pair: &Pair<'_>, told: &mut dyn FnMut(&Told)) -> CliResult<Paired> {
    absent(pair.seat)?;
    let base = pair.lys.trim_end_matches('/');
    let mut seed = Zeroizing::new([0_u8; 32]);
    rand::rng().fill_bytes(seed.as_mut_slice());
    let identity = Arc::new(Ed25519Identity::from_seed(&seed));
    let request = create_certificate_request(&identity, pair.name)
        .map_err(|error| refused("pair_request_unmade", error.to_string()))?;
    let body = json!({
        "request": pem::encode_certificate_request(&request),
        "name": pair.name,
    })
    .to_string();
    let asked = answered(base, "POST", "/pair", body.as_bytes())?;
    let wait = text(&asked, "wait", "/pair")?;
    told(&Told {
        code: text(&asked, "code", "/pair")?,
        address: text(&asked, "address", "/pair")?,
    });
    let route = format!("/pair/{wait}");
    let answer = answered(base, "GET", &route, b"")?;
    let certificate_pem = text(&answer, "certificate_pem", &route)?;
    let der = pem::decode_certificate(certificate_pem.as_bytes(), Path::new(&route))
        .map_err(|_unread| unreadable(&route, "carries a certificate PEM that cannot be read"))?;
    let key = certificate_subject_public_key(&der)
        .map_err(|_unread| unreadable(&route, "carries a certificate that cannot be read"))?;
    if key != identity.public_key_bytes() {
        return Err(unreadable(
            &route,
            "certifies another key than the one made here",
        ));
    }
    let agent = text(&answer, "agent", &route)?;
    let serial = text(&answer, "serial", &route)?;
    let apps = answer
        .get("apps")
        .and_then(Value::as_array)
        .map(|all| {
            all.iter()
                .map(|app| {
                    Ok(Reach {
                        id: text(app, "id", &route)?,
                        name: text(app, "name", &route)?,
                        address: text(app, "address", &route)?,
                    })
                })
                .collect::<CliResult<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut seat = SeatFile::new(&seed, (base, &agent), None);
    seat.certificate_pem = Some(certificate_pem);
    seat.serial = Some(serial.clone());
    let written = seat
        .to_json()
        .map_err(|error| refused("seat_file_unwritten", error.to_string()))?;
    write_whole(pair.seat, written.as_bytes())?;
    Ok(Paired {
        path: pair.seat.to_path_buf(),
        agent,
        handle: text(&answer, "handle", &route)?,
        person: text(&answer, "person", &route)?,
        serial,
        apps,
    })
}

fn unreadable(route: &str, why: &str) -> crate::commands::error::CliError {
    refused(
        "pair_answer_unreadable",
        format!("Lys's answer to {route} {why}; no seat file was written"),
    )
}

/// The string member `key` of `answer`, or `pair_answer_unreadable`.
fn text(answer: &Value, key: &str, route: &str) -> CliResult<String> {
    answer
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| unreadable(route, &format!("names no {key}")))
}

/// Send `method` `route` to Lys at `base` and read its answer: the JSON it
/// answered 200 with, or its refusal by its own name with the route. A GET
/// Lys holds open is held here too: the transport has no clock.
fn answered(base: &str, method: &str, route: &str, body: &[u8]) -> CliResult<Value> {
    let target = endpoint(&format!("{base}{route}"))?;
    let transport = if target.secure {
        Transport::Tls(TrustPath::system()?)
    } else {
        Transport::Loopback
    };
    let headers: [(&str, &[u8]); 2] = [
        ("content-type", b"application/json"),
        ("accept", b"application/json"),
    ];
    let response = post(
        &transport,
        &target.authority,
        &Request {
            method,
            path: &target.path,
            headers: &headers,
            body,
        },
    )
    .map_err(|unanswered| {
        let reason = named(&unanswered);
        let (name, words) = reason
            .split_once(": ")
            .unwrap_or(("pair_unanswered", &reason));
        refused(name, format!("{words} ({method} {route})"))
    })?;
    let answer: Value = serde_json::from_slice(&response.body).map_err(|_unread| {
        refused(
            "pair_answer_unreadable",
            format!(
                "Lys answered {method} {route} with {} and no JSON",
                response.status
            ),
        )
    })?;
    if response.status != 200 {
        let name = ["refusal", "error"]
            .iter()
            .find_map(|key| answer.get(*key).and_then(Value::as_str))
            .unwrap_or("lys_refused");
        let words = ["reason", "words", "message"]
            .iter()
            .find_map(|key| answer.get(*key).and_then(Value::as_str))
            .unwrap_or("");
        return Err(refused(
            name,
            format!(
                "{words} (Lys answered {method} {route} with {})",
                response.status
            ),
        ));
    }
    Ok(answer)
}

#[cfg(test)]
#[path = "pair_tests.rs"]
mod tests;
