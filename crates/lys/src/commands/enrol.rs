//! `lys agent enrol` (AGENTS-017 R1, R5): one act, run on the computer the new
//! agent will run on, that makes the agent's key, has Lys certify it, and
//! writes the seat file where the agent's reviewed profile expects it.
//!
//! The key is made here, in memory wiped when dropped, and never leaves this
//! computer: Lys is sent the PKCS#10 request over it, as the PEM `lys ca
//! request` writes, and answers the certificate. The request is posted to
//! `POST /agents/{id}/certificates` either signed as the registering agent,
//! exactly as `lys mcp` signs (`--as`, AGENTS-016 R4's first certificate), or
//! carrying the responsible person's app pass (`--pass`, every later one). The
//! route's own refusals are printed as Lys answered them, with the route.
//!
//! The seat file is the one shape every reader reads (`lys_core::seat_file`):
//! written whole beside its path and linked into place, mode 0600, so a reader
//! never sees half a file; a path that already names anything is refused
//! `seat_file_exists` before a key is made, and again at the link, never
//! overwritten. What is printed is the path, the agent and the serial; never
//! key material.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::ca::{certificate_subject_public_key, create_certificate_request};
use lys_core::keys::Ed25519Identity;
use lys_core::seat_file::{DoorSeat, SeatFile};
use rand::RngCore;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::cli::EnrolArgs;
use crate::commands::error::CliResult;
use crate::commands::mcp_bridge::tls::{Transport, TrustPath, post};
use crate::commands::mcp_bridge::{Endpoint, endpoint, named, read_key, signature};
use crate::commands::output::Emitter;
use crate::commands::pem;
use crate::commands::seat_client::refused;
use crate::identity::loopback_http::Request;

/// Who asks Lys for the certificate.
pub enum By<'a> {
    /// The registering agent, signing as `lys mcp` signs, with the key in its
    /// seat or key file; its id is named, or read from the seat file.
    Agent {
        /// The registering agent's seat file or PKCS#8 PEM.
        file: &'a Path,
        /// Its id, when the file does not name it.
        id: Option<&'a str>,
    },
    /// The responsible person, carrying their app pass read from a file.
    Person {
        /// The file holding the person's app pass.
        pass: &'a Path,
    },
}

/// One enrolment: where Lys is, the agent, who asks, where the seat file goes
/// and, for a seat an app's door founded, the door's members.
pub struct Enrol<'a> {
    /// Lys's API address: `https://<lys host>/api`, or `http://` on loopback.
    pub url: &'a str,
    /// The agent the key is certified for.
    pub agent: &'a str,
    /// Who asks.
    pub by: By<'a>,
    /// The seat file's path.
    pub out: &'a Path,
    /// The door's members, when an app's door founded the seat.
    pub door: Option<DoorSeat>,
}

/// What an enrolment wrote: printed, never the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enrolled {
    /// The seat file.
    pub path: PathBuf,
    /// The agent.
    pub agent: String,
    /// The certificate's serial in Lys's certificate log.
    pub serial: String,
}

/// Runs `lys agent enrol`.
///
/// # Errors
/// `seat_file_exists` naming the path; `enrol_as_agent_unknown` when `--as`
/// names a key file with no agent id; Lys's refusal by its own name with the
/// route; `enrol_answer_unreadable` when Lys answered no certificate over this
/// key; the transport's own refusals as `lys mcp` names them.
pub fn run(args: &EnrolArgs, json: bool) -> CliResult<()> {
    let by = match (&args.as_file, &args.pass) {
        (_, Some(pass)) => By::Person { pass },
        (Some(file), None) => By::Agent {
            file,
            id: args.as_agent.as_deref(),
        },
        (None, None) => {
            return Err(refused(
                "enrol_asker_missing",
                "name who asks: --as <the registering agent's seat file> or --pass <your app pass file>",
            ));
        }
    };
    let door = match (&args.door, &args.handle, &args.device) {
        (Some(door_url), Some(handle), Some(device)) => Some(DoorSeat {
            door_url: door_url.clone(),
            handle: handle.clone(),
            device: device.clone(),
        }),
        _ => None,
    };
    let enrolled = enrol(&Enrol {
        url: &args.url,
        agent: &args.agent,
        by,
        out: &args.out,
        door,
    })?;
    let mut emitter = Emitter::new(json);
    emitter.field("seat file", "path", enrolled.path.display().to_string());
    emitter.field("agent", "agent", enrolled.agent);
    emitter.field("serial", "serial", enrolled.serial);
    emitter.finish();
    Ok(())
}

