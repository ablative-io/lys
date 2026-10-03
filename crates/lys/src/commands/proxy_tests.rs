use lys_home::proxy::forward::Base;

use super::{anthropic_upstream, names_itself, parsed};
use crate::identity::install::proxy::shown;

#[test]
fn the_upstream_is_the_flag_else_the_record_else_anthropic_and_never_the_environment()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let record = dir.path().join("upstream.json");
    std::fs::write(
        &record,
        r#"{"anthropic":"https://gateway.example/anthropic","from":"login"}"#,
    )?;
    assert_eq!(
        anthropic_upstream(None, Some(&record))?,
        (
            "https://gateway.example/anthropic".to_owned(),
            "login".to_owned()
        )
    );
    assert_eq!(
        anthropic_upstream(Some("http://flag.example".to_owned()), Some(&record))?,
        ("http://flag.example".to_owned(), "flag".to_owned())
    );
    assert_eq!(
        anthropic_upstream(None, None)?,
        ("https://api.anthropic.com".to_owned(), "default".to_owned())
    );
    assert!(anthropic_upstream(None, Some(&dir.path().join("absent.json"))).is_err());
    Ok(())
}

#[test]
fn a_named_upstream_never_shows_its_user_password_or_query() {
    assert_eq!(
        shown("https://user:pass@gateway.example:8443/v1/anthropic?key=k#f"),
        "https://gateway.example:8443/v1/anthropic"
    );
    assert_eq!(
        shown("https://api.anthropic.com"),
        "https://api.anthropic.com"
    );
}

#[test]
fn an_upstream_naming_the_proxy_itself_is_found() -> Result<(), Box<dyn std::error::Error>> {
    let listen = "127.0.0.1:8484";
    let itself = |base: &str| -> Result<bool, lys_home::proxy::error::ProxyError> {
        Ok(names_itself(&Base::parse(base)?, listen))
    };
    assert!(itself("http://127.0.0.1:8484/anthropic")?);
    assert!(itself("http://localhost:8484/anthropic")?);
    assert!(!itself("http://127.0.0.1:8485/anthropic")?);
    assert!(!itself("https://api.anthropic.com")?);
    Ok(())
}

#[test]
fn an_upstream_with_no_scheme_is_refused_by_name_before_it_is_asked_anything() {
    let refused = parsed("who:secret@127.0.0.1:8484/anthropic?key=k");
    let Err(lys_home::proxy::error::ProxyError::BadUpstream { base, .. }) = refused else {
        panic!("a base with no scheme parsed: {refused:?}");
    };
    assert!(
        !base.contains("secret") && !base.contains("key=k"),
        "{base}"
    );
}
