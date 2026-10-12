//! Missing request allowances are recorded explicitly before a profile is saved.

use serde::{Deserialize, Deserializer};

use crate::provisioning_store::SessionSettings;

#[derive(Clone)]
pub(super) struct IncomingSession {
    pub settings: SessionSettings,
    pub migrated: bool,
}

impl<'de> Deserialize<'de> for IncomingSession {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| serde::de::Error::custom("session must be an object"))?;
        let migrated = !object.contains_key("stall_after");
        if migrated {
            object.insert("stall_after".to_owned(), 300.into());
        }
        let settings = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self { settings, migrated })
    }
}

pub(super) fn recorded(agent: &str, version: u32) {
    tracing::warn!(
        agent,
        version,
        migration = "profile_stall_after_recorded",
        "provisioning wrote session.stall_after=300 into the saved version because the request omitted it"
    );
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{IncomingSession, recorded};
    use crate::provisioning_store::{ProvisioningStore, Version};
    use crate::warn_lines::WarnLines;

    #[test]
    fn an_omitted_request_stall_allowance_is_saved_explicitly_and_logged_once()
    -> Result<(), Box<dyn std::error::Error>> {
        let incoming: IncomingSession = serde_json::from_value(serde_json::json!({
            "compact": "/compact", "sensitive": false
        }))?;
        assert!(incoming.migrated);
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("provisioning.json");
        let mut store = ProvisioningStore::open(&path)?;
        let mut version: Version = serde_json::from_value(serde_json::json!({
            "number": 0, "operation": "request", "set_by": "person", "set_at": 1,
            "settings": {"model_access": [], "tools": [], "skills": [],
                "mcp_servers": [], "instructions": "", "note": ""}
        }))?;
        version.settings.session = Some(incoming.settings);
        let heard = Arc::new(Mutex::new(Vec::<String>::new()));
        let lines = Arc::clone(&heard);
        tracing::subscriber::with_default(
            WarnLines::new(Arc::new(move |line| match lines.lock() {
                Ok(mut lines) => lines.push(line.to_owned()),
                Err(error) => panic!("fixture log poisoned: {error}"),
            })),
            || -> Result<(), crate::error::ServerError> {
                let number = store.set("agent", 0, version)?;
                recorded("agent", number);
                Ok(())
            },
        )?;
        let reopened = ProvisioningStore::open(&path)?;
        let kept = reopened.profile("agent").ok_or("saved profile missing")?;
        let saved = serde_json::to_value(&kept.versions[0])?;
        assert_eq!(saved["settings"]["session"]["stall_after"], 300);
        assert_eq!(saved["settings"]["session"]["compact"], "/compact");
        let lines = heard.lock().map_err(|error| error.to_string())?;
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].contains("provisioning wrote session.stall_after=300 into the saved version")
        );
        assert!(lines[0].contains("agent=agent version=1"));
        Ok(())
    }

    #[test]
    fn explicit_request_stall_allowances_remain_strict() -> Result<(), Box<dyn std::error::Error>> {
        let explicit: IncomingSession =
            serde_json::from_value(serde_json::json!({"stall_after": 12}))?;
        assert!(!explicit.migrated);
        assert_eq!(explicit.settings.stall_after, 12);
        for invalid in [
            serde_json::json!(0),
            serde_json::json!(null),
            serde_json::json!("300"),
            serde_json::json!(u64::MAX),
        ] {
            assert!(
                serde_json::from_value::<IncomingSession>(
                    serde_json::json!({"stall_after": invalid})
                )
                .is_err()
            );
        }
        assert!(
            serde_json::from_value::<IncomingSession>(serde_json::json!({"unknown": true}))
                .is_err()
        );
        Ok(())
    }
}
