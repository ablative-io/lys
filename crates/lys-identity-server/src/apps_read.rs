//! What a caller is shown of the apps it may see: the administrator, an app
//! and a registrar read the registry's view, and a person reads only an
//! approved app's id, name and standing.

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
    },
    Registry(Box<AppView>),
}

#[derive(Serialize)]
pub(crate) struct AppsRead {
    pub(crate) apps: Vec<AppRead>,
}

pub(crate) fn read_view(who: &Acting, app: &crate::apps_state::App) -> AppRead {
    if matches!(who, Acting::Person(_)) {
        AppRead::Personal {
            id: app.registered.app.clone(),
            name: app.registered.name.clone(),
            state: app.standing(),
        }
    } else {
        AppRead::Registry(Box::new(AppView::from(app)))
    }
}
