#![cfg(test)]
use identity_contract::apps::{Auth, login, ok, op, post, seeded};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::json;
use std::error::Error;

#[tokio::test]
async fn seat_program_rereview_keeps_old_inputs_and_atomically_reviews_the_found_digests()
-> Result<(), Box<dyn Error>> {
    let (service, people) = seeded().await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let agent = people.people[0].agents[0].id.to_string();
    let path = format!("/agents/{agent}/provisioning");
    let spec = json!({"executable":"/opt/seat/host", "executable_sha256":"a".repeat(64),
        "arguments":["run","--config","{seat_file}"], "seat_file":"/opt/seat/seat.json",
        "seat_file_sha256":"b".repeat(64), "adapter":"acp/1"});
    let body = json!({"operation":op()?, "from_version":0, "model_access":[], "tools":[],
        "skills":[], "mcp_servers":[], "instructions":"", "note":"fixture",
        "harness":{"name":"Seat host", "program":"/opt/seat/host", "package":"seat 1",
            "description":{"rendering_contract":"seat-program/v1", "seat_program":spec,
                "models":{"minimum":0,"maximum":0,"further_encoding":{"kind":"array"}},
                "permissions":{"modes":[],"rule_forms":[]},
                "mcp":{"transports":[],"working_directory":false,"handle_variables":false,"channel_policies":[]}}}});
    ok(post(&service, &path, Auth::Cookie(&cookie), &body).await?)?;
    ok(post(&service, &format!("{path}/1/review"), Auth::Cookie(&cookie), &json!({"operation":op()?})).await?)?;
    let operation = op()?;
    let changed = json!({"operation":operation, "executable_sha256":"c".repeat(64), "seat_file_sha256":"d".repeat(64)});
    let endpoint = format!("{path}/1/seat-program-review");
    let answer = ok(post(&service, &endpoint, Auth::Cookie(&cookie), &changed).await?)?;
    assert_eq!(answer["recorded"]["version"], 2);
    assert_eq!(answer["profile"]["reviewed_by"], people.people[0].id.to_string());
    let (_, read) = service.get(&path, Some(&cookie)).await?;
    let found = &read["profile"]["harness"]["description"]["seat_program"];
    assert_eq!(found["executable_sha256"], "c".repeat(64));
    assert_eq!(found["seat_file_sha256"], "d".repeat(64));
    for field in ["arguments", "executable", "seat_file", "adapter"] {
        assert_eq!(found[field], spec[field]);
    }
    let retry = ok(post(&service, &endpoint, Auth::Cookie(&cookie), &changed).await?)?;
    assert_eq!(retry["recorded"], answer["recorded"]);
    assert_eq!(retry["versions"].as_array().ok_or("versions missing")?.len(), 2);
    Ok(())
}
