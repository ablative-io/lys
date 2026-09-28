//! One JSON request to the `SpiceDB` gateway and its answer, over a plain
//! TCP connection that is closed after the answer.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// How long one connect, write or read may take.
const WAIT: Duration = Duration::from_secs(5);

/// An answer: its status and its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The HTTP status.
    pub status: u16,
    /// The body, with any chunking removed.
    pub body: String,
}

/// Post `body` to `path` at `authority`, a host and port, carrying `bearer`.
/// The error is a reason in words and never holds the bearer.
pub fn post_json(authority: &str, path: &str, bearer: &str, body: &str) -> Result<Answer, String> {
    let unreachable = |error: std::io::Error| format!("{authority} could not be reached: {error}");
    let address = authority
        .to_socket_addrs()
        .map_err(unreachable)?
        .next()
        .ok_or_else(|| format!("{authority} names no address"))?;
    let mut stream = TcpStream::connect_timeout(&address, WAIT).map_err(unreachable)?;
    stream.set_read_timeout(Some(WAIT)).map_err(unreachable)?;
    stream.set_write_timeout(Some(WAIT)).map_err(unreachable)?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {bearer}\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).map_err(unreachable)?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(unreachable)?;
    parse(&raw).ok_or_else(|| format!("{authority} answered what is not HTTP"))
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
