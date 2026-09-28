//! The `lys-home` command line: import, render, fewshot, ingest-call,
//! resume-check, render-launch, given, given-check, lantern, fork, handover,
//! ship, fetch. Every command prints one
//! JSON report of paths, hashes and counts, never transcript, block or body
//! content. A missing required argument is refused by clap with exit code 2,
//! naming the argument. A command that refuses exits 1, except `given-check`,
//! which answers as `diff` does: 0 on matches, 1 on differs, 2 on a refusal
//! ([`given`]).
//! `ship` and `fetch` ([`crate::cli_move`]) print two refusals as reports on
//! stdout with exit 1: `stale_index` and `verification_failed`.

mod fewshot;
pub mod given;
mod resume;

pub use resume::{ResumeReport, resume_check};

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use serde_json::{Value, json};

use crate::cli::given::{GivenArgs, GivenCheckArgs, Outcome, STATUS_REFUSED};
use crate::cli_fork::ForkArgs;
use crate::cli_lantern::LanternAction;
use crate::cli_move::{FetchArgs, ShipArgs};
use crate::cli_translate::TranslateArgs;
use crate::error::HomeError;
use crate::harness::claude_code::import::import_claude_code;
use crate::harness::claude_code::launch::{LaunchArgs, render_launch};
use crate::harness::claude_code::render::{RenderTarget, render_claude_code};
use crate::record::call::{Api, CallMeta, ingest_call_files};
use crate::record::handover::handover;
use crate::record::{Home, now};

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

/// The arguments of `handover`: the outgoing home and session, the letter's
/// entry ids in path order, and the successor home to write. No rule is
/// taken, since a letter is not a rule.
#[derive(Clone, Debug, PartialEq, Eq, Args)]
pub struct HandoverArgs {
    /// The outgoing home, read and never written.
    #[arg(long)]
    pub home: PathBuf,
    /// The outgoing session id.
    #[arg(long)]
    pub from: String,
    /// The letter's entry ids, in path order.
    #[arg(long, required = true, num_args = 1..)]
    pub letter: Vec<String>,
    /// The successor home: an absent path or an empty directory.
    #[arg(long)]
    pub successor: PathBuf,
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
    /// Fork a child session from a lantern's point, with its ancestry on both sides.
    Fork(ForkArgs),
    /// Translate a session into a Codex rollout with a loss account beside it.
    TranslateCodex(TranslateArgs),
    /// Hand an outgoing session's letter to a new successor home as inherited memory.
    Handover(HandoverArgs),
    /// Ship the home as the one ref refs/lys/home to a bare repository at a path on this machine.
    Ship(ShipArgs),
    /// Fetch a shipped home into a new, empty home and hang an arrival beside each session's head.
    Fetch(FetchArgs),
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
        Command::Ship(args) => crate::cli_move::run_ship(&args),
        Command::Fetch(args) => crate::cli_move::run_fetch(&args),
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
            let mut s = home.stage_session(&session, "")?;
            let report = import_claude_code(&claude_code, &mut s, &blocks)?;
            s.publish()?;
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
            let written = fewshot::fewshot(&out, &turns, &cwd)?;
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
            let check = resume::resume_check(&rendered, &forked)?;
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
        Command::Fork(args) => crate::cli_fork::run(&args),
        Command::TranslateCodex(args) => crate::cli_translate::run(&args),
        Command::Handover(args) => {
            let report = handover(&args.home, &args.from, &args.letter, &args.successor)?;
            let report = serde_json::to_value(&report).map_err(|source| HomeError::Json {
                context: "the handover report could not be serialised",
                source,
            })?;
            Ok(json!({"command": "handover", "report": report}))
        }
        Command::Ship(args) => crate::cli_move::run_ship(&args).map(|outcome| outcome.report),
        Command::Fetch(args) => crate::cli_move::run_fetch(&args).map(|outcome| outcome.report),
    }
}
