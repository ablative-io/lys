//! Reading the install's permission model.
use std::collections::BTreeSet;

use lys_identity::grants::{Action, Model, Relation};
use serde::Deserialize;

use super::{Config, invalid};
use crate::error::ServerError;

/// The permission model as its file writes it: a version, and each relation
/// with the actions it carries.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelFile {
    version: u64,
    relations: std::collections::BTreeMap<String, Vec<String>>,
}

impl Config {
    /// The permission model, read from its file and checked. It is read only
    /// when the apps log does not yet hold the app `lys`, to record it there
    /// once; afterwards the log is the only source and the file is not read.
    ///
    /// # Errors
    /// Refuses an unreadable model file, malformed model bytes or relations
    /// and app kinds the identity model cannot accept.
    pub fn grant_model(&self) -> Result<Model, ServerError> {
        let path = &self.grant_model_file;
        let text = std::fs::read_to_string(path)
            .map_err(|error| invalid(format!("{} could not be read: {error}", path.display())))?;
        let file: ModelFile = serde_json::from_str(&text)
            .map_err(|error| invalid(format!("{} is not a model: {error}", path.display())))?;
        let mut relations = Vec::with_capacity(file.relations.len());
        for (relation, actions) in file.relations {
            let actions = actions
                .iter()
                .map(|action| Action::new(action))
                .collect::<Result<BTreeSet<_>, _>>()
                .map_err(|error| invalid(format!("grant model: {error}")))?;
            let relation = Relation::new(&relation)
                .map_err(|error| invalid(format!("grant model: {error}")))?;
            relations.push((relation, actions));
        }
        Model::new(file.version, relations)
            .map_err(|error| invalid(format!("grant model: {error}")))
    }
}
