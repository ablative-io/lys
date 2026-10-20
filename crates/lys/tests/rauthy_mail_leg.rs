#![cfg(test)]
//! The measured leg of DIRECTORY-110 R2: does the pinned issuer, given SMTP,
//! write to a person whose account `POST /users` makes, as `accounts::make`
//! makes it? It runs the image compose.yaml pins (already on the machine;
//! nothing is downloaded) with its SMTP pointed at a capturing sink, makes
//! an account, and records what arrives: the letter's subject and body, or
//! the issuer's own line saying its send failed.
//!
//! Declared `test = false`: it needs Docker and the image, so it runs by
//! hand on the identity leg (`cargo nextest run -p lys --test
//! rauthy_mail_leg`), never under the workspace run. The record is written
//! to the path in `LEG_RECORD`, else printed. No secret is in it: every
//! value the container is given is made here and named nowhere.
//!
//! Events, never waits: the issuer is up when its log says its SMTP
//! connection opened, or down when it says the connection failed or the
//! container ends; the outcome is the sink's next letter, or an issuer log
//! line saying the send failed, or the container ending.

use std::error::Error;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use identity_contract::mail_sink::{Captured, MailSink};
use lys::identity::credentials::{Credential, ENCRYPTION_KEY_ID, Shape};
use lys::identity::prepare::{API_KEY_NAME, BOOTSTRAP_ACCOUNT_EMAIL, bootstrap_api_key};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

const IMAGE: &str = "ghcr.io/sebadob/rauthy:0.36.2@sha256:f7d3c501402165e023edbd958b032b41c9cfdac5ea7f8ca7d62217327145577e";
const CONTAINER: &str = "lys-mail-leg";
const EMAIL: &str = "leg@example.test";

/// What the issuer's log said, one line at a time, until the container ends.
enum Said {
    Line(String),
    Ended,
}

/// Every line of `reader`, sent as it is read.
fn forward(reader: impl std::io::Read, lines: &UnboundedSender<Said>) {
    for line in BufReader::new(reader).lines().map_while(Result::ok) {
        if lines.send(Said::Line(line)).is_err() {
            return;
        }
    }
}

/// Follow the container's log on threads of its own: each line as it is
/// written, then `Ended` when the log ends.
fn follow_log() -> UnboundedReceiver<Said> {
    let (sender, lines) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let spawned = Command::new("docker")
            .args(["logs", "-f", CONTAINER])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let Ok(mut child) = spawned else {
            // The receiver is the test; when it is gone, nobody is told.
            let _told = sender.send(Said::Ended);
            return;
        };
        let stderr = child.stderr.take();
        let from_stderr = sender.clone();
        let errors = std::thread::spawn(move || {
            if let Some(stderr) = stderr {
                forward(stderr, &from_stderr);
            }
        });
        if let Some(stdout) = child.stdout.take() {
            forward(stdout, &sender);
        }
        // Both readers have ended at the log's end; the process's status is
        // not the leg's record, the log is.
        let _joined = errors.join();
        let _ended = child.wait();
        let _told = sender.send(Said::Ended);
    });
    lines
}

fn free_port() -> Result<u16, Box<dyn Error>> {
    Ok(std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port())
}

