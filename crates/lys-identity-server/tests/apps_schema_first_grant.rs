#![cfg(test)]
//! An app's schema may declare, per kind, the relation a newly admitted
//! person is given first (DIRECTORY-116 R2): `first_grant` names one of the
//! kind's own relations, the dry run and the write refuse any other name at
//! its pointer writing nothing, and a taken declaration is answered by the
//! schema and by the grant model the Add a person screen reads.

use identity_contract::apps::{
    Auth, NOTES, TestResult, get, login, ok, op, post, put, refused, registered, seeded,
    workspace_schema,
};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::{Value, json};

/// The fixture schema with the workspace's first grant declared as `relation`.
fn with_first_grant(relation: &str) -> Value {
    let mut schema = workspace_schema(NOTES);
    schema["kinds"][format!("{NOTES}.workspace")]["first_grant"] = json!(relation);
    schema
}

#[tokio::test]
async fn a_first_grant_naming_no_relation_of_its_kind_is_refused_at_its_pointer_writing_nothing()
-> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    let path = format!("/apps/{NOTES}/schema");
    let pointer = format!("/kinds/{NOTES}.workspace/first_grant");
    let dry = json!({"replaces": 1, "schema": with_first_grant("poster")});
    let checked = post(
        &service,
        &format!("{path}/check"),
        Auth::Cookie(&admin),
        &dry,
    )
    .await?;
    refused(&checked, 400, "schema_invalid")?;
    assert!(checked.1.to_string().contains(&pointer), "{}", checked.1);
    assert!(checked.1.to_string().contains("`poster`"), "{}", checked.1);
    let change = json!({"operation": op()?, "replaces": 1, "schema": with_first_grant("poster")});
    let written = put(&service, &path, Auth::Cookie(&admin), &change).await?;
    refused(&written, 400, "schema_invalid")?;
    assert!(written.1.to_string().contains(&pointer), "{}", written.1);
    let current = ok(get(&service, &path, Auth::Cookie(&admin)).await?)?;
    assert_eq!(current["version"], 1, "nothing was written: {current}");
    Ok(())
}

#[tokio::test]
async fn a_first_grant_naming_a_declared_relation_is_taken_and_answered_by_the_schema_and_the_model()
-> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    let path = format!("/apps/{NOTES}/schema");
    let workspace = format!("{NOTES}.workspace");
    let dry = json!({"replaces": 1, "schema": with_first_grant("member")});
    ok(post(
        &service,
        &format!("{path}/check"),
        Auth::Cookie(&admin),
        &dry,
    )
    .await?)?;
    let change = json!({"operation": op()?, "replaces": 1, "schema": with_first_grant("member")});
    ok(put(&service, &path, Auth::Cookie(&admin), &change).await?)?;
    let current = ok(get(&service, &path, Auth::Cookie(&admin)).await?)?;
    assert_eq!(current["version"], 2, "{current}");
    assert_eq!(current["schema"], with_first_grant("member"), "{current}");
    let model = ok(get(&service, "/grants/model", Auth::Cookie(&admin)).await?)?;
    assert_eq!(
        model["kinds"][&workspace]["first_grant"],
        json!("member"),
        "{model}"
    );
    assert!(
        model["kinds"][format!("{NOTES}.doc")]
            .get("first_grant")
            .is_none(),
        "a kind declaring none answers none: {model}"
    );
    Ok(())
}
