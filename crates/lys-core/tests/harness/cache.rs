use std::fs::{self, File};
use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};

fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update(bytes.len().to_le_bytes());
    hash.update(bytes);
}

fn sources(hash: &mut Sha256, root: &Path, dir: &Path) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("cannot list Go scaffold sources")
        .map(|entry| entry.expect("cannot read Go scaffold directory entry"))
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let kind = entry
            .file_type()
            .expect("cannot inspect Go scaffold source");
        assert!(
            !kind.is_symlink(),
            "Go scaffold source must not be a symlink"
        );
        if kind.is_dir() {
            sources(hash, root, &path);
        } else {
            assert!(kind.is_file(), "Go scaffold source must be a regular file");
            field(
                hash,
                path.strip_prefix(root)
                    .expect("Go source is outside its scaffold")
                    .as_os_str()
                    .as_encoded_bytes(),
            );
            field(
                hash,
                &fs::read(path).expect("cannot read Go scaffold source"),
            );
        }
    }
}

fn source_hash(scaffold: &Path) -> Vec<u8> {
    let mut hash = Sha256::new();
    sources(&mut hash, scaffold, scaffold);
    hash.finalize().to_vec()
}

fn executable_hash(path: &Path) -> Vec<u8> {
    Sha256::digest(fs::read(path).expect("cannot read cached Go executable")).to_vec()
}

fn environment(go: &Path, scaffold: &Path, gocache: &Path) -> Vec<u8> {
    let result = Command::new(go)
        .arg("env")
        .args([
            "GOVERSION",
            "GOROOT",
            "GOOS",
            "GOARCH",
            "GOAMD64",
            "GOARM",
            "GOARM64",
            "GOMIPS",
            "GOMIPS64",
            "GOPPC64",
            "GORISCV64",
            "GOWASM",
            "GOEXPERIMENT",
            "CGO_ENABLED",
            "CC",
            "CXX",
            "CGO_CFLAGS",
            "CGO_CPPFLAGS",
            "CGO_CXXFLAGS",
            "CGO_LDFLAGS",
            "GOFIPS140",
            "GOWORK",
        ])
        .current_dir(scaffold)
        .env("GOFLAGS", "-mod=vendor")
        .env("GOPROXY", "off")
        .env("GOTOOLCHAIN", "local")
        .env("GOWORK", "off")
        .env("GOCACHE", gocache)
        .output()
        .expect("cannot inspect Go toolchain for the conformance cache");
    assert!(
        result.status.success(),
        "Go toolchain environment failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    result.stdout
}

/// Reuses a source-addressed executable without sharing a test's mutable files.
///
/// # Panics
///
/// Fails on a broken toolchain, changed build inputs, corrupt cache or any I/O
/// error. The lock covers publication and copying, so readers see a full build.
pub fn build(go: &Path, scaffold: &Path, out: &Path, compile: impl FnOnce(&Path, &Path)) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("lys-go-conformance-v1");
    let gocache = root.join("gocache");
    fs::create_dir_all(&gocache).expect("cannot create shared Go compiler cache");
    let environment = environment(go, scaffold, &gocache);
    let source = source_hash(scaffold);
    let mut hash = Sha256::new();
    field(&mut hash, b"offline-vendor-build-v1");
    field(&mut hash, &source);
    field(&mut hash, go.as_os_str().as_encoded_bytes());
    field(&mut hash, &environment);
    let key = format!("{:x}", hash.finalize());
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join(format!("{key}.lock")))
        .expect("cannot open Go executable cache lock");
    lock.lock().expect("cannot lock Go executable cache");
    let entry = root.join(key);
    if !entry
        .try_exists()
        .expect("cannot inspect Go executable cache")
    {
        let staging = tempfile::tempdir_in(&root).expect("cannot stage Go executable cache");
        let executable = staging.path().join("tool");
        compile(&executable, &gocache);
        assert_eq!(
            source_hash(scaffold),
            source,
            "Go scaffold source changed while its executable was built"
        );
        fs::write(staging.path().join("sha256"), executable_hash(&executable))
            .expect("cannot write Go executable cache checksum");
        fs::rename(staging.path(), &entry).expect("cannot publish Go executable cache");
    }
    let executable = entry.join("tool");
    assert_eq!(
        executable_hash(&executable),
        fs::read(entry.join("sha256")).expect("cannot read Go executable cache checksum"),
        "cached Go executable checksum does not match"
    );
    assert_eq!(
        source_hash(scaffold),
        source,
        "Go scaffold source changed before using its cached executable"
    );
    fs::copy(executable, out).expect("cannot copy cached Go executable into the test");
}
