#![cfg(test)]
//! Fixture evidence cannot authorise a qualification pin for the runner's
//! adapters. Each test drives `scripts/qualify` (`qual.sh` stage a, and
//! `judge.py`) against the stored qualifier fixtures, case for case as the
//! retired `scripts/qualify/test_qualify.py` did, under the same names.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const ADAPTERS: [&str; 2] = ["claude-code", "codex"];
const PYTHON: &str = "/usr/bin/python3";
const DISK: &str = r"#!/bin/sh
printf 'Filesystem Blocks Used Available Capacity Mounted\nfixture 100 0 100 0%% /\n'
";
const EXAMPLE: &str = r#"import json
from pathlib import Path
import sys

arguments = sys.argv[1:]
fields = dict(zip(arguments[::2], arguments[1::2]))
if len(arguments) != 6 or set(fields) != {"--adapter", "--directory", "--evidence"}:
    raise RuntimeError("fixture_arguments_invalid")
adapter = fields["--adapter"]
root = Path(__file__).resolve().parent
controls = json.loads((root / "controls.json").read_text())
if adapter not in controls["fixtures"]:
    raise RuntimeError("fixture_adapter_invalid")
empty = not any(Path(fields["--directory"]).iterdir())
if not empty:
    raise RuntimeError("fixture_directory_not_empty")
with (root / "calls.jsonl").open("a") as calls:
    calls.write(json.dumps({"adapter": adapter, "directory_empty": empty}) + "\n")
records = controls["records"]
records[0]["fixture"] = controls["fixtures"][adapter]
with Path(fields["--evidence"]).open("x") as evidence:
    for record in records:
        evidence.write(json.dumps(record) + "\n")
"#;

/// The repository root, `scripts/qualify` and the qualifier fixtures.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scripts() -> PathBuf {
    root().join("scripts/qualify")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/qualifier")
}

fn executable(path: &Path, text: &str) -> Result<(), Box<dyn Error>> {
    fs::write(path, text)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").expect("writing to a String cannot fail");
        out
    })
}

/// `{step: {field: value}}`: one step's record with one field overwritten.
fn marker(step: &str, field: &str, value: Value) -> Value {
    let mut fields = serde_json::Map::new();
    fields.insert(field.to_owned(), value);
    let mut steps = serde_json::Map::new();
    steps.insert(step.to_owned(), Value::Object(fields));
    Value::Object(steps)
}

/// What one stage-a run left behind.
struct Staged {
    stdout: String,
    stderr: String,
    pins: Vec<u8>,
    evidence: BTreeMap<&'static str, Vec<u8>>,
}

impl Staged {
    /// The run's one `END ` line.
    fn end(&self) -> Result<&str, Box<dyn Error>> {
        let lines: Vec<&str> = self
            .stdout
            .lines()
            .filter(|line| line.starts_with("END "))
            .collect();
        assert_eq!(lines.len(), 1, "{}{}", self.stdout, self.stderr);
        Ok(lines[0])
    }

    /// The first evidence record of `adapter`.
    fn first(&self, adapter: &str) -> Result<Value, Box<dyn Error>> {
        let evidence = self.evidence.get(adapter).ok_or("no evidence")?;
        let line = evidence
            .split(|byte| *byte == b'\n')
            .next()
            .ok_or("empty")?;
        Ok(serde_json::from_slice(line)?)
    }

    fn assert_pins(&self, adapters: &[&str]) -> TestResult {
        let pins = String::from_utf8(self.pins.clone())?;
        let pins: Vec<Vec<&str>> = pins
            .lines()
            .map(|line| line.split_whitespace().collect())
            .collect();
        assert_eq!(pins.len(), adapters.len());
        for (pin, adapter) in pins.iter().zip(adapters) {
            assert_eq!(pin.len(), 4);
            let evidence = self.evidence.get(adapter).ok_or("no evidence")?;
            let digest = hex(&Sha256::digest(evidence));
            assert_eq!(pin[..3], [*adapter, "0.1.2", digest.as_str()]);
            let name = Path::new(pin[3]).file_name().ok_or("no evidence name")?;
            assert_eq!(name.to_string_lossy(), format!("evidence-{adapter}.jsonl"));
        }
        Ok(())
    }

