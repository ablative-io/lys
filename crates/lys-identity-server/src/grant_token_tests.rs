use super::grant_token_store::Tokens;
use super::grant_tokens::TokenError;
use lys_identity::grants::GrantId;
use std::error::Error;

#[test]
fn grant_token_restart_keeps_only_hash_and_named_revocation() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("tokens.json");
    let grant = GrantId::from_bytes([1; 16]);
    let mut store = Tokens::open(file.clone())?;
    let issued = store.issue(grant, 200, 100)?;
    let bytes = std::fs::read_to_string(&file)?;
    assert!(!bytes.contains(&issued.token));
    let mut reopened = Tokens::open_at(file.clone(), 100)?;
    assert_eq!(
        reopened.lookup(&issued.token, 101)?.grant,
        grant.to_string()
    );
    reopened.revoke(grant, &issued.id, 101)?;
    assert!(matches!(
        reopened.lookup(&issued.token, 101),
        Err(TokenError::Revoked)
    ));
    assert!(matches!(
        Tokens::open_at(file, 101)?.lookup(&issued.token, 101),
        Err(TokenError::Revoked)
    ));
    Ok(())
}

#[test]
fn grant_token_expiry_and_lifetime_are_explicit() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut store = Tokens::open(dir.path().join("tokens.json"))?;
    let grant = GrantId::from_bytes([1; 16]);
    assert!(matches!(
        store.issue(grant, 100, 100),
        Err(TokenError::Expiry)
    ));
    assert!(matches!(
        store.issue(grant, 86_501, 100),
        Err(TokenError::Expiry)
    ));
    let issued = store.issue(grant, 200, 100)?;
    assert!(matches!(
        store.lookup(&issued.token, 200),
        Err(TokenError::Expired)
    ));
    assert!(matches!(
        store.lookup("invalid", 101),
        Err(TokenError::Unknown)
    ));
    Ok(())
}

#[test]
fn grant_token_failed_revoke_stops_cached_admission() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("tokens.json");
    let grant = GrantId::from_bytes([1; 16]);
    let mut store = Tokens::open(file.clone())?;
    let issued = store.issue(grant, 200, 100)?;
    std::fs::remove_file(&file)?;
    std::fs::create_dir(&file)?;
    assert!(matches!(
        store.revoke(grant, &issued.id, 101),
        Err(TokenError::Unavailable(_))
    ));
    assert!(matches!(
        store.lookup(&issued.token, 101),
        Err(TokenError::Unavailable(_))
    ));
    Ok(())
}

#[test]
fn grant_token_cookie_conflict_is_refused_before_lookup() -> Result<(), Box<dyn Error>> {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(super::grant_tokens::HEADER, "opaque".parse()?);
    headers.insert(
        axum::http::header::COOKIE,
        "lys_directory_session=anything".parse()?,
    );
    assert!(matches!(
        super::grant_tokens::header(&headers),
        Err(TokenError::CookieConflict(_))
    ));
    Ok(())
}

fn fixture() -> Result<
    (
        lys_identity::projection::Projection,
        lys_identity::grants::GrantBook,
        lys_identity::PersonId,
    ),
    Box<dyn Error>,
