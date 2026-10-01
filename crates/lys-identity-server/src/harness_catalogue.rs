//! Named programme choices, read once before serving and joined to reviewed builds.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::sync::{Arc, Mutex};

use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::{Json, Router, routing::get};
use lys_home::harness::description::{Description, FurtherModels};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::routes::{AppState, Shared, signed_in};

#[path = "harness_installed.rs"]
mod installed;

#[cfg(test)]
#[path = "harness_catalogue_tests.rs"]
mod tests;

type InstalledCopies = BTreeMap<String, (installed::CopyKey, Result<BuildView, String>)>;

/// A model choice, with the programme's default first.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelChoice {
    /// The value the programme accepts.
    pub id: String,
    /// The name a person chooses.
    pub label: String,
}

/// A permission mode and what it allows.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModeChoice {
    /// The value the programme accepts.
    pub id: String,
    /// What choosing this mode allows.
    pub meaning: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgramFile {
    name: String,
    command: String,
    #[serde(default)]
    commands: Option<Vec<String>>,
    line: String,
    models: Vec<ModelChoice>,
    modes: Vec<ModeChoice>,
    description: Description,
    sources: Vec<String>,
}

/// Where a declared build was read from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BuildSource {
    /// An executable found on the service's search path.
    Installed,
    /// A reviewed profile version.
    Profile,
}

/// One build declared in a reviewed profile.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, utoipa::ToSchema)]
pub struct BuildView {
    /// The name given to this build.
    pub name: String,
    /// Its absolute programme path.
    pub program: String,
    /// Its package or build identity.
    pub package: String,
    /// Where this declaration was read from.
    #[serde(rename = "from")]
    pub source: BuildSource,
}

/// One described programme, independent of whether a build has been declared.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProgramView {
    /// Its plain name.
    pub name: String,
    /// The standard command, resolved by the machine that runs it.
    pub command: String,
    /// Executable names to discover, with the standard command first.
    pub commands: Vec<String>,
    /// One line saying what it is.
    pub line: String,
    /// Named models, with the default first.
    pub models: Vec<ModelChoice>,
    /// Named permission modes and their meanings.
    pub modes: Vec<ModeChoice>,
    /// Native instruction choices supported by the registered renderer.
    pub instructions_modes: Vec<String>,
    /// The declared-harness capability and rendering contract.
    #[schema(value_type = Object)]
    pub description: Description,
    /// All distinct builds declared in reviewed profile versions.
    pub builds: Vec<BuildView>,
    /// Why no installed executable could be offered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_found: Option<String>,
}

/// Every programme the service describes.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CatalogueView {
    /// Programme choices in catalogue order.
    pub programs: Vec<ProgramView>,
}

/// Validated descriptions kept for the service's lifetime.
#[derive(Debug)]
pub struct Catalogue {
    programs: Vec<ProgramView>,
    installed: Mutex<InstalledCopies>,
}

impl Catalogue {
    /// Read the descriptions embedded in the service binary before serving.
    pub fn embedded() -> Result<Self, ServerError> {
        Self::read(&[
            (
                "docs/harness/catalogue/claude-code.json",
                include_str!("../../../docs/harness/catalogue/claude-code.json"),
            ),
            (
                "docs/harness/catalogue/codex.json",
                include_str!("../../../docs/harness/catalogue/codex.json"),
            ),
        ])
    }

    /// Parse named source files through the same boundary used at startup.
    pub fn read(sources: &[(&str, &str)]) -> Result<Self, ServerError> {
        let mut programs: Vec<ProgramView> = Vec::new();
        for &(file, text) in sources {
            let mut parsed: ProgramFile =
                serde_json::from_str(text).map_err(|error| unreadable(file, error.to_string()))?;
            let commands = match parsed.commands.take() {
                Some(commands) => commands,
                None => vec![parsed.command.clone()],
            };
            parsed.commands = Some(commands.clone());
            validate(&parsed).map_err(|reason| unreadable(file, reason))?;
            if programs.iter().any(|kept| {
                kept.name == parsed.name
                    || kept.description.rendering_contract == parsed.description.rendering_contract
            }) {
                return Err(unreadable(
                    file,
                    "programme name or rendering contract is repeated",
                ));
            }
            programs.push(ProgramView {
                name: parsed.name,
                command: parsed.command,
                commands,
                line: parsed.line,
                models: parsed.models,
                modes: parsed.modes,
                instructions_modes: vec![
                    "keep".to_owned(),
                    "append".to_owned(),
                    "replace".to_owned(),
                ],
                description: parsed.description,
                builds: Vec::new(),
                not_found: None,
            });
        }
        if programs.is_empty() {
            return Err(unreadable(
                "catalogue",
                "no description files were supplied",
            ));
        }
        programs.retain(|program| {
            lys_home::harness::rendering::registered(&program.description.rendering_contract)
        });
        Ok(Self {
            programs,
            installed: Mutex::new(BTreeMap::new()),
        })
    }

