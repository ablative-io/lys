//! Observe an installed adapter without granting it product qualification.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use lys_runner::harness_control::{Binding, Controller, Kind, Pending, Transport, Update, process};
use lys_runner::operations::OperationState;
use serde_json::{Value, json};

#[path = "qualify_adapter/owned.rs"]
mod owned;
use owned::{Cancellation, Message, Owned, child_entry};

type Result<T> = std::result::Result<T, String>;
const STEPS: [&str; 7] = [
    "launcher",
    "version_report",
    "init",
    "reminder_admitted",
    "compaction",
    "stop",
    "terminal_writes",
];

struct Options {
    adapter: String,
    directory: PathBuf,
    evidence: PathBuf,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut fields = std::collections::BTreeMap::new();
        for pair in args.chunks(2) {
            let [key, value] = pair else {
                return Err("qualification_arguments_invalid".to_owned());
            };
            if !matches!(key.as_str(), "--adapter" | "--directory" | "--evidence")
                || value.is_empty()
                || fields.insert(key.as_str(), value).is_some()
            {
                return Err("qualification_arguments_invalid".to_owned());
            }
        }
        let adapter = (*fields
            .get("--adapter")
            .ok_or("qualification_adapter_missing")?)
        .clone();
        if !matches!(adapter.as_str(), "claude-code" | "codex") {
            return Err("qualification_adapter_invalid".to_owned());
        }
        Ok(Self {
            adapter,
            directory: PathBuf::from(
                fields
                    .get("--directory")
                    .ok_or("qualification_directory_missing")?,
            ),
            evidence: PathBuf::from(
                fields
                    .get("--evidence")
                    .ok_or("qualification_evidence_missing")?,
            ),
        })
    }
}

struct Evidence {
    file: File,
    next: usize,
    closed: bool,
}

impl Evidence {
    fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            file: OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .map_err(io_failed)?,
            next: 0,
            closed: false,
        })
    }
    fn record(&mut self, step: &str, mut detail: Value) -> Result<()> {
        if self.closed || STEPS.get(self.next).copied() != Some(step) {
            return Err("qualification_evidence_order_invalid".to_owned());
        }
        detail
            .as_object_mut()
            .ok_or("qualification_evidence_shape_invalid")?
            .insert("step".to_owned(), json!(step));
        serde_json::to_writer(&mut self.file, &detail)
            .map_err(|error| format!("qualification_evidence_failed:{error}"))?;
        self.file.write_all(b"\n").map_err(io_failed)?;
        self.next += 1;
        Ok(())
    }
    fn failed(&mut self, reason: &str) -> Result<()> {
        if let Some(step) = STEPS.get(self.next).copied() {
            self.record(step, json!({"observed":false,"reason":code(reason)}))?;
        }
        self.closed = true;
        self.file.sync_all().map_err(io_failed)
    }
}

fn code(reason: &str) -> &str {
    reason.split_once(':').map_or(reason, |(name, _)| name)
}
fn io_failed(error: impl std::fmt::Display) -> String {
    format!("qualification_io_failed:{error}")
}

fn version(bytes: &[u8], adapter: &str) -> Result<(String, String)> {
    let report = std::str::from_utf8(bytes)
        .map_err(|error| format!("qualification_version_report_invalid:{error}"))?;
    let line = report.strip_suffix('\n').unwrap_or(report);
    let line = line.strip_suffix('\r').unwrap_or(line);
    let number = if adapter == "claude-code" {
        line.strip_suffix(" (Claude Code)")
    } else {
        line.strip_prefix("codex-cli ")
    }
    .ok_or("qualification_version_report_invalid")?;
    if number.split('.').count() != 3
        || number
            .split('.')
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err("qualification_version_report_invalid".to_owned());
    }
    Ok((line.to_owned(), number.to_owned()))
}

