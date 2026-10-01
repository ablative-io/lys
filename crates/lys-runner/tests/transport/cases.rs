//! Response bounds and connection reuse are observable without clock waits.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use super::{Channel, parse};

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
