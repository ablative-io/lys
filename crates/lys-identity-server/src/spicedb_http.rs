//! One JSON request to the `SpiceDB` gateway and its answer, over a plain
//! TCP connection that is closed after the answer.
//!
//! The endpoint is a socket address, so no name lookup is waited on. The
//! socket is non-blocking and every wait, the connect, each write and each
//! read, is made in poll(2) given no time bound, over the socket and, inside
//! a request's scope (`spicedb_cancel`), that request's wake pipe. A call ends
//! on `SpiceDB`'s answer, on `SpiceDB` closing the connection, or on its
//! request leaving, and never on a clock.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};

use rustix::event::{PollFd, PollFlags};
use rustix::io::{Errno, FdFlags};
use rustix::net::{AddressFamily, SocketType};

use crate::spicedb_cancel::{Cancel, LEFT, current};

/// An answer: its status and its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The HTTP status.
    pub status: u16,
    /// The body, with any chunking removed.
    pub body: String,
}

/// Why an exchange ended without an answer.
enum Ended {
    /// The request that asked left.
    Left,
    /// The connect, a write or a read failed.
    Failed(io::Error),
}

impl From<io::Error> for Ended {
    fn from(error: io::Error) -> Self {
        Self::Failed(error)
    }
}

/// Post `body` to `path` at `authority`, a socket address, carrying
/// `bearer`. The error is a reason in words and never holds the bearer.
pub fn post_json(authority: &str, path: &str, bearer: &str, body: &str) -> Result<Answer, String> {
    let cancel = current();
    if cancel.as_deref().is_some_and(Cancel::cancelled) {
        return Err(LEFT.to_owned());
    }
    let address: SocketAddr = authority
        .parse()
        .map_err(|error| format!("{authority} is not an address: {error}"))?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {bearer}\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let wake = cancel.as_deref().map(Cancel::wake);
    match exchange(address, request.as_bytes(), wake) {
        Ok(raw) => parse(&raw).ok_or_else(|| format!("{authority} answered what is not HTTP")),
        Err(Ended::Left) => Err(LEFT.to_owned()),
        Err(Ended::Failed(error)) => Err(format!("{authority} could not be reached: {error}")),
    }
}

/// Connect to `address`, write `request` whole and read the answer to its
/// end, every wait in [`wait`]. The socket is closed when this returns.
fn exchange(
    address: SocketAddr,
    request: &[u8],
    wake: Option<BorrowedFd<'_>>,
) -> Result<Vec<u8>, Ended> {
    let mut stream = TcpStream::from(connect(address, wake)?);
    let mut written = 0;
    while written < request.len() {
        match stream.write(&request[written..]) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero).into()),
            Ok(count) => written += count,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                wait(&stream, PollFlags::OUT, wake)?;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
    let mut raw = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => return Ok(raw),
            Ok(count) => raw.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                wait(&stream, PollFlags::IN, wake)?;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
}

/// A non-blocking socket connected to `address`, the connect waited on in
/// [`wait`].
fn connect(address: SocketAddr, wake: Option<BorrowedFd<'_>>) -> Result<OwnedFd, Ended> {
    let family = if address.is_ipv4() {
        AddressFamily::INET
    } else {
        AddressFamily::INET6
    };
    let socket = rustix::net::socket(family, SocketType::STREAM, None).map_err(io::Error::from)?;
    rustix::io::fcntl_setfd(&socket, FdFlags::CLOEXEC).map_err(io::Error::from)?;
    rustix::io::ioctl_fionbio(&socket, true).map_err(io::Error::from)?;
    match rustix::net::connect(&socket, &address) {
        Ok(()) => return Ok(socket),
        Err(Errno::INPROGRESS | Errno::INTR) => {}
        Err(error) => return Err(io::Error::from(error).into()),
    }
    wait(&socket, PollFlags::OUT, wake)?;
    rustix::net::sockopt::socket_error(&socket)
        .map_err(io::Error::from)?
        .map_err(io::Error::from)?;
    Ok(socket)
}

/// Wait in poll(2), given no time bound, until `socket` is ready for `want` or
/// shows an error or a close, or until `wake` becomes readable because the
/// request left. With no `wake` only the socket is waited on.
fn wait(socket: &impl AsFd, want: PollFlags, wake: Option<BorrowedFd<'_>>) -> Result<(), Ended> {
    loop {
        let mut watched = vec![PollFd::new(socket, want)];
        if let Some(wake) = wake {
            watched.push(PollFd::from_borrowed_fd(wake, PollFlags::IN));
        }
        match rustix::event::poll(&mut watched, None) {
            Ok(_) => {}
            Err(Errno::INTR) => continue,
            Err(error) => return Err(io::Error::from(error).into()),
        }
        if watched
            .get(1)
            .is_some_and(|woken| !woken.revents().is_empty())
        {
            return Err(Ended::Left);
        }
        if watched
            .first()
            .is_some_and(|ready| !ready.revents().is_empty())
        {
            return Ok(());
        }
    }
}

/// The answer `raw` holds, or nothing when it is not an HTTP answer.
fn parse(raw: &[u8]) -> Option<Answer> {
    let split = raw.windows(4).position(|window| window == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&raw[..split]).ok()?;
    let rest = &raw[split + 4..];
    let mut lines = head.split("\r\n");
    let status = lines.next()?.split(' ').nth(1)?.parse().ok()?;
    let chunked = lines.any(|line| {
        let line = line.to_ascii_lowercase();
        line.starts_with("transfer-encoding:") && line.contains("chunked")
    });
    let body = if chunked {
        unchunk(rest)?
    } else {
        rest.to_vec()
    };
    Some(Answer {
        status,
        body: String::from_utf8(body).ok()?,
    })
}

/// The bytes the chunks of `rest` hold, in order.
fn unchunk(mut rest: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let end = rest.windows(2).position(|window| window == b"\r\n")?;
        let size = std::str::from_utf8(&rest[..end]).ok()?;
        let size = usize::from_str_radix(size.split(';').next()?.trim(), 16).ok()?;
        rest = &rest[end + 2..];
        if size == 0 {
            return Some(body);
        }
        body.extend_from_slice(rest.get(..size)?);
        rest = rest.get(size + 2..)?;
    }
}

#[cfg(test)]
mod tests {
    use super::{Answer, parse};

    #[test]
    fn a_plain_answer_is_read_whole() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        assert_eq!(
            parse(raw),
            Some(Answer {
                status: 200,
                body: "{}".to_owned()
            })
        );
    }

    #[test]
    fn a_chunked_answer_is_joined() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n{\"a\r\n4\r\n\":1}\r\n0\r\n\r\n";
        assert_eq!(
            parse(raw),
            Some(Answer {
                status: 200,
                body: "{\"a\":1}".to_owned()
            })
        );
    }

    #[test]
    fn a_torn_answer_is_not_an_answer() {
        assert_eq!(
            parse(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nab"),
            None
        );
        assert_eq!(parse(b"nothing"), None);
    }
}
