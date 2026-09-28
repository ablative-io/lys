//! Loopback HTTP for the identity tests: plain requests to Rauthy and
//! `SpiceDB`, and a proxy that loses one response after Rauthy has acted on
//! its request.

use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use super::fixtures::{Deployment, TestResult};

/// Sends one request and returns the status and body.
pub fn request(
    address: &str,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> TestResult<(u16, String)> {
    let (head, rest) = exchange(address, method, path, headers, body)?;
    let status = head.split(' ').nth(1).ok_or("no status")?.parse::<u16>()?;
    let body = if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        dechunk(&rest)?
    } else {
        rest
    };
    Ok((status, body))
}

/// Sends one request with no body and returns the status and the value of
/// every `Set-Cookie` header of the answer, in order.
pub fn cookies_set(address: &str, method: &str, path: &str) -> TestResult<(u16, Vec<String>)> {
    let (head, _rest) = exchange(address, method, path, &[], None)?;
    let status = head.split(' ').nth(1).ok_or("no status")?.parse::<u16>()?;
    let cookies = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _value)| name.eq_ignore_ascii_case("set-cookie"))
        .map(|(_name, value)| value.trim().to_string())
        .collect();
    Ok((status, cookies))
}

/// Sends one request and returns the answer's head and what follows it.
fn exchange(
    address: &str,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> TestResult<(String, String)> {
    let mut stream = TcpStream::connect(address)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut head = format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n");
    for (name, value) in headers {
        write!(head, "{name}: {value}\r\n")?;
    }
    let payload = body.unwrap_or_default();
    if body.is_some() {
        head.push_str("Content-Type: application/json\r\n");
    }
    write!(head, "Content-Length: {}\r\n\r\n", payload.len())?;
    stream.write_all(head.as_bytes())?;
    stream.write_all(payload.as_bytes())?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, rest) = text.split_once("\r\n\r\n").ok_or("no header terminator")?;
    Ok((head.to_string(), rest.to_string()))
}

fn dechunk(mut rest: &str) -> TestResult<String> {
    let mut body = String::new();
    loop {
        let (size, tail) = rest.split_once("\r\n").ok_or("chunk size line")?;
        let size = usize::from_str_radix(size.trim(), 16)?;
        if size == 0 {
            return Ok(body);
        }
        body.push_str(tail.get(..size).ok_or("short chunk")?);
        rest = tail.get(size + 2..).ok_or("chunk terminator")?;
    }
}

/// The header configure's bootstrap API key is presented in.
pub fn api_key(deployment: &Deployment) -> TestResult<String> {
    Ok(format!(
        "API-Key lys_configure${}",
        deployment.secret("rauthy-bootstrap-api-secret")?
    ))
}

/// A GET or POST to Rauthy's admin API with the bootstrap API key, parsed
/// as JSON; refuses any status other than 200.
pub fn rauthy_json(
    deployment: &Deployment,
    method: &str,
    path: &str,
) -> TestResult<serde_json::Value> {
    let key = api_key(deployment)?;
    let (status, body) = request(
        &deployment.rauthy_address(),
        method,
        path,
        &[("Authorization", &key)],
        None,
    )?;
    if status != 200 {
        return Err(format!("{method} {path} answered {status}").into());
    }
    Ok(serde_json::from_str(&body)?)
}

/// A proxy in front of Rauthy that forwards everything, except that the
/// first request whose request line starts with its prefix is forwarded,
/// answered by Rauthy, and then cut off before the answer reaches the caller.
pub struct LossyProxy {
    /// The loopback port the proxy listens on.
    pub port: u16,
    stop: Arc<AtomicBool>,
    handle: JoinHandle<Result<u32, String>>,
}

impl LossyProxy {
    /// Starts the proxy in front of `upstream`.
    pub fn start(upstream: String, prefix: &'static str) -> TestResult<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            serve(&listener, &flag, &upstream, prefix).map_err(|e| e.to_string())
        });
        Ok(Self { port, stop, handle })
    }

    /// Stops the proxy and returns how many answers it lost.
    pub fn finish(self) -> TestResult<u32> {
        self.stop.store(true, Ordering::SeqCst);
        let served = self
            .handle
            .join()
            .map_err(|panic| format!("the proxy thread panicked: {panic:?}"))?;
        Ok(served?)
    }
}

fn serve(
    listener: &TcpListener,
    stop: &AtomicBool,
    upstream: &str,
    prefix: &str,
) -> TestResult<u32> {
    let mut lost = 0;
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((client, _)) => {
                client.set_nonblocking(false)?;
                lost += forward(client, upstream, prefix, lost == 0)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(lost)
}

fn forward(mut client: TcpStream, upstream: &str, prefix: &str, may_lose: bool) -> TestResult<u32> {
    client.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let read = client.read(&mut buffer)?;
        if read == 0 {
            return Ok(0);
        }
        request.extend_from_slice(&buffer[..read]);
        if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
            break end + 4;
        }
    };
    let head = String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
    let length = head
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .map_or(Ok(0), |value| value.trim().parse::<usize>())?;
    while request.len() < header_end + length {
        let read = client.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
    }
    let mut server = TcpStream::connect(upstream)?;
    server.write_all(&request)?;
    let mut response = Vec::new();
    server.read_to_end(&mut response)?;
    if may_lose && request.starts_with(prefix.as_bytes()) {
        client.shutdown(Shutdown::Both)?;
        return Ok(1);
    }
    client.write_all(&response)?;
    Ok(0)
}
