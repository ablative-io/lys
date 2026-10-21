//! The admission's letter step on an install with no mail settings
//! (DIRECTORY-112 R3): in place of the added letter, a proving token is
//! minted and its digest recorded in the foundings store as an admission,
//! naming the person and the organisation they were admitted into, and the
//! link `<public origin>/#/added/<token>` is handed in the answer for the
//! admitter to give them. The link signs the person in once, by
//! `HandedLink`, through POST /organisations/found/prove. An unspent
//! admission link for the same person is replaced in the same write, so one
//! link at a time signs them in.

use lys_identity::{OrganisationId, PersonId};

use crate::error::ServerError;
use crate::foundings_store::{Founding, Kind, digest, new_token};
use crate::mail::LetterView;
use crate::organisations_found::foundings;
use crate::routes::AppState;

/// Mint and record `person`'s admission link and answer the letter view
/// that hands it.
///
/// # Errors
/// Names the operating system's refusal to give random bytes, or a
/// foundings write that did not reach the disk.
pub(crate) fn hand(
    state: &AppState,
    person: PersonId,
    organisation: OrganisationId,
    email: &str,
    operation: &str,
) -> Result<LetterView, ServerError> {
    let token = new_token()?;
    let admission = Founding {
        organisation: organisation.to_string(),
        founder: person.to_string(),
        email: email.to_owned(),
        operation: operation.to_owned(),
        spent: false,
        handed: true,
        kind: Kind::Admission,
    };
    let mut held = foundings(state)?;
    let replaced = held
        .unspent_admission_for(&admission.founder)
        .map(str::to_owned);
    held.record(digest(&token), admission, replaced.as_deref())?;
    drop(held);
    let link = format!("{}/#/added/{token}", state.oidc.public_origin());
    Ok(LetterView::handed("added", email.to_owned(), link))
}
