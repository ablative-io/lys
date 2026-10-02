//! `lys-home render-launch` (HOME-002 R7): from a kept template and a session,
//! the files a Claude Code launch needs, in one directory, plus one JSON
//! report carrying the launch line; the render is recorded on the session as
//! a `template_render` event beside the context path.
//!
//! In order: the template is parsed (R1); the home and the session are opened
//! under the session's lock; each of the five target files is refused by path
//! if it exists; the session head hash is taken (R4); the session is rendered
//! with its loss account (R2); the template is stored (R3), only once the
//! render has succeeded, so a refused render stores nothing; the MCP file, the
//! environment file (R6) and the instructions file are written; the five files,
//! and the seed file when the render wrote one (HOME-006 R6), are hashed; the
//! manifest block is stored; the documents the session is
//! given are resolved (HOME-003 R4); the event is appended (R5) and the
//! `lys.given` entry under it; the report is returned with the given hash,
//! the SHA-256 of the record's canonical bytes, and says signed or unsigned.
//!
//! With `--key`, a raw 32-byte Ed25519 seed file, the key is loaded and the
//! two statement files are refused by path before anything is written; after
//! the given entry, the record is signed as a `lys.given_statement` (see
//! [`crate::record::given_statement`]) and the canonical bytes and the
//! statement are written as `given-data.json` and `given-statement.cose`, the
//! pair `lys verify --attestation --payload` reads. Without it none of that is
//! written. The `template_render` event never carries the given hash.
//! Nothing runs: the launch line is text in the report (ADR-007). `--out` is
//! made absolute first, so the manifest, the launch line and the given entry
//! never carry a relative path; the render is given the rendered file's path
//! and never looks for the user's home directory, which is read only to name
//! the session's config directory when the template sets none. Nothing is
//! written outside `--out` and the home, and nothing after a refusal; a
//! resolution that fails leaves the render's files and no entry of either
//! kind on the session, and the command fails by the document's path.

use std::path::{Path, PathBuf};

use clap::Args;
use lys_core::Ed25519Identity;
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::events::{ManifestFile, RenderManifest, template_render};
use crate::harness::claude_code::given::{
    CONFIG_DIR_NAME, CONFIG_DIR_VARIABLE, ConfigDir, ConfigSource, resolve_given,
};
use crate::harness::claude_code::launch_env::{Judge, confinement_named, write_env_file, write_new};
use crate::harness::claude_code::render::{RenderTarget, render_claude_code};
use crate::harness::claude_code::seed::seed_argument;
use crate::harness::claude_code::template::{Template, read_template};
use crate::harness::skills;
use crate::record::blocks::Hash;
use crate::record::entries::{CUSTOM_HARNESS_EVENT, EntryBody};
use crate::record::given::GivenRecord;
use crate::record::given_statement::GivenStatement;
use crate::record::{Home, safe_component};

