//! Explicit MCP requests signed by the configured agent's broker lease.

use std::io::{self, Write};

use serde::Serialize;
use serde_json::Value;

use super::agent::{AgentClient, AgentLease, AgentResponse};
use crate::error::RunnerError;

const BODY_LIMIT: usize = 2 * 1024 * 1024;
const HEADERS: [(&str, &str); 3] = [
    ("Content-Type", "application/json"),
    ("Accept", "application/json, text/event-stream"),
    ("MCP-Protocol-Version", "2025-11-25"),
];

/// A tool's declared name and the exact local HTTP act it represents.
pub struct ToolCall<'a> {
    /// The name returned by the service's tool catalogue.
    pub name: &'a str,
    /// GET, POST, PUT, PATCH or DELETE, matching that tool.
    pub method: &'a str,
    /// The local route, including its query, matching that tool.
    pub path: &'a str,
    /// The route's JSON body, when required by its schema.
    pub body: Option<&'a Value>,
}

/// A stateless JSON MCP facade over an explicitly configured signed client.
///
/// No request acquires ambient credentials or retries a failed exchange.
pub struct McpClient {
    agent: AgentClient,
}

#[derive(Serialize)]
struct Request<P> {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<P>,
}

#[derive(Serialize)]
struct Call<'a> {
    name: &'a str,
    arguments: Arguments<'a>,
}

#[derive(Serialize)]
struct Arguments<'a> {
    method: &'a str,
    path: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a Value>,
}

struct Body(Vec<u8>);

impl Write for Body {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > BODY_LIMIT - self.0.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "MCP request exceeds 2097152 bytes",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn invalid(reason: impl Into<String>) -> RunnerError {
    RunnerError::refused("McpRequestInvalid", reason)
}

fn headers<'a>(extra: &[(&'a str, &'a str)]) -> Result<Vec<(&'a str, &'a str)>, RunnerError> {
    if extra
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("cookie"))
    {
        return Err(RunnerError::refused(
            "AgentCookieRefused",
            "a signed agent request cannot carry a cookie",
        ));
    }
    if extra.iter().any(|(name, _)| {
        HEADERS
            .iter()
            .any(|(reserved, _)| name.eq_ignore_ascii_case(reserved))
    }) {
        return Err(invalid("a header replaces MCP transport metadata"));
    }
    let mut headers = Vec::with_capacity(HEADERS.len() + extra.len());
    headers.extend_from_slice(&HEADERS);
    headers.extend_from_slice(extra);
    Ok(headers)
}

impl McpClient {
    /// Use this client's configured server, broker and TLS authority.
    #[must_use]
    pub fn new(agent: AgentClient) -> Self {
        Self { agent }
    }

    /// Sign and POST a catalogue request with the caller's correlation id.
    ///
    /// The bounded HTTP status and exact JSON response, including any refusal,
    /// are returned unchanged for the caller to interpret.
    ///
    /// # Errors
    /// Returns malformed input, mixed credentials, proof, broker and transport errors by name.
    pub fn tools_list(
        &self,
        lease: &AgentLease<'_>,
        id: u64,
        extra: &[(&str, &str)],
    ) -> Result<AgentResponse, RunnerError> {
        self.send(
            lease,
            &Request::<Call<'_>> {
                jsonrpc: "2.0",
                id,
                method: "tools/list",
                params: None,
            },
            extra,
        )
    }

    /// Sign and POST this tool call without copying its JSON argument tree.
    ///
    /// The service decides whether the name, route, schema and authority agree.
    /// Its bounded response and refusal status are returned unchanged.
    ///
    /// # Errors
    /// Returns invalid local acts, mixed credentials, proof, broker and transport errors by name.
    pub fn tools_call(
        &self,
        lease: &AgentLease<'_>,
        id: u64,
        call: &ToolCall<'_>,
        extra: &[(&str, &str)],
    ) -> Result<AgentResponse, RunnerError> {
        if call.name.is_empty()
            || call.name.len() > 128
            || !call
                .name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
            || !["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&call.method)
            || !call.path.starts_with('/')
            || call.path.starts_with("//")
            || !call.path.is_ascii()
            || call.path.contains('#')
            || call.path.bytes().any(|byte| byte <= b' ' || byte == 127)
        {
            return Err(invalid("the tool name, method or local path is invalid"));
        }
        self.send(
            lease,
            &Request {
                jsonrpc: "2.0",
                id,
                method: "tools/call",
                params: Some(Call {
                    name: call.name,
                    arguments: Arguments {
                        method: call.method,
                        path: call.path,
                        body: call.body,
                    },
                }),
            },
            extra,
        )
    }

    fn send<P: Serialize>(
        &self,
        lease: &AgentLease<'_>,
        request: &Request<P>,
        extra: &[(&str, &str)],
    ) -> Result<AgentResponse, RunnerError> {
        let headers = headers(extra)?;
        let mut body = Body(Vec::new());
        serde_json::to_writer(&mut body, request).map_err(|error| invalid(error.to_string()))?;
        // The bytes hashed for signing are the same buffer written to the service.
        self.agent.send(lease, "POST", "/mcp", &headers, &body.0)
    }
}
