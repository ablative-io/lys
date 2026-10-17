//! The configuration and compose files a build runs with, rendered for the
//! new build from the install's recorded choices.
//!
//! The files are the compose definition and its database initialisation,
//! the compose environment, the directory service's configuration, and the
//! shipped permission model when the one held is older. The
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
use super::super::install::{self, server_config};
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
    ///
    /// # Errors
    /// Returns named configuration or rendering failures for the requested install files.
    fn render(
        &self,
        layout: &Layout,
        build: &BTreeMap<String, String>,
        screens: bool,
    ) -> IdentityResult<Vec<RenderedFile>>;
}

/// The templates compiled into this `lys`.
#[derive(Debug, Default)]
pub struct Templates {
    /// The message service connection to write in place of the one the
    /// install carries, when the upgrade was given one.
    pub messages: Option<serde_json::Value>,
}

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
        let config = DeploymentConfig::load_install(&layout.deployment_config())?;
        let state = config.state_dir();
        let mut credentials = Vec::with_capacity(SECRETS.len());
        for spec in SECRETS {
            credentials.push((spec, read_secret(&state, spec)?));
        }
        let environment = prepare::render_env(&config, &credentials)?;
        let mut carried = server_config::carried(layout)?.ok_or_else(|| {
            IdentityError::new(
                ErrorKind::NotInstalled,
                "read recorded choices",
                "identity.json",
                "no directory service configuration; run `lys identity install`",
            )
            .at(&layout.service_config())
        })?;
        if let Some(messages) = &self.messages {
            carried.message_service = Some(messages.clone());
        }
        let mut service = server_config::render(layout, &config, &carried, screens);
        let previous = std::fs::read(layout.service_config())
            .map(Zeroizing::new)
            .map_err(|error| rendering_failed("identity.json", &error))?;
        let previous: serde_json::Value = serde_json::from_slice(&previous)
            .map_err(|error| rendering_failed("identity.json", &error))?;
        carry_existing_settings(&previous, &mut service);
        carry_sign_in(&previous, &mut service)?;
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
        let model = install::model_replacement(&layout.grant_model()).map(|shipped| {
            file(
                "model.json",
                layout.grant_model(),
                shipped.as_bytes(),
                false,
                false,
            )
        });
        let mut files = vec![
            file(
                "estate-approval.json",
                layout.data_dir().join("estate-approval.json"),
                layout::ESTATE_PLAN.as_bytes(),
                true,
                false,
            ),
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
        ];
        files.extend(model);
        Ok(files)
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;

fn carry_existing_settings(previous: &serde_json::Value, next: &mut serde_json::Value) {
    let (Some(previous), Some(next)) = (previous.as_object(), next.as_object_mut()) else {
        return;
    };
    for (key, value) in previous {
        // A renamed message bridge has already been carried under its current name.
        if key.ends_with("_messages")
            && next.contains_key("message_service")
            && value.as_object().is_some_and(|bridge| {
                bridge.len() == 2 && bridge.contains_key("url") && bridge.contains_key("bindings")
            })
        {
            continue;
        }
        match next.get_mut(key) {
            Some(rendered) => carry_existing_settings(value, rendered),
            None => {
                next.insert(key.clone(), value.clone());
            }
        }
    }
}

fn carry_sign_in(previous: &serde_json::Value, next: &mut serde_json::Value) -> IdentityResult<()> {
    for key in ["secure_cookie", "session_seconds"] {
        let Some(value) = previous.get(key) else {
            continue;
        };
        let valid = match key {
            "secure_cookie" => value.is_boolean(),
            _ => value.as_u64().is_some_and(|seconds| seconds > 0),
        };
        if !valid {
            return Err(IdentityError::new(
                ErrorKind::ConfigInvalid,
                "carry sign-in settings",
                "identity.json",
                format!("{key} is not a valid sign-in setting"),
            ));
        }
        // The former product default is not an operator's choice: the
        // rendered length stands (DIRECTORY-099 R4).
        if key == "session_seconds" && value.as_u64() == Some(server_config::FORMER_SESSION_SECONDS)
        {
            continue;
        }
        next[key] = value.clone();
    }
    Ok(())
}

