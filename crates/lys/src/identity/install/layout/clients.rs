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
    /// The one commit every release shares.
    commit: String,
    files: Vec<ClientFile>,
}

/// One executable read from a platform's folder.
struct ClientFile {
    platform: String,
    name: &'static str,
    bytes: Vec<u8>,
}

/// What one attempt placed under `clients/`, kept on the upgrade's intent
/// record so a refused attempt removes exactly what it placed and nothing
/// that was there before it (DIRECTORY-115 R1).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacedClients {
    /// Files the attempt wrote, in the order it wrote them.
    pub files: Vec<PathBuf>,
    /// Directories the attempt made, parents first.
    pub directories: Vec<PathBuf>,
    /// The `clients/<platform>/<commit>` directories among them: the
    /// releases the attempt made, named when they are removed.
    pub releases: Vec<PathBuf>,
}

impl PlacedClients {
    /// Whether the attempt placed nothing: every file was already there with the same bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.directories.is_empty()
    }

    /// Removes every file and directory the attempt placed, deepest first,
    /// naming each release directory removed. A path already gone is passed over.
    ///
    /// # Errors
    /// Returns a named filesystem failure at the first path that could not be removed.
    pub fn remove(&self, say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
        for file in &self.files {
            let made = file
                .parent()
                .is_some_and(|parent| self.directories.iter().any(|dir| dir == parent));
            match std::fs::remove_file(file) {
                Ok(()) if made => {}
                Ok(()) => say(&format!("client file removed: {}", file.display())),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(client_error(file, error.to_string())),
            }
        }
        for directory in self.directories.iter().rev() {
            match std::fs::remove_dir(directory) {
                Ok(()) if self.releases.contains(directory) => {
                    say(&format!("client release removed: {}", directory.display()));
                }
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(client_error(directory, error.to_string())),
            }
        }
        Ok(())
    }

    /// Makes `dir` when it is absent and records that this attempt made it.
    fn make_dir(&mut self, dir: &std::path::Path) -> IdentityResult<()> {
        if dir.is_dir() {
            return Ok(());
        }
        std::fs::create_dir(dir).map_err(|error| client_error(dir, error.to_string()))?;
        self.directories.push(dir.to_path_buf());
        Ok(())
    }
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

fn client_commit<'a>(host: &std::path::Path, stamp: &'a str) -> IdentityResult<&'a str> {
    let (commit, suffix) = stamp.split_once(';').unwrap_or((stamp, ""));
    let refusal = if stamp == "not built from a git commit" {
        "client_commit_untracked"
    } else if commit.len() == 40
        && commit
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        match suffix {
            "" | " stated" => return Ok(commit),
            " dirty" | " stated; dirty" => "client_commit_dirty",
            _ => "client_commit_invalid",
        }
    } else {
        "client_commit_invalid"
    };
    Err(client_error(host, format!("{refusal}: {stamp}")))
}

