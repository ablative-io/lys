//! An agent's kept provisioning profile rendered as the home's launch
//! template, and the start command that names it.
//!
//! The template is the one the home keeps by hash and renders a harness
//! launch from; it is checked by the home's own parser, so a machine's home
//! takes these bytes as they are. Secrets ride only as use-only handles,
//! named by handle id, never a value. The start command is text: the agent's
//! identity, the session it reports under, the machine, the template's hash
//! and the handle ids, then the machine's runtime. Nothing here runs it.

use std::collections::BTreeSet;

use lys_home::harness::rendering::{SecretBinding, render as render_contract, shell_word};
use lys_home::harness::skills::SkillFile;
use lys_runner::judge::Policy;
use serde::Serialize;

use crate::error::ServerError;
use crate::launch_harness::fields;
use crate::launch_permissions::settings as permissions;
use crate::provisioning_store::Version;

/// One handle the agent holds, as the start command names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct HandleName {
    /// The handle's id, which is not the handle.
    pub id: String,
    /// The secret it stands for.
    pub secret: String,
    /// The variable the launch sets to the handle id.
    pub env: String,
}

/// What a start is rendered for.
pub struct Start<'a> {
    /// The agent.
    pub agent: &'a str,
    /// The session the started agent reports under.
    pub session: &'a str,
    /// The machine it starts on.
    pub machine: &'a str,
    /// The runtime installed on that machine.
    pub runtime: &'a str,
    /// The profile version it starts from.
    pub version: &'a Version,
    /// The text of each skill that version pinned.
    pub skills: &'a [SkillFile],
    /// The agent's latest Tool policy, when it has one.
    pub policy: Option<&'a Policy>,
}

/// A rendered start.
pub struct Rendered {
    /// The template's bytes, exactly as hashed.
    pub template: String,
    /// The SHA-256 of those bytes, as the home keeps the template by.
    pub template_sha256: String,
    /// The start command, as text.
    pub command: String,
}

/// The variable a handle on `secret` is set in, unique among `taken`.
pub fn handle_variable(secret: &str, taken: &mut BTreeSet<String>) -> String {
    let stem: String = secret
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    let base = format!("LYS_HANDLE_{stem}");
    let mut name = base.clone();
    let mut next = 2_u32;
    while taken.contains(&name) {
        name = format!("{base}_{next}");
        next = next.saturating_add(1);
    }
    taken.insert(name.clone());
    name
}

/// Render `start` with the agent's `handles`: the template the home checks
/// and keeps by hash, and the command a machine's runtime is given.
pub fn render(start: &Start<'_>, handles: &[HandleName]) -> Result<Rendered, ServerError> {
    let unrenderable = |reason: String| ServerError::LaunchUnrenderable { reason };
    let fields = fields(start, handles)?;
    let settings = &start.version.settings;
    let granted = permissions(
        settings.permissions.as_ref(),
        &settings.tools,
        start.policy,
        &fields.harness.description.permissions,
    )?;
    let held: Vec<SecretBinding> = handles
        .iter()
        .map(|handle| SecretBinding {
            env: handle.env.clone(),
            handle: handle.id.clone(),
        })
        .collect();
    let rendered = render_contract(&fields, start.skills, &granted, &held)
        .map_err(|error| unrenderable(error.to_string()))?;
    let template = rendered.text;
    let template_sha256 = rendered.sha256;
    let handle_ids: Vec<&str> = handles.iter().map(|handle| handle.id.as_str()).collect();
    let words = [
        "env".to_owned(),
        format!("LYS_AGENT={}", start.agent),
        format!("LYS_SESSION={}", start.session),
        format!("LYS_MACHINE={}", start.machine),
        format!("LYS_LAUNCH_TEMPLATE={template_sha256}"),
        format!("LYS_HANDLES={}", handle_ids.join(",")),
        start.runtime.to_owned(),
    ];
    let command = words
        .iter()
        .map(|word| shell_word(word))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Rendered {
        template,
        template_sha256,
        command,
    })
}
