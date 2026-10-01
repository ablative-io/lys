//! Derived positions preserve recording order without copying retained records.

use std::collections::BTreeMap;

use super::{Kept, Review, Version, builds, edits, unavailable};
use crate::error::ServerError;

pub(super) type Position = (usize, usize);

#[derive(Default, Clone)]
pub(super) struct Indexes {
    pub profiles: BTreeMap<String, usize>,
    pub operations: BTreeMap<String, Position>,
    pub versions: BTreeMap<usize, BTreeMap<u32, usize>>,
    pub latest_reviewed: BTreeMap<usize, usize>,
    pub servers: BTreeMap<String, (usize, usize, usize)>,
    pub skills: BTreeMap<String, BTreeMap<String, usize>>,
    pub latest_skills: BTreeMap<String, usize>,
    pub reviewed_builds: builds::ReviewedBuilds,
    reviews: BTreeMap<String, Vec<Position>>,
}

impl Indexes {
    pub fn rebuild(kept: &Kept) -> Self {
        let mut indexes = Self::default();
        for (profile_index, profile) in kept.profiles.iter().enumerate() {
            indexes
                .profiles
                .entry(profile.agent.clone())
                .or_insert(profile_index);
            for (version_index, version) in profile.versions.iter().enumerate() {
                #[cfg(test)]
                {
                    builds::visit();
                    super::HISTORY_VISITS.with(|visits| visits.set(visits.get() + 1));
                }
                indexes.version((profile_index, version_index), version);
            }
        }
        for (position, skill) in kept.skills.iter().enumerate() {
            indexes.skill(position, &skill.name, &skill.sha256);
        }
        indexes
    }

    fn version(&mut self, position: Position, version: &Version) {
        let (profile_index, version_index) = position;
        self.operations
            .entry(version.operation.clone())
            .or_insert(position);
        self.versions
            .entry(profile_index)
            .or_default()
            .entry(version.number)
            .or_insert(version_index);
        if let Some(review) = &version.reviewed {
            let uses = self.reviews.entry(review.operation.clone()).or_default();
            if !uses.contains(&position) {
                uses.push(position);
            }
            self.latest_reviewed
                .entry(profile_index)
                .and_modify(|kept| *kept = (*kept).max(version_index))
                .or_insert(version_index);
            for (server_index, server) in version.settings.mcp_servers.iter().enumerate() {
                let declared = (profile_index, version_index, server_index);
                self.servers
                    .entry(server.name.clone())
                    .and_modify(|kept| *kept = (*kept).min(declared))
                    .or_insert(declared);
            }
        }
        builds::insert(&mut self.reviewed_builds, builds::entry(version));
    }

    fn skill(&mut self, position: usize, name: &str, digest: &str) {
        self.skills
            .entry(name.to_owned())
            .or_default()
            .entry(digest.to_owned())
            .or_insert(position);
        self.latest_skills.insert(name.to_owned(), position);
    }

    pub fn location(&self, agent: &str, number: u32) -> Option<Position> {
        let profile = *self.profiles.get(agent)?;
        Some((profile, *self.versions.get(&profile)?.get(&number)?))
    }

    pub fn review_reused(
        &self,
        kept: &Kept,
        agent: &str,
        number: u32,
        review: &Review,
    ) -> Result<bool, ServerError> {
        if self.operations.contains_key(&review.operation) {
            return Ok(true);
        }
        for &(profile, version) in self.reviews.get(&review.operation).into_iter().flatten() {
            let profile = kept
                .profiles
                .get(profile)
                .ok_or_else(|| unavailable("review operation index names no profile"))?;
            let version = profile
                .versions
                .get(version)
                .ok_or_else(|| unavailable("review operation index names no version"))?;
            let kept_review = version
                .reviewed
                .as_ref()
                .ok_or_else(|| unavailable("review operation index names no review"))?;
            if profile.agent != agent || version.number != number || kept_review.by != review.by {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn update(&mut self, kept: &Kept, change: edits::Changed) -> Result<(), ServerError> {
        match change {
            edits::Changed::Version(position) => {
                let profile = kept
                    .profiles
                    .get(position.0)
                    .ok_or_else(|| unavailable("profile index is outside the retained records"))?;
                let version = profile
                    .versions
                    .get(position.1)
                    .ok_or_else(|| unavailable("version index is outside the retained profile"))?;
                self.profiles
                    .entry(profile.agent.clone())
                    .or_insert(position.0);
                self.version(position, version);
            }
            edits::Changed::Skill(position) => {
                let skill = kept
                    .skills
                    .get(position)
                    .ok_or_else(|| unavailable("skill index is outside the retained records"))?;
                self.skill(position, &skill.name, &skill.sha256);
            }
        }
        Ok(())
    }
}
