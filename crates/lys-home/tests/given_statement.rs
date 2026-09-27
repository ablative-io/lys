//! The given statement end to end through the built binary (HOME-016 R3 and
//! R4): the fixture template, with a fixture secret value in its env slot,
//! rendered over the fixture session with no key, with two test keys made
//! here in temporary directories, and with keys that must be refused. No
//! seed is committed; every loop asserts how many cases it ran.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use lys_core::attestation::{
    Attestation, verify_attestation_bytes, verify_attestation_bytes_by_signer,
};
use lys_core::{Ed25519Identity, TrustError};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use lys_home::harness::claude_code::launch::{LaunchArgs, render_launch};
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
const UUID: &str = "00000000-0000-4000-8000-000000000016";
/// The fixture secret value, set in the template's env slot.
const SECRET: &str = "fixture-secret-value-7f3a9c";
/// The text of the fixture session's first user message.
const MESSAGE: &str = "fixture turn one";
/// Bytes 0x00 to 0x1F as a seed, in hex and standard base64.
const SEED_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const SEED_BASE64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
/// Bytes 0x00 to 0x1E, a 31-byte key file, in hex and standard base64.
const SHORT_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e";
const SHORT_BASE64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHg==";
const LAUNCH_FILES: usize = 5;

type Outcome = Result<(), Box<dyn std::error::Error>>;
type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn text(path: &Path) -> Fallible<&str> {
    Ok(path.to_str().ok_or("a UTF-8 path")?)
}

fn report(output: &Output) -> Fallible<Value> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn file_count(dir: &Path) -> Fallible<usize> {
    Ok(std::fs::read_dir(dir)?.count())
}

/// A home holding the fixture session, a config directory `c`, a HOME `h`,
/// and the fixture template with `c` and the secret in its env slot.
struct Fixture {
    dir: tempfile::TempDir,
    home: PathBuf,
    h: PathBuf,
    template: PathBuf,
    outs: usize,
}

impl Fixture {
    fn new() -> Fallible<Self> {
        let dir = tempfile::tempdir()?;
        let home = dir.path().join("home");
        Home::open(&home)?;
        std::fs::copy(SESSION, home.join("sessions").join("fixture.jsonl"))?;
        let c = dir.path().join("c");
        let h = dir.path().join("h");
        for path in [&c, &h, &dir.path().join("w")] {
            std::fs::create_dir_all(path)?;
        }
        let mut template: Value = serde_json::from_slice(&std::fs::read(TEMPLATE)?)?;
        template["slots"]["env"]["CLAUDE_CONFIG_DIR"] = json!(text(&c)?);
        template["slots"]["env"]["LYS_FIXTURE_SECRET"] = json!(SECRET);
        let path = dir.path().join("template.json");
        std::fs::write(&path, serde_json::to_vec(&template)?)?;
        Ok(Self {
            dir,
            home,
            h,
            template: path,
            outs: 0,
        })
    }

    fn session_file(&self) -> PathBuf {
        self.home.join("sessions").join("fixture.jsonl")
    }

    /// A fresh, empty out directory.
    fn out(&mut self) -> Fallible<PathBuf> {
        self.outs += 1;
        let out = self.dir.path().join(format!("out{}", self.outs));
        std::fs::create_dir(&out)?;
        Ok(out)
    }

    fn args(&self, out: &Path, key: Option<&Path>) -> LaunchArgs {
        LaunchArgs {
            home: self.home.clone(),
            session: "fixture".to_owned(),
            template: self.template.clone(),
            uuid: UUID.to_owned(),
            cwd: self.dir.path().join("w").display().to_string(),
            model: "claude-fixture".to_owned(),
            version: "2.1.283".to_owned(),
            out: out.to_path_buf(),
            key: key.map(Path::to_path_buf),
        }
    }

    fn render_into(&self, out: &Path, key: Option<&Path>) -> Fallible<Output> {
        let a = self.args(out, key);
        let mut command = Command::new(BIN);
        command.args([
            "render-launch",
            "--home",
            text(&a.home)?,
            "--session",
            &a.session,
        ]);
        command.args([
            "--template",
            text(&a.template)?,
            "--uuid",
            &a.uuid,
            "--cwd",
            &a.cwd,
        ]);
        command.args([
            "--model",
            &a.model,
            "--version",
            &a.version,
            "--out",
            text(out)?,
        ]);
        if let Some(key) = key {
            command.args(["--key", text(key)?]);
        }
        Ok(command
            .env_remove("CLAUDE_CONFIG_DIR")
            .env("HOME", &self.h)
            .output()?)
    }