    /// Refused with no pins: `proved=0`, each adapter failing for `reason`.
    fn refused(&self, reason: &str) -> TestResult {
        assert_eq!(self.pins, b"");
        let end = self.end()?;
        assert!(end.contains("result=FAIL proved=0"), "{end}");
        for adapter in ADAPTERS {
            assert!(end.contains(&format!("{adapter}:{reason}")), "{end}");
        }
        Ok(())
    }
}

/// Run `qual.sh a` with each adapter's example marked `fixtures[adapter]`
/// and each step's record in `markers` overwritten with its fields.
fn stage_a(fixtures_of: &Value, markers: &Value) -> Result<Staged, Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let root = fs::canonicalize(directory.path())?;
    let tools = root.join("tools");
    fs::create_dir(&tools)?;
    executable(&tools.join("df"), DISK)?;
    let stored = fs::read_to_string(fixtures().join("fixture-pass.jsonl"))?;
    let mut records = stored
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<Value>, _>>()?;
    for (step, fields) in markers.as_object().ok_or("markers are an object")? {
        let record = records
            .iter_mut()
            .find(|record| record["step"] == step.as_str())
            .ok_or("no record of the step")?;
        for (field, value) in fields.as_object().ok_or("fields are an object")? {
            record[field] = value.clone();
        }
    }
    let controls = json!({"fixtures": fixtures_of, "records": records});
    fs::write(root.join("controls.json"), serde_json::to_vec(&controls)?)?;
    let helper = root.join("example.py");
    fs::write(&helper, EXAMPLE)?;
    let helper = helper.to_str().ok_or("a path in UTF-8")?;
    assert!(!helper.contains('\''), "the helper path quotes plainly");
    let example = root.join("example");
    executable(
        &example,
        &format!("#!/bin/sh\nexec {PYTHON} -B '{helper}' \"$@\"\n"),
    )?;
    let output = root.join("output");
    fs::create_dir(&output)?;
    fs::write(
        output.join("versions-stage-a.txt"),
        "claude-code 0.1.2\ncodex 0.1.2\n",
    )?;
    let completed = Command::new("/bin/bash")
        .arg(scripts().join("qual.sh"))
        .args(["a", "--out"])
        .arg(&output)
        .arg("--example")
        .arg(&example)
        .current_dir(&root)
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", tools.display()))
        .env("LC_ALL", "C")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()?;
    let stdout = String::from_utf8(completed.stdout)?;
    let stderr = String::from_utf8(completed.stderr)?;
    assert_eq!(completed.status.code(), Some(0), "{stdout}{stderr}");
    let stage = output.join("stage-a");
    let mut evidence = BTreeMap::new();
    for adapter in ADAPTERS {
        evidence.insert(
            adapter,
            fs::read(stage.join(format!("evidence-{adapter}.jsonl")))?,
        );
    }
    let calls = fs::read_to_string(root.join("calls.jsonl"))?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<Value>, _>>()?;
    let expected: Vec<Value> = ADAPTERS
        .iter()
        .map(|adapter| json!({"adapter": adapter, "directory_empty": true}))
        .collect();
    assert_eq!(calls, expected);
    Ok(Staged {
        stdout,
        stderr,
        pins: fs::read(stage.join("pins.txt"))?,
        evidence,
    })
}

/// Each adapter marked `marker`.
fn every(marker: &Value) -> Value {
    json!({"claude-code": marker, "codex": marker})
}

/// `value` is exactly the JSON integer `number`, never a boolean or a float.
fn integer(value: &Value, number: u64) {
    assert!(value.is_u64(), "{value} is an integer");
    assert_eq!(value, &json!(number));
}

#[test]
fn test_fixture_evidence_is_refused_by_name_and_leaves_no_pins() -> TestResult {
    let result = stage_a(&every(&json!(true)), &json!({}))?;
    result.refused("fixture-evidence-is-never-a-pin")?;
    for adapter in ADAPTERS {
        assert_eq!(result.first(adapter)?["fixture"], json!(true));
    }
    Ok(())
}

