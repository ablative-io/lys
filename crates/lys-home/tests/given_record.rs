//! The context record end to end through the built binary (HOME-003 R6):
//! the fixture template rendered for a fixture working directory holding a
//! CLAUDE.md and a memory index under the fixture config directory, with
//! HOME a fresh directory and `CLAUDE_CONFIG_DIR` removed from the rendering
//! process's environment, so nothing here depends on the machine's own
//! configuration. Every assertion over a documents list first counts it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use lys_home::harness::claude_code::projects_slug;
use lys_home::record::given::GivenRecord;
use lys_home::{Home, Session};

const BIN: &str = env!("CARGO_BIN_EXE_lys-home");
const TEMPLATE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/template.json"
);
const SESSION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/session.jsonl"
);
const UUID: &str = "00000000-0000-4000-8000-000000000003";
/// The one line of the fixture CLAUDE.md; searched for, never quoted in a name.
const SENTENCE: &str =
    "Fixture instructions: the record carries this line's hash and never the line.\n";
const MEMORY: &str = "# Fixture memory index\n";
const KINDS: [&str; 4] = [
    "appended_instructions",
    "mcp_config",
    "claude_md_chain",
    "memory_index",
];

type Outcome = Result<(), Box<dyn std::error::Error>>;
type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn text(path: &Path) -> Fallible<&str> {
    Ok(path.to_str().ok_or("a UTF-8 path")?)
}

fn put(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, bytes)
}

