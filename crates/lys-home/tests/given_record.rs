#![cfg(test)]
//! The context record end to end through the built binary (HOME-003 R6):
//! the fixture template rendered for a fixture working directory holding a
//! CLAUDE.md, with a user CLAUDE.md and a memory index under the fixture
//! config directory, HOME a fresh directory and `CLAUDE_CONFIG_DIR` removed
//! from the rendering process's environment, so nothing here depends on the
//! machine's own configuration. Every assertion over a documents list first counts it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use lys_home::harness::claude_code::given::{DocumentKind, GivenDocument};
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
/// The one line of the fixture user CLAUDE.md in the config directory.
const USER_SENTENCE: &str =
    "Fixture user file: the record lists it after the render's two files.\n";
const MEMORY: &str = "# Fixture memory index\n";
const KINDS: [&str; 5] = [
    "appended_instructions",
    "mcp_config",
    "user_claude_md",
    "claude_md_chain",
    "memory_index",
];

type Outcome = Result<(), Box<dyn std::error::Error>>;
type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

#[path = "given_record/refusals.rs"]
mod refusals;

/// Held by every test for its whole body. A child spawned by one test thread
/// inherits every open descriptor of this process between its fork and its
/// exec, so a session lock another test thread held and just dropped stays
/// held by that child for the moment, and the next open of the session is
/// refused as `SessionHeld`. The lock is advisory and per open file, which is
/// what the record promises; so these tests, which both hold sessions in this
/// process and spawn the binary, run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

/// Take the fixture serialisation lock; interrupted fixture state refuses reuse.
fn serially() -> MutexGuard<'static, ()> {
    SERIAL.lock().expect("test state lock poisoned")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            write!(s, "{b:02x}").expect("writing to a String cannot fail");
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
/// with the fixture CLAUDE.md, a config directory `c` with a user CLAUDE.md
/// and the memory index for `w`, a HOME `h` with no `.claude`, and the
/// fixture template with `CLAUDE_CONFIG_DIR` set to `c` in its env slot.
/// Every path is built from the temporary directory canonicalised, `base`.
struct Fixture {
    dir: tempfile::TempDir,
    base: PathBuf,
    home: PathBuf,
    w: PathBuf,
    c: PathBuf,
    h: PathBuf,
    template: PathBuf,
    renders: usize,
}

impl Fixture {
    fn new() -> Fallible<Self> {
        Self::build(false)
    }

    /// The layout of HOME-010 R4: `w` and `c` under `base/real`, each with
    /// its CLAUDE.md, and no memory index file. The symlink `base/link` is
    /// made by [`Fixture::link`].
    fn under_real() -> Fallible<Self> {
        Self::build(true)
    }

    fn build(under_real: bool) -> Fallible<Self> {
        let dir = tempfile::tempdir()?;
        let base = dir.path().canonicalize()?;
        let home = base.join("home");
        Home::open(&home)?;
        std::fs::copy(SESSION, home.join("sessions").join("fixture.jsonl"))?;
        let root = if under_real {
            base.join("real")
        } else {
            base.clone()
        };
        let w = root.join("w");
        let c = root.join("c");
        let h = base.join("h");
        put(&w.join("CLAUDE.md"), SENTENCE.as_bytes())?;
        put(&c.join("CLAUDE.md"), USER_SENTENCE.as_bytes())?;
        if !under_real {
            put(&Self::memory_index(&c, &w)?, MEMORY.as_bytes())?;
        }
        std::fs::create_dir_all(&c)?;
        std::fs::create_dir_all(&h)?;
        let template_path = base.join("template.json");
        let fixture = Self {
            dir,
            base,
            home,
            w,
            c,
            h,
            template: template_path,
            renders: 0,
        };
        fixture.set_config(text(&fixture.c)?)?;
        Ok(fixture)
    }

    /// Write the fixture template with `CLAUDE_CONFIG_DIR` set to `config`.
    fn set_config(&self, config: &str) -> Fallible<()> {
        let mut template: Value = serde_json::from_slice(&std::fs::read(TEMPLATE)?)?;
        template["slots"]["env"]["CLAUDE_CONFIG_DIR"] = json!(config);
        std::fs::write(&self.template, serde_json::to_vec_pretty(&template)?)?;
        Ok(())
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
        let cwd = text(&self.w)?.to_owned();
        self.render_in(&cwd)
    }

