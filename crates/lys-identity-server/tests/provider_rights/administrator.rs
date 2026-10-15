//! Administrator standing is usable by the product's own pass verifier.

use std::collections::BTreeSet;

use identity_contract::apps::{
    Auth, BEA, FILES, NOTES, TestResult, login, ok, op, post, put, registered, root,
    workspace_schema,
};
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity_server::dev_seed::Seeded;
use lys_identity_server::grants_batch::ADMINISTRATOR as RECEIPT;
use lys_pass::{Decision, Mode};
use serde_json::json;

use super::fixture::{now, refreshed, signed_in, verified, world};

async fn placed(service: &Service, credential: &str, app: &str, id: &str) -> TestResult {
    ok(post(
        service,
        &format!("/apps/{app}/placements"),
        Auth::Bearer(credential),
        &json!({
            "operation": op()?,
            "child": {"kind": format!("{app}.channel"), "id": id},
            "parent": {"kind": format!("{app}.workspace"), "id": "team"},
            "restricted": id == "private",
        }),
    )
    .await?)?;
    Ok(())
}

async fn prepared() -> Result<(Service, Seeded, String, String), Box<dyn std::error::Error>> {
    let (service, seeded) = world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let credential = registered(&service, &admin, NOTES).await?;
    let foreign = registered(&service, &admin, FILES).await?;
    let mut schema = workspace_schema(NOTES);
    schema["kinds"][format!("{NOTES}.channel")]["actions"] = json!(["export", "read", "write"]);
    schema["kinds"][format!("{NOTES}.channel")]["roles"] =
        json!({"steward": ["export", "read", "write"]});
    ok(put(
        &service,
        &format!("/apps/{NOTES}/schema"),
        Auth::Cookie(&admin),
        &json!({"operation": op()?, "replaces": 1, "schema": schema}),
    )
    .await?)?;
    for id in ["general", "private"] {
        placed(&service, &credential, NOTES, id).await?;
    }
    placed(&service, &foreign, FILES, "elsewhere").await?;
    Ok((service, seeded, admin, credential))
}

#[tokio::test]
async fn an_administrator_pass_without_grants_carries_every_known_resource_and_all_actions()
-> TestResult {
    let (service, seeded, admin, credential) = prepared().await?;
    let answer = signed_in(&service, &admin, NOTES).await?;
    let pass = verified(&service, &answer, NOTES).await?;
    assert_eq!(pass.claims().sub, seeded.people[0].id.to_string());
    assert_eq!(pass.claims().rights_truncated, None);
    let expected = [
        (
            format!("{NOTES}.channel"),
            "general",
            vec!["export", "read", "write"],
        ),
        (
            format!("{NOTES}.channel"),
            "private",
            vec!["export", "read", "write"],
        ),
        (format!("{NOTES}.workspace"), "team", vec!["read", "write"]),
    ];
    assert_eq!(pass.claims().rights.len(), expected.len());
    for (right, (kind, id, actions)) in pass.claims().rights.iter().zip(expected) {
        assert_eq!(right.resource.kind, kind);
        assert_eq!(right.resource.id, id);
        assert_eq!(right.actions, actions);
        assert_eq!(right.mode, Mode::Outright);
        assert_eq!(right.grant, RECEIPT);
        for action in actions {
            assert_eq!(
                pass.evaluate(&kind, id, action, now()?)?,
                Decision::Allowed { grant: RECEIPT }
            );
        }
    }
    for (kind, id) in [
        (format!("{NOTES}.undeclared"), "general"),
        (format!("{NOTES}.channel"), "unknown"),
    ] {
        assert_eq!(pass.evaluate(&kind, id, "read", now()?)?, Decision::Refused);
    }
    // A cached pass reading must see a new placement on the next issue.
    placed(&service, &credential, NOTES, "new").await?;
    let (status, answer) = refreshed(&service, NOTES, &answer).await?;
    assert_eq!(status, 200);
    let pass = verified(&service, &answer, NOTES).await?;
    assert_eq!(pass.claims().rights.len(), 4);
    assert_eq!(
        pass.evaluate(&format!("{NOTES}.channel"), "new", "export", now()?)?,
        Decision::Allowed { grant: RECEIPT }
    );
    Ok(())
}

#[tokio::test]
async fn a_non_administrator_pass_without_grants_still_carries_no_rights() -> TestResult {
    let (service, seeded, _, _) = prepared().await?;
    let bea = service.sign_in(login(BEA)).await?;
    let answer = signed_in(&service, &bea, NOTES).await?;
    let pass = verified(&service, &answer, NOTES).await?;
    assert_eq!(pass.claims().sub, seeded.people[1].id.to_string());
    assert!(pass.claims().rights.is_empty());
    assert_eq!(
        pass.evaluate(&format!("{NOTES}.workspace"), "team", "read", now()?)?,
        Decision::Refused
    );
    Ok(())
}

#[tokio::test]
async fn administrator_pass_rights_merge_grant_resources_without_duplicate_entries() -> TestResult {
    let (service, seeded, admin, _) = prepared().await?;
    let administrator = seeded.people[0].id.to_string();
    let bea = seeded.people[1].id.to_string();
    for (subject, kind, id, relation) in [
        (
            &administrator,
            format!("{NOTES}.workspace"),
            "team",
            "member",
        ),
        (&administrator, format!("{NOTES}.doc"), "own", "reader"),
        (&bea, format!("{NOTES}.doc"), "another", "editor"),
        (&administrator, format!("{FILES}.doc"), "foreign", "editor"),
    ] {
        ok(root(&service, &admin, subject, (&kind, id), relation).await?)?;
    }
    let answer = signed_in(&service, &admin, NOTES).await?;
    let pass = verified(&service, &answer, NOTES).await?;
    let resources: BTreeSet<_> = pass
        .claims()
        .rights
        .iter()
        .map(|right| (&right.resource.kind, &right.resource.id))
        .collect();
    assert_eq!(resources.len(), 5);
    assert_eq!(resources.len(), pass.claims().rights.len());
    for id in ["own", "another"] {
        assert_eq!(
            pass.evaluate(&format!("{NOTES}.doc"), id, "write", now()?)?,
            Decision::Allowed { grant: RECEIPT }
        );
    }
    assert!(
        pass.claims()
            .rights
            .iter()
            .all(|right| right.grant == RECEIPT)
    );
    Ok(())
}
