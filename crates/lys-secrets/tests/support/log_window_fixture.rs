use std::error::Error;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use lys_secrets::{Broker, LocalGrants};
use sha2::{Digest, Sha256};

use super::{AT_MS, Folders, LINES, OUTCOME, build, copy_tree, paths_in};

// The executable includes the linked storage implementation; the source and
// line parameters also bind the cache to the fixture's definition.
fn key() -> Result<String, Box<dyn Error>> {
    let mut digest = Sha256::new();
    digest.update(include_bytes!("../log_window.rs"));
    digest.update(include_bytes!("log_window_fixture.rs"));
    digest.update(LINES.to_le_bytes());
    digest.update(AT_MS.to_le_bytes());
    digest.update(OUTCOME.as_bytes());
    let mut executable = File::open(std::env::current_exe()?)?;
    let mut buffer = [0; 8192];
    loop {
        let read = executable.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn prepare() -> Result<PathBuf, Box<dyn Error>> {
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("log-window-cache");
    fs::create_dir_all(&cache)?;
    let key = key()?;
    let locked = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(cache.join(format!("{key}.lock")))?;
    let waiting = Instant::now();
    rustix::fs::fcntl_lock(&locked, rustix::fs::FlockOperation::LockExclusive)?;
    let waited = waiting.elapsed();
    let started = Instant::now();
    let ready = cache.join(key);
    if ready.try_exists()? {
        let paths = paths_in(&ready.join("broker"), &ready.join("keys"));
        let broker = Broker::open(&paths, LocalGrants::new(), Box::new(|| AT_MS))?;
        if broker.audit().len() != LINES {
            return Err("the cached fixture does not hold the required line count".into());
        }
        eprintln!(
            "log_window fixture reused: {:?}; cache wait: {waited:?}",
            started.elapsed()
        );
    } else {
        let folders = Folders {
            dir: tempfile::Builder::new()
                .prefix("building-")
                .tempdir_in(&cache)?,
        };
        build(&folders)?;
        // Readers see the whole closed fixture or no fixture. A failed build
        // never publishes its temporary directory.
        fs::rename(folders.dir.path(), &ready)?;
        eprintln!(
            "log_window fixture built: {:?}; cache wait: {waited:?}",
            started.elapsed()
        );
    }
    Ok(ready)
}

fn built() -> Result<&'static Path, Box<dyn Error>> {
    static BUILT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    BUILT
        .get_or_init(|| prepare().map_err(|error| format!("preparing log fixture: {error}")))
        .as_ref()
        .map(PathBuf::as_path)
        .map_err(|error| error.as_str().into())
}

pub(super) fn copy() -> Result<Folders, Box<dyn Error>> {
    let built = built()?;
    let started = Instant::now();
    let copy = Folders {
        dir: tempfile::tempdir()?,
    };
    copy_tree(&built.join("broker"), &copy.root())?;
    copy_tree(&built.join("keys"), &copy.keys())?;
    eprintln!("log_window fixture copied: {:?}", started.elapsed());
    Ok(copy)
}
