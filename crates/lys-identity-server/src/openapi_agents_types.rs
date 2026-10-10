//! Schemas for the agents cluster's records (AGENTS-001): the words, the
//! variables and the schedules.

use lys_openapi::Api;

use crate::openapi_table::{GET, POST};
use crate::openapi_types::Entry;
use crate::schedules_api::{ChangeBody, SetBody as ScheduleBody, StopBody, View as SchedulesView};
use crate::schedules_state::Item as ScheduleItem;
use crate::variables_api::PatchBody;
use crate::variables_state::Read as VariablesRead;
use crate::words_api::{Delivered, ForAgent, PreviewBody, SaveBody, Saved, TemplateBody};

/// The words, variables and schedules routes, each with the types it takes
/// and answers.
pub(crate) fn agents(api: &mut Api) -> Vec<Entry> {
    let (save, saved) = (api.schema::<SaveBody>(), api.schema::<Saved>());
    let (template, preview) = (api.schema::<TemplateBody>(), api.schema::<PreviewBody>());
    let (delivered, for_agent) = (api.schema::<Delivered>(), api.schema::<ForAgent>());
    let (patch, read) = (api.schema::<PatchBody>(), api.schema::<VariablesRead>());
    let (schedule, change) = (api.schema::<ScheduleBody>(), api.schema::<ChangeBody>());
    let (stop, item) = (api.schema::<StopBody>(), api.schema::<ScheduleItem>());
    let schedules = api.schema::<SchedulesView>();
    vec![
        (POST, "/words/preview", Some(preview), Some(delivered)),
        (
            POST,
            "/words/templates/{name}",
            Some(template),
            Some(saved.clone()),
        ),
        (POST, "/words/{slot}", Some(save.clone()), Some(saved.clone())),
        (GET, "/agents/{id}/words", None, Some(for_agent)),
        (
            POST,
            "/agents/{id}/words/{slot}",
            Some(save.clone()),
            Some(saved.clone()),
        ),
        (
            POST,
            "/runtime/sessions/{id}/words/{slot}",
            Some(save),
            Some(saved),
        ),
        (GET, "/agents/{id}/variables", None, Some(read.clone())),
        (
            POST,
            "/agents/{id}/variables",
            Some(patch.clone()),
            Some(read.clone()),
        ),
        (
            GET,
            "/runtime/sessions/{id}/variables",
            None,
            Some(read.clone()),
        ),
        (
            POST,
            "/runtime/sessions/{id}/variables",
            Some(patch.clone()),
            Some(read.clone()),
        ),
        (GET, "/me/variables", None, Some(read.clone())),
        (POST, "/me/variables", Some(patch.clone()), Some(read.clone())),
        (GET, "/me/session/variables", None, Some(read.clone())),
        (POST, "/me/session/variables", Some(patch), Some(read)),
        (GET, "/schedules", None, Some(schedules)),
        (POST, "/schedules", Some(schedule), Some(item.clone())),
        (GET, "/schedules/{id}", None, Some(item.clone())),
        (POST, "/schedules/{id}/change", Some(change), Some(item.clone())),
        (POST, "/schedules/{id}/stop", Some(stop), Some(item)),
    ]
}
