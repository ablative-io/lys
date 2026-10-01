//! Response bounds and connection reuse are observable without clock waits.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, mpsc};

use super::{Channel, Stream, parse, read_response};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn oversized_content_length_is_refused_before_body_allocation() -> TestResult {
    let result = parse(b"HTTP/1.1 200 OK\r\nContent-Length: 1048577\r\n\r\n");
    assert_eq!(
        result.err().ok_or("oversized length accepted")?.name(),
        "runner_dial_response_too_large"
    );
    Ok(())
}

#[test]
fn oversized_chunk_is_refused_before_body_allocation() -> TestResult {
    let result = parse(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n100001\r\n");
    assert_eq!(
        result.err().ok_or("oversized chunk accepted")?.name(),
        "runner_dial_response_too_large"
    );
    Ok(())
}

#[test]
fn oversized_headers_are_refused_by_name() -> TestResult {
    let response = format!(
        "HTTP/1.1 200 OK\r\nX-Large: {}\r\nContent-Length: 0\r\n\r\n",
        "x".repeat(16_384)
    );
    assert_eq!(
        parse(response.as_bytes())
            .err()
            .ok_or("oversized headers accepted")?
            .name(),
        "runner_dial_headers_too_large"
    );
    Ok(())
}

#[test]
fn ambiguous_or_truncated_framing_is_refused() -> TestResult {
    for raw in [
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\nxx".as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nx",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nx\r\n0\r\n",
    ] {
        assert!(
            parse(raw).is_err(),
            "ambiguous or truncated framing accepted"
        );
    }
    Ok(())
}

#[test]
fn sequential_control_requests_reuse_one_connection() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let serving = std::thread::spawn(move || -> Result<usize, String> {
        let mut requests = 0;
        let mut connections = 0;
        while requests < 2 {
            let (stream, _) = listener.accept().map_err(|error| error.to_string())?;
            connections += 1;
            let mut stream = BufReader::new(stream);
            loop {
                let mut close = false;
                loop {
                    let mut line = String::new();
                    if stream
                        .read_line(&mut line)
                        .map_err(|error| error.to_string())?
                        == 0
                    {
                        return Err("request ended before its headers".to_owned());
                    }
                    if line == "\r\n" {
                        break;
                    }
                    if line.eq_ignore_ascii_case("Connection: close\r\n") {
                        close = true;
                    }
                }
                requests += 1;
                let ending = if close || requests == 2 {
                    "Connection: close\r\n"
                } else {
                    ""
                };
                let response = format!("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n{ending}\r\nok");
                stream
                    .get_mut()
                    .write_all(response.as_bytes())
                    .map_err(|error| error.to_string())?;
                stream
                    .get_mut()
                    .flush()
                    .map_err(|error| error.to_string())?;
                if close || requests == 2 {
                    break;
                }
            }
        }
        Ok(connections)
    });
    let channel = Channel::for_server(&format!("http://{address}"), None)?;
    for route in ["/epoch", "/reply"] {
        assert_eq!(channel.send("GET", route, &[], b"")?.body, b"ok");
    }
    assert_eq!(serving.join().map_err(|_| "server panicked")??, 1);
    Ok(())
}

fn request(stream: &mut impl BufRead) -> Result<String, String> {
    let mut first = String::new();
    stream
        .read_line(&mut first)
        .map_err(|error| error.to_string())?;
    if first.is_empty() {
        return Err("request ended".to_owned());
    }
    loop {
        let mut line = String::new();
        if stream
            .read_line(&mut line)
            .map_err(|error| error.to_string())?
            == 0
        {
            return Err("request headers ended".to_owned());
        }
        if line == "\r\n" {
            return Ok(first);
        }
    }
}

#[test]
fn blocked_long_poll_and_control_share_exactly_two_connections() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let (ready, waiting) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let serving = std::thread::spawn(move || -> Result<usize, String> {
        let (stream, _) = listener.accept().map_err(|error| error.to_string())?;
        let poll = std::thread::spawn(move || -> Result<(), String> {
            let mut stream = BufReader::new(stream);
            assert!(request(&mut stream)?.contains("/next"));
            ready.send(()).map_err(|error| error.to_string())?;
            released.recv().map_err(|error| error.to_string())?;
            for round in 0..2 {
                if round == 1 {
                    request(&mut stream)?;
                }
                stream
                    .get_mut()
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        });
        let (stream, _) = listener.accept().map_err(|error| error.to_string())?;
        let mut stream = BufReader::new(stream);
        for _ in 0..2 {
            assert!(request(&mut stream)?.contains("/reply"));
            stream
                .get_mut()
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                .map_err(|error| error.to_string())?;
        }
        release.send(()).map_err(|error| error.to_string())?;
        poll.join().map_err(|_| "poll server panicked")??;
        Ok(2)
    });
    let channel = Arc::new(Channel::for_server(&format!("http://{address}"), None)?);
    let polling = Arc::clone(&channel);
    let client = std::thread::spawn(move || -> Result<(), String> {
        for _ in 0..2 {
            polling
                .send("GET", "/runner/dial/m/next", &[], b"")
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    });
    waiting.recv()?;
    for _ in 0..2 {
        channel.send("GET", "/reply", &[], b"")?;
    }
    client.join().map_err(|_| "poll client panicked")??;
    assert_eq!(serving.join().map_err(|_| "server panicked")??, 2);
    for lane in [&channel.poll, &channel.control] {
        assert!(lane.lock().map_err(|error| error.to_string())?.is_some());
    }
    Ok(())
}

#[test]
fn reused_connections_have_kernel_keepalive_configured() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let channel = Channel::for_server(&format!("http://{}", listener.local_addr()?), None)?;
    let connected = channel.connect()?;
    let Stream::Plain(stream) = connected.get_ref() else {
        return Err("expected loopback stream".into());
    };
    assert!(rustix::net::sockopt::socket_keepalive(stream)?);
    assert_eq!(rustix::net::sockopt::tcp_keepidle(stream)?.as_secs(), 60);
    assert_eq!(rustix::net::sockopt::tcp_keepintvl(stream)?.as_secs(), 10);
    assert_eq!(rustix::net::sockopt::tcp_keepcnt(stream)?, 3);
    Ok(())
}

#[test]
fn chunk_trailers_are_consumed_before_the_next_response() -> TestResult {
    let mut stream = std::io::Cursor::new(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2;extension=yes\r\nhi\r\n0\r\nX-Trailer: kept\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nyes");
    let (first, reusable) = read_response(&mut stream)?;
    assert_eq!(first.body, b"hi");
    assert!(reusable);
    assert_eq!(read_response(&mut stream)?.0.body, b"yes");
    Ok(())
}

#[test]
fn close_delimited_bodies_are_bounded_and_not_reused() -> TestResult {
    let mut raw = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
    raw.resize(raw.len() + 1_048_576, b'x');
    let (response, reusable) = read_response(&mut std::io::Cursor::new(&raw))?;
    assert_eq!(response.body.len(), 1_048_576);
    assert!(!reusable);
    raw.push(b'x');
    assert_eq!(
        parse(&raw)
            .err()
            .ok_or("oversized close-delimited body accepted")?
            .name(),
        "runner_dial_response_too_large"
    );
    assert!(parse(b"HTTP/1.1 200 OK\r\n\r\n").is_err());
    Ok(())
}
