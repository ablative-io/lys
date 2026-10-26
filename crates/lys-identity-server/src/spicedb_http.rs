//! One JSON request to the `SpiceDB` gateway and its answer, over a plain
//! kept HTTP/1.1 client. Bytes, a close or a kernel error end each read.
//! Socket ownership belongs to the caller; no pool lock is held over I/O.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

/// An answer: its status and its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The HTTP status.
    pub status: u16,
    /// The body, with any chunking removed.
    pub body: String,
}

/// Kept HTTP connections checked out for one call, without holding a pool lock over I/O.
#[derive(Debug, Default)]
pub struct Client {
    idle: std::sync::Mutex<Vec<(String, std::io::BufReader<TcpStream>)>>,
}

impl Client {
    /// Post one request, reusing a connection and reconnecting a closed read connection once.
    ///
    /// `hold` receives each fresh, checked-out or replacement socket before any
    /// request bytes are written. Its returned release callback runs after that
    /// exchange, before pooling the socket or opening a replacement. Refusing
    /// either callback ends the call without reconnecting or pooling its socket.
    /// The caller configures socket policy and owns any cloned close handle.
    ///
    /// # Errors
    /// Names an unavailable peer, malformed answer, unavailable pool or a caller hook's refusal.
    pub fn post_json<Release>(
        &self,
        authority: &str,
        path: &str,
        bearer: &str,
        body: &str,
        hold: impl Fn(&TcpStream) -> Result<Release, String>,
    ) -> Result<Answer, String>
    where
        Release: FnOnce() -> Result<(), String>,
    {
        if path
            .bytes()
            .chain(bearer.bytes())
            .any(|byte| byte.is_ascii_control())
        {
            return Err(format!(
                "{authority} request headers contain a control character"
            ));
        }
        let mut pooled = self
            .idle
            .lock()
            .map_err(|error| format!("{authority} connection pool is poisoned: {error}"))?;
        let held = pooled
            .iter()
            .position(|(host, _)| host == authority)
            .map(|n| pooled.swap_remove(n).1);
        drop(pooled);
        let reused = held.is_some();
        let mut stream = match held {
            Some(stream) => stream,
            None => connect(authority)?,
        };
        let release = hold(stream.get_ref())?;
        let mut result = exchange(&mut stream, authority, path, bearer, body);
        release()?;
        if reused
            && result.as_ref().is_err_and(|error| error.reconnect)
            && !path.ends_with("/write")
            && !path.ends_with("/delete")
        {
            stream = connect(authority)?;
            let release = hold(stream.get_ref())?;
            result = exchange(&mut stream, authority, path, bearer, body);
            release()?;
        }
        let (answer, keep) = result.map_err(|error| error.words)?;
        if keep {
            self.idle
                .lock()
                .map_err(|error| format!("{authority} connection pool is poisoned: {error}"))?
                .push((authority.to_owned(), stream));
        }
        Ok(answer)
    }
}

fn connect(authority: &str) -> Result<std::io::BufReader<TcpStream>, String> {
    let unreachable = |error: std::io::Error| format!("{authority} could not be reached: {error}");
    let address = authority
        .to_socket_addrs()
        .map_err(unreachable)?
        .next()
        .ok_or_else(|| format!("{authority} names no address"))?;
    let stream = TcpStream::connect(address).map_err(unreachable)?;
    stream.set_nodelay(true).map_err(unreachable)?;
    Ok(std::io::BufReader::new(stream))
}

struct Failure {
    words: String,
    reconnect: bool,
}

fn exchange(
    stream: &mut std::io::BufReader<TcpStream>,
    authority: &str,
    path: &str,
    bearer: &str,
    body: &str,
) -> Result<(Answer, bool), Failure> {
    use std::io::BufRead;
    let io = |error: std::io::Error| Failure {
        reconnect: matches!(
            error.kind(),
            std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::NotConnected
        ),
        words: format!("{authority} could not be reached: {error}"),
    };
    let malformed = || Failure {
        words: format!("{authority} answered what is not HTTP"),
        reconnect: false,
    };
    let malformed_by = |error: &dyn std::fmt::Display| Failure {
        words: format!("{authority} answered what is not HTTP: {error}"),
        reconnect: false,
    };
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {bearer}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{body}",
        body.len()
    );
    stream.get_mut().write_all(request.as_bytes()).map_err(io)?;
    let mut raw = Vec::new();
    let mut length = None;
    let mut chunked = false;
    let mut close = false;
    loop {
        let mut line = Vec::new();
        if stream.read_until(b'\n', &mut line).map_err(io)? == 0 {
            return Err(io(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)));
        }
        if raw.len() + line.len() > 65_536 {
            return Err(malformed());
        }
        let text = std::str::from_utf8(&line)
            .map_err(|error| malformed_by(&error))?
            .trim()
            .to_ascii_lowercase();
        if let Some(value) = text.strip_prefix("content-length:") {
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|error| malformed_by(&error))?,
            );
        }
        if let Some(value) = text.strip_prefix("transfer-encoding:") {
            chunked = value.trim() == "chunked";
        }
        if let Some(value) = text.strip_prefix("connection:") {
            close = value.split(',').any(|v| v.trim() == "close");
        }
        raw.extend_from_slice(&line);
        if line == b"\r\n" {
            break;
        }
    }
    if !raw.starts_with(b"HTTP/1.") {
        return Err(malformed());
    }
    if chunked {
        read_chunks(stream, &mut raw, &io, &malformed, &malformed_by)?;
    } else if let Some(length) = length {
        if length > 64 * 1024 * 1024 {
            return Err(malformed());
        }
        let before = raw.len();
        raw.resize(before + length, 0);
        stream.read_exact(&mut raw[before..]).map_err(io)?;
    } else if close {
        stream.read_to_end(&mut raw).map_err(io)?;
    } else {
        return Err(malformed());
    }
    let answer = parse(&raw).ok_or_else(malformed)?;
    Ok((answer, !close))
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

#[cfg(test)]
#[path = "spicedb_http_kept_tests.rs"]
mod kept_tests;

fn read_chunks(
    stream: &mut std::io::BufReader<TcpStream>,
    raw: &mut Vec<u8>,
    io: &impl Fn(std::io::Error) -> Failure,
    malformed: &impl Fn() -> Failure,
    malformed_by: &impl Fn(&dyn std::fmt::Display) -> Failure,
) -> Result<(), Failure> {
    use std::io::BufRead;
    loop {
        let mut line = Vec::new();
        if stream.read_until(b'\n', &mut line).map_err(io)? == 0 {
            return Err(malformed());
        }
        let size = std::str::from_utf8(&line)
            .map_err(|error| malformed_by(&error))?
            .trim()
            .split(';')
            .next()
            .ok_or_else(malformed)?;
        let size = usize::from_str_radix(size, 16).map_err(|error| malformed_by(&error))?;
        raw.extend_from_slice(&line);
        if size == 0 {
            loop {
                let mut trailer = Vec::new();
                if stream.read_until(b'\n', &mut trailer).map_err(io)? == 0 {
                    return Err(malformed());
                }
                raw.extend_from_slice(&trailer);
                if trailer == b"\r\n" {
                    break;
                }
            }
            break;
        }
        let before = raw.len();
        let after = before
            .checked_add(size)
            .and_then(|n| n.checked_add(2))
            .ok_or_else(malformed)?;
        if after > 64 * 1024 * 1024 {
            return Err(malformed());
        }
        raw.resize(after, 0);
        stream.read_exact(&mut raw[before..]).map_err(io)?;
        if !raw.ends_with(b"\r\n") {
            return Err(malformed());
        }
    }
    Ok(())
}