/// The MCP configuration file's name under `--out`.
pub const MCP_FILE: &str = "mcp.json";
/// The environment (settings) file's name under `--out`.
pub const ENV_FILE: &str = "env.json";
/// The appended-instructions file's name under `--out`.
pub const INSTRUCTIONS_FILE: &str = "instructions.md";
/// The given statement's file name under `--out`.
pub const STATEMENT_FILE: &str = "given-statement.cose";
/// The given statement's payload file name under `--out`.
pub const PAYLOAD_FILE: &str = "given-data.json";

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
    /// A raw 32-byte Ed25519 seed file; when given, what the session was
    /// given is signed as a given statement.
    #[arg(long)]
    pub key: Option<PathBuf>,
    /// The runner's Unix socket the session's tool calls are judged on;
    /// when given, the settings file installs `lys runner judge` as the
    /// session's `PreToolUse` hook for every tool.
    #[arg(long, requires = "judge_program")]
    pub judge_socket: Option<PathBuf>,
    /// The `lys` program the hook runs, by absolute path.
    #[arg(long, requires = "judge_socket")]
    pub judge_program: Option<PathBuf>,
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
    confinement_named(&template)?;
    safe_component("uuid", &args.uuid)?;
    let key = args
        .key
        .as_deref()
        .map(|path| {
            Ed25519Identity::load(path).map_err(|e| HomeError::SigningKey {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })
        })
        .transpose()?;
    let home = Home::open(&args.home)?;
    let mut session = home.open_session(&args.session)?;
    let out = std::path::absolute(&args.out)
        .map_err(|e| HomeError::io("resolving the out directory", &args.out, e))?;
    let [rendered, loss, mcp_file, env, instructions] = targets(&out, &args.uuid);
    let signing = key.map(|key| (key, out.join(STATEMENT_FILE), out.join(PAYLOAD_FILE)));
    let mut refused = vec![&rendered, &loss, &mcp_file, &env, &instructions];
    refused.extend(signing.iter().flat_map(|(_, cose, data)| [cose, data]));
    for path in refused {
        if path.exists() {
            return Err(HomeError::LaunchTargetExists { path: path.clone() });
        }
    }
    let process_home = std::env::var_os("HOME").map(PathBuf::from);
    let config_dir = ConfigDir::resolve(
        template.env.get(CONFIG_DIR_VARIABLE).map(String::as_str),
        process_home.as_deref(),
    )?;
    let operators = process_home
        .as_deref()
        .map(|home| home.join(CONFIG_DIR_NAME));
    let same_dir = |a: &Path, b: &Path| match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    };
    let operators_own = operators
        .as_deref()
        .is_some_and(|operators| same_dir(&config_dir.path, operators));
    if !template.skills.is_empty() && (config_dir.source != ConfigSource::Template || operators_own)
    {
        return Err(HomeError::SkillDirectory {
            path: config_dir.path,
        });
    }
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
    // The template is stored only once its render has succeeded, so a
    // refused render leaves the home's templates as they were.
    let stored = home.templates().put(&template_bytes)?;
    let mcp = Value::Object(std::mem::take(&mut template.mcp));
    let mcp_bytes = serde_json::to_vec_pretty(&mcp).map_err(|source| HomeError::Json {
        context: "the MCP configuration could not be serialised",
        source,
    })?;
    write_new(&mcp_file, &with_newline(mcp_bytes))?;
    let judge = match (&args.judge_socket, &args.judge_program) {
        (Some(socket), Some(program)) => Some(Judge::new(program, socket)?),
        _ => None,
    };
    write_env_file(&template, judge.as_ref(), &env)?;
    write_new(&instructions, template.instructions.as_bytes())?;
    let skills = if template.skills.is_empty() {
        Vec::new()
    } else {
        skills::write(&config_dir.path, &template.skills)?
    };
    let skill_paths: Vec<PathBuf> = skills
        .iter()
        .map(|skill| config_dir.path.join(&skill.path))
        .collect();
    let mut written: Vec<&Path> = vec![&rendered, &loss, &mcp_file, &env, &instructions];
    if let Some(seed) = &render.seed {
        written.push(seed);
    }
    written.extend(skill_paths.iter().map(PathBuf::as_path));
    let mut files = Vec::with_capacity(written.len());
    for path in written {
        files.push(ManifestFile {
            path: path.to_path_buf(),
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
    let resolution = resolve_given(&args.cwd, config_dir, &out)?;
    let given = GivenRecord::claude_code(resolution, environment_names(&template));
    let event_id = session.append_beside(EntryBody::Custom {
        custom_type: CUSTOM_HARNESS_EVENT.to_owned(),
        data: Some(event.data()?),
    })?;
    let given_id = given.append_under(&mut session, &event_id)?;
    let canonical = given.canonical_bytes()?;
    let signed = match &signing {
        Some((key, cose_file, data_file)) => {
            let signed = GivenStatement::sign(&mut session, &blocks, &given_id, &canonical, key)?;
            write_new(data_file, canonical.as_bytes())?;
            write_new(cose_file, &signed.cose)?;
            Some((signed.entry, cose_file, data_file))
        }
        None => None,
    };
    let launch = launch_line(
        &template,
        &rendered,
        &mcp_file,
        &env,
        &instructions,
        render.seed.as_deref(),
    );
    let mut report = json!({
        "command": "render-launch",
        "template": stored.hash.as_str(),
        "session_head": session_head.as_str(),
        "uuid": args.uuid,
        "files": manifest.files,
        "launch": launch,
        "event": event_id,
        "manifest": manifest_put.hash.as_str(),
        "render": render,
        "given": given_id,
        "given_documents": given.documents.len(),
        "given_sha256": canonical.hash().as_str(),
        "skills": skills,
        "signing": if signed.is_some() { "signed" } else { "unsigned" },
    });
    if let Some((entry, cose_file, data_file)) = signed {
        report["statement"] = json!(entry);
        report["statement_file"] = json!(cose_file);
        report["payload_file"] = json!(data_file);
    }
    Ok(report)
}

/// The names of the variables the environment file sets for the session:
/// the template's variables and each use-only secret's variable, which the
/// record sorts. Never a value or a handle.
fn environment_names(template: &Template) -> Vec<String> {
    let mut names: Vec<String> = template.env.keys().cloned().collect();
    names.extend(template.use_only.iter().map(|secret| secret.env.clone()));
    names
}

/// The launch line: the rendered file resumed by path with `--fork-session`,
/// the three files, then the template's flags in order, then the seed as
/// the first prompt when the render wrote one. Never run here.
#[must_use]
pub fn launch_line(
    template: &Template,
    rendered: &Path,
    mcp: &Path,
    env: &Path,
    instructions: &Path,
    seed: Option<&Path>,
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
    if let Some(seed) = seed {
        words.push(seed_argument(seed));
    }
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