    /// One render as [`Fixture::render`] with `cwd` as `--cwd`.
    fn render_in(&mut self, cwd: &str) -> Fallible<(PathBuf, Output)> {
        self.renders += 1;
        let out = self.base.join(format!("out{}", self.renders));
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
                cwd,
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
fn the_first_render_records_five_documents_in_the_request_s_order_under_the_render_event() -> Outcome
{
    let serial = serially();
    let mut fixture = Fixture::new()?;
    let (out, report) = rendered(&mut fixture)?;
    let records = fixture.records()?;
    assert_eq!(records.len(), 1);
    let (id, record) = &records[0];
    assert_eq!(report["given"], json!(id));
    assert_eq!(report["given_documents"], 5);
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
    assert_eq!(documents.len(), 5);
    let kinds: Vec<&str> = documents.iter().map(|d| d.kind.name()).collect();
    assert_eq!(kinds, KINDS);
    let on_disk = [
        out.join("instructions.md"),
        out.join("mcp.json"),
        fixture.c.join("CLAUDE.md"),
        fixture.claude_md(),
        Fixture::memory_index(&fixture.c, &fixture.w)?,
    ];
    assert_eq!(on_disk.len(), documents.len());
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
    assert_eq!(documents[2].path, fixture.c.join("CLAUDE.md"));
    assert_eq!(documents[2].length, USER_SENTENCE.len() as u64);
    assert_eq!(documents[2].sha256, sha256_hex(USER_SENTENCE.as_bytes()));
    assert_eq!(documents[3].path, fixture.claude_md());
    assert!(documents[4].path.starts_with(&fixture.c));
    let data = record.data()?;
    assert_eq!(
        data["config_dir"],
        json!({"path": text(&fixture.c)?, "source": "template"})
    );
    assert_eq!(
        data["environment"],
        json!(["CLAUDE_CONFIG_DIR", "LYS_FIXTURE_MODE", "LYS_FIXTURE_TOKEN"])
    );
    assert_eq!(record.harness, "claude-code");
    assert_eq!(record.harness_version, "2.1.283");
    drop(serial);
    Ok(())
}

#[test]
fn two_renders_give_equal_lists_and_one_changed_byte_changes_exactly_one_hash() -> Outcome {
    let serial = serially();
    let mut fixture = Fixture::new()?;
    rendered(&mut fixture)?;
    rendered(&mut fixture)?;
    let records = fixture.records()?;
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].1.documents.len(), 5);
    assert_eq!(records[1].1.documents.len(), 5);
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
    assert_eq!(before.documents.len(), 5);
    assert_eq!(after.documents.len(), 5);
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
    assert_eq!(after.documents[3].kind.name(), "claude_md_chain");
    assert_eq!(after.documents[3].sha256, sha256_hex(&bytes));
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
            5
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
    drop(serial);
    Ok(())
}

impl Fixture {
    /// `base/real`, and `base/link` made here as a symlink to it.
    #[cfg(unix)]
    fn link(&self) -> Fallible<(PathBuf, PathBuf)> {
        let real = self.base.join("real");
        let link = self.base.join("link");
        std::os::unix::fs::symlink(&real, &link)?;
        Ok((real, link))
    }

    /// A render in `cwd` under the config directory `config` that must
    /// succeed: its report and the given entry's data as the session file
    /// holds it.
    fn given_in(&mut self, cwd: &str, config: &str) -> Fallible<(Value, Value)> {
        self.set_config(config)?;
        let (_, output) = self.render_in(cwd)?;
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        let report = stdout_json(&output)?;
        let id = report["given"].as_str().ok_or("an id")?;
        let mut found = Vec::new();
        for line in self.lines()? {
            let entry: Value = serde_json::from_str(&line)?;
            if entry["id"] == id {
                found.push(entry["data"].clone());
            }
        }
        assert_eq!(found.len(), 1);
        let data = found.pop().ok_or("one entry")?;
        Ok((report, data))
    }

    /// Append `record` as a `lys.given` entry under the render event `event`,
    /// as a record written by an earlier build holds it; returns its id.
    fn append(&self, record: &GivenRecord, event: &Value) -> Fallible<String> {
        let mut session = self.session()?;
        let parent = event.as_str().ok_or("an id")?;
        let id = record.append_under(&mut session, parent)?;
        drop(session);
        Ok(id)
    }
}