#[test]
fn test_real_marked_control_pins_both_exact_evidence_digests() -> TestResult {
    let result = stage_a(&every(&json!(false)), &json!({}))?;
    let end = result.end()?;
    assert!(end.contains("result=PASS proved=2 failed=[]"), "{end}");
    result.assert_pins(&ADAPTERS)?;
    for adapter in ADAPTERS {
        assert_eq!(result.first(adapter)?["fixture"], json!(false));
    }
    Ok(())
}

#[test]
fn test_a_fixture_partner_cannot_supply_a_second_pin() -> TestResult {
    let result = stage_a(&json!({"claude-code": false, "codex": true}), &json!({}))?;
    let end = result.end()?;
    assert!(end.contains("result=FAIL proved=1"), "{end}");
    assert!(
        end.contains("codex:fixture-evidence-is-never-a-pin"),
        "{end}"
    );
    result.assert_pins(&["claude-code"])
}

/// A numeric stand-in `number` for the boolean `field`: marked on every
/// adapter (`fixture`) or on the launcher's record (`observed`).
fn numeric_marker(field: &str, number: u64) -> TestResult {
    let result = if field == "fixture" {
        stage_a(&every(&json!(number)), &json!({}))?
    } else {
        let markers = marker("launcher", field, json!(number));
        stage_a(&every(&json!(false)), &markers)?
    };
    result.refused(&format!("launcher:{field}-not-boolean"))?;
    for adapter in ADAPTERS {
        integer(&result.first(adapter)?[field], number);
    }
    Ok(())
}

#[test]
fn test_a_numeric_zero_fixture_marker_is_refused_without_pins() -> TestResult {
    numeric_marker("fixture", 0)
}

#[test]
fn test_a_numeric_one_fixture_marker_is_refused_without_pins() -> TestResult {
    numeric_marker("fixture", 1)
}

#[test]
fn test_a_numeric_zero_observed_marker_is_refused_without_pins() -> TestResult {
    numeric_marker("observed", 0)
}

#[test]
fn test_a_numeric_one_observed_marker_is_refused_without_pins() -> TestResult {
    numeric_marker("observed", 1)
}

#[test]
fn test_identity_markers_refuse_numeric_stand_ins_without_pins() -> TestResult {
    for (step, field, value, reason) in [
        (
            "launcher",
            "claims_spawn",
            0,
            "not-example-owned-or-claims-spawn",
        ),
        (
            "compaction",
            "turn_completed",
            1,
            "no-evidence-or-turn-completion",
        ),
        ("stop", "exit_observed", 1, "exit-not-observed"),
    ] {
        let result = stage_a(&every(&json!(false)), &marker(step, field, json!(value)))?;
        result
            .refused(&format!("{step}:{reason}"))
            .map_err(|error| format!("marker {field}: {error}"))?;
    }
    Ok(())
}

#[test]
fn test_stored_fixture_pass_and_failure_keep_their_named_verdicts() -> TestResult {
    for (name, code, prefix) in [
        ("fixture-pass.jsonl", 0, "FIXTURE codex 0.1.2 "),
        (
            "fail-stop-red.jsonl",
            1,
            "FAIL codex version_report not-observed:qualification_cancelled: \
             the qualification was stopped;cleanup:ok",
        ),
    ] {
        let fixture = fixtures().join(name);
        let completed = Command::new(PYTHON)
            .arg("-B")
            .arg(scripts().join("judge.py"))
            .arg("stage-a")
            .arg(&fixture)
            .arg("codex")
            .current_dir(root())
            .env_clear()
            .env("PATH", "/bin:/usr/bin")
            .env("LC_ALL", "C")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .output()?;
        let stdout = String::from_utf8(completed.stdout)?;
        let stderr = String::from_utf8(completed.stderr)?;
        assert_eq!(
            completed.status.code(),
            Some(code),
            "{name}: {stdout}{stderr}"
        );
        assert!(stdout.starts_with(prefix), "{name}: {stdout}");
        assert_eq!(stdout.lines().count(), 1, "{name}: {stdout}");
        if code == 0 {
            let digest = hex(&Sha256::digest(fs::read(&fixture)?));
            assert_eq!(stdout.trim(), format!("{prefix}{digest}"));
        }
    }
    Ok(())
}
