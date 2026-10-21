//! Lys's one sender (DIRECTORY-110 R3): opened from the install's mail
//! settings, it sends each letter once per act and records every send.
//!
//! A letter is sent after the act it reports is recorded, never for one
//! refused. The record of sends is an append-only file beside the
//! directory's data, one line per letter, written whole and flushed before
//! the send is reported; a letter whose record exists is not sent again, so
//! asking an act again resends only what did not go. A letter the sign-in
//! service sent itself is recorded here too, by its sender, so a repeat of
//! that act writes nothing beside it. The password is read from its file
//! when the sender opens and is never in the configuration, a log, a
//! refusal or a letter.

use std::io::Write as _;
use std::path::PathBuf;

use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use zeroize::Zeroizing;

use crate::error::ServerError;
use crate::mail_letters::Letter;

/// The `mail` section of the service configuration, as the install writes
/// it from deployment.toml's `[mail]`: the server, the sender, how the
/// connection is secured, where the password is kept and where sends are
/// recorded.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MailSettings {
    /// The server's host name or address.
    pub host: String,
    /// Its port.
    pub port: u16,
    /// The user the server is signed in to as, when it takes a sign-in.
    #[serde(default)]
    pub user: Option<String>,
    /// The owner-only file holding that user's password.
    #[serde(default)]
    pub password_file: Option<PathBuf>,
    /// The sender every letter carries, as `Lys <lys@example.test>`.
    pub from: String,
    /// How the connection is secured.
    pub tls: Tls,
    /// The append-only record of every letter sent.
    pub sent_file: PathBuf,
}

/// How the mail server's connection is secured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tls {
    /// TLS from the first byte.
    Implicit,
    /// Upgraded after the greeting.
    Starttls,
    /// Plain; the install accepts it for a loopback host alone.
    None,
}

/// Who sent a letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Sender {
    /// Lys, through its own sender.
    Lys,
    /// The sign-in service, whose own letter on a new account is the added letter.
    Issuer,
}

/// What an answer says of the letter its act owed.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct LetterView {
    /// The letter: `added`, `access_given` or `founded`.
    pub letter: String,
    /// The address it went to, when one was known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// Whether it went.
    pub sent: bool,
    /// Who sent it.
    pub by: Sender,
    /// Why it did not go, when it did not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The link the letter would have carried, handed here because the
    /// install has no mail settings (DIRECTORY-112 R1, R3); absent whenever
    /// the install has mail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handed: Option<String>,
}

/// Why a handed link is in the answer and not in a letter.
pub const HANDED_REASON: &str = "Lys has no mail settings, so the link is handed here";

impl LetterView {
    /// A letter sent, or found already sent, for this act, by `by`.
    #[must_use]
    pub fn sent(letter: &Letter, by: Sender) -> Self {
        Self {
            letter: letter.name().to_owned(),
            to: Some(letter.to().to_owned()),
            sent: true,
            by,
            reason: None,
            handed: None,
        }
    }

    /// A letter that did not go, by name, with why.
    #[must_use]
    pub fn unsent(letter: &'static str, to: Option<String>, reason: impl Into<String>) -> Self {
        Self {
            letter: letter.to_owned(),
            to,
            sent: false,
            by: Sender::Lys,
            reason: Some(reason.into()),
            handed: None,
        }
    }

    /// A letter that did not go because the install has no mail settings,
    /// its link handed in the answer instead (DIRECTORY-112 R1, R3).
    #[must_use]
    pub fn handed(letter: &'static str, to: String, link: String) -> Self {
        Self {
            handed: Some(link),
            ..Self::unsent(letter, Some(to), HANDED_REASON)
        }
    }
}

/// One line of the sent record.
#[derive(Debug, Serialize, Deserialize)]
struct Sent {
    operation: String,
    letter: String,
    to: String,
    by: Sender,
    at: u64,
}

/// The sender.
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    sent_file: PathBuf,
    /// One record read and append at a time, so two acts never interleave.
    record: Mutex<()>,
}

impl std::fmt::Debug for Mailer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mailer")
            .field("from", &self.from.to_string())
            .field("sent_file", &self.sent_file)
            .finish_non_exhaustive()
    }
}

fn invalid(reason: impl Into<String>) -> ServerError {
    ServerError::ConfigInvalid {
        reason: reason.into(),
    }
}

