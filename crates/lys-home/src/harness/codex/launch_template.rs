//! Native templates retain every input so an unsupported value is refused.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::harness::launch_fields::{Channel, LaunchFields, Transport};
use crate::harness::rendering::{RenderRefusal, RenderedTemplate, SecretBinding, refused};
use crate::harness::skills::{SkillFile, check};
use crate::record::blocks::Hash;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Template {
    pub(super) harness: String,
    pub(super) fields: LaunchFields,
    pub(super) skills: Vec<SkillFile>,
    pub(super) permissions: Permissions,
    pub(super) environment: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Permissions {
    pub(super) allow: Vec<String>,
    pub(super) deny: Vec<String>,
    pub(super) ask: Vec<String>,
    pub(super) hard_rules: Vec<Value>,
    pub(super) additional_directories: Vec<String>,
    pub(super) default_mode: Option<String>,
}

fn plain(text: &str) -> bool {
    !text.is_empty() && !text.chars().any(char::is_control)
}

fn variable(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(|c: char| c.is_ascii_digit())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn permissions(given: &Permissions) -> Result<(), String> {
    if !given.allow.is_empty()
        || !given.deny.is_empty()
        || !given.ask.is_empty()
        || !given.hard_rules.is_empty()
    {
        return Err(
            "permissions: native tool rules and hard policy rules are unsupported".to_owned(),
        );
    }
    if given
        .default_mode
        .as_deref()
        .is_some_and(|mode| !matches!(mode, "read-only" | "workspace-write" | "danger-full-access"))
    {
        return Err("permissions: unsupported sandbox mode".to_owned());
    }
    if given
        .additional_directories
        .iter()
        .any(|path| !path.starts_with('/') || !plain(path))
    {
        return Err("permissions: additional directories must be absolute plain paths".to_owned());
    }
    Ok(())
}

fn checked(template: &Template) -> Result<(), String> {
    if template.harness != "codex"
        || template.fields.harness.description.rendering_contract != "codex/template-v1"
    {
        return Err("description.rendering_contract: mismatched native template".to_owned());
    }
    let fields = &template.fields;
    if fields.models.len() != 1 || fields.models.iter().any(|model| !plain(model)) {
        return Err("models: the native launch requires exactly one plain model".to_owned());
    }
    if fields.instructions.contains('\0') {
        return Err("instructions: text contains a null byte".to_owned());
    }
    permissions(&template.permissions)?;
    let mut names = BTreeSet::new();
    for server in &fields.mcp_servers {
        if !plain(&server.name) || !names.insert(&server.name) {
            return Err("mcp_servers: names must be plain and distinct".to_owned());
        }
        if server.channel != Channel::Off {
            return Err("mcp_servers: wake channels are unsupported".to_owned());
        }
        match &server.transport {
            Transport::Http { url }
                if !plain(url) || !(url.starts_with("http://") || url.starts_with("https://")) =>
            {
                return Err("mcp_servers: HTTP addresses must use HTTP or HTTPS".to_owned());
            }
            Transport::Stdio {
                program,
                args,
                cwd,
                env,
                handles,
            } => {
                if !plain(program)
                    || args.iter().any(|arg| arg.contains('\0'))
                    || cwd
                        .as_ref()
                        .is_some_and(|path| !path.starts_with('/') || !plain(path))
                {
                    return Err(
                        "mcp_servers: invalid command, argument or working directory".to_owned(),
                    );
                }
                let mut vars = BTreeSet::new();
                for name in env
                    .iter()
                    .map(|one| &one.name)
                    .chain(handles.iter().map(|one| &one.name))
                {
                    if !variable(name) || !vars.insert(name) {
                        return Err(
                            "mcp_servers: environment names must be valid and distinct".to_owned()
                        );
                    }
                }
                if env.iter().any(|one| one.text.contains('\0'))
                    || handles.iter().any(|one| !plain(&one.handle_id))
                {
                    return Err("mcp_servers: invalid environment value or handle".to_owned());
                }
            }
            Transport::Http { .. } => {}
        }
    }
    if template
        .environment
        .iter()
        .any(|(name, value)| !variable(name) || name == "CODEX_HOME" || value.contains('\0'))
    {
        return Err("secrets: invalid or reserved environment variable".to_owned());
    }
    let mut skill_names = BTreeSet::new();
    let mut kept = Vec::new();
    for skill in &template.skills {
        kept.push(check(skill).map_err(|error| error.to_string())?);
        if !skill_names.insert(&skill.name) {
            return Err("skills: repeated skill name".to_owned());
        }
    }
    if fields.skills != kept {
        return Err("skills: files differ from the recorded skill declarations".to_owned());
    }
    Ok(())
}

pub(super) fn parse(text: &str) -> Result<Template, String> {
    let template: Template = serde_json::from_str(text)
        .map_err(|error| format!("template: invalid native template shape: {error}"))?;
    checked(&template)?;
    Ok(template)
}

pub(crate) fn render(
    fields: &LaunchFields,
    skills: &[SkillFile],
    granted: &Value,
    secrets: &[SecretBinding],
) -> Result<RenderedTemplate, RenderRefusal> {
    let permissions: Permissions = serde_json::from_value(granted.clone()).map_err(|error| {
        refused(
            fields,
            "permissions",
            &format!("invalid native permissions shape: {error}"),
        )
    })?;
    let mut environment = BTreeMap::from([
        ("LYS_AGENT".to_owned(), fields.identity.agent.clone()),
        ("LYS_SESSION".to_owned(), fields.identity.session.clone()),
        ("LYS_MACHINE".to_owned(), fields.identity.machine.clone()),
        (
            "LYS_PROVISIONING_VERSION".to_owned(),
            fields.identity.version.to_string(),
        ),
    ]);
    if let (Some(_), Some(run)) = (&fields.model_proxy, &fields.run) {
        // The run's key, from which the start tells the runner to count
        // the run's calls as the proxy sees them pass.
        if !crate::proxy::usage::is_run_key(run) {
            return Err(refused(
                fields,
                "run",
                "a run key is 32 lowercase hexadecimal digits, as a launch mints it",
            ));
        }
        environment.insert(
            crate::harness::rendering::RUN_VARIABLE.to_owned(),
            run.clone(),
        );
    }
    for held in secrets {
        if held.handle.is_empty()
            || environment
                .insert(held.env.clone(), held.handle.clone())
                .is_some()
        {
            return Err(refused(
                fields,
                "secrets",
                "empty handle or repeated environment variable",
            ));
        }
    }
    let template = Template {
        harness: "codex".to_owned(),
        fields: fields.clone(),
        skills: skills.to_vec(),
        permissions,
        environment,
    };
    checked(&template).map_err(|reason| {
        let member = reason
            .split_once(':')
            .map_or("template", |(member, _)| member);
        refused(fields, member, &reason)
    })?;
    let text = serde_json::to_string_pretty(&template).map_err(|error| {
        refused(
            fields,
            "template",
            &format!("cannot encode native template: {error}"),
        )
    })?;
    Ok(RenderedTemplate {
        harness: "codex".to_owned(),
        sha256: Hash::of(text.as_bytes()).as_str().to_owned(),
        text,
    })
}
