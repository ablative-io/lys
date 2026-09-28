//! The stores kept in one file each, opened at start, each saying what it
//! holds.

use crate::config::Config;
use crate::error::ServerError;
use crate::network_store::NetworkStore;
use crate::provisioning_store::ProvisioningStore;
use crate::roles_store::RolesStore;

/// The machines, roles and provisioning profiles, each when its file is named.
pub type FileStores = (
    Option<NetworkStore>,
    Option<RolesStore>,
    Option<ProvisioningStore>,
);

/// The machines, roles and provisioning profiles `config` names, each
/// saying through `say` how many it holds; none for a file not named.
pub fn opened(
    config: &Config,
    say: &(dyn Fn(&str) + Send + Sync),
) -> Result<FileStores, ServerError> {
    let network = config
        .network_file
        .as_deref()
        .map(NetworkStore::open)
        .transpose()?;
    if let Some(store) = &network {
        say(&format!(
            "machines read from one file, holding {} machines",
            store.machines().len()
        ));
    }
    let roles = config
        .roles_file
        .as_deref()
        .map(RolesStore::open)
        .transpose()?;
    if let Some(store) = &roles {
        say(&format!(
            "roles read from one file, holding {} roles",
            store.roles().len()
        ));
    }
    let provisioning = config
        .provisioning_file
        .as_deref()
        .map(ProvisioningStore::open)
        .transpose()?;
    if let Some(store) = &provisioning {
        say(&format!(
            "provisioning profiles read from one file, holding {} profiles",
            store.profiles().len()
        ));
    }
    Ok((network, roles, provisioning))
}