> {
    use lys_identity::event::{Change, IdentityEvent};
    use lys_identity::grants::{
        Action, Grant, GrantBook, GrantChange, GrantEvent, GrantParts, PassOn, Relation, Resource,
        Source, Window,
    };
    use lys_identity::{
        Actor, AuthMethod, IdentityId, LoginBinding, OperationId, PersonId, Profile, Provenance,
    };
    let person = PersonId::from_bytes([1; 16]);
    let actor = Actor::new(
        LoginBinding::new("https://issuer.test", "owner")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let mut directory = lys_identity::projection::Projection::new();
    directory.apply(
        &IdentityEvent::new(
            OperationId::from_bytes([1; 16]),
            actor,
            IdentityId::Person(person),
            1,
            Change::SetupPerson {
                profile: Profile::new("owner")?,
            },
        )?,
        0,
    )?;
    let mut book = GrantBook::new();
    for n in [1, 2] {
        let operation = OperationId::from_bytes([n; 16]);
        let grant = Grant::new(GrantParts {
            id: GrantId::from_bytes([n; 16]),
            issuer: IdentityId::Person(person),
            holder: IdentityId::Person(person),
            responsible: person,
            resource: Resource::new("lys.note", "one")?,
            relation: Relation::new("viewer")?,
            actions: [Action::new("read")?].into(),
            pass_on: PassOn::UseOnly,
            source: Source::Root,
            window: Window::new(0, None)?,
            model_version: 1,
            operation,
        })?;
        book.apply(
            &GrantEvent::new(
                operation,
                IdentityId::Person(person),
                1,
                GrantChange::Issue(Box::new(grant)),
            )?,
            u64::from(n),
        )?;
    }
    Ok((directory, book, person))
}

#[test]
fn grant_token_admission_never_substitutes_another_live_grant() -> Result<(), Box<dyn Error>> {
    use lys_identity::grants::{Action, GrantChange, GrantEvent, Resource};
    use lys_identity::{IdentityId, OperationId};
    let (directory, mut book, person) = fixture()?;
    let dir = tempfile::tempdir()?;
    let mut store = Tokens::open(dir.path().join("tokens.json"))?;
    let id = GrantId::from_bytes([1; 16]);
    let issued = store.issue(id, 200, 100)?;
    let resource = Resource::new("lys.note", "one")?;
    let action = Action::new("read")?;
    assert_eq!(
        super::grant_tokens::validate_with(
            &store,
            &book,
            &directory,
            &issued.token,
            &resource,
            &action,
            101
        )?,
        id
    );
    assert!(matches!(
        super::grant_tokens::validate_with(
            &store,
            &book,
            &directory,
            &issued.token,
            &Resource::new("lys.note", "two")?,
            &action,
            101
        ),
        Err(TokenError::Scope)
    ));
    assert!(matches!(
        super::grant_tokens::validate_with(
            &store,
            &book,
            &directory,
            &issued.token,
            &resource,
            &Action::new("write")?,
            101
        ),
        Err(TokenError::Scope)
    ));
    book.apply(
        &GrantEvent::new(
            OperationId::from_bytes([3; 16]),
            IdentityId::Person(person),
            101,
            GrantChange::Revoke {
                grant: id,
                reason: "end bound grant".to_owned(),
            },
        )?,
        3,
    )?;
    assert!(
        lys_identity::grants::admission::effective(
            &book,
            &directory,
            GrantId::from_bytes([2; 16]),
            101
        )
        .is_ok()
    );
    assert!(matches!(
        super::grant_tokens::validate_with(
            &store,
            &book,
            &directory,
            &issued.token,
            &resource,
            &action,
            101
        ),
        Err(TokenError::Authority(crate::error::ServerError::Grant(
            lys_identity::grants::GrantError::Revoked { .. }
        )))
    ));
    Ok(())
}

#[test]
fn grant_token_administration_requires_its_responsible_person() -> Result<(), Box<dyn Error>> {
    use lys_identity::{IdentityId, PersonId};
    let person = PersonId::from_bytes([1; 16]);
    super::grant_tokens::responsible(IdentityId::Person(person), person)?;
    assert!(matches!(
        super::grant_tokens::responsible(IdentityId::Person(PersonId::from_bytes([2; 16])), person),
        Err(TokenError::Responsible)
    ));
    Ok(())
}

#[test]
fn grant_token_table_of_two_thousand_records_takes_another() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("tokens.json");
    let grant = GrantId::from_bytes([1; 16]);
    let entries: serde_json::Map<String, serde_json::Value> = (0..2000u32)
        .map(|n| {
            (
                format!("{n:064x}"),
                serde_json::json!({"grant":grant.to_string(),"expires_at":300,"revoked":true}),
            )
        })
        .collect();
    std::fs::write(
        &file,
        serde_json::to_vec(&serde_json::json!({"format":"lys-grant-tokens/v1","tokens":entries}))?,
    )?;
    let mut store = Tokens::open_at(file.clone(), 100)?;
    let issued = store.issue(grant, 200, 100)?;
    assert_eq!(store.lookup(&issued.token, 150)?.grant, grant.to_string());
    let reopened = Tokens::open_at(file.clone(), 150)?;
    assert_eq!(held(&file)?, 2001);
    assert_eq!(
        reopened.lookup(&issued.token, 150)?.grant,
        grant.to_string()
    );
    assert!(matches!(
        reopened.lookup(&"a".repeat(43), 150),
        Err(TokenError::Unknown)
    ));
    Ok(())
}