/// Make the key, have Lys certify it, and write the seat file.
///
/// # Errors
/// As [`run`].
pub fn enrol(enrol: &Enrol<'_>) -> CliResult<Enrolled> {
    absent(enrol.out)?;
    let base = enrol.url.trim_end_matches('/');
    let route = format!("/agents/{}/certificates", enrol.agent);
    let target = endpoint(&format!("{base}{route}"))?;
    let asker = Asker::read(&enrol.by)?;
    let mut seed = Zeroizing::new([0_u8; 32]);
    rand::rng().fill_bytes(seed.as_mut_slice());
    let identity = Arc::new(Ed25519Identity::from_seed(&seed));
    let request = create_certificate_request(&identity, enrol.agent)
        .map_err(|error| refused("enrol_request_unmade", error.to_string()))?;
    let operation = lys_identity::OperationId::generate()
        .map_err(|error| refused("enrol_request_unmade", error.to_string()))?
        .to_string();
    let body = json!({
        "operation": operation,
        "request": pem::encode_certificate_request(&request),
    })
    .to_string();
    let answer = asked(&target, &asker, &route, body.as_bytes())?;
    let (certificate_pem, serial) = certified(&answer, &operation, &identity, &route)?;
    let mut seat = SeatFile::new(&seed, (base, enrol.agent), enrol.door.clone());
    seat.certificate_pem = Some(certificate_pem);
    seat.serial = Some(serial.clone());
    let text = seat
        .to_json()
        .map_err(|error| refused("seat_file_unwritten", error.to_string()))?;
    write_whole(enrol.out, text.as_bytes())?;
    Ok(Enrolled {
        path: enrol.out.to_path_buf(),
        agent: enrol.agent.to_owned(),
        serial,
    })
}

/// The one who asks, read once: an agent's key and id, or a person's pass.
enum Asker {
    Agent {
        identity: Box<Ed25519Identity>,
        id: String,
    },
    Person {
        pass: Zeroizing<String>,
    },
}

impl Asker {
    fn read(by: &By<'_>) -> CliResult<Self> {
        match by {
            By::Agent { file, id } => {
                let identity = read_key(file)?;
                let id = match id {
                    Some(id) => (*id).to_owned(),
                    None => named_agent(file).ok_or_else(|| {
                        refused(
                            "enrol_as_agent_unknown",
                            format!(
                                "{} names no agent: give --as-agent <the registering agent's id>",
                                file.display()
                            ),
                        )
                    })?,
                };
                Ok(Self::Agent {
                    identity: Box::new(identity),
                    id,
                })
            }
            By::Person { pass: file } => {
                let text = Zeroizing::new(fs::read_to_string(file).map_err(|error| {
                    refused(
                        "enrol_pass_unreadable",
                        format!("{}: {}", file.display(), error.kind()),
                    )
                })?);
                let pass = Zeroizing::new(text.trim().to_owned());
                if pass.is_empty() {
                    return Err(refused(
                        "enrol_pass_unreadable",
                        format!("{} holds no pass", file.display()),
                    ));
                }
                Ok(Self::Person { pass })
            }
        }
    }
}

/// The agent a seat file names as its own, when it is a seat file that does.
fn named_agent(file: &Path) -> Option<String> {
    let text = Zeroizing::new(fs::read_to_string(file).ok()?);
    SeatFile::from_json(&text).ok()?.lys_agent_id.clone()
}