fn stdout_json(output: &Output) -> Fallible<Value> {
    let stdout = std::str::from_utf8(&output.stdout)?;
    assert_eq!(stdout.lines().count(), 1, "one report line: {stdout}");
    Ok(serde_json::from_str(stdout)?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The fixture: a home holding the fixture session, a working directory `w`
/// with the fixture CLAUDE.md, a config directory `c` with the memory index
/// for `w`, a HOME `h` with no `.claude`, and the fixture template with
/// `CLAUDE_CONFIG_DIR` set to `c` in its env slot.
struct Fixture {
    dir: tempfile::TempDir,
    home: PathBuf,
    w: PathBuf,
    c: PathBuf,
    h: PathBuf,
    template: PathBuf,
    renders: usize,
}

impl Fixture {
    fn new() -> Fallible<Self> {
        let dir = tempfile::tempdir()?;
        let home = dir.path().join("home");
        Home::open(&home)?;
        std::fs::copy(SESSION, home.join("sessions").join("fixture.jsonl"))?;
        let w = dir.path().join("w");
        let c = dir.path().join("c");
        let h = dir.path().join("h");
        put(&w.join("CLAUDE.md"), SENTENCE.as_bytes())?;
        put(&Self::memory_index(&c, &w)?, MEMORY.as_bytes())?;
        std::fs::create_dir_all(&c)?;
        std::fs::create_dir_all(&h)?;
        let mut template: Value = serde_json::from_slice(&std::fs::read(TEMPLATE)?)?;
        template["slots"]["env"]["CLAUDE_CONFIG_DIR"] = json!(text(&c)?);
        let template_path = dir.path().join("template.json");
        std::fs::write(&template_path, serde_json::to_vec_pretty(&template)?)?;
        Ok(Self {
            dir,
            home,
            w,
            c,
            h,
            template: template_path,
            renders: 0,
        })
    }

    fn memory_index(config: &Path, cwd: &Path) -> Fallible<PathBuf> {
        Ok(config
            .join("projects")
            .join(projects_slug(text(cwd)?))
            .join("memory")
            .join("MEMORY.md"))
    }

    fn claude_md(&self) -> PathBuf {
        self.w.join("CLAUDE.md")
    }

    /// One render into a fresh out directory; returns the out directory and
    /// the process's output. HOME is `h`; `CLAUDE_CONFIG_DIR` is removed.
    fn render(&mut self) -> Fallible<(PathBuf, Output)> {
        self.renders += 1;
        let out = self.dir.path().join(format!("out{}", self.renders));
        std::fs::create_dir(&out)?;
        let output = Command::new(BIN)
            .args([
                "render-launch",
                "--home",
                text(&self.home)?,
                "--session",
                "fixture",
                "--template",
                text(&self.template)?,
                "--uuid",
                UUID,
                "--cwd",
                text(&self.w)?,
                "--model",
                "claude-fixture",
                "--version",
                "2.1.283",
                "--out",
                text(&out)?,
            ])
            .env_remove("CLAUDE_CONFIG_DIR")
            .env("HOME", &self.h)
            .output()?;
        Ok((out, output))
    }

    fn given(&self) -> Fallible<Output> {
        Ok(Command::new(BIN)
            .args(["given", "--home", text(&self.home)?, "--session", "fixture"])
            .output()?)
    }

    fn check(&self, entry: &str, path: &str, file: Option<&Path>) -> Fallible<Output> {
        let mut command = Command::new(BIN);
        command.args([
            "given-check",
            "--home",
            text(&self.home)?,
            "--session",
            "fixture",
            "--entry",
            entry,
            "--path",
            path,
        ]);
        if let Some(file) = file {
            command.args(["--file", text(file)?]);
        }
        Ok(command.output()?)
    }

    fn session(&self) -> Fallible<Session> {
        Ok(Session::open(
            self.home.join("sessions").join("fixture.jsonl"),
        )?)
    }

    /// The given records of the session in file order, each with its id.
    fn records(&self) -> Fallible<Vec<(String, GivenRecord)>> {
        let session = self.session()?;
        let records = GivenRecord::read_all(&session)?;
        drop(session);
        Ok(records)
    }

    /// The lines of the session file.
    fn lines(&self) -> Fallible<Vec<String>> {
        let file = std::fs::read_to_string(self.home.join("sessions").join("fixture.jsonl"))?;
        Ok(file.lines().map(str::to_owned).collect())
    }
}

/// A render that must succeed: its report.
fn rendered(fixture: &mut Fixture) -> Fallible<(PathBuf, Value)> {
    let (out, output) = fixture.render()?;
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    Ok((out, stdout_json(&output)?))
}

#[test]
fn the_first_render_records_four_documents_in_order_under_the_render_event() -> Outcome {
    let mut fixture = Fixture::new()?;
    let (out, report) = rendered(&mut fixture)?;
    let records = fixture.records()?;
    assert_eq!(records.len(), 1);
    let (id, record) = &records[0];
    assert_eq!(report["given"], json!(id));
    assert_eq!(report["given_documents"], 4);
    let session = fixture.session()?;
    let entry = session.entry(id)?;
    assert_eq!(
        entry.parent_id(),
        Some(report["event"].as_str().ok_or("an id")?)
    );
    assert_eq!(session.head()?, Some("e4"));
    assert_eq!(session.context_path()?.len(), 4);
    drop(session);
    let documents = &record.documents;
    assert_eq!(documents.len(), 4);
    let kinds: Vec<&str> = documents.iter().map(|d| d.kind.name()).collect();
    assert_eq!(kinds, KINDS);
    let on_disk = [
        out.join("instructions.md"),
        out.join("mcp.json"),
        fixture.claude_md(),
        Fixture::memory_index(&fixture.c, &fixture.w)?,
    ];
    for (document, path) in documents.iter().zip(&on_disk) {
        let bytes = std::fs::read(path)?;
        assert_eq!(
            document.length,
            bytes.len() as u64,
            "{}",
            document.kind.name()
        );
        assert_eq!(
            document.sha256,
            sha256_hex(&bytes),
            "{}",
            document.kind.name()
        );
    }
    assert_eq!(documents[0].path, Path::new("instructions.md"));
    assert_eq!(documents[1].path, Path::new("mcp.json"));
    assert_eq!(documents[2].path, fixture.claude_md());
    assert!(documents[3].path.starts_with(&fixture.c));
    let data = record.data()?;
    assert_eq!(
        data["config_dir"],
        json!({"path": text(&fixture.c)?, "source": "template"})
    );
    assert_eq!(
        data["environment"],
        json!(["CLAUDE_CONFIG_DIR", "LYS_FIXTURE_MODE", "LYS_FIXTURE_TOKEN"])
    );
    assert_eq!(kinds.iter().filter(|k| **k == "user_claude_md").count(), 0);
    assert_eq!(record.harness, "claude-code");
    assert_eq!(record.harness_version, "2.1.283");
    Ok(())
}

#[test]
fn two_renders_give_equal_lists_and_one_changed_byte_changes_exactly_one_hash() -> Outcome {
    let mut fixture = Fixture::new()?;
    rendered(&mut fixture)?;
    rendered(&mut fixture)?;
    let records = fixture.records()?;
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].1.documents.len(), 4);
    assert_eq!(records[1].1.documents.len(), 4);
    assert_eq!(records[0].1.documents, records[1].1.documents);
    assert_eq!(records[0].1, records[1].1);
    let second = records[1].0.clone();
    let claude_md = fixture.claude_md();
    let listed = text(&claude_md)?.to_owned();
    let matches = fixture.check(&second, &listed, Some(&claude_md))?;
    assert_eq!(matches.status.code(), Some(0), "{}", stderr(&matches));
    let matches_report = stdout_json(&matches)?;
    assert_eq!(matches_report["answer"], "matches");
    let mut bytes = std::fs::read(&claude_md)?;
    let last = bytes.len() - 2;
    bytes[last] = if bytes[last] == b'x' { b'y' } else { b'x' };
    std::fs::write(&claude_md, &bytes)?;
    let (_, third_report) = rendered(&mut fixture)?;
    let records = fixture.records()?;
    assert_eq!(records.len(), 3);
    assert_eq!(records[2].0, third_report["given"].as_str().ok_or("an id")?);
    let (before, after) = (&records[1].1, &records[2].1);
    assert_eq!(before.documents.len(), 4);
    assert_eq!(after.documents.len(), 4);
    let mut differing = Vec::new();
    for (b, a) in before.documents.iter().zip(&after.documents) {
        assert_eq!(b.kind, a.kind);
        assert_eq!(b.path, a.path);
        assert_eq!(b.length, a.length);
        if b.sha256 != a.sha256 {
            differing.push(a.kind.name());
        }
    }
    assert_eq!(differing, ["claude_md_chain"]);
    assert_eq!(after.documents[2].sha256, sha256_hex(&bytes));
    assert_eq!(before.config_dir, after.config_dir);
    assert_eq!(before.environment, after.environment);
    assert_eq!(before.kinds, after.kinds);
    let differs = fixture.check(&second, &listed, Some(&claude_md))?;
    assert_eq!(differs.status.code(), Some(1), "{}", stderr(&differs));
    let differs_report = stdout_json(&differs)?;
    assert_eq!(differs_report["answer"], "differs");
    assert_eq!(differs_report["command"], matches_report["command"]);
    assert_eq!(differs_report["listed"], matches_report["listed"]);
    let given = fixture.given()?;
    assert_eq!(given.status.code(), Some(0), "{}", stderr(&given));
    let given_report = stdout_json(&given)?;
    assert_eq!(given_report["count"], 3);
    let listed_records = given_report["records"].as_array().ok_or("a list")?;
    assert_eq!(listed_records.len(), 3);
    for (listed_record, (id, record)) in listed_records.iter().zip(&records) {
        assert_eq!(listed_record["entry"], json!(id));
        assert_eq!(listed_record["documents"], record.data()?["documents"]);
        assert_eq!(listed_record["kinds"], record.data()?["kinds"]);
        assert_eq!(listed_record["config_dir"], record.data()?["config_dir"]);
        assert_eq!(listed_record["environment"], record.data()?["environment"]);
        assert_eq!(
            listed_record["documents"].as_array().ok_or("a list")?.len(),
            4
        );
    }
    let sentence = SENTENCE.trim();
    let lines = fixture.lines()?;
    let given_lines: Vec<&String> = lines.iter().filter(|l| l.contains("lys.given")).collect();
    assert_eq!(given_lines.len(), 3);
    for line in &given_lines {
        assert_eq!(line.matches(sentence).count(), 0);
    }
    for output in [&given, &matches, &differs] {
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .matches(sentence)
                .count(),
            0
        );
        assert_eq!(stderr(output).matches(sentence).count(), 0);
    }
    let header: Value = serde_json::from_str(&lines[0])?;
    assert_eq!(header["type"], "session");
    assert!(lines.len() > 1);
    for line in &lines[1..] {
        let entry: Value = serde_json::from_str(line)?;
        for key in ["id", "parentId", "timestamp"] {
            assert!(entry.get(key).is_some(), "{key}");
        }
    }
    Ok(())
}

