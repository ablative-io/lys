//! `lys-home render-launch` (HOME-002 R7): from a kept template and a session,
//! the files a Claude Code launch needs, in one directory, plus one JSON
//! report carrying the launch line; the render is recorded on the session as
//! a `template_render` event beside the context path.
//!
//! In order: the template is parsed (R1); the home and the session are opened
//! under the session's lock; each of the five target files is refused by path
//! if it exists; the template is stored (R3); the session head hash is taken
//! (R4); the session is rendered with its loss account (R2); the MCP file, the
//! environment file (R6) and the instructions file are written; the five files
//! are hashed; the manifest block is stored and the event appended (R5); the
//! report is returned. Nothing runs: the launch line is text in the report
//! (ADR-007). `--out` is made absolute first, so the manifest and the launch
//! line never carry a relative path; the render is given the rendered file's
//! path and never looks for the user's home directory. Nothing is written
//! outside `--out` and the home, and nothing after a refusal.

use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::events::{ManifestFile, RenderManifest, template_render};
use crate::harness::claude_code::launch_env::{write_env_file, write_new};
use crate::harness::claude_code::render::{RenderTarget, render_claude_code};
use crate::harness::claude_code::template::{Template, read_template};
use crate::record::blocks::Hash;
use crate::record::entries::{CUSTOM_HARNESS_EVENT, EntryBody};
use crate::record::{Home, safe_component};

/// The MCP configuration file's name under `--out`.
pub const MCP_FILE: &str = "mcp.json";
/// The environment (settings) file's name under `--out`.
pub const ENV_FILE: &str = "env.json";
/// The appended-instructions file's name under `--out`.
pub const INSTRUCTIONS_FILE: &str = "instructions.md";

/// The arguments of `render-launch`.
#[derive(Clone, Debug, PartialEq, Eq, Args)]
pub struct LaunchArgs {
    /// The home directory.
    #[arg(long)]
    pub home: PathBuf,
    /// The session to launch.
    #[arg(long)]
    pub session: String,
    /// The launch template file.
    #[arg(long)]
    pub template: PathBuf,
    /// The session id the rendered file carries.
    #[arg(long)]
    pub uuid: String,
    /// The working directory the rendered records name.
    #[arg(long)]
    pub cwd: String,
    /// The model the file is rendered for.
    #[arg(long)]
    pub model: String,
    /// The Claude Code version written into records.
    #[arg(long)]
    pub version: String,
    /// The directory the five files are written into.
    #[arg(long)]
    pub out: PathBuf,
}

/// The five files a launch is made of, under `out`, in write order.
fn targets(out: &Path, uuid: &str) -> [PathBuf; 5] {
    [
        out.join(format!("{uuid}.jsonl")),
        out.join(format!("{uuid}.loss.json")),
        out.join(MCP_FILE),
        out.join(ENV_FILE),
        out.join(INSTRUCTIONS_FILE),
    ]
}

/// Run `render-launch` and return its report.
pub fn render_launch(args: &LaunchArgs) -> Result<Value, HomeError> {
    let (mut template, template_bytes) = read_template(&args.template)?;
    safe_component("uuid", &args.uuid)?;
    let home = Home::open(&args.home)?;
    let mut session = home.open_session(&args.session)?;
    let out = std::path::absolute(&args.out)
        .map_err(|e| HomeError::io("resolving the out directory", &args.out, e))?;
    let [rendered, loss, mcp_file, env, instructions] = targets(&out, &args.uuid);
    for path in [&rendered, &loss, &mcp_file, &env, &instructions] {
        if path.exists() {
            return Err(HomeError::LaunchTargetExists { path: path.clone() });
        }
    }
    let stored = home.templates().put(&template_bytes)?;
    let session_head = session.head_hash()?;
    let head = session.head()?.map(str::to_owned);
    let target = RenderTarget {
        session_id: args.uuid.clone(),
        cwd: args.cwd.clone(),
        model: args.model.clone(),
        version: args.version.clone(),
        out: Some(rendered.clone()),
        canon: template.canon.clone(),
    };
    let render = render_claude_code(&session, &target, None)?;
    let mcp = Value::Object(std::mem::take(&mut template.mcp));
    let mcp_bytes = serde_json::to_vec_pretty(&mcp).map_err(|source| HomeError::Json {
        context: "the MCP configuration could not be serialised",
        source,
    })?;
    write_new(&mcp_file, &with_newline(mcp_bytes))?;
    write_env_file(&template, &env)?;
    write_new(&instructions, template.instructions.as_bytes())?;
    let mut files = Vec::with_capacity(5);
    for path in [&rendered, &loss, &mcp_file, &env, &instructions] {
        files.push(ManifestFile {
            path: path.clone(),
            sha256: hash_file(path)?.as_str().to_owned(),
        });
    }
    let manifest = RenderManifest {
        template: stored.hash.as_str().to_owned(),
        session_head: session_head.as_str().to_owned(),
        head,
        uuid: args.uuid.clone(),
        files,
    };
    let blocks = home.blocks()?;
    let manifest_put = manifest.store(&blocks)?;
    let event = template_render(
        &stored.hash,
        &session_head,
        &manifest_put.hash,
        manifest.files.len() as u64,
    );
    let event_id = session.append_beside(EntryBody::Custom {
        custom_type: CUSTOM_HARNESS_EVENT.to_owned(),
        data: Some(event.data()?),
    })?;
    let launch = launch_line(&template, &rendered, &mcp_file, &env, &instructions);
    Ok(json!({
        "command": "render-launch",
        "template": stored.hash.as_str(),
        "session_head": session_head.as_str(),
        "uuid": args.uuid,
        "files": manifest.files,
        "launch": launch,
        "event": event_id,
        "manifest": manifest_put.hash.as_str(),
        "render": render,
    }))
}

/// The launch line: the rendered file resumed by path with `--fork-session`,
/// the three files, then the template's flags in order. Never run here.
#[must_use]
pub fn launch_line(
    template: &Template,
    rendered: &Path,
    mcp: &Path,
    env: &Path,
    instructions: &Path,
) -> String {
    let mut words: Vec<String> = vec![
        "claude".to_owned(),
        "--resume".to_owned(),
        shell_word(&rendered.display().to_string()),
        "--fork-session".to_owned(),
        "--mcp-config".to_owned(),
        shell_word(&mcp.display().to_string()),
        "--settings".to_owned(),
        shell_word(&env.display().to_string()),
        "--append-system-prompt-file".to_owned(),
        shell_word(&instructions.display().to_string()),
    ];
    words.extend(template.flags.iter().map(|flag| shell_word(flag)));
    words.join(" ")
}

/// One argument for a POSIX shell: as is when every character is a letter,
/// a digit or one of `._/=:-`; otherwise single-quoted, with a quote inside
/// written as `'\''`.
#[must_use]
pub fn shell_word(arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._/=:-".contains(c));
    if plain {
        arg.to_owned()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

fn with_newline(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(b'\n');
    bytes
}

fn hash_file(path: &Path) -> Result<Hash, HomeError> {
    let bytes = std::fs::read(path).map_err(|e| HomeError::io("hashing a launch file", path, e))?;
    Ok(Hash::of(&bytes))
}
