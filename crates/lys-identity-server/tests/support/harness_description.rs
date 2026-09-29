use serde_json::{Value, json};

pub fn declared() -> Value {
    json!({"name": "Claude Code", "program": "/opt/seat/bin/claude", "package": "claude-code-seat",
        "description": {
            "models": {"minimum": 1, "maximum": null, "further_encoding": {"kind": "delimited", "separator": ","}},
            "permissions": {"modes": ["acceptEdits", "auto", "bypassPermissions", "manual", "dontAsk", "plan"], "rule_forms": ["tool_specifier"]},
            "mcp": {"transports": ["http", "stdio"], "working_directory": false, "handle_variables": true, "channel_policies": ["off", "wake"]},
            "rendering_contract": "claude-code/template-v1"
        }
    })
}
