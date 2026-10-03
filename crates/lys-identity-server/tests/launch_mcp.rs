//! HOME-037 R4: an MCP server is an address or a command. A command server
//! renders as its stdio entry with its exact program and arguments, each
//! nonsecret setting as its text by the one rule, each secret as the
//! agent's handle id and never a value, and a waking channel as the flag
//! Claude Code enables it by. What would carry a credential is refused when
//! the profile is recorded, naming the server and where, never the value.

#[path = "support/runner_start.rs"]
pub mod support;
use support::{PLANTED, Table, operation};

use std::error::Error;

use lys_home::harness::claude_code::template::parse_template;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

/// A credential a profile tries to carry inline; it must never be repeated.
const INLINE: &str = "abcdef0123456789";

impl Table {
    /// Record `servers` as the agent's profile, from `from`.
    async fn record(&self, from: u32, servers: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        let body = json!({
            "operation": operation()?, "from_version": from, "working_folder": "/tmp",
            "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
            "mcp_servers": servers, "instructions": "", "note": "",
            "permissions": {"default_mode": "plan"}, "harness": self.harness(),
        });
        let path = format!("/agents/{}/provisioning", self.agent());
        self.service.post(&path, Some(&self.ada), &body).await
    }

    /// Record `servers`, review them, and ask for a start on a new machine.
    async fn start_profile(
        &self,
        from: u32,
        servers: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let (status, set) = self.record(from, servers).await?;
        assert_eq!(status, 200, "{set}");
        let path = format!("/agents/{}/provisioning/{}/review", self.agent(), from + 1);
        let reviewed = json!({ "operation": operation()? });
        let (status, set) = self.service.post(&path, Some(&self.ada), &reviewed).await?;
        assert_eq!(status, 200, "{set}");
        let machine = operation()?;
        let body = json!({
            "operation": machine, "name": format!("Box {from}"), "kind": "server",
            "runtime": "manifold", "slots": 1, "may_run": [self.agent()], "may_reach": [],
        });
        let (status, named) = self
            .service
            .post("/network/machines", Some(&self.ada), &body)
            .await?;
        assert_eq!(status, 200, "{named}");
        self.ok(
            &format!("/network/machines/{machine}/runner"),
            &json!({"runner": {"kind": "lys"}}),
        )
        .await?;
        let path = format!("/agents/{}/start-command", self.agent());
        let body = json!({ "machine": machine, "operation": operation()? });
        self.service.post(&path, Some(&self.ada), &body).await
    }
}

/// The five command servers a real seat runs, with typed settings and a secret.
fn seat() -> Value {
    let plain = |name: &str| {
        json!({ "name": name, "command": { "program": format!("/opt/seat/bin/{name}"),
            "args": ["--stdio", "--seat", "archie"] } })
    };
    json!([
        plain("meridian"),
        plain("manifold"),
        plain("hammerbarn"),
        { "name": "cambium-mcp", "channel": "wake", "command": {
            "program": "/opt/seat/bin/cambium-mcp", "args": ["serve", "--workspace", "archie"],
            "env": { "CAMBIUM_QUIET": false, "CAMBIUM_RETRIES": 0, "CAMBIUM_PREFIX": "",
                     "CAMBIUM_SEAT_FILE": "/seat/archie.toml",
                     "GIT_HOST_TOKEN": { "handle": "git-host token" } } } },
        plain("excalidraw"),
    ])
}

#[tokio::test]
async fn five_command_servers_render_exactly_with_typed_settings_handles_and_a_waking_channel()
-> TestResult {
    let table = Table::unprofiled().await?;
    let (status, start) = table.start_profile(0, &seat()).await?;
    assert_eq!(status, 200, "{start}");
    assert!(
        !start.to_string().contains(PLANTED),
        "a value reached the answer"
    );
    let template = parse_template(start["template"].as_str().ok_or("no template")?.as_bytes())?;
    for name in ["meridian", "manifold", "hammerbarn", "excalidraw"] {
        assert_eq!(
            template.mcp["mcpServers"][name],
            json!({ "type": "stdio", "command": format!("/opt/seat/bin/{name}"),
                    "args": ["--stdio", "--seat", "archie"], "env": {} }),
            "{name}"
        );
    }
    assert_eq!(
        template.mcp["mcpServers"]["cambium-mcp"],
        json!({ "type": "stdio", "command": "/opt/seat/bin/cambium-mcp",
                "args": ["serve", "--workspace", "archie"],
                "env": { "CAMBIUM_QUIET": "false", "CAMBIUM_RETRIES": "0", "CAMBIUM_PREFIX": "",
                         "CAMBIUM_SEAT_FILE": "/seat/archie.toml", "GIT_HOST_TOKEN": "h-live" } }),
    );
    let channels = template.flags.iter().position(|flag| flag == "--channels");
    assert_eq!(
        channels
            .and_then(|at| template.flags.get(at + 1))
            .map(String::as_str),
        Some("server:cambium-mcp"),
        "{:?}",
        template.flags
    );
    assert_eq!(start["left_out"], json!([]));

    let (status, again) = table.start_profile(1, &seat()).await?;
    assert_eq!(status, 200, "{again}");
    let again = parse_template(again["template"].as_str().ok_or("no template")?.as_bytes())?;
    assert_eq!(
        again.mcp, template.mcp,
        "the same profile renders the same servers"
    );
    table.close()
}

