//! The `lys-home` command line: import, render, fewshot, ingest-call,
//! resume-check. Every command prints one JSON report of paths, hashes and
//! counts, never transcript, block or body content. A missing required
//! argument is refused by clap with exit code 2, naming the argument.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::AUTHORED;
use crate::harness::claude_code::import::import_claude_code;
use crate::harness::claude_code::render::{RenderTarget, render_claude_code};
use crate::record::call::{Api, CallMeta, ingest_call_files};
use crate::record::{Home, fresh_id, now};

/// The home: sessions held under an identity, in Pi's session tree.
#[derive(Debug, Parser)]
#[command(name = "lys-home", version, about)]
pub struct Cli {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// The commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Import a Claude Code JSONL into a session of the home.
    Import {
        /// The home directory.
        #[arg(long)]
        home: PathBuf,
        /// The Claude Code transcript file.
        #[arg(long = "claude-code")]
        claude_code: PathBuf,
        /// The session id to create in the home.
        #[arg(long)]
        session: String,
    },
    /// Render a session of the home as a Claude Code JSONL.
    Render {
        /// The home directory.
        #[arg(long)]
        home: PathBuf,
        /// The session to render.
        #[arg(long)]
        session: String,
        /// The session id the rendered file carries.
        #[arg(long)]
        uuid: String,
        /// The working directory the records name.
        #[arg(long)]
        cwd: String,
        /// The model the file is rendered for.
        #[arg(long)]
        model: String,
        /// Where to write; default is Claude Code's own place for the cwd.
        #[arg(long)]
        out: Option<PathBuf>,
        /// The Claude Code version to write into records.
        #[arg(long, default_value = "2.1.281")]
        version: String,
    },
    /// Write a hand-authored few-shot Claude Code JSONL from a turns file.
    Fewshot {
        /// The file to write.
        #[arg(long)]
        out: PathBuf,
        /// The turns: one per line, `user: text` or `assistant: text`.
        #[arg(long)]
        turns: PathBuf,
        /// The working directory the records name.
        #[arg(long, default_value = "/authored")]
        cwd: String,
    },
    /// Record a captured call (request and response files) in a session.
    IngestCall {
        /// The home directory.
        #[arg(long)]
        home: PathBuf,
        /// The session to append to.
        #[arg(long)]
        session: String,
        /// The proxy's stable call id.
        #[arg(long = "call-id")]
        call_id: String,
        /// The provider.
        #[arg(long)]
        provider: String,
        /// The api: anthropic-messages, openai-chat-completions, openai-responses.
        #[arg(long)]
        api: String,
        /// The model.
        #[arg(long)]
        model: String,
        /// The request body file.
        #[arg(long)]
        request: PathBuf,
        /// The response body file.
        #[arg(long)]
        response: PathBuf,
        /// A JSON file holding the parts the proxy assembled from a stream.
        #[arg(long)]
        parts: Option<PathBuf>,
        /// When the call started, RFC 3339.
        #[arg(long = "started-at")]
        started_at: Option<String>,
        /// How long it took.
        #[arg(long = "duration-ms", default_value_t = 0)]
        duration_ms: u64,
    },
    /// Count tool actions in a forked file that repeat ones in a rendered file.
    ResumeCheck {
        /// The rendered file that was resumed.
        rendered: PathBuf,
        /// The file the resume wrote.
        forked: PathBuf,
    },
}

/// Run a command and return its report; a non-zero exit is the caller's to
/// decide from the error.
pub fn run(cli: Cli) -> Result<Value, HomeError> {
    match cli.command {
        Command::Import {
            home,
            claude_code,
            session,
        } => {
            let home = Home::open(home)?;
            let blocks = home.blocks()?;
            let mut s = home.create_session(&session, "", None)?;
            let report = import_claude_code(&claude_code, &mut s, &blocks)?;
            Ok(json!({"command": "import", "session": session, "file": s.file(), "report": report}))
        }
        Command::Render {
            home,
            session,
            uuid,
            cwd,
            model,
            out,
            version,
        } => {
            let home = Home::open(home)?;
            let s = home.open_session(&session)?;
            let user_home =
                std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from);
            let target = RenderTarget {
                session_id: uuid,
                cwd,
                model,
                version,
                out,
            };
            let report = render_claude_code(&s, &target, &user_home)?;
            Ok(json!({"command": "render", "report": report}))
        }
        Command::Fewshot { out, turns, cwd } => {
            let written = fewshot(&out, &turns, &cwd)?;
            println!("claude --resume {}", out.display());
            Ok(json!({"command": "fewshot", "file": out, "records": written, "authored": true}))
        }
        Command::IngestCall {
            home,
            session,
            call_id,
            provider,
            api,
            model,
            request,
            response,
            parts,
            started_at,
            duration_ms,
        } => {
            let api = match api.as_str() {
                "anthropic-messages" => Api::Messages,
                "openai-chat-completions" => Api::ChatCompletions,
                "openai-responses" => Api::Responses,
                _ => {
                    return Err(HomeError::BodyShape {
                        api: "cli",
                        reason: "api must be anthropic-messages, openai-chat-completions or openai-responses",
                    });
                }
            };
            let parts = match parts {
                Some(path) => {
                    let text = std::fs::read(&path)
                        .map_err(|e| HomeError::io("reading the parts file", &path, e))?;
                    let value: Vec<Value> =
                        serde_json::from_slice(&text).map_err(|source| HomeError::Json {
                            context: "the parts file is not a JSON array",
                            source,
                        })?;
                    Some(value)
                }
                None => None,
            };
            let meta = CallMeta {
                call_id,
                provider,
                api,
                model,
                started_at: started_at.unwrap_or_else(now),
                duration_ms,
                stream: parts.is_some(),
            };
            let home = Home::open(home)?;
            let blocks = home.blocks()?;
            let mut s = home.open_session(&session)?;
            let report = ingest_call_files(&mut s, &blocks, &meta, &request, &response, parts)?;
            Ok(json!({"command": "ingest-call", "report": report}))
        }
        Command::ResumeCheck { rendered, forked } => {
            let repeated = repeated_tool_actions(&rendered, &forked)?;
            Ok(json!({"command": "resume-check", "repeated_tool_actions": repeated}))
        }
    }
}