/// Say, through `say`, the one line naming the change when `next` moves an
/// install whose `previous` identity.json was still at the former default.
pub(super) fn say_sign_in_moved(
    previous: &serde_json::Value,
    next: &serde_json::Value,
    say: &mut dyn FnMut(&str),
) {
    let seconds = |value: &serde_json::Value| {
        value
            .get("session_seconds")
            .and_then(serde_json::Value::as_u64)
    };
    let (Some(was), Some(now)) = (seconds(previous), seconds(next)) else {
        return;
    };
    if was == server_config::FORMER_SESSION_SECONDS && now != was {
        say(&format!(
            "identity.json: session_seconds {was} -> {now} (a sign-in now lasts thirty days; the \
             former eight-hour default is not carried)"
        ));
    }
}

#[cfg(test)]
mod sign_in_tests {
    #[test]
    fn an_upgrade_keeps_existing_keys_that_its_template_does_not_name() {
        let previous = serde_json::json!({
            "trusted_proxies": ["127.0.0.1"],
            "operator_choice": {"limit": 17},
            "provider": {"rights_bytes": 8192},
        });
        let mut next = serde_json::json!({"provider": {"pass_seconds": 300}});
        super::carry_existing_settings(&previous, &mut next);
        assert_eq!(next["trusted_proxies"], previous["trusted_proxies"]);
        assert_eq!(next["operator_choice"], previous["operator_choice"]);
        assert_eq!(next["provider"]["rights_bytes"], 8192);
        assert_eq!(next["provider"]["pass_seconds"], 300);
        super::carry_existing_settings(&previous, &mut next);
        assert_eq!(next["trusted_proxies"], previous["trusted_proxies"]);
    }

    #[test]
    fn an_upgrade_keeps_secure_cookies_and_the_operators_session_duration()
    -> crate::identity::error::IdentityResult<()> {
        let mut next = serde_json::json!({"secure_cookie": false, "session_seconds": 28800});
        super::carry_sign_in(
            &serde_json::json!({"secure_cookie": true, "session_seconds": 43200}),
            &mut next,
        )?;
        assert_eq!(
            next,
            serde_json::json!({"secure_cookie": true, "session_seconds": 43200})
        );
        let mut said = Vec::new();
        super::say_sign_in_moved(
            &serde_json::json!({"session_seconds": 43200}),
            &next,
            &mut |line| said.push(line.to_owned()),
        );
        assert!(
            said.is_empty(),
            "an operator's own length is carried silently"
        );
        Ok(())
    }

    /// DIRECTORY-099 R4: an install still at the former eight-hour default
    /// is moved to thirty days, and the upgrade says so once.
    #[test]
    fn an_upgrade_moves_the_former_default_and_says_so_once()
    -> crate::identity::error::IdentityResult<()> {
        let previous = serde_json::json!({"secure_cookie": true, "session_seconds": 28800});
        let mut next = serde_json::json!({"secure_cookie": false, "session_seconds": 2_592_000});
        super::carry_sign_in(&previous, &mut next)?;
        assert_eq!(
            next,
            serde_json::json!({"secure_cookie": true, "session_seconds": 2_592_000})
        );
        let mut said = Vec::new();
        super::say_sign_in_moved(&previous, &next, &mut |line| said.push(line.to_owned()));
        assert_eq!(
            said,
            [
                "identity.json: session_seconds 28800 -> 2592000 (a sign-in now lasts thirty \
                 days; the former eight-hour default is not carried)"
            ]
        );
        Ok(())
    }

    /// DIRECTORY-099 R4: a session length that is not a positive number of
    /// seconds is still refused by name, never moved.
    #[test]
    fn an_upgrade_still_refuses_a_session_length_that_is_not_seconds() {
        for previous in [
            serde_json::json!({"session_seconds": 0}),
            serde_json::json!({"session_seconds": "28800"}),
        ] {
            let mut next = serde_json::json!({"session_seconds": 2_592_000});
            let refused = super::carry_sign_in(&previous, &mut next);
            assert!(
                refused.is_err_and(|error| {
                    error.kind() == crate::identity::error::ErrorKind::ConfigInvalid
                }),
                "{previous}"
            );
        }
    }
}
