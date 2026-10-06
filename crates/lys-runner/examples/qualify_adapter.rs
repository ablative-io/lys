//! Observe an installed adapter without granting it product qualification.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use lys_runner::harness_control::{Binding, Controller, Kind, Pending, Transport, Update, process};
use lys_runner::operations::OperationState;
use serde_json::{Value, json};

#[path = "qualify_adapter/diagnostics.rs"]
mod diagnostics;
use diagnostics::{Compaction, check_compaction};

#[path = "qualify_adapter/owned.rs"]
mod owned;
use owned::{Cancellation, Message, Owned, child_entry};

type Result<T> = std::result::Result<T, String>;
const STEPS: [&str; 8] = [
    "launcher",
    "version_report",
    "bound",
    "reminder_admitted",
    "init",
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
    cleanup: String,
    compaction: Option<Compaction>,
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
            cleanup: "not_started".to_owned(),
            compaction: None,
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
        if reason.trim().is_empty() || self.cleanup.trim().is_empty() {
            return Err(
                "qualification_evidence_failure_invalid: reason and cleanup must be named"
                    .to_owned(),
            );
        }
        if let Some(step) = STEPS.get(self.next).copied() {
            let mut detail = json!({"observed":false,"reason":reason,"cleanup":self.cleanup});
            if step == "compaction" {
                if let Some(facts) = &self.compaction {
                    detail["compaction"] = serde_json::to_value(facts)
                        .map_err(|error| format!("qualification_evidence_failed:{error}"))?;
                }
            }
            self.record(step, detail)?;
        }
        self.closed = true;
        self.file.sync_all().map_err(io_failed)
    }
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
    transport: Transport,
    proof: &mut Option<String>,
    compaction: Option<&mut Compaction>,
) -> Result<Update> {
    let Message::Frame(frame) = owned.next(cancel).await? else {
        return Err("qualification_frame_invalid".to_owned());
    };
    if let Some(facts) = compaction {
        facts.observe(&frame)?;
    }
    let update = controller
        .ingest(source, &frame)
        .map_err(|error| error.to_string())?;
    if transport == Transport::Claude
        && update
            .events
            .iter()
            .any(|event| event.event == "control_initialized")
    {
        *proof = frame
            .get("claude_code_version")
            .and_then(Value::as_str)
            .map(str::to_owned);
    } else if transport == Transport::Codex
        && frame.get("id").and_then(Value::as_str) == Some("lys-initialize")
    {
        *proof = frame
            .pointer("/result/userAgent")
            .and_then(Value::as_str)
            .and_then(|agent| agent.split_whitespace().next())
            .and_then(|product| product.split_once('/'))
            .map(|(_, version)| version.to_owned());
    }
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
        Controller::new(binding.clone(), transport).map_err(|error| error.to_string())?;
    let bootstrap = controller.bootstrap();
    let frame = &bootstrap
        .dispatches
        .first()
        .ok_or("qualification_initialize_missing")?
        .frame;
    let correlation = frame
        .get("request_id")
        .or_else(|| frame.get("id"))
        .and_then(Value::as_str)
        .ok_or("qualification_initialize_uncorrelated")?
        .to_owned();
    let mut proof = None;
    owned.write(&bootstrap)?;
    loop {
        let update = ingest(
            owned,
            &mut controller,
            &binding,
            cancel,
            transport,
            &mut proof,
            None,
        )
        .await?;
        if update
            .events
            .iter()
            .any(|event| event.event == "control_bound")
        {
            break;
        }
    }
    evidence.record("bound", json!({"correlation":correlation}))?;
    let pending = Pending::new(
        "qualification-reminder".to_owned(),
        Kind::Reminder,
        "Reply exactly ready.".to_owned(),
    );
    let correlation = pending.uuid.clone();
    owned.write(
        &controller
            .enqueue(pending)
            .map_err(|error| error.to_string())?,
    )?;
    loop {
        let update = ingest(
            owned,
            &mut controller,
            &binding,
            cancel,
            transport,
            &mut proof,
            None,
        )
        .await?;
        if update.receipts.iter().any(|receipt| {
            receipt.operation == "qualification-reminder"
                && receipt.state == OperationState::Confirmed
                && receipt.reason == "harness_admitted"
        }) {
            break;
        }
    }
    evidence.record("reminder_admitted", json!({"correlation":correlation}))?;
    let version = proof.as_deref().ok_or("qualification_init_unobserved")?;
    if version != binding.harness_version {
        return Err(
            "qualification_version_mismatch: the serving harness differs from its version report"
                .to_owned(),
        );
    }
    evidence.record("init", json!({"version":version}))?;
    while !controller.idle() {
        ingest(
            owned,
            &mut controller,
            &binding,
            cancel,
            transport,
            &mut proof,
            None,
        )
        .await?;
    }
    let pending = Pending::new(
        "qualification-compaction".to_owned(),
        Kind::Compact,
        String::new(),
    );
    evidence.compaction = Some(Compaction::new(&binding.conversation, &pending.uuid));
    owned.write(
        &controller
            .enqueue(pending)
            .map_err(|error| error.to_string())?,
    )?;
    loop {
        let update = ingest(
            owned,
            &mut controller,
            &binding,
            cancel,
            transport,
            &mut proof,
            evidence.compaction.as_mut(),
        )
        .await?;
        if check_compaction(&update)? {
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
        process::executable(&selected(&options.adapter)?).map_err(|error| error.to_string())?;
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
            .map_err(|error| error.to_string())?,
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
        eprintln!("{error}");
        evidence.cleanup = match owned.stop(true) {
            Ok(_) => "ok".to_owned(),
            Err(cleanup) => cleanup,
        };
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
        eprintln!("{error}");
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
    fn a_missing_step_keeps_lys_words_and_discards_harness_text()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("evidence.jsonl");
        let mut evidence = Evidence::create(&path)?;
        evidence.record(
            "launcher",
            json!({"kind":"example-owned","claims_spawn":false,"fixture":true}),
        )?;
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
        let mut controller = Controller::new(binding.clone(), Transport::Codex)?;
        let error = controller
            .ingest(
                &binding,
                &json!({"id":"lys-initialize",
            "error":{"message":"fixture private harness phrase"}}),
            )
            .err()
            .ok_or("harness error was accepted")?;
        evidence.failed(&error.to_string())?;
        let records: Vec<Value> = std::fs::read_to_string(&path)?
            .lines()
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()?;
        assert_eq!(
            records[1]["reason"],
            "control_protocol_unsupported: initialize was refused"
        );
        assert_eq!(records[1]["cleanup"], "not_started");
        assert!(!std::fs::read_to_string(path)?.contains("fixture private harness phrase"));
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
        let claude = Controller::new(binding, Transport::Claude)?.bootstrap();
        assert_eq!(claude.dispatches.len(), 1);
        assert_eq!(
            claude.dispatches[0].frame["request"]["subtype"],
            "initialize"
        );
        assert_eq!(claude.dispatches[0].frame["type"], "control_request");
        Ok(())
    }

    #[test]
    fn a_fixture_pass_emits_the_eight_contract_records()
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
        evidence.record("bound", json!({"correlation":"lys-initialize"}))?;
        let pending = Pending::new("fixture-reminder".to_owned(), Kind::Reminder, String::new());
        evidence.record("reminder_admitted", json!({"correlation":pending.uuid}))?;
        evidence.record("init", json!({"version":"0.1.2"}))?;
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
    #[test]
    fn an_empty_reason_or_cleanup_is_refused_before_writing_evidence()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        for (reason, cleanup) in [("", "ok"), ("named failure", "")] {
            let folder = tempfile::tempdir()?;
            let path = folder.path().join("evidence.jsonl");
            let mut evidence = Evidence::create(&path)?;
            evidence.cleanup = cleanup.to_owned();
            assert!(evidence.failed(reason).is_err());
            assert!(std::fs::read(path)?.is_empty());
        }
        Ok(())
    }
    #[test]
    fn a_failed_compaction_keeps_the_typed_receipt_and_lys_reason() -> Result<()> {
        for state in [OperationState::Refused, OperationState::Uncertain] {
            let mut update = Update::default();
            update.receipts.push(lys_runner::harness_control::Receipt {
                operation: "qualification-compaction".to_owned(),
                state,
                reason: "not_compacted".to_owned(),
            });
            let reason = check_compaction(&update)
                .err()
                .ok_or("failed receipt accepted")?;
            assert!(reason.contains(&format!("{state:?}")), "{reason}");
            assert!(reason.contains("not_compacted"), "{reason}");
        }
        Ok(())
    }

    #[test]
    fn compaction_facts_keep_only_correlated_closed_values_and_frame_order() -> Result<()> {
        let mut facts = Compaction::new("private-conversation", "private-uuid");
        let boundary = json!({"type":"system","subtype":"compact_boundary", "session_id":"private-conversation", "compact_metadata":{"secret":"private fixture phrase"}});
        let replay = json!({"type":"user","session_id":"private-conversation", "uuid":"private-uuid", "parent_tool_use_id":null, "message":{"role":"user","content":"private fixture phrase"}});
        let status = json!({"type":"system","subtype":"status", "session_id":"private-conversation", "status":"compacting", "compact_result":"failed", "compact_error":"private fixture phrase"});
        for frame in [
            &boundary,
            &replay,
            &status,
            &json!({"type":"result", "session_id":"private-conversation", "is_error":true}),
        ] {
            let mut foreign = frame.clone();
            foreign["session_id"] = json!("other");
            facts.observe(&foreign)?;
        }
        let empty = serde_json::to_value(&facts).map_err(io_failed)?;
        facts.observe(&json!({"type":"user","session_id":"private-conversation", "uuid":"other", "parent_tool_use_id":null, "message":{"role":"user"}}))?;
        assert_eq!(serde_json::to_value(&facts).map_err(io_failed)?, empty);
        facts.observe(&replay)?;
        facts.observe(&boundary)?;
        facts.observe(&replay)?;
        facts.observe(&status)?;
        facts.observe(&json!({"type":"system","subtype":"status", "session_id":"private-conversation", "status":"requesting"}))?;
        facts.observe(&json!({"type":"system","subtype":"status", "session_id":"private-conversation", "status":null, "compact_result":"success"}))?;
        facts.observe(&json!({"type":"result", "session_id":"private-conversation", "is_error":true, "result":"private fixture phrase"}))?;
        let value = serde_json::to_value(&facts).map_err(io_failed)?;
        assert_eq!(
            value,
            json!({
                "compact_boundary_seen":true,"matching_user_replay_seen":true,
                "replay_before_boundary":true,"replay_after_boundary":true,
                "result_seen":true,"result_is_error":true,
                "statuses":[{"status":"compacting","compact_result":"failed"},
                    {"status":"requesting","compact_result":null},{"status":null,"compact_result":"success"}]
            })
        );
        let encoded = value.to_string();
        for private in [
            "private fixture phrase",
            "private-conversation",
            "private-uuid",
            "compact_error",
            "compact_metadata",
        ] {
            assert!(!encoded.contains(private));
        }
        Ok(())
    }

    #[test]
    fn compaction_status_facts_refuse_unknown_values_and_overflow_without_text() -> Result<()> {
        let mut facts = Compaction::new("conversation", "uuid");
        for fields in [
            json!({}),
            json!({"status":"private fixture phrase"}),
            json!({"status":null,"compact_result":"private fixture phrase"}),
            json!({"status":null,"compact_result":null}),
        ] {
            let mut frame = json!({"type":"system","subtype":"status","session_id":"conversation"});
            frame
                .as_object_mut()
                .ok_or("fixture shape invalid")?
                .extend(fields.as_object().ok_or("fixture fields invalid")?.clone());
            let error = facts
                .observe(&frame)
                .err()
                .ok_or("invalid status was accepted")?;
            assert!(error.starts_with("qualification_compaction_status_invalid"));
            assert!(!error.contains("private fixture phrase"));
        }
        let status =
            json!({"type":"system","subtype":"status", "session_id":"conversation", "status":null});
        for _ in 0..32 {
            facts.observe(&status)?;
        }
        let before = serde_json::to_value(&facts).map_err(io_failed)?;
        assert_eq!(
            facts.observe(&status).err().as_deref(),
            Some("qualification_compaction_status_limit: more than 32 status frames")
        );
        assert_eq!(serde_json::to_value(&facts).map_err(io_failed)?, before);
        Ok(())
    }

    #[test]
    fn compaction_replay_order_distinguishes_boundary_first_and_missing_boundary() -> Result<()> {
        let replay = json!({"type":"user","session_id":"conversation","uuid":"uuid", "parent_tool_use_id":null,"message":{"role":"user"}});
        for boundary_first in [true, false] {
            let mut facts = Compaction::new("conversation", "uuid");
            if boundary_first {
                facts.observe(&json!({"type":"system","subtype":"compact_boundary","session_id":"conversation"}))?;
            }
            facts.observe(&replay)?;
            facts
                .observe(&json!({"type":"result","session_id":"conversation","is_error":false}))?;
            let value = serde_json::to_value(&facts).map_err(io_failed)?;
            assert_eq!(value["compact_boundary_seen"], boundary_first);
            assert_eq!(value["replay_after_boundary"], boundary_first);
            assert_eq!(value["replay_before_boundary"], !boundary_first);
            assert_eq!(value["result_is_error"], false);
        }
        Ok(())
    }

    #[test]
    fn failed_compaction_evidence_contains_actual_receipt_facts_and_cleanup()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("evidence.jsonl");
        let mut evidence = Evidence::create(&path)?;
        for step in &STEPS[..5] {
            evidence.record(step, json!({"fixture":true}))?;
        }
        evidence.cleanup = "ok".to_owned();
        let mut facts = Compaction::new("conversation", "uuid");
        facts.observe(&json!({"type":"result","session_id":"conversation","is_error":false, "result":"private fixture phrase"}))?;
        evidence.compaction = Some(facts);
        let mut update = Update::default();
        update.receipts.push(lys_runner::harness_control::Receipt {
            operation: "qualification-compaction".to_owned(),
            state: OperationState::Refused,
            reason: "not_compacted".to_owned(),
        });
        let error = check_compaction(&update)
            .err()
            .ok_or("failed compaction was accepted")?;
        evidence.failed(&error)?;
        let text = std::fs::read_to_string(path)?;
        let records: Vec<Value> = text
            .lines()
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()?;
        assert_eq!(records.len(), 6);
        assert_eq!(records[5]["step"], "compaction");
        assert_eq!(
            records[5]["reason"],
            "qualification_compaction_unconfirmed: state=Refused; reason=not_compacted"
        );
        assert_eq!(records[5]["cleanup"], "ok");
        assert_eq!(records[5]["compaction"]["result_is_error"], false);
        assert_eq!(records[5]["compaction"]["compact_boundary_seen"], false);
        assert!(!text.contains("private fixture phrase"));
        Ok(())
    }

    #[test]
    fn compaction_receipts_require_both_confirmed_state_and_compacted_reason() -> Result<()> {
        let mut update = Update::default();
        assert!(!check_compaction(&update)?);
        update.receipts.push(lys_runner::harness_control::Receipt {
            operation: "other".to_owned(),
            state: OperationState::Confirmed,
            reason: "harness_compacted".to_owned(),
        });
        assert!(!check_compaction(&update)?);
        update.receipts.push(lys_runner::harness_control::Receipt {
            operation: "qualification-compaction".to_owned(),
            state: OperationState::Confirmed,
            reason: "harness_admitted".to_owned(),
        });
        assert!(check_compaction(&update).is_err());
        update.receipts[1].reason = "harness_compacted".to_owned();
        assert!(check_compaction(&update)?);
        Ok(())
    }

    #[test]
    fn a_malformed_result_is_seen_without_retaining_its_outcome_text() -> Result<()> {
        let mut facts = Compaction::new("conversation", "uuid");
        let error = facts.observe(&json!({"type":"result","session_id":"conversation","is_error":"private fixture phrase"}))
            .err().ok_or("malformed result was accepted")?;
        assert_eq!(
            error,
            "qualification_compaction_result_invalid: is_error must be a boolean"
        );
        let value = serde_json::to_value(&facts).map_err(io_failed)?;
        assert_eq!(value["result_seen"], true);
        assert_eq!(value["result_is_error"], Value::Null);
        assert!(!value.to_string().contains("private fixture phrase"));
        Ok(())
    }
}
