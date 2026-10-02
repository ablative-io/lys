//! Recorded bootstrap authority is never replaced by startup after a model change.

use lys_identity::grants::{Grant, GrantRecord, PassOn, RecipientKind, Relation, Resource, Source};
use lys_identity::{IdentityId, PersonId, ServiceAccountId};
use serde_json::json;

use crate::error::ServerError;
use crate::grants::Judged;
use crate::import_bootstrap::operation;

struct Bootstrap<'a> {
    account: &'a str,
    owner: PersonId,
    holder: ServiceAccountId,
    collection: &'a str,
    resource: Resource,
    editor: Relation,
}

impl Bootstrap<'_> {
    fn operation(&self, kind: &str, version: u64) -> lys_identity::OperationId {
        let suffix = if version == 1 {
            String::new()
        } else {
            format!("/v{version}")
        };
        operation(self.account, &format!("{}/{kind}{suffix}", self.collection))
    }

    fn root_matches(&self, root: &Grant) -> bool {
        let parts = root.parts();
        parts.issuer == IdentityId::Person(self.owner)
            && parts.holder == IdentityId::Person(self.owner)
            && parts.responsible == self.owner
            && parts.resource == self.resource
            && parts.relation == self.editor
            && parts.source == Source::Root
            && parts.window.starts_at() == 0
            && parts.window.ends_at().is_none()
            && parts.operation == self.operation("root", parts.model_version)
            && matches!(&parts.pass_on, PassOn::To { actions, recipients }
                if actions == &parts.actions && recipients.len() == 1
                    && recipients.contains(&RecipientKind::ServiceAccount))
    }

    fn delegation_matches(&self, root: &Grant, child: &Grant) -> bool {
        let parts = child.parts();
        parts.issuer == IdentityId::Person(self.owner)
            && parts.holder == IdentityId::ServiceAccount(self.holder)
            && parts.responsible == self.owner
            && parts.resource == self.resource
            && parts.relation == self.editor
            && parts.source == Source::Grant(root.id())
            && parts.actions == *root.actions()
            && parts.pass_on == PassOn::UseOnly
            && parts.window == root.window()
            && parts.model_version == root.parts().model_version
            && parts.operation == self.operation("delegated", parts.model_version)
    }

    fn complete(&self, judged: &Judged<'_>, root: &Grant) -> Result<bool, ServerError> {
        let operation = self.operation("delegated", root.parts().model_version);
        let child = judged
            .grants
            .book()
            .on_resource(&self.resource)
            .find(|record| record.grant().parts().operation == operation);
        if let Some(child) = child {
            if self.delegation_matches(root, child.grant()) {
                return Ok(true);
            }
            return Err(self.interrupted(
                judged,
                "the recorded delegation does not match its bootstrap root",
            )?);
        }
        if judged.grants.book().operation(operation).is_some() {
            return Err(self.interrupted(
                judged,
                "the delegation operation has a different recorded answer",
            )?);
        }
        Ok(false)
    }

    fn interrupted(&self, judged: &Judged<'_>, reason: &str) -> Result<ServerError, ServerError> {
        let version = judged.grants.model().version();
        let recorded_model_version = self
            .roots(judged)
            .min_by_key(|record| record.index())
            .map(|record| record.grant().parts().model_version);
        let carried: Vec<_> = judged
            .grants
            .model()
            .actions(&self.editor)?
            .iter()
            .map(lys_identity::grants::Action::as_str)
            .collect();
        let root_operation = self.operation("root", version);
        let source = judged
            .grants
            .book()
            .on_resource(&self.resource)
            .find(|record| record.grant().parts().operation == root_operation)
            .map_or_else(
                || "use grant from the root response".to_owned(),
                |record| record.grant().id().to_string(),
            );
        Ok(ServerError::BootstrapInterrupted {
            reason: json!({
                "reason":reason,
                "recorded_model_version":recorded_model_version,
                "current_model_version":version,
                "act":"disable import_credential_file, restart, sign in as the configured administrator, record these requests, then restore import_credential_file",
                "root_path":"/grants/roots",
                "root":{"operation":root_operation.to_string(),"route":"api","holder":self.owner.to_string(),
                    "resource":{"kind":"directory","id":self.collection},"relation":"editor",
                    "pass_on":{"kind":"to","actions":carried,"recipients":["service_account"]},
                    "window":{"starts_at":0,"ends_at":null}},
                "delegation_path":"/grants",
                "delegation":{"operation":self.operation("delegated",version).to_string(),"route":"api","source":source,
                    "recipient":self.account,"responsible":self.owner.to_string(),
                    "resource":{"kind":"directory","id":self.collection},"relation":"editor",
                    "pass_on":{"kind":"use_only"},"window":{"starts_at":0,"ends_at":null}}
            }).to_string(),
        })
    }

    fn roots<'a>(&'a self, judged: &'a Judged<'_>) -> impl Iterator<Item = &'a GrantRecord> + 'a {
        judged
            .grants
            .book()
            .on_resource(&self.resource)
            .filter(|record| {
                let grant = record.grant();
                grant.parts().operation == self.operation("root", grant.parts().model_version)
            })
    }

    fn resume(&self, judged: &Judged<'_>) -> Result<(), ServerError> {
        let Some(first) = self.roots(judged).min_by_key(|record| record.index()) else {
            return Err(self.interrupted(
                judged,
                "another bootstrap collection was recorded; this collection is incomplete",
            )?);
        };
        let root = first.grant();
        if !self.root_matches(root) {
            return Err(self.interrupted(
                judged,
                "the recorded bootstrap root does not match the configured owner and loader",
            )?);
        }
        if self.complete(judged, root)? {
            return Ok(());
        }
        // A replacement cannot make a withdrawn or ended predecessor stand again.
        lys_identity::grants::admission::effective(
            judged.grants.book(),
            judged.directory,
            root.id(),
            crate::session::now(),
        )?;
        let version = judged.grants.model().version();
        for replacement in self.roots(judged) {
            let replacement = replacement.grant();
            if replacement.parts().model_version > root.parts().model_version
                && replacement.id() != root.id()
                && self.root_matches(replacement)
                && (replacement.parts().model_version != version
                    || replacement.actions() == judged.grants.model().actions(&self.editor)?)
                && self.complete(judged, replacement)?
            {
                return Ok(());
            }
        }
        Err(self.interrupted(judged, "a bootstrap root was recorded without its matching delegation; startup will not issue replacement authority")?)
    }
}

/// Check both collections before issuing anything. A partial installation is
/// recovered by the administrator through the ordinary recorded grant routes.
pub(crate) fn recorded(
    judged: &Judged<'_>,
    account: &str,
    owner: PersonId,
    holder: ServiceAccountId,
) -> Result<bool, ServerError> {
    let collections = ["apps", "agents"]
        .map(|collection| {
            Ok(Bootstrap {
                account,
                owner,
                holder,
                collection,
                resource: Resource::new("directory", collection)?,
                editor: Relation::new("editor")?,
            })
        })
        .into_iter()
        .collect::<Result<Vec<_>, ServerError>>()?;
    let any = collections
        .iter()
        .any(|bootstrap| bootstrap.roots(judged).next().is_some());
    if any {
        for bootstrap in &collections {
            bootstrap.resume(judged)?;
        }
    } else {
        for bootstrap in &collections {
            for kind in ["root", "delegated"] {
                if judged
                    .grants
                    .book()
                    .operation(bootstrap.operation(kind, judged.grants.model().version()))
                    .is_some()
                {
                    return Err(bootstrap.interrupted(
                        judged,
                        "a bootstrap operation already has a recorded answer",
                    )?);
                }
            }
        }
    }
    Ok(any)
}
