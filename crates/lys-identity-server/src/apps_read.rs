//! What a caller is shown of the apps it may see: the administrator, an app
//! and a registrar read the registry's view, and a person reads only an
//! approved app's id, name, standing and address: the origin of its first
//! sign-in redirect, where Open <app> takes them.

use serde::Serialize;

use crate::apps_binding::Acting;
use crate::apps_state::{By, Standing};
use crate::apps_views::AppView;

/// Whether `who` may see `app`: the administrator sees every app, an app
/// itself, a registrar those it registered, and a person only those whose
/// approved client has a sign-in destination.
pub(crate) fn sees(who: &Acting, app: &crate::apps_state::App) -> bool {
    match who {
        Acting::Administrator(_) => true,
        Acting::App { app: own, .. } => *own == app.registered.app,
        Acting::Registrar { service_account } => {
            app.registered.by
                == By::ServiceAccount {
                    id: service_account.clone(),
                }
        }
        Acting::Person(_) => {
            app.standing() == Standing::Approved
                && app
                    .approved
                    .as_ref()
                    .is_some_and(|approved| approved.client.client_id == app.registered.app)
                && app
                    .sign_in
                    .as_ref()
                    .is_some_and(|settings| !settings.redirects.is_empty())
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum AppRead {
    Personal {
        id: String,
        name: String,
        state: Standing,
        /// The origin of the app's first sign-in redirect; absent when that
        /// address has no web origin.
        #[serde(skip_serializing_if = "Option::is_none")]
        address: Option<String>,
    },
    Registry(Box<AppView>),
}

#[derive(Serialize)]
pub(crate) struct AppsRead {
    pub(crate) apps: Vec<AppRead>,
    /// Lys's own address for an app on another computer to call back.
    pub(crate) public_address: Option<String>,
    /// Why Lys has no public address, when it has none.
    pub(crate) public_address_reason: Option<String>,
}

pub(crate) fn read_view(who: &Acting, app: &crate::apps_state::App) -> AppRead {
    if matches!(who, Acting::Person(_)) {
        AppRead::Personal {
            id: app.registered.app.clone(),
            name: app.registered.name.clone(),
            state: app.standing(),
            address: address(app),
        }
    } else {
        AppRead::Registry(Box::new(AppView::from(app)))
    }
}

/// The origin of `app`'s first sign-in redirect, when it is an http or https
/// address.
pub(crate) fn address(app: &crate::apps_state::App) -> Option<String> {
    let first = app.sign_in.as_ref()?.redirects.first()?;
    let parsed = reqwest::Url::parse(first).ok()?;
    matches!(parsed.scheme(), "http" | "https").then(|| parsed.origin().ascii_serialization())
}

#[cfg(test)]
#[path = "apps_read_tests.rs"]
mod tests;
