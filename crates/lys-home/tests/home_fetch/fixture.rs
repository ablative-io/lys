use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use lys_home::Home;
use lys_home::record::tracked::tracked_set;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{BIN, SESSION, TEMPLATE, UUID, git_text, lys_home, sha256_of, text};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    render_status: Option<i32>,
    ship_status: Option<i32>,
    commit: String,
    files: BTreeMap<String, String>,
}

struct Cached {
    root: PathBuf,
    record: Record,
}

fn key() -> Result<String, Box<dyn Error>> {
    let mut digest = Sha256::new();
    digest.update(include_bytes!("fixture.rs"));
    digest.update(fs::read(SESSION)?);
    digest.update(fs::read(TEMPLATE)?);
    digest.update(git_text(&["--version"])?);
    for path in [std::env::current_exe()?, PathBuf::from(BIN)] {
        let mut executable = File::open(path)?;
        let mut buffer = [0; 8192];
        loop {
            let read = executable.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub(super) fn copy_tree(from: &Path, to: &Path, repository: bool) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        if !repository && entry.file_name().to_string_lossy().starts_with(".git") {
            continue;
        }
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target, true)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn hashes(home: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    tracked_set(&Home::read(home)?)?
        .into_iter()
        .map(|path| Ok((path.clone(), sha256_of(&home.join(path))?)))
        .collect()
}

fn shipped(
    dir: &Path,
    home: &Path,
    remote: &Path,
) -> Result<(String, Option<i32>), Box<dyn Error>> {
    let output = lys_home(
        dir,
        &["ship", "--home", text(home)?, "--remote", text(remote)?],
    )?;
    if !output.status.success() {
        return Err(format!(
            "shipping the fixture failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let value: Value = serde_json::from_slice(&output.stdout)?;
    let commit = value["report"]["commit"]
        .as_str()
        .ok_or("a commit")?
        .to_owned();
    Ok((commit, output.status.code()))
}

fn build(dir: &Path) -> Result<Record, Box<dyn Error>> {
    let root = dir.join("home");
    let home = Home::open(&root)?;
    fs::copy(SESSION, home.session_path("fixture")?)?;
    let out = dir.join("out");
    fs::create_dir(&out)?;
    let rendered = lys_home(
        dir,
        &[
            "render-launch",
            "--home",
            text(&root)?,
            "--session",
            "fixture",
            "--template",
            TEMPLATE,
            "--uuid",
            UUID,
            "--cwd",
            "/fixture",
            "--model",
            "claude-fixture",
            "--version",
            "2.1.283",
            "--out",
            text(&out)?,
        ],
    )?;
    if !rendered.status.success() {
        return Err(format!(
            "rendering the fixture failed ({}): {}",
            rendered.status,
            String::from_utf8_lossy(&rendered.stderr)
        )
        .into());
    }
    let (commit, ship_status) = shipped(dir, &root, &dir.join("remote.git"))?;
    Ok(Record {
        render_status: rendered.status.code(),
        ship_status,
        commit,
        files: hashes(&root)?,
    })
}

fn prepare() -> Result<Cached, Box<dyn Error>> {
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("home-fetch-cache");
    fs::create_dir_all(&cache)?;
    let key = key()?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(cache.join(format!("{key}.lock")))?;
    lock.lock()?;
    let ready = cache.join(key);
    if !ready.try_exists()? {
        let started = Instant::now();
        let stage = tempfile::Builder::new()
            .prefix("building-")
            .tempdir_in(&cache)?;
        let record = build(stage.path())?;
        fs::write(
            stage.path().join("fixture.json"),
            serde_json::to_vec(&record)?,
        )?;
        fs::rename(stage.path(), &ready)?;
        eprintln!("home_fetch fixture built: {:?}", started.elapsed());
    }
    let mut bytes = Vec::new();
    File::open(ready.join("fixture.json"))?
        .take(1_048_577)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err("the home fixture metadata exceeds its bound".into());
    }
    let record: Record = serde_json::from_slice(&bytes)?;
    if record.render_status != Some(0)
        || record.ship_status != Some(0)
        || record.files.is_empty()
        || hashes(&ready.join("home"))? != record.files
        || git_text(&[
            "--git-dir",
            text(&ready.join("remote.git"))?,
            "rev-parse",
            "refs/lys/home",
        ])? != record.commit
        || git_text(&[
            "-C",
            text(&ready.join("home"))?,
            "rev-parse",
            "refs/lys/home",
        ])? != record.commit
    {
        return Err("the cached home fixture does not match its recorded build".into());
    }
    Ok(Cached {
        root: ready,
        record,
    })
}

fn cached() -> Result<&'static Cached, Box<dyn Error>> {
    static FIXTURE: OnceLock<Result<Cached, String>> = OnceLock::new();
    FIXTURE
        .get_or_init(|| prepare().map_err(|error| format!("preparing home fixture: {error}")))
        .as_ref()
        .map_err(|error| error.as_str().into())
}

pub(super) fn home(dir: &Path, name: &str) -> Result<(PathBuf, Option<i32>), Box<dyn Error>> {
    let fixture = cached()?;
    let root = dir.join(name);
    copy_tree(&fixture.root.join("home"), &root, false)?;
    copy_tree(
        &fixture.root.join("out"),
        &dir.join(format!("{name}-out")),
        true,
    )?;
    Ok((root, fixture.record.render_status))
}

pub(super) fn ship(
    dir: &Path,
    home: &Path,
    remote: &Path,
) -> Result<(String, Option<i32>), Box<dyn Error>> {
    let fixture = cached()?;
    // Changed homes still exercise the real ship command. Only the unchanged
    // setup uses the already shipped repository, copied without shared files.
    if !remote.try_exists()?
        && !home.join(".git").try_exists()?
        && hashes(home)? == fixture.record.files
    {
        copy_tree(&fixture.root.join("home/.git"), &home.join(".git"), true)?;
        copy_tree(&fixture.root.join("remote.git"), remote, true)?;
        Ok((fixture.record.commit.clone(), fixture.record.ship_status))
    } else {
        shipped(dir, home, remote)
    }
}
