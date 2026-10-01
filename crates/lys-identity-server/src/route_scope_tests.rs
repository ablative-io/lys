#![cfg(test)]

#[test]
fn assigned_collections_declare_the_action_they_exercise() {
    let cases = [
        ("/grants", "grant"),
        ("/requests", "request"),
        ("/reviews", "review"),
        ("/resources", "resource"),
        ("/secrets", "secret"),
        ("/configuration", "configuration"),
        ("/skills", "skill"),
        ("/harnesses", "harness"),
        ("/connections", "connection"),
        ("/changes", "change"),
    ];
    let mut tested = 0;
    for (path, kind) in cases {
        let (resource, action) = super::token_scope("GET", path).expect(path);
        assert_eq!(resource.kind(), kind);
        assert_eq!(resource.id(), "all");
        assert_eq!(action.as_str(), "read");
        tested += 1;
    }
    assert_eq!(tested, 10);
    assert!(super::token_scope("GET", "/me").is_err());
    assert!(super::token_scope("POST", "/me/account/email").is_err());
}

#[test]
fn assigned_parameterized_scopes_keep_the_named_resource() {
    let (resource, action) = super::token_scope("POST", "/grants/grant-one/revoke")
        .expect("grant revoke declares its scope");
    assert_eq!(resource.kind(), "grant");
    assert_eq!(resource.id(), "grant-one");
    assert_eq!(action.as_str(), "grant.revoke");
}
