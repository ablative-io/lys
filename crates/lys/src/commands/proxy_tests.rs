use super::{anthropic_upstream, names_itself, shown};

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
fn an_upstream_naming_the_proxy_itself_is_found() {
    let listen = "127.0.0.1:8484";
    assert!(names_itself("http://127.0.0.1:8484/anthropic", listen));
    assert!(names_itself("http://localhost:8484/anthropic", listen));
    assert!(!names_itself("http://127.0.0.1:8485/anthropic", listen));
    assert!(!names_itself("https://api.anthropic.com", listen));
}