#[test]
fn given_check_refuses_with_status_2_naming_the_id_the_path_or_the_file() -> Outcome {
    let mut fixture = Fixture::new()?;
    rendered(&mut fixture)?;
    let records = fixture.records()?;
    assert_eq!(records.len(), 1);
    let entry = records[0].0.clone();
    let claude_md = fixture.claude_md();
    let listed = text(&claude_md)?.to_owned();

    let unknown = fixture.check("no-such-entry", &listed, Some(&claude_md))?;
    assert_eq!(unknown.status.code(), Some(2), "{}", stderr(&unknown));
    assert!(
        stderr(&unknown).contains("no-such-entry"),
        "{}",
        stderr(&unknown)
    );
    assert!(unknown.stdout.is_empty());

    let unlisted_path = text(&fixture.w.join("OTHER.md"))?.to_owned();
    let unlisted = fixture.check(&entry, &unlisted_path, Some(&claude_md))?;
    assert_eq!(unlisted.status.code(), Some(2), "{}", stderr(&unlisted));
    assert!(
        stderr(&unlisted).contains(&unlisted_path),
        "{}",
        stderr(&unlisted)
    );
    assert!(unlisted.stdout.is_empty());

    let no_file = fixture.check(&entry, &listed, None)?;
    assert_eq!(no_file.status.code(), Some(2), "{}", stderr(&no_file));
    assert!(stderr(&no_file).contains("--file"), "{}", stderr(&no_file));
    assert!(no_file.stdout.is_empty());

    let absent = fixture.dir.path().join("absent.md");
    let missing = fixture.check(&entry, &listed, Some(&absent))?;
    assert_eq!(missing.status.code(), Some(2), "{}", stderr(&missing));
    assert!(
        stderr(&missing).contains(text(&absent)?),
        "{}",
        stderr(&missing)
    );
    assert!(missing.stdout.is_empty());

    let event = fixture.session()?.customs_everywhere("lys.harness_event")?;
    assert_eq!(event.len(), 1);
    let not_given = fixture.check(event[0].id(), &listed, Some(&claude_md))?;
    assert_eq!(not_given.status.code(), Some(2), "{}", stderr(&not_given));
    assert!(
        stderr(&not_given).contains(event[0].id()),
        "{}",
        stderr(&not_given)
    );
    Ok(())
}

