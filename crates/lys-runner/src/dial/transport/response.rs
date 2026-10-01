//! Reading one bounded HTTP/1.1 response: the status line, headers, and a
//! body framed by length or by chunks, each within its limit.

use std::io::{BufRead, Read};

use super::{BODY_LIMIT, HEADER_LIMIT, Response, failed, token};
use crate::error::RunnerError;

fn too_large() -> RunnerError {
    RunnerError::refused(
        "runner_dial_response_too_large",
        "the response exceeds 1048576 bytes",
    )
}

fn line(stream: &mut impl BufRead, remaining: &mut usize) -> Result<String, RunnerError> {
    let mut bytes = Vec::new();
    stream
        .take(u64::try_from(*remaining).map_err(|error| failed(error.to_string()))? + 1)
        .read_until(b'\n', &mut bytes)
        .map_err(|error| failed(format!("response metadata was not read: {error}")))?;
    if bytes.len() > *remaining {
        return Err(RunnerError::refused(
            "runner_dial_headers_too_large",
            "response metadata exceeds 16384 bytes",
        ));
    }
    *remaining -= bytes.len();
    let bytes = bytes
        .strip_suffix(b"\r\n")
        .ok_or_else(|| failed("response metadata has no complete line"))?;
    String::from_utf8(bytes.to_vec())
        .map_err(|error| failed(format!("response metadata is not UTF-8: {error}")))
}

fn header(value: &str) -> Result<(String, String), RunnerError> {
    let (name, value) = value
        .split_once(':')
        .ok_or_else(|| failed("a response header has no colon"))?;
    if !token(name)
        || value
            .bytes()
            .any(|byte| (byte < b' ' && byte != b'\t') || byte == 127)
    {
        return Err(failed("a response header is invalid"));
    }
    Ok((name.to_owned(), value.trim().to_owned()))
}

fn response_head(
    stream: &mut impl BufRead,
    remaining: &mut usize,
) -> Result<(Response, bool), RunnerError> {
    let first = line(stream, remaining)?;
    let mut parts = first.splitn(3, ' ');
    let version = parts
        .next()
        .ok_or_else(|| failed("the answer has no HTTP version"))?;
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(failed("the answer has an unsupported HTTP version"));
    }
    let code = parts
        .next()
        .ok_or_else(|| failed("the answer has no status"))?;
    let status = code
        .parse::<u16>()
        .map_err(|error| failed(format!("the status is invalid: {error}")))?;
    if code.len() != 3 || !(200..=599).contains(&status) {
        return Err(failed("the answer has an unsupported status"));
    }
    let mut headers = Vec::new();
    loop {
        let value = line(stream, remaining)?;
        if value.is_empty() {
            break;
        }
        headers.push(header(&value)?);
    }
    let response = Response {
        status,
        headers,
        body: Vec::new(),
    };
    let mut reusable = version == "HTTP/1.1";
    for (_, value) in response
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("connection"))
    {
        if value
            .split(',')
            .any(|word| word.trim().eq_ignore_ascii_case("keep-alive"))
        {
            reusable = true;
        }
    }
    if response
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("connection"))
        .any(|(_, value)| {
            value
                .split(',')
                .any(|word| word.trim().eq_ignore_ascii_case("close"))
        })
    {
        reusable = false;
    }
    Ok((response, reusable))
}

fn framing(response: &Response) -> Result<(Option<usize>, bool), RunnerError> {
    let mut length = None;
    let mut chunked = false;
    for (name, value) in &response.headers {
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some()
                || value.is_empty()
                || !value.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err(failed("the content length is repeated or invalid"));
            }
            let given = value
                .parse::<usize>()
                .map_err(|error| failed(format!("the content length is invalid: {error}")))?;
            if given > BODY_LIMIT {
                return Err(too_large());
            }
            length = Some(given);
        }
        if name.eq_ignore_ascii_case("transfer-encoding") {
            if chunked || !value.eq_ignore_ascii_case("chunked") {
                return Err(failed("the transfer encoding is repeated or unsupported"));
            }
            chunked = true;
        }
    }
    if chunked && length.is_some() {
        return Err(failed("the answer has ambiguous body framing"));
    }
    Ok((length, chunked))
}

pub(super) fn read_response(stream: &mut impl BufRead) -> Result<(Response, bool), RunnerError> {
    let mut remaining = HEADER_LIMIT;
    let (mut response, mut reusable) = response_head(stream, &mut remaining)?;
    let (length, chunked) = framing(&response)?;
    response.body = if response.status == 204 || response.status == 304 {
        if chunked || length.is_some_and(|length| length != 0) {
            return Err(failed("a bodyless response declares a body"));
        }
        Vec::new()
    } else if chunked {
        chunks(stream, &mut remaining)?
    } else if let Some(length) = length {
        let mut body = vec![0; length];
        stream
            .read_exact(&mut body)
            .map_err(|error| failed(format!("the response body is incomplete: {error}")))?;
        body
    } else {
        if reusable {
            return Err(failed("a persistent response has no body framing"));
        }
        reusable = false;
        let mut body = Vec::new();
        stream
            .take(1_048_577)
            .read_to_end(&mut body)
            .map_err(|error| failed(format!("the response body was not read: {error}")))?;
        if body.len() > BODY_LIMIT {
            return Err(too_large());
        }
        body
    };
    Ok((response, reusable))
}

fn chunks(stream: &mut impl BufRead, remaining: &mut usize) -> Result<Vec<u8>, RunnerError> {
    let mut body = Vec::new();
    loop {
        let value = line(stream, remaining)?;
        let digits = value.split(';').next().unwrap_or_default();
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(failed("a chunk size is invalid"));
        }
        let size = usize::from_str_radix(digits, 16)
            .map_err(|error| failed(format!("a chunk size is invalid: {error}")))?;
        if size > BODY_LIMIT - body.len() {
            return Err(too_large());
        }
        if size == 0 {
            loop {
                let trailer = line(stream, remaining)?;
                if trailer.is_empty() {
                    return Ok(body);
                }
                let (name, _) = header(&trailer)?;
                if ["content-length", "transfer-encoding", "connection"]
                    .iter()
                    .any(|reserved| name.eq_ignore_ascii_case(reserved))
                {
                    return Err(failed("a trailer overrides response framing"));
                }
            }
        }
        let start = body.len();
        body.resize(start + size, 0);
        stream
            .read_exact(&mut body[start..])
            .map_err(|error| failed(format!("a chunk is incomplete: {error}")))?;
        let mut ending = [0; 2];
        stream
            .read_exact(&mut ending)
            .map_err(|error| failed(format!("a chunk ending is incomplete: {error}")))?;
        if ending != *b"\r\n" {
            return Err(failed("a chunk does not end its line"));
        }
    }
}

#[cfg(test)]
pub(super) fn parse(raw: &[u8]) -> Result<Response, RunnerError> {
    read_response(&mut std::io::Cursor::new(raw)).map(|(response, _)| response)
}
