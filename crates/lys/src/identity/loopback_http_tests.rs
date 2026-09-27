use std::error::Error;
use std::io::{Read, Write};
use std::net::{Ipv6Addr, SocketAddr, TcpListener};
use std::time::Duration;

use super::*;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const TIMEOUT: Duration = Duration::from_secs(3);

fn authority(host: &str, port: u16) -> Authority {
    Authority {
        host: host.to_string(),
        port,
    }
}

#[test]
fn a_bracketed_ipv6_host_is_unbracketed() {
    assert_eq!(
        Authority::from_http_url("http://[::1]:8080"),
        Ok(authority("::1", 8080))
    );
    assert_eq!(
        Authority::parse("[::1]:5432", None),
        Ok(authority("::1", 5432))
    );
    assert_eq!(authority("::1", 8080).to_string(), "[::1]:8080");
    assert_eq!(
        Authority::parse("::1:5432", None),
        Err("an IPv6 host is written in brackets")
    );
    assert_eq!(
        Authority::parse("[::1", None),
        Err("an IPv6 host is closed with ]")
    );
}

#[test]
fn a_bracketed_ipv6_host_resolves() -> TestResult {
    let parsed = Authority::from_http_url("http://[::1]:8080")?;
    let addresses: Vec<SocketAddr> = (parsed.host.as_str(), parsed.port)
        .to_socket_addrs()?
        .collect();
    assert_eq!(addresses, [SocketAddr::from((Ipv6Addr::LOCALHOST, 8080))]);
    Ok(())
}

#[test]
fn a_missing_port_is_the_scheme_default_never_a_digit_of_the_host() {
    assert_eq!(
        Authority::from_http_url("http://[::1]"),
        Ok(authority("::1", HTTP_DEFAULT_PORT))
    );
    assert_eq!(
        Authority::from_http_url("http://[::1]/"),
        Ok(authority("::1", HTTP_DEFAULT_PORT))
    );
    assert_eq!(
        Authority::from_http_url("http://localhost"),
        Ok(authority("localhost", HTTP_DEFAULT_PORT))
    );
    assert_eq!(
        Authority::from_http_url("http://127.0.0.1:8443/"),
        Ok(authority("127.0.0.1", 8443))
    );
    assert_eq!(
        Authority::parse("localhost", None),
        Err("expected host:port")
    );
    assert_eq!(
        Authority::from_http_url("http://localhost:"),
        Err("the port is not a number from 1 to 65535")
    );
    assert_eq!(
        Authority::from_http_url("http://localhost:0"),
        Err("the port is not a number from 1 to 65535")
    );
    assert_eq!(
        Authority::from_http_url("https://localhost:8443"),
        Err("expected http://host:port")
    );
}

#[test]
fn every_resolved_address_is_tried_until_one_accepts() -> TestResult {
    let refusing = TcpListener::bind("127.0.0.1:0")?.local_addr()?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let accepting = listener.local_addr()?;
    let stream = connect_any(&[refusing, accepting], TIMEOUT)?;
    assert_eq!(stream.peer_addr()?, accepting);
    let refusal = connect_any(&[refusing], TIMEOUT)
        .err()
        .ok_or("a closed port accepted")?;
    assert!(refusal.starts_with(&refusing.to_string()), "{refusal}");
    Ok(())
}

#[test]
fn a_name_reaches_a_service_published_only_on_ipv4() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let stream = authority("localhost", port).connect(TIMEOUT)?;
    assert_eq!(stream.peer_addr()?, listener.local_addr()?);
    Ok(())
}

#[test]
fn the_status_line_is_parsed_field_by_field() {
    assert_eq!(parse_status_line("HTTP/1.1 200 OK"), Ok(200));
    assert_eq!(
        parse_status_line("HTTP/1.0 503 Service Unavailable"),
        Ok(503)
    );
    assert_eq!(parse_status_line("HTTP/1.1 204"), Ok(204));
    assert_eq!(
        parse_status_line("HTTP/1.1 2000 OK"),
        Err("the status is not three digits")
    );
    assert_eq!(
        parse_status_line("HTTP/1.1 20"),
        Err("the status is cut short")
    );
    assert_eq!(
        parse_status_line("HTTP/2 200"),
        Err("the answer is not HTTP/1.x")
    );
    assert_eq!(
        parse_status_line("HTTP/1.1 999 Odd"),
        Err("the status is outside 100 to 599")
    );
}

#[test]
fn a_response_body_is_read_by_length_or_by_chunk() {
    assert_eq!(
        parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nokextra"),
        Ok(Response {
            status: 200,
            body: b"ok".to_vec()
        })
    );
    assert_eq!(
        parse_response(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nok\r\n0\r\n\r\n"
        ),
        Ok(Response {
            status: 200,
            body: b"ok".to_vec()
        })
    );
    assert_eq!(
        parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nok"),
        Err("the body was cut short")
    );
    assert_eq!(
        parse_response(b"HTTP/1.1 200 OK\r\n"),
        Err("the response was cut short")
    );
}

#[test]
fn an_exchange_sends_the_request_and_reads_the_whole_response() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = std::thread::spawn(move || -> Result<Vec<u8>, std::io::Error> {
        let (mut stream, _) = listener.accept()?;
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.ends_with(b"\r\n\r\n") {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 4\r\n\r\ndown")?;
        Ok(request)
    });
    let request = Request {
        method: "GET",
        path: "/healthz",
        headers: &[("Accept", b"text/plain".as_slice())],
        body: &[],
    };
    let response = exchange(&authority("127.0.0.1", port), TIMEOUT, &request);
    let sent = server
        .join()
        .map_err(|panic| format!("the server panicked: {panic:?}"))??;
    assert_eq!(
        String::from_utf8(sent)?,
        format!(
            "GET /healthz HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nAccept: text/plain\r\nContent-Length: 0\r\n\r\n"
        )
    );
    assert_eq!(
        response,
        Ok(Response {
            status: 503,
            body: b"down".to_vec()
        })
    );
    Ok(())
}