#[test]
fn the_rendering_process_s_config_dir_is_never_read_and_home_dot_claude_is_the_fallback() -> Outcome
{
    let mut fixture = Fixture::new()?;
    // A template setting no CLAUDE_CONFIG_DIR, a process CLAUDE_CONFIG_DIR
    // naming a directory with its own memory index and CLAUDE.md, and a
    // HOME whose .claude holds the user CLAUDE.md and a memory index.
    let mut template: Value = serde_json::from_slice(&std::fs::read(&fixture.template)?)?;
    template["slots"]["env"]
        .as_object_mut()
        .ok_or("an object")?
        .remove("CLAUDE_CONFIG_DIR");
    std::fs::write(&fixture.template, serde_json::to_vec_pretty(&template)?)?;
    let other = fixture.dir.path().join("other");
    put(
        &Fixture::memory_index(&other, &fixture.w)?,
        b"other memory\n",
    )?;
    put(&other.join("CLAUDE.md"), b"other user file\n")?;
    let home_config = fixture.h.join(".claude");
    put(
        &Fixture::memory_index(&home_config, &fixture.w)?,
        b"home memory\n",
    )?;
    put(&home_config.join("CLAUDE.md"), b"home user file\n")?;
    fixture.renders += 1;
    let out = fixture.dir.path().join("out-home");
    std::fs::create_dir(&out)?;
    let output = Command::new(BIN)
        .args([
            "render-launch",
            "--home",
            text(&fixture.home)?,
            "--session",
            "fixture",
            "--template",
            text(&fixture.template)?,
            "--uuid",
            UUID,
            "--cwd",
            text(&fixture.w)?,
            "--model",
            "claude-fixture",
            "--version",
            "2.1.283",
            "--out",
            text(&out)?,
        ])
        .env("CLAUDE_CONFIG_DIR", &other)
        .env("HOME", &fixture.h)
        .output()?;
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let records = fixture.records()?;
    assert_eq!(records.len(), 1);
    let record = &records[0].1;
    assert_eq!(record.documents.len(), 5);
    let kinds: Vec<&str> = record.documents.iter().map(|d| d.kind.name()).collect();
    assert_eq!(
        kinds,
        [
            "user_claude_md",
            "appended_instructions",
            "mcp_config",
            "claude_md_chain",
            "memory_index"
        ]
    );
    assert_eq!(record.documents[0].path, home_config.join("CLAUDE.md"));
    assert_eq!(
        record.documents[4].path,
        Fixture::memory_index(&home_config, &fixture.w)?
    );
    assert_eq!(
        record
            .documents
            .iter()
            .filter(|d| d.path.starts_with(&other))
            .count(),
        0
    );
    let data = record.data()?;
    assert_eq!(
        data["config_dir"],
        json!({"path": text(&home_config)?, "source": "home"})
    );
    assert_eq!(
        data["environment"],
        json!(["LYS_FIXTURE_MODE", "LYS_FIXTURE_TOKEN"])
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn an_unreadable_claude_md_fails_the_render_by_path_with_no_given_entry() -> Outcome {
    use std::os::unix::fs::PermissionsExt;
    let mut fixture = Fixture::new()?;
    rendered(&mut fixture)?;
    let claude_md = fixture.claude_md();
    std::fs::set_permissions(&claude_md, std::fs::Permissions::from_mode(0o000))?;
    let (_, output) = fixture.render()?;
    std::fs::set_permissions(&claude_md, std::fs::Permissions::from_mode(0o644))?;
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stderr(&output).contains(text(&claude_md)?),
        "{}",
        stderr(&output)
    );
    assert_eq!(stderr(&output).matches(SENTENCE.trim()).count(), 0);
    assert!(output.stdout.is_empty());
    let records = fixture.records()?;
    assert_eq!(records.len(), 1);
    let session = fixture.session()?;
    assert_eq!(session.customs_everywhere("lys.harness_event")?.len(), 2);
    assert_eq!(session.head()?, Some("e4"));
    drop(session);
    Ok(())
}
