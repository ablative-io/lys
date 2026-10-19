#![cfg(test)]
//! AGENTS-017 R1 and R5, against the fixture identity server: `lys agent
//! enrol` makes the key here, has Lys certify it as the agent that registered
//! the new agent, and writes the seat file the bridge reads, mode 0600, never
//! over one that exists; Lys's refusal is printed by its name with the route;
//! with the door's members the file carries Cambium's `SeatFile` member names.
//! No clocks: each step ends on Lys's answer or the command's return.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::attestation::sign_attestation;
use lys_core::ca::{certificate_subject_public_key, create_certificate_request};
use lys_core::keys::Ed25519Identity;
use lys_core::seat_file::{DoorSeat, SeatFile};
use lys_identity::OperationId;
use lys_identity::grants::shipped_model;
use lys_identity_server::agent_signature::{HEADER, payload};
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use super::{By, Enrol, Enrolled, enrol};
use crate::commands::mcp_bridge::read_key;
use crate::commands::pem::decode_certificate;

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-enrol";
/// The registering agent's seed, kept in its seat file as `privateKeyPem`.
const CREATOR_SEED: [u8; 32] = [5; 32];

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: format!("{subject}@example.test"),
    }
}

fn op() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn ok((status, answer): (u16, Value)) -> Result<Value, Box<dyn Error>> {
    if status == 200 {
        Ok(answer)
    } else {
        Err(format!("status {status}: {answer}").into())
    }
}

/// Lowercase hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// A service on the shipped model where Bea's agent, the creator, holds a
/// certificate and Bea's grant of `agent.create`, so it registers agents for
/// her and issues each one's first certificate; its seat file is beside.
struct Lys {
    service: Service,
    creator: String,
    creator_key: Arc<Ed25519Identity>,
    creator_seat: PathBuf,
    dir: tempfile::TempDir,
}

impl Lys {
    async fn up() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) = Service::start_asking(&shipped_model(), None, None, |config| {
            Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)
        })
        .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let person = seeded.people[1].id.to_string();
        let creator = seeded.people[1].agents[0].id.to_string();
        let creator_key = Arc::new(Ed25519Identity::from_seed(&Zeroizing::new(CREATOR_SEED)));
        let request = STANDARD.encode(create_certificate_request(&creator_key, &creator)?);
        ok(service
            .post(
                &format!("/agents/{creator}/certificates"),
                Some(&bea),
                &json!({"operation": op()?, "request": request}),
            )
            .await?)?;
        let resource = json!({"kind": "person", "id": person});
        let window = json!({"starts_at": 0, "ends_at": null});
        let root = ok(service
            .post(
                "/grants/roots",
                Some(&ada),
                &json!({"operation": op()?, "route": "api", "holder": person,
                    "resource": resource, "relation": "only.agent.create",
                    "pass_on": {"kind": "to", "actions": ["agent.create"], "recipients": ["agent"]},
                    "window": window}),
            )
            .await?)?;
        ok(service
            .post(
                "/grants",
                Some(&bea),
                &json!({"operation": op()?, "route": "api", "source": root["grant"],
                    "recipient": creator, "responsible": person, "resource": resource,
                    "relation": "only.agent.create", "pass_on": {"kind": "use_only"},
                    "window": window}),
            )
            .await?)?;
        let dir = tempfile::tempdir()?;
        let creator_seat = dir.path().join("creator.seat.json");
        let seat = SeatFile::new(
            &Zeroizing::new(CREATOR_SEED),
            (&service.base, &creator),
            None,
        );
        std::fs::write(&creator_seat, seat.to_json()?.as_bytes())?;
        std::fs::set_permissions(&creator_seat, std::fs::Permissions::from_mode(0o600))?;
        Ok(Self {
            service,
            creator,
            creator_key,
            creator_seat,
            dir,
        })
    }

    /// The creator registers a new agent for Bea, signed as `lys mcp` signs;
    /// its id.
    async fn registered(&self, name: &str) -> Result<String, Box<dyn Error>> {
        let body = serde_json::to_vec(&json!({"operation": op()?, "display_name": name}))?;
        let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce = hex(OperationId::generate()?.as_bytes());
        let cose = sign_attestation(&payload("POST", "/agents", &body, at, &nonce), &self.creator_key)
            .to_cose_bytes();
        let header = format!("{} {at} {nonce} {}", self.creator, hex(&cose));
        let answer = ok(self.service.post_signed("/agents", (HEADER, &header), body).await?)?;
        Ok(answer["agent"].as_str().ok_or("no agent")?.to_owned())
    }

    /// `lys agent enrol` for `agent` as the creator, writing `out`, on a
    /// blocking thread so the service keeps answering on the runtime.
    async fn enrolled(
        &self,
        agent: &str,
        out: &Path,
        door: Option<DoorSeat>,
    ) -> Result<Result<Enrolled, String>, Box<dyn Error>> {
        let (url, agent, out, seat) = (
            self.service.base.clone(),
            agent.to_owned(),
            out.to_path_buf(),
            self.creator_seat.clone(),
        );
        Ok(tokio::task::spawn_blocking(move || {
            enrol(&Enrol {
                url: &url,
                agent: &agent,
                by: By::Agent {
                    file: &seat,
                    id: None,
                },
                out: &out,
                door,
            })
            .map_err(|refusal| refusal.to_string())
        })
        .await?)
    }
}

