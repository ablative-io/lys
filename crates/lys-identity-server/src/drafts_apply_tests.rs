use crate::drafts_test_support::Table;
use lys_identity::{AuthMethod, OperationId};
use serde_json::{Value, json};
use std::error::Error;

async fn draft(
    table: &Table,
    path: &str,
    target: Value,
    body: Value,
) -> Result<(String, Value), Box<dyn Error>> {
    let id = OperationId::generate()?.to_string();
    let value=json!({"operation":id,"target":target,"method":"POST","path":path,"body":body.to_string(),"note":"review"}).to_string().into_bytes();
    let (header, _) = table.header(&value)?;
    let (status, answer) = table
        .service
        .post_signed("/drafts", (crate::agent_signature::HEADER, &header), value)
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok((id, answer))
}

#[tokio::test]
async fn approval_applies_provisioning_once_as_the_person_and_lists_its_receipt()
-> Result<(), Box<dyn Error>> {
    let table = Table::start().await?;
    let path = format!("/agents/{}/provisioning", table.agent);
    let target = json!({"kind":"agent","id":table.agent,"action":"agent.provisioning.set"});
    let (id,created)=draft(&table,&path,target,json!({"operation":OperationId::generate()?.to_string(),"from_version":0,"model_access":[],"tools":[],"skills":[],"mcp_servers":[],"instructions":"drafted","note":"reviewed"})).await?;
    let body = json!({"operation":OperationId::generate()?.to_string(),"creation_hash":created["creation_hash"],"application":OperationId::generate()?.to_string()});
    let (status, answer) = table
        .service
        .post(&format!("/drafts/{id}/approve"), Some(&table.owner), &body)
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["execution"]["state"], "executed", "{answer}");
    let (_, profile) = table.service.get(&path, Some(&table.owner)).await?;
    assert_eq!(profile["profile"]["instructions"], "drafted");
    assert_eq!(profile["profile"]["operation"], body["application"]);
    let index = answer["execution"]["receipt_index"]
        .as_u64()
        .ok_or("receipt absent")?;
    let (_, receipt) = table
        .service
        .get(&format!("/receipts/{index}"), None)
        .await?;
    assert_eq!(receipt["receipt"]["actor"]["draft"], id);
    assert_eq!(receipt["receipt"]["actor"]["authentication"], "oidc");
    let size = table.service.log_size().await?;
    let (_, retry) = table
        .service
        .post(&format!("/drafts/{id}/approve"), Some(&table.owner), &body)
        .await?;
    assert_eq!(answer, retry);
    assert_eq!(table.service.log_size().await?, size);
    let mut directory = table.directory()?;
    let held = directory
        .projection()?
        .draft(id.parse()?)
        .ok_or("draft absent")?;
    assert_eq!(
        held.approved
            .as_ref()
            .ok_or("approval absent")?
            .0
            .actor
            .provenance()
            .method(),
        AuthMethod::Oidc
    );
    let (_, listed) = table
        .service
        .get("/drafts?state=decided", Some(&table.owner))
        .await?;
    assert_eq!(listed["drafts"][0]["state"], "executed");
    assert_eq!(listed["drafts"][0]["execution"]["receipt_index"], index);
    Ok(())
}

