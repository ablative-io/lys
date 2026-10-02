//! A draft an agent makes through MCP keeps the message the agent signed
//! at the MCP door as its evidence.

use super::*;

#[tokio::test]
async fn a_draft_made_through_mcp_keeps_the_signed_message_as_its_evidence() -> Result {
    let table = Table::start().await?;
    let draft = OperationId::generate()?;
    let message = serde_json::to_vec(&json!({"jsonrpc":"2.0", "id":1, "method":"tools/call",
        "params":{"name":"change", "arguments":{"method":"POST", "path":"/drafts", "body":table.body(draft)}}}))?;
    let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let nonce = crate::routes::hex(OperationId::generate()?.as_bytes());
    let payload = crate::agent_signature::payload("POST", "/mcp", &message, at, &nonce);
    let signature = crate::routes::hex(&sign_attestation(&payload, &table.key).to_cose_bytes());
    let header = format!("{} {at} {nonce} {signature}", table.agent);
    let answer: Value = reqwest::Client::new()
        .post(format!("{}/mcp", table.service.base))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header(crate::agent_signature::HEADER, &header)
        .body(message.clone())
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        answer["result"]["structuredContent"]["status"], 200,
        "{answer}"
    );
    assert!(answer["result"]["receipt"].is_object(), "{answer}");
    let mut directory = table.directory()?;
    let held = directory
        .projection()?
        .draft(draft)
        .ok_or("draft not retained")?;
    let proof = held.created.evidence.as_ref().ok_or("evidence missing")?;
    assert_eq!(
        (proof.path.as_str(), proof.body.as_slice()),
        ("/mcp", message.as_slice())
    );
    let kept = held
        .created
        .request_signature
        .as_ref()
        .ok_or("header missing")?;
    assert_eq!(
        (kept.header.as_slice(), kept.payload.as_slice()),
        (header.as_bytes(), payload.as_slice())
    );
    Ok(())
}
