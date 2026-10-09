//! Reporting indexes change with their signed events, so reads need no ancestry scan.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use super::{
    AgentId, IdentityError, IdentityId, LifecycleState, PersonId, Projection, Record, ReportingGap,
    unknown,
};

impl Projection {
    /// Resolve a new reporting edge, naming each invalid target or chain.
    pub fn resolve_reporting(
        &self,
        target: IdentityId,
        moving: Option<AgentId>,
    ) -> Result<PersonId, IdentityError> {
        let mut seen = HashSet::new();
        let mut chain = Vec::new();
        if let Some(agent) = moving {
            seen.insert(IdentityId::Agent(agent));
            chain.push(agent.to_string());
        }
        let mut next = target;
        let mut first = true;
        let mut agents = 0;
        loop {
            chain.push(next.to_string());
            if !seen.insert(next) {
                return Err(IdentityError::AnswersToCycle { chain });
            }
            let Some(record) = self.record(next) else {
                return Err(if first {
                    IdentityError::AnswersToUnknown {
                        identity: next.to_string(),
                    }
                } else {
                    IdentityError::NoAccountablePerson { chain }
                });
            };
            if record.state != LifecycleState::Active {
                return Err(if first {
                    IdentityError::AnswersToInactive {
                        identity: next.to_string(),
                        state: record.state,
                    }
                } else {
                    IdentityError::NoAccountablePerson { chain }
                });
            }
            match next {
                IdentityId::Person(person) => return Ok(person),
                IdentityId::Agent(_) => {
                    agents += 1;
                    if agents > self.agent_count {
                        return Err(IdentityError::AnswersToCycle { chain });
                    }
                    next = record
                        .reports_to
                        .ok_or_else(|| IdentityError::NoAccountablePerson {
                            chain: chain.clone(),
                        })?;
                }
                IdentityId::ServiceAccount(_)
                | IdentityId::Connector(_)
                | IdentityId::Machine(_) => {
                    return Err(IdentityError::AnswersToUnknown {
                        identity: next.to_string(),
                    });
                }
            }
            first = false;
        }
    }

    pub(super) fn check_reporting_change(
        &self,
        identity: IdentityId,
        from: IdentityId,
        to: IdentityId,
        responsible_from: PersonId,
        responsible_to: PersonId,
    ) -> Result<(), IdentityError> {
        let record = self.record(identity).ok_or_else(|| unknown(identity))?;
        let agent = match identity {
            IdentityId::Agent(agent) => agent,
            IdentityId::Person(_)
            | IdentityId::ServiceAccount(_)
            | IdentityId::Connector(_)
            | IdentityId::Machine(_) => {
                return Err(IdentityError::ChangeMismatch {
                    reason: "a reporting change names an agent",
                });
            }
        };
        if record.reports_to != Some(from) || record.responsible != Some(responsible_from) {
            return Err(IdentityError::ChangeMismatch {
                reason: "the reporting change does not name the current edge and accountable person",
            });
        }
        if self.resolve_reporting(to, Some(agent))? != responsible_to {
            return Err(IdentityError::ChangeMismatch {
                reason: "the new accountable person differs from the reporting chain",
            });
        }
        Ok(())
    }

    fn target_accountability(
        &self,
        target: IdentityId,
    ) -> Result<(PersonId, Option<ReportingGap>), IdentityError> {
        let record = self.record(target).ok_or_else(|| unknown(target))?;
        let person = match target {
            IdentityId::Person(person) => person,
            IdentityId::Agent(_) => record.responsible.ok_or(IdentityError::ChangeMismatch {
                reason: "a reporting agent has no accountable person",
            })?,
            IdentityId::ServiceAccount(_) => {
                return Err(IdentityError::ChangeMismatch {
                    reason: "a service account is not a reporting target",
                });
            }
            IdentityId::Connector(_) => {
                return Err(IdentityError::ChangeMismatch {
                    reason: "a connector is not a reporting target",
                });
            }
            IdentityId::Machine(_) => {
                return Err(IdentityError::ChangeMismatch {
                    reason: "a machine is not a reporting target",
                });
            }
        };
        let gap = if record.state == LifecycleState::Active {
            record.reporting_gap
        } else {
            Some(ReportingGap {
                identity: target,
                state: record.state,
            })
        };
        Ok((person, gap))
    }

    pub(super) fn register_reporting(
        &mut self,
        identity: IdentityId,
        mut record: Record,
        target: IdentityId,
    ) -> Result<(), IdentityError> {
        let responsible = record.responsible.ok_or(IdentityError::ChangeMismatch {
            reason: "a registered agent has no accountable person",
        })?;
        let (_, gap) = self.target_accountability(target)?;
        record.reports_to = Some(target);
        record.reporting_gap = gap;
        Arc::make_mut(&mut self.records).insert(identity, record);
        Arc::make_mut(&mut self.agents_by_person)
            .entry(responsible)
            .or_default()
            .insert(identity);
        Arc::make_mut(&mut self.reporting_children)
            .entry(target)
            .or_default()
            .insert(identity);
        self.agent_count += 1;
        Ok(())
    }