/// The seat file's key certified by its own certificate, mode 0600, read by
/// the bridge.
fn certified_seat(out: &Path) -> Result<SeatFile, Box<dyn Error>> {
    let mode = std::fs::metadata(out)?.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the seat file is mode {mode:o}");
    let held = read_key(out)?;
    let seat = SeatFile::from_json(&std::fs::read_to_string(out)?)?;
    let pem = seat.certificate_pem.as_deref().ok_or("no certificatePem")?;
    let der = decode_certificate(pem.as_bytes(), out)?;
    assert_eq!(
        certificate_subject_public_key(&der)?,
        held.public_key_bytes(),
        "the certificate is over the key the file holds"
    );
    Ok(seat)
}

#[tokio::test(flavor = "multi_thread")]
async fn enrol_writes_a_certified_seat_file_once_and_prints_lys_refusals_by_name() -> TestResult {
    let lys = Lys::up().await?;
    let agent = lys.registered("Newbie").await?;
    let out = lys.dir.path().join("newbie.seat.json");

    let enrolled = lys.enrolled(&agent, &out, None).await??;
    assert_eq!(enrolled.path, out);
    assert_eq!(enrolled.agent, agent);
    let seat = certified_seat(&out)?;
    assert_eq!(seat.lys_agent_id.as_deref(), Some(agent.as_str()));
    assert_eq!(seat.lys_url.as_deref(), Some(lys.service.base.as_str()));
    assert_eq!(seat.serial.as_deref(), Some(enrolled.serial.as_str()));
    assert_eq!(seat.door(), None, "no door was named");

    // The same --out again is refused by name, naming the path, and the file stands.
    let before = std::fs::read(&out)?;
    let again = lys
        .enrolled(&agent, &out, None)
        .await?
        .err()
        .ok_or("a second enrol replaced the seat file")?;
    assert!(again.contains("seat_file_exists"), "{again}");
    assert!(again.contains(&out.display().to_string()), "{again}");
    assert_eq!(std::fs::read(&out)?, before, "the seat file is unchanged");

    // Lys's own refusal, by its name and with the route: a second first certificate.
    let elsewhere = lys.dir.path().join("second.seat.json");
    let refused = lys
        .enrolled(&agent, &elsewhere, None)
        .await?
        .err()
        .ok_or("Lys issued a second first certificate")?;
    assert!(refused.contains("certificate_not_first"), "{refused}");
    assert!(
        refused.contains(&format!("POST /agents/{agent}/certificates")),
        "{refused}"
    );
    assert!(!elsewhere.exists(), "a refused enrol wrote a seat file");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn with_a_door_the_seat_file_carries_cambiums_member_names() -> TestResult {
    let lys = Lys::up().await?;
    let agent = lys.registered("Scott's seat").await?;
    let out = lys.dir.path().join("scott.seat.json");
    let door = DoorSeat {
        door_url: "https://cambium.example.test".to_owned(),
        handle: "scott".to_owned(),
        device: "scotts-laptop".to_owned(),
    };
    lys.enrolled(&agent, &out, Some(door.clone())).await??;
    let members: Value = serde_json::from_str(&std::fs::read_to_string(&out)?)?;
    // Cambium's SeatFile (cambium-door contract/agent_seat.rs), member by member.
    assert_eq!(members["version"], 1);
    assert_eq!(members["doorUrl"], door.door_url.as_str());
    assert_eq!(members["handle"], door.handle.as_str());
    assert_eq!(members["device"], door.device.as_str());
    assert!(members["privateKeyPem"].as_str().is_some_and(|pem| pem.contains("PRIVATE KEY")));
    assert_eq!(members["lysUrl"], lys.service.base.as_str());
    assert_eq!(members["lysAgentId"], agent.as_str());
    let seat = certified_seat(&out)?;
    assert_eq!(seat.door(), Some(door));
    Ok(())
}