fn docker(args: &[String]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("docker").args(args).output()?;
    if !output.status.success() {
        return Err(format!(
            "docker {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn remove_container() -> Result<String, Box<dyn Error>> {
    docker(&["rm".to_owned(), "-f".to_owned(), CONTAINER.to_owned()])
}

/// `POST` `body` to `url` with `headers`, answering the status and body.
async fn post(
    url: &str,
    api_key: &str,
    body: &serde_json::Value,
) -> Result<(u16, String), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(url)
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {api_key}"))
        .json(body)
        .send()
        .await?;
    let status = answer.status().as_u16();
    Ok((status, answer.text().await?))
}

/// The container's environment: the issuer's own keys as compose.yaml
/// gives them, and its SMTP pointed at the sink on the host.
fn environment(port: u16, sink_port: u16, api_secret: &str) -> Vec<(&'static str, String)> {
    let secret = |length| {
        Credential::generate("secret", Shape::Alphanumeric(length))
            .expose()
            .to_owned()
    };
    vec![
        ("HIQLITE", "true".to_owned()),
        ("HQL_NODE_ID", "1".to_owned()),
        ("HQL_NODES", "1 localhost:8100 localhost:8200".to_owned()),
        ("HQL_DATA_DIR", "/app/data".to_owned()),
        ("HQL_SECRET_RAFT", secret(48)),
        ("HQL_SECRET_API", secret(48)),
        (
            "ENC_KEYS",
            Credential::generate("enc", Shape::EncryptionKey)
                .expose()
                .to_owned(),
        ),
        ("ENC_KEY_ACTIVE", ENCRYPTION_KEY_ID.to_owned()),
        ("LISTEN_SCHEME", "http".to_owned()),
        ("PUB_URL", format!("localhost:{port}")),
        ("RP_ID", "localhost".to_owned()),
        ("RP_ORIGIN", format!("http://localhost:{port}")),
        ("COOKIE_MODE", "danger-insecure".to_owned()),
        ("HEALTH_CHECK_DELAY_SECS", "0".to_owned()),
        ("BOOTSTRAP_ADMIN_EMAIL", BOOTSTRAP_ACCOUNT_EMAIL.to_owned()),
        ("BOOTSTRAP_ADMIN_PASSWORD_PLAIN", secret(32)),
        ("BOOTSTRAP_API_KEY", bootstrap_api_key()),
        ("BOOTSTRAP_API_KEY_SECRET", api_secret.to_owned()),
        ("SMTP_URL", "host.docker.internal".to_owned()),
        ("SMTP_PORT", sink_port.to_string()),
        ("SMTP_TLS_MODE", "danger-insecure".to_owned()),
        ("SMTP_FROM", "Lys <lys@example.test>".to_owned()),
        // Read by the issuer at boot in every mode (an expect on each) and
        // used in none of the danger-insecure ones: the leg's first run on
        // the Mac ended at "SMTP_USERNAME is not set" without them.
        ("SMTP_USERNAME", "unused".to_owned()),
        ("SMTP_PASSWORD", "unused".to_owned()),
        ("LOG_LEVEL", "info".to_owned()),
    ]
}

/// Read the log until the issuer says its SMTP connection opened (`true`),
/// or says it failed or ends (`false`), with every line read.
async fn smtp_opened(log: &mut UnboundedReceiver<Said>) -> (bool, Vec<String>) {
    let mut said = Vec::new();
    while let Some(line) = log.recv().await {
        match line {
            Said::Line(text) => {
                let opened = text.contains("SMTP connection opened")
                    || text.contains("Successfully connected");
                let failed =
                    text.contains("Could not connect") || text.contains("retries exceeded");
                said.push(text);
                if opened {
                    return (true, said);
                }
                if failed {
                    return (false, said);
                }
            }
            Said::Ended => return (false, said),
        }
    }
    (false, said)
}

/// The sink's next letter, or the issuer's own word that its send failed,
/// or the container ending: whichever comes first, as words for the record.
async fn outcome(sink: &mut MailSink, log: &mut UnboundedReceiver<Said>) -> String {
    loop {
        tokio::select! {
            letter = sink.next() => {
                return match letter {
                    Some(Captured { from, to, body, headers }) => {
                        let subject = headers
                            .iter()
                            .find(|line| line.to_ascii_lowercase().starts_with("subject:"))
                            .cloned()
                            .unwrap_or_default();
                        format!(
                            "a letter arrived: from {from} to {} with {subject}; body: {}",
                            to.join(", "),
                            body.lines().take(12).collect::<Vec<_>>().join(" / ")
                        )
                    }
                    None => "the sink closed".to_owned(),
                };
            }
            said = log.recv() => match said {
                Some(Said::Line(text)) => {
                    if text.contains("Error sending") || text.contains("error sending") {
                        return format!("the issuer's send failed: {text}");
                    }
                }
                Some(Said::Ended) | None => return "the issuer ended before any letter".to_owned(),
            }
        }
    }
}

#[tokio::test]
async fn the_issuer_given_smtp_writes_or_does_not_to_a_newly_made_account()
-> Result<(), Box<dyn Error>> {
    let mut sink = MailSink::start().await?;
    let port = free_port()?;
    let api_secret = Credential::generate("api", Shape::Alphanumeric(64));
    let mut args: Vec<String> = ["run", "-d", "--name", CONTAINER, "-p"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    args.push(format!("127.0.0.1:{port}:8080"));
    for (name, value) in environment(port, sink.port(), api_secret.expose()) {
        args.push("-e".to_owned());
        args.push(format!("{name}={value}"));
    }
    args.push(IMAGE.to_owned());
    docker(&args)?;
    let mut log = follow_log();
    let (connected, said) = smtp_opened(&mut log).await;
    if !connected {
        let removed = remove_container();
        return Err(format!(
            "the issuer did not open its SMTP connection ({removed:?}):\n{}",
            said.join("\n")
        )
        .into());
    }
    let request = serde_json::json!({
        "email": EMAIL,
        "given_name": "Leg",
        "family_name": "Example",
        "language": "en",
        "groups": null,
        "roles": [],
        "user_expires": null,
    });
    let (status, body) = post(
        &format!("http://127.0.0.1:{port}/auth/v1/users"),
        &format!("{API_KEY_NAME}${}", api_secret.expose()),
        &request,
    )
    .await?;
    let found = if (200..300).contains(&status) {
        outcome(&mut sink, &mut log).await
    } else {
        format!("POST /users answered {status}: {body}")
    };
    let removed = remove_container();
    let record = serde_json::json!({
        "image": IMAGE,
        "request": request,
        "post_users_status": status,
        "outcome": found,
        "container_removed": removed.is_ok(),
    });
    let rendered = serde_json::to_string_pretty(&record)?;
    match std::env::var("LEG_RECORD") {
        Ok(path) => std::fs::write(&path, rendered)?,
        Err(_) => println!("{rendered}"),
    }
    Ok(())
}
