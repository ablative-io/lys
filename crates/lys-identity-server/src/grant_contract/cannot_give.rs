//! Typed cannot-give answers refuse unknown reason names.

use lys_identity::grants::{CannotGiveList, CannotGiveReason, CannotGiveSubject};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An answer named a cannot-give reason outside the six. It is refused
/// whole, never defaulted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "unknown_cannot_give_reason: `{reason}` is not a cannot-give reason, which is one of sign_in_identity, above_what_you_hold, lent_to_you, use_only, people_only or agents_only"
)]
pub struct UnknownCannotGiveReason {
    /// The reason the answer named.
    pub reason: String,
}

/// Why an item cannot be given, using admission's reason names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CannotGiveReasonView(pub CannotGiveReason);

impl utoipa::ToSchema for CannotGiveReasonView {}

impl utoipa::PartialSchema for CannotGiveReasonView {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .enum_values(Some(CannotGiveReason::ALL.map(CannotGiveReason::name)))
            .build()
            .into()
    }
}

impl Serialize for CannotGiveReasonView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.name())
    }
}

impl<'de> Deserialize<'de> for CannotGiveReasonView {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let reason = String::deserialize(deserializer)?;
        CannotGiveReason::from_name(&reason)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom(UnknownCannotGiveReason { reason }))
    }
}

/// What one cannot-give item is about. A grant or service-account item names
/// the caller's own grant; a relation item names the relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "subject", rename_all = "snake_case")]
pub enum CannotGiveSubjectView {
    /// A grant the caller holds.
    Grant {
        /// The caller's grant.
        grant: String,
    },
    /// A relation of the model on the source grant's resource.
    Relation {
        /// The relation's name.
        relation: String,
    },
    /// A service account the caller holds.
    ServiceAccount {
        /// The caller's grant on it.
        grant: String,
    },
    /// The caller's own sign-in identity.
    SignInIdentity,
}

/// One thing the caller cannot give, with its one reason. No item carries a standing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CannotGiveItemView {
    /// What it is.
    #[serde(flatten)]
    pub subject: CannotGiveSubjectView,
    /// The one reason it cannot be given.
    pub reason: CannotGiveReasonView,
    /// Whether it is the grant the form was opened from.
    pub source: bool,
}

/// Everything the caller cannot give the recipient, in the list's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CannotGiveAnswer {
    /// The grant the form was opened from.
    pub source: String,
    /// The recipient asked about.
    pub recipient: String,
    /// The items, in order.
    pub items: Vec<CannotGiveItemView>,
}

impl From<&CannotGiveList> for CannotGiveAnswer {
    fn from(list: &CannotGiveList) -> Self {
        Self {
            source: list.source.to_string(),
            recipient: list.recipient.to_string(),
            items: list
                .items
                .iter()
                .map(|item| CannotGiveItemView {
                    subject: match &item.subject {
                        CannotGiveSubject::Grant(grant) => CannotGiveSubjectView::Grant {
                            grant: grant.to_string(),
                        },
                        CannotGiveSubject::Relation(relation) => CannotGiveSubjectView::Relation {
                            relation: relation.as_str().to_owned(),
                        },
                        CannotGiveSubject::ServiceAccount(grant) => {
                            CannotGiveSubjectView::ServiceAccount {
                                grant: grant.to_string(),
                            }
                        }
                        CannotGiveSubject::SignInIdentity => CannotGiveSubjectView::SignInIdentity,
                    },
                    reason: CannotGiveReasonView(item.reason),
                    source: item.source,
                })
                .collect(),
        }
    }
}
