//! Rendering contracts live in the home. The server passes neutral fields
//! and never selects a product; the registry refuses an unknown identifier.
use serde_json::{Map, Value, json};

use super::claude_code::HARNESS;
/// Quote one shell word without changing its bytes.
pub use super::claude_code::launch::shell_word;
use super::claude_code::template::{FILL_RESUME_BY_PATH, parse_template};
use super::description::FurtherModels;
use super::launch_fields::{
    Channel, LaunchFields, LaunchMcp, ProfileLaunchFields, SeatFields, Transport,
};
use super::skills::SkillFile;

/// A resolved secret, carried only as a handle identifier.
pub struct SecretBinding {
    /// The environment variable assigned the handle.
    pub env: String,
    /// The opaque handle identifier, never its value.
    pub handle: String,
}

/// A template and its content identity after native-parser validation.
pub struct RenderedTemplate {
    /// The native template format selected by this rendering contract.
    pub harness: String,
    /// The exact template bytes, as text.
    pub text: String,
    /// The hash checked by the home.
    pub sha256: String,
}

/// A refusal names the contract member that cannot be represented.
#[derive(Debug, thiserror::Error)]
#[error("rendering contract {contract}: {member}: {reason}")]
pub struct RenderRefusal {
    /// The registry identifier.
    pub contract: String,
    /// The member the refusal concerns.
    pub member: String,
    /// Why the member cannot be represented.
    pub reason: String,
}

pub(super) fn refused(
    fields: &LaunchFields,
    member: &str,
    reason: &(impl ToString + ?Sized),
) -> RenderRefusal {
    RenderRefusal {
        contract: fields.harness.description.rendering_contract.clone(),
        member: member.to_owned(),
        reason: reason.to_string(),
    }
}

/// The variable a run's model calls are sent to Lys's proxy through.
pub const PROXY_VARIABLE: &str = "ANTHROPIC_BASE_URL";

/// The variable a rendered template says a run's key in: the key put first
/// on the path of the proxy's address the run is given
/// ([`LaunchFields::run`]).
pub const RUN_VARIABLE: &str = "LYS_RUN";

/// The variable a runner names its own `lys` program in, for every run: the
/// program the run's status line runs.
pub const PROGRAM_VARIABLE: &str = "LYS_PROGRAM";

/// The variable a runner names its own socket in, for the same run: where
/// the run's status line hands what the harness reported.
pub const SOCKET_VARIABLE: &str = "LYS_RUNNER_SOCKET";

/// The status line every Claude Code run is given. The proxy counts a
/// proxied run's calls; the cost in dollars, the running time and a plan's
/// windows are what the harness itself reports to its status line, and this
/// command hands them to the runner. It names the two variables the runner
/// sets, never a path of its own, so one template writes one sequence of
/// bytes on every machine. A run no runner started (`lys home launch` run by
/// hand) has neither variable, and its line says so rather than running an
/// empty program on an empty socket.
pub const STATUS_LINE_COMMAND: &str = "if [ -n \"$LYS_PROGRAM\" ] && [ -n \"$LYS_RUNNER_SOCKET\" ]; then \"$LYS_PROGRAM\" runner status-line --socket \"$LYS_RUNNER_SOCKET\"; else echo 'Lys: not counted (no runner started this run)'; fi";

/// Inputs retaining optional seat declarations alongside the core launch fields.
pub trait TemplateFields {
    /// The declarations shared by every launch.
    fn launch_fields(&self) -> &LaunchFields;
    /// The explicitly supplied seat declarations, when carried by this input.
    fn seat_fields(&self) -> Option<&SeatFields>;
}

impl TemplateFields for LaunchFields {
    fn launch_fields(&self) -> &LaunchFields {
        self
    }
    fn seat_fields(&self) -> Option<&SeatFields> {
        None
    }
}

impl TemplateFields for ProfileLaunchFields {
    fn launch_fields(&self) -> &LaunchFields {
        &self.launch
    }
    fn seat_fields(&self) -> Option<&SeatFields> {
        Some(&self.seat)
    }
}

impl<T: TemplateFields + ?Sized> TemplateFields for &T {
    fn launch_fields(&self) -> &LaunchFields {
        T::launch_fields(self)
    }
    fn seat_fields(&self) -> Option<&SeatFields> {
        T::seat_fields(self)
    }
}

pub(crate) fn filtered_seat(seat: &SeatFields) -> SeatFields {
    let mut seat = seat.clone();
    if let Some(settings) = &mut seat.settings {
        settings.retain(|key, _| {
            !matches!(
                key.as_str(),
                "env" | "statusLine" | "permissions" | "hooks" | "enabledPlugins"
            )
        });
    }
    seat
}

