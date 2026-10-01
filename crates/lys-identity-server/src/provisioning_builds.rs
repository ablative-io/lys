//! Reviewed build metadata is indexed once and extended after confirmed writes.

use std::collections::{BTreeMap, BTreeSet};

use crate::harness_catalogue::{BuildSource, BuildView};

use super::{Profile, Version};

pub(super) type ReviewedBuilds = BTreeMap<String, BTreeSet<BuildView>>;
pub(super) type Entry = Option<(String, BuildView)>;

#[cfg(test)]
std::thread_local! {
    static INDEX_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn index_visits() -> usize {
    INDEX_VISITS.with(std::cell::Cell::get)
}

pub(super) fn entry(version: &Version) -> Entry {
    version.reviewed.as_ref()?;
    let harness = version.settings.harness.as_ref()?;
    Some((
        harness.description.rendering_contract.clone(),
        BuildView {
            name: harness.name.clone(),
            program: harness.program.clone(),
            package: harness.package.clone(),
            source: BuildSource::Profile,
        },
    ))
}

pub(super) fn insert(builds: &mut ReviewedBuilds, entry: Entry) {
    if let Some((contract, build)) = entry {
        builds.entry(contract).or_default().insert(build);
    }
}

pub(super) fn index(profiles: &[Profile]) -> ReviewedBuilds {
    let mut builds = BTreeMap::new();
    for version in profiles.iter().flat_map(|profile| &profile.versions) {
        #[cfg(test)]
        INDEX_VISITS.with(|visits| visits.set(visits.get() + 1));
        insert(&mut builds, entry(version));
    }
    builds
}
