//! Copy private fixture files and validate every cached byte before reuse.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use ring::digest::{Context, SHA256};

pub(crate) fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Context::new(&SHA256);
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    let digest = hash.finish();
    let mut encoded = String::with_capacity(64);
    for byte in digest.as_ref() {
        std::fmt::Write::write_fmt(&mut encoded, format_args!("{byte:02x}"))
            .map_err(io::Error::other)?;
    }
    Ok(encoded)
}

pub(crate) fn inventory(root: &Path) -> io::Result<BTreeMap<String, String>> {
    let mut found = BTreeMap::new();
    visit(root, root, &mut found)?;
    Ok(found)
}

fn visit(root: &Path, dir: &Path, found: &mut BTreeMap<String, String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            visit(root, &path, found)?;
        } else if kind.is_file() {
            let relative = path.strip_prefix(root).map_err(io::Error::other)?;
            let name = relative
                .to_str()
                .ok_or_else(|| io::Error::other("non-UTF8 template path"))?;
            found.insert(name.to_owned(), hash_file(&path)?);
        } else {
            return Err(io::Error::other(format!(
                "template contains a non-regular entry: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

pub(crate) fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            copy_file(&entry.path(), &target)?;
        } else {
            return Err(io::Error::other(
                "template copy refused a non-regular entry",
            ));
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn copy_file(from: &Path, to: &Path) -> io::Result<()> {
    use rustix::fs::{CloneFlags, fclonefileat};
    let source = File::open(from)?;
    let parent = to
        .parent()
        .ok_or_else(|| io::Error::other("copy destination has no parent"))?;
    let name = to
        .file_name()
        .ok_or_else(|| io::Error::other("copy destination has no name"))?;
    match fclonefileat(&source, File::open(parent)?, name, CloneFlags::empty()) {
        Ok(()) => Ok(()),
        Err(error) if matches!(error, rustix::io::Errno::NOTSUP | rustix::io::Errno::XDEV) => {
            eprintln!("service template: recursive-copy fallback ({error})");
            fs::copy(from, to)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(not(target_os = "macos"))]
fn copy_file(from: &Path, to: &Path) -> io::Result<()> {
    fs::copy(from, to)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_hash_keeps_exact_sha256_across_read_boundaries() -> io::Result<()> {
        use sha2::{Digest, Sha256};

        let dir = tempfile::tempdir()?;
        let path = dir.path().join("bytes");
        for (bytes, expected) in [
            (
                &b""[..],
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                &b"abc"[..],
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
        ] {
            fs::write(&path, bytes)?;
            assert_eq!(hash_file(&path)?, expected);
        }
        let mut bytes = vec![13; 8192 * 2 + 1];
        fs::write(&path, &bytes)?;
        assert_eq!(hash_file(&path)?, format!("{:x}", Sha256::digest(&bytes)));
        bytes[8192 * 2] = 17;
        fs::write(&path, &bytes)?;
        assert_eq!(hash_file(&path)?, format!("{:x}", Sha256::digest(&bytes)));
        Ok(())
    }

    #[test]
    fn private_copies_do_not_mutate_the_template_or_each_other() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let source = dir.path().join("source");
        fs::create_dir(&source)?;
        fs::write(source.join("snapshot.bin"), b"signed bytes")?;
        let before = inventory(&source)?;
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        copy_tree(&source, &first)?;
        copy_tree(&source, &second)?;
        fs::write(first.join("snapshot.bin"), b"private change")?;
        assert_eq!(inventory(&source)?, before);
        assert_eq!(inventory(&second)?, before);
        assert_ne!(inventory(&first)?, before);
        Ok(())
    }

    #[test]
    fn inventory_refuses_symlinks() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        std::os::unix::fs::symlink("missing", dir.path().join("link"))?;
        assert!(inventory(dir.path()).is_err());
        Ok(())
    }
}
