//! One permission refusal shape for every product route.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Error, Target};

/// The grant the caller needs to request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Needed {
    /// The application audience.
    pub app: String,
    /// The exact resource kind.
    pub kind: String,
    /// The exact resource id.
    pub id: String,
    /// The requested action.
    pub action: String,
}

/// The contents of the shared refusal envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Refused {
    /// The resource as kind and id.
    pub resource: String,
    /// The action refused.
    pub action: String,
    /// The grant that would permit it.
    pub needed: Needed,
    /// The Lys address at which to request that grant.
    pub request: String,
}

/// A serialized refusal with no product-specific outer shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Refusal {
    /// The refused act and how to request permission.
    pub refused: Refused,
}

impl Refusal {
    /// Name an exact needed grant and the issuer's existing request screen.
    pub fn new(app: &str, kind: &str, id: &str, action: &str, issuer: &str) -> Result<Self, Error> {
        let target = Target::new(kind, id, action)?;
        if app.is_empty() || !crate::rights::audience_owns(kind, app) {
            return Err(Error::RightsOutsideAudience);
        }
        let mut base = Url::parse(issuer)?;
        if !matches!(base.scheme(), "http" | "https")
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(Error::Invalid("grant request issuer is invalid"));
        }
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        let mut request = base;
        request.set_fragment(Some("/requests"));
        Ok(Self {
            refused: Refused {
                resource: format!("{kind}:{id}"),
                action: target.action.clone(),
                needed: Needed {
                    app: app.to_owned(),
                    kind: target.kind,
                    id: target.id,
                    action: target.action,
                },
                request: request.into(),
            },
        })
    }
}
