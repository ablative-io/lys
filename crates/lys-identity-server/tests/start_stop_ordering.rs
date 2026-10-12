#![cfg(test)]
//! End waits for the harness exit; another start cannot overlap that wait.

#[path = "support/runner_start.rs"]
pub mod support;

use serde_json::{Value, json};
use std::error::Error;
use std::io::Write;
use std::os::unix::net::UnixListener;
use support::{Table, operation};

#[tokio::test(flavor = "multi_thread")]
async fn start_refuses_until_stop_has_reaped_the_harness() -> Result<(), Box<dyn Error>> {
    let table = Table::unprofiled().await?;
    let signal_dir = tempfile::tempdir_in("/tmp")?;
    let gate_path = signal_dir.path().join("release.sock");
    let gate = UnixListener::bind(&gate_path)?;
    let program = table.dir.path().join("declared-harness");
    let script = format!(
        "#!/usr/bin/python3\nimport signal, socket, sys, os\nfirst = not os.path.exists({:?})\nopen({:?}, 'a').close()\ndef end(signum, frame):\n if not first: sys.exit(0)\n s = socket.socket(socket.AF_UNIX)\n s.connect({:?})\n s.recv(1)\n sys.exit(0)\nsignal.signal(signal.SIGHUP, end)\nprint('profile-read', flush=True)\nsignal.pause()\n",
        table.dir.path().join("first").display().to_string(),
        table.dir.path().join("first").display().to_string(),
        gate_path.display().to_string()
    );
    std::fs::write(program, script)?;
    let provisioning = format!("/agents/{}/provisioning", table.agent());
    table
        .ok(
            &provisioning,
            &json!({
                "operation": operation()?, "from_version": 0,
                "model_access": ["claude-fable-5-1"], "tools": [], "skills": [], "mcp_servers": [],
                "instructions": "", "note": "", "harness": table.harness(),
                "permissions": {"default_mode": "plan"}, "working_folder": table.dir.path(),
                "session": {"config_dir": table.dir.path().join("config-home")}
            }),
        )
        .await?;
    table
        .ok(
            &format!("{provisioning}/1/review"),
            &json!({"operation": operation()?}),
        )
        .await?;
    let machine = table.lys_machine("Stopping writer").await?;
    let (_, profile) = table
        .service
        .get(
            &format!("/agents/{}/provisioning", table.agent()),
            Some(&table.ada),
        )
        .await?;
    let version = profile["profile"]["operation"]
        .as_str()
        .ok_or("profile has no operation")?;
    let path = format!("/agents/{}/start", table.agent());
    let body = json!({"profile_version": version, "machine": machine});
    let (status, first) = table.service.post(&path, Some(&table.ada), &body).await?;
    assert_eq!(status, 200, "{first}");
    let session = first["runner"]["session"]
        .as_str()
        .ok_or("start has no session")?;
    let client = lys_runner::Client::new(
        table.dir.path().join("runner.sock"),
        std::sync::Arc::clone(&table.server_key),
    );
    client.ask(&lys_runner::Act::Wait {
        session: session.to_owned(),
        cursor: Some(0),
        pattern: "profile-read".to_owned(),
        regex: false,
    })?;
    let stop_url = format!("{}/agents/{}/stop", table.service.base, table.agent());
    let cookie = table.ada.clone();
    let stopped = tokio::spawn(async move {
        let response = reqwest::Client::new()
            .post(stop_url)
            .header("cookie", cookie)
            .header("content-type", "application/json")
            .body(
                json!({"operation": operation().map_err(|error| error.to_string())?,
                "reason": "ordered restart"})
                .to_string(),
            )
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status().as_u16();
        let text = response.text().await.map_err(|error| error.to_string())?;
        Ok::<_, String>((
            status,
            serde_json::from_str::<Value>(&text).map_err(|error| error.to_string())?,
        ))
    });
    let (mut release, _) = gate.accept()?;
    let transition =
        json!({"operation": operation()?, "transition": "activate", "reason": "restart"});
    table
        .ok(
            &format!("/identities/{}/transitions", table.agent()),
            &transition,
        )
        .await?;
    let second = table.service.post(&path, Some(&table.ada), &body).await?;
    release.write_all(b"x")?;
    let (stop_status, stop_record) = stopped.await??;
    assert_eq!(stop_status, 200, "{stop_record}");
    assert_eq!(
        stop_record["sessions_confirmed"].as_array().map(Vec::len),
        Some(1),
        "{stop_record}"
    );
    assert_eq!(
        stop_record["sessions_confirmed"][0]["session"], session,
        "{stop_record}"
    );
    assert!(
        stop_record["sessions_confirmed"][0]["confirmation"]
            .as_str()
            .is_some_and(|confirmation| confirmation.contains("exit status 0")),
        "{stop_record}"
    );
    assert_ne!(
        second.0, 200,
        "a second writer overlapped the ending process: {}",
        second.1
    );
    assert_eq!(second.1["error"], "process_lingering", "{}", second.1);
    assert!(
        second.1["words"]
            .as_str()
            .is_some_and(|words| words.contains(session)),
        "{}",
        second.1
    );
    table.close()
}
