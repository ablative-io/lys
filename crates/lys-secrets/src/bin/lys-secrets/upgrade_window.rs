//! The managed broker's audit format during a reversible upgrade.

#[cfg(test)]
mod tests {
    use crate::cli::Where;
    use crate::files::{FileGrants, Layout, now_ms};
    use lys_secrets::Broker;

    #[test]
    fn put_back_reader_keeps_its_format_before_started() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let at = Where {
            root: root.path().join("secrets"),
            keys: root.path().join("keys"),
        };
        let layout = Layout::new(&at.root, &at.keys);
        layout.prepare()?;
        drop(Broker::create(&layout.paths(), FileGrants::new(layout.grants()), Box::new(now_ms))?);
        let log = layout.paths().log_dir;
        let mut config: serde_json::Value = serde_json::from_slice(&std::fs::read(log.join("log.json"))?)?;
        config["format"] = "lys/log-dir/v1".into();
        std::fs::write(log.join("log.json"), serde_json::to_vec(&config)?)?;
        std::fs::remove_dir_all(log.join("leaves/segments"))?;
        std::fs::write(log.join("state.json"), br#"{"tree_size":0,"root_hash":"47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="}"#)?;
        std::fs::create_dir(root.path().join("install"))?;
        std::fs::write(root.path().join("install/upgrade.json"), br#"{"steps":[]}"#)?;
        drop(crate::open(&at)?);
        let config: serde_json::Value = serde_json::from_slice(&std::fs::read(log.join("log.json"))?)?;
        assert_eq!(config["format"], "lys/log-dir/v1", "put-back must use the installed reader's format until Started");
        Ok(())
    }
}
