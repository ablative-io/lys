//! The declared permission model served to access pickers.

use lys_identity::grants::Model;
use serde::Serialize;
use std::collections::BTreeMap;

/// One app kind's agent actions and the relations that carry them.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct KindView {
    /// The acts this kind permits an agent to hold.
    pub agents: Vec<String>,
    /// Each relation with every action it actually carries.
    pub relations: BTreeMap<String, Vec<String>>,
    /// The relation a person newly admitted to the app is given first, when
    /// its schema declares one (DIRECTORY-116 R2); absent when it declares none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_grant: Option<String>,
}

/// The permission model grants are judged against.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ModelView {
    /// Plain sentences for the shipped model’s actions.
    pub action_sentences: BTreeMap<String, String>,
    /// The model's version.
    pub version: u64,
    /// Each relation's name, with the actions it carries.
    pub relations: BTreeMap<String, Vec<String>>,
    /// The acts no agent may be given, so a screen never offers them.
    pub withheld_from_agents: Vec<String>,
    /// Each app kind's declared actions and the actions an agent may hold.
    pub kinds: BTreeMap<String, KindView>,
}

impl From<&Model> for ModelView {
    fn from(model: &Model) -> Self {
        Self {
            action_sentences: lys_identity::grants::shipped::ACTION_SENTENCES
                .iter()
                .map(|(action, sentence)| ((*action).to_owned(), (*sentence).to_owned()))
                .collect(),
            kinds: model
                .kinds()
                .iter()
                .map(|(kind, declared)| {
                    (
                        kind.clone(),
                        KindView {
                            first_grant: declared.first_grant.as_ref().map(ToString::to_string),
                            agents: declared
                                .agents
                                .iter()
                                .map(|action| action.as_str().to_owned())
                                .collect(),
                            relations: declared
                                .relations
                                .iter()
                                .map(|(relation, actions)| {
                                    (
                                        relation.to_string(),
                                        actions
                                            .iter()
                                            .map(|action| action.as_str().to_owned())
                                            .collect(),
                                    )
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
            version: model.version(),
            relations: model
                .relations()
                .map(|(relation, actions)| {
                    (
                        relation.to_string(),
                        actions
                            .iter()
                            .map(|action| action.as_str().to_owned())
                            .collect(),
                    )
                })
                .collect(),
            withheld_from_agents: lys_identity::grants::WITHHELD_FROM_AGENTS
                .iter()
                .map(|action| (*action).to_owned())
                .collect(),
        }
    }
}
