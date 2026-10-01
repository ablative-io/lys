use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use lys_secrets::{BrokerPaths, EntryClass, SecretStore, StoreKey};
use sha2::{Digest, Sha256};

use super::{Failure, START_MS, build, seals};

const MANIFEST: &str = "fixture-hashes.json";

fn file_hash(path: &Path) -> Result<String, Failure> {
    let mut digest = Sha256::new();
    let mut file = File::open(path)?;
    let mut buffer = [0; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn key() -> Result<String, Failure> {
    let mut digest = Sha256::new();
    digest.update(include_bytes!("fixture.rs"));
    digest.update(include_bytes!("world_fixture.rs"));
    digest.update(START_MS.to_le_bytes());
    digest.update(file_hash(&std::env::current_exe()?)?.as_bytes());
    Ok(format!("{:x}", digest.finalize()))
}

fn hashes(root: &Path, dir: &Path, found: &mut BTreeMap<PathBuf, String>) -> Result<(), Failure> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            hashes(root, &path, found)?;
        } else if kind.is_file() {
            let relative = path.strip_prefix(root)?;
            if relative != Path::new(MANIFEST) {
                found.insert(relative.to_owned(), file_hash(&path)?);
            }
        } else {
            return Err("secrets_fixture_invalid: unexpected file type".into());
        }
    }
    Ok(())
}

fn manifest(dir: &Path) -> Result<BTreeMap<PathBuf, String>, Failure> {
    let file = File::open(dir.join(MANIFEST))?;
    if file.metadata()?.len() > 65_536 {
        return Err("secrets_fixture_invalid: checksum manifest is too large".into());
    }
    Ok(serde_json::from_reader(file)?)
}

fn check_hashes(dir: &Path, expected: &BTreeMap<PathBuf, String>) -> Result<(), Failure> {
    let mut actual = BTreeMap::new();
    hashes(dir, dir, &mut actual)?;
    if &actual != expected {
        return Err("secrets_fixture_invalid: template bytes changed".into());
    }
    Ok(())
}

fn prepare() -> Result<PathBuf, Failure> {
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("secrets-world-cache");
    fs::create_dir_all(&cache)?;
    let key = key()?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(cache.join(format!("{key}.lock")))?;
    rustix::fs::fcntl_lock(&lock, rustix::fs::FlockOperation::LockExclusive)?;
    let ready = cache.join(key);
    if ready.try_exists()? {
        check_hashes(&ready, &manifest(&ready)?)?;
    } else {
        let temporary = tempfile::tempdir_in(&cache)?;
        build(temporary.path())?;
        let mut manifest = BTreeMap::new();
        hashes(temporary.path(), temporary.path(), &mut manifest)?;
        fs::write(
            temporary.path().join(MANIFEST),
            serde_json::to_vec(&manifest)?,
        )?;
        fs::rename(temporary.path(), &ready)?;
    }
    Ok(ready)
}

fn template() -> Result<&'static Path, Failure> {
    static TEMPLATE: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    TEMPLATE
        .get_or_init(|| prepare().map_err(|error| format!("secrets_fixture_prepare: {error}")))
        .as_ref()
        .map(PathBuf::as_path)
        .map_err(|error| error.as_str().into())
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), Failure> {
    fs::create_dir(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &destination)?;
        } else {
            return Err("secrets_fixture_invalid: unexpected file type".into());
        }
    }
    fs::set_permissions(target, fs::metadata(source)?.permissions())?;
    Ok(())
}

pub(super) fn copy(dir: &Path) -> Result<(), Failure> {
    let template = template()?;
    for name in ["keys", "store", "log"] {
        copy_tree(&template.join(name), &dir.join(name))?;
    }
    check_hashes(dir, &manifest(template)?)?;
    Ok(())
}

pub(super) fn validate(paths: &BrokerPaths) -> Result<(), Failure> {
    let key = StoreKey::load(&paths.store_key, &[&paths.store_dir, &paths.log_dir])?;
    let store = SecretStore::open(&paths.store_dir, &key)?;
    if store.entries().count() != 6 {
        return Err("secrets_fixture_invalid: expected six sealed entries".into());
    }
    for (name, owner, scope) in seals() {
        let entry = store
            .entry(name)
            .ok_or("secrets_fixture_invalid: a sealed entry is missing")?;
        if entry.owner != owner || entry.class != EntryClass::Credential {
            return Err(
                "secrets_fixture_invalid: a sealed entry has another owner or class".into(),
            );
        }
        if store.scope(name) != Some(scope) {
            return Err("secrets_fixture_invalid: a sealed entry has another scope".into());
        }
    }
    Ok(())
}
