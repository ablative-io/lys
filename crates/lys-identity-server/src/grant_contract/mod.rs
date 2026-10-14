//! The grant routes' typed wire: every request body they take and every
//! answer they write.

mod cannot_give;
mod model;
mod requests;
mod views;

pub use requests::{
    ActionBody, CannotGiveBody, DelegateBody, PAGE_MAX, PassOnWire, ResourceWire, RevokeBody,
    RootBody, RouteWire, WhoBody, WindowWire, grant_id,
};
pub use views::{
    GrantList, GrantView, HolderView, LastUseView, LogView, PassOnView, PermitView, ReceiptView,
    RecordedView, RefusedView, ResourceView, StandingView, UnreportedView, UseEventView, WhoPage,
    WindowView,
};

pub use cannot_give::{
    CannotGiveAnswer, CannotGiveItemView, CannotGiveReasonView, CannotGiveSubjectView,
    UnknownCannotGiveReason,
};
pub use model::{KindView, ModelView};
