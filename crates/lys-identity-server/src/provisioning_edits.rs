//! A proposed edit borrows unchanged records until its replacement is durable.

use serde::ser::{Serialize, SerializeSeq, Serializer};

use super::{Kept, Profile, Review, Settings, SkillText, Version, unavailable};
use crate::error::ServerError;

pub(super) enum Edit {
    Set {
        profile: usize,
        agent: String,
        version: Box<Version>,
    },
    Review {
        profile: usize,
        version: usize,
        review: Review,
    },
    Skill(SkillText),
}

pub(super) enum Changed {
    Version((usize, usize)),
    Skill(usize),
}

#[derive(serde::Serialize)]
struct Proposed<'a> {
    profiles: Profiles<'a>,
    #[serde(skip_serializing_if = "Skills::is_empty")]
    skills: Skills<'a>,
}

struct Profiles<'a> {
    kept: &'a [Profile],
    edit: &'a Edit,
}

#[derive(serde::Serialize)]
struct ProfileView<'a> {
    agent: &'a str,
    versions: Versions<'a>,
}

struct Versions<'a> {
    kept: &'a [Version],
    edit: &'a Edit,
}

#[derive(serde::Serialize)]
struct Reviewed<'a> {
    number: u32,
    operation: &'a str,
    settings: &'a Settings,
    set_by: &'a str,
    set_at: u64,
    reviewed: &'a Review,
}

struct Skills<'a> {
    kept: &'a [SkillText],
    edit: &'a Edit,
}

impl Serialize for Profiles<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let added = matches!(self.edit, Edit::Set { profile, .. } if *profile == self.kept.len());
        let mut sequence = serializer.serialize_seq(Some(self.kept.len() + usize::from(added)))?;
        for (position, profile) in self.kept.iter().enumerate() {
            let changed = match self.edit {
                Edit::Set { profile, .. } | Edit::Review { profile, .. } => *profile == position,
                Edit::Skill(_) => false,
            };
            if changed {
                sequence.serialize_element(&ProfileView {
                    agent: &profile.agent,
                    versions: Versions {
                        kept: &profile.versions,
                        edit: self.edit,
                    },
                })?;
            } else {
                sequence.serialize_element(profile)?;
            }
        }
        if let Edit::Set { profile, agent, .. } = self.edit {
            if *profile == self.kept.len() {
                sequence.serialize_element(&ProfileView {
                    agent,
                    versions: Versions {
                        kept: &[],
                        edit: self.edit,
                    },
                })?;
            }
        }
        sequence.end()
    }
}

impl Serialize for Versions<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let added = matches!(self.edit, Edit::Set { .. });
        let mut sequence = serializer.serialize_seq(Some(self.kept.len() + usize::from(added)))?;
        for (position, version) in self.kept.iter().enumerate() {
            match self.edit {
                Edit::Review {
                    version: target,
                    review,
                    ..
                } if *target == position => sequence.serialize_element(&Reviewed {
                    number: version.number,
                    operation: &version.operation,
                    settings: &version.settings,
                    set_by: &version.set_by,
                    set_at: version.set_at,
                    reviewed: review,
                })?,
                _ => sequence.serialize_element(version)?,
            }
        }
        if let Edit::Set { version, .. } = self.edit {
            sequence.serialize_element(version)?;
        }
        sequence.end()
    }
}

impl Skills<'_> {
    fn is_empty(&self) -> bool {
        self.kept.is_empty() && !matches!(self.edit, Edit::Skill(_))
    }
}

impl Serialize for Skills<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let added = matches!(self.edit, Edit::Skill(_));
        let mut sequence = serializer.serialize_seq(Some(self.kept.len() + usize::from(added)))?;
        for skill in self.kept {
            sequence.serialize_element(skill)?;
        }
        if let Edit::Skill(skill) = self.edit {
            sequence.serialize_element(skill)?;
        }
        sequence.end()
    }
}

pub(super) fn encode(kept: &Kept, edit: &Edit) -> Result<Vec<u8>, ServerError> {
    serde_json::to_vec_pretty(&Proposed {
        profiles: Profiles {
            kept: &kept.profiles,
            edit,
        },
        skills: Skills {
            kept: &kept.skills,
            edit,
        },
    })
    .map_err(unavailable)
}

pub(super) fn apply(kept: &mut Kept, edit: Edit) -> Result<Changed, ServerError> {
    match edit {
        Edit::Set {
            profile,
            agent,
            version,
        } => {
            let version = *version;
            if profile == kept.profiles.len() {
                kept.profiles.push(Profile {
                    agent,
                    versions: vec![version],
                });
                Ok(Changed::Version((profile, 0)))
            } else {
                let target = kept.profiles.get_mut(profile).ok_or_else(|| {
                    unavailable("proposed profile index is outside the retained records")
                })?;
                let position = target.versions.len();
                target.versions.push(version);
                Ok(Changed::Version((profile, position)))
            }
        }
        Edit::Review {
            profile,
            version,
            review,
        } => {
            let target = kept
                .profiles
                .get_mut(profile)
                .and_then(|profile| profile.versions.get_mut(version))
                .ok_or_else(|| {
                    unavailable("proposed review index is outside the retained profile")
                })?;
            target.reviewed = Some(review);
            Ok(Changed::Version((profile, version)))
        }
        Edit::Skill(skill) => {
            let position = kept.skills.len();
            kept.skills.push(skill);
            Ok(Changed::Skill(position))
        }
    }
}