    pub(super) fn apply_reporting_change(
        &mut self,
        identity: IdentityId,
        target: IdentityId,
        index: u64,
    ) -> Result<(), IdentityError> {
        let from = self
            .record(identity)
            .and_then(|record| record.reports_to)
            .ok_or(IdentityError::ChangeMismatch {
                reason: "the changed agent has no reporting edge",
            })?;
        let old = Arc::make_mut(&mut self.reporting_children)
            .get_mut(&from)
            .ok_or(IdentityError::ChangeMismatch {
                reason: "the reporting parent index is missing",
            })?;
        if !old.remove(&identity) {
            return Err(IdentityError::ChangeMismatch {
                reason: "the reporting parent index omits its child",
            });
        }
        if old.is_empty() {
            Arc::make_mut(&mut self.reporting_children).remove(&from);
        }
        Arc::make_mut(&mut self.reporting_children)
            .entry(target)
            .or_default()
            .insert(identity);
        self.held(identity)?.reports_to = Some(target);
        self.refresh_reporting_record(identity, target, Some(index))?;
        self.refresh_reporting_children(identity, Some(index))
    }

    fn refresh_reporting_record(
        &mut self,
        identity: IdentityId,
        target: IdentityId,
        index: Option<u64>,
    ) -> Result<(), IdentityError> {
        let (person, gap) = self.target_accountability(target)?;
        let previous = self
            .record(identity)
            .and_then(|record| record.responsible)
            .ok_or(IdentityError::ChangeMismatch {
                reason: "the reporting child has no accountable person",
            })?;
        if previous != person {
            let old = Arc::make_mut(&mut self.agents_by_person)
                .get_mut(&previous)
                .ok_or(IdentityError::ChangeMismatch {
                    reason: "the accountable person index is missing",
                })?;
            if !old.remove(&identity) {
                return Err(IdentityError::ChangeMismatch {
                    reason: "the accountable person index omits its agent",
                });
            }
            if old.is_empty() {
                Arc::make_mut(&mut self.agents_by_person).remove(&previous);
            }
            Arc::make_mut(&mut self.agents_by_person)
                .entry(person)
                .or_default()
                .insert(identity);
        }
        let record = self.held(identity)?;
        record.responsible = Some(person);
        record.reporting_gap = gap;
        if let Some(index) = index {
            record.events.push(index);
        }
        Ok(())
    }

    pub(super) fn refresh_reporting_children(
        &mut self,
        root: IdentityId,
        index: Option<u64>,
    ) -> Result<(), IdentityError> {
        let mut pending = VecDeque::from([root]);
        let mut visited = 0;
        while let Some(parent) = pending.pop_front() {
            let children: Vec<_> = self
                .reporting_children
                .get(&parent)
                .into_iter()
                .flat_map(|children| children.iter().copied())
                .collect();
            for child in children {
                visited += 1;
                if visited > self.agent_count {
                    return Err(IdentityError::ChangeMismatch {
                        reason: "the reporting child index contains a cycle",
                    });
                }
                self.refresh_reporting_record(child, parent, index)?;
                pending.push_back(child);
            }
        }
        Ok(())
    }

    pub(super) fn rebuild_reporting_indexes(&mut self) -> Result<(), IdentityError> {
        Arc::make_mut(&mut self.agents_by_person).clear();
        Arc::make_mut(&mut self.reporting_children).clear();
        self.agent_count = 0;
        let mut pending = VecDeque::new();
        for (identity, record) in self.records.iter() {
            match identity {
                IdentityId::Person(_) => {
                    if record.responsible.is_some() || record.reports_to.is_some() {
                        return Err(IdentityError::ChangeMismatch {
                            reason: "a person snapshot has an agent reporting edge",
                        });
                    }
                    pending.push_back(*identity);
                }
                IdentityId::Agent(_) => {
                    let person = record.responsible.ok_or(IdentityError::ChangeMismatch {
                        reason: "an agent snapshot has no accountable person",
                    })?;
                    let target = record.reports_to.ok_or(IdentityError::ChangeMismatch {
                        reason: "an agent snapshot has no reporting edge",
                    })?;
                    Arc::make_mut(&mut self.agents_by_person)
                        .entry(person)
                        .or_default()
                        .insert(*identity);
                    Arc::make_mut(&mut self.reporting_children)
                        .entry(target)
                        .or_default()
                        .insert(*identity);
                    self.agent_count += 1;
                }
                IdentityId::ServiceAccount(_) => {
                    return Err(IdentityError::ChangeMismatch {
                        reason: "a service account is not a stored directory identity",
                    });
                }
                IdentityId::Connector(_) => {
                    return Err(IdentityError::ChangeMismatch {
                        reason: "a connector is not a stored directory identity",
                    });
                }
                IdentityId::Machine(_) => {
                    return Err(IdentityError::ChangeMismatch {
                        reason: "a machine is not a stored directory identity",
                    });
                }
            }
        }
        let mut visited = 0;
        while let Some(parent) = pending.pop_front() {
            let children: Vec<_> = self
                .reporting_children
                .get(&parent)
                .into_iter()
                .flat_map(|children| children.iter().copied())
                .collect();
            let (person, gap) = self.target_accountability(parent)?;
            for child in children {
                let record = self.held(child)?;
                if record.responsible != Some(person) {
                    return Err(IdentityError::ChangeMismatch {
                        reason: "snapshot accountability disagrees with its reporting edge",
                    });
                }
                record.reporting_gap = gap;
                visited += 1;
                pending.push_back(child);
            }
        }
        if visited != self.agent_count {
            return Err(IdentityError::ChangeMismatch {
                reason: "a snapshot reporting chain does not reach a person",
            });
        }
        Ok(())
    }
}
