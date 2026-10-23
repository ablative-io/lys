#![cfg(test)]
//! AGENTS-020 R4, against the fixture identity server: `lys agent pair` makes
//! the key here, tells the four words as soon as Lys gives them, holds until
//! the person decides, and writes the seat file the bridge reads, mode 0600,
//! never over one that exists; a decline is its own name and writes nothing.
//! No clocks: the person's decision is sent once the words have been told,
//! and each step ends on Lys's answer or the command's return.

use std::error::Error;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::ca::certificate_subject_public_key;
use lys_core::seat_file::SeatFile;
use lys_identity::OperationId;
use lys_identity::grants::shipped_model;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

use super::{Pair, Paired, Told, pair};
use crate::commands::mcp_bridge::read_key;
use crate::commands::pem::decode_certificate;

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-pair";

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

/// A service on the shipped model with the administrator signed in, and a
/// root they hold that may be passed on to an agent: the roots choice.
struct Lys {
    service: Service,
    ada: String,
    person: String,
    root: String,
}

impl Lys {
    async fn up() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) = Service::start_asking(&shipped_model(), None, None, |config| {
            Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)
        })
        .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let person = seeded.people[0].id.to_string();
        let root = ok(service
            .post(
                "/grants/roots",
                Some(&ada),
                &json!({"operation": op()?, "route": "api", "holder": person,
                    "resource": {"kind": "person", "id": person}, "relation": "only.agent.create",
                    "pass_on": {"kind": "to", "actions": ["agent.create"], "recipients": ["agent"]},
                    "window": {"starts_at": 0, "ends_at": null}}),
            )
            .await?)?;
        let root = root["grant"].as_str().ok_or("no grant")?.to_owned();
        Ok(Self {
            service,
            ada,
            person,
            root,
        })
    }

    /// `lys agent pair` for `name` writing `seat`, on a blocking thread so the
    /// service keeps answering on the runtime; what it told the person
    /// arrives on the channel as soon as Lys gave it.
    fn paired(
        &self,
        name: &str,
        seat: &Path,
    ) -> (
        tokio::task::JoinHandle<Result<Paired, String>>,
        mpsc::Receiver<Told>,
    ) {
        Self::paired_at(&self.service.base, name, seat)
    }

    /// As [`Lys::paired`], reaching Lys at `url`.
    fn paired_at(
        url: &str,
        name: &str,
        seat: &Path,
    ) -> (
        tokio::task::JoinHandle<Result<Paired, String>>,
        mpsc::Receiver<Told>,
    ) {
        let (tx, rx) = mpsc::channel();
        let (url, name, seat) = (url.to_owned(), name.to_owned(), seat.to_path_buf());
        let handle = tokio::task::spawn_blocking(move || {
            pair(
                &Pair {
                    lys: &url,
                    name: &name,
                    seat: &seat,
                },
                &mut |told| {
                    let _sent = tx.send(told.clone());
                },
            )
            .map_err(|refusal| refusal.to_string())
        });
        (handle, rx)
    }

    /// The person's decision on `code`.
    async fn decide(&self, code: &str, approve: bool) -> Result<Value, Box<dyn Error>> {
        let path = format!(
            "/pair/{}/{}",
            code.replace(' ', "%20"),
            if approve { "approve" } else { "decline" }
        );
        let body = json!({"operation": op()?, "activate": op()?, "certificate": op()?,
            "display_name": "Pancake the Paired",
            "grants": [{"operation": op()?, "source": self.root,
                "resource": {"kind": "person", "id": self.person}, "relation": "only.agent.create",
                "window": {"starts_at": 0, "ends_at": null}}]});
        ok(self.service.post(&path, Some(&self.ada), &body).await?)
    }
}

/// What the command told, received on its own event.
async fn told(rx: mpsc::Receiver<Told>) -> Result<Told, Box<dyn Error>> {
    Ok(tokio::task::spawn_blocking(move || rx.recv()).await??)
}

