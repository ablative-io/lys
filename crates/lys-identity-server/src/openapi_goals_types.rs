//! Schemas for the goals, expectations and deliverables on agents and teams.

use lys_openapi::Api;

use crate::goals_api::{GoalsView, MarkBody, SetBody as GoalBody};
use crate::goals_edit::{ActiveBody, WordsBody};
use crate::goals_views::ItemView as GoalItem;
use crate::openapi_table::{GET, POST};
use crate::openapi_types::Entry;

/// The goals, expectations and deliverables on agents and teams.
pub(crate) fn goals(api: &mut Api) -> Vec<Entry> {
    let (list, item) = (api.schema::<GoalsView>(), api.schema::<GoalItem>());
    let (set, mark) = (api.schema::<GoalBody>(), api.schema::<MarkBody>());
    let (active, words) = (api.schema::<ActiveBody>(), api.schema::<WordsBody>());
    vec![
        (GET, "/agents/{id}/goals", None, Some(list.clone())),
        (
            POST,
            "/agents/{id}/goals",
            Some(set.clone()),
            Some(item.clone()),
        ),
        (GET, "/teams/{id}/goals", None, Some(list)),
        (POST, "/teams/{id}/goals", Some(set), Some(item.clone())),
        (POST, "/goals/{goal}/mark", Some(mark), Some(item.clone())),
        (
            POST,
            "/goals/{goal}/active",
            Some(active),
            Some(item.clone()),
        ),
        (POST, "/goals/{goal}/words", Some(words), Some(item)),
    ]
}
