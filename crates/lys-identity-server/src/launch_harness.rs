//! Skill text lookup and the shared neutral launch-field checks.

use lys_home::harness::skills::SkillFile;

use crate::error::ServerError;
use crate::provisioning_store::{ProvisioningStore, Version};

pub use crate::start_checks::fields;

/// The text of each skill `version` pinned, as the launch carries it; a pin
/// whose text Lys no longer keeps is refused.
pub fn skill_files(
    store: &ProvisioningStore,
    version: &Version,
) -> Result<Vec<SkillFile>, ServerError> {
    version
        .settings
        .skill_pins
        .iter()
        .map(|pin| {
            store
                .skill(&pin.name, &pin.sha256)
                .map(|kept| SkillFile {
                    name: kept.name.clone(),
                    text: kept.text.clone(),
                    sha256: kept.sha256.clone(),
                })
                .ok_or_else(|| ServerError::SkillUnknown {
                    name: pin.name.clone(),
                })
        })
        .collect()
}
