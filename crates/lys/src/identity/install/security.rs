//! Install-owned provider keys and development operator credentials.

use super::{
    DeploymentConfig, Ed25519Identity, ErrorKind, IdentityError, IdentityResult, Profile,
    private_files, server_config, setup_code,
};

/// The install's operator token: under the development profile, made once
/// and kept owner-only; under the service profile, removed if one stands,
/// so nothing on disk can act as the administrator. Says what it did.
pub(super) fn operator_token(
    config: &DeploymentConfig,
    profile: Profile,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    let path = config.state_dir().join(server_config::OPERATOR_TOKEN_FILE);
    match (profile, path.exists()) {
        (Profile::Development, true) | (Profile::Service, false) => Ok(()),
        (Profile::Development, false) => {
            let token = setup_code::generate();
            private_files::write(&path, token.expose().as_bytes())?;
            say("operator token made: development profile");
            Ok(())
        }
        (Profile::Service, true) => {
            std::fs::remove_file(&path).map_err(|error| {
                IdentityError::new(
                    ErrorKind::PrivateFileIo,
                    "remove",
                    "operator token",
                    error.to_string(),
                )
                .at(&path)
            })?;
            say("operator token removed: a service install keeps none");
            Ok(())
        }
    }
}

/// The key Lys signs products' ID tokens with, kept in the state folder.
pub(super) fn provider_key(config: &DeploymentConfig) -> IdentityResult<()> {
    let path = config.state_dir().join(server_config::PROVIDER_KEY_FILE);
    Ed25519Identity::load_or_generate(&path).map_err(|error| {
        IdentityError::new(
            ErrorKind::PrivateFileIo,
            "generate provider key",
            "provider key",
            error.to_string(),
        )
        .at(&path)
    })?;
    Ok(())
}