impl Mailer {
    /// Open the sender from `settings`, reading the password from its file.
    ///
    /// # Errors
    /// Refuses a sender that is not an address, a host that cannot be
    /// dialled, a user with no password file, or a password file that
    /// cannot be read; the password itself is never in the refusal.
    pub fn open(settings: &MailSettings) -> Result<Self, ServerError> {
        let from: Mailbox = settings
            .from
            .parse()
            .map_err(|error| invalid(format!("mail.from is not an address: {error}")))?;
        let builder = match settings.tls {
            Tls::Implicit => AsyncSmtpTransport::<Tokio1Executor>::relay(&settings.host),
            Tls::Starttls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.host),
            Tls::None => Ok(AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(
                &settings.host,
            )),
        }
        .map_err(|error| {
            invalid(format!(
                "the mail server {} cannot be dialled: {error}",
                settings.host
            ))
        })?;
        let mut builder = builder.port(settings.port);
        if let Some(user) = &settings.user {
            let file = settings
                .password_file
                .as_ref()
                .ok_or_else(|| invalid("mail.user is set with no password_file"))?;
            let password = Zeroizing::new(std::fs::read_to_string(file).map_err(|error| {
                invalid(format!(
                    "the mail password at {} cannot be read: {error}",
                    file.display()
                ))
            })?);
            builder = builder.credentials(Credentials::new(
                user.clone(),
                password.trim_end().to_owned(),
            ));
        }
        Ok(Self {
            transport: builder.build(),
            from,
            sent_file: settings.sent_file.clone(),
            record: Mutex::new(()),
        })
    }

    /// Send `letter` for `operation` unless the record says it already went.
    ///
    /// # Errors
    /// Answers `MailNotSent` with the server's reply when the letter does
    /// not go, leaving the act it reports recorded; a record that cannot be
    /// read or written is named as the directory being unavailable.
    pub async fn send_once(
        &self,
        operation: &str,
        letter: &Letter,
    ) -> Result<LetterView, ServerError> {
        let _one = self.record.lock().await;
        if let Some(by) = self.recorded(operation, letter.name())? {
            return Ok(LetterView::sent(letter, by));
        }
        let not_sent = |reply: String| ServerError::MailNotSent {
            letter: letter.name().to_owned(),
            to: letter.to().to_owned(),
            reply,
        };
        let to = letter.mailbox().map_err(not_sent)?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(letter.subject())
            .header(ContentType::TEXT_PLAIN)
            .body(letter.body())
            .map_err(|error| not_sent(format!("the letter could not be built: {error}")))?;
        self.transport
            .send(message)
            .await
            .map_err(|error| not_sent(error.to_string()))?;
        self.append(operation, letter, Sender::Lys)?;
        Ok(LetterView::sent(letter, Sender::Lys))
    }

    /// Record that the sign-in service sent `letter` itself for
    /// `operation`, so a repeat of the act writes nothing beside it.
    ///
    /// # Errors
    /// A record that cannot be read or written is named as the directory
    /// being unavailable.
    pub async fn note_sent_by_issuer(
        &self,
        operation: &str,
        letter: &Letter,
    ) -> Result<LetterView, ServerError> {
        let _one = self.record.lock().await;
        if let Some(by) = self.recorded(operation, letter.name())? {
            return Ok(LetterView::sent(letter, by));
        }
        self.append(operation, letter, Sender::Issuer)?;
        Ok(LetterView::sent(letter, Sender::Issuer))
    }

    /// Who the record says sent the letter named `letter` for `operation`,
    /// if anyone: a founding asked again reads this before it mints a new
    /// proving link (DIRECTORY-111 R1).
    ///
    /// # Errors
    /// A record that cannot be read is named as the directory being
    /// unavailable.
    pub async fn recorded_by(
        &self,
        operation: &str,
        letter: &str,
    ) -> Result<Option<Sender>, ServerError> {
        let _one = self.record.lock().await;
        self.recorded(operation, letter)
    }

    /// Who the record says sent `letter` for `operation`, if anyone.
    fn recorded(&self, operation: &str, letter: &str) -> Result<Option<Sender>, ServerError> {
        let text = match std::fs::read_to_string(&self.sent_file) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(record_unavailable(&self.sent_file, &error)),
        };
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let sent: Sent = serde_json::from_str(line)
                .map_err(|error| record_unavailable(&self.sent_file, &error))?;
            if sent.operation == operation && sent.letter == letter {
                return Ok(Some(sent.by));
            }
        }
        Ok(None)
    }

    /// Record the send: one line, written whole and flushed.
    fn append(&self, operation: &str, letter: &Letter, by: Sender) -> Result<(), ServerError> {
        let sent = Sent {
            operation: operation.to_owned(),
            letter: letter.name().to_owned(),
            to: letter.to().to_owned(),
            by,
            at: crate::session::now(),
        };
        let mut line = serde_json::to_vec(&sent)
            .map_err(|error| record_unavailable(&self.sent_file, &error))?;
        line.push(b'\n');
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.sent_file)
            .map_err(|error| record_unavailable(&self.sent_file, &error))?;
        file.write_all(&line)
            .and_then(|()| file.sync_all())
            .map_err(|error| record_unavailable(&self.sent_file, &error))
    }
}

fn record_unavailable(file: &std::path::Path, error: &dyn std::fmt::Display) -> ServerError {
    ServerError::DirectoryUnavailable {
        reason: format!(
            "the sent-letters record at {} cannot be used: {error}",
            file.display()
        ),
    }
}