#[tokio::test]
async fn a_credential_or_an_unrepresentable_setting_is_refused_by_name_when_recorded() -> TestResult
{
    let table = Table::unprofiled().await?;
    let command = |args: Value, env: Value| {
        json!([{ "name": "meridian", "command": { "program": "/opt/seat/bin/meridian",
            "args": args, "env": env } }])
    };
    let inline = [
        command(json!([format!("--token={INLINE}")]), json!({})),
        command(json!(["--api-key", INLINE]), json!({})),
        command(json!([]), json!({ "MERIDIAN_TOKEN": INLINE })),
        command(json!([format!("sk-{INLINE}")]), json!({})),
        json!([{ "name": "meridian", "url": format!("https://seat:{INLINE}@meridian.example.test/mcp") }]),
        json!([{ "name": "meridian", "url": format!("https://meridian.example.test/mcp?access_token={INLINE}") }]),
    ];
    for servers in inline {
        let (status, answer) = table.record(0, &servers).await?;
        assert_eq!(status, 400, "{answer}");
        assert_eq!(answer["refusal"], "McpCredentialInline", "{answer}");
        assert!(
            !answer.to_string().contains(INLINE),
            "the value is repeated: {answer}"
        );
    }
    for env in [
        json!({ "RATE": 1.5 }),
        json!({ "LIST": [1] }),
        json!({ "ODD": { "handle": 1 } }),
    ] {
        let (status, answer) = table.record(0, &command(json!([]), env)).await?;
        assert_eq!(status, 400, "{answer}");
        assert_eq!(answer["refusal"], "McpSettingUnrepresentable", "{answer}");
    }
    let started = |program: &str, args: Value, cwd: Value| json!([{ "name": "meridian", "command": { "program": program, "args": args, "cwd": cwd } }]);
    for inexact in [
        started(" /opt/seat/bin/meridian", json!([]), Value::Null),
        started("/opt/seat/bin/meridian ", json!([]), Value::Null),
        started("/opt/seat/bin/mer\0idian", json!([]), Value::Null),
        started(
            "/opt/seat/bin/meridian",
            json!(["--seat\0archie"]),
            Value::Null,
        ),
        started("/opt/seat/bin/meridian", json!([]), json!("")),
        started("/opt/seat/bin/meridian", json!([]), json!("  ")),
        started("/opt/seat/bin/meridian", json!([]), json!(" /seat")),
        started("/opt/seat/bin/meridian", json!([]), json!("/se\0at")),
    ] {
        let (status, answer) = table.record(0, &inexact).await?;
        assert_eq!(status, 400, "{inexact} {answer}");
        assert_eq!(
            answer["refusal"], "McpSettingUnrepresentable",
            "{inexact} {answer}"
        );
    }
    let both = json!([{ "name": "meridian", "url": "https://meridian.example.test/mcp",
        "command": { "program": "/opt/seat/bin/meridian" } }]);
    let (status, answer) = table.record(0, &both).await?;
    assert_eq!(
        (status, answer["refusal"].clone()),
        (400, json!("RequestMalformed"))
    );
    table.close()
}

#[tokio::test]
async fn a_start_refuses_a_secret_without_a_handle_and_a_directory_claude_code_cannot_take()
-> TestResult {
    let table = Table::unprofiled().await?;
    let unheld = json!([{ "name": "meridian", "command": { "program": "/opt/seat/bin/meridian",
        "env": { "MERIDIAN_AUTH": { "handle": "not held" } } } }]);
    let (status, answer) = table.start_profile(0, &unheld).await?;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["refusal"], "McpHandleUnsupported", "{answer}");
    let placed = json!([{ "name": "meridian", "command": { "program": "/opt/seat/bin/meridian",
        "cwd": "/seat" } }]);
    let (status, answer) = table.record(1, &placed).await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "McpSettingUnrepresentable", "{answer}");
    table.close()
}
