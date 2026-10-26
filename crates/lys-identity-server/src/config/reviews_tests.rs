#![cfg(test)]
//! DIRECTORY-123: the period is written by install, required by the service.
use serde_json::json;

use super::super::Config;

#[test]
fn missing_or_malformed_review_period_is_refused_by_its_full_name()
-> Result<(), Box<dyn std::error::Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("identity.json");
    for value in [
        json!({}),
        json!({"reviews": {}}),
        json!({"reviews": {"period_days": 0}}),
        json!({"reviews": {"period_days": -1}}),
        json!({"reviews": {"period_days": 1.5}}),
        json!({"reviews": {"period_days": "30"}}),
        json!({"reviews": {"period_days": null}}),
    ] {
        std::fs::write(&path, serde_json::to_vec(&value)?)?;
        let refused = Config::load(&path).expect_err("no written, positive whole period");
        assert!(
            refused.to_string().contains("[reviews] period_days"),
            "{refused}"
        );
    }
    Ok(())
}
