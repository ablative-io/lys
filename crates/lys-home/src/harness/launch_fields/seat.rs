//! Optional seat declarations share one validator across native templates and profiles.

use super::LaunchFields;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// One command a seat appends to a Claude hook event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookCommand {
    /// The tool or event matcher, when restricted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// The seat's command, preserved as given.
    pub command: String,
    /// Its execution budget in seconds, when declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
}

/// The optional seat declarations shared by profile views and native templates.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatFields {
    /// Commands appended per Claude hook event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks: Option<BTreeMap<String, Vec<HookCommand>>>,
    /// The seat's own status command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_line: Option<String>,
    /// Extra native settings, without the separately governed keys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<Map<String, Value>>,
    /// The existing Claude session or Codex thread to resume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
    /// The window the person declared, in tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    /// The absolute config directory explicitly chosen by a person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_dir: Option<String>,
}

/// A refused seat field with the cause of a failed conversion.
#[derive(Debug, thiserror::Error)]
#[error("{field}: {reason}{detail}")]
pub struct SeatFieldRefusal {
    /// The exact field that failed validation.
    pub field: String,
    /// The rule its value failed.
    pub reason: &'static str,
    /// The underlying conversion failure, including its separator, when present.
    pub detail: String,
}

fn seat_refused(field: impl Into<String>, reason: &'static str) -> SeatFieldRefusal {
    SeatFieldRefusal {
        field: field.into(),
        reason,
        detail: String::new(),
    }
}

fn seat_failed(
    field: impl Into<String>,
    reason: &'static str,
    error: impl std::fmt::Display,
) -> SeatFieldRefusal {
    SeatFieldRefusal {
        field: field.into(),
        reason,
        detail: format!(": {error}"),
    }
}

fn member<T: serde::de::DeserializeOwned>(
    slots: &Map<String, Value>,
    name: &str,
) -> Result<Option<T>, SeatFieldRefusal> {
    slots
        .get(name)
        .map(|value| {
            serde_json::from_value(value.clone())
                .map_err(|error| seat_failed(name, "has an invalid value shape", error))
        })
        .transpose()
}

fn hooks(
    slots: &Map<String, Value>,
) -> Result<Option<BTreeMap<String, Vec<HookCommand>>>, SeatFieldRefusal> {
    let Some(given) = slots.get("hooks") else {
        return Ok(None);
    };
    let events = given
        .as_object()
        .ok_or_else(|| seat_refused("hooks", "must be an event map"))?;
    let mut hooks = BTreeMap::new();
    for (event, commands) in events {
        let field = format!("hooks.{event}");
        let commands = commands
            .as_array()
            .ok_or_else(|| seat_refused(&field, "must be a command list"))?;
        let mut kept = Vec::new();
        for (index, command) in commands.iter().enumerate() {
            let field = format!("{field}[{index}]");
            let object = command
                .as_object()
                .ok_or_else(|| seat_refused(&field, "must be a command object"))?;
            for key in object.keys() {
                if !["matcher", "command", "timeout"].contains(&key.as_str()) {
                    return Err(seat_refused(
                        format!("{field}.{key}"),
                        "is not a hook command field",
                    ));
                }
            }
            let command = object
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| seat_refused(format!("{field}.command"), "must be command text"))?;
            let matcher = match object.get("matcher") {
                None | Some(Value::Null) => None,
                Some(Value::String(matcher)) => Some(matcher.clone()),
                Some(_) => {
                    return Err(seat_refused(
                        format!("{field}.matcher"),
                        "must be matcher text",
                    ));
                }
            };
            let timeout = match object.get("timeout") {
                None | Some(Value::Null) => None,
                Some(given) => {
                    let invalid =
                        || seat_refused(format!("{field}.timeout"), "must be positive u32 seconds");
                    let value = given.as_u64().ok_or_else(invalid)?;
                    let value = u32::try_from(value).map_err(|error| {
                        seat_failed(
                            format!("{field}.timeout"),
                            "must be positive u32 seconds",
                            error,
                        )
                    })?;
                    if value == 0 {
                        return Err(invalid());
                    }
                    Some(value)
                }
            };
            kept.push(HookCommand {
                matcher,
                command: command.to_owned(),
                timeout,
            });
        }
        hooks.insert(event.clone(), kept);
    }
    Ok(Some(hooks))
}

impl SeatFields {
    /// The optional fields accepted as template slots.
    pub const NAMES: [&'static str; 6] = [
        "hooks",
        "status_line",
        "settings",
        "resume",
        "context_window",
        "config_dir",
    ];