/// A given-check that must exit 0 answering `matches`: its report.
fn matched(output: &Output) -> Fallible<Value> {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    let report = stdout_json(output)?;
    assert_eq!(report["answer"], "matches");
    Ok(report)
}

/// A given-check that must be refused with status 2 and nothing on stdout,
/// its stderr naming every one of `named`.
fn refused(output: &Output, named: &[&str]) {
    assert_eq!(output.status.code(), Some(2), "{}", stderr(output));
    assert!(output.stdout.is_empty());
    for name in named {
        assert!(stderr(output).contains(name), "{}", stderr(output));
    }
}

fn document(path: &Path, bytes: &[u8]) -> GivenDocument {
    GivenDocument {
        kind: DocumentKind::ClaudeMdChain,
        path: path.to_path_buf(),
        length: bytes.len() as u64,
        sha256: sha256_hex(bytes),
    }
}

#[cfg(unix)]
#[test]
fn three_shapes_of_one_directory_record_and_compare_as_one_path() -> Outcome {
    let serial = serially();
    let mut fixture = Fixture::under_real()?;
    let (real, link) = fixture.link()?;
    let (w, c) = (text(&fixture.w)?.to_owned(), text(&fixture.c)?.to_owned());
    let (reference_report, reference) = fixture.given_in(&w, &c)?;
    assert_eq!(reference["config_dir"]["path"], json!(c));
    let documents = reference["documents"].as_array().ok_or("a list")?;
    assert_eq!(documents.len(), 4);
    let dotted_w = real.join("w").join("..").join("w");
    let dotted_c = real.join("c").join("..").join("c");
    let shapes = [
        (link.join("w"), text(&link.join("c"))?.to_owned()),
        (dotted_w, text(&dotted_c)?.to_owned()),
        (real.join("w"), format!("{c}/")),
    ];
    let mut compared = 0;
    for (cwd, config) in &shapes {
        let (report, data) = fixture.given_in(text(cwd)?, config)?;
        assert_ne!(report["given"], reference_report["given"]);
        for key in ["config_dir", "documents"] {
            assert_eq!(
                serde_json::to_vec(&data[key])?,
                serde_json::to_vec(&reference[key])?,
                "{key} of --cwd {} and CLAUDE_CONFIG_DIR {config}",
                cwd.display()
            );
        }
        compared += 1;
    }
    assert_eq!(compared, 3);
    let entry = reference_report["given"].as_str().ok_or("an id")?;
    let claude_md = fixture.claude_md();
    let mut checks = 0;
    for path in [
        link.join("w").join("CLAUDE.md"),
        real.join("w").join("..").join("w").join("CLAUDE.md"),
    ] {
        let path = text(&path)?;
        let report = matched(&fixture.check(entry, path, Some(&claude_md))?)?;
        assert_eq!(report["path"], json!(path));
        checks += 1;
    }
    assert_eq!(checks, 2);
    drop(serial);
    Ok(())
}

