//! Signed PTY starts bind hooks to the declared harness configuration.

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::peer::Collected;
use crate::protocol::{Act, Answer, Greeting, Launch, sign_request};
use crate::session::Sessions;
use crate::tracking_store::Body;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;
const NATIVE: &str = "019bcd60-1a27-7000-8000-123456789abc";

struct Wire {
    sessions: Arc<Sessions>,
    directory: tempfile::TempDir,
}

impl Wire {
    fn new() -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let sessions = Sessions::open(&directory.path().join("state"), 4096)?;
        Ok(Self {
            sessions,
            directory,
        })
    }

    fn start(
        &self,
        tracking: &Value,
        environment: &Value,
        caller: bool,
        proxy: bool,
    ) -> TestResult {
        let launch: Launch = serde_json::from_value(json!({
            "session":"wire", "program":"/bin/cat", "arguments":[],
            "directory":"/", "environment":environment, "columns":80, "rows":24
        }))?;
        let mut value = json!({"act":"start", "launch":launch, "tracking":tracking});
        if proxy {
            self.sessions
                .proxy_state(&self.directory.path().join("proxy"));
            value["proxy"] = json!({"run":"0123456789abcdef0123456789abcdef",
                "context_window":200_000, "profile_version":1, "account":null});
        }
        if caller {
            value = json!({"act":"as_caller", "caller":"person", "done":value});
        }
        let act: Act = serde_json::from_value(value)?;
        let key = lys_core::Ed25519Identity::load_or_generate(&self.directory.path().join("key"))?;
        let greeting = Greeting::fresh(self.sessions.runner());
        let line = sign_request(&key, &greeting, &act)?;
        let answer = crate::socket::dispatch(
            &self.sessions,
            &key.public_key_bytes(),
            &greeting,
            &line,
            &AtomicBool::new(false),
        );
        assert!(matches!(answer, Answer::Started { .. }), "{answer:?}");
        Ok(())
    }

    fn hook(&self, path: &std::path::Path) -> Result<String, crate::RunnerError> {
        self.sessions.collect(
            "wire",
            &Collected::Hook {
                event: "SessionStart".to_owned(),
                input: json!({"session_id":NATIVE, "transcript_path":path}),
            },
        )
    }

    fn start_record(&self) -> Result<Value, Box<dyn Error>> {
        let table = self.sessions.lock()?;
        let coverage = table
            .feed
            .page(None)?
            .entries
            .into_iter()
            .find_map(|entry| match entry.body {
                Body::Coverage(coverage) if coverage.state == "tracking_started" => Some(coverage),
                _ => None,
            })
            .ok_or("tracking start absent")?;
        assert_eq!(coverage.harness_version, None);
        let words: Value = serde_json::from_str(&coverage.words)?;
        assert_eq!(words["harness_version"], Value::Null);
        assert_eq!(words["context_window"], Value::Null);
        Ok(words)
    }
}

impl Drop for Wire {
    fn drop(&mut self) {
        if let Err(error) = self.sessions.stop_all() {
            crate::error::said(&format!("wire_test_cleanup_failed: {error}"));
        }
    }
}

#[test]
fn wire_starts_bind_spec_transcripts_without_proxy_and_keep_proxy_accounting_with_native_hooks()
-> TestResult {
    for caller in [false, true] {
        for proxy in [false, true] {
            let wire = Wire::new()?;
            let home = wire.directory.path().join("spec");
            let other = wire.directory.path().join("environment");
            let path = home.join(format!("projects/-/{NATIVE}.jsonl"));
            std::fs::create_dir_all(path.parent().ok_or("transcript parent absent")?)?;
            std::fs::write(&path, b"")?;
            wire.start(
                &json!({"harness":"claude_code", "config_home":home}),
                &json!({"CLAUDE_CONFIG_DIR":other}),
                caller,
                proxy,
            )?;
            let wrong = other.join(format!("projects/-/{NATIVE}.jsonl"));
            if proxy {
                let before = wire
                    .sessions
                    .lock()?
                    .feed
                    .source("wire")
                    .cloned()
                    .ok_or("source absent")?;
                wire.hook(&wrong)?;
                let mut expected = before.clone();
                expected.native_conversation = Some(NATIVE.to_owned());
                assert_eq!(wire.sessions.lock()?.feed.source("wire"), Some(&expected));
                assert_ne!(expected.path, path.display().to_string());
                assert_ne!(expected.path, wrong.display().to_string());
                assert_eq!(wire.start_record()?["config_home_source"], "spec");
                continue;
            }
            assert_eq!(
                wire.hook(&wrong)
                    .err()
                    .ok_or("wrong transcript accepted")?
                    .name(),
                "transcript_unbound"
            );
            wire.hook(&path)?;
            let table = wire.sessions.lock()?;
            assert_eq!(
                table.feed.source("wire").ok_or("source absent")?.path,
                path.display().to_string()
            );
            assert!(table.feed.page(None)?.entries.iter().any(|entry|
                matches!(&entry.body, Body::Coverage(coverage) if coverage.state == "source_bound")));
            drop(table);
            assert_eq!(wire.start_record()?["config_home_source"], "spec");
        }
    }
    Ok(())
}

#[test]
fn wire_tracking_names_the_environment_config_home_source() -> TestResult {
    for (harness, name) in [
        ("claude_code", "CLAUDE_CONFIG_DIR"),
        ("codex", "CODEX_HOME"),
    ] {
        let wire = Wire::new()?;
        let home = wire.directory.path().join("environment");
        wire.start(
            &json!({"harness":harness, "config_home":null}),
            &json!({name:home}),
            false,
            false,
        )?;
        let record = wire.start_record()?;
        assert_eq!(record["config_home_source"], "environment");
        assert_eq!(record["config_home"], json!(home));
    }
    Ok(())
}

#[test]
fn wire_tracking_names_the_runner_home_source() -> TestResult {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("runner HOME absent")?);
    for (harness, folder) in [("claude_code", ".claude"), ("codex", ".codex")] {
        let wire = Wire::new()?;
        wire.start(&json!({"harness":harness}), &json!({}), false, false)?;
        let record = wire.start_record()?;
        assert_eq!(record["config_home_source"], "home");
        assert_eq!(record["config_home"], json!(home.join(folder)));
    }
    Ok(())
}
