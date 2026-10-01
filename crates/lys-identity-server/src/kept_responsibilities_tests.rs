#![cfg(test)]

use super::Kept;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn old_installs_load_the_six_default_responsibilities() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let kept = Kept::load(&dir.path().join("kept-responsibilities.json"))?;
    assert_eq!(kept.routes.len(), 6);
    assert!(kept.keeps("POST", "/setup"));
    assert!(kept.keeps("POST", "/grants/roots"));
    assert!(kept.keeps("POST", "/sign-in-providers"));
    assert!(!kept.keeps("GET", "/configuration"));
    Ok(())
}

#[test]
fn deployment_data_overrides_defaults_and_matches_route_parameters() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let file = dir.path().join("kept-responsibilities.json");
    std::fs::write(&file, r#"[{"method":"POST","path":"/grants/{id}/revoke"}]"#)?;
    let kept = Kept::load(&file)?;
    assert!(kept.keeps("POST", "/grants/a/revoke"));
    assert!(!kept.keeps("POST", "/grants//revoke"));
    assert!(!kept.keeps("POST", "/grants/a/revoke/extra"));
    assert!(!kept.keeps("POST", "/setup"));
    Ok(())
}

#[test]
fn malformed_unknown_and_duplicate_deployment_routes_refuse() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let file = dir.path().join("kept-responsibilities.json");
    for data in [
        "invalid",
        r#"[{"method":"POST","path":"/missing-route"}]"#,
        r#"[{"method":"GET","path":"/setup"}]"#,
        r#"[{"method":"POST","path":"/setup","extra":true}]"#,
        r#"[{"method":"POST","path":"/setup"},{"method":"POST","path":"/setup"}]"#,
        r#"[{"method":"POST","path":"/setup"},{"method":"post","path":"/setup"}]"#,
    ] {
        std::fs::write(&file, data)?;
        assert!(Kept::load(&file).is_err(), "accepted {data}");
    }
    Ok(())
}
