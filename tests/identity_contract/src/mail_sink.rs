//! A capturing SMTP sink: the mail server every letter test points Lys and
//! the issuer at (DIRECTORY-110). It speaks enough SMTP to take a letter
//! whole, keeps each one with its envelope and decoded text, and can be told
//! to refuse the next letters at the end of their data, as a server that
//! will not take a message does. No letter ever leaves the test.
//!
//! The accept loop runs on the test's runtime until the runtime ends, so a
//! fixture that drops its handle keeps a working server; the handle is what
//! reads the letters.

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use lys_identity_server::mail::{MailSettings, Tls};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// One letter the sink took: its envelope and its text, headers and all,
/// with a quoted-printable body decoded.
#[derive(Debug, Clone)]
pub struct Captured {
    /// The envelope sender.
    pub from: String,
    /// The envelope recipients, in the order given.
    pub to: Vec<String>,
    /// The headers, one per entry, as sent.
    pub headers: Vec<String>,
    /// The body, decoded.
    pub body: String,
}

impl Captured {
    /// The value of header `name`, case-insensitively, if the letter carries it.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find_map(|line| {
            let (found, value) = line.split_once(':')?;
            found
                .eq_ignore_ascii_case(name)
                .then(|| value.trim())
        })
    }

    /// The subject.
    #[must_use]
    pub fn subject(&self) -> &str {
        self.header("Subject").unwrap_or("")
    }
}

/// The sink's handle: its port, the letters it took, and its refusing switch.
pub struct MailSink {
    port: u16,
    letters: mpsc::UnboundedReceiver<Captured>,
    refusing: Arc<AtomicBool>,
}

impl MailSink {
    /// Start a sink on a loopback port of its own.
    ///
    /// # Errors
    /// Fails when no loopback port can be bound.
    pub async fn start() -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let (sender, letters) = mpsc::unbounded_channel();
        let refusing = Arc::new(AtomicBool::new(false));
        let switch = Arc::clone(&refusing);
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                tokio::spawn(serve(stream, sender.clone(), Arc::clone(&switch)));
            }
        });
        Ok(Self {
            port,
            letters,
            refusing,
        })
    }

    /// The port the sink listens on.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The service's mail settings for this sink: plain, unsigned, from Lys,
    /// recording sends at `sent_file`.
    #[must_use]
    pub fn settings(&self, sent_file: PathBuf) -> MailSettings {
        MailSettings {
            host: "127.0.0.1".to_owned(),
            port: self.port,
            user: None,
            password_file: None,
            from: "Lys <lys@example.test>".to_owned(),
            tls: Tls::None,
            sent_file,
        }
    }

    /// Whether the sink refuses every letter at the end of its data from
    /// now on, as a server that will not take a message does.
    pub fn refuse(&self, on: bool) {
        self.refusing.store(on, Ordering::SeqCst);
    }

    /// The next letter taken, awaited.
    pub async fn next(&mut self) -> Option<Captured> {
        self.letters.recv().await
    }

    /// A letter already taken and not yet read, without waiting.
    pub fn taken(&mut self) -> Option<Captured> {
        self.letters.try_recv().ok()
    }
}

/// One connection's conversation, from greeting to quit.
async fn serve(stream: TcpStream, sender: mpsc::UnboundedSender<Captured>, refusing: Arc<AtomicBool>) {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    if write.write_all(b"220 sink ESMTP\r\n").await.is_err() {
        return;
    }
    let mut from = String::new();
    let mut to = Vec::new();
    while let Ok(Some(line)) = lines.next_line().await {
        let verb = line.split(' ').next().unwrap_or("").to_ascii_uppercase();
        let answer: &[u8] = match verb.as_str() {
            "EHLO" | "HELO" => b"250-sink\r\n250 OK\r\n",
            "MAIL" => {
                from = address_of(&line);
                b"250 OK\r\n"
            }
            "RCPT" => {
                to.push(address_of(&line));
                b"250 OK\r\n"
            }
            "DATA" => {
                if write
                    .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")
                    .await
                    .is_err()
                {
                    return;
                }
                let mut data = Vec::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    if line == "." {
                        break;
                    }
                    data.push(line.strip_prefix('.').map_or(line.clone(), str::to_owned));
                }
                if refusing.load(Ordering::SeqCst) {
                    b"554 5.7.1 the sink refuses this letter\r\n"
                } else {
                    let taken = captured(&from, &to, &data);
                    from.clear();
                    to.clear();
                    // A test that dropped its handle reads no more letters;
                    // the letter is still taken, as a server would.
                    let _delivered = sender.send(taken);
                    b"250 OK queued\r\n"
                }
            }
            "RSET" | "NOOP" => b"250 OK\r\n",
            "QUIT" => {
                // The conversation is over either way; a peer gone before
                // the farewell is not a failure of the sink.
                let _farewell = write.write_all(b"221 Bye\r\n").await;
                return;
            }
            _ => b"500 unknown\r\n",
        };
        if write.write_all(answer).await.is_err() {
            return;
        }
    }
}

/// The address between the angle brackets of a MAIL FROM or RCPT TO line.
fn address_of(line: &str) -> String {
    line.split_once('<')
        .and_then(|(_, rest)| rest.split_once('>'))
        .map_or_else(String::new, |(address, _)| address.to_owned())
}

/// The letter from its data lines: headers to the first empty line, then the
/// body, decoded when the headers say quoted-printable.
fn captured(from: &str, to: &[String], data: &[String]) -> Captured {
    let split = data.iter().position(String::is_empty).unwrap_or(data.len());
    let headers = unfolded(&data[..split]);
    let body_lines = data.get(split + 1..).unwrap_or(&[]);
    let quoted = headers.iter().any(|line| {
        line.to_ascii_lowercase()
            .starts_with("content-transfer-encoding: quoted-printable")
    });
    let body = if quoted {
        decoded(body_lines)
    } else {
        body_lines.join("\n")
    };
    Captured {
        from: from.to_owned(),
        to: to.to_vec(),
        headers,
        body,
    }
}

/// Header lines with their folded continuations joined back.
fn unfolded(lines: &[String]) -> Vec<String> {
    let mut headers: Vec<String> = Vec::new();
    for line in lines {
        if line.starts_with([' ', '\t'])
            && let Some(last) = headers.last_mut()
        {
            last.push(' ');
            last.push_str(line.trim_start());
        } else {
            headers.push(line.clone());
        }
    }
    headers
}

/// Quoted-printable undone: soft breaks joined, `=XX` bytes restored.
fn decoded(lines: &[String]) -> String {
    let mut bytes = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let (text, soft) = line
            .strip_suffix('=')
            .map_or((line.as_str(), false), |text| (text, true));
        let mut rest = text.as_bytes();
        while let Some(at) = rest.iter().position(|byte| *byte == b'=') {
            bytes.extend_from_slice(&rest[..at]);
            let hex = rest.get(at + 1..at + 3).and_then(|pair| std::str::from_utf8(pair).ok());
            match hex.and_then(|pair| u8::from_str_radix(pair, 16).ok()) {
                Some(byte) => {
                    bytes.push(byte);
                    rest = &rest[at + 3..];
                }
                None => {
                    bytes.push(b'=');
                    rest = &rest[at + 1..];
                }
            }
        }
        bytes.extend_from_slice(rest);
        if !soft && index + 1 < lines.len() {
            bytes.push(b'\n');
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