fn selected(adapter: &str) -> Result<PathBuf> {
    let name = if adapter == "claude-code" {
        "claude"
    } else {
        "codex"
    };
    let paths = std::env::var_os("PATH").ok_or("qualification_path_missing")?;
    std::env::split_paths(&paths)
        .map(|path| path.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| "qualification_executable_missing".to_owned())
}

fn conversation() -> String {
    let mut bytes: [u8; 16] = rand::random();
    bytes[6] = (bytes[6] & 15) | 64;
    bytes[8] = (bytes[8] & 63) | 128;
    let hex = lys_runner::protocol::hex(&bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

async fn ingest(
    owned: &mut Owned,
    controller: &mut Controller,
    source: &Binding,
    cancel: &mut Cancellation,
) -> Result<Update> {
    let Message::Frame(frame) = owned.next(cancel).await? else {
        return Err("qualification_frame_invalid".to_owned());
    };
    let update = controller
        .ingest(source, &frame)
        .map_err(|error| error.name())?;
    owned.write(&update)?;
    Ok(update)
}

async fn protocol(
    owned: &mut Owned,
    binding: Binding,
    transport: Transport,
    evidence: &mut Evidence,
    cancel: &mut Cancellation,
) -> Result<()> {
    let mut controller =
        Controller::new(binding.clone(), transport).map_err(|error| error.name())?;
    owned.write(&controller.bootstrap())?;
    loop {
        let update = ingest(owned, &mut controller, &binding, cancel).await?;
        if update
            .events
            .iter()
            .any(|event| event.event == "control_bound")
        {
            break;
        }
    }
    evidence.record("init", json!({"version":binding.harness_version}))?;
    let pending = Pending::new(
        "qualification-reminder".to_owned(),
        Kind::Reminder,
        "Reply exactly ready.".to_owned(),
    );
    let correlation = pending.uuid.clone();
    owned.write(&controller.enqueue(pending).map_err(|error| error.name())?)?;
    loop {
        let update = ingest(owned, &mut controller, &binding, cancel).await?;
        if update.receipts.iter().any(|receipt| {
            receipt.operation == "qualification-reminder"
                && receipt.state == OperationState::Confirmed
                && receipt.reason == "harness_admitted"
        }) {
            break;
        }
    }
    evidence.record("reminder_admitted", json!({"correlation":correlation}))?;
    while !controller.idle() {
        ingest(owned, &mut controller, &binding, cancel).await?;
    }
    let pending = Pending::new(
        "qualification-compaction".to_owned(),
        Kind::Compact,
        String::new(),
    );
    owned.write(&controller.enqueue(pending).map_err(|error| error.name())?)?;
    loop {
        let update = ingest(owned, &mut controller, &binding, cancel).await?;
        if let Some(receipt) = update
            .receipts
            .iter()
            .find(|receipt| receipt.operation == "qualification-compaction")
        {
            if receipt.state != OperationState::Confirmed || receipt.reason != "harness_compacted" {
                return Err("qualification_compaction_unconfirmed".to_owned());
            }
            break;
        }
    }
    evidence.record("compaction", json!({"evidence":if transport == Transport::Claude {"compact_boundary"} else {"ContextCompaction"},"turn_completed":true}))
}

async fn observe(options: &Options, evidence: &mut Evidence) -> Result<()> {
    evidence.record(
        "launcher",
        json!({"kind":"example-owned","claims_spawn":false,"fixture":false}),
    )?;
    let mut cancel = Cancellation::new()?;
    let directory = options.directory.canonicalize().map_err(io_failed)?;
    if directory
        .read_dir()
        .map_err(io_failed)?
        .next()
        .transpose()
        .map_err(io_failed)?
        .is_some()
    {
        return Err("qualification_directory_not_empty".to_owned());
    }
    let selected =
        process::executable(&selected(&options.adapter)?).map_err(|error| error.name())?;
    let mut probe = Owned::start(&selected, &directory, "probe", "", &mut cancel).await?;
    let Message::Version(bytes) = probe.next(&mut cancel).await? else {
        return Err("qualification_version_report_invalid".to_owned());
    };
    let (line, version) = version(&bytes, &options.adapter)?;
    if !probe.wait_exit()?.success() {
        return Err("qualification_version_probe_failed".to_owned());
    }
    evidence.record("version_report", json!({"line":line,"version":version}))?;
    let transport = if options.adapter == "claude-code" {
        Transport::Claude
    } else {
        Transport::Codex
    };
    let conversation = if transport == Transport::Claude {
        conversation()
    } else {
        String::new()
    };
    let mut owned = Owned::start(
        &selected,
        &directory,
        &options.adapter,
        &conversation,
        &mut cancel,
    )
    .await?;
    let binding = Binding {
        session: "qualification".to_owned(),
        generation: 1,
        leader: owned.leader.clone().ok_or("qualification_child_unproved")?,
        conversation,
        entry: process::executable(&std::env::current_exe().map_err(io_failed)?)
            .map_err(|error| error.name())?,
        harness: selected,
        harness_version: version,
        adapter: if transport == Transport::Claude {
            "claude-stream-json/1"
        } else {
            "codex-app-server/1"
        }
        .to_owned(),
    };
    if let Err(error) = protocol(&mut owned, binding, transport, evidence, &mut cancel).await {
        if let Err(cleanup) = owned.stop(true) {
            return Err(format!(
                "qualification_cleanup_failed:{};{}",
                code(&error),
                code(&cleanup)
            ));
        }
        return Err(error);
    }
    let status = owned.stop(true)?;
    evidence.record(
        "stop",
        json!({"exit_observed":true,"status":status.to_string()}),
    )?;
    evidence.record("terminal_writes", json!({"count":0}))?;
    evidence.file.sync_all().map_err(io_failed)
}

async fn run(args: &[String]) -> Result<()> {
    let options = Options::parse(args)?;
    let mut evidence = Evidence::create(&options.evidence)?;
    if let Err(error) = observe(&options, &mut evidence).await {
        evidence.failed(&error)?;
        return Err(error);
    }
    Ok(())
}

fn main() {
    let result = (|| {
        let args: Vec<String> = std::env::args_os()
            .skip(1)
            .map(|arg| {
                arg.into_string().map_err(|arg| {
                    format!(
                        "qualification_arguments_invalid:non-text argument of {} bytes",
                        arg.len()
                    )
                })
            })
            .collect::<Result<_>>()?;
        if args.first().is_some_and(|arg| arg == "--owned-child") {
            return child_entry(&args);
        }
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(io_failed)?
            .block_on(run(&args))
    })();
    if let Err(error) = result {
        eprintln!("{}", code(&error));
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lys_runner::harness_control::Executable;

    #[test]
    fn version_reports_are_bounded_closed_values() {
        assert_eq!(
            version(b"2.1.291 (Claude Code)\n", "claude-code").ok(),
            Some(("2.1.291 (Claude Code)".to_owned(), "2.1.291".to_owned()))
        );
        assert_eq!(
            version(b"codex-cli 0.159.2\n", "codex").ok(),
            Some(("codex-cli 0.159.2".to_owned(), "0.159.2".to_owned()))
        );
        for bytes in [
            b"arbitrary account text".as_slice(),
            b"codex-cli 0.1.2\nsecret",
            b"codex-cli 0..2\n",
        ] {
            assert!(version(bytes, "codex").is_err());
        }
    }

    #[test]
    fn a_missing_step_ends_the_ordered_evidence_without_private_error_words()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("evidence.jsonl");
        let mut evidence = Evidence::create(&path)?;
        evidence.record(
            "launcher",
            json!({"kind":"example-owned","claims_spawn":false,"fixture":true}),
        )?;
        evidence.failed("qualification_cancelled:private harness text")?;
        let later = evidence.record("init", json!({"version":"0.1.2"}));
        println!(
            "QUALIFIER_FAIL_STOP_EVIDENCE_BEGIN\n{}QUALIFIER_EVIDENCE_END",
            std::fs::read_to_string(&path)?
        );
        assert!(later.is_err());
        drop(evidence);
        let text = std::fs::read_to_string(path)?;
        let records: Vec<Value> = text
            .lines()
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()?;
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[1],
            json!({"step":"version_report","observed":false,"reason":"qualification_cancelled"})
        );
        assert!(!text.contains("private harness text"));
        Ok(())
    }

    #[test]
    fn argument_duplicates_and_unknown_inputs_are_refused() {
        let args: Vec<String> = [
            "--adapter",
            "codex",
            "--directory",
            "dir",
            "--evidence",
            "file",
        ]
        .map(str::to_owned)
        .into();
        assert!(Options::parse(&args).is_ok());
        for extra in [
            vec!["--adapter", "codex"],
            vec!["--unknown", "text"],
            vec!["--evidence"],
        ] {
            let mut invalid = args.clone();
            invalid.extend(extra.into_iter().map(str::to_owned));
            assert!(Options::parse(&invalid).is_err());
        }
    }

    #[test]
    fn the_shared_bootstrap_only_supplies_handshake_frames()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let executable = Executable {
            path: "fixture".to_owned(),
            sha256: "fixture".to_owned(),
        };
        let binding = Binding {
            session: "session".to_owned(),
            generation: 1,
            leader: lys_runner::peer::Leader {
                pid: 2,
                start: lys_runner::peer::StartIdentity("fixture".to_owned()),
            },
            conversation: String::new(),
            entry: executable.clone(),
            harness: executable,
            harness_version: "0.1.2".to_owned(),
            adapter: "codex-app-server/1".to_owned(),
        };
        let codex = Controller::new(binding.clone(), Transport::Codex)?;
        let before = codex.control_status();
        let update = codex.bootstrap();
        assert_eq!(update.dispatches.len(), 1);
        assert_eq!(
            update.dispatches[0].frame.get("method"),
            Some(&json!("initialize"))
        );
        assert_eq!(codex.control_status(), before);
        assert!(update.events.is_empty());
        assert!(update.receipts.is_empty());
        assert!(
            Controller::new(binding, Transport::Claude)?
                .bootstrap()
                .dispatches
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn a_fixture_pass_emits_the_seven_contract_records()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("evidence.jsonl");
        let mut evidence = Evidence::create(&path)?;
        evidence.record(
            "launcher",
            json!({"kind":"example-owned","claims_spawn":false,"fixture":true}),
        )?;
        evidence.record(
            "version_report",
            json!({"line":"codex-cli 0.1.2","version":"0.1.2"}),
        )?;
        evidence.record("init", json!({"version":"0.1.2"}))?;
        let pending = Pending::new("fixture-reminder".to_owned(), Kind::Reminder, String::new());
        evidence.record("reminder_admitted", json!({"correlation":pending.uuid}))?;
        evidence.record(
            "compaction",
            json!({"evidence":"ContextCompaction","turn_completed":true}),
        )?;
        evidence.record(
            "stop",
            json!({"exit_observed":true,"status":"exit status: 0"}),
        )?;
        evidence.record("terminal_writes", json!({"count":0}))?;
        evidence.file.sync_all()?;
        let text = std::fs::read_to_string(path)?;
        let records: Vec<Value> = text
            .lines()
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()?;
        assert_eq!(records.len(), STEPS.len());
        assert_eq!(records[0].get("fixture"), Some(&json!(true)));
        for (record, step) in records.iter().zip(STEPS) {
            assert_eq!(record.get("step"), Some(&json!(step)));
        }
        println!("QUALIFIER_FIXTURE_PASS_EVIDENCE_BEGIN\n{text}QUALIFIER_EVIDENCE_END");
        Ok(())
    }
}