#[cfg(unix)]
#[test]
fn given_check_compares_canonical_paths_and_names_what_is_unresolved() -> Outcome {
    let serial = serially();
    let mut fixture = Fixture::under_real()?;
    let (real, link) = fixture.link()?;
    let (w, c) = (text(&fixture.w)?.to_owned(), text(&fixture.c)?.to_owned());
    let report = fixture.given_in(&w, &c)?.0;
    let entry = report["given"].as_str().ok_or("an id")?.to_owned();
    let claude_md = fixture.claude_md();
    let file = Some(claude_md.as_path());

    let through_link = link.join("w").join("CLAUDE.md");
    let linked_path = text(&through_link)?;
    let linked = matched(&fixture.check(&entry, linked_path, file)?)?;
    assert_eq!(linked["path"], json!(linked_path));
    assert_eq!(linked["unresolved"], json!([]));

    let dotted = real.join("w").join("..").join("w").join("CLAUDE.md");
    matched(&fixture.check(&entry, text(&dotted)?, file)?)?;

    let records = fixture.records()?;
    assert_eq!(records.len(), 1);
    let mut earlier = records[0].1.clone();
    earlier.documents = vec![document(&through_link, SENTENCE.as_bytes())];
    let earlier_id = fixture.append(&earlier, &report["event"])?;
    matched(&fixture.check(&earlier_id, text(&claude_md)?, file)?)?;

    let gone_dir = fixture.base.join("gone");
    let gone = gone_dir.join("CLAUDE.md");
    let copy = real.join("copy.md");
    put(&copy, SENTENCE.as_bytes())?;
    let mut absent = records[0].1.clone();
    absent.documents = vec![document(&gone, SENTENCE.as_bytes())];
    let absent_id = fixture.append(&absent, &report["event"])?;
    let kept = matched(&fixture.check(&absent_id, text(&gone)?, Some(&copy))?)?;
    assert_eq!(kept["unresolved"], json!([text(&gone)?]));
    let gone_dotted = gone_dir.join("..").join("gone").join("CLAUDE.md");
    let gone_dotted = text(&gone_dotted)?;
    let dotted_gone = fixture.check(&absent_id, gone_dotted, Some(&copy))?;
    refused(&dotted_gone, &[gone_dotted]);

    let written = fixture.base.join("out1").join("instructions.md");
    let written_file = Some(written.as_path());
    matched(&fixture.check(&entry, "instructions.md", written_file)?)?;

    let mut keys_checked = 0;
    for raw in fixture.lines()? {
        let parsed: Value = serde_json::from_str(&raw)?;
        if parsed["id"] == json!(entry) {
            let data = parsed["data"].as_object().ok_or("an object")?;
            let mut keys: Vec<&String> = data.keys().collect();
            keys.sort();
            assert_eq!(
                keys,
                [
                    "config_dir",
                    "documents",
                    "environment",
                    "harness",
                    "harness_version",
                    "kinds"
                ]
            );
            keys_checked += 1;
        }
    }
    assert_eq!(keys_checked, 1);
    drop(serial);
    Ok(())
}

#[cfg(unix)]
#[test]
fn given_names_each_record_s_unresolved_paths_as_recorded() -> Outcome {
    let serial = serially();
    let mut fixture = Fixture::under_real()?;
    fixture.link()?;
    let (w, c) = (text(&fixture.w)?.to_owned(), text(&fixture.c)?.to_owned());
    let gone = format!("{}/", text(&fixture.base.join("gone").join("c"))?);
    let report = fixture.given_in(&w, &gone)?.0;
    fixture.given_in(&w, &c)?;
    let records = fixture.records()?;
    assert_eq!(records.len(), 2);
    let absent = fixture.w.join("absent.md");
    let mut listing_absent = records[1].1.clone();
    let extra = document(&absent, b"absent\n");
    listing_absent.documents.push(extra);
    fixture.append(&listing_absent, &report["event"])?;
    let given = fixture.given()?;
    assert_eq!(given.status.code(), Some(0), "{}", stderr(&given));
    let given_report = stdout_json(&given)?;
    let listed = given_report["records"].as_array().ok_or("a list")?;
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0]["config_dir"]["path"], json!(gone));
    assert_eq!(listed[0]["unresolved"], json!([gone]));
    assert_eq!(listed[1]["unresolved"], json!([]));
    assert_eq!(listed[2]["unresolved"], json!([text(&absent)?]));
    drop(serial);
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_path_that_cannot_be_searched_is_refused_by_render_launch_and_given_check() -> Outcome {
    use std::os::unix::fs::PermissionsExt;

    let serial = serially();
    let mut fixture = Fixture::under_real()?;
    fixture.link()?;
    let locked = fixture.base.join("locked");
    let locked_w = locked.join("w");
    let behind = locked_w.join("CLAUDE.md");
    put(&behind, SENTENCE.as_bytes())?;
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))?;
    let rendered = fixture.render_in(text(&locked_w)?);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))?;
    let output = rendered?.1;
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let shown = stderr(&output);
    assert!(shown.contains("canonicalising a given path"), "{shown}");
    assert!(shown.contains(text(&locked_w)?), "{shown}");
    assert!(output.stdout.is_empty());
    assert_eq!(fixture.records()?.len(), 0);

    let (w, c) = (text(&fixture.w)?.to_owned(), text(&fixture.c)?.to_owned());
    let report = fixture.given_in(&w, &c)?.0;
    let entry = report["given"].as_str().ok_or("an id")?.to_owned();
    let claude_md = fixture.claude_md();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))?;
    let checked = fixture.check(&entry, text(&behind)?, Some(&claude_md));
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))?;
    let named = ["canonicalising a given path", text(&behind)?];
    refused(&checked?, &named);
    drop(serial);
    Ok(())
}