/// Run the seat's status command after Lys with the same input and one output line.
pub fn status_line_command(command: Option<&str>) -> String {
    let Some(command) = command else {
        return STATUS_LINE_COMMAND.to_owned();
    };
    let command = shell_word(command);
    format!(
        "lys_status_input=$(cat && printf '.') || exit $?; lys_status_input=${{lys_status_input%.}}; \
         lys_status_words=$(printf '%s' \"$lys_status_input\" | {{ {STATUS_LINE_COMMAND}; }}) || exit $?; \
         printf '%s' \"$lys_status_words\" || exit $?; \
         lys_seat_words=$(printf '%s' \"$lys_status_input\" | sh -c {command}) || {{ \
         lys_status_exit=$?; printf 'seat status line failed (exit %s)\\n' \"$lys_status_exit\" >&2; exit \"$lys_status_exit\"; }}; \
         printf ' | ' || exit $?; printf '%s' \"$lys_seat_words\" | tr '\\n' ' ' || exit $?; printf '\\n'"
    )
}

/// The proxy's address with `run` put first on its path:
/// `http://host:port/anthropic` becomes `http://host:port/<run>/anthropic`.
/// None when `proxy` is not an address with a scheme and a host.
#[must_use]
pub fn keyed(proxy: &str, run: &str) -> Option<String> {
    let (scheme, rest) = proxy.split_once("://")?;
    let (authority, path) = match rest.find('/') {
        Some(at) => rest.split_at(at),
        None => (rest, ""),
    };
    if scheme.is_empty() || authority.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{authority}/{run}{path}"))
}

/// The address a Codex run is given for the proxy: the proxy's own host,
/// the run's key first on the path when the run has one, then `/openai/v1`,
/// whatever path the configured address names for Anthropic. None when
/// `proxy` is not an address with a scheme and a host.
#[must_use]
pub fn openai_base(proxy: &str, run: Option<&str>) -> Option<String> {
    let (scheme, rest) = proxy.split_once("://")?;
    let authority = rest.split('/').next().filter(|host| !host.is_empty())?;
    if scheme.is_empty() {
        return None;
    }
    Some(run.map_or_else(
        || format!("{scheme}://{authority}/openai/v1"),
        |run| format!("{scheme}://{authority}/{run}/openai/v1"),
    ))
}

enum Contract {
    Native,
    Codex,
}

fn resolve(contract: &str) -> Option<Contract> {
    match contract {
        "claude-code/template-v1" => Some(Contract::Native),
        "codex/template-v1" => Some(Contract::Codex),
        _ => None,
    }
}

/// Whether the rendering registry can resolve this explicit contract.
pub fn registered(contract: &str) -> bool {
    resolve(contract).is_some()
}

/// Resolve the explicit rendering contract; display names are never examined.
pub fn render(
    input: &impl TemplateFields,
    skills: &[SkillFile],
    permissions: &Value,
    secrets: &[SecretBinding],
) -> Result<RenderedTemplate, RenderRefusal> {
    let fields = input.launch_fields();
    let mut rendered = match resolve(&fields.harness.description.rendering_contract) {
        Some(Contract::Native) => native_template(fields, skills, permissions, secrets),
        Some(Contract::Codex) => {
            super::codex::launch_template::render(fields, skills, permissions, secrets)
        }
        None => Err(refused(
            fields,
            "description.rendering_contract",
            "unknown identifier",
        )),
    }?;
    if let Some(seat) = input.seat_fields() {
        let seat = filtered_seat(seat);
        seat.check(&fields.harness.description.rendering_contract)
            .map_err(|error| refused(fields, &error.field, error.reason))?;
        let Value::Object(slots) =
            serde_json::to_value(&seat).map_err(|error| refused(fields, "seat", &error))?
        else {
            return Err(refused(
                fields,
                "seat",
                "the seat declarations are not an object",
            ));
        };
        if !slots.is_empty() {
            let mut value: Value = serde_json::from_str(&rendered.text)
                .map_err(|error| refused(fields, "template", &error))?;
            let target = if rendered.harness == HARNESS {
                value.get_mut("slots").and_then(Value::as_object_mut)
            } else {
                value.as_object_mut()
            }
            .ok_or_else(|| refused(fields, "template", "the native slot object is missing"))?;
            target.extend(slots);
            rendered.text = serde_json::to_string_pretty(&value)
                .map_err(|error| refused(fields, "template", &error))?;
            crate::record::blocks::Hash::of(rendered.text.as_bytes())
                .as_str()
                .clone_into(&mut rendered.sha256);
        }
    }
    Ok(rendered)
}

fn flags(fields: &LaunchFields) -> Result<Vec<String>, RenderRefusal> {
    let mut flags = Vec::new();
    if let Some((model, further)) = fields.models.split_first() {
        flags.extend(["--model".to_owned(), model.clone()]);
        if !further.is_empty() {
            match &fields.harness.description.models.further_encoding {
                FurtherModels::Delimited { separator } if separator == "," => {
                    flags.extend(["--fallback-model".to_owned(), further.join(separator)]);
                }
                _ => {
                    return Err(refused(
                        fields,
                        "description.models.further_encoding",
                        "this rendering contract requires comma-separated further models",
                    ));
                }
            }
        }
    }
    let waking: Vec<String> = fields
        .mcp_servers
        .iter()
        .filter(|server| server.channel == Channel::Wake)
        .map(|server| format!("server:{}", server.name))
        .collect();
    if !waking.is_empty() {
        flags.push("--channels".to_owned());
        flags.extend(waking);
    }
    Ok(flags)
}

