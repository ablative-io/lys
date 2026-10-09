//! The managed broker's audit format during a reversible upgrade.

use std::path::Path;
use std::sync::Arc;

use lys_log_store::file::MigrationReady;
use lys_log_store::{StoreError, StoreResult};
use serde::Deserialize;

#[derive(Deserialize)]
struct Intent {
    steps: Vec<Step>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Step {
    Stopped,
    DataKept,
    BinariesKept,
    BinariesPlaced,
    ConfigurationKept,
    ConfigurationPlaced,
    ScreensKept,
    ScreensPlaced,
    ComposeApplied,
    Started,
}

pub(crate) fn check(root: &Path) -> Option<MigrationReady> {
    if root.file_name()? != "secrets" {
        return None;
    }
    let intent = root.parent()?.join("install/upgrade.json");
    Some(Arc::new(move || committed(&intent)))
}

fn committed(path: &Path) -> StoreResult<bool> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(source) => {
            return Err(StoreError::Io {
                context: format!("failed to read upgrade commit point {}", path.display()),
                source,
            });
        }
    };
    let intent: Intent = serde_json::from_slice(&bytes).map_err(|error| StoreError::Corrupt {
        path: path.to_path_buf(),
        reason: format!("upgrade intent does not read: {error}"),
    })?;
    Ok(intent
        .steps
        .iter()
        .any(|step| matches!(step, Step::Started)))
}

#[cfg(test)]
mod tests {
    use crate::cli::Where;
    use crate::files::{FileGrants, Layout, now_ms};
    use lys_secrets::Broker;

    #[test]
    fn commit_point_refuses_unknown_or_malformed_steps() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let intent = root.path().join("upgrade.json");
        assert!(super::committed(&intent)?);
        for bytes in [
            br#"{"steps":[]}"#.as_slice(),
            br#"{"steps":["binaries_placed"]}"#,
        ] {
            std::fs::write(&intent, bytes)?;
            assert!(!super::committed(&intent)?);
        }
        std::fs::write(&intent, br#"{"steps":["started"]}"#)?;
        assert!(super::committed(&intent)?);
        for bytes in [
            br#"{"steps":["unrecognised"]}"#.as_slice(),
            br#"{"steps":null}"#,
            b"{",
        ] {
            std::fs::write(&intent, bytes)?;
            assert!(super::committed(&intent).is_err());
        }
        assert!(super::check(&root.path().join("unmanaged")).is_none());
        Ok(())
    }

    #[test]
    fn put_back_reader_keeps_its_format_before_started() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let at = Where {
            root: root.path().join("secrets"),
            keys: root.path().join("keys"),
        };
        let layout = Layout::new(&at.root, &at.keys);
        layout.prepare()?;
        drop(Broker::create(
            &layout.paths(),
            FileGrants::new(layout.grants()),
            Box::new(now_ms),
        )?);
        let log = layout.paths().log_dir;
        let mut config: serde_json::Value =
            serde_json::from_slice(&std::fs::read(log.join("log.json"))?)?;
        config["format"] = "lys/log-dir/v1".into();
        std::fs::write(log.join("log.json"), serde_json::to_vec(&config)?)?;
        std::fs::remove_dir_all(log.join("leaves/segments"))?;
        std::fs::write(
            log.join("state.json"),
            br#"{"tree_size":0,"root_hash":"47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="}"#,
        )?;
        std::fs::create_dir(root.path().join("install"))?;
        std::fs::write(root.path().join("install/upgrade.json"), br#"{"steps":[]}"#)?;
        drop(crate::open(&at)?);
        let config: serde_json::Value =
            serde_json::from_slice(&std::fs::read(log.join("log.json"))?)?;
        assert_eq!(
            config["format"], "lys/log-dir/v1",
            "put-back must use the installed reader's format until Started"
        );
        Ok(())
    }
}
