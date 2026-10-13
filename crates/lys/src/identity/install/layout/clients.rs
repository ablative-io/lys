//! Immutable executable releases and their transaction-owned serving manifest.

use super::Layout;
use crate::identity::error::{ErrorKind, IdentityError, IdentityResult};
use std::path::PathBuf;

/// One installed client release and the digests of its executable pair.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientRelease {
    /// The source commit shared by the install and the clients.
    pub commit: String,
    /// Executable names and their SHA-256 digests.
    pub files: std::collections::BTreeMap<String, String>,
}

/// Client inputs read completely before an install changes its running build.
pub struct ClientSet {
    /// The releases the build record will serve.
    pub releases: std::collections::BTreeMap<String, ClientRelease>,
    files: Vec<(PathBuf, Vec<u8>)>,
}

fn client_error(path: &std::path::Path, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(
        ErrorKind::BinaryMissing,
        "prepare clients",
        "clients",
        detail,
    )
    .at(path)
}

/// The Rust target of the host client that was built beside this installer.
pub fn host_platform() -> String {
    let suffix = if cfg!(target_os = "macos") {
        "apple-darwin"
    } else if cfg!(target_env = "musl") {
        "unknown-linux-musl"
    } else {
        "unknown-linux-gnu"
    };
    format!("{}-{suffix}", std::env::consts::ARCH)
}

/// Read every supplied client before making any change to the install.
///
/// # Errors
/// Refuses an invalid platform, missing executable, unreadable file or unstamped commit.
pub fn prepare_clients(
    source: Option<&std::path::Path>,
    host: &std::path::Path,
    commit: &str,
) -> IdentityResult<ClientSet> {
    use sha2::{Digest, Sha256};
    if commit.len() < 8
        || !commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(client_error(
            host,
            "client_commit_invalid: clients require a clean source commit",
        ));
    }
    let folders = if let Some(source) = source {
        let mut folders = Vec::new();
        for entry in
            std::fs::read_dir(source).map_err(|error| client_error(source, error.to_string()))?
        {
            let entry = entry.map_err(|error| client_error(source, error.to_string()))?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| client_error(source, "a platform is not UTF-8"))?;
            if !entry
                .file_type()
                .map_err(|error| client_error(&entry.path(), error.to_string()))?
                .is_dir()
                || name.is_empty()
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            {
                return Err(client_error(
                    &entry.path(),
                    "client_platform_invalid: expected one folder per Rust target",
                ));
            }
            folders.push((name, entry.path()));
        }
        folders.sort();
        if folders.is_empty() {
            return Err(client_error(
                source,
                "clients_empty: the folder holds no platforms",
            ));
        }
        folders
    } else {
        vec![(host_platform(), host.to_path_buf())]
    };
    let mut set = ClientSet {
        releases: std::collections::BTreeMap::new(),
        files: Vec::new(),
    };
    for (platform, folder) in folders {
        let mut files = std::collections::BTreeMap::new();
        for name in ["lys", "lys-containment-entry"] {
            let path = folder.join(name);
            let bytes = std::fs::read(&path).map_err(|error| {
                client_error(
                    &path,
                    format!("client_binary_missing: {platform} requires {name}: {error}"),
                )
            })?;
            if bytes.is_empty() {
                return Err(client_error(
                    &path,
                    format!("client_binary_empty: {platform} holds an empty {name}"),
                ));
            }
            files.insert(name.to_owned(), format!("{:x}", Sha256::digest(&bytes)));
            set.files
                .push((PathBuf::from(&platform).join(commit).join(name), bytes));
        }
        set.releases.insert(
            platform,
            ClientRelease {
                commit: commit.to_owned(),
                files,
            },
        );
    }
    Ok(set)
}

impl ClientSet {
    /// Place immutable releases, refusing any different bytes already at the same commit.
    ///
    /// # Errors
    /// Returns a named filesystem failure or an existing release digest mismatch.
    pub fn place(&self, layout: &Layout) -> IdentityResult<()> {
        for (relative, bytes) in &self.files {
            let target = layout.root.join("clients").join(relative);
            let parent = target
                .parent()
                .ok_or_else(|| client_error(&target, "client file has no parent"))?;
            std::fs::create_dir_all(parent)
                .map_err(|error| client_error(parent, error.to_string()))?;
            match std::fs::read(&target) {
                Ok(held) if held == *bytes => continue,
                Ok(_) => {
                    return Err(client_error(
                        &target,
                        "client_release_changed: the same platform and commit already hold different bytes",
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(client_error(&target, error.to_string())),
            }
            crate::identity::upgrade::swap::place_file(&target, bytes, false)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))
                    .map_err(|error| client_error(&target, error.to_string()))?;
            }
        }
        Ok(())
    }

