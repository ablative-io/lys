use super::environment;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};

fn launch(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}
fn value<'a>(kept: &'a BTreeMap<OsString, OsString>, name: &str) -> Option<&'a str> {
    kept.get(OsStr::new(name)).and_then(|value| value.to_str())
}

#[test]
fn openai_seat_program_keeps_only_the_proxy_account_selector() {
    let kept = environment(&launch(&[
        ("LYS_ACCOUNT_VARIABLE", "CODEX_API_KEY"),
        ("CODEX_API_KEY", "lys-account.first"),
        ("OPENAI_API_KEY", "fixture-credential"),
    ]));
    assert_eq!(value(&kept, "CODEX_API_KEY"), Some("lys-account.first"));
    assert_eq!(value(&kept, "OPENAI_API_KEY"), None);
}

#[test]
fn anthropic_seat_program_keeps_the_selector_and_explicit_credential_clears() {
    let kept = environment(&launch(&[
        ("LYS_ACCOUNT_VARIABLE", "ANTHROPIC_AUTH_TOKEN"),
        ("ANTHROPIC_AUTH_TOKEN", "lys-account.first"),
        ("ANTHROPIC_API_KEY", ""),
        ("CLAUDE_CODE_OAUTH_TOKEN", ""),
        ("OTHER_AUTH_TOKEN", "fixture-credential"),
    ]));
    assert_eq!(
        value(&kept, "ANTHROPIC_AUTH_TOKEN"),
        Some("lys-account.first")
    );
    assert_eq!(value(&kept, "ANTHROPIC_API_KEY"), Some(""));
    assert_eq!(value(&kept, "CLAUDE_CODE_OAUTH_TOKEN"), Some(""));
    assert_eq!(value(&kept, "OTHER_AUTH_TOKEN"), None);
}

#[test]
fn a_provider_credential_is_removed_when_no_proxy_selector_is_bound() {
    let kept = environment(&launch(&[
        ("LYS_ACCOUNT_VARIABLE", "CODEX_API_KEY"),
        ("CODEX_API_KEY", "fixture-credential"),
    ]));
    assert_eq!(value(&kept, "CODEX_API_KEY"), None);
}