#[tokio::test]
async fn a_crash_after_approval_resumes_once_and_route_refusals_close_the_draft()
-> Result<(), Box<dyn Error>> {
    use lys_identity::draft_event::{Approved, DraftEvent};
    let table = Table::start().await?;
    let (id,created)=draft(&table,&format!("/agents/{}/provisioning",table.agent),json!({"kind":"agent","id":table.agent,"action":"agent.provisioning.set"}),json!({"operation":OperationId::generate()?.to_string(),"from_version":88,"model_access":[],"tools":[],"skills":[],"mcp_servers":[],"instructions":"","note":""})).await?;
    let operation = OperationId::generate()?;
    let application = OperationId::generate()?;
    let mut directory = table.directory()?;
    let held = directory
        .projection()?
        .draft(id.parse()?)
        .ok_or("draft absent")?
        .clone();
    let actor = lys_identity::Actor::new(
        held.created.actor.binding().clone(),
        lys_identity::Provenance::new(AuthMethod::Oidc, crate::session::now()),
    );
    directory.record_draft(DraftEvent::Approved(std::sync::Arc::new(Approved {
        operation,
        application,
        actor,
        recorded_at: crate::session::now(),
        draft: id.parse()?,
        draft_hash: held.hash,
        evidence: None,
    })))?;
    drop(directory);
    let body = json!({"operation":operation.to_string(),"application":application.to_string(),"creation_hash":created["creation_hash"]});
    let (status, answer) = table
        .service
        .post(&format!("/drafts/{id}/approve"), Some(&table.owner), &body)
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["execution"]["state"], "refused_on_execution");
    assert_eq!(answer["execution"]["refusal"], "ProvisioningChanged");
    let size = table.service.log_size().await?;
    let (_, retry) = table
        .service
        .post(&format!("/drafts/{id}/approve"), Some(&table.owner), &body)
        .await?;
    assert_eq!(retry, answer);
    assert_eq!(table.service.log_size().await?, size);
    Ok(())
}

#[tokio::test]
async fn draft_target_must_equal_the_declared_route_kind_action_and_id()
-> Result<(), Box<dyn Error>> {
    let table = Table::start().await?;
    for (field, value) in [
        ("id", "another-agent"),
        ("action", "agent.start"),
        ("kind", "person"),
    ] {
        let mut body = table.body(OperationId::generate()?);
        body["target"] = json!({"kind":"agent","id":table.agent,"action":"agent.provisioning.set"});
        body["path"] = json!(format!("/agents/{}/provisioning", table.agent));
        body["target"][field] = json!(value);
        let bytes = body.to_string().into_bytes();
        let (header, _) = table.header(&bytes)?;
        let before = table.service.log_size().await?;
        let (status, answer) = table
            .service
            .post_signed("/drafts", (crate::agent_signature::HEADER, &header), bytes)
            .await?;
        assert_eq!(status, 409, "{answer}");
        assert_eq!(answer["refusal"], "DraftChangeInvalid");
        let reason = answer["reason"].as_str().ok_or("reason absent")?;
        assert!(reason.contains(value));
        assert!(reason.contains(&table.agent));
        assert_eq!(table.service.log_size().await?, before);
    }
    Ok(())
}

#[tokio::test]
async fn execution_keeps_the_routes_own_not_held_and_unknown_profile_refusals()
-> Result<(), Box<dyn Error>> {
    let table = Table::start().await?;
    for (path, target, body, expected) in [
        (
            "/grants/check".to_owned(),
            json!({"kind":"grant","id":"all","action":"grant.check"}),
            json!({"route":"api","resource":{"kind":"person","id":table.person},"action":"write"}),
            "NotHeld",
        ),
        (
            format!("/agents/{}/provisioning/999/review", table.agent),
            json!({"kind":"agent","id":table.agent,"action":"agent.provisioning.review"}),
            json!({"operation":OperationId::generate()?.to_string()}),
            "ProfileVersionUnknown",
        ),
    ] {
        let (id, created) = draft(&table, &path, target, body).await?;
        let body = json!({"operation":OperationId::generate()?.to_string(),"application":OperationId::generate()?.to_string(),"creation_hash":created["creation_hash"]});
        let (status, answer) = table
            .service
            .post(&format!("/drafts/{id}/approve"), Some(&table.owner), &body)
            .await?;
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer["execution"]["state"], "refused_on_execution");
        assert_eq!(answer["execution"]["refusal"], expected, "{answer}");
        let (_, listed) = table
            .service
            .get("/drafts?state=decided", Some(&table.owner))
            .await?;
        let row = listed["drafts"]
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["id"] == id))
            .ok_or("draft absent")?;
        assert_eq!(row["execution"]["refusal"], expected);
        assert!(
            row["execution"]["reason"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        );
    }
    Ok(())
}