/// Write the authored file: user records as plain strings, assistant records
/// with model `authored`, the parent chain intact, a fresh session id.
fn fewshot(out: &Path, turns: &Path, cwd: &str) -> Result<u64, HomeError> {
    if out.exists() {
        return Err(HomeError::Exists {
            path: out.to_path_buf(),
        });
    }
    let text = std::fs::read_to_string(turns)
        .map_err(|e| HomeError::io("reading the turns file", turns, e))?;
    let session_id = uuid_shaped();
    let mut prev: Option<String> = None;
    let mut lines = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let (role, body) = line.split_once(": ").ok_or_else(|| HomeError::Malformed {
            path: turns.to_path_buf(),
            line: n + 1,
            what: "turn (`user: text` or `assistant: text`)",
            reason: "no `role: ` prefix".to_owned(),
        })?;
        if body.contains("\"type\":\"thinking\"") || body.contains("\"type\": \"thinking\"") {
            return Err(HomeError::Malformed {
                path: turns.to_path_buf(),
                line: n + 1,
                what: "turn",
                reason: "a thinking block is never authored".to_owned(),
            });
        }
        let uuid = uuid_shaped();
        let message = match role {
            "user" => json!({"role": "user", "content": body}),
            "assistant" => {
                json!({"id": format!("msg_authored_{n}"), "type": "message", "role": "assistant", "model": AUTHORED,
                "content": [{"type": "text", "text": body}], "stop_reason": "end_turn", "stop_sequence": null,
                "usage": {"input_tokens": 0, "output_tokens": 0}})
            }
            _ => {
                return Err(HomeError::Malformed {
                    path: turns.to_path_buf(),
                    line: n + 1,
                    what: "turn",
                    reason: "role must be user or assistant".to_owned(),
                });
            }
        };
        lines.push(json!({
            "parentUuid": prev, "isSidechain": false, "userType": "external", "cwd": cwd,
            "sessionId": session_id, "version": "2.1.281", "gitBranch": "", "uuid": uuid,
            "timestamp": now(), "type": role, "message": message,
        }));
        prev = Some(uuid);
    }
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| HomeError::io("creating the output directory", dir, e))?;
    }
    let mut body = String::new();
    for l in &lines {
        body.push_str(&serde_json::to_string(l).map_err(|source| HomeError::Json {
            context: "a record could not be serialised",
            source,
        })?);
        body.push('\n');
    }
    std::fs::write(out, body).map_err(|e| HomeError::io("writing the authored file", out, e))?;
    Ok(lines.len() as u64)
}

/// A fresh id in Claude Code's uuid shape.
fn uuid_shaped() -> String {
    let h = fresh_id();
    format!(
        "{}-{}-4{}-8{}-{}",
        &h[..8],
        &h[8..12],
        &h[13..16],
        &h[17..20],
        &h[20..32]
    )
}

/// Tool-use ids in the forked file that also appear in the rendered file.
fn repeated_tool_actions(rendered: &Path, forked: &Path) -> Result<u64, HomeError> {
    let ids = |path: &Path| -> Result<Vec<String>, HomeError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| HomeError::io("reading a transcript", path, e))?;
        let mut out = Vec::new();
        for line in text.lines() {
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if let Some(parts) = v
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(Value::as_array)
            {
                for p in parts {
                    if p.get("type").and_then(Value::as_str) == Some("tool_use") {
                        if let Some(id) = p.get("id").and_then(Value::as_str) {
                            out.push(id.to_owned());
                        }
                    }
                }
            }
        }
        Ok(out)
    };
    let before = ids(rendered)?;
    let after = ids(forked)?;
    // A tool action the fork *inherited* by copying the rendered records is not a repeat;
    // a repeat is an id that appears more times in the fork than in the rendered file.
    let mut repeated = 0u64;
    for id in before.iter().collect::<std::collections::BTreeSet<_>>() {
        let b = before.iter().filter(|x| *x == id).count();
        let a = after.iter().filter(|x| *x == id).count();
        repeated += u64::try_from(a.saturating_sub(b)).unwrap_or(u64::MAX);
    }
    Ok(repeated)
}
