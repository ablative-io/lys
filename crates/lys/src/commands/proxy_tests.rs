use super::{Source, anthropic_upstream, names_itself, shown};

#[test]
fn the_login_s_own_base_is_the_upstream_when_no_flag_names_one() {
    let gateway = "https://gateway.example/anthropic".to_owned();
    assert_eq!(
        anthropic_upstream(None, Some(gateway.clone())),
        (gateway, Source::Login)
    );
    assert_eq!(
        anthropic_upstream(Some("http://flag.example".to_owned()), Some("x".to_owned())),
        ("http://flag.example".to_owned(), Source::Flag)
    );
    assert_eq!(
        anthropic_upstream(None, Some(String::new())),
        ("https://api.anthropic.com".to_owned(), Source::Default)
    );
    assert_eq!(
        anthropic_upstream(None, None),
        ("https://api.anthropic.com".to_owned(), Source::Default)
    );
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