    /// The manifest is replaced by the existing configuration transaction, so rollback restores its old set.
    pub fn manifest(
        &self,
        layout: &Layout,
    ) -> IdentityResult<crate::identity::upgrade::render::RenderedFile> {
        let bytes = serde_json::to_vec_pretty(&self.releases)
            .map_err(|error| client_error(&layout.root, error.to_string()))?;
        Ok(crate::identity::upgrade::render::RenderedFile {
            name: "clients.json",
            target: layout.root.join("clients.json"),
            bytes: zeroize::Zeroizing::new(bytes),
            private: false,
            compose: false,
        })
    }
}

/// Read a placed manifest. An old install has no clients until this installer supplies them.
pub fn client_releases(
    layout: &Layout,
) -> IdentityResult<std::collections::BTreeMap<String, ClientRelease>> {
    let path = layout.root.join("clients.json");
    match std::fs::read(&path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|error| client_error(&path, error.to_string()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(std::collections::BTreeMap::new())
        }
        Err(error) => Err(client_error(&path, error.to_string())),
    }
}

#[cfg(test)]
mod client_tests {
    use super::{Layout, prepare_clients};

    #[test]
    fn two_client_platforms_are_placed_with_their_pair_and_manifest_and_missing_binaries_are_named()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = tempfile::tempdir()?;
        let install = tempfile::tempdir()?;
        let platforms = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
        for platform in platforms {
            let folder = source.path().join(platform);
            std::fs::create_dir(&folder)?;
            for name in ["lys", "lys-containment-entry"] {
                std::fs::write(folder.join(name), format!("{platform}/{name}"))?;
            }
        }
        let set = prepare_clients(Some(source.path()), source.path(), "0123456789abcdef")?;
        let layout = Layout::at(install.path().to_path_buf());
        set.place(&layout)?;
        assert_eq!(set.releases.len(), 2);
        for platform in platforms {
            for name in ["lys", "lys-containment-entry"] {
                let path = layout
                    .root
                    .join("clients")
                    .join(platform)
                    .join("0123456789abcdef")
                    .join(name);
                assert_eq!(std::fs::read_to_string(path)?, format!("{platform}/{name}"));
                assert_eq!(set.releases[platform].files[name].len(), 64);
            }
        }
        let manifest = set.manifest(&layout)?;
        assert_eq!(manifest.name, "clients.json");
        let held: std::collections::BTreeMap<String, super::ClientRelease> =
            serde_json::from_slice(&manifest.bytes)?;
        assert_eq!(held, set.releases);
        std::fs::remove_file(
            source
                .path()
                .join(platforms[1])
                .join("lys-containment-entry"),
        )?;
        let refused = prepare_clients(Some(source.path()), source.path(), "0123456789abcdef")
            .err()
            .ok_or("missing binary accepted")?;
        let words = refused.to_string();
        assert!(words.contains("client_binary_missing"));
        assert!(words.contains("lys-containment-entry"));
        Ok(())
    }

    #[test]
    fn absent_clients_use_the_host_pair_and_never_replace_a_release_at_the_same_commit()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = tempfile::tempdir()?;
        let install = tempfile::tempdir()?;
        for name in ["lys", "lys-containment-entry"] {
            std::fs::write(source.path().join(name), name)?;
        }
        let first = prepare_clients(None, source.path(), "0123456789abcdef")?;
        let layout = Layout::at(install.path().to_path_buf());
        first.place(&layout)?;
        assert_eq!(first.releases.len(), 1);
        assert!(first.releases.contains_key(&super::host_platform()));
        std::fs::write(source.path().join("lys"), "changed executable")?;
        let changed = prepare_clients(None, source.path(), "0123456789abcdef")?;
        let error = changed
            .place(&layout)
            .err()
            .ok_or("immutable release overwritten")?;
        assert!(error.to_string().contains("client_release_changed"));
        Ok(())
    }
}
