//! Reviewed process inputs are independent of a program's seat-file format.
use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::launch_fields::{LaunchFields, SeatFields};
use super::rendering::{RenderRefusal, RenderedTemplate, SecretBinding, refused};
use super::rendering_launch::Launch;
use crate::record::blocks::Hash;

/// The product-independent rendering contract for a program hosting its own seat.
pub const CONTRACT: &str = "seat-program/v1";
/// Signed metadata kept separately from the opaque product file.
pub const FILE: &str = "seat-program.json";

/// Exact process inputs retained with the person's reviewed description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Value")]
pub struct SeatProgram {
    /// The absolute executable spelling used at start.
    pub executable: String,
    /// The executable bytes reviewed by the person.
    pub executable_sha256: String,
    /// Ordered literal arguments, including exactly one seat-file placeholder.
    pub arguments: Vec<String>,
    /// The absolute path of an opaque file owned by the program.
    pub seat_file: String,
    /// The opaque file bytes reviewed by the person.
    pub seat_file_sha256: String,
    /// The versioned standard-stream adapter.
    pub adapter: String,
}

impl TryFrom<Value> for SeatProgram {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let object = value
            .as_object()
            .ok_or("seat_program: requires an object")?;
        for field in object.keys() {
            if ![
                "executable",
                "executable_sha256",
                "arguments",
                "seat_file",
                "seat_file_sha256",
                "adapter",
            ]
            .contains(&field.as_str())
            {
                return Err(format!("seat_program.{field}: is not a seat-program field"));
            }
        }
        let text = |field: &str| {
            object
                .get(field)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("seat_program.{field}: is missing or is not literal text"))
        };
        let arguments = object
            .get("arguments")
            .and_then(Value::as_array)
            .ok_or("seat_program.arguments: is missing or is not a list")?
            .iter()
            .map(|argument| {
                argument.as_str().map(str::to_owned).ok_or_else(|| {
                    "seat_program.arguments: every argument must be literal text".to_owned()
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let spec = Self {
            executable: text("executable")?,
            executable_sha256: text("executable_sha256")?,
            arguments,
            seat_file: text("seat_file")?,
            seat_file_sha256: text("seat_file_sha256")?,
            adapter: text("adapter")?,
        };
        spec.check()?;
        Ok(spec)
    }
}

impl SeatProgram {
    /// Validate reviewed paths, digests, literals and the declared adapter.
    ///
    /// # Errors
    /// Names the malformed member without quoting argument or file contents.
    pub fn check(&self) -> Result<(), String> {
        for (field, path) in [
            ("executable", &self.executable),
            ("seat_file", &self.seat_file),
        ] {
            if !Path::new(path).is_absolute()
                || !super::rendering_launch::command_or_absolute_path(path)
            {
                return Err(format!(
                    "seat_program.{field}: an absolute file path is required"
                ));
            }
        }
        for (field, digest) in [
            ("executable_sha256", &self.executable_sha256),
            ("seat_file_sha256", &self.seat_file_sha256),
        ] {
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(format!(
                    "seat_program.{field}: 64 lowercase hexadecimal SHA-256 digits are required"
                ));
            }
        }
        if self
            .arguments
            .iter()
            .filter(|argument| argument.as_str() == "{seat_file}")
            .count()
            != 1
            || self
                .arguments
                .iter()
                .any(|argument| argument.contains('\0'))
        {
            return Err("seat_program.arguments: literal strings with exactly one {seat_file} argument and no null bytes are required".to_owned());
        }
        if self.adapter != "acp/1" {
            return Err("seat_program.adapter: the registered ACP adapter is acp/1".to_owned());
        }
        Ok(())
    }

    /// The reviewed literals, with only the entire placeholder argument replaced.
    #[must_use]
    pub fn argv(&self) -> Vec<String> {
        self.arguments
            .iter()
            .map(|argument| {
                if argument == "{seat_file}" {
                    self.seat_file.clone()
                } else {
                    argument.clone()
                }
            })
            .collect()
    }
}

/// Reject native seat settings rather than silently omitting them.
///
/// # Errors
/// Names the native member that a seat file must own.
pub fn check_seat(seat: &SeatFields) -> Result<(), String> {
    let value = serde_json::to_value(seat).map_err(|error| format!("seat_program: {error}"))?;
    if let Some(field) = value.as_object().and_then(|members| members.keys().next()) {
        return Err(format!(
            "{field}: native fields are unavailable for {CONTRACT}; the reviewed seat file owns the program's settings"
        ));
    }
    Ok(())
}

pub(super) fn render(
    fields: &LaunchFields,
    seat: Option<&SeatFields>,
    permissions: &Value,
    secrets: &[SecretBinding],
) -> Result<RenderedTemplate, RenderRefusal> {
    let fail = |field: &str, reason: &str| refused(fields, field, reason);
    let spec = fields
        .harness
        .description
        .seat_program
        .as_ref()
        .ok_or_else(|| {
            fail(
                "description.seat_program",
                "the reviewed process inputs are missing",
            )
        })?;
    spec.check()
        .map_err(|error| fail("description.seat_program", &error))?;
    if fields.harness.program != spec.executable {
        return Err(fail(
            "harness.program",
            "must equal seat_program.executable",
        ));
    }
    if let Some(seat) = seat {
        check_seat(seat).map_err(|error| fail("seat", &error))?;
    }
    for (field, present) in [
        ("models", !fields.models.is_empty()),
        ("mcp_servers", !fields.mcp_servers.is_empty()),
        ("skills", !fields.skills.is_empty()),
        ("plugins", !fields.plugins.is_empty()),
        ("instructions", !fields.instructions.is_empty()),
        ("secrets", !secrets.is_empty()),
    ] {
        if present {
            return Err(fail(field, "the reviewed seat file owns this setting"));
        }
    }
    if let Some(members) = permissions.as_object() {
        for (field, value) in members {
            if !value.is_null() && !value.as_array().is_some_and(Vec::is_empty) {
                return Err(fail(
                    &format!("permissions.{field}"),
                    "native confinement is unavailable for a seat program",
                ));
            }
        }
    }
    let mut environment = BTreeMap::from([
        ("LYS_AGENT", fields.identity.agent.clone()),
        ("LYS_SESSION", fields.identity.session.clone()),
        ("LYS_MACHINE", fields.identity.machine.clone()),
        (
            "LYS_PROVISIONING_VERSION",
            fields.identity.version.to_string(),
        ),
    ]);
    if let Some(run) = &fields.run {
        if !crate::proxy::usage::is_run_key(run) {
            return Err(fail("run", "a run key is 32 lowercase hexadecimal digits"));
        }
        environment.insert("LYS_RUN", run.clone());
    }
    if let Some(proxy) = &fields.model_proxy {
        let run = fields
            .run
            .as_deref()
            .ok_or_else(|| fail("run", "the proxy requires a run key"))?;
        let anthropic = super::rendering::keyed(proxy, run)
            .ok_or_else(|| fail("model_proxy", "the proxy address has no host"))?;
        let openai = super::rendering::openai_base(proxy, Some(run))
            .ok_or_else(|| fail("model_proxy", "the proxy address has no host"))?;
        environment.insert("ANTHROPIC_BASE_URL", anthropic);
        environment.insert("OPENAI_BASE_URL", openai);
    }
    let text =
        serde_json::to_string_pretty(&json!({"seat_program": spec, "environment": environment}))
            .map_err(|error| fail("template", &error.to_string()))?;
    Ok(RenderedTemplate {
        harness: CONTRACT.to_owned(),
        sha256: Hash::of(text.as_bytes()).as_str().to_owned(),
        text,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Template {
    seat_program: SeatProgram,
    environment: BTreeMap<String, String>,
}

pub(super) fn launch(program: &str, text: &str) -> Result<Launch, String> {
    let template: Template =
        serde_json::from_str(text).map_err(|error| format!("{CONTRACT}: {error}"))?;
    template.seat_program.check()?;
    if program != template.seat_program.executable {
        return Err("harness.program: differs from seat_program.executable".to_owned());
    }
    let metadata =
        serde_json::to_string(&template.seat_program).map_err(|error| error.to_string())?;
    let metadata_sha256 = Hash::of(metadata.as_bytes()).as_str().to_owned();
    Ok(Launch {
        program: program.to_owned(),
        arguments: template.seat_program.argv(),
        environment: template.environment,
        argument_files: BTreeMap::new(),
        environment_paths: BTreeMap::new(),
        files: vec![super::rendering_launch::File {
            path: FILE.to_owned(),
            text: metadata,
            sha256: metadata_sha256,
        }],
    })
}