/// The Rust target of the host client that was built beside this installer.
#[must_use]
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
/// Refuses an invalid platform, missing executable, unreadable file, or dirty, untracked or malformed stamp.
pub fn prepare_clients(
    source: Option<&std::path::Path>,
    host: &std::path::Path,
    commit: &str,
) -> IdentityResult<ClientSet> {
    use sha2::{Digest, Sha256};
    let commit = client_commit(host, commit)?;
    let folders = if let Some(source) = source {
        let mut folders = Vec::new();
        for entry in
            std::fs::read_dir(source).map_err(|error| client_error(source, error.to_string()))?
        {
            let entry = entry.map_err(|error| client_error(source, error.to_string()))?;
            let name = entry.file_name().into_string().map_err(|name| {
                client_error(
                    source,
                    format!(
                        "client_platform_invalid: a platform is not UTF-8: {}",
                        name.display()
                    ),
                )
            })?;
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
        commit: commit.to_owned(),
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
            set.files.push(ClientFile {
                platform: platform.clone(),
                name,
                bytes,
            });
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
    /// Place immutable releases, recording in `placed` every file and
    /// directory this attempt makes. A file already holding the same bytes is
    /// left as it is. One holding different bytes is refused by name when
    /// `clients.json` names its commit for its platform (the bytes are a
    /// served release), and replaced, with a note, when it does not: it was
    /// placed by an attempt that did not finish (DIRECTORY-115 R2).
    ///
    /// # Errors
    /// Returns a named filesystem failure or a served release's digest mismatch.
    pub fn place(
        &self,
        layout: &Layout,
        placed: &mut PlacedClients,
        say: &mut dyn FnMut(&str),
    ) -> IdentityResult<()> {
        let served = client_releases(layout)?;
        let root = layout.root.join("clients");
        for file in &self.files {
            let platform_dir = root.join(&file.platform);
            let release_dir = platform_dir.join(&self.commit);
            placed.make_dir(&root)?;
            placed.make_dir(&platform_dir)?;
            if !release_dir.is_dir() {
                placed.make_dir(&release_dir)?;
                placed.releases.push(release_dir.clone());
            }
            let target = release_dir.join(file.name);
            let is_served = served
                .get(&file.platform)
                .is_some_and(|release| release.commit == self.commit);
            match std::fs::read(&target) {
                Ok(held) if held == file.bytes => continue,
                Ok(_) if is_served => {
                    return Err(client_error(
                        &target,
                        "client_release_changed: the same platform and commit already hold different bytes",
                    ));
                }
                Ok(_) => say(&format!(
                    "client_release_replaced: {} was placed by an earlier attempt that did not finish",
                    target.display()
                )),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(client_error(&target, error.to_string())),
            }
            crate::identity::upgrade::swap::place_file(&target, &file.bytes, false)?;
            placed.files.push(target.clone());
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
    ///
    /// # Errors
    /// Returns named manifest, platform or client-release validation failures.
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
///
/// # Errors
/// Returns named manifest, platform or client-release validation failures.
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

/// Every `clients/<platform>/<commit>` directory under `layout` that
/// `clients.json` does not name for its platform, sorted: the pile an
/// upgrade reports and never prunes (DIRECTORY-115 R3). An install with no
/// `clients/` holds none.
///
/// # Errors
/// Returns a named manifest or filesystem failure.
pub fn unreferenced_releases(layout: &Layout) -> IdentityResult<Vec<PathBuf>> {
    let served = client_releases(layout)?;
    let root = layout.root.join("clients");
    let directories = |dir: &std::path::Path| -> IdentityResult<Vec<PathBuf>> {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(client_error(dir, error.to_string())),
        };
        let mut found = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| client_error(dir, error.to_string()))?;
            if entry
                .file_type()
                .map_err(|error| client_error(&entry.path(), error.to_string()))?
                .is_dir()
            {
                found.push(entry.path());
            }
        }
        found.sort();
        Ok(found)
    };
    let mut unreferenced = Vec::new();
    for platform_dir in directories(&root)? {
        let named = platform_dir
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|platform| served.get(platform))
            .map(|release| release.commit.as_str());
        for release_dir in directories(&platform_dir)? {
            let commit = release_dir.file_name().and_then(|name| name.to_str());
            if commit != named {
                unreferenced.push(release_dir);
            }
        }
    }
    Ok(unreferenced)
}

#[cfg(test)]
mod client_tests {
    use super::{ClientSet, Layout, PlacedClients, prepare_clients};

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
    const OLDER: &str = "89abcdef0123456789abcdef0123456789abcdef";
    const REPLACED: &str = "was placed by an earlier attempt that did not finish";

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    /// Places `set` silently, as a finished attempt leaves it.
    fn place(set: &ClientSet, layout: &Layout) -> TestResult<PlacedClients> {
        let mut placed = PlacedClients::default();
        set.place(layout, &mut placed, &mut |_| {})?;
        Ok(placed)
    }

    /// `clients.json` names `set`'s releases: they are served.
    fn served(set: &ClientSet, layout: &Layout) -> TestResult {
        let manifest = set.manifest(layout)?;
        crate::identity::upgrade::swap::place_file(&manifest.target, &manifest.bytes, false)?;
        Ok(())
    }

    fn host_file(layout: &Layout, commit: &str, name: &str) -> std::path::PathBuf {
        layout
            .root
            .join("clients")
            .join(super::host_platform())
            .join(commit)
            .join(name)
    }

    fn host_pair() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
        let folder = tempfile::tempdir()?;
        for name in ["lys", "lys-containment-entry"] {
            std::fs::write(folder.path().join(name), name)?;
        }
        Ok(folder)
    }

    #[test]
    fn a_stated_client_stamp_places_the_pair_under_its_leading_commit()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = host_pair()?;
        let install = tempfile::tempdir()?;
        let set = prepare_clients(None, source.path(), &format!("{COMMIT}; stated"))?;
        let layout = Layout::at(install.path().to_path_buf());
        let placed = place(&set, &layout)?;
        let platform = super::host_platform();
        assert_eq!(
            placed.releases,
            [layout.root.join("clients").join(&platform).join(COMMIT)]
        );
        assert_eq!(placed.files.len(), 2);
        assert_eq!(set.releases[&platform].commit, COMMIT);
        for name in ["lys", "lys-containment-entry"] {
            assert_eq!(
                std::fs::read_to_string(
                    layout
                        .root
                        .join("clients")
                        .join(&platform)
                        .join(COMMIT)
                        .join(name)
                )?,
                name
            );
        }
        Ok(())
    }

    #[test]
    fn dirty_client_stamps_are_refused_by_name_with_the_entire_stamp()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = host_pair()?;
        for stamp in [
            format!("{COMMIT}; dirty"),
            format!("{COMMIT}; stated; dirty"),
        ] {
            let error = prepare_clients(None, source.path(), &stamp)
                .err()
                .ok_or("dirty client stamp accepted")?;
            assert!(
                error
                    .to_string()
                    .contains(&format!("client_commit_dirty: {stamp}")),
                "{error}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_bare_client_commit_is_accepted_without_changing_its_manifest()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = host_pair()?;
        let set = prepare_clients(None, source.path(), COMMIT)?;
        assert_eq!(set.releases[&super::host_platform()].commit, COMMIT);
        Ok(())
    }

    #[test]
    fn an_untracked_client_stamp_is_refused_by_name_with_the_entire_stamp()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = host_pair()?;
        let stamp = "not built from a git commit";
        let error = prepare_clients(None, source.path(), stamp)
            .err()
            .ok_or("untracked client stamp accepted")?;
        assert!(
            error
                .to_string()
                .contains(&format!("client_commit_untracked: {stamp}")),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn malformed_client_stamps_name_the_entire_stamp_they_refuse()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = host_pair()?;
        for stamp in [
            "01234567".to_owned(),
            COMMIT.to_uppercase(),
            format!("{COMMIT}; unknown"),
        ] {
            let error = prepare_clients(None, source.path(), &stamp)
                .err()
                .ok_or("malformed client stamp accepted")?;
            assert!(
                error
                    .to_string()
                    .contains(&format!("client_commit_invalid: {stamp}")),
                "{error}"
            );
        }
        Ok(())
    }

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
        let set = prepare_clients(
            Some(source.path()),
            source.path(),
            "0123456789abcdef0123456789abcdef01234567",
        )?;
        let layout = Layout::at(install.path().to_path_buf());
        place(&set, &layout)?;
        assert_eq!(set.releases.len(), 2);
        for platform in platforms {
            for name in ["lys", "lys-containment-entry"] {
                let path = layout
                    .root
                    .join("clients")
                    .join(platform)
                    .join("0123456789abcdef0123456789abcdef01234567")
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
        let refused = prepare_clients(
            Some(source.path()),
            source.path(),
            "0123456789abcdef0123456789abcdef01234567",
        )
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
        let first = prepare_clients(
            None,
            source.path(),
            "0123456789abcdef0123456789abcdef01234567",
        )?;
        let layout = Layout::at(install.path().to_path_buf());
        place(&first, &layout)?;
        served(&first, &layout)?;
        assert_eq!(first.releases.len(), 1);
        assert!(first.releases.contains_key(&super::host_platform()));
        std::fs::write(source.path().join("lys"), "changed executable")?;
        let changed = prepare_clients(
            None,
            source.path(),
            "0123456789abcdef0123456789abcdef01234567",
        )?;
        let error = place(&changed, &layout)
            .err()
            .ok_or("immutable release overwritten")?;
        assert!(error.to_string().contains("client_release_changed"));
        Ok(())
    }

    #[test]
    fn a_stranded_client_release_the_manifest_does_not_name_is_replaced_and_noted() -> TestResult {
        let source = host_pair()?;
        let install = tempfile::tempdir()?;
        let layout = Layout::at(install.path().to_path_buf());
        let stranded = prepare_clients(None, source.path(), COMMIT)?;
        place(&stranded, &layout)?;
        // The manifest names an older commit: the bytes at COMMIT were merely placed.
        let older = prepare_clients(None, source.path(), OLDER)?;
        place(&older, &layout)?;
        served(&older, &layout)?;
        std::fs::write(source.path().join("lys"), "rebuilt executable")?;
        let rebuilt = prepare_clients(None, source.path(), COMMIT)?;
        let mut said = Vec::new();
        let mut placed = PlacedClients::default();
        rebuilt.place(&layout, &mut placed, &mut |line| said.push(line.to_owned()))?;
        let target = host_file(&layout, COMMIT, "lys");
        assert_eq!(std::fs::read_to_string(&target)?, "rebuilt executable");
        assert_eq!(
            said,
            [format!(
                "client_release_replaced: {} {REPLACED}",
                target.display()
            )]
        );
        assert_eq!(placed.files, [target]);
        assert!(placed.directories.is_empty() && placed.releases.is_empty());
        assert_eq!(
            std::fs::read_to_string(host_file(&layout, OLDER, "lys"))?,
            "lys",
            "the served release is untouched"
        );
        Ok(())
    }

    #[test]
    fn a_served_client_release_with_different_bytes_is_refused_by_name() -> TestResult {
        let source = host_pair()?;
        let install = tempfile::tempdir()?;
        let layout = Layout::at(install.path().to_path_buf());
        let served_set = prepare_clients(None, source.path(), COMMIT)?;
        place(&served_set, &layout)?;
        served(&served_set, &layout)?;
        std::fs::write(source.path().join("lys-containment-entry"), "rebuilt entry")?;
        let rebuilt = prepare_clients(None, source.path(), COMMIT)?;
        let mut said = Vec::new();
        let mut placed = PlacedClients::default();
        let error = rebuilt
            .place(&layout, &mut placed, &mut |line| said.push(line.to_owned()))
            .err()
            .ok_or("a served release was replaced")?;
        assert!(
            error.to_string().contains("client_release_changed"),
            "{error}"
        );
        assert!(said.is_empty(), "{said:?}");
        assert!(placed.is_empty(), "{placed:?}");
        assert_eq!(
            std::fs::read_to_string(host_file(&layout, COMMIT, "lys-containment-entry"))?,
            "lys-containment-entry"
        );
        Ok(())
    }

    #[test]
    fn a_client_release_with_the_same_bytes_is_a_no_op_served_or_not() -> TestResult {
        let source = host_pair()?;
        let install = tempfile::tempdir()?;
        let layout = Layout::at(install.path().to_path_buf());
        let first = prepare_clients(None, source.path(), COMMIT)?;
        place(&first, &layout)?;
        let again = prepare_clients(None, source.path(), COMMIT)?;
        let mut said = Vec::new();
        let mut placed = PlacedClients::default();
        again.place(&layout, &mut placed, &mut |line| said.push(line.to_owned()))?;
        assert!(placed.is_empty() && said.is_empty(), "{placed:?} {said:?}");
        served(&first, &layout)?;
        again.place(&layout, &mut placed, &mut |line| said.push(line.to_owned()))?;
        assert!(placed.is_empty() && said.is_empty(), "{placed:?} {said:?}");
        Ok(())
    }
}