    fn render(&mut self, key: Option<&Path>) -> Fallible<(PathBuf, Output)> {
        let out = self.out()?;
        let output = self.render_into(&out, key)?;
        Ok((out, output))
    }

    /// Every entry line of the session file of `custom_type`, parsed.
    fn customs(&self, custom_type: &str) -> Fallible<Vec<(String, Value)>> {
        let text = std::fs::read_to_string(self.session_file())?;
        let mut found = Vec::new();
        for line in text.lines() {
            let value: Value = serde_json::from_str(line)?;
            if value["customType"] == custom_type {
                found.push((line.to_owned(), value));
            }
        }
        Ok(found)
    }
}

/// A test key made here by lys-core in its own temporary directory, which
/// lives as long as the key.
struct TestKey {
    dir: tempfile::TempDir,
    public: [u8; 32],
}

impl TestKey {
    fn new() -> Fallible<Self> {
        let dir = tempfile::tempdir()?;
        let public =
            Ed25519Identity::load_or_generate(&dir.path().join("test.key"))?.public_key_bytes();
        Ok(Self { dir, public })
    }

    fn path(&self) -> PathBuf {
        self.dir.path().join("test.key")
    }
}

#[test]
fn a_render_without_a_key_is_unsigned_and_writes_no_statement() -> Outcome {
    let mut fixture = Fixture::new()?;
    let (out, output) = fixture.render(None)?;
    let report = report(&output)?;
    assert_eq!(report["signing"], "unsigned");
    let given = report["given_sha256"].as_str().ok_or("given_sha256")?;
    assert_eq!(given.len(), 64);
    assert!(
        given
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    for member in ["statement", "statement_file", "payload_file"] {
        assert!(report.get(member).is_none(), "{member}");
    }
    assert_eq!(file_count(&out)?, LAUNCH_FILES);
    assert!(fixture.customs("lys.given_statement")?.is_empty());
    Ok(())
}

#[test]
fn a_keyed_render_is_signed_and_its_record_and_event_match_an_unkeyed_one() -> Outcome {
    let mut fixture = Fixture::new()?;
    let key = TestKey::new()?;
    let (out, unkeyed) = fixture.render(None)?;
    report(&unkeyed)?;
    std::fs::remove_dir_all(&out)?;
    std::fs::create_dir(&out)?;
    let head_before = Session::open(fixture.session_file())?.head_hash()?;
    let output = fixture.render_into(&out, Some(&key.path()))?;
    let report = report(&output)?;
    assert_eq!(
        Session::open(fixture.session_file())?.head_hash()?,
        head_before
    );
    assert_eq!(report["signing"], "signed");
    assert_eq!(file_count(&out)?, LAUNCH_FILES + 2);
    let statements = fixture.customs("lys.given_statement")?;
    assert_eq!(statements.len(), 1);
    assert_eq!(statements[0].1["id"], report["statement"]);

    let given = report["given_sha256"].as_str().ok_or("given_sha256")?;
    assert_eq!(
        hex(&Sha256::digest(std::fs::read(out.join("given-data.json"))?)),
        given
    );
    for (member, name) in [
        ("statement_file", "given-statement.cose"),
        ("payload_file", "given-data.json"),
    ] {
        let path = Path::new(report[member].as_str().ok_or("a path")?);
        assert!(path.is_absolute() && path.ends_with(name), "{member}");
    }

    let records = fixture.customs("lys.given")?;
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].1["data"], records[1].1["data"]);
    let events = fixture.customs("lys.harness_event")?;
    assert_eq!(events.len(), 2);
    let keys = |v: &Value| {
        v["data"]
            .as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>())
    };
    assert_eq!(keys(&events[0].1), keys(&events[1].1));
    for (line, _) in &events {
        assert!(!line.contains(given));
    }
    Ok(())
}

#[test]
fn a_short_key_file_is_refused_by_path_before_anything_is_written() -> Outcome {
    let mut fixture = Fixture::new()?;
    let key = fixture.dir.path().join("short.key");
    std::fs::write(&key, (0u8..31).collect::<Vec<_>>())?;
    let session_before = std::fs::read(fixture.session_file())?;
    let (out, output) = fixture.render(Some(&key))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(text(&key)?));
    assert_eq!(file_count(&out)?, 0);
    assert_eq!(std::fs::read(fixture.session_file())?, session_before);

    let error = render_launch(&fixture.args(&out, Some(&key)))
        .err()
        .ok_or("a refusal")?;
    let debug = format!("{error:?}");
    assert_eq!(hex(&(0u8..31).collect::<Vec<_>>()), SHORT_HEX);
    assert!(!debug.contains(SHORT_HEX) && !debug.contains(SHORT_BASE64));
    Ok(())
}