/// The seat file's key certified by its own certificate, mode 0600.
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
async fn pair_tells_the_words_then_writes_a_certified_seat_file_after_approval() -> TestResult {
    let lys = Lys::up().await?;
    let dir = tempfile::tempdir()?;
    let seat = dir.path().join("pancake.seat.json");
    let (handle, rx) = lys.paired("Pancake", &seat);
    let told = told(rx).await?;
    assert_eq!(told.code.split(' ').count(), 4, "{}", told.code);
    assert!(told.address.ends_with("/pair"), "{}", told.address);
    assert!(
        !seat.exists(),
        "the seat file was written before any decision"
    );
    let approved = lys.decide(&told.code, true).await?;
    let paired = handle.await??;
    assert_eq!(paired.agent, approved["agent"], "{approved}");
    assert_eq!(paired.serial, approved["serial"], "{approved}");
    assert_eq!(paired.handle, "Pancake the Paired");
    assert!(!paired.person.is_empty());
    assert_eq!(paired.path, seat);
    let written = certified_seat(&seat)?;
    assert_eq!(written.lys_agent_id.as_deref(), Some(paired.agent.as_str()));
    assert_eq!(written.lys_url.as_deref(), Some(lys.service.base.as_str()));
    assert_eq!(written.serial.as_deref(), Some(paired.serial.as_str()));
    assert!(written.door_url.is_none() && written.handle.is_none());
    // The root the person gave is the agent's.
    let grants = ok(lys.service.get("/grants", Some(&lys.ada)).await?)?;
    assert!(
        grants["grants"]
            .as_array()
            .is_some_and(|all| all.iter().any(|grant| {
                grant["holder"] == paired.agent && grant["relation"] == "only.agent.create"
            })),
        "{grants}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn pair_declined_is_named_and_nothing_is_written() -> TestResult {
    let lys = Lys::up().await?;
    let dir = tempfile::tempdir()?;
    let seat = dir.path().join("declined.seat.json");
    let (handle, rx) = lys.paired("Pancake", &seat);
    let told = told(rx).await?;
    lys.decide(&told.code, false).await?;
    let refusal = handle
        .await?
        .expect_err("a declined pairing wrote a seat file");
    assert!(refusal.contains("pair_declined"), "{refusal}");
    assert!(
        !seat.exists(),
        "a declined pairing wrote {}",
        seat.display()
    );
    assert!(
        std::fs::read_dir(dir.path())?.next().is_none(),
        "something was left beside the seat path"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn pair_refuses_an_existing_seat_path_before_making_a_key() -> TestResult {
    let lys = Lys::up().await?;
    let dir = tempfile::tempdir()?;
    let seat = dir.path().join("taken.seat.json");
    std::fs::write(&seat, b"taken")?;
    let (handle, rx) = lys.paired("Pancake", &seat);
    let refusal = handle.await?.expect_err("an existing path was replaced");
    assert!(refusal.contains("seat_file_exists"), "{refusal}");
    assert!(
        rx.try_recv().is_err(),
        "words were asked for before the path was checked"
    );
    assert_eq!(std::fs::read(&seat)?, b"taken");
    Ok(())
}

/// What a proxy in front of Lys saw: holds on `GET /pair/{wait}` and takes.
struct Seen {
    holds: AtomicUsize,
    takes: AtomicUsize,
}

/// A proxy that cuts the first hold on `GET /pair/{wait}` the way Cloudflare
/// does, a 524 with no body, and carries every other exchange to Lys whole.
/// Answers the base the command is given.
fn cutting_proxy(upstream: &str) -> Result<(String, Arc<Seen>), Box<dyn Error>> {
    let authority = upstream
        .strip_prefix("http://")
        .and_then(|rest| rest.split('/').next())
        .ok_or("the fixture's base is not http://")?
        .to_owned();
    let path = upstream
        .strip_prefix("http://")
        .and_then(|rest| rest.find('/').map(|at| rest[at..].to_owned()))
        .unwrap_or_default();
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let seen = Arc::new(Seen {
        holds: AtomicUsize::new(0),
        takes: AtomicUsize::new(0),
    });
    let counted = Arc::clone(&seen);
    std::thread::spawn(move || {
        for client in listener.incoming().flatten() {
            let authority = authority.clone();
            let counted = Arc::clone(&counted);
            std::thread::spawn(move || carry(client, &authority, &counted));
        }
    });
    Ok((format!("http://127.0.0.1:{port}{path}"), seen))
}

fn carry(mut client: TcpStream, authority: &str, seen: &Seen) {
    let mut head = [0_u8; 512];
    let Ok(peeked) = client.peek(&mut head) else {
        return;
    };
    let line = String::from_utf8_lossy(&head[..peeked]);
    let request = line.lines().next().unwrap_or_default().to_owned();
    let hold =
        request.starts_with("GET ") && request.contains("/pair/") && !request.contains("/taken");
    if request.starts_with("POST ") && request.contains("/taken") {
        seen.takes.fetch_add(1, Ordering::SeqCst);
    }
    if hold && seen.holds.fetch_add(1, Ordering::SeqCst) == 0 {
        let mut consumed = vec![0_u8; peeked];
        let _ = client.read_exact(&mut consumed);
        let _ = client.write_all(
            b"HTTP/1.1 524 A Timeout Occurred\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        );
        return;
    }
    let Ok(upstream) = TcpStream::connect(authority) else {
        return;
    };
    let (Ok(mut up), Ok(mut down)) = (upstream.try_clone(), client.try_clone()) else {
        return;
    };
    let to_lys = std::thread::spawn(move || {
        let _ = std::io::copy(&mut client, &mut up);
        let _ = up.shutdown(std::net::Shutdown::Write);
    });
    let mut from_lys = upstream;
    let _ = std::io::copy(&mut from_lys, &mut down);
    let _ = down.shutdown(std::net::Shutdown::Write);
    let _ = to_lys.join();
}

#[tokio::test(flavor = "multi_thread")]
async fn pair_holds_again_when_a_proxy_cuts_the_wait_and_tells_lys_the_answer_is_taken()
-> TestResult {
    let lys = Lys::up().await?;
    let (base, seen) = cutting_proxy(&lys.service.base)?;
    let dir = tempfile::tempdir()?;
    let seat = dir.path().join("cut.seat.json");
    let (handle, rx) = Lys::paired_at(&base, "Pancake", &seat);
    let told = told(rx).await?;
    // The first hold is cut by the proxy before any decision; the command
    // holds again, and that second hold is the one the decision answers.
    let approved = lys.decide(&told.code, true).await?;
    let paired = handle.await??;
    assert_eq!(paired.agent, approved["agent"], "{approved}");
    assert!(
        seen.holds.load(Ordering::SeqCst) >= 2,
        "the cut hold was not held again: {} hold(s)",
        seen.holds.load(Ordering::SeqCst)
    );
    assert_eq!(
        seen.takes.load(Ordering::SeqCst),
        1,
        "the answer was not taken once"
    );
    let written = certified_seat(&seat)?;
    assert_eq!(written.lys_url.as_deref(), Some(base.as_str()));
    // Taken, the pairing is gone from Lys: the person's lookup finds nothing.
    let gone = lys
        .service
        .get(
            &format!("/pair/{}/waiting", told.code.replace(' ', "%20")),
            Some(&lys.ada),
        )
        .await?;
    assert_eq!(gone.0, 404, "{}", gone.1);
    Ok(())
}
