//! A machine's pass (ACCESS-005 R1: a grant to the machine "is in its
//! pass").
//!
//! A machine signs in nowhere: it proves itself with the key its join
//! recorded, where a person uses the sign-in (`machine_pass.rs` judges that
//! signature before anything here is asked). It is then issued the same
//! pass a person is, from the same reading (`rights_claim::pass`): its
//! rights on the audience app's own kinds as the grants live at issue give
//! them, its holder asserted as `machine` with the person answering for it,
//! and a retired machine refused `HolderRetired`. The audience is an app
//! approved on the Apps screen, judged from the apps' record as it stands
//! at this request, so an app not approved or retired is refused by the
//! apps' own names.
//!
//! The pass lives the provider's `pass_seconds` and no longer: with no
//! sign-in to stand on, no refresh token is issued and the machine asks
//! again with its key. The pass is not kept in the provider's tokens, which
//! hold what a sign-in issued and end with it, so Lys's own routes that
//! take a bearer pass (`bearer::pass_holder`) refuse it `TokenUnknown`;
//! products verify it offline with lys-pass, as they verify every pass.
//!
//! The apps lock is taken and released before the grants are read; no
//! provider lock is held under either.

use lys_identity::{IdentityId, MachineId};
use serde::Serialize;

use super::endpoints::{apps, provider};
use super::grant_binding::asked;
use super::rights_claim::pass;
use crate::apps_binding::sign_in_app;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

/// A machine's pass, answered once to the machine that asked.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub(crate) struct MachinePass {
    /// The pass: a compact JWS whose claims are lys-pass's.
    pub(crate) access_token: String,
    /// Always `Bearer`.
    pub(crate) token_type: String,
    /// How many seconds from now the pass is good for.
    pub(crate) expires_in: u64,
    /// The pass's signed grant binding, when the request asked for one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) grant_binding: Option<String>,
}

/// The pass of `machine` to the approved app `app`, at this request, with
/// its grant binding when `binding` names the version served. The machine
/// is the caller's to have authenticated.
pub(crate) fn machine_pass(
    state: &AppState,
    machine: MachineId,
    app: &str,
    binding: Option<&str>,
) -> Result<MachinePass, ServerError> {
    let provider = provider(state)?;
    // An unsupported binding version is refused before anything is read.
    let bind = asked(binding)?;
    let audience = {
        let mut apps = apps(state)?;
        apps.settle()?;
        sign_in_app(apps.held(), app)?.registered.app.clone()
    };
    let at = now();
    let issued = pass(
        state,
        provider,
        (IdentityId::Machine(machine), audience.as_str()),
        (at, at.saturating_add(provider.pass_seconds)),
        bind,
    )?;
    Ok(MachinePass {
        access_token: issued.token,
        token_type: "Bearer".to_owned(),
        expires_in: issued.expires_at.saturating_sub(at),
        grant_binding: issued.binding,
    })
}
