//! A later build's model, applied over the app `lys`'s schema at start.
//!
//! Once the apps log holds the app `lys`, the log is its only source and the
//! model file is read only to learn whether a later build ships more: when
//! the file's version is above the held one and its schema differs, it is
//! applied as the next version, by the service at start. A model that would
//! take a relation or an action away is refused by name, since it could
//! strand a standing grant; only the administrator changes `lys` that way.

use lys_identity::grants::{ANY_KIND, AppSchema, LYS_APP};

use crate::apps_state::{Applied, By, Line};
use crate::apps_store::AppStore;
use crate::config::Config;
use crate::error::ServerError;
use crate::session::now;

/// Apply the model file over the held schema of version `held`, when the
/// file is a later version that only adds, saying through `say` which.
pub(crate) fn upgrade(
    config: &Config,
    store: &mut AppStore,
    held: u64,
    say: &dyn Fn(&str),
) -> Result<(), ServerError> {
    let model = config.grant_model()?;
    let file = config.grant_model_file.display();
    let current = store
        .app(LYS_APP)
        .and_then(|lys| lys.current())
        .map(|version| AppSchema::parse(LYS_APP, &version.schema))
        .transpose()
        .map_err(|error| {
            refused(format!(
                "the held schema of the app lys is unreadable: {error}"
            ))
        })?
        .ok_or_else(|| refused("the app lys holds no current schema".to_owned()))?;
    let relations = model
        .relations()
        .map(|(relation, actions)| (relation.clone(), actions.clone()))
        .collect();
    let shipped = AppSchema::lys(relations);
    if model.version() <= held || shipped == current {
        say(&format!(
            "grant_model_file_ignored: the app lys holds schema version {held} in the apps log, which is its only source; {file} at version {} adds nothing to it",
            model.version()
        ));
        return Ok(());
    }
    let (Some(was), Some(now_kind)) =
        (current.kinds().get(ANY_KIND), shipped.kinds().get(ANY_KIND))
    else {
        return Err(refused("the app lys's schema has no kind".to_owned()));
    };
    for (relation, actions) in &was.relations {
        let taken: Vec<&str> = now_kind
            .relations
            .get(relation)
            .map_or_else(
                || actions.iter().collect::<Vec<_>>(),
                |kept| actions.difference(kept).collect::<Vec<_>>(),
            )
            .into_iter()
            .map(lys_identity::grants::Action::as_str)
            .collect();
        if !taken.is_empty() {
            return Err(refused(format!(
                "{file} at version {} takes {} from the relation `{relation}`; a model applied at start only adds, and the administrator changes the app lys's schema otherwise",
                model.version(),
                taken.join(", ")
            )));
        }
    }
    let version = held + 1;
    store.keep(Line::Applied(Applied {
        operation: format!("lys-model-{version}"),
        app: LYS_APP.to_owned(),
        version,
        schema: shipped.to_json(),
        proposal: None,
        by: By::Start,
        at: now(),
    }))?;
    say(&format!(
        "lys_model_applied: {file} at version {} adds to the app lys's schema, now version {version}",
        model.version()
    ));
    Ok(())
}

fn refused(reason: String) -> ServerError {
    ServerError::ConfigInvalid { reason }
}
