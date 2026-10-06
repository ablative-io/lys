//! The connector an approval makes (DIRECTORY-080 R1, box 12.2): an app's
//! own identity, answering to the administrator who approved it as the
//! person who holds their login at that moment. Its id is made once, from
//! the secure random source, and kept in the approval's own batch.

use std::str::FromStr;
use std::sync::Arc;

use lys_identity::projection::Projection;
use lys_identity::projection::accounts::Accounts;
use lys_identity::{ConnectorId, LoginBinding, PersonId, Profile};

use crate::apps_error::AppError;
use crate::apps_state::{By, Connected, Held, Standing};
use crate::error::ServerError;

/// The person who holds the login `by` acted under, or the refusal that no
/// one does: a connector answers to a person, never to a bare login.
pub(crate) fn approver(by: &By, projection: &Projection) -> Result<PersonId, ServerError> {
    let login = match by {
        By::Person { login } | By::Operator { login } => login,
        By::ServiceAccount { id } => {
            return Err(AppError::ConnectorNeedsAPerson {
                login: format!("service account {id}"),
            }
            .into());
        }
        By::Start => {
            return Err(AppError::ConnectorNeedsAPerson {
                login: "the service at start".to_owned(),
            }
            .into());
        }
    };
    let binding = LoginBinding::new(&login.provider, &login.subject)?;
    projection.person_for(&binding).ok_or_else(|| {
        AppError::ConnectorNeedsAPerson {
            login: format!("{} {}", login.provider, login.subject),
        }
        .into()
    })
}

/// `accounts` with every app's connector beside them, as the engine judges
/// them (box 12.4): each answers to its approver and is retired with its
/// app. Its profile is the app's id, which is always a name the directory
/// records. Unchanged when no app holds a connector.
pub(crate) fn with_connectors(
    mut accounts: Arc<Accounts>,
    apps: &Held,
) -> Result<Arc<Accounts>, ServerError> {
    for app in &apps.apps {
        let Some(line) = &app.connector else {
            continue;
        };
        let (By::Person { login } | By::Operator { login }) = &line.by else {
            return Err(AppError::ConnectorNeedsAPerson {
                login: format!("the connector line `{}`'s approval", line.operation),
            }
            .into());
        };
        Arc::make_mut(&mut accounts).put_connector(
            ConnectorId::from_str(&line.connector)?,
            PersonId::from_str(&line.approver)?,
            &Profile::new(&line.app)?,
            app.standing() == Standing::Retired,
            &LoginBinding::new(&login.provider, &login.subject)?,
        );
    }
    Ok(accounts)
}

/// The connector line for app `app`, approved under `operation` by `by`: a
/// new connector id answering to the person `by`'s login names.
pub(crate) fn made(
    by: &By,
    projection: &Projection,
    operation: &str,
    app: &str,
    at: u64,
) -> Result<Connected, ServerError> {
    let approver = approver(by, projection)?;
    Ok(Connected {
        operation: operation.to_owned(),
        app: app.to_owned(),
        connector: ConnectorId::generate()?.to_string(),
        approver: approver.to_string(),
        by: by.clone(),
        at,
    })
}
