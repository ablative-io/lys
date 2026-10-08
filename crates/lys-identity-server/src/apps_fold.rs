//! The fold's rules: which line the apps as they stand take, and what each
//! line does to its app once taken. `Held::hold` applies them in order, and
//! a start replays them over the leaves after the snapshot.

use super::{
    App, Applied, By, Connected, Held, Line, LysRecorded, Refused, Registered, SignInSet, Standing,
    Version,
};

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
        connector: None,
        declined: None,
        retired: None,
        versions: vec![version],
        pending: None,
        history: Vec::new(),
    }
}

/// Whether `line` is the sign-in settings or the connector kept beside
/// `kept`, an approval of the same app under the same operation: the one case
/// where an operation names more than one line.
pub(super) fn beside_its_approval(kept: &Line, line: &Line) -> bool {
    match (kept, line) {
        (
            Line::Approved(approved),
            Line::SignInSet(SignInSet { app, operation, .. })
            | Line::Connector(Connected { app, operation, .. }),
        ) => approved.app == *app && approved.operation == *operation,
        _ => false,
    }
}

/// Refuse a connector line unless its connector and its approver read back
/// as a connector's id and a person's id.
pub(super) fn ids_read(connected: &Connected) -> Result<(), String> {
    let named =
        |error: lys_identity::IdentityError| format!("line `{}`: {error}", connected.operation);
    connected
        .connector
        .parse::<lys_identity::ConnectorId>()
        .map_err(named)?;
    connected
        .approver
        .parse::<lys_identity::PersonId>()
        .map_err(named)?;
    Ok(())
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
        Line::Retired(_) | Line::Connector(_)
            if app.registered.app == lys_identity::grants::LYS_APP =>
        {
            Err(Refused::Lys)
        }
        Line::Retired(_)
        | Line::Proposed(_)
        | Line::Applied(_)
        | Line::Placed(_)
        | Line::Connector(_)
        | Line::ClientCredentialIssued(_)
        | Line::ClientCredentialRevoked(_)
        | Line::CustodyPrepared(_)
            if standing != Standing::Approved =>
        {
            Err(Refused::Standing(standing))
        }
        Line::Connector(_) if app.connector.is_some() => Err(Refused::Exists),
        Line::CustodyPrepared(prepared)
            if app.approved.is_none()
                || prepared.client.client_id != prepared.app
                || prepared.client.secret_sha256.len() != 64
                || !prepared.client.secret_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
                || prepared.owner.parse::<lys_identity::PersonId>().is_err()
                || prepared.client_secret_ref != format!("lys-app-{}-{}-client", prepared.owner, prepared.app)
                || prepared.api_credential_ref != if prepared.bearer_issued {
                    format!("lys-app-{}-{}-api-{}", prepared.owner, prepared.app, prepared.operation)
                } else { format!("lys-app-{}-{}-api", prepared.owner, prepared.app) } =>
        {
            Err(Refused::Credential)
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
        Line::ClientCredentialIssued(issued)
            if app
                .client_credentials()
                .iter()
                .any(|held| held.issued.credential_id == issued.credential_id) =>
        {
            Err(Refused::Exists)
        }
        Line::ClientCredentialRevoked(revoked)
            if !app
                .live_client_credentials()
                .contains(&revoked.credential_id) =>
        {
            Err(Refused::Credential)
        }
        _ => Ok(()),
    }
}

/// Apply `line`, already allowed, to `app`.
pub(super) fn apply(app: &mut App, line: &Line) {
    match line {
        Line::CustodyPrepared(prepared) => {
            if let Some(approved) = &mut app.approved {
                approved.client = prepared.client.clone();
            }
        }
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
        Line::Connector(connected) => app.connector = Some(connected.clone()),
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
        // A credential's lines are read from the app's history, where every
        // line after its registration is kept (`App::client_credentials`).
        Line::Lys(_)
        | Line::Registered(_)
        | Line::Placed(_)
        | Line::Registrar(_)
        | Line::ClientCredentialIssued(_)
        | Line::ClientCredentialRevoked(_)
        | Line::ClientCredentialsEnded(_) => {}
    }
}
