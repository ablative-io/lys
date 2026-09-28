//! The configuration and compose files a build runs with, rendered for the
//! new build from the install's recorded choices.
//!
//! The files are the compose definition and its database initialisation,
//! the compose environment, and the directory service's configuration. The
//! recorded choices are `deployment.toml`, the credentials under its state
//! directory and the administrator the service's configuration names; the
//! templates are the ones compiled into the `lys` that runs the upgrade,
//! which is why it refuses, by name, to render for a build other than its
//! own.
//!
//! Invariants: rendering only reads. A credential is read as it is stored
//! and never generated or rewritten; one that is missing is refused by
//! name. A rendered file's bytes are never printed: its `Debug` names it
//! and its length only.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use zeroize::Zeroizing;

use super::super::config::DeploymentConfig;
use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::install::layout::{self, Layout};
use super::super::install::server_config;
use super::super::prepare::{self, COMPOSE_ENV, SECRETS, read_secret};

/// The build of the `lys` running now, as its `--version` names it.
pub const OWN_BUILD: &str = env!("LYS_BUILD");

/// One file rendered for the new build.
pub struct RenderedFile {
    /// Its name in `config.previous/`.
    pub name: &'static str,
    /// Where it is placed.
    pub target: PathBuf,
    /// Its bytes; the compose environment's carry credentials.
    pub bytes: Zeroizing<Vec<u8>>,
    /// Whether it is written owner-only.
    pub private: bool,
    /// Whether it is part of the compose definition.
    pub compose: bool,
}

impl fmt::Debug for RenderedFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderedFile")
            .field("name", &self.name)
            .field("target", &self.target)
            .field("bytes", &self.bytes.len())
            .field("private", &self.private)
            .field("compose", &self.compose)
            .finish()
    }
}

impl RenderedFile {
    /// Whether the file in place already holds exactly these bytes.
    pub fn in_place(&self) -> bool {
        std::fs::read(&self.target)
            .map(Zeroizing::new)
            .is_ok_and(|placed| placed.as_slice() == self.bytes.as_slice())
    }
}

/// Renders the files a build runs with.
pub trait Render {
    /// The files for `build` (each binary's commit) under `layout`, with the
    /// screens served when `screens` is true.
    fn render(
        &self,
        layout: &Layout,
        build: &BTreeMap<String, String>,
        screens: bool,
    ) -> IdentityResult<Vec<RenderedFile>>;
}

/// The templates compiled into this `lys`.
#[derive(Debug)]
pub struct Templates;

fn rendering_failed(resource: &str, error: &impl ToString) -> IdentityError {
    IdentityError::new(
        ErrorKind::RenderFailed,
        "render",
        resource,
        error.to_string(),
    )
}

impl Render for Templates {
    fn render(
        &self,
        layout: &Layout,
        build: &BTreeMap<String, String>,
        screens: bool,
    ) -> IdentityResult<Vec<RenderedFile>> {
        if let Some((name, commit)) = build.iter().find(|(_, commit)| *commit != OWN_BUILD) {
            return Err(IdentityError::new(
                ErrorKind::UpgradeBuildDiffers,
                "render for the new build",
                name,
                format!(
                    "this lys is {OWN_BUILD} and the new {name} is {commit}; the configuration is \
                     rendered from the templates of the lys that runs the upgrade, so run the lys \
                     built with them: `lys identity upgrade --from` from beside the new binaries"
                ),
            ));
        }
        let config = DeploymentConfig::load(&layout.deployment_config())?;
        let state = config.state_dir();
        let mut credentials = Vec::with_capacity(SECRETS.len());
        for spec in SECRETS {
            credentials.push((spec, read_secret(&state, spec)?));
        }
        let environment = prepare::render_env(&config, &credentials)?;
        let administrator = server_config::recorded_administrator(layout)?;
        let service = server_config::render(layout, &config, &administrator, screens);
        let service = serde_json::to_vec_pretty(&service)
            .map_err(|error| rendering_failed("identity.json", &error))?;
        let file = |name, target, bytes: &[u8], private, compose| RenderedFile {
            name,
            target,
            bytes: Zeroizing::new(bytes.to_vec()),
            private,
            compose,
        };
        let deploy = layout.deploy_dir();
        Ok(vec![
            file(
                "compose.yaml",
                deploy.join("compose.yaml"),
                layout::COMPOSE_YAML.as_bytes(),
                false,
                true,
            ),
            file(
                "postgres-init.sql",
                deploy.join("postgres-init.sql"),
                layout::POSTGRES_INIT_SQL.as_bytes(),
                false,
                false,
            ),
            file(
                COMPOSE_ENV,
                state.join(COMPOSE_ENV),
                environment.as_bytes(),
                true,
                true,
            ),
            file(
                "identity.json",
                layout.service_config(),
                &service,
                true,
                false,
            ),
        ])
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