    /// Parse explicitly supplied slots and validate them for the named contract.
    ///
    /// # Errors
    /// Names an unknown field, invalid value or unsupported harness field.
    pub fn parse(slots: &Map<String, Value>, contract: &str) -> Result<Self, SeatFieldRefusal> {
        for key in slots.keys() {
            if !Self::NAMES.contains(&key.as_str()) {
                return Err(seat_refused(key, "is not a seat field"));
            }
        }
        let fields = Self {
            hooks: hooks(slots)?,
            status_line: member(slots, "status_line")?,
            settings: member(slots, "settings")?,
            resume: member(slots, "resume")?,
            context_window: member(slots, "context_window")?,
            config_dir: member(slots, "config_dir")?,
        };
        fields.check(contract)?;
        Ok(fields)
    }

    /// Validate every declared value before it can be stored or rendered.
    ///
    /// # Errors
    /// Names the invalid field and the rule it failed, without its contents.
    pub fn check(&self, contract: &str) -> Result<(), SeatFieldRefusal> {
        self.check_hooks(contract)?;
        self.check_status_line(contract)?;
        self.check_settings()?;
        self.check_session(contract)
    }

    fn check_hooks(&self, contract: &str) -> Result<(), SeatFieldRefusal> {
        const EVENTS: [&str; 12] = [
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PostToolUse",
            "PostToolUseFailure",
            "Stop",
            "SubagentStop",
            "PreCompact",
            "PostCompact",
            "SessionEnd",
            "Notification",
            "FileChanged",
        ];
        let claude = contract == "claude-code/template-v1";
        if let Some(hooks) = &self.hooks {
            if !claude {
                return Err(seat_refused("hooks", "requires a Claude Code harness"));
            }
            for (event, commands) in hooks {
                if !EVENTS.contains(&event.as_str()) {
                    return Err(seat_refused(
                        format!("hooks.{event}"),
                        "is not a Claude hook event",
                    ));
                }
                for (index, command) in commands.iter().enumerate() {
                    let field = format!("hooks.{event}[{index}]");
                    if command.command.trim().is_empty() || command.command.contains('\0') {
                        return Err(seat_refused(
                            format!("{field}.command"),
                            "must be nonempty command text without null bytes",
                        ));
                    }
                    if command
                        .matcher
                        .as_ref()
                        .is_some_and(|matcher| matcher.contains('\0'))
                    {
                        return Err(seat_refused(
                            format!("{field}.matcher"),
                            "must contain no null bytes",
                        ));
                    }
                    if command.timeout == Some(0) {
                        return Err(seat_refused(
                            format!("{field}.timeout"),
                            "must be positive seconds",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn check_status_line(&self, contract: &str) -> Result<(), SeatFieldRefusal> {
        let claude = contract == "claude-code/template-v1";
        if let Some(command) = &self.status_line {
            if !claude {
                return Err(seat_refused(
                    "status_line",
                    "requires a Claude Code harness",
                ));
            }
            if command.trim().is_empty() || command.contains('\0') {
                return Err(seat_refused(
                    "status_line",
                    "must be nonempty command text without null bytes",
                ));
            }
        }
        Ok(())
    }

    fn check_settings(&self) -> Result<(), SeatFieldRefusal> {
        if let Some(settings) = &self.settings {
            for key in [
                "env",
                "statusLine",
                "permissions",
                "hooks",
                "enabledPlugins",
            ] {
                if settings.contains_key(key) {
                    return Err(seat_refused(
                        format!("settings.{key}"),
                        "has its own governed profile field",
                    ));
                }
            }
        }
        Ok(())
    }

    fn check_session(&self, contract: &str) -> Result<(), SeatFieldRefusal> {
        if let Some(resume) = &self.resume {
            let shape = resume.len() == 36
                && resume.bytes().enumerate().all(|(at, byte)| {
                    if [8, 13, 18, 23].contains(&at) {
                        byte == b'-'
                    } else {
                        byte.is_ascii_hexdigit()
                    }
                });
            let reason = match contract {
                "claude-code/template-v1" => "must be a Claude session UUID",
                "codex/template-v1" => "must be a Codex thread UUID",
                _ => {
                    return Err(seat_refused(
                        "resume",
                        "requires a declared Claude Code or Codex harness",
                    ));
                }
            };
            if !shape {
                return Err(seat_refused("resume", reason));
            }
        }
        if self.context_window == Some(0) {
            return Err(seat_refused("context_window", "must be positive tokens"));
        }
        if let Some(directory) = &self.config_dir
            && (!std::path::Path::new(directory).is_absolute()
                || directory.chars().any(char::is_control))
        {
            return Err(seat_refused(
                "config_dir",
                "must be an absolute path without control characters",
            ));
        }
        Ok(())
    }
}

/// A launch view retaining optional seat declarations beside the existing core fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileLaunchFields {
    /// The core declarations retained by existing launch consumers.
    #[serde(flatten)]
    pub launch: LaunchFields,
    /// The optional seat declarations read by the native rendering stage.
    #[serde(flatten)]
    pub seat: SeatFields,
}

impl std::ops::Deref for ProfileLaunchFields {
    type Target = LaunchFields;
    fn deref(&self) -> &Self::Target {
        &self.launch
    }
}
