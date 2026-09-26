//! The `lys-home` command line: import, render, fewshot, ingest-call,
//! resume-check, render-launch, given, given-check, lantern. Every command prints one
//! JSON report of paths, hashes and counts, never transcript, block or body
//! content. A missing required argument is refused by clap with exit code 2,
//! naming the argument. A command that refuses exits 1, except `given-check`,
//! which answers as `diff` does: 0 on matches, 1 on differs, 2 on a refusal
//! ([`given`]).

pub mod given;

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde_json::{Value, json};

use crate::cli::given::{GivenArgs, GivenCheckArgs, Outcome, STATUS_REFUSED};
use crate::cli_lantern::LanternAction;
use crate::error::HomeError;
use crate::harness::claude_code::AUTHORED;
use crate::harness::claude_code::import::import_claude_code;
use crate::harness::claude_code::launch::{LaunchArgs, render_launch};
use crate::harness::claude_code::render::{RenderTarget, render_claude_code};
use crate::record::call::{Api, CallMeta, ingest_call_files};
use crate::record::canon::Role;
use crate::record::{Home, fresh_id, now};

/// The home: sessions held under an identity, in Pi's session tree.
#[derive(Debug, Parser)]
#[command(name = "lys-home", version, about)]
pub struct Cli {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// The canon's own commands.
#[derive(Debug, Subcommand)]
pub enum CanonAction {
    /// Create an empty canon file with its header.
    Create {
        /// The canon file to create.
        #[arg(long)]
        canon: PathBuf,
    },
    /// Add one example: entries copied whole from a session of a home, or an authored turns file.
    Add {
        /// The canon file.
        #[arg(long)]
        canon: PathBuf,
        /// The home holding the source session.
        #[arg(long, requires = "from", conflicts_with = "authored")]
        home: Option<PathBuf>,
        /// The source session id.
        #[arg(long, requires = "home", conflicts_with = "authored")]
        from: Option<String>,
        /// The entry ids to copy, in order.
        #[arg(long, num_args = 1.., requires = "from", conflicts_with = "authored")]
        entries: Vec<String>,
        /// A turns file for an authored example.
        #[arg(long)]
        authored: Option<PathBuf>,
        /// The rule this example shows, stated short.
        #[arg(long)]
        rule: String,
        /// Who is curating it.
        #[arg(long)]
        by: String,
    },
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
        /// A canon file whose examples go first, before the session's own entries.
        #[arg(long)]
        canon: Option<PathBuf>,
    },
    /// The canon: the curated examples every new session starts from.
    Canon {
        /// What to do with it.
        #[command(subcommand)]
        action: CanonAction,
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
    /// Write the files a Claude Code launch needs from a kept template and a session.
    RenderLaunch(LaunchArgs),
    /// List a session's given records: what each rendered session was given, as paths, lengths and hashes.
    Given(GivenArgs),
    /// Check a document a given record lists against a file on disk by hash: exit 0 on matches, 1 on differs, 2 on a refusal.
    GivenCheck(GivenCheckArgs),
    /// Lanterns: light one at an entry, add an epilogue, recall by note or by point.
    Lantern {
        /// What to do with them.
        #[command(subcommand)]
        action: LanternAction,
    },
}

impl Command {
    /// The status the process exits with when this command refuses: 2 for
    /// `given-check`, as `diff` does, and 1 for every other command.
    #[must_use]
    pub fn refusal_status(&self) -> i32 {
        match self {
            Self::GivenCheck(_) => STATUS_REFUSED,
            _ => 1,
        }
    }
}

/// Run a command and return its report; a refusal is the caller's to exit on
/// with [`Command::refusal_status`].
pub fn run(cli: Cli) -> Result<Value, HomeError> {
    run_with_status(cli).map(|outcome| outcome.report)
}

/// Run a command and return its report with the status the process exits
/// with: 0 for every command but a `given-check` that answers differs.
pub fn run_with_status(cli: Cli) -> Result<Outcome, HomeError> {
    match cli.command {
        Command::GivenCheck(args) => given::check(&args),
        other => report(other).map(Outcome::done),
    }
}

fn report(command: Command) -> Result<Value, HomeError> {
    match command {
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
            canon,
        } => {
            let home = Home::open(home)?;
            let s = home.open_session(&session)?;
            let user_home = std::env::var_os("HOME").map(PathBuf::from);
            let target = RenderTarget {
                session_id: uuid,
                cwd,
                model,
                version,
                out,
                canon,
            };
            let report = render_claude_code(&s, &target, user_home.as_deref())?;
            Ok(json!({"command": "render", "report": report}))
        }
        Command::Canon { action } => match action {
            CanonAction::Create { canon } => {
                crate::record::canon::create(&canon)?;
                Ok(json!({"command": "canon create", "canon": canon}))
            }
            CanonAction::Add {
                canon,
                home,
                from,
                entries,
                authored,
                rule,
                by,
            } => {
                let report = match (authored, home, from) {
                    (Some(turns), _, _) => {
                        crate::record::canon::add_authored(&canon, &turns, &rule, &by)?
                    }
                    (None, Some(home), Some(from)) => {
                        let home = Home::open(home)?;
                        let s = home.open_session(&from)?;
                        crate::record::canon::add_from(&canon, &s, &entries, &rule, &by)?
                    }
                    (None, _, _) => {
                        return Err(HomeError::BodyShape {
                            api: "canon",
                            reason: "an example comes from --home/--from/--entries or from --authored",
                        });
                    }
                };
                Ok(json!({"command": "canon add", "report": report}))
            }
        },
        Command::Fewshot { out, turns, cwd } => {
            let written = fewshot(&out, &turns, &cwd)?;
            // The person's own command, carried in the one JSON report; never run here.
            let resume = format!("claude --resume {}", out.display());
            Ok(
                json!({"command": "fewshot", "file": out, "records": written, "authored": true, "resume": resume}),
            )
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
            let check = resume_check(&rendered, &forked)?;
            if check.repeated_tool_use_ids != 0 {
                return Err(HomeError::RepeatedToolActions {
                    count: check.repeated_tool_use_ids,
                });
            }
            Ok(json!({"command": "resume-check", "report": check}))
        }
        Command::RenderLaunch(args) => render_launch(&args),
        Command::Given(args) => given::list(&args),
        Command::GivenCheck(args) => given::check(&args).map(|outcome| outcome.report),
        Command::Lantern { action } => crate::cli_lantern::run(action),
    }
}

