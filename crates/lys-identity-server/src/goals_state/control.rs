//! Current-control reads use derived locations instead of fired history.

use super::{
    Change, Changed, Delivery, Fired, Held, Item, Marked, PendingReminder, Resent, Sent, Standing,
    index,
};

impl Held {
    /// Validate every resend input before a leaf or folded state is changed.
    pub fn check_resent(&self, resent: &Resent) -> Result<(), String> {
        let location = self
            .index
            .sent
            .get(&resent.prior)
            .ok_or("resent prior operation is unknown")?;
        let prior = self.pending_at(*location)?;
        if prior.sent.state != Delivery::Uncertain {
            return Err("resent prior delivery is not uncertain".to_owned());
        }
        if resent.by != prior.item.goal.responsible {
            return Err("resent caller is not the responsible person".to_owned());
        }
        if prior.item.standing != Standing::Open || !prior.active {
            return Err("resent goal is closed or inactive".to_owned());
        }
        if resent.fired.goal != prior.item.goal.id
            || resent.fired.reminder != prior.fired.reminder
            || resent.fired.due != prior.fired.due
            || resent.fired.fired < prior.sent.at
        {
            return Err(
                "resent occurrence does not preserve its prior goal and due instant".to_owned(),
            );
        }
        if self.kept(&resent.fired.operation)
            || self.index.firings.contains_key(&resent.fired.operation)
        {
            return Err("resent occurrence identity was already used".to_owned());
        }
        if resent.fired.sent.len() != 1 || resent.fired.refused.is_some() {
            return Err("resent occurrence names exactly one intended delivery".to_owned());
        }
        let sent = &resent.fired.sent[0];
        if sent.session.is_empty()
            || sent.operation.is_empty()
            || sent.state != Delivery::Pending
            || self.index.sent.contains_key(&sent.operation)
            || sent.operation == resent.prior
        {
            return Err("resent delivery has an invalid or reused identity".to_owned());
        }
        Ok(())
    }

    /// The firing under one stable identity, without walking goal history.
    pub fn firing(&self, operation: &str) -> Option<&Fired> {
        let (item, firing) = self.index.firings.get(operation)?;
        self.items.get(*item)?.fired.get(*firing)
    }

    /// Every delivery still asked of a runner, with the text it types.
    pub fn unsettled(&self) -> Result<Vec<(Sent, String)>, String> {
        let mut unsettled = Vec::with_capacity(self.index.pending.len());
        for location in self.index.pending.keys() {
            let pending = self.pending_at(*location)?;
            if pending.active && pending.item.standing == Standing::Open {
                unsettled.push((
                    pending.sent.clone(),
                    crate::goals_store::text_with_words(
                        pending.item,
                        pending.words,
                        pending.fired.fired,
                    ),
                ));
            }
        }
        Ok(unsettled)
    }

    /// Pending occurrences in record order, without visiting settled history.
    pub fn pending(&self) -> Result<Vec<PendingReminder<'_>>, String> {
        self.index
            .pending
            .keys()
            .map(|location| self.pending_at(*location))
            .collect()
    }

    /// Pending deliveries for one session, borrowing their current aim and words.
    pub fn pending_for_session(&self, session: &str) -> Result<Vec<PendingReminder<'_>>, String> {
        self.index
            .pending_by_session
            .get(session)
            .into_iter()
            .flat_map(|locations| locations.iter())
            .map(|location| self.pending_at(*location))
            .collect()
    }

    /// Pending deliveries for one aim, including those awaiting a named refusal.
    pub fn pending_for_goal(&self, goal: &str) -> Result<Vec<PendingReminder<'_>>, String> {
        self.index
            .items
            .get(goal)
            .and_then(|position| self.index.pending_by_goal.get(position))
            .into_iter()
            .flat_map(|locations| locations.iter())
            .map(|location| self.pending_at(*location))
            .collect()
    }

    /// The current queued delivery under one stable operation identity.
    pub fn pending_operation(
        &self,
        operation: &str,
    ) -> Result<Option<PendingReminder<'_>>, String> {
        self.index
            .sent
            .get(operation)
            .filter(|location| self.index.pending.contains_key(location))
            .map(|location| self.pending_at(*location))
            .transpose()
    }

    fn pending_at(&self, location: index::Location) -> Result<PendingReminder<'_>, String> {
        let (position, firing, delivery) = location;
        let item = self
            .items
            .get(position)
            .ok_or_else(|| format!("pending reminder item {position} is absent"))?;
        let fired = item.fired.get(firing).ok_or_else(|| {
            format!(
                "pending reminder occurrence {firing} is absent from goal `{}`",
                item.goal.id
            )
        })?;
        let sent = fired.sent.get(delivery).ok_or_else(|| {
            format!(
                "pending reminder delivery {delivery} is absent from occurrence `{}`",
                fired.operation
            )
        })?;
        let words = match self.index.current_words.get(&position) {
            Some(change) => match &item
                .changes
                .get(*change)
                .ok_or_else(|| {
                    format!(
                        "current words revision {change} is absent from goal `{}`",
                        item.goal.id
                    )
                })?
                .change
            {
                Change::Words { words } => words.as_str(),
                Change::Active { .. } => {
                    return Err(format!(
                        "current words revision {change} of goal `{}` changes activity instead",
                        item.goal.id
                    ));
                }
            },
            None => item.goal.words.as_str(),
        };
        Ok(PendingReminder {
            item,
            fired,
            sent,
            prior: self.resends.get(&fired.operation).map(String::as_str),
            words,
            active: self
                .index
                .current_active
                .get(&position)
                .copied()
                .unwrap_or(item.goal.active),
            version: item
                .changes
                .last()
                .map_or(item.goal.id.as_str(), |change| change.operation.as_str()),
        })
    }
}

impl Held {
    /// The item `id`.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.index
            .items
            .get(id)
            .and_then(|position| self.items.get(*position))
    }

    pub(super) fn position(&self, id: &str) -> Result<usize, String> {
        self.index
            .items
            .get(id)
            .copied()
            .ok_or_else(|| format!("no goal `{id}` is held"))
    }

    pub(super) fn item_mut(&mut self, id: &str) -> Result<&mut Item, String> {
        let position = self.position(id)?;
        self.items
            .get_mut(position)
            .ok_or_else(|| format!("no goal `{id}` is held"))
    }

    /// The judgement kept under `operation`.
    pub fn marked(&self, operation: &str) -> Option<&Marked> {
        self.items
            .get(*self.index.marked.get(operation)?)?
            .marked
            .as_ref()
    }

    /// The change kept under an operation id.
    pub fn changed(&self, operation: &str) -> Option<&Changed> {
        let (item, change) = self.index.changed.get(operation)?;
        self.items.get(*item)?.changes.get(*change)
    }

    /// Whether `operation` names a firing, event or aim change already kept.
    pub fn kept(&self, operation: &str) -> bool {
        self.index.kept.contains(operation)
    }
}
