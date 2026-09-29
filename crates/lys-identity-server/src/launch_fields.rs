//! A profile's models checked against its recorded description, never its name.
use lys_home::harness::description::FurtherModels;
use lys_home::harness::launch_fields::DeclaredHarness;

use crate::error::ServerError;

/// Refuse an absent model or the first model the described build cannot carry.
pub fn models(harness: &DeclaredHarness, models: &[String]) -> Result<(), ServerError> {
    let contract = &harness.description.models;
    let refused = |model: &str, reason: &str| ServerError::ModelUnrepresentable {
        harness: harness.name.clone(),
        model: model.to_owned(),
        reason: reason.to_owned(),
    };
    if models.len() < contract.minimum {
        return Err(refused(
            "",
            "description.models.minimum exceeds the supplied model count",
        ));
    }
    if let Some(maximum) = contract.maximum {
        if maximum < contract.minimum {
            return Err(refused(
                "",
                "description.models.maximum is below its minimum",
            ));
        }
        if let Some(model) = models.get(maximum) {
            return Err(refused(
                model,
                "description.models.maximum does not admit this model",
            ));
        }
    }
    if let FurtherModels::Delimited { separator } = &contract.further_encoding {
        if separator.is_empty() {
            return Err(refused(
                "",
                "description.models.further_encoding.separator is empty",
            ));
        }
        if let Some(model) = models.iter().find(|model| model.contains(separator)) {
            return Err(refused(
                model,
                "the model contains description.models.further_encoding.separator",
            ));
        }
    }
    Ok(())
}
