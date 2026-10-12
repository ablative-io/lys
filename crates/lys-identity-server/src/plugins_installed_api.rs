//! The Claude Code plugins installed on the computer this service runs on, read
//! from the login's own config folder, so the agent page can say beside the
//! plugins field which named plugin a run here would be refused for
//! (`plugin_not_installed`). A run on another computer is checked by that
//! computer's runner before it starts.

use std::path::PathBuf;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use serde::Serialize;

use crate::error::ServerError;
use crate::routes::{Shared, signed_in};

/// The plugins installed in this computer's Claude config folder.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct InstalledPlugins {
    /// The folder read: the login's `~/.claude`, where a run's sign-in lives.
    pub folder: Option<String>,
    /// Each installed plugin as name@marketplace; empty when none, or unread.
    pub installed: Vec<String>,
    /// Why the folder could not be read, when it could not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unread: Option<String>,
}

/// What [`read`] answers for `folder`.
#[must_use]
pub fn installed_in(folder: Option<PathBuf>) -> InstalledPlugins {
    let Some(folder) = folder else {
        return InstalledPlugins {
            folder: None,
            installed: Vec::new(),
            unread: Some("this service has no HOME, so no Claude config folder".to_owned()),
        };
    };
    let shown = folder.display().to_string();
    match lys_home::harness::claude_code::installed_plugins::installed(&folder) {
        Ok(installed) => InstalledPlugins {
            folder: Some(shown),
            installed: installed.into_iter().collect(),
            unread: None,
        },
        Err(error) => InstalledPlugins {
            folder: Some(shown),
            installed: Vec::new(),
            unread: Some(error.to_string()),
        },
    }
}

/// `GET /harnesses/plugins`.
///
/// # Errors
/// Refuses a caller who is not signed in.
pub(crate) async fn read(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<InstalledPlugins>, ServerError> {
    signed_in(&state, &headers)?;
    let folder = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude"));
    Ok(Json(installed_in(folder)))
}

#[cfg(test)]
mod tests {
    use super::installed_in;

    #[test]
    fn the_folder_s_installed_plugins_are_listed_and_a_missing_home_is_named() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir
            .path()
            .join(lys_home::harness::claude_code::installed_plugins::INSTALLED_FILE);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, r#"{"version":2,"plugins":{"argus@ablative":[{}]}}"#).unwrap();
        let answer = installed_in(Some(dir.path().to_path_buf()));
        assert_eq!(answer.installed, vec!["argus@ablative".to_owned()]);
        assert!(answer.unread.is_none());
        let none = installed_in(None);
        assert!(none.installed.is_empty());
        assert!(none.unread.is_some());
    }
}