/// Write the authored file: user records as plain strings, assistant records
/// with model `authored`, the parent chain intact, a fresh session id.
fn fewshot(out: &Path, turns: &Path, cwd: &str) -> Result<u64, HomeError> {
    let turns = crate::record::canon::parse_turns(turns)?;
    let session_id = uuid_shaped();
    let mut prev: Option<String> = None;
    let mut lines = Vec::new();
    for (n, (role, body)) in turns.into_iter().enumerate() {
        let uuid = uuid_shaped();
        let (kind, message) = match role {
            Role::User => ("user", json!({"role": "user", "content": body})),
            Role::Assistant => (
                "assistant",
                json!({"id": format!("msg_authored_{n}"), "type": "message", "role": "assistant", "model": AUTHORED,
                "content": [{"type": "text", "text": body}], "stop_reason": "end_turn", "stop_sequence": null,
                "usage": {"input_tokens": 0, "output_tokens": 0}}),
            ),
        };
        lines.push(json!({
            "parentUuid": prev, "isSidechain": false, "userType": "external", "cwd": cwd,
            "sessionId": session_id, "version": "2.1.281", "gitBranch": "", "uuid": uuid,
            "timestamp": now(), "type": kind, "message": message,
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
    // Never over an existing transcript: the create is exclusive, so a second
    // writer between a check and a write cannot slip in; then the file and its
    // directory are synced before the path is reported.
    let mut file = match std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(out)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(HomeError::Exists {
                path: out.to_path_buf(),
            });
        }
        Err(e) => return Err(HomeError::io("creating the authored file", out, e)),
    };
    file.write_all(body.as_bytes())
        .map_err(|e| HomeError::io("writing the authored file", out, e))?;
    file.sync_all()
        .map_err(|e| HomeError::io("syncing the authored file", out, e))?;
    if let Some(dir) = out.parent() {
        crate::record::blocks::sync_dir(dir)?;
    }
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

/// What a resume check measured. `repeated_tool_use_ids` is exactly that: ids
/// that appear more times in the fork than in the rendered file; it says
/// nothing about an action repeated under a fresh id. `new_tool_uses` counts
/// every `tool_use` part in the fork's own records (those not copied from the
/// rendered file), which for a one-turn question answerable without tools is
/// expected to be 0.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct ResumeReport {
    /// Records in the rendered file.
    pub rendered_records: u64,
    /// Records in the fork.
    pub forked_records: u64,
    /// Records in the fork whose uuid is not in the rendered file.
    pub forked_new_records: u64,
    /// `tool_use` ids appearing more times in the fork than in the rendered file.
    pub repeated_tool_use_ids: u64,
    /// `tool_use` parts in the fork's new records.
    pub new_tool_uses: u64,
}

/// One record's uuid and its `tool_use` ids.
fn tool_uses(path: &Path) -> Result<Vec<(String, Vec<String>)>, HomeError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| HomeError::io("reading a transcript", path, e))?;
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line).map_err(|e| HomeError::Malformed {
            path: path.to_path_buf(),
            line: n + 1,
            what: "Claude Code record",
            reason: e.to_string(),
        })?;
        let uuid = v
            .get("uuid")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let mut ids = Vec::new();
        if let Some(parts) = v
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(Value::as_array)
        {
            for p in parts {
                if p.get("type").and_then(Value::as_str) == Some("tool_use")
                    && let Some(id) = p.get("id").and_then(Value::as_str)
                {
                    ids.push(id.to_owned());
                }
            }
        }
        out.push((uuid, ids));
    }
    Ok(out)
}

fn resume_check(rendered: &Path, forked: &Path) -> Result<ResumeReport, HomeError> {
    let before = tool_uses(rendered)?;
    let after = tool_uses(forked)?;
    let rendered_uuids: std::collections::BTreeSet<&str> =
        before.iter().map(|(u, _)| u.as_str()).collect();
    let count = |records: &[(String, Vec<String>)], id: &str| -> u64 {
        u64::try_from(
            records
                .iter()
                .flat_map(|(_, ids)| ids.iter())
                .filter(|x| x.as_str() == id)
                .count(),
        )
        .unwrap_or(u64::MAX)
    };
    // A tool action the fork *inherited* by copying the rendered records is not a repeat;
    // a repeat is an id that appears more times in the fork than in the rendered file.
    let mut repeated = 0u64;
    for id in before
        .iter()
        .flat_map(|(_, ids)| ids.iter())
        .collect::<std::collections::BTreeSet<_>>()
    {
        repeated += count(&after, id).saturating_sub(count(&before, id));
    }
    let new: Vec<&(String, Vec<String>)> = after
        .iter()
        .filter(|(u, _)| !rendered_uuids.contains(u.as_str()))
        .collect();
    Ok(ResumeReport {
        rendered_records: before.len() as u64,
        forked_records: after.len() as u64,
        forked_new_records: new.len() as u64,
        repeated_tool_use_ids: repeated,
        new_tool_uses: new.iter().map(|(_, ids)| ids.len() as u64).sum(),
    })
}