fn server_entry(fields: &LaunchFields, server: &LaunchMcp) -> Result<Value, RenderRefusal> {
    match &server.transport {
        Transport::Http { url } => Ok(json!({"type": "http", "url": url})),
        Transport::Stdio {
            program,
            args,
            cwd,
            env,
            handles,
        } => {
            if cwd.is_some() {
                return Err(refused(
                    fields,
                    &format!("mcp_servers.{}.cwd", server.name),
                    "this rendering contract has no command working-directory slot",
                ));
            }
            let mut vars = Map::new();
            for one in env {
                vars.insert(one.name.clone(), json!(one.text));
            }
            for one in handles {
                vars.insert(one.name.clone(), json!(one.handle_id));
            }
            Ok(json!({"type": "stdio", "command": program, "args": args, "env": vars}))
        }
    }
}

fn native_template(
    fields: &LaunchFields,
    skills: &[SkillFile],
    permissions: &Value,
    secrets: &[SecretBinding],
) -> Result<RenderedTemplate, RenderRefusal> {
    if permissions
        .get("default_mode")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(refused(
            fields,
            "permissions",
            "choose how this agent is confined",
        ));
    }
    let mut servers = Map::new();
    for server in &fields.mcp_servers {
        servers.insert(server.name.clone(), server_entry(fields, server)?);
    }
    let mut body = json!({
        "harness": HARNESS, "flags": flags(fields)?,
        "slots": {
            "transcript": {"fill": FILL_RESUME_BY_PATH, "canon": null},
            "mcp": {"mcpServers": servers},
            "env": {"LYS_AGENT": fields.identity.agent, "LYS_SESSION": fields.identity.session,
                "LYS_MACHINE": fields.identity.machine, "LYS_PROVISIONING_VERSION": fields.identity.version.to_string()},
            "secrets": {"use_only": secrets.iter().map(|held| json!({"env": held.env, "handle": held.handle})).collect::<Vec<_>>(),
                "readable": [], "reader": ""},
            "instructions": fields.instructions
        }
    });
    if let Some(proxy) = &fields.model_proxy {
        // The profile's own base would be lost under the proxy's, silently.
        if secrets.iter().any(|held| held.env == PROXY_VARIABLE) {
            return Err(refused(
                fields,
                &format!("secrets.{PROXY_VARIABLE}"),
                "the profile names its own ANTHROPIC_BASE_URL and Lys's model proxy would replace it; \
                 remove it from the profile, or start this agent where no model proxy is configured",
            ));
        }
        match &fields.run {
            // The run's key goes first on the proxy's path and in the run's
            // environment, from which the start tells the runner the key.
            Some(run) => {
                if !crate::proxy::usage::is_run_key(run) {
                    return Err(refused(
                        fields,
                        "run",
                        "a run key is 32 lowercase hexadecimal digits, as a launch mints it",
                    ));
                }
                let address = keyed(proxy, run).ok_or_else(|| {
                    refused(
                        fields,
                        "model_proxy",
                        "the model proxy's address has no host a run key can follow",
                    )
                })?;
                body["slots"]["env"][PROXY_VARIABLE] = json!(address);
                body["slots"]["env"][RUN_VARIABLE] = json!(run);
            }
            None => body["slots"]["env"][PROXY_VARIABLE] = json!(proxy),
        }
    }
    if !skills.is_empty() {
        body["slots"]["skills"] = json!(skills);
    }
    if !fields.plugins.is_empty() {
        body["slots"]["plugins"] = json!(fields.plugins);
    }
    let native = super::rendering_permissions::render(permissions)
        .map_err(|error| refused(fields, "permissions", &error))?;
    let empty = native.as_object().is_some_and(|members| {
        members
            .values()
            .all(|value| value.as_array().is_some_and(Vec::is_empty))
    });
    if !empty {
        body["slots"]["permissions"] = native;
    }
    let bytes =
        serde_json::to_vec_pretty(&body).map_err(|error| refused(fields, "template", &error))?;
    let parsed = parse_template(&bytes).map_err(|error| refused(fields, "template", &error))?;
    let text = String::from_utf8(bytes).map_err(|error| refused(fields, "template", &error))?;
    Ok(RenderedTemplate {
        harness: HARNESS.to_owned(),
        text,
        sha256: parsed.hash.as_str().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::registered;

    #[test]
    fn only_registered_contracts_are_answered_as_registered() {
        assert!(registered("claude-code/template-v1"));
        assert!(registered("codex/template-v1"));
        assert!(!registered("Claude Code"));
        assert!(!registered(""));
    }
}
