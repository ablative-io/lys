#![cfg(test)]

use std::collections::BTreeSet;
use std::error::Error;

use lys_identity::grants::{Action, Model, Relation, shipped};
use lys_identity_server::grant_contract::ModelView;
use serde_json::Value;

#[test]
fn every_shipped_action_is_served_with_a_plain_sentence() -> Result<(), Box<dyn Error>> {
    let actions: Value = serde_json::from_str(&shipped::shipped_model())?;
    let owner = actions["relations"]["owner"]
        .as_array()
        .ok_or("missing owner")?;
    let names = owner
        .iter()
        .map(|action| Action::new(action.as_str().ok_or("invalid action")?).map_err(Into::into))
        .collect::<Result<BTreeSet<_>, Box<dyn Error>>>()?;
    let model = Model::new(shipped::SHIPPED_VERSION, [(Relation::new("owner")?, names)])?;
    let view = serde_json::to_value(ModelView::from(&model))?;
    let sentences = view["action_sentences"]
        .as_object()
        .ok_or("missing action sentences")?;
    for action in owner {
        let action = action.as_str().ok_or("invalid action")?;
        let sentence = sentences
            .get(action)
            .and_then(Value::as_str)
            .ok_or("missing sentence")?;
        assert!(
            !sentence.is_empty() && !sentence.contains('.') && !sentence.contains('-'),
            "{action}: {sentence}"
        );
    }
    assert_eq!(sentences["agent.start"], "Start this agent");
    Ok(())
}
