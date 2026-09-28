//! The grant routes' typed wire: every request body they take and every
//! answer they write.

mod requests;
mod views;

pub use requests::{
    ActionBody, CannotGiveBody, DelegateBody, PAGE_MAX, PassOnWire, ResourceWire, RevokeBody,
    RootBody, RouteWire, WhoBody, WindowWire, grant_id,
};
pub use views::{
    CannotGiveAnswer, CannotGiveItemView, CannotGiveReasonView, CannotGiveSubjectView, GrantList,
    GrantView, HolderView, LastUseView, LogView, ModelView, PassOnView, PermitView, ReceiptView,
    RecordedView, RefusedView, ResourceView, StandingView, UnknownCannotGiveReason, UnreportedView,
    UseEventView, WhoPage, WindowView,
};