#[test]
fn an_existing_statement_file_is_refused_by_path_and_nothing_is_written() -> Outcome {
    let mut fixture = Fixture::new()?;
    let key = TestKey::new()?;
    let out = fixture.out()?;
    let existing = out.join("given-statement.cose");
    std::fs::write(&existing, b"")?;
    let session_before = std::fs::read(fixture.session_file())?;
    let output = fixture.render_into(&out, Some(&key.path()))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(text(&existing)?));
    assert_eq!(file_count(&out)?, 1);
    assert_eq!(std::fs::read(fixture.session_file())?, session_before);
    Ok(())
}

#[test]
fn no_seed_byte_reaches_the_report_or_stderr() -> Outcome {
    let mut fixture = Fixture::new()?;
    let key = fixture.dir.path().join("counting.key");
    let seed: Vec<u8> = (0u8..32).collect();
    assert_eq!(hex(&seed), SEED_HEX);
    std::fs::write(&key, &seed)?;
    let (_, output) = fixture.render(Some(&key))?;
    assert!(output.status.success());
    for stream in [&output.stdout, &output.stderr] {
        let stream = String::from_utf8_lossy(stream);
        assert!(!stream.contains(SEED_HEX) && !stream.contains(SEED_BASE64));
    }
    Ok(())
}

#[test]
fn the_statement_verifies_offline_names_its_signer_and_carries_no_secret() -> Outcome {
    let mut fixture = Fixture::new()?;
    let (key_a, key_b) = (TestKey::new()?, TestKey::new()?);
    let (public_a, public_b) = (key_a.public, key_b.public);
    let (out_a, output_a) = fixture.render(Some(&key_a.path()))?;
    let report_a = report(&output_a)?;
    let (out_b, output_b) = fixture.render(Some(&key_b.path()))?;
    report(&output_b)?;
    let cose_a = std::fs::read(out_a.join("given-statement.cose"))?;
    let data_a = std::fs::read(out_a.join("given-data.json"))?;
    let cose_b = std::fs::read(out_b.join("given-statement.cose"))?;
    let data_b = std::fs::read(out_b.join("given-data.json"))?;
    verify_attestation_bytes(&cose_a, &data_a)?;

    let mut flipped = 0;
    for offset in 0..cose_a.len() {
        let mut altered = cose_a.clone();
        altered[offset] ^= 1;
        let refused = verify_attestation_bytes(&altered, &data_a);
        assert!(
            matches!(refused, Err(TrustError::InvalidSignature)),
            "offset {offset}"
        );
        flipped += 1;
    }
    assert_eq!(flipped, cose_a.len());
    assert!((191..=199).contains(&cose_a.len()), "{}", cose_a.len());

    verify_attestation_bytes_by_signer(&cose_a, &data_a, &public_a)?;
    assert!(verify_attestation_bytes_by_signer(&cose_b, &data_b, &public_a).is_err());
    assert!(verify_attestation_bytes_by_signer(&cose_a, &data_a, &public_b).is_err());

    let given = report_a["given_sha256"].as_str().ok_or("given_sha256")?;
    assert_eq!(
        hex(&Attestation::from_cose_bytes(&cose_a)?.payload_hash),
        given
    );
    let records = fixture.customs("lys.given")?;
    assert_eq!(records.len(), 2);
    let recomputed = hex(&Sha256::digest(serde_json::to_vec(&records[0].1["data"])?));
    assert_eq!(recomputed, given);

    let statements = fixture.customs("lys.given_statement")?;
    assert_eq!(statements.len(), 2);
    let block_hash = statements[0].1["data"]["statement"]
        .as_str()
        .ok_or("a hash")?;
    let block = Home::open(&fixture.home)?
        .blocks()?
        .get(&lys_home::Hash::parse(block_hash)?)?;
    let searched: [&[u8]; 4] = [&cose_a, &data_a, &block, statements[0].0.as_bytes()];
    let mut found = 0;
    for bytes in searched {
        for needle in [SECRET, MESSAGE] {
            found += bytes
                .windows(needle.len())
                .filter(|window| *window == needle.as_bytes())
                .count();
        }
    }
    assert_eq!(searched.len(), 4);
    assert_eq!(found, 0);
    Ok(())
}
