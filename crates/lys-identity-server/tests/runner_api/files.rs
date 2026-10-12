//! Read every artifact to check that credentials never reached storage.

use super::{Error, Found, Path};

/// Every file under `dir`, read whole.
pub(super) fn files(dir: &Path) -> Result<Vec<Found>, Box<dyn Error>> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_owned()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                found.push((path.clone(), std::fs::read(&path)?));
            }
        }
    }
    Ok(found)
}