    /// Discover installed copies on an explicit search path without changing process state.
    ///
    /// # Errors
    /// Refuses a poisoned discovery cache; individual copy failures are served as reasons.
    pub fn installed_in(&self, path: &OsStr) -> Result<CatalogueView, ServerError> {
        let mut cache = self
            .installed
            .lock()
            .map_err(|error| unreadable("installed_copies", error.to_string()))?;
        let mut programs = self.programs.clone();
        for program in &mut programs {
            let mut reasons = Vec::new();
            for command in &program.commands {
                let executable = match installed::resolve(command, path) {
                    Ok(executable) => executable,
                    Err(reason) => {
                        cache.remove(command);
                        reasons.push(reason);
                        continue;
                    }
                };
                let found = cache.entry(command.clone()).or_insert_with(|| {
                    (
                        executable.key.clone(),
                        installed::version(command, &executable, path),
                    )
                });
                if found.0 != executable.key {
                    *found = (
                        executable.key.clone(),
                        installed::version(command, &executable, path),
                    );
                }
                match &found.1 {
                    Ok(copy) => program.builds.push(copy.clone()),
                    Err(reason) => reasons.push(reason.clone()),
                }
            }
            if program.builds.is_empty() {
                program.not_found = Some(reasons.join("; "));
            }
        }
        Ok(CatalogueView { programs })
    }

    fn view(&self, state: &AppState) -> Result<CatalogueView, ServerError> {
        let path = match std::env::var_os("PATH") {
            Some(path) => path,
            None => OsString::new(),
        };
        let mut programs = self.installed_in(&path)?.programs;
        if state.provisioning.is_some() {
            crate::provisioning_api::with_provisioning(state, |store| {
                profile_builds(&mut programs, store);
                Ok(())
            })?;
        }
        Ok(CatalogueView { programs })
    }
}

fn profile_builds(
    programs: &mut [ProgramView],
    store: &crate::provisioning_store::ProvisioningStore,
) {
    for program in programs {
        program.builds.extend(
            store
                .reviewed_builds(&program.description.rendering_contract)
                .cloned(),
        );
    }
}

fn unreadable(file: &str, reason: impl Into<String>) -> ServerError {
    ServerError::HarnessCatalogueUnreadable {
        file: file.to_owned(),
        reason: reason.into(),
    }
}

fn line(text: &str) -> bool {
    !text.is_empty() && text.trim() == text && !text.chars().any(char::is_control)
}

fn distinct_lines<'a>(values: impl IntoIterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .all(|value| line(value) && seen.insert(value))
}

fn validate(program: &ProgramFile) -> Result<(), &'static str> {
    if !line(&program.name) || !line(&program.line) {
        return Err("name and line must each be nonempty plain text on one line");
    }
    let expected_command = match program.description.rendering_contract.as_str() {
        "claude-code/template-v1" => Some("claude"),
        "codex/template-v1" => Some("codex"),
        _ => None,
    };
    let commands = program.commands.as_deref().ok_or("commands are absent")?;
    if commands.first() != Some(&program.command)
        || expected_command.is_some_and(|command| command != program.command)
        || !distinct_lines(commands.iter().map(String::as_str))
        || commands
            .iter()
            .any(|command| !installed::command_name(command))
    {
        return Err("commands must be distinct executable names with the standard command first");
    }
    if program.models.is_empty()
        || !distinct_lines(program.models.iter().map(|model| model.id.as_str()))
        || program.models.iter().any(|model| !line(&model.label))
    {
        return Err("models must have distinct nonempty ids and plain labels, default first");
    }
    if program.modes.is_empty()
        || !distinct_lines(program.modes.iter().map(|mode| mode.id.as_str()))
        || program.modes.iter().any(|mode| !line(&mode.meaning))
        || !program.modes.iter().map(|mode| &mode.id).eq(program
            .description
            .permissions
            .modes
            .iter())
    {
        return Err("modes must have distinct ids and plain meanings matching the description");
    }
    let description = &program.description;
    if !line(&description.rendering_contract)
        || description.models.minimum == 0
        || description
            .models
            .maximum
            .is_some_and(|maximum| maximum < description.models.minimum)
    {
        return Err("rendering contract and model cardinality must be valid");
    }
    if let FurtherModels::Delimited { separator } = &description.models.further_encoding {
        if !line(separator)
            || program
                .models
                .iter()
                .any(|model| model.id.contains(separator))
        {
            return Err("model delimiter must be nonempty and absent from model ids");
        }
    }
    if !distinct_lines(
        description
            .permissions
            .rule_forms
            .iter()
            .map(String::as_str),
    ) || !distinct_lines(description.mcp.transports.iter().map(String::as_str))
        || description
            .mcp
            .channel_policies
            .iter()
            .enumerate()
            .any(|(index, channel)| description.mcp.channel_policies[..index].contains(channel))
    {
        return Err("rule forms, MCP transports and channel policies must be distinct");
    }
    if program.sources.is_empty()
        || !distinct_lines(program.sources.iter().map(String::as_str))
        || program.sources.iter().any(|source| {
            reqwest::Url::parse(source).map_or(true, |url| {
                url.scheme() != "https" || url.host_str().is_none()
            })
        })
    {
        return Err("sources must name distinct published HTTPS addresses");
    }
    Ok(())
}

pub(crate) fn routes() -> Router<Shared> {
    Router::new().route("/harnesses", get(read))
}

async fn read(
    State(state): State<Shared>,
    Extension(catalogue): Extension<Arc<Catalogue>>,
    headers: HeaderMap,
) -> Result<Json<CatalogueView>, ServerError> {
    signed_in(&state, &headers)?;
    let answer = tokio::task::spawn_blocking(move || catalogue.view(&state))
        .await
        .map_err(|error| {
            unreadable(
                "installed_copies",
                format!("discovery task did not complete: {error}"),
            )
        })??;
    Ok(Json(answer))
}
