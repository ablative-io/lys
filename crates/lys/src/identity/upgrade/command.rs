//! Upgrade authority checks and per-session outcome reporting.

use super::super::error::{ErrorKind, IdentityResult};
use super::super::install;
use super::super::install::layout::Layout;
use super::{
    Compose, Parts, Render, refuse, render, require_install, units, upgrade, verified_config,
    version,
};
use crate::commands::output::Emitter;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// What the operator chose.
#[derive(Debug)]
pub struct Options {
    /// Built client pairs grouped by Rust target.
    pub clients: Option<PathBuf>,
    /// The folder holding the newly built binaries.
    pub from: PathBuf,
    /// A compiled screens package to verify and place.
    pub surface: Option<PathBuf>,
    /// The data root; the platform's application data path when absent.
    pub root: Option<PathBuf>,
    /// A JSON file holding the message service connection to write in
    /// place of the one the install carries.
    pub message_service: Option<PathBuf>,
}

/// Runs `lys identity upgrade` after checking the live issuer authority.
///
/// # Errors
/// Returns named upgrade configuration, preparation or execution failures.
pub fn run(options: &Options, json: bool) -> IdentityResult<()> {
    let client_program = options.from.join("lys");
    let clients = install::layout::prepare_clients(
        options.clients.as_deref(),
        &options.from,
        &version(&client_program, "lys")?,
    )?;
    let layout = match &options.root {
        Some(root) => Layout::at(root.clone()),
        None => Layout::discover()?,
    };
    let mut emitter = Emitter::new(json);
    emitter.field("root", "root", layout.root.display().to_string());
    let units = units(&layout)?;
    require_install(&layout)?;
    let config = verified_config(&layout)?;
    install::server_keys(&layout, &config)?;
    let templates = render::Templates {
        messages: options
            .message_service
            .as_deref()
            .map(install::server_config::messages_from)
            .transpose()?,
    };
    if templates.messages.is_some() {
        emitter.note("message service connection from the given file");
    }
    let runner = options.from.join("lys");
    let mut parts = Parts {
        runner: Some(&runner),
        units: &units,
        engine: &mut Compose,
        render: &ClientRender {
            base: &templates,
            clients: &clients,
        },
        clients: Some(&clients),
    };
    let mut session_outcomes = Vec::new();
    let record = upgrade(
        &layout,
        &options.from,
        options.surface.as_deref(),
        &mut parts,
        &mut |line| {
            emitter.note(line);
            if line.contains(": resumed") || line.contains(": failed:") {
                session_outcomes.push(line.to_owned());
            }
        },
    )?;
    if json {
        emitter.field(
            "session resume",
            "session_resume",
            serde_json::json!(&session_outcomes),
        );
    }
    let value = serde_json::to_value(&record).map_err(|error| {
        refuse(
            ErrorKind::RenderFailed,
            "render",
            "build.json",
            error.to_string(),
        )
    })?;
    if json {
        emitter.field("build", "build", value);
    }
    report_service(&layout, &mut emitter)?;
    report_unreferenced(&layout, &mut emitter)?;
    require_resumed(&layout, &session_outcomes)?;
    let address =
        install::ports::PersonAddress::load(&layout, install::ports::Ports::load(&layout)?)?;
    address.say_fallback(&mut emitter);
    let address = address.origin;
    if json {
        emitter.field("address", "url", address);
    } else {
        super::super::stop::write_address(&mut std::io::stdout(), &address).map_err(|error| {
            refuse(
                ErrorKind::PrivateFileIo,
                "print",
                "address",
                error.to_string(),
            )
        })?;
    }
    emitter.finish();
    Ok(())
}

pub(super) struct ClientRender<'a> {
    pub(super) base: &'a dyn Render,
    pub(super) clients: &'a install::layout::ClientSet,
}

impl Render for ClientRender<'_> {
    fn render(
        &self,
        layout: &Layout,
        build: &BTreeMap<String, String>,
        screens: bool,
    ) -> IdentityResult<Vec<render::RenderedFile>> {
        let mut files = self.base.render(layout, build, screens)?;
        files.push(self.clients.manifest(layout)?);
        Ok(files)
    }
}

fn report_service(layout: &Layout, emitter: &mut Emitter) -> IdentityResult<()> {
    let placed = install::server_config::carried(layout)?;
    let membership = placed
        .as_ref()
        .and_then(|carried| carried.membership.as_ref());
    emitter.note(&install::server_config::membership_readback(membership));
    let rights_bytes = placed
        .as_ref()
        .and_then(|carried| carried.rights_bytes.as_ref());
    emitter.note(&install::server_config::provider_readback(
        rights_bytes,
        &layout.logs_dir().join("identity.log"),
    )?);
    Ok(())
}

/// Names every release directory the manifest does not, one line each
/// through `say`, and answers them: the pile is shown, never pruned
/// (DIRECTORY-115 R3).
pub(super) fn say_unreferenced(
    layout: &Layout,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<Vec<PathBuf>> {
    let unreferenced = install::layout::unreferenced_releases(layout)?;
    for directory in &unreferenced {
        say(&format!(
            "client release unreferenced: {}: clients.json does not name it; nothing deletes it",
            directory.display()
        ));
    }
    Ok(unreferenced)
}

/// [`say_unreferenced`] into the upgrade's notes and, in JSON, the paths
/// under `clients_unreferenced` (an empty list when every release is named).
pub(super) fn report_unreferenced(
    layout: &Layout,
    emitter: &mut Emitter,
) -> IdentityResult<Vec<PathBuf>> {
    let unreferenced = say_unreferenced(layout, &mut |line| emitter.note(line))?;
    if emitter.is_json() {
        let paths: Vec<String> = unreferenced
            .iter()
            .map(|directory| directory.display().to_string())
            .collect();
        emitter.field(
            "clients unreferenced",
            "clients_unreferenced",
            serde_json::json!(paths),
        );
    }
    Ok(unreferenced)
}

fn require_resumed(layout: &Layout, session_outcomes: &[String]) -> IdentityResult<()> {
    if session_outcomes
        .iter()
        .any(|line| line.contains(": failed:"))
    {
        return Err(refuse(
            ErrorKind::UpgradeFailed,
            "resume sessions",
            "installed build",
            format!(
                "the build is installed; {}; retained launches remain in {}",
                session_outcomes.join("; "),
                layout.run_dir().join("upgrade-sessions.json").display()
            ),
        ));
    }
    Ok(())
}
