use super::*;

#[test]
fn codex_refuses_colliding_command_variables() -> TestResult {
    let mut fields = fields()?;
    fields.mcp_servers.push(LaunchMcp {
        name: "fixture".to_owned(),
        transport: Transport::Http {
            url: "https://example.test/mcp".to_owned(),
        },
        channel: Channel::Wake,
    });
    fields.mcp_servers[0].transport = Transport::Stdio {
        program: "/opt/fixture/mcp".to_owned(),
        args: vec![],
        cwd: None,
        env: vec![EnvText {
            name: "TOKEN".to_owned(),
            text: "literal".to_owned(),
        }],
        handles: vec![HandleEnv {
            name: "TOKEN".to_owned(),
            handle_id: "handle-fixture".to_owned(),
        }],
    };
    let error = render(&fields, &[], &permissions("read-only"), &[])
        .err()
        .ok_or("variable silently overwritten")?;
    assert_eq!(error.member, "mcp_servers");
    Ok(())
}

#[test]
fn codex_launch_carries_the_liminal_ticket_and_wake_policy() -> TestResult {
    let mut fields = fields()?;
    fields.mcp_servers.push(LaunchMcp {
        name: "liminal".to_owned(),
        transport: Transport::Stdio {
            program: "/opt/liminal".to_owned(),
            args: vec![
                "ticket".to_owned(),
                "--agent".to_owned(),
                "seat-7".to_owned(),
            ],
            cwd: Some("/tmp".to_owned()),
            env: vec![],
            handles: vec![],
        },
        channel: Channel::Wake,
    });
    let rendered = render(&fields, &[], &permissions("read-only"), &[])?;
    let launch = rendering_launch::render(
        "codex/template-v1",
        &fields.harness.program,
        &rendered.text,
        InstructionsMode::Keep,
    )?;
    let setting = launch
        .arguments
        .iter()
        .find_map(|arg| arg.strip_prefix("mcp_servers.liminal="))
        .ok_or("liminal MCP setting missing")?;
    let parsed: toml::Value = toml::from_str(&format!("server = {setting}"))?;
    assert_eq!(parsed["server"]["channel"].as_str(), Some("wake"));
    assert_eq!(parsed["server"]["command"].as_str(), Some("/opt/liminal"));
    assert_eq!(parsed["server"]["args"][0].as_str(), Some("ticket"));
    fields.mcp_servers[0].channel = Channel::Off;
    let off = render(&fields, &[], &permissions("read-only"), &[])?;
    let off = rendering_launch::render(
        "codex/template-v1",
        &fields.harness.program,
        &off.text,
        InstructionsMode::Keep,
    )?;
    let setting = off
        .arguments
        .iter()
        .find_map(|arg| arg.strip_prefix("mcp_servers.liminal="))
        .ok_or("liminal MCP setting missing")?;
    let parsed: toml::Value = toml::from_str(&format!("server = {setting}"))?;
    assert_eq!(parsed["server"]["channel"].as_str(), Some("off"));
    Ok(())
}

#[test]
fn codex_refuses_http_wake_by_server_and_transport_name() -> TestResult {
    let mut fields = fields()?;
    fields.mcp_servers.push(LaunchMcp {
        name: "liminal".to_owned(),
        transport: Transport::Http {
            url: "https://example.test/mcp".to_owned(),
        },
        channel: Channel::Wake,
    });
    let error = render(&fields, &[], &permissions("read-only"), &[])
        .err()
        .ok_or("unsupported transport silently admitted")?;
    assert_eq!(error.member, "mcp_servers");
    assert!(error.to_string().contains("liminal"));
    assert!(error.to_string().contains("stdio"));
    Ok(())
}
