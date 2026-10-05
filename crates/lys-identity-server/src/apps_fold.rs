//! The fold's rules: which line the apps as they stand take, and what each
//! line does to its app once taken. `Held::hold` applies them in order, and
//! a start replays them over the leaves after the snapshot.

use super::{App, Applied, By, Held, Line, LysRecorded, Refused, Registered, Standing, Version};

/// The app `lys` as its one line records it: approved at the model's version.
pub(super) fn lys_app(lys: LysRecorded) -> App {
    let version = Version {
        version: lys.version,
        schema: lys.schema.clone(),
        operation: lys.operation.clone(),
        by: By::Start,
        at: lys.at,
    };
    App {
        registered: Registered {
            operation: lys.operation,
            app: lys_identity::grants::LYS_APP.to_owned(),
            name: "Lys".to_owned(),
            redirects: Vec::new(),
            schema: lys.schema,
            service_account: None,
            by: By::Start,
            at: lys.at,
        },
        approved: None,
        sign_in: None,
        declined: None,
        retired: None,
        versions: vec![version],
        pending: None,
        history: Vec::new(),
    }
}

/// Whether `line` is the sign-in settings kept beside `kept`, an approval of
/// the same app under the same operation: the one case where an operation
/// names two lines.
pub(super) fn beside_its_approval(kept: &Line, line: &Line) -> bool {
    matches!(
        (kept, line),
        (Line::Approved(approved), Line::SignInSet(set))
            if approved.app == set.app && approved.operation == set.operation
    )
}

/// Whether `line` may be kept on `app` as it stands.
pub(super) fn allows_on(app: &App, line: &Line, held: &Held) -> Result<(), Refused> {
    let standing = app.standing();
    let current = app.current().map_or(0, |version| version.version);
    match line {
        Line::Approved(_) | Line::Declined(_) if standing != Standing::Pending => {
            Err(Refused::Standing(standing))
        }
        Line::SignInSet(_) if standing != Standing::Approved => Err(Refused::Standing(standing)),
        // The settings beside an approval are kept once: a second line under
        // the approval's operation is a repeat, not a change.
        Line::SignInSet(set)
            if app.sign_in.is_some()
                && app
                    .approved
                    .as_ref()
                    .is_some_and(|approved| approved.operation == set.operation) =>
        {
            Err(Refused::Exists)
        }
        Line::Retired(_) if app.registered.app == lys_identity::grants::LYS_APP => {
            Err(Refused::Lys)
        }
        Line::Retired(_) | Line::Proposed(_) | Line::Applied(_) | Line::Placed(_)
            if standing != Standing::Approved =>
        {
            Err(Refused::Standing(standing))
        }
        Line::Proposed(_) if app.pending.is_some() => Err(Refused::Pending),
        Line::Proposed(proposed) if proposed.replaces != current => Err(Refused::Moved(current)),
        Line::Applied(applied) if applied.version != current + 1 => Err(Refused::Moved(current)),
        Line::Applied(Applied {
            proposal: Some(proposal),
            ..
        }) if app.pending.as_ref().map(|pending| &pending.operation) != Some(proposal) => {
            Err(Refused::NothingPending)
        }
        Line::Applied(Applied { proposal: None, .. }) if app.pending.is_some() => {
            Err(Refused::Pending)
        }
        Line::ChangeDeclined(_) if app.pending.is_none() => Err(Refused::NothingPending),
        Line::Placed(placed) if held.parent(&placed.child_kind, &placed.child_id).is_some() => {
            Err(Refused::Placed)
        }
        _ => Ok(()),
    }
}

/// Apply `line`, already allowed, to `app`.
pub(super) fn apply(app: &mut App, line: &Line) {
    match line {
        Line::Approved(approved) => {
            app.versions.push(Version {
                version: 1,
                schema: app.registered.schema.clone(),
                operation: approved.operation.clone(),
                by: approved.by.clone(),
                at: approved.at,
            });
            app.approved = Some(approved.clone());
        }
        Line::SignInSet(set) => app.sign_in = Some(set.clone()),
        Line::Declined(declined) => app.declined = Some(declined.clone()),
        Line::Retired(retired) => app.retired = Some(retired.clone()),
        Line::Proposed(proposed) => app.pending = Some(proposed.clone()),
        Line::ChangeDeclined(_) => app.pending = None,
        Line::Applied(applied) => {
            app.pending = None;
            app.versions.push(Version {
                version: applied.version,
                schema: applied.schema.clone(),
                operation: applied.operation.clone(),
                by: applied.by.clone(),
                at: applied.at,
            });
        }
        Line::Lys(_) | Line::Registered(_) | Line::Placed(_) | Line::Registrar(_) => {}
    }
}