/// Refuse a seat file path that already names anything, before a key is made.
pub(crate) fn absent(out: &Path) -> CliResult<()> {
    match fs::symlink_metadata(out) {
        Ok(_) => Err(exists(out)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(refused(
            "seat_file_unwritten",
            format!("{}: {error}", out.display()),
        )),
    }
}

pub(crate) fn exists(out: &Path) -> crate::commands::error::CliError {
    refused(
        "seat_file_exists",
        format!(
            "{} already exists; lys agent enrol never replaces a seat file",
            out.display()
        ),
    )
}

/// Post the request as `asker`, and read Lys's answer: the certificates view,
/// or Lys's refusal by its own name with the route.
fn asked(target: &Endpoint, asker: &Asker, route: &str, body: &[u8]) -> CliResult<Value> {
    let transport = if target.secure {
        Transport::Tls(TrustPath::system()?)
    } else {
        Transport::Loopback
    };
    let credential = match asker {
        Asker::Agent { identity, id } => (
            "lys-agent-signature",
            signature(identity, id, &target.signed_path, body),
        ),
        Asker::Person { pass } => ("authorization", format!("Bearer {}", pass.as_str())),
    };
    let credential = (credential.0, Zeroizing::new(credential.1));
    let headers: [(&str, &[u8]); 3] = [
        ("content-type", b"application/json"),
        ("accept", b"application/json"),
        (credential.0, credential.1.as_bytes()),
    ];
    let response = post(
        &transport,
        &target.authority,
        &Request {
            method: "POST",
            path: &target.path,
            headers: &headers,
            body,
        },
    )
    .map_err(|unanswered| {
        let reason = named(&unanswered);
        let (name, words) = reason
            .split_once(": ")
            .unwrap_or(("enrol_unanswered", &reason));
        refused(name, format!("{words} (POST {route})"))
    })?;
    let answer: Value = serde_json::from_slice(&response.body).map_err(|_unread| {
        refused(
            "enrol_answer_unreadable",
            format!(
                "Lys answered POST {route} with {} and no JSON",
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
                "{words} (Lys answered POST {route} with {})",
                response.status
            ),
        ));
    }
    Ok(answer)
}

/// The certificate Lys recorded under `operation`, as PEM, and its serial;
/// refused unless it certifies the key made here.
fn certified(
    answer: &Value,
    operation: &str,
    identity: &Ed25519Identity,
    route: &str,
) -> CliResult<(String, String)> {
    let unreadable = |why: &str| {
        refused(
            "enrol_answer_unreadable",
            format!("Lys's answer to POST {route} {why}; no seat file was written"),
        )
    };
    let certificate = answer
        .get("certificates")
        .and_then(Value::as_array)
        .and_then(|all| all.iter().find(|one| one["serial"] == operation))
        .ok_or_else(|| unreadable("names no certificate under this request's operation"))?;
    let der = certificate
        .get("der")
        .and_then(Value::as_str)
        .and_then(|der| STANDARD.decode(der).ok())
        .ok_or_else(|| unreadable("carries no certificate DER"))?;
    let key = certificate_subject_public_key(&der)
        .map_err(|_unread| unreadable("carries a certificate that cannot be read"))?;
    if key != identity.public_key_bytes() {
        return Err(unreadable("certifies another key than the one made here"));
    }
    let pem = certificate
        .get("pem")
        .and_then(Value::as_str)
        .ok_or_else(|| unreadable("carries no certificate PEM"))?;
    if pem::decode_certificate(pem.as_bytes(), Path::new(route))
        .ok()
        .as_deref()
        != Some(&der[..])
    {
        return Err(unreadable("carries a PEM that is not its DER"));
    }
    Ok((pem.to_owned(), operation.to_owned()))
}

/// Write `bytes` at `out` whole, mode 0600: into a new file beside it, flushed,
/// then linked into place, which refuses a path that already names anything.
pub(crate) fn write_whole(out: &Path, bytes: &[u8]) -> CliResult<()> {
    let unwritten = |error: &std::io::Error| {
        refused("seat_file_unwritten", format!("{}: {error}", out.display()))
    };
    let name = out.file_name().ok_or_else(|| {
        refused(
            "seat_file_unwritten",
            format!("{} names no file", out.display()),
        )
    })?;
    let mut beside_name = std::ffi::OsString::from(".");
    beside_name.push(name);
    beside_name.push(format!(".enrol-{}", std::process::id()));
    let beside = out.with_file_name(beside_name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&beside)
        .map_err(|error| unwritten(&error))?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let linked = written.and_then(|()| fs::hard_link(&beside, out));
    let removed = fs::remove_file(&beside);
    match linked {
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Err(exists(out)),
        Err(error) => return Err(unwritten(&error)),
        Ok(()) => {}
    }
    removed.map_err(|error| unwritten(&error))?;
    if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::File::open(parent)
            .and_then(|folder| folder.sync_all())
            .map_err(|error| unwritten(&error))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "enrol_tests.rs"]
mod tests;
