//! Membership pages (ACCESS-006 R3): the resources one subject may take an
//! action on within a workspace, and the subjects who may take an action on
//! one channel, a bounded page at a time.
//!
//! A page answers under the same contract version and grant log as a single
//! [`MembershipDecision`](crate::membership::MembershipDecision), at one
//! named revision, and every row is the decision Lys would give that single
//! question at that revision: a page never lists a resource or recipient the
//! decision would refuse. The caller states both bounds, a row count and an
//! encoded-byte budget; there is no default page and no unlimited one.
//!
//! The provider names its own ceiling on both bounds, an operator setting;
//! a page asking for more is refused [`PAGE_OVER_CEILING`] before anything
//! is looked up, and a provider with no ceiling configured refuses every
//! page [`PAGES_UNCONFIGURED`] rather than serve an unbounded one.
//!
//! The continuation is an opaque cursor the provider issues and keeps: the
//! text is a random handle the provider looks up, so it cannot be forged or
//! edited. It binds the asker, the grant log and epoch, the question and the
//! last key reached; it never grants anything, since every row of a later
//! page is decided again at that page's revision. A cursor for another asker
//! or question is refused [`CURSOR_FOREIGN`], one from another log or epoch
//! [`CURSOR_RESET`], one the provider cannot read [`CURSOR_MALFORMED`], and
//! one it no longer keeps (its retention passed, its listing completed, or
//! the provider restarted) [`CURSOR_EXPIRED`]. The provider keeps a finite
//! number of cursors; a page that would need one more than that is refused
//! [`CURSORS_FULL`].

use serde::{Deserialize, Serialize};

use crate::membership::{GrantLog, Refused, Subject, check};
use crate::rights::{Mode, Resource};

/// A bound of zero rows or zero bytes was asked for.
pub const BOUND_INVALID: &str = "membership_page_bound_invalid";
/// One row alone is larger than the byte budget, so no page can carry it.
pub const ROW_OVER_BUDGET: &str = "membership_page_row_over_budget";
/// The cursor was issued for another question.
pub const CURSOR_FOREIGN: &str = "membership_cursor_foreign";
/// The cursor was issued from another grant log or epoch.
pub const CURSOR_RESET: &str = "membership_cursor_reset";
/// The cursor is not one this provider issued.
pub const CURSOR_MALFORMED: &str = "membership_cursor_malformed";
/// The cursor is no longer kept: its retention passed, its listing
/// completed, or the provider restarted since it was issued.
pub const CURSOR_EXPIRED: &str = "membership_cursor_expired";
/// The provider already keeps as many cursors as its setting allows.
pub const CURSORS_FULL: &str = "membership_cursors_full";
/// The provider's kept cursors cannot be read, so no page that continues
/// or issues one is served.
pub const CURSORS_UNAVAILABLE: &str = "membership_cursors_unavailable";
/// A bound asks for more than the provider's configured ceiling.
pub const PAGE_OVER_CEILING: &str = "membership_page_over_ceiling";
/// The provider has no page ceiling configured, so it serves no page.
pub const PAGES_UNCONFIGURED: &str = "membership_pages_unconfigured";

/// The caller's bounds on one page, both required and both positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(as = MembershipPageBounds))]
pub struct PageBounds {
    /// The most rows the page carries.
    pub rows: u32,
    /// The most bytes the page's rows encode to, as JSON.
    pub bytes: u32,
}

/// The resources of `kind` within `workspace` that `subject` may `action`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(as = MembershipResourcePageRequest))]
pub struct ResourcePageRequest {
    /// The contract version the asker speaks.
    pub contract: u32,
    /// The grant log the asker's revisions belong to.
    pub log: GrantLog,
    /// The workspace the resources are placed under.
    pub workspace: Resource,
    /// Who is asked about.
    pub subject: Subject,
    /// The kind of resource listed.
    pub kind: String,
    /// The action.
    pub action: String,
    /// The least receipt revision the page must reflect.
    pub at_least: u64,
    /// The page's bounds.
    pub bounds: PageBounds,
    /// The cursor of the previous page; absent for the first.
    pub after: Option<String>,
}

/// The subjects who may `action` on `resource`, placed under `workspace`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(as = MembershipRecipientPageRequest))]
pub struct RecipientPageRequest {
    /// The contract version the asker speaks.
    pub contract: u32,
    /// The grant log the asker's revisions belong to.
    pub log: GrantLog,
    /// The workspace the resource is placed under.
    pub workspace: Resource,
    /// The channel or other placed resource.
    pub resource: Resource,
    /// The action.
    pub action: String,
    /// The least receipt revision the page must reflect.
    pub at_least: u64,
    /// The page's bounds.
    pub bounds: PageBounds,
    /// The cursor of the previous page; absent for the first.
    pub after: Option<String>,
}