#[test]
fn grant_token_duplicate_persisted_digest_is_refused() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("tokens.json");
    let key = "a".repeat(64);
    let entry = serde_json::json!({"grant":GrantId::from_bytes([1;16]).to_string(),"expires_at":200,"revoked":false}).to_string();
    std::fs::write(
        &file,
        format!(
            r#"{{"format":"lys-grant-tokens/v1","tokens":{{"{key}":{entry},"{key}":{entry}}}}}"#
        ),
    )?;
    assert!(matches!(
        Tokens::open(file),
        Err(TokenError::Unavailable(_))
    ));
    Ok(())
}

#[test]
fn grant_token_cached_lookup_does_not_reopen_disk() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("tokens.json");
    let mut store = Tokens::open(file.clone())?;
    let grant = GrantId::from_bytes([1; 16]);
    let issued = store.issue(grant, 200, 100)?;
    std::fs::remove_file(file)?;
    assert_eq!(store.lookup(&issued.token, 101)?.grant, grant.to_string());
    Ok(())
}

/// The token digests the table on disk holds.
fn table(
    file: &std::path::Path,
) -> Result<serde_json::Map<String, serde_json::Value>, Box<dyn Error>> {
    let stored: serde_json::Value = serde_json::from_slice(&std::fs::read(file)?)?;
    stored
        .get("tokens")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .ok_or_else(|| "the table holds no tokens map".into())
}

fn held(file: &std::path::Path) -> Result<usize, Box<dyn Error>> {
    Ok(table(file)?.len())
}

#[test]
fn grant_token_table_prunes_expired_entries_and_keeps_revocation_until_expiry()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("tokens.json");
    let grant = GrantId::from_bytes([1; 16]);
    let mut store = Tokens::open_at(file.clone(), 100)?;
    let short = store.issue(grant, 150, 100)?;
    let revoked_short = store.issue(grant, 160, 100)?;
    let revoked = store.issue(grant, 400, 100)?;
    let live = store.issue(grant, 500, 100)?;
    store.revoke(grant, &revoked_short.id, 101)?;
    store.revoke(grant, &revoked.id, 101)?;
    assert_eq!(held(&file)?, 4);
    let fresh = store.issue(grant, 600, 200)?;
    let on_disk = table(&file)?;
    assert_eq!(
        on_disk.len(),
        3,
        "the issue's one write removed both expired"
    );
    for gone in [&short, &revoked_short] {
        assert!(!on_disk.contains_key(&gone.id));
        assert!(matches!(
            store.lookup(&gone.token, 200),
            Err(TokenError::Unknown)
        ));
    }
    assert!(matches!(
        store.lookup(&revoked.token, 200),
        Err(TokenError::Revoked)
    ));
    let reopened = Tokens::open_at(file.clone(), 200)?;
    assert!(matches!(
        reopened.lookup(&revoked.token, 300),
        Err(TokenError::Revoked)
    ));
    assert_eq!(reopened.lookup(&live.token, 300)?.grant, grant.to_string());
    assert_eq!(reopened.lookup(&fresh.token, 300)?.grant, grant.to_string());
    let later = Tokens::open_at(file.clone(), 450)?;
    let on_disk = table(&file)?;
    assert_eq!(
        on_disk.len(),
        2,
        "opening past its expiry removed the revoked entry"
    );
    assert!(!on_disk.contains_key(&revoked.id));
    assert!(matches!(
        later.lookup(&revoked.token, 450),
        Err(TokenError::Unknown)
    ));
    assert_eq!(later.lookup(&live.token, 450)?.grant, grant.to_string());
    assert_eq!(later.lookup(&fresh.token, 450)?.grant, grant.to_string());
    Ok(())
}
