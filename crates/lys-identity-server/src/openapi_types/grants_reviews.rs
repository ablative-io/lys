//! The grant and review request and answer schemas.
use lys_openapi::Api;

use super::Entry;
use crate::directory_views::{ReceiptPage, ServiceKeyView};
use crate::grant_contract::{
    ActionBody, CannotGiveAnswer, DelegateBody, GrantList, GrantView, ModelView, PermitView,
    RecordedView, RevokeBody, RootBody, WhoBody, WhoPage,
};
use crate::grants_reach::{ReachAnswer, ReachBody};
use crate::openapi_table::{GET, POST};
use crate::reviews_api::{KeepAgentBody, KeepBody, ReviewView};
use crate::reviews_state::Kept;

pub(super) fn grants_and_reviews(api: &mut Api) -> Vec<Entry> {
    let list = api.schema::<GrantList>();
    let (delegate, root) = (api.schema::<DelegateBody>(), api.schema::<RootBody>());
    let recorded = api.schema::<RecordedView>();
    let (action, permit) = (api.schema::<ActionBody>(), api.schema::<PermitView>());
    let (who, page) = (api.schema::<WhoBody>(), api.schema::<WhoPage>());
    let revoke = api.schema::<RevokeBody>();
    let keep = api.schema::<KeepBody>();
    vec![
        (
            GET,
            "/agent/grants",
            None,
            Some(api.schema::<crate::agent_grants_api::AgentGrants>()),
        ),
        (GET, "/grants", None, Some(list)),
        (POST, "/grants", Some(delegate), Some(recorded.clone())),
        (GET, "/grants/model", None, Some(api.schema::<ModelView>())),
        (POST, "/grants/roots", Some(root), Some(recorded.clone())),
        (
            POST,
            "/grants/agent-roots",
            None,
            Some(api.schema::<crate::agent_roots::AgentRoots>()),
        ),
        (
            POST,
            "/grants/check",
            Some(action.clone()),
            Some(permit.clone()),
        ),
        (POST, "/grants/why", Some(action), Some(permit)),
        (POST, "/grants/who", Some(who), Some(page)),
        (
            POST,
            "/grants/held",
            Some(api.schema::<crate::grant_contract::ActionBody>()),
            Some(api.schema::<crate::grants::held::HeldAnswer>()),
        ),
        (
            POST,
            "/grants/reach",
            Some(api.schema::<ReachBody>()),
            Some(api.schema::<ReachAnswer>()),
        ),
        (
            GET,
            "/grants/cannot-give",
            None,
            Some(api.schema::<CannotGiveAnswer>()),
        ),
        (GET, "/grants/{id}", None, Some(api.schema::<GrantView>())),
        (POST, "/grants/{id}/revoke", Some(revoke), Some(recorded)),
        (
            GET,
            "/receipts/{index}",
            None,
            Some(api.schema::<ReceiptPage>()),
        ),
        (
            GET,
            "/service-key",
            None,
            Some(api.schema::<ServiceKeyView>()),
        ),
        (GET, "/reviews", None, Some(api.schema::<ReviewView>())),
        (
            POST,
            "/reviews/agents/{agent}/keep",
            Some(api.schema::<KeepAgentBody>()),
            Some(api.schema::<crate::reviews_state::KeptBatch>()),
        ),
        (
            POST,
            "/reviews/{grant}/keep",
            Some(keep),
            Some(api.schema::<Kept>()),
        ),
    ]
}