/// One allowed membership: who, on what, by which grant reaching from where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(as = MembershipPageRow))]
pub struct PageRow {
    /// The subject.
    pub subject: Subject,
    /// The resource.
    pub resource: Resource,
    /// The grant whose authority reaches it.
    pub grant: String,
    /// The resource whose grant reaches it.
    pub scope: Resource,
    /// How the grant is exercised; a held mode is listed, since the
    /// membership exists, and the act still waits for its approval.
    pub mode: Mode,
}

/// A page, or the refusal of the whole page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(as = MembershipPageOutcome))]
pub enum PageOutcome {
    /// The rows of this page.
    Page {
        /// The rows, in the stable order of their keys.
        rows: Vec<PageRow>,
        /// How many rows the page carries.
        returned: u64,
        /// How many candidates this page decided and left out, refused.
        skipped: u64,
        /// Whether no row remains after this page.
        complete: bool,
        /// The cursor of the next page; absent when complete.
        next: Option<String>,
    },
    /// The page is refused, with no row.
    Refused {
        /// The stable refusal name.
        refusal: String,
        /// The refusal's words.
        reason: String,
    },
}

/// The answer to a page request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(as = MembershipPage))]
pub struct MembershipPage {
    /// The contract version answered under.
    pub contract: u32,
    /// The grant log decided from.
    pub log: GrantLog,
    /// The revision every row was decided at; absent when refused before
    /// the grants were asked.
    pub revision: Option<u64>,
    /// The page.
    pub outcome: PageOutcome,
}

/// Refuse `bounds` asking for more than the provider's `ceiling`, a bound
/// at a time; [`validate_resources`] and [`validate_recipients`] have
/// already refused a zero bound.
pub fn within_ceiling(bounds: PageBounds, ceiling: PageBounds) -> Result<(), Refused> {
    if bounds.rows > ceiling.rows || bounds.bytes > ceiling.bytes {
        return Err(Refused {
            name: PAGE_OVER_CEILING,
            reason: format!(
                "a page is bounded by at most {} rows and {} bytes here",
                ceiling.rows, ceiling.bytes
            ),
        });
    }
    Ok(())
}

fn bounded(bounds: PageBounds) -> Result<(), Refused> {
    if bounds.rows == 0 || bounds.bytes == 0 {
        return Err(Refused {
            name: BOUND_INVALID,
            reason: "a page is bounded by at least one row and one byte".to_owned(),
        });
    }
    Ok(())
}

/// Refuse a resource page request this contract cannot ask.
pub fn validate_resources(request: &ResourcePageRequest, served: &GrantLog) -> Result<(), Refused> {
    let mut members = vec![
        ("workspace kind", request.workspace.kind.as_str()),
        ("workspace id", request.workspace.id.as_str()),
        ("subject id", request.subject.id.as_str()),
        ("subject kind", request.subject.kind.as_str()),
        ("kind", request.kind.as_str()),
        ("action", request.action.as_str()),
    ];
    if let Some(after) = &request.after {
        members.push(("cursor", after.as_str()));
    }
    check(request.contract, &request.log, served, &members)?;
    bounded(request.bounds)
}

/// Refuse a recipient page request this contract cannot ask.
pub fn validate_recipients(
    request: &RecipientPageRequest,
    served: &GrantLog,
) -> Result<(), Refused> {
    let mut members = vec![
        ("workspace kind", request.workspace.kind.as_str()),
        ("workspace id", request.workspace.id.as_str()),
        ("resource kind", request.resource.kind.as_str()),
        ("resource id", request.resource.id.as_str()),
        ("action", request.action.as_str()),
    ];
    if let Some(after) = &request.after {
        members.push(("cursor", after.as_str()));
    }
    check(request.contract, &request.log, served, &members)?;
    bounded(request.bounds)
}

/// The page refusing a request by `name` before the grants were asked.
#[must_use]
pub fn refused_page(served: &GrantLog, name: &str, reason: &str) -> MembershipPage {
    MembershipPage {
        contract: crate::membership::CONTRACT_VERSION,
        log: served.clone(),
        revision: None,
        outcome: PageOutcome::Refused {
            refusal: name.to_owned(),
            reason: reason.to_owned(),
        },
    }
}
